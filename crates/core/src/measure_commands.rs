//! Medidas 3D da ferramenta Medir: gravar e limpar, sempre com Undo. O gesto
//! (dois cliques com prévia) vive na ponte; aqui só a mutação do documento.

use petunia_project::{MeasurementItem, ProjectChanges};

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

/// Menor distância aceita: abaixo disso os dois pontos são o mesmo.
pub const MIN_MEASURE_DISTANCE: f32 = 1.0e-4;

/// Grava uma medida entre dois pontos de mundo.
#[derive(Debug, Clone)]
pub struct AddMeasurementCmd {
    pub item: MeasurementItem,
}

impl Command for AddMeasurementCmd {
    fn label(&self) -> &'static str {
        "add measurement"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        let finite = self
            .item
            .start
            .iter()
            .chain(&self.item.end)
            .all(|value| value.is_finite());
        if !finite {
            return Err("The measurement points must be finite");
        }
        if self.item.distance.is_finite() && self.item.distance >= MIN_MEASURE_DISTANCE {
            Ok(())
        } else {
            Err("The two points are the same")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.add_measurement(self.item.clone());
        state.mark_dirty();
        state.set_status(format!("{}: {:.3} m", self.item.name, self.item.distance));
        Ok(())
    }
}

/// Remove todas as medidas gravadas (`measure.clear`).
#[derive(Debug, Clone, Default)]
pub struct ClearMeasurementsCmd;

impl Command for ClearMeasurementsCmd {
    fn label(&self) -> &'static str {
        "clear measurements"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.project.measurements.is_empty() {
            Err("There are no measurements to clear")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let count = state.document.project.measurements.len();
        if count == 0 {
            return Err(CommandError::NoChange("nenhuma medida".into()));
        }
        state.document.project.measurements.clear();
        state.mark_dirty();
        state.set_status(format!("{count} medida(s) removida(s)"));
        Ok(())
    }
}
