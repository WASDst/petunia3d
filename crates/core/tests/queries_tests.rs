//! Testes unitários para Application API (Queries & DTOs) — Gauntlet G9 / F-010.

use petunia_core::{
    AddPrimitiveCmd, AppState, PrimitiveKind, ProjectService, SelectAllCmd, SelectMode,
    SelectionDomain,
};
use uuid::Uuid;

#[test]
fn test_query_scene_hierarchy_returns_valid_dtos_and_stable_uuids() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Sphere))
        .expect("add sphere");
    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Cylinder))
        .expect("add cylinder");

    let hierarchy = state.query_scene_hierarchy();
    assert_eq!(hierarchy.assets.len(), 3);
    assert_eq!(hierarchy.assets[0].name, "Cube");
    assert_eq!(hierarchy.assets[1].name, "Sphere");
    assert_eq!(hierarchy.assets[2].name, "Cylinder");

    // Todos os IDs devem ser válidos e únicos
    let mut ids = std::collections::HashSet::new();
    for asset in &hierarchy.assets {
        assert!(!asset.id.is_nil());
        assert!(ids.insert(asset.id), "IDs de assets devem ser únicos");
    }

    // O ativo deve coincidir com active_id
    assert_eq!(hierarchy.active_id, Some(hierarchy.assets[2].id));
    assert!(hierarchy.assets[2].is_active);
    assert!(!hierarchy.assets[0].is_active);
    assert!(hierarchy.total_verts > 0);
    assert!(hierarchy.total_faces > 0);
}

#[test]
fn test_query_selection_details_and_center() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    state.set_selection_domain(SelectionDomain::Vertex);
    state.dispatch(&SelectAllCmd).expect("SelectAll");

    let sel_dto = state.query_selection_details();
    assert!(sel_dto.active_asset_id.is_some());
    assert_eq!(sel_dto.active_asset_name.as_deref(), Some("Cube"));
    assert_eq!(sel_dto.selected_verts_count, 8);
    assert_eq!(sel_dto.selection_domain, SelectionDomain::Vertex);
    assert_eq!(sel_dto.select_mode, SelectMode::Vertex);
    assert!(sel_dto.selection_center.is_some());
}

#[test]
fn test_query_tool_status() {
    let mut state = AppState::new("en");
    state.active_tool = "extrude".to_string();
    state.set_status("ready".to_string());

    let status_dto = state.query_tool_status();
    assert_eq!(status_dto.active_tool, "extrude");
    assert!(!status_dto.is_modal_active);
    assert_eq!(status_dto.status, "ready");
}

#[test]
fn test_set_active_asset_by_id_and_delete_by_id() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Plane))
        .expect("add plane");
    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Sphere))
        .expect("add sphere");

    let cube_id = state.project.assets[0].id;
    let plane_id = state.project.assets[1].id;
    let sphere_id = state.project.assets[2].id;

    // Ativa por UUID estável
    assert!(state.set_active_asset_by_id(plane_id));
    assert_eq!(state.project.active().unwrap().id, plane_id);

    assert!(state.set_active_asset_by_id(cube_id));
    assert_eq!(state.project.active().unwrap().id, cube_id);

    // ID inexistente retorna false sem pânico
    let fake_id = Uuid::new_v4();
    assert!(!state.set_active_asset_by_id(fake_id));

    // Deleta por UUID estável
    assert!(state.delete_asset_by_id(plane_id));
    assert_eq!(state.project.assets.len(), 2);
    assert_eq!(state.project.assets[0].id, cube_id);
    assert_eq!(state.project.assets[1].id, sphere_id);

    // Localizar por ID
    assert!(state.find_asset_by_id(cube_id).is_some());
    assert!(state.find_asset_by_id(plane_id).is_none());
}

#[test]
fn test_export_selected_uuid_resilience() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Sphere))
        .expect("add sphere");
    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Capsule))
        .expect("add capsule");

    let cube_id = state.project.assets[0].id;
    let _sphere_id = state.project.assets[1].id;
    let capsule_id = state.project.assets[2].id;

    state.ui.export_selected = vec![cube_id, capsule_id];
    let indices = state.export_selected_indices();
    assert_eq!(indices, vec![0, 2]);

    // Deleta o asset do meio (Sphere)
    state.project.remove(1);

    // O índice do Capsule foi atualizado automaticamente de 2 para 1
    let indices_after = state.export_selected_indices();
    assert_eq!(indices_after, vec![0, 1]);
    assert_eq!(state.project.assets[indices_after[0]].id, cube_id);
    assert_eq!(state.project.assets[indices_after[1]].id, capsule_id);
}
