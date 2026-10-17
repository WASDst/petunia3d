//! Camada 4 (dados hostis) do Operation Graph (O01): o grafo vem do arquivo e é
//! entrada não confiável. Mutações estruturais determinísticas sobre o
//! `document.json` de um projeto com Boolean + Mirror passam pelas mesmas etapas
//! do carregamento (`validate`, migração da pilha legada) e pela avaliação.
//! Invariantes: nunca há pânico, e depois de `validate()` o grafo é válido
//! (ou foi descartado) e cada objeto tem uma malha avaliada utilizável.

use petunia_mesh::Mesh;
use petunia_project::op_graph::{ParamValue, TYPE_BOOLEAN, TYPE_MIRROR};
use petunia_project::{Asset, ModifierInstance, Project};
use serde_json::{Value, json};

/// xorshift64*: reprodutível sem dependência nova.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn seed_project() -> Project {
    let mut project = Project::new();
    project.assets.clear();
    let wall = Asset::new("Wall", Mesh::cube(2.0));
    let mut cutter = Asset::new("Cutter", Mesh::cube(1.0));
    cutter.transform.translation = [0.8, 0.0, 0.0];
    let mut legacy = Asset::new("Legacy", Mesh::cube(1.0));
    legacy.modifiers.push(ModifierInstance::mirror(0, 0.001));
    let (wall_id, cutter_id) = (wall.id, cutter.id);
    project.assets.extend([wall, cutter, legacy]);
    project.active = 0;

    let graph = &mut project.op_graph;
    let wall_node = graph.ensure_object_node(wall_id).unwrap();
    let cutter_node = graph.ensure_object_node(cutter_id).unwrap();
    let boolean = graph
        .add_node(
            TYPE_BOOLEAN,
            &[("operation", ParamValue::Choice("difference".into()))],
            [0.0; 2],
        )
        .unwrap();
    let mirror = graph.add_node(TYPE_MIRROR, &[], [0.0; 2]).unwrap();
    graph.connect(cutter_node, 0, boolean, 0).unwrap();
    graph.connect(boolean, 0, wall_node, 0).unwrap();
    graph.connect(mirror, 0, wall_node, 0).unwrap();
    project.refresh_op_graph();
    assert!(project.op_graph_errors.is_empty(), "semente válida");
    project
}

fn random_uuid(rng: &mut Rng) -> Value {
    json!(uuid::Uuid::from_u64_pair(rng.next(), rng.next()).to_string())
}

fn hostile_param(rng: &mut Rng) -> Value {
    match rng.below(9) {
        0 => json!({"Choice": "xor"}),
        1 => json!({"Choice": "w"}),
        2 => json!({"Float": 1.0e308}),
        3 => json!({"Float": -1.0e308}),
        4 => json!({"Int": i64::MIN}),
        5 => json!({"Bool": true}),
        6 => json!({"Asset": uuid::Uuid::from_u64_pair(rng.next(), rng.next()).to_string()}),
        7 => json!({"Choice": "x".repeat(10_000)}),
        _ => json!("not-a-param-value"),
    }
}

fn node_at(graph: &mut Value, index: usize) -> Option<&mut Value> {
    graph.get_mut("nodes")?.get_mut(index)
}

fn push_edge(graph: &mut Value, edge: Value) {
    if let Some(edges) = graph.get_mut("edges").and_then(Value::as_array_mut) {
        edges.push(edge);
    }
}

/// Uma mutação estrutural sobre o objeto `op_graph` serializado.
fn mutate(graph: &mut Value, rng: &mut Rng) {
    let node_ids: Vec<Value> = graph
        .get("nodes")
        .and_then(Value::as_array)
        .map(|nodes| nodes.iter().map(|n| n["id"].clone()).collect())
        .unwrap_or_default();
    let pick = |rng: &mut Rng| {
        if node_ids.is_empty() {
            random_uuid(rng)
        } else {
            node_ids[rng.below(node_ids.len())].clone()
        }
    };
    match rng.below(16) {
        0 => {
            if let Some(node) = node_at(graph, rng.below(node_ids.len())) {
                node["type_id"] = json!("evil.unknown");
            }
        }
        1 => {
            let edge =
                json!({"from": random_uuid(rng), "from_port": 0, "to": pick(rng), "to_port": 0});
            push_edge(graph, edge);
        }
        2 => {
            let id = pick(rng);
            let edge = json!({"from": id.clone(), "from_port": 0, "to": id, "to_port": 0});
            push_edge(graph, edge);
        }
        3 => {
            let edge = json!({"from": pick(rng), "from_port": u16::MAX, "to": pick(rng), "to_port": rng.below(4)});
            push_edge(graph, edge);
        }
        4 => {
            // Operação ligada de volta à própria ferramenta: ciclo.
            let edge = json!({"from": pick(rng), "from_port": 0, "to": pick(rng), "to_port": 0});
            push_edge(graph, edge);
        }
        5 => {
            let key = [
                "operation",
                "axis",
                "weld",
                "cleanup",
                "asset",
                "positive_to_negative",
                "bogus",
            ][rng.below(7)];
            let value = hostile_param(rng);
            if let Some(params) = node_at(graph, rng.below(node_ids.len()))
                .and_then(|node| node.get_mut("params"))
                .and_then(Value::as_object_mut)
            {
                params.insert(key.into(), value);
            }
        }
        6 => {
            // Nó duplicado (mesmo id).
            if let Some(nodes) = graph.get_mut("nodes").and_then(Value::as_array_mut)
                && !nodes.is_empty()
            {
                let copy = nodes[rng.below(nodes.len())].clone();
                nodes.push(copy);
            }
        }
        7 => {
            if let Some(nodes) = graph.get_mut("nodes").and_then(Value::as_array_mut)
                && let Some(template) = nodes.first().cloned()
            {
                for _ in 0..600 {
                    let mut node = template.clone();
                    node["id"] = random_uuid(rng);
                    nodes.push(node);
                }
            }
        }
        8 => {
            if let Some(edges) = graph.get_mut("edges").and_then(Value::as_array_mut)
                && let Some(template) = edges.first().cloned()
            {
                for _ in 0..2100 {
                    edges.push(template.clone());
                }
            }
        }
        9 => graph["schema_version"] = json!(u32::MAX),
        10 => {
            if let Some(map) = graph.as_object_mut() {
                map.remove(["nodes", "edges"][rng.below(2)]);
            }
        }
        11 => {
            if let Some(node) = node_at(graph, rng.below(node_ids.len())) {
                node["position"] = json!([1.0e38, -1.0e38]);
            }
        }
        12 => {
            if let Some(node) = node_at(graph, rng.below(node_ids.len())) {
                node["version"] = json!(u32::MAX);
            }
        }
        13 => {
            // Objeto apontando para um asset que não existe.
            let asset = json!({"Asset": random_uuid(rng)});
            if let Some(params) = node_at(graph, rng.below(node_ids.len()))
                .and_then(|node| node.get_mut("params"))
                .and_then(Value::as_object_mut)
            {
                params.insert("asset".into(), asset);
            }
        }
        14 => {
            if let Some(edges) = graph.get_mut("edges").and_then(Value::as_array_mut)
                && !edges.is_empty()
            {
                let i = rng.below(edges.len());
                edges.remove(i);
            }
        }
        _ => graph["revision"] = json!(u64::MAX),
    }
}

/// Mesmo caminho do `format::finish_load` + avaliação usada pelo viewport/export.
fn load_and_evaluate(mut project: Project) {
    project.validate();
    assert!(
        project.op_graph.validate().is_ok(),
        "validate() deixa o grafo válido"
    );
    let _ = project.migrate_legacy_modifiers();
    let _ = project.refresh_op_graph();
    for asset in &project.assets {
        let mesh = asset.evaluated_mesh();
        assert!(
            mesh.verts
                .iter()
                .all(|v| v.pos.iter().all(|c| c.is_finite())),
            "malha avaliada com coordenada não finita"
        );
    }
}

#[test]
fn hostile_graph_documents_never_panic_and_leave_a_valid_graph() {
    let seed = serde_json::to_value(seed_project()).unwrap();
    let mut rng = Rng(0x5EED_0F_6A_7A9E);
    let (mut parsed, mut rejected) = (0, 0);
    for round in 0..400 {
        let mut document = seed.clone();
        let mutations = 1 + round % 4;
        for _ in 0..mutations {
            mutate(&mut document["op_graph"], &mut rng);
        }
        match serde_json::from_value::<Project>(document) {
            Ok(project) => {
                parsed += 1;
                load_and_evaluate(project);
            }
            // Tipo errado no JSON: erro tipado na desserialização, sem pânico.
            Err(_) => rejected += 1,
        }
    }
    assert!(
        parsed > 200,
        "a maioria das mutações chega à validação ({parsed} / {rejected})"
    );
}

#[test]
fn hostile_legacy_modifiers_never_panic_on_migration() {
    let mut seed = serde_json::to_value(seed_project()).unwrap();
    let legacy = seed["assets"][2]["modifiers"][0].clone();
    for (field, value) in [
        ("weld", json!(1.0e38)),
        ("weld", json!(-1.0)),
        ("axis", json!(usize::MAX)),
        ("axis", json!(7)),
    ] {
        let mut modifier = legacy.clone();
        for kind in ["Mirror", "Symmetry"] {
            if let Some(body) = modifier["kind"].get_mut(kind) {
                body[field] = value.clone();
            }
        }
        seed["assets"][2]["modifiers"] = json!([modifier]);
        if let Ok(project) = serde_json::from_value::<Project>(seed.clone()) {
            load_and_evaluate(project);
        }
    }
}
