//! UV islands, packing, texel density and diagnostics (UV0 only).

use std::collections::{HashMap, HashSet, VecDeque};

use super::{Mesh, edge_key};

/// One connected UV chart.
#[derive(Clone, Debug)]
pub struct UvIsland {
    pub faces: Vec<usize>,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl UvIsland {
    pub fn area(&self) -> f32 {
        let w = (self.max[0] - self.min[0]).max(0.0);
        let h = (self.max[1] - self.min[1]).max(0.0);
        w * h
    }

    pub fn center(&self) -> [f32; 2] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
        ]
    }
}

/// UV Health (website/docs/07-uv/architecture-decision.md, "UV Health").
///
/// Somente diagnóstico: UV fora de `0..1` e overlap não são erros (mirrored e
/// stacked UV são válidos). Stretch é **Area Stretch** por triângulo derivado,
/// normalizado pela densidade média da malha: `max(r, 1/r) - 1`, em que `r` é a
/// razão área-UV/área-3D do triângulo dividida pela razão global. `0` = sem
/// distorção; `1` = o dobro (ou metade) da área esperada.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UvDiagnostics {
    pub island_count: usize,
    /// Pares de ilhas cujas caixas UV se cruzam (indício barato; ver `overlap_ratio`).
    pub overlapping_islands: usize,
    /// Faces sem área UV (ou com UV incompleta).
    pub zero_area_faces: usize,
    pub out_of_range_corners: usize,
    /// Média do Area Stretch ponderada pela área 3D.
    pub mean_stretch: f32,
    pub max_stretch: f32,
    /// Fração dos texels cobertos que recebem mais de uma face (`0..=1`).
    pub overlap_ratio: f32,
    /// Edges com UV dividida mas sem seam marcada ("UV split without seam").
    pub splits_without_seam: usize,
    /// Seams internas cuja UV continua contínua ("Seam without UV split").
    pub seams_without_split: usize,
}

impl Mesh {
    pub fn mark_seam(&mut self, a: u32, b: u32) {
        self.uv_seams.insert(edge_key(a, b));
    }

    pub fn clear_seam(&mut self, a: u32, b: u32) {
        self.uv_seams.remove(&edge_key(a, b));
    }

    pub fn toggle_seam(&mut self, a: u32, b: u32) {
        let k = edge_key(a, b);
        if !self.uv_seams.remove(&k) {
            self.uv_seams.insert(k);
        }
    }

    pub fn pin_uv(&mut self, face_idx: usize, corner_idx: usize) {
        self.uv_pinned.insert((face_idx, corner_idx));
    }

    pub fn unpin_uv(&mut self, face_idx: usize, corner_idx: usize) {
        self.uv_pinned.remove(&(face_idx, corner_idx));
    }

    pub fn toggle_pin_uv(&mut self, face_idx: usize, corner_idx: usize) {
        if !self.uv_pinned.remove(&(face_idx, corner_idx)) {
            self.uv_pinned.insert((face_idx, corner_idx));
        }
    }

    pub fn is_uv_pinned(&self, face_idx: usize, corner_idx: usize) -> bool {
        self.uv_pinned.contains(&(face_idx, corner_idx))
    }

    pub fn clear_all_pins(&mut self) {
        self.uv_pinned.clear();
    }

    pub fn pin_selected_faces_uv(&mut self, uv_selected: &HashSet<usize>) {
        let all = uv_selected.is_empty();
        for (fi, face) in self.faces.iter().enumerate() {
            if all || uv_selected.contains(&fi) {
                for c in 0..face.uv.len() {
                    self.uv_pinned.insert((fi, c));
                }
            }
        }
    }

    pub fn unpin_selected_faces_uv(&mut self, uv_selected: &HashSet<usize>) {
        let all = uv_selected.is_empty();
        for (fi, face) in self.faces.iter().enumerate() {
            if all || uv_selected.contains(&fi) {
                for c in 0..face.uv.len() {
                    self.uv_pinned.remove(&(fi, c));
                }
            }
        }
    }

    pub fn toggle_pin_selected_faces_uv(&mut self, uv_selected: &HashSet<usize>) -> bool {
        let all = uv_selected.is_empty();
        let mut any_pinned = false;
        for (fi, face) in self.faces.iter().enumerate() {
            if all || uv_selected.contains(&fi) {
                for c in 0..face.uv.len() {
                    if self.uv_pinned.contains(&(fi, c)) {
                        any_pinned = true;
                        break;
                    }
                }
            }
            if any_pinned {
                break;
            }
        }

        if any_pinned {
            self.unpin_selected_faces_uv(uv_selected);
        } else {
            self.pin_selected_faces_uv(uv_selected);
        }
        true
    }

    /// Edges 3D (`edge_key`) em que faces vizinhas não concordam na UV dos dois
    /// extremos — onde a UV de fato se divide. Consulta pura: serve para derivar
    /// seams de chart (Auto Unwrap) e para o diagnóstico "UV split without seam".
    /// Faces com UV incompleta são ignoradas.
    pub fn uv_split_edges(&self) -> HashSet<(u32, u32)> {
        const EPS: f32 = 1.0e-5;
        // edge_key → UVs (no extremo menor, no extremo maior) de cada face.
        let mut sides: HashMap<(u32, u32), Vec<([f32; 2], [f32; 2])>> = HashMap::new();
        for face in &self.faces {
            let n = face.verts.len();
            if n < 3 || face.uv.len() != n {
                continue;
            }
            for k in 0..n {
                let (a, b) = (face.verts[k], face.verts[(k + 1) % n]);
                let (uv_a, uv_b) = (face.uv[k], face.uv[(k + 1) % n]);
                let side = if a <= b { (uv_a, uv_b) } else { (uv_b, uv_a) };
                sides.entry(edge_key(a, b)).or_default().push(side);
            }
        }
        let differs =
            |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).abs() > EPS || (p[1] - q[1]).abs() > EPS;
        sides
            .into_iter()
            .filter(|(_, s)| {
                s.iter()
                    .skip(1)
                    .any(|o| differs(o.0, s[0].0) || differs(o.1, s[0].1))
            })
            .map(|(edge, _)| edge)
            .collect()
    }

    /// Ilha UV derivada: faces ligadas por edges sem seam **e** com UV contínua
    /// nos dois extremos (website/docs/07-uv/architecture-decision.md, "Islands").
    /// Uma divisão de UV sem seam também separa ilhas; o diagnóstico a reporta.
    pub fn uv_islands(&self) -> Vec<UvIsland> {
        let splits = self.uv_split_edges();
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            for k in 0..n {
                let e = edge_key(face.verts[k], face.verts[(k + 1) % n]);
                edge_faces.entry(e).or_default().push(fi);
            }
        }
        for (edge, faces) in &edge_faces {
            if self.uv_seams.contains(edge) || splits.contains(edge) {
                continue;
            }
            if faces.len() == 2 {
                adj.entry(faces[0]).or_default().push(faces[1]);
                adj.entry(faces[1]).or_default().push(faces[0]);
            }
        }
        for list in adj.values_mut() {
            list.sort_unstable();
        }
        let mut seen = vec![false; self.faces.len()];
        let mut islands = Vec::new();
        for start in 0..self.faces.len() {
            if seen[start] {
                continue;
            }
            let mut q = VecDeque::new();
            let mut faces = Vec::new();
            seen[start] = true;
            q.push_back(start);
            while let Some(fi) = q.pop_front() {
                faces.push(fi);
                if let Some(nbs) = adj.get(&fi) {
                    for &nb in nbs {
                        if !seen[nb] {
                            seen[nb] = true;
                            q.push_back(nb);
                        }
                    }
                }
            }
            let mut min = [f32::MAX, f32::MAX];
            let mut max = [f32::MIN, f32::MIN];
            for &fi in &faces {
                for uv in &self.faces[fi].uv {
                    min[0] = min[0].min(uv[0]);
                    min[1] = min[1].min(uv[1]);
                    max[0] = max[0].max(uv[0]);
                    max[1] = max[1].max(uv[1]);
                }
            }
            if min[0] > max[0] {
                min = [0.0, 0.0];
                max = [0.0, 0.0];
            }
            islands.push(UvIsland { faces, min, max });
        }
        islands
    }

    /// Average texels-per-unit for selected (or all) faces.
    pub fn texel_density(&self, texture_w: u32, selected_only: bool) -> f32 {
        let (mut world_area, mut uv_area) = (0.0f32, 0.0f32);
        for (i, face) in self.faces.iter().enumerate() {
            if selected_only && !face.selected {
                continue;
            }
            if face.uv.len() != face.verts.len()
                || face.verts.iter().any(|v| *v as usize >= self.verts.len())
            {
                continue;
            }
            for corners in self.face_triangle_corners(i) {
                let [a, b, c] = corners.map(|j| self.verts[face.verts[j] as usize].vec());
                let world = (b - a).cross(c - a).length() * 0.5;
                let [ua, ub, uc] = corners.map(|j| glam::Vec2::from_array(face.uv[j]));
                let uv = (ub - ua).perp_dot(uc - ua).abs() * 0.5;
                if world > 1e-8 && world.is_finite() && uv.is_finite() {
                    world_area += world;
                    uv_area += uv;
                }
            }
        }
        if world_area <= 1e-8 {
            0.0
        } else {
            (uv_area / world_area).sqrt() * texture_w as f32
        }
    }

    /// Texel density em px/unidade de mundo das faces (`faces` vazio = todas),
    /// para uma textura `[largura, altura]`: `sqrt(área UV / área 3D) ·
    /// sqrt(largura · altura)` — vale também para texturas não quadradas
    /// (website/docs/07-uv/architecture-decision.md, "Texel Density").
    pub fn texel_density_px(&self, faces: &HashSet<usize>, texture_size: [u32; 2]) -> f32 {
        let (world, uv) = self.uv_world_areas(|fi| faces.is_empty() || faces.contains(&fi));
        if world <= 1e-8 {
            return 0.0;
        }
        (uv / world).sqrt() * ((texture_size[0] as f32) * (texture_size[1] as f32)).sqrt()
    }

    /// Set Density: escala cada ilha que toca `faces` (vazio = todas) em torno
    /// do centro da própria caixa UV até `target_px` px/unidade. Ilhas sem área
    /// são ignoradas. Retorna as ilhas alteradas.
    pub fn set_texel_density(
        &mut self,
        faces: &HashSet<usize>,
        texture_size: [u32; 2],
        target_px: f32,
    ) -> usize {
        if !target_px.is_finite() || target_px <= 0.0 {
            return 0;
        }
        let mut changed = 0;
        for island in self.uv_islands() {
            if !faces.is_empty() && !island.faces.iter().any(|f| faces.contains(f)) {
                continue;
            }
            let members: HashSet<usize> = island.faces.iter().copied().collect();
            let current = self.texel_density_px(&members, texture_size);
            if current <= 1e-6 {
                continue;
            }
            let scale = target_px / current;
            if (scale - 1.0).abs() < 1e-5 {
                continue;
            }
            let center = island.center();
            for &fi in &island.faces {
                for uv in &mut self.faces[fi].uv {
                    uv[0] = center[0] + (uv[0] - center[0]) * scale;
                    uv[1] = center[1] + (uv[1] - center[1]) * scale;
                }
            }
            changed += 1;
        }
        changed
    }

    /// Soma (área 3D, área UV) dos triângulos derivados das faces aceitas.
    fn uv_world_areas(&self, accept: impl Fn(usize) -> bool) -> (f32, f32) {
        let (mut world_area, mut uv_area) = (0.0f32, 0.0f32);
        for (fi, face) in self.faces.iter().enumerate() {
            if !accept(fi)
                || face.uv.len() != face.verts.len()
                || face.verts.iter().any(|v| *v as usize >= self.verts.len())
            {
                continue;
            }
            for corners in self.face_triangle_corners(fi) {
                let [a, b, c] = corners.map(|j| self.verts[face.verts[j] as usize].vec());
                let world = (b - a).cross(c - a).length() * 0.5;
                let [ua, ub, uc] = corners.map(|j| glam::Vec2::from_array(face.uv[j]));
                let uv = (ub - ua).perp_dot(uc - ua).abs() * 0.5;
                if world > 1e-8 && world.is_finite() && uv.is_finite() {
                    world_area += world;
                    uv_area += uv;
                }
            }
        }
        (world_area, uv_area)
    }

    pub fn normalize_texel_density(&mut self, texture_w: u32, target: f32) {
        let current = self.texel_density(texture_w, false).max(1e-6);
        let s = (target / current).clamp(0.01, 100.0);
        for face in &mut self.faces {
            for uv in &mut face.uv {
                uv[0] = 0.5 + (uv[0] - 0.5) * s;
                uv[1] = 0.5 + (uv[1] - 0.5) * s;
            }
        }
    }

    /// Equaliza a densidade de texels individualmente entre as ilhas UV,
    /// garantindo que ilhas com proporções diferentes alcancem o mesmo ratio UV/3D.
    pub fn equalize_texel_density(&mut self, selected_faces: &HashSet<usize>) -> usize {
        let islands = self.uv_islands();
        if islands.is_empty() {
            return 0;
        }

        let has_sel = !selected_faces.is_empty();
        let target_indices: Vec<usize> = if has_sel {
            islands
                .iter()
                .enumerate()
                .filter(|(_, isl)| isl.faces.iter().any(|f| selected_faces.contains(f)))
                .map(|(i, _)| i)
                .collect()
        } else {
            (0..islands.len()).collect()
        };

        if target_indices.is_empty() {
            return 0;
        }

        let mut island_metrics = Vec::new();
        let mut total_3d = 0.0f32;
        let mut total_uv = 0.0f32;

        for &idx in &target_indices {
            let isl = &islands[idx];
            let mut area_3d = 0.0f32;
            let mut area_uv = 0.0f32;
            let mut center_uv = [0.0f32; 2];
            let mut uv_count = 0;

            for &fi in &isl.faces {
                let face = &self.faces[fi];
                for corners in self.face_triangle_corners(fi) {
                    let [p0, p1, p2] = corners.map(|i| self.verts[face.verts[i] as usize].vec());
                    let tri_3d = (p1 - p0).cross(p2 - p0).length() * 0.5;
                    area_3d += tri_3d;

                    if face.uv.len() >= face.verts.len() {
                        let uv0 = face.uv[corners[0]];
                        let uv1 = face.uv[corners[1]];
                        let uv2 = face.uv[corners[2]];
                        let tri_uv = ((uv1[0] - uv0[0]) * (uv2[1] - uv0[1])
                            - (uv2[0] - uv0[0]) * (uv1[1] - uv0[1]))
                            .abs()
                            * 0.5;
                        area_uv += tri_uv;
                    }
                }
                for uv in &face.uv {
                    center_uv[0] += uv[0];
                    center_uv[1] += uv[1];
                    uv_count += 1;
                }
            }

            if uv_count > 0 {
                center_uv[0] /= uv_count as f32;
                center_uv[1] /= uv_count as f32;
            }

            if area_3d > 1e-6 && area_uv > 1e-8 {
                total_3d += area_3d;
                total_uv += area_uv;
            }
            island_metrics.push((idx, area_3d, area_uv, center_uv));
        }

        if total_3d <= 1e-6 || total_uv <= 1e-8 {
            return 0;
        }

        let target_density = (total_uv / total_3d).sqrt();
        let mut modified = 0;

        for (idx, area_3d, area_uv, center_uv) in island_metrics {
            if area_3d <= 1e-6 || area_uv <= 1e-8 {
                continue;
            }
            let current_density = (area_uv / area_3d).sqrt();
            let scale_factor = (target_density / current_density).clamp(0.05, 20.0);
            if (scale_factor - 1.0).abs() < 1e-4 {
                continue;
            }

            for &fi in &islands[idx].faces {
                for uv in &mut self.faces[fi].uv {
                    uv[0] = center_uv[0] + (uv[0] - center_uv[0]) * scale_factor;
                    uv[1] = center_uv[1] + (uv[1] - center_uv[1]) * scale_factor;
                }
            }
            modified += 1;
        }

        modified
    }

    pub fn project_from_view(
        &mut self,
        view_x: glam::Vec3,
        view_y: glam::Vec3,
        view_origin: glam::Vec3,
    ) {
        let positions: Vec<glam::Vec3> = self.verts.iter().map(|v| v.vec()).collect();
        self.project_from_view_positions(&positions, view_x, view_y, view_origin);
    }

    /// Projeção de vista sobre `positions` (uma por vértice, ex.: posições em
    /// mundo de um objeto transformado), gravando UV nos cantos autorais.
    /// `positions` com tamanho diferente de `verts` não faz nada.
    pub fn project_from_view_positions(
        &mut self,
        positions: &[glam::Vec3],
        view_x: glam::Vec3,
        view_y: glam::Vec3,
        view_origin: glam::Vec3,
    ) {
        if positions.len() != self.verts.len() {
            return;
        }
        let vx = view_x.normalize_or_zero();
        let vy = view_y.normalize_or_zero();
        if vx.length_squared() < 1e-8 || vy.length_squared() < 1e-8 {
            self.project_planar();
            return;
        }
        let mut us = Vec::new();
        let mut vs = Vec::new();
        for &position in positions {
            let p = position - view_origin;
            us.push(p.dot(vx));
            vs.push(p.dot(vy));
        }
        let (u_min, u_max) = minmax(&us);
        let (v_min, v_max) = minmax(&vs);
        let du = (u_max - u_min).max(1e-6);
        let dv = (v_max - v_min).max(1e-6);
        for face in &mut self.faces {
            face.uv = face
                .verts
                .iter()
                .map(|&vi| {
                    [
                        (us[vi as usize] - u_min) / du,
                        (vs[vi as usize] - v_min) / dv,
                    ]
                })
                .collect();
        }
    }

    pub fn uv_diagnostics(&self) -> UvDiagnostics {
        const ZERO_UV_AREA: f32 = 1.0e-10;
        let islands = self.uv_islands();
        let mut overlapping = 0;
        for i in 0..islands.len() {
            for j in (i + 1)..islands.len() {
                if aabb_overlap(
                    islands[i].min,
                    islands[i].max,
                    islands[j].min,
                    islands[j].max,
                ) {
                    overlapping += 1;
                }
            }
        }

        // (área 3D, área UV) de cada triângulo derivado com UV completa.
        let mut triangles: Vec<(f32, f32)> = Vec::new();
        let mut zero_area = 0;
        let mut out_of_range = 0;
        for (fi, face) in self.faces.iter().enumerate() {
            if face.verts.len() < 3 || face.uv.len() != face.verts.len() {
                zero_area += 1;
                continue;
            }
            out_of_range += face
                .uv
                .iter()
                .filter(|uv| !(0.0..=1.0).contains(&uv[0]) || !(0.0..=1.0).contains(&uv[1]))
                .count();
            let mut face_uv_area = 0.0;
            for corners in self.face_triangle_corners(fi) {
                let [a, b, c] = corners.map(|k| self.verts[face.verts[k] as usize].vec());
                let world = (b - a).cross(c - a).length() * 0.5;
                let [ua, ub, uc] = corners.map(|k| glam::Vec2::from_array(face.uv[k]));
                let uv = (ub - ua).perp_dot(uc - ua).abs() * 0.5;
                if world.is_finite() && uv.is_finite() {
                    face_uv_area += uv;
                    triangles.push((world, uv));
                }
            }
            if face_uv_area < ZERO_UV_AREA {
                zero_area += 1;
            }
        }

        let (world_total, uv_total) = triangles
            .iter()
            .fold((0.0f32, 0.0f32), |(w, u), t| (w + t.0, u + t.1));
        let (mut stretch_sum, mut stretch_max, mut stretch_weight) = (0.0f32, 0.0f32, 0.0f32);
        if world_total > 1.0e-12 && uv_total > ZERO_UV_AREA {
            let density = uv_total / world_total;
            for &(world, uv) in &triangles {
                if world <= 1.0e-12 || uv < ZERO_UV_AREA {
                    continue; // contado em zero_area_faces, não como stretch infinito
                }
                let r = (uv / world) / density;
                let stretch = r.max(1.0 / r) - 1.0;
                stretch_sum += stretch * world;
                stretch_weight += world;
                stretch_max = stretch_max.max(stretch);
            }
        }

        let splits = self.uv_split_edges();
        let mut edge_use: HashMap<(u32, u32), usize> = HashMap::new();
        for face in &self.faces {
            let n = face.verts.len();
            for k in 0..n {
                *edge_use
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_default() += 1;
            }
        }
        let seams_without_split = self
            .uv_seams
            .iter()
            .filter(|e| edge_use.get(e).is_some_and(|&n| n >= 2) && !splits.contains(e))
            .count();
        let splits_without_seam = splits.iter().filter(|e| !self.uv_seams.contains(e)).count();

        UvDiagnostics {
            island_count: islands.len(),
            overlapping_islands: overlapping,
            zero_area_faces: zero_area,
            out_of_range_corners: out_of_range,
            mean_stretch: if stretch_weight > 0.0 {
                stretch_sum / stretch_weight
            } else {
                0.0
            },
            max_stretch: stretch_max,
            overlap_ratio: self.uv_overlap_ratio(),
            splits_without_seam,
            seams_without_split,
        }
    }
}

fn minmax(v: &[f32]) -> (f32, f32) {
    let mut mn = f32::MAX;
    let mut mx = f32::MIN;
    for &x in v {
        mn = mn.min(x);
        mx = mx.max(x);
    }
    if mn > mx { (0.0, 1.0) } else { (mn, mx) }
}

fn aabb_overlap(amin: [f32; 2], amax: [f32; 2], bmin: [f32; 2], bmax: [f32; 2]) -> bool {
    amin[0] < bmax[0] && amax[0] > bmin[0] && amin[1] < bmax[1] && amax[1] > bmin[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_pinning_methods() {
        let mut m = Mesh::cube(2.0);
        assert!(!m.is_uv_pinned(0, 0));
        assert_eq!(m.uv_pinned.len(), 0);

        m.pin_uv(0, 0);
        assert!(m.is_uv_pinned(0, 0));
        assert_eq!(m.uv_pinned.len(), 1);

        m.unpin_uv(0, 0);
        assert!(!m.is_uv_pinned(0, 0));

        m.toggle_pin_uv(0, 1);
        assert!(m.is_uv_pinned(0, 1));
        m.toggle_pin_uv(0, 1);
        assert!(!m.is_uv_pinned(0, 1));

        let mut sel = HashSet::new();
        sel.insert(0);
        m.pin_selected_faces_uv(&sel);
        assert_eq!(m.uv_pinned.len(), m.faces[0].uv.len());

        m.clear_all_pins();
        assert_eq!(m.uv_pinned.len(), 0);
    }

    #[test]
    fn test_uv_pinning_preserves_projection() {
        let mut m = Mesh::cube(2.0);
        m.project_cube();

        // Altera artificialmente o UV do canto (0, 0) para [0.42, 0.42] e fixa ele
        m.faces[0].uv[0] = [0.42, 0.42];
        m.pin_uv(0, 0);

        // Re-projeta cúbico e planar
        m.project_cube();
        assert_eq!(m.faces[0].uv[0], [0.42, 0.42]);

        m.project_planar();
        assert_eq!(m.faces[0].uv[0], [0.42, 0.42]);
    }

    #[test]
    fn test_uv_stitch_welds_seam_edge() {
        let mut m = Mesh::cube(2.0);
        m.project_cube();
        // Separamos artificialmente o UV de uma face adjacente
        m.faces[1].uv[0] = [0.99, 0.99];
        let filter = HashSet::new();
        let stitched = m.stitch_uv(&filter);
        assert!(stitched > 0);
    }

    #[test]
    fn area_stretch_ignores_uniform_scale_and_measures_distortion() {
        let mut m = Mesh::plane(2.0);
        m.faces[0].uv = vec![[0.0, 0.0], [0.1, 0.0], [0.1, 0.1], [0.0, 0.1]];
        let d = m.uv_diagnostics();
        assert!(d.mean_stretch.abs() < 1e-4 && d.max_stretch.abs() < 1e-4);

        // Dois quads lado a lado; um deles recebe o dobro da área UV.
        let mut two = Mesh::plane(2.0);
        let mut other = Mesh::plane(2.0);
        other.translate_selected([5.0, 0.0, 0.0]);
        two.join(&other);
        two.faces[0].uv = vec![[0.0, 0.0], [0.2, 0.0], [0.2, 0.2], [0.0, 0.2]];
        two.faces[1].uv = vec![[0.5, 0.0], [0.7, 0.0], [0.7, 0.4], [0.5, 0.4]];
        let d = two.uv_diagnostics();
        assert!(d.max_stretch > 0.3, "max_stretch={}", d.max_stretch);
        assert_eq!(d.zero_area_faces, 0);
    }

    #[test]
    fn diagnostics_report_splits_and_seams_that_disagree() {
        let mut m = Mesh::cube(2.0);
        m.project_cube();
        let base = m.uv_diagnostics();
        assert_eq!(base.seams_without_split, 0);

        // Sem seams marcadas, toda divisão de UV aparece como "split without seam";
        // marcar exatamente essas edges zera o diagnóstico.
        let splits = m.uv_split_edges();
        assert_eq!(base.splits_without_seam, splits.len());
        m.uv_seams.extend(splits.iter().copied());
        assert_eq!(m.uv_diagnostics().splits_without_seam, 0);
    }

    #[test]
    fn uv_split_without_seam_separates_islands() {
        let mut m = Mesh::plane(2.0);
        let mut other = Mesh::plane(2.0);
        other.translate_selected([2.0, 0.0, 0.0]);
        m.join(&other);
        let _remap = m.weld_with_remap(1e-4);
        assert_eq!(m.faces.len(), 2);
        let shared = m.uv_split_edges();
        // Os planos vieram com UVs independentes (0..1 cada): a edge comum se divide.
        assert!(!shared.is_empty());
        assert_eq!(m.uv_islands().len(), 2);
    }

    #[test]
    fn test_uv_relax_smooths_corners() {
        let mut m = Mesh::cube(2.0);
        m.project_cube();
        let filter = HashSet::new();
        let relaxed = m.relax_uv(&filter, 5);
        assert_eq!(relaxed, m.faces.len());
    }
}

#[cfg(test)]
mod density_tests {
    use crate::Mesh;
    #[test]
    fn density_px_uses_both_texture_axes_and_set_density_hits_the_target() {
        use std::collections::HashSet;
        let mut mesh = Mesh::plane(2.0);
        let all = HashSet::new();
        let square = mesh.texel_density_px(&all, [512, 512]);
        assert!((square - mesh.texel_density(512, false)).abs() < 1e-3);
        // 1024×256 tem a mesma quantidade de pixels que 512².
        assert!((mesh.texel_density_px(&all, [1024, 256]) - square).abs() < 1e-3);

        assert_eq!(mesh.set_texel_density(&all, [512, 512], 64.0), 1);
        assert!((mesh.texel_density_px(&all, [512, 512]) - 64.0).abs() < 1e-2);
        assert_eq!(mesh.set_texel_density(&all, [512, 512], f32::NAN), 0);
    }

    #[test]
    fn density_scales_with_linear_resolution_and_inverse_world_scale() {
        let mut mesh = Mesh::plane(2.0);
        let density = mesh.texel_density(256, false);
        assert!(density > 0.0);
        assert!((mesh.texel_density(512, false) / density - 2.0).abs() < 1e-5);
        for vertex in &mut mesh.verts {
            for v in &mut vertex.pos {
                *v *= 2.0;
            }
        }
        assert!((mesh.texel_density(256, false) / density - 0.5).abs() < 1e-5);
        mesh.normalize_texel_density(256, 32.0);
        assert!((mesh.texel_density(256, false) - 32.0).abs() < 1e-3);
    }
}
