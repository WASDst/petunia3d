//! Receita de superfície ligada a um material (O05): o grafo de nós de
//! textura/cor do `SurfaceRecipe` (O04) passa a morar no documento, é avaliado
//! na CPU e escreve os mapas do `Material` (base, rugosidade, metálico,
//! emissão, altura). O catálogo de nós é **descritivo** — rótulo, categoria e
//! parâmetros tipados — para a GUI gerar os controles sem código por nó.
//!
//! Avaliação tolerante a edição: só os nós que alimentam uma Saída conectada são
//! avaliados; nós soltos ou Saídas sem entrada não quebram o material.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Canvas;
use crate::material::Material;
use crate::paint_layers::PaintEffect;
use crate::surface_recipe::{
    NodeSpec, RecipeEdge, RecipeError, RecipeNode, RecipeOutputChannel, RecipeResult, SocketType,
    SocketValue, SurfaceRecipe,
};

/// Limite de nós por receita (arquivo hostil e custo de avaliação).
pub const MAX_RECIPE_NODES: usize = 128;
/// Lado da imagem de entrada quando o material não tem textura base.
pub const DEFAULT_SOURCE_SIZE: u32 = 512;

/// Tipo de parâmetro exibido pela GUI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecipeParamKind {
    Float { min: f64, max: f64 },
    Int { min: i64, max: i64 },
    Bool,
    Color,
    Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecipeParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: RecipeParamKind,
}

/// Tipo de nó do catálogo (o mesmo `NodeSpec` pode aparecer como nós de
/// catálogo distintos, ex.: Ruído e Inverter são `Effect`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecipeNodeKind {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub params: &'static [RecipeParamSpec],
}

/// Valor de parâmetro trocado com a GUI.
#[derive(Clone, Debug, PartialEq)]
pub enum RecipeParamValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Color([f32; 4]),
    Choice(String),
}

const CHANNELS: &[&str] = &["base_color", "roughness", "metallic", "emission", "height"];

const fn float(key: &'static str, label: &'static str, min: f64, max: f64) -> RecipeParamSpec {
    RecipeParamSpec {
        key,
        label,
        kind: RecipeParamKind::Float { min, max },
    }
}

const fn color(key: &'static str, label: &'static str) -> RecipeParamSpec {
    RecipeParamSpec {
        key,
        label,
        kind: RecipeParamKind::Color,
    }
}

/// Catálogo de nós da receita, na ordem do menu "Adicionar nó".
pub const NODE_CATALOG: &[RecipeNodeKind] = &[
    RecipeNodeKind {
        id: "image",
        label: "Image",
        category: "input",
        params: &[],
    },
    RecipeNodeKind {
        id: "checker",
        label: "Checker",
        category: "pattern",
        params: &[
            RecipeParamSpec {
                key: "cell_size",
                label: "Cell size",
                kind: RecipeParamKind::Int { min: 1, max: 512 },
            },
            color("color_a", "Color A"),
            color("color_b", "Color B"),
        ],
    },
    RecipeNodeKind {
        id: "gradient",
        label: "Gradient",
        category: "pattern",
        params: &[
            color("color_a", "Color A"),
            color("color_b", "Color B"),
            float("from_x", "Start X", 0.0, 1.0),
            float("from_y", "Start Y", 0.0, 1.0),
            float("to_x", "End X", 0.0, 1.0),
            float("to_y", "End Y", 0.0, 1.0),
            RecipeParamSpec {
                key: "radial",
                label: "Radial",
                kind: RecipeParamKind::Bool,
            },
        ],
    },
    RecipeNodeKind {
        id: "noise",
        label: "Noise",
        category: "pattern",
        params: &[
            float("intensity", "Intensity", 0.0, 1.0),
            RecipeParamSpec {
                key: "seed",
                label: "Seed",
                kind: RecipeParamKind::Int {
                    min: 0,
                    max: 100_000,
                },
            },
        ],
    },
    RecipeNodeKind {
        id: "tint",
        label: "Tint",
        category: "color",
        params: &[color("color", "Color")],
    },
    RecipeNodeKind {
        id: "mix",
        label: "Mix",
        category: "color",
        params: &[float("factor", "Factor", 0.0, 1.0)],
    },
    RecipeNodeKind {
        id: "multiply",
        label: "Multiply",
        category: "color",
        params: &[],
    },
    RecipeNodeKind {
        id: "invert",
        label: "Invert",
        category: "color",
        params: &[],
    },
    RecipeNodeKind {
        id: "posterize",
        label: "Posterize",
        category: "color",
        params: &[RecipeParamSpec {
            key: "levels",
            label: "Levels",
            kind: RecipeParamKind::Int { min: 2, max: 32 },
        }],
    },
    RecipeNodeKind {
        id: "output",
        label: "Material output",
        category: "output",
        params: &[RecipeParamSpec {
            key: "channel",
            label: "Channel",
            kind: RecipeParamKind::Choice(CHANNELS),
        }],
    },
];

/// Receita de superfície de um material.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialRecipe {
    pub graph: SurfaceRecipe,
    /// Imagem do nó "Imagem", capturada ao criar a receita (textura base do
    /// material ou uma tela branca): a avaliação nunca lê o que ela própria escreveu.
    pub source: Canvas,
    /// Canais escritos pela última avaliação (limpos antes da próxima).
    #[serde(default)]
    pub generated: Vec<RecipeOutputChannel>,
    /// Erro da última avaliação, para a GUI (não persistido).
    #[serde(skip)]
    pub last_error: Option<String>,
}

impl MaterialRecipe {
    /// Receita inicial: Imagem → Saída (cor base), já avaliável.
    pub fn new_for(material: &Material) -> Self {
        let source = material
            .albedo_texture
            .clone()
            .unwrap_or_else(|| Canvas::new(DEFAULT_SOURCE_SIZE, DEFAULT_SOURCE_SIZE, [255; 4]));
        let mut graph = SurfaceRecipe::new(format!("{} recipe", material.name));
        let image = Uuid::new_v4();
        let output = Uuid::new_v4();
        // Construção a partir do catálogo: não pode falhar.
        let _ = graph.add_node(node(image, NodeSpec::TextureInput));
        let _ = graph.add_node(node(
            output,
            NodeSpec::Output {
                channel: RecipeOutputChannel::BaseColor,
            },
        ));
        let _ = graph.connect(image, 0, output, 0);
        Self {
            graph,
            source,
            generated: Vec::new(),
            last_error: None,
        }
    }

    /// Adiciona um nó do catálogo com parâmetros padrão.
    pub fn add_node(&mut self, kind: &str) -> RecipeResult<Uuid> {
        if self.graph.nodes.len() >= MAX_RECIPE_NODES {
            return Err(RecipeError::InvalidSocket {
                edge: self.graph.edges.len(),
            });
        }
        let spec = default_spec(kind).ok_or(RecipeError::UnknownNode(Uuid::nil()))?;
        let id = Uuid::new_v4();
        self.graph.add_node(node(id, spec))?;
        Ok(id)
    }

    /// Liga `from.out → to.in`. Uma entrada já ocupada é substituída (como
    /// arrastar um fio novo sobre o soquete no editor). Ciclos são recusados.
    pub fn connect(
        &mut self,
        from: Uuid,
        from_socket: u16,
        to: Uuid,
        to_socket: u16,
    ) -> RecipeResult<()> {
        let previous: Vec<RecipeEdge> = self.graph.edges.clone();
        self.graph
            .edges
            .retain(|e| !(e.to == to && e.to_socket == to_socket));
        // Só ciclos são recusados: entradas ainda soltas são normais na edição.
        let result = if from == to || reaches(&self.graph, to, from) {
            Err(RecipeError::CycleDetected {
                path: vec![from, to],
            })
        } else {
            self.graph.connect(from, from_socket, to, to_socket)
        };
        if result.is_err() {
            self.graph.edges = previous;
        }
        result
    }

    pub fn disconnect(&mut self, to: Uuid, to_socket: u16) -> bool {
        let before = self.graph.edges.len();
        self.graph
            .edges
            .retain(|e| !(e.to == to && e.to_socket == to_socket));
        let changed = self.graph.edges.len() != before;
        if changed {
            self.bump();
        }
        changed
    }

    pub fn remove_node(&mut self, id: Uuid) -> bool {
        let before = self.graph.nodes.len();
        self.graph.nodes.retain(|n| n.id != id);
        if self.graph.nodes.len() == before {
            return false;
        }
        self.graph.edges.retain(|e| e.from != id && e.to != id);
        self.bump();
        true
    }

    /// Define um parâmetro validado pelo catálogo.
    pub fn set_param(&mut self, id: Uuid, key: &str, value: &RecipeParamValue) -> RecipeResult<()> {
        let node = self
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or(RecipeError::UnknownNode(id))?;
        let kind = kind_of(&node.spec).ok_or(RecipeError::UnknownNode(id))?;
        let spec = kind
            .params
            .iter()
            .find(|p| p.key == key)
            .ok_or(RecipeError::InvalidSocket { edge: 0 })?;
        let checked = check_param(spec, value).ok_or(RecipeError::InvalidSocket { edge: 0 })?;
        apply_param(&mut node.spec, key, &checked);
        self.bump();
        Ok(())
    }

    /// Avalia e escreve os mapas no material. Mantém os mapas anteriores se a
    /// avaliação falhar (o erro fica em `last_error`).
    pub fn evaluate_into(&mut self, material: &mut Material) {
        match self.evaluate_outputs() {
            Ok(outputs) => {
                for channel in std::mem::take(&mut self.generated) {
                    clear_channel(material, channel);
                }
                for (channel, value) in outputs {
                    if write_channel(material, channel, value) {
                        self.generated.push(channel);
                    }
                }
                self.last_error = None;
            }
            Err(error) => self.last_error = Some(error.to_string()),
        }
    }

    /// Só avalia o que alimenta Saídas conectadas.
    fn evaluate_outputs(&self) -> RecipeResult<Vec<(RecipeOutputChannel, SocketValue)>> {
        let mut needed: Vec<Uuid> = self
            .graph
            .nodes
            .iter()
            .filter(|n| matches!(n.spec, NodeSpec::Output { .. }))
            .filter(|n| self.graph.edges.iter().any(|e| e.to == n.id))
            .map(|n| n.id)
            .collect();
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = needed.pop() {
            if seen.len() > MAX_RECIPE_NODES || !seen.insert(id) {
                continue;
            }
            needed.extend(
                self.graph
                    .edges
                    .iter()
                    .filter(|e| e.to == id)
                    .map(|e| e.from),
            );
        }
        let mut pruned = self.graph.clone();
        pruned.nodes.retain(|n| seen.contains(&n.id));
        pruned
            .edges
            .retain(|e| seen.contains(&e.from) && seen.contains(&e.to));
        if pruned.nodes.is_empty() {
            return Ok(Vec::new());
        }
        pruned.evaluate(&self.source)
    }

    /// Repara uma receita vinda de arquivo: limite de nós, ids únicos e arestas
    /// reconstruídas com as regras de `connect` (tipos, soquetes, ciclos).
    pub fn repair(&mut self) -> usize {
        let mut fixes = 0;
        if self.graph.nodes.len() > MAX_RECIPE_NODES {
            fixes += self.graph.nodes.len() - MAX_RECIPE_NODES;
            self.graph.nodes.truncate(MAX_RECIPE_NODES);
        }
        let mut ids = std::collections::HashSet::new();
        let before = self.graph.nodes.len();
        self.graph.nodes.retain(|n| ids.insert(n.id));
        fixes += before - self.graph.nodes.len();
        for node in &mut self.graph.nodes {
            fixes += sanitize_spec(&mut node.spec);
        }
        let edges = std::mem::take(&mut self.graph.edges);
        for edge in edges {
            let acyclic = edge.from != edge.to && !reaches(&self.graph, edge.to, edge.from);
            let ok = acyclic
                && self
                    .graph
                    .connect(edge.from, edge.from_socket, edge.to, edge.to_socket)
                    .is_ok();
            if !ok {
                fixes += 1;
            }
        }
        self.source.validate();
        fixes
    }

    fn bump(&mut self) {
        self.graph.param_revision = self.graph.param_revision.wrapping_add(1);
    }
}

/// Há caminho de `start` até `goal` seguindo as arestas? (busca limitada)
fn reaches(graph: &SurfaceRecipe, start: Uuid, goal: Uuid) -> bool {
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![start];
    while let Some(current) = stack.pop() {
        if current == goal {
            return true;
        }
        if seen.len() > MAX_RECIPE_NODES * 4 || !seen.insert(current) {
            continue;
        }
        stack.extend(
            graph
                .edges
                .iter()
                .filter(|e| e.from == current)
                .map(|e| e.to),
        );
    }
    false
}

fn node(id: Uuid, spec: NodeSpec) -> RecipeNode {
    RecipeNode {
        id,
        spec,
        exposed: Vec::new(),
    }
}

/// `NodeSpec` padrão de um tipo do catálogo.
pub fn default_spec(kind: &str) -> Option<NodeSpec> {
    let white = [1.0, 1.0, 1.0, 1.0];
    let gray = [0.2, 0.2, 0.2, 1.0];
    Some(match kind {
        "image" => NodeSpec::TextureInput,
        "checker" => NodeSpec::Checker {
            cell_size: 32,
            color_a: white,
            color_b: gray,
        },
        "gradient" => NodeSpec::Gradient {
            from: [0.0, 0.0],
            to: [1.0, 1.0],
            color_a: white,
            color_b: gray,
            radial: false,
        },
        "noise" => NodeSpec::Effect {
            effect: PaintEffect::Grain {
                intensity: 0.25,
                seed: 1,
            },
        },
        "tint" => NodeSpec::Tint {
            color: [1.0, 0.6, 0.4, 1.0],
        },
        "mix" => NodeSpec::Mix { factor: 0.5 },
        "multiply" => NodeSpec::Multiply,
        "invert" => NodeSpec::Effect {
            effect: PaintEffect::Invert,
        },
        "posterize" => NodeSpec::Effect {
            effect: PaintEffect::Posterize { levels: 4 },
        },
        "output" => NodeSpec::Output {
            channel: RecipeOutputChannel::BaseColor,
        },
        _ => return None,
    })
}

/// Tipo de catálogo de um nó (nós fora do catálogo, como os de material O04,
/// não têm controles gerados).
pub fn kind_of(spec: &NodeSpec) -> Option<&'static RecipeNodeKind> {
    let id = match spec {
        NodeSpec::TextureInput => "image",
        NodeSpec::Checker { .. } => "checker",
        NodeSpec::Gradient { .. } => "gradient",
        NodeSpec::Effect {
            effect: PaintEffect::Grain { .. },
        } => "noise",
        NodeSpec::Effect {
            effect: PaintEffect::Invert,
        } => "invert",
        NodeSpec::Effect {
            effect: PaintEffect::Posterize { .. },
        } => "posterize",
        NodeSpec::Tint { .. } => "tint",
        NodeSpec::Mix { .. } => "mix",
        NodeSpec::Multiply => "multiply",
        NodeSpec::Output { .. } => "output",
        _ => return None,
    };
    NODE_CATALOG.iter().find(|kind| kind.id == id)
}

/// Valores atuais dos parâmetros de catálogo de um nó.
pub fn params_of(spec: &NodeSpec) -> Vec<(&'static str, RecipeParamValue)> {
    use RecipeParamValue as V;
    match spec {
        NodeSpec::Checker {
            cell_size,
            color_a,
            color_b,
        } => vec![
            ("cell_size", V::Int(i64::from(*cell_size))),
            ("color_a", V::Color(*color_a)),
            ("color_b", V::Color(*color_b)),
        ],
        NodeSpec::Gradient {
            from,
            to,
            color_a,
            color_b,
            radial,
        } => vec![
            ("color_a", V::Color(*color_a)),
            ("color_b", V::Color(*color_b)),
            ("from_x", V::Float(f64::from(from[0]))),
            ("from_y", V::Float(f64::from(from[1]))),
            ("to_x", V::Float(f64::from(to[0]))),
            ("to_y", V::Float(f64::from(to[1]))),
            ("radial", V::Bool(*radial)),
        ],
        NodeSpec::Effect {
            effect: PaintEffect::Grain { intensity, seed },
        } => vec![
            ("intensity", V::Float(f64::from(*intensity))),
            ("seed", V::Int(i64::from(*seed))),
        ],
        NodeSpec::Effect {
            effect: PaintEffect::Posterize { levels },
        } => vec![("levels", V::Int(i64::from(*levels)))],
        NodeSpec::Tint { color } => vec![("color", V::Color(*color))],
        NodeSpec::Mix { factor } => vec![("factor", V::Float(f64::from(*factor)))],
        NodeSpec::Output { channel } => vec![("channel", V::Choice(channel_id(*channel).into()))],
        _ => Vec::new(),
    }
}

pub fn channel_id(channel: RecipeOutputChannel) -> &'static str {
    match channel {
        RecipeOutputChannel::BaseColor => "base_color",
        RecipeOutputChannel::Roughness => "roughness",
        RecipeOutputChannel::Metallic => "metallic",
        RecipeOutputChannel::Emission => "emission",
        RecipeOutputChannel::Height => "height",
        RecipeOutputChannel::Mask => "mask",
    }
}

fn channel_from_id(id: &str) -> Option<RecipeOutputChannel> {
    Some(match id {
        "base_color" => RecipeOutputChannel::BaseColor,
        "roughness" => RecipeOutputChannel::Roughness,
        "metallic" => RecipeOutputChannel::Metallic,
        "emission" => RecipeOutputChannel::Emission,
        "height" => RecipeOutputChannel::Height,
        _ => return None,
    })
}

/// Normaliza o valor contra a especificação; `None` se o tipo não confere.
fn check_param(spec: &RecipeParamSpec, value: &RecipeParamValue) -> Option<RecipeParamValue> {
    use RecipeParamValue as V;
    match (spec.kind, value) {
        (RecipeParamKind::Float { min, max }, V::Float(v)) if v.is_finite() => {
            Some(V::Float(v.clamp(min, max)))
        }
        (RecipeParamKind::Float { min, max }, V::Int(v)) => {
            Some(V::Float((*v as f64).clamp(min, max)))
        }
        (RecipeParamKind::Int { min, max }, V::Int(v)) => Some(V::Int((*v).clamp(min, max))),
        (RecipeParamKind::Int { min, max }, V::Float(v)) if v.is_finite() => {
            Some(V::Int((v.round() as i64).clamp(min, max)))
        }
        (RecipeParamKind::Bool, V::Bool(v)) => Some(V::Bool(*v)),
        (RecipeParamKind::Color, V::Color(c)) if c.iter().all(|v| v.is_finite()) => {
            Some(V::Color(c.map(|v| v.clamp(0.0, 1.0))))
        }
        (RecipeParamKind::Choice(options), V::Choice(c)) if options.contains(&c.as_str()) => {
            Some(V::Choice(c.clone()))
        }
        _ => None,
    }
}

fn apply_param(spec: &mut NodeSpec, key: &str, value: &RecipeParamValue) {
    use RecipeParamValue as V;
    let as_f32 = |v: &RecipeParamValue| match v {
        V::Float(f) => *f as f32,
        V::Int(i) => *i as f32,
        _ => 0.0,
    };
    let as_u32 = |v: &RecipeParamValue| match v {
        V::Int(i) => u32::try_from(*i).unwrap_or(0),
        _ => 0,
    };
    match (spec, key, value) {
        (NodeSpec::Checker { cell_size, .. }, "cell_size", v) => *cell_size = as_u32(v).max(1),
        (NodeSpec::Checker { color_a, .. }, "color_a", V::Color(c))
        | (NodeSpec::Gradient { color_a, .. }, "color_a", V::Color(c)) => *color_a = *c,
        (NodeSpec::Checker { color_b, .. }, "color_b", V::Color(c))
        | (NodeSpec::Gradient { color_b, .. }, "color_b", V::Color(c)) => *color_b = *c,
        (NodeSpec::Gradient { from, .. }, "from_x", v) => from[0] = as_f32(v),
        (NodeSpec::Gradient { from, .. }, "from_y", v) => from[1] = as_f32(v),
        (NodeSpec::Gradient { to, .. }, "to_x", v) => to[0] = as_f32(v),
        (NodeSpec::Gradient { to, .. }, "to_y", v) => to[1] = as_f32(v),
        (NodeSpec::Gradient { radial, .. }, "radial", V::Bool(b)) => *radial = *b,
        (
            NodeSpec::Effect {
                effect: PaintEffect::Grain { intensity, .. },
            },
            "intensity",
            v,
        ) => *intensity = as_f32(v),
        (
            NodeSpec::Effect {
                effect: PaintEffect::Grain { seed, .. },
            },
            "seed",
            v,
        ) => *seed = as_u32(v),
        (
            NodeSpec::Effect {
                effect: PaintEffect::Posterize { levels },
            },
            "levels",
            v,
        ) => *levels = u8::try_from(as_u32(v)).unwrap_or(32),
        (NodeSpec::Tint { color }, "color", V::Color(c)) => *color = *c,
        (NodeSpec::Mix { factor }, "factor", v) => *factor = as_f32(v),
        (NodeSpec::Output { channel }, "channel", V::Choice(c)) => {
            if let Some(next) = channel_from_id(c) {
                *channel = next;
            }
        }
        _ => {}
    }
}

/// Traz cada parâmetro de catálogo de volta à faixa válida. Retorna correções.
fn sanitize_spec(spec: &mut NodeSpec) -> usize {
    let Some(kind) = kind_of(spec) else {
        return 0;
    };
    let mut fixes = 0;
    for (key, value) in params_of(spec) {
        let Some(param) = kind.params.iter().find(|p| p.key == key) else {
            continue;
        };
        match check_param(param, &value) {
            Some(clean) if clean == value => {}
            Some(clean) => {
                apply_param(spec, key, &clean);
                fixes += 1;
            }
            None => {
                if let Some(default) = default_spec(kind.id) {
                    let default_value = params_of(&default)
                        .into_iter()
                        .find(|(k, _)| *k == key)
                        .map(|(_, v)| v);
                    if let Some(default_value) = default_value {
                        apply_param(spec, key, &default_value);
                    }
                }
                fixes += 1;
            }
        }
    }
    fixes
}

fn clear_channel(material: &mut Material, channel: RecipeOutputChannel) {
    match channel {
        RecipeOutputChannel::BaseColor => material.albedo_texture = None,
        RecipeOutputChannel::Roughness => material.roughness_texture = None,
        RecipeOutputChannel::Metallic => material.metallic_texture = None,
        RecipeOutputChannel::Emission => material.emission_texture = None,
        RecipeOutputChannel::Height => material.height_texture = None,
        RecipeOutputChannel::Mask => {}
    }
}

fn write_channel(
    material: &mut Material,
    channel: RecipeOutputChannel,
    value: SocketValue,
) -> bool {
    let SocketValue::Texture(canvas) = value else {
        return false;
    };
    let slot = match channel {
        RecipeOutputChannel::BaseColor => &mut material.albedo_texture,
        RecipeOutputChannel::Roughness => &mut material.roughness_texture,
        RecipeOutputChannel::Metallic => &mut material.metallic_texture,
        RecipeOutputChannel::Emission => &mut material.emission_texture,
        RecipeOutputChannel::Height => &mut material.height_texture,
        RecipeOutputChannel::Mask => return false,
    };
    *slot = Some(canvas);
    true
}

/// Tipos dos soquetes de um nó (para a GUI desenhar portas e validar fios).
pub fn sockets(spec: &NodeSpec) -> (Vec<SocketType>, Vec<SocketType>) {
    (spec.inputs(), spec.outputs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> (Material, MaterialRecipe) {
        let material = Material::new("Test");
        let recipe = MaterialRecipe::new_for(&material);
        (material, recipe)
    }

    fn output_node(recipe: &MaterialRecipe) -> Uuid {
        recipe
            .graph
            .nodes
            .iter()
            .find(|n| matches!(n.spec, NodeSpec::Output { .. }))
            .unwrap()
            .id
    }

    #[test]
    fn initial_recipe_writes_the_base_color_map() {
        let (mut material, mut recipe) = recipe();
        recipe.evaluate_into(&mut material);
        assert_eq!(recipe.last_error, None);
        assert!(material.albedo_texture.is_some());
        assert_eq!(recipe.generated, vec![RecipeOutputChannel::BaseColor]);
    }

    #[test]
    fn checker_into_roughness_and_loose_nodes_do_not_break_evaluation() {
        let (mut material, mut recipe) = recipe();
        let checker = recipe.add_node("checker").unwrap();
        let output = recipe.add_node("output").unwrap();
        recipe
            .set_param(
                output,
                "channel",
                &RecipeParamValue::Choice("roughness".into()),
            )
            .unwrap();
        recipe.connect(checker, 0, output, 0).unwrap();
        // Nó solto (Tint sem entrada) e Saída sem entrada: ignorados.
        recipe.add_node("tint").unwrap();
        recipe.add_node("output").unwrap();
        recipe.evaluate_into(&mut material);
        assert_eq!(recipe.last_error, None);
        assert!(material.roughness_texture.is_some());
        assert!(material.albedo_texture.is_some());
    }

    #[test]
    fn disconnecting_an_output_clears_the_map_it_generated() {
        let (mut material, mut recipe) = recipe();
        recipe.evaluate_into(&mut material);
        let output = output_node(&recipe);
        assert!(recipe.disconnect(output, 0));
        recipe.evaluate_into(&mut material);
        assert!(material.albedo_texture.is_none());
        assert!(recipe.generated.is_empty());
    }

    #[test]
    fn params_are_validated_and_clamped_by_the_catalog() {
        let (_, mut recipe) = recipe();
        let mix = recipe.add_node("mix").unwrap();
        recipe
            .set_param(mix, "factor", &RecipeParamValue::Float(7.0))
            .unwrap();
        let factor = params_of(
            &recipe
                .graph
                .nodes
                .iter()
                .find(|n| n.id == mix)
                .unwrap()
                .spec,
        );
        assert_eq!(factor, vec![("factor", RecipeParamValue::Float(1.0))]);
        assert!(
            recipe
                .set_param(mix, "factor", &RecipeParamValue::Bool(true))
                .is_err()
        );
        assert!(
            recipe
                .set_param(mix, "nope", &RecipeParamValue::Float(0.1))
                .is_err()
        );
        assert!(
            recipe
                .set_param(mix, "factor", &RecipeParamValue::Float(f64::NAN))
                .is_err()
        );
        let output = output_node(&recipe);
        assert!(
            recipe
                .set_param(output, "channel", &RecipeParamValue::Choice("mask".into()))
                .is_err()
        );
    }

    #[test]
    fn connect_replaces_an_occupied_input_and_refuses_cycles_and_type_mismatches() {
        let (_, mut recipe) = recipe();
        let output = output_node(&recipe);
        let checker = recipe.add_node("checker").unwrap();
        recipe.connect(checker, 0, output, 0).unwrap();
        assert_eq!(
            recipe.graph.edges.iter().filter(|e| e.to == output).count(),
            1
        );
        let tint_a = recipe.add_node("tint").unwrap();
        let tint_b = recipe.add_node("tint").unwrap();
        recipe.connect(tint_a, 0, tint_b, 0).unwrap();
        let edges = recipe.graph.edges.clone();
        assert!(recipe.connect(tint_b, 0, tint_a, 0).is_err(), "ciclo");
        assert_eq!(recipe.graph.edges, edges, "recusa não altera o grafo");
        assert!(
            recipe.connect(checker, 3, tint_a, 0).is_err(),
            "soquete inválido"
        );
    }

    #[test]
    fn repair_fixes_hostile_params_and_edges() {
        let (_, mut recipe) = recipe();
        let checker = recipe.add_node("checker").unwrap();
        if let Some(node) = recipe.graph.nodes.iter_mut().find(|n| n.id == checker) {
            node.spec = NodeSpec::Checker {
                cell_size: 0,
                color_a: [f32::NAN, 2.0, -1.0, 1.0],
                color_b: [0.0; 4],
            };
        }
        recipe.graph.edges.push(RecipeEdge {
            from: Uuid::new_v4(),
            from_socket: 0,
            to: checker,
            to_socket: 0,
        });
        let duplicate = recipe.graph.nodes[0].clone();
        recipe.graph.nodes.push(duplicate);
        assert!(recipe.repair() >= 3);
        for edge in &recipe.graph.edges {
            assert!(!reaches(&recipe.graph, edge.to, edge.from), "sem ciclos");
        }
        let params = params_of(
            &recipe
                .graph
                .nodes
                .iter()
                .find(|n| n.id == checker)
                .unwrap()
                .spec,
        );
        assert!(params.contains(&("cell_size", RecipeParamValue::Int(1))));
    }

    #[test]
    fn every_catalog_kind_has_a_default_spec_and_round_trips() {
        for kind in NODE_CATALOG {
            let spec = default_spec(kind.id).unwrap();
            assert_eq!(kind_of(&spec).map(|k| k.id), Some(kind.id));
            let keys: Vec<_> = params_of(&spec).into_iter().map(|(k, _)| k).collect();
            let declared: Vec<_> = kind.params.iter().map(|p| p.key).collect();
            let mut a = keys.clone();
            let mut b = declared.clone();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "{}: parâmetros do catálogo × valores", kind.id);
        }
    }
}
