//! Testes de integração de Soberania Headless (Gauntlet G8 / F-011).
//!
//! Comprova que todas as operações de modelagem, gerenciamento de projetos,
//! execução de comandos, histórico undo/redo e exportação de malhas funcionam
//! de forma pura, determinística e ultrarrápida sem qualquer dependência gráfica.

use std::path::PathBuf;
use std::time::Instant;

use petunia_core::{
    AddPrimitiveCmd, AppState, ClearSelectionCmd, DeleteSelectionCmd, DuplicateSelectionCmd,
    InvertSelectionCmd, PrimitiveKind, ProjectService, SelectAllCmd,
};
use petunia_module_model::{
    BevelTool, ConnectTool, DissolveTool, ExtrudeTool, InsetTool, MergeTool, MirrorTool,
    PrimitivesTool, PushPullTool, SliceTool, SubdivideTool, TransformTool,
};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("petunia_test_{}_{}", std::process::id(), name))
}

#[test]
fn test_headless_project_creation_and_inspection() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    assert_eq!(state.project.assets.len(), 1);
    assert_eq!(state.project.assets[0].name, "Cube");
    assert_eq!(state.project.assets[0].mesh.vert_count(), 8);
    assert_eq!(state.project.assets[0].mesh.faces.len(), 6);

    // Adiciona primitivas
    PrimitivesTool::add_primitive(&mut state, "Sphere");
    PrimitivesTool::add_primitive(&mut state, "Cylinder8");

    assert_eq!(state.project.assets.len(), 3);
    let (total_verts, total_tris) = state.project.totals();
    assert!(total_verts > 24);
    assert!(total_tris > 6);

    // Salva projeto
    let p_path = temp_path("inspect.petunia");
    ProjectService::save_project(&mut state, &p_path).expect("Salvar projeto headless");

    // Carrega em novo estado
    let mut loaded_state = AppState::new("en");
    ProjectService::load_project(&mut loaded_state, &p_path).expect("Carregar projeto headless");

    assert_eq!(loaded_state.project.assets.len(), 3);
    assert_eq!(loaded_state.project.assets[1].name, "Sphere");
    assert_eq!(loaded_state.project.assets[2].name, "Cylinder8");

    let _ = std::fs::remove_file(p_path);
}

#[test]
fn test_headless_command_dispatch_and_history() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    // 1. AddPrimitiveCmd
    let add_cmd = AddPrimitiveCmd::new(PrimitiveKind::Plane);
    state.dispatch(&add_cmd).expect("AddPrimitiveCmd");
    assert_eq!(state.project.assets.len(), 2);
    assert_eq!(state.project.assets[1].name, "Plane");

    // 2. SelectAllCmd no contexto de edição de vértices
    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.select_mode = petunia_core::SelectMode::Vertex;
    state.dispatch(&SelectAllCmd).expect("SelectAllCmd");
    let active_mesh = state.project.active_mesh().expect("Active mesh");
    assert!(active_mesh.verts.iter().all(|v| v.selected));

    // 3. ClearSelectionCmd
    state
        .dispatch(&ClearSelectionCmd)
        .expect("ClearSelectionCmd");
    let active_mesh = state.project.active_mesh().expect("Active mesh");
    assert!(active_mesh.verts.iter().all(|v| !v.selected));

    // 4. InvertSelectionCmd
    state
        .dispatch(&InvertSelectionCmd)
        .expect("InvertSelectionCmd");
    assert!(
        state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .all(|v| v.selected)
    );

    // 5. DuplicateSelectionCmd em Edit Mode (duplica geometria)
    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.select_mode = petunia_core::SelectMode::Vertex;
    let verts_before = state.project.active_mesh().unwrap().vert_count();
    state
        .dispatch(&DuplicateSelectionCmd)
        .expect("DuplicateSelectionCmd");
    assert_eq!(
        state.project.active_mesh().unwrap().vert_count(),
        verts_before * 2
    );

    // 6. DeleteSelectionCmd
    state
        .dispatch(&DeleteSelectionCmd)
        .expect("DeleteSelectionCmd");
    assert_eq!(
        state.project.active_mesh().unwrap().vert_count(),
        verts_before
    );

    // 7. Undo até o estado inicial
    assert!(state.undo());
    assert!(state.undo());
    assert!(state.undo());

    // 8. Redo
    assert!(state.redo());
}

#[test]
fn test_headless_geometric_tools_execution() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    // Seleciona uma face para extrusão limpa
    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.select_mode = petunia_core::SelectMode::Face;
    if let Some(m) = state.project.active_mesh_mut() {
        m.deselect_all();
        m.faces[0].selected = true;
        for &vi in &m.faces[0].verts {
            m.verts[vi as usize].selected = true;
        }
    }
    state.sync_selection();

    // Extrude
    state.extrude_dist = 1.0;
    let initial_verts = state.project.active_mesh().unwrap().vert_count();
    ExtrudeTool::apply(&mut state);
    assert!(state.project.active_mesh().unwrap().vert_count() > initial_verts);

    // Seleciona tudo para as próximas operações
    state.dispatch(&SelectAllCmd).expect("SelectAll");

    // Subdivide
    let v_pre_sub = state.project.active_mesh().unwrap().vert_count();
    SubdivideTool::apply_subdivide(&mut state);
    assert!(state.project.active_mesh().unwrap().vert_count() > v_pre_sub);

    // Transform Scale
    state.transform_scale = 1.5;
    TransformTool::apply_scale(&mut state);

    // Inset
    state.inset_distance = 0.1;
    InsetTool::apply(&mut state);

    // Bevel
    state.bevel_amount = 0.1;
    BevelTool::apply(&mut state);

    // PushPull
    state.push_dist = 0.2;
    PushPullTool::apply(&mut state);

    // Slice
    SliceTool::apply_slice(&mut state, glam::Vec3::Y, false);

    // Mirror
    state.mirror_axis = 0;
    state.mirror_weld = 0.01;
    MirrorTool::apply(&mut state);

    // Merge
    MergeTool::apply(&mut state);

    // Dissolve
    DissolveTool::apply(&mut state);

    // Connect
    ConnectTool::apply(&mut state);

    assert!(state.project.active_mesh().unwrap().vert_count() > 0);
}

#[test]
fn test_headless_export_obj_and_glb() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    let obj_path = temp_path("headless_cube.obj");
    let glb_path = temp_path("headless_cube.glb");

    // Export OBJ
    ProjectService::export_obj(&state, state.project.active, &obj_path)
        .expect("Exportar OBJ headless");
    assert!(obj_path.exists());
    let obj_str = std::fs::read_to_string(&obj_path).expect("Ler OBJ gerado");
    assert!(obj_str.contains("v "));
    assert!(obj_str.contains("f "));

    // Export GLB
    let all_assets = vec![0];
    ProjectService::export_glb(&state, &all_assets, &glb_path).expect("Exportar GLB headless");
    assert!(glb_path.exists());
    let glb_bytes = std::fs::read(&glb_path).expect("Ler GLB gerado");
    assert!(glb_bytes.len() >= 12);
    // Valida magic glTF em little endian: 0x46546C67 ("glTF")
    assert_eq!(&glb_bytes[0..4], b"glTF");

    let _ = std::fs::remove_file(obj_path);
    let _ = std::fs::remove_file(glb_path);
}

#[test]
fn test_headless_complete_session_roundtrip() {
    let start = Instant::now();

    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    // Adiciona modelo pelo caminho canônico de comando
    PrimitivesTool::add_primitive(&mut state, "Cylinder8");

    // Seleciona uma face e extrude no contexto de edição.
    state.set_edit_mode(petunia_core::EditMode::Edit);
    state.select_mode = petunia_core::SelectMode::Face;
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.deselect_all();
        mesh.faces[0].selected = true;
        for &vertex in &mesh.faces[0].verts {
            mesh.verts[vertex as usize].selected = true;
        }
    }
    state.sync_selection();
    state
        .dispatch(&petunia_core::ExtrudeSelectedCmd { dist: 2.0 })
        .expect("Extrude");

    // Undo e Redo
    assert!(state.undo());
    assert!(state.redo());

    // Salva projeto e exporta
    let prj_file = temp_path("perf_session.petunia");
    let glb_file = temp_path("perf_session.glb");

    ProjectService::save_project(&mut state, &prj_file).expect("Save");
    ProjectService::export_glb(&state, &[state.project.active], &glb_file).expect("Export");

    // Determinismo: recarregar preserva a cena; sem assert de tempo de parede.
    let mut reloaded = AppState::new("en");
    ProjectService::load_project(&mut reloaded, &prj_file).expect("Reload");
    assert_eq!(
        reloaded.project.assets.len(),
        state.project.assets.len(),
        "roundtrip deve preservar contagem de assets"
    );
    let glb_bytes = std::fs::read(&glb_file).expect("Ler GLB");
    assert_eq!(&glb_bytes[0..4], b"glTF", "magic GLB válido");

    let elapsed = start.elapsed();
    println!("⏱️ Sessão headless completa executada em: {:.2?}", elapsed);

    let _ = std::fs::remove_file(prj_file);
    let _ = std::fs::remove_file(glb_file);
}
