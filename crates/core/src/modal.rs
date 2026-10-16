//! Transactional viewport previews. Each update starts from the original mesh;
//! only commit records history, and cancel restores the exact project snapshot.

use crate::context::{CommandContext, EditorView};
use glam::{EulerRot, Quat, Vec3};
use petunia_mesh::Mesh;
use petunia_project::{Project, ProjectChanges};

use crate::{AppState, EditMode, Selection, SnapKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModalKind {
    Move,
    Rotate,
    Scale,
    Extrude,
    ExtrudeIndividual,
    Inset,
    Bevel,
    PushPull,
}

impl ModalKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Extrude => "Extrude",
            Self::ExtrudeIndividual => "Extrude Individual",
            Self::Inset => "Inset (factor)",
            Self::Bevel => "Bevel",
            Self::PushPull => "Push/Pull",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalConstraint {
    #[default]
    Free,
    Axis(usize),
    /// Plane perpendicular to the given axis (0 = YZ, 1 = XZ, 2 = XY).
    Plane(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalError {
    AlreadyActive,
    NoActiveMesh,
    NoSelection,
    FacesRequired,
    EdgesRequired,
    InvalidInput,
    InvalidMesh,
    NoActiveOperation,
    UnsupportedTopology,
    ActiveLocked,
    UnsupportedFrame,
}

impl std::fmt::Display for ModalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AlreadyActive => "Confirm or cancel the active operation first",
            Self::NoActiveMesh => "No active mesh",
            Self::NoSelection => "Select geometry first",
            Self::FacesRequired => "Select a face first",
            Self::EdgesRequired => "Select an edge first",
            Self::InvalidInput => "Invalid or out-of-range transform value",
            Self::InvalidMesh => "Operation rejected: invalid mesh",
            Self::NoActiveOperation => "No active operation",
            Self::UnsupportedTopology => {
                "Bevel supports one manifold edge with simple corners; select a supported edge"
            }
            Self::ActiveLocked => "Active object is locked",
            Self::UnsupportedFrame => "Object frame cannot be changed safely (shear, procedural source or rig bind compensation)",
        })
    }
}

impl std::error::Error for ModalError {}

/// Identificador monotônico de cada operação modal aberta.
static NEXT_MODAL_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub struct ModalOp {
    /// Identidade do gesto: caches por gesto (snap, prévia) usam esta chave.
    id: u64,
    pub kind: ModalKind,
    pub constraint: ModalConstraint,
    pub frame: crate::TransformFrame,
    pub pivot: Vec3,
    pub normal: Vec3,
    /// Distance (Extrude/Push/Inset/Bevel, meters), degrees, or scale factor.
    pub value: f32,
    /// Absolute XYZ delta, Euler XYZ degrees, or per-axis scale factors.
    pub components: Vec3,
    /// Frozen per-point pivots for the Individual Origins policy.
    pivots: Option<Vec<Vec3>>,
    individual_origins: bool,
    object_mode: bool,
    edit_origin: bool,
    original: Project,
    pub source_revision: u64,
    pub published_revision: u64,
    selection: Selection,
    source: Mesh,
    changed: bool,
    /// Alvo em que o último update encaixou de fato (não apenas "snap ligado").
    snap: Option<SnapKind>,
    /// Estado de antes de um prelúdio (imprint de região, forma nova) feito
    /// fora do histórico: o gesto inteiro vira uma entrada de Undo e Esc
    /// restaura exatamente este estado (constituição 11).
    before_prelude: Option<Box<(Project, Selection)>>,
    /// A malha ativa é uma folha de região solta: extrudar para o lado
    /// negativo inverte todas as faces para o sólido continuar voltado para fora.
    flip_when_negative: bool,
    /// Prévia incremental: evita clonar e validar a malha inteira por evento.
    preview: PreviewCache,
    /// Edit Mode: frame local do objeto → mundo. Pivô, normal, eixos e
    /// distâncias do gesto são de mundo; `source` e a malha publicada são
    /// locais, então posições são lidas como `W·local` e escritas como
    /// `W⁻¹·mundo`. Identidade em Object Mode (que ainda move vértices).
    object_to_world: glam::Mat4,
}

/// Estado da prévia incremental de uma operação modal.
///
/// A prévia exata (clonar a origem, aplicar a operação, validar) roda no
/// primeiro evento e sempre que a topologia pode mudar; entre esses pontos as
/// posições são escritas direto na malha publicada. O resultado confirmado é
/// sempre o exato: uma prévia interpolada é recalculada antes do commit.
#[derive(Default)]
struct PreviewCache {
    /// A malha publicada tem a topologia da origem (Move/Rotate/Scale/Push).
    source_topology_live: bool,
    /// Vértices da origem que a transformação move.
    affected: Option<Vec<u32>>,
    extrude: Option<ExtrudePreview>,
    linear: LinearPreview,
    /// Modo objeto: matriz rígida (original → atual) de cada objeto em
    /// movimento, para o renderer reaproveitar a geometria triangulada.
    rigid: Vec<(uuid::Uuid, glam::Mat4)>,
}

/// Extrude: a topologia com a tampa em distância zero, construída uma vez.
struct ExtrudePreview {
    base: Mesh,
    cap: Vec<u32>,
    /// Estado de inversão das faces publicado (`None` = prévia sem extrusão).
    live_flip: Option<bool>,
}

/// Inset/Bevel/Extrude Individual: posições afins no valor, verificadas.
/// `(valor de referência, [(vértice, posição na referência, inclinação)])`.
type LinearModel = (f32, Vec<(u32, Vec3, Vec3)>);

#[derive(Default)]
struct LinearPreview {
    samples: Vec<(f32, Mesh)>,
    model: Option<LinearModel>,
    validated: bool,
    disabled: bool,
    /// A malha publicada veio do modelo (não da operação exata).
    predicted: bool,
    since_check: u32,
    range: (f32, f32),
}

/// Prévias interpoladas entre duas verificações exatas.
const LINEAR_RECHECK_EVERY: u32 = 12;

impl ModalOp {
    /// A prévia difere do estado original (confirmar criará uma entrada de Undo).
    pub(crate) fn changed(&self) -> bool {
        self.changed
    }

    /// Verdadeiro somente quando o último update foi atraído por um alvo de snap.
    pub fn snapped(&self) -> bool {
        self.snap.is_some()
    }

    /// Tipo do alvo que encaixou no último update, para o rótulo do HUD.
    pub fn snap_kind(&self) -> Option<SnapKind> {
        self.snap
    }

    /// Vértices da malha de origem que se movem com a operação: não servem de
    /// alvo de snap (senão o ponto encaixaria em si mesmo).
    /// Malha de origem congelada no início da operação (sem a prévia).
    pub fn source_mesh(&self) -> &Mesh {
        &self.source
    }

    /// Identidade estável durante toda a operação.
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn moving_vertices(&self) -> Vec<bool> {
        self.source
            .verts
            .iter()
            .map(|v| !self.edit_origin && v.selected)
            .collect()
    }

    /// Snap candidates use the frozen geometry AND frame, including after a
    /// preview has changed Asset::transform or rebased local coordinates.
    pub fn snap_source_asset(&self, id: uuid::Uuid) -> Option<&petunia_project::Asset> {
        self.original.assets.iter().find(|a| a.id == id)
    }

    pub fn excludes_snap_object(&self, id: uuid::Uuid) -> bool {
        self.object_mode
            && !self.edit_origin
            && self.original.active().is_none_or(|a| a.id != id)
            && self.selection.assets.contains(&id)
            && self
                .snap_source_asset(id)
                .is_some_and(|asset| !asset.locked)
    }

    /// Ponto de mundo que representa o valor atual, quando ele é uma posição.
    ///
    /// Move usa o deslocamento absoluto; Extrude/Push-Pull, a distância ao longo
    /// da normal. Rotate, Scale, Inset e Bevel não têm um ponto de mundo
    /// correspondente (graus, fator ou fração), então retornam `None`.
    pub fn current_point(&self) -> Option<Vec3> {
        match self.kind {
            ModalKind::Move => Some(self.pivot + self.components),
            ModalKind::Extrude | ModalKind::ExtrudeIndividual | ModalKind::PushPull => {
                Some(self.pivot + self.normal * self.value)
            }
            ModalKind::Rotate | ModalKind::Scale | ModalKind::Inset | ModalKind::Bevel => None,
        }
    }

    /// Produz a projeção de feedback unificado da operação modal (MF-001 a MF-008).
    pub fn feedback(
        &self,
        current_revision: u64,
        target_id: Option<uuid::Uuid>,
        target_name: Option<&str>,
        active_mesh: Option<&Mesh>,
    ) -> crate::modeling_feedback::OperationFeedback {
        use crate::modeling_feedback::{
            ActiveDimension, ActiveDimensionDescriptor, AffectedRegion, FeedbackMarker,
            FeedbackWarning, MeasurementQuality, OperationFeedback, WarningSeverity,
        };

        let is_stale = current_revision != self.published_revision;
        let mut warnings = Vec::new();
        let mut dimensions = Vec::new();
        let mut markers = Vec::new();

        if is_stale {
            warnings.push(FeedbackWarning::error(
                "STALE_REVISION",
                format!(
                    "Document revision modified from {} to {} during operation",
                    self.source_revision, current_revision
                ),
            ));
        }

        match self.kind {
            ModalKind::Extrude | ModalKind::ExtrudeIndividual => {
                dimensions.push(ActiveDimensionDescriptor {
                    label: "Distance".to_string(),
                    value: self.value,
                    unit: "m".into(),
                    min: None,
                    max: None,
                });
                if !self.value.is_finite() {
                    warnings.push(FeedbackWarning::error(
                        "INVALID_DISTANCE",
                        "Extrude distance must be finite",
                    ));
                } else if self.value < 0.0 {
                    warnings.push(FeedbackWarning::info(
                        "EXTRUDE_NEGATIVE",
                        "Extrusion in negative direction creates internal cavity",
                    ));
                }
            }
            ModalKind::Inset => {
                dimensions.push(ActiveDimensionDescriptor {
                    label: "Offset".to_string(),
                    value: self.value,
                    unit: "".into(),
                    min: Some(0.0),
                    max: Some(1.0),
                });
                if !self.value.is_finite() || self.value < 0.0 || self.value > 1.0 {
                    warnings.push(FeedbackWarning::exceeded_limit(
                        "factor",
                        self.value,
                        1.0,
                        "Inset factor must be between 0.0 and 1.0",
                    ));
                }
                if self.value >= 0.85 {
                    if self.value <= 1.0 {
                        warnings.push(FeedbackWarning::approaching_limit(
                            "factor",
                            self.value,
                            1.0,
                            "Inset factor approaching self-intersection threshold (> 0.85)",
                        ));
                    }
                    warnings.push(FeedbackWarning::warning(
                        "INSET_COLLAPSE_LIMIT",
                        "Inset factor approaching face collapse limit",
                    ));
                    markers.push(FeedbackMarker::LimitBoundary {
                        center: self.pivot.to_array(),
                        radius: 1.0,
                        label: "Collapse Limit".to_string(),
                    });
                }
            }
            ModalKind::Bevel => {
                dimensions.push(ActiveDimensionDescriptor {
                    label: "Offset".to_string(),
                    value: self.value,
                    unit: "m".into(),
                    min: Some(0.0),
                    max: None,
                });
                if !self.value.is_finite() || self.value < 0.0 {
                    warnings.push(FeedbackWarning::exceeded_limit(
                        "offset",
                        self.value,
                        0.0,
                        "Bevel offset must be non-negative",
                    ));
                }
                if self.value > 0.5 {
                    warnings.push(FeedbackWarning::warning(
                        "BEVEL_OVERLAP_LIMIT",
                        "Bevel offset exceeds half edge length, causing geometric overlap",
                    ));
                }
            }
            ModalKind::Scale => {
                dimensions.push(ActiveDimensionDescriptor {
                    label: "Factor".to_string(),
                    value: self.value,
                    unit: "×".into(),
                    min: Some(0.0),
                    max: None,
                });
                if self.value.abs() < 1e-4 {
                    warnings.push(FeedbackWarning::warning(
                        "SCALE_NEAR_ZERO",
                        "Scale factor near zero will collapse geometry",
                    ));
                }
                if self.value < 0.0
                    || self.components.x < 0.0
                    || self.components.y < 0.0
                    || self.components.z < 0.0
                {
                    warnings.push(FeedbackWarning::warning(
                        "NEGATIVE_SCALE",
                        "Negative scale inverts normal orientation and face winding",
                    ));
                }
            }
            ModalKind::Rotate => {
                dimensions.push(ActiveDimensionDescriptor {
                    label: "Angle".to_string(),
                    value: self.value,
                    unit: "°".into(),
                    min: None,
                    max: None,
                });
            }
            ModalKind::Move | ModalKind::PushPull => {
                dimensions.push(ActiveDimensionDescriptor {
                    label: "Distance".to_string(),
                    value: self.value,
                    unit: "m".into(),
                    min: None,
                    max: None,
                });
                if !self.value.is_finite() || !self.components.is_finite() {
                    warnings.push(FeedbackWarning::error(
                        "INVALID_VALUE",
                        "Transform values must be finite",
                    ));
                }
            }
        }

        if let Some(curr) = self.current_point() {
            markers.push(FeedbackMarker::Point {
                position: curr.to_array(),
                label: format!("{} Current", self.kind.label()),
                is_snapped: self.snapped(),
            });
        }

        let (verts, edges, faces) = if self.object_mode {
            (
                self.source.verts.len(),
                self.source.edges_unique().len(),
                self.source.faces.len(),
            )
        } else {
            (
                self.source.verts.iter().filter(|v| v.selected).count(),
                self.source.selected_edges.len(),
                self.source.faces.iter().filter(|f| f.selected).count(),
            )
        };

        let can_commit = !is_stale
            && !warnings
                .iter()
                .any(|w| w.severity == WarningSeverity::Error);

        let (op_id, active_dim) = match self.kind {
            ModalKind::Extrude => ("Extrude", ActiveDimension::Distance),
            ModalKind::ExtrudeIndividual => ("ExtrudeIndividual", ActiveDimension::Distance),
            ModalKind::Inset => ("Inset", ActiveDimension::Offset),
            ModalKind::Bevel => ("Bevel", ActiveDimension::Offset),
            ModalKind::Scale => ("Scale", ActiveDimension::Factor),
            ModalKind::Rotate => ("Rotate", ActiveDimension::Angle),
            ModalKind::Move => ("Move", ActiveDimension::Distance),
            ModalKind::PushPull => ("PushPull", ActiveDimension::Distance),
        };

        OperationFeedback {
            operation_id: op_id.to_string(),
            operation_name: self.kind.label().to_string(),
            active_dimension: active_dim,
            numeric_value: self.value,
            preview_mesh: active_mesh.cloned(),
            commit_enabled: can_commit,
            source_revision: self.source_revision,
            current_revision,
            is_stale,
            can_commit,
            target_id,
            target_name: target_name.unwrap_or("Active Mesh").to_string(),
            quality: if is_stale {
                MeasurementQuality::Stale
            } else {
                MeasurementQuality::Exact
            },
            affected_region: AffectedRegion {
                vertices_count: verts,
                edges_count: edges,
                faces_count: faces,
            },
            ghost_mesh: Some(self.source.clone()),
            dimensions,
            markers,
            warnings,
            partial_selection: None,
            operand_a: target_id,
            operand_b: None,
            plane: None,
            radius: None,
            strength: None,
        }
    }
}

fn valid_mesh(mesh: &Mesh) -> bool {
    mesh.verts.iter().all(|v| v.vec().is_finite())
        && mesh.faces.iter().all(|f| {
            f.verts.len() >= 3
                && f.uv.len() == f.verts.len()
                && f.verts.iter().all(|&v| (v as usize) < mesh.verts.len())
                && f.uv.iter().flatten().all(|v| v.is_finite())
        })
        && mesh.selected_edges.iter().all(|&(a, b)| {
            a != b && (a as usize) < mesh.verts.len() && (b as usize) < mesh.verts.len()
        })
}

fn same_geometry(a: &Mesh, b: &Mesh) -> bool {
    a.verts.len() == b.verts.len()
        && a.faces.len() == b.faces.len()
        && a.verts
            .iter()
            .zip(&b.verts)
            .all(|(a, b)| a.pos == b.pos && a.color == b.color)
        && a.faces
            .iter()
            .zip(&b.faces)
            .all(|(a, b)| a.verts == b.verts && a.uv == b.uv)
}

/// Connected selected elements share an individual pivot. Loose selected points
/// retain their own position; an object uses its complete geometric center.
fn individual_pivots(mesh: &Mesh, object: bool) -> Vec<Vec3> {
    if object {
        let center =
            mesh.verts.iter().map(|v| v.vec()).sum::<Vec3>() / mesh.verts.len().max(1) as f32;
        return vec![center; mesh.verts.len()];
    }
    let mut neighbors = vec![Vec::new(); mesh.verts.len()];
    for (a, b) in mesh.edges_unique() {
        let (a, b) = (a as usize, b as usize);
        if mesh.verts.get(a).is_some_and(|v| v.selected)
            && mesh.verts.get(b).is_some_and(|v| v.selected)
        {
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
    }
    let mut pivots: Vec<_> = mesh.verts.iter().map(|v| v.vec()).collect();
    let mut visited = vec![false; mesh.verts.len()];
    for seed in 0..mesh.verts.len() {
        if visited[seed] || !mesh.verts[seed].selected {
            continue;
        }
        let mut stack = vec![seed];
        let mut group = Vec::new();
        while let Some(index) = stack.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            group.push(index);
            stack.extend(neighbors[index].iter().copied().filter(|&i| !visited[i]));
        }
        let center = group.iter().map(|&i| mesh.verts[i].vec()).sum::<Vec3>() / group.len() as f32;
        for index in group {
            pivots[index] = center;
        }
    }
    pivots
}

fn frame_components(
    modal: &ModalOp,
    translation: Vec3,
    value: f32,
    snap: Option<SnapKind>,
) -> Result<Vec3, ModalError> {
    if matches!(modal.constraint, ModalConstraint::Axis(i) | ModalConstraint::Plane(i) if i > 2) {
        return Err(ModalError::InvalidInput);
    }
    Ok(match modal.kind {
        ModalKind::Move => match modal.constraint {
            ModalConstraint::Free => translation,
            ModalConstraint::Axis(i) => {
                let a = modal.frame.axis(i);
                a * if snap.is_some() {
                    translation.dot(a)
                } else {
                    value
                }
            }
            ModalConstraint::Plane(i) => {
                let a = modal.frame.axis(i);
                translation - a * translation.dot(a)
            }
        },
        ModalKind::Rotate => {
            let normal = match modal.constraint {
                ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => modal.frame.axis(i),
                ModalConstraint::Free => modal.normal,
            };
            let (x, y, z) =
                Quat::from_axis_angle(normal, value.to_radians()).to_euler(EulerRot::XYZ);
            Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
        }
        ModalKind::Scale => match modal.constraint {
            ModalConstraint::Free => Vec3::splat(value),
            ModalConstraint::Axis(i) => Vec3::ONE + modal.frame.axis(i) * (value - 1.0),
            ModalConstraint::Plane(i) => Vec3::splat(value) + modal.frame.axis(i) * (1.0 - value),
        },
        _ => Vec3::ZERO,
    })
}

fn frame_matrix(kind: ModalKind, components: Vec3, pivot: Vec3) -> glam::Mat4 {
    let around = |m| glam::Mat4::from_translation(pivot) * m * glam::Mat4::from_translation(-pivot);
    match kind {
        ModalKind::Move => glam::Mat4::from_translation(components),
        ModalKind::Rotate => around(glam::Mat4::from_quat(Quat::from_euler(
            EulerRot::XYZ,
            components.x.to_radians(),
            components.y.to_radians(),
            components.z.to_radians(),
        ))),
        ModalKind::Scale => around(glam::Mat4::from_scale(components)),
        _ => glam::Mat4::IDENTITY,
    }
}

/// Keep the old baked representation operational until explicit origin
/// normalization. Do not overwrite its accumulated metadata with a new TRS.
fn legacy_object_preview(
    source: &petunia_project::Asset,
    next: &mut petunia_project::Asset,
    kind: ModalKind,
    components: Vec3,
    world_delta: glam::Mat4,
) -> Result<bool, ModalError> {
    if source.skeleton_id.is_some() || source.skin_data.is_some() {
        return Err(ModalError::UnsupportedFrame);
    }
    let local = source.world_matrix().inverse() * world_delta * source.world_matrix();
    if !local.is_finite() {
        return Err(ModalError::InvalidInput);
    }
    for (vertex, original) in next.mesh.verts.iter_mut().zip(&source.mesh.verts) {
        let point = local.transform_point3(original.vec());
        if !point.is_finite() {
            return Err(ModalError::InvalidInput);
        }
        vertex.pos = point.to_array();
    }
    next.origin = source
        .origin
        .map(|p| local.transform_point3(Vec3::from(p)).to_array());
    match kind {
        ModalKind::Move => next.position = (Vec3::from(source.position) + components).to_array(),
        ModalKind::Rotate => next.rotation = (Vec3::from(source.rotation) + components).to_array(),
        ModalKind::Scale => next.scale = (Vec3::from(source.scale) * components).to_array(),
        _ => return Err(ModalError::InvalidInput),
    }
    if !Vec3::from(next.position).is_finite()
        || !Vec3::from(next.rotation).is_finite()
        || !Vec3::from(next.scale).is_finite()
        || next.origin.is_some_and(|p| !Vec3::from(p).is_finite())
    {
        return Err(ModalError::InvalidInput);
    }
    Ok(!same_geometry(&next.mesh, &source.mesh)
        || next.position != source.position
        || next.rotation != source.rotation
        || next.scale != source.scale
        || next.origin != source.origin)
}

impl AppState {
    /// Direção de Extrude/Push conforme a restrição ativa.
    fn modal_direction(modal: &ModalOp) -> Vec3 {
        match modal.constraint {
            ModalConstraint::Axis(i) => modal.frame.axis(i),
            ModalConstraint::Plane(i) => {
                let a = modal.frame.axis(i);
                (modal.normal - a * modal.normal.dot(a)).normalize_or_zero()
            }
            ModalConstraint::Free => modal.normal,
        }
    }
}

/// Sessão interativa de transformação modal por ponteiro e teclado numérico.
#[derive(Debug, Clone)]
pub struct PointerSession {
    /// Posição 2D de tela inicial do clique/arrasto (pixels lógicos).
    pub anchor: [f32; 2],
    /// Buffer de entrada de texto numérico (ex: "1.5", "-45").
    pub numeric: String,
    /// Flag indicando se a sessão foi iniciada via arraste de gizmo.
    pub drag_handle: bool,
    /// Flag indicando se a pré-visualização atual é válida.
    pub valid_preview: bool,
    /// Última posição 2D de tela registrada.
    pub last_pos: [f32; 2],
}

impl PointerSession {
    pub fn new(anchor: [f32; 2], drag_handle: bool) -> Self {
        Self {
            anchor,
            numeric: String::new(),
            drag_handle,
            valid_preview: true,
            last_pos: anchor,
        }
    }

    pub fn push_char(&mut self, c: char) -> bool {
        if (c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')) && self.numeric.len() < 64 {
            self.numeric.push(if c == ',' { '.' } else { c });
            true
        } else {
            false
        }
    }

    pub fn pop_char(&mut self) -> Option<char> {
        self.numeric.pop()
    }

    pub fn parse_numeric(&self) -> Option<f32> {
        self.numeric.parse::<f32>().ok()
    }
}

impl CommandContext<'_> {
    pub fn begin_modal(&mut self, kind: ModalKind) -> Result<(), ModalError> {
        if self.modal.is_some()
            || self.mesh_preview.is_some()
            || self.paint_stroke.is_some()
            || self.uv_transform.is_some()
        {
            return Err(ModalError::AlreadyActive);
        }
        if self.is_active_locked() {
            return Err(ModalError::ActiveLocked);
        }
        if self.editor.edit_pivot {
            if kind != ModalKind::Move {
                return Err(ModalError::UnsupportedFrame);
            }
            self.document
                .active()
                .ok_or(ModalError::NoActiveMesh)?
                .validate_origin_rebase()
                .map_err(|_| ModalError::UnsupportedFrame)?;
        }
        self.sync_selection();
        let mesh = self
            .document
            .active_mesh()
            .ok_or(ModalError::NoActiveMesh)?;
        if !valid_mesh(mesh) {
            return Err(ModalError::InvalidMesh);
        }
        let mut source = mesh.clone();
        let transform = matches!(kind, ModalKind::Move | ModalKind::Rotate | ModalKind::Scale);
        if transform {
            self.gizmo_mode = kind;
        }
        if transform && (self.edit_mode() == EditMode::Object || self.editor.edit_pivot) {
            source.select_all();
        } else {
            // Edições topológicas ou de sub-elementos congelam a primitiva paramétrica em malha B-Rep estática.
            self.freeze_active_primitive();

            // Face and edge selection must transform their vertices too.
            for face in &source.faces {
                if face.selected {
                    for &vi in &face.verts {
                        source.verts[vi as usize].selected = true;
                    }
                }
            }
            for &(a, b) in &source.selected_edges {
                source.verts[a as usize].selected = true;
                source.verts[b as usize].selected = true;
            }
        }
        if !source.has_selection() || source.verts.is_empty() {
            return Err(ModalError::NoSelection);
        }
        if matches!(
            kind,
            ModalKind::Extrude
                | ModalKind::ExtrudeIndividual
                | ModalKind::Inset
                | ModalKind::PushPull
        ) {
            if source.selected_face_count() == 0 {
                source.sync_face_selection_from_verts();
            }
            if source.selected_face_count() == 0 {
                return Err(ModalError::FacesRequired);
            }
        }
        if kind == ModalKind::Bevel && source.selected_edges.is_empty() {
            let sel_verts = source.verts.iter().filter(|v| v.selected).count();
            if sel_verts == 0 {
                source.sync_edge_selection_from_verts();
                if source.selected_edges.is_empty() {
                    return Err(ModalError::EdgesRequired);
                }
            }
        }
        let object_to_world = self
            .document
            .active()
            .map_or(glam::Mat4::IDENTITY, petunia_project::Asset::world_matrix);
        if !object_to_world.is_finite() || !object_to_world.inverse().is_finite() {
            return Err(ModalError::InvalidInput);
        }
        let mut normal = Vec3::ZERO;
        for (fi, face) in source.faces.iter().enumerate() {
            if face.selected {
                normal += source.face_normal(fi);
            }
        }
        // Normal local → mundo pela inversa-transposta.
        normal = object_to_world
            .inverse()
            .transpose()
            .transform_vector3(normal)
            .normalize_or_zero();
        if normal == Vec3::ZERO || transform {
            normal = self.camera.forward().normalize_or_zero();
        }
        let initial_constraint = match self.locked_axes {
            [true, false, false] => ModalConstraint::Axis(0),
            [false, true, false] => ModalConstraint::Axis(1),
            [false, false, true] => ModalConstraint::Axis(2),
            [false, true, true] => ModalConstraint::Plane(0),
            [true, false, true] => ModalConstraint::Plane(1),
            [true, true, false] => ModalConstraint::Plane(2),
            _ => ModalConstraint::Free,
        };
        let pivot = self.calculate_pivot(self.editor.pivot_point);
        if !pivot.is_finite() {
            return Err(ModalError::InvalidInput);
        }
        let frame = self.calculate_transform_frame(self.editor.transform_orientation);
        let individual_origins =
            transform && self.editor.pivot_point == crate::PivotPoint::IndividualOrigins;
        // Centroides locais → mundo (mapas afins preservam centroides).
        let pivots = individual_origins.then(|| {
            individual_pivots(&source, self.edit_mode() == EditMode::Object)
                .into_iter()
                .map(|p| object_to_world.transform_point3(p))
                .collect()
        });
        self.modal = Some(ModalOp {
            id: NEXT_MODAL_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            preview: PreviewCache::default(),
            kind,
            frame,
            pivots,
            individual_origins,
            object_mode: self.edit_mode() == EditMode::Object,
            edit_origin: self.editor.edit_pivot,
            constraint: initial_constraint,
            pivot,
            normal,
            value: if kind == ModalKind::Scale { 1.0 } else { 0.0 },
            components: if kind == ModalKind::Scale {
                Vec3::ONE
            } else {
                Vec3::ZERO
            },
            original: self.document.project.clone(),
            source_revision: self.document.project.revision_id(),
            published_revision: self.document.project.revision_id(),
            selection: self.editor.selection.clone(),
            source,
            changed: false,
            snap: None,
            before_prelude: None,
            flip_when_negative: false,
            object_to_world,
        });
        self.pending_modal = None;
        self.mark_dirty();
        Ok(())
    }

    /// Inicia `kind` depois de um prelúdio já aplicado ao documento fora do
    /// histórico (ex.: imprint de uma região numa face). `before` é o estado
    /// anterior ao prelúdio: confirmar grava uma única entrada de Undo a partir
    /// dele; cancelar (ou confirmar sem mudança) volta exatamente a ele. Em
    /// erro, o documento também volta a `before`.
    pub fn begin_modal_after_prelude(
        &mut self,
        kind: ModalKind,
        before: Project,
        before_selection: Selection,
    ) -> Result<(), ModalError> {
        match self.begin_modal(kind) {
            Ok(()) => {
                if let Some(modal) = self.modal.as_mut() {
                    modal.before_prelude = Some(Box::new((before, before_selection)));
                }
                Ok(())
            }
            Err(error) => {
                if !matches!(error, ModalError::AlreadyActive) {
                    self.restore_before_prelude(before, before_selection);
                }
                Err(error)
            }
        }
    }

    pub(crate) fn restore_before_prelude(&mut self, before: Project, selection: Selection) {
        let revision_clock = self.document.project.revision_clock();
        self.document.project = before;
        self.document
            .project
            .rebase_revisions_after_restore(revision_clock);
        self.editor.selection = selection;
        self.events.emit(crate::AppEvent::SelectionChanged(
            self.editor.selection.clone(),
        ));
        self.emit_project_changed(ProjectChanges::ALL);
    }

    /// Marca a operação ativa como extrusão de uma folha de região solta.
    pub(crate) fn set_modal_flip_when_negative(&mut self) {
        if let Some(modal) = self.modal.as_mut() {
            modal.flip_when_negative = true;
        }
    }

    pub fn set_modal_constraint(&mut self, constraint: ModalConstraint) -> Result<(), ModalError> {
        if matches!(constraint, ModalConstraint::Axis(i) | ModalConstraint::Plane(i) if i > 2) {
            return Err(ModalError::InvalidInput);
        }
        let modal = self.modal.as_mut().ok_or(ModalError::NoActiveOperation)?;
        modal.constraint = constraint;
        self.locked_axes = match constraint {
            ModalConstraint::Axis(0) => [true, false, false],
            ModalConstraint::Axis(1) => [false, true, false],
            ModalConstraint::Axis(2) => [false, false, true],
            ModalConstraint::Plane(0) => [false, true, true],
            ModalConstraint::Plane(1) => [true, false, true],
            ModalConstraint::Plane(2) => [true, true, false],
            _ => [false, false, false],
        };
        self.mark_dirty();
        Ok(())
    }

    /// `translation` is an absolute world-space delta from the start, `value`
    /// is the absolute scalar (including exact numeric input), never a frame delta.
    /// The core never snaps: snapping is one screen-space pass by the caller
    /// ([`crate::inference`]), reported through [`AppState::update_modal_snapped`].
    pub fn update_modal(&mut self, translation: Vec3, value: f32) -> Result<(), ModalError> {
        self.update_modal_snapped(translation, value, None)
    }

    /// Same as [`AppState::update_modal`], recording which snap target the caller
    /// already applied to `translation` (P3D-040: one snap pass per gesture).
    pub fn update_modal_snapped(
        &mut self,
        translation: Vec3,
        value: f32,
        snap: Option<SnapKind>,
    ) -> Result<(), ModalError> {
        if let Some(result) = self.update_modal_fast(translation, value, snap) {
            return result;
        }
        let result = self.update_modal_slow(translation, value, snap);
        if result.is_ok() {
            self.record_modal_preview(value);
        }
        result
    }

    /// Caminho incremental. `None` = usar a prévia exata.
    pub(crate) fn update_modal_fast(
        &mut self,
        translation: Vec3,
        value: f32,
        snap: Option<SnapKind>,
    ) -> Option<Result<(), ModalError>> {
        if !translation.is_finite() || !value.is_finite() || value.abs() > 1.0e6 {
            return None;
        }
        let proportional = self.proportional_editing;
        let modal = self.editor.tools.modal.as_mut()?;
        if modal.edit_origin || modal.object_mode {
            return None;
        }
        let direction = AppState::modal_direction(modal);
        let to_world = modal.object_to_world;
        let to_local = to_world.inverse();
        let mesh = &mut self.document.active_mut()?.mesh;
        let changes = match modal.kind {
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale | ModalKind::PushPull => {
                if !modal.preview.source_topology_live
                    || (proportional && modal.kind != ModalKind::PushPull)
                    || mesh.verts.len() != modal.source.verts.len()
                    || (modal.kind == ModalKind::Scale && value.abs() < 1.0e-6)
                {
                    return None;
                }
                let source = &modal.source;
                let affected = modal.preview.affected.get_or_insert_with(|| {
                    source
                        .verts
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.selected)
                        .map(|(i, _)| i as u32)
                        .collect()
                });
                let pivots = modal.pivots.as_deref();
                let pivot_of = |i: usize| pivots.map_or(modal.pivot, |p| p[i]);
                let (components, transform): (Vec3, Box<dyn Fn(usize, Vec3) -> Vec3 + '_>) =
                    match modal.kind {
                        ModalKind::Move => {
                            let delta = match modal.constraint {
                                ModalConstraint::Free => translation,
                                ModalConstraint::Axis(i) => {
                                    let a = modal.frame.axis(i);
                                    if snap.is_some() {
                                        a * translation.dot(a)
                                    } else {
                                        a * value
                                    }
                                }
                                ModalConstraint::Plane(i) => {
                                    let a = modal.frame.axis(i);
                                    translation - a * translation.dot(a)
                                }
                            };
                            (delta, Box::new(move |_, p| p + delta))
                        }
                        ModalKind::PushPull => {
                            let delta = direction * value;
                            (Vec3::ZERO, Box::new(move |_, p| p + delta))
                        }
                        ModalKind::Rotate => {
                            let normal = match modal.constraint {
                                ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => {
                                    modal.frame.axis(i)
                                }
                                ModalConstraint::Free => modal.normal,
                            };
                            let rotation = Quat::from_axis_angle(normal, value.to_radians());
                            let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
                            (
                                Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
                                Box::new(move |i, p| {
                                    let pivot = pivot_of(i);
                                    pivot + rotation * (p - pivot)
                                }),
                            )
                        }
                        _ => {
                            let frame = modal.frame;
                            let constraint = modal.constraint;
                            let factors = match constraint {
                                ModalConstraint::Free => Vec3::splat(value),
                                ModalConstraint::Axis(i) => {
                                    Vec3::ONE + frame.axis(i) * (value - 1.0)
                                }
                                ModalConstraint::Plane(i) => {
                                    Vec3::splat(value) + frame.axis(i) * (1.0 - value)
                                }
                            };
                            (
                                factors,
                                Box::new(move |i, p| {
                                    let pivot = pivot_of(i);
                                    let v = p - pivot;
                                    match constraint {
                                        ModalConstraint::Free => pivot + v * value,
                                        ModalConstraint::Axis(ax) => {
                                            let a = frame.axis(ax);
                                            let v_parallel = a * v.dot(a);
                                            let v_perp = v - v_parallel;
                                            pivot + v_perp + v_parallel * value
                                        }
                                        ModalConstraint::Plane(pl) => {
                                            let a = frame.axis(pl);
                                            let v_parallel = a * v.dot(a);
                                            let v_perp = v - v_parallel;
                                            pivot + v_parallel + v_perp * value
                                        }
                                    }
                                }),
                            )
                        }
                    };
                let mut changed = false;
                for &index in affected.iter() {
                    let index = index as usize;
                    let base = source.verts[index].vec();
                    let next = to_local
                        .transform_point3(transform(index, to_world.transform_point3(base)));
                    if !next.is_finite() {
                        return Some(Err(ModalError::InvalidMesh));
                    }
                    changed |= next != base;
                    mesh.verts[index].pos = next.to_array();
                }
                modal.value = value;
                modal.components = components;
                modal.changed = changed;
                if modal.kind == ModalKind::PushPull {
                    ProjectChanges::GEOMETRY
                } else {
                    ProjectChanges::POSITIONS
                }
            }
            ModalKind::Extrude => {
                let flip = modal.flip_when_negative && value < 0.0;
                let extrude = modal.preview.extrude.as_ref()?;
                if value == 0.0
                    || extrude.live_flip != Some(flip)
                    || mesh.verts.len() != extrude.base.verts.len()
                {
                    return None;
                }
                let local_direction = to_local.transform_vector3(direction);
                for &index in &extrude.cap {
                    let index = index as usize;
                    let next = extrude.base.verts[index].vec() + local_direction * value;
                    if !next.is_finite() {
                        return Some(Err(ModalError::InvalidMesh));
                    }
                    mesh.verts[index].pos = next.to_array();
                }
                modal.value = value;
                modal.changed = true;
                ProjectChanges::GEOMETRY
            }
            ModalKind::Inset | ModalKind::Bevel | ModalKind::ExtrudeIndividual => {
                let linear = &mut modal.preview.linear;
                let (reference, vertices) = linear.model.as_ref()?;
                let span = (linear.range.1 - linear.range.0).abs().max(1.0e-3);
                if !linear.validated
                    || linear.disabled
                    || value == 0.0
                    || linear.since_check >= LINEAR_RECHECK_EVERY
                    || value < linear.range.0 - span * 0.5
                    || value > linear.range.1 + span * 0.5
                    || (modal.kind == ModalKind::Inset && value < 0.0)
                    || (modal.kind == ModalKind::Bevel && value < 0.0)
                    || linear
                        .samples
                        .last()
                        .is_none_or(|(_, sample)| sample.verts.len() != mesh.verts.len())
                {
                    return None;
                }
                let offset = value - reference;
                for &(index, at_reference, slope) in vertices {
                    let next = at_reference + slope * offset;
                    if !next.is_finite() {
                        return None;
                    }
                    mesh.verts[index as usize].pos = next.to_array();
                }
                linear.predicted = true;
                linear.since_check += 1;
                modal.value = value;
                modal.changed = true;
                ProjectChanges::GEOMETRY
            }
        };
        modal.snap = snap;
        self.emit_project_changed(changes);
        let rev = self.document.project.revision_id();
        if let Some(m) = self.editor.tools.modal.as_mut() {
            m.published_revision = rev;
        }
        Some(Ok(()))
    }

    /// Depois de uma prévia exata: prepara os caches do caminho incremental.
    pub(crate) fn record_modal_preview(&mut self, value: f32) {
        // Object frames have no geometry model to fit. Legacy baked objects
        // keep their rigid cache so pointer updates do not retriangulate them.
        if self.frame_modal() {
            return;
        }
        let proportional = self.proportional_editing;
        if let Some(modal) = self.editor.tools.modal.as_mut() {
            modal.preview.rigid.clear();
        }
        let published = self.document.active().map(|asset| &*asset.mesh);
        let Some(modal) = self.editor.tools.modal.as_mut() else {
            return;
        };
        match modal.kind {
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale | ModalKind::PushPull => {
                // Com edição proporcional a prévia exata desloca vizinhos que o
                // caminho incremental não reescreve.
                modal.preview.source_topology_live =
                    !(proportional && modal.kind != ModalKind::PushPull);
            }
            ModalKind::Extrude => {
                if value == 0.0 {
                    if let Some(extrude) = modal.preview.extrude.as_mut() {
                        extrude.live_flip = None;
                    }
                    return;
                }
                let flip = modal.flip_when_negative && value < 0.0;
                let extrude = modal.preview.extrude.get_or_insert_with(|| {
                    let mut base = modal.source.clone();
                    base.extrude_selected(0.0);
                    let cap = base
                        .verts
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.selected)
                        .map(|(i, _)| i as u32)
                        .collect();
                    ExtrudePreview {
                        base,
                        cap,
                        live_flip: None,
                    }
                });
                extrude.live_flip = Some(flip);
            }
            ModalKind::Inset | ModalKind::Bevel | ModalKind::ExtrudeIndividual => {
                let Some(published) = published else {
                    return;
                };
                let linear = &mut modal.preview.linear;
                linear.predicted = false;
                linear.since_check = 0;
                if linear.disabled || value == 0.0 {
                    return;
                }
                let same_topology = |a: &Mesh, b: &Mesh| {
                    a.verts.len() == b.verts.len()
                        && a.faces.len() == b.faces.len()
                        && a.faces
                            .iter()
                            .zip(&b.faces)
                            .all(|(x, y)| x.verts == y.verts)
                };
                if let Some((reference, vertices)) = linear.model.as_ref() {
                    // Reverificação: a operação exata precisa bater com o modelo.
                    let offset = value - reference;
                    let scale = published
                        .verts
                        .iter()
                        .map(|v| v.vec().abs().max_element())
                        .fold(1.0_f32, f32::max);
                    let fits = linear
                        .samples
                        .last()
                        .is_some_and(|(_, sample)| same_topology(sample, published))
                        && vertices.iter().all(|&(index, at_reference, slope)| {
                            published.verts.get(index as usize).is_some_and(|v| {
                                v.vec().distance(at_reference + slope * offset) <= 1.0e-4 * scale
                            })
                        });
                    if fits {
                        linear.validated = true;
                        linear.range = (linear.range.0.min(value), linear.range.1.max(value));
                    } else {
                        linear.disabled = true;
                        linear.model = None;
                    }
                    return;
                }
                if let Some((first_value, first)) = linear.samples.first()
                    && (value - first_value).abs() > 1.0e-6
                    && same_topology(first, published)
                {
                    let delta = value - first_value;
                    let vertices = first
                        .verts
                        .iter()
                        .zip(&published.verts)
                        .enumerate()
                        .filter_map(|(index, (a, b))| {
                            let (a, b) = (a.vec(), b.vec());
                            (a != b).then(|| (index as u32, b, (b - a) / delta))
                        })
                        .collect();
                    linear.model = Some((value, vertices));
                    linear.range = (value.min(*first_value), value.max(*first_value));
                }
                linear.samples.clear();
                linear.samples.push((value, published.clone()));
            }
        }
    }

    /// A prévia exata, usada no primeiro evento e quando a topologia muda.
    pub(crate) fn update_modal_slow(
        &mut self,
        translation: Vec3,
        value: f32,
        snap: Option<SnapKind>,
    ) -> Result<(), ModalError> {
        if !translation.is_finite() || !value.is_finite() || value.abs() > 1.0e6 {
            return Err(ModalError::InvalidInput);
        }
        let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
        if modal.kind == ModalKind::Scale && value.abs() < 1.0e-6 {
            return Err(ModalError::InvalidInput);
        }
        if modal.kind == ModalKind::Inset && value < 0.0 {
            return Err(ModalError::InvalidInput);
        }
        if modal.kind == ModalKind::Bevel && value < 0.0 {
            return Err(ModalError::InvalidInput);
        }
        if self.frame_modal() {
            let components = frame_components(modal, translation, value, snap)?;
            return self.publish_frame_modal(components, value, snap);
        }
        // A operação roda em mundo (pivô, eixos e distâncias do gesto) e volta
        // ao frame local antes de publicar.
        let to_world = modal.object_to_world;
        let mut mesh = modal.source.clone();
        if to_world != glam::Mat4::IDENTITY {
            mesh.transform_affine(to_world);
        }
        let mut components = Vec3::ZERO;
        let direction = match modal.constraint {
            ModalConstraint::Axis(i) => modal.frame.axis(i),
            ModalConstraint::Plane(i) => {
                let a = modal.frame.axis(i);
                (modal.normal - a * modal.normal.dot(a)).normalize_or_zero()
            }
            ModalConstraint::Free => modal.normal,
        };
        let use_proportional = self.proportional_editing
            && matches!(
                modal.kind,
                ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
            );
        match modal.kind {
            ModalKind::Move => {
                let delta = match modal.constraint {
                    ModalConstraint::Free => translation,
                    // A translação só vence o valor quando houve um encaixe
                    // de fato (snap ligado sem alvo usa o valor projetado).
                    ModalConstraint::Axis(i) => {
                        let a = modal.frame.axis(i);
                        if snap.is_some() {
                            a * translation.dot(a)
                        } else {
                            a * value
                        }
                    }
                    ModalConstraint::Plane(i) => {
                        let a = modal.frame.axis(i);
                        translation - a * translation.dot(a)
                    }
                };
                components = delta;
                if use_proportional {
                    for vertex in &mut mesh.verts {
                        if vertex.selected {
                            vertex.pos = (vertex.vec() + delta).to_array();
                        } else {
                            let dist = (vertex.vec() - modal.pivot).length();
                            let weight = crate::proportional::calculate_falloff_weight(
                                dist,
                                self.editor.proportional_settings.radius,
                                self.editor.proportional_settings.falloff,
                            );
                            if weight > 0.0 {
                                vertex.pos = (vertex.vec() + delta * weight).to_array();
                            }
                        }
                    }
                } else {
                    mesh.translate_selected(delta.to_array());
                }
            }
            ModalKind::Rotate => {
                let normal = match modal.constraint {
                    ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => modal.frame.axis(i),
                    ModalConstraint::Free => modal.normal,
                };
                let rotation = Quat::from_axis_angle(normal, value.to_radians());
                let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
                components = Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees());
                for (index, vertex) in mesh.verts.iter_mut().enumerate() {
                    let pivot = modal
                        .pivots
                        .as_ref()
                        .map_or(modal.pivot, |pivots| pivots[index]);
                    if vertex.selected {
                        vertex.pos = (pivot + rotation * (vertex.vec() - pivot)).to_array();
                    } else if use_proportional {
                        let dist = (vertex.vec() - modal.pivot).length();
                        let weight = crate::proportional::calculate_falloff_weight(
                            dist,
                            self.editor.proportional_settings.radius,
                            self.editor.proportional_settings.falloff,
                        );
                        if weight > 0.0 {
                            let rotated = pivot + rotation * (vertex.vec() - pivot);
                            vertex.pos = vertex.vec().lerp(rotated, weight).to_array();
                        }
                    }
                }
            }
            ModalKind::Scale => {
                let frame = modal.frame;
                let constraint = modal.constraint;
                let factors = match constraint {
                    ModalConstraint::Free => Vec3::splat(value),
                    ModalConstraint::Axis(i) => Vec3::ONE + frame.axis(i) * (value - 1.0),
                    ModalConstraint::Plane(i) => Vec3::splat(value) + frame.axis(i) * (1.0 - value),
                };
                components = factors;
                for (index, vertex) in mesh.verts.iter_mut().enumerate() {
                    let pivot = modal
                        .pivots
                        .as_ref()
                        .map_or(modal.pivot, |pivots| pivots[index]);
                    let v = vertex.vec() - pivot;
                    let target = match constraint {
                        ModalConstraint::Free => pivot + v * value,
                        ModalConstraint::Axis(ax) => {
                            let a = frame.axis(ax);
                            let v_parallel = a * v.dot(a);
                            let v_perp = v - v_parallel;
                            pivot + v_perp + v_parallel * value
                        }
                        ModalConstraint::Plane(pl) => {
                            let a = frame.axis(pl);
                            let v_parallel = a * v.dot(a);
                            let v_perp = v - v_parallel;
                            pivot + v_parallel + v_perp * value
                        }
                    };
                    if vertex.selected {
                        vertex.pos = target.to_array();
                    } else if use_proportional {
                        let dist = (vertex.vec() - modal.pivot).length();
                        let weight = crate::proportional::calculate_falloff_weight(
                            dist,
                            self.editor.proportional_settings.radius,
                            self.editor.proportional_settings.falloff,
                        );
                        if weight > 0.0 {
                            vertex.pos = vertex.vec().lerp(target, weight).to_array();
                        }
                    }
                }
            }
            ModalKind::Extrude if value != 0.0 => {
                // Build topology once per absolute preview, then constrain the new cap.
                mesh.extrude_selected(0.0);
                for vertex in mesh.verts.iter_mut().filter(|v| v.selected) {
                    vertex.pos = (vertex.vec() + direction * value).to_array();
                }
                if modal.flip_when_negative && value < 0.0 {
                    for face in &mut mesh.faces {
                        face.verts.reverse();
                        face.uv.reverse();
                    }
                }
            }
            ModalKind::ExtrudeIndividual if value != 0.0 => mesh.extrude_individual(value),
            // Inset métrico (unidades de mundo) com limite anti-interseção.
            ModalKind::Inset if value != 0.0 => {
                let faces: Vec<usize> = mesh
                    .faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.selected)
                    .map(|(i, _)| i)
                    .collect();
                mesh.inset_faces(&faces, value, self.tools.inset_mode);
            }
            ModalKind::Bevel if value != 0.0 => {
                let segs = self.tools.bevel_segments.clamp(1, 8);
                let clamp = self.tools.bevel_clamp_overlap;
                let affect_verts = self.tools.bevel_affect_vertices;
                let has_verts = mesh.verts.iter().any(|v| v.selected);
                let (applied, skipped) = if affect_verts && has_verts {
                    mesh.bevel_selected_vertex(value, clamp)
                } else {
                    mesh.bevel_selected_full(value, segs, clamp)
                };
                if applied == 0 || skipped > 0 {
                    return Err(ModalError::UnsupportedTopology);
                }
            }
            ModalKind::PushPull => mesh.translate_selected((direction * value).to_array()),
            _ => {}
        }
        if to_world != glam::Mat4::IDENTITY {
            mesh.transform_affine(to_world.inverse());
        }
        self.publish_modal_mesh(mesh, value, components)?;
        if let Some(modal) = self.modal.as_mut() {
            modal.snap = snap;
        }
        Ok(())
    }

    /// Absolute XYZ property fields; rotation uses Euler XYZ angles in degrees.
    /// Each call rebuilds from the same transaction snapshot, avoiding drift.
    pub fn update_modal_components(&mut self, components: Vec3) -> Result<(), ModalError> {
        if !components.is_finite() || components.abs().max_element() > 1.0e6 {
            return Err(ModalError::InvalidInput);
        }
        let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
        if !matches!(
            modal.kind,
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
        ) || (modal.kind == ModalKind::Scale && components.abs().min_element() < 1.0e-6)
        {
            return Err(ModalError::InvalidInput);
        }
        let value = if modal.kind == ModalKind::Scale {
            components.x
        } else {
            components.length()
        };
        if self.frame_modal() {
            return self.publish_frame_modal(components, value, None);
        }
        let mut mesh = modal.source.clone();
        let to_world = modal.object_to_world;
        if to_world != glam::Mat4::IDENTITY {
            mesh.transform_affine(to_world);
        }
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            components.x.to_radians(),
            components.y.to_radians(),
            components.z.to_radians(),
        );
        for (index, vertex) in mesh
            .verts
            .iter_mut()
            .enumerate()
            .filter(|(_, v)| v.selected)
        {
            let pivot = modal
                .pivots
                .as_ref()
                .map_or(modal.pivot, |pivots| pivots[index]);
            vertex.pos = match modal.kind {
                ModalKind::Move => vertex.vec() + components,
                ModalKind::Rotate => pivot + rotation * (vertex.vec() - pivot),
                ModalKind::Scale => pivot + components * (vertex.vec() - pivot),
                _ => return Err(ModalError::InvalidInput),
            }
            .to_array();
        }
        let value = if modal.kind == ModalKind::Scale {
            components.x
        } else {
            components.length()
        };
        if to_world != glam::Mat4::IDENTITY {
            mesh.transform_affine(to_world.inverse());
        }
        self.publish_modal_mesh(mesh, value, components)?;
        if let Some(modal) = self.modal.as_mut() {
            modal.constraint = ModalConstraint::Free;
        }
        Ok(())
    }

    pub(crate) fn publish_modal_mesh(
        &mut self,
        mut mesh: Mesh,
        value: f32,
        components: Vec3,
    ) -> Result<(), ModalError> {
        if !valid_mesh(&mesh) {
            return Err(ModalError::InvalidMesh);
        }
        let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
        let changed = !same_geometry(&mesh, &modal.source);
        let kind = modal.kind;
        if !changed && let Some(original) = modal.original.active_mesh() {
            for (v, source) in mesh.verts.iter_mut().zip(&original.verts) {
                v.selected = source.selected;
            }
            for (face, source) in mesh.faces.iter_mut().zip(&original.faces) {
                face.selected = source.selected;
            }
            mesh.selected_edges = original.selected_edges.clone();
        }
        self.document
            .active_mut()
            .ok_or(ModalError::NoActiveMesh)?
            .mesh = mesh.into();
        if let Some(modal) = self.modal.as_mut() {
            modal.value = value;
            modal.components = components;
            modal.changed = changed;
        }
        self.sync_selection();
        self.emit_project_changed(match kind {
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale => ProjectChanges::POSITIONS,
            _ => ProjectChanges::GEOMETRY,
        });
        let current_rev = self.document.project.revision_id();
        if let Some(modal) = self.modal.as_mut() {
            modal.published_revision = current_rev;
        }
        Ok(())
    }

    pub fn commit_modal(&mut self) -> bool {
        if let Some(modal) = self.modal.as_ref()
            && self.document.project.revision_id() != modal.published_revision
        {
            self.cancel_modal();
            return false;
        }
        // O resultado confirmado é sempre o da operação exata.
        if let Some((value, components, snap)) = self
            .modal
            .as_ref()
            .filter(|modal| modal.preview.linear.predicted)
            .map(|modal| (modal.value, modal.components, modal.snap))
        {
            let _ = self.update_modal_slow(components, value, snap);
        }
        let Some(modal) = self.modal.take() else {
            return false;
        };
        let mut restored_changes = None;
        if let Some(before) = modal.before_prelude {
            let (before, selection) = *before;
            if modal.changed {
                self.document.undo.checkpoint_sized(
                    modal.kind.label(),
                    &before,
                    before.estimated_bytes(),
                );
                self.mark_document_dirty();
            } else {
                self.restore_before_prelude(before, selection);
            }
        } else if modal.changed {
            self.document.undo.checkpoint_sized(
                modal.kind.label(),
                &modal.original,
                modal.original.estimated_bytes(),
            );
            self.mark_document_dirty();
        } else {
            let revision_clock = self.document.project.revision_clock();
            let previewed = revision_clock != modal.original.revision_clock();
            self.document.project = modal.original;
            self.editor.selection = modal.selection;
            if previewed {
                self.document
                    .project
                    .rebase_revisions_after_restore(revision_clock);
                restored_changes = Some(match modal.kind {
                    ModalKind::Move | ModalKind::Rotate | ModalKind::Scale => {
                        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
                    }
                    _ => ProjectChanges::GEOMETRY,
                });
            }
        }
        self.pointer_session = None;
        self.pending_modal = None;
        self.locked_axes = [false; 3];
        if let Some(changes) = restored_changes {
            self.emit_project_changed(changes);
        }
        true
    }

    pub fn cancel_modal(&mut self) -> bool {
        self.pointer_session = None;
        self.pending_modal = None;
        let Some(modal) = self.modal.take() else {
            return false;
        };
        if let Some(before) = modal.before_prelude {
            let (before, selection) = *before;
            self.restore_before_prelude(before, selection);
            self.locked_axes = [false; 3];
            return true;
        }
        let revision_clock = self.document.project.revision_clock();
        self.document.project = modal.original;
        self.document
            .project
            .rebase_revisions_after_restore(revision_clock);
        self.editor.selection = modal.selection;
        self.locked_axes = [false; 3];
        self.events.emit(crate::AppEvent::SelectionChanged(
            self.editor.selection.clone(),
        ));
        let changes = match modal.kind {
            ModalKind::Move | ModalKind::Rotate | ModalKind::Scale => {
                ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
            }
            _ => ProjectChanges::GEOMETRY,
        };
        self.emit_project_changed(changes);
        true
    }
    fn frame_modal(&self) -> bool {
        self.modal.as_ref().is_some_and(|m| {
            (m.edit_origin || m.object_mode)
                && matches!(
                    m.kind,
                    ModalKind::Move | ModalKind::Rotate | ModalKind::Scale
                )
        })
    }

    /// Compute all targets from the frozen snapshot before publishing any of
    /// them. An unsupported TRS (shear) retains the last valid preview.
    fn publish_frame_modal(
        &mut self,
        components: Vec3,
        value: f32,
        snap: Option<SnapKind>,
    ) -> Result<(), ModalError> {
        let modal = self.modal.as_ref().ok_or(ModalError::NoActiveOperation)?;
        let active = modal.original.active().ok_or(ModalError::NoActiveMesh)?;
        if self.document.active().map(|a| a.id) != Some(active.id) {
            return Err(ModalError::NoActiveMesh);
        }
        let edit_origin = modal.edit_origin;
        let mut changed = false;
        let mut legacy_geometry = false;
        let mut rigid = Vec::new();
        let mut updates = Vec::new();
        for source in modal.original.assets.iter().filter(|a| {
            !a.locked
                && (a.id == active.id || (!edit_origin && modal.selection.assets.contains(&a.id)))
        }) {
            if !self
                .document
                .assets
                .iter()
                .any(|a| a.id == source.id && !a.locked)
            {
                return Err(ModalError::ActiveLocked);
            }
            let mut next = source.clone();
            if edit_origin {
                if components != Vec3::ZERO {
                    next.rebase_origin_world(source.origin_world() + components)
                        .map_err(|_| ModalError::UnsupportedFrame)?;
                }
                changed |= next.transform != source.transform || next.origin != source.origin;
            } else {
                let pivot = if modal.individual_origins {
                    source.origin_world()
                } else {
                    modal.pivot
                };
                let matrix = frame_matrix(modal.kind, components, pivot);
                if source.has_legacy_baked_transform() {
                    changed |=
                        legacy_object_preview(source, &mut next, modal.kind, components, matrix)?;
                    legacy_geometry = true;
                    if !source.has_enabled_modifiers() {
                        rigid.push((source.id, matrix));
                    }
                    updates.push(next);
                    continue;
                }
                let transform = if matrix == glam::Mat4::IDENTITY {
                    source.transform
                } else {
                    petunia_project::ObjectTransform::from_mat4(matrix * source.world_matrix())
                        .map_err(|_| ModalError::UnsupportedFrame)?
                };
                let matrix = transform.to_mat4();
                if !transform.is_finite()
                    || source
                        .mesh
                        .verts
                        .iter()
                        .any(|v| !matrix.transform_point3(v.vec()).is_finite())
                {
                    return Err(ModalError::InvalidInput);
                }
                changed |= transform != source.transform;
                next.set_object_transform(transform);
            }
            updates.push(next);
        }
        for next in updates {
            if let Some(asset) = self.document.assets.iter_mut().find(|a| a.id == next.id) {
                *asset = next;
            }
        }
        if let Some(modal) = self.modal.as_mut() {
            modal.components = components;
            modal.value = value;
            modal.changed = changed;
            modal.snap = snap;
            // Canonical objects use their model matrix directly. Only baked
            // compatibility objects require the relative rigid cache.
            modal.preview.rigid = rigid;
        }
        self.emit_project_changed(if edit_origin || legacy_geometry {
            ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
        } else {
            ProjectChanges::TRANSFORMS
        });
        let current_rev = self.document.project.revision_id();
        if let Some(modal) = self.modal.as_mut() {
            modal.published_revision = current_rev;
        }
        Ok(())
    }
}

impl<'a> EditorView<'a> {
    pub(crate) fn modal_flips_when_negative(self) -> bool {
        self.modal
            .as_ref()
            .is_some_and(|modal| modal.flip_when_negative)
    }

    /// Estado posterior ao prelúdio (para reaplicar a "Última operação").
    pub(crate) fn modal_prelude_state(self) -> Option<(Project, Selection)> {
        let modal = self.modal.as_ref()?;
        modal
            .before_prelude
            .as_ref()
            .map(|_| (modal.original.clone(), modal.selection.clone()))
    }

    /// Matrizes rígidas (original → prévia) dos objetos em transformação no
    /// modo objeto. Vazio fora desse caso; o renderer as usa para não
    /// triangular de novo um objeto que só se move.
    pub fn rigid_preview_transforms(self) -> &'a [(uuid::Uuid, glam::Mat4)] {
        self.editor
            .tools
            .modal
            .as_ref()
            .map_or(&[], |modal| modal.preview.rigid.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_face() -> AppState {
        let mut state = AppState::new("en");
        state.set_edit_mode(EditMode::Edit);
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.deselect_all();
        mesh.faces[0].selected = true;
        mesh.sync_vert_selection_from_faces();
        state.sync_selection();
        state
    }

    // Compare every mesh attribute, including UVs, colors and selection flags.
    fn assert_exact_mesh(actual: &Mesh, expected: &Mesh) {
        assert!(same_geometry(actual, expected));
        assert_eq!(actual.selected_edges, expected.selected_edges);
        assert_eq!(
            actual.verts.iter().map(|v| v.selected).collect::<Vec<_>>(),
            expected
                .verts
                .iter()
                .map(|v| v.selected)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            actual.faces.iter().map(|f| f.selected).collect::<Vec<_>>(),
            expected
                .faces
                .iter()
                .map(|f| f.selected)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn numeric_xyz_fields_rebuild_absolute_transform_and_commit_once() {
        for (kind, values) in [
            (ModalKind::Move, Vec3::new(1.25, -2.0, 0.5)),
            (ModalKind::Rotate, Vec3::new(90.0, 0.0, 0.0)),
            (ModalKind::Scale, Vec3::new(2.0, 0.5, -1.0)),
        ] {
            let mut state = selected_face();
            let original = state.project.active_mesh().unwrap().clone();
            state.begin_modal(kind).unwrap();
            let pivot = state.modal.as_ref().unwrap().pivot;
            state.update_modal_components(Vec3::splat(0.25)).unwrap();
            state.update_modal_components(values).unwrap();
            for (before, after) in original
                .verts
                .iter()
                .zip(&state.project.active_mesh().unwrap().verts)
            {
                let relative = before.vec() - pivot;
                let expected = if before.selected {
                    match kind {
                        ModalKind::Move => before.vec() + values,
                        ModalKind::Rotate => pivot + Vec3::new(relative.x, -relative.z, relative.y),
                        ModalKind::Scale => pivot + relative * values,
                        _ => unreachable!(),
                    }
                } else {
                    before.vec()
                };
                assert!((after.vec() - expected).length() < 1.0e-5, "{kind:?}");
            }
            assert_eq!(state.modal.as_ref().unwrap().components, values);
            assert_eq!(state.project.undo.depth(), (0, 0));
            state.commit_modal();
            assert_eq!(state.project.undo.depth(), (1, 0));
            state.undo();
            assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        }
    }

    #[test]
    fn invalid_numeric_axis_preserves_last_valid_preview() {
        let mut state = selected_face();
        state.begin_modal(ModalKind::Scale).unwrap();
        state
            .update_modal_components(Vec3::new(1.0, 2.0, 3.0))
            .unwrap();
        let preview = state.project.active_mesh().unwrap().clone();
        for invalid in [
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::splat(f32::NAN),
            Vec3::splat(1.0e7),
        ] {
            assert_eq!(
                state.update_modal_components(invalid),
                Err(ModalError::InvalidInput)
            );
            assert_exact_mesh(state.project.active_mesh().unwrap(), &preview);
        }
        assert_eq!(state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn cancel_restores_every_operation_without_touching_history() {
        for kind in [
            ModalKind::Move,
            ModalKind::Rotate,
            ModalKind::Scale,
            ModalKind::Extrude,
            ModalKind::Inset,
            ModalKind::Bevel,
            ModalKind::PushPull,
        ] {
            let mut state = selected_face();
            if kind == ModalKind::Bevel {
                state
                    .project
                    .active_mesh_mut()
                    .unwrap()
                    .selected_edges
                    .insert((0, 1));
            }
            state.checkpoint("prior");
            let before = state.project.clone();
            state.begin_modal(kind).unwrap();
            state.update_modal(Vec3::new(0.4, 0.2, 0.0), 0.25).unwrap();
            state.update_modal(Vec3::new(0.8, 0.4, 0.0), 0.5).unwrap();
            assert!(state.cancel_modal());
            assert_eq!(state.project.undo.depth(), (1, 0), "{kind:?}");
            assert_exact_mesh(
                state.project.active_mesh().unwrap(),
                before.active_mesh().unwrap(),
            );
        }
    }

    #[test]
    fn modal_bevel_supports_single_vertex_selection() {
        let mut state = AppState::default();
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.selected_edges.clear();
        for f in &mut mesh.faces {
            f.selected = false;
        }
        for v in &mut mesh.verts {
            v.selected = false;
        }
        mesh.verts[0].selected = true;

        assert!(state.begin_modal(ModalKind::Bevel).is_ok());
        assert!(state.update_modal(Vec3::ZERO, 0.2).is_ok());
        assert!(state.commit_modal());

        let result = state.project.active_mesh().unwrap();
        let report = result.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((result.verts.len(), result.faces.len()), (10, 7));
    }

    #[test]
    fn modal_bevel_supports_multiple_vertices_batch() {
        let mut state = AppState::default();
        let mesh = state.project.active_mesh_mut().unwrap();
        mesh.selected_edges.clear();
        for f in &mut mesh.faces {
            f.selected = false;
        }
        for v in &mut mesh.verts {
            v.selected = true; // todos os 8 vértices
        }

        assert!(state.begin_modal(ModalKind::Bevel).is_ok());
        assert!(state.update_modal(Vec3::ZERO, 0.2).is_ok());
        assert!(state.commit_modal());

        let result = state.project.active_mesh().unwrap();
        let report = result.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((result.verts.len(), result.faces.len()), (24, 14));
    }

    #[test]
    fn multiple_previews_commit_one_checkpoint_and_roundtrip() {
        let mut state = selected_face();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Move).unwrap();
        for value in [0.2, 0.3, 0.8] {
            state.update_modal(Vec3::X * value, value).unwrap();
            assert_eq!(state.project.undo.depth(), (0, 0));
        }
        let result = state.project.active_mesh().unwrap().clone();
        for (before, after) in original.verts.iter().zip(&result.verts) {
            let expected = before.vec()
                + if before.selected {
                    Vec3::X * 0.8
                } else {
                    Vec3::ZERO
                };
            assert!((after.vec() - expected).length() < 1.0e-6);
        }
        assert!(state.commit_modal());
        assert_eq!(state.project.undo.depth(), (1, 0));
        assert!(state.undo());
        assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        assert!(state.redo());
        assert_exact_mesh(state.project.active_mesh().unwrap(), &result);
    }

    #[test]
    fn topology_undo_and_redo_resynchronize_cached_selection() {
        let mut state = selected_face();
        let original_verts = state.selection.verts.clone();
        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 0.5).unwrap();
        let extruded_verts = state.selection.verts.clone();
        assert_ne!(extruded_verts, original_verts);
        state.commit_modal();
        state.undo();
        assert_eq!(state.selection.verts, original_verts);
        state.redo();
        assert_eq!(state.selection.verts, extruded_verts);
    }

    #[test]
    fn cancel_preserves_redo_and_undo_cancels_active_preview() {
        let mut state = selected_face();
        state.checkpoint("prior");
        state
            .project
            .active_mesh_mut()
            .unwrap()
            .translate_selected([1.0, 0.0, 0.0]);
        state.undo();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Extrude).unwrap();
        state.update_modal(Vec3::ZERO, 1.0).unwrap();
        assert!(state.undo());
        assert_eq!(state.project.undo.depth(), (0, 1));
        assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
    }

    #[test]
    fn identity_after_preview_does_not_create_history_or_geometry() {
        for kind in [
            ModalKind::Move,
            ModalKind::Extrude,
            ModalKind::Inset,
            ModalKind::Scale,
        ] {
            let mut state = selected_face();
            let original = state.project.active_mesh().unwrap().clone();
            state.begin_modal(kind).unwrap();
            state.update_modal(Vec3::X, 0.2).unwrap();
            state
                .update_modal(Vec3::ZERO, if kind == ModalKind::Scale { 1.0 } else { 0.0 })
                .unwrap();
            state.commit_modal();
            assert_eq!(state.project.undo.depth(), (0, 0));
            assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        }
    }

    /// Edit Mode num objeto transformado: o gesto é em mundo, a escrita é local
    /// (website/docs/03-geometry/transforms-origin-pivot-gizmos.md, "World-space
    /// interaction em Edit Mode").
    #[test]
    fn edit_mode_moves_are_world_deltas_on_transformed_objects() {
        let mut state = selected_face();
        state.project.active_mut().unwrap().transform = petunia_project::ObjectTransform::from_trs(
            Vec3::new(3.0, 0.0, 0.0),
            Quat::from_rotation_y(0.5),
            Vec3::splat(2.0),
        );
        let before = state.project.active().unwrap().world_positions();
        let selected: Vec<bool> = state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .map(|v| v.selected)
            .collect();
        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::X, 0.0).unwrap();
        let after = state.project.active().unwrap().world_positions();
        for ((b, a), moved) in before.iter().zip(&after).zip(&selected) {
            let expected = if *moved { *b + Vec3::X } else { *b };
            assert!(a.distance(expected) < 1e-4, "{a} != {expected}");
        }
        assert!(state.commit_modal());
    }

    #[test]
    fn axes_and_planes_constrain_absolute_translation() {
        let mut state = selected_face();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Move).unwrap();
        state
            .set_modal_constraint(ModalConstraint::Axis(2))
            .unwrap();
        state.update_modal(Vec3::splat(9.0), -1.25).unwrap();
        let vertex = original.verts.iter().position(|v| v.selected).unwrap();
        assert_eq!(
            state.project.active_mesh().unwrap().verts[vertex].vec(),
            original.verts[vertex].vec() - Vec3::Z * 1.25
        );
        state
            .set_modal_constraint(ModalConstraint::Plane(1))
            .unwrap();
        state.update_modal(Vec3::new(1.0, 2.0, 3.0), 0.0).unwrap();
        assert_eq!(
            state.project.active_mesh().unwrap().verts[vertex].vec(),
            original.verts[vertex].vec() + Vec3::new(1.0, 0.0, 3.0)
        );
    }

    #[test]
    fn rotation_and_plane_scale_preserve_pivot_and_constrained_coordinates() {
        let mut state = selected_face();
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Rotate).unwrap();
        let pivot = state.modal.as_ref().unwrap().pivot;
        state
            .set_modal_constraint(ModalConstraint::Axis(1))
            .unwrap();
        state.update_modal(Vec3::ZERO, 90.0).unwrap();
        for (before, after) in original
            .verts
            .iter()
            .zip(&state.project.active_mesh().unwrap().verts)
        {
            if before.selected {
                let relative = before.vec() - pivot;
                let expected = pivot + Vec3::new(relative.z, relative.y, -relative.x);
                assert!((after.vec() - expected).length() < 1.0e-5);
            } else {
                assert_eq!(after.pos, before.pos);
            }
        }
        state.cancel_modal();
        state.begin_modal(ModalKind::Scale).unwrap();
        state
            .set_modal_constraint(ModalConstraint::Plane(2))
            .unwrap();
        state.update_modal(Vec3::ZERO, 2.0).unwrap();
        for (before, after) in original
            .verts
            .iter()
            .zip(&state.project.active_mesh().unwrap().verts)
        {
            if before.selected {
                assert_eq!(
                    after.vec(),
                    pivot + (before.vec() - pivot) * Vec3::new(2.0, 2.0, 1.0)
                );
            } else {
                assert_eq!(after.pos, before.pos);
            }
        }
    }

    #[test]
    fn invalid_inputs_leave_last_preview_untouched() {
        let mut state = selected_face();
        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::X, 1.0).unwrap();
        let preview = state.project.active_mesh().unwrap().clone();
        assert_eq!(
            state.update_modal(Vec3::ZERO, f32::NAN),
            Err(ModalError::InvalidInput)
        );
        assert_eq!(
            state.update_modal(Vec3::splat(f32::INFINITY), 1.0),
            Err(ModalError::InvalidInput)
        );
        assert_eq!(
            state.set_modal_constraint(ModalConstraint::Axis(99)),
            Err(ModalError::InvalidInput)
        );
        assert_exact_mesh(state.project.active_mesh().unwrap(), &preview);
        assert_eq!(state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn corrupted_mesh_rejected_before_indexing() {
        let mut state = selected_face();
        state.project.active_mesh_mut().unwrap().faces[0].verts[0] = u32::MAX;
        assert_eq!(
            state.begin_modal(ModalKind::Extrude),
            Err(ModalError::InvalidMesh)
        );
        assert!(state.modal.is_none());
    }

    #[test]
    fn empty_selection_rejected_in_edit_mode_and_object_mode_moves_frame() {
        let mut state = AppState::new("en");
        state.project.active_mesh_mut().unwrap().deselect_all();
        state.set_edit_mode(EditMode::Edit);
        assert_eq!(
            state.begin_modal(ModalKind::Move),
            Err(ModalError::NoSelection)
        );
        state.set_edit_mode(EditMode::Object);
        let original = state.project.active_mesh().unwrap().clone();
        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::Y, 1.0).unwrap();
        state.commit_modal();
        assert_exact_mesh(state.project.active_mesh().unwrap(), &original);
        let actual = state.project.active().unwrap().world_positions();
        for (before, after) in original.verts.iter().zip(actual) {
            assert_eq!(after, before.vec() + Vec3::Y);
        }
        assert_eq!(
            state.project.active().unwrap().transform.translation,
            Vec3::Y.to_array()
        );
    }

    #[test]
    fn locked_asset_rejects_begin_modal() {
        let mut state = AppState::new("en");
        state.toggle_lock_active();
        assert!(state.is_active_locked());
        assert_eq!(
            state.begin_modal(ModalKind::Move),
            Err(ModalError::ActiveLocked)
        );
    }

    #[test]
    fn axis_lock_initialization_and_toggling() {
        let mut state = selected_face();

        // 1. Fora do modal: toggle e labels
        assert_eq!(state.locked_axes, [false; 3]);
        assert!(!state.is_axis_locked(0));
        assert!(state.active_axis_constraint_label().is_none());

        state.toggle_axis_lock(0);
        assert!(state.is_axis_locked(0));
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Eixo X", [235, 75, 75]))
        );

        // 2. Herança para o modal
        state.begin_modal(ModalKind::Move).unwrap();
        assert_eq!(
            state.modal.as_ref().map(|m| m.constraint),
            Some(ModalConstraint::Axis(0))
        );
        assert!(state.is_axis_locked(0));

        // 3. Alternância dentro do modal
        state.toggle_axis_lock(1); // ativa eixo Y
        assert_eq!(
            state.modal.as_ref().map(|m| m.constraint),
            Some(ModalConstraint::Axis(1))
        );
        assert!(!state.is_axis_locked(0));
        assert!(state.is_axis_locked(1));
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Eixo Y", [85, 195, 100]))
        );

        // 4. Commit reseta o travamento
        state.commit_modal();
        assert_eq!(state.locked_axes, [false; 3]);
        assert!(!state.is_axis_locked(1));
        assert!(state.active_axis_constraint_label().is_none());

        // 5. Cancelamento também reseta o travamento
        state.locked_axes = [true, false, true]; // Plano XZ pré-configurado
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Plano XZ", [142, 68, 173]))
        );
        state.begin_modal(ModalKind::Scale).unwrap();
        assert_eq!(
            state.modal.as_ref().map(|m| m.constraint),
            Some(ModalConstraint::Plane(1))
        );
        state.cancel_modal();
        assert_eq!(state.locked_axes, [false; 3]);
    }

    #[test]
    fn feedback_reports_snap_only_when_the_caller_snapped() {
        let mut state = selected_face();
        state.snap_enabled = true;
        state.session.snap_settings.enabled = true;
        state.begin_modal(ModalKind::Move).unwrap();
        let pivot = state.modal.as_ref().unwrap().pivot;
        let vertex = state.modal.as_ref().unwrap().source.verts[0].vec();

        // Snap ligado não encaixa nada no core: uma única passada, feita por
        // quem conhece a tela (P3D-040). O ponto sobre um vértice fica livre.
        state
            .update_modal(vertex - pivot, (vertex - pivot).length())
            .unwrap();
        let feedback = state.current_tool_feedback().unwrap();
        assert!(!feedback.is_snapped, "snap ligado não significa encaixe");
        assert_eq!(feedback.snap_kind, None);

        state
            .update_modal_snapped(
                vertex - pivot,
                (vertex - pivot).length(),
                Some(SnapKind::Point),
            )
            .unwrap();
        let feedback = state.current_tool_feedback().unwrap();
        assert!(feedback.is_snapped);
        assert_eq!(feedback.snap_kind, Some(SnapKind::Point));
        assert!((feedback.current - vertex).length() < 1.0e-4);

        // O próximo update sem encaixe limpa o rótulo.
        state.update_modal(Vec3::new(0.0, 0.0, 3.0), 3.0).unwrap();
        assert!(!state.current_tool_feedback().unwrap().is_snapped);
    }

    #[test]
    fn rotation_and_scale_have_no_world_point_or_guide_line() {
        let mut state = selected_face();
        state.begin_modal(ModalKind::Rotate).unwrap();
        state.update_modal(Vec3::ZERO, 45.0).unwrap();
        assert!(state.modal.as_ref().unwrap().current_point().is_none());
        let feedback = state.current_tool_feedback().unwrap();
        assert!(
            feedback.guide_line.is_none(),
            "graus não viram posição de mundo"
        );
        state.cancel_modal();

        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::new(1.0, 0.0, 0.0), 1.0).unwrap();
        let modal = state.modal.as_ref().unwrap();
        assert_eq!(modal.current_point(), Some(modal.pivot + Vec3::X));
    }

    #[test]
    fn test_proportional_editing_move_influences_unselected_vertices() {
        let mut state = selected_face();
        state.proportional_editing = true;
        state.session.proportional_settings.radius = 3.0;
        state.session.proportional_settings.falloff =
            crate::proportional::ProportionalFalloff::Linear;

        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::new(0.0, 1.0, 0.0), 1.0).unwrap();

        let mesh = state.project.active_mesh().unwrap();
        let selected_count = mesh.verts.iter().filter(|v| v.selected).count();
        let unselected_moved_count = mesh
            .verts
            .iter()
            .filter(|v| !v.selected && v.pos[1] > -1.0)
            .count();

        assert_eq!(selected_count, 4);
        assert!(unselected_moved_count > 0);
        state.commit_modal();
    }

    #[test]
    fn test_modal_tool_feedback_generation() {
        let mut state = selected_face();
        assert!(state.current_tool_feedback().is_none());

        state.begin_modal(ModalKind::Move).unwrap();
        state.update_modal(Vec3::new(1.0, 0.0, 0.0), 1.0).unwrap();

        let fb = state.current_tool_feedback().expect("feedback generated");
        assert!(fb.guide_line.is_some());
        assert!(fb.delta_text.contains("1.00"));
        assert!(!fb.is_snapped);

        state.cancel_modal();
        assert!(state.current_tool_feedback().is_none());
    }
}
