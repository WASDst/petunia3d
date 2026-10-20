//! Adapter equivalence: CLI/FFI/MCP/Lua intents share the Application spine.

use petunia_core::{
    AddPrimitiveCmd, AppState, CommandIntent, ExtrudeSelectedCmd, PrimitiveKind, ProjectService,
    SelectAllCmd,
};

fn primed() -> AppState {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.dispatch(&SelectAllCmd).unwrap();
    state
}

#[test]
fn dispatch_intent_matches_typed_command_for_extrude() {
    let mut a = primed();
    let mut b = primed();
    a.dispatch(&ExtrudeSelectedCmd {
        dist: 0.5,
        ..Default::default()
    })
    .unwrap();
    b.dispatch_intent(&CommandIntent {
        command: "model.extrude".into(),
        asset_id: None,
        args: vec![0.5],
    })
    .unwrap();
    let va = a.project.active_mesh().unwrap().vert_count();
    let vb = b.project.active_mesh().unwrap().vert_count();
    assert_eq!(va, vb);
}

#[test]
fn unknown_primitive_is_an_error() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let err = PrimitiveKind::parse("NotAThing").unwrap_err();
    assert!(matches!(
        err,
        petunia_core::CommandError::UnknownPrimitive(_)
    ));
    let before = state.project.assets.len();
    let result = state.dispatch_intent(&CommandIntent {
        command: "model.add_primitive".into(),
        asset_id: Some("NotAThing".into()),
        args: vec![],
    });
    assert!(result.is_err());
    assert_eq!(state.project.assets.len(), before);
}

#[test]
fn add_primitive_command_is_the_only_creation_path() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let before = state.project.assets.len();
    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Plane))
        .unwrap();
    assert_eq!(state.project.assets.len(), before + 1);
}

#[test]
fn history_reports_byte_metrics() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Sphere))
        .unwrap();
    let metrics = state.project.history_metrics();
    assert!(metrics.history_entries >= 1);
    assert!(metrics.history_bytes > 0);
    assert!(metrics.largest_entry > 0);
}

#[test]
fn scene_fingerprint_uses_revisions_after_mutation() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let flags = petunia_core::FingerprintFlags {
        shading: petunia_core::Shading::Solid,
        xray: false,
        show_triangulation: false,
        textured: false,
        edit_mode_is_edit: false,
        show_wireframe_overlay: false,
        show_face_orientation: false,
        show_uv_checker: false,
    };
    let a = petunia_core::fingerprint_scene(&state.project.project, &[], flags);
    state.emit_mesh_changed();
    let b = petunia_core::fingerprint_scene(&state.project.project, &[], flags);
    assert_ne!(a.mesh, b.mesh);
}

#[test]
fn stale_target_ids_fail_instead_of_hitting_the_active_object() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let active_before = state.project.active;
    let faces_before = state.project.active_mesh().unwrap().faces.len();
    for stale in [uuid::Uuid::new_v4().to_string(), "not-a-uuid".to_string()] {
        let result = state.dispatch_intent(&CommandIntent {
            command: "model.subdivide".into(),
            asset_id: Some(stale.clone()),
            args: vec![1.0],
        });
        assert!(
            matches!(result, Err(petunia_core::CommandError::StaleReference(ref id)) if *id == stale)
        );
    }
    assert_eq!(state.project.active, active_before);
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        faces_before
    );
}

#[test]
fn failed_transactions_roll_back_document_and_history() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let assets_before = state.project.assets.len();
    let depth_before = state.project.undo.depth();
    let result = state.transaction("plugin batch", |state| {
        state.dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Plane))?;
        state.dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Plane))?;
        Err::<(), _>(petunia_core::CommandError::Execution("boom".into()))
    });
    assert!(result.is_err());
    assert_eq!(state.project.assets.len(), assets_before);
    assert_eq!(state.project.undo.depth(), depth_before);
}

#[test]
fn successful_transactions_are_one_undo_step() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let assets_before = state.project.assets.len();
    let (undo_before, _) = state.project.undo.depth();
    state
        .transaction("plugin batch", |state| {
            state.dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Plane))?;
            state.dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Plane))
        })
        .unwrap();
    assert_eq!(state.project.assets.len(), assets_before + 2);
    assert_eq!(state.project.undo.depth().0, undo_before + 1);
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), assets_before);
}
