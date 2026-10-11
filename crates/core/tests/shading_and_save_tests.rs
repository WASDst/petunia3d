//! Sharp/Smooth by Angle/Orient (G18) e fidelidade do dirty no Save (P02).
//! Headless: sem UI, sem GPU.

use petunia_core::state::AppState;
use petunia_core::{
    ClearSharpCmd, MarkSharpCmd, OrientConsistentlyCmd, OrientOutwardCmd, SmoothByAngleCmd,
};
use petunia_mesh::Mesh;

fn state_with(mesh: Mesh) -> AppState {
    let mut state = AppState::new("en");
    state.project.project.assets.clear();
    state.project.project.add("Subject", mesh);
    state.project.project.active = 0;
    state
}

fn select_first_edge(state: &mut AppState) -> (u32, u32) {
    let mesh = state.project.active_mesh_mut().unwrap();
    let face = &mesh.faces[0];
    let edge = petunia_mesh::edge_key(face.verts[0], face.verts[1]);
    mesh.selected_edges.clear();
    mesh.selected_edges.insert(edge);
    edge
}

#[test]
fn mark_and_clear_sharp_are_undoable_and_independent_of_seams() {
    let mut state = state_with(Mesh::cube(1.0));
    let edge = select_first_edge(&mut state);

    state.dispatch(&MarkSharpCmd).unwrap();
    assert!(
        state
            .project
            .active_mesh()
            .unwrap()
            .sharp_edges
            .contains(&edge)
    );
    assert!(state.project.active_mesh().unwrap().uv_seams.is_empty());
    // Repetir não muda nada: sem entrada de histórico.
    let depth = state.project.undo.depth();
    assert!(state.dispatch(&MarkSharpCmd).is_err());
    assert_eq!(state.project.undo.depth(), depth);

    assert!(state.undo());
    assert!(state.project.active_mesh().unwrap().sharp_edges.is_empty());
    assert!(state.redo());
    assert!(
        state
            .project
            .active_mesh()
            .unwrap()
            .sharp_edges
            .contains(&edge)
    );

    state.dispatch(&ClearSharpCmd).unwrap();
    assert!(state.project.active_mesh().unwrap().sharp_edges.is_empty());
}

#[test]
fn sharp_requires_a_selected_edge() {
    let mut state = state_with(Mesh::cube(1.0));
    state
        .project
        .active_mesh_mut()
        .unwrap()
        .selected_edges
        .clear();
    assert!(state.dispatch(&MarkSharpCmd).is_err());
    assert!(state.dispatch(&ClearSharpCmd).is_err());
}

#[test]
fn smooth_by_angle_makes_the_object_smooth_and_records_sharp_edges() {
    let mut state = state_with(Mesh::cube(1.0));
    let id = state.project.active().unwrap().id;
    state
        .dispatch(&SmoothByAngleCmd { angle_deg: 30.0 })
        .unwrap();
    assert!(state.project.project.is_smooth_shaded(id));
    assert_eq!(state.project.active_mesh().unwrap().sharp_edges.len(), 12);
    assert!(state.undo());
    assert!(!state.project.project.is_smooth_shaded(id));
    assert!(state.project.active_mesh().unwrap().sharp_edges.is_empty());
}

#[test]
fn smooth_by_angle_rejects_out_of_range_angles() {
    let mut state = state_with(Mesh::cube(1.0));
    assert!(
        state
            .dispatch(&SmoothByAngleCmd { angle_deg: -1.0 })
            .is_err()
    );
    assert!(
        state
            .dispatch(&SmoothByAngleCmd { angle_deg: 181.0 })
            .is_err()
    );
    assert!(
        state
            .dispatch(&SmoothByAngleCmd {
                angle_deg: f32::NAN
            })
            .is_err()
    );
}

#[test]
fn orient_outward_refuses_open_meshes_but_orient_consistently_accepts_them() {
    let mut state = state_with(Mesh::plane(2.0));
    assert!(state.dispatch(&OrientOutwardCmd).is_err());
    state.dispatch(&OrientConsistentlyCmd).unwrap();
}

#[test]
fn orient_outward_fixes_an_inside_out_cube() {
    let mut cube = Mesh::cube(1.0);
    for face in &mut cube.faces {
        face.verts.reverse();
        face.uv.reverse();
    }
    let mut state = state_with(cube);
    state.dispatch(&OrientOutwardCmd).unwrap();
    let mesh = state.project.active_mesh().unwrap();
    let center = mesh.center();
    for fi in 0..mesh.faces.len() {
        let face_center = mesh.faces[fi]
            .verts
            .iter()
            .map(|&v| mesh.verts[v as usize].vec())
            .sum::<glam::Vec3>()
            / mesh.faces[fi].verts.len() as f32;
        assert!(mesh.face_normal(fi).dot(face_center - center) > 0.0);
    }
}

#[test]
fn legacy_smooth_list_migrates_to_the_object() {
    let mut state = state_with(Mesh::cube(1.0));
    let id = state.project.active().unwrap().id;
    state.project.project.smooth_shaded_assets.push(id);
    state.project.project.validate();
    assert!(state.project.project.is_smooth_shaded(id));
    assert!(
        state.project.project.smooth_shaded_assets.is_empty(),
        "a lista legada nunca é uma segunda autoridade"
    );
}

#[test]
fn sharp_edges_survive_a_save_load_round_trip() {
    let mut state = state_with(Mesh::cube(1.0));
    select_first_edge(&mut state);
    state.dispatch(&MarkSharpCmd).unwrap();
    let bytes = petunia_project::format::encode_zip(&state.project.project).unwrap();
    let back = petunia_project::format::load_bytes(&bytes).unwrap();
    assert_eq!(
        back.assets[0].mesh.sharp_edges,
        state.project.active_mesh().unwrap().sharp_edges
    );
}

#[test]
fn saving_an_older_revision_keeps_newer_edits_dirty() {
    let mut state = state_with(Mesh::cube(1.0));
    state.mark_document_clean();
    select_first_edge(&mut state);
    state.dispatch(&MarkSharpCmd).unwrap();
    assert!(state.is_document_dirty());

    // O Save começa neste estado...
    let ticket = state.project.save_point();
    // ...e o usuário edita enquanto a gravação ainda não terminou.
    state
        .dispatch(&SmoothByAngleCmd { angle_deg: 45.0 })
        .unwrap();
    state.project.mark_saved(ticket);
    assert!(
        state.is_document_dirty(),
        "salvar a revisão antiga não limpa a edição mais recente"
    );
    // Desfazer até o estado gravado volta a limpo.
    assert!(state.undo());
    assert!(!state.is_document_dirty());
}

#[test]
fn dirty_has_a_single_authority_in_the_history() {
    let mut state = state_with(Mesh::cube(1.0));
    state.mark_document_clean();
    assert!(!state.is_document_dirty());
    state.mark_document_dirty();
    assert!(state.is_document_dirty());
    state.mark_document_clean();
    assert!(!state.is_document_dirty());
}
