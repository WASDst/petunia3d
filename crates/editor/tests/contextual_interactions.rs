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
