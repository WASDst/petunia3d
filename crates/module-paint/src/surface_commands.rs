//! Comandos transacionais de pintura de superfície; domínio sem toolkit.
use crate::{
    PaintModule,
    projection::{DepthBuffer, ProjectionParams},
};
use glam::Vec3;
use petunia_core::{AppState, BrushSettings, Command, CommandError};
use petunia_project::{Canvas, ProjectChanges, SplineResource, SurfaceAttachment};

fn clear_stroke(state: &mut AppState) {
    state.session.tools.paint_stroke = None;
    state.session.tools.paint_buffer = None;
    state.session.tools.paint_restriction = None;
    state.session.tools.paint_lock_face = None;
}
#[derive(Clone, Default)]
pub enum PathPaintMode {
    #[default]
    Stroke,
    Ribbon,
    RepeatedStamp(Canvas),
}

#[derive(Clone)]
pub struct PaintPathCmd {
    pub anchors: Vec<SurfaceAttachment>,
    pub settings: BrushSettings,
    pub name: String,
    pub mode: PathPaintMode,
}
impl Command for PaintPathCmd {
    fn label(&self) -> &'static str {
        "paint surface path"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TEXTURES | ProjectChanges::SPLINES
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let asset = state.project.active().ok_or("No active asset")?;
        if asset.locked || self.anchors.len() < 2 || self.anchors.len() > 8192 {
            return Err("Path needs an unlocked surface and two points");
        }
        if self.anchors.iter().any(|a| a.target != asset.id) {
            return Err("The path belongs to another surface");
        }
        Ok(())
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let frames =
            petunia_project::evaluate_surface_attachments(&state.project.project, &self.anchors)
                .map_err(|e| CommandError::Execution(e.to_string()))?;
        let nodes: Vec<_> = frames
            .into_iter()
            .map(|f| Vec3::from_array(f.position))
            .collect();
        state.begin_paint_stroke();
        let count = match &self.mode {
            PathPaintMode::Stroke => {
                PaintModule::paint_surface_path(state, &nodes, false, false, self.settings, false)
            }
            PathPaintMode::Ribbon => {
                let original = state.session.tools.brush_style;
                state.session.tools.brush_style.tip = petunia_core::BrushTip::Square;
                let mut settings = self.settings;
                settings.hardness = 1.0;
                settings.spacing = 0.1;
                let count =
                    PaintModule::paint_surface_path(state, &nodes, false, false, settings, false);
                state.session.tools.brush_style = original;
                count
            }
            PathPaintMode::RepeatedStamp(image) => {
                paint_stamps(state, &nodes, image, self.settings, &self.name)
            }
        };
        if count == 0 {
            clear_stroke(state);
            return Err(CommandError::Execution("No eligible paint pixels".into()));
        }
        let points: Vec<_> = nodes.iter().map(|n| n.to_array().map(f64::from)).collect();
        let mut spline = SplineResource::from_polyline(&self.name, &points, false);
        for (point, anchor) in spline.points.iter_mut().zip(&self.anchors) {
            point.attachment = Some(*anchor);
        }
        let result = state
            .project
            .project
            .add_spline(spline)
            .map_err(|e| CommandError::Execution(e.to_string()));
        clear_stroke(state);
        result.map(|_| ())
    }
}
#[derive(Clone)]
pub struct ProjectionPaintCmd {
    pub image: Canvas,
    pub params: ProjectionParams,
    pub scene_triangles: Vec<[Vec3; 3]>,
    pub first_face: Option<usize>,
    pub name: String,
}
impl Command for ProjectionPaintCmd {
    fn label(&self) -> &'static str {
        "project image on surface"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TEXTURES
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let asset = state.project.active().ok_or("No active asset")?;
        if asset.locked {
            Err("Surface is locked")
        } else {
            Ok(())
        }
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mesh = state
            .project
            .active_mesh()
            .cloned()
            .ok_or(CommandError::NoActiveAsset)?;
        let depth = DepthBuffer::build(
            self.params.view_proj,
            self.params.viewport[0],
            self.params.viewport[1],
            self.scene_triangles.iter().copied(),
        );
        state.begin_paint_stroke();
        PaintModule::ensure_stack(state);
        let (w, h) = state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .map_or((256, 256), |c| (c.w, c.h));
        if let Some(stack) = state
            .project
            .active_mut()
            .and_then(|a| a.paint_stack.as_mut())
        {
            stack.add_layer(petunia_project::PaintLayer::new(&self.name, w, h, [0; 4]));
        }
        let mut changed = false;
        PaintModule::restricted_edit(state, self.first_face, |canvas| {
            changed = !crate::projection::project_image_onto_mesh(
                canvas,
                &mesh,
                &|_| true,
                &depth,
                &self.image,
                &self.params,
            )
            .is_empty();
        });
        changed &= state
            .project
            .active()
            .and_then(|a| a.paint_stack.as_ref())
            .and_then(|s| s.active())
            .and_then(|l| l.canvas())
            .is_some_and(|canvas| canvas.pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 0));
        clear_stroke(state);
        if !changed {
            return Err(CommandError::Execution("No eligible paint pixels".into()));
        }
        PaintModule::composite_active(state);
        Ok(())
    }
}

fn paint_stamps(
    state: &mut AppState,
    nodes: &[Vec3],
    image: &Canvas,
    settings: BrushSettings,
    name: &str,
) -> usize {
    let Some(mesh) = state.project.active_mesh().cloned() else {
        return 0;
    };
    let radius = nodes
        .iter()
        .map(|n| state.world_radius_for_px(*n, settings.size_px))
        .fold(f32::INFINITY, f32::min);
    if !radius.is_finite() || radius <= 0.0 {
        return 0;
    }
    let samples = crate::sample_path(
        nodes,
        false,
        false,
        (radius * 2.0 * settings.spacing.max(1.0)).max(1e-5),
        256,
    );
    let first_face = crate::project_path_onto_surface(&mesh, &samples, radius * 4.0, &|_| true)
        .first()
        .map(|h| h.face);
    let restriction = PaintModule::resolve_restriction(state, first_face);
    let hits = crate::project_path_onto_surface(&mesh, &samples, radius * 4.0, &|face| {
        restriction.as_ref().is_none_or(|r| r.allows_face(face))
    });
    PaintModule::ensure_stack(state);
    // A stamp writes raster pixels, never mutates the live decal used as its source.
    let (w, h) = state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .map_or((256, 256), |c| (c.w, c.h));
    if let Some(stack) = state
        .project
        .active_mut()
        .and_then(|a| a.paint_stack.as_mut())
    {
        stack.add_layer(petunia_project::PaintLayer::new(name, w, h, [0; 4]));
    }
    let mut touched = 0;
    PaintModule::restricted_edit(state, first_face, |canvas| {
        for (index, hit) in hits.iter().enumerate() {
            let normal = mesh.face_normal(hit.face).normalize_or_zero();
            if normal == Vec3::ZERO {
                continue;
            }
            let tangent = hits.get(index + 1).map_or_else(
                || {
                    hits.get(index.wrapping_sub(1))
                        .map_or(Vec3::X, |p| hit.position - p.position)
                },
                |p| p.position - hit.position,
            );
            let mut u = (tangent - normal * tangent.dot(normal)).normalize_or_zero();
            if u == Vec3::ZERO {
                u = normal
                    .cross(if normal.y.abs() > 0.9 {
                        Vec3::X
                    } else {
                        Vec3::Y
                    })
                    .normalize_or_zero();
            }
            let v = normal.cross(u);
            let mut owned = vec![false; (canvas.w * canvas.h) as usize];
            for face in 0..mesh.faces.len() {
                if restriction.as_ref().is_some_and(|r| !r.allows_face(face))
                    || mesh.face_normal(face).dot(normal) < 0.5
                {
                    continue;
                }
                mesh.rasterize_face_near(
                    face,
                    hit.position,
                    radius * 1.5,
                    canvas.w,
                    canvas.h,
                    1.0,
                    |x, y, _, pos, _| {
                        let i = (y * canvas.w + x) as usize;
                        if owned[i] {
                            return;
                        }
                        let d = pos - hit.position;
                        let uv = [
                            0.5 + d.dot(u) / (2.0 * radius),
                            0.5 - d.dot(v) / (2.0 * radius),
                        ];
                        if !(0.0..=1.0).contains(&uv[0]) || !(0.0..=1.0).contains(&uv[1]) {
                            return;
                        }
                        let mut src =
                            crate::projection::sample_bilinear(image, uv).map(|c| c * 255.0);
                        src[3] *= settings.strength;
                        if src[3] <= 0.0 {
                            return;
                        }
                        if let Some(dst) = canvas.get(x, y) {
                            canvas.set(x, y, PaintModule::over(dst, src));
                            owned[i] = true;
                            touched += 1;
                        }
                    },
                );
            }
        }
    });
    if state
        .project
        .active()
        .and_then(|a| a.paint_stack.as_ref())
        .and_then(|s| s.active())
        .and_then(|l| l.canvas())
        .is_none_or(|c| !c.pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 0))
    {
        touched = 0;
    }
    PaintModule::composite_active(state);
    touched
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;
    fn surface() -> AppState {
        let mut state = AppState::default();
        state.project.assets.clear();
        state.project.add("Surface", Mesh::plane(2.0));
        state
    }
    fn anchors(state: &AppState) -> Vec<SurfaceAttachment> {
        let target = state.project.active().unwrap().id;
        [-0.4, 0.4]
            .into_iter()
            .map(|x| {
                petunia_project::project_ray_to_surface_target(
                    &state.project.project,
                    target,
                    [x, 2.0, 0.0],
                    [0.0, -1.0, 0.0],
                    10.0,
                )
                .unwrap()
                .unwrap()
                .attachment
            })
            .collect()
    }
    #[test]
    fn path_paint_and_persistent_anchors_are_one_undo() {
        let mut state = surface();
        let before = state.project.undo.depth().0;
        let command = PaintPathCmd {
            anchors: anchors(&state),
            settings: BrushSettings::default(),
            name: "Path".into(),
            mode: PathPaintMode::Stroke,
        };
        state.dispatch(&command).unwrap();
        assert_eq!(state.project.undo.depth().0, before + 1);
        assert!(
            state
                .project
                .splines
                .last()
                .unwrap()
                .points
                .iter()
                .all(|p| p.attachment.is_some())
        );
        assert!(state.session.tools.paint_stroke.is_none());
        state.undo();
        assert!(state.project.splines.is_empty());
        assert!(state.project.active().unwrap().texture.is_none());
    }
    #[test]
    fn stale_attachment_rolls_back_without_history() {
        let mut state = surface();
        let command = PaintPathCmd {
            anchors: anchors(&state),
            settings: BrushSettings::default(),
            name: "Path".into(),
            mode: PathPaintMode::Stroke,
        };
        state.project.active_mesh_mut().unwrap().faces[0]
            .verts
            .reverse();
        let before = state.project.project.clone();
        let depth = state.project.undo.depth();
        assert!(state.dispatch(&command).is_err());
        assert_eq!(
            serde_json::to_value(&state.project.project).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        assert_eq!(state.project.undo.depth(), depth);
    }
    #[test]
    fn blocked_projection_does_not_leave_an_empty_layer_or_undo() {
        let mut state = surface();
        for vertex in &mut state.project.active_mesh_mut().unwrap().verts {
            vertex.pos = [vertex.pos[0] * 0.8, vertex.pos[2] * 0.8, 0.5];
        }
        state.session.tools.brush_lock = petunia_core::BrushLock::SelectedFaces;
        let triangles =
            crate::projection::mesh_world_triangles(state.project.active_mesh().unwrap()).collect();
        let before = state.project.project.clone();
        let depth = state.project.undo.depth();
        let command = ProjectionPaintCmd {
            image: Canvas::new(2, 2, [255, 0, 0, 255]),
            params: ProjectionParams {
                view_proj: glam::Mat4::IDENTITY,
                viewport: [64, 64],
                placement: crate::projection::StencilPlacement {
                    center_px: [32.0; 2],
                    size_px: [64.0; 2],
                    ..Default::default()
                },
                opacity: 1.0,
                back_face_cull: false,
                depth_bias: 1e-4,
            },
            scene_triangles: triangles,
            first_face: Some(0),
            name: "Projection".into(),
        };
        assert!(state.dispatch(&command).is_err());
        assert_eq!(
            serde_json::to_value(&state.project.project).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        assert_eq!(state.project.undo.depth(), depth);
    }
}
