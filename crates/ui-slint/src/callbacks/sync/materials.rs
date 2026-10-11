//! Materiais, quick actions, modificadores e seções.

use crate::callbacks::*;

pub(super) fn sync_materials(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_active_material_name(vm.active_material_name.as_str().into());
    let material_slots: Vec<slint::SharedString> = vm
        .material_slots
        .iter()
        .map(|slot| slot.as_str().into())
        .collect();
    window.set_material_slots(material_slots.as_slice().into());
    window.set_active_material_slot(vm.active_material_slot);
    window.set_material_has_selection(vm.material_has_selection);
    window.set_material_id(vm.material_id.as_str().into());
    window.set_material_name(vm.material_name.as_str().into());
    window.set_material_profile(vm.material_profile.as_str().into());
    window.set_material_profile_label(vm.material_profile_label.as_str().into());
    window.set_material_base_color(slint::Color::from_argb_f32(
        1.0,
        vm.material_base_color[0],
        vm.material_base_color[1],
        vm.material_base_color[2],
    ));
    let material_palette: Vec<slint::Color> = vm
        .material_palette
        .iter()
        .map(|color| slint::Color::from_argb_f32(1.0, color[0], color[1], color[2]))
        .collect();
    window.set_material_palette(material_palette.as_slice().into());
    window.set_material_roughness(vm.material_roughness);
    window.set_material_metallic(vm.material_metallic);
    window.set_material_normal_scale(vm.material_normal_scale);
    window.set_material_emission(slint::Color::from_argb_f32(
        1.0,
        vm.material_emission[0],
        vm.material_emission[1],
        vm.material_emission[2],
    ));
    window.set_material_emission_strength(vm.material_emission_strength);
    window.set_material_alpha_mode(vm.material_alpha_mode.as_str().into());
    window.set_material_alpha_cutoff(vm.material_alpha_cutoff);
    window.set_material_has_albedo(vm.material_has_albedo);
    window.set_material_albedo_label(vm.material_albedo_label.as_str().into());

    let quick_actions: Vec<QuickActionEntry> = vm
        .quick_actions
        .iter()
        .map(|action| QuickActionEntry {
            id: action.id.as_str().into(),
            label: action.label.as_str().into(),
            enabled: action.enabled,
            pinned: action.pinned,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_quick_actions(), quick_actions, |model| {
        window.set_quick_actions(model)
    });
    let quick_action_candidates: Vec<QuickActionEntry> = vm
        .quick_action_candidates
        .iter()
        .map(|action| QuickActionEntry {
            id: action.id.as_str().into(),
            label: action.label.as_str().into(),
            enabled: action.enabled,
            pinned: action.pinned,
        })
        .collect();
    crate::refresh::set_model_if_changed(
        window.get_quick_action_candidates(),
        quick_action_candidates,
        |model| window.set_quick_action_candidates(model),
    );
    let modifier_rows: Vec<ModifierEntry> = vm
        .modifier_rows
        .iter()
        .map(|modifier| ModifierEntry {
            id: modifier.id.as_str().into(),
            title: modifier.title.as_str().into(),
            subtitle: modifier.subtitle.as_str().into(),
            enabled: modifier.enabled,
            kind: modifier.kind.as_str().into(),
            axis: modifier.axis,
            positive_to_negative: modifier.positive_to_negative,
            can_move_up: modifier.can_move_up,
            can_move_down: modifier.can_move_down,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_modifier_rows(), modifier_rows, |model| {
        window.set_modifier_rows(model)
    });
    let section_states: Vec<SectionState> = vm
        .section_states
        .iter()
        .map(|state| SectionState {
            id: state.id.as_str().into(),
            pin_open: state.pin_open,
            open: state.open,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_section_states(), section_states, |model| {
        window.set_section_states(model)
    });
    window.set_active_asset_is_parametric(vm.active_asset_is_parametric);
}
