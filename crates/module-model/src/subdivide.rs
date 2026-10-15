use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct SubdivideTool;

impl Tool for SubdivideTool {
    fn id(&self) -> &'static str {
        "subdivide"
    }
    fn label_key(&self) -> &'static str {
        "tools.subdivide"
    }
    fn hint_key(&self) -> &'static str {
        "hints.subdivide"
    }
    fn icon(&self) -> &'static str {
        "⊞"
    }
    fn shortcut(&self) -> &'static str {
        "W"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.set_status(state.t("hints.subdivide"));
        state.mark_dirty();
    }
}

impl SubdivideTool {
    pub fn apply_subdivide(state: &mut AppState) {
        if let Err(err) = state.dispatch(&petunia_core::SubdivideSelectionCmd) {
            state.set_status(format!("subdivide: {err}"));
        }
    }

    pub fn apply_triangulate(state: &mut AppState) {
        if let Err(err) = state.dispatch(&petunia_core::TriangulateMeshCmd) {
            state.set_status(format!("triangulate: {err}"));
        }
    }
}
