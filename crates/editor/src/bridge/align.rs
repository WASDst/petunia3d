//! Operações e intents de alinhamento e distribuição 3D de objetos.

use petunia_core::align_commands::{AlignObjectsCmd, DistributeObjectsCmd};
use petunia_project::{AlignAxis, AlignMode, AlignReference};

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Executa o alinhamento dos objetos selecionados segundo eixo, modo e referência canônica.
    pub fn align_objects(
        &mut self,
        axis: AlignAxis,
        mode: AlignMode,
        reference: AlignReference,
    ) -> bool {
        let cmd = AlignObjectsCmd::new(axis, mode, reference);
        match petunia_core::command::CommandDispatcher::dispatch(&mut self.state, &cmd) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Executa a distribuição uniforme do espaçamento entre 3 ou mais objetos ao longo de um eixo.
    pub fn distribute_objects(&mut self, axis: AlignAxis) -> bool {
        let cmd = DistributeObjectsCmd::new(axis);
        match petunia_core::command::CommandDispatcher::dispatch(&mut self.state, &cmd) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }
}
