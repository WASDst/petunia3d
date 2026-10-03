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
    /// Borrão: arrasta a cor já pintada na direção do traço.
    Smudge,
    /// Desfoque local (média da vizinhança) sob o pincel.
    Blur,
    /// Clareia (*dodge*) o que já foi pintado.
    Dodge,
    /// Escurece (*burn*) o que já foi pintado.
    Burn,
    /// Spray: pontos espalhados dentro do raio, densidade por `spray_density`.
    Spray,
    /// Carimbo de clonagem: copia a textura de um ponto de origem (Ctrl+clique).
    Clone,
    /// Elipse preenchida inscrita entre dois cantos (cap. 44: círculo/forma).
    Ellipse,
}

/// Transient, toolkit-independent projector used as a brush opacity mask.
#[derive(Clone, Debug)]
pub struct SurfaceStencil {
    pub target: uuid::Uuid,
    pub image: std::sync::Arc<petunia_project::Canvas>,
    pub world_to_clip: glam::Mat4,
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
    pub mirror: bool,
    pub luminance: bool,
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

/// Faces elegíveis para receber tinta no traço atual (P3D-132).
///
/// Resolvido uma vez por traço a partir da seleção, da trava de pincel e do
/// isolamento. A máscara de texels (2D) é rasterizada sob demanda, porque o
/// caminho 3D só precisa da lista de faces.
#[derive(Debug)]
pub struct PaintRestriction {
    pub faces: Vec<bool>,
    mask: std::sync::OnceLock<petunia_mesh::CoverageMask>,
}

impl PaintRestriction {
    pub fn new(faces: Vec<bool>) -> Self {
        Self {
            faces,
            mask: std::sync::OnceLock::new(),
        }
    }

    pub fn allows_face(&self, face: usize) -> bool {
        self.faces.get(face).copied().unwrap_or(false)
    }

    pub fn allowed_count(&self) -> usize {
        self.faces.iter().filter(|f| **f).count()
    }

    /// Máscara de texels `w × h` das faces elegíveis (com 1 px de sangria).
    pub fn mask(&self, mesh: &petunia_mesh::Mesh, w: u32, h: u32) -> &petunia_mesh::CoverageMask {
        self.mask.get_or_init(|| {
            mesh.uv_coverage_mask(
                self.faces
                    .iter()
                    .enumerate()
                    .filter_map(|(i, ok)| ok.then_some(i)),
                w,
                h,
                1.0,
            )
        })
    }
}

/// Buffer do traço atual: fotografia da camada antes do traço e cobertura
/// acumulada por texel. O resultado é sempre `mistura(base, cor, cobertura)`,
/// então dabs sobrepostos não escurecem além do teto de opacidade (Photoshop:
/// *flow* acumula, *opacity* limita).
#[derive(Clone, Debug)]
pub struct StrokeBuffer {
    pub layer: uuid::Uuid,
    pub w: u32,
    pub h: u32,
    /// RGBA8 da camada antes do traço.
    pub base: Vec<u8>,
    /// Cobertura acumulada (`0..=1`) por texel.
    pub coverage: Vec<f32>,
    /// Dabs já aplicados neste traço (alimenta jitter e espalhamento determinísticos).
    pub dabs: u32,
    /// Centro do último dab em texels (direção do Smudge).
    pub last_texel: Option<[f32; 2]>,
    /// Deslocamento origem→destino do Clone, fixado no primeiro dab do traço.
    pub clone_offset: Option<[i32; 2]>,
}

impl StrokeBuffer {
    pub fn new(layer: uuid::Uuid, w: u32, h: u32, base: Vec<u8>) -> Self {
        Self {
            layer,
            w,
            h,
            base,
            coverage: vec![0.0; (w as usize) * (h as usize)],
            dabs: 0,
            last_texel: None,
            clone_offset: None,
        }
    }
}

impl BrushType {
    /// Pincéis de traço livre (carimbam dabs).
    pub const fn is_free_brush(self) -> bool {
        matches!(
            self,
            Self::Pixel
                | Self::Soft
                | Self::Eraser
                | Self::Airbrush
                | Self::Smudge
                | Self::Blur
                | Self::Dodge
                | Self::Burn
                | Self::Spray
                | Self::Clone
        )
    }

    /// Formas com ancoragem press→release.
    pub const fn is_shape(self) -> bool {
        matches!(self, Self::Line | Self::Rectangle | Self::Ellipse)
    }

    /// Forma canônica do cursor de preview (iniciativa Paint).
    pub const fn preview_kind(self) -> BrushPreviewKind {
        match self {
            Self::Pixel | Self::Soft | Self::Airbrush | Self::Dodge | Self::Burn | Self::Spray => {
                BrushPreviewKind::Ring
            }
            Self::Eraser | Self::Smudge | Self::Blur | Self::Clone => BrushPreviewKind::HollowRing,
            Self::Eyedropper => BrushPreviewKind::Crosshair,
            Self::Fill | Self::Line | Self::Rectangle | Self::Ellipse => BrushPreviewKind::None,
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
        8 => BrushType::Smudge,
        9 => BrushType::Blur,
        10 => BrushType::Dodge,
        11 => BrushType::Burn,
        12 => BrushType::Spray,
        13 => BrushType::Clone,
        14 => BrushType::Ellipse,
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
        BrushType::Smudge => 8,
        BrushType::Blur => 9,
        BrushType::Dodge => 10,
        BrushType::Burn => 11,
        BrushType::Spray => 12,
        BrushType::Clone => 13,
        BrushType::Ellipse => 14,
    }
}

/// Forma da ponta do pincel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrushTip {
    #[default]
    Round,
    Square,
    Diamond,
}

/// Como a cor do pincel se combina com a que já está na camada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrushBlend {
    #[default]
    Normal,
    Multiply,
    Screen,
    Add,
    Darken,
    Lighten,
}

impl BrushBlend {
    /// Combina um canal (`base` sobre a camada, `src` do pincel), ambos `0..=255`.
    pub fn channel(self, base: f32, src: f32) -> f32 {
        match self {
            Self::Normal => src,
            Self::Multiply => base * src / 255.0,
            Self::Screen => 255.0 - (255.0 - base) * (255.0 - src) / 255.0,
            Self::Add => (base + src).min(255.0),
            Self::Darken => base.min(src),
            Self::Lighten => base.max(src),
        }
    }
}

/// Comportamento do traço além do descriptor básico (iniciativa Paint, 2E).
///
/// Mora no estado de ferramentas porque `BrushSettings` é literal em muitos
/// pontos e porque estes parâmetros mudam *como* o traço é montado, não o que
/// um dab individual faz.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrushStyle {
    pub tip: BrushTip,
    /// Rotação da ponta em graus (`-180..=180`); só altera pontas não redondas
    /// ou achatadas.
    pub angle_deg: f32,
    /// Achatamento da ponta (`0.1..=1`; `1` = sem achatar).
    pub roundness: f32,
    pub blend: BrushBlend,
    /// Estabilizador do traço (`0..=0.95`): média móvel do cursor.
    pub smoothing: f32,
    /// Variação aleatória do tamanho por dab (`0..=1`).
    pub size_jitter: f32,
    /// Variação aleatória da opacidade por dab (`0..=1`).
    pub opacity_jitter: f32,
    /// Deslocamento aleatório do dab, em raios (`0..=2`).
    pub scatter: f32,
    /// Densidade do Spray (`0.05..=1`).
    pub spray_density: f32,
    /// Semente do traço: o mesmo traço sempre produz o mesmo resultado.
    pub seed: u32,
    #[serde(default)]
    pub alpha_lock: bool,
    #[serde(default)]
    pub dithering: bool,
    /// Passo na rampa ordenada da paleta: -1 sombra, +1 luz, 0 pintura normal.
    #[serde(default)]
    pub palette_step: i8,
    #[serde(default)]
    pub pixel_perfect: bool,
}

impl Default for BrushStyle {
    fn default() -> Self {
        Self {
            tip: BrushTip::Round,
            angle_deg: 0.0,
            roundness: 1.0,
            blend: BrushBlend::Normal,
            smoothing: 0.0,
            size_jitter: 0.0,
            opacity_jitter: 0.0,
            scatter: 0.0,
            spray_density: 0.35,
            seed: 1,
            alpha_lock: false,
            dithering: false,
            palette_step: 0,
            pixel_perfect: false,
        }
    }
}

impl BrushStyle {
    pub fn sanitized(self) -> Self {
        let finite = |v: f32, lo: f32, hi: f32, default: f32| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                default
            }
        };
        Self {
            tip: self.tip,
            angle_deg: finite(self.angle_deg, -180.0, 180.0, 0.0),
            roundness: finite(self.roundness, 0.1, 1.0, 1.0),
            blend: self.blend,
            smoothing: finite(self.smoothing, 0.0, 0.95, 0.0),
            size_jitter: finite(self.size_jitter, 0.0, 1.0, 0.0),
            opacity_jitter: finite(self.opacity_jitter, 0.0, 1.0, 0.0),
            scatter: finite(self.scatter, 0.0, 2.0, 0.0),
            spray_density: finite(self.spray_density, 0.05, 1.0, 0.35),
            seed: self.seed,
            alpha_lock: self.alpha_lock,
            dithering: self.dithering,
            palette_step: self.palette_step.clamp(-1, 1),
            pixel_perfect: self.pixel_perfect,
        }
    }

    /// Distância normalizada (`0` centro, `1` borda) de um ponto `(a, b)` (em
    /// raios) ao centro, conforme a ponta, o ângulo e o achatamento.
    pub fn tip_distance(&self, a: f32, b: f32) -> f32 {
        let (sin, cos) = self.angle_deg.to_radians().sin_cos();
        let u = a * cos + b * sin;
        let v = (-a * sin + b * cos) / self.roundness.max(0.1);
        match self.tip {
            BrushTip::Round => (u * u + v * v).sqrt(),
            BrushTip::Square => u.abs().max(v.abs()),
            BrushTip::Diamond => u.abs() + v.abs(),
        }
    }
}

/// Número pseudoaleatório determinístico em `[0, 1)` a partir de uma semente e
/// três inteiros (hash de bits; sem estado global).
pub fn hash01(seed: u32, a: u32, b: u32, c: u32) -> f32 {
    let mut h = seed ^ 0x9e37_79b9;
    for v in [a, b, c] {
        h = (h ^ v).wrapping_mul(0x85eb_ca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2_ae35);
        h ^= h >> 16;
    }
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Estabilizador de cursor: suaviza o caminho antes de virar dabs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointStabilizer {
    smoothed: Option<[f32; 2]>,
}

impl PointStabilizer {
    /// Aplica a média móvel exponencial (`strength` 0 = sem suavização).
    pub fn filter(&mut self, point: [f32; 2], strength: f32) -> [f32; 2] {
        let k = if strength.is_finite() {
            strength.clamp(0.0, 0.95)
        } else {
            0.0
        };
        let next = match self.smoothed {
            Some(prev) if k > 0.0 => [
                prev[0] + (point[0] - prev[0]) * (1.0 - k),
                prev[1] + (point[1] - prev[1]) * (1.0 - k),
            ],
            _ => point,
        };
        self.smoothed = Some(next);
        next
    }

    pub fn reset(&mut self) {
        self.smoothed = None;
    }
}

/// Pincel nomeado: tipo, descriptor e estilo (P3D-153, presets de ferramenta).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrushPreset {
    pub name: String,
    pub settings: BrushSettings,
    pub style: BrushStyle,
}

impl BrushPreset {
    /// Presets que vêm com o app (não editáveis, sempre disponíveis).
    pub fn builtin() -> Vec<BrushPreset> {
        let preset =
            |name: &str, kind, size, hardness, strength, flow, spacing, style: BrushStyle| {
                BrushPreset {
                    name: name.to_string(),
                    settings: BrushSettings {
                        kind,
                        size_px: size,
                        hardness,
                        strength,
                        flow,
                        spacing,
                    },
                    style,
                }
            };
        let plain = BrushStyle::default();
        vec![
            preset("Pencil", BrushType::Pixel, 2.0, 1.0, 1.0, 1.0, 0.1, plain),
            preset(
                "Soft brush",
                BrushType::Soft,
                24.0,
                0.2,
                1.0,
                1.0,
                0.15,
                plain,
            ),
            preset(
                "Ink",
                BrushType::Soft,
                10.0,
                0.9,
                1.0,
                1.0,
                0.08,
                BrushStyle {
                    smoothing: 0.5,
                    ..plain
                },
            ),
            preset(
                "Marker",
                BrushType::Soft,
                28.0,
                0.8,
                0.6,
                1.0,
                0.1,
                BrushStyle {
                    tip: BrushTip::Square,
                    blend: BrushBlend::Multiply,
                    ..plain
                },
            ),
            preset(
                "Airbrush",
                BrushType::Airbrush,
                40.0,
                0.0,
                1.0,
                0.3,
                0.1,
                plain,
            ),
            preset(
                "Spray can",
                BrushType::Spray,
                48.0,
                0.5,
                1.0,
                1.0,
                0.12,
                BrushStyle {
                    spray_density: 0.3,
                    ..plain
                },
            ),
            preset(
                "Chalk",
                BrushType::Soft,
                26.0,
                0.6,
                0.85,
                1.0,
                0.12,
                BrushStyle {
                    size_jitter: 0.25,
                    opacity_jitter: 0.35,
                    scatter: 0.25,
                    ..plain
                },
            ),
            preset("Smudge", BrushType::Smudge, 28.0, 0.4, 0.8, 1.0, 0.1, plain),
        ]
    }

    /// Arquivo dos presets do usuário.
    pub fn user_path() -> std::path::PathBuf {
        petunia_config::UserPreferences::config_dir().join("brush-presets.json")
    }

    /// Presets do usuário em `path` (vazio se não existir ou estiver corrompido).
    pub fn load_user_from(path: &std::path::Path) -> Vec<BrushPreset> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Vec<BrushPreset>>(&text).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|mut p| {
                p.settings = p.settings.sanitized();
                p.style = p.style.sanitized();
                p
            })
            .collect()
    }

    /// Grava os presets do usuário de forma atômica.
    pub fn save_user_to(path: &std::path::Path, presets: &[BrushPreset]) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(presets)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
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

/// Pixels de diâmetro por unidade do controle "Size" (`paint_radius`).
///
/// Fonte única da relação entre o slider e o pincel: o carimbo na viewport 3D,
/// o clique direto na malha, o pincel do canvas 2D e o anel de preview usam
/// **este** fator, sem constantes soltas.
pub const BRUSH_PX_PER_UNIT: f32 = 16.0;

/// Diâmetro em pixels equivalente ao valor do slider "Size".
pub fn brush_size_px_from_slider(size: f32) -> f32 {
    (size * BRUSH_PX_PER_UNIT).max(2.0)
}

#[cfg(test)]
mod brush_size_contract_tests {
    use super::*;

    #[test]
    fn slider_maps_to_pixels_with_a_floor() {
        assert_eq!(brush_size_px_from_slider(0.8), 12.8);
        assert_eq!(brush_size_px_from_slider(0.01), 2.0);
        assert_eq!(brush_size_px_from_slider(10.0), 160.0);
    }
}
