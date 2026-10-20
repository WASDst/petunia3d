//! Derived display atlas for authored per-face surface bindings. Neither the
//! source materials nor the mesh UVs are rewritten by this renderer adapter.
use petunia_project::{Asset, Canvas, Project, SurfaceMaterial};

type SurfaceTriangle = ([f32; 3], [f32; 3], [f32; 3], [f32; 2]);

fn binding(asset: &Asset, slot: Option<usize>) -> Option<&SurfaceMaterial> {
    slot.and_then(|slot| asset.surface_materials.get(slot))
        .or_else(|| asset.surface_materials.first())
}

fn layout(asset: &Asset) -> (u32, u32) {
    let count = asset.surface_materials.len().clamp(1, 256) as u32;
    let columns = (count as f32).sqrt().ceil() as u32;
    (columns, count.div_ceil(columns))
}

pub(crate) fn atlas(project: &Project, asset: &Asset) -> Option<Canvas> {
    if asset.surface_materials.is_empty() {
        return None;
    }
    let (columns, rows) = layout(asset);
    let max_side = asset
        .surface_materials
        .iter()
        .take(256)
        .filter_map(|b| {
            asset.texture.as_ref().or(b.texture.as_ref()).or_else(|| {
                b.material_id
                    .and_then(|id| project.get_material(id))
                    .and_then(|m| m.albedo_texture.as_ref())
            })
        })
        .map(|c| c.w.max(c.h))
        .max()
        .unwrap_or(1);
    // Includes a replicated border in every tile. Bound the derived allocation
    // independently of hostile source texture dimensions and material counts.
    let side = max_side
        .saturating_add(2)
        .clamp(4, 4096 / columns.max(rows));
    let mut canvas = Canvas::new(columns * side, rows * side, [255; 4]);
    for (slot, surface) in asset.surface_materials.iter().take(256).enumerate() {
        let texture = asset
            .texture
            .as_ref()
            .or(surface.texture.as_ref())
            .or_else(|| {
                surface
                    .material_id
                    .and_then(|id| project.get_material(id))
                    .and_then(|m| m.albedo_texture.as_ref())
            });
        let Some(texture) = texture else {
            continue;
        };
        let column = slot as u32 % columns;
        let row = slot as u32 / columns;
        for y in 0..side {
            for x in 0..side {
                let u = x.saturating_sub(1).min(side - 3) as f32 / (side - 3) as f32;
                let v = y.saturating_sub(1).min(side - 3) as f32 / (side - 3) as f32;
                let color = texture
                    .get(
                        (u * texture.w.saturating_sub(1) as f32).round() as u32,
                        (v * texture.h.saturating_sub(1) as f32).round() as u32,
                    )
                    .unwrap_or([255; 4]);
                let offset = (((row * side + y) * canvas.w + column * side + x) * 4) as usize;
                if let Some(pixel) = canvas.pixels.get_mut(offset..offset.saturating_add(4)) {
                    pixel.copy_from_slice(&color);
                }
            }
        }
    }
    Some(canvas)
}

pub(crate) fn triangles(
    project: &Project,
    asset: &Asset,
    mesh: &petunia_core::Mesh,
    atlas: Option<&Canvas>,
) -> Vec<SurfaceTriangle> {
    let (columns, _) = layout(asset);
    mesh.shaded_corners(asset.surface_shading)
        .into_iter()
        .map(|corner| {
            let slot = mesh.faces.get(corner.face).and_then(|f| f.material_slot);
            let surface = binding(asset, slot);
            let material = if asset.texture.is_some() {
                asset.material(project)
            } else {
                surface
                    .and_then(|b| b.material_id)
                    .and_then(|id| project.get_material(id))
            };
            let mut color = material
                .map(|m| [m.base_color[0], m.base_color[1], m.base_color[2]])
                .or_else(|| surface.map(|b| b.base_color))
                .unwrap_or(asset.base_color);
            let default_vertex_color =
                [[0.72, 0.73, 0.78], [0.75, 0.75, 0.78]]
                    .iter()
                    .any(|default| {
                        corner
                            .color
                            .iter()
                            .zip(default)
                            .all(|(a, b)| (*a - *b).abs() < 0.001)
                    });
            if !default_vertex_color {
                color = corner.color;
            }
            if let Some(m) = material {
                for (c, emission) in color.iter_mut().zip(m.emission_color) {
                    *c = (*c + emission * m.emission_strength).clamp(0., 1.);
                }
            }
            let normal = if material.is_some_and(|m| {
                matches!(
                    m.profile,
                    petunia_project::ShaderProfile::Unlit
                        | petunia_project::ShaderProfile::Emissive
                )
            }) {
                [0.; 3]
            } else {
                corner.normal
            };
            let uv = if let Some(atlas) = atlas {
                let side = atlas.w / columns;
                let slot = slot
                    .filter(|s| *s < asset.surface_materials.len().min(256))
                    .unwrap_or(0) as u32;
                let column = slot % columns;
                let row = slot / columns;
                // Canvas is top-origin; mesh UVs are bottom-origin.
                [
                    (column as f32 * side as f32
                        + 1.5
                        + corner.uv[0].clamp(0., 1.) * (side - 3) as f32)
                        / atlas.w as f32,
                    1. - (row as f32 * side as f32
                        + 1.5
                        + (1. - corner.uv[1].clamp(0., 1.)) * (side - 3) as f32)
                        / atlas.h as f32,
                ]
            } else {
                corner.uv
            };
            (corner.position, normal, color, uv)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_atlas_resolves_per_face_paint_without_changing_authored_uvs() {
        let project = Project::new();
        let mut asset = Asset::new("mixed", petunia_core::Mesh::cube(2.));
        asset.surface_materials = vec![
            SurfaceMaterial {
                material_id: None,
                base_color: [1.; 3],
                texture: Some(Canvas::new(2, 2, [255, 0, 0, 255])),
            },
            SurfaceMaterial {
                material_id: None,
                base_color: [1.; 3],
                texture: Some(Canvas::new(2, 2, [0, 0, 255, 255])),
            },
        ];
        for (i, face) in asset.mesh.faces.iter_mut().enumerate() {
            face.material_slot = Some(i % 2);
        }
        let before = asset.mesh.clone();
        let canvas = atlas(&project, &asset).unwrap();
        let triangles = triangles(&project, &asset, &asset.mesh, Some(&canvas));
        let samples: Vec<_> = triangles
            .iter()
            .map(|t| {
                canvas
                    .get(
                        (t.3[0] * canvas.w as f32) as u32,
                        ((1. - t.3[1]) * canvas.h as f32) as u32,
                    )
                    .unwrap()
            })
            .collect();
        assert!(samples.contains(&[255, 0, 0, 255]));
        assert!(samples.contains(&[0, 0, 255, 255]));
        assert_eq!(asset.mesh.faces[0].uv, before.faces[0].uv);
        assert_eq!(asset.mesh.faces[1].uv, before.faces[1].uv);
        asset.texture = Some(Canvas::new(2, 2, [0, 255, 0, 255]));
        let painted = atlas(&project, &asset).unwrap();
        let painted_triangles = self::triangles(&project, &asset, &asset.mesh, Some(&painted));
        assert!(painted_triangles.iter().all(|t| painted.get(
            (t.3[0] * painted.w as f32) as u32,
            ((1. - t.3[1]) * painted.h as f32) as u32
        ) == Some([0, 255, 0, 255])));
        assert_eq!(
            asset.surface_materials[0]
                .texture
                .as_ref()
                .unwrap()
                .get(0, 0),
            Some([255, 0, 0, 255])
        );
    }
}
