use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct TransformTool;

impl Tool for TransformTool {
    fn id(&self) -> &'static str {
        "transform"
    }
    fn label_key(&self) -> &'static str {
        "tools.transform"
    }
    fn hint_key(&self) -> &'static str {
        "hints.transform"
    }
    fn icon(&self) -> &'static str {
        "✥"
    }
    fn shortcut(&self) -> &'static str {
        "T"
    }
    fn on_activate(&self, state: &mut AppState) {
        // Universal Transform is a persistent gizmo mode, not an implicit Move
        // transaction. G/R/S still start the individual modal operations.
        state.pending_modal = None;
        state.set_status(state.t("hints.transform"));
        state.mark_dirty();
    }
}

impl TransformTool {
    pub fn apply_move(state: &mut AppState) {
        let d = state.transform_delta;
        if let Err(err) = state.dispatch(&petunia_core::TranslateSelectionCmd { delta: d }) {
            state.set_status(format!("move: {err}"));
        }
    }

    pub fn apply_scale(state: &mut AppState) {
        let s = state.transform_scale;
        if let Err(err) = state.dispatch(&petunia_core::ScaleSelectionCmd { factor: s }) {
            state.set_status(format!("scale: {err}"));
        }
    }

    pub fn apply_duplicate(state: &mut AppState) {
        if let Err(err) = state.dispatch(&petunia_core::DuplicateSelectionCmd) {
            state.set_status(format!("duplicate: {err}"));
        }
    }

    pub fn apply_delete(state: &mut AppState) {
        if let Err(err) = state.dispatch(&petunia_core::DeleteSelectionCmd) {
            state.set_status(format!("delete: {err}"));
        }
    }
}
