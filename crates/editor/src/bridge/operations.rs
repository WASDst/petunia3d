//! Operações em andamento: Escape, confirmação, click-away e modais numéricos.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Ajusta a última operação pelo texto do card (mesma entrada de Undo).
    pub fn commit_last_operation_text(&mut self, text: &str) -> bool {
        if self.decal_last_operation_adjustable()
            && let Some(last) = self.decal_last_operation.as_ref()
        {
            return match numeric::parse_numeric_with_base(text, last.value) {
                Ok(value) => self.adjust_decal_last_operation(value),
                Err(_) => false,
            };
        }
        let Some(last) = self.last_operation.as_ref() else {
            return false;
        };
        match numeric::parse_numeric_with_base(text, last.primary_value()) {
            Ok(value) => self.adjust_last_operation(value),
            Err(_) => false,
        }
    }

    /// Reaplica a última operação com `value`, substituindo o gesto no
    /// histórico sem aumentar a profundidade do Undo.
    pub fn adjust_last_operation(&mut self, value: f32) -> bool {
        let Some(last) = self.last_operation.clone() else {
            return false;
        };
        if !self.state.last_operation_is_current(&last) {
            self.last_operation = None;
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::TOOL_GRAMMAR_EXPIRED),
            );
            return false;
        }
        match self.state.adjust_last_operation(&last, value) {
            Ok(next) => {
                self.last_operation = next;
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::TOOL_GRAMMAR_ADJUSTED),
                );
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// A "Última operação" ainda pode ser ajustada.
    pub fn last_operation_adjustable(&self) -> bool {
        self.tool_modal.is_none()
            && self
                .last_operation
                .as_ref()
                .is_some_and(|last| self.state.last_operation_is_current(last))
    }

    /// Abre a sessão modal de uma ferramenta paramétrica com preview próprio.
    pub fn begin_tool_modal(&mut self, kind: ToolModalKind) -> bool {
        if self
            .state
            .project
            .active()
            .is_some_and(|a| a.geometry_source.is_some())
        {
            self.state.set_status(
                "This object is parametric. Choose Make Editable before editing its faces",
            );
            return false;
        }
        // Um valor digitado numa operação anterior nunca vaza para a próxima.
        self.tool_session.clear_numeric();
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        // Troca de domínio visível; nunca seleciona tudo por conta própria
        // (constituição 11). Sem seleção aplicável, o core recusa com erro.
        if self.state.selection_domain() == SelectionDomain::Object {
            match kind {
                ToolModalKind::Extrude
                | ToolModalKind::ExtrudeIndividual
                | ToolModalKind::Inset
                | ToolModalKind::PushPull => {
                    self.state.set_selection_domain(SelectionDomain::Face);
                    self.sync_viewport_context();
                }
                ToolModalKind::Bevel => {
                    self.state.set_selection_domain(SelectionDomain::Edge);
                    self.sync_viewport_context();
                }
                ToolModalKind::ScaleSelection => {}
            }
        }
        match self.state.begin_modal(kind.modal_kind()) {
            Ok(()) => {
                self.tool_modal = Some(kind);
                self.state.session.tools.active_tool = match kind {
                    ToolModalKind::ScaleSelection => "scale",
                    ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual => "extrude",
                    ToolModalKind::Inset => "inset",
                    ToolModalKind::Bevel => "bevel",
                    ToolModalKind::PushPull => "push_pull",
                }
                .to_string();
                self.keyboard_tool_modal_active = false;
                let initial = match kind {
                    ToolModalKind::Inset => 0.2,
                    ToolModalKind::Bevel => 0.05,
                    ToolModalKind::ScaleSelection => 1.0,
                    _ => 0.0,
                };
                self.tool_modal_value = initial;
                if initial != 0.0 {
                    let _ = self.state.update_modal(glam::Vec3::ZERO, initial);
                }
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Ajusta o preview pelo arrasto vertical na viewport.
    pub fn scrub_tool_modal(&mut self, delta_y: f32, fine: bool) -> bool {
        let Some(kind) = self.tool_modal else {
            return false;
        };
        if !self.tool_session.numeric_text().is_empty() {
            // Enquanto houver texto digitado, ele controla o valor (constituição 11).
            return false;
        }
        let step = kind.step();
        let delta_y = delta_y
            * if fine { 0.1 } else { 1.0 }
            * if self.state.ui.invert_vertical_drag {
                -1.0
            } else {
                1.0
            };
        let world_per_pixel =
            self.state.session.camera.visible_height() / self.viewport_size[1].max(1.0);
        let delta = match kind {
            ToolModalKind::Inset => -delta_y * step * 0.5,
            ToolModalKind::Bevel => -delta_y * world_per_pixel * 0.5,
            ToolModalKind::ScaleSelection => -delta_y * step * 0.5,
            _ => -delta_y * world_per_pixel * 0.5,
        };
        self.set_tool_modal_value(self.tool_modal_value + delta)
    }

    /// Define o valor absoluto do preview (arrasto e campo numérico).
    pub fn set_tool_modal_value(&mut self, value: f32) -> bool {
        let Some(kind) = self.tool_modal else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        let (minimum, maximum) = kind.bounds();
        let value = value.clamp(minimum, maximum);
        if self.state.update_modal(glam::Vec3::ZERO, value).is_err() {
            // Topologia recusada (ex.: bevel inválido): mantém o último preview.
            return false;
        }
        self.tool_modal_value = value;
        self.state.mark_dirty();
        true
    }

    /// Confirma a ferramenta paramétrica como uma única operação de undo.
    pub fn commit_tool_modal(&mut self) -> bool {
        self.release_latched_session();
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.tool_session.clear_numeric();
        self.keyboard_tool_modal_active = false;
        self.last_operation = self.state.commit_modal_gesture();
        self.state.mark_dirty();
        true
    }

    pub fn cancel_tool_modal(&mut self) -> bool {
        self.release_latched_session();
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.tool_session.clear_numeric();
        self.keyboard_tool_modal_active = false;
        self.state.cancel_modal();
        self.state.mark_dirty();
        true
    }

    pub(crate) fn cancel_active_operation(&mut self) -> bool {
        let mut cancelled = self.cancel_profile_transform();
        cancelled |= self.cancel_profile_authoring_preview();
        cancelled |= self.cancel_solid_shape_builder();
        cancelled |= self.generator_parameter_gesture(2);
        cancelled |= self.cancel_paint_stroke();
        cancelled |= self.cancel_tool_modal();
        cancelled |= self.cancel_loop_cut();
        if self.profile_volume_mode.is_some() {
            cancelled |= self.cancel_profile_volume();
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            self.state.session.tools.active_tool = "select".to_string();
            self.loop_cut_hover_ring = None;
            self.loop_cut_hover_source = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            cancelled = true;
        }
        cancelled |= self.cancel_slice();
        cancelled |= self.cancel_knife();
        cancelled |= self.cancel_transform();
        if self.micro_inspector_open {
            self.micro_inspector_open = false;
            self.overlays.remove(OverlayId::MicroInspector);
            cancelled = true;
        }
        self.drag = None;
        self.gizmo_drag = None;
        self.tool_session.clear_numeric();
        self.instant_transform = false;
        cancelled
    }

    pub fn handle_escape(&mut self) -> bool {
        // A decisão modal tem precedência sobre cancelar gestos do documento.
        // Não atravessar um topo não dismissible para atingir a ferramenta.
        if self
            .overlays
            .top()
            .is_some_and(|entry| entry.kind == OverlayKind::Modal)
        {
            if let Some(entry) = self.overlays.esc() {
                self.hide_overlay(entry.id);
            }
            return true;
        }
        if self.close_context_menu() {
            return true;
        }
        if self.close_menu() {
            return true;
        }
        if self.micro_inspector_open {
            self.micro_inspector_open = false;
            self.overlays.remove(OverlayId::MicroInspector);
            self.state.mark_dirty();
            return true;
        }
        if self.cancel_rename() {
            return true;
        }
        if self.tool_session.is_gesture_active() {
            let effect = self.tool_session.key(petunia_core::ToolKey::Cancel);
            self.apply_tool_effect(effect, false, false);
            return true;
        }
        if self.cancel_paint_stroke() {
            return true;
        }
        if (matches!(
            self.state.session.tools.active_tool.as_str(),
            "path_paint" | "projection"
        ) || self.surface_paint.stencil_mode)
            && self.cancel_surface_paint()
        {
            return true;
        }
        // Escada do Esc: pontos coletados antes de sair da ferramenta.
        if !self.poly_pen_points.is_empty() {
            self.poly_pen_points.clear();
            self.state.mark_dirty();
            return true;
        }
        if self.cancel_paint_stroke() {
            return true;
        }
        if self.cancel_tool_modal() {
            return true;
        }
        if self.drag.take().is_some() {
            self.cancel_transform();
            return true;
        }
        if self.cancel_loop_cut() {
            return true;
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            self.state.session.tools.active_tool = "select".to_string();
            self.loop_cut_hover_ring = None;
            self.loop_cut_hover_source = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            self.state.mark_dirty();
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_CANCELLED),
            );
            return true;
        }
        if self.profile_volume_mode.is_some() {
            self.cancel_profile_volume();
            return true;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            self.cancel_profile_point_drag();
            self.state.profile.clear();
            self.state.session.active_shape = None;
            self.state.session.planar_selection.point = None;
            self.profile_drag_target = None;
            self.profile_edit_gesture = None;
            self.state.session.tools.active_tool = "select".to_string();
            self.state.mark_dirty();
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_PROFILE_EDITING_FINISHED),
            );
            return true;
        }
        if self.state.primitive_session_valid() {
            // A primitiva criada é um gesto confirmado: Esc fecha o card e a
            // mantém; Undo a remove (constituição 11).
            self.state.finalize_primitive_session();
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::TOOL_GRAMMAR_PRIMITIVE_KEPT),
            );
            self.state.mark_dirty();
            return true;
        }
        if self.cancel_slice() {
            return true;
        }
        if self.cancel_knife() {
            return true;
        }
        if self.cancel_transform() {
            return true;
        }
        if self.state.session.edit_pivot {
            self.state.session.edit_pivot = false;
            self.state.mark_dirty();
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_EDIT_PIVOT_EXITED),
            );
            return true;
        }
        if let Some(entry) = self.overlays.esc() {
            self.hide_overlay(entry.id);
            return true;
        }
        self.exit_grammar_tool()
    }

    pub fn handle_click_away(&mut self) -> bool {
        // Um click-away trata somente o topo: nunca fecha camadas por baixo
        // de uma superfície pinada ou que exige decisão explícita.
        if !self.overlays.is_empty() {
            let Some(entry) = self.overlays.click_away() else {
                return false;
            };
            match entry.id {
                OverlayId::ContextMenu | OverlayId::OutlinerContextMenu => {
                    self.close_context_menu();
                }
                OverlayId::MenuBar => {
                    self.close_menu();
                }
                _ => self.hide_overlay(entry.id),
            }
            return true;
        }
        false
    }

    pub(crate) fn preview_modal_text(&mut self) {
        let Ok(value) = numeric::parse_numeric(self.tool_session.numeric_text()) else {
            return;
        };
        if self.tool_modal.is_some() {
            self.set_tool_modal_value(value);
            return;
        }
        let Some(modal) = self.state.session.tools.modal.as_ref() else {
            return;
        };
        let direction = match modal.constraint {
            petunia_core::ModalConstraint::Axis(index) => {
                let mut axis = glam::Vec3::ZERO;
                axis[index] = 1.0;
                axis
            }
            petunia_core::ModalConstraint::Plane(index) => {
                let mut direction = self.state.session.camera.right();
                direction[index] = 0.0;
                direction.normalize_or_zero()
            }
            petunia_core::ModalConstraint::Free => self.state.session.camera.right(),
        };
        if let Err(error) = self.state.update_modal(direction * value, value) {
            self.state.set_status(error.to_string());
        }
    }

    pub(crate) fn confirm_active_operation(&mut self) -> bool {
        if self.solid_shape_builder.is_some() {
            return self.commit_solid_shape_builder();
        }
        if self.decal_gesture.is_some() {
            let effect = self.tool_session.key(petunia_core::ToolKey::Confirm);
            if effect == petunia_core::ToolEffect::Nothing {
                // Gesto aberto pelo caminho do traço (sem sessão da gramática).
                return self.commit_decal_gesture();
            }
            return self.apply_tool_effect(effect, false, false);
        }
        if matches!(
            self.state.session.tools.active_tool.as_str(),
            "path_paint" | "projection"
        ) {
            return self.apply_surface_paint();
        }
        if !self.poly_pen_points.is_empty() {
            return self.poly_pen_close();
        }
        if self.tool_modal.is_some() {
            return self.commit_tool_modal();
        }
        if self.loop_cut.is_some() {
            return self.commit_loop_cut();
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            return self.place_loop_cut_hover();
        }
        if self.state.session.tools.active_tool == "slice" {
            return self.commit_slice();
        }
        if self.state.session.tools.active_tool == "cut"
            || self.state.session.tools.active_tool == "knife"
        {
            return self.commit_knife();
        }
        if self.profile_volume_mode.is_some() {
            return self.commit_profile_volume();
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            return self.finish_draw_curve();
        }
        if self.drag.is_some() {
            return self.end_viewport_transform();
        }
        self.commit_transform()
    }
}
