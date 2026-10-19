//! Paletas do seletor de cor: criar, renomear, apagar e editar cores, sempre com
//! Undo. `palette = None` é a paleta do projeto (RGB; o alfa é descartado); as
//! demais são paletas nomeadas (RGBA).

use petunia_project::ProjectChanges;
use petunia_project::palette::{
    MAX_PALETTE_COLORS, MAX_PALETTE_NAME_CHARS, MAX_PALETTES, NamedPalette, valid_rgba,
};
use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

/// Operação sobre as paletas.
#[derive(Clone, Debug, PartialEq)]
pub enum PaletteOp {
    Create {
        name: String,
    },
    Rename {
        palette: Uuid,
        name: String,
    },
    Delete {
        palette: Uuid,
    },
    AddColor {
        palette: Option<Uuid>,
        color: [f32; 4],
    },
    SetColor {
        palette: Option<Uuid>,
        index: usize,
        color: [f32; 4],
    },
    RemoveColor {
        palette: Option<Uuid>,
        index: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaletteCmd {
    pub op: PaletteOp,
}

fn checked_name(name: &str) -> Result<String, CommandError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_PALETTE_NAME_CHARS {
        return Err(CommandError::Execution(format!(
            "o nome da paleta precisa ter de 1 a {MAX_PALETTE_NAME_CHARS} caracteres"
        )));
    }
    Ok(name.to_string())
}

fn checked_color(color: &[f32; 4]) -> Result<[f32; 4], CommandError> {
    if valid_rgba(color) {
        Ok(*color)
    } else {
        Err(CommandError::Execution(
            "cor inválida (precisa estar em 0..1)".into(),
        ))
    }
}

/// Aplica a operação ao documento (mesmo código para o ensaio e a execução).
fn apply(state: &mut CommandContext<'_>, op: &PaletteOp) -> Result<String, CommandError> {
    let project = &mut state.document.project;
    match op {
        PaletteOp::Create { name } => {
            let name = checked_name(name)?;
            if project.palettes.len() >= MAX_PALETTES {
                return Err(CommandError::Execution("paletas demais".into()));
            }
            if project.palettes.iter().any(|p| p.name == name) {
                return Err(CommandError::Execution(
                    "já existe uma paleta com esse nome".into(),
                ));
            }
            project.palettes.push(NamedPalette::new(name.clone()));
            Ok(format!("Paleta {name} criada"))
        }
        PaletteOp::Rename { palette, name } => {
            let name = checked_name(name)?;
            if project
                .palettes
                .iter()
                .any(|p| p.name == name && p.id != *palette)
            {
                return Err(CommandError::Execution(
                    "já existe uma paleta com esse nome".into(),
                ));
            }
            let target = project
                .palettes
                .iter_mut()
                .find(|p| p.id == *palette)
                .ok_or_else(|| CommandError::StaleReference(palette.to_string()))?;
            if target.name == name {
                return Err(CommandError::NoChange("o nome é o mesmo".into()));
            }
            target.name = name.clone();
            Ok(format!("Paleta renomeada para {name}"))
        }
        PaletteOp::Delete { palette } => {
            let before = project.palettes.len();
            project.palettes.retain(|p| p.id != *palette);
            if project.palettes.len() == before {
                return Err(CommandError::StaleReference(palette.to_string()));
            }
            Ok("Paleta apagada".into())
        }
        PaletteOp::AddColor { palette, color } => {
            let color = checked_color(color)?;
            match palette {
                None => {
                    if project.palette.len() >= MAX_PALETTE_COLORS {
                        return Err(CommandError::Execution("a paleta está cheia".into()));
                    }
                    project.palette.push([color[0], color[1], color[2]]);
                    state.document.palette = state.document.project.palette.clone();
                }
                Some(id) => {
                    let target = project
                        .palettes
                        .iter_mut()
                        .find(|p| p.id == *id)
                        .ok_or_else(|| CommandError::StaleReference(id.to_string()))?;
                    if target.colors.len() >= MAX_PALETTE_COLORS {
                        return Err(CommandError::Execution("a paleta está cheia".into()));
                    }
                    target.colors.push(color);
                }
            }
            Ok("Cor adicionada à paleta".into())
        }
        PaletteOp::SetColor {
            palette,
            index,
            color,
        } => {
            let color = checked_color(color)?;
            match palette {
                None => {
                    let slot = project
                        .palette
                        .get_mut(*index)
                        .ok_or_else(|| CommandError::Execution("cor fora da paleta".into()))?;
                    *slot = [color[0], color[1], color[2]];
                    state.document.palette = state.document.project.palette.clone();
                }
                Some(id) => {
                    let target = project
                        .palettes
                        .iter_mut()
                        .find(|p| p.id == *id)
                        .ok_or_else(|| CommandError::StaleReference(id.to_string()))?;
                    let slot = target
                        .colors
                        .get_mut(*index)
                        .ok_or_else(|| CommandError::Execution("cor fora da paleta".into()))?;
                    *slot = color;
                }
            }
            Ok("Cor da paleta alterada".into())
        }
        PaletteOp::RemoveColor { palette, index } => {
            match palette {
                None => {
                    if *index >= project.palette.len() {
                        return Err(CommandError::Execution("cor fora da paleta".into()));
                    }
                    project.palette.remove(*index);
                    state.document.palette = state.document.project.palette.clone();
                }
                Some(id) => {
                    let target = project
                        .palettes
                        .iter_mut()
                        .find(|p| p.id == *id)
                        .ok_or_else(|| CommandError::StaleReference(id.to_string()))?;
                    if *index >= target.colors.len() {
                        return Err(CommandError::Execution("cor fora da paleta".into()));
                    }
                    target.colors.remove(*index);
                }
            }
            Ok("Cor removida da paleta".into())
        }
    }
}

impl Command for PaletteCmd {
    fn label(&self) -> &'static str {
        match self.op {
            PaletteOp::Create { .. } => "create palette",
            PaletteOp::Rename { .. } => "rename palette",
            PaletteOp::Delete { .. } => "delete palette",
            PaletteOp::AddColor { .. } => "add palette color",
            PaletteOp::SetColor { .. } => "edit palette color",
            PaletteOp::RemoveColor { .. } => "remove palette color",
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        let named = |id: &Uuid| project.palettes.iter().find(|p| p.id == *id);
        match &self.op {
            PaletteOp::Create { name } => {
                if name.trim().is_empty() {
                    Err("The palette needs a name")
                } else if project.palettes.len() >= MAX_PALETTES {
                    Err("Too many palettes")
                } else {
                    Ok(())
                }
            }
            PaletteOp::Rename { palette, .. } | PaletteOp::Delete { palette } => named(palette)
                .map(|_| ())
                .ok_or("The palette no longer exists"),
            PaletteOp::AddColor { palette, color } => {
                if !valid_rgba(color) {
                    return Err("Invalid color");
                }
                match palette {
                    Some(id) => named(id).map(|_| ()).ok_or("The palette no longer exists"),
                    None => Ok(()),
                }
            }
            PaletteOp::SetColor { palette, index, .. }
            | PaletteOp::RemoveColor { palette, index } => {
                let len = match palette {
                    Some(id) => named(id)
                        .map(|p| p.colors.len())
                        .ok_or("The palette no longer exists")?,
                    None => project.palette.len(),
                };
                if *index < len {
                    Ok(())
                } else {
                    Err("The color is not in the palette")
                }
            }
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let message = apply(state, &self.op)?;
        state.mark_dirty();
        state.set_status(message);
        Ok(())
    }
}
