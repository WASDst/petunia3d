//! Estado base do shell: workspace, status, ferramenta, shading e seleção.

use crate::callbacks::*;

pub(super) fn sync_shell(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_active_workspace(vm.workspace_label().into());
    crate::animate::sync_animate_properties(window, &vm.animate);
    window.set_saved(vm.saved);
    window.set_can_undo(vm.can_undo);
    window.set_can_redo(vm.can_redo);
    window.set_shortcuts((&vm.shortcuts).into());
    window.set_status_severity(vm.status_severity);
    window.set_status_message(vm.status_message.as_str().into());
    window.set_active_tool(vm.active_tool.as_str().into());
    let domain_str = match vm.selection_domain {
        SelectionDomain::Object => "OBJECT",
        SelectionDomain::Vertex => "POINT",
        SelectionDomain::Edge => "EDGE",
        SelectionDomain::Face => "FACE",
    };
    window.set_selection_domain(domain_str.into());
    window.set_is_orthographic(vm.is_orthographic);
    window.set_is_wireframe(vm.is_wireframe);
    window.set_shading_mode(vm.shading_mode.as_str().into());
    window.set_show_xray(vm.show_xray);
    window.set_shading_popover_open(vm.shading_popover_open);
    window.set_xray_opacity(vm.xray_opacity);
    for state in &vm.section_states {
        match state.id.as_str() {
            "parts" => {
                window.set_model_parts_open(state.open);
            }
            "transform" => {
                window.set_model_transform_open(state.open);
            }
            "material" => {
                window.set_model_material_open(state.open);
            }
            "object" => {
                window.set_model_object_open(state.open);
            }
            "modifiers" => {
                window.set_model_modifiers_open(state.open);
            }
            "quick_actions" => {
                window.set_quick_actions_section_open(state.open);
            }
            _ => {}
        }
    }
}
