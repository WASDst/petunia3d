//! Volume do perfil DRAW: extrude, revolve, sweep e parâmetros.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_profile_depth(&mut self, depth: f32) -> bool {
        if !depth.is_finite()
            || depth <= 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        self.state.profile.depth = depth.clamp(0.01, 1000.0);
        self.state.mark_dirty();
        if self.active_profile_closed() {
            if self.profile_volume_mode.is_none() {
                self.enter_profile_volume("extrude");
            } else if self.profile_volume_mode
                == Some(petunia_module_model::ProfileVolumeMode::Extrude)
            {
                self.update_profile_volume_preview();
            }
        }
        true
    }

    pub fn set_profile_wall_thickness(&mut self, thickness: f32) -> bool {
        if !thickness.is_finite()
            || thickness < 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        let thickness = thickness.clamp(0.0, 1000.0);
        self.state.profile.wall_thickness = thickness;
        if let Some((profile, _)) = self.active_profile_resources() {
            let mut profile = profile.clone();
            if (profile.wall_thickness - f64::from(thickness)).abs() <= f64::EPSILON {
                return false;
            }
            profile.wall_thickness = f64::from(thickness);
            if let Err(error) = self
                .state
                .dispatch(&petunia_core::UpdateProfileCmd { profile })
            {
                self.state.set_status(error.to_string());
                return false;
            }
        } else {
            self.state.mark_dirty();
        }
        if self.profile_volume_mode == Some(petunia_module_model::ProfileVolumeMode::Extrude) {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn set_profile_revolve_angle(&mut self, angle_deg: f32) -> bool {
        if !angle_deg.is_finite() || angle_deg <= 0.0 {
            return false;
        }
        self.state.profile.revolve_angle = angle_deg.clamp(1.0, 360.0);
        self.state.mark_dirty();
        if self.profile_volume_mode == Some(petunia_module_model::ProfileVolumeMode::Revolve) {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn enter_profile_volume(&mut self, mode_str: &str) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        self.profile_pointer_up();
        let Some(profile_state) = self.active_profile_state() else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_DRAW_A_PROFILE_BEFORE_GENERATING_VOLUME),
            );
            return false;
        };
        let mode = match mode_str {
            "extrude" => {
                if !profile_state.closed && profile_state.points.len() >= 3 && !self.close_profile()
                {
                    return false;
                }
                if !self.active_profile_closed() {
                    self.state.set_status(self.state.t_id(
                        petunia_config::text_id::STATUS_EXTRUDE_REQUIRES_A_CLOSED_PROFILE_AT_LEAST,
                    ));
                    return false;
                }
                petunia_module_model::ProfileVolumeMode::Extrude
            }
            "revolve" => {
                if profile_state.points.len() < 2 {
                    self.state.set_status(
                        self.state.t_id(
                            petunia_config::text_id::STATUS_REVOLVE_REQUIRES_AT_LEAST_2_POINTS,
                        ),
                    );
                    return false;
                }
                petunia_module_model::ProfileVolumeMode::Revolve
            }
            "sweep" => {
                if profile_state.points.len() < 2 {
                    self.state
                        .set_status(self.state.t_id(
                            petunia_config::text_id::STATUS_SWEEP_REQUIRES_AT_LEAST_2_POINTS,
                        ));
                    return false;
                }
                petunia_module_model::ProfileVolumeMode::Sweep
            }
            _ => return false,
        };

        self.profile_volume_mode = Some(mode);
        self.update_profile_volume_preview();
        self.state.set_status(match mode {
            petunia_module_model::ProfileVolumeMode::Extrude => "Interactive Extrude: drag vertically to adjust depth, Enter to confirm, Esc to cancel",
            petunia_module_model::ProfileVolumeMode::Revolve => "Interactive Revolve: drag horizontally to adjust angle, Enter to confirm, Esc to cancel",
            petunia_module_model::ProfileVolumeMode::Sweep => "Interactive Sweep: Enter to confirm, Esc to cancel",
        });
        true
    }

    pub fn update_profile_volume_preview(&mut self) -> bool {
        let Some(mode) = self.profile_volume_mode else {
            return false;
        };
        let Some(profile) = self.active_profile_state() else {
            return false;
        };

        let mesh_res = match mode {
            petunia_module_model::ProfileVolumeMode::Extrude => {
                petunia_module_model::build_extrude_mesh(&profile)
            }
            petunia_module_model::ProfileVolumeMode::Revolve => {
                petunia_module_model::build_revolve_mesh(&profile)
            }
            petunia_module_model::ProfileVolumeMode::Sweep => {
                // O resultado vira objeto novo (identidade): caminho no mundo.
                petunia_module_model::build_sweep_mesh(
                    &profile,
                    crate::projection::active_mesh_in_world(&self.state).as_deref(),
                )
            }
        };

        match mesh_res {
            Ok(mesh) => {
                // Prévia só no viewport: o documento recebe o objeto no commit.
                let id = *self
                    .profile_preview_id
                    .get_or_insert_with(uuid::Uuid::new_v4);
                self.show_new_object_preview(id, mesh)
            }
            Err(e) => {
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_VOLUME_PREVIEW_ERROR),
                    &[("e", e.to_string())],
                ));
                false
            }
        }
    }

    pub fn commit_profile_volume(&mut self) -> bool {
        if self.profile_volume_mode.is_none() {
            return false;
        }
        // A prévia é o resultado; sem ela (erro de geração), gera agora.
        let preview = self
            .profile_preview_id
            .take()
            .and_then(|_| self.new_object_preview_mesh().cloned());
        self.clear_tool_preview();
        let generated = if let Some(mesh) = preview {
            Some(mesh)
        } else if let Some(mode) = self.profile_volume_mode {
            let Some(profile) = self.active_profile_state() else {
                return false;
            };
            let mesh_res = match mode {
                petunia_module_model::ProfileVolumeMode::Extrude => {
                    petunia_module_model::build_extrude_mesh(&profile)
                }
                petunia_module_model::ProfileVolumeMode::Revolve => {
                    petunia_module_model::build_revolve_mesh(&profile)
                }
                petunia_module_model::ProfileVolumeMode::Sweep => {
                    // O resultado vira objeto novo (identidade): caminho no mundo.
                    petunia_module_model::build_sweep_mesh(
                        &profile,
                        crate::projection::active_mesh_in_world(&self.state).as_deref(),
                    )
                }
            };
            mesh_res.ok()
        } else {
            None
        };
        // Objeto novo por comando: uma entrada de Undo, invariantes validadas.
        if let Some(mesh) = generated {
            let command = petunia_core::AddMeshAssetCmd {
                label: "draw profile",
                name: "Profile".to_string(),
                mesh,
            };
            if let Err(error) = self.state.dispatch(&command) {
                self.state.set_status(format!("draw profile: {error}"));
                return false;
            }
        }

        self.state.profile.clear();
        self.active_profile_id = None;
        self.profile_selected_point = None;
        self.profile_drag_target = None;
        self.profile_edit_gesture = None;
        self.profile_volume_mode = None;
        self.state.set_selection_domain(SelectionDomain::Object);
        self.sync_viewport_context();
        self.state.session.tools.active_tool = "select".to_string();
        self.state.sync_selection();
        self.state.mark_dirty();
        self.state.set_status(self.state.t("profile.generated"));
        true
    }

    pub fn cancel_profile_volume(&mut self) -> bool {
        let had_preview = self.profile_preview_id.is_some() || self.profile_volume_mode.is_some();
        // O documento nunca recebeu a prévia: basta descartá-la.
        if self.profile_preview_id.take().is_some() {
            self.clear_tool_preview();
        }
        self.profile_volume_mode = None;
        if had_preview {
            self.state.set_status(
                self.state.t_id(
                    petunia_config::text_id::STATUS_VOLUME_GENERATION_CANCELLED_2D_PROFILE_KEPT,
                ),
            );
        }
        had_preview
    }

    pub fn generate_profile_extrude(&mut self) -> bool {
        if self.enter_profile_volume("extrude") {
            self.commit_profile_volume()
        } else {
            false
        }
    }

    pub fn generate_profile_revolve(&mut self) -> bool {
        if self.enter_profile_volume("revolve") {
            self.commit_profile_volume()
        } else {
            false
        }
    }

    pub fn generate_profile_sweep(&mut self) -> bool {
        if self.enter_profile_volume("sweep") {
            self.commit_profile_volume()
        } else {
            false
        }
    }
}
