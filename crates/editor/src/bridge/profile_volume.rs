//! DRAW volume sessions: transient preview, persistent source, one command at commit.
use crate::bridge::tool_preview::TOOL_PREVIEW_REVISION_BIT;
use crate::*;
use petunia_project::PoseOverride;
use petunia_project::{GeometryGenerator, GeometryGeneratorKind};
use std::sync::Arc;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn begin_free_region_volume(&mut self, hit: &petunia_core::RegionHit) -> bool {
        self.cancel_profile_volume();
        let points: Vec<_> = hit.region.outer.iter().map(|p| [p[0], p[1], 0.0]).collect();
        let spline =
            petunia_project::SplineResource::from_polyline("Region contour", &points, true);
        let plane = petunia_project::Workplane {
            origin: hit.plane.origin.as_dvec3().to_array(),
            right: hit.plane.right.as_dvec3().to_array(),
            up: hit.plane.up.as_dvec3().to_array(),
            normal: hit.plane.normal.as_dvec3().to_array(),
        };
        let mut shape = petunia_project::PlanarShape::new("Region", spline.id, plane);
        shape.holes = hit.region.holes.clone();
        let generator = GeometryGenerator::new(GeometryGeneratorKind::Extrude {
            shape_object: shape.id,
            distance: self.state.profile.depth,
            cap_start: self.draw_caps[0],
            cap_end: self.draw_caps[1],
        });
        let project = &self.state.project.project;
        self.profile_generator_draft = Some((
            generator,
            [project.spline_revision, project.procedural_revision],
        ));
        self.profile_region_revision = Some(project.revision_id());
        self.profile_region_source = Some((spline, shape));
        self.profile_preview_id = Some(uuid::Uuid::new_v4());
        self.profile_volume_mode = Some(petunia_module_model::ProfileVolumeMode::Extrude);
        self.update_profile_volume_preview()
    }

    pub fn set_profile_depth(&mut self, depth: f32) -> bool {
        if !depth.is_finite() || depth.abs() < 1.0e-6 || depth.abs() > 100_000.0 {
            return false;
        }
        self.state.profile.depth = depth;
        self.state.mark_dirty();
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    /// Compatibility action: thickness is materialized as an explicit editable
    /// inner contour, never retained as an invisible shape parameter.
    pub fn set_profile_wall_thickness(&mut self, thickness: f32) -> bool {
        if !thickness.is_finite() || thickness <= 0.0 || self.profile_volume_mode.is_some() {
            return false;
        }
        let Some((profile, spline)) = self.active_profile_resources() else {
            return false;
        };
        if !spline.closed {
            return false;
        }
        let Ok(table) = spline.arc_length_table(0.005) else {
            return false;
        };
        let points: Vec<_> = table
            .polyline()
            .into_iter()
            .map(|p| [p[0] as f32, p[1] as f32])
            .collect();
        let inner = petunia_mesh::curve::offset_polygon(&points, -thickness);
        if inner.len() < 3 {
            return false;
        }
        let mut profile = profile.clone();
        profile.wall_thickness = 0.0;
        profile
            .holes
            .push(inner.into_iter().map(|p| p.map(f64::from)).collect());
        let result = self
            .state
            .dispatch(&petunia_core::UpdateProfileCmd { profile });
        if let Err(error) = result {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.profile.wall_thickness = 0.0;
        true
    }

    pub fn set_profile_revolve_angle(&mut self, angle: f32) -> bool {
        if !angle.is_finite() || !(0.01..=360.0).contains(&angle) {
            return false;
        }
        self.state.profile.revolve_angle = angle;
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        self.state.mark_dirty();
        true
    }

    fn draft_generator_kind(
        &self,
        mode: petunia_module_model::ProfileVolumeMode,
    ) -> Option<GeometryGeneratorKind> {
        let shape = self
            .profile_region_source
            .as_ref()
            .map(|(_, s)| s.id)
            .or(self.state.session.active_shape)?;
        Some(match mode {
            petunia_module_model::ProfileVolumeMode::Extrude => GeometryGeneratorKind::Extrude {
                shape_object: shape,
                distance: self.state.profile.depth,
                cap_start: self.draw_caps[0],
                cap_end: self.draw_caps[1],
            },
            petunia_module_model::ProfileVolumeMode::Revolve => GeometryGeneratorKind::Revolve {
                profile_object: shape,
                axis: self.draw_revolve_axis,
                angle: self.state.profile.revolve_angle,
                segments: self.state.profile.revolve_segments,
                cap_start: self.draw_caps[0],
                cap_end: self.draw_caps[1],
            },
            petunia_module_model::ProfileVolumeMode::Sweep => {
                let source_spline = self.state.project.project.get_profile(shape)?.spline_id;
                let path_id = self.profile_sweep_path?;
                let path = self.state.project.project.get_spline(path_id)?;
                if path.id == source_spline || path.points.len() < 2 {
                    return None;
                }
                GeometryGeneratorKind::Sweep {
                    shape_object: shape,
                    path_object: path.id,
                    cap_start: self.draw_caps[0],
                    cap_end: self.draw_caps[1],
                }
            }
        })
    }

    pub fn enter_profile_volume(&mut self, mode: &str) -> bool {
        if self.modeling_mode != ModelingMode::Draw || self.state.workspace != Workspace::Model {
            return false;
        }
        self.profile_pointer_up();
        if self
            .active_profile_resources()
            .is_some_and(|(shape, _)| shape.locked || !shape.visible)
        {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.shape_is_locked_or_hidden",
                )));
            return false;
        }
        let mode = match mode {
            "extrude" if self.active_profile_closed() => {
                petunia_module_model::ProfileVolumeMode::Extrude
            }
            "revolve" if self.active_profile_point_count() >= 2 => {
                petunia_module_model::ProfileVolumeMode::Revolve
            }
            "sweep" if self.active_profile_closed() => {
                petunia_module_model::ProfileVolumeMode::Sweep
            }
            _ => {
                self.state
                    .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.choose_a_compatible_shape_close_it_explicitly_before_extrusion",
                )));
                return false;
            }
        };
        let Some(kind) = self.draft_generator_kind(mode) else {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.sweep_requires_an_explicit_spline_path",
                )));
            return false;
        };
        self.cancel_profile_volume();
        let project = &self.state.project.project;
        self.profile_generator_draft = Some((
            GeometryGenerator::new(kind),
            [project.spline_revision, project.procedural_revision],
        ));
        self.profile_region_revision = Some(project.revision_id());
        self.profile_volume_mode = Some(mode);
        self.profile_preview_id = Some(uuid::Uuid::new_v4());
        self.update_profile_volume_preview()
    }

    pub fn update_profile_volume_preview(&mut self) -> bool {
        let Some(mode) = self.profile_volume_mode else {
            return false;
        };
        if self
            .profile_region_revision
            .is_some_and(|revision| revision != self.state.project.project.revision_id())
        {
            self.clear_tool_preview();
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.region_geometry_changed_cancel_and_restart_the_preview",
                )));
            return false;
        }
        let Some(kind) = self.draft_generator_kind(mode) else {
            self.clear_tool_preview();
            return false;
        };
        let Some((generator, revision)) = self.profile_generator_draft.as_mut() else {
            return false;
        };
        let project = &self.state.project.project;
        if *revision != [project.spline_revision, project.procedural_revision] {
            self.clear_tool_preview();
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.source_changed_cancel_and_restart_this_volume_preview",
                )));
            return false;
        }
        generator.kind = kind;
        let result = if let Some((spline, shape)) = &self.profile_region_source {
            let mut candidate = project.clone();
            candidate
                .add_spline(spline.clone())
                .map(|_| ())
                .map_err(|e| petunia_project::GeneratorError::Geometry(e.to_string()))
                .and_then(|_| {
                    candidate
                        .add_profile(shape.clone())
                        .map_err(|e| petunia_project::GeneratorError::Geometry(e.to_string()))
                })
                .and_then(|_| generator.evaluate(&candidate))
        } else {
            generator.evaluate(project)
        };
        match result {
            Ok(mesh) => {
                self.profile_volume_diagnostic = None;
                let Some(id) = self.profile_preview_id else {
                    return false;
                };
                self.show_new_object_preview(id, mesh)
            }
            Err(error) => {
                self.profile_volume_diagnostic = Some(error.to_string());
                self.clear_tool_preview();
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn commit_profile_volume(&mut self) -> bool {
        if !self.update_profile_volume_preview() {
            return false;
        }
        let Some((generator, source_revision)) = self.profile_generator_draft.clone() else {
            return false;
        };
        let Some(object_id) = self.profile_preview_id else {
            return false;
        };
        let command = petunia_core::CreateGeometryGeneratorCmd {
            object_id,
            name: match generator.kind {
                GeometryGeneratorKind::Extrude { .. } => "Extrusion",
                GeometryGeneratorKind::Revolve { .. } => "Revolution",
                _ => "Sweep",
            }
            .into(),
            generator,
            source_revision,
        };
        let result = if let Some((spline, shape)) = self.profile_region_source.clone() {
            self.state
                .dispatch(&petunia_core::CreateRegionGeneratorCmd {
                    expected_document_revision: self.profile_region_revision.unwrap_or(u64::MAX),
                    object_id,
                    spline,
                    shape,
                    generator: command.generator,
                    source_revision,
                })
        } else {
            self.state.dispatch(&command)
        };
        if let Err(error) = result {
            self.state.set_status(error.to_string());
            return false;
        }
        self.clear_tool_preview();
        self.profile_generator_draft = None;
        self.profile_region_source = None;
        self.profile_region_revision = None;
        self.profile_preview_id = None;
        self.profile_volume_mode = None;
        self.profile_volume_diagnostic = None;
        self.state.session.active_shape = None;
        self.state.session.planar_selection.contour = None;
        self.state.set_selection_domain(SelectionDomain::Object);
        self.state
            .set_status(self.state.t_id(petunia_config::TextId::new(
                "draw_status.parametric_volume_created_the_source_shape_remains_editable",
            )));
        self.sync_viewport_context();
        true
    }

    pub fn cancel_profile_volume(&mut self) -> bool {
        let had =
            self.profile_generator_draft.take().is_some() || self.profile_volume_mode.is_some();
        if self.profile_preview_id.take().is_some() {
            self.clear_tool_preview();
        }
        self.profile_volume_mode = None;
        self.profile_volume_diagnostic = None;
        self.profile_region_source = None;
        self.profile_region_revision = None;
        if had {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.volume_preview_cancelled_source_shape_preserved",
                )));
        }
        had
    }

    pub fn generator_parameter_gesture(&mut self, phase: i32) -> bool {
        if phase == 0 {
            if self.profile_volume_mode.is_some() || self.profile_authoring_preview.is_some() {
                return false;
            }
            let Some(asset) = self.state.project.active() else {
                return false;
            };
            let Some(generator) = asset.geometry_source.clone() else {
                return false;
            };
            let project = &self.state.project.project;
            self.generator_parameter_preview = Some((asset.id, generator, project.revision_id()));
            return true;
        }
        let Some((object_id, generator, clock)) = self.generator_parameter_preview.take() else {
            return false;
        };
        self.clear_tool_preview();
        if phase != 1 {
            return true;
        }
        let project = &self.state.project.project;
        if clock != project.revision_id() {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.generator_source_changed_preview_discarded",
                )));
            return false;
        }
        let command = petunia_core::UpdateGeometryGeneratorCmd {
            object_id,
            expected_revision: generator.revision,
            generator,
        };
        match self.state.dispatch(&command) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn edit_selected_generator_distance(&mut self, distance: f32) -> bool {
        self.edit_selected_generator_parameter("distance", distance)
    }

    pub fn edit_selected_generator_parameter(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let Some(asset) = self.state.project.active() else {
            return false;
        };
        let Some(mut generator) = self
            .generator_parameter_preview
            .as_ref()
            .filter(|(id, _, _)| *id == asset.id)
            .map(|(_, g, _)| g.clone())
            .or_else(|| asset.geometry_source.clone())
        else {
            return false;
        };
        match (&mut generator.kind, field) {
            (GeometryGeneratorKind::Extrude { distance, .. }, "distance") => *distance = value,
            (GeometryGeneratorKind::Revolve { angle, .. }, "angle") => *angle = value,
            (GeometryGeneratorKind::Revolve { segments, .. }, "segments")
                if value.fract() == 0.0 && (3.0..=512.0).contains(&value) =>
            {
                *segments = value as u32
            }
            _ => return false,
        }
        let object_id = asset.id;
        if let Some((target, draft, _)) = self.generator_parameter_preview.as_mut() {
            if *target != object_id {
                return false;
            }
            let mesh = match generator.evaluate(&self.state.project.project) {
                Ok(mesh) => mesh,
                Err(error) => {
                    self.state.set_status(error.to_string());
                    return false;
                }
            };
            *draft = generator;
            self.tool_preview_revision = self.tool_preview_revision.wrapping_add(1);
            let mut preview =
                PoseOverride::new(TOOL_PREVIEW_REVISION_BIT | self.tool_preview_revision);
            preview.insert(object_id, mesh);
            self.tool_preview = Some(Arc::new(preview));
            self.state.mark_dirty();
            return true;
        }
        let command = petunia_core::UpdateGeometryGeneratorCmd {
            object_id,
            expected_revision: generator.revision,
            generator,
        };
        match self.state.dispatch(&command) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn make_selected_generated_editable(&mut self) -> bool {
        let Some(asset) = self.state.project.active() else {
            return false;
        };
        let Some(source) = &asset.geometry_source else {
            return false;
        };
        let command = petunia_core::MakeGeneratedEditableCmd {
            object_id: asset.id,
            expected_revision: source.revision,
        };
        match self.state.dispatch(&command) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn generate_profile_extrude(&mut self) -> bool {
        self.enter_profile_volume("extrude") && self.commit_profile_volume()
    }
    pub fn generate_profile_revolve(&mut self) -> bool {
        self.enter_profile_volume("revolve") && self.commit_profile_volume()
    }
    pub fn generate_profile_sweep(&mut self) -> bool {
        self.enter_profile_volume("sweep") && self.commit_profile_volume()
    }
}
