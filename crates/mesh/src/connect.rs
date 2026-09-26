//! Boundary bridging. Unequal vertex counts are joined with quads and triangles;
//! all validation happens on a candidate mesh before publishing a mutation.
use std::collections::{BTreeMap, BTreeSet};

use crate::{Face, Mesh, edge_key};

impl Mesh {
    /// Remove two caps and bridge their boundaries, including unequal counts.
    pub fn connect_loops(&mut self, face_a: usize, face_b: usize) -> Result<(), String> {
        self.connect_loops_with_options(face_a, face_b, 1, 0)
    }

    /// Remove two caps and bridge their boundaries with specified segments and twist.
    pub fn connect_loops_with_options(
        &mut self,
        face_a: usize,
        face_b: usize,
        segments: usize,
        twist: i32,
    ) -> Result<(), String> {
        if face_a == face_b {
            return Err("Select two different faces".into());
        }
        let a = self
            .faces
            .get(face_a)
            .ok_or("Invalid first face")?
            .verts
            .clone();
        let mut b = self
            .faces
            .get(face_b)
            .ok_or("Invalid second face")?
            .verts
            .clone();
        if a.len() < 3 || b.len() < 3 {
            return Err("Faces require at least three points".into());
        }
        // Strip winding matches each removed cap at its boundary.
        b.reverse();
        let material = self.faces[face_a].material_slot;
        let mut candidate = self.clone();
        candidate.faces.remove(face_a.max(face_b));
        candidate.faces.remove(face_a.min(face_b));
        candidate.bridge_boundaries(&a, &b, true, material, segments, twist)?;
        *self = candidate;
        Ok(())
    }

    /// Connect exactly two selected boundary chains. Open chains remain open at
    /// their ends; closed rings wrap once. Branches and non-boundaries are errors.
    pub fn connect_selected_edges(&mut self) -> Result<(), String> {
        self.connect_selected_edges_with_options(1, 0)
    }

    /// Connect exactly two selected boundary chains with specified segments and twist.
    pub fn connect_selected_edges_with_options(
        &mut self,
        segments: usize,
        twist: i32,
    ) -> Result<(), String> {
        let mut adjacency = BTreeMap::<u32, Vec<u32>>::new();
        for &(a, b) in &self.selected_edges {
            if a == b || a as usize >= self.verts.len() || b as usize >= self.verts.len() {
                return Err("Invalid boundary edge".into());
            }
            let incident = self
                .faces
                .iter()
                .filter(|face| {
                    (0..face.verts.len()).any(|i| {
                        edge_key(face.verts[i], face.verts[(i + 1) % face.verts.len()])
                            == edge_key(a, b)
                    })
                })
                .count();
            if incident != 1 {
                return Err("Connect requires boundary edges with one adjacent face".into());
            }
            adjacency.entry(a).or_default().push(b);
            adjacency.entry(b).or_default().push(a);
        }
        if adjacency.values().any(|neighbors| neighbors.len() > 2) {
            return Err("Boundary selection branches; select two simple loops".into());
        }
        let mut remaining: BTreeSet<_> = adjacency.keys().copied().collect();
        let mut chains = Vec::new();
        while let Some(&seed) = remaining.first() {
            let mut component = BTreeSet::new();
            let mut stack = vec![seed];
            while let Some(v) = stack.pop() {
                if !component.insert(v) {
                    continue;
                }
                stack.extend(adjacency[&v].iter().copied());
            }
            let start = component
                .iter()
                .find(|v| adjacency[v].len() == 1)
                .copied()
                .unwrap_or(seed);
            let closed = adjacency[&start].len() == 2;
            let mut chain = Vec::new();
            let (mut previous, mut current) = (None, start);
            loop {
                chain.push(current);
                remaining.remove(&current);
                let next = adjacency[&current]
                    .iter()
                    .copied()
                    .filter(|v| Some(*v) != previous)
                    .min();
                let Some(next) = next else {
                    break;
                };
                if next == start {
                    break;
                }
                if chain.contains(&next) {
                    return Err("Boundary crosses itself".into());
                }
                previous = Some(current);
                current = next;
            }
            if chain.len() < 2 {
                return Err("Select at least one edge per boundary".into());
            }
            chains.push((chain, closed));
        }
        if chains.len() != 2 || chains[0].1 != chains[1].1 {
            return Err("Select two open chains or two closed loops".into());
        }
        let (mut a, closed) = chains.remove(0);
        let (mut b, _) = chains.remove(0);
        let boundary_direction = |chain: &[u32]| {
            self.faces.iter().any(|face| {
                (0..face.verts.len()).any(|i| {
                    face.verts[i] == chain[0] && face.verts[(i + 1) % face.verts.len()] == chain[1]
                })
            })
        };
        if boundary_direction(&a) {
            a.reverse();
        }
        if !boundary_direction(&b) {
            b.reverse();
        }
        let mut candidate = self.clone();
        candidate.bridge_boundaries(&a, &b, closed, None, segments, twist)?;
        *self = candidate;
        Ok(())
    }

    fn bridge_boundaries(
        &mut self,
        a: &[u32],
        b: &[u32],
        closed: bool,
        material: Option<usize>,
        segments: usize,
        twist: i32,
    ) -> Result<(), String> {
        if a.iter()
            .chain(b)
            .any(|&v| v as usize >= self.verts.len() || !self.verts[v as usize].vec().is_finite())
        {
            return Err("Invalid boundary geometry".into());
        }
        if a.iter().any(|v| b.contains(v)) {
            return Err("Boundaries must not share points".into());
        }
        let mut b = b.to_vec();
        if closed && !b.is_empty() {
            // Preserve boundary winding; choose cyclic offset adjusted by twist.
            let best = (0..b.len())
                .min_by(|&i, &j| {
                    let cost = |shift| {
                        a.iter()
                            .enumerate()
                            .map(|(k, &v)| {
                                let other = b[(k * b.len() / a.len() + shift) % b.len()];
                                (self.verts[v as usize].vec() - self.verts[other as usize].vec())
                                    .length_squared()
                            })
                            .sum::<f32>()
                    };
                    cost(i).total_cmp(&cost(j))
                })
                .unwrap_or(0);
            let offset = ((best as i32 + twist).rem_euclid(b.len() as i32)) as usize;
            b.rotate_left(offset);
        }
        let na = if closed { a.len() } else { a.len() - 1 };
        let nb = if closed { b.len() } else { b.len() - 1 };
        if na == 0 || nb == 0 {
            return Err("Empty boundary".into());
        }

        let segments = segments.max(1);
        let mut intermediate_map = std::collections::HashMap::<(u32, u32, usize), u32>::new();

        let (mut i, mut j) = (0, 0);
        let mut faces = Vec::new();
        while i < na || j < nb {
            // Rational progress avoids floating point tie errors on equal loops.
            let advance_a = i < na && (j == nb || (i + 1) * nb <= (j + 1) * na);
            let advance_b = j < nb && (i == na || (j + 1) * na <= (i + 1) * nb);
            let av = a[i % a.len()];
            let bv = b[j % b.len()];
            let a_next = a[(i + 1) % a.len()];
            let b_next = b[(j + 1) % b.len()];

            for s in 0..segments {
                let u_frac0 = s as f32 / segments as f32;
                let u_frac1 = (s + 1) as f32 / segments as f32;

                let mut get_pt = |mesh: &mut Mesh, u: u32, v: u32, step: usize| -> u32 {
                    if step == 0 {
                        return u;
                    }
                    if step == segments {
                        return v;
                    }
                    let key = (u, v, step);
                    if let Some(&idx) = intermediate_map.get(&key) {
                        return idx;
                    }
                    let p_u = mesh.verts[u as usize].vec();
                    let p_v = mesh.verts[v as usize].vec();
                    let t = step as f32 / segments as f32;
                    let pos = p_u.lerp(p_v, t);
                    let mut new_vert = mesh.verts[u as usize].clone();
                    new_vert.pos = pos.to_array();
                    new_vert.selected = true;
                    let idx = mesh.verts.len() as u32;
                    mesh.verts.push(new_vert);
                    intermediate_map.insert(key, idx);
                    idx
                };

                let (verts, uv) = match (advance_a, advance_b) {
                    (true, true) => {
                        let p0 = get_pt(self, av, bv, s);
                        let p1 = get_pt(self, a_next, b_next, s);
                        let p2 = get_pt(self, a_next, b_next, s + 1);
                        let p3 = get_pt(self, av, bv, s + 1);
                        (
                            vec![p0, p1, p2, p3],
                            vec![
                                [i as f32 / na as f32, u_frac0],
                                [(i + 1) as f32 / na as f32, u_frac0],
                                [(j + 1) as f32 / nb as f32, u_frac1],
                                [j as f32 / nb as f32, u_frac1],
                            ],
                        )
                    }
                    (true, false) => {
                        let p0 = get_pt(self, av, bv, s);
                        let p1 = get_pt(self, a_next, bv, s);
                        let p2 = get_pt(self, a_next, bv, s + 1);
                        let p3 = get_pt(self, av, bv, s + 1);
                        if p2 == p3 {
                            (
                                vec![p0, p1, p2],
                                vec![
                                    [i as f32 / na as f32, u_frac0],
                                    [(i + 1) as f32 / na as f32, u_frac0],
                                    [j as f32 / nb as f32, u_frac1],
                                ],
                            )
                        } else {
                            (
                                vec![p0, p1, p2, p3],
                                vec![
                                    [i as f32 / na as f32, u_frac0],
                                    [(i + 1) as f32 / na as f32, u_frac0],
                                    [j as f32 / nb as f32, u_frac1],
                                    [j as f32 / nb as f32, u_frac1],
                                ],
                            )
                        }
                    }
                    (false, true) => {
                        let p0 = get_pt(self, av, bv, s);
                        let p1 = get_pt(self, av, b_next, s);
                        let p2 = get_pt(self, av, b_next, s + 1);
                        let p3 = get_pt(self, av, bv, s + 1);
                        if p0 == p1 {
                            (
                                vec![p0, p2, p3],
                                vec![
                                    [i as f32 / na as f32, u_frac0],
                                    [(j + 1) as f32 / nb as f32, u_frac1],
                                    [j as f32 / nb as f32, u_frac1],
                                ],
                            )
                        } else {
                            (
                                vec![p0, p1, p2, p3],
                                vec![
                                    [i as f32 / na as f32, u_frac0],
                                    [i as f32 / na as f32, u_frac0],
                                    [(j + 1) as f32 / nb as f32, u_frac1],
                                    [j as f32 / nb as f32, u_frac1],
                                ],
                            )
                        }
                    }
                    _ => return Err("Cannot advance boundary connection".into()),
                };

                let p0 = self.verts[verts[0] as usize].vec();
                if (self.verts[verts[1] as usize].vec() - p0)
                    .cross(self.verts[verts[2] as usize].vec() - p0)
                    .length_squared()
                    < 1.0e-14
                {
                    return Err("Connection would create a degenerate face".into());
                }
                let mut face = Face::with_uv(verts, uv);
                face.selected = true;
                face.material_slot = material;
                faces.push(face);
            }

            if advance_a {
                i += 1;
            }
            if advance_b {
                j += 1;
            }
        }
        self.deselect_all();
        self.faces.extend(faces);
        let (_, defects) = crate::HalfEdgeMesh::from_mesh(self);
        if !defects.is_empty() {
            return Err("Connection would create invalid topology or winding".into());
        }
        self.sync_vert_selection_from_faces();
        Ok(())
    }
}
