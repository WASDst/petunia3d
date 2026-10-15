//! Chanfro transacional de aresta convexa com extremos trivalentes.

use std::collections::HashMap;

use crate::{Face, Mesh, Vertex};

/// Limite de segmentos do kernel (a UI comum oferece 1–8).
pub const MAX_BEVEL_SEGMENTS: u32 = 16;

/// Resultado de [`Mesh::bevel_edges`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BevelReport {
    pub requested: f32,
    pub segments: u32,
    /// Arestas chanfradas (0 quando o conjunto foi recusado).
    pub beveled_edges: usize,
    /// Arestas cujo offset pedido foi reduzido pelo limite de overlap
    /// disponível (0.45 da aresta vizinha com `clamp_overlap`, teto duro de
    /// 0.95 sem ele). Zero quando a largura pedida foi aplicada por inteiro.
    pub clamped_edges: usize,
    /// Arestas (`edge_key` da malha original) sem solução: com qualquer uma
    /// aqui, a malha não mudou. Numa recusa por interação entre arestas do
    /// conjunto, contém **todas** as arestas pedidas — o conjunto é a unidade
    /// transacional, não a aresta individual.
    pub unsupported_edges: Vec<(u32, u32)>,
}

/// Chanfro transacional com suporte a multi-segmentos e clamp overlap opcional.
/// Devolve `(malha, clamped)`: `clamped = true` quando o offset pedido excedeu
/// o limite de overlap e foi reduzido em pelo menos um extremo, para o
/// chamador reportar a largura efetiva sem re-auditar a geometria.
pub(crate) fn bevel_edge_segments_clamped(
    mesh: &Mesh,
    a: u32,
    b: u32,
    amount: f32,
    segments: u32,
    clamp_overlap: bool,
) -> Option<(Mesh, bool)> {
    if mesh.verts.iter().any(|v| !v.vec().is_finite())
        || mesh.faces.iter().any(|face| {
            face.verts.len() < 3
                || face.uv.len() != face.verts.len()
                || face.verts.iter().any(|&v| v as usize >= mesh.verts.len())
        })
    {
        return None;
    }
    let adjacent = mesh.edge_faces(a, b);
    if adjacent.len() != 2 || a == b {
        return None;
    }
    let input_report = mesh.validate_topology();
    if !input_report.is_manifold {
        return None;
    }
    for endpoint in [a, b] {
        if mesh
            .faces
            .iter()
            .filter(|f| f.verts.contains(&endpoint))
            .count()
            != 3
        {
            return None;
        }
    }
    let origin = mesh.verts.get(a as usize)?.vec();
    for (&face, &other) in adjacent.iter().zip(adjacent.iter().rev()) {
        let normal = mesh.face_normal(face);
        if !normal.is_finite() || normal.length_squared() < 0.5 {
            return None;
        }
        // Uma dobra côncava exige uma solução diferente de interseção de planos.
        if mesh.faces[other].verts.iter().any(|&v| {
            mesh.verts
                .get(v as usize)
                .is_none_or(|p| !p.vec().is_finite() || normal.dot(p.vec() - origin) > 1e-5)
        }) {
            return None;
        }
    }
    if mesh
        .face_normal(adjacent[0])
        .dot(mesh.face_normal(adjacent[1]))
        > 0.9999
    {
        return None;
    }

    let mut result = mesh.clone();
    let mut clamped = false;
    // Cada offset fica em uma aresta já existente; preserva os planos incidentes.
    let mut offsets: HashMap<(u32, u32), (u32, f32)> = HashMap::new();
    for &fi in &adjacent {
        let face = &mesh.faces[fi];
        for (endpoint, other) in [(a, b), (b, a)] {
            let k = face.verts.iter().position(|&v| v == endpoint)?;
            let prev = face.verts[(k + face.verts.len() - 1) % face.verts.len()];
            let next = face.verts[(k + 1) % face.verts.len()];
            let neighbor = if prev == other {
                next
            } else if next == other {
                prev
            } else {
                return None;
            };
            let mut vertex = mesh.verts.get(endpoint as usize)?.clone();
            let target = mesh.verts.get(neighbor as usize)?;
            let length = vertex.vec().distance(target.vec());
            if !length.is_finite() || length <= 1e-6 {
                return None;
            }
            let max_t = if clamp_overlap { 0.45 } else { 0.95 };
            let requested_t = amount / length;
            let t = requested_t.min(max_t);
            if requested_t > max_t {
                clamped = true;
            }
            vertex.pos = vertex.vec().lerp(target.vec(), t).to_array();
            for (color, target_color) in vertex.color.iter_mut().zip(target.color) {
                *color += (target_color - *color) * t;
            }
            vertex.selected = true;
            let id = u32::try_from(result.verts.len()).ok()?;
            result.verts.push(vertex);
            offsets.insert((endpoint, neighbor), (id, t));
        }
    }
    if offsets.len() != 4 {
        return None;
    }

    let edge_offsets = |fi: usize| -> Option<(u32, u32)> {
        let face = &mesh.faces[fi];
        let endpoint_offset = |endpoint, other| -> Option<u32> {
            let k = face.verts.iter().position(|&v| v == endpoint)?;
            let prev = face.verts[(k + face.verts.len() - 1) % face.verts.len()];
            let next = face.verts[(k + 1) % face.verts.len()];
            let neighbor = if prev == other { next } else { prev };
            offsets.get(&(endpoint, neighbor)).map(|&(id, _)| id)
        };
        Some((endpoint_offset(a, b)?, endpoint_offset(b, a)?))
    };
    let (a1, b1) = edge_offsets(adjacent[0])?;
    let (a2, b2) = edge_offsets(adjacent[1])?;
    let face = &mesh.faces[adjacent[0]];
    let k = face.verts.iter().position(|&v| v == a)?;
    let winding_reversed = face.verts[(k + 1) % face.verts.len()] == b;

    let segs = segments.clamp(1, MAX_BEVEL_SEGMENTS) as usize;
    let mut rings: Vec<(u32, u32)> = Vec::with_capacity(segs + 1);
    rings.push((a1, b1));

    if segs > 1 {
        let p_a = mesh.verts[a as usize].vec();
        let p_b = mesh.verts[b as usize].vec();
        let va1 = result.verts[a1 as usize].vec();
        let va2 = result.verts[a2 as usize].vec();
        let vb1 = result.verts[b1 as usize].vec();
        let vb2 = result.verts[b2 as usize].vec();
        let ma = (va1 + va2) * 0.5;
        let mb = (vb1 + vb2) * 0.5;

        for s in 1..segs {
            let t = s as f32 / segs as f32;
            let bulge = (1.0 - (2.0 * t - 1.0) * (2.0 * t - 1.0)) * 0.4142;
            let pos_a = va1.lerp(va2, t) + (p_a - ma) * bulge;
            let pos_b = vb1.lerp(vb2, t) + (p_b - mb) * bulge;

            let mut va = result.verts[a1 as usize].clone();
            va.pos = pos_a.to_array();
            va.selected = true;
            let idx_a = result.verts.len() as u32;
            result.verts.push(va);

            let mut vb = result.verts[b1 as usize].clone();
            vb.pos = pos_b.to_array();
            vb.selected = true;
            let idx_b = result.verts.len() as u32;
            result.verts.push(vb);

            rings.push((idx_a, idx_b));
        }
    }
    rings.push((a2, b2));

    for (fi, face) in mesh.faces.iter().enumerate() {
        if face.uv.len() != face.verts.len() {
            return None;
        }
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        for (k, &vertex) in face.verts.iter().enumerate() {
            if vertex != a && vertex != b {
                vertices.push(vertex);
                uvs.push(face.uv[k]);
                continue;
            }
            let prev = (k + face.verts.len() - 1) % face.verts.len();
            let next = (k + 1) % face.verts.len();
            let prev_neighbor = face.verts[prev];
            let next_neighbor = face.verts[next];

            let is_adjacent = adjacent.contains(&fi);
            if is_adjacent {
                for (neighbor_k, neighbor) in [(prev, prev_neighbor), (next, next_neighbor)] {
                    if neighbor == a || neighbor == b {
                        continue;
                    }
                    let &(offset, t) = offsets.get(&(vertex, neighbor))?;
                    vertices.push(offset);
                    uvs.push([
                        face.uv[k][0] + (face.uv[neighbor_k][0] - face.uv[k][0]) * t,
                        face.uv[k][1] + (face.uv[neighbor_k][1] - face.uv[k][1]) * t,
                    ]);
                }
            } else {
                let &(offset_prev, t_prev) = offsets.get(&(vertex, prev_neighbor))?;
                let &(offset_next, t_next) = offsets.get(&(vertex, next_neighbor))?;
                let uv_prev = [
                    face.uv[k][0] + (face.uv[prev][0] - face.uv[k][0]) * t_prev,
                    face.uv[k][1] + (face.uv[prev][1] - face.uv[k][1]) * t_prev,
                ];
                let uv_next = [
                    face.uv[k][0] + (face.uv[next][0] - face.uv[k][0]) * t_next,
                    face.uv[k][1] + (face.uv[next][1] - face.uv[k][1]) * t_next,
                ];

                vertices.push(offset_prev);
                uvs.push(uv_prev);

                if segs > 1 {
                    let is_vertex_a = vertex == a;
                    let forward = if is_vertex_a {
                        offset_prev == a1
                    } else {
                        offset_prev == b1
                    };
                    for s in 1..segs {
                        let step = if forward { s } else { segs - s };
                        let ring_v = if is_vertex_a {
                            rings[step].0
                        } else {
                            rings[step].1
                        };
                        let alpha = s as f32 / segs as f32;
                        let uv_interp = [
                            uv_prev[0] + (uv_next[0] - uv_prev[0]) * alpha,
                            uv_prev[1] + (uv_next[1] - uv_prev[1]) * alpha,
                        ];
                        vertices.push(ring_v);
                        uvs.push(uv_interp);
                    }
                }

                vertices.push(offset_next);
                uvs.push(uv_next);
            }
        }
        result.faces[fi] = Face::with_uv(vertices, uvs);
    }

    for s in 0..segs {
        let (ra0, rb0) = rings[s];
        let (ra1, rb1) = rings[s + 1];
        let indices = if winding_reversed {
            vec![rb0, ra0, ra1, rb1]
        } else {
            vec![ra0, rb0, rb1, ra1]
        };
        let mut ribbon = Face::with_uv(
            indices,
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        );
        ribbon.selected = true;
        result.faces.push(ribbon);
    }

    result.selected_edges.clear();
    result.remove_isolated_vertices();
    result.sync_vert_selection_from_faces();
    let report = result.validate_topology();
    if !report.is_manifold || report.is_closed != input_report.is_closed {
        return None;
    }
    // O offset por arestas nem sempre produz ribbon plano em sólidos irregulares.
    // Rejeita o candidato inteiro antes de publicar uma face torcida/degenerada.
    for (fi, face) in result.faces.iter().enumerate() {
        let normal = result.face_normal(fi);
        let origin = result.verts.get(*face.verts.first()? as usize)?.vec();
        if !normal.is_finite()
            || normal.length_squared() < 0.5
            || face.verts.iter().any(|&v| {
                let offset = result.verts[v as usize].vec() - origin;
                !offset.is_finite() || normal.dot(offset).abs() > 1e-5 * offset.length().max(1.0)
            })
        {
            return None;
        }
    }
    Some((result, clamped))
}

/// Chanfro transacional de vértice com preservação estrita de 2-manifold e fechamento.
pub(crate) fn bevel_vertex(mesh: &Mesh, v: u32, amount: f32, clamp_overlap: bool) -> Option<Mesh> {
    if !amount.is_finite() || amount <= 1e-5 {
        return None;
    }
    let v_idx = v as usize;
    if v_idx >= mesh.verts.len() || !mesh.verts[v_idx].vec().is_finite() {
        return None;
    }
    if mesh.faces.iter().any(|face| {
        face.verts.len() < 3
            || face.uv.len() != face.verts.len()
            || face
                .verts
                .iter()
                .any(|&idx| idx as usize >= mesh.verts.len())
    }) {
        return None;
    }

    let input_report = mesh.validate_topology();
    if !input_report.is_manifold {
        return None;
    }

    // Identifica as faces incidentes ao vértice v
    let incident_faces: Vec<usize> = mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.verts.contains(&v))
        .map(|(i, _)| i)
        .collect();

    if incident_faces.len() < 3 {
        return None;
    }

    // Coleta arestas incidentes direcionadas: no corner de cada face, (prev -> v -> next)
    let mut face_corners = Vec::new();
    let mut neighbors_set = std::collections::HashSet::new();
    for &fi in &incident_faces {
        let face = &mesh.faces[fi];
        let n = face.verts.len();
        let k = face.verts.iter().position(|&vert| vert == v)?;
        let prev = face.verts[(k + n - 1) % n];
        let next = face.verts[(k + 1) % n];
        if prev == v || next == v || prev == next {
            return None;
        }
        neighbors_set.insert(prev);
        neighbors_set.insert(next);
        face_corners.push((fi, k, prev, next));
    }

    // Para vértice manifold fechado, o número de arestas incidentes é igual ao número de faces incidentes
    if neighbors_set.len() != incident_faces.len() {
        return None;
    }

    let mut result = mesh.clone();
    let v_pos = mesh.verts[v_idx].vec();

    // Cria os novos vértices ao longo de cada aresta incidente
    let mut neighbor_new_vert = HashMap::new();
    let mut neighbor_t = HashMap::new();
    for &u in &neighbors_set {
        let u_pos = mesh.verts[u as usize].vec();
        let edge_vec = u_pos - v_pos;
        let edge_len = edge_vec.length();
        if edge_len < 1e-5 {
            return None;
        }
        let t = if clamp_overlap {
            (amount / edge_len).clamp(0.01, 0.49)
        } else {
            (amount / edge_len).clamp(0.01, 0.99)
        };
        let new_pos = v_pos + edge_vec * t;
        let v_vert = &mesh.verts[v_idx];
        let u_vert = &mesh.verts[u as usize];
        let new_color = [
            v_vert.color[0] * (1.0 - t) + u_vert.color[0] * t,
            v_vert.color[1] * (1.0 - t) + u_vert.color[1] * t,
            v_vert.color[2] * (1.0 - t) + u_vert.color[2] * t,
        ];
        let new_v_idx = result.verts.len() as u32;
        let mut vert = Vertex::new(new_pos.x, new_pos.y, new_pos.z);
        vert.color = new_color;
        vert.selected = true;
        result.verts.push(vert);
        neighbor_new_vert.insert(u, new_v_idx);
        neighbor_t.insert(u, t);
    }

    // Atualiza as faces incidentes: substitui o vértice v por (v'_prev, v'_next)
    let mut cut_segments = Vec::new(); // segmentos orientados para a face de tampa: v'_next -> v'_prev
    for (fi, k, prev, next) in face_corners {
        let &new_prev = neighbor_new_vert.get(&prev)?;
        let &new_next = neighbor_new_vert.get(&next)?;
        let &t_prev = neighbor_t.get(&prev)?;
        let &t_next = neighbor_t.get(&next)?;

        let face = &mesh.faces[fi];
        let uv_v = face.uv[k];
        let prev_k = (k + face.verts.len() - 1) % face.verts.len();
        let next_k = (k + 1) % face.verts.len();
        let uv_prev = face.uv[prev_k];
        let uv_next = face.uv[next_k];

        let uv_new_prev = [
            uv_v[0] + (uv_prev[0] - uv_v[0]) * t_prev,
            uv_v[1] + (uv_prev[1] - uv_v[1]) * t_prev,
        ];
        let uv_new_next = [
            uv_v[0] + (uv_next[0] - uv_v[0]) * t_next,
            uv_v[1] + (uv_next[1] - uv_v[1]) * t_next,
        ];

        let mut new_verts = Vec::new();
        let mut new_uvs = Vec::new();
        for (idx, (&vert, &uv)) in face.verts.iter().zip(&face.uv).enumerate() {
            if idx == k {
                new_verts.push(new_prev);
                new_uvs.push(uv_new_prev);
                new_verts.push(new_next);
                new_uvs.push(uv_new_next);
            } else {
                new_verts.push(vert);
                new_uvs.push(uv);
            }
        }
        result.faces[fi] = Face::with_uv(new_verts, new_uvs);
        // O corte na face fi vai de new_prev para new_next.
        // A face de fechamento (cap) deve percorrer o segmento no sentido oposto: new_next -> new_prev.
        cut_segments.push((new_next, new_prev));
    }

    // Encadeia os cut_segments para formar o polígono da face de fechamento (cap face)
    let mut cap_verts = Vec::new();
    let current = cut_segments[0].0;
    cap_verts.push(current);
    let mut target = cut_segments[0].1;

    for _ in 1..cut_segments.len() {
        cap_verts.push(target);
        let next_seg = cut_segments.iter().find(|seg| seg.0 == target)?;
        target = next_seg.1;
    }
    if target != cap_verts[0] {
        return None;
    }

    let mut cap_face = Face::new(cap_verts);
    cap_face.selected = true;
    cap_face.fix_uv();
    result.faces.push(cap_face);

    result.selected_edges.clear();
    result.remove_isolated_vertices();
    result.sync_vert_selection_from_faces();

    let report = result.validate_topology();
    if !report.is_manifold || report.is_closed != input_report.is_closed {
        return None;
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cube_edge_bevel_preserves_closed_planar_surface() {
        let cube = Mesh::cube(2.0);
        for edge in cube.edges_unique() {
            let mut mesh = cube.clone();
            mesh.selected_edges.insert(edge);
            assert_eq!(mesh.bevel_selected(0.2), (1, 0));
            let report = mesh.validate_topology();
            assert!(
                report.is_manifold && report.is_closed,
                "{edge:?}: {report:?}"
            );
            assert_eq!((mesh.verts.len(), mesh.faces.len()), (10, 7));
            for (fi, face) in mesh.faces.iter().enumerate() {
                let normal = mesh.face_normal(fi);
                let origin = mesh.verts[face.verts[0] as usize].vec();
                assert!(
                    face.verts
                        .iter()
                        .all(|&vi| normal.dot(mesh.verts[vi as usize].vec() - origin).abs() < 1e-5)
                );
                assert!(normal.dot(mesh.face_centroid(fi)) > 0.0);
                assert_eq!(face.verts.len(), face.uv.len());
            }
        }
    }

    #[test]
    fn unsupported_multiple_edges_and_nonfinite_width_are_atomic() {
        let mut mesh = Mesh::cube(2.0);
        mesh.selected_edges.extend([(0, 1), (1, 2)]);
        let before = format!("{mesh:?}");
        assert_eq!(mesh.bevel_selected(0.2), (0, 2));
        assert_eq!(format!("{mesh:?}"), before);
        mesh.selected_edges.remove(&(1, 2));
        for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            let before = format!("{mesh:?}");
            assert_eq!(mesh.bevel_selected(value), (0, 1));
            assert_eq!(format!("{mesh:?}"), before);
        }
    }

    #[test]
    fn twisted_surface_bevel_is_rejected_without_partial_geometry() {
        let mut mesh = Mesh::cube(2.0);
        mesh.verts[6].pos[0] += 0.7;
        mesh.selected_edges.insert((0, 1));
        let before = format!("{mesh:?}");
        assert_eq!(mesh.bevel_selected(0.2), (0, 1));
        assert_eq!(format!("{mesh:?}"), before);
    }

    #[test]
    fn corrupt_face_indices_and_uvs_are_rejected_before_geometry_access() {
        for invalid_index in [false, true] {
            let mut mesh = Mesh::cube(2.0);
            mesh.selected_edges.insert((0, 1));
            if invalid_index {
                mesh.faces[0].verts[1] = u32::MAX;
            } else {
                mesh.faces[0].uv.clear();
            }
            let before = format!("{mesh:?}");
            assert_eq!(mesh.bevel_selected(0.2), (0, 1));
            assert_eq!(format!("{mesh:?}"), before);
        }
    }

    #[test]
    fn every_cube_vertex_bevel_preserves_closed_manifold_surface() {
        let cube = Mesh::cube(2.0);
        for vi in 0..cube.verts.len() as u32 {
            let result = bevel_vertex(&cube, vi, 0.2, true);
            assert!(result.is_some(), "bevel_vertex failed on vertex {vi}");
            let mesh = result.unwrap();
            let report = mesh.validate_topology();
            assert!(
                report.is_manifold && report.is_closed,
                "vertex {vi}: {report:?}"
            );
            assert_eq!((mesh.verts.len(), mesh.faces.len()), (10, 7));
            for (fi, face) in mesh.faces.iter().enumerate() {
                assert_eq!(face.verts.len(), face.uv.len());
                let normal = mesh.face_normal(fi);
                assert!(normal.is_finite());
                assert!(normal.length_squared() > 0.5);
            }
        }
    }

    #[test]
    fn sequential_multiple_vertices_bevel() {
        let cube = Mesh::cube(2.0);
        let orig_positions: Vec<[f32; 3]> = cube.verts.iter().map(|v| v.pos).collect();
        let mut mesh = cube;
        for (i, orig_pos) in orig_positions.iter().enumerate() {
            let vi = mesh.verts.iter().position(|v| {
                let d = (v.pos[0] - orig_pos[0])
                    .hypot(v.pos[1] - orig_pos[1])
                    .hypot(v.pos[2] - orig_pos[2]);
                d < 1e-4
            });
            assert!(vi.is_some(), "could not find vertex {i}");
            let next = bevel_vertex(&mesh, vi.unwrap() as u32, 0.2, true);
            assert!(next.is_some(), "bevel_vertex failed on vertex {i}");
            mesh = next.unwrap();
        }
        let report = mesh.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        // Truncated cube has 8 triangles (1 for each beveled corner) + 6 octagons = 14 faces!
        assert_eq!(mesh.faces.len(), 14);
        // 8 corners * 3 vertices per corner = 24 vertices!
        assert_eq!(mesh.verts.len(), 24);
    }

    #[test]
    fn opposite_disjoint_edges_bevel() {
        let cube = Mesh::cube(2.0);
        let pos_5 = cube.verts[5].pos;
        let pos_6 = cube.verts[6].pos;

        let (m1, _) = bevel_edge_segments_clamped(&cube, 0, 3, 0.2, 1, true)
            .expect("bevel edge (0, 3) failed");
        assert!(m1.validate_topology().is_manifold);

        // Find edge with positions matching pos_5 and pos_6
        let v5 = m1
            .verts
            .iter()
            .position(|v| {
                (v.pos[0] - pos_5[0])
                    .hypot(v.pos[1] - pos_5[1])
                    .hypot(v.pos[2] - pos_5[2])
                    < 1e-4
            })
            .unwrap() as u32;
        let v6 = m1
            .verts
            .iter()
            .position(|v| {
                (v.pos[0] - pos_6[0])
                    .hypot(v.pos[1] - pos_6[1])
                    .hypot(v.pos[2] - pos_6[2])
                    < 1e-4
            })
            .unwrap() as u32;

        let (m2, _) = bevel_edge_segments_clamped(&m1, v5, v6, 0.2, 1, true)
            .expect("bevel edge (v5, v6) failed");
        let report = m2.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((m2.verts.len(), m2.faces.len()), (12, 8));
    }

    #[test]
    fn four_parallel_edges_bevel_cylinder_pillar() {
        let cube = Mesh::cube(2.0);
        let vertical_edges = [(0, 3), (1, 2), (4, 7), (5, 6)];
        let target_edge_positions: Vec<([f32; 3], [f32; 3])> = vertical_edges
            .iter()
            .map(|&(a, b)| (cube.verts[a].pos, cube.verts[b].pos))
            .collect();

        let mut mesh = cube;
        for (pos_a, pos_b) in target_edge_positions {
            let va = mesh
                .verts
                .iter()
                .position(|v| {
                    (v.pos[0] - pos_a[0])
                        .hypot(v.pos[1] - pos_a[1])
                        .hypot(v.pos[2] - pos_a[2])
                        < 1e-4
                })
                .unwrap() as u32;
            let vb = mesh
                .verts
                .iter()
                .position(|v| {
                    (v.pos[0] - pos_b[0])
                        .hypot(v.pos[1] - pos_b[1])
                        .hypot(v.pos[2] - pos_b[2])
                        < 1e-4
                })
                .unwrap() as u32;
            let (next, _) = bevel_edge_segments_clamped(&mesh, va, vb, 0.2, 1, true)
                .expect("sequential bevel step failed");
            mesh = next;
        }
        let report = mesh.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((mesh.verts.len(), mesh.faces.len()), (16, 10));
    }

    #[test]
    fn mixed_sets_with_unsupported_edges_change_nothing_and_list_them() {
        let mut mesh = Mesh::cube(2.0);
        let mut sheet = Mesh::plane(1.0);
        for v in &mut sheet.verts {
            v.pos[0] += 10.0;
        }
        mesh.join(&sheet);
        let open_edge = crate::edge_key(8, 9); // borda aberta do plano
        let before = format!("{mesh:?}");
        let report = mesh.bevel_edges(&[(0, 3), open_edge], 0.2, 1, true);
        assert_eq!(report.beveled_edges, 0);
        assert_eq!(report.unsupported_edges, vec![open_edge]);
        assert_eq!(format!("{mesh:?}"), before);
    }

    #[test]
    fn edge_order_does_not_change_the_result() {
        let run = |edges: &[(u32, u32)]| {
            let mut mesh = Mesh::cube(2.0);
            let report = mesh.bevel_edges(edges, 0.2, 2, true);
            (
                report.beveled_edges,
                format!("{:?}", mesh.verts.iter().map(|v| v.pos).collect::<Vec<_>>()),
            )
        };
        assert_eq!(run(&[(0, 3), (4, 5)]), run(&[(5, 4), (3, 0)]));
    }

    #[test]
    fn segments_above_four_are_accepted() {
        let mut mesh = Mesh::cube(2.0);
        let report = mesh.bevel_edges(&[(0, 3)], 0.2, 8, true);
        assert_eq!((report.beveled_edges, report.segments), (1, 8));
    }

    #[test]
    fn clamped_edges_reports_edges_limited_by_overlap_guard() {
        // Offset de 1.0 numa aresta de 2.0 é 0.5 da aresta vizinha: com o clamp
        // ativo (teto 0.45) a largura é reduzida; sem clamp (teto 0.95) não.
        let mut clamped_mesh = Mesh::cube(2.0);
        let clamped = clamped_mesh.bevel_edges(&[(0, 3)], 1.0, 1, true);
        assert_eq!((clamped.beveled_edges, clamped.clamped_edges), (1, 1));

        let mut free_mesh = Mesh::cube(2.0);
        let free = free_mesh.bevel_edges(&[(0, 3)], 1.0, 1, false);
        assert_eq!((free.beveled_edges, free.clamped_edges), (1, 0));

        // Largura abaixo do limite: o relatório não inventa clamp.
        let mut small_mesh = Mesh::cube(2.0);
        let small = small_mesh.bevel_edges(&[(0, 3)], 0.4, 1, true);
        assert_eq!((small.beveled_edges, small.clamped_edges), (1, 0));
    }
}
