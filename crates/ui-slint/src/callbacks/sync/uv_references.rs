//! UV e gerenciador de referências.

use crate::callbacks::*;

pub(super) fn sync_uv_references(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_uv_stats(vm.uv_stats.as_str().into());
    window.set_reference_manager_visible(vm.reference_manager_open);
    window.set_reference_total_count(vm.reference_total_count);
    let ref_slots: Vec<ReferenceSlotData> = vm
        .reference_slots
        .iter()
        .map(|s| ReferenceSlotData {
            axis: s.axis.as_str().into(),
            axis_tag: s.axis_tag.as_str().into(),
            title: s.title.as_str().into(),
            has_image: s.has_image,
            image_name: s.image_name.as_str().into(),
            dimensions: s.dimensions.as_str().into(),
            visible: s.visible,
            locked: s.locked,
            opacity: s.opacity,
            size: s.size,
            offset: s.offset,
            rotation: s.rotation,
            fine_open: s.fine_open,
            thumbnail: s.thumbnail.clone().unwrap_or_default(),
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_reference_slots(), ref_slots, |model| {
        window.set_reference_slots(model)
    });
    window.set_uv_layout_commands(vm.uv_editor.layout_commands.as_str().into());
    window.set_uv_seam_commands(vm.uv_editor.seam_commands.as_str().into());
    window.set_uv_pinned_commands(vm.uv_editor.pinned_commands.as_str().into());
    window.set_uv_selected_commands(vm.uv_editor.selected_commands.as_str().into());
    window.set_uv_island_count(vm.uv_editor.island_count as i32);
    window.set_uv_face_count(vm.uv_editor.face_count as i32);
    window.set_uv_selected_face(vm.uv_editor.selected_face);
    window.set_uv_selected_count(vm.uv_editor.uv_selected_count as i32);
    window.set_uv_layout_truncated(vm.uv_editor.truncated);
    window.set_uv_show_texture(vm.uv_show_texture);
}
