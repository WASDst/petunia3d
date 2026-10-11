//! Workspace UV.

use super::*;

pub(crate) fn connect_uv_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let uv_click_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_editor_clicked(move |u, v, extend| {
        if let Ok(mut bridge) = uv_click_bridge.lock() {
            bridge.uv_editor_click(u, v, extend);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_moved(move |du, dv| {
        if let Ok(mut bridge) = uv_move_bridge.lock() {
            bridge.uv_move_selected(du, dv);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_scale_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_scaled(move |factor| {
        if let Ok(mut bridge) = uv_scale_bridge.lock() {
            bridge.uv_scale_selected(factor);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_rotate_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_rotated(move |degrees| {
        if let Ok(mut bridge) = uv_rotate_bridge.lock() {
            bridge.uv_rotate_selected(degrees);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_seam_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_seam_toggled(move || {
        if let Ok(mut bridge) = uv_seam_bridge.lock() {
            bridge.toggle_selected_uv_seams();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_seams_cleared(move || {
        if let Ok(mut bridge) = uv_clear_bridge.lock() {
            bridge.clear_all_uv_seams();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_pin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_pins(move || {
        if let Ok(mut bridge) = uv_pin_bridge.lock() {
            bridge.toggle_selected_uv_pins();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_clear_pins_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_clear_all_uv_pins(move || {
        if let Ok(mut bridge) = uv_clear_pins_bridge.lock() {
            bridge.clear_all_uv_pins();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_equalize_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_equalize_texel_density(move || {
        if let Ok(mut bridge) = uv_equalize_bridge.lock() {
            bridge.uv_equalize_texel_density();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
}
