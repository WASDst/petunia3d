//! Sincronização do `ShellViewModel` com as propriedades da janela Slint.
//!
//! Cada submódulo cobre uma área do shell; a ordem de chamada abaixo preserva a
//! ordem relativa original de primeira ocorrência.

use crate::*;

mod labels;
mod materials;
mod modeling;
mod outliner;
mod paint;
mod settings;
mod shell;
mod transform_state;
mod uv_references;
mod viewport_hud;

pub(crate) fn sync_window_properties(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    puffin::profile_function!();
    shell::sync_shell(window, vm);
    viewport_hud::sync_viewport_hud(window, vm);
    outliner::sync_outliner(window, vm);
    paint::sync_paint(window, vm);
    transform_state::sync_transform_state(window, vm);
    materials::sync_materials(window, vm);
    uv_references::sync_uv_references(window, vm);
    labels::sync_labels(window, vm);
    modeling::sync_modeling(window, vm);
    settings::sync_settings(window, vm);
}
