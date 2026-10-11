//! HUD, overlays e guias de operação sobre a viewport.

use crate::callbacks::*;

pub(super) fn sync_viewport_hud(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_transform_instant_active(vm.transform_instant_active);
    window.set_gizmo_hover_axis(vm.gizmo_hover_axis);
    window.set_gizmo_active_axis(vm.gizmo_active_axis);
    window.set_gizmo_constraint_axis_a(vm.gizmo_constraint_axes[0]);
    window.set_gizmo_constraint_axis_b(vm.gizmo_constraint_axes[1]);
    window.set_gizmo_has_hover(vm.gizmo_has_hover);
    window.set_gizmo_center_active(vm.gizmo_center_active);
    window.set_gizmo_center_hover(vm.gizmo_center_hover);
    window.set_operation_preview_commands(vm.operation_preview_commands.as_str().into());
    window.set_drag_link_commands(vm.drag_link_commands.as_str().into());
    window.set_proportional_circle_commands(vm.proportional_circle_commands.as_str().into());
    window.set_operation_hud_active(vm.operation_hud_active);
    window.set_operation_hud_title(vm.operation_hud_title.as_str().into());
    window.set_operation_hud_subject(vm.operation_hud_subject.as_str().into());
    window.set_operation_hud_hint(vm.operation_hud_hint.as_str().into());
    window.set_context_hint(vm.context_hint.as_str().into());
    window.set_hover_label(vm.hover_label.as_str().into());
    window.set_selection_summary(vm.selection_summary.as_str().into());
    let hud_lines: Vec<slint::SharedString> = vm
        .operation_hud_lines
        .iter()
        .map(|line| line.as_str().into())
        .collect();
    window.set_operation_hud_lines(hud_lines.as_slice().into());
    window.set_extension_radius(vm.draw_extension_values[0]);
    window.set_extension_length(vm.draw_extension_values[1]);
    window.set_extension_angle(vm.draw_extension_values[2]);
    window.set_extension_sides(vm.draw_extension_values[3]);
    window.set_extension_tolerance(vm.draw_extension_values[4]);
    window.set_export_padding(vm.export_padding);
    window.set_trace_threshold(vm.trace_threshold);
    window.set_stencil_visible(vm.stencil_image.is_some());
    window.set_stencil_image(vm.stencil_image.clone().unwrap_or_default());
    sync_overlay_models(window, &vm.selection_overlay, &vm.gizmo);
    window.set_keyboard_tool_modal_active(vm.keyboard_tool_modal_active);
    window.set_is_instant_tool_mode(vm.is_instant_tool_mode);
    window.set_tool_grammar_active(vm.tool_grammar_active);
    window.set_tool_gesture_latched(vm.tool_gesture_latched);
    window.set_last_operation_active(vm.last_operation_active);
    window.set_last_operation_title(vm.last_operation_title.as_str().into());
    window.set_last_operation_value(vm.last_operation_value);
    window.set_last_operation_step(vm.last_operation_step);
    window.set_last_operation_unit(vm.last_operation_unit.as_str().into());
    window.set_gizmo_gpu(vm.gizmo_gpu);
    window.set_tool_modal_active(vm.tool_modal_active);
    window.set_tool_modal_id(vm.tool_modal_id.as_str().into());
    window.set_tool_options_active(vm.tool_options_active);
    window.set_tool_options_title(vm.tool_options_title.as_str().into());
    window.set_tool_options_hint(vm.tool_options_hint.as_str().into());
    window.set_tool_modal_title(vm.tool_modal_title.as_str().into());
    window.set_tool_modal_label(vm.tool_modal_label.as_str().into());
    window.set_tool_modal_value(vm.tool_modal_value);
    window.set_tool_modal_step(vm.tool_modal_step);
    window.set_tool_modal_min(vm.tool_modal_min);
    window.set_tool_modal_max(vm.tool_modal_max);
    window.set_hud_pill_visible(vm.hud_pill_visible);
    window.set_hud_pill_x(vm.hud_pill_x);
    window.set_hud_pill_y(vm.hud_pill_y);
    window.set_hud_pill_title(vm.hud_pill_title.as_str().into());
    window.set_hud_pill_value(vm.hud_pill_value.as_str().into());
    window.set_hud_pill_badge(vm.hud_pill_badge.as_str().into());
    window.set_hud_pill_hint(vm.hud_pill_hint.as_str().into());
    window.set_axis_guide_visible(vm.axis_guide_visible);
    window.set_axis_guide_commands(vm.axis_guide_commands.as_str().into());
    window.set_axis_guide_color(slint::Color::from_rgb_u8(
        vm.axis_guide_color[0],
        vm.axis_guide_color[1],
        vm.axis_guide_color[2],
    ));
    window.set_axis_guide_label(vm.axis_guide_label.as_str().into());
    window.set_axis_guide_label_x(vm.axis_guide_label_x);
    window.set_axis_guide_label_y(vm.axis_guide_label_y);
    window.set_dimension_visible(vm.dimension_visible);
    window.set_dimension_commands(vm.dimension_commands.as_str().into());
    window.set_dimension_text(vm.dimension_text.as_str().into());
    window.set_dimension_x(vm.dimension_x);
    window.set_dimension_y(vm.dimension_y);
    window.set_snap_marker_visible(vm.snap_marker_visible);
    window.set_snap_marker_x(vm.snap_marker_x);
    window.set_snap_marker_y(vm.snap_marker_y);
    window.set_snap_marker_label(vm.snap_marker_label.as_str().into());
    window.set_snap_marker_round(vm.snap_marker_round);

    window.set_measure_visible(vm.measure_visible);
    window.set_measure_commands(vm.measure_commands.as_str().into());
    window.set_measure_text(vm.measure_text.as_str().into());
    window.set_measure_x(vm.measure_x);
    window.set_measure_y(vm.measure_y);
    window.set_measure_distance(vm.measure_distance);
    window.set_measure_dx(vm.measure_dx);
    window.set_measure_dy(vm.measure_dy);
    window.set_measure_dz(vm.measure_dz);
    window.set_measure_angle_deg(vm.measure_angle_deg);
    window.set_measure_hud_text(vm.measure_hud_text.as_str().into());
    let measure_tags: Vec<MeasureTag> = vm
        .measure_tags
        .iter()
        .map(|tag| MeasureTag {
            text: tag.text.as_str().into(),
            x: tag.x,
            y: tag.y,
        })
        .collect();
    crate::refresh::set_model_if_changed(window.get_measure_tags(), measure_tags, |model| {
        window.set_measure_tags(model)
    });

    let world_axis_tags: Vec<WorldAxisTag> = vm
        .world_axis_labels
        .iter()
        .map(|tag| WorldAxisTag {
            text: tag.text.as_str().into(),
            x: tag.x,
            y: tag.y,
            tint: slint::Color::from_rgb_u8(tag.color[0], tag.color[1], tag.color[2]),
        })
        .collect();
    crate::refresh::set_model_if_changed(
        window.get_world_axis_labels(),
        world_axis_tags,
        |model| window.set_world_axis_labels(model),
    );

    window.set_micro_inspector_open(vm.micro_inspector_open);
    window.set_micro_inspector_x(vm.micro_inspector_x);
    window.set_micro_inspector_y(vm.micro_inspector_y);

    window.set_protractor_visible(vm.protractor_visible);
    window.set_protractor_wedge_commands(vm.protractor_wedge_commands.as_str().into());
    window.set_protractor_ticks_commands(vm.protractor_ticks_commands.as_str().into());
    window.set_overlay_top_id(vm.overlay_top_id.as_str().into());
}
