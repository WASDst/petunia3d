//! Pilha de modificadores.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Asset displayed by a section: the pinned asset when set and resolvable,
    /// otherwise the active selection. Fail-safe fallback, never panics.
    /// Asset exibido por uma seção: o asset fixado quando definido e resolvível,
    /// senão a seleção ativa. Fallback fail-safe, nunca pânico.
    pub fn section_asset(
        &self,
        section: petunia_config::InspectorSectionId,
    ) -> Option<&petunia_project::Asset> {
        self.section_layouts[section_layout::section_index(section)]
            .pinned_asset
            .as_deref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .and_then(|id| {
                self.state
                    .project
                    .assets
                    .iter()
                    .find(|asset| asset.id == id)
            })
            .or_else(|| self.state.project.active())
    }

    /// Index of the asset owning a modifier id (any asset, not just the active
    /// one) so sections pinned to another asset act on the displayed stack.
    /// Modifier ids are unique; the active path behaves exactly as before.
    /// Índice do asset dono de um modifier (qualquer asset, não só o ativo)
    /// para seções fixadas agirem na pilha exibida. Ids são únicos; o caminho
    /// ativo comporta-se exatamente como antes.
    pub(crate) fn find_modifier_owner(&self, id: uuid::Uuid) -> Option<usize> {
        self.state
            .project
            .assets
            .iter()
            .position(|asset| asset.modifiers.iter().any(|modifier| modifier.id == id))
    }

    /// Despacha uma operação da pilha; recusa (inclusive "nada mudaria") não
    /// empilha Undo e vira status.
    fn dispatch_modifier(&mut self, op: petunia_core::ModifierOp) -> bool {
        match self.state.dispatch(&petunia_core::ModifierCmd { op }) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn add_modifier(&mut self, kind: &str) -> bool {
        // Creation follows the displayed Modifiers section: pinned asset when
        // set, active selection otherwise. / A criação segue a seção exibida.
        let Some(asset) = self
            .section_asset(petunia_config::InspectorSectionId::Modifiers)
            .map(|asset| asset.id)
        else {
            return false;
        };
        let instance = match kind {
            "mirror" => petunia_project::ModifierInstance::mirror(0, 0.001),
            "symmetry" => petunia_project::ModifierInstance::symmetry(0, true, 0.001),
            _ => return false,
        };
        self.dispatch_modifier(petunia_core::ModifierOp::Add { asset, instance })
    }

    pub fn set_modifier_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Ok(modifier) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        // Já no estado pedido: nada a fazer, sem status de erro.
        let unchanged = self.find_modifier_owner(modifier).is_none_or(|index| {
            self.state.project.assets[index]
                .modifiers
                .iter()
                .any(|item| item.id == modifier && item.enabled == enabled)
        });
        if unchanged {
            return false;
        }
        self.dispatch_modifier(petunia_core::ModifierOp::SetEnabled { modifier, enabled })
    }

    pub fn remove_modifier(&mut self, id: &str) -> bool {
        let Ok(modifier) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if self.find_modifier_owner(modifier).is_none() {
            return false;
        }
        self.dispatch_modifier(petunia_core::ModifierOp::Remove { modifier })
    }

    pub fn move_modifier(&mut self, id: &str, direction: i32) -> bool {
        let Ok(modifier) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(index) = self.find_modifier_owner(modifier) else {
            return false;
        };
        // Na borda: o botão não faz nada (sem status de erro), como antes.
        let stack = &self.state.project.assets[index].modifiers;
        let position = stack.iter().position(|item| item.id == modifier);
        let movable = position.is_some_and(|m| {
            if direction < 0 {
                m > 0
            } else {
                m + 1 < stack.len()
            }
        });
        if !movable {
            return false;
        }
        self.dispatch_modifier(petunia_core::ModifierOp::Move {
            modifier,
            up: direction < 0,
        })
    }

    pub fn set_modifier_axis(&mut self, id: &str, axis: i32) -> bool {
        let Ok(modifier) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if self.find_modifier_owner(modifier).is_none() {
            return false;
        }
        self.dispatch_modifier(petunia_core::ModifierOp::SetAxis {
            modifier,
            axis: axis.clamp(0, 2) as usize,
        })
    }

    pub fn set_modifier_direction(&mut self, id: &str, direction_value: bool) -> bool {
        let Ok(modifier) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if self.find_modifier_owner(modifier).is_none() {
            return false;
        }
        self.dispatch_modifier(petunia_core::ModifierOp::SetDirection {
            modifier,
            positive_to_negative: direction_value,
        })
    }

    pub fn apply_modifier(&mut self, id: &str) -> bool {
        let Ok(modifier) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if self.find_modifier_owner(modifier).is_none() {
            return false;
        }
        self.dispatch_modifier(petunia_core::ModifierOp::Apply { modifier })
    }
}
