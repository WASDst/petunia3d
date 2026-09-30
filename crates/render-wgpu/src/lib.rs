//! Renderer wgpu: malha sombreada + arestas + grid estilo Blender + quads de referência.
//! Buffers de malha/arestas/referência são revisionados: câmera atualiza só o
//! uniform; geometria só reconstrói quando o fingerprint da cena muda (Wave 1).

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use petunia_core::Camera;
use petunia_core::RefAxis;
use petunia_core::{FingerprintFlags, SceneFingerprint, TextureUpdate, fingerprint_scene};
use petunia_project::Project;
use petunia_render::Shading;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MeshVertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    pos: [f32; 3],
    color: [f32; 3],
}

/// Canto de uma aresta larga: as duas pontas, a cor e `corner` =
/// (ponta 0/1, lado −1/+1). O vertex shader expande a faixa em pixels.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WideLineVertex {
    a: [f32; 3],
    b: [f32; 3],
    color: [f32; 3],
    /// (ponta 0/1, lado −1/+1, multiplicador da largura).
    corner: [f32; 3],
}

/// Aparência das arestas no viewport (capítulo 05, aparência por workspace).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeMode {
    /// Faces limpas; arestas só com o overlay de wireframe (PAINT, UV).
    #[default]
    Overlay,
    /// DRAW: leitura de forma — só arestas de feição (bordas e dobras).
    Features,
    /// POLY: leitura de topologia — todas as arestas finas, as de feição
    /// reforçadas.
    Topology,
}

/// Plano de trabalho do DRAW em destaque (capítulo 05): origem e eixos do
/// frame do perfil, em coordenadas de mundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkplaneOverlay {
    pub origin: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
}

/// Fração da altura visível coberta pelo recorte do plano (meia largura).
const WORKPLANE_HALF_EXTENT: f32 = 0.3;
/// Divisões da grade do recorte em cada metade.
const WORKPLANE_HALF_CELLS: i32 = 4;
const WORKPLANE_FILL: [f32; 4] = [0.36, 0.62, 0.95, 0.10];
const WORKPLANE_GRID: [f32; 4] = [0.46, 0.70, 0.98, 0.35];
const WORKPLANE_AXIS: [f32; 4] = [0.56, 0.78, 1.0, 0.75];

/// Recorte translúcido do plano de trabalho com grade, centrado na origem.
/// O tamanho acompanha a altura visível, então lê igual em qualquer zoom; o
/// recorte é empurrado um pouco para a câmera para não brigar com a face.
fn append_workplane(
    triangles: &mut Vec<SelectionVertex>,
    plane: WorkplaneOverlay,
    camera: &Camera,
    viewport_height: u32,
) {
    let origin = Vec3::from(plane.origin);
    let right = Vec3::from(plane.right).normalize_or_zero();
    let up = Vec3::from(plane.up).normalize_or_zero();
    let mut normal = right.cross(up).normalize_or_zero();
    if right == Vec3::ZERO || up == Vec3::ZERO || normal == Vec3::ZERO {
        return;
    }
    let perspective_scale = if camera.proj == petunia_core::Projection::Perspective {
        ((origin - camera.eye()).dot(camera.forward()) / camera.distance.max(0.01)).max(0.01)
    } else {
        1.0
    };
    let visible = camera.visible_height() * perspective_scale;
    if normal.dot(camera.eye() - origin) < 0.0 {
        normal = -normal;
    }
    let center = origin + normal * visible * 0.002;
    let half = visible * WORKPLANE_HALF_EXTENT;
    let corner = |u: f32, v: f32| center + right * u + up * v;
    let quad = [
        corner(-half, -half),
        corner(half, -half),
        corner(half, half),
        corner(-half, half),
    ];
    for index in [0usize, 1, 2, 0, 2, 3] {
        triangles.push(SelectionVertex {
            pos: quad[index].to_array(),
            color: WORKPLANE_FILL,
        });
    }
    let step = half / WORKPLANE_HALF_CELLS as f32;
    for cell in -WORKPLANE_HALF_CELLS..=WORKPLANE_HALF_CELLS {
        let offset = cell as f32 * step;
        let (color, width) = if cell == 0 {
            (WORKPLANE_AXIS, 1.5)
        } else {
            (WORKPLANE_GRID, 1.0)
        };
        for (start, end) in [
            (corner(offset, -half), corner(offset, half)),
            (corner(-half, offset), corner(half, offset)),
        ] {
            append_edge_band(triangles, start, end, camera, viewport_height, width, color);
        }
    }
}

/// Ângulo entre faces vizinhas acima do qual a aresta é "de feição".
pub const CREASE_DEGREES: f32 = 30.0;
/// Arestas comuns (não de feição) em relação à largura base.
const THIN_EDGE_SCALE: f32 = 0.67;
const FEATURE_EDGE_COLOR: [f32; 3] = [0.05, 0.05, 0.06];
const THIN_EDGE_COLOR: [f32; 3] = [0.16, 0.17, 0.19];

/// Converte pares de `LineVertex` (LineList) em faixas de dois triângulos;
/// `widths[i]` multiplica a largura base do par `i`.
fn wide_lines_from_pairs(pairs: &[LineVertex], widths: &[f32]) -> Vec<WideLineVertex> {
    let mut out = Vec::with_capacity(pairs.len() * 3);
    for (index, pair) in pairs.as_chunks::<2>().0.iter().enumerate() {
        let width = widths.get(index).copied().unwrap_or(1.0);
        let (a, b) = (pair[0].pos, pair[1].pos);
        for (end, side) in [
            (0.0, -1.0),
            (0.0, 1.0),
            (1.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),
            (1.0, -1.0),
        ] {
            let color = if end == 0.0 {
                pair[0].color
            } else {
                pair[1].color
            };
            out.push(WideLineVertex {
                a,
                b,
                color,
                corner: [end, side, width],
            });
        }
    }
    out
}

/// Vértice da camada de seleção: posição + cor com alpha.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SelectionVertex {
    pos: [f32; 3],
    color: [f32; 4],
}

/// Disco orientado para a tela. O raio é em pixels lógicos do target WGPU,
/// independente do zoom e da profundidade do ponto.
fn append_point_disc(
    triangles: &mut Vec<SelectionVertex>,
    point: Vec3,
    camera: &Camera,
    viewport_height: u32,
    radius_px: f32,
    color: [f32; 4],
) {
    let perspective_scale = if camera.proj == petunia_core::Projection::Perspective {
        ((point - camera.eye()).dot(camera.forward()) / camera.distance.max(0.01)).max(0.01)
    } else {
        1.0
    };
    let radius =
        camera.visible_height() * perspective_scale * radius_px / viewport_height.max(1) as f32;
    let right = camera.right() * radius;
    let up = camera.up() * radius;
    const SIDES: usize = 12;
    for side in 0..SIDES {
        let a = side as f32 * std::f32::consts::TAU / SIDES as f32;
        let b = (side + 1) as f32 * std::f32::consts::TAU / SIDES as f32;
        let pa = point + right * a.cos() + up * a.sin();
        let pb = point + right * b.cos() + up * b.sin();
        for position in [point, pa, pb] {
            triangles.push(SelectionVertex {
                pos: position.to_array(),
                color,
            });
        }
    }
}

/// Largura padrão das arestas em px (Plasticity: 1,5 px normais).
pub const DEFAULT_LINE_WIDTH_PX: f32 = 1.5;

/// Faixa de aresta voltada à câmera, com largura em pixels lógicos.
fn append_edge_band(
    triangles: &mut Vec<SelectionVertex>,
    start: Vec3,
    end: Vec3,
    camera: &Camera,
    viewport_height: u32,
    width_px: f32,
    color: [f32; 4],
) {
    let direction = end - start;
    let side = direction.cross(camera.forward()).normalize_or_zero();
    if side.length_squared() < 1.0e-8 {
        return;
    }
    let midpoint = (start + end) * 0.5;
    let perspective_scale = if camera.proj == petunia_core::Projection::Perspective {
        ((midpoint - camera.eye()).dot(camera.forward()) / camera.distance.max(0.01)).max(0.01)
    } else {
        1.0
    };
    let half_width = camera.visible_height() * perspective_scale * width_px
        / viewport_height.max(1) as f32
        * 0.5;
    let offset = side * half_width;
    let corners = [start - offset, start + offset, end + offset, end - offset];
    for index in [0usize, 1, 2, 0, 2, 3] {
        triangles.push(SelectionVertex {
            pos: corners[index].to_array(),
            color,
        });
    }
}

#[cfg(test)]
mod point_disc_tests {
    use super::*;

    #[test]
    fn point_marker_is_a_filled_disc_that_grows_for_hover() {
        let camera = Camera::default();
        let mut regular = Vec::new();
        let mut hovered = Vec::new();
        let color = [0.49, 0.86, 1.0, 1.0];
        append_point_disc(&mut regular, Vec3::ZERO, &camera, 768, 3.0, color);
        append_point_disc(&mut hovered, Vec3::ZERO, &camera, 768, 5.0, color);
        assert_eq!(regular.len(), 36);
        assert_eq!(hovered.len(), regular.len());
        assert!(
            regular
                .as_chunks::<3>()
                .0
                .iter()
                .all(|triangle| triangle[0].pos == [0.0; 3])
        );
        let regular_radius = Vec3::from_array(regular[1].pos).length();
        let hover_radius = Vec3::from_array(hovered[1].pos).length();
        assert!(hover_radius > regular_radius * 1.6);
    }

    #[test]
    fn selected_edge_band_uses_triangles_and_configured_width() {
        let camera = Camera::default();
        let mut narrow = Vec::new();
        let mut thick = Vec::new();
        let color = [0.2, 0.7, 0.9, 1.0];
        append_edge_band(&mut narrow, Vec3::ZERO, Vec3::X, &camera, 768, 1.0, color);
        append_edge_band(&mut thick, Vec3::ZERO, Vec3::X, &camera, 768, 5.0, color);
        assert_eq!(narrow.len(), 6);
        assert_eq!(thick.len(), 6);
        assert!(thick.iter().all(|vertex| vertex.color == color));
        let narrow_span =
            (Vec3::from_array(narrow[0].pos) - Vec3::from_array(narrow[1].pos)).length();
        let thick_span = (Vec3::from_array(thick[0].pos) - Vec3::from_array(thick[1].pos)).length();
        assert!(thick_span > narrow_span * 4.9);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    /// xyz = direção da luz (normalizada), w = livre.
    light_dir: [f32; 4],
    /// x = ambiente, y = difusa, z/w = livres.
    light_params: [f32; 4],
    /// x = alpha do X-Ray, y/z = tamanho do alvo em px físicos, w = largura
    /// das arestas em px físicos.
    xray: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RefVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RefUniform {
    opacity: f32,
    _pad: [f32; 3],
}

struct RefGpu {
    width: u32,
    height: u32,
    len: usize,
    hash: std::cell::Cell<u64>,
    texture: wgpu::Texture,
    /// Mantida viva junto ao bind group (wgpu é ref-counted, mas explícito é mais seguro).
    #[allow(dead_code)]
    view: wgpu::TextureView,
    params: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

/// Slot de textura do canvas de um asset, revisionado pelo documento.
struct AssetTexGpu {
    asset_id: uuid::Uuid,
    width: u32,
    height: u32,
    revision: u64,
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

/// Faixa de vértices de um asset no VB único + textura (paridade GL).
struct MeshRange {
    start: u32,
    count: u32,
    asset_id: Option<uuid::Uuid>,
}

pub struct Renderer {
    depth_format: wgpu::TextureFormat,
    depth_view: Option<wgpu::TextureView>,
    depth_size: (u32, u32),
    /// Amostras por pixel de todas as pipelines e do depth (1 = sem MSAA).
    sample_count: u32,
    /// Pixels físicos do alvo por pixel lógico da UI. Larguras e raios de
    /// overlays são especificados em px lógicos e convertidos por esta razão.
    pixel_ratio: f32,
    mesh_pipeline: wgpu::RenderPipeline,
    mesh_xray_pipeline: wgpu::RenderPipeline,
    mesh_tex_pipeline: wgpu::RenderPipeline,
    mesh_tex_xray_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    xray: bool,
    xray_opacity: f32,
    selection_rgb: [u8; 3],
    selection_thickness: f32,
    ref_pipeline: wgpu::RenderPipeline,
    ref_xray_pipeline: wgpu::RenderPipeline,
    cam_buffer: wgpu::Buffer,
    cam_bind_group: wgpu::BindGroup,
    ref_tex_layout: wgpu::BindGroupLayout,
    mesh_tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    mesh_vb: Option<wgpu::Buffer>,
    mesh_count: u32,
    mesh_ranges: Vec<MeshRange>,
    asset_tex: Vec<AssetTexGpu>,
    line_vb: Option<wgpu::Buffer>,
    line_count: u32,
    wide_line_pipeline: wgpu::RenderPipeline,
    wide_line_xray_pipeline: wgpu::RenderPipeline,
    /// Largura das arestas em px lógicos; o uniform recebe × `pixel_ratio`.
    line_width_px: f32,
    edge_mode: EdgeMode,
    workplane: Option<WorkplaneOverlay>,
    /// A camada de seleção precisa ser refeita (mudou algo fora da cena).
    selection_dirty: bool,
    selection_tri_pipeline: wgpu::RenderPipeline,
    selection_line_pipeline: wgpu::RenderPipeline,
    selection_tri_xray_pipeline: wgpu::RenderPipeline,
    selection_line_xray_pipeline: wgpu::RenderPipeline,
    /// Preenchimento translúcido das faces selecionadas (depth test, sem write).
    selection_tri_vb: Option<wgpu::Buffer>,
    selection_tri_count: u32,
    /// Contorno e marcadores da seleção (depth test).
    selection_line_vb: Option<wgpu::Buffer>,
    selection_line_count: u32,
    grid_vb: wgpu::Buffer,
    grid_count: u32,
    ref_vb: Option<wgpu::Buffer>,
    ref_count: u32,
    ref_gpu: Vec<RefGpu>,
    pub show_overlays: bool,
    pub show_grid: bool,
    last_fingerprint: Option<SceneFingerprint>,
    last_domain: Option<petunia_core::SelectionDomain>,
    /// Passo do grid atualmente na GPU, para reconstruir só ao cruzar degrau.
    grid_step: f32,
    /// Último alvo de preselection desenhado.
    last_hover: petunia_core::HoverTarget,
    studio_light_follows_camera: bool,
    last_selection_view_proj: Option<[f32; 16]>,
    mesh_rebuilds: u64,
    skipped_frames: u64,
    pending_texture_updates: Vec<TextureUpdate>,
    texture_upload_scratch: Vec<u8>,
    texture_upload_calls: u64,
    texture_bytes_uploaded: u64,
    full_texture_uploads: u64,
    partial_texture_uploads: u64,
}

/// Seleção: cor chapada, sem iluminação, com alpha. A seleção precisa ser
/// legível sobre qualquer shading e nunca depender da luz da cena.
const SELECTION_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: In) -> Out {
    var out: Out;
    out.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

const MESH_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) wpos: vec3<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.color = in.color;
    o.wpos = in.pos;
    return o;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    if (length(in.normal) < 0.1) {
        return vec4<f32>(in.color, 1.0);
    }
    let light = normalize(cam.light_dir.xyz);
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let c = in.color * (cam.light_params.x + cam.light_params.y * diff);
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_xray(in: Out) -> @location(0) vec4<f32> {
    if (length(in.normal) < 0.1) {
        return vec4<f32>(in.color, cam.xray.x);
    }
    let light = normalize(cam.light_dir.xyz);
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let c = in.color * (cam.light_params.x + cam.light_params.y * diff);
    return vec4<f32>(c, cam.xray.x);
}
"#;

/// Variante texturizada da malha (paridade com o backend GL): multiplica a
/// cor do vértice pelo texel do canvas do asset. Fora do modo texturizado
/// (ou sem canvas) o range usa o pipeline de cor sólida.
const MESH_TEX_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) wpos: vec3<f32>,
    @location(3) uv: vec2<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.color = in.color;
    o.wpos = in.pos;
    o.uv = in.uv;
    return o;
}
"#;

/// Monta o shader texturizado com as constantes de luz compartilhadas.
fn mesh_tex_wgsl() -> String {
    const FRAG: &str = r#"
@fragment
fn fs_tex(in: Out) -> @location(0) vec4<f32> {
    let t = textureSample(tex, smp, in.uv);
    let base = in.color * t.rgb;
    if (length(in.normal) < 0.1) {
        return vec4<f32>(base, 1.0);
    }
    let light = normalize(cam.light_dir.xyz);
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let c = base * (cam.light_params.x + cam.light_params.y * diff);
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_tex_xray(in: Out) -> @location(0) vec4<f32> {
    let t = textureSample(tex, smp, in.uv);
    let base = in.color * t.rgb;
    if (length(in.normal) < 0.1) {
        return vec4<f32>(base, cam.xray.x);
    }
    let light = normalize(cam.light_dir.xyz);
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let c = base * (cam.light_params.x + cam.light_params.y * diff);
    return vec4<f32>(c, cam.xray.x);
}
"#;
    (MESH_TEX_WGSL.to_string() + FRAG)
        .replace("LIGHT_X", &petunia_render::scene::LIGHT_DIR[0].to_string())
        .replace("LIGHT_Y", &petunia_render::scene::LIGHT_DIR[1].to_string())
        .replace("LIGHT_Z", &petunia_render::scene::LIGHT_DIR[2].to_string())
        .replace(
            "LIGHT_AMB",
            &petunia_render::scene::LIGHT_AMBIENT.to_string(),
        )
        .replace(
            "LIGHT_DIF",
            &petunia_render::scene::LIGHT_DIFFUSE.to_string(),
        )
}

/// Monta o shader da malha com as constantes de luz compartilhadas
/// (`render::scene`), mantendo uma fonte só.
fn mesh_wgsl() -> String {
    MESH_WGSL
        .replace("LIGHT_X", &petunia_render::scene::LIGHT_DIR[0].to_string())
        .replace("LIGHT_Y", &petunia_render::scene::LIGHT_DIR[1].to_string())
        .replace("LIGHT_Z", &petunia_render::scene::LIGHT_DIR[2].to_string())
        .replace(
            "LIGHT_AMB",
            &petunia_render::scene::LIGHT_AMBIENT.to_string(),
        )
        .replace(
            "LIGHT_DIF",
            &petunia_render::scene::LIGHT_DIFFUSE.to_string(),
        )
}

const LINE_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    // Depth bias in clip space: pull lines slightly toward near plane to eliminate z-fighting with coplanar faces
    o.clip.z -= 0.0001 * o.clip.w;
    o.color = in.color;
    return o;
}
@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

/// Arestas de largura constante em pixels (Bærentzen et al.; Plasticity):
/// cada aresta vira uma faixa de dois triângulos expandida na tela; o MSAA
/// suaviza as bordas. A largura não muda com zoom, distância ou DPI.
const WIDE_LINE_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) a: vec3<f32>,
    @location(1) b: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) corner: vec3<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    let ca = cam.view_proj * vec4<f32>(in.a, 1.0);
    let cb = cam.view_proj * vec4<f32>(in.b, 1.0);
    let size = max(cam.xray.yz, vec2<f32>(1.0, 1.0));
    let sa = ca.xy / max(ca.w, 1e-5) * size * 0.5;
    let sb = cb.xy / max(cb.w, 1e-5) * size * 0.5;
    var dir = sb - sa;
    if (length(dir) < 1e-5) {
        dir = vec2<f32>(1.0, 0.0);
    }
    dir = normalize(dir);
    let normal = vec2<f32>(-dir.y, dir.x);
    var p = ca;
    if (in.corner.x > 0.5) {
        p = cb;
    }
    let half_width = max(cam.xray.w * in.corner.z, 1.0) * 0.5;
    let offset_ndc = normal * in.corner.y * half_width / (size * 0.5);
    o.clip = vec4<f32>(p.xy + offset_ndc * p.w, p.z - 0.0001 * p.w, p.w);
    o.color = in.color;
    return o;
}
@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

const REF_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
struct Params { opacity: f32, _p0: f32, _p1: f32, _p2: f32 };
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var tex: texture_2d<f32>;
@group(1) @binding(2) var smp: sampler;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.uv = in.uv;
    return o;
}
@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, in.uv);
    return vec4<f32>(c.rgb, c.a * params.opacity);
}
"#;

/// Passo do grid correspondente a uma escala visível.
fn adaptive_grid_step(visible_height: f32) -> f32 {
    if visible_height > 60.0 {
        10.0
    } else if visible_height > 24.0 {
        5.0
    } else if visible_height > 8.0 {
        1.0
    } else if visible_height > 3.0 {
        0.5
    } else {
        0.1
    }
}

fn grid_lines() -> Vec<LineVertex> {
    petunia_render::scene::grid_lines()
        .into_iter()
        .flat_map(|(a, b, c)| {
            [
                LineVertex { pos: a, color: c },
                LineVertex { pos: b, color: c },
            ]
        })
        .collect()
}

/// Grid adaptativo: o passo acompanha a escala visível da câmera.
///
/// Afastado, o passo cresce (linhas de 1 m desapareceriam no moiré);
/// aproximado, subdivide. O grid nunca fica nem ilegível nem dominante.
fn adaptive_grid_lines(visible_height: f32) -> Vec<LineVertex> {
    let step = adaptive_grid_step(visible_height);
    let extent = (step * 40.0).clamp(20.0, 400.0);
    petunia_render::scene::grid_lines_custom(extent, step, 0.42, false, 30.0)
        .into_iter()
        .flat_map(|(a, b, c)| {
            [
                LineVertex { pos: a, color: c },
                LineVertex { pos: b, color: c },
            ]
        })
        .collect()
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::with_sample_count(device, format, 1)
    }

    /// Cria o renderer para um alvo com `sample_count` amostras (MSAA).
    ///
    /// O adaptador deve fornecer um alvo de cor multisample com o mesmo número
    /// de amostras e resolvê-lo para a textura exibida.
    pub fn with_sample_count(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let sample_count = sample_count.max(1);
        let msaa = wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        };
        let cam_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simple3d-cam"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cam_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("simple3d-cam-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                // O fragment lê a luz e a opacidade de X-Ray deste uniform.
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let cam_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simple3d-cam-bg"),
            layout: &cam_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buffer.as_entire_binding(),
            }],
        });

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-mesh"),
            source: wgpu::ShaderSource::Wgsl(mesh_wgsl().into()),
        });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-line"),
            source: wgpu::ShaderSource::Wgsl(LINE_WGSL.into()),
        });
        let ref_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-ref"),
            source: wgpu::ShaderSource::Wgsl(REF_WGSL.into()),
        });

        let mesh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-mesh-layout"),
            bind_group_layouts: &[Some(&cam_layout)],
            immediate_size: 0,
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-mesh-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: msaa,
            multiview_mask: None,
            cache: None,
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-line-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: msaa,
            multiview_mask: None,
            cache: None,
        });

        let wide_line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-wide-line-shader"),
            source: wgpu::ShaderSource::Wgsl(WIDE_LINE_WGSL.into()),
        });
        let wide_line = |label: &'static str, compare: wgpu::CompareFunction| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&mesh_layout),
                vertex: wgpu::VertexState {
                    module: &wide_line_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<WideLineVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x3],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &wide_line_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(compare),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: msaa,
                multiview_mask: None,
                cache: None,
            })
        };
        let wide_line_pipeline =
            wide_line("simple3d-wide-line-pipe", wgpu::CompareFunction::LessEqual);
        let wide_line_xray_pipeline = wide_line(
            "simple3d-wide-line-xray-pipe",
            wgpu::CompareFunction::Always,
        );

        let mesh_xray_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-mesh-xray-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_xray"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: msaa,
            multiview_mask: None,
            cache: None,
        });

        // Camada de seleção: um pipeline para o preenchimento translúcido das
        // faces e outro para contorno/marcadores. Ambos com depth test e sem
        // depth write, para não ocluir a geometria nem se sobrepor a si mesmos.
        let selection_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-selection-shader"),
            source: wgpu::ShaderSource::Wgsl(SELECTION_WGSL.into()),
        });
        let selection_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-selection-layout"),
            bind_group_layouts: &[Some(&cam_layout)],
            immediate_size: 0,
        });
        let selection_attrs = [Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SelectionVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
        })];
        let selection_pipeline = |label, topology, xray| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&selection_layout),
                vertex: wgpu::VertexState {
                    module: &selection_shader,
                    entry_point: Some("vs_main"),
                    buffers: &selection_attrs,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &selection_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(if xray {
                        wgpu::CompareFunction::Always
                    } else {
                        wgpu::CompareFunction::LessEqual
                    }),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: msaa,
                multiview_mask: None,
                cache: None,
            })
        };
        let selection_tri_pipeline = selection_pipeline(
            "selection-tri",
            wgpu::PrimitiveTopology::TriangleList,
            false,
        );
        let selection_line_pipeline =
            selection_pipeline("selection-line", wgpu::PrimitiveTopology::LineList, false);
        let selection_tri_xray_pipeline = selection_pipeline(
            "selection-tri-xray",
            wgpu::PrimitiveTopology::TriangleList,
            true,
        );
        let selection_line_xray_pipeline = selection_pipeline(
            "selection-line-xray",
            wgpu::PrimitiveTopology::LineList,
            true,
        );

        // refs: layout do grupo 1 (params + textura + sampler)
        let ref_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("simple3d-ref-tex-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let ref_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-ref-layout"),
            bind_group_layouts: &[Some(&cam_layout), Some(&ref_tex_layout)],
            immediate_size: 0,
        });
        let ref_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-ref-pipe"),
            layout: Some(&ref_layout),
            vertex: wgpu::VertexState {
                module: &ref_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RefVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ref_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: msaa,
            multiview_mask: None,
            cache: None,
        });

        let ref_xray_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-ref-xray-pipeline"),
            layout: Some(&ref_layout),
            vertex: wgpu::VertexState {
                module: &ref_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RefVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ref_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: msaa,
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("simple3d-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Malha texturizada (paridade GL): grupo 1 = textura + sampler.
        let mesh_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("simple3d-mesh-tex-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let mesh_tex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-mesh-tex"),
            source: wgpu::ShaderSource::Wgsl(mesh_tex_wgsl().into()),
        });
        let mesh_tex_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-mesh-tex-layout"),
            bind_group_layouts: &[Some(&cam_layout), Some(&mesh_tex_layout)],
            immediate_size: 0,
        });
        let tex_vb_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2],
        };
        let mesh_tex_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-mesh-tex-pipe"),
            layout: Some(&mesh_tex_pipe_layout),
            vertex: wgpu::VertexState {
                module: &mesh_tex_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(tex_vb_layout.clone())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_tex_shader,
                entry_point: Some("fs_tex"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: msaa,
            multiview_mask: None,
            cache: None,
        });
        let mesh_tex_xray_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("simple3d-mesh-tex-xray-pipe"),
                layout: Some(&mesh_tex_pipe_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_tex_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(tex_vb_layout)],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_tex_shader,
                    entry_point: Some("fs_tex_xray"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: msaa,
                multiview_mask: None,
                cache: None,
            });

        let grid = grid_lines();
        let grid_count = grid.len() as u32;
        let grid_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simple3d-grid"),
            contents: bytemuck::cast_slice(&grid),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            depth_format: wgpu::TextureFormat::Depth24Plus,
            depth_view: None,
            depth_size: (0, 0),
            sample_count,
            pixel_ratio: 1.0,
            mesh_pipeline,
            mesh_xray_pipeline,
            mesh_tex_pipeline,
            mesh_tex_xray_pipeline,
            line_pipeline,
            xray: false,
            xray_opacity: 0.42,
            selection_rgb: [233, 106, 0],
            selection_thickness: 2.0,
            ref_pipeline,
            ref_xray_pipeline,
            cam_buffer,
            cam_bind_group,
            ref_tex_layout,
            mesh_tex_layout,
            sampler,
            mesh_vb: None,
            mesh_count: 0,
            mesh_ranges: Vec::new(),
            asset_tex: Vec::new(),
            line_vb: None,
            line_count: 0,
            wide_line_pipeline,
            wide_line_xray_pipeline,
            line_width_px: DEFAULT_LINE_WIDTH_PX,
            edge_mode: EdgeMode::Overlay,
            workplane: None,
            selection_dirty: false,
            selection_tri_pipeline,
            selection_line_pipeline,
            selection_tri_xray_pipeline,
            selection_line_xray_pipeline,
            selection_tri_vb: None,
            selection_tri_count: 0,
            selection_line_vb: None,
            selection_line_count: 0,
            grid_vb,
            grid_count,
            ref_vb: None,
            ref_count: 0,
            ref_gpu: Vec::new(),
            show_overlays: true,
            show_grid: true,
            last_fingerprint: None,
            last_domain: None,
            grid_step: 1.0,
            last_hover: petunia_core::HoverTarget::None,
            studio_light_follows_camera: true,
            last_selection_view_proj: None,
            mesh_rebuilds: 0,
            skipped_frames: 0,
            pending_texture_updates: Vec::new(),
            texture_upload_scratch: Vec::new(),
            texture_upload_calls: 0,
            texture_bytes_uploaded: 0,
            full_texture_uploads: 0,
            partial_texture_uploads: 0,
        }
    }

    /// Quantas vezes os buffers de geometria foram reconstruídos (telemetria Wave 1).
    /// Vértices de aresta enviados à GPU (6 por aresta: faixa de 2 triângulos).
    pub fn edge_vertex_count(&self) -> u32 {
        self.line_count
    }

    pub fn mesh_rebuilds(&self) -> u64 {
        self.mesh_rebuilds
    }

    /// Quantos `update()` pularam reconstrução por fingerprint idêntico.
    pub fn skipped_frames(&self) -> u64 {
        self.skipped_frames
    }

    pub fn texture_upload_calls(&self) -> u64 {
        self.texture_upload_calls
    }

    pub fn texture_bytes_uploaded(&self) -> u64 {
        self.texture_bytes_uploaded
    }

    pub fn full_texture_uploads(&self) -> u64 {
        self.full_texture_uploads
    }

    pub fn partial_texture_uploads(&self) -> u64 {
        self.partial_texture_uploads
    }

    pub fn queue_texture_updates(&mut self, updates: Vec<TextureUpdate>) {
        self.pending_texture_updates.extend(updates);
    }

    /// Invalida o cache manualmente (ex. após troca de backend ou teste).
    pub fn invalidate_cache(&mut self) {
        self.last_fingerprint = None;
    }

    /// Opacidade da geometria em X-Ray, aplicada no uniform do shader.
    /// Luz de estúdio (Solid/Material): `true` acompanha a câmera, como no
    /// Plasticity e no Cinema 4D — a forma continua legível de qualquer lado;
    /// `false` a mantém fixa no mundo.
    pub fn set_studio_light_follows_camera(&mut self, follows: bool) {
        self.studio_light_follows_camera = follows;
    }

    /// Plano de trabalho em destaque (DRAW com a ferramenta de desenho).
    pub fn set_workplane(&mut self, workplane: Option<WorkplaneOverlay>) {
        if self.workplane != workplane {
            self.workplane = workplane;
            self.selection_dirty = true;
        }
    }

    /// Aparência das arestas (DRAW/POLY/overlay); mudar reconstrói as linhas.
    pub fn set_edge_mode(&mut self, mode: EdgeMode) {
        if self.edge_mode != mode {
            self.edge_mode = mode;
            self.last_fingerprint = None;
        }
    }

    /// Largura das arestas em px lógicos (constante em qualquer zoom e DPI).
    pub fn set_line_width_px(&mut self, width: f32) {
        if width.is_finite() {
            self.line_width_px = width.clamp(1.0, 8.0);
        }
    }

    pub fn set_xray_opacity(&mut self, opacity: f32) {
        self.xray_opacity = opacity.clamp(0.1, 0.9);
    }

    pub fn set_selection_style(&mut self, rgb: [u8; 3], thickness: f32) {
        let thickness = thickness.clamp(1.0, 6.0);
        if self.selection_rgb != rgb || (self.selection_thickness - thickness).abs() > f32::EPSILON
        {
            self.selection_rgb = rgb;
            self.selection_thickness = thickness;
            self.last_selection_view_proj = None;
        }
    }

    /// Amostras por pixel usadas pelas pipelines deste renderer.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Define quantos pixels físicos do alvo correspondem a um pixel lógico.
    pub fn set_pixel_ratio(&mut self, ratio: f32) {
        let ratio = if ratio.is_finite() && ratio > 0.0 {
            ratio.clamp(0.5, 4.0)
        } else {
            1.0
        };
        if (self.pixel_ratio - ratio).abs() > f32::EPSILON {
            self.pixel_ratio = ratio;
            // Bandas e discos de seleção dependem da altura lógica.
            self.last_selection_view_proj = None;
        }
    }

    /// Altura do alvo em px lógicos: base das larguras de overlay.
    fn logical_height(&self) -> u32 {
        (self.depth_size.1 as f32 / self.pixel_ratio)
            .round()
            .max(1.0) as u32
    }

    pub fn set_overlays(&mut self, show_overlays: bool, show_grid: bool) {
        self.show_overlays = show_overlays;
        self.show_grid = show_grid;
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.depth_size == (width, height) && self.depth_view.is_some() {
            return;
        }
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("simple3d-depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: self.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_view = Some(tex.create_view(&Default::default()));
        self.depth_size = (width, height);
    }

    /// Atualiza uniforms de câmera todo frame; reconstrói buffers de geometria
    /// somente quando o fingerprint da cena muda (Wave 1 — P0-A).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Project,
        refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        shading: Shading,
        xray: bool,
        show_triangulation: bool,
        textured: bool,
        show_wireframe_overlay: bool,
        show_face_orientation: bool,
        show_uv_checker: bool,
        edit_domain: petunia_core::SelectionDomain,
        hover: petunia_core::HoverTarget,
    ) {
        puffin::profile_function!();
        self.xray = xray;
        // Trocar de domínio muda a camada de seleção, não só a malha.
        let domain_changed = self.last_domain != Some(edit_domain);
        self.last_domain = Some(edit_domain);
        // Preselection entra no fingerprint: mover o mouse sobre a geometria
        // precisa redesenhar a camada, mas nada mais.
        let hover_changed = self.last_hover != hover;
        self.last_hover = hover;
        let selection_view_proj = camera.view_proj().to_cols_array();
        let camera_changed = self.last_selection_view_proj != Some(selection_view_proj);
        self.last_selection_view_proj = Some(selection_view_proj);
        // Grid adaptativo: reconstrói só quando a escala visível cruza um degrau.
        let wanted_step = adaptive_grid_step(camera.visible_height());
        if (wanted_step - self.grid_step).abs() > f32::EPSILON {
            self.grid_step = wanted_step;
            let grid = adaptive_grid_lines(camera.visible_height());
            self.grid_count = grid.len() as u32;
            self.grid_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("simple3d-grid"),
                contents: bytemuck::cast_slice(&grid),
                usage: wgpu::BufferUsages::VERTEX,
            });
        }
        // Fonte de luz por modo: Solid/Material usam o estúdio fixo da viewport,
        // Rendered usa a primeira luz habilitada da cena. Sem luz na cena o
        // Rendered cai no estúdio em vez de renderizar preto.
        let (light_dir, ambient, diffuse) = match (shading.uses_scene_light(), scene.active_light())
        {
            (true, Some(light)) => {
                let [x, y, z] = light.normalized_direction();
                let intensity = light.intensity.clamp(0.0, 8.0);
                (
                    [x, y, z, 0.0],
                    petunia_render::scene::LIGHT_AMBIENT * 0.35,
                    petunia_render::scene::LIGHT_DIFFUSE * intensity,
                )
            }
            _ => {
                let [x, y, z] = if self.studio_light_follows_camera {
                    petunia_render::scene::studio_light_for_camera(camera)
                } else {
                    petunia_render::scene::LIGHT_DIR
                };
                (
                    [x, y, z, 0.0],
                    petunia_render::scene::LIGHT_AMBIENT,
                    petunia_render::scene::LIGHT_DIFFUSE,
                )
            }
        };
        queue.write_buffer(
            &self.cam_buffer,
            0,
            bytemuck::cast_slice(&[CameraUniform {
                view_proj: camera.view_proj().to_cols_array_2d(),
                light_dir,
                light_params: [ambient, diffuse, 0.0, 0.0],
                xray: [
                    self.xray_opacity,
                    self.depth_size.0.max(1) as f32,
                    self.depth_size.1.max(1) as f32,
                    self.line_width_px * self.pixel_ratio,
                ],
            }]),
        );

        let fp = fingerprint_scene(
            scene,
            refs,
            FingerprintFlags {
                shading,
                xray,
                show_triangulation,
                textured,
                edit_mode_is_edit: false,
                show_wireframe_overlay,
                show_face_orientation,
                show_uv_checker,
            },
        );
        let mesh_changed = self.last_fingerprint.map(|f| f.mesh) != Some(fp.mesh);
        let texture_changed = self.last_fingerprint.map(|f| f.textures) != Some(fp.textures);
        let refs_changed = self.last_fingerprint.map(|f| f.refs_layout) != Some(fp.refs_layout);
        let texture_updates = std::mem::take(&mut self.pending_texture_updates);
        let samples_textures = textured || shading.samples_material();
        if !mesh_changed && !refs_changed {
            if texture_changed || !texture_updates.is_empty() {
                self.sync_asset_textures(
                    device,
                    queue,
                    scene,
                    samples_textures,
                    &texture_updates,
                    texture_changed && texture_updates.is_empty(),
                );
            }
            self.last_fingerprint = Some(fp);
            if hover_changed
                || camera_changed
                || domain_changed
                || std::mem::take(&mut self.selection_dirty)
            {
                self.update_selection_layer(device, scene, camera, edit_domain, hover);
            }
            self.skipped_frames += 1;
            return;
        }
        self.last_fingerprint = Some(fp);
        // Qualquer mudança em geometria OU layout de refs reconstrói os buffers
        // de cena; câmera sozinha retorna cedo acima sem triangulação nem alloc.
        self.mesh_rebuilds += 1;

        // malha
        let mut mv: Vec<MeshVertex> = Vec::new();
        let mut lv: Vec<LineVertex> = Vec::new();
        let mut line_widths: Vec<f32> = Vec::new();
        let mut mesh_ranges: Vec<MeshRange> = Vec::new();
        // Wireframe não preenche; os outros três modos preenchem e diferem no
        // que amostram: cor do objeto, textura do material, ou material sob a
        // luz da cena. `smooth` é ortogonal: normais suavizadas por vértice.
        let is_wire = !shading.fills_faces();
        let unlit = false;
        for obj in &scene.assets {
            let smooth = scene.is_smooth_shaded(obj.id);
            if !obj.visible {
                continue;
            }
            let range_start = mv.len() as u32;
            let mesh = obj.evaluated_mesh_ref();
            if !is_wire {
                let (mat_profile, mat_color, has_emission, emission_color) =
                    if let Some(mat) = obj.material(scene) {
                        (
                            mat.profile,
                            [mat.base_color[0], mat.base_color[1], mat.base_color[2]],
                            mat.emission_strength > 0.0,
                            [
                                mat.emission_color[0] * mat.emission_strength,
                                mat.emission_color[1] * mat.emission_strength,
                                mat.emission_color[2] * mat.emission_strength,
                            ],
                        )
                    } else {
                        (
                            petunia_project::ShaderProfile::Pbr,
                            obj.base_color,
                            false,
                            [0.0, 0.0, 0.0],
                        )
                    };

                let obj_unlit = unlit
                    || mat_profile == petunia_project::ShaderProfile::Unlit
                    || mat_profile == petunia_project::ShaderProfile::Emissive;

                let tris = if obj_unlit {
                    mesh.to_triangles_unlit()
                } else {
                    mesh.to_triangles_smooth(smooth)
                };
                for (pos, n, mut col, uv) in tris {
                    if show_face_orientation {
                        let to_cam = (camera.eye() - glam::Vec3::from(pos)).normalize_or_zero();
                        let n_vec = glam::Vec3::from(n);
                        if n_vec.dot(to_cam) >= 0.0 {
                            col = [0.2, 0.45, 0.95];
                        } else {
                            col = [0.95, 0.2, 0.2];
                        }
                    } else if show_uv_checker {
                        let u_cell = (uv[0] * 16.0).floor() as i32;
                        let v_cell = (uv[1] * 16.0).floor() as i32;
                        let is_even = (u_cell + v_cell).rem_euclid(2) == 0;
                        let u_macro = u_cell.rem_euclid(8) == 0;
                        let v_macro = v_cell.rem_euclid(8) == 0;
                        if is_even {
                            if u_macro {
                                col = [0.95, 0.55, 0.45];
                            } else if v_macro {
                                col = [0.45, 0.75, 0.95];
                            } else {
                                col = [0.85, 0.85, 0.85];
                            }
                        } else {
                            if u_macro {
                                col = [0.45, 0.20, 0.18];
                            } else if v_macro {
                                col = [0.18, 0.28, 0.45];
                            } else {
                                col = [0.25, 0.25, 0.25];
                            }
                        }
                    } else {
                        if (col[0] - 0.72).abs() < 0.02
                            && (col[1] - 0.73).abs() < 0.02
                            && (col[2] - 0.78).abs() < 0.02
                        {
                            col = mat_color;
                        }
                        if has_emission {
                            col = [
                                (col[0] + emission_color[0]).min(1.0),
                                (col[1] + emission_color[1]).min(1.0),
                                (col[2] + emission_color[2]).min(1.0),
                            ];
                        }
                    }
                    mv.push(MeshVertex {
                        pos,
                        normal: n,
                        color: col,
                        uv,
                    });
                }
            }
            let range_count = mv.len() as u32 - range_start;
            if range_count > 0 {
                let tex_canvas = obj
                    .texture
                    .as_ref()
                    .or_else(|| obj.material(scene).and_then(|m| m.albedo_texture.as_ref()));
                mesh_ranges.push(MeshRange {
                    start: range_start,
                    count: range_count,
                    // Material Preview e Rendered sempre amostram o material; nos
                    // outros modos a textura é opt-in pelo toggle `textured`.
                    asset_id: if (textured || shading.samples_material()) && tex_canvas.is_some() {
                        Some(obj.id)
                    } else {
                        None
                    },
                });
            }

            if is_wire {
                // Wireframe mostra topologia, não seleção: arestas neutras para
                // a camada de seleção continuar sendo o único destaque.
                for (a, b, sel) in mesh.to_edges() {
                    let c = if sel {
                        [1.0, 0.62, 0.20]
                    } else {
                        [0.62, 0.66, 0.74]
                    };
                    lv.push(LineVertex { pos: a, color: c });
                    lv.push(LineVertex { pos: b, color: c });
                    line_widths.push(1.0);
                }
            } else {
                // Aparência por modo (capítulo 05): DRAW lê forma (só arestas
                // de feição), POLY lê topologia (todas, feição reforçada); o
                // overlay de wireframe acrescenta as arestas finas em qualquer
                // modo. Sem modo nem overlay, faces limpas.
                for (a, b, _sel, feature) in mesh.to_classified_edges(CREASE_DEGREES) {
                    let show_thin = show_wireframe_overlay || self.edge_mode == EdgeMode::Topology;
                    let show_feature = show_thin || self.edge_mode == EdgeMode::Features;
                    let (visible, color, width) = if feature {
                        (show_feature, FEATURE_EDGE_COLOR, 1.0)
                    } else {
                        (show_thin, THIN_EDGE_COLOR, THIN_EDGE_SCALE)
                    };
                    if visible {
                        lv.push(LineVertex { pos: a, color });
                        lv.push(LineVertex { pos: b, color });
                        line_widths.push(width);
                    }
                }
            }
            if show_triangulation {
                let diag_c = [0.3, 0.65, 0.95];
                for (a, b) in mesh.triangulation_wireframe() {
                    line_widths.push(THIN_EDGE_SCALE);
                    lv.push(LineVertex {
                        pos: a,
                        color: diag_c,
                    });
                    lv.push(LineVertex {
                        pos: b,
                        color: diag_c,
                    });
                }
            }
        }
        self.update_selection_layer(device, scene, camera, edit_domain, hover);

        self.mesh_count = mv.len() as u32;
        self.mesh_ranges = mesh_ranges;
        self.sync_asset_textures(
            device,
            queue,
            scene,
            samples_textures,
            &texture_updates,
            texture_changed && texture_updates.is_empty(),
        );
        self.mesh_vb = if mv.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-mesh-vb"),
                    contents: bytemuck::cast_slice(&mv),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        let wide = wide_lines_from_pairs(&lv, &line_widths);
        self.line_count = wide.len() as u32;
        self.line_vb = if wide.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-edge-vb"),
                    contents: bytemuck::cast_slice(&wide),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };

        // referências: garante texturas e monta quads (matemática em render::scene)
        self.ensure_ref_textures(device, refs);
        let mut rv: Vec<RefVertex> = Vec::new();
        for r in refs.iter().filter(|r| r.visible) {
            let plane = match r.axis {
                RefAxis::Front => petunia_render::scene::RefPlane::Front,
                RefAxis::Back => petunia_render::scene::RefPlane::Back,
                RefAxis::Left => petunia_render::scene::RefPlane::Left,
                RefAxis::Right | RefAxis::Side => petunia_render::scene::RefPlane::Right,
                RefAxis::Top => petunia_render::scene::RefPlane::Top,
                RefAxis::Bottom => petunia_render::scene::RefPlane::Bottom,
            };
            let aspect = r.width as f32 / r.height.max(1) as f32;
            let quad = petunia_render::scene::ref_quad_with_rot(
                plane, r.offset, r.size, aspect, r.rotation,
            );
            for (p, uv) in quad.iter().zip(petunia_render::scene::QUAD_UVS_TOP_LEFT) {
                rv.push(RefVertex { pos: *p, uv });
            }
        }
        // expande quads (4 verts) p/ 2 tris (6 verts)
        let mut tris: Vec<RefVertex> = Vec::new();
        for q in rv.chunks(4) {
            if q.len() == 4 {
                tris.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
            }
        }
        self.ref_count = tris.len() as u32;
        self.ref_vb = if tris.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-ref-vb"),
                    contents: bytemuck::cast_slice(&tris),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        let _ = queue;
    }

    /// Atualiza somente os buffers de seleção e preselection. Mover o cursor
    /// não reconstrói a malha, texturas ou referências da cena.
    fn update_selection_layer(
        &mut self,
        device: &wgpu::Device,
        scene: &Project,
        camera: &Camera,
        edit_domain: petunia_core::SelectionDomain,
        hover: petunia_core::HoverTarget,
    ) {
        // Camada de seleção: geometria própria, com depth test no render. Só o
        // ativo contribui, e só o domínio atual — um vértice selecionado não
        // pode virar face pintada, que era a contaminação antiga.
        // Larguras e raios em px lógicos, mesmo com o alvo em px físicos.
        let logical_height = self.logical_height();
        let mut sel_tri: Vec<SelectionVertex> = Vec::new();
        let mut sel_line: Vec<SelectionVertex> = Vec::new();
        if let Some(asset) = scene.assets.get(scene.active) {
            let mesh = asset.evaluated_mesh_ref();
            let domain = edit_domain;
            // Seleção: laranja quente com alpha, como Blender/C4D. Legível
            // sobre qualquer shading porque o shader não aplica luz.
            let selected = self.selection_rgb.map(|channel| channel as f32 / 255.0);
            let face_color = [selected[0], selected[1], selected[2], 0.32];
            let edge_color = [selected[0], selected[1], selected[2], 1.0];
            let point_color = edge_color;

            if domain == petunia_core::SelectionDomain::Edge {
                let guide_color = [0.70, 0.75, 0.83, 0.76];
                for (a, b) in mesh.edges_unique() {
                    if mesh.selected_edges.contains(&(a, b)) {
                        continue;
                    }
                    let (Some(start), Some(end)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    else {
                        continue;
                    };
                    sel_line.push(SelectionVertex {
                        pos: start.pos,
                        color: guide_color,
                    });
                    sel_line.push(SelectionVertex {
                        pos: end.pos,
                        color: guide_color,
                    });
                }
            }

            if domain == petunia_core::SelectionDomain::Vertex {
                let guide_color = [0.73, 0.78, 0.85, 0.9];
                for vertex in mesh.verts.iter().filter(|vertex| !vertex.selected) {
                    append_point_disc(
                        &mut sel_tri,
                        vertex.vec(),
                        camera,
                        logical_height,
                        (self.selection_thickness * 1.5).clamp(3.5, 5.5),
                        guide_color,
                    );
                }
            }

            if domain == petunia_core::SelectionDomain::Face {
                for (fi, face) in mesh
                    .faces
                    .iter()
                    .enumerate()
                    .filter(|(_, face)| face.selected)
                {
                    if face.verts.len() < 3 {
                        continue;
                    }
                    for corners in mesh.face_triangle_corners(fi) {
                        let [p0, p1, p2] =
                            corners.map(|i| mesh.verts[face.verts[i] as usize].vec());
                        for point in [p0, p1, p2] {
                            sel_tri.push(SelectionVertex {
                                pos: point.to_array(),
                                color: face_color,
                            });
                        }
                    }
                }
            }

            if domain == petunia_core::SelectionDomain::Edge {
                for &(a, b) in &mesh.selected_edges {
                    let (Some(va), Some(vb)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    else {
                        continue;
                    };
                    append_edge_band(
                        &mut sel_tri,
                        va.vec(),
                        vb.vec(),
                        camera,
                        logical_height,
                        self.selection_thickness.max(2.5),
                        edge_color,
                    );
                }
            }

            if domain == petunia_core::SelectionDomain::Vertex {
                for vertex in mesh.verts.iter().filter(|vertex| vertex.selected) {
                    append_point_disc(
                        &mut sel_tri,
                        vertex.vec(),
                        camera,
                        logical_height,
                        (self.selection_thickness * 2.0).clamp(5.0, 7.0),
                        point_color,
                    );
                }
            }
        }
        // Preselection: mesma linguagem da seleção, porém mais fraca — o
        // usuário vê o que vai clicar sem confundir com o que já selecionou.
        // Ciano #7DDCFF único em GPU, software e shell (era cinza-azulado
        // só aqui, destoando do hover do restante da UI).
        let hover_line = [0.49f32, 0.86, 1.0, 0.85];
        let hover_tri = [0.49f32, 0.86, 1.0, 0.18];
        if let Some(asset) = scene.assets.get(scene.active) {
            let mesh = asset.evaluated_mesh_ref();
            match hover {
                petunia_core::HoverTarget::Vertex(index) => {
                    if let Some(vertex) = mesh.verts.get(index as usize) {
                        append_point_disc(
                            &mut sel_tri,
                            vertex.vec(),
                            camera,
                            logical_height,
                            (self.selection_thickness * 2.5).clamp(6.5, 8.5),
                            hover_line,
                        );
                    }
                }
                petunia_core::HoverTarget::Edge(a, b) => {
                    if let (Some(va), Some(vb)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    {
                        append_edge_band(
                            &mut sel_tri,
                            va.vec(),
                            vb.vec(),
                            camera,
                            logical_height,
                            self.selection_thickness * 1.35,
                            hover_line,
                        );
                    }
                }
                petunia_core::HoverTarget::Face(index) => {
                    if let Some(face) = mesh.faces.get(index)
                        && face.verts.len() >= 3
                    {
                        for corners in mesh.face_triangle_corners(index) {
                            let [p0, p1, p2] =
                                corners.map(|i| mesh.verts[face.verts[i] as usize].vec());
                            for point in [p0, p1, p2] {
                                sel_tri.push(SelectionVertex {
                                    pos: point.to_array(),
                                    color: hover_tri,
                                });
                            }
                        }
                    }
                }
                petunia_core::HoverTarget::Object(_) | petunia_core::HoverTarget::None => {}
            }
        }
        if let Some(plane) = self.workplane {
            append_workplane(&mut sel_tri, plane, camera, logical_height);
        }
        self.selection_dirty = false;
        self.selection_tri_count = sel_tri.len() as u32;
        self.selection_tri_vb = if sel_tri.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-selection-tri"),
                    contents: bytemuck::cast_slice(&sel_tri),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        self.selection_line_count = sel_line.len() as u32;
        self.selection_line_vb = if sel_line.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-selection-line"),
                    contents: bytemuck::cast_slice(&sel_line),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
    }

    fn ensure_ref_textures(
        &mut self,
        device: &wgpu::Device,
        refs: &[petunia_core::ReferenceImage],
    ) {
        // Reconstrói slots cujo tamanho/conteúdo mudou; mantém os demais (evita re-upload).
        while self.ref_gpu.len() < refs.len() {
            self.ref_gpu
                .push(self.make_ref_slot(device, &refs[self.ref_gpu.len()]));
        }
        self.ref_gpu.truncate(refs.len());
        for (i, r) in refs.iter().enumerate() {
            let slot = &self.ref_gpu[i];
            if slot.width != r.width || slot.height != r.height || slot.len != r.rgba.len() {
                self.ref_gpu[i] = self.make_ref_slot(device, r);
            }
        }
    }

    fn make_ref_slot(&self, device: &wgpu::Device, r: &petunia_core::ReferenceImage) -> RefGpu {
        let (w, h) = (r.width.max(1), r.height.max(1));
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("simple3d-ref"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simple3d-ref-params-slot"),
            size: std::mem::size_of::<RefUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view = tex.create_view(&Default::default());
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simple3d-ref-bg"),
            layout: &self.ref_tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        RefGpu {
            width: r.width,
            height: r.height,
            len: r.rgba.len(),
            hash: std::cell::Cell::new(0),
            texture: tex,
            view,
            params,
            bind_group: bg,
        }
    }

    /// Garante slots de textura dos assets com canvas, consumindo dirty regions
    /// declaradas pelo domínio. Sem declaração, uma mudança de fingerprint usa
    /// upload integral conservador para preservar call sites legados.
    fn sync_asset_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Project,
        textured: bool,
        updates: &[TextureUpdate],
        force_full_without_updates: bool,
    ) {
        use petunia_project::Canvas;
        if !textured {
            self.asset_tex.clear();
            return;
        }

        let wanted: Vec<(uuid::Uuid, &Canvas)> = scene
            .assets
            .iter()
            .filter(|o| o.visible)
            .filter_map(|o| {
                let cv: &Canvas = o
                    .texture
                    .as_ref()
                    .or_else(|| o.material(scene).and_then(|m| m.albedo_texture.as_ref()))?;
                Some((o.id, cv))
            })
            .collect();
        // Poda slots sem canvas/asset correspondente.
        self.asset_tex.retain(|slot| {
            wanted
                .iter()
                .any(|(id, cv)| *id == slot.asset_id && cv.w == slot.width && cv.h == slot.height)
        });
        let declared_updates = !updates.is_empty();
        let mut upload_calls = 0_u64;
        let mut uploaded_bytes = 0_u64;
        let mut full_uploads = 0_u64;
        let mut partial_uploads = 0_u64;
        let mut scratch = std::mem::take(&mut self.texture_upload_scratch);

        for (id, cv) in wanted {
            let (w, h, pixels) = (cv.w, cv.h, &cv.pixels);
            if let Some(slot) = self.asset_tex.iter_mut().find(|slot| slot.asset_id == id) {
                let update = updates.iter().find(|update| update.asset_id == id);
                let full_upload = update.is_some_and(TextureUpdate::is_full)
                    || force_full_without_updates
                    || (!declared_updates && slot.revision != scene.texture_revision);
                if full_upload {
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &slot.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        pixels,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * w),
                            rows_per_image: Some(h),
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    upload_calls += 1;
                    uploaded_bytes += pixels.len() as u64;
                    full_uploads += 1;
                } else if let Some(regions) = update.and_then(|update| update.regions.as_ref()) {
                    for &region in regions {
                        let Some(region) = region.clipped(w, h) else {
                            continue;
                        };
                        scratch.clear();
                        scratch.reserve(region.byte_len() as usize);
                        for row in region.y..region.y + region.height {
                            let start = ((row * w + region.x) * 4) as usize;
                            let end = start + (region.width * 4) as usize;
                            scratch.extend_from_slice(&pixels[start..end]);
                        }
                        queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &slot.texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d {
                                    x: region.x,
                                    y: region.y,
                                    z: 0,
                                },
                                aspect: wgpu::TextureAspect::All,
                            },
                            &scratch,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * region.width),
                                rows_per_image: Some(region.height),
                            },
                            wgpu::Extent3d {
                                width: region.width,
                                height: region.height,
                                depth_or_array_layers: 1,
                            },
                        );
                        upload_calls += 1;
                        uploaded_bytes += region.byte_len();
                        partial_uploads += 1;
                    }
                }
                slot.revision = scene.texture_revision;
                continue;
            }
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("simple3d-asset-tex"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("simple3d-asset-tex-bg"),
                layout: &self.mesh_tex_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.asset_tex.push(AssetTexGpu {
                asset_id: id,
                width: w,
                height: h,
                revision: scene.texture_revision,
                texture,
                view,
                bind_group,
            });
            upload_calls += 1;
            uploaded_bytes += pixels.len() as u64;
            full_uploads += 1;
        }
        self.texture_upload_calls += upload_calls;
        self.texture_bytes_uploaded += uploaded_bytes;
        self.full_texture_uploads += full_uploads;
        self.partial_texture_uploads += partial_uploads;
        self.texture_upload_scratch = scratch;
    }

    /// Upload dos pixels + opacidade por ref (chamado todo frame; pixels só sobem quando a revisão muda).
    pub fn upload_ref_pixels(&self, queue: &wgpu::Queue, refs: &[petunia_core::ReferenceImage]) {
        for (i, r) in refs.iter().enumerate() {
            let Some(slot) = self.ref_gpu.get(i) else {
                continue;
            };
            // Revisão monotônica em vez de FNV sobre todos os pixels a cada frame.
            let h = r.revision;
            if !r.rgba.is_empty()
                && (r.width, r.height) == (slot.width, slot.height)
                && r.rgba.len() == slot.len
                && slot.hash.get() != h
            {
                slot.hash.set(h);
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &slot.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &r.rgba,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * r.width),
                        rows_per_image: Some(r.height),
                    },
                    wgpu::Extent3d {
                        width: r.width,
                        height: r.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            queue.write_buffer(
                &slot.params,
                0,
                bytemuck::cast_slice(&[RefUniform {
                    opacity: if r.visible { r.opacity } else { 0.0 },
                    _pad: [0.0; 3],
                }]),
            );
        }
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>, refs: &[petunia_core::ReferenceImage]) {
        puffin::profile_function!();
        pass.set_bind_group(0, &self.cam_bind_group, &[]);

        // Grid 3D condicional aos overlays
        if self.show_overlays && self.show_grid {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.grid_vb.slice(..));
            pass.draw(0..self.grid_count, 0..1);
        }

        // Referências padrão (não X-Ray): desenhadas ANTES da geometria sólida com depth test
        if self.show_overlays
            && let Some(vb) = &self.ref_vb
        {
            pass.set_pipeline(&self.ref_pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            let mut start = 0u32;
            for (i, r) in refs.iter().enumerate().filter(|(_, r)| r.visible) {
                if !r.xray
                    && let Some(slot) = self.ref_gpu.get(i)
                {
                    pass.set_bind_group(1, &slot.bind_group, &[]);
                    pass.draw(start..start + 6, 0..1);
                }
                start += 6;
            }
        }

        // malha sólida (ou raio-x com transparência); ranges com canvas usam
        // o pipeline texturizado (paridade com o backend GL).
        if let Some(vb) = &self.mesh_vb {
            pass.set_vertex_buffer(0, vb.slice(..));
            if self.mesh_ranges.is_empty() {
                if self.xray {
                    pass.set_pipeline(&self.mesh_xray_pipeline);
                } else {
                    pass.set_pipeline(&self.mesh_pipeline);
                }
                pass.draw(0..self.mesh_count, 0..1);
            } else {
                for range in &self.mesh_ranges {
                    let tex_bg = range
                        .asset_id
                        .and_then(|id| self.asset_tex.iter().find(|s| s.asset_id == id));
                    match (tex_bg, self.xray) {
                        (Some(slot), false) => {
                            pass.set_pipeline(&self.mesh_tex_pipeline);
                            pass.set_bind_group(1, &slot.bind_group, &[]);
                        }
                        (Some(slot), true) => {
                            pass.set_pipeline(&self.mesh_tex_xray_pipeline);
                            pass.set_bind_group(1, &slot.bind_group, &[]);
                        }
                        (None, true) => {
                            pass.set_pipeline(&self.mesh_xray_pipeline);
                        }
                        (None, false) => {
                            pass.set_pipeline(&self.mesh_pipeline);
                        }
                    }
                    pass.draw(range.start..range.start + range.count, 0..1);
                }
            }
        }
        // Seleção: preenchimento translúcido e contorno/marcadores, ambos com
        // depth test. Fica depois da geometria e antes das arestas para que o
        // wireframe permaneça legível por cima da seleção.
        if let Some(vb) = &self.selection_tri_vb {
            pass.set_pipeline(if self.xray {
                &self.selection_tri_xray_pipeline
            } else {
                &self.selection_tri_pipeline
            });
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.selection_tri_count, 0..1);
        }
        if let Some(vb) = &self.selection_line_vb {
            pass.set_pipeline(if self.xray {
                &self.selection_line_xray_pipeline
            } else {
                &self.selection_line_pipeline
            });
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.selection_line_count, 0..1);
        }

        // arestas (faixas de largura constante)
        if let Some(vb) = &self.line_vb {
            if self.xray {
                pass.set_pipeline(&self.wide_line_xray_pipeline);
            } else {
                pass.set_pipeline(&self.wide_line_pipeline);
            }
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.line_count, 0..1);
        }

        // Referências X-Ray: overlay pass desenhado APÓS a geometria com depth test bypass
        if self.show_overlays
            && let Some(vb) = &self.ref_vb
        {
            pass.set_pipeline(&self.ref_xray_pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            let mut start = 0u32;
            for (i, r) in refs.iter().enumerate().filter(|(_, r)| r.visible) {
                if r.xray
                    && let Some(slot) = self.ref_gpu.get(i)
                {
                    pass.set_bind_group(1, &slot.bind_group, &[]);
                    pass.draw(start..start + 6, 0..1);
                }
                start += 6;
            }
        }

        let _ = Vec3::ZERO;
    }

    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_view.as_ref()
    }
}
