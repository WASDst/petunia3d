//! Interactive planar creation using the same gesture/session lifecycle as editing.
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanarPrimitiveKind {
    Rectangle,
    Circle,
    Ellipse,
    Polygon,
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn begin_planar_primitive(
        &mut self,
        kind: PlanarPrimitiveKind,
        at: [f32; 2],
    ) -> bool {
        if self.modeling_mode != ModelingMode::Draw
            || self.profile_volume_mode.is_some()
            || self.profile_authoring_preview.is_some()
        {
            return false;
        }
        let previous = self.state.session.active_shape.take();
        let ndc = [
            at[0] / self.viewport_size[0].max(1.0) * 2.0 - 1.0,
            1.0 - at[1] / self.viewport_size[1].max(1.0) * 2.0,
        ];
        self.resolve_auto_workplane_at(ndc[0], ndc[1]);
        let Some(anchor) = self.profile_screen_to_plane(ndc[0], ndc[1]) else {
            self.state.session.active_shape = previous;
            return false;
        };
        let spline = petunia_project::SplineResource::new(
            "Shape contour",
            petunia_project::SplineInterpolation::Polyline,
        );
        let profile = petunia_project::PlanarShape::new(
            match kind {
                PlanarPrimitiveKind::Rectangle => "Rectangle",
                PlanarPrimitiveKind::Circle => "Circle",
                PlanarPrimitiveKind::Ellipse => "Ellipse",
                PlanarPrimitiveKind::Polygon => "Polygon",
            },
            spline.id,
            self.draft_profile_workplane(),
        );
        self.state.session.active_shape = Some(profile.id);
        self.state.session.planar_selection.contour = None;
        self.profile_authoring_preview =
            Some(crate::bridge::profile_draft::ProfileAuthoringPreview {
                profile,
                spline,
                inner: Vec::new(),
                source: None,
                evaluation_project: Box::new(self.state.project.project.clone()),
                previous_shape: previous,
            });
        self.planar_primitive_numeric = false;
        self.tool_gesture = Some(ToolGesture::PlanarPrimitive { kind, anchor });
        let initial = [anchor[0] + 1.0, anchor[1] + 1.0];
        self.update_planar_primitive_points(kind, anchor, initial);
        self.state.set_status(
            "Draw the shape; enter dimensions for precision. Confirm to create, Esc to cancel",
        );
        true
    }

    pub(crate) fn update_planar_primitive(
        &mut self,
        kind: PlanarPrimitiveKind,
        anchor: [f64; 2],
        at: [f32; 2],
    ) -> bool {
        if self.planar_primitive_numeric {
            return false;
        }
        let ndc = [
            at[0] / self.viewport_size[0].max(1.0) * 2.0 - 1.0,
            1.0 - at[1] / self.viewport_size[1].max(1.0) * 2.0,
        ];
        let Some(point) = self.profile_screen_to_plane(ndc[0], ndc[1]) else {
            return false;
        };
        self.update_planar_primitive_points(kind, anchor, point)
    }

    fn update_planar_primitive_points(
        &mut self,
        kind: PlanarPrimitiveKind,
        a: [f64; 2],
        b: [f64; 2],
    ) -> bool {
        let (width, height) = ((b[0] - a[0]).abs(), (b[1] - a[1]).abs());
        let radius = (b[0] - a[0]).hypot(b[1] - a[1]);
        if !radius.is_finite() || radius > 100_000.0 || radius < 1.0e-6 {
            return false;
        }
        if matches!(
            kind,
            PlanarPrimitiveKind::Rectangle | PlanarPrimitiveKind::Ellipse
        ) && (width < 1.0e-6 || height < 1.0e-6)
        {
            return false;
        }
        let points: Vec<[f64; 3]> = match kind {
            PlanarPrimitiveKind::Rectangle => vec![
                [a[0], a[1], 0.0],
                [b[0], a[1], 0.0],
                [b[0], b[1], 0.0],
                [a[0], b[1], 0.0],
            ],
            _ => {
                let sides = if kind == PlanarPrimitiveKind::Polygon {
                    self.draw_parameters.sides.clamp(3, 512)
                } else {
                    32
                };
                let (center, x, y) = if kind == PlanarPrimitiveKind::Ellipse {
                    (
                        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5],
                        width * 0.5,
                        height * 0.5,
                    )
                } else {
                    (a, radius, radius)
                };
                (0..sides)
                    .map(|i| {
                        let angle = std::f64::consts::TAU * i as f64 / sides as f64;
                        [
                            center[0] + angle.cos() * x,
                            center[1] + angle.sin() * y,
                            0.0,
                        ]
                    })
                    .collect()
            }
        };
        let Some(draft) = self.profile_authoring_preview.as_mut() else {
            return false;
        };
        let previous = &draft.spline.points;
        draft.spline.points = points
            .into_iter()
            .enumerate()
            .map(|(i, position)| {
                let mut point = petunia_project::SplinePoint::new(position);
                if let Some(old) = previous.get(i) {
                    point.id = old.id;
                }
                point
            })
            .collect();
        draft.spline.closed = true;
        draft.profile.primitive = match kind {
            PlanarPrimitiveKind::Rectangle => {
                Some(petunia_project::profile::ProfilePrimitive::Rectangle)
            }
            PlanarPrimitiveKind::Circle | PlanarPrimitiveKind::Ellipse => {
                Some(petunia_project::profile::ProfilePrimitive::Ellipse)
            }
            PlanarPrimitiveKind::Polygon => {
                Some(petunia_project::profile::ProfilePrimitive::RegularPolygon)
            }
        };
        self.state.mark_dirty();
        true
    }
}
