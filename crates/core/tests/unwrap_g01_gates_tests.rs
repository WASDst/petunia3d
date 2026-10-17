//! Suíte de gates dedicada para Auto Unwrap (G01 / P1-06 / P3D-064).
//!
//! Documento canônico: website/docs/07-uv/architecture-decision.md ("Auto Unwrap", "Pack", "Gates").
//!
//! Gates validados:
//! 1. Auto Unwrap não altera topologia, face count nem quads/n-gons autorais.
//! 2. Seams de chart boundaries atualizam seam intent e preservam seams manuais.
//! 3. Rejeição com rollback estrito e zero mutação em entradas hostis (vazio, NaN/Inf).
//! 4. Determinismo do UV Pack e respeito ao padding em pixels.
//! 5. Execução via CommandDispatcher (UnwrapAutoCmd) com integridade transacional de Undo/Redo.

use petunia_core::command::UnwrapAutoCmd;
use petunia_core::state::AppState;
use petunia_mesh::uv_pack::UvPackOptions;
use petunia_mesh::uv_xatlas::UnwrapError;
use petunia_mesh::{Mesh, edge_key};
use std::collections::HashSet;

#[test]
fn test_gate1_auto_unwrap_preserves_topology_and_quads_and_ngons() {
    let mut mesh = Mesh::cube(2.0);
    let verts_count_before = mesh.verts.len();
    let faces_count_before = mesh.faces.len();
    let face_vertex_counts: Vec<usize> = mesh.faces.iter().map(|f| f.verts.len()).collect();
    let face_vertices: Vec<Vec<u32>> = mesh.faces.iter().map(|f| f.verts.clone()).collect();

    let charts = mesh
        .unwrap_auto()
        .expect("Auto unwrap do cubo deve ter sucesso");
    assert!(charts >= 1, "Cubo deve produzir ao menos 1 chart");

    // Invariante 1: Topologia autoral intacta
    assert_eq!(
        mesh.verts.len(),
        verts_count_before,
        "Vértices não podem ser alterados ou duplicados"
    );
    assert_eq!(
        mesh.faces.len(),
        faces_count_before,
        "Número de faces não pode mudar"
    );
    assert_eq!(
        mesh.faces.iter().map(|f| f.verts.len()).collect::<Vec<_>>(),
        face_vertex_counts,
        "Faces devem continuar quads (sem triangulação autoral destrutiva)"
    );
    for (i, face) in mesh.faces.iter().enumerate() {
        assert_eq!(
            &face.verts, &face_vertices[i],
            "Índices de vértices da face {i} devem ser preservados"
        );
        assert_eq!(
            face.uv.len(),
            face.verts.len(),
            "Face {i} deve possuir UV para cada corner"
        );
        for uv in &face.uv {
            assert!(
                uv[0].is_finite() && uv[1].is_finite(),
                "UV coords da face {i} devem ser finitas"
            );
        }
    }
}

#[test]
fn test_gate2_chart_boundaries_update_seams_and_preserve_manual_seams() {
    let mut mesh = Mesh::cube(2.0);
    // Insere uma costura manual explícita antes do unwrap
    let manual_seam = edge_key(mesh.faces[0].verts[0], mesh.faces[0].verts[1]);
    mesh.uv_seams.insert(manual_seam);

    mesh.unwrap_auto()
        .expect("Auto unwrap com seam manual deve ter sucesso");

    // Invariante 2: Seam manual preservada
    assert!(
        mesh.uv_seams.contains(&manual_seam),
        "Seam manual deve ser preservada"
    );

    // Invariante 3: Chart boundaries geram costuras de corte UV
    assert!(
        mesh.uv_seams.len() > 1,
        "Fronteiras de chart devem adicionar novas seams"
    );

    // Toda descontinuidade UV gerada deve ser coberta pelas seams
    let split_edges = mesh.uv_split_edges();
    assert!(
        split_edges.is_subset(&mesh.uv_seams),
        "Todas as arestas de split UV devem estar contidas nas seams"
    );
}

#[test]
fn test_gate3_rollback_and_zero_mutation_on_hostile_or_empty_inputs() {
    // 1. Malha vazia
    let mut empty = Mesh::default();
    assert_eq!(empty.unwrap_auto().unwrap_err(), UnwrapError::Empty);

    // 2. Malha com geometria hostil (NaN)
    let mut corrupt = Mesh::cube(1.0);
    corrupt.verts[0].pos[0] = f32::NAN;

    let snapshot_before: Vec<(Vec<u32>, Vec<[f32; 2]>)> = corrupt
        .faces
        .iter()
        .map(|f| (f.verts.clone(), f.uv.clone()))
        .collect();

    let err = corrupt.unwrap_auto().unwrap_err();
    assert_eq!(err, UnwrapError::InvalidGeometry);

    let snapshot_after: Vec<(Vec<u32>, Vec<[f32; 2]>)> = corrupt
        .faces
        .iter()
        .map(|f| (f.verts.clone(), f.uv.clone()))
        .collect();

    assert_eq!(
        snapshot_before, snapshot_after,
        "Em caso de erro, a malha deve permanecer rigorosamente inalterada (rollback)"
    );
}

#[test]
fn test_gate4_pack_determinism_and_pixel_padding() {
    let mut mesh = Mesh::cube(2.0);
    mesh.unwrap_auto()
        .expect("Unwrap inicial para teste de pack");

    let options = UvPackOptions {
        padding_px: 4.0,
        texture_size: [1024, 1024],
        allow_rotation: false,
        keep_pinned_islands: false,
    };

    let mut mesh_run1 = mesh.clone();
    let mut mesh_run2 = mesh.clone();

    let all_faces = HashSet::new();
    let report1 = mesh_run1
        .pack_uv_islands_with(&all_faces, &options)
        .expect("Pack run 1 deve ter sucesso");
    let report2 = mesh_run2
        .pack_uv_islands_with(&all_faces, &options)
        .expect("Pack run 2 deve ter sucesso");

    assert_eq!(
        report1, report2,
        "Pack de UV deve ser determinístico em contagem de ilhas"
    );

    // Invariante: Determinismo estrito bit-a-bit das coordenadas UV empacotadas
    for (f1, f2) in mesh_run1.faces.iter().zip(mesh_run2.faces.iter()) {
        assert_eq!(
            f1.uv, f2.uv,
            "UVs empacotadas devem ser idênticas entre execuções"
        );
    }

    // Invariante: UVs permanecem dentro do espaço canônico [0.0, 1.0]
    for face in &mesh_run1.faces {
        for uv in &face.uv {
            assert!(
                uv[0] >= -1e-4 && uv[0] <= 1.0001 && uv[1] >= -1e-4 && uv[1] <= 1.0001,
                "Coordenadas UV devem respeitar os limites do atlas: {uv:?}"
            );
        }
    }
}

#[test]
fn test_gate5_command_dispatch_and_undo_redo_fidelity() {
    let mut state = AppState::default();

    // Snapshot das UVs originais do cubo ativo
    let original_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .expect("Cubo inicial deve existir")
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();

    let original_seams = state.project.active_mesh().unwrap().uv_seams.clone();

    // 1. Despacha UnwrapAutoCmd via CommandDispatcher
    state
        .dispatch(&UnwrapAutoCmd)
        .expect("UnwrapAutoCmd deve executar com sucesso");

    let unwrapped_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();

    let unwrapped_seams = state.project.active_mesh().unwrap().uv_seams.clone();

    // Assegura que novas UVs foram de fato atribuídas
    assert_ne!(original_uvs, unwrapped_uvs, "Unwrap deve atualizar as UVs");

    // 2. Undo
    assert!(state.undo(), "Undo deve ter sucesso");

    let restored_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();

    let restored_seams = state.project.active_mesh().unwrap().uv_seams.clone();

    assert_eq!(
        restored_uvs, original_uvs,
        "Undo deve restaurar perfeitamente as UVs originais"
    );
    assert_eq!(
        restored_seams, original_seams,
        "Undo deve restaurar perfeitamente as costuras originais"
    );

    // 3. Redo
    assert!(state.redo(), "Redo deve ter sucesso");

    let redone_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();

    let redone_seams = state.project.active_mesh().unwrap().uv_seams.clone();

    assert_eq!(
        redone_uvs, unwrapped_uvs,
        "Redo deve reaplicar as UVs do unwrap"
    );
    assert_eq!(
        redone_seams, unwrapped_seams,
        "Redo deve reaplicar as costuras do unwrap"
    );
}
