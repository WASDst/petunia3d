//! Pilha de modificadores por comando.
//!
//! Cada operação é validada antes de qualquer entrada de Undo (`can_execute`):
//! alterar um eixo que não muda, a direção de um Mirror ou um modificador que
//! não existe é recusado sem empilhar Undo vazio. Ids de modificador são
//! únicos no projeto; o objeto dono é achado pelo id.

use petunia_project::{ModifierInstance, ModifierKind, Project, ProjectChanges};
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

/// (índice do objeto, índice do modificador na pilha).
fn locate(document: &Project, modifier: Uuid) -> Option<(usize, usize)> {
    document
        .assets
        .iter()
        .enumerate()
        .find_map(|(asset, item)| {
            item.modifiers
                .iter()
                .position(|instance| instance.id == modifier)
                .map(|index| (asset, index))
        })
}

/// Operação sobre a pilha de modificadores.
#[derive(Clone, Debug, PartialEq)]
pub enum ModifierOp {
    /// Acrescenta `instance` ao fim da pilha do objeto `asset`.
    Add {
        asset: Uuid,
        instance: ModifierInstance,
    },
    SetEnabled {
        modifier: Uuid,
        enabled: bool,
    },
    Remove {
        modifier: Uuid,
    },
    /// Troca com o vizinho de cima (`up`) ou de baixo.
    Move {
        modifier: Uuid,
        up: bool,
    },
    /// Eixo `0..=2` de Mirror/Symmetry.
    SetAxis {
        modifier: Uuid,
        axis: usize,
    },
    /// Direção da Symmetry; Mirror não tem direção.
    SetDirection {
        modifier: Uuid,
        positive_to_negative: bool,
    },
    /// Grava só este modificador na malha base (como se fosse o primeiro) e o
    /// remove; os demais continuam na pilha. Modificador desligado é recusado.
    Apply {
        modifier: Uuid,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModifierCmd {
    pub op: ModifierOp,
}

impl Default for ModifierCmd {
    fn default() -> Self {
        Self {
            op: ModifierOp::Remove {
                modifier: Uuid::nil(),
            },
        }
    }
}

impl ModifierCmd {
    fn check(&self, document: &Project) -> Result<(), &'static str> {
        const MISSING: &str = "modifier not found";
        match &self.op {
            ModifierOp::Add { asset, .. } => document
                .assets
                .iter()
                .any(|item| item.id == *asset)
                .then_some(())
                .ok_or("object not found"),
            ModifierOp::SetEnabled { modifier, enabled } => {
                let (a, m) = locate(document, *modifier).ok_or(MISSING)?;
                (document.assets[a].modifiers[m].enabled != *enabled)
                    .then_some(())
                    .ok_or("modifier already in that state")
            }
            ModifierOp::Remove { modifier } => {
                locate(document, *modifier).map(|_| ()).ok_or(MISSING)
            }
            ModifierOp::Move { modifier, up } => {
                let (a, m) = locate(document, *modifier).ok_or(MISSING)?;
                let len = document.assets[a].modifiers.len();
                let movable = if *up { m > 0 } else { m + 1 < len };
                movable
                    .then_some(())
                    .ok_or("modifier is already at the edge")
            }
            ModifierOp::SetAxis { modifier, axis } => {
                if *axis > 2 {
                    return Err("axis must be 0, 1 or 2");
                }
                let (a, m) = locate(document, *modifier).ok_or(MISSING)?;
                let current = match document.assets[a].modifiers[m].kind {
                    ModifierKind::Mirror { axis, .. } | ModifierKind::Symmetry { axis, .. } => axis,
                };
                (current != *axis)
                    .then_some(())
                    .ok_or("modifier already uses that axis")
            }
            ModifierOp::SetDirection {
                modifier,
                positive_to_negative,
            } => {
                let (a, m) = locate(document, *modifier).ok_or(MISSING)?;
                match document.assets[a].modifiers[m].kind {
                    ModifierKind::Symmetry {
                        positive_to_negative: current,
                        ..
                    } => (current != *positive_to_negative)
                        .then_some(())
                        .ok_or("modifier already uses that direction"),
                    ModifierKind::Mirror { .. } => Err("mirror has no direction"),
                }
            }
            ModifierOp::Apply { modifier } => {
                let (a, m) = locate(document, *modifier).ok_or(MISSING)?;
                document.assets[a].modifiers[m]
                    .enabled
                    .then_some(())
                    .ok_or("modifier is disabled")
            }
        }
    }
}

impl Command for ModifierCmd {
    fn label(&self) -> &'static str {
        match self.op {
            ModifierOp::Add { .. } => "add modifier",
            ModifierOp::SetEnabled { .. } => "toggle modifier",
            ModifierOp::Remove { .. } => "remove modifier",
            ModifierOp::Move { .. } => "reorder modifier",
            ModifierOp::SetAxis { .. } => "change modifier axis",
            ModifierOp::SetDirection { .. } => "change modifier direction",
            ModifierOp::Apply { .. } => "apply modifier",
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        self.check(state.document)
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        self.check(state.document)
            .map_err(|reason| CommandError::Execution(reason.into()))?;
        let document: &mut Project = state.document;
        match &self.op {
            ModifierOp::Add { asset, instance } => {
                if let Some(item) = document.assets.iter_mut().find(|item| item.id == *asset) {
                    item.modifiers.push(instance.clone());
                }
            }
            op => {
                let modifier = match op {
                    ModifierOp::SetEnabled { modifier, .. }
                    | ModifierOp::Remove { modifier }
                    | ModifierOp::Move { modifier, .. }
                    | ModifierOp::SetAxis { modifier, .. }
                    | ModifierOp::SetDirection { modifier, .. }
                    | ModifierOp::Apply { modifier } => *modifier,
                    ModifierOp::Add { .. } => return Ok(()),
                };
                let (a, m) = locate(document, modifier)
                    .ok_or_else(|| CommandError::StaleReference(modifier.to_string()))?;
                let asset = &mut document.assets[a];
                match op {
                    ModifierOp::SetEnabled { enabled, .. } => asset.modifiers[m].enabled = *enabled,
                    ModifierOp::Remove { .. } => {
                        asset.modifiers.remove(m);
                    }
                    ModifierOp::Move { up, .. } => {
                        let target = if *up { m - 1 } else { m + 1 };
                        asset.modifiers.swap(m, target);
                    }
                    ModifierOp::SetAxis { axis: value, .. } => match &mut asset.modifiers[m].kind {
                        ModifierKind::Mirror { axis, .. } | ModifierKind::Symmetry { axis, .. } => {
                            *axis = *value;
                        }
                    },
                    ModifierOp::SetDirection {
                        positive_to_negative: value,
                        ..
                    } => {
                        if let ModifierKind::Symmetry {
                            positive_to_negative,
                            ..
                        } = &mut asset.modifiers[m].kind
                        {
                            *positive_to_negative = *value;
                        }
                    }
                    ModifierOp::Apply { .. } => {
                        let instance = asset.modifiers.remove(m);
                        let mut mesh = asset.mesh.to_owned_value();
                        match instance.kind {
                            petunia_project::ModifierKind::Mirror { axis, weld } => {
                                mesh.mirror(axis, weld);
                            }
                            petunia_project::ModifierKind::Symmetry {
                                axis,
                                positive_to_negative,
                                weld,
                            } => {
                                mesh.symmetrize(axis, positive_to_negative, weld);
                            }
                        }
                        asset.mesh = mesh.into();
                    }
                    ModifierOp::Add { .. } => {}
                }
            }
        }
        Ok(())
    }
}
