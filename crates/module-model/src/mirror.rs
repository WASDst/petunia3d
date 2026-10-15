use petunia_core::AppState;

use super::Tool;

/// Mirror simples e previsível (§8.11): eixo + weld no plano.
#[derive(Default)]
pub struct MirrorTool;

impl Tool for MirrorTool {
    fn id(&self) -> &'static str {
        "mirror"
    }
    fn label_key(&self) -> &'static str {
        "tools.mirror"
    }
    fn hint_key(&self) -> &'static str {
        "hints.mirror"
    }
    fn icon(&self) -> &'static str {
        "◧"
    }
    fn shortcut(&self) -> &'static str {
        "Ctrl+M"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.set_status(state.t("hints.mirror"));
        state.mark_dirty();
    }
}

impl MirrorTool {
    pub fn apply(state: &mut AppState) {
        let cmd = petunia_core::MirrorMeshCmd {
            axis: state.mirror_axis,
            weld: state.mirror_weld,
        };
        if let Err(err) = state.dispatch(&cmd) {
            state.set_status(format!("mirror: {err}"));
        }
    }
}
