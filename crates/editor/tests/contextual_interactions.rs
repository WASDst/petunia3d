use glam::Vec3;
use petunia_core::{AppState, NavigationAnchor, PivotPoint};
use petunia_editor::{
    GrammarTool, ModelingMode, PlaceholderViewport, SlintUiBridge, UiIntent, ViewportGesture,
};

fn bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    b.resize_viewport(800, 600);
    b
}

#[test]
fn presets_share_tools_and_restore_the_free_camera_without_document_change() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("bevel".into()));
    let free = b.state.session.camera.clone();
    let assets = serde_json::to_value(&b.state.project.project.assets).unwrap();
    let selection = b.state.session.selection.clone();
    b.apply(UiIntent::SetModelingMode(ModelingMode::Draw));
    assert_eq!(b.state.active_tool, "bevel");
    assert_eq!(b.state.session.camera.proj, petunia_core::Projection::Ortho);
    b.apply(UiIntent::SetActiveTool("shape_builder".into()));
    b.apply(UiIntent::SetModelingMode(ModelingMode::Poly));
    assert_eq!(b.state.active_tool, "shape_builder");
    assert_eq!(b.grammar_tool(), Some(GrammarTool::ShapeBuilder));
    assert_eq!(b.state.session.camera, free);
    assert_eq!(
        serde_json::to_value(&b.state.project.project.assets).unwrap(),
        assets
    );
    assert_eq!(b.state.session.selection, selection);
}

#[test]
fn navigation_anchor_is_explicit_and_switching_it_does_not_move_camera() {
    let mut b = bridge();
    b.state.session.cursor_3d = [3.0, 2.0, 1.0];
    let before = b.state.session.camera.clone();
    b.apply(UiIntent::SetNavigationAnchor(NavigationAnchor::Cursor3D));
    assert_eq!(b.state.session.camera, before);
    let anchor = Vec3::from(b.state.session.cursor_3d);
    let old_distance = (before.eye() - anchor).length();
    b.orbit_viewport(20.0, 10.0);
    assert!(((b.state.session.camera.eye() - anchor).length() - old_distance).abs() < 1e-4);
    let cursor = b.state.session.cursor_3d;
    b.apply_viewport_gesture(ViewportGesture::Pan { dx: 12.0, dy: 8.0 });
    assert_eq!(b.state.session.cursor_3d, cursor);
    let screen_before = b.state.session.camera.project_ndc(anchor);
    b.apply_viewport_gesture(ViewportGesture::Zoom { delta: 70.0 });
    let screen_after = b.state.session.camera.project_ndc(anchor);
    assert!((screen_before.truncate() - screen_after.truncate()).length() < 1e-4);
}

#[test]
fn navigation_does_not_consult_transform_pivot_or_active_cursor_tool() {
    let mut a = bridge();
    let mut b = bridge();
    b.state.session.pivot_point = PivotPoint::Cursor3D;
    b.state.session.cursor_3d = [50.0, 60.0, 70.0];
    b.state.active_tool = "cursor_3d".into();
    a.orbit_viewport(25.0, 7.0);
    b.orbit_viewport(25.0, 7.0);
    assert_eq!(a.state.session.camera, b.state.session.camera);
}

#[test]
fn xray_wireframe_and_select_through_have_independent_state() {
    let mut b = bridge();
    b.state.session.show_xray = true;
    b.state.session.shading = petunia_core::Shading::Wireframe;
    assert!(!b.state.session.select_through);
    b.apply(UiIntent::SetSelectThrough(true));
    b.state.session.show_xray = false;
    assert!(b.state.session.select_through);
    assert!(b.state.session.viewport_session().select_through);
}

#[test]
fn removed_select_shortcut_has_no_physical_w_fallback() {
    let mut b = bridge();
    b.state.ui.keybinds.remove_binding("model.tool_select");
    b.state.active_tool = "rotate".into();
    assert!(!b.route_shortcut("W", false, false, false));
    assert_eq!(b.state.active_tool, "rotate");
}

#[test]
fn uv_shortcuts_open_a_domain_preview_and_enter_escape_use_its_transaction() {
    let mut b = bridge();
    b.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Uv));
    let mesh = b.state.project.active_mesh().unwrap().clone();
    b.state.session.uv_selection.sync = false;
    b.state.session.uv_selection.set_faces(&mesh, [0]);
    let source = b.state.project.active_mesh().unwrap().faces[0].uv.clone();
    assert!(b.route_shortcut("G", false, false, false));
    assert!(b.state.modal.is_none());
    assert!(b.state.session.tools.uv_transform.is_some());
    b.apply(UiIntent::UpdateUvTransform(
        petunia_mesh::UvTransform::Translate(glam::Vec2::splat(0.2)),
    ));
    assert_eq!(b.state.project.active_mesh().unwrap().faces[0].uv, source);
    assert!(b.route_shortcut("Escape", false, false, false));
    assert!(b.state.session.tools.uv_transform.is_none());
    assert_eq!(b.state.project.active_mesh().unwrap().faces[0].uv, source);
    assert!(b.route_shortcut("G", false, false, false));
    b.apply(UiIntent::UpdateUvTransform(
        petunia_mesh::UvTransform::Translate(glam::Vec2::splat(0.2)),
    ));
    assert!(b.route_shortcut("Enter", false, false, false));
    assert_ne!(b.state.project.active_mesh().unwrap().faces[0].uv, source);
}

#[cfg(feature = "animation-workspace")]
#[test]
fn animate_keys_do_not_delete_geometry_or_switch_to_previous_tool() {
    let mut b = bridge();
    b.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Animate));
    let id = b
        .state
        .animate_add_rig(petunia_core::RigPresetKind::Humanoid)
        .unwrap();
    let bone = b.state.project.project.get_skeleton(id).unwrap().bones[1].id;
    b.state.animate_select_bone(Some(bone));
    let assets = serde_json::to_value(&b.state.project.project.assets).unwrap();
    assert!(b.route_shortcut("I", false, false, false));
    assert!(b.state.session.animate.active_clip.is_some());
    assert!(b.route_shortcut("Space", false, false, false));
    assert!(b.state.session.animate.playing);
    assert!(b.state.session.animate.set_timeline_frame_rate(60.0));
    assert!(b.route_shortcut("Right", false, false, false));
    assert!((b.state.session.animate.time - 1.0 / 60.0).abs() < 1e-6);
    b.state.animate_seek(0.0);
    assert!(b.route_shortcut("Delete", false, false, false));
    assert_eq!(
        serde_json::to_value(&b.state.project.project.assets).unwrap(),
        assets
    );
    b.state
        .ui
        .keybinds
        .remove_binding("animate.delete_keyframe");
    assert!(!b.route_shortcut("Delete", false, false, false));
    assert_eq!(
        serde_json::to_value(&b.state.project.project.assets).unwrap(),
        assets
    );
}

#[test]
fn transform_center_copies_cursor_once_and_recopy_is_explicit() {
    let mut b = bridge();
    b.state.session.cursor_3d = [1.0, 2.0, 3.0];
    assert!(b.state.session.copy_cursor_to_transform_center());
    b.state.session.cursor_3d = [4.0, 5.0, 6.0];
    assert_eq!(
        b.state.calculate_pivot(PivotPoint::Cursor3D),
        Vec3::new(1.0, 2.0, 3.0)
    );
    assert!(b.state.session.copy_cursor_to_transform_center());
    assert_eq!(
        b.state.calculate_pivot(PivotPoint::Cursor3D),
        Vec3::new(4.0, 5.0, 6.0)
    );
    let camera = b.state.session.camera.clone();
    assert!(!b.place_cursor_3d(f32::NAN, 0.5));
    assert_eq!(b.state.session.camera, camera);
    assert_eq!(b.state.session.cursor_3d, [4.0, 5.0, 6.0]);
}

#[test]
fn effective_profiles_override_physical_fallbacks_and_removed_keys_stay_removed() {
    let mut b = bridge();
    b.state.ui.keybinds = petunia_config::keybinds::Keybinds::load_profile("3ds-max");
    let shading = b.state.session.shading;
    assert!(b.route_shortcut("Z", false, false, false));
    assert_eq!(
        b.state.session.shading, shading,
        "Max Z frames, it must not toggle wireframe"
    );
    b.state.ui.keybinds = petunia_config::keybinds::Keybinds::load_profile("cinema-4d");
    assert!(b.route_shortcut("D", false, false, false));
    assert!(
        !b.state.session.edit_pivot,
        "Cinema D activates extrude, not pivot edit"
    );
    b.state.ui.keybinds = petunia_config::keybinds::Keybinds::load_profile("petunia-notebook");
    assert!(!b.route_shortcut("1", false, false, false));
    b.state.ui.keybinds.remove_binding("model.delete");
    let before = serde_json::to_value(&b.state.project.project.assets).unwrap();
    assert!(!b.route_shortcut("Delete", false, false, false));
    assert_eq!(
        serde_json::to_value(&b.state.project.project.assets).unwrap(),
        before
    );
    b.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Paint));
    b.state.ui.keybinds.remove_binding("paint.size_decrease");
    let size = b.state.canvas_brush;
    assert!(!b.route_shortcut("[", false, false, false));
    assert_eq!(b.state.canvas_brush, size);
}

#[test]
fn contextual_picking_prioritizes_subcomponents_over_objects_when_all_filters_enabled() {
    let mut b = bridge();
    assert!(b.state.project.active().is_some());

    b.state.session.selection.filters = petunia_core::SelectionFilters::objects_only();
    let hit_obj = b.pick_viewport_target(0.5, 0.5);
    assert!(matches!(hit_obj, petunia_core::HoverTarget::Object(_)));

    b.state.session.selection.filters = petunia_core::SelectionFilters::all();
    let hit_ctx = b.pick_viewport_target(0.5, 0.5);
    assert!(matches!(
        hit_ctx,
        petunia_core::HoverTarget::Face(_)
            | petunia_core::HoverTarget::Vertex(_)
            | petunia_core::HoverTarget::Edge(_, _)
    ));

    b.state.session.selection.filters.faces = false;
    b.state.session.selection.filters.vertices = false;
    b.state.session.selection.filters.edges = false;
    let hit_fallback = b.pick_viewport_target(0.5, 0.5);
    assert!(matches!(hit_fallback, petunia_core::HoverTarget::Object(_)));
}

#[test]
fn navigation_decoupling_pan_frame_and_history_invariants_h20() {
    let mut b = bridge();
    b.state.session.cursor_3d = [10.0, 20.0, 30.0];
    let cursor = b.state.session.cursor_3d;

    // Pan viewport: cursor_3d MUST NOT move
    b.apply_viewport_gesture(ViewportGesture::Pan {
        dx: 45.0,
        dy: -30.0,
    });
    assert_eq!(b.state.session.cursor_3d, cursor);

    // Zoom viewport: cursor_3d MUST NOT move
    b.apply_viewport_gesture(ViewportGesture::Zoom { delta: 120.0 });
    assert_eq!(b.state.session.cursor_3d, cursor);

    // Orbit viewport: cursor_3d MUST NOT move
    b.apply_viewport_gesture(ViewportGesture::Orbit { dx: 15.0, dy: 10.0 });
    assert_eq!(b.state.session.cursor_3d, cursor);

    // Navigation and camera actions MUST NOT create Undo entries or mark document dirty
    assert!(!b.state.project.undo.can_undo());
    assert!(!b.state.project.is_dirty());

    // Switching NavigationAnchor MUST NOT create Undo entries or dirty document
    b.apply(UiIntent::SetNavigationAnchor(NavigationAnchor::Cursor3D));
    assert_eq!(
        b.state.session.navigation_anchor,
        NavigationAnchor::Cursor3D
    );
    assert!(!b.state.project.undo.can_undo());
    assert!(!b.state.project.is_dirty());

    // Placing cursor with invalid floats MUST be rejected safely
    assert!(!b.place_cursor_3d(f32::NAN, 0.5));
    assert!(!b.place_cursor_3d(0.5, f32::INFINITY));
    assert_eq!(b.state.session.cursor_3d, cursor);

    // Freezing transform center keeps pivot isolated until explicit recopy
    assert!(b.state.session.copy_cursor_to_transform_center());
    b.state.session.cursor_3d = [100.0, 200.0, 300.0];
    assert_eq!(
        b.state.calculate_pivot(PivotPoint::Cursor3D),
        Vec3::new(10.0, 20.0, 30.0)
    );
    assert!(b.state.session.copy_cursor_to_transform_center());
    assert_eq!(
        b.state.calculate_pivot(PivotPoint::Cursor3D),
        Vec3::new(100.0, 200.0, 300.0)
    );
}

#[test]
fn draw_poly_preset_switching_and_roundtrip_invariants_h21() {
    let mut b = bridge();
    let initial_assets = serde_json::to_value(&b.state.project.project.assets).unwrap();
    let initial_selection = b.state.session.selection.clone();
    let initial_camera = b.state.session.camera.clone();

    // 1. Switch to DRAW mode: sets Ortho and Draw mode without mutating geometry or selection
    b.apply(UiIntent::SetModelingMode(ModelingMode::Draw));
    assert_eq!(b.modeling_mode, ModelingMode::Draw);
    assert_eq!(b.state.session.camera.proj, petunia_core::Projection::Ortho);
    assert_eq!(
        serde_json::to_value(&b.state.project.project.assets).unwrap(),
        initial_assets
    );
    assert_eq!(b.state.session.selection, initial_selection);
    assert!(!b.state.project.undo.can_undo());

    // 2. In DRAW mode, free orbit drag is redirected to planar pan (gesture is never dead)
    let camera_before_orbit = b.state.session.camera.clone();
    b.apply_viewport_gesture(ViewportGesture::Orbit { dx: 20.0, dy: 10.0 });
    // Camera remains Ortho; orientation did not pitch/yaw freely
    assert_eq!(b.state.session.camera.proj, petunia_core::Projection::Ortho);
    assert_ne!(b.state.session.camera.eye(), camera_before_orbit.eye());

    // 3. Return to POLY mode: restores camera without creating Undo entry
    b.apply(UiIntent::SetModelingMode(ModelingMode::Poly));
    assert_eq!(b.modeling_mode, ModelingMode::Poly);
    assert_eq!(b.state.session.camera.proj, initial_camera.proj);
    assert_eq!(
        serde_json::to_value(&b.state.project.project.assets).unwrap(),
        initial_assets
    );
    assert_eq!(b.state.session.selection, initial_selection);
    assert!(!b.state.project.undo.can_undo());
}

#[test]
fn shape_builder_explicit_operands_and_hole_preservation_invariants_h24() {
    let mut b = bridge();
    let plane = petunia_core::ProfileWorkplane {
        origin: [0.0, 0.0, 5.0],
        right: [1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    };

    let add_square =
        |b: &mut SlintUiBridge<PlaceholderViewport>, x: f64, y: f64, size: f64, name: &str| {
            let pts = [
                [x, y, 0.0],
                [x + size, y, 0.0],
                [x + size, y + size, 0.0],
                [x, y + size, 0.0],
            ];
            let spline = petunia_core::SplineResource::from_polyline(name, &pts, true);
            let profile = petunia_core::ProfileResource::new(name, spline.id, plane);
            let id = profile.id;
            b.state.project.project.splines.push(spline);
            b.state.project.project.profiles.push(profile);
            id
        };

    // 1. Setup 3 squares: sq1 and sq2 overlap, sq3 is distant
    let sq1_id = add_square(&mut b, 0.0, 0.0, 2.0, "sq1");
    let sq2_id = add_square(&mut b, 1.0, 1.0, 2.0, "sq2");
    let sq3_id = add_square(&mut b, 10.0, 10.0, 2.0, "sq3");

    // Add a hole in sq1
    let hole = vec![[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]];
    if let Some(sq1) = b.state.project.get_profile_mut(sq1_id) {
        sq1.holes.push(hole);
    }

    assert_eq!(b.state.project.project.profiles.len(), 3);
    let initial_undo = b.state.project.undo.depth().0;

    // 2. Reject invalid operands gracefully without panicking
    let cmd_empty = petunia_core::ShapeEditCmd {
        plane: None,
        operands: Some(vec![]),
        edit: petunia_core::ShapeEdit::Pathfinder(petunia_core::PathfinderOp::Unite),
    };
    assert!(b.state.dispatch(&cmd_empty).is_err());

    let cmd_single = petunia_core::ShapeEditCmd {
        plane: None,
        operands: Some(vec![sq1_id]),
        edit: petunia_core::ShapeEdit::Pathfinder(petunia_core::PathfinderOp::Unite),
    };
    assert!(b.state.dispatch(&cmd_single).is_err());

    // 3. Dispatch Unite with explicit operands (sq1 and sq2 only)
    let cmd = petunia_core::ShapeEditCmd::pathfinder_with_operands(
        petunia_core::PathfinderOp::Unite,
        vec![sq1_id, sq2_id],
    );
    b.state
        .dispatch(&cmd)
        .expect("Unite with explicit operands should succeed");

    // 4. Untouched sq3 preserves ID, name and spline
    let sq3 = b
        .state
        .project
        .get_profile(sq3_id)
        .expect("sq3 must be preserved");
    assert_eq!(sq3.id, sq3_id);
    assert_eq!(sq3.name, "sq3");

    // 5. Resulting merged shape preserves the hole from sq1
    assert_eq!(b.state.project.project.profiles.len(), 2);
    let merged = b
        .state
        .project
        .project
        .profiles
        .iter()
        .find(|p| p.id != sq3_id)
        .expect("merged profile must exist");
    assert_eq!(
        merged.holes.len(),
        1,
        "Hole must be preserved in merged profile"
    );

    // 6. Undo restores all 3 original profiles atomically (single undo step)
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 1);
    b.state.undo();
    assert_eq!(b.state.project.project.profiles.len(), 3);
    let sq1_restored = b
        .state
        .project
        .get_profile(sq1_id)
        .expect("sq1 must be restored");
    assert_eq!(sq1_restored.holes.len(), 1);
}

#[test]
fn solid_shape_builder_two_operands_and_undo_invariants_h25() {
    let mut b = bridge();
    let mut cube1 = petunia_mesh::Mesh::cube(2.0);
    cube1.triangulate();
    for f in &mut cube1.faces {
        f.material_slot = Some(1);
    }
    b.state.project.assets[0].mesh = cube1.into();
    let asset1_id = b.state.project.assets[0].id;

    let mut cube2 = petunia_mesh::Mesh::cube(2.0);
    for v in &mut cube2.verts {
        v.pos[0] += 1.0;
    }
    cube2.triangulate();
    for f in &mut cube2.faces {
        f.material_slot = Some(2);
    }
    let asset2 = petunia_project::Asset::new("Cube2", cube2);
    let asset2_id = asset2.id;
    b.state.project.assets.push(asset2);

    let mut cube3 = petunia_mesh::Mesh::cube(2.0);
    for v in &mut cube3.verts {
        v.pos[0] += 10.0;
    }
    cube3.triangulate();
    let asset3 = petunia_project::Asset::new("Cube3", cube3);
    let asset3_id = asset3.id;
    b.state.project.assets.push(asset3);

    assert_eq!(b.state.project.assets.len(), 3);
    let initial_undo = b.state.project.undo.depth().0;

    // 0. Invalid operands rejection: non-existent operand or active asset as operand
    let cmd_invalid = petunia_core::SolidShapeEditCmd::with_operand(
        petunia_core::SolidOperation::UniteAll,
        uuid::Uuid::new_v4(),
    );
    assert!(b.state.dispatch(&cmd_invalid).is_err());

    let cmd_self = petunia_core::SolidShapeEditCmd::with_operand(
        petunia_core::SolidOperation::UniteAll,
        asset1_id,
    );
    assert!(b.state.dispatch(&cmd_self).is_err());

    // 1. Extract Overlap: extracts shared volume into a 4th asset atomically
    let cmd_extract = petunia_core::SolidShapeEditCmd::extract_overlap_with_operand(asset2_id);
    b.state
        .dispatch(&cmd_extract)
        .expect("extract overlap should succeed");
    assert_eq!(b.state.project.assets.len(), 4);
    assert_eq!(b.state.project.assets[2].id, asset3_id, "Cube3 untouched");
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 1);

    // 2. Undo restores the 3 original objects
    b.state.undo();
    assert_eq!(b.state.project.assets.len(), 3);
    assert_eq!(b.state.project.undo.depth().0, initial_undo);

    // 3. Remove Overlap: cuts overlap from both operands, preserving material slots
    let cmd_remove = petunia_core::SolidShapeEditCmd::remove_overlap_with_operand(asset2_id);
    b.state
        .dispatch(&cmd_remove)
        .expect("remove overlap should succeed");
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 1);
    assert!(
        b.state.project.assets[0]
            .mesh
            .faces
            .iter()
            .any(|f| f.material_slot == Some(1))
    );
    assert!(
        b.state.project.assets[1]
            .mesh
            .faces
            .iter()
            .any(|f| f.material_slot == Some(2))
    );
    assert_eq!(b.state.project.assets[2].id, asset3_id, "Cube3 untouched");

    b.state.undo();
    assert_eq!(b.state.project.undo.depth().0, initial_undo);

    // 4. Unite: merges all regions
    let cmd_unite = petunia_core::SolidShapeEditCmd::unite_with_operand(asset2_id);
    b.state
        .dispatch(&cmd_unite)
        .expect("solid unite should succeed");
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 1);
    assert_eq!(b.state.project.assets[2].id, asset3_id, "Cube3 untouched");

    b.state.undo();
    assert_eq!(b.state.project.undo.depth().0, initial_undo);
    assert_eq!(b.state.project.assets.len(), 3);
}

#[test]
fn solid_composition_multi_solid_and_reedit_invariants_h26() {
    let mut b = bridge();

    // Setup 3 cubes:
    // Base cube at origin
    let mut cube1 = petunia_mesh::Mesh::cube(2.0);
    cube1.triangulate();
    b.state.project.assets[0].mesh = cube1.into();
    let _base_id = b.state.project.assets[0].id;

    // Cube 2 at (1.0, 0, 0)
    let mut cube2 = petunia_mesh::Mesh::cube(2.0);
    for v in &mut cube2.verts {
        v.pos[0] += 1.0;
    }
    cube2.triangulate();
    let asset2 = petunia_project::Asset::new("Cube2", cube2);
    let asset2_id = asset2.id;
    b.state.project.assets.push(asset2);

    // Cube 3 at (2.0, 0, 0)
    let mut cube3 = petunia_mesh::Mesh::cube(2.0);
    for v in &mut cube3.verts {
        v.pos[0] += 2.0;
    }
    cube3.triangulate();
    let asset3 = petunia_project::Asset::new("Cube3", cube3);
    let asset3_id = asset3.id;
    b.state.project.assets.push(asset3);

    let initial_undo = b.state.project.undo.depth().0;

    // 1. Create composition with Cube2 and Cube3 into Base
    let create_cmd = petunia_core::CreateSolidCompositionCmd::new(
        vec![asset2_id, asset3_id],
        petunia_core::SolidOperation::UniteAll,
    );
    b.state
        .dispatch(&create_cmd)
        .expect("Create solid composition should succeed");

    assert!(
        b.state.project.assets[0].is_composition(),
        "Base asset must have composition recipe"
    );
    assert!(
        !b.state.project.assets[1].visible,
        "Secondary operand 1 must be hidden"
    );
    assert!(
        !b.state.project.assets[2].visible,
        "Secondary operand 2 must be hidden"
    );
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 1);

    let recipe = b.state.project.assets[0].composition.as_ref().unwrap();
    assert_eq!(recipe.steps.len(), 2);
    assert_eq!(recipe.status, petunia_project::CompositionStatus::Valid);
    let step2_id = recipe.steps[1].id;

    // Direct polygon editing is rejected before bake ("Sem bake silencioso")
    let subdivide_cmd = petunia_core::command::SubdivideSelectionCmd;
    assert_eq!(
        b.state.dispatch(&subdivide_cmd),
        Err(petunia_core::command::CommandError::Execution(
            "Bake solid composition before direct polygon editing".to_string()
        ))
    );

    // 2. Toggle step (disable operand 3)
    let toggle_cmd = petunia_core::ToggleSolidCompositionStepCmd::new(step2_id, false);
    b.state
        .dispatch(&toggle_cmd)
        .expect("Toggle step should succeed");
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 2);
    let recipe_after = b.state.project.assets[0].composition.as_ref().unwrap();
    assert!(!recipe_after.steps[1].enabled);

    // 3. Invalidation check when an operand mesh changes topology
    let mut modified_mesh = petunia_mesh::Mesh::cube(3.0);
    modified_mesh.triangulate();
    b.state.project.assets[1].mesh = modified_mesh.into();
    let mut recipe_check = b.state.project.assets[0].composition.clone().unwrap();
    let invalidated = recipe_check.check_invalidation(&b.state.project.assets);
    assert!(invalidated);
    assert_eq!(
        recipe_check.status,
        petunia_project::CompositionStatus::NeedsEvaluation
    );

    // 4. Bake composition (converts to static mesh - "Sem bake silencioso")
    let bake_cmd = petunia_core::BakeSolidCompositionCmd::new();
    b.state
        .dispatch(&bake_cmd)
        .expect("Bake composition should succeed");
    assert!(
        !b.state.project.assets[0].is_composition(),
        "Bake must consume the recipe and leave static mesh"
    );
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 3);

    // After explicit bake, direct polygon editing is permitted
    assert_ne!(
        b.state.dispatch(&subdivide_cmd),
        Err(petunia_core::command::CommandError::Execution(
            "Bake solid composition before direct polygon editing".to_string()
        ))
    );

    // 5. Transactional Undo sequence
    assert!(b.state.undo(), "Undo bake");
    assert!(
        b.state.project.assets[0].is_composition(),
        "Undo must restore recipe"
    );

    assert!(b.state.undo(), "Undo toggle step");
    let restored_recipe = b.state.project.assets[0].composition.as_ref().unwrap();
    assert!(
        restored_recipe.steps[1].enabled,
        "Step 2 re-enabled by undo"
    );

    assert!(b.state.undo(), "Undo create composition");
    assert!(
        !b.state.project.assets[0].is_composition(),
        "Undo must clear composition"
    );
    assert!(
        b.state.project.assets[1].visible,
        "Undo must restore operand 1 visibility"
    );
    assert!(
        b.state.project.assets[2].visible,
        "Undo must restore operand 2 visibility"
    );
    assert_eq!(b.state.project.undo.depth().0, initial_undo);
}
