use petunia_application::{ActionOutcome, Application};
#[test]
fn shared_select_action_is_reachable_and_unknown_action_is_explicit() {
    let mut app = Application::new(Default::default());
    app.set_tool("transform", None);
    assert_eq!(app.run_action("model.tool_select"), ActionOutcome::None);
    assert_eq!(app.state.active_tool, "select");
    assert_eq!(
        app.run_action("unknown.action"),
        ActionOutcome::UnknownAction
    );
    assert_eq!(app.state.active_tool, "select");
}
