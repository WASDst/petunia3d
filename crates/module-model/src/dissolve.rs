use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct DissolveTool;

impl Tool for DissolveTool {
    fn id(&self) -> &'static str {
        "dissolve"
    }
    fn label_key(&self) -> &'static str {
        "tools.dissolve"
    }
    fn hint_key(&self) -> &'static str {
        "hints.dissolve"
    }
    fn icon(&self) -> &'static str {
        "⌫"
    }
    fn shortcut(&self) -> &'static str {
        "X"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.set_status(state.t("hints.dissolve"));
    }
}

impl DissolveTool {
    pub fn apply(state: &mut AppState) {
        if let Err(err) = state.dispatch(&petunia_core::DissolveCmd) {
            state.set_status(format!("dissolve: {err}"));
        }
    }
}
