//! Fluxo de prefabs no estado do editor: salvar não polui a cena, instanciar é undoável.

use petunia_core::AppState;

fn state_with_cube() -> AppState {
    let mut state = AppState::default();
    if state.project.assets.is_empty() {
        state.project.assets.push(petunia_project::Asset::new(
            "Cube",
            petunia_mesh::Mesh::cube(1.0),
        ));
        state.project.active = 0;
    }
    state
}

#[test]
fn saving_a_prefab_keeps_the_scene_unchanged() {
    let mut state = state_with_cube();
    let scene = state.project.assets.len();
    let id = state.save_selection_as_prefab(None).expect("prefab");
    assert_eq!(state.project.assets.len(), scene);
    assert_eq!(state.project.prefabs.len(), 1);
    assert_eq!(state.project.prefabs[0].id, id);
    assert!(state.ui.status.contains(&state.project.prefabs[0].name));
}

#[test]
fn instantiating_places_a_selected_copy_and_undo_removes_it() {
    let mut state = state_with_cube();
    let scene = state.project.assets.len();
    let id = state.save_selection_as_prefab(Some("Crate")).unwrap();
    assert!(state.instantiate_prefab(id, Some([5.0, 0.0, 0.0])));
    assert_eq!(state.project.assets.len(), scene + 1);
    assert_eq!(state.session.selection.assets.len(), 1);
    assert_eq!(state.project.prefab_instance_count(id), 1);
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), scene, "undo remove a instância");
    assert_eq!(state.project.prefabs.len(), 1, "o prefab permanece");
}

#[test]
fn deleting_a_prefab_is_undoable_and_unknown_ids_are_refused() {
    let mut state = state_with_cube();
    let id = state.save_selection_as_prefab(None).unwrap();
    assert!(state.delete_prefab(id));
    assert!(state.project.prefabs.is_empty());
    assert!(state.undo());
    assert_eq!(state.project.prefabs.len(), 1);
    let unknown = uuid::Uuid::new_v4();
    assert!(!state.delete_prefab(unknown));
    assert!(!state.instantiate_prefab(unknown, None));
    assert!(!state.rename_prefab(id, "  "));
}
