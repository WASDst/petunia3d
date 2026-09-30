//! Petunia3D — Sistema de Feedback de Ferramentas Modais (P3D-131).
//!
//! Fornece descritores de apresentação desacoplados de egui para
//! linhas de guia, badges de delta, estado de eixos e dicas de status.

use glam::Vec3;

/// Descritor de apresentação de feedback modal emitido por operações em andamento.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolFeedback {
    /// Ponto de ancoragem / pivô original da operação (em espaço de mundo).
    pub origin: Vec3,
    /// Ponto atual sob o cursor / alvo ativo (em espaço de mundo).
    pub current: Vec3,
    /// Linha de guia entre a origem e o ponto atual.
    pub guide_line: Option<(Vec3, Vec3)>,
    /// Rótulo curto com o delta ou magnitude (ex: "Δ 1.25 m", "Rot 45.0°", "Scale 1.50×").
    pub delta_text: String,
    /// Valor escalar numérico.
    pub delta_value: f32,
    /// Eixo restrito ativo (0 = X, 1 = Y, 2 = Z), se houver.
    pub axis_constraint: Option<usize>,
    /// Plano restrito ativo (0 = YZ, 1 = XZ, 2 = XY), se houver.
    pub plane_constraint: Option<usize>,
    /// Indicador se o ponto atual foi atraído por snap magnético.
    pub is_snapped: bool,
    /// Tipo do alvo que encaixou (forma + rótulo no HUD), quando houver.
    pub snap_kind: Option<crate::inference::SnapKind>,
    /// Instruções de tecla para a barra de status.
    pub status_hint: &'static str,
}

impl ToolFeedback {
    pub fn new(
        origin: Vec3,
        current: Vec3,
        delta_text: impl Into<String>,
        delta_value: f32,
    ) -> Self {
        Self {
            origin,
            current,
            guide_line: Some((origin, current)),
            delta_text: delta_text.into(),
            delta_value,
            axis_constraint: None,
            plane_constraint: None,
            is_snapped: false,
            snap_kind: None,
            status_hint: "LMB Confirm · RMB / Esc Cancel · X/Y/Z Axis",
        }
    }
}
