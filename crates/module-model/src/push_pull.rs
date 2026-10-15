use petunia_core::AppState;

use super::Tool;

/// Push/Pull (§8.7): empurra/puxa faces ao longo da normal, resposta imediata.
#[derive(Default)]
pub struct PushPullTool;

impl Tool for PushPullTool {
    fn id(&self) -> &'static str {
        "pushpull"
    }
    fn label_key(&self) -> &'static str {
        "tools.pushpull"
    }
    fn hint_key(&self) -> &'static str {
        "hints.pushpull"
    }
    fn icon(&self) -> &'static str {
        "⇅"
    }
    fn shortcut(&self) -> &'static str {
        "P"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.pending_modal = Some(petunia_core::ModalKind::PushPull);
        state.mark_dirty();
    }
}

impl PushPullTool {
    pub fn apply(state: &mut AppState) {
        let cmd = petunia_core::PushPullApplyCmd {
            dist: state.push_dist,
        };
        if let Err(err) = state.dispatch(&cmd) {
            state.set_status(format!("push/pull: {err}"));
        }
    }
}
