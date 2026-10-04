//! glTF/GLB import boundary (`gltf`, `gltf-json`, P0-05).
//!
//! glTF *export* already produces binary GLB in [`crate::export`]. This module
//! owns the import side: parsing GLB (`.glb`) and glTF JSON (`.gltf`) with
//! geometry (positions, indices, UVs, vertex colors), PBR materials and
//! embedded/external textures, headless and independent from file dialogs
//! and UI.
//!
//! Conventions:
//! - GLB is self-contained (BIN chunk). `.gltf` may reference external
//!   `.bin` buffers and images relative to a base directory, or `data:`
//!   URIs. Network URIs are rejected.
//! - Node transforms are baked into positions (world space), so files
//!   authored with rotated/scaled nodes look the same in Petunia.
//! - UVs (`TEXCOORD_0`) go to per-face-corner `Face.uv`; vertex colors
//!   (`COLOR_0`) go to `Vertex.color`. Missing attributes fall back to
//!   `[0,0]` and the default grey.
//! - One Petunia [`Mesh`] per glTF mesh (primitives merged). Per-face
//!   `material_slot` points into the returned [`GltfImport::materials`].
//! - Textures are decoded via `image` (PNG/JPEG) into [`crate::Canvas`]
//!   (max 1024, clamped by `Canvas::new`). Failures become warnings,
//!   never silent drops nor panics.

/// Structural summary of a validated glTF JSON document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GltfSummary {
    /// `asset.version`, e.g. `"2.0"`.
    pub version: String,
    /// Number of scenes.
    pub scenes: usize,
    /// Number of nodes.
    pub nodes: usize,
    /// Number of meshes.
    pub meshes: usize,
    /// Number of buffers (external or GLB chunk references).
    pub buffers: usize,
}

/// glTF import failure with a stable message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GltfImportError {
    /// The document is not valid JSON / glTF.
    #[error("glTF parse error: {0}")]
    Parse(String),
    /// The document parses but violates structural expectations.
    #[error("glTF validation error: {0}")]
    Validation(String),
    /// The file exceeds safety limits.
    #[error("glTF exceeds size limits")]
    TooLarge,
}

/// Skin/animação de um GLB (cap. 45 F0). Ver [`crate::gltf_rig::import_rig`].
pub use crate::gltf_rig::import_rig;
use gltf_json::validation::Validate;
use petunia_mesh::{Face, Mesh, Vertex};
use std::path::Path;

/// Imported GLB meshes (name, mesh).
pub type GlbMeshes = Vec<(String, Mesh)>;

/// Full glTF/GLB import: geometry + PBR materials + textures.
///
/// `mesh_materials[i]` is the dominant material index for `meshes[i]`
/// (first primitive with a material), or `None` when unassigned.
/// Per-face `Face.material_slot` is also set to the material index
/// (local to [`GltfImport::materials`]) whenever the primitive has one.
#[derive(Clone, Debug, Default)]
pub struct GltfImport {
    pub meshes: Vec<(String, Mesh)>,
    pub materials: Vec<crate::Material>,
    pub mesh_materials: Vec<Option<usize>>,
    pub warnings: Vec<String>,
}

/// Limits guarding against hostile files (zip-bomb class).
const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;
const MAX_BUFFER_BYTES: usize = 256 * 1024 * 1024;
const MAX_IMAGES: usize = 64;
const MAX_CANVAS_DIM: u32 = 1024;

/// Imports a GLB (or glTF with embedded buffers) into meshes.
///
/// Kept for backward compatibility (geometry only). UVs and vertex colors
/// are now preserved; materials/textures are dropped — use
/// [`import_scene_bytes`] when you need them.
pub fn import_glb_bytes(
    data: &[u8],
    name_hint: &str,
    triangulate: bool,
    scale: f32,
) -> Result<GlbMeshes, GltfImportError> {
    let scene = import_scene_bytes(data, None, name_hint, triangulate, scale, false)?;
    Ok(scene.meshes)
}

/// Full import of GLB **or** `.gltf` JSON with materials and textures.
///
/// `base_dir` resolves external `.bin` buffers and image URIs for `.gltf`
/// files (typically `path.parent()`). `None` still supports GLB plus
/// embedded `data:` URIs. Set `import_materials` to false to skip texture
/// decoding (geometry only).
pub fn import_scene_bytes(
    data: &[u8],
    base_dir: Option<&Path>,
    name_hint: &str,
    triangulate: bool,
    scale: f32,
    import_materials: bool,
) -> Result<GltfImport, GltfImportError> {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let gltf = gltf::Gltf::from_slice(data).map_err(|e| GltfImportError::Parse(e.to_string()))?;

    // Structural version gate (same rule as parse_gltf_json).
    let version = gltf.as_json().asset.version.clone();
    let major = version
        .split('.')
        .next()
        .and_then(|n| n.parse::<u32>().ok())
        .ok_or_else(|| GltfImportError::Validation(format!("bad asset.version '{version}'")))?;
    if major != 2 {
        return Err(GltfImportError::Validation(format!(
            "unsupported glTF major version {major}"
        )));
    }

    let buffers = load_buffers(&gltf, data, base_dir)?;
    let get = |b: gltf::Buffer<'_>| buffers.get(b.index()).map(|v| v.as_slice());

    let mut warnings: Vec<String> = Vec::new();

    // ---- Materials (+ textures) ----
    let materials: Vec<crate::Material> = if import_materials {
        import_materials_list(&gltf, &buffers, base_dir, &mut warnings)
    } else {
        Vec::new()
    };
    // Map glTF material index -> position in `materials` (1:1 when enabled).
    let mat_lut: Vec<Option<usize>> = (0..gltf.materials().len())
        .map(|i| if import_materials { Some(i) } else { None })
        .collect();

    // ---- Node world transforms (baked) ----
    let world = node_world_matrices(&gltf);
    // mesh index -> list of (node name, world matrix) instancing it.
    let mut mesh_instances: Vec<Vec<(Option<String>, glam::Mat4)>> =
        vec![Vec::new(); gltf.meshes().len()];
    for node in gltf.nodes() {
        if let Some(mesh) = node.mesh() {
            let name = node.name().map(|s| s.to_string());
            mesh_instances[mesh.index()].push((name, world[node.index()]));
        }
    }

    // ---- Geometry ----
    let mut out_meshes: Vec<(String, Mesh)> = Vec::new();
    let mut out_mesh_materials: Vec<Option<usize>> = Vec::new();

    for (mi, mesh) in gltf.meshes().enumerate() {
        let instances = if mesh_instances[mi].is_empty() {
            vec![(None, glam::Mat4::IDENTITY)]
        } else {
            mesh_instances[mi].clone()
        };
        for (inst_i, (node_name, matrix)) in instances.iter().enumerate() {
            let mut out = Mesh::default();
            let mut dominant: Option<usize> = None;
            let mut skipped_modes = 0;
            let mut total_verts = 0usize;

            for primitive in mesh.primitives() {
                let mode = primitive.mode();
                // Resolve indices according to primitive mode.
                let index_list: Option<Vec<u32>> = {
                    let reader = primitive.reader(&get);
                    match mode {
                        gltf::mesh::Mode::Triangles => reader
                            .read_indices()
                            .map(|it| it.into_u32().collect::<Vec<_>>()),
                        gltf::mesh::Mode::TriangleStrip | gltf::mesh::Mode::TriangleFan => {
                            // Convert strip/fan to a triangle list.
                            let seq: Vec<u32> = match reader.read_indices() {
                                Some(it) => it.into_u32().collect(),
                                None => {
                                    // Non-indexed strip/fan: implicit 0..n.
                                    let n = reader
                                        .read_positions()
                                        .map(|it| it.len() as u32)
                                        .unwrap_or(0);
                                    (0..n).collect()
                                }
                            };
                            Some(strip_fan_to_tris(
                                &seq,
                                mode == gltf::mesh::Mode::TriangleStrip,
                            ))
                        }
                        _ => {
                            skipped_modes += 1;
                            continue;
                        }
                    }
                };
                let reader = primitive.reader(&get);
                let Some(positions) = reader.read_positions() else {
                    continue;
                };
                let positions: Vec<[f32; 3]> = positions.collect();
                if positions.is_empty() {
                    continue;
                }
                let uvs: Option<Vec<[f32; 2]>> =
                    reader.read_tex_coords(0).map(|it| it.into_f32().collect());
                let colors: Option<Vec<[f32; 3]>> =
                    reader.read_colors(0).map(|it| it.into_rgb_f32().collect());

                if total_verts + positions.len() > MAX_VERTICES {
                    return Err(GltfImportError::TooLarge);
                }
                let mat_idx = primitive
                    .material()
                    .index()
                    .and_then(|i| mat_lut.get(i).copied().flatten());

                let base = out.verts.len() as u32;
                for (vi, pos) in positions.iter().enumerate() {
                    let p = matrix.transform_point3(glam::Vec3::from_array(*pos)) * scale;
                    let mut v = Vertex::new(p.x, p.y, p.z);
                    if !v.pos.iter().all(|x| x.is_finite()) {
                        v.pos = [0.0, 0.0, 0.0];
                    }
                    if let Some(cols) = &colors
                        && let Some(c) = cols.get(vi)
                    {
                        let mut cc = *c;
                        if cc.iter().all(|x| x.is_finite()) {
                            for x in &mut cc {
                                *x = x.clamp(0.0, 1.0);
                            }
                            v.color = cc;
                        }
                    }
                    out.verts.push(v);
                }
                total_verts += positions.len();

                let local_mat_slot = mat_idx;
                if dominant.is_none() {
                    dominant = local_mat_slot;
                }
                // Emit faces.
                if let Some(idx) = index_list {
                    if idx.len() % 3 != 0 {
                        warnings.push(format!(
                            "malha '{}': índices incompletos ignorados",
                            mesh.name().unwrap_or("?")
                        ));
                    }
                    for tri in idx.chunks(3) {
                        if tri.len() != 3 {
                            continue;
                        }
                        if out.faces.len() >= MAX_FACES {
                            return Err(GltfImportError::TooLarge);
                        }
                        let vs = [base + tri[0], base + tri[1], base + tri[2]];
                        if vs.iter().any(|&i| (i as usize) >= out.verts.len()) {
                            continue;
                        }
                        let f_uv: Vec<[f32; 2]> = tri
                            .iter()
                            .map(|&i| {
                                sanitize_uv(uvs.as_ref().and_then(|u| u.get(i as usize).copied()))
                            })
                            .collect();
                        let mut face = Face::with_uv(vs.to_vec(), f_uv);
                        face.material_slot = local_mat_slot;
                        out.push_face(face);
                    }
                } else {
                    // Non-indexed triangles.
                    if !positions.len().is_multiple_of(3) {
                        warnings.push(format!(
                            "malha '{}': vértices não indexados incompletos",
                            mesh.name().unwrap_or("?")
                        ));
                    }
                    let n = positions.len() / 3;
                    for t in 0..n {
                        if out.faces.len() >= MAX_FACES {
                            return Err(GltfImportError::TooLarge);
                        }
                        let vs = [
                            base + (t * 3) as u32,
                            base + (t * 3 + 1) as u32,
                            base + (t * 3 + 2) as u32,
                        ];
                        let f_uv: Vec<[f32; 2]> = (0..3)
                            .map(|c| {
                                sanitize_uv(uvs.as_ref().and_then(|u| u.get(t * 3 + c).copied()))
                            })
                            .collect();
                        let mut face = Face::with_uv(vs.to_vec(), f_uv);
                        face.material_slot = local_mat_slot;
                        out.push_face(face);
                    }
                }
            }

            if skipped_modes > 0 {
                warnings.push(format!(
                    "malha '{}': {skipped_modes} primitiva(s) de pontos/linhas ignorada(s)",
                    mesh.name().unwrap_or("?")
                ));
            }
            if triangulate {
                // glTF is already triangles; keep quad-merging off — just validate.
                out.triangulate();
            }
            out.validate();
            // Remap check: validate() may drop degenerate faces; keep slot.
            if out.verts.is_empty() || out.faces.is_empty() {
                continue;
            }
            let base_name = mesh
                .name()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| node_name.clone())
                .unwrap_or_else(|| format!("{name_hint}_{mi}"));
            let name = if instances.len() > 1 {
                format!("{base_name}#{inst_i}")
            } else {
                base_name
            };
            out_meshes.push((name, out));
            out_mesh_materials.push(dominant);
        }
    }

    if out_meshes.is_empty() {
        return Err(GltfImportError::Validation(
            "GLB does not contain mesh primitives".into(),
        ));
    }
    Ok(GltfImport {
        meshes: out_meshes,
        mesh_materials: out_mesh_materials,
        materials,
        warnings,
    })
}

/// `.gltf` file import with external buffers/images resolved from `base_dir`.
pub fn import_gltf_file(
    path: &Path,
    name_hint: &str,
    triangulate: bool,
    scale: f32,
    import_materials: bool,
) -> Result<GltfImport, GltfImportError> {
    let data = std::fs::read(path).map_err(|e| GltfImportError::Parse(format!("I/O: {e}")))?;
    let base = path.parent();
    import_scene_bytes(&data, base, name_hint, triangulate, scale, import_materials)
}

fn sanitize_uv(uv: Option<[f32; 2]>) -> [f32; 2] {
    match uv {
        Some([u, v]) if u.is_finite() && v.is_finite() => [u, v],
        _ => [0.0, 0.0],
    }
}

fn strip_fan_to_tris(seq: &[u32], strip: bool) -> Vec<u32> {
    let mut out = Vec::new();
    if seq.len() < 3 {
        return out;
    }
    if strip {
        for i in 0..seq.len() - 2 {
            let (a, b, c) = (seq[i], seq[i + 1], seq[i + 2]);
            if a == b || b == c || a == c {
                continue;
            }
            // Alternate winding to preserve facing.
            if i % 2 == 0 {
                out.extend_from_slice(&[a, b, c]);
            } else {
                out.extend_from_slice(&[b, a, c]);
            }
        }
    } else {
        // Fan: (0, i, i+1).
        for i in 1..seq.len() - 1 {
            let (a, b, c) = (seq[0], seq[i], seq[i + 1]);
            if a == b || b == c || a == c {
                continue;
            }
            out.extend_from_slice(&[a, b, c]);
        }
    }
    out
}

fn node_world_matrices(gltf: &gltf::Gltf) -> Vec<glam::Mat4> {
    let n = gltf.nodes().len();
    let mut parent: Vec<Option<usize>> = vec![None; n];
    for node in gltf.nodes() {
        for child in node.children() {
            if child.index() < n {
                parent[child.index()] = Some(node.index());
            }
        }
    }
    let mut world = vec![glam::Mat4::IDENTITY; n];
    fn fill(node: gltf::Node, up: glam::Mat4, out: &mut [glam::Mat4]) {
        let local = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
        let w = up * local;
        if node.index() < out.len() {
            out[node.index()] = w;
        }
        for c in node.children() {
            fill(c, w, out);
        }
    }
    for node in gltf.nodes().filter(|nd| parent[nd.index()].is_none()) {
        fill(node, glam::Mat4::IDENTITY, &mut world);
    }
    // Sanitize non-finite matrices (hostile files).
    for m in &mut world {
        let cols = m.to_cols_array();
        if !cols.iter().all(|x| x.is_finite()) {
            *m = glam::Mat4::IDENTITY;
        }
    }
    world
}

fn load_buffers(
    gltf: &gltf::Gltf,
    data: &[u8],
    base_dir: Option<&Path>,
) -> Result<Vec<Vec<u8>>, GltfImportError> {
    let blob = gltf.blob.as_deref();
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(gltf.buffers().len().max(1));
    for buffer in gltf.buffers() {
        let bytes = match buffer.source() {
            gltf::buffer::Source::Bin => blob
                .map(|b| b.to_vec())
                .ok_or_else(|| GltfImportError::Validation("GLB sem chunk BIN".to_string()))?,
            gltf::buffer::Source::Uri(uri) => resolve_buffer_uri(uri, &buffer, base_dir)?,
        };
        if bytes.len() > MAX_BUFFER_BYTES {
            return Err(GltfImportError::TooLarge);
        }
        // Declared length check (truncation = corrupt, not panic).
        if bytes.len() < buffer.length() {
            return Err(GltfImportError::Validation(format!(
                "buffer {} truncado ({} < {})",
                buffer.index(),
                bytes.len(),
                buffer.length()
            )));
        }
        out.push(bytes);
    }
    // GLB sem buffers declarados mas com blob: expõe o blob como buffer 0
    // (alguns exports omitem a declaração; o reader pede índice 0).
    if out.is_empty()
        && let Some(b) = blob
    {
        out.push(b.to_vec());
    }
    // JSON sem buffers (só estrutura): retorna vazio — readers darão None.
    let _ = data;
    Ok(out)
}

fn resolve_buffer_uri(
    uri: &str,
    buffer: &gltf::Buffer,
    base_dir: Option<&Path>,
) -> Result<Vec<u8>, GltfImportError> {
    if let Some(b64) = strip_data_uri(uri, None) {
        return decode_b64(&b64).ok_or_else(|| {
            GltfImportError::Validation(format!("buffer {} com data URI inválido", buffer.index()))
        });
    }
    if uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("file://") {
        return Err(GltfImportError::Validation(format!(
            "buffer {} com URI remota (sem rede no import)",
            buffer.index()
        )));
    }
    let Some(base) = base_dir else {
        return Err(GltfImportError::Validation(format!(
            "buffer {} externo ('{uri}') exige diretório base",
            buffer.index()
        )));
    };
    let rel = percent_decode(uri);
    let path = base.join(&rel);
    let bytes =
        std::fs::read(&path).map_err(|e| GltfImportError::Parse(format!("buffer '{uri}': {e}")))?;
    if bytes.len() > MAX_BUFFER_BYTES {
        return Err(GltfImportError::TooLarge);
    }
    Ok(bytes)
}

fn import_materials_list(
    gltf: &gltf::Gltf,
    buffers: &[Vec<u8>],
    base_dir: Option<&Path>,
    warnings: &mut Vec<String>,
) -> Vec<crate::Material> {
    let mut out = Vec::new();
    for (i, m) in gltf.materials().enumerate() {
        if i >= MAX_IMAGES * 4 {
            warnings.push("materiais excedentes ignorados".to_string());
            break;
        }
        out.push(material_from_gltf(&m, buffers, base_dir, warnings));
    }
    out
}

fn material_from_gltf(
    m: &gltf::Material,
    buffers: &[Vec<u8>],
    base_dir: Option<&Path>,
    warnings: &mut Vec<String>,
) -> crate::Material {
    let name = m.name().unwrap_or("Material").to_string();
    let pbr = m.pbr_metallic_roughness();
    let bc = pbr.base_color_factor();
    let mut mat = crate::Material::new(name);
    mat.base_color = sanitize_rgba(bc);
    mat.metallic = pbr.metallic_factor().clamp(0.0, 1.0);
    mat.roughness = pbr.roughness_factor().clamp(0.0, 1.0);
    let emissive = m.emissive_factor();
    let strength = 1.0f32;
    if emissive.iter().any(|x| *x > 1e-5) && strength.is_finite() && strength > 0.0 {
        mat.emission_color = [
            emissive[0].clamp(0.0, 1.0),
            emissive[1].clamp(0.0, 1.0),
            emissive[2].clamp(0.0, 1.0),
        ];
        mat.emission_strength = strength.clamp(0.0, 100.0);
    }
    mat.alpha_mode = match m.alpha_mode() {
        gltf::material::AlphaMode::Opaque => crate::AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => crate::AlphaMode::Mask,
        gltf::material::AlphaMode::Blend => crate::AlphaMode::Blend,
    };
    mat.alpha_cutoff = m.alpha_cutoff().unwrap_or(0.5).clamp(0.0, 1.0);
    if let Some(tx) = m.normal_texture() {
        mat.normal_scale = tx.scale().clamp(0.0, 10.0);
    }

    // Textures (failures -> warnings, slot stays None).
    if let Some(info) = pbr.base_color_texture() {
        match decode_texture_image(&info.texture().source(), buffers, base_dir) {
            Ok(canvas) => mat.albedo_texture = Some(canvas),
            Err(w) => warnings.push(format!("baseColorTexture: {w}")),
        }
    }
    if let Some(info) = pbr.metallic_roughness_texture() {
        match decode_texture_image(&info.texture().source(), buffers, base_dir) {
            Ok(canvas) => {
                mat.roughness_texture = Some(canvas.clone());
                mat.metallic_texture = Some(canvas);
            }
            Err(w) => warnings.push(format!("metallicRoughnessTexture: {w}")),
        }
    }
    if let Some(info) = m.normal_texture() {
        match decode_texture_image(&info.texture().source(), buffers, base_dir) {
            Ok(canvas) => mat.normal_texture = Some(canvas),
            Err(w) => warnings.push(format!("normalTexture: {w}")),
        }
    }
    if let Some(info) = m.emissive_texture() {
        match decode_texture_image(&info.texture().source(), buffers, base_dir) {
            Ok(canvas) => mat.emission_texture = Some(canvas),
            Err(w) => warnings.push(format!("emissiveTexture: {w}")),
        }
    }
    if m.occlusion_texture().is_some() {
        warnings.push("occlusionTexture ignorada (sem canal dedicado)".to_string());
    }
    mat.validate();
    mat
}

fn sanitize_rgba(c: [f32; 4]) -> [f32; 4] {
    let mut o = c;
    for x in &mut o {
        if !x.is_finite() {
            *x = 1.0;
        }
        *x = x.clamp(0.0, 1.0);
    }
    o
}

fn decode_texture_image(
    image: &gltf::Image,
    buffers: &[Vec<u8>],
    base_dir: Option<&Path>,
) -> Result<crate::Canvas, String> {
    let bytes = image_bytes(image, buffers, base_dir)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("imagem excede 64 MiB".to_string());
    }
    let dyn_img = image::load_from_memory(&bytes).map_err(|e| format!("decode: {e}"))?;
    let mut rgba = dyn_img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    if w == 0 || h == 0 {
        return Err("imagem vazia".to_string());
    }
    if w > MAX_CANVAS_DIM || h > MAX_CANVAS_DIM {
        let s = (MAX_CANVAS_DIM as f32 / w.max(h) as f32).min(1.0);
        let nw = ((w as f32 * s) as u32).max(1);
        let nh = ((h as f32 * s) as u32).max(1);
        rgba = image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Triangle);
    }
    let (w, h) = (rgba.width(), rgba.height());
    Ok(crate::Canvas {
        w,
        h,
        pixels: rgba.into_raw().into(),
    })
}

fn image_bytes(
    image: &gltf::Image,
    buffers: &[Vec<u8>],
    base_dir: Option<&Path>,
) -> Result<Vec<u8>, String> {
    match image.source() {
        gltf::image::Source::View { view, .. } => {
            let buf = buffers
                .get(view.buffer().index())
                .ok_or_else(|| "bufferView sem buffer".to_string())?;
            let off = view.offset();
            let len = view.length();
            if off.saturating_add(len) > buf.len() {
                return Err("bufferView de imagem fora dos limites".to_string());
            }
            Ok(buf[off..off + len].to_vec())
        }
        gltf::image::Source::Uri { uri, .. } => {
            if let Some(b64) = strip_data_uri(uri, None) {
                return decode_b64(&b64).ok_or_else(|| "data URI de imagem inválido".to_string());
            }
            if uri.starts_with("http://")
                || uri.starts_with("https://")
                || uri.starts_with("file://")
            {
                return Err("imagem remota (sem rede no import)".to_string());
            }
            let Some(base) = base_dir else {
                return Err(format!("imagem externa ('{uri}') exige diretório base"));
            };
            let rel = percent_decode(uri);
            let path = base.join(&rel);
            std::fs::read(&path).map_err(|e| format!("imagem '{uri}': {e}"))
        }
    }
}

/// Returns the base64 payload of a `data:` URI, or `None` when `uri` is not
/// a data URI. When `expected_prefix` is `Some`, only matching MIME types
/// are accepted.
fn strip_data_uri(uri: &str, expected_prefix: Option<&str>) -> Option<String> {
    if !uri.starts_with("data:") {
        return None;
    }
    let comma = uri.find(',')?;
    let (meta, payload) = (&uri[5..comma], &uri[comma + 1..]);
    if !meta.contains(";base64") {
        return None;
    }
    if let Some(prefix) = expected_prefix
        && !meta.starts_with(prefix)
    {
        return None;
    }
    Some(payload.to_string())
}

fn decode_b64(payload: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    // Data URIs may contain whitespace; strip it before decoding.
    let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(&clean)
        .ok()
        .or_else(|| {
            base64::engine::general_purpose::STANDARD_NO_PAD
                .decode(&clean)
                .ok()
        })
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((h * 16 + l) as char);
            i += 3;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parses and structurally validates a glTF JSON document.
///
/// Rejects documents without an `asset.version`, with an unsupported major
/// version, or with inconsistent mesh/buffer counts. Returns a [`GltfSummary`]
/// for the caller to drive buffer decoding and scene construction.
pub fn parse_gltf_json(data: &[u8]) -> Result<GltfSummary, GltfImportError> {
    let text = std::str::from_utf8(data).map_err(|e| GltfImportError::Parse(e.to_string()))?;
    let root: gltf_json::Root =
        serde_json::from_str(text).map_err(|e| GltfImportError::Parse(e.to_string()))?;
    let mut failures: Vec<String> = Vec::new();
    root.validate(&root, gltf_json::Path::new, &mut |path, error| {
        failures.push(format!("{}: {error:?}", path().as_str()));
    });
    if let Some(first) = failures.into_iter().next() {
        return Err(GltfImportError::Validation(first));
    }

    let version = root.asset.version.clone();
    let major = version
        .split('.')
        .next()
        .and_then(|n| n.parse::<u32>().ok())
        .ok_or_else(|| GltfImportError::Validation(format!("bad asset.version '{version}'")))?;
    if major != 2 {
        return Err(GltfImportError::Validation(format!(
            "unsupported glTF major version {major}"
        )));
    }
    Ok(GltfSummary {
        version,
        scenes: root.scenes.len(),
        nodes: root.nodes.len(),
        meshes: root.meshes.len(),
        buffers: root.buffers.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_GLTF: &str = r#"{
        "asset": { "version": "2.0" },
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
        "buffers": [{ "byteLength": 36 }],
        "bufferViews": [{ "buffer": 0, "byteLength": 36 }],
        "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 1.0] }]
    }"#;

    #[test]
    fn valid_gltf_summarizes_structure() {
        let summary = parse_gltf_json(MINIMAL_GLTF.as_bytes()).unwrap();
        assert_eq!(summary.version, "2.0");
        assert_eq!(summary.scenes, 1);
        assert_eq!(summary.nodes, 1);
        assert_eq!(summary.meshes, 1);
        assert_eq!(summary.buffers, 1);
    }

    #[test]
    fn malformed_json_is_a_parse_error() {
        let err = parse_gltf_json(b"{ not json").unwrap_err();
        assert!(matches!(err, GltfImportError::Parse(_)));
    }

    #[test]
    fn missing_asset_is_rejected() {
        let err = parse_gltf_json(b"{}").unwrap_err();
        assert!(matches!(
            err,
            GltfImportError::Parse(_) | GltfImportError::Validation(_)
        ));
    }

    #[test]
    fn unsupported_major_version_is_rejected() {
        let doc = MINIMAL_GLTF.replace("\"2.0\"", "\"1.0\"");
        assert_eq!(
            parse_gltf_json(doc.as_bytes()).unwrap_err(),
            GltfImportError::Validation("unsupported glTF major version 1".to_string())
        );
    }

    #[test]
    fn glb_roundtrip_preserves_uv_and_color() {
        // Build a Petunia project, export GLB, re-import geometry.
        use petunia_mesh::Mesh;
        let mut project = crate::Project::new();
        project.add("Plane", Mesh::plane(1.0));
        let bytes = crate::export::export_gltf(&project, &[0, 1]).expect("glb");
        let meshes = import_glb_bytes(&bytes, "t", true, 1.0).expect("import");
        assert_eq!(meshes.len(), 2);
        for (_, m) in &meshes {
            assert!(!m.verts.is_empty() && !m.faces.is_empty());
            assert!(m.faces.iter().all(|f| f.uv.len() == f.verts.len()));
        }
    }

    #[test]
    fn glb_with_texture_imports_material_and_canvas() {
        use crate::Canvas;
        use petunia_mesh::Mesh;
        let mut project = crate::Project::new();
        project.add("Plane", Mesh::plane(1.0));
        project.assets[1].texture = Some(Canvas::new(8, 8, [200, 30, 30, 255]));
        let bytes = crate::export::export_gltf(&project, &[1]).expect("glb");
        let scene = import_scene_bytes(&bytes, None, "t", true, 1.0, true).expect("import");
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.materials.len(), 1);
        let mat = &scene.materials[0];
        let canvas = mat.albedo_texture.as_ref().expect("albedo texture");
        assert_eq!((canvas.w, canvas.h), (8, 8));
        // Reddish pixel survives PNG round-trip.
        let px = canvas.get(0, 0).unwrap();
        assert!(px[0] > 150 && px[1] < 80, "{px:?}");
        // Face points at the imported material.
        let (_, mesh) = &scene.meshes[0];
        assert!(mesh.faces.iter().all(|f| f.material_slot == Some(0)));
    }

    #[test]
    fn gltf_json_with_data_uri_buffer_imports() {
        // Minimal triangle: positions (36 B) + u16 indices (6 B) = 42 B.
        let mut bin = Vec::new();
        for p in [[0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for x in p {
                bin.extend_from_slice(&x.to_le_bytes());
            }
        }
        bin.extend_from_slice(&[0u8, 0, 1, 0, 2, 0]);
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bin);
        let doc = format!(
            r#"{{
            "asset": {{"version": "2.0"}},
            "scenes": [{{"nodes": [0]}}],
            "nodes": [{{"mesh": 0}}],
            "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}, "indices": 1}}]}}],
            "buffers": [{{"byteLength": 42, "uri": "data:application/octet-stream;base64,{b64}"}}],
            "bufferViews": [
                {{"buffer": 0, "byteOffset": 0, "byteLength": 36, "target": 34962}},
                {{"buffer": 0, "byteOffset": 36, "byteLength": 6, "target": 34963}}
            ],
            "accessors": [
                {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0,0.0,0.0], "max": [1.0,1.0,0.0]}},
                {{"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}}
            ]
        }}"#
        );
        let scene =
            import_scene_bytes(doc.as_bytes(), None, "tri", true, 1.0, false).expect("import");
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].1.faces.len(), 1);
    }

    #[test]
    fn hostile_mode_is_skipped_not_panic() {
        // POINTS primitive must be skipped with a warning, not panic.
        let doc = r#"{
            "asset": {"version": "2.0"},
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "mode": 0}]}],
            "buffers": [{"byteLength": 12}],
            "bufferViews": [{"buffer": 0, "byteLength": 12}],
            "accessors": [{"bufferView": 0, "componentType": 5126, "count": 1, "type": "VEC3"}]
        }"#;
        let err = import_scene_bytes(doc.as_bytes(), None, "t", true, 1.0, false).unwrap_err();
        assert!(matches!(
            err,
            GltfImportError::Validation(_) | GltfImportError::Parse(_)
        ));
    }
}
