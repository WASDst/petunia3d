//! Testes de integração e regressão para UV Selection & Transform (V01).
//!
//! Cobre:
//! - Pins tipados (UvPinCmd, PinOp::Pin, Unpin, Toggle, ClearAll com FaceCornerIndex)
//! - Split UV sem duplicar vértices 3D (UvSplitCmd insere seams sem alterar mesh.verts)
//! - Stitch UV com suporte stationary/moving e respeito a pins (UvStitchCmd)
//! - Transform modal preview, commit atômico (1 Undo) e cancel/Esc (0 Undo)
//! - Solver constraints e rejeição de floats não-finitos (NaN/Inf)

use glam::Vec2;
use petunia_core::UvTransformKind;
use petunia_core::command::{UvSplitCmd, UvStitchCmd};
use petunia_core::edit_api::{PinOp, UvPinCmd};
use petunia_core::state::AppState;
use petunia_mesh::{FaceCornerIndex, Mesh, UvTransform};
use petunia_project::ProjectChanges;
use std::collections::HashSet;

fn setup_uv_test_state() -> AppState {
    let mut state = AppState::default();
    let mesh = state
        .project
        .active_mesh()
        .expect("cubo padrão ativo")
        .clone();
    state.session.uv_selection.sync = false;
    state.session.uv_selection.set_faces(&mesh, [0]);
    state
}

#[test]
fn test_v01_typed_uv_pins_lifecycle() {
    let mut state = setup_uv_test_state();

    // 1. Fixar cantos específicos (FaceCornerIndex)
    let corner = FaceCornerIndex { face: 0, corner: 1 };
    state
        .dispatch(&UvPinCmd {
            op: PinOp::Pin,
            corners: Some(vec![corner]),
        })
        .expect("UvPinCmd::Pin deve ter sucesso");

    let mesh = state.project.active_mesh().unwrap();
    assert!(mesh.uv_pinned.contains(&(0, 1)));

    // 2. Toggle pin
    state
        .dispatch(&UvPinCmd {
            op: PinOp::Toggle,
            corners: Some(vec![corner]),
        })
        .expect("UvPinCmd::Toggle deve ter sucesso");

    let mesh = state.project.active_mesh().unwrap();
    assert!(!mesh.uv_pinned.contains(&(0, 1)));

    // 3. Clear all pins
    state
        .dispatch(&UvPinCmd {
            op: PinOp::Pin,
            corners: Some(vec![corner, FaceCornerIndex { face: 0, corner: 2 }]),
        })
        .expect("fixar cantos");
    assert!(!state.project.active_mesh().unwrap().uv_pinned.is_empty());

    state
        .dispatch(&UvPinCmd {
            op: PinOp::ClearAll,
            corners: None,
        })
        .expect("UvPinCmd::ClearAll deve ter sucesso");

    assert!(state.project.active_mesh().unwrap().uv_pinned.is_empty());
}

#[test]
fn test_v01_uv_split_never_duplicates_3d_vertices() {
    let mut state = setup_uv_test_state();
    let initial_verts_count = state.project.active_mesh().unwrap().verts.len();
    let initial_undo = state.project.undo.depth();

    // Split da face UV 0
    let report = state
        .dispatch_detailed(&UvSplitCmd)
        .expect("UvSplitCmd deve executar");

    assert_eq!(report.label, "uv split");
    assert_eq!(report.changes, ProjectChanges::UVS);
    assert_eq!(report.undo_depth, initial_undo.0 + 1);

    let mesh = state.project.active_mesh().unwrap();
    // Invariante essencial do contrato: Split UV nunca altera nem duplica vértices 3D
    assert_eq!(
        mesh.verts.len(),
        initial_verts_count,
        "Split UV não pode duplicar nem criar vértices 3D"
    );
    assert!(
        !mesh.uv_seams.is_empty(),
        "Split UV deve criar seams nas bordas da seleção"
    );
}

#[test]
fn test_v01_uv_stitch_stationary_moving_and_respects_pins() {
    let mut state = setup_uv_test_state();

    // Cria duas faces contíguas (edge 1-4) com UVs separadas e seam ativa
    let verts = vec![
        petunia_mesh::Vertex::new(0.0, 0.0, 0.0),
        petunia_mesh::Vertex::new(1.0, 0.0, 0.0),
        petunia_mesh::Vertex::new(2.0, 0.0, 0.0),
        petunia_mesh::Vertex::new(0.0, 1.0, 0.0),
        petunia_mesh::Vertex::new(1.0, 1.0, 0.0),
        petunia_mesh::Vertex::new(2.0, 1.0, 0.0),
    ];
    let left = petunia_mesh::Face::with_uv(
        vec![0, 1, 4, 3],
        vec![[0.0, 0.0], [0.2, 0.0], [0.2, 0.2], [0.0, 0.2]],
    );
    let right = petunia_mesh::Face::with_uv(
        vec![1, 2, 5, 4],
        vec![[0.8, 0.5], [0.8, 0.6], [0.7, 0.6], [0.7, 0.5]],
    );
    let mut pair_mesh = Mesh {
        verts,
        faces: vec![left, right],
        ..Mesh::default()
    };
    pair_mesh.uv_seams.insert(petunia_mesh::edge_key(1, 4));

    *state.project.active_mesh_mut().unwrap() = pair_mesh.clone();
    state.session.uv_selection.set_faces(&pair_mesh, [0, 1]);

    // Costura sem pins via UvStitchCmd: une as ilhas e remove a seam compartilhada
    state
        .dispatch(&UvStitchCmd)
        .expect("UvStitchCmd deve costurar a ilha");

    let mesh = state.project.active_mesh().unwrap();
    assert!(!mesh.uv_seams.contains(&petunia_mesh::edge_key(1, 4)));
    assert_eq!(mesh.uv_islands().len(), 1);

    // Se a ilha móvel estiver fixada (pinned), o stitch deve recusar
    let mut pinned_pair = pair_mesh;
    pinned_pair.pin_uv(1, 0); // fixa a face 1

    let stitch_report = pinned_pair.stitch_uv_to(0, &HashSet::new());
    assert_eq!(
        stitch_report.pinned_islands, 1,
        "Ilha com pin deve ser recusada para movimentação no Stitch"
    );
    assert_eq!(stitch_report.stitched_edges, 0);
}

#[test]
fn test_v01_uv_transform_modal_preview_commit_and_cancel_lifecycle() {
    let mut state = setup_uv_test_state();
    let original_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();
    let initial_undo = state.project.undo.depth();

    // 1. Ciclo de cancelamento (Esc): prévia não polui o documento nem o Undo
    state
        .begin_uv_transform(UvTransformKind::Move)
        .expect("begin_uv_transform deve ter sucesso");

    // 20 atualizações contínuas de drag
    for step in 1..=20 {
        state
            .update_uv_transform(UvTransform::Translate(Vec2::new(
                0.01 * step as f32,
                0.02 * step as f32,
            )))
            .expect("update_uv_transform");
    }

    // Documento canônico permanece inalterado durante a prévia
    let current_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();
    assert_eq!(current_uvs, original_uvs);

    // Cancelar (Esc)
    assert!(state.cancel_uv_transform());
    assert_eq!(state.project.undo.depth(), initial_undo);

    // 2. Ciclo de confirmação (Enter / Click): exatamente 1 entrada de Undo
    state
        .begin_uv_transform(UvTransformKind::Scale)
        .expect("begin_uv_transform");

    state
        .update_uv_transform(UvTransform::Scale(Vec2::splat(1.5)))
        .expect("update_uv_transform");

    // Rejeição de valores não-finitos
    assert!(
        state
            .update_uv_transform(UvTransform::Scale(Vec2::splat(f32::NAN)))
            .is_err(),
        "NaN deve ser rejeitado no UV transform"
    );

    // Commit atômico
    state
        .commit_uv_transform()
        .expect("commit_uv_transform deve ter sucesso");

    assert_eq!(
        state.project.undo.depth().0,
        initial_undo.0 + 1,
        "Commit do transform UV deve gerar exatamente uma única entrada de Undo"
    );

    // Desfazer restaura UVs originais
    assert!(state.undo());
    let restored_uvs: Vec<Vec<[f32; 2]>> = state
        .project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();
    assert_eq!(restored_uvs, original_uvs);
}
