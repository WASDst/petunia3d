//! Operações do Poly Pen (ADR 007, Onda 5; modelo: Polygon Pen do Cinema 4D).
//!
//! - [`Mesh::add_pen_polygon`]: um polígono desenhado ponto a ponto, que pode
//!   reaproveitar pontos existentes. A orientação segue os vizinhos (aresta
//!   compartilhada percorrida no sentido oposto) ou, sem vizinhos, fica voltada
//!   para quem olha.
//! - [`Mesh::extrude_edge`]: Ctrl-arrastar uma aresta de borda cria um quad
//!   novo a partir dela; os pontos novos ficam selecionados para o arrasto.
//!
//! Ambas recusam resultados não manifold em vez de corromper a malha.

use std::collections::HashMap;

use glam::Vec3;

use crate::{Face, Mesh, Vertex, edge_key};

/// Ponto de um polígono do Poly Pen: existente ou novo (em mundo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PenPoint {
    Existing(u32),
    New([f32; 3]),
}

/// Por que a operação foi recusada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PolyPenError {
    #[error("a polygon needs at least 3 different points")]
    TooFewPoints,
    #[error("point does not exist")]
    MissingPoint,
    #[error("polygon is degenerate")]
    Degenerate,
    #[error("the new face would share an edge with two faces already")]
    NonManifold,
    #[error("only border edges can be extruded")]
    NotBorderEdge,
}

impl Mesh {
    /// Faces que usam cada aresta dirigida `a → b`.
    fn directed_edges(&self) -> HashMap<(u32, u32), usize> {
        let mut edges = HashMap::new();
        for (index, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            for k in 0..n {
                edges.insert((face.verts[k], face.verts[(k + 1) % n]), index);
            }
        }
        edges
    }

    fn undirected_use(&self) -> HashMap<(u32, u32), usize> {
        let mut uses = HashMap::new();
        for face in &self.faces {
            let n = face.verts.len();
            for k in 0..n {
                *uses
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_insert(0) += 1;
            }
        }
        uses
    }

    /// Adiciona o polígono e devolve o índice da face nova. `viewer` aponta da
    /// superfície para a câmera (usado só quando não há vizinhos).
    pub fn add_pen_polygon(
        &mut self,
        points: &[PenPoint],
        viewer: Vec3,
    ) -> Result<usize, PolyPenError> {
        let mut indices = Vec::with_capacity(points.len());
        let mut new_vertices = Vec::new();
        for point in points {
            match *point {
                PenPoint::Existing(index) => {
                    if index as usize >= self.verts.len() {
                        return Err(PolyPenError::MissingPoint);
                    }
                    indices.push(index);
                }
                PenPoint::New(position) => {
                    if !position.iter().all(|v| v.is_finite()) {
                        return Err(PolyPenError::Degenerate);
                    }
                    let index = (self.verts.len() + new_vertices.len()) as u32;
                    let mut vertex = Vertex::new(position[0], position[1], position[2]);
                    if let Some(first) = self.verts.first() {
                        vertex.color = first.color;
                    }
                    new_vertices.push(vertex);
                    indices.push(index);
                }
            }
        }
        let mut distinct = indices.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if indices.len() < 3 || distinct.len() != indices.len() {
            return Err(PolyPenError::TooFewPoints);
        }
        let position = |index: u32| -> Vec3 {
            let i = index as usize;
            if i < self.verts.len() {
                self.verts[i].vec()
            } else {
                new_vertices[i - self.verts.len()].vec()
            }
        };
        let n = indices.len();
        let mut normal = Vec3::ZERO;
        for k in 0..n {
            normal += position(indices[k]).cross(position(indices[(k + 1) % n]));
        }
        if normal.length() < 1.0e-8 {
            return Err(PolyPenError::Degenerate);
        }

        // Aresta compartilhada com um vizinho: percorrê-la no sentido oposto.
        let directed = self.directed_edges();
        let same_way = (0..n).any(|k| directed.contains_key(&(indices[k], indices[(k + 1) % n])));
        let opposite = (0..n).any(|k| directed.contains_key(&(indices[(k + 1) % n], indices[k])));
        if same_way && opposite {
            return Err(PolyPenError::NonManifold);
        }
        if same_way || (!opposite && normal.dot(viewer) < 0.0) {
            indices.reverse();
        }
        let uses = self.undirected_use();
        if (0..n).any(|k| {
            uses.get(&edge_key(indices[k], indices[(k + 1) % n]))
                .is_some_and(|&count| count >= 2)
        }) {
            return Err(PolyPenError::NonManifold);
        }

        self.verts.extend(new_vertices);
        let face_index = self.faces.len();
        self.push_face(Face::new(indices));
        Ok(face_index)
    }

    /// Extruda a aresta de borda `a–b`: cria `a'`, `b'` sobre os originais e o
    /// quad que os liga. Só `a'` e `b'` ficam selecionados (a aresta nova).
    /// Devolve `(a', b', face)`.
    pub fn extrude_edge(&mut self, a: u32, b: u32) -> Result<(u32, u32, usize), PolyPenError> {
        if a == b || a as usize >= self.verts.len() || b as usize >= self.verts.len() {
            return Err(PolyPenError::MissingPoint);
        }
        let directed = self.directed_edges();
        let (from, to) = match (
            directed.contains_key(&(a, b)),
            directed.contains_key(&(b, a)),
        ) {
            (true, false) => (a, b),
            (false, true) => (b, a),
            _ => return Err(PolyPenError::NotBorderEdge),
        };
        let base = self.verts.len() as u32;
        let (from_new, to_new) = (base, base + 1);
        for index in [from, to] {
            let mut vertex = self.verts[index as usize].clone();
            vertex.selected = false;
            self.verts.push(vertex);
        }
        // O vizinho percorre from → to; o quad novo percorre to → from.
        let face_index = self.faces.len();
        self.push_face(Face::new(vec![to, from, from_new, to_new]));
        self.deselect_all();
        self.verts[from_new as usize].selected = true;
        self.verts[to_new as usize].selected = true;
        self.selected_edges.clear();
        self.selected_edges.insert(edge_key(from_new, to_new));
        let (a_new, b_new) = if from == a {
            (from_new, to_new)
        } else {
            (to_new, from_new)
        };
        Ok((a_new, b_new, face_index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quad no plano XY (normal +Z) com arestas de borda.
    fn quad() -> Mesh {
        let mut mesh = Mesh::default();
        for [x, y] in [[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
            mesh.verts.push(Vertex::new(x, y, 0.0));
        }
        mesh.faces.push(Face::new(vec![0, 1, 2, 3]));
        mesh
    }

    fn manifold(mesh: &Mesh) -> bool {
        mesh.undirected_use().values().all(|&count| count <= 2)
            && mesh.directed_edges().len()
                == mesh.faces.iter().map(|f| f.verts.len()).sum::<usize>()
    }

    #[test]
    fn free_polygon_faces_the_viewer() {
        let mut mesh = Mesh::default();
        let points = [
            PenPoint::New([0.0, 0.0, 0.0]),
            PenPoint::New([0.0, 1.0, 0.0]),
            PenPoint::New([1.0, 0.0, 0.0]),
        ];
        let face = mesh.add_pen_polygon(&points, Vec3::Z).unwrap();
        assert!(mesh.face_normal(face).z > 0.9, "voltado para +Z");
        let mut back = Mesh::default();
        let face = back.add_pen_polygon(&points, -Vec3::Z).unwrap();
        assert!(back.face_normal(face).z < -0.9);
    }

    #[test]
    fn polygon_on_a_border_edge_follows_the_neighbor_winding() {
        let mut mesh = quad();
        // Reaproveita a aresta 1–2 e desenha à direita, na ordem "errada".
        let points = [
            PenPoint::Existing(1),
            PenPoint::Existing(2),
            PenPoint::New([2.0, 0.5, 0.0]),
        ];
        let face = mesh.add_pen_polygon(&points, -Vec3::Z).unwrap();
        assert_eq!(mesh.verts.len(), 5);
        assert!(
            mesh.face_normal(face).z > 0.9,
            "mesma orientação do vizinho"
        );
        assert!(manifold(&mesh));
    }

    #[test]
    fn invalid_polygons_are_refused_without_changes() {
        let mut mesh = quad();
        let before = (mesh.verts.len(), mesh.faces.len());
        let duplicate = [
            PenPoint::Existing(0),
            PenPoint::Existing(0),
            PenPoint::Existing(1),
        ];
        assert_eq!(
            mesh.add_pen_polygon(&duplicate, Vec3::Z),
            Err(PolyPenError::TooFewPoints)
        );
        let collinear = [
            PenPoint::Existing(0),
            PenPoint::Existing(1),
            PenPoint::New([2.0, 0.0, 0.0]),
        ];
        assert_eq!(
            mesh.add_pen_polygon(&collinear, Vec3::Z),
            Err(PolyPenError::Degenerate)
        );
        assert_eq!(
            mesh.add_pen_polygon(
                &[
                    PenPoint::Existing(9),
                    PenPoint::Existing(0),
                    PenPoint::Existing(1)
                ],
                Vec3::Z
            ),
            Err(PolyPenError::MissingPoint)
        );
        // Terceira face na aresta 0–1: recusada.
        mesh.add_pen_polygon(
            &[
                PenPoint::Existing(1),
                PenPoint::Existing(0),
                PenPoint::New([0.5, -1.0, 0.0]),
            ],
            Vec3::Z,
        )
        .unwrap();
        let before_third = mesh.faces.len();
        assert_eq!(
            mesh.add_pen_polygon(
                &[
                    PenPoint::Existing(0),
                    PenPoint::Existing(1),
                    PenPoint::New([0.5, -1.0, 1.0])
                ],
                Vec3::Z
            ),
            Err(PolyPenError::NonManifold)
        );
        assert_eq!(mesh.faces.len(), before_third);
        assert!(before.1 < mesh.faces.len());
    }

    #[test]
    fn extruding_a_border_edge_adds_a_consistent_quad() {
        let mut mesh = quad();
        let (a, b, face) = mesh.extrude_edge(1, 2).unwrap();
        assert_eq!(mesh.verts.len(), 6);
        assert_eq!(mesh.selected_edges.len(), 1);
        assert!(mesh.selected_edges.contains(&edge_key(a, b)));
        assert!(mesh.verts[a as usize].selected && mesh.verts[b as usize].selected);
        assert_eq!(mesh.verts.iter().filter(|v| v.selected).count(), 2);
        // Move a aresta nova para fora e confere a orientação.
        for index in [a, b] {
            mesh.verts[index as usize].pos[0] += 1.0;
        }
        assert!(mesh.face_normal(face).z > 0.9);
        assert!(manifold(&mesh));
    }

    #[test]
    fn interior_edges_cannot_be_extruded() {
        let mut mesh = quad();
        mesh.add_pen_polygon(
            &[
                PenPoint::Existing(1),
                PenPoint::Existing(2),
                PenPoint::New([2.0, 0.5, 0.0]),
            ],
            Vec3::Z,
        )
        .unwrap();
        assert_eq!(mesh.extrude_edge(1, 2), Err(PolyPenError::NotBorderEdge));
        assert_eq!(mesh.extrude_edge(1, 1), Err(PolyPenError::MissingPoint));
    }
}
