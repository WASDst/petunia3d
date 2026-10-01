//! Layout de vistas 3D: 1, 2 ou 4 (estudo `docs/development/multi-viewport-study.md`, F1).
//!
//! Modelo **puro** (sem janela, sem renderer): calcula os retângulos de cada
//! vista, recua o layout quando a área não comporta os mínimos do capítulo 36 e
//! escolhe qual vista passiva re-renderizar no quadro. Fica pronto e testado
//! antes de qualquer mudança visível; a ligação com o markup e com o renderer
//! (F0/F3) depende da decisão registrada no estudo.

use petunia_core::ViewPreset;

/// Largura mínima lógica por vista (capítulo 36).
pub const MIN_VIEW_WIDTH: f32 = 480.0;
/// Altura mínima lógica por vista.
pub const MIN_VIEW_HEIGHT: f32 = 360.0;
/// Espaço entre vistas, em px lógicos.
pub const VIEW_GAP: f32 = 4.0;

/// Quantas vistas 3D aparecem e como se distribuem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ViewLayout {
    #[default]
    Single,
    /// Duas vistas lado a lado.
    Split2,
    /// Quatro vistas em grade 2×2.
    Quad,
}

/// Retângulo de uma vista (px lógicos, origem no canto superior esquerdo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ViewLayout {
    pub fn count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Split2 => 2,
            Self::Quad => 4,
        }
    }

    /// Id estável usado pela UI e pelo arquivo de preferências.
    pub fn id(self) -> &'static str {
        match self {
            Self::Single => "1",
            Self::Split2 => "2",
            Self::Quad => "4",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "1" => Some(Self::Single),
            "2" => Some(Self::Split2),
            "4" => Some(Self::Quad),
            _ => None,
        }
    }

    /// Retângulos das vistas dentro de `width × height`. A vista `0` é a principal
    /// (única editável); no Quad ela ocupa o quadrante superior esquerdo.
    pub fn rects(self, width: f32, height: f32) -> Vec<ViewRect> {
        let (w, h) = (width.max(0.0), height.max(0.0));
        match self {
            Self::Single => vec![ViewRect {
                x: 0.0,
                y: 0.0,
                w,
                h,
            }],
            Self::Split2 => {
                let half = ((w - VIEW_GAP) * 0.5).max(0.0);
                vec![
                    ViewRect {
                        x: 0.0,
                        y: 0.0,
                        w: half,
                        h,
                    },
                    ViewRect {
                        x: half + VIEW_GAP,
                        y: 0.0,
                        w: half,
                        h,
                    },
                ]
            }
            Self::Quad => {
                let cw = ((w - VIEW_GAP) * 0.5).max(0.0);
                let ch = ((h - VIEW_GAP) * 0.5).max(0.0);
                let (cx, cy) = (cw + VIEW_GAP, ch + VIEW_GAP);
                vec![
                    ViewRect {
                        x: 0.0,
                        y: 0.0,
                        w: cw,
                        h: ch,
                    },
                    ViewRect {
                        x: cx,
                        y: 0.0,
                        w: cw,
                        h: ch,
                    },
                    ViewRect {
                        x: 0.0,
                        y: cy,
                        w: cw,
                        h: ch,
                    },
                    ViewRect {
                        x: cx,
                        y: cy,
                        w: cw,
                        h: ch,
                    },
                ]
            }
        }
    }

    /// Todas as vistas respeitam os mínimos de largura e altura.
    pub fn fits(self, width: f32, height: f32) -> bool {
        self.rects(width, height)
            .iter()
            .all(|r| r.w >= MIN_VIEW_WIDTH && r.h >= MIN_VIEW_HEIGHT)
    }

    /// O maior layout (≤ `self`) que cabe: 4 → 2 → 1. Nunca empilha vistas ilegíveis.
    pub fn downgrade(self, width: f32, height: f32) -> Self {
        let mut layout = self;
        loop {
            if layout.fits(width, height) {
                return layout;
            }
            layout = match layout {
                Self::Quad => Self::Split2,
                Self::Split2 | Self::Single => return Self::Single,
            };
        }
    }

    /// Presets iniciais por vista: a principal em perspectiva e, no Quad, as três
    /// ortográficas clássicas.
    pub fn default_presets(self) -> Vec<ViewPreset> {
        match self {
            Self::Single => vec![ViewPreset::Persp],
            Self::Split2 => vec![ViewPreset::Persp, ViewPreset::Front],
            Self::Quad => vec![
                ViewPreset::Persp,
                ViewPreset::Top,
                ViewPreset::Front,
                ViewPreset::Right,
            ],
        }
    }
}

/// Escolhe qual vista **passiva** re-renderizar no quadro (no máximo uma).
///
/// `dirty[i]` diz se a vista `i` mudou; a `0` é a ativa e fica de fora. O
/// round-robin evita que uma vista sempre suja esfomeie as outras; `hovered`
/// (vista sob o ponteiro) tem prioridade.
#[derive(Clone, Debug, Default)]
pub struct PassiveScheduler {
    cursor: usize,
}

impl PassiveScheduler {
    pub fn next(&mut self, dirty: &[bool], hovered: Option<usize>) -> Option<usize> {
        let n = dirty.len();
        if n < 2 {
            return None;
        }
        if let Some(h) = hovered
            && h > 0
            && h < n
            && dirty[h]
        {
            return Some(h);
        }
        for step in 0..(n - 1) {
            let i = 1 + (self.cursor + step) % (n - 1);
            if dirty[i] {
                self.cursor = i % (n - 1);
                return Some(i);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rects_tile_the_area_without_overlap() {
        for layout in [ViewLayout::Single, ViewLayout::Split2, ViewLayout::Quad] {
            let rects = layout.rects(1920.0, 1080.0);
            assert_eq!(rects.len(), layout.count());
            for (i, a) in rects.iter().enumerate() {
                assert!(
                    a.x >= 0.0
                        && a.y >= 0.0
                        && a.x + a.w <= 1920.0 + 0.01
                        && a.y + a.h <= 1080.0 + 0.01
                );
                for b in rects.iter().skip(i + 1) {
                    let separate = a.x + a.w <= b.x + 0.01
                        || b.x + b.w <= a.x + 0.01
                        || a.y + a.h <= b.y + 0.01
                        || b.y + b.h <= a.y + 0.01;
                    assert!(separate, "{a:?} sobrepõe {b:?}");
                }
            }
        }
    }

    #[test]
    fn layouts_degrade_to_what_fits() {
        // 1920×1080: o quad dá ~958×538, acima dos mínimos
        assert_eq!(ViewLayout::Quad.downgrade(1920.0, 1080.0), ViewLayout::Quad);
        // 1280×720: o quad não cabe (alturas de ~358), o split cabe (638×720)
        assert_eq!(
            ViewLayout::Quad.downgrade(1280.0, 720.0),
            ViewLayout::Split2
        );
        // 900×600: nem o split cabe (448 de largura)
        assert_eq!(ViewLayout::Quad.downgrade(900.0, 600.0), ViewLayout::Single);
        assert_eq!(ViewLayout::Single.downgrade(10.0, 10.0), ViewLayout::Single);
    }

    #[test]
    fn ids_round_trip() {
        for layout in [ViewLayout::Single, ViewLayout::Split2, ViewLayout::Quad] {
            assert_eq!(ViewLayout::from_id(layout.id()), Some(layout));
        }
        assert_eq!(ViewLayout::from_id("3"), None);
    }

    #[test]
    fn quad_defaults_are_persp_top_front_right() {
        assert_eq!(
            ViewLayout::Quad.default_presets(),
            vec![
                ViewPreset::Persp,
                ViewPreset::Top,
                ViewPreset::Front,
                ViewPreset::Right
            ]
        );
    }

    #[test]
    fn scheduler_renders_at_most_one_passive_view_and_never_starves() {
        let mut s = PassiveScheduler::default();
        let dirty = [true, true, true, true];
        let first = s.next(&dirty, None).unwrap();
        let second = s.next(&dirty, None).unwrap();
        let third = s.next(&dirty, None).unwrap();
        let mut seen = vec![first, second, third];
        seen.sort_unstable();
        assert_eq!(seen, vec![1, 2, 3], "round-robin visita todas as passivas");
        assert!(!seen.contains(&0), "a vista ativa nunca entra na fila");
    }

    #[test]
    fn hovered_view_has_priority_and_clean_views_are_skipped() {
        let mut s = PassiveScheduler::default();
        assert_eq!(s.next(&[true, true, true, true], Some(3)), Some(3));
        assert_eq!(s.next(&[true, false, false, false], None), None);
        assert_eq!(s.next(&[true, false, true, false], None), Some(2));
        assert_eq!(s.next(&[true], None), None);
    }
}
