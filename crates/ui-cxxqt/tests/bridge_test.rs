use petunia_ui_cxxqt::bridge::shell::PetuniaShellBridgeRust;
use petunia_ui_cxxqt::render::opengl_viewport::{
    QtViewportRenderer, petunia_qt_app_add_modifier, petunia_qt_app_apply_modifier,
    petunia_qt_app_delete_keyframe, petunia_qt_app_execute_command, petunia_qt_app_export_gltf,
    petunia_qt_app_export_obj, petunia_qt_app_frame_selection, petunia_qt_app_get_active_asset,
    petunia_qt_app_get_asset_color, petunia_qt_app_get_asset_count, petunia_qt_app_get_asset_name,
    petunia_qt_app_get_asset_position, petunia_qt_app_get_asset_rotation,
    petunia_qt_app_get_asset_scale, petunia_qt_app_get_asset_visible,
    petunia_qt_app_get_material_pbr, petunia_qt_app_get_modifier_count,
    petunia_qt_app_get_modifier_info, petunia_qt_app_get_scene_stats,
    petunia_qt_app_get_selection_domain, petunia_qt_app_get_shading,
    petunia_qt_app_get_surface_shading, petunia_qt_app_get_timeline_info,
    petunia_qt_app_get_uv_diagnostics, petunia_qt_app_get_uv_face_count,
    petunia_qt_app_get_uv_face_verts, petunia_qt_app_has_keyframe, petunia_qt_app_insert_keyframe,
    petunia_qt_app_is_dirty, petunia_qt_app_load_project, petunia_qt_app_move_modifier,
    petunia_qt_app_quick_action, petunia_qt_app_redo, petunia_qt_app_remove_modifier,
    petunia_qt_app_save_project, petunia_qt_app_search_commands, petunia_qt_app_select_asset,
    petunia_qt_app_selected_count, petunia_qt_app_set_asset_color,
    petunia_qt_app_set_asset_position, petunia_qt_app_set_asset_rotation,
    petunia_qt_app_set_asset_scale, petunia_qt_app_set_material_pbr,
    petunia_qt_app_set_selection_domain, petunia_qt_app_set_shading,
    petunia_qt_app_set_surface_shading, petunia_qt_app_set_timeline_frame,
    petunia_qt_app_toggle_modifier, petunia_qt_app_toggle_playback,
    petunia_qt_app_toggle_visibility, petunia_qt_app_undo, petunia_qt_camera_get_orientation,
    petunia_qt_camera_get_view_name, petunia_qt_camera_orbit, petunia_qt_camera_pan,
    petunia_qt_camera_reset, petunia_qt_camera_set_preset, petunia_qt_camera_toggle_projection,
    petunia_qt_camera_zoom, petunia_qt_gizmo_drag_axis, petunia_qt_gizmo_get_projected,
    petunia_qt_gizmo_hit_test, petunia_qt_tablet_paint_stroke, petunia_qt_viewport_hover,
    petunia_qt_viewport_select,
};

fn reset_editor_fixture() {
    petunia_ui_cxxqt::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            *renderer.borrow_mut() = QtViewportRenderer::new();
        })
        .unwrap();
}
#[test]
fn test_shell_bridge_properties_default() {
    let bridge = PetuniaShellBridgeRust::default();
    assert_eq!(bridge.selected_count, 0);
    assert!(!bridge.can_undo);
    assert!(!bridge.can_redo);
    assert!(!bridge.is_dirty);
}

#[test]
fn test_qt_viewport_renderer_initial_state_and_navigation() {
    let mut renderer = QtViewportRenderer::new();
    assert!(!renderer.is_initialized());
    assert!(!renderer.app.state.project.assets.is_empty());

    let initial_yaw = renderer.app.state.session.camera.yaw;
    let initial_dist = renderer.app.state.session.camera.distance;
    let initial_target = renderer.app.state.session.camera.target;

    renderer.orbit_camera(10.0, 5.0);
    assert_ne!(renderer.app.state.session.camera.yaw, initial_yaw);

    renderer.zoom_camera(2.0);
    assert_ne!(renderer.app.state.session.camera.distance, initial_dist);

    renderer.pan_camera(5.0, 5.0);
    assert_ne!(renderer.app.state.session.camera.target, initial_target);

    renderer.reset_camera();
    assert_eq!(renderer.app.state.session.camera.target, glam::Vec3::ZERO);
}

#[test]
fn test_qt_viewport_command_and_outliner_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        // Initial asset count (default cube)
        let initial_count = petunia_qt_app_get_asset_count();
        assert!(initial_count >= 1);

        // Read asset name via FFI
        let mut name_buf = [0i8; 64];
        let ok = petunia_qt_app_get_asset_name(0, name_buf.as_mut_ptr(), 64);
        assert!(ok);
        let c_str = std::ffi::CStr::from_ptr(name_buf.as_ptr());
        assert_eq!(c_str.to_str().unwrap(), "Cube");

        // Add another cube via command FFI
        let cmd = std::ffi::CString::new("model.add_cube").unwrap();
        let executed = petunia_qt_app_execute_command(cmd.as_ptr());
        assert!(executed);
        assert_eq!(petunia_qt_app_get_asset_count(), initial_count + 1);
        assert!(petunia_qt_app_is_dirty());
        assert!(petunia_qt_app_undo());
        assert_eq!(petunia_qt_app_get_asset_count(), initial_count);
        assert!(petunia_qt_app_redo());
        assert_eq!(petunia_qt_app_get_asset_count(), initial_count + 1);

        // Toggle visibility
        assert!(petunia_qt_app_get_asset_visible(0));
        petunia_qt_app_toggle_visibility(0, false);
        assert!(!petunia_qt_app_get_asset_visible(0));
        petunia_qt_app_toggle_visibility(0, true);
        assert!(petunia_qt_app_get_asset_visible(0));

        // Position get and set
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;
        assert!(petunia_qt_app_get_asset_position(0, &mut x, &mut y, &mut z));
        petunia_qt_app_set_asset_position(0, 1.5, 2.5, 3.5);
        petunia_qt_app_get_asset_position(0, &mut x, &mut y, &mut z);
        assert_eq!(x, 1.5);
        assert_eq!(y, 2.5);
        assert_eq!(z, 3.5);

        // Rotation get and set
        let mut rx = 0.0f32;
        let mut ry = 0.0f32;
        let mut rz = 0.0f32;
        assert!(petunia_qt_app_get_asset_rotation(
            0, &mut rx, &mut ry, &mut rz
        ));
        petunia_qt_app_set_asset_rotation(0, 45.0, 90.0, 180.0);
        petunia_qt_app_get_asset_rotation(0, &mut rx, &mut ry, &mut rz);
        assert_eq!(rx, 45.0);
        assert_eq!(ry, 90.0);
        assert_eq!(rz, 180.0);

        // Scale get and set
        let mut sx = 1.0f32;
        let mut sy = 1.0f32;
        let mut sz = 1.0f32;
        assert!(petunia_qt_app_get_asset_scale(0, &mut sx, &mut sy, &mut sz));
        petunia_qt_app_set_asset_scale(0, 2.0, 3.0, 0.5);
        petunia_qt_app_get_asset_scale(0, &mut sx, &mut sy, &mut sz);
        assert_eq!(sx, 2.0);
        assert_eq!(sy, 3.0);
        assert_eq!(sz, 0.5);

        // Color get and set
        let mut cr = 0.8f32;
        let mut cg = 0.8f32;
        let mut cb = 0.8f32;
        assert!(petunia_qt_app_get_asset_color(0, &mut cr, &mut cg, &mut cb));
        let original_color = [cr, cg, cb];
        petunia_qt_app_set_asset_color(0, 0.9, 0.4, 0.1);
        petunia_qt_app_get_asset_color(0, &mut cr, &mut cg, &mut cb);
        assert!((cr - 0.9).abs() < 1e-4);
        assert!((cg - 0.4).abs() < 1e-4);
        assert!((cb - 0.1).abs() < 1e-4);

        // Scene stats
        let mut verts = 0i32;
        let mut tris = 0i32;
        assert!(petunia_qt_app_get_scene_stats(&mut verts, &mut tris));
        assert!(verts > 0);
        assert!(tris > 0);

        // Selection domain & camera frame
        petunia_qt_app_set_selection_domain(1); // vertex
        petunia_qt_app_set_selection_domain(2); // edge
        petunia_qt_app_set_selection_domain(3); // face
        petunia_qt_app_set_selection_domain(0); // object
        petunia_qt_app_frame_selection();

        // Select asset
        petunia_qt_app_select_asset(0);

        // Property edits now participate in history: undo the last color edit.
        assert!(petunia_qt_app_undo());
        assert_eq!(petunia_qt_app_get_asset_count(), initial_count + 1);
        assert!(petunia_qt_app_get_asset_color(0, &mut cr, &mut cg, &mut cb));
        assert_eq!([cr, cg, cb], original_color);
        assert!(petunia_qt_app_redo());
        assert!(petunia_qt_app_get_asset_color(0, &mut cr, &mut cg, &mut cb));
        assert_eq!([cr, cg, cb], [0.9, 0.4, 0.1]);

        // Command palette search FFI
        let mut search_buf = [0i8; 2048];
        let query_cstr = std::ffi::CString::new("cube").unwrap();
        let matches =
            petunia_qt_app_search_commands(query_cstr.as_ptr(), search_buf.as_mut_ptr(), 2048);
        assert!(matches > 0);
        let results_str = std::ffi::CStr::from_ptr(search_buf.as_ptr())
            .to_str()
            .unwrap();
        assert!(results_str.contains("model.add_cube"));

        // Camera navigation FFI
        petunia_qt_camera_orbit(12.0, -8.0);
        petunia_qt_camera_pan(4.0, 2.0);
        petunia_qt_camera_zoom(1.5);
        petunia_qt_camera_reset();

        // Camera presets, projection, and HUD
        petunia_qt_camera_set_preset(1); // Front
        petunia_qt_camera_set_preset(3); // Right
        petunia_qt_camera_set_preset(5); // Top
        petunia_qt_camera_toggle_projection(); // Ortho
        petunia_qt_camera_toggle_projection(); // Persp

        let mut view_buf = [0i8; 64];
        assert!(petunia_qt_camera_get_view_name(view_buf.as_mut_ptr(), 64));
        let view_str = std::ffi::CStr::from_ptr(view_buf.as_ptr())
            .to_str()
            .unwrap();
        assert!(!view_str.is_empty());

        let mut rx = 0.0f32;
        let mut ry = 0.0f32;
        let mut rz = 0.0f32;
        let mut ux = 0.0f32;
        let mut uy = 0.0f32;
        let mut uz = 0.0f32;
        let mut fx = 0.0f32;
        let mut fy = 0.0f32;
        let mut fz = 0.0f32;
        assert!(petunia_qt_camera_get_orientation(
            &mut rx, &mut ry, &mut rz, &mut ux, &mut uy, &mut uz, &mut fx, &mut fy, &mut fz,
        ));
        assert!(rx.is_finite() && ux.is_finite() && fx.is_finite());

        // Shading modes (Wireframe, Solid, Material, Render)
        assert_eq!(petunia_qt_app_get_shading(), 1); // Solid default
        petunia_qt_app_set_shading(0); // Wireframe
        assert_eq!(petunia_qt_app_get_shading(), 0);
        petunia_qt_app_set_shading(2); // Material Preview
        assert_eq!(petunia_qt_app_get_shading(), 2);
        petunia_qt_app_set_shading(1); // Solid

        // Save and load project
        let temp_dir =
            std::env::temp_dir().join(format!("petunia_cxxqt_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let proj_file = temp_dir.join("test_studio.petunia");
        let proj_cstr = std::ffi::CString::new(proj_file.to_str().unwrap()).unwrap();
        assert!(petunia_qt_app_save_project(proj_cstr.as_ptr()));
        assert!(proj_file.exists());
        assert!(petunia_qt_app_load_project(proj_cstr.as_ptr()));

        // Export GLTF / GLB and Wavefront OBJ
        let glb_file = temp_dir.join("test_model.glb");
        let glb_cstr = std::ffi::CString::new(glb_file.to_str().unwrap()).unwrap();
        assert!(petunia_qt_app_export_gltf(glb_cstr.as_ptr()));
        assert!(glb_file.exists());
        assert_eq!(&std::fs::read(&glb_file).unwrap()[..4], b"glTF");
        // Open clears selection; OBJ exports the explicitly active asset.
        assert_eq!(petunia_qt_app_selected_count(), 0);
        petunia_qt_app_select_asset(0);

        let obj_file = temp_dir.join("test_model.obj");
        let obj_cstr = std::ffi::CString::new(obj_file.to_str().unwrap()).unwrap();
        assert!(petunia_qt_app_export_obj(obj_cstr.as_ptr()));
        assert!(obj_file.exists());
        let obj = std::fs::read_to_string(&obj_file).unwrap();
        assert!(obj.lines().any(|line| line.starts_with("v ")));
        assert!(obj.lines().any(|line| line.starts_with("f ")));

        let _ = std::fs::remove_dir_all(&temp_dir);

        // Tablet stylus event FFI with pressure and tilt
        petunia_qt_tablet_paint_stroke(100.0, 150.0, 0.75, 12.0, -5.0, false);
        petunia_qt_tablet_paint_stroke(105.0, 155.0, 0.90, 10.0, -4.0, true);
    }
}

#[test]
fn test_widgets_studio_window_creation() {
    reset_editor_fixture();
    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        let _app = petunia_ui_cxxqt::widgets::PetuniaWidgetsApp::new();
        let window = petunia_ui_cxxqt::widgets::PetuniaStudioWindow::new();
        window.show();
    }
}

#[test]
fn test_viewport_raycast_selection_and_domains() {
    reset_editor_fixture();
    // 1. Domain switching
    petunia_qt_app_set_selection_domain(0); // Object
    assert_eq!(petunia_qt_app_get_selection_domain(), 0);

    // 2. Outliner selection
    petunia_qt_app_select_asset(0);
    assert_eq!(petunia_qt_app_get_active_asset(), 0);

    // 3. Viewport selection in Object mode at center (where cube is positioned)
    let hit_center = petunia_qt_viewport_select(400.0, 300.0, 800.0, 600.0, false);
    assert!(hit_center);
    assert_eq!(petunia_qt_app_selected_count(), 1);

    // 4. Viewport selection in empty space (corner 10, 10) deselects
    let hit_corner = petunia_qt_viewport_select(10.0, 10.0, 800.0, 600.0, false);
    assert!(!hit_corner);
    assert_eq!(petunia_qt_app_selected_count(), 0);

    // 5. Select back
    assert!(petunia_qt_viewport_select(
        400.0, 300.0, 800.0, 600.0, false
    ));
    assert_eq!(petunia_qt_app_selected_count(), 1);

    // 6. Hover test
    let _ = petunia_qt_viewport_hover(400.0, 300.0, 800.0, 600.0);
    let _ = petunia_qt_viewport_hover(10.0, 10.0, 800.0, 600.0);

    // 7. Vertex mode
    petunia_qt_app_set_selection_domain(1); // Vertex
    assert_eq!(petunia_qt_app_get_selection_domain(), 1);
    let _ = petunia_qt_viewport_select(400.0, 300.0, 800.0, 600.0, false);

    // 8. Edge mode
    petunia_qt_app_set_selection_domain(2); // Edge
    assert_eq!(petunia_qt_app_get_selection_domain(), 2);
    let _ = petunia_qt_viewport_select(400.0, 300.0, 800.0, 600.0, false);

    // 9. Face mode
    petunia_qt_app_set_selection_domain(3); // Face
    assert_eq!(petunia_qt_app_get_selection_domain(), 3);
    let _ = petunia_qt_viewport_select(400.0, 300.0, 800.0, 600.0, false);

    // 10. Return to Object mode
    petunia_qt_app_set_selection_domain(0);
    assert_eq!(petunia_qt_app_get_selection_domain(), 0);
}

#[test]
fn test_modifiers_stack_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        petunia_qt_app_select_asset(0);
        let initial_mods = petunia_qt_app_get_modifier_count(0);

        // Add Mirror modifier (kind=0, axis=0: X, weld=0.001)
        let added = petunia_qt_app_add_modifier(0, 0, 0, 0.001);
        assert!(added);
        assert_eq!(petunia_qt_app_get_modifier_count(0), initial_mods + 1);

        // Get modifier info
        let mut name_buf = [0i8; 64];
        let mut enabled = false;
        let mut axis = -1;
        let mut weld = 0.0f32;
        let ok = petunia_qt_app_get_modifier_info(
            0,
            initial_mods,
            name_buf.as_mut_ptr(),
            64,
            &mut enabled,
            &mut axis,
            &mut weld,
        );
        assert!(ok);
        assert!(enabled);
        assert_eq!(axis, 0);

        // Toggle enabled
        petunia_qt_app_toggle_modifier(0, initial_mods, false);
        let _ = petunia_qt_app_get_modifier_info(
            0,
            initial_mods,
            name_buf.as_mut_ptr(),
            64,
            &mut enabled,
            &mut axis,
            &mut weld,
        );
        assert!(!enabled);

        // Add second modifier (Symmetry kind=1)
        let added2 = petunia_qt_app_add_modifier(0, 1, 1, 0.002);
        assert!(added2);
        assert_eq!(petunia_qt_app_get_modifier_count(0), initial_mods + 2);

        // Move modifier down and up
        assert!(petunia_qt_app_move_modifier(0, initial_mods, false));
        assert!(petunia_qt_app_move_modifier(0, initial_mods + 1, true));

        // Remove one modifier
        let removed = petunia_qt_app_remove_modifier(0, initial_mods + 1);
        assert!(removed);
        assert_eq!(petunia_qt_app_get_modifier_count(0), initial_mods + 1);

        // Re-enable before apply (disabled modifiers cannot be applied)
        petunia_qt_app_toggle_modifier(0, initial_mods, true);

        // Apply modifier
        let applied = petunia_qt_app_apply_modifier(0, initial_mods);
        assert!(applied);
        assert_eq!(petunia_qt_app_get_modifier_count(0), initial_mods);
    }
}

#[test]
fn test_pbr_material_and_surface_shading_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        petunia_qt_app_select_asset(0);

        // Test PBR properties
        let mut rough = 0.0f32;
        let mut metal = 0.0f32;
        let mut emission = 0.0f32;
        let mut prof = 0i32;

        assert!(petunia_qt_app_get_material_pbr(
            0,
            &mut rough,
            &mut metal,
            &mut emission,
            &mut prof
        ));

        // Update PBR material properties
        assert!(petunia_qt_app_set_material_pbr(0, 0.35, 0.85, 2.5, 0));
        assert!(petunia_qt_app_get_material_pbr(
            0,
            &mut rough,
            &mut metal,
            &mut emission,
            &mut prof
        ));
        assert!((rough - 0.35).abs() < 1e-4);
        assert!((metal - 0.85).abs() < 1e-4);
        assert!((emission - 2.5).abs() < 1e-4);
        assert_eq!(prof, 0);

        // Surface Shading: Flat vs Smooth
        assert_eq!(petunia_qt_app_get_surface_shading(0), 0); // Flat default
        assert!(petunia_qt_app_set_surface_shading(0, 1)); // Smooth
        assert_eq!(petunia_qt_app_get_surface_shading(0), 1);
        assert!(petunia_qt_app_set_surface_shading(0, 0)); // Flat
        assert_eq!(petunia_qt_app_get_surface_shading(0), 0);
    }
}

#[test]
fn test_quick_actions_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        petunia_qt_app_select_asset(0);

        let sel_all_cmd = std::ffi::CString::new("select_all").unwrap();
        assert!(petunia_qt_app_quick_action(sel_all_cmd.as_ptr()));

        let desel_cmd = std::ffi::CString::new("deselect_all").unwrap();
        assert!(petunia_qt_app_quick_action(desel_cmd.as_ptr()));

        let orig_cmd = std::ffi::CString::new("origin_to_geometry").unwrap();
        assert!(petunia_qt_app_quick_action(orig_cmd.as_ptr()));
    }
}

#[test]
fn test_3d_transform_gizmo_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        petunia_qt_app_select_asset(0);

        let mut orig = [0.0f32; 2];
        let mut px = [0.0f32; 2];
        let mut py = [0.0f32; 2];
        let mut pz = [0.0f32; 2];

        let projected = petunia_qt_gizmo_get_projected(
            800.0,
            600.0,
            orig.as_mut_ptr(),
            px.as_mut_ptr(),
            py.as_mut_ptr(),
            pz.as_mut_ptr(),
        );
        assert!(projected);
        assert!(orig[0].is_finite() && orig[1].is_finite());
        assert!(px[0].is_finite() && py[0].is_finite() && pz[0].is_finite());

        // Center hit test
        let hit_center = petunia_qt_gizmo_hit_test(orig[0], orig[1], 800.0, 600.0);
        assert_eq!(hit_center, 4); // Center

        // Miss hit test
        let hit_miss = petunia_qt_gizmo_hit_test(10.0, 10.0, 800.0, 600.0);
        assert_eq!(hit_miss, 0); // None

        // Drag axis X
        let mut initial_pos = [0.0f32; 3];
        assert!(petunia_qt_app_get_asset_position(
            0,
            &mut initial_pos[0],
            &mut initial_pos[1],
            &mut initial_pos[2]
        ));
        assert!(petunia_qt_gizmo_drag_axis(1, 10.0, 0.0, 800.0, 600.0));
        let mut new_pos = [0.0f32; 3];
        assert!(petunia_qt_app_get_asset_position(
            0,
            &mut new_pos[0],
            &mut new_pos[1],
            &mut new_pos[2]
        ));
        // Position should be modified
        assert_ne!(initial_pos[0], new_pos[0]);
    }
}

#[test]
fn test_uv_canvas_and_diagnostics_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        petunia_qt_app_select_asset(0);

        let faces = petunia_qt_app_get_uv_face_count(0);
        assert!(faces > 0);

        let mut uv_buf = [0.0f32; 8]; // 4 UV pairs
        let n_verts = petunia_qt_app_get_uv_face_verts(0, 0, uv_buf.as_mut_ptr(), 4);
        assert!(n_verts >= 3);

        let mut islands = 0i32;
        let mut overlap = 0.0f32;
        let mut stretch = 0.0f32;
        assert!(petunia_qt_app_get_uv_diagnostics(
            0,
            &mut islands,
            &mut overlap,
            &mut stretch
        ));
        assert!(islands >= 1);
        assert!(overlap >= 0.0);
        assert!(stretch >= 0.0);
    }
}

#[test]
fn test_timeline_and_dopesheet_ffi() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        reset_editor_fixture();
        petunia_qt_app_select_asset(0);
        petunia_ui_cxxqt::render::opengl_viewport::VIEWPORT_RENDERER
            .with(|renderer| {
                let mut host = renderer.borrow_mut();
                let e = &mut host.app;
                e.request(&serde_json::json!({"action":"workspace","id":"ANIMATE"}))
                    .unwrap();
                let id = e.snapshot()["animate"]["creature_presets"][0]["id"].clone();
                e.request(&serde_json::json!({"action":"animate","id":"add-creature","arg":id}))
                    .unwrap();
                let bone = e.snapshot()["animate"]["bones"][0]["id"].clone();
                e.request(&serde_json::json!({"action":"animate","id":"select-bone","arg":bone}))
                    .unwrap();
            })
            .unwrap();

        let mut cur = 0i32;
        let mut total = 0i32;
        let mut playing = false;
        assert!(petunia_qt_app_get_timeline_info(
            &mut cur,
            &mut total,
            &mut playing
        ));
        assert_eq!(cur, 0);
        let expected = petunia_ui_cxxqt::render::opengl_viewport::VIEWPORT_RENDERER
            .with(|renderer| {
                renderer
                    .borrow()
                    .app
                    .controller
                    .animate_view_model()
                    .total_frames
            })
            .unwrap();
        assert_eq!(total, expected);
        assert!(total >= 42);

        // Frame setting
        petunia_qt_app_set_timeline_frame(42);
        assert!(petunia_qt_app_get_timeline_info(
            &mut cur,
            &mut total,
            &mut playing
        ));
        assert_eq!(cur, 42);

        // Keyframes
        assert!(!petunia_qt_app_has_keyframe(0, 42));
        assert!(petunia_qt_app_insert_keyframe(0, 42));
        assert!(petunia_qt_app_has_keyframe(0, 42));
        // Authored keyframes play even without a procedural Motion.
        assert!(petunia_qt_app_toggle_playback());
        assert!(!petunia_qt_app_toggle_playback());
        assert!(petunia_qt_app_delete_keyframe(0, 42));
        assert!(!petunia_qt_app_has_keyframe(0, 42));

        // An empty clip cannot play; add a real procedural Motion for this path.
        assert!(!petunia_qt_app_toggle_playback());
        petunia_ui_cxxqt::render::opengl_viewport::VIEWPORT_RENDERER
            .with(|renderer| {
                let mut host = renderer.borrow_mut();
                let e = &mut host.app;
                let motion = e.snapshot()["animate"]["catalog"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["available"] == true)
                    .unwrap()["id"]
                    .clone();
                e.request(&serde_json::json!({"action":"animate","id":"add-motion","arg":motion}))
                    .unwrap();
                e.request(&serde_json::json!({"action":"animate","id":"seek","value":0.}))
                    .unwrap();
            })
            .unwrap();
        let now_playing = petunia_qt_app_toggle_playback();
        assert!(now_playing);
        let now_stopped = petunia_qt_app_toggle_playback();
        assert!(!now_stopped);

        // Reset frame
        petunia_qt_app_set_timeline_frame(1);
    }
}
