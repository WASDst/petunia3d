//! Oculto não é bloqueado: o objeto continua selecionável (Partes, nós) e
//! transformável (modal de mover, girar, escalar). Só `locked` recusa.

use glam::Vec3;
use petunia_core::state::AppState;
use petunia_core::{ModalKind, ProjectService};
use petunia_mesh::Mesh;

/// Cena com A, B e C; B oculto.
fn scene() -> AppState {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.project.assets[0].name = "A".into();
    state.project.add("B", Mesh::cube(1.0));
    state.project.add("C", Mesh::cube(1.0));
    state.project.assets[1].visible = false;
    state.project.active = 0;
    state
}

fn active_name(state: &AppState) -> String {
    state
        .project
        .active()
        .map(|asset| asset.name.clone())
        .unwrap_or_default()
}

#[test]
fn a_hidden_object_can_be_selected_by_index() {
    let mut state = scene();
    state.select_object(Some(1), false);
    assert_eq!(active_name(&state), "B");
    assert!(
        state
            .session
            .selection
            .assets
            .contains(&state.project.assets[1].id),
        "entrou na seleção"
    );
}

#[test]
fn a_hidden_object_can_join_a_multiple_selection() {
    let mut state = scene();
    state.select_object(Some(0), false);
    state.select_object(Some(1), true);
    assert_eq!(state.session.selection.assets.len(), 2);
}

#[test]
fn a_locked_object_is_still_refused() {
    let mut state = scene();
    state.project.assets[2].locked = true;
    state.select_object(Some(2), false);
    assert_ne!(active_name(&state), "C");
}

#[test]
fn select_all_still_skips_hidden_objects() {
    let mut state = scene();
    state.dispatch(&petunia_core::SelectAllCmd).unwrap();
    let hidden = state.project.assets[1].id;
    assert!(!state.session.selection.assets.contains(&hidden));
}

#[test]
fn a_hidden_object_can_be_moved_rotated_and_scaled() {
    for kind in [ModalKind::Move, ModalKind::Rotate, ModalKind::Scale] {
        let mut state = scene();
        state.select_object(Some(1), false);
        assert_eq!(active_name(&state), "B");
        let before = state.project.active().expect("ativo").transform;
        state
            .begin_modal(kind)
            .unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        let components = match kind {
            ModalKind::Move => Vec3::new(2.0, 0.0, 0.0),
            ModalKind::Rotate => Vec3::new(0.0, 30.0, 0.0),
            _ => Vec3::splat(2.0),
        };
        state.update_modal_components(components).expect("prévia");
        assert!(state.commit_modal(), "{kind:?} confirma");
        let after = state.project.active().expect("ativo").transform;
        assert_ne!(after, before, "{kind:?} alterou o objeto oculto");
        assert!(
            !state.project.active().expect("ativo").visible,
            "continua oculto"
        );
    }
}
