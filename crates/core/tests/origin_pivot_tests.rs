//! Pivô = origem do objeto (ADR-C16): mover é um rebase que não muda a geometria
//! no mundo, com Undo exato. Centro da face/aresta, origem do mundo e ponto livre.

use glam::Vec3;
use petunia_core::ProjectService;
use petunia_core::origin_commands::{origin_refusal, selection_center_world};
use petunia_core::state::AppState;
use petunia_mesh::Mesh;

/// Projeto com um cubo editável (não paramétrico) ativo, deslocado em (2, 0, 0).
fn scene() -> AppState {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.project.add("Caixa", Mesh::cube(2.0));
    state.project.active = state.project.assets.len() - 1;
    let asset = state.project.active_mut().expect("ativo");
    let mut transform = asset.transform;
    transform.translation = [2.0, 0.0, 0.0];
    asset.set_object_transform(transform);
    state
}

fn world(state: &AppState) -> Vec<Vec3> {
    state.project.active().expect("ativo").world_positions()
}

fn origin(state: &AppState) -> Vec3 {
    state.project.active().expect("ativo").origin_world()
}

fn same(a: &[Vec3], b: &[Vec3]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (*x - *y).length() < 1.0e-4)
}

#[test]
fn origin_to_the_world_origin_keeps_the_geometry_and_undoes_exactly() {
    let mut state = scene();
    let before = world(&state);
    let depth = state.project.undo.depth().0;
    state.set_origin_world().expect("pivô na origem do mundo");
    assert!(origin(&state).length() < 1.0e-5);
    assert!(
        same(&world(&state), &before),
        "a geometria no mundo não muda"
    );
    assert_eq!(state.project.undo.depth().0, depth + 1, "um passo de Undo");
    assert!(state.undo());
    assert!((origin(&state) - Vec3::new(2.0, 0.0, 0.0)).length() < 1.0e-5);
    assert!(same(&world(&state), &before));
}

#[test]
fn origin_to_a_free_point_keeps_the_geometry() {
    let mut state = scene();
    let before = world(&state);
    state
        .set_origin_point(Vec3::new(1.0, 2.0, 3.0))
        .expect("pivô num ponto");
    assert!((origin(&state) - Vec3::new(1.0, 2.0, 3.0)).length() < 1.0e-5);
    assert!(same(&world(&state), &before));
}

#[test]
fn a_non_finite_point_is_refused_without_touching_the_object() {
    let mut state = scene();
    let before = world(&state);
    assert!(
        state
            .set_origin_point(Vec3::new(f32::NAN, 0.0, 0.0))
            .is_err()
    );
    assert!(same(&world(&state), &before));
}

#[test]
fn origin_to_a_selected_face_is_its_center_not_the_whole_mesh() {
    let mut state = scene();
    let target = {
        let mesh = state.project.active_mesh_mut().expect("malha");
        mesh.deselect_all();
        mesh.faces[0].selected = true;
        let asset = state.project.active().expect("ativo");
        let corners = asset.mesh.faces[0].verts.clone();
        let sum: Vec3 = corners
            .iter()
            .map(|&vi| asset.to_world_point(asset.mesh.verts[vi as usize].vec()))
            .sum();
        sum / corners.len() as f32
    };
    state.set_origin_selection().expect("pivô na face");
    assert!(
        (origin(&state) - target).length() < 1.0e-4,
        "{:?} != {:?}",
        origin(&state),
        target
    );
    // O centro de uma face de um cubo nunca coincide com o centro do cubo.
    assert!((origin(&state) - Vec3::new(2.0, 0.0, 0.0)).length() > 0.5);
}

#[test]
fn origin_to_a_selected_edge_is_its_midpoint() {
    let mut state = scene();
    let (a, b) = {
        let mesh = state.project.active_mesh_mut().expect("malha");
        mesh.deselect_all();
        let edge = mesh.edges_unique()[0];
        mesh.selected_edges.extend([edge]);
        edge
    };
    let expected = {
        let asset = state.project.active().expect("ativo");
        (asset.to_world_point(asset.mesh.verts[a as usize].vec())
            + asset.to_world_point(asset.mesh.verts[b as usize].vec()))
            * 0.5
    };
    state.set_origin_selection().expect("pivô na aresta");
    assert!((origin(&state) - expected).length() < 1.0e-4);
}

#[test]
fn origin_to_a_selected_vertex_is_that_vertex() {
    let mut state = scene();
    let expected = {
        let mesh = state.project.active_mesh_mut().expect("malha");
        mesh.deselect_all();
        mesh.verts[3].selected = true;
        let asset = state.project.active().expect("ativo");
        asset.to_world_point(asset.mesh.verts[3].vec())
    };
    state.set_origin_selection().expect("pivô no vértice");
    assert!((origin(&state) - expected).length() < 1.0e-4);
}

#[test]
fn origin_to_the_selection_needs_a_selection() {
    let mut state = scene();
    state
        .project
        .active_mesh_mut()
        .expect("malha")
        .deselect_all();
    assert!(state.set_origin_selection().is_err());
}

#[test]
fn selection_center_is_none_without_elements() {
    let mut state = scene();
    state
        .project
        .active_mesh_mut()
        .expect("malha")
        .deselect_all();
    let asset = state.project.active().expect("ativo");
    assert!(selection_center_world(asset).is_err());
}

#[test]
fn locked_objects_say_why_the_pivot_cannot_move() {
    let mut state = scene();
    assert_eq!(origin_refusal(state.project.active().expect("ativo")), None);
    state.project.active_mut().expect("ativo").locked = true;
    assert!(origin_refusal(state.project.active().expect("ativo")).is_some());
    assert!(state.set_origin_world().is_err());
}
