//! Exportação com validação: OBJ (texto) e GLB (binário glTF 2.0).
//! GLB escrito à mão (sem dependência pesada); validado em teste com `gltf`.

use super::{Asset, Canvas, Project};
use crate::gltf_rig::{
    BinBuilder, COMPONENT_U16, RigDoc, SkinBinding, TARGET_ARRAY_BUFFER, bind_skin, build_rig_doc,
};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("I/O: {0}")]
    Io(String),
    #[error("nada para exportar")]
    Empty,
    #[error("{0}")]
    Other(String),
}

pub fn export_obj(asset: &Asset) -> String {
    asset.evaluated_mesh().to_obj()
}

/// Export-only dilation: the editable document and paint layers stay intact.
pub fn export_gltf_with_padding(
    project: &Project,
    indices: &[usize],
    padding: u32,
) -> Result<Vec<u8>, ExportError> {
    if padding == 0 {
        return export_gltf(project, indices);
    }
    let mut snapshot = project.clone();
    for &i in indices {
        let Some(asset) = snapshot.assets.get_mut(i) else {
            continue;
        };
        let mesh = asset.evaluated_mesh_ref();
        let padded = asset
            .texture
            .as_ref()
            .map(|texture| bleed_outside_uv(texture, padding, std::iter::once(mesh.as_ref())));
        drop(mesh);
        if let Some(padded) = padded {
            asset.texture = Some(padded);
        }
    }
    for material in &mut snapshot.materials {
        let meshes: Vec<_> = indices
            .iter()
            .filter_map(|&i| project.assets.get(i))
            .filter(|a| a.material_id == Some(material.id))
            .map(|a| a.evaluated_mesh_ref())
            .collect();
        if meshes.is_empty() {
            continue;
        }
        for texture in [
            &mut material.albedo_texture,
            &mut material.normal_texture,
            &mut material.roughness_texture,
            &mut material.metallic_texture,
            &mut material.emission_texture,
            &mut material.height_texture,
        ]
        .into_iter()
        .flatten()
        {
            *texture = bleed_outside_uv(texture, padding, meshes.iter().map(|m| m.as_ref()));
        }
    }
    export_gltf(&snapshot, indices)
}

/// Dilation is confined to atlas gutters, preserving transparent artwork inside
/// mapped faces (including SVG cutouts). Shared material masks use all consumers.
fn bleed_outside_uv<'a>(
    texture: &Canvas,
    padding: u32,
    meshes: impl Iterator<Item = &'a petunia_mesh::Mesh>,
) -> Canvas {
    let mut out = texture.with_bleed(padding);
    for mesh in meshes {
        for face in 0..mesh.faces.len() {
            mesh.rasterize_face_texels(face, texture.w, texture.h, 0.0, |x, y, _, _| {
                let index = ((y * texture.w + x) * 4) as usize;
                out.pixels[index..index + 4].copy_from_slice(&texture.pixels[index..index + 4]);
            });
        }
    }
    out
}

#[cfg(test)]
mod bleed_tests {
    use super::*;
    #[test]
    fn padding_preserves_transparent_artwork_inside_uv_faces() {
        let mut mesh = petunia_mesh::Mesh::plane(2.0);
        mesh.faces[0].uv = vec![[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
        let mut texture = Canvas::new(16, 16, [0; 4]);
        texture.set(4, 8, [255, 0, 0, 255]);
        let out = bleed_outside_uv(&texture, 2, std::iter::once(&mesh));
        assert_eq!(out.get(5, 8), Some([0; 4]));
        assert_eq!(out.get(3, 8), Some([255, 0, 0, 255]));
        assert_eq!(texture.get(3, 8), Some([0; 4]));
    }
}

/// Decal Sets do asset (decalques com variantes ou trilha) como JSON, para
/// `extras.petunia` do material: a textura exportada é a composição parada
/// ("bake universal"); a metadata permite reproduzir a troca na engine.
fn decal_sets_json(asset: &Asset) -> Option<String> {
    use crate::paint_layers::LayerKind;
    let sets: Vec<serde_json::Value> = asset
        .paint_stack
        .as_ref()?
        .layers
        .iter()
        .filter_map(|layer| match &layer.kind {
            LayerKind::Decal(decal)
                if decal.variants.len() > 1
                    || decal.track.as_ref().is_some_and(|t| !t.keys.is_empty()) =>
            {
                let names: Vec<&str> = (0..decal.variant_count())
                    .map(|index| decal.variant_name(index, &layer.name))
                    .collect();
                let track = decal.track.as_ref().map(|track| {
                    serde_json::json!({
                        "fps": track.fps,
                        "length_frames": track.length_frames,
                        "looping": track.looping,
                        "keys": track.keys.iter().map(|k| [k.frame, k.variant]).collect::<Vec<_>>(),
                    })
                });
                Some(serde_json::json!({
                    "name": layer.name,
                    "variants": names,
                    "variant_index": decal.variant_index,
                    "interpolation": "STEP",
                    "track": track,
                }))
            }
            _ => None,
        })
        .collect();
    (!sets.is_empty()).then(|| serde_json::Value::Array(sets).to_string())
}

/// Exporta assets como um único GLB (uma mesh por asset).
pub fn export_gltf(project: &Project, indices: &[usize]) -> Result<Vec<u8>, ExportError> {
    let picked: Vec<&Asset> = indices
        .iter()
        .filter_map(|&i| project.assets.get(i))
        .collect();
    if picked.is_empty() {
        return Err(ExportError::Empty);
    }

    // achata: por asset, tris (pos, nrm, uv) + índices u32 + PNG do Albedo
    struct Part {
        name: String,
        color: [f32; 4],
        roughness: f32,
        metallic: f32,
        emissive: [f32; 3],
        pos: Vec<f32>,
        nrm: Vec<f32>,
        uv: Vec<f32>,
        idx: Vec<u32>,
        texture_png: Option<Vec<u8>>,
        /// Decal Sets do asset (cap. 39) como JSON para `extras.petunia`.
        decal_sets: Option<String>,
        skin: Option<SkinBinding>,
        joints: Vec<u16>,
        weights: Vec<f32>,
    }
    let mut parts = Vec::new();
    for a in picked {
        let mut m = a.evaluated_mesh();
        m.triangulate();
        // M6: valida em vez de panicar (malha pode vir de arquivo hostil)
        if !m
            .verts
            .iter()
            .all(|v| v.pos.iter().all(|x| x.is_finite()) && v.color.iter().all(|x| x.is_finite()))
            || !a.base_color.iter().all(|x| x.is_finite())
        {
            return Err(ExportError::Other(format!(
                "malha '{}' com NaN/inf",
                a.name
            )));
        }
        for f in &m.faces {
            if f.verts.len() != 3
                || f.uv.len() != 3
                || f.verts.iter().any(|&vi| (vi as usize) >= m.verts.len())
                || f.uv.iter().any(|uv| !uv.iter().all(|x| x.is_finite()))
            {
                return Err(ExportError::Other(format!(
                    "malha '{}' inválida para export",
                    a.name
                )));
            }
        }

        let (roughness, metallic, emissive, base_color_rgba) =
            if let Some(mat) = a.material(project) {
                (
                    mat.roughness.clamp(0.0, 1.0),
                    mat.metallic.clamp(0.0, 1.0),
                    [
                        mat.emission_color[0] * mat.emission_strength,
                        mat.emission_color[1] * mat.emission_strength,
                        mat.emission_color[2] * mat.emission_strength,
                    ],
                    mat.base_color,
                )
            } else {
                (
                    0.9,
                    0.0,
                    [0.0, 0.0, 0.0],
                    [a.base_color[0], a.base_color[1], a.base_color[2], 1.0],
                )
            };

        let normals = m.compute_normals();
        // Textura pintada (Albedo): asset.texture ou canal do material.
        let texture_png = a
            .texture
            .as_ref()
            .or_else(|| {
                a.material(project)
                    .and_then(|mat| mat.albedo_texture.as_ref())
            })
            .and_then(encode_png);
        let mut p = Part {
            name: a.name.clone(),
            color: base_color_rgba,
            roughness,
            metallic,
            emissive,
            pos: Vec::new(),
            nrm: Vec::new(),
            uv: Vec::new(),
            idx: Vec::new(),
            texture_png,
            decal_sets: decal_sets_json(a),
            skin: bind_skin(project, a, m.verts.len()),
            joints: Vec::new(),
            weights: Vec::new(),
        };
        let mut lut: std::collections::HashMap<(u32, u32), u32> = Default::default();
        for f in &m.faces {
            if f.verts.len() != 3 {
                continue;
            }
            for k in 0..3 {
                let vi = f.verts[k];
                let key = (
                    vi,
                    f.uv[k][0].to_bits() ^ f.uv[k][1].to_bits().rotate_left(1),
                );
                let id = *lut.entry(key).or_insert_with(|| {
                    let id = (p.pos.len() / 3) as u32;
                    let vv = &m.verts[vi as usize];
                    p.pos.extend_from_slice(&vv.pos);
                    p.nrm.extend_from_slice(&normals[vi as usize]);
                    p.uv.extend_from_slice(&[f.uv[k][0], f.uv[k][1]]);
                    if let Some(skin) = &p.skin {
                        p.joints.extend_from_slice(&skin.joints[vi as usize]);
                        p.weights.extend_from_slice(&skin.weights[vi as usize]);
                    }
                    id
                });
                p.idx.push(id);
            }
        }
        if p.idx.is_empty() {
            continue;
        }
        parts.push(p);
    }
    if parts.is_empty() {
        return Err(ExportError::Empty);
    }

    // monta BIN
    let mut bb = BinBuilder::default();
    /// Accessors de cada parte (JOINTS_0/WEIGHTS_0 só em partes com skin).
    struct MeshAcc {
        pos: usize,
        nrm: usize,
        uv: usize,
        idx: usize,
        skin_attrs: Option<(usize, usize)>,
    }
    let mut mesh_acc: Vec<MeshAcc> = Vec::new();
    for p in &parts {
        let mut a = [0usize; 3];
        for (k, data) in [&p.pos, &p.nrm, &p.uv].iter().enumerate() {
            let comps = if k == 2 { 2 } else { 3 };
            let mm = if k == 0 {
                let mut mn = vec![f32::MAX; 3];
                let mut mx = vec![f32::MIN; 3];
                for v in data.chunks(3) {
                    for c in 0..3 {
                        if let Some(&x) = v.get(c) {
                            mn[c] = mn[c].min(x);
                            mx[c] = mx[c].max(x);
                        }
                    }
                }
                Some((mn, mx))
            } else {
                None
            };
            a[k] = bb.push_f32(
                data,
                if k == 2 { "VEC2" } else { "VEC3" },
                comps,
                TARGET_ARRAY_BUFFER,
                mm,
            );
        }
        let idx_bytes: Vec<u8> = p.idx.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = bb.push_view(&idx_bytes, 34963);
        let idx = bb.push_acc(view, p.idx.len(), "SCALAR", 5125, None);
        let skin_attrs = if p.skin.is_some() {
            let jv = bb.push_view(bytemuck::cast_slice(&p.joints), TARGET_ARRAY_BUFFER);
            let j = bb.push_acc(jv, p.joints.len() / 4, "VEC4", COMPONENT_U16, None);
            let w = bb.push_f32(&p.weights, "VEC4", 4, TARGET_ARRAY_BUFFER, None);
            Some((j, w))
        } else {
            None
        };
        mesh_acc.push(MeshAcc {
            pos: a[0],
            nrm: a[1],
            uv: a[2],
            idx,
            skin_attrs,
        });
    }

    // Skins, joints e animações (uma skin por esqueleto usado).
    let mut skeleton_ids: Vec<uuid::Uuid> = Vec::new();
    for p in &parts {
        if let Some(skin) = &p.skin
            && !skeleton_ids.contains(&skin.skeleton_id)
        {
            skeleton_ids.push(skin.skeleton_id);
        }
    }
    let rig = if skeleton_ids.is_empty() {
        RigDoc::default()
    } else {
        build_rig_doc(project, &skeleton_ids, parts.len(), &mut bb)?
    };

    // monta JSON
    let mut j = String::from("{\"asset\":{\"version\":\"2.0\",\"generator\":\"Petunia3D\"},");
    j.push_str("\"scene\":0,\"scenes\":[{\"nodes\":[");
    let scene_nodes: Vec<usize> = (0..parts.len())
        .chain(rig.scene_roots.iter().copied())
        .collect();
    for (i, n) in scene_nodes.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        j.push_str(&n.to_string());
    }
    j.push_str("]}],\"nodes\":[");
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        let skin_field = p
            .skin
            .as_ref()
            .and_then(|s| rig.skin_of.get(&s.skeleton_id))
            .map(|k| format!(",\"skin\":{k}"))
            .unwrap_or_default();
        j.push_str(&format!(
            "{{\"name\":{},\"mesh\":{i}{skin_field}}}",
            json_str(&p.name)
        ));
    }
    for node in &rig.nodes {
        j.push(',');
        j.push_str(&node.to_string());
    }
    j.push_str("],\"meshes\":[");
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        let a = &mesh_acc[i];
        let skin_attrs = a
            .skin_attrs
            .map(|(jn, w)| format!(",\"JOINTS_0\":{jn},\"WEIGHTS_0\":{w}"))
            .unwrap_or_default();
        j.push_str(&format!(
            "{{\"name\":{},\"primitives\":[{{\"attributes\":{{\"POSITION\":{},\"NORMAL\":{},\"TEXCOORD_0\":{}{skin_attrs}}},\"indices\":{},\"material\":{i}}}]}}",
            json_str(&p.name), a.pos, a.nrm, a.uv, a.idx
        ));
    }
    j.push_str("],\"materials\":[");
    // Imagens PNG embutidas (uma por parte com textura pintada).
    let mut part_image: Vec<Option<usize>> = Vec::with_capacity(parts.len());
    let mut image_views: Vec<usize> = Vec::new();
    for p in &parts {
        if let Some(png) = &p.texture_png {
            let view_idx = bb.push_view(png, 0);
            image_views.push(view_idx);
            part_image.push(Some(image_views.len() - 1));
        } else {
            part_image.push(None);
        }
    }
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        let mut pbr = format!(
            "\"baseColorFactor\":[{:.4},{:.4},{:.4},{:.4}],\"metallicFactor\":{:.4},\"roughnessFactor\":{:.4}",
            p.color[0], p.color[1], p.color[2], p.color[3], p.metallic, p.roughness,
        );
        if let Some(img_idx) = part_image[i] {
            pbr.push_str(&format!(",\"baseColorTexture\":{{\"index\":{img_idx}}}"));
        }
        let extras = p
            .decal_sets
            .as_ref()
            .map(|sets| format!(",\"extras\":{{\"petunia\":{{\"decal_sets\":{sets}}}}}"))
            .unwrap_or_default();
        j.push_str(&format!(
            "{{\"name\":{},\"doubleSided\":true,\"pbrMetallicRoughness\":{{{pbr}}},\"emissiveFactor\":[{:.4},{:.4},{:.4}]{extras}}}",
            json_str(&format!("{}_mat", p.name)),
            p.emissive[0], p.emissive[1], p.emissive[2]
        ));
    }
    if !image_views.is_empty() {
        j.push_str("],\"images\":[");
        for (i, view_idx) in image_views.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&format!(
                "{{\"name\":\"petunia_tex_{i}\",\"bufferView\":{view_idx},\"mimeType\":\"image/png\"}}"
            ));
        }
        j.push_str("],\"textures\":[");
        for (i, _) in image_views.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&format!("{{\"source\":{i}}}"));
        }
    }
    j.push(']');
    if !rig.skins.is_empty() {
        j.push_str(",\"skins\":");
        j.push_str(&serde_json::Value::Array(rig.skins.clone()).to_string());
    }
    if !rig.animations.is_empty() {
        j.push_str(",\"animations\":");
        j.push_str(&serde_json::Value::Array(rig.animations.clone()).to_string());
    }
    j.push_str(",\"accessors\":[");
    for (i, (v, count, ty, comp, mm)) in bb.accs.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        j.push_str(&format!(
            "{{\"bufferView\":{v},\"componentType\":{comp},\"count\":{count},\"type\":\"{ty}\""
        ));
        if let Some((mn, mx)) = mm {
            let fmt = |v: &Vec<f32>| {
                v.iter()
                    .map(|x| format!("{x:.6}"))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            j.push_str(&format!(",\"min\":[{}],\"max\":[{}]", fmt(mn), fmt(mx)));
        }
        j.push('}');
    }
    j.push_str("],\"bufferViews\":[");
    for (i, (off, len, target)) in bb.views.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        // Views sem target (imagens, IBM, animação): glTF só marca vertex/index.
        if *target == 0 {
            j.push_str(&format!(
                "{{\"buffer\":0,\"byteOffset\":{off},\"byteLength\":{len}}}"
            ));
        } else {
            j.push_str(&format!(
                "{{\"buffer\":0,\"byteOffset\":{off},\"byteLength\":{len},\"target\":{target}}}"
            ));
        }
    }
    bb.align4();
    j.push_str(&format!(
        "],\"buffers\":[{{\"byteLength\":{}}}]}}",
        bb.bin.len()
    ));
    let bin = std::mem::take(&mut bb.bin);

    // chunk JSON (pad espaço)
    let mut json = j.into_bytes();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    // GLB
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&0x46546C67u32.to_le_bytes()); // 'glTF'
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // 'JSON'
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E4942u32.to_le_bytes()); // 'BIN\0'
    out.extend_from_slice(&bin);
    Ok(out)
}

/// Codifica um canvas RGBA8 em PNG (textura pintada no GLB).
/// `None` em canvas vazio ou falha de encoder (export segue sem textura).
fn encode_png(canvas: &super::Canvas) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    if canvas.w == 0 || canvas.h == 0 || canvas.pixels.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut out);
    enc.write_image(
        &canvas.pixels,
        canvas.w,
        canvas.h,
        image::ExtendedColorType::Rgba8,
    )
    .ok()?;
    Some(out)
}

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                o.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Validação legível antes de salvar: escolhe pasta e escreve um arquivo por asset (OBJ) ou um GLB.
pub fn export_report(project: &Project, indices: &[usize], include_gltf: bool) -> Vec<String> {
    let mut lines = Vec::new();
    if indices.is_empty() {
        lines.push("nada selecionado".to_string());
        return lines;
    }
    for &i in indices {
        match project.assets.get(i) {
            Some(a) => lines.push(format!(
                "{}: {} verts, {} tris{}",
                a.name,
                a.mesh.vert_count(),
                a.mesh.tri_count(),
                if include_gltf { ", GLB ok" } else { ", OBJ ok" }
            )),
            None => lines.push(format!("índice {i} inválido")),
        }
        if include_gltf && let Some(a) = project.assets.get(i) {
            if let Some(note) = crate::gltf_rig::skin_export_note(project, a) {
                lines.push(format!("aviso: {note}"));
            }
            if let Some(sid) = a.skeleton_id {
                for note in crate::gltf_rig::motion_export_notes(project, sid) {
                    lines.push(format!("aviso: {note}"));
                }
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    fn sample_project() -> Project {
        let mut p = Project::new();
        p.add("Plane", Mesh::plane(1.0));
        p
    }

    #[test]
    fn obj_has_vt() {
        let p = sample_project();
        let s = export_obj(&p.assets[0]);
        assert!(s.contains("v ") && s.contains("vt ") && s.contains('f'));
    }

    #[test]
    fn glb_roundtrip_via_gltf_crate() {
        let p = sample_project();
        let bytes = export_gltf(&p, &[0, 1]).expect("glb");
        assert_eq!(&bytes[0..4], b"glTF");
        let (doc, buffers, _) = gltf::import_slice(&bytes).expect("parse glb");
        assert_eq!(doc.meshes().len(), 2);
        assert_eq!(buffers.len(), 1);
        let total_verts: usize = doc.meshes().flat_map(|m| m.primitives()).map(|_| 0).sum();
        let _ = total_verts;
        // posições do cubo: 8+ verts únicos (cubo tem 8 verts, plano 4)
        let mut n = 0;
        for mesh in doc.meshes() {
            for prim in mesh.primitives() {
                let r = prim.reader(|b| Some(&buffers[b.index()]));
                n += r.read_positions().map(|it| it.len()).unwrap_or(0);
                assert!(r.read_normals().is_some());
                assert!(r.read_tex_coords(0).is_some());
                assert!(r.read_indices().is_some());
            }
        }
        assert!(n >= 12, "verts insuficientes: {n}");
    }

    fn rigged_project() -> (Project, crate::rig::Skeleton) {
        use crate::{AnimationAsset, AnimationLibrary, RigPreset, compute_auto_skin_weights};
        let mut p = sample_project();
        let skel = RigPreset::humanoid(1.0);
        let mesh = p.assets[0].mesh.clone();
        p.assets[0].skin_data = Some(compute_auto_skin_weights(&mesh, &skel));
        p.assets[0].skeleton_id = Some(skel.id);
        p.animations.push(AnimationAsset::new(
            "Walk",
            AnimationLibrary::humanoid_walk(&skel),
        ));
        p.animations.push(AnimationAsset::new(
            "Idle",
            AnimationLibrary::humanoid_idle(&skel),
        ));
        p.add_skeleton(skel.clone());
        (p, skel)
    }

    fn node_world(doc: &gltf::Document) -> Vec<glam::Mat4> {
        fn walk(node: gltf::Node, parent: glam::Mat4, out: &mut Vec<glam::Mat4>) {
            let local = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
            let world = parent * local;
            out[node.index()] = world;
            for c in node.children() {
                walk(c, world, out);
            }
        }
        let mut out = vec![glam::Mat4::IDENTITY; doc.nodes().len()];
        for root in doc.default_scene().unwrap().nodes() {
            walk(root, glam::Mat4::IDENTITY, &mut out);
        }
        out
    }

    #[test]
    fn glb_exports_skin_joints_weights_and_animations() {
        let (p, skel) = rigged_project();
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, buffers, _) = gltf::import_slice(&bytes).expect("parse glb");
        let get = |b: gltf::Buffer| Some(&buffers[b.index()][..]);

        // Skin: um por esqueleto, um joint por osso, IBM idêntica à do Rig Core.
        assert_eq!(doc.skins().count(), 1);
        let skin = doc.skins().next().unwrap();
        assert_eq!(skin.joints().count(), skel.bones.len());
        let ibm: Vec<[[f32; 4]; 4]> = skin
            .reader(get)
            .read_inverse_bind_matrices()
            .expect("IBM")
            .collect();
        assert_eq!(ibm.len(), skel.bones.len());
        for (m, bone) in ibm.iter().zip(&skel.bones) {
            let expected = glam::Mat4::from_cols_array(&bone.inverse_bind_matrix);
            assert!(glam::Mat4::from_cols_array_2d(m).abs_diff_eq(expected, 1e-5));
        }

        // Nó da malha referencia o skin; pesos somam 1 e joints estão no intervalo.
        let mesh_node = doc.nodes().find(|n| n.mesh().is_some()).unwrap();
        assert!(mesh_node.skin().is_some());
        let prim = mesh_node.mesh().unwrap().primitives().next().unwrap();
        let r = prim.reader(get);
        let joints: Vec<[u16; 4]> = r.read_joints(0).expect("JOINTS_0").into_u16().collect();
        let weights: Vec<[f32; 4]> = r.read_weights(0).expect("WEIGHTS_0").into_f32().collect();
        assert_eq!(joints.len(), weights.len());
        assert!(!joints.is_empty());
        for (j, w) in joints.iter().zip(&weights) {
            assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-4);
            assert!(j.iter().all(|&x| (x as usize) < skel.bones.len()));
        }

        // Em repouso a matriz de cada joint (mundo × IBM) é identidade.
        let world = node_world(&doc);
        for (joint, m) in skin.joints().zip(&ibm) {
            let skin_m = world[joint.index()] * glam::Mat4::from_cols_array_2d(m);
            assert!(skin_m.abs_diff_eq(glam::Mat4::IDENTITY, 1e-4), "{skin_m:?}");
        }

        // Animações: Walk (2 canais de rotação) e Idle (translação + escala).
        assert_eq!(doc.animations().count(), 2);
        let walk = doc.animations().find(|a| a.name() == Some("Walk")).unwrap();
        assert_eq!(walk.channels().count(), 2);
        for ch in walk.channels() {
            assert_eq!(ch.target().property(), gltf::animation::Property::Rotation);
            let cr = ch.reader(get);
            let times: Vec<f32> = cr.read_inputs().unwrap().collect();
            assert_eq!(times, vec![0.0, 0.5, 1.0]);
        }
        let idle = doc.animations().find(|a| a.name() == Some("Idle")).unwrap();
        let props: Vec<_> = idle.channels().map(|c| c.target().property()).collect();
        assert!(props.contains(&gltf::animation::Property::Translation));
        assert!(props.contains(&gltf::animation::Property::Scale));
    }

    #[test]
    fn glb_carries_decal_sets_in_material_extras() {
        use crate::paint_layers::{
            DecalLayer, DecalVariant, DecalVariantTrack, PaintLayer, PaintLayerStack,
        };
        let mut p = sample_project();
        let mut decal = DecalLayer::new(Canvas::new(4, 4, [255; 4]), [0.5; 2], [0.3; 2], 0.0);
        decal.add_variant(
            "Neutra",
            DecalVariant {
                name: "Sorriso".into(),
                image: Canvas::new(4, 4, [0, 255, 0, 255]),
                source_svg: None,
            },
        );
        let mut track = DecalVariantTrack::default();
        track.set_key(0, 0);
        track.set_key(4, 1);
        decal.track = Some(track);
        let mut stack = PaintLayerStack::with_base("Base", Canvas::new(8, 8, [0, 0, 0, 255]));
        stack.add_layer(PaintLayer::new_decal("Boca", decal));
        p.assets[0].paint_stack = Some(stack);
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, _, _) = gltf::import_slice(&bytes).expect("parse glb");
        let extras = doc
            .materials()
            .next()
            .unwrap()
            .extras()
            .clone()
            .expect("extras");
        let extras: serde_json::Value = serde_json::from_str(extras.get()).unwrap();
        let set = &extras["petunia"]["decal_sets"][0];
        assert_eq!(set["name"], "Boca");
        assert_eq!(set["variants"], serde_json::json!(["Neutra", "Sorriso"]));
        assert_eq!(set["interpolation"], "STEP");
        assert_eq!(set["track"]["keys"], serde_json::json!([[0, 0], [4, 1]]));
        // Sem Decal Set, o material não ganha extras.
        p.assets[0].paint_stack = None;
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, _, _) = gltf::import_slice(&bytes).expect("parse glb");
        assert!(doc.materials().next().unwrap().extras().is_none());
    }

    #[test]
    fn glb_without_rig_keeps_no_skin_or_animation() {
        let p = sample_project();
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, _, _) = gltf::import_slice(&bytes).expect("parse glb");
        assert_eq!(doc.skins().count(), 0);
        assert_eq!(doc.animations().count(), 0);
    }

    #[test]
    fn glb_skin_is_skipped_with_a_note_when_weights_do_not_match_the_mesh() {
        let (mut p, _) = rigged_project();
        p.assets[0].skin_data.as_mut().unwrap().vertex_weights.pop();
        let note = crate::gltf_rig::skin_export_note(&p, &p.assets[0]).expect("nota");
        assert!(note.contains("sem skin"), "{note}");
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, _, _) = gltf::import_slice(&bytes).expect("parse glb");
        assert_eq!(doc.skins().count(), 0);
        assert_eq!(doc.animations().count(), 0);
    }

    #[test]
    fn glb_rejects_non_monotonic_keyframe_times() {
        let (mut p, _) = rigged_project();
        let track = &mut p.animations[0].clip.tracks[0];
        track.rotations[1].time = track.rotations[0].time; // empata os tempos
        let err = export_gltf(&p, &[0]).unwrap_err();
        assert!(err.to_string().contains("não crescentes"), "{err}");
    }

    #[test]
    fn glb_embeds_painted_texture_as_png() {
        use crate::Canvas;
        let mut p = sample_project();
        // Textura pintada no asset 0.
        p.assets[0].texture = Some(Canvas::new(8, 8, [200, 30, 30, 255]));
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, buffers, _) = gltf::import_slice(&bytes).expect("parse glb");
        assert_eq!(doc.images().len(), 1);
        assert_eq!(doc.textures().len(), 1);
        let img = doc.images().next().unwrap();
        let (view, mime) = match img.source() {
            gltf::image::Source::View { view, mime_type } => (view, mime_type),
            gltf::image::Source::Uri { .. } => panic!("esperava imagem embutida"),
        };
        assert_eq!(mime, "image/png");
        let buf = &buffers[view.buffer().index()];
        let png = &buf[view.offset()..view.offset() + view.length()];
        assert_eq!(&png[1..4], b"PNG");
        let decoded = image::load_from_memory(png).expect("png válido");
        let px = decoded.to_rgba8();
        assert_eq!(px.get_pixel(0, 0).0, [200, 30, 30, 255]);
        // Material referencia a textura.
        let mat = doc.materials().next().expect("material exists");
        assert!(mat.pbr_metallic_roughness().base_color_texture().is_some());
    }

    #[test]
    fn glb_without_texture_has_no_images() {
        let p = sample_project();
        let bytes = export_gltf(&p, &[0]).expect("glb");
        let (doc, _, _) = gltf::import_slice(&bytes).expect("parse glb");
        assert_eq!(doc.images().len(), 0);
        assert_eq!(doc.textures().len(), 0);
        let mat = doc.materials().next().expect("material exists");
        assert!(mat.pbr_metallic_roughness().base_color_texture().is_none());
    }

    #[test]
    fn glb_empty_errors() {
        let p = Project::default();
        assert!(export_gltf(&p, &[]).is_err());
    }

    #[test]
    fn glb_nan_mesh_errors_not_panics() {
        // M5/M6: NaN vira Err legível, nunca panic nem JSON inválido
        let mut p = Project::new();
        p.assets[0].mesh.verts[0].pos = [f32::NAN, 0.0, 0.0];
        assert!(export_gltf(&p, &[0]).is_err());
    }

    #[test]
    fn json_str_escapes_controls() {
        let s = json_str("a\"b\\c\nd\te\x01f");
        assert_eq!(s, "\"a\\\"b\\\\c\\nd\\u0009e\\u0001f\"");
        let v: serde_json::Value = serde_json::from_str(&s).expect("json válido");
        assert_eq!(v.as_str().unwrap().chars().count(), 11);
    }
}
