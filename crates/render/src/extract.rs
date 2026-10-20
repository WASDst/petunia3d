//! Adaptador documento/editor → [`RenderFrameInput`].
//!
//! É o **único** ponto do crate que conhece `Project`, câmera e estado de
//! seleção. Triangula, resolve cor de material, classifica arestas por
//! [`EdgeMode`], monta os overlays de componente, posiciona referências e
//! resolve luz/contorno; o renderer só consome o snapshot.
//!
//! Cache por objeto (`ObjectId` + revisão de superfície/arestas): câmera nunca
//! invalida, seleção não refaz a superfície e transformações rígidas em
//! andamento só trocam a matriz de modelo.

use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use petunia_core::viewport::PhysicalViewport;
use petunia_core::{
    Camera, FingerprintFlags, HoverTarget, RefAxis, ReferenceImage, SelectionDomain, TextureUpdate,
    fingerprint_scene,
};
use petunia_project::{Asset, PoseOverride, Project, ShaderProfile, mesh_to_draw};
use uuid::Uuid;

use crate::Shading;
use crate::frame::{
    CameraMatrices, ComponentOverlays, DirtyRect, IDENTITY, OutlineEntry, OverlayFaceVertex,
    OverlayLine, OverlayPoint, OwnedReference, OwnedRenderFrame, OwnedTexture, RenderEdge,
    RenderFrameInput, RenderLight, RenderMesh, RenderObject, RenderOutline, RenderReferenceImage,
    RenderTexture, RenderVertex, ViewportPalette, ViewportRenderSettings,
};
use crate::overlays::{OverlayShape, WorkplaneOverlay, edge_overlay_visible};
use crate::scene::{self, RefPlane};

/// Cor padrão da geometria nova; substituída pela cor do material.
const DEFAULT_GEOMETRY_COLOR: [f32; 3] = [0.72, 0.73, 0.78];
/// Ângulo entre faces vizinhas acima do qual a aresta é "de feição".
pub const CREASE_DEGREES: f32 = 30.0;
/// Arestas comuns (não de feição) em relação à largura base.
const THIN_EDGE_SCALE: f32 = 0.67;
const FEATURE_EDGE_COLOR: [f32; 3] = [0.05, 0.05, 0.06];
const THIN_EDGE_COLOR: [f32; 3] = [0.16, 0.17, 0.19];
const WIRE_EDGE_COLOR: [f32; 3] = [0.62, 0.66, 0.74];
const WIRE_SELECTED_EDGE_COLOR: [f32; 3] = [1.0, 0.62, 0.20];
const TRIANGULATION_COLOR: [f32; 3] = [0.3, 0.65, 0.95];
/// Preselection: ciano #7DDCFF único em GPU, software e shell.
// B12 (guia de linguagem visual): componentes discretos e escuros, só o que está
// selecionado ganha cor e tamanho; o hover é um destaque, não um halo.
const HOVER_LINE: [f32; 4] = [0.49, 0.86, 1.0, 0.9];
const HOVER_FACE: [f32; 4] = [0.49, 0.86, 1.0, 0.12];
const EDGE_GUIDE: [f32; 4] = [0.05, 0.05, 0.06, 0.9];
const VERTEX_GUIDE: [f32; 4] = [0.04, 0.04, 0.05, 1.0];
/// Raio dos pontos de vértice em px lógicos: comum, selecionado e sob o cursor.
const VERTEX_RADIUS: f32 = 2.0;
const VERTEX_SELECTED_RADIUS: f32 = 3.0;
const VERTEX_HOVER_RADIUS: f32 = 3.5;
/// Tinta da face selecionada (o contorno na cor de seleção dá a forma).
const FACE_SELECTED_ALPHA: f32 = 0.22;

struct Hasher(u64);

impl Hasher {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
    fn mix(&mut self, value: u64) {
        self.0 ^= value;
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
    }
    fn f32(&mut self, value: f32) {
        self.mix(u64::from(value.to_bits()));
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Chave barata das flags que afetam grid/eixos.
fn overlay_key(settings: &ViewportRenderSettings) -> u64 {
    let mut h = Hasher::new();
    h.mix(u64::from(settings.show_overlays));
    h.mix(u64::from(settings.show_grid));
    h.mix(u64::from(settings.show_axes));
    for visible in settings.axes_visible {
        h.mix(u64::from(visible));
    }
    for value in [
        settings.grid.size,
        settings.grid.spacing,
        settings.grid.opacity,
        settings.grid.isometric_angle_deg,
    ] {
        h.f32(value);
    }
    h.mix(u64::from(settings.grid.show_isometric_guide));
    h.finish()
}

fn plane_for(axis: RefAxis) -> RefPlane {
    match axis {
        RefAxis::Front => RefPlane::Front,
        RefAxis::Back => RefPlane::Back,
        RefAxis::Left => RefPlane::Left,
        RefAxis::Right | RefAxis::Side => RefPlane::Right,
        RefAxis::Top => RefPlane::Top,
        RefAxis::Bottom => RefPlane::Bottom,
    }
}

/// Converte a lista de triângulos expandida em malha indexada (sem duplicatas exatas).
fn index_triangles(
    triangles: impl IntoIterator<Item = ([f32; 3], [f32; 3], [f32; 3], [f32; 2])>,
    mut recolor: impl FnMut([f32; 3]) -> [f32; 3],
) -> RenderMesh {
    let mut mesh = RenderMesh::default();
    let mut seen: HashMap<[u32; 11], u32> = HashMap::new();
    for (position, normal, color, uv) in triangles {
        let color = recolor(color);
        let key = [
            position[0].to_bits(),
            position[1].to_bits(),
            position[2].to_bits(),
            normal[0].to_bits(),
            normal[1].to_bits(),
            normal[2].to_bits(),
            color[0].to_bits(),
            color[1].to_bits(),
            color[2].to_bits(),
            uv[0].to_bits(),
            uv[1].to_bits(),
        ];
        let index = *seen.entry(key).or_insert_with(|| {
            mesh.vertices.push(RenderVertex {
                position,
                normal,
                color: [color[0], color[1], color[2], 1.0],
                texture_coordinates: uv,
            });
            (mesh.vertices.len() - 1) as u32
        });
        mesh.indices.push(index);
    }
    mesh
}

/// Estilo e alvo da camada de seleção de componentes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentStyle {
    pub domain: SelectionDomain,
    pub hover: HoverTarget,
    pub selection_rgb: [u8; 3],
    /// Espessura da seleção em px lógicos (1–6).
    pub thickness: f32,
}

impl Default for ComponentStyle {
    fn default() -> Self {
        Self {
            domain: SelectionDomain::Object,
            hover: HoverTarget::None,
            selection_rgb: [233, 106, 0],
            thickness: 2.0,
        }
    }
}

/// Entradas de um frame, independentes de `AppState`: quem tiver projeto, câmera e
/// flags (viewport Slint, testes, ferramentas) produz um frame sem tocar o estado
/// da aplicação.
pub struct SceneParts<'a> {
    pub project: &'a Project,
    pub refs: &'a [ReferenceImage],
    pub camera: &'a Camera,
    pub settings: ViewportRenderSettings,
    pub palette: ViewportPalette,
    pub viewport: PhysicalViewport,
    pub framebuffer_height: u32,
    /// Luz de estúdio presa à câmera (Solid/Material); `false` fixa no mundo.
    pub studio_light_follows_camera: bool,
    /// Malhas deformadas por skin (preview do Animate).
    pub pose: Option<&'a PoseOverride>,
    /// Objetos em transformação rígida neste frame: matriz original → atual.
    pub rigid_previews: &'a [(Uuid, Mat4)],
    pub components: ComponentStyle,
    /// Objetos com contorno de seleção e o ativo.
    pub outlined: &'a [Uuid],
    pub outlined_active: Option<Uuid>,
    /// Regiões de textura alteradas desde o último frame (Paint).
    pub texture_updates: &'a [TextureUpdate],
    pub workplane: Option<WorkplaneOverlay>,
    pub screen_overlay: &'a [OverlayShape],
}

impl<'a> SceneParts<'a> {
    /// Partes mínimas: tudo o mais nos padrões (sem seleção, pose ou overlays).
    pub fn new(project: &'a Project, camera: &'a Camera, viewport: PhysicalViewport) -> Self {
        Self {
            project,
            refs: &[],
            camera,
            settings: ViewportRenderSettings::default(),
            palette: ViewportPalette::default(),
            viewport,
            framebuffer_height: viewport.height,
            studio_light_follows_camera: true,
            pose: None,
            rigid_previews: &[],
            components: ComponentStyle::default(),
            outlined: &[],
            outlined_active: None,
            texture_updates: &[],
            workplane: None,
            screen_overlay: &[],
        }
    }
}

/// Objeto em cache com as chaves que o produziram.
struct CachedObject {
    surface_key: u64,
    edge_key: u64,
    object: RenderObject,
}

/// Cache de pixels copiados para frames sem empréstimo (por id + revisão).
#[derive(Default)]
struct PixelCache {
    textures: HashMap<Uuid, (u64, Arc<Vec<u8>>)>,
    references: Vec<(u64, Arc<Vec<u8>>)>,
}

/// Produz frames a partir do documento e guarda os objetos derivados entre frames.
#[derive(Default)]
pub struct SceneExtractor {
    objects: Arc<Vec<RenderObject>>,
    cache: HashMap<Uuid, CachedObject>,
    /// Objeto capturado no início de uma transformação rígida e a inversa da
    /// matriz daquele instante.
    rigid_base: HashMap<Uuid, (RenderObject, Mat4)>,
    /// Chave global (fingerprint + flags) do último conjunto de objetos.
    scene_key: Option<u64>,
    components: Arc<ComponentOverlays>,
    components_key: Option<u64>,
    outline: RenderOutline,
    rebuilds: u64,
    object_rebuilds: u64,
    pixels: PixelCache,
    surface_atlases: HashMap<Uuid, petunia_project::Canvas>,
    surface_atlases_key: Option<(u64, u64, bool)>,
}

impl SceneExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Quantas vezes o conjunto de objetos foi reavaliado (telemetria/benchmarks).
    pub fn rebuild_count(&self) -> u64 {
        self.rebuilds
    }

    /// Quantos objetos foram triangulados de novo (os demais vieram do cache).
    pub fn object_rebuild_count(&self) -> u64 {
        self.object_rebuilds
    }

    /// Revisão da camada de componentes (muda só quando o conteúdo muda).
    pub fn components_revision(&self) -> u64 {
        self.components.revision
    }

    /// Monta o snapshot a partir de partes avulsas.
    pub fn extract_parts<'a>(&'a mut self, parts: SceneParts<'a>) -> RenderFrameInput<'a> {
        puffin::profile_function!();
        let SceneParts {
            project,
            refs,
            camera,
            mut settings,
            palette,
            viewport,
            framebuffer_height,
            studio_light_follows_camera,
            pose,
            rigid_previews,
            components,
            outlined,
            outlined_active,
            texture_updates,
            workplane,
            screen_overlay,
        } = parts;
        settings.light = resolve_light(
            project,
            camera,
            settings.shading,
            studio_light_follows_camera,
        );
        let samples_textures = settings.textured || settings.shading.samples_material();
        let fingerprint = fingerprint_scene(
            project,
            refs,
            FingerprintFlags {
                shading: settings.shading,
                xray: settings.xray,
                show_triangulation: settings.show_triangulation,
                textured: samples_textures,
                edit_mode_is_edit: settings.edit_mode_is_edit,
                show_wireframe_overlay: settings.show_wireframe_overlay,
                show_face_orientation: false,
                show_uv_checker: false,
            },
        );

        let mut scene_key = Hasher::new();
        scene_key.mix(fingerprint.mesh);
        if project
            .assets
            .iter()
            .any(|a| !a.surface_materials.is_empty())
        {
            scene_key.mix(fingerprint.textures);
        }
        scene_key.mix(settings.shading as u64);
        scene_key.mix(settings.edge_mode as u64);
        scene_key.mix(u64::from(settings.show_wireframe_overlay));
        scene_key.mix(u64::from(settings.show_triangulation));
        scene_key.mix(u64::from(samples_textures));
        if settings.shading == Shading::Wireframe {
            scene_key.mix(fingerprint.selection);
        }
        scene_key.mix(pose.map_or(u64::MAX, |p| p.revision));
        // A matriz de mundo do objeto (P01/G10) muda só a matriz de modelo: o
        // objeto é reaproveitado do cache, sem triangular nem reenviar a malha.
        for asset in project.assets.iter().filter(|asset| asset.visible) {
            for value in asset.world_matrix().to_cols_array() {
                scene_key.f32(value);
            }
        }
        for (id, matrix) in rigid_previews {
            scene_key.mix(id.as_u64_pair().0);
            for value in matrix.to_cols_array() {
                scene_key.f32(value);
            }
        }
        let scene_key = scene_key.finish();
        if self.scene_key != Some(scene_key) {
            let atlas_key = (fingerprint.mesh, fingerprint.textures, samples_textures);
            if self.surface_atlases_key != Some(atlas_key) {
                self.surface_atlases = if samples_textures {
                    project
                        .assets
                        .iter()
                        .filter(|a| a.visible)
                        .filter_map(|a| {
                            crate::surface_materials::atlas(project, a).map(|canvas| (a.id, canvas))
                        })
                        .collect()
                } else {
                    HashMap::new()
                };
                self.surface_atlases_key = Some(atlas_key);
            }
            self.rebuild_objects(project, &settings, samples_textures, pose, rigid_previews);
            self.scene_key = Some(scene_key);
            self.rebuilds += 1;
        }

        let mut components_key = Hasher::new();
        components_key.mix(fingerprint.mesh);
        components_key.mix(fingerprint.selection);
        components_key.mix(pose.map_or(u64::MAX, |p| p.revision));
        components_key.mix(components.domain as u64);
        components_key.mix(hover_key(components.hover));
        components_key.mix(u64::from(u32::from_le_bytes([
            components.selection_rgb[0],
            components.selection_rgb[1],
            components.selection_rgb[2],
            0,
        ])));
        components_key.f32(components.thickness);
        components_key.mix(project.active as u64);
        if let Some(asset) = project.assets.get(project.active) {
            for value in asset.world_matrix().to_cols_array() {
                components_key.f32(value);
            }
            // A cor da parte também entra: trocar só a cor refaz os overlays.
            let [r, g, b] = project.selection_color_of(asset).unwrap_or([0; 3]);
            components_key.mix(u64::from(u32::from_le_bytes([r, g, b, 1])));
        }
        let components_key = components_key.finish();
        if self.components_key != Some(components_key) {
            let mut overlays = build_component_overlays(project, pose, &components);
            overlays.revision = self.components.revision.wrapping_add(1);
            self.components = Arc::new(overlays);
            self.components_key = Some(components_key);
        }

        let textures = project
            .assets
            .iter()
            .filter(|asset| asset.visible)
            .filter_map(|asset| {
                let canvas = self
                    .surface_atlases
                    .get(&asset.id)
                    .or_else(|| object_canvas(project, samples_textures, asset))?;
                let dirty = texture_updates
                    .iter()
                    .find(|update| update.asset_id == asset.id)
                    .and_then(|update| update.regions.as_ref())
                    .map(|regions| {
                        regions
                            .iter()
                            .map(|r| DirtyRect {
                                x: r.x,
                                y: r.y,
                                width: r.width,
                                height: r.height,
                            })
                            .collect()
                    });
                Some(RenderTexture {
                    id: asset.id,
                    width: canvas.w,
                    height: canvas.h,
                    revision: fingerprint.textures ^ fingerprint.mesh,
                    pixels: &canvas.pixels,
                    dirty: if asset.surface_materials.is_empty() {
                        dirty
                    } else {
                        None
                    },
                })
            })
            .collect();

        let references = refs
            .iter()
            .map(|image| {
                let aspect = image.width as f32 / image.height.max(1) as f32;
                RenderReferenceImage {
                    width: image.width,
                    height: image.height,
                    revision: image.revision,
                    rgba: &image.rgba,
                    quad: scene::ref_quad_with_rot(
                        plane_for(image.axis),
                        image.offset,
                        image.size,
                        aspect,
                        image.rotation,
                    ),
                    opacity: image.opacity,
                    visible: image.visible,
                    xray: image.xray,
                }
            })
            .collect();

        self.outline = resolve_outline(project, &components, outlined, outlined_active);
        RenderFrameInput {
            viewport,
            framebuffer_height,
            camera: CameraMatrices::from_camera(camera),
            palette,
            settings,
            overlay_revision: overlay_key(&settings),
            objects: &self.objects,
            textures,
            references,
            components: &self.components,
            outline: &self.outline,
            workplane,
            screen_overlay,
        }
    }

    /// Reaproveita os objetos cujas chaves não mudaram e triangula os demais.
    fn rebuild_objects(
        &mut self,
        project: &Project,
        settings: &ViewportRenderSettings,
        samples_textures: bool,
        pose: Option<&PoseOverride>,
        rigid_previews: &[(Uuid, Mat4)],
    ) {
        puffin::profile_function!();
        self.rigid_base
            .retain(|id, _| rigid_previews.iter().any(|(rid, _)| rid == id));
        let mut previous = std::mem::take(&mut self.cache);
        let mut objects = Vec::with_capacity(project.assets.len());
        for asset in project.assets.iter().filter(|asset| asset.visible) {
            let rigid = rigid_previews
                .iter()
                .find(|(id, _)| *id == asset.id)
                .map(|(_, matrix)| *matrix)
                .filter(|_| pose.is_none());
            if let Some(matrix) = rigid {
                // Só se move: o objeto capturado recebe a matriz relativa, sem
                // triangular nem classificar arestas de novo.
                let base = match self.rigid_base.get(&asset.id) {
                    Some(base) => base,
                    None => {
                        let mesh = mesh_to_draw(pose, asset);
                        let object = build_object(
                            project,
                            asset,
                            &mesh,
                            settings,
                            samples_textures,
                            self.surface_atlases.get(&asset.id),
                        );
                        self.object_rebuilds += 1;
                        self.rigid_base
                            .entry(asset.id)
                            .or_insert((object, matrix.inverse()))
                    }
                };
                let mut object = base.0.clone();
                object.transform = (matrix * base.1 * asset.world_matrix()).to_cols_array();
                objects.push(object);
                continue;
            }
            let mesh = mesh_to_draw(pose, asset);
            let surface_key = surface_key(project, asset, &mesh, settings, samples_textures);
            let edge_key = edge_key(&mesh, settings);
            let object = match previous.remove(&asset.id) {
                Some(cached)
                    if cached.surface_key == surface_key && cached.edge_key == edge_key =>
                {
                    cached.object
                }
                Some(cached) if cached.surface_key == surface_key => {
                    let mut object = cached.object;
                    object.edges = build_edges(&mesh, settings);
                    object.edge_revision = edge_key;
                    object
                }
                _ => {
                    self.object_rebuilds += 1;
                    build_object(
                        project,
                        asset,
                        &mesh,
                        settings,
                        samples_textures,
                        self.surface_atlases.get(&asset.id),
                    )
                }
            };
            let mut object = object;
            object.transform = asset.world_matrix().to_cols_array();
            self.cache.insert(
                asset.id,
                CachedObject {
                    surface_key,
                    edge_key,
                    object: object.clone(),
                },
            );
            objects.push(object);
        }
        // Prévia de objeto novo: só desenho, com a aparência padrão de um
        // objeto recém-criado (é o que o commit vai criar). Mesmo cache por id.
        for (id, mesh) in pose.map_or(&[][..], PoseOverride::transient) {
            let mut asset = Asset::new("preview", mesh.clone());
            asset.id = *id;
            let surface_key = surface_key(project, &asset, mesh, settings, samples_textures);
            let edge_key = edge_key(mesh, settings);
            let object = match previous.remove(id) {
                Some(cached)
                    if cached.surface_key == surface_key && cached.edge_key == edge_key =>
                {
                    cached.object
                }
                _ => {
                    self.object_rebuilds += 1;
                    build_object(project, &asset, mesh, settings, samples_textures, None)
                }
            };
            self.cache.insert(
                *id,
                CachedObject {
                    surface_key,
                    edge_key,
                    object: object.clone(),
                },
            );
            objects.push(object);
        }
        self.objects = Arc::new(objects);
    }

    /// Frame sem empréstimos (para desenhar depois, com o contexto GL corrente). Os
    /// pixels só são copiados quando a revisão da textura/referência muda.
    pub fn extract_owned(&mut self, parts: SceneParts<'_>) -> OwnedRenderFrame {
        let screen_overlay = parts.screen_overlay.to_vec();
        let cached_textures: HashMap<Uuid, u64> = self
            .pixels
            .textures
            .iter()
            .map(|(id, (revision, _))| (*id, *revision))
            .collect();
        let cached_refs: Vec<u64> = self.pixels.references.iter().map(|(r, _)| *r).collect();
        let (header, textures, references) = {
            let input = self.extract_parts(parts);
            let textures: Vec<_> = input
                .textures
                .iter()
                .map(|t| {
                    let fresh = cached_textures.get(&t.id) != Some(&t.revision);
                    (
                        t.id,
                        t.width,
                        t.height,
                        t.revision,
                        if fresh { t.pixels.to_vec() } else { Vec::new() },
                        t.dirty.clone(),
                    )
                })
                .collect();
            let references: Vec<_> = input
                .references
                .iter()
                .enumerate()
                .map(|(index, r)| {
                    let fresh = cached_refs.get(index) != Some(&r.revision);
                    (
                        r.width,
                        r.height,
                        r.revision,
                        if fresh { r.rgba.to_vec() } else { Vec::new() },
                        r.quad,
                        r.opacity,
                        r.visible,
                        r.xray,
                    )
                })
                .collect();
            let header = (
                input.viewport,
                input.framebuffer_height,
                input.camera,
                input.palette,
                input.settings,
                input.overlay_revision,
                input.workplane,
                input.outline.clone(),
            );
            (header, textures, references)
        };
        let (
            viewport,
            framebuffer_height,
            camera,
            palette,
            settings,
            overlay_revision,
            workplane,
            outline,
        ) = header;
        let mut owned_textures = Vec::new();
        let mut live = HashMap::new();
        for (id, width, height, revision, pixels, dirty) in textures {
            let shared = match self.pixels.textures.remove(&id) {
                Some((rev, arc)) if rev == revision => arc,
                _ => Arc::new(pixels),
            };
            live.insert(id, (revision, Arc::clone(&shared)));
            owned_textures.push(OwnedTexture {
                id,
                width,
                height,
                revision,
                pixels: shared,
                dirty,
            });
        }
        self.pixels.textures = live;
        let previous = std::mem::take(&mut self.pixels.references);
        let mut owned_refs = Vec::new();
        for (index, (width, height, revision, rgba, quad, opacity, visible, xray)) in
            references.into_iter().enumerate()
        {
            let shared = match previous.get(index) {
                Some((rev, arc)) if *rev == revision => Arc::clone(arc),
                _ => Arc::new(rgba),
            };
            self.pixels.references.push((revision, Arc::clone(&shared)));
            owned_refs.push(OwnedReference {
                width,
                height,
                revision,
                rgba: shared,
                quad,
                opacity,
                visible,
                xray,
            });
        }
        OwnedRenderFrame {
            viewport,
            framebuffer_height,
            camera,
            palette,
            settings,
            overlay_revision,
            objects: Arc::clone(&self.objects),
            textures: owned_textures,
            references: owned_refs,
            components: Arc::clone(&self.components),
            outline,
            workplane,
            screen_overlay,
        }
    }
}

fn hover_key(hover: HoverTarget) -> u64 {
    match hover {
        HoverTarget::None => 0,
        HoverTarget::Vertex(index) => 1 << 60 | u64::from(index),
        HoverTarget::Edge(a, b) => 2 << 60 | (u64::from(a) << 30) ^ u64::from(b),
        HoverTarget::Face(index) => 3 << 60 | index as u64,
        HoverTarget::Object(index) => 4 << 60 | index as u64,
    }
}

/// Cor de fundo do viewport para um tema (tokens resolvidos fora do renderer).
pub fn palette_for_theme(theme_id: &str) -> ViewportPalette {
    let background = petunia_config::ThemeRegistry::global()
        .get_theme(theme_id)
        .map(|theme| {
            theme
                .colors
                .get_token_color(petunia_config::ThemeToken::BgCanvas)
                .to_rgba_f32()
        })
        .unwrap_or(ViewportPalette::default().background);
    ViewportPalette { background }
}

/// Fonte de luz por modo: Solid/Material usam o estúdio da viewport (preso à
/// câmera por padrão), Rendered usa a primeira luz habilitada da cena e cai no
/// estúdio quando a cena não tem luz (em vez de renderizar preto).
pub fn resolve_light(
    project: &Project,
    camera: &Camera,
    shading: Shading,
    studio_light_follows_camera: bool,
) -> RenderLight {
    let normalize = |[x, y, z]: [f32; 3]| {
        let length = (x * x + y * y + z * z).sqrt();
        if length > 1e-6 {
            [x / length, y / length, z / length]
        } else {
            RenderLight::default().direction
        }
    };
    match (shading.uses_scene_light(), project.active_light()) {
        (true, Some(light)) => RenderLight {
            direction: normalize(light.normalized_direction()),
            ambient: scene::LIGHT_AMBIENT * 0.35,
            diffuse: scene::LIGHT_DIFFUSE * light.intensity.clamp(0.0, 8.0),
        },
        _ => RenderLight {
            direction: normalize(if studio_light_follows_camera {
                scene::studio_light_for_camera(camera)
            } else {
                scene::LIGHT_DIR
            }),
            ambient: scene::LIGHT_AMBIENT,
            diffuse: scene::LIGHT_DIFFUSE,
        },
    }
}

fn rgb_f32(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|channel| f32::from(channel) / 255.0)
}

/// Contorno dos objetos selecionados: base em 62 % da cor de seleção e o ativo
/// na cor da parte (ou da seleção).
/// Cor do contorno de um objeto: a da própria parte (`selection_overlay_color`)
/// ou a global. Os níveis seguem o papel: ativo pleno, selecionado mais escuro,
/// hover discreto.
fn outline_rgb(project: &Project, asset: &Asset, style: &ComponentStyle) -> [f32; 3] {
    rgb_f32(
        project
            .selection_color_of(asset)
            .unwrap_or(style.selection_rgb),
    )
}

fn resolve_outline(
    project: &Project,
    style: &ComponentStyle,
    selected: &[Uuid],
    active: Option<Uuid>,
) -> RenderOutline {
    let mut entries = Vec::with_capacity(selected.len() + 1);
    let find = |id: Uuid| project.assets.iter().find(|asset| asset.id == id);
    // Hover primeiro: qualquer seleção o cobre.
    if style.domain == SelectionDomain::Object
        && let HoverTarget::Object(index) = style.hover
        && let Some(asset) = project.assets.get(index)
        && asset.visible
        && !selected.contains(&asset.id)
    {
        let rgb = outline_rgb(project, asset, style);
        entries.push(OutlineEntry {
            object_id: asset.id,
            color: [
                rgb[0] * 0.5 + 0.25,
                rgb[1] * 0.5 + 0.25,
                rgb[2] * 0.5 + 0.25,
                1.0,
            ],
        });
    }
    let active = active.filter(|id| selected.contains(id));
    for id in selected.iter().copied().filter(|id| Some(*id) != active) {
        if let Some(asset) = find(id) {
            let rgb = outline_rgb(project, asset, style);
            entries.push(OutlineEntry {
                object_id: id,
                color: [rgb[0] * 0.62, rgb[1] * 0.62, rgb[2] * 0.62, 1.0],
            });
        }
    }
    if let Some(id) = active
        && let Some(asset) = find(id)
    {
        let rgb = outline_rgb(project, asset, style);
        entries.push(OutlineEntry {
            object_id: id,
            color: [rgb[0], rgb[1], rgb[2], 1.0],
        });
    }
    RenderOutline {
        entries,
        radius_px: style.thickness.clamp(1.0, 4.0),
    }
}

/// Canvas a texturar: do objeto ou do albedo do material.
fn object_canvas<'a>(
    project: &'a Project,
    samples_textures: bool,
    asset: &'a Asset,
) -> Option<&'a petunia_project::Canvas> {
    if !samples_textures {
        return None;
    }
    asset.texture.as_ref().or_else(|| {
        asset
            .material(project)
            .and_then(|material| material.albedo_texture.as_ref())
    })
}

/// Chave da superfície de um objeto: posições, cores, faces, UVs, slots,
/// Sharp/Smooth, material e textura. A seleção nunca entra.
fn surface_key(
    project: &Project,
    asset: &Asset,
    mesh: &petunia_core::Mesh,
    settings: &ViewportRenderSettings,
    samples_textures: bool,
) -> u64 {
    let mut h = Hasher::new();
    h.mix(u64::from(settings.shading == Shading::Wireframe));
    h.mix(asset.surface_shading as u64);
    h.mix(u64::from(
        object_canvas(project, samples_textures, asset).is_some(),
    ));
    for c in asset.base_color {
        h.f32(c);
    }
    if let Some(material) = asset.material(project) {
        h.mix(material.profile as u64);
        for c in material
            .base_color
            .iter()
            .chain(material.emission_color.iter())
        {
            h.f32(*c);
        }
        h.f32(material.emission_strength);
    }
    if !asset.surface_materials.is_empty() {
        h.mix(project.texture_revision);
    }
    for surface in &asset.surface_materials {
        for c in surface.base_color {
            h.f32(c);
        }
        if let Some(id) = surface.material_id {
            for byte in id.as_bytes() {
                h.mix(u64::from(*byte));
            }
            if let Some(material) = project.get_material(id) {
                h.mix(material.profile as u64);
                for c in material
                    .base_color
                    .iter()
                    .chain(material.emission_color.iter())
                {
                    h.f32(*c);
                }
                h.f32(material.emission_strength);
            }
        }
    }
    h.mix(mesh.verts.len() as u64);
    h.mix(mesh.faces.len() as u64);
    for vertex in &mesh.verts {
        for c in vertex.pos.iter().chain(vertex.color.iter()) {
            h.f32(*c);
        }
    }
    for face in &mesh.faces {
        h.mix(face.verts.len() as u64);
        for &index in &face.verts {
            h.mix(u64::from(index));
        }
        for uv in &face.uv {
            h.f32(uv[0]);
            h.f32(uv[1]);
        }
        h.mix(face.material_slot.map_or(u64::MAX, |slot| slot as u64));
    }
    let mut sharp: Vec<_> = mesh.sharp_edges.iter().copied().collect();
    sharp.sort_unstable();
    for (a, b) in sharp {
        h.mix((u64::from(a) << 32) | u64::from(b));
    }
    h.finish()
}

/// Chave das arestas: topologia/posições, modo de leitura e (só no Wireframe,
/// que pinta a seleção na própria aresta) as arestas selecionadas.
fn edge_key(mesh: &petunia_core::Mesh, settings: &ViewportRenderSettings) -> u64 {
    let mut h = Hasher::new();
    let wire = settings.shading == Shading::Wireframe;
    h.mix(u64::from(wire));
    h.mix(settings.edge_mode as u64);
    h.mix(u64::from(settings.show_wireframe_overlay));
    h.mix(u64::from(settings.show_triangulation));
    h.mix(mesh.verts.len() as u64);
    for vertex in &mesh.verts {
        for c in vertex.pos {
            h.f32(c);
        }
    }
    for face in &mesh.faces {
        h.mix(face.verts.len() as u64);
        for &index in &face.verts {
            h.mix(u64::from(index));
        }
    }
    if wire {
        let mut edges: Vec<_> = mesh.selected_edges.iter().copied().collect();
        edges.sort_unstable();
        for (a, b) in edges {
            h.mix((u64::from(a) << 32) | u64::from(b));
        }
    }
    h.finish()
}

/// Arestas do objeto com cor e largura por modo (capítulo 05, "Overlays, não
/// novos modos"). No Wireframe todas, as selecionadas destacadas; nos demais o
/// toggle do overlay é o mestre das finas e o modo mantém as de feição.
pub fn build_edges(
    mesh: &petunia_core::Mesh,
    settings: &ViewportRenderSettings,
) -> Vec<RenderEdge> {
    let mut edges = Vec::new();
    if settings.shading == Shading::Wireframe {
        for (a, b, selected) in mesh.to_edges() {
            edges.push(RenderEdge {
                a,
                b,
                color: if selected {
                    WIRE_SELECTED_EDGE_COLOR
                } else {
                    WIRE_EDGE_COLOR
                },
                width: 1.0,
                normals: [[0.0; 3]; 2],
            });
        }
    } else {
        let adjacency = edge_face_normals(mesh);
        for (a, b, _selected, feature) in mesh.to_classified_edges(CREASE_DEGREES) {
            if !edge_overlay_visible(settings.show_wireframe_overlay, settings.edge_mode, feature) {
                continue;
            }
            let (color, width) = if feature {
                (FEATURE_EDGE_COLOR, 1.0)
            } else {
                (THIN_EDGE_COLOR, THIN_EDGE_SCALE)
            };
            let normals = adjacency
                .get(&position_key(a, b))
                .copied()
                .unwrap_or([[0.0; 3]; 2]);
            edges.push(RenderEdge {
                a,
                b,
                color,
                width,
                normals,
            });
        }
    }
    if settings.show_triangulation {
        for (a, b) in mesh.triangulation_wireframe() {
            edges.push(RenderEdge {
                a,
                b,
                color: TRIANGULATION_COLOR,
                width: THIN_EDGE_SCALE,
                normals: [[0.0; 3]; 2],
            });
        }
    }
    edges
}

/// Chave de aresta pelas posições dos extremos, ordem-independente.
type EdgeKey = ([u32; 3], [u32; 3]);

/// Normais das (até duas) faces vizinhas de uma aresta.
type EdgeNormals = [[f32; 3]; 2];

/// Chave de aresta pelas posições (independe da ordem dos extremos).
fn position_key(a: [f32; 3], b: [f32; 3]) -> EdgeKey {
    let (ka, kb) = (a.map(f32::to_bits), b.map(f32::to_bits));
    if ka <= kb { (ka, kb) } else { (kb, ka) }
}

/// Normais das (até) duas faces de cada aresta. Arestas com mais de duas faces
/// (não-manifold) ou com uma só ficam com zero: sempre desenhadas.
fn edge_face_normals(mesh: &petunia_core::Mesh) -> HashMap<EdgeKey, EdgeNormals> {
    let mut faces_of: HashMap<EdgeKey, (u8, EdgeNormals)> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        let normal = mesh.face_normal(index);
        if !normal.is_finite() || normal.length_squared() < 1.0e-12 {
            continue;
        }
        let count = face.verts.len();
        for corner in 0..count {
            let (Some(&ia), Some(&ib)) =
                (face.verts.get(corner), face.verts.get((corner + 1) % count))
            else {
                continue;
            };
            let (Some(va), Some(vb)) = (mesh.verts.get(ia as usize), mesh.verts.get(ib as usize))
            else {
                continue;
            };
            let entry = faces_of
                .entry(position_key(va.pos, vb.pos))
                .or_insert((0, [[0.0; 3]; 2]));
            if let Some(slot) = entry.1.get_mut(usize::from(entry.0)) {
                *slot = normal.to_array();
            }
            entry.0 = entry.0.saturating_add(1);
        }
    }
    faces_of
        .into_iter()
        .map(|(key, (count, normals))| (key, if count == 2 { normals } else { [[0.0; 3]; 2] }))
        .collect()
}

fn build_object(
    project: &Project,
    asset: &Asset,
    mesh: &petunia_core::Mesh,
    settings: &ViewportRenderSettings,
    samples_textures: bool,
    atlas: Option<&petunia_project::Canvas>,
) -> RenderObject {
    puffin::profile_function!();
    let render_mesh = if settings.shading == Shading::Wireframe {
        RenderMesh::default()
    } else if !asset.surface_materials.is_empty() {
        index_triangles(
            crate::surface_materials::triangles(project, asset, mesh, atlas),
            |color| color,
        )
    } else {
        let (profile, material_color, emission) = match asset.material(project) {
            Some(material) => (
                material.profile,
                [
                    material.base_color[0],
                    material.base_color[1],
                    material.base_color[2],
                ],
                (material.emission_strength > 0.0).then(|| {
                    [
                        material.emission_color[0] * material.emission_strength,
                        material.emission_color[1] * material.emission_strength,
                        material.emission_color[2] * material.emission_strength,
                    ]
                }),
            ),
            None => (ShaderProfile::Pbr, asset.base_color, None),
        };
        let unlit = matches!(profile, ShaderProfile::Unlit | ShaderProfile::Emissive);
        let triangles = if unlit {
            mesh.to_triangles_unlit()
        } else {
            mesh.to_triangles_shaded(asset.surface_shading)
        };
        index_triangles(triangles, |mut color| {
            if (color[0] - DEFAULT_GEOMETRY_COLOR[0]).abs() < 0.02
                && (color[1] - DEFAULT_GEOMETRY_COLOR[1]).abs() < 0.02
                && (color[2] - DEFAULT_GEOMETRY_COLOR[2]).abs() < 0.02
            {
                color = material_color;
            }
            if let Some(glow) = emission {
                color = [
                    (color[0] + glow[0]).min(1.0),
                    (color[1] + glow[1]).min(1.0),
                    (color[2] + glow[2]).min(1.0),
                ];
            }
            color
        })
    };
    RenderObject {
        object_id: asset.id,
        surface_revision: surface_key(project, asset, mesh, settings, samples_textures),
        edge_revision: edge_key(mesh, settings),
        mesh: render_mesh,
        texture: atlas
            .or_else(|| object_canvas(project, samples_textures, asset))
            .map(|_| asset.id),
        edges: build_edges(mesh, settings),
        transform: IDENTITY,
    }
}

fn face_fill(
    out: &mut Vec<OverlayFaceVertex>,
    mesh: &petunia_core::Mesh,
    face_index: usize,
    color: [f32; 4],
) {
    let Some(face) = mesh.faces.get(face_index) else {
        return;
    };
    if face.verts.len() < 3 {
        return;
    }
    for corners in mesh.face_triangle_corners(face_index) {
        for corner in corners {
            let Some(vertex) = face
                .verts
                .get(corner)
                .and_then(|&index| mesh.verts.get(index as usize))
            else {
                continue;
            };
            out.push(OverlayFaceVertex {
                position: vertex.pos,
                color,
            });
        }
    }
}

/// Camada de seleção e preselection do objeto ativo, no domínio atual: um
/// vértice selecionado nunca vira face pintada. Larguras e raios em px lógicos.
pub fn build_component_overlays(
    project: &Project,
    pose: Option<&PoseOverride>,
    style: &ComponentStyle,
) -> ComponentOverlays {
    let mut overlays = ComponentOverlays::default();
    let Some(asset) = project.assets.get(project.active) else {
        return overlays;
    };
    let mesh = mesh_to_draw(pose, asset);
    let selected = rgb_f32(
        project
            .selection_color_of(asset)
            .unwrap_or(style.selection_rgb),
    );
    let face_color = [selected[0], selected[1], selected[2], FACE_SELECTED_ALPHA];
    let edge_color = [selected[0], selected[1], selected[2], 1.0];
    let thickness = style.thickness;
    let segment = |a: u32, b: u32| {
        Some((
            mesh.verts.get(a as usize)?.pos,
            mesh.verts.get(b as usize)?.pos,
        ))
    };

    match style.domain {
        SelectionDomain::Edge => {
            for (a, b) in mesh.edges_unique() {
                if mesh.selected_edges.contains(&(a, b)) {
                    continue;
                }
                if let Some((a, b)) = segment(a, b) {
                    overlays.lines.push(OverlayLine {
                        a,
                        b,
                        color: EDGE_GUIDE,
                        width_px: 1.0,
                    });
                }
            }
            let mut chosen: Vec<_> = mesh.selected_edges.iter().copied().collect();
            chosen.sort_unstable();
            for (a, b) in chosen {
                if let Some((a, b)) = segment(a, b) {
                    overlays.lines.push(OverlayLine {
                        a,
                        b,
                        color: edge_color,
                        width_px: thickness.clamp(1.5, 3.0),
                    });
                }
            }
        }
        SelectionDomain::Vertex => {
            for vertex in &mesh.verts {
                // A espessura de seleção (preferência) engrossa um pouco os pontos.
                let grow = (thickness - 2.0).max(0.0) * 0.5;
                overlays.points.push(if vertex.selected {
                    OverlayPoint {
                        position: vertex.pos,
                        color: edge_color,
                        radius_px: VERTEX_SELECTED_RADIUS + grow,
                    }
                } else {
                    OverlayPoint {
                        position: vertex.pos,
                        color: VERTEX_GUIDE,
                        radius_px: VERTEX_RADIUS + grow,
                    }
                });
            }
            // Selecionados por cima das guias.
            overlays
                .points
                .sort_by(|a, b| a.radius_px.total_cmp(&b.radius_px));
        }
        SelectionDomain::Face => {
            for (index, face) in mesh.faces.iter().enumerate() {
                if face.selected {
                    face_fill(&mut overlays.faces, &mesh, index, face_color);
                    // Contorno fino na cor de seleção: a forma da face lê sem
                    // precisar de tinta forte.
                    let count = face.verts.len();
                    for corner in 0..count {
                        let (Some(&a), Some(&b)) =
                            (face.verts.get(corner), face.verts.get((corner + 1) % count))
                        else {
                            continue;
                        };
                        if let Some((a, b)) = segment(a, b) {
                            overlays.lines.push(OverlayLine {
                                a,
                                b,
                                color: edge_color,
                                width_px: thickness.clamp(1.0, 2.0),
                            });
                        }
                    }
                }
            }
        }
        SelectionDomain::Object => {}
    }

    match style.hover {
        HoverTarget::Vertex(index) => {
            if let Some(vertex) = mesh.verts.get(index as usize) {
                overlays.points.push(OverlayPoint {
                    position: vertex.pos,
                    color: HOVER_LINE,
                    radius_px: VERTEX_HOVER_RADIUS + (thickness - 2.0).max(0.0) * 0.5,
                });
            }
        }
        HoverTarget::Edge(a, b) => {
            if let Some((a, b)) = segment(a, b) {
                overlays.lines.push(OverlayLine {
                    a,
                    b,
                    color: HOVER_LINE,
                    width_px: thickness.clamp(1.5, 2.5),
                });
            }
        }
        HoverTarget::Face(index) => face_fill(&mut overlays.faces, &mesh, index, HOVER_FACE),
        HoverTarget::Object(_) | HoverTarget::None => {}
    }
    // A malha é local ao objeto (P01/G10): os overlays vão para o mundo pela
    // mesma matriz que o renderer usa na superfície.
    let world = asset.world_matrix();
    if world != Mat4::IDENTITY {
        let to_world = |p: [f32; 3]| world.transform_point3(glam::Vec3::from(p)).to_array();
        for vertex in &mut overlays.faces {
            vertex.position = to_world(vertex.position);
        }
        for line in &mut overlays.lines {
            line.a = to_world(line.a);
            line.b = to_world(line.b);
        }
        for point in &mut overlays.points {
            point.position = to_world(point.position);
        }
    }
    overlays
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlays::EdgeMode;
    use petunia_mesh::{Face, Vertex};

    #[test]
    fn identical_vertices_share_one_index() {
        let tri = |p: [f32; 3]| (p, [0.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 0.0]);
        let mesh = index_triangles(
            [
                tri([0.0, 0.0, 0.0]),
                tri([1.0, 0.0, 0.0]),
                tri([0.0, 1.0, 0.0]),
                tri([1.0, 0.0, 0.0]),
                tri([1.0, 1.0, 0.0]),
                tri([0.0, 1.0, 0.0]),
            ],
            |color| color,
        );
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 1, 3, 2]);
    }

    #[test]
    fn recolor_applies_before_deduplication() {
        let tri = |p: [f32; 3]| (p, [0.0, 0.0, 1.0], [0.72, 0.73, 0.78], [0.0, 0.0]);
        let mesh = index_triangles(
            [tri([0.0; 3]), tri([1.0, 0.0, 0.0]), tri([0.0, 1.0, 0.0])],
            |_| [0.1, 0.2, 0.3],
        );
        assert_eq!(mesh.vertices[0].color, [0.1, 0.2, 0.3, 1.0]);
    }

    #[test]
    fn overlay_key_changes_with_flags_only() {
        let base = ViewportRenderSettings::default();
        let mut toggled = base;
        toggled.show_grid = false;
        assert_ne!(overlay_key(&base), overlay_key(&toggled));
        assert_eq!(overlay_key(&base), overlay_key(&base));
    }

    fn viewport() -> PhysicalViewport {
        PhysicalViewport {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        }
    }

    /// Dois quads coplanares lado a lado: a aresta do meio não é de feição.
    fn two_quads() -> petunia_core::Mesh {
        let mut mesh = petunia_core::Mesh::default();
        for [x, y] in [
            [-2.0f32, -1.0],
            [0.0, -1.0],
            [2.0, -1.0],
            [2.0, 1.0],
            [0.0, 1.0],
            [-2.0, 1.0],
        ] {
            mesh.verts.push(Vertex::new(x, y, 0.0));
        }
        mesh.faces.push(Face::new(vec![0, 1, 4, 5]));
        mesh.faces.push(Face::new(vec![1, 2, 3, 4]));
        mesh
    }

    #[test]
    fn draw_hides_flat_edges_and_poly_with_overlay_shows_them_thinner() {
        let mesh = two_quads();
        let mut settings = ViewportRenderSettings {
            edge_mode: EdgeMode::Features,
            ..ViewportRenderSettings::default()
        };
        let features = build_edges(&mesh, &settings);
        // 6 de borda (feição), a do meio (plana) some.
        assert_eq!(features.len(), 6);
        settings.edge_mode = EdgeMode::Topology;
        settings.show_wireframe_overlay = true;
        let topology = build_edges(&mesh, &settings);
        assert_eq!(topology.len(), 7);
        assert_eq!(topology.iter().filter(|e| e.width < 1.0).count(), 1);
        settings.edge_mode = EdgeMode::Overlay;
        settings.show_wireframe_overlay = false;
        assert!(build_edges(&mesh, &settings).is_empty());
    }

    #[test]
    fn edge_normals_come_from_the_two_neighbour_faces() {
        let mesh = two_quads();
        let key = position_key([0.0, -1.0, 0.0], [0.0, 1.0, 0.0]);
        let adjacency = edge_face_normals(&mesh);
        let normals = adjacency.get(&key).copied().expect("aresta do meio");
        assert_eq!(normals[0], [0.0, 0.0, 1.0]);
        assert_eq!(normals[1], [0.0, 0.0, 1.0]);
        // A chave não depende da ordem dos extremos.
        assert_eq!(position_key([0.0, 1.0, 0.0], [0.0, -1.0, 0.0]), key);

        // O overlay entrega as normais à aresta que separa as duas faces.
        let settings = ViewportRenderSettings {
            edge_mode: EdgeMode::Topology,
            show_wireframe_overlay: true,
            ..ViewportRenderSettings::default()
        };
        let edges = build_edges(&mesh, &settings);
        let middle = edges
            .iter()
            .find(|edge| position_key(edge.a, edge.b) == key)
            .expect("aresta do meio no overlay");
        assert_eq!(middle.normals, normals);
        // Bordas abertas ficam sem normal: sempre desenhadas, mesmo sem Raio-X.
        assert_eq!(
            edges
                .iter()
                .filter(|edge| edge.normals == [[0.0; 3]; 2])
                .count(),
            6
        );
    }

    #[test]
    fn closed_mesh_knows_both_faces_and_non_manifold_edges_draw_always() {
        let corners = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let mut cube = petunia_core::Mesh::default();
        for corner in corners {
            cube.verts
                .push(Vertex::new(corner[0], corner[1], corner[2]));
        }
        for face in [
            [0usize, 1, 2, 3],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ] {
            cube.faces
                .push(Face::new(face.iter().map(|index| *index as u32).collect()));
        }
        let adjacency = edge_face_normals(&cube);
        assert_eq!(adjacency.len(), 12, "cubo: 12 arestas");
        assert!(
            adjacency
                .values()
                .all(|normals| normals[0] != [0.0; 3] && normals[1] != [0.0; 3]),
            "cubo fechado: toda aresta tem as duas faces"
        );

        // Uma terceira face na mesma aresta é não-manifold: sem normais, sempre desenhada.
        cube.faces.push(Face::new(vec![0, 1, 2]));
        let adjacency = edge_face_normals(&cube);
        assert_eq!(
            adjacency[&position_key(corners[0], corners[1])],
            [[0.0; 3]; 2]
        );
    }

    #[test]
    fn camera_and_selection_do_not_rebuild_the_surface() {
        let mut project = Project::default();
        project.add("Quads", two_quads());
        let mut camera = Camera::default();
        let mut extractor = SceneExtractor::new();
        let revision = {
            let frame = extractor.extract_parts(SceneParts::new(&project, &camera, viewport()));
            frame.objects[0].surface_revision
        };
        let rebuilt = extractor.object_rebuild_count();
        camera.orbit(0.3, 0.1);
        let _ = extractor.extract_parts(SceneParts::new(&project, &camera, viewport()));
        assert_eq!(
            extractor.object_rebuild_count(),
            rebuilt,
            "câmera não refaz"
        );

        let components_before = extractor.components_revision();
        project.active_mesh_mut().unwrap().faces[0].selected = true;
        let mut parts = SceneParts::new(&project, &camera, viewport());
        parts.components.domain = SelectionDomain::Face;
        let frame = extractor.extract_parts(parts);
        assert_eq!(frame.objects[0].surface_revision, revision);
        assert_eq!(
            frame.components.faces.len(),
            6,
            "quad selecionado = 2 triângulos"
        );
        assert_eq!(
            extractor.object_rebuild_count(),
            rebuilt,
            "seleção não refaz"
        );
        assert_ne!(extractor.components_revision(), components_before);
    }

    #[test]
    fn hover_changes_only_the_component_layer() {
        let mut project = Project::default();
        project.add("Quads", two_quads());
        let camera = Camera::default();
        let mut extractor = SceneExtractor::new();
        let _ = extractor.extract_parts(SceneParts::new(&project, &camera, viewport()));
        let rebuilt = extractor.object_rebuild_count();
        let mut parts = SceneParts::new(&project, &camera, viewport());
        parts.components.hover = HoverTarget::Vertex(0);
        let frame = extractor.extract_parts(parts);
        assert_eq!(frame.components.points.len(), 1);
        assert_eq!(extractor.object_rebuild_count(), rebuilt);
    }

    #[test]
    fn rigid_preview_moves_by_matrix_without_retriangulating() {
        let mut project = Project::default();
        project.add("Quads", two_quads());
        let id = project.assets[0].id;
        let camera = Camera::default();
        let mut extractor = SceneExtractor::new();
        let first = Mat4::from_translation(glam::Vec3::X);
        let previews = [(id, first)];
        let mut parts = SceneParts::new(&project, &camera, viewport());
        parts.rigid_previews = &previews;
        let revision = extractor.extract_parts(parts).objects[0].surface_revision;
        let rebuilt = extractor.object_rebuild_count();
        // A malha do documento se move, a matriz acompanha.
        for vertex in &mut project.active_mesh_mut().unwrap().verts {
            vertex.pos[0] += 1.0;
        }
        let second = Mat4::from_translation(glam::Vec3::X * 2.0);
        let previews = [(id, second)];
        let mut parts = SceneParts::new(&project, &camera, viewport());
        parts.rigid_previews = &previews;
        let (surface_revision, transform) = {
            let frame = extractor.extract_parts(parts);
            (
                frame.objects[0].surface_revision,
                frame.objects[0].transform,
            )
        };
        assert_eq!(surface_revision, revision);
        assert_eq!(extractor.object_rebuild_count(), rebuilt);
        let relative = Mat4::from_cols_array(&transform);
        assert!((relative.w_axis.x - 1.0).abs() < 1e-5, "{relative:?}");
    }

    #[test]
    fn object_transform_moves_by_model_matrix_without_retriangulating() {
        let mut project = Project::default();
        project.add("Quads", two_quads());
        let camera = Camera::default();
        let mut extractor = SceneExtractor::new();
        let first = extractor.extract_parts(SceneParts::new(&project, &camera, viewport()));
        assert_eq!(first.objects[0].transform, IDENTITY);
        let revision = first.objects[0].surface_revision;
        let rebuilt = extractor.object_rebuild_count();
        project.assets[0].transform.translation = [0.0, 2.0, 0.0];
        let (surface, transform) = {
            let frame = extractor.extract_parts(SceneParts::new(&project, &camera, viewport()));
            (
                frame.objects[0].surface_revision,
                frame.objects[0].transform,
            )
        };
        assert_eq!(surface, revision, "a superfície local não muda");
        assert_eq!(
            extractor.object_rebuild_count(),
            rebuilt,
            "sem triangular de novo"
        );
        assert_eq!(Mat4::from_cols_array(&transform).w_axis.y, 2.0);
    }

    #[test]
    fn outline_active_must_be_selected() {
        let project = Project::new();
        let id = project.assets[0].id;
        let style = ComponentStyle::default();
        let outline = resolve_outline(&project, &style, &[], Some(id));
        assert!(outline.entries.is_empty());
        let outline = resolve_outline(&project, &style, &[id], Some(id));
        assert_eq!(outline.entries.len(), 1);
        assert_eq!(outline.entries[0].object_id, id);
    }

    #[test]
    fn outline_uses_each_parts_own_color_and_the_active_one_last() {
        let mut project = Project::new();
        let first = project.assets[0].id;
        project.add("Segundo", petunia_core::Mesh::cube(1.0));
        let second = project.assets[1].id;
        project.assets[0].selection_overlay_color = Some([255, 0, 0]);
        project.assets[1].selection_overlay_color = Some([0, 0, 255]);
        let style = ComponentStyle::default();
        let outline = resolve_outline(&project, &style, &[first, second], Some(first));
        assert_eq!(outline.entries.len(), 2);
        assert_eq!(outline.entries[0].object_id, second);
        assert!(outline.entries[0].color[2] > outline.entries[0].color[0]);
        assert_eq!(outline.entries[1].object_id, first);
        assert!(outline.entries[1].color[0] > outline.entries[1].color[2]);
    }

    #[test]
    fn outline_uses_the_group_color_unless_the_object_has_its_own() {
        let mut project = Project::new();
        let first = project.assets[0].id;
        project.add("Segundo", petunia_core::Mesh::cube(1.0));
        let second = project.assets[1].id;
        project.add_collection("Rodas");
        for asset in &mut project.assets {
            asset.collection = Some("Rodas".into());
        }
        project.set_collection_color("Rodas", Some([0, 255, 0]), false);
        project.assets[0].selection_overlay_color = Some([255, 0, 0]);
        let style = ComponentStyle::default();
        let green_dominant = |color: [f32; 4]| color[1] > color[0] && color[1] > color[2];
        let red_dominant = |color: [f32; 4]| color[0] > color[1] && color[0] > color[2];
        let outline = resolve_outline(&project, &style, &[first, second], Some(first));
        let color_of = |id| {
            outline
                .entries
                .iter()
                .find(|entry| entry.object_id == id)
                .map(|entry| entry.color)
                .expect("entrada do objeto")
        };
        assert!(
            red_dominant(color_of(first)),
            "a cor própria vence por padrão"
        );
        assert!(green_dominant(color_of(second)), "o outro herda a do grupo");

        // Forçar: o grupo vence a cor própria.
        project.set_collection_color("Rodas", Some([0, 255, 0]), true);
        let forced = resolve_outline(&project, &style, &[first, second], Some(first));
        assert!(
            forced
                .entries
                .iter()
                .all(|entry| green_dominant(entry.color))
        );
    }

    #[test]
    fn rendered_without_scene_light_falls_back_to_studio() {
        let mut project = Project::default();
        // O projeto padrão traz uma luz: sem ela, o Rendered usa o estúdio.
        project.lights.clear();
        let camera = Camera::default();
        let light = resolve_light(&project, &camera, Shading::Rendered, false);
        assert_eq!(light.ambient, scene::LIGHT_AMBIENT);
        let length: f32 = light.direction.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-4);
    }
}
