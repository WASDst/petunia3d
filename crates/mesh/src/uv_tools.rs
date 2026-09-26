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

/// Stretch / overlap / invalid UV report.
#[derive(Clone, Debug, Default)]
pub struct UvDiagnostics {
    pub island_count: usize,
    pub overlapping_islands: usize,
    pub zero_area_faces: usize,
    pub out_of_range_corners: usize,
    pub mean_stretch: f32,
    pub max_stretch: f32,
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

    /// Costura (stitch / weld) as arestas de costura (seams) no espaço UV, unificando as coordenadas
    /// UV das faces adjacentes nas arestas compartilhadas da malha 3D.
    pub fn stitch_uv(&mut self, faces_filter: &HashSet<usize>) -> usize {
        let all = faces_filter.is_empty();
        let mut stitched = 0;
        let edges = self.edges_unique();

        for (u, v) in edges {
            let adj = self.edge_faces(u, v);
            if adj.len() == 2 {
                let (f1_idx, f2_idx) = (adj[0], adj[1]);
                if (all || faces_filter.contains(&f1_idx))
                    && (all || faces_filter.contains(&f2_idx))
                {
                    let f1 = &self.faces[f1_idx];
                    let f2 = &self.faces[f2_idx];
                    let p1_u = f1.verts.iter().position(|&x| x == u);
                    let p1_v = f1.verts.iter().position(|&x| x == v);
                    let p2_u = f2.verts.iter().position(|&x| x == u);
                    let p2_v = f2.verts.iter().position(|&x| x == v);

                    if let (Some(i1_u), Some(i1_v), Some(i2_u), Some(i2_v)) =
                        (p1_u, p1_v, p2_u, p2_v)
                        && i1_u < f1.uv.len()
                        && i1_v < f1.uv.len()
                        && i2_u < f2.uv.len()
                        && i2_v < f2.uv.len()
                    {
                        let uv1_u = f1.uv[i1_u];
                        let uv1_v = f1.uv[i1_v];
                        let uv2_u = f2.uv[i2_u];
                        let uv2_v = f2.uv[i2_v];

                        let diff_u = (uv1_u[0] - uv2_u[0]).abs() + (uv1_u[1] - uv2_u[1]).abs();
                        let diff_v = (uv1_v[0] - uv2_v[0]).abs() + (uv1_v[1] - uv2_v[1]).abs();

                        if diff_u > 1e-4 || diff_v > 1e-4 {
                            let mid_u = [(uv1_u[0] + uv2_u[0]) * 0.5, (uv1_u[1] + uv2_u[1]) * 0.5];
                            let mid_v = [(uv1_v[0] + uv2_v[0]) * 0.5, (uv1_v[1] + uv2_v[1]) * 0.5];

                            if !self.uv_pinned.contains(&(f1_idx, i1_u)) {
                                self.faces[f1_idx].uv[i1_u] = mid_u;
                            }
                            if !self.uv_pinned.contains(&(f2_idx, i2_u)) {
                                self.faces[f2_idx].uv[i2_u] = mid_u;
                            }
                            if !self.uv_pinned.contains(&(f1_idx, i1_v)) {
                                self.faces[f1_idx].uv[i1_v] = mid_v;
                            }
                            if !self.uv_pinned.contains(&(f2_idx, i2_v)) {
                                self.faces[f2_idx].uv[i2_v] = mid_v;
                            }
                            self.uv_seams.remove(&edge_key(u, v));
                            stitched += 1;
                        }
                    }
                }
            }
        }
        stitched
    }

    /// Suavização e relaxamento iterativo (Laplacian smoothing) das coordenadas UV não fixadas
    /// para minimizar distorção e estiramento de texels.
    pub fn relax_uv(&mut self, faces_filter: &HashSet<usize>, iterations: usize) -> usize {
        let all = faces_filter.is_empty();
        let iters = iterations.clamp(1, 20);
        let mut relaxed = 0;

        for _ in 0..iters {
            let mut new_uvs: Vec<(usize, usize, [f32; 2])> = Vec::new();

            for (fi, face) in self.faces.iter().enumerate() {
                if all || faces_filter.contains(&fi) {
                    let m = face.uv.len();
                    if m >= 3 {
                        for c in 0..m {
                            if !self.uv_pinned.contains(&(fi, c)) {
                                let prev = face.uv[(c + m - 1) % m];
                                let next = face.uv[(c + 1) % m];
                                let curr = face.uv[c];
                                let target = [(prev[0] + next[0]) * 0.5, (prev[1] + next[1]) * 0.5];
                                let smoothed = [
                                    curr[0] * 0.5 + target[0] * 0.5,
                                    curr[1] * 0.5 + target[1] * 0.5,
                                ];
                                new_uvs.push((fi, c, smoothed));
                            }
                        }
                        relaxed += 1;
                    }
                }
            }

            for (fi, c, uv) in new_uvs {
                self.faces[fi].uv[c] = uv;
            }
        }
        relaxed
    }

    /// Faces sharing a non-seam UV edge form an island.
    pub fn uv_islands(&self) -> Vec<UvIsland> {
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
            if self.uv_seams.contains(edge) {
                continue;
            }
            if faces.len() == 2 {
                adj.entry(faces[0]).or_default().push(faces[1]);
                adj.entry(faces[1]).or_default().push(faces[0]);
            }
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

    /// Pack islands into 0..1 with padding, no overlaps.
    pub fn pack_uv_islands(&mut self, padding: f32) -> usize {
        let padding = padding.clamp(0.0, 0.25);
        let mut islands = self.uv_islands();
        if islands.is_empty() {
            return 0;
        }
        islands.sort_by(|a, b| {
            b.area()
                .partial_cmp(&a.area())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = islands.len();
        let cols = (n as f32).sqrt().ceil().max(1.0) as usize;
        let cell = (1.0 / cols as f32).max(1e-6);
        let usable = (cell - padding * 2.0).max(1e-6);
        for (i, island) in islands.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let ox = col as f32 * cell + padding;
            let oy = row as f32 * cell + padding;
            let w = (island.max[0] - island.min[0]).max(1e-6);
            let h = (island.max[1] - island.min[1]).max(1e-6);
            let s = (usable / w).min(usable / h);
            for &fi in &island.faces {
                for uv in &mut self.faces[fi].uv {
                    uv[0] = ox + (uv[0] - island.min[0]) * s;
                    uv[1] = oy + (uv[1] - island.min[1]) * s;
                }
            }
        }
        n
    }

    /// Average texels-per-unit for selected (or all) faces.
    pub fn texel_density(&self, texture_w: u32, selected_only: bool) -> f32 {
        let mut acc = 0.0;
        let mut n = 0.0;
        for face in &self.faces {
            if selected_only && !face.selected {
                continue;
            }
            if face.verts.len() < 3 || face.uv.len() < 3 {
                continue;
            }
            let a = self.verts[face.verts[0] as usize].vec();
            let b = self.verts[face.verts[1] as usize].vec();
            let c = self.verts[face.verts[2] as usize].vec();
            let world = (b - a).cross(c - a).length() * 0.5;
            let ua = glam::Vec2::from_array(face.uv[0]);
            let ub = glam::Vec2::from_array(face.uv[1]);
            let uc = glam::Vec2::from_array(face.uv[2]);
            let uv_area = (ub - ua).perp_dot(uc - ua).abs() * 0.5;
            if world > 1e-8 {
                acc += (uv_area * texture_w as f32) / world;
                n += 1.0;
            }
        }
        if n <= 0.0 { 0.0 } else { acc / n }
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
        let vx = view_x.normalize_or_zero();
        let vy = view_y.normalize_or_zero();
        if vx.length_squared() < 1e-8 || vy.length_squared() < 1e-8 {
            self.project_planar();
            return;
        }
        let mut us = Vec::new();
        let mut vs = Vec::new();
        for v in &self.verts {
            let p = v.vec() - view_origin;
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
        let mut zero_area = 0;
        let mut oor = 0;
        let mut stretch_sum = 0.0;
        let mut stretch_max: f32 = 0.0;
        let mut stretch_n = 0.0;
        for face in &self.faces {
            if face.uv.len() < 3 || face.verts.len() < 3 {
                zero_area += 1;
                continue;
            }
            for uv in &face.uv {
                if !(0.0..=1.0).contains(&uv[0]) || !(0.0..=1.0).contains(&uv[1]) {
                    oor += 1;
                }
            }
            let a = self.verts[face.verts[0] as usize].vec();
            let b = self.verts[face.verts[1] as usize].vec();
            let world = (b - a).length().max(1e-8);
            let uv_len = (glam::Vec2::from_array(face.uv[1]) - glam::Vec2::from_array(face.uv[0]))
                .length()
                .max(1e-8);
            let s = (uv_len / world - 1.0).abs();
            stretch_sum += s;
            stretch_max = stretch_max.max(s);
            stretch_n += 1.0;
            let ua = glam::Vec2::from_array(face.uv[0]);
            let ub = glam::Vec2::from_array(face.uv[1]);
            let uc = glam::Vec2::from_array(face.uv[2]);
            if (ub - ua).perp_dot(uc - ua).abs() < 1e-10 {
                zero_area += 1;
            }
        }
        UvDiagnostics {
            island_count: islands.len(),
            overlapping_islands: overlapping,
            zero_area_faces: zero_area,
            out_of_range_corners: oor,
            mean_stretch: if stretch_n == 0.0 {
                0.0
            } else {
                stretch_sum / stretch_n
            },
            max_stretch: stretch_max,
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
    fn test_uv_relax_smooths_corners() {
        let mut m = Mesh::cube(2.0);
        m.project_cube();
        let filter = HashSet::new();
        let relaxed = m.relax_uv(&filter, 5);
        assert_eq!(relaxed, m.faces.len() * 5);
    }
}
