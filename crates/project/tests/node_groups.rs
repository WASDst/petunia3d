//! Grupos de nós do editor de nós: modelo, poda, nomes, dados hostis e arquivos antigos.

use petunia_project::op_graph::{
    GraphError, MAX_GROUP_NAME_CHARS, MAX_GROUPS, NodeGroup, OpGraph, TYPE_BOOLEAN, TYPE_MIRROR,
};
use uuid::Uuid;

fn graph_with(count: usize) -> (OpGraph, Vec<Uuid>) {
    let mut graph = OpGraph::default();
    let ids = (0..count)
        .map(|i| {
            let kind = if i % 2 == 0 {
                TYPE_BOOLEAN
            } else {
                TYPE_MIRROR
            };
            graph.add_node(kind, &[], [i as f32 * 10.0, 0.0]).unwrap()
        })
        .collect();
    (graph, ids)
}

#[test]
fn grouping_creates_a_named_frame_with_a_unique_default_name() {
    let (mut graph, ids) = graph_with(4);
    let first = graph.group_nodes(&ids[..2], None).unwrap();
    let second = graph.group_nodes(&ids[2..], None).unwrap();
    let names: Vec<_> = graph.groups.iter().map(|g| g.name.clone()).collect();
    assert_eq!(names, ["Grupo de nós 1", "Grupo de nós 2"]);
    assert_ne!(first, second);
    assert_eq!(graph.group_of(ids[0]).map(|g| g.id), Some(first));
}

#[test]
fn a_node_belongs_to_one_group_and_emptied_groups_disappear() {
    let (mut graph, ids) = graph_with(3);
    graph.group_nodes(&ids[..2], None).unwrap();
    // Reagrupar um deles tira-o do grupo anterior; o outro continua.
    let second = graph.group_nodes(&ids[1..2], Some("Sozinho")).unwrap();
    assert_eq!(graph.groups.len(), 2);
    assert_eq!(graph.group_of(ids[1]).map(|g| g.id), Some(second));
    // Reagrupar o único membro do primeiro esvazia e remove o grupo.
    graph.group_nodes(&ids[..1], Some("Outro")).unwrap();
    assert_eq!(graph.groups.len(), 2, "o primeiro esvaziou e saiu");
    assert!(graph.groups.iter().all(|g| !g.nodes.is_empty()));
}

#[test]
fn removing_a_node_removes_it_from_its_group_and_empty_groups() {
    let (mut graph, ids) = graph_with(2);
    graph.group_nodes(&ids, Some("Dupla")).unwrap();
    graph.remove_node(ids[0]).unwrap();
    assert_eq!(graph.groups[0].nodes, vec![ids[1]]);
    graph.remove_node(ids[1]).unwrap();
    assert!(graph.groups.is_empty());
}

#[test]
fn ungroup_keeps_the_nodes() {
    let (mut graph, ids) = graph_with(2);
    let group = graph.group_nodes(&ids, None).unwrap();
    graph.ungroup(group).unwrap();
    assert!(graph.groups.is_empty());
    assert_eq!(graph.nodes.len(), 2);
    assert!(matches!(
        graph.ungroup(group),
        Err(GraphError::UnknownNode(_))
    ));
}

#[test]
fn grouping_validates_its_input() {
    let (mut graph, ids) = graph_with(2);
    assert!(graph.group_nodes(&[], None).is_err(), "nada para agrupar");
    assert!(matches!(
        graph.group_nodes(&[Uuid::new_v4()], None),
        Err(GraphError::UnknownNode(_))
    ));
    assert!(graph.groups.is_empty(), "uma recusa não deixa rastro");
    // Ids repetidos entram uma vez só.
    let group = graph.group_nodes(&[ids[0], ids[0]], None).unwrap();
    assert_eq!(graph.groups[0].nodes.len(), 1);
    assert_eq!(graph.groups[0].id, group);
    // Nome vazio ou longo demais é recusado (aparado antes).
    assert!(
        graph.group_nodes(&ids[1..], Some("   ")).is_ok(),
        "vazio cai no padrão"
    );
    let long = "x".repeat(MAX_GROUP_NAME_CHARS + 1);
    assert!(graph.group_nodes(&ids[1..], Some(&long)).is_err());
}

#[test]
fn rename_trims_refuses_clashes_and_bumps_the_revision() {
    let (mut graph, ids) = graph_with(2);
    let a = graph.group_nodes(&ids[..1], Some("Rodas")).unwrap();
    let b = graph.group_nodes(&ids[1..], Some("Chassi")).unwrap();
    let revision = graph.revision;
    graph.rename_group(a, "  Pneus ").unwrap();
    assert_eq!(graph.groups[0].name, "Pneus");
    assert!(graph.revision > revision);
    assert!(
        graph.rename_group(a, "Chassi").is_err(),
        "nome de outro grupo"
    );
    assert!(graph.rename_group(a, "").is_err());
    assert!(graph.rename_group(b, "Pneus").is_err());
    // O mesmo nome no próprio grupo não é conflito.
    let revision = graph.revision;
    graph.rename_group(a, "Pneus").unwrap();
    assert_eq!(graph.revision, revision, "sem mudança, sem revisão nova");
    assert!(graph.rename_group(Uuid::new_v4(), "X").is_err());
}

#[test]
fn moving_several_nodes_is_one_atomic_change() {
    let (mut graph, ids) = graph_with(3);
    let revision = graph.revision;
    graph
        .set_positions(&[(ids[0], [5.0, 6.0]), (ids[2], [7.0, 8.0])])
        .unwrap();
    assert_eq!(graph.node(ids[0]).unwrap().position, [5.0, 6.0]);
    assert_eq!(graph.node(ids[2]).unwrap().position, [7.0, 8.0]);
    assert_eq!(graph.revision, revision + 1, "uma única revisão");
    // Uma posição inválida recusa o lote inteiro, sem mover ninguém.
    let before = graph.node(ids[1]).unwrap().position;
    assert!(
        graph
            .set_positions(&[(ids[1], [1.0, 1.0]), (ids[0], [f32::NAN, 0.0])])
            .is_err()
    );
    assert_eq!(graph.node(ids[1]).unwrap().position, before);
}

#[test]
fn hostile_group_data_is_repaired_and_then_validates() {
    let (mut graph, ids) = graph_with(2);
    let ghost = Uuid::new_v4();
    graph.groups = vec![
        NodeGroup {
            id: Uuid::new_v4(),
            name: "Boa".into(),
            nodes: vec![ids[0], ghost, ids[0]],
        },
        NodeGroup {
            id: Uuid::new_v4(),
            name: "   ".into(),
            nodes: vec![ids[0], ids[1]],
        },
        NodeGroup {
            id: Uuid::new_v4(),
            name: "Vazia".into(),
            nodes: vec![ghost],
        },
    ];
    assert!(graph.validate().is_err(), "membro inexistente/duplicado");
    let fixes = graph.repair();
    assert!(fixes > 0);
    graph.validate().expect("reparado, passa na validação");
    assert_eq!(graph.groups.len(), 2, "o grupo só de fantasmas saiu");
    assert!(graph.groups.iter().all(|g| !g.name.trim().is_empty()));
    let members: usize = graph.groups.iter().map(|g| g.nodes.len()).sum();
    assert_eq!(members, 2, "cada nó em um grupo só");
}

#[test]
fn too_many_groups_are_trimmed_by_repair() {
    let (mut graph, ids) = graph_with(2);
    graph.groups = (0..MAX_GROUPS + 20)
        .map(|n| NodeGroup {
            id: Uuid::new_v4(),
            name: format!("G{n}"),
            nodes: vec![if n == 0 { ids[0] } else { ids[1] }],
        })
        .collect();
    graph.repair();
    assert!(graph.groups.len() <= MAX_GROUPS);
    graph.validate().expect("válido");
}

#[test]
fn a_valid_graph_keeps_its_revision_through_repair() {
    let (mut graph, ids) = graph_with(2);
    graph.group_nodes(&ids, Some("Par")).unwrap();
    let before = graph.clone();
    assert_eq!(graph.repair(), 0);
    assert_eq!(graph, before, "grafo válido sai idêntico, revisão incluída");
}

#[test]
fn files_written_before_groups_still_load_and_empty_groups_are_not_saved() {
    let (graph, ids) = graph_with(2);
    let json = serde_json::to_value(&graph).unwrap();
    assert!(
        json.get("groups").is_none(),
        "sem grupos, o campo nem é gravado"
    );
    let loaded: OpGraph = serde_json::from_value(json).unwrap();
    assert!(loaded.groups.is_empty());
    let mut grouped = graph.clone();
    grouped.group_nodes(&ids, Some("Par")).unwrap();
    let round: OpGraph = serde_json::from_value(serde_json::to_value(&grouped).unwrap()).unwrap();
    assert_eq!(round, grouped);
}
