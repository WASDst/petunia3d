//! Projeção do Operation Graph para a GUI (snapshot) e tradução de pedidos `graph_*`
//! em comandos do núcleo. Apresentação e roteamento apenas: toda mutação é um
//! `OpGraphCmd` (validação, Undo e revisão pelo `CommandDispatcher`).

use petunia_core::{GraphOp, OpGraphCmd};
use petunia_project::Project;
use petunia_project::op_graph::{self, OpGraph, ParamKind, ParamSpec, ParamValue, TYPE_OBJECT};
use serde_json::{Value, json};
use uuid::Uuid;

fn param_json(value: &ParamValue) -> Value {
    match value {
        ParamValue::Bool(v) => json!(v),
        ParamValue::Int(v) => json!(v),
        ParamValue::Float(v) => json!(v),
        ParamValue::Choice(v) => json!(v),
        ParamValue::Asset(v) => json!(v.to_string()),
    }
}

fn spec_json(spec: &ParamSpec) -> Value {
    let (kind, extra) = match spec.kind {
        ParamKind::Bool => ("bool", json!({})),
        ParamKind::Int { min, max } => ("int", json!({"min": min, "max": max})),
        ParamKind::Float { min, max } => ("float", json!({"min": min, "max": max})),
        ParamKind::Choice(options) => ("choice", json!({"options": options})),
        ParamKind::Asset => ("asset", json!({})),
    };
    let mut object = json!({"key": spec.key, "label": spec.label, "kind": kind});
    if let (Some(map), Some(extra)) = (object.as_object_mut(), extra.as_object()) {
        map.extend(extra.clone());
    }
    object
}

fn asset_name(project: &Project, id: Uuid) -> String {
    project
        .assets
        .iter()
        .find(|asset| asset.id == id)
        .map_or_else(|| "?".to_string(), |asset| asset.name.clone())
}

/// Linha de operação (para o stack do objeto): gerada do descriptor.
fn operation_row(graph: &OpGraph, project: &Project, node: &op_graph::OpNode) -> Value {
    let descriptor = op_graph::descriptor(&node.type_id);
    let params: serde_json::Map<String, Value> = node
        .params
        .iter()
        .map(|(key, value)| (key.clone(), param_json(value)))
        .collect();
    let tool = graph.tool_of(node.id);
    json!({
        "id": node.id.to_string(),
        "type_id": node.type_id,
        "label": descriptor.map_or(node.type_id.as_str(), |d| d.label),
        "known": descriptor.is_some(),
        "enabled": node.enabled,
        "params": params,
        "specs": descriptor.map(|d| d.params.iter().map(spec_json).collect::<Vec<_>>()).unwrap_or_default(),
        "tool": tool.map(|id| json!({"id": id.to_string(), "name": asset_name(project, id)})),
        "targets": graph.targets_of(node.id).iter().map(|id| json!({"id": id.to_string(), "name": asset_name(project, *id)})).collect::<Vec<_>>(),
        "error": project.op_graph_errors.get(&node.id),
    })
}

/// Visão do grafo para o objeto ativo + o grafo completo (canvas de nós).
pub(crate) fn snapshot(project: &Project, active: Option<Uuid>) -> Value {
    let graph = &project.op_graph;
    let mut applied = Vec::new();
    let mut as_tool = Vec::new();
    let mut candidates = Vec::new();
    if let Some(active) = active {
        applied = graph
            .operations_on(active)
            .into_iter()
            .map(|node| operation_row(graph, project, node))
            .collect();
        as_tool = graph
            .operations_using_tool(active)
            .into_iter()
            .map(|node| operation_row(graph, project, node))
            .collect();
        // Objetos a que a operação pode ser aplicada (todos, menos a própria ferramenta).
        candidates = project
            .assets
            .iter()
            .filter(|asset| asset.id != active)
            .map(|asset| json!({"id": asset.id.to_string(), "name": asset.name}))
            .collect();
    }
    let nodes: Vec<Value> = graph
        .nodes
        .iter()
        .map(|node| {
            let descriptor = op_graph::descriptor(&node.type_id);
            let title = if node.type_id == TYPE_OBJECT {
                match node.params.get("asset") {
                    Some(ParamValue::Asset(id)) => asset_name(project, *id),
                    _ => "?".into(),
                }
            } else {
                descriptor.map_or(node.type_id.clone(), |d| d.label.to_string())
            };
            let ports = |ports: &[op_graph::PortSpec]| {
                ports
                    .iter()
                    .map(|p| json!({"name": p.name, "label": p.label, "type": p.ty.id(), "multi": p.multi}))
                    .collect::<Vec<_>>()
            };
            let mut row = operation_row(graph, project, node);
            if let Some(map) = row.as_object_mut() {
                map.insert("title".into(), json!(title));
                map.insert("position".into(), json!(node.position));
                map.insert("category".into(), json!(descriptor.map_or("unknown", |d| d.category)));
                map.insert("inputs".into(), json!(descriptor.map(|d| ports(d.inputs)).unwrap_or_default()));
                map.insert("outputs".into(), json!(descriptor.map(|d| ports(d.outputs)).unwrap_or_default()));
            }
            row
        })
        .collect();
    json!({
        "applied": applied,
        "as_tool": as_tool,
        "candidates": candidates,
        "nodes": nodes,
        "edges": graph.edges.iter().map(|e| {
            let port = |node: Uuid, output: bool, index: u16| {
                graph
                    .node(node)
                    .and_then(|n| op_graph::descriptor(&n.type_id))
                    .and_then(|d| if output { d.outputs } else { d.inputs }.get(usize::from(index)))
            };
            let from_port = port(e.from, true, e.from_port);
            let to_port = port(e.to, false, e.to_port);
            json!({
                "from": e.from.to_string(), "from_port": e.from_port,
                "to": e.to.to_string(), "to_port": e.to_port,
                "from_port_name": from_port.map(|p| p.name),
                "to_port_name": to_port.map(|p| p.name),
                "type": from_port.map_or("geometry", |p| p.ty.id()),
            })
        }).collect::<Vec<_>>(),
        "revision": graph.revision,
        "descriptors": op_graph::BUILTIN_NODES.iter().map(|d| json!({
            "type_id": d.type_id, "label": d.label, "category": d.category,
        })).collect::<Vec<_>>(),
    })
}

fn uuid(request: &Value, key: &str) -> Result<Uuid, String> {
    request[key]
        .as_str()
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or_else(|| format!("Identificador inválido: {key}"))
}

fn param_value(request: &Value) -> Result<ParamValue, String> {
    let value = &request["value"];
    if let Some(flag) = value.as_bool() {
        return Ok(ParamValue::Bool(flag));
    }
    if let Some(text) = value.as_str() {
        return Ok(match Uuid::parse_str(text) {
            Ok(id) if request["as_asset"].as_bool() == Some(true) => ParamValue::Asset(id),
            _ => ParamValue::Choice(text.to_string()),
        });
    }
    if let Some(number) = value.as_f64() {
        return Ok(if request["integer"].as_bool() == Some(true) {
            ParamValue::Int(number as i64)
        } else {
            ParamValue::Float(number)
        });
    }
    Err("Valor de parâmetro inválido".into())
}

/// Traduz `graph_*` em `OpGraphCmd`. `active` é o objeto ativo; `selected` a seleção.
pub(crate) fn command_for(
    action: &str,
    request: &Value,
    active: Option<Uuid>,
    selected: &[Uuid],
) -> Result<Option<OpGraphCmd>, String> {
    let op = match action {
        "graph_add_boolean" => {
            // A ferramenta é o objeto ativo; os demais selecionados já viram alvos.
            let tool = match request["tool"].as_str() {
                Some(_) => uuid(request, "tool")?,
                None => active.ok_or("Selecione o objeto que será a ferramenta")?,
            };
            GraphOp::AddBoolean {
                tool,
                targets: selected.iter().copied().filter(|id| *id != tool).collect(),
                operation: request["operation"]
                    .as_str()
                    .unwrap_or("difference")
                    .to_string(),
                hide_tool: request["hide_tool"].as_bool().unwrap_or(true),
            }
        }
        "graph_add_operation" => GraphOp::AddOperation {
            target: match request["target"].as_str() {
                Some(_) => uuid(request, "target")?,
                None => active.ok_or("Selecione um objeto")?,
            },
            type_id: match request["kind"].as_str() {
                Some("mirror") => op_graph::TYPE_MIRROR.into(),
                Some("symmetry") => op_graph::TYPE_SYMMETRY.into(),
                _ => return Err("Operação desconhecida".into()),
            },
        },
        "graph_set_param" => GraphOp::SetParam {
            node: uuid(request, "node")?,
            key: request["key"]
                .as_str()
                .ok_or("Parâmetro inválido: key")?
                .to_string(),
            value: param_value(request)?,
        },
        "graph_set_enabled" => GraphOp::SetEnabled {
            node: uuid(request, "node")?,
            enabled: request["enabled"]
                .as_bool()
                .ok_or("Booleano inválido: enabled")?,
        },
        "graph_apply" => GraphOp::Apply {
            operation: uuid(request, "operation")?,
            target: uuid(request, "target")?,
        },
        "graph_unapply" => GraphOp::Unapply {
            operation: uuid(request, "operation")?,
            target: uuid(request, "target")?,
        },
        "graph_remove" => GraphOp::RemoveNode {
            node: uuid(request, "node")?,
        },
        "graph_move_node" => GraphOp::MoveNode {
            node: uuid(request, "node")?,
            position: [
                request["x"].as_f64().ok_or("Número inválido: x")? as f32,
                request["y"].as_f64().ok_or("Número inválido: y")? as f32,
            ],
        },
        "graph_bake" => GraphOp::Bake {
            target: match request["target"].as_str() {
                Some(_) => uuid(request, "target")?,
                None => active.ok_or("Selecione um objeto")?,
            },
        },
        _ => return Ok(None),
    };
    Ok(Some(OpGraphCmd::new(op)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;
    use petunia_project::Asset;
    use petunia_project::op_graph::TYPE_BOOLEAN;

    #[test]
    fn requests_become_typed_commands() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let cmd = command_for(
            "graph_add_boolean",
            &json!({"operation": "union"}),
            Some(a),
            &[a, b],
        )
        .unwrap()
        .unwrap();
        match cmd.op {
            GraphOp::AddBoolean {
                tool,
                targets,
                operation,
                hide_tool,
            } => {
                assert_eq!(tool, a);
                assert_eq!(targets, vec![b], "a ferramenta nunca é alvo de si mesma");
                assert_eq!(operation, "union");
                assert!(hide_tool);
            }
            other => panic!("{other:?}"),
        }
        assert!(command_for("graph_add_boolean", &json!({}), None, &[]).is_err());
        assert!(
            command_for(
                "graph_add_operation",
                &json!({"kind": "twist"}),
                Some(a),
                &[a]
            )
            .is_err()
        );
        assert!(command_for("graph_set_param", &json!({"node": "no"}), Some(a), &[]).is_err());
        assert!(
            command_for("something_else", &json!({}), Some(a), &[])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn snapshot_describes_applied_and_tool_operations_for_the_ui() {
        let mut project = Project::new();
        project.assets.clear();
        let wall = Asset::new("parede", Mesh::cube(2.0));
        let cutter = Asset::new("cortador", Mesh::cube(1.0));
        let (wall_id, cutter_id) = (wall.id, cutter.id);
        project.assets.push(wall);
        project.assets.push(cutter);
        let object = project.op_graph.ensure_object_node(cutter_id).unwrap();
        let boolean = project
            .op_graph
            .add_node(TYPE_BOOLEAN, &[], [0.0; 2])
            .unwrap();
        project.op_graph.connect(object, 0, boolean, 0).unwrap();
        let wall_node = project.op_graph.ensure_object_node(wall_id).unwrap();
        project.op_graph.connect(boolean, 0, wall_node, 0).unwrap();
        let on_wall = snapshot(&project, Some(wall_id));
        assert_eq!(on_wall["applied"].as_array().unwrap().len(), 1);
        assert_eq!(on_wall["applied"][0]["tool"]["name"], "cortador");
        assert!(on_wall["as_tool"].as_array().unwrap().is_empty());
        let on_cutter = snapshot(&project, Some(cutter_id));
        assert_eq!(on_cutter["as_tool"].as_array().unwrap().len(), 1);
        assert_eq!(on_cutter["as_tool"][0]["targets"][0]["name"], "parede");
        assert_eq!(on_cutter["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(on_cutter["nodes"].as_array().unwrap().len(), 3);
        assert_eq!(on_cutter["edges"].as_array().unwrap().len(), 2);
        // a UI é gerada do descriptor: especificações dos parâmetros vêm publicadas
        let specs = on_cutter["as_tool"][0]["specs"].as_array().unwrap();
        assert!(
            specs
                .iter()
                .any(|s| s["key"] == "operation" && s["kind"] == "choice")
        );
    }
}
