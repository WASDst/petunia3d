//! Organização da cena: grupos (Ctrl+G), arrastar e soltar na Estrutura,
//! renomear grupo, isolamento da seleção e Join por seleção. Tudo headless,
//! com Undo de uma única entrada por gesto.

use petunia_core::state::AppState;
use petunia_core::{
    GroupSelectionCmd, JoinObjectsCmd, MoveAssetsCmd, ProjectService, RenameGroupCmd, UngroupCmd,
};
use uuid::Uuid;

/// Cena com três cubos A, B, C (nesta ordem); `A` ativo, sem seleção extra.
fn scene() -> (AppState, [Uuid; 3]) {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.project.assets[0].name = "A".into();
    state.project.add("B", petunia_mesh::Mesh::cube(1.0));
    state.project.add("C", petunia_mesh::Mesh::cube(1.0));
    state.project.active = 0;
    let ids = [
        state.project.assets[0].id,
        state.project.assets[1].id,
        state.project.assets[2].id,
    ];
    (state, ids)
}

fn names(state: &AppState) -> Vec<String> {
    state
        .project
        .assets
        .iter()
        .map(|a| a.name.clone())
        .collect()
}

fn select(state: &mut AppState, ids: &[Uuid]) {
    state.session.selection.assets = ids.to_vec();
}

#[test]
fn group_gathers_the_selection_in_one_undo_step() {
    let (mut state, [a, b, _c]) = scene();
    select(&mut state, &[a, b]);
    let before = state.project.undo.depth().0;
    state.dispatch(&GroupSelectionCmd::default()).unwrap();
    assert_eq!(state.project.undo.depth().0, before + 1);
    assert_eq!(
        state.project.project.collections,
        vec!["Grupo 1".to_string()]
    );
    let group = |i: usize| state.project.assets[i].collection.clone();
    assert_eq!(group(0).as_deref(), Some("Grupo 1"));
    assert_eq!(group(1).as_deref(), Some("Grupo 1"));
    assert_eq!(group(2), None, "C não estava selecionado");
    assert!(state.undo());
    assert!(state.project.project.collections.is_empty());
    assert!(state.project.assets.iter().all(|a| a.collection.is_none()));
}

#[test]
fn group_names_never_collide() {
    let (mut state, [a, b, c]) = scene();
    for (index, id) in [a, b, c].into_iter().enumerate() {
        state.project.active = index;
        select(&mut state, &[id]);
        state.dispatch(&GroupSelectionCmd::default()).unwrap();
    }
    assert_eq!(
        state.project.project.collections,
        vec![
            "Grupo 1".to_string(),
            "Grupo 2".to_string(),
            "Grupo 3".to_string()
        ]
    );
}

#[test]
fn ungroup_dissolves_the_group_and_keeps_the_objects() {
    let (mut state, [a, b, _]) = scene();
    select(&mut state, &[a, b]);
    state.dispatch(&GroupSelectionCmd::default()).unwrap();
    state.dispatch(&UngroupCmd::default()).unwrap();
    assert!(state.project.project.collections.is_empty());
    assert!(state.project.assets.iter().all(|a| a.collection.is_none()));
    assert_eq!(state.project.assets.len(), 3);
    // Sem grupo no ativo, desagrupar é recusado.
    assert!(state.dispatch(&UngroupCmd::default()).is_err());
}

#[test]
fn drop_reorders_before_a_target_and_moves_between_groups() {
    let (mut state, [a, b, c]) = scene();
    select(&mut state, &[a]);
    state.dispatch(&GroupSelectionCmd::default()).unwrap();

    // Arrastar C para antes de B: ordem A, C, B (um passo de Undo).
    let depth = state.project.undo.depth().0;
    state
        .dispatch(&MoveAssetsCmd {
            ids: vec![c],
            before: Some(b),
            collection: Some(None),
        })
        .unwrap();
    assert_eq!(names(&state), ["A", "C", "B"]);
    assert_eq!(state.project.undo.depth().0, depth + 1);

    // Soltar B sobre o grupo: entra em "Grupo 1" e vai para o fim.
    state
        .dispatch(&MoveAssetsCmd {
            ids: vec![b],
            before: None,
            collection: Some(Some("Grupo 1".into())),
        })
        .unwrap();
    fn group_of(state: &AppState, name: &str) -> Option<String> {
        state
            .project
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .and_then(|asset| asset.collection.clone())
    }
    assert_eq!(group_of(&state, "B").as_deref(), Some("Grupo 1"));
    assert_eq!(group_of(&state, "C"), None);

    // Desfazer devolve o grupo e a ordem anteriores.
    assert!(state.undo());
    assert_eq!(group_of(&state, "B"), None);
    assert_eq!(names(&state), ["A", "C", "B"]);
}

#[test]
fn drop_moves_several_objects_keeping_their_relative_order() {
    let (mut state, [a, b, c]) = scene();
    state
        .dispatch(&MoveAssetsCmd {
            ids: vec![c, a],
            before: Some(b),
            collection: None,
        })
        .unwrap();
    // A e C ficam antes de B, na ordem em que já estavam na cena (A antes de C).
    assert_eq!(names(&state), ["A", "C", "B"]);
}

#[test]
fn drop_keeps_the_same_active_object() {
    let (mut state, [a, b, c]) = scene();
    state.project.active = 2;
    let active = state.project.assets[2].id;
    assert_eq!(active, c);
    state
        .dispatch(&MoveAssetsCmd {
            ids: vec![c],
            before: Some(a),
            collection: None,
        })
        .unwrap();
    assert_eq!(state.project.active().map(|asset| asset.id), Some(c));
    let _ = b;
}

#[test]
fn drop_refuses_noops_missing_groups_and_self_targets() {
    let (mut state, [a, b, _]) = scene();
    // Mesma ordem e mesmo grupo: nada a alterar.
    assert!(
        state
            .dispatch(&MoveAssetsCmd {
                ids: vec![a],
                before: Some(b),
                collection: None,
            })
            .is_err()
    );
    // Grupo inexistente.
    assert!(
        state
            .dispatch(&MoveAssetsCmd {
                ids: vec![a],
                before: None,
                collection: Some(Some("Fantasma".into())),
            })
            .is_err()
    );
    // Soltar um objeto antes dele mesmo.
    assert!(
        state
            .dispatch(&MoveAssetsCmd {
                ids: vec![a],
                before: Some(a),
                collection: None,
            })
            .is_err()
    );
    assert_eq!(names(&state), ["A", "B", "C"]);
}

#[test]
fn drop_is_refused_while_isolated() {
    let (mut state, [a, _, c]) = scene();
    select(&mut state, &[a]);
    state.toggle_isolate();
    assert!(
        state
            .dispatch(&MoveAssetsCmd {
                ids: vec![c],
                before: Some(a),
                collection: None,
            })
            .is_err()
    );
}

#[test]
fn rename_group_updates_the_members_and_rejects_clashes() {
    let (mut state, [a, b, _]) = scene();
    select(&mut state, &[a]);
    state.dispatch(&GroupSelectionCmd::default()).unwrap();
    // O ativo acompanha a seleção (como na GUI): B é o único objeto escolhido.
    state.project.active = 1;
    select(&mut state, &[b]);
    state
        .dispatch(&GroupSelectionCmd {
            name: Some("Rodas".into()),
        })
        .unwrap();

    state
        .dispatch(&RenameGroupCmd {
            from: "Grupo 1".into(),
            to: "  Chassi ".into(),
        })
        .unwrap();
    assert_eq!(
        state.project.project.collections,
        vec!["Chassi".to_string(), "Rodas".to_string()]
    );
    assert_eq!(
        state.project.assets[0].collection.as_deref(),
        Some("Chassi")
    );
    // Nome vazio e nome já usado são recusados.
    assert!(
        state
            .dispatch(&RenameGroupCmd {
                from: "Chassi".into(),
                to: "   ".into(),
            })
            .is_err()
    );
    assert!(
        state
            .dispatch(&RenameGroupCmd {
                from: "Chassi".into(),
                to: "Rodas".into(),
            })
            .is_err()
    );
}

#[test]
fn isolation_keeps_the_whole_selection_and_restores_visibility() {
    let (mut state, [a, b, _]) = scene();
    select(&mut state, &[a, b]);
    state.toggle_isolate();
    let visible: Vec<bool> = state
        .project
        .assets
        .iter()
        .map(|asset| asset.visible)
        .collect();
    assert_eq!(visible, [true, true, false], "só a seleção fica na cena");

    // Trocar o ativo mantém a seleção isolada (não esconde o novo ativo).
    state.refresh_isolation();
    assert!(state.project.assets[1].visible);

    state.toggle_isolate();
    assert!(
        state.project.assets.iter().all(|asset| asset.visible),
        "sair do isolamento restaura as visibilidades"
    );
}

#[test]
fn join_merges_every_other_selected_object_into_the_active_one() {
    let (mut state, [a, b, c]) = scene();
    let verts_total: usize = state
        .project
        .assets
        .iter()
        .map(|asset| asset.mesh.verts.len())
        .sum();
    select(&mut state, &[a, b, c]);
    let depth = state.project.undo.depth().0;
    state.dispatch(&JoinObjectsCmd).unwrap();
    assert_eq!(state.project.undo.depth().0, depth + 1, "um único Undo");
    assert_eq!(state.project.assets.len(), 1);
    assert_eq!(state.project.assets[0].id, a);
    assert_eq!(state.project.assets[0].mesh.verts.len(), verts_total);
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 3);
}

#[test]
fn second_selected_asset_is_the_implicit_operand() {
    let (mut state, [a, b, _]) = scene();
    assert_eq!(state.second_selected_asset(), None);
    select(&mut state, &[a, b]);
    assert_eq!(state.second_selected_asset(), Some(b));
}

#[test]
fn group_command_needs_something_to_group() {
    let mut state = AppState::new("en");
    state.project.assets.clear();
    state.session.selection.assets.clear();
    assert!(state.dispatch(&GroupSelectionCmd::default()).is_err());
}
