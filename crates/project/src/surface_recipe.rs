//! Surface Recipe graph headless (P3D-113, cap. 42 — "Presets antes de graphs").
//!
//! Modelo de dados **independente do widget visual**: DAG com sockets tipados,
//! rejeição de ciclos, avaliador determinístico com memoização, schema
//! versionado e erros explícitos. Os efeitos (P3D-134) são reutilizados como
//! nodes internos (`NodeSpec::Effect`) — a filosofia canônica: o usuário vê
//! presets; o graph é Advanced-only.
//!
//! Não há UI aqui: este módulo vive em `petunia_project` (dado persistente e
//! serializável) e pode ser avaliado headless em testes, CLI ou MCP.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::Canvas;
use crate::material::Material;
use crate::paint_layers::{PaintEffect, apply_effect};

/// Versão do schema do graph (serializada; cap. 42: "schema versionado").
pub const RECIPE_SCHEMA_VERSION: u32 = 1;

/// Erros explícitos do graph (cap. 42: "explicit errors").
#[derive(Debug, Clone, PartialEq)]
pub enum RecipeError {
    /// Node duplicado no graph.
    DuplicateNode(Uuid),
    /// Aresta referencia node inexistente.
    UnknownNode(Uuid),
    /// Ciclo detectado; `path` mostra o loop encontrado.
    CycleDetected { path: Vec<Uuid> },
    /// Socket de origem/destino fora do range do node.
    InvalidSocket { edge: usize },
    /// Socket de destino já conectado por outra aresta.
    SocketAlreadyWired { node: Uuid, socket: u16 },
    /// Tipo da origem não casa com o esperado no destino.
    TypeMismatch {
        node: Uuid,
        expected: SocketType,
        got: SocketType,
    },
    /// Node de entrada sem fonte conectada.
    MissingInput { node: Uuid, socket: u16 },
}

impl std::fmt::Display for RecipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateNode(id) => write!(f, "node duplicado: {id}"),
            Self::UnknownNode(id) => write!(f, "node inexistente: {id}"),
            Self::CycleDetected { path } => write!(f, "ciclo detectado: {path:?}"),
            Self::InvalidSocket { edge } => write!(f, "socket inválido na aresta {edge}"),
            Self::SocketAlreadyWired { node, socket } => {
                write!(f, "socket {socket} de {node} já conectado")
            }
            Self::TypeMismatch {
                node,
                expected,
                got,
            } => write!(
                f,
                "tipo incompatível em {node}: esperado {}, recebido {}",
                expected.name(),
                got.name()
            ),
            Self::MissingInput { node, socket } => {
                write!(f, "entrada {socket} de {node} sem fonte")
            }
        }
    }
}

impl std::error::Error for RecipeError {}

pub type RecipeResult<T> = Result<T, RecipeError>;

/// Tipos que circulam nos sockets (cap. 42: "typed sockets").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketType {
    Color,
    Value,
    Texture,
}

impl SocketType {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Color => "Color",
            Self::Value => "Value",
            Self::Texture => "Texture",
        }
    }
}

/// Valor avaliado circulando no graph.
#[derive(Clone, Debug, PartialEq)]
pub enum SocketValue {
    Color([f32; 4]),
    Value(f32),
    Texture(Canvas),
}

/// Canais de saída de um recipe (P3D-062: paint de mapas via canais).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipeOutputChannel {
    BaseColor,
    Roughness,
    Metallic,
    Emission,
    Height,
    Mask,
}

impl RecipeOutputChannel {
    pub const fn ordinal(self) -> usize {
        match self {
            Self::BaseColor => 0,
            Self::Roughness => 1,
            Self::Metallic => 2,
            Self::Emission => 3,
            Self::Height => 4,
            Self::Mask => 5,
        }
    }
}

/// Especificação de um node. Nodes iniciais do cap. 42: entradas
/// Texture/Color/Value, efeitos (reuso de `PaintEffect`), Mix, Multiply,
/// geradores Checker/Gradient e outputs de canais.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeSpec {
    /// Entrada de textura: injeta o canvas do asset (avaliador recebe a fonte).
    TextureInput,
    ColorInput {
        color: [f32; 4],
    },
    ValueInput {
        value: f32,
    },
    /// Aplica um efeito P3D-134 sobre a textura de entrada.
    Effect {
        effect: PaintEffect,
    },
    /// Multiplica a textura por uma cor (tint).
    Tint {
        color: [f32; 4],
    },
    /// Mescla duas texturas com fator `t` (0..=1): `a*(1-t) + b*t`.
    Mix {
        factor: f32,
    },
    /// Multiplica duas texturas pixel a pixel.
    Multiply,
    /// Gradiente linear (ou radial) gerado proceduralmente.
    Gradient {
        from: [f32; 2],
        to: [f32; 2],
        color_a: [f32; 4],
        color_b: [f32; 4],
        radial: bool,
    },
    /// Padrão checker procedural.
    Checker {
        cell_size: u32,
        color_a: [f32; 4],
        color_b: [f32; 4],
    },
    /// Saída para um canal do material.
    Output {
        channel: RecipeOutputChannel,
    },
    /// Material PBR com parâmetros escalares e entradas de mapa (O04).
    PbrMaterial {
        base_color: [f32; 4],
        roughness: f32,
        metallic: f32,
        emission_strength: f32,
    },
    /// Atribui o material avaliado a um slot da malha (O04).
    AssignMaterial {
        slot: usize,
    },
}

impl NodeSpec {
    /// Tipos esperados nas entradas (sockets de entrada, em ordem).
    pub fn inputs(&self) -> Vec<SocketType> {
        match self {
            Self::TextureInput
            | Self::ColorInput { .. }
            | Self::ValueInput { .. }
            | Self::Gradient { .. }
            | Self::Checker { .. } => vec![],
            Self::Effect { .. }
            | Self::Tint { .. }
            | Self::PbrMaterial { .. }
            | Self::AssignMaterial { .. } => vec![SocketType::Texture],
            Self::Mix { .. } | Self::Multiply => vec![SocketType::Texture, SocketType::Texture],
            Self::Output { .. } => vec![SocketType::Texture],
        }
    }

    /// Tipos produzidos nas saídas (um socket por tipo listado).
    pub fn outputs(&self) -> Vec<SocketType> {
        match self {
            Self::TextureInput => vec![SocketType::Texture],
            Self::ColorInput { .. } => vec![SocketType::Color],
            Self::ValueInput { .. } => vec![SocketType::Value],
            Self::Output { .. } => vec![],
            Self::Effect { .. }
            | Self::Tint { .. }
            | Self::Mix { .. }
            | Self::Multiply
            | Self::Gradient { .. }
            | Self::Checker { .. }
            | Self::PbrMaterial { .. }
            | Self::AssignMaterial { .. } => vec![SocketType::Texture],
        }
    }
}

/// Node do graph com parâmetros expostos (cap. 42: "parâmetros podem ser
/// explicitamente expostos pelo recipe").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeNode {
    pub id: Uuid,
    pub spec: NodeSpec,
    /// Chaves de parâmetros expostos como preset (vazio = nada exposto).
    pub exposed: Vec<String>,
}

/// Aresta direcionada com sockets indexados.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeEdge {
    pub from: Uuid,
    pub from_socket: u16,
    pub to: Uuid,
    pub to_socket: u16,
}

/// Surface Recipe serializável (DAG; ciclos rejeitados na validação).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceRecipe {
    pub schema_version: u32,
    pub name: String,
    pub nodes: Vec<RecipeNode>,
    pub edges: Vec<RecipeEdge>,
    /// Revisão de parâmetros para invalidação de cache externo (cap. 42).
    /// Não participa da avaliação; quem guarda cache compara este valor.
    pub param_revision: u64,
}

impl SurfaceRecipe {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: RECIPE_SCHEMA_VERSION,
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            param_revision: 0,
        }
    }

    /// Adiciona um node (rejeita id duplicado).
    pub fn add_node(&mut self, node: RecipeNode) -> RecipeResult<()> {
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(RecipeError::DuplicateNode(node.id));
        }
        self.nodes.push(node);
        self.param_revision = self.param_revision.wrapping_add(1);
        Ok(())
    }

    /// Conecta dois sockets (valida existência e tipos).
    pub fn connect(
        &mut self,
        from: Uuid,
        from_socket: u16,
        to: Uuid,
        to_socket: u16,
    ) -> RecipeResult<()> {
        let src = self
            .nodes
            .iter()
            .find(|n| n.id == from)
            .ok_or(RecipeError::UnknownNode(from))?;
        let dst = self
            .nodes
            .iter()
            .find(|n| n.id == to)
            .ok_or(RecipeError::UnknownNode(to))?;
        let edge_idx = self.edges.len();
        let out_ty = *src
            .spec
            .outputs()
            .get(from_socket as usize)
            .ok_or(RecipeError::InvalidSocket { edge: edge_idx })?;
        let in_ty = *dst
            .spec
            .inputs()
            .get(to_socket as usize)
            .ok_or(RecipeError::InvalidSocket { edge: edge_idx })?;
        if out_ty != in_ty {
            return Err(RecipeError::TypeMismatch {
                node: to,
                expected: in_ty,
                got: out_ty,
            });
        }
        if self
            .edges
            .iter()
            .any(|e| e.to == to && e.to_socket == to_socket)
        {
            return Err(RecipeError::SocketAlreadyWired {
                node: to,
                socket: to_socket,
            });
        }
        self.edges.push(RecipeEdge {
            from,
            from_socket,
            to,
            to_socket,
        });
        self.param_revision = self.param_revision.wrapping_add(1);
        Ok(())
    }

    /// Valida o graph inteiro: ids, arestas, tipos, entradas e ciclos.
    pub fn validate(&self) -> RecipeResult<()> {
        let mut seen = std::collections::HashSet::new();
        for node in &self.nodes {
            if !seen.insert(node.id) {
                return Err(RecipeError::DuplicateNode(node.id));
            }
        }
        for (i, edge) in self.edges.iter().enumerate() {
            if !seen.contains(&edge.from) || !seen.contains(&edge.to) {
                return Err(RecipeError::UnknownNode(if !seen.contains(&edge.from) {
                    edge.from
                } else {
                    edge.to
                }));
            }
            // Re-valida sockets e tipos (edges são públicos: não confiar só no `connect`).
            let src = self
                .nodes
                .iter()
                .find(|n| n.id == edge.from)
                .ok_or(RecipeError::UnknownNode(edge.from))?;
            let dst = self
                .nodes
                .iter()
                .find(|n| n.id == edge.to)
                .ok_or(RecipeError::UnknownNode(edge.to))?;
            let out_ty = *src
                .spec
                .outputs()
                .get(edge.from_socket as usize)
                .ok_or(RecipeError::InvalidSocket { edge: i })?;
            let in_ty = *dst
                .spec
                .inputs()
                .get(edge.to_socket as usize)
                .ok_or(RecipeError::InvalidSocket { edge: i })?;
            if out_ty != in_ty {
                return Err(RecipeError::TypeMismatch {
                    node: edge.to,
                    expected: in_ty,
                    got: out_ty,
                });
            }
        }
        // Toposort (Kahn) para detectar ciclos com caminho.
        let mut indegree: HashMap<Uuid, usize> = self.nodes.iter().map(|n| (n.id, 0)).collect();
        for edge in &self.edges {
            *indegree.entry(edge.to).or_insert(0) += 1;
        }
        let mut queue: Vec<Uuid> = self
            .nodes
            .iter()
            .filter(|n| indegree.get(&n.id).copied().unwrap_or(0) == 0)
            .map(|n| n.id)
            .collect();
        let mut visited = 0;
        while let Some(id) = queue.pop() {
            visited += 1;
            for edge in self.edges.iter().filter(|e| e.from == id) {
                if let Some(d) = indegree.get_mut(&edge.to) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push(edge.to);
                    }
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(RecipeError::CycleDetected {
                path: self.find_cycle_path(),
            });
        }
        // Entradas obrigatórias presentes.
        for node in &self.nodes {
            for (socket, _) in node.spec.inputs().iter().enumerate() {
                if !self
                    .edges
                    .iter()
                    .any(|e| e.to == node.id && e.to_socket == socket as u16)
                {
                    return Err(RecipeError::MissingInput {
                        node: node.id,
                        socket: socket as u16,
                    });
                }
            }
        }
        Ok(())
    }

    /// DFS iterativa/limitada para achar um caminho de ciclo (p/ mensagem de erro útil).
    fn find_cycle_path(&self) -> Vec<Uuid> {
        const MAX_CYCLE_DEPTH: usize = 512;
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            Unvisited,
            InStack,
            Done,
        }
        let mut marks: HashMap<Uuid, Mark> = HashMap::new();
        let mut stack: Vec<Uuid> = Vec::new();
        let mut cycle: Option<Vec<Uuid>> = None;

        fn visit(
            recipe: &SurfaceRecipe,
            id: Uuid,
            marks: &mut HashMap<Uuid, Mark>,
            stack: &mut Vec<Uuid>,
            cycle: &mut Option<Vec<Uuid>>,
        ) {
            if stack.len() >= MAX_CYCLE_DEPTH {
                return;
            }
            marks.insert(id, Mark::InStack);
            stack.push(id);
            for edge in recipe.edges.iter().filter(|e| e.from == id) {
                match marks.get(&edge.to).copied().unwrap_or(Mark::Unvisited) {
                    Mark::Unvisited => {
                        visit(recipe, edge.to, marks, stack, cycle);
                        if cycle.is_some() {
                            return;
                        }
                    }
                    Mark::InStack => {
                        let start = stack.iter().position(|&n| n == edge.to).unwrap_or(0);
                        *cycle = Some(stack[start..].to_vec());
                        return;
                    }
                    Mark::Done => {}
                }
            }
            stack.pop();
            marks.insert(id, Mark::Done);
        }

        // Ordem determinística: iteração na ordem dos nodes do graph.
        for node in &self.nodes {
            if !marks.contains_key(&node.id) {
                visit(self, node.id, &mut marks, &mut stack, &mut cycle);
                if cycle.is_some() {
                    break;
                }
            }
        }
        cycle.unwrap_or_default()
    }

    /// Ordem topológica determinística (ordem de inserção dos nodes).
    pub fn topological_order(&self) -> RecipeResult<Vec<Uuid>> {
        self.validate()?;
        let mut indegree: HashMap<Uuid, usize> = self.nodes.iter().map(|n| (n.id, 0)).collect();
        for edge in &self.edges {
            *indegree.entry(edge.to).or_insert(0) += 1;
        }
        let mut ready: Vec<Uuid> = self
            .nodes
            .iter()
            .filter(|n| indegree.get(&n.id).copied().unwrap_or(0) == 0)
            .map(|n| n.id)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = ready.pop() {
            order.push(id);
            for edge in self.edges.iter().filter(|e| e.from == id) {
                if let Some(d) = indegree.get_mut(&edge.to) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        ready.push(edge.to);
                    }
                }
            }
        }
        Ok(order)
    }

    /// Avalia o graph de forma determinística sobre o canvas fonte.
    ///
    /// Retorna os canais de saída (nodes `Output`) com seus valores finais.
    /// Memoização por node (cap. 42: "avaliação cacheável"): cada node é
    /// avaliado uma única vez por chamada.
    pub fn evaluate(
        &self,
        source: &Canvas,
    ) -> RecipeResult<Vec<(RecipeOutputChannel, SocketValue)>> {
        self.validate()?;
        let order = self.topological_order()?;
        let mut memo: HashMap<Uuid, SocketValue> = HashMap::new();

        for id in order {
            let node = self
                .nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or(RecipeError::UnknownNode(id))?;
            let value = evaluate_node(node, self, source, &memo)?;
            memo.insert(id, value);
        }

        let mut outputs: Vec<(RecipeOutputChannel, SocketValue)> = Vec::new();
        for node in &self.nodes {
            if let NodeSpec::Output { channel } = node.spec {
                let value = memo
                    .get(&node.id)
                    .cloned()
                    .ok_or(RecipeError::MissingInput {
                        node: node.id,
                        socket: 0,
                    })?;
                outputs.push((channel, value));
            }
        }
        outputs.sort_by_key(|(ch, _)| ch.ordinal());
        Ok(outputs)
    }

    /// Avaliação com cache por revisão (O04): evita recomputar quando nem o graph nem o canvas mudaram.
    pub fn evaluate_cached(
        &self,
        source: &Canvas,
        cache: &mut SurfaceRecipeCache,
    ) -> RecipeResult<Vec<(RecipeOutputChannel, SocketValue)>> {
        if let (Some(outputs), true) = (
            &cache.cached_outputs,
            cache.is_valid(self.param_revision, source),
        ) {
            return Ok(outputs.clone());
        }
        let outputs = self.evaluate(source)?;
        cache.last_revision = self.param_revision;
        cache.last_source_len = source.pixels.len();
        cache.cached_outputs = Some(outputs.clone());
        Ok(outputs)
    }

    /// Avalia a receita e preenche/atualiza os canais de um `Material` PBR (O04).
    pub fn evaluate_to_material(
        &self,
        source: &Canvas,
        base_material: Option<&Material>,
    ) -> RecipeResult<Material> {
        let evaluated_channels = self.evaluate(source)?;
        let mut mat = base_material
            .cloned()
            .unwrap_or_else(|| Material::new("Recipe Material"));
        for (channel, val) in evaluated_channels {
            match (channel, val) {
                (RecipeOutputChannel::BaseColor, SocketValue::Texture(canvas)) => {
                    mat.albedo_texture = Some(canvas);
                }
                (RecipeOutputChannel::BaseColor, SocketValue::Color(rgba)) => {
                    mat.base_color = rgba;
                }
                (RecipeOutputChannel::Roughness, SocketValue::Texture(canvas)) => {
                    mat.roughness_texture = Some(canvas);
                }
                (RecipeOutputChannel::Roughness, SocketValue::Value(v)) => {
                    mat.roughness = v.clamp(0.0, 1.0);
                }
                (RecipeOutputChannel::Metallic, SocketValue::Texture(canvas)) => {
                    mat.metallic_texture = Some(canvas);
                }
                (RecipeOutputChannel::Metallic, SocketValue::Value(v)) => {
                    mat.metallic = v.clamp(0.0, 1.0);
                }
                (RecipeOutputChannel::Emission, SocketValue::Texture(canvas)) => {
                    mat.emission_texture = Some(canvas);
                }
                (RecipeOutputChannel::Emission, SocketValue::Color(rgb)) => {
                    mat.emission_color = [rgb[0], rgb[1], rgb[2]];
                    mat.emission_strength = rgb[3].max(1.0);
                }
                (RecipeOutputChannel::Height, SocketValue::Texture(canvas)) => {
                    mat.height_texture = Some(canvas);
                }
                (RecipeOutputChannel::Height, SocketValue::Value(v)) => {
                    mat.normal_scale = v;
                }
                _ => {}
            }
        }
        Ok(mat)
    }

    /// Bake da receita de superfície para um Material estático persistente com novo UUID (O04).
    pub fn bake(&self, source: &Canvas, name: &str) -> RecipeResult<Material> {
        let mut mat = self.evaluate_to_material(source, None)?;
        mat.name = name.to_string();
        mat.id = Uuid::new_v4();
        Ok(mat)
    }
}

/// Cache determinístico de avaliação da receita por revisão e tamanho da fonte (O04).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceRecipeCache {
    pub last_revision: u64,
    pub last_source_len: usize,
    pub cached_outputs: Option<Vec<(RecipeOutputChannel, SocketValue)>>,
}

impl SurfaceRecipeCache {
    pub fn is_valid(&self, revision: u64, source: &Canvas) -> bool {
        self.cached_outputs.is_some()
            && self.last_revision == revision
            && self.last_source_len == source.pixels.len()
    }
}

/// Avalia um único node (entradas lidas do memo via arestas de entrada).
fn evaluate_node(
    node: &RecipeNode,
    recipe: &SurfaceRecipe,
    source: &Canvas,
    memo: &HashMap<Uuid, SocketValue>,
) -> RecipeResult<SocketValue> {
    let inputs: Vec<SocketValue> = {
        let mut conn: Vec<&RecipeEdge> = recipe.edges.iter().filter(|e| e.to == node.id).collect();
        conn.sort_by_key(|e| e.to_socket);
        conn.iter()
            .map(|e| {
                memo.get(&e.from).cloned().ok_or(RecipeError::MissingInput {
                    node: node.id,
                    socket: e.to_socket,
                })
            })
            .collect::<RecipeResult<Vec<_>>>()?
    };

    Ok(match &node.spec {
        NodeSpec::TextureInput => SocketValue::Texture(source.clone()),
        NodeSpec::ColorInput { color } => SocketValue::Color(*color),
        NodeSpec::ValueInput { value } => SocketValue::Value(*value),
        NodeSpec::Effect { effect } => {
            let mut tex = expect_texture(&inputs, node.id, 0)?;
            apply_effect(&mut tex, effect);
            SocketValue::Texture(tex)
        }
        NodeSpec::Tint { color } => {
            let mut tex = expect_texture(&inputs, node.id, 0)?;
            for px in tex.pixels.chunks_mut(4) {
                for (i, c) in color.iter().enumerate().take(3) {
                    px[i] = ((px[i] as f32 / 255.0 * c) * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
            }
            SocketValue::Texture(tex)
        }
        NodeSpec::Mix { factor } => {
            let a = expect_texture(&inputs, node.id, 0)?;
            let b = expect_texture(&inputs, node.id, 1)?;
            let t = factor.clamp(0.0, 1.0);
            SocketValue::Texture(mix_canvas(&a, &b, t))
        }
        NodeSpec::Multiply => {
            let a = expect_texture(&inputs, node.id, 0)?;
            let b = expect_texture(&inputs, node.id, 1)?;
            SocketValue::Texture(multiply_canvas(&a, &b))
        }
        NodeSpec::Gradient {
            from,
            to,
            color_a,
            color_b,
            radial,
        } => SocketValue::Texture(gradient_canvas(
            source.w, source.h, *from, *to, *color_a, *color_b, *radial,
        )),
        NodeSpec::Checker {
            cell_size,
            color_a,
            color_b,
        } => SocketValue::Texture(checker_canvas(
            source.w, source.h, *cell_size, *color_a, *color_b,
        )),
        NodeSpec::PbrMaterial {
            base_color,
            roughness: _,
            metallic: _,
            emission_strength: _,
        } => {
            let mut tex = expect_texture(&inputs, node.id, 0)?;
            for px in tex.pixels.chunks_mut(4) {
                for (i, c) in base_color.iter().enumerate().take(3) {
                    px[i] = ((px[i] as f32 / 255.0 * c) * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
            }
            SocketValue::Texture(tex)
        }
        NodeSpec::AssignMaterial { .. } => {
            let tex = expect_texture(&inputs, node.id, 0)?;
            SocketValue::Texture(tex)
        }
        NodeSpec::Output { .. } => inputs.first().cloned().ok_or(RecipeError::MissingInput {
            node: node.id,
            socket: 0,
        })?,
    })
}

fn expect_texture(inputs: &[SocketValue], node: Uuid, socket: u16) -> RecipeResult<Canvas> {
    match inputs.get(socket as usize) {
        Some(SocketValue::Texture(t)) => Ok(t.clone()),
        other => Err(RecipeError::TypeMismatch {
            node,
            expected: SocketType::Texture,
            got: other.map_or(SocketType::Value, |v| match v {
                SocketValue::Color(_) => SocketType::Color,
                SocketValue::Value(_) => SocketType::Value,
                SocketValue::Texture(_) => SocketType::Texture,
            }),
        }),
    }
}

fn mix_canvas(a: &Canvas, b: &Canvas, t: f32) -> Canvas {
    let (w, h) = (a.w.min(b.w), a.h.min(b.h));
    let mut out = a.clone();
    for y in 0..h {
        for x in 0..w {
            if let (Some(pa), Some(pb)) = (a.get(x, y), b.get(x, y)) {
                let lerp = |i: usize| (pa[i] as f32 * (1.0 - t) + pb[i] as f32 * t) as u8;
                out.set(x, y, [lerp(0), lerp(1), lerp(2), pa[3]]);
            }
        }
    }
    out
}

fn multiply_canvas(a: &Canvas, b: &Canvas) -> Canvas {
    let (w, h) = (a.w.min(b.w), a.h.min(b.h));
    let mut out = a.clone();
    for y in 0..h {
        for x in 0..w {
            if let (Some(pa), Some(pb)) = (a.get(x, y), b.get(x, y)) {
                let mul = |i: usize| ((pa[i] as f32 / 255.0 * pb[i] as f32) * 255.0) as u8;
                out.set(x, y, [mul(0), mul(1), mul(2), pa[3]]);
            }
        }
    }
    out
}

fn gradient_canvas(
    w: u32,
    h: u32,
    from: [f32; 2],
    to: [f32; 2],
    color_a: [f32; 4],
    color_b: [f32; 4],
    radial: bool,
) -> Canvas {
    let mut out = Canvas::new(w, h, [0, 0, 0, 255]);
    for y in 0..h {
        for x in 0..w {
            let t = if radial {
                let p = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
                let d = ((p[0] - from[0]).powi(2) + (p[1] - from[1]).powi(2)).sqrt();
                (d / ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2))
                    .sqrt()
                    .max(1e-4))
                .clamp(0.0, 1.0)
            } else {
                let p = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
                let dir = [to[0] - from[0], to[1] - from[1]];
                let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt().max(1e-4);
                (((p[0] - from[0]) * dir[0] + (p[1] - from[1]) * dir[1]) / (len * len))
                    .clamp(0.0, 1.0)
            };
            let lerp = |i: usize| color_a[i] * (1.0 - t) + color_b[i] * t;
            out.set(
                x,
                y,
                [
                    (lerp(0) * 255.0) as u8,
                    (lerp(1) * 255.0) as u8,
                    (lerp(2) * 255.0) as u8,
                    (lerp(3) * 255.0) as u8,
                ],
            );
        }
    }
    out
}

fn checker_canvas(w: u32, h: u32, cell_size: u32, color_a: [f32; 4], color_b: [f32; 4]) -> Canvas {
    let cell = cell_size.max(1);
    let mut out = Canvas::new(w, h, [0, 0, 0, 255]);
    for y in 0..h {
        for x in 0..w {
            let on = ((x / cell) + (y / cell)).is_multiple_of(2);
            let c = if on { color_a } else { color_b };
            out.set(
                x,
                y,
                [
                    (c[0] * 255.0) as u8,
                    (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8,
                    (c[3] * 255.0) as u8,
                ],
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Canvas {
        let mut cv = Canvas::new(4, 4, [255, 0, 0, 255]);
        cv.set(0, 0, [0, 0, 0, 255]);
        cv.set(3, 3, [255, 255, 255, 255]);
        cv
    }

    #[test]
    fn invert_chain_is_deterministic_and_cached() {
        let mut recipe = SurfaceRecipe::new("Invert");
        let input = Uuid::from_u128(1);
        let effect = Uuid::from_u128(2);
        let output = Uuid::from_u128(3);
        recipe
            .add_node(RecipeNode {
                id: input,
                spec: NodeSpec::TextureInput,
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: effect,
                spec: NodeSpec::Effect {
                    effect: PaintEffect::Invert,
                },
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: output,
                spec: NodeSpec::Output {
                    channel: RecipeOutputChannel::BaseColor,
                },
                exposed: vec![],
            })
            .unwrap();
        recipe.connect(input, 0, effect, 0).unwrap();
        recipe.connect(effect, 0, output, 0).unwrap();

        let first = recipe.evaluate(&source()).unwrap();
        let second = recipe.evaluate(&source()).unwrap();
        assert_eq!(first, second, "avaliação determinística");

        let (channel, value) = &first[0];
        assert_eq!(*channel, RecipeOutputChannel::BaseColor);
        let SocketValue::Texture(tex) = value else {
            panic!("saída deve ser textura");
        };
        // (0,0) era preto → vira branco invertido.
        assert_eq!(tex.get(0, 0), Some([255, 255, 255, 255]));
        assert_eq!(tex.get(3, 3), Some([0, 0, 0, 255]));
    }

    #[test]
    fn cycle_is_rejected_with_path() {
        let mut recipe = SurfaceRecipe::new("Loop");
        let a = Uuid::from_u128(11);
        let b = Uuid::from_u128(12);
        recipe
            .add_node(RecipeNode {
                id: a,
                spec: NodeSpec::Effect {
                    effect: PaintEffect::Invert,
                },
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: b,
                spec: NodeSpec::Effect {
                    effect: PaintEffect::Invert,
                },
                exposed: vec![],
            })
            .unwrap();
        recipe.connect(a, 0, b, 0).unwrap();
        recipe.connect(b, 0, a, 0).unwrap();
        match recipe.validate() {
            Err(RecipeError::CycleDetected { path }) => {
                assert_eq!(path.len(), 2, "caminho do ciclo: {path:?}");
            }
            other => panic!("esperava CycleDetected, veio {other:?}"),
        }
    }

    #[test]
    fn type_mismatch_and_unknown_node_are_explicit() {
        let mut recipe = SurfaceRecipe::new("Bad");
        let color = Uuid::from_u128(21);
        let effect = Uuid::from_u128(22);
        recipe
            .add_node(RecipeNode {
                id: color,
                spec: NodeSpec::ColorInput {
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: effect,
                spec: NodeSpec::Effect {
                    effect: PaintEffect::Invert,
                },
                exposed: vec![],
            })
            .unwrap();
        assert_eq!(
            recipe.connect(color, 0, effect, 0),
            Err(RecipeError::TypeMismatch {
                node: effect,
                expected: SocketType::Texture,
                got: SocketType::Color,
            })
        );
        assert_eq!(
            recipe.connect(Uuid::from_u128(999), 0, effect, 0),
            Err(RecipeError::UnknownNode(Uuid::from_u128(999)))
        );
    }

    #[test]
    fn missing_input_is_reported() {
        let mut recipe = SurfaceRecipe::new("Dangling");
        let effect = Uuid::from_u128(31);
        recipe
            .add_node(RecipeNode {
                id: effect,
                spec: NodeSpec::Effect {
                    effect: PaintEffect::Invert,
                },
                exposed: vec![],
            })
            .unwrap();
        assert_eq!(
            recipe.validate(),
            Err(RecipeError::MissingInput {
                node: effect,
                socket: 0
            })
        );
    }

    #[test]
    fn mix_and_multiply_compose() {
        let mut recipe = SurfaceRecipe::new("Mix");
        let input = Uuid::from_u128(41);
        let checker = Uuid::from_u128(42);
        let mix = Uuid::from_u128(43);
        let output = Uuid::from_u128(44);
        for (id, spec) in [
            (input, NodeSpec::TextureInput),
            (
                checker,
                NodeSpec::Checker {
                    cell_size: 2,
                    color_a: [0.0, 0.0, 0.0, 1.0],
                    color_b: [1.0, 1.0, 1.0, 1.0],
                },
            ),
            (mix, NodeSpec::Mix { factor: 0.5 }),
            (
                output,
                NodeSpec::Output {
                    channel: RecipeOutputChannel::BaseColor,
                },
            ),
        ] {
            recipe
                .add_node(RecipeNode {
                    id,
                    spec,
                    exposed: vec![],
                })
                .unwrap();
        }
        recipe.connect(input, 0, mix, 0).unwrap();
        recipe.connect(checker, 0, mix, 1).unwrap();
        recipe.connect(mix, 0, output, 0).unwrap();
        let outs = recipe.evaluate(&source()).unwrap();
        let SocketValue::Texture(tex) = &outs[0].1 else {
            panic!("saída deve ser textura");
        };
        // Pixel fonte vermelho puro [255,0,0] em (1,1) mesclado 50% com a célula
        // preta do checker → vermelho ~50%.
        let p = tex.get(1, 1).unwrap();
        assert!(p[0] >= 126 && p[0] <= 129, "meio vermelho esperado: {p:?}");
    }

    #[test]
    fn recipe_serializes_roundtrip_with_schema_version() {
        let mut recipe = SurfaceRecipe::new("Rusty Metal");
        let input = Uuid::from_u128(51);
        let effect = Uuid::from_u128(52);
        recipe
            .add_node(RecipeNode {
                id: input,
                spec: NodeSpec::TextureInput,
                exposed: vec!["texture".into()],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: effect,
                spec: NodeSpec::Effect {
                    effect: PaintEffect::Grain {
                        intensity: 0.3,
                        seed: 7,
                    },
                },
                exposed: vec!["intensity".into(), "seed".into()],
            })
            .unwrap();
        recipe.connect(input, 0, effect, 0).unwrap();

        let json = serde_json::to_string(&recipe).unwrap();
        let back: SurfaceRecipe = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, RECIPE_SCHEMA_VERSION);
        assert_eq!(back, recipe);
    }

    #[test]
    fn pbr_material_and_assign_node_inputs_outputs() {
        let pbr = NodeSpec::PbrMaterial {
            base_color: [0.8, 0.2, 0.2, 1.0],
            roughness: 0.3,
            metallic: 0.9,
            emission_strength: 0.0,
        };
        assert_eq!(pbr.inputs(), vec![SocketType::Texture]);
        assert_eq!(pbr.outputs(), vec![SocketType::Texture]);

        let assign = NodeSpec::AssignMaterial { slot: 1 };
        assert_eq!(assign.inputs(), vec![SocketType::Texture]);
        assert_eq!(assign.outputs(), vec![SocketType::Texture]);
    }

    #[test]
    fn evaluate_to_material_and_bake_populates_material() {
        let mut recipe = SurfaceRecipe::new("Procedural Gold");
        let input = Uuid::from_u128(61);
        let pbr = Uuid::from_u128(62);
        let out_albedo = Uuid::from_u128(63);
        let out_roughness = Uuid::from_u128(64);

        recipe
            .add_node(RecipeNode {
                id: input,
                spec: NodeSpec::TextureInput,
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: pbr,
                spec: NodeSpec::PbrMaterial {
                    base_color: [1.0, 0.8, 0.2, 1.0],
                    roughness: 0.25,
                    metallic: 1.0,
                    emission_strength: 0.0,
                },
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: out_albedo,
                spec: NodeSpec::Output {
                    channel: RecipeOutputChannel::BaseColor,
                },
                exposed: vec![],
            })
            .unwrap();
        let checker_roughness = Uuid::from_u128(65);
        recipe
            .add_node(RecipeNode {
                id: checker_roughness,
                spec: NodeSpec::Checker {
                    cell_size: 2,
                    color_a: [0.2, 0.2, 0.2, 1.0],
                    color_b: [0.8, 0.8, 0.8, 1.0],
                },
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: out_roughness,
                spec: NodeSpec::Output {
                    channel: RecipeOutputChannel::Roughness,
                },
                exposed: vec![],
            })
            .unwrap();

        recipe.connect(input, 0, pbr, 0).unwrap();
        recipe.connect(pbr, 0, out_albedo, 0).unwrap();
        recipe
            .connect(checker_roughness, 0, out_roughness, 0)
            .unwrap();

        let canvas = source();
        let mat = recipe.evaluate_to_material(&canvas, None).unwrap();
        assert!(mat.albedo_texture.is_some());
        assert!(mat.roughness_texture.is_some());

        // Bake cria material estático com novo UUID e nome solicitado
        let baked = recipe.bake(&canvas, "Static Gold").unwrap();
        assert_eq!(baked.name, "Static Gold");
        assert_ne!(baked.id, mat.id);
        assert!(baked.albedo_texture.is_some());
    }

    #[test]
    fn cached_evaluation_avoids_recomputation_when_revision_matches() {
        let mut recipe = SurfaceRecipe::new("Cached Recipe");
        let input = Uuid::from_u128(71);
        let out = Uuid::from_u128(72);
        recipe
            .add_node(RecipeNode {
                id: input,
                spec: NodeSpec::TextureInput,
                exposed: vec![],
            })
            .unwrap();
        recipe
            .add_node(RecipeNode {
                id: out,
                spec: NodeSpec::Output {
                    channel: RecipeOutputChannel::BaseColor,
                },
                exposed: vec![],
            })
            .unwrap();
        recipe.connect(input, 0, out, 0).unwrap();

        let canvas = source();
        let mut cache = SurfaceRecipeCache::default();
        assert!(!cache.is_valid(recipe.param_revision, &canvas));

        let first = recipe.evaluate_cached(&canvas, &mut cache).unwrap();
        assert!(cache.is_valid(recipe.param_revision, &canvas));

        let second = recipe.evaluate_cached(&canvas, &mut cache).unwrap();
        assert_eq!(first, second);

        // Incrementar param_revision invalida o cache
        recipe.param_revision += 1;
        assert!(!cache.is_valid(recipe.param_revision, &canvas));
    }
}
