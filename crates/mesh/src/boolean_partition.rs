//! Edge-connected regions of a solid partition. Never weld vertex-only contacts.
use crate::boolean::BooleanError;
use crate::{Mesh, edge_key};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(crate) fn edge_connected_components(mesh: &Mesh) -> Result<Vec<Mesh>, BooleanError> {
    let mut incident: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); mesh.faces.len()];
    for (fi, face) in mesh.faces.iter().enumerate() {
        if face.verts.len() < 3
            || face.verts.len() != face.uv.len()
            || face.uv.iter().flatten().any(|v| !v.is_finite())
        {
            return Err(BooleanError::InvalidOutput("invalid face corners"));
        }
        for (ci, &a) in face.verts.iter().enumerate() {
            let Some(&b) = face.verts.get((ci + 1) % face.verts.len()) else {
                return Err(BooleanError::InvalidOutput("invalid edge"));
            };
            if mesh.verts.get(a as usize).is_none() || mesh.verts.get(b as usize).is_none() {
                return Err(BooleanError::InvalidOutput("invalid vertex index"));
            }
            let edge = incident.entry(edge_key(a, b)).or_default();
            if a == b || edge.len() >= 2 {
                return Err(BooleanError::InvalidOutput("non-manifold edge"));
            }
            for &other in edge.iter() {
                if let Some(adj) = neighbors.get_mut(fi) {
                    adj.push(other);
                }
                if let Some(adj) = neighbors.get_mut(other) {
                    adj.push(fi);
                }
            }
            edge.push(fi);
        }
    }
    let mut seen = BTreeSet::new();
    let mut regions = Vec::new();
    for start in 0..mesh.faces.len() {
        if !seen.insert(start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut faces = BTreeSet::new();
        while let Some(fi) = queue.pop_front() {
            faces.insert(fi);
            for &other in neighbors.get(fi).into_iter().flatten() {
                if seen.insert(other) {
                    queue.push_back(other);
                }
            }
        }
        let mut region = Mesh::default();
        let mut remap = BTreeMap::new();
        for old_face in faces {
            let Some(face) = mesh.faces.get(old_face) else {
                return Err(BooleanError::InvalidOutput("missing face"));
            };
            let mut next = face.clone();
            let new_face = region.faces.len();
            for (ci, index) in next.verts.iter_mut().enumerate() {
                let old = *index;
                let mapped = if let Some(&new) = remap.get(&old) {
                    new
                } else {
                    let vertex = mesh
                        .verts
                        .get(old as usize)
                        .ok_or(BooleanError::InvalidOutput("missing vertex"))?;
                    if !vertex.vec().is_finite() {
                        return Err(BooleanError::InvalidOutput("non-finite vertex"));
                    }
                    let new = u32::try_from(region.verts.len())
                        .map_err(|_| BooleanError::InvalidOutput("region too large"))?;
                    remap.insert(old, new);
                    region.verts.push(vertex.clone());
                    new
                };
                *index = mapped;
                if mesh.uv_pinned.contains(&(old_face, ci)) {
                    region.uv_pinned.insert((new_face, ci));
                }
            }
            region.faces.push(next);
        }
        for (source, target) in [
            (&mesh.selected_edges, &mut region.selected_edges),
            (&mesh.uv_seams, &mut region.uv_seams),
            (&mesh.sharp_edges, &mut region.sharp_edges),
        ] {
            for &(a, b) in source {
                if let (Some(&a), Some(&b)) = (remap.get(&a), remap.get(&b)) {
                    target.insert(edge_key(a, b));
                }
            }
        }
        regions.push(region);
    }
    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    #[test]
    fn touching_at_one_vertex_is_not_a_connected_region() {
        let mut mesh = Mesh::cube(1.0);
        // Two surface triangles share vertex 0 but no edge.
        mesh.faces = vec![Face::new(vec![0, 1, 2]), Face::new(vec![0, 3, 4])];
        mesh.faces[0].uv = vec![[0.1, 0.2]; 3];
        mesh.faces[1].uv = vec![[0.3, 0.4]; 3];
        mesh.uv_pinned.insert((1, 1));
        let regions = edge_connected_components(&mesh).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[1].faces[0].uv, mesh.faces[1].uv);
        assert!(regions[1].uv_pinned.contains(&(0, 1)));
    }

    #[test]
    fn corrupt_indices_are_errors() {
        let mut mesh = Mesh::cube(1.0);
        mesh.faces[0].verts[0] = u32::MAX;
        assert!(edge_connected_components(&mesh).is_err());
    }
}
