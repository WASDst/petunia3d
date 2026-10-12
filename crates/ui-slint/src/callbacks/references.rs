//! Gerenciador de referências.

use super::*;

pub(crate) fn connect_references_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let ref_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_manager_toggle(move || {
        if let Ok(mut bridge) = ref_toggle_bridge.lock() {
            bridge.toggle_reference_manager();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ref_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_manager_closed(move || {
        if let Ok(mut bridge) = ref_close_bridge.lock() {
            bridge.close_reference_manager();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ref_pick_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_pick(move |axis| {
        let bridge = Arc::clone(&ref_pick_bridge);
        let window_weak = window_weak.clone();
        let axis = axis.to_string();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.open_reference_image().await else {
                return;
            };
            let file_name = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "reference".to_string());
            let Ok((width, height, rgba)) = files::load_image_rgba(&path) else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.load_reference_slot(&axis, file_name, width, height, rgba);
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
    });

    let ref_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_remove(move |axis| {
        if let Ok(mut bridge) = ref_remove_bridge.lock() {
            bridge.remove_reference_slot(axis.as_str());
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

    let ref_align_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_align(move |axis| {
        if let Ok(mut bridge) = ref_align_bridge.lock() {
            bridge.align_reference_view(axis.as_str());
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

    let ref_vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_toggle_visible(move |axis| {
        if let Ok(mut bridge) = ref_vis_bridge.lock() {
            bridge.toggle_reference_visible(axis.as_str());
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

    let ref_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_toggle_lock(move |axis| {
        if let Ok(mut bridge) = ref_lock_bridge.lock() {
            bridge.toggle_reference_lock(axis.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let ref_param_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_slot_set_param(move |axis, param, val| {
        if let Ok(mut bridge) = ref_param_bridge.lock() {
            bridge.set_reference_param(axis.as_str(), param.as_str(), val);
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade()
                && let Some(frame) = new_frame
            {
                window.set_viewport_image(frame);
            }
        }
    });

    let ref_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reference_clear_all(move || {
        if let Ok(mut bridge) = ref_clear_bridge.lock() {
            bridge.clear_all_references();
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
