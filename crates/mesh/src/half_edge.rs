//! Petunia3D — Estrutura Half-Edge e Invariantes Topológicos.
//!
//! Fornece representação de topologia orientada por arestas com suporte a:
//! - Identificação de arestas gêmeas (twins) e bordas abertas (boundaries).
//! - Queries de estrela de vértices (vertex star), vizinhos de faces e loops de contorno.
//! - Validação estrita de invariantes de variedade (manifold), detecção de leques não-variedade,
//!   e vértices/arestas órfãs.
//! - Conversão bidirecional entre `Mesh` indexada e `HalfEdgeMesh`.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::{Face, Mesh, Vertex, edge_key};

/// Identificadores fortemente tipados.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VertexId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FaceId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HalfEdgeId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub usize);

/// Erros de topologia detectados durante a construção ou validação.
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyDefect {
    #[error(
        "Aresta não-manifold compartilhada por mais de duas faces: ({u}, {v}) encontrada {count} vezes"
    )]
    NonManifoldEdge { u: usize, v: usize, count: usize },

    #[error("Vértice não-manifold com leques de faces desconectados no índice {vertex}: {reason}")]
    NonManifoldVertex { vertex: usize, reason: String },

    #[error(
        "Inconsistência de gêmeo (twin): half-edge {he} aponta para twin {twin}, mas twin aponta para {twin_of_twin:?}"
    )]
    TwinMismatch {
        he: usize,
        twin: usize,
        twin_of_twin: Option<usize>,
    },

    #[error("Inconsistência de loop next/prev no half-edge {he}")]
    NextPrevMismatch { he: usize },

    #[error("Face degenerada {face} com menos de 3 vértices ({count} vértices)")]
    DegenerateFace { face: usize, count: usize },

    #[error("Vértice órfão isolado {vertex} sem arestas incidentes")]
    OrphanVertex { vertex: usize },

    #[error("Ciclo de face {face} corrompido ou aberto")]
    BrokenFaceLoop { face: usize },
}

/// Relatório consolidado de validação topológica.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TopologyReport {
    pub is_manifold: bool,
    pub is_closed: bool,
    pub boundary_edge_count: usize,
    pub boundary_loop_count: usize,
    pub isolated_vertex_count: usize,
    pub defects: Vec<TopologyDefect>,
}

/// Vértice na estrutura Half-Edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HEVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    /// Um half-edge saindo deste vértice (se houver).
    pub half_edge: Option<HalfEdgeId>,
    pub selected: bool,
}

impl HEVertex {
    pub fn vec(&self) -> Vec3 {
        Vec3::from(self.pos)
    }
}

/// Face na estrutura Half-Edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HEFace {
    /// Um dos half-edges pertencentes ao ciclo perimétrico desta face.
    pub half_edge: HalfEdgeId,
    pub selected: bool,
}

/// Aresta direcionada (Half-Edge).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HalfEdge {
    pub origin: VertexId,
    pub target: VertexId,
    /// Face interior deste half-edge (None caso seja contorno virtual de fronteira).
    pub face: Option<FaceId>,
    pub next: HalfEdgeId,
    pub prev: HalfEdgeId,
    /// Half-edge oposto na direção reversa (target -> origin), se existir.
    pub twin: Option<HalfEdgeId>,
    /// Aresta não-direcionada correspondente.
    pub edge: EdgeId,
    /// Coordenada de textura para o vértice de origem neste contexto de face.
    pub uv: [f32; 2],
}

/// Aresta não-direcionada correspondente a um par de gêmeos (ou borda única).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HEEdge {
    pub half_edge: HalfEdgeId,
    pub selected: bool,
}

/// Malha completa representada por Half-Edge.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HalfEdgeMesh {
    pub vertices: Vec<HEVertex>,
    pub faces: Vec<HEFace>,
    pub half_edges: Vec<HalfEdge>,
    pub edges: Vec<HEEdge>,
}

impl HalfEdgeMesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Constrói `HalfEdgeMesh` a partir de uma `Mesh` padrão.
    /// Registra defeitos caso a malha de entrada possua topologia não-manifold.
    pub fn from_mesh(mesh: &Mesh) -> (Self, Vec<TopologyDefect>) {
        let mut defects = Vec::new();
        let mut hem = HalfEdgeMesh::new();

        // 1. Cria vértices
        hem.vertices.reserve(mesh.verts.len());
        for v in &mesh.verts {
            hem.vertices.push(HEVertex {
                pos: v.pos,
                color: v.color,
                half_edge: None,
                selected: v.selected,
            });
        }

        // Mapeamento de aresta direcionada (u, v) -> HalfEdgeId
        let mut directed_edges: HashMap<(u32, u32), HalfEdgeId> = HashMap::new();
        // Mapeamento de aresta não direcionada edge_key(u, v) -> EdgeId
        let mut undirected_edges: HashMap<(u32, u32), EdgeId> = HashMap::new();
        // Rastreador de multiplicidade para detecção de non-manifold
        let mut directed_counts: HashMap<(u32, u32), usize> = HashMap::new();
        let mut undirected_counts: HashMap<(u32, u32), usize> = HashMap::new();

        // 2. Constrói faces e half-edges
        for (fi, face) in mesh.faces.iter().enumerate() {
            let m = face.verts.len();
            if m < 3 {
                defects.push(TopologyDefect::DegenerateFace { face: fi, count: m });
                continue;
            }

            let face_id = FaceId(hem.faces.len());
            let base_he_idx = hem.half_edges.len();

            for k in 0..m {
                let u = face.verts[k];
                let v = face.verts[(k + 1) % m];
                let d_count = directed_counts.entry((u, v)).or_insert(0);
                *d_count += 1;
                if *d_count > 1 {
                    defects.push(TopologyDefect::NonManifoldEdge {
                        u: u as usize,
                        v: v as usize,
                        count: *d_count,
                    });
                }

                let key = edge_key(u, v);
                let u_count = undirected_counts.entry(key).or_insert(0);
                *u_count += 1;
                if *u_count > 2 {
                    defects.push(TopologyDefect::NonManifoldEdge {
                        u: key.0 as usize,
                        v: key.1 as usize,
                        count: *u_count,
                    });
                }

                let key = edge_key(u, v);
                let edge_id = match undirected_edges.get(&key) {
                    Some(&eid) => eid,
                    None => {
                        let eid = EdgeId(hem.edges.len());
                        let is_sel = mesh.selected_edges.contains(&key);
                        hem.edges.push(HEEdge {
                            half_edge: HalfEdgeId(base_he_idx + k),
                            selected: is_sel,
                        });
                        undirected_edges.insert(key, eid);
                        eid
                    }
                };

                let uv = if k < face.uv.len() {
                    face.uv[k]
                } else {
                    [0.0, 0.0]
                };

                let he_id = HalfEdgeId(base_he_idx + k);
                let next_id = HalfEdgeId(base_he_idx + (k + 1) % m);
                let prev_id = HalfEdgeId(base_he_idx + (k + m - 1) % m);

                hem.half_edges.push(HalfEdge {
                    origin: VertexId(u as usize),
                    target: VertexId(v as usize),
                    face: Some(face_id),
                    next: next_id,
                    prev: prev_id,
                    twin: None,
                    edge: edge_id,
                    uv,
                });

                directed_edges.insert((u, v), he_id);

                // Define outgoing half-edge no vértice
                if (u as usize) < hem.vertices.len() {
                    hem.vertices[u as usize].half_edge = Some(he_id);
                }
            }

            hem.faces.push(HEFace {
                half_edge: HalfEdgeId(base_he_idx),
                selected: face.selected,
            });
        }

        // 3. Conecta twins
        for (he_idx, he) in hem.half_edges.iter_mut().enumerate() {
            let rev_key = (he.target.0 as u32, he.origin.0 as u32);
            if let Some(&twin_id) = directed_edges.get(&rev_key)
                && twin_id.0 != he_idx
            {
                he.twin = Some(twin_id);
            }
        }

        // 4. Checa vértices órfãos
        for (vi, v) in hem.vertices.iter().enumerate() {
            if v.half_edge.is_none() {
                defects.push(TopologyDefect::OrphanVertex { vertex: vi });
            }
        }

        (hem, defects)
    }

    /// Converte a representação Half-Edge de volta para `Mesh` indexada.
    pub fn to_mesh(&self) -> Mesh {
        let mut mesh = Mesh::default();

        mesh.verts.reserve(self.vertices.len());
        for v in &self.vertices {
            mesh.verts.push(Vertex {
                pos: v.pos,
                color: v.color,
                selected: v.selected,
            });
        }

        mesh.faces.reserve(self.faces.len());
        for face in &self.faces {
            let mut verts = Vec::new();
            let mut uvs = Vec::new();

            let start = face.half_edge;
            let mut curr = start;
            let mut guard = 0;
            const MAX_VERTS_PER_FACE: usize = 1000;

            loop {
                if curr.0 >= self.half_edges.len() {
                    break;
                }
                let he = &self.half_edges[curr.0];
                verts.push(he.origin.0 as u32);
                uvs.push(he.uv);

                curr = he.next;
                guard += 1;
                if curr == start || guard > MAX_VERTS_PER_FACE {
                    break;
                }
            }

            if verts.len() >= 3 {
                let mut f = Face::with_uv(verts, uvs);
                f.selected = face.selected;
                mesh.faces.push(f);
            }
        }

        // Reconstrói arestas selecionadas
        for edge in &self.edges {
            if edge.selected && edge.half_edge.0 < self.half_edges.len() {
                let he = &self.half_edges[edge.half_edge.0];
                let key = edge_key(he.origin.0 as u32, he.target.0 as u32);
                mesh.selected_edges.insert(key);
            }
        }

        mesh
    }

    /// Valida formalmente os invariantes estruturais da malha.
    pub fn validate_invariants(&self) -> Vec<TopologyDefect> {
        let mut defects = Vec::new();

        // 1. Validação de Half-Edges
        for (i, he) in self.half_edges.iter().enumerate() {
            // Checa next / prev
            if he.next.0 >= self.half_edges.len() || he.prev.0 >= self.half_edges.len() {
                defects.push(TopologyDefect::NextPrevMismatch { he: i });
                continue;
            }

            let next_he = &self.half_edges[he.next.0];
            let prev_he = &self.half_edges[he.prev.0];

            if next_he.prev.0 != i || prev_he.next.0 != i {
                defects.push(TopologyDefect::NextPrevMismatch { he: i });
            }

            // Checa consistência de gêmeos (twin)
            if let Some(twin_id) = he.twin {
                if twin_id.0 >= self.half_edges.len() {
                    defects.push(TopologyDefect::TwinMismatch {
                        he: i,
                        twin: twin_id.0,
                        twin_of_twin: None,
                    });
                } else {
                    let twin_he = &self.half_edges[twin_id.0];
                    if twin_he.twin != Some(HalfEdgeId(i)) {
                        defects.push(TopologyDefect::TwinMismatch {
                            he: i,
                            twin: twin_id.0,
                            twin_of_twin: twin_he.twin.map(|t| t.0),
                        });
                    }
                    if he.origin != twin_he.target || he.target != twin_he.origin {
                        defects.push(TopologyDefect::TwinMismatch {
                            he: i,
                            twin: twin_id.0,
                            twin_of_twin: Some(twin_id.0),
                        });
                    }
                }
            }
        }

        // 2. Validação de Faces
        for (fi, face) in self.faces.iter().enumerate() {
            let start = face.half_edge;
            if start.0 >= self.half_edges.len() {
                defects.push(TopologyDefect::BrokenFaceLoop { face: fi });
                continue;
            }

            let mut curr = start;
            let mut count = 0;
            let mut closed = false;
            let max_steps = self.half_edges.len() + 1;

            while count < max_steps {
                let he = &self.half_edges[curr.0];
                if he.face != Some(FaceId(fi)) {
                    defects.push(TopologyDefect::BrokenFaceLoop { face: fi });
                    break;
                }
                count += 1;
                curr = he.next;
                if curr == start {
                    closed = true;
                    break;
                }
            }

            if !closed || count < 3 {
                if count < 3 {
                    defects.push(TopologyDefect::DegenerateFace { face: fi, count });
                } else {
                    defects.push(TopologyDefect::BrokenFaceLoop { face: fi });
                }
            }
        }

        // 3. Validação de Vértices e Detecção de Non-Manifold Fans
        for (vi, v) in self.vertices.iter().enumerate() {
            if v.half_edge.is_none() {
                defects.push(TopologyDefect::OrphanVertex { vertex: vi });
            } else {
                let star_faces = self.vertex_star_faces(VertexId(vi));
                if star_faces.is_empty() {
                    defects.push(TopologyDefect::OrphanVertex { vertex: vi });
                } else if star_faces.len() > 1 {
                    // Verifica se todas as faces incidentes formam um único leque contínuo ao redor de vi
                    let mut visited = HashSet::new();
                    let mut queue = vec![star_faces[0]];
                    visited.insert(star_faces[0]);

                    while let Some(curr_f) = queue.pop() {
                        let curr_verts = self.face_vertices(curr_f);
                        for &other_f in &star_faces {
                            if !visited.contains(&other_f) {
                                let other_verts = self.face_vertices(other_f);
                                let shares_edge = curr_verts
                                    .iter()
                                    .any(|&cv| cv.0 != vi && other_verts.contains(&cv));
                                if shares_edge {
                                    visited.insert(other_f);
                                    queue.push(other_f);
                                }
                            }
                        }
                    }

                    if visited.len() != star_faces.len() {
                        defects.push(TopologyDefect::NonManifoldVertex {
                            vertex: vi,
                            reason: format!(
                                "Vértice com leques desconectados de faces (pinch/bowtie: {} de {} conectadas)",
                                visited.len(),
                                star_faces.len()
                            ),
                        });
                    }
                }
            }
        }

        defects
    }

    /// Retorna os identificadores dos vértices que compõem o contorno da face em ordem.
    pub fn face_vertices(&self, face: FaceId) -> Vec<VertexId> {
        let mut verts = Vec::new();
        if face.0 >= self.faces.len() {
            return verts;
        }

        let start = self.faces[face.0].half_edge;
        let mut curr = start;
        let mut count = 0;
        let max_verts = self.half_edges.len();

        while count < max_verts {
            if curr.0 >= self.half_edges.len() {
                break;
            }
            let he = &self.half_edges[curr.0];
            verts.push(he.origin);
            curr = he.next;
            count += 1;
            if curr == start {
                break;
            }
        }

        verts
    }

    /// Retorna as coordenadas UV dos vértices da face em ordem.
    pub fn face_uvs(&self, face: FaceId) -> Vec<[f32; 2]> {
        let mut uvs = Vec::new();
        if face.0 >= self.faces.len() {
            return uvs;
        }

        let start = self.faces[face.0].half_edge;
        let mut curr = start;
        let mut count = 0;
        let max_verts = self.half_edges.len();

        while count < max_verts {
            if curr.0 >= self.half_edges.len() {
                break;
            }
            let he = &self.half_edges[curr.0];
            uvs.push(he.uv);
            curr = he.next;
            count += 1;
            if curr == start {
                break;
            }
        }

        uvs
    }

    /// Retorna faces vizinhas compartilhando uma aresta com a face informada.
    pub fn face_neighbors(&self, face: FaceId) -> Vec<FaceId> {
        let mut neighbors = Vec::new();
        if face.0 >= self.faces.len() {
            return neighbors;
        }

        let start = self.faces[face.0].half_edge;
        let mut curr = start;
        let mut count = 0;
        let max_edges = self.half_edges.len();

        while count < max_edges {
            if curr.0 >= self.half_edges.len() {
                break;
            }
            let he = &self.half_edges[curr.0];
            if let Some(twin_id) = he.twin
                && twin_id.0 < self.half_edges.len()
                && let Some(other_face) = self.half_edges[twin_id.0].face
                && other_face != face
            {
                neighbors.push(other_face);
            }
            curr = he.next;
            count += 1;
            if curr == start {
                break;
            }
        }

        neighbors
    }

    /// Estrela de half-edges saindo do vértice (outgoing half-edges).
    pub fn vertex_star_half_edges(&self, v: VertexId) -> Vec<HalfEdgeId> {
        let mut out = Vec::new();
        for (i, he) in self.half_edges.iter().enumerate() {
            if he.origin == v {
                out.push(HalfEdgeId(i));
            }
        }
        out
    }

    /// Faces incidentes ao vértice informado.
    pub fn vertex_star_faces(&self, v: VertexId) -> Vec<FaceId> {
        let mut faces = Vec::new();
        for he_id in self.vertex_star_half_edges(v) {
            if let Some(f) = self.half_edges[he_id.0].face
                && !faces.contains(&f)
            {
                faces.push(f);
            }
        }
        faces
    }

    /// Vértices adjacentes conectados por arestas ao vértice informado.
    pub fn vertex_neighbors(&self, v: VertexId) -> Vec<VertexId> {
        let mut neighbors = Vec::new();
        for he_id in self.vertex_star_half_edges(v) {
            let target = self.half_edges[he_id.0].target;
            if !neighbors.contains(&target) {
                neighbors.push(target);
            }
        }
        neighbors
    }

    /// Half-edges de contorno/fronteira (sem gêmeo / open boundary).
    pub fn boundary_half_edges(&self) -> Vec<HalfEdgeId> {
        self.half_edges
            .iter()
            .enumerate()
            .filter(|(_, he)| he.twin.is_none())
            .map(|(i, _)| HalfEdgeId(i))
            .collect()
    }

    /// Loops fechados de contorno (fronteira) na malha.
    pub fn boundary_loops(&self) -> Vec<Vec<HalfEdgeId>> {
        let boundary_edges = self.boundary_half_edges();
        if boundary_edges.is_empty() {
            return Vec::new();
        }

        // Mapeia target -> half_edge de fronteira (para continuar a caminhada de contorno)
        let mut outgoing_boundaries: HashMap<VertexId, Vec<HalfEdgeId>> = HashMap::new();
        for &be in &boundary_edges {
            let he = &self.half_edges[be.0];
            outgoing_boundaries.entry(he.origin).or_default().push(be);
        }

        let mut visited: HashSet<HalfEdgeId> = HashSet::new();
        let mut loops = Vec::new();

        for &be in &boundary_edges {
            if visited.contains(&be) {
                continue;
            }

            let mut loop_edges = Vec::new();
            let mut curr = be;

            while !visited.contains(&curr) {
                visited.insert(curr);
                loop_edges.push(curr);

                let he = &self.half_edges[curr.0];
                let next_target = he.target;

                // Procura o próximo half-edge de contorno com origem em next_target
                let next_candidates = outgoing_boundaries.get(&next_target);
                if let Some(candidates) = next_candidates {
                    let next_edge = candidates.iter().find(|&&cand| !visited.contains(&cand));
                    match next_edge {
                        Some(&ne) => curr = ne,
                        None => break,
                    }
                } else {
                    break;
                }
            }

            if !loop_edges.is_empty() {
                loops.push(loop_edges);
            }
        }

        loops
    }

    /// Verifica se a malha é uma variedade fechada (2-manifold sem bordas abertas).
    pub fn is_closed_manifold(&self) -> bool {
        self.boundary_half_edges().is_empty() && self.validate_invariants().is_empty()
    }
}

// ---------------------------------------------------------------------------
// Extensões de Validação e Topologia diretamente em Mesh
// ---------------------------------------------------------------------------

impl Mesh {
    /// Produz relatório formal da integridade topológica da malha.
    pub fn validate_topology(&self) -> TopologyReport {
        let (hem, mut initial_defects) = HalfEdgeMesh::from_mesh(self);
        let mut invariant_defects = hem.validate_invariants();
        initial_defects.append(&mut invariant_defects);

        // Deduplica defeitos para o relatório
        let mut seen = HashSet::new();
        let mut unique_defects = Vec::new();
        for d in initial_defects {
            let key = format!("{:?}", d);
            if seen.insert(key) {
                unique_defects.push(d);
            }
        }

        let boundaries = hem.boundary_half_edges();
        let loops = hem.boundary_loops();
        let isolated = self
            .verts
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.faces.iter().any(|f| f.verts.contains(&(*i as u32))))
            .count();

        let is_manifold = unique_defects.is_empty();
        let is_closed = boundaries.is_empty();

        TopologyReport {
            is_manifold,
            is_closed,
            boundary_edge_count: boundaries.len(),
            boundary_loop_count: loops.len(),
            isolated_vertex_count: isolated,
            defects: unique_defects,
        }
    }

    /// Remove vértices órfãos não referenciados por nenhuma face da malha.
    /// Retorna a quantidade de vértices removidos.
    pub fn remove_isolated_vertices(&mut self) -> usize {
        let n = self.verts.len();
        let mut used = vec![false; n];
        for f in &self.faces {
            for &vi in &f.verts {
                if (vi as usize) < n {
                    used[vi as usize] = true;
                }
            }
        }

        let mut remap = vec![u32::MAX; n];
        let mut new_verts = Vec::new();
        for (i, &is_used) in used.iter().enumerate() {
            if is_used {
                remap[i] = new_verts.len() as u32;
                new_verts.push(self.verts[i].clone());
            }
        }

        let removed = n - new_verts.len();
        if removed == 0 {
            return 0;
        }

        for f in &mut self.faces {
            for vi in &mut f.verts {
                *vi = remap[*vi as usize];
            }
        }

        // Atualiza arestas selecionadas
        let old_edges: Vec<(u32, u32)> = self.selected_edges.drain().collect();
        for (a, b) in old_edges {
            let na = remap[a as usize];
            let nb = remap[b as usize];
            if na != u32::MAX && nb != u32::MAX && na != nb {
                self.selected_edges.insert(edge_key(na, nb));
            }
        }

        // Seams e Sharp seguem o mesmo remap: sem isso os flags passariam a
        // apontar para outros vértices da malha depois da compactação
        // (website/docs/03-geometry/normals-shading-sharp-edges.md).
        let remap_edges = |set: &HashSet<(u32, u32)>| -> HashSet<(u32, u32)> {
            let mut out = HashSet::new();
            for &(a, b) in set {
                let na = remap[a as usize];
                let nb = remap[b as usize];
                if na != u32::MAX && nb != u32::MAX && na != nb {
                    out.insert(edge_key(na, nb));
                }
            }
            out
        };
        self.uv_seams = remap_edges(&self.uv_seams);
        self.sharp_edges = remap_edges(&self.sharp_edges);

        self.verts = new_verts;
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_is_closed_manifold() {
        let cube = Mesh::cube(2.0);
        let (hem, defects) = HalfEdgeMesh::from_mesh(&cube);
        assert!(
            defects.is_empty(),
            "Cube should not have initial defects: {:?}",
            defects
        );
        let invariants = hem.validate_invariants();
        assert!(
            invariants.is_empty(),
            "Cube invariants failed: {:?}",
            invariants
        );
        assert!(hem.is_closed_manifold());
        assert_eq!(hem.boundary_half_edges().len(), 0);
        assert_eq!(hem.faces.len(), 6);
        assert_eq!(hem.vertices.len(), 8);
    }

    #[test]
    fn cube_half_edge_roundtrip() {
        let cube = Mesh::cube(2.0);
        let (hem, _) = HalfEdgeMesh::from_mesh(&cube);
        let roundtrip = hem.to_mesh();
        assert_eq!(roundtrip.verts.len(), 8);
        assert_eq!(roundtrip.faces.len(), 6);
    }

    #[test]
    fn plane_has_boundary_loop() {
        let plane = Mesh::plane(2.0);
        let (hem, defects) = HalfEdgeMesh::from_mesh(&plane);
        assert!(defects.is_empty());
        assert!(!hem.is_closed_manifold());
        let boundaries = hem.boundary_half_edges();
        assert_eq!(boundaries.len(), 4);
        let loops = hem.boundary_loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4);
    }

    #[test]
    fn topological_queries() {
        let cube = Mesh::cube(2.0);
        let (hem, _) = HalfEdgeMesh::from_mesh(&cube);
        // Cada face do cubo tem 4 vizinhos (1 para cada aresta)
        for fi in 0..6 {
            let neighbors = hem.face_neighbors(FaceId(fi));
            assert_eq!(neighbors.len(), 4, "Face {} should have 4 neighbors", fi);
        }
        // Vértice no cubo deve ter 3 faces incidentes
        let v_faces = hem.vertex_star_faces(VertexId(0));
        assert_eq!(v_faces.len(), 3);
    }

    #[test]
    fn non_manifold_edge_detected() {
        let mut m = Mesh::plane(2.0);
        // Adiciona uma face com a exata mesma aresta direcionada (0, 3)
        m.faces.push(Face::new(vec![0, 3, 2]));
        let report = m.validate_topology();
        assert!(!report.is_manifold);
        let has_non_manifold = report
            .defects
            .iter()
            .any(|d| matches!(d, TopologyDefect::NonManifoldEdge { .. }));
        assert!(has_non_manifold, "Expected NonManifoldEdge defect");
    }

    #[test]
    fn remove_isolated_vertices() {
        let mut m = Mesh::cube(2.0);
        m.verts.push(Vertex::new(10.0, 10.0, 10.0));
        m.verts.push(Vertex::new(20.0, 20.0, 20.0));
        assert_eq!(m.verts.len(), 10);
        let removed = m.remove_isolated_vertices();
        assert_eq!(removed, 2);
        assert_eq!(m.verts.len(), 8);
        assert_eq!(m.faces.len(), 6);
    }

    #[test]
    fn non_manifold_vertex_detected() {
        let mut m = Mesh::default();
        m.verts.push(Vertex::new(0.0, 0.0, 0.0)); // 0
        m.verts.push(Vertex::new(1.0, 0.0, 0.0)); // 1
        m.verts.push(Vertex::new(0.0, 1.0, 0.0)); // 2
        m.faces.push(Face::new(vec![0, 1, 2]));
        m.verts.push(Vertex::new(-1.0, 0.0, 0.0)); // 3
        m.verts.push(Vertex::new(0.0, -1.0, 0.0)); // 4
        m.faces.push(Face::new(vec![0, 3, 4]));

        let (hem, _) = HalfEdgeMesh::from_mesh(&m);
        let defects = hem.validate_invariants();
        let has_nm_vertex = defects
            .iter()
            .any(|d| matches!(d, TopologyDefect::NonManifoldVertex { vertex: 0, .. }));
        assert!(
            has_nm_vertex,
            "Expected NonManifoldVertex at pinch vertex 0"
        );
    }

    #[test]
    fn remove_isolated_vertices_remaps_seams_and_sharp() {
        // Órfão no início: todo índice real desloca um, forçando o remap.
        let mut mesh = Mesh::plane(2.0);
        mesh.verts.insert(0, Vertex::new(9.0, 9.0, 0.0));
        for face in &mut mesh.faces {
            for vertex in &mut face.verts {
                *vertex += 1;
            }
        }
        mesh.uv_seams.insert(edge_key(1, 2));
        mesh.sharp_edges.insert(edge_key(3, 4));

        assert_eq!(mesh.remove_isolated_vertices(), 1);

        assert_eq!(mesh.verts.len(), 4);
        assert_eq!(mesh.uv_seams, HashSet::from([edge_key(0, 1)]));
        assert_eq!(mesh.sharp_edges, HashSet::from([edge_key(2, 3)]));
    }
}
