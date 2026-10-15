//! Operandos e operações booleanas.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Define o operando B das operações booleanas.
    pub fn set_boolean_operand(&mut self, id: &str) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if !self.state.project.assets.iter().any(|a| a.id == asset) {
            return false;
        }
        self.state.session.tools.boolean_operand = Some(asset);
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_BOOLEAN_OPERAND_SET_FUSE_CUT_OR_INTERSECT),
        );
        true
    }

    /// Liga/desliga o cleanup do resultado booleano (quads em vez de triângulos do kernel).
    pub fn set_boolean_cleanup(&mut self, cleanup: bool) -> bool {
        if self.state.session.tools.boolean_cleanup == cleanup {
            return false;
        }
        self.state.session.tools.boolean_cleanup = cleanup;
        self.state.set_status(if cleanup {
            "Clean quads on: coplanar regions are merged after Fuse/Cut/Intersect"
        } else {
            "Clean quads off: the kernel triangulation is kept"
        });
        true
    }

    /// Liga/desliga o modificador **Keep Parts**.
    pub fn set_boolean_keep_parts(&mut self, keep: bool) -> bool {
        if self.state.session.tools.boolean_keep_parts == keep {
            return false;
        }
        self.state.session.tools.boolean_keep_parts = keep;
        self.state.set_status(if keep {
            "Keep Parts on: the operand stays in the scene"
        } else {
            "Keep Parts off: the operand is consumed"
        });
        true
    }

    /// **Join** pelo id canônico.
    pub fn join_operand(&mut self) -> bool {
        match self.execute_core_command("model.join") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn clear_boolean_operand(&mut self) -> bool {
        if self.state.session.tools.boolean_operand.take().is_none() {
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_BOOLEAN_OPERAND_CLEARED),
        );
        true
    }

    /// Executa Fuse/Cut/Intersect pelo id canônico do comando.
    pub fn boolean_op(&mut self, id: &str) -> bool {
        if self.state.session.tools.boolean_operand.is_none()
            && self.state.project.assets.len() == 2
        {
            let active_id = self.state.project.active().map(|a| a.id);
            if let Some(other) = self
                .state
                .project
                .assets
                .iter()
                .find(|a| Some(a.id) != active_id)
            {
                self.state.session.tools.boolean_operand = Some(other.id);
            }
        }
        match self.execute_core_command(id) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }
}
