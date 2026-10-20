//! Junção de objetos selecionados. Operações booleanas existem só como nós do
//! Operation Graph; não há operando global nem comando destrutivo direto.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// **Join** pelo id canônico: funde os demais selecionados no ativo.
    pub fn join_selected(&mut self) -> bool {
        match self.execute_core_command("model.join") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }
}
