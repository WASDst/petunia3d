//! Petunia3D mesh — operações de modelagem.

use super::{Face, Mesh, Vertex, edge_key, triangulate};
use glam::Vec3;
use std::collections::{HashMap, HashSet};

impl Mesh {
    // ---------------- seleção ----------------

    pub fn select_all(&mut self) {
        for v in &mut self.verts {
            v.selected = true;
        }
        for f in &mut self.faces {
            f.selected = true;
        }
        self.selected_edges = self.edges_unique().into_iter().collect();
    }
    pub fn deselect_all(&mut self) {
        for v in &mut self.verts {
            v.selected = false;
        }
        for f in &mut self.faces {
            f.selected = false;
        }
        self.selected_edges.clear();
    }

    /// Inverte o estado de seleção de todos os vértices e sincroniza faces e arestas.
    pub fn invert_selection(&mut self) {
        for v in &mut self.verts {
            v.selected = !v.selected;
        }
        self.sync_face_selection_from_verts();
        self.sync_edge_selection_from_verts();
    }

    /// Seleciona todos os vértices e faces conectados geometricamente aos elementos já selecionados (Select Linked).
    pub fn select_linked(&mut self) {
        let mut queue = std::collections::VecDeque::new();
        let mut visited = vec![false; self.verts.len()];

        for (i, v) in self.verts.iter().enumerate() {
            if v.selected {
                queue.push_back(i);
                visited[i] = true;
            }
        }

        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); self.verts.len()];
        for f in &self.faces {
            let m = f.verts.len();
            for k in 0..m {
                let u = f.verts[k] as usize;
                let v = f.verts[(k + 1) % m] as usize;
                if u < self.verts.len() && v < self.verts.len() {
                    adj[u].push(v);
                    adj[v].push(u);
                }
            }
        }

        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    self.verts[v].selected = true;
                    queue.push_back(v);
                }
            }
        }

        self.sync_face_selection_from_verts();
        self.sync_edge_selection_from_verts();
    }

    /// Seleciona vértices contidos dentro de um retângulo em coordenadas normalizadas de tela (NDC [-1, 1]).
    pub fn box_select(
        &mut self,
        min_ndc: [f32; 2],
        max_ndc: [f32; 2],
        view_proj: &[f32; 16],
        add_mode: bool,
    ) {
        let vp = glam::Mat4::from_cols_array(view_proj);
        let x_min = min_ndc[0].min(max_ndc[0]);
        let x_max = min_ndc[0].max(max_ndc[0]);
        let y_min = min_ndc[1].min(max_ndc[1]);
        let y_max = min_ndc[1].max(max_ndc[1]);

        for v in &mut self.verts {
            let clip = vp * v.vec().extend(1.0);
            let inside = if clip.w > 1e-4 {
                let ndc = clip.truncate() / clip.w;
                ndc.x >= x_min
                    && ndc.x <= x_max
                    && ndc.y >= y_min
                    && ndc.y <= y_max
                    && ndc.z >= -1.0
                    && ndc.z <= 1.0
            } else {
                false
            };
            if add_mode {
                v.selected |= inside;
            } else {
                v.selected = inside;
            }
        }
        self.sync_face_selection_from_verts();
        self.sync_edge_selection_from_verts();
    }
    pub fn selected_vert_count(&self) -> usize {
        self.verts.iter().filter(|v| v.selected).count()
    }
    pub fn selected_face_count(&self) -> usize {
        self.faces.iter().filter(|f| f.selected).count()
    }
    pub fn has_selection(&self) -> bool {
        self.selected_vert_count() > 0
            || self.selected_face_count() > 0
            || !self.selected_edges.is_empty()
    }

    /// Retorna todas as arestas pertencentes ao Edge Loop contínuo a partir de uma aresta semente.
    /// Segue a topologia através de vértices de valência 4 em malhas quadriláteras ou bordas de contorno.
    pub fn edge_loop(&self, seed: (u32, u32)) -> Vec<(u32, u32)> {
        let seed_key = edge_key(seed.0, seed.1);
        if !self.edges_unique().contains(&seed_key) {
            return Vec::new();
        }
        let mut loop_set = std::collections::HashSet::new();
        loop_set.insert(seed_key);

        let walk_half = |start_u: u32, start_v: u32| -> Vec<(u32, u32)> {
            let mut half = Vec::new();
            let mut prev = start_u;
            let mut curr = start_v;
            let mut visited = std::collections::HashSet::new();
            visited.insert(edge_key(start_u, start_v));

            loop {
                let edge_faces = self.edge_faces(prev, curr);
                let incident_faces: Vec<usize> = self
                    .faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.verts.contains(&curr))
                    .map(|(i, _)| i)
                    .collect();

                let next_candidate: Option<u32> = if edge_faces.len() == 1 {
                    // Aresta de contorno: seguir pela próxima aresta de contorno conectada a curr
                    let mut b_edges = Vec::new();
                    for &fi in &incident_faces {
                        let f = &self.faces[fi];
                        let m = f.verts.len();
                        for k in 0..m {
                            let a = f.verts[k];
                            let b = f.verts[(k + 1) % m];
                            if (a == curr || b == curr)
                                && edge_key(a, b) != edge_key(prev, curr)
                                && self.edge_faces(a, b).len() == 1
                            {
                                let other = if a == curr { b } else { a };
                                b_edges.push(other);
                            }
                        }
                    }
                    b_edges.dedup();
                    if b_edges.len() == 1 {
                        Some(b_edges[0])
                    } else {
                        None
                    }
                } else if edge_faces.len() == 2 && incident_faces.len() == 4 {
                    // Vértice interior de malha regular: valência 4 e todas as 4 faces são quads
                    if incident_faces
                        .iter()
                        .all(|&fi| self.faces[fi].verts.len() == 4)
                    {
                        let mut all_neighbors: Vec<u32> = Vec::new();
                        for &fi in &incident_faces {
                            let f = &self.faces[fi];
                            if let Some(pos) = f.verts.iter().position(|&x| x == curr) {
                                let v1 = f.verts[(pos + 3) % 4];
                                let v2 = f.verts[(pos + 1) % 4];
                                if !all_neighbors.contains(&v1) {
                                    all_neighbors.push(v1);
                                }
                                if !all_neighbors.contains(&v2) {
                                    all_neighbors.push(v2);
                                }
                            }
                        }
                        let mut shared_face_neighbors = std::collections::HashSet::new();
                        shared_face_neighbors.insert(prev);
                        for &fi in &edge_faces {
                            let f = &self.faces[fi];
                            if let Some(pos) = f.verts.iter().position(|&x| x == curr) {
                                shared_face_neighbors.insert(f.verts[(pos + 3) % 4]);
                                shared_face_neighbors.insert(f.verts[(pos + 1) % 4]);
                            }
                        }
                        all_neighbors
                            .into_iter()
                            .find(|neighbor| !shared_face_neighbors.contains(neighbor))
                    } else {
                        None
                    }
                } else {
                    None
                };

                match next_candidate {
                    Some(next_v) => {
                        let key = edge_key(curr, next_v);
                        if !visited.insert(key) {
                            break;
                        }
                        half.push(key);
                        prev = curr;
                        curr = next_v;
                    }
                    None => break,
                }
            }
            half
        };

        for e in walk_half(seed.0, seed.1) {
            loop_set.insert(e);
        }
        for e in walk_half(seed.1, seed.0) {
            loop_set.insert(e);
        }

        loop_set.into_iter().collect()
    }

    /// Seleciona o Edge Loop completo a partir de uma aresta semente. Retorna a contagem de arestas no loop.
    pub fn select_edge_loop(&mut self, seed: (u32, u32), extend: bool) -> usize {
        let loop_edges = self.edge_loop(seed);
        let count = loop_edges.len();
        if !extend {
            self.selected_edges.clear();
        }
        for e in loop_edges {
            self.selected_edges.insert(e);
        }
        for v in &mut self.verts {
            v.selected = false;
        }
        for &(a, b) in &self.selected_edges {
            if let Some(v) = self.verts.get_mut(a as usize) {
                v.selected = true;
            }
            if let Some(v) = self.verts.get_mut(b as usize) {
                v.selected = true;
            }
        }
        count
    }

    /// Seleciona o Vertex Loop a partir de um vértice semente (caminhando pelas arestas de loop).
    pub fn select_vertex_loop(&mut self, seed_vertex: u32, extend: bool) -> usize {
        let mut incident_edges = Vec::new();
        for f in &self.faces {
            let m = f.verts.len();
            for k in 0..m {
                let u = f.verts[k];
                let v = f.verts[(k + 1) % m];
                if u == seed_vertex || v == seed_vertex {
                    let key = if u < v { (u, v) } else { (v, u) };
                    if !incident_edges.contains(&key) {
                        incident_edges.push(key);
                    }
                }
            }
        }
        if incident_edges.is_empty() {
            if !extend {
                self.deselect_all();
            }
            if let Some(v) = self.verts.get_mut(seed_vertex as usize) {
                v.selected = true;
                return 1;
            }
            return 0;
        }
        let loop_edges = self.edge_loop(incident_edges[0]);
        if !extend {
            self.deselect_all();
        }
        let mut count = 0;
        for (u, v) in loop_edges {
            if let Some(vertex) = self.verts.get_mut(u as usize)
                && !vertex.selected
            {
                vertex.selected = true;
                count += 1;
            }
            if let Some(vertex) = self.verts.get_mut(v as usize)
                && !vertex.selected
            {
                vertex.selected = true;
                count += 1;
            }
        }
        count
    }

    /// Retorna os índices de faces que compõem um Face Loop (faixa contínua de quads) a partir de uma face semente.
    pub fn face_loop(&self, seed_face: usize, seed_edge: Option<(u32, u32)>) -> Vec<usize> {
        if seed_face >= self.faces.len() {
            return Vec::new();
        }
        let seed = &self.faces[seed_face];
        if seed.verts.len() != 4 {
            return vec![seed_face];
        }

        let mut edge_to_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            let m = f.verts.len();
            for k in 0..m {
                let u = f.verts[k];
                let v = f.verts[(k + 1) % m];
                let key = if u < v { (u, v) } else { (v, u) };
                edge_to_faces.entry(key).or_default().push(fi);
            }
        }

        let get_edge_key = |f: &Face, idx: usize| -> (u32, u32) {
            let u = f.verts[idx];
            let v = f.verts[(idx + 1) % f.verts.len()];
            if u < v { (u, v) } else { (v, u) }
        };

        let (start_edge_a, start_edge_b) = if let Some(se) = seed_edge {
            let norm_se = if se.0 < se.1 {
                (se.0, se.1)
            } else {
                (se.1, se.0)
            };
            if get_edge_key(seed, 0) == norm_se || get_edge_key(seed, 2) == norm_se {
                (get_edge_key(seed, 0), get_edge_key(seed, 2))
            } else {
                (get_edge_key(seed, 1), get_edge_key(seed, 3))
            }
        } else {
            (get_edge_key(seed, 0), get_edge_key(seed, 2))
        };

        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        result.push(seed_face);
        visited.insert(seed_face);

        let mut walk_dir = |mut current_face: usize, mut exit_edge: (u32, u32)| {
            let mut list = Vec::new();
            while let Some(neighbors) = edge_to_faces.get(&exit_edge) {
                let next_face = neighbors.iter().copied().find(|&fi| fi != current_face);
                let Some(next_fi) = next_face else {
                    break;
                };
                if visited.contains(&next_fi) {
                    break;
                }
                let next_f = &self.faces[next_fi];
                if next_f.verts.len() != 4 {
                    list.push(next_fi);
                    visited.insert(next_fi);
                    break;
                }
                list.push(next_fi);
                visited.insert(next_fi);

                let mut exit_idx = None;
                for k in 0..4 {
                    if get_edge_key(next_f, k) == exit_edge {
                        exit_idx = Some(k);
                        break;
                    }
                }
                let Some(entry_k) = exit_idx else {
                    break;
                };
                let opposite_k = (entry_k + 2) % 4;
                exit_edge = get_edge_key(next_f, opposite_k);
                current_face = next_fi;
            }
            list
        };

        let half_a = walk_dir(seed_face, start_edge_a);
        for fi in half_a {
            result.push(fi);
        }
        let half_b = walk_dir(seed_face, start_edge_b);
        for fi in half_b {
            result.push(fi);
        }

        result
    }

    /// Seleciona o Face Loop completo a partir de uma face semente. Retorna a contagem de faces selecionadas.
    pub fn select_face_loop(
        &mut self,
        seed_face: usize,
        seed_edge: Option<(u32, u32)>,
        extend: bool,
    ) -> usize {
        let loop_faces = self.face_loop(seed_face, seed_edge);
        let count = loop_faces.len();
        if !extend {
            self.deselect_all();
        }
        for &fi in &loop_faces {
            if let Some(f) = self.faces.get_mut(fi) {
                f.selected = true;
            }
        }
        self.sync_vert_selection_from_faces();
        count
    }

    pub fn sync_face_selection_from_verts(&mut self) {
        for f in &mut self.faces {
            f.selected =
                !f.verts.is_empty() && f.verts.iter().all(|&i| self.verts[i as usize].selected);
        }
    }
    pub fn sync_vert_selection_from_faces(&mut self) {
        for v in &mut self.verts {
            v.selected = false;
        }
        for f in &self.faces {
            if f.selected {
                for &i in &f.verts {
                    self.verts[i as usize].selected = true;
                }
            }
        }
    }
    /// Arestas totalmente selecionadas (ambos os extremos marcados).
    pub fn sync_edge_selection_from_verts(&mut self) {
        self.selected_edges.clear();
        for (a, b) in self.edges_unique() {
            if self.verts[a as usize].selected && self.verts[b as usize].selected {
                self.selected_edges.insert((a, b));
            }
        }
    }

    // ---------------- transformações ----------------

    pub fn translate_selected(&mut self, d: [f32; 3]) {
        let any = self.verts.iter().any(|v| v.selected);
        for v in &mut self.verts {
            if v.selected || !any {
                v.pos[0] += d[0];
                v.pos[1] += d[1];
                v.pos[2] += d[2];
            }
        }
    }

    pub fn scale_selected(&mut self, s: f32, center: [f32; 3]) {
        for v in &mut self.verts {
            if v.selected {
                for (p, c) in v.pos.iter_mut().zip(center.iter()) {
                    *p = c + (*p - c) * s;
                }
            }
        }
    }

    /// Escala por eixo (selecionados ou tudo, como `translate_selected`).
    pub fn scale_selected_factors(&mut self, f: [f32; 3], center: [f32; 3]) {
        let any = self.verts.iter().any(|v| v.selected);
        for v in &mut self.verts {
            if v.selected || !any {
                for ((p, c), s) in v.pos.iter_mut().zip(center.iter()).zip(f.iter()) {
                    *p = c + (*p - c) * s;
                }
            }
        }
    }

    /// Rotaciona por ângulos de Euler XYZ (radianos) ao redor de `center`
    /// (selecionados ou tudo, como `translate_selected`).
    pub fn rotate_selected(&mut self, euler_rad: [f32; 3], center: [f32; 3]) {
        use glam::EulerRot;
        let mat = glam::Mat3::from_euler(EulerRot::XYZ, euler_rad[0], euler_rad[1], euler_rad[2]);
        let c = Vec3::from(center);
        let any = self.verts.iter().any(|v| v.selected);
        for v in &mut self.verts {
            if v.selected || !any {
                v.pos = (c + mat * (v.vec() - c)).to_array();
            }
        }
    }

    pub fn selection_center(&self) -> [f32; 3] {
        let mut c = Vec3::ZERO;
        let mut n = 0;
        for v in &self.verts {
            if v.selected {
                c += v.vec();
                n += 1;
            }
        }
        if n == 0 {
            for v in &self.verts {
                c += v.vec();
            }
            n = self.verts.len().max(1);
        }
        (c / n as f32).to_array()
    }

    // ---------------- operações ----------------

    pub fn extrude_selected_result(&mut self, dist: f32) -> crate::TopologyResult {
        let vb = self.verts.len();
        let fb = self.faces.len();
        self.extrude_selected(dist);
        crate::TopologyResult::from_counts(vb, fb, self.verts.len(), self.faces.len())
    }

    pub fn extrude_selected(&mut self, dist: f32) {
        if !dist.is_finite() {
            return;
        }
        let sel: Vec<usize> = self
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.selected)
            .map(|(i, _)| i)
            .collect();
        if sel.is_empty() {
            return;
        }
        // A região compartilha vértices e apenas o contorno ganha paredes.
        // Distância zero é permitida para a preparação de previews modais.
        let mut normal = Vec3::ZERO;
        let mut edge_uses: HashMap<(u32, u32), usize> = HashMap::new();
        for &fi in &sel {
            let src = &self.faces[fi];
            if src.verts.len() < 3
                || src.uv.len() != src.verts.len()
                || src.verts.iter().any(|&vi| {
                    self.verts
                        .get(vi as usize)
                        .is_none_or(|v| !v.vec().is_finite())
                })
            {
                return;
            }
            normal += self.face_normal(fi);
            for k in 0..src.verts.len() {
                *edge_uses
                    .entry(edge_key(src.verts[k], src.verts[(k + 1) % src.verts.len()]))
                    .or_default() += 1;
            }
        }
        if edge_uses.values().any(|&count| count > 2) {
            return;
        }
        let delta = normal.normalize_or_zero() * dist;
        if !delta.is_finite() {
            return;
        }
        let mut remap = HashMap::new();
        let mut new_vertices = Vec::new();
        for &fi in &sel {
            for &vi in &self.faces[fi].verts {
                if let std::collections::hash_map::Entry::Vacant(entry) = remap.entry(vi) {
                    let mut v = self.verts[vi as usize].clone();
                    v.pos = (v.vec() + delta).to_array();
                    if !v.vec().is_finite() {
                        return;
                    }
                    v.selected = true;
                    let Ok(new_index) = u32::try_from(self.verts.len() + new_vertices.len()) else {
                        return;
                    };
                    entry.insert(new_index);
                    new_vertices.push(v);
                }
            }
        }
        self.verts.extend(new_vertices);
        for &fi in &sel {
            let src = self.faces[fi].clone();
            let new_idx: Vec<u32> = src.verts.iter().map(|vi| remap[vi]).collect();
            // Substitui a base: mantê-la criaria três faces na mesma aresta.
            self.faces[fi].verts = new_idx.clone();
            let m = src.verts.len();
            for k in 0..m {
                let k2 = (k + 1) % m;
                if edge_uses[&edge_key(src.verts[k], src.verts[k2])] != 1 {
                    continue;
                }
                // laterais: u ao longo do perímetro, v 0 embaixo / 1 em cima
                let side = Face::with_uv(
                    vec![src.verts[k], src.verts[k2], new_idx[k2], new_idx[k]],
                    vec![
                        [k as f32 / m as f32, 0.0],
                        [(k + 1) as f32 / m as f32, 0.0],
                        [(k + 1) as f32 / m as f32, 1.0],
                        [k as f32 / m as f32, 1.0],
                    ],
                );
                self.push_face(side);
            }
        }
        self.selected_edges.clear();
        self.remove_isolated_vertices();
        self.sync_vert_selection_from_faces();
    }

    /// Extrusão individual de cada face selecionada ao longo de sua própria normal (Alt+E).
    /// Gera prismas desacoplados sem compartilhar paredes laterais entre faces vizinhas.
    pub fn extrude_individual(&mut self, dist: f32) {
        if !dist.is_finite() {
            return;
        }
        let sel: Vec<usize> = self
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.selected)
            .map(|(i, _)| i)
            .collect();
        if sel.is_empty() {
            return;
        }

        for &fi in &sel {
            let src = self.faces[fi].clone();
            let m = src.verts.len();
            if m < 3 {
                continue;
            }
            let normal = self.face_normal(fi);
            let delta = normal.normalize_or_zero() * dist;

            // Cria novos vértices exclusivos para o topo desta face
            let mut top_verts = Vec::with_capacity(m);
            for &vi in &src.verts {
                let mut v = self.verts[vi as usize].clone();
                v.pos = (v.vec() + delta).to_array();
                v.selected = true;
                let new_idx = self.verts.len() as u32;
                self.verts.push(v);
                top_verts.push(new_idx);
            }

            // Atualiza a face original para ser a tampa superior extrudada
            self.faces[fi].verts = top_verts.clone();

            // Gera as paredes laterais conectando a base original ao novo topo
            for k in 0..m {
                let k2 = (k + 1) % m;
                let bottom0 = src.verts[k];
                let bottom1 = src.verts[k2];
                let top0 = top_verts[k];
                let top1 = top_verts[k2];

                let wall_face = Face::new(vec![bottom0, bottom1, top1, top0]);
                self.push_face(wall_face);
            }
        }

        self.selected_edges.clear();
        self.sync_vert_selection_from_faces();
    }

    /// Push/Pull: move a seleção ao longo da normal média (sem criar faces).
    pub fn push_pull(&mut self, dist: f32) {
        let mut n = Vec3::ZERO;
        let mut count = 0;
        for (i, f) in self.faces.iter().enumerate() {
            if f.selected {
                n += self.face_normal(i);
                count += 1;
            }
        }
        let dir = if count > 0 {
            (n / count as f32).normalize_or_zero()
        } else {
            Vec3::Y
        };
        let d = (dir * dist).to_array();
        self.translate_selected(d);
    }

    /// Inset métrico com proteção de auto-interseção e preservação de orientação topológica.
    pub fn inset_selected(&mut self, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let sel: Vec<usize> = self
            .faces
            .iter()
            .enumerate()
            .filter(|(_, x)| x.selected)
            .map(|(i, _)| i)
            .collect();
        for &fi in &sel {
            let src = self.faces[fi].clone();
            let m = src.verts.len();
            if m < 3 {
                continue;
            }
            let orig_normal = self.face_normal(fi);

            let mut c = Vec3::ZERO;
            let mut cuv = [0.0f32; 2];
            for (k, &vi) in src.verts.iter().enumerate() {
                c += self.verts[vi as usize].vec();
                cuv[0] += src.uv[k][0];
                cuv[1] += src.uv[k][1];
            }
            let mf = m as f32;
            c /= mf;
            cuv[0] /= mf;
            cuv[1] /= mf;

            // Ajusta o fator dinamicamente para prevenir auto-interseção ou inversão de winding
            let mut safe_f = factor.clamp(0.01, 0.95);
            for _ in 0..5 {
                let p0 = self.verts[src.verts[0] as usize].vec().lerp(c, safe_f);
                let p1 = self.verts[src.verts[1] as usize].vec().lerp(c, safe_f);
                let p2 = self.verts[src.verts[2] as usize].vec().lerp(c, safe_f);
                let inner_n = (p1 - p0).cross(p2 - p0);
                if inner_n.dot(orig_normal) > 1e-4 {
                    break;
                }
                safe_f *= 0.5;
            }

            let mut inner = Vec::with_capacity(m);
            let mut inner_uv = Vec::with_capacity(m);
            for (k, &vi) in src.verts.iter().enumerate() {
                let p = self.verts[vi as usize].vec().lerp(c, safe_f);
                let uv = [
                    src.uv[k][0] + (cuv[0] - src.uv[k][0]) * safe_f,
                    src.uv[k][1] + (cuv[1] - src.uv[k][1]) * safe_f,
                ];
                self.verts.push(Vertex {
                    pos: p.to_array(),
                    color: self.verts[vi as usize].color,
                    selected: true,
                });
                inner.push((self.verts.len() - 1) as u32);
                inner_uv.push(uv);
            }
            self.faces[fi].verts = inner.clone();
            self.faces[fi].uv = inner_uv;

            for k in 0..m {
                let k2 = (k + 1) % m;
                let quad_uv = vec![
                    src.uv[k],
                    src.uv[k2],
                    self.faces[fi].uv[k2],
                    self.faces[fi].uv[k],
                ];
                let f_new = Face::with_uv(
                    vec![src.verts[k], src.verts[k2], inner[k2], inner[k]],
                    quad_uv,
                );
                self.push_face(f_new);
            }
        }
        self.selected_edges.clear();
        self.sync_vert_selection_from_faces();
    }

    /// Subdivide selected triangle/quad faces with one uniform cut per edge.
    ///
    /// This is the compatibility entry point used by older commands. The actual
    /// implementation is `subdivide_selected_cuts`, which treats `cuts` as the
    /// number of inserted edge cuts on the original face (not recursive levels).
    pub fn subdivide_selected(&mut self) {
        self.subdivide_selected_cuts(1);
    }

    /// Subdivide selected triangle/quad faces using exactly `cuts` uniformly
    /// spaced cuts on every original edge.
    ///
    /// A quad produces `(cuts + 1)^2` quads. A triangle produces
    /// `(cuts + 1)^2` triangles. Boundary split vertices are shared between
    /// adjacent selected faces so the selected region does not crack.
    pub fn subdivide_selected_cuts(&mut self, cuts: u32) {
        let segments = cuts.clamp(1, 64) + 1;
        let selected_faces: Vec<usize> = self
            .faces
            .iter()
            .enumerate()
            .filter(|(_, face)| face.selected)
            .map(|(index, _)| index)
            .collect();
        if selected_faces.is_empty() {
            return;
        }

        let mut edge_vertices: HashMap<(u32, u32, u32), u32> = HashMap::new();
        let mut replacement_faces = Vec::new();
        let mut remove_faces = Vec::new();

        for face_index in selected_faces {
            let source = self.faces[face_index].clone();
            if source.uv.len() != source.verts.len() {
                continue;
            }

            match source.verts.as_slice() {
                [a, b, c, d] => {
                    let [a, b, c, d] = [*a, *b, *c, *d];
                    let side = (segments + 1) as usize;
                    let mut grid = vec![vec![0_u32; side]; side];

                    let pa = self.verts[a as usize].vec();
                    let pb = self.verts[b as usize].vec();
                    let pc = self.verts[c as usize].vec();
                    let pd = self.verts[d as usize].vec();
                    let ca = self.verts[a as usize].color;
                    let cb = self.verts[b as usize].color;
                    let cc = self.verts[c as usize].color;
                    let cd = self.verts[d as usize].color;

                    for row in 0..=segments {
                        for column in 0..=segments {
                            let vertex_index = if row == 0 {
                                self.subdivide_edge_vertex(
                                    &mut edge_vertices,
                                    a,
                                    b,
                                    column,
                                    segments,
                                )
                            } else if column == segments {
                                self.subdivide_edge_vertex(&mut edge_vertices, b, c, row, segments)
                            } else if row == segments {
                                self.subdivide_edge_vertex(
                                    &mut edge_vertices,
                                    d,
                                    c,
                                    column,
                                    segments,
                                )
                            } else if column == 0 {
                                self.subdivide_edge_vertex(&mut edge_vertices, a, d, row, segments)
                            } else {
                                let u = column as f32 / segments as f32;
                                let v = row as f32 / segments as f32;
                                let top = pa.lerp(pb, u);
                                let bottom = pd.lerp(pc, u);
                                let position = top.lerp(bottom, v);
                                let top_color = [
                                    ca[0] + (cb[0] - ca[0]) * u,
                                    ca[1] + (cb[1] - ca[1]) * u,
                                    ca[2] + (cb[2] - ca[2]) * u,
                                ];
                                let bottom_color = [
                                    cd[0] + (cc[0] - cd[0]) * u,
                                    cd[1] + (cc[1] - cd[1]) * u,
                                    cd[2] + (cc[2] - cd[2]) * u,
                                ];
                                let color = [
                                    top_color[0] + (bottom_color[0] - top_color[0]) * v,
                                    top_color[1] + (bottom_color[1] - top_color[1]) * v,
                                    top_color[2] + (bottom_color[2] - top_color[2]) * v,
                                ];
                                let index = self.verts.len() as u32;
                                self.verts.push(Vertex {
                                    pos: position.to_array(),
                                    color,
                                    selected: true,
                                });
                                index
                            };
                            grid[row as usize][column as usize] = vertex_index;
                        }
                    }

                    let uv_at = |column: u32, row: u32| {
                        let u = column as f32 / segments as f32;
                        let v = row as f32 / segments as f32;
                        let top = [
                            source.uv[0][0] + (source.uv[1][0] - source.uv[0][0]) * u,
                            source.uv[0][1] + (source.uv[1][1] - source.uv[0][1]) * u,
                        ];
                        let bottom = [
                            source.uv[3][0] + (source.uv[2][0] - source.uv[3][0]) * u,
                            source.uv[3][1] + (source.uv[2][1] - source.uv[3][1]) * u,
                        ];
                        [
                            top[0] + (bottom[0] - top[0]) * v,
                            top[1] + (bottom[1] - top[1]) * v,
                        ]
                    };

                    for row in 0..segments {
                        for column in 0..segments {
                            let mut face = Face::with_uv(
                                vec![
                                    grid[row as usize][column as usize],
                                    grid[row as usize][(column + 1) as usize],
                                    grid[(row + 1) as usize][(column + 1) as usize],
                                    grid[(row + 1) as usize][column as usize],
                                ],
                                vec![
                                    uv_at(column, row),
                                    uv_at(column + 1, row),
                                    uv_at(column + 1, row + 1),
                                    uv_at(column, row + 1),
                                ],
                            );
                            face.selected = true;
                            face.material_slot = source.material_slot;
                            replacement_faces.push(face);
                        }
                    }
                    remove_faces.push(face_index);
                }
                [a, b, c] => {
                    let [a, b, c] = [*a, *b, *c];
                    let mut grid: HashMap<(u32, u32), u32> = HashMap::new();
                    let pa = self.verts[a as usize].vec();
                    let pb = self.verts[b as usize].vec();
                    let pc = self.verts[c as usize].vec();
                    let ca = self.verts[a as usize].color;
                    let cb = self.verts[b as usize].color;
                    let cc = self.verts[c as usize].color;

                    for i in 0..=segments {
                        for j in 0..=segments - i {
                            let vertex_index = if j == 0 {
                                self.subdivide_edge_vertex(&mut edge_vertices, a, b, i, segments)
                            } else if i == 0 {
                                self.subdivide_edge_vertex(&mut edge_vertices, a, c, j, segments)
                            } else if i + j == segments {
                                self.subdivide_edge_vertex(&mut edge_vertices, b, c, j, segments)
                            } else {
                                let wb = i as f32 / segments as f32;
                                let wc = j as f32 / segments as f32;
                                let wa = 1.0 - wb - wc;
                                let position = pa * wa + pb * wb + pc * wc;
                                let color = [
                                    ca[0] * wa + cb[0] * wb + cc[0] * wc,
                                    ca[1] * wa + cb[1] * wb + cc[1] * wc,
                                    ca[2] * wa + cb[2] * wb + cc[2] * wc,
                                ];
                                let index = self.verts.len() as u32;
                                self.verts.push(Vertex {
                                    pos: position.to_array(),
                                    color,
                                    selected: true,
                                });
                                index
                            };
                            grid.insert((i, j), vertex_index);
                        }
                    }

                    let uv_at = |i: u32, j: u32| {
                        let wb = i as f32 / segments as f32;
                        let wc = j as f32 / segments as f32;
                        let wa = 1.0 - wb - wc;
                        [
                            source.uv[0][0] * wa + source.uv[1][0] * wb + source.uv[2][0] * wc,
                            source.uv[0][1] * wa + source.uv[1][1] * wb + source.uv[2][1] * wc,
                        ]
                    };

                    for i in 0..segments {
                        for j in 0..segments - i {
                            let mut first = Face::with_uv(
                                vec![grid[&(i, j)], grid[&(i + 1, j)], grid[&(i, j + 1)]],
                                vec![uv_at(i, j), uv_at(i + 1, j), uv_at(i, j + 1)],
                            );
                            first.selected = true;
                            first.material_slot = source.material_slot;
                            replacement_faces.push(first);

                            if i + j < segments - 1 {
                                let mut second = Face::with_uv(
                                    vec![
                                        grid[&(i + 1, j)],
                                        grid[&(i + 1, j + 1)],
                                        grid[&(i, j + 1)],
                                    ],
                                    vec![uv_at(i + 1, j), uv_at(i + 1, j + 1), uv_at(i, j + 1)],
                                );
                                second.selected = true;
                                second.material_slot = source.material_slot;
                                replacement_faces.push(second);
                            }
                        }
                    }
                    remove_faces.push(face_index);
                }
                _ => {}
            }
        }

        remove_faces.sort_unstable_by(|left, right| right.cmp(left));
        for face_index in remove_faces {
            self.faces.remove(face_index);
        }
        for face in replacement_faces {
            self.push_face(face);
        }
        self.selected_edges.clear();
        self.sync_vert_selection_from_faces();
    }

    fn subdivide_edge_vertex(
        &mut self,
        cache: &mut HashMap<(u32, u32, u32), u32>,
        a: u32,
        b: u32,
        step: u32,
        segments: u32,
    ) -> u32 {
        if step == 0 {
            return a;
        }
        if step >= segments {
            return b;
        }

        let (low, high) = edge_key(a, b);
        let canonical_step = if a == low { step } else { segments - step };
        let key = (low, high, canonical_step);
        if let Some(&index) = cache.get(&key) {
            return index;
        }

        let t = step as f32 / segments as f32;
        let start = &self.verts[a as usize];
        let end = &self.verts[b as usize];
        let position = start.vec().lerp(end.vec(), t);
        let color = [
            start.color[0] + (end.color[0] - start.color[0]) * t,
            start.color[1] + (end.color[1] - start.color[1]) * t,
            start.color[2] + (end.color[2] - start.color[2]) * t,
        ];
        let index = self.verts.len() as u32;
        self.verts.push(Vertex {
            pos: position.to_array(),
            color,
            selected: true,
        });
        cache.insert(key, index);
        index
    }

    /// Chanfra uma aresta convexa manifold com extremidades trivalentes (1 segmento padrão).
    pub fn bevel_selected(&mut self, amount: f32) -> (usize, usize) {
        self.bevel_selected_full(amount, 1, true)
    }

    /// Chanfra uma aresta convexa manifold com suporte a multi-segmentos para filetagem arredondada.
    pub fn bevel_selected_segments(&mut self, amount: f32, segments: u32) -> (usize, usize) {
        self.bevel_selected_full(amount, segments, true)
    }

    /// Chanfra arestas ou vértices manifold selecionados com multi-segmentos e clamp de overlap opcional (P3D-044).
    pub fn bevel_selected_full(
        &mut self,
        amount: f32,
        segments: u32,
        clamp_overlap: bool,
    ) -> (usize, usize) {
        let count = self.selected_edges.len();
        if count == 0 {
            self.bevel_selected_vertex(amount, clamp_overlap)
        } else {
            if !amount.is_finite() || amount <= 0.0 {
                return (0, count);
            }
            if count == 1 {
                let Some(&(a, b)) = self.selected_edges.iter().next() else {
                    return (0, 0);
                };
                match crate::bevel::bevel_edge_segments_clamped(
                    self,
                    a,
                    b,
                    amount,
                    segments,
                    clamp_overlap,
                ) {
                    Some(mesh) => {
                        *self = mesh;
                        (1, 0)
                    }
                    None => (0, 1),
                }
            } else {
                let target_edge_positions: Vec<([f32; 3], [f32; 3])> = self
                    .selected_edges
                    .iter()
                    .map(|&(a, b)| (self.verts[a as usize].pos, self.verts[b as usize].pos))
                    .collect();

                let mut working = self.clone();
                let mut applied = 0;

                for (pos_a, pos_b) in target_edge_positions {
                    let va = working.verts.iter().position(|v| {
                        (v.pos[0] - pos_a[0])
                            .hypot(v.pos[1] - pos_a[1])
                            .hypot(v.pos[2] - pos_a[2])
                            < 1e-4
                    });
                    let vb = working.verts.iter().position(|v| {
                        (v.pos[0] - pos_b[0])
                            .hypot(v.pos[1] - pos_b[1])
                            .hypot(v.pos[2] - pos_b[2])
                            < 1e-4
                    });
                    let (Some(va), Some(vb)) = (va, vb) else {
                        return (0, count);
                    };
                    match crate::bevel::bevel_edge_segments_clamped(
                        &working,
                        va as u32,
                        vb as u32,
                        amount,
                        segments,
                        clamp_overlap,
                    ) {
                        Some(next) => {
                            working = next;
                            applied += 1;
                        }
                        None => {
                            return (0, count);
                        }
                    }
                }

                *self = working;
                (applied, 0)
            }
        }
    }

    /// Chanfra vértices manifold selecionados com clamp de overlap opcional (P3D-044).
    pub fn bevel_selected_vertex(&mut self, amount: f32, clamp_overlap: bool) -> (usize, usize) {
        let sel_verts: Vec<u32> = self
            .verts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.selected)
            .map(|(i, _)| i as u32)
            .collect();
        let total = sel_verts.len();
        if total == 0 || !amount.is_finite() || amount <= 0.0 {
            return (0, total);
        }

        let target_positions: Vec<[f32; 3]> = sel_verts
            .iter()
            .map(|&vi| self.verts[vi as usize].pos)
            .collect();

        let mut working = self.clone();
        let mut applied = 0;

        for target_pos in target_positions {
            let found = working.verts.iter().position(|v| {
                (v.pos[0] - target_pos[0])
                    .hypot(v.pos[1] - target_pos[1])
                    .hypot(v.pos[2] - target_pos[2])
                    < 1e-4
            });
            let Some(vi) = found else {
                return (0, total);
            };
            match crate::bevel::bevel_vertex(&working, vi as u32, amount, clamp_overlap) {
                Some(next) => {
                    working = next;
                    applied += 1;
                }
                None => {
                    return (0, total);
                }
            }
        }

        *self = working;
        (applied, 0)
    }

    /// Inverte a diagonal de triangulação interna de quads selecionados (rotacionando o fan de corte),
    /// ou executa um edge-flip em aresta selecionada compartilhada por dois triângulos adjacentes.
    pub fn flip_diagonal(&mut self) -> bool {
        let mut modified = false;

        // 1. Quads selecionados: rotação cíclica inverte a diagonal interna do fan
        for face in &mut self.faces {
            if face.selected && face.verts.len() == 4 {
                face.verts.rotate_left(1);
                if face.uv.len() == 4 {
                    face.uv.rotate_left(1);
                }
                modified = true;
            }
        }
        if modified {
            return true;
        }

        // 2. Aresta selecionada entre dois triângulos adjacentes (Delaunay Edge Flip)
        let edge = if let Some(&(u, v)) = self.selected_edges.iter().next() {
            Some((u, v))
        } else {
            let sel_verts: Vec<u32> = self
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .map(|(i, _)| i as u32)
                .collect();
            if sel_verts.len() == 2 {
                Some((sel_verts[0], sel_verts[1]))
            } else {
                None
            }
        };

        if let Some((u, v)) = edge {
            let adj = self.edge_faces(u, v);
            if adj.len() == 2 {
                let f0 = &self.faces[adj[0]];
                let f1 = &self.faces[adj[1]];
                if f0.verts.len() == 3 && f1.verts.len() == 3 {
                    let w0 = f0.verts.iter().copied().find(|&x| x != u && x != v);
                    let w1 = f1.verts.iter().copied().find(|&x| x != u && x != v);
                    if let (Some(w0), Some(w1)) = (w0, w1)
                        && w0 != w1
                    {
                        // Guard against non-manifold edge vertex indexing to prevent runtime panics.
                        // Proteção contra indexação de vértice não-manifold para prevenir pânicos em tempo de execução.
                        let Some(p_u) = f0.verts.iter().position(|&x| x == u) else {
                            return false;
                        };
                        let next_u = f0.verts[(p_u + 1) % 3];
                        let (new_f0, new_f1) = if next_u == v {
                            (vec![w0, u, w1], vec![w1, v, w0])
                        } else {
                            (vec![w0, w1, u], vec![w1, w0, v])
                        };

                        self.faces[adj[0]] = Face::new(new_f0);
                        self.faces[adj[1]] = Face::new(new_f1);
                        self.faces[adj[0]].selected = true;
                        self.faces[adj[1]].selected = true;
                        self.selected_edges.clear();
                        self.selected_edges.insert(crate::edge_key(w0, w1));
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Revoluciona a seleção de vértices/arestas em torno de um eixo (0: X, 1: Y, 2: Z)
    /// passando por `center` com `segments` subdivisões ao longo de `angle_deg` graus.
    pub fn revolve_selection(
        &mut self,
        segments: u32,
        angle_deg: f32,
        axis: usize,
        center: [f32; 3],
    ) -> bool {
        let sel_indices: Vec<u32> = self
            .verts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.selected)
            .map(|(i, _)| i as u32)
            .collect();
        if sel_indices.len() < 2 {
            return false;
        }

        let seg = segments.clamp(3, 64) as usize;
        let total_rad = angle_deg.to_radians();
        let is_full_circle = (angle_deg.abs() - 360.0).abs() < 1e-3;
        let c = Vec3::from(center);
        let ax = axis.min(2);

        let rotate_pt = |pt: Vec3, rad: f32| -> Vec3 {
            let rel = pt - c;
            let (cos, sin) = (rad.cos(), rad.sin());
            let rot = match ax {
                0 => Vec3::new(rel.x, rel.y * cos - rel.z * sin, rel.y * sin + rel.z * cos),
                1 => Vec3::new(rel.x * cos + rel.z * sin, rel.y, -rel.x * sin + rel.z * cos),
                _ => Vec3::new(rel.x * cos - rel.y * sin, rel.x * sin + rel.y * cos, rel.z),
            };
            rot + c
        };

        let mut edges_to_revolve = Vec::new();
        for &(u, v) in &self.selected_edges {
            if sel_indices.contains(&u) && sel_indices.contains(&v) {
                edges_to_revolve.push((u, v));
            }
        }
        if edges_to_revolve.is_empty() {
            for i in 0..sel_indices.len() - 1 {
                edges_to_revolve.push((sel_indices[i], sel_indices[i + 1]));
            }
        }
        if edges_to_revolve.is_empty() {
            return false;
        }

        let steps = if is_full_circle { seg } else { seg + 1 };
        let mut rings: Vec<Vec<u32>> = Vec::with_capacity(sel_indices.len());

        for &vi in &sel_indices {
            let orig_pos = self.verts[vi as usize].vec();
            let mut ring = Vec::with_capacity(steps);
            ring.push(vi);
            for s in 1..steps {
                let frac = s as f32 / seg as f32;
                let angle = frac * total_rad;
                let new_pos = rotate_pt(orig_pos, angle);
                let mut v = self.verts[vi as usize].clone();
                v.pos = new_pos.to_array();
                v.selected = true;
                let new_idx = self.verts.len() as u32;
                self.verts.push(v);
                ring.push(new_idx);
            }
            rings.push(ring);
        }

        let vert_to_ring_idx: HashMap<u32, usize> = sel_indices
            .iter()
            .enumerate()
            .map(|(r_idx, &v_idx)| (v_idx, r_idx))
            .collect();

        for (u, v) in edges_to_revolve {
            let Some(&ring_u_idx) = vert_to_ring_idx.get(&u) else {
                continue;
            };
            let Some(&ring_v_idx) = vert_to_ring_idx.get(&v) else {
                continue;
            };
            let ring_u = &rings[ring_u_idx];
            let ring_v = &rings[ring_v_idx];

            for s in 0..seg {
                let u0 = ring_u[s];
                let u1 = if is_full_circle && s + 1 == seg {
                    ring_u[0]
                } else {
                    ring_u[s + 1]
                };
                let v0 = ring_v[s];
                let v1 = if is_full_circle && s + 1 == seg {
                    ring_v[0]
                } else {
                    ring_v[s + 1]
                };

                let mut face = Face::new(vec![u0, u1, v1, v0]);
                face.selected = true;
                self.push_face(face);
            }
        }

        self.selected_edges.clear();
        self.sync_vert_selection_from_faces();
        true
    }

    /// Mirror da seleção (ou tudo) no eixo, com weld opcional no plano.
    pub fn mirror(&mut self, axis: usize, weld_eps: f32) {
        let axis = axis.min(2);
        let any_sel = self.verts.iter().any(|v| v.selected);
        // coleta antes (não itera + empurra ao mesmo tempo)
        let src: Vec<(u32, Vertex)> = self
            .verts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.selected || !any_sel)
            .map(|(i, v)| (i as u32, v.clone()))
            .collect();
        let mut remap: HashMap<u32, u32> = HashMap::new();
        for (i, v) in src {
            let mut nv = v;
            nv.pos[axis] = -nv.pos[axis];
            self.verts.push(nv);
            remap.insert(i, (self.verts.len() - 1) as u32);
        }
        let mut nf = Vec::new();
        for f in &self.faces {
            let all = f.verts.iter().all(|i| remap.contains_key(i));
            let any_face = any_sel && f.selected;
            if (any_sel && any_face) || (!any_sel && all) {
                let mut verts: Vec<u32> =
                    f.verts.iter().map(|i| *remap.get(i).unwrap_or(i)).collect();
                verts.reverse(); // espelho inverte winding
                let mut q = Face::with_uv(verts, f.uv.iter().rev().copied().collect());
                q.selected = true;
                nf.push(q);
            }
        }
        for f in nf {
            self.push_face(f);
        }
        if weld_eps > 0.0 {
            self.weld(weld_eps);
        }
        self.sync_vert_selection_from_faces();
    }

    /// Funde vértices duplicados dentro de `eps` (O(n²), ok p/ low-poly).
    /// Índices manuais são necessários (mutação durante iteração).
    #[allow(clippy::needless_range_loop)]
    pub fn weld(&mut self, eps: f32) {
        let n = self.verts.len();
        let mut rep: Vec<u32> = (0..n as u32).collect();
        for i in 0..n {
            if rep[i] != i as u32 {
                continue;
            }
            for j in (i + 1)..n {
                if rep[j] != j as u32 {
                    continue;
                }
                let d = (self.verts[i].vec() - self.verts[j].vec()).length();
                if d <= eps {
                    rep[j] = i as u32;
                    self.verts[i].selected |= self.verts[j].selected;
                }
            }
        }
        // compacta
        let mut new_id = vec![u32::MAX; n];
        let mut nv = Vec::new();
        for i in 0..n {
            if rep[i] == i as u32 {
                new_id[i] = nv.len() as u32;
                nv.push(self.verts[i].clone());
            }
        }
        for i in 0..n {
            if rep[i] != i as u32 {
                new_id[i] = new_id[rep[i] as usize];
            }
        }
        for f in &mut self.faces {
            for vi in &mut f.verts {
                *vi = new_id[*vi as usize];
            }
        }
        self.faces.retain(|f| {
            let mut u = f.verts.clone();
            u.sort_unstable();
            u.dedup();
            u.len() >= 3
        });
        self.verts = nv;
        // remapeia arestas selecionadas
        let edges: Vec<(u32, u32)> = self.selected_edges.iter().copied().collect();
        self.selected_edges.clear();
        for (a, b) in edges {
            let (na, nb) = (new_id[a as usize], new_id[b as usize]);
            if na != u32::MAX && nb != u32::MAX && na != nb {
                self.selected_edges.insert(edge_key(na, nb));
            }
        }
    }

    /// Funde duplicados e retorna quantos vértices foram removidos.
    pub fn weld_merged_count(&mut self, eps: f32) -> usize {
        let before = self.verts.len();
        self.weld(eps.max(0.0));
        before.saturating_sub(self.verts.len())
    }

    /// Symmetrize (§8.11): copia um lado para o outro e solda a costura.
    ///
    /// `positive_to_negative` = true copia +eixo para −eixo (descarta o lado
    /// negativo); false faz o inverso. Vértices com `|coord| <= eps` são
    /// considerados sobre o plano, mantidos e snapados para 0 no eixo.
    /// Retorna quantos vértices foram espelhados. No-op (0) sem lado fonte,
    /// para nunca apagar a malha por direção vazia.
    pub fn symmetrize(&mut self, axis: usize, positive_to_negative: bool, eps: f32) -> usize {
        let ax = axis.min(2);
        let eps = eps.max(0.0);
        let center_eps = if eps > 0.0 { eps } else { 1e-5 };
        let n = self.verts.len();
        if n == 0 {
            return 0;
        }
        let mut new_id: Vec<Option<u32>> = vec![None; n];
        let mut is_source = vec![false; n];
        let mut new_verts: Vec<Vertex> = Vec::with_capacity(n * 2);
        for (i, v) in self.verts.iter().enumerate() {
            let c = v.pos[ax];
            let is_center = c.abs() <= center_eps;
            let on_source = if positive_to_negative {
                c > center_eps
            } else {
                c < -center_eps
            };
            if is_center || on_source {
                let mut nv = v.clone();
                if is_center {
                    nv.pos[ax] = 0.0;
                }
                new_id[i] = Some(new_verts.len() as u32);
                is_source[i] = on_source && !is_center;
                new_verts.push(nv);
            }
        }
        let source_count = is_source.iter().filter(|&&s| s).count();
        if source_count == 0 {
            return 0;
        }
        // Espelha os vértices fonte.
        let mut mirror_of: Vec<Option<u32>> = vec![None; n];
        for i in 0..n {
            if is_source[i] {
                let orig_new = new_id[i].expect("fonte sempre mantida") as usize;
                let mut mv = new_verts[orig_new].clone();
                mv.pos[ax] = -mv.pos[ax];
                mirror_of[i] = Some(new_verts.len() as u32);
                new_verts.push(mv);
            }
        }
        // Faces mantidas + cópias espelhadas das que tocam o lado fonte.
        let mut kept: Vec<Face> = Vec::new();
        let mut mirrored: Vec<Face> = Vec::new();
        for f in &self.faces {
            if f.verts
                .iter()
                .all(|vi| (*vi as usize) < n && new_id[*vi as usize].is_some())
            {
                let mut nf = Face::with_uv(
                    f.verts
                        .iter()
                        .map(|vi| new_id[*vi as usize].expect("vert mantido"))
                        .collect(),
                    f.uv.clone(),
                );
                nf.selected = f.selected;
                nf.material_slot = f.material_slot;
                kept.push(nf);
                let touches_source = f.verts.iter().any(|vi| is_source[*vi as usize]);
                if touches_source {
                    let mut mverts: Vec<u32> = f
                        .verts
                        .iter()
                        .map(|vi| {
                            let idx = *vi as usize;
                            if is_source[idx] {
                                mirror_of[idx].expect("espelho da fonte")
                            } else {
                                new_id[idx].expect("centro mantido")
                            }
                        })
                        .collect();
                    mverts.reverse();
                    let mut muv = f.uv.clone();
                    muv.reverse();
                    let mut mf = Face::with_uv(mverts, muv);
                    mf.selected = f.selected;
                    mf.material_slot = f.material_slot;
                    mirrored.push(mf);
                }
            }
        }
        self.verts = new_verts;
        kept.extend(mirrored);
        self.faces = kept;
        if eps > 0.0 {
            self.weld(eps);
        }
        self.selected_edges.clear();
        self.sync_vert_selection_from_faces();
        source_count
    }

    pub fn merge_center(&mut self) {
        let sel: Vec<u32> = self
            .verts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.selected)
            .map(|(i, _)| i as u32)
            .collect();
        if sel.len() < 2 {
            return;
        }
        let mut c = Vec3::ZERO;
        for &i in &sel {
            c += self.verts[i as usize].vec();
        }
        c /= sel.len() as f32;
        let keep = sel[0] as usize;
        self.verts[keep].pos = c.to_array();
        self.verts[keep].selected = true;
        let keep_orig = sel[0] as usize;
        let dead_set: std::collections::HashSet<usize> =
            sel.iter().skip(1).map(|&i| i as usize).collect();
        let mut remap = vec![0u32; self.verts.len()];
        let mut new_verts = Vec::with_capacity(self.verts.len() - dead_set.len());

        for (i, v) in self.verts.iter().enumerate() {
            if dead_set.contains(&i) {
                continue;
            }
            let new_idx = new_verts.len() as u32;
            new_verts.push(v.clone());
            remap[i] = new_idx;
        }
        let keep_new = remap[keep_orig];
        for &d in &dead_set {
            remap[d] = keep_new;
        }

        self.verts = new_verts;
        for f in &mut self.faces {
            for vi in &mut f.verts {
                if let Some(&new_i) = remap.get(*vi as usize) {
                    *vi = new_i;
                }
            }
        }
        self.faces.retain(|f| {
            let mut uniq = f.verts.clone();
            uniq.sort_unstable();
            uniq.dedup();
            uniq.len() >= 3
        });
        for f in &mut self.faces {
            f.fix_uv();
        }
        self.selected_edges.clear();
    }

    pub fn delete_selected(&mut self) {
        if self.faces.iter().any(|f| f.selected) {
            self.faces = self.faces.iter().filter(|f| !f.selected).cloned().collect();
            let mut used = vec![false; self.verts.len()];
            for f in &self.faces {
                for &i in &f.verts {
                    used[i as usize] = true;
                }
            }
            let mut remap = vec![0u32; self.verts.len()];
            let mut nv = Vec::new();
            for (i, v) in self.verts.iter().enumerate() {
                if used[i] {
                    remap[i] = nv.len() as u32;
                    nv.push(v.clone());
                }
            }
            for f in &mut self.faces {
                for vi in &mut f.verts {
                    *vi = remap[*vi as usize];
                }
            }
            self.verts = nv;
        } else {
            let mut remap = vec![u32::MAX; self.verts.len()];
            let mut nv = Vec::new();
            for (i, v) in self.verts.iter().enumerate() {
                if !v.selected {
                    remap[i] = nv.len() as u32;
                    nv.push(v.clone());
                }
            }
            self.faces
                .retain(|f| f.verts.iter().all(|&i| remap[i as usize] != u32::MAX));
            for f in &mut self.faces {
                for vi in &mut f.verts {
                    *vi = remap[*vi as usize];
                }
            }
            self.verts = nv;
        }
        self.deselect_all();
    }

    pub fn duplicate_selected(&mut self) {
        let sel: Vec<(u32, Vertex)> = self
            .verts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.selected)
            .map(|(i, v)| (i as u32, v.clone()))
            .collect();
        let mut remap: HashMap<u32, u32> = HashMap::new();
        for (i, v) in sel {
            let mut nv = v;
            nv.pos[0] += 0.3;
            self.verts.push(nv);
            remap.insert(i, (self.verts.len() - 1) as u32);
        }
        let mut nf = Vec::new();
        for f in &self.faces {
            if f.selected {
                let verts: Vec<u32> = f.verts.iter().map(|i| *remap.get(i).unwrap_or(i)).collect();
                let mut q = Face::with_uv(verts, f.uv.clone());
                q.selected = true;
                nf.push(q);
            }
        }
        for f in &mut self.faces {
            f.selected = false;
        }
        for f in nf {
            self.push_face(f);
        }
        self.sync_vert_selection_from_faces();
    }

    pub fn triangulate(&mut self) {
        let mut out = Vec::new();
        for (fi, face) in self.faces.iter().enumerate() {
            if face.verts.len() <= 3 {
                out.push(face.clone());
                continue;
            }
            let corners = self.face_triangle_corners(fi);
            // Never discard an unsupported polygon silently.
            if corners.len() != face.verts.len() - 2 {
                out.push(face.clone());
                continue;
            }
            for triangle in corners {
                let mut split = face.clone();
                split.verts = triangle.map(|i| face.verts[i]).to_vec();
                split.uv = triangle
                    .map(|i| face.uv.get(i).copied().unwrap_or_default())
                    .to_vec();
                out.push(split);
            }
        }
        self.faces = out;
    }

    // ---------------- novas operações geométricas ----------------

    /// Inverte o sentido de enrolamento (winding) e as normais das faces selecionadas
    /// (ou de todas as faces se nenhuma estiver selecionada).
    pub fn flip_normals(&mut self) {
        let any_sel = self.faces.iter().any(|f| f.selected);
        for f in &mut self.faces {
            if f.selected || !any_sel {
                f.verts.reverse();
                f.uv.reverse();
            }
        }
    }

    /// Recalcula e unifica a orientação das faces em componentes conexos
    /// para que as normais apontem consistentemente para fora (volume positivo).
    pub fn recalculate_normals(&mut self) {
        if self.faces.is_empty() {
            return;
        }

        // 1. Mapeia arestas direcionadas para faces
        let mut edge_to_face: HashMap<(u32, u32), usize> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            let m = f.verts.len();
            for k in 0..m {
                let u = f.verts[k];
                let v = f.verts[(k + 1) % m];
                edge_to_face.insert((u, v), fi);
            }
        }

        // 2. BFS para unificar orientação por componente
        let mut visited = vec![false; self.faces.len()];
        for start_fi in 0..self.faces.len() {
            if visited[start_fi] {
                continue;
            }

            let mut component = Vec::new();
            let mut queue = std::collections::VecDeque::new();
            visited[start_fi] = true;
            queue.push_back(start_fi);
            component.push(start_fi);

            while let Some(fi) = queue.pop_front() {
                let m = self.faces[fi].verts.len();
                for k in 0..m {
                    let u = self.faces[fi].verts[k];
                    let v = self.faces[fi].verts[(k + 1) % m];

                    // Face vizinha orientada no sentido correto deve ter aresta (v, u)
                    if let Some(&adj_fi) = edge_to_face.get(&(v, u)) {
                        if !visited[adj_fi] {
                            visited[adj_fi] = true;
                            queue.push_back(adj_fi);
                            component.push(adj_fi);
                        }
                    } else if let Some(&adj_fi) = edge_to_face.get(&(u, v)) {
                        // Face vizinha está com orientação invertida: precisa inverter
                        if !visited[adj_fi] {
                            visited[adj_fi] = true;
                            self.faces[adj_fi].verts.reverse();
                            self.faces[adj_fi].uv.reverse();
                            queue.push_back(adj_fi);
                            component.push(adj_fi);
                        }
                    }
                }
            }

            // 3. Testa se o volume do componente é negativo (dentro para fora)
            let mut signed_volume = 0.0f32;
            for &fi in &component {
                let f = &self.faces[fi];
                if f.verts.len() >= 3 {
                    let p0 = self.verts[f.verts[0] as usize].vec();
                    for k in 1..(f.verts.len() - 1) {
                        let p1 = self.verts[f.verts[k] as usize].vec();
                        let p2 = self.verts[f.verts[k + 1] as usize].vec();
                        signed_volume += p0.dot(p1.cross(p2));
                    }
                }
            }

            // Se o volume assinado for negativo, inverte todas as faces do componente
            if signed_volume < -1e-5 {
                for &fi in &component {
                    self.faces[fi].verts.reverse();
                    self.faces[fi].uv.reverse();
                }
            }
        }
    }

    /// Dissolve elementos selecionados (arestas compartilhadas, faces adjacentes ou vértices redundantes)
    /// sem deixar buracos na geometria.
    pub fn dissolve_selected(&mut self) {
        // 1. Se não houver arestas selecionadas explicitamente, checa faces selecionadas
        if self.selected_edges.is_empty() {
            let sel_faces: Vec<usize> = self
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.selected)
                .map(|(i, _)| i)
                .collect();

            if sel_faces.len() >= 2 {
                for &(a, b) in &self.edges_unique() {
                    let adj = self.edge_faces(a, b);
                    if adj.len() == 2 && sel_faces.contains(&adj[0]) && sel_faces.contains(&adj[1])
                    {
                        self.selected_edges.insert((a.min(b), a.max(b)));
                    }
                }
            }
        }

        // 2. Se ainda não houver arestas, verifica se há vértices selecionados
        if self.selected_edges.is_empty() {
            let sel_verts: Vec<u32> = self
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .map(|(i, _)| i as u32)
                .collect();
            if !sel_verts.is_empty() {
                self.dissolve_vertices(&sel_verts);
                return;
            }
        }

        // 3. Dissolve iterativo de arestas selecionadas compartilhadas por exatamente 2 faces
        loop {
            let mut dissolved_any = false;
            let edges_to_dissolve: Vec<(u32, u32)> = self.selected_edges.iter().copied().collect();
            for (a, b) in edges_to_dissolve {
                let adj = self.edge_faces(a, b);
                if adj.len() == 2 {
                    let (f1_idx, f2_idx) = (adj[0], adj[1]);
                    let f1 = &self.faces[f1_idx];
                    let f2 = &self.faces[f2_idx];

                    // Encontra posições dos vértices nas duas faces
                    let p1_a = f1.verts.iter().position(|&x| x == a);
                    let p1_b = f1.verts.iter().position(|&x| x == b);
                    let p2_a = f2.verts.iter().position(|&x| x == a);
                    let p2_b = f2.verts.iter().position(|&x| x == b);

                    if let (Some(i1_a), Some(i1_b), Some(i2_a), Some(i2_b)) =
                        (p1_a, p1_b, p2_a, p2_b)
                    {
                        // Constrói polígono fundido
                        let mut merged_verts = Vec::new();
                        let mut merged_uvs = Vec::new();

                        let m1 = f1.verts.len();
                        let m2 = f2.verts.len();

                        // Se f1 vai de a -> b e f2 vai de b -> a
                        let f1_forward = (i1_a + 1) % m1 == i1_b;
                        let start1 = if f1_forward { i1_b } else { i1_a };

                        let mut k = start1;
                        loop {
                            merged_verts.push(f1.verts[k]);
                            merged_uvs.push(f1.uv[k]);
                            k = (k + 1) % m1;
                            if (f1_forward && k == i1_a) || (!f1_forward && k == i1_b) {
                                merged_verts.push(f1.verts[k]);
                                merged_uvs.push(f1.uv[k]);
                                break;
                            }
                        }

                        let start2 = if f1_forward {
                            (i2_a + 1) % m2
                        } else {
                            (i2_b + 1) % m2
                        };
                        let end2 = if f1_forward { i2_b } else { i2_a };
                        let mut k2 = start2;
                        while k2 != end2 {
                            merged_verts.push(f2.verts[k2]);
                            merged_uvs.push(f2.uv[k2]);
                            k2 = (k2 + 1) % m2;
                        }

                        // Remove vértices consecutivos duplicados
                        let mut clean_verts = Vec::new();
                        let mut clean_uvs = Vec::new();
                        for (idx, &v) in merged_verts.iter().enumerate() {
                            if clean_verts.last() != Some(&v) {
                                clean_verts.push(v);
                                clean_uvs.push(merged_uvs[idx]);
                            }
                        }
                        if clean_verts.len() > 1 && clean_verts.first() == clean_verts.last() {
                            clean_verts.pop();
                            clean_uvs.pop();
                        }

                        if clean_verts.len() >= 3 {
                            self.faces[f1_idx].verts = clean_verts;
                            self.faces[f1_idx].uv = clean_uvs;
                            self.faces[f1_idx].selected = true;
                            self.faces.remove(f2_idx);
                            self.selected_edges.remove(&(a.min(b), a.max(b)));
                            dissolved_any = true;
                            break;
                        }
                    }
                }
            }
            if !dissolved_any {
                break;
            }
        }
        self.remove_isolated_vertices();
        self.selected_edges.clear();
    }

    /// Dissolve um conjunto de vértices:
    /// - Vértices com valência 2 em contorno de polígonos têm as duas arestas unificadas.
    /// - Vértices internos cercados por um leque fechado de faces têm o leque fundido em um polígono.
    pub fn dissolve_vertices(&mut self, verts_to_dissolve: &[u32]) {
        for &v in verts_to_dissolve {
            if v as usize >= self.verts.len() {
                continue;
            }
            let incident_face_indices: Vec<usize> = self
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.verts.contains(&v))
                .map(|(i, _)| i)
                .collect();

            if incident_face_indices.is_empty() {
                continue;
            }

            let mut neighbors = std::collections::HashSet::new();
            for &fi in &incident_face_indices {
                let f = &self.faces[fi];
                if let Some(pos) = f.verts.iter().position(|&x| x == v) {
                    let prev = f.verts[(pos + f.verts.len() - 1) % f.verts.len()];
                    let next = f.verts[(pos + 1) % f.verts.len()];
                    neighbors.insert(prev);
                    neighbors.insert(next);
                }
            }

            if neighbors.len() == 2 {
                for &fi in &incident_face_indices {
                    let f = &mut self.faces[fi];
                    if let Some(pos) = f.verts.iter().position(|&x| x == v) {
                        f.verts.remove(pos);
                        if pos < f.uv.len() {
                            f.uv.remove(pos);
                        }
                    }
                }
                self.faces.retain(|f| f.verts.len() >= 3);
            } else if incident_face_indices.len() >= 3 {
                let mut outer_edges = Vec::new();
                for &fi in &incident_face_indices {
                    let f = &self.faces[fi];
                    let n = f.verts.len();
                    for i in 0..n {
                        let e_a = f.verts[i];
                        let e_b = f.verts[(i + 1) % n];
                        if e_a != v && e_b != v {
                            outer_edges.push((e_a, e_b, f.uv[i], f.uv[(i + 1) % n]));
                        }
                    }
                }

                if outer_edges.len() >= 3 {
                    let mut cycle_verts = Vec::new();
                    let mut cycle_uvs = Vec::new();
                    let mut current = outer_edges[0].0;
                    let mut visited_edges = std::collections::HashSet::new();

                    let mut valid_cycle = true;
                    for _ in 0..outer_edges.len() {
                        let mut found = false;
                        for (idx, &(e_a, e_b, uv_a, _)) in outer_edges.iter().enumerate() {
                            if !visited_edges.contains(&idx) && e_a == current {
                                cycle_verts.push(e_a);
                                cycle_uvs.push(uv_a);
                                visited_edges.insert(idx);
                                current = e_b;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            valid_cycle = false;
                            break;
                        }
                    }

                    if valid_cycle && current == cycle_verts[0] && cycle_verts.len() >= 3 {
                        let first_fi = incident_face_indices[0];
                        let mat = self.faces[first_fi].material_slot;
                        let mut new_face = Face::with_uv(cycle_verts, cycle_uvs);
                        new_face.material_slot = mat;
                        new_face.selected = true;

                        let mut sorted_indices = incident_face_indices.clone();
                        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
                        for fi in sorted_indices {
                            self.faces.remove(fi);
                        }
                        self.push_face(new_face);
                    }
                }
            }
        }
        self.remove_isolated_vertices();
    }

    /// Fatiamento planar (Slice Plane): bissecciona a geometria ao longo de um plano definido por
    /// ponto e normal, inserindo vértices de corte e dividindo faces interpolando posições e UVs.
    /// Com `fill_cap`, mantém apenas o lado positivo da normal e fecha o corte.
    /// Sem tampa, preserva ambos os lados e apenas subdivide a superfície.
    pub fn slice_plane(&mut self, plane_point: Vec3, plane_normal: Vec3, fill_cap: bool) {
        if !plane_point.is_finite() || !plane_normal.is_finite() {
            return;
        }
        let n = plane_normal.normalize_or_zero();
        if n.length_squared() < 1e-4 {
            return;
        }
        if fill_cap {
            let distances = || self.verts.iter().map(|v| (v.vec() - plane_point).dot(n));
            if distances().any(|d| d < -1e-5) && !distances().any(|d| d > 1e-5) {
                *self = Self::default();
                return;
            }
        }

        let mut new_faces: Vec<Face> = Vec::new();
        let mut original_face_indices_to_remove: Vec<usize> = Vec::new();
        let mut cut_edge_cache: std::collections::HashMap<(u32, u32), u32> =
            std::collections::HashMap::new();

        for (fi, face) in self.faces.iter().enumerate() {
            let m = face.verts.len();
            if m < 3 {
                continue;
            }

            // Distância assinada de cada vértice ao plano
            let mut dists = Vec::with_capacity(m);
            for &vi in &face.verts {
                let p = self.verts[vi as usize].vec();
                let d = (p - plane_point).dot(n);
                dists.push(d);
            }

            let eps = 1e-5;
            let all_pos = dists.iter().all(|&d| d >= -eps);
            let all_neg = dists.iter().all(|&d| d <= eps);

            // Se todos os vértices estão no mesmo lado do plano, mantém a face
            if all_pos {
                continue;
            }
            if all_neg {
                if fill_cap {
                    original_face_indices_to_remove.push(fi);
                }
                continue;
            }

            // A face cruza o plano: divide o polígono
            original_face_indices_to_remove.push(fi);

            let mut pos_poly: Vec<(u32, [f32; 2])> = Vec::new();
            let mut neg_poly: Vec<(u32, [f32; 2])> = Vec::new();

            for k in 0..m {
                let k2 = (k + 1) % m;
                let v1 = face.verts[k];
                let v2 = face.verts[k2];
                let uv1 = face.uv[k];
                let uv2 = face.uv[k2];
                let d1 = dists[k];
                let d2 = dists[k2];

                if d1 >= -eps {
                    pos_poly.push((v1, uv1));
                }
                if d1 <= eps {
                    neg_poly.push((v1, uv1));
                }

                // Aresta cruza o plano: interpola ou reutiliza ponto de corte compartilhado
                if (d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps) {
                    let e_key = edge_key(v1, v2);
                    let t = (d1 / (d1 - d2)).clamp(0.0, 1.0);
                    let uv_cut = [
                        uv1[0] + (uv2[0] - uv1[0]) * t,
                        uv1[1] + (uv2[1] - uv1[1]) * t,
                    ];

                    let cut_idx = if let Some(&existing) = cut_edge_cache.get(&e_key) {
                        existing
                    } else {
                        let p1 = self.verts[v1 as usize].vec();
                        let p2 = self.verts[v2 as usize].vec();
                        let p_cut = p1.lerp(p2, t);
                        let col1 = self.verts[v1 as usize].color;
                        let col2 = self.verts[v2 as usize].color;
                        let col_cut = [
                            col1[0] + (col2[0] - col1[0]) * t,
                            col1[1] + (col2[1] - col1[1]) * t,
                            col1[2] + (col2[2] - col1[2]) * t,
                        ];

                        self.verts.push(Vertex {
                            pos: p_cut.to_array(),
                            color: col_cut,
                            selected: false,
                        });
                        let idx = (self.verts.len() - 1) as u32;
                        cut_edge_cache.insert(e_key, idx);
                        idx
                    };

                    pos_poly.push((cut_idx, uv_cut));
                    neg_poly.push((cut_idx, uv_cut));
                }
            }

            if pos_poly.len() >= 3 {
                let verts = pos_poly.iter().map(|(v, _)| *v).collect();
                let uvs = pos_poly.iter().map(|(_, uv)| *uv).collect();
                new_faces.push(Face::with_uv(verts, uvs));
            }
            if !fill_cap && neg_poly.len() >= 3 {
                let verts = neg_poly.iter().map(|(v, _)| *v).collect();
                let uvs = neg_poly.iter().map(|(_, uv)| *uv).collect();
                new_faces.push(Face::with_uv(verts, uvs));
            }
        }

        // Remove faces originais que foram fatiadas
        original_face_indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for fi in original_face_indices_to_remove {
            self.faces.remove(fi);
        }

        // Adiciona sub-faces fatiadas
        for f in new_faces {
            self.push_face(f);
        }

        // Deriva o contorno do resultado, incluindo vértices que já estavam no plano.
        // Apenas arestas incidentes a uma face precisam de tampa; arestas internas
        // ou faces coplanares preservadas não recebem geometria sobreposta.
        if fill_cap {
            let mut edge_uses: HashMap<(u32, u32), (usize, (u32, u32))> = HashMap::new();
            for face in &self.faces {
                for k in 0..face.verts.len() {
                    let a = face.verts[k];
                    let b = face.verts[(k + 1) % face.verts.len()];
                    let entry = edge_uses.entry(edge_key(a, b)).or_insert((0, (a, b)));
                    entry.0 += 1;
                }
            }
            let mut remaining_edges: Vec<(u32, u32)> = edge_uses
                .values()
                .filter(|(count, (a, b))| {
                    *count == 1
                        && [a, b].iter().all(|&&vi| {
                            (self.verts[vi as usize].vec() - plane_point).dot(n).abs() <= 1e-5
                        })
                })
                .map(|(_, edge)| *edge)
                .collect();
            remaining_edges.sort_unstable();
            while !remaining_edges.is_empty() {
                let first = remaining_edges.remove(0);
                let mut loop_verts = vec![first.0, first.1];
                let mut current = first.1;
                let mut closed = false;

                loop {
                    let mut found = None;
                    for (i, &(u, v)) in remaining_edges.iter().enumerate() {
                        if u == current {
                            found = Some((i, v));
                            break;
                        } else if v == current {
                            found = Some((i, u));
                            break;
                        }
                    }
                    if let Some((i, next_v)) = found {
                        remaining_edges.remove(i);
                        if next_v == first.0 {
                            closed = true;
                            break;
                        }
                        loop_verts.push(next_v);
                        current = next_v;
                    } else {
                        break;
                    }
                }

                if closed && loop_verts.len() >= 3 {
                    let norm = plane_normal.normalize_or_zero();
                    let tangent = if norm.x.abs() < 0.9 {
                        Vec3::X.cross(norm).normalize_or_zero()
                    } else {
                        Vec3::Y.cross(norm).normalize_or_zero()
                    };
                    let bitangent = norm.cross(tangent).normalize_or_zero();

                    let cap_uvs: Vec<[f32; 2]> = loop_verts
                        .iter()
                        .map(|&vi| {
                            let p = self.verts[vi as usize].vec() - plane_point;
                            [p.dot(tangent) * 0.5 + 0.5, p.dot(bitangent) * 0.5 + 0.5]
                        })
                        .collect();

                    let mut cap = Face::with_uv(loop_verts, cap_uvs);
                    let c_norm = triangulate::face_normal_of(&self.verts, &cap.verts);
                    if c_norm.dot(norm) > 0.0 {
                        cap.verts.reverse();
                        cap.uv.reverse();
                    }
                    cap.selected = true;
                    self.push_face(cap);
                }
            }
            self.selected_edges.clear();
            self.remove_isolated_vertices();
            self.sync_vert_selection_from_faces();
        }
    }

    /// Extrusão por varredura (Sweep): estruza um perfil 2D ao longo de um caminho 3D (path),
    /// calculando referenciais locais (Rotation-Minimizing Frames) para evitar torções inesperadas.
    pub fn sweep(
        &mut self,
        profile: &[[f32; 2]],
        path: &[Vec3],
        cap_ends: bool,
    ) -> Result<(), String> {
        let swept = Self::from_sweep(
            profile,
            path,
            crate::sweep::SweepOptions {
                closed_path: false,
                closed_profile: true,
                cap_start: cap_ends,
                cap_end: cap_ends,
                miter: true,
                miter_limit: 3.0,
            },
        )?;
        self.join(&swept);
        Ok(())
    }

    // ---------------- operações de combinação ----------------

    /// Combina outra malha nesta, mantendo ilhas disjuntas em um único objeto.
    pub fn join(&mut self, other: &Mesh) {
        let base_idx = self.verts.len() as u32;
        self.verts.extend(other.verts.iter().cloned());

        for f in &other.faces {
            let remapped_verts: Vec<u32> = f.verts.iter().map(|&i| i + base_idx).collect();
            let mut nf = Face::with_uv(remapped_verts, f.uv.clone());
            nf.selected = f.selected;
            self.push_face(nf);
        }

        for &(a, b) in &other.selected_edges {
            self.selected_edges.insert((a + base_idx, b + base_idx));
        }
    }

    /// Funde outra malha com esta, soldando vértices coincidentes dentro do raio `eps`.
    pub fn fuse_approximate(&mut self, other: &Mesh, eps: f32) {
        self.join(other);
        self.weld(eps);
    }

    /// Cria uma nova face conectando elementos selecionados (3+ arestas, 3+ vértices ou fecha triângulo com 2 arestas).
    /// Se apenas 2 vértices estiverem selecionados, cria uma aresta conectando-os.
    pub fn make_face_from_selection(&mut self) -> Result<(), &'static str> {
        let sel_verts: Vec<u32> = self
            .verts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.selected)
            .map(|(i, _)| i as u32)
            .collect();

        let sel_edges: Vec<(u32, u32)> = self.selected_edges.iter().copied().collect();

        // Caso 1: Arestas selecionadas
        if sel_edges.len() >= 2 {
            if sel_edges.len() == 2 {
                let e0 = sel_edges[0];
                let e1 = sel_edges[1];
                let mut vset = HashSet::new();
                vset.insert(e0.0);
                vset.insert(e0.1);
                vset.insert(e1.0);
                vset.insert(e1.1);

                if vset.len() == 3 {
                    let shared = if e0.0 == e1.0 || e0.0 == e1.1 {
                        e0.0
                    } else {
                        e0.1
                    };
                    let other0 = if e0.0 == shared { e0.1 } else { e0.0 };
                    let other1 = if e1.0 == shared { e1.1 } else { e1.0 };
                    let tri = vec![other0, shared, other1];

                    let mut sorted = tri.clone();
                    sorted.sort_unstable();
                    if self.faces.iter().any(|f| {
                        let mut fs = f.verts.clone();
                        fs.sort_unstable();
                        fs == sorted
                    }) {
                        return Err("Face already exists");
                    }

                    let mut face = Face::new(tri);
                    face.selected = true;
                    self.push_face(face);
                    self.sync_vert_selection_from_faces();
                    self.sync_edge_selection_from_verts();
                    self.recalculate_normals();
                    return Ok(());
                } else if vset.len() == 4 {
                    let (a, b) = e0;
                    let (c, d) = e1;
                    let pa = self.verts[a as usize].vec();
                    let pb = self.verts[b as usize].vec();
                    let pc = self.verts[c as usize].vec();
                    let pd = self.verts[d as usize].vec();
                    let d1 = pa.distance(pc) + pb.distance(pd);
                    let d2 = pa.distance(pd) + pb.distance(pc);
                    let quad = if d1 < d2 {
                        vec![a, b, d, c]
                    } else {
                        vec![a, b, c, d]
                    };

                    let mut sorted = quad.clone();
                    sorted.sort_unstable();
                    if self.faces.iter().any(|f| {
                        let mut fs = f.verts.clone();
                        fs.sort_unstable();
                        fs == sorted
                    }) {
                        return Err("Face already exists");
                    }

                    let mut face = Face::new(quad);
                    face.selected = true;
                    self.push_face(face);
                    self.sync_vert_selection_from_faces();
                    self.sync_edge_selection_from_verts();
                    self.recalculate_normals();
                    return Ok(());
                }
            }

            // 3 ou mais arestas selecionadas: constrói um caminho/ciclo encadeado
            let mut adj: std::collections::HashMap<u32, Vec<u32>> =
                std::collections::HashMap::new();
            for &(u, v) in &sel_edges {
                adj.entry(u).or_default().push(v);
                adj.entry(v).or_default().push(u);
            }

            let start = adj
                .iter()
                .find(|(_, nbrs)| nbrs.len() == 1)
                .map(|(&v, _)| v)
                .or_else(|| adj.keys().copied().next())
                .ok_or("No vertices in selected edges")?;

            let mut path = Vec::new();
            let mut visited_edges: HashSet<(u32, u32)> = HashSet::new();
            let mut curr = start;
            path.push(curr);

            loop {
                let next_opt = adj.get(&curr).and_then(|nbrs| {
                    nbrs.iter()
                        .copied()
                        .find(|&nbr| !visited_edges.contains(&edge_key(curr, nbr)))
                });
                if let Some(next) = next_opt {
                    visited_edges.insert(edge_key(curr, next));
                    if next == start {
                        break;
                    }
                    path.push(next);
                    curr = next;
                } else {
                    break;
                }
            }

            if path.len() >= 3 {
                let mut sorted = path.clone();
                sorted.sort_unstable();
                if self.faces.iter().any(|f| {
                    let mut fs = f.verts.clone();
                    fs.sort_unstable();
                    fs == sorted
                }) {
                    return Err("Face already exists");
                }

                let mut face = Face::new(path);
                face.selected = true;
                self.push_face(face);
                self.sync_vert_selection_from_faces();
                self.sync_edge_selection_from_verts();
                self.recalculate_normals();
                return Ok(());
            }
        }

        // Caso 2: Vértices selecionados diretamente
        if sel_verts.len() == 2 {
            self.selected_edges
                .insert(edge_key(sel_verts[0], sel_verts[1]));
            return Ok(());
        }

        if sel_verts.len() == 3 {
            let mut sorted = sel_verts.clone();
            sorted.sort_unstable();
            if self.faces.iter().any(|f| {
                let mut fs = f.verts.clone();
                fs.sort_unstable();
                fs == sorted
            }) {
                return Err("Face already exists");
            }

            let mut face = Face::new(sel_verts);
            face.selected = true;
            self.push_face(face);
            self.sync_vert_selection_from_faces();
            self.sync_edge_selection_from_verts();
            self.recalculate_normals();
            return Ok(());
        }

        if sel_verts.len() >= 4 {
            let mut centroid = Vec3::ZERO;
            for &vi in &sel_verts {
                centroid += self.verts[vi as usize].vec();
            }
            centroid /= sel_verts.len() as f32;

            let mut normal = Vec3::ZERO;
            for i in 0..sel_verts.len() {
                let p0 = self.verts[sel_verts[i] as usize].vec() - centroid;
                let p1 = self.verts[sel_verts[(i + 1) % sel_verts.len()] as usize].vec() - centroid;
                normal += p0.cross(p1);
            }
            let normal = normal.normalize_or_zero();
            let normal = if normal.length_squared() > 0.1 {
                normal
            } else {
                Vec3::Y
            };

            let u = (self.verts[sel_verts[0] as usize].vec() - centroid).normalize_or_zero();
            let u = if u.length_squared() > 0.1 {
                u
            } else {
                let fallback = if normal.x.abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                normal.cross(fallback).normalize_or_zero()
            };
            let v = normal.cross(u).normalize_or_zero();

            let mut ordered = sel_verts.clone();
            ordered.sort_by(|&a, &b| {
                let pa = self.verts[a as usize].vec() - centroid;
                let pb = self.verts[b as usize].vec() - centroid;
                let angle_a = pa.dot(v).atan2(pa.dot(u));
                let angle_b = pb.dot(v).atan2(pb.dot(u));
                angle_a
                    .partial_cmp(&angle_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut sorted = ordered.clone();
            sorted.sort_unstable();
            if self.faces.iter().any(|f| {
                let mut fs = f.verts.clone();
                fs.sort_unstable();
                fs == sorted
            }) {
                return Err("Face already exists");
            }

            let mut face = Face::new(ordered);
            face.selected = true;
            self.push_face(face);
            self.sync_vert_selection_from_faces();
            self.sync_edge_selection_from_verts();
            self.recalculate_normals();
            return Ok(());
        }

        Err(
            "Select at least 3 edges or vertices to create a face (or 2 vertices to connect an edge)",
        )
    }
}

#[cfg(test)]
mod region_tests {
    use super::*;

    fn assert_closed(mesh: &Mesh) {
        let report = mesh.validate_topology();
        assert!(report.is_manifold, "{report:?}");
        assert!(report.is_closed, "{report:?}");
        assert_eq!(report.isolated_vertex_count, 0);
        assert!(mesh.faces.iter().all(|f| f.uv.len() == f.verts.len()));
        assert!(mesh.verts.iter().all(|v| v.vec().is_finite()));
    }

    #[test]
    fn extrude_cube_face_replaces_base_and_preserves_closed_surface() {
        let mut mesh = Mesh::cube(2.0);
        mesh.faces[0].selected = true;
        mesh.extrude_selected(0.5);
        assert_closed(&mesh);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (12, 10));
        assert_eq!(mesh.faces.iter().filter(|f| f.selected).count(), 1);
        assert!(
            mesh.faces[0]
                .verts
                .iter()
                .all(|&i| mesh.verts[i as usize].pos[2] == -1.5)
        );
    }

    #[test]
    fn extrude_adjacent_faces_has_shared_cap_and_only_boundary_walls() {
        let mut mesh = Mesh::cube(2.0);
        mesh.faces[0].selected = true;
        mesh.faces[2].selected = true;
        mesh.extrude_selected(0.5);
        assert_closed(&mesh);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (14, 12));
        assert_eq!(
            mesh.faces[0]
                .verts
                .iter()
                .filter(|vi| mesh.faces[2].verts.contains(vi))
                .count(),
            2
        );
    }

    #[test]
    fn extrude_large_region_removes_old_interior_vertices() {
        let mut mesh = Mesh::cube(2.0);
        mesh.select_all();
        mesh.faces[1].selected = false;
        mesh.extrude_selected(0.2);
        assert_closed(&mesh);
        assert_eq!(mesh.verts.len(), 12);
    }

    #[test]
    fn extrude_negative_distance_remains_closed() {
        let mut mesh = Mesh::cube(2.0);
        mesh.faces[0].selected = true;
        mesh.extrude_selected(-0.25);
        assert_closed(&mesh);
        assert!(
            mesh.faces[0]
                .verts
                .iter()
                .all(|&i| mesh.verts[i as usize].pos[2] == -0.75)
        );
    }

    #[test]
    fn extrude_zero_prepares_selected_cap_for_modal_displacement() {
        let mut mesh = Mesh::cube(2.0);
        mesh.faces[0].selected = true;
        mesh.extrude_selected(0.0);
        for v in mesh.verts.iter_mut().filter(|v| v.selected) {
            v.pos[2] -= 0.5;
        }
        assert_closed(&mesh);
        assert_eq!(mesh.verts.iter().filter(|v| v.selected).count(), 4);
    }

    #[test]
    fn nonfinite_extrusion_and_invalid_inset_do_not_mutate() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut mesh = Mesh::cube(2.0);
            mesh.faces[0].selected = true;
            let before = format!("{mesh:?}");
            mesh.extrude_selected(value);
            assert_eq!(format!("{mesh:?}"), before);
            mesh.inset_selected(value);
            assert_eq!(format!("{mesh:?}"), before);
        }
        let mut mesh = Mesh::cube(2.0);
        mesh.faces[0].selected = true;
        let before = format!("{mesh:?}");
        mesh.inset_selected(0.0);
        mesh.inset_selected(-1.0);
        assert_eq!(format!("{mesh:?}"), before);
    }

    #[test]
    fn inset_cube_face_is_closed_even_at_clamped_maximum() {
        let mut mesh = Mesh::cube(2.0);
        mesh.faces[0].selected = true;
        mesh.inset_selected(1e20);
        assert_closed(&mesh);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (12, 10));
    }

    #[test]
    fn capped_slice_retains_positive_side_and_outward_cap() {
        let mut mesh = Mesh::cube(2.0);
        mesh.slice_plane(Vec3::ZERO, Vec3::Y, true);
        assert_closed(&mesh);
        assert!(mesh.verts.iter().all(|v| v.pos[1] >= 0.0));
        let cap = mesh.faces.iter().position(|f| f.selected).expect("cut cap");
        assert!(mesh.face_normal(cap).dot(-Vec3::Y) > 0.99);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (8, 6));
    }

    #[test]
    fn capped_slice_through_existing_vertices_is_welded() {
        for normal in [Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 1.0)] {
            let mut mesh = Mesh::cube(2.0);
            let point = if normal.z == 0.0 {
                Vec3::ZERO
            } else {
                Vec3::new(1.0, 0.0, 0.0)
            };
            mesh.slice_plane(point, normal, true);
            assert_closed(&mesh);
            assert!(
                mesh.verts
                    .iter()
                    .all(|v| (v.vec() - point).dot(normal) >= -1e-5)
            );
        }
    }

    #[test]
    fn tangent_slice_does_not_duplicate_existing_face() {
        let mut mesh = Mesh::cube(2.0);
        mesh.slice_plane(Vec3::new(0.0, -1.0, 0.0), Vec3::Y, true);
        assert_closed(&mesh);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (8, 6));

        mesh.slice_plane(Vec3::new(0.0, 1.0, 0.0), Vec3::Y, true);
        assert!(mesh.faces.is_empty());
        assert!(mesh.verts.is_empty());
    }

    #[test]
    fn uncapped_slice_subdivides_both_sides_without_internal_face() {
        let mut mesh = Mesh::cube(2.0);
        mesh.slice_plane(Vec3::ZERO, Vec3::Y, false);
        assert_closed(&mesh);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (12, 10));
        assert!(mesh.verts.iter().any(|v| v.pos[1] < 0.0));
        assert!(mesh.verts.iter().any(|v| v.pos[1] > 0.0));
    }

    #[test]
    fn weld_merges_duplicates_within_eps() {
        let mut mesh = Mesh::cube(2.0);
        let before = mesh.verts.len();
        // Duplica um vértice exatamente.
        let dup = mesh.verts[0].clone();
        mesh.verts.push(dup);
        assert_eq!(mesh.verts.len(), before + 1);
        let removed = mesh.weld_merged_count(0.0001);
        assert_eq!(removed, 1);
        assert_eq!(mesh.verts.len(), before);
    }

    #[test]
    fn symmetrize_copies_positive_to_negative_and_snaps_center() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
        mesh.verts.push(Vertex::new(-5.0, 0.0, 0.0));
        let mirrored = mesh.symmetrize(0, true, 0.001);
        assert_eq!(mirrored, 1);
        // Fonte + centro + espelho; lado negativo original descartado.
        assert_eq!(mesh.verts.len(), 3);
        assert!(mesh.verts.iter().any(|v| (v.pos[0] + 1.0).abs() < 1e-5));
        assert!(mesh.verts.iter().any(|v| v.pos[0].abs() < 1e-6));
        assert!(!mesh.verts.iter().any(|v| (v.pos[0] + 5.0).abs() < 1e-5));
    }

    #[test]
    fn symmetrize_without_source_is_noop() {
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(-1.0, 0.0, 0.0));
        let before = format!("{mesh:?}");
        assert_eq!(mesh.symmetrize(0, true, 0.001), 0);
        assert_eq!(format!("{mesh:?}"), before);
    }

    #[test]
    fn rotate_selected_quarter_turn_about_z() {
        let mut mesh = Mesh::cube(2.0);
        let c = mesh.selection_center();
        mesh.rotate_selected([0.0, 0.0, std::f32::consts::FRAC_PI_2], c);
        // (1,1,1) gira para (-1,1,1) em torno do centro.
        assert!(
            mesh.verts.iter().any(|v| (v.pos[0] + 1.0).abs() < 1e-4
                && (v.pos[1] - 1.0).abs() < 1e-4
                && (v.pos[2] - 1.0).abs() < 1e-4),
            "rotação Z de 90° esperada"
        );
        let report = mesh.validate_topology();
        assert!(report.is_manifold, "{report:?}");
    }

    #[test]
    fn scale_selected_factors_is_per_axis() {
        let mut mesh = Mesh::cube(2.0);
        let c = mesh.selection_center();
        mesh.scale_selected_factors([2.0, 1.0, 0.5], c);
        let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
        let (mut min_z, mut max_z) = (f32::INFINITY, f32::NEG_INFINITY);
        for v in &mesh.verts {
            min_x = min_x.min(v.pos[0]);
            max_x = max_x.max(v.pos[0]);
            min_z = min_z.min(v.pos[2]);
            max_z = max_z.max(v.pos[2]);
        }
        assert!((max_x - min_x - 4.0).abs() < 1e-4);
        assert!((max_z - min_z - 1.0).abs() < 1e-4);
    }

    #[test]
    fn edge_loop_selects_complete_boundary_on_plane() {
        let mut mesh = Mesh::plane(2.0);
        // Plane has 1 quad face, 4 boundary edges.
        let edges = mesh.edges_unique();
        let loop_edges = mesh.edge_loop(edges[0]);
        assert_eq!(loop_edges.len(), 4, "boundary loop of plane has 4 edges");
        mesh.select_edge_loop(edges[0], false);
        assert_eq!(mesh.selected_edges.len(), 4);
        assert_eq!(mesh.selected_vert_count(), 4);
    }

    #[test]
    fn face_loop_selects_quad_ring_on_cube() {
        let mut mesh = Mesh::cube(2.0);
        // Cube has 6 quad faces. A face loop around 4 side faces forms a closed quad ring.
        let loop_faces = mesh.face_loop(0, None);
        assert_eq!(
            loop_faces.len(),
            4,
            "face loop around cube ring contains 4 faces"
        );
        let count = mesh.select_face_loop(0, None, false);
        assert_eq!(count, 4);
        assert_eq!(mesh.faces.iter().filter(|f| f.selected).count(), 4);
    }

    #[test]
    fn vertex_loop_selects_ring_on_plane() {
        let mut mesh = Mesh::plane(2.0);
        // Plane has 1 quad face, 4 boundary vertices in the loop
        let count = mesh.select_vertex_loop(0, false);
        assert_eq!(
            count, 4,
            "vertex loop around plane boundary contains 4 vertices"
        );
        assert_eq!(mesh.selected_vert_count(), 4);
    }

    #[test]
    fn make_face_from_selected_edges_and_vertices() {
        // 1. A partir de 4 arestas (cubo sem a face superior)
        let mut mesh = Mesh::cube(2.0);
        assert_eq!(mesh.faces.len(), 6);
        let removed_face = mesh.faces.remove(0); // Remove uma face
        assert_eq!(mesh.faces.len(), 5);

        // Seleciona as 4 arestas da face removida
        let n = removed_face.verts.len();
        for i in 0..n {
            let u = removed_face.verts[i];
            let v = removed_face.verts[(i + 1) % n];
            mesh.selected_edges.insert(edge_key(u, v));
        }

        assert!(mesh.make_face_from_selection().is_ok());
        assert_eq!(mesh.faces.len(), 6);
        assert!(mesh.faces.last().unwrap().selected);

        // 2. A partir de 3 vértices soltos (triângulo)
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(0.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
        for v in &mut mesh.verts {
            v.selected = true;
        }
        assert!(mesh.make_face_from_selection().is_ok());
        assert_eq!(mesh.faces.len(), 1);
        assert_eq!(mesh.faces[0].verts.len(), 3);

        // 3. A partir de 2 vértices (conecta com aresta)
        let mut mesh = Mesh::default();
        mesh.verts.push(Vertex::new(0.0, 0.0, 0.0));
        mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
        mesh.verts[0].selected = true;
        mesh.verts[1].selected = true;
        assert!(mesh.make_face_from_selection().is_ok());
        assert_eq!(mesh.selected_edges.len(), 1);
        assert!(mesh.selected_edges.contains(&edge_key(0, 1)));
    }
}
