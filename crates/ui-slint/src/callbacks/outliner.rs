//! Outliner, renomear, filtros de assets, prefabs e cor das partes.

use super::*;

pub(crate) fn connect_outliner_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let scene_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_requested(move || {
        if let Ok(mut bridge) = scene_bridge.lock()
            && let Some(window) = window_weak.upgrade()
        {
            if window.get_compact_shell() {
                bridge.apply(UiIntent::ToggleSceneDrawer);
            } else {
                if bridge.scene_drawer_visible {
                    bridge.apply(UiIntent::ToggleSceneDrawer);
                }
                window.set_model_parts_open(true);
            }
            let vm = bridge.view_model();
            window.set_scene_drawer_visible(bridge.scene_drawer_visible);
            sync_window_properties(&window, &vm);
        }
    });

    let scene_pin_bridge = Arc::clone(&bridge);
    window.on_scene_pin_requested(move |pinned| {
        if let Ok(mut bridge) = scene_pin_bridge.lock() {
            bridge.overlays.set_pinned(OverlayId::SceneDrawer, pinned);
        }
    });

    let parts_query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_query_changed(move |query| {
        if let Ok(mut bridge) = parts_query_bridge.lock() {
            bridge.set_parts_query(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_filter_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_selected_only_changed(move |enabled| {
        if let Ok(mut bridge) = parts_filter_bridge.lock() {
            bridge.set_parts_selected_only(enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_sort_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_sort_changed(move |enabled| {
        if let Ok(mut bridge) = parts_sort_bridge.lock() {
            bridge.set_parts_sort_by_name(enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_query_changed(move |query| {
        if let Ok(mut bridge) = asset_query_bridge.lock() {
            bridge.set_asset_query(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_sort_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_sort_changed(move |sort_by_name| {
        if let Ok(mut bridge) = asset_sort_bridge.lock() {
            bridge.set_asset_sort_by_name(sort_by_name);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_thumbnail_size_changed(move |size| {
        if let Ok(mut bridge) = asset_size_bridge.lock() {
            bridge.set_asset_thumbnail_size(size);
            if let Some(window) = window_weak.upgrade() {
                window.set_asset_thumbnail_size(size);
            }
        }
    });

    let rename_edit_bridge = Arc::clone(&bridge);
    window.on_rename_edited(move |text| {
        if let Ok(mut bridge) = rename_edit_bridge.lock()
            && let Some(draft) = bridge.rename_draft.as_mut()
        {
            *draft = text.to_string();
        }
    });

    let rename_commit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_rename_committed(move |text| {
        if let Ok(mut bridge) = rename_commit_bridge.lock() {
            bridge.commit_rename(text.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let rename_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_rename_cancelled(move || {
        if let Ok(mut bridge) = rename_cancel_bridge.lock() {
            bridge.cancel_rename();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let context_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_context_requested(move |id, x, y| {
        if let Ok(mut bridge) = context_bridge.lock() {
            bridge.open_context_menu(id.as_str(), x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let place_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_place_requested(move |id| {
        if let Ok(mut bridge) = place_bridge.lock() {
            bridge.place_prefab(id.as_str());
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

    let prefab_delete_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_delete_requested(move |id| {
        if let Ok(mut bridge) = prefab_delete_bridge.lock() {
            bridge.delete_prefab(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prefab_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_update_requested(move |id| {
        if let Ok(mut bridge) = prefab_update_bridge.lock() {
            bridge.update_prefab(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prefab_favorite_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_prefab_favorite_requested(move |id, favorite| {
        if let Ok(mut bridge) = prefab_favorite_bridge.lock() {
            bridge.set_prefab_favorite(id.as_str(), favorite);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let only_favorites_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_only_favorites_changed(move |only| {
        if let Ok(mut bridge) = only_favorites_bridge.lock() {
            bridge.set_asset_only_favorites(only);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_select(move |id, extend| {
        if let Ok(mut bridge) = select_bridge.lock() {
            if let Some(profile_uuid_str) = id.as_str().strip_prefix("profile:") {
                if let Ok(profile_id) = uuid::Uuid::parse_str(profile_uuid_str) {
                    bridge.cancel_active_operation();
                    bridge.activate_profile(profile_id);
                    bridge.state.select_object(None, false);
                    bridge.reset_transform_fields();
                }
            } else if let Some(index) = bridge
                .state
                .project
                .assets
                .iter()
                .position(|a| a.id.to_string() == id.as_str())
            {
                bridge.cancel_active_operation();
                bridge.active_profile_id = None;
                bridge.state.select_object(Some(index), extend);
                bridge.reset_transform_fields();
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

    let vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_toggle_visibility(move |id| {
        if let Ok(mut bridge) = vis_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneAssetVisibility(
                id.as_str().to_string(),
            ));
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

    let lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_toggle_lock(move |id| {
        if let Ok(mut bridge) = lock_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneAssetLock(id.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let part_color_open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_picker_open(move |id| {
        let id_str = id.to_string();
        if let Ok(bridge) = part_color_open_bridge.lock()
            && let Some(window) = window_weak.upgrade()
            && let Ok(asset_id) = uuid::Uuid::parse_str(&id_str)
            && let Some(asset) = bridge
                .state
                .project
                .project
                .assets
                .iter()
                .find(|a| a.id == asset_id)
        {
            let custom_color = asset.selection_overlay_color;
            let eff = custom_color.unwrap_or(bridge.state.ui.selection_rgb);
            let hex = format!("#{:02X}{:02X}{:02X}", eff[0], eff[1], eff[2]);
            window.set_part_color_popover_id(id_str.as_str().into());
            window.set_part_color_popover_name(asset.name.as_str().into());
            window.set_part_color_preview(slint::Color::from_rgb_u8(eff[0], eff[1], eff[2]));
            window.set_part_color_hex(hex.into());
            window.set_part_color_has_custom(custom_color.is_some());
            window.set_color_wheel_image(crate::color_wheel::generate_color_wheel_image(130));
            window.set_part_color_popover_open(true);
        }
    });

    let wheel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_wheel_interacted(move |x, y| {
        if let Some(window) = window_weak.upgrade() {
            let id = window.get_part_color_popover_id().to_string();
            let dx = x - 65.0;
            let dy = y - 65.0;
            let rgb = crate::color_wheel::sample_wheel_color(dx, dy, 64.0);
            let hex = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            window.set_part_color_preview(slint::Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
            window.set_part_color_hex(hex.into());
            window.set_part_color_has_custom(true);

            if let Ok(mut bridge) = wheel_bridge.lock() {
                bridge.set_part_selection_color(&id, Some(rgb));
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let hex_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_hex_entered(move |hex| {
        let clean = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        if let Ok(val) = u32::from_str_radix(clean, 16) {
            let rgb = match clean.len() {
                6 => [
                    ((val >> 16) & 0xFF) as u8,
                    ((val >> 8) & 0xFF) as u8,
                    (val & 0xFF) as u8,
                ],
                3 => [
                    (((val >> 8) & 0xF) * 17) as u8,
                    (((val >> 4) & 0xF) * 17) as u8,
                    ((val & 0xF) * 17) as u8,
                ],
                _ => return,
            };
            if let Some(window) = window_weak.upgrade() {
                let id = window.get_part_color_popover_id().to_string();
                window.set_part_color_preview(slint::Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
                window.set_part_color_has_custom(true);
                if let Ok(mut bridge) = hex_bridge.lock() {
                    bridge.set_part_selection_color(&id, Some(rgb));
                    let vm = bridge.view_model();
                    let new_frame = bridge.render_viewport();
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        }
    });

    let preset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_preset_clicked(move |color| {
        if let Some(window) = window_weak.upgrade() {
            let id = window.get_part_color_popover_id().to_string();
            let rgb = [color.red(), color.green(), color.blue()];
            let hex = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            window.set_part_color_preview(color);
            window.set_part_color_hex(hex.into());
            window.set_part_color_has_custom(true);

            if let Ok(mut bridge) = preset_bridge.lock() {
                bridge.set_part_selection_color(&id, Some(rgb));
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let reset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_part_color_reset_requested(move || {
        if let Some(window) = window_weak.upgrade() {
            let id = window.get_part_color_popover_id().to_string();
            if let Ok(mut bridge) = reset_bridge.lock() {
                bridge.set_part_selection_color(&id, None);
                let default_rgb = bridge.state.ui.selection_rgb;
                let hex = format!(
                    "#{:02X}{:02X}{:02X}",
                    default_rgb[0], default_rgb[1], default_rgb[2]
                );
                window.set_part_color_preview(slint::Color::from_rgb_u8(
                    default_rgb[0],
                    default_rgb[1],
                    default_rgb[2],
                ));
                window.set_part_color_hex(hex.into());
                window.set_part_color_has_custom(false);

                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let sep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_separate_selection_requested(move || {
        if let Ok(mut bridge) = sep_bridge.lock() {
            let _ = bridge.execute_core_command("model.separate_selection");
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

    let move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    // Reorders an asset in the scene hierarchy / Reordena um asset na hierarquia da cena
    window.on_scene_move(move |id, delta| {
        if let Ok(mut bridge) = move_bridge.lock() {
            bridge.apply(UiIntent::MoveSceneAsset {
                id: id.as_str().to_string(),
                delta,
            });
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

    let delete_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_delete_requested(move || {
        if let Ok(mut bridge) = delete_bridge.lock() {
            bridge.delete_or_dissolve_selection();
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

    let dup_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_duplicate_requested(move || {
        if let Ok(mut bridge) = dup_bridge.lock() {
            bridge.duplicate_selection();
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

    let copy_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_copy_requested(move || {
        if let Ok(mut bridge) = copy_bridge.lock() {
            bridge.copy_selection();
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

    let paste_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paste_requested(move || {
        if let Ok(mut bridge) = paste_bridge.lock() {
            bridge.paste_clipboard();
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

    let proj_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_projection_requested(move || {
        if let Ok(mut bridge) = proj_bridge.lock() {
            bridge.apply(UiIntent::ToggleProjection);
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

    let modifier_direction_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_direction_changed(move |id, direction| {
        if let Ok(mut bridge) = modifier_direction_bridge.lock() {
            bridge.set_modifier_direction(id.as_str(), direction);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
}
