//! Revisão de cena para caches de GPU (Wave 1 — P0-A/P0-B).
//!
//! Invariante: movimento de câmera **nunca** altera o fingerprint. Somente
//! mudanças em geometria, seleção relevante para render, materiais, visibilidade
//! ou flags de sombreamento invalidam os buffers. Backends WGPU e OpenGL usam a
//! mesma função para decidir se reconstróem buffers ou apenas atualizam uniforms.
//!
//! O hash percorre metadados + conteúdo posicional (O(verts+faces) no pior caso),
//! muito mais barato que triangulação + alocação de `Vec` + criação de buffer GPU
//! por frame. Não há estado global: chamadores guardam o último fingerprint.

use petunia_project::Project;

use super::state::{ReferenceImage, Shading};

/// Fingerprint separado para geometria vs layout de referências.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneFingerprint {
    /// Geometria + materiais + seleção + flags de sombreamento.
    pub mesh: u64,
    /// Conteúdo/revisão das texturas, separado dos buffers de geometria.
    pub textures: u64,
    /// Layout/transform de quads de referência (não pixels; pixels têm hash próprio).
    pub refs_layout: u64,
}

fn mix(mut h: u64, v: u64) -> u64 {
    h ^= v
        .wrapping_add(0x9e3779b97f4a7c15)
        .wrapping_add(h << 6)
        .wrapping_add(h >> 2);
    h
}

fn hash_bytes(h: u64, bytes: &[u8]) -> u64 {
    let mut acc = h;
    for &b in bytes {
        acc = acc.wrapping_mul(0x100000001b3) ^ (b as u64);
    }
    acc
}

fn hash_f32(h: u64, v: f32) -> u64 {
    mix(h, v.to_bits() as u64)
}

/// Flags de render que afetam buffers GPU (tudo que não é câmera).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FingerprintFlags {
    pub shading: Shading,
    pub xray: bool,
    pub show_triangulation: bool,
    pub textured: bool,
    pub edit_mode_is_edit: bool,
    pub show_wireframe_overlay: bool,
    pub show_face_orientation: bool,
    pub show_uv_checker: bool,
}

/// Fingerprint barato da cena para invalidação de buffers GPU.
///
/// Primary path: mix **revision counters** (O(assets)), not vertex/texture
/// bytes. When a mesh has never been revisioned (legacy files, tests that
/// poke fields directly) we fall back to a content hash of that asset only.
pub fn fingerprint_scene(
    project: &Project,
    refs: &[ReferenceImage],
    flags: FingerprintFlags,
) -> SceneFingerprint {
    let mut h: u64 = 0xcbf29ce484222325;
    h = mix(h, flags.shading as u64);
    h = mix(h, flags.xray as u64);
    h = mix(h, flags.show_triangulation as u64);
    h = mix(h, flags.textured as u64);
    h = mix(h, flags.edit_mode_is_edit as u64);
    h = mix(h, flags.show_wireframe_overlay as u64);
    h = mix(h, flags.show_face_orientation as u64);
    h = mix(h, flags.show_uv_checker as u64);
    h = mix(h, project.assets.len() as u64);
    h = mix(h, project.materials.len() as u64);
    h = mix(h, project.topology_revision);
    h = mix(h, project.position_revision);
    h = mix(h, project.normal_revision);
    h = mix(h, project.selection_revision);
    h = mix(h, project.uv_revision);
    h = mix(h, project.color_revision);
    h = mix(h, project.material_revision);
    h = mix(h, project.transform_revision);

    for mat in &project.materials {
        h = hash_bytes(h, mat.id.as_bytes());
        for c in mat.base_color {
            h = hash_f32(h, c);
        }
        h = mix(h, mat.profile as u64);
        h = hash_f32(h, mat.emission_strength);
        if let Some(tex) = mat.albedo_texture.as_ref() {
            h = mix(h, tex.w as u64);
            h = mix(h, tex.h as u64);
            h = mix(h, tex.pixels.len() as u64);
        }
    }

    for asset in &project.assets {
        h = hash_bytes(h, asset.id.as_bytes());
        h = mix(h, asset.visible as u64);
        h = mix(h, asset.mesh.verts.len() as u64);
        h = mix(h, asset.mesh.faces.len() as u64);
        h = mix(h, asset.modifiers.len() as u64);
        for modifier in &asset.modifiers {
            h = hash_bytes(h, modifier.id.as_bytes());
            h = mix(h, modifier.enabled as u64);
            match modifier.kind {
                petunia_project::ModifierKind::Mirror { axis, weld } => {
                    h = mix(h, 1);
                    h = mix(h, axis as u64);
                    h = hash_f32(h, weld);
                }
                petunia_project::ModifierKind::Symmetry {
                    axis,
                    positive_to_negative,
                    weld,
                } => {
                    h = mix(h, 2);
                    h = mix(h, axis as u64);
                    h = mix(h, positive_to_negative as u64);
                    h = hash_f32(h, weld);
                }
            }
        }
        for c in asset.base_color {
            h = hash_f32(h, c);
        }
        h = mix(
            h,
            asset
                .material_id
                .map(|id| {
                    let mut x: u64 = 0;
                    for &b in id.as_bytes() {
                        x = x.wrapping_mul(31).wrapping_add(b as u64);
                    }
                    x
                })
                .unwrap_or(0),
        );
        let unrevisioned = project.topology_revision == 0
            && project.position_revision == 0
            && project.normal_revision == 0
            && project.selection_revision == 0
            && project.uv_revision == 0
            && project.color_revision == 0
            && project.material_revision == 0;
        if unrevisioned {
            for v in &asset.mesh.verts {
                for c in v.pos {
                    h = hash_f32(h, c);
                }
                h = mix(h, v.selected as u64);
                for c in v.color {
                    h = hash_f32(h, c);
                }
            }
            for f in &asset.mesh.faces {
                h = mix(h, f.verts.len() as u64);
                for &i in &f.verts {
                    h = mix(h, i as u64);
                }
                for uv in &f.uv {
                    h = hash_f32(h, uv[0]);
                    h = hash_f32(h, uv[1]);
                }
                h = mix(h, f.selected as u64);
                h = mix(h, f.material_slot.unwrap_or(usize::MAX) as u64);
            }
            let mut selected_edges: Vec<_> = asset.mesh.selected_edges.iter().copied().collect();
            selected_edges.sort_unstable();
            for (a, b) in selected_edges {
                h = mix(h, a as u64);
                h = mix(h, b as u64);
            }
            let mut seams: Vec<_> = asset.mesh.uv_seams.iter().copied().collect();
            seams.sort_unstable();
            for (a, b) in seams {
                h = mix(h, a as u64);
                h = mix(h, b as u64);
            }
            let mut pins: Vec<_> = asset.mesh.uv_pinned.iter().copied().collect();
            pins.sort_unstable();
            for (face, corner) in pins {
                h = mix(h, face as u64);
                h = mix(h, corner as u64);
            }
        }
        if let Some(canvas) = asset.texture.as_ref() {
            h = mix(h, canvas.w as u64);
            h = mix(h, canvas.h as u64);
            h = mix(h, canvas.pixels.len() as u64);
        }
    }

    let mut textures: u64 = 0xcbf29ce484222325;
    textures = mix(textures, project.texture_revision);
    for material in &project.materials {
        textures = hash_bytes(textures, material.id.as_bytes());
        if let Some(canvas) = material.albedo_texture.as_ref() {
            textures = mix(textures, canvas.w as u64);
            textures = mix(textures, canvas.h as u64);
            textures = mix(textures, canvas.pixels.len() as u64);
        }
    }
    for asset in &project.assets {
        textures = hash_bytes(textures, asset.id.as_bytes());
        if let Some(canvas) = asset.texture.as_ref() {
            textures = mix(textures, canvas.w as u64);
            textures = mix(textures, canvas.h as u64);
            textures = mix(textures, canvas.pixels.len() as u64);
        }
    }

    let mut r: u64 = 0xcbf29ce484222325;
    r = mix(r, refs.len() as u64);
    for rf in refs {
        r = mix(r, rf.width as u64);
        r = mix(r, rf.height as u64);
        r = mix(r, rf.visible as u64);
        r = mix(r, rf.xray as u64);
        r = mix(r, rf.axis as u64);
        r = hash_f32(r, rf.offset);
        r = hash_f32(r, rf.size);
        r = hash_f32(r, rf.rotation);
        r = hash_f32(r, rf.opacity);
    }

    SceneFingerprint {
        mesh: h,
        textures,
        refs_layout: r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::{Face, Mesh, Vertex};
    use petunia_project::Canvas;

    fn cube_project() -> Project {
        let mut p = Project::new();
        p.assets.clear();
        let mut mesh = Mesh {
            verts: vec![
                Vertex::new(0.0, 0.0, 0.0),
                Vertex::new(1.0, 0.0, 0.0),
                Vertex::new(1.0, 1.0, 0.0),
                Vertex::new(0.0, 1.0, 0.0),
            ],
            ..Default::default()
        };
        mesh.push_face(Face::new(vec![0, 1, 2, 3]));
        let asset = petunia_project::Asset::new("cube", mesh);
        p.assets.push(asset);
        p
    }

    fn flags(shading: Shading) -> FingerprintFlags {
        FingerprintFlags {
            shading,
            xray: false,
            show_triangulation: false,
            textured: false,
            edit_mode_is_edit: false,
            show_wireframe_overlay: false,
            show_face_orientation: false,
            show_uv_checker: false,
        }
    }

    #[test]
    fn identical_scene_same_fingerprint() {
        let p = cube_project();
        let a = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        let b = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        assert_eq!(a, b);
    }

    #[test]
    fn shading_flag_changes_mesh_fingerprint() {
        let p = cube_project();
        let a = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        let b = fingerprint_scene(&p, &[], flags(Shading::Wireframe));
        assert_ne!(a.mesh, b.mesh);
    }

    #[test]
    fn vertex_move_changes_fingerprint_without_count_change() {
        let mut p = cube_project();
        let a = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        p.assets[0].mesh.verts[0].pos = [5.0, 0.0, 0.0];
        let b = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        assert_ne!(a.mesh, b.mesh);
    }

    #[test]
    fn selection_change_invalidates() {
        let mut p = cube_project();
        let a = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        p.assets[0].mesh.verts[0].selected = true;
        let b = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        assert_ne!(a.mesh, b.mesh);
    }

    #[test]
    fn direct_uv_change_invalidates_unrevisioned_legacy_project() {
        let mut project = cube_project();
        let before = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));
        project.assets[0].mesh.faces[0].uv[0] = [0.25, 0.75];
        let after = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));
        assert_ne!(before.mesh, after.mesh);
    }

    #[test]
    fn visibility_change_invalidates() {
        let mut p = cube_project();
        let a = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        p.assets[0].visible = false;
        let b = fingerprint_scene(&p, &[], flags(Shading::MaterialPreview));
        assert_ne!(a.mesh, b.mesh);
    }

    #[test]
    fn uv_revision_changes_mesh_fingerprint_without_geometry_revisions() {
        let mut project = cube_project();
        project.bump_topology();
        let before = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));

        project.bump_uvs();
        let after = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));

        assert_ne!(before.mesh, after.mesh);
        assert_eq!(project.position_revision, 0);
    }

    #[test]
    fn texture_revision_invalidates_texture_without_rebuilding_mesh() {
        let mut project = cube_project();
        project.assets[0].texture = Some(Canvas::new(16, 16, [0, 0, 0, 255]));
        let before = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));

        project.assets[0]
            .texture
            .as_mut()
            .unwrap()
            .set(3, 4, [255, 0, 0, 255]);
        project.bump_textures();
        let after = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));

        assert_eq!(before.mesh, after.mesh);
        assert_ne!(before.textures, after.textures);
    }

    #[test]
    fn direct_texture_mutation_requires_revision_publication() {
        let mut project = cube_project();
        project.assets[0].texture = Some(Canvas::new(8, 8, [0, 0, 0, 255]));
        let before = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));

        project.assets[0]
            .texture
            .as_mut()
            .unwrap()
            .set(1, 2, [0, 255, 0, 255]);
        let unpublished = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));
        project.bump_textures();
        let published = fingerprint_scene(&project, &[], flags(Shading::MaterialPreview));

        assert_eq!(before, unpublished);
        assert_eq!(before.mesh, published.mesh);
        assert_ne!(before.textures, published.textures);
    }
}
