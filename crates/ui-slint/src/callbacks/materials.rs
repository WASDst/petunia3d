//! Materiais e texturas.

use super::*;

pub(crate) fn connect_materials_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let assign_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_assign_material_slot(move |slot| {
        if slot < 0 {
            return;
        }
        if let Ok(mut bridge) = assign_mat_bridge.lock() {
            bridge.apply(UiIntent::AssignMaterialSlot(slot as usize));
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

    let create_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_create_material(move || {
        if let Ok(mut bridge) = create_mat_bridge.lock() {
            bridge.apply(UiIntent::CreateMaterial);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dup_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_duplicate_material(move |slot| {
        if slot < 0 {
            return;
        }
        if let Ok(mut bridge) = dup_mat_bridge.lock() {
            bridge.apply(UiIntent::DuplicateMaterial(slot as usize));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let material_slot_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_slot_selected(move |slot| {
        if let Ok(mut bridge) = material_slot_bridge.lock() {
            bridge.select_material_slot(slot);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_base_color_changed(move |red, green, blue| {
        if let Ok(mut bridge) = material_color_bridge.lock() {
            bridge.set_active_material_base_color(red, green, blue);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_scalar_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_scalar_committed(move |field, slot, text| {
        let Ok(mut bridge) = material_scalar_bridge.lock() else {
            return false;
        };
        let Ok(value) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        bridge.select_material_slot(slot);
        let result = bridge.set_active_material_scalar(field.as_str(), value);
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        result
    });

    let material_profile_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_profile_changed(move |profile| {
        if let Ok(mut bridge) = material_profile_bridge.lock() {
            bridge.set_active_material_profile(profile);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_alpha_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_alpha_mode_changed(move |mode| {
        if let Ok(mut bridge) = material_alpha_bridge.lock() {
            bridge.set_active_material_alpha_mode(mode);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_texture_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_albedo_create(move || {
        if let Ok(mut bridge) = material_texture_bridge.lock() {
            bridge.create_albedo_texture();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let material_texture_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_albedo_clear(move || {
        if let Ok(mut bridge) = material_texture_clear_bridge.lock() {
            bridge.clear_albedo_texture();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let remove_material_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_remove_material(move |slot| {
        if let Ok(mut bridge) = remove_material_bridge.lock() {
            bridge.remove_material(slot);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
}
