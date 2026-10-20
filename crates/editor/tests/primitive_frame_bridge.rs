use petunia_core::{AppState, PrimitiveKind};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};

fn bridge() -> SlintUiBridge<PlaceholderViewport> {
    SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default())
}

#[test]
fn primitive_intent_and_numeric_adapter_keep_frame_and_reject_non_finite_input() {
    let mut b = bridge();
    b.state.session.cursor_3d = [3.0, -2.0, 4.0];
    b.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    let a = b.state.project.active().unwrap();
    assert_eq!(a.transform.translation, [3.0, -2.0, 4.0]);
    assert_eq!(a.origin, None);
    let frame = a.transform;
    let depth = b.state.project.undo.depth();
    let before = serde_json::to_value(&b.state.project.project).unwrap();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(!b.update_primitive_param_float("width", value));
        assert_eq!(
            serde_json::to_value(&b.state.project.project).unwrap(),
            before
        );
        assert_eq!(b.state.project.undo.depth(), depth);
    }
    b.state.session.cursor_3d = [100.0; 3];
    assert!(b.update_primitive_param_float("width", 5.0));
    assert_eq!(b.state.project.active().unwrap().transform, frame);
    assert_eq!(b.state.project.undo.depth(), depth);
    assert!(b.state.confirm_primitive());
    let depth = b.state.project.undo.depth().0;
    assert!(b.update_primitive_param_float("height", 3.0));
    assert_eq!(b.state.project.undo.depth().0, depth + 1);
    assert_eq!(b.state.project.active().unwrap().transform, frame);
    assert!(b.state.undo());
    assert_eq!(b.state.project.active().unwrap().transform, frame);
}

#[test]
fn numeric_adapter_uses_active_object_when_creation_session_is_stale() {
    let mut b = bridge();
    b.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    let id = b.state.project.active().unwrap().id;
    b.state.project.active = 0;
    assert!(!b.state.primitive_session_valid());
    assert!(b.active_primitive_descriptor_or_session().is_none());
    assert!(!b.update_primitive_param_float("width", 10.0));
    assert!(
        b.state
            .project
            .assets
            .iter()
            .any(|a| a.id == id && a.parametric.is_some())
    );
}
