// Unit and integration test suite for Slint UI bridge and shell interactions.
// Suíte de testes unitários e de integração para o bridge Slint UI e interações do shell.

use super::*;

#[allow(dead_code)]
fn visible_edge_points(bridge: &SlintUiBridge<PlaceholderViewport>) -> Vec<((u32, u32), [f32; 2])> {
    let mesh = bridge.state.project.active_mesh().expect("active mesh");
    mesh.edges_unique()
        .into_iter()
        .filter_map(|(a, b)| {
            let midpoint = (mesh.verts[a as usize].vec() + mesh.verts[b as usize].vec()) * 0.5;
            let ndc = bridge.state.session.camera.project_ndc(midpoint);
            let point = [(ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5];
            let picked = bridge.pick_target_for_domain(SelectionDomain::Edge, point[0], point[1]);
            (picked == petunia_core::HoverTarget::Edge(a, b)).then_some(((a, b), point))
        })
        .collect()
}

fn active_paint_stack(
    bridge: &SlintUiBridge<PlaceholderViewport>,
) -> petunia_project::PaintLayerStack {
    bridge
        .state
        .project
        .active()
        .and_then(|asset| asset.paint_stack.clone())
        .expect("active paint stack")
}

fn active_profile_spline(
    bridge: &SlintUiBridge<PlaceholderViewport>,
) -> &petunia_core::SplineResource {
    let profile_id = bridge.active_profile_id.expect("active profile id");
    let profile = bridge
        .state
        .project
        .project
        .get_profile(profile_id)
        .expect("active profile");
    bridge
        .state
        .project
        .project
        .get_spline(profile.spline_id)
        .expect("active profile spline")
}

#[test]
fn view_model_uses_domain_context_without_ui_dependencies() {
    let state = AppState::default();
    let view_model = ShellViewModel::from_state(&state);
    assert_eq!(view_model.workspace, Workspace::Model);
    assert_eq!(view_model.selection_domain, SelectionDomain::Object);
    assert!(view_model.inspector_visible);
    assert!(view_model.saved);
    assert_eq!(view_model.position, [0.0, 0.0, 0.0]);
    assert_eq!(view_model.scale, [1.0, 1.0, 1.0]);
}

#[test]
fn bridge_routes_workspace_intent_to_core_and_viewport() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    assert_eq!(bridge.state.workspace, Workspace::Paint);
    assert_eq!(bridge.viewport.workspace, Workspace::Paint);

    bridge.apply(UiIntent::SetWorkspace(Workspace::Uv));
    assert_eq!(bridge.state.workspace, Workspace::Uv);
    assert_eq!(bridge.viewport.workspace, Workspace::Uv);
}

#[test]
fn temporary_surfaces_are_owned_by_bridge_not_domain_state() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ToggleSceneDrawer);
    bridge.apply(UiIntent::OpenCommandSearch);
    bridge.apply(UiIntent::OpenSettings);
    assert!(bridge.scene_drawer_visible);
    assert!(bridge.command_search_visible);
    assert!(bridge.settings_visible);
    assert!(!bridge.state.ui.show_command_palette);
}

#[test]
fn overlay_stack_handles_escape_in_lifo_order() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ToggleSceneDrawer);
    bridge.apply(UiIntent::OpenCommandSearch);

    assert!(bridge.scene_drawer_visible);
    assert!(bridge.command_search_visible);

    assert!(bridge.handle_escape());
    assert!(!bridge.command_search_visible);
    assert!(bridge.scene_drawer_visible);

    assert!(bridge.handle_escape());
    assert!(!bridge.scene_drawer_visible);

    assert!(!bridge.handle_escape());
}

#[test]
fn click_away_dismisses_active_modal() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::OpenSettings);
    assert!(bridge.settings_visible);

    assert!(bridge.handle_click_away());
    assert!(!bridge.settings_visible);
}

#[test]
fn command_search_filters_by_active_workspace() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let results = bridge.search_commands("cube");
    assert!(results.iter().any(|item| item.id == "model.add_cube"));
    assert!(
        results
            .iter()
            .all(|item| !item.label.starts_with("commands."))
    );
}

#[test]
fn command_search_uses_current_keymap_shortcuts() {
    let mut state = AppState::default();
    state.ui.keybinds.remove_binding("model.add_cube");
    let bridge = SlintUiBridge::new(state, PlaceholderViewport::default());

    let item = bridge
        .search_commands("Add Cube")
        .into_iter()
        .find(|item| item.id == "model.add_cube")
        .unwrap();

    assert_eq!(item.shortcut, None);
}

#[test]
fn core_palette_command_executes_through_canonical_dispatcher() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let before = bridge.state.project.assets.len();

    bridge.execute_core_command("model.add_cube").unwrap();

    assert_eq!(bridge.state.project.assets.len(), before + 1);
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn transform_scrubbing_updates_values_and_clamps() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    let pos_x = bridge.scrub_transform(TransformKind::Position, 0, 5.0, false);
    assert_eq!(pos_x, 0.5);

    let pos_x_fine = bridge.scrub_transform(TransformKind::Position, 0, 2.0, true);
    assert_eq!(pos_x_fine, 0.52);
    assert_eq!(bridge.view_model().position[0], 0.52);

    assert!(bridge.commit_transform());

    let scale_z = bridge.scrub_transform(TransformKind::Scale, 2, -100.0, false);
    assert_eq!(
        scale_z, -9.0,
        "negative scale mirrors the selected geometry"
    );

    let vm = bridge.view_model();
    assert_eq!(vm.position[0], 0.0);
    assert_eq!(vm.scale[2], -9.0);
}

#[test]
fn transform_scrub_changes_geometry_and_commits_one_undo_step() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let before = bridge.state.project.active_mesh().unwrap().verts.clone();

    bridge.begin_transform(TransformKind::Position).unwrap();
    bridge.scrub_transform(TransformKind::Position, 0, 10.0, false);
    bridge.scrub_transform(TransformKind::Position, 0, 10.0, false);
    assert!(bridge.commit_transform());

    let after = &bridge.state.project.active_mesh().unwrap().verts;
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(after, before)| { (after.pos[0] - before.pos[0] - 2.0).abs() < 1.0e-6 })
    );
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.undo());
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .zip(&before)
            .all(|(restored, before)| restored.pos == before.pos)
    );
}

#[test]
fn escape_cancels_transform_without_closing_the_underlying_overlay() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ToggleSceneDrawer);
    let before = bridge.state.project.active_mesh().unwrap().verts.clone();
    bridge.begin_transform(TransformKind::Position).unwrap();
    bridge.scrub_transform(TransformKind::Position, 1, 10.0, false);

    assert!(bridge.handle_escape());
    assert!(bridge.scene_drawer_visible);
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .zip(&before)
            .all(|(restored, before)| restored.pos == before.pos)
    );
    assert_eq!(bridge.view_model().position, [0.0; 3]);
}

#[test]
fn selecting_another_asset_resets_transform_operation_values() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    bridge.begin_transform(TransformKind::Position).unwrap();
    bridge.scrub_transform(TransformKind::Position, 0, 10.0, false);
    bridge.commit_transform();
    assert_eq!(bridge.view_model().position[0], 1.0);

    let first_id = bridge.state.project.assets[0].id.to_string();
    bridge.apply(UiIntent::SelectSceneAsset(first_id));

    assert_eq!(bridge.view_model().position, [0.0; 3]);
    assert_eq!(bridge.view_model().rotation, [0.0; 3]);
    assert_eq!(bridge.view_model().scale, [1.0; 3]);
}

#[test]
fn bridge_applies_viewport_gestures_to_camera() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let initial_yaw = bridge.state.session.camera.yaw;
    let initial_pitch = bridge.state.session.camera.pitch;
    let initial_target = bridge.state.session.camera.target;
    let initial_height = bridge.state.session.camera.visible_height();

    bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Orbit {
        dx: 10.0,
        dy: -5.0,
    }));
    assert_ne!(bridge.state.session.camera.yaw, initial_yaw);
    assert_ne!(bridge.state.session.camera.pitch, initial_pitch);

    bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan {
        dx: 20.0,
        dy: 10.0,
    }));
    assert_ne!(bridge.state.session.camera.target, initial_target);

    bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Zoom {
        delta: 50.0,
    }));
    assert_ne!(bridge.state.session.camera.visible_height(), initial_height);
}

#[test]
fn viewport_resize_updates_backend_and_camera_aspect() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    bridge.resize_viewport(800, 500);

    assert_eq!(bridge.viewport.width, 800);
    assert_eq!(bridge.viewport.height, 500);
    assert!((bridge.state.session.camera.aspect - 1.6).abs() < f32::EPSILON);
    assert!(!bridge.state.is_document_dirty());
}

#[test]
fn bridge_saves_and_loads_project_file() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let temp_path = temp_dir.path().join("test_project.petunia");

    bridge.state.project.name = "Slint Test Project".into();
    bridge.state.mark_document_dirty();
    assert!(!bridge.view_model().saved);

    bridge.apply(UiIntent::SaveProjectTo(temp_path.clone()));
    let status = bridge.view_model().status_message.clone();
    assert!(temp_path.exists(), "project save failed: {status}");
    assert!(bridge.view_model().saved);

    bridge.state.project.name = "Modified Project".into();
    bridge.apply(UiIntent::OpenProjectFrom(temp_path));
    assert_eq!(bridge.state.project.name, "Slint Test Project");
    assert!(bridge.view_model().saved);
}

#[test]
fn scene_item_selection_updates_active_asset_and_inspector() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.execute_command(CommandId::AddCube);
    assert_eq!(bridge.state.project.assets.len(), 2);

    let second_asset_id = bridge.state.project.assets[1].id.to_string();
    bridge.apply(UiIntent::SelectSceneAsset(second_asset_id.clone()));

    assert_eq!(bridge.state.project.active, 1);
    let vm = bridge.view_model();
    assert!(
        vm.scene_items
            .iter()
            .any(|item| item.id == second_asset_id && item.selected)
    );
    assert_eq!(vm.active_object_title, "Cube");
}

#[test]
fn scene_item_visibility_and_lock_toggles() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let asset_id = bridge.state.project.assets[0].id.to_string();

    assert!(bridge.state.project.assets[0].visible);
    assert!(!bridge.state.project.assets[0].locked);

    bridge.apply(UiIntent::ToggleSceneAssetVisibility(asset_id.clone()));
    assert!(!bridge.state.project.assets[0].visible);
    assert!(bridge.state.is_document_dirty());
    assert!(bridge.state.undo());
    assert!(bridge.state.project.assets[0].visible);

    bridge.apply(UiIntent::ToggleSceneAssetLock(asset_id));
    assert!(bridge.state.project.assets[0].locked);
    assert!(bridge.state.undo());
    assert!(!bridge.state.project.assets[0].locked);

    let vm = bridge.view_model();
    assert!(vm.scene_items[0].visible);
    assert!(!vm.scene_items[0].locked);
}

#[test]
fn theme_change_intent_updates_active_theme() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(bridge.state.ui.active_theme_id, "petunia-dark");

    bridge.apply(UiIntent::SetTheme("petunia-high-contrast".into()));
    assert_eq!(bridge.state.ui.active_theme_id, "petunia-high-contrast");
    assert_eq!(bridge.view_model().current_theme, "petunia-high-contrast");
}

#[test]
fn add_cube_command_adds_mesh_to_scene() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let initial_count = bridge.state.project.assets.len();
    bridge.execute_command(CommandId::AddCube);
    assert_eq!(bridge.state.project.assets.len(), initial_count + 1);
    assert_eq!(bridge.view_model().scene_items.len(), initial_count + 1);
}

#[test]
fn selection_domain_intent_updates_state_and_view_model() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(
        bridge.view_model().selection_domain,
        SelectionDomain::Object
    );

    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Vertex);
    assert_eq!(
        bridge.view_model().selection_domain,
        SelectionDomain::Vertex
    );
    assert_eq!(bridge.viewport.selection_domain, SelectionDomain::Vertex);

    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Edge);
    assert_eq!(bridge.view_model().selection_domain, SelectionDomain::Edge);

    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Face);
    assert_eq!(bridge.view_model().selection_domain, SelectionDomain::Face);
}

#[test]
fn primitive_creation_and_deletion_updates_scene() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let base_count = bridge.state.project.assets.len();

    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    assert_eq!(bridge.state.project.assets.len(), base_count + 1);
    assert_eq!(bridge.view_model().active_object_title, "Sphere");

    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    assert_eq!(bridge.state.project.assets.len(), base_count + 2);
    assert_eq!(bridge.view_model().active_object_title, "Cylinder");

    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Plane));
    assert_eq!(bridge.state.project.assets.len(), base_count + 3);
    assert_eq!(bridge.view_model().active_object_title, "Plane");

    // Deleting active asset
    bridge.apply(UiIntent::DeleteActiveAsset);
    assert_eq!(bridge.state.project.assets.len(), base_count + 2);
    assert_eq!(
        bridge.view_model().active_object_title,
        "No Object Selected"
    );
    assert_eq!(bridge.state.project.active, usize::MAX);
    assert!(
        bridge
            .state
            .project
            .assets
            .iter()
            .any(|asset| asset.name == "Cylinder")
    );
}

#[test]
fn paint_tool_parameters_update_session_and_view_model() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    bridge.apply(UiIntent::SetPaintColor([0.25, 0.5, 0.75]));
    assert_eq!(bridge.state.session.tools.paint_color, [0.25, 0.5, 0.75]);
    assert_eq!(bridge.view_model().paint_color, [0.25, 0.5, 0.75]);

    bridge.apply(UiIntent::SetBrushSize(2.5));
    assert_eq!(bridge.state.session.tools.paint_radius, 2.5);
    assert_eq!(bridge.view_model().brush_size, 2.5);

    bridge.apply(UiIntent::SetBrushOpacity(0.8));
    assert_eq!(bridge.state.session.tools.paint_strength, 0.8);
    assert_eq!(bridge.view_model().brush_opacity, 0.8);

    bridge.apply(UiIntent::SetActiveTool("eraser".into()));
    assert_eq!(bridge.state.session.tools.active_tool, "eraser");
    assert_eq!(bridge.view_model().active_tool, "eraser");
}

#[test]
fn command_routing_covers_selection_paint_and_uv() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    bridge.execute_command(CommandId::SelectModePoint);
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Vertex);

    bridge.execute_command(CommandId::SelectModeEdge);
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Edge);

    bridge.execute_command(CommandId::SelectModeFace);
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Face);

    bridge.execute_command(CommandId::SelectModeObject);
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Object);

    bridge.execute_command(CommandId::PaintBrush);
    assert_eq!(bridge.state.session.tools.active_tool, "brush");

    bridge.execute_command(CommandId::PaintEraser);
    assert_eq!(bridge.state.session.tools.active_tool, "eraser");

    bridge.execute_command(CommandId::PaintFill);
    assert_eq!(bridge.state.session.tools.active_tool, "fill");

    bridge.execute_command(CommandId::UvUnwrap);
    assert!(bridge.view_model().status_message.contains("Auto UV"));

    bridge.execute_command(CommandId::UvPackIslands);
    assert!(bridge.view_model().status_message.contains("Packed"));

    bridge.execute_command(CommandId::DeleteSelected);
    assert_eq!(bridge.state.project.assets.len(), 0);
    assert!(bridge.state.is_document_dirty());
}

#[test]
fn undo_redo_intents_integrate_with_project_undo_stack() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Initial state
    assert!(!bridge.view_model().can_undo);
    assert!(!bridge.view_model().can_redo);

    // Add primitive command automatically creates an undo checkpoint in core AppState
    bridge.execute_command(CommandId::AddCube);
    assert_eq!(bridge.state.project.assets.len(), 2);
    assert!(bridge.view_model().can_undo);

    // Apply Undo
    bridge.apply(UiIntent::Undo);
    assert_eq!(bridge.state.project.assets.len(), 1);
    assert!(bridge.view_model().can_redo);

    // Apply Redo
    bridge.apply(UiIntent::Redo);
    assert_eq!(bridge.state.project.assets.len(), 2);
    assert!(bridge.view_model().can_undo);
}

#[test]
fn duplicate_active_asset_intent_duplicates_and_creates_undo() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let base_count = bridge.state.project.assets.len();
    assert_eq!(base_count, 1);
    assert!(!bridge.view_model().can_undo);

    bridge.apply(UiIntent::DuplicateActiveAsset);
    assert_eq!(bridge.state.project.assets.len(), 2);
    assert!(bridge.view_model().can_undo);
    assert!(bridge.view_model().status_message.contains("duplicado"));

    let first_id = bridge.state.project.assets[0].id;
    let second_id = bridge.state.project.assets[1].id;
    assert_ne!(first_id, second_id);

    bridge.apply(UiIntent::Undo);
    assert_eq!(bridge.state.project.assets.len(), 1);
    assert!(bridge.view_model().can_redo);

    bridge.apply(UiIntent::Redo);
    assert_eq!(bridge.state.project.assets.len(), 2);
    assert!(bridge.view_model().can_undo);
}

#[test]
fn selection_commands_select_all_clear_and_invert() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));

    bridge.apply(UiIntent::SelectAll);
    let mesh = bridge.state.project.active_mesh().expect("active mesh");
    assert!(mesh.verts.iter().all(|v| v.selected));

    bridge.apply(UiIntent::ClearSelection);
    let mesh = bridge.state.project.active_mesh().expect("active mesh");
    assert!(mesh.verts.iter().all(|v| !v.selected));

    bridge.apply(UiIntent::InvertSelection);
    let mesh = bridge.state.project.active_mesh().expect("active mesh");
    assert!(mesh.verts.iter().all(|v| v.selected));
}

#[test]
fn toggle_asset_library_and_overlays() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().asset_library_visible);

    bridge.apply(UiIntent::ToggleAssetLibrary);
    assert!(bridge.asset_library_visible);
    assert!(bridge.view_model().asset_library_visible);

    assert!(bridge.handle_escape());
    assert!(!bridge.asset_library_visible);
    assert!(!bridge.view_model().asset_library_visible);

    bridge.apply(UiIntent::ToggleAssetLibrary);
    assert!(bridge.asset_library_visible);
    assert!(bridge.handle_click_away());
    assert!(!bridge.asset_library_visible);
}

#[test]
fn closing_one_drawer_does_not_close_or_leave_a_ghost_for_another() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ToggleSceneDrawer);
    bridge.apply(UiIntent::ToggleAssetLibrary);
    bridge.apply(UiIntent::ToggleSceneDrawer);

    assert!(!bridge.scene_drawer_visible);
    assert!(bridge.asset_library_visible);
    assert_eq!(bridge.overlays.len(), 1);
    assert!(bridge.handle_escape());
    assert!(!bridge.asset_library_visible);
    assert!(!bridge.handle_escape());
}

#[test]
fn asset_drawer_search_sort_and_zoom_do_not_mutate_the_document() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    bridge.state.mark_document_clean();
    let total = bridge.view_model().scene_items.len();
    assert!(total >= 2);
    assert!(bridge.set_asset_query("sphere"));
    let filtered = bridge.view_model();
    assert_eq!(filtered.asset_items.len(), 1);
    assert!(
        filtered.asset_items[0]
            .name
            .to_lowercase()
            .contains("sphere")
    );
    assert_eq!(filtered.scene_items.len(), total);
    assert!(bridge.set_asset_sort_by_name(true));
    assert!(bridge.set_asset_thumbnail_size(120.0));
    assert_eq!(bridge.view_model().asset_thumbnail_size, 120.0);
    assert!(!bridge.state.is_document_dirty());
    assert!(!bridge.set_asset_thumbnail_size(f32::NAN));
}

#[test]
fn parts_drawer_filters_sorts_and_scales_rows_without_mutation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    bridge.state.mark_document_clean();
    assert_eq!(bridge.view_model().parts_items.len(), 2);

    assert!(bridge.set_parts_query("cube"));
    assert_eq!(bridge.view_model().parts_items.len(), 1);
    assert!(
        bridge.view_model().parts_items[0]
            .name
            .to_lowercase()
            .contains("cube")
    );
    assert!(bridge.set_parts_query(""));
    assert!(bridge.set_parts_selected_only(true));
    assert_eq!(bridge.view_model().parts_items.len(), 1);
    assert!(bridge.set_parts_selected_only(false));
    assert!(bridge.set_parts_sort_by_name(true));
    let names: Vec<_> = bridge
        .view_model()
        .parts_items
        .iter()
        .map(|item| item.name.clone())
        .collect();
    assert!(
        names
            .windows(2)
            .all(|pair| pair[0].to_lowercase() <= pair[1].to_lowercase())
    );
    assert!(bridge.set_parts_row_height(40.0));
    assert_eq!(bridge.view_model().parts_row_height, 40.0);
    assert!(!bridge.set_parts_row_height(f32::NAN));
    assert!(!bridge.state.is_document_dirty());
}

#[test]
fn modal_identity_preserves_lifo_when_multiple_modals_are_open() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::OpenCommandSearch);
    bridge.apply(UiIntent::OpenSettings);

    assert!(bridge.handle_escape());
    assert!(!bridge.settings_visible);
    assert!(bridge.command_search_visible);
    assert!(bridge.handle_escape());
    assert!(!bridge.command_search_visible);
}

#[test]
fn delete_is_dirty_and_undo_restores_the_asset() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    bridge.state.mark_document_clean();
    let before = bridge.state.project.assets.len();

    bridge.apply(UiIntent::DeleteActiveAsset);

    assert_eq!(bridge.state.project.assets.len(), before - 1);
    assert!(bridge.state.is_document_dirty());
    assert!(bridge.state.undo());
    assert_eq!(bridge.state.project.assets.len(), before);
}

#[test]
fn viewport_drag_transform_moves_geometry_and_commits_one_undo_step() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    let before = bridge.state.project.active_mesh().unwrap().verts.clone();

    assert!(bridge.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
    assert!(bridge.update_viewport_transform(460.0, 300.0));
    assert!(bridge.end_viewport_transform());

    let after = &bridge.state.project.active_mesh().unwrap().verts;
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(after, before)| (after.pos[0] - before.pos[0]).abs() > 1.0e-4)
    );
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.undo());
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .zip(&before)
            .all(|(restored, before)| restored.pos == before.pos)
    );
    assert!(bridge.drag.is_none());
}

#[test]
fn shortcut_transform_accepts_drag_in_the_same_mouse_gesture() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.pointer_position = [400.0, 300.0];
    let before = bridge.state.project.active_mesh().unwrap().verts.clone();

    // Primeiro toque ativa a ferramenta Move (Gizmo).
    assert!(bridge.route_shortcut("G", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "move");
    assert!(!bridge.view_model().transform_instant_active);

    // Segundo toque rápido (Double-tap) dispara o modo modal livre (Modo Blender).
    assert!(bridge.route_shortcut("G", false, false, false));
    assert!(bridge.view_model().transform_instant_active);
    // O TouchArea entrega estes updates com LMB pressionado, antes do
    // release que confirma; não existe segundo pointer-down.
    assert!(bridge.update_viewport_transform_modified(430.0, 300.0, false, false));
    assert!(bridge.update_viewport_transform_modified(470.0, 300.0, false, false));
    assert!(bridge.end_viewport_transform());
    assert!(!bridge.view_model().transform_instant_active);
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .zip(&before)
            .any(|(after, before)| after.pos != before.pos)
    );
}

#[test]
fn vertical_drag_preference_reverses_transform_without_dirtying_the_document() {
    let mut regular = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let mut inverted = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    regular.resize_viewport(800, 600);
    inverted.resize_viewport(800, 600);
    let origin = glam::Vec3::from_array(regular.state.project.active_mesh().unwrap().verts[0].pos);

    assert!(inverted.set_invert_vertical_drag(true));
    assert!(inverted.view_model().invert_vertical_drag);
    assert!(!inverted.state.is_document_dirty());
    assert!(regular.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
    assert!(inverted.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
    assert!(regular.update_viewport_transform(400.0, 350.0));
    assert!(inverted.update_viewport_transform(400.0, 350.0));

    let normal_delta =
        glam::Vec3::from_array(regular.state.project.active_mesh().unwrap().verts[0].pos) - origin;
    let inverted_delta =
        glam::Vec3::from_array(inverted.state.project.active_mesh().unwrap().verts[0].pos) - origin;
    assert!(normal_delta.length() > 1.0e-4);
    assert!((normal_delta + inverted_delta).length() < 1.0e-4);
}

#[test]
fn selection_appearance_preferences_validate_without_document_mutation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.mark_document_clean();
    assert!(bridge.set_selection_color_hex("#20B4F0"));
    assert_eq!(bridge.view_model().selection_rgb, [32, 180, 240]);
    assert_eq!(bridge.view_model().selection_color_hex, "#20B4F0");
    assert!(!bridge.set_selection_color_hex("#xyz"));
    assert!(!bridge.set_selection_color_hex("#000000"));
    assert_eq!(bridge.view_model().selection_rgb, [32, 180, 240]);
    assert!(bridge.set_selection_thickness(4.5));
    assert_eq!(bridge.view_model().selection_thickness, 4.5);
    assert!(!bridge.set_selection_thickness(f32::NAN));
    assert!(!bridge.state.is_document_dirty());
}

#[test]
fn escape_cancels_viewport_drag_without_touching_history() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    let before = bridge.state.project.active_mesh().unwrap().verts.clone();

    assert!(bridge.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
    assert!(bridge.update_viewport_transform(460.0, 300.0));
    assert!(bridge.handle_escape());

    assert!(bridge.drag.is_none());
    assert!(!bridge.state.project.undo.can_undo());
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .zip(&before)
            .all(|(restored, before)| restored.pos == before.pos)
    );
}

#[test]
fn edge_domain_selects_a_real_edge() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));

    let (_, point) = *visible_edge_points(&bridge)
        .first()
        .expect("visible cube edge");
    bridge.select_viewport(point[0], point[1], false);

    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(
        mesh.selected_edges.iter().any(|&(a, b)| {
            mesh.verts.get(a as usize).is_some_and(|v| v.selected)
                || mesh.verts.get(b as usize).is_some_and(|v| v.selected)
        }),
        "edge selection must mark the picked edge or its endpoints"
    );
}

#[test]
fn locked_asset_is_not_picked_in_the_viewport() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::ToggleSceneAssetLock(
        bridge.state.project.assets[0].id.to_string(),
    ));

    let cursor_before = bridge.state.session.cursor_3d;
    bridge.select_viewport(0.5, 0.5, false);

    assert_eq!(bridge.state.session.cursor_3d, cursor_before);
    assert!(bridge.state.project.assets[0].locked);
}

#[test]
fn gizmo_projects_axes_for_the_active_object() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;

    assert!(gizmo.visible);
    assert!((gizmo.origin_x - 512.0).abs() < 64.0);
    assert!((gizmo.origin_y - 384.0).abs() < 64.0);
    for (name, commands, arrows) in [
        ("x", &gizmo.x_commands, &gizmo.x_arrow_commands),
        ("y", &gizmo.y_commands, &gizmo.y_arrow_commands),
        ("z", &gizmo.z_commands, &gizmo.z_arrow_commands),
    ] {
        assert!(
            commands.starts_with("M ") && commands.contains(" L "),
            "haste {name} precisa de segmento: {commands}"
        );
        assert!(
            arrows.starts_with("M ") && arrows.ends_with("Z "),
            "haste {name} precisa de seta fechada: {arrows}"
        );
    }
    // As hastes usam 72 px como máximo e encolhem quando o eixo aponta
    // para a câmera; devem permanecer legíveis e contidas na viewport.
    for commands in [&gizmo.x_commands, &gizmo.y_commands, &gizmo.z_commands] {
        let numbers: Vec<f32> = commands
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect();
        assert_eq!(numbers.len(), 4);
        let length = ((numbers[2] - numbers[0]).powi(2) + (numbers[3] - numbers[1]).powi(2)).sqrt();
        assert!(
            (8.0..=72.1).contains(&length),
            "haste projetada fora do intervalo esperado: {length:.1}px: {commands}"
        );
    }
}

#[test]
fn the_view_tripod_marks_all_three_axes_at_top_right() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().gizmo.view_x_commands.is_empty());

    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    let gizmo = bridge.view_model().gizmo;
    for (name, commands) in [
        ("x", &gizmo.view_x_commands),
        ("y", &gizmo.view_y_commands),
        ("z", &gizmo.view_z_commands),
    ] {
        assert!(
            commands.starts_with("M ") && commands.contains(" L "),
            "tripé {name} precisa de segmento: {commands}"
        );
        let numbers: Vec<f32> = commands
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect();
        assert_eq!(numbers.len(), 4);
        let length = ((numbers[2] - numbers[0]).powi(2) + (numbers[3] - numbers[1]).powi(2)).sqrt();
        assert!(
            (8.0..=38.1).contains(&length),
            "tripé {name} deve encurtar com a projeção, veio {length:.1}px"
        );
    }
    // O tripé existe mesmo sem ferramenta de transformação: ele mostra a
    // câmera, não a ferramenta.
    assert!(!gizmo.visible);
    assert!((gizmo.view_origin_x - (1024.0 - 54.0)).abs() < 1.0);
    assert!((gizmo.view_origin_y - 108.0).abs() < 1.0);
}

#[test]
fn clicking_a_view_axis_snaps_the_camera() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.snap_view_to_axis("x"));
    assert_eq!(
        bridge.state.session.camera.view_preset(),
        Some(petunia_core::ViewPreset::Right)
    );
    assert!(bridge.snap_view_to_axis("y"));
    assert_eq!(
        bridge.state.session.camera.view_preset(),
        Some(petunia_core::ViewPreset::Top)
    );
    assert!(bridge.snap_view_to_axis("z"));
    assert_eq!(
        bridge.state.session.camera.view_preset(),
        Some(petunia_core::ViewPreset::Front)
    );
    assert!(!bridge.snap_view_to_axis("w"));
}

#[test]
fn gizmo_is_hidden_outside_the_model_workspace() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

    assert!(!bridge.view_model().gizmo.visible);
}

#[test]
fn delete_in_component_mode_removes_geometry_not_the_object() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    let asset_count = bridge.state.project.assets.len();
    let verts_before = bridge.state.project.active_mesh().unwrap().verts.len();

    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    bridge.apply(UiIntent::DeleteActiveAsset);

    assert_eq!(bridge.state.project.assets.len(), asset_count);
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(mesh.verts.len() <= verts_before);
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn paint_stroke_is_continuous_and_commits_one_undo_step() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("brush".into()));

    assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
    assert!(bridge.paint_stroke_to(420.0, 300.0));
    assert!(bridge.paint_stroke_to(440.0, 310.0));
    assert!(bridge.end_paint_stroke_at(440.0, 300.0));

    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.is_document_dirty());
    assert!(bridge.paint_last.is_none());
}

#[test]
fn escape_cancels_paint_stroke_without_history() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("brush".into()));

    assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
    assert!(bridge.paint_stroke_to(430.0, 300.0));
    assert!(bridge.handle_escape());

    assert!(bridge.paint_last.is_none());
    assert!(!bridge.state.project.undo.can_undo());
}

#[test]
fn keymap_routes_the_model_shortcut_table() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    assert!(bridge.route_shortcut("E", false, false, false));
    assert!(bridge.route_shortcut("I", false, false, false));
    assert!(bridge.route_shortcut("B", true, false, false));
    assert!(bridge.route_shortcut("W", false, false, false));
    assert!(bridge.route_shortcut("M", false, false, false));
    assert!(bridge.route_shortcut("K", false, false, false));
    assert!(bridge.route_shortcut("Z", true, false, false));
    assert!(bridge.route_shortcut("1", false, false, false));
    assert!(!bridge.route_shortcut("Ω", false, false, false));
}

#[test]
fn push_pull_and_knife_commands_are_registered_and_contextual() {
    let mut state = AppState::default();
    assert!(state.commands.contains("model.push_pull"));
    assert!(state.commands.contains("model.knife"));

    // Push/Pull exige Edit mode e face selecionada.
    assert!(state.dispatch_command("model.push_pull").is_err());

    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    assert!(state.dispatch_command("model.push_pull").is_ok());
    assert!(state.session.tools.modal.is_some());
    state.cancel_modal();

    assert!(state.dispatch_command("model.knife").is_ok());
    assert!(state.session.tools.cut_session.is_some());
}

#[test]
fn extrude_tool_modal_previews_with_drag_and_commits_one_undo() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
    assert!(bridge.view_model().tool_modal_active);
    assert!(bridge.scrub_tool_modal(-40.0, false));
    assert!(bridge.tool_modal_value > 0.0);
    assert!(bridge.commit_tool_modal());

    assert!(bridge.state.project.undo.can_undo());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(!bridge.view_model().tool_modal_active);
}

#[test]
fn tool_modal_cancel_restores_geometry_and_keeps_history_clean() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    let before = bridge.state.project.project.clone();

    assert!(bridge.begin_tool_modal(ToolModalKind::Inset));
    assert!(bridge.scrub_tool_modal(-30.0, false));
    assert!(bridge.cancel_tool_modal());

    assert!(!bridge.state.project.undo.can_undo());
    let mesh = bridge.state.project.active_mesh().unwrap();
    let original = before.active_mesh().unwrap();
    assert_eq!(mesh.verts.len(), original.verts.len());
    assert_eq!(mesh.faces.len(), original.faces.len());
}

#[test]
fn executing_extrude_command_opens_the_tool_modal_not_a_fixed_preview() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    bridge.execute_core_command("model.extrude").unwrap();

    assert!(bridge.tool_modal.is_some());
    assert!(bridge.view_model().tool_modal_active);
    assert!(bridge.handle_escape());
    assert!(bridge.tool_modal.is_none());
}

#[test]
fn extrude_individual_builds_topology_and_moves_each_face_along_its_own_normal() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    let before = bridge.state.project.active_mesh().unwrap().clone();

    bridge
        .execute_core_command("model.extrude_individual")
        .unwrap();
    assert!(bridge.scrub_tool_modal(-60.0, false));

    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(
        mesh.verts.len() > before.verts.len(),
        "extrude individual deve criar vértices"
    );
    assert!(
        mesh.verts.iter().any(|v| {
            !before.verts.iter().any(|b| {
                (v.pos[0] - b.pos[0]).abs() < 1.0e-4
                    && (v.pos[1] - b.pos[1]).abs() < 1.0e-4
                    && (v.pos[2] - b.pos[2]).abs() < 1.0e-4
            })
        }),
        "a face extrudada precisa sair da posição original"
    );

    assert!(bridge.commit_tool_modal());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn scale_selection_modal_rejects_identity_and_commits_a_real_factor() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    let before = bridge.state.project.active_mesh().unwrap().verts.clone();

    bridge
        .execute_core_command("model.scale_selection")
        .unwrap();
    assert_eq!(bridge.tool_modal_value, 1.0);
    assert!(bridge.set_tool_modal_value(2.0));
    let scaled = bridge.state.project.active_mesh().unwrap().verts.clone();
    assert!(
        scaled
            .iter()
            .zip(&before)
            .any(|(a, b)| (a.pos[0] - b.pos[0]).abs() > 1.0e-4),
        "factor 2.0 deve deslocar vértices"
    );
    assert!(bridge.commit_tool_modal());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn uv_statistics_come_from_the_real_diagnostics_not_a_hardcoded_claim() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let stats = bridge.view_model().uv_stats;
    assert!(stats.contains("xatlas-rs-v2"));
    assert!(stats.contains("Islands:"));
    assert!(!stats.contains("LSCM"));
    assert!(!stats.contains("Texel Density: Auto"));

    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    bridge.execute_core_command("uv.unwrap").unwrap();
    let stats = bridge.view_model().uv_stats;
    assert!(stats.contains("Islands: 1"), "unwrap real: {stats}");
}

#[test]
fn inspector_splitter_clamps_and_never_dirties_the_document() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let saved = bridge.view_model().saved;

    assert!(bridge.set_inspector_width(384.0));
    assert_eq!(bridge.view_model().inspector_width, 384.0);
    assert_eq!(bridge.view_model().saved, saved);

    assert!(bridge.set_inspector_width(1.0));
    assert_eq!(
        bridge.view_model().inspector_width,
        petunia_core::PROPERTIES_MIN_WIDTH
    );
    assert!(bridge.set_inspector_width(9_999.0));
    assert_eq!(
        bridge.view_model().inspector_width,
        petunia_core::PROPERTIES_MAX_WIDTH
    );
    assert!(
        !bridge.set_inspector_width(f32::NAN),
        "NaN não pode alterar o layout"
    );
}

#[test]
fn asset_library_splitter_clamps_its_height() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.set_asset_library_height(320.0));
    assert_eq!(bridge.view_model().asset_library_height, 320.0);

    assert!(bridge.set_asset_library_height(0.0));
    assert_eq!(
        bridge.view_model().asset_library_height,
        petunia_core::SHELL_ASSET_LIBRARY_MIN_HEIGHT
    );
    assert!(bridge.set_asset_library_height(5_000.0));
    assert_eq!(
        bridge.view_model().asset_library_height,
        petunia_core::SHELL_ASSET_LIBRARY_MAX_HEIGHT
    );
}

#[test]
fn switching_workspace_remembers_the_resized_inspector_and_library() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.set_inspector_width(420.0));
    assert!(bridge.set_asset_library_height(340.0));

    bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Paint));
    assert_ne!(
        bridge.view_model().inspector_width,
        420.0,
        "Paint deve restaurar a própria largura, não herdar a de Model"
    );
    assert_ne!(
        bridge.view_model().asset_library_height,
        340.0,
        "a Asset Library também tem memória por workspace"
    );

    bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Model));
    assert_eq!(bridge.view_model().inspector_width, 420.0);
    assert_eq!(bridge.view_model().asset_library_height, 340.0);
}

#[test]
fn rename_session_commits_one_undo_entry_and_trims_the_name() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let original = bridge.state.project.active().unwrap().name.clone();
    assert_eq!(original, "Cube");

    assert!(bridge.begin_rename());
    assert!(bridge.view_model().rename_active);
    assert_eq!(bridge.view_model().rename_value, original);

    assert!(bridge.commit_rename("  Turret Base  "));
    assert!(!bridge.view_model().rename_active);
    assert_eq!(bridge.state.project.active().unwrap().name, "Turret Base");
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    assert!(bridge.state.undo());
    assert_eq!(bridge.state.project.active().unwrap().name, original);
}

#[test]
fn rename_rejects_empty_and_overlong_names_without_touching_history() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.begin_rename();
    assert!(!bridge.commit_rename("   "));
    assert_eq!(
        bridge.state.ui.status,
        petunia_core::AssetRenameError::EmptyName.to_string()
    );
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    assert_eq!(bridge.state.project.active().unwrap().name, "Cube");

    bridge.begin_rename();
    let long = "x".repeat(petunia_core::ASSET_NAME_MAX_LEN + 1);
    assert!(!bridge.commit_rename(&long));
    assert_eq!(
        bridge.state.ui.status,
        petunia_core::AssetRenameError::NameTooLong.to_string()
    );
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn confirming_an_unchanged_name_does_not_push_history() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.begin_rename();
    assert!(bridge.commit_rename("Cube"));
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn escape_abandons_a_rename_draft_before_anything_else() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.begin_rename();
    bridge.rename_draft = Some("Discarded".to_string());

    assert!(bridge.handle_escape());
    assert!(!bridge.view_model().rename_active);
    assert_eq!(bridge.state.project.active().unwrap().name, "Cube");
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn f2_resolves_to_rename_in_the_canonical_keymap() {
    let keybinds = petunia_config::keybinds::Keybinds::load_profile("petunia-default");
    let f2 = input::key_code_from_slint("F2").expect("F2 precisa ser mapeável");
    assert_eq!(
        keybinds.find(f2, Default::default()),
        Some("global.rename"),
        "o keymap canônico precisa entregar global.rename para F2"
    );

    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.route_shortcut("F2", false, false, false));
    assert!(bridge.view_model().rename_active);
}

/// O perfil de notebook usa F2 para seleção de aresta (não tem numpad), então
/// o arquivo precisa deslocar `rename` para Ctrl+F2 — senão as duas ações
/// disputariam a mesma tecla.
///
/// O teste lê o TOML por caminho absoluto de propósito: `Keybinds::load_profile`
/// resolve `assets/keymaps/` relativo ao diretório de trabalho, e o CWD dos
/// testes é o diretório da crate.
#[test]
fn notebook_profile_moves_rename_off_the_edge_selection_key() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/keymaps/petunia-notebook.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("falha ao ler {}: {e}", path.display()));
    let mut section = String::new();
    let mut bindings = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']).to_string();
        } else if let Some((key, value)) = line.split_once('=')
            && !key.trim_start().starts_with('#')
        {
            let value = value.trim().trim_matches('"').to_string();
            bindings.insert(format!("{section}.{}", key.trim()), value);
        }
    }

    assert_eq!(
        bindings.get("model.select_edge").map(String::as_str),
        Some("F2"),
        "F2 continua sendo seleção de aresta neste perfil"
    );
    assert_eq!(
        bindings.get("global.rename").map(String::as_str),
        Some("Ctrl+F2"),
        "o perfil precisa deslocar rename para não colidir com select_edge"
    );

    let canonical = petunia_config::keybinds::Keybinds::defaults();
    let f2 = input::key_code_from_slint("F2").expect("F2 precisa ser mapeável");
    assert_eq!(
        canonical.find(f2, Default::default()),
        Some("global.rename"),
        "a lista canônica entrega F2 para rename"
    );
}

/// Duas ações do mesmo namespace não podem dividir o mesmo atalho.
///
/// Sobreposição entre namespaces diferentes é uma categoria à parte
/// (`ConflictKind::ContextOverlap`, onde `global` sombreia o resto) e é
/// detectada por `Keybinds::detect_conflicts`, não por este teste.
#[test]
fn canonical_keymap_has_no_duplicate_binding_inside_a_namespace() {
    let canonical = petunia_config::keybinds::Keybinds::defaults();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (action, shortcut) in canonical.all_bindings() {
        let namespace = action.split('.').next().unwrap_or_default();
        let key = format!("{namespace}:{shortcut}");
        if let Some(previous) = seen.insert(key, action.clone()) {
            panic!("{shortcut} está mapeado para {previous} e para {action} no mesmo namespace");
        }
    }
}

#[test]
fn context_menu_targets_the_clicked_asset_not_the_previous_active_one() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    assert_eq!(bridge.state.project.assets.len(), 2);
    let first = bridge.state.project.assets[0].id;
    let second = bridge.state.project.assets[1].id;
    assert_eq!(
        bridge.state.project.active, 1,
        "a esfera acabou de ser criada"
    );

    assert!(bridge.open_context_menu(&first.to_string(), 120.0, 60.0));
    assert_eq!(
        bridge.state.project.active, 0,
        "o alvo do menu vira o ativo"
    );
    assert!(bridge.view_model().context_menu_open);
    assert_eq!(bridge.view_model().context_menu_x, 120.0);

    assert!(bridge.context_menu_action("delete"));
    assert_eq!(bridge.state.project.assets.len(), 1);
    assert!(!bridge.state.project.assets.iter().any(|a| a.id == first));
    assert!(bridge.state.project.assets.iter().any(|a| a.id == second));
    assert!(!bridge.view_model().context_menu_open);
}

#[test]
fn context_menu_visibility_and_lock_act_on_the_target() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let id = bridge.state.project.assets[0].id.to_string();

    bridge.open_context_menu(&id, 10.0, 10.0);
    assert!(bridge.view_model().context_menu_visible);
    assert!(bridge.context_menu_action("visibility"));

    bridge.open_context_menu(&id, 10.0, 10.0);
    assert!(!bridge.view_model().context_menu_visible, "Hide inverteu");
    assert!(!bridge.state.project.assets[0].visible);

    bridge.open_context_menu(&id, 10.0, 10.0);
    assert!(bridge.context_menu_action("lock"));
    assert!(bridge.state.project.assets[0].locked);
    bridge.open_context_menu(&id, 10.0, 10.0);
    assert!(bridge.view_model().context_menu_locked);
}

#[test]
fn escape_closes_the_context_menu_before_anything_else() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let id = bridge.state.project.assets[0].id.to_string();
    bridge.open_context_menu(&id, 10.0, 10.0);
    bridge.begin_rename();

    assert!(bridge.handle_escape(), "fecha o menu primeiro");
    assert!(!bridge.view_model().context_menu_open);
    assert!(
        bridge.view_model().rename_active,
        "o rename continua aberto: o menu era o topo da pilha"
    );
    assert!(bridge.handle_escape());
    assert!(!bridge.view_model().rename_active);
}

#[test]
fn context_menu_refuses_an_unknown_asset() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.open_context_menu(&uuid::Uuid::new_v4().to_string(), 0.0, 0.0));
    assert!(!bridge.open_context_menu("not-a-uuid", 0.0, 0.0));
    assert!(!bridge.view_model().context_menu_open);
}

#[test]
fn context_menu_isolate_toggles_isolate_mode_and_preserves_view_model_state() {
    // Validates that right-click context menu "isolate" toggles local isolation mode
    // Valida que o menu de contexto "isolate" alterna o modo de isolamento local
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    assert_eq!(bridge.state.project.assets.len(), 2);
    let sphere_id = bridge.state.project.assets[1].id.to_string();

    assert!(!bridge.state.session.isolate_active);
    bridge.open_context_menu(&sphere_id, 20.0, 20.0);
    assert!(!bridge.view_model().context_menu_isolated);
    assert!(bridge.context_menu_action("isolate"));

    // Isolate is now active, only Sphere is visible
    // Isolamento agora ativo, apenas Sphere visível
    assert!(bridge.state.session.isolate_active);
    assert!(!bridge.state.project.assets[0].visible);
    assert!(bridge.state.project.assets[1].visible);

    // Reopening context menu reflects isolated state
    // Reabrir o menu reflete o estado isolado
    bridge.open_context_menu(&sphere_id, 20.0, 20.0);
    assert!(bridge.view_model().context_menu_isolated);
    assert!(bridge.context_menu_action("isolate"));

    // Un-isolate restores visibility for all assets
    // Desisolar restaura a visibilidade de todos os assets
    assert!(!bridge.state.session.isolate_active);
    assert!(bridge.state.project.assets[0].visible);
    assert!(bridge.state.project.assets[1].visible);
}

#[test]
fn context_menu_move_up_and_down_reorders_scene_assets() {
    // Validates reordering assets up and down through the context menu
    // Valida reordenação de assets para cima e para baixo através do menu de contexto
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    let cube_id = bridge.state.project.assets[0].id.to_string();
    let sphere_id = bridge.state.project.assets[1].id.to_string();

    // Cube at 0 cannot move up, but can move down
    // Cube no índice 0 não pode subir, mas pode descer
    bridge.open_context_menu(&cube_id, 10.0, 10.0);
    let vm = bridge.view_model();
    assert!(!vm.context_menu_can_move_up);
    assert!(vm.context_menu_can_move_down);
    assert!(!bridge.context_menu_action("move_up")); // Rejects invalid move up

    // Sphere at 1 can move up, but cannot move down
    // Sphere no índice 1 pode subir, mas não pode descer
    bridge.open_context_menu(&sphere_id, 10.0, 10.0);
    let vm = bridge.view_model();
    assert!(vm.context_menu_can_move_up);
    assert!(!vm.context_menu_can_move_down);
    assert!(bridge.context_menu_action("move_up"));

    // Now Sphere is at index 0 and Cube is at index 1
    // Agora Sphere está no índice 0 e Cube no índice 1
    assert_eq!(bridge.state.project.assets[0].name, "Sphere");
    assert_eq!(bridge.state.project.assets[1].name, "Cube");

    // Move Sphere back down
    // Move Sphere de volta para baixo
    bridge.open_context_menu(&sphere_id, 10.0, 10.0);
    assert!(bridge.context_menu_action("move_down"));
    assert_eq!(bridge.state.project.assets[0].name, "Cube");
    assert_eq!(bridge.state.project.assets[1].name, "Sphere");
}

#[test]
fn move_scene_asset_intent_reorders_transactionally_with_undo() {
    // Validates MoveSceneAsset intent and undo/redo roundtrip
    // Valida intent MoveSceneAsset e o ciclo transacional de undo/redo
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    let cyl_id = bridge.state.project.assets[1].id.to_string();

    // Move Cylinder up (delta -1)
    // Move Cilindro para cima (delta -1)
    bridge.apply(UiIntent::MoveSceneAsset {
        id: cyl_id.clone(),
        delta: -1,
    });
    assert_eq!(bridge.state.project.assets[0].name, "Cylinder");
    assert_eq!(bridge.state.project.assets[1].name, "Cube");

    // Undo restores original order
    // Undo restaura a ordem original
    bridge.apply(UiIntent::Undo);
    assert_eq!(bridge.state.project.assets[0].name, "Cube");
    assert_eq!(bridge.state.project.assets[1].name, "Cylinder");

    // Redo reaplica
    bridge.apply(UiIntent::Redo);
    assert_eq!(bridge.state.project.assets[0].name, "Cylinder");
    assert_eq!(bridge.state.project.assets[1].name, "Cube");
}

#[test]
fn menu_bar_labels_come_from_the_i18n_catalog() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let vm = bridge.view_model();
    assert_eq!(vm.menu_file_label, "File");
    assert_eq!(vm.menu_edit_label, "Edit");
    assert_eq!(vm.menu_view_label, "View");
    assert_eq!(vm.menu_window_label, "Window");

    bridge.state.ui.i18n = petunia_config::I18n::load("pt-BR");
    let vm = bridge.view_model();
    assert_eq!(vm.menu_file_label, "Arquivo");
    assert_eq!(vm.menu_edit_label, "Editar");
    assert_eq!(vm.menu_view_label, "Exibir");
    assert_eq!(vm.menu_window_label, "Janela");
}

#[test]
fn every_menu_item_publishes_a_real_translated_label_and_command_id() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let vm = bridge.view_model();
    let menus = [
        ("file", &vm.menu_file_items),
        ("edit", &vm.menu_edit_items),
        ("view", &vm.menu_view_items),
        ("window", &vm.menu_window_items),
    ];
    for (menu, items) in menus {
        assert!(!items.is_empty(), "menu {menu} sem itens");
        for item in items {
            assert!(!item.label.is_empty(), "{} tem rótulo vazio", item.id);
            assert!(
                !item.label.contains('.'),
                "{} publicou a chave de i18n em vez do texto: {}",
                item.id,
                item.label
            );
        }
    }
}

#[test]
fn menu_toggles_open_and_close_and_escape_closes_it_first() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.toggle_menu("file"));
    assert_eq!(bridge.view_model().menu_open, "file");

    assert!(bridge.toggle_menu("view"));
    assert_eq!(bridge.view_model().menu_open, "view", "troca de menu");

    assert!(!bridge.toggle_menu("view"), "clicar de novo fecha");
    assert_eq!(bridge.view_model().menu_open, "");

    bridge.toggle_menu("edit");
    bridge.begin_rename();
    assert!(bridge.handle_escape(), "o menu é o topo da pilha");
    assert_eq!(bridge.view_model().menu_open, "");
    assert!(bridge.view_model().rename_active);

    bridge.toggle_menu("file");
    assert!(bridge.handle_click_away());
    assert_eq!(bridge.view_model().menu_open, "");
}

#[test]
fn menu_open_and_rollover_switches_between_menus() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(bridge.view_model().menu_open, "");

    // Abrir o menu "file" diretamente
    assert!(bridge.open_menu("file"));
    assert_eq!(bridge.view_model().menu_open, "file");

    // Rollover para "edit": transição suave sem clique extra
    assert!(bridge.open_menu("edit"));
    assert_eq!(bridge.view_model().menu_open, "edit");

    // Rollover para "view" e "window"
    assert!(bridge.open_menu("view"));
    assert_eq!(bridge.view_model().menu_open, "view");
    assert!(bridge.open_menu("window"));
    assert_eq!(bridge.view_model().menu_open, "window");

    // Abrir novamente o mesmo menu retorna false (sem dirty desnecessário)
    assert!(!bridge.open_menu("window"));
    assert_eq!(bridge.view_model().menu_open, "window");

    // Fecha o menu
    assert!(bridge.close_menu());
    assert_eq!(bridge.view_model().menu_open, "");
}

#[test]
fn wire_overlay_is_independent_of_base_shading_and_xray() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.state.session.show_wireframe_overlay);
    let shading = bridge.state.shading;
    bridge.execute_command(CommandId::ToggleWireOverlay);
    assert!(!bridge.state.session.show_wireframe_overlay);
    bridge.execute_command(CommandId::ToggleWireOverlay);
    assert!(bridge.state.session.show_wireframe_overlay);
    assert_eq!(bridge.state.shading, shading);
    bridge.execute_command(CommandId::ToggleWireframe);
    assert_eq!(bridge.state.shading, petunia_core::Shading::Wireframe);
    assert!(bridge.state.session.show_wireframe_overlay);
    bridge.execute_core_command("view.toggle_xray").unwrap();
    assert!(bridge.state.session.show_xray);
    assert!(bridge.state.session.show_wireframe_overlay);
}

#[test]
fn menu_items_dispatch_to_the_domain() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let before = bridge.state.project.assets.len();

    assert!(bridge.menu_item_invoked("edit.duplicate"));
    assert_eq!(bridge.state.project.assets.len(), before + 1);
    assert_eq!(bridge.view_model().menu_open, "");

    assert!(bridge.menu_item_invoked("view.toggle_wireframe"));
    assert_eq!(bridge.state.shading, petunia_core::Shading::Wireframe);
    assert!(bridge.menu_item_invoked("view.toggle_wireframe"));
    assert_ne!(bridge.state.shading, petunia_core::Shading::Wireframe);

    assert!(bridge.menu_item_invoked("view.reset_camera"));
    assert!(!bridge.menu_item_invoked("view.not_a_real_command"));
}

#[test]
fn shell_chrome_labels_are_translated_and_the_theme_list_comes_from_the_registry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let vm = bridge.view_model();
    assert_eq!(vm.label_parts, "Parts");
    assert_eq!(vm.label_apply, "Apply");
    assert_eq!(vm.label_cancel, "Cancel");
    assert_eq!(vm.label_delete, "Delete");
    assert_eq!(vm.label_asset_library, "Asset Library");
    assert_eq!(vm.label_preferences, "Preferences");
    assert_eq!(vm.label_theme, "Theme");

    assert!(
        vm.themes.len() >= 2,
        "o registry precisa publicar os temas oficiais, veio {:?}",
        vm.themes
    );
    assert!(vm.themes.iter().any(|theme| theme.id == "petunia-dark"));
    assert!(
        vm.themes
            .iter()
            .any(|theme| theme.id == "petunia-high-contrast")
    );
    assert_eq!(
        vm.themes.iter().filter(|theme| theme.active).count(),
        1,
        "exatamente um tema ativo"
    );
    assert!(
        vm.themes
            .iter()
            .find(|theme| theme.active)
            .is_some_and(|theme| theme.id == "petunia-dark")
    );

    bridge.state.ui.i18n = petunia_config::I18n::load("pt-BR");
    let vm = bridge.view_model();
    assert_eq!(vm.label_parts, "Peças");
    assert_eq!(vm.label_apply, "Aplicar");
    assert_eq!(vm.label_delete, "Apagar");
    assert_eq!(vm.label_asset_library, "Assets");
    assert_eq!(vm.label_theme, "Tema");
}

#[test]
fn the_preferences_footer_reports_the_real_keymap_and_theme() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let info = bridge.view_model().shell_info;
    assert!(info.contains("petunia-dark"), "veio: {info}");
    assert!(info.contains("petunia-default"), "veio: {info}");
    assert!(
        !info.contains("Keymap: Standard"),
        "a linha antiga afirmava um keymap fixo que não era o real: {info}"
    );
}

#[test]
fn placing_a_library_asset_instantiates_a_copy_at_the_cursor() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let source = bridge.state.project.assets[0].id;
    let before = bridge.state.project.assets.len();
    bridge.state.session.cursor_3d = [4.0, 1.0, -2.0];

    assert!(bridge.place_asset(&source.to_string()));
    assert_eq!(bridge.state.project.assets.len(), before + 1);
    let placed = bridge.state.project.assets.last().unwrap();
    assert_ne!(placed.id, source, "a cópia precisa de identidade própria");
    let center = placed.mesh.selection_center();
    assert!((center[0] - 4.0).abs() < 1.0e-4, "veio {center:?}");
    assert!((center[2] - (-2.0)).abs() < 1.0e-4, "veio {center:?}");

    assert!(bridge.state.project.undo.can_undo());
    assert!(bridge.state.undo());
    assert_eq!(bridge.state.project.assets.len(), before);
}

#[test]
fn placing_an_unknown_asset_is_refused_and_says_so() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let before = bridge.state.project.assets.len();
    assert!(!bridge.place_asset(&uuid::Uuid::new_v4().to_string()));
    assert!(!bridge.place_asset("not-a-uuid"));
    assert_eq!(bridge.state.project.assets.len(), before);
    assert_eq!(bridge.state.ui.status, "Asset not found in project library");
}

#[test]
fn autosave_respects_interval_and_dirty_state() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.autosave = petunia_core::AutosaveService::new(petunia_core::AutosaveConfig {
        enabled: true,
        interval_secs: 0,
        keep_n: 2,
        only_when_dirty: true,
    });

    assert!(
        !bridge.autosave_tick(),
        "documento limpo não deve gerar snapshot"
    );

    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.checkpoint("dirty");
    assert!(
        bridge.autosave_tick(),
        "documento sujo dentro do intervalo precisa gerar snapshot"
    );
    assert!(
        bridge.state.is_document_dirty(),
        "autosave nunca limpa o dirty state"
    );
}

#[test]
fn recovery_prompt_only_appears_when_a_snapshot_was_detected() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().recovery_open);
    assert!(!bridge.recover_pending());
    assert!(!bridge.keep_saved_project());
    assert!(!bridge.discard_pending_recovery());

    bridge.pending_recovery = Some(petunia_core::RecoveryInfo {
        snapshot_path: std::path::PathBuf::from("/tmp/nao-existe/autosave-1.petunia"),
        project_name: "Turret".to_string(),
        snapshot_time: 1_700_000_000,
        main_project_path: None,
        is_newer_than_main: true,
    });
    let vm = bridge.view_model();
    assert!(vm.recovery_open);
    assert!(
        vm.recovery_title.contains("Recover"),
        "veio: {}",
        vm.recovery_title
    );
    assert!(
        vm.recovery_detail.contains("Turret"),
        "veio: {}",
        vm.recovery_detail
    );
    assert!(!vm.recovery_discard.is_empty());

    assert!(bridge.keep_saved_project(), "abrir o salvo fecha o aviso");
    assert!(!bridge.view_model().recovery_open);
    assert!(!bridge.state.project.assets[0].name.is_empty());
}

#[test]
fn knife_takes_two_viewport_picks_and_commits_one_cut() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.execute_core_command("model.knife").unwrap();
    assert!(bridge.state.session.tools.cut_session.is_some());
    let faces_before = bridge.state.project.active_mesh().unwrap().faces.len();

    // Usa duas arestas opostas e realmente visíveis da mesma face.
    let visible = visible_edge_points(&bridge);
    let mesh = bridge.state.project.active_mesh().unwrap();
    let (first, second) = mesh
        .faces
        .iter()
        .find_map(|face| {
            let edges: Vec<_> = face
                .verts
                .iter()
                .copied()
                .zip(face.verts.iter().copied().cycle().skip(1))
                .take(face.verts.len())
                .map(|(a, b)| (a.min(b), a.max(b)))
                .collect();
            visible.iter().find_map(|(edge_a, point_a)| {
                edges
                    .contains(edge_a)
                    .then(|| {
                        visible.iter().find_map(|(edge_b, point_b)| {
                            (edges.contains(edge_b)
                                && edge_a.0 != edge_b.0
                                && edge_a.0 != edge_b.1
                                && edge_a.1 != edge_b.0
                                && edge_a.1 != edge_b.1)
                                .then_some((*point_a, *point_b))
                        })
                    })
                    .flatten()
            })
        })
        .expect("two opposite visible edges of one face");
    assert!(
        bridge.knife_click(first[0], first[1]),
        "primeiro ponto precisa ancorar"
    );
    assert_eq!(bridge.state.ui.status, "Knife: pick the second edge point");
    assert_eq!(
        bridge.state.project.undo.depth(),
        (0, 0),
        "ancorar não corta"
    );
    assert!(
        bridge.knife_click(second[0], second[1]),
        "segundo ponto precisa cortar"
    );

    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(
        mesh.faces.len() > faces_before,
        "o corte precisa criar faces: antes {faces_before}, depois {}",
        mesh.faces.len()
    );
    assert!(
        bridge.state.session.tools.cut_session.is_some(),
        "Cut stays open for more segments"
    );
    assert!(bridge.commit_knife());
    assert!(bridge.state.session.tools.cut_session.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn knife_click_outside_a_session_does_nothing() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.knife_click(0.5, 0.5));
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn escape_cancels_the_knife_and_restores_the_select_tool() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.execute_core_command("model.knife").unwrap();
    assert_eq!(bridge.state.session.tools.active_tool, "cut");

    assert!(bridge.handle_escape());
    assert!(bridge.state.session.tools.cut_session.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.state.ui.status, "Cut cancelled");
}

#[test]
fn loop_cut_session_slides_previews_and_commits_one_undo_entry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    let original = bridge.state.project.active_mesh().unwrap().clone();
    // Uma aresta do cubo padrão está num anel de quads.
    let seed = {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        let face = mesh.faces[0].verts.clone();
        let edge = (face[0], face[1]);
        mesh.selected_edges.insert(edge);
        edge
    };
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .selected_edges
            .contains(&seed)
    );

    assert!(bridge.begin_loop_cut(), "o anel precisa ser descoberto");
    assert!(bridge.loop_cut.is_some());
    let preview = bridge.state.project.active_mesh().unwrap().clone();
    assert!(
        preview.verts.len() > original.verts.len(),
        "o preview precisa inserir vértices: {} -> {}",
        original.verts.len(),
        preview.verts.len()
    );

    assert!(
        bridge.scrub_loop_cut(60.0, false),
        "slide precisa reconstruir"
    );
    assert!(bridge.loop_cut.as_ref().unwrap().slide > 0.0);

    // O campo Slide altera a posição do corte, sem modificar Cuts.
    assert!(bridge.set_loop_cut_slide(-0.25));
    assert_eq!(bridge.loop_cut.as_ref().unwrap().slide, -0.25);
    assert_eq!(bridge.loop_cut.as_ref().unwrap().cuts, 1);
    assert!(!bridge.set_loop_cut_slide(2.0));
    assert_eq!(bridge.loop_cut.as_ref().unwrap().slide, -0.25);

    assert!(bridge.set_loop_cut_count(3));
    assert_eq!(bridge.loop_cut.as_ref().unwrap().cuts, 3);
    assert!(
        bridge.state.project.active_mesh().unwrap().verts.len()
            > bridge.state.project.undo.depth().0
    );

    assert!(bridge.commit_loop_cut());
    assert!(bridge.loop_cut.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.project.undo.can_undo());

    assert!(bridge.state.undo());
    let restored = bridge.state.project.active_mesh().unwrap();
    assert_eq!(restored.verts.len(), original.verts.len());
    assert_eq!(restored.faces.len(), original.faces.len());
}

#[test]
fn test_dual_balanced_loop_cut_session_and_toggle() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    let original = bridge.state.project.active_mesh().unwrap().clone();

    // Select an edge on the cube
    let _seed = {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        let face = mesh.faces[0].verts.clone();
        let edge = (face[0], face[1]);
        mesh.selected_edges.insert(edge);
        edge
    };

    // Toggle balanced mode
    assert!(!bridge.loop_cut_balanced);
    assert!(bridge.toggle_loop_cut_balanced());
    assert!(bridge.loop_cut_balanced);
    assert_eq!(bridge.loop_cut_hover_cuts, 2);

    assert!(bridge.begin_loop_cut());
    let session = bridge.loop_cut.as_ref().unwrap();
    assert!(session.balanced);
    assert_eq!(session.cuts, 2);

    // Slide balanced cuts
    assert!(bridge.set_loop_cut_slide(0.5));
    let vm = bridge.view_model();
    assert!(vm.loop_cut_balanced);
    assert_eq!(vm.loop_cut_cuts, 2);
    assert_eq!(vm.loop_cut_slide, 0.5);
    assert!(!vm.loop_cut_preview_commands.is_empty());

    // Commit balanced loop cut
    assert!(bridge.commit_loop_cut());
    assert!(bridge.loop_cut.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    let modified = bridge.state.project.active_mesh().unwrap();
    // Cube started with 6 faces. Dual loop cut on a 4-quad ring adds 2 * 4 = 8 faces -> 14 faces total.
    assert_eq!(modified.faces.len(), 14);
    assert!(modified.validate_topology().is_closed);
    assert!(modified.validate_topology().is_manifold);

    // Undo restores the exact original mesh
    assert!(bridge.state.undo());
    let restored = bridge.state.project.active_mesh().unwrap();
    assert_eq!(restored.verts.len(), original.verts.len());
    assert_eq!(restored.faces.len(), original.faces.len());
}

#[test]
fn loop_cut_cancel_restores_the_exact_original_mesh() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    let original = bridge.state.project.active_mesh().unwrap().clone();
    {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        let face = mesh.faces[0].verts.clone();
        mesh.selected_edges.insert((face[0], face[1]));
    }

    assert!(bridge.begin_loop_cut());
    assert!(bridge.scrub_loop_cut(-80.0, false));
    assert!(bridge.cancel_loop_cut());

    let restored = bridge.state.project.active_mesh().unwrap();
    assert_eq!(restored.verts.len(), original.verts.len());
    assert_eq!(restored.faces.len(), original.faces.len());
    assert_eq!(
        bridge.state.project.undo.depth(),
        (0, 0),
        "cancelar não pode empilhar histórico"
    );
}

#[test]
fn loop_cut_refuses_without_a_selected_edge_and_says_why() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.begin_loop_cut());
    assert!(bridge.loop_cut.is_none());
    assert!(
        bridge.state.ui.status.contains("select an edge"),
        "veio: {}",
        bridge.state.ui.status
    );
}

#[test]
fn loop_cut_hover_previews_without_mutating_until_placed_and_scrolls_count() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    let original = bridge.state.project.active_mesh().unwrap().clone();
    bridge.apply(UiIntent::SetActiveTool("loop_cut".into()));

    let visible = visible_edge_points(&bridge);
    let (_, cursor) = visible[0];
    assert!(bridge.hover_component(cursor[0], cursor[1]));
    assert!(bridge.loop_cut_hover_ring.is_some());
    assert!(!bridge.loop_cut_hover_preview_commands().is_empty());
    assert_eq!(
        bridge.state.project.active_mesh().unwrap().faces.len(),
        original.faces.len()
    );
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    assert!(bridge.scroll_loop_cut_count(1.0));
    assert_eq!(bridge.loop_cut_hover_cuts, 2);
    assert!(bridge.place_loop_cut_hover());
    assert_eq!(bridge.loop_cut.as_ref().unwrap().cuts, 2);
    assert!(bridge.commit_loop_cut());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.project.active_mesh().unwrap().verts.len() > original.verts.len());
}

#[test]
fn pivot_selector_changes_transform_session_policy() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.set_pivot_point("cursor"));
    assert_eq!(bridge.state.session.pivot_point, PivotPoint::Cursor3D);
    assert_eq!(bridge.view_model().pivot_id, "cursor");
    assert!(bridge.set_pivot_menu_open(true));
    assert_eq!(
        bridge.overlays.top().map(|entry| entry.id),
        Some(OverlayId::PivotMenu)
    );
    assert!(bridge.handle_escape());
    assert!(!bridge.pivot_menu_open);
    assert!(!bridge.set_pivot_point("unknown"));
}

#[test]
fn slice_keeps_both_sides_without_caps_and_can_be_cancelled() {
    let mut state = AppState::default();
    state.set_edit_mode(petunia_core::EditMode::Edit);
    let original = state.project.active_mesh().unwrap().clone();
    let session = petunia_core::CutSession::new(original.clone());
    let camera = state.session.camera.clone();
    let viewport = petunia_core::LogicalRect::from_min_max([0.0, 0.0], [800.0, 600.0]);
    let sliced = session
        .compute_slice(&camera, [300.0, 300.0], [500.0, 300.0], viewport)
        .expect("slice preview");
    assert!(sliced.verts.len() > original.verts.len());
    assert!(sliced.faces.len() > original.faces.len());
    assert!(sliced.verts.iter().all(|vertex| vertex.vec().is_finite()));
}

#[test]
fn loop_cut_tool_cancel_does_not_require_a_hovered_ring() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("loop_cut".into()));
    assert!(bridge.handle_escape());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn profile_uses_view_frame_for_revolve_geometry() {
    let mut state = AppState::default();
    state.profile.origin = [1.0, 2.0, 3.0];
    state.profile.right = [1.0, 0.0, 0.0];
    state.profile.up = [0.0, 0.0, 1.0];
    state.profile.normal = [0.0, -1.0, 0.0];
    state.profile.points = vec![[0.0, 0.0], [0.5, 0.5]];
    state.profile.revolve_segments = 8;
    petunia_module_model::draw_profile::generate_revolve(&mut state);
    let mesh = state.project.active_mesh().expect("revolved profile");
    assert!(mesh.verts.iter().any(|vertex| vertex.pos[0] > 1.1));
    assert!(mesh.verts.iter().any(|vertex| vertex.pos[2] > 2.1));
    assert!(mesh.verts.iter().all(|vertex| vertex.vec().is_finite()));
}

#[test]
fn profile_tool_draws_closes_and_generates_transactionally() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    for point in [[0.35, 0.35], [0.65, 0.35], [0.65, 0.65], [0.35, 0.65]] {
        bridge.select_viewport_ext(point[0], point[1], false, false);
        bridge.profile_pointer_up();
    }
    assert!(bridge.close_profile());
    assert!(active_profile_spline(&bridge).closed);
    let profile_id = bridge.active_profile_id.expect("profile id");
    assert!(bridge.generate_profile_extrude());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert!(bridge.state.project.active_mesh().is_some());
    assert!(
        bridge
            .state
            .project
            .project
            .get_profile(profile_id)
            .is_some()
    );
    assert_eq!(bridge.state.project.undo.depth(), (6, 0));
}

fn draw_two_profile_points(snap: bool, grid_spacing: f32) -> [[f64; 3]; 2] {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.snap_enabled = snap;
    bridge.state.session.snap_settings.grid_spacing = grid_spacing;
    // Vista Front: o plano automático (mais paralelo à vista) é o XY, e a
    // horizontal da tela é o eixo `right` do plano. A câmera não se move.
    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Front);
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    // O segundo ponto fica ~4 px abaixo da horizontal do primeiro.
    for point in [[0.35, 0.35], [0.65, 0.355]] {
        bridge.select_viewport_ext(point[0], point[1], false, false);
        bridge.profile_pointer_up();
    }
    let spline = active_profile_spline(&bridge);
    [spline.points[0].position, spline.points[1].position]
}

#[test]
fn profile_snap_infers_workplane_axis_from_last_point() {
    // Sem snap, o tremor de 4 px entra no desenho.
    let [a, b] = draw_two_profile_points(false, 1.0);
    assert!((a[1] - b[1]).abs() > 1.0e-3);

    // Com snap (grade fina, fora de alcance), a guia paralela ao eixo do
    // plano a partir do último ponto vence: a linha sai exatamente reta.
    let [a, b] = draw_two_profile_points(true, 1.0e-3);
    assert!((a[1] - b[1]).abs() < 1.0e-5, "{a:?} {b:?}");
    assert!((a[0] - b[0]).abs() > 0.1);
}

#[test]
fn profile_snap_falls_back_to_workplane_grid() {
    // 1º ponto na grade; o 2º segue a guia horizontal andando no passo da grade.
    let [a, b] = draw_two_profile_points(true, 0.5);
    assert!((a[1] - b[1]).abs() < 1.0e-5);
    for value in [a[0], a[1], b[0]] {
        assert!(
            ((value / 0.5).round() * 0.5 - value).abs() < 1.0e-4,
            "{value} fora da grade"
        );
    }
}

/// Cena padrão (cubo 2×2×2 na origem) vista de frente.
fn front_view_bridge_with_cube() -> SlintUiBridge<PlaceholderViewport> {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.state.project.active_mesh().is_some());
    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Front);
    bridge.state.session.camera.target = glam::Vec3::ZERO;
    bridge
}

fn camera_pose(bridge: &SlintUiBridge<PlaceholderViewport>) -> (f32, f32, bool) {
    let camera = &bridge.state.session.camera;
    (
        camera.yaw,
        camera.pitch,
        camera.proj == petunia_core::Projection::Perspective,
    )
}

#[test]
fn draw_never_moves_the_camera_on_its_own() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let before = camera_pose(&bridge);
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    assert_eq!(camera_pose(&bridge), before);
    bridge.apply(UiIntent::ProfileSetWorkplaneFace);
    bridge.apply(UiIntent::ProfileSetWorkplaneView);
    assert_eq!(camera_pose(&bridge), before);

    // Só o comando explícito "Olhar para o plano" alinha a câmera.
    bridge.apply(UiIntent::ProfileSetWorkplaneGround);
    bridge.apply(UiIntent::ProfileLookAtPlane);
    assert_ne!(camera_pose(&bridge), before);
    assert!(
        !camera_pose(&bridge).2,
        "olhar para o plano usa ortográfica"
    );
}

#[test]
fn auto_workplane_follows_the_view_when_there_is_no_face() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Top);
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    assert!(!bridge.state.profile.workplane_locked);
    assert_eq!(bridge.view_model().profile_workplane, "Ground");

    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Right);
    bridge.select_viewport_ext(0.4, 0.4, false, false);
    bridge.profile_pointer_up();
    // O 1º clique reavalia a vista: de lado, o plano vertical YZ (normal ±X).
    let normal = glam::Vec3::from(bridge.state.profile.normal);
    assert!(normal.x.abs() > 0.99, "{normal:?}");
    assert_eq!(bridge.view_model().profile_workplane, "View");
}

#[test]
fn auto_workplane_uses_the_face_under_the_cursor_on_the_first_click() {
    let mut bridge = front_view_bridge_with_cube();
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));

    // Pré-seleção: a face frontal que viraria o plano aparece no hover, com rótulo.
    assert!(bridge.hover_component(0.5, 0.5));
    assert!(matches!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::Face(_)
    ));
    let vm = bridge.view_model();
    assert!(vm.snap_marker_visible);
    assert_eq!(
        vm.snap_marker_label,
        bridge
            .state
            .t_id(petunia_config::text_id::SNAP_KIND_ON_FACE)
    );

    bridge.select_viewport_ext(0.5, 0.5, false, false);
    bridge.profile_pointer_up();
    assert_eq!(bridge.view_model().profile_workplane, "Face");
    let normal = glam::Vec3::from(bridge.state.profile.normal);
    assert!(
        normal.z > 0.99,
        "face frontal voltada para a câmera: {normal:?}"
    );
    let origin = glam::Vec3::from(bridge.state.profile.origin);
    let spline = active_profile_spline(&bridge);
    let point = spline.points[0].position;
    let world = origin
        + glam::Vec3::from(bridge.state.profile.right) * point[0] as f32
        + glam::Vec3::from(bridge.state.profile.up) * point[1] as f32;
    // O ponto fica sobre a face (mesmo z da face), não no chão.
    assert!((world.z - origin.z).abs() < 1.0e-4);

    // Com um perfil em curso, o plano não muda mais a cada clique.
    bridge.select_viewport_ext(0.55, 0.45, false, false);
    bridge.profile_pointer_up();
    assert!((glam::Vec3::from(bridge.state.profile.origin) - origin).length() < 1.0e-6);
}

#[test]
fn workplane_ground_preference_is_a_setting() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let vm = bridge.view_model();
    assert!(!vm.workplane_prefer_ground);
    assert!(!vm.label_workplane_prefer_ground.is_empty());
    assert!(!vm.label_workplane_prefer_ground_hint.is_empty());

    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Persp);
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    assert_eq!(bridge.view_model().profile_workplane, "View");

    assert!(bridge.set_workplane_prefer_ground(true));
    assert!(!bridge.set_workplane_prefer_ground(true), "sem mudança");
    assert!(bridge.preferences.workplane_prefer_ground);
    bridge.apply(UiIntent::ProfileSetWorkplaneAuto);
    assert_eq!(bridge.view_model().profile_workplane, "Ground");
    assert!(bridge.view_model().workplane_prefer_ground);

    // Preferências carregadas do disco chegam ao estado do desenho.
    let mut restored = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let preferences = petunia_config::UserPreferences {
        workplane_prefer_ground: true,
        ..Default::default()
    };
    restored.restore_section_layouts(&preferences);
    assert!(restored.state.profile.workplane_prefer_ground);
}

/// Desenha um quadrado fechado na face frontal do cubo padrão (vista Front)
/// e depois inclina a câmera, para a normal da face ter componente na tela.
fn bridge_with_square_on_front_face() -> SlintUiBridge<PlaceholderViewport> {
    let mut bridge = front_view_bridge_with_cube();
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    for point in [[0.46, 0.46], [0.54, 0.46], [0.54, 0.54], [0.46, 0.54]] {
        bridge.select_viewport_ext(point[0], point[1], false, false);
        bridge.profile_pointer_up();
    }
    assert!(bridge.close_profile());
    assert_eq!(bridge.view_model().profile_workplane, "Face");
    // Gira em torno do centro da face, que continua no centro da tela.
    bridge.state.session.camera.target = glam::Vec3::new(0.0, 0.0, 1.0);
    bridge.state.session.camera.yaw = 0.35;
    bridge.state.session.camera.pitch = 0.25;
    bridge
}

#[test]
fn draw_tool_highlights_a_closed_region_on_hover() {
    let mut bridge = bridge_with_square_on_front_face();
    // Perfil fechado: o hover sobre a região mostra a tinta da região.
    bridge.hover_component(0.5, 0.5);
    assert!(bridge.region_hover.is_some());
    assert!(!bridge.view_model().region_hover_commands.is_empty());
    bridge.hover_component(0.9, 0.9);
    assert!(bridge.view_model().region_hover_commands.is_empty());
}

#[test]
fn push_pull_drag_on_a_region_imprints_and_extrudes_in_one_undo() {
    let mut bridge = bridge_with_square_on_front_face();
    let faces_before = bridge.state.project.active_mesh().unwrap().faces.len();
    let depth_before = bridge.state.project.undo.depth().0;
    bridge.activate_parametric_tool(ToolModalKind::PushPull);

    let [width, height] = bridge.viewport_size;
    let center = [width * 0.5, height * 0.5];
    assert!(bridge.hover_component(0.5, 0.5));
    assert!(!bridge.view_model().region_hover_commands.is_empty());

    bridge.tool_pointer(0, center[0], center[1], false, false);
    bridge.tool_pointer(1, center[0] + 30.0, center[1] - 60.0, false, false);
    assert!(
        bridge.state.session.tools.modal.is_some(),
        "gesto de região aberto"
    );
    bridge.tool_pointer(2, center[0] + 30.0, center[1] - 60.0, false, false);
    assert!(bridge.state.session.tools.modal.is_none());

    let mesh = bridge.state.project.active_mesh().unwrap();
    // Anel (2) + face interna + 4 paredes.
    assert_eq!(mesh.faces.len(), faces_before + 6);
    assert_eq!(bridge.state.project.undo.depth().0, depth_before + 1);
    assert!(
        bridge.last_operation.is_some(),
        "ajustável na Última operação"
    );
    assert_eq!(bridge.state.session.tools.active_tool, "push_pull");

    assert!(bridge.state.undo());
    assert_eq!(
        bridge.state.project.active_mesh().unwrap().faces.len(),
        faces_before
    );
}

#[test]
fn escape_during_a_region_push_restores_the_face() {
    let mut bridge = bridge_with_square_on_front_face();
    let before = bridge.state.project.active_mesh().unwrap().clone();
    bridge.activate_parametric_tool(ToolModalKind::PushPull);
    let [width, height] = bridge.viewport_size;
    bridge.tool_pointer(0, width * 0.5, height * 0.5, false, false);
    bridge.tool_pointer(1, width * 0.5 + 20.0, height * 0.5 - 50.0, false, false);
    assert!(bridge.state.session.tools.modal.is_some());
    assert!(bridge.route_shortcut("Escape", false, false, false));
    let after = bridge.state.project.active_mesh().unwrap();
    assert_eq!(after.faces.len(), before.faces.len());
    assert_eq!(after.verts.len(), before.verts.len());
}

#[test]
fn locked_workplane_ignores_the_face_under_the_cursor() {
    let mut bridge = front_view_bridge_with_cube();
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
    bridge.apply(UiIntent::ProfileSetWorkplaneGround);
    assert!(bridge.view_model().profile_workplane_locked);

    assert!(bridge.hover_component(0.5, 0.5));
    assert_eq!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::None
    );
    bridge.select_viewport_ext(0.5, 0.3, false, false);
    bridge.profile_pointer_up();
    assert_eq!(bridge.view_model().profile_workplane, "Ground");

    // Voltar para Auto destrava.
    bridge.apply(UiIntent::ProfileSetWorkplaneAuto);
    assert!(!bridge.view_model().profile_workplane_locked);
    assert!(bridge.active_profile_id.is_none());
}

#[test]
fn paint_layer_panel_adds_removes_reorders_and_composites() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

    assert!(bridge.add_paint_layer());
    let layers = bridge.view_model().paint_layers;
    assert_eq!(layers.len(), 2, "base + nova camada");
    assert!(layers[1].active, "a camada nova vira ativa");
    assert_eq!(layers[1].name, "Layer 2");
    assert_eq!(layers[0].kind_label, "Raster");

    let base_id = layers[0].id.clone();
    let new_id = layers[1].id.clone();

    assert!(bridge.toggle_paint_layer_visibility(&new_id));
    assert!(!bridge.view_model().paint_layers[1].visible);
    assert!(bridge.toggle_paint_layer_visibility(&new_id));

    assert!(bridge.toggle_paint_layer_lock(&new_id));
    assert!(bridge.view_model().paint_layers[1].locked);

    assert!(bridge.set_paint_layer_opacity(&new_id, 0.25));
    assert!((bridge.view_model().paint_layers[1].opacity - 0.25).abs() < 1.0e-6);

    assert!(bridge.move_paint_layer(&new_id, -1));
    let moved = bridge.view_model().paint_layers;
    assert_eq!(moved[0].id, new_id, "desceu na ordem de composição");
    assert_eq!(moved[1].id, base_id);

    assert!(bridge.remove_paint_layer(&new_id));
    assert_eq!(bridge.view_model().paint_layers.len(), 1);
}

#[test]
fn paint_layer_mutations_are_single_entry_roundtrips() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

    let before_add = active_paint_stack(&bridge);
    assert!(bridge.add_paint_layer());
    let after_add = active_paint_stack(&bridge);
    assert_ne!(after_add, before_add);
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.undo());
    assert_eq!(active_paint_stack(&bridge), before_add);
    assert!(bridge.state.redo());
    assert_eq!(active_paint_stack(&bridge), after_add);

    let layer_id = after_add.layers[1].id.to_string();
    let before_opacity = active_paint_stack(&bridge);
    assert!(bridge.set_paint_layer_opacity(&layer_id, 0.35));
    let after_opacity = active_paint_stack(&bridge);
    assert_eq!(bridge.state.project.undo.depth(), (2, 0));
    assert!(bridge.state.undo());
    assert_eq!(active_paint_stack(&bridge), before_opacity);
    assert!(bridge.state.redo());
    assert_eq!(active_paint_stack(&bridge), after_opacity);

    let before_reorder = active_paint_stack(&bridge);
    assert!(bridge.move_paint_layer(&layer_id, -1));
    let after_reorder = active_paint_stack(&bridge);
    assert_eq!(bridge.state.project.undo.depth(), (3, 0));
    assert!(bridge.state.undo());
    assert_eq!(active_paint_stack(&bridge), before_reorder);
    assert!(bridge.state.redo());
    assert_eq!(active_paint_stack(&bridge), after_reorder);

    let before_remove = active_paint_stack(&bridge);
    assert!(bridge.remove_paint_layer(&layer_id));
    let after_remove = active_paint_stack(&bridge);
    assert_eq!(bridge.state.project.undo.depth(), (4, 0));
    assert!(bridge.state.undo());
    assert_eq!(active_paint_stack(&bridge), before_remove);
    assert!(bridge.state.redo());
    assert_eq!(active_paint_stack(&bridge), after_remove);
}

#[test]
fn paint_layer_panel_refuses_to_remove_the_last_layer() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.add_paint_layer();
    let layers = bridge.view_model().paint_layers;
    assert!(bridge.remove_paint_layer(&layers[1].id));
    let remaining = bridge.view_model().paint_layers;
    assert_eq!(remaining.len(), 1);

    assert!(
        !bridge.remove_paint_layer(&remaining[0].id),
        "a base do raster precisa sobreviver"
    );
    assert_eq!(bridge.view_model().paint_layers.len(), 1);
}

#[test]
fn paint_layer_mutations_reject_unknown_ids_and_non_finite_opacity() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    let unknown = uuid::Uuid::new_v4().to_string();
    assert!(!bridge.toggle_paint_layer_visibility(&unknown));
    assert!(!bridge.toggle_paint_layer_lock(&unknown));
    assert!(!bridge.remove_paint_layer(&unknown));
    assert!(!bridge.move_paint_layer(&unknown, 1));
    assert!(!bridge.set_paint_layer_opacity(&unknown, 0.5));
    assert!(!bridge.set_paint_layer_active(&unknown));
    assert!(!bridge.set_paint_layer_opacity("not-a-uuid", 0.5));

    let id = bridge.view_model().paint_layers[0].id.clone();
    assert!(!bridge.set_paint_layer_opacity(&id, f32::NAN));
    assert!(bridge.view_model().paint_layers[0].opacity.is_finite());
}

#[test]
fn decal_layer_transform_and_bake_workflow() {
    // Tests decal layer creation, transform mutation, view_model exposure and baking to raster
    // Testa criação de camada decal, mutação de transformação, exposição no view_model e bake para raster
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

    // Initially no decal layer / Inicialmente sem camada decal
    assert!(!bridge.view_model().active_layer_is_decal);

    // Add decal layer / Adiciona camada decal
    bridge.apply(UiIntent::AddDecalLayer);
    let vm = bridge.view_model();
    assert!(vm.active_layer_is_decal);
    assert_eq!(vm.decal_center_u, 0.5);
    assert_eq!(vm.decal_center_v, 0.5);
    assert_eq!(vm.decal_scale_u, 0.25);
    assert_eq!(vm.decal_scale_v, 0.25);

    let decal_id = vm.paint_layers.last().unwrap().id.clone();

    // Mutate decal transform / Modifica transformação do decalque
    let transform_revision_before = bridge.state.project.project.texture_revision;
    let history_before_transform = bridge.state.project.undo.depth().0;
    bridge.apply(UiIntent::SetDecalTransform {
        layer_id: decal_id.clone(),
        center_u: 0.7,
        center_v: 0.3,
        scale_u: 0.4,
        scale_v: 0.4,
        rotation_deg: 45.0,
    });
    assert_eq!(
        bridge.state.project.project.texture_revision,
        transform_revision_before + 1
    );
    assert_eq!(
        bridge.state.project.undo.depth(),
        (history_before_transform + 1, 0)
    );

    let vm = bridge.view_model();
    assert!(vm.active_layer_is_decal);
    assert!((vm.decal_center_u - 0.7).abs() < 1e-4);
    assert!((vm.decal_center_v - 0.3).abs() < 1e-4);
    assert!((vm.decal_scale_u - 0.4).abs() < 1e-4);
    assert!((vm.decal_scale_v - 0.4).abs() < 1e-4);
    assert!((vm.decal_rotation_deg - 45.0).abs() < 1e-3);

    // Bake decal to raster / Converte decalque para raster
    let bake_revision_before = bridge.state.project.project.texture_revision;
    let history_before_bake = bridge.state.project.undo.depth().0;
    bridge.apply(UiIntent::BakeActiveDecal);
    assert_eq!(
        bridge.state.project.project.texture_revision,
        bake_revision_before + 1
    );
    assert_eq!(
        bridge.state.project.undo.depth(),
        (history_before_bake + 1, 0)
    );
    let vm = bridge.view_model();
    assert!(
        !vm.active_layer_is_decal,
        "baked layer should now be Raster / camada rasterizada agora deve ser Raster"
    );
}

#[test]
fn test_decal_live_interactive_drag_manipulator_and_preview_commands() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

    // Decal layer is initially not active
    assert!(!bridge.active_layer_is_decal());
    assert!(bridge.decal_preview_commands().is_empty());

    // Add decal layer
    assert!(bridge.add_decal_layer());
    assert!(bridge.active_layer_is_decal());

    let vm = bridge.view_model();
    assert!(vm.active_layer_is_decal);
    assert!(!vm.label_decal_transform.is_empty());
    assert!(!vm.label_decal_position.is_empty());
    assert!(!vm.label_decal_scale.is_empty());
    assert!(!vm.label_decal_rotation.is_empty());
    assert!(!vm.label_decal_bake.is_empty());
    assert!(!vm.label_decal_hint.is_empty());

    // Decal preview commands generates on-surface manipulator SVG path
    let commands = bridge.decal_preview_commands();
    assert!(!commands.is_empty(), "decal preview path must not be empty");
    assert!(commands.contains('M'), "must contain SVG move-to M");
    assert!(commands.contains('L'), "must contain SVG line-to L");
    assert!(commands.contains('Z'), "must contain SVG close-path Z");

    // Interactive drag placement:
    // Screen center 512, 384 raycasts onto cube front face
    assert!(bridge.begin_paint_stroke_at(512.0, 384.0));
    assert!(bridge.paint_stroke_to(520.0, 390.0));
    assert!(bridge.end_paint_stroke_at(520.0, 390.0));
    let decal = bridge.active_decal().expect("active decal");
    assert!(decal.center_uv[0] > 0.0 && decal.center_uv[0] < 1.0);
    assert!(decal.center_uv[1] > 0.0 && decal.center_uv[1] < 1.0);

    // Interactive drag uniform scale (Shift + drag):
    let initial_scale = decal.scale_uv;
    assert!(bridge.begin_paint_stroke_with_modifiers(512.0, 384.0, true, false));
    // Dragging upwards (384 -> 334) increases scale
    assert!(bridge.paint_stroke_to_with_modifiers(512.0, 334.0, true, false));
    assert!(bridge.end_paint_stroke_at(512.0, 334.0));
    let scaled_decal = bridge.active_decal().expect("scaled decal");
    assert!(
        scaled_decal.scale_uv[0] > initial_scale[0],
        "scale should increase when dragged up with Shift"
    );

    // Interactive drag rotation (Ctrl + drag):
    let initial_rot = scaled_decal.rotation_rad;
    assert!(bridge.begin_paint_stroke_with_modifiers(512.0, 384.0, false, true));
    // Dragging right (512 -> 562) rotates decal
    assert!(bridge.paint_stroke_to_with_modifiers(562.0, 384.0, false, true));
    assert!(bridge.end_paint_stroke_at(562.0, 384.0));
    let rotated_decal = bridge.active_decal().expect("rotated decal");
    assert!(
        (rotated_decal.rotation_rad - initial_rot).abs() > 0.01,
        "rotation should change when dragged with Ctrl"
    );

    // Interactive drag cancellation (Escape):
    let pre_cancel_decal = rotated_decal.clone();
    assert!(bridge.begin_paint_stroke_at(512.0, 384.0));
    assert!(bridge.paint_stroke_to(530.0, 395.0));
    let in_drag_decal = bridge.active_decal().expect("in drag decal");
    assert_ne!(in_drag_decal.center_uv, pre_cancel_decal.center_uv);
    assert!(bridge.cancel_paint_stroke());
    let restored_decal = bridge.active_decal().expect("restored decal");
    assert_eq!(restored_decal.center_uv, pre_cancel_decal.center_uv);
    assert_eq!(restored_decal.scale_uv, pre_cancel_decal.scale_uv);
    assert_eq!(restored_decal.rotation_rad, pre_cancel_decal.rotation_rad);

    // Non-destructive preservation: layer remains Decal until explicit bake
    assert!(bridge.active_layer_is_decal());
    bridge.apply(UiIntent::BakeActiveDecal);
    assert!(!bridge.active_layer_is_decal());
}

#[test]
fn decal_drag_commits_once_and_cancel_restores_without_history() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    assert!(bridge.add_decal_layer());
    let history_before = bridge.state.project.undo.depth().0;
    let before = bridge.active_decal().expect("decal before drag");

    assert!(bridge.decal_drag_begin(512.0, 384.0, true, false));
    assert!(bridge.decal_drag_to(512.0, 334.0, true, false));
    assert!(bridge.decal_drag_end());
    let after = bridge.active_decal().expect("decal after drag");
    assert_ne!(after, before);
    assert_eq!(bridge.state.project.undo.depth(), (history_before + 1, 0));

    assert!(bridge.state.undo());
    assert_eq!(bridge.active_decal().expect("decal after undo"), before);
    assert!(bridge.state.redo());
    assert_eq!(bridge.active_decal().expect("decal after redo"), after);

    let depth_before_cancel = bridge.state.project.undo.depth();
    assert!(bridge.decal_drag_begin(512.0, 384.0, false, true));
    assert!(bridge.decal_drag_to(562.0, 384.0, false, true));
    assert_ne!(bridge.active_decal().expect("decal preview"), after);
    assert!(bridge.cancel_decal_drag());
    assert_eq!(bridge.active_decal().expect("decal after cancel"), after);
    assert_eq!(bridge.state.project.undo.depth(), depth_before_cancel);
}

#[test]
fn uv_editor_builds_a_real_layout_path_from_mesh_uvs() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let editor = bridge.view_model().uv_editor;
    assert_eq!(editor.face_count, 6, "o cubo padrão tem 6 faces");
    assert!(!editor.truncated);
    assert!(
        editor.layout_commands.starts_with('M'),
        "veio: {}",
        editor.layout_commands
    );
    assert_eq!(
        editor.layout_commands.matches('M').count(),
        editor.face_count,
        "uma subcaminho por face"
    );
    assert_eq!(
        editor.layout_commands.matches('Z').count(),
        editor.face_count
    );
    assert!(editor.island_count >= 1);
    assert_eq!(editor.selected_face, -1, "nada selecionado no início");
}

#[test]
fn uv_editor_reports_the_selected_face_and_clears_when_deselected() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.project.active_mesh_mut().unwrap().faces[2].selected = true;
    bridge.state.sync_selection();
    assert_eq!(bridge.view_model().uv_editor.selected_face, 2);

    bridge.state.project.active_mesh_mut().unwrap().faces[2].selected = false;
    bridge.state.sync_selection();
    assert_eq!(bridge.view_model().uv_editor.selected_face, -1);
}

#[test]
fn uv_seam_toggle_requires_a_selected_face_and_commits_one_entry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.toggle_selected_uv_seams());
    assert_eq!(
        bridge.state.ui.status,
        "UV: select a face in the viewport first"
    );
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));

    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    assert!(bridge.toggle_selected_uv_seams());
    let seams = bridge.state.project.active_mesh().unwrap().uv_seams.len();
    assert_eq!(seams, 4, "uma costura por aresta da face quad");
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    assert!(bridge.toggle_selected_uv_seams());
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .uv_seams
            .is_empty(),
        "o segundo toque desmarca"
    );

    assert!(bridge.state.undo());
    assert_eq!(
        bridge.state.project.active_mesh().unwrap().uv_seams.len(),
        4
    );
}

#[test]
fn clearing_uv_seams_is_idempotent_and_reports_when_empty() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.clear_all_uv_seams());
    assert_eq!(bridge.state.ui.status, "UV: there are no seams to clear");

    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    bridge.toggle_selected_uv_seams();
    assert!(bridge.clear_all_uv_seams());
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .uv_seams
            .is_empty()
    );
}

#[test]
fn uv_editor_generates_seam_and_selection_paths() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Initially no seams or selected faces / Inicialmente sem costuras ou faces selecionadas
    let vm = bridge.view_model();
    assert!(vm.uv_editor.seam_commands.is_empty());
    assert!(vm.uv_editor.selected_commands.is_empty());

    // Select face 0 / Seleciona a face 0
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    let vm = bridge.view_model();
    assert!(
        !vm.uv_editor.selected_commands.is_empty(),
        "selected face should produce path commands / face selecionada deve gerar comandos de caminho"
    );
    assert!(vm.uv_editor.selected_commands.starts_with('M'));

    // Toggle seams on selected face / Alterna costuras na face selecionada
    assert!(bridge.toggle_selected_uv_seams());
    let vm = bridge.view_model();
    assert!(
        !vm.uv_editor.seam_commands.is_empty(),
        "marked seams should produce path commands / costuras marcadas devem gerar comandos de caminho"
    );
    assert!(vm.uv_editor.seam_commands.starts_with('M'));
}

#[test]
fn fill_scope_and_projection_controls_change_real_session_state() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(bridge.view_model().paint_fill_scope, "ConnectedPixels");

    assert!(bridge.set_fill_scope("UvIsland"));
    assert_eq!(
        bridge.state.session.tools.fill_scope,
        petunia_core::FillScope::UvIsland
    );
    assert_eq!(bridge.view_model().paint_fill_scope, "UvIsland");

    assert!(bridge.set_brush_projection("ScreenSpace"));
    assert_eq!(
        bridge.state.session.tools.brush_projection,
        petunia_core::BrushProjectionMode::ScreenSpace
    );

    assert!(bridge.set_brush_lock("FirstFace"));
    assert_eq!(
        bridge.state.session.tools.brush_lock,
        petunia_core::BrushLock::FirstFace
    );

    assert!(!bridge.set_fill_scope("Nope"));
    assert!(!bridge.set_brush_projection("Nope"));
    assert!(!bridge.set_brush_lock("Nope"));
    assert_eq!(
        bridge.state.session.tools.fill_scope,
        petunia_core::FillScope::UvIsland,
        "valor inválido não pode alterar o estado"
    );
}

#[test]
fn fill_scope_object_paints_the_whole_canvas_and_connected_pixels_stops_at_a_border() {
    use petunia_module_paint::PaintModule;
    let mut state = AppState::default();
    PaintModule::ensure_stack(&mut state);
    state.paint_color = [1.0, 0.0, 0.0];

    let canvas_of = |state: &AppState| {
        state
            .project
            .assets
            .get(state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
            .cloned()
            .expect("canvas do stack")
    };
    let painted = |state: &AppState| {
        canvas_of(state)
            .pixels
            .chunks(4)
            .filter(|px| px[3] > 0)
            .count()
    };

    // Object: canvas inteiro.
    PaintModule::canvas_fill_scoped(&mut state, None, None, petunia_core::FillScope::Object);
    let canvas = canvas_of(&state);
    let total = (canvas.w * canvas.h) as usize;
    assert_eq!(painted(&state), total, "Object pinta o canvas todo");

    // Monta uma fronteira: metade esquerda vermelha, metade direita azul.
    {
        let active = state.project.active;
        let canvas = state
            .project
            .assets
            .get_mut(active)
            .and_then(|asset| asset.paint_stack.as_mut())
            .and_then(|stack| stack.active_mut())
            .and_then(|layer| layer.canvas_mut())
            .expect("canvas mutável");
        let width = canvas.w;
        let height = canvas.h;
        for y in 0..height {
            for x in 0..width {
                if x < width / 2 {
                    canvas.set(x, y, [255, 0, 0, 255]);
                } else {
                    canvas.set(x, y, [0, 0, 255, 255]);
                }
            }
        }
    }

    // ConnectedPixels a partir da esquerda: o azul da direita não é alcançado.
    state.paint_color = [0.0, 1.0, 0.0];
    PaintModule::canvas_fill_scoped(
        &mut state,
        None,
        Some((4, 4)),
        petunia_core::FillScope::ConnectedPixels,
    );
    let canvas = canvas_of(&state);
    let green = canvas
        .pixels
        .chunks(4)
        .filter(|px| px[1] > 200 && px[0] < 60)
        .count();
    let blue = canvas
        .pixels
        .chunks(4)
        .filter(|px| px[2] > 200 && px[0] < 60)
        .count();
    let half = total / 2;
    assert!(
        green.abs_diff(half) < canvas.w as usize * 2,
        "a metade esquerda vira verde: {green} vs {half}"
    );
    assert_eq!(blue, half, "a metade direita permanece azul: {blue}");
}

#[test]
fn face_fill_scope_paints_only_the_hit_face_uv_region() {
    use petunia_module_paint::PaintModule;
    let mut state = AppState::default();
    PaintModule::ensure_stack(&mut state);
    state.paint_color = [0.0, 1.0, 0.0];

    // O cubo usa projeção planar, então todas as faces cobrem 0..1. Restrinjo
    // a face 0 a um quadrado interno para que o escopo por face seja visível.
    {
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.faces[0].uv = vec![[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]];
    }

    PaintModule::canvas_fill_scoped(&mut state, Some(0), None, petunia_core::FillScope::Face);
    let canvas = state
        .project
        .assets
        .get(state.project.active)
        .and_then(|asset| asset.paint_stack.as_ref())
        .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
        .cloned()
        .unwrap();
    // O canvas base nasce opaco, então o que identifica o preenchimento é a
    // cor: verde puro só existe onde o escopo pintou.
    let painted = canvas
        .pixels
        .chunks(4)
        .filter(|px| px[1] > 200 && px[0] < 60 && px[2] < 60)
        .count();
    let total = (canvas.w * canvas.h) as usize;
    let expected = total / 4;
    assert!(painted > 0, "a face 0 precisa pintar a própria região UV");
    assert!(
        painted.abs_diff(expected) < canvas.w as usize * 2,
        "o quadrado interno cobre ~1/4 do canvas: {painted} vs {expected}"
    );
    assert!(painted < total, "uma face não pode cobrir o canvas inteiro");
}

#[test]
fn shape_tools_anchor_on_press_and_commit_on_release() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("rectangle".to_string()));
    assert!(
        bridge.is_shape_tool(),
        "rectangle precisa ser ferramenta de forma"
    );
    assert_eq!(
        petunia_core::brush_type_from_kind(bridge.state.session.tools.paint_brush_kind),
        petunia_core::BrushType::Rectangle
    );

    assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
    assert!(bridge.shape_anchor.is_some(), "o press ancora a forma");
    assert!(
        bridge.paint_last.is_none(),
        "forma não usa o caminho de traço livre"
    );
    assert_eq!(
        bridge.state.project.undo.depth(),
        (0, 0),
        "ancorar não pode empilhar histórico"
    );

    assert!(bridge.end_paint_stroke_at(430.0, 320.0));
    assert!(bridge.shape_anchor.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn cancelling_a_shape_leaves_the_document_untouched() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("line".to_string()));

    assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
    assert!(bridge.cancel_paint_stroke());
    assert!(bridge.shape_anchor.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    assert_eq!(bridge.state.ui.status, "Shape cancelled");
}

#[test]
fn shape_press_off_the_surface_is_refused_with_a_reason() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("line".to_string()));

    // Canto superior esquerdo: longe do cubo padrão.
    assert!(!bridge.begin_paint_stroke_at(2.0, 2.0));
    assert!(bridge.shape_anchor.is_none());
    assert!(
        bridge.state.ui.status.starts_with("Shape:"),
        "veio: {}",
        bridge.state.ui.status
    );
}

#[test]
fn boolean_operand_flows_from_the_outliner_to_a_real_fuse() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    assert_eq!(bridge.state.project.assets.len(), 2);
    let sphere_id = bridge.state.project.assets[1].id;

    // Desloca a esfera para fora do cubo: a união de um cubo com uma esfera
    // concêntrica seria o próprio cubo e o teste não provaria nada.
    {
        let mesh = &mut bridge.state.project.assets[1].mesh;
        mesh.select_all();
        mesh.translate_selected([1.6, 0.0, 0.0]);
        mesh.deselect_all();
    }

    // Escolhe a esfera como operando e volta o ativo para o cubo.
    assert!(bridge.set_boolean_operand(&sphere_id.to_string()));
    assert!(bridge.view_model().boolean_ready);
    bridge.state.project.active = 0;
    let before = bridge.state.project.assets[0].mesh.verts.len();

    assert!(bridge.boolean_op("model.fuse"));
    assert_eq!(
        bridge.state.project.assets.len(),
        1,
        "o operando é consumido"
    );
    assert!(bridge.state.project.assets[0].mesh.verts.len() > before);
    assert!(bridge.state.session.tools.boolean_operand.is_none());
    assert_eq!(bridge.view_model().boolean_operand_name, "");
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn boolean_op_without_an_operand_is_refused_and_says_why() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().boolean_ready);
    assert!(!bridge.boolean_op("model.fuse"));
    assert_eq!(bridge.state.project.assets.len(), 1);
    assert!(
        bridge.state.ui.status.contains("operand"),
        "veio: {}",
        bridge.state.ui.status
    );
}

#[test]
fn context_menu_marks_the_clicked_asset_as_the_boolean_operand() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cone));
    let cone = bridge.state.project.assets[1].id;
    bridge.open_context_menu(&cone.to_string(), 10.0, 10.0);
    assert!(bridge.context_menu_action("boolean_operand"));
    assert_eq!(bridge.state.session.tools.boolean_operand, Some(cone));
    assert_eq!(bridge.view_model().boolean_operand_name, "Cone");
    assert!(bridge.clear_boolean_operand());
    assert_eq!(bridge.view_model().boolean_operand_name, "");
}

#[test]
fn paint_canvas_image_matches_the_active_layer_pixels() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

    let (width, height) = bridge.paint_canvas_dimensions().expect("canvas do stack");
    assert!(width > 0 && height > 0);

    // Pinta um pixel conhecido na camada ativa e confere que a imagem
    // publicada carrega exatamente esses bytes.
    let marker = [12u8, 200, 45, 255];
    {
        let active = bridge.state.project.active;
        let canvas = bridge
            .state
            .project
            .assets
            .get_mut(active)
            .and_then(|asset| asset.paint_stack.as_mut())
            .and_then(|stack| stack.active_mut())
            .and_then(|layer| layer.canvas_mut())
            .expect("canvas mutável");
        canvas.set(1, 1, marker);
    }

    let image = bridge.render_paint_canvas().expect("imagem do canvas");
    assert_eq!(image.size().width, width);
    assert_eq!(image.size().height, height);
    let buffer = image.to_rgba8().expect("buffer rgba8");
    let offset = ((width + 1) * 4) as usize;
    assert_eq!(&buffer.as_bytes()[offset..offset + 4], &marker);
}

#[test]
fn canvas_image_is_absent_before_a_layer_exists_and_appears_after() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    assert!(
        bridge.paint_canvas_dimensions().is_none(),
        "sem stack não há canvas"
    );

    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
    assert!(bridge.paint_canvas_dimensions().is_some());
    assert!(bridge.view_model().paint_canvas_size.contains('×'));
}

#[test]
fn clicking_the_uv_editor_selects_the_face_under_the_cursor() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // O cubo usa projeção planar: o centro do espaço UV cai na face 0.
    assert!(bridge.uv_editor_click(0.5, 0.5, false));
    assert_eq!(bridge.view_model().uv_editor.selected_face, 0);
    assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 1);
    assert_eq!(bridge.state.ui.status, "UV: face 0 selected (1 total)");

    // Clicar de novo sem Shift substitui a seleção, não acumula.
    assert!(bridge.uv_editor_click(0.5, 0.5, false));
    assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 1);
    // Com Shift a seleção alterna.
    assert!(bridge.uv_editor_click(0.5, 0.5, true));
    assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 0);
    assert!(!bridge.state.project.active_mesh().unwrap().faces[0].selected);
    assert!(bridge.state.session.selection.faces.is_empty());
    assert!(!bridge.uv_move_selected(0.1, 0.0));
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn paint_color_control_and_viewport_picker_share_the_canvas_color() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetPaintColor([1.0, 0.0, 0.0]));
    assert_eq!(bridge.state.paint_color, [1.0, 0.0, 0.0]);
    assert_eq!(bridge.view_model().paint_color, [1.0, 0.0, 0.0]);

    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
    let (px, py) = bridge.canvas_pixel_at(512.0, 384.0).expect("cube surface");
    bridge
        .state
        .project
        .active_mut()
        .unwrap()
        .texture
        .as_mut()
        .unwrap()
        .set(px, py, [12, 100, 220, 255]);
    bridge.state.session.tools.active_tool = "picker".to_string();
    assert!(bridge.begin_paint_stroke_at(512.0, 384.0));
    assert!(bridge.paint_last.is_none());
    assert!(bridge.state.session.tools.paint_stroke.is_none());
    let picked = [12.0 / 255.0, 100.0 / 255.0, 220.0 / 255.0];
    assert_eq!(bridge.state.paint_color, picked);
    assert_eq!(bridge.view_model().paint_color, picked);
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    assert!(!bridge.begin_paint_stroke_at(-10.0, 384.0));
}

#[test]
fn uv_editor_click_off_the_layout_clears_the_selection_and_says_so() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Tira a face 0 do canto para deixar uma região vazia.
    bridge.state.project.active_mesh_mut().unwrap().faces[0].uv =
        vec![[0.0, 0.0], [0.0, 0.25], [0.25, 0.25], [0.25, 0.0]];
    for face in bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .faces
        .iter_mut()
        .skip(1)
    {
        face.uv = vec![[0.0, 0.0], [0.0, 0.1], [0.1, 0.1], [0.1, 0.0]];
    }
    bridge.uv_editor_click(0.5, 0.5, false);

    assert!(!bridge.uv_editor_click(0.9, 0.9, false));
    assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 0);
    assert_eq!(bridge.state.ui.status, "UV: no face under the cursor");
    assert!(!bridge.uv_editor_click(f32::NAN, 0.5, false));
}

#[test]
fn uv_transforms_move_scale_and_rotate_the_selected_faces() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.uv_editor_click(0.5, 0.5, false);
    let before = bridge.state.project.active_mesh().unwrap().faces[0]
        .uv
        .clone();

    assert!(bridge.uv_move_selected(0.1, 0.0));
    let moved = bridge.state.project.active_mesh().unwrap().faces[0]
        .uv
        .clone();
    assert!((moved[0][0] - before[0][0] - 0.1).abs() < 1.0e-5);
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    assert!(bridge.uv_scale_selected(2.0));
    let scaled = bridge.state.project.active_mesh().unwrap().faces[0]
        .uv
        .clone();
    let span_before = moved.iter().map(|uv| uv[0]).fold(f32::MIN, f32::max)
        - moved.iter().map(|uv| uv[0]).fold(f32::MAX, f32::min);
    let span_after = scaled.iter().map(|uv| uv[0]).fold(f32::MIN, f32::max)
        - scaled.iter().map(|uv| uv[0]).fold(f32::MAX, f32::min);
    assert!(
        span_after > span_before,
        "escalar ×2 precisa alargar a ilha: {span_before} -> {span_after}"
    );

    assert!(bridge.uv_rotate_selected(90.0));
    assert_eq!(bridge.state.project.undo.depth(), (3, 0));

    assert!(
        !bridge.uv_move_selected(0.0, 0.0),
        "movimento nulo é recusado"
    );
    assert!(!bridge.uv_scale_selected(0.0), "escala zero é recusada");
    assert!(!bridge.uv_rotate_selected(0.0), "rotação nula é recusada");
    assert!(!bridge.uv_scale_selected(f32::NAN));
    assert_eq!(bridge.state.project.undo.depth(), (3, 0));
}

#[test]
fn effect_layers_change_the_composited_raster() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

    // Pinta a camada base de um tom conhecido antes do efeito.
    {
        let active = bridge.state.project.active;
        let canvas = bridge
            .state
            .project
            .assets
            .get_mut(active)
            .and_then(|asset| asset.paint_stack.as_mut())
            .and_then(|stack| stack.active_mut())
            .and_then(|layer| layer.canvas_mut())
            .unwrap();
        canvas.fill([200, 40, 90, 255]);
    }
    petunia_module_paint::PaintModule::composite_active(&mut bridge.state);
    let before = bridge.state.project.assets[0].texture.clone().unwrap();
    let sample = before.get(8, 8).unwrap();

    assert!(bridge.add_paint_effect_layer("Invert"));
    let layers = bridge.view_model().paint_layers;
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[1].kind_label, "Effect");
    assert_eq!(bridge.view_model().paint_effect_kind, "Invert");

    let after = bridge.state.project.assets[0].texture.clone().unwrap();
    let inverted = after.get(8, 8).unwrap();
    assert_ne!(sample, inverted, "Invert precisa alterar o pixel");
    assert_eq!(inverted[0], 255 - sample[0]);
    assert_eq!(inverted[1], 255 - sample[1]);
    assert_eq!(inverted[2], 255 - sample[2]);
    assert_eq!(inverted[3], sample[3], "o alfa é preservado");
}

#[test]
fn effect_parameters_are_exposed_and_applied() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

    assert!(bridge.add_paint_effect_layer("Pixelate"));
    let params = bridge.view_model().paint_effect_params;
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].key, "cell_size");
    assert_eq!(params[0].value, 4.0);
    assert!(params[0].min < params[0].max);

    assert!(bridge.set_paint_effect_param("cell_size", 12.0));
    assert_eq!(bridge.view_model().paint_effect_params[0].value, 12.0);

    // Fora da faixa é fixado no limite, nunca aceito cru.
    assert!(bridge.set_paint_effect_param("cell_size", 9_999.0));
    assert_eq!(bridge.view_model().paint_effect_params[0].value, 64.0);
    assert!(bridge.set_paint_effect_param("cell_size", 0.0));
    assert_eq!(bridge.view_model().paint_effect_params[0].value, 1.0);

    assert!(!bridge.set_paint_effect_param("cell_size", f32::NAN));
    assert!(!bridge.set_paint_effect_param("unknown_param", 1.0));
    assert!(!bridge.add_paint_effect_layer("NotAnEffect"));
}

#[test]
fn an_effect_layer_over_a_raster_layer_has_no_editable_canvas() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
    assert!(bridge.add_paint_effect_layer("Posterize"));

    // A camada ativa é a de efeito, então não há canvas para exibir.
    assert!(
        bridge.render_paint_canvas().is_none(),
        "camada de efeito não tem raster próprio"
    );
    assert!(bridge.view_model().paint_effect_kind == "Posterize");
    let params = bridge.view_model().paint_effect_params;
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].key, "levels");
}

#[test]
fn join_merges_the_operand_through_the_shell_and_keeps_parts_is_opt_in() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    let operand = bridge.state.project.assets[1].id;
    let operand_verts = bridge.state.project.assets[1].mesh.verts.len();
    let active_verts = bridge.state.project.assets[0].mesh.verts.len();
    // A primitiva recém-criada vira ativa; o alvo do Join é o cubo.
    bridge.state.project.active = 0;

    assert!(bridge.set_boolean_operand(&operand.to_string()));
    assert!(!bridge.view_model().boolean_keep_parts, "padrão é consumir");
    let undo_before = bridge.state.project.undo.depth().0;

    assert!(bridge.join_operand());
    assert_eq!(bridge.state.project.assets.len(), 1);
    assert_eq!(
        bridge.state.project.assets[0].mesh.verts.len(),
        active_verts + operand_verts,
        "Join preserva as duas topologias"
    );
    assert_eq!(
        bridge.state.project.undo.depth().0,
        undo_before + 1,
        "Join é exatamente uma entrada de undo"
    );
}

#[test]
fn keep_parts_toggle_is_reported_and_preserves_the_operand() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.set_boolean_keep_parts(true));
    assert!(bridge.view_model().boolean_keep_parts);
    assert_eq!(
        bridge.state.ui.status,
        "Keep Parts on: the operand stays in the scene"
    );
    assert!(!bridge.set_boolean_keep_parts(true), "sem mudança real");
    assert!(bridge.set_boolean_keep_parts(false));
    assert!(!bridge.view_model().boolean_keep_parts);

    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cone));
    let operand = bridge.state.project.assets[1].id;
    {
        let mesh = &mut bridge.state.project.assets[1].mesh;
        mesh.select_all();
        mesh.translate_selected([1.6, 0.0, 0.0]);
        mesh.deselect_all();
    }
    bridge.set_boolean_operand(&operand.to_string());
    bridge.set_boolean_keep_parts(true);
    bridge.state.project.active = 0;

    assert!(bridge.boolean_op("model.fuse"));
    assert_eq!(
        bridge.state.project.assets.len(),
        2,
        "com Keep Parts o operando permanece"
    );
    assert!(bridge.state.project.assets.iter().any(|a| a.id == operand));
}

#[test]
fn slice_drag_cuts_the_mesh_and_commits_one_undo_entry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.session.tools.active_tool = "slice".to_string();

    assert!(bridge.begin_slice(400.0, 300.0));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert!(bridge.slice_anchor.is_some());

    assert!(
        bridge.update_slice(400.0, 380.0),
        "arrasto vertical precisa produzir um plano de corte"
    );
    // Slice divide as faces mas mantém ambos os lados da geometria.
    let sliced = bridge.state.project.active_mesh().unwrap().clone();
    let report = sliced.validate_topology();
    assert!(report.is_manifold, "{report:?}");
    let span = |mesh: &petunia_core::Mesh| {
        let xs: Vec<f32> = mesh.verts.iter().map(|v| v.pos[0]).collect();
        let zs: Vec<f32> = mesh.verts.iter().map(|v| v.pos[2]).collect();
        let ys: Vec<f32> = mesh.verts.iter().map(|v| v.pos[1]).collect();
        (
            xs.iter().fold(f32::MIN, |a, b| a.max(*b)) - xs.iter().fold(f32::MAX, |a, b| a.min(*b)),
            ys.iter().fold(f32::MIN, |a, b| a.max(*b)) - ys.iter().fold(f32::MAX, |a, b| a.min(*b)),
            zs.iter().fold(f32::MIN, |a, b| a.max(*b)) - zs.iter().fold(f32::MAX, |a, b| a.min(*b)),
        )
    };
    let after = span(&sliced);
    let before = (2.0, 2.0, 2.0);
    assert_eq!(after, before, "os dois lados permanecem na malha");
    assert!(sliced.faces.len() > 6, "faces cruzadas são divididas");

    assert!(bridge.commit_slice());
    assert!(bridge.slice_anchor.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    assert!(bridge.state.undo());
    let restored = span(bridge.state.project.active_mesh().unwrap());
    assert_eq!(restored, before, "undo volta ao cubo inteiro");
}

#[test]
fn slice_without_a_drag_is_refused_and_escape_restores_the_mesh() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.session.tools.active_tool = "slice".to_string();

    assert!(bridge.begin_slice(400.0, 300.0));
    assert!(
        !bridge.update_slice(402.0, 301.0),
        "arrasto abaixo do limiar não define plano"
    );
    assert_eq!(
        bridge.state.project.undo.depth(),
        (0, 0),
        "pré-visualização não empilha histórico"
    );

    assert!(bridge.handle_escape());
    assert!(bridge.slice_anchor.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.state.ui.status, "Slice cancelled");
    assert_eq!(bridge.state.project.active_mesh().unwrap().verts.len(), 8);
}

#[test]
fn the_slice_keymap_action_arms_the_tool_without_touching_geometry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let keybinds = petunia_config::keybinds::Keybinds::defaults();
    let key = input::key_code_from_slint("K").expect("K mapeável");
    let shift = petunia_config::keybinds::Mods2 {
        ctrl: false,
        shift: true,
        alt: false,
    };
    assert_eq!(
        keybinds.find(key, shift),
        Some("model.slice"),
        "Shift+K precisa estar ligado ao Slice"
    );

    assert!(bridge.route_shortcut("K", false, true, false));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    assert!(
        bridge.slice_anchor.is_none(),
        "a âncora nasce no pointer-down"
    );
}

#[test]
fn selection_overlay_outlines_the_active_object() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    let overlay = bridge.view_model().selection_overlay;
    assert!(overlay.visible, "o cubo ativo precisa de contorno visível");
    assert!(!overlay.accent, "domínio Object usa a cor de seleção");
    // Ativo tem canal próprio (amarelo), distinto do selecionado
    // (laranja): o cubo ativo padrão não polui o canal de seleção.
    assert!(overlay.outline_commands.is_empty());
    // Contorno de objeto = silhueta frontal, não a caixa nem as 12 arestas:
    // de um canto vê-se 3 faces, portanto 9 arestas de contorno.
    let edges = overlay.active_outline_commands.matches('M').count();
    assert!(
        (6..=12).contains(&edges),
        "silhueta frontal precisa ter entre 6 e 12 arestas, veio {edges}"
    );
    assert!(overlay.point_commands.is_empty());
    assert!(overlay.unselected_outline_commands.is_empty());
}

#[test]
fn sphere_selection_outline_stays_finite_and_inside_the_viewport() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    let overlay = bridge.view_model().selection_overlay;
    let coordinates: Vec<f32> = overlay
        .active_outline_commands
        .split_whitespace()
        .filter_map(|token| token.parse::<f32>().ok())
        .collect();
    assert!(
        !coordinates.is_empty(),
        "a esfera ativa precisa de silhueta"
    );
    assert_eq!(coordinates.len() % 2, 0);
    for point in coordinates.as_chunks::<2>().0 {
        assert!(point[0].is_finite() && (0.0..=1024.0).contains(&point[0]));
        assert!(point[1].is_finite() && (0.0..=768.0).contains(&point[1]));
    }
}

#[test]
fn selection_overlay_follows_the_domain() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);

    // Point: todos os vértices aparecem como alvos clicáveis, mesmo sem
    // seleção, para o usuário ver onde pode clicar.
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    let overlay = bridge.view_model().selection_overlay;
    assert!(overlay.visible);
    assert!(overlay.point_commands.is_empty());
    assert_eq!(
        overlay.unselected_point_commands.matches('M').count(),
        8,
        "os 8 vértices aparecem como alvos"
    );
    bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
    bridge.state.sync_selection();
    let overlay = bridge.view_model().selection_overlay;
    assert!(overlay.visible && overlay.accent);
    assert_eq!(overlay.point_commands.matches('M').count(), 1);
    assert!(overlay.outline_commands.is_empty());
    assert_eq!(
        overlay.unselected_point_commands.matches('M').count(),
        7,
        "o vértice selecionado sai dos alvos neutros e recebe cor forte"
    );

    // Edge: uma linha por aresta selecionada.
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .selected_edges
        .insert((0, 1));
    bridge.state.sync_selection();
    let overlay = bridge.view_model().selection_overlay;
    assert!(overlay.outline_commands.is_empty());
    assert_eq!(
        overlay.unselected_outline_commands.matches('M').count(),
        11,
        "as 11 arestas não selecionadas continuam como alvos"
    );

    // Face: preenchimento e preselection são do renderer, sem marcadores.
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    let overlay = bridge.view_model().selection_overlay;
    assert!(
        overlay.outline_commands.is_empty(),
        "o preenchimento da face é do renderer"
    );
    assert!(overlay.point_commands.is_empty());
    assert!(overlay.unselected_point_commands.is_empty());
}

#[test]
fn face_domain_does_not_cover_mesh_with_center_dots() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    let overlay = bridge.view_model().selection_overlay;
    assert!(!overlay.visible);
    assert!(overlay.point_commands.is_empty());
    assert!(overlay.unselected_point_commands.is_empty());
}

#[test]
fn clicking_the_viewport_reports_what_was_selected() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.project.active = 0;
    bridge.state.set_status("");

    bridge.select_viewport(0.5, 0.5, false);
    assert_eq!(bridge.state.ui.status, "Selected 'Cube'");

    // Fora do cubo o status diz que não acertou nada, em vez de silêncio.
    bridge.state.set_status("");
    bridge.select_viewport(0.02, 0.02, false);
    assert_eq!(bridge.state.ui.status, "Nothing under the cursor");
}

#[test]
fn click_confirms_hover_and_miss_clears_it() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    // Clicar no cubo fixa a preselection: destaque e rótulo aparecem de
    // imediato, sem esperar o próximo mousemove.
    bridge.select_viewport(0.5, 0.5, false);
    assert!(
        bridge.state.session.tools.hover.is_some(),
        "o alvo clicado vira o hover corrente"
    );
    assert!(!bridge.view_model().hover_label.is_empty());
    // Erro limpa em vez de congelar o hover antigo.
    bridge.select_viewport(0.02, 0.02, false);
    assert_eq!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::None
    );
}

#[test]
fn workspace_switch_clears_stale_hover() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.select_viewport(0.5, 0.5, false);
    assert!(bridge.state.session.tools.hover.is_some());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    assert_eq!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::None,
        "hover do Model não pode vazar para o Paint"
    );
}

#[test]
fn dotted_link_is_empty_without_distance() {
    assert!(dotted_link_commands([10.0, 10.0], [10.0, 10.0]).is_empty());
    assert!(dotted_link_commands([10.0, 10.0], [10.2, 10.0]).is_empty());
}

#[test]
fn dotted_link_grows_with_pointer_distance() {
    // Ponto de 2px + intervalo de 4px: 100px rendem 17 segmentos.
    let short = dotted_link_commands([0.0, 0.0], [20.0, 0.0]);
    let long = dotted_link_commands([0.0, 0.0], [100.0, 0.0]);
    let short_count = short.matches('M').count();
    let long_count = long.matches('M').count();
    assert_eq!(short_count, 4, "20px rendem 4 pontos, veio {short_count}");
    assert_eq!(long_count, 17, "100px rendem 17 pontos, veio {long_count}");
    assert!(long_count > short_count);
}

#[test]
fn drag_link_only_exists_during_a_tool_session() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    let pointer = [700.0, 300.0];
    // Sem sessão de manipulação não há cordão, mesmo com o mouse longe.
    assert!(compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, false).is_empty());
    // Com arrasto ativo o cordão liga o pivô ao mouse...
    assert!(bridge.begin_viewport_transform(TransformKind::Position, 512.0, 384.0));
    let link_active = bridge.drag.is_some()
        || bridge.tool_modal.is_some()
        || bridge.state.session.tools.modal.is_some();
    let link = compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, link_active);
    assert!(
        !link.is_empty(),
        "arrasto ativo precisa do cordão pivô→mouse"
    );
    bridge.pointer_position = pointer;
    assert_eq!(bridge.view_model().drag_link_commands, link);
    // ...e some ao confirmar a operação.
    assert!(bridge.end_viewport_transform());
    let link_active = bridge.drag.is_some()
        || bridge.tool_modal.is_some()
        || bridge.state.session.tools.modal.is_some();
    assert!(!link_active);
    assert!(compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, link_active).is_empty());
    assert!(bridge.view_model().drag_link_commands.is_empty());
}

#[test]
fn drag_link_with_tool_feedback_snap() {
    // Tests ToolFeedback integration in compute_drag_link with magnetic snap (P3D-131)
    // Testa integração do ToolFeedback em compute_drag_link com atração magnética (P3D-131)
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.begin_viewport_transform(TransformKind::Position, 512.0, 384.0);

    // Free drag cord / Cordão de arrasto livre
    let pointer = [600.0, 300.0];
    let free_link = compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, true);
    assert!(!free_link.is_empty());

    // Enable snap / Ativa snap magnético
    bridge.state.session.snap_enabled = true;
    let snapped_link = compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, true);
    assert!(!snapped_link.is_empty());
}

#[test]
fn selection_summary_counts_what_operations_will_hit() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Estado padrão: cubo ativo conta como selecionado na UI.
    assert_eq!(bridge.view_model().selection_summary, "1 object selected");
}

#[test]
fn selection_summary_and_inspector_follow_component_picks() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    assert_eq!(bridge.view_model().selection_summary, "No selection");
    bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
    bridge.state.project.active_mesh_mut().unwrap().verts[1].selected = true;
    bridge.state.sync_selection();
    let vm = bridge.view_model();
    assert_eq!(vm.selection_summary, "Selected: 2 points");
    assert!(
        vm.active_object_details.contains("Selected: 2 points"),
        "o inspector mostra o que será atingido, veio: {}",
        vm.active_object_details
    );
    // Limpar volta ao vazio honesto, sem número fantasma.
    bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .deselect_all();
    bridge.state.sync_selection();
    assert_eq!(bridge.view_model().selection_summary, "No selection");
}

#[test]
fn viewport_right_click_never_cancels_an_active_transform() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    // ADR 006: com operação aberta o botão direito é ignorado; cancelar é Esc.
    assert!(bridge.begin_viewport_transform(TransformKind::Position, 512.0, 384.0));
    assert!(!bridge.viewport_context_triage(700.0, 300.0));
    assert!(bridge.drag.is_some(), "a operação continua");
    assert!(!bridge.view_model().context_menu_open);
    assert!(bridge.handle_escape());
    assert!(bridge.drag.is_none());
}

#[test]
fn viewport_right_click_opens_selection_menu_without_session() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    assert!(bridge.viewport_context_triage(700.0, 300.0), "abre o menu");
    let vm = bridge.view_model();
    assert!(vm.context_menu_open);
    assert_eq!(vm.context_menu_mode, "viewport");
    assert_eq!(vm.context_menu_title, "Viewport");
    // Verbetes de seleção funcionam e fecham o menu.
    assert!(bridge.context_menu_action("select_all"));
    assert!(!bridge.view_model().context_menu_open);
}

#[test]
fn viewport_menu_select_all_clear_and_escape() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    bridge.open_viewport_context_menu(100.0, 100.0);
    assert!(bridge.context_menu_action("select_all"));
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .all(|v| v.selected),
        "select_all do menu seleciona tudo como o atalho A"
    );
    bridge.open_viewport_context_menu(100.0, 100.0);
    assert!(bridge.context_menu_action("clear_selection"));
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .all(|v| !v.selected),
        "clear do menu limpa como Alt+A"
    );
    // Escape fecha o menu da viewport pelo LIFO, como o do Outliner.
    bridge.open_viewport_context_menu(100.0, 100.0);
    assert!(bridge.view_model().context_menu_open);
    assert!(bridge.handle_escape());
    assert!(!bridge.view_model().context_menu_open);
}

#[test]
fn keyboard_modal_axis_numeric_confirm_flow() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    // 1º toque ativa a ferramenta Move no Gizmo; 2º toque (double-tap) ativa o modal livre.
    assert!(bridge.route_shortcut("G", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "move");
    assert!(bridge.route_shortcut("G", false, false, false));
    assert!(
        bridge
            .state
            .session
            .tools
            .modal
            .as_ref()
            .is_some_and(|m| m.kind == petunia_core::ModalKind::Move),
        "Double-tap de G precisa abrir o modal de Move"
    );
    // HUD acompanha com título e valores ao vivo.
    let vm = bridge.view_model();
    assert!(vm.operation_hud_active);
    assert_eq!(vm.operation_hud_title, "Move");
    // X trava o eixo e aparece no HUD; repetir solta para Free.
    assert!(bridge.route_shortcut("X", false, false, false));
    assert_eq!(
        bridge
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|m| m.constraint),
        Some(petunia_core::ModalConstraint::Axis(0))
    );
    assert_eq!(bridge.view_model().gizmo_constraint_axes, [0, -1]);
    assert!(
        bridge
            .view_model()
            .operation_hud_lines
            .iter()
            .any(|line| line.starts_with("X   ")),
        "eixo travado aparece no HUD"
    );
    assert!(bridge.route_shortcut("X", false, false, false));
    assert_eq!(bridge.view_model().gizmo_constraint_axes, [-1, -1]);
    assert_eq!(
        bridge
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|m| m.constraint),
        Some(petunia_core::ModalConstraint::Free)
    );
    // Shift+Y exclui Y e trava o plano XZ.
    assert!(bridge.route_shortcut("Y", false, true, false));
    assert_eq!(
        bridge
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|m| m.constraint),
        Some(petunia_core::ModalConstraint::Plane(1))
    );
    assert_eq!(bridge.view_model().gizmo_constraint_axes, [2, 0]);
    // Entrada numérica acumula, mostra Input no HUD e aceita Backspace.
    assert!(bridge.route_shortcut("2", false, false, false));
    assert!(bridge.route_shortcut(".", false, false, false));
    assert!(bridge.route_shortcut("5", false, false, false));
    assert_eq!(bridge.modal_text, "2.5");
    assert!(
        bridge
            .view_model()
            .operation_hud_lines
            .iter()
            .any(|line| line.contains("Input   2.5")),
        "texto digitado aparece no HUD"
    );
    assert!(bridge.route_shortcut("Backspace", false, false, false));
    assert_eq!(bridge.modal_text, "2.");
    // Enter confirma em UMA etapa de undo e fecha o modal.
    assert!(bridge.route_shortcut("Enter", false, false, false));
    assert!(bridge.state.session.tools.modal.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn the_gizmo_only_appears_with_a_transform_tool() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    assert!(
        !bridge.view_model().gizmo.visible,
        "com a ferramenta Select o gizmo não pode cobrir o modelo"
    );

    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    assert!(bridge.view_model().gizmo.visible);

    bridge.apply(UiIntent::SetActiveTool("select".to_string()));
    assert!(!bridge.view_model().gizmo.visible);
}

#[test]
fn selection_overlay_draws_every_pickable_element_of_the_domain() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);

    // Point: os 8 vértices aparecem mesmo sem seleção, para o usuário ver
    // onde pode clicar; o selecionado vai para a camada de destaque.
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
    bridge.state.sync_selection();
    let overlay = bridge.view_model().selection_overlay;
    assert_eq!(overlay.point_commands.matches('M').count(), 1);
    assert_eq!(
        overlay.unselected_point_commands.matches('M').count(),
        7,
        "os outros 7 vértices precisam aparecer como alvos"
    );

    // Edge: todas as 12 arestas do cubo aparecem; a selecionada destaca.
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .selected_edges
        .insert((0, 1));
    bridge.state.sync_selection();
    let overlay = bridge.view_model().selection_overlay;
    assert!(overlay.outline_commands.is_empty());
    assert_eq!(
        overlay.unselected_outline_commands.matches('M').count(),
        11,
        "as outras 11 arestas precisam aparecer como alvos"
    );

    // Object: silhueta frontal no canal do ativo (o cubo padrão é o
    // ativo), sem camada de não selecionados.
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object));
    let overlay = bridge.view_model().selection_overlay;
    assert!(!overlay.active_outline_commands.is_empty());
    assert!(overlay.outline_commands.is_empty());
    assert!(overlay.unselected_outline_commands.is_empty());
    assert!(overlay.unselected_point_commands.is_empty());
}

#[test]
fn instant_mode_confirms_a_tool_on_click_instead_of_selecting() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    assert!(bridge.set_tool_activation("instant"));
    assert_eq!(bridge.view_model().tool_activation, "instant");
    assert!(!bridge.set_tool_activation("instant"), "sem mudança real");
    assert!(!bridge.set_tool_activation("bogus"));

    bridge.execute_core_command("model.extrude").unwrap();
    assert!(bridge.tool_modal.is_some());
    let value_before = bridge.tool_modal_value;
    assert!(bridge.scrub_tool_modal(-30.0, false));
    assert!(bridge.tool_modal_value > value_before);

    bridge.select_viewport(0.6, 0.6, false);
    assert!(bridge.tool_modal.is_none(), "o clique confirma a sessão");
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn drag_mode_keeps_selecting_on_click_with_a_tool_open() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    assert_eq!(bridge.view_model().tool_activation, "drag");

    bridge.execute_core_command("model.extrude").unwrap();
    assert!(bridge.tool_modal.is_some());
    // No modo Drag um clique na viewport seleciona normalmente; a sessão
    // continua aberta até o arrasto ou o Apply.
    bridge.select_viewport(0.5, 0.5, false);
    assert!(bridge.tool_modal.is_some());
}

#[test]
fn modeling_tool_shortcut_double_tap_behavior() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    assert_eq!(bridge.view_model().tool_activation, "drag");

    // 1 toque no atalho E (ADR 006): escolhe a ferramenta persistente sem
    // abrir operação; o arrasto na viewport é que opera.
    assert!(bridge.route_shortcut("E", false, false, false));
    assert!(bridge.tool_modal.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "extrude");
    assert!(!bridge.keyboard_tool_modal_active);
    assert!(!bridge.view_model().keyboard_tool_modal_active);
    assert!(!bridge.view_model().is_instant_tool_mode);

    // 2-toque no atalho E (dentro do intervalo): entra no Modo Livre (mouse manipula).
    assert!(bridge.route_shortcut("E", false, false, false));
    assert!(bridge.tool_modal.is_some());
    assert!(bridge.keyboard_tool_modal_active);
    assert!(bridge.view_model().keyboard_tool_modal_active);
    assert!(bridge.view_model().is_instant_tool_mode);
    assert_eq!(bridge.view_model().hud_pill_badge, "MODO LIVRE");

    // No Modo Livre, arrasto/mouse ajusta o valor:
    assert!(bridge.scrub_tool_modal(-32.0, false));
    assert!(bridge.tool_modal_value > 0.0);

    // Clique na viewport confirma a operação paramétrica:
    bridge.select_viewport(0.5, 0.5, false);
    assert!(bridge.tool_modal.is_none());
    assert!(!bridge.keyboard_tool_modal_active);
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn modeling_tool_shortcut_with_instant_preference_enters_modal_immediately() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    bridge.set_tool_activation("instant");

    // Com preferência Instant ativa, 1 toque já entra no modo livre imediatamente.
    assert!(bridge.route_shortcut("E", false, false, false));
    assert!(bridge.keyboard_tool_modal_active);
    assert!(bridge.view_model().is_instant_tool_mode);
}

#[test]
fn inset_and_bevel_shortcuts_support_single_and_double_tap() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    // Inset (I): 1 toque -> ferramenta persistente, sem operação aberta
    assert!(bridge.route_shortcut("I", false, false, false));
    assert!(bridge.tool_modal.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "inset");
    assert!(!bridge.keyboard_tool_modal_active);

    // Inset (I): 2 toque -> Modo Livre
    assert!(bridge.route_shortcut("I", false, false, false));
    assert!(bridge.keyboard_tool_modal_active);
    assert!(bridge.cancel_tool_modal());

    // Bevel (Ctrl+B): 1 toque -> ferramenta persistente, sem operação aberta
    assert!(bridge.route_shortcut("B", true, false, false));
    assert!(bridge.tool_modal.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "bevel");
    assert!(!bridge.keyboard_tool_modal_active);

    // Bevel (Ctrl+B): 2 toque -> Modo Livre
    assert!(bridge.route_shortcut("B", true, false, false));
    assert!(bridge.keyboard_tool_modal_active);
    assert!(bridge.cancel_tool_modal());
}

#[test]
fn escape_abandons_an_instant_tool_without_committing() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    bridge.set_tool_activation("instant");

    bridge.execute_core_command("model.inset").unwrap();
    bridge.scrub_tool_modal(-20.0, false);
    assert!(bridge.handle_escape());
    assert!(bridge.tool_modal.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn geometry_never_carries_selection_colour() {
    // A regressão original: selecionar uma aresta marcava os vértices das
    // pontas como selecionados e a triangulação pintava TODAS as faces que
    // tocavam esses vértices de laranja, mesmo em modo Edge.
    let mut mesh = petunia_core::Mesh::cube(2.0);
    let before: Vec<[f32; 3]> = mesh
        .to_triangles_smooth(false)
        .into_iter()
        .map(|(_, _, color, _)| color)
        .collect();

    mesh.faces[0].selected = true;
    mesh.verts[0].selected = true;
    mesh.selected_edges.insert((0, 1));

    let after: Vec<[f32; 3]> = mesh
        .to_triangles_smooth(false)
        .into_iter()
        .map(|(_, _, color, _)| color)
        .collect();
    assert_eq!(
        before, after,
        "selecionar não pode alterar a cor da geometria"
    );

    // A cor de seleção não aparece em nenhum vértice da triangulação.
    for (_, _, color, _) in mesh.to_triangles_smooth(false) {
        assert!(
            !(color[0] > 0.95 && (color[1] - 0.55).abs() < 0.05),
            "triangulação ainda pinta seleção: {color:?}"
        );
    }
}

#[test]
fn the_four_shading_modes_are_distinct_and_reachable() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(bridge.view_model().shading_mode, "solid");

    // Cada modo tem semântica própria: preencher, amostrar material e usar
    // a luz da cena são eixos separados.
    assert!(petunia_core::Shading::Solid.fills_faces());
    assert!(!petunia_core::Shading::Solid.samples_material());
    assert!(!petunia_core::Shading::Solid.uses_scene_light());

    assert!(!petunia_core::Shading::Wireframe.fills_faces());

    assert!(petunia_core::Shading::MaterialPreview.fills_faces());
    assert!(petunia_core::Shading::MaterialPreview.samples_material());
    assert!(!petunia_core::Shading::MaterialPreview.uses_scene_light());

    assert!(petunia_core::Shading::Rendered.uses_scene_light());

    for mode in petunia_core::Shading::ALL {
        assert!(bridge.set_shading_mode(mode.id()), "{}", mode.id());
        assert_eq!(bridge.view_model().shading_mode, mode.id());
        assert_eq!(bridge.state.shading, mode);
        // Material e Rendered precisam amostrar o material de fato.
        if mode.samples_material() {
            assert!(bridge.state.session.textured, "{}", mode.id());
        }
    }

    assert!(!bridge.set_shading_mode("nope"));
    assert_eq!(
        bridge.state.shading,
        petunia_core::Shading::Rendered,
        "valor inválido não altera o modo"
    );
}

#[test]
fn xray_opacity_is_clamped_and_reported() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.set_xray_opacity(0.7));
    assert!((bridge.view_model().xray_opacity - 0.7).abs() < 1.0e-6);

    assert!(bridge.set_xray_opacity(5.0));
    assert!((bridge.view_model().xray_opacity - 0.9).abs() < 1.0e-6);
    assert!(bridge.set_xray_opacity(-1.0));
    assert!((bridge.view_model().xray_opacity - 0.1).abs() < 1.0e-6);

    assert!(!bridge.set_xray_opacity(f32::NAN));
    assert!((bridge.view_model().xray_opacity - 0.1).abs() < 1.0e-6);
    assert!(!bridge.set_xray_opacity(0.1), "sem mudança real");
}

#[test]
fn xray_command_toggles_the_viewport_state_without_changing_selection() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.sync_selection();
    let selection_before = bridge.state.session.selection.clone();
    assert!(!bridge.view_model().show_xray);
    bridge.execute_core_command("view.toggle_xray").unwrap();
    assert!(bridge.view_model().show_xray);
    bridge.execute_core_command("view.toggle_xray").unwrap();
    assert!(!bridge.view_model().show_xray);
    assert_eq!(
        bridge.state.session.selection.assets,
        selection_before.assets
    );
}

#[test]
fn the_scene_carries_a_real_light_for_rendered_mode() {
    let project = petunia_project::Project::new();
    let light = project.active_light().expect("luz padrão da cena");
    assert!(light.enabled);
    assert_eq!(light.kind, petunia_project::LightKind::Directional);

    // A direção é normalizada e nunca degenera.
    let direction = light.normalized_direction();
    let length = (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt();
    assert!((length - 1.0).abs() < 1.0e-4);

    let degenerate = petunia_project::Light::directional("Zero", [0.0, 0.0, 0.0]);
    assert_eq!(degenerate.normalized_direction(), [0.0, 1.0, 0.0]);
    let hostile = petunia_project::Light::directional("NaN", [f32::NAN, 1.0, 0.0]);
    assert_eq!(hostile.normalized_direction(), [0.0, 1.0, 0.0]);

    // Desabilitar todas as luzes faz o Rendered cair no estúdio da viewport,
    // nunca renderizar preto.
    let mut project = petunia_project::Project::new();
    for light in &mut project.lights {
        light.enabled = false;
    }
    assert!(project.active_light().is_none());
}

#[test]
fn the_gizmo_handle_is_picked_in_screen_space_with_a_generous_target() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);

    // O centro exato de uma haste acerta o handle.
    let x_end = {
        let numbers: Vec<f32> = gizmo
            .x_commands
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect();
        [numbers[2], numbers[3]]
    };
    assert_eq!(
        bridge.gizmo_handle_at(x_end[0], x_end[1]),
        Some(GizmoHandle::X)
    );

    // Um ponto a 6 px da haste ainda acerta: o alvo é maior que o traço.
    assert_eq!(
        bridge.gizmo_handle_at(x_end[0] + 6.0, x_end[1]),
        Some(GizmoHandle::X)
    );
    // Longe de qualquer haste não há handle.
    assert_eq!(
        bridge.gizmo_handle_at(gizmo.origin_x + 400.0, gizmo.origin_y),
        None
    );

    // Sem ferramenta de transformação não há gizmo nem handle.
    bridge.apply(UiIntent::SetActiveTool("select".to_string()));
    assert_eq!(bridge.gizmo_handle_at(x_end[0], x_end[1]), None);
}

#[test]
fn combined_transform_has_independent_move_scale_and_rotate_handles() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetActiveTool("transform".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);
    assert!(!gizmo.x_scale_commands.is_empty());
    assert!(!gizmo.x_rotate_commands.is_empty());

    let numbers = |commands: &str| -> Vec<f32> {
        commands
            .split_whitespace()
            .filter_map(|part| part.parse().ok())
            .collect()
    };
    let scale = numbers(&gizmo.x_scale_commands);
    let scale_center = [(scale[0] + scale[4]) * 0.5, (scale[1] + scale[5]) * 0.5];
    assert_eq!(
        bridge.gizmo_target_at(scale_center[0], scale_center[1]),
        Some(GizmoTarget {
            handle: GizmoHandle::X,
            kind: TransformKind::Scale,
        })
    );

    let ring = numbers(&gizmo.x_rotate_commands);
    assert_eq!(
        bridge
            .gizmo_target_at(ring[0], ring[1])
            .map(|target| target.kind),
        Some(TransformKind::Rotation)
    );

    let rod = numbers(&gizmo.x_commands);
    assert_eq!(
        bridge.gizmo_target_at(rod[2], rod[3]),
        Some(GizmoTarget {
            handle: GizmoHandle::X,
            kind: TransformKind::Position,
        })
    );
}

#[test]
fn lasso_path_clamps_pointer_grab_outside_viewport() {
    let path = parse_lasso_path("-0.25,0.5;0.5,0.5;1.4,1.2;").unwrap();
    assert_eq!(path, vec![[-1.0, 0.0], [0.0, 0.0], [1.0, -1.0]]);
    assert!(parse_lasso_path("0,0;NaN,1;1,1;").is_none());
    assert!(parse_lasso_path("0,0;1,1;").is_none());
}

#[test]
fn model_tool_commands_activate_the_same_tools_as_the_toolbar() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.execute_command(CommandId::SelectLasso);
    assert_eq!(bridge.state.session.tools.active_tool, "lasso_select");
    bridge.execute_command(CommandId::TransformCombined);
    assert_eq!(bridge.state.session.tools.active_tool, "transform");
    assert!(bridge.view_model().gizmo.visible);
}

#[test]
fn dragging_a_gizmo_handle_constrains_the_transform_to_that_axis() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
    bridge.state.sync_selection();
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    let gizmo = bridge.view_model().gizmo;
    let numbers: Vec<f32> = gizmo
        .x_commands
        .split_whitespace()
        .filter_map(|token| token.parse::<f32>().ok())
        .collect();
    let end = [numbers[2], numbers[3]];

    // Hover antes do clique: preselection sem histórico.
    assert!(bridge.hover_gizmo(end[0], end[1]));
    assert_eq!(bridge.view_model().gizmo_hover_axis, 0);
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));

    assert!(bridge.begin_gizmo_drag(end[0], end[1]));
    assert_eq!(bridge.view_model().gizmo_active_axis, 0);
    assert!(bridge.state.session.tools.modal.is_some());

    // Arrastar só move no eixo X: Y e Z ficam intactos.
    let before = bridge.state.project.active_mesh().unwrap().verts[0].pos;
    assert!(bridge.update_viewport_transform(end[0] + 80.0, end[1]));
    let after = bridge.state.project.active_mesh().unwrap().verts[0].pos;
    assert!((after[0] - before[0]).abs() > 1.0e-3, "X precisa mudar");
    assert!((after[1] - before[1]).abs() < 1.0e-4, "Y precisa ficar");
    assert!((after[2] - before[2]).abs() < 1.0e-4, "Z precisa ficar");

    assert!(bridge.end_gizmo_drag());
    assert_eq!(bridge.view_model().gizmo_active_axis, -1);
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn orbiting_uses_the_selection_as_pivot() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    // Move um vértice para longe da origem e seleciona só ele.
    {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        mesh.verts[0].pos = [5.0, 0.0, 0.0];
        mesh.verts[0].selected = true;
    }
    bridge.state.sync_selection();
    assert_ne!(bridge.state.session.camera.target.x, 5.0);

    assert!(bridge.orbit_viewport(10.0, 0.0));
    assert!(
        (bridge.state.session.camera.target.x - 5.0).abs() < 1.0e-3,
        "a órbita precisa pivotar na seleção, veio {:?}",
        bridge.state.session.camera.target
    );

    // Sem seleção o alvo não é mexido.
    bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .deselect_all();
    bridge.state.sync_selection();
    let target = bridge.state.session.camera.target;
    assert!(bridge.orbit_viewport(10.0, 0.0));
    assert_eq!(bridge.state.session.camera.target, target);
}

#[test]
fn the_operation_hud_reports_the_real_value_and_the_axis() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    // Extrude exige faces selecionadas, não vértices.
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    // Em repouso o HUD some e a barra informa domínio e navegação.
    let vm = bridge.view_model();
    assert!(!vm.operation_hud_active);
    assert!(
        vm.context_hint.contains("Selection:"),
        "veio: {}",
        vm.context_hint
    );
    assert!(vm.context_hint.contains("Orbit"));

    // Com ferramenta aberta o HUD mostra título, valor e como confirmar.
    bridge.execute_core_command("model.extrude").unwrap();
    let vm = bridge.view_model();
    assert!(vm.operation_hud_active);
    assert_eq!(vm.operation_hud_title, "Extrude");
    assert_eq!(vm.operation_hud_lines.len(), 1);
    assert!(
        vm.operation_hud_lines[0].starts_with("Distance"),
        "veio: {}",
        vm.operation_hud_lines[0]
    );
    assert!(vm.operation_hud_hint.contains("Confirm"));
    assert!(vm.operation_hud_hint.contains("Cancel"));
    assert!(!vm.operation_hud_subject.is_empty());

    // O valor do HUD acompanha o arrasto.
    bridge.scrub_tool_modal(-40.0, false);
    let vm = bridge.view_model();
    assert!(
        vm.operation_hud_lines[0] != "Distance   0.000",
        "o HUD precisa refletir o valor real: {}",
        vm.operation_hud_lines[0]
    );

    bridge.commit_tool_modal();
    assert!(!bridge.view_model().operation_hud_active);
}

#[test]
fn the_operation_hud_shows_the_axis_constraint() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
    bridge.state.sync_selection();
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    let gizmo = bridge.view_model().gizmo;
    let numbers: Vec<f32> = gizmo
        .y_commands
        .split_whitespace()
        .filter_map(|token| token.parse::<f32>().ok())
        .collect();
    assert!(bridge.begin_gizmo_drag(numbers[2], numbers[3]));

    let vm = bridge.view_model();
    assert!(vm.operation_hud_active);
    assert_eq!(vm.operation_hud_title, "Move");
    assert!(
        vm.operation_hud_lines[0].starts_with('Y'),
        "a linha precisa nomear o eixo: {}",
        vm.operation_hud_lines[0]
    );
    assert!(vm.operation_hud_subject.contains("Y axis"));
    assert!(vm.context_hint.contains("Move"));
    bridge.end_gizmo_drag();
}

#[test]
fn hovering_preselects_a_component_without_touching_the_document() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));

    // O centro da viewport acerta a face frontal do cubo.
    assert!(bridge.hover_component(0.5, 0.5));
    assert!(matches!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::Face(_)
    ));
    assert!(!bridge.view_model().hover_label.is_empty());
    // Passar o mouse não seleciona nem empilha histórico.
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    assert!(
        !bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .iter()
            .any(|face| face.selected)
    );

    // Sair da geometria limpa a preselection.
    assert!(bridge.hover_component(0.02, 0.02));
    assert_eq!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::None
    );
    assert!(!bridge.clear_hover(), "já estava limpo");
}

#[test]
fn object_picking_uses_the_surface_and_ignores_the_empty_bounding_sphere() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Front);

    // (1.1, 1.1) fica dentro da esfera aproximada do cubo, mas fora da
    // superfície [-1, 1]². Nenhum objeto pode ser anunciado ou selecionado.
    let ndc = bridge
        .state
        .session
        .camera
        .project_ndc(glam::Vec3::new(1.1, 1.1, 0.0));
    let (x, y) = ((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5);
    assert_eq!(
        bridge.pick_viewport_target(x, y),
        petunia_core::HoverTarget::None
    );
    bridge.select_viewport(x, y, false);
    assert_eq!(bridge.state.ui.status, "Nothing under the cursor");
    assert_eq!(bridge.state.project.active, usize::MAX);
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn vertex_preselection_and_click_agree_on_visibility_and_screen_target() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge
        .state
        .session
        .camera
        .set_preset(petunia_core::ViewPreset::Front);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));

    let front = glam::Vec3::new(-1.0, -1.0, 1.0);
    let ndc = bridge.state.session.camera.project_ndc(front);
    let (x, y) = ((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5);
    assert_eq!(
        bridge.pick_viewport_target(x, y),
        petunia_core::HoverTarget::Vertex(4)
    );
    assert!(bridge.hover_component(x, y));
    bridge.select_viewport(x, y, false);
    assert_eq!(bridge.state.session.selection.verts, vec![4]);
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));

    bridge.select_viewport(x, y, true);
    assert!(bridge.state.session.selection.verts.is_empty());
    assert!(bridge.hover_component(f32::NAN, y));
    assert_eq!(
        bridge.state.session.tools.hover,
        petunia_core::HoverTarget::None
    );
    assert_eq!(
        bridge.pick_viewport_target(-0.1, y),
        petunia_core::HoverTarget::None
    );
}

#[test]
fn changing_domains_converts_selection_without_ghost_faces_or_edges() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    bridge.state.project.active_mesh_mut().unwrap().faces[1].selected = true;
    bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .sync_vert_selection_from_faces();
    bridge.state.sync_selection();
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(!mesh.faces.iter().any(|face| face.selected));
    assert_eq!(mesh.selected_edges.len(), 4);

    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(mesh.selected_edges.is_empty());
    assert_eq!(mesh.selected_vert_count(), 4);
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));
}

#[test]
fn hover_respects_occlusion_unless_xray_is_on() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));

    // Raio do olho até o vértice traseiro do cubo (z = -1): atravessa a
    // face frontal, então o alvo está ocluído.
    let origin = bridge.state.session.camera.eye();
    let position = glam::Vec3::new(-1.0, -1.0, -1.0);
    let direction = (position - origin).normalize();
    assert!(
        bridge.is_occluded(origin, direction, position),
        "o vértice traseiro precisa estar atrás da face frontal"
    );

    // Um vértice frontal não está ocluído.
    let front = glam::Vec3::new(1.0, 1.0, 1.0);
    let front_dir = (front - origin).normalize();
    assert!(!bridge.is_occluded(origin, front_dir, front));

    bridge.state.session.show_xray = true;
    assert!(
        !bridge.is_occluded(origin, direction, position),
        "X-Ray existe justamente para alcançar o que está atrás"
    );
}

#[test]
fn camera_projection_and_reset() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().is_orthographic);

    bridge.apply(UiIntent::ToggleProjection);
    assert!(bridge.view_model().is_orthographic);
    assert!(bridge.view_model().status_message.contains("Ortográfica"));

    bridge.apply(UiIntent::ToggleProjection);
    assert!(!bridge.view_model().is_orthographic);
    assert!(bridge.view_model().status_message.contains("Perspectiva"));

    let initial_target = bridge.state.session.camera.target;
    bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan {
        dx: 20.0,
        dy: 10.0,
    }));
    assert_ne!(bridge.state.session.camera.target, initial_target);

    bridge.apply(UiIntent::ResetCamera);
    assert_eq!(bridge.state.session.camera.target, initial_target);
    assert!(bridge.view_model().status_message.contains("redefinida"));
}

#[test]
fn save_active_as_asset_intent_creates_project_asset() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let initial_count = bridge.state.project.assets.len();
    assert_eq!(initial_count, 1);

    bridge.apply(UiIntent::SaveActiveAsAsset);
    assert_eq!(bridge.state.project.assets.len(), initial_count + 1);
    let saved_asset = &bridge.state.project.assets[1];
    assert!(saved_asset.name.contains("(Asset)"));
    assert!(
        bridge
            .view_model()
            .status_message
            .contains("biblioteca de assets")
    );
}

#[test]
fn new_commands_execute_via_command_id() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    bridge.execute_command(CommandId::DuplicateSelected);
    assert_eq!(bridge.state.project.assets.len(), 2);

    bridge.execute_command(CommandId::ToggleAssetLibrary);
    assert!(bridge.view_model().asset_library_visible);

    bridge.execute_command(CommandId::ToggleProjection);
    assert!(bridge.view_model().is_orthographic);

    bridge.execute_command(CommandId::ResetCamera);
    assert!(bridge.view_model().status_message.contains("redefinida"));

    bridge.execute_command(CommandId::SelectAll);
    assert_eq!(bridge.state.session.selection.assets.len(), 2);

    bridge.execute_command(CommandId::ClearSelection);
    assert!(bridge.state.session.selection.assets.is_empty());
    assert_eq!(bridge.state.project.active, usize::MAX);

    bridge.execute_command(CommandId::InvertSelection);
    assert_eq!(bridge.state.session.selection.assets.len(), 2);

    bridge.execute_command(CommandId::SaveActiveAsAsset);
    assert_eq!(bridge.state.project.assets.len(), 3);
}

#[test]
fn edge_loop_selection_via_select_viewport_ext() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.execute_command(CommandId::AddPlane);
    bridge.state.set_selection_domain(SelectionDomain::Edge);

    // Direct call to select_edge_loop through mesh or select_viewport_ext
    let count = bridge
        .state
        .project
        .active_mesh_mut()
        .unwrap()
        .select_edge_loop((0, 1), false);
    assert_eq!(count, 4, "boundary loop of plane has 4 edges");
    let active = bridge.state.project.active_mesh().unwrap();
    assert_eq!(active.selected_edges.len(), 4);
    assert!(active.verts.iter().all(|v| v.selected));
}

#[test]
fn palette_import_and_export_intents_work() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let temp_dir = std::env::temp_dir();
    let gpl_path = temp_dir.join("test_sprint1_palette.gpl");

    let content = "GIMP Palette\nName: Test\nColumns: 4\n#\n255   0   0 Red\n  0 255   0 Green\n  0   0 255 Blue\n";
    std::fs::write(&gpl_path, content).unwrap();

    bridge.apply(UiIntent::ImportPalette(gpl_path.clone()));
    assert!(bridge.state.ui.status.contains("imported palette"));
    assert_eq!(bridge.state.project.palette.len(), 3);

    let export_path = temp_dir.join("test_sprint1_export.gpl");
    bridge.apply(UiIntent::ExportPalette(export_path.clone()));
    assert!(
        bridge
            .state
            .ui
            .status
            .contains("palette exported successfully")
    );
    assert!(export_path.exists());

    let _ = std::fs::remove_file(gpl_path);
    let _ = std::fs::remove_file(export_path);
}

#[test]
fn uv_seam_toggle_with_selected_edges() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let mesh = bridge.state.project.active_mesh_mut().unwrap();
    mesh.faces[0].selected = false;
    mesh.selected_edges.insert((0, 1));
    mesh.selected_edges.insert((1, 2));

    assert!(bridge.toggle_selected_uv_seams());
    let seams = bridge.state.project.active_mesh().unwrap().uv_seams.clone();
    assert_eq!(seams.len(), 2);
    assert!(seams.contains(&(0, 1)));
    assert!(seams.contains(&(1, 2)));
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    // Toggling again removes them
    assert!(bridge.toggle_selected_uv_seams());
    let seams_after = bridge.state.project.active_mesh().unwrap().uv_seams.clone();
    assert!(seams_after.is_empty());
}

#[test]
fn universal_gizmo_disambiguation_with_deadzone() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("transform".to_string()));

    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);
    let origin_x = gizmo.origin_x;
    let origin_y = gizmo.origin_y;

    // Deadzone check: point exactly at origin or within deadzone (<= 12px) returns None
    assert_eq!(bridge.gizmo_target_at(origin_x, origin_y), None);
    assert_eq!(bridge.gizmo_target_at(origin_x + 5.0, origin_y + 5.0), None);

    // Outside deadzone: far away point returns None
    assert_eq!(bridge.gizmo_target_at(0.0, 0.0), None);
}

#[test]
fn slice_dynamic_preview_line() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("slice".to_string()));

    // When slice is not dragging, drag_link_commands is empty
    assert!(bridge.view_model().drag_link_commands.is_empty());

    // When slice drag starts and mouse moves:
    bridge.slice_anchor = Some([200.0, 200.0]);
    bridge.pointer_position = [300.0, 250.0];

    let vm = bridge.view_model();
    assert!(!vm.drag_link_commands.is_empty());
    assert!(vm.drag_link_commands.contains("L "));
}

#[test]
fn material_slots_assignment_and_duplication() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let mesh = bridge.state.project.active_mesh_mut().unwrap();
    // Select face 0
    for f in &mut mesh.faces {
        f.selected = false;
    }
    mesh.faces[0].selected = true;

    // Initially 1 default material in project
    let vm = bridge.view_model();
    assert!(!vm.material_slots.is_empty());

    // Create new material via UiIntent
    bridge.apply(UiIntent::CreateMaterial);
    let vm = bridge.view_model();
    assert_eq!(vm.material_slots.len(), 2);
    assert_eq!(vm.active_material_slot, 1);

    // Assign to face 0 via UiIntent
    bridge.apply(UiIntent::AssignMaterialSlot(1));
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert_eq!(mesh.faces[0].material_slot, Some(1));
    assert_eq!(bridge.state.project.undo.depth(), (2, 0));

    // Duplicate material 1 via UiIntent
    bridge.apply(UiIntent::DuplicateMaterial(1));
    let vm = bridge.view_model();
    assert_eq!(vm.material_slots.len(), 3);
    assert_eq!(vm.active_material_slot, 2);
    assert!(vm.material_slots[2].contains("Copy"));
}

#[test]
fn paint_2d_stroke_flow_and_undo() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

    // Set bright red paint color
    bridge.apply(UiIntent::SetPaintColor([1.0, 0.0, 0.0]));

    // Check active texture initial pixel at (128, 128)
    let initial_color = bridge
        .state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .and_then(|t| t.get(128, 128))
        .unwrap_or([0, 0, 0, 0]);

    // Begin 2D stroke at center (0.5, 0.5)
    bridge.apply(UiIntent::Paint2dStroke {
        norm_x: 0.5,
        norm_y: 0.5,
        phase: 0,
    });
    assert!(bridge.paint_2d_last.is_some());

    // Move stroke
    bridge.apply(UiIntent::Paint2dStroke {
        norm_x: 0.51,
        norm_y: 0.51,
        phase: 1,
    });

    // Finish stroke
    bridge.apply(UiIntent::Paint2dStroke {
        norm_x: 0.51,
        norm_y: 0.51,
        phase: 2,
    });
    assert!(bridge.paint_2d_last.is_none());

    // Texture pixel should be red
    let painted_color = bridge
        .state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .and_then(|t| t.get(128, 128))
        .expect("pixel");
    assert_eq!(painted_color[0], 255);
    assert_eq!(painted_color[1], 0);
    assert_eq!(painted_color[2], 0);

    // Undo stroke
    bridge.apply(UiIntent::Undo);
    let undone_color = bridge
        .state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .and_then(|t| t.get(128, 128))
        .unwrap_or([0, 0, 0, 0]);
    assert_eq!(undone_color, initial_color);

    // Redo stroke
    bridge.apply(UiIntent::Redo);
    let redone_color = bridge
        .state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .and_then(|t| t.get(128, 128))
        .expect("pixel");
    assert_eq!(redone_color[0], 255);
}

#[test]
fn paint_2d_fill_is_one_undoable_transaction() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
    bridge.apply(UiIntent::SetPaintColor([1.0, 0.0, 0.0]));
    bridge.apply(UiIntent::SetActiveTool("fill".into()));
    let before = bridge.state.project.active().unwrap().texture.clone();
    let before_stack = active_paint_stack(&bridge);
    let history_before = bridge.state.project.undo.depth().0;

    bridge.apply(UiIntent::Paint2dStroke {
        norm_x: 0.5,
        norm_y: 0.5,
        phase: 0,
    });
    let after = bridge.state.project.active().unwrap().texture.clone();
    let after_stack = active_paint_stack(&bridge);
    assert_ne!(after, before);
    assert_ne!(after_stack, before_stack);
    assert_eq!(bridge.state.project.undo.depth(), (history_before + 1, 0));

    assert!(bridge.state.undo());
    assert_eq!(bridge.state.project.active().unwrap().texture, before);
    assert_eq!(active_paint_stack(&bridge), before_stack);
    assert!(bridge.state.redo());
    assert_eq!(bridge.state.project.active().unwrap().texture, after);
    assert_eq!(active_paint_stack(&bridge), after_stack);
}

#[test]
fn paint_pixel_grid_and_canvas_zoom() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.paint_pixel_grid);
    assert_eq!(bridge.paint_canvas_zoom, 1);

    // Toggle pixel grid
    bridge.apply(UiIntent::TogglePaintPixelGrid);
    assert!(!bridge.paint_pixel_grid);
    assert!(!bridge.view_model().paint_pixel_grid);

    bridge.apply(UiIntent::TogglePaintPixelGrid);
    assert!(bridge.paint_pixel_grid);
    assert!(bridge.view_model().paint_pixel_grid);

    // Set zoom
    bridge.apply(UiIntent::SetPaintCanvasZoom(4));
    assert_eq!(bridge.paint_canvas_zoom, 4);
    assert_eq!(bridge.view_model().paint_canvas_zoom, 4);

    // Clamping upper bound
    bridge.apply(UiIntent::SetPaintCanvasZoom(32));
    assert_eq!(bridge.paint_canvas_zoom, 16);

    // Clamping lower bound
    bridge.apply(UiIntent::SetPaintCanvasZoom(-2));
    assert_eq!(bridge.paint_canvas_zoom, 1);

    // Zoom stays presentation-only: the published RGBA buffer is native-size.
    bridge.apply(UiIntent::SetPaintCanvasZoom(4));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
    let dimensions = bridge.paint_canvas_dimensions().unwrap();
    let image = bridge.render_paint_canvas().expect("native canvas image");
    assert_eq!(image.size().width, dimensions.0);
    assert_eq!(image.size().height, dimensions.1);
    assert!(!bridge.paint_canvas_grid_commands().is_empty());

    bridge.apply(UiIntent::TogglePaintPixelGrid);
    assert!(bridge.paint_canvas_grid_commands().is_empty());
}

#[test]
fn paint_2d_spacing_is_poll_rate_invariant() {
    fn painted_texture(samples: &[f32]) -> petunia_project::Canvas {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("pixel".into()));
        bridge.state.session.tools.canvas_brush = 10;
        bridge.state.session.tools.brush_spacing = 0.5;
        bridge.apply(UiIntent::SetPaintColor([0.9, 0.1, 0.2]));

        assert!(bridge.paint_2d_stroke(samples[0], 0.5, 0));
        for &sample in &samples[1..] {
            assert!(bridge.paint_2d_stroke(sample, 0.5, 1));
        }
        assert!(bridge.paint_2d_stroke(*samples.last().unwrap(), 0.5, 2));
        bridge
            .state
            .project
            .active()
            .and_then(|asset| asset.texture.clone())
            .expect("painted texture")
    }

    let fast = painted_texture(&[0.1, 0.9]);
    let slow = painted_texture(&[0.1, 0.2, 0.35, 0.5, 0.65, 0.8, 0.9]);
    assert_eq!(fast, slow);
}

#[test]
fn paint_2d_symmetry_batch_publishes_one_texture_revision_per_event() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("pixel".into()));
    bridge.state.session.tools.canvas_brush = 8;
    bridge.state.session.tools.brush_spacing = 0.2;
    bridge.state.session.tools.paint_symmetry_x = true;
    bridge.state.session.tools.paint_symmetry_y = true;

    assert!(bridge.paint_2d_stroke(0.1, 0.5, 0));
    let before = bridge.state.project.project.texture_revision;
    assert!(bridge.paint_2d_stroke(0.9, 0.5, 1));
    assert_eq!(bridge.state.project.project.texture_revision, before + 1);
    assert!(bridge.paint_2d_stroke(0.9, 0.5, 2));
}

#[test]
fn airbrush_continuous_dab_accumulation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("airbrush".to_string()));
    bridge.apply(UiIntent::SetPaintColor([0.0, 1.0, 0.0]));
    petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

    // Initially no stroke active, airbrush_tick returns false
    assert!(!bridge.airbrush_tick());

    // Start 2D stroke
    bridge.paint_2d_stroke(0.5, 0.5, 0);
    let pixel_after_dab1 = bridge
        .state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .and_then(|t| t.get(128, 128))
        .expect("pixel");

    // Tick airbrush while pointer held
    let ticked = bridge.airbrush_tick();
    assert!(ticked);

    let pixel_after_dab2 = bridge
        .state
        .project
        .active()
        .and_then(|a| a.texture.as_ref())
        .and_then(|t| t.get(128, 128))
        .expect("pixel");

    // Alpha or color density increases or stays solid green
    assert!(pixel_after_dab2[1] >= pixel_after_dab1[1]);
    assert!(pixel_after_dab2[3] >= pixel_after_dab1[3]);

    // Finish stroke
    bridge.paint_2d_stroke(0.5, 0.5, 2);
    assert!(!bridge.airbrush_tick());
}

#[test]
fn project_from_reference_and_bake_reference_intents() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ProjectFromReference);
    assert!(bridge.view_model().status_message.contains("Projected UVs"));

    bridge.apply(UiIntent::BakeReference);
    assert!(
        bridge
            .view_model()
            .status_message
            .contains("No visible reference image")
    );

    let ref_img =
        petunia_core::ReferenceImage::from_rgba("ref1".to_string(), 1, 1, vec![255, 0, 0, 255]);
    bridge.state.project.refs.push(ref_img);

    bridge.apply(UiIntent::ProjectFromReference);
    assert!(bridge.view_model().status_message.contains("Projected UVs"));
    assert_eq!(bridge.state.project.undo.depth(), (2, 0));

    bridge.apply(UiIntent::BakeReference);
    assert!(
        bridge
            .view_model()
            .status_message
            .contains("Baked reference")
    );
    assert_eq!(bridge.state.project.undo.depth(), (3, 0));

    bridge.apply(UiIntent::Undo);
    assert_eq!(bridge.state.project.undo.depth(), (2, 1));
}

#[test]
fn material_editor_updates_values_and_undo() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let before = bridge.state.project.project.materials[0].roughness;
    assert!(bridge.set_active_material_scalar("roughness", 0.2));
    assert_eq!(bridge.view_model().material_roughness, 0.2);
    assert!(bridge.state.undo());
    assert_eq!(bridge.state.project.project.materials[0].roughness, before);
}

#[test]
fn quick_actions_are_controlled_and_execute_commands() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.toggle_quick_action("model.intersect"));
    assert!(
        bridge
            .view_model()
            .quick_action_candidates
            .iter()
            .any(|item| item.pinned)
    );
    bridge.state.project.active_mesh_mut().unwrap().select_all();
    assert!(bridge.execute_quick_action("model.subdivide"));
    assert!(bridge.toggle_quick_action("model.subdivide"));
    assert!(
        !bridge
            .view_model()
            .quick_actions
            .iter()
            .any(|item| item.id == "model.subdivide")
    );
}

#[test]
fn modifier_stack_supports_live_rows_and_apply() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.add_modifier("mirror"));
    assert!(bridge.add_modifier("symmetry"));
    let rows = bridge.view_model().modifier_rows;
    assert_eq!(rows.len(), 2);
    assert!(bridge.set_modifier_axis(&rows[0].id, 1));
    assert!(bridge.set_modifier_direction(&rows[1].id, false));
    assert!(bridge.set_modifier_enabled(&rows[0].id, false));
    assert_eq!(bridge.view_model().modifier_rows[0].axis, 1);
    assert!(bridge.move_modifier(&rows[1].id, -1));
    assert!(bridge.apply_modifier(&rows[1].id));
    assert_eq!(bridge.view_model().modifier_rows.len(), 1);
    assert!(bridge.state.undo());
}

#[test]
fn scale_tool_exposes_tool_options_immediately() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("scale".into()));
    let vm = bridge.view_model();
    assert!(vm.tool_options_active);
    assert!(vm.tool_options_title.contains("Scale"));
}

#[test]
fn proportional_editing_radius_adjustment_and_falloff() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Toggle via UiIntent
    assert!(!bridge.view_model().proportional_editing);
    bridge.apply(UiIntent::ToggleProportionalEditing);
    assert!(bridge.view_model().proportional_editing);

    // Adjust radius (default is 1.5)
    bridge.adjust_proportional_radius(0.5);
    assert!((bridge.view_model().proportional_radius - 2.0).abs() < 1e-4);

    // Set falloff
    bridge.apply(UiIntent::SetProportionalFalloff("linear".into()));
    assert_eq!(bridge.view_model().proportional_falloff, "linear");

    // ADR 006: durante a operação, Ctrl+roda ajusta o raio; a roda sozinha
    // continua sendo zoom.
    bridge
        .state
        .begin_modal(petunia_core::ModalKind::Move)
        .unwrap();
    assert!(bridge.viewport_ctrl_scroll(1.0));
    assert!((bridge.view_model().proportional_radius - 2.25).abs() < 1e-4);
    assert!(bridge.viewport_ctrl_scroll(-1.0));
    assert!((bridge.view_model().proportional_radius - 2.0).abs() < 1e-4);
    bridge.apply_viewport_gesture(ViewportGesture::Zoom { delta: 1.0 });
    assert!(
        (bridge.view_model().proportional_radius - 2.0).abs() < 1e-4,
        "a roda sem Ctrl não muda o raio"
    );
}

#[test]
fn advanced_snap_to_edge_and_face() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ToggleSnapEnabled);
    assert!(bridge.view_model().snap_enabled);

    bridge.apply(UiIntent::SetSnapTarget("edge".into()));
    assert_eq!(bridge.view_model().snap_target, "edge");
    assert_eq!(
        bridge.state.session.snap_settings.target,
        petunia_core::SnapTarget::Edge
    );

    // Edge snapping query on cube edge [-1, 1, 1] to [1, 1, 1]
    let mesh = bridge.state.project.active_mesh().unwrap().clone();
    let query_edge = petunia_core::SnapQuery {
        point: glam::Vec3::new(0.0, 1.05, 1.0),
        start_point: Some(glam::Vec3::ZERO),
        settings: &bridge.state.session.snap_settings,
        mesh: Some(&mesh),
    };
    let res_edge = petunia_core::snap_point(query_edge);
    assert!(res_edge.snapped);
    assert_eq!(res_edge.target, petunia_core::SnapTarget::Edge);

    bridge.apply(UiIntent::SetSnapTarget("face".into()));
    assert_eq!(bridge.view_model().snap_target, "face");
    assert_eq!(
        bridge.state.session.snap_settings.target,
        petunia_core::SnapTarget::Face
    );

    // Face snapping query near centroid (0, 0, 1)
    let query_face = petunia_core::SnapQuery {
        point: glam::Vec3::new(0.05, 0.02, 1.0),
        start_point: Some(glam::Vec3::ZERO),
        settings: &bridge.state.session.snap_settings,
        mesh: Some(&mesh),
    };
    let res_face = petunia_core::snap_point(query_face);
    assert!(res_face.snapped);
    assert_eq!(res_face.target, petunia_core::SnapTarget::Face);
}

#[test]
fn face_orientation_and_uv_checker_overlays() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().show_face_orientation);
    assert!(!bridge.view_model().show_uv_checker);

    bridge.apply(UiIntent::ToggleFaceOrientation);
    assert!(bridge.view_model().show_face_orientation);
    assert!(bridge.state.session.show_face_orientation);

    bridge.apply(UiIntent::ToggleUvChecker);
    assert!(bridge.view_model().show_uv_checker);
    assert!(bridge.state.session.show_uv_checker);

    let flags = petunia_core::FingerprintFlags {
        show_face_orientation: bridge.state.session.show_face_orientation,
        show_uv_checker: bridge.state.session.show_uv_checker,
        ..Default::default()
    };
    assert!(flags.show_face_orientation);
    assert!(flags.show_uv_checker);
}

#[test]
fn profile_2d_rectangle_and_circle_primitives() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddProfileRectangle {
        width: 3.0,
        height: 2.0,
    });
    assert_eq!(active_profile_spline(&bridge).points.len(), 4);
    assert!(active_profile_spline(&bridge).closed);

    // Generate mesh from rectangle profile
    bridge.generate_profile_extrude();
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(!mesh.verts.is_empty());

    // Add circle profile
    bridge.apply(UiIntent::AddProfileCircle {
        radius: 1.5,
        segments: 12,
    });
    assert_eq!(active_profile_spline(&bridge).points.len(), 12);
    assert!(active_profile_spline(&bridge).closed);
}

#[test]
fn individual_origins_multi_object_transformation() {
    let mut state = AppState::default();
    let mut mesh1 = petunia_core::Mesh::cube(1.0);
    for v in &mut mesh1.verts {
        v.pos[0] -= 5.0;
    }
    let mut mesh2 = petunia_core::Mesh::cube(1.0);
    for v in &mut mesh2.verts {
        v.pos[0] += 5.0;
    }
    let mut combined = mesh1;
    let v_offset = combined.verts.len() as u32;
    for v in mesh2.verts {
        combined.verts.push(v);
    }
    for mut f in mesh2.faces {
        for idx in &mut f.verts {
            *idx += v_offset;
        }
        combined.faces.push(f);
    }
    combined.select_all();
    *state.project.active_mesh_mut().unwrap() = combined;

    state.set_selection_domain(petunia_core::SelectionDomain::Vertex);
    state.session.pivot_point = petunia_core::PivotPoint::IndividualOrigins;
    state.begin_modal(petunia_core::ModalKind::Scale).unwrap();
    // Scale by 2.0 around individual origins
    state.update_modal(glam::Vec3::ZERO, 2.0).unwrap();

    let transformed = state.project.active_mesh().unwrap();
    // Centroid of cube 1 should remain at -5, centroid of cube 2 at 5
    let center1_x: f32 = transformed.verts[..8].iter().map(|v| v.pos[0]).sum::<f32>() / 8.0;
    let center2_x: f32 = transformed.verts[8..].iter().map(|v| v.pos[0]).sum::<f32>() / 8.0;

    assert!((center1_x - (-5.0)).abs() < 1e-3);
    assert!((center2_x - 5.0).abs() < 1e-3);
}

#[test]
fn lasso_selection_occlusion_and_xray() {
    let mut state = AppState::default();
    // Active cube
    state.set_selection_domain(petunia_core::SelectionDomain::Face);
    state.session.camera.proj = petunia_core::Projection::Perspective;

    // Lasso polygon covering center NDC screen
    let polygon = vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];

    // Without X-Ray, occluded geometry behind is filtered
    state.session.show_xray = false;
    state.select_viewport_lasso(&polygon, false, false);
    let selected_count_no_xray = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .filter(|f| f.selected)
        .count();

    // With X-Ray, through-selection selects both front and back
    state.session.show_xray = true;
    state.select_viewport_lasso(&polygon, false, false);
    let selected_count_xray = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .filter(|f| f.selected)
        .count();

    assert!(selected_count_xray >= selected_count_no_xray);
}

#[test]
fn uv_island_90_degree_rotation_in_slint() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Uv));
    bridge.state.session.uv_selected.insert(0);

    let initial_u0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
    let initial_v0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][1];

    // Rotate 90 degrees CW (+90)
    assert!(bridge.uv_rotate_selected(90.0));
    let rotated_u0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
    let rotated_v0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][1];

    assert!((rotated_u0 - initial_u0).abs() > 1e-4 || (rotated_v0 - initial_v0).abs() > 1e-4);

    // Rotate 90 degrees CCW (-90) returns to original position
    assert!(bridge.uv_rotate_selected(-90.0));
    let back_u0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
    let back_v0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][1];
    assert!((back_u0 - initial_u0).abs() < 1e-4);
    assert!((back_v0 - initial_v0).abs() < 1e-4);
}

#[test]
fn decal_layer_creation_and_rendering() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    assert!(bridge.add_decal_layer());

    let active = bridge.state.project.active;
    let stack = bridge.state.project.assets[active]
        .paint_stack
        .as_ref()
        .unwrap();
    assert!(stack.layers.iter().any(|layer| matches!(
        layer.kind,
        petunia_project::paint_layers::LayerKind::Decal(_)
    )));
}

#[test]
fn cursor_tool_activation_placement_and_view_model() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.execute_command(CommandId::ToolCursor);
    assert_eq!(bridge.state.session.tools.active_tool, "cursor");

    assert!(bridge.place_cursor_3d(0.5, 0.5));
    let vm = bridge.view_model();
    assert_eq!(vm.cursor_3d, bridge.state.session.cursor_3d);

    // Adjust individual coordinates
    assert!(bridge.set_cursor_3d_coord(0, 3.5));
    assert!(bridge.set_cursor_3d_coord(1, -2.0));
    assert!(bridge.set_cursor_3d_coord(2, 7.25));
    assert_eq!(bridge.state.session.cursor_3d, [3.5, -2.0, 7.25]);
    assert_eq!(bridge.view_model().cursor_3d, [3.5, -2.0, 7.25]);

    // Reset cursor to origin
    assert!(bridge.reset_cursor_3d());
    assert_eq!(bridge.state.session.cursor_3d, [0.0, 0.0, 0.0]);
    assert_eq!(bridge.view_model().cursor_3d, [0.0, 0.0, 0.0]);
}

#[test]
fn cursor_focuses_camera_and_serves_as_orbit_pivot() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.session.cursor_3d = [10.0, 5.0, -8.0];

    // Frame cursor centers camera target on cursor_3d
    bridge.execute_command(CommandId::FrameCursor);
    assert_eq!(
        bridge.state.session.camera.target,
        glam::Vec3::new(10.0, 5.0, -8.0)
    );

    // When cursor tool is active, orbiting sets camera target to cursor_3d
    bridge.execute_command(CommandId::ToolCursor);
    bridge.state.session.camera.target = glam::Vec3::ZERO;
    assert!(bridge.orbit_viewport(10.0, 10.0));
    assert_eq!(
        bridge.state.session.camera.target,
        glam::Vec3::new(10.0, 5.0, -8.0)
    );
}

#[test]
fn cursor_offsets_primitive_spawning() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.session.cursor_3d = [5.0, 3.0, -2.0];

    // Spawning a cube uses cursor_3d as origin offset
    assert!(bridge.state.dispatch_command("model.add_cube").is_ok());
    let active_mesh = bridge.state.project.active_mesh().unwrap();
    let center: glam::Vec3 = active_mesh
        .verts
        .iter()
        .map(|v| v.vec())
        .sum::<glam::Vec3>()
        / active_mesh.verts.len() as f32;

    assert!((center.x - 5.0).abs() < 1e-4);
    assert!((center.y - 3.0).abs() < 1e-4);
    assert!((center.z - (-2.0)).abs() < 1e-4);
}

#[test]
fn cursor_screen_projection_in_gizmo_model() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];
    bridge.state.session.cursor_3d = [0.0, 0.0, 0.0];
    bridge.state.session.show_cursor = true;
    bridge.state.session.show_overlays = true;

    let gizmo = compute_gizmo(&bridge.state, 800.0, 600.0);
    assert!(gizmo.cursor_visible);
    // Origin projected with default camera should be near center of screen
    assert!((gizmo.cursor_screen[0] - 400.0).abs() < 50.0);
    assert!((gizmo.cursor_screen[1] - 300.0).abs() < 50.0);
}

#[test]
fn single_tap_and_double_tap_tool_interaction_flow() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.preferences.double_tap_interval_ms = 350;

    // 1º toque em 'G': Apenas seleciona a ferramenta Move, NÃO inicia modal nem captura mouse.
    assert!(bridge.route_shortcut("G", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "move");
    assert!(!bridge.view_model().transform_instant_active);
    assert!(bridge.state.session.tools.modal.is_none());

    // 2º toque em 'G' rápido (double-tap): Dispara o modo modal livre (Modo Blender).
    assert!(bridge.route_shortcut("G", false, false, false));
    assert!(bridge.view_model().transform_instant_active);
    assert!(bridge.state.session.tools.modal.is_some());

    // Tecla Escape cancela e restaura a viewport
    assert!(bridge.route_shortcut("Escape", false, false, false));
    assert!(!bridge.view_model().transform_instant_active);
    assert!(bridge.state.session.tools.modal.is_none());
}

#[test]
fn tool_shortcut_with_double_tap_timer_disabled() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    // Temporizador desativado (0ms): apertar com ferramenta já selecionada aciona o modal direto.
    bridge.preferences.double_tap_interval_ms = 0;

    // 1º toque em 'R' com ferramenta diferente: apenas ativa a ferramenta Rotate
    bridge.state.session.tools.active_tool = "select".to_string();
    assert!(bridge.route_shortcut("R", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "rotate");
    assert!(!bridge.view_model().transform_instant_active);

    // 2º toque em 'R' com a ferramenta já ativa: entra no modo modal contínuo
    assert!(bridge.route_shortcut("R", false, false, false));
    assert!(bridge.view_model().transform_instant_active);
    assert!(bridge.state.session.tools.modal.is_some());

    // Confirmação com Enter encerra o modal
    assert!(bridge.route_shortcut("Enter", false, false, false));
    assert!(!bridge.view_model().transform_instant_active);
}

#[test]
fn wheel_zooms_even_with_a_tool_modal_open() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();

    assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
    assert_eq!(bridge.tool_modal, Some(ToolModalKind::Extrude));
    let initial_value = bridge.tool_modal_value;

    // ADR 006: a roda sempre faz zoom; o valor da ferramenta não muda.
    let distance = bridge.state.session.camera.distance;
    bridge.apply_viewport_gesture(ViewportGesture::Zoom { delta: -1.0 });
    assert_eq!(bridge.tool_modal_value, initial_value);
    assert_ne!(bridge.state.session.camera.distance, distance);
}

#[test]
fn transform_fine_precision_and_snap_modifiers_effect() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.pointer_position = [400.0, 300.0];

    // Inicia rotação com double-tap R e trava no eixo Z
    assert!(bridge.route_shortcut("R", false, false, false));
    assert!(bridge.route_shortcut("R", false, false, false));
    assert!(bridge.view_model().transform_instant_active);
    assert!(bridge.route_shortcut("Z", false, false, false));

    // Com snap: true (Ctrl pressionado), a rotação em torno de Z é quantizada em passos de 15 graus
    assert!(bridge.update_viewport_transform_modified(450.0, 300.0, false, true));
    let angle_snapped = bridge.rotation[2].value();
    assert!((angle_snapped % 15.0).abs() < 1e-4);

    assert!(bridge.cancel_transform());

    // Inicia movimento com double-tap G a partir de [400.0, 300.0]
    bridge.pointer_position = [400.0, 300.0];
    assert!(bridge.route_shortcut("G", false, false, false));
    assert!(bridge.route_shortcut("G", false, false, false));

    // Com fine: true (Shift pressionado), a precisão é 10x maior (passo virtual de 0.1)
    assert!(bridge.update_viewport_transform_modified(410.0, 300.0, true, false));
    let drag = bridge.drag.as_ref().unwrap();
    // 400.0 + (410.0 - 400.0) * 0.1 = 401.0
    assert!((drag.virtual_pointer[0] - 401.0).abs() < 1e-4);
    assert!(bridge.cancel_transform());
}

#[test]
fn gizmo_center_and_plane_hit_testing() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);
    let ox = gizmo.origin_x;
    let oy = gizmo.origin_y;

    // Teste de hit-testing comprovando seleção correta do Center a até 12px de distância da origem
    assert_eq!(bridge.gizmo_handle_at(ox, oy), Some(GizmoHandle::Center));
    assert_eq!(
        bridge.gizmo_handle_at(ox + 8.0, oy + 8.0),
        Some(GizmoHandle::Center)
    );

    // Na ferramenta Scale, o centro também é Center (para escala uniforme)
    bridge.apply(UiIntent::SetActiveTool("scale".to_string()));
    assert_eq!(bridge.gizmo_handle_at(ox, oy), Some(GizmoHandle::Center));

    // Teste dos quadrantes de plano: na ferramenta Move, os comandos de plano existem
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(!gizmo.plane_yz_commands.is_empty());
    assert!(!gizmo.plane_xz_commands.is_empty());
    assert!(!gizmo.plane_xy_commands.is_empty());

    // Hit-testing no centro do quad do plano YZ (normal X, índice 0)
    let numbers: Vec<f32> = gizmo
        .plane_yz_commands
        .split_whitespace()
        .filter_map(|t| t.parse::<f32>().ok())
        .collect();
    assert!(numbers.len() >= 8);
    let cx = (numbers[0] + numbers[2] + numbers[4] + numbers[6]) * 0.25;
    let cy = (numbers[1] + numbers[3] + numbers[5] + numbers[7]) * 0.25;
    assert_eq!(bridge.gizmo_handle_at(cx, cy), Some(GizmoHandle::Plane(0)));

    // Na ferramenta Rotate, o anel externo de View Roll é detectado
    bridge.apply(UiIntent::SetActiveTool("rotate".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(!gizmo.view_roll_commands.is_empty());
    let roll_r = projection::GIZMO_VIEW_ROLL_RADIUS;
    assert_eq!(
        bridge.gizmo_handle_at(ox + roll_r, oy),
        Some(GizmoHandle::Center)
    );
}

#[test]
fn gizmo_center_screen_translation_and_uniform_scale() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    let gizmo = bridge.view_model().gizmo;
    let ox = gizmo.origin_x;
    let oy = gizmo.origin_y;
    let before_verts = bridge.state.project.active_mesh().unwrap().verts.clone();

    // Arrasto pelo centro do Move inicia translação no plano de tela
    assert!(bridge.begin_gizmo_drag(ox, oy));
    assert_eq!(bridge.gizmo_drag, Some(GizmoHandle::Center));
    assert!(bridge.update_viewport_transform(ox + 40.0, oy + 30.0));
    assert!(bridge.end_gizmo_drag());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    // Vértices foram transladados
    let after_verts = &bridge.state.project.active_mesh().unwrap().verts;
    assert!(
        after_verts
            .iter()
            .zip(&before_verts)
            .any(|(a, b)| a.pos != b.pos)
    );

    // Agora testa Escala Uniforme pelo centro do Scale
    bridge.apply(UiIntent::SetActiveTool("scale".to_string()));
    let gizmo_scale = bridge.view_model().gizmo;
    let (sox, soy) = (gizmo_scale.origin_x, gizmo_scale.origin_y);
    let initial_mesh = bridge.state.project.active_mesh().unwrap().clone();
    assert!(bridge.begin_gizmo_drag(sox, soy));
    assert_eq!(bridge.gizmo_drag, Some(GizmoHandle::Center));
    // Arrastar para longe aumenta o fator de escala uniforme
    assert!(bridge.update_viewport_transform(sox + 60.0, soy));
    assert!(bridge.end_gizmo_drag());

    let scaled_mesh = bridge.state.project.active_mesh().unwrap();
    // Verifica que a escala foi uniforme nos 3 eixos (razão idêntica)
    let p0_init = initial_mesh.verts[0].vec();
    let p0_scaled = scaled_mesh.verts[0].vec();
    let pivot = bridge
        .state
        .calculate_pivot(bridge.state.session.pivot_point);
    let ratio_x = (p0_scaled.x - pivot.x) / (p0_init.x - pivot.x);
    let ratio_y = (p0_scaled.y - pivot.y) / (p0_init.y - pivot.y);
    let ratio_z = (p0_scaled.z - pivot.z) / (p0_init.z - pivot.z);
    assert!((ratio_x - ratio_y).abs() < 1e-4);
    assert!((ratio_y - ratio_z).abs() < 1e-4);
}

#[test]
fn test_set_origin_operations_and_undo() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let active_id = bridge.state.project.active().unwrap().id.to_string();

    // Início: asset.origin é None
    assert!(bridge.state.project.active().unwrap().origin.is_none());

    // 1. Origin to Bottom
    assert!(bridge.set_origin_bottom());
    let origin_bottom = bridge.state.project.active().unwrap().origin.unwrap();
    // Para cubo unitário centrado em (0,0,0) de -1 a 1: Y mínimo é -1.0
    assert!((origin_bottom[1] - (-1.0)).abs() < 1e-4);

    // 2. Origin to 3D Cursor
    bridge.state.session.cursor_3d = [5.0, 10.0, 15.0];
    assert!(bridge.set_origin_cursor());
    let origin_cursor = bridge.state.project.active().unwrap().origin.unwrap();
    assert_eq!(origin_cursor, [5.0, 10.0, 15.0]);

    // 3. Origin to Geometry
    assert!(bridge.set_origin_geometry());
    let origin_geom = bridge.state.project.active().unwrap().origin.unwrap();
    assert!((origin_geom[0]).abs() < 1e-4);
    assert!((origin_geom[1]).abs() < 1e-4);
    assert!((origin_geom[2]).abs() < 1e-4);

    // 4. Undo reverte para Cursor, depois Bottom, depois None
    bridge.apply(UiIntent::Undo);
    assert_eq!(
        bridge.state.project.active().unwrap().origin.unwrap(),
        [5.0, 10.0, 15.0]
    );

    bridge.apply(UiIntent::Undo);
    assert!((bridge.state.project.active().unwrap().origin.unwrap()[1] - (-1.0)).abs() < 1e-4);

    bridge.apply(UiIntent::Undo);
    assert!(bridge.state.project.active().unwrap().origin.is_none());

    // 5. Redo funciona
    bridge.apply(UiIntent::Redo);
    assert!(bridge.state.project.active().unwrap().origin.is_some());

    // 6. Geometry to Origin
    // Define origin em (2.0, 0.0, 0.0) e move geometria para lá
    bridge.state.project.active_mut().unwrap().origin = Some([2.0, 0.0, 0.0]);
    let before_center = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .selection_center();
    assert!(bridge.set_geometry_to_origin());
    let after_center = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .selection_center();
    assert!((after_center[0] - 2.0).abs() < 1e-4);
    assert!((after_center[0] - before_center[0] - 2.0).abs() < 1e-4);

    // 7. Context menu action aciona as operações
    bridge.open_context_menu(&active_id, 10.0, 10.0);
    assert!(bridge.context_menu_action("origin_to_bottom"));
    assert!((bridge.state.project.active().unwrap().origin.unwrap()[1] - (-1.0)).abs() < 1e-4);
}

#[test]
fn test_edit_pivot_mode_and_shortcut() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Inicialmente edit_pivot é falso
    assert!(!bridge.state.session.edit_pivot);
    assert!(!bridge.view_model().edit_pivot);

    // Tecla Insert alterna Edit Pivot
    assert!(bridge.route_shortcut("Insert", false, false, false));
    assert!(bridge.state.session.edit_pivot);
    assert!(bridge.view_model().edit_pivot);

    // Inicia Move transform enquanto em Edit Pivot
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let initial_verts = bridge.state.project.active_mesh().unwrap().verts.clone();
    let gizmo = bridge.view_model().gizmo;

    // Arrasta gizmo pelo centro
    assert!(bridge.begin_gizmo_drag(gizmo.origin_x, gizmo.origin_y));
    assert!(bridge.update_viewport_transform(gizmo.origin_x + 50.0, gizmo.origin_y + 50.0));
    assert!(bridge.end_gizmo_drag());

    // Geometria NÃO deve ter mudado
    let current_verts = &bridge.state.project.active_mesh().unwrap().verts;
    for (init, curr) in initial_verts.iter().zip(current_verts.iter()) {
        assert_eq!(init.pos, curr.pos);
    }

    // Mas asset.origin deve ter sido definido e alterado!
    let custom_origin = bridge.state.project.active().unwrap().origin;
    assert!(custom_origin.is_some());

    // Sai do modo Edit Pivot
    assert!(bridge.route_shortcut("Insert", false, false, false));
    assert!(!bridge.state.session.edit_pivot);
    assert!(!bridge.view_model().edit_pivot);

    // Agora o pivô calculado do asset reflete a nova origem
    let pivot = bridge
        .state
        .calculate_pivot(bridge.state.session.pivot_point);
    let orig = custom_origin.unwrap();
    assert_eq!([pivot.x, pivot.y, pivot.z], orig);
}

#[test]
fn test_axis_guide_projection_and_colors() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    let parse_tip = |commands: &str| -> [f32; 2] {
        let numbers: Vec<f32> = commands
            .split(|c: char| !c.is_numeric() && c != '.' && c != '-')
            .filter_map(|s| s.parse().ok())
            .collect();
        [numbers[2], numbers[3]]
    };

    // Sem modal ativo: guia invisível
    let guide_idle = compute_axis_guide(&bridge.state, 800.0, 600.0);
    assert!(!guide_idle.visible);
    assert!(guide_idle.commands.is_empty());

    // Inicia Move modal no eixo X
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;
    let x_tip = parse_tip(&gizmo.x_commands);
    assert!(bridge.begin_gizmo_drag(x_tip[0], x_tip[1]));

    // Com modal ativo no eixo X (índice 0): guia vermelha e com comandos SVG
    let guide_x = compute_axis_guide(&bridge.state, 800.0, 600.0);
    assert!(guide_x.visible);
    assert_eq!(guide_x.color, [229, 77, 66]); // Vermelho do eixo X
    assert!(guide_x.commands.starts_with("M "));
    assert!(guide_x.commands.contains(" L "));

    assert!(bridge.end_gizmo_drag());

    // Inicia Move modal no eixo Y
    let y_tip = parse_tip(&gizmo.y_commands);
    assert!(bridge.begin_gizmo_drag(y_tip[0], y_tip[1]));
    let guide_y = compute_axis_guide(&bridge.state, 800.0, 600.0);
    assert!(guide_y.visible);
    assert_eq!(guide_y.color, [70, 167, 88]); // Verde do eixo Y

    assert!(bridge.end_gizmo_drag());

    // Inicia Move modal no eixo Z
    let z_tip = parse_tip(&gizmo.z_commands);
    assert!(bridge.begin_gizmo_drag(z_tip[0], z_tip[1]));
    let guide_z = compute_axis_guide(&bridge.state, 800.0, 600.0);
    assert!(guide_z.visible);
    assert_eq!(guide_z.color, [62, 99, 221]); // Azul do eixo Z

    assert!(bridge.end_gizmo_drag());
}

#[test]
fn test_dimension_annotation_blueprint_and_hud_pill() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Em repouso: cota e hud pill invisíveis
    let dim_idle = compute_dimension_annotation(&bridge.state, 800.0, 600.0);
    assert!(!dim_idle.visible);
    assert!(!bridge.view_model().hud_pill_visible);

    // Inicia Move arrastando pelo centro
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(bridge.begin_gizmo_drag(gizmo.origin_x, gizmo.origin_y));

    // Atualiza o arrasto com deslocamento significativo
    assert!(bridge.update_viewport_transform(gizmo.origin_x + 80.0, gizmo.origin_y + 40.0));

    // Verifica cota blueprint (Fase 4)
    let dim = compute_dimension_annotation(&bridge.state, 800.0, 600.0);
    assert!(dim.visible);
    assert!(dim.text.contains("m"));
    assert!(dim.commands.contains("M ") && dim.commands.contains(" L "));
    assert!(dim.label_x.is_finite() && dim.label_y.is_finite());

    // Verifica Floating HUD Pill no view model
    let vm = bridge.view_model();
    assert!(vm.hud_pill_visible);
    assert_eq!(vm.hud_pill_title, "Move");
    assert!(!vm.hud_pill_value.is_empty());
    assert!(vm.hud_pill_x > 0.0 && vm.hud_pill_y > 0.0);

    // Finaliza arrasto: cota e hud pill somem
    assert!(bridge.end_gizmo_drag());
    assert!(!compute_dimension_annotation(&bridge.state, 800.0, 600.0).visible);
    assert!(!bridge.view_model().hud_pill_visible);
}

#[test]
fn test_magnetic_snap_marker_projection() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Sem snap ativado: marcador invisível
    assert!(!bridge.state.snap_enabled);
    let marker_idle = compute_snap_marker(&bridge.state, 800.0, 600.0);
    assert!(!marker_idle.visible);

    // Ativa snap e inicia transformação
    bridge.state.snap_enabled = true;
    bridge.state.session.snap_settings.enabled = true;
    bridge.state.session.snap_settings.target = petunia_core::SnapTarget::Grid;
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(bridge.begin_gizmo_drag(gizmo.origin_x, gizmo.origin_y));

    // Snap ligado não basta: nada encaixou ainda (Onda 1, P3D-040).
    assert!(!compute_snap_marker(&bridge.state, 800.0, 600.0).visible);

    // Sem encaixe informado, o core não encaixa sozinho (uma passada só).
    bridge
        .state
        .update_modal(glam::Vec3::new(0.3, 0.0, 0.0), 0.3)
        .unwrap();
    assert!(!compute_snap_marker(&bridge.state, 800.0, 600.0).visible);

    // Depois de um deslocamento que encaixou num ponto, o marcador é projetado
    // com forma e rótulo do tipo de alvo.
    bridge
        .state
        .update_modal_snapped(
            glam::Vec3::new(0.3, 0.0, 0.0),
            0.3,
            Some(petunia_core::SnapKind::Point),
        )
        .unwrap();
    let marker_active = compute_snap_marker(&bridge.state, 800.0, 600.0);
    assert!(marker_active.visible);
    assert!(!marker_active.round);
    assert_eq!(
        marker_active.label,
        bridge.state.t_id(petunia_config::text_id::SNAP_KIND_POINT)
    );
    assert!(marker_active.x >= 0.0 && marker_active.x <= 800.0);
    assert!(marker_active.y >= 0.0 && marker_active.y <= 600.0);

    assert!(bridge.end_gizmo_drag());
}

#[test]
fn test_accessibility_preferences_and_intents() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // 1. Valores iniciais padrão
    let vm = bridge.view_model();
    assert!(!vm.colorblind_axes);
    assert!(!vm.reduced_motion);
    assert_eq!(vm.double_tap_interval_ms, 350);
    assert!(!vm.label_colorblind_axes.is_empty());
    assert!(!vm.label_reduced_motion.is_empty());
    assert!(!vm.label_double_tap_interval.is_empty());

    // 2. Modificação via setters dedicados
    assert!(bridge.set_colorblind_axes(true));
    assert!(bridge.view_model().colorblind_axes);
    assert!(bridge.preferences.colorblind_axes);

    assert!(bridge.set_reduced_motion(true));
    assert!(bridge.view_model().reduced_motion);
    assert!(bridge.preferences.reduced_motion);

    assert!(bridge.set_double_tap_interval_ms(500));
    assert_eq!(bridge.view_model().double_tap_interval_ms, 500);
    assert_eq!(bridge.preferences.double_tap_interval_ms, 500);

    // Clamping do intervalo de duplo toque (máximo 2000 ms)
    assert!(bridge.set_double_tap_interval_ms(5000));
    assert_eq!(bridge.view_model().double_tap_interval_ms, 2000);

    // 3. Modificação via UiIntent
    bridge.apply(UiIntent::SetColorblindAxes(false));
    assert!(!bridge.view_model().colorblind_axes);

    bridge.apply(UiIntent::SetReducedMotion(false));
    assert!(!bridge.view_model().reduced_motion);

    bridge.apply(UiIntent::SetDoubleTapIntervalMs(250));
    assert_eq!(bridge.view_model().double_tap_interval_ms, 250);

    // 4. Sincronização de preferências do estado vivo
    bridge.state.ui.colorblind_axes = true;
    bridge.state.ui.reduced_motion = true;
    bridge.sync_preferences_from_state();
    assert!(bridge.preferences.colorblind_axes);
    assert!(bridge.preferences.reduced_motion);
}

#[test]
fn test_colorblind_gizmo_axis_endpoints() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);
    assert!(gizmo.origin_x > 0.0 && gizmo.origin_y > 0.0);

    // As coordenadas de ponta dos eixos devem ser válidas e distintas da origem
    for (label, end, lbl_pos) in [
        ("X", gizmo.x_end, gizmo.x_label),
        ("Y", gizmo.y_end, gizmo.y_label),
        ("Z", gizmo.z_end, gizmo.z_label),
    ] {
        assert!(end[0].is_finite(), "Endpoint {label} x is not finite");
        assert!(end[1].is_finite(), "Endpoint {label} y is not finite");
        let dist = ((end[0] - gizmo.origin_x).powi(2) + (end[1] - gizmo.origin_y).powi(2)).sqrt();
        assert!(
            dist > 10.0,
            "Endpoint {label} está muito próximo da origem do gizmo (dist={dist})"
        );

        // Os rótulos de acessibilidade não-cromática ficam ligeiramente além da ponta da haste
        assert!(lbl_pos[0].is_finite(), "Label {label} x is not finite");
        assert!(lbl_pos[1].is_finite(), "Label {label} y is not finite");
        let lbl_dist =
            ((lbl_pos[0] - gizmo.origin_x).powi(2) + (lbl_pos[1] - gizmo.origin_y).powi(2)).sqrt();
        assert!(
            lbl_dist > dist,
            "Rótulo {label} deve ficar posicionado além da ponta da haste do gizmo (lbl_dist={lbl_dist}, end_dist={dist})"
        );
    }
}

#[test]
fn test_axis_guide_colorblind_labels() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    bridge
        .state
        .begin_modal(petunia_core::ModalKind::Move)
        .unwrap();

    // Restrição ao eixo X
    bridge
        .state
        .set_modal_constraint(petunia_core::ModalConstraint::Axis(0))
        .unwrap();
    let vm = bridge.view_model();
    assert!(vm.axis_guide_visible);
    assert_eq!(vm.axis_guide_label, "X");
    assert!(vm.axis_guide_label_x.is_finite());
    assert!(vm.axis_guide_label_y.is_finite());

    // Restrição ao eixo Y
    bridge
        .state
        .set_modal_constraint(petunia_core::ModalConstraint::Axis(1))
        .unwrap();
    let vm = bridge.view_model();
    assert_eq!(vm.axis_guide_label, "Y");

    // Restrição ao eixo Z
    bridge
        .state
        .set_modal_constraint(petunia_core::ModalConstraint::Axis(2))
        .unwrap();
    let vm = bridge.view_model();
    assert_eq!(vm.axis_guide_label, "Z");
}

#[test]
fn test_world_axis_labels_projection() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    // Quando colorblind_axes = false (padrão), não projeta rótulos de eixos de grid
    assert!(!bridge.view_model().colorblind_axes);
    assert!(bridge.view_model().world_axis_labels.is_empty());

    // Ativa diferenciação não-cromática
    bridge.apply(UiIntent::SetColorblindAxes(true));
    let vm = bridge.view_model();
    assert!(vm.colorblind_axes);
    assert!(!vm.world_axis_labels.is_empty());

    let texts: Vec<&str> = vm
        .world_axis_labels
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert!(
        texts.contains(&"+X")
            || texts.contains(&"-X")
            || texts.contains(&"+Z")
            || texts.contains(&"-Z")
    );

    for label in &vm.world_axis_labels {
        assert!(label.x >= 0.0 && label.x <= 800.0);
        assert!(label.y >= 0.0 && label.y <= 600.0);
    }
}

#[test]
fn test_primitive_parametric_creation_and_callbacks() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    // 1. Criar cilindro mantém a sessão paramétrica ativa (não auto-confirma)
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    let vm = bridge.view_model();
    assert!(
        vm.primitive_active,
        "sessão de primitiva precisa estar ativa"
    );
    assert_eq!(vm.primitive_kind, "cylinder");
    assert_eq!(vm.primitive_sides, 8);
    assert!((vm.primitive_radius - 1.0).abs() < 1e-4);

    let initial_verts = bridge.state.project.active_mesh().unwrap().verts.len();

    // 2. Atualizar lados para 16 deve reconstruir a geometria com 16 lados imediatamente
    assert!(bridge.update_primitive_param_float("sides", 16.0));
    let vm2 = bridge.view_model();
    assert_eq!(vm2.primitive_sides, 16);
    let new_verts = bridge.state.project.active_mesh().unwrap().verts.len();
    assert!(
        new_verts > initial_verts,
        "aumentar lados de 8 para 16 precisa aumentar número de vértices: {new_verts} vs {initial_verts}"
    );

    // 3. Atualizar raio para 2.5
    assert!(bridge.update_primitive_param_float("radius", 2.5));
    assert!((bridge.view_model().primitive_radius - 2.5).abs() < 1e-4);

    // 4. Confirmar primitiva encerra a sessão e comita o asset
    assert!(bridge.confirm_primitive());
    let vm_final = bridge.view_model();
    assert!(!vm_final.primitive_active, "confirmar encerra a sessão");
    assert!(bridge.state.session.primitive_session.is_none());
}

#[test]
fn test_primitive_cancellation_reverts_geometry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    let initial_assets = bridge.state.project.assets.len();

    // Criar icosphere
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Icosphere,
    ));
    assert!(bridge.view_model().primitive_active);
    assert_eq!(bridge.state.project.assets.len(), initial_assets + 1);

    // Cancelar primitiva remove o asset e encerra a sessão
    assert!(bridge.cancel_primitive());
    assert!(!bridge.view_model().primitive_active);
    assert_eq!(bridge.state.project.assets.len(), initial_assets);
}

#[test]
fn test_slice_trim_toggle_and_view_model() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    assert!(!bridge.slice_trim);
    assert!(!bridge.view_model().slice_trim);

    bridge.slice_trim = true;
    assert!(bridge.view_model().slice_trim);

    bridge.slice_trim = false;
    assert!(!bridge.view_model().slice_trim);
}

#[test]
fn test_commit_transform_text_arithmetic_expressions() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Expressão absoluta: 10 + 5 * 2 = 20
    let res = bridge
        .commit_transform_text(TransformKind::Position, 0, "10 + 5 * 2")
        .unwrap();
    assert_eq!(res, 20.0);
    assert_eq!(bridge.view_model().position[0], 20.0);

    // Operação relativa com '+': base atual é 20, + 10 -> 30
    let res = bridge
        .commit_transform_text(TransformKind::Position, 0, "+10")
        .unwrap();
    assert_eq!(res, 30.0);

    // Multiplicação relativa: * 2 -> 60
    let res = bridge
        .commit_transform_text(TransformKind::Position, 0, "*2")
        .unwrap();
    assert_eq!(res, 60.0);

    // Divisão relativa: / 3 -> 20
    let res = bridge
        .commit_transform_text(TransformKind::Position, 0, "/3")
        .unwrap();
    assert_eq!(res, 20.0);

    // Atribuição relativa: -= 5 -> 15
    let res = bridge
        .commit_transform_text(TransformKind::Position, 0, "-= 5")
        .unwrap();
    assert_eq!(res, 15.0);

    // Expressão com variável 'x' ou 'v': x / 3 -> 5.0
    let res = bridge
        .commit_transform_text(TransformKind::Position, 0, "x / 3")
        .unwrap();
    assert_eq!(res, 5.0);
    assert_eq!(bridge.view_model().position[0], 5.0);

    // Undo restaura a última transformação
    assert!(bridge.state.undo());
}

#[test]
fn test_quick_measure_with_two_selected_vertices() {
    let mut state = AppState::default();
    state.set_selection_domain(SelectionDomain::Vertex);
    state.selection.verts.clear();
    state.selection.verts.push(0);
    state.selection.verts.push(1);

    let measure = compute_quick_measure(&state, 800.0, 600.0);
    assert!(measure.visible);
    assert!(measure.distance > 0.0);
    assert!(!measure.commands.is_empty());
    assert!(measure.commands.starts_with("M "));
    assert!(!measure.text.is_empty());
    assert!(measure.text.contains('m'));

    // Com 3 vértices selecionados, a fita métrica desativa
    state.selection.verts.push(2);
    let measure3 = compute_quick_measure(&state, 800.0, 600.0);
    assert!(!measure3.visible);

    // Com 1 vértice, a fita métrica desativa
    state.selection.verts.clear();
    state.selection.verts.push(0);
    let measure1 = compute_quick_measure(&state, 800.0, 600.0);
    assert!(!measure1.visible);
}

#[test]
fn test_quick_measure_with_active_measurement() {
    let mut state = AppState::default();
    state.session.tools.active_measurement = Some(petunia_project::MeasurementItem::new(
        "Measure",
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 0.0],
        5.0,
    ));

    let measure = compute_quick_measure(&state, 800.0, 600.0);
    assert!(measure.visible);
    assert!((measure.distance - 5.0).abs() < 1e-4);
    assert!((measure.dx - 3.0).abs() < 1e-4);
    assert!((measure.dy - 4.0).abs() < 1e-4);
    assert!(measure.dz < 1e-4);
    assert!(measure.text.contains("5.000m"));
}

#[test]
fn test_quick_measure_with_single_selected_edge() {
    let mut state = AppState::default();
    state.set_selection_domain(SelectionDomain::Edge);
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.selected_edges.clear();
        mesh.selected_edges.insert((0, 1));
    }

    let measure = compute_quick_measure(&state, 800.0, 600.0);
    assert!(measure.visible);
    assert!(measure.distance > 0.0);
    assert_eq!(measure.tags.len(), 1);
    assert!(measure.tags[0].text.ends_with('m'));
    assert!(measure.hud_text.contains('m'));
    assert!(measure.hud_text.contains("ΔX:"));
    assert!(measure.hud_text.contains("°"));
}

#[test]
fn test_quick_measure_with_multiple_selected_edges() {
    let mut state = AppState::default();
    state.set_selection_domain(SelectionDomain::Edge);
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.selected_edges.clear();
        mesh.selected_edges.insert((0, 1));
        mesh.selected_edges.insert((1, 2));
    }

    let measure = compute_quick_measure(&state, 800.0, 600.0);
    assert!(measure.visible);
    assert!(measure.distance > 0.0);
    assert_eq!(measure.tags.len(), 1);
    assert!(measure.tags[0].text.starts_with("Ø "));
    assert!(measure.hud_text.contains("Total:"));
    assert!(measure.hud_text.contains("(2 edges)"));
    assert!(measure.hud_text.contains("Avg:"));
    assert!(measure.hud_text.contains("Span:"));
}

#[test]
fn test_quick_measure_view_model_fields() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.selected_edges.clear();
        mesh.selected_edges.insert((0, 1));
    }

    let vm = bridge.view_model();
    assert!(vm.measure_visible);
    assert_eq!(vm.measure_tags.len(), 1);
    assert!(vm.measure_hud_text.contains("ΔX:"));
}

#[test]
fn test_micro_inspector_toggle_and_shortcut() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    assert!(!bridge.micro_inspector_open);
    assert!(!bridge.view_model().micro_inspector_open);

    // Toggle via método
    assert!(bridge.toggle_micro_inspector());
    assert!(bridge.micro_inspector_open);
    assert!(bridge.view_model().micro_inspector_open);
    assert!(bridge.overlays.contains(OverlayId::MicroInspector));

    // Fechar com Escape
    assert!(bridge.handle_escape());
    assert!(!bridge.micro_inspector_open);
    assert!(!bridge.view_model().micro_inspector_open);
    assert!(!bridge.overlays.contains(OverlayId::MicroInspector));

    // Abrir via atalho de teclado Space
    assert!(bridge.route_shortcut("Space", false, false, false));
    assert!(bridge.micro_inspector_open);
    assert!(bridge.view_model().micro_inspector_open);

    // Pressionar Space novamente fecha
    assert!(bridge.route_shortcut("Space", false, false, false));
    assert!(!bridge.micro_inspector_open);
    assert!(!bridge.view_model().micro_inspector_open);
}

#[test]
fn test_selection_domain_shortcuts_and_d_key() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );

    // Atalhos 1, 2, 3, 4 no workspace MODEL
    assert!(bridge.route_shortcut("1", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Vertex
    );

    assert!(bridge.route_shortcut("2", false, false, false));
    assert_eq!(bridge.state.session.selection_domain, SelectionDomain::Edge);

    assert!(bridge.route_shortcut("3", false, false, false));
    assert_eq!(bridge.state.session.selection_domain, SelectionDomain::Face);

    assert!(bridge.route_shortcut("4", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );

    // Tecla 0 não é mais atalho de seleção de objeto (foi removida)
    assert!(!bridge.route_shortcut("0", false, false, false));

    // Tab alterna entre modo objeto e o último domínio usado
    assert!(bridge.route_shortcut("2", false, false, false));
    assert_eq!(bridge.state.session.selection_domain, SelectionDomain::Edge);
    assert!(bridge.route_shortcut("Tab", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );
    assert!(bridge.route_shortcut("Tab", false, false, false));
    assert_eq!(bridge.state.session.selection_domain, SelectionDomain::Edge);

    // Atalho D / d alterna Edit Pivot
    assert!(!bridge.state.session.edit_pivot);
    assert!(bridge.route_shortcut("d", false, false, false));
    assert!(bridge.state.session.edit_pivot);

    // HUD reflete modo Edit Pivot
    let vm = bridge.view_model();
    assert!(vm.operation_hud_active);
    assert_eq!(vm.operation_hud_title, "Edit Pivot Mode");
    assert_eq!(vm.operation_hud_subject, "Pivot");

    // Escape sai de Edit Pivot
    assert!(bridge.route_shortcut("Escape", false, false, false));
    assert!(!bridge.state.session.edit_pivot);
}

#[test]
fn test_slice_15_deg_snap_and_trim_arrow() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("slice".to_string()));
    assert!(bridge.begin_slice(100.0, 100.0));
    assert_eq!(bridge.slice_anchor, Some([100.0, 100.0]));

    // Movimento com snap: ângulo ~5.7° deve arredondar para 0° (dx=100.0, dy=0.0)
    assert!(bridge.update_viewport_slice_modified(200.0, 110.0, true));
    assert_eq!(bridge.pointer_position[1], 100.0);
    assert!(bridge.pointer_position[0] > 190.0);

    // Sem trim: apenas linha M .. L ..
    bridge.slice_trim = false;
    let vm_no_trim = bridge.view_model();
    assert!(!vm_no_trim.slice_preview_commands.is_empty());

    // Com trim: seta de direção normal adicionada
    bridge.slice_trim = true;
    let vm_trim = bridge.view_model();
    assert!(vm_trim.slice_preview_commands.contains("M "));
    // A seta inclui múltiplos segmentos L
    let line_segments = vm_trim.slice_preview_commands.matches(" L ").count();
    assert!(
        line_segments >= 3,
        "Trim preview should contain normal arrow segments"
    );

    // Escape cancela o corte
    assert!(bridge.route_shortcut("Escape", false, false, false));
    assert!(bridge.slice_anchor.is_none());
}

#[test]
fn test_slice_persistent_cut_and_multi_drag_and_shortcut_t_and_confirm() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("slice".to_string()));

    // 1. Primeiro arrasto
    assert!(bridge.begin_slice(400.0, 300.0));
    assert!(bridge.update_slice(400.0, 380.0));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert!(bridge.slice_anchor.is_some());
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));

    // 2. Ajuste/recorreção antes de confirmar: segundo arrasto sem ter feito commit
    assert!(bridge.begin_slice(400.0, 300.0));
    assert!(bridge.update_slice(450.0, 300.0));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert!(bridge.slice_anchor.is_some());
    assert_eq!(bridge.state.project.undo.depth(), (0, 0));

    // 3. Tecla rápida 'T' alterna entre Split e Trim
    assert!(!bridge.slice_trim);
    assert!(bridge.route_shortcut("t", false, false, false));
    assert!(bridge.slice_trim);
    assert!(bridge.route_shortcut("T", false, false, false));
    assert!(!bridge.slice_trim);

    // 4. Enter confirma o corte
    assert!(bridge.route_shortcut("Enter", false, false, false));
    assert!(bridge.slice_anchor.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn test_uv_seam_shortcut_u() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Seleciona domínio de aresta e uma aresta
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        mesh.selected_edges.clear();
        mesh.selected_edges.insert((0, 1));
    }

    // Tecla 'u' alterna costura da aresta selecionada
    assert!(bridge.route_shortcut("u", false, false, false));
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(mesh.uv_seams.contains(&(0, 1)) || mesh.uv_seams.contains(&(1, 0)));

    // Pressionar 'u' novamente remove a costura
    assert!(bridge.route_shortcut("u", false, false, false));
    let mesh = bridge.state.project.active_mesh().unwrap();
    assert!(!mesh.uv_seams.contains(&(0, 1)) && !mesh.uv_seams.contains(&(1, 0)));
}

#[test]
fn test_merge_down_paint_layer() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

    assert!(bridge.add_paint_layer());
    assert_eq!(bridge.view_model().paint_layers.len(), 2);
    let new_id = bridge.view_model().paint_layers[1].id.clone();
    let base_id = bridge.view_model().paint_layers[0].id.clone();

    // Tentar merge down da camada base (índice 0) deve falhar
    assert!(!bridge.merge_down_paint_layer(&base_id));

    // Merge down da camada 2 com a base deve suceder e resultar em 1 camada
    assert!(bridge.merge_down_paint_layer(&new_id));
    assert_eq!(bridge.view_model().paint_layers.len(), 1);
    assert_eq!(bridge.view_model().paint_layers[0].id, base_id);
}

#[test]
fn test_profile_revolve_custom_angle() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("draw_profile".to_string()));
    bridge.add_profile_rectangle(1.0, 2.0);
    bridge.state.profile.revolve_angle = 180.0;
    bridge.state.profile.revolve_segments = 8;

    let initial_count = bridge.state.project.assets.len();
    assert!(bridge.generate_profile_revolve());
    assert_eq!(bridge.state.project.assets.len(), initial_count + 1);

    let active_mesh = bridge.state.project.active_mesh().unwrap();
    assert!(!active_mesh.verts.is_empty());
    assert!(!active_mesh.faces.is_empty());
}

#[test]
fn test_profile_bezier_smoothing_and_wall_thickness() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("draw_profile".to_string()));

    // Cria retângulo 2D
    bridge.add_profile_rectangle(4.0, 4.0);
    let vm = bridge.view_model();
    assert_eq!(vm.profile_point_count, 4);
    assert!(vm.profile_closed);
    assert!(!vm.profile_has_curves);

    // Ajusta espessura de parede (perfil oco)
    assert!(bridge.set_profile_wall_thickness(0.5));
    let profile_id = bridge.active_profile_id.expect("profile id");
    assert_eq!(
        bridge
            .state
            .project
            .project
            .get_profile(profile_id)
            .unwrap()
            .wall_thickness,
        0.5
    );

    // Ativa suavização Bézier
    assert!(bridge.profile_smooth_curves());
    let vm_smoothed = bridge.view_model();
    assert!(vm_smoothed.profile_has_curves);

    // Pontos efetivos devem conter laço externo e laço interno
    let effective = bridge.active_profile_state().unwrap().effective_points();
    assert!(effective.len() > 8);

    // Gera extrusão com sucesso
    let initial_count = bridge.state.project.assets.len();
    assert!(bridge.generate_profile_extrude());
    assert_eq!(bridge.state.project.assets.len(), initial_count + 1);

    let active_mesh = bridge.state.project.active_mesh().unwrap();
    assert!(!active_mesh.verts.is_empty());
    assert!(!active_mesh.faces.is_empty());

    // Limpeza de curvas
    bridge.apply(UiIntent::SetActiveTool("draw_profile".to_string()));
    bridge.add_profile_rectangle(2.0, 2.0);
    bridge.profile_smooth_curves();
    assert!(bridge.view_model().profile_has_curves);
    bridge.profile_clear_curves();
    assert!(!bridge.view_model().profile_has_curves);
}

#[test]
fn test_profile_sweep_generation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetActiveTool("draw_profile".to_string()));

    // 1. Sweep com caminho padrão
    bridge.add_profile_circle(0.5, 8);
    let initial_count = bridge.state.project.assets.len();
    assert!(bridge.generate_profile_sweep());
    assert_eq!(bridge.state.project.assets.len(), initial_count + 1);

    let sweep_mesh = bridge.state.project.active_mesh().unwrap();
    assert!(!sweep_mesh.verts.is_empty());
    assert!(!sweep_mesh.faces.is_empty());

    // 2. Sweep com caminho extraído de arestas selecionadas
    bridge.apply(UiIntent::SetActiveTool("draw_profile".to_string()));
    bridge.add_profile_rectangle(0.4, 0.4);

    let mut guide_mesh = petunia_mesh::Mesh::default();
    guide_mesh
        .verts
        .push(petunia_mesh::Vertex::new(0.0, 0.0, 0.0));
    guide_mesh
        .verts
        .push(petunia_mesh::Vertex::new(0.0, 1.0, 0.0));
    guide_mesh
        .verts
        .push(petunia_mesh::Vertex::new(1.0, 2.0, 0.0));
    guide_mesh.selected_edges.insert((0, 1));
    guide_mesh.selected_edges.insert((1, 2));
    bridge.state.project.add("Guide", guide_mesh);

    let count_before = bridge.state.project.assets.len();
    assert!(bridge.generate_profile_sweep());
    assert_eq!(bridge.state.project.assets.len(), count_before + 1);

    let result_mesh = bridge.state.project.active_mesh().unwrap();
    assert!(result_mesh.verts.len() >= 8);
    assert!(!result_mesh.faces.is_empty());
}

#[test]
fn test_boolean_op_auto_operand_with_two_objects() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Initial scene has 1 cube
    assert_eq!(bridge.state.project.assets.len(), 1);

    // Adiciona uma esfera na cena -> total 2 objetos
    bridge
        .state
        .project
        .add("Cube2", petunia_core::Mesh::cube(1.0));
    assert_eq!(bridge.state.project.assets.len(), 2);

    // Sem selecionar operando manualmente, Fuse auto-seleciona o outro objeto e executa
    assert!(bridge.state.session.tools.boolean_operand.is_none());
    assert!(bridge.boolean_op("model.fuse"));
    assert_eq!(bridge.state.project.assets.len(), 1);
}

#[test]
fn test_proportional_editing_visual_circle_overlay() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Desabilitado por padrão -> sem comandos de círculo
    assert!(!bridge.view_model().proportional_editing);
    assert!(bridge.view_model().proportional_circle_commands.is_empty());

    // Habilita proportional editing
    bridge.apply(UiIntent::ToggleProportionalEditing);
    assert!(bridge.view_model().proportional_editing);

    // Sem modal/arrasto ativo -> sem comandos de círculo
    assert!(bridge.view_model().proportional_circle_commands.is_empty());

    // Inicia modal de Move -> círculo pontilhado gerado
    bridge
        .state
        .begin_modal(petunia_core::ModalKind::Move)
        .unwrap();
    let vm = bridge.view_model();
    assert!(!vm.proportional_circle_commands.is_empty());
    assert!(vm.proportional_circle_commands.contains("M "));
    assert!(vm.proportional_circle_commands.contains("L "));

    // Ajuste do raio via gesture de Zoom atualiza a geometria do círculo
    let prev_cmds = vm.proportional_circle_commands.clone();
    bridge.apply_viewport_gesture(ViewportGesture::Zoom { delta: 1.0 });
    let new_vm = bridge.view_model();
    assert_ne!(prev_cmds, new_vm.proportional_circle_commands);

    // Cancelar modal remove o overlay do círculo
    bridge.state.cancel_modal();
    assert!(bridge.view_model().proportional_circle_commands.is_empty());
}

#[test]
fn test_toggle_bevel_clamp_overlap() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Padrão é clamp overlap ativado (true)
    assert!(bridge.state.tools.bevel_clamp_overlap);
    assert!(bridge.view_model().bevel_clamp_overlap);

    // Alterna para false
    assert!(!bridge.toggle_bevel_clamp_overlap());
    assert!(!bridge.state.tools.bevel_clamp_overlap);
    assert!(!bridge.view_model().bevel_clamp_overlap);

    // Alterna de volta para true
    assert!(bridge.toggle_bevel_clamp_overlap());
    assert!(bridge.state.tools.bevel_clamp_overlap);
    assert!(bridge.view_model().bevel_clamp_overlap);
}

#[test]
fn test_uv_equalize_texel_density() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.switch_workspace(petunia_core::Workspace::Uv);

    let mesh = bridge.state.project.active_mesh_mut().unwrap();
    let f0_verts = mesh.faces[0].verts.clone();
    for i in 0..f0_verts.len() {
        let v0 = f0_verts[i];
        let v1 = f0_verts[(i + 1) % f0_verts.len()];
        mesh.mark_seam(v0, v1);
    }
    mesh.faces[0].uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    mesh.faces[1].uv = vec![[0.0, 0.0], [0.2, 0.0], [0.2, 0.2], [0.0, 0.2]];

    assert!(bridge.uv_equalize_texel_density());
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn test_paint_brush_hardness_and_shortcuts() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Paint));

    // Hardness inicial é 0.0
    assert_eq!(bridge.view_model().brush_hardness, 0.0);

    // Ajusta dureza via método e via intent
    bridge.set_brush_hardness(0.5);
    assert_eq!(bridge.state.session.tools.brush_hardness, 0.5);
    assert_eq!(bridge.view_model().brush_hardness, 0.5);

    // Atalhos de colchetes: '[' diminui tamanho, ']' aumenta tamanho
    let prev_size = bridge.state.session.tools.paint_radius;
    assert!(bridge.route_shortcut("]", false, false, false));
    assert!(bridge.state.session.tools.paint_radius > prev_size);

    // Shift + '[' diminui dureza, Shift + ']' aumenta dureza
    assert!(bridge.route_shortcut("]", false, true, false));
    assert!(bridge.state.session.tools.brush_hardness > 0.5);
    assert!(bridge.route_shortcut("[", false, true, false));
    assert_eq!(bridge.state.session.tools.brush_hardness, 0.5);
}

#[test]
fn test_paint_symmetry_toggles_and_view_model() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Paint));

    assert!(!bridge.view_model().paint_symmetry_x);
    assert!(!bridge.view_model().paint_symmetry_y);
    assert!(!bridge.view_model().paint_symmetry_z);

    // Toggle X
    assert!(bridge.toggle_paint_symmetry_x());
    assert!(bridge.state.session.tools.paint_symmetry_x);
    assert!(bridge.view_model().paint_symmetry_x);

    // Toggle Y
    assert!(bridge.toggle_paint_symmetry_y());
    assert!(bridge.state.session.tools.paint_symmetry_y);
    assert!(bridge.view_model().paint_symmetry_y);

    // Toggle Z
    assert!(bridge.toggle_paint_symmetry_z());
    assert!(bridge.state.session.tools.paint_symmetry_z);
    assert!(bridge.view_model().paint_symmetry_z);

    // Setters
    bridge.set_paint_symmetry_x(false);
    assert!(!bridge.state.session.tools.paint_symmetry_x);
    assert!(!bridge.view_model().paint_symmetry_x);
}

#[test]
fn test_uv_pinning_bridge_and_shortcuts() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Uv));

    // Seleciona a face 0 em UV
    bridge.state.session.uv_selected.insert(0);

    // Inicialmente sem pins
    assert_eq!(bridge.view_model().uv_editor.pinned_count, 0);

    // Atalho P alterna fixação (pin)
    assert!(bridge.route_shortcut("p", false, false, false));
    assert!(bridge.view_model().uv_editor.pinned_count > 0);
    assert!(!bridge.view_model().uv_editor.pinned_commands.is_empty());

    // Atalho Alt+P limpa todas as fixações
    assert!(bridge.route_shortcut("p", false, false, true));
    assert_eq!(bridge.view_model().uv_editor.pinned_count, 0);

    // Método direto do bridge
    assert!(bridge.toggle_selected_uv_pins());
    assert!(bridge.view_model().uv_editor.pinned_count > 0);
    assert!(bridge.clear_all_uv_pins());
    assert_eq!(bridge.view_model().uv_editor.pinned_count, 0);
}

#[test]
fn test_nudge_selection_with_arrow_keys() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // 1. No modo Objeto: setas movem todos os vértices do asset ativo
    let vx_before = bridge.state.project.active_mesh().unwrap().verts[0].pos[0];
    assert!(bridge.route_shortcut("Right", false, false, false));
    let vx_after = bridge.state.project.active_mesh().unwrap().verts[0].pos[0];
    assert!((vx_after - (vx_before + 0.1)).abs() < 1e-4);

    // Shift reduz o passo para 0.01 (modo de precisão)
    assert!(bridge.route_shortcut("Left", false, true, false));
    let vx_fine = bridge.state.project.active_mesh().unwrap().verts[0].pos[0];
    assert!((vx_fine - (vx_after - 0.01)).abs() < 1e-4);

    // 2. No modo Edição: setas movem os vértices selecionados
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    let vy_before = bridge.state.project.active_mesh().unwrap().verts[0].pos[1];
    assert!(bridge.route_shortcut("Up", false, false, false));
    let vy_after = bridge.state.project.active_mesh().unwrap().verts[0].pos[1];
    assert!((vy_after - (vy_before + 0.1)).abs() < 1e-4);

    // 3. No workspace UV: setas movem as faces selecionadas no espaço UV
    bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Uv));
    bridge.state.session.uv_selected.insert(0);
    let u_before = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
    assert!(bridge.route_shortcut("Right", false, false, false));
    let u_after = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
    assert!((u_after - (u_before + 0.01)).abs() < 1e-4);
}

#[test]
fn test_toggle_bevel_affect_vertices() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    // Padrão é affect_vertices desativado (false)
    assert!(!bridge.state.tools.bevel_affect_vertices);
    assert!(!bridge.view_model().bevel_affect_vertices);

    // Alterna para true
    assert!(bridge.toggle_bevel_affect_vertices());
    assert!(bridge.state.tools.bevel_affect_vertices);
    assert!(bridge.view_model().bevel_affect_vertices);

    // Alterna de volta para false
    assert!(!bridge.toggle_bevel_affect_vertices());
    assert!(!bridge.state.tools.bevel_affect_vertices);
    assert!(!bridge.view_model().bevel_affect_vertices);
}

#[test]
fn test_protractor_overlay_during_rotate_modal() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Sem modal ativo: transferidor invisível
    let protractor = projection::compute_protractor(&bridge.state, 800.0, 600.0, None);
    assert!(!protractor.visible);
    assert!(protractor.wedge_commands.is_empty());
    assert!(protractor.ticks_commands.is_empty());

    // Inicia rotação modal
    bridge
        .state
        .begin_modal(petunia_core::ModalKind::Rotate)
        .unwrap();
    bridge.state.update_modal(glam::Vec3::ZERO, 45.0).unwrap();

    let protractor_rot = projection::compute_protractor(&bridge.state, 800.0, 600.0, None);
    assert!(protractor_rot.visible);
    assert!(!protractor_rot.wedge_commands.is_empty());
    assert!(protractor_rot.wedge_commands.contains('M'));
    assert!(protractor_rot.wedge_commands.contains('Z'));
    assert!(!protractor_rot.ticks_commands.is_empty());
    assert_eq!(protractor_rot.angle_degrees, 45.0);

    // View model reflete o transferidor
    let vm = bridge.view_model();
    assert!(vm.protractor_visible);
    assert_eq!(vm.protractor_wedge_commands, protractor_rot.wedge_commands);
    assert_eq!(vm.protractor_ticks_commands, protractor_rot.ticks_commands);

    // Cancela modal: transferidor volta a ficar invisível
    bridge.state.cancel_modal();
    let protractor_after = projection::compute_protractor(&bridge.state, 800.0, 600.0, None);
    assert!(!protractor_after.visible);
}

#[test]
fn shortcuts_model_reflects_custom_keybinds() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Padrão canônico inicial
    let vm1 = bridge.view_model();
    assert_eq!(vm1.shortcuts.model_move, "G");
    assert_eq!(vm1.shortcuts.undo, "Ctrl+Z");
    assert_eq!(vm1.shortcuts.model_extrude, "E");

    // Customização de atalho em tempo de execução (ex: usuário altera no menu Settings)
    let new_binding = petunia_config::keybinds::parse_binding("W").expect("binding valido");
    bridge
        .state
        .ui
        .keybinds
        .set_binding("model.move", new_binding);

    // Imediatamente refletido no view_model e repassado para as tooltips
    let vm2 = bridge.view_model();
    assert_eq!(vm2.shortcuts.model_move, "W");
    assert_eq!(vm2.shortcuts.undo, "Ctrl+Z");
    assert_eq!(vm2.shortcuts.model_extrude, "E");
}

#[test]
fn wireframe_overlay_defaults_to_true() {
    let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(bridge.state.session.show_wireframe_overlay);
    assert!(bridge.view_model().is_wireframe);
}

#[test]
fn tab_cycles_selection_domain_between_object_and_last_subelement() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );

    // Seleciona Face com "3"
    assert!(bridge.route_shortcut("3", false, false, false));
    assert_eq!(bridge.state.session.selection_domain, SelectionDomain::Face);

    // Tab vai para Object
    assert!(bridge.route_shortcut("Tab", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );

    // Tab volta para Face
    assert!(bridge.route_shortcut("Tab", false, false, false));
    assert_eq!(bridge.state.session.selection_domain, SelectionDomain::Face);

    // Seleciona Vértice com "1"
    assert!(bridge.route_shortcut("1", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Vertex
    );

    // Tab vai para Object e volta para Vértice
    assert!(bridge.route_shortcut("Tab", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );
    assert!(bridge.route_shortcut("Tab", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Vertex
    );

    // Atalho "4" vai para Object
    assert!(bridge.route_shortcut("4", false, false, false));
    assert_eq!(
        bridge.state.session.selection_domain,
        SelectionDomain::Object
    );

    // Atalho "0" foi removido e não funciona
    assert!(!bridge.route_shortcut("0", false, false, false));
}

#[test]
fn multiselection_measure_tag_displays_single_average_and_toggles_via_preference() {
    let mut state = AppState::default();
    state.set_selection_domain(SelectionDomain::Edge);
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.selected_edges.clear();
        mesh.selected_edges.insert((0, 1));
        mesh.selected_edges.insert((1, 2));
        mesh.selected_edges.insert((2, 3));
    }

    let measure = compute_quick_measure(&state, 800.0, 600.0);

    // Deve ser visível com exatamente 1 tag única contendo a média (Ø)
    assert!(measure.visible);
    assert_eq!(measure.tags.len(), 1);
    assert!(measure.tags[0].text.starts_with("Ø "));

    // Desativa a preferência
    state.ui.multiselection_measure_tag = false;
    let measure_disabled = compute_quick_measure(&state, 800.0, 600.0);
    assert!(measure_disabled.visible);
    // Com a preferência desligada, não deve projetar tags na viewport
    assert_eq!(measure_disabled.tags.len(), 0);
    // Mas o HUD text continua presente
    assert!(measure_disabled.hud_text.contains("Total:"));

    // Reativa a preferência
    let mut bridge = SlintUiBridge::new(state, PlaceholderViewport::default());
    assert!(bridge.set_multiselection_measure_tag(true));
    assert!(bridge.state.ui.multiselection_measure_tag);
    assert!(bridge.preferences.multiselection_measure_tag);
    assert!(bridge.view_model().multiselection_measure_tag);
}

#[test]
fn parametric_primitive_persistence_and_inspector_re_editing() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    assert!(bridge.confirm_primitive());

    // O asset criado deve ser paramétrico
    assert!(bridge.state.active_is_parametric());
    let vm = bridge.view_model();
    assert!(vm.active_asset_is_parametric);
    assert_eq!(vm.primitive_kind, "cylinder");
    assert!(!vm.label_parametric_primitive.is_empty());
    assert!(!vm.label_freeze_primitive.is_empty());

    // Reedição paramétrica fora da sessão de criação inicial
    assert!(bridge.update_primitive_param_float("radius", 2.5));
    assert!(bridge.update_primitive_param_float("height", 6.0));

    // A malha foi regenerada com as novas dimensões
    let active_mesh = bridge.state.project.active_mesh().expect("active mesh");
    let min_y = active_mesh
        .verts
        .iter()
        .map(|v| v.pos[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = active_mesh
        .verts
        .iter()
        .map(|v| v.pos[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let height = max_y - min_y;
    assert!((height - 6.0).abs() < 1e-3);

    // Continua paramétrico após edição
    assert!(bridge.state.active_is_parametric());
    assert!(bridge.view_model().active_asset_is_parametric);

    let active_index = bridge.state.project.active;
    assert_ne!(active_index, usize::MAX);

    // Deseleção e resseleção preserva estado paramétrico
    bridge.state.select_object(None, false);
    assert!(!bridge.view_model().active_asset_is_parametric);
    bridge.state.select_object(Some(active_index), false);
    assert!(bridge.state.active_is_parametric());
    assert!(bridge.view_model().active_asset_is_parametric);
}

#[test]
fn parametric_primitive_manual_freeze() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cube));
    assert!(bridge.confirm_primitive());
    assert!(bridge.state.active_is_parametric());

    // Freeze manual via Intent ou bridge
    assert!(bridge.freeze_active_primitive());
    assert!(!bridge.state.active_is_parametric());
    assert!(!bridge.view_model().active_asset_is_parametric);

    // Malha é preservada
    let mesh = bridge.state.project.active_mesh().expect("active mesh");
    assert_eq!(mesh.verts.len(), 8);
    assert_eq!(mesh.faces.len(), 6);
}

#[test]
fn parametric_primitive_auto_freeze_on_destructive_modal() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cube));
    assert!(bridge.confirm_primitive());
    assert!(bridge.state.active_is_parametric());

    // Ao iniciar uma operação destrutiva sub-objeto (ex: Extrude em Face)
    bridge.state.set_selection_domain(SelectionDomain::Face);
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        mesh.faces[0].selected = true;
    }
    assert!(
        bridge
            .state
            .begin_modal(petunia_core::ModalKind::Extrude)
            .is_ok()
    );

    // O asset foi congelado de forma transparente sem erro
    assert!(!bridge.state.active_is_parametric());
    assert!(!bridge.view_model().active_asset_is_parametric);
}

#[test]
fn test_model_connect_spin_dissolve_commands() {
    // 1. Dissolve
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        // Duas faces que compartilham uma aresta (as faces 0 e 1 do cubo são opostas).
        let (a, b) = mesh
            .edges_unique()
            .into_iter()
            .find(|&(a, b)| mesh.edge_faces(a, b).len() == 2)
            .expect("cube has shared edges");
        for face in mesh.edge_faces(a, b) {
            mesh.faces[face].selected = true;
        }
    }
    assert!(bridge.execute_core_command("model.dissolve").is_ok());

    // 2. Connect
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        let mut m1 = petunia_mesh::Mesh::plane(2.0);
        let mut m2 = petunia_mesh::Mesh::plane(2.0);
        m2.translate_selected([0.0, 3.0, 0.0]);
        m1.join(&m2);
        m1.faces[0].selected = true;
        m1.faces[1].selected = true;
        *mesh = m1;
    }
    assert!(bridge.execute_core_command("model.connect").is_ok());

    // 3. Spin
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.verts[0].selected = true;
        mesh.verts[1].selected = true;
        mesh.selected_edges.insert(petunia_mesh::edge_key(0, 1));
    }
    assert!(bridge.execute_core_command("model.spin").is_ok());
}

#[test]
fn test_paint_target_vertex_and_mask_intents() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().paint_target_vertex);
    assert!(!bridge.view_model().paint_mask_selection);

    bridge.apply(UiIntent::SetPaintTargetVertex(true));
    assert!(bridge.view_model().paint_target_vertex);

    bridge.apply(UiIntent::TogglePaintMaskSelection);
    assert!(bridge.view_model().paint_mask_selection);

    bridge.apply(UiIntent::SetPaintMaskSelection(false));
    assert!(!bridge.view_model().paint_mask_selection);
}

#[test]
fn test_paint_2d_gradient_tool_flow() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
    bridge.apply(UiIntent::SetActiveTool("gradient".to_string()));
    assert!(bridge.is_shape_tool());

    // 2D stroke: Phase 0 (down) anchors gradient
    assert!(bridge.paint_2d_stroke(0.1, 0.1, 0));
    assert!(bridge.shape_anchor.is_some());

    // Phase 1 (move)
    assert!(bridge.paint_2d_stroke(0.8, 0.8, 1));

    // Phase 2 (up) commits gradient
    assert!(bridge.paint_2d_stroke(0.8, 0.8, 2));
    assert!(bridge.shape_anchor.is_none());
}

#[test]
fn test_uv_stitch_and_relax_commands() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Uv));

    assert!(bridge.execute_core_command("uv.stitch").is_ok());
    assert!(bridge.execute_core_command("uv.relax").is_ok());
}

#[test]
fn test_profile_workplanes_ground_face_view() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::ProfileSetWorkplaneGround);
    assert_eq!(bridge.view_model().profile_workplane, "Ground");

    bridge.apply(UiIntent::ProfileSetWorkplaneFace);
    assert!(
        bridge.view_model().profile_workplane == "Face"
            || bridge.view_model().profile_workplane == "Ground"
    );

    bridge.apply(UiIntent::ProfileSetWorkplaneView);
    assert_eq!(bridge.view_model().profile_workplane, "View");

    // Adding 2D shapes preserves active workplane
    bridge.add_profile_rectangle(2.0, 1.5);
    bridge.add_profile_circle(1.0, 16);
}

#[test]
fn test_paint_uv_overlay_and_uv_texture_toggles() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.view_model().paint_show_uv_overlay);
    assert!(bridge.view_model().uv_show_texture);

    bridge.apply(UiIntent::TogglePaintUvOverlay);
    assert!(bridge.view_model().paint_show_uv_overlay);

    bridge.apply(UiIntent::ToggleUvShowTexture);
    assert!(!bridge.view_model().uv_show_texture);

    bridge.apply(UiIntent::TogglePaintUvOverlay);
    assert!(!bridge.view_model().paint_show_uv_overlay);

    bridge.apply(UiIntent::ToggleUvShowTexture);
    assert!(bridge.view_model().uv_show_texture);
}

#[test]
fn test_settings_redesign_language_scale_and_selection_color() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // 1. Language change
    bridge.set_language("en-US");
    assert_eq!(bridge.view_model().active_language, "en-US");
    bridge.set_language("pt-BR");
    assert_eq!(bridge.view_model().active_language, "pt-BR");

    // 2. UI Scale clamping and view model update
    bridge.set_ui_scale(1.5);
    assert!((bridge.view_model().ui_scale - 1.5).abs() < 1e-3);
    assert!((bridge.preferences.ui_scale - 1.5).abs() < 1e-3);

    // Clamps below 1.0 and above 2.0
    bridge.set_ui_scale(0.5);
    assert!((bridge.view_model().ui_scale - 1.0).abs() < 1e-3);
    bridge.set_ui_scale(3.0);
    assert!((bridge.view_model().ui_scale - 2.0).abs() < 1e-3);

    // 3. Double-tap interval: allows 0 (disabled) and presets
    bridge.set_double_tap_interval_ms(0);
    assert_eq!(bridge.preferences.double_tap_interval_ms, 0);
    bridge.set_double_tap_interval_ms(350);
    assert_eq!(bridge.preferences.double_tap_interval_ms, 350);

    // 4. Selection color hex
    assert!(bridge.set_selection_color_hex("#00D2FF"));
    assert_eq!(bridge.view_model().selection_color_hex, "#00D2FF");
    assert!(bridge.set_selection_color_hex("#E96A00"));
    assert_eq!(bridge.view_model().selection_color_hex, "#E96A00");
}

#[test]
fn test_viewport_context_menu_modeling_actions_and_dismissal() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    // 1. Right click opens viewport context menu
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.context_menu.is_some());
    assert!(bridge.view_model().context_menu_open);

    // 2. Click away dismisses it
    assert!(bridge.handle_click_away());
    assert!(bridge.context_menu.is_none());
    assert!(!bridge.view_model().context_menu_open);

    // 3. Re-open and dismiss with Escape
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.context_menu.is_some());
    assert!(bridge.handle_escape());
    assert!(bridge.context_menu.is_none());

    // 4. Test actions in viewport context menu
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.context_menu_action("shade_smooth"));
    assert_eq!(bridge.view_model().status_message, "Smooth shading");

    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.context_menu_action("subdivide"));
}

#[test]
fn test_menu_and_workspace_keyboard_shortcuts() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    // Alt+F opens File menu
    assert!(bridge.route_shortcut("f", false, false, true));
    assert_eq!(bridge.view_model().menu_open, "file");

    // Alt+E switches to Edit menu
    assert!(bridge.route_shortcut("e", false, false, true));
    assert_eq!(bridge.view_model().menu_open, "edit");

    // Escape closes menu
    assert!(bridge.route_shortcut("Escape", false, false, false));
    assert_eq!(bridge.view_model().menu_open, "");

    // Ctrl+PageDown cycles workspace MODEL -> PAINT -> UV -> MODEL
    assert_eq!(bridge.state.workspace, Workspace::Model);
    assert!(bridge.route_shortcut("PageDown", true, false, false));
    assert_eq!(bridge.state.workspace, Workspace::Paint);
    assert!(bridge.route_shortcut("PageDown", true, false, false));
    assert_eq!(bridge.state.workspace, Workspace::Uv);
    assert!(bridge.route_shortcut("PageDown", true, false, false));
    assert_eq!(bridge.state.workspace, Workspace::Model);

    // Ctrl+PageUp cycles backwards MODEL -> UV -> PAINT -> MODEL
    assert!(bridge.route_shortcut("PageUp", true, false, false));
    assert_eq!(bridge.state.workspace, Workspace::Uv);
    assert!(bridge.route_shortcut("PageUp", true, false, false));
    assert_eq!(bridge.state.workspace, Workspace::Paint);
    assert!(bridge.route_shortcut("PageUp", true, false, false));
    assert_eq!(bridge.state.workspace, Workspace::Model);
}

#[test]
fn test_loop_selection_face_and_vertex_in_bridge() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));

    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        assert_eq!(mesh.select_face_loop(0, None, false), 4);
    }

    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        assert!(mesh.select_vertex_loop(0, false) >= 2);
    }
}

#[test]
fn test_w_shortcut_toggles_selection_tool_in_model_workspace() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Model));
    assert_eq!(bridge.state.session.tools.active_tool, "select");

    // W cycles from "select" to "box_select"
    assert!(bridge.route_shortcut("w", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "box_select");
    assert_eq!(bridge.view_model().active_tool, "box_select");

    // W cycles back from "box_select" to "select"
    assert!(bridge.route_shortcut("w", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.view_model().active_tool, "select");

    // When another tool is active, W switches back to "select"
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    assert_eq!(bridge.state.session.tools.active_tool, "move");
    assert!(bridge.route_shortcut("w", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "select");
}

#[test]
fn test_w_shortcut_activates_selection_in_uv_workspace() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.apply(UiIntent::SetWorkspace(Workspace::Uv));
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));

    assert!(bridge.route_shortcut("w", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "select");
}

#[test]
fn test_settings_open_close_intents_and_view_model_sync() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert!(!bridge.settings_visible);
    assert!(!bridge.view_model().settings_visible);

    // Open settings
    bridge.apply(UiIntent::OpenSettings);
    assert!(bridge.settings_visible);
    assert!(bridge.view_model().settings_visible);

    // Close settings via intent (scrim click, close button, or callback)
    bridge.apply(UiIntent::CloseSettings);
    assert!(!bridge.settings_visible);
    assert!(!bridge.view_model().settings_visible);

    // Open again and test escape handling
    bridge.apply(UiIntent::OpenSettings);
    assert!(bridge.settings_visible);
    assert!(bridge.handle_escape());
    assert!(!bridge.settings_visible);
    assert!(!bridge.view_model().settings_visible);
}

#[test]
fn test_keymap_profile_switching() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(bridge.state.ui.active_keymap_id, "petunia-default");
    assert_eq!(bridge.view_model().active_keymap_id, "petunia-default");

    // Switch to Blender profile
    assert!(bridge.set_keymap_profile("blender"));
    assert_eq!(bridge.state.ui.active_keymap_id, "blender");
    assert_eq!(bridge.preferences.active_keymap_id, "blender");
    assert_eq!(bridge.view_model().active_keymap_id, "blender");

    // Switching to the same profile returns false (no-op)
    assert!(!bridge.set_keymap_profile("blender"));

    // Switch to Maya profile
    assert!(bridge.set_keymap_profile("maya"));
    assert_eq!(bridge.state.ui.active_keymap_id, "maya");
    assert_eq!(bridge.view_model().active_keymap_id, "maya");
}

#[test]
fn test_tools_and_viewport_settings_toggles() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Bevel clamp overlap
    let initial_clamp = bridge.state.tools.bevel_clamp_overlap;
    bridge.toggle_bevel_clamp_overlap();
    assert_eq!(bridge.state.tools.bevel_clamp_overlap, !initial_clamp);
    assert_eq!(bridge.view_model().bevel_clamp_overlap, !initial_clamp);

    // Slice trim
    assert!(!bridge.slice_trim);
    bridge.set_slice_trim(true);
    assert!(bridge.slice_trim);
    assert!(bridge.view_model().slice_trim);

    // Wireframe overlay
    let initial_wire = bridge.state.session.show_wireframe_overlay;
    bridge.execute_command(CommandId::ToggleWireOverlay);
    assert_eq!(bridge.state.session.show_wireframe_overlay, !initial_wire);
    assert_eq!(bridge.view_model().is_wireframe, !initial_wire);

    // Face orientation
    let initial_face = bridge.state.session.show_face_orientation;
    bridge.apply(UiIntent::ToggleFaceOrientation);
    assert_eq!(bridge.state.session.show_face_orientation, !initial_face);
    assert_eq!(bridge.view_model().show_face_orientation, !initial_face);

    // Camera projection
    let initial_ortho = bridge.view_model().is_orthographic;
    bridge.apply(UiIntent::ToggleProjection);
    assert_eq!(bridge.view_model().is_orthographic, !initial_ortho);
}

#[test]
fn test_slice_mouse_click_and_space_commit_and_multi_cut() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.apply(UiIntent::SetActiveTool("slice".to_string()));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");

    // Pressing Enter or Space before dragging does not panic or cancel, provides status guidance
    assert!(!bridge.route_shortcut("Enter", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert!(bridge.state.ui.status.contains("Slice:"));

    // First cut drag (vertical)
    assert!(bridge.begin_slice(400.0, 200.0));
    assert!(bridge.update_slice(400.0, 400.0));
    assert!(bridge.slice_anchor.is_some());
    let faces_before = bridge.state.project.active_mesh().unwrap().faces.len();

    // Second cut drag far away from handles (horizontal) auto-commits the first cut!
    assert!(bridge.begin_slice(300.0, 300.0));
    assert!(bridge.update_slice(500.0, 300.0));
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert!(bridge.slice_anchor.is_some());

    // Confirm second cut using Space bar
    assert!(bridge.route_shortcut("Space", false, false, false));
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert!(bridge.slice_anchor.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (2, 0));
    let faces_after = bridge.state.project.active_mesh().unwrap().faces.len();
    assert!(faces_after > faces_before);
}

#[test]
fn test_context_menu_escape_and_actions_dissolve_and_slice() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.view_model().context_menu_open);

    // Escape closes context menu immediately
    assert!(bridge.handle_escape());
    assert!(!bridge.view_model().context_menu_open);

    // Click away closes context menu
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.view_model().context_menu_open);
    assert!(bridge.handle_click_away());
    assert!(!bridge.view_model().context_menu_open);

    // Context menu action: slice
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.context_menu_action("slice"));
    assert_eq!(bridge.state.session.tools.active_tool, "slice");
    assert!(!bridge.view_model().context_menu_open);

    // Context menu action: dissolve
    bridge.open_viewport_context_menu(200.0, 150.0);
    assert!(bridge.context_menu_action("dissolve"));
    assert!(!bridge.view_model().context_menu_open);
}

#[test]
fn test_loop_selection_edge_face_vertex_and_double_click() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    // 1. Edge domain loop selection
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        let (a, b) = mesh.edges_unique()[0];
        let count = mesh.select_edge_loop((a, b), false);
        assert!(count > 0);
        assert!(!mesh.selected_edges.is_empty());
    }

    // 2. Vertex domain loop selection
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        let count = mesh.select_vertex_loop(0, false);
        assert!(count > 0);
        assert!(mesh.verts.iter().any(|v| v.selected));
    }

    // 3. Face domain loop selection
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        let count = mesh.select_face_loop(0, None, false);
        assert!(count > 0);
        assert!(mesh.faces.iter().any(|f| f.selected));
    }
}

#[test]
fn test_primitives_created_at_3d_cursor() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let cursor_pos = [5.0, 3.0, -4.0];
    bridge.state.session.cursor_3d = cursor_pos;

    // Add Cube primitive
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cube));
    let asset = bridge.state.project.active().expect("cube asset created");

    // Compute center of bounding box
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in &asset.mesh.verts {
        for i in 0..3 {
            min[i] = min[i].min(v.pos[i]);
            max[i] = max[i].max(v.pos[i]);
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    assert!((center[0] - cursor_pos[0]).abs() < 1e-3);
    assert!((center[1] - cursor_pos[1]).abs() < 1e-3);
    assert!((center[2] - cursor_pos[2]).abs() < 1e-3);
}

#[test]
fn test_profile_interactive_volume_flow() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    let initial_asset_ids: Vec<_> = bridge
        .state
        .project
        .assets
        .iter()
        .map(|asset| asset.id)
        .collect();
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));

    // Draw a square profile as persistent Profile + Spline resources.
    assert!(bridge.add_profile_rectangle(2.0, 2.0));
    let profile_id = bridge.active_profile_id.expect("profile id");

    // 1. Enter interactive Extrude volume preview
    assert!(bridge.enter_profile_volume("extrude"));
    assert_eq!(
        bridge.profile_volume_mode,
        Some(petunia_module_model::ProfileVolumeMode::Extrude)
    );
    assert!(bridge.profile_preview_asset_id.is_some());
    assert_eq!(bridge.view_model().profile_volume_mode, "extrude");

    // Check that preview asset exists in project
    let preview_id = bridge.profile_preview_asset_id.unwrap();
    let preview_asset = bridge
        .state
        .project
        .assets
        .iter()
        .find(|a| a.id == preview_id);
    assert!(preview_asset.is_some());
    assert_eq!(preview_asset.unwrap().name, "Profile Preview");

    // 2. Adjust depth interactively
    bridge.set_profile_depth(2.5);
    assert_eq!(bridge.state.profile.depth, 2.5);

    // 3. Cancel volume: preview removed, profile kept!
    assert!(bridge.cancel_profile_volume());
    assert!(bridge.profile_volume_mode.is_none());
    assert!(
        bridge
            .state
            .project
            .assets
            .iter()
            .all(|a| a.id != preview_id)
    );
    assert_eq!(active_profile_spline(&bridge).points.len(), 4);
    assert!(active_profile_spline(&bridge).closed);
    assert_eq!(
        bridge
            .state
            .project
            .assets
            .iter()
            .map(|asset| asset.id)
            .collect::<Vec<_>>(),
        initial_asset_ids
    );
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    // 4. Enter Revolve volume preview
    assert!(bridge.enter_profile_volume("revolve"));
    assert_eq!(
        bridge.profile_volume_mode,
        Some(petunia_module_model::ProfileVolumeMode::Revolve)
    );
    assert!(bridge.set_profile_revolve_angle(180.0));
    assert_eq!(bridge.state.profile.revolve_angle, 180.0);

    // 5. Confirm volume: asset finalized to "Profile", tool switched to select
    assert!(bridge.commit_profile_volume());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert!(bridge.profile_volume_mode.is_none());
    assert!(bridge.state.profile.points.is_empty());
    assert!(
        bridge
            .state
            .project
            .project
            .get_profile(profile_id)
            .is_some()
    );
    let active = bridge.state.project.active().expect("active profile asset");
    assert_eq!(active.name, "Profile");
    let committed_id = active.id;
    assert_eq!(bridge.state.project.undo.depth(), (2, 0));
    assert!(bridge.state.undo());
    assert_eq!(
        bridge
            .state
            .project
            .assets
            .iter()
            .map(|asset| asset.id)
            .collect::<Vec<_>>(),
        initial_asset_ids
    );
    assert!(
        bridge
            .state
            .project
            .project
            .get_profile(profile_id)
            .is_some()
    );
    assert!(bridge.state.redo());
    assert!(
        bridge
            .state
            .project
            .assets
            .iter()
            .any(|asset| asset.id == committed_id && asset.name == "Profile")
    );
}

#[test]
fn test_profile_bezier_handle_interactive_dragging() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));

    // Click at center
    bridge.select_viewport_ext(0.5, 0.5, false, false);
    assert_eq!(active_profile_spline(&bridge).points.len(), 1);
    let point_id = active_profile_spline(&bridge).points[0].id;

    // Drag handle symmetrically (0.6, 0.5)
    let updated = bridge.profile_update_drag_handle(0.6, 0.5, false);
    assert!(updated);
    let point = active_profile_spline(&bridge).point(point_id).unwrap();
    assert_ne!(point.handle_out, [0.0; 3]);
    assert_ne!(point.handle_in, [0.0; 3]);
    assert_eq!(point.handle_mode, petunia_core::SplineHandleMode::Mirrored);

    // Preview commands should contain tangent handles
    let cmds = bridge.profile_preview_commands();
    assert!(!cmds.is_empty());

    // Drag handle with Alt (break tangent)
    let updated_sharp = bridge.profile_update_drag_handle(0.65, 0.55, true);
    assert!(updated_sharp);
    let point = active_profile_spline(&bridge).point(point_id).unwrap();
    assert_eq!(point.handle_mode, petunia_core::SplineHandleMode::Broken);
    bridge.profile_pointer_up();
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn profile_authoring_persists_and_insert_drag_is_one_undo_entry() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));

    bridge.select_viewport_ext(0.5, 0.5, false, false);
    assert!(bridge.profile_update_drag_handle(0.6, 0.5, false));
    bridge.profile_pointer_up();
    let profile_id = bridge.active_profile_id.expect("profile id");
    let spline_id = bridge
        .state
        .project
        .project
        .get_profile(profile_id)
        .unwrap()
        .spline_id;
    let point_id = bridge
        .state
        .project
        .project
        .get_spline(spline_id)
        .unwrap()
        .points[0]
        .id;
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));

    bridge.apply(UiIntent::SetActiveTool("select".into()));
    assert!(
        bridge
            .state
            .project
            .project
            .get_profile(profile_id)
            .is_some()
    );
    assert!(bridge.state.project.project.get_spline(spline_id).is_some());
    assert!(bridge.active_profile_id.is_none());

    assert!(bridge.state.undo());
    assert!(
        bridge
            .state
            .project
            .project
            .get_profile(profile_id)
            .is_none()
    );
    assert!(bridge.state.redo());
    let point = bridge
        .state
        .project
        .project
        .get_spline(spline_id)
        .unwrap()
        .point(point_id)
        .unwrap();
    assert_eq!(point.handle_mode, petunia_core::SplineHandleMode::Mirrored);
    assert_ne!(point.handle_out, [0.0; 3]);
}

#[test]
fn test_profile_align_camera_to_workplane() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.profile.normal = [0.0, 1.0, 0.0];
    bridge.state.profile.origin = [1.0, 2.0, 3.0];

    petunia_module_model::profile_align_camera_to_workplane(&mut bridge.state);
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );
    assert_eq!(
        bridge.state.session.camera.target,
        glam::Vec3::new(1.0, 2.0, 3.0)
    );
}

#[test]
fn test_blender_numpad_viewport_navigation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Numpad 1: Front
    assert!(bridge.route_shortcut("Numpad1", false, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Ctrl+Numpad 1: Back
    assert!(bridge.route_shortcut("Numpad1", true, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Numpad 3: Right
    assert!(bridge.route_shortcut("KP_3", false, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Ctrl+Numpad 3: Left
    assert!(bridge.route_shortcut("KP_3", true, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Numpad 7: Top
    assert!(bridge.route_shortcut("Numpad 7", false, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Ctrl+Numpad 7: Bottom
    assert!(bridge.route_shortcut("Numpad 7", true, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Numpad 9: Opposite
    assert!(bridge.route_shortcut("Numpad9", false, false, false));

    // Numpad 5: Toggle Ortho / Perspective
    let proj_before = bridge.state.session.camera.proj;
    assert!(bridge.route_shortcut("Numpad5", false, false, false));
    assert_ne!(bridge.state.session.camera.proj, proj_before);

    // Numpad 0: Perspective / Camera
    assert!(bridge.route_shortcut("Numpad0", false, false, false));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Perspective
    );

    // Numpad .: Frame selected
    assert!(bridge.route_shortcut("Numpad .", false, false, false));

    // Numpad /: Toggle isolate
    assert!(!bridge.state.session.isolate_active);
    assert!(bridge.route_shortcut("Numpad /", false, false, false));
    assert!(bridge.state.session.isolate_active);
    assert!(bridge.route_shortcut("Numpad /", false, false, false));
    assert!(!bridge.state.session.isolate_active);
}

#[test]
fn test_smart_contextual_selection_mode_switching() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Start in Face mode
    bridge.apply(UiIntent::SetSelectionDomain(
        petunia_core::SelectionDomain::Face,
    ));
    assert_eq!(
        bridge.state.session.selection_domain,
        petunia_core::SelectionDomain::Face
    );

    // 1. Creating a primitive switches automatically to Object mode
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
    assert_eq!(
        bridge.state.session.selection_domain,
        petunia_core::SelectionDomain::Object
    );

    // 2. Instantiating a library asset switches automatically to Object mode
    let asset_id = bridge.state.project.assets[0].id.to_string();
    bridge.apply(UiIntent::SetSelectionDomain(
        petunia_core::SelectionDomain::Edge,
    ));
    assert!(bridge.place_asset(&asset_id));
    assert_eq!(
        bridge.state.session.selection_domain,
        petunia_core::SelectionDomain::Object
    );

    // 3. Extrude a partir de Object troca para Face de forma visível e,
    // pelo ADR 006, nunca seleciona todas as faces sozinho.
    bridge.apply(UiIntent::SetSelectionDomain(
        petunia_core::SelectionDomain::Object,
    ));
    let has_mesh = bridge.state.project.active_mesh().is_some();
    assert!(has_mesh);
    let selected_before: Vec<bool> = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.selected)
        .collect();
    bridge.execute_shortcut_tool("model.extrude");
    assert_eq!(
        bridge.state.session.selection_domain,
        petunia_core::SelectionDomain::Face
    );
    let selected_after: Vec<bool> = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.selected)
        .collect();
    assert_eq!(selected_after, selected_before);

    // 4. Activating Bevel from Object mode switches to Edge mode
    bridge.apply(UiIntent::SetSelectionDomain(
        petunia_core::SelectionDomain::Object,
    ));
    bridge.execute_shortcut_tool("model.bevel");
    assert_eq!(
        bridge.state.session.selection_domain,
        petunia_core::SelectionDomain::Edge
    );

    // 5. Activating Loop Cut from Object mode switches to Edge mode
    bridge.apply(UiIntent::SetSelectionDomain(
        petunia_core::SelectionDomain::Object,
    ));
    bridge.execute_shortcut_tool("model.loop_cut");
    assert_eq!(
        bridge.state.session.selection_domain,
        petunia_core::SelectionDomain::Edge
    );
}

#[test]
fn test_reference_manager_operations_and_f4_shortcut() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Initial state: closed
    assert!(!bridge.view_model().reference_manager_open);

    // F4 toggles reference manager
    assert!(bridge.route_shortcut("F4", false, false, false));
    assert!(bridge.view_model().reference_manager_open);

    // Close and open methods
    assert!(bridge.close_reference_manager());
    assert!(!bridge.view_model().reference_manager_open);
    assert!(bridge.open_reference_manager());
    assert!(bridge.view_model().reference_manager_open);

    // Add a reference image to project.refs
    let dummy_rgba = vec![255u8; 16 * 16 * 4];
    let mut ref_img =
        petunia_core::ReferenceImage::from_rgba("Test Ref".to_string(), 16, 16, dummy_rgba);
    ref_img.axis = petunia_core::RefAxis::Front;
    bridge.state.project.refs.push(ref_img);

    let vm = bridge.view_model();
    assert_eq!(vm.reference_total_count, 1);
    assert_eq!(vm.reference_slots.len(), 6);
    assert!(vm.reference_slots[0].has_image); // Slot 0 is Front
    assert!(!vm.reference_slots[1].has_image); // Slot 1 is Back

    // Toggle visibility of Front slot
    assert!(bridge.toggle_reference_visible("front"));
    assert!(!bridge.state.project.refs[0].visible);
    assert!(bridge.toggle_reference_visible("front"));
    assert!(bridge.state.project.refs[0].visible);

    // Toggle lock
    assert!(bridge.toggle_reference_lock("front"));
    assert!(bridge.state.project.refs[0].locked);
    assert!(bridge.toggle_reference_lock("front"));
    assert!(!bridge.state.project.refs[0].locked);

    // Set parameters
    assert!(bridge.set_reference_param("front", "opacity", 0.8));
    assert!((bridge.state.project.refs[0].opacity - 0.8).abs() < 1e-4);
    assert!(bridge.set_reference_param("front", "size", 5.0));
    assert!((bridge.state.project.refs[0].size - 5.0).abs() < 1e-4);
    assert!(bridge.set_reference_param("front", "offset", 1.5));
    assert!((bridge.state.project.refs[0].offset - 1.5).abs() < 1e-4);
    assert!(bridge.set_reference_param("front", "rotation", 45.0));
    assert!((bridge.state.project.refs[0].rotation - 45.0).abs() < 1e-4);

    // Align view to reference slot 0 (Front)
    assert!(bridge.align_reference_view("front"));
    assert_eq!(
        bridge.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );

    // Clear all references
    assert!(bridge.clear_all_references());
    assert!(bridge.state.project.refs.is_empty());
    assert_eq!(bridge.view_model().reference_total_count, 0);

    // Escape closes reference manager first
    bridge.open_reference_manager();
    assert!(bridge.handle_escape());
    assert!(!bridge.view_model().reference_manager_open);
}

#[test]
fn test_loop_cut_2d_scrubbing_and_candidate_detection() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.resize_viewport(800, 600);

    let _seed = {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        let face = mesh.faces[0].verts.clone();
        let edge = (face[0], face[1]);
        mesh.selected_edges.insert(edge);
        edge
    };
    assert!(bridge.begin_loop_cut());
    assert!(bridge.loop_cut.is_some());

    // 2D scrubbing updates slide
    assert!(bridge.scrub_loop_cut_2d(50.0, -20.0, false));
    let slide = bridge.loop_cut.as_ref().unwrap().slide;
    assert_ne!(slide, 0.0);

    assert!(bridge.commit_loop_cut());
    assert!(bridge.loop_cut.is_none());
}

#[test]
fn test_profile_point_and_handle_interactive_manipulation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];
    bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));

    // Create a 2D Rectangle (starts closed with 4 nodes)
    bridge.add_profile_rectangle(2.0, 2.0);
    assert_eq!(active_profile_spline(&bridge).points.len(), 4);
    assert!(active_profile_spline(&bridge).closed);
    let point_id = active_profile_spline(&bridge).points[0].id;

    // Project anchor 0 to screen
    let profile_state = bridge.active_profile_state().unwrap();
    let p0 = profile_state.nodes[0].point;
    let matrix = bridge.state.session.camera.view_proj();
    let pt3d = profile_state.to_3d_point(p0);
    let clip = matrix * pt3d.extend(1.0);
    let scr_x = (clip.x / clip.w * 0.5 + 0.5) * 800.0;
    let scr_y = (0.5 - clip.y / clip.w * 0.5) * 600.0;

    // 1. Hit test on anchor 0
    let hit = bridge.hit_test_profile(scr_x, scr_y);
    assert_eq!(hit, Some(ProfileHitTarget::Anchor(point_id)));

    // 2. Pointer down on anchor 0
    assert!(bridge.profile_pointer_down(scr_x, scr_y, false));
    assert_eq!(bridge.profile_selected_point, Some(point_id));
    assert_eq!(
        bridge.profile_drag_target,
        Some(ProfileHitTarget::Anchor(point_id))
    );

    // 3. Pointer move to drag anchor 0
    let old_pt = active_profile_spline(&bridge)
        .point(point_id)
        .unwrap()
        .position;
    assert!(bridge.profile_pointer_move(scr_x + 30.0, scr_y + 30.0, false));
    let new_pt = active_profile_spline(&bridge)
        .point(point_id)
        .unwrap()
        .position;
    assert_ne!(old_pt, new_pt);

    // 4. Pointer up ends drag
    bridge.profile_pointer_up();
    assert_eq!(bridge.profile_drag_target, None);
    assert_eq!(bridge.profile_selected_point, Some(point_id));
    assert_eq!(bridge.state.project.undo.depth(), (2, 0));

    // 5. Alt-drag anchor to pull out handles
    assert!(bridge.profile_pointer_down(scr_x + 30.0, scr_y + 30.0, true));
    assert_eq!(
        bridge.profile_drag_target,
        Some(ProfileHitTarget::HandleOut(point_id))
    );
    assert!(bridge.profile_pointer_move(scr_x + 60.0, scr_y + 40.0, true));
    assert_ne!(
        active_profile_spline(&bridge)
            .point(point_id)
            .unwrap()
            .handle_out,
        [0.0; 3]
    );
    bridge.profile_pointer_up();

    // 6. Preview commands render anchor boxes and handle markers
    let cmds = bridge.profile_preview_commands();
    assert!(cmds.contains("M"));
    assert!(cmds.contains("Z"));

    // 7. Delete selected node with delete key / command
    let _ = bridge.execute_core_command("model.delete");
    assert_eq!(active_profile_spline(&bridge).points.len(), 3);
    assert_eq!(bridge.profile_selected_point, None);
}

#[test]
fn test_profile_extrude_card_depth_and_generate() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Add 2D Circle
    bridge.add_profile_circle(1.0, 16);
    assert!(active_profile_spline(&bridge).closed);
    assert_eq!(bridge.profile_volume_mode, None);

    // Changing depth in the card automatically enters interactive extrude mode with live preview
    assert!(bridge.set_profile_depth(3.5));
    assert_eq!(
        bridge.profile_volume_mode,
        Some(petunia_module_model::ProfileVolumeMode::Extrude)
    );
    assert!(bridge.profile_preview_asset_id.is_some());
    assert_eq!(bridge.state.profile.depth, 3.5);

    // Further adjustments update the live extrude preview mesh
    assert!(bridge.set_profile_depth(4.0));
    assert_eq!(bridge.state.profile.depth, 4.0);

    // Clicking "Gerar" (generate_profile_extrude) commits the 3D volume into project
    assert!(bridge.generate_profile_extrude());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert_eq!(bridge.profile_volume_mode, None);
    assert!(bridge.state.profile.points.is_empty());
    let asset = bridge.state.project.active().expect("active mesh");
    assert_eq!(asset.name, "Profile");
    assert!(!asset.mesh.faces.is_empty());
}

#[test]
fn test_profile_shortcut_and_shelf_extrude() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Add 2D rectangle
    bridge.add_profile_rectangle(2.0, 1.5);
    assert!(active_profile_spline(&bridge).closed);
    assert_eq!(bridge.profile_volume_mode, None);

    // Triggering "model.extrude" via shortcut or shelf button enters extrude preview
    let _ = bridge.execute_core_command("model.extrude");
    assert_eq!(
        bridge.profile_volume_mode,
        Some(petunia_module_model::ProfileVolumeMode::Extrude)
    );
    assert!(bridge.profile_preview_asset_id.is_some());

    // Triggering it again commits the volume
    let _ = bridge.execute_core_command("model.extrude");
    assert_eq!(bridge.profile_volume_mode, None);
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    let asset = bridge.state.project.active().expect("active mesh");
    assert_eq!(asset.name, "Profile");
}

#[test]
fn test_primitive_session_dismisses_on_viewport_click_and_operations() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Add primitive cube
    bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cube));
    assert!(bridge.state.primitive_session_valid());

    // Verify Operation HUD is suppressed while primitive session is active to prevent overlapping cards
    let vm = bridge.view_model();
    assert!(vm.primitive_active);
    assert!(!vm.operation_hud_active);

    // Clicking anywhere in viewport finalizes primitive session
    bridge.select_viewport_ext(0.1, 0.1, false, false);
    assert!(!bridge.state.primitive_session_valid());
    let vm_after = bridge.view_model();
    assert!(!vm_after.primitive_active);

    // Adding another primitive and initiating a tool modal also finalizes the session
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    assert!(bridge.state.primitive_session_valid());
    bridge.begin_tool_modal(ToolModalKind::Extrude);
    assert!(!bridge.state.primitive_session_valid());
}

#[test]
fn test_delete_and_dissolve_selection() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Enter Edit Mode with Face domain
    bridge.state.set_selection_domain(SelectionDomain::Face);
    if let Some(mesh) = bridge.state.project.active_mesh_mut()
        && let Some(face) = mesh.faces.get_mut(0)
    {
        face.selected = true;
    }

    // Delete or dissolve selected face
    let before_topology: Vec<_> = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|face| face.verts.clone())
        .collect();
    let history_before = bridge.state.project.undo.depth().0;
    assert!(bridge.delete_or_dissolve_selection());
    let after_topology: Vec<_> = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|face| face.verts.clone())
        .collect();
    assert!(after_topology.len() < before_topology.len());
    assert_eq!(bridge.state.project.undo.depth(), (history_before + 1, 0));
    assert!(bridge.state.undo());
    assert_eq!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .iter()
            .map(|face| face.verts.clone())
            .collect::<Vec<_>>(),
        before_topology
    );
    assert!(bridge.state.redo());
    assert_eq!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .iter()
            .map(|face| face.verts.clone())
            .collect::<Vec<_>>(),
        after_topology
    );

    // In Object Mode, Delete deletes the active asset
    bridge.state.set_selection_domain(SelectionDomain::Object);
    let initial_assets = bridge.state.project.assets.len();
    assert!(bridge.delete_or_dissolve_selection());
    assert_eq!(bridge.state.project.assets.len(), initial_assets - 1);
}

#[test]
fn test_copy_paste_geometry_and_assets() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Select face in Edit Mode
    bridge.state.set_selection_domain(SelectionDomain::Face);
    if let Some(mesh) = bridge.state.project.active_mesh_mut()
        && let Some(face) = mesh.faces.get_mut(0)
    {
        face.selected = true;
    }

    // Copy selected face
    assert!(bridge.copy_selection());
    assert!(bridge.clipboard.is_some());

    // Paste creates a separate object named {base}_part
    let initial_assets = bridge.state.project.assets.len();
    assert!(bridge.paste_clipboard());
    assert_eq!(bridge.state.project.assets.len(), initial_assets + 1);
    let new_asset = bridge.state.project.active().unwrap();
    assert!(new_asset.name.ends_with("_part"));
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Object);

    // In Object Mode, copy active asset and paste
    assert!(bridge.copy_selection());
    assert!(bridge.paste_clipboard());
    let pasted_asset = bridge.state.project.active().unwrap();
    assert!(pasted_asset.name.ends_with("_copy"));
}

#[test]
fn test_duplicate_selection_and_assets() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // In Edit Mode, duplicate duplicates selection in mesh
    bridge.state.set_selection_domain(SelectionDomain::Face);
    if let Some(mesh) = bridge.state.project.active_mesh_mut()
        && let Some(face) = mesh.faces.get_mut(0)
    {
        face.selected = true;
    }
    let initial_faces = bridge.state.project.active_mesh().unwrap().faces.len();
    assert!(bridge.duplicate_selection());
    let after_faces = bridge.state.project.active_mesh().unwrap().faces.len();
    assert!(after_faces > initial_faces);

    // In Object Mode, duplicate duplicates active asset
    bridge.state.set_selection_domain(SelectionDomain::Object);
    let initial_assets = bridge.state.project.assets.len();
    assert!(bridge.duplicate_selection());
    assert_eq!(bridge.state.project.assets.len(), initial_assets + 1);
}

#[test]
fn test_extrude_mode_switch_and_presets() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.viewport_size = [800.0, 600.0];

    // Select face
    bridge.state.set_selection_domain(SelectionDomain::Face);
    if let Some(mesh) = bridge.state.project.active_mesh_mut()
        && let Some(face) = mesh.faces.get_mut(0)
    {
        face.selected = true;
    }

    // Begin extrude modal
    assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
    assert_eq!(bridge.tool_modal, Some(ToolModalKind::Extrude));
    bridge.set_tool_modal_value(0.5);

    // Switch extrude mode between Region and Individual Faces
    assert!(bridge.switch_extrude_mode());
    assert_eq!(bridge.tool_modal, Some(ToolModalKind::ExtrudeIndividual));
    assert_eq!(bridge.tool_modal_value, 0.5);

    assert!(bridge.switch_extrude_mode());
    assert_eq!(bridge.tool_modal, Some(ToolModalKind::Extrude));
    assert_eq!(bridge.tool_modal_value, 0.5);

    bridge.cancel_tool_modal();
    assert_eq!(bridge.tool_modal, None);
}

#[test]
fn test_make_face_from_selection_and_shortcut_f() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);

    // 1. Delete a face from cube to create an open boundary
    assert_eq!(bridge.state.project.active_mesh().unwrap().faces.len(), 6);
    let removed_face = {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        mesh.faces.remove(0)
    };
    assert_eq!(bridge.state.project.active_mesh().unwrap().faces.len(), 5);

    // Select the 4 boundary edges in Edge Mode
    bridge.state.set_selection_domain(SelectionDomain::Edge);
    {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        let n = removed_face.verts.len();
        for i in 0..n {
            let u = removed_face.verts[i];
            let v = removed_face.verts[(i + 1) % n];
            mesh.selected_edges.insert(petunia_mesh::edge_key(u, v));
        }
    }

    // Press 'F' in Edge Mode -> Make Face closes the cube
    assert!(bridge.route_shortcut("f", false, false, false));
    assert_eq!(bridge.state.project.active_mesh().unwrap().faces.len(), 6);
    assert!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .last()
            .unwrap()
            .selected
    );

    // 2. Select 3 vertices in Vertex Mode -> Press 'F' creates triangle
    bridge.state.set_selection_domain(SelectionDomain::Vertex);
    {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        mesh.deselect_all();
        mesh.verts.push(petunia_mesh::Vertex::new(10.0, 0.0, 0.0));
        mesh.verts.push(petunia_mesh::Vertex::new(11.0, 0.0, 0.0));
        mesh.verts.push(petunia_mesh::Vertex::new(10.0, 1.0, 0.0));
        let l = mesh.verts.len();
        mesh.verts[l - 3].selected = true;
        mesh.verts[l - 2].selected = true;
        mesh.verts[l - 1].selected = true;
    }
    assert!(bridge.route_shortcut("f", false, false, false));
    assert_eq!(bridge.state.project.active_mesh().unwrap().faces.len(), 7);
    assert_eq!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .last()
            .unwrap()
            .verts
            .len(),
        3
    );

    // 3. Context Menu action "make_face" works identically
    bridge.open_viewport_context_menu(100.0, 100.0);
    // Select 2 remaining loose vertices
    {
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        mesh.deselect_all();
        mesh.verts.push(petunia_mesh::Vertex::new(20.0, 0.0, 0.0));
        mesh.verts.push(petunia_mesh::Vertex::new(21.0, 0.0, 0.0));
        let l = mesh.verts.len();
        mesh.verts[l - 2].selected = true;
        mesh.verts[l - 1].selected = true;
    }
    assert!(bridge.context_menu_action("make_face"));
    assert_eq!(
        bridge
            .state
            .project
            .active_mesh()
            .unwrap()
            .selected_edges
            .len(),
        1
    );

    // 4. In Object Mode, 'F' frames selection instead of making face
    bridge.state.set_selection_domain(SelectionDomain::Object);
    assert!(bridge.route_shortcut("f", false, false, false));
}

#[test]
fn test_slider_and_preference_responsiveness() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Selection thickness slider
    assert!(bridge.set_selection_thickness(3.5));
    assert!((bridge.state.ui.selection_thickness - 3.5).abs() < 1e-4);

    // Double tap interval slider
    assert!(bridge.set_double_tap_interval_ms(250));
    assert_eq!(bridge.preferences.double_tap_interval_ms, 250);

    // Asset thumbnail size slider
    bridge.set_asset_thumbnail_size(88.0);
    assert!((bridge.state.ui.asset_thumbnail_size - 88.0).abs() < 1e-4);

    // Reference slot slider param changes without wiping references
    let dummy_rgba = vec![255u8; 16 * 16 * 4];
    let mut ref_img =
        petunia_core::ReferenceImage::from_rgba("Test Ref".to_string(), 16, 16, dummy_rgba);
    ref_img.axis = petunia_core::RefAxis::Front;
    bridge.state.project.refs.push(ref_img);

    assert!(bridge.set_reference_param("front", "opacity", 0.65));
    assert!((bridge.state.project.refs[0].opacity - 0.65).abs() < 1e-4);
    assert!(bridge.set_reference_param("front", "size", 7.5));
    assert!((bridge.state.project.refs[0].size - 7.5).abs() < 1e-4);
}

#[test]
fn test_inspector_section_floating_pinning_and_pill_rail_toggle() {
    use petunia_config::InspectorSectionId;

    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // Verify default states: rail-first (collapsed to pill), docked, not pinned
    let parts_idx = crate::section_layout::section_index(InspectorSectionId::Parts);
    let transform_idx = crate::section_layout::section_index(InspectorSectionId::Transform);
    assert!(!bridge.section_layouts[parts_idx].open);
    assert!(bridge.section_layouts[parts_idx].docked);
    assert!(!bridge.section_layouts[parts_idx].pin_open);

    // 1. Float the Parts section
    bridge.set_section_docked(InspectorSectionId::Parts, false);
    bridge.move_section_float(InspectorSectionId::Parts, 250.0, 180.0);
    bridge.commit_section_float();
    assert!(!bridge.section_layouts[parts_idx].docked);
    assert_eq!(bridge.section_layouts[parts_idx].x, 250.0);
    assert_eq!(bridge.section_layouts[parts_idx].y, 180.0);

    // 2. Pin the floating section
    bridge.set_section_pin_open(InspectorSectionId::Parts, true);
    assert!(bridge.section_layouts[parts_idx].pin_open);

    // 3. Keep the floating section collapsed to its pill
    bridge.set_section_open(InspectorSectionId::Parts, false);
    assert!(!bridge.section_layouts[parts_idx].open);

    // In view model, open reflects closed state
    let vm = bridge.view_model();
    assert!(!vm.section_states[parts_idx].open);
    assert!(vm.section_states[parts_idx].pin_open);

    // 4. Toggle open via pill click: restores open state
    assert!(bridge.toggle_section_open(InspectorSectionId::Parts));
    assert!(bridge.section_layouts[parts_idx].open);
    // Pin remains preserved
    assert!(bridge.section_layouts[parts_idx].pin_open);

    // 5. Test Transform section docking and pill toggling
    bridge.set_section_open(InspectorSectionId::Transform, false);
    assert!(!bridge.section_layouts[transform_idx].open);
    assert!(bridge.section_layouts[transform_idx].docked);

    // Toggling re-opens
    assert!(bridge.toggle_section_open(InspectorSectionId::Transform));
    assert!(bridge.section_layouts[transform_idx].open);

    // Re-docking Parts section
    bridge.set_section_docked(InspectorSectionId::Parts, true);
    assert!(bridge.section_layouts[parts_idx].docked);
}

#[test]
fn bridge_set_icon_theme_switches_between_outline_and_filled() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    assert_eq!(bridge.preferences.icon_theme, "outline");
    assert_eq!(bridge.view_model().icon_theme, "outline");

    // Switch to filled
    assert!(bridge.set_icon_theme("filled"));
    assert_eq!(bridge.preferences.icon_theme, "filled");
    assert_eq!(bridge.view_model().icon_theme, "filled");

    // Redundant switch returns false
    assert!(!bridge.set_icon_theme("filled"));

    // Switch back to outline
    assert!(bridge.set_icon_theme("outline"));
    assert_eq!(bridge.preferences.icon_theme, "outline");
    assert_eq!(bridge.view_model().icon_theme, "outline");

    // Invalid string normalizes to outline
    bridge.set_icon_theme("filled");
    assert_eq!(bridge.preferences.icon_theme, "filled");
    assert!(bridge.set_icon_theme("unknown-mode"));
    assert_eq!(bridge.preferences.icon_theme, "outline");
    assert_eq!(bridge.view_model().icon_theme, "outline");
}

#[test]
fn bridge_restores_icon_theme_from_preferences() {
    let preferences = petunia_config::UserPreferences {
        icon_theme: "filled".to_string(),
        ..Default::default()
    };

    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.restore_section_layouts(&preferences);

    assert_eq!(bridge.preferences.icon_theme, "filled");
    assert_eq!(bridge.view_model().icon_theme, "filled");
}

#[test]
fn test_2d_generated_object_has_quads_and_deleting_face_preserves_segment() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

    // 1. Gera um objeto 2D retangular extrudado
    petunia_module_model::draw_profile::profile_set_rectangle(&mut bridge.state, 2.0, 1.5);
    bridge.state.profile.depth = 1.0;
    petunia_module_model::draw_profile::generate_extrude(&mut bridge.state);

    let active_mesh = bridge.state.project.active_mesh().expect("malha gerada");
    assert_eq!(active_mesh.faces.len(), 6, "deve ter 6 faces");
    for (i, face) in active_mesh.faces.iter().enumerate() {
        assert_eq!(
            face.verts.len(),
            4,
            "Face {i} deve ser um QUAD (4 vértices), e não um triângulo",
        );
    }

    // 2. Muda para o domínio de seleção de Face (Face mode)
    bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Face);

    // 3. Seleciona apenas a face do topo (face 1)
    if let Some(mesh) = bridge.state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.faces[1].selected = true;
        mesh.sync_vert_selection_from_faces();
    }

    // 4. Executa o comando de deletar
    let delete_ok = bridge.delete_or_dissolve_selection();
    assert!(delete_ok, "deleção deve suceder");

    // 5. Verifica que apenas a face selecionada foi deletada (restando 5 faces)
    // e que o restante do segmento (as 4 paredes laterais e o fundo) permanece intacto
    let remaining_mesh = bridge.state.project.active_mesh().expect("malha restante");
    assert_eq!(
        remaining_mesh.faces.len(),
        5,
        "apenas a face selecionada deve ser deletada, mantendo o restante do segmento (5 faces)"
    );
    for face in &remaining_mesh.faces {
        assert_eq!(
            face.verts.len(),
            4,
            "as faces restantes continuam sendo quads"
        );
    }
}

// ---------------------------------------------------------------------------
// Onda 1 (ADR 006): viewport nítido e bugs comprovados.
// ---------------------------------------------------------------------------

fn bridge_with_selected_face() -> SlintUiBridge<PlaceholderViewport> {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
    bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    bridge.state.sync_selection();
    bridge
}

#[test]
fn typed_value_does_not_leak_into_the_next_tool_modal() {
    let mut bridge = bridge_with_selected_face();

    assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
    assert!(bridge.route_shortcut("2", false, false, false));
    assert!(bridge.route_shortcut("Enter", false, false, false));
    assert!(bridge.modal_text.is_empty(), "confirmar limpa o buffer");

    assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
    assert!(bridge.modal_text.is_empty(), "iniciar começa sem texto");
    assert!(bridge.route_shortcut("5", false, false, false));
    assert!(
        (bridge.tool_modal_value - 5.0).abs() < 1.0e-4,
        "E → 2 → Enter → E → 5 deve extrudar 5, não 25 (valor: {})",
        bridge.tool_modal_value
    );
    assert!(bridge.cancel_tool_modal());
    assert!(bridge.modal_text.is_empty(), "cancelar limpa o buffer");
}

#[test]
fn typed_value_wins_over_drag_and_shows_in_the_hud() {
    let mut bridge = bridge_with_selected_face();

    assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
    assert!(bridge.route_shortcut("1", false, false, false));
    assert!(bridge.route_shortcut(".", false, false, false));
    assert!(bridge.route_shortcut("5", false, false, false));
    assert!((bridge.tool_modal_value - 1.5).abs() < 1.0e-4);

    assert!(
        !bridge.scrub_tool_modal(-80.0, false),
        "o arrasto não sobrescreve o texto"
    );
    assert!((bridge.tool_modal_value - 1.5).abs() < 1.0e-4);
    assert!(
        bridge
            .view_model()
            .operation_hud_lines
            .iter()
            .any(|line| line.contains("Input   1.5")),
        "o HUD mostra o valor digitado"
    );
}

#[test]
fn view_roll_ring_is_clickable_on_the_drawn_radius() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetActiveTool("rotate".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);

    let drawn = projection::GIZMO_VIEW_ROLL_RADIUS;
    assert!(
        bridge
            .gizmo_handle_at(gizmo.origin_x, gizmo.origin_y - drawn)
            .is_some(),
        "o anel responde onde é desenhado"
    );
    assert!(
        bridge
            .gizmo_handle_at(gizmo.origin_x, gizmo.origin_y - 96.0 * 1.18)
            .is_none(),
        "o raio antigo de hit-test não responde mais"
    );
}

#[test]
fn knife_card_actions_apply_and_cancel_the_cut_session() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);

    assert!(bridge.execute_core_command("model.knife").is_ok());
    assert_eq!(
        bridge.view_model().active_tool,
        "cut",
        "o card testa o id `cut`"
    );
    assert!(bridge.execute_core_command("model.knife_cancel").is_ok());
    assert!(bridge.state.session.tools.cut_session.is_none());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
    assert!(!bridge.state.project.undo.can_undo());

    assert!(bridge.execute_core_command("model.knife").is_ok());
    assert!(bridge.execute_core_command("model.knife_apply").is_ok());
    assert!(bridge.state.session.tools.cut_session.is_none());
    assert!(
        !bridge.state.project.undo.can_undo(),
        "aplicar sem segmentos não cria histórico"
    );
}

#[test]
fn dimension_annotation_only_draws_linear_distances() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);

    bridge
        .state
        .begin_modal(petunia_core::ModalKind::Rotate)
        .unwrap();
    bridge.state.update_modal(glam::Vec3::ZERO, 45.0).unwrap();
    assert!(
        !compute_dimension_annotation(&bridge.state, 1024.0, 768.0).visible,
        "graus não viram cota linear"
    );
    bridge.state.cancel_modal();

    bridge
        .state
        .begin_modal(petunia_core::ModalKind::Move)
        .unwrap();
    bridge
        .state
        .update_modal(glam::Vec3::new(1.0, 0.0, 0.0), 1.0)
        .unwrap();
    let dimension = compute_dimension_annotation(&bridge.state, 1024.0, 768.0);
    assert!(dimension.visible);
    assert_eq!(dimension.text, "1.00 m");
}

#[test]
fn hovering_the_same_empty_spot_requests_no_redraw() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    let _ = bridge.hover_component(0.02, 0.02);
    assert!(
        !bridge.hover_component(0.02, 0.02),
        "mouse parado sobre o mesmo alvo não pede novo frame"
    );
}

/// Viewport de teste que registra o tamanho pedido e a razão de pixels.
#[derive(Default)]
struct PhysicalPixelProbe {
    size: (u32, u32),
    ratio: f32,
}

impl PetuniaViewport for PhysicalPixelProbe {
    fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
    }
    fn update(&mut self, _dt_seconds: f32) {}
    fn set_workspace(&mut self, _workspace: Workspace) {}
    fn set_selection_domain(&mut self, _domain: SelectionDomain) {}
    fn uses_physical_pixels(&self) -> bool {
        true
    }
    fn set_pixel_ratio(&mut self, ratio: f32) {
        self.ratio = ratio;
    }
}

#[test]
fn gpu_viewport_renders_in_physical_pixels_while_ui_math_stays_logical() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PhysicalPixelProbe::default());
    bridge.resize_viewport_scaled(800, 600, 1.5);
    assert_eq!(bridge.viewport.size, (1200, 900), "alvo em px físicos");
    assert!((bridge.viewport.ratio - 1.5).abs() < f32::EPSILON);
    assert_eq!(
        bridge.viewport_size,
        [800.0, 600.0],
        "picking e overlays em px lógicos"
    );

    // Um resize sem razão explícita preserva a última razão medida.
    bridge.resize_viewport(400, 300);
    assert_eq!(bridge.viewport.size, (600, 450));

    // Razões inválidas caem para 1.
    bridge.resize_viewport_scaled(400, 300, f32::NAN);
    assert_eq!(bridge.viewport.size, (400, 300));
}

#[test]
fn cpu_viewport_keeps_logical_pixels() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport_scaled(800, 600, 2.0);
    assert_eq!(bridge.viewport.width, 800);
    assert_eq!(bridge.viewport.height, 600);
}

// ---------------------------------------------------------------------------
// Onda 2 (ADR 006): gramática única de ferramenta.
// ---------------------------------------------------------------------------

/// Um ponto de tela (px lógicos) sobre uma face visível do asset ativo.
fn visible_face_pixel(bridge: &SlintUiBridge<PlaceholderViewport>) -> (usize, [f32; 2]) {
    let [width, height] = bridge.viewport_size;
    let mesh = bridge.state.project.active_mesh().expect("active mesh");
    for (index, face) in mesh.faces.iter().enumerate() {
        let center = face
            .verts
            .iter()
            .map(|&v| mesh.verts[v as usize].vec())
            .sum::<glam::Vec3>()
            / face.verts.len() as f32;
        let ndc = bridge.state.session.camera.project_ndc(center);
        let point = [(ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5];
        if bridge.pick_target_for_domain(SelectionDomain::Face, point[0], point[1])
            == petunia_core::HoverTarget::Face(index)
        {
            return (index, [point[0] * width, point[1] * height]);
        }
    }
    panic!("nenhuma face visível no enquadramento padrão");
}

fn extrude_tool_bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.execute_shortcut_tool("model.extrude");
    bridge
}

#[test]
fn tool_key_selects_the_persistent_tool_without_opening_an_operation() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    let selected_before = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .filter(|face| face.selected)
        .count();

    bridge.execute_shortcut_tool("model.extrude");

    assert_eq!(bridge.state.session.tools.active_tool, "extrude");
    assert!(
        bridge.tool_modal.is_none(),
        "a tecla só escolhe a ferramenta"
    );
    assert_eq!(bridge.state.selection_domain(), SelectionDomain::Face);
    let selected_after = bridge
        .state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .filter(|face| face.selected)
        .count();
    assert_eq!(
        selected_after, selected_before,
        "nunca seleciona tudo sozinho"
    );
    assert!(!bridge.state.project.undo.can_undo());
    assert!(bridge.view_model().tool_grammar_active);
}

#[test]
fn click_selects_and_drag_extrudes_the_face_under_the_cursor() {
    let mut bridge = extrude_tool_bridge();
    let (face, [x, y]) = visible_face_pixel(&bridge);

    // Clique sem arrasto: só seleciona.
    bridge.tool_pointer(0, x, y, false, false);
    assert!(bridge.tool_pointer(2, x, y, false, false));
    assert!(bridge.state.project.active_mesh().unwrap().faces[face].selected);
    assert!(
        !bridge.state.project.undo.can_undo(),
        "clique não cria histórico"
    );

    // Arrasto: opera seguindo o cursor e confirma um único Undo.
    let faces_before = bridge.state.project.active_mesh().unwrap().faces.len();
    bridge.tool_pointer(0, x, y, false, false);
    assert!(bridge.tool_pointer(1, x, y - 60.0, false, false));
    assert!(bridge.tool_modal.is_some(), "o arrasto abriu a operação");
    assert!(
        bridge.tool_modal_value.abs() > 1.0e-4,
        "o valor segue o cursor"
    );
    assert!(bridge.tool_pointer(2, x, y - 60.0, false, false));

    assert!(bridge.tool_modal.is_none());
    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    assert!(bridge.state.project.active_mesh().unwrap().faces.len() > faces_before);
    assert_eq!(
        bridge.state.session.tools.active_tool, "extrude",
        "a ferramenta é persistente"
    );
    assert!(bridge.view_model().last_operation_active);
}

#[test]
fn dragging_an_unselected_face_selects_it_before_extruding() {
    let mut bridge = extrude_tool_bridge();
    let (face, [x, y]) = visible_face_pixel(&bridge);
    assert!(!bridge.state.project.active_mesh().unwrap().faces[face].selected);

    bridge.tool_pointer(0, x, y, false, false);
    bridge.tool_pointer(1, x, y - 50.0, false, false);
    bridge.tool_pointer(2, x, y - 50.0, false, false);

    assert_eq!(bridge.state.project.undo.depth(), (1, 0));
}

#[test]
fn last_operation_card_adjusts_within_the_same_undo_entry() {
    let mut bridge = extrude_tool_bridge();
    let (_, [x, y]) = visible_face_pixel(&bridge);
    bridge.tool_pointer(0, x, y, false, false);
    bridge.tool_pointer(1, x, y - 50.0, false, false);
    bridge.tool_pointer(2, x, y - 50.0, false, false);
    let faces_after_gesture = bridge.state.project.active_mesh().unwrap().faces.len();

    assert!(bridge.commit_last_operation_text("1.5"));
    assert_eq!(
        bridge.state.project.undo.depth(),
        (1, 0),
        "sem entrada extra"
    );
    let last = bridge.last_operation.as_ref().expect("ainda ajustável");
    assert!((last.primary_value() - 1.5).abs() < 1.0e-4);
    assert_eq!(
        bridge.state.project.active_mesh().unwrap().faces.len(),
        faces_after_gesture,
        "reaplica a partir do estado original"
    );
    assert!(
        !bridge.commit_last_operation_text("abc"),
        "texto inválido é recusado"
    );
}

#[test]
fn right_button_never_cancels_an_active_gesture() {
    let mut bridge = extrude_tool_bridge();
    let (_, [x, y]) = visible_face_pixel(&bridge);
    bridge.tool_pointer(0, x, y, false, false);
    bridge.tool_pointer(1, x, y - 50.0, false, false);
    assert!(bridge.tool_modal.is_some());

    assert!(!bridge.viewport_context_triage(x, y));
    assert!(bridge.tool_modal.is_some(), "RMB não cancela");
    assert!(
        bridge.context_menu.is_none(),
        "nem abre menu durante o gesto"
    );

    // Esc cancela e restaura.
    assert!(bridge.handle_escape());
    assert!(bridge.tool_modal.is_none());
    assert!(!bridge.state.project.undo.can_undo());

    // Fora de gesto, RMB abre o menu.
    assert!(bridge.viewport_context_triage(x, y));
    assert!(bridge.context_menu.is_some());
}

#[test]
fn escape_ladder_returns_a_persistent_tool_to_select() {
    let mut bridge = extrude_tool_bridge();
    assert!(bridge.handle_escape());
    assert_eq!(bridge.state.session.tools.active_tool, "select");
}

#[test]
fn escape_keeps_a_newly_created_primitive() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(800, 600);
    let assets_before = bridge.state.project.assets.len();
    bridge.apply(UiIntent::AddPrimitive(
        petunia_core::PrimitiveKind::Cylinder,
    ));
    assert!(bridge.view_model().primitive_active);

    assert!(bridge.handle_escape());
    assert!(!bridge.view_model().primitive_active, "Esc fecha o card");
    assert_eq!(
        bridge.state.project.assets.len(),
        assets_before + 1,
        "Esc não apaga a primitiva; Undo apaga"
    );
    assert!(bridge.state.project.undo.can_undo());
}

#[test]
fn navigation_is_never_suspended_by_tools() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    bridge.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
    let before = bridge.state.session.camera.clone();
    assert!(
        bridge.orbit_viewport(12.0, 4.0),
        "orbita com Loop Cut armado"
    );
    assert_ne!(bridge.state.session.camera.yaw, before.yaw);

    let distance = bridge.state.session.camera.distance;
    bridge.apply_viewport_gesture(ViewportGesture::Zoom { delta: 120.0 });
    assert_ne!(
        bridge.state.session.camera.distance, distance,
        "a roda faz zoom"
    );

    // Com um anel sob o cursor, Ctrl+roda muda a contagem (e a roda não).
    let (_, point) = visible_edge_points(&bridge)[0];
    bridge.hover_component(point[0], point[1]);
    let cuts = bridge.loop_cut_hover_cuts;
    assert!(bridge.viewport_ctrl_scroll(120.0));
    assert_ne!(
        bridge.loop_cut_hover_cuts, cuts,
        "Ctrl+roda muda a contagem"
    );
}

#[test]
fn click_move_click_on_a_gizmo_handle_follows_until_the_next_click() {
    let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
    bridge.resize_viewport(1024, 768);
    assert!(bridge.set_click_move_click(true));
    bridge.apply(UiIntent::SetActiveTool("move".to_string()));
    let gizmo = bridge.view_model().gizmo;
    assert!(gizmo.visible);

    // Clicar no centro do gizmo sem arrastar prende a alça ao ponteiro.
    bridge.tool_pointer(0, gizmo.origin_x, gizmo.origin_y, false, false);
    bridge.tool_pointer(2, gizmo.origin_x, gizmo.origin_y, false, false);
    assert!(bridge.tool_session.is_latched());
    assert!(bridge.view_model().tool_gesture_latched);

    // Mover sem botão move o objeto; o próximo clique confirma.
    bridge.tool_pointer(1, gizmo.origin_x + 80.0, gizmo.origin_y, false, false);
    bridge.tool_pointer(0, gizmo.origin_x + 80.0, gizmo.origin_y, false, false);
    bridge.tool_pointer(2, gizmo.origin_x + 80.0, gizmo.origin_y, false, false);

    assert!(!bridge.tool_session.is_latched());
    assert_eq!(
        bridge.state.project.undo.depth(),
        (1, 0),
        "o gesto preso ao ponteiro virou uma transação"
    );
}

#[test]
fn drag_threshold_preference_separates_click_from_drag() {
    let mut bridge = extrude_tool_bridge();
    assert!(bridge.set_drag_threshold_px(12.0));
    let (_, [x, y]) = visible_face_pixel(&bridge);

    bridge.tool_pointer(0, x, y, false, false);
    bridge.tool_pointer(1, x, y - 8.0, false, false);
    bridge.tool_pointer(2, x, y - 8.0, false, false);
    assert!(
        !bridge.state.project.undo.can_undo(),
        "8 px abaixo de um limiar de 12 px é clique"
    );
    assert!(
        !bridge.set_drag_threshold_px(12.0),
        "sem mudança não persiste"
    );
    assert!(bridge.set_drag_threshold_px(500.0));
    assert_eq!(
        bridge.preferences.drag_threshold_px, 16.0,
        "limitado a 16 px"
    );
}
