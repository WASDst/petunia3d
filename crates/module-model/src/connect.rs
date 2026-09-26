use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct ConnectTool;

impl Tool for ConnectTool {
    fn id(&self) -> &'static str {
        "connect"
    }
    fn label_key(&self) -> &'static str {
        "tools.connect"
    }
    fn hint_key(&self) -> &'static str {
        "hints.connect"
    }
    fn icon(&self) -> &'static str {
        "☍"
    }
    fn shortcut(&self) -> &'static str {
        "Ctrl+J"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.set_status(state.t("hints.connect"));
    }
}

impl ConnectTool {
    pub fn apply(state: &mut AppState) {
        let sel_faces: Vec<usize> = if let Some(m) = state.project.active_mesh() {
            m.faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.selected)
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        };

        if sel_faces.len() == 2 {
            if let Err(err) = state.dispatch(&petunia_core::ConnectLoopsCmd::default()) {
                state.set_status(format!("{}: {err}", state.t("status.connect_err")));
            }
        } else {
            state.set_status(state.t("status.connect_need_2"));
        }
    }
}
