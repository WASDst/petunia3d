//! Quadro de prévia das ferramentas na vista (ADR-C16, B3): extrude, push/pull,
//! inset, bevel, escala da seleção e knife.
//!
//! A operação já é visível na malha em prévia; o quadro acrescenta o que a malha
//! não diz: de onde a operação parte (âncora), para onde vai (seta ao longo da
//! normal), o valor atual com a unidade e o texto digitado. Fica perto da
//! operação, não num canto. A GUI só desenha; os números vêm do modal.

use crate::projection::project_world_point;
use crate::{PetuniaViewport, SlintUiBridge, ToolModalKind};

/// O que a vista desenha para a ferramenta em curso. Coordenadas em px da vista.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolPreviewFrame {
    /// `extrude`, `push_pull`, `inset`, `bevel`, `scale` ou `knife`.
    pub tool: &'static str,
    pub title: String,
    pub anchor: [f32; 2],
    /// Ponta da seta (extrude/push-pull ao longo da normal; knife até o ponteiro).
    pub tip: Option<[f32; 2]>,
    pub value: f32,
    /// `m`, `×` ou `seg` (segmentos do knife).
    pub unit: &'static str,
    /// Valor digitado ainda não confirmado (vazio = valor do arrasto).
    pub typed: String,
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Quadro da ferramenta paramétrica ou do knife em curso; `None` sem operação
    /// ou com a âncora fora da vista (atrás da câmera).
    pub fn tool_preview_frame(&self) -> Option<ToolPreviewFrame> {
        let camera = &self.state.session.camera;
        let viewport = self.viewport_size;
        if viewport[0] <= 1.0 || viewport[1] <= 1.0 {
            return None;
        }
        if let Some(kind) = self.tool_modal {
            let modal = self.state.session.tools.modal.as_ref()?;
            let anchor = project_world_point(camera, viewport, modal.pivot)?;
            let value = self.tool_modal_value;
            let along_normal = matches!(
                kind,
                ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual | ToolModalKind::PushPull
            );
            let tip = if along_normal
                && value.is_finite()
                && value.abs() > 1.0e-6
                && modal.normal.is_finite()
                && modal.normal.length_squared() > 1.0e-8
            {
                project_world_point(
                    camera,
                    viewport,
                    modal.pivot + modal.normal.normalize() * value,
                )
            } else {
                None
            };
            let (tool, unit) = match kind {
                ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual => ("extrude", "m"),
                ToolModalKind::PushPull => ("push_pull", "m"),
                ToolModalKind::Inset => ("inset", "m"),
                ToolModalKind::Bevel => ("bevel", "m"),
                ToolModalKind::ScaleSelection => ("scale", "×"),
            };
            return Some(ToolPreviewFrame {
                tool,
                title: self.state.t(kind.title_key()),
                anchor,
                tip,
                value,
                unit,
                typed: self.tool_session.numeric_text().to_string(),
            });
        }
        if self.state.session.tools.active_tool != "knife" {
            return None;
        }
        let session = self.state.session.tools.cut_session.as_ref()?;
        let start = session.edge_start.as_ref()?;
        let world = session.object_to_world.transform_point3(start.position);
        let anchor = project_world_point(camera, viewport, world)?;
        let pointer = self.pointer_position;
        Some(ToolPreviewFrame {
            tool: "knife",
            title: self.state.t("tools.knife"),
            anchor,
            tip: pointer.iter().all(|v| v.is_finite()).then_some(pointer),
            value: session.segments as f32,
            unit: "seg",
            typed: String::new(),
        })
    }
}
