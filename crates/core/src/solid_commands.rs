//! Comandos do ShapeBuilder sólido de dois operandos (H25).
//!
//! Integra partição A-only/overlap/B-only, componentes por aresta e RegionPlan
//! com o sistema transacional de comandos do Petunia3D.

use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

pub use petunia_mesh::SolidOperation;

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

/// Comando para criar uma composição sólida reeditável a partir de fontes estáveis (H26).
#[derive(Clone, Debug, PartialEq)]
pub struct CreateSolidCompositionCmd {
    pub base: Option<Uuid>,
    pub operands: Vec<Uuid>,
    pub op: SolidOperation,
    pub budget: Option<petunia_mesh::ShapeBuilderBudget>,
}

impl CreateSolidCompositionCmd {
    pub fn new(operands: Vec<Uuid>, op: SolidOperation) -> Self {
        Self {
            base: None,
            operands,
            op,
            budget: None,
        }
    }

    pub fn with_base(base: Uuid, operands: Vec<Uuid>, op: SolidOperation) -> Self {
        Self {
            base: Some(base),
            operands,
            op,
            budget: None,
        }
    }
}

impl Command for CreateSolidCompositionCmd {
    fn label(&self) -> &'static str {
        "create solid composition"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let base_id = match self.base {
            Some(id) => id,
            None => {
                let Some(active) = state.document.active() else {
                    return Err("No active asset for solid composition");
                };
                active.id
            }
        };

        if self.operands.is_empty() {
            return Err("At least one operand required for solid composition");
        }

        if self.operands.contains(&base_id) {
            return Err("Base asset cannot be used as its own operand");
        }

        for &op_id in &self.operands {
            if !state.document.assets.iter().any(|a| a.id == op_id) {
                return Err("Referenced operand asset does not exist in document");
            }
        }

        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.create_solid_composition(self.base, &self.operands, self.op, self.budget)?;
        state.set_status(format!(
            "Solid Composition created with {} operands",
            self.operands.len()
        ));
        Ok(())
    }
}

/// Comando explícito para conversão de composição sólida em malha poligonal estática (Bake, H26).
/// Sem bake silencioso: apenas este comando consome a receita e materializa EditableMesh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BakeSolidCompositionCmd {
    pub target: Option<Uuid>,
}

impl BakeSolidCompositionCmd {
    pub const fn new() -> Self {
        Self { target: None }
    }

    pub const fn for_asset(target: Uuid) -> Self {
        Self {
            target: Some(target),
        }
    }
}

impl Command for BakeSolidCompositionCmd {
    fn label(&self) -> &'static str {
        "bake solid composition"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = match self.target {
            Some(id) => state.document.assets.iter().find(|a| a.id == id),
            None => state.document.active(),
        };
        let Some(asset) = asset else {
            return Err("Target asset not found");
        };
        if !asset.is_composition() {
            return Err("Target asset is not a solid composition");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.bake_solid_composition(self.target)?;
        state.set_status("Solid Composition baked to static mesh");
        Ok(())
    }
}

/// Comando para habilitar ou desabilitar um passo de composição sólida reeditável.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToggleSolidCompositionStepCmd {
    pub target: Option<Uuid>,
    pub step_id: Uuid,
    pub enabled: bool,
}

impl ToggleSolidCompositionStepCmd {
    pub fn new(step_id: Uuid, enabled: bool) -> Self {
        Self {
            target: None,
            step_id,
            enabled,
        }
    }
}

impl Command for ToggleSolidCompositionStepCmd {
    fn label(&self) -> &'static str {
        "toggle solid composition step"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = match self.target {
            Some(id) => state.document.assets.iter().find(|a| a.id == id),
            None => state.document.active(),
        };
        let Some(asset) = asset else {
            return Err("Target asset not found");
        };
        let Some(recipe) = &asset.composition else {
            return Err("Target asset is not a solid composition");
        };
        if !recipe.steps.iter().any(|s| s.id == self.step_id) {
            return Err("Step not found in solid composition recipe");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.toggle_solid_composition_step(self.target, self.step_id, self.enabled)?;
        state.set_status("Solid composition step updated");
        Ok(())
    }
}

/// Comando para reordenar passos de composição sólida reeditável.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReorderSolidCompositionStepCmd {
    pub target: Option<Uuid>,
    pub from: usize,
    pub to: usize,
}

impl ReorderSolidCompositionStepCmd {
    pub fn new(from: usize, to: usize) -> Self {
        Self {
            target: None,
            from,
            to,
        }
    }
}

impl Command for ReorderSolidCompositionStepCmd {
    fn label(&self) -> &'static str {
        "reorder solid composition step"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = match self.target {
            Some(id) => state.document.assets.iter().find(|a| a.id == id),
            None => state.document.active(),
        };
        let Some(asset) = asset else {
            return Err("Target asset not found");
        };
        let Some(recipe) = &asset.composition else {
            return Err("Target asset is not a solid composition");
        };
        if self.from >= recipe.steps.len() || self.to >= recipe.steps.len() {
            return Err("Step index out of range");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.reorder_solid_composition_steps(self.target, self.from, self.to)?;
        state.set_status("Solid composition steps reordered");
        Ok(())
    }
}
