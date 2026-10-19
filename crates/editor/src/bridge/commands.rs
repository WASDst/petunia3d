//! Execução de comandos nomeados, atalhos e ações rápidas.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn toggle_quick_action(&mut self, id: &str) -> bool {
        let pinned = self
            .state
            .ui
            .model_quick_action_ids()
            .iter()
            .any(|item| item == id);
        self.state.ui.set_model_quick_action_pinned(id, !pinned)
    }

    pub fn reset_quick_actions(&mut self) -> bool {
        self.state.ui.reset_model_quick_actions()
    }

    pub fn execute_quick_action(&mut self, id: &str) -> bool {
        if !self
            .state
            .ui
            .model_quick_action_ids()
            .iter()
            .any(|item| item == id)
        {
            return false;
        }
        match id {
            "model.loop_cut" => {
                self.apply(UiIntent::SetActiveTool("loop_cut".into()));
                true
            }
            other => {
                if let Err(error) = self.execute_core_command(other) {
                    self.state.set_status(error.to_string());
                    false
                } else {
                    true
                }
            }
        }
    }

    pub fn execute_command(&mut self, id: CommandId) {
        self.command_search_visible = false;
        self.overlays.remove(OverlayId::CommandPalette);
        match id {
            CommandId::SaveProject => self.apply(UiIntent::SaveProject),
            CommandId::OpenProject => self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_OPEN_REQUESTED),
            ),
            CommandId::Undo => {
                let _ = self.execute_core_command("edit.undo");
            }
            CommandId::Redo => {
                let _ = self.execute_core_command("edit.redo");
            }
            CommandId::AddCube => {
                let _ = self.execute_core_command("model.add_cube");
            }
            CommandId::AddSphere => {
                let _ = self.execute_core_command("model.add_sphere");
            }
            CommandId::AddCylinder => {
                let _ = self.execute_core_command("model.add_cylinder");
            }
            CommandId::AddPlane => {
                let _ = self.execute_core_command("model.add_plane");
            }
            CommandId::DeleteSelected => {
                let _ = self.execute_core_command("edit.delete");
            }
            CommandId::SelectModeObject => {
                let _ = self.execute_core_command("select.domain_object");
            }
            CommandId::SelectModePoint => {
                let _ = self.execute_core_command("select.domain_vertex");
            }
            CommandId::SelectModeEdge => {
                let _ = self.execute_core_command("select.domain_edge");
            }
            CommandId::SelectModeFace => {
                let _ = self.execute_core_command("select.domain_face");
            }
            CommandId::SelectLasso => self.apply(UiIntent::SetActiveTool("lasso_select".into())),
            CommandId::ToolCursor => self.apply(UiIntent::SetActiveTool("cursor".into())),
            CommandId::FrameCursor => {
                self.frame_cursor();
            }
            CommandId::TransformCombined => self.apply(UiIntent::SetActiveTool("transform".into())),
            CommandId::PaintBrush => self.apply(UiIntent::SetActiveTool("brush".to_string())),
            CommandId::PaintEraser => self.apply(UiIntent::SetActiveTool("eraser".to_string())),
            CommandId::PaintFill => self.apply(UiIntent::SetActiveTool("fill".to_string())),
            CommandId::UvUnwrap => {
                if let Err(error) = self.state.dispatch_command("uv.unwrap_auto") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::UvPackIslands => {
                if let Err(error) = self.state.dispatch_command("uv.pack_islands") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::FrameSelection => {
                if let Err(error) = self.state.dispatch_command("view.frame_selection") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::ToggleWireframe => {
                if let Err(error) = self.state.dispatch_command("view.toggle_wireframe") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::ToggleWireOverlay => {
                if let Err(error) = self.state.dispatch_command("view.toggle_wire_overlay") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::OpenSettings => self.apply(UiIntent::OpenSettings),
            CommandId::ToggleSceneDrawer => self.apply(UiIntent::ToggleSceneDrawer),
            CommandId::ToggleInspector => self.apply(UiIntent::ToggleInspector),
            CommandId::TogglePlanarNavigation => self.apply(UiIntent::TogglePlanarNavigation),
            CommandId::DuplicateSelected => {
                let _ = self.execute_core_command("edit.duplicate");
            }
            CommandId::SelectAll => {
                let _ = self.execute_core_command("select.all");
            }
            CommandId::ClearSelection => {
                let _ = self.execute_core_command("select.none");
            }
            CommandId::InvertSelection => {
                let _ = self.execute_core_command("select.invert");
            }
            CommandId::ToggleAssetLibrary => self.apply(UiIntent::ToggleAssetLibrary),
            CommandId::ResetCamera => self.apply(UiIntent::ResetCamera),
            CommandId::ToggleProjection => self.apply(UiIntent::ToggleProjection),
            CommandId::SaveActiveAsAsset => self.apply(UiIntent::SaveActiveAsAsset),
            CommandId::ToggleFaceOrientation => self.apply(UiIntent::ToggleFaceOrientation),
            CommandId::ToggleUvChecker => self.apply(UiIntent::ToggleUvChecker),
            CommandId::ToggleProportionalEditing => self.apply(UiIntent::ToggleProportionalEditing),
            CommandId::ToggleSnap => self.apply(UiIntent::ToggleSnapEnabled),
            CommandId::OriginToGeometry => self.apply(UiIntent::SetOriginGeometry),
            CommandId::OriginToBottom => self.apply(UiIntent::SetOriginBottom),
            CommandId::OriginToCursor => self.apply(UiIntent::SetOriginCursor),
            CommandId::OriginToSelection => self.apply(UiIntent::SetOriginSelection),
            CommandId::GeometryToOrigin => self.apply(UiIntent::SetGeometryToOrigin),
            CommandId::ToggleEditPivot => self.apply(UiIntent::ToggleEditPivot),
            CommandId::SeparateSelection => {
                let _ = self.execute_core_command("model.separate_selection");
            }
        }
    }

    pub fn search_commands(&self, query: &str) -> Vec<petunia_core::CommandPaletteItem> {
        let mut items = self.state.commands.query(query, &self.state);
        items.extend(self.surface_action_items(query));
        items
    }

    pub fn execute_core_command(&mut self, id: &str) -> Result<(), petunia_core::CommandError> {
        self.command_search_visible = false;
        self.overlays.remove(OverlayId::CommandPalette);
        if self.execute_draw_extension(id) || self.execute_surface_action(id) {
            return Ok(());
        }
        let pathfinder = match id {
            "draw.unite" => Some(petunia_core::PathfinderOp::Unite),
            "draw.subtract" => Some(petunia_core::PathfinderOp::Subtract),
            "draw.intersect" => Some(petunia_core::PathfinderOp::Intersect),
            "draw.exclude" => Some(petunia_core::PathfinderOp::Exclude),
            _ => None,
        };
        if let Some(op) = pathfinder {
            return if self.run_pathfinder(op) {
                Ok(())
            } else {
                Err(petunia_core::CommandError::Execution(
                    self.state.ui.status.clone(),
                ))
            };
        }
        if self.state.primitive_session_valid()
            && id != "primitive.confirm"
            && id != "primitive.cancel"
        {
            self.state.finalize_primitive_session();
        }
        if id == "model.delete" {
            self.delete_or_dissolve_selection();
            return Ok(());
        }
        // Ações do card da faca: aplicar todos os segmentos como um Undo ou
        // restaurar a malha original.
        if id == "model.knife_apply" {
            self.commit_knife();
            return Ok(());
        }
        if id == "model.knife_cancel" {
            self.cancel_knife();
            return Ok(());
        }
        if id == "model.duplicate" {
            self.duplicate_selection();
            return Ok(());
        }
        if id == "edit.copy" {
            self.copy_selection();
            return Ok(());
        }
        if id == "edit.paste" {
            self.paste_clipboard();
            return Ok(());
        }
        if id == "model.extrude"
            && self.state.session.tools.active_tool == "draw_profile"
            && (self.active_profile_closed() || self.active_profile_point_count() >= 3)
        {
            if self.profile_volume_mode == Some(petunia_module_model::ProfileVolumeMode::Extrude) {
                self.commit_profile_volume();
            } else {
                self.enter_profile_volume("extrude");
            }
            return Ok(());
        }
        // Ferramentas paramétricas abrem uma sessão modal com Tool Properties
        // próprias em vez de rodar como one-shot de valor fixo.
        if let Some(kind) = ToolModalKind::from_id(id) {
            self.begin_tool_modal(kind);
            return Ok(());
        }
        if id.starts_with("select.domain_") || id == "select.cycle_domain" {
            let res = self.state.dispatch_command(id);
            self.report_selection_domain();
            return res;
        }
        if id == "select.all" || id == "select.none" || id == "select.invert" {
            return self.state.dispatch_command(id);
        }
        match id {
            "uv.unwrap" => self.state.dispatch_command("uv.unwrap_auto"),
            "uv.pack_islands" => self.state.dispatch_command("uv.pack_islands"),
            "uv.stitch" => self.state.dispatch_command("uv.stitch"),
            "uv.relax" => self.state.dispatch_command("uv.relax"),
            "model.connect" => self.state.dispatch_command("model.connect"),
            "model.spin" => self.state.dispatch_command("model.spin"),
            "model.dissolve" => self.state.dispatch_command("model.dissolve"),
            "uv.equalize_texel_density" => {
                self.uv_equalize_texel_density();
                Ok(())
            }
            "model.frame_selection" => self.state.dispatch_command("view.frame_selection"),
            "view.camera_front" => {
                self.set_view_preset(petunia_core::ViewPreset::Front);
                Ok(())
            }
            "view.camera_back" => {
                self.set_view_preset(petunia_core::ViewPreset::Back);
                Ok(())
            }
            "view.camera_right" => {
                self.set_view_preset(petunia_core::ViewPreset::Right);
                Ok(())
            }
            "view.camera_left" => {
                self.set_view_preset(petunia_core::ViewPreset::Left);
                Ok(())
            }
            "view.camera_top" => {
                self.set_view_preset(petunia_core::ViewPreset::Top);
                Ok(())
            }
            "view.camera_bottom" => {
                self.set_view_preset(petunia_core::ViewPreset::Bottom);
                Ok(())
            }
            "view.camera_iso_2_1" | "view.pseudo_isometric_2to1" => {
                self.set_view_preset(petunia_core::ViewPreset::PseudoIsometric2To1);
                Ok(())
            }
            "view.camera_persp" => {
                self.set_view_preset(petunia_core::ViewPreset::Persp);
                Ok(())
            }
            "view.camera_opposite" => {
                self.toggle_view_opposite();
                Ok(())
            }
            "view.toggle_projection" => {
                self.apply(UiIntent::ToggleProjection);
                Ok(())
            }
            "view.isolate" => {
                self.state.toggle_isolate();
                Ok(())
            }
            other => self.state.dispatch_command(other),
        }
    }

    pub(crate) fn execute_shortcut_tool(&mut self, id: &str) {
        // Troca de domínio visível; nunca seleciona tudo (constituição 11).
        if self.state.selection_domain() == SelectionDomain::Object {
            match id {
                "model.extrude"
                | "model.extrude_individual"
                | "model.inset"
                | "model.push_pull" => {
                    self.state.set_selection_domain(SelectionDomain::Face);
                    self.sync_viewport_context();
                }
                "model.bevel" => {
                    self.state.set_selection_domain(SelectionDomain::Edge);
                    self.sync_viewport_context();
                }
                "model.loop_cut" => {
                    self.state.set_selection_domain(SelectionDomain::Edge);
                    self.sync_viewport_context();
                }
                _ => {}
            }
        }

        let now = std::time::Instant::now();
        let interval_ms = self.preferences.double_tap_interval_ms;

        let is_double_tap = if interval_ms > 0 {
            if let Some((ref last_tool, _, last_time)) = self.last_tool_press {
                last_tool == id && now.duration_since(last_time).as_millis() <= interval_ms as u128
            } else {
                false
            }
        } else {
            self.tool_modal.as_ref().is_some_and(|m| m.id() == id)
        };

        let kind = ToolModalKind::from_id(id);
        let is_instant_pref =
            self.state.session.tools.tool_activation == petunia_core::ToolActivation::Instant;

        if is_double_tap || is_instant_pref {
            self.last_tool_press = None;
            if self.tool_modal.is_none() {
                let _ = self.execute_core_command(id);
            }
            if let Some(open) = self.tool_modal {
                self.keyboard_tool_modal_active = true;
                // Gramática única: a operação segue o ponteiro numa sessão
                // travada; clique confirma, Esc cancela, dígitos digitam.
                let [px, py] = self.pointer_position;
                let _ = self
                    .tool_session
                    .begin_latched([px, py], petunia_core::PressTarget::Surface);
                if let Some(frame) = self.drag_frame() {
                    self.tool_gesture = Some(ToolGesture::Parametric {
                        kind: open,
                        frame,
                        anchor: [px, py],
                        start_value: self.tool_modal_value,
                    });
                }
                let label = kind.map_or_else(
                    || self.state.t_id(petunia_config::text_id::HUD_TOOL_FALLBACK),
                    |k| self.state.t(k.title_key()),
                );
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_FREE_MODE_MOVE_THE_MOUSE_CLICK_OR),
                    &[("0", label.to_string())],
                ));
            }
        } else if let Some(kind) = kind.filter(|kind| *kind != ToolModalKind::ScaleSelection) {
            // Gramática única: a tecla seleciona a ferramenta persistente; a
            // operação acontece pelo arrasto (ADR 007). O duplo toque continua
            // abrindo o modo que segue o ponteiro.
            let origin_tool = self.state.session.tools.active_tool.clone();
            self.last_tool_press = Some((id.to_string(), origin_tool, now));
            self.keyboard_tool_modal_active = false;
            self.activate_parametric_tool(kind);
        } else {
            let origin_tool = self.state.session.tools.active_tool.clone();
            self.last_tool_press = Some((id.to_string(), origin_tool, now));
            if self.tool_modal.is_none() {
                let _ = self.execute_core_command(id);
            }
            self.keyboard_tool_modal_active = false;
        }
    }

    pub fn route_shortcut(&mut self, text: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        if self.keymap_editor.capturing.is_some() {
            if text == "Escape" || text == "Esc" {
                return self.cancel_keymap_capture();
            }
            return self.capture_keymap_key(text, ctrl, shift, alt);
        }
        if (text == "Escape" || text == "Esc") && !ctrl && !alt && !shift {
            return self.handle_escape();
        }
        if text == "Enter" && !ctrl && !alt {
            return self.confirm_active_operation();
        }
        // Animate owns its authoring/transport keys before geometry fallbacks.
        if self.state.workspace.is_animate()
            && self.rename_draft.is_none()
            && !self.state.is_interacting()
            && self.drag.is_none()
        {
            let action = input::key_code_from_slint(text).and_then(|key| {
                self.state
                    .ui
                    .keybinds
                    .find_in_context(key, Mods2 { ctrl, shift, alt }, "animate")
            });
            let intent = match action {
                Some("animate.toggle_play") => Some(animate::AnimateIntent::TogglePlay),
                Some("animate.step_backward") => Some(animate::AnimateIntent::StepFrame(-1)),
                Some("animate.step_forward") => Some(animate::AnimateIntent::StepFrame(1)),
                Some("animate.insert_keyframe") => Some(animate::AnimateIntent::InsertKeyframe),
                Some("animate.delete_keyframe") => {
                    Some(animate::AnimateIntent::DeleteKeyframe(None))
                }
                Some("animate.jump_to_start") => Some(animate::AnimateIntent::JumpToStart),
                _ => None,
            };
            if let Some(intent) = intent {
                self.apply(UiIntent::Animate(intent));
                return true;
            }
            if matches!(text, "Delete" | "Backspace") {
                return false;
            }
        }
        if self.decal_gesture.is_some() && !ctrl && !alt {
            // Buffer numérico do gesto de decalque (constituição 11).
            if text == "Tab" {
                return self.decal_toggle_param();
            }
            // A tecla é do buffer mesmo quando o texto ainda não é um valor
            // aplicável (ex.: "0" antes de "0.5"): a prévia só não muda.
            if text == "Backspace" || text == "Delete" {
                self.tool_session.pop_numeric();
                self.decal_typed_input();
                return true;
            }
            if text.len() == 1
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+' | '*' | '/'))
            {
                self.tool_session.push_numeric(text);
                self.decal_typed_input();
                return true;
            }
        }
        if text == "F4" && !ctrl && !alt && !shift {
            return self.toggle_reference_manager();
        }
        if ctrl
            && !alt
            && text.eq_ignore_ascii_case("g")
            && self.state.workspace == Workspace::Model
        {
            return if shift {
                self.ungroup(None)
            } else {
                self.group_selection()
            };
        }
        if ctrl && !shift && !alt && text.eq_ignore_ascii_case("c") {
            return self.copy_selection();
        }
        if ctrl && !shift && !alt && text.eq_ignore_ascii_case("v") {
            return self.paste_clipboard();
        }
        if ((ctrl && !shift) || (shift && !ctrl)) && !alt && text.eq_ignore_ascii_case("d") {
            return self.duplicate_selection();
        }
        if text == "Backspace" && !ctrl && !alt && !shift && !self.poly_pen_points.is_empty() {
            // Collecting: Backspace remove o último ponto (constituição 11).
            self.poly_pen_points.pop();
            self.state.mark_dirty();
            return true;
        }
        if matches!(text, "Delete" | "Backspace")
            && !ctrl
            && !alt
            && !shift
            && self.state.session.tools.modal.is_some()
            && !self.tool_session.numeric_text().is_empty()
        {
            self.tool_session.pop_numeric();
            if self.tool_session.numeric_text().is_empty() {
                let [x, y] = self.pointer_position;
                self.update_viewport_transform(x, y);
            } else {
                self.preview_modal_text();
            }
            return true;
        }
        // Idle Delete/Backspace belongs to the effective keymap.
        if self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
        {
            // Atalhos de navegação da viewport (estilo Blender / Teclado Numérico)
            if let Some(key) = numpad_key(text) {
                let plain = !ctrl && !alt && !shift;
                let opposite = (ctrl || alt) && key.len() == 1;
                use petunia_core::ViewPreset as V;
                match key {
                    "1" if opposite => return self.set_view_preset(V::Back),
                    "1" if plain => return self.set_view_preset(V::Front),
                    "2" if plain => return self.set_view_preset(V::PseudoIsometric2To1),
                    "3" if opposite => return self.set_view_preset(V::Left),
                    "3" if plain => return self.set_view_preset(V::Right),
                    "7" if opposite => return self.set_view_preset(V::Bottom),
                    "7" if plain => return self.set_view_preset(V::Top),
                    "9" if plain => return self.toggle_view_opposite(),
                    "0" if plain => return self.set_view_preset(V::Persp),
                    "5" if plain => {
                        self.apply(UiIntent::ToggleProjection);
                        return true;
                    }
                    "." if plain => {
                        let _ = self.state.dispatch_command("view.frame_selection");
                        return true;
                    }
                    "/" if plain => {
                        self.state.toggle_isolate();
                        self.state.mark_dirty();
                        return true;
                    }
                    _ => {}
                }
            }
        }

        if (text == " " || text == "Space") && !ctrl && !alt && !shift {
            if self.state.session.tools.active_tool == "slice"
                && (self.slice_anchor.is_some() || self.state.session.tools.cut_session.is_some())
            {
                return self.commit_slice();
            }
            if (self.state.session.tools.active_tool == "cut"
                || self.state.session.tools.active_tool == "knife")
                && self.state.session.tools.cut_session.is_some()
            {
                return self.commit_knife();
            }
            if self.tool_modal.is_some() || self.loop_cut.is_some() || self.drag.is_some() {
                return self.confirm_active_operation();
            }
            // Ocioso: o keymap decide (`Space` = ferramenta anterior).
        }
        if alt && !ctrl && !shift {
            match text.to_ascii_lowercase().as_str() {
                "f" => return self.toggle_menu("file"),
                "e" => return self.toggle_menu("edit"),
                "v" => return self.toggle_menu("view"),
                "h" => return self.toggle_menu("help"),
                _ => {}
            }
        }
        if ctrl && !shift && !alt {
            match text {
                "PageUp" => {
                    let next = match self.state.workspace {
                        #[cfg(not(feature = "animation-workspace"))]
                        Workspace::Model => Workspace::Uv,
                        #[cfg(feature = "animation-workspace")]
                        Workspace::Model => Workspace::Animate,
                        Workspace::Paint => Workspace::Model,
                        Workspace::Uv => Workspace::Paint,
                        #[cfg(feature = "animation-workspace")]
                        Workspace::Animate => Workspace::Uv,
                    };
                    self.apply(UiIntent::SetWorkspace(next));
                    return true;
                }
                "PageDown" => {
                    let next = match self.state.workspace {
                        Workspace::Model => Workspace::Paint,
                        Workspace::Paint => Workspace::Uv,
                        #[cfg(not(feature = "animation-workspace"))]
                        Workspace::Uv => Workspace::Model,
                        #[cfg(feature = "animation-workspace")]
                        Workspace::Uv => Workspace::Animate,
                        #[cfg(feature = "animation-workspace")]
                        Workspace::Animate => Workspace::Model,
                    };
                    self.apply(UiIntent::SetWorkspace(next));
                    return true;
                }
                _ => {}
            }
        }
        if self.state.session.tools.active_tool == "slice"
            && text.eq_ignore_ascii_case("t")
            && !ctrl
            && !alt
        {
            let next_trim = !self.slice_trim;
            self.set_slice_trim(next_trim);
            self.state.set_status(if next_trim {
                "Slice: Modo Trim (Remove o lado cortado) · Enter para confirmar"
            } else {
                "Slice: Modo Split (Mantém ambos os lados) · Enter para confirmar"
            });
            return true;
        }
        if text.eq_ignore_ascii_case("u")
            && !ctrl
            && !alt
            && !shift
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && (self.state.workspace == Workspace::Uv
                || self.state.session.selection_domain == SelectionDomain::Edge)
            && self.toggle_selected_uv_seams()
        {
            return true;
        }
        if text.eq_ignore_ascii_case("p")
            && !ctrl
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && self.state.workspace == Workspace::Uv
        {
            if alt {
                if self.clear_all_uv_pins() {
                    return true;
                }
            } else if self.toggle_selected_uv_pins() {
                return true;
            }
        }

        if self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && !alt
        {
            match text {
                "Left" | "Right" | "Up" | "Down" => {
                    if self.state.workspace == Workspace::Model {
                        let step = if shift {
                            0.01
                        } else if ctrl {
                            1.0
                        } else {
                            0.1
                        };
                        let (dx, dy, dz) = match text {
                            "Left" => (-step, 0.0, 0.0),
                            "Right" => (step, 0.0, 0.0),
                            "Up" => (0.0, step, 0.0),
                            "Down" => (0.0, -step, 0.0),
                            _ => (0.0, 0.0, 0.0),
                        };
                        if self.nudge_selection(dx, dy, dz) {
                            return true;
                        }
                    } else if self.state.workspace == Workspace::Uv {
                        let step = if shift {
                            0.002
                        } else if ctrl {
                            0.05
                        } else {
                            0.01
                        };
                        let (du, dv) = match text {
                            "Left" => (-step, 0.0),
                            "Right" => (step, 0.0),
                            "Up" => (0.0, step),
                            "Down" => (0.0, -step),
                            _ => (0.0, 0.0),
                        };
                        if self.uv_move_selected(du, dv) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        if self.state.session.tools.modal.is_some() && !ctrl && !alt {
            if let Some(axis) = ["x", "y", "z"]
                .iter()
                .position(|axis| text.eq_ignore_ascii_case(axis))
            {
                let requested = if shift {
                    petunia_core::ModalConstraint::Plane(axis)
                } else {
                    petunia_core::ModalConstraint::Axis(axis)
                };
                let current = self
                    .state
                    .session
                    .tools
                    .modal
                    .as_ref()
                    .map(|op| op.constraint);
                let constraint = if current == Some(requested) {
                    petunia_core::ModalConstraint::Free
                } else {
                    requested
                };
                let _ = self.state.set_modal_constraint(constraint);
                if let Some(drag) = self.drag.as_mut() {
                    drag.rotation_angle = 0.0;
                    drag.last_angle = 0.0;
                }
                if !self.tool_session.numeric_text().is_empty() {
                    self.preview_modal_text();
                } else {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                }
                return true;
            }
            if text == "Backspace" {
                self.tool_session.pop_numeric();
                if self.tool_session.numeric_text().is_empty() {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                } else {
                    self.preview_modal_text();
                }
                return true;
            }
            if text.len() == 1
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))
            {
                self.tool_session.push_numeric(text);
                self.preview_modal_text();
                return true;
            }
        }
        let Some(key) = input::key_code_from_slint(text) else {
            return false;
        };
        let mods = Mods2 { ctrl, shift, alt };
        let context = match self.state.workspace {
            Workspace::Model => "model",
            Workspace::Paint => "paint",
            Workspace::Uv => "uv",
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => "animate",
        };
        let Some(action) = self
            .state
            .ui
            .keybinds
            .find_in_context(key, mods, context)
            .map(str::to_owned)
        else {
            return false;
        };
        match action.as_str() {
            "uv.move" => self.apply(UiIntent::BeginUvTransform(
                petunia_core::UvTransformKind::Move,
            )),
            "uv.rotate" => self.apply(UiIntent::BeginUvTransform(
                petunia_core::UvTransformKind::Rotate,
            )),
            "uv.scale" => self.apply(UiIntent::BeginUvTransform(
                petunia_core::UvTransformKind::Scale,
            )),
            "global.undo" => self.apply(UiIntent::Undo),
            "global.redo" => self.apply(UiIntent::Redo),
            "global.reset_camera" => self.apply(UiIntent::ResetCamera),
            "global.toggle_projection" => self.apply(UiIntent::ToggleProjection),
            "global.toggle_wireframe" | "view.toggle_wireframe" => {
                if let Err(error) = self.execute_core_command("view.toggle_wireframe") {
                    self.state.set_status(error.to_string());
                }
            }
            "model.select_object" => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object))
            }
            "model.select_vertex" => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex))
            }
            "model.select_edge" => self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge)),
            "model.select_face" => self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face)),
            "model.tool_select" | "model.select" | "model.select_tool" => {
                self.apply(UiIntent::SetActiveTool("select".into()));
            }
            "model.box_select" => self.apply(UiIntent::SetActiveTool("box_select".into())),
            "model.move" | "model.transform" => {
                self.begin_keyboard_transform(TransformKind::Position)
            }
            "model.rotate" => self.begin_keyboard_transform(TransformKind::Rotation),
            "model.scale" => self.begin_keyboard_transform(TransformKind::Scale),
            "model.make_face" => {
                let _ = self.execute_core_command("model.make_face");
            }
            "model.frame_selection" => {
                let _ = self.execute_core_command("view.frame_selection");
            }
            "model.toggle_edit_pivot" => self.apply(UiIntent::ToggleEditPivot),
            "model.extrude" => {
                if self.state.session.tools.active_tool == "draw_profile"
                    && (self.active_profile_closed() || self.active_profile_point_count() >= 3)
                {
                    if self.profile_volume_mode
                        == Some(petunia_module_model::ProfileVolumeMode::Extrude)
                    {
                        self.commit_profile_volume();
                    } else {
                        self.enter_profile_volume("extrude");
                    }
                } else {
                    self.execute_shortcut_tool("model.extrude");
                }
            }
            "model.inset" => {
                self.execute_shortcut_tool("model.inset");
            }
            "model.bevel" => {
                self.execute_shortcut_tool("model.bevel");
            }
            "model.delete" => {
                if self.state.session.tools.active_tool == "draw_profile" {
                    self.delete_selected_profile_point();
                } else if self.state.selection_domain().is_component() {
                    self.delete_or_dissolve_selection();
                } else {
                    self.apply(UiIntent::DeleteActiveAsset);
                }
            }
            "model.push_pull" => {
                self.execute_shortcut_tool("model.push_pull");
            }
            "model.knife" => {
                let _ = self.execute_core_command("model.knife");
            }
            "model.extrude_individual" => {
                self.execute_shortcut_tool("model.extrude_individual");
            }
            "model.subdivide" => {
                let _ = self.execute_core_command("model.subdivide");
            }
            "model.merge" => {
                let _ = self.execute_core_command("model.merge");
            }
            "model.loop_cut" => {
                self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
            }
            "model.measure" => {
                self.apply(UiIntent::SetActiveTool("measure".to_string()));
            }
            "model.primitives" => {
                self.set_add_menu_open(true);
            }
            "global.previous_tool" => return self.swap_to_previous_tool(),
            "global.micro_inspector" => return self.toggle_micro_inspector(),
            "window.workspace_draw" => {
                self.apply(UiIntent::SetModelingMode(ModelingMode::Draw));
            }
            "window.workspace_poly" => {
                self.apply(UiIntent::SetModelingMode(ModelingMode::Poly));
            }
            "window.workspace_paint" => self.apply(UiIntent::SetWorkspace(Workspace::Paint)),
            "window.workspace_uv" => self.apply(UiIntent::SetWorkspace(Workspace::Uv)),
            #[cfg(feature = "animation-workspace")]
            "window.workspace_animate" => self.apply(UiIntent::SetWorkspace(Workspace::Animate)),
            "paint.select" => self.apply(UiIntent::SetActiveTool("select".into())),
            "paint.isolate" => {
                self.toggle_paint_isolate();
            }
            "paint.airbrush" => self.apply(UiIntent::SetActiveTool("airbrush".into())),
            "paint.eraser" => self.apply(UiIntent::SetActiveTool("eraser".into())),
            "paint.color_picker" => self.apply(UiIntent::SetActiveTool("picker".into())),
            "paint.fill" => self.apply(UiIntent::SetActiveTool("fill".into())),
            "paint.gradient" => self.apply(UiIntent::SetActiveTool("gradient".into())),
            "paint.gradient_radial" => {
                self.apply(UiIntent::SetActiveTool("gradient_radial".into()))
            }
            "paint.ellipse" => self.apply(UiIntent::SetActiveTool("ellipse".into())),
            "paint.line" => self.apply(UiIntent::SetActiveTool("line".into())),
            "paint.rectangle" => self.apply(UiIntent::SetActiveTool("rectangle".into())),
            "paint.paint" => self.apply(UiIntent::SetActiveTool("brush".into())),
            "paint.size_decrease" => self.adjust_brush_size(-1.0),
            "paint.size_increase" => self.adjust_brush_size(1.0),
            "paint.hardness_decrease" => self.adjust_brush_hardness(-0.1),
            "paint.hardness_increase" => self.adjust_brush_hardness(0.1),
            "global.cycle_mode" => {
                let _ = self.execute_core_command("select.cycle_domain");
            }
            "global.rename" => {
                self.begin_rename();
            }
            "model.slice" => {
                // A ação do keymap só arma a ferramenta; a âncora nasce no
                // pointer-down da viewport.
                self.apply(UiIntent::SetActiveTool("slice".to_string()));
            }
            "model.draw_profile" => {
                self.apply(UiIntent::SetActiveTool("draw_profile".to_string()));
            }
            "model.shape_builder" => {
                self.apply(UiIntent::SetActiveTool("shape_builder".to_string()));
            }
            "draw.unite" => return self.run_pathfinder(petunia_core::PathfinderOp::Unite),
            "draw.subtract" => return self.run_pathfinder(petunia_core::PathfinderOp::Subtract),
            "draw.intersect" => {
                return self.run_pathfinder(petunia_core::PathfinderOp::Intersect);
            }
            "draw.exclude" => return self.run_pathfinder(petunia_core::PathfinderOp::Exclude),
            "global.save_project" => self.apply(UiIntent::SaveProject),
            "global.help" => {
                let _ = self.execute_core_command("help.documentation");
            }
            other
                if other.starts_with("view.")
                    || other.starts_with("model.")
                    || other.starts_with("uv.")
                    || other.starts_with("draw.")
                    || other.starts_with("paint.") =>
            {
                if let Err(error) = self.execute_core_command(other) {
                    self.state.set_status(error.to_string());
                }
            }
            "window.command_palette" => self.apply(UiIntent::OpenCommandSearch),
            _ => return false,
        }
        true
    }

    /// `paint.isolate` (C2): no PAINT, isolar é fantasma (Raio-X) e preserva o
    /// Raio-X do usuário; nos outros workspaces é o isolamento comum. Exposto
    /// também como ação de request para o controle da seção Projeção.
    pub fn toggle_paint_isolate(&mut self) -> bool {
        if self.state.workspace == Workspace::Paint {
            self.state.toggle_paint_isolate_ghost();
        } else {
            self.state.toggle_isolate();
        }
        true
    }
}
