use petunia_core::{AppState, WeldCmd};

use super::Tool;

#[derive(Default)]
pub struct MergeTool;
impl Tool for MergeTool {
    fn id(&self) -> &'static str {
        "merge"
    }
    fn label_key(&self) -> &'static str {
        "tools.merge"
    }
    fn hint_key(&self) -> &'static str {
        "hints.merge"
    }
    fn icon(&self) -> &'static str {
        "⛭"
    }
    fn shortcut(&self) -> &'static str {
        "M"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.set_status(state.t("hints.merge"));
        state.mark_dirty();
    }
}

impl MergeTool {
    pub fn apply(state: &mut AppState) {
        if let Err(err) = state.dispatch(&petunia_core::MergeCenterCmd) {
            state.set_status(format!("merge: {err}"));
        }
    }

    pub fn apply_by_distance(state: &mut AppState) {
        let cmd = WeldCmd {
            eps: state.merge_dist.max(0.0),
        };
        if let Err(err) = state.dispatch(&cmd) {
            state.set_status(format!("merge by distance: {err}"));
        }
    }
}
