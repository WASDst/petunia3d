//! Petunia3D — Motor de Edição Proporcional (P3D-039).
//!
//! Fornece cálculo determinístico de influência/falloff por distância ou raio,
//! permitindo deformações orgânicas suaves sem acoplamento com a interface.

use serde::{Deserialize, Serialize};

/// Tipo de curva de decaimento (falloff) proporcional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProportionalFalloff {
    #[default]
    Smooth,
    Linear,
    Sphere,
    Sharp,
    Constant,
}

impl ProportionalFalloff {
    pub fn all() -> [Self; 5] {
        [
            Self::Smooth,
            Self::Linear,
            Self::Sphere,
            Self::Sharp,
            Self::Constant,
        ]
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Smooth => "proportional.smooth",
            Self::Linear => "proportional.linear",
            Self::Sphere => "proportional.sphere",
            Self::Sharp => "proportional.sharp",
            Self::Constant => "proportional.constant",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
            Self::Linear => "Linear",
            Self::Sphere => "Sphere",
            Self::Sharp => "Sharp",
            Self::Constant => "Constant",
        }
    }
}

/// Configurações de edição proporcional da sessão.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProportionalSettings {
    pub enabled: bool,
    pub radius: f32,
    pub falloff: ProportionalFalloff,
}

impl Default for ProportionalSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 1.5,
            falloff: ProportionalFalloff::Smooth,
        }
    }
}

/// Calcula o peso de influência [0.0, 1.0] para um elemento a uma certa `distance` do centro.
pub fn calculate_falloff_weight(distance: f32, radius: f32, falloff: ProportionalFalloff) -> f32 {
    let r = radius.max(1e-4);
    if distance <= 0.0 {
        return 1.0;
    }
    if distance >= r {
        return 0.0;
    }

    let t = (distance / r).clamp(0.0, 1.0);

    match falloff {
        ProportionalFalloff::Constant => 1.0,
        ProportionalFalloff::Linear => 1.0 - t,
        ProportionalFalloff::Sharp => (1.0 - t) * (1.0 - t),
        ProportionalFalloff::Sphere => (1.0 - t * t).max(0.0).sqrt(),
        ProportionalFalloff::Smooth => {
            // Curva suave de Hermite: 3*(1-t)^2 - 2*(1-t)^3 (tangente zero em t=0 e t=1)
            let inv_t = 1.0 - t;
            inv_t * inv_t * (3.0 - 2.0 * inv_t)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_falloff_boundaries() {
        for falloff in ProportionalFalloff::all() {
            let w_origin = calculate_falloff_weight(0.0, 2.0, falloff);
            assert_eq!(w_origin, 1.0, "{falloff:?} at origin must be 1.0");

            let w_outside = calculate_falloff_weight(2.5, 2.0, falloff);
            assert_eq!(w_outside, 0.0, "{falloff:?} outside radius must be 0.0");

            let w_boundary = calculate_falloff_weight(2.0, 2.0, falloff);
            assert_eq!(w_boundary, 0.0, "{falloff:?} at boundary must be 0.0");
        }
    }

    #[test]
    fn test_falloff_monotonicity() {
        let r = 2.0;
        let d1 = 0.5;
        let d2 = 1.2;
        for falloff in [
            ProportionalFalloff::Smooth,
            ProportionalFalloff::Linear,
            ProportionalFalloff::Sphere,
            ProportionalFalloff::Sharp,
        ] {
            let w1 = calculate_falloff_weight(d1, r, falloff);
            let w2 = calculate_falloff_weight(d2, r, falloff);
            assert!(
                w1 > w2,
                "{falloff:?} must decrease monotonically as distance increases"
            );
            assert!(w1 > 0.0 && w1 < 1.0);
            assert!(w2 > 0.0 && w2 < 1.0);
        }
    }

    #[test]
    fn test_constant_falloff() {
        let r = 2.0;
        assert_eq!(
            calculate_falloff_weight(0.5, r, ProportionalFalloff::Constant),
            1.0
        );
        assert_eq!(
            calculate_falloff_weight(1.8, r, ProportionalFalloff::Constant),
            1.0
        );
        assert_eq!(
            calculate_falloff_weight(2.1, r, ProportionalFalloff::Constant),
            0.0
        );
    }
}
