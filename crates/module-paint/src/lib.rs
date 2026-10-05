//! module-paint — workspace PAINT (P3D-055 a P3D-062, P3D-132).
//!
//! Motor de pintura 2D e 3D por projeção UV:
//! - Pincéis: Pixel Brush rígido (P3D-056), Soft Brush com atenuação suave (P3D-057),
//!   Borracha (P3D-058), Flood Fill (P3D-059), Conta-gotas / Eyedropper (P3D-060),
//!   Linha e Retângulo (cap. 15/44, raster ops simples).
//! - Pilha de camadas por asset em `petunia_project` (P3D-061); este crate
//!   re-exporta os tipos e implementa as operações sobre a camada ativa.
//! - Pintura direta sobre malha 3D via coordenadas baricêntricas e UV (P3D-062).
//! - Isolamento de seleção / Paint Masks (P3D-132).
//! - Sincronização direta com o modelo canônico de Material (P3D-050).

mod engine;
mod path_paint;
pub mod projection;
mod surface_commands;
pub use engine::dab_falloff;
pub use path_paint::{
    MAX_LINE_PIXELS, MAX_PATH_DABS, MAX_PATH_NODES, SurfaceHit, line_pixels, pixel_perfect,
    project_path_onto_surface, sample_path, snap_to_surface,
};
pub use surface_commands::{PaintPathCmd, PathPaintMode, ProjectionPaintCmd};

use std::collections::VecDeque;

use glam::Vec3;
use petunia_core::{AppState, Module};
use petunia_project::Canvas;

pub use petunia_project::paint_layers::{
    DecalLayer, LayerBlendMode, LayerKind, PaintEffect, PaintLayer, PaintLayerStack, blend_pixels,
};

/// `BrushType`/`BrushSettings` vivem em `petunia_core` (descriptor único,
/// iniciativa Paint — P3D-056/057). Re-exportados aqui por compatibilidade
/// com call sites existentes (`petunia_module_paint::BrushType`).
pub use petunia_core::{BrushSettings, BrushType};

/// Lado (px) da textura de pintura criada para um objeto sem textura.
/// 1024 dá detalhe de decalque e traço sem estourar o orçamento do histórico.
pub const DEFAULT_TEXTURE_SIDE: u32 = 1024;

/// Lados que o usuário pode escolher para a textura de pintura.
pub const TEXTURE_SIDES: [u32; 4] = [256, 512, 1024, 2048];

/// Tiles alterados por uma amostra consolidada de pintura raster.
///
/// A lista é normalizada antes de sair do módulo: índices ordenados, únicos e
/// limitados às dimensões do canvas. Ela é o contrato entre mutação, compositor
/// parcial e upload parcial da textura.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtyTiles {
    indices: Vec<u32>,
}

impl DirtyTiles {
    pub fn as_slice(&self) -> &[u32] {
        &self.indices
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    #[cfg(test)]
    fn mark_dab(&mut self, x: u32, y: u32, radius: u32, canvas_w: u32, canvas_h: u32) {
        use petunia_project::paint_layers::TILE_SIZE;

        if canvas_w == 0 || canvas_h == 0 || x >= canvas_w || y >= canvas_h {
            return;
        }
        let tiles_x = canvas_w.div_ceil(TILE_SIZE).max(1);
        let x0 = x.saturating_sub(radius).min(canvas_w - 1);
        let y0 = y.saturating_sub(radius).min(canvas_h - 1);
        let x1 = x.saturating_add(radius).min(canvas_w - 1);
        let y1 = y.saturating_add(radius).min(canvas_h - 1);
        for ty in (y0 / TILE_SIZE)..=(y1 / TILE_SIZE) {
            for tx in (x0 / TILE_SIZE)..=(x1 / TILE_SIZE) {
                self.indices.push(ty * tiles_x + tx);
            }
        }
    }

    fn mark_all(&mut self, canvas_w: u32, canvas_h: u32) {
        use petunia_project::paint_layers::TILE_SIZE;

        let tiles_x = canvas_w.div_ceil(TILE_SIZE);
        let tiles_y = canvas_h.div_ceil(TILE_SIZE);
        self.indices
            .extend((0..tiles_y).flat_map(|ty| (0..tiles_x).map(move |tx| ty * tiles_x + tx)));
    }

    fn normalize(&mut self) {
        self.indices.sort_unstable();
        self.indices.dedup();
    }
}

/// Traço de forma (Line/Rectangle) com estilo do pincel ativo.
#[derive(Clone, Copy, Debug)]
pub struct ShapeStroke {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    pub brush: BrushType,
    pub color: [u8; 4],
    pub strength: f32,
}

#[derive(Default)]
pub struct PaintModule;

impl PaintModule {
    pub fn new() -> Self {
        Self
    }

    /// Preenche a seleção de vértices (ou toda a malha se nada estiver selecionado) com a cor atual.
    pub fn fill_selection(state: &mut AppState) -> usize {
        let before = state.project.project.clone();
        let col = state.paint_color;
        let mut n = 0;
        if let Some(m) = state.project.active_mesh_mut() {
            let any = m.verts.iter().any(|v| v.selected);
            for v in &mut m.verts {
                if v.selected || !any {
                    v.color = col;
                    n += 1;
                }
            }
        }
        if n > 0 {
            state.project.checkpoint_snapshot("fill", &before);
            state.emit_color_changed();
        }
        n
    }

    /// Pinta cores diretamente nos vértices da malha dentro de um raio esférico no espaço 3D (P3D-150).
    pub fn paint_vertex_color_3d(
        state: &mut AppState,
        hit_pos: Vec3,
        radius_world: f32,
        strength: f32,
        color: [f32; 3],
    ) -> usize {
        Self::paint_vertex_color_3d_batch(state, &[hit_pos], radius_world, strength, color)
    }

    /// Aplica todos os dabs de vertex paint e publica uma única revisão de cor.
    pub fn paint_vertex_color_3d_batch(
        state: &mut AppState,
        hit_positions: &[Vec3],
        radius_world: f32,
        strength: f32,
        color: [f32; 3],
    ) -> usize {
        let mut affected = 0;
        let Some(mesh) = state.project.active_mesh_mut() else {
            return 0;
        };
        let r = radius_world.max(0.01);
        let r_sq = r * r;
        for &hit_pos in hit_positions {
            for v in &mut mesh.verts {
                let p = v.vec();
                let d_sq = (p - hit_pos).length_squared();
                if d_sq <= r_sq {
                    let dist = d_sq.sqrt();
                    let t = (1.0 - (dist / r)).max(0.0);
                    let factor = (t * t * strength).clamp(0.0, 1.0);
                    for (dst, &src) in v.color.iter_mut().zip(color.iter()) {
                        *dst = *dst * (1.0 - factor) + src * factor;
                    }
                    affected += 1;
                }
            }
        }
        if affected > 0 {
            state.emit_color_changed();
            state.mark_dirty();
        }
        affected
    }

    /// Eyedropper: copia a cor do vértice para o pincel.
    pub fn eyedrop_vertex(state: &mut AppState, vi: usize) {
        if let Some(color) = state
            .project
            .assets
            .get(state.project.active)
            .and_then(|asset| asset.mesh.verts.get(vi))
            .map(|vertex| vertex.color)
        {
            state.paint_color = color;
            state.session.tools.paint_color = color;
            state.mark_dirty();
        }
    }

    /// Registra cor na palette (recentes, máx 16) no ProjectState.
    pub fn push_palette(state: &mut AppState, c: [f32; 3]) {
        state
            .project
            .palette
            .retain(|&x| (x[0] - c[0]).abs() + (x[1] - c[1]).abs() + (x[2] - c[2]).abs() > 1e-3);
        state.project.palette.insert(0, c);
        state.project.palette.truncate(16);
    }

    /// Substitui a paleta atual no ProjectState.
    pub fn set_palette(state: &mut AppState, pal: Vec<[f32; 3]>) {
        state.project.palette = pal;
        state.mark_dirty();
    }

    /// Importa paleta a partir de arquivo (.hex ou .gpl) usando o ProjectService.
    pub fn import_palette_file(
        state: &mut AppState,
        path: &std::path::Path,
    ) -> Result<usize, petunia_core::ProjectServiceError> {
        petunia_core::ProjectService::import_palette(state, path)
    }

    /// Exporta a paleta atual para arquivo (.gpl) usando o ProjectService.
    pub fn export_palette_file(
        state: &AppState,
        path: &std::path::Path,
    ) -> Result<(), petunia_core::ProjectServiceError> {
        petunia_core::ProjectService::export_palette(
            &state.project.palette,
            "Petunia Palette",
            path,
        )
    }

    // ---- Pilha de camadas por asset (P3D-061) ----

    /// Garante a pilha de camadas do ativo: migra a `texture` existente para
    /// a camada base (sem perda) ou cria base nova. Sem checkpoint aqui —
    /// quem chama decide a transação.
    pub fn ensure_stack(state: &mut AppState) {
        Self::ensure_canvas(state);
        let active_idx = state.project.active;
        let needs = !state
            .project
            .assets
            .get(active_idx)
            .is_some_and(|a| a.paint_stack.is_some());
        if !needs {
            return;
        }
        let base = state
            .project
            .assets
            .get(active_idx)
            .and_then(|a| a.texture.clone())
            .unwrap_or_else(|| {
                Canvas::new(DEFAULT_TEXTURE_SIDE, DEFAULT_TEXTURE_SIDE, [0, 0, 0, 0])
            });
        if let Some(o) = state.project.assets.get_mut(active_idx) {
            o.paint_stack = Some(PaintLayerStack::with_base("Base", base));
        }
    }

    /// Recompõe o stack na `texture` do ativo e sincroniza o Albedo.
    /// Chamar após qualquer mutação de camada (sem checkpoint próprio).
    ///
    /// Canonical raster is `Asset.paint_stack`. `Asset.texture` is the composed
    /// cache. `Material.albedo_texture` is a derived alias of that cache.
    pub fn composite_active(state: &mut AppState) {
        Self::composite_active_tiles(state, &[]);
    }

    /// Partial composition of dirty tiles when the stack is tileable.
    /// Empty `dirty_tiles` falls back to a full composite.
    pub fn composite_active_tiles(state: &mut AppState, dirty_tiles: &[u32]) {
        let active_idx = state.project.active;
        let (w, h, tileable, has_stack) = match state.project.assets.get(active_idx) {
            Some(a) => {
                let (w, h) = a
                    .texture
                    .as_ref()
                    .map(|c| (c.w, c.h))
                    .unwrap_or((DEFAULT_TEXTURE_SIDE, DEFAULT_TEXTURE_SIDE));
                let tileable = a.paint_stack.as_ref().is_some_and(|s| s.is_tileable());
                (w, h, tileable, a.paint_stack.is_some())
            }
            None => return,
        };
        if !has_stack {
            return;
        }
        let partial = tileable && !dirty_tiles.is_empty();
        let time = state.session.decal_time;
        if partial {
            if let Some(asset) = state.project.assets.get_mut(active_idx)
                && let Some(stack) = asset.paint_stack.as_ref()
            {
                let canvas = asset
                    .texture
                    .get_or_insert_with(|| Canvas::new(w, h, [0, 0, 0, 0]));
                stack.composite_tiles_on(canvas, dirty_tiles, Some(&*asset.mesh), time);
            }
        } else {
            state
                .project
                .project
                .composite_paint_stack_at(active_idx, time);
        }
        if partial {
            Self::sync_material_tiles(state, active_idx, dirty_tiles);
        }
        if partial {
            state.emit_texture_changed_regions(Self::dirty_tile_regions(dirty_tiles, w, h));
        } else {
            state.emit_texture_changed();
        }
        state.render.canvas_dirty = true;
        state.mark_dirty();
    }

    fn dirty_tile_regions(
        dirty_tiles: &[u32],
        canvas_width: u32,
        canvas_height: u32,
    ) -> Vec<petunia_core::TextureDirtyRect> {
        use petunia_project::paint_layers::TILE_SIZE;

        let tiles_x = canvas_width.div_ceil(TILE_SIZE).max(1);
        dirty_tiles
            .iter()
            .filter_map(|&tile| {
                let x = (tile % tiles_x) * TILE_SIZE;
                let y = (tile / tiles_x) * TILE_SIZE;
                (x < canvas_width && y < canvas_height).then_some(petunia_core::TextureDirtyRect {
                    x,
                    y,
                    width: (x + TILE_SIZE).min(canvas_width) - x,
                    height: (y + TILE_SIZE).min(canvas_height) - y,
                })
            })
            .collect()
    }

    fn sync_material_tiles(state: &mut AppState, active_idx: usize, dirty_tiles: &[u32]) {
        use petunia_project::paint_layers::TILE_SIZE;

        let project = &mut state.project.project;
        let (assets, materials) = (&project.assets, &mut project.materials);
        let Some(asset) = assets.get(active_idx) else {
            return;
        };
        let (Some(material_id), Some(source)) = (asset.material_id, asset.texture.as_ref()) else {
            return;
        };
        let Some(material) = materials
            .iter_mut()
            .find(|material| material.id == material_id)
        else {
            return;
        };
        let Some(target) = material.albedo_texture.as_mut() else {
            material.albedo_texture = Some(source.clone());
            return;
        };
        if (target.w, target.h) != (source.w, source.h)
            || target.pixels.len() != source.pixels.len()
        {
            *target = source.clone();
            return;
        }

        let tiles_x = source.w.div_ceil(TILE_SIZE).max(1);
        for &tile in dirty_tiles {
            let tile_x = tile % tiles_x;
            let tile_y = tile / tiles_x;
            let x0 = tile_x * TILE_SIZE;
            let y0 = tile_y * TILE_SIZE;
            let x1 = (x0 + TILE_SIZE).min(source.w);
            let y1 = (y0 + TILE_SIZE).min(source.h);
            if x0 >= x1 || y0 >= y1 {
                continue;
            }
            for y in y0..y1 {
                let start = ((y * source.w + x0) * 4) as usize;
                let end = ((y * source.w + x1) * 4) as usize;
                target.pixels[start..end].copy_from_slice(&source.pixels[start..end]);
            }
        }
    }

    /// Redimensiona canvas base + todas as camadas raster (operação explícita;
    /// quem chama faz checkpoint antes).
    pub fn resize_canvas(state: &mut AppState, w: u32, h: u32) {
        Self::ensure_stack(state);
        let active_idx = state.project.active;
        if let Some(o) = state.project.assets.get_mut(active_idx) {
            if let Some(cv) = o.texture.as_ref() {
                o.texture = Some(cv.resized(w, h));
            }
            if let Some(stack) = o.paint_stack.as_mut() {
                for layer in &mut stack.layers {
                    if let Some(cv) = layer.canvas_mut() {
                        let next = cv.resized(w, h);
                        *cv = next;
                    }
                }
            }
        }
        Self::composite_active(state);
    }

    /// Retângulo de texels que o decalque ativo cobre na textura do ativo.
    pub fn active_decal_bounds(state: &AppState) -> Option<[u32; 4]> {
        let asset = state.project.active()?;
        let texture = asset.texture.as_ref()?;
        let LayerKind::Decal(decal) = &asset.paint_stack.as_ref()?.active()?.kind else {
            return None;
        };
        decal.texel_bounds(Some(&*asset.mesh), texture.w, texture.h)
    }

    /// Recompõe só os tiles que o decalque ativo cobria (`before`) ou cobre
    /// agora (P3D-061: mover um decalque não recompõe a textura inteira).
    pub fn composite_decal_region(state: &mut AppState, before: Option<[u32; 4]>) {
        let Some((w, h)) = state
            .project
            .active()
            .and_then(|asset| asset.texture.as_ref())
            .map(|texture| (texture.w, texture.h))
        else {
            Self::composite_active(state);
            return;
        };
        let mut tiles: Vec<u32> = [before, Self::active_decal_bounds(state)]
            .into_iter()
            .flatten()
            .flat_map(|bounds| PaintLayerStack::tiles_for_bounds(bounds, w, h))
            .collect();
        tiles.sort_unstable();
        tiles.dedup();
        if tiles.is_empty() {
            // Nada coberto antes nem agora: nenhum texel muda.
            return;
        }
        Self::composite_active_tiles(state, &tiles);
    }

    /// Textura composta do ativo com cada variante do decalque `layer_id`
    /// (exportação em quadros do Decal Set, cap. 39 "bake universal").
    pub fn decal_variant_textures(state: &AppState, layer_id: uuid::Uuid) -> Vec<(String, Canvas)> {
        let Some(asset) = state.project.active() else {
            return Vec::new();
        };
        let Some(stack) = asset.paint_stack.as_ref() else {
            return Vec::new();
        };
        let Some(layer) = stack.layers.iter().find(|layer| layer.id == layer_id) else {
            return Vec::new();
        };
        let LayerKind::Decal(decal) = &layer.kind else {
            return Vec::new();
        };
        let (w, h) = asset
            .texture
            .as_ref()
            .map_or((DEFAULT_TEXTURE_SIDE, DEFAULT_TEXTURE_SIDE), |c| (c.w, c.h));
        (0..decal.variant_count())
            .map(|index| {
                let mut frame_stack = stack.clone();
                if let Some(LayerKind::Decal(variant)) = frame_stack
                    .layers
                    .iter_mut()
                    .find(|l| l.id == layer_id)
                    .map(|l| &mut l.kind)
                {
                    variant.set_variant(index);
                }
                let mut canvas = Canvas::new(w, h, [0, 0, 0, 0]);
                frame_stack.composite_on(&mut canvas, Some(&*asset.mesh), None);
                (decal.variant_name(index, &layer.name).to_owned(), canvas)
            })
            .collect()
    }

    /// Troca a resolução da textura de pintura do ativo (quadrada, um dos
    /// [`TEXTURE_SIDES`]), reamostrando todas as camadas raster, numa entrada
    /// do histórico. `false` se o lado não é permitido, já é o atual ou o
    /// objeto está travado.
    pub fn set_texture_side(state: &mut AppState, side: u32) -> bool {
        if !TEXTURE_SIDES.contains(&side) {
            return false;
        }
        let Some(asset) = state.project.active() else {
            return false;
        };
        if asset.locked || asset.texture.as_ref().map(|c| (c.w, c.h)) == Some((side, side)) {
            return false;
        }
        state.checkpoint("texture resolution");
        Self::resize_canvas(state, side, side);
        state.render.canvas_dirty = true;
        state.mark_dirty();
        true
    }

    pub fn has_canvas(state: &AppState) -> bool {
        if let Some(o) = state.project.assets.get(state.project.active) {
            if o.texture.is_some() {
                return true;
            }
            if let Some(mat) = o.material(&state.project.project) {
                return mat.albedo_texture.is_some();
            }
        }
        false
    }

    /// Garante que o canvas existe no asset e no material associado.
    pub fn ensure_canvas(state: &mut AppState) {
        let active_idx = state.project.active;
        let mut base_col = [0.75, 0.75, 0.78];
        let mut mat_id = None;

        if let Some(o) = state.project.assets.get(active_idx) {
            base_col = o.base_color;
            mat_id = o.material_id;
        }

        let fill_rgba = [
            (base_col[0] * 255.0) as u8,
            (base_col[1] * 255.0) as u8,
            (base_col[2] * 255.0) as u8,
            255,
        ];

        // Sincroniza no Material associado se houver
        if let Some(mid) = mat_id
            && let Some(mat) = state.project.project.get_material_mut(mid)
            && mat.albedo_texture.is_none()
        {
            mat.albedo_texture = Some(Canvas::new(
                DEFAULT_TEXTURE_SIDE,
                DEFAULT_TEXTURE_SIDE,
                fill_rgba,
            ));
        }

        // Sincroniza no Asset
        if let Some(o) = state.project.active_mut()
            && o.texture.is_none()
        {
            o.texture = Some(Canvas::new(
                DEFAULT_TEXTURE_SIDE,
                DEFAULT_TEXTURE_SIDE,
                fill_rgba,
            ));
            state.mark_dirty();
        }
    }

    /// Aplica carimbo do pincel rígido pixel-perfect (P3D-056).
    pub fn stamp_pixel_brush(canvas: &mut Canvas, cx: u32, cy: u32, radius: u32, color: [u8; 4]) {
        let r = radius as i32;
        let r2 = r * r;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r2 {
                    let px = (cx as i32 + dx).max(0) as u32;
                    let py = (cy as i32 + dy).max(0) as u32;
                    canvas.set(px, py, color);
                }
            }
        }
    }

    /// Aplica carimbo do pincel suave com atenuação quadrática (P3D-057).
    pub fn stamp_soft_brush(
        canvas: &mut Canvas,
        cx: u32,
        cy: u32,
        radius: u32,
        color: [u8; 4],
        strength: f32,
    ) {
        Self::stamp_soft_brush_hard(canvas, cx, cy, radius, color, strength, 0.0);
    }

    /// Carimbo suave com dureza explícita (iniciativa Paint).
    ///
    /// Semântica Photoshop-like de duas zonas: núcleo sólido de raio
    /// `hardness * radius` + anel externo com falloff quadrático suave.
    /// - `hardness = 1` → borda rígida (equivale ao Pixel no centro do dab);
    /// - `hardness = 0` → falloff quadrático puro (comportamento legado exato).
    pub fn stamp_soft_brush_hard(
        canvas: &mut Canvas,
        cx: u32,
        cy: u32,
        radius: u32,
        color: [u8; 4],
        strength: f32,
        hardness: f32,
    ) {
        let r = radius as i32;
        let r_f = radius as f32;
        let str_k = strength.clamp(0.0, 1.0);
        let core = hardness.clamp(0.0, 1.0) * r_f;

        for dy in -r..=r {
            for dx in -r..=r {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist > r_f {
                    continue;
                }
                let factor = if dist <= core {
                    str_k
                } else if core >= r_f {
                    0.0
                } else {
                    let t = (dist - core) / (r_f - core).max(1e-4);
                    (1.0 - t) * (1.0 - t) * str_k
                };
                if factor <= 0.0 {
                    continue;
                }
                let px = (cx as i32 + dx).max(0) as u32;
                let py = (cy as i32 + dy).max(0) as u32;

                if let Some(bg) = canvas.get(px, py) {
                    let blended = [
                        (bg[0] as f32 * (1.0 - factor) + color[0] as f32 * factor) as u8,
                        (bg[1] as f32 * (1.0 - factor) + color[1] as f32 * factor) as u8,
                        (bg[2] as f32 * (1.0 - factor) + color[2] as f32 * factor) as u8,
                        (bg[3] as f32 * (1.0 - factor) + color[3] as f32 * factor) as u8,
                    ];
                    canvas.set(px, py, blended);
                }
            }
        }
    }

    /// Aplica carimbo de borracha atenuando ou removendo o alfa (P3D-058).
    pub fn stamp_eraser(canvas: &mut Canvas, cx: u32, cy: u32, radius: u32, strength: f32) {
        let r = radius as i32;
        let r_f = radius as f32;
        let str_k = strength.clamp(0.0, 1.0);

        for dy in -r..=r {
            for dx in -r..=r {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist <= r_f {
                    let factor = (1.0 - (dist / r_f)) * str_k;
                    let px = (cx as i32 + dx).max(0) as u32;
                    let py = (cy as i32 + dy).max(0) as u32;

                    if let Some(mut bg) = canvas.get(px, py) {
                        bg[3] = (bg[3] as f32 * (1.0 - factor).max(0.0)) as u8;
                        canvas.set(px, py, bg);
                    }
                }
            }
        }
    }

    /// Linha de Bresenham com o pincel atual (Pixel=sólido, Soft=carimbos
    /// espaçados, Eraser=apaga). Uma transaction (quem chama faz checkpoint).
    pub fn stroke_line(canvas: &mut Canvas, stroke: &ShapeStroke) {
        let (mut x0, mut y0) = (stroke.x0 as i32, stroke.y0 as i32);
        let (x1, y1) = (stroke.x1 as i32, stroke.y1 as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut step = 0u32;
        loop {
            if x0 >= 0 && y0 >= 0 {
                let (px, py) = (x0 as u32, y0 as u32);
                match stroke.brush {
                    BrushType::Soft => {
                        // Carimbo espaçado p/ não empilhar opacidade no traço.
                        if step.is_multiple_of(2) {
                            Self::stamp_soft_brush(
                                canvas,
                                px,
                                py,
                                1,
                                stroke.color,
                                stroke.strength,
                            );
                        }
                    }
                    BrushType::Eraser => Self::stamp_eraser(canvas, px, py, 1, stroke.strength),
                    _ => {
                        canvas.set(px, py, stroke.color);
                    }
                }
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
            step += 1;
        }
    }

    /// Elipse preenchida inscrita no retângulo entre dois cantos (ordem qualquer).
    pub fn stroke_ellipse(canvas: &mut Canvas, stroke: &ShapeStroke) {
        let (xa, xb) = (stroke.x0.min(stroke.x1), stroke.x0.max(stroke.x1));
        let (ya, yb) = (stroke.y0.min(stroke.y1), stroke.y0.max(stroke.y1));
        let (cx, cy) = ((xa + xb) as f32 * 0.5, (ya + yb) as f32 * 0.5);
        // Meia-largura/altura em texels (pelo menos meio texel: um clique é um ponto).
        let rx = ((xb - xa) as f32 * 0.5).max(0.5);
        let ry = ((yb - ya) as f32 * 0.5).max(0.5);
        for y in ya..=yb {
            for x in xa..=xb {
                let nx = (x as f32 - cx) / rx;
                let ny = (y as f32 - cy) / ry;
                if nx * nx + ny * ny <= 1.0 {
                    canvas.set(x, y, stroke.color);
                }
            }
        }
    }

    /// Retângulo preenchido entre dois cantos (ordem qualquer).
    pub fn stroke_rect(canvas: &mut Canvas, stroke: &ShapeStroke) {
        let (xa, xb) = (stroke.x0.min(stroke.x1), stroke.x0.max(stroke.x1));
        let (ya, yb) = (stroke.y0.min(stroke.y1), stroke.y0.max(stroke.y1));
        match stroke.brush {
            BrushType::Eraser => {
                for y in ya..=yb {
                    for x in xa..=xb {
                        Self::stamp_eraser(canvas, x, y, 0, stroke.strength);
                    }
                }
            }
            BrushType::Soft => {
                for y in ya..=yb {
                    for x in xa..=xb {
                        // Raio 1: raio 0 degenera a atenuação (0/0).
                        Self::stamp_soft_brush(canvas, x, y, 1, stroke.color, stroke.strength);
                    }
                }
            }
            _ => {
                for y in ya..=yb {
                    for x in xa..=xb {
                        canvas.set(x, y, stroke.color);
                    }
                }
            }
        }
    }

    /// Preenchimento flood-fill 4-conectado com tolerância de cor (P3D-059).
    pub fn flood_fill(
        canvas: &mut Canvas,
        start_x: u32,
        start_y: u32,
        new_color: [u8; 4],
        tolerance: u8,
    ) {
        if start_x >= canvas.w || start_y >= canvas.h {
            return;
        }
        let target_color = match canvas.get(start_x, start_y) {
            Some(c) => c,
            None => return,
        };

        if colors_match(target_color, new_color, 0) {
            return;
        }

        let mut queue = VecDeque::new();
        let mut visited = vec![false; (canvas.w * canvas.h) as usize];

        queue.push_back((start_x, start_y));
        let idx = (start_y * canvas.w + start_x) as usize;
        visited[idx] = true;

        let w = canvas.w as i32;
        let h = canvas.h as i32;

        while let Some((x, y)) = queue.pop_front() {
            canvas.set(x, y, new_color);

            let neighbors = [
                (x as i32 + 1, y as i32),
                (x as i32 - 1, y as i32),
                (x as i32, y as i32 + 1),
                (x as i32, y as i32 - 1),
            ];

            for (nx, ny) in neighbors {
                if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    let ux = nx as u32;
                    let uy = ny as u32;
                    let uidx = (uy * canvas.w + ux) as usize;
                    if !visited[uidx] {
                        visited[uidx] = true;
                        if let Some(col) = canvas.get(ux, uy)
                            && colors_match(col, target_color, tolerance)
                        {
                            queue.push_back((ux, uy));
                        }
                    }
                }
            }
        }
    }

    /// Pinta no canvas 2D com suporte aos 5 tipos de pincéis (API legado).
    /// Delega para o descriptor canônico ([`BrushSettings`]).
    pub fn canvas_brush_advanced(
        state: &mut AppState,
        x: u32,
        y: u32,
        brush: BrushType,
        radius: u32,
        strength: f32,
    ) {
        let settings = BrushSettings {
            kind: brush,
            size_px: (radius.max(1) * 2) as f32,
            hardness: state.session.tools.brush_hardness,
            strength,
            flow: state.session.tools.brush_flow,
            spacing: state.session.tools.brush_spacing,
        };
        Self::canvas_brush_with_settings(state, x, y, settings);
    }

    /// Pinta no canvas 2D com o descriptor canônico (iniciativa Paint).
    ///
    /// Carimba um único dab. A interpolação incremental do traço entre eventos
    /// é responsabilidade de quem alimenta os pontos — use
    /// [`petunia_core::StrokeSampler`] para preservar o residual entre eventos.
    pub fn canvas_brush_with_settings(
        state: &mut AppState,
        x: u32,
        y: u32,
        settings: BrushSettings,
    ) {
        Self::canvas_brush_batch(state, &[(x, y)], settings);
    }

    fn canvas_brush_batch(
        state: &mut AppState,
        points: &[(u32, u32)],
        settings: BrushSettings,
    ) -> DirtyTiles {
        if points.is_empty() {
            return DirtyTiles::default();
        }
        Self::ensure_stack(state);
        let s = settings.sanitized();
        let color = [
            (state.paint_color[0] * 255.0) as u8,
            (state.paint_color[1] * 255.0) as u8,
            (state.paint_color[2] * 255.0) as u8,
            255,
        ];

        let active_idx = state.project.active;
        // Conta-gotas amostra o composto (o que o usuário vê — P3D-060).
        if s.kind == BrushType::Eyedropper {
            if let Some(&(x, y)) = points.last()
                && let Some(o) = state.project.assets.get(active_idx)
                && let Some(cv) = o.texture.as_ref()
                && x < cv.w
                && y < cv.h
                && let Some(c) = cv.get(x, y)
            {
                let picked = [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ];
                state.paint_color = picked;
                state.session.tools.paint_color = picked;
            }
            return DirtyTiles::default();
        }
        if matches!(s.kind, BrushType::Line | BrushType::Rectangle) {
            return DirtyTiles::default();
        }

        let mut dirty = if s.kind == BrushType::Fill {
            // O balde preenche uma região conectada: não usa dabs nem buffer.
            let mut dirty = DirtyTiles::default();
            if let Some(o) = state.project.assets.get_mut(active_idx)
                && let Some(stack) = o.paint_stack.as_mut()
                && let Some(layer) = stack.active_mut()
                && !layer.locked
                && let Some(cv) = layer.canvas_mut()
            {
                for &(x, y) in points {
                    Self::flood_fill(cv, x, y, color, 16);
                }
                dirty.mark_all(cv.w, cv.h);
            }
            dirty
        } else {
            Self::stamp_dabs_2d(state, points, s)
        };
        dirty.normalize();
        if !dirty.is_empty() {
            Self::composite_active_tiles(state, dirty.as_slice());
        }
        dirty
    }

    /// Pinta no canvas 2D aplicando simetria em tempo real nos eixos X e Y se habilitados.
    pub fn canvas_brush_with_symmetry(
        state: &mut AppState,
        x: u32,
        y: u32,
        settings: BrushSettings,
    ) {
        Self::canvas_brush_batch_with_symmetry(state, &[(x, y)], settings);
    }

    /// Pinta vários dabs e suas cópias simétricas com uma única composição.
    pub fn canvas_brush_batch_with_symmetry(
        state: &mut AppState,
        points: &[(u32, u32)],
        settings: BrushSettings,
    ) -> DirtyTiles {
        if points.is_empty() {
            return DirtyTiles::default();
        }

        let sym_x = state.session.tools.paint_symmetry_x;
        let sym_y = state.session.tools.paint_symmetry_y;
        let (width, height) = match state
            .project
            .assets
            .get(state.project.active)
            .and_then(|a| a.texture.as_ref())
        {
            Some(t) => (t.w, t.h),
            None => (DEFAULT_TEXTURE_SIDE, DEFAULT_TEXTURE_SIDE),
        };

        let mut dabs = Vec::with_capacity(points.len() * 4);
        for &(x, y) in points {
            let mut sample = vec![(x, y)];
            let sx = (width - 1).saturating_sub(x);
            let sy = (height - 1).saturating_sub(y);
            if sym_x {
                sample.push((sx, y));
            }
            if sym_y {
                sample.push((x, sy));
            }
            if sym_x && sym_y {
                sample.push((sx, sy));
            }
            sample.sort_unstable();
            sample.dedup();
            dabs.extend(sample);
        }
        Self::canvas_brush_batch(state, &dabs, settings)
    }

    /// Confirma forma (Line/Rectangle) entre dois pontos do canvas.
    /// Roteia o estilo pelo pincel ativo (Pixel=sólido, Soft=suave,
    /// Eraser=apaga). Quem chama faz 1 checkpoint antes.
    pub fn commit_shape(state: &mut AppState, stroke: ShapeStroke) {
        let first_face = Self::face_at_texel_of_active(state, stroke.x0, stroke.y0);
        Self::restricted_edit(state, first_face, |cv| match stroke.brush {
            BrushType::Rectangle => Self::stroke_rect(cv, &stroke),
            BrushType::Ellipse => Self::stroke_ellipse(cv, &stroke),
            _ => Self::stroke_line(cv, &stroke),
        });
        Self::composite_active(state);
    }

    /// Wrapper compatível com API legado.
    pub fn canvas_brush(state: &mut AppState, x: u32, y: u32, erase: bool) {
        let brush = if erase {
            BrushType::Eraser
        } else {
            BrushType::Pixel
        };
        Self::canvas_brush_advanced(state, x, y, brush, state.canvas_brush, 1.0);
    }

    /// Preenche a camada ativa com a cor selecionada.
    pub fn canvas_fill(state: &mut AppState) {
        let color = Self::paint_color_rgba(state);
        Self::restricted_edit(state, None, |cv| cv.fill(color));
        Self::composite_active(state);
    }

    /// Composição "over" (alfa reto) de `src` sobre `dst`.
    fn over(dst: [u8; 4], src: [f32; 4]) -> [u8; 4] {
        let sa = (src[3] / 255.0).clamp(0.0, 1.0);
        let da = dst[3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            return [0, 0, 0, 0];
        }
        let mut out = [0u8; 4];
        for c in 0..3 {
            let v = (src[c] * sa + dst[c] as f32 * da * (1.0 - sa)) / out_a;
            out[c] = v.round().clamp(0.0, 255.0) as u8;
        }
        out[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
        out
    }

    fn lerp_color(a: [u8; 4], b: [u8; 4], t: f32) -> [f32; 4] {
        let mut out = [0.0; 4];
        for c in 0..4 {
            out[c] = a[c] as f32 * (1.0 - t) + b[c] as f32 * t;
        }
        out
    }

    /// Aplica um gradiente linear na camada ativa de (x0, y0) até (x1, y1) entre duas cores RGBA,
    /// **por cima** do que já existe (a transparência da cor final preserva a pintura abaixo).
    pub fn canvas_gradient_linear(
        state: &mut AppState,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        color_start: [u8; 4],
        color_end: [u8; 4],
    ) {
        let first_face = Self::face_at_texel_of_active(state, x0, y0);
        let dither = state.session.tools.brush_style.dithering;
        Self::restricted_edit(state, first_face, |cv| {
            let dx = x1 as f32 - x0 as f32;
            let dy = y1 as f32 - y0 as f32;
            let len_sq = dx * dx + dy * dy;
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let t = if len_sq < 1.0 {
                        0.0
                    } else {
                        let proj = (x as f32 - x0 as f32) * dx + (y as f32 - y0 as f32) * dy;
                        (proj / len_sq).clamp(0.0, 1.0)
                    };
                    let src = if dither {
                        const BAYER: [[u8; 4]; 4] =
                            [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
                        if t >= (BAYER[(y % 4) as usize][(x % 4) as usize] as f32 + 0.5) / 16.0 {
                            color_end.map(f32::from)
                        } else {
                            color_start.map(f32::from)
                        }
                    } else {
                        Self::lerp_color(color_start, color_end, t)
                    };
                    if let Some(dst) = cv.get(x, y) {
                        cv.set(x, y, Self::over(dst, src));
                    }
                }
            }
        });
        Self::composite_active(state);
    }

    /// Gradiente radial: `color_start` no centro `(cx, cy)` até `color_end` a `radius` texels.
    pub fn canvas_gradient_radial(
        state: &mut AppState,
        cx: u32,
        cy: u32,
        radius: f32,
        color_start: [u8; 4],
        color_end: [u8; 4],
    ) {
        let first_face = Self::face_at_texel_of_active(state, cx, cy);
        let radius = radius.max(1.0);
        let dither = state.session.tools.brush_style.dithering;
        Self::restricted_edit(state, first_face, |cv| {
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let d =
                        ((x as f32 - cx as f32).powi(2) + (y as f32 - cy as f32).powi(2)).sqrt();
                    let t = (d / radius).clamp(0.0, 1.0);
                    let src = if dither {
                        const BAYER: [[u8; 4]; 4] =
                            [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
                        if t >= (BAYER[(y % 4) as usize][(x % 4) as usize] as f32 + 0.5) / 16.0 {
                            color_end.map(f32::from)
                        } else {
                            color_start.map(f32::from)
                        }
                    } else {
                        Self::lerp_color(color_start, color_end, t)
                    };
                    if let Some(dst) = cv.get(x, y) {
                        cv.set(x, y, Self::over(dst, src));
                    }
                }
            }
        });
        Self::composite_active(state);
    }

    /// Preenche uma região delimitada por um polígono UV (scanline par-ímpar).
    ///
    /// As UVs vêm em 0..1 com origem embaixo; o canvas é topo-esquerda.
    pub fn fill_uv_polygon(canvas: &mut Canvas, uv: &[[f32; 2]], color: [u8; 4]) {
        if uv.len() < 3 {
            return;
        }
        let points: Vec<(f32, f32)> = uv
            .iter()
            .filter(|p| p[0].is_finite() && p[1].is_finite())
            .map(|p| (p[0] * canvas.w as f32, (1.0 - p[1]) * canvas.h as f32))
            .collect();
        if points.len() < 3 {
            return;
        }
        let min_y = points
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let max_y = points
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(canvas.h as f32) as u32;

        for y in min_y..max_y {
            let scan_y = y as f32 + 0.5;
            let mut crossings: Vec<f32> = Vec::new();
            for index in 0..points.len() {
                let (x0, y0) = points[index];
                let (x1, y1) = points[(index + 1) % points.len()];
                if (y0 <= scan_y && y1 > scan_y) || (y1 <= scan_y && y0 > scan_y) {
                    let t = (scan_y - y0) / (y1 - y0);
                    crossings.push(x0 + t * (x1 - x0));
                }
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in crossings.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let start = pair[0].floor().max(0.0) as u32;
                let end = pair[1].ceil().min(canvas.w as f32) as u32;
                for x in start..end {
                    canvas.set(x, y, color);
                }
            }
        }
    }

    /// Expande (dilata) a cor pintada em direção aos pixels vazios adjacentes (alpha == 0)
    /// para evitar frestas e artefatos de costura no mapeamento UV (UV bleed).
    pub fn dilate_canvas(canvas: &mut Canvas, color: [u8; 4], bleed: usize) {
        let w = canvas.w as i32;
        let h = canvas.h as i32;
        for _ in 0..bleed {
            let mut to_fill = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if let Some(px) = canvas.get(x as u32, y as u32)
                        && px[3] == 0
                    {
                        let neighbors = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
                        let has_colored_neighbor = neighbors.iter().any(|&(nx, ny)| {
                            if nx >= 0 && nx < w && ny >= 0 && ny < h {
                                if let Some(np) = canvas.get(nx as u32, ny as u32) {
                                    np[3] > 0
                                        && np[0] == color[0]
                                        && np[1] == color[1]
                                        && np[2] == color[2]
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        });
                        if has_colored_neighbor {
                            to_fill.push((x as u32, y as u32));
                        }
                    }
                }
            }
            if to_fill.is_empty() {
                break;
            }
            for (x, y) in to_fill {
                canvas.set(x, y, color);
            }
        }
    }

    /// Preenche o canvas respeitando o `FillScope` (uma semente, algoritmos distintos).
    ///
    /// `face_hint` é a face sob o cursor, quando existe: sem ela os escopos por
    /// face não têm semente e caem em `Object`.
    pub fn canvas_fill_scoped(
        state: &mut AppState,
        face_hint: Option<usize>,
        seed: Option<(u32, u32)>,
        scope: petunia_core::FillScope,
    ) {
        Self::ensure_stack(state);
        let color = Self::paint_color_rgba(state);
        // No canvas 2D não há face sob o cursor: a semente (texel) a revela.
        let face_hint = face_hint
            .or_else(|| seed.and_then(|(x, y)| Self::face_at_texel_of_active(state, x, y)));
        let scope = if face_hint.is_none()
            && matches!(
                scope,
                petunia_core::FillScope::Face | petunia_core::FillScope::UvIsland
            ) {
            petunia_core::FillScope::Object
        } else {
            scope
        };

        // Coleta os polígonos UV elegíveis antes de emprestar o canvas.
        let polygons: Vec<Vec<[f32; 2]>> = match scope {
            petunia_core::FillScope::ConnectedPixels => Vec::new(),
            petunia_core::FillScope::Object => Vec::new(),
            petunia_core::FillScope::Face => face_hint
                .and_then(|face| {
                    state
                        .project
                        .active_mesh()
                        .and_then(|mesh| mesh.faces.get(face))
                        .map(|face| vec![face.uv.clone()])
                })
                .unwrap_or_default(),
            petunia_core::FillScope::SelectedFaces => state
                .project
                .active_mesh()
                .map(|mesh| {
                    mesh.faces
                        .iter()
                        .filter(|face| face.selected)
                        .map(|face| face.uv.clone())
                        .collect()
                })
                .unwrap_or_default(),
            petunia_core::FillScope::UvIsland => face_hint
                .and_then(|face| {
                    let mesh = state.project.active_mesh()?;
                    let islands = mesh.uv_islands();
                    let island = islands.iter().find(|island| island.faces.contains(&face))?;
                    Some(
                        island
                            .faces
                            .iter()
                            .filter_map(|&fi| mesh.faces.get(fi).map(|face| face.uv.clone()))
                            .collect(),
                    )
                })
                .unwrap_or_default(),
        };

        Self::restricted_edit(state, face_hint, |cv| match scope {
            petunia_core::FillScope::ConnectedPixels => {
                if let Some((x, y)) = seed {
                    Self::flood_fill(cv, x, y, color, 16);
                } else {
                    cv.fill(color);
                }
            }
            petunia_core::FillScope::Object => cv.fill(color),
            _ => {
                if polygons.is_empty() {
                    cv.fill(color);
                }
                for polygon in &polygons {
                    Self::fill_uv_polygon(cv, polygon, color);
                }
                if matches!(scope, petunia_core::FillScope::UvIsland) {
                    Self::dilate_canvas(cv, color, 2);
                }
            }
        });

        Self::composite_active(state);
    }

    /// Primeira face (em ordem de índice) cuja UV cobre o texel da camada ativa.
    fn face_at_texel_of_active(state: &AppState, x: u32, y: u32) -> Option<usize> {
        let (w, h) = Self::active_canvas_dims(state)?;
        state
            .project
            .active_mesh()?
            .faces_at_texel(x, y, w, h)
            .first()
            .copied()
    }

    /// Edita a camada ativa respeitando a restrição de faces (P3D-132).
    ///
    /// Operações de região inteira (balde, gradiente, forma) rodam sobre a
    /// camada e depois devolvem os texels fora da máscara ao estado anterior.
    /// Retorna `false` se a camada está travada ou a restrição não deixa
    /// nenhuma face elegível.
    pub(crate) fn restricted_edit(
        state: &mut AppState,
        first_face: Option<usize>,
        edit: impl FnOnce(&mut Canvas),
    ) -> bool {
        Self::ensure_stack(state);
        let in_stroke = state.session.tools.paint_stroke.is_some();
        if !in_stroke {
            state.session.tools.paint_restriction = None;
            state.session.tools.paint_lock_face = None;
        }
        let restriction = Self::resolve_restriction(state, first_face);
        let blocked = Self::restriction_blocks_everything(restriction.as_ref());
        let alpha_lock = state.session.tools.brush_style.alpha_lock;
        let mut done = false;
        if !blocked {
            let active = state.project.active;
            if let Some(asset) = state.project.assets.get_mut(active)
                && !asset.locked
                && let Some(stack) = asset.paint_stack.as_mut()
                && let Some(layer) = stack.active_mut()
                && !layer.locked
                && layer.is_paintable()
                && let Some(cv) = layer.canvas_mut()
            {
                let alpha_before = alpha_lock.then(|| cv.pixels.clone());
                let snapshot = restriction.as_ref().map(|_| cv.pixels.clone());
                edit(cv);
                if let (Some(r), Some(snapshot)) = (restriction.as_ref(), snapshot) {
                    let mask = r.mask(&asset.mesh, cv.w, cv.h);
                    for (i, allowed) in mask.texels.iter().enumerate() {
                        if *allowed == 0 {
                            cv.pixels[i * 4..i * 4 + 4]
                                .copy_from_slice(&snapshot[i * 4..i * 4 + 4]);
                        }
                    }
                }
                if let Some(before) = alpha_before {
                    for (pixel, old) in cv
                        .pixels
                        .as_chunks_mut::<4>()
                        .0
                        .iter_mut()
                        .zip(before.as_chunks::<4>().0)
                    {
                        if old[3] == 0 {
                            pixel.copy_from_slice(old);
                        } else {
                            pixel[3] = old[3];
                        }
                    }
                }
                done = true;
            }
        }
        if !in_stroke {
            state.session.tools.paint_restriction = None;
            state.session.tools.paint_lock_face = None;
        }
        done
    }

    /// Limpa a camada ativa (alfa zero).
    pub fn canvas_clear(state: &mut AppState) {
        Self::ensure_stack(state);
        let active_idx = state.project.active;
        if let Some(o) = state.project.assets.get_mut(active_idx)
            && let Some(stack) = o.paint_stack.as_mut()
            && let Some(layer) = stack.active_mut()
            && let Some(cv) = layer.canvas_mut()
        {
            cv.fill([0, 0, 0, 0]);
        }
        Self::composite_active(state);
    }

    // ---- Pintura 3D Direta sobre Malha via UV (P3D-062, P3D-132) ----

    /// UV do ponto de impacto numa face (raycast → baricêntricas → UV).
    /// `None` = fora da face ou máscara de isolamento vetou.
    pub fn face_hit_uv(
        state: &AppState,
        face_idx: usize,
        hit_pos: Vec3,
        isolate_selection: bool,
    ) -> Option<[f32; 2]> {
        let active_asset = state.project.assets.get(state.project.active)?;
        let mesh = &active_asset.mesh;
        let face = mesh.faces.get(face_idx)?;

        // P3D-132: Paint Masks / Face & Selection Isolation
        if isolate_selection {
            let has_selected_faces = mesh.faces.iter().any(|f| f.selected);
            if has_selected_faces && !face.selected {
                return None;
            }
        }

        let m = face.verts.len();
        if m < 3 || face.uv.len() < m {
            return None;
        }

        // Usa exatamente os triângulos do renderer e do picking. Um fan cobre
        // espaço vazio em faces côncavas e projeta tinta na UV errada.
        for [a, b, c] in mesh.face_triangle_corners(face_idx) {
            let idx0 = face.verts[a] as usize;
            let idx1 = face.verts[b] as usize;
            let idx2 = face.verts[c] as usize;

            if idx0 < mesh.verts.len() && idx1 < mesh.verts.len() && idx2 < mesh.verts.len() {
                let v0 = mesh.verts[idx0].vec();
                let v1 = mesh.verts[idx1].vec();
                let v2 = mesh.verts[idx2].vec();

                let uv0 = face.uv[a];
                let uv1 = face.uv[b];
                let uv2 = face.uv[c];

                if let Some(interpolated) = barycentric_uv(hit_pos, v0, v1, v2, uv0, uv1, uv2) {
                    return Some(interpolated);
                }
            }
        }
        None
    }

    /// Converte UV [0,1] em pixel do canvas (com wrap + flip V).
    pub fn uv_to_px(state: &AppState, uv: [f32; 2]) -> Option<(u32, u32)> {
        let (w, h) = state
            .project
            .assets
            .get(state.project.active)
            .and_then(|o| {
                o.paint_stack
                    .as_ref()
                    .and_then(|s| s.active())
                    .and_then(|l| l.canvas())
                    .or(o.texture.as_ref())
            })
            .map(|cv| (cv.w, cv.h))?;
        if w == 0 || h == 0 {
            return None;
        }
        let u = uv[0].rem_euclid(1.0);
        let v = uv[1].rem_euclid(1.0);
        Some((
            ((u * w as f32) as u32).min(w.saturating_sub(1)),
            (((1.0 - v) * h as f32) as u32).min(h.saturating_sub(1)),
        ))
    }

    /// Encontra as coordenadas UV na malha para uma posição tridimensional arbitrária
    /// varrendo as faces (e triângulos gerados para render/picking).
    pub fn find_mesh_uv_at_pos(
        state: &AppState,
        pos: Vec3,
        isolate_selection: bool,
    ) -> Option<[f32; 2]> {
        let active_asset = state.project.assets.get(state.project.active)?;
        let mesh = &active_asset.mesh;

        // 1ª passada: teste exato/tolerante padrão em cada face
        for (fi, _) in mesh.faces.iter().enumerate() {
            if let Some(uv) = Self::face_hit_uv(state, fi, pos, isolate_selection) {
                return Some(uv);
            }
        }

        // 2ª passada: tolerância ampliada (para pequenas variações de ponto flutuante em malhas curvas)
        for (fi, face) in mesh.faces.iter().enumerate() {
            if isolate_selection {
                let has_selected_faces = mesh.faces.iter().any(|f| f.selected);
                if has_selected_faces && !face.selected {
                    continue;
                }
            }
            let m = face.verts.len();
            if m < 3 || face.uv.len() < m {
                continue;
            }
            for [a, b, c] in mesh.face_triangle_corners(fi) {
                let idx0 = face.verts[a] as usize;
                let idx1 = face.verts[b] as usize;
                let idx2 = face.verts[c] as usize;
                if idx0 < mesh.verts.len() && idx1 < mesh.verts.len() && idx2 < mesh.verts.len() {
                    let v0 = mesh.verts[idx0].vec();
                    let v1 = mesh.verts[idx1].vec();
                    let v2 = mesh.verts[idx2].vec();
                    let uv0 = face.uv[a];
                    let uv1 = face.uv[b];
                    let uv2 = face.uv[c];
                    if let Some(uv) = barycentric_uv_tolerant(pos, v0, v1, v2, uv0, uv1, uv2, -0.05)
                    {
                        return Some(uv);
                    }
                }
            }
        }
        None
    }

    /// Pinta na textura 2D do modelo projetando o ponto de impacto 3D nas
    /// coordenadas UV da face (API legado → descriptor canônico).
    pub fn paint_mesh_3d(
        state: &mut AppState,
        face_idx: usize,
        hit_pos: Vec3,
        brush: BrushType,
        radius: u32,
        strength: f32,
        isolate_selection: bool,
    ) -> bool {
        let settings = BrushSettings {
            kind: brush,
            size_px: (radius.max(1) * 2) as f32,
            hardness: state.session.tools.brush_hardness,
            strength,
            flow: state.session.tools.brush_flow,
            spacing: state.session.tools.brush_spacing,
        };
        Self::paint_mesh_3d_with_settings(state, face_idx, hit_pos, settings, isolate_selection)
    }

    /// Pinta na textura via UV com o descriptor canônico (iniciativa Paint).
    pub fn paint_mesh_3d_with_settings(
        state: &mut AppState,
        face_idx: usize,
        hit_pos: Vec3,
        settings: BrushSettings,
        isolate_selection: bool,
    ) -> bool {
        Self::paint_mesh_3d_batch_with_settings(
            state,
            &[(face_idx, hit_pos)],
            settings,
            isolate_selection,
        )
    }

    /// Pinta vários hits 3D com pincel esférico, respeitando simetria, seleção
    /// e trava de pincel, e compõe o canvas uma única vez.
    ///
    /// `isolate_selection` força a restrição pela seleção mesmo que o isolamento
    /// não esteja ligado na sessão. Fora de um traço (`begin_paint_stroke`) a
    /// restrição e o buffer valem só para esta chamada.
    pub fn paint_mesh_3d_batch_with_settings(
        state: &mut AppState,
        hits: &[(usize, Vec3)],
        settings: BrushSettings,
        isolate_selection: bool,
    ) -> bool {
        let s = settings.sanitized();
        if hits.is_empty() || s.kind.is_shape() {
            return false;
        }
        Self::ensure_canvas(state);
        Self::ensure_stack(state);

        let in_stroke = state.session.tools.paint_stroke.is_some();
        let forced = isolate_selection && !state.session.tools.paint_isolate_selection;
        if !in_stroke {
            state.session.tools.paint_restriction = None;
            state.session.tools.paint_buffer = None;
        }
        if forced {
            state.session.tools.paint_isolate_selection = true;
        }
        let dirty = Self::stamp_dabs_3d(state, hits, s);
        if forced {
            state.session.tools.paint_isolate_selection = false;
        }
        if !in_stroke {
            state.session.tools.paint_restriction = None;
            state.session.tools.paint_buffer = None;
        }
        let mut dirty = dirty;
        dirty.normalize();
        if dirty.is_empty() {
            return false;
        }
        Self::composite_active_tiles(state, dirty.as_slice());
        true
    }

    fn lock_allows_face(state: &AppState, face_idx: usize) -> bool {
        use petunia_core::BrushLock;
        match state.session.tools.brush_lock {
            BrushLock::None | BrushLock::FirstObject => true,
            BrushLock::FirstFace => state
                .session
                .selection
                .faces
                .first()
                .copied()
                .is_none_or(|f| f == face_idx),
            BrushLock::SelectedFaces => {
                let Some(mesh) = state.project.active_mesh() else {
                    return false;
                };
                mesh.faces.get(face_idx).is_some_and(|f| f.selected)
                    || state.session.selection.faces.contains(&face_idx)
            }
        }
    }

    /// Screen-space brush: stamp every visible face whose projected UV falls
    /// inside the dab. Reuses the same canvas brush foundation.
    pub fn paint_screen_space(
        state: &mut AppState,
        hit_pos: Vec3,
        settings: BrushSettings,
    ) -> usize {
        if !Self::lock_allows_face(state, 0)
            && matches!(
                state.session.tools.brush_lock,
                petunia_core::BrushLock::SelectedFaces | petunia_core::BrushLock::FirstFace
            )
        {
            // still iterate faces below with per-face lock
        }
        let isolate = state.session.tools.paint_isolate_selection
            || matches!(
                state.session.tools.brush_lock,
                petunia_core::BrushLock::SelectedFaces
            );
        let face_count = state
            .project
            .active_mesh()
            .map(|m| m.faces.len())
            .unwrap_or(0);
        let mut hits = Vec::new();
        for fi in 0..face_count {
            if !Self::lock_allows_face(state, fi) {
                continue;
            }
            if Self::face_hit_uv(state, fi, hit_pos, isolate).is_some() {
                hits.push((fi, hit_pos));
            }
        }
        let count = hits.len();
        if count > 0 {
            Self::paint_mesh_3d_batch_with_settings(state, &hits, settings, isolate);
        }
        count
    }

    pub fn fill_scope(state: &mut AppState, scope: petunia_core::FillScope) {
        use petunia_core::FillScope;
        Self::ensure_stack(state);
        match scope {
            FillScope::ConnectedPixels | FillScope::Object => Self::canvas_fill(state),
            FillScope::Face | FillScope::SelectedFaces | FillScope::UvIsland => {
                let isolate = !matches!(scope, FillScope::Object);
                let color = [
                    (state.paint_color[0] * 255.0) as u8,
                    (state.paint_color[1] * 255.0) as u8,
                    (state.paint_color[2] * 255.0) as u8,
                    255,
                ];
                let uvs: Vec<[f32; 2]> = state
                    .project
                    .active_mesh()
                    .map(|m| {
                        m.faces
                            .iter()
                            .enumerate()
                            .filter(|(i, f)| {
                                !isolate || f.selected || state.session.selection.faces.contains(i)
                            })
                            .flat_map(|(_, f)| f.uv.iter().copied())
                            .collect()
                    })
                    .unwrap_or_default();
                let active = state.project.active;
                if let Some(o) = state.project.assets.get_mut(active)
                    && let Some(stack) = o.paint_stack.as_mut()
                    && let Some(layer) = stack.active_mut()
                    && !layer.locked
                    && let Some(cv) = layer.canvas_mut()
                {
                    let (w, h) = (cv.w, cv.h);
                    for uv in uvs {
                        let u = uv[0].rem_euclid(1.0);
                        let v = uv[1].rem_euclid(1.0);
                        let x = ((u * w as f32) as u32).min(w.saturating_sub(1));
                        let y = (((1.0 - v) * h as f32) as u32).min(h.saturating_sub(1));
                        cv.set(x, y, color);
                    }
                }
                Self::composite_active(state);
            }
        }
    }

    pub fn add_decal(
        state: &mut AppState,
        image: Canvas,
        center_uv: [f32; 2],
        scale_uv: [f32; 2],
        rotation_rad: f32,
    ) -> Option<uuid::Uuid> {
        Self::ensure_stack(state);
        state.checkpoint("add decal");
        let active = state.project.active;
        let id = state.project.assets.get_mut(active).and_then(|o| {
            let stack = o.paint_stack.as_mut()?;
            let decal = DecalLayer::new(image, center_uv, scale_uv, rotation_rad);
            Some(stack.add_layer(PaintLayer::new_decal("Decal", decal)))
        });
        Self::composite_active(state);
        id
    }

    pub fn add_layer_group(state: &mut AppState, name: &str) -> Option<uuid::Uuid> {
        Self::ensure_stack(state);
        state.checkpoint("add layer group");
        let active = state.project.active;
        state
            .project
            .assets
            .get_mut(active)
            .and_then(|o| o.paint_stack.as_mut().map(|s| s.add_group(name)))
    }
}

/// Calcula interpolação baricêntrica de coordenadas UV para um ponto dentro de um triângulo 3D.
pub fn barycentric_uv(
    p: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    uv_a: [f32; 2],
    uv_b: [f32; 2],
    uv_c: [f32; 2],
) -> Option<[f32; 2]> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-8 {
        return None;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    // Tolerância para pontos na borda ou levemente fora do triângulo
    let eps = -1.0e-4;
    if u >= eps && v >= eps && w >= eps {
        let interpolated_u = u * uv_a[0] + v * uv_b[0] + w * uv_c[0];
        let interpolated_v = u * uv_a[1] + v * uv_b[1] + w * uv_c[1];
        Some([interpolated_u, interpolated_v])
    } else {
        None
    }
}

/// Calcula interpolação baricêntrica de coordenadas UV com tolerância customizada.
#[allow(clippy::too_many_arguments)]
pub fn barycentric_uv_tolerant(
    p: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    uv_a: [f32; 2],
    uv_b: [f32; 2],
    uv_c: [f32; 2],
    eps: f32,
) -> Option<[f32; 2]> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-8 {
        return None;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    if u >= eps && v >= eps && w >= eps {
        let interpolated_u =
            u.clamp(0.0, 1.0) * uv_a[0] + v.clamp(0.0, 1.0) * uv_b[0] + w.clamp(0.0, 1.0) * uv_c[0];
        let interpolated_v =
            u.clamp(0.0, 1.0) * uv_a[1] + v.clamp(0.0, 1.0) * uv_b[1] + w.clamp(0.0, 1.0) * uv_c[1];
        Some([interpolated_u, interpolated_v])
    } else {
        None
    }
}

fn colors_match(a: [u8; 4], b: [u8; 4], tol: u8) -> bool {
    let t = tol as i16;
    (a[0] as i16 - b[0] as i16).abs() <= t
        && (a[1] as i16 - b[1] as i16).abs() <= t
        && (a[2] as i16 - b[2] as i16).abs() <= t
        && (a[3] as i16 - b[3] as i16).abs() <= t
}

impl Module for PaintModule {
    fn id(&self) -> &'static str {
        "paint"
    }

    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn std::any::Any + 'static) {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_lock_keeps_transparency_for_brush_eraser_and_gradient() {
        let mut state = AppState::default();
        let mut canvas = Canvas::new(8, 8, [0; 4]);
        canvas.set(3, 3, [10, 20, 30, 128]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_raster("Alpha", canvas.clone()));
        let asset = state.project.active_mut().unwrap();
        asset.texture = Some(canvas);
        asset.paint_stack = Some(stack);
        state.session.tools.brush_style.alpha_lock = true;
        state.paint_color = [1.0, 0.0, 0.0];
        for kind in [BrushType::Pixel, BrushType::Eraser] {
            let settings = BrushSettings {
                kind,
                size_px: 1.0,
                hardness: 1.0,
                strength: 1.0,
                ..Default::default()
            };
            PaintModule::canvas_brush_with_settings(&mut state, 3, 3, settings);
            PaintModule::canvas_brush_with_settings(&mut state, 0, 0, settings);
            let layer = state
                .project
                .active()
                .unwrap()
                .paint_stack
                .as_ref()
                .unwrap()
                .active()
                .unwrap()
                .canvas()
                .unwrap();
            assert_eq!(layer.get(3, 3).unwrap()[3], 128);
            assert_eq!(layer.get(0, 0).unwrap(), [0; 4]);
        }
        PaintModule::canvas_gradient_linear(
            &mut state,
            0,
            0,
            7,
            0,
            [255, 0, 0, 255],
            [0, 0, 255, 255],
        );
        let layer = state
            .project
            .active()
            .unwrap()
            .paint_stack
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .canvas()
            .unwrap();
        assert_eq!(layer.get(3, 3).unwrap()[3], 128);
        assert_eq!(layer.get(0, 0).unwrap(), [0; 4]);
    }

    #[test]
    fn dithered_gradient_uses_only_the_two_endpoint_colors() {
        let mut state = AppState::default();
        let canvas = Canvas::new(16, 8, [0; 4]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_raster("Gradient", canvas.clone()));
        let asset = state.project.active_mut().unwrap();
        asset.texture = Some(canvas);
        asset.paint_stack = Some(stack);
        state.session.tools.brush_style.dithering = true;
        PaintModule::canvas_gradient_linear(
            &mut state,
            0,
            0,
            15,
            0,
            [0, 0, 0, 255],
            [255, 255, 255, 255],
        );
        let layer = state
            .project
            .active()
            .unwrap()
            .paint_stack
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .canvas()
            .unwrap();
        assert!(
            layer
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [0, 0, 0, 255] || *p == [255; 4])
        );
        let (pixels, _) = layer.pixels.as_chunks::<4>();
        assert!(pixels.contains(&[0, 0, 0, 255]));
        assert!(pixels.contains(&[255; 4]));
    }

    #[test]
    fn vertex_fill_advances_only_color_revision_and_is_undoable() {
        let mut state = AppState::new("en");
        state.paint_color = [0.1, 0.7, 0.3];
        let before = state.project.project.revision_clock();
        let original = state.project.active_mesh().unwrap().verts[0].color;

        assert!(PaintModule::fill_selection(&mut state) > 0);
        let after = state.project.project.revision_clock();
        assert_eq!(state.project.undo.depth(), (1, 0));
        for index in [0, 1, 2, 3, 4, 6, 7, 8, 9] {
            assert_eq!(after[index], before[index], "revision index {index}");
        }
        assert_eq!(after[5], before[5] + 1);
        assert!(state.undo());
        assert_eq!(
            state.project.active_mesh().unwrap().verts[0].color,
            original
        );
    }

    #[test]
    fn vertex_eyedropper_updates_both_paint_paths() {
        let mut state = AppState::new("en");
        state.project.active_mesh_mut().unwrap().verts[0].color = [0.2, 0.4, 0.8];
        PaintModule::eyedrop_vertex(&mut state, 0);
        assert_eq!(state.paint_color, [0.2, 0.4, 0.8]);
        assert_eq!(state.session.tools.paint_color, state.paint_color);
    }

    #[test]
    fn paint_uv_hit_uses_render_triangles_for_a_concave_face() {
        use petunia_mesh::{Face, Vertex};

        let mut state = AppState::new("en");
        let mesh = state.project.active_mesh_mut().expect("active mesh");
        mesh.verts = [[0.0, 0.0], [3.0, 0.0], [3.0, 3.0], [1.5, 1.0], [0.0, 3.0]]
            .map(|[x, y]| Vertex::new(x, y, 0.0))
            .to_vec();
        mesh.faces = vec![Face::with_uv(
            vec![0, 1, 2, 3, 4],
            vec![
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.5, 1.0 / 3.0],
                [0.0, 1.0],
            ],
        )];

        assert!(PaintModule::face_hit_uv(&state, 0, Vec3::new(1.0, 0.5, 0.0), false).is_some());
        assert!(
            PaintModule::face_hit_uv(&state, 0, Vec3::new(1.5, 1.4, 0.0), false).is_none(),
            "the concave notch must not receive paint"
        );
    }

    #[test]
    fn test_push_and_set_palette_sync() {
        let mut state = AppState::new("en");

        PaintModule::push_palette(&mut state, [0.5, 0.5, 0.5]);
        assert_eq!(state.project.palette[0], [0.5, 0.5, 0.5]);

        let p8 = petunia_project::preset_pico8();
        PaintModule::set_palette(&mut state, p8.clone());
        assert_eq!(state.project.palette.len(), 16);
        assert_eq!(state.project.palette, p8);
    }

    /// Face do ativo mais voltada para `dir` e o centro dela.
    fn face_toward(state: &AppState, dir: Vec3) -> (usize, Vec3) {
        let mesh = state.project.active_mesh().expect("malha ativa");
        let fi = (0..mesh.faces.len())
            .max_by(|&a, &b| {
                mesh.face_normal(a)
                    .dot(dir)
                    .partial_cmp(&mesh.face_normal(b).dot(dir))
                    .unwrap()
            })
            .expect("face");
        let verts = &mesh.faces[fi].verts;
        let center = verts
            .iter()
            .map(|&v| mesh.verts[v as usize].vec())
            .sum::<Vec3>()
            / verts.len() as f32;
        (fi, center)
    }

    #[test]
    fn test_pixel_brush_and_canvas_stamp() {
        let mut cv = Canvas::new(8, 8, [0, 0, 0, 255]);
        PaintModule::stamp_pixel_brush(&mut cv, 4, 4, 1, [255, 255, 255, 255]);

        assert_eq!(cv.get(4, 4), Some([255, 255, 255, 255]));
        assert_eq!(cv.get(4, 3), Some([255, 255, 255, 255]));
        assert_eq!(cv.get(4, 5), Some([255, 255, 255, 255]));
        assert_eq!(cv.get(0, 0), Some([0, 0, 0, 255]));
    }

    #[test]
    fn test_hardness_shapes_falloff() {
        let mut hard = Canvas::new(16, 16, [0, 0, 0, 255]);
        PaintModule::stamp_soft_brush_hard(&mut hard, 8, 8, 4, [255, 0, 0, 255], 1.0, 1.0);
        let mut soft = Canvas::new(16, 16, [0, 0, 0, 255]);
        PaintModule::stamp_soft_brush_hard(&mut soft, 8, 8, 4, [255, 0, 0, 255], 1.0, 0.0);

        // Dureza máxima: borda do dab totalmente opaca (núcleo sólido).
        let hard_edge = hard.get(8 + 3, 8).unwrap();
        assert_eq!(hard_edge[0], 255);
        // Dureza zero: borda do dab atenua fortemente (falloff quadrático).
        let soft_edge = soft.get(8 + 3, 8).unwrap();
        assert!(
            soft_edge[0] < hard_edge[0],
            "soft {soft_edge:?} vs hard {hard_edge:?}"
        );
    }

    #[test]
    fn test_soft_brush_legacy_delegates_exact_behavior() {
        // O falloff quadrático legado é exatamente hardness = 0.
        let mut legacy = Canvas::new(16, 16, [0, 0, 0, 255]);
        PaintModule::stamp_soft_brush(&mut legacy, 8, 8, 4, [255, 0, 0, 255], 1.0);
        let mut explicit = Canvas::new(16, 16, [0, 0, 0, 255]);
        PaintModule::stamp_soft_brush_hard(&mut explicit, 8, 8, 4, [255, 0, 0, 255], 1.0, 0.0);
        assert_eq!(legacy.pixels, explicit.pixels);
    }

    #[test]
    fn test_airbrush_flow_modulates_strength() {
        let full = Canvas::new(8, 8, [0, 0, 0, 255]);
        let s_full = petunia_core::BrushSettings {
            kind: BrushType::Airbrush,
            size_px: 6.0,
            flow: 1.0,
            ..Default::default()
        };
        let s_weak = petunia_core::BrushSettings {
            kind: BrushType::Airbrush,
            size_px: 6.0,
            flow: 0.1,
            ..Default::default()
        };
        let mut state_a = AppState::new("en");
        let mut state_b = AppState::new("en");
        state_a.paint_color = [1.0, 0.0, 0.0];
        state_b.paint_color = [1.0, 0.0, 0.0];
        PaintModule::canvas_brush_with_settings(&mut state_a, 4, 4, s_full);
        PaintModule::canvas_brush_with_settings(&mut state_b, 4, 4, s_weak);
        let center_a = state_a
            .project
            .active()
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .get(4, 4)
            .unwrap();
        let center_b = state_b
            .project
            .active()
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .get(4, 4)
            .unwrap();
        assert!(
            center_a[0] > center_b[0],
            "fluxo maior deve depositar mais tinta: {center_a:?} vs {center_b:?}"
        );
        let _ = full;
    }

    #[test]
    fn test_paint_mesh_3d_with_settings_uses_radius() {
        let mut state = AppState::new("en");
        state.paint_color = [0.2, 0.8, 0.4];
        let (face, hit) = face_toward(&state, Vec3::Z);
        let settings = petunia_core::BrushSettings {
            kind: BrushType::Pixel,
            size_px: 8.0,
            ..Default::default()
        };
        let painted =
            PaintModule::paint_mesh_3d_with_settings(&mut state, face, hit, settings, false);
        assert!(painted);
        assert!(PaintModule::has_canvas(&state));
        let tex = state.project.active().unwrap().texture.as_ref().unwrap();
        let painted_px = tex.pixels.chunks(4).any(|p| p[1] > 150 && p[0] < 120);
        assert!(painted_px, "textura deve conter a cor pintada (51,204,102)");
    }

    #[test]
    fn painting_one_face_does_not_paint_the_others() {
        let mut state = AppState::new("en");
        state.paint_color = [1.0, 0.0, 0.0];
        let (front, hit) = face_toward(&state, Vec3::Z);
        let (back, _) = face_toward(&state, -Vec3::Z);
        let settings = BrushSettings {
            kind: BrushType::Pixel,
            size_px: 30.0,
            ..Default::default()
        };
        assert!(PaintModule::paint_mesh_3d_with_settings(
            &mut state, front, hit, settings, false
        ));
        let asset = state.project.active().unwrap();
        let tex = asset.texture.as_ref().unwrap();
        let painted = |face: usize| {
            let mask = asset.mesh.uv_coverage_mask([face], tex.w, tex.h, 0.0);
            (0..tex.w * tex.h).any(|i| {
                mask.texels[i as usize] != 0
                    && tex.pixels[i as usize * 4..i as usize * 4 + 3] == [255, 0, 0]
            })
        };
        assert!(painted(front));
        assert!(!painted(back), "a face oposta não pode receber tinta");
        // Nenhuma outra face além das vizinhas ao redor do pincel.
        let others = (0..asset.mesh.faces.len())
            .filter(|&f| f != front && painted(f))
            .count();
        assert!(others <= 4, "{others} faces pintadas");
    }

    #[test]
    fn test_eraser_reduces_alpha() {
        let mut cv = Canvas::new(8, 8, [100, 100, 100, 255]);
        PaintModule::stamp_eraser(&mut cv, 4, 4, 2, 1.0);

        let center = cv.get(4, 4).unwrap();
        assert_eq!(center[3], 0);
    }

    #[test]
    fn test_flood_fill_changes_connected_region() {
        let mut cv = Canvas::new(4, 4, [0, 0, 0, 255]);
        cv.set(1, 0, [255, 0, 0, 255]); // barreira
        cv.set(1, 1, [255, 0, 0, 255]);
        cv.set(1, 2, [255, 0, 0, 255]);
        cv.set(1, 3, [255, 0, 0, 255]);

        PaintModule::flood_fill(&mut cv, 0, 0, [0, 255, 0, 255], 0);

        assert_eq!(cv.get(0, 0), Some([0, 255, 0, 255]));
        assert_eq!(cv.get(0, 3), Some([0, 255, 0, 255]));
        // Do outro lado da barreira deve permanecer intacto
        assert_eq!(cv.get(2, 0), Some([0, 0, 0, 255]));
    }

    #[test]
    fn test_barycentric_uv_interpolation() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 2.0, 0.0);

        let uv_a = [0.0, 0.0];
        let uv_b = [1.0, 0.0];
        let uv_c = [0.0, 1.0];

        // Centro do triângulo
        let p = Vec3::new(0.5, 0.5, 0.0);
        let uv = barycentric_uv(p, a, b, c, uv_a, uv_b, uv_c).expect("ponto dentro do triângulo");
        assert!((uv[0] - 0.25).abs() < 1e-4);
        assert!((uv[1] - 0.25).abs() < 1e-4);
    }

    #[test]
    fn test_paint_mesh_3d_hit_and_material_sync() {
        let mut state = AppState::new("en");
        state.paint_color = [0.2, 0.8, 0.4];

        let (face, hit_pos) = face_toward(&state, Vec3::Z);
        let painted =
            PaintModule::paint_mesh_3d(&mut state, face, hit_pos, BrushType::Pixel, 2, 1.0, false);

        assert!(painted);
        assert!(PaintModule::has_canvas(&state));

        // Sincronização verificada no Asset e no Material ativo
        let asset = state.project.active().unwrap();
        assert!(asset.texture.is_some());
        let mat = state.project.active_material().unwrap();
        assert!(mat.albedo_texture.is_some());
    }

    #[test]
    fn test_stack_migrates_existing_texture_without_loss() {
        let mut state = AppState::new("en");
        PaintModule::ensure_canvas(&mut state);
        // Marca a textura legada com um pixel conhecido.
        if let Some(o) = state.project.active_mut()
            && let Some(cv) = o.texture.as_mut()
        {
            cv.set(10, 10, [11, 22, 33, 255]);
        }
        PaintModule::ensure_stack(&mut state);
        let asset = state.project.active().unwrap();
        let stack = asset.paint_stack.as_ref().expect("stack criado");
        assert_eq!(stack.layers.len(), 1);
        // Conteúdo migrado para a base e recomposto sem perda.
        let base = stack.layers[0].canvas().expect("base raster");
        assert_eq!(base.get(10, 10), Some([11, 22, 33, 255]));
        PaintModule::composite_active(&mut state);
        let tex = state.project.active().unwrap().texture.as_ref().unwrap();
        assert_eq!(tex.get(10, 10), Some([11, 22, 33, 255]));
    }

    #[test]
    fn test_resize_keeps_content_proportionally() {
        let mut state = AppState::new("en");
        PaintModule::ensure_canvas(&mut state);
        state.checkpoint("resize test");
        PaintModule::resize_canvas(&mut state, 64, 64);
        let asset = state.project.active().unwrap();
        let tex = asset.texture.as_ref().unwrap();
        assert_eq!((tex.w, tex.h), (64, 64));
        let stack = asset.paint_stack.as_ref().unwrap();
        assert_eq!(
            (
                stack.layers[0].canvas().unwrap().w,
                stack.layers[0].canvas().unwrap().h
            ),
            (64, 64)
        );
        assert!(state.project.undo.can_undo());
    }

    #[test]
    fn test_layer_add_paint_composite_and_undo() {
        let mut state = AppState::new("en");
        state.paint_color = [1.0, 0.0, 0.0];
        PaintModule::ensure_stack(&mut state);
        state.checkpoint("layer add");
        let active = state.project.active;
        if let Some(o) = state.project.assets.get_mut(active)
            && let Some(stack) = o.paint_stack.as_mut()
        {
            stack.add_layer(PaintLayer::new("Top", 256, 256, [0, 0, 0, 0]));
        }
        // Pinta na camada ativa (topo) e recompõe.
        PaintModule::canvas_brush_advanced(&mut state, 20, 20, BrushType::Pixel, 2, 1.0);
        let tex = state.project.active().unwrap().texture.as_ref().unwrap();
        let px = tex.get(20, 20).unwrap();
        assert!(px[0] > 200, "vermelho do topo deve dominar: {px:?}");
        state.undo();
        let tex = state.project.active().unwrap().texture.as_ref().unwrap();
        assert_eq!(tex.get(20, 20), Some([191, 191, 198, 255]));
    }

    #[test]
    fn test_stroke_line_is_continuous_diagonal() {
        let mut cv = Canvas::new(8, 8, [0, 0, 0, 255]);
        PaintModule::stroke_line(
            &mut cv,
            &ShapeStroke {
                x0: 0,
                y0: 0,
                x1: 7,
                y1: 7,
                brush: BrushType::Pixel,
                color: [255, 255, 255, 255],
                strength: 1.0,
            },
        );
        for k in 0..8 {
            assert_eq!(
                cv.get(k, k),
                Some([255, 255, 255, 255]),
                "falha em ({k},{k})"
            );
        }
        assert_eq!(cv.get(0, 7), Some([0, 0, 0, 255]));
    }

    #[test]
    fn test_stroke_rect_fills_bounds() {
        let mut cv = Canvas::new(8, 8, [0, 0, 0, 255]);
        PaintModule::stroke_rect(
            &mut cv,
            &ShapeStroke {
                x0: 5,
                y0: 5,
                x1: 2,
                y1: 2,
                brush: BrushType::Pixel,
                color: [0, 255, 0, 255],
                strength: 1.0,
            },
        );
        assert_eq!(cv.get(2, 2), Some([0, 255, 0, 255]));
        assert_eq!(cv.get(5, 5), Some([0, 255, 0, 255]));
        assert_eq!(cv.get(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(cv.get(7, 7), Some([0, 0, 0, 255]));
    }

    #[test]
    fn test_face_hit_uv_and_shape_commit_on_cube() {
        let mut state = AppState::new("en");
        let hit = Vec3::new(0.0, 0.0, 1.0);
        let uv = PaintModule::face_hit_uv(&state, 0, hit, false).expect("uv na face 0");
        assert!(uv[0].is_finite() && uv[1].is_finite());
        PaintModule::ensure_stack(&mut state);
        state.checkpoint("shape test");
        let (x0, y0) = PaintModule::uv_to_px(&state, uv).expect("px válido");
        PaintModule::commit_shape(
            &mut state,
            ShapeStroke {
                x0,
                y0,
                x1: x0 + 4,
                y1: y0 + 4,
                brush: BrushType::Line,
                color: [255, 0, 0, 255],
                strength: 1.0,
            },
        );
        assert!(state.project.undo.can_undo());
        state.undo();
        let tex = state.project.active().unwrap().texture.as_ref().unwrap();
        assert_eq!(
            tex.get(x0.min(255), y0.min(255)),
            Some([191, 191, 198, 255])
        );
    }

    #[test]
    fn test_paint_mask_selection_isolation() {
        let mut state = AppState::new("en");
        let (front, front_hit) = face_toward(&state, Vec3::Z);
        let (right, right_hit) = face_toward(&state, Vec3::X);

        // Seleciona apenas a face da direita
        if let Some(m) = state.project.active_mesh_mut() {
            m.faces[right].selected = true;
        }

        // Pintar na face da frente com isolamento ativado -> rejeitado (P3D-132)
        let painted = PaintModule::paint_mesh_3d(
            &mut state,
            front,
            front_hit,
            BrushType::Pixel,
            2,
            1.0,
            true,
        );
        assert!(!painted);

        // Pintar na face selecionada com isolamento ativado -> aceito
        let painted = PaintModule::paint_mesh_3d(
            &mut state,
            right,
            right_hit,
            BrushType::Pixel,
            2,
            1.0,
            true,
        );
        assert!(painted);
    }

    #[test]
    fn mask_with_nothing_selected_paints_nothing() {
        let mut state = AppState::new("en");
        state.session.tools.paint_isolate_selection = true;
        let (face, hit) = face_toward(&state, Vec3::Z);
        let settings = BrushSettings {
            kind: BrushType::Pixel,
            size_px: 10.0,
            ..Default::default()
        };
        assert!(!PaintModule::paint_mesh_3d_with_settings(
            &mut state, face, hit, settings, false
        ));
    }

    #[test]
    fn mask_restricts_the_2d_canvas_to_the_selected_faces() {
        let mut state = AppState::new("en");
        let (right, _) = face_toward(&state, Vec3::X);
        state.project.active_mesh_mut().unwrap().faces[right].selected = true;
        state.session.tools.paint_isolate_selection = true;
        state.paint_color = [0.0, 1.0, 0.0];
        PaintModule::ensure_stack(&mut state);
        let (w, h) = PaintModule::active_canvas_dims(&state).unwrap();
        // Dab enorme no centro do canvas: só o que cai na UV da face escolhida pinta.
        let settings = BrushSettings {
            kind: BrushType::Pixel,
            size_px: 2.0 * w as f32,
            ..Default::default()
        };
        PaintModule::canvas_brush_batch_with_symmetry(&mut state, &[(w / 2, h / 2)], settings);
        let asset = state.project.active().unwrap();
        let tex = asset.texture.as_ref().unwrap();
        let allowed = asset.mesh.uv_coverage_mask([right], w, h, 1.0);
        let mut inside = 0;
        for (i, px) in tex.pixels.chunks(4).enumerate() {
            let green = px[..3] == [0, 255, 0];
            if green {
                assert!(allowed.texels[i] != 0, "texel {i} fora da face selecionada");
                inside += 1;
            }
        }
        assert!(inside > 0);
    }

    #[test]
    fn eraser_follows_hardness_like_the_soft_brush() {
        use crate::engine::dab_falloff;
        for t in [0.0, 0.3, 0.6, 0.9] {
            assert_eq!(
                dab_falloff(BrushType::Eraser, t, 0.4),
                dab_falloff(BrushType::Soft, t, 0.4)
            );
        }
        assert_eq!(dab_falloff(BrushType::Eraser, 0.5, 1.0), 1.0, "dura");
        assert!(dab_falloff(BrushType::Eraser, 0.5, 0.0) < 0.3, "suave");
    }

    #[test]
    fn partial_selection_restricts_painting_without_the_toggle() {
        let mut state = AppState::new("en");
        assert!(!state.session.tools.paint_isolate_selection);
        let (right, right_hit) = face_toward(&state, Vec3::X);
        let (front, front_hit) = face_toward(&state, Vec3::Z);
        state.project.active_mesh_mut().unwrap().faces[right].selected = true;
        let settings = BrushSettings {
            kind: BrushType::Pixel,
            size_px: 10.0,
            ..Default::default()
        };
        assert!(!PaintModule::paint_mesh_3d_with_settings(
            &mut state, front, front_hit, settings, false
        ));
        PaintModule::reset_stroke_scratch(&mut state);
        assert!(PaintModule::paint_mesh_3d_with_settings(
            &mut state, right, right_hit, settings, false
        ));
        PaintModule::reset_stroke_scratch(&mut state);
        // Tudo selecionado não é uma restrição: pinta em qualquer face.
        for face in &mut state.project.active_mesh_mut().unwrap().faces {
            face.selected = true;
        }
        assert!(PaintModule::paint_mesh_3d_with_settings(
            &mut state, front, front_hit, settings, false
        ));
    }

    #[test]
    fn restriction_mask_never_bleeds_into_unselected_neighbors() {
        let mut state = AppState::new("en");
        let (right, _) = face_toward(&state, Vec3::X);
        state.project.active_mesh_mut().unwrap().faces[right].selected = true;
        PaintModule::ensure_stack(&mut state);
        let (w, h) = PaintModule::active_canvas_dims(&state).unwrap();
        let restriction = PaintModule::resolve_restriction(&mut state, None).expect("restrição");
        let mesh = state.project.active_mesh().unwrap();
        let mask = restriction.mask(mesh, w, h);
        let others =
            mesh.uv_coverage_mask((0..mesh.faces.len()).filter(|&i| i != right), w, h, 0.0);
        let mine = mesh.uv_coverage_mask([right], w, h, 0.0);
        for i in 0..mask.texels.len() {
            if mask.texels[i] != 0 && others.texels[i] != 0 {
                assert!(mine.texels[i] != 0, "texel {i} de outra face na máscara");
            }
        }
        assert!(mask.covered() >= mine.covered());
    }

    #[test]
    fn overlapping_flow_does_not_darken_past_the_strength_cap() {
        let mut state = AppState::new("en");
        state.paint_color = [1.0, 1.0, 1.0];
        PaintModule::ensure_stack(&mut state);
        state.begin_paint_stroke();
        let settings = BrushSettings {
            kind: BrushType::Soft,
            size_px: 10.0,
            hardness: 1.0,
            strength: 0.5,
            flow: 1.0,
            spacing: 0.1,
        };
        for _ in 0..6 {
            PaintModule::canvas_brush_batch_with_symmetry(&mut state, &[(20, 20)], settings);
        }
        state.finish_paint_stroke(false);
        let tex = state.project.active().unwrap().texture.as_ref().unwrap();
        let base = state.project.active().unwrap().base_color;
        let expected = (base[0] * 255.0 * 0.5 + 255.0 * 0.5) as i32;
        let got = tex.get(20, 20).unwrap()[0] as i32;
        assert!(
            (got - expected).abs() <= 2,
            "got {got}, expected ~{expected}"
        );
    }

    #[test]
    fn test_paint_layer_stack_composition() {
        let mut base = Canvas::new(8, 8, [0, 0, 0, 255]);
        let mut stack = PaintLayerStack::new();

        // Camada 1: Vermelho com 50% de opacidade
        let mut layer1 = PaintLayer::new("Layer 1", 8, 8, [255, 0, 0, 255]);
        layer1.opacity = 0.5;
        stack.add_layer(layer1);

        stack.composite(&mut base);
        let px = base.get(0, 0).unwrap();
        // 0 * 0.5 + 255 * 0.5 = 128 (aprox)
        assert!((px[0] as i32 - 128).abs() <= 2);
    }

    #[test]
    fn test_decal_layer_projection() {
        let mut base = Canvas::new(16, 16, [0, 0, 0, 255]);
        let mut stack = PaintLayerStack::new();

        // Decalque 4x4 totalmente amarelo
        let decal_img = Canvas::new(4, 4, [255, 255, 0, 255]);
        // Posicionado no centro do UV (0.5, 0.5) ocupando 50% do UV (0.5, 0.5)
        let decal = DecalLayer::new(decal_img, [0.5, 0.5], [0.5, 0.5], 0.0);
        stack.add_layer(PaintLayer::new_decal("Sticker Decal", decal));

        stack.composite(&mut base);

        // O centro (8, 8) deve ser amarelo
        let center = base.get(8, 8).unwrap();
        assert_eq!(center, [255, 255, 0, 255]);

        // Os cantos externos (0, 0) devem permanecer pretos (sem cobertura do decal)
        let corner = base.get(0, 0).unwrap();
        assert_eq!(corner, [0, 0, 0, 255]);
    }

    #[test]
    fn test_paint_effects_pixelate_and_posterize() {
        // Teste do efeito Pixelate
        let mut base = Canvas::new(4, 4, [0, 0, 0, 255]);
        base.set(0, 0, [200, 100, 50, 255]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect(
            "Pixelate 2x2",
            PaintEffect::Pixelate { cell_size: 2 },
        ));
        stack.composite(&mut base);

        // O pixel vizinho no bloco 2x2 deve ter adotado a cor do topo do bloco
        assert_eq!(base.get(1, 1), Some([200, 100, 50, 255]));

        // Teste do efeito Posterize
        let mut base_post = Canvas::new(2, 2, [120, 130, 140, 255]);
        let mut stack_post = PaintLayerStack::new();
        stack_post.add_layer(PaintLayer::new_effect(
            "Posterize 2 levels",
            PaintEffect::Posterize { levels: 2 },
        ));
        stack_post.composite(&mut base_post);
        let px = base_post.get(0, 0).unwrap();
        // Com 2 níveis, valores ~128 são quantizados para 0 ou 255
        assert!(px[0] == 0 || px[0] == 255);

        // Teste do efeito Invert
        let mut base_inv = Canvas::new(2, 2, [255, 0, 100, 255]);
        let mut stack_inv = PaintLayerStack::new();
        stack_inv.add_layer(PaintLayer::new_effect("Invert", PaintEffect::Invert));
        stack_inv.composite(&mut base_inv);
        assert_eq!(base_inv.get(0, 0), Some([0, 255, 155, 255]));
    }

    #[test]
    fn test_dilate_canvas_uv_island() {
        let mut cv = Canvas::new(16, 16, [0, 0, 0, 0]);
        let color = [255, 100, 50, 255];
        // Pinta um pixel isolado em (5, 5)
        cv.set(5, 5, color);

        // Dilata 2 passos
        PaintModule::dilate_canvas(&mut cv, color, 2);

        // O pixel original permanece pintado
        assert_eq!(cv.get(5, 5), Some(color));
        // O vizinho imediato (distância 1) foi preenchido
        assert_eq!(cv.get(5, 4), Some(color));
        assert_eq!(cv.get(5, 6), Some(color));
        assert_eq!(cv.get(4, 5), Some(color));
        assert_eq!(cv.get(6, 5), Some(color));
        // O vizinho a 2 pixels de distância (distância 2) também foi preenchido
        assert_eq!(cv.get(5, 3), Some(color));
        assert_eq!(cv.get(5, 7), Some(color));
        // Pixels a 3 ou mais pixels de distância permanecem transparentes
        assert_eq!(cv.get(5, 2), Some([0, 0, 0, 0]));
        assert_eq!(cv.get(0, 0), Some([0, 0, 0, 0]));
    }

    #[test]
    fn test_paint_3d_symmetry_x_and_find_mesh_uv() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        let asset = state.project.assets.get_mut(active).unwrap();
        asset.mesh = petunia_mesh::Mesh::cube(2.0).into();
        asset.texture = Some(Canvas::new(64, 64, [0, 0, 0, 255]));

        // Cubo tem faces em x = +1.0 e x = -1.0 (centros em (±1, 0, 0)).
        let (fi, hit_pos) = face_toward(&state, Vec3::X);
        let (mirror_fi, mirror_pos) = face_toward(&state, -Vec3::X);
        assert!((hit_pos - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4);
        assert!((mirror_pos - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-4);
        assert_ne!(fi, mirror_fi);

        // Habilita simetria no eixo X
        state.session.tools.paint_symmetry_x = true;
        state.paint_color = [1.0, 0.0, 0.0];

        let settings = BrushSettings {
            kind: BrushType::Pixel,
            size_px: 2.0,
            hardness: 1.0,
            strength: 1.0,
            flow: 1.0,
            spacing: 0.1,
        };

        let ok = PaintModule::paint_mesh_3d_with_settings(&mut state, fi, hit_pos, settings, false);
        assert!(ok);

        // O UV do centro de cada face vira um texel; ambos devem estar pintados.
        let mesh = &state.project.assets[active].mesh;
        let centroid = |face: usize| {
            let uv = &mesh.faces[face].uv;
            let n = uv.len() as f32;
            [
                uv.iter().map(|u| u[0]).sum::<f32>() / n,
                uv.iter().map(|u| u[1]).sum::<f32>() / n,
            ]
        };
        let px_hit = PaintModule::uv_to_px(&state, centroid(fi)).unwrap();
        let px_sym = PaintModule::uv_to_px(&state, centroid(mirror_fi)).unwrap();
        let canvas = state.project.assets[active].texture.as_ref().unwrap();
        assert_eq!(canvas.get(px_hit.0, px_hit.1), Some([255, 0, 0, 255]));
        assert_eq!(canvas.get(px_sym.0, px_sym.1), Some([255, 0, 0, 255]));
    }

    #[test]
    fn test_canvas_brush_with_symmetry_2d() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        let asset = state.project.assets.get_mut(active).unwrap();
        asset.texture = Some(Canvas::new(32, 32, [0, 0, 0, 255]));

        state.session.tools.paint_symmetry_x = true;
        state.paint_color = [0.0, 1.0, 0.0];

        let settings = BrushSettings {
            kind: BrushType::Pixel,
            size_px: 1.0,
            hardness: 1.0,
            strength: 1.0,
            flow: 1.0,
            spacing: 0.1,
        };

        // Pinta em x = 5, y = 10
        PaintModule::canvas_brush_with_symmetry(&mut state, 5, 10, settings);

        let canvas = state.project.assets[active].texture.as_ref().unwrap();
        assert_eq!(canvas.get(5, 10), Some([0, 255, 0, 255]));
        // Simétrico no eixo X em 32x32: 31 - 5 = 26
        assert_eq!(canvas.get(26, 10), Some([0, 255, 0, 255]));
    }

    #[test]
    fn batched_symmetry_composes_once_and_syncs_only_dirty_tiles() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        state.project.assets[active].texture = Some(Canvas::new(64, 64, [0, 0, 0, 255]));
        state.session.tools.paint_symmetry_x = true;
        state.session.tools.paint_symmetry_y = true;
        state.paint_color = [0.2, 0.8, 0.1];
        PaintModule::ensure_stack(&mut state);

        let before_revision = state.project.project.texture_revision;
        let dirty = PaintModule::canvas_brush_batch_with_symmetry(
            &mut state,
            &[(5, 10), (38, 42)],
            BrushSettings {
                kind: BrushType::Pixel,
                size_px: 6.0,
                hardness: 1.0,
                strength: 1.0,
                flow: 1.0,
                spacing: 0.25,
            },
        );

        assert!(!dirty.is_empty());
        assert!(dirty.indices.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(state.project.project.texture_revision, before_revision + 1);

        let asset = &state.project.assets[active];
        let material = asset
            .material(&state.project.project)
            .expect("material associado");
        assert_eq!(material.albedo_texture.as_ref(), asset.texture.as_ref());
        let updates = state.render.take_texture_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].asset_id, asset.id);
        assert_eq!(
            (updates[0].canvas_width, updates[0].canvas_height),
            (64, 64)
        );
        if let Some(regions) = updates[0].regions.as_ref() {
            assert!(regions.len() <= dirty.len());
        } else {
            assert_eq!(dirty.len(), 4);
        }
    }

    #[test]
    fn dirty_tiles_clamp_dabs_to_canvas_bounds() {
        let mut dirty = DirtyTiles::default();
        dirty.mark_dab(63, 63, 12, 64, 64);
        dirty.mark_dab(100, 100, 12, 64, 64);
        dirty.normalize();

        assert_eq!(dirty.as_slice(), &[3]);
    }

    #[test]
    fn test_paint_vertex_color_3d() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        state.project.assets[active].mesh = petunia_mesh::Mesh::cube(2.0).into();
        let hit = glam::Vec3::new(1.0, 1.0, 1.0);
        let count = PaintModule::paint_vertex_color_3d(&mut state, hit, 1.5, 1.0, [1.0, 0.0, 0.0]);
        assert!(count > 0);
        let mesh = &state.project.assets[active].mesh;
        let v_corner = mesh
            .verts
            .iter()
            .find(|v| {
                (v.pos[0] - 1.0).abs() < 1e-3
                    && (v.pos[1] - 1.0).abs() < 1e-3
                    && (v.pos[2] - 1.0).abs() < 1e-3
            })
            .unwrap();
        assert!((v_corner.color[0] - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_canvas_gradient_linear() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        state.project.assets[active].texture = Some(Canvas::new(16, 16, [0, 0, 0, 255]));
        PaintModule::canvas_gradient_linear(
            &mut state,
            0,
            0,
            15,
            0,
            [255, 0, 0, 255],
            [0, 0, 255, 255],
        );
        let canvas = state.project.assets[active].texture.as_ref().unwrap();
        let left = canvas.get(0, 0).unwrap();
        let right = canvas.get(15, 0).unwrap();
        assert_eq!(left, [255, 0, 0, 255]);
        assert_eq!(right, [0, 0, 255, 255]);
    }

    // ---- Brush system (2E) ----

    fn fresh_state_with_canvas(size: u32, fill: [u8; 4]) -> AppState {
        let mut state = AppState::new("en");
        let active = state.project.active;
        state.project.assets[active].texture = Some(Canvas::new(size, size, fill));
        PaintModule::ensure_stack(&mut state);
        state
    }

    fn texture(state: &AppState) -> &Canvas {
        state.project.active().unwrap().texture.as_ref().unwrap()
    }

    fn brush(kind: BrushType, size: f32) -> BrushSettings {
        BrushSettings {
            kind,
            size_px: size,
            hardness: 1.0,
            strength: 1.0,
            flow: 1.0,
            spacing: 0.15,
        }
    }

    #[test]
    fn square_tip_fills_the_corners_that_a_round_tip_leaves_empty() {
        let corner = |tip: petunia_core::BrushTip| {
            let mut state = fresh_state_with_canvas(64, [0, 0, 0, 255]);
            state.session.tools.brush_style.tip = tip;
            state.paint_color = [1.0, 1.0, 1.0];
            PaintModule::canvas_brush_with_settings(
                &mut state,
                32,
                32,
                brush(BrushType::Pixel, 20.0),
            );
            texture(&state).get(32 + 9, 32 + 9).unwrap()[0]
        };
        assert_eq!(corner(petunia_core::BrushTip::Round), 0);
        assert_eq!(corner(petunia_core::BrushTip::Square), 255);
    }

    #[test]
    fn spray_is_deterministic_and_sparser_than_a_solid_dab() {
        let paint = |density: f32| {
            let mut state = fresh_state_with_canvas(64, [0, 0, 0, 255]);
            state.session.tools.brush_style.spray_density = density;
            state.paint_color = [1.0, 1.0, 1.0];
            PaintModule::canvas_brush_with_settings(
                &mut state,
                32,
                32,
                brush(BrushType::Spray, 30.0),
            );
            texture(&state).pixels.clone()
        };
        let a = paint(0.3);
        assert_eq!(a, paint(0.3), "mesma semente, mesmo resultado");
        let lit = |px: &Vec<u8>| px.chunks(4).filter(|p| p[0] > 0).count();
        assert!(lit(&a) > 0);
        assert!(lit(&a) < lit(&paint(1.0)));
    }

    #[test]
    fn dodge_lightens_and_burn_darkens_the_existing_color() {
        let run = |kind: BrushType| {
            let mut state = fresh_state_with_canvas(32, [100, 100, 100, 255]);
            PaintModule::canvas_brush_with_settings(&mut state, 16, 16, brush(kind, 10.0));
            texture(&state).get(16, 16).unwrap()[0]
        };
        assert!(run(BrushType::Dodge) > 100);
        assert!(run(BrushType::Burn) < 100);
    }

    #[test]
    fn blur_softens_a_hard_edge() {
        let mut state = fresh_state_with_canvas(32, [0, 0, 0, 255]);
        {
            let active = state.project.active;
            let stack = state.project.assets[active].paint_stack.as_mut().unwrap();
            let cv = stack.active_mut().unwrap().canvas_mut().unwrap();
            for y in 0..32 {
                for x in 16..32 {
                    cv.set(x, y, [255, 255, 255, 255]);
                }
            }
        }
        PaintModule::composite_active(&mut state);
        PaintModule::canvas_brush_with_settings(&mut state, 16, 16, brush(BrushType::Blur, 14.0));
        let edge = texture(&state).get(15, 16).unwrap()[0];
        assert!(
            edge > 0 && edge < 255,
            "borda deve ficar intermediária: {edge}"
        );
    }

    #[test]
    fn clone_copies_from_the_source_offset_and_does_nothing_without_one() {
        let mut state = fresh_state_with_canvas(64, [0, 0, 0, 255]);
        {
            let active = state.project.active;
            let stack = state.project.assets[active].paint_stack.as_mut().unwrap();
            let cv = stack.active_mut().unwrap().canvas_mut().unwrap();
            for y in 8..16 {
                for x in 8..16 {
                    cv.set(x, y, [200, 40, 40, 255]);
                }
            }
        }
        PaintModule::composite_active(&mut state);
        // sem origem: nada muda
        PaintModule::canvas_brush_with_settings(&mut state, 40, 40, brush(BrushType::Clone, 8.0));
        assert_eq!(texture(&state).get(40, 40).unwrap(), [0, 0, 0, 255]);
        // origem em (12, 12): o dab em (40, 40) copia o quadrado vermelho
        state.session.tools.clone_source = Some([12.0, 12.0]);
        PaintModule::canvas_brush_with_settings(&mut state, 40, 40, brush(BrushType::Clone, 8.0));
        assert_eq!(texture(&state).get(40, 40).unwrap(), [200, 40, 40, 255]);
    }

    #[test]
    fn smudge_drags_color_along_the_stroke() {
        let mut state = fresh_state_with_canvas(64, [0, 0, 0, 255]);
        {
            let active = state.project.active;
            let stack = state.project.assets[active].paint_stack.as_mut().unwrap();
            let cv = stack.active_mut().unwrap().canvas_mut().unwrap();
            for y in 0..64 {
                for x in 0..24 {
                    cv.set(x, y, [255, 255, 255, 255]);
                }
            }
        }
        PaintModule::composite_active(&mut state);
        state.begin_paint_stroke();
        let s = brush(BrushType::Smudge, 12.0);
        for x in [20u32, 24, 28, 32, 36] {
            PaintModule::canvas_brush_with_settings(&mut state, x, 32, s);
        }
        state.finish_paint_stroke(false);
        // a cor branca foi arrastada para a direita da borda original (x = 24)
        assert!(texture(&state).get(30, 32).unwrap()[0] > 0);
    }

    #[test]
    fn jitter_changes_the_stroke_but_the_same_seed_repeats_it() {
        let run = |seed: u32| {
            let mut state = fresh_state_with_canvas(64, [0, 0, 0, 255]);
            state.session.tools.brush_style.size_jitter = 0.8;
            state.session.tools.brush_style.scatter = 1.0;
            state.session.tools.brush_style.seed = seed;
            state.paint_color = [1.0, 1.0, 1.0];
            state.begin_paint_stroke();
            for x in [10u32, 20, 30, 40, 50] {
                PaintModule::canvas_brush_with_settings(
                    &mut state,
                    x,
                    32,
                    brush(BrushType::Pixel, 8.0),
                );
            }
            state.finish_paint_stroke(false);
            texture(&state).pixels.clone()
        };
        assert_eq!(run(7), run(7));
        assert_ne!(run(7), run(8));
    }

    #[test]
    fn ellipse_fills_the_inscribed_oval_and_leaves_the_corners() {
        let mut cv = Canvas::new(32, 32, [0, 0, 0, 255]);
        PaintModule::stroke_ellipse(
            &mut cv,
            &ShapeStroke {
                x0: 4,
                y0: 4,
                x1: 27,
                y1: 27,
                brush: BrushType::Ellipse,
                color: [255, 255, 255, 255],
                strength: 1.0,
            },
        );
        assert_eq!(cv.get(16, 16), Some([255, 255, 255, 255]));
        assert_eq!(
            cv.get(4, 4),
            Some([0, 0, 0, 255]),
            "o canto do retângulo fica de fora"
        );
        assert_eq!(
            cv.get(16, 5),
            Some([255, 255, 255, 255]),
            "perto do topo da elipse"
        );
    }

    #[test]
    fn gradients_composite_over_existing_paint() {
        let mut state = fresh_state_with_canvas(32, [0, 0, 255, 255]);
        // radial: vermelho opaco no centro -> transparente na borda; o azul de baixo aparece
        PaintModule::canvas_gradient_radial(
            &mut state,
            16,
            16,
            10.0,
            [255, 0, 0, 255],
            [255, 0, 0, 0],
        );
        let tex = texture(&state);
        assert_eq!(tex.get(16, 16), Some([255, 0, 0, 255]));
        assert_eq!(
            tex.get(0, 0),
            Some([0, 0, 255, 255]),
            "fora do raio nada muda"
        );
        let mid = tex.get(21, 16).unwrap();
        assert!(
            mid[0] > 0 && mid[2] > 0,
            "meio-termo mistura as duas cores: {mid:?}"
        );
    }
}
