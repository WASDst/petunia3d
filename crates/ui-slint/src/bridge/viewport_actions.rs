//! Execução de DTOs de viewport no bridge; adapters Slint só validam/encaminham.
use crate::viewport_intents::{
    PhysicalPointerModifiers, ViewportHoverIntent, ViewportSelectionIntent,
    ViewportToolPointerIntent,
};
use crate::{
    GrammarTool, PARAMETRIC_HANDLE_TARGET, PetuniaViewport, SlintUiBridge, ViewportPointerPhase,
};
use petunia_config::keybinds::Mods2;
use petunia_core::SelectionDomain;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn apply_viewport_selection(&mut self, intent: ViewportSelectionIntent) {
        match intent {
            ViewportSelectionIntent::Point {
                at,
                extend,
                loop_select,
            } => {
                let [x, y] = at.xy();
                self.select_viewport_ext(x, y, extend, loop_select);
            }
            ViewportSelectionIntent::Box { from, to, mode } => {
                let (add, subtract) = mode.flags();
                self.state
                    .select_viewport_box(from.ndc(), to.ndc(), add, subtract);
                let summary = match self.state.selection_domain() {
                    SelectionDomain::Object => {
                        let total = self.state.session.selection.assets.len();
                        if total == 0 {
                            "Box select: nothing in the region".to_string()
                        } else {
                            format!("Box select: {total} object(s)")
                        }
                    }
                    SelectionDomain::Vertex | SelectionDomain::Edge | SelectionDomain::Face => {
                        match self.state.project.active_mesh() {
                            Some(mesh) => {
                                let points = mesh.verts.iter().filter(|v| v.selected).count();
                                let faces = mesh.faces.iter().filter(|f| f.selected).count();
                                let edges = mesh.selected_edges.len();
                                match self.state.selection_domain() {
                                    SelectionDomain::Vertex => {
                                        if points == 0 {
                                            "Box select: nothing in the region".to_string()
                                        } else {
                                            format!("Box select: {points} point(s)")
                                        }
                                    }
                                    SelectionDomain::Edge => {
                                        if edges == 0 {
                                            "Box select: nothing in the region".to_string()
                                        } else {
                                            format!("Box select: {edges} edge(s)")
                                        }
                                    }
                                    _ => {
                                        if faces == 0 {
                                            "Box select: nothing in the region".to_string()
                                        } else {
                                            format!("Box select: {faces} face(s)")
                                        }
                                    }
                                }
                            }
                            None => "Box select: no active object".to_string(),
                        }
                    }
                };
                self.state.set_status(summary);
            }
            ViewportSelectionIntent::Lasso { polygon, mode } => {
                let (add, subtract) = mode.flags();
                self.state
                    .select_viewport_lasso(polygon.ndc_points(), add, subtract);
                let message = self
                    .state
                    .t_id(petunia_config::text_id::STATUS_LASSO_SELECTION_UPDATED);
                self.state.set_status(message);
            }
        }
    }

    pub fn apply_viewport_hover(&mut self, intent: ViewportHoverIntent) -> bool {
        match intent {
            ViewportHoverIntent::Clear => self.clear_hover(),
            ViewportHoverIntent::At(at) => {
                let [x, y] = at.xy();
                if self.state.session.tools.active_tool == "loop_cut" {
                    let [width, height] = self.viewport_size;
                    self.update_loop_cut_hover(x * width, y * height)
                } else {
                    crate::perf::measure("hover_component", || self.hover_component(x, y))
                }
            }
        }
    }

    pub fn tool_pointer_intent(&mut self, intent: ViewportToolPointerIntent) -> bool {
        let ViewportToolPointerIntent {
            phase,
            at,
            modifiers,
        } = intent;
        let [x, y] = at.xy();
        let PhysicalPointerModifiers { shift, ctrl, alt } = modifiers;

        puffin::profile_function!();
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        self.pointer_position = [x, y];
        self.tool_pointer_alt = alt;

        // Os booleans recebidos aqui descrevem teclas físicas pressionadas.
        // O significado (precision/snap/extend/alternate) vem do keymap ativo.
        let held = Mods2 { ctrl, shift, alt };
        let precision = self
            .state
            .ui
            .keybinds
            .pointer_modifier(petunia_config::keybinds::POINTER_PRECISION)
            .held_in(held);
        let snap = self
            .state
            .ui
            .keybinds
            .pointer_modifier(petunia_config::keybinds::POINTER_SNAP)
            .held_in(held);
        let extend = self
            .state
            .ui
            .keybinds
            .pointer_modifier(petunia_config::keybinds::POINTER_EXTEND)
            .held_in(held);
        let alternate = self
            .state
            .ui
            .keybinds
            .pointer_modifier(petunia_config::keybinds::POINTER_ALTERNATE)
            .held_in(held);

        let effect = match phase {
            ViewportPointerPhase::Press => {
                self.tool_press_parametric_handle = self.parametric_handle_at([x, y]);
                let target = if self.grammar_tool() == Some(GrammarTool::Decal) {
                    self.decal_press_target(x, y)
                } else if self.grammar_tool() == Some(GrammarTool::DrawProfile) {
                    self.profile_press_target(x, y)
                } else if self.tool_press_parametric_handle {
                    petunia_core::PressTarget::Handle(PARAMETRIC_HANDLE_TARGET)
                } else if self.profile_transform_target_at(x, y) {
                    petunia_core::PressTarget::Handle(0)
                } else if self.gizmo_target_at(x, y).is_some() {
                    petunia_core::PressTarget::Handle(1)
                } else {
                    petunia_core::PressTarget::Surface
                };
                self.tool_press_extend = extend;
                self.tool_press_alternate = alternate;
                self.tool_session.press([x, y], target)
            }
            ViewportPointerPhase::Move => self.tool_session.move_to([x, y]),
            ViewportPointerPhase::Release => self.tool_session.release([x, y]),
            ViewportPointerPhase::Cancel => {
                if self.tool_session.is_gesture_active() {
                    self.tool_session.key(petunia_core::ToolKey::Cancel)
                } else {
                    self.tool_session.reset();
                    petunia_core::ToolEffect::Nothing
                }
            }
        };
        self.apply_tool_effect(effect, precision, snap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport_intents::{
        NormalizedViewportPoint, RegionSelectionMode, ViewportLassoPolygon,
    };
    use crate::{PlaceholderViewport, UiIntent};

    #[test]
    fn typed_regions_select_add_subtract_and_lasso_without_rebuilding_geometry() {
        let mut bridge = SlintUiBridge::new(
            petunia_core::AppState::default(),
            PlaceholderViewport::default(),
        );
        bridge
            .execute_core_command("model.add_cube")
            .expect("cube fixture");
        let vertices_before = bridge.state.scene_verts();
        let objects_before = bridge.state.project.assets.len();
        let from = NormalizedViewportPoint::new(0.0, 0.0).unwrap();
        let to = NormalizedViewportPoint::new(1.0, 1.0).unwrap();
        bridge.apply(UiIntent::ViewportSelection(ViewportSelectionIntent::Box {
            from,
            to,
            mode: RegionSelectionMode::Replace,
        }));
        let selected = bridge.state.session.selection.assets.clone();
        assert!(
            !selected.is_empty(),
            "real viewport query must find the cube"
        );
        bridge.apply(UiIntent::ViewportSelection(ViewportSelectionIntent::Box {
            from,
            to,
            mode: RegionSelectionMode::Add,
        }));
        assert_eq!(
            bridge.state.session.selection.assets, selected,
            "add must not duplicate IDs"
        );
        bridge.apply(UiIntent::ViewportSelection(ViewportSelectionIntent::Box {
            from,
            to,
            mode: RegionSelectionMode::Subtract,
        }));
        assert!(bridge.state.session.selection.assets.is_empty());
        bridge.apply(UiIntent::ViewportSelection(
            ViewportSelectionIntent::Lasso {
                polygon: ViewportLassoPolygon::from_slint_path("0,0;1,0;1,1;0,1").unwrap(),
                mode: RegionSelectionMode::Replace,
            },
        ));
        assert_eq!(
            bridge.state.session.selection.assets, selected,
            "lasso shares the existing selection query"
        );
        assert_eq!(bridge.state.scene_verts(), vertices_before);
        assert_eq!(bridge.state.project.assets.len(), objects_before);
    }
}
