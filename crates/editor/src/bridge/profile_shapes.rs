//! Formas paramétricas do perfil DRAW (retângulo, círculo, dimensões).

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn replace_profile_shape(
        &mut self,
        name: &str,
        points: Vec<[f64; 3]>,
        primitive: petunia_project::profile::ProfilePrimitive,
    ) -> bool {
        if points.len() < 3 || points.iter().flatten().any(|value| !value.is_finite()) {
            return false;
        }
        if let Some((profile, spline)) = self.active_profile_resources()
            && !spline.closed
        {
            let mut profile = profile.clone();
            profile.primitive = Some(primitive);
            let mut spline = spline.clone();
            spline.points = points
                .into_iter()
                .map(petunia_core::SplinePoint::new)
                .collect();
            spline.closed = true;
            spline.interpolation = petunia_core::SplineInterpolation::Polyline;
            if let Err(error) = self
                .state
                .dispatch(&draw_shapes::TransformProfileCmd { spline, profile })
            {
                self.state.set_status(error.to_string());
                return false;
            }
        } else {
            let spline = petunia_core::SplineResource::from_polyline(name, &points, true);
            let mut profile =
                petunia_core::ProfileResource::new(name, spline.id, self.draft_profile_workplane());
            profile.wall_thickness = f64::from(self.state.profile.wall_thickness.max(0.0));
            profile.primitive = Some(primitive);
            let profile_id = profile.id;
            if let Err(error) = self
                .state
                .dispatch(&petunia_core::CreateProfileCmd { spline, profile })
            {
                self.state.set_status(error.to_string());
                return false;
            }
            self.state.select_object(None, false);
            self.state.session.active_shape = Some(profile_id);
        }
        self.state.session.planar_selection.point = None;
        self.profile_drag_target = None;
        self.profile_edit_gesture = None;
        self.state.session.tools.active_tool = "draw_profile".to_string();
        true
    }

    pub fn profile_dimension_gesture(&mut self, phase: i32) {
        if phase == 0 {
            if self.profile_volume_mode.is_none() && self.profile_transform_gesture.is_none() {
                self.begin_profile_authoring_preview();
            }
        } else if phase == 1 {
            self.commit_profile_authoring_preview();
        } else {
            self.cancel_profile_authoring_preview();
        }
    }

    pub fn set_profile_dimensions(&mut self, width: f32, height: f32, segments: usize) -> bool {
        if self.profile_transform_gesture.is_some() || self.profile_volume_mode.is_some() {
            return false;
        }
        let Some((profile, spline)) = self.active_profile_resources() else {
            return false;
        };
        let Some(next) =
            profile.resize_primitive(spline, f64::from(width), f64::from(height), segments)
        else {
            return false;
        };
        let profile = profile.clone();
        if next.points.len() == spline.points.len()
            && next.points.iter().zip(&spline.points).all(|(a, b)| {
                let a = glam::DVec3::from_array(a.position);
                let b = glam::DVec3::from_array(b.position);
                a.distance(b) <= 1e-7 * (1.0 + b.length())
            })
        {
            return false;
        }
        if let Some(draft) = self.profile_authoring_preview.as_mut() {
            draft.spline = next;
            if matches!(self.tool_gesture, Some(ToolGesture::PlanarPrimitive { .. })) {
                self.planar_primitive_numeric = true;
            }
            self.refresh_profile_authoring_preview();
            self.state.mark_dirty();
            return true;
        }
        let accepted = self
            .state
            .dispatch(&draw_shapes::TransformProfileCmd {
                profile,
                spline: next,
            })
            .is_ok();
        if accepted {
            self.state.session.planar_selection.point = None;
            self.state.render.mark_dirty();
        }
        accepted
    }

    pub fn add_profile_rectangle(&mut self, width: f32, height: f32) -> bool {
        self.cancel_profile_volume();
        if !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
            || width > 100_000.0
            || height > 100_000.0
        {
            return false;
        }
        let half_width = f64::from(width) * 0.5;
        let half_height = f64::from(height) * 0.5;
        if !self.replace_profile_shape(
            "Rectangle Profile",
            vec![
                [-half_width, -half_height, 0.0],
                [half_width, -half_height, 0.0],
                [half_width, half_height, 0.0],
                [-half_width, half_height, 0.0],
            ],
            petunia_project::profile::ProfilePrimitive::Rectangle,
        ) {
            return false;
        }
        self.state.render.mark_dirty();
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_RECTANGLE_PROFILE_X_CREATED),
            &[
                ("width", format!("{width:.1}")),
                ("height", format!("{height:.1}")),
            ],
        ));
        true
    }

    pub fn add_profile_circle(&mut self, radius: f32, segments: usize) -> bool {
        self.cancel_profile_volume();
        if !radius.is_finite()
            || radius <= 0.0
            || radius > 100_000.0
            || !(6..=512).contains(&segments)
        {
            return false;
        }
        let points = (0..segments)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / segments as f64;
                [
                    f64::from(radius) * angle.cos(),
                    f64::from(radius) * angle.sin(),
                    0.0,
                ]
            })
            .collect();
        if !self.replace_profile_shape(
            "Circle Profile",
            points,
            petunia_project::profile::ProfilePrimitive::Ellipse,
        ) {
            return false;
        }
        self.state.render.mark_dirty();
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_CIRCLE_PROFILE_RADIUS_SEG_CREATED),
            &[
                ("radius", format!("{radius:.1}")),
                ("segments", format!("{segments}")),
            ],
        ));
        true
    }
}
