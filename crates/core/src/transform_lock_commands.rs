//! Travar e destravar canais do transform de objetos (ADR-C16, B7), com Undo.
//!
//! A trava é dado do documento (`Asset::transform_locks`); quem a respeita é cada
//! caminho que escreve o transform: o modal de objeto (gizmo e G/R/S), o comando
//! de transform digitado e o nudge por setas.

use petunia_project::transform_locks::axis_label;
use petunia_project::{ProjectChanges, TransformChannel};

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};

/// Trava (ou destrava) `channel`/`axis` no objeto ativo e nos demais selecionados.
#[derive(Clone, Debug, PartialEq)]
pub struct SetTransformLockCmd {
    pub channel: TransformChannel,
    pub axis: usize,
    pub locked: bool,
}

impl Command for SetTransformLockCmd {
    fn label(&self) -> &'static str {
        if self.locked {
            "lock transform channel"
        } else {
            "unlock transform channel"
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if self.axis > 2 {
            return Err("Invalid axis");
        }
        if state.document.active().is_none() {
            return Err("No active object");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if self.axis > 2 {
            return Err(CommandError::Execution(format!(
                "eixo inválido: {}",
                self.axis
            )));
        }
        let active = state
            .document
            .active()
            .map(|asset| asset.id)
            .ok_or(CommandError::NoActiveAsset)?;
        let selected = state.editor.selection.assets.clone();
        let mut changed = 0usize;
        for asset in state
            .document
            .assets
            .iter_mut()
            .filter(|asset| asset.id == active || selected.contains(&asset.id))
        {
            if asset.transform_locks.is_locked(self.channel, self.axis) != self.locked
                && asset
                    .transform_locks
                    .set(self.channel, self.axis, self.locked)
            {
                changed += 1;
            }
        }
        if changed == 0 {
            return Err(CommandError::NoChange("a trava já está assim".into()));
        }
        state.mark_dirty();
        state.set_status(format!(
            "{} {} {}",
            self.channel.label(),
            axis_label(self.axis),
            if self.locked { "travada" } else { "destravada" }
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppState, ModalKind, SetAssetTransformCmd};
    use glam::Vec3;

    fn lock(state: &mut AppState, channel: TransformChannel, axis: usize, locked: bool) {
        state
            .dispatch(&SetTransformLockCmd {
                channel,
                axis,
                locked,
            })
            .expect("trava aplicada");
    }

    #[test]
    fn locking_is_document_data_with_undo() {
        let mut state = AppState::new("en");
        lock(&mut state, TransformChannel::Rotation, 2, true);
        let locks = state.project.active().expect("cubo").transform_locks;
        assert!(locks.is_locked(TransformChannel::Rotation, 2));
        assert!(state.undo(), "travar entra no Undo");
        let locks = state.project.active().expect("cubo").transform_locks;
        assert!(locks.is_empty());
        assert!(
            state
                .dispatch(&SetTransformLockCmd {
                    channel: TransformChannel::Scale,
                    axis: 9,
                    locked: true,
                })
                .is_err(),
            "eixo hostil é recusado"
        );
    }

    #[test]
    fn object_modal_keeps_locked_axes_in_place() {
        let mut state = AppState::new("en");
        lock(&mut state, TransformChannel::Position, 1, true);
        state.begin_modal(ModalKind::Move).expect("G no objeto");
        state
            .update_modal_components(Vec3::new(1.0, 2.0, 3.0))
            .expect("prévia");
        assert!(state.commit_modal());
        let translation = state.project.active().expect("cubo").transform.translation;
        assert!((translation[0] - 1.0).abs() < 1e-5, "{translation:?}");
        assert!(
            translation[1].abs() < 1e-5,
            "Y travado não anda: {translation:?}"
        );
        assert!((translation[2] - 3.0).abs() < 1e-5, "{translation:?}");
    }

    #[test]
    fn typed_value_on_a_locked_channel_is_refused_with_the_reason() {
        let mut state = AppState::new("en");
        lock(&mut state, TransformChannel::Scale, 0, true);
        let error = state
            .dispatch(&SetAssetTransformCmd {
                scale: Some([2.0, 1.0, 1.0]),
                ..Default::default()
            })
            .expect_err("escala X travada");
        assert!(error.to_string().contains("Escala X"), "{error}");
        state
            .dispatch(&SetAssetTransformCmd {
                scale: Some([1.0, 3.0, 1.0]),
                ..Default::default()
            })
            .expect("Y está livre");
    }
}
