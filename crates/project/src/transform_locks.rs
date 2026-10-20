//! Travas por canal do transform do objeto (ADR-C16, B7).
//!
//! Cada canal (posição, rotação, escala) × eixo (X, Y, Z) pode ser travado. A
//! trava é dado do documento: viaja no arquivo, entra no Undo e vale para todo
//! caminho que escreve o transform do objeto (gizmo, G/R/S, campos e atalhos).
//! A rotação trava pelos ângulos de Euler XYZ autorais (`Asset::rotation`), que
//! são o que a pessoa lê e digita nos campos.

use glam::{EulerRot, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::object_transform::ObjectTransform;

/// Canal do transform travável.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransformChannel {
    Position,
    Rotation,
    Scale,
}

impl TransformChannel {
    pub const ALL: [Self; 3] = [Self::Position, Self::Rotation, Self::Scale];

    pub fn id(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::Rotation => "rotation",
            Self::Scale => "scale",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|channel| channel.id() == id)
    }

    /// Nome para mensagens de recusa.
    pub fn label(self) -> &'static str {
        match self {
            Self::Position => "Posição",
            Self::Rotation => "Rotação",
            Self::Scale => "Escala",
        }
    }
}

/// Nome do eixo para mensagens (`0 → "X"`).
pub fn axis_label(axis: usize) -> &'static str {
    match axis {
        0 => "X",
        1 => "Y",
        2 => "Z",
        _ => "?",
    }
}

/// Travas de um objeto. Todas desligadas por padrão.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TransformLocks {
    pub position: [bool; 3],
    pub rotation: [bool; 3],
    pub scale: [bool; 3],
}

impl TransformLocks {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn channel(&self, channel: TransformChannel) -> [bool; 3] {
        match channel {
            TransformChannel::Position => self.position,
            TransformChannel::Rotation => self.rotation,
            TransformChannel::Scale => self.scale,
        }
    }

    /// Eixo fora de 0..3 é ignorado (dado hostil nunca indexa fora).
    pub fn is_locked(&self, channel: TransformChannel, axis: usize) -> bool {
        self.channel(channel).get(axis).copied().unwrap_or(false)
    }

    /// Retorna `false` quando o eixo não existe.
    pub fn set(&mut self, channel: TransformChannel, axis: usize, locked: bool) -> bool {
        let slots = match channel {
            TransformChannel::Position => &mut self.position,
            TransformChannel::Rotation => &mut self.rotation,
            TransformChannel::Scale => &mut self.scale,
        };
        let Some(slot) = slots.get_mut(axis) else {
            return false;
        };
        *slot = locked;
        true
    }

    /// Primeiro canal travado que mudaria de `before` para `after` (valores
    /// autorais: posição, Euler em graus, escala). Usado para recusar uma edição
    /// digitada com o motivo, em vez de ignorá-la em silêncio.
    pub fn first_violation(
        &self,
        before: [[f32; 3]; 3],
        after: [[f32; 3]; 3],
    ) -> Option<(TransformChannel, usize)> {
        const EPSILON: f32 = 1e-5;
        for (index, channel) in TransformChannel::ALL.into_iter().enumerate() {
            let locks = self.channel(channel);
            for axis in 0..3 {
                let (Some(was), Some(now)) = (
                    before.get(index).and_then(|values| values.get(axis)),
                    after.get(index).and_then(|values| values.get(axis)),
                ) else {
                    continue;
                };
                if locks.get(axis).copied().unwrap_or(false) && (was - now).abs() > EPSILON {
                    return Some((channel, axis));
                }
            }
        }
        None
    }

    /// Devolve `next` com os canais travados restaurados de `previous`.
    /// `previous_euler_deg` é a rotação autoral de `previous`; o resultado traz o
    /// Euler coerente com o transform devolvido. Sem travas, `next` passa intacto.
    pub fn constrain(
        &self,
        previous: &ObjectTransform,
        previous_euler_deg: [f32; 3],
        next: ObjectTransform,
    ) -> (ObjectTransform, Option<[f32; 3]>) {
        if self.is_empty() {
            return (next, None);
        }
        let mut out = next;
        for axis in 0..3 {
            if self.position[axis] {
                out.translation[axis] = previous.translation[axis];
            }
            if self.scale[axis] {
                out.scale[axis] = previous.scale[axis];
            }
        }
        if !self.rotation.iter().any(|locked| *locked) {
            return (out, None);
        }
        let (x, y, z) = next.rotation().to_euler(EulerRot::XYZ);
        let mut euler = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
        for axis in 0..3 {
            if self.rotation[axis] {
                euler[axis] = previous_euler_deg[axis];
            }
        }
        if !euler.iter().all(|value| value.is_finite()) {
            return (*previous, Some(previous_euler_deg));
        }
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler[0].to_radians(),
            euler[1].to_radians(),
            euler[2].to_radians(),
        );
        out = ObjectTransform::from_trs(
            Vec3::from_array(out.translation),
            rotation,
            Vec3::from_array(out.scale),
        );
        (out, Some(euler))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moved() -> (ObjectTransform, ObjectTransform) {
        let before = ObjectTransform::IDENTITY;
        let after = ObjectTransform::from_trs(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_euler(EulerRot::XYZ, 0.3, 0.2, 0.1),
            Vec3::new(2.0, 2.0, 2.0),
        );
        (before, after)
    }

    #[test]
    fn no_locks_pass_the_transform_through() {
        let (before, after) = moved();
        let (out, euler) = TransformLocks::default().constrain(&before, [0.0; 3], after);
        assert_eq!(out, after);
        assert!(euler.is_none());
    }

    #[test]
    fn locked_channels_keep_their_previous_values() {
        let (before, after) = moved();
        let mut locks = TransformLocks::default();
        assert!(locks.set(TransformChannel::Position, 1, true));
        assert!(locks.set(TransformChannel::Scale, 0, true));
        assert!(locks.set(TransformChannel::Rotation, 2, true));
        let (out, euler) = locks.constrain(&before, [0.0; 3], after);
        assert_eq!(out.translation, [1.0, 0.0, 3.0]);
        assert_eq!(out.scale, [1.0, 2.0, 2.0]);
        let euler = euler.expect("rotação travada devolve o Euler");
        assert!(euler[2].abs() < 1e-4, "Z travado em 0°: {euler:?}");
        assert!((euler[0] - 0.3f32.to_degrees()).abs() < 1e-3);
    }

    #[test]
    fn hostile_axes_never_index_out_of_range() {
        let mut locks = TransformLocks::default();
        assert!(!locks.set(TransformChannel::Position, 7, true));
        assert!(!locks.is_locked(TransformChannel::Position, 7));
        assert!(locks.is_empty());
    }

    #[test]
    fn violation_names_the_first_locked_channel_that_would_change() {
        let mut locks = TransformLocks::default();
        locks.set(TransformChannel::Rotation, 1, true);
        let before = [[0.0; 3], [0.0, 10.0, 0.0], [1.0; 3]];
        let mut after = before;
        after[0][0] = 5.0;
        assert_eq!(
            locks.first_violation(before, after),
            None,
            "X da posição está livre"
        );
        after[1][1] = 20.0;
        assert_eq!(
            locks.first_violation(before, after),
            Some((TransformChannel::Rotation, 1))
        );
    }
}
