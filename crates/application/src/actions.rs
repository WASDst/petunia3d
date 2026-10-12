//! Ações nomeadas do editor (atalhos, menus): tradução de ação em efeito.
//!
//! O host traduz teclas físicas em nomes de ação (`global.undo`, `model.move`...) e
//! a Application as executa; nenhum tipo de janela ou entrada de plataforma entra aqui.

use petunia_core::events::AppEvent;
use petunia_core::{
    ClearSelectionCmd, Command, CommandError, DeleteSelectionCmd, DuplicateSelectionCmd, EditMode,
    InvertSelectionCmd, SelectAllCmd, SelectMode,
};

use crate::Application;

/// O que o host precisa fazer depois de uma ação (efeitos de plataforma).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionOutcome {
    #[default]
    None,
    /// O usuário pediu Save (diálogo/arquivo é do host).
    SaveRequested,
    /// Enquadrar a seleção (cálculo de câmera da camada de UI).
    FrameSelection,
}

impl Application {
    /// Fronteira para comandos tipados: preserva a validação, o checkpoint e
    /// a propagação de efeitos do único dispatcher da sessão.
    pub fn execute_command(&mut self, command: &dyn Command) -> Result<(), CommandError> {
        self.state.dispatch(command)
    }

    /// Fronteira para comandos registrados; desconhecidos ou indisponíveis
    /// devolvem o erro canônico sem alterar documento nem histórico.
    pub fn execute_registered_command(&mut self, id: &str) -> Result<(), CommandError> {
        self.state.dispatch_command(id)
    }

    /// Undo e Redo usam o histórico e as sessões de preview já existentes.
    pub fn undo(&mut self) -> bool {
        self.state.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.state.redo()
    }

    /// Executa a ação nomeada. Ações desconhecidas são ignoradas.
    pub fn run_action(&mut self, action: &str) -> ActionOutcome {
        let mut outcome = ActionOutcome::None;
        match action {
            "global.undo" => {
                self.undo();
            }
            "global.redo" => {
                self.redo();
            }
            "global.save_project" => {
                outcome = ActionOutcome::SaveRequested;
            }
            "global.toggle_wireframe" | "view.toggle_wireframe" => {
                self.state.shading = match self.state.shading {
                    petunia_render::Shading::Wireframe => petunia_render::Shading::Solid,
                    _ => petunia_render::Shading::Wireframe,
                };
                self.state.mark_dirty();
            }
            "global.help" => {
                self.state.ui.show_help = !self.state.ui.show_help;
                self.state.mark_dirty();
            }
            "global.command_palette" | "window.command_palette" => {
                self.state.ui.show_command_palette = !self.state.ui.show_command_palette;
                if self.state.ui.show_command_palette {
                    self.state.ui.command_palette_query.clear();
                    self.state.ui.command_palette_selected_index = 0;
                }
                self.state.mark_dirty();
            }
            "global.settings" => {
                self.state.ui.show_settings = !self.state.ui.show_settings;
                self.state.mark_dirty();
            }
            "global.toggle_projection" | "view.toggle_projection" => {
                self.state.camera_frame = None;
                self.state.camera.toggle_projection();
                self.state.mark_dirty();
            }
            "global.reset_camera" | "view.reset_camera" => {
                self.state.camera_frame = None;
                self.state.camera.reset();
                self.state.mark_dirty();
            }
            "global.cycle_mode" => {
                // Tab nunca entra num "modo" separado: alterna o domínio de seleção (P3D-015).
                match self.state.selection_domain() {
                    petunia_core::SelectionDomain::Object => {
                        self.state.set_edit_mode(EditMode::Edit);
                    }
                    _ => self.state.set_edit_mode(EditMode::Object),
                }
                self.state.mark_dirty();
            }
            "model.select_vertex" => self.set_tool("select", Some(SelectMode::Vertex)),
            "model.select_edge" => self.set_tool("select", Some(SelectMode::Edge)),
            "model.select_face" => self.set_tool("select", Some(SelectMode::Face)),
            "model.select_object" => {
                self.state.set_edit_mode(EditMode::Object);
                self.state.active_tool = "select".into();
                self.state.mark_dirty();
            }
            "model.frame_selection" => outcome = ActionOutcome::FrameSelection,
            "model.transform" => self.set_tool("transform", None),
            "model.move" | "model.rotate" | "model.scale" => {
                let (tool_id, kind) = match action {
                    "model.move" => ("move", petunia_core::modal::ModalKind::Move),
                    "model.rotate" => ("rotate", petunia_core::modal::ModalKind::Rotate),
                    _ => ("scale", petunia_core::modal::ModalKind::Scale),
                };
                self.state.pending_modal = Some(kind);
                self.state.active_tool = tool_id.into();
                self.state.mark_dirty();
            }
            "model.primitives" => self.set_tool("primitives", None),
            "model.draw_profile" => self.set_tool("draw_profile", None),
            "model.merge" => self.set_tool("merge", None),
            "model.inset" => self.set_tool("inset", None),
            "model.bevel" => self.set_tool("bevel", None),
            "model.subdivide" => self.set_tool("subdivide", None),
            "model.push_pull" => self.set_tool("pushpull", None),
            "model.extrude" => {
                self.set_tool("extrude", None);
            }
            "model.extrude_individual" => {
                let dist = if self.state.extrude_dist == 0.0 {
                    0.5
                } else {
                    self.state.extrude_dist
                };
                let _ = self
                    .state
                    .dispatch(&petunia_core::ExtrudeIndividualCmd { dist });
            }
            "model.flip_diagonal" => {
                let _ = self.state.dispatch(&petunia_core::FlipDiagonalCmd);
            }
            "model.revolve" => self.set_tool("revolve", None),
            "model.delete" => {
                let _ = self.state.dispatch(&DeleteSelectionCmd);
            }
            "model.duplicate" => {
                let _ = self.state.dispatch(&DuplicateSelectionCmd);
            }
            "model.slice" => self.set_tool("slice", None),
            "model.knife" | "model.loop_cut" => {
                self.state.active_tool = if action == "model.knife" {
                    "knife"
                } else {
                    "loop_cut"
                }
                .into();
                self.state.mark_dirty();
            }
            "model.connect" => self.set_tool("connect", None),
            "model.dissolve" => self.set_tool("dissolve", None),
            "model.select_all" => {
                let _ = self.state.dispatch(&SelectAllCmd);
            }
            "model.deselect_all" => {
                let _ = self.state.dispatch(&ClearSelectionCmd);
            }
            "model.invert_selection" => {
                let _ = self.state.dispatch(&InvertSelectionCmd);
            }
            "model.select_linked" => {
                let _ = self.state.dispatch(&petunia_core::SelectLinkedCmd);
            }
            "paint.paint" => self.set_tool("paint", None),
            "paint.size_decrease"
            | "paint.size_increase"
            | "paint.hardness_decrease"
            | "paint.hardness_increase"
                if self.state.workspace == petunia_core::Workspace::Paint
                    || self.state.active_tool == "paint" =>
            {
                let size_step = 1.0;
                let hardness_step = 0.05;
                match action {
                    "paint.size_decrease" => {
                        self.state.canvas_brush = self
                            .state
                            .canvas_brush
                            .saturating_sub(size_step as u32)
                            .max(1);
                    }
                    "paint.size_increase" => {
                        self.state.canvas_brush = self
                            .state
                            .canvas_brush
                            .saturating_add(size_step as u32)
                            .min(512);
                    }
                    "paint.hardness_decrease" => {
                        self.state.brush_hardness =
                            (self.state.brush_hardness - hardness_step).clamp(0.0, 1.0);
                    }
                    "paint.hardness_increase" => {
                        self.state.brush_hardness =
                            (self.state.brush_hardness + hardness_step).clamp(0.0, 1.0);
                    }
                    _ => {}
                }
                self.state.mark_dirty();
            }
            _ => {}
        }
        outcome
    }

    pub fn set_tool(&mut self, id: &str, select: Option<SelectMode>) {
        self.state.active_tool = id.to_string();
        if let Some(sm) = select {
            self.state.set_edit_mode(EditMode::Edit);
            self.state.select_mode = sm;
            match sm {
                SelectMode::Edge => {
                    if let Some(m) = self.state.project.active_mesh_mut() {
                        m.sync_edge_selection_from_verts();
                    }
                }
                SelectMode::Face => {
                    if let Some(m) = self.state.project.active_mesh_mut() {
                        m.sync_face_selection_from_verts();
                    }
                }
                SelectMode::Vertex => {}
            }
        }
        if id == "paint" {
            self.state.set_edit_mode(EditMode::TexturePaint);
        }
        if let Some(t) = self.tools.get(id) {
            t.on_activate(&mut self.state);
        }
        self.state
            .events
            .emit(AppEvent::ToolActivated(id.to_string()));
        self.state.mark_dirty();
    }
}
