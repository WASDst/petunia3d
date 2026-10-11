//! Transformação, cursor 3D, pivô, snap e edição proporcional.

use crate::callbacks::*;

pub(super) fn sync_transform_state(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_pos_x(vm.position[0]);
    window.set_pos_y(vm.position[1]);
    window.set_pos_z(vm.position[2]);
    window.set_rot_x(vm.rotation[0]);
    window.set_rot_y(vm.rotation[1]);
    window.set_rot_z(vm.rotation[2]);
    window.set_scale_x(vm.scale[0]);
    window.set_scale_y(vm.scale[1]);
    window.set_scale_z(vm.scale[2]);
    window.set_cursor_x(vm.cursor_3d[0]);
    window.set_cursor_y(vm.cursor_3d[1]);
    window.set_cursor_z(vm.cursor_3d[2]);

    window.set_proportional_editing(vm.proportional_editing);
    window.set_proportional_radius(vm.proportional_radius);
    window.set_proportional_falloff(vm.proportional_falloff.as_str().into());
    window.set_snap_enabled(vm.snap_enabled);
    window.set_snap_target(vm.snap_target.as_str().into());

    window.set_pivot_id(vm.pivot_id.as_str().into());
    window.set_pivot_label(vm.pivot_label.as_str().into());
    window.set_pivot_median_label(vm.pivot_median_label.as_str().into());
    window.set_pivot_bounds_label(vm.pivot_bounds_label.as_str().into());
    window.set_pivot_cursor_label(vm.pivot_cursor_label.as_str().into());
    window.set_pivot_individual_label(vm.pivot_individual_label.as_str().into());
    window.set_edit_pivot(vm.edit_pivot);
    window.set_drag_threshold_px(vm.drag_threshold_px);
    window.set_snap_radius_px(vm.snap_radius_px);
}
