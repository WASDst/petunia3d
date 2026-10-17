//! Comandos transacionais de alinhamento e distribuição 3D de objetos.
//!
//! Integra os cálculos puros de `petunia_project::align` com o despachante
//! transacional do Petunia3D (`CommandDispatcher`), garantindo auto-checkpointing de
//! Undo/Redo, validação de pré-condições e proteção contra estados degenerados.

use petunia_project::{
    AlignAxis, AlignMode, AlignReference, ProjectChanges, compute_alignment_deltas,
    compute_distribution_deltas,
};
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

// --- Funções Auxiliares de Resolução de Seleção ---

fn resolve_target_assets(
    document: &petunia_project::Project,
    selected_assets: &[Uuid],
) -> Vec<Uuid> {
    let mut ids = Vec::new();

    // 1. Adiciona os objetos explicitamente selecionados na sessão
    for &id in selected_assets {
        if !ids.contains(&id) && document.assets.iter().any(|a| a.id == id && !a.locked) {
            ids.push(id);
        }
    }

    // 2. Assegura que o objeto ativo esteja presente
    if let Some(active) = document.active()
        && !active.locked
        && !ids.contains(&active.id)
    {
        ids.push(active.id);
    }

    ids
}

// --- Comando: AlignObjectsCmd ---

/// Comando para alinhar objetos 3D no espaço mundial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlignObjectsCmd {
    pub axis: AlignAxis,
    pub mode: AlignMode,
    pub reference: AlignReference,
}

impl AlignObjectsCmd {
    pub const fn new(axis: AlignAxis, mode: AlignMode, reference: AlignReference) -> Self {
        Self {
            axis,
            mode,
            reference,
        }
    }

    pub const fn center_x() -> Self {
        Self::new(
            AlignAxis::X,
            AlignMode::Center,
            AlignReference::ActiveObject,
        )
    }

    pub const fn center_y() -> Self {
        Self::new(
            AlignAxis::Y,
            AlignMode::Center,
            AlignReference::ActiveObject,
        )
    }

    pub const fn center_z() -> Self {
        Self::new(
            AlignAxis::Z,
            AlignMode::Center,
            AlignReference::ActiveObject,
        )
    }

    pub const fn min_x() -> Self {
        Self::new(AlignAxis::X, AlignMode::Min, AlignReference::ActiveObject)
    }

    pub const fn max_x() -> Self {
        Self::new(AlignAxis::X, AlignMode::Max, AlignReference::ActiveObject)
    }

    pub const fn min_y() -> Self {
        Self::new(AlignAxis::Y, AlignMode::Min, AlignReference::ActiveObject)
    }

    pub const fn max_y() -> Self {
        Self::new(AlignAxis::Y, AlignMode::Max, AlignReference::ActiveObject)
    }

    pub const fn min_z() -> Self {
        Self::new(AlignAxis::Z, AlignMode::Min, AlignReference::ActiveObject)
    }

    pub const fn max_z() -> Self {
        Self::new(AlignAxis::Z, AlignMode::Max, AlignReference::ActiveObject)
    }
}

impl Command for AlignObjectsCmd {
    fn label(&self) -> &'static str {
        match (self.axis, self.mode) {
            (AlignAxis::X, AlignMode::Min) => "align min x",
            (AlignAxis::X, AlignMode::Center) => "align center x",
            (AlignAxis::X, AlignMode::Max) => "align max x",
            (AlignAxis::Y, AlignMode::Min) => "align min y",
            (AlignAxis::Y, AlignMode::Center) => "align center y",
            (AlignAxis::Y, AlignMode::Max) => "align max y",
            (AlignAxis::Z, AlignMode::Min) => "align min z",
            (AlignAxis::Z, AlignMode::Center) => "align center z",
            (AlignAxis::Z, AlignMode::Max) => "align max z",
        }
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        true
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TRANSFORMS | ProjectChanges::POSITIONS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let participating =
            resolve_target_assets(&state.document.project, &state.editor.selection.assets);

        let min_required = match self.reference {
            AlignReference::WorldOrigin => 1,
            AlignReference::ActiveObject | AlignReference::SelectionBounds => 2,
        };

        if participating.len() < min_required {
            return Err("Select at least 2 objects to align (or 1 for World Origin)");
        }

        if self.reference == AlignReference::ActiveObject && state.document.active().is_none() {
            return Err("No active object to serve as alignment anchor");
        }

        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let participating_ids =
            resolve_target_assets(&state.document.project, &state.editor.selection.assets);

        let active_id = state.document.active().map(|a| a.id);

        // Coleta referências imutáveis para cálculo
        let asset_refs: Vec<&petunia_project::Asset> = state
            .document
            .assets
            .iter()
            .filter(|a| participating_ids.contains(&a.id))
            .collect();

        let deltas =
            compute_alignment_deltas(&asset_refs, active_id, self.axis, self.mode, self.reference)
                .map_err(|e| CommandError::Execution(format!("Alignment failed: {e}")))?;

        let mut moved_count = 0;
        for (id, delta) in deltas {
            if delta.length_squared() <= 1.0e-9 {
                continue;
            }
            if let Some(asset) = state.document.assets.iter_mut().find(|a| a.id == id) {
                asset
                    .translate_world(delta)
                    .map_err(|e| CommandError::Execution(format!("Transform error: {e:?}")))?;
                moved_count += 1;
            }
        }

        let axis_str = match self.axis {
            AlignAxis::X => "X",
            AlignAxis::Y => "Y",
            AlignAxis::Z => "Z",
        };
        let mode_str = match self.mode {
            AlignMode::Min => "Min",
            AlignMode::Center => "Center",
            AlignMode::Max => "Max",
        };

        state.set_status(format!(
            "Aligned {moved_count} objects along {axis_str} ({mode_str})"
        ));
        Ok(())
    }
}

// --- Comando: DistributeObjectsCmd ---

/// Comando para distribuir uniformemente o espaçamento entre 3 ou mais objetos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DistributeObjectsCmd {
    pub axis: AlignAxis,
}

impl DistributeObjectsCmd {
    pub const fn new(axis: AlignAxis) -> Self {
        Self { axis }
    }

    pub const fn on_x() -> Self {
        Self::new(AlignAxis::X)
    }

    pub const fn on_y() -> Self {
        Self::new(AlignAxis::Y)
    }

    pub const fn on_z() -> Self {
        Self::new(AlignAxis::Z)
    }
}

impl Command for DistributeObjectsCmd {
    fn label(&self) -> &'static str {
        match self.axis {
            AlignAxis::X => "distribute objects x",
            AlignAxis::Y => "distribute objects y",
            AlignAxis::Z => "distribute objects z",
        }
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        true
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TRANSFORMS | ProjectChanges::POSITIONS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let participating =
            resolve_target_assets(&state.document.project, &state.editor.selection.assets);

        if participating.len() < 3 {
            return Err("Select at least 3 objects to distribute spacing");
        }

        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let participating_ids =
            resolve_target_assets(&state.document.project, &state.editor.selection.assets);

        let asset_refs: Vec<&petunia_project::Asset> = state
            .document
            .assets
            .iter()
            .filter(|a| participating_ids.contains(&a.id))
            .collect();

        let deltas = compute_distribution_deltas(&asset_refs, self.axis)
            .map_err(|e| CommandError::Execution(format!("Distribution failed: {e}")))?;

        let mut moved_count = 0;
        for (id, delta) in deltas {
            if delta.length_squared() <= 1.0e-9 {
                continue;
            }
            if let Some(asset) = state.document.assets.iter_mut().find(|a| a.id == id) {
                asset
                    .translate_world(delta)
                    .map_err(|e| CommandError::Execution(format!("Transform error: {e:?}")))?;
                moved_count += 1;
            }
        }

        let axis_str = match self.axis {
            AlignAxis::X => "X",
            AlignAxis::Y => "Y",
            AlignAxis::Z => "Z",
        };

        state.set_status(format!(
            "Distributed {moved_count} objects along {axis_str}"
        ));
        Ok(())
    }
}
