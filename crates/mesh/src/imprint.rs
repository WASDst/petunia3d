//! Imprint de região numa face e folha de região solta (ADR 007, Onda 4).
//!
//! Draw on Face: uma região desenhada **dentro** de uma face plana vira uma
//! face própria; o anel ao redor é dividido em duas faces por duas "pontes"
//! (a malha não tem faces com furo). Empurrar a face interna para dentro cria
//! um rebaixo; puxar para fora, um ressalto — a extrusão comum faz o resto.
//!
//! Região solta (sem face hospedeira): uma folha fechada com o fundo invertido
//! e o topo selecionado; extrudar o topo produz um sólido fechado.

use geo::{BooleanOps, Coord, LineString, Polygon, TriangulateEarcut};
use glam::Vec3;

use crate::{Face, Mesh, Vertex};

/// Por que o imprint não pode ser feito.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImprintError {
    #[error("face does not exist or is degenerate")]
    FaceMissing,
    #[error("face or region is not planar")]
    NotPlanar,
    #[error("region must lie strictly inside the face")]
    OutsideFace,
    #[error("region is degenerate")]
    Degenerate,
    #[error("could not split the ring around the region")]
    NoBridge,
}

/// Referencial 2D de um plano.
#[derive(Debug, Clone, Copy)]
struct Frame {
    origin: Vec3,
    u: Vec3,
    v: Vec3,
}

impl Frame {
    fn project(&self, p: Vec3) -> [f64; 2] {
        let d = p - self.origin;
        [f64::from(d.dot(self.u)), f64::from(d.dot(self.v))]
    }

    fn lift(&self, p: [f64; 2]) -> Vec3 {
        self.origin + self.u * p[0] as f32 + self.v * p[1] as f32
    }
}

/// Vértice do anel: do contorno externo (índice no polígono) ou do furo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RingIndex {
    Outer(usize),
    Hole(usize),
}

/// Grava a região `region` (pontos em mundo, qualquer sentido) na face
/// `face`. Devolve a malha nova e o índice da face interna, única selecionada.
/// Se a região coincide com a própria face, só a seleciona.
pub fn imprint_region(
    mesh: &Mesh,
    face: usize,
    region: &[Vec3],
) -> Result<(Mesh, usize), ImprintError> {
    let host = mesh.faces.get(face).ok_or(ImprintError::FaceMissing)?;
    if host.verts.len() < 3 || host.verts.iter().any(|&v| v as usize >= mesh.verts.len()) {
        return Err(ImprintError::FaceMissing);
    }
    let corners: Vec<Vec3> = host
        .verts
        .iter()
        .map(|&v| mesh.verts[v as usize].vec())
        .collect();
    let normal = mesh.face_normal(face).normalize_or_zero();
    let u = (corners[1] - corners[0]).normalize_or_zero();
    if normal == Vec3::ZERO || u == Vec3::ZERO {
        return Err(ImprintError::FaceMissing);
    }
    let frame = Frame {
        origin: corners[0],
        u,
        v: normal.cross(u).normalize_or_zero(),
    };
    let size = corners
        .iter()
        .map(|c| (*c - corners[0]).length())
        .fold(0.0_f32, f32::max)
        .max(1.0e-6);
    let tolerance = size * 1.0e-4;
    let off_plane = |p: &Vec3| (*p - frame.origin).dot(normal).abs() > tolerance;
    if corners.iter().any(off_plane) || region.iter().any(off_plane) {
        return Err(ImprintError::NotPlanar);
    }

    let outer: Vec<[f64; 2]> = corners.iter().map(|c| frame.project(*c)).collect();
    let mut hole: Vec<[f64; 2]> = dedup_ring(region.iter().map(|p| frame.project(*p)).collect());
    if hole.len() < 3 || signed_area(&hole).abs() < f64::from(tolerance * tolerance) {
        return Err(ImprintError::Degenerate);
    }
    if signed_area(&hole) < 0.0 {
        hole.reverse();
    }
    if signed_area(&outer) <= 0.0 {
        return Err(ImprintError::FaceMissing);
    }

    let tol = f64::from(tolerance);
    // Região igual à face: nada a gravar, basta selecionar a face.
    if hole.len() == outer.len()
        && hole.iter().all(|p| {
            outer
                .iter()
                .any(|q| (p[0] - q[0]).hypot(p[1] - q[1]) <= tol)
        })
    {
        let mut result = mesh.clone();
        result.deselect_all();
        result.faces[face].selected = true;
        result.sync_vert_selection_from_faces();
        return Ok((result, face));
    }

    let strictly_inside =
        hole.iter().all(|p| point_in_polygon(&outer, *p)) && !rings_touch(&outer, &hole, tol);
    if strictly_inside && let Some((half_a, half_b)) = split_ring(&outer, &hole, tol) {
        let mut result = mesh.clone();
        result.deselect_all();
        let base = result.verts.len();
        for point in &hole {
            let mut vertex = Vertex::new(0.0, 0.0, 0.0);
            vertex.pos = frame.lift(*point).to_array();
            vertex.color = mesh.verts[host.verts[0] as usize].color;
            result.verts.push(vertex);
        }
        let to_vertex = |index: RingIndex| -> u32 {
            match index {
                RingIndex::Outer(i) => host.verts[i],
                RingIndex::Hole(j) => (base + j) as u32,
            }
        };
        let to_uv = |index: RingIndex| -> [f32; 2] {
            let p = match index {
                RingIndex::Outer(i) => outer[i],
                RingIndex::Hole(j) => hole[j],
            };
            [p[0] as f32, p[1] as f32]
        };
        let make_face = |ring: &[RingIndex]| -> Face {
            let mut face = Face::with_uv(
                ring.iter().map(|&i| to_vertex(i)).collect(),
                ring.iter().map(|&i| to_uv(i)).collect(),
            );
            face.material_slot = host.material_slot;
            face
        };
        result.faces[face] = make_face(&half_a);
        result.push_face(make_face(&half_b));
        let inner: Vec<RingIndex> = (0..hole.len()).map(RingIndex::Hole).collect();
        let mut inner_face = make_face(&inner);
        inner_face.selected = true;
        let inner_index = result.faces.len();
        result.push_face(inner_face);
        result.sync_vert_selection_from_faces();
        return Ok((result, inner_index));
    }

    // Região que cruza arestas da face hospedeira ou toca bordas: recorte booleano
    imprint_boolean_cut(mesh, face, &outer, &hole, frame, tol)
}

/// Imprint geral: a região (contorno e furos em mundo, num plano) é gravada
/// em **todas** as faces coplanares que ela toca — inclusive cruzando arestas
/// entre faces, ocupando várias faces coplanares ou com furos.
///
/// Cada face tocada é substituída pelas peças `face ∩ região` (selecionadas)
/// e `face \ região` (não selecionadas); polígonos com furo são triangulados.
/// Vértices são soldados por posição e os que caem sobre arestas de faces
/// vizinhas são inseridos nelas, para a malha não ganhar junções em T.
/// Devolve a malha nova e os índices das faces internas.
pub fn imprint_region_general(
    mesh: &Mesh,
    outer: &[Vec3],
    holes: &[Vec<Vec3>],
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Result<(Mesh, Vec<usize>), ImprintError> {
    let normal = plane_normal.normalize_or_zero();
    if normal == Vec3::ZERO || outer.len() < 3 {
        return Err(ImprintError::Degenerate);
    }
    let seed = outer
        .iter()
        .map(|p| *p - plane_point)
        .find(|d| d.length_squared() > 1.0e-12)
        .unwrap_or(Vec3::X);
    let u = (seed - normal * seed.dot(normal)).normalize_or_zero();
    let u = if u == Vec3::ZERO {
        normal.any_orthonormal_vector()
    } else {
        u
    };
    let frame = Frame {
        origin: plane_point,
        u,
        v: normal.cross(u),
    };
    let project_ring = |ring: &[Vec3]| -> Vec<[f64; 2]> {
        dedup_ring(ring.iter().map(|p| frame.project(*p)).collect())
    };
    let outer_2d = project_ring(outer);
    if outer_2d.len() < 3 || signed_area(&outer_2d).abs() < 1.0e-12 {
        return Err(ImprintError::Degenerate);
    }
    let region = Polygon::new(
        geo_ring(&outer_2d),
        holes.iter().map(|h| geo_ring(&project_ring(h))).collect(),
    );
    let size = outer_2d
        .iter()
        .map(|p| p[0].abs().max(p[1].abs()))
        .fold(1.0e-6, f64::max);
    let plane_tolerance = (size as f32 * 1.0e-5).max(1.0e-5);

    // Faces coplanares com a região.
    let coplanar: Vec<usize> = (0..mesh.faces.len())
        .filter(|&fi| {
            let face = &mesh.faces[fi];
            if face.verts.len() < 3 {
                return false;
            }
            let n = mesh.face_normal(fi).normalize_or_zero();
            n.dot(normal).abs() > 0.9999
                && face.verts.iter().all(|&v| {
                    (mesh.verts[v as usize].vec() - plane_point)
                        .dot(normal)
                        .abs()
                        <= plane_tolerance
                })
        })
        .collect();

    let mut result = mesh.clone();
    let original_vertex_count = result.verts.len();
    let mut replaced: Vec<usize> = Vec::new();
    let mut new_faces: Vec<Face> = Vec::new();
    let weld = (size * 1.0e-7).max(1.0e-9);
    let find_or_add = |result: &mut Mesh, p: Vec3, color: [f32; 3]| -> u32 {
        if let Some(i) = result
            .verts
            .iter()
            .position(|v| f64::from(v.vec().distance_squared(p)) <= weld * weld)
        {
            return i as u32;
        }
        result.verts.push(Vertex {
            pos: p.to_array(),
            color,
            selected: false,
        });
        (result.verts.len() - 1) as u32
    };
    for &fi in &coplanar {
        let face = mesh.faces[fi].clone();
        let face_2d: Vec<[f64; 2]> = face
            .verts
            .iter()
            .map(|&v| frame.project(mesh.verts[v as usize].vec()))
            .collect();
        let face_poly = Polygon::new(geo_ring(&face_2d), vec![]);
        let inside = face_poly.intersection(&region);
        let inside_area: f64 = inside.0.iter().map(|p| polygon_area(p)).sum();
        if inside_area <= 1.0e-12 * size * size {
            continue;
        }
        let outside = face_poly.difference(&region);
        let face_normal = mesh.face_normal(fi).normalize_or_zero();
        let flip = face_normal.dot(normal) < 0.0;
        let points_3d: Vec<Vec3> = face
            .verts
            .iter()
            .map(|&v| mesh.verts[v as usize].vec())
            .collect();
        let uv_at = crate::ops::planar_uv_map(&points_3d, &face.uv, face_normal);
        let color = mesh.verts[face.verts[0] as usize].color;
        replaced.push(fi);
        for (pieces, selected) in [(&inside, true), (&outside, false)] {
            for polygon in &pieces.0 {
                if polygon_area(polygon) <= 1.0e-12 * size * size {
                    continue;
                }
                // Polígonos sem furo viram uma face; com furo, triângulos.
                let loops: Vec<Vec<[f64; 2]>> = if polygon.interiors().is_empty() {
                    let mut ring = dedup_ring(open_ring_points(polygon.exterior()));
                    if signed_area(&ring) < 0.0 {
                        ring.reverse();
                    }
                    vec![ring]
                } else {
                    let raw = polygon.earcut_triangles_raw();
                    raw.triangle_indices
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .map(|t| {
                            let mut tri: Vec<[f64; 2]> =
                                t.iter().map(|i| raw.vertices[*i]).collect();
                            if signed_area(&tri) < 0.0 {
                                tri.reverse();
                            }
                            tri
                        })
                        .collect()
                };
                for mut ring in loops {
                    if ring.len() < 3 {
                        continue;
                    }
                    if flip {
                        ring.reverse();
                    }
                    let mut verts = Vec::with_capacity(ring.len());
                    let mut uv = Vec::with_capacity(ring.len());
                    for p in &ring {
                        let world = frame.lift(*p);
                        let index = find_or_add(&mut result, world, color);
                        if verts.last() != Some(&index) {
                            verts.push(index);
                            uv.push(uv_at(world));
                        }
                    }
                    if verts.len() >= 3 && verts.first() == verts.last() {
                        verts.pop();
                        uv.pop();
                    }
                    if verts.len() < 3 {
                        continue;
                    }
                    let mut new_face = Face::with_uv(verts, uv);
                    new_face.selected = selected;
                    new_face.material_slot = face.material_slot;
                    new_faces.push(new_face);
                }
            }
        }
    }
    if replaced.is_empty() {
        return Err(ImprintError::OutsideFace);
    }

    // Faces mantidas: insere nas arestas os vértices novos que caem sobre elas.
    let added: Vec<u32> = (original_vertex_count as u32..result.verts.len() as u32).collect();
    replaced.sort_unstable();
    let mut kept: Vec<Face> = result
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| replaced.binary_search(i).is_err())
        .map(|(_, f)| {
            let mut f = f.clone();
            f.selected = false;
            f
        })
        .collect();
    let verts_snapshot: Vec<Vec3> = result.verts.iter().map(|v| v.vec()).collect();
    let split_edges = |face: &mut Face| {
        let n = face.verts.len();
        let mut verts = Vec::with_capacity(n);
        let mut uv = Vec::with_capacity(n);
        for k in 0..n {
            let (a, b) = (face.verts[k], face.verts[(k + 1) % n]);
            let (pa, pb) = (verts_snapshot[a as usize], verts_snapshot[b as usize]);
            verts.push(a);
            uv.push(face.uv[k]);
            let edge = pb - pa;
            let length_sq = edge.length_squared();
            if length_sq < 1.0e-18 {
                continue;
            }
            let mut on_edge: Vec<(f32, u32)> = added
                .iter()
                .filter(|&&i| i != a && i != b)
                .filter_map(|&i| {
                    let p = verts_snapshot[i as usize];
                    let t = (p - pa).dot(edge) / length_sq;
                    let off = (pa + edge * t).distance_squared(p);
                    (t > 1.0e-6 && t < 1.0 - 1.0e-6 && f64::from(off) <= weld * weld * 100.0)
                        .then_some((t, i))
                })
                .collect();
            on_edge.sort_by(|x, y| x.0.total_cmp(&y.0));
            let (ua, ub) = (face.uv[k], face.uv[(k + 1) % n]);
            for (t, i) in on_edge {
                verts.push(i);
                uv.push([ua[0] + (ub[0] - ua[0]) * t, ua[1] + (ub[1] - ua[1]) * t]);
            }
        }
        face.verts = verts;
        face.uv = uv;
    };
    for face in kept.iter_mut().chain(new_faces.iter_mut()) {
        split_edges(face);
    }
    let first_new = kept.len();
    result.faces = kept;
    result.faces.extend(new_faces);
    let inside: Vec<usize> = (first_new..result.faces.len())
        .filter(|&i| result.faces[i].selected)
        .collect();
    result.selected_edges.clear();
    result.sync_vert_selection_from_faces();
    Ok((result, inside))
}

fn polygon_area(polygon: &Polygon<f64>) -> f64 {
    let ring = |line: &LineString<f64>| signed_area(&open_ring_points(line)).abs();
    ring(polygon.exterior()) - polygon.interiors().iter().map(ring).sum::<f64>()
}

fn geo_ring(points: &[[f64; 2]]) -> LineString<f64> {
    let mut coords: Vec<Coord<f64>> = points.iter().map(|p| Coord { x: p[0], y: p[1] }).collect();
    if coords.first() != coords.last()
        && let Some(first) = coords.first().copied()
    {
        coords.push(first);
    }
    LineString::new(coords)
}

fn open_ring_points(line: &LineString<f64>) -> Vec<[f64; 2]> {
    let mut points: Vec<[f64; 2]> = line.coords().map(|c| [c.x, c.y]).collect();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1.0e-12);
    if points.len() > 1 {
        let (first, last) = (points[0], points[points.len() - 1]);
        if (first[0] - last[0]).hypot(first[1] - last[1]) < 1.0e-12 {
            points.pop();
        }
    }
    points
}

fn imprint_boolean_cut(
    mesh: &Mesh,
    face: usize,
    outer: &[[f64; 2]],
    hole: &[[f64; 2]],
    frame: Frame,
    tol: f64,
) -> Result<(Mesh, usize), ImprintError> {
    let host = &mesh.faces[face];
    let outer_poly = Polygon::new(geo_ring(outer), vec![]);
    let hole_poly = Polygon::new(geo_ring(hole), vec![]);
    let intersection = outer_poly.intersection(&hole_poly);
    let difference = outer_poly.difference(&hole_poly);

    let min_area = tol * tol;
    let mut inter_rings = Vec::new();
    for poly in &intersection.0 {
        let mut pts = open_ring_points(poly.exterior());
        if pts.len() >= 3 && signed_area(&pts).abs() >= min_area {
            if signed_area(&pts) < 0.0 {
                pts.reverse();
            }
            inter_rings.push(pts);
        }
    }
    if inter_rings.is_empty() {
        return Err(ImprintError::OutsideFace);
    }

    let mut diff_rings = Vec::new();
    for poly in &difference.0 {
        let mut pts = open_ring_points(poly.exterior());
        if pts.len() >= 3 && signed_area(&pts).abs() >= min_area {
            if signed_area(&pts) < 0.0 {
                pts.reverse();
            }
            diff_rings.push(pts);
        }
    }

    let mut result = mesh.clone();
    result.deselect_all();
    let host_color = mesh.verts[host.verts[0] as usize].color;

    let mut resolve_point = |p: [f64; 2]| -> u32 {
        for (i, &v_idx) in host.verts.iter().enumerate() {
            let op = outer[i];
            if (op[0] - p[0]).hypot(op[1] - p[1]) <= tol {
                return v_idx;
            }
        }
        for (idx, v) in result.verts.iter().enumerate() {
            let wp = frame.lift(p).to_array();
            if (v.pos[0] - wp[0])
                .hypot(v.pos[1] - wp[1])
                .hypot(v.pos[2] - wp[2]) as f64
                <= tol
            {
                return idx as u32;
            }
        }
        let new_idx = result.verts.len() as u32;
        let mut nv = Vertex::new(0.0, 0.0, 0.0);
        nv.pos = frame.lift(p).to_array();
        nv.color = host_color;
        result.verts.push(nv);
        new_idx
    };

    let mut new_faces: Vec<Face> = Vec::new();
    // Faces externas da diferença
    for ring in diff_rings {
        let indices: Vec<u32> = ring.iter().map(|&p| resolve_point(p)).collect();
        let uvs: Vec<[f32; 2]> = ring.iter().map(|&p| [p[0] as f32, p[1] as f32]).collect();
        let mut f = Face::with_uv(indices, uvs);
        f.material_slot = host.material_slot;
        new_faces.push(f);
    }

    let mut inner_indices = Vec::new();
    for ring in inter_rings {
        let indices: Vec<u32> = ring.iter().map(|&p| resolve_point(p)).collect();
        let uvs: Vec<[f32; 2]> = ring.iter().map(|&p| [p[0] as f32, p[1] as f32]).collect();
        let mut f = Face::with_uv(indices, uvs);
        f.selected = true;
        f.material_slot = host.material_slot;
        inner_indices.push(f);
    }

    if !new_faces.is_empty() {
        let first_diff = new_faces.remove(0);
        result.faces[face] = first_diff;
        for f in new_faces {
            result.push_face(f);
        }
        let ret_inner = result.faces.len();
        for f in inner_indices {
            result.push_face(f);
        }
        result.sync_vert_selection_from_faces();
        Ok((result, ret_inner))
    } else if !inner_indices.is_empty() {
        let first_inter = inner_indices.remove(0);
        result.faces[face] = first_inter;
        for f in inner_indices {
            result.push_face(f);
        }
        result.sync_vert_selection_from_faces();
        Ok((result, face))
    } else {
        Err(ImprintError::Degenerate)
    }
}

/// Folha fechada de uma região solta num plano: fundo com a normal invertida
/// e topo (selecionado) com a normal do plano. `outer` e `holes` em 2D no
/// referencial `origin/right/up`. Aceita no máximo um furo.
pub fn region_sheet(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    origin: Vec3,
    right: Vec3,
    up: Vec3,
) -> Result<Mesh, ImprintError> {
    let normal = right.cross(up).normalize_or_zero();
    if normal == Vec3::ZERO {
        return Err(ImprintError::Degenerate);
    }
    let frame = Frame {
        origin,
        u: right.normalize_or_zero(),
        v: up.normalize_or_zero(),
    };
    let mut outer = dedup_ring(outer.to_vec());
    if outer.len() < 3 || signed_area(&outer).abs() < 1.0e-12 {
        return Err(ImprintError::Degenerate);
    }
    if signed_area(&outer) < 0.0 {
        outer.reverse();
    }
    let size = outer
        .iter()
        .map(|p| p[0].abs().max(p[1].abs()))
        .fold(1.0e-6, f64::max);
    let tol = size * 1.0e-6;

    let (rings, points): (Vec<Vec<RingIndex>>, Vec<[f64; 2]>) = match holes {
        [] => (
            vec![(0..outer.len()).map(RingIndex::Outer).collect()],
            outer.clone(),
        ),
        [hole] => {
            let mut hole = dedup_ring(hole.clone());
            if hole.len() < 3 {
                return Err(ImprintError::Degenerate);
            }
            if signed_area(&hole) < 0.0 {
                hole.reverse();
            }
            let (a, b) = split_ring(&outer, &hole, tol).ok_or(ImprintError::NoBridge)?;
            let mut points = outer.clone();
            points.extend(hole.iter().copied());
            (vec![a, b], points)
        }
        _ => {
            let ring = |points: &[[f64; 2]]| {
                LineString::new(points.iter().map(|p| Coord { x: p[0], y: p[1] }).collect())
            };
            if holes
                .iter()
                .any(|h| h.len() < 3 || h.iter().flatten().any(|v| !v.is_finite()))
            {
                return Err(ImprintError::Degenerate);
            }
            let polygon = Polygon::new(ring(&outer), holes.iter().map(|h| ring(h)).collect());
            let triangulation = polygon.earcut_triangles_raw();
            let mut mesh = Mesh::default();
            for p in &triangulation.vertices {
                let mut vertex = Vertex::new(0.0, 0.0, 0.0);
                vertex.pos = frame.lift(*p).to_array();
                mesh.verts.push(vertex);
            }
            for indices in triangulation.triangle_indices.as_chunks::<3>().0 {
                let mut verts: Vec<_> = indices.iter().map(|i| *i as u32).collect();
                let points: Vec<_> = indices.iter().map(|i| triangulation.vertices[*i]).collect();
                if signed_area(&points) < 0.0 {
                    verts.reverse();
                }
                let uv: Vec<_> = verts
                    .iter()
                    .map(|i| triangulation.vertices[*i as usize].map(|v| v as f32))
                    .collect();
                let mut bottom = Face::with_uv(
                    verts.iter().rev().copied().collect(),
                    uv.iter().rev().copied().collect(),
                );
                bottom.selected = false;
                mesh.push_face(bottom);
                let mut top = Face::with_uv(verts, uv);
                top.selected = true;
                mesh.push_face(top);
            }
            if mesh.faces.is_empty() {
                return Err(ImprintError::Degenerate);
            }
            mesh.sync_vert_selection_from_faces();
            return Ok(mesh);
        }
    };
    let outer_len = outer.len();
    let index_of = |i: RingIndex| -> u32 {
        match i {
            RingIndex::Outer(i) => i as u32,
            RingIndex::Hole(j) => (outer_len + j) as u32,
        }
    };

    let mut mesh = Mesh::default();
    for p in &points {
        let mut vertex = Vertex::new(0.0, 0.0, 0.0);
        vertex.pos = frame.lift(*p).to_array();
        mesh.verts.push(vertex);
    }
    for ring in &rings {
        let verts: Vec<u32> = ring.iter().map(|&i| index_of(i)).collect();
        let uv: Vec<[f32; 2]> = verts
            .iter()
            .map(|&v| {
                let p = points[v as usize];
                [p[0] as f32, p[1] as f32]
            })
            .collect();
        let mut bottom = Face::with_uv(
            verts.iter().rev().copied().collect(),
            uv.iter().rev().copied().collect(),
        );
        bottom.selected = false;
        mesh.push_face(bottom);
        let mut top = Face::with_uv(verts, uv);
        top.selected = true;
        mesh.push_face(top);
    }
    mesh.sync_vert_selection_from_faces();
    Ok(mesh)
}

fn dedup_ring(mut ring: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    ring.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1.0e-9);
    while ring.len() > 1 {
        let (first, last) = (ring[0], ring[ring.len() - 1]);
        if (first[0] - last[0]).hypot(first[1] - last[1]) < 1.0e-9 {
            ring.pop();
        } else {
            break;
        }
    }
    ring
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    crate::arrangement::signed_area(points)
}

fn point_in_polygon(polygon: &[[f64; 2]], point: [f64; 2]) -> bool {
    crate::arrangement::point_in_polygon(polygon, point)
}

fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Segmentos se tocam (inclusive nas pontas ou colineares sobrepostos).
fn segments_touch(p1: [f64; 2], p2: [f64; 2], q1: [f64; 2], q2: [f64; 2], tol: f64) -> bool {
    let d1 = orient(q1, q2, p1);
    let d2 = orient(q1, q2, p2);
    let d3 = orient(p1, p2, q1);
    let d4 = orient(p1, p2, q2);
    let scale_q = (q2[0] - q1[0]).hypot(q2[1] - q1[1]).max(1.0e-12);
    let scale_p = (p2[0] - p1[0]).hypot(p2[1] - p1[1]).max(1.0e-12);
    let (t1, t2) = (tol * scale_q, tol * scale_p);
    if ((d1 > t1 && d2 < -t1) || (d1 < -t1 && d2 > t1))
        && ((d3 > t2 && d4 < -t2) || (d3 < -t2 && d4 > t2))
    {
        return true;
    }
    let on = |a: [f64; 2], b: [f64; 2], p: [f64; 2], d: f64, t: f64| {
        d.abs() <= t
            && p[0] >= a[0].min(b[0]) - tol
            && p[0] <= a[0].max(b[0]) + tol
            && p[1] >= a[1].min(b[1]) - tol
            && p[1] <= a[1].max(b[1]) + tol
    };
    on(q1, q2, p1, d1, t1)
        || on(q1, q2, p2, d2, t1)
        || on(p1, p2, q1, d3, t2)
        || on(p1, p2, q2, d4, t2)
}

fn rings_touch(outer: &[[f64; 2]], hole: &[[f64; 2]], tol: f64) -> bool {
    let (n, m) = (outer.len(), hole.len());
    (0..n).any(|i| {
        (0..m).any(|j| {
            segments_touch(
                outer[i],
                outer[(i + 1) % n],
                hole[j],
                hole[(j + 1) % m],
                tol,
            )
        })
    })
}

/// Divide o anel entre `outer` (anti-horário) e `hole` (anti-horário) em dois
/// polígonos simples anti-horários ligados por duas pontes.
fn split_ring(
    outer: &[[f64; 2]],
    hole: &[[f64; 2]],
    tol: f64,
) -> Option<(Vec<RingIndex>, Vec<RingIndex>)> {
    let (n, m) = (outer.len(), hole.len());
    let point = |i: RingIndex| match i {
        RingIndex::Outer(i) => outer[i],
        RingIndex::Hole(j) => hole[j],
    };
    // Uma ponte é válida se não toca nenhuma aresta que não a encoste.
    let visible = |o: usize, h: usize| -> bool {
        let (a, b) = (outer[o], hole[h]);
        let outer_ok = (0..n).all(|i| {
            let k = (i + 1) % n;
            i == o || k == o || !segments_touch(a, b, outer[i], outer[k], tol)
        });
        let hole_ok = (0..m).all(|j| {
            let k = (j + 1) % m;
            j == h || k == h || !segments_touch(a, b, hole[j], hole[k], tol)
        });
        outer_ok && hole_ok
    };
    let nearest_outer = |h: usize, exclude: Option<usize>| -> Vec<usize> {
        let mut order: Vec<usize> = (0..n).filter(|&o| Some(o) != exclude).collect();
        order.sort_by(|&x, &y| {
            let dx = (outer[x][0] - hole[h][0]).hypot(outer[x][1] - hole[h][1]);
            let dy = (outer[y][0] - hole[h][0]).hypot(outer[y][1] - hole[h][1]);
            dx.total_cmp(&dy)
        });
        order
    };
    for a in 0..m {
        let b = (a + m / 2) % m;
        if a == b {
            continue;
        }
        for &o0 in nearest_outer(a, None).iter().filter(|&&o| visible(o, a)) {
            for &o1 in nearest_outer(b, Some(o0))
                .iter()
                .filter(|&&o| visible(o, b))
            {
                let half_a = ring_path(o0, o1, n, b, a, m);
                let half_b = ring_path(o1, o0, n, a, b, m);
                let valid = |ring: &[RingIndex]| {
                    let points: Vec<[f64; 2]> = ring.iter().map(|&i| point(i)).collect();
                    signed_area(&points) > 0.0 && is_simple(&points, tol)
                };
                if valid(&half_a) && valid(&half_b) {
                    return Some((half_a, half_b));
                }
            }
        }
    }
    None
}

/// Contorno de `from` a `to` no sentido anti-horário do externo, ponte para o
/// furo em `hole_from` e volta pelo furo no sentido horário até `hole_to`.
fn ring_path(
    from: usize,
    to: usize,
    n: usize,
    hole_from: usize,
    hole_to: usize,
    m: usize,
) -> Vec<RingIndex> {
    let mut ring = Vec::new();
    let mut i = from;
    loop {
        ring.push(RingIndex::Outer(i));
        if i == to {
            break;
        }
        i = (i + 1) % n;
    }
    let mut j = hole_from;
    loop {
        ring.push(RingIndex::Hole(j));
        if j == hole_to {
            break;
        }
        j = (j + m - 1) % m;
    }
    ring
}

fn is_simple(points: &[[f64; 2]], tol: f64) -> bool {
    let k = points.len();
    if k < 3 {
        return false;
    }
    for i in 0..k {
        for j in (i + 1)..k {
            // Arestas vizinhas compartilham um vértice: não conta.
            if j == i + 1 || (i == 0 && j == k - 1) {
                continue;
            }
            if segments_touch(
                points[i],
                points[(i + 1) % k],
                points[j],
                points[(j + 1) % k],
                tol,
            ) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quad 4×4 no plano XY, normal +Z.
    fn quad() -> Mesh {
        let mut mesh = Mesh::default();
        for [x, y] in [[0.0f32, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]] {
            mesh.verts.push(Vertex::new(x, y, 0.0));
        }
        mesh.faces.push(Face::new(vec![0, 1, 2, 3]));
        mesh
    }

    fn square(x: f32, y: f32, s: f32) -> Vec<Vec3> {
        vec![
            Vec3::new(x, y, 0.0),
            Vec3::new(x + s, y, 0.0),
            Vec3::new(x + s, y + s, 0.0),
            Vec3::new(x, y + s, 0.0),
        ]
    }

    fn area_of(mesh: &Mesh, face: usize) -> f32 {
        let f = &mesh.faces[face];
        let n = f.verts.len();
        let mut sum = Vec3::ZERO;
        for i in 0..n {
            let a = mesh.verts[f.verts[i] as usize].vec();
            let b = mesh.verts[f.verts[(i + 1) % n] as usize].vec();
            sum += a.cross(b);
        }
        sum.z * 0.5
    }

    #[test]
    fn imprint_splits_the_face_into_ring_and_inner_region() {
        let (mesh, inner) = imprint_region(&quad(), 0, &square(1.0, 1.0, 2.0)).unwrap();
        assert_eq!(mesh.faces.len(), 3);
        assert_eq!(mesh.verts.len(), 8);
        assert!(mesh.faces[inner].selected);
        assert_eq!(mesh.faces.iter().filter(|f| f.selected).count(), 1);
        // Todas as faces mantêm a normal +Z (área positiva) e somam a original.
        let areas: Vec<f32> = (0..3).map(|f| area_of(&mesh, f)).collect();
        assert!(areas.iter().all(|a| *a > 0.0), "{areas:?}");
        assert!((areas.iter().sum::<f32>() - 16.0).abs() < 1e-4);
        assert!((area_of(&mesh, inner) - 4.0).abs() < 1e-4);
    }

    #[test]
    fn imprint_then_extrude_makes_a_closed_boss_and_recess() {
        for distance in [1.0, -1.0] {
            let (mut mesh, _) = imprint_region(&quad(), 0, &square(1.0, 1.0, 2.0)).unwrap();
            mesh.extrude_selected(distance);
            // 3 faces + 4 paredes; cada aresta interna usada por 2 faces.
            assert_eq!(mesh.faces.len(), 7, "distância {distance}");
            let mut uses = std::collections::HashMap::new();
            for f in &mesh.faces {
                for k in 0..f.verts.len() {
                    let (a, b) = (f.verts[k], f.verts[(k + 1) % f.verts.len()]);
                    *uses.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                }
            }
            assert!(uses.values().all(|&c| c <= 2));
        }
    }

    #[test]
    fn imprint_accepts_any_winding_and_rejects_outside_regions() {
        let mut reversed = square(1.0, 1.0, 1.0);
        reversed.reverse();
        assert!(imprint_region(&quad(), 0, &reversed).is_ok());
        // Região totalmente fora da face [0..4]:
        assert_eq!(
            imprint_region(&quad(), 0, &square(5.0, 5.0, 2.0)).unwrap_err(),
            ImprintError::OutsideFace
        );
        let tilted: Vec<Vec3> = square(1.0, 1.0, 1.0)
            .into_iter()
            .map(|p| p + Vec3::Z * p.x)
            .collect();
        assert_eq!(
            imprint_region(&quad(), 0, &tilted).unwrap_err(),
            ImprintError::NotPlanar
        );
        assert_eq!(
            imprint_region(&quad(), 9, &square(1.0, 1.0, 1.0)).unwrap_err(),
            ImprintError::FaceMissing
        );
    }

    #[test]
    fn region_equal_to_the_face_only_selects_it() {
        let (mesh, face) = imprint_region(&quad(), 0, &square(0.0, 0.0, 4.0)).unwrap();
        assert_eq!(face, 0);
        assert_eq!(mesh.faces.len(), 1);
        assert!(mesh.faces[0].selected);
    }

    #[test]
    fn imprint_works_on_a_triangle_host() {
        let mut mesh = Mesh::default();
        for [x, y] in [[0.0f32, 0.0], [6.0, 0.0], [0.0, 6.0]] {
            mesh.verts.push(Vertex::new(x, y, 0.0));
        }
        mesh.faces.push(Face::new(vec![0, 1, 2]));
        let (mesh, inner) = imprint_region(&mesh, 0, &square(1.0, 1.0, 1.0)).unwrap();
        assert!((area_of(&mesh, inner) - 1.0).abs() < 1e-4);
        let total: f32 = (0..mesh.faces.len()).map(|f| area_of(&mesh, f)).sum();
        assert!((total - 18.0).abs() < 1e-3);
    }

    #[test]
    fn free_sheet_extrudes_into_a_closed_solid() {
        let outer = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let mut mesh = region_sheet(&outer, &[], Vec3::ZERO, Vec3::X, Vec3::Y).unwrap();
        assert_eq!(mesh.faces.len(), 2);
        assert!(mesh.faces[1].selected && !mesh.faces[0].selected);
        mesh.extrude_selected(1.0);
        assert_eq!(mesh.faces.len(), 6, "caixa fechada");
        assert!(mesh.verts.iter().any(|v| (v.pos[2] - 1.0).abs() < 1e-6));

        let hole = vec![[0.5, 0.5], [1.5, 0.5], [1.5, 1.5], [0.5, 1.5]];
        let mut ring = region_sheet(&outer, &[hole], Vec3::ZERO, Vec3::X, Vec3::Y).unwrap();
        assert_eq!(ring.faces.len(), 4, "duas metades em cima e embaixo");
        ring.extrude_selected(1.0);
        assert_eq!(ring.faces.len(), 4 + 8, "paredes externas e internas");
    }

    #[test]
    fn imprint_region_crossing_edges_cuts_the_face() {
        // Quad de [0,0] a [4,4] (área 16)
        // Região de [2,2] a [5,5] (cruza a borda em x=4 e y=4)
        // A interseção deve ser de [2,2] a [4,4] (área 4)
        let crossing_region = square(2.0, 2.0, 3.0);
        let (mesh, inner) = imprint_region(&quad(), 0, &crossing_region).unwrap();
        assert!(mesh.faces.len() >= 2);
        assert!(mesh.faces[inner].selected);
        assert!((area_of(&mesh, inner) - 4.0).abs() < 1e-3);
    }
}
