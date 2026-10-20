//! Thin interaction adapter over the shared two-solid region session.
use crate::bridge::tool_preview::TOOL_PREVIEW_REVISION_BIT;
use crate::*;
use petunia_project::PoseOverride;
use std::sync::Arc;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn begin_solid_shape_builder(&mut self) -> bool {
        if self.state.workspace != Workspace::Model
            || self.modeling_mode != ModelingMode::Draw
            || self.profile_volume_mode.is_some()
            || self.profile_authoring_preview.is_some()
            || self.state.modal.is_some()
        {
            return false;
        }
        let ids = &self.state.session.selection.assets;
        let [first, second] = ids.as_slice() else {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.select_exactly_two_solid_objects",
                )));
            return false;
        };
        let result = petunia_core::SolidShapeBuilderSession::begin(
            &self.state.project.project,
            *first,
            *second,
        );
        let session = match result {
            Ok(session) => session,
            Err(error) => {
                self.state.set_status(error.to_string());
                return false;
            }
        };
        self.tool_preview_revision = self.tool_preview_revision.wrapping_add(1);
        let mut preview = PoseOverride::new(TOOL_PREVIEW_REVISION_BIT | self.tool_preview_revision);
        preview.insert(session.first.id, petunia_core::Mesh::default());
        preview.insert(session.second.id, petunia_core::Mesh::default());
        for region in &session.regions {
            let mut mesh = region.mesh.clone();
            mesh.transform_affine(session.first.world_matrix());
            preview.insert_transient(region.id, mesh);
        }
        self.tool_preview = Some(Arc::new(preview));
        self.solid_shape_builder = Some(session);
        self.solid_hover = None;
        self.solid_cycle = 0;
        self.solid_xray = false;
        self.state.session.tools.active_tool = "shape_builder".into();
        self.state.set_status(
            "Shape Builder: choose Unite, Remove or Extract; select regions and confirm",
        );
        self.state.mark_dirty();
        true
    }

    /// Keyboard/assistive alternative to ray-based region selection.
    pub fn toggle_solid_region_id(&mut self, id: uuid::Uuid) -> bool {
        let Some(session) = self.solid_shape_builder.as_mut() else {
            return false;
        };
        if session.validate(&self.state.project.project).is_err() {
            return false;
        }
        let Some(index) = session.regions.iter().position(|region| region.id == id) else {
            return false;
        };
        if !session.selected.remove(&index) {
            session.selected.insert(index);
        }
        self.solid_hover = Some(index);
        self.state.mark_dirty();
        true
    }

    pub fn set_solid_shape_intent(&mut self, id: &str) -> bool {
        let Some(session) = self.solid_shape_builder.as_mut() else {
            return false;
        };
        session.intent = match id {
            "unite" => petunia_core::SolidShapeIntent::Unite,
            "remove" => petunia_core::SolidShapeIntent::Remove,
            "extract" => petunia_core::SolidShapeIntent::Extract,
            _ => return false,
        };
        self.state.mark_dirty();
        true
    }

    pub(crate) fn hover_solid_region(&mut self, pixel: [f32; 2]) -> bool {
        let previous = self.solid_hover;
        self.solid_pointer = pixel;
        let Some(session) = self.solid_shape_builder.as_ref() else {
            return false;
        };
        if session.validate(&self.state.project.project).is_err() {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.solid_source_changed_cancel_and_restart_shape_builder",
                )));
            self.solid_hover = None;
            return previous.is_some();
        }
        if self.viewport_size.iter().any(|v| *v <= 1.0) || pixel.iter().any(|v| !v.is_finite()) {
            return false;
        }
        let (origin, direction) = self.state.session.camera.ray(
            pixel[0] / self.viewport_size[0] * 2.0 - 1.0,
            1.0 - pixel[1] / self.viewport_size[1] * 2.0,
        );
        let hits = session.hits(origin, direction);
        self.solid_hover = hits
            .get(self.solid_cycle % hits.len().max(1))
            .map(|(_, i)| *i);
        previous != self.solid_hover
    }

    pub fn cycle_solid_region(&mut self) -> bool {
        if self.solid_shape_builder.is_none() {
            return false;
        }
        self.solid_cycle = self.solid_cycle.wrapping_add(1);
        self.hover_solid_region(self.solid_pointer);
        self.state.mark_dirty();
        true
    }

    pub fn toggle_solid_xray(&mut self) -> bool {
        if self.solid_shape_builder.is_none() {
            return false;
        }
        self.solid_xray = !self.solid_xray;
        self.state.mark_dirty();
        true
    }

    pub(crate) fn select_solid_region(&mut self, pixel: [f32; 2], toggle: bool) -> bool {
        self.hover_solid_region(pixel);
        let Some(index) = self.solid_hover else {
            return false;
        };
        let Some(session) = self.solid_shape_builder.as_mut() else {
            return false;
        };
        if toggle && session.selected.contains(&index) {
            session.selected.remove(&index);
        } else {
            session.selected.insert(index);
        }
        self.state.mark_dirty();
        true
    }

    pub fn commit_solid_shape_builder(&mut self) -> bool {
        let Some(session) = self.solid_shape_builder.as_ref() else {
            return false;
        };
        let command = petunia_core::CommitSolidShapeBuilderCmd {
            session: session.clone(),
        };
        match self.state.dispatch(&command) {
            Ok(()) => {
                self.solid_shape_builder = None;
                self.solid_hover = None;
                self.solid_xray = false;
                self.clear_tool_preview();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn cancel_solid_shape_builder(&mut self) -> bool {
        if self.solid_shape_builder.take().is_none() {
            return false;
        }
        self.solid_hover = None;
        self.solid_xray = false;
        self.clear_tool_preview();
        self.state
            .set_status(self.state.t_id(petunia_config::TextId::new(
                "draw_status.shape_builder_cancelled_both_source_objects_are_unchanged",
            )));
        true
    }

    pub(crate) fn solid_region_commands(&self) -> String {
        let Some(session) = &self.solid_shape_builder else {
            return String::new();
        };
        let mut commands = String::new();
        let mut remaining = 40_000usize;
        let world = session.first.world_matrix();
        for index in session.selected.iter().copied().chain(self.solid_hover) {
            let Some(region) = session.regions.get(index) else {
                continue;
            };
            for face in &region.mesh.faces {
                if face.verts.len() > remaining {
                    return commands;
                }
                remaining -= face.verts.len();
                let points = face
                    .verts
                    .iter()
                    .filter_map(|v| region.mesh.verts.get(*v as usize))
                    .map(|v| world.transform_point3(v.vec()));
                crate::projection::write_clipped_path(
                    &self.state.session.camera,
                    self.viewport_size,
                    points,
                    true,
                    &mut commands,
                );
            }
        }
        commands
    }
}
