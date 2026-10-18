//! Comandos do Operation Graph (ADR-C13): operações persistentes e re-editáveis.
//!
//! Toda mutação do grafo passa por aqui — UI, Lua e MCP usam o mesmo caminho,
//! com validação prévia (`can_execute`), checkpoint de Undo e recusa sem efeito
//! colateral. A malha autoral dos objetos nunca é alterada, exceto no **Bake**
//! explícito (Make Editable).

use petunia_project::op_graph::{
    GraphError, OpGraph, ParamValue, TYPE_BOOLEAN, TYPE_MIRROR, TYPE_SYMMETRY,
};
use petunia_project::{Project, ProjectChanges};
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

/// Operação sobre o grafo.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphOp {
    /// Booleana com `tool` como ferramenta; `targets` pode ser vazio (o usuário liga
    /// a saída depois). `hide_tool` oculta o objeto-ferramenta no documento.
    AddBoolean {
        tool: Uuid,
        targets: Vec<Uuid>,
        operation: String,
        hide_tool: bool,
    },
    /// Operação de um objeto só (`petunia.mirror` / `petunia.symmetry`).
    AddOperation {
        target: Uuid,
        type_id: String,
    },
    SetParam {
        node: Uuid,
        key: String,
        value: ParamValue,
    },
    SetEnabled {
        node: Uuid,
        enabled: bool,
    },
    /// Liga a saída da operação ao objeto (a operação passa a agir sobre ele).
    Apply {
        operation: Uuid,
        target: Uuid,
    },
    /// Desliga a operação do objeto.
    Unapply {
        operation: Uuid,
        target: Uuid,
    },
    RemoveNode {
        node: Uuid,
    },
    MoveNode {
        node: Uuid,
        position: [f32; 2],
    },
    /// Make Editable: grava o resultado avaliado na malha do objeto e remove as
    /// operações que agiam sobre ele.
    Bake {
        target: Uuid,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpGraphCmd {
    pub op: GraphOp,
}

impl OpGraphCmd {
    pub fn new(op: GraphOp) -> Self {
        Self { op }
    }
}

impl Default for OpGraphCmd {
    fn default() -> Self {
        Self {
            op: GraphOp::RemoveNode { node: Uuid::nil() },
        }
    }
}

fn refuse(error: GraphError) -> CommandError {
    CommandError::Execution(error.to_string())
}

fn asset_index(document: &Project, id: Uuid) -> Result<usize, CommandError> {
    document
        .assets
        .iter()
        .position(|asset| asset.id == id)
        .ok_or_else(|| CommandError::StaleReference(id.to_string()))
}

/// Aplica `op` ao documento. Usado tanto na validação (sobre uma cópia) quanto
/// na execução, para que as duas nunca divirjam.
fn apply(document: &mut Project, op: &GraphOp) -> Result<(), CommandError> {
    match op {
        GraphOp::AddBoolean {
            tool,
            targets,
            operation,
            hide_tool,
        } => {
            asset_index(document, *tool)?;
            if targets.contains(tool) {
                return Err(refuse(GraphError::SelfReference));
            }
            for target in targets {
                asset_index(document, *target)?;
            }
            let graph = &mut document.op_graph;
            let tool_node = graph.ensure_object_node(*tool).map_err(refuse)?;
            let boolean = graph
                .add_node(
                    TYPE_BOOLEAN,
                    &[("operation", ParamValue::Choice(operation.clone()))],
                    [220.0, 0.0],
                )
                .map_err(refuse)?;
            graph.connect(tool_node, 0, boolean, 0).map_err(refuse)?;
            for target in targets {
                let node = graph.ensure_object_node(*target).map_err(refuse)?;
                graph.connect(boolean, 0, node, 0).map_err(refuse)?;
            }
            if *hide_tool {
                let index = asset_index(document, *tool)?;
                document.assets[index].visible = false;
            }
            Ok(())
        }
        GraphOp::AddOperation { target, type_id } => {
            asset_index(document, *target)?;
            if type_id != TYPE_MIRROR && type_id != TYPE_SYMMETRY {
                return Err(CommandError::Execution(format!(
                    "`{type_id}` is not a single-object operation"
                )));
            }
            let graph = &mut document.op_graph;
            let object = graph.ensure_object_node(*target).map_err(refuse)?;
            let count = graph.operations_on(*target).len() as f32;
            let node = graph
                .add_node(type_id, &[], [220.0, count * 110.0])
                .map_err(refuse)?;
            graph.connect(node, 0, object, 0).map_err(refuse)?;
            Ok(())
        }
        GraphOp::SetParam { node, key, value } => document
            .op_graph
            .set_param(*node, key, value.clone())
            .map_err(refuse),
        GraphOp::SetEnabled { node, enabled } => document
            .op_graph
            .set_enabled(*node, *enabled)
            .map_err(refuse),
        GraphOp::Apply { operation, target } => {
            asset_index(document, *target)?;
            if document.op_graph.tool_of(*operation) == Some(*target) {
                return Err(refuse(GraphError::SelfReference));
            }
            let graph = &mut document.op_graph;
            let object = graph.ensure_object_node(*target).map_err(refuse)?;
            graph.connect(*operation, 0, object, 0).map_err(refuse)
        }
        GraphOp::Unapply { operation, target } => {
            let graph = &mut document.op_graph;
            let object = graph
                .object_node(*target)
                .ok_or_else(|| CommandError::StaleReference(target.to_string()))?;
            graph.disconnect(*operation, object).map_err(refuse)
        }
        GraphOp::RemoveNode { node } => {
            let tool = document.op_graph.tool_of(*node);
            let was_hidden_boolean = document
                .op_graph
                .node(*node)
                .is_some_and(|n| n.type_id == TYPE_BOOLEAN);
            document.op_graph.remove_node(*node).map_err(refuse)?;
            // A ferramenta volta a aparecer quando nenhuma booleana a usa mais.
            if was_hidden_boolean
                && let Some(tool) = tool
                && document.op_graph.operations_using_tool(tool).is_empty()
                && let Some(asset) = document.assets.iter_mut().find(|asset| asset.id == tool)
            {
                asset.visible = true;
            }
            Ok(())
        }
        GraphOp::MoveNode { node, position } => document
            .op_graph
            .set_position(*node, *position)
            .map_err(refuse),
        GraphOp::Bake { target } => {
            let index = asset_index(document, *target)?;
            let operations: Vec<Uuid> = document
                .op_graph
                .operations_on(*target)
                .iter()
                .map(|node| node.id)
                .collect();
            if operations.is_empty() {
                return Err(CommandError::Execution(
                    "no graph operations act on this object".into(),
                ));
            }
            let evaluation = document.op_graph.evaluate_asset(&document.assets, *target);
            let Some(mesh) = evaluation.mesh else {
                return Err(CommandError::Execution("nothing to bake".into()));
            };
            if mesh.faces.is_empty() {
                return Err(CommandError::Execution("the result is empty".into()));
            }
            let object = document.op_graph.object_node(*target);
            let mut tools = Vec::new();
            for operation in &operations {
                if let Some(tool) = document.op_graph.tool_of(*operation) {
                    tools.push(tool);
                }
                if let Some(object) = object {
                    let _ = document.op_graph.disconnect(*operation, object);
                }
                // Operação sem mais nenhum alvo: some do grafo.
                if document.op_graph.targets_of(*operation).is_empty() {
                    let _ = document.op_graph.remove_node(*operation);
                }
            }
            for tool in tools {
                if document.op_graph.operations_using_tool(tool).is_empty()
                    && let Some(asset) = document.assets.iter_mut().find(|asset| asset.id == tool)
                {
                    asset.visible = true;
                }
            }
            let asset = &mut document.assets[index];
            asset.mesh = mesh.into();
            asset.modifiers.clear();
            asset.parametric = None;
            asset.set_graph_cache(None);
            Ok(())
        }
    }
}

impl Command for OpGraphCmd {
    fn label(&self) -> &'static str {
        match self.op {
            GraphOp::AddBoolean { .. } => "add boolean operation",
            GraphOp::AddOperation { .. } => "add operation",
            GraphOp::SetParam { .. } => "edit operation",
            GraphOp::SetEnabled { .. } => "toggle operation",
            GraphOp::Apply { .. } => "apply operation to object",
            GraphOp::Unapply { .. } => "remove operation from object",
            GraphOp::RemoveNode { .. } => "remove operation",
            GraphOp::MoveNode { .. } => "move node",
            GraphOp::Bake { .. } => "make editable",
        }
    }

    fn changes(&self) -> ProjectChanges {
        match self.op {
            // Posição de nó não altera geometria.
            GraphOp::MoveNode { .. } => ProjectChanges::NONE,
            _ => ProjectChanges::GEOMETRY,
        }
    }

    /// As operações editam só o grafo; a malha autoral continua sendo a fonte.
    fn freezes_active_primitive(&self) -> bool {
        matches!(self.op, GraphOp::Bake { .. })
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        // Ensaio sobre uma cópia do grafo e dos objetos referenciados: a mesma
        // lógica da execução, sem efeito no documento.
        let mut rehearsal = Project {
            assets: state.document.assets.clone(),
            op_graph: state.document.op_graph.clone(),
            ..Project::default()
        };
        apply(&mut rehearsal, &self.op).map_err(|error| match error {
            CommandError::StaleReference(_) => "the referenced object no longer exists",
            _ => "the operation cannot be applied",
        })
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut rehearsal = Project {
            assets: state.document.assets.clone(),
            op_graph: state.document.op_graph.clone(),
            ..Project::default()
        };
        // A mensagem detalhada vem do ensaio; o documento só muda se ele passar.
        apply(&mut rehearsal, &self.op)?;
        apply(state.document, &self.op)
    }
}

/// Remove do grafo tudo que dependia de objetos que deixaram de existir.
pub fn prune_stale(graph: &mut OpGraph, existing: &[Uuid]) {
    let stale: Vec<Uuid> = graph
        .nodes
        .iter()
        .filter_map(|node| match node.params.get("asset") {
            Some(ParamValue::Asset(id)) if !existing.contains(id) => Some(*id),
            _ => None,
        })
        .collect();
    for id in stale {
        graph.forget_asset(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use petunia_mesh::Mesh;
    use petunia_project::Asset;

    fn state_with_two_cubes() -> (AppState, Uuid, Uuid) {
        let mut state = AppState::default();
        state.project.project.assets.clear();
        let mut wall = Asset::new("parede", Mesh::cube(2.0));
        let mut cutter = Asset::new("cortador", Mesh::cube(1.0));
        cutter.transform.translation = [1.0, 0.0, 0.0];
        wall.transform.translation = [0.0; 3];
        let (wall_id, cutter_id) = (wall.id, cutter.id);
        state.project.project.assets.push(wall);
        state.project.project.assets.push(cutter);
        state.project.project.active = 0;
        (state, wall_id, cutter_id)
    }

    fn boolean(tool: Uuid, targets: Vec<Uuid>) -> OpGraphCmd {
        OpGraphCmd::new(GraphOp::AddBoolean {
            tool,
            targets,
            operation: "difference".into(),
            hide_tool: true,
        })
    }

    /// Posições ordenadas (quantizadas) para comparar malhas independentemente da
    /// ordem de vértices.
    fn sorted_positions(mesh: &Mesh) -> Vec<[i64; 3]> {
        let mut out: Vec<[i64; 3]> = mesh
            .verts
            .iter()
            .map(|v| v.pos.map(|c| (f64::from(c) * 1e4).round() as i64))
            .collect();
        out.sort_unstable();
        out
    }

    /// O nó Boolean produz a mesma geometria que o kernel chamado direto, para as
    /// três operações, com e sem limpeza de topologia (O02). Booleanas existem
    /// só como nós: não há comando destrutivo equivalente.
    #[test]
    fn boolean_node_matches_the_kernel() {
        use petunia_mesh::boolean::{BooleanOp, boolean_meshes, boolean_meshes_clean};
        for (name, op) in [
            ("union", BooleanOp::Union),
            ("difference", BooleanOp::Difference),
            ("intersection", BooleanOp::Intersection),
        ] {
            for cleanup in [true, false] {
                let (mut graph_state, wall, cutter) = state_with_two_cubes();
                graph_state
                    .dispatch(&OpGraphCmd::new(GraphOp::AddBoolean {
                        tool: cutter,
                        targets: vec![wall],
                        operation: name.into(),
                        hide_tool: true,
                    }))
                    .unwrap();
                let node = graph_state
                    .project
                    .project
                    .op_graph
                    .operations_using_tool(cutter)[0]
                    .id;
                graph_state
                    .dispatch(&OpGraphCmd::new(GraphOp::SetParam {
                        node,
                        key: "cleanup".into(),
                        value: ParamValue::Bool(cleanup),
                    }))
                    .unwrap();
                graph_state.project.project.refresh_op_graph();
                assert!(
                    graph_state.project.project.op_graph_errors.is_empty(),
                    "{name}/{cleanup}: {:?}",
                    graph_state.project.project.op_graph_errors
                );
                let from_graph = graph_state.project.project.assets[0].evaluated_mesh();

                let assets = &graph_state.project.project.assets;
                let a = assets[0].mesh.to_owned_value();
                let b = assets[1].mesh_in_frame_of(&assets[0]);
                let from_kernel = if cleanup {
                    boolean_meshes_clean(&a, &b, op).unwrap().0
                } else {
                    let (mut ta, mut tb) = (a, b);
                    ta.triangulate();
                    tb.triangulate();
                    boolean_meshes(&ta, &tb, op).unwrap()
                };

                assert_eq!(
                    from_graph.faces.len(),
                    from_kernel.faces.len(),
                    "{name}/{cleanup}: faces"
                );
                assert_eq!(
                    sorted_positions(&from_graph),
                    sorted_positions(&from_kernel),
                    "{name}/{cleanup}: posições"
                );
            }
        }
    }

    #[test]
    fn add_boolean_is_one_undo_step_and_keeps_the_authored_mesh() {
        let (mut state, wall, cutter) = state_with_two_cubes();
        let authored = state.project.project.assets[0].mesh.to_owned_value();
        state.dispatch(&boolean(cutter, vec![wall])).unwrap();
        state.project.project.refresh_op_graph();
        let wall_asset = &state.project.project.assets[0];
        assert!(wall_asset.has_graph_result(), "resultado em cache");
        assert_eq!(
            wall_asset.mesh.to_owned_value().faces.len(),
            authored.faces.len()
        );
        assert!(wall_asset.evaluated_mesh().faces.len() > authored.faces.len());
        assert!(
            !state.project.project.assets[1].visible,
            "ferramenta oculta"
        );
        // Undo remove tudo de uma vez
        state.undo();
        state.project.project.refresh_op_graph();
        assert!(state.project.project.op_graph.is_empty());
        assert!(state.project.project.assets[1].visible);
        assert!(!state.project.project.assets[0].has_graph_result());
    }

    #[test]
    fn target_can_be_connected_after_creating_the_node() {
        let (mut state, wall, cutter) = state_with_two_cubes();
        state.dispatch(&boolean(cutter, vec![])).unwrap();
        let node = state.project.project.op_graph.operations_using_tool(cutter)[0].id;
        assert!(state.project.project.op_graph.targets_of(node).is_empty());
        state
            .dispatch(&OpGraphCmd::new(GraphOp::Apply {
                operation: node,
                target: wall,
            }))
            .unwrap();
        assert_eq!(state.project.project.op_graph.targets_of(node), vec![wall]);
        // o cortador não pode ser alvo da própria operação
        let error = state
            .dispatch(&OpGraphCmd::new(GraphOp::Apply {
                operation: node,
                target: cutter,
            }))
            .unwrap_err();
        assert!(error.to_string().contains("own tool") || !error.to_string().is_empty());
        state
            .dispatch(&OpGraphCmd::new(GraphOp::Unapply {
                operation: node,
                target: wall,
            }))
            .unwrap();
        assert!(state.project.project.op_graph.targets_of(node).is_empty());
    }

    #[test]
    fn invalid_commands_are_refused_without_undo_entries_or_changes() {
        let (mut state, wall, cutter) = state_with_two_cubes();
        let depth = state.project.undo.depth().0;
        // alvo é a própria ferramenta
        assert!(state.dispatch(&boolean(wall, vec![wall])).is_err());
        // operação inexistente
        assert!(
            state
                .dispatch(&OpGraphCmd::new(GraphOp::AddBoolean {
                    tool: cutter,
                    targets: vec![wall],
                    operation: "xor".into(),
                    hide_tool: true,
                }))
                .is_err()
        );
        // objeto fantasma
        assert!(state.dispatch(&boolean(Uuid::new_v4(), vec![])).is_err());
        assert!(state.project.project.op_graph.is_empty());
        assert!(state.project.project.assets[1].visible, "nada foi ocultado");
        assert_eq!(
            state.project.undo.depth().0,
            depth,
            "recusa não empilha Undo"
        );
    }

    #[test]
    fn param_edit_toggle_and_remove_round_trip_through_undo() {
        let (mut state, wall, cutter) = state_with_two_cubes();
        state.dispatch(&boolean(cutter, vec![wall])).unwrap();
        let node = state.project.project.op_graph.operations_using_tool(cutter)[0].id;
        state
            .dispatch(&OpGraphCmd::new(GraphOp::SetParam {
                node,
                key: "operation".into(),
                value: ParamValue::Choice("union".into()),
            }))
            .unwrap();
        assert_eq!(
            state.project.project.op_graph.node(node).unwrap().params["operation"],
            ParamValue::Choice("union".into())
        );
        state
            .dispatch(&OpGraphCmd::new(GraphOp::SetEnabled {
                node,
                enabled: false,
            }))
            .unwrap();
        state.project.project.refresh_op_graph();
        assert!(
            !state.project.project.assets[0].has_graph_result(),
            "desativada não age"
        );
        state.undo();
        state.undo();
        assert_eq!(
            state.project.project.op_graph.node(node).unwrap().params["operation"],
            ParamValue::Choice("difference".into())
        );
        state
            .dispatch(&OpGraphCmd::new(GraphOp::RemoveNode { node }))
            .unwrap();
        assert!(state.project.project.op_graph.is_empty());
        assert!(
            state.project.project.assets[1].visible,
            "ferramenta volta a aparecer"
        );
    }

    #[test]
    fn mirror_operation_applies_and_bake_makes_the_result_editable() {
        let (mut state, wall, _) = state_with_two_cubes();
        // desloca a malha para que o espelho não solde tudo em si mesmo
        state.project.project.assets[0]
            .mesh
            .transform_affine(glam::Mat4::from_translation(glam::Vec3::new(3.0, 0.0, 0.0)));
        let verts_before = state.project.project.assets[0].mesh.verts.len();
        state
            .dispatch(&OpGraphCmd::new(GraphOp::AddOperation {
                target: wall,
                type_id: TYPE_MIRROR.into(),
            }))
            .unwrap();
        state.project.project.refresh_op_graph();
        assert!(state.project.project.assets[0].evaluated_mesh().verts.len() > verts_before);
        state
            .dispatch(&OpGraphCmd::new(GraphOp::Bake { target: wall }))
            .unwrap();
        state.project.project.refresh_op_graph();
        let baked = &state.project.project.assets[0];
        assert!(
            baked.mesh.verts.len() > verts_before,
            "o resultado virou malha autoral"
        );
        assert!(
            state.project.project.op_graph.is_empty(),
            "as operações foram consumidas"
        );
        assert!(!baked.has_graph_result());
        // Bake sem operações é recusado
        assert!(
            state
                .dispatch(&OpGraphCmd::new(GraphOp::Bake { target: wall }))
                .is_err()
        );
    }

    #[test]
    fn moving_a_node_does_not_touch_geometry_or_the_cache() {
        let (mut state, wall, cutter) = state_with_two_cubes();
        state.dispatch(&boolean(cutter, vec![wall])).unwrap();
        state.project.project.refresh_op_graph();
        let key = state.project.project.assets[0].graph_cache_key();
        let node = state.project.project.op_graph.operations_using_tool(cutter)[0].id;
        state
            .dispatch(&OpGraphCmd::new(GraphOp::MoveNode {
                node,
                position: [40.0, 12.0],
            }))
            .unwrap();
        state.project.project.refresh_op_graph();
        assert_eq!(state.project.project.assets[0].graph_cache_key(), key);
        let error = state
            .dispatch(&OpGraphCmd::new(GraphOp::MoveNode {
                node,
                position: [f32::NAN, 0.0],
            }))
            .unwrap_err();
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn moving_the_tool_reevaluates_the_result() {
        let (mut state, wall, cutter) = state_with_two_cubes();
        state.dispatch(&boolean(cutter, vec![wall])).unwrap();
        state.project.project.refresh_op_graph();
        let before = state.project.project.assets[0].graph_cache_key();
        state.project.project.assets[1].transform.translation[0] = 0.2;
        state.project.project.refresh_op_graph();
        assert_ne!(state.project.project.assets[0].graph_cache_key(), before);
    }
}
