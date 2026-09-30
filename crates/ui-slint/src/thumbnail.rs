//! Miniaturas de prefabs renderizadas pelo caminho de software (sem GPU).
//!
//! O resultado é RGBA8 com fundo transparente (pixels intocados pelo depth
//! buffer), pronto para virar `slint::Image` na camada de UI.

use glam::Vec3;
use petunia_core::Camera;
use petunia_project::Project;
use petunia_project::prefab::Prefab;

use crate::viewport_soft::Software3dViewport;
use crate::{PetuniaViewport, ViewportRenderState};

/// Lado (px) das miniaturas geradas.
pub const THUMBNAIL_SIZE: u32 = 128;

/// Bytes RGBA8 `THUMBNAIL_SIZE²` do prefab visto em 3/4, ou `None` sem geometria.
pub fn render_prefab_thumbnail(prefab: &Prefab) -> Option<Vec<u8>> {
    let radius = prefab
        .parts
        .iter()
        .flat_map(|part| part.mesh.verts.iter())
        .map(|v| Vec3::from_array(v.pos).length())
        .fold(0.0f32, f32::max);
    if !radius.is_finite() || radius <= 1.0e-6 {
        return None;
    }
    let mut project = Project {
        assets: prefab.parts.clone(),
        ..Project::default()
    };
    for asset in &mut project.assets {
        asset.visible = true;
    }
    let mut camera = Camera {
        aspect: 1.0,
        yaw: 0.75,
        pitch: 0.45,
        ..Camera::default()
    };
    camera.frame(Vec3::ZERO, radius);
    let mut viewport = Software3dViewport::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    let state = ViewportRenderState {
        show_grid: false,
        show_wireframe_overlay: false,
        ..ViewportRenderState::default()
    };
    viewport.render_frame(&project, &[], &camera, state)?;
    let mut rgba = viewport.color_buffer.clone();
    for (pixel, depth) in rgba
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(&viewport.depth_buffer)
    {
        if *depth >= 1.0 {
            pixel.copy_from_slice(&[0, 0, 0, 0]);
        }
    }
    Some(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;
    use petunia_project::Asset;

    #[test]
    fn thumbnail_has_geometry_pixels_and_transparent_background() {
        let mut project = Project::default();
        let cube = Asset::new("Cube", Mesh::cube(1.0));
        let id = cube.id;
        project.assets.push(cube);
        let prefab_id = project.create_prefab(&[id], None).unwrap();
        let prefab = project.prefabs.iter().find(|p| p.id == prefab_id).unwrap();
        let rgba = render_prefab_thumbnail(prefab).expect("thumbnail");
        assert_eq!(rgba.len(), (THUMBNAIL_SIZE * THUMBNAIL_SIZE * 4) as usize);
        let opaque = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] == 255)
            .count();
        let clear = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] == 0).count();
        assert!(opaque > 500, "a malha deve aparecer ({opaque} px)");
        assert!(clear > 500, "o fundo deve ficar transparente ({clear} px)");
    }
}
