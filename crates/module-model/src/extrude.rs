use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct ExtrudeTool;

impl Tool for ExtrudeTool {
    fn id(&self) -> &'static str {
        "extrude"
    }
    fn label_key(&self) -> &'static str {
        "tools.extrude"
    }
    fn hint_key(&self) -> &'static str {
        "hints.extrude"
    }
    fn icon(&self) -> &'static str {
        "⬆"
    }
    fn shortcut(&self) -> &'static str {
        "E"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.pending_modal = Some(petunia_core::ModalKind::Extrude);
        state.mark_dirty();
    }
}

impl ExtrudeTool {
    pub fn apply(state: &mut AppState) {
        let d = state.extrude_dist;
        let cmd = petunia_core::ExtrudeSelectedCmd {
            dist: d,
            // `None`: usa o modo escolhido na ferramenta (C8).
            mode: None,
        };
        if let Err(err) = state.dispatch(&cmd) {
            state.set_status(format!("extrude: {err}"));
        }
    }

    pub fn apply_individual(state: &mut AppState) {
        let d = state.extrude_dist;
        let cmd = petunia_core::ExtrudeIndividualCmd { dist: d };
        if let Err(err) = state.dispatch(&cmd) {
            state.set_status(format!("extrude individual: {err}"));
        }
    }
}
