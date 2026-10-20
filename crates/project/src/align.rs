//! Alinhamento e distribuição 3D de objetos no espaço de mundo.
//!
//! Fornece estruturas de bounding box AABB e algoritmos matemáticos puros para
//! alinhar e distribuir objetos de cena segundo referências canônicas
//! (Objeto Ativo, Caixa da Seleção e Origem do Mundo).
//!
//! Em conformidade com o padrão Craft (P01/G10/H19/H25) e diretrizes de
//! robustez (ausência de panics, validação estrita de números finitos).

use glam::Vec3;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::Asset;

// --- Constantes ---

const MIN_OBJECT_SPAN_EPSILON: f32 = 1.0e-7;

// --- Estruturas e Enums Principais ---

/// Caixa delimitadora 3D alinhada aos eixos (AABB) em coordenadas de mundo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox3D {
    pub min: Vec3,
    pub max: Vec3,
}

/// Referência de ancoragem para operações de alinhamento.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlignReference {
    /// O objeto ativo permanece fixo como âncora; os demais movem-se até ele.
    #[default]
    ActiveObject,
    /// Alinha em relação aos limites extremos da seleção combinada.
    SelectionBounds,
    /// Alinha em relação à origem do sistema de coordenadas do mundo (0, 0, 0).
    WorldOrigin,
}

/// Modo de alinhamento ao longo do eixo escolhido.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlignMode {
    /// Borda mínima (-X esquerda, -Y base/chão, -Z fundo).
    Min,
    /// Centro geométrico médio ((min + max) / 2).
    #[default]
    Center,
    /// Borda máxima (+X direita, +Y topo, +Z frente).
    Max,
}

/// Eixo 3D sobre o qual o alinhamento ou distribuição é calculado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignAxis {
    X,
    Y,
    Z,
}

impl AlignAxis {
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    #[inline]
    pub fn vector_from_scalar(self, scalar: f32) -> Vec3 {
        match self {
            Self::X => Vec3::new(scalar, 0.0, 0.0),
            Self::Y => Vec3::new(0.0, scalar, 0.0),
            Self::Z => Vec3::new(0.0, 0.0, scalar),
        }
    }
}

// --- Tipos de Erro ---

/// Erros decorrentes da validação matemática ou topológica do alinhamento.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AlignError {
    #[error(
        "Quantidade insuficiente de objetos para alinhamento: requer pelo menos {required}, encontrados {found}"
    )]
    InsufficientObjects { required: usize, found: usize },

    #[error("Objeto ativo com ID {0} não encontrado na lista de objetos selecionados")]
    ActiveObjectNotFound(Uuid),

    #[error("O objeto com ID {0} possui malha vazia ou caixa delimitadora com valores não finitos")]
    InvalidBounds(Uuid),

    #[error("O cálculo de alinhamento resultou em delta não finito")]
    NonFiniteDelta,

    #[error("Distribuição requer caixa com span maior que zero no eixo selecionado")]
    DegenerateSpan,
}

// --- Implementações de Métodos (impl BoundingBox3D) ---

impl BoundingBox3D {
    /// Cria uma nova bounding box com limites explícitos.
    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Retorna o centro geométrico da caixa.
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Retorna as dimensões (largura, altura, profundidade) da caixa.
    #[inline]
    pub fn size(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    /// Verifica se as coordenadas mínimas e máximas são valores finitos válidos.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }

    /// Funde esta caixa com outra, gerando a menor AABB que engloba ambas.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Testa interseção volumétrica estrita entre duas AABBs no espaço de mundo (broad-phase).
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Constrói a AABB a partir de um iterador arbitrário de pontos no espaço.
    pub fn from_points(iter: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        let (min, max) = iter.into_iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), p| (min.min(p), max.max(p)),
        );
        if min.is_finite() && max.is_finite() && min.x <= max.x && min.y <= max.y && min.z <= max.z
        {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Constrói a AABB em espaço de mundo para um `Asset` existente.
    pub fn from_asset(asset: &Asset) -> Option<Self> {
        Self::from_points(asset.world_positions())
    }

    /// Retorna o valor escalar característico de um eixo conforme o modo de alinhamento.
    pub fn scalar_for(&self, axis: AlignAxis, mode: AlignMode) -> f32 {
        let idx = axis.index();
        match mode {
            AlignMode::Min => self.min[idx],
            AlignMode::Center => self.center()[idx],
            AlignMode::Max => self.max[idx],
        }
    }
}

// --- Funções Públicas de Cálculo de Alinhamento ---

/// Calcula os vetores de deslocamento mundial `Vec<(Uuid, Vec3)>` necessários para
/// alinhar os objetos informados segundo os parâmetros requisitados.
///
/// Não muta os objetos; devolve a lista de deltas para aplicação transacional pelo chamador.
pub fn compute_alignment_deltas(
    assets: &[&Asset],
    active_id: Option<Uuid>,
    axis: AlignAxis,
    mode: AlignMode,
    reference: AlignReference,
) -> Result<Vec<(Uuid, Vec3)>, AlignError> {
    let min_required = match reference {
        AlignReference::WorldOrigin => 1,
        AlignReference::ActiveObject | AlignReference::SelectionBounds => 2,
    };

    if assets.len() < min_required {
        return Err(AlignError::InsufficientObjects {
            required: min_required,
            found: assets.len(),
        });
    }

    // Coleta as caixas de todos os assets participantes
    let mut bounds_list = Vec::with_capacity(assets.len());
    for asset in assets {
        let bounds = BoundingBox3D::from_asset(asset).ok_or(AlignError::InvalidBounds(asset.id))?;
        bounds_list.push((asset.id, bounds));
    }

    // Determina o valor escalar alvo do eixo
    let target_scalar =
        match reference {
            AlignReference::WorldOrigin => 0.0,
            AlignReference::ActiveObject => {
                let active_uuid =
                    active_id.ok_or_else(|| AlignError::ActiveObjectNotFound(Uuid::nil()))?;
                let (_, active_bounds) = bounds_list
                    .iter()
                    .find(|(id, _)| *id == active_uuid)
                    .ok_or(AlignError::ActiveObjectNotFound(active_uuid))?;
                active_bounds.scalar_for(axis, mode)
            }
            AlignReference::SelectionBounds => {
                let first_bounds = bounds_list.first().map(|(_, b)| *b).ok_or(
                    AlignError::InsufficientObjects {
                        required: min_required,
                        found: 0,
                    },
                )?;
                let union_bounds = bounds_list
                    .iter()
                    .skip(1)
                    .fold(first_bounds, |acc, (_, b)| acc.union(b));
                union_bounds.scalar_for(axis, mode)
            }
        };

    if !target_scalar.is_finite() {
        return Err(AlignError::NonFiniteDelta);
    }

    // Calcula os vetores de translação para cada objeto
    let mut deltas = Vec::with_capacity(assets.len());
    for (id, bounds) in bounds_list {
        if reference == AlignReference::ActiveObject && Some(id) == active_id {
            // O objeto âncora ativo permanece estritamente imóvel
            deltas.push((id, Vec3::ZERO));
            continue;
        }

        let current_scalar = bounds.scalar_for(axis, mode);
        let delta_scalar = target_scalar - current_scalar;

        if !delta_scalar.is_finite() {
            return Err(AlignError::NonFiniteDelta);
        }

        let delta_vec = axis.vector_from_scalar(delta_scalar);
        deltas.push((id, delta_vec));
    }

    Ok(deltas)
}

/// Calcula os vetores de deslocamento mundial `Vec<(Uuid, Vec3)>` necessários para
/// distribuir uniformemente o espaçamento entre 3 ou mais objetos ao longo de um eixo.
///
/// Os dois objetos nas extremidades permanecem em suas posições; os objetos intermediários
/// são distribuídos com gaps idênticos entre suas bordas consecutivas.
pub fn compute_distribution_deltas(
    assets: &[&Asset],
    axis: AlignAxis,
) -> Result<Vec<(Uuid, Vec3)>, AlignError> {
    if assets.len() < 3 {
        return Err(AlignError::InsufficientObjects {
            required: 3,
            found: assets.len(),
        });
    }

    let axis_idx = axis.index();

    // Coleta as caixas
    let mut item_list = Vec::with_capacity(assets.len());
    for asset in assets {
        let bounds = BoundingBox3D::from_asset(asset).ok_or(AlignError::InvalidBounds(asset.id))?;
        item_list.push((asset.id, bounds));
    }

    // Ordena os objetos pela borda mínima ao longo do eixo
    item_list.sort_by(|(_, a), (_, b)| {
        a.min[axis_idx]
            .partial_cmp(&b.min[axis_idx])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let first_min = item_list
        .first()
        .map(|(_, b)| b.min[axis_idx])
        .unwrap_or(0.0);
    let last_max = item_list
        .last()
        .map(|(_, b)| b.max[axis_idx])
        .unwrap_or(0.0);
    let total_span = last_max - first_min;

    if total_span <= MIN_OBJECT_SPAN_EPSILON || !total_span.is_finite() {
        return Err(AlignError::DegenerateSpan);
    }

    // Soma do tamanho de todos os objetos no eixo
    let total_object_sizes: f32 = item_list.iter().map(|(_, b)| b.size()[axis_idx]).sum();

    let free_space = total_span - total_object_sizes;
    let gaps_count = (item_list.len() - 1) as f32;
    let gap = (free_space / gaps_count).max(0.0);

    let mut deltas = Vec::with_capacity(item_list.len());
    let mut current_pos = first_min;

    for (i, (id, bounds)) in item_list.into_iter().enumerate() {
        let size = bounds.size()[axis_idx];
        let current_min = bounds.min[axis_idx];

        let delta_scalar = if i == 0 {
            // O primeiro objeto é a âncora inicial
            0.0
        } else if i == assets.len() - 1 {
            // O último objeto é a âncora final
            0.0
        } else {
            let desired_min = current_pos;
            desired_min - current_min
        };

        if !delta_scalar.is_finite() {
            return Err(AlignError::NonFiniteDelta);
        }

        deltas.push((id, axis.vector_from_scalar(delta_scalar)));
        current_pos += size + gap;
    }

    Ok(deltas)
}
