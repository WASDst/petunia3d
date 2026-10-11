//! Testes de decomposição e isolamento de sub-estados do Core.
//! Valida que ProjectState, EditorSession, ToolState, UiState e RenderResources
//! podem ser operados de forma 100% isolada e desacoplada.

use petunia_core::KeyText;
use petunia_core::command::{CommandContext, CommandDispatcher, SelectAllCmd};
use petunia_core::events::EventBus;
use petunia_core::modal::ModalKind;
use petunia_core::queries::query_selection_details;
use petunia_core::state::{
    ActiveTool, ActiveToolSessionKind, AppState, DocumentSession, EditMode, EditorSession,
    ProjectState, ReferenceImage, RenderResources, Shading, TextureDirtyRect, ToolSessionManager,
    ToolState, UiState, ViewportSession,
};
use petunia_mesh::Mesh;
use petunia_project::{Canvas, Project};

#[test]
fn test_project_state_isolated_lifecycle() {
    let mut project_state = ProjectState::new();
    assert_eq!(project_state.assets.len(), 1);
    assert_eq!(project_state.scene_tris(), 12);
    assert_eq!(project_state.scene_verts(), 8);

    // Testar duplicação de asset via ProjectState
    project_state.add("CustomPlane", Mesh::plane(2.0));
    assert_eq!(project_state.assets.len(), 2);
    assert_eq!(project_state.assets[1].name, "CustomPlane");

    // Testar checkpoints e undo no ProjectState isolado
    project_state.checkpoint("add custom plane");
    assert!(project_state.undo.can_undo());

    // Testar locking do asset ativo
    assert!(!project_state.is_active_locked());
    let toggle_res = project_state.toggle_lock_active();
    assert!(toggle_res.is_some());
    assert!(project_state.is_active_locked());

    // Testar picking em geometria no ProjectState isolado (apontando para vértice em 1.0, 0.0, 1.0 do CustomPlane)
    let pick = project_state.pick_vertex(
        glam::Vec3::new(1.0, 5.0, 1.0),
        glam::Vec3::new(0.0, -1.0, 0.0),
    );
    assert!(pick.is_some());
}

#[test]
fn test_editor_session_isolated_lifecycle() {
    let mut session = EditorSession::new();
    assert_eq!(session.edit_mode(), EditMode::Object);
    assert_eq!(session.locked_axes, [false; 3]);
    assert!(
        session.snap_enabled,
        "Smart Snap ligado por padrão (cap. 01)"
    );
    assert!(!session.isolate_active);

    // Testar restrições de eixo no EditorSession
    assert!(!session.is_axis_locked(0, None));
    session.locked_axes[0] = true;
    assert!(session.is_axis_locked(0, None));
    assert_eq!(
        session.active_axis_constraint_label(None),
        Some(("Eixo X", [235, 75, 75]))
    );

    // Testar sincronização de seleção no EditorSession isolado
    let project = Project::new();
    let mut events = EventBus::new();
    session.sync_selection(&project, &mut events);
    assert!(session.selection.asset.is_some());
}

#[test]
fn test_tool_state_isolated_lifecycle() {
    let mut tools = ToolState::new();
    assert_eq!(tools.active_tool, "select");
    assert_eq!(tools.gizmo_mode, ModalKind::Move);
    assert!(tools.modal.is_none());
    assert!(tools.pointer_session.is_none());
    assert!(tools.cut_session.is_none());
    assert!(tools.mesh_preview.is_none());

    // Modificar parâmetros da ferramenta
    tools.extrude_dist = 1.25;
    tools.paint_radius = 0.5;
    tools.active_tool = "extrude".to_string();
    assert_eq!(tools.extrude_dist, 1.25);
    assert_eq!(tools.paint_radius, 0.5);
    assert_eq!(tools.active_tool, "extrude");
}

#[test]
fn test_ui_state_isolated_lifecycle() {
    let mut ui = UiState::new("en");
    assert_eq!(ui.properties_tab, "object");
    assert_eq!(ui.outliner_search, "");
    assert!(!ui.show_settings);
    assert!(!ui.show_asset_browser);

    ui.outliner_search = "search_test".to_string();
    ui.set_status("ready");
    assert_eq!(ui.outliner_search, "search_test");
    assert_eq!(ui.status, "ready");
}

#[test]
fn test_render_resources_isolated_lifecycle() {
    let mut render = RenderResources::new();
    assert!(render.dirty);
    assert!(render.canvas_dirty);
    assert_eq!(render.backend_name, "");

    assert!(render.consume_dirty());
    assert!(!render.dirty);

    render.mark_dirty();
    assert!(render.dirty);
}

#[test]
fn texture_updates_clip_deduplicate_and_full_update_dominates() {
    let project = Project::new();
    let asset_id = project.assets[0].id;
    let mut render = RenderResources::new();
    let region = TextureDirtyRect {
        x: 48,
        y: 48,
        width: 32,
        height: 32,
    };

    render.mark_texture_regions(asset_id, 64, 64, [region, region]);
    let partial = render.take_texture_updates();
    assert_eq!(partial.len(), 1);
    assert_eq!(
        partial[0].regions.as_deref(),
        Some(
            [TextureDirtyRect {
                x: 48,
                y: 48,
                width: 16,
                height: 16,
            }]
            .as_slice()
        )
    );

    render.mark_texture_regions(asset_id, 64, 64, [region]);
    render.mark_texture_full(asset_id, 64, 64);
    render.mark_texture_regions(asset_id, 64, 64, [region]);
    let full = render.take_texture_updates();
    assert_eq!(full.len(), 1);
    assert!(full[0].is_full());

    render.mark_texture_regions(
        asset_id,
        64,
        64,
        [
            TextureDirtyRect {
                x: 0,
                y: 0,
                width: 32,
                height: 64,
            },
            TextureDirtyRect {
                x: 32,
                y: 0,
                width: 32,
                height: 64,
            },
        ],
    );
    assert!(render.take_texture_updates()[0].is_full());
}

#[test]
fn regional_texture_event_bumps_revision_and_publishes_transient_update() {
    let mut state = AppState::new("en");
    let active = state.project.active;
    let asset_id = state.project.assets[active].id;
    state.project.assets[active].texture = Some(Canvas::new(64, 32, [0, 0, 0, 255]));
    let revision = state.project.project.texture_revision;

    state.emit_texture_changed_regions(vec![TextureDirtyRect {
        x: 8,
        y: 4,
        width: 12,
        height: 6,
    }]);

    assert_eq!(state.project.project.texture_revision, revision + 1);
    assert_eq!(
        state.render.take_texture_updates(),
        vec![petunia_core::TextureUpdate {
            asset_id,
            canvas_width: 64,
            canvas_height: 32,
            regions: Some(vec![TextureDirtyRect {
                x: 8,
                y: 4,
                width: 12,
                height: 6,
            }]),
        }]
    );
}

#[test]
fn regional_texture_event_reaches_assets_inheriting_shared_material() {
    let mut state = AppState::new("en");
    let active = state.project.active;
    let material_id = state.project.assets[active].material_id.unwrap();
    state.project.assets[active].texture = Some(Canvas::new(32, 32, [0, 0, 0, 255]));
    state.project.materials[0].albedo_texture = Some(Canvas::new(32, 32, [0, 0, 0, 255]));
    let mut sibling = petunia_project::Asset::new("Sibling", Mesh::cube(1.0));
    sibling.material_id = Some(material_id);
    let sibling_id = sibling.id;
    state.project.assets.push(sibling);

    state.emit_texture_changed_regions(vec![TextureDirtyRect {
        x: 4,
        y: 5,
        width: 6,
        height: 7,
    }]);

    let updates = state.render.take_texture_updates();
    assert_eq!(updates.len(), 2);
    assert!(updates.iter().any(|update| update.asset_id == sibling_id));
}

#[test]
fn test_app_state_composition_and_deref_coercion() {
    let mut state = AppState::new("en");

    // Acesso transparente via Deref a ProjectState -> Project
    assert_eq!(state.project.assets.len(), 1);
    assert_eq!(state.project.assets[0].name, "Cube");

    // Acesso transparente via Deref a EditorSession
    assert_eq!(state.edit_mode(), EditMode::Object);
    assert_eq!(state.camera.fov_y, 45.0_f32.to_radians());

    // Acesso transparente via Deref transitivo a ToolState
    assert_eq!(state.active_tool, "select");
    state.active_tool = "knife".to_string();
    assert_eq!(state.session.tools.active_tool, "knife");

    // Acesso explícito aos sub-estados
    state.ui.outliner_search = "Cube".to_string();
    assert_eq!(state.ui.outliner_search, "Cube");

    state.render.stats.fps = 60.0;
    assert_eq!(state.render.stats.fps, 60.0);
}

#[test]
fn test_document_session_isolated_lifecycle() {
    let mut doc_session = DocumentSession::new();
    assert_eq!(doc_session.document().assets.len(), 1);
    assert_eq!(doc_session.scene_tris(), 12);
    assert_eq!(doc_session.file_path(), None);
    assert!(!doc_session.is_dirty());

    doc_session.set_file_path(Some("scene.p3d".to_string()));
    assert_eq!(
        doc_session.file_path().map(|s| s.as_str()),
        Some("scene.p3d")
    );

    doc_session.document_mut().add("Sphere", Mesh::cube(2.0));
    assert_eq!(doc_session.document().assets.len(), 2);

    doc_session.checkpoint("added sphere");
    assert!(doc_session.is_dirty());
    assert!(doc_session.history().can_undo());

    doc_session.mark_clean();
    assert!(!doc_session.is_dirty());

    let can_undo = doc_session.history_mut().can_undo();
    assert!(can_undo);
}

#[test]
fn test_viewport_session_isolated_lifecycle() {
    let mut vs = ViewportSession::new();
    assert_eq!(vs.shading, Shading::Solid);
    assert_eq!(vs.cursor_3d, [0.0, 0.0, 0.0]);
    assert!(vs.snap_enabled);
    assert!(vs.show_grid);

    // Modificar configurações na sessão da viewport
    vs.shading = Shading::Wireframe;
    vs.cursor_3d = [1.0, 2.0, 3.0];
    vs.snap_enabled = false;
    vs.camera.distance = 15.0;

    let mut session = EditorSession::new();
    session.set_viewport_session(vs.clone());

    assert_eq!(session.shading, Shading::Wireframe);
    assert_eq!(session.cursor_3d, [1.0, 2.0, 3.0]);
    assert!(!session.snap_enabled);
    assert_eq!(session.camera.distance, 15.0);

    let retrieved_vs = session.viewport_session();
    assert_eq!(retrieved_vs, vs);

    let from_vs = ViewportSession::from(&session);
    assert_eq!(from_vs, vs);
}

#[test]
fn test_app_state_session_accessors() {
    let mut state = AppState::new("en");

    // DocumentSession accessors
    assert_eq!(state.document_session().assets.len(), 1);
    state
        .document_session_mut()
        .set_file_path(Some("project.p3d".to_string()));
    assert_eq!(
        state.document_session().file_path().map(|s| s.as_str()),
        Some("project.p3d")
    );

    // EditorSession accessors
    assert_eq!(state.editor_session().edit_mode(), EditMode::Object);
    state.editor_session_mut().locked_axes[1] = true;
    assert!(state.editor_session().locked_axes[1]);

    // ViewportSession accessors
    let mut vs = state.viewport_session();
    vs.cursor_3d = [5.0, 5.0, 5.0];
    state.set_viewport_session(vs);
    assert_eq!(state.viewport_session().cursor_3d, [5.0, 5.0, 5.0]);

    // ToolSessionManager accessors
    let tsm: &ToolSessionManager = state.tool_session_manager();
    assert_eq!(tsm.active_tool, "select");
    state.tool_session_manager_mut().active_tool = "extrude".to_string();
    assert_eq!(state.tool_session_manager().active_tool, "extrude");
}

#[test]
fn test_reference_image_in_authored_document() {
    let mut doc_session = DocumentSession::new();
    assert!(doc_session.references().is_empty());

    let ref_img = ReferenceImage::from_rgba("blueprint".to_string(), 100, 100, vec![255; 40000]);
    doc_session.references_mut().push(ref_img);

    assert_eq!(doc_session.references().len(), 1);
    assert_eq!(doc_session.references()[0].name, "blueprint");

    doc_session.checkpoint("added ref");
    assert!(doc_session.history().can_undo());

    doc_session.reset();
    assert!(doc_session.references().is_empty());
}

#[test]
fn test_tool_session_manager_active_tool_and_session() {
    let mut tsm = ToolSessionManager::new();
    assert_eq!(tsm.active_tool_info().id, "select");
    assert_eq!(tsm.active_session_kind(), ActiveToolSessionKind::None);
    assert!(!tsm.is_session_active());

    tsm.set_active_tool_info(ActiveTool {
        id: "knife".to_string(),
        activation: petunia_core::ToolActivation::Instant,
    });
    assert_eq!(tsm.active_tool_info().id, "knife");
    assert_eq!(tsm.active_tool, "knife");
}

#[test]
fn test_command_context_dispatch_and_queries() {
    let mut doc_session = DocumentSession::new();
    let mut editor_session = EditorSession::new();
    let mut events = EventBus::new();

    // Queries sobre DocumentSession isolado
    let hierarchy = doc_session.query_scene_hierarchy();
    assert_eq!(hierarchy.assets.len(), 1);
    assert_eq!(hierarchy.total_verts, 8);

    // Queries cruzadas desacopladas
    let sel_details = query_selection_details(&doc_session, &editor_session);
    assert_eq!(sel_details.selected_verts_count, 0);

    // Despacho sobre CommandContext em SelectionDomain::Object
    let mut ctx = CommandContext::new(&mut doc_session, &mut editor_session, &mut events, &KeyText);
    let select_all = SelectAllCmd;
    let res = CommandDispatcher::dispatch_context(&mut ctx, &select_all);
    assert!(res.is_ok());
    assert_eq!(editor_session.selection.assets.len(), 1);

    // Despacho sobre CommandContext em SelectionDomain::Vertex
    editor_session.selection_domain = petunia_core::SelectionDomain::Vertex;
    let mut ctx_vertex =
        CommandContext::new(&mut doc_session, &mut editor_session, &mut events, &KeyText);
    let res = CommandDispatcher::dispatch_context(&mut ctx_vertex, &select_all);
    assert!(res.is_ok());

    let updated_sel = query_selection_details(&doc_session, &editor_session);
    assert_eq!(updated_sel.selected_verts_count, 8);
}
