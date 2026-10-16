use petunia_core::{AppState, ModalConstraint, PrimitiveKind, ToolActivation};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, TransformKind, UiIntent};

fn bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    b.state.confirm_primitive();
    b
}

#[test]
fn test_h11_instant_activation_returns_to_origin_tool_on_commit() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("select".into()));
    b.apply(UiIntent::SetToolActivation(ToolActivation::Instant));
    assert_eq!(b.state.session.tools.active_tool, "select");

    // Begin modal transform (G / Move)
    b.apply(UiIntent::BeginModalTransform(TransformKind::Position));
    assert_eq!(b.state.session.tools.active_tool, "move");
    assert_eq!(
        b.state.session.tools.temporary_modal_origin_tool(),
        Some("select")
    );
    assert!(b.instant_transform);
    assert!(b.tool_session.is_latched());

    // Initial undo depth
    let initial_depth = b.state.project.undo.depth().0;

    // Move without holding button (click-move-click)
    b.tool_pointer(1, 100.0, 100.0, false, false);

    // Commit via Enter or Click
    b.apply(UiIntent::CommitModalTransform);

    // Assert return to origin tool
    assert_eq!(b.state.session.tools.active_tool, "select");
    assert_eq!(b.state.session.tools.temporary_modal_origin_tool(), None);
    assert!(!b.instant_transform);
    assert!(!b.tool_session.is_latched());

    // Exactly 1 undo entry added
    assert_eq!(b.state.project.undo.depth().0, initial_depth + 1);
}

#[test]
fn test_h11_instant_activation_returns_to_origin_tool_on_cancel() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("poly_pen".into()));
    b.apply(UiIntent::SetToolActivation(ToolActivation::Instant));
    assert_eq!(b.state.session.tools.active_tool, "poly_pen");

    // Begin modal transform (R / Rotate)
    b.apply(UiIntent::BeginModalTransform(TransformKind::Rotation));
    assert_eq!(b.state.session.tools.active_tool, "rotate");
    assert_eq!(
        b.state.session.tools.temporary_modal_origin_tool(),
        Some("poly_pen")
    );
    assert!(b.instant_transform);

    let initial_depth = b.state.project.undo.depth().0;

    // Pointer move
    b.tool_pointer(1, 50.0, 80.0, false, false);

    // Cancel via Esc
    b.apply(UiIntent::CancelModalTransform);

    // Restores origin tool
    assert_eq!(b.state.session.tools.active_tool, "poly_pen");
    assert_eq!(b.state.session.tools.temporary_modal_origin_tool(), None);
    assert!(!b.instant_transform);
    assert!(!b.tool_session.is_latched());

    // 0 undo entries on cancel
    assert_eq!(b.state.project.undo.depth().0, initial_depth);
}

#[test]
fn test_h11_blender_keymap_profile_enables_instant_transform() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("select".into()));
    b.apply(UiIntent::SetToolActivation(ToolActivation::Drag));
    b.state.ui.active_keymap_id = "blender".to_string();

    // In blender profile, G triggers instant transform even with Drag activation preference
    b.route_shortcut("g", false, false, false);
    assert_eq!(b.state.session.tools.active_tool, "move");
    assert_eq!(
        b.state.session.tools.temporary_modal_origin_tool(),
        Some("select")
    );
    assert!(b.instant_transform);
    assert!(b.tool_session.is_latched());

    // Esc restores select
    b.route_shortcut("Escape", false, false, false);
    assert_eq!(b.state.session.tools.active_tool, "select");
    assert_eq!(b.state.session.tools.temporary_modal_origin_tool(), None);
}

#[test]
fn test_h11_double_tap_in_drag_mode_triggers_instant_transform() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("select".into()));
    b.apply(UiIntent::SetToolActivation(ToolActivation::Drag));
    b.state.ui.active_keymap_id = "petunia-default".to_string();
    b.preferences.double_tap_interval_ms = 500;

    // Single tap in default keymap with Drag activation sets persistent tool
    b.apply(UiIntent::BeginModalTransform(TransformKind::Scale));
    assert_eq!(b.state.session.tools.active_tool, "scale");
    assert_eq!(b.state.session.tools.temporary_modal_origin_tool(), None);
    assert!(!b.instant_transform);

    // Return to select
    b.apply(UiIntent::SetActiveTool("select".into()));

    // Double tap: two calls within interval
    b.apply(UiIntent::BeginModalTransform(TransformKind::Scale));
    b.apply(UiIntent::BeginModalTransform(TransformKind::Scale));
    assert_eq!(b.state.session.tools.active_tool, "scale");
    assert_eq!(
        b.state.session.tools.temporary_modal_origin_tool(),
        Some("select")
    );
    assert!(b.instant_transform);
    assert!(b.tool_session.is_latched());

    // Cancel restores select
    b.apply(UiIntent::CancelModalTransform);
    assert_eq!(b.state.session.tools.active_tool, "select");
}

#[test]
fn test_h11_typed_numeric_input_overrides_mouse_and_commits_exact_delta() {
    let mut b = bridge();
    b.apply(UiIntent::SetToolActivation(ToolActivation::Instant));
    b.apply(UiIntent::BeginModalTransform(TransformKind::Position));

    // Lock to X axis
    b.route_shortcut("x", false, false, false);
    assert_eq!(
        b.state.session.tools.modal.as_ref().map(|m| m.constraint),
        Some(ModalConstraint::Axis(0))
    );

    // Move pointer (should not override numeric buffer once typed)
    b.tool_pointer(1, 200.0, 200.0, false, false);

    // Type "3.5"
    b.route_shortcut("3", false, false, false);
    b.route_shortcut(".", false, false, false);
    b.route_shortcut("5", false, false, false);
    assert_eq!(b.tool_session.numeric_text(), "3.5");

    // Pointer movement is ignored when numeric buffer is non-empty
    b.tool_pointer(1, 999.0, 999.0, false, false);
    assert_eq!(b.tool_session.numeric_text(), "3.5");

    // Press Backspace
    b.route_shortcut("Backspace", false, false, false);
    assert_eq!(b.tool_session.numeric_text(), "3.");

    // Type "0" -> "3.0"
    b.route_shortcut("0", false, false, false);
    assert_eq!(b.tool_session.numeric_text(), "3.0");

    // Confirm with Enter
    b.route_shortcut("Enter", false, false, false);

    // Active tool restored
    assert_eq!(b.state.session.tools.temporary_modal_origin_tool(), None);

    // Verify asset position changed by exactly 3.0 on X
    let asset = b.state.project.active().unwrap();
    assert!((asset.position[0] - 3.0).abs() < 1e-4);
}

#[test]
fn test_h11_axis_and_plane_constraints_and_toggle_to_free() {
    let mut b = bridge();
    b.apply(UiIntent::SetToolActivation(ToolActivation::Instant));
    b.apply(UiIntent::BeginModalTransform(TransformKind::Position));

    // Press X -> Axis(0)
    b.route_shortcut("x", false, false, false);
    assert_eq!(
        b.state.session.tools.modal.as_ref().map(|m| m.constraint),
        Some(ModalConstraint::Axis(0))
    );

    // Press X again -> toggles back to Free
    b.route_shortcut("x", false, false, false);
    assert_eq!(
        b.state.session.tools.modal.as_ref().map(|m| m.constraint),
        Some(ModalConstraint::Free)
    );

    // Press Shift+Z -> Plane(2) (orthogonal to Z, i.e. XY plane)
    b.route_shortcut("z", false, true, false);
    assert_eq!(
        b.state.session.tools.modal.as_ref().map(|m| m.constraint),
        Some(ModalConstraint::Plane(2))
    );

    // Press Shift+Z again -> Free
    b.route_shortcut("z", false, true, false);
    assert_eq!(
        b.state.session.tools.modal.as_ref().map(|m| m.constraint),
        Some(ModalConstraint::Free)
    );

    b.apply(UiIntent::CancelModalTransform);
}

#[test]
fn test_h11_click_move_click_operation_without_holding_button() {
    let mut b = bridge();
    b.apply(UiIntent::SetToolActivation(ToolActivation::Instant));
    b.apply(UiIntent::SetActiveTool("brush".into()));

    // Press G (or BeginModalTransform) -> enters Latched phase
    b.apply(UiIntent::BeginModalTransform(TransformKind::Position));
    assert_eq!(b.state.session.tools.active_tool, "move");
    assert!(b.tool_session.is_latched());

    // Move pointer without any mouse buttons pressed (phase: 1)
    b.tool_pointer(1, 120.0, 140.0, false, false);
    assert!(b.tool_session.is_latched());

    // Left click (phase: 0) commits latched session
    b.tool_pointer(0, 120.0, 140.0, false, false);

    // Verifications:
    assert!(!b.tool_session.is_latched());
    assert!(!b.instant_transform);
    assert_eq!(b.state.session.tools.active_tool, "brush");
}

#[test]
fn test_h11_switching_transform_modes_preserves_origin_tool() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("draw_profile".into()));
    b.apply(UiIntent::SetToolActivation(ToolActivation::Instant));

    // Begin Move
    b.apply(UiIntent::BeginModalTransform(TransformKind::Position));
    assert_eq!(b.state.session.tools.active_tool, "move");
    assert_eq!(
        b.state.session.tools.temporary_modal_origin_tool(),
        Some("draw_profile")
    );

    // Switch to Rotate directly
    b.apply(UiIntent::BeginModalTransform(TransformKind::Rotation));
    assert_eq!(b.state.session.tools.active_tool, "rotate");
    assert_eq!(
        b.state.session.tools.temporary_modal_origin_tool(),
        Some("draw_profile")
    );

    // Cancel restores original origin tool "draw_profile"
    b.apply(UiIntent::CancelModalTransform);
    assert_eq!(b.state.session.tools.active_tool, "draw_profile");
    assert_eq!(b.state.session.tools.temporary_modal_origin_tool(), None);
}
