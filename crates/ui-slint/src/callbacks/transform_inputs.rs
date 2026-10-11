//! Transformação via Inspector, cursor 3D, snap e edição proporcional.

use super::*;

pub(crate) fn connect_transform_inputs_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
    throttle: std::rc::Rc<crate::refresh::RefreshThrottle>,
) {
    let transform_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrubbed(move |kind_str, axis, delta, fine| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_bridge.lock() {
            let val = bridge.scrub_transform(kind, axis as usize, delta, fine);
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                match (kind, axis) {
                    (TransformKind::Position, 0) => window.set_pos_x(val),
                    (TransformKind::Position, 1) => window.set_pos_y(val),
                    (TransformKind::Position, 2) => window.set_pos_z(val),
                    (TransformKind::Rotation, 0) => window.set_rot_x(val),
                    (TransformKind::Rotation, 1) => window.set_rot_y(val),
                    (TransformKind::Rotation, 2) => window.set_rot_z(val),
                    (TransformKind::Scale, 0) => window.set_scale_x(val),
                    (TransformKind::Scale, 1) => window.set_scale_y(val),
                    (TransformKind::Scale, 2) => window.set_scale_z(val),
                    _ => {}
                }
            }
        }
    });

    let cursor_coord_bridge = Arc::clone(&bridge);
    let cursor_coord_window = window.as_weak();
    window.on_cursor_coord_committed(move |axis_idx, val| {
        if let Ok(mut bridge) = cursor_coord_bridge.lock()
            && bridge.set_cursor_3d_coord(axis_idx as usize, val)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = cursor_coord_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let reset_cursor_bridge = Arc::clone(&bridge);
    let reset_cursor_window = window.as_weak();
    window.on_reset_cursor_requested(move || {
        if let Ok(mut bridge) = reset_cursor_bridge.lock()
            && bridge.reset_cursor_3d()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = reset_cursor_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let frame_cursor_bridge = Arc::clone(&bridge);
    let frame_cursor_window = window.as_weak();
    window.on_frame_cursor_requested(move || {
        if let Ok(mut bridge) = frame_cursor_bridge.lock()
            && bridge.frame_cursor()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = frame_cursor_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let transform_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_started(move |kind_str| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock()
            && let Err(error) = bridge.begin_transform(kind)
        {
            bridge.state.set_status(error.to_string());
            if let Some(window) = window_weak.upgrade() {
                window.set_status_severity(3);
                window.set_status_message(error.to_string().into());
            }
        }
    });

    let transform_finish_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_finished(move || {
        if let Ok(mut bridge) = transform_finish_bridge.lock() {
            bridge.commit_transform();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade()
                && let Some(frame) = frame
            {
                window.set_viewport_image(frame);
            }
        }
    });

    let transform_text_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_text_committed(move |kind_str, axis, text| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return false,
        };
        let Ok(mut bridge) = transform_text_bridge.lock() else {
            return false;
        };
        let result = bridge.commit_transform_text(kind, axis as usize, text.as_str());
        if let Err(error) = result {
            let message = crate::tr::fill(
                &bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_INVALID_NUMERIC_VALUE),
                &[("error", format!("{error:?}"))],
            );
            bridge.state.set_status(message);
        }
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        result.is_ok()
    });

    let transform_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_cancelled(move || {
        if let Ok(mut bridge) = transform_cancel_bridge.lock() {
            bridge.cancel_transform();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let tool_hover_throttle = std::rc::Rc::clone(&throttle);
    window.on_tool_modal_hovered(move |delta, x, y| {
        if let Ok(mut bridge) = tool_hover_bridge.lock()
            && bridge.is_instant_tool_mode()
            && bridge.tool_modal.is_some()
        {
            bridge.pointer_position = [x, y];
            bridge.scrub_tool_modal(delta, false);
            if let Some(window) = window_weak.upgrade() {
                crate::refresh::refresh_interactive(
                    &window,
                    &tool_hover_bridge,
                    &mut *bridge,
                    &tool_hover_throttle,
                );
            }
        }
    });

    let tool_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_scrubbed(move |delta, fine, x, y| {
        if let Ok(mut bridge) = tool_scrub_bridge.lock() {
            bridge.pointer_position = [x, y];
            bridge.scrub_tool_modal(delta, fine);
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

    let tool_text_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_text_committed(move |text| {
        let Ok(mut bridge) = tool_text_bridge.lock() else {
            return false;
        };
        let base = bridge.tool_modal_value;
        let accepted = match numeric::parse_numeric_with_base(text.as_str(), base) {
            Ok(value) => bridge.set_tool_modal_value(value),
            Err(error) => {
                let message = crate::tr::fill(
                    &bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_INVALID_VALUE),
                    &[("error", format!("{error:?}"))],
                );
                bridge.state.set_status(message);
                false
            }
        };
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let tool_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_apply(move || {
        if let Ok(mut bridge) = tool_apply_bridge.lock() {
            bridge.commit_tool_modal();
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

    let tool_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_cancel(move || {
        if let Ok(mut bridge) = tool_cancel_bridge.lock() {
            bridge.cancel_tool_modal();
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

    let extrude_mode_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_toggle_extrude_mode(move || {
        if let Ok(mut bridge) = extrude_mode_bridge.lock()
            && bridge.switch_extrude_mode()
        {
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

    // Gramática única (constituição 11): o botão principal das ferramentas
    // persistentes chega aqui como pressionar (0), mover (1), soltar (2) ou
    // cancelar (3). Movimentos sem efeito não redesenham nada.
    let last_operation_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_last_operation_committed(move |text| {
        let Ok(mut bridge) = last_operation_bridge.lock() else {
            return false;
        };
        let accepted = bridge.commit_last_operation_text(text.as_str());
        let vm = bridge.view_model();
        let new_frame = bridge.render_viewport();
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &vm);
            if let Some(frame) = new_frame {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let prop_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_proportional_editing(move || {
        if let Ok(mut bridge) = prop_bridge.lock() {
            bridge.toggle_proportional_editing();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_rad_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_proportional_radius(move |radius| {
        if let Ok(mut bridge) = prop_rad_bridge.lock() {
            bridge.set_proportional_radius(radius);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_fall_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_proportional_falloff(move |falloff| {
        if let Ok(mut bridge) = prop_fall_bridge.lock() {
            bridge.set_proportional_falloff(falloff.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_adj_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_adjust_proportional_radius(move |delta| {
        if let Ok(mut bridge) = prop_adj_bridge.lock() {
            bridge.adjust_proportional_radius(delta);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let snap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_snap_enabled(move || {
        if let Ok(mut bridge) = snap_bridge.lock() {
            bridge.toggle_snap_enabled();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let snap_target_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_snap_target(move |target| {
        if let Ok(mut bridge) = snap_target_bridge.lock() {
            bridge.set_snap_target(target.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dimension_gesture_bridge = Arc::clone(&bridge);
    let dimension_gesture_window = window.as_weak();
    window.on_profile_dimension_gesture(move |phase| {
        if let Ok(mut bridge) = dimension_gesture_bridge.lock() {
            bridge.profile_dimension_gesture(phase);
            if let Some(window) = dimension_gesture_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
    let domain_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_domain_changed(move |domain_str| {
        let domain = match domain_str.as_str() {
            "OBJECT" => SelectionDomain::Object,
            "POINT" => SelectionDomain::Vertex,
            "EDGE" => SelectionDomain::Edge,
            "FACE" => SelectionDomain::Face,
            _ => return,
        };
        if let Ok(mut bridge) = domain_bridge.lock() {
            bridge.apply(UiIntent::SetSelectionDomain(domain));
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

    let tool_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_active_tool_changed(move |tool| {
        if let Ok(mut bridge) = tool_bridge.lock() {
            bridge.apply(UiIntent::SetActiveTool(tool.as_str().to_string()));
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

    let primitive_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_primitive_requested(move |kind_str| {
        let Some(kind) = petunia_core::PrimitiveKind::parse(kind_str.as_str()).ok() else {
            if let Ok(mut bridge) = primitive_bridge.lock() {
                let message = crate::tr::fill(
                    &bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_UNKNOWN_PRIMITIVE),
                    &[("kind_str", format!("{kind_str}"))],
                );
                bridge.state.set_status(message);
            }
            return;
        };
        if let Ok(mut bridge) = primitive_bridge.lock() {
            bridge.apply(UiIntent::AddPrimitive(kind));
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

    let pen_mode_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_poly_pen_mode_selected(move |mode| {
        if let Ok(mut bridge) = pen_mode_bridge.lock() {
            bridge.apply(UiIntent::SetPolyPenMode(mode.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });
}
