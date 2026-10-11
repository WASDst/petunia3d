//! Menus do Header e menu de contexto.

use super::*;

pub(crate) fn connect_menus_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let context_action_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_context_menu_action(move |action| {
        if let Ok(mut bridge) = context_action_bridge.lock() {
            bridge.context_menu_action(action.as_str());
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

    let menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_toggled(move |id| {
        if let Ok(mut bridge) = menu_bridge.lock() {
            bridge.toggle_menu(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let menu_open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_opened(move |id| {
        if let Ok(mut bridge) = menu_open_bridge.lock() {
            if !bridge.open_menu(id.as_str()) {
                return;
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let menu_item_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_item_invoked(move |id| {
        let id = id.to_string();
        // Diálogos de arquivo continuam no caminho assíncrono já testado.
        let opens_dialog = matches!(
            id.as_str(),
            "file.open" | "file.save" | "file.save_as" | "file.import_obj"
        );
        // "Parts" abre a seção no Inspector (ou a gaveta no shell compacto):
        // é o mesmo caminho do antigo botão da top bar.
        if id == "window.parts" {
            if let Ok(mut bridge) = menu_item_bridge.lock() {
                bridge.close_menu();
            }
            if let Some(window) = window_weak.upgrade() {
                window.set_menu_open("".into());
                window.invoke_scene_requested();
            }
            return;
        }
        if let Ok(mut bridge) = menu_item_bridge.lock() {
            bridge.close_menu();
            if opens_dialog {
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    window.invoke_command_file_action(id.as_str().into());
                }
                return;
            }
            bridge.menu_item_invoked(id.as_str());
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

    let pivot_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_pivot_set(move |id| {
        if let Ok(mut bridge) = pivot_bridge.lock() {
            bridge.set_pivot_point(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pivot_menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_pivot_menu_toggled(move |open| {
        if let Ok(mut bridge) = pivot_menu_bridge.lock() {
            bridge.set_pivot_menu_open(open);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
}
