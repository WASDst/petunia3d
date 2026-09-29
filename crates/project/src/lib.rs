//! Petunia3D — projeto autocontido: assets com UUID persistente,
//! serialização versionada (postcard) e exportação (OBJ, glTF/GLB).

use petunia_mesh::Mesh;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domínios persistentes que uma operação alterou.
///
/// O bitset mantém a invalidação independente de toolkit e permite que o owner
/// transacional avance somente as revisões realmente afetadas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectChanges(u16);

impl ProjectChanges {
    pub const NONE: Self = Self(0);
    pub const TOPOLOGY: Self = Self(1 << 0);
    pub const POSITIONS: Self = Self(1 << 1);
    pub const SELECTION: Self = Self(1 << 2);
    pub const UVS: Self = Self(1 << 3);
    pub const MATERIALS: Self = Self(1 << 4);
    pub const TEXTURES: Self = Self(1 << 5);
    pub const TRANSFORMS: Self = Self(1 << 6);
    pub const NORMALS: Self = Self(1 << 7);
    pub const COLORS: Self = Self(1 << 8);
    pub const SPLINES: Self = Self(1 << 9);
    pub const PROCEDURAL: Self = Self(1 << 10);
    pub const GEOMETRY: Self =
        Self(Self::TOPOLOGY.0 | Self::POSITIONS.0 | Self::NORMALS.0 | Self::UVS.0 | Self::COLORS.0);
    pub const ALL: Self = Self(
        Self::GEOMETRY.0
            | Self::SELECTION.0
            | Self::MATERIALS.0
            | Self::TEXTURES.0
            | Self::TRANSFORMS.0
            | Self::SPLINES.0
            | Self::PROCEDURAL.0,
    );

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for ProjectChanges {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for ProjectChanges {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

pub mod animation;
pub mod autosave;
pub mod export;
pub mod format;
pub mod import_gltf;
pub mod import_obj;
pub mod io_atomic;
pub mod material;
pub mod model_library;
pub mod package;
pub mod paint_layers;
pub mod palette;
pub mod path_generator;
pub mod pipeline;
pub mod profile;
pub mod rig;
pub mod spline;
pub mod surface_attachment;
pub mod surface_recipe;

pub use animation::{
    AnimationAsset, AnimationClip, AnimationLibrary, BoneTrack, Interpolation, Keyframe,
    RetargetProfile, RigPreset, auto_fit_humanoid, compute_auto_skin_weights,
};
pub use autosave::{AutosaveConfig, AutosaveService, RecoveryInfo, SessionLockInfo};
pub use export::{ExportError, export_gltf, export_obj};
pub use import_gltf::{GlbMeshes, GltfImportError, GltfSummary, import_glb_bytes, parse_gltf_json};
pub use import_obj::{ObjImportError, import_obj_bytes};
pub use io_atomic::{AtomicIoError, TempScope, atomic_write};
pub use material::{AlphaMode, Material, ShaderProfile, TextureChannel};
pub use model_library::{AssetSummary, ModelLibraryQuery, ModelLibraryService, ModelLibrarySort};
pub use package::{
    Attachment, PACKAGE_VERSION, PackageError, PackageManifest, open_package, open_package_bytes,
    save_package, save_package_bytes,
};
pub use paint_layers::{
    DecalLayer, LayerBlendMode, LayerKind, PaintEffect, PaintLayer, PaintLayerStack, apply_effect,
    blend_pixels,
};
pub use palette::{export_gpl, export_hex, import_gpl, import_hex, preset_gameboy, preset_pico8};
pub use path_generator::{
    MAX_GENERATED_VERTICES, PathGenerator, PathGeneratorDependencies, PathGeneratorDiagnostics,
    PathGeneratorError, PathGeneratorEvaluation, PathGeneratorEvaluationCache, PathGeneratorKind,
    PathGeneratorQuality, PathGeneratorWarning, SweepGeneratorParameters,
};
pub use pipeline::{
    BatchExportReport, DeliveryPipeline, ExportOptions, ExportReport, FileFormat,
    FormatCapabilities, FormatExporter, FormatImporter, ImportOptions, ImportPayload,
    PipelineError,
};
pub use profile::{ProfileError, ProfileResource, ProfileWorkplane};
pub use rig::{Bone, RigError, Skeleton, SkinData, Transform3D, VertexSkinWeight};
pub use spline::{
    ArcLengthTable, SplineError, SplineEvaluationCache, SplineFrame, SplineHandleMode,
    SplineInterpolation, SplinePoint, SplineResource, SplineSample, SplineSnapSettings,
    snap_spline_position,
};
pub use surface_attachment::{
    SurfaceAttachment, SurfaceAttachmentError, SurfaceAttachmentStatus, SurfaceFrame, SurfaceHit,
    SurfaceTriangleHandle, detach_surface_attachment_keep_world, evaluate_surface_attachment,
    project_ray_to_surface, project_ray_to_surface_target, reproject_surface_attachment,
    reproject_surface_attachment_to_target, slide_surface_attachment, surface_attachment_status,
};
pub use surface_recipe::{
    NodeSpec, RECIPE_SCHEMA_VERSION, RecipeEdge, RecipeError, RecipeNode, RecipeOutputChannel,
    RecipeResult, SocketType, SocketValue, SurfaceRecipe,
};

/// Canvas de textura simples (albedo) por asset — workspace PAINT.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    /// RGBA8 row-major, origem em cima.
    #[serde(with = "pixel_bytes")]
    pub pixels: Vec<u8>,
}

/// Serialização compacta de pixels: base64 em formatos legíveis (JSON, ~1,33×)
/// e bytes crus em binários (postcard). A leitura aceita também o array de
/// números dos arquivos antigos, então projetos existentes continuam abrindo.
mod pixel_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::de::{self, SeqAccess, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&STANDARD.encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    struct BytesVisitor;

    impl<'de> Visitor<'de> for BytesVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("base64 string, byte array or sequence of u8")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Vec<u8>, E> {
            STANDARD.decode(v).map_err(E::custom)
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Vec<u8>, E> {
            Ok(v.to_vec())
        }

        fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Vec<u8>, E> {
            Ok(v)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
            let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(1 << 24));
            while let Some(byte) = seq.next_element::<u8>()? {
                out.push(byte);
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(BytesVisitor)
        } else {
            // Postcard não é auto-descritivo: `serialize_bytes` grava
            // varint + bytes, idêntico ao layout de `Vec<u8>` antigo.
            deserializer.deserialize_seq(BytesVisitor)
        }
    }
}

impl Canvas {
    pub fn new(w: u32, h: u32, fill: [u8; 4]) -> Self {
        let w = w.clamp(1, 1024);
        let h = h.clamp(1, 1024);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for i in (0..pixels.len()).step_by(4) {
            pixels[i..i + 4].copy_from_slice(&fill);
        }
        Self { w, h, pixels }
    }

    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let i = ((y * self.w + x) * 4) as usize;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }

    pub fn set(&mut self, x: u32, y: u32, c: [u8; 4]) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        self.pixels[i..i + 4].copy_from_slice(&c);
    }

    pub fn fill(&mut self, c: [u8; 4]) {
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i..i + 4].copy_from_slice(&c);
        }
    }

    /// Redimensiona por vizinho mais próximo (operação explícita de resize;
    /// preserva o conteúdo proporcionalmente, sem filtros caros).
    pub fn resized(&self, w: u32, h: u32) -> Self {
        let w = w.clamp(1, 1024);
        let h = h.clamp(1, 1024);
        if w == self.w && h == self.h {
            return self.clone();
        }
        let mut out = Self::new(w, h, [0, 0, 0, 0]);
        for y in 0..h {
            for x in 0..w {
                let sx = ((x as f32 * self.w as f32) / w as f32) as u32;
                let sy = ((y as f32 * self.h as f32) / h as f32) as u32;
                if let Some(px) = self.get(sx.min(self.w - 1), sy.min(self.h - 1)) {
                    out.set(x, y, px);
                }
            }
        }
        out
    }

    /// Repara canvas vindo de arquivo (M4): dims 1..1024 + pixels exatos.
    pub fn validate(&mut self) {
        self.w = self.w.clamp(1, 1024);
        self.h = self.h.clamp(1, 1024);
        let want = (self.w * self.h * 4) as usize;
        self.pixels.resize(want, 0);
    }
}

/// Operação não destrutiva persistente avaliada sobre a malha-base do asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ModifierKind {
    Mirror {
        axis: usize,
        weld: f32,
    },
    Symmetry {
        axis: usize,
        positive_to_negative: bool,
        weld: f32,
    },
}

/// Instância ordenada de modifier. O UUID mantém identidade estável para UI,
/// reordenação e futuras animações/serialization migrations.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModifierInstance {
    pub id: Uuid,
    pub enabled: bool,
    pub kind: ModifierKind,
}

impl ModifierInstance {
    pub fn mirror(axis: usize, weld: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            kind: ModifierKind::Mirror {
                axis: axis.min(2),
                weld: weld.max(0.0),
            },
        }
    }

    pub fn symmetry(axis: usize, positive_to_negative: bool, weld: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            kind: ModifierKind::Symmetry {
                axis: axis.min(2),
                positive_to_negative,
                weld: weld.max(0.0),
            },
        }
    }
}

/// Um asset do projeto. `id` nunca muda (rename seguro).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub name: String,
    pub mesh: Mesh,
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub collection: Option<String>,
    pub base_color: [f32; 3],
    pub texture: Option<Canvas>,
    /// Pilha de camadas de pintura (P3D-061). `None` = legado/sem camadas:
    /// a `texture` é a representação composta.
    #[serde(default)]
    pub paint_stack: Option<PaintLayerStack>,
    #[serde(default)]
    pub material_id: Option<Uuid>,
    #[serde(default)]
    pub skeleton_id: Option<Uuid>,
    #[serde(default)]
    pub skin_data: Option<SkinData>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub modifiers: Vec<ModifierInstance>,
    #[serde(default)]
    pub origin: Option<[f32; 3]>,
    /// Especificação paramétrica viva da primitiva (None = malha estática B-Rep).
    #[serde(default)]
    pub parametric: Option<petunia_mesh::PrimitiveDescriptor>,
    #[serde(skip)]
    eval_cache: Option<(u64, u64, Mesh)>,
}

impl Asset {
    pub fn new(name: &str, mesh: Mesh) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            mesh,
            visible: true,
            locked: false,
            collection: None,
            base_color: [0.75, 0.75, 0.78],
            texture: None,
            material_id: None,
            skeleton_id: None,
            skin_data: None,
            favorite: false,
            tags: Vec::new(),
            modifiers: Vec::new(),
            origin: None,
            parametric: None,
            paint_stack: None,
            eval_cache: None,
        }
    }

    /// Retorna verdadeiro se este asset é uma primitiva paramétrica reeditável.
    pub fn is_parametric(&self) -> bool {
        self.parametric.is_some()
    }

    /// Congela transparentemente a primitiva em malha estática B-Rep comum.
    pub fn freeze_parametric(&mut self) -> bool {
        if self.parametric.is_some() {
            self.parametric = None;
            true
        } else {
            false
        }
    }

    /// Obtém o material atribuído ao asset a partir do projeto.
    pub fn material<'a>(&self, project: &'a Project) -> Option<&'a Material> {
        self.material_id.and_then(|id| project.get_material(id))
    }

    /// Hash estável do estado completo da pilha de modificadores para invalidação de cache.
    pub fn modifier_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mix = |mut h: u64, v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x100000001b3);
            h
        };
        h = mix(h, self.modifiers.len() as u64);
        for m in &self.modifiers {
            for b in m.id.as_bytes() {
                h = mix(h, *b as u64);
            }
            h = mix(h, m.enabled as u64);
            match m.kind {
                ModifierKind::Mirror { axis, weld } => {
                    h = mix(h, 1);
                    h = mix(h, axis as u64);
                    h = mix(h, weld.to_bits() as u64);
                }
                ModifierKind::Symmetry {
                    axis,
                    positive_to_negative,
                    weld,
                } => {
                    h = mix(h, 2);
                    h = mix(h, axis as u64);
                    h = mix(h, positive_to_negative as u64);
                    h = mix(h, weld.to_bits() as u64);
                }
            }
        }
        h
    }

    fn source_mesh_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        let mix = |mut value: u64, next: u64| {
            value ^= next;
            value.wrapping_mul(0x100000001b3)
        };
        hash = mix(hash, self.mesh.verts.len() as u64);
        hash = mix(hash, self.mesh.faces.len() as u64);
        for vertex in &self.mesh.verts {
            for component in vertex.pos {
                hash = mix(hash, component.to_bits() as u64);
            }
            for component in vertex.color {
                hash = mix(hash, component.to_bits() as u64);
            }
            hash = mix(hash, vertex.selected as u64);
        }
        for face in &self.mesh.faces {
            hash = mix(hash, face.verts.len() as u64);
            for &index in &face.verts {
                hash = mix(hash, index as u64);
            }
            hash = mix(hash, face.uv.len() as u64);
            for uv in &face.uv {
                hash = mix(hash, uv[0].to_bits() as u64);
                hash = mix(hash, uv[1].to_bits() as u64);
            }
            hash = mix(hash, face.selected as u64);
            hash = mix(hash, face.material_slot.unwrap_or(usize::MAX) as u64);
        }
        let mut selected_edges: Vec<_> = self.mesh.selected_edges.iter().copied().collect();
        selected_edges.sort_unstable();
        hash = mix(hash, 0x5345_4c45_4447_4553);
        hash = mix(hash, selected_edges.len() as u64);
        for (a, b) in selected_edges {
            hash = mix(hash, a as u64);
            hash = mix(hash, b as u64);
        }
        let mut seams: Vec<_> = self.mesh.uv_seams.iter().copied().collect();
        seams.sort_unstable();
        hash = mix(hash, 0x5556_5345_414d_5300);
        hash = mix(hash, seams.len() as u64);
        for (a, b) in seams {
            hash = mix(hash, a as u64);
            hash = mix(hash, b as u64);
        }
        let mut pinned: Vec<_> = self.mesh.uv_pinned.iter().copied().collect();
        pinned.sort_unstable();
        hash = mix(hash, 0x5556_5049_4e53_0000);
        hash = mix(hash, pinned.len() as u64);
        for (face_index, corner_index) in pinned {
            hash = mix(hash, face_index as u64);
            hash = mix(hash, corner_index as u64);
        }
        hash
    }

    /// Há algum modifier habilitado que altere a malha avaliada?
    pub fn has_enabled_modifiers(&self) -> bool {
        self.modifiers.iter().any(|modifier| modifier.enabled)
    }

    /// Malha avaliada emprestada quando não há modifiers (caso comum: zero
    /// cópia e zero hash) e calculada quando há. Preferir esta API em caminhos
    /// por frame (render, seleção, picking).
    pub fn evaluated_mesh_ref(&self) -> std::borrow::Cow<'_, Mesh> {
        if self.has_enabled_modifiers() {
            std::borrow::Cow::Owned(self.evaluated_mesh())
        } else {
            std::borrow::Cow::Borrowed(&self.mesh)
        }
    }

    /// Avalia a pilha de modifiers sem alterar a malha-base.
    /// Render, preview e export usam este resultado; edição continua operando
    /// sobre `mesh`, preservando a natureza não destrutiva da pilha.
    pub fn evaluated_mesh(&self) -> Mesh {
        if !self.has_enabled_modifiers() {
            return self.mesh.clone();
        }
        let key = (self.source_mesh_hash(), self.modifier_hash());
        if let Some((k0, k1, cached)) = &self.eval_cache
            && (*k0, *k1) == key
        {
            return cached.clone();
        }
        let mut mesh = self.mesh.clone();
        for modifier in &self.modifiers {
            if !modifier.enabled {
                continue;
            }
            match modifier.kind {
                ModifierKind::Mirror { axis, weld } => mesh.mirror(axis, weld),
                ModifierKind::Symmetry {
                    axis,
                    positive_to_negative,
                    weld,
                } => {
                    mesh.symmetrize(axis, positive_to_negative, weld);
                }
            }
        }
        mesh
    }

    /// Cache-aware evaluation. Callers with `&mut Asset` reuse the last result.
    pub fn evaluated_mesh_cached(&mut self) -> &Mesh {
        let key = (self.source_mesh_hash(), self.modifier_hash());
        let miss = self
            .eval_cache
            .as_ref()
            .is_none_or(|(k0, k1, _)| (*k0, *k1) != key);
        if miss {
            let mesh = self.evaluated_mesh();
            self.eval_cache = Some((key.0, key.1, mesh));
        }
        &self.eval_cache.as_ref().unwrap().2
    }

    /// Duplicata com novo UUID.
    pub fn duplicate(&self) -> Self {
        let mut c = self.clone();
        c.id = Uuid::new_v4();
        c.name = format!("{} copy", self.name);
        c
    }

    /// Adiciona uma tag normalizada (minúscula, sem espaços extras).
    pub fn add_tag(&mut self, tag: &str) -> bool {
        let trimmed = tag.trim().to_lowercase();
        if trimmed.is_empty() || self.tags.iter().any(|t| t.to_lowercase() == trimmed) {
            return false;
        }
        self.tags.push(trimmed);
        true
    }

    /// Remove uma tag.
    pub fn remove_tag(&mut self, tag: &str) {
        let trimmed = tag.trim().to_lowercase();
        self.tags.retain(|t| t.to_lowercase() != trimmed);
    }

    /// Verifica se possui determinada tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        let trimmed = tag.trim().to_lowercase();
        self.tags.iter().any(|t| t.to_lowercase() == trimmed)
    }

    /// Alterna estado de favorito.
    pub fn toggle_favorite(&mut self) {
        self.favorite = !self.favorite;
    }
}

fn default_palette() -> Vec<[f32; 3]> {
    vec![
        [1.0, 0.2, 0.2],
        [1.0, 0.8, 0.2],
        [0.2, 0.8, 0.3],
        [0.3, 0.5, 1.0],
    ]
}

fn default_stroke_color() -> [f32; 4] {
    [0.0, 0.74, 0.83, 1.0] // Ciano característico do Blender
}

fn default_stroke_width() -> f32 {
    2.0
}

const fn default_true() -> bool {
    true
}

const fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

/// Traço de anotação livre em espaço 3D (ferramenta Annotate).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnnotationStroke {
    pub points: Vec<[f32; 3]>,
    #[serde(default = "default_stroke_color")]
    pub color: [f32; 4],
    #[serde(default = "default_stroke_width")]
    pub width: f32,
}

impl Default for AnnotationStroke {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            color: default_stroke_color(),
            width: default_stroke_width(),
        }
    }
}

/// Item de anotação pertencente à collection de Anotações do projeto.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnnotationItem {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub group: Option<String>,
    pub strokes: Vec<AnnotationStroke>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub translation: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3], // Graus de Euler XYZ
    #[serde(default = "default_scale")]
    pub scale: [f32; 3], // [1.0, 1.0, 1.0]
}

impl AnnotationItem {
    pub fn new(name: impl Into<String>, strokes: Vec<AnnotationStroke>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            group: None,
            strokes,
            visible: true,
            locked: false,
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn transform_matrix(&self) -> glam::Mat4 {
        let t = glam::Vec3::from(self.translation);
        let r = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            self.rotation[0].to_radians(),
            self.rotation[1].to_radians(),
            self.rotation[2].to_radians(),
        );
        let s = glam::Vec3::from(self.scale);
        glam::Mat4::from_scale_rotation_translation(s, r, t)
    }

    pub fn transform_point(&self, pt: [f32; 3]) -> [f32; 3] {
        let m = self.transform_matrix();
        let v = m.transform_point3(glam::Vec3::from(pt));
        [v.x, v.y, v.z]
    }

    pub fn center(&self) -> [f32; 3] {
        let mut sum = glam::Vec3::ZERO;
        let mut count = 0;
        let m = self.transform_matrix();
        for s in &self.strokes {
            for &pt in &s.points {
                sum += m.transform_point3(glam::Vec3::from(pt));
                count += 1;
            }
        }
        if count > 0 {
            let avg = sum / (count as f32);
            [avg.x, avg.y, avg.z]
        } else {
            self.translation
        }
    }
}

/// Item de medição tridimensional com distância euclidiana e deltas cartesianos.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MeasurementItem {
    pub id: Uuid,
    pub name: String,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub distance: f32,
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl MeasurementItem {
    pub fn new(name: impl Into<String>, start: [f32; 3], end: [f32; 3], distance: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            start,
            end,
            distance,
            visible: true,
        }
    }

    pub fn deltas(&self) -> [f32; 3] {
        [
            (self.end[0] - self.start[0]).abs(),
            (self.end[1] - self.start[1]).abs(),
            (self.end[2] - self.start[2]).abs(),
        ]
    }
}

fn default_project_name() -> String {
    "Untitled".to_string()
}

/// Projeto: metadados, lista de assets com UUID persistente, paleta, coleções, anotações e medições.
/// Tipo de luz de cena (P3D-134). V1 tem direcional; os demais entram depois.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LightKind {
    /// Direcional: só a direção importa, como o sol.
    #[default]
    Directional,
    /// Ponto: posição + alcance.
    Point,
}

impl LightKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Directional => "directional",
            Self::Point => "point",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "directional" => Some(Self::Directional),
            "point" => Some(Self::Point),
            _ => None,
        }
    }
}

/// Luz de cena usada pelo modo Rendered da viewport.
///
/// A direção é normalizada no renderer; `intensity` escala a contribuição
/// difusa e `color` tinge a luz. Sem luz habilitada o Rendered cai no estúdio
/// da viewport em vez de renderizar preto.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub id: Uuid,
    pub name: String,
    pub kind: LightKind,
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub enabled: bool,
}

impl Light {
    pub fn directional(name: impl Into<String>, direction: [f32; 3]) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind: LightKind::Directional,
            direction,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            enabled: true,
        }
    }

    /// Direção normalizada, caindo no eixo Y quando degenerada.
    pub fn normalized_direction(&self) -> [f32; 3] {
        let [x, y, z] = self.direction;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return [0.0, 1.0, 0.0];
        }
        let length = (x * x + y * y + z * z).sqrt();
        if length <= 1.0e-5 {
            return [0.0, 1.0, 0.0];
        }
        [x / length, y / length, z / length]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "default_project_name")]
    pub name: String,
    pub assets: Vec<Asset>,
    pub active: usize,
    /// Transient selection context carried by in-memory undo snapshots. The
    /// document file keeps selection in the editor session, not authored data.
    #[serde(skip)]
    pub history_selection: Vec<Uuid>,
    #[serde(default = "default_palette")]
    pub palette: Vec<[f32; 3]>,
    #[serde(default)]
    pub collections: Vec<String>,
    #[serde(default)]
    pub annotations: Vec<AnnotationItem>,
    #[serde(default)]
    pub annotation_groups: Vec<String>,
    #[serde(default)]
    pub measurements: Vec<MeasurementItem>,
    #[serde(default = "default_true")]
    pub annotations_visible: bool,
    #[serde(default)]
    pub annotations_locked: bool,
    #[serde(default = "default_true")]
    pub measurements_visible: bool,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub lights: Vec<Light>,
    #[serde(default)]
    pub skeletons: Vec<Skeleton>,
    #[serde(default)]
    pub animations: Vec<AnimationAsset>,
    /// Scene-level revision counters for GPU invalidation (not hashed content).
    #[serde(default)]
    pub topology_revision: u64,
    #[serde(default)]
    pub position_revision: u64,
    #[serde(default)]
    pub selection_revision: u64,
    #[serde(default)]
    pub material_revision: u64,
    #[serde(default)]
    pub texture_revision: u64,
    #[serde(default)]
    pub transform_revision: u64,
    #[serde(default)]
    pub normal_revision: u64,
    #[serde(default)]
    pub uv_revision: u64,
    #[serde(default)]
    pub color_revision: u64,
    #[serde(default)]
    pub spline_revision: u64,
    /// Append-only no schema binário legado: novos campos persistentes devem
    /// permanecer após todos os campos V1 existentes para não deslocar postcard.
    #[serde(default)]
    pub splines: Vec<SplineResource>,
    #[serde(default)]
    pub procedural_revision: u64,
    #[serde(default)]
    pub profiles: Vec<ProfileResource>,
    #[serde(default)]
    pub path_generators: Vec<PathGenerator>,
    /// Assets com sombreamento suave (normais interpoladas). Ausente = flat,
    /// o padrão do cap. 05. Fica no projeto — não no `Asset` — para não deslocar
    /// o layout postcard legado, que embute `Vec<Asset>`.
    #[serde(default)]
    pub smooth_shaded_assets: Vec<Uuid>,
}

impl Project {
    /// O asset usa sombreamento suave (Shade Smooth)?
    pub fn is_smooth_shaded(&self, asset_id: Uuid) -> bool {
        self.smooth_shaded_assets.contains(&asset_id)
    }

    /// Define Flat/Smooth de um asset. Retorna `true` quando algo mudou.
    /// Avança apenas a revisão de normais.
    pub fn set_smooth_shaded(&mut self, asset_id: Uuid, smooth: bool) -> bool {
        if !self.assets.iter().any(|asset| asset.id == asset_id) {
            return false;
        }
        let changed = if smooth {
            if self.is_smooth_shaded(asset_id) {
                false
            } else {
                self.smooth_shaded_assets.push(asset_id);
                true
            }
        } else {
            let before = self.smooth_shaded_assets.len();
            self.smooth_shaded_assets.retain(|id| *id != asset_id);
            self.smooth_shaded_assets.len() != before
        };
        if changed {
            self.bump_normals();
        }
        changed
    }

    /// Primeira luz habilitada da cena, se houver.
    pub fn active_light(&self) -> Option<&Light> {
        self.lights.iter().find(|light| light.enabled)
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: default_project_name(),
            assets: Vec::new(),
            splines: Vec::new(),
            profiles: Vec::new(),
            path_generators: Vec::new(),
            smooth_shaded_assets: Vec::new(),
            active: 0,
            history_selection: Vec::new(),
            palette: default_palette(),
            collections: Vec::new(),
            annotations: Vec::new(),
            annotation_groups: Vec::new(),
            measurements: Vec::new(),
            annotations_visible: true,
            annotations_locked: false,
            measurements_visible: true,
            materials: vec![Material::new("Default Material")],
            // Uma direcional padrão: o modo Rendered precisa de luz real, e sem
            // nenhuma a cena renderizaria preta.
            lights: vec![Light::directional("Key Light", [0.4, 0.9, 0.6])],
            skeletons: Vec::new(),
            animations: Vec::new(),
            topology_revision: 0,
            position_revision: 0,
            normal_revision: 0,
            selection_revision: 0,
            uv_revision: 0,
            color_revision: 0,
            material_revision: 0,
            texture_revision: 0,
            transform_revision: 0,
            spline_revision: 0,
            procedural_revision: 0,
        }
    }
}

fn canvas_heap_bytes(canvas: &Canvas) -> usize {
    canvas.pixels.capacity()
}

fn string_vec_heap_bytes(capacity: usize, values: &[String]) -> usize {
    capacity
        .saturating_mul(std::mem::size_of::<String>())
        .saturating_add(values.iter().fold(0usize, |bytes, value| {
            bytes.saturating_add(value.capacity())
        }))
}

fn mesh_heap_bytes(mesh: &Mesh) -> usize {
    let mut bytes = mesh
        .verts
        .capacity()
        .saturating_mul(std::mem::size_of::<petunia_mesh::Vertex>());
    bytes = bytes.saturating_add(
        mesh.faces
            .capacity()
            .saturating_mul(std::mem::size_of::<petunia_mesh::Face>()),
    );
    for face in &mesh.faces {
        bytes = bytes.saturating_add(
            face.verts
                .capacity()
                .saturating_mul(std::mem::size_of::<u32>()),
        );
        bytes = bytes.saturating_add(
            face.uv
                .capacity()
                .saturating_mul(std::mem::size_of::<[f32; 2]>()),
        );
    }
    bytes = bytes.saturating_add(
        mesh.selected_edges
            .capacity()
            .saturating_mul(std::mem::size_of::<(u32, u32)>() + 1),
    );
    bytes = bytes.saturating_add(
        mesh.uv_seams
            .capacity()
            .saturating_mul(std::mem::size_of::<(u32, u32)>() + 1),
    );
    bytes.saturating_add(
        mesh.uv_pinned
            .capacity()
            .saturating_mul(std::mem::size_of::<(usize, usize)>() + 1),
    )
}

impl Project {
    pub fn bump_topology(&mut self) {
        self.topology_revision = self.topology_revision.wrapping_add(1);
    }
    pub fn bump_positions(&mut self) {
        self.position_revision = self.position_revision.wrapping_add(1);
    }
    pub fn bump_normals(&mut self) {
        self.normal_revision = self.normal_revision.wrapping_add(1);
    }
    pub fn bump_selection(&mut self) {
        self.selection_revision = self.selection_revision.wrapping_add(1);
    }
    pub fn bump_uvs(&mut self) {
        self.uv_revision = self.uv_revision.wrapping_add(1);
    }
    pub fn bump_colors(&mut self) {
        self.color_revision = self.color_revision.wrapping_add(1);
    }
    pub fn bump_materials(&mut self) {
        self.material_revision = self.material_revision.wrapping_add(1);
    }
    pub fn bump_textures(&mut self) {
        self.texture_revision = self.texture_revision.wrapping_add(1);
    }
    pub fn bump_transforms(&mut self) {
        self.transform_revision = self.transform_revision.wrapping_add(1);
    }
    pub fn bump_splines(&mut self) {
        self.spline_revision = self.spline_revision.wrapping_add(1);
    }
    pub fn bump_procedural(&mut self) {
        self.procedural_revision = self.procedural_revision.wrapping_add(1);
    }

    pub fn bump_changes(&mut self, changes: ProjectChanges) {
        if changes.contains(ProjectChanges::TOPOLOGY) {
            self.bump_topology();
        }
        if changes.contains(ProjectChanges::POSITIONS) {
            self.bump_positions();
        }
        if changes.contains(ProjectChanges::NORMALS) {
            self.bump_normals();
        }
        if changes.contains(ProjectChanges::SELECTION) {
            self.bump_selection();
        }
        if changes.contains(ProjectChanges::UVS) {
            self.bump_uvs();
        }
        if changes.contains(ProjectChanges::COLORS) {
            self.bump_colors();
        }
        if changes.contains(ProjectChanges::MATERIALS) {
            self.bump_materials();
        }
        if changes.contains(ProjectChanges::TEXTURES) {
            self.bump_textures();
        }
        if changes.contains(ProjectChanges::TRANSFORMS) {
            self.bump_transforms();
        }
        if changes.contains(ProjectChanges::SPLINES) {
            self.bump_splines();
        }
        if changes.contains(ProjectChanges::PROCEDURAL) {
            self.bump_procedural();
        }
    }

    /// Keeps cache revisions monotonic when undo/redo restores an older snapshot.
    pub fn revision_clock(&self) -> [u64; 11] {
        [
            self.topology_revision,
            self.position_revision,
            self.normal_revision,
            self.selection_revision,
            self.uv_revision,
            self.color_revision,
            self.material_revision,
            self.texture_revision,
            self.transform_revision,
            self.spline_revision,
            self.procedural_revision,
        ]
    }

    pub fn rebase_revisions_after_restore(&mut self, previous: [u64; 11]) {
        self.topology_revision = self.topology_revision.max(previous[0]);
        self.position_revision = self.position_revision.max(previous[1]);
        self.normal_revision = self.normal_revision.max(previous[2]);
        self.selection_revision = self.selection_revision.max(previous[3]);
        self.uv_revision = self.uv_revision.max(previous[4]);
        self.color_revision = self.color_revision.max(previous[5]);
        self.material_revision = self.material_revision.max(previous[6]);
        self.texture_revision = self.texture_revision.max(previous[7]);
        self.transform_revision = self.transform_revision.max(previous[8]);
        self.spline_revision = self.spline_revision.max(previous[9]);
        self.procedural_revision = self.procedural_revision.max(previous[10]);
    }

    /// Rebuilds the derived texture cache for one asset from its canonical
    /// paint stack and mirrors it to the assigned material albedo channel.
    pub fn composite_paint_stack(&mut self, asset_index: usize) -> bool {
        let Some(asset) = self.assets.get(asset_index) else {
            return false;
        };
        let Some(stack) = asset.paint_stack.as_ref() else {
            return false;
        };
        let (width, height) = asset
            .texture
            .as_ref()
            .map(|canvas| (canvas.w, canvas.h))
            .unwrap_or((256, 256));
        let material_id = asset.material_id;
        let mut composed = Canvas::new(width, height, [0, 0, 0, 0]);
        stack.composite(&mut composed);

        if let Some(asset) = self.assets.get_mut(asset_index) {
            asset.texture = Some(composed.clone());
        }
        if let Some(material_id) = material_id
            && let Some(material) = self.get_material_mut(material_id)
        {
            material.albedo_texture = Some(composed);
        }
        true
    }

    /// Approximate retained size of an owned history snapshot.
    ///
    /// Uses allocation capacities from the already-cloned snapshot, so history
    /// budgeting accounts for the dominant heap payloads rather than only the
    /// shallow `Project` header.
    pub fn estimated_bytes(&self) -> usize {
        let mut n = std::mem::size_of::<Self>();
        n = n.saturating_add(self.name.capacity());
        n = n.saturating_add(
            self.assets
                .capacity()
                .saturating_mul(std::mem::size_of::<Asset>()),
        );
        for asset in &self.assets {
            n = n.saturating_add(asset.name.capacity());
            n = n.saturating_add(asset.collection.as_ref().map_or(0, String::capacity));
            n = n.saturating_add(
                asset
                    .tags
                    .capacity()
                    .saturating_mul(std::mem::size_of::<String>()),
            );
            n = asset
                .tags
                .iter()
                .fold(n, |bytes, tag| bytes.saturating_add(tag.capacity()));
            n = n.saturating_add(
                asset
                    .modifiers
                    .capacity()
                    .saturating_mul(std::mem::size_of::<ModifierInstance>()),
            );
            n = n.saturating_add(mesh_heap_bytes(&asset.mesh));
            if let Some(tex) = asset.texture.as_ref() {
                n = n.saturating_add(canvas_heap_bytes(tex));
            }
            if let Some(stack) = asset.paint_stack.as_ref() {
                n = n.saturating_add(
                    stack
                        .layers
                        .capacity()
                        .saturating_mul(std::mem::size_of::<PaintLayer>()),
                );
                for layer in &stack.layers {
                    n = n.saturating_add(layer.name.capacity());
                    if let Some(cv) = layer.canvas() {
                        n = n.saturating_add(canvas_heap_bytes(cv));
                    }
                }
            }
            if let Some(skin) = asset.skin_data.as_ref() {
                n = n.saturating_add(
                    skin.vertex_weights
                        .capacity()
                        .saturating_mul(std::mem::size_of::<VertexSkinWeight>()),
                );
            }
            if let Some((_, _, mesh)) = asset.eval_cache.as_ref() {
                n = n.saturating_add(mesh_heap_bytes(mesh));
            }
        }
        n = n.saturating_add(
            self.splines
                .capacity()
                .saturating_mul(std::mem::size_of::<SplineResource>()),
        );
        for spline in &self.splines {
            n = n.saturating_add(spline.name.capacity());
            n = n.saturating_add(
                spline
                    .points
                    .capacity()
                    .saturating_mul(std::mem::size_of::<SplinePoint>()),
            );
        }
        n = n.saturating_add(
            self.profiles
                .capacity()
                .saturating_mul(std::mem::size_of::<ProfileResource>()),
        );
        for profile in &self.profiles {
            n = n.saturating_add(profile.name.capacity());
        }
        n = n.saturating_add(
            self.path_generators
                .capacity()
                .saturating_mul(std::mem::size_of::<PathGenerator>()),
        );
        for generator in &self.path_generators {
            n = n.saturating_add(generator.name.capacity());
        }
        n = n.saturating_add(
            self.history_selection
                .capacity()
                .saturating_mul(std::mem::size_of::<Uuid>()),
        );
        n = n.saturating_add(
            self.palette
                .capacity()
                .saturating_mul(std::mem::size_of::<[f32; 3]>()),
        );
        n = n.saturating_add(string_vec_heap_bytes(
            self.collections.capacity(),
            &self.collections,
        ));
        n = n.saturating_add(
            self.annotations
                .capacity()
                .saturating_mul(std::mem::size_of::<AnnotationItem>()),
        );
        for annotation in &self.annotations {
            n = n.saturating_add(annotation.name.capacity());
            n = n.saturating_add(annotation.group.as_ref().map_or(0, String::capacity));
            n = n.saturating_add(
                annotation
                    .strokes
                    .capacity()
                    .saturating_mul(std::mem::size_of::<AnnotationStroke>()),
            );
            for stroke in &annotation.strokes {
                n = n.saturating_add(
                    stroke
                        .points
                        .capacity()
                        .saturating_mul(std::mem::size_of::<[f32; 3]>()),
                );
            }
        }
        n = n.saturating_add(string_vec_heap_bytes(
            self.annotation_groups.capacity(),
            &self.annotation_groups,
        ));
        n = n.saturating_add(
            self.measurements
                .capacity()
                .saturating_mul(std::mem::size_of::<MeasurementItem>()),
        );
        for measurement in &self.measurements {
            n = n.saturating_add(measurement.name.capacity());
        }
        n = n.saturating_add(
            self.materials
                .capacity()
                .saturating_mul(std::mem::size_of::<Material>()),
        );
        for mat in &self.materials {
            n = n.saturating_add(mat.name.capacity());
            for texture in [
                mat.albedo_texture.as_ref(),
                mat.normal_texture.as_ref(),
                mat.roughness_texture.as_ref(),
                mat.metallic_texture.as_ref(),
                mat.emission_texture.as_ref(),
                mat.height_texture.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                n = n.saturating_add(canvas_heap_bytes(texture));
            }
        }
        n = n.saturating_add(
            self.lights
                .capacity()
                .saturating_mul(std::mem::size_of::<Light>()),
        );
        for light in &self.lights {
            n = n.saturating_add(light.name.capacity());
        }
        n = n.saturating_add(
            self.skeletons
                .capacity()
                .saturating_mul(std::mem::size_of::<Skeleton>()),
        );
        for skeleton in &self.skeletons {
            n = n.saturating_add(skeleton.name.capacity());
            n = n.saturating_add(
                skeleton
                    .bones
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Bone>()),
            );
            for bone in &skeleton.bones {
                n = n.saturating_add(bone.name.capacity());
            }
        }
        n = n.saturating_add(
            self.animations
                .capacity()
                .saturating_mul(std::mem::size_of::<AnimationAsset>()),
        );
        for animation in &self.animations {
            n = n.saturating_add(animation.name.capacity());
            n = n.saturating_add(animation.preset.as_ref().map_or(0, String::capacity));
            n = n.saturating_add(string_vec_heap_bytes(
                animation.tags.capacity(),
                &animation.tags,
            ));
            n = n.saturating_add(animation.clip.name.capacity());
            n = n.saturating_add(
                animation
                    .clip
                    .tracks
                    .capacity()
                    .saturating_mul(std::mem::size_of::<BoneTrack>()),
            );
            for track in &animation.clip.tracks {
                n = n.saturating_add(track.bone_name.capacity());
                n = n.saturating_add(
                    track
                        .translations
                        .capacity()
                        .saturating_mul(std::mem::size_of::<Keyframe<[f32; 3]>>()),
                );
                n = n.saturating_add(
                    track
                        .rotations
                        .capacity()
                        .saturating_mul(std::mem::size_of::<Keyframe<[f32; 4]>>()),
                );
                n = n.saturating_add(
                    track
                        .scales
                        .capacity()
                        .saturating_mul(std::mem::size_of::<Keyframe<[f32; 3]>>()),
                );
            }
        }
        n.max(1)
    }

    pub fn new() -> Self {
        let def_mat = Material::new("Default Material");
        let def_mat_id = def_mat.id;
        let mut cube = Asset::new("Cube", Mesh::cube(2.0));
        cube.material_id = Some(def_mat_id);
        Self {
            id: Uuid::new_v4(),
            name: default_project_name(),
            assets: vec![cube],
            splines: Vec::new(),
            profiles: Vec::new(),
            path_generators: Vec::new(),
            smooth_shaded_assets: Vec::new(),
            active: 0,
            history_selection: Vec::new(),
            palette: default_palette(),
            collections: Vec::new(),
            annotations: Vec::new(),
            annotation_groups: Vec::new(),
            measurements: Vec::new(),
            annotations_visible: true,
            annotations_locked: false,
            measurements_visible: true,
            materials: vec![def_mat],
            lights: vec![Light::directional("Key Light", [0.4, 0.9, 0.6])],
            skeletons: Vec::new(),
            animations: Vec::new(),
            topology_revision: 0,
            position_revision: 0,
            normal_revision: 0,
            selection_revision: 0,
            uv_revision: 0,
            color_revision: 0,
            material_revision: 0,
            texture_revision: 0,
            transform_revision: 0,
            spline_revision: 0,
            procedural_revision: 0,
        }
    }

    pub fn get_skeleton(&self, id: Uuid) -> Option<&Skeleton> {
        self.skeletons.iter().find(|s| s.id == id)
    }

    pub fn get_skeleton_mut(&mut self, id: Uuid) -> Option<&mut Skeleton> {
        self.skeletons.iter_mut().find(|s| s.id == id)
    }

    pub fn add_skeleton(&mut self, skeleton: Skeleton) {
        self.skeletons.push(skeleton);
    }

    pub fn remove_skeleton(&mut self, id: Uuid) {
        self.skeletons.retain(|s| s.id != id);
        for a in &mut self.assets {
            if a.skeleton_id == Some(id) {
                a.skeleton_id = None;
                a.skin_data = None;
            }
        }
    }

    pub fn get_animation(&self, id: Uuid) -> Option<&AnimationAsset> {
        self.animations.iter().find(|a| a.id == id)
    }

    pub fn get_animation_mut(&mut self, id: Uuid) -> Option<&mut AnimationAsset> {
        self.animations.iter_mut().find(|a| a.id == id)
    }

    pub fn add_animation(&mut self, animation: AnimationAsset) {
        self.animations.push(animation);
    }

    pub fn remove_animation(&mut self, id: Uuid) {
        self.animations.retain(|a| a.id != id);
    }

    pub fn add_annotation(&mut self, item: AnnotationItem) {
        self.annotations.push(item);
    }

    pub fn remove_annotation(&mut self, id: Uuid) {
        self.annotations.retain(|a| a.id != id);
    }

    pub fn add_annotation_group(&mut self, name: &str) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() || self.annotation_groups.iter().any(|g| g == trimmed) {
            return false;
        }
        self.annotation_groups.push(trimmed.to_string());
        true
    }

    pub fn remove_annotation_group(&mut self, name: &str) {
        self.annotation_groups.retain(|g| g != name);
        for a in &mut self.annotations {
            if a.group.as_deref() == Some(name) {
                a.group = None;
            }
        }
    }

    pub fn add_measurement(&mut self, item: MeasurementItem) {
        self.measurements.push(item);
    }

    pub fn remove_measurement(&mut self, id: Uuid) {
        self.measurements.retain(|m| m.id != id);
    }

    pub fn add_collection(&mut self, name: &str) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() || self.collections.iter().any(|c| c == trimmed) {
            return false;
        }
        self.collections.push(trimmed.to_string());
        true
    }

    pub fn remove_collection(&mut self, name: &str) {
        self.collections.retain(|c| c != name);
        for a in &mut self.assets {
            if a.collection.as_deref() == Some(name) {
                a.collection = None;
            }
        }
    }

    pub fn active(&self) -> Option<&Asset> {
        self.assets.get(self.active)
    }
    pub fn active_mesh(&self) -> Option<&Mesh> {
        self.active().map(|o| &o.mesh)
    }

    pub fn active_mut(&mut self) -> Option<&mut Asset> {
        self.assets.get_mut(self.active)
    }
    pub fn active_mesh_mut(&mut self) -> Option<&mut Mesh> {
        self.active_mut().map(|o| &mut o.mesh)
    }

    pub fn add(&mut self, name: &str, mesh: Mesh) {
        self.assets.push(Asset::new(name, mesh));
        self.active = self.assets.len() - 1;
    }

    pub fn remove(&mut self, i: usize) {
        if i >= self.assets.len() {
            return;
        }
        let removed = self.assets.remove(i);
        self.history_selection.retain(|id| *id != removed.id);
        if self.assets.is_empty() || self.active == usize::MAX {
            self.active = usize::MAX;
        } else if i < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.assets.len() - 1);
        }
    }

    /// Reordena um asset da posição `from` para a posição `to`, mantendo o asset ativo selecionado.
    /// Reorders an asset from position `from` to position `to`, preserving the active asset selection.
    pub fn reorder_asset(&mut self, from: usize, to: usize) -> bool {
        if from >= self.assets.len() || to >= self.assets.len() || from == to {
            return false;
        }
        let active_id = self.assets.get(self.active).map(|a| a.id);
        let asset = self.assets.remove(from);
        self.assets.insert(to, asset);
        if let Some(id) = active_id
            && let Some(idx) = self.find(id)
        {
            self.active = idx;
        }
        true
    }

    pub fn totals(&self) -> (usize, usize) {
        let (mut v, mut f) = (0, 0);
        for o in &self.assets {
            v += o.mesh.vert_count();
            f += o.mesh.tri_count();
        }
        (v, f)
    }

    pub fn find(&self, id: Uuid) -> Option<usize> {
        self.assets.iter().position(|a| a.id == id)
    }

    /// Localiza asset por ID estável retornando índice e referência.
    pub fn find_by_id(&self, id: Uuid) -> Option<(usize, &Asset)> {
        self.assets.iter().enumerate().find(|(_, a)| a.id == id)
    }

    /// Localiza asset por ID estável retornando índice e referência mutável.
    pub fn find_by_id_mut(&mut self, id: Uuid) -> Option<(usize, &mut Asset)> {
        self.assets.iter_mut().enumerate().find(|(_, a)| a.id == id)
    }

    /// Remove asset por ID estável mantendo invariants de seleção.
    pub fn remove_by_id(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.find(id) {
            self.remove(pos);
            true
        } else {
            false
        }
    }

    /// Duplica asset por ID gerando novo UUID persistente e ativando-o.
    pub fn duplicate_by_id(&mut self, id: Uuid) -> Option<Uuid> {
        let dup = {
            let (_, asset) = self.find_by_id(id)?;
            asset.duplicate()
        };
        let new_id = dup.id;
        self.assets.push(dup);
        self.active = self.assets.len() - 1;
        Some(new_id)
    }

    // ---- Spline Resources (P3D-161) ----

    pub fn add_spline(&mut self, spline: SplineResource) -> Result<Uuid, SplineError> {
        if self.splines.iter().any(|existing| existing.id == spline.id) {
            return Err(SplineError::DuplicateSpline(spline.id));
        }
        spline.validate_authoring()?;
        let id = spline.id;
        self.splines.push(spline);
        Ok(id)
    }

    pub fn get_spline(&self, id: Uuid) -> Option<&SplineResource> {
        self.splines.iter().find(|spline| spline.id == id)
    }

    pub fn get_spline_mut(&mut self, id: Uuid) -> Option<&mut SplineResource> {
        self.splines.iter_mut().find(|spline| spline.id == id)
    }

    pub fn resolved_spline(&self, id: Uuid) -> Result<SplineResource, SplineError> {
        let mut spline = self
            .get_spline(id)
            .cloned()
            .ok_or(SplineError::SplineNotFound(id))?;
        for point in &mut spline.points {
            if let Some(attachment) = point.attachment {
                let frame = evaluate_surface_attachment(self, &attachment)?;
                point.position = frame.position.map(f64::from);
            }
        }
        Ok(spline)
    }

    pub fn remove_spline(&mut self, id: Uuid) -> Result<SplineResource, SplineError> {
        if let Some(profile) = self.profiles.iter().find(|profile| profile.spline_id == id) {
            return Err(SplineError::SplineUsedByProfile(profile.id));
        }
        if let Some(generator) = self
            .path_generators
            .iter()
            .find(|generator| generator.path_id == id)
        {
            return Err(SplineError::SplineUsedByGenerator(generator.id));
        }
        let index = self
            .splines
            .iter()
            .position(|spline| spline.id == id)
            .ok_or(SplineError::SplineNotFound(id))?;
        Ok(self.splines.remove(index))
    }

    // ---- Shape-first Profiles and Path Generators (P3D-160/P3D-168) ----

    pub fn add_profile(&mut self, profile: ProfileResource) -> Result<Uuid, ProfileError> {
        if self
            .profiles
            .iter()
            .any(|existing| existing.id == profile.id)
        {
            return Err(ProfileError::DuplicateProfile(profile.id));
        }
        if let Some(existing) = self
            .profiles
            .iter()
            .find(|existing| existing.spline_id == profile.spline_id)
        {
            return Err(ProfileError::SplineAlreadyOwned(existing.id));
        }
        let spline = self
            .get_spline(profile.spline_id)
            .ok_or(ProfileError::SplineNotFound(profile.spline_id))?;
        profile.validate_authoring(spline)?;
        let id = profile.id;
        self.profiles.push(profile);
        Ok(id)
    }

    pub fn get_profile(&self, id: Uuid) -> Option<&ProfileResource> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn get_profile_mut(&mut self, id: Uuid) -> Option<&mut ProfileResource> {
        self.profiles.iter_mut().find(|profile| profile.id == id)
    }

    pub fn remove_profile(&mut self, id: Uuid) -> Result<ProfileResource, ProfileError> {
        if let Some(generator) = self
            .path_generators
            .iter()
            .find(|generator| generator.profile_id == id)
        {
            return Err(ProfileError::ProfileInUse(generator.id));
        }
        let index = self
            .profiles
            .iter()
            .position(|profile| profile.id == id)
            .ok_or(ProfileError::ProfileNotFound(id))?;
        Ok(self.profiles.remove(index))
    }

    pub fn add_path_generator(
        &mut self,
        generator: PathGenerator,
    ) -> Result<Uuid, PathGeneratorError> {
        if self
            .path_generators
            .iter()
            .any(|existing| existing.id == generator.id)
        {
            return Err(PathGeneratorError::DuplicateGenerator(generator.id));
        }
        path_generator::validate_path_generator(self, &generator)?;
        let id = generator.id;
        self.path_generators.push(generator);
        Ok(id)
    }

    pub fn get_path_generator(&self, id: Uuid) -> Option<&PathGenerator> {
        self.path_generators
            .iter()
            .find(|generator| generator.id == id)
    }

    pub fn get_path_generator_mut(&mut self, id: Uuid) -> Option<&mut PathGenerator> {
        self.path_generators
            .iter_mut()
            .find(|generator| generator.id == id)
    }

    pub fn validate_path_generator(&self, id: Uuid) -> Result<(), PathGeneratorError> {
        let generator = self
            .get_path_generator(id)
            .ok_or(PathGeneratorError::GeneratorNotFound(id))?;
        self.validate_path_generator_candidate(generator)
    }

    pub fn validate_path_generator_candidate(
        &self,
        generator: &PathGenerator,
    ) -> Result<(), PathGeneratorError> {
        path_generator::validate_path_generator(self, generator)
    }

    pub fn set_sweep_generator_parameters(
        &mut self,
        id: Uuid,
        parameters: SweepGeneratorParameters,
    ) -> Result<bool, PathGeneratorError> {
        self.get_path_generator_mut(id)
            .ok_or(PathGeneratorError::GeneratorNotFound(id))?
            .set_sweep_parameters(parameters)
    }

    pub fn remove_path_generator(&mut self, id: Uuid) -> Result<PathGenerator, PathGeneratorError> {
        let index = self
            .path_generators
            .iter()
            .position(|generator| generator.id == id)
            .ok_or(PathGeneratorError::GeneratorNotFound(id))?;
        Ok(self.path_generators.remove(index))
    }

    pub fn evaluate_path_generator<'a>(
        &self,
        id: Uuid,
        quality: PathGeneratorQuality,
        cache: &'a mut PathGeneratorEvaluationCache,
    ) -> Result<&'a PathGeneratorEvaluation, PathGeneratorError> {
        cache.evaluate(self, id, quality)
    }

    // ---- Material Management (P3D-050) ----

    pub fn add_material(&mut self, mat: Material) -> Uuid {
        let id = mat.id;
        self.materials.push(mat);
        id
    }

    pub fn get_material(&self, id: Uuid) -> Option<&Material> {
        self.materials.iter().find(|m| m.id == id)
    }

    pub fn get_material_mut(&mut self, id: Uuid) -> Option<&mut Material> {
        self.materials.iter_mut().find(|m| m.id == id)
    }

    pub fn remove_material(&mut self, id: Uuid) -> bool {
        let before = self.materials.len();
        self.materials.retain(|m| m.id != id);
        let removed = self.materials.len() < before;
        if removed {
            for a in &mut self.assets {
                if a.material_id == Some(id) {
                    a.material_id = None;
                }
            }
        }
        removed
    }

    pub fn active_material(&self) -> Option<&Material> {
        let active_asset = self.active()?;
        active_asset
            .material(self)
            .or_else(|| self.materials.first())
    }

    pub fn active_material_mut(&mut self) -> Option<&mut Material> {
        let mat_id = self
            .active()
            .and_then(|a| a.material_id)
            .or_else(|| self.materials.first().map(|m| m.id))?;
        self.get_material_mut(mat_id)
    }

    /// Normaliza projeto vindo de arquivo (M2/M3): malhas válidas,
    /// no mínimo 1 asset, `active` dentro dos limites e materiais íntegros (P3D-050).
    pub fn validate(&mut self) {
        // Remove ids de sombreamento suave que não apontam mais para um asset.
        let asset_ids: std::collections::HashSet<Uuid> =
            self.assets.iter().map(|asset| asset.id).collect();
        self.smooth_shaded_assets
            .retain(|id| asset_ids.contains(id));
        self.smooth_shaded_assets.sort_unstable();
        self.smooth_shaded_assets.dedup();
        let mut spline_ids = std::collections::HashSet::with_capacity(self.splines.len());
        for spline in &mut self.splines {
            if !spline_ids.insert(spline.id) {
                spline.id = Uuid::new_v4();
                spline_ids.insert(spline.id);
            }
            spline.validate();
        }
        let mut profile_ids = std::collections::HashSet::with_capacity(self.profiles.len());
        for profile in &mut self.profiles {
            if !profile_ids.insert(profile.id) {
                profile.id = Uuid::new_v4();
                profile_ids.insert(profile.id);
            }
            profile.validate_loaded();
        }
        let mut generator_ids =
            std::collections::HashSet::with_capacity(self.path_generators.len());
        for generator in &mut self.path_generators {
            if !generator_ids.insert(generator.id) {
                generator.id = Uuid::new_v4();
                generator_ids.insert(generator.id);
            }
            generator.validate_loaded();
        }
        for mat in &mut self.materials {
            mat.validate();
        }
        if self.materials.is_empty() {
            self.materials.push(Material::new("Default Material"));
        }
        let fallback_mat_id = self.materials[0].id;

        for a in &mut self.assets {
            a.mesh.validate();
            if let Some(cv) = a.texture.as_mut() {
                cv.validate();
            }
            if !a.base_color.iter().all(|x| x.is_finite()) {
                a.base_color = [0.75, 0.75, 0.78];
            }
            if a.material_id.is_none()
                || !self.materials.iter().any(|m| Some(m.id) == a.material_id)
            {
                a.material_id = Some(fallback_mat_id);
            }
        }
        // An empty document and an explicitly cleared object selection are valid.
        if self.assets.is_empty() {
            self.active = usize::MAX;
        } else if self.active != usize::MAX {
            self.active = self.active.min(self.assets.len() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_remove_shifts_active_index() {
        let mut p = Project::new();
        p.add("Asset 1", Mesh::cube(1.0));
        p.add("Asset 2", Mesh::cube(1.0));
        assert_eq!(p.assets.len(), 3);
        p.active = 2; // Asset 2 active

        // Remove Asset 0 (before active)
        p.remove(0);
        assert_eq!(p.assets.len(), 2);
        assert_eq!(p.active, 1);
        assert_eq!(p.assets[p.active].name, "Asset 2");

        // Remove active asset
        p.remove(1);
        assert_eq!(p.assets.len(), 1);
        assert_eq!(p.active, 0);
        assert_eq!(p.assets[p.active].name, "Asset 1");
    }

    #[test]
    fn test_annotation_item_transform() {
        let mut stroke = AnnotationStroke::default();
        stroke.points.push([1.0, 0.0, 0.0]);
        stroke.points.push([3.0, 0.0, 0.0]);

        let mut item = AnnotationItem::new("Note 1", vec![stroke]);
        item.translation = [10.0, 5.0, 2.0];
        item.scale = [2.0, 2.0, 2.0];

        let p0 = item.transform_point([1.0, 0.0, 0.0]);
        assert_eq!(p0, [12.0, 5.0, 2.0]);

        let center = item.center();
        assert_eq!(center, [14.0, 5.0, 2.0]);
    }

    #[test]
    fn test_project_annotations_and_measurements_management() {
        let mut p = Project::new();
        assert!(p.annotations.is_empty());
        assert!(p.measurements.is_empty());

        let a_item = AnnotationItem::new("A1", vec![AnnotationStroke::default()]);
        let a_id = a_item.id;
        p.add_annotation(a_item);
        assert_eq!(p.annotations.len(), 1);

        assert!(p.add_annotation_group("Rascunhos"));
        p.annotations[0].group = Some("Rascunhos".to_string());
        p.remove_annotation_group("Rascunhos");
        assert!(p.annotations[0].group.is_none());

        p.remove_annotation(a_id);
        assert!(p.annotations.is_empty());

        let m_item = MeasurementItem::new("M1", [0.0, 0.0, 0.0], [1.0, 2.0, 2.0], 3.0);
        let m_id = m_item.id;
        p.add_measurement(m_item);
        assert_eq!(p.measurements.len(), 1);
        assert_eq!(p.measurements[0].deltas(), [1.0, 2.0, 2.0]);

        p.remove_measurement(m_id);
        assert!(p.measurements.is_empty());
    }

    #[test]
    fn evaluated_mesh_cache_tracks_source_content_not_only_counts() {
        let mut asset = Asset::new("Cached", Mesh::cube(2.0));
        let original = asset.evaluated_mesh_cached().verts[0].pos;

        asset.mesh.verts[0].pos[0] += 3.0;
        let updated = asset.evaluated_mesh_cached().verts[0].pos;

        assert_ne!(updated, original);
        assert_eq!(updated, asset.mesh.verts[0].pos);
    }

    #[test]
    fn project_changes_advance_only_declared_revisions() {
        let mut project = Project::new();
        project.bump_changes(ProjectChanges::UVS | ProjectChanges::TEXTURES);

        assert_eq!(project.uv_revision, 1);
        assert_eq!(project.texture_revision, 1);
        assert_eq!(project.topology_revision, 0);
        assert_eq!(project.position_revision, 0);
        assert_eq!(project.normal_revision, 0);
        assert_eq!(project.selection_revision, 0);
        assert_eq!(project.color_revision, 0);
        assert_eq!(project.material_revision, 0);
        assert_eq!(project.transform_revision, 0);
        assert_eq!(project.spline_revision, 0);
        assert_eq!(project.procedural_revision, 0);
    }

    #[test]
    fn restored_revision_clock_advances_only_the_published_domain() {
        let previous = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
        let mut restored = Project::new();

        restored.rebase_revisions_after_restore(previous);
        assert_eq!(restored.revision_clock(), previous);

        restored.bump_changes(ProjectChanges::UVS);
        let mut expected = previous;
        expected[4] += 1;
        assert_eq!(restored.revision_clock(), expected);
    }

    #[test]
    fn spline_storage_is_counted_in_history_budget() {
        let mut project = Project::new();
        let before = project.estimated_bytes();
        project
            .add_spline(SplineResource::from_polyline(
                "Hair guide",
                &[[0.0, 0.0, 0.0], [0.0, 1.0, 0.2], [0.2, 2.0, 0.4]],
                false,
            ))
            .unwrap();

        assert!(project.estimated_bytes() > before);
    }
}
