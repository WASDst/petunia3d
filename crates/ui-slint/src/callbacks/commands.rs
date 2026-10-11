//! Atalhos, busca de comandos, ações rápidas, Escape e click-away.

use super::*;

pub(crate) fn connect_commands_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let shortcut_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shortcut_requested(move |text, ctrl, shift, alt| {
        if let Ok(mut bridge) = shortcut_bridge.lock() {
            bridge.route_shortcut(text.as_str(), ctrl, shift, alt);
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
    let search_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_search_requested(move || {
        if let Ok(mut bridge) = search_bridge.lock() {
            bridge.apply(UiIntent::OpenCommandSearch);
            let items: Vec<CommandItem> = bridge
                .search_commands("")
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.into(),
                    label: cmd.label.into(),
                    shortcut: cmd.shortcut.unwrap_or_default().into(),
                    available: cmd.is_available,
                    disabled_reason: cmd.disabled_reason.unwrap_or_default().into(),
                })
                .collect();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(true);
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                window.set_command_results(model.into());
            }
        }
    });

    let query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_query_changed(move |query| {
        let items: Vec<CommandItem> = if let Ok(bridge) = query_bridge.lock() {
            bridge
                .search_commands(query.as_str())
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.into(),
                    label: cmd.label.into(),
                    shortcut: cmd.shortcut.unwrap_or_default().into(),
                    available: cmd.is_available,
                    disabled_reason: cmd.disabled_reason.unwrap_or_default().into(),
                })
                .collect()
        } else {
            Vec::new()
        };
        if let Some(window) = window_weak.upgrade() {
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            window.set_command_results(model.into());
        }
    });

    let exec_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_executed(move |id_str| {
        if let Ok(mut bridge) = exec_bridge.lock() {
            if let Some(command) = CommandId::from_id_str(id_str.as_str()) {
                if command == CommandId::ToggleSceneDrawer
                    && let Some(window) = window_weak.upgrade()
                    && !window.get_compact_shell()
                {
                    if bridge.scene_drawer_visible {
                        bridge.apply(UiIntent::ToggleSceneDrawer);
                    }
                    window.set_model_parts_open(true);
                } else {
                    bridge.execute_command(command);
                }
            } else if let Err(error) = bridge.execute_core_command(id_str.as_str()) {
                bridge.state.set_status(error.to_string());
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(bridge.command_search_visible);
                window.set_settings_visible(bridge.settings_visible);
                window.set_scene_drawer_visible(bridge.scene_drawer_visible);
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let esc_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_escape_requested(move || {
        if let Ok(mut bridge) = esc_bridge.lock() {
            bridge.handle_escape();
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

    let click_away_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_click_away_requested(move || {
        if let Ok(mut bridge) = click_away_bridge.lock() {
            bridge.handle_click_away();
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

    let add_menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_menu_changed(move |open| {
        if let Ok(mut bridge) = add_menu_bridge.lock() {
            bridge.set_add_menu_open(open);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let mask_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_mask_selection_toggled(move || {
        if let Ok(mut bridge) = mask_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintMaskSelection);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let wire_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_wireframe_requested(move || {
        if let Ok(mut bridge) = wire_bridge.lock() {
            bridge.execute_command(CommandId::ToggleWireOverlay);
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

    let quick_execute_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_action_executed(move |id| {
        if let Ok(mut bridge) = quick_execute_bridge.lock() {
            bridge.execute_quick_action(id.as_str());
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

    let quick_action_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_action_toggled(move |id| {
        if let Ok(mut bridge) = quick_action_bridge.lock() {
            bridge.toggle_quick_action(id.as_str());
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let quick_reset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_actions_reset(move || {
        if let Ok(mut bridge) = quick_reset_bridge.lock() {
            bridge.reset_quick_actions();
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
}
