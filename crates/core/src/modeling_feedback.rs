//! Feedback comum de operações de modelagem (VSP-006 / MF-001 a MF-008).
//!
//! Reúne projeção unificada de fonte/revisão, alvos, transação, parâmetros com limites,
//! malha ghost (fonte fixa), prévia, região afetada e diagnósticos tipados antes do commit.
//! O renderer consome essa projeção diretamente sem repetir o cálculo geométrico.

use glam::Vec3;
use petunia_mesh::Mesh;
use uuid::Uuid;

use crate::AppState;

/// Nível de severidade dos diagnósticos de modelagem.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}

pub type DiagnosticSeverity = WarningSeverity;

/// Diagnóstico / aviso estruturado emitido por inspeção de operação ou preflight.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FeedbackWarning {
    pub severity: WarningSeverity,
    pub code: String,
    pub message: String,
    pub parameter: Option<String>,
    pub limit: Option<f32>,
    pub current: Option<f32>,
}

impl FeedbackWarning {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: WarningSeverity::Info,
            code: code.into(),
            message: message.into(),
            parameter: None,
            limit: None,
            current: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: WarningSeverity::Warning,
            code: code.into(),
            message: message.into(),
            parameter: None,
            limit: None,
            current: None,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: WarningSeverity::Error,
            code: code.into(),
            message: message.into(),
            parameter: None,
            limit: None,
            current: None,
        }
    }

    pub fn approaching_limit(
        param: impl Into<String>,
        current: f32,
        limit: f32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: WarningSeverity::Warning,
            code: "APPROACHING_LIMIT".to_string(),
            message: message.into(),
            parameter: Some(param.into()),
            limit: Some(limit),
            current: Some(current),
        }
    }

    pub fn exceeded_limit(
        param: impl Into<String>,
        current: f32,
        limit: f32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: WarningSeverity::Error,
            code: "EXCEEDED_LIMIT".to_string(),
            message: message.into(),
            parameter: Some(param.into()),
            limit: Some(limit),
            current: Some(current),
        }
    }
}

/// Marcador ou alça de feedback visual projetada no espaço 3D (MF-001/MF-004/MF-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FeedbackMarker {
    Point {
        position: [f32; 3],
        label: String,
        is_snapped: bool,
    },
    LimitBoundary {
        center: [f32; 3],
        radius: f32,
        label: String,
    },
    PlaneDisc {
        center: [f32; 3],
        normal: [f32; 3],
        radius: f32,
    },
    BrushFootprint {
        center: [f32; 3],
        normal: [f32; 3],
        radius: f32,
        falloff_radius: f32,
    },
}

/// Tipo canônico de dimensão ativa sob ajuste contínuo (VSP-003 / MF-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ActiveDimension {
    #[default]
    Distance,
    Factor,
    Angle,
    Offset,
    Count,
}

/// Descritor numérico de dimensão ativa com rótulo e unidades.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActiveDimensionDescriptor {
    pub label: String,
    pub value: f32,
    pub unit: String,
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl ActiveDimensionDescriptor {
    pub fn new(label: impl Into<String>, value: f32, unit: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            unit: unit.into(),
            min: None,
            max: None,
        }
    }

    pub fn with_bounds(mut self, min: Option<f32>, max: Option<f32>) -> Self {
        self.min = min;
        self.max = max;
        self
    }
}

/// Qualidade da medição ou feedback geométrico (VSP-003 / MF-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MeasurementQuality {
    Exact,
    Approximate,
    Stale,
    Unavailable,
}

/// Relatório de compatibilidade para seleções parciais ou mistas (MF-007 / MF-T08).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PartialSelectionReport {
    pub requested_domain: String,
    pub selected_vertices: usize,
    pub selected_edges: usize,
    pub selected_faces: usize,
    pub is_compatible: bool,
    pub reason: Option<String>,
    pub ignored_targets: Vec<Uuid>,
    pub supported_targets: Vec<Uuid>,
    pub requires_explicit_choice: bool,
}

/// Região afetada pela operação na malha autoral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AffectedRegion {
    pub vertices_count: usize,
    pub edges_count: usize,
    pub faces_count: usize,
}

/// Diagnóstico tipado de uma operação de modelagem em andamento ou inspecionada.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModelingDiagnostic {
    ApproachingLimit {
        parameter: String,
        current: f32,
        limit: f32,
        message: String,
    },
    ExceededLimit {
        parameter: String,
        current: f32,
        limit: f32,
        message: String,
    },
    NegativeScale {
        message: String,
    },
    StaleRevision {
        source_revision: u64,
        current_revision: u64,
    },
    LockedAsset {
        asset_id: Uuid,
    },
    NonPlanarWarning {
        message: String,
    },
    Notice {
        message: String,
    },
}

impl ModelingDiagnostic {
    pub fn severity(&self) -> WarningSeverity {
        match self {
            Self::Notice { .. } => WarningSeverity::Info,
            Self::ApproachingLimit { .. }
            | Self::NegativeScale { .. }
            | Self::NonPlanarWarning { .. } => WarningSeverity::Warning,
            Self::ExceededLimit { .. } | Self::StaleRevision { .. } | Self::LockedAsset { .. } => {
                WarningSeverity::Error
            }
        }
    }
}

/// Projeção unificada de feedback de operação (MF-001–MF-008).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperationFeedback {
    pub operation_id: String,
    pub operation_name: String,
    pub active_dimension: ActiveDimension,
    pub numeric_value: f32,
    pub preview_mesh: Option<Mesh>,
    pub ghost_mesh: Option<Mesh>,
    pub commit_enabled: bool,
    pub can_commit: bool,
    pub is_stale: bool,
    pub source_revision: u64,
    pub current_revision: u64,
    pub target_id: Option<Uuid>,
    pub target_name: String,
    pub quality: MeasurementQuality,
    pub affected_region: AffectedRegion,
    pub dimensions: Vec<ActiveDimensionDescriptor>,
    pub markers: Vec<FeedbackMarker>,
    pub warnings: Vec<FeedbackWarning>,
    pub partial_selection: Option<PartialSelectionReport>,
    pub operand_a: Option<Uuid>,
    pub operand_b: Option<Uuid>,
    pub plane: Option<[f32; 4]>,
    pub radius: Option<f32>,
    pub strength: Option<f32>,
}

impl OperationFeedback {
    pub fn has_warnings(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Warning)
    }

    pub fn has_errors(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Error)
    }

    pub fn max_severity(&self) -> WarningSeverity {
        self.warnings
            .iter()
            .map(|w| w.severity)
            .max()
            .unwrap_or(WarningSeverity::Info)
    }

    pub fn is_partial(&self) -> bool {
        self.partial_selection
            .as_ref()
            .is_some_and(|p| !p.ignored_targets.is_empty() || !p.is_compatible)
            || self
                .warnings
                .iter()
                .any(|w| w.code == "INCOMPATIBLE_SELECTION" || w.code == "LOCKED_ASSET")
    }

    pub fn hud_summary(&self) -> String {
        let status = if self.is_stale {
            "STALE"
        } else if !self.can_commit {
            "BLOCKED"
        } else if self.has_warnings() {
            "WARNING"
        } else {
            "VALID"
        };
        let param_str = self
            .dimensions
            .first()
            .map(|d| format!("{:.3} {}", d.value, d.unit))
            .unwrap_or_else(|| format!("{:.3}", self.numeric_value));
        format!(
            "{}: {} (verts: {}, faces: {}) [{}]",
            self.operation_name,
            param_str,
            self.affected_region.vertices_count,
            self.affected_region.faces_count,
            status
        )
    }
}

pub type ModelingFeedback = OperationFeedback;

/// Feedback estruturado de operações booleanas e espelhamento (MF-003 / MF-004).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BooleanModelingFeedback {
    pub operand_a: Option<Uuid>,
    pub operand_b: Option<Uuid>,
    pub operation_name: String,
    pub keep_parts: bool,
    pub plane: Option<[f32; 4]>,
    pub is_valid: bool,
    pub diagnostics: Vec<ModelingDiagnostic>,
}

impl BooleanModelingFeedback {
    pub fn new(
        operand_a: Option<Uuid>,
        operand_b: Option<Uuid>,
        operation_name: impl Into<String>,
        keep_parts: bool,
        plane: Option<[f32; 4]>,
    ) -> Self {
        let op_name = operation_name.into();
        let mut diagnostics = Vec::new();
        let mut is_valid = true;

        if operand_a.is_none() {
            diagnostics.push(ModelingDiagnostic::Notice {
                message: "No primary operand selected".to_string(),
            });
            is_valid = false;
        }

        if operand_a == operand_b && operand_a.is_some() {
            diagnostics.push(ModelingDiagnostic::ExceededLimit {
                parameter: "operand".to_string(),
                current: 0.0,
                limit: 0.0,
                message: "Self-boolean is not supported (operand A equals operand B)".to_string(),
            });
            is_valid = false;
        }

        Self {
            operand_a,
            operand_b,
            operation_name: op_name,
            keep_parts,
            plane,
            is_valid,
            diagnostics,
        }
    }
}

/// Inspetor de feedback de modelagem (VSP-006 / MF-001 a MF-008).
pub struct ModelingFeedbackInspector;

impl ModelingFeedbackInspector {
    /// Inspeciona a operação modal ou sessão ativa no estado da aplicação.
    pub fn inspect_current_operation(state: &AppState) -> Option<OperationFeedback> {
        if let Some(modal) = state.modal.as_ref() {
            let current_revision = state.project.revision_id();
            let active_asset = state.project.active();
            let target_id = active_asset.map(|a| a.id);
            let target_name = active_asset.map(|a| a.name.as_str());
            let active_mesh = state.project.active_mesh();
            return Some(modal.feedback(current_revision, target_id, target_name, active_mesh));
        }

        if let Some(preview) = state.mesh_preview.as_ref() {
            let current_revision = state.project.revision_id();
            let active_asset = state.project.active();
            let target_id = active_asset.map(|a| a.id);
            let target_name = active_asset
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Active Mesh".to_string());
            let source_revision = preview.original.revision_id();
            let is_stale = current_revision != source_revision;
            let mut warnings = Vec::new();
            if is_stale {
                warnings.push(FeedbackWarning::error(
                    "STALE_REVISION",
                    "Document revision modified during preview session",
                ));
            }
            let ghost_mesh = preview.original.active_mesh().cloned();
            let preview_mesh = state.project.active_mesh().cloned();
            let (verts, edges, faces) = if let Some(mesh) = preview.original.active_mesh() {
                (
                    mesh.verts.len(),
                    mesh.selected_edges.len(),
                    mesh.faces.len(),
                )
            } else {
                (0, 0, 0)
            };
            let can_commit = !is_stale
                && !warnings
                    .iter()
                    .any(|w| w.severity == WarningSeverity::Error);

            return Some(OperationFeedback {
                operation_id: preview.label.to_string(),
                operation_name: preview.label.to_string(),
                active_dimension: ActiveDimension::Distance,
                numeric_value: 0.0,
                preview_mesh,
                ghost_mesh,
                commit_enabled: can_commit,
                can_commit,
                source_revision,
                current_revision,
                is_stale,
                target_id,
                target_name,
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
                dimensions: Vec::new(),
                markers: Vec::new(),
                warnings,
                partial_selection: None,
                operand_a: None,
                operand_b: None,
                plane: None,
                radius: None,
                strength: None,
            });
        }

        None
    }

    /// Preflight de operação booleana (MF-003 / MF-T04).
    pub fn preflight_boolean(state: &AppState, op_a: Uuid, op_b: Uuid) -> OperationFeedback {
        let current_revision = state.project.revision_id();
        let mut warnings = Vec::new();

        let asset_a = state.project.assets.iter().find(|a| a.id == op_a);
        let asset_b = state.project.assets.iter().find(|a| a.id == op_b);

        if asset_a.is_none() {
            warnings.push(FeedbackWarning::error(
                "MISSING_OPERAND_A",
                format!("Primary operand asset {op_a} not found in project"),
            ));
        }
        if asset_b.is_none() {
            warnings.push(FeedbackWarning::error(
                "MISSING_OPERAND_B",
                format!("Secondary operand asset {op_b} not found in project"),
            ));
        }

        if op_a == op_b {
            warnings.push(FeedbackWarning::error(
                "SELF_BOOLEAN",
                "Self-boolean is not supported (operand A equals operand B)",
            ));
        }

        let is_open_a = asset_a.is_some_and(|a| !a.mesh.is_closed_manifold());
        let is_open_b = asset_b.is_some_and(|b| !b.mesh.is_closed_manifold());
        if is_open_a || is_open_b {
            warnings.push(FeedbackWarning::error(
                "OPEN_BOOLEAN_OPERAND",
                "Boolean operation requires watertight solids; open surface detected",
            ));
        }

        if let Some(a) = asset_a
            && a.locked
        {
            warnings.push(FeedbackWarning::error(
                "LOCKED_ASSET",
                format!("Operand A '{}' is locked", a.name),
            ));
        }
        if let Some(b) = asset_b
            && b.locked
        {
            warnings.push(FeedbackWarning::error(
                "LOCKED_ASSET",
                format!("Operand B '{}' is locked", b.name),
            ));
        }

        let can_commit = !warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Error);

        OperationFeedback {
            operation_id: "Boolean".to_string(),
            operation_name: "Boolean".to_string(),
            active_dimension: ActiveDimension::Count,
            numeric_value: 0.0,
            preview_mesh: None,
            ghost_mesh: asset_a.map(|a| (*a.mesh).clone()),
            commit_enabled: can_commit,
            can_commit,
            source_revision: current_revision,
            current_revision,
            is_stale: false,
            target_id: Some(op_a),
            target_name: asset_a
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Operand A".to_string()),
            quality: MeasurementQuality::Exact,
            affected_region: AffectedRegion::default(),
            dimensions: Vec::new(),
            markers: Vec::new(),
            warnings,
            partial_selection: None,
            operand_a: Some(op_a),
            operand_b: Some(op_b),
            plane: None,
            radius: None,
            strength: None,
        }
    }

    /// Preflight de simetria / Mirror (MF-004 / MF-T05).
    pub fn preflight_mirror(
        state: &AppState,
        asset_id: Uuid,
        normal: [f32; 3],
        offset: f32,
    ) -> OperationFeedback {
        let current_revision = state.project.revision_id();
        let mut warnings = Vec::new();
        let asset = state.project.assets.iter().find(|a| a.id == asset_id);

        if asset.is_none() {
            warnings.push(FeedbackWarning::error(
                "MISSING_ASSET",
                format!("Asset {asset_id} not found in project"),
            ));
        } else if let Some(a) = asset
            && a.locked
        {
            warnings.push(FeedbackWarning::error(
                "LOCKED_ASSET",
                format!("Asset '{}' is locked", a.name),
            ));
        }

        let n = Vec3::from(normal);
        if !n.is_finite() || n.length_squared() < 1e-6 {
            warnings.push(FeedbackWarning::error(
                "INVALID_NORMAL",
                "Mirror plane normal must be non-zero and finite",
            ));
        }

        if !offset.is_finite() {
            warnings.push(FeedbackWarning::error(
                "INVALID_OFFSET",
                "Mirror plane offset must be finite",
            ));
        }

        if let Some(a) = asset {
            let n_unit = n.normalize_or_zero();
            let is_asymmetric = a.mesh.verts.iter().any(|v| {
                let p = v.vec();
                let dist = p.dot(n_unit) - offset;
                let reflected = p - 2.0 * dist * n_unit;
                !a.mesh
                    .verts
                    .iter()
                    .any(|other| (other.vec() - reflected).length_squared() < 1e-4)
            });
            if is_asymmetric {
                warnings.push(FeedbackWarning::info(
                    "ASYMMETRIC_SOURCE_DETECTED",
                    "Source geometry is not symmetrical across the mirror plane; asymmetric details will be mirrored",
                ));
            }
        }

        let can_commit = !warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Error);

        OperationFeedback {
            operation_id: "Mirror".to_string(),
            operation_name: "Mirror".to_string(),
            active_dimension: ActiveDimension::Offset,
            numeric_value: offset,
            preview_mesh: None,
            ghost_mesh: asset.map(|a| (*a.mesh).clone()),
            commit_enabled: can_commit,
            can_commit,
            source_revision: current_revision,
            current_revision,
            is_stale: false,
            target_id: Some(asset_id),
            target_name: asset
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Target Asset".to_string()),
            quality: MeasurementQuality::Exact,
            affected_region: AffectedRegion::default(),
            dimensions: vec![ActiveDimensionDescriptor {
                label: "Offset".to_string(),
                value: offset,
                unit: "m".to_string(),
                min: None,
                max: None,
            }],
            markers: vec![FeedbackMarker::PlaneDisc {
                center: (n.normalize_or_zero() * offset).to_array(),
                normal: [normal[0], normal[1], normal[2]],
                radius: 1.0,
            }],
            warnings,
            partial_selection: None,
            operand_a: Some(asset_id),
            operand_b: None,
            plane: Some([normal[0], normal[1], normal[2], offset]),
            radius: None,
            strength: None,
        }
    }

    /// Preflight de pincel de escultura / deformação (MF-005 / MF-T06).
    pub fn preflight_sculpt(
        radius: f32,
        strength: f32,
        hit: [f32; 3],
        _normal: [f32; 3],
    ) -> OperationFeedback {
        let mut warnings = Vec::new();

        if !radius.is_finite() || radius <= 0.0 {
            warnings.push(FeedbackWarning::error(
                "INVALID_RADIUS",
                "Sculpt brush radius must be greater than zero",
            ));
        }
        if !strength.is_finite() {
            warnings.push(FeedbackWarning::error(
                "INVALID_STRENGTH",
                "Sculpt brush strength must be finite",
            ));
        } else if strength > 1.0 || strength < 0.0 {
            warnings.push(FeedbackWarning::warning(
                "EXTREME_STRENGTH",
                "Sculpt strength outside recommended [0.0, 1.0] range",
            ));
        }

        let can_commit = !warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Error);

        OperationFeedback {
            operation_id: "sculpt".to_string(),
            operation_name: "Sculpt".to_string(),
            active_dimension: ActiveDimension::Distance,
            numeric_value: radius,
            preview_mesh: None,
            ghost_mesh: None,
            commit_enabled: can_commit,
            can_commit,
            source_revision: 0,
            current_revision: 0,
            is_stale: false,
            target_id: None,
            target_name: "Active Surface".to_string(),
            quality: MeasurementQuality::Exact,
            affected_region: AffectedRegion::default(),
            dimensions: vec![
                ActiveDimensionDescriptor {
                    label: "Radius".to_string(),
                    value: radius,
                    unit: "m".to_string(),
                    min: Some(0.001),
                    max: None,
                },
                ActiveDimensionDescriptor {
                    label: "Strength".to_string(),
                    value: strength,
                    unit: "%".to_string(),
                    min: Some(0.0),
                    max: Some(1.0),
                },
            ],
            markers: vec![FeedbackMarker::BrushFootprint {
                center: hit,
                normal: _normal,
                radius,
                falloff_radius: radius * 0.5,
            }],
            warnings,
            partial_selection: None,
            operand_a: None,
            operand_b: None,
            plane: None,
            radius: Some(radius),
            strength: Some(strength),
        }
    }

    /// Preflight de remesh sobre malha com skin bind (MF-007 / MF-T07).
    pub fn preflight_remesh_skin(state: &AppState, asset_id: Uuid) -> OperationFeedback {
        let current_revision = state.project.revision_id();
        let mut warnings = Vec::new();

        let asset = state.project.assets.iter().find(|a| a.id == asset_id);
        let has_skin = asset.is_some_and(|a| a.skeleton_id.is_some() || a.skin_data.is_some());

        if has_skin {
            warnings.push(FeedbackWarning::warning(
                "SKIN_REMAP_REQUIRED",
                "Remeshing this asset will discard vertex skinning weights. Re-binding will be required.",
            ));
            warnings.push(FeedbackWarning::warning(
                "RIG_BIND_LOSS",
                "Remeshing this asset will discard vertex skinning weights. Re-binding will be required.",
            ));
        } else {
            warnings.push(FeedbackWarning::info(
                "CLEAN_TOPOLOGY",
                "Asset has no active skin binding; remesh is safe for rig data.",
            ));
        }

        let can_commit = !warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Error);

        OperationFeedback {
            operation_id: "Remesh".to_string(),
            operation_name: "Remesh".to_string(),
            active_dimension: ActiveDimension::Count,
            numeric_value: 0.0,
            preview_mesh: None,
            ghost_mesh: asset.map(|a| (*a.mesh).clone()),
            commit_enabled: can_commit,
            can_commit,
            source_revision: current_revision,
            current_revision,
            is_stale: false,
            target_id: Some(asset_id),
            target_name: asset
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Target Asset".to_string()),
            quality: MeasurementQuality::Exact,
            affected_region: AffectedRegion::default(),
            dimensions: Vec::new(),
            markers: Vec::new(),
            warnings,
            partial_selection: None,
            operand_a: Some(asset_id),
            operand_b: None,
            plane: None,
            radius: None,
            strength: None,
        }
    }

    /// Preflight de seleção mista ou parcialmente incompatível (MF-007 / MF-T08).
    pub fn preflight_partial_selection(
        state: &AppState,
        requested_domain: &str,
    ) -> OperationFeedback {
        let current_revision = state.project.revision_id();
        let mut warnings = Vec::new();

        let (verts, edges, faces) = if let Some(mesh) = state.project.active_mesh() {
            (
                mesh.verts.iter().filter(|v| v.selected).count(),
                mesh.selected_edges.len(),
                mesh.faces.iter().filter(|f| f.selected).count(),
            )
        } else {
            (
                state.session.selection.verts.len(),
                state.session.selection.edges.len(),
                state.session.selection.faces.len(),
            )
        };

        let mut is_compatible = true;
        let mut reason = None;

        let has_locked_asset = state.session.selection.assets.iter().any(|id| {
            state
                .project
                .assets
                .iter()
                .find(|a| a.id == *id)
                .is_some_and(|a| a.locked)
        });

        let mut ignored_targets = Vec::new();
        let mut supported_targets = Vec::new();

        for asset_id in &state.session.selection.assets {
            if let Some(asset) = state.project.assets.iter().find(|a| a.id == *asset_id) {
                if asset.locked {
                    ignored_targets.push(*asset_id);
                } else {
                    supported_targets.push(*asset_id);
                }
            }
        }

        let requires_explicit_choice = !ignored_targets.is_empty() && !supported_targets.is_empty();

        if has_locked_asset {
            warnings.push(FeedbackWarning::warning(
                "LOCKED_ASSET",
                "Selection contains locked objects that cannot be modified",
            ));
        }

        match requested_domain {
            "Face" | "face_operation" if faces == 0 => {
                is_compatible = false;
                let msg = if verts > 0 || edges > 0 {
                    "Operation requires faces; selected vertices/edges cannot be inferred as complete faces"
                } else {
                    "No geometry selected; face selection required"
                };
                reason = Some(msg.to_string());
                warnings.push(FeedbackWarning::error("INCOMPATIBLE_SELECTION", msg));
            }
            "Edge" if edges == 0 => {
                is_compatible = false;
                let msg = "Operation requires edges; select at least one edge";
                reason = Some(msg.to_string());
                warnings.push(FeedbackWarning::error("INCOMPATIBLE_SELECTION", msg));
            }
            "Vertex" if verts == 0 => {
                is_compatible = false;
                let msg = "Operation requires vertices; select at least one vertex";
                reason = Some(msg.to_string());
                warnings.push(FeedbackWarning::error("INCOMPATIBLE_SELECTION", msg));
            }
            _ => {}
        }

        let can_commit = is_compatible
            && !has_locked_asset
            && !warnings
                .iter()
                .any(|w| w.severity == WarningSeverity::Error);

        OperationFeedback {
            operation_id: "Selection Validation".to_string(),
            operation_name: "Selection Validation".to_string(),
            active_dimension: ActiveDimension::Count,
            numeric_value: faces as f32,
            preview_mesh: None,
            ghost_mesh: state.project.active_mesh().cloned(),
            commit_enabled: can_commit,
            can_commit,
            source_revision: current_revision,
            current_revision,
            is_stale: false,
            target_id: state.project.active().map(|a| a.id),
            target_name: state
                .project
                .active()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Active Object".to_string()),
            quality: MeasurementQuality::Exact,
            affected_region: AffectedRegion {
                vertices_count: verts,
                edges_count: edges,
                faces_count: faces,
            },
            dimensions: Vec::new(),
            markers: Vec::new(),
            warnings,
            partial_selection: Some(PartialSelectionReport {
                requested_domain: requested_domain.to_string(),
                selected_vertices: verts,
                selected_edges: edges,
                selected_faces: faces,
                is_compatible: is_compatible && !has_locked_asset,
                reason,
                ignored_targets,
                supported_targets,
                requires_explicit_choice,
            }),
            operand_a: None,
            operand_b: None,
            plane: None,
            radius: None,
            strength: None,
        }
    }
}
