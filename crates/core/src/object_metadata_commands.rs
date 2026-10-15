//! Authored object presentation metadata uses the shared transactional writer.
use crate::{Command, CommandContext, CommandError};
use petunia_project::ProjectChanges;
#[derive(Debug, Clone)]
pub struct SetAssetSelectionColorCmd {
    pub object: uuid::Uuid,
    pub color: Option<[u8; 3]>,
}
impl Command for SetAssetSelectionColorCmd {
    fn label(&self) -> &'static str {
        "set object selection color"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }
    fn execute(&self, ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let asset = ctx
            .document
            .assets
            .iter_mut()
            .find(|a| a.id == self.object)
            .ok_or_else(|| CommandError::StaleReference(self.object.to_string()))?;
        if asset.locked {
            return Err(CommandError::Execution("Object is locked".into()));
        }
        if asset.selection_overlay_color == self.color {
            return Err(CommandError::NoChange("Selection color unchanged".into()));
        }
        asset.selection_overlay_color = self.color;
        Ok(())
    }
}
