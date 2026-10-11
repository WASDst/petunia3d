//! PAINT: pincel, camadas, canvas 2D, simetria e efeitos.

use super::*;

pub(crate) fn connect_paint_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let effect_add_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_effect_layer_added(move |kind| {
        if let Ok(mut bridge) = effect_add_bridge.lock() {
            if !bridge.add_paint_effect_layer(kind.as_str()) {
                let message = crate::tr::fill(
                    &bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_UNKNOWN_EFFECT_LAYER),
                    &[("kind", format!("{kind}"))],
                );
                bridge.state.set_status(message);
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let effect_param_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_effect_param_set(move |key, value| {
        if let Ok(mut bridge) = effect_param_bridge.lock() {
            bridge.set_paint_effect_param(key.as_str(), value);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let decal_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_decal_layer_added(move || {
        if let Ok(mut bridge) = decal_bridge.lock() {
            bridge.add_decal_layer();
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

    let fill_scope_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_fill_scope_set(move |scope| {
        if let Ok(mut bridge) = fill_scope_bridge.lock() {
            bridge.set_fill_scope(scope.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let brush_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_lock_set(move |lock| {
        if let Ok(mut bridge) = brush_lock_bridge.lock() {
            bridge.set_brush_lock(lock.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_stroke_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_2d_stroke(move |norm_x, norm_y, phase, shift, raster_px| {
        if let Ok(mut bridge) = paint_stroke_bridge.lock() {
            bridge.paint_2d_raster_px = raster_px;
            if !bridge.paint_2d_line(norm_x, norm_y, phase, shift) {
                bridge.apply(UiIntent::Paint2dStroke {
                    norm_x,
                    norm_y,
                    phase,
                });
            }
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pixel_grid_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_paint_pixel_grid(move || {
        if let Ok(mut bridge) = pixel_grid_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintPixelGrid);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
            }
        }
    });

    let canvas_zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_paint_canvas_zoom(move |zoom| {
        if let Ok(mut bridge) = canvas_zoom_bridge.lock() {
            bridge.apply(UiIntent::SetPaintCanvasZoom(zoom));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
            }
        }
    });

    // Painel de camadas do PAINT: cada ação recompoe o raster canônico.
    let paint_layer_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_added(move || {
        if let Ok(mut bridge) = paint_layer_bridge.lock() {
            bridge.add_paint_layer();
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

    let paint_group_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_group_added(move || {
        if let Ok(mut bridge) = paint_group_bridge.lock() {
            bridge.add_paint_group();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_removed(move |id| {
        if let Ok(mut bridge) = paint_remove_bridge.lock() {
            if !bridge.remove_paint_layer(id.as_str()) {
                let message = bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_THE_LAST_LAYER_CANNOT_BE_REMOVED);
                bridge.state.set_status(message);
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

    let paint_merge_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_merged_down(move |id| {
        if let Ok(mut bridge) = paint_merge_bridge.lock() {
            if !bridge.merge_down_paint_layer(id.as_str()) {
                let message = bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_CANNOT_MERGE_DOWN_THIS_LAYER);
                bridge.state.set_status(message);
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

    let paint_active_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_activated(move |id| {
        if let Ok(mut bridge) = paint_active_bridge.lock() {
            bridge.set_paint_layer_active(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_visibility_toggled(move |id| {
        if let Ok(mut bridge) = paint_vis_bridge.lock() {
            bridge.toggle_paint_layer_visibility(id.as_str());
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

    let paint_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_lock_toggled(move |id| {
        if let Ok(mut bridge) = paint_lock_bridge.lock() {
            bridge.toggle_paint_layer_lock(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_moved(move |id, delta| {
        if let Ok(mut bridge) = paint_move_bridge.lock() {
            bridge.move_paint_layer(id.as_str(), delta);
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

    let paint_opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_opacity_set(move |id, opacity| {
        if let Ok(mut bridge) = paint_opacity_bridge.lock() {
            bridge.set_paint_layer_opacity(id.as_str(), opacity);
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

    let color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_color_changed(move |r, g, b| {
        if let Ok(mut bridge) = color_bridge.lock() {
            bridge.apply(UiIntent::SetPaintColor([r, g, b]));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_size_changed(move |size| {
        if let Ok(mut bridge) = size_bridge.lock() {
            bridge.apply(UiIntent::SetBrushSize(size));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_opacity_changed(move |opacity| {
        if let Ok(mut bridge) = opacity_bridge.lock() {
            bridge.apply(UiIntent::SetBrushOpacity(opacity));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let hardness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_hardness_changed(move |hardness| {
        if let Ok(mut bridge) = hardness_bridge.lock() {
            bridge.apply(UiIntent::SetBrushHardness(hardness));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let sym_x_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_symmetry_x_toggled(move || {
        if let Ok(mut bridge) = sym_x_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintSymmetryX);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let sym_y_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_symmetry_y_toggled(move || {
        if let Ok(mut bridge) = sym_y_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintSymmetryY);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let sym_z_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_symmetry_z_toggled(move || {
        if let Ok(mut bridge) = sym_z_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintSymmetryZ);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let target_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_target_vertex_changed(move |val| {
        if let Ok(mut bridge) = target_bridge.lock() {
            bridge.apply(UiIntent::SetPaintTargetVertex(val));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let decal_transform_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    // Handles decal transform changes (P3D-133) / Trata alterações na transformação do decalque (P3D-133)
    window.on_decal_transform_changed(move |id, cu, cv, su, sv, rot| {
        if let Ok(mut bridge) = decal_transform_bridge.lock() {
            let target_id = if id.is_empty() {
                bridge
                    .state
                    .project
                    .assets
                    .get(bridge.state.project.active)
                    .and_then(|asset| asset.paint_stack.as_ref())
                    .and_then(|stack| stack.active())
                    .map(|layer| layer.id.to_string())
                    .unwrap_or_default()
            } else {
                id.as_str().to_string()
            };
            if !target_id.is_empty() {
                bridge.apply(UiIntent::SetDecalTransform {
                    layer_id: target_id,
                    center_u: cu,
                    center_v: cv,
                    scale_u: su,
                    scale_v: sv,
                    rotation_deg: rot,
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
        }
    });

    let texture_side_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_texture_side_selected(move |side| {
        if let Ok(mut bridge) = texture_side_bridge.lock() {
            bridge.apply(UiIntent::SetPaintTextureSide(side.max(0) as u32));
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                bridge.publish_canvas_image(&window);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let bake_decal_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    // Bakes active decal layer to static raster (P3D-160) / Converte decalque ativo em raster estático (P3D-160)
    window.on_bake_active_decal(move || {
        if let Ok(mut bridge) = bake_decal_bridge.lock() {
            bridge.apply(UiIntent::BakeActiveDecal);
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
