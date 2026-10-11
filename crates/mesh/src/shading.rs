//! Normais derivadas, Surface Shading e Sharp Edges (website/docs/03-geometry/normals-shading-sharp-edges.md).
//!
//! Persistem somente decisões autorais: winding, [`SurfaceShading`] (no objeto) e as
//! arestas Sharp (`Mesh::sharp_edges`). Normais e *smoothing islands* são derivadas
//! daqui, por face-corner, e **uma única implementação** serve render e export.

use std::collections::{HashMap, HashSet, VecDeque};

use glam::Vec3;

use crate::{Mesh, edge_key};

/// Política de sombreamento da superfície (propriedade do objeto, não do viewport).
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum SurfaceShading {
    /// Cada corner usa a normal geométrica da face.
    #[default]
    Flat,
    /// Corners compartilham a normal do *smoothing island* (limitado por arestas Sharp).
    Smooth,
}

impl SurfaceShading {
    pub fn from_smooth(smooth: bool) -> Self {
        if smooth { Self::Smooth } else { Self::Flat }
    }

    pub fn is_smooth(self) -> bool {
        self == Self::Smooth
    }
}

/// Vértice de triângulo já sombreado (saída de [`Mesh::shaded_corners`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadedCorner {
    /// Índice do vértice autoral.
    pub vertex: u32,
    /// Face autoral de origem.
    pub face: usize,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

/// Normais por face-corner: `normals[face][corner]`, na ordem de `Face::verts`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CornerNormals {
    pub normals: Vec<Vec<[f32; 3]>>,
}

impl CornerNormals {
    pub fn corner(&self, face: usize, corner: usize) -> [f32; 3] {
        self.normals
            .get(face)
            .and_then(|face| face.get(corner))
            .copied()
            .unwrap_or([0.0, 0.0, 0.0])
    }
}

/// Ângulo interno do polígono no corner `corner` da face (radianos).
fn corner_angle(mesh: &Mesh, face: usize, corner: usize) -> f32 {
    let verts = &mesh.faces[face].verts;
    let n = verts.len();
    let at = |i: usize| mesh.verts[verts[i % n] as usize].vec();
    let p = at(corner);
    let prev = (at(corner + n - 1) - p).normalize_or_zero();
    let next = (at(corner + 1) - p).normalize_or_zero();
    prev.dot(next).clamp(-1.0, 1.0).acos()
}

/// Cálculo canônico das normais por corner. Render e export consomem este resultado.
///
/// Smooth: para cada vértice, as faces incidentes são agrupadas atravessando apenas
/// arestas que **não** são Sharp; cada grupo acumula as normais de face ponderadas
/// pelo ângulo do corner e normaliza.
pub fn compute_corner_normals(mesh: &Mesh, shading: SurfaceShading) -> CornerNormals {
    let face_normals: Vec<Vec3> = (0..mesh.faces.len())
        .map(|fi| {
            if mesh.faces[fi].verts.len() < 3 {
                Vec3::ZERO
            } else {
                mesh.face_normal(fi)
            }
        })
        .collect();
    let mut normals: Vec<Vec<[f32; 3]>> = mesh
        .faces
        .iter()
        .enumerate()
        .map(|(fi, face)| vec![face_normals[fi].to_array(); face.verts.len()])
        .collect();
    if shading == SurfaceShading::Flat {
        return CornerNormals { normals };
    }

    let valid_vertex = |v: u32| (v as usize) < mesh.verts.len();
    // Faces (e posição do corner) incidentes em cada vértice; faces por aresta não orientada.
    let mut at_vertex: HashMap<u32, Vec<(usize, usize)>> = HashMap::new();
    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        if face.verts.len() < 3 || !face.verts.iter().all(|&v| valid_vertex(v)) {
            continue;
        }
        let n = face.verts.len();
        for corner in 0..n {
            at_vertex
                .entry(face.verts[corner])
                .or_default()
                .push((fi, corner));
            let key = edge_key(face.verts[corner], face.verts[(corner + 1) % n]);
            let faces = edge_faces.entry(key).or_default();
            if !faces.contains(&fi) {
                faces.push(fi);
            }
        }
    }

    for (&vertex, incident) in &at_vertex {
        let corner_of: HashMap<usize, usize> = incident.iter().copied().collect();
        let mut visited: HashSet<usize> = HashSet::new();
        for &(start, _) in incident {
            if !visited.insert(start) {
                continue;
            }
            // Smoothing island: BFS por arestas do vértice que não são Sharp.
            let mut island = vec![start];
            let mut queue = VecDeque::from([start]);
            while let Some(fi) = queue.pop_front() {
                let face = &mesh.faces[fi];
                let n = face.verts.len();
                let corner = corner_of[&fi];
                for other in [
                    face.verts[(corner + n - 1) % n],
                    face.verts[(corner + 1) % n],
                ] {
                    let key = edge_key(vertex, other);
                    if mesh.sharp_edges.contains(&key) {
                        continue;
                    }
                    for &neighbour in edge_faces.get(&key).into_iter().flatten() {
                        if corner_of.contains_key(&neighbour) && visited.insert(neighbour) {
                            island.push(neighbour);
                            queue.push_back(neighbour);
                        }
                    }
                }
            }
            let mut sum = Vec3::ZERO;
            for &fi in &island {
                sum += face_normals[fi] * corner_angle(mesh, fi, corner_of[&fi]);
            }
            let shared = sum.normalize_or_zero();
            let resolved = if shared == Vec3::ZERO {
                None
            } else {
                Some(shared.to_array())
            };
            for &fi in &island {
                if let Some(normal) = resolved {
                    normals[fi][corner_of[&fi]] = normal;
                }
            }
        }
    }
    CornerNormals { normals }
}

impl Mesh {
    /// Marca arestas como Sharp (decisão autoral; não é seam nem feature edge).
    /// Ignora self-edges e vértices inexistentes. Devolve quantas arestas mudaram.
    pub fn mark_sharp(&mut self, edges: impl IntoIterator<Item = (u32, u32)>) -> usize {
        let mut changed = 0;
        for (a, b) in edges {
            if a == b || (a as usize) >= self.verts.len() || (b as usize) >= self.verts.len() {
                continue;
            }
            if self.sharp_edges.insert(edge_key(a, b)) {
                changed += 1;
            }
        }
        changed
    }

    /// Remove a marca Sharp das arestas. Devolve quantas arestas mudaram.
    pub fn clear_sharp(&mut self, edges: impl IntoIterator<Item = (u32, u32)>) -> usize {
        edges
            .into_iter()
            .filter(|&(a, b)| self.sharp_edges.remove(&edge_key(a, b)))
            .count()
    }

    pub fn is_sharp(&self, a: u32, b: u32) -> bool {
        self.sharp_edges.contains(&edge_key(a, b))
    }

    /// Smooth by Angle: grava como Sharp as arestas internas cujo ângulo diedral
    /// excede `threshold_deg`. Arestas de borda não são marcadas (já delimitam o
    /// smoothing). Não cria estado permanente de Auto Smooth. Devolve quantas
    /// arestas foram marcadas.
    pub fn smooth_by_angle(&mut self, threshold_deg: f32) -> usize {
        let threshold = threshold_deg.clamp(0.0, 180.0).to_radians();
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            if n < 3 {
                continue;
            }
            for k in 0..n {
                edge_faces
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_default()
                    .push(fi);
            }
        }
        let mut to_mark = Vec::new();
        for (&edge, faces) in &edge_faces {
            if faces.len() != 2 {
                continue;
            }
            let a = self.face_normal(faces[0]);
            let b = self.face_normal(faces[1]);
            if a.dot(b).clamp(-1.0, 1.0).acos() > threshold {
                to_mark.push(edge);
            }
        }
        self.mark_sharp(to_mark)
    }

    /// Vértices de triângulo com a normal por corner do cálculo canônico, na mesma
    /// triangulação de render e picking. Mantém o índice do vértice autoral (skin,
    /// export) e a face de origem.
    pub fn shaded_corners(&self, shading: SurfaceShading) -> Vec<ShadedCorner> {
        let corner_normals = compute_corner_normals(self, shading);
        let mut out = Vec::new();
        for (fi, face) in self.faces.iter().enumerate() {
            for corners in self.face_triangle_corners(fi) {
                for corner in corners {
                    let vertex = &self.verts[face.verts[corner] as usize];
                    out.push(ShadedCorner {
                        vertex: face.verts[corner],
                        face: fi,
                        position: vertex.pos,
                        normal: corner_normals.corner(fi, corner),
                        color: vertex.color,
                        uv: face.uv.get(corner).copied().unwrap_or([0.0, 0.0]),
                    });
                }
            }
        }
        out
    }

    /// Triangulação por corner com normais do cálculo canônico.
    /// Saída: `(posição, normal, cor, uv)` por vértice de triângulo.
    pub fn to_triangles_shaded(&self, shading: SurfaceShading) -> Vec<crate::Tri> {
        self.shaded_corners(shading)
            .into_iter()
            .map(|c| (c.position, c.normal, c.color, c.uv))
            .collect()
    }

    /// OBJ com `vn` por face-corner (índices de posição/uv/normal separados).
    pub fn to_obj_with_normals(&self, shading: SurfaceShading) -> String {
        let corner_normals = compute_corner_normals(self, shading);
        let mut s = String::from("# petunia3d\n");
        for v in &self.verts {
            s.push_str(&format!("v {} {} {}\n", v.pos[0], v.pos[1], v.pos[2]));
        }
        let mut vt_index: Vec<Vec<usize>> = Vec::new();
        let mut next_vt = 1usize;
        for f in &self.faces {
            let mut row = Vec::new();
            for q in &f.uv {
                s.push_str(&format!("vt {} {}\n", q[0], 1.0 - q[1]));
                row.push(next_vt);
                next_vt += 1;
            }
            vt_index.push(row);
        }
        // Normais únicas (por bits) para reaproveitar índices entre corners.
        let mut normal_ids: HashMap<[u32; 3], usize> = HashMap::new();
        let mut normal_lines = String::new();
        let mut face_lines = String::new();
        for (fi, (f, vt)) in self.faces.iter().zip(&vt_index).enumerate() {
            face_lines.push('f');
            for (k, &vi) in f.verts.iter().enumerate() {
                let n = corner_normals.corner(fi, k);
                let key = [n[0].to_bits(), n[1].to_bits(), n[2].to_bits()];
                let next_id = normal_ids.len() + 1;
                let id = *normal_ids.entry(key).or_insert_with(|| {
                    normal_lines.push_str(&format!("vn {} {} {}\n", n[0], n[1], n[2]));
                    next_id
                });
                match vt.get(k) {
                    Some(t) => face_lines.push_str(&format!(" {}/{}/{}", vi + 1, t, id)),
                    None => face_lines.push_str(&format!(" {}//{}", vi + 1, id)),
                }
            }
            face_lines.push('\n');
        }
        s.push_str(&normal_lines);
        s.push_str(&face_lines);
        s
    }

    /// Inverte o winding das faces selecionadas (todas, se nenhuma estiver selecionada).
    pub fn flip_faces(&mut self) {
        self.flip_normals();
    }

    /// Propaga winding consistente por componente conexo. Funciona para malhas abertas
    /// e fechadas; **não** afirma qual lado é o exterior.
    pub fn orient_consistently(&mut self) {
        self.recalculate_normals_with(false);
    }

    /// Orienta consistentemente e escolhe o exterior pelo volume com sinal. Só vale para
    /// componentes fechados e manifold; malhas abertas são recusadas.
    pub fn orient_outward(&mut self) -> Result<(), OrientError> {
        if !self.is_closed_manifold() {
            return Err(OrientError::OpenMesh);
        }
        self.recalculate_normals_with(true);
        Ok(())
    }

    /// Toda aresta pertence a exatamente duas faces.
    pub fn is_closed_manifold(&self) -> bool {
        if self.faces.is_empty() {
            return false;
        }
        let mut count: HashMap<(u32, u32), usize> = HashMap::new();
        for face in &self.faces {
            let n = face.verts.len();
            if n < 3 {
                return false;
            }
            for k in 0..n {
                *count
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_default() += 1;
            }
        }
        count.values().all(|&c| c == 2)
    }
}

/// Falhas de orientação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OrientError {
    #[error("Orient Outward só vale para malhas fechadas e manifold")]
    OpenMesh,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    #[test]
    fn flat_cube_has_face_normals_on_every_corner() {
        let cube = Mesh::cube(1.0);
        let normals = compute_corner_normals(&cube, SurfaceShading::Flat);
        for (fi, face) in cube.faces.iter().enumerate() {
            let expected = cube.face_normal(fi).to_array();
            for corner in 0..face.verts.len() {
                assert!(close(normals.corner(fi, corner), expected));
            }
        }
    }

    #[test]
    fn smooth_cube_without_sharp_averages_to_the_diagonal() {
        let cube = Mesh::cube(1.0);
        let normals = compute_corner_normals(&cube, SurfaceShading::Smooth);
        let diagonal = 1.0 / 3.0f32.sqrt();
        for (fi, face) in cube.faces.iter().enumerate() {
            for corner in 0..face.verts.len() {
                let n = normals.corner(fi, corner);
                assert!(
                    close([n[0].abs(), n[1].abs(), n[2].abs()], [diagonal; 3]),
                    "{n:?}"
                );
            }
        }
    }

    #[test]
    fn smooth_cube_with_every_edge_sharp_looks_flat() {
        let mut cube = Mesh::cube(1.0);
        let edges: Vec<_> = cube
            .faces
            .iter()
            .flat_map(|f| {
                let n = f.verts.len();
                (0..n)
                    .map(|k| (f.verts[k], f.verts[(k + 1) % n]))
                    .collect::<Vec<_>>()
            })
            .collect();
        cube.mark_sharp(edges);
        let smooth = compute_corner_normals(&cube, SurfaceShading::Smooth);
        let flat = compute_corner_normals(&cube, SurfaceShading::Flat);
        for fi in 0..cube.faces.len() {
            for corner in 0..cube.faces[fi].verts.len() {
                assert!(close(smooth.corner(fi, corner), flat.corner(fi, corner)));
            }
        }
    }

    #[test]
    fn sharp_boundary_around_a_face_splits_its_corner_normals() {
        let mut cube = Mesh::cube(1.0);
        let face = cube.faces[0].verts.clone();
        let n = face.len();
        cube.mark_sharp((0..n).map(|k| (face[k], face[(k + 1) % n])));
        assert!(cube.uv_seams.is_empty(), "Sharp não cria seam");
        let normals = compute_corner_normals(&cube, SurfaceShading::Smooth);
        let expected = cube.face_normal(0).to_array();
        for corner in 0..n {
            assert!(
                close(normals.corner(0, corner), expected),
                "a face isolada fica plana"
            );
        }
        // As outras faces seguem suaves entre si no mesmo vértice.
        let other = normals.corner(1, 0);
        assert!(!close(other, cube.face_normal(1).to_array()) || cube.faces.len() == 1);
    }

    #[test]
    fn uv_seam_without_sharp_keeps_normals_smooth() {
        let mut cube = Mesh::cube(1.0);
        let (a, b) = (cube.faces[0].verts[0], cube.faces[0].verts[1]);
        cube.uv_seams.insert(edge_key(a, b));
        let with_seam = compute_corner_normals(&cube, SurfaceShading::Smooth);
        cube.uv_seams.clear();
        let without = compute_corner_normals(&cube, SurfaceShading::Smooth);
        assert_eq!(with_seam, without, "seam não afeta normais");
    }

    #[test]
    fn smooth_by_angle_marks_cube_edges_but_not_a_flat_grid() {
        let mut cube = Mesh::cube(1.0);
        assert_eq!(cube.smooth_by_angle(30.0), 12);
        let mut plane = Mesh::plane(2.0);
        assert_eq!(plane.smooth_by_angle(30.0), 0);
    }

    #[test]
    fn mark_and_clear_sharp_ignore_invalid_edges() {
        let mut cube = Mesh::cube(1.0);
        assert_eq!(cube.mark_sharp([(0, 0), (0, 999)]), 0);
        assert_eq!(cube.mark_sharp([(0, 1)]), 1);
        assert_eq!(cube.mark_sharp([(1, 0)]), 0, "mesma aresta não orientada");
        assert_eq!(cube.clear_sharp([(1, 0)]), 1);
        assert!(cube.sharp_edges.is_empty());
    }

    #[test]
    fn orient_outward_refuses_open_meshes() {
        let mut plane = Mesh::plane(1.0);
        assert_eq!(plane.orient_outward(), Err(OrientError::OpenMesh));
        plane.orient_consistently();
    }

    #[test]
    fn shaded_triangles_use_the_canonical_normals() {
        let cube = Mesh::cube(1.0);
        let tris = cube.to_triangles_shaded(SurfaceShading::Smooth);
        let diagonal = 1.0 / 3.0f32.sqrt();
        assert!(!tris.is_empty());
        for (_, n, _, _) in tris {
            assert!(close([n[0].abs(), n[1].abs(), n[2].abs()], [diagonal; 3]));
        }
    }
}
