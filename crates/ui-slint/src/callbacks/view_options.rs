//! Shading, X-Ray, orientação de faces e overlays de vista.

use super::*;

pub(crate) fn connect_view_options_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    for (kind, which) in [("matcap", 0u8), ("ao", 1u8)] {
        let _ = kind;
        let render_bridge = Arc::clone(&bridge);
        let window_weak = window.as_weak();
        let apply = move |enabled: bool| {
            if let Ok(mut bridge) = render_bridge.lock() {
                let changed = if which == 0 {
                    bridge.set_viewport_matcap(enabled)
                } else {
                    bridge.set_viewport_ambient_occlusion(enabled)
                };
                if changed {
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
            }
        };
        if which == 0 {
            window.on_viewport_matcap_set(apply);
        } else {
            window.on_viewport_ambient_occlusion_set(apply);
        }
    }

    let shading_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shading_mode_set(move |id| {
        if let Ok(mut bridge) = shading_bridge.lock() {
            bridge.set_shading_mode(id.as_str());
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

    let xray_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_xray_requested(move || {
        if let Ok(mut bridge) = xray_toggle_bridge.lock() {
            if let Err(error) = bridge.execute_core_command("view.toggle_xray") {
                bridge.state.set_status(error.to_string());
            }
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

    let xray_opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_xray_opacity_set(move |opacity| {
        if let Ok(mut bridge) = xray_opacity_bridge.lock() {
            bridge.set_xray_opacity(opacity);
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_xray_opacity(opacity);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let shading_popover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shading_popover_toggled(move |open| {
        if let Ok(mut bridge) = shading_popover_bridge.lock() {
            bridge.shading_popover_open = open;
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let face_orient_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_face_orientation(move || {
        if let Ok(mut bridge) = face_orient_bridge.lock() {
            bridge.toggle_face_orientation();
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

    let uv_checker_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_checker(move || {
        if let Ok(mut bridge) = uv_checker_bridge.lock() {
            bridge.toggle_uv_checker();
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

    let view_axis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_view_axis_clicked(move |axis| {
        if let Ok(mut bridge) = view_axis_bridge.lock() {
            bridge.snap_view_to_axis(axis.as_str());
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

    let micro_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_micro_inspector_toggle(move || {
        if let Ok(mut bridge) = micro_bridge.lock() {
            bridge.toggle_micro_inspector();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let toggle_paint_uv_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_paint_show_uv(move || {
        if let Ok(mut bridge) = toggle_paint_uv_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintUvOverlay);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let toggle_uv_tex_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_show_texture(move || {
        if let Ok(mut bridge) = toggle_uv_tex_bridge.lock() {
            bridge.apply(UiIntent::ToggleUvShowTexture);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let pip_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_pip_toggle_requested(move || {
        if let Ok(mut bridge) = pip_bridge.lock() {
            bridge.toggle_paint_pip();
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
}
