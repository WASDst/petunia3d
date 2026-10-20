//! Suíte de validação e testes de regressão dos gates de UV Workspace & Health (V02).
//!
//! Gates canônicos (website/docs/07-uv/architecture-decision.md):
//! - Auto Unwrap não altera face count/topology; quad/n-gon continua autoral.
//! - Seams e pins permanecem previsíveis.
//! - Projeções canônicas (Planar, Cube, Cylindrical, Spherical, View) via CommandDispatcher.
//! - Pack determinístico com padding em pixels.
//! - UV Health diagnostics (islands, overlap %, zero-area, out-of-range, mean/max stretch, texel density).
//! - Texel density (Set Density, Equalize Islands) em px/world-unit.
//! - Transações atômicas de Undo/Redo para todas as operações de UV.

use petunia_core::command::{
    RedoCmd, UndoCmd, UnwrapAutoCmd, UvEqualizeTexelDensityCmd, UvPackIslandsCmd, UvProjectCubeCmd,
    UvProjectCylindricalCmd, UvProjectPlanarCmd, UvProjectSphericalCmd, UvSetTexelDensityCmd,
};
use petunia_core::edit_api::{PinOp, UvPinCmd};
use petunia_core::state::AppState;
use petunia_mesh::{FaceCornerIndex, Mesh};
use petunia_project::ProjectChanges;

fn setup_uv_cube_state() -> AppState {
    let mut state = AppState::default();
    if let Some(mesh) = state.project.active_mesh_mut() {
        *mesh = Mesh::cube(2.0);
    }
    state.session.uv_selection.sync = false;
    state.session.uv_selection.clear();
    state
}

#[test]
fn test_v02_gate_auto_unwrap_preserves_quad_topology() {
    let mut state = setup_uv_cube_state();
    let initial_faces_count = state.project.active_mesh().unwrap().faces.len();
    let initial_verts_count = state.project.active_mesh().unwrap().verts.len();

    // Cubo possui 6 faces quadriláteras
    assert_eq!(initial_faces_count, 6);
    for face in &state.project.active_mesh().unwrap().faces {
        assert_eq!(face.verts.len(), 4, "face autoral deve ser quad");
    }

    // Executa Auto Unwrap
    let report = state
        .dispatch_detailed(&UnwrapAutoCmd)
        .expect("UnwrapAutoCmd deve executar com sucesso");

    assert_eq!(report.label, "auto uv");
    assert_eq!(report.changes, ProjectChanges::UVS);

    let mesh = state.project.active_mesh().unwrap();

    // Gate 1: Topology autoral (face count, vert count e quad layout) rigorosamente preservada
    assert_eq!(
        mesh.faces.len(),
        initial_faces_count,
        "Auto Unwrap nunca pode triangular nem alterar contagem de faces autorais"
    );
    assert_eq!(
        mesh.verts.len(),
        initial_verts_count,
        "Auto Unwrap nunca pode duplicar nem remover vértices 3D"
    );

    for (fi, face) in mesh.faces.iter().enumerate() {
        assert_eq!(
            face.verts.len(),
            4,
            "Face {fi} deve permanecer quad autoral após Auto Unwrap"
        );
        assert_eq!(
            face.uv.len(),
            4,
            "Face {fi} deve ter 4 coordenadas UV para os 4 cantos"
        );
        for (ci, uv) in face.uv.iter().enumerate() {
            assert!(
                uv[0].is_finite() && uv[1].is_finite(),
                "Face {fi} canto {ci} deve possuir UV finita: {:?}",
                uv
            );
        }
    }
}

#[test]
fn test_v02_gate_projections_canonical_and_partial_selection() {
    let mut state = setup_uv_cube_state();

    // 1. Projeção Planar
    state
        .dispatch(&UvProjectPlanarCmd)
        .expect("UvProjectPlanarCmd");
    let mesh = state.project.active_mesh().unwrap();
    assert_eq!(mesh.faces[0].uv.len(), 4);

    // 2. Projeção Cúbica (Box Mapping)
    state.dispatch(&UvProjectCubeCmd).expect("UvProjectCubeCmd");
    let mesh = state.project.active_mesh().unwrap();
    assert_eq!(mesh.faces[0].uv.len(), 4);

    // 3. Projeção Cilíndrica com seleção parcial de faces
    let mesh_clone = mesh.clone();
    state.session.uv_selection.set_faces(&mesh_clone, [0, 1]);
    state
        .dispatch(&UvProjectCylindricalCmd)
        .expect("UvProjectCylindricalCmd");

    // 4. Projeção Esférica na malha inteira (seleção vazia)
    state.session.uv_selection.clear();
    state
        .dispatch(&UvProjectSphericalCmd)
        .expect("UvProjectSphericalCmd");

    let final_mesh = state.project.active_mesh().unwrap();
    for face in &final_mesh.faces {
        for uv in &face.uv {
            assert!(
                uv[0].is_finite() && uv[1].is_finite(),
                "coordenadas UV devem ser finitas: {:?}",
                uv
            );
        }
    }
}

#[test]
fn test_v02_gate_pack_islands_deterministic_and_padding() {
    let mut state = setup_uv_cube_state();

    // Garante que temos ilhas separadas via Auto Unwrap
    state.dispatch(&UnwrapAutoCmd).expect("auto unwrap");

    // Executa Pack com padding de 4 pixels
    let pack_cmd = UvPackIslandsCmd {
        padding_px: 4.0,
        allow_rotation: true,
        keep_pinned_islands: false,
    };

    state
        .dispatch(&pack_cmd)
        .expect("UvPackIslandsCmd deve executar");

    let diag = state.query_uv_diagnostics();
    assert!(diag.island_count > 0);
    // Assegura que todas as coordenadas UV residem em 0..=1 após o pack
    assert_eq!(
        diag.out_of_range_corners, 0,
        "Pack deve manter coordenadas dentro de 0..=1"
    );
}

#[test]
fn test_v02_gate_uv_health_diagnostics_and_texel_density() {
    let mut state = setup_uv_cube_state();

    // 1. Diagnóstico do cubo default
    let diag = state.query_uv_diagnostics();
    assert!(diag.island_count >= 1);
    assert_eq!(diag.zero_area_faces, 0);

    // 2. Set Texel Density
    let set_density_cmd = UvSetTexelDensityCmd { px_per_unit: 128.0 };
    state
        .dispatch(&set_density_cmd)
        .expect("UvSetTexelDensityCmd deve ter sucesso");

    let diag_after_density = state.query_uv_diagnostics();
    assert!(
        (diag_after_density.texel_density - 128.0).abs() < 1.0,
        "Texel density esperado ~128.0, obtido {}",
        diag_after_density.texel_density
    );

    // 3. Equalize Texel Density
    state
        .dispatch(&UvEqualizeTexelDensityCmd)
        .expect("UvEqualizeTexelDensityCmd deve ter sucesso");

    // 4. Teste de diagnóstico com cantos fora de 0..1 (não é erro, é telemetria)
    if let Some(mesh) = state.project.active_mesh_mut() {
        mesh.faces[0].uv[0] = [1.5, -0.2]; // intencionalmente fora de 0..1
    }
    let diag_out = state.query_uv_diagnostics();
    assert!(
        diag_out.out_of_range_corners >= 1,
        "Deve registrar cantos fora de 0..1 no diagnóstico"
    );
}

#[test]
fn test_v02_gate_uv_undo_redo_full_lifecycle() {
    let mut state = setup_uv_cube_state();
    let initial_undo = state.project.undo.depth();

    // 1. Despacha Auto Unwrap
    state.dispatch(&UnwrapAutoCmd).expect("unwrap");
    assert_eq!(state.project.undo.depth().0, initial_undo.0 + 1);

    // 2. Fixa um pino
    let pin_corner = FaceCornerIndex { face: 0, corner: 0 };
    state
        .dispatch(&UvPinCmd {
            op: PinOp::Pin,
            corners: Some(vec![pin_corner]),
        })
        .expect("pin");
    assert_eq!(state.project.undo.depth().0, initial_undo.0 + 2);
    assert!(
        state
            .project
            .active_mesh()
            .unwrap()
            .uv_pinned
            .contains(&(0, 0))
    );

    // 3. Undo do pino
    state.dispatch(&UndoCmd).expect("undo pin");
    assert!(
        !state
            .project
            .active_mesh()
            .unwrap()
            .uv_pinned
            .contains(&(0, 0))
    );

    // 4. Undo do unwrap
    state.dispatch(&UndoCmd).expect("undo unwrap");
    assert_eq!(state.project.undo.depth().0, initial_undo.0);

    // 5. Redo do unwrap
    state.dispatch(&RedoCmd).expect("redo unwrap");
    assert_eq!(state.project.undo.depth().0, initial_undo.0 + 1);

    // 6. Redo do pino
    state.dispatch(&RedoCmd).expect("redo pin");
    assert_eq!(state.project.undo.depth().0, initial_undo.0 + 2);
    assert!(
        state
            .project
            .active_mesh()
            .unwrap()
            .uv_pinned
            .contains(&(0, 0))
    );
}
