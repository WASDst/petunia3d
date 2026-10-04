//! Motor de dabs do Paint: restrição por face/seleção, pincel esférico 3D e
//! buffer de traço (P3D-056/057/062/132).
//!
//! Princípios:
//! - **Uma restrição por traço** ([`PaintRestriction`]): isolamento pela
//!   seleção e trava de pincel viram a mesma lista de faces elegíveis, usada
//!   pelo caminho 3D (candidatas) e pelo 2D (máscara de texels).
//! - **Pincel esférico no 3D**: o dab pinta os texels cujo ponto de superfície
//!   está a até `r_world` do ponto de impacto. Faces que só compartilham a
//!   mesma região da textura (UV sobreposta) ou ficam do outro lado do objeto
//!   nunca recebem tinta.
//! - **Buffer de traço**: o pixel final é `mistura(base, cor, cobertura)`;
//!   *flow* acumula, *strength* limita (sem escurecimento por sobreposição).

use std::sync::Arc;

use glam::Vec3;
use petunia_core::{
    AppState, BrushBlend, BrushLock, BrushSettings, BrushStyle, BrushTip, BrushType,
    PaintRestriction, StrokeBuffer, hash01,
};
use petunia_mesh::Mesh;
use petunia_project::Canvas;
use petunia_project::paint_layers::TILE_SIZE;

use crate::{DirtyTiles, PaintModule};

/// Sangria de borda (texels) para não deixar costuras na filtragem bilinear.
const BLEED_PX: f32 = 1.0;

/// Peso do dab (`0..=1`) na distância normalizada `t` (`0` = centro).
pub fn dab_falloff(kind: BrushType, t: f32, hardness: f32) -> f32 {
    if !(0.0..=1.0).contains(&t) {
        return 0.0;
    }
    match kind {
        BrushType::Pixel => 1.0,
        BrushType::Eraser => 1.0 - t,
        _ => {
            let core = hardness.clamp(0.0, 1.0);
            if t <= core {
                1.0
            } else if core >= 1.0 {
                0.0
            } else {
                let k = (t - core) / (1.0 - core).max(1e-4);
                (1.0 - k) * (1.0 - k)
            }
        }
    }
}

/// Parâmetros resolvidos de um dab.
#[derive(Clone, Copy)]
struct DabParams {
    kind: BrushType,
    color: [u8; 4],
    hardness: f32,
    /// Acúmulo por dab (`flow`, e `strength` no Airbrush).
    rate: f32,
    /// Teto de opacidade do traço.
    cap: f32,
    blend: BrushBlend,
    /// Densidade do Spray (`0` = não é spray).
    spray: f32,
    seed: u32,
    /// Deslocamento do Smudge em texels (centro atual − centro anterior).
    delta: [f32; 2],
    /// Deslocamento origem→destino do Clone (`None` = sem origem definida).
    clone: Option<[i32; 2]>,
    blur_radius: i32,
}

impl DabParams {
    fn new(settings: BrushSettings, style: &BrushStyle, color: [u8; 4]) -> Self {
        let s = settings.sanitized();
        let airbrush = s.kind == BrushType::Airbrush;
        Self {
            kind: s.kind,
            color,
            hardness: if airbrush { 0.0 } else { s.hardness },
            rate: if airbrush {
                (s.flow * s.strength).clamp(0.0, 1.0)
            } else {
                s.flow
            },
            cap: if airbrush { 1.0 } else { s.strength },
            blend: style.blend,
            spray: if s.kind == BrushType::Spray {
                style.spray_density
            } else {
                0.0
            },
            seed: style.seed,
            delta: [0.0; 2],
            clone: None,
            blur_radius: 1,
        }
    }
}

fn stencil_weight(stencil: &petunia_core::brush::SurfaceStencil, position: Vec3) -> f32 {
    let clip = stencil.world_to_clip * position.extend(1.0);
    if !clip.is_finite() || clip.w <= 1e-6 {
        return 0.0;
    }
    let screen = [clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5];
    let placement = crate::projection::StencilPlacement {
        center_px: stencil.center,
        size_px: stencil.size,
        rotation_rad: stencil.rotation,
        mirror_x: stencil.mirror,
    };
    crate::projection::stencil_alpha(
        &stencil.image,
        &placement,
        screen,
        if stencil.luminance {
            crate::projection::StencilChannel::Luma
        } else {
            crate::projection::StencilChannel::Alpha
        },
    )
}

/// Canvas + buffer + tiles sujos de um traço em andamento.
struct Target<'a> {
    cv: &'a mut Canvas,
    buf: &'a mut StrokeBuffer,
    dirty: Vec<bool>,
    tiles_x: u32,
    alpha_lock: bool,
    dithering: bool,
    palette_step: i8,
    palette: Vec<[u8; 3]>,
}

fn lerp_px(a: [u8; 4], b: [f32; 4], m: f32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for c in 0..4 {
        out[c] = (a[c] as f32 * (1.0 - m) + b[c] * m)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    out
}

impl Target<'_> {
    #[inline]
    fn px(pixels: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    }

    fn mark(&mut self, x: u32, y: u32) {
        let tile = (y / TILE_SIZE) * self.tiles_x + x / TILE_SIZE;
        if let Some(flag) = self.dirty.get_mut(tile as usize) {
            *flag = true;
        }
    }

    /// Borrão: puxa a cor vizinha (no sentido oposto ao movimento) sobre o texel.
    fn smudge(&mut self, x: u32, y: u32, weight: f32, p: &DabParams) {
        let a = (weight * p.rate * p.cap).clamp(0.0, 1.0);
        if a <= 0.0 || (p.delta[0].abs() < 0.01 && p.delta[1].abs() < 0.01) {
            return;
        }
        let (w, h) = (self.buf.w, self.buf.h);
        let sx = (x as f32 - p.delta[0]).round().clamp(0.0, (w - 1) as f32) as u32;
        let sy = (y as f32 - p.delta[1]).round().clamp(0.0, (h - 1) as f32) as u32;
        let cur = Self::px(&self.cv.pixels, w, x, y);
        let src = Self::px(&self.cv.pixels, w, sx, sy);
        let mut out = lerp_px(cur, src.map(f32::from), a);
        if self.alpha_lock {
            out[3] = Self::px(&self.buf.base, w, x, y)[3];
        }
        let i = ((y * w + x) * 4) as usize;
        self.cv.pixels[i..i + 4].copy_from_slice(&out);
        self.mark(x, y);
    }

    /// Cor de destino do texel para os pincéis que derivam da própria camada.
    fn derived_target(&self, x: u32, y: u32, base: [u8; 4], p: &DabParams) -> Option<[f32; 4]> {
        let (w, h) = (self.buf.w as i32, self.buf.h as i32);
        match p.kind {
            BrushType::Dodge => Some([
                base[0] as f32 + (255.0 - base[0] as f32) * 0.5,
                base[1] as f32 + (255.0 - base[1] as f32) * 0.5,
                base[2] as f32 + (255.0 - base[2] as f32) * 0.5,
                base[3] as f32,
            ]),
            BrushType::Burn => Some([
                base[0] as f32 * 0.5,
                base[1] as f32 * 0.5,
                base[2] as f32 * 0.5,
                base[3] as f32,
            ]),
            BrushType::Blur => {
                let r = p.blur_radius.max(1);
                let mut acc = [0.0f32; 4];
                let mut n = 0.0;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (sx, sy) = (x as i32 + dx, y as i32 + dy);
                        if sx < 0 || sy < 0 || sx >= w || sy >= h {
                            continue;
                        }
                        let px = Self::px(&self.buf.base, self.buf.w, sx as u32, sy as u32);
                        for c in 0..4 {
                            acc[c] += px[c] as f32;
                        }
                        n += 1.0;
                    }
                }
                (n > 0.0).then(|| acc.map(|v| v / n))
            }
            BrushType::Clone => {
                let [ox, oy] = p.clone?;
                let sx = (x as i32 + ox).clamp(0, w - 1) as u32;
                let sy = (y as i32 + oy).clamp(0, h - 1) as u32;
                Some(Self::px(&self.buf.base, self.buf.w, sx, sy).map(f32::from))
            }
            _ => None,
        }
    }

    #[inline]
    fn apply(&mut self, x: u32, y: u32, weight: f32, p: &DabParams) {
        if x >= self.buf.w || y >= self.buf.h || weight <= 0.0 {
            return;
        }
        let original = Self::px(&self.buf.base, self.buf.w, x, y);
        if self.alpha_lock && original[3] == 0 {
            return;
        }
        if p.kind == BrushType::Smudge {
            self.smudge(x, y, weight, p);
            return;
        }
        let mut weight = weight;
        if p.spray > 0.0 {
            // Cada ponto do spray é cheio; a densidade cai em direção à borda.
            let gate = hash01(p.seed, x, y, self.buf.dabs);
            if gate >= p.spray * (0.35 + 0.65 * weight) {
                return;
            }
            weight = 1.0;
        }
        let i = (y * self.buf.w + x) as usize;
        let cov0 = self.buf.coverage[i];
        let cov1 = if p.kind == BrushType::Pixel {
            1.0
        } else {
            cov0 + (1.0 - cov0) * (weight * p.rate).clamp(0.0, 1.0)
        };
        if cov1 <= cov0 + 1e-6 {
            return;
        }
        let base = Self::px(&self.buf.base, self.buf.w, x, y);
        let mut out = if p.kind == BrushType::Eraser {
            let mut out = base;
            out[3] = (base[3] as f32 * (1.0 - cov1 * p.cap).max(0.0)) as u8;
            out
        } else {
            let mut target = match self.derived_target(x, y, base, p) {
                Some(t) => t,
                None if matches!(p.kind, BrushType::Clone) => return,
                None => [
                    p.blend.channel(base[0] as f32, p.color[0] as f32),
                    p.blend.channel(base[1] as f32, p.color[1] as f32),
                    p.blend.channel(base[2] as f32, p.color[2] as f32),
                    p.color[3] as f32,
                ],
            };
            if self.palette_step != 0 && !self.palette.is_empty() && p.kind != BrushType::Eraser {
                let nearest = self
                    .palette
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, rgb)| {
                        (0..3)
                            .map(|c| (rgb[c] as i32 - base[c] as i32).pow(2))
                            .sum::<i32>()
                    })
                    .map_or(0, |(i, _)| i);
                let index = (nearest as isize + self.palette_step as isize)
                    .clamp(0, self.palette.len() as isize - 1) as usize;
                for (channel, value) in target.iter_mut().take(3).zip(self.palette[index]) {
                    *channel = value as f32;
                }
            }
            let coverage = if p.kind == BrushType::Pixel {
                weight.clamp(0.0, 1.0)
            } else {
                (cov1 * p.cap).clamp(0.0, 1.0)
            };
            let m = if self.dithering {
                const BAYER: [[u8; 4]; 4] =
                    [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
                if coverage >= (BAYER[(y % 4) as usize][(x % 4) as usize] as f32 + 0.5) / 16.0 {
                    1.0
                } else {
                    0.0
                }
            } else {
                coverage
            };
            lerp_px(base, target, m)
        };
        if self.alpha_lock {
            out[3] = base[3];
        }
        self.buf.coverage[i] = cov1;
        self.cv.pixels[i * 4..i * 4 + 4].copy_from_slice(&out);
        self.mark(x, y);
    }

    fn into_dirty(self) -> DirtyTiles {
        DirtyTiles {
            indices: self
                .dirty
                .iter()
                .enumerate()
                .filter_map(|(i, d)| d.then_some(i as u32))
                .collect(),
        }
    }
}

/// Parâmetros do dab `n` do traço: tamanho, opacidade e deslocamento com jitter.
struct DabJitter {
    radius_scale: f32,
    cap_scale: f32,
    /// Deslocamento aleatório em unidades de raio (ângulo, distância).
    scatter: (f32, f32),
}

fn dab_jitter(style: &BrushStyle, n: u32) -> DabJitter {
    let signed = |salt: u32| hash01(style.seed, n, salt, 11) * 2.0 - 1.0;
    DabJitter {
        radius_scale: (1.0 + signed(0) * style.size_jitter).max(0.1),
        cap_scale: 1.0 - style.opacity_jitter * hash01(style.seed, n, 3, 14),
        scatter: (
            hash01(style.seed, n, 1, 12) * std::f32::consts::TAU,
            hash01(style.seed, n, 2, 13).sqrt() * style.scatter,
        ),
    }
}

impl PaintModule {
    /// Faces elegíveis pelo isolamento de seleção e pela trava de pincel.
    ///
    /// Resolvido uma vez por traço (fica em `paint_restriction`). `first_face`
    /// é a face sob o primeiro toque, usada pela trava `FirstFace`.
    pub fn resolve_restriction(
        state: &mut AppState,
        first_face: Option<usize>,
    ) -> Option<Arc<PaintRestriction>> {
        if let Some(cached) = &state.session.tools.paint_restriction {
            return cached.clone();
        }
        let tools = &state.session.tools;
        let by_selection =
            tools.paint_isolate_selection || tools.brush_lock == BrushLock::SelectedFaces;
        let by_first_face = tools.brush_lock == BrushLock::FirstFace;
        let resolved = if !by_selection && !by_first_face {
            None
        } else {
            state.project.active_mesh().map(|mesh| {
                let mut flags = vec![true; mesh.faces.len()];
                if by_selection {
                    for (i, face) in mesh.faces.iter().enumerate() {
                        flags[i] = face.selected || state.session.selection.faces.contains(&i);
                    }
                }
                if by_first_face {
                    let locked = state.session.tools.paint_lock_face.unwrap_or(first_face);
                    for (i, flag) in flags.iter_mut().enumerate() {
                        *flag &= Some(i) == locked;
                    }
                }
                Arc::new(PaintRestriction::new(flags))
            })
        };
        if by_first_face {
            state
                .session
                .tools
                .paint_lock_face
                .get_or_insert(first_face);
        }
        state.session.tools.paint_restriction = Some(resolved.clone());
        resolved
    }

    /// Sem faces elegíveis com a restrição ativa (ex.: máscara ligada e nada
    /// selecionado): o motor não pinta e a UI explica.
    pub fn restriction_blocks_everything(restriction: Option<&Arc<PaintRestriction>>) -> bool {
        restriction.is_some_and(|r| r.allowed_count() == 0)
    }

    /// Tamanho do canvas da camada ativa.
    pub(crate) fn active_canvas_dims(state: &AppState) -> Option<(u32, u32)> {
        let asset = state.project.assets.get(state.project.active)?;
        asset
            .paint_stack
            .as_ref()
            .and_then(|s| s.active())
            .and_then(|l| l.canvas())
            .or(asset.texture.as_ref())
            .map(|c| (c.w, c.h))
    }

    /// Executa `f` com o canvas da camada ativa, o buffer do traço e a malha.
    ///
    /// Cria o buffer de traço na primeira chamada. Retorna `None` se não há
    /// camada pintável (ou ela está travada).
    fn with_stroke_target<R>(
        state: &mut AppState,
        f: impl FnOnce(&mut Target, &Mesh) -> R,
    ) -> Option<(R, DirtyTiles)> {
        let style = state.session.tools.brush_style;
        let palette: Vec<[u8; 3]> = state
            .project
            .palette
            .iter()
            .map(|rgb| rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8))
            .collect();
        let active = state.project.active;
        let asset = state.project.assets.get_mut(active)?;
        if asset.locked {
            return None;
        }
        let (mesh, stack) = (&asset.mesh, asset.paint_stack.as_mut()?);
        let layer = stack.active_mut()?;
        if layer.locked || !layer.is_paintable() {
            return None;
        }
        let layer_id = layer.id;
        let cv = layer.canvas_mut()?;
        let buf_slot = &mut state.session.tools.paint_buffer;
        let stale = buf_slot
            .as_ref()
            .is_none_or(|b| b.layer != layer_id || (b.w, b.h) != (cv.w, cv.h));
        if stale {
            *buf_slot = Some(StrokeBuffer::new(
                layer_id,
                cv.w,
                cv.h,
                cv.pixels.to_owned_value(),
            ));
        }
        let buf = buf_slot.as_mut()?;
        let tiles_x = cv.w.div_ceil(TILE_SIZE).max(1);
        let tiles_y = cv.h.div_ceil(TILE_SIZE).max(1);
        let mut target = Target {
            cv,
            buf,
            dirty: vec![false; (tiles_x * tiles_y) as usize],
            tiles_x,
            alpha_lock: style.alpha_lock,
            dithering: style.dithering,
            palette_step: style.palette_step,
            palette,
        };
        let out = f(&mut target, mesh);
        Some((out, target.into_dirty()))
    }

    pub(crate) fn paint_color_rgba(state: &AppState) -> [u8; 4] {
        let c = state.paint_color;
        [
            (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            255,
        ]
    }

    /// Dabs 2D (coordenadas de texel) com restrição por face e buffer de traço.
    pub(crate) fn stamp_dabs_2d(
        state: &mut AppState,
        points: &[(u32, u32)],
        settings: BrushSettings,
    ) -> DirtyTiles {
        if points.is_empty() {
            return DirtyTiles::default();
        }
        Self::ensure_stack(state);
        let Some((w, h)) = Self::active_canvas_dims(state) else {
            return DirtyTiles::default();
        };
        let standalone = state.session.tools.paint_stroke.is_none();
        if standalone {
            Self::reset_stroke_scratch(state);
        }
        let first_face = state.project.active_mesh().and_then(|m| {
            points
                .first()
                .and_then(|&(x, y)| m.faces_at_texel(x, y, w, h).first().copied())
        });
        let restriction = Self::resolve_restriction(state, first_face);
        if Self::restriction_blocks_everything(restriction.as_ref()) {
            return DirtyTiles::default();
        }
        let style = state.session.tools.brush_style.sanitized();
        let clone_source = state.session.tools.clone_source;
        let params = DabParams::new(settings, &style, Self::paint_color_rgba(state));
        let radius0 = settings.sanitized().size_px * 0.5;
        let stencil = state
            .session
            .tools
            .paint_stencil
            .as_ref()
            .filter(|stencil| {
                state
                    .project
                    .active()
                    .is_some_and(|a| a.id == stencil.target)
            })
            .cloned();
        let result = Self::with_stroke_target(state, |target, mesh| {
            let mask = restriction.as_ref().map(|r| r.mask(mesh, w, h));
            for &(cx, cy) in points {
                let n = target.buf.dabs;
                target.buf.dabs = n.wrapping_add(1);
                let jitter = dab_jitter(&style, n);
                let radius = (radius0 * jitter.radius_scale).max(0.5);
                let scatter = jitter.scatter.1 * radius;
                let center = [
                    cx as f32 + scatter * jitter.scatter.0.cos(),
                    cy as f32 + scatter * jitter.scatter.0.sin(),
                ];
                let mut dab = params;
                dab.cap *= jitter.cap_scale;
                dab.blur_radius = ((radius / 6.0).round() as i32).clamp(1, 4);
                if dab.kind == BrushType::Smudge {
                    dab.delta = target
                        .buf
                        .last_texel
                        .map_or([0.0; 2], |l| [center[0] - l[0], center[1] - l[1]]);
                    target.buf.last_texel = Some(center);
                }
                if dab.kind == BrushType::Clone {
                    if target.buf.clone_offset.is_none() {
                        target.buf.clone_offset = clone_source.map(|s| {
                            [
                                (s[0] - center[0]).round() as i32,
                                (s[1] - center[1]).round() as i32,
                            ]
                        });
                    }
                    dab.clone = target.buf.clone_offset;
                }
                let reach = (radius * 1.5).ceil() as i64 + 1;
                let (x0, x1) = (
                    (center[0] as i64 - reach).max(0),
                    (center[0] as i64 + reach).min(w as i64 - 1),
                );
                let (y0, y1) = (
                    (center[1] as i64 - reach).max(0),
                    (center[1] as i64 + reach).min(h as i64 - 1),
                );
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let a = (x as f32 - center[0]) / radius;
                        let b = (y as f32 - center[1]) / radius;
                        let t = style.tip_distance(a, b);
                        if t > 1.0 {
                            continue;
                        }
                        if mask.is_some_and(|m| !m.allows(x as u32, y as u32)) {
                            continue;
                        }
                        target.apply(
                            x as u32,
                            y as u32,
                            dab_falloff(dab.kind, t, dab.hardness)
                                * stencil.as_ref().map_or(1.0, |stencil| {
                                    mesh.uv_to_world([
                                        x as f32 / w as f32,
                                        1.0 - y as f32 / h as f32,
                                    ])
                                    .map_or(0.0, |(position, _)| stencil_weight(stencil, position))
                                }),
                            &dab,
                        );
                    }
                }
            }
        });
        if standalone {
            Self::reset_stroke_scratch(state);
        }
        let Some(((), dirty)) = result else {
            return DirtyTiles::default();
        };
        dirty
    }

    /// Esquece restrição, trava e buffer do traço (fim de traço ou dab avulso).
    pub(crate) fn reset_stroke_scratch(state: &mut AppState) {
        state.session.tools.paint_restriction = None;
        state.session.tools.paint_buffer = None;
        state.session.tools.paint_lock_face = None;
    }

    /// Dabs 3D: pincel esférico de raio `r_world` em cada ponto de impacto.
    ///
    /// `hits` = `(face sob o cursor, ponto de impacto)`. A simetria espelha o
    /// ponto de impacto e repete o dab (a superfície espelhada recebe o seu).
    pub(crate) fn stamp_dabs_3d(
        state: &mut AppState,
        hits: &[(usize, Vec3)],
        settings: BrushSettings,
    ) -> DirtyTiles {
        if hits.is_empty() {
            return DirtyTiles::default();
        }
        Self::ensure_stack(state);
        let Some((w, h)) = Self::active_canvas_dims(state) else {
            return DirtyTiles::default();
        };
        let restriction = Self::resolve_restriction(state, hits.first().map(|h| h.0));
        if Self::restriction_blocks_everything(restriction.as_ref()) {
            return DirtyTiles::default();
        }
        let style = state.session.tools.brush_style.sanitized();
        let clone_source = state.session.tools.clone_source;
        let params = DabParams::new(settings, &style, Self::paint_color_rgba(state));
        let size_px = settings.sanitized().size_px;
        let (sx, sy, sz) = (
            state.session.tools.paint_symmetry_x,
            state.session.tools.paint_symmetry_y,
            state.session.tools.paint_symmetry_z,
        );
        struct Dab {
            hit: Vec3,
            normal: Vec3,
            radius: f32,
            /// Texel do ponto de impacto (direção do Smudge); `None` se fora de UV.
            texel: Option<[f32; 2]>,
        }
        let mut dabs: Vec<Dab> = Vec::with_capacity(hits.len());
        if let Some(mesh) = state.project.active_mesh() {
            for &(face, hit) in hits {
                if face >= mesh.faces.len() {
                    continue;
                }
                let normal = mesh.face_normal(face).normalize_or_zero();
                let radius = state.world_radius_for_px(hit, size_px);
                let texel = Self::face_hit_uv(state, face, hit, false)
                    .and_then(|uv| Self::uv_to_px(state, uv))
                    .map(|(x, y)| [x as f32, y as f32]);
                for flip in 0u8..8 {
                    let (fx, fy, fz) = (flip & 1 != 0, flip & 2 != 0, flip & 4 != 0);
                    if (fx && !sx) || (fy && !sy) || (fz && !sz) {
                        continue;
                    }
                    let m = Vec3::new(
                        if fx { -1.0 } else { 1.0 },
                        if fy { -1.0 } else { 1.0 },
                        if fz { -1.0 } else { 1.0 },
                    );
                    dabs.push(Dab {
                        hit: hit * m,
                        normal: normal * m,
                        radius,
                        // A simetria só repete o dab: o borrão segue o original.
                        texel: if flip == 0 { texel } else { None },
                    });
                }
            }
        }
        let plain_round = style.tip == BrushTip::Round && style.roundness >= 0.999;
        let stencil = state
            .session
            .tools
            .paint_stencil
            .as_ref()
            .filter(|stencil| {
                state
                    .project
                    .active()
                    .is_some_and(|a| a.id == stencil.target)
            })
            .cloned();
        let result = Self::with_stroke_target(state, |target, mesh| {
            // Normal, centro e raio de cada face, uma vez por chamada (e não por
            // dab): descarta de graça as faces longe do pincel.
            let infos: Vec<(Vec3, Vec3, f32)> = (0..mesh.faces.len())
                .map(|fi| {
                    let face = &mesh.faces[fi];
                    let points: Vec<Vec3> = face
                        .verts
                        .iter()
                        .filter_map(|&v| mesh.verts.get(v as usize).map(|v| v.vec()))
                        .collect();
                    let center = points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32;
                    let radius = points
                        .iter()
                        .map(|p| p.distance(center))
                        .fold(0.0f32, f32::max);
                    (mesh.face_normal(fi).normalize_or_zero(), center, radius)
                })
                .collect();
            for dab_in in &dabs {
                let n = target.buf.dabs;
                target.buf.dabs = n.wrapping_add(1);
                let jitter = dab_jitter(&style, n);
                let r_world = (dab_in.radius * jitter.radius_scale).max(1.0e-5);
                // Base tangente ao ponto de impacto (ponta, ângulo e espalhamento).
                let normal = if dab_in.normal == Vec3::ZERO {
                    Vec3::Y
                } else {
                    dab_in.normal
                };
                let reference = if normal.y.abs() > 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                let u = normal.cross(reference).normalize_or_zero();
                let v = normal.cross(u);
                let scatter = jitter.scatter.1 * r_world;
                let hit = dab_in.hit
                    + (u * jitter.scatter.0.cos() + v * jitter.scatter.0.sin()) * scatter;
                let mut dab = params;
                dab.cap *= jitter.cap_scale;
                dab.blur_radius = 2;
                if dab.kind == BrushType::Smudge
                    && let Some(texel) = dab_in.texel
                {
                    let delta = target
                        .buf
                        .last_texel
                        .map_or([0.0; 2], |l| [texel[0] - l[0], texel[1] - l[1]]);
                    // Salto entre charts da textura não é movimento do pincel.
                    let jump = delta[0].hypot(delta[1]) > 0.25 * w.max(h) as f32;
                    dab.delta = if jump { [0.0; 2] } else { delta };
                    target.buf.last_texel = Some(texel);
                }
                if dab.kind == BrushType::Clone {
                    if target.buf.clone_offset.is_none()
                        && let (Some(src), Some(texel)) = (clone_source, dab_in.texel)
                    {
                        target.buf.clone_offset = Some([
                            (src[0] - texel[0]).round() as i32,
                            (src[1] - texel[1]).round() as i32,
                        ]);
                    }
                    dab.clone = target.buf.clone_offset;
                }
                for (fi, &(face_normal, face_center, face_radius)) in infos.iter().enumerate() {
                    if restriction.as_ref().is_some_and(|r| !r.allows_face(fi)) {
                        continue;
                    }
                    if face_center.distance(hit)
                        > face_radius + r_world + 0.02 * face_radius.max(0.01)
                    {
                        continue;
                    }
                    // Não pinta a superfície voltada para o lado oposto.
                    if dab_in.normal != Vec3::ZERO && face_normal.dot(dab_in.normal) < -0.5 {
                        continue;
                    }
                    mesh.rasterize_face_near(
                        fi,
                        hit,
                        r_world,
                        w,
                        h,
                        BLEED_PX,
                        |x, y, t_dist, pos, r_eff| {
                            let t = if plain_round {
                                t_dist
                            } else {
                                let d = pos - hit;
                                style.tip_distance(d.dot(u) / r_eff, d.dot(v) / r_eff)
                            };
                            target.apply(
                                x,
                                y,
                                dab_falloff(dab.kind, t, dab.hardness)
                                    * stencil
                                        .as_ref()
                                        .map_or(1.0, |stencil| stencil_weight(stencil, pos)),
                                &dab,
                            );
                        },
                    );
                }
            }
        });
        let Some(((), dirty)) = result else {
            return DirtyTiles::default();
        };
        dirty
    }
}
