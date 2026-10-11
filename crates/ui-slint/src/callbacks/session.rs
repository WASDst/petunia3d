//! Arquivo, workspace, undo/redo, recuperação e tela inicial.

use super::*;

pub(crate) fn connect_session_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let workspace_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_workspace_changed(move |workspace| {
        let mode = match workspace.as_str() {
            "DRAW" => Some(crate::ModelingMode::Draw),
            "POLY" => Some(crate::ModelingMode::Poly),
            _ => None,
        };
        if let Some(mode) = mode
            && let Ok(mut bridge) = workspace_bridge.lock()
        {
            bridge.apply(UiIntent::SetModelingMode(mode));
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
            return;
        }
        let workspace = match workspace.as_str() {
            "MODEL" => Workspace::Model,
            "PAINT" => Workspace::Paint,
            "UV" => Workspace::Uv,
            #[cfg(feature = "animation-workspace")]
            "ANIMATE" => Workspace::Animate,
            _ => return,
        };
        if let Ok(mut bridge) = workspace_bridge.lock() {
            bridge.apply(UiIntent::SetWorkspace(workspace));
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            let canvas_img = if workspace == Workspace::Paint || workspace == Workspace::Uv {
                bridge.render_paint_canvas()
            } else {
                None
            };
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = canvas_img {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let save_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_save_requested(move || {
        let existing = save_bridge
            .lock()
            .ok()
            .and_then(|bridge| bridge.state.project.project_path.clone());
        if let Some(path) = existing {
            if let Ok(mut bridge) = save_bridge.lock() {
                bridge.apply(UiIntent::SaveProjectTo(PathBuf::from(path)));
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    window.set_status_severity(vm.status_severity);
                    window.set_status_message(vm.status_message.as_str().into());
                }
            }
            return;
        }
        let bridge = Arc::clone(&save_bridge);
        let window_weak = window_weak.clone();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.save_project().await else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.apply(UiIntent::SaveProjectTo(path));
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    window.set_status_severity(vm.status_severity);
                    window.set_status_message(vm.status_message.as_str().into());
                }
            }
        });
    });

    let open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_open_project_requested(move || {
        let bridge = Arc::clone(&open_bridge);
        let window_weak = window_weak.clone();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.open_project().await else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.apply(UiIntent::OpenProjectFrom(path));
                persist_user_preferences(&mut bridge);
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let file_command_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_file_action(move |id| {
        let bridge = Arc::clone(&file_command_bridge);
        let window_weak = window_weak.clone();
        let id = id.to_string();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let path = match id.as_str() {
                "file.open" => service.open_project().await,
                "file.save" | "file.save_as" => service.save_project().await,
                "file.import_obj" => service.import_model().await,
                "file.export_obj" => service.export_obj().await,
                "file.export_glb" => service.export_glb().await,
                "palette.import" => service.import_palette().await,
                "palette.export" => service.export_palette().await,
                "paint.import_decal" | "paint.add_decal_variant" => {
                    service.open_decal_image().await
                }
                "paint.export_decal_frames" => service.select_asset_folder().await,
                "draw.import_svg" => service.open_svg().await,
                _ => None,
            };
            let Some(path) = path else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                let intent = match id.as_str() {
                    "file.open" => UiIntent::OpenProjectFrom(path),
                    "file.import_obj" => UiIntent::ImportModelFrom(path),
                    "file.export_obj" => UiIntent::ExportActiveObjTo(path),
                    "file.export_glb" => UiIntent::ExportSceneGlbTo(path),
                    "palette.import" => UiIntent::ImportPalette(path),
                    "palette.export" => UiIntent::ExportPalette(path),
                    "paint.import_decal" => UiIntent::ImportDecalFrom(path),
                    "paint.add_decal_variant" => UiIntent::AddDecalVariantFrom(path),
                    "paint.export_decal_frames" => UiIntent::ExportDecalFramesTo(path),
                    "draw.import_svg" => UiIntent::ImportSvgProfiles(path),
                    _ => UiIntent::SaveProjectTo(path),
                };
                let needs_render = matches!(
                    id.as_str(),
                    "file.open"
                        | "file.import_obj"
                        | "paint.import_decal"
                        | "paint.add_decal_variant"
                );
                let touches_recents =
                    matches!(id.as_str(), "file.open" | "file.save" | "file.save_as");
                bridge.apply(intent);
                if touches_recents {
                    // Home: abrir e salvar atualizam a lista de recentes.
                    persist_user_preferences(&mut bridge);
                }
                bridge.command_search_visible = false;
                bridge.overlays.remove(OverlayId::CommandPalette);
                let vm = bridge.view_model();
                let new_frame = if needs_render {
                    bridge.render_viewport()
                } else {
                    None
                };
                if let Some(window) = window_weak.upgrade() {
                    window.set_command_search_visible(false);
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let home_close_bridge = Arc::clone(&bridge);
    let home_window_weak = window.as_weak();
    window.on_home_closed(move || {
        if let Ok(mut bridge) = home_close_bridge.lock() {
            bridge.home_open = false;
            if let Some(window) = home_window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let home_recent_bridge = Arc::clone(&bridge);
    let home_recent_weak = window.as_weak();
    window.on_home_recent_opened(move |path| {
        if let Ok(mut bridge) = home_recent_bridge.lock() {
            bridge.apply(UiIntent::OpenProjectFrom(std::path::PathBuf::from(
                path.as_str(),
            )));
            persist_user_preferences(&mut bridge);
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = home_recent_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let recover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_recover_requested(move || {
        if let Ok(mut bridge) = recover_bridge.lock() {
            bridge.recover_pending();
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

    let keep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_keep_requested(move || {
        if let Ok(mut bridge) = keep_bridge.lock() {
            bridge.keep_saved_project();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let discard_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_discard_requested(move || {
        if let Ok(mut bridge) = discard_bridge.lock() {
            bridge.discard_pending_recovery();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let undo_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_undo_requested(move || {
        if let Ok(mut bridge) = undo_bridge.lock() {
            bridge.apply(UiIntent::Undo);
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

    let redo_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_redo_requested(move || {
        if let Ok(mut bridge) = redo_bridge.lock() {
            bridge.apply(UiIntent::Redo);
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

    let asset_lib_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_library_requested(move || {
        if let Ok(mut bridge) = asset_lib_bridge.lock() {
            bridge.apply(UiIntent::ToggleAssetLibrary);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_asset_library_visible(bridge.asset_library_visible);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let reset_cam_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reset_camera_requested(move || {
        if let Ok(mut bridge) = reset_cam_bridge.lock() {
            bridge.apply(UiIntent::ResetCamera);
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

    let save_asset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_save_active_as_asset_requested(move || {
        if let Ok(mut bridge) = save_asset_bridge.lock() {
            bridge.apply(UiIntent::SaveActiveAsAsset);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });
}
