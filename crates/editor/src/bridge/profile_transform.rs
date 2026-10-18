//! Transformação do perfil DRAW.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn profile_transform_target_at(&self, x: f32, y: f32) -> bool {
        self.draw_shapes_visible()
            && self.profile_volume_mode.is_none()
            && self.state.modal.is_none()
            && self.tool_modal.is_none()
            && matches!(
                self.state.session.tools.active_tool.as_str(),
                "move" | "transform" | "rotate" | "scale"
            )
            && self.state.session.active_shape.is_some()
            && self.profile_hit_at([x, y]) == self.state.session.active_shape
    }

    pub(crate) fn begin_profile_transform(
        &mut self,
        kind: TransformKind,
        anchor: [f32; 2],
    ) -> bool {
        if self.profile_volume_mode.is_some() {
            return false;
        }
        let Some((profile, spline)) = self.active_contour_resources() else {
            return false;
        };
        let count = spline.points.len();
        if count == 0 || profile.locked {
            return false;
        }
        // Point move só o nó; Curve, os dois nós do segmento; Shape, a forma toda.
        let selected: std::collections::HashSet<_> =
            if let Some(id) = self.state.session.planar_selection.point {
                [id].into_iter().collect()
            } else if let Some(segment) = self.state.session.planar_selection.segment
                && segment < spline.segment_count()
            {
                let next = (segment + 1) % count;
                [spline.points.get(segment), spline.points.get(next)]
                    .into_iter()
                    .flatten()
                    .map(|p| p.id)
                    .collect()
            } else {
                std::collections::HashSet::new()
            };
        let mut cx = 0.0f64;
        let mut cy = 0.0f64;
        let mut used = 0usize;
        for point in &spline.points {
            if selected.is_empty() || selected.contains(&point.id) {
                cx += point.position[0];
                cy += point.position[1];
                used += 1;
            }
        }
        if used == 0 {
            return false;
        }
        let gesture = ProfileTransformGesture {
            project_before: self.state.project.project.clone(),
            selected_points: selected,
            spline: spline.clone(),
            profile: profile.clone(),
            kind,
            anchor,
            center: [cx / used as f64, cy / used as f64],
            constraint: petunia_core::ModalConstraint::Free,
            components: glam::Vec3::ZERO,
        };
        if !self.begin_profile_authoring_preview() {
            return false;
        }
        self.profile_transform_gesture = Some(gesture);
        self.tool_gesture = Some(ToolGesture::Transform { gizmo: false });
        self.tool_session.clear_numeric();
        true
    }

    pub(crate) fn profile_transform_delta(
        &self,
        gesture: &ProfileTransformGesture,
        current: [f32; 2],
        snap: bool,
    ) -> Option<(glam::Vec2, f32)> {
        let [width, height] = self.viewport_size;
        let to_plane = |pixel: [f32; 2]| {
            self.profile_screen_to_plane(
                pixel[0] / width * 2.0 - 1.0,
                1.0 - pixel[1] / height * 2.0,
            )
        };
        let start = to_plane(gesture.anchor)?;
        let current = to_plane(current)?;
        match gesture.kind {
            TransformKind::Position => {
                let mut delta = glam::Vec2::new(
                    (current[0] - start[0]) as f32,
                    (current[1] - start[1]) as f32,
                );
                if snap {
                    let step = self.state.session.snap_settings.grid_spacing.max(0.001);
                    delta = (delta / step).round() * step;
                }
                Some((delta, delta.length()))
            }
            TransformKind::Rotation => {
                let center = glam::Vec2::from_array(gesture.center.map(|value| value as f32));
                let angle = |point: [f64; 2]| {
                    let vector = glam::Vec2::new(point[0] as f32, point[1] as f32) - center;
                    vector.y.atan2(vector.x).to_degrees()
                };
                let mut degrees = (angle(current) - angle(start) + 180.0).rem_euclid(360.0) - 180.0;
                if snap {
                    degrees = (degrees / 15.0).round() * 15.0;
                }
                Some((glam::Vec2::splat(degrees.to_radians()), degrees))
            }
            TransformKind::Scale => {
                let center = glam::Vec2::from_array(gesture.center.map(|value| value as f32));
                let start = glam::Vec2::new(start[0] as f32, start[1] as f32) - center;
                let current = glam::Vec2::new(current[0] as f32, current[1] as f32) - center;
                let radius = gesture
                    .spline
                    .points
                    .iter()
                    .map(|point| {
                        glam::Vec2::new(point.position[0] as f32, point.position[1] as f32)
                            .distance(center)
                    })
                    .fold(0.001_f32, f32::max);
                let factor = if start.length() < radius * 0.01 {
                    1.0 + (current.x - start.x) / radius
                } else {
                    current.length() / start.length()
                }
                .clamp(0.01, 100.0);
                Some((glam::Vec2::splat(factor), factor))
            }
        }
    }

    pub(crate) fn update_profile_transform(&mut self, current: [f32; 2], snap: bool) -> bool {
        let Some(source) = self.profile_transform_gesture.as_ref() else {
            return false;
        };
        let kind = source.kind;
        let Some((delta, scalar)) = self.profile_transform_delta(source, current, snap) else {
            return false;
        };
        // Sem clonar o gesto (ele carrega o projeto de antes) nem restaurar o
        // projeto inteiro por evento: só o perfil e o spline mudam no gesto.
        let Some(gesture) = self.profile_transform_gesture.as_ref() else {
            return false;
        };
        let components = match kind {
            TransformKind::Position => glam::Vec3::new(delta.x, delta.y, 0.0),
            TransformKind::Rotation => glam::Vec3::new(scalar, 0.0, 0.0),
            TransformKind::Scale => glam::Vec3::new(delta.x, delta.y, 1.0),
        };
        let center = glam::DVec2::new(gesture.center[0], gesture.center[1]);
        let radians = if kind == TransformKind::Rotation {
            delta.x as f64
        } else {
            0.0
        };
        let (sin, cos) = radians.sin_cos();
        let factor = if kind == TransformKind::Scale {
            delta.x as f64
        } else {
            1.0
        };
        let mut spline = gesture.spline.clone();
        let mut profile = gesture.profile.clone();
        let apply_point =
            |position: &mut [f64; 3], handle_in: &mut [f64; 3], handle_out: &mut [f64; 3]| {
                let mut local = glam::DVec2::new(position[0], position[1]) - center;
                match kind {
                    TransformKind::Position => {
                        let mut shift = glam::DVec2::new(delta.x as f64, delta.y as f64);
                        if let Some(index) = match gesture.constraint {
                            petunia_core::ModalConstraint::Axis(index) => Some(index),
                            _ => None,
                        } {
                            if index == 0 {
                                shift.y = 0.0;
                            } else {
                                shift.x = 0.0;
                            }
                        }
                        local += shift;
                    }
                    TransformKind::Rotation => {
                        local = glam::DVec2::new(
                            local.x * cos - local.y * sin,
                            local.x * sin + local.y * cos,
                        );
                        for handle in [&mut *handle_in, &mut *handle_out] {
                            let x = handle[0] * cos - handle[1] * sin;
                            let y = handle[0] * sin + handle[1] * cos;
                            handle[0] = x;
                            handle[1] = y;
                        }
                    }
                    TransformKind::Scale => {
                        local *= factor;
                        handle_in[0] *= factor;
                        handle_in[1] *= factor;
                        handle_out[0] *= factor;
                        handle_out[1] *= factor;
                    }
                }
                position[0] = center.x + local.x;
                position[1] = center.y + local.y;
                position[2] = 0.0;
            };
        for point in &mut spline.points {
            if !gesture.selected_points.is_empty() && !gesture.selected_points.contains(&point.id) {
                continue;
            }
            apply_point(
                &mut point.position,
                &mut point.handle_in,
                &mut point.handle_out,
            );
        }
        for hole in &mut profile.holes {
            for point in hole {
                let mut position = [point[0], point[1], 0.0];
                let mut handle_in = [0.0; 3];
                let mut handle_out = [0.0; 3];
                apply_point(&mut position, &mut handle_in, &mut handle_out);
                *point = [position[0], position[1]];
            }
        }
        let mut contour_shape = profile.clone();
        contour_shape.spline_id = spline.id;
        if contour_shape.validate_authoring(&spline).is_err() {
            return false;
        }
        let Some(draft) = self.profile_authoring_preview.as_mut() else {
            return false;
        };
        let is_outer = draft.spline.id == spline.id;
        if is_outer {
            draft.spline = spline;
        } else if let Some(contour) = draft.inner.iter_mut().find(|s| s.id == spline.id) {
            *contour = spline;
        }
        draft.profile = profile;
        // Each update starts from the fixed authoring source, including holes.
        for contour in &mut draft.inner {
            if !is_outer || !gesture.selected_points.is_empty() {
                continue;
            }
            if let Some(source) = gesture.project_before.get_spline(contour.id) {
                *contour = source.clone();
            }
            for point in &mut contour.points {
                apply_point(
                    &mut point.position,
                    &mut point.handle_in,
                    &mut point.handle_out,
                );
            }
        }
        if let Some(gesture) = self.profile_transform_gesture.as_mut() {
            gesture.components = components;
        }
        self.refresh_profile_authoring_preview();
        self.state.mark_dirty();
        true
    }

    pub(crate) fn commit_profile_transform(&mut self) -> bool {
        if self.profile_transform_gesture.take().is_none() {
            return false;
        }
        self.tool_session.clear_numeric();
        self.commit_profile_authoring_preview()
    }

    pub(crate) fn cancel_profile_transform(&mut self) -> bool {
        if self.profile_transform_gesture.take().is_none() {
            return false;
        }
        self.tool_session.clear_numeric();
        self.cancel_profile_authoring_preview()
    }
}
