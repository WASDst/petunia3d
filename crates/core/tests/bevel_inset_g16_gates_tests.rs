//! Suíte de testes sintéticos e gates para Inset & Bevel (G16).
//!
//! Gates canônicos (website/docs/03-geometry/direct-modeling-semantics.md):
//! - Inset Region como default coplanar e preservação de Individual.
//! - Bevel edge-set transacional: recusa atômica em bloco quando há aresta sem suporte.
//! - Bevel clamp overlap: limitação de largura e feedback reportado.
//! - Proveniência de UV e material slots em Bevel de arestas e vértices.
//! - Last Operation: ajuste contínuo de valor no mesmo undo step e restauração segura em erro.

use petunia_core::command::BevelCmd;
use petunia_core::modal::{ModalError, ModalKind};
use petunia_core::state::AppState;
use petunia_mesh::Mesh;
use petunia_project::ProjectChanges;

fn setup_cube_with_materials(side: f32) -> AppState {
    let mut state = AppState::default();
    let mut mesh = Mesh::cube(side);
    for (i, face) in mesh.faces.iter_mut().enumerate() {
        face.material_slot = Some(i % 2);
    }
    if let Some(active_mesh) = state.project.active_mesh_mut() {
        *active_mesh = mesh;
    }
    state
}

#[test]
fn test_g16_gate_bevel_edge_set_transactional_refusal() {
    let mut state = setup_cube_with_materials(2.0);
    // Adiciona uma folha aberta não-manifold desconectada
    let mut sheet = Mesh::plane(1.0);
    for v in &mut sheet.verts {
        v.pos[0] += 10.0;
    }
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.join(&sheet);
        // Seleciona uma aresta válida do cubo e uma aresta de borda aberta do plano
        mesh.selected_edges.insert((0, 3));
        let open_edge = petunia_mesh::edge_key(8, 9);
        mesh.selected_edges.insert(open_edge);
    }

    let before = state.project.active_mesh().unwrap().clone();

    // Executa BevelCmd
    let result = state.dispatch_detailed(&BevelCmd {
        amount: 0.2,
        segments: 1,
        clamp_overlap: true,
        affect_vertices: false,
    });

    // Invariante: O conjunto inteiro deve ser recusado; nada muda na malha
    assert!(
        result.is_err(),
        "deve recusar conjunto contendo aresta não-suportada"
    );
    let after = state.project.active_mesh().unwrap();
    assert_eq!(after.verts.len(), before.verts.len());
    assert_eq!(after.faces.len(), before.faces.len());
}

#[test]
fn test_g16_gate_bevel_clamp_overlap_feedback() {
    let mut state = setup_cube_with_materials(2.0);
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.selected_edges.insert((0, 3));
    }

    // Offset de 1.0 numa aresta de 2.0 excede o limite com clamp ativo (0.45)
    let report = state
        .dispatch_detailed(&BevelCmd {
            amount: 1.0,
            segments: 1,
            clamp_overlap: true,
            affect_vertices: false,
        })
        .expect("BevelCmd deve executar com clamp");

    assert_eq!(report.label, "bevel");
    assert_eq!(report.changes, ProjectChanges::GEOMETRY);
    assert!(
        state.ui.status.contains("clamped"),
        "status deve reportar que a largura foi limitada: '{}'",
        state.ui.status
    );
}

#[test]
fn test_g16_gate_bevel_uv_and_material_provenance() {
    let mut state = setup_cube_with_materials(2.0);

    // 1. Bevel de aresta com material slot configurado
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.selected_edges.clear();
        mesh.selected_edges.insert((0, 3));
    }

    state
        .dispatch(&BevelCmd {
            amount: 0.2,
            segments: 2,
            clamp_overlap: true,
            affect_vertices: false,
        })
        .expect("bevel edges");

    let mesh = state.project.active_mesh().unwrap();

    // Todas as faces (modificadas e novas ribbons) devem possuir material_slot válido e UVs finitas
    for (fi, face) in mesh.faces.iter().enumerate() {
        assert!(
            face.material_slot.is_some(),
            "Face {fi} não pode perder o material slot silenciosamente"
        );
        assert_eq!(
            face.verts.len(),
            face.uv.len(),
            "Face {fi} deve ter correspondência exata entre vértices e UVs"
        );
        for (ci, uv) in face.uv.iter().enumerate() {
            assert!(
                uv[0].is_finite() && uv[1].is_finite(),
                "Face {fi} canto {ci} UV deve ser finita: {:?}",
                uv
            );
        }
    }

    // 2. Bevel de vértice com material slot configurado
    let mut vert_state = setup_cube_with_materials(2.0);
    if let Some(mesh) = vert_state.project.active_mesh_mut() {
        mesh.selected_edges.clear();
        for v in &mut mesh.verts {
            v.selected = false;
        }
        mesh.verts[0].selected = true;
    }

    vert_state
        .dispatch(&BevelCmd {
            amount: 0.2,
            segments: 1,
            clamp_overlap: true,
            affect_vertices: true,
        })
        .expect("bevel vertex");

    let vert_mesh = vert_state.project.active_mesh().unwrap();
    for (fi, face) in vert_mesh.faces.iter().enumerate() {
        assert!(
            face.material_slot.is_some(),
            "Face {fi} no bevel de vértice não pode perder material_slot"
        );
        assert_eq!(face.verts.len(), face.uv.len());
        for uv in &face.uv {
            assert!(uv[0].is_finite() && uv[1].is_finite());
        }
    }
}

#[test]
fn test_g16_gate_bevel_last_operation_continuous_adjustment() {
    let mut state = setup_cube_with_materials(2.0);
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.selected_edges.insert((0, 3));
    }

    // Inicia e confirma um gesto modal de Bevel
    state
        .begin_modal(ModalKind::Bevel)
        .expect("begin_modal Bevel");
    state
        .update_modal(glam::Vec3::ZERO, 0.2)
        .expect("update_modal Bevel");
    let last = state
        .commit_modal_gesture()
        .expect("commit_modal_gesture deve gerar LastOperation");

    let initial_depth = state.project.undo.depth();
    assert_eq!(initial_depth, (1, 0), "exatamente 1 entrada de Undo");
    assert!(state.last_operation_is_current(&last));

    // Ajusta o valor da última operação para 0.4
    let adjusted = state
        .adjust_last_operation(&last, 0.4)
        .expect("ajuste válido deve ter sucesso")
        .expect("deve devolver LastOperation atualizada");

    assert_eq!(adjusted.kind, ModalKind::Bevel);
    assert_eq!(adjusted.value, 0.4);
    // Invariante de Last Operation: profundidade de histórico permanece inalterada
    assert_eq!(state.project.undo.depth(), (1, 0));

    // Ajuste com valor inválido (negativo) é recusado e restaura o documento anterior
    let before_err = state.project.active_mesh().unwrap().clone();
    let err = state.adjust_last_operation(&adjusted, -0.5);
    assert_eq!(err, Err(ModalError::InvalidInput));
    assert_eq!(state.project.undo.depth(), (1, 0));

    let after_err = state.project.active_mesh().unwrap();
    assert_eq!(after_err.faces.len(), before_err.faces.len());
    assert_eq!(after_err.verts.len(), before_err.verts.len());
}
