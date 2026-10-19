//! Projeção da receita de nós do material ativo para a GUI (O05) e tradução de
//! pedidos `material_recipe_*` em `MaterialRecipeCmd`. Apresentação e roteamento
//! apenas: validação, Undo e reavaliação ficam no comando.

use petunia_core::{MaterialRecipeCmd, RecipeOp};
use petunia_project::material_recipe::{
    self, NODE_CATALOG, RecipeParamKind, RecipeParamSpec, RecipeParamValue,
};
use petunia_project::{Project, SocketType};
use serde_json::{Value, json};
use uuid::Uuid;

fn socket_id(socket: SocketType) -> &'static str {
    match socket {
        SocketType::Color => "color",
        SocketType::Value => "value",
        SocketType::Texture => "texture",
    }
}

fn spec_json(spec: &RecipeParamSpec) -> Value {
    let (kind, extra) = match spec.kind {
        RecipeParamKind::Float { min, max } => ("float", json!({"min": min, "max": max})),
        RecipeParamKind::Int { min, max } => ("int", json!({"min": min, "max": max})),
        RecipeParamKind::Bool => ("bool", json!({})),
        RecipeParamKind::Color => ("color", json!({})),
        RecipeParamKind::Choice(options) => ("choice", json!({"options": options})),
    };
    let mut object = json!({"key": spec.key, "label": spec.label, "kind": kind});
    if let (Some(map), Some(extra)) = (object.as_object_mut(), extra.as_object()) {
        map.extend(extra.clone());
    }
    object
}

fn value_json(value: &RecipeParamValue) -> Value {
    match value {
        RecipeParamValue::Float(v) => json!(v),
        RecipeParamValue::Int(v) => json!(v),
        RecipeParamValue::Bool(v) => json!(v),
        RecipeParamValue::Color(c) => json!(c),
        RecipeParamValue::Choice(c) => json!(c),
    }
}

/// Receita do material `material` (ou `{}` sem material).
pub(crate) fn snapshot(project: &Project, material: Option<Uuid>) -> Value {
    let catalog: Vec<Value> = NODE_CATALOG
        .iter()
        .map(|kind| json!({"id": kind.id, "label": kind.label, "category": kind.category}))
        .collect();
    let Some(material) = material.and_then(|id| project.materials.iter().find(|m| m.id == id))
    else {
        return json!({"material_id": null, "has_recipe": false, "catalog": catalog});
    };
    let Some(recipe) = &material.recipe else {
        return json!({
            "material_id": material.id.to_string(),
            "material_name": material.name,
            "has_recipe": false,
            "catalog": catalog,
        });
    };
    let nodes: Vec<Value> = recipe
        .graph
        .nodes
        .iter()
        .map(|node| {
            let kind = material_recipe::kind_of(&node.spec);
            let (inputs, outputs) = material_recipe::sockets(&node.spec);
            let params: serde_json::Map<String, Value> = material_recipe::params_of(&node.spec)
                .into_iter()
                .map(|(key, value)| (key.to_string(), value_json(&value)))
                .collect();
            json!({
                "id": node.id.to_string(),
                "kind": kind.map_or("unknown", |k| k.id),
                "label": kind.map_or("Node", |k| k.label),
                "category": kind.map_or("unknown", |k| k.category),
                "known": kind.is_some(),
                "params": params,
                "specs": kind.map(|k| k.params.iter().map(spec_json).collect::<Vec<_>>()).unwrap_or_default(),
                "position": recipe.positions.get(&node.id).copied().unwrap_or([0.0, 0.0]),
                "inputs": inputs.iter().map(|s| socket_id(*s)).collect::<Vec<_>>(),
                "outputs": outputs.iter().map(|s| socket_id(*s)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let edges: Vec<Value> = recipe
        .graph
        .edges
        .iter()
        .map(|edge| {
            let ty = recipe
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.from)
                .and_then(|n| n.spec.outputs().get(usize::from(edge.from_socket)).copied());
            json!({
                "from": edge.from.to_string(), "from_socket": edge.from_socket,
                "to": edge.to.to_string(), "to_socket": edge.to_socket,
                "type": ty.map_or("texture", socket_id),
            })
        })
        .collect();
    json!({
        "material_id": material.id.to_string(),
        "material_name": material.name,
        "has_recipe": true,
        "nodes": nodes,
        "edges": edges,
        "error": recipe.last_error,
        "generated": recipe.generated.iter().map(|c| material_recipe::channel_id(*c)).collect::<Vec<_>>(),
        "revision": recipe.graph.param_revision,
        "catalog": catalog,
    })
}

fn uuid(request: &Value, key: &str) -> Result<Uuid, String> {
    request[key]
        .as_str()
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or_else(|| format!("Identificador inválido: {key}"))
}

fn socket(request: &Value, key: &str) -> Result<u16, String> {
    request[key]
        .as_u64()
        .and_then(|v| u16::try_from(v).ok())
        .ok_or_else(|| format!("Soquete inválido: {key}"))
}

fn param_value(request: &Value) -> Result<RecipeParamValue, String> {
    let value = &request["value"];
    if let Some(flag) = value.as_bool() {
        return Ok(RecipeParamValue::Bool(flag));
    }
    if let Some(text) = value.as_str() {
        return Ok(RecipeParamValue::Choice(text.to_string()));
    }
    if let Some(array) = value.as_array() {
        let mut color = [1.0f32; 4];
        if array.len() < 3 || array.len() > 4 {
            return Err("Cor inválida".into());
        }
        for (slot, item) in color.iter_mut().zip(array) {
            *slot = item.as_f64().ok_or("Cor inválida")? as f32;
        }
        return Ok(RecipeParamValue::Color(color));
    }
    if let Some(number) = value.as_i64() {
        return Ok(RecipeParamValue::Int(number));
    }
    value
        .as_f64()
        .map(RecipeParamValue::Float)
        .ok_or_else(|| "Valor inválido".into())
}

/// Comando para um pedido `material_recipe_*` (ou `None` se não for um deles).
pub(crate) fn command_for(
    action: &str,
    request: &Value,
    material: Option<Uuid>,
) -> Result<Option<MaterialRecipeCmd>, String> {
    let op = match action {
        "material_recipe_create" => RecipeOp::Create,
        "material_recipe_add" => RecipeOp::AddNode {
            kind: request["kind"]
                .as_str()
                .ok_or("Tipo de nó inválido")?
                .to_string(),
            // C4: opcionalmente já ligado à saída arrastada, na posição da
            // soltura (menu ao arrastar uma saída).
            link: match request["from"].as_str() {
                Some(_) => Some((uuid(request, "from")?, socket(request, "from_socket")?)),
                None => None,
            },
            position: match request["x"].as_f64() {
                Some(x) => Some([
                    x as f32,
                    request["y"].as_f64().ok_or("Número inválido: y")? as f32,
                ]),
                None => None,
            },
        },
        "material_recipe_connect" => RecipeOp::Connect {
            from: uuid(request, "from")?,
            from_socket: socket(request, "from_socket")?,
            to: uuid(request, "to")?,
            to_socket: socket(request, "to_socket")?,
        },
        "material_recipe_disconnect" => RecipeOp::Disconnect {
            to: uuid(request, "to")?,
            to_socket: socket(request, "to_socket")?,
        },
        "material_recipe_set" => RecipeOp::SetParam {
            node: uuid(request, "node")?,
            key: request["key"]
                .as_str()
                .ok_or("Parâmetro inválido: key")?
                .to_string(),
            value: param_value(request)?,
        },
        "material_recipe_remove" => RecipeOp::RemoveNode {
            node: uuid(request, "node")?,
        },
        "material_recipe_move" => RecipeOp::MoveNode {
            node: uuid(request, "node")?,
            position: [
                request["x"].as_f64().ok_or("Número inválido: x")? as f32,
                request["y"].as_f64().ok_or("Número inválido: y")? as f32,
            ],
        },
        "material_recipe_bake" => RecipeOp::Detach,
        _ => return Ok(None),
    };
    let material = material.ok_or("Nenhum material ativo")?;
    Ok(Some(MaterialRecipeCmd::new(material, op)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_project::Material;

    #[test]
    fn requests_map_to_typed_commands_and_reject_bad_input() {
        let id = Uuid::new_v4();
        let cmd = command_for("material_recipe_add", &json!({"kind": "checker"}), Some(id))
            .unwrap()
            .unwrap();
        assert_eq!(
            cmd.op,
            RecipeOp::AddNode {
                kind: "checker".into(),
                link: None,
                position: None
            }
        );
        // C4: o pedido de nó já ligado traz a saída e a posição.
        let linked = command_for(
            "material_recipe_add",
            &json!({"kind": "tint", "from": id.to_string(), "from_socket": 0, "x": 12.0, "y": 34.0}),
            Some(id),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            linked.op,
            RecipeOp::AddNode {
                kind: "tint".into(),
                link: Some((id, 0)),
                position: Some([12.0, 34.0])
            }
        );
        let set = command_for(
            "material_recipe_set",
            &json!({"node": id.to_string(), "key": "color", "value": [1.0, 0.5, 0.0]}),
            Some(id),
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            set.op,
            RecipeOp::SetParam {
                value: RecipeParamValue::Color([1.0, 0.5, 0.0, 1.0]),
                ..
            }
        ));
        assert!(command_for("material_recipe_connect", &json!({"from": "x"}), Some(id)).is_err());
        assert!(command_for("material_recipe_create", &json!({}), None).is_err());
        assert!(
            command_for("graph_apply", &json!({}), Some(id))
                .unwrap()
                .is_none()
        );
        assert!(
            command_for(
                "material_recipe_set",
                &json!({"node": id.to_string(), "key": "k", "value": [1]}),
                Some(id)
            )
            .is_err()
        );
    }

    #[test]
    fn snapshot_describes_nodes_sockets_params_and_catalog() {
        let mut project = Project::default();
        let mut material = Material::new("Wood");
        let mut recipe = material_recipe::MaterialRecipe::new_for(&material);
        recipe.add_node("checker").unwrap();
        recipe.evaluate_into(&mut material);
        material.recipe = Some(recipe);
        let id = material.id;
        project.materials.push(material);
        let view = snapshot(&project, Some(id));
        assert_eq!(view["has_recipe"], true);
        let nodes = view["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        let checker = nodes.iter().find(|n| n["kind"] == "checker").unwrap();
        assert_eq!(checker["outputs"][0], "texture");
        assert_eq!(checker["params"]["cell_size"], 32);
        assert!(
            checker["specs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["kind"] == "color")
        );
        assert_eq!(view["edges"][0]["type"], "texture");
        assert!(view["catalog"].as_array().unwrap().len() >= 10);
        assert_eq!(snapshot(&project, None)["has_recipe"], false);
    }
}
