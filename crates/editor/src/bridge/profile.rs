//! Perfil DRAW: workplane, pontos, hit-test e ponteiro.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn active_profile_resources(
        &self,
    ) -> Option<(
        &petunia_core::ProfileResource,
        &petunia_core::SplineResource,
    )> {
        if let Some(draft) = &self.profile_authoring_preview {
            return Some((&draft.profile, &draft.spline));
        }
        let profile = self
            .state
            .session
            .active_shape
            .and_then(|id| self.state.project.project.get_profile(id))?;
        let spline = self.state.project.project.get_spline(profile.spline_id)?;
        Some((profile, spline))
    }

    pub(crate) fn active_profile_state(&self) -> Option<petunia_core::ProfileState> {
        let (profile, spline) = self.active_contour_resources()?;
        let mut state = self.state.profile.clone();
        state.points = spline
            .points
            .iter()
            .map(|point| [point.position[0] as f32, point.position[1] as f32])
            .collect();
        state.nodes = spline
            .points
            .iter()
            .map(|point| {
                let handle_in = [point.handle_in[0] as f32, point.handle_in[1] as f32];
                let handle_out = [point.handle_out[0] as f32, point.handle_out[1] as f32];
                petunia_mesh::curve::BezierNode {
                    point: [point.position[0] as f32, point.position[1] as f32],
                    handle_in: (handle_in != [0.0; 2]).then_some(handle_in),
                    handle_out: (handle_out != [0.0; 2]).then_some(handle_out),
                    kind: match point.handle_mode {
                        petunia_core::SplineHandleMode::Broken => {
                            petunia_mesh::curve::BezierNodeKind::Sharp
                        }
                        petunia_core::SplineHandleMode::Aligned => {
                            petunia_mesh::curve::BezierNodeKind::Smooth
                        }
                        petunia_core::SplineHandleMode::Mirrored => {
                            petunia_mesh::curve::BezierNodeKind::Symmetric
                        }
                    },
                }
            })
            .collect();
        state.origin = profile.workplane.origin.map(|value| value as f32);
        state.right = profile.workplane.right.map(|value| value as f32);
        state.up = profile.workplane.up.map(|value| value as f32);
        state.normal = profile.workplane.normal.map(|value| value as f32);
        state.closed = spline.closed;
        state.wall_thickness = 0.0;
        state.holes = self
            .state
            .project
            .project
            .shape_hole_contours(profile, 0.005)
            .ok()?;
        Some(state)
    }

    pub(crate) fn active_profile_point_count(&self) -> usize {
        self.active_profile_resources()
            .map_or(0, |(_, spline)| spline.points.len())
    }

    pub(crate) fn active_profile_closed(&self) -> bool {
        self.active_profile_resources()
            .is_some_and(|(_, spline)| spline.closed)
    }

    pub(crate) fn active_profile_has_curves(&self) -> bool {
        if let Some(id) = self.state.session.planar_selection.point {
            return self
                .active_contour_resources()
                .is_some_and(|(_, spline)| spline.point_is_curved(id));
        }
        self.active_contour_resources().is_some_and(|(_, spline)| {
            spline.interpolation == petunia_core::SplineInterpolation::CubicBezier
                && spline
                    .points
                    .iter()
                    .any(|point| point.handle_in != [0.0; 3] || point.handle_out != [0.0; 3])
        })
    }

    /// Nome traduzido do plano de trabalho atual (Chão/Face/Vista).
    pub(crate) fn workplane_display_name(&self) -> String {
        use petunia_config::text_id as t;
        let id = match self.state.profile.workplane_kind {
            petunia_core::WorkplaneKind::Ground => t::UI_PROFILE_PLANE_GROUND,
            petunia_core::WorkplaneKind::Face => t::UI_PROFILE_PLANE_FACE,
            petunia_core::WorkplaneKind::View => t::UI_PROFILE_PLANE_VIEW,
        };
        self.state.t_id(id)
    }

    /// Plano automático no 1º clique de um perfil novo: a face sob o cursor
    /// (se houver) ou o plano do mundo mais paralelo à vista atual. A câmera
    /// não se move. Não faz nada com o plano travado ou com perfil em curso.
    pub(crate) fn resolve_auto_workplane_at(&mut self, ndc_x: f32, ndc_y: f32) {
        if self.state.session.active_workplane.locked || self.active_profile_resources().is_some() {
            return;
        }
        let normalized = [(ndc_x + 1.0) * 0.5, (1.0 - ndc_y) * 0.5];
        let face = match self.pick_target_for_domain(
            SelectionDomain::Face,
            normalized[0],
            normalized[1],
        ) {
            petunia_core::HoverTarget::Face(face) => Some(face),
            _ => None,
        };
        let captured = face.is_some_and(|face| {
            petunia_module_model::profile_capture_face_index(&mut self.state, face)
        });
        if !captured {
            petunia_module_model::profile_capture_world_plane_for_view(&mut self.state);
        }
        let plane = petunia_project::Workplane {
            origin: self.state.profile.origin.map(f64::from),
            right: self.state.profile.right.map(f64::from),
            up: self.state.profile.up.map(f64::from),
            normal: self.state.profile.normal.map(f64::from),
        };
        let _ = self
            .state
            .session
            .active_workplane
            .set(petunia_core::WorkplaneMode::Auto, plane);
    }

    pub(crate) fn draft_profile_workplane(&self) -> petunia_core::ProfileWorkplane {
        self.state.session.active_workplane.plane
    }

    pub(crate) fn profile_screen_to_plane(&self, ndc_x: f32, ndc_y: f32) -> Option<[f64; 2]> {
        self.profile_screen_to_plane_snapped(ndc_x, ndc_y)
            .map(|(point, _)| point)
    }

    /// Ponto no plano do perfil e o alvo de snap que o produziu, se houver.
    pub(crate) fn profile_screen_to_plane_snapped(
        &self,
        ndc_x: f32,
        ndc_y: f32,
    ) -> Option<([f64; 2], Option<petunia_core::ScreenSnapHit>)> {
        let workplane = self.active_profile_resources().map_or_else(
            || self.draft_profile_workplane(),
            |(profile, _)| profile.workplane,
        );
        let (ray_origin, ray_direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let normal = glam::Vec3::from_array(workplane.normal.map(|value| value as f32));
        let denominator = ray_direction.dot(normal);
        if denominator.abs() < 1.0e-6 {
            return None;
        }
        let origin = glam::Vec3::from_array(workplane.origin.map(|value| value as f32));
        let distance = (origin - ray_origin).dot(normal) / denominator;
        if distance < 0.0 && self.state.session.camera.proj == petunia_core::Projection::Perspective
        {
            return None;
        }
        let right = glam::Vec3::from_array(workplane.right.map(|value| value as f32));
        let up = glam::Vec3::from_array(workplane.up.map(|value| value as f32));
        let mut hit = ray_origin + ray_direction * distance;
        let mut snap_hit = None;
        if self.state.session.snap_settings.enabled {
            // Mesma passada de snap das transformações (P3D-040): pontos e
            // arestas da malha, guias paralelas aos eixos do plano a partir do
            // último ponto e, por último, a grade do plano de trabalho.
            let to_world = |p: [f64; 3]| origin + right * p[0] as f32 + up * p[1] as f32;
            let anchor = self
                .active_profile_resources()
                .and_then(|(_, spline)| spline.points.last())
                .map(|last| petunia_core::SnapAnchor {
                    point: to_world(last.position),
                    axes: [Some(right), Some(up), None],
                });
            let settings = &self.state.session.snap_settings;
            let cursor = glam::Vec2::new(ndc_x + 1.0, 1.0 - ndc_y)
                * glam::Vec2::from_array(self.viewport_size)
                * 0.5;
            let grid = petunia_core::SnapGrid {
                origin,
                right,
                up,
                spacing: settings.grid_spacing,
            };
            let mut mask = petunia_core::SnapMask::for_target(settings.target);
            mask.grid = matches!(
                settings.target,
                petunia_core::SnapTarget::Smart | petunia_core::SnapTarget::Grid
            );
            // Guias paralela/perpendicular ao último segmento e ângulos de 15°
            // a partir do último ponto (inferência do desenho, capítulo 46).
            let reference = self.active_profile_resources().and_then(|(_, spline)| {
                let [.., before, last] = spline.points.as_slice() else {
                    return None;
                };
                let direction = to_world(last.position) - to_world(before.position);
                (direction.length_squared() > 1.0e-10).then_some(direction)
            });
            if let Some(snapped) = self.screen_snap_guided(
                cursor,
                mask,
                anchor,
                Some(grid),
                reference,
                Some(self.state.session.snap_settings.angle_step_degrees),
            ) {
                hit = snapped.point;
                snap_hit = Some(snapped);
            }
        }
        let settings = &self.state.session.snap_settings;
        if settings.enabled
            && settings.target == petunia_core::SnapTarget::Increment
            && settings.grid_spacing.is_finite()
            && settings.grid_spacing > 0.0
        {
            let anchor = self
                .active_contour_resources()
                .and_then(|(_, s)| s.points.last())
                .map_or(origin, |last| {
                    origin + right * last.position[0] as f32 + up * last.position[1] as f32
                });
            let offset = hit - anchor;
            let step = settings.grid_spacing;
            hit = anchor
                + right * (offset.dot(right) / step).round() * step
                + up * (offset.dot(up) / step).round() * step;
            snap_hit = Some(petunia_core::ScreenSnapHit {
                point: hit,
                kind: petunia_core::SnapKind::Increment,
                pixel_distance: 0.0,
                reference: None,
            });
        }
        if let Some((anchor, direction)) = self.draw_inference_guide {
            hit = anchor + direction * (hit - anchor).dot(direction);
            snap_hit = Some(petunia_core::ScreenSnapHit {
                point: hit,
                kind: petunia_core::SnapKind::Parallel,
                reference: None,
                pixel_distance: 0.0,
            });
        }
        // Pontos fora do plano (arestas de outra face) são projetados nele.
        let offset = hit - origin;
        let point = [f64::from(offset.dot(right)), f64::from(offset.dot(up))];
        // O marcador mostra o ponto que realmente entra no perfil (projetado).
        let snap_hit = snap_hit.map(|snapped| {
            let projected = origin + right * point[0] as f32 + up * point[1] as f32;
            petunia_core::ScreenSnapHit {
                point: projected,
                reference: if projected.distance_squared(snapped.point) <= 1.0e-10 {
                    snapped.reference
                } else {
                    None
                },
                ..snapped
            }
        });
        Some((point, snap_hit))
    }

    pub(crate) fn update_profile_handle(
        &mut self,
        point_id: uuid::Uuid,
        incoming: bool,
        cursor: [f64; 2],
        break_tangent: bool,
        new_node_drag: bool,
    ) -> bool {
        let Some(spline_id) = self
            .profile_edit_gesture
            .as_ref()
            .map(|gesture| gesture.spline_id)
        else {
            return false;
        };
        if !self.begin_profile_authoring_preview() {
            return false;
        }
        let Some(spline) = self.authoring_contour_mut(spline_id) else {
            return false;
        };
        let Some(point) = spline.point(point_id).cloned() else {
            return false;
        };
        let handle = [
            cursor[0] - point.position[0],
            cursor[1] - point.position[1],
            0.0,
        ];
        if handle[0].hypot(handle[1]) <= 0.02 {
            return false;
        }
        let mode = if break_tangent {
            petunia_core::SplineHandleMode::Broken
        } else if new_node_drag {
            petunia_core::SplineHandleMode::Mirrored
        } else {
            point.handle_mode
        };
        let (mut handle_in, mut handle_out) = (point.handle_in, point.handle_out);
        if incoming {
            handle_in = handle;
            match mode {
                petunia_core::SplineHandleMode::Broken => {}
                petunia_core::SplineHandleMode::Mirrored => {
                    handle_out = handle.map(|component| -component);
                }
                petunia_core::SplineHandleMode::Aligned => {
                    let length = glam::DVec3::from_array(handle_out)
                        .length()
                        .max(glam::DVec3::from_array(handle).length());
                    handle_out =
                        (-glam::DVec3::from_array(handle).normalize_or_zero() * length).to_array();
                }
            }
        } else {
            handle_out = handle;
            match mode {
                petunia_core::SplineHandleMode::Broken => {}
                petunia_core::SplineHandleMode::Mirrored => {
                    handle_in = handle.map(|component| -component);
                }
                petunia_core::SplineHandleMode::Aligned => {
                    let length = glam::DVec3::from_array(handle_in)
                        .length()
                        .max(glam::DVec3::from_array(handle).length());
                    handle_in =
                        (-glam::DVec3::from_array(handle).normalize_or_zero() * length).to_array();
                }
            }
        }
        let revision = spline.revision;
        if spline
            .set_handles(point_id, handle_in, handle_out, mode)
            .is_err()
        {
            return false;
        }
        spline.revision = revision;
        spline.interpolation = petunia_core::SplineInterpolation::CubicBezier;
        true
    }

    pub(crate) fn add_profile_point(&mut self, ndc_x: f32, ndc_y: f32) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        self.resolve_auto_workplane_at(ndc_x, ndc_y);
        let Some(position) = self.profile_screen_to_plane(ndc_x, ndc_y) else {
            return false;
        };
        if self.active_profile_closed() {
            return false;
        }
        if self.active_profile_point_count()
            >= petunia_module_model::draw_profile::MAX_PROFILE_POINTS
        {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_PROFILE_LIMIT_REACHED_4096_POINTS),
            );
            return false;
        }

        let position = if self.draw_parameters.constrain {
            self.active_profile_resources()
                .and_then(|(_, s)| s.points.last())
                .map_or(position, |last| {
                    let angle = self.draw_parameters.angle.to_radians();
                    [
                        last.position[0] + self.draw_parameters.length * angle.cos(),
                        last.position[1] + self.draw_parameters.length * angle.sin(),
                    ]
                })
        } else {
            position
        };
        let point = petunia_core::SplinePoint::new([position[0], position[1], 0.0]);
        let point_id = point.id;
        if self.active_profile_resources().is_some() {
            if !self.begin_profile_authoring_preview() {
                return false;
            }
        } else {
            let spline = petunia_core::SplineResource::new(
                "Curve",
                petunia_core::SplineInterpolation::CubicBezier,
            );
            let profile = petunia_project::PlanarShape::new(
                "Shape",
                spline.id,
                self.draft_profile_workplane(),
            );
            self.state.session.active_shape = Some(profile.id);
            self.profile_authoring_preview =
                Some(crate::bridge::profile_draft::ProfileAuthoringPreview {
                    profile,
                    spline,
                    inner: Vec::new(),
                    source: None,
                    evaluation_project: Box::new(self.state.project.project.clone()),
                    previous_shape: None,
                });
        }
        let Some(draft) = self.profile_authoring_preview.as_mut() else {
            return false;
        };
        let spline_id = draft.spline.id;
        if draft.spline.add_point(point).is_err() {
            return false;
        }
        self.state.session.planar_selection.point = Some(point_id);
        self.profile_drag_target = Some(ProfileHitTarget::HandleOut(point_id));
        self.profile_edit_gesture = Some(ProfileEditGesture {
            spline_id,
            point_id,
        });
        true
    }

    pub(crate) fn profile_preview_commands(&self) -> String {
        let Some(profile) = self.active_profile_state() else {
            return String::new();
        };
        let points = profile.tessellated_points();
        if points.is_empty() || self.viewport_size[0] <= 1.0 || self.viewport_size[1] <= 1.0 {
            return String::new();
        }
        let matrix = self.state.session.camera.view_proj();
        let project_pt = |p: [f32; 2]| -> Option<[f32; 2]> {
            let pt_3d = profile.to_3d_point(p);
            let point = matrix * pt_3d.extend(1.0);
            if !point.is_finite() || point.w <= 0.05 || point.z < 0.0 || point.z > point.w {
                None
            } else {
                Some([
                    (point.x / point.w * 0.5 + 0.5) * self.viewport_size[0],
                    (0.5 - point.y / point.w * 0.5) * self.viewport_size[1],
                ])
            }
        };

        let mut commands = String::new();
        use std::fmt::Write as _;

        // Loop externo
        let mut first = true;
        for &pt in &points {
            if let Some(screen_pt) = project_pt(pt) {
                let action = if first {
                    first = false;
                    'M'
                } else {
                    'L'
                };
                let _ = write!(
                    commands,
                    "{action} {:.2} {:.2} ",
                    screen_pt[0], screen_pt[1]
                );
            }
        }

        if profile.closed {
            commands.push_str("Z ");

            // Se for oco (wall_thickness > 0), desenha também o laço interno no preview
            if profile.wall_thickness > 0.0 && points.len() >= 3 {
                let inner = profile.inner_points();
                let mut first_inner = true;
                for &pt in &inner {
                    if let Some(screen_pt) = project_pt(pt) {
                        let action = if first_inner {
                            first_inner = false;
                            'M'
                        } else {
                            'L'
                        };
                        let _ = write!(
                            commands,
                            "{action} {:.2} {:.2} ",
                            screen_pt[0], screen_pt[1]
                        );
                    }
                }
                if !first_inner {
                    commands.push_str("Z ");
                }
            }
        } else {
            let [x, y] = self.pointer_position;
            let _ = write!(commands, "L {:.2} {:.2} ", x, y);
        }

        // Alças tangentes e marcadores de nós Bézier
        let Some((_, spline)) = self.active_profile_resources() else {
            return String::new();
        };
        for (node, spline_point) in profile.nodes.iter().zip(&spline.points) {
            let p_anchor = project_pt(node.point);
            let is_selected = self.state.session.planar_selection.point == Some(spline_point.id);

            if let (Some(anchor_scr), Some(h_out)) = (p_anchor, node.handle_out) {
                let out_pt = [node.point[0] + h_out[0], node.point[1] + h_out[1]];
                if let Some(out_scr) = project_pt(out_pt) {
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} ",
                        anchor_scr[0], anchor_scr[1], out_scr[0], out_scr[1]
                    );
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        out_scr[0] - 2.5,
                        out_scr[1] - 2.5,
                        out_scr[0] + 2.5,
                        out_scr[1] - 2.5,
                        out_scr[0] + 2.5,
                        out_scr[1] + 2.5,
                        out_scr[0] - 2.5,
                        out_scr[1] + 2.5
                    );
                }
            }
            if let (Some(anchor_scr), Some(h_in)) = (p_anchor, node.handle_in) {
                let in_pt = [node.point[0] + h_in[0], node.point[1] + h_in[1]];
                if let Some(in_scr) = project_pt(in_pt) {
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} ",
                        anchor_scr[0], anchor_scr[1], in_scr[0], in_scr[1]
                    );
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        in_scr[0] - 2.5,
                        in_scr[1] - 2.5,
                        in_scr[0] + 2.5,
                        in_scr[1] - 2.5,
                        in_scr[0] + 2.5,
                        in_scr[1] + 2.5,
                        in_scr[0] - 2.5,
                        in_scr[1] + 2.5
                    );
                }
            }

            // Marcador do ponto âncora (destacado se selecionado)
            if let Some(anchor_scr) = p_anchor {
                let s = if is_selected { 5.0 } else { 3.0 };
                let _ = write!(
                    commands,
                    "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                    anchor_scr[0] - s,
                    anchor_scr[1] - s,
                    anchor_scr[0] + s,
                    anchor_scr[1] - s,
                    anchor_scr[0] + s,
                    anchor_scr[1] + s,
                    anchor_scr[0] - s,
                    anchor_scr[1] + s
                );
            }
        }

        commands
    }

    pub fn close_profile(&mut self) -> bool {
        if self.active_profile_closed() {
            return false;
        }
        let Some((_, spline)) = self.active_profile_resources() else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_PROFILE_REQUIRES_AT_LEAST_THREE_POINTS),
            );
            return false;
        };
        if spline.points.len() < 3 {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_PROFILE_REQUIRES_AT_LEAST_THREE_POINTS),
            );
            return false;
        }
        if !self.close_profile_with_mirror() {
            return false;
        }
        self.state.set_status(self.state.t_id(
            petunia_config::text_id::STATUS_PROFILE_CLOSED_CHOOSE_GENERATE_VOLUME_OR_REVOLVE,
        ));
        true
    }

    pub fn set_profile_smoothness(&mut self, smoothness: f32) -> bool {
        if !smoothness.is_finite()
            || smoothness <= 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        petunia_module_model::profile_set_curve_smoothness(&mut self.state, smoothness);
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn profile_smooth_curves(&mut self) -> bool {
        // Com um ponto selecionado, curvar só ele (Shape → ponto).
        if let Some(id) = self.state.session.planar_selection.point {
            return self.profile_set_point_curved(Some(id), true);
        }
        let Some((_, spline)) = self.active_contour_resources() else {
            return false;
        };
        let mut updated = spline.clone();
        let mut path = petunia_mesh::curve::BezierPath {
            nodes: self
                .active_profile_state()
                .map_or_else(Vec::new, |profile| profile.nodes),
            closed: spline.closed,
        };
        path.auto_smooth(0.25);
        updated.interpolation = petunia_core::SplineInterpolation::CubicBezier;
        for (point, node) in updated.points.iter_mut().zip(path.nodes) {
            let handle_in = node.handle_in.unwrap_or([0.0; 2]);
            let handle_out = node.handle_out.unwrap_or([0.0; 2]);
            point.handle_in = [f64::from(handle_in[0]), f64::from(handle_in[1]), 0.0];
            point.handle_out = [f64::from(handle_out[0]), f64::from(handle_out[1]), 0.0];
            point.handle_mode = petunia_core::SplineHandleMode::Aligned;
        }
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: updated })
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_PROFILE_CURVES_SMOOTHED_CUBIC_BEZIER),
        );
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn profile_clear_curves(&mut self) -> bool {
        if let Some(id) = self.state.session.planar_selection.point {
            return self.profile_set_point_curved(Some(id), false);
        }
        let Some((_, spline)) = self.active_contour_resources() else {
            return false;
        };
        let mut updated = spline.clone();
        updated.interpolation = petunia_core::SplineInterpolation::Polyline;
        for point in &mut updated.points {
            point.handle_in = [0.0; 3];
            point.handle_out = [0.0; 3];
            point.handle_mode = petunia_core::SplineHandleMode::Broken;
        }
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: updated })
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_PROFILE_CORNERS_SHARPENED),
        );
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn profile_update_drag_handle(
        &mut self,
        norm_x: f32,
        norm_y: f32,
        break_tangent: bool,
    ) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        let Some(gesture) = self.profile_edit_gesture.as_ref() else {
            return false;
        };
        let point_id = gesture.point_id;
        let ndc_x = norm_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - norm_y.clamp(0.0, 1.0) * 2.0;
        let Some(point) = self.profile_screen_to_plane(ndc_x, ndc_y) else {
            return false;
        };
        let updated = self.update_profile_handle(point_id, false, point, break_tangent, true);
        if updated {
            self.state.mark_dirty();
            if self.profile_volume_mode.is_some() {
                self.update_profile_volume_preview();
            }
        }
        updated
    }

    /// Realiza hit-testing em screen space contra âncoras e alças Bézier do perfil ativo.
    pub fn hit_test_profile(&self, screen_x: f32, screen_y: f32) -> Option<ProfileHitTarget> {
        if self.state.session.tools.active_tool != "draw_profile"
            && !["points", "segments"].contains(&self.draw_component_context.as_str())
        {
            return None;
        }
        let profile = self.active_profile_state()?;
        let (_, spline) = self.active_contour_resources()?;
        if profile.points.is_empty() || self.viewport_size[0] <= 1.0 || self.viewport_size[1] <= 1.0
        {
            return None;
        }
        let matrix = self.state.session.camera.view_proj();
        let project_pt = |p: [f32; 2]| -> Option<[f32; 2]> {
            let pt_3d = profile.to_3d_point(p);
            let point = matrix * pt_3d.extend(1.0);
            if !point.is_finite() || point.w <= 0.05 || point.z < 0.0 || point.z > point.w {
                None
            } else {
                Some([
                    (point.x / point.w * 0.5 + 0.5) * self.viewport_size[0],
                    (0.5 - point.y / point.w * 0.5) * self.viewport_size[1],
                ])
            }
        };

        let hit_radius = 16.0f32;
        let mut best_target = None;
        let mut best_dist = hit_radius;

        for (node, spline_point) in profile.nodes.iter().zip(&spline.points) {
            // Test handle_out
            if let Some(h_out) = node.handle_out {
                let out_pt = [node.point[0] + h_out[0], node.point[1] + h_out[1]];
                if let Some(scr) = project_pt(out_pt) {
                    let dist = (scr[0] - screen_x).hypot(scr[1] - screen_y);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some(ProfileHitTarget::HandleOut(spline_point.id));
                    }
                }
            }
            // Test handle_in
            if let Some(h_in) = node.handle_in {
                let in_pt = [node.point[0] + h_in[0], node.point[1] + h_in[1]];
                if let Some(scr) = project_pt(in_pt) {
                    let dist = (scr[0] - screen_x).hypot(scr[1] - screen_y);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some(ProfileHitTarget::HandleIn(spline_point.id));
                    }
                }
            }
            // Test anchor point
            if let Some(scr) = project_pt(node.point) {
                let dist = (scr[0] - screen_x).hypot(scr[1] - screen_y);
                if dist < best_dist {
                    best_dist = dist;
                    best_target = Some(ProfileHitTarget::Anchor(spline_point.id));
                }
            }
        }

        best_target
    }

    /// Processa clique inicial do ponteiro para selecionar e iniciar arraste de nós/alças do perfil.
    pub fn profile_pointer_down(&mut self, screen_x: f32, screen_y: f32, alt: bool) -> bool {
        self.profile_pointer_down_ex(screen_x, screen_y, alt, false)
    }

    /// Como [`Self::profile_pointer_down`]; `ctrl` força um ponto novo mesmo
    /// sobre a aresta ou o contorno de uma forma existente.
    pub fn profile_pointer_down_ex(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        alt: bool,
        ctrl: bool,
    ) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        if let Some(target) = self.hit_test_profile(screen_x, screen_y) {
            let point_id = match target {
                ProfileHitTarget::Anchor(id)
                | ProfileHitTarget::HandleOut(id)
                | ProfileHitTarget::HandleIn(id) => id,
            };
            let Some((spline_id, spline_closed, point_count, point_index)) =
                self.active_contour_resources().and_then(|(_, spline)| {
                    let point_index = spline
                        .points
                        .iter()
                        .position(|point| point.id == point_id)?;
                    Some((spline.id, spline.closed, spline.points.len(), point_index))
                })
            else {
                return false;
            };
            match target {
                ProfileHitTarget::Anchor(id) => {
                    // Fecha perfil se clicar no primeiro nó de um perfil aberto com >= 3 pontos
                    if !spline_closed && point_count >= 3 && point_index == 0 {
                        self.state.session.planar_selection.point = Some(id);
                        self.profile_drag_target = None;
                        return self.close_profile();
                    }
                    // Duplo clique num nó alterna reto ↔ curva.
                    // (o clique é registrado no pointer-up, só se não houve arraste)
                    let double = self
                        .profile_last_anchor_click
                        .is_some_and(|(last, at)| last == id && at.elapsed().as_millis() < 400);
                    if double {
                        self.profile_last_anchor_click = None;
                        self.state.session.planar_selection.point = Some(id);
                        let curved = self
                            .active_contour_resources()
                            .is_some_and(|(_, spline)| spline.point_is_curved(id));
                        self.profile_drag_target = None;
                        self.profile_edit_gesture = None;
                        self.profile_set_point_curved(Some(id), !curved);
                        return true;
                    }
                    self.state.session.planar_selection.point = Some(id);
                    if alt {
                        self.profile_drag_target = Some(ProfileHitTarget::HandleOut(id));
                    } else {
                        self.profile_drag_target = Some(ProfileHitTarget::Anchor(id));
                    }
                }
                ProfileHitTarget::HandleOut(id) => {
                    self.state.session.planar_selection.point = Some(id);
                    self.profile_drag_target = Some(ProfileHitTarget::HandleOut(id));
                }
                ProfileHitTarget::HandleIn(id) => {
                    self.state.session.planar_selection.point = Some(id);
                    self.profile_drag_target = Some(ProfileHitTarget::HandleIn(id));
                }
            }
            self.profile_edit_gesture = Some(ProfileEditGesture {
                spline_id,
                point_id,
            });
            self.state.mark_dirty();
            return true;
        }
        self.profile_drag_target = None;
        self.state.session.planar_selection.point = None;
        self.profile_edit_gesture = None;
        if !ctrl {
            let pixel = [screen_x, screen_y];
            // Aresta do perfil fechado: insere um nó sem mudar a forma.
            if self.active_profile_closed()
                && let Some(hit) = self.profile_segment_hit(pixel)
                && self.insert_profile_node(hit).is_some()
            {
                self.state.mark_dirty();
                return true;
            }
            // Contorno de outra forma: ela passa a ser a forma em edição
            // (só enquanto não há um perfil aberto sendo desenhado).
            let drawing = self
                .active_contour_resources()
                .is_some_and(|(_, spline)| !spline.closed);
            if !drawing
                && let Some(id) = self.profile_hit_at(pixel)
                && self.activate_profile(id)
            {
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_SHAPE_SELECTED),
                );
                return true;
            }
        }
        self.state.mark_dirty();
        false
    }

    /// Processa movimento do ponteiro durante o arraste de um nó ou alça.
    pub fn profile_pointer_move(&mut self, screen_x: f32, screen_y: f32, alt: bool) -> bool {
        let Some(target) = self.profile_drag_target else {
            return false;
        };
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        let nx = (screen_x / width).clamp(0.0, 1.0);
        let ny = (screen_y / height).clamp(0.0, 1.0);
        let ndc_x = nx * 2.0 - 1.0;
        let ndc_y = 1.0 - ny * 2.0;

        let Some(point) = self.profile_screen_to_plane(ndc_x, ndc_y) else {
            return false;
        };

        match target {
            ProfileHitTarget::Anchor(point_id) => {
                let Some(gesture) = self.profile_edit_gesture.as_ref() else {
                    return false;
                };
                let spline_id = gesture.spline_id;
                if !self.begin_profile_authoring_preview() {
                    return false;
                }
                let Some(spline) = self.authoring_contour_mut(spline_id) else {
                    return false;
                };
                let revision = spline.revision;
                if spline
                    .move_point(point_id, [point[0], point[1], 0.0])
                    .is_err()
                {
                    return false;
                }
                spline.revision = revision;
            }
            ProfileHitTarget::HandleOut(point_id) => {
                if !self.update_profile_handle(point_id, false, point, alt, false) {
                    return false;
                }
            }
            ProfileHitTarget::HandleIn(point_id) => {
                if !self.update_profile_handle(point_id, true, point, alt, false) {
                    return false;
                }
            }
        }
        self.refresh_profile_authoring_preview();
        self.state.mark_dirty();
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    /// Alvo do botão principal no Draw Profile: alça de profundidade
    /// (`Handle(3)`), nó/alça do perfil (`Handle(2)`) ou o viewport.
    pub(crate) fn profile_press_target(&self, x: f32, y: f32) -> petunia_core::PressTarget {
        let (_, depth_handle, _) = self.depth_handle_commands();
        if let Some([hx, hy]) = depth_handle
            && (x - hx).abs() <= 20.0
            && (y - hy).abs() <= 20.0
        {
            return petunia_core::PressTarget::Handle(3);
        }
        if self.hit_test_profile(x, y).is_some() {
            return petunia_core::PressTarget::Handle(2);
        }
        petunia_core::PressTarget::Surface
    }

    /// Clique sem arrasto no Draw Profile: nó (seleciona; duplo clique
    /// alterna reto ↔ curva; o primeiro nó fecha), aresta/contorno (insere nó
    /// ou ativa a forma) ou vazio (adiciona ponto / seleciona).
    pub(crate) fn draw_profile_click(&mut self, at: [f32; 2]) -> bool {
        if self.profile_pointer_down_ex(
            at[0],
            at[1],
            self.tool_pointer_alt,
            self.tool_press_alternate,
        ) {
            if self.profile_edit_gesture.is_some() {
                self.profile_pointer_up();
            }
            self.state.mark_dirty();
            return true;
        }
        let [width, height] = self.viewport_size;
        if width > 1.0 && height > 1.0 {
            self.select_viewport_ext(at[0] / width, at[1] / height, false, false);
        }
        // O nó novo nasce na prévia; sem arrasto de alça, o clique o confirma.
        if self.profile_edit_gesture.is_some() {
            self.profile_pointer_up();
        }
        true
    }

    /// Início de arrasto no Draw Profile (o mesmo roteamento do clique).
    pub(crate) fn begin_draw_profile_gesture(
        &mut self,
        anchor: [f32; 2],
        target: petunia_core::PressTarget,
    ) -> bool {
        let depth = self.state.profile.depth;
        let depth_gesture = ToolGesture::ProfileDepth {
            start: depth,
            anchor_y: anchor[1],
        };
        if target == petunia_core::PressTarget::Handle(3) {
            if self.profile_volume_mode.is_none() && !self.enter_profile_volume("extrude") {
                return false;
            }
            self.tool_gesture = Some(depth_gesture);
            return true;
        }
        if self.profile_pointer_down_ex(
            anchor[0],
            anchor[1],
            self.tool_pointer_alt,
            self.tool_press_alternate,
        ) {
            if self.profile_edit_gesture.is_none() {
                return false;
            }
            self.tool_gesture = Some(ToolGesture::ProfilePoint);
            return true;
        }
        let gesture = match self.profile_volume_mode {
            Some(petunia_module_model::ProfileVolumeMode::Extrude) => depth_gesture,
            Some(petunia_module_model::ProfileVolumeMode::Revolve) => ToolGesture::ProfileRevolve {
                start: if self.state.profile.revolve_angle <= 0.0 {
                    360.0
                } else {
                    self.state.profile.revolve_angle
                },
                anchor_x: anchor[0],
            },
            Some(petunia_module_model::ProfileVolumeMode::Sweep) => return false,
            None if self.active_profile_closed() => return false,
            None => {
                let [width, height] = self.viewport_size;
                if width <= 1.0 || height <= 1.0 {
                    return false;
                }
                self.select_viewport_ext(anchor[0] / width, anchor[1] / height, false, false);
                ToolGesture::ProfileHandlePull
            }
        };
        self.tool_gesture = Some(gesture);
        true
    }

    /// Esc durante o arrasto de um nó/alça: o ponto volta exatamente ao estado
    /// de antes do gesto, sem entrada de Undo.
    pub(crate) fn cancel_profile_point_drag(&mut self) -> bool {
        self.profile_drag_target = None;
        let had = self.profile_edit_gesture.take().is_some();
        self.cancel_profile_authoring_preview() || had
    }

    pub fn profile_pointer_up(&mut self) {
        let target = self.profile_drag_target.take();
        let Some(gesture) = self.profile_edit_gesture.take() else {
            return;
        };
        if matches!(target, Some(ProfileHitTarget::Anchor(_))) {
            self.profile_last_anchor_click = Some((gesture.point_id, std::time::Instant::now()));
        }
        if self.profile_authoring_preview.is_some() {
            self.commit_profile_authoring_preview();
        }
    }

    pub(crate) fn delete_selected_profile_point(&mut self) -> bool {
        self.profile_pointer_up();
        let Some(point_id) = self.state.session.planar_selection.point else {
            return false;
        };
        let Some((_, spline)) = self.active_contour_resources() else {
            return false;
        };
        let spline_id = spline.id;
        if let Err(error) = self.state.dispatch(&petunia_core::DeleteSplinePointCmd {
            spline_id,
            point_id,
        }) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.session.planar_selection.point = None;
        self.profile_drag_target = None;
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }
}
