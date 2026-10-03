//! Layout de UV sem sobreposição e máscaras de cobertura (P3D-062, P3D-132).
//!
//! `project_planar` projeta cada face no plano do eixo dominante: duas faces
//! opostas (frente/trás, topo/base) caem exatamente no mesmo retângulo da
//! textura. Pintar uma delas pinta a outra. Este módulo entrega:
//!
//! - [`Mesh::layout_uv_charts`]: agrupa faces em charts por normal, projeta cada
//!   chart no seu plano médio e empacota tudo em `0..1` com densidade uniforme
//!   e sem sobreposição (determinístico, preserva quads);
//! - [`Mesh::uv_overlap_ratio`]: fração dos texels cobertos por mais de uma face;
//! - [`Mesh::uv_coverage_mask`]: rasteriza as UVs de um conjunto de faces num
//!   mapa de texels permitidos — a base da restrição de pintura por face,
//!   seleção e ilha (P3D-132).

use std::collections::{HashMap, HashSet};

use glam::{Vec2, Vec3};

use super::{Mesh, edge_key};

/// Ângulo máximo (graus) entre a normal de uma face e a normal média do chart.
pub const CHART_ANGLE_DEG: f32 = 40.0;

/// Mapa de texels de uma textura `w × h` (`1` = permitido).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageMask {
    pub w: u32,
    pub h: u32,
    pub texels: Vec<u8>,
}

impl CoverageMask {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            texels: vec![0; (w as usize) * (h as usize)],
        }
    }

    #[inline]
    pub fn allows(&self, x: u32, y: u32) -> bool {
        x < self.w && y < self.h && self.texels[(y * self.w + x) as usize] != 0
    }

    pub fn covered(&self) -> usize {
        self.texels.iter().filter(|t| **t != 0).count()
    }
}

struct Chart {
    faces: Vec<usize>,
    normal: Vec3,
}

impl Mesh {
    fn face_adjacency(&self) -> Vec<Vec<usize>> {
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            for k in 0..n {
                edge_faces
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_default()
                    .push(fi);
            }
        }
        let mut adj = vec![Vec::new(); self.faces.len()];
        for (edge, faces) in &edge_faces {
            if faces.len() == 2 && !self.uv_seams.contains(edge) {
                adj[faces[0]].push(faces[1]);
                adj[faces[1]].push(faces[0]);
            }
        }
        for list in &mut adj {
            list.sort_unstable();
        }
        adj
    }

    fn face_area(&self, fi: usize) -> f32 {
        let face = &self.faces[fi];
        if face.verts.len() < 3 {
            return 0.0;
        }
        let a = self.verts[face.verts[0] as usize].vec();
        let mut area = 0.0;
        for k in 1..face.verts.len() - 1 {
            let b = self.verts[face.verts[k] as usize].vec();
            let c = self.verts[face.verts[k + 1] as usize].vec();
            area += (b - a).cross(c - a).length() * 0.5;
        }
        area
    }

    fn build_charts(&self, angle_deg: f32) -> Vec<Chart> {
        let adj = self.face_adjacency();
        let cos_limit = angle_deg.to_radians().cos();
        let normals: Vec<Vec3> = (0..self.faces.len())
            .map(|fi| self.face_normal(fi).normalize_or_zero())
            .collect();
        let areas: Vec<f32> = (0..self.faces.len()).map(|fi| self.face_area(fi)).collect();
        let mut seen = vec![false; self.faces.len()];
        let mut charts = Vec::new();
        for start in 0..self.faces.len() {
            if seen[start] {
                continue;
            }
            seen[start] = true;
            let mut faces = vec![start];
            let mut acc = normals[start] * areas[start].max(1e-9);
            let mut queue = std::collections::VecDeque::from([start]);
            while let Some(fi) = queue.pop_front() {
                for &nb in &adj[fi] {
                    if seen[nb] {
                        continue;
                    }
                    let chart_normal = acc.normalize_or_zero();
                    if normals[nb].dot(chart_normal) >= cos_limit {
                        seen[nb] = true;
                        faces.push(nb);
                        acc += normals[nb] * areas[nb].max(1e-9);
                        queue.push_back(nb);
                    }
                }
            }
            charts.push(Chart {
                faces,
                normal: acc.normalize_or_zero(),
            });
        }
        charts
    }

    /// Base ortonormal (direita, cima) de um plano com normal `n`, estável.
    fn chart_basis(n: Vec3) -> (Vec3, Vec3) {
        let n = if n.length_squared() < 1e-12 {
            Vec3::Y
        } else {
            n
        };
        let reference = if n.y.abs() > 0.9 { Vec3::Z } else { Vec3::Y };
        let up = (reference - n * reference.dot(n)).normalize_or_zero();
        let right = up.cross(n).normalize_or_zero();
        (right, up)
    }

    /// Reorganiza as UVs em charts sem sobreposição, com densidade uniforme.
    ///
    /// Cantos fixados (`uv_pinned`) mantêm o valor. Retorna o número de charts.
    pub fn layout_uv_charts(&mut self, padding: f32) -> usize {
        if self.faces.is_empty() || self.verts.is_empty() {
            return 0;
        }
        let padding = padding.clamp(0.0, 0.1);
        let charts = self.build_charts(CHART_ANGLE_DEG);

        // UVs locais (em unidades de mundo) por chart.
        struct Local {
            corners: Vec<(usize, Vec<Vec2>)>,
            min: Vec2,
            size: Vec2,
        }
        let mut locals: Vec<Local> = Vec::with_capacity(charts.len());
        for chart in &charts {
            let (right, up) = Self::chart_basis(chart.normal);
            let mut min = Vec2::splat(f32::MAX);
            let mut max = Vec2::splat(f32::MIN);
            let mut corners = Vec::with_capacity(chart.faces.len());
            for &fi in &chart.faces {
                let pts: Vec<Vec2> = self.faces[fi]
                    .verts
                    .iter()
                    .map(|&vi| {
                        let p = self.verts[vi as usize].vec();
                        Vec2::new(p.dot(right), p.dot(up))
                    })
                    .collect();
                for p in &pts {
                    min = min.min(*p);
                    max = max.max(*p);
                }
                corners.push((fi, pts));
            }
            locals.push(Local {
                corners,
                min,
                size: (max - min).max(Vec2::splat(1e-5)),
            });
        }

        // Empacotamento por prateleiras; procura a maior escala que cabe.
        let order: Vec<usize> = {
            let mut o: Vec<usize> = (0..locals.len()).collect();
            o.sort_by(|&a, &b| {
                locals[b]
                    .size
                    .y
                    .partial_cmp(&locals[a].size.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.cmp(&b))
            });
            o
        };
        let try_pack = |scale: f32| -> Option<Vec<Vec2>> {
            let mut origins = vec![Vec2::ZERO; locals.len()];
            let (mut x, mut y, mut row_h) = (padding, padding, 0.0f32);
            for &i in &order {
                let w = locals[i].size.x * scale;
                let h = locals[i].size.y * scale;
                if x + w + padding > 1.0 {
                    x = padding;
                    y += row_h + padding;
                    row_h = 0.0;
                }
                if x + w + padding > 1.0 || y + h + padding > 1.0 {
                    return None;
                }
                origins[i] = Vec2::new(x, y);
                x += w + padding;
                row_h = row_h.max(h);
            }
            Some(origins)
        };
        let total: f32 = locals.iter().map(|l| l.size.x * l.size.y).sum();
        let mut scale = (1.0 / total.max(1e-9)).sqrt();
        let mut origins = None;
        for _ in 0..64 {
            if let Some(o) = try_pack(scale) {
                origins = Some(o);
                break;
            }
            scale *= 0.92;
        }
        let Some(mut origins) = origins else {
            return 0;
        };
        // Ajuste final: amplia o layout até ocupar a área útil (`padding..1-padding`)
        // e o centraliza, para que o maior eixo use a textura inteira.
        let (mut max_x, mut max_y) = (padding, padding);
        for (i, l) in locals.iter().enumerate() {
            max_x = max_x.max(origins[i].x + l.size.x * scale);
            max_y = max_y.max(origins[i].y + l.size.y * scale);
        }
        let used = (max_x - padding).max(max_y - padding).max(1e-6);
        let fit = (1.0 - 2.0 * padding) / used;
        let offset = Vec2::new(
            ((1.0 - 2.0 * padding) - (max_x - padding) * fit) * 0.5,
            ((1.0 - 2.0 * padding) - (max_y - padding) * fit) * 0.5,
        );
        for o in &mut origins {
            *o = Vec2::splat(padding) + offset + (*o - Vec2::splat(padding)) * fit;
        }
        let scale = scale * fit;

        for (i, local) in locals.iter().enumerate() {
            for (fi, pts) in &local.corners {
                for (c, p) in pts.iter().enumerate() {
                    if self.uv_pinned.contains(&(*fi, c)) {
                        continue;
                    }
                    let uv = origins[i] + (*p - local.min) * scale;
                    self.faces[*fi].uv[c] = [uv.x, uv.y];
                }
            }
        }
        charts.len()
    }

    /// Fração dos texels cobertos que recebem mais de uma face (`0..=1`).
    ///
    /// Usa uma grade fixa de 128×128; faces degeneradas (UV sem área) contam
    /// como sobrepostas porque todas colapsam no mesmo texel.
    pub fn uv_overlap_ratio(&self) -> f32 {
        const RES: u32 = 128;
        let mut count = vec![0u16; (RES * RES) as usize];
        let mut degenerate = 0usize;
        for fi in 0..self.faces.len() {
            let mut hit = false;
            for [a, b, c] in self.face_triangle_corners(fi) {
                let face = &self.faces[fi];
                let (Some(&ua), Some(&ub), Some(&uc)) =
                    (face.uv.get(a), face.uv.get(b), face.uv.get(c))
                else {
                    continue;
                };
                rasterize_triangle(ua, ub, uc, RES, RES, -0.02, |x, y| {
                    hit = true;
                    let slot = &mut count[(y * RES + x) as usize];
                    *slot = slot.saturating_add(1);
                });
            }
            if !hit {
                degenerate += 1;
            }
        }
        let covered = count.iter().filter(|c| **c > 0).count();
        let overlapped = count.iter().filter(|c| **c > 1).count();
        if covered == 0 {
            return if self.faces.is_empty() { 0.0 } else { 1.0 };
        }
        let ratio = overlapped as f32 / covered as f32;
        if degenerate > 0 {
            ratio.max(degenerate as f32 / self.faces.len() as f32)
        } else {
            ratio
        }
    }

    /// `true` quando pintar numa face afetaria outra (UV sobrepostas ou nulas).
    pub fn uv_needs_layout(&self) -> bool {
        self.uv_overlap_ratio() > 0.02
    }

    /// Rasteriza as UVs das `faces` num mapa de texels `w × h`.
    ///
    /// `dilate_px` alarga cada triângulo (em texels) para cobrir a sangria de
    /// borda usada pela filtragem bilinear. V é invertido (`y = (1 - v) * h`),
    /// igual ao mapeamento UV → pixel do canvas.
    pub fn uv_coverage_mask(
        &self,
        faces: impl IntoIterator<Item = usize>,
        w: u32,
        h: u32,
        dilate_px: f32,
    ) -> CoverageMask {
        let mut mask = CoverageMask::new(w, h);
        if w == 0 || h == 0 {
            return mask;
        }
        let tol = -dilate_px.max(0.0);
        let seen: HashSet<usize> = faces.into_iter().collect();
        let mut ordered: Vec<usize> = seen.into_iter().collect();
        ordered.sort_unstable();
        for fi in ordered {
            let Some(face) = self.faces.get(fi) else {
                continue;
            };
            for [a, b, c] in self.face_triangle_corners(fi) {
                let (Some(&ua), Some(&ub), Some(&uc)) =
                    (face.uv.get(a), face.uv.get(b), face.uv.get(c))
                else {
                    continue;
                };
                rasterize_triangle(ua, ub, uc, w, h, tol, |x, y| {
                    mask.texels[(y * w + x) as usize] = 1;
                });
            }
        }
        mask
    }

    /// Índices das faces cujo UV cobre o texel `(x, y)` de uma textura `w × h`.
    pub fn faces_at_texel(&self, x: u32, y: u32, w: u32, h: u32) -> Vec<usize> {
        let mut out = Vec::new();
        for fi in 0..self.faces.len() {
            let face = &self.faces[fi];
            let mut found = false;
            for [a, b, c] in self.face_triangle_corners(fi) {
                let (Some(&ua), Some(&ub), Some(&uc)) =
                    (face.uv.get(a), face.uv.get(b), face.uv.get(c))
                else {
                    continue;
                };
                rasterize_impl(ua, ub, uc, w, h, 0.0, Some((x, y)), |_, _| found = true);
                if found {
                    break;
                }
            }
            if found {
                out.push(fi);
            }
        }
        out
    }
}

impl Mesh {
    /// Texels de uma face a até `r_world` de `hit` em 3D (pincel esférico).
    ///
    /// Para cada texel dentro da UV da face chama `f(x, y, t)` com a
    /// distância normalizada (`0` = centro, `1` = borda) do texel ao `hit`, a posição
    /// de mundo do texel e o raio efetivo usado (`r_eff`). O raio em texels é derivado da
    /// densidade local de cada triângulo, então o custo é proporcional ao
    /// tamanho do pincel, não da face. `dilate_px` alarga a UV (sangria).
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_face_near(
        &self,
        fi: usize,
        hit: Vec3,
        r_world: f32,
        w: u32,
        h: u32,
        dilate_px: f32,
        mut f: impl FnMut(u32, u32, f32, Vec3, f32),
    ) {
        let Some(face) = self.faces.get(fi) else {
            return;
        };
        if w == 0 || h == 0 || !(r_world.is_finite() && r_world > 0.0) {
            return;
        }
        for [a, b, c] in self.face_triangle_corners(fi) {
            let (Some(&ua), Some(&ub), Some(&uc)) =
                (face.uv.get(a), face.uv.get(b), face.uv.get(c))
            else {
                continue;
            };
            let v0 = self.verts[face.verts[a] as usize].vec();
            let v1 = self.verts[face.verts[b] as usize].vec();
            let v2 = self.verts[face.verts[c] as usize].vec();
            let world_area2 = (v1 - v0).cross(v2 - v0).length();
            if world_area2 < 1e-12 {
                continue;
            }
            let (t0, t1, t2) = (to_texel(ua, w, h), to_texel(ub, w, h), to_texel(uc, w, h));
            if !(t0.is_finite() && t1.is_finite() && t2.is_finite()) {
                continue;
            }
            let area2 = (t1 - t0).perp_dot(t2 - t0);
            if area2.abs() < 1e-9 {
                continue;
            }
            // texels por unidade de mundo neste triângulo
            let density = (area2.abs() / world_area2).sqrt();
            // O dab sempre alcança ao menos o texel sob o ponto de impacto.
            let r_eff = r_world.max(0.75 / density.max(1e-6));
            let cp = closest_point_on_triangle(hit, v0, v1, v2);
            if cp.distance(hit) > r_eff {
                continue;
            }
            let rad = (r_eff * density).ceil() as i64 + 2 + dilate_px.ceil() as i64;
            let bary_cp = barycentric3(cp, v0, v1, v2);
            let center = t0 * bary_cp.x + t1 * bary_cp.y + t2 * bary_cp.z;
            let x0 = ((center.x.floor() as i64) - rad)
                .max(t0.x.min(t1.x).min(t2.x).floor() as i64 - 1)
                .max(0);
            let x1 = ((center.x.floor() as i64) + rad)
                .min(t0.x.max(t1.x).max(t2.x).ceil() as i64 + 1)
                .min(w as i64 - 1);
            let y0 = ((center.y.floor() as i64) - rad)
                .max(t0.y.min(t1.y).min(t2.y).floor() as i64 - 1)
                .max(0);
            let y1 = ((center.y.floor() as i64) + rad)
                .min(t0.y.max(t1.y).max(t2.y).ceil() as i64 + 1)
                .min(h as i64 - 1);
            let sign = area2.signum();
            let edges = [(t0, t1), (t1, t2), (t2, t0)];
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let inside = edges.iter().all(|(s, e)| {
                        let edge = *e - *s;
                        sign * edge.perp_dot(p - *s) / edge.length().max(1e-9) >= -dilate_px
                    });
                    if !inside {
                        continue;
                    }
                    let l0 = (t1 - p).perp_dot(t2 - p) / area2;
                    let l1 = (t2 - p).perp_dot(t0 - p) / area2;
                    let l2 = 1.0 - l0 - l1;
                    let (c0, c1, c2) = (l0.max(0.0), l1.max(0.0), l2.max(0.0));
                    let sum = (c0 + c1 + c2).max(1e-9);
                    let pos = (v0 * c0 + v1 * c1 + v2 * c2) / sum;
                    let dist = pos.distance(hit);
                    if dist <= r_eff {
                        f(x as u32, y as u32, dist / r_eff, pos, r_eff);
                    }
                }
            }
        }
    }
}

impl Mesh {
    /// Visita todos os texels das UVs de uma face (textura `w × h`).
    ///
    /// Para cada texel cujo centro cai dentro de um triângulo UV da face chama
    /// `f(x, y, posição_de_mundo_do_centro_do_texel, normal_da_face)`. Usa a
    /// mesma triangulação, o mesmo teste de borda (independente do winding) e a
    /// mesma conversão UV→texel (V invertido) de [`Mesh::rasterize_face_near`].
    ///
    /// `dilate_px > 0` alarga a UV (sangria): texels fora do triângulo recebem
    /// a posição do ponto mais próximo dentro dele (baricêntricas presas em
    /// `0..1`), então o resultado continua sobre a superfície. Um texel pode
    /// ser visitado mais de uma vez (diagonal de quad, dilatação); quem chama
    /// deve deduplicar se a operação não for idempotente. Face inexistente,
    /// textura vazia, UV/vértice inválido ou triângulo degenerado: não visita
    /// nada.
    pub fn rasterize_face_texels(
        &self,
        face: usize,
        w: u32,
        h: u32,
        dilate_px: f32,
        mut f: impl FnMut(u32, u32, Vec3, Vec3),
    ) {
        let Some(fc) = self.faces.get(face) else {
            return;
        };
        if w == 0 || h == 0 {
            return;
        }
        let dilate = if dilate_px.is_finite() {
            dilate_px.max(0.0)
        } else {
            0.0
        };
        // Já devolve vazio para vértice fora de faixa ou não finito.
        let corners = self.face_triangle_corners(face);
        if corners.is_empty() {
            return;
        }
        let normal = self.face_normal(face);
        let pad = dilate.ceil() as i64 + 1;
        for [a, b, c] in corners {
            let (Some(&ua), Some(&ub), Some(&uc)) = (fc.uv.get(a), fc.uv.get(b), fc.uv.get(c))
            else {
                continue;
            };
            let (Some(&ia), Some(&ib), Some(&ic)) =
                (fc.verts.get(a), fc.verts.get(b), fc.verts.get(c))
            else {
                continue;
            };
            let (Some(va), Some(vb), Some(vc)) = (
                self.verts.get(ia as usize),
                self.verts.get(ib as usize),
                self.verts.get(ic as usize),
            ) else {
                continue;
            };
            let (v0, v1, v2) = (va.vec(), vb.vec(), vc.vec());
            if (v1 - v0).cross(v2 - v0).length() < 1e-12 {
                continue;
            }
            let (t0, t1, t2) = (to_texel(ua, w, h), to_texel(ub, w, h), to_texel(uc, w, h));
            if !(t0.is_finite() && t1.is_finite() && t2.is_finite()) {
                continue;
            }
            let area2 = (t1 - t0).perp_dot(t2 - t0);
            if area2.abs() < 1e-9 || !area2.is_finite() {
                continue;
            }
            let lim_x = w as f32;
            let lim_y = h as f32;
            let x0 = (t0.x.min(t1.x).min(t2.x).floor().clamp(-1.0, lim_x) as i64 - pad).max(0);
            let x1 =
                (t0.x.max(t1.x).max(t2.x).ceil().clamp(-1.0, lim_x) as i64 + pad).min(w as i64 - 1);
            let y0 = (t0.y.min(t1.y).min(t2.y).floor().clamp(-1.0, lim_y) as i64 - pad).max(0);
            let y1 =
                (t0.y.max(t1.y).max(t2.y).ceil().clamp(-1.0, lim_y) as i64 + pad).min(h as i64 - 1);
            let sign = area2.signum();
            let edges = [(t0, t1), (t1, t2), (t2, t0)];
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let inside = edges.iter().all(|(s, e)| {
                        let edge = *e - *s;
                        sign * edge.perp_dot(p - *s) / edge.length().max(1e-9) >= -dilate
                    });
                    if !inside {
                        continue;
                    }
                    let l0 = (t1 - p).perp_dot(t2 - p) / area2;
                    let l1 = (t2 - p).perp_dot(t0 - p) / area2;
                    let l2 = 1.0 - l0 - l1;
                    let (c0, c1, c2) = (l0.max(0.0), l1.max(0.0), l2.max(0.0));
                    let sum = (c0 + c1 + c2).max(1e-9);
                    let pos = (v0 * c0 + v1 * c1 + v2 * c2) / sum;
                    f(x as u32, y as u32, pos, normal);
                }
            }
        }
    }
}

/// Coordenadas baricêntricas de `p` (já no plano) em relação a `a, b, c`.
fn barycentric3(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let denom = (d00 * d11 - d01 * d01).max(1e-20);
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    Vec3::new(1.0 - v - w, v, w)
}

/// Ponto do triângulo mais próximo de `p` (Ericson, *Real-Time Collision Detection*).
pub fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * t;
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// Converte UV em coordenada contínua de texel (V invertido).
#[inline]
fn to_texel(uv: [f32; 2], w: u32, h: u32) -> Vec2 {
    Vec2::new(uv[0] * w as f32, (1.0 - uv[1]) * h as f32)
}

/// Chama `f` para cada texel cujo centro fica dentro do triângulo UV
/// (alargado por `-tol` texels; `tol` negativo = dilatação).
fn rasterize_triangle(
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    w: u32,
    h: u32,
    tol: f32,
    f: impl FnMut(u32, u32),
) {
    rasterize_impl(a, b, c, w, h, tol, None, f);
}

/// Mesmo teste, restrito a um único texel (`only`) quando informado.
#[allow(clippy::too_many_arguments)]
fn rasterize_impl(
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    w: u32,
    h: u32,
    tol: f32,
    only: Option<(u32, u32)>,
    mut f: impl FnMut(u32, u32),
) {
    let pa = to_texel(a, w, h);
    let pb = to_texel(b, w, h);
    let pc = to_texel(c, w, h);
    if !(pa.is_finite() && pb.is_finite() && pc.is_finite()) {
        return;
    }
    let area2 = (pb - pa).perp_dot(pc - pa);
    if area2.abs() < 1e-9 {
        return;
    }
    let (x0, x1, y0, y1) = if let Some((ox, oy)) = only {
        (ox as i64, ox as i64, oy as i64, oy as i64)
    } else {
        let pad = tol.abs().ceil() as i64 + 1;
        (
            (pa.x.min(pb.x).min(pc.x).floor() as i64 - pad).max(0),
            (pa.x.max(pb.x).max(pc.x).ceil() as i64 + pad).min(w as i64 - 1),
            (pa.y.min(pb.y).min(pc.y).floor() as i64 - pad).max(0),
            (pa.y.max(pb.y).max(pc.y).ceil() as i64 + pad).min(h as i64 - 1),
        )
    };
    let sign = area2.signum();
    let edges = [(pa, pb), (pb, pc), (pc, pa)];
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let inside = edges.iter().all(|(s, e)| {
                let edge = *e - *s;
                let len = edge.length().max(1e-9);
                // distância assinada (positiva para dentro) em texels
                sign * edge.perp_dot(p - *s) / len >= tol
            });
            if inside && x >= 0 && y >= 0 && x < w as i64 && y < h as i64 {
                f(x as u32, y as u32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_planar_projection_overlaps_but_chart_layout_does_not() {
        let mut cube = Mesh::cube(2.0);
        cube.project_planar();
        assert!(cube.uv_overlap_ratio() > 0.2, "planar cube must overlap");
        let charts = cube.layout_uv_charts(0.01);
        assert_eq!(charts, 6);
        assert!(
            cube.uv_overlap_ratio() < 0.01,
            "ratio = {}",
            cube.uv_overlap_ratio()
        );
    }

    #[test]
    fn chart_layout_stays_inside_the_unit_square_and_keeps_quads() {
        let mut cube = Mesh::cube(1.0);
        cube.layout_uv_charts(0.01);
        assert!(cube.faces.iter().all(|f| f.verts.len() == 4));
        for face in &cube.faces {
            for uv in &face.uv {
                assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
            }
        }
    }

    #[test]
    fn coverage_mask_of_one_face_excludes_the_others() {
        let mut cube = Mesh::cube(1.0);
        cube.layout_uv_charts(0.02);
        let one = cube.uv_coverage_mask([0], 128, 128, 0.0);
        let all = cube.uv_coverage_mask(0..cube.faces.len(), 128, 128, 0.0);
        assert!(one.covered() > 0);
        assert!(one.covered() * 4 < all.covered());
    }

    #[test]
    fn faces_at_texel_finds_only_the_owner_after_layout() {
        let mut cube = Mesh::cube(1.0);
        cube.layout_uv_charts(0.02);
        let uv = {
            let f = &cube.faces[2];
            let s: [f32; 2] =
                f.uv.iter()
                    .fold([0.0, 0.0], |a, u| [a[0] + u[0], a[1] + u[1]]);
            [s[0] / 4.0, s[1] / 4.0]
        };
        let (x, y) = ((uv[0] * 128.0) as u32, ((1.0 - uv[1]) * 128.0) as u32);
        assert_eq!(cube.faces_at_texel(x, y, 128, 128), vec![2]);
    }

    #[test]
    fn spherical_dab_on_one_face_never_reaches_the_opposite_face() {
        let mut cube = Mesh::cube(2.0);
        cube.layout_uv_charts(0.02);
        // centro da face 1 (z = +1): pincel de raio 0.5 não alcança a face 0 (z = -1)
        let hit = Vec3::new(0.0, 0.0, 1.0);
        let mut on_face1 = 0;
        cube.rasterize_face_near(1, hit, 0.5, 128, 128, 0.0, |_, _, _, _, _| on_face1 += 1);
        let mut on_face0 = 0;
        cube.rasterize_face_near(0, hit, 0.5, 128, 128, 0.0, |_, _, _, _, _| on_face0 += 1);
        assert!(on_face1 > 0);
        assert_eq!(on_face0, 0);
    }

    #[test]
    fn closest_point_clamps_to_triangle() {
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let p = closest_point_on_triangle(Vec3::new(0.2, 0.2, 5.0), a, b, c);
        assert!(p.distance(Vec3::new(0.2, 0.2, 0.0)) < 1e-5);
        assert_eq!(
            closest_point_on_triangle(Vec3::new(-3.0, -3.0, 0.0), a, b, c),
            a
        );
    }

    #[test]
    fn layout_is_deterministic() {
        let mut a = Mesh::cube(1.0);
        let mut b = Mesh::cube(1.0);
        a.layout_uv_charts(0.01);
        b.layout_uv_charts(0.01);
        assert_eq!(
            a.faces.iter().map(|f| f.uv.clone()).collect::<Vec<_>>(),
            b.faces.iter().map(|f| f.uv.clone()).collect::<Vec<_>>()
        );
    }

    /// Quad no plano z=0 de (0,0) a (2,2), UV cobrindo `0..1`.
    fn unit_uv_quad() -> Mesh {
        let mut m = Mesh {
            verts: vec![
                crate::Vertex::new(0.0, 0.0, 0.0),
                crate::Vertex::new(2.0, 0.0, 0.0),
                crate::Vertex::new(2.0, 2.0, 0.0),
                crate::Vertex::new(0.0, 2.0, 0.0),
            ],
            faces: Vec::new(),
            selected_edges: Default::default(),
            uv_seams: Default::default(),
            uv_pinned: Default::default(),
        };
        m.faces.push(crate::Face::with_uv(
            vec![0, 1, 2, 3],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ));
        m
    }

    #[test]
    fn face_texels_cover_the_whole_uv_square_with_world_positions() {
        let m = unit_uv_quad();
        let mut seen = HashSet::new();
        m.rasterize_face_texels(0, 8, 8, 0.0, |x, y, pos, n| {
            seen.insert((x, y));
            // texel (x, y): u = (x+.5)/8, v = 1-(y+.5)/8; mundo = uv * 2
            let ex = (x as f32 + 0.5) / 8.0 * 2.0;
            let ey = (1.0 - (y as f32 + 0.5) / 8.0) * 2.0;
            assert!(
                (pos.x - ex).abs() < 1e-4 && (pos.y - ey).abs() < 1e-4,
                "{pos:?}"
            );
            assert!(pos.z.abs() < 1e-6);
            assert!(n.distance(Vec3::Z) < 1e-5 || n.distance(-Vec3::Z) < 1e-5);
        });
        assert_eq!(seen.len(), 64);
    }

    #[test]
    fn face_texels_dilation_adds_a_border_clamped_to_the_surface() {
        let m = unit_uv_quad();
        let mut plain = HashSet::new();
        m.rasterize_face_texels(0, 8, 8, 0.0, |x, y, _, _| {
            plain.insert((x, y));
        });
        let mut dilated = HashSet::new();
        m.rasterize_face_texels(0, 8, 8, 1.0, |x, y, pos, _| {
            dilated.insert((x, y));
            assert!(pos.x >= -1e-4 && pos.x <= 2.0001 && pos.y >= -1e-4 && pos.y <= 2.0001);
        });
        // a UV preenche a textura inteira: não há onde dilatar
        assert_eq!(plain, dilated);

        // metade da textura: dilatar acrescenta uma coluna
        let mut half = unit_uv_quad();
        half.faces[0].uv = vec![[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]];
        let mut a = 0;
        half.rasterize_face_texels(0, 8, 8, 0.0, |_, _, _, _| a += 1);
        let mut b = 0;
        half.rasterize_face_texels(0, 8, 8, 1.0, |_, _, _, _| b += 1);
        assert!(b > a, "{a} vs {b}");
    }

    #[test]
    fn face_texels_ignore_invalid_input() {
        let mut m = unit_uv_quad();
        let mut hits = 0;
        m.rasterize_face_texels(9, 8, 8, 0.0, |_, _, _, _| hits += 1);
        m.rasterize_face_texels(0, 0, 8, 0.0, |_, _, _, _| hits += 1);
        m.rasterize_face_texels(0, 8, 0, 0.0, |_, _, _, _| hits += 1);
        assert_eq!(hits, 0);
        // dilatação NaN/negativa vira 0 e não entra em pânico
        m.rasterize_face_texels(0, 8, 8, f32::NAN, |_, _, _, _| hits += 1);
        m.rasterize_face_texels(0, 8, 8, -3.0, |_, _, _, _| hits += 1);
        // 2 chamadas × 64 texels + os texels da diagonal visitados duas vezes
        let valid_hits = hits;
        assert!(valid_hits >= 128 && valid_hits <= 160, "{valid_hits}");
        // UV NaN e vértice NaN: nada
        let mut bad_uv = m.clone();
        bad_uv.faces[0].uv = vec![[f32::NAN, 0.0]; 4];
        let mut bad_vert = m.clone();
        bad_vert.verts[0].pos[0] = f32::NAN;
        let mut bad_idx = m.clone();
        bad_idx.faces[0].verts[1] = 99;
        for mesh in [bad_uv, bad_vert, bad_idx] {
            mesh.rasterize_face_texels(0, 8, 8, 1.0, |_, _, _, _| hits += 1);
        }
        // UV degenerada
        m.faces[0].uv = vec![[0.3, 0.3]; 4];
        m.rasterize_face_texels(0, 8, 8, 0.0, |_, _, _, _| hits += 1);
        assert_eq!(hits, valid_hits);
    }

    #[test]
    fn face_texels_work_with_either_uv_winding() {
        let mut m = unit_uv_quad();
        m.faces[0].uv.reverse();
        m.faces[0].verts.reverse();
        let mut uniq = HashSet::new();
        m.rasterize_face_texels(0, 8, 8, 0.0, |x, y, _, _| {
            uniq.insert((x, y));
        });
        assert_eq!(uniq.len(), 64);
    }
}
