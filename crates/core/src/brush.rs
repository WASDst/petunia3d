//! Brush descriptor unificado (P3D-056/057 — iniciativa Paint).
//!
//! `BrushSettings` é a **única fonte da verdade** dos parâmetros de pincel.
//! Tamanho em pixels de tela (estilo Photoshop): o raio de mundo é derivado
//! na profundidade do hit via `AppState::brush_world_radius`, de modo que o
//! anel de preview e o carimbo real nunca divergem.
//!
//! Este módulo não depende de `egui` nem de UI: módulos de pintura e viewport
//! consomem exatamente o mesmo descriptor.

use serde::{Deserialize, Serialize};

/// Tipo de pincel ativo no motor de pintura (P3D-056 a P3D-060).
///
/// Discriminantes são **append-only**: variantes novas entram no fim para
/// preservar qualquer valor serializado existente. `Line`/`Rectangle`
/// (cap. 15/44) e `Airbrush` (iniciativa Paint) seguiram essa regra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrushType {
    /// Pincel rígido com pixels exatos sem anti-aliasing (P3D-056).
    #[default]
    Pixel,
    /// Pincel com atenuação radial suave controlada por `hardness` (P3D-057).
    Soft,
    /// Borracha que atenua ou remove o canal alfa (P3D-058).
    Eraser,
    /// Balde de preenchimento flood-fill por tolerância (P3D-059).
    Fill,
    /// Amostrador de cor / conta-gotas (P3D-060).
    Eyedropper,
    /// Linha reta entre dois pontos (cap. 15/44: raster op simples).
    Line,
    /// Retângulo preenchido entre dois cantos (cap. 15/44). Idem.
    Rectangle,
    /// Airbrush: acúmulo contínuo modulado por `flow` (iniciativa Paint).
    Airbrush,
}

/// How a 3D brush sample maps onto the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrushProjectionMode {
    #[default]
    Surface,
    ScreenSpace,
}

/// Restricts which surfaces receive paint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrushLock {
    #[default]
    None,
    FirstObject,
    FirstFace,
    SelectedFaces,
}

/// Fill foundation scopes (single algorithm, different seeds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FillScope {
    #[default]
    ConnectedPixels,
    Face,
    SelectedFaces,
    UvIsland,
    Object,
}

impl BrushType {
    /// Pincéis de traço livre (carimbam dabs).
    pub const fn is_free_brush(self) -> bool {
        matches!(
            self,
            Self::Pixel | Self::Soft | Self::Eraser | Self::Airbrush
        )
    }

    /// Formas com ancoragem press→release.
    pub const fn is_shape(self) -> bool {
        matches!(self, Self::Line | Self::Rectangle)
    }

    /// Forma canônica do cursor de preview (iniciativa Paint).
    pub const fn preview_kind(self) -> BrushPreviewKind {
        match self {
            Self::Pixel | Self::Soft | Self::Airbrush => BrushPreviewKind::Ring,
            Self::Eraser => BrushPreviewKind::HollowRing,
            Self::Eyedropper => BrushPreviewKind::Crosshair,
            Self::Fill | Self::Line | Self::Rectangle => BrushPreviewKind::None,
        }
    }
}

/// Como o cursor de preview deve ser desenhado na viewport.
///
/// Contrato puro entre core e UI (iniciativa Paint): o motor decide a
/// **semântica** (o que o pincel faz); a UI decide apenas o traço/pintura.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushPreviewKind {
    /// Anel com preenchimento translúcido na cor do pincel
    /// (Pixel/Soft/Airbrush: o anel é exatamente a pegada do dab).
    Ring,
    /// Anel vazado (Borracha: apaga, não deposita).
    HollowRing,
    /// Cruz de precisão (Conta-gotas: amostra, não pinta).
    Crosshair,
    /// Sem cursor de área (Fill/Line/Rectangle: a própria forma é o preview).
    None,
}

/// Estilo derivado do descriptor para renderizar o cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushPreviewStyle {
    pub kind: BrushPreviewKind,
    /// Cor do anel/cruz em [r,g,b] 0..=1.
    pub tint: [f32; 3],
    /// Alpha do preenchimento interno do anel (força × fluxo).
    pub fill_alpha: f32,
}

impl BrushPreviewStyle {
    /// Deriva o estilo a partir do descriptor e da cor ativa do pincel.
    /// O raio real (mundo ou pixels) sai de `AppState::brush_world_radius`
    /// / `BrushSettings::radius_px` — a UI nunca recalcula por conta própria.
    pub fn from_settings(settings: BrushSettings, color: [f32; 3]) -> Self {
        let s = settings.sanitized();
        Self {
            kind: s.kind.preview_kind(),
            tint: color,
            fill_alpha: (s.strength * s.flow).clamp(0.0, 1.0),
        }
    }
}

/// Mapeamento legado do índice de UI (`paint_brush_kind`) → [`BrushType`].
///
/// Ponte de compatibilidade enquanto a UI migra para `BrushSettings.kind`:
/// a ordem espelha o array de botões histórico (0=Pixel … 6=Rectangle) e
/// `Airbrush` entra como 7, no fim.
pub const fn brush_type_from_kind(kind: usize) -> BrushType {
    match kind {
        1 => BrushType::Soft,
        2 => BrushType::Eraser,
        3 => BrushType::Fill,
        4 => BrushType::Eyedropper,
        5 => BrushType::Line,
        6 => BrushType::Rectangle,
        7 => BrushType::Airbrush,
        _ => BrushType::Pixel,
    }
}

/// Índice legado de UI para um [`BrushType`] (inverso do mapa acima).
pub const fn kind_from_brush_type(kind: BrushType) -> usize {
    match kind {
        BrushType::Pixel => 0,
        BrushType::Soft => 1,
        BrushType::Eraser => 2,
        BrushType::Fill => 3,
        BrushType::Eyedropper => 4,
        BrushType::Line => 5,
        BrushType::Rectangle => 6,
        BrushType::Airbrush => 7,
    }
}

/// Descriptor completo de pincel (P3D-056/057: parâmetros são descriptors,
/// não lógica de widget).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrushSettings {
    pub kind: BrushType,
    /// Diâmetro em **pixels de tela** (Photoshop-like; escala com zoom via
    /// profundidade no hit — ver `AppState::brush_world_radius`).
    pub size_px: f32,
    /// Dureza `0..=1`: `1` = borda rígida (Pixel), `0` = falloff máximo.
    pub hardness: f32,
    /// Força/opacidade por dab, `0..=1`.
    pub strength: f32,
    /// Fluxo de tinta por dab (Airbrush), `0..=1`.
    pub flow: f32,
    /// Espaçamento entre dabs como fração do diâmetro, `0.01..=1.0`
    /// (P3D-057: traço independente do poll rate do ponteiro).
    pub spacing: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            kind: BrushType::Pixel,
            size_px: 4.0,
            hardness: 1.0,
            strength: 1.0,
            flow: 1.0,
            spacing: 0.15,
        }
    }
}

impl BrushSettings {
    /// Sanitiza todos os parâmetros para domínios válidos (nunca NaN/∞).
    pub fn sanitized(self) -> Self {
        Self {
            kind: self.kind,
            size_px: if self.size_px.is_finite() {
                self.size_px.clamp(1.0, 512.0)
            } else {
                Self::default().size_px
            },
            hardness: clamp01(self.hardness),
            strength: clamp01(self.strength),
            flow: clamp01(self.flow),
            spacing: if self.spacing.is_finite() {
                self.spacing.clamp(0.01, 1.0)
            } else {
                Self::default().spacing
            },
        }
    }

    /// Raio do carimbo em pixels de canvas, derivado do diâmetro de tela.
    pub fn radius_px(self) -> u32 {
        (self.size_px * 0.5).round().max(1.0) as u32
    }

    /// Passo entre dabs em pixels para o diâmetro atual.
    pub fn dab_step_px(self) -> f32 {
        (self.size_px * self.spacing).max(0.5)
    }

    /// Amostras de dab ao longo de um segmento, espaçadas por
    /// `dab_step_px`. Traços rápidos e lentos com os mesmos extremos
    /// produzem exatamente o mesmo conjunto (P3D-057).
    pub fn stroke_dabs(self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(u32, u32)> {
        let mut sampler = StrokeSampler::default();
        let mut dabs = sampler.begin([x0, y0], self.dab_step_px());
        dabs.extend(sampler.extend([x1, y1]));
        dabs.into_iter()
            .map(|point| {
                (
                    point[0].round().max(0.0) as u32,
                    point[1].round().max(0.0) as u32,
                )
            })
            .collect()
    }
}

/// Amostrador incremental de strokes por comprimento de arco em espaço 2D.
///
/// O residual entre eventos é preservado, portanto dividir o mesmo caminho em
/// frequências diferentes de pointer events produz os mesmos dabs. A camada de
/// apresentação escolhe o espaço (pixels de textura ou pixels lógicos de tela)
/// e apenas entrega pontos; o sampler não conhece UI, teclas ou ferramentas.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeSampler {
    last_input: Option<[f32; 2]>,
    step: f32,
    distance_to_next: f32,
}

impl Default for StrokeSampler {
    fn default() -> Self {
        Self {
            last_input: None,
            step: 1.0,
            distance_to_next: 1.0,
        }
    }
}

impl StrokeSampler {
    /// Inicia uma sessão e emite o dab inicial.
    pub fn begin(&mut self, point: [f32; 2], step: f32) -> Vec<[f32; 2]> {
        if !point[0].is_finite() || !point[1].is_finite() {
            self.reset();
            return Vec::new();
        }
        self.step = sanitize_step(step);
        self.distance_to_next = self.step;
        self.last_input = Some(point);
        vec![point]
    }

    /// Acrescenta um pointer sample e emite somente os dabs regularmente
    /// espaçados que cabem no novo segmento.
    pub fn extend(&mut self, point: [f32; 2]) -> Vec<[f32; 2]> {
        if !point[0].is_finite() || !point[1].is_finite() {
            return Vec::new();
        }
        let Some(previous) = self.last_input else {
            return self.begin(point, self.step);
        };
        self.last_input = Some(point);

        let delta = [point[0] - previous[0], point[1] - previous[1]];
        let segment_length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        if segment_length <= f32::EPSILON {
            return Vec::new();
        }

        let direction = [delta[0] / segment_length, delta[1] / segment_length];
        let mut travelled = 0.0;
        let mut dabs = Vec::new();
        while travelled + self.distance_to_next <= segment_length + f32::EPSILON {
            travelled += self.distance_to_next;
            dabs.push([
                previous[0] + direction[0] * travelled,
                previous[1] + direction[1] * travelled,
            ]);
            self.distance_to_next = self.step;
        }
        self.distance_to_next -= segment_length - travelled;
        if self.distance_to_next <= f32::EPSILON {
            self.distance_to_next = self.step;
        }
        dabs
    }

    /// Encerra a sessão sem emitir um dab artificial no pointer-up.
    pub fn reset(&mut self) {
        self.last_input = None;
        self.distance_to_next = self.step;
    }

    pub fn is_active(&self) -> bool {
        self.last_input.is_some()
    }
}

fn sanitize_step(step: f32) -> f32 {
    if step.is_finite() { step.max(0.5) } else { 1.0 }
}

/// Clamp 0..=1 tolerante a NaN (NaN cai em 0, o valor neutro dos parâmetros).
fn clamp01(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sanitizes_and_clamps() {
        let s = BrushSettings {
            size_px: 0.0,
            hardness: 2.0,
            strength: -1.0,
            flow: f32::NAN,
            spacing: 0.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.size_px, 1.0);
        assert_eq!(s.hardness, 1.0);
        assert_eq!(s.strength, 0.0);
        assert_eq!(s.flow, 0.0);
        assert_eq!(s.spacing, 0.01);
    }

    #[test]
    fn kind_mapping_roundtrip() {
        for kind in [
            BrushType::Pixel,
            BrushType::Soft,
            BrushType::Eraser,
            BrushType::Fill,
            BrushType::Eyedropper,
            BrushType::Line,
            BrushType::Rectangle,
            BrushType::Airbrush,
        ] {
            assert_eq!(brush_type_from_kind(kind_from_brush_type(kind)), kind);
        }
    }

    #[test]
    fn stroke_dabs_are_poll_rate_invariant() {
        let s = BrushSettings {
            size_px: 10.0,
            spacing: 0.2,
            ..Default::default()
        };
        let mut fast_sampler = StrokeSampler::default();
        let mut fast = fast_sampler.begin([0.0, 0.0], s.dab_step_px());
        fast.extend(fast_sampler.extend([100.0, 100.0]));

        let mut slow_sampler = StrokeSampler::default();
        let mut slow = slow_sampler.begin([0.0, 0.0], s.dab_step_px());
        for i in 1..=10 {
            slow.extend(slow_sampler.extend([i as f32 * 10.0, i as f32 * 10.0]));
        }

        assert_eq!(fast.len(), slow.len());
        for (fast, slow) in fast.iter().zip(&slow) {
            assert!((fast[0] - slow[0]).abs() < 1e-3);
            assert!((fast[1] - slow[1]).abs() < 1e-3);
        }
    }

    #[test]
    fn stroke_sampler_carries_sub_step_residual_between_events() {
        let mut sampler = StrokeSampler::default();
        assert_eq!(sampler.begin([0.0, 0.0], 5.0), vec![[0.0, 0.0]]);
        assert!(sampler.extend([2.0, 0.0]).is_empty());
        assert_eq!(sampler.extend([4.0, 0.0]), Vec::<[f32; 2]>::new());
        assert_eq!(sampler.extend([6.0, 0.0]), vec![[5.0, 0.0]]);
        assert_eq!(sampler.extend([11.0, 0.0]), vec![[10.0, 0.0]]);
    }

    #[test]
    fn stroke_sampler_rejects_non_finite_input_without_corrupting_session() {
        let mut sampler = StrokeSampler::default();
        sampler.begin([1.0, 2.0], 2.0);
        assert!(sampler.extend([f32::NAN, 2.0]).is_empty());
        assert_eq!(sampler.extend([3.0, 2.0]), vec![[3.0, 2.0]]);
        sampler.begin([f32::INFINITY, 0.0], 2.0);
        assert!(!sampler.is_active());
    }

    #[test]
    fn preview_kind_matches_brush_semantics() {
        use BrushPreviewKind as K;
        for (kind, expected) in [
            (BrushType::Pixel, K::Ring),
            (BrushType::Soft, K::Ring),
            (BrushType::Airbrush, K::Ring),
            (BrushType::Eraser, K::HollowRing),
            (BrushType::Eyedropper, K::Crosshair),
            (BrushType::Fill, K::None),
            (BrushType::Line, K::None),
            (BrushType::Rectangle, K::None),
        ] {
            assert_eq!(kind.preview_kind(), expected);
        }
    }

    #[test]
    fn preview_style_derives_tint_and_alpha() {
        let s = BrushSettings {
            kind: BrushType::Soft,
            strength: 0.8,
            flow: 0.5,
            ..Default::default()
        };
        let style = BrushPreviewStyle::from_settings(s, [0.2, 0.4, 0.9]);
        assert_eq!(style.kind, BrushPreviewKind::Ring);
        assert_eq!(style.tint, [0.2, 0.4, 0.9]);
        assert!((style.fill_alpha - 0.4).abs() < 1e-6);
    }
}
