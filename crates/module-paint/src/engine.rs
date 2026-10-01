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
use petunia_core::{AppState, BrushLock, BrushSettings, BrushType, PaintRestriction, StrokeBuffer};
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
}

impl DabParams {
    fn new(settings: BrushSettings, color: [u8; 4]) -> Self {
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
        }
    }
}

/// Canvas + buffer + tiles sujos de um traço em andamento.
struct Target<'a> {
    cv: &'a mut Canvas,
    buf: &'a mut StrokeBuffer,
    dirty: Vec<bool>,
    tiles_x: u32,
}

impl Target<'_> {
    #[inline]
    fn apply(&mut self, x: u32, y: u32, weight: f32, p: &DabParams) {
        if x >= self.buf.w || y >= self.buf.h || weight <= 0.0 {
            return;
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
        self.buf.coverage[i] = cov1;
        let base = &self.buf.base[i * 4..i * 4 + 4];
        let mut out = [base[0], base[1], base[2], base[3]];
        if p.kind == BrushType::Eraser {
            out[3] = (base[3] as f32 * (1.0 - cov1 * p.cap).max(0.0)) as u8;
        } else {
            let m = if p.kind == BrushType::Pixel {
                1.0
            } else {
                (cov1 * p.cap).clamp(0.0, 1.0)
            };
            for c in 0..4 {
                out[c] = (base[c] as f32 * (1.0 - m) + p.color[c] as f32 * m).round() as u8;
            }
        }
        self.cv.pixels[i * 4..i * 4 + 4].copy_from_slice(&out);
        let tile = (y / TILE_SIZE) * self.tiles_x + x / TILE_SIZE;
        if let Some(flag) = self.dirty.get_mut(tile as usize) {
            *flag = true;
        }
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
        let active = state.project.active;
        let asset = state.project.assets.get_mut(active)?;
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
            *buf_slot = Some(StrokeBuffer::new(layer_id, cv.w, cv.h, cv.pixels.clone()));
        }
        let buf = buf_slot.as_mut()?;
        let tiles_x = cv.w.div_ceil(TILE_SIZE).max(1);
        let tiles_y = cv.h.div_ceil(TILE_SIZE).max(1);
        let mut target = Target {
            cv,
            buf,
            dirty: vec![false; (tiles_x * tiles_y) as usize],
            tiles_x,
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
        let params = DabParams::new(settings, Self::paint_color_rgba(state));
        let radius = settings.sanitized().radius_px();
        let result = Self::with_stroke_target(state, |target, mesh| {
            let mask = restriction.as_ref().map(|r| r.mask(mesh, w, h));
            let r_f = radius as f32;
            for &(cx, cy) in points {
                let (x0, x1) = (cx.saturating_sub(radius), (cx + radius).min(w - 1));
                let (y0, y1) = (cy.saturating_sub(radius), (cy + radius).min(h - 1));
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let dx = x as f32 - cx as f32;
                        let dy = y as f32 - cy as f32;
                        let t = (dx * dx + dy * dy).sqrt() / r_f.max(0.5);
                        if t > 1.0 {
                            continue;
                        }
                        if mask.is_some_and(|m| !m.allows(x, y)) {
                            continue;
                        }
                        target.apply(x, y, dab_falloff(params.kind, t, params.hardness), &params);
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
        let params = DabParams::new(settings, Self::paint_color_rgba(state));
        let size_px = settings.sanitized().size_px;
        let (sx, sy, sz) = (
            state.session.tools.paint_symmetry_x,
            state.session.tools.paint_symmetry_y,
            state.session.tools.paint_symmetry_z,
        );
        // (ponto, normal da face de origem, raio de mundo)
        let mut dabs: Vec<(Vec3, Vec3, f32)> = Vec::with_capacity(hits.len());
        if let Some(mesh) = state.project.active_mesh() {
            for &(face, hit) in hits {
                if face >= mesh.faces.len() {
                    continue;
                }
                let normal = mesh.face_normal(face).normalize_or_zero();
                let r = state.world_radius_for_px(hit, size_px);
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
                    dabs.push((hit * m, normal * m, r));
                }
            }
        }
        let result = Self::with_stroke_target(state, |target, mesh| {
            for &(hit, normal, r_world) in &dabs {
                for fi in 0..mesh.faces.len() {
                    if restriction.as_ref().is_some_and(|r| !r.allows_face(fi)) {
                        continue;
                    }
                    // Não pinta a superfície voltada para o lado oposto.
                    if normal != Vec3::ZERO
                        && mesh.face_normal(fi).normalize_or_zero().dot(normal) < -0.5
                    {
                        continue;
                    }
                    mesh.rasterize_face_near(fi, hit, r_world, w, h, BLEED_PX, |x, y, t| {
                        target.apply(x, y, dab_falloff(params.kind, t, params.hardness), &params);
                    });
                }
            }
        });
        let Some(((), dirty)) = result else {
            return DirtyTiles::default();
        };
        dirty
    }
}
