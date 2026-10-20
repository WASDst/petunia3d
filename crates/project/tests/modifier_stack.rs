use petunia_mesh::Mesh;
use petunia_project::{Asset, ModifierInstance, Project, export_obj, format};

#[test]
fn modifier_evaluation_does_not_mutate_base_mesh() {
    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    let base_vertices = asset.mesh.verts.len();
    let base_faces = asset.mesh.faces.len();
    asset.modifiers.push(ModifierInstance::mirror(0, 0.0));

    let evaluated = asset.evaluated_mesh();

    assert_eq!(asset.mesh.verts.len(), base_vertices);
    assert_eq!(asset.mesh.faces.len(), base_faces);
    assert_eq!(evaluated.verts.len(), base_vertices * 2);
    assert_eq!(evaluated.faces.len(), base_faces * 2);
}

#[test]
fn disabled_modifier_is_not_evaluated() {
    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    let mut mirror = ModifierInstance::mirror(0, 0.0);
    mirror.enabled = false;
    asset.modifiers.push(mirror);

    let evaluated = asset.evaluated_mesh();

    assert_eq!(evaluated.verts.len(), asset.mesh.verts.len());
    assert_eq!(evaluated.faces.len(), asset.mesh.faces.len());
}

#[test]
fn export_obj_uses_evaluated_modifier_stack() {
    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    asset.modifiers.push(ModifierInstance::mirror(0, 0.0));

    let obj = export_obj(&asset);
    let exported_vertices = obj.lines().filter(|line| line.starts_with("v ")).count();

    assert_eq!(exported_vertices, asset.mesh.verts.len() * 2);
}

#[test]
fn modifier_stack_roundtrips_through_project_format() {
    let mut project = Project::new();
    project.assets.clear();
    let mut asset = Asset::new("Modified Cube", Mesh::cube(2.0));
    let mirror = ModifierInstance::mirror(0, 0.0);
    let mirror_id = mirror.id;
    asset.modifiers.push(mirror);
    asset
        .modifiers
        .push(ModifierInstance::symmetry(1, true, 0.001));
    project.assets.push(asset);
    project.active = 0;

    let path = std::env::temp_dir().join(format!(
        "petunia_modifier_roundtrip_{}.petunia",
        uuid::Uuid::new_v4()
    ));
    format::save(&project, &path).unwrap();
    let loaded = format::load(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    // Carregar migra a pilha legada para o Operation Graph sem perda (O02).
    assert_eq!(loaded.assets.len(), 1);
    assert!(loaded.assets[0].modifiers.is_empty());
    let operations = loaded.op_graph.operations_on(loaded.assets[0].id);
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].id, mirror_id, "o nó herda o UUID do modifier");
    assert_eq!(
        operations[0].type_id,
        petunia_project::op_graph::TYPE_MIRROR
    );
    assert_eq!(
        operations[1].type_id,
        petunia_project::op_graph::TYPE_SYMMETRY
    );
    assert_eq!(
        loaded.assets[0].evaluated_mesh().verts.len(),
        project.assets[0].evaluated_mesh().verts.len()
    );
}

#[test]
fn legacy_modifier_migration_is_lossless_and_ordered_before_graph_operations() {
    use petunia_project::op_graph::{ParamValue, TYPE_MIRROR, TYPE_SYMMETRY};

    let mut project = Project::new();
    project.assets.clear();
    let mut mesh = Mesh::cube(2.0);
    for vertex in &mut mesh.verts {
        vertex.pos[0] += 1.5;
        vertex.pos[1] += 0.4;
    }
    let mut asset = Asset::new("Legacy", mesh);
    let mirror = ModifierInstance::mirror(0, 0.0);
    let mut symmetry = ModifierInstance::symmetry(1, false, 0.002);
    symmetry.enabled = false;
    let ids = (mirror.id, symmetry.id);
    asset.modifiers.push(mirror);
    asset.modifiers.push(symmetry);
    let asset_id = asset.id;
    project.assets.push(asset);
    project.active = 0;
    let before = project.assets[0].evaluated_mesh();

    // Uma operação de grafo pré-existente deve continuar depois do legado.
    let object = project.op_graph.ensure_object_node(asset_id).unwrap();
    let later = project
        .op_graph
        .add_node(
            TYPE_MIRROR,
            &[("axis", ParamValue::Choice("z".into()))],
            [0.0; 2],
        )
        .unwrap();
    project.op_graph.set_enabled(later, false).unwrap();
    project.op_graph.connect(later, 0, object, 0).unwrap();

    assert_eq!(project.migrate_legacy_modifiers(), 2);
    assert!(project.assets[0].modifiers.is_empty());
    let ops = project.op_graph.operations_on(asset_id);
    let order: Vec<_> = ops.iter().map(|op| op.id).collect();
    assert_eq!(order, vec![ids.0, ids.1, later]);
    assert_eq!(ops[1].type_id, TYPE_SYMMETRY);
    assert!(!ops[1].enabled, "estado desligado preservado");
    assert_eq!(
        ops[1].params.get("axis"),
        Some(&ParamValue::Choice("y".into()))
    );
    assert_eq!(
        ops[1].params.get("positive_to_negative"),
        Some(&ParamValue::Bool(false))
    );

    // Mesmo resultado geométrico (a operação extra está desligada).
    let after = project.assets[0].evaluated_mesh();
    assert_eq!(after.verts.len(), before.verts.len());
    assert_eq!(after.faces.len(), before.faces.len());
    for (a, b) in after.verts.iter().zip(&before.verts) {
        let d = (0..3)
            .map(|i| (a.pos[i] - b.pos[i]).abs())
            .fold(0.0_f32, f32::max);
        assert!(d < 1e-5);
    }
    // Idempotente.
    assert_eq!(project.migrate_legacy_modifiers(), 0);
}

#[test]
fn legacy_modifier_with_out_of_range_weld_keeps_legacy_stack() {
    let mut project = Project::new();
    project.assets.clear();
    let mut asset = Asset::new("Weld", Mesh::cube(2.0));
    asset.modifiers.push(ModifierInstance::mirror(0, 50.0));
    project.assets.push(asset);
    project.active = 0;
    let graph_before = project.op_graph.clone();
    assert_eq!(project.migrate_legacy_modifiers(), 0);
    assert_eq!(
        project.assets[0].modifiers.len(),
        1,
        "sem perda: legado intacto"
    );
    assert_eq!(project.op_graph.nodes.len(), graph_before.nodes.len());
}

#[test]
fn modifier_cache_invalidates_when_properties_change() {
    use petunia_project::ModifierKind;

    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    let mirror = ModifierInstance::mirror(0, 0.0);
    asset.modifiers.push(mirror);

    let first_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(first_eval.verts.len(), 16);

    // Toggle enabled off (modifiers.len() does NOT change!)
    asset.modifiers[0].enabled = false;
    let second_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(second_eval.verts.len(), 8);

    // Toggle enabled back on
    asset.modifiers[0].enabled = true;
    let third_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(third_eval.verts.len(), 16);

    // Change axis from 0 to 1
    asset.modifiers[0].kind = ModifierKind::Mirror { axis: 1, weld: 0.0 };
    let fourth_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(fourth_eval.verts.len(), 16);
    assert_ne!(first_eval.verts[8].pos, fourth_eval.verts[8].pos);
}
