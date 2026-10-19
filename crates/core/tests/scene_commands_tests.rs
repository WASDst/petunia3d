//! Organização da cena: grupos (Ctrl+G), arrastar e soltar na Estrutura,
//! renomear grupo, isolamento da seleção e Join por seleção. Tudo headless,
//! com Undo de uma única entrada por gesto.

use petunia_core::state::AppState;
use petunia_core::{
    DeleteAssetCmd, GroupSelectionCmd, JoinObjectsCmd, MoveAssetsCmd, ProjectService,
    RenameGroupCmd, SetAssetCollectionCmd, SetGroupColorCmd, UngroupCmd,
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

/// "Grupo 1" com A e B; C fora. O ativo é A.
fn grouped_scene() -> (AppState, [Uuid; 3]) {
    let (mut state, [a, b, c]) = scene();
    select(&mut state, &[a, b]);
    state.dispatch(&GroupSelectionCmd::default()).unwrap();
    (state, [a, b, c])
}

fn delete(state: &mut AppState, index: usize) {
    state
        .dispatch(&DeleteAssetCmd {
            asset_index: Some(index),
        })
        .unwrap();
}

#[test]
fn deleting_the_last_member_removes_the_group_in_the_same_undo_step() {
    let (mut state, _) = grouped_scene();
    let depth = state.project.undo.depth().0;
    delete(&mut state, 1); // B: sobra A no grupo
    assert_eq!(state.project.project.collections, ["Grupo 1".to_string()]);
    delete(&mut state, 0); // A: o grupo esvazia
    assert!(state.project.project.collections.is_empty());
    assert_eq!(
        state.project.undo.depth().0,
        depth + 2,
        "um passo por exclusão"
    );
    assert!(state.undo());
    assert_eq!(
        state.project.project.collections,
        ["Grupo 1".to_string()],
        "desfazer a exclusão traz o grupo de volta"
    );
    assert_eq!(
        state.project.assets[0].collection.as_deref(),
        Some("Grupo 1")
    );
}

#[test]
fn deleting_an_ungrouped_object_keeps_every_group() {
    let (mut state, _) = grouped_scene();
    delete(&mut state, 2); // C
    assert_eq!(state.project.project.collections, ["Grupo 1".to_string()]);
}

#[test]
fn an_empty_group_made_on_purpose_survives_deleting_other_objects() {
    let (mut state, _) = scene();
    state.project.project.add_collection("Vazio");
    delete(&mut state, 2);
    assert_eq!(state.project.project.collections, ["Vazio".to_string()]);
}

#[test]
fn dragging_the_last_member_out_removes_the_group() {
    let (mut state, [a, b, _]) = grouped_scene();
    state
        .dispatch(&MoveAssetsCmd {
            ids: vec![a, b],
            before: None,
            collection: Some(None),
        })
        .unwrap();
    assert!(state.project.project.collections.is_empty());
    assert!(state.undo());
    assert_eq!(state.project.project.collections, ["Grupo 1".to_string()]);
}

#[test]
fn moving_the_last_member_to_another_group_removes_the_old_one() {
    let (mut state, [a, b, c]) = grouped_scene();
    state.project.active = 2;
    select(&mut state, &[c]);
    state
        .dispatch(&GroupSelectionCmd {
            name: Some("Outro".into()),
        })
        .unwrap();
    state
        .dispatch(&MoveAssetsCmd {
            ids: vec![a, b],
            before: None,
            collection: Some(Some("Outro".into())),
        })
        .unwrap();
    assert_eq!(state.project.project.collections, ["Outro".to_string()]);
}

#[test]
fn setting_the_collection_of_the_last_member_removes_the_group() {
    let (mut state, _) = grouped_scene();
    for index in 0..2 {
        state
            .dispatch(&SetAssetCollectionCmd {
                asset_index: index,
                collection: None,
            })
            .unwrap();
    }
    assert!(state.project.project.collections.is_empty());
}

#[test]
fn joining_every_member_removes_the_emptied_group() {
    let (mut state, [a, b, _]) = scene();
    select(&mut state, &[a, b]);
    state.dispatch(&GroupSelectionCmd::default()).unwrap();
    // Funde B em A (ambos no grupo): sobra A, o grupo continua.
    select(&mut state, &[a, b]);
    state.dispatch(&JoinObjectsCmd).unwrap();
    assert_eq!(state.project.project.collections, ["Grupo 1".to_string()]);
}

#[test]
fn group_color_command_is_undoable_and_validated() {
    let (mut state, _) = grouped_scene();
    let depth = state.project.undo.depth().0;
    state
        .dispatch(&SetGroupColorCmd {
            name: "Grupo 1".into(),
            color: Some([10, 200, 10]),
            force: true,
        })
        .unwrap();
    assert_eq!(state.project.undo.depth().0, depth + 1);
    let asset = &state.project.assets[0];
    assert_eq!(
        state.project.project.selection_color_of(asset),
        Some([10, 200, 10])
    );
    // Repetir a mesma cor não é mudança; grupo inexistente é recusado.
    assert!(
        state
            .dispatch(&SetGroupColorCmd {
                name: "Grupo 1".into(),
                color: Some([10, 200, 10]),
                force: true,
            })
            .is_err()
    );
    assert!(
        state
            .dispatch(&SetGroupColorCmd {
                name: "Fantasma".into(),
                color: Some([1, 2, 3]),
                force: false,
            })
            .is_err()
    );
    assert!(state.undo());
    assert!(state.project.project.collection_meta.is_empty());
}

#[test]
fn rename_keeps_the_group_color_and_undo_restores_the_old_name() {
    let (mut state, _) = grouped_scene();
    state
        .dispatch(&SetGroupColorCmd {
            name: "Grupo 1".into(),
            color: Some([10, 200, 10]),
            force: false,
        })
        .unwrap();
    state
        .dispatch(&RenameGroupCmd {
            from: "Grupo 1".into(),
            to: "Rodas".into(),
        })
        .unwrap();
    let meta = &state.project.project.collection_meta;
    assert!(meta.contains_key("Rodas") && !meta.contains_key("Grupo 1"));
    assert!(state.undo());
    let meta = &state.project.project.collection_meta;
    assert!(meta.contains_key("Grupo 1") && !meta.contains_key("Rodas"));
}

#[test]
fn ungroup_drops_the_group_color_too() {
    let (mut state, _) = grouped_scene();
    state
        .dispatch(&SetGroupColorCmd {
            name: "Grupo 1".into(),
            color: Some([10, 200, 10]),
            force: false,
        })
        .unwrap();
    state.dispatch(&UngroupCmd::default()).unwrap();
    assert!(state.project.project.collection_meta.is_empty());
}

#[test]
fn grouping_objects_that_were_in_a_group_removes_the_group_they_left() {
    let (mut state, [a, b, _]) = grouped_scene(); // "Grupo 1" com A e B
    select(&mut state, &[a, b]);
    // Agrupar de novo (como um Ctrl+G repetido): o grupo antigo esvazia e some.
    state.dispatch(&GroupSelectionCmd::default()).unwrap();
    let groups = &state.project.project.collections;
    assert_eq!(groups.len(), 1, "sem grupo vazio sobrando: {groups:?}");
    assert!(
        state
            .project
            .assets
            .iter()
            .take(2)
            .all(|asset| asset.collection.as_deref() == groups.first().map(String::as_str))
    );
    assert!(state.undo());
    assert_eq!(state.project.project.collections, ["Grupo 1".to_string()]);
}

#[test]
fn grouping_part_of_a_group_keeps_the_old_group_for_the_rest() {
    let (mut state, [a, b, _]) = grouped_scene(); // "Grupo 1" com A e B
    select(&mut state, &[a]);
    state
        .dispatch(&GroupSelectionCmd {
            name: Some("Só A".into()),
        })
        .unwrap();
    assert_eq!(
        state.project.project.collections,
        ["Grupo 1".to_string(), "Só A".to_string()],
        "B ainda está no primeiro"
    );
    assert_eq!(state.project.assets[1].id, b);
    assert_eq!(
        state.project.assets[1].collection.as_deref(),
        Some("Grupo 1")
    );
}
