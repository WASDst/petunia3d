//! Comandos do ShapeBuilder sólido de dois operandos (H25).
//!
//! Integra partição A-only/overlap/B-only, componentes por aresta e RegionPlan
//! com o sistema transacional de comandos do Petunia3D.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

/// Operação do ShapeBuilder sólido sobre o RegionPlan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolidOperation {
    /// Une todas as regiões ativas em um único sólido (equivalente a A ∪ B).
    UniteAll,
    /// Remove as regiões de overlap de todos os operandos.
    RemoveOverlap,
    /// Extrai o overlap como um novo objeto na cena, retirando-o dos operandos de origem.
    ExtractOverlap,
}

/// Relatório de execução do ShapeBuilder sólido.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SolidShapeBuilderReport {
    /// Se o primeiro operando permaneceu totalmente intocado.
    pub preserved_first: bool,
    /// Se o segundo operando permaneceu totalmente intocado.
    pub preserved_second: bool,
    /// Quantidade de novos objetos criados no documento.
    pub created_objects: usize,
    /// Quantidade total de regiões sólidas decompostas.
    pub regions_count: usize,
}

/// Comando transacional do ShapeBuilder sólido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolidShapeEditCmd {
    pub operand: Option<Uuid>,
    pub op: SolidOperation,
}

impl Default for SolidShapeEditCmd {
    fn default() -> Self {
        Self {
            operand: None,
            op: SolidOperation::UniteAll,
        }
    }
}

impl SolidShapeEditCmd {
    pub const fn new(op: SolidOperation) -> Self {
        Self { operand: None, op }
    }

    pub const fn with_operand(op: SolidOperation, operand: Uuid) -> Self {
        Self {
            operand: Some(operand),
            op,
        }
    }

    pub const fn unite() -> Self {
        Self::new(SolidOperation::UniteAll)
    }

    pub const fn unite_with_operand(operand: Uuid) -> Self {
        Self::with_operand(SolidOperation::UniteAll, operand)
    }

    pub const fn remove_overlap() -> Self {
        Self::new(SolidOperation::RemoveOverlap)
    }

    pub const fn remove_overlap_with_operand(operand: Uuid) -> Self {
        Self::with_operand(SolidOperation::RemoveOverlap, operand)
    }

    pub const fn extract_overlap() -> Self {
        Self::new(SolidOperation::ExtractOverlap)
    }

    pub const fn extract_overlap_with_operand(operand: Uuid) -> Self {
        Self::with_operand(SolidOperation::ExtractOverlap, operand)
    }
}

impl Command for SolidShapeEditCmd {
    fn label(&self) -> &'static str {
        match self.op {
            SolidOperation::UniteAll => "solid shape unite",
            SolidOperation::RemoveOverlap => "solid shape remove overlap",
            SolidOperation::ExtractOverlap => "solid shape extract overlap",
        }
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            return Err("No active mesh");
        }
        let operand = self.operand.or(state.editor.tools.boolean_operand);
        let Some(operand) = operand else {
            return Err("Choose a boolean operand first");
        };
        if !state
            .document
            .assets
            .iter()
            .any(|asset| asset.id == operand)
        {
            return Err("The boolean operand no longer exists");
        }
        if state
            .document
            .active()
            .is_some_and(|asset| asset.id == operand)
        {
            return Err("The boolean operand cannot be the active object");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let report = state.apply_solid_shape_builder(self.operand, self.op)?;
        state.set_status(format!(
            "Solid Shape Builder: {} regions, {} objects created",
            report.regions_count, report.created_objects
        ));
        Ok(())
    }
}
