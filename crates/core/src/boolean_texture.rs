//! Transfers painted albedo into the output atlas after a boolean.
use glam::Vec3;
use petunia_mesh::{Mesh, boolean_cleanup::surface_sources};
use petunia_project::material::TextureChannel;
use petunia_project::{Asset, Canvas, Material, Project};

/// Precompute source triangles once per face, retaining non-affine quad UVs.
struct UvSampler(Vec<([Vec3; 3], [[f32; 2]; 3])>);
impl UvSampler {
    fn new(mesh: &Mesh, face: usize) -> Self {
        Self(
            mesh.face_triangle_corners(face)
                .into_iter()
                .map(|corners| {
                    let f = &mesh.faces[face];
                    (
                        corners.map(|c| mesh.verts[f.verts[c] as usize].vec()),
                        corners.map(|c| f.uv.get(c).copied().unwrap_or([0.0; 2])),
                    )
                })
                .collect(),
        )
    }
    fn at(&self, point: Vec3) -> [f32; 2] {
        self.locate(point).0
    }
    fn locate(&self, point: Vec3) -> ([f32; 2], usize) {
        let mut fallback = ([0.0; 2], f32::NEG_INFINITY, 0);
        for (index, (p, uv)) in self.0.iter().enumerate() {
            let (u, v, q) = (p[1] - p[0], p[2] - p[0], point - p[0]);
            let (aa, ab, bb) = (u.dot(u), u.dot(v), v.dot(v));
            let det = aa * bb - ab * ab;
            if det.abs() < 1e-20 {
                continue;
            }
            let s = (q.dot(u) * bb - q.dot(v) * ab) / det;
            let t = (q.dot(v) * aa - q.dot(u) * ab) / det;
            let weight = s.min(t).min(1.0 - s - t);
            let value = [
                uv[0][0] * (1.0 - s - t) + uv[1][0] * s + uv[2][0] * t,
                uv[0][1] * (1.0 - s - t) + uv[1][1] * s + uv[2][1] * t,
            ];
            if weight >= -1e-4 {
                return (value, index);
            }
            if weight > fallback.1 {
                fallback = (value, weight, index);
            }
        }
        (fallback.0, fallback.2)
    }
    fn basis(&self, point: Vec3) -> Option<[Vec3; 3]> {
        let (_, index) = self.locate(point);
        let (p, uv) = self.0.get(index)?;
        let (e1, e2) = (p[1] - p[0], p[2] - p[0]);
        let (du1, dv1, du2, dv2) = (
            uv[1][0] - uv[0][0],
            uv[1][1] - uv[0][1],
            uv[2][0] - uv[0][0],
            uv[2][1] - uv[0][1],
        );
        let det = du1 * dv2 - dv1 * du2;
        if det.abs() < 1e-12 {
            return None;
        }
        let n = e1.cross(e2).normalize_or_zero();
        let t = (e1 * dv2 - e2 * dv1) / det;
        let t = (t - n * t.dot(n)).normalize_or_zero();
        let raw_b = (e2 * du1 - e1 * du2) / det;
        let b = n.cross(t)
            * if n.cross(t).dot(raw_b) < 0.0 {
                -1.0
            } else {
                1.0
            };
        Some([t, b, n])
    }
}

fn reorient_normal(color: [u8; 4], source: &UvSampler, output: &UvSampler, pos: Vec3) -> [u8; 4] {
    let (Some([t, mut b, mut n]), Some([ot, ob, on])) = (source.basis(pos), output.basis(pos))
    else {
        return color;
    };
    if n.dot(on) < 0.0 {
        n = -n;
        b = -b;
    }
    let v = Vec3::new(color[0] as f32, color[1] as f32, color[2] as f32) / 127.5 - Vec3::ONE;
    let world = (t * v.x + b * v.y + n * v.z).normalize_or_zero();
    let local = Vec3::new(world.dot(ot), world.dot(ob), world.dot(on)).normalize_or_zero();
    let rgb = ((local + Vec3::ONE) * 127.5)
        .to_array()
        .map(|v| v.clamp(0.0, 255.0).round() as u8);
    [rgb[0], rgb[1], rgb[2], color[3]]
}

pub(crate) struct Transfer {
    pub albedo: Canvas,
    pub material: Option<Material>,
}

pub(crate) fn transfer(
    project: &Project,
    result: &mut Mesh,
    a: &Asset,
    b: &Asset,
) -> Option<Transfer> {
    let textures = [a, b].map(|asset| {
        asset.texture.as_ref().or_else(|| {
            asset
                .material(project)
                .and_then(|m| m.albedo_texture.as_ref())
        })
    });
    // No atlas change is needed for wholly untextured operands.
    if textures.iter().all(|t| t.is_none())
        && [a, b].iter().all(|asset| {
            asset.material(project).is_none_or(|m| {
                [
                    TextureChannel::Normal,
                    TextureChannel::Roughness,
                    TextureChannel::Metallic,
                    TextureChannel::Emission,
                    TextureChannel::Height,
                ]
                .iter()
                .all(|c| m.channel_texture(*c).is_none())
            })
        })
    {
        return None;
    }
    let sources = surface_sources(result, &a.mesh, &b.mesh);
    result.layout_uv_charts(petunia_mesh::primitives::DEFAULT_UV_PADDING);
    let size = textures
        .iter()
        .flatten()
        .map(|c| c.w.max(c.h))
        .chain(
            [a, b]
                .into_iter()
                .filter_map(|a| a.material(project))
                .flat_map(|m| {
                    [
                        m.normal_texture.as_ref(),
                        m.roughness_texture.as_ref(),
                        m.metallic_texture.as_ref(),
                        m.emission_texture.as_ref(),
                        m.height_texture.as_ref(),
                    ]
                })
                .flatten()
                .map(|c| c.w.max(c.h)),
        )
        .max()
        .unwrap_or(256)
        .clamp(64, 1024);
    let mut out = Canvas::new(size, size, [0; 4]);
    for (face, source) in sources.iter().enumerate() {
        let Some((side, source_face)) = source else {
            continue;
        };
        let asset = if *side == 0 { a } else { b };
        let tex = textures[*side as usize];
        let sampler = UvSampler::new(&asset.mesh, *source_face);
        let base = asset
            .base_color
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        result.rasterize_face_texels(face, size, size, 0.0, |x, y, pos, _| {
            let uv = sampler.at(pos);
            let color = tex
                .and_then(|c| {
                    c.get(
                        (uv[0].clamp(0.0, 1.0) * (c.w - 1) as f32).round() as u32,
                        ((1.0 - uv[1]).clamp(0.0, 1.0) * (c.h - 1) as f32).round() as u32,
                    )
                })
                .unwrap_or([base[0], base[1], base[2], 255]);
            out.pixels[((y * size + x) * 4) as usize..((y * size + x) * 4 + 4) as usize]
                .copy_from_slice(&color);
        });
    }
    let mut material = a
        .material(project)
        .cloned()
        .or_else(|| b.material(project).cloned());
    if let Some(material) = &mut material {
        material.id = uuid::Uuid::new_v4();
        material.albedo_texture = Some(out.clone());
        for channel in [
            TextureChannel::Normal,
            TextureChannel::Roughness,
            TextureChannel::Metallic,
            TextureChannel::Emission,
            TextureChannel::Height,
        ] {
            let maps = [a, b].map(|asset| {
                asset
                    .material(project)
                    .and_then(|m| m.channel_texture(channel))
            });
            if maps.iter().all(|m| m.is_none()) {
                material.set_channel_texture(channel, None);
                continue;
            }
            let mut atlas = Canvas::new(size, size, [0; 4]);
            for (face, source) in sources.iter().enumerate() {
                let Some((side, source_face)) = source else {
                    continue;
                };
                let asset = if *side == 0 { a } else { b };
                let sampler = UvSampler::new(&asset.mesh, *source_face);
                let output_sampler = UvSampler::new(result, face);
                let source_material = asset.material(project);
                let scalar = match channel {
                    TextureChannel::Roughness => source_material.map_or(0.5, |m| m.roughness),
                    TextureChannel::Metallic => source_material.map_or(0.0, |m| m.metallic),
                    _ => 0.0,
                };
                let value = (scalar.clamp(0.0, 1.0) * 255.0).round() as u8;
                let fallback = if channel == TextureChannel::Normal {
                    [128, 128, 255, 255]
                } else {
                    [value, value, value, 255]
                };
                result.rasterize_face_texels(face, size, size, 0.0, |x, y, pos, _| {
                    let uv = sampler.at(pos);
                    let color = maps[*side as usize]
                        .and_then(|c| {
                            c.get(
                                (uv[0].clamp(0.0, 1.0) * (c.w - 1) as f32).round() as u32,
                                ((1.0 - uv[1]).clamp(0.0, 1.0) * (c.h - 1) as f32).round() as u32,
                            )
                        })
                        .unwrap_or(fallback);
                    let color = if channel == TextureChannel::Normal {
                        reorient_normal(color, &sampler, &output_sampler, pos)
                    } else {
                        color
                    };
                    let index = ((y * size + x) * 4) as usize;
                    atlas.pixels[index..index + 4].copy_from_slice(&color);
                });
            }
            material.set_channel_texture(channel, Some(atlas));
        }
    }
    Some(Transfer {
        albedo: out,
        material,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tangent_normal_rotates_with_the_new_uv_basis() {
        let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let source = UvSampler(vec![(positions, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]])]);
        let output = UvSampler(vec![(positions, [[0.0, 0.0], [0.0, -1.0], [1.0, 0.0]])]);
        let color = reorient_normal(
            [255, 128, 128, 255],
            &source,
            &output,
            Vec3::new(0.2, 0.2, 0.0),
        );
        assert!(color[1] < 2);
        assert!((color[0] as i32 - 128).abs() < 2);
        let flat = reorient_normal(
            [128, 128, 255, 255],
            &source,
            &output,
            Vec3::new(0.2, 0.2, 0.0),
        );
        assert!(flat[2] > 253);
    }

    #[test]
    fn cut_transfers_both_operand_textures_into_a_new_atlas() {
        let state = crate::AppState::default();
        let mut a = state.project.assets[0].clone();
        let mut b = a.clone();
        a.mesh = Mesh::cube(2.0).into();
        b.mesh = Mesh::cube(1.0).into();
        for vertex in &mut b.mesh.verts {
            vertex.pos[0] += 0.8;
        }
        a.texture = Some(Canvas::new(64, 64, [255, 0, 0, 255]));
        b.texture = Some(Canvas::new(64, 64, [0, 0, 255, 255]));
        let (mut result, _) = petunia_mesh::boolean::boolean_meshes_clean(
            &a.mesh,
            &b.mesh,
            petunia_mesh::boolean::BooleanOp::Difference,
        )
        .unwrap();
        let atlas = transfer(&state.project.project, &mut result, &a, &b)
            .unwrap()
            .albedo;
        assert!(
            atlas
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p == &[255, 0, 0, 255])
        );
        assert!(
            atlas
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p == &[0, 0, 255, 255])
        );
        assert!(result.faces.iter().all(|f| f.uv.len() == f.verts.len()));
        assert_eq!(
            a.texture.as_ref().unwrap().get(0, 0),
            Some([255, 0, 0, 255])
        );
    }
}
