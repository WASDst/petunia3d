//! Testes de decomposição e isolamento de sub-estados do Core.
//! Valida que ProjectState, EditorSession, ToolState, UiState e RenderResources
//! podem ser operados de forma 100% isolada e desacoplada.

use petunia_core::events::EventBus;
use petunia_core::modal::ModalKind;
use petunia_core::state::{
    AppState, EditMode, EditorSession, ProjectState, RenderResources, TextureDirtyRect, ToolState,
    UiState,
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
    assert!(!session.snap_enabled);
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
