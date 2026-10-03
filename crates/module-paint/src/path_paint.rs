//! Path Paint (PAINT, P3D-158/160/161/062; foundations 43/44): pinta o pincel
//! ao longo de um caminho definido por pontos clicados sobre a superfície.
//!
//! Camadas, de baixo para cima (cada uma pura e testável isoladamente):
//! 1. [`sample_path`] — poligonal ou Catmull-Rom centrípeta reamostrada em
//!    espaçamento **igual por comprimento de arco**;
//! 2. [`snap_to_surface`] / [`project_path_onto_surface`] — cola cada amostra
//!    no ponto mais próximo das faces permitidas ([`SurfaceHit`]);
//! 3. [`PaintModule::paint_surface_path`] / [`PaintModule::paint_surface_line`]
//!    — carimba os dabs dentro de **um único traço** (um Undo), reutilizando
//!    o motor 3D (restrição por seleção/trava, buffer de opacidade por traço);
//! 4. [`line_pixels`] / [`pixel_perfect`] — traço reto e filtro *pixel perfect*
//!    para linhas de 1 px no canvas 2D.
//!
//! O módulo não conhece UI, teclas nem câmera além do que o motor 3D já usa
//! (`AppState::world_radius_for_px`). Oclusão não é tratada aqui: quem chama
//! escolhe os nós sobre a superfície visível.

use std::sync::Arc;

use glam::Vec3;
use petunia_core::{AppState, BrushSettings, PaintRestriction};
use petunia_mesh::Mesh;
use petunia_mesh::uv_layout::closest_point_on_triangle;

use crate::PaintModule;

/// Teto de nós de entrada considerados (entrada hostil não vira alocação sem limite).
pub const MAX_PATH_NODES: usize = 8192;
/// Teto de dabs de um único caminho.
pub const MAX_PATH_DABS: usize = 4096;
/// Maior lado (em pixels) aceito por [`line_pixels`].
pub const MAX_LINE_PIXELS: i64 = 1 << 20;

/// Coordenada de mundo acima da qual o nó é tratado como inválido.
const MAX_COORD: f32 = 1.0e15;
/// Subdivisões por segmento ao achatar a Catmull-Rom antes de reamostrar.
const SPLINE_SUBDIVISIONS: usize = 32;
/// Distância abaixo da qual dois pontos consecutivos são o mesmo ponto.
const DUPLICATE_EPS: f32 = 1.0e-6;

// ---------------------------------------------------------------------------
// 1. Amostragem do caminho
// ---------------------------------------------------------------------------

/// Reamostra o caminho definido por `nodes` em espaçamento igual por
/// comprimento de arco.
///
/// - `smooth = false`: poligonal; `smooth = true`: Catmull-Rom **centrípeta**
///   (passa por todos os nós, sem laços nem cúspides por overshoot).
/// - Sempre inclui o primeiro nó e, em caminho aberto, o último. Caminho
///   fechado dá a volta (o ponto final não repete o inicial) e o passo é
///   ajustado para fechar a volta com espaçamento uniforme. Fechar exige ao
///   menos 3 nós distintos; com menos, o caminho é tratado como aberto.
/// - `spacing` inválido (`<= 0`, NaN, infinito) vira 1/100 do comprimento total.
///   O espaçamento nunca é menor que o necessário para respeitar `max_samples`.
/// - Nós não finitos (ou absurdamente grandes) são descartados, assim como
///   nós consecutivos repetidos. Só os primeiros [`MAX_PATH_NODES`] valem.
pub fn sample_path(
    nodes: &[Vec3],
    closed: bool,
    smooth: bool,
    spacing: f32,
    max_samples: usize,
) -> Vec<Vec3> {
    if max_samples == 0 {
        return Vec::new();
    }
    let mut pts = clean_nodes(nodes);
    if pts.is_empty() {
        return Vec::new();
    }
    if pts.len() == 1 || max_samples == 1 {
        return vec![pts[0]];
    }
    // Fechar o laço: o último nó não pode repetir o primeiro.
    if closed
        && pts.len() > 1
        && let (Some(&first), Some(&last)) = (pts.first(), pts.last())
        && first.distance(last) <= DUPLICATE_EPS
    {
        pts.pop();
    }
    let closed = closed && pts.len() >= 3;

    let dense = if smooth {
        flatten_catmull_rom(&pts, closed)
    } else {
        let mut line = pts.clone();
        if closed {
            line.push(pts[0]);
        }
        line
    };
    resample_equal_arc(&dense, closed, spacing, max_samples)
}

/// Remove nós inválidos e repetidos consecutivos; limita a quantidade.
fn clean_nodes(nodes: &[Vec3]) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = Vec::with_capacity(nodes.len().min(MAX_PATH_NODES));
    for &p in nodes {
        if !p.is_finite() || p.abs().max_element() > MAX_COORD {
            continue;
        }
        if out.last().is_some_and(|q| q.distance(p) <= DUPLICATE_EPS) {
            continue;
        }
        out.push(p);
        if out.len() >= MAX_PATH_NODES {
            break;
        }
    }
    out
}

/// Catmull-Rom centrípeta (alpha = 0,5) achatada em poligonal densa.
/// Aberta: as pontas usam o vizinho refletido. Fechada: índices circulares e a
/// poligonal termina de volta no primeiro nó.
fn flatten_catmull_rom(pts: &[Vec3], closed: bool) -> Vec<Vec3> {
    let n = pts.len();
    let at = |i: isize| -> Vec3 {
        if closed {
            pts[i.rem_euclid(n as isize) as usize]
        } else if i < 0 {
            2.0 * pts[0] - pts[1]
        } else if i as usize >= n {
            2.0 * pts[n - 1] - pts[n - 2]
        } else {
            pts[i as usize]
        }
    };
    let segments = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(segments * SPLINE_SUBDIVISIONS + 1);
    out.push(pts[0]);
    for s in 0..segments as isize {
        let (p0, p1, p2, p3) = (at(s - 1), at(s), at(s + 1), at(s + 2));
        // Nós centrípetos; o piso evita divisão por zero em pontos coincidentes.
        let knot = |a: Vec3, b: Vec3| a.distance(b).max(1.0e-9).sqrt();
        let t0 = 0.0f32;
        let t1 = t0 + knot(p0, p1);
        let t2 = t1 + knot(p1, p2);
        let t3 = t2 + knot(p2, p3);
        for k in 1..=SPLINE_SUBDIVISIONS {
            let t = t1 + (t2 - t1) * (k as f32 / SPLINE_SUBDIVISIONS as f32);
            let a1 = p0 * ((t1 - t) / (t1 - t0)) + p1 * ((t - t0) / (t1 - t0));
            let a2 = p1 * ((t2 - t) / (t2 - t1)) + p2 * ((t - t1) / (t2 - t1));
            let a3 = p2 * ((t3 - t) / (t3 - t2)) + p3 * ((t - t2) / (t3 - t2));
            let b1 = a1 * ((t2 - t) / (t2 - t0)) + a2 * ((t - t0) / (t2 - t0));
            let b2 = a2 * ((t3 - t) / (t3 - t1)) + a3 * ((t - t1) / (t3 - t1));
            let c = b1 * ((t2 - t) / (t2 - t1)) + b2 * ((t - t1) / (t2 - t1));
            // Passo exato no último ponto: o segmento termina no nó, sem deriva.
            out.push(if k == SPLINE_SUBDIVISIONS { p2 } else { c });
        }
    }
    out.retain(|p| p.is_finite());
    out
}

/// Reamostra uma poligonal densa em passos iguais de comprimento de arco.
fn resample_equal_arc(dense: &[Vec3], closed: bool, spacing: f32, max_samples: usize) -> Vec<Vec3> {
    let Some(&first) = dense.first() else {
        return Vec::new();
    };
    // Comprimento acumulado (f64: poligonais longas e densas somam sem perda).
    let mut cum = Vec::with_capacity(dense.len());
    let mut total = 0.0f64;
    cum.push(0.0f64);
    for w in dense.windows(2) {
        total += w[0].distance(w[1]) as f64;
        cum.push(total);
    }
    if !total.is_finite() || total <= 0.0 {
        return vec![first];
    }
    let last = dense[dense.len() - 1];

    // Espaçamento efetivo: válido e respeitando `max_samples`.
    let requested = if spacing.is_finite() && spacing > 0.0 {
        spacing as f64
    } else {
        total / 100.0
    };
    let intervals_cap = if closed {
        max_samples as f64
    } else {
        (max_samples - 1) as f64
    };
    let step = requested.max(total / intervals_cap).max(f64::MIN_POSITIVE);

    let mut out: Vec<Vec3> = Vec::new();
    let mut seg = 0usize;
    let point_at = |d: f64, seg: &mut usize| -> Vec3 {
        while *seg + 2 < cum.len() && cum[*seg + 1] < d {
            *seg += 1;
        }
        let (a, b) = (dense[*seg], dense[*seg + 1]);
        let len = cum[*seg + 1] - cum[*seg];
        let t = if len > 0.0 {
            ((d - cum[*seg]) / len).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        a.lerp(b, t)
    };

    if closed {
        // Passo ajustado: a volta fecha com espaçamento uniforme.
        let n = ((total / step).round() as usize).clamp(1, max_samples);
        let step = total / n as f64;
        for i in 0..n {
            out.push(point_at(step * i as f64, &mut seg));
        }
    } else {
        let mut d = 0.0f64;
        while d <= total && out.len() < max_samples {
            out.push(point_at(d, &mut seg));
            d += step;
        }
        // O último nó sempre entra (a menos que a amostra já caia nele).
        let needs_end = out
            .last()
            .is_none_or(|p| p.distance(last) > (step * 1.0e-4) as f32);
        if needs_end {
            if out.len() >= max_samples {
                out.truncate(max_samples - 1);
            }
            out.push(last);
        }
        if let Some(p) = out.first_mut() {
            *p = first;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 2. Projeção na superfície
// ---------------------------------------------------------------------------

/// Ponto da superfície encontrado: face e posição de mundo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceHit {
    pub face: usize,
    pub position: Vec3,
}

/// Triângulos (posições de mundo) de uma face, no mesmo corte que o desenho e
/// o picking usam. Faces inválidas ou degeneradas não produzem triângulos.
fn face_tris(mesh: &Mesh, fi: usize, out: &mut Vec<[Vec3; 3]>) {
    let Some(face) = mesh.faces.get(fi) else {
        return;
    };
    for [a, b, c] in mesh.face_triangle_corners(fi) {
        let (Some(&va), Some(&vb), Some(&vc)) =
            (face.verts.get(a), face.verts.get(b), face.verts.get(c))
        else {
            continue;
        };
        let (Some(pa), Some(pb), Some(pc)) = (
            mesh.verts.get(va as usize),
            mesh.verts.get(vb as usize),
            mesh.verts.get(vc as usize),
        ) else {
            continue;
        };
        let tri = [pa.vec(), pb.vec(), pc.vec()];
        if tri.iter().all(|p| p.is_finite()) {
            out.push(tri);
        }
    }
}

/// Distância ao quadrado de `p` à caixa `[lo, hi]`.
fn dist_sq_to_aabb(p: Vec3, lo: Vec3, hi: Vec3) -> f32 {
    let d = (lo - p).max(p - hi).max(Vec3::ZERO);
    d.length_squared()
}

/// Melhor ponto de uma lista de triângulos para `p` (distância ao quadrado).
fn closest_in_tris(tris: &[[Vec3; 3]], p: Vec3) -> Option<(f32, Vec3)> {
    let mut best: Option<(f32, Vec3)> = None;
    for &[a, b, c] in tris {
        let q = closest_point_on_triangle(p, a, b, c);
        if !q.is_finite() {
            continue; // triângulo degenerado
        }
        let d = q.distance_squared(p);
        if d.is_finite() && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, q));
        }
    }
    best
}

fn valid_query(point: Vec3, max_distance: f32) -> bool {
    point.is_finite() && !max_distance.is_nan() && max_distance >= 0.0
}

/// Ponto mais próximo de `point` nas faces permitidas, até `max_distance`.
///
/// Empates de distância resolvem para a menor face (determinismo). Faz uma
/// varredura linear com teste de caixa; para muitas consultas na mesma malha
/// use [`project_path_onto_surface`].
pub fn snap_to_surface(
    mesh: &Mesh,
    point: Vec3,
    max_distance: f32,
    face_allowed: &dyn Fn(usize) -> bool,
) -> Option<SurfaceHit> {
    if !valid_query(point, max_distance) {
        return None;
    }
    let mut limit_sq = if max_distance.is_finite() {
        max_distance * max_distance
    } else {
        f32::INFINITY
    };
    let mut best: Option<SurfaceHit> = None;
    let mut tris = Vec::new();
    for fi in 0..mesh.faces.len() {
        if !face_allowed(fi) {
            continue;
        }
        tris.clear();
        face_tris(mesh, fi, &mut tris);
        let Some((lo, hi)) = tris_bounds(&tris) else {
            continue;
        };
        if dist_sq_to_aabb(point, lo, hi) > limit_sq {
            continue;
        }
        if let Some((d, q)) = closest_in_tris(&tris, point)
            && d <= limit_sq
            && (d < limit_sq || best.is_none())
        {
            // `<` estrito mantém a menor face em empate; o primeiro acerto
            // exato no limite também vale.
            limit_sq = d;
            best = Some(SurfaceHit {
                face: fi,
                position: q,
            });
        }
    }
    best
}

fn tris_bounds(tris: &[[Vec3; 3]]) -> Option<(Vec3, Vec3)> {
    let first = tris.first()?;
    let mut lo = first[0];
    let mut hi = first[0];
    for tri in tris {
        for &p in tri {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    Some((lo, hi))
}

/// Faces permitidas pré-processadas: triângulos, caixas e grade uniforme.
struct SurfaceIndex {
    tris: Vec<[Vec3; 3]>,
    /// Faixa `[start, end)` de `tris` por entrada.
    ranges: Vec<(u32, u32)>,
    faces: Vec<usize>,
    lo: Vec<Vec3>,
    hi: Vec<Vec3>,
    origin: Vec3,
    cell: f32,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
    /// Entradas grandes demais para a grade: testadas em toda consulta.
    big: Vec<u32>,
}

impl SurfaceIndex {
    fn new(mesh: &Mesh, face_allowed: &dyn Fn(usize) -> bool) -> Self {
        let mut idx = Self {
            tris: Vec::new(),
            ranges: Vec::new(),
            faces: Vec::new(),
            lo: Vec::new(),
            hi: Vec::new(),
            origin: Vec3::ZERO,
            cell: 1.0,
            dims: [1, 1, 1],
            cells: Vec::new(),
            big: Vec::new(),
        };
        for fi in 0..mesh.faces.len() {
            if !face_allowed(fi) {
                continue;
            }
            let start = idx.tris.len();
            face_tris(mesh, fi, &mut idx.tris);
            let Some((lo, hi)) = tris_bounds(&idx.tris[start..]) else {
                continue;
            };
            idx.ranges.push((start as u32, idx.tris.len() as u32));
            idx.faces.push(fi);
            idx.lo.push(lo);
            idx.hi.push(hi);
        }
        if idx.faces.is_empty() {
            return idx;
        }
        let lo = idx
            .lo
            .iter()
            .fold(Vec3::splat(f32::INFINITY), |a, &b| a.min(b));
        let hi = idx
            .hi
            .iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), |a, &b| a.max(b));
        let ext = (hi - lo).max(Vec3::splat(1.0e-6));
        // Refina a célula até ter ~1 célula por face (teto 65k), no máximo 256 por eixo.
        let target = idx.faces.len().clamp(1, 1 << 16);
        let max_ext = ext.max_element();
        let mut cell = max_ext / 2.0;
        let dims_for = |cell: f32| -> [usize; 3] {
            [ext.x, ext.y, ext.z].map(|e| ((e / cell).ceil() as usize).clamp(1, 256))
        };
        while dims_for(cell).iter().product::<usize>() < target && cell > max_ext / 256.0 {
            cell *= 0.5;
        }
        idx.origin = lo;
        idx.cell = cell;
        idx.dims = dims_for(cell);
        idx.cells = vec![Vec::new(); idx.dims.iter().product()];
        for e in 0..idx.faces.len() {
            let (a, b) = (idx.cell_coords(idx.lo[e]), idx.cell_coords(idx.hi[e]));
            let count: usize = (0..3).map(|k| b[k] - a[k] + 1).product();
            if count > 64 {
                idx.big.push(e as u32);
                continue;
            }
            for z in a[2]..=b[2] {
                for y in a[1]..=b[1] {
                    for x in a[0]..=b[0] {
                        let c = idx.cell_index([x, y, z]);
                        idx.cells[c].push(e as u32);
                    }
                }
            }
        }
        idx
    }

    fn cell_coords(&self, p: Vec3) -> [usize; 3] {
        let rel = (p - self.origin) / self.cell;
        [rel.x, rel.y, rel.z]
            .into_iter()
            .zip(self.dims)
            .map(|(v, n)| (v.max(0.0).floor() as usize).min(n - 1))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or([0, 0, 0])
    }

    fn cell_index(&self, c: [usize; 3]) -> usize {
        (c[2] * self.dims[1] + c[1]) * self.dims[0] + c[0]
    }

    /// Vizinho mais próximo exato: busca em caixas crescentes e confirma que
    /// nada mais perto escapou da caixa consultada.
    fn nearest(&self, p: Vec3, max_distance: f32) -> Option<SurfaceHit> {
        if self.faces.is_empty() || !valid_query(p, max_distance) {
            return None;
        }
        let mut seen = vec![false; self.faces.len()];
        let mut best: Option<(f32, usize, Vec3)> = None; // (d², entrada, ponto)
        let mut radius = self.cell;
        loop {
            let radius_c = radius.min(max_distance);
            let box_lo = p - Vec3::splat(radius_c);
            let box_hi = p + Vec3::splat(radius_c);
            let limit_sq = match best {
                Some((d, _, _)) => d.min(max_distance * max_distance),
                None => max_distance * max_distance,
            };
            let mut visit = |e: usize, best: &mut Option<(f32, usize, Vec3)>| {
                if seen[e] {
                    return;
                }
                seen[e] = true;
                let limit = best.map_or(limit_sq, |(d, _, _)| d.min(limit_sq));
                if dist_sq_to_aabb(p, self.lo[e], self.hi[e]) > limit {
                    return;
                }
                let (s, t) = self.ranges[e];
                let Some((d, q)) = closest_in_tris(&self.tris[s as usize..t as usize], p) else {
                    return;
                };
                if d > limit_sq {
                    return;
                }
                let better = match *best {
                    None => true,
                    Some((bd, be, _)) => d < bd || (d == bd && self.faces[e] < self.faces[be]),
                };
                if better {
                    *best = Some((d, e, q));
                }
            };
            for &e in &self.big {
                visit(e as usize, &mut best);
            }
            // Caixa consultada interseca a grade?
            let grid_hi = self.origin
                + Vec3::new(
                    self.dims[0] as f32,
                    self.dims[1] as f32,
                    self.dims[2] as f32,
                ) * self.cell;
            let overlaps = box_lo.cmple(grid_hi).all() && box_hi.cmpge(self.origin).all();
            if overlaps {
                let a = self.cell_coords(box_lo);
                let b = self.cell_coords(box_hi);
                for z in a[2]..=b[2] {
                    for y in a[1]..=b[1] {
                        for x in a[0]..=b[0] {
                            for &e in &self.cells[self.cell_index([x, y, z])] {
                                visit(e as usize, &mut best);
                            }
                        }
                    }
                }
            }
            match best {
                // Tudo a até `radius_c` foi visto: o melhor é exato se couber nela.
                Some((d, _, _)) if d.sqrt() <= radius_c => break,
                Some((d, _, _)) => radius = d.sqrt() * 1.0001 + f32::EPSILON,
                None if radius >= max_distance => break,
                None => radius *= 2.0,
            }
            if !radius.is_finite() {
                break;
            }
        }
        let (d, e, q) = best?;
        (d <= max_distance * max_distance || max_distance.is_infinite()).then_some(SurfaceHit {
            face: self.faces[e],
            position: q,
        })
    }
}

/// Cola cada amostra no ponto mais próximo das faces permitidas.
///
/// Amostras sem face a até `max_distance` são descartadas; pontos consecutivos
/// iguais (mesma face e mesma posição) colapsam em um. Pré-processa a malha
/// uma única vez (caixas por face + grade uniforme), então ~2000 amostras em
/// ~20k faces custam bem menos que 2000 varreduras lineares.
pub fn project_path_onto_surface(
    mesh: &Mesh,
    samples: &[Vec3],
    max_distance: f32,
    face_allowed: &dyn Fn(usize) -> bool,
) -> Vec<SurfaceHit> {
    if samples.is_empty() || max_distance.is_nan() || max_distance < 0.0 {
        return Vec::new();
    }
    let index = SurfaceIndex::new(mesh, face_allowed);
    let mut out: Vec<SurfaceHit> = Vec::with_capacity(samples.len());
    for &s in samples {
        let Some(hit) = index.nearest(s, max_distance) else {
            continue;
        };
        if out.last().is_some_and(|l| {
            l.face == hit.face && l.position.distance(hit.position) <= DUPLICATE_EPS
        }) {
            continue;
        }
        out.push(hit);
    }
    out
}

// ---------------------------------------------------------------------------
// 3. Pintura (entradas de PaintModule)
// ---------------------------------------------------------------------------

/// Diagonal da caixa da malha (0 se vazia).
fn mesh_diagonal(mesh: &Mesh) -> f32 {
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for v in &mesh.verts {
        let p = v.vec();
        if p.is_finite() {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    if lo.x > hi.x { 0.0 } else { lo.distance(hi) }
}

impl PaintModule {
    /// Pinta o pincel ao longo de um caminho cujos `nodes` já estão sobre a
    /// superfície visível (oclusão é responsabilidade de quem chama).
    ///
    /// Devolve o número de dabs carimbados (sem contar as cópias da simetria).
    /// O caminho inteiro roda num **único traço**: `begin_paint_stroke` ...
    /// `finish_paint_stroke(false)`, logo é uma entrada de Undo e o buffer do
    /// traço limita a opacidade. Se já houver um traço em andamento (o caller
    /// o abriu), ele é reaproveitado e **não** é finalizado aqui. Sem poder
    /// abrir o traço (modal/preview ativo) nada é pintado.
    ///
    /// Espaçamento: `settings.size_px * settings.spacing` pixels de tela,
    /// convertido para mundo pela mesma `AppState::world_radius_for_px` do
    /// dab 3D, na profundidade do nó mais próximo da câmera (nunca deixa
    /// buracos). Restrições (seleção, trava, `isolate_selection`) valem como
    /// num traço comum; as amostras só colam em faces elegíveis.
    pub fn paint_surface_path(
        state: &mut AppState,
        nodes: &[Vec3],
        closed: bool,
        smooth: bool,
        settings: BrushSettings,
        isolate_selection: bool,
    ) -> usize {
        let s = settings.sanitized();
        if s.kind.is_shape() || nodes.is_empty() {
            return 0;
        }
        let Some(mesh) = state.project.active_mesh() else {
            return 0;
        };
        if mesh.faces.is_empty() {
            return 0;
        }
        let diagonal = mesh_diagonal(mesh);
        // Passo de mundo = diâmetro do pincel de `dab_step_px` na menor
        // profundidade do caminho (mesma conversão do dab 3D).
        let step_px = s.dab_step_px();
        let mut world_step = f32::INFINITY;
        let mut world_radius = f32::INFINITY;
        for &n in nodes.iter().filter(|n| n.is_finite()).take(MAX_PATH_NODES) {
            world_step = world_step.min(2.0 * state.world_radius_for_px(n, step_px));
            world_radius = world_radius.min(state.world_radius_for_px(n, s.size_px));
        }
        if !world_step.is_finite() || !world_radius.is_finite() {
            return 0;
        }
        let samples = sample_path(nodes, closed, smooth, world_step, MAX_PATH_DABS);
        if samples.is_empty() {
            return 0;
        }

        // Abre o traço (um Undo) antes de qualquer mutação.
        let owns_stroke = state.session.tools.paint_stroke.is_none();
        if owns_stroke {
            state.begin_paint_stroke();
            if state.session.tools.paint_stroke.is_none() {
                return 0;
            }
        }

        let restriction = Self::path_restriction(state, &samples, isolate_selection);
        let allowed = |f: usize| restriction.as_ref().is_none_or(|r| r.allows_face(f));
        let max_distance = (diagonal * 0.25).max(world_radius * 4.0);
        let hits: Vec<(usize, Vec3)> = match state.project.active_mesh() {
            Some(mesh) => project_path_onto_surface(mesh, &samples, max_distance, &allowed)
                .into_iter()
                .map(|h| (h.face, h.position))
                .collect(),
            None => Vec::new(),
        };
        let painted = !hits.is_empty()
            && Self::paint_mesh_3d_batch_with_settings(state, &hits, s, isolate_selection);
        if owns_stroke {
            state.finish_paint_stroke(false);
        }
        if painted { hits.len() } else { 0 }
    }

    /// Linha reta entre dois pontos da superfície (Shift+clique): mesma
    /// maquinaria de [`PaintModule::paint_surface_path`] com dois nós.
    pub fn paint_surface_line(
        state: &mut AppState,
        a: Vec3,
        b: Vec3,
        settings: BrushSettings,
        isolate_selection: bool,
    ) -> usize {
        Self::paint_surface_path(state, &[a, b], false, false, settings, isolate_selection)
    }

    /// Resolve (e guarda no traço) a restrição de faces do caminho, igual a um
    /// traço comum. Com trava na primeira face, a primeira amostra elegível
    /// pela seleção define a face travada.
    fn path_restriction(
        state: &mut AppState,
        samples: &[Vec3],
        isolate_selection: bool,
    ) -> Option<Arc<PaintRestriction>> {
        use petunia_core::BrushLock;
        let forced = isolate_selection && !state.session.tools.paint_isolate_selection;
        if forced {
            state.session.tools.paint_isolate_selection = true;
        }
        let first_face = if state.session.tools.brush_lock == BrushLock::FirstFace {
            let by_selection = state.session.tools.paint_isolate_selection;
            let sel = &state.session.selection.faces;
            state.project.active_mesh().and_then(|mesh| {
                let eligible =
                    |f: usize| !by_selection || mesh.faces[f].selected || sel.contains(&f);
                let first = *samples.first()?;
                snap_to_surface(mesh, first, f32::INFINITY, &eligible).map(|h| h.face)
            })
        } else {
            None
        };
        let restriction = Self::resolve_restriction(state, first_face);
        if forced {
            state.session.tools.paint_isolate_selection = false;
        }
        restriction
    }
}

// ---------------------------------------------------------------------------
// 4. Linha reta e pixel perfect (canvas 2D)
// ---------------------------------------------------------------------------

/// Pixels da reta de Bresenham entre `a` e `b`, extremos incluídos, na ordem
/// de `a` para `b`. Retas com mais de [`MAX_LINE_PIXELS`] de lado devolvem
/// vazio (entrada hostil não vira alocação gigante).
pub fn line_pixels(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (x0, y0) = (a.0 as i64, a.1 as i64);
    let (x1, y1) = (b.0 as i64, b.1 as i64);
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    if dx.max(-dy) > MAX_LINE_PIXELS {
        return Vec::new();
    }
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    let mut out = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    loop {
        out.push((x as i32, y as i32));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

/// Filtro *pixel perfect* (estilo Aseprite) para uma linha livre de 1 px:
/// remove o pixel do meio de cada canto em L (passo horizontal seguido de
/// vertical, ou o inverso, entre vizinhos 4-conectados), deixando a diagonal
/// limpa. Primeiro e último pixels permanecem; nada além dos cantos em L muda
/// (repetidos, saltos e retas ficam como estão).
pub fn pixel_perfect(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let step = |a: (i32, i32), b: (i32, i32)| (b.0 as i64 - a.0 as i64, b.1 as i64 - a.1 as i64);
    let n = points.len();
    let mut out: Vec<(i32, i32)> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        if i > 0 && i + 1 < n {
            let (prev, cur, next) = (points[i - 1], points[i], points[i + 1]);
            let (d1, d2) = (step(prev, cur), step(cur, next));
            let unit = |d: (i64, i64)| d.0.abs() + d.1.abs() == 1;
            // Passos unitários e perpendiculares: `cur` é o canto do L.
            if unit(d1) && unit(d2) && (d1.0 == 0) != (d2.0 == 0) {
                // Mantém `next` e pula-o como candidato (como o Aseprite).
                out.push(next);
                i += 2;
                continue;
            }
        }
        out.push(points[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::BrushType;
    use petunia_mesh::{Face, Vertex};

    fn dists(p: &[Vec3]) -> Vec<f32> {
        p.windows(2).map(|w| w[0].distance(w[1])).collect()
    }

    // ---- sample_path ----

    #[test]
    fn polyline_has_equal_arc_spacing_and_includes_ends() {
        // L: 10 em X depois 10 em Y; passo 1 cai exatamente nas pontas e no canto.
        let nodes = [
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
        ];
        let s = sample_path(&nodes, false, false, 1.0, 1000);
        assert_eq!(s.len(), 21);
        assert_eq!(s[0], nodes[0]);
        assert!(s.last().unwrap().distance(nodes[2]) < 1e-5);
        // Comprimento de arco entre amostras: 1 (no canto a corda também é 1).
        for d in dists(&s) {
            assert!((d - 1.0).abs() < 0.02, "d={d}");
        }
    }

    #[test]
    fn open_path_always_ends_on_last_node_even_with_remainder() {
        let nodes = [Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)];
        let s = sample_path(&nodes, false, false, 3.0, 1000);
        assert_eq!(s.len(), 5); // 0,3,6,9,10
        assert_eq!(s[0], Vec3::ZERO);
        assert!((s[4].x - 10.0).abs() < 1e-6);
        assert!((s[3].x - 9.0).abs() < 1e-5);
    }

    #[test]
    fn closed_catmull_rom_circle_has_even_spacing() {
        let r = 5.0;
        let nodes: Vec<Vec3> = (0..8)
            .map(|i| {
                let t = i as f32 / 8.0 * std::f32::consts::TAU;
                Vec3::new(r * t.cos(), r * t.sin(), 0.0)
            })
            .collect();
        let s = sample_path(&nodes, true, true, 0.5, 4096);
        assert!(s.len() > 50);
        // Fechado: inclui a volta (último->primeiro) e o primeiro não se repete.
        let mut ring = s.clone();
        ring.push(s[0]);
        let d = dists(&ring);
        let mean = d.iter().sum::<f32>() / d.len() as f32;
        for &x in &d {
            assert!((x - mean).abs() / mean < 0.02, "d={x} mean={mean}");
        }
        // A curva passa pelo primeiro nó e fica perto do círculo (sem overshoot grande).
        assert!(s[0].distance(nodes[0]) < 1e-5);
        for p in &s {
            assert!(
                (p.length() - r).abs() < 0.2,
                "fora do círculo: {}",
                p.length()
            );
        }
    }

    #[test]
    fn catmull_rom_passes_through_every_node() {
        let nodes = [
            Vec3::ZERO,
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(3.0, 2.5, 1.0),
            Vec3::new(5.0, 0.0, 0.0),
        ];
        let s = sample_path(&nodes, false, true, 0.05, 4096);
        for n in nodes {
            let near = s
                .iter()
                .map(|p| p.distance(n))
                .fold(f32::INFINITY, f32::min);
            assert!(near < 0.03, "nó {n:?} a {near}");
        }
        assert_eq!(s[0], nodes[0]);
        assert!(s.last().unwrap().distance(nodes[3]) < 1e-5);
    }

    #[test]
    fn centripetal_spline_has_no_cusp_on_uneven_nodes() {
        // Nós muito desiguais: a variante uniforme faria laço/cúspide.
        let nodes = [
            Vec3::ZERO,
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.1, 0.1, 0.0),
        ];
        let s = sample_path(&nodes, false, true, 0.05, 8192);
        // Sem laço em X: monotônico não decrescente (tolerância numérica).
        for w in s.windows(2) {
            assert!(
                w[1].x >= w[0].x - 0.02,
                "recuo em X: {} -> {}",
                w[0].x,
                w[1].x
            );
        }
    }

    #[test]
    fn sample_path_respects_max_samples_and_keeps_end() {
        let nodes = [Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0)];
        let s = sample_path(&nodes, false, false, 0.0001, 10);
        assert!(s.len() <= 10);
        assert_eq!(s[0], Vec3::ZERO);
        assert!((s.last().unwrap().x - 100.0).abs() < 1e-4);
        let closed = sample_path(
            &[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0)],
            true,
            true,
            1e-9,
            7,
        );
        assert!(closed.len() <= 7 && !closed.is_empty());
    }

    #[test]
    fn sample_path_degenerate_inputs_are_harmless() {
        assert!(sample_path(&[], false, false, 1.0, 10).is_empty());
        assert!(sample_path(&[Vec3::X], false, true, 1.0, 0).is_empty());
        assert_eq!(sample_path(&[Vec3::X], false, true, 1.0, 10), vec![Vec3::X]);
        // Todos coincidentes: um ponto.
        assert_eq!(
            sample_path(&[Vec3::X; 5], true, true, 1.0, 10),
            vec![Vec3::X]
        );
        // NaN/inf descartados.
        let nodes = [
            Vec3::new(f32::NAN, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::new(f32::INFINITY, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(1.0e30, 0.0, 0.0),
        ];
        let s = sample_path(&nodes, false, false, 1.0, 100);
        assert_eq!(s.len(), 5);
        assert!(s.iter().all(|p| p.is_finite()));
        // Espaçamento inválido: usa o mínimo sensato.
        for bad in [0.0, -3.0, f32::NAN, f32::INFINITY] {
            let s = sample_path(&[Vec3::ZERO, Vec3::X], false, false, bad, 1000);
            assert!(s.len() >= 2 && s.len() <= 1000, "{bad}: {}", s.len());
            assert_eq!(s[0], Vec3::ZERO);
            assert!(s.last().unwrap().distance(Vec3::X) < 1e-6);
        }
        // Fechado com 2 nós vira aberto; fechado repetindo o primeiro nó não duplica.
        assert!(sample_path(&[Vec3::ZERO, Vec3::X], true, true, 0.1, 100).len() >= 2);
        let sq = [Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::ZERO];
        let s = sample_path(&sq, true, false, 0.25, 100);
        assert!(s.iter().skip(1).all(|p| p.distance(s[0]) > 1e-3));
    }

    #[test]
    fn sample_path_is_deterministic() {
        let nodes: Vec<Vec3> = (0..20)
            .map(|i| Vec3::new(i as f32, (i as f32 * 0.7).sin(), (i as f32 * 0.3).cos()))
            .collect();
        let a = sample_path(&nodes, false, true, 0.3, 2000);
        let b = sample_path(&nodes, false, true, 0.3, 2000);
        assert_eq!(a, b);
    }

    #[test]
    fn sample_path_huge_node_count_is_capped() {
        let nodes: Vec<Vec3> = (0..100_000)
            .map(|i| Vec3::new(i as f32, 0.0, 0.0))
            .collect();
        let s = sample_path(&nodes, false, true, 1.0, 5000);
        assert!(s.len() <= 5000 && !s.is_empty());
    }

    // ---- superfície ----

    /// Grade `n x n` de quads triangulados no plano XZ (y = 0), lado `size`.
    fn grid_mesh(n: usize, size: f32) -> Mesh {
        let mut m = Mesh::default();
        for j in 0..=n {
            for i in 0..=n {
                m.verts.push(Vertex::new(
                    i as f32 / n as f32 * size,
                    0.0,
                    j as f32 / n as f32 * size,
                ));
            }
        }
        let id = |i: usize, j: usize| (j * (n + 1) + i) as u32;
        for j in 0..n {
            for i in 0..n {
                m.push_face(Face::new(vec![id(i, j), id(i, j + 1), id(i + 1, j)]));
                m.push_face(Face::new(vec![
                    id(i + 1, j),
                    id(i, j + 1),
                    id(i + 1, j + 1),
                ]));
            }
        }
        m
    }

    #[test]
    fn snap_picks_nearest_face_and_honours_face_allowed() {
        let m = Mesh::cube(2.0);
        // Ponto sobre a face z=+1 (face 1), levemente fora.
        let p = Vec3::new(0.2, 0.1, 1.3);
        let hit = snap_to_surface(&m, p, 10.0, &|_| true).unwrap();
        assert_eq!(hit.face, 1);
        assert!((hit.position - Vec3::new(0.2, 0.1, 1.0)).length() < 1e-5);
        // Sem a face 1: cai numa face vizinha (lateral), nunca na proibida.
        let hit = snap_to_surface(&m, p, 10.0, &|f| f != 1).unwrap();
        assert_ne!(hit.face, 1);
        // Nada permitido, ou fora do alcance: nada.
        assert!(snap_to_surface(&m, p, 10.0, &|_| false).is_none());
        assert!(snap_to_surface(&m, p, 0.1, &|_| true).is_none());
        // Exatamente sobre a superfície com distância 0.
        let on = snap_to_surface(&m, Vec3::new(0.0, 0.0, 1.0), 0.0, &|_| true).unwrap();
        assert_eq!(on.face, 1);
    }

    #[test]
    fn snap_hostile_inputs_return_none() {
        let m = Mesh::cube(2.0);
        let all = |_: usize| true;
        assert!(snap_to_surface(&m, Vec3::new(f32::NAN, 0.0, 0.0), 1.0, &all).is_none());
        assert!(snap_to_surface(&m, Vec3::ZERO, f32::NAN, &all).is_none());
        assert!(snap_to_surface(&m, Vec3::ZERO, -1.0, &all).is_none());
        assert!(snap_to_surface(&Mesh::default(), Vec3::ZERO, 1.0, &all).is_none());
        // Infinito como alcance é válido.
        assert!(snap_to_surface(&m, Vec3::splat(50.0), f32::INFINITY, &all).is_some());
    }

    #[test]
    fn snap_ignores_broken_and_degenerate_faces() {
        let mut m = Mesh::cube(2.0);
        m.verts.push(Vertex::new(f32::NAN, 0.0, 0.0));
        let bad = (m.verts.len() - 1) as u32;
        m.push_face(Face::new(vec![0, 1, bad]));
        m.push_face(Face::new(vec![0, 0, 0]));
        m.push_face(Face::new(vec![0, 1, 999]));
        let hit = snap_to_surface(&m, Vec3::new(0.0, 0.0, 1.2), 5.0, &|_| true).unwrap();
        assert_eq!(hit.face, 1);
        let all = project_path_onto_surface(&m, &[Vec3::new(0.0, 0.0, 1.2)], 5.0, &|_| true);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].face, 1);
    }

    #[test]
    fn project_matches_linear_snap_and_collapses_duplicates() {
        let m = grid_mesh(8, 4.0);
        let pts: Vec<Vec3> = (0..40)
            .map(|i| {
                Vec3::new(
                    0.1 + i as f32 * 0.09,
                    0.3 - 0.01 * i as f32,
                    0.7 + (i as f32 * 0.2).sin(),
                )
            })
            .collect();
        let hits = project_path_onto_surface(&m, &pts, 5.0, &|_| true);
        assert_eq!(hits.len(), pts.len());
        for (h, p) in hits.iter().zip(&pts) {
            let lin = snap_to_surface(&m, *p, 5.0, &|_| true).unwrap();
            assert_eq!(h.face, lin.face);
            assert!(h.position.distance(lin.position) < 1e-5);
        }
        // Amostras repetidas colapsam; as que perdem a superfície são descartadas.
        let rep = [pts[0], pts[0], pts[0], Vec3::new(0.0, 100.0, 0.0), pts[1]];
        let hits = project_path_onto_surface(&m, &rep, 1.0, &|_| true);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn project_path_is_deterministic_on_ties() {
        // Ponto exatamente sobre a aresta compartilhada de dois quads: sempre a menor face.
        let m = Mesh::cube(2.0);
        let p = Vec3::new(1.5, 1.0, 1.0); // fora da quina z=+1,y=... aresta entre faces
        let a = project_path_onto_surface(&m, &[p], 5.0, &|_| true);
        let b = project_path_onto_surface(&m, &[p], 5.0, &|_| true);
        assert_eq!(a, b);
        let lin = snap_to_surface(&m, p, 5.0, &|_| true).unwrap();
        assert_eq!(a[0].face, lin.face);
    }

    #[test]
    fn project_hostile_inputs() {
        let m = Mesh::cube(2.0);
        let all = |_: usize| true;
        assert!(project_path_onto_surface(&m, &[], 1.0, &all).is_empty());
        let nan = [Vec3::splat(f32::NAN)];
        assert!(project_path_onto_surface(&m, &nan, 1.0, &all).is_empty());
        assert!(project_path_onto_surface(&m, &[Vec3::ZERO], f32::NAN, &all).is_empty());
        assert!(project_path_onto_surface(&Mesh::default(), &[Vec3::ZERO], 1.0, &all).is_empty());
        assert!(project_path_onto_surface(&m, &[Vec3::ZERO], 1.0, &|_| false).is_empty());
        // Ponto muito longe com alcance infinito ainda acha a malha.
        let far = [Vec3::new(1.0e9, 0.0, 0.0)];
        let hits = project_path_onto_surface(&m, &far, f32::INFINITY, &all);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn projecting_2000_samples_on_a_5k_face_mesh_is_fast() {
        let m = grid_mesh(50, 10.0); // 5000 triângulos
        assert_eq!(m.faces.len(), 5000);
        let pts: Vec<Vec3> = (0..2000)
            .map(|i| {
                let t = i as f32 / 2000.0;
                Vec3::new(
                    5.0 + 4.0 * (t * 20.0).cos(),
                    0.2,
                    5.0 + 4.0 * (t * 20.0).sin(),
                )
            })
            .collect();
        let start = std::time::Instant::now();
        let hits = project_path_onto_surface(&m, &pts, 1.0, &|_| true);
        let took = start.elapsed();
        assert_eq!(hits.len(), 2000);
        assert!(took.as_secs_f32() < 1.0, "demorou {took:?}");
    }

    // ---- pintura ----

    fn cube_state() -> AppState {
        let mut state = AppState::new("en");
        state.paint_color = [1.0, 0.0, 0.0];
        state
    }

    fn red_brush(size: f32) -> BrushSettings {
        BrushSettings {
            kind: BrushType::Pixel,
            size_px: size,
            ..Default::default()
        }
    }

    /// Faces com algum texel vermelho na textura ativa.
    fn painted_faces(state: &AppState) -> Vec<usize> {
        let asset = state.project.active().unwrap();
        let Some(tex) = asset.texture.as_ref() else {
            return Vec::new();
        };
        (0..asset.mesh.faces.len())
            .filter(|&f| {
                let mask = asset.mesh.uv_coverage_mask([f], tex.w, tex.h, 0.0);
                (0..tex.w * tex.h).any(|i| {
                    mask.texels[i as usize] != 0
                        && tex.pixels[i as usize * 4..i as usize * 4 + 3] == [255, 0, 0]
                })
            })
            .collect()
    }

    fn history(state: &AppState) -> usize {
        state.project.history_metrics().history_entries
    }

    /// Face do cubo mais voltada para `dir` e metade do lado.
    fn face_toward(state: &AppState, dir: Vec3) -> usize {
        let mesh = state.project.active_mesh().unwrap();
        (0..mesh.faces.len())
            .max_by(|&a, &b| {
                mesh.face_normal(a)
                    .dot(dir)
                    .partial_cmp(&mesh.face_normal(b).dot(dir))
                    .unwrap()
            })
            .unwrap()
    }

    fn cube_half(state: &AppState) -> f32 {
        let mesh = state.project.active_mesh().unwrap();
        mesh.verts
            .iter()
            .map(|v| v.pos[0].abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn path_over_a_cube_edge_paints_both_faces_and_never_the_opposite() {
        let mut state = cube_state();
        let h = cube_half(&state);
        let (front, top) = (face_toward(&state, Vec3::Z), face_toward(&state, Vec3::Y));
        let back = face_toward(&state, -Vec3::Z);
        let bottom = face_toward(&state, -Vec3::Y);
        // Da frente (z=+h) para o topo (y=+h), cruzando a aresta.
        let nodes = [
            Vec3::new(0.0, 0.0, h),
            Vec3::new(0.0, h, h),
            Vec3::new(0.0, h, 0.0),
        ];
        let before = history(&state);
        let n = PaintModule::paint_surface_path(
            &mut state,
            &nodes,
            false,
            false,
            red_brush(24.0),
            false,
        );
        assert!(n > 2, "dabs: {n}");
        let faces = painted_faces(&state);
        assert!(faces.contains(&front), "frente não pintada: {faces:?}");
        assert!(faces.contains(&top), "topo não pintado: {faces:?}");
        assert!(!faces.contains(&back), "face oposta pintada");
        assert!(!faces.contains(&bottom), "face oposta pintada");
        assert_eq!(history(&state), before + 1, "um Undo por caminho");
    }

    #[test]
    fn path_is_one_undo_entry_and_undo_restores_the_texture() {
        let mut state = cube_state();
        let h = cube_half(&state);
        let nodes = [
            Vec3::new(-h * 0.5, 0.0, h),
            Vec3::new(0.0, h * 0.3, h),
            Vec3::new(h * 0.5, 0.0, h),
        ];
        let before = history(&state);
        let n = PaintModule::paint_surface_path(
            &mut state,
            &nodes,
            false,
            true,
            red_brush(20.0),
            false,
        );
        assert!(n >= 3);
        assert_eq!(history(&state), before + 1);
        assert!(
            state.session.tools.paint_stroke.is_none(),
            "traço deve fechar"
        );
        assert!(!painted_faces(&state).is_empty());
        assert!(state.undo());
        assert!(
            painted_faces(&state).is_empty(),
            "Undo desfaz o caminho inteiro"
        );
    }

    #[test]
    fn path_reuses_a_stroke_opened_by_the_caller() {
        let mut state = cube_state();
        let h = cube_half(&state);
        state.begin_paint_stroke();
        let before = history(&state);
        let a = Vec3::new(-h * 0.5, 0.0, h);
        let b = Vec3::new(h * 0.5, 0.0, h);
        assert!(PaintModule::paint_surface_line(&mut state, a, b, red_brush(20.0), false) > 0);
        assert!(
            state.session.tools.paint_stroke.is_some(),
            "não fecha o traço alheio"
        );
        assert_eq!(history(&state), before);
        state.finish_paint_stroke(false);
        assert_eq!(history(&state), before + 1);
    }

    #[test]
    fn selection_restriction_is_respected() {
        let mut state = cube_state();
        let h = cube_half(&state);
        let (front, top) = (face_toward(&state, Vec3::Z), face_toward(&state, Vec3::Y));
        state.session.selection.faces = vec![front];
        if let Some(m) = state.project.active_mesh_mut() {
            m.faces[front].selected = true;
        }
        let nodes = [
            Vec3::new(0.0, 0.0, h),
            Vec3::new(0.0, h, h),
            Vec3::new(0.0, h, 0.0),
        ];
        let n = PaintModule::paint_surface_path(
            &mut state,
            &nodes,
            false,
            false,
            red_brush(24.0),
            true,
        );
        assert!(n > 0);
        let faces = painted_faces(&state);
        assert!(faces.contains(&front));
        assert!(
            !faces.contains(&top),
            "topo não selecionado não pode pintar: {faces:?}"
        );
        // Isolamento ligado e nada selecionado: não pinta nada.
        let mut empty = cube_state();
        empty.session.tools.paint_isolate_selection = true;
        let n = PaintModule::paint_surface_path(
            &mut empty,
            &nodes,
            false,
            false,
            red_brush(24.0),
            false,
        );
        assert_eq!(n, 0);
        assert!(painted_faces(&empty).is_empty());
    }

    #[test]
    fn forced_isolation_does_not_leak_into_the_session() {
        let mut state = cube_state();
        let h = cube_half(&state);
        let front = face_toward(&state, Vec3::Z);
        state.session.selection.faces = vec![front];
        let nodes = [Vec3::new(-0.2 * h, 0.0, h), Vec3::new(0.2 * h, 0.0, h)];
        PaintModule::paint_surface_path(&mut state, &nodes, false, false, red_brush(20.0), true);
        assert!(!state.session.tools.paint_isolate_selection);
        assert!(state.session.tools.paint_restriction.is_none());
        assert!(state.session.tools.paint_stroke.is_none());
    }

    #[test]
    fn first_face_lock_keeps_the_path_on_its_first_face() {
        let mut state = cube_state();
        state.session.tools.brush_lock = petunia_core::BrushLock::FirstFace;
        let h = cube_half(&state);
        let (front, top) = (face_toward(&state, Vec3::Z), face_toward(&state, Vec3::Y));
        let nodes = [
            Vec3::new(0.0, 0.0, h),
            Vec3::new(0.0, h, h),
            Vec3::new(0.0, h, 0.0),
        ];
        assert!(
            PaintModule::paint_surface_path(
                &mut state,
                &nodes,
                false,
                false,
                red_brush(24.0),
                false
            ) > 0
        );
        let faces = painted_faces(&state);
        assert!(faces.contains(&front));
        assert!(!faces.contains(&top), "trava na primeira face: {faces:?}");
    }

    #[test]
    fn straight_line_paints_between_two_surface_points() {
        let mut state = cube_state();
        let h = cube_half(&state);
        let a = Vec3::new(-h * 0.8, 0.0, h);
        let b = Vec3::new(h * 0.8, 0.0, h);
        let n = PaintModule::paint_surface_line(&mut state, a, b, red_brush(16.0), false);
        assert!(n >= 2);
        assert!(painted_faces(&state).contains(&face_toward(&state, Vec3::Z)));
    }

    #[test]
    fn path_paint_hostile_inputs_paint_nothing_and_leave_no_stroke() {
        let mut state = cube_state();
        let before = history(&state);
        let b = red_brush(16.0);
        assert_eq!(
            PaintModule::paint_surface_path(&mut state, &[], false, true, b, false),
            0
        );
        let nan = [Vec3::splat(f32::NAN), Vec3::splat(f32::INFINITY)];
        assert_eq!(
            PaintModule::paint_surface_path(&mut state, &nan, true, true, b, false),
            0
        );
        let shape = BrushSettings {
            kind: BrushType::Line,
            ..b
        };
        let nodes = [Vec3::ZERO, Vec3::X];
        assert_eq!(
            PaintModule::paint_surface_path(&mut state, &nodes, false, false, shape, false),
            0
        );
        let wild = BrushSettings {
            size_px: f32::NAN,
            spacing: f32::NAN,
            ..b
        };
        // Pincel com NaN é saneado; não entra em pânico.
        let _ = PaintModule::paint_surface_path(&mut state, &nodes, false, false, wild, false);
        assert!(state.session.tools.paint_stroke.is_none());
        assert!(history(&state) <= before + 1);
        // Ponto único: um dab.
        let h = cube_half(&state);
        let one = [Vec3::new(0.0, 0.0, h)];
        assert_eq!(
            PaintModule::paint_surface_path(&mut state, &one, false, false, b, false),
            1
        );
    }

    #[test]
    fn path_paint_is_deterministic() {
        let run = || {
            let mut state = cube_state();
            let h = cube_half(&state);
            let nodes = [
                Vec3::new(-h * 0.5, -h * 0.5, h),
                Vec3::new(0.0, h * 0.5, h),
                Vec3::new(h * 0.5, -h * 0.2, h),
            ];
            let n = PaintModule::paint_surface_path(
                &mut state,
                &nodes,
                false,
                true,
                red_brush(18.0),
                false,
            );
            let px = state
                .project
                .active()
                .unwrap()
                .texture
                .as_ref()
                .unwrap()
                .pixels
                .clone();
            (n, px)
        };
        assert_eq!(run(), run());
    }

    // ---- pixel perfect / Bresenham ----

    #[test]
    fn pixel_perfect_removes_l_corners_of_a_staircase() {
        let stair = [(0, 0), (1, 0), (1, 1), (2, 1), (2, 2), (3, 2), (3, 3)];
        assert_eq!(pixel_perfect(&stair), vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
        // Um único L.
        assert_eq!(
            pixel_perfect(&[(0, 0), (0, 1), (1, 1)]),
            vec![(0, 0), (1, 1)]
        );
        assert_eq!(
            pixel_perfect(&[(5, 5), (4, 5), (4, 4)]),
            vec![(5, 5), (4, 4)]
        );
    }

    #[test]
    fn pixel_perfect_changes_nothing_but_l_corners() {
        let straight = [(0, 0), (1, 0), (2, 0), (3, 0)];
        assert_eq!(pixel_perfect(&straight), straight.to_vec());
        let diag = [(0, 0), (1, 1), (2, 2)];
        assert_eq!(pixel_perfect(&diag), diag.to_vec());
        // Curva em U (volta): passos antiparalelos não são L.
        let u = [(0, 0), (1, 0), (1, 1), (0, 1)];
        // (1,0) é L (0,0)->(1,0)->(1,1): removido; (1,1)->(0,1) preservado.
        assert_eq!(pixel_perfect(&u), vec![(0, 0), (1, 1), (0, 1)]);
        // Saltos e repetidos ficam.
        let jumps = [(0, 0), (5, 5), (5, 5), (6, 5)];
        assert_eq!(pixel_perfect(&jumps), jumps.to_vec());
        // Vazio, 1 e 2 pontos.
        assert!(pixel_perfect(&[]).is_empty());
        assert_eq!(pixel_perfect(&[(1, 2)]), vec![(1, 2)]);
        assert_eq!(pixel_perfect(&[(1, 2), (1, 3)]), vec![(1, 2), (1, 3)]);
        // Extremos de i32 não estouram.
        let edge = [
            (i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX),
            (i32::MIN, i32::MAX),
        ];
        assert_eq!(pixel_perfect(&edge), edge.to_vec());
    }

    #[test]
    fn pixel_perfect_keeps_first_and_last_and_is_idempotent_on_output() {
        let l = [(0, 0), (1, 0), (1, 1), (1, 2), (2, 2), (2, 3), (3, 3)];
        let once = pixel_perfect(&l);
        assert_eq!(once.first(), l.first());
        assert_eq!(once.last(), l.last());
        assert!(once.len() < l.len());
        assert_eq!(pixel_perfect(&l), once);
    }

    #[test]
    fn bresenham_covers_inclusive_endpoints_in_every_octant() {
        for &(a, b) in &[
            ((0, 0), (5, 2)),
            ((0, 0), (2, 5)),
            ((5, 2), (0, 0)),
            ((0, 0), (-4, -4)),
            ((3, 3), (3, 3)),
            ((0, 0), (0, -6)),
            ((-2, 7), (6, -3)),
        ] {
            let line = line_pixels(a, b);
            assert_eq!(line.first(), Some(&a));
            assert_eq!(line.last(), Some(&b));
            let dx = (b.0 - a.0).abs();
            let dy = (b.1 - a.1).abs();
            assert_eq!(line.len() as i32, dx.max(dy) + 1);
            for w in line.windows(2) {
                assert!((w[1].0 - w[0].0).abs() <= 1 && (w[1].1 - w[0].1).abs() <= 1);
            }
        }
    }

    #[test]
    fn bresenham_rejects_absurd_lengths_and_does_not_overflow() {
        assert!(line_pixels((i32::MIN, 0), (i32::MAX, 0)).is_empty());
        assert!(line_pixels((0, 0), (0, MAX_LINE_PIXELS as i32 + 1)).is_empty());
        assert_eq!(
            line_pixels((0, 0), (MAX_LINE_PIXELS as i32, 0)).len() as i64,
            MAX_LINE_PIXELS + 1
        );
    }

    #[test]
    fn pixel_perfect_cleans_a_bresenham_freehand_wobble() {
        // Traço livre 4-conectado em degraus: vira diagonal pura.
        let mut pts = Vec::new();
        for i in 0..6 {
            pts.push((i, i));
            pts.push((i + 1, i));
        }
        let pp = pixel_perfect(&pts);
        assert!(pp.len() < pts.len());
        for w in pp.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            assert!(dx.max(dy) == 1, "passo inesperado {w:?}");
        }
    }
}
