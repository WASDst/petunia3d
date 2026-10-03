//! Shared viewport query contract for picking, hover, screen-space paint and decals.
//! CPU fallback; GPU query buffers can fill the same DTO later.

use glam::Vec3;
use uuid::Uuid;

/// Per-sample hit from a viewport query (pixel or ray).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportQuerySample {
    pub object_id: Uuid,
    pub face_index: usize,
    pub depth: f32,
    pub uv: [f32; 2],
    pub world: Vec3,
    pub normal: Vec3,
}

/// Batch of samples covering a dab / pick / hover.
#[derive(Clone, Debug, Default)]
pub struct ViewportQueryBuffer {
    pub samples: Vec<ViewportQuerySample>,
}

impl ViewportQueryBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, sample: ViewportQuerySample) {
        self.samples.push(sample);
    }

    pub fn nearest(&self) -> Option<&ViewportQuerySample> {
        self.samples
            .iter()
            .min_by(|a, b| a.depth.total_cmp(&b.depth))
    }
}

/// Immutable scene geometry for a batch of visibility queries. Evaluated
/// meshes and triangulation are created once per batch, never per candidate.
pub struct ViewportSceneQuery {
    surfaces: Vec<QuerySurface>,
}

struct QuerySurface {
    asset_index: usize,
    locked: bool,
    min: Vec3,
    max: Vec3,
    triangles: Vec<[Vec3; 3]>,
}

impl ViewportSceneQuery {
    pub fn new(project: &petunia_project::Project) -> Self {
        let surfaces = project
            .assets
            .iter()
            .enumerate()
            .filter(|(_, asset)| asset.visible)
            .filter_map(|(asset_index, asset)| {
                let mesh = asset.evaluated_mesh_ref();
                let triangles: Vec<_> = mesh
                    .faces
                    .iter()
                    .enumerate()
                    .flat_map(|(fi, face)| {
                        mesh.face_triangle_corners(fi).into_iter().map(|corners| {
                            corners.map(|corner| mesh.verts[face.verts[corner] as usize].vec())
                        })
                    })
                    .collect();
                if triangles.is_empty() {
                    return None;
                }
                let min = triangles
                    .iter()
                    .flatten()
                    .copied()
                    .fold(Vec3::splat(f32::INFINITY), Vec3::min);
                let max = triangles
                    .iter()
                    .flatten()
                    .copied()
                    .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
                Some(QuerySurface {
                    asset_index,
                    locked: asset.locked,
                    min,
                    max,
                    triangles,
                })
            })
            .collect();
        Self { surfaces }
    }

    fn ray_hits_bounds(surface: &QuerySurface, origin: Vec3, direction: Vec3, limit: f32) -> bool {
        let (mut near, mut far) = (0.0_f32, limit);
        for axis in 0..3 {
            let lo = surface.min[axis] - 1.0e-5;
            let hi = surface.max[axis] + 1.0e-5;
            if direction[axis].abs() < 1.0e-10 {
                if origin[axis] < lo || origin[axis] > hi {
                    return false;
                }
            } else {
                let a = (lo - origin[axis]) / direction[axis];
                let b = (hi - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if near > far {
                    return false;
                }
            }
        }
        true
    }

    /// Locked objects still occlude, even though they cannot be picked.
    pub fn nearest_object(&self, camera: &crate::Camera, ndc: [f32; 2]) -> Option<usize> {
        let (origin, direction) = camera.ray(ndc[0], ndc[1]);
        let matrix = camera.view_proj();
        let mut nearest: Option<(usize, f32, bool)> = None;
        for surface in &self.surfaces {
            if !Self::ray_hits_bounds(
                surface,
                origin,
                direction,
                nearest.map_or(f32::INFINITY, |(_, d, _)| d),
            ) {
                continue;
            }
            for &[a, b, c] in &surface.triangles {
                let Some(depth) = petunia_mesh::triangulate::ray_tri(origin, direction, a, b, c)
                else {
                    continue;
                };
                let clip = matrix * (origin + direction * depth).extend(1.0);
                if clip.w > 0.0
                    && clip.z >= 0.0
                    && clip.z <= clip.w
                    && nearest.is_none_or(|(_, d, _)| depth < d)
                {
                    nearest = Some((surface.asset_index, depth, surface.locked));
                }
            }
        }
        nearest
            .filter(|(_, _, locked)| !locked)
            .map(|(index, _, _)| index)
    }

    pub fn point_visible(&self, camera: &crate::Camera, point: Vec3) -> bool {
        let matrix = camera.view_proj();
        let clip = matrix * point.extend(1.0);
        if !clip.is_finite() || clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w {
            return false;
        }
        let (origin, direction) = camera.ray(clip.x / clip.w, clip.y / clip.w);
        let target = (point - origin).dot(direction);
        let limit = target - 1.0e-5 * target.abs().max(1.0);
        for surface in &self.surfaces {
            if !Self::ray_hits_bounds(surface, origin, direction, limit) {
                continue;
            }
            for &[a, b, c] in &surface.triangles {
                if let Some(depth) = petunia_mesh::triangulate::ray_tri(origin, direction, a, b, c)
                    && depth < limit
                {
                    // Geometry outside the camera depth range cannot occlude.
                    let hit = matrix * (origin + direction * depth).extend(1.0);
                    if hit.w > 0.0 && hit.z >= 0.0 {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Convenience query for callers with a single candidate. Use one scene query
/// for a batch (box selection or screen-space component picking).
pub fn point_visible(
    project: &petunia_project::Project,
    camera: &crate::Camera,
    point: Vec3,
) -> bool {
    ViewportSceneQuery::new(project).point_visible(camera, point)
}

impl crate::AppState {
    /// Rectangle in NDC coordinates. Selection changes are non-destructive and
    /// preserve the active domain; depth is bypassed only by X-Ray/Wireframe.
    pub fn select_viewport_box(&mut self, a: [f32; 2], b: [f32; 2], add: bool, subtract: bool) {
        if a.iter().chain(&b).any(|v| !v.is_finite()) {
            return;
        }
        self.select_viewport_region(
            |p| {
                p[0] >= a[0].min(b[0])
                    && p[0] <= a[0].max(b[0])
                    && p[1] >= a[1].min(b[1])
                    && p[1] <= a[1].max(b[1])
            },
            |from, to| segment_box_hit_t(from, to, a, b),
            add,
            subtract,
        );
    }

    /// Seleciona por polígono de tela em NDC; oclusão segue as mesmas regras
    /// da seleção por caixa (X-Ray e Wireframe atravessam a malha).
    pub fn select_viewport_lasso(&mut self, polygon: &[[f32; 2]], add: bool, subtract: bool) {
        if polygon.len() < 3
            || polygon.len() > 4096
            || polygon.iter().flatten().any(|value| !value.is_finite())
        {
            return;
        }
        self.select_viewport_region(
            |p| point_in_polygon_ndc(p, polygon),
            |from, to| segment_polygon_hit_t(from, to, polygon),
            add,
            subtract,
        );
    }

    fn select_viewport_region(
        &mut self,
        contains: impl Fn([f32; 2]) -> bool,
        segment_hit_t: impl Fn([f32; 2], [f32; 2]) -> Option<f32>,
        add: bool,
        subtract: bool,
    ) {
        if self.modal.is_some() || self.mesh_preview.is_some() || self.paint_stroke.is_some() {
            return;
        }
        let vp = self.session.camera.view_proj();
        let scene = ViewportSceneQuery::new(&self.project.project);
        let project = |point: Vec3| {
            let clip = vp * point.extend(1.0);
            if !clip.is_finite() || clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w {
                return None;
            }
            let p = clip.truncate() / clip.w;
            Some([p.x, p.y])
        };
        let through = self.session.show_xray || self.session.shading == crate::Shading::Wireframe;
        let inside = |point: Vec3| {
            project(point).is_some_and(|p| {
                contains(p) && (through || scene.point_visible(&self.session.camera, point))
            })
        };
        let edge_inside = |a: Vec3, b: Vec3| {
            let (Some(pa), Some(pb)) = (project(a), project(b)) else {
                return false;
            };
            if !through
                && glam::Vec2::from_array(pa).distance_squared(glam::Vec2::from_array(pb))
                    <= 1.0e-12
            {
                return false;
            }
            if inside(a) || inside(b) {
                return true;
            }
            segment_hit_t(pa, pb)
                .is_some_and(|t| through || scene.point_visible(&self.session.camera, a.lerp(b, t)))
        };
        if self.selection_domain() == crate::SelectionDomain::Object {
            let hits: Vec<_> = self
                .project
                .assets
                .iter()
                .filter(|asset| asset.visible && !asset.locked)
                .filter(|asset| {
                    let mesh = asset.evaluated_mesh_ref();
                    mesh.verts.iter().any(|v| inside(v.vec()))
                        || mesh.edges_unique().into_iter().any(|(a, b)| {
                            edge_inside(mesh.verts[a as usize].vec(), mesh.verts[b as usize].vec())
                        })
                })
                .map(|asset| asset.id)
                .collect();
            if !add && !subtract {
                self.session.selection.assets.clear();
            }
            for id in hits {
                if subtract {
                    self.session
                        .selection
                        .assets
                        .retain(|selected| *selected != id);
                } else if !self.session.selection.assets.contains(&id) {
                    self.session.selection.assets.push(id);
                }
            }
            self.project.active = self
                .session
                .selection
                .assets
                .last()
                .and_then(|id| self.project.assets.iter().position(|asset| asset.id == *id))
                .unwrap_or(usize::MAX);
        } else {
            let Some(asset) = self
                .project
                .active()
                .filter(|asset| asset.visible && !asset.locked)
            else {
                return;
            };
            let mesh = &asset.mesh;
            let domain = self.selection_domain();
            let vertices: Vec<_> = if domain == crate::SelectionDomain::Vertex {
                mesh.verts
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| inside(v.vec()))
                    .map(|(i, _)| i)
                    .collect()
            } else {
                Vec::new()
            };
            let edges: Vec<_> = if domain == crate::SelectionDomain::Edge {
                mesh.edges_unique()
                    .into_iter()
                    .filter(|&(a, b)| {
                        edge_inside(mesh.verts[a as usize].vec(), mesh.verts[b as usize].vec())
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let faces: Vec<_> = if domain == crate::SelectionDomain::Face {
                mesh.faces
                    .iter()
                    .enumerate()
                    .filter(|&(index, face)| {
                        if face.verts.len() < 3 {
                            return false;
                        }
                        let center = face
                            .verts
                            .iter()
                            .map(|&v| mesh.verts[v as usize].vec())
                            .sum::<Vec3>()
                            / face.verts.len() as f32;
                        let ndc = self.session.camera.project_ndc(center);
                        let (_, direction) = self.session.camera.ray(ndc.x, ndc.y);
                        if !through && mesh.face_normal(index).dot(direction).abs() <= 1.0e-6 {
                            return false;
                        }
                        let visible = |point: Vec3| {
                            through
                                || scene
                                    .point_visible(&self.session.camera, point.lerp(center, 0.001))
                        };
                        inside(center)
                            || face.verts.iter().any(|&v| {
                                let point = mesh.verts[v as usize].vec();
                                project(point).is_some_and(|p| contains(p) && visible(point))
                            })
                            || face.verts.iter().enumerate().any(|(i, &a)| {
                                let b = face.verts[(i + 1) % face.verts.len()];
                                let a = mesh.verts[a as usize].vec();
                                let b = mesh.verts[b as usize].vec();
                                let (Some(pa), Some(pb)) = (project(a), project(b)) else {
                                    return false;
                                };
                                segment_hit_t(pa, pb).is_some_and(|t| visible(a.lerp(b, t)))
                            })
                    })
                    .map(|(i, _)| i)
                    .collect()
            } else {
                Vec::new()
            };
            let Some(mesh) = self.project.active_mesh_mut() else {
                return;
            };
            if !add && !subtract {
                mesh.deselect_all();
            }
            for i in vertices {
                mesh.verts[i].selected = !subtract;
            }
            for edge in edges {
                if subtract {
                    mesh.selected_edges.remove(&edge);
                } else {
                    mesh.selected_edges.insert(edge);
                }
            }
            for i in faces {
                mesh.faces[i].selected = !subtract;
            }
            if domain == crate::SelectionDomain::Face {
                mesh.sync_vert_selection_from_faces();
            }
            if domain == crate::SelectionDomain::Edge {
                for v in &mut mesh.verts {
                    v.selected = false;
                }
                for &(a, b) in &mesh.selected_edges {
                    mesh.verts[a as usize].selected = true;
                    mesh.verts[b as usize].selected = true;
                }
            }
        }
        self.session.tools.hover = crate::HoverTarget::None;
        self.sync_selection();
        self.mark_dirty();
    }
}

fn point_in_polygon_ndc(point: [f32; 2], polygon: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        if (current[1] > point[1]) != (previous[1] > point[1]) {
            let crossing = current[0]
                + (point[1] - current[1]) * (previous[0] - current[0]) / (previous[1] - current[1]);
            if point[0] < crossing {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn segment_box_hit_t(from: [f32; 2], to: [f32; 2], a: [f32; 2], b: [f32; 2]) -> Option<f32> {
    let min = [a[0].min(b[0]), a[1].min(b[1])];
    let max = [a[0].max(b[0]), a[1].max(b[1])];
    let corners = [min, [max[0], min[1]], max, [min[0], max[1]]];
    segment_polygon_hit_t(from, to, &corners)
}

fn segment_polygon_hit_t(from: [f32; 2], to: [f32; 2], polygon: &[[f32; 2]]) -> Option<f32> {
    polygon
        .iter()
        .enumerate()
        .filter_map(|(i, &a)| segment_intersection_t(from, to, a, polygon[(i + 1) % polygon.len()]))
        .min_by(f32::total_cmp)
}

fn segment_intersection_t(from: [f32; 2], to: [f32; 2], a: [f32; 2], b: [f32; 2]) -> Option<f32> {
    let cross = |u: [f32; 2], v: [f32; 2]| u[0] * v[1] - u[1] * v[0];
    let direction = [to[0] - from[0], to[1] - from[1]];
    let boundary = [b[0] - a[0], b[1] - a[1]];
    let denominator = cross(direction, boundary);
    if denominator.abs() <= 1.0e-8 {
        return None;
    }
    let offset = [a[0] - from[0], a[1] - from[1]];
    let t = cross(offset, boundary) / denominator;
    let u = cross(offset, direction) / denominator;
    (0.0..=1.0)
        .contains(&t)
        .then_some(t)
        .filter(|_| (0.0..=1.0).contains(&u))
}

#[cfg(test)]
mod lasso_tests {
    use super::{point_in_polygon_ndc, segment_box_hit_t, segment_polygon_hit_t};

    fn cube_state(domain: crate::SelectionDomain, xray: bool, wireframe: bool) -> crate::AppState {
        let mut state = crate::AppState::default();
        state.project.assets = vec![petunia_project::Asset::new(
            "cube",
            petunia_mesh::Mesh::cube(2.0),
        )];
        state.project.active = 0;
        state.session.camera.set_preset(crate::ViewPreset::Front);
        state.session.camera.aspect = 1.0;
        state.session.camera.ortho_half_h = 2.0;
        state.set_selection_domain(domain);
        state.project.active_mesh_mut().unwrap().deselect_all();
        state.session.show_xray = xray;
        state.session.shading = if wireframe {
            crate::Shading::Wireframe
        } else {
            crate::Shading::Solid
        };
        state
    }

    fn select_region(state: &mut crate::AppState, lasso: bool, a: [f32; 2], b: [f32; 2]) {
        if lasso {
            state.select_viewport_lasso(&[a, [b[0], a[1]], b, [a[0], b[1]]], false, false);
        } else {
            state.select_viewport_box(a, b, false, false);
        }
    }

    #[test]
    fn front_ortho_cube_box_lasso_select_only_visible_components_without_through_selection() {
        use crate::SelectionDomain::{Edge, Face, Vertex};
        for lasso in [false, true] {
            for (xray, wireframe) in [(false, false), (true, false), (false, true)] {
                let through = xray || wireframe;
                for domain in [Face, Vertex, Edge] {
                    let mut state = cube_state(domain, xray, wireframe);
                    select_region(&mut state, lasso, [-0.6, -0.6], [0.6, 0.6]);
                    let mesh = state.project.active_mesh().unwrap();
                    let context =
                        format!("{domain:?}, lasso={lasso}, xray={xray}, wireframe={wireframe}");
                    match domain {
                        Face => {
                            for face in &mesh.faces {
                                assert_eq!(
                                    face.selected,
                                    through
                                        || face
                                            .verts
                                            .iter()
                                            .all(|&v| mesh.verts[v as usize].pos[2] == 1.0),
                                    "{context}"
                                );
                            }
                        }
                        Vertex => {
                            for vertex in &mesh.verts {
                                assert_eq!(
                                    vertex.selected,
                                    through || vertex.pos[2] == 1.0,
                                    "{context}"
                                );
                            }
                        }
                        Edge => {
                            for (a, b) in mesh.edges_unique() {
                                assert_eq!(
                                    mesh.selected_edges.contains(&(a, b)),
                                    through
                                        || (mesh.verts[a as usize].pos[2] == 1.0
                                            && mesh.verts[b as usize].pos[2] == 1.0),
                                    "{context}: {a}-{b}"
                                );
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    #[test]
    fn front_ortho_cube_box_lasso_border_crossing_selects_front_face_and_edge_only() {
        use crate::SelectionDomain::{Edge, Face, Vertex};
        for lasso in [false, true] {
            for domain in [Face, Vertex, Edge] {
                let mut state = cube_state(domain, false, false);
                select_region(&mut state, lasso, [-0.2, 0.49], [0.2, 0.51]);
                let mesh = state.project.active_mesh().unwrap();
                match domain {
                    Face => {
                        let selected: Vec<_> =
                            mesh.faces.iter().filter(|face| face.selected).collect();
                        assert_eq!(selected.len(), 1, "lasso={lasso}");
                        assert!(
                            selected[0]
                                .verts
                                .iter()
                                .all(|&v| mesh.verts[v as usize].pos[2] == 1.0)
                        );
                    }
                    Vertex => assert!(mesh.verts.iter().all(|vertex| !vertex.selected)),
                    Edge => {
                        assert_eq!(mesh.selected_edges.len(), 1, "lasso={lasso}");
                        assert!(
                            mesh.selected_edges
                                .iter()
                                .all(|&(a, b)| mesh.verts[a as usize].pos[2] == 1.0
                                    && mesh.verts[b as usize].pos[2] == 1.0)
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn concave_lasso_does_not_select_its_bounding_box() {
        let polygon = [
            [-0.8, -0.8],
            [0.8, -0.8],
            [0.8, -0.2],
            [-0.2, -0.2],
            [-0.2, 0.8],
            [-0.8, 0.8],
        ];
        assert!(point_in_polygon_ndc([-0.5, 0.5], &polygon));
        assert!(point_in_polygon_ndc([0.5, -0.5], &polygon));
        assert!(!point_in_polygon_ndc([0.5, 0.5], &polygon));
    }

    #[test]
    fn box_detects_an_edge_crossing_without_an_endpoint_inside() {
        let hit = segment_box_hit_t([-0.8, 0.0], [0.8, 0.0], [-0.2, -0.2], [0.2, 0.2]);
        assert!(hit.is_some_and(|t| (0.0..1.0).contains(&t)));
        assert_eq!(
            segment_box_hit_t([-0.8, 0.7], [0.8, 0.7], [-0.2, -0.2], [0.2, 0.2],),
            None
        );
    }

    #[test]
    fn concave_lasso_detects_only_crossings_of_its_actual_boundary() {
        let polygon = [
            [-0.8, -0.8],
            [0.8, -0.8],
            [0.8, -0.2],
            [-0.2, -0.2],
            [-0.2, 0.8],
            [-0.8, 0.8],
        ];
        assert!(segment_polygon_hit_t([-0.6, 0.5], [0.6, 0.5], &polygon).is_some());
        assert!(segment_polygon_hit_t([0.3, 0.3], [0.7, 0.7], &polygon).is_none());
    }
}
