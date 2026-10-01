//! Seleção por loops: Edge Loop, Edge Ring e expansão da seleção atual.
//!
//! Regras do Edge Loop (as mesmas que o usuário espera de um modelador de
//! polígonos), aplicadas em cada vértice `curr` ao chegar por uma aresta
//! `prev → curr`:
//!
//! 1. **Contorno**: aresta com uma só face segue a outra aresta de contorno.
//! 2. **Valência 4 interior**: segue a aresta *oposta* (a que não divide face com
//!    a de chegada), qualquer que seja o tamanho das faces.
//! 3. **Valência 3 numa face grande (n-gon > 4 lados)**: a aresta de chegada é
//!    borda do n-gon, então o loop contorna o n-gon. É o anel das tampas de
//!    extrusões, cilindros e resultados de booleanos.
//! 4. Qualquer outro caso (polos, valência ≥ 5, vértices não manifold) encerra.
//!
//! Todas as consultas usam mapas de adjacência montados uma vez (O(F)); o
//! caminho antigo varria todas as faces a cada passo.

use std::collections::{HashMap, HashSet};

use crate::{Mesh, edge_key};

type EdgeKey = (u32, u32);

/// Adjacência aresta→faces e vértice→faces, montada uma vez por consulta.
struct Adjacency {
    edge_faces: HashMap<EdgeKey, Vec<usize>>,
    vert_faces: HashMap<u32, Vec<usize>>,
}

impl Adjacency {
    fn build(mesh: &Mesh) -> Self {
        let mut edge_faces: HashMap<EdgeKey, Vec<usize>> = HashMap::new();
        let mut vert_faces: HashMap<u32, Vec<usize>> = HashMap::new();
        for (fi, face) in mesh.faces.iter().enumerate() {
            let m = face.verts.len();
            for k in 0..m {
                let (a, b) = (face.verts[k], face.verts[(k + 1) % m]);
                let faces = edge_faces.entry(edge_key(a, b)).or_default();
                if !faces.contains(&fi) {
                    faces.push(fi);
                }
                let around = vert_faces.entry(a).or_default();
                if !around.contains(&fi) {
                    around.push(fi);
                }
            }
        }
        Self {
            edge_faces,
            vert_faces,
        }
    }

    fn faces_of(&self, key: EdgeKey) -> &[usize] {
        self.edge_faces.get(&key).map_or(&[], Vec::as_slice)
    }
}

/// Vizinhos de `vertex` dentro de `face` (anterior e seguinte no contorno).
fn neighbors_in_face(mesh: &Mesh, face: usize, vertex: u32) -> Option<[u32; 2]> {
    let verts = &mesh.faces[face].verts;
    let m = verts.len();
    let pos = verts.iter().position(|&v| v == vertex)?;
    Some([verts[(pos + m - 1) % m], verts[(pos + 1) % m]])
}

impl Mesh {
    /// Próximo vértice do loop ao chegar em `curr` vindo de `prev`.
    fn loop_next(&self, adj: &Adjacency, prev: u32, curr: u32) -> Option<u32> {
        let arriving = adj.faces_of(edge_key(prev, curr));
        let around = adj.vert_faces.get(&curr)?;
        match arriving.len() {
            1 => {
                let mut candidates: Vec<u32> = Vec::new();
                for &fi in around {
                    for n in neighbors_in_face(self, fi, curr)? {
                        if n != prev
                            && adj.faces_of(edge_key(curr, n)).len() == 1
                            && !candidates.contains(&n)
                        {
                            candidates.push(n);
                        }
                    }
                }
                (candidates.len() == 1).then(|| candidates[0])
            }
            2 => {
                let mut neighbors: Vec<u32> = Vec::new();
                for &fi in around {
                    for n in neighbors_in_face(self, fi, curr)? {
                        if !neighbors.contains(&n) {
                            neighbors.push(n);
                        }
                    }
                }
                if around.len() == 4 && neighbors.len() == 4 {
                    let mut shared: HashSet<u32> = HashSet::from([prev]);
                    for &fi in arriving {
                        shared.extend(neighbors_in_face(self, fi, curr)?);
                    }
                    let mut opposite = neighbors.into_iter().filter(|n| !shared.contains(n));
                    let next = opposite.next()?;
                    opposite.next().is_none().then_some(next)
                } else if around.len() == 3 && neighbors.len() == 3 {
                    let hub = arriving
                        .iter()
                        .copied()
                        .find(|&fi| self.faces[fi].verts.len() > 4)?;
                    let [before, after] = neighbors_in_face(self, hub, curr)?;
                    if before == prev {
                        Some(after)
                    } else if after == prev {
                        Some(before)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Todas as arestas do Edge Loop que passa pela aresta semente.
    pub fn edge_loop(&self, seed: (u32, u32)) -> Vec<(u32, u32)> {
        let adj = Adjacency::build(self);
        self.edge_loop_with(&adj, seed)
    }

    fn edge_loop_with(&self, adj: &Adjacency, seed: (u32, u32)) -> Vec<EdgeKey> {
        let seed_key = edge_key(seed.0, seed.1);
        if adj.faces_of(seed_key).is_empty() {
            return Vec::new();
        }
        let mut loop_set: HashSet<EdgeKey> = HashSet::from([seed_key]);
        let mut order = vec![seed_key];
        for (start, first) in [(seed.0, seed.1), (seed.1, seed.0)] {
            let (mut prev, mut curr) = (start, first);
            while let Some(next) = self.loop_next(adj, prev, curr) {
                let key = edge_key(curr, next);
                if !loop_set.insert(key) {
                    break;
                }
                order.push(key);
                prev = curr;
                curr = next;
            }
        }
        order
    }

    /// Seleciona o Edge Loop completo a partir de uma aresta semente.
    /// Retorna a contagem de arestas do loop.
    pub fn select_edge_loop(&mut self, seed: (u32, u32), extend: bool) -> usize {
        let edges = self.edge_loop(seed);
        self.apply_edge_selection(edges, extend)
    }

    /// Todas as arestas do Edge Ring (arestas paralelas atravessando quads).
    pub fn edge_ring(&self, seed: (u32, u32)) -> Vec<(u32, u32)> {
        let adj = Adjacency::build(self);
        self.edge_ring_with(&adj, seed)
    }

    fn edge_ring_with(&self, adj: &Adjacency, seed: (u32, u32)) -> Vec<EdgeKey> {
        let seed_key = edge_key(seed.0, seed.1);
        let mut ring: HashSet<EdgeKey> = HashSet::from([seed_key]);
        let mut order = vec![seed_key];
        for &start_face in adj.faces_of(seed_key) {
            let (mut face, mut entry) = (start_face, seed_key);
            loop {
                let verts = &self.faces[face].verts;
                if verts.len() != 4 {
                    break;
                }
                let Some(k) = (0..4).find(|&k| edge_key(verts[k], verts[(k + 1) % 4]) == entry)
                else {
                    break;
                };
                let opposite = edge_key(verts[(k + 2) % 4], verts[(k + 3) % 4]);
                if !ring.insert(opposite) {
                    break;
                }
                order.push(opposite);
                let Some(next) = adj
                    .faces_of(opposite)
                    .iter()
                    .copied()
                    .find(|&other| other != face)
                else {
                    break;
                };
                face = next;
                entry = opposite;
            }
        }
        order
    }

    /// Seleciona o Edge Ring completo a partir de uma aresta semente.
    pub fn select_edge_ring(&mut self, seed: (u32, u32), extend: bool) -> usize {
        let edges = self.edge_ring(seed);
        self.apply_edge_selection(edges, extend)
    }

    /// Expande cada aresta selecionada para o seu Edge Loop (ou Ring).
    /// Retorna o total de arestas selecionadas depois da expansão.
    pub fn expand_selection_to_edge_loops(&mut self, ring: bool) -> usize {
        let adj = Adjacency::build(self);
        let seeds: Vec<EdgeKey> = self.selected_edges.iter().copied().collect();
        let mut all: Vec<EdgeKey> = Vec::new();
        for seed in seeds {
            if ring {
                all.extend(self.edge_ring_with(&adj, seed));
            } else {
                all.extend(self.edge_loop_with(&adj, seed));
            }
        }
        self.apply_edge_selection(all, true);
        self.selected_edges.len()
    }

    fn apply_edge_selection(&mut self, edges: Vec<EdgeKey>, extend: bool) -> usize {
        let count = edges.len();
        if !extend {
            self.deselect_all();
        }
        for edge in edges {
            self.selected_edges.insert(edge);
        }
        for vertex in &mut self.verts {
            vertex.selected = false;
        }
        let ends: Vec<EdgeKey> = self.selected_edges.iter().copied().collect();
        for (a, b) in ends {
            for index in [a, b] {
                if let Some(vertex) = self.verts.get_mut(index as usize) {
                    vertex.selected = true;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    /// Prisma de `n` lados com tampas n-gon (o que uma extrusão de perfil gera).
    fn prism(n: usize) -> Mesh {
        let mut mesh = Mesh::default();
        for ring in 0..2 {
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                mesh.verts
                    .push(crate::Vertex::new(a.cos(), ring as f32, a.sin()));
            }
        }
        let n32 = n as u32;
        for k in 0..n32 {
            let next = (k + 1) % n32;
            mesh.faces
                .push(Face::new(vec![k, next, next + n32, k + n32]));
        }
        mesh.faces.push(Face::new((0..n32).rev().collect()));
        mesh.faces.push(Face::new((n32..2 * n32).collect()));
        mesh
    }

    #[test]
    fn cap_ring_of_a_prism_is_one_loop() {
        let mesh = prism(12);
        let loop_edges = mesh.edge_loop((0, 1));
        assert_eq!(loop_edges.len(), 12, "o anel da tampa inteira");
        let unique: HashSet<_> = loop_edges.iter().collect();
        assert_eq!(unique.len(), 12);
        assert!(loop_edges.iter().all(|&(a, b)| a < 12 && b < 12));
    }

    #[test]
    fn vertical_edge_of_a_prism_does_not_leak_into_the_caps() {
        let mesh = prism(8);
        assert_eq!(mesh.edge_loop((0, 8)).len(), 1);
    }

    #[test]
    fn middle_ring_of_a_subdivided_cylinder_closes_through_valence_four() {
        // Cilindro com um anel intermediário: o loop do meio dá a volta.
        let mut mesh = Mesh::default();
        let n = 10usize;
        for ring in 0..3 {
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                mesh.verts
                    .push(crate::Vertex::new(a.cos(), ring as f32, a.sin()));
            }
        }
        let n32 = n as u32;
        for ring in 0..2u32 {
            for k in 0..n32 {
                let next = (k + 1) % n32;
                let (base, up) = (ring * n32, (ring + 1) * n32);
                mesh.faces
                    .push(Face::new(vec![base + k, base + next, up + next, up + k]));
            }
        }
        mesh.faces.push(Face::new((0..n32).rev().collect()));
        mesh.faces.push(Face::new((2 * n32..3 * n32).collect()));
        // aresta do anel do meio (vértices 10..20)
        let loop_edges = mesh.edge_loop((n32, n32 + 1));
        assert_eq!(loop_edges.len(), n, "dá a volta pelo meio");
        // aresta da tampa de cima
        assert_eq!(mesh.edge_loop((2 * n32, 2 * n32 + 1)).len(), n);
    }

    /// Grade `n × n` de quads no plano XZ.
    fn grid(n: u32) -> Mesh {
        let mut mesh = Mesh::default();
        for z in 0..=n {
            for x in 0..=n {
                mesh.verts
                    .push(crate::Vertex::new(x as f32, 0.0, z as f32));
            }
        }
        let stride = n + 1;
        for z in 0..n {
            for x in 0..n {
                let a = z * stride + x;
                mesh.faces
                    .push(Face::new(vec![a, a + 1, a + stride + 1, a + stride]));
            }
        }
        mesh
    }

    #[test]
    fn grid_loop_runs_straight_through_interior_vertices() {
        let mesh = grid(4);
        // aresta horizontal na linha z = 2, da coluna 1 para a 2
        let stride = 5u32;
        let seed = (2 * stride + 1, 2 * stride + 2);
        let loop_edges = mesh.edge_loop(seed);
        assert_eq!(loop_edges.len(), 4, "uma linha inteira da grade 4x4");
        assert!(
            loop_edges
                .iter()
                .all(|&(a, b)| a / stride == 2 && b / stride == 2),
            "todas na mesma linha"
        );
    }

    #[test]
    fn edge_ring_crosses_quads_in_a_strip() {
        let mesh = prism(8);
        let ring = mesh.edge_ring((0, 8));
        assert_eq!(ring.len(), 8, "arestas verticais paralelas em volta do prisma");
    }

    #[test]
    fn expand_selection_grows_each_selected_edge() {
        let mut mesh = prism(6);
        mesh.selected_edges.insert(edge_key(0, 1));
        let total = mesh.expand_selection_to_edge_loops(false);
        assert_eq!(total, 6);
        assert_eq!(mesh.selected_vert_count(), 6);
    }

    #[test]
    fn seed_not_in_the_mesh_returns_nothing() {
        let mesh = prism(6);
        assert!(mesh.edge_loop((0, 3)).is_empty());
    }
}
