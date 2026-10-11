//! Configurações, keymap, brush presets e preferências de apresentação.

use super::*;

pub(crate) fn connect_settings_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let settings_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_settings_requested(move || {
        if let Ok(mut bridge) = settings_bridge.lock() {
            bridge.apply(UiIntent::OpenSettings);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_settings_visible(true);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let settings_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_settings_closed(move || {
        if let Ok(mut bridge) = settings_close_bridge.lock() {
            persist_user_preferences(&mut bridge);
            bridge.apply(UiIntent::CloseSettings);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_settings_visible(false);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let keymap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_keymap_changed(move |keymap_id| {
        if let Ok(mut bridge) = keymap_bridge.lock() {
            if bridge.set_keymap_profile(keymap_id.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    // Editor de atalhos: criar/apagar perfil, capturar, restaurar e limpar.
    macro_rules! keymap_callback {
        ($register:ident, $persist:expr, |$bridge:ident $(, $arg:ident)*| $body:expr) => {{
            let handle = Arc::clone(&bridge);
            let window_weak = window.as_weak();
            window.$register(move |$($arg),*| {
                if let Ok(mut $bridge) = handle.lock() {
                    let changed: bool = $body;
                    if changed && $persist {
                        persist_user_preferences(&mut $bridge);
                    }
                    let vm = $bridge.view_model();
                    if let Some(window) = window_weak.upgrade() {
                        sync_window_properties(&window, &vm);
                    }
                }
            });
        }};
    }
    keymap_callback!(on_brush_param_changed, false, |b, name, value| {
        b.set_brush_param(name.as_str(), value)
    });
    keymap_callback!(on_brush_tip_changed, false, |b, tip| b
        .set_brush_tip(tip.as_str()));
    keymap_callback!(on_brush_blend_changed, false, |b, blend| b
        .set_brush_blend(blend.as_str()));
    keymap_callback!(on_brush_preset_applied, false, |b, index| b
        .apply_brush_preset(index.max(0) as usize));
    keymap_callback!(on_brush_preset_saved, false, |b| {
        let count = b.all_brush_presets().len();
        b.save_brush_preset(&format!("Custom {}", count + 1))
    });
    keymap_callback!(on_brush_preset_deleted, false, |b, index| b
        .delete_brush_preset(index.max(0) as usize));
    keymap_callback!(on_keymap_profile_create, true, |b| b
        .create_keymap_profile(""));
    keymap_callback!(on_keymap_profile_delete, true, |b, id| b
        .delete_keymap_profile(id.as_str()));
    keymap_callback!(on_keymap_capture_requested, true, |b, action| b
        .begin_keymap_capture(action.as_str()));
    keymap_callback!(on_keymap_capture_cancelled, true, |b| b
        .cancel_keymap_capture());
    keymap_callback!(on_keymap_capture_key, true, |b, key, ctrl, shift, alt| b
        .capture_keymap_key(key.as_str(), ctrl, shift, alt));
    keymap_callback!(on_keymap_binding_reset, true, |b, action| b
        .reset_keymap_binding(action.as_str()));
    keymap_callback!(on_keymap_binding_clear, true, |b, action| b
        .clear_keymap_binding(action.as_str()));

    let activation_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_activation_set(move |id| {
        if let Ok(mut bridge) = activation_bridge.lock() {
            bridge.set_tool_activation(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let vertical_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_invert_vertical_drag_set(move |invert| {
        if let Ok(mut bridge) = vertical_drag_bridge.lock() {
            if bridge.set_invert_vertical_drag(invert) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let colorblind_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_colorblind_axes_set(move |enabled| {
        if let Ok(mut bridge) = colorblind_bridge.lock() {
            if bridge.set_colorblind_axes(enabled) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                sync_viewport_overlays(&window, &bridge);
            }
        }
    });

    let reduced_motion_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reduced_motion_set(move |enabled| {
        if let Ok(mut bridge) = reduced_motion_bridge.lock() {
            if bridge.set_reduced_motion(enabled) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let double_tap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_double_tap_interval_set(move |interval| {
        if let Ok(mut bridge) = double_tap_bridge.lock() {
            bridge.set_double_tap_interval_ms(interval.max(0) as u64);
            if let Some(window) = window_weak.upgrade() {
                window.set_double_tap_interval_ms(interval);
            }
        }
    });

    let double_tap_rel_bridge = Arc::clone(&bridge);
    window.on_double_tap_interval_released(move |_interval| {
        if let Ok(mut bridge) = double_tap_rel_bridge.lock() {
            persist_user_preferences(&mut bridge);
        }
    });

    let measure_tag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_multiselection_measure_tag_set(move |enabled| {
        if let Ok(mut bridge) = measure_tag_bridge.lock() {
            if bridge.set_multiselection_measure_tag(enabled) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                sync_viewport_overlays(&window, &bridge);
            }
        }
    });

    let selection_color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_text_matches(|text, query| {
        let query = query.trim().to_lowercase();
        query.is_empty() || text.to_lowercase().contains(&query)
    });

    let home_pref_bridge = Arc::clone(&bridge);
    let home_pref_weak = window.as_weak();
    window.on_show_home_on_start_set(move |enabled| {
        if let Ok(mut bridge) = home_pref_bridge.lock()
            && bridge.set_show_home_on_start(enabled)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = home_pref_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let accent_color_bridge = Arc::clone(&bridge);
    let accent_window_weak = window.as_weak();
    window.on_accent_color_set(move |hex| {
        if let Ok(mut bridge) = accent_color_bridge.lock() {
            if bridge.set_accent_color_hex(hex.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = accent_window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    window.on_selection_color_set(move |hex| {
        if let Ok(mut bridge) = selection_color_bridge.lock() {
            if bridge.set_selection_color_hex(hex.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let selection_thickness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_thickness_set(move |thickness| {
        if let Ok(mut bridge) = selection_thickness_bridge.lock() {
            bridge.set_selection_thickness(thickness);
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_selection_thickness(thickness);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let selection_thickness_rel_bridge = Arc::clone(&bridge);
    window.on_selection_thickness_released(move |_thickness| {
        if let Ok(mut bridge) = selection_thickness_rel_bridge.lock() {
            persist_user_preferences(&mut bridge);
        }
    });

    let language_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_language_changed(move |lang| {
        if let Ok(mut bridge) = language_bridge.lock() {
            if bridge.set_language(lang.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ui_scale_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_ui_scale_set(move |scale| {
        if let Ok(mut bridge) = ui_scale_bridge.lock() {
            if bridge.set_ui_scale(scale) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let icon_theme_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_icon_theme_set(move |theme| {
        if let Ok(mut bridge) = icon_theme_bridge.lock() {
            if bridge.set_icon_theme(theme.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let drag_threshold_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_drag_threshold_set(move |pixels| {
        if let Ok(mut bridge) = drag_threshold_bridge.lock()
            && bridge.set_drag_threshold_px(pixels)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let snap_radius_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_snap_radius_set(move |pixels| {
        if let Ok(mut bridge) = snap_radius_bridge.lock()
            && bridge.set_snap_radius_px(pixels)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let click_move_click_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_click_move_click_set(move |enabled| {
        if let Ok(mut bridge) = click_move_click_bridge.lock()
            && bridge.set_click_move_click(enabled)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let show_tool_labels_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_show_tool_labels_set(move |enabled| {
        if let Ok(mut bridge) = show_tool_labels_bridge.lock()
            && bridge.set_show_tool_labels(enabled)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let studio_light_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_studio_light_follows_camera_set(move |follows| {
        if let Ok(mut bridge) = studio_light_bridge.lock()
            && bridge.set_studio_light_follows_camera(follows)
        {
            persist_user_preferences(&mut bridge);
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let workplane_ground_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_workplane_prefer_ground_set(move |enabled| {
        if let Ok(mut bridge) = workplane_ground_bridge.lock()
            && bridge.set_workplane_prefer_ground(enabled)
        {
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let projection_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_projection_set(move |projection| {
        if let Ok(mut bridge) = projection_bridge.lock() {
            bridge.set_brush_projection(projection.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dimensions_bridge = Arc::clone(&bridge);
    let dimensions_window = window.as_weak();
    window.on_profile_dimension_set(move |field, text| {
        let Ok(mut bridge) = dimensions_bridge.lock() else {
            return false;
        };
        let vm = bridge.view_model();
        let base = match field.as_str() {
            "width" => vm.profile_width,
            "height" => vm.profile_height,
            "segments" => vm.profile_segments as f32,
            _ => return false,
        };
        let Ok(value) = numeric::parse_numeric_with_base(text.as_str(), base) else {
            return false;
        };
        let (width, height, segments) = match field.as_str() {
            "width" => (value, vm.profile_height, vm.profile_segments),
            "height" => (vm.profile_width, value, vm.profile_segments),
            "segments" if (6.0..=512.0).contains(&value) && value.fract() == 0.0 => {
                (vm.profile_width, vm.profile_height, value as i32)
            }
            _ => return false,
        };
        let accepted = bridge.set_profile_dimensions(width, height, segments as usize);
        if let Some(window) = dimensions_window.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let theme_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_theme_changed(move |theme_id| {
        if let Ok(mut bridge) = theme_bridge.lock() {
            bridge.apply(UiIntent::SetTheme(theme_id.as_str().to_string()));
            persist_user_preferences(&mut bridge);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });
}
