//! Screen-space component picking shared by clicks and hover previews.
//! Tolerances are physical pixels; pass the viewport size in the same units.

#![forbid(unsafe_code)]

use crate::{Camera, SelectMode};
use glam::{Mat4, Vec2, Vec3};
use petunia_mesh::{Mesh, triangulate};

pub const VERTEX_RADIUS_PIXELS: f32 = 8.0;
pub const EDGE_RADIUS_PIXELS: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickComponent {
    Vertex(usize),
    Edge(u32, u32),
    Face(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickHit {
    pub component: PickComponent,
    pub position: Vec3,
}

pub(crate) struct Triangle {
    pub(crate) face: usize,
    pub(crate) points: [Vec3; 3],
}

/// Picks the nearest visible component within a fixed pixel tolerance.
/// `xray` disables face occlusion for vertices and edges (also use for wireframe).
/// Invalid viewports, coordinates and malformed faces are ignored safely.
pub fn pick_mesh(
    mesh: &Mesh,
    camera: &Camera,
    viewport_pixels: Vec2,
    cursor_ndc: Vec2,
    mode: SelectMode,
    xray: bool,
) -> Option<PickHit> {
    pick_mesh_filtered(
        mesh,
        camera,
        viewport_pixels,
        cursor_ndc,
        mode,
        xray,
        |_| true,
    )
}

/// Same component picker with an additional scene visibility predicate. It is
/// evaluated for each candidate, so an occluded near candidate cannot hide a
/// visible candidate inside the same pixel tolerance.
pub fn pick_mesh_filtered(
    mesh: &Mesh,
    camera: &Camera,
    viewport_pixels: Vec2,
    cursor_ndc: Vec2,
    mode: SelectMode,
    xray: bool,
    visible: impl Fn(Vec3) -> bool,
) -> Option<PickHit> {
    if !viewport_pixels.is_finite()
        || viewport_pixels.min_element() <= 0.0
        || !cursor_ndc.is_finite()
    {
        return None;
    }
    let matrix = camera.view_proj();
    let inverse = matrix.inverse();
    if !inverse.is_finite() {
        return None;
    }
    let triangles = triangles(mesh);
    if mode == SelectMode::Face {
        let (origin, direction) = ray(inverse, cursor_ndc)?;
        return nearest_face(&triangles, origin, direction).and_then(|(face, distance)| {
            let position = origin + direction * distance;
            project(matrix, position)
                .filter(|_| xray || visible(position))
                .map(|_| PickHit {
                    component: PickComponent::Face(face),
                    position,
                })
        });
    }
    let pixel_scale = viewport_pixels * 0.5;
    let mut best: Option<(f32, f32, PickHit)> = None;
    let mut consider = |component, position: Vec3, tolerance: f32| {
        let Some(ndc) = project(matrix, position) else {
            return;
        };
        let pixel_distance = ((ndc.truncate() - cursor_ndc) * pixel_scale).length();
        if pixel_distance > tolerance {
            return;
        }
        if !xray && (occluded(&triangles, inverse, ndc.truncate(), position) || !visible(position))
        {
            return;
        }
        // Pixel distance gives predictable targeting; depth breaks overlapping ties.
        if best.as_ref().is_none_or(|(distance, depth, _)| {
            pixel_distance < *distance - 0.01
                || ((pixel_distance - distance).abs() <= 0.01 && ndc.z < *depth)
        }) {
            best = Some((
                pixel_distance,
                ndc.z,
                PickHit {
                    component,
                    position,
                },
            ));
        }
    };
    match mode {
        SelectMode::Vertex => {
            for (index, vertex) in mesh.verts.iter().enumerate() {
                consider(
                    PickComponent::Vertex(index),
                    vertex.vec(),
                    VERTEX_RADIUS_PIXELS,
                );
            }
        }
        SelectMode::Edge => {
            for (a, b) in mesh.edges_unique() {
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                let Some((pa, pb)) = clip_depth(matrix, va.vec(), vb.vec()) else {
                    continue;
                };
                let (Some(sa), Some(sb)) = (project(matrix, pa), project(matrix, pb)) else {
                    continue;
                };
                let start = sa.truncate() * pixel_scale;
                let segment = (sb.truncate() - sa.truncate()) * pixel_scale;
                let fraction = if segment.length_squared() > 1e-8 {
                    ((cursor_ndc * pixel_scale - start).dot(segment) / segment.length_squared())
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                // Perspective-correct interpolation of the closest screen-space point.
                let wa = (matrix * pa.extend(1.0)).w;
                let wb = (matrix * pb.extend(1.0)).w;
                let fraction = fraction * wa / ((1.0 - fraction) * wb + fraction * wa);
                consider(
                    PickComponent::Edge(a, b),
                    pa.lerp(pb, fraction),
                    EDGE_RADIUS_PIXELS,
                );
            }
        }
        SelectMode::Face => {}
    }
    best.map(|(_, _, hit)| hit)
}

pub(crate) fn project(matrix: Mat4, position: Vec3) -> Option<Vec3> {
    let clip = matrix * position.extend(1.0);
    if !clip.is_finite() || clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    (-1e-5..=1.00001).contains(&ndc.z).then_some(ndc)
}

pub(crate) fn ray(inverse: Mat4, ndc: Vec2) -> Option<(Vec3, Vec3)> {
    let near = inverse.project_point3(ndc.extend(0.0));
    let far = inverse.project_point3(ndc.extend(1.0));
    let direction = (far - near).normalize_or_zero();
    (near.is_finite() && direction.is_finite() && direction.length_squared() > 0.5)
        .then_some((near, direction))
}

pub(crate) fn clip_depth(matrix: Mat4, mut a: Vec3, mut b: Vec3) -> Option<(Vec3, Vec3)> {
    if !a.is_finite() || !b.is_finite() {
        return None;
    }
    for far in [false, true] {
        let ca = matrix * a.extend(1.0);
        let cb = matrix * b.extend(1.0);
        let da = if far { ca.w - ca.z } else { ca.z };
        let db = if far { cb.w - cb.z } else { cb.z };
        if da < 0.0 && db < 0.0 {
            return None;
        }
        if (da < 0.0) != (db < 0.0) {
            let clipped = a.lerp(b, da / (da - db));
            if da < 0.0 {
                a = clipped;
            } else {
                b = clipped;
            }
        }
    }
    Some((a, b))
}

pub(crate) fn triangles(mesh: &Mesh) -> Vec<Triangle> {
    let mut result = Vec::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        result.extend(
            mesh.face_triangle_corners(face_index)
                .into_iter()
                .map(|corners| Triangle {
                    face: face_index,
                    points: corners.map(|corner| mesh.verts[face.verts[corner] as usize].vec()),
                }),
        );
    }
    result
}

pub(crate) fn nearest_face(
    triangles: &[Triangle],
    origin: Vec3,
    direction: Vec3,
) -> Option<(usize, f32)> {
    triangles
        .iter()
        .filter_map(|triangle| {
            let [a, b, c] = triangle.points;
            triangulate::ray_tri(origin, direction, a, b, c)
                .filter(|distance| distance.is_finite())
                .map(|distance| (triangle.face, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub(crate) fn occluded(triangles: &[Triangle], inverse: Mat4, ndc: Vec2, position: Vec3) -> bool {
    let Some((origin, direction)) = ray(inverse, ndc) else {
        return true;
    };
    let distance = (position - origin).dot(direction);
    let tolerance = (distance.abs() * 1e-5).max(1e-5);
    nearest_face(triangles, origin, direction)
        .is_some_and(|(_, face_distance)| face_distance < distance - tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewPreset;
    use petunia_mesh::{Face, Vertex};

    fn camera() -> Camera {
        let mut camera = Camera::default();
        camera.set_preset(ViewPreset::Front);
        camera.aspect = 1.0;
        camera.ortho_half_h = 2.0;
        camera
    }

    fn pick(mesh: &Mesh, cursor: Vec2, mode: SelectMode, xray: bool) -> Option<PickHit> {
        pick_mesh(mesh, &camera(), Vec2::splat(800.0), cursor, mode, xray)
    }

    #[test]
    fn vertex_tolerance_stays_eight_pixels_at_different_zoom_and_resolution() {
        let mesh = Mesh {
            verts: vec![Vertex::new(0.0, 0.0, 0.0)],
            ..Mesh::default()
        };
        for size in [400.0, 1600.0] {
            for zoom in [1.0, 10.0] {
                let mut camera = camera();
                camera.ortho_half_h = zoom;
                for (offset, expected) in [(7.9, true), (8.1, false)] {
                    assert_eq!(
                        pick_mesh(
                            &mesh,
                            &camera,
                            Vec2::splat(size),
                            Vec2::new(offset * 2.0 / size, 0.0),
                            SelectMode::Vertex,
                            false
                        )
                        .is_some(),
                        expected
                    );
                }
            }
        }
    }

    #[test]
    fn edge_is_continuous_and_has_five_pixel_tolerance() {
        let mesh = Mesh {
            verts: vec![
                Vertex::new(-1.0, 0.0, 0.0),
                Vertex::new(1.0, 0.0, 0.0),
                Vertex::new(0.0, -1.0, 0.0),
            ],
            faces: vec![Face::new(vec![0, 1, 2])],
            ..Mesh::default()
        };
        let cursor = Vec2::new(0.073, 4.9 / 400.0);
        let hit = pick(&mesh, cursor, SelectMode::Edge, false).unwrap();
        assert_eq!(hit.component, PickComponent::Edge(0, 1));
        assert!((hit.position.x - 0.146).abs() < 1e-5);
        assert!(
            pick(
                &mesh,
                Vec2::new(0.073, 5.1 / 400.0),
                SelectMode::Edge,
                false
            )
            .is_none()
        );
    }

    #[test]
    fn hidden_vertex_and_edge_require_xray() {
        let mut mesh = Mesh::cube(2.0);
        let index = mesh.verts.len();
        mesh.verts.extend([
            Vertex::new(0.0, 0.0, -2.0),
            Vertex::new(0.5, 0.0, -2.0),
            Vertex::new(0.0, -0.5, -2.0),
        ]);
        mesh.faces.push(Face::new(vec![
            index as u32,
            index as u32 + 1,
            index as u32 + 2,
        ]));
        assert!(pick(&mesh, Vec2::ZERO, SelectMode::Vertex, false).is_none());
        assert_eq!(
            pick(&mesh, Vec2::ZERO, SelectMode::Vertex, true)
                .unwrap()
                .component,
            PickComponent::Vertex(index)
        );
        assert!(pick(&mesh, Vec2::new(0.1, 0.0), SelectMode::Edge, false).is_none());
        assert!(pick(&mesh, Vec2::new(0.1, 0.0), SelectMode::Edge, true).is_some());
    }

    #[test]
    fn faces_choose_nearest_surface() {
        let mesh = Mesh::cube(2.0);
        let hit = pick(&mesh, Vec2::ZERO, SelectMode::Face, false).unwrap();
        assert!((hit.position.z - 1.0).abs() < 1e-4);
    }

    #[test]
    fn concave_polygon_does_not_pick_notch() {
        let mesh = Mesh {
            verts: vec![
                Vertex::new(-1.0, -1.0, 0.0),
                Vertex::new(1.0, -1.0, 0.0),
                Vertex::new(1.0, 1.0, 0.0),
                Vertex::new(0.0, 0.0, 0.0),
                Vertex::new(-1.0, 1.0, 0.0),
            ],
            faces: vec![Face::new(vec![0, 1, 2, 3, 4])],
            ..Mesh::default()
        };
        assert!(pick(&mesh, Vec2::new(0.0, 0.35), SelectMode::Face, false).is_none());
        assert!(pick(&mesh, Vec2::new(0.0, -0.35), SelectMode::Face, false).is_some());
    }

    #[test]
    fn malformed_mesh_and_invalid_viewport_are_safe() {
        let mesh = Mesh {
            verts: vec![Vertex::new(f32::NAN, 0.0, 0.0)],
            faces: vec![Face::new(vec![0, 99, 100])],
            ..Mesh::default()
        };
        for mode in [SelectMode::Vertex, SelectMode::Edge, SelectMode::Face] {
            assert!(pick(&mesh, Vec2::ZERO, mode, false).is_none());
            assert!(pick_mesh(&mesh, &camera(), Vec2::ZERO, Vec2::ZERO, mode, false).is_none());
        }
    }

    #[test]
    fn perspective_edge_hit_projects_back_to_cursor() {
        let mesh = Mesh {
            verts: vec![
                Vertex::new(-1.0, 0.0, 2.0),
                Vertex::new(1.0, 0.0, -2.0),
                Vertex::new(0.0, -1.0, 0.0),
            ],
            faces: vec![Face::new(vec![0, 1, 2])],
            ..Mesh::default()
        };
        let mut camera = camera();
        camera.proj = crate::Projection::Perspective;
        let a = camera.project_ndc(mesh.verts[0].vec()).truncate();
        let b = camera.project_ndc(mesh.verts[1].vec()).truncate();
        let cursor = a.lerp(b, 0.37);
        let hit = pick_mesh(
            &mesh,
            &camera,
            Vec2::splat(800.0),
            cursor,
            SelectMode::Edge,
            true,
        )
        .unwrap();
        assert_eq!(hit.component, PickComponent::Edge(0, 1));
        assert!(camera.project_ndc(hit.position).truncate().distance(cursor) < 1e-5);
    }

    #[test]
    fn top_view_pick_matches_render_projection() {
        let mesh = Mesh::cube(2.0);
        let mut camera = camera();
        camera.set_preset(ViewPreset::Top);
        let cursor = Vec2::new(0.1, 0.2);
        let hit = pick_mesh(
            &mesh,
            &camera,
            Vec2::splat(800.0),
            cursor,
            SelectMode::Face,
            false,
        )
        .unwrap();
        assert!(camera.project_ndc(hit.position).truncate().distance(cursor) < 1e-4);
        assert!((hit.position.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn vertices_behind_camera_are_not_pickable_in_xray() {
        let mesh = Mesh {
            verts: vec![Vertex::new(0.0, 0.0, 9.0)],
            ..Mesh::default()
        };
        assert!(pick(&mesh, Vec2::ZERO, SelectMode::Vertex, true).is_none());
    }
}
