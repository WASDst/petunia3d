//! Operation Graph (ADR-C13): operações persistentes e re-editáveis como nós.
//!
//! Modelo: **o nó é a operação; o fio aplica a operação a um objeto.**
//!
//! ```text
//!   [Objeto: Cortador].geometry ──▶ [Boolean: Diferença].tool
//!                                   [Boolean].operation ──▶ [Objeto: Parede].operations
//!                                   [Boolean].operation ──▶ [Objeto: Piso].operations   (fan-out)
//! ```
//!
//! Um nó `petunia.object` representa um objeto (asset) do documento: sua saída
//! `geometry` é a malha autoral (usada como ferramenta) e sua entrada
//! `operations` (multi) recebe as operações aplicadas a ele, na ordem das
//! arestas. A malha autoral do objeto **nunca é alterada**: o resultado vive no
//! cache de avaliação (`Asset::graph_cache`) até o Bake explícito.
//!
//! Registro modular: cada tipo de nó é um [`NodeDescriptor`] estático (portas
//! tipadas, esquema de parâmetros, versão). A UI (inspector, HUD, canvas) é
//! gerada do descriptor; um novo nó é um descriptor + um ramo em
//! [`OpGraph::apply_operation`]. Tipos desconhecidos (plugin ausente) são
//! preservados como nós fantasma e ignorados na avaliação, com aviso.
//!
//! Este módulo é dado puro: sem Qt, sem GPU, sem dependência de UI.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use petunia_mesh::Mesh;
use petunia_mesh::boolean::{BooleanOp, boolean_meshes_clean};

use crate::Asset;

/// Versão do schema serializado do grafo.
pub const OP_GRAPH_SCHEMA_VERSION: u32 = 1;
/// Limites duros (dados de arquivo são hostis).
pub const MAX_NODES: usize = 512;
pub const MAX_EDGES: usize = 2048;
/// Triângulos máximos aceitos como entrada de uma booleana.
pub const MAX_BOOLEAN_FACES: usize = 400_000;

pub const TYPE_OBJECT: &str = "petunia.object";
pub const TYPE_BOOLEAN: &str = "petunia.boolean";
pub const TYPE_MIRROR: &str = "petunia.mirror";
pub const TYPE_SYMMETRY: &str = "petunia.symmetry";

/// Tipo de dado que trafega numa porta.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    /// Malha de um objeto (saída de `petunia.object`).
    Geometry,
    /// Operação diferida, aplicável a um objeto.
    Operation,
}

impl PortType {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Geometry => "geometry",
            Self::Operation => "operation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortSpec {
    pub name: &'static str,
    pub label: &'static str,
    pub ty: PortType,
    /// Entrada que aceita várias arestas (ordem = ordem das arestas).
    pub multi: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamKind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Float {
        min: f64,
        max: f64,
    },
    Choice(&'static [&'static str]),
    /// Referência a um objeto do documento (`Uuid` de asset).
    Asset,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefaultValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Choice(&'static str),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub default: DefaultValue,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeDescriptor {
    pub type_id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub inputs: &'static [PortSpec],
    pub outputs: &'static [PortSpec],
    pub params: &'static [ParamSpec],
    /// Versão do tipo (migração de parâmetros quando o descriptor evolui).
    pub version: u32,
}

const BOOLEAN_OPERATIONS: &[&str] = &["union", "difference", "intersection"];
const AXES: &[&str] = &["x", "y", "z"];

const OBJECT: NodeDescriptor = NodeDescriptor {
    type_id: TYPE_OBJECT,
    label: "Object",
    category: "source",
    inputs: &[PortSpec {
        name: "operations",
        label: "Operations",
        ty: PortType::Operation,
        multi: true,
    }],
    outputs: &[PortSpec {
        name: "geometry",
        label: "Geometry",
        ty: PortType::Geometry,
        multi: false,
    }],
    params: &[ParamSpec {
        key: "asset",
        label: "Object",
        kind: ParamKind::Asset,
        default: DefaultValue::None,
    }],
    version: 1,
};

const BOOLEAN: NodeDescriptor = NodeDescriptor {
    type_id: TYPE_BOOLEAN,
    label: "Boolean",
    category: "operation",
    inputs: &[PortSpec {
        name: "tool",
        label: "Tool",
        ty: PortType::Geometry,
        multi: false,
    }],
    outputs: &[PortSpec {
        name: "operation",
        label: "Operation",
        ty: PortType::Operation,
        multi: false,
    }],
    params: &[
        ParamSpec {
            key: "operation",
            label: "Operation",
            kind: ParamKind::Choice(BOOLEAN_OPERATIONS),
            default: DefaultValue::Choice("difference"),
        },
        ParamSpec {
            key: "cleanup",
            label: "Clean up topology",
            kind: ParamKind::Bool,
            default: DefaultValue::Bool(true),
        },
    ],
    version: 1,
};

const MIRROR: NodeDescriptor = NodeDescriptor {
    type_id: TYPE_MIRROR,
    label: "Mirror",
    category: "operation",
    inputs: &[],
    outputs: &[PortSpec {
        name: "operation",
        label: "Operation",
        ty: PortType::Operation,
        multi: false,
    }],
    params: &[
        ParamSpec {
            key: "axis",
            label: "Axis",
            kind: ParamKind::Choice(AXES),
            default: DefaultValue::Choice("x"),
        },
        ParamSpec {
            key: "weld",
            label: "Weld distance",
            kind: ParamKind::Float {
                min: 0.0,
                max: 10.0,
            },
            default: DefaultValue::Float(0.001),
        },
    ],
    version: 1,
};

const SYMMETRY: NodeDescriptor = NodeDescriptor {
    type_id: TYPE_SYMMETRY,
    label: "Symmetry",
    category: "operation",
    inputs: &[],
    outputs: &[PortSpec {
        name: "operation",
        label: "Operation",
        ty: PortType::Operation,
        multi: false,
    }],
    params: &[
        ParamSpec {
            key: "axis",
            label: "Axis",
            kind: ParamKind::Choice(AXES),
            default: DefaultValue::Choice("x"),
        },
        ParamSpec {
            key: "positive_to_negative",
            label: "Positive to negative",
            kind: ParamKind::Bool,
            default: DefaultValue::Bool(true),
        },
        ParamSpec {
            key: "weld",
            label: "Weld distance",
            kind: ParamKind::Float {
                min: 0.0,
                max: 10.0,
            },
            default: DefaultValue::Float(0.001),
        },
    ],
    version: 1,
};

/// Registro estático de nós embutidos.
pub const BUILTIN_NODES: &[NodeDescriptor] = &[OBJECT, BOOLEAN, MIRROR, SYMMETRY];

pub fn descriptor(type_id: &str) -> Option<&'static NodeDescriptor> {
    BUILTIN_NODES.iter().find(|node| node.type_id == type_id)
}

/// Valor de parâmetro de um nó.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Choice(String),
    Asset(Uuid),
}

impl ParamValue {
    fn from_default(default: DefaultValue) -> Option<Self> {
        match default {
            DefaultValue::Bool(v) => Some(Self::Bool(v)),
            DefaultValue::Int(v) => Some(Self::Int(v)),
            DefaultValue::Float(v) => Some(Self::Float(v)),
            DefaultValue::Choice(v) => Some(Self::Choice(v.to_string())),
            DefaultValue::None => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum GraphError {
    #[error("node not found: {0}")]
    UnknownNode(Uuid),
    #[error("unknown node type: {0}")]
    UnknownType(String),
    #[error("invalid port `{port}` on {node}")]
    InvalidPort { node: Uuid, port: u16 },
    #[error("port types do not match: {from} → {to}")]
    TypeMismatch {
        from: &'static str,
        to: &'static str,
    },
    #[error("the connection would create a cycle")]
    Cycle,
    #[error("input already connected")]
    SocketOccupied,
    #[error("connection already exists")]
    DuplicateEdge,
    #[error("invalid parameter `{key}`: {reason}")]
    InvalidParam { key: String, reason: String },
    #[error("object not found: {0}")]
    MissingAsset(Uuid),
    #[error("an object cannot be its own tool")]
    SelfReference,
    #[error("graph limit exceeded: {0}")]
    TooLarge(&'static str),
    #[error("boolean failed: {0}")]
    Boolean(String),
}

/// Nó do grafo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpNode {
    pub id: Uuid,
    pub type_id: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub params: BTreeMap<String, ParamValue>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Posição no editor de nós (dado de documento, não afeta a avaliação).
    #[serde(default)]
    pub position: [f32; 2],
}

fn default_true() -> bool {
    true
}

/// Aresta `from.from_port → to.to_port`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpEdge {
    pub from: Uuid,
    pub from_port: u16,
    pub to: Uuid,
    pub to_port: u16,
}

/// Grafo de operações do documento.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpGraph {
    pub schema_version: u32,
    pub nodes: Vec<OpNode>,
    pub edges: Vec<OpEdge>,
    /// Sobe a cada mutação (invalidação de caches externos).
    pub revision: u64,
}

impl Default for OpGraph {
    fn default() -> Self {
        Self {
            schema_version: OP_GRAPH_SCHEMA_VERSION,
            nodes: Vec::new(),
            edges: Vec::new(),
            revision: 0,
        }
    }
}

/// Resultado da avaliação de um objeto.
#[derive(Clone, Debug, Default)]
pub struct AssetEvaluation {
    /// `None` = nenhuma operação ativa age neste objeto.
    pub mesh: Option<Mesh>,
    /// Erros por nó (não modais): a operação falha é ignorada e a cadeia segue.
    pub errors: Vec<(Uuid, String)>,
}

impl OpGraph {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: Uuid) -> Option<&OpNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    fn node_mut(&mut self, id: Uuid) -> Option<&mut OpNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    // ── Construção ──────────────────────────────────────────────────────────

    /// Cria um nó com os parâmetros padrão do descriptor (e `overrides` validados).
    pub fn add_node(
        &mut self,
        type_id: &str,
        overrides: &[(&str, ParamValue)],
        position: [f32; 2],
    ) -> Result<Uuid, GraphError> {
        if self.nodes.len() >= MAX_NODES {
            return Err(GraphError::TooLarge("nodes"));
        }
        let descriptor =
            descriptor(type_id).ok_or_else(|| GraphError::UnknownType(type_id.to_string()))?;
        let mut params = BTreeMap::new();
        for spec in descriptor.params {
            if let Some(value) = ParamValue::from_default(spec.default) {
                params.insert(spec.key.to_string(), value);
            }
        }
        for (key, value) in overrides {
            let spec = descriptor
                .params
                .iter()
                .find(|spec| spec.key == *key)
                .ok_or_else(|| GraphError::InvalidParam {
                    key: (*key).to_string(),
                    reason: "unknown parameter".into(),
                })?;
            validate_param(spec, value)?;
            params.insert((*key).to_string(), value.clone());
        }
        let node = OpNode {
            id: Uuid::new_v4(),
            type_id: type_id.to_string(),
            version: descriptor.version,
            params,
            enabled: true,
            position,
        };
        let id = node.id;
        self.nodes.push(node);
        self.bump();
        Ok(id)
    }

    /// Remove o nó e as arestas que o tocam. Nós `petunia.object` que ficam sem
    /// nenhuma aresta são descartados (não carregam estado próprio).
    pub fn remove_node(&mut self, id: Uuid) -> Result<(), GraphError> {
        let before = self.nodes.len();
        self.nodes.retain(|node| node.id != id);
        if self.nodes.len() == before {
            return Err(GraphError::UnknownNode(id));
        }
        self.edges.retain(|edge| edge.from != id && edge.to != id);
        self.prune_unused_objects();
        self.bump();
        Ok(())
    }

    /// Descarta nós `petunia.object` sem arestas.
    pub fn prune_unused_objects(&mut self) {
        let used: HashSet<Uuid> = self
            .edges
            .iter()
            .flat_map(|edge| [edge.from, edge.to])
            .collect();
        self.nodes
            .retain(|node| node.type_id != TYPE_OBJECT || used.contains(&node.id));
    }

    fn port_spec(
        &self,
        node: Uuid,
        port: u16,
        output: bool,
    ) -> Result<Option<&'static PortSpec>, GraphError> {
        let node_ref = self.node(node).ok_or(GraphError::UnknownNode(node))?;
        let Some(descriptor) = descriptor(&node_ref.type_id) else {
            return Ok(None); // nó fantasma: sem portas conhecidas
        };
        let ports = if output {
            descriptor.outputs
        } else {
            descriptor.inputs
        };
        ports
            .get(usize::from(port))
            .map(Some)
            .ok_or(GraphError::InvalidPort { node, port })
    }

    /// Conecta `from.from_port → to.to_port` validando existência, tipos,
    /// ocupação e ciclos.
    pub fn connect(
        &mut self,
        from: Uuid,
        from_port: u16,
        to: Uuid,
        to_port: u16,
    ) -> Result<(), GraphError> {
        if self.edges.len() >= MAX_EDGES {
            return Err(GraphError::TooLarge("edges"));
        }
        let (Some(out), Some(input)) = (
            self.port_spec(from, from_port, true)?,
            self.port_spec(to, to_port, false)?,
        ) else {
            return Err(GraphError::UnknownType("phantom".into()));
        };
        if out.ty != input.ty {
            return Err(GraphError::TypeMismatch {
                from: out.ty.id(),
                to: input.ty.id(),
            });
        }
        let edge = OpEdge {
            from,
            from_port,
            to,
            to_port,
        };
        if self.edges.contains(&edge) {
            return Err(GraphError::DuplicateEdge);
        }
        if !input.multi
            && self
                .edges
                .iter()
                .any(|e| e.to == to && e.to_port == to_port)
        {
            return Err(GraphError::SocketOccupied);
        }
        if from == to || self.reaches(to, from) {
            return Err(GraphError::Cycle);
        }
        self.edges.push(edge);
        self.bump();
        Ok(())
    }

    pub fn disconnect(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        let before = self.edges.len();
        self.edges
            .retain(|edge| !(edge.from == from && edge.to == to));
        if self.edges.len() == before {
            return Err(GraphError::UnknownNode(to));
        }
        self.prune_unused_objects();
        self.bump();
        Ok(())
    }

    /// Existe caminho de `start` até `goal` seguindo as arestas?
    fn reaches(&self, start: Uuid, goal: Uuid) -> bool {
        let mut seen = HashSet::new();
        let mut stack = vec![start];
        while let Some(current) = stack.pop() {
            if current == goal {
                return true;
            }
            if !seen.insert(current) {
                continue;
            }
            stack.extend(
                self.edges
                    .iter()
                    .filter(|edge| edge.from == current)
                    .map(|edge| edge.to),
            );
        }
        false
    }

    pub fn set_param(
        &mut self,
        node: Uuid,
        key: &str,
        value: ParamValue,
    ) -> Result<(), GraphError> {
        let type_id = self
            .node(node)
            .ok_or(GraphError::UnknownNode(node))?
            .type_id
            .clone();
        let descriptor =
            descriptor(&type_id).ok_or_else(|| GraphError::UnknownType(type_id.clone()))?;
        let spec = descriptor
            .params
            .iter()
            .find(|spec| spec.key == key)
            .ok_or_else(|| GraphError::InvalidParam {
                key: key.to_string(),
                reason: "unknown parameter".into(),
            })?;
        validate_param(spec, &value)?;
        if let Some(node) = self.node_mut(node) {
            if node.params.get(key) == Some(&value) {
                return Ok(());
            }
            node.params.insert(key.to_string(), value);
        }
        self.bump();
        Ok(())
    }

    pub fn set_enabled(&mut self, node: Uuid, enabled: bool) -> Result<(), GraphError> {
        let node = self.node_mut(node).ok_or(GraphError::UnknownNode(node))?;
        if node.enabled != enabled {
            node.enabled = enabled;
            self.bump();
        }
        Ok(())
    }

    pub fn set_position(&mut self, node: Uuid, position: [f32; 2]) -> Result<(), GraphError> {
        if !position.iter().all(|c| c.is_finite()) {
            return Err(GraphError::InvalidParam {
                key: "position".into(),
                reason: "non-finite".into(),
            });
        }
        self.node_mut(node)
            .ok_or(GraphError::UnknownNode(node))?
            .position = position;
        self.bump();
        Ok(())
    }

    // ── Consultas por objeto ────────────────────────────────────────────────

    /// Nó `petunia.object` do asset, se existir.
    pub fn object_node(&self, asset: Uuid) -> Option<Uuid> {
        self.nodes
            .iter()
            .find(|node| {
                node.type_id == TYPE_OBJECT
                    && node.params.get("asset") == Some(&ParamValue::Asset(asset))
            })
            .map(|node| node.id)
    }

    /// Cria (ou reaproveita) o nó de objeto do asset.
    pub fn ensure_object_node(&mut self, asset: Uuid) -> Result<Uuid, GraphError> {
        if let Some(id) = self.object_node(asset) {
            return Ok(id);
        }
        let index = self.nodes.len() as f32;
        self.add_node(
            TYPE_OBJECT,
            &[("asset", ParamValue::Asset(asset))],
            [0.0, index * 120.0],
        )
    }

    /// Operações que agem sobre `asset`, na ordem das arestas.
    pub fn operations_on(&self, asset: Uuid) -> Vec<&OpNode> {
        let Some(object) = self.object_node(asset) else {
            return Vec::new();
        };
        self.edges
            .iter()
            .filter(|edge| edge.to == object && edge.to_port == 0)
            .filter_map(|edge| self.node(edge.from))
            .collect()
    }

    /// Objetos sobre os quais a operação `node` age (suas saídas).
    pub fn targets_of(&self, node: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter(|edge| edge.from == node)
            .filter_map(|edge| self.node(edge.to))
            .filter_map(|target| match target.params.get("asset") {
                Some(ParamValue::Asset(asset)) if target.type_id == TYPE_OBJECT => Some(*asset),
                _ => None,
            })
            .collect()
    }

    /// Objeto usado como ferramenta pela operação `node` (entrada `tool`).
    pub fn tool_of(&self, node: Uuid) -> Option<Uuid> {
        let edge = self
            .edges
            .iter()
            .find(|edge| edge.to == node && edge.to_port == 0)?;
        match self.node(edge.from)?.params.get("asset") {
            Some(ParamValue::Asset(asset)) => Some(*asset),
            _ => None,
        }
    }

    /// Operações que têm `asset` como ferramenta.
    pub fn operations_using_tool(&self, asset: Uuid) -> Vec<&OpNode> {
        self.nodes
            .iter()
            .filter(|node| node.type_id == TYPE_BOOLEAN && self.tool_of(node.id) == Some(asset))
            .collect()
    }

    /// Ids de assets que têm ao menos uma operação ativa.
    pub fn targets(&self) -> Vec<Uuid> {
        let mut targets = Vec::new();
        for node in &self.nodes {
            if node.type_id != TYPE_OBJECT {
                continue;
            }
            if let Some(ParamValue::Asset(asset)) = node.params.get("asset")
                && self
                    .operations_on(*asset)
                    .iter()
                    .any(|operation| operation.enabled)
            {
                targets.push(*asset);
            }
        }
        targets
    }

    /// Remove tudo que se refere a `asset` (objeto apagado do documento).
    pub fn forget_asset(&mut self, asset: Uuid) -> bool {
        let object = self.object_node(asset);
        let doomed: Vec<Uuid> = self
            .nodes
            .iter()
            .filter(|node| {
                node.type_id == TYPE_BOOLEAN && self.tool_of(node.id) == Some(asset)
                    || Some(node.id) == object
            })
            .map(|node| node.id)
            .collect();
        if doomed.is_empty() {
            return false;
        }
        for id in doomed {
            let _ = self.remove_node(id);
        }
        true
    }

    // ── Validação ───────────────────────────────────────────────────────────

    /// Valida limites, referências, tipos e ciclos (o arquivo é entrada hostil).
    pub fn validate(&self) -> Result<(), GraphError> {
        if self.nodes.len() > MAX_NODES {
            return Err(GraphError::TooLarge("nodes"));
        }
        if self.edges.len() > MAX_EDGES {
            return Err(GraphError::TooLarge("edges"));
        }
        let ids: HashSet<Uuid> = self.nodes.iter().map(|node| node.id).collect();
        if ids.len() != self.nodes.len() {
            return Err(GraphError::DuplicateEdge);
        }
        for edge in &self.edges {
            for id in [edge.from, edge.to] {
                if !ids.contains(&id) {
                    return Err(GraphError::UnknownNode(id));
                }
            }
            let out = self.port_spec(edge.from, edge.from_port, true)?;
            let input = self.port_spec(edge.to, edge.to_port, false)?;
            if let (Some(out), Some(input)) = (out, input)
                && out.ty != input.ty
            {
                return Err(GraphError::TypeMismatch {
                    from: out.ty.id(),
                    to: input.ty.id(),
                });
            }
        }
        // Ciclos: Kahn.
        let mut indegree: HashMap<Uuid, usize> = ids.iter().map(|id| (*id, 0)).collect();
        for edge in &self.edges {
            *indegree.entry(edge.to).or_default() += 1;
        }
        let mut ready: Vec<Uuid> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop() {
            visited += 1;
            for edge in self.edges.iter().filter(|edge| edge.from == id) {
                if let Some(degree) = indegree.get_mut(&edge.to) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.push(edge.to);
                    }
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(GraphError::Cycle);
        }
        Ok(())
    }

    // ── Avaliação ───────────────────────────────────────────────────────────

    /// Chave de dependência do resultado do objeto: muda quando qualquer coisa
    /// que influencia a avaliação muda (malha autoral, modifiers, parâmetros dos
    /// nós, ferramentas e transformações). Só hashes baratos, sem malhas.
    pub fn dependency_key(&self, assets: &[Asset], target: Uuid) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mix = |mut value: u64, next: u64| {
            value ^= next;
            value.wrapping_mul(0x100_0000_01b3)
        };
        let Some(asset) = assets.iter().find(|asset| asset.id == target) else {
            return hash;
        };
        hash = mix(hash, geometry_hash(&asset.mesh));
        hash = mix(hash, asset.modifier_hash());
        hash = mix(hash, transform_hash(asset));
        for operation in self.operations_on(target) {
            for byte in operation.id.as_bytes() {
                hash = mix(hash, u64::from(*byte));
            }
            hash = mix(hash, u64::from(operation.enabled));
            for byte in operation.type_id.bytes() {
                hash = mix(hash, u64::from(byte));
            }
            for (key, value) in &operation.params {
                for byte in key.bytes() {
                    hash = mix(hash, u64::from(byte));
                }
                hash = mix(hash, param_hash(value));
            }
            if let Some(tool) = self.tool_of(operation.id)
                && let Some(tool_asset) = assets.iter().find(|asset| asset.id == tool)
            {
                hash = mix(hash, geometry_hash(&tool_asset.mesh));
                hash = mix(hash, transform_hash(tool_asset));
            }
        }
        hash
    }

    /// Avalia o resultado do objeto. A malha autoral não é tocada.
    pub fn evaluate_asset(&self, assets: &[Asset], target: Uuid) -> AssetEvaluation {
        let mut evaluation = AssetEvaluation::default();
        let Some(asset) = assets.iter().find(|asset| asset.id == target) else {
            return evaluation;
        };
        let operations: Vec<&OpNode> = self
            .operations_on(target)
            .into_iter()
            .filter(|operation| operation.enabled)
            .collect();
        if operations.is_empty() {
            return evaluation;
        }
        // A cadeia parte da malha já avaliada pelos modifiers legados do objeto.
        let mut mesh = asset.modifier_evaluated_mesh();
        for operation in operations {
            match self.apply_operation(operation, asset, assets, &mesh) {
                Ok(next) => mesh = next,
                Err(error) => evaluation.errors.push((operation.id, error.to_string())),
            }
        }
        evaluation.mesh = Some(mesh);
        evaluation
    }

    /// Aplica uma operação a `mesh` (no frame local de `owner`).
    fn apply_operation(
        &self,
        operation: &OpNode,
        owner: &Asset,
        assets: &[Asset],
        mesh: &Mesh,
    ) -> Result<Mesh, GraphError> {
        let descriptor = descriptor(&operation.type_id)
            .ok_or_else(|| GraphError::UnknownType(operation.type_id.clone()))?;
        match descriptor.type_id {
            TYPE_MIRROR => {
                let mut result = mesh.clone();
                result.mirror(axis_param(operation), float_param(operation, "weld", 0.001));
                Ok(result)
            }
            TYPE_SYMMETRY => {
                let mut result = mesh.clone();
                result.symmetrize(
                    axis_param(operation),
                    bool_param(operation, "positive_to_negative", true),
                    float_param(operation, "weld", 0.001),
                );
                Ok(result)
            }
            TYPE_BOOLEAN => {
                let tool_id = self.tool_of(operation.id).ok_or(GraphError::InvalidPort {
                    node: operation.id,
                    port: 0,
                })?;
                if tool_id == owner.id {
                    return Err(GraphError::SelfReference);
                }
                let tool = assets
                    .iter()
                    .find(|asset| asset.id == tool_id)
                    .ok_or(GraphError::MissingAsset(tool_id))?;
                let tool_mesh = tool.mesh_in_frame_of(owner);
                if mesh.faces.len().saturating_add(tool_mesh.faces.len()) > MAX_BOOLEAN_FACES {
                    return Err(GraphError::TooLarge("boolean input"));
                }
                let op = match choice_param(operation, "operation", "difference") {
                    "union" => BooleanOp::Union,
                    "intersection" => BooleanOp::Intersection,
                    _ => BooleanOp::Difference,
                };
                let (result, _report) = boolean_meshes_clean(mesh, &tool_mesh, op)
                    .map_err(|error| GraphError::Boolean(error.to_string()))?;
                Ok(result)
            }
            other => Err(GraphError::UnknownType(other.to_string())),
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn validate_param(spec: &ParamSpec, value: &ParamValue) -> Result<(), GraphError> {
    let invalid = |reason: &str| GraphError::InvalidParam {
        key: spec.key.to_string(),
        reason: reason.to_string(),
    };
    match (spec.kind, value) {
        (ParamKind::Bool, ParamValue::Bool(_)) => Ok(()),
        (ParamKind::Int { min, max }, ParamValue::Int(v)) => {
            if (min..=max).contains(v) {
                Ok(())
            } else {
                Err(invalid("out of range"))
            }
        }
        (ParamKind::Float { min, max }, ParamValue::Float(v)) => {
            if v.is_finite() && (min..=max).contains(v) {
                Ok(())
            } else {
                Err(invalid("not finite or out of range"))
            }
        }
        (ParamKind::Choice(options), ParamValue::Choice(v)) => {
            if options.contains(&v.as_str()) {
                Ok(())
            } else {
                Err(invalid("not an allowed choice"))
            }
        }
        (ParamKind::Asset, ParamValue::Asset(id)) => {
            if id.is_nil() {
                Err(invalid("nil object"))
            } else {
                Ok(())
            }
        }
        _ => Err(invalid("wrong value type")),
    }
}

fn axis_param(node: &OpNode) -> usize {
    match choice_param(node, "axis", "x") {
        "y" => 1,
        "z" => 2,
        _ => 0,
    }
}

fn choice_param<'a>(node: &'a OpNode, key: &str, default: &'a str) -> &'a str {
    match node.params.get(key) {
        Some(ParamValue::Choice(value)) => value.as_str(),
        _ => default,
    }
}

fn bool_param(node: &OpNode, key: &str, default: bool) -> bool {
    match node.params.get(key) {
        Some(ParamValue::Bool(value)) => *value,
        _ => default,
    }
}

fn float_param(node: &OpNode, key: &str, default: f32) -> f32 {
    match node.params.get(key) {
        Some(ParamValue::Float(value)) if value.is_finite() => *value as f32,
        _ => default,
    }
}

fn param_hash(value: &ParamValue) -> u64 {
    match value {
        ParamValue::Bool(v) => u64::from(*v) + 1,
        ParamValue::Int(v) => *v as u64,
        ParamValue::Float(v) => v.to_bits(),
        ParamValue::Choice(v) => v
            .bytes()
            .fold(7_u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b))),
        ParamValue::Asset(v) => v.as_u128() as u64,
    }
}

/// Hash só de geometria (posições e topologia): seleção e cor não invalidam.
fn geometry_hash(mesh: &Mesh) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mix = |mut value: u64, next: u64| {
        value ^= next;
        value.wrapping_mul(0x100_0000_01b3)
    };
    hash = mix(hash, mesh.verts.len() as u64);
    hash = mix(hash, mesh.faces.len() as u64);
    for vertex in &mesh.verts {
        for component in vertex.pos {
            hash = mix(hash, u64::from(component.to_bits()));
        }
    }
    for face in &mesh.faces {
        hash = mix(hash, face.verts.len() as u64);
        for index in &face.verts {
            hash = mix(hash, *index as u64);
        }
    }
    hash
}

fn transform_hash(asset: &Asset) -> u64 {
    let t = &asset.transform;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for component in t
        .translation
        .iter()
        .chain(t.rotation.iter())
        .chain(t.scale.iter())
    {
        hash ^= u64::from(component.to_bits());
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_asset(name: &str, size: f32, offset: [f32; 3]) -> Asset {
        let mut asset = Asset::new(name, Mesh::cube(size));
        asset.transform.translation = offset;
        asset
    }

    fn boolean_graph(target: &Asset, tool: &Asset, op: &str) -> (OpGraph, Uuid) {
        let mut graph = OpGraph::default();
        let target_node = graph.ensure_object_node(target.id).unwrap();
        let tool_node = graph.ensure_object_node(tool.id).unwrap();
        let boolean = graph
            .add_node(
                TYPE_BOOLEAN,
                &[("operation", ParamValue::Choice(op.into()))],
                [200.0, 0.0],
            )
            .unwrap();
        graph.connect(tool_node, 0, boolean, 0).unwrap();
        graph.connect(boolean, 0, target_node, 0).unwrap();
        (graph, boolean)
    }

    #[test]
    fn descriptors_are_unique_and_well_formed() {
        let mut seen = HashSet::new();
        for node in BUILTIN_NODES {
            assert!(seen.insert(node.type_id), "type_id repetido");
            assert!(node.type_id.contains('.'), "id namespaced");
            let mut keys = HashSet::new();
            for param in node.params {
                assert!(
                    keys.insert(param.key),
                    "parâmetro repetido em {}",
                    node.type_id
                );
                if let (ParamKind::Choice(options), DefaultValue::Choice(default)) =
                    (param.kind, param.default)
                {
                    assert!(options.contains(&default));
                }
            }
        }
    }

    #[test]
    fn add_node_uses_defaults_and_validates_overrides() {
        let mut graph = OpGraph::default();
        let id = graph.add_node(TYPE_BOOLEAN, &[], [0.0, 0.0]).unwrap();
        assert_eq!(
            graph.node(id).unwrap().params["operation"],
            ParamValue::Choice("difference".into())
        );
        assert!(matches!(
            graph.add_node(
                TYPE_BOOLEAN,
                &[("operation", ParamValue::Choice("xor".into()))],
                [0.0; 2]
            ),
            Err(GraphError::InvalidParam { .. })
        ));
        assert!(matches!(
            graph.add_node(
                TYPE_MIRROR,
                &[("weld", ParamValue::Float(f64::NAN))],
                [0.0; 2]
            ),
            Err(GraphError::InvalidParam { .. })
        ));
        assert!(matches!(
            graph.add_node("acme.unknown", &[], [0.0; 2]),
            Err(GraphError::UnknownType(_))
        ));
    }

    #[test]
    fn connect_enforces_types_occupancy_and_cycles() {
        let mut graph = OpGraph::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let object_a = graph.ensure_object_node(a).unwrap();
        let object_b = graph.ensure_object_node(b).unwrap();
        let boolean = graph.add_node(TYPE_BOOLEAN, &[], [0.0; 2]).unwrap();
        // geometria → operação é incompatível
        assert!(matches!(
            graph.connect(object_a, 0, object_b, 0),
            Err(GraphError::TypeMismatch { .. })
        ));
        graph.connect(object_a, 0, boolean, 0).unwrap();
        // `tool` aceita uma única aresta
        assert_eq!(
            graph.connect(object_b, 0, boolean, 0),
            Err(GraphError::SocketOccupied)
        );
        graph.connect(boolean, 0, object_b, 0).unwrap();
        assert_eq!(
            graph.connect(boolean, 0, object_b, 0),
            Err(GraphError::DuplicateEdge)
        );
        // o mesmo operador pode alimentar vários objetos (fan-out)
        let c = graph.ensure_object_node(Uuid::new_v4()).unwrap();
        graph.connect(boolean, 0, c, 0).unwrap();
        // ciclo: object_b → ... → object_a seria fechar o laço
        let second = graph.add_node(TYPE_BOOLEAN, &[], [0.0; 2]).unwrap();
        graph.connect(object_b, 0, second, 0).unwrap();
        assert_eq!(
            graph.connect(second, 0, object_a, 0),
            Err(GraphError::Cycle)
        );
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn invalid_ports_and_missing_nodes_are_rejected() {
        let mut graph = OpGraph::default();
        let object = graph.ensure_object_node(Uuid::new_v4()).unwrap();
        assert!(matches!(
            graph.connect(object, 7, object, 0),
            Err(GraphError::InvalidPort { .. })
        ));
        assert!(matches!(
            graph.connect(Uuid::new_v4(), 0, object, 0),
            Err(GraphError::UnknownNode(_))
        ));
        assert!(graph.remove_node(Uuid::new_v4()).is_err());
    }

    #[test]
    fn remove_node_cleans_edges_and_orphan_objects() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [0.5, 0.0, 0.0]);
        let (mut graph, boolean) = boolean_graph(&target, &tool, "difference");
        assert_eq!(graph.nodes.len(), 3);
        graph.remove_node(boolean).unwrap();
        assert!(graph.nodes.is_empty(), "objetos órfãos não ficam no grafo");
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn boolean_difference_is_non_destructive_and_changes_the_evaluated_mesh() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [1.0, 0.0, 0.0]);
        let authored = target.mesh.to_owned_value();
        let assets = vec![target.clone(), tool.clone()];
        let (graph, _) = boolean_graph(&target, &tool, "difference");
        let evaluation = graph.evaluate_asset(&assets, target.id);
        assert!(evaluation.errors.is_empty(), "{:?}", evaluation.errors);
        let result = evaluation.mesh.expect("malha avaliada");
        assert!(
            result.faces.len() > authored.faces.len(),
            "o corte adiciona faces"
        );
        // a malha autoral do objeto permanece intacta
        assert_eq!(
            assets[0].mesh.to_owned_value().faces.len(),
            authored.faces.len()
        );
    }

    #[test]
    fn boolean_union_intersection_and_difference_differ() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.5, [1.0, 0.0, 0.0]);
        let assets = vec![target.clone(), tool.clone()];
        let faces = |op: &str| {
            let (graph, _) = boolean_graph(&target, &tool, op);
            graph
                .evaluate_asset(&assets, target.id)
                .mesh
                .unwrap()
                .faces
                .len()
        };
        let (union, difference, intersection) =
            (faces("union"), faces("difference"), faces("intersection"));
        assert!(union != difference || difference != intersection);
        assert!(intersection > 0);
    }

    #[test]
    fn fan_out_applies_the_same_operation_to_several_objects() {
        let wall = cube_asset("parede", 2.0, [0.0; 3]);
        let floor = cube_asset("piso", 2.0, [10.0, 0.0, 0.0]);
        let cutter = cube_asset("cortador", 1.0, [0.5, 0.0, 0.0]);
        let mut graph = OpGraph::default();
        let cutter_node = graph.ensure_object_node(cutter.id).unwrap();
        let boolean = graph.add_node(TYPE_BOOLEAN, &[], [0.0; 2]).unwrap();
        graph.connect(cutter_node, 0, boolean, 0).unwrap();
        for target in [&wall, &floor] {
            let node = graph.ensure_object_node(target.id).unwrap();
            graph.connect(boolean, 0, node, 0).unwrap();
        }
        assert_eq!(graph.targets_of(boolean).len(), 2);
        assert_eq!(graph.tool_of(boolean), Some(cutter.id));
        let assets = vec![wall.clone(), floor.clone(), cutter.clone()];
        let on_wall = graph.evaluate_asset(&assets, wall.id);
        let on_floor = graph.evaluate_asset(&assets, floor.id);
        assert!(on_wall.mesh.is_some());
        // o piso está longe do cortador: o resultado existe, mas não é corte real
        assert!(on_floor.mesh.is_some());
    }

    #[test]
    fn disabled_operations_do_not_act() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [0.5, 0.0, 0.0]);
        let assets = vec![target.clone(), tool.clone()];
        let (mut graph, boolean) = boolean_graph(&target, &tool, "difference");
        graph.set_enabled(boolean, false).unwrap();
        assert!(graph.evaluate_asset(&assets, target.id).mesh.is_none());
        assert!(graph.targets().is_empty());
    }

    #[test]
    fn self_reference_and_missing_tool_report_errors_per_node() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [0.0; 3]);
        let (graph, boolean) = boolean_graph(&target, &tool, "difference");
        // sem o objeto-ferramenta no documento
        let evaluation = graph.evaluate_asset(std::slice::from_ref(&target), target.id);
        assert!(evaluation.errors.iter().any(|(id, _)| *id == boolean));
        // o resultado continua sendo a malha anterior (a operação falha é ignorada)
        assert!(evaluation.mesh.is_some());

        let mut own = OpGraph::default();
        let object = own.ensure_object_node(target.id).unwrap();
        let node = own.add_node(TYPE_BOOLEAN, &[], [0.0; 2]).unwrap();
        own.connect(object, 0, node, 0).unwrap();
        // alimentar o próprio alvo fecha um ciclo no grafo
        assert_eq!(own.connect(node, 0, object, 0), Err(GraphError::Cycle));
    }

    #[test]
    fn mirror_and_symmetry_nodes_reuse_the_mesh_operators() {
        // Fora do plano de espelho: um cubo centrado soldaria todo o espelho em si mesmo.
        let mut mesh = Mesh::cube(2.0);
        mesh.transform_affine(glam::Mat4::from_translation(glam::Vec3::new(0.0, 3.0, 0.0)));
        let asset = Asset::new("alvo", mesh);
        let mut graph = OpGraph::default();
        let object = graph.ensure_object_node(asset.id).unwrap();
        let mirror = graph
            .add_node(
                TYPE_MIRROR,
                &[("axis", ParamValue::Choice("y".into()))],
                [0.0; 2],
            )
            .unwrap();
        graph.connect(mirror, 0, object, 0).unwrap();
        let result = graph
            .evaluate_asset(std::slice::from_ref(&asset), asset.id)
            .mesh
            .unwrap();
        assert!(result.verts.len() > asset.mesh.verts.len());
    }

    #[test]
    fn dependency_key_tracks_inputs_but_ignores_selection() {
        let mut target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [0.5, 0.0, 0.0]);
        let (mut graph, boolean) = boolean_graph(&target, &tool, "difference");
        let assets = vec![target.clone(), tool.clone()];
        let base = graph.dependency_key(&assets, target.id);
        assert_eq!(
            base,
            graph.dependency_key(&assets, target.id),
            "determinístico"
        );

        // seleção de vértices não invalida
        target.mesh.verts[0].selected = true;
        assert_eq!(
            base,
            graph.dependency_key(&[target.clone(), tool.clone()], target.id)
        );

        // mover a ferramenta invalida
        let mut moved = tool.clone();
        moved.transform.translation[0] = 3.0;
        assert_ne!(
            base,
            graph.dependency_key(&[target.clone(), moved], target.id)
        );

        // trocar o parâmetro invalida
        graph
            .set_param(boolean, "operation", ParamValue::Choice("union".into()))
            .unwrap();
        assert_ne!(base, graph.dependency_key(&assets, target.id));
    }

    #[test]
    fn set_param_validates_and_is_idempotent() {
        let mut graph = OpGraph::default();
        let mirror = graph.add_node(TYPE_MIRROR, &[], [0.0; 2]).unwrap();
        let revision = graph.revision;
        graph
            .set_param(mirror, "weld", ParamValue::Float(0.001))
            .unwrap();
        assert_eq!(graph.revision, revision, "mesmo valor não muda a revisão");
        assert!(
            graph
                .set_param(mirror, "weld", ParamValue::Float(-1.0))
                .is_err()
        );
        assert!(
            graph
                .set_param(mirror, "weld", ParamValue::Float(f64::INFINITY))
                .is_err()
        );
        assert!(
            graph
                .set_param(mirror, "axis", ParamValue::Choice("w".into()))
                .is_err()
        );
        assert!(
            graph
                .set_param(mirror, "nope", ParamValue::Bool(true))
                .is_err()
        );
        assert!(
            graph
                .set_param(mirror, "axis", ParamValue::Bool(true))
                .is_err()
        );
    }

    #[test]
    fn hostile_graphs_are_rejected_by_validate() {
        let mut graph = OpGraph::default();
        let a = graph.add_node(TYPE_MIRROR, &[], [0.0; 2]).unwrap();
        graph.edges.push(OpEdge {
            from: a,
            from_port: 0,
            to: Uuid::new_v4(),
            to_port: 0,
        });
        assert!(matches!(graph.validate(), Err(GraphError::UnknownNode(_))));
        graph.edges.clear();
        // ciclo escrito à mão (o arquivo ignora `connect`)
        let object = graph.ensure_object_node(Uuid::new_v4()).unwrap();
        let boolean = graph.add_node(TYPE_BOOLEAN, &[], [0.0; 2]).unwrap();
        graph.edges.push(OpEdge {
            from: object,
            from_port: 0,
            to: boolean,
            to_port: 0,
        });
        graph.edges.push(OpEdge {
            from: boolean,
            from_port: 0,
            to: object,
            to_port: 0,
        });
        assert_eq!(graph.validate(), Err(GraphError::Cycle));
        // excesso de nós
        let mut big = OpGraph::default();
        for _ in 0..=MAX_NODES {
            big.nodes.push(OpNode {
                id: Uuid::new_v4(),
                type_id: TYPE_MIRROR.into(),
                version: 1,
                params: BTreeMap::new(),
                enabled: true,
                position: [0.0; 2],
            });
        }
        assert_eq!(big.validate(), Err(GraphError::TooLarge("nodes")));
    }

    #[test]
    fn unknown_node_types_survive_as_phantoms_and_are_skipped() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let mut graph = OpGraph::default();
        let object = graph.ensure_object_node(target.id).unwrap();
        let phantom = OpNode {
            id: Uuid::new_v4(),
            type_id: "acme.twist".into(),
            version: 3,
            params: BTreeMap::from([("angle".to_string(), ParamValue::Float(30.0))]),
            enabled: true,
            position: [0.0; 2],
        };
        graph.nodes.push(phantom.clone());
        // arestas de arquivo para um nó fantasma não derrubam a validação
        graph.edges.push(OpEdge {
            from: phantom.id,
            from_port: 0,
            to: object,
            to_port: 0,
        });
        assert!(graph.validate().is_ok());
        let evaluation = graph.evaluate_asset(std::slice::from_ref(&target), target.id);
        assert_eq!(
            evaluation.errors.len(),
            1,
            "aviso por nó, sem falhar a cadeia"
        );
        // os dados do nó fantasma são preservados no round-trip
        let json = serde_json::to_string(&graph).unwrap();
        let back: OpGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node(phantom.id), Some(&phantom));
    }

    #[test]
    fn forget_asset_removes_its_nodes_and_operations_it_provides() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [0.5, 0.0, 0.0]);
        let (mut graph, _) = boolean_graph(&target, &tool, "difference");
        assert!(graph.forget_asset(tool.id));
        assert!(graph.nodes.is_empty(), "a booleana do cortador some junto");
        assert!(!graph.forget_asset(tool.id));
    }

    #[test]
    fn serde_round_trip_preserves_the_graph() {
        let target = cube_asset("alvo", 2.0, [0.0; 3]);
        let tool = cube_asset("ferramenta", 1.0, [0.5, 0.0, 0.0]);
        let (graph, _) = boolean_graph(&target, &tool, "union");
        let json = serde_json::to_string(&graph).unwrap();
        let back: OpGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, back);
        // campos ausentes usam padrões seguros
        let minimal: OpGraph =
            serde_json::from_str(r#"{"schema_version":1,"nodes":[],"edges":[],"revision":0}"#)
                .unwrap();
        assert!(minimal.is_empty());
    }
}
