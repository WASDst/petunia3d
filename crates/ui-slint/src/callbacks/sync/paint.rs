//! PAINT, camadas, decals e canvas.

use crate::callbacks::*;

pub(super) fn sync_paint(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_paint_color(slint::Color::from_argb_f32(
        1.0,
        vm.paint_color[0],
        vm.paint_color[1],
        vm.paint_color[2],
    ));
    window.set_brush_size(vm.brush_size);
    window.set_brush_opacity(vm.brush_opacity);
    window.set_brush_hardness(vm.brush_hardness);
    window.set_paint_symmetry_x(vm.paint_symmetry_x);
    window.set_paint_symmetry_y(vm.paint_symmetry_y);
    window.set_paint_symmetry_z(vm.paint_symmetry_z);
    window.set_paint_target_vertex(vm.paint_target_vertex);
    window.set_paint_mask_selection(vm.paint_mask_selection);
    window.set_decal_preview_commands(vm.decal_preview_commands.as_str().into());
    let decal_variants: Vec<slint::SharedString> = vm
        .decal_variant_names
        .iter()
        .map(|name| name.as_str().into())
        .collect();
    window.set_decal_variant_names(decal_variants.as_slice().into());
    window.set_decal_variant_index(vm.decal_variant_index);
    window.set_decal_frame(vm.decal_frame);
    window.set_decal_track_length(vm.decal_track_length);
    window.set_decal_track_fps(vm.decal_track_fps);
    window.set_decal_keys_text(vm.decal_keys_text.as_str().into());
    window.set_decal_has_keys(vm.decal_has_keys);
    window.set_decal_frame_keyed(vm.decal_frame_keyed);
    window.set_decal_playing(vm.decal_playing);
    window.set_paint_layer_count(vm.paint_layer_count.as_str().into());
    window.set_paint_effect_kind(vm.paint_effect_kind.as_str().into());
    let effect_params: Vec<PaintEffectParamEntry> = vm
        .paint_effect_params
        .iter()
        .map(|param| PaintEffectParamEntry {
            key: param.key.as_str().into(),
            label: param.label.as_str().into(),
            value: param.value,
            minimum: param.min,
            maximum: param.max,
        })
        .collect();
    window.set_paint_effect_params(effect_params.as_slice().into());
    window.set_paint_canvas_size(vm.paint_canvas_size.as_str().into());
    window.set_paint_canvas_revision(vm.paint_canvas_revision);
    window.set_paint_fill_scope(vm.paint_fill_scope.as_str().into());
    window.set_paint_projection(vm.paint_projection.as_str().into());
    window.set_paint_lock(vm.paint_lock.as_str().into());
    let layer_entries: Vec<PaintLayerEntry> = vm
        .paint_layers
        .iter()
        .map(|layer| PaintLayerEntry {
            id: layer.id.as_str().into(),
            name: layer.name.as_str().into(),
            visible: layer.visible,
            locked: layer.locked,
            opacity: layer.opacity,
            active: layer.active,
            is_group: layer.is_group,
            kind_label: layer.kind_label.as_str().into(),
            kind: layer.kind.as_str().into(),
            blend: layer.blend,
            depth: layer.depth,
        })
        .collect();
    window.set_paint_layers(layer_entries.as_slice().into());
    window.set_active_layer_is_decal(vm.active_layer_is_decal);
    window.set_decal_surface(vm.decal_surface);
    window.set_paint_texture_side(vm.paint_texture_side);
    window.set_decal_center_u(vm.decal_center_u);
    window.set_decal_center_v(vm.decal_center_v);
    window.set_decal_scale_u(vm.decal_scale_u);
    window.set_decal_scale_v(vm.decal_scale_v);
    window.set_decal_rotation_deg(vm.decal_rotation_deg);
    {
        let b = &vm.brush_panel;
        window.set_brush_flow(b.flow);
        window.set_brush_spacing(b.spacing);
        window.set_brush_smoothing(b.smoothing);
        window.set_brush_size_jitter(b.size_jitter);
        window.set_brush_opacity_jitter(b.opacity_jitter);
        window.set_brush_scatter(b.scatter);
        window.set_brush_spray_density(b.spray_density);
        window.set_brush_angle(b.angle);
        window.set_brush_roundness(b.roundness);
        window.set_brush_tip(b.tip.as_str().into());
        window.set_brush_blend(b.blend.as_str().into());
        window.set_brush_preset_index(b.active_preset);
        window.set_clone_source_set(b.clone_source_set);
        let names: Vec<slint::SharedString> = b.presets.iter().map(|n| n.as_str().into()).collect();
        let current: Vec<slint::SharedString> =
            slint::Model::iter(&window.get_brush_presets()).collect();
        if current != names {
            crate::refresh::set_model_if_changed(window.get_brush_presets(), names, |model| {
                window.set_brush_presets(model)
            });
        }
    }
    window.set_paint_show_uv(vm.paint_show_uv_overlay);
    window.set_paint_pixel_grid(vm.paint_pixel_grid);
    window.set_paint_canvas_zoom(vm.paint_canvas_zoom);
    window.set_paint_canvas_grid_commands(vm.paint_canvas_grid_commands.as_str().into());
}
