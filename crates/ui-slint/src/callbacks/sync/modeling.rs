//! Perfil DRAW, POLY, Loop Cut, primitivas e opções de ferramentas de modelagem.

use crate::callbacks::*;

pub(super) fn sync_modeling(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_profile_workplane_locked(vm.profile_workplane_locked);
    window.set_loop_cut_active(vm.loop_cut_active);
    window.set_loop_cut_slide(vm.loop_cut_slide);
    window.set_loop_cut_cuts(vm.loop_cut_cuts);
    window.set_loop_cut_preview_commands(vm.loop_cut_preview_commands.as_str().into());
    window.set_loop_cut_armed(vm.loop_cut_armed);
    window.set_loop_cut_balanced(vm.loop_cut_balanced);
    window.set_profile_active(vm.profile_active);
    window.set_profile_point_count(vm.profile_point_count);
    window.set_profile_closed(vm.profile_closed);
    window.set_profile_preview_commands(vm.profile_preview_commands.as_str().into());
    window.set_region_hover_commands(vm.region_hover_commands.as_str().into());
    window.set_profile_selection_commands(vm.profile_selection_commands.as_str().into());
    window.set_region_shapes_commands(vm.region_shapes_commands.as_str().into());
    window.set_keymap_capture_action(vm.keymap_capture_action.as_str().into());
    window.set_profile_outline_commands(vm.profile_outline_commands.as_str().into());
    window.set_poly_pen_preview_commands(vm.poly_pen_preview_commands.as_str().into());
    window.set_poly_pen_mode(vm.poly_pen_mode.as_str().into());
    window.set_modeling_mode(vm.modeling_mode.as_str().into());
    window.set_profile_parametric(vm.profile_parametric);
    window.set_profile_ellipse(vm.profile_ellipse);
    window.set_profile_width(vm.profile_width);
    window.set_profile_height(vm.profile_height);
    window.set_profile_segments(vm.profile_segments);
    window.set_profile_depth(vm.profile_depth);
    window.set_profile_wall_thickness(vm.profile_wall_thickness);
    window.set_profile_smoothness(vm.profile_smoothness);
    window.set_profile_has_curves(vm.profile_has_curves);
    window.set_profile_workplane(vm.profile_workplane.as_str().into());
    window.set_profile_volume_mode(vm.profile_volume_mode.as_str().into());
    window.set_profile_revolve_angle(vm.profile_revolve_angle);
    window.set_primitive_active(vm.primitive_active);
    window.set_primitive_kind(vm.primitive_kind.as_str().into());
    window.set_primitive_title(vm.primitive_title.as_str().into());
    window.set_primitive_radius(vm.primitive_radius);
    window.set_primitive_radius_b(vm.primitive_radius_b);
    window.set_primitive_height(vm.primitive_height);
    window.set_primitive_depth(vm.primitive_depth);
    window.set_primitive_width(vm.primitive_width);
    window.set_primitive_sides(vm.primitive_sides);
    window.set_primitive_rings(vm.primitive_rings);
    window.set_primitive_subdiv(vm.primitive_subdiv);
    window.set_primitive_cap_top(vm.primitive_cap_top);
    window.set_primitive_cap_bottom(vm.primitive_cap_bottom);
    window.set_primitive_fill_disc(vm.primitive_fill_disc);
    window.set_slice_trim(vm.slice_trim);
    window.set_bevel_clamp_overlap(vm.bevel_clamp_overlap);
    window.set_bevel_affect_vertices(vm.bevel_affect_vertices);
}
