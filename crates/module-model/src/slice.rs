use glam::Vec3;
use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct SliceTool;

impl Tool for SliceTool {
    fn id(&self) -> &'static str {
        "slice"
    }
    fn label_key(&self) -> &'static str {
        "tools.slice"
    }
    fn hint_key(&self) -> &'static str {
        "hints.slice"
    }
    fn icon(&self) -> &'static str {
        "✂"
    }
    fn shortcut(&self) -> &'static str {
        "Shift+K"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.set_status(state.t("hints.slice"));
    }
}

impl SliceTool {
    pub fn apply_slice(state: &mut AppState, normal: Vec3, cap: bool) {
        let cmd = petunia_core::SlicePlaneCmd {
            normal: normal.to_array(),
            cap,
        };
        if let Err(err) = state.dispatch(&cmd) {
            state.set_status(format!("slice: {err}"));
        }
    }
}
