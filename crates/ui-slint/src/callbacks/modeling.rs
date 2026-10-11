//! Booleanas, modificadores, Loop Cut, Slice, primitivas e perfil DRAW.

use super::*;

pub(crate) fn connect_modeling_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let keep_parts_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_keep_parts_set(move |keep| {
        if let Ok(mut bridge) = keep_parts_bridge.lock() {
            bridge.set_boolean_keep_parts(keep);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let cleanup_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_cleanup_set(move |cleanup| {
        if let Ok(mut bridge) = cleanup_bridge.lock() {
            bridge.set_boolean_cleanup(cleanup);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let join_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_join_requested(move || {
        if let Ok(mut bridge) = join_bridge.lock() {
            bridge.join_operand();
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

    let prof_rect_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_profile_rectangle(move |width, height| {
        if let Ok(mut bridge) = prof_rect_bridge.lock() {
            bridge.add_profile_rectangle(width, height);
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

    let prof_circle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_profile_circle(move |radius, segments| {
        if let Ok(mut bridge) = prof_circle_bridge.lock() {
            bridge.add_profile_circle(radius, segments as usize);
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

    let operand_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_operand_cleared(move || {
        if let Ok(mut bridge) = operand_clear_bridge.lock() {
            bridge.clear_boolean_operand();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let operand_set_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_operand_set(move |id| {
        if let Ok(mut bridge) = operand_set_bridge.lock() {
            bridge.set_boolean_operand(id.as_str());
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

    let extension_bridge = Arc::clone(&bridge);
    let extension_window = window.as_weak();
    window.on_extension_parameter_set(move |key, text| {
        let Ok(mut bridge) = extension_bridge.lock() else {
            return false;
        };
        let changed = bridge.set_extension_parameter(key.as_str(), text.as_str());
        if let Some(window) = extension_window.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
        }
        changed
    });
    let loop_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_scrubbed(move |delta_x, delta_y| {
        if let Ok(mut bridge) = loop_scrub_bridge.lock() {
            bridge.scrub_loop_cut_2d(delta_x, delta_y, false);
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

    let loop_slide_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_slide_committed(move |text| {
        let Ok(mut bridge) = loop_slide_bridge.lock() else {
            return false;
        };
        let accepted = match numeric::parse_numeric(text.as_str()) {
            Ok(value) => {
                bridge
                    .loop_cut
                    .as_ref()
                    .is_some_and(|session| session.slide == value)
                    || bridge.set_loop_cut_slide(value)
            }
            Err(_) => {
                let message = bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_SLIDE_MUST_BE_BETWEEN_1);
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

    let loop_count_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_count_committed(move |text| {
        if let Ok(mut bridge) = loop_count_bridge.lock() {
            match text.trim().parse::<i32>() {
                Ok(cuts) => {
                    bridge.adjust_loop_cut_count_from_input(cuts.clamp(1, 32) as usize);
                }
                Err(_) => {
                    let message = bridge
                        .state
                        .t_id(petunia_config::text_id::STATUS_LOOP_CUT_CUTS_MUST_BE_A_WHOLE);
                    bridge.state.set_status(message);
                }
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

    let loop_place_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_place(move || {
        if let Ok(mut bridge) = loop_place_bridge.lock() {
            bridge.place_loop_cut_hover();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_close(move || {
        if let Ok(mut bridge) = profile_close_bridge.lock() {
            bridge.close_profile();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let profile_depth_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_depth_set(move |text| {
        let Ok(depth) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        if let Ok(mut bridge) = profile_depth_bridge.lock() {
            let accepted = bridge.set_profile_depth(depth);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
            accepted
        } else {
            false
        }
    });

    let profile_enter_extrude_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_enter_extrude(move || {
        if let Ok(mut bridge) = profile_enter_extrude_bridge.lock() {
            bridge.enter_profile_volume("extrude");
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_enter_revolve_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_enter_revolve(move || {
        if let Ok(mut bridge) = profile_enter_revolve_bridge.lock() {
            bridge.enter_profile_volume("revolve");
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_enter_sweep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_enter_sweep(move || {
        if let Ok(mut bridge) = profile_enter_sweep_bridge.lock() {
            bridge.enter_profile_volume("sweep");
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_volume_confirm_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_volume_confirm(move || {
        if let Ok(mut bridge) = profile_volume_confirm_bridge.lock() {
            bridge.commit_profile_volume();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_volume_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_volume_cancel(move || {
        if let Ok(mut bridge) = profile_volume_cancel_bridge.lock() {
            bridge.cancel_profile_volume();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_revolve_angle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_revolve_angle_set(move |text| {
        let Ok(angle) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        if let Ok(mut bridge) = profile_revolve_angle_bridge.lock() {
            let accepted = bridge.set_profile_revolve_angle(angle);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
            accepted
        } else {
            false
        }
    });

    let profile_generate_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_generate(move || {
        if let Ok(mut bridge) = profile_generate_bridge.lock() {
            bridge.generate_profile_extrude();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_revolve_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_revolve(move || {
        if let Ok(mut bridge) = profile_revolve_bridge.lock() {
            bridge.generate_profile_revolve();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_sweep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_sweep(move || {
        if let Ok(mut bridge) = profile_sweep_bridge.lock() {
            bridge.generate_profile_sweep();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_wall_thickness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_wall_thickness_set(move |text| {
        let Ok(thickness) = text.trim().parse::<f32>() else {
            return false;
        };
        if let Ok(mut bridge) = profile_wall_thickness_bridge.lock() {
            let accepted = bridge.set_profile_wall_thickness(thickness);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
            accepted
        } else {
            false
        }
    });

    let profile_smoothness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_smoothness_set(move |text| {
        let Ok(smoothness) = text.trim().parse::<f32>() else {
            return false;
        };
        if let Ok(mut bridge) = profile_smoothness_bridge.lock() {
            let accepted = bridge.set_profile_smoothness(smoothness);
            if accepted && let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
            accepted
        } else {
            false
        }
    });

    let profile_smooth_curves_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_smooth_curves(move || {
        if let Ok(mut bridge) = profile_smooth_curves_bridge.lock() {
            bridge.profile_smooth_curves();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let profile_clear_curves_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_clear_curves(move || {
        if let Ok(mut bridge) = profile_clear_curves_bridge.lock() {
            bridge.profile_clear_curves();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_ground_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_ground(move || {
        if let Ok(mut bridge) = wp_ground_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneGround);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_face_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_face(move || {
        if let Ok(mut bridge) = wp_face_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneFace);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_auto_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_auto(move || {
        if let Ok(mut bridge) = wp_auto_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneAuto);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_look_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_look_at_plane(move || {
        if let Ok(mut bridge) = wp_look_bridge.lock() {
            bridge.apply(UiIntent::ProfileLookAtPlane);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let wp_view_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_set_workplane_view(move || {
        if let Ok(mut bridge) = wp_view_bridge.lock() {
            bridge.apply(UiIntent::ProfileSetWorkplaneView);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let loop_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_apply(move || {
        if let Ok(mut bridge) = loop_apply_bridge.lock() {
            bridge.commit_loop_cut();
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

    let loop_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_cancel(move || {
        if let Ok(mut bridge) = loop_cancel_bridge.lock() {
            bridge.cancel_loop_cut();
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

    let loop_balanced_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_loop_cut_balanced(move || {
        if let Ok(mut bridge) = loop_balanced_bridge.lock() {
            bridge.toggle_loop_cut_balanced();
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

    let prim_float_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_param_float(move |param, val| {
        if let Ok(mut bridge) = prim_float_bridge.lock()
            && bridge.update_primitive_param_float(param.as_str(), val)
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

    let prim_bool_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_param_bool(move |param, val| {
        if let Ok(mut bridge) = prim_bool_bridge.lock()
            && bridge.update_primitive_param_bool(param.as_str(), val)
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

    let prim_confirm_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_confirm(move || {
        if let Ok(mut bridge) = prim_confirm_bridge.lock() {
            bridge.confirm_primitive();
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

    let prim_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_primitive_cancel(move || {
        if let Ok(mut bridge) = prim_cancel_bridge.lock() {
            bridge.cancel_primitive();
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

    let freeze_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_freeze_primitive_requested(move || {
        if let Ok(mut bridge) = freeze_bridge.lock() {
            bridge.apply(UiIntent::FreezeActivePrimitive);
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

    let slice_trim_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_slice_trim_set(move |trim| {
        if let Ok(mut bridge) = slice_trim_bridge.lock() {
            bridge.set_slice_trim(trim);
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

    let slice_confirm_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_slice_confirm(move || {
        if let Ok(mut bridge) = slice_confirm_bridge.lock()
            && bridge.commit_slice()
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

    let slice_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_slice_cancel(move || {
        if let Ok(mut bridge) = slice_cancel_bridge.lock()
            && bridge.cancel_slice()
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

    let bevel_clamp_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_bevel_clamp_overlap(move || {
        if let Ok(mut bridge) = bevel_clamp_bridge.lock() {
            bridge.toggle_bevel_clamp_overlap();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let bevel_affect_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_bevel_affect_vertices(move || {
        if let Ok(mut bridge) = bevel_affect_bridge.lock() {
            bridge.toggle_bevel_affect_vertices();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let modifier_add_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_add(move |kind| {
        if let Ok(mut bridge) = modifier_add_bridge.lock() {
            bridge.add_modifier(kind.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_toggle(move |id, enabled| {
        if let Ok(mut bridge) = modifier_toggle_bridge.lock() {
            bridge.set_modifier_enabled(id.as_str(), enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_remove(move |id| {
        if let Ok(mut bridge) = modifier_remove_bridge.lock() {
            bridge.remove_modifier(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_move(move |id, direction| {
        if let Ok(mut bridge) = modifier_move_bridge.lock() {
            bridge.move_modifier(id.as_str(), direction);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_apply(move |id| {
        if let Ok(mut bridge) = modifier_apply_bridge.lock() {
            bridge.apply_modifier(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_axis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_axis_changed(move |id, axis| {
        if let Ok(mut bridge) = modifier_axis_bridge.lock() {
            bridge.set_modifier_axis(id.as_str(), axis);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
}
