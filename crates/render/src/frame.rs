//! Boundary neutro Application → Renderer.
//!
//! Contrato de `website/docs/04-renderer/architecture-decision.md` (“Boundary
//! principal”): o renderer **não** recebe `AppState`, `Project`, seleção nem
//! preferências. Ele recebe apenas este snapshot, já resolvido (triangulação,
//! normais, cor de material, quads de referência, paleta, luz, overlays de
//! componente e gizmo).
//!
//! Quem produz o snapshot é o [`crate::extract::SceneExtractor`] — o único
//! ponto do crate que conhece `AppState`/`Project`.

use std::sync::Arc;

use petunia_core::viewport::PhysicalViewport;
use uuid::Uuid;

use crate::Shading;
use crate::overlays::{EdgeMode, OverlayShape, WorkplaneOverlay};

/// Cores concretas do viewport (tokens do tema já resolvidos).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPalette {
    /// Cor de fundo (RGBA linear de 0 a 1).
    pub background: [f32; 4],
}

impl Default for ViewportPalette {
    fn default() -> Self {
        Self {
            background: [0.117, 0.117, 0.133, 1.0],
        }
    }
}

/// Parâmetros do grid global.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderGrid {
    pub size: f32,
    pub spacing: f32,
    pub opacity: f32,
    pub show_isometric_guide: bool,
    pub isometric_angle_deg: f32,
}

/// Matrizes e base da câmera já resolvidas. A projeção segue a convenção do
/// `glam` (`*_rh`, profundidade em `[0, 1]`); o renderer GL remapeia para o
/// clip `[-1, 1]` no vertex shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraMatrices {
    pub view_proj: [f32; 16],
    pub proj: [f32; 16],
    pub inv_proj: [f32; 16],
    pub eye: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub orthographic: bool,
    /// Altura visível no plano focal, em unidades de mundo.
    pub visible_height: f32,
    /// Distância do olho ao alvo (escala de perspectiva dos overlays).
    pub distance: f32,
}

impl CameraMatrices {
    pub fn from_camera(camera: &petunia_core::Camera) -> Self {
        let proj = camera.proj();
        Self {
            view_proj: camera.view_proj().to_cols_array(),
            proj: proj.to_cols_array(),
            inv_proj: proj.inverse().to_cols_array(),
            eye: camera.eye().to_array(),
            forward: camera.forward().to_array(),
            right: camera.right().to_array(),
            up: camera.up().to_array(),
            orthographic: camera.proj == petunia_core::Projection::Ortho,
            visible_height: camera.visible_height(),
            distance: camera.distance,
        }
    }

    /// Câmera identidade (testes e frames sintéticos).
    pub fn from_view_proj(view_proj: [f32; 16]) -> Self {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        Self {
            view_proj,
            proj: identity,
            inv_proj: identity,
            eye: [0.0, 0.0, 1.0],
            forward: [0.0, 0.0, -1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            orthographic: true,
            visible_height: 2.0,
            distance: 1.0,
        }
    }
}

/// Luz já resolvida para o frame (estúdio, estúdio presa à câmera ou luz da cena).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderLight {
    /// Direção normalizada **para** a luz.
    pub direction: [f32; 3],
    pub ambient: f32,
    pub diffuse: f32,
}

impl Default for RenderLight {
    fn default() -> Self {
        let [x, y, z] = crate::scene::LIGHT_DIR;
        let length = (x * x + y * y + z * z).sqrt().max(1e-6);
        Self {
            direction: [x / length, y / length, z / length],
            ambient: crate::scene::LIGHT_AMBIENT,
            diffuse: crate::scene::LIGHT_DIFFUSE,
        }
    }
}

/// Flags de apresentação que alteram o desenho (nunca a câmera).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRenderSettings {
    pub shading: Shading,
    pub xray: bool,
    /// Alpha da geometria em X-Ray.
    pub xray_opacity: f32,
    pub show_triangulation: bool,
    pub show_wireframe_overlay: bool,
    pub show_face_orientation: bool,
    pub show_uv_checker: bool,
    /// Amostrar a textura do objeto/material em vez da cor.
    pub textured: bool,
    /// Modo de edição de componentes.
    pub edit_mode_is_edit: bool,
    pub show_overlays: bool,
    pub show_grid: bool,
    pub show_axes: bool,
    pub grid: RenderGrid,
    /// Leitura das arestas por modo (DRAW = feições, POLY = topologia).
    pub edge_mode: EdgeMode,
    /// Largura base das arestas em px lógicos.
    pub line_width_px: f32,
    /// Px físicos do alvo por px lógico (HiDPI/escala da UI).
    pub pixel_ratio: f32,
    /// Matcap procedural no Solid.
    pub matcap: bool,
    /// Oclusão ambiente pedida (o renderer só aplica em Solid/Material sem X-Ray).
    pub ambient_occlusion: bool,
    /// Amostras de MSAA pedidas (1 = desligado); o renderer limita à capacidade.
    pub msaa_samples: u32,
    pub light: RenderLight,
}

impl ViewportRenderSettings {
    /// AO vale neste frame: faces iluminadas e opacas.
    pub fn ambient_occlusion_active(&self) -> bool {
        self.ambient_occlusion
            && !self.xray
            && matches!(self.shading, Shading::Solid | Shading::MaterialPreview)
    }
}

impl Default for ViewportRenderSettings {
    fn default() -> Self {
        Self {
            shading: Shading::Solid,
            xray: false,
            xray_opacity: 0.45,
            show_triangulation: false,
            show_wireframe_overlay: false,
            show_face_orientation: false,
            show_uv_checker: false,
            textured: false,
            edit_mode_is_edit: false,
            show_overlays: true,
            show_grid: true,
            show_axes: true,
            grid: RenderGrid {
                size: 10.0,
                spacing: 1.0,
                opacity: 0.5,
                show_isometric_guide: false,
                isometric_angle_deg: 30.0,
            },
            edge_mode: EdgeMode::Overlay,
            line_width_px: DEFAULT_LINE_WIDTH_PX,
            pixel_ratio: 1.0,
            matcap: false,
            ambient_occlusion: false,
            msaa_samples: 4,
            light: RenderLight::default(),
        }
    }
}

/// Largura padrão das arestas em px lógicos.
pub const DEFAULT_LINE_WIDTH_PX: f32 = 1.5;

/// Vértice de render já resolvido (posição, normal de canto, cor e UV por canto).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderVertex {
    pub position: [f32; 3],
    /// Normal nula = unlit (o shader usa a cor direta).
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub texture_coordinates: [f32; 2],
}

/// Malha triangulada derivada; nunca é persistida.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderMesh {
    pub vertices: Vec<RenderVertex>,
    /// Lista de triângulos (múltiplo de 3).
    pub indices: Vec<u32>,
}

/// Aresta do overlay de topologia/feição, cor e largura já resolvidas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderEdge {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub color: [f32; 3],
    /// Multiplicador de [`ViewportRenderSettings::line_width_px`].
    pub width: f32,
}

/// Objeto desenhável. Selecionar ou mover a câmera não altera as revisões.
#[derive(Debug, Clone)]
pub struct RenderObject {
    pub object_id: Uuid,
    /// Muda somente quando a superfície (vértices/índices/cores/UV) muda.
    pub surface_revision: u64,
    /// Muda quando as arestas/diagonais do objeto mudam.
    pub edge_revision: u64,
    pub mesh: RenderMesh,
    /// Textura a amostrar: `RenderTexture::id` em `RenderFrameInput::textures`.
    pub texture: Option<Uuid>,
    pub edges: Vec<RenderEdge>,
    /// Matriz de modelo (coluna-maior). Identidade para malhas em mundo;
    /// transformações rígidas em andamento usam a matriz sem refazer a malha.
    pub transform: [f32; 16],
}

pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Pixels RGBA8 de uma textura de objeto, emprestados do documento.
#[derive(Debug, Clone)]
pub struct RenderTexture<'a> {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
    /// Identidade do conteúdo; o renderer só reenvia quando muda.
    pub revision: u64,
    pub pixels: &'a [u8],
    /// Regiões alteradas desde a revisão anterior (`None` = envio integral).
    pub dirty: Option<Vec<DirtyRect>>,
}

/// Retângulo de pixels alterados numa textura (origem no topo-esquerdo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DirtyRect {
    pub fn clipped(self, width: u32, height: u32) -> Option<Self> {
        let x = self.x.min(width);
        let y = self.y.min(height);
        let right = self.x.saturating_add(self.width).min(width);
        let bottom = self.y.saturating_add(self.height).min(height);
        (x < right && y < bottom).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

/// Imagem de referência já posicionada no mundo.
#[derive(Debug, Clone, Copy)]
pub struct RenderReferenceImage<'a> {
    pub width: u32,
    pub height: u32,
    pub revision: u64,
    pub rgba: &'a [u8],
    /// Quatro cantos no mundo (sentido anti-horário a partir de baixo-esquerda).
    pub quad: [[f32; 3]; 4],
    pub opacity: f32,
    pub visible: bool,
    /// Desenhada por cima da malha, sem depth test.
    pub xray: bool,
}

/// Vértice de face do overlay de componentes (preenchimento translúcido).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayFaceVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

/// Segmento do overlay em mundo com largura constante em px lógicos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayLine {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub color: [f32; 4],
    pub width_px: f32,
}

/// Marcador de ponto em mundo, disco de raio constante em px lógicos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayPoint {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub radius_px: f32,
}

/// Seleção e preselection de componentes do objeto ativo (buffers próprios:
/// trocar seleção nunca reenvia a `RenderMesh`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentOverlays {
    /// Muda quando o conteúdo muda; o renderer só reenvia nesse caso.
    pub revision: u64,
    /// Lista de triângulos (múltiplo de 3).
    pub faces: Vec<OverlayFaceVertex>,
    pub lines: Vec<OverlayLine>,
    pub points: Vec<OverlayPoint>,
}

impl ComponentOverlays {
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty() && self.lines.is_empty() && self.points.is_empty()
    }
}

/// Contorno de seleção de objetos (domínio Object), em largura constante.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderOutline {
    pub selected: Vec<Uuid>,
    /// Ativo, desenhado com `active_color` (precisa estar em `selected`).
    pub active: Option<Uuid>,
    pub color: [f32; 4],
    pub active_color: [f32; 4],
    /// Raio em px lógicos.
    pub radius_px: f32,
}

/// Snapshot completo de um frame.
#[derive(Debug, Clone)]
pub struct RenderFrameInput<'a> {
    pub viewport: PhysicalViewport,
    /// Altura do framebuffer em pixels físicos (origem GL é embaixo).
    pub framebuffer_height: u32,
    pub camera: CameraMatrices,
    pub palette: ViewportPalette,
    pub settings: ViewportRenderSettings,
    /// Chave de invalidação do grid/eixos.
    pub overlay_revision: u64,
    pub objects: &'a [RenderObject],
    pub textures: Vec<RenderTexture<'a>>,
    pub references: Vec<RenderReferenceImage<'a>>,
    pub components: &'a ComponentOverlays,
    pub outline: &'a RenderOutline,
    pub workplane: Option<WorkplaneOverlay>,
    /// Gizmo/ferramentas em px lógicos a partir do topo-esquerdo do viewport.
    pub screen_overlay: &'a [OverlayShape],
}

/// Textura com pixels compartilhados (frame que atravessa threads/callbacks).
#[derive(Debug, Clone)]
pub struct OwnedTexture {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
    pub revision: u64,
    pub pixels: Arc<Vec<u8>>,
    pub dirty: Option<Vec<DirtyRect>>,
}

/// Imagem de referência com pixels compartilhados.
#[derive(Debug, Clone)]
pub struct OwnedReference {
    pub width: u32,
    pub height: u32,
    pub revision: u64,
    pub rgba: Arc<Vec<u8>>,
    pub quad: [[f32; 3]; 4],
    pub opacity: f32,
    pub visible: bool,
    pub xray: bool,
}

/// Frame sem empréstimos: pode ser guardado e desenhado depois, quando o contexto
/// GL estiver corrente (o notifier do Slint). Pixels grandes são compartilhados por
/// `Arc` e só copiados quando a revisão muda.
#[derive(Debug, Clone)]
pub struct OwnedRenderFrame {
    pub viewport: PhysicalViewport,
    pub framebuffer_height: u32,
    pub camera: CameraMatrices,
    pub palette: ViewportPalette,
    pub settings: ViewportRenderSettings,
    pub overlay_revision: u64,
    pub objects: Arc<Vec<RenderObject>>,
    pub textures: Vec<OwnedTexture>,
    pub references: Vec<OwnedReference>,
    pub components: Arc<ComponentOverlays>,
    pub outline: RenderOutline,
    pub workplane: Option<WorkplaneOverlay>,
    pub screen_overlay: Vec<OverlayShape>,
}

impl OwnedRenderFrame {
    /// Visão emprestada para o renderer.
    pub fn as_input(&self) -> RenderFrameInput<'_> {
        RenderFrameInput {
            viewport: self.viewport,
            framebuffer_height: self.framebuffer_height,
            camera: self.camera,
            palette: self.palette,
            settings: self.settings,
            overlay_revision: self.overlay_revision,
            objects: &self.objects,
            textures: self
                .textures
                .iter()
                .map(|t| RenderTexture {
                    id: t.id,
                    width: t.width,
                    height: t.height,
                    revision: t.revision,
                    pixels: &t.pixels,
                    dirty: t.dirty.clone(),
                })
                .collect(),
            references: self
                .references
                .iter()
                .map(|r| RenderReferenceImage {
                    width: r.width,
                    height: r.height,
                    revision: r.revision,
                    rgba: &r.rgba,
                    quad: r.quad,
                    opacity: r.opacity,
                    visible: r.visible,
                    xray: r.xray,
                })
                .collect(),
            components: &self.components,
            outline: &self.outline,
            workplane: self.workplane,
            screen_overlay: &self.screen_overlay,
        }
    }

    /// Frame mínimo (fundo, sem objetos) para testes e inicialização.
    pub fn empty(viewport: PhysicalViewport, camera: CameraMatrices) -> Self {
        Self {
            viewport,
            framebuffer_height: viewport.height,
            camera,
            palette: ViewportPalette::default(),
            settings: ViewportRenderSettings::default(),
            overlay_revision: 0,
            objects: Arc::new(Vec::new()),
            textures: Vec::new(),
            references: Vec::new(),
            components: Arc::new(ComponentOverlays::default()),
            outline: RenderOutline::default(),
            workplane: None,
            screen_overlay: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_is_opaque() {
        assert_eq!(ViewportPalette::default().background[3], 1.0);
    }

    #[test]
    fn ambient_occlusion_only_for_opaque_lit_modes() {
        let mut settings = ViewportRenderSettings {
            ambient_occlusion: true,
            ..ViewportRenderSettings::default()
        };
        assert!(settings.ambient_occlusion_active());
        settings.xray = true;
        assert!(!settings.ambient_occlusion_active());
        settings.xray = false;
        settings.shading = Shading::Wireframe;
        assert!(!settings.ambient_occlusion_active());
        settings.shading = Shading::Rendered;
        assert!(!settings.ambient_occlusion_active());
    }

    #[test]
    fn dirty_rect_is_clipped_to_the_texture() {
        let rect = DirtyRect {
            x: 60,
            y: 2,
            width: 10,
            height: 3,
        };
        assert_eq!(
            rect.clipped(64, 64),
            Some(DirtyRect {
                x: 60,
                y: 2,
                width: 4,
                height: 3
            })
        );
        assert_eq!(rect.clipped(32, 32), None);
    }
}
