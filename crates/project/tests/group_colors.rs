//! Cor de seleção por grupo (ADR-C16, item 1): o mais específico vence, a opção
//! "forçar" inverte a ordem, a poda do grupo vazio e o arquivo retrocompatível.

use petunia_mesh::Mesh;
use petunia_project::{CollectionMeta, Project, format};
use tempfile::tempdir;

const OBJECT: [u8; 3] = [200, 10, 10];
const GROUP: [u8; 3] = [10, 200, 10];

/// Projeto com `A` (índice 0) e `B` (índice 1) dentro do grupo "Rodas".
fn grouped() -> Project {
    let mut project = Project::new();
    project.assets[0].name = "A".into();
    project.add("B", Mesh::cube(1.0));
    assert!(project.add_collection("Rodas"));
    for asset in &mut project.assets {
        asset.collection = Some("Rodas".into());
    }
    project
}

#[test]
fn without_any_color_the_global_one_applies() {
    let project = grouped();
    assert_eq!(project.selection_color_of(&project.assets[0]), None);
}

#[test]
fn the_group_color_applies_to_members_without_their_own() {
    let mut project = grouped();
    assert!(project.set_collection_color("Rodas", Some(GROUP), false));
    assert_eq!(project.selection_color_of(&project.assets[0]), Some(GROUP));
    assert_eq!(project.selection_color_of(&project.assets[1]), Some(GROUP));
}

#[test]
fn the_most_specific_color_wins_by_default() {
    let mut project = grouped();
    project.set_collection_color("Rodas", Some(GROUP), false);
    project.assets[0].selection_overlay_color = Some(OBJECT);
    assert_eq!(project.selection_color_of(&project.assets[0]), Some(OBJECT));
    assert_eq!(project.selection_color_of(&project.assets[1]), Some(GROUP));
}

#[test]
fn forcing_the_group_color_overrides_the_objects() {
    let mut project = grouped();
    project.assets[0].selection_overlay_color = Some(OBJECT);
    project.set_collection_color("Rodas", Some(GROUP), true);
    assert_eq!(project.selection_color_of(&project.assets[0]), Some(GROUP));
}

#[test]
fn objects_outside_the_group_are_not_affected() {
    let mut project = grouped();
    project.set_collection_color("Rodas", Some(GROUP), true);
    project.assets[1].collection = None;
    assert_eq!(project.selection_color_of(&project.assets[1]), None);
}

#[test]
fn forcing_without_a_color_is_meaningless_and_cleans_up() {
    let mut project = grouped();
    assert!(
        !project.set_collection_color("Rodas", None, true),
        "nada a mudar"
    );
    assert!(project.collection_meta.is_empty());
    project.set_collection_color("Rodas", Some(GROUP), true);
    assert!(project.set_collection_color("Rodas", None, true));
    assert!(project.collection_meta.is_empty(), "sem cor, sem metadados");
}

#[test]
fn unknown_groups_are_refused() {
    let mut project = grouped();
    assert!(!project.set_collection_color("Fantasma", Some(GROUP), false));
    assert!(project.collection_meta.is_empty());
}

#[test]
fn rename_carries_the_color_and_refuses_clashes() {
    let mut project = grouped();
    project.add_collection("Chassi");
    project.set_collection_color("Rodas", Some(GROUP), true);
    assert!(!project.rename_collection("Rodas", "Chassi"), "nome em uso");
    assert!(!project.rename_collection("Rodas", "   "), "nome vazio");
    assert!(project.rename_collection("Rodas", "  Pneus "));
    assert_eq!(
        project.collection_meta.get("Pneus"),
        Some(&CollectionMeta {
            color: Some(GROUP),
            force_color: true
        })
    );
    assert!(!project.collection_meta.contains_key("Rodas"));
    assert_eq!(project.assets[0].collection.as_deref(), Some("Pneus"));
    assert_eq!(project.selection_color_of(&project.assets[0]), Some(GROUP));
}

#[test]
fn removing_the_last_member_removes_the_group_and_its_color() {
    let mut project = grouped();
    project.set_collection_color("Rodas", Some(GROUP), false);
    project.remove(1);
    assert_eq!(
        project.collections,
        vec!["Rodas".to_string()],
        "ainda tem A"
    );
    project.remove(0);
    assert!(project.collections.is_empty());
    assert!(project.collection_meta.is_empty());
}

#[test]
fn an_empty_group_made_on_purpose_survives_unrelated_removals() {
    let mut project = grouped();
    project.add_collection("Vazio");
    project.remove(1);
    assert!(project.collections.contains(&"Vazio".to_string()));
    assert!(
        !project.prune_collection_if_empty("Rodas"),
        "A ainda está nele"
    );
}

#[test]
fn group_color_survives_save_and_load() {
    let mut project = grouped();
    project.set_collection_color("Rodas", Some(GROUP), true);
    let dir = tempdir().unwrap();
    let path = dir.path().join("grupo.petunia");
    format::save(&project, &path).unwrap();
    let loaded = format::load(&path).unwrap();
    assert_eq!(loaded.collection_meta, project.collection_meta);
    assert_eq!(
        loaded.selection_color_of(&loaded.assets[0]),
        Some(GROUP),
        "a cor forçada continua valendo depois de recarregar"
    );
}

#[test]
fn files_written_before_group_colors_still_open() {
    let project = grouped();
    let mut json = serde_json::to_value(&project).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("collection_meta");
    let loaded: Project = serde_json::from_value(json).unwrap();
    assert!(loaded.collection_meta.is_empty());
    assert_eq!(loaded.collections, vec!["Rodas".to_string()]);
}
