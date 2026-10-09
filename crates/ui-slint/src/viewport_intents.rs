//! DTOs da boundary Slint → bridge. Unidades explícitas, sem toolkit/GPU/domínio duplicado.

use crate::ViewportPointerPhase;

/// Coordenadas normalizadas da viewport (origem superior esquerda).
/// Capture pode entregar valores fora de 0..1; não limitar arrastes válidos à borda.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedViewportPoint([f32; 2]);

impl NormalizedViewportPoint {
    pub fn new(x: f32, y: f32) -> Option<Self> {
        let point = Self([x, y]);
        (x.is_finite() && y.is_finite() && point.ndc().iter().all(|v| v.is_finite()))
            .then_some(point)
    }

    pub const fn xy(self) -> [f32; 2] {
        self.0
    }

    pub fn ndc(self) -> [f32; 2] {
        [self.0[0] * 2.0 - 1.0, 1.0 - self.0[1] * 2.0]
    }
}

/// Pixels lógicos relativos à viewport, nunca pixels físicos/HiDPI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalViewportPoint([f32; 2]);

impl LogicalViewportPoint {
    pub fn new(x: f32, y: f32) -> Option<Self> {
        (x.is_finite() && y.is_finite()).then_some(Self([x, y]))
    }

    pub const fn xy(self) -> [f32; 2] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionSelectionMode {
    Replace,
    Add,
    Subtract,
}

impl RegionSelectionMode {
    /// Preserva precedência existente: subtract vence quando ambos estão ativos.
    pub const fn from_flags(add: bool, subtract: bool) -> Self {
        if subtract {
            Self::Subtract
        } else if add {
            Self::Add
        } else {
            Self::Replace
        }
    }

    pub const fn flags(self) -> (bool, bool) {
        match self {
            Self::Replace => (false, false),
            Self::Add => (true, false),
            Self::Subtract => (false, true),
        }
    }
}

/// Polígono NDC validado pelo parser existente (tamanho, finitude, três pontos).
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportLassoPolygon(Vec<[f32; 2]>);

impl ViewportLassoPolygon {
    pub fn from_slint_path(path: &str) -> Option<Self> {
        crate::projection::parse_lasso_path(path).map(Self)
    }

    pub fn ndc_points(&self) -> &[[f32; 2]] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewportSelectionIntent {
    Point {
        at: NormalizedViewportPoint,
        extend: bool,
        loop_select: bool,
    },
    Box {
        from: NormalizedViewportPoint,
        to: NormalizedViewportPoint,
        mode: RegionSelectionMode,
    },
    Lasso {
        polygon: ViewportLassoPolygon,
        mode: RegionSelectionMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportHoverIntent {
    At(NormalizedViewportPoint),
    Clear,
}

/// Teclas físicas na boundary; o keymap Application continua atribuindo significado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalPointerModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportToolPointerIntent {
    pub phase: ViewportPointerPhase,
    pub at: LogicalViewportPoint,
    pub modifiers: PhysicalPointerModifiers,
}

impl ViewportToolPointerIntent {
    /// API Slint temporariamente inteira: rejeitar códigos desconhecidos antes de executar.
    pub fn from_slint(
        phase: i32,
        x: f32,
        y: f32,
        modifiers: PhysicalPointerModifiers,
    ) -> Option<Self> {
        let phase = match phase {
            0 => ViewportPointerPhase::Press,
            1 => ViewportPointerPhase::Move,
            2 => ViewportPointerPhase::Release,
            3 => ViewportPointerPhase::Cancel,
            _ => return None,
        };
        Some(Self {
            phase,
            at: LogicalViewportPoint::new(x, y)?,
            modifiers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_units_preserve_capture_and_reject_nonfinite_projection() {
        assert_eq!(
            NormalizedViewportPoint::new(0.25, 0.75).unwrap().ndc(),
            [-0.5, -0.5]
        );
        assert_eq!(
            NormalizedViewportPoint::new(-0.5, 1.5).unwrap().ndc(),
            [-2.0, -2.0]
        );
        assert_eq!(
            LogicalViewportPoint::new(-12.0, 720.0).unwrap().xy(),
            [-12.0, 720.0]
        );
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(NormalizedViewportPoint::new(invalid, 0.5).is_none());
            assert!(LogicalViewportPoint::new(0.5, invalid).is_none());
        }
        assert!(NormalizedViewportPoint::new(f32::MAX, 0.5).is_none());
    }

    #[test]
    fn invalid_wire_phases_cannot_become_a_tool_cancel() {
        for phase in [-1, 4, i32::MAX] {
            assert!(
                ViewportToolPointerIntent::from_slint(
                    phase,
                    1.0,
                    2.0,
                    PhysicalPointerModifiers::default()
                )
                .is_none()
            );
        }
        let modifiers = PhysicalPointerModifiers {
            shift: true,
            ctrl: false,
            alt: true,
        };
        let event = ViewportToolPointerIntent::from_slint(3, -20.0, 100.0, modifiers).unwrap();
        assert_eq!(event.phase, ViewportPointerPhase::Cancel);
        assert_eq!(event.modifiers, modifiers);
        assert_eq!(event.at.xy(), [-20.0, 100.0]);
        assert!(ViewportToolPointerIntent::from_slint(0, f32::NAN, 2.0, modifiers).is_none());
    }

    #[test]
    fn lasso_keeps_existing_bounds_and_subtract_precedence() {
        let polygon = ViewportLassoPolygon::from_slint_path("-1,0;1,0;1,2").unwrap();
        assert_eq!(
            polygon.ndc_points(),
            &[[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0]]
        );
        assert!(ViewportLassoPolygon::from_slint_path("0,0;1,0").is_none());
        assert!(ViewportLassoPolygon::from_slint_path("0,0;NaN,0;1,1").is_none());
        assert_eq!(
            RegionSelectionMode::from_flags(true, true),
            RegionSelectionMode::Subtract
        );
    }
}
