//! DRAW workspace policy. Native widgets only select modes or dispatch actions.
use crate::*;
use petunia_core::WorkplaneMode;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_draw_workplane(&mut self, id: &str) -> bool {
        if self.profile_volume_mode.is_some()
            || self.profile_edit_gesture.is_some()
            || self.profile_transform_gesture.is_some()
        {
            self.state.set_status(
                "Confirm or cancel the current gesture before changing construction plane",
            );
            return false;
        }
        let mode = match id {
            "auto" => WorkplaneMode::Auto,
            "ground" => WorkplaneMode::Ground,
            "view" => WorkplaneMode::View,
            "face" => WorkplaneMode::Face,
            "selection" => WorkplaneMode::Selection,
            "custom" => WorkplaneMode::Custom,
            "reference" => WorkplaneMode::Reference,
            _ => return false,
        };
        self.draw_inference_guide = None;
        self.draw_inference_toggle = false;
        let old = self.state.profile.clone();
        let captured = match mode {
            WorkplaneMode::Auto => {
                self.state.profile.workplane_locked = false;
                petunia_module_model::profile_capture_auto(&mut self.state);
                true
            }
            WorkplaneMode::Ground => {
                petunia_module_model::profile_capture_ground(&mut self.state);
                true
            }
            WorkplaneMode::View => {
                petunia_module_model::profile_capture_view(&mut self.state);
                true
            }
            WorkplaneMode::Face => petunia_module_model::profile_capture_face(&mut self.state),
            WorkplaneMode::Selection => {
                petunia_module_model::profile_capture_from_selection(&mut self.state)
            }
            WorkplaneMode::Custom => true,
            WorkplaneMode::Reference => {
                let plane = self
                    .state
                    .project
                    .refs
                    .iter()
                    .find(|r| r.visible)
                    .and_then(|r| petunia_project::Workplane::from_reference(r).ok());
                if let Some(plane) = plane {
                    self.project_workplane_compatibility(plane);
                    true
                } else {
                    false
                }
            }
        };
        if !captured {
            self.state.profile = old;
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.no_compatible_construction_plane_in_this_selection",
                )));
            return false;
        }
        let plane = petunia_project::Workplane {
            origin: self.state.profile.origin.map(f64::from),
            right: self.state.profile.right.map(f64::from),
            up: self.state.profile.up.map(f64::from),
            normal: self.state.profile.normal.map(f64::from),
        };
        if self
            .state
            .session
            .active_workplane
            .set(mode, plane)
            .is_err()
        {
            self.state.profile = old;
            return false;
        }
        self.state.profile.workplane_locked = mode != WorkplaneMode::Auto;
        self.state.session.active_shape = None;
        self.state.session.planar_selection.point = None;
        self.state.session.planar_selection.segment = None;
        self.profile_hover_snap = None;
        self.state.set_status(format!(
            "Workplane: {id}. Align View explicitly to follow this plane"
        ));
        self.state.mark_dirty();
        true
    }

    pub fn set_custom_draw_workplane(&mut self, plane: petunia_project::Workplane) -> bool {
        if self.profile_volume_mode.is_some()
            || self.profile_edit_gesture.is_some()
            || self.profile_authoring_preview.is_some()
            || self.profile_transform_gesture.is_some()
        {
            return false;
        }
        if self
            .state
            .session
            .active_workplane
            .set(WorkplaneMode::Custom, plane)
            .is_err()
        {
            return false;
        }
        self.project_workplane_compatibility(self.state.session.active_workplane.plane);
        self.state.profile.workplane_locked = true;
        self.state.session.active_shape = None;
        self.state.mark_dirty();
        true
    }

    pub(crate) fn project_workplane_compatibility(&mut self, plane: petunia_project::Workplane) {
        self.state.profile.origin = plane.origin.map(|v| v as f32);
        self.state.profile.right = plane.right.map(|v| v as f32);
        self.state.profile.up = plane.up.map(|v| v as f32);
        self.state.profile.normal = plane.normal.map(|v| v as f32);
    }

    pub(crate) fn align_draw_view(&mut self) -> bool {
        if self.state.session.planar_saved_camera.is_none() {
            self.state.session.planar_saved_camera = Some(self.state.session.camera.clone());
        }
        let plane = self.state.session.active_workplane.plane;
        if !self.state.session.camera.align_to_workplane(plane) {
            return false;
        }
        self.state.session.draw_planar_navigation = true;
        self.state.mark_dirty();
        true
    }

    pub fn set_profile_sweep_path(&mut self, id: uuid::Uuid) -> bool {
        if self.profile_volume_mode.is_some() {
            return false;
        }
        let Some(path) = self.state.project.project.get_spline(id) else {
            return false;
        };
        if self
            .active_profile_resources()
            .is_some_and(|(p, _)| p.spline_id == id || p.hole_splines.contains(&id))
            || path.points.len() < 2
            || path.validate_authoring().is_err()
        {
            return false;
        }
        self.profile_sweep_path = Some(id);
        self.state.mark_dirty();
        true
    }

    pub fn toggle_selected_shape_flag(&mut self, field: &str) -> bool {
        if self.profile_volume_mode.is_some() || self.profile_authoring_preview.is_some() {
            return false;
        }
        let Some((shape, _)) = self.active_profile_resources() else {
            return false;
        };
        if !["visible", "locked"].contains(&field) {
            return false;
        }
        let command = petunia_core::SetPlanarShapeFlagsCmd {
            shape: shape.id,
            expected_revision: shape.revision,
            visible: if field == "visible" {
                !shape.visible
            } else {
                shape.visible
            },
            locked: if field == "locked" {
                !shape.locked
            } else {
                shape.locked
            },
        };
        match self.state.dispatch(&command) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn set_draw_edit_context(&mut self, context: &str) -> bool {
        if !["shape", "points", "segments", "regions"].contains(&context)
            || self.profile_volume_mode.is_some()
        {
            return false;
        }
        self.apply(UiIntent::SetActiveTool("select".into()));
        self.draw_component_context = context.into();
        self.state.session.planar_selection.point = None;
        self.state.session.planar_selection.segment = None;
        self.region_selected = None;
        self.state.mark_dirty();
        true
    }

    pub fn select_draw_shape(&mut self, id: uuid::Uuid) -> bool {
        if self.profile_authoring_preview.is_some() {
            return false;
        }
        let activated = self.activate_profile(id);
        if activated {
            self.state.set_selection_domain(SelectionDomain::Object);
            self.state.select_object(None, false);
        }
        activated
    }

    pub fn draw_workspace_snapshot(&self) -> serde_json::Value {
        let project = &self.state.project.project;
        let shape = self.active_profile_resources();
        let live = self.profile_volume_mode.is_some();
        let mode = match self.state.session.active_workplane.mode {
            WorkplaneMode::Auto => "auto",
            WorkplaneMode::Ground => "ground",
            WorkplaneMode::Face => "face",
            WorkplaneMode::View => "view",
            WorkplaneMode::Selection => "selection",
            WorkplaneMode::Custom => "custom",
            WorkplaneMode::Reference => "reference",
        };
        let generator = self
            .generator_parameter_preview
            .as_ref()
            .map(|(_, g, _)| g)
            .or_else(|| project.active().and_then(|a| a.geometry_source.as_ref()));
        let (kind, distance, angle, segments) =
            generator.map_or(("none", 0.0, 360.0, 12), |g| match g.kind {
                petunia_project::GeometryGeneratorKind::Extrude { distance, .. } => {
                    ("extrude", distance, 360.0, 12)
                }
                petunia_project::GeometryGeneratorKind::Revolve {
                    angle, segments, ..
                } => ("revolve", 0.0, angle, segments),
                petunia_project::GeometryGeneratorKind::Sweep { .. } => ("sweep", 0.0, 360.0, 12),
                petunia_project::GeometryGeneratorKind::Primitive(_) => {
                    ("primitive", 0.0, 360.0, 12)
                }
            });
        let options = self
            .profile_generator_draft
            .as_ref()
            .map(|(g, _)| g)
            .or(generator);
        let (axis, caps) =
            options.map_or((self.draw_revolve_axis, self.draw_caps), |g| match g.kind {
                petunia_project::GeometryGeneratorKind::Revolve {
                    axis,
                    cap_start,
                    cap_end,
                    ..
                } => (axis, [cap_start, cap_end]),
                petunia_project::GeometryGeneratorKind::Extrude {
                    cap_start, cap_end, ..
                }
                | petunia_project::GeometryGeneratorKind::Sweep {
                    cap_start, cap_end, ..
                } => (self.draw_revolve_axis, [cap_start, cap_end]),
                _ => (self.draw_revolve_axis, self.draw_caps),
            });
        let reason = if live {
            "Confirm or cancel the volume preview"
        } else if shape.is_none() {
            "Select or draw a shape"
        } else if shape.is_some_and(|(p, _)| p.locked || !p.visible) {
            "Shape is locked or hidden"
        } else if !self.active_profile_closed() {
            "Close the shape explicitly to extrude"
        } else {
            ""
        };
        let mut snapshot = serde_json::json!({
            "active":self.modeling_mode == ModelingMode::Draw,
            "solid_active": self.solid_shape_builder.is_some(),
            "can_prepare_solid": self.solid_shape_builder.is_none() && !live && self.state.session.selection.assets.len() == 2,
            "solid_can_commit": self.solid_shape_builder.as_ref().is_some_and(|s| !s.selected.is_empty()),
            "solid_intent": self.solid_shape_builder.as_ref().map_or("unite", |s| match s.intent { petunia_core::SolidShapeIntent::Unite => "unite", petunia_core::SolidShapeIntent::Remove => "remove", petunia_core::SolidShapeIntent::Extract => "extract" }),
            "solid_regions": self.solid_shape_builder.as_ref().map(|session| session.regions.iter().enumerate().map(|(index, region)| {
                let key = match region.class {
                    petunia_mesh::boolean::SolidRegionClass::FirstOnly => "draw_status.first_solid_only",
                    petunia_mesh::boolean::SolidRegionClass::Intersection => "draw_status.shared_volume",
                    petunia_mesh::boolean::SolidRegionClass::SecondOnly => "draw_status.second_solid_only",
                };
                serde_json::json!({"id":region.id,"name":format!("{} · {}",index+1,self.state.t(key)),"active":session.selected.contains(&index)})
            }).collect::<Vec<_>>()).unwrap_or_default(),
            "solid_summary": self.solid_shape_builder.as_ref().map_or_else(String::new, |s| format!("{} regions · {} selected", s.regions.len(), s.selected.len())),
            "shape_available": shape.is_some(),
            "shape_name": shape.map_or("",|(p,_)|p.name.as_str()),
            "has_primitive_dimensions": shape.is_some_and(|(p,s)|p.primitive_frame(s).is_some()),
            "can_edit_shape": !live && shape.is_some_and(|(p,_)|!p.locked && p.visible),
            "edit_context": self.draw_component_context,
            "inference_locked": self.draw_inference_guide.is_some(),
            "revolve_axis": if axis == petunia_project::RevolutionAxis::Vertical { "vertical" } else { "horizontal" },
            "cap_start": caps[0], "cap_end": caps[1], "rotation_delta": 0.0, "scale_factor": 1.0,
            "origin_x": self.state.session.active_workplane.plane.origin[0],
            "origin_y": self.state.session.active_workplane.plane.origin[1],
            "origin_z": self.state.session.active_workplane.plane.origin[2],
            "angle_step": self.state.session.snap_settings.angle_step_degrees,
            "workplane_mode": mode, "workplane": self.state.session.active_workplane.plane,
        });
        let details = serde_json::json!({
            "volume_live": live, "can_extrude": !live && self.active_profile_closed() && shape.is_some_and(|(p,_)| !p.locked),
            "can_revolve": !live && self.active_profile_point_count() >= 2 && shape.is_some_and(|(p,_)|!p.locked && p.visible),
            "can_sweep": !live && self.active_profile_closed() && self.profile_sweep_path.is_some() && shape.is_some_and(|(p,_)|!p.locked && p.visible),
            "can_finish_curve": !live && self.state.session.tools.active_tool == "draw_profile" && self.active_profile_point_count() >= 2,
            "can_close": !live && !self.active_profile_closed() && self.active_profile_point_count() >= 3,
            "can_edit_dimensions": !live && shape.is_some_and(|(p,s)| !p.locked && p.primitive_frame(s).is_some()),
            "can_make_editable": generator.is_some() && project.active().is_some_and(|a| !a.locked && a.generator_diagnostic.is_none()),
            "is_extrude": kind == "extrude", "is_revolve": kind == "revolve",
            "generator_kind": kind, "generator_distance": distance, "generator_angle": angle, "generator_segments": segments,
            "diagnostic": self.profile_volume_diagnostic.clone().or_else(|| project.active().and_then(|a| a.generator_diagnostic.clone())).unwrap_or_else(|| reason.into()),
            "contours": shape.map(|(p,s)| std::iter::once(s).chain(p.hole_splines.iter().filter_map(|id| project.get_spline(*id)))
                .map(|s| serde_json::json!({"id":s.id,"name":s.name,"active":self.state.session.planar_selection.contour.unwrap_or(p.spline_id)==s.id})).collect::<Vec<_>>()).unwrap_or_default(),
            "shapes": project.profiles.iter().map(|p| serde_json::json!({"id": p.id, "name": p.name, "visible":p.visible, "locked":p.locked, "active": self.state.session.active_shape == Some(p.id)})).collect::<Vec<_>>(),
            "paths": project.splines.iter().filter(|s| s.points.len() >= 2 && !project.profiles.iter().any(|p| p.hole_splines.contains(&s.id)) && !shape.is_some_and(|(p,_)|p.spline_id == s.id))
                .map(|s| serde_json::json!({"id": s.id, "name": s.name, "active": self.profile_sweep_path == Some(s.id)})).collect::<Vec<_>>()
        });
        if let (Some(target), serde_json::Value::Object(details)) =
            (snapshot.as_object_mut(), details)
        {
            target.extend(details);
        }
        let contour = self.active_contour_resources();
        let selected = contour.and_then(|(_, s)| {
            self.state
                .session
                .planar_selection
                .point
                .and_then(|id| s.point(id))
        });
        let point = serde_json::json!({
            "can_edit_point": !live && self.profile_authoring_preview.is_none() && selected.is_some() && contour.is_some_and(|(p,_)|!p.locked && p.visible),
            "point_x": selected.map_or(0.0,|p|p.position[0]),"point_y":selected.map_or(0.0,|p|p.position[1]),
            "point_in_x":selected.map_or(0.0,|p|p.handle_in[0]),"point_in_y":selected.map_or(0.0,|p|p.handle_in[1]),
            "point_out_x":selected.map_or(0.0,|p|p.handle_out[0]),"point_out_y":selected.map_or(0.0,|p|p.handle_out[1]),
            "point_mode":selected.map_or("broken",|p|match p.handle_mode{petunia_core::SplineHandleMode::Broken=>"broken",petunia_core::SplineHandleMode::Aligned=>"aligned",petunia_core::SplineHandleMode::Mirrored=>"mirrored"}),
            "points":contour.map(|(_,s)|s.points.iter().enumerate().map(|(i,p)|serde_json::json!({"id":p.id,"name":format!("{} · ({:.3}, {:.3})",i+1,p.position[0],p.position[1]),"active":self.state.session.planar_selection.point==Some(p.id)})).collect::<Vec<_>>()).unwrap_or_default(),
        });
        if let (Some(target), serde_json::Value::Object(point)) = (snapshot.as_object_mut(), point)
        {
            target.extend(point);
        }
        snapshot
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn finish_draw_curve(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile"
            || self.profile_volume_mode.is_some()
        {
            return false;
        }
        self.profile_pointer_up();
        if self.active_profile_point_count() < 2 {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.a_curve_needs_at_least_two_points",
                )));
            return false;
        }
        self.tool_session.reset();
        self.tool_gesture = None;
        self.state.session.tools.active_tool = "select".into();
        self.state.session.planar_selection.point = None;
        self.profile_drag_target = None;
        self.state
            .set_status(self.state.t_id(petunia_config::TextId::new(
                "draw_status.curve_kept_editable_choose_a_volume_operation_explicitly",
            )));
        self.state.mark_dirty();
        true
    }

    pub fn start_new_draw_curve(&mut self) -> bool {
        self.cancel_profile_point_drag();
        self.cancel_profile_volume();
        self.state.session.active_shape = None;
        self.state.session.planar_selection = Default::default();
        self.state.session.planar_selection.contour = None;
        self.state.session.tools.active_tool = "draw_profile".into();
        self.tool_session.reset();
        self.state.mark_dirty();
        true
    }

    pub(crate) fn draw_numeric_input(&mut self, text: &str) -> bool {
        if !matches!(
            self.tool_gesture,
            Some(
                ToolGesture::PlanarPrimitive { .. }
                    | ToolGesture::ProfileDepth { .. }
                    | ToolGesture::ProfileRevolve { .. }
            )
        ) {
            return false;
        }
        if text == "Backspace" || text == "Delete" {
            self.tool_session.pop_numeric();
        } else if text.len() == 1
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+' | '*' | '/'))
        {
            self.tool_session.push_numeric(text);
        } else {
            return false;
        }
        if self.tool_session.numeric_text().is_empty() {
            self.planar_primitive_numeric = false;
            return true;
        }
        let Ok(value) = numeric::parse_numeric(self.tool_session.numeric_text()) else {
            return true;
        };
        match self.tool_gesture {
            Some(ToolGesture::ProfileDepth { .. }) => {
                self.set_profile_depth(value);
            }
            Some(ToolGesture::ProfileRevolve { .. }) => {
                self.set_profile_revolve_angle(value);
            }
            Some(ToolGesture::PlanarPrimitive { kind, .. }) if value > 0.0 => {
                let (_, height) = self
                    .active_profile_resources()
                    .and_then(|(p, s)| p.primitive_frame(s))
                    .map_or((value, value), |(_, x, y)| {
                        ((x.length() * 2.0) as f32, (y.length() * 2.0) as f32)
                    });
                let width = if kind == PlanarPrimitiveKind::Circle {
                    value * 2.0
                } else {
                    value
                };
                let height = if matches!(
                    kind,
                    PlanarPrimitiveKind::Circle | PlanarPrimitiveKind::Polygon
                ) {
                    width
                } else {
                    height
                };
                let segments = if kind == PlanarPrimitiveKind::Polygon {
                    self.draw_parameters.sides
                } else {
                    32
                };
                self.set_profile_dimensions(width, height, segments);
            }
            _ => {}
        }
        true
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_draw_angle_step(&mut self, value: f32) -> bool {
        if !value.is_finite() || !(1.0..=180.0).contains(&value) {
            return false;
        }
        self.state.session.snap_settings.angle_step_degrees = value;
        self.state.mark_dirty();
        true
    }

    pub fn set_draw_plane_component(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite() || value.abs() > 100_000.0 {
            return false;
        }
        let mut plane = self.state.session.active_workplane.plane;
        match field {
            "origin_x" => plane.origin[0] = f64::from(value),
            "origin_y" => plane.origin[1] = f64::from(value),
            "origin_z" => plane.origin[2] = f64::from(value),
            "rotate_x" | "rotate_y" | "rotate_z" => {
                let axis = match field {
                    "rotate_x" => glam::DVec3::X,
                    "rotate_y" => glam::DVec3::Y,
                    _ => glam::DVec3::Z,
                };
                let rotation = glam::DQuat::from_axis_angle(axis, f64::from(value).to_radians());
                plane.right = (rotation * glam::DVec3::from_array(plane.right)).to_array();
                plane.up = (rotation * glam::DVec3::from_array(plane.up)).to_array();
                plane.normal = (rotation * glam::DVec3::from_array(plane.normal)).to_array();
            }
            _ => return false,
        }
        self.set_custom_draw_workplane(plane)
    }

    fn capture_draw_inference(&mut self) {
        let Some((shape, spline)) = self.active_contour_resources() else {
            return;
        };
        let Some(last) = spline.points.last() else {
            return;
        };
        let plane = shape.workplane;
        let anchor = glam::Vec3::from_array(plane.origin.map(|v| v as f32))
            + glam::Vec3::from_array(plane.right.map(|v| v as f32)) * last.position[0] as f32
            + glam::Vec3::from_array(plane.up.map(|v| v as f32)) * last.position[1] as f32;
        let Some(hit) = self.profile_hover_snap else {
            return;
        };
        if !matches!(
            hit.kind,
            petunia_core::SnapKind::Axis(_)
                | petunia_core::SnapKind::Parallel
                | petunia_core::SnapKind::Perpendicular
                | petunia_core::SnapKind::Angle(_)
        ) {
            return;
        }
        let Some(direction) = (hit.point - anchor).try_normalize() else {
            return;
        };
        self.draw_inference_guide = Some((anchor, direction));
    }

    pub(crate) fn update_draw_inference_hold(&mut self, held: bool) {
        if held && !self.draw_inference_held && self.draw_inference_guide.is_none() {
            self.capture_draw_inference();
        }
        if !held && self.draw_inference_held && !self.draw_inference_toggle {
            self.draw_inference_guide = None;
        }
        self.draw_inference_held = held;
    }

    pub fn toggle_draw_inference(&mut self) -> bool {
        if self.draw_inference_toggle {
            self.draw_inference_toggle = false;
            self.draw_inference_guide = None;
        } else {
            self.capture_draw_inference();
            if self.draw_inference_guide.is_none() {
                self.state
                    .set_status(self.state.t_id(petunia_config::TextId::new(
                        "draw_status.hover_an_axis_or_angle_guide_before_locking_inference",
                    )));
                return false;
            }
            self.draw_inference_toggle = true;
        }
        self.state
            .set_status(self.state.t_id(petunia_config::TextId::new(
                if self.draw_inference_guide.is_some() {
                    "draw_status.inference_locked"
                } else {
                    "draw_status.inference_unlocked"
                },
            )));
        self.state.mark_dirty();
        true
    }

    pub fn set_draw_volume_option(&mut self, field: &str, value: &str) -> bool {
        match (field, value) {
            ("axis", "vertical") => {
                self.draw_revolve_axis = petunia_project::RevolutionAxis::Vertical
            }
            ("axis", "horizontal") => {
                self.draw_revolve_axis = petunia_project::RevolutionAxis::Horizontal
            }
            ("cap_start", "true") => self.draw_caps[0] = true,
            ("cap_start", "false") => self.draw_caps[0] = false,
            ("cap_end", "true") => self.draw_caps[1] = true,
            ("cap_end", "false") => self.draw_caps[1] = false,
            _ => return false,
        }
        if self.profile_volume_mode.is_some() {
            return self.update_profile_volume_preview();
        }
        if let Some(asset) = self.state.project.active()
            && let Some(mut generator) = asset.geometry_source.clone()
        {
            match (&mut generator.kind, field) {
                (petunia_project::GeometryGeneratorKind::Revolve { axis, .. }, "axis") => {
                    *axis = self.draw_revolve_axis
                }
                (
                    petunia_project::GeometryGeneratorKind::Extrude {
                        cap_start, cap_end, ..
                    }
                    | petunia_project::GeometryGeneratorKind::Revolve {
                        cap_start, cap_end, ..
                    }
                    | petunia_project::GeometryGeneratorKind::Sweep {
                        cap_start, cap_end, ..
                    },
                    "cap_start" | "cap_end",
                ) => {
                    if field == "cap_start" {
                        *cap_start = value == "true";
                    } else {
                        *cap_end = value == "true";
                    }
                }
                _ => {
                    self.state.mark_dirty();
                    return true;
                }
            }
            let command = petunia_core::UpdateGeometryGeneratorCmd {
                object_id: asset.id,
                expected_revision: generator.revision,
                generator,
            };
            return match self.state.dispatch(&command) {
                Ok(()) => true,
                Err(e) => {
                    self.state.set_status(e.to_string());
                    false
                }
            };
        }
        self.state.mark_dirty();
        true
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Relative numeric operations apply to a shape and all its contours in one transaction.
    pub fn transform_draw_shape(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite()
            || value.abs() > 100_000.0
            || self.profile_authoring_preview.is_some()
            || self.profile_volume_mode.is_some()
        {
            return false;
        }
        if !self.begin_profile_authoring_preview() {
            return false;
        }
        let Some(draft) = self.profile_authoring_preview.as_mut() else {
            return false;
        };
        if draft.profile.locked {
            self.cancel_profile_authoring_preview();
            return false;
        }
        match field {
            "translate_x" => draft.profile.workplane.origin[0] += f64::from(value),
            "translate_y" => draft.profile.workplane.origin[1] += f64::from(value),
            "translate_z" => draft.profile.workplane.origin[2] += f64::from(value),
            "rotate_x" | "rotate_y" | "rotate_z" => {
                let axis = match field {
                    "rotate_x" => glam::DVec3::X,
                    "rotate_y" => glam::DVec3::Y,
                    _ => glam::DVec3::Z,
                };
                let rotation = glam::DQuat::from_axis_angle(axis, f64::from(value).to_radians());
                let plane = &mut draft.profile.workplane;
                plane.right = (rotation * glam::DVec3::from_array(plane.right)).to_array();
                plane.up = (rotation * glam::DVec3::from_array(plane.up)).to_array();
                plane.normal = (rotation * glam::DVec3::from_array(plane.normal)).to_array();
            }
            "scale_x" | "scale_y" => {
                if value.abs() < 1.0e-6 {
                    self.cancel_profile_authoring_preview();
                    return false;
                }
                let axis = usize::from(field == "scale_y");
                for contour in std::iter::once(&mut draft.spline).chain(draft.inner.iter_mut()) {
                    for point in &mut contour.points {
                        if let Some(coordinate) = point.position.get_mut(axis) {
                            *coordinate *= f64::from(value);
                        }
                        if let Some(coordinate) = point.handle_in.get_mut(axis) {
                            *coordinate *= f64::from(value);
                        }
                        if let Some(coordinate) = point.handle_out.get_mut(axis) {
                            *coordinate *= f64::from(value);
                        }
                    }
                }
            }
            _ => {
                self.cancel_profile_authoring_preview();
                return false;
            }
        }
        self.commit_profile_authoring_preview()
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn select_draw_point(&mut self, id: uuid::Uuid) -> bool {
        if self.profile_authoring_preview.is_some()
            || self.profile_volume_mode.is_some()
            || !self
                .active_contour_resources()
                .is_some_and(|(_, s)| s.point(id).is_some())
        {
            return false;
        }
        if !self.set_draw_edit_context("points") {
            return false;
        }
        self.state.session.planar_selection.point = Some(id);
        self.state.mark_dirty();
        true
    }

    pub fn set_draw_point_parameter(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite()
            || value.abs() > 100_000.0
            || self.profile_authoring_preview.is_some()
            || self.profile_volume_mode.is_some()
        {
            return false;
        }
        let Some((shape, source)) = self.active_contour_resources() else {
            return false;
        };
        if shape.locked || !shape.visible {
            return false;
        }
        let Some(id) = self.state.session.planar_selection.point else {
            return false;
        };
        let mut spline = source.clone();
        let Some(point) = spline.point_mut(id) else {
            return false;
        };
        let axis = usize::from(field.ends_with('y'));
        let result = match field {
            "x" | "y" => {
                if let Some(position) = point.position.get_mut(axis) {
                    *position = f64::from(value);
                }
                Ok(())
            }
            "in_x" | "in_y" => {
                let mut handle = point.handle_in;
                if let Some(coordinate) = handle.get_mut(axis) {
                    *coordinate = f64::from(value);
                }
                point.set_handle_in(handle)
            }
            "out_x" | "out_y" => {
                let mut handle = point.handle_out;
                if let Some(coordinate) = handle.get_mut(axis) {
                    *coordinate = f64::from(value);
                }
                point.set_handle_out(handle)
            }
            _ => return false,
        };
        if result.is_err() {
            return false;
        }
        if field.starts_with("in_") || field.starts_with("out_") {
            spline.interpolation = petunia_core::SplineInterpolation::CubicBezier;
        }
        match self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline })
        {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn set_draw_point_mode(&mut self, id: &str) -> bool {
        if self.profile_authoring_preview.is_some() || self.profile_volume_mode.is_some() {
            return false;
        }
        let mode = match id {
            "broken" => petunia_core::SplineHandleMode::Broken,
            "aligned" => petunia_core::SplineHandleMode::Aligned,
            "mirrored" => petunia_core::SplineHandleMode::Mirrored,
            _ => return false,
        };
        let Some((shape, source)) = self.active_contour_resources() else {
            return false;
        };
        if shape.locked || !shape.visible {
            return false;
        }
        let Some(id) = self.state.session.planar_selection.point else {
            return false;
        };
        let Some(point) = source.point(id) else {
            return false;
        };
        let mut spline = source.clone();
        let incoming = if point.handle_in == [0.0; 3] && point.handle_out != [0.0; 3] {
            point.handle_out.map(|v| -v)
        } else {
            point.handle_in
        };
        if spline
            .set_handles(id, incoming, point.handle_out, mode)
            .is_err()
        {
            return false;
        }
        match self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline })
        {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn rename_draw_shape(&mut self, name: &str) -> bool {
        if name.trim().is_empty()
            || name.len() > 256
            || self.profile_authoring_preview.is_some()
            || self.profile_volume_mode.is_some()
        {
            return false;
        }
        let Some((profile, _)) = self.active_profile_resources() else {
            return false;
        };
        let mut profile = profile.clone();
        profile.name = name.trim().to_owned();
        match self
            .state
            .dispatch(&petunia_core::UpdateProfileCmd { profile })
        {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }
}
