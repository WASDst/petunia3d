//! 3D boolean provider behind a Petunia boundary (`manifold-rust`, P0-04).
//!
//! Provider mesh types never leak: Petunia [`Mesh`] goes in, a validated
//! Petunia [`Mesh`] comes out. Every conversion validates finiteness,
//! indices and manifold status, and failures carry stable diagnostics.

use manifold_rust::manifold::Manifold;
use manifold_rust::types::MeshGL;

use crate::{Face, Mesh, Vertex};

/// Boolean operation supported by the provider boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    /// Union (A + B).
    Union,
    /// Difference (A - B).
    Difference,
    /// Intersection (A ∩ B).
    Intersection,
}

/// Boolean provider failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BooleanError {
    /// An input mesh violates preconditions (empty, non-finite, bad indices).
    #[error("boolean input invalid: {0}")]
    InvalidInput(&'static str),
    /// The provider reported a non-ok status.
    #[error("boolean kernel error")]
    Kernel,
    /// The provider output violates domain invariants.
    #[error("boolean output invalid: {0}")]
    InvalidOutput(&'static str),
}

/// Runs `op` on two closed triangle meshes.
///
/// Inputs must be triangulated with finite coordinates. Output is validated
/// the same way before returning.
pub fn boolean_meshes(a: &Mesh, b: &Mesh, op: BooleanOp) -> Result<Mesh, BooleanError> {
    let ma = to_provider(a).ok_or(BooleanError::InvalidInput("mesh A"))?;
    let mb = to_provider(b).ok_or(BooleanError::InvalidInput("mesh B"))?;
    let result = match op {
        BooleanOp::Union => ma.union(&mb),
        BooleanOp::Difference => ma.difference(&mb),
        BooleanOp::Intersection => ma.intersection(&mb),
    };
    if result.status() != manifold_rust::types::Error::NoError {
        return Err(BooleanError::Kernel);
    }
    from_provider(&result).ok_or(BooleanError::InvalidOutput("mesh"))
}

/// Booleano com cleanup seguro (cap. 04): triangula as cópias para o kernel e
/// devolve o resultado com as regiões coplanares fundidas de volta em quads,
/// n-gons e faixas (anéis), mais o relatório do que foi feito.
///
/// `a` e `b` são os operandos originais (não triangulados).
pub fn boolean_meshes_clean(
    a: &Mesh,
    b: &Mesh,
    op: BooleanOp,
) -> Result<(Mesh, crate::boolean_cleanup::CleanupReport), BooleanError> {
    let (mut ta, mut tb) = (a.clone(), b.clone());
    ta.triangulate();
    tb.triangulate();
    let raw = boolean_meshes(&ta, &tb, op)?;
    Ok(crate::boolean_cleanup::cleanup_boolean_result(&raw, a, b))
}

/// Union of two axis-aligned cubes; the canonical smoke fixture.
pub fn boolean_cubes(size_a: f64, size_b: f64, op: BooleanOp) -> Result<Mesh, BooleanError> {
    use manifold_rust::linalg::Vec3;
    let a = Manifold::cube(Vec3::new(size_a, size_a, size_a), true);
    let b = Manifold::cube(Vec3::new(size_b, size_b, size_b), true);
    let result = match op {
        BooleanOp::Union => a.union(&b),
        BooleanOp::Difference => a.difference(&b),
        BooleanOp::Intersection => a.intersection(&b),
    };
    if result.status() != manifold_rust::types::Error::NoError {
        return Err(BooleanError::Kernel);
    }
    from_provider(&result).ok_or(BooleanError::InvalidOutput("cube result"))
}

fn to_provider(mesh: &Mesh) -> Option<Manifold> {
    if mesh.verts.is_empty() || mesh.faces.is_empty() {
        return None;
    }
    let mut vert_properties = Vec::with_capacity(mesh.verts.len() * 3);
    for v in &mesh.verts {
        if !v.pos.iter().all(|x| x.is_finite()) {
            return None;
        }
        vert_properties.extend_from_slice(&v.pos);
    }
    let mut tri_verts = Vec::with_capacity(mesh.faces.len() * 3);
    let count = mesh.verts.len() as u32;
    for f in &mesh.faces {
        if f.verts.len() != 3 {
            return None;
        }
        if f.verts.iter().any(|i| *i >= count) {
            return None;
        }
        tri_verts.extend_from_slice(&f.verts);
    }
    let mesh_gl = MeshGL {
        num_prop: 3,
        vert_properties,
        tri_verts,
        ..Default::default()
    };
    Some(Manifold::from_mesh_gl(&mesh_gl))
}

fn from_provider(manifold: &Manifold) -> Option<Mesh> {
    if manifold.status() != manifold_rust::types::Error::NoError || manifold.is_empty() {
        return None;
    }
    let mesh_gl = manifold.get_mesh_gl(-1);
    if !mesh_gl.vert_properties.len().is_multiple_of(3)
        || !mesh_gl.tri_verts.len().is_multiple_of(3)
    {
        return None;
    }
    let mut mesh = Mesh::default();
    let (vert_chunks, _) = mesh_gl.vert_properties.as_chunks::<3>();
    for chunk in vert_chunks {
        let [x, y, z] = *chunk;
        if ![x, y, z].iter().all(|v| v.is_finite()) {
            return None;
        }
        mesh.verts.push(Vertex {
            pos: [x, y, z],
            color: [0.75, 0.75, 0.78],
            selected: false,
        });
    }
    let count = mesh.verts.len() as u32;
    let (tri_chunks, _) = mesh_gl.tri_verts.as_chunks::<3>();
    for tri in tri_chunks {
        let [a, b, c] = *tri;
        if a >= count || b >= count || c >= count {
            return None;
        }
        let mut face = Face::new(vec![a, b, c]);
        face.uv = vec![[0.0, 0.0]; 3];
        mesh.faces.push(face);
    }
    if mesh.faces.is_empty() {
        return None;
    }
    Some(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_mesh(size: f32) -> Mesh {
        Mesh::cube(size)
    }

    #[test]
    fn union_of_identical_cubes_has_cube_volume() {
        let mesh = boolean_cubes(2.0, 2.0, BooleanOp::Union).unwrap();
        assert!(!mesh.faces.is_empty());
        // Cube 2x2x2 centered: volume 8. Triangulated cube has 12 tris.
        assert_eq!(mesh.faces.len(), 12);
    }

    #[test]
    fn difference_of_identical_cubes_is_empty_or_rejected() {
        // A - A removes everything: provider yields empty, mapped to InvalidOutput.
        assert_eq!(
            boolean_cubes(2.0, 2.0, BooleanOp::Difference).unwrap_err(),
            BooleanError::InvalidOutput("cube result")
        );
    }

    #[test]
    fn mesh_roundtrip_through_provider() {
        let cube = cube_mesh(1.0);
        let triangulated = {
            let mut m = cube.clone();
            m.triangulate();
            m
        };
        let back = boolean_meshes(&triangulated, &triangulated, BooleanOp::Union).unwrap();
        assert!(!back.faces.is_empty());
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let empty = Mesh::default();
        assert_eq!(
            boolean_meshes(&empty, &empty, BooleanOp::Union).unwrap_err(),
            BooleanError::InvalidInput("mesh A")
        );
        let mut bad = Mesh::cube(1.0);
        bad.triangulate();
        bad.verts[0].pos[0] = f32::NAN;
        assert_eq!(
            boolean_meshes(&bad, &bad, BooleanOp::Union).unwrap_err(),
            BooleanError::InvalidInput("mesh A")
        );
    }

    #[test]
    fn disjoint_union_keeps_both_volumes() {
        use manifold_rust::linalg::Vec3;
        let a = Manifold::cube(Vec3::new(1.0, 1.0, 1.0), true);
        let b = Manifold::cube(Vec3::new(1.0, 1.0, 1.0), true).translate(Vec3::new(5.0, 0.0, 0.0));
        let union = a.union(&b);
        assert!((union.volume() - 2.0).abs() < 1e-6);
    }
}

/// A mathematically empty boolean is a valid result, distinct from kernel failure.
#[derive(Debug, Clone)]
pub enum BooleanSolid {
    Empty,
    Mesh(Box<Mesh>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidRegionClass {
    FirstOnly,
    Intersection,
    SecondOnly,
}

#[derive(Debug, Clone)]
pub struct SolidRegion {
    pub class: SolidRegionClass,
    pub mesh: Mesh,
}

#[derive(Debug, Clone)]
pub struct BooleanPartition {
    pub regions: Vec<SolidRegion>,
}

const MAX_SOLID_TRIANGLES: usize = 200_000;
const MAX_SOLID_COMPONENTS: usize = 128;

/// Prepare the operands once, then derive the three disjoint volume classes.
/// No repairs, remeshing or per-pointer evaluation are performed here.
pub fn partition_solids(first: &Mesh, second: &Mesh) -> Result<BooleanPartition, BooleanError> {
    validate_solid_operand(first)?;
    validate_solid_operand(second)?;
    let (mut a, mut b) = (first.clone(), second.clone());
    a.triangulate();
    b.triangulate();
    if a.faces.len().saturating_add(b.faces.len()) > MAX_SOLID_TRIANGLES {
        return Err(BooleanError::InvalidInput("solid triangle budget"));
    }
    let ma = to_provider(&a).ok_or(BooleanError::InvalidInput("first solid"))?;
    let mb = to_provider(&b).ok_or(BooleanError::InvalidInput("second solid"))?;
    if ma.status() != manifold_rust::types::Error::NoError
        || mb.status() != manifold_rust::types::Error::NoError
    {
        return Err(BooleanError::InvalidInput(
            "operands must be manifold closed solids",
        ));
    }
    let bounds = |mesh: &Mesh| {
        mesh.verts.iter().fold(
            (
                glam::Vec3::splat(f32::INFINITY),
                glam::Vec3::splat(f32::NEG_INFINITY),
            ),
            |(min, max), v| (min.min(v.vec()), max.max(v.vec())),
        )
    };
    let (amin, amax) = bounds(first);
    let (bmin, bmax) = bounds(second);
    if amax.cmplt(bmin).any() || bmax.cmplt(amin).any() {
        let mut regions = Vec::new();
        for mesh in connected_face_components(first)? {
            regions.push(SolidRegion {
                class: SolidRegionClass::FirstOnly,
                mesh,
            });
        }
        for mesh in connected_face_components(second)? {
            regions.push(SolidRegion {
                class: SolidRegionClass::SecondOnly,
                mesh,
            });
        }
        if regions.len() > MAX_SOLID_COMPONENTS {
            return Err(BooleanError::InvalidOutput("too many disconnected regions"));
        }
        return Ok(BooleanPartition { regions });
    }
    let mut regions = Vec::new();
    let mut decomposed_faces = 0usize;
    for (class, result) in [
        (SolidRegionClass::FirstOnly, ma.difference(&mb)),
        (SolidRegionClass::Intersection, ma.intersection(&mb)),
        (SolidRegionClass::SecondOnly, mb.difference(&ma)),
    ] {
        if result.status() != manifold_rust::types::Error::NoError {
            return Err(BooleanError::Kernel);
        }
        if result.is_empty() {
            continue;
        }
        let raw = from_provider(&result).ok_or(BooleanError::InvalidOutput("partition"))?;
        decomposed_faces = decomposed_faces
            .checked_add(raw.faces.len())
            .ok_or(BooleanError::InvalidOutput("partition budget"))?;
        if decomposed_faces > MAX_SOLID_TRIANGLES {
            return Err(BooleanError::InvalidOutput("partition budget"));
        }
        let (mesh, _) = crate::boolean_cleanup::cleanup_boolean_surfaces(&raw, first, second);
        for component in connected_face_components(&mesh)? {
            regions.push(SolidRegion {
                class,
                mesh: component,
            });
            if regions.len() > MAX_SOLID_COMPONENTS {
                return Err(BooleanError::InvalidOutput("too many disconnected regions"));
            }
        }
    }
    Ok(BooleanPartition { regions })
}

fn validate_solid_operand(mesh: &Mesh) -> Result<(), BooleanError> {
    mesh.validate_invariants()
        .map_err(|_| BooleanError::InvalidInput("mesh invariants"))?;
    if mesh.faces.is_empty()
        || mesh.faces.len() > MAX_SOLID_TRIANGLES
        || mesh.verts.len() > MAX_SOLID_TRIANGLES
    {
        return Err(BooleanError::InvalidInput("empty or excessive solid"));
    }
    let triangles = mesh
        .faces
        .iter()
        .try_fold(0usize, |n, f| {
            n.checked_add(f.verts.len().saturating_sub(2))
        })
        .ok_or(BooleanError::InvalidInput("triangle budget"))?;
    if triangles > MAX_SOLID_TRIANGLES {
        return Err(BooleanError::InvalidInput("triangle budget"));
    }
    let mut edges = std::collections::HashMap::<(u32, u32), (u32, i32)>::new();
    for face in &mesh.faces {
        for (a, b) in face
            .verts
            .iter()
            .zip(face.verts.iter().cycle().skip(1))
            .take(face.verts.len())
        {
            let key = if a < b { (*a, *b) } else { (*b, *a) };
            let entry = edges.entry(key).or_default();
            entry.0 = entry.0.saturating_add(1);
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    if edges
        .values()
        .any(|(count, direction)| *count != 2 || *direction != 0)
    {
        return Err(BooleanError::InvalidInput(
            "mesh must be closed and consistently oriented",
        ));
    }
    Ok(())
}

/// Edge-connected face components. Vertex-only contacts remain separate.
pub fn connected_face_components(mesh: &Mesh) -> Result<Vec<Mesh>, BooleanError> {
    use std::collections::{HashMap, VecDeque};
    mesh.validate_invariants()
        .map_err(|_| BooleanError::InvalidInput("component mesh"))?;
    if mesh.faces.len() > MAX_SOLID_TRIANGLES {
        return Err(BooleanError::InvalidInput("component budget"));
    }
    let mut incident: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        for (a, b) in face
            .verts
            .iter()
            .zip(face.verts.iter().cycle().skip(1))
            .take(face.verts.len())
        {
            incident
                .entry(if a < b { (*a, *b) } else { (*b, *a) })
                .or_default()
                .push(index);
        }
    }
    let mut visited = vec![false; mesh.faces.len()];
    let mut result = Vec::new();
    for seed in 0..mesh.faces.len() {
        if visited.get(seed).copied().unwrap_or(true) {
            continue;
        }
        let mut queue = VecDeque::from([seed]);
        let mut faces = Vec::new();
        while let Some(index) = queue.pop_front() {
            let Some(flag) = visited.get_mut(index) else {
                continue;
            };
            if *flag {
                continue;
            }
            *flag = true;
            let Some(face) = mesh.faces.get(index) else {
                continue;
            };
            faces.push(index);
            for (a, b) in face
                .verts
                .iter()
                .zip(face.verts.iter().cycle().skip(1))
                .take(face.verts.len())
            {
                let key = if a < b { (*a, *b) } else { (*b, *a) };
                if let Some(neighbors) = incident.get(&key) {
                    for neighbor in neighbors {
                        if !visited.get(*neighbor).copied().unwrap_or(true) {
                            queue.push_back(*neighbor);
                        }
                    }
                }
            }
        }
        if result.len() >= MAX_SOLID_COMPONENTS {
            return Err(BooleanError::InvalidOutput("component budget"));
        }
        let mut compact = Mesh::default();
        let mut remap: HashMap<u32, u32> = HashMap::new();
        for index in faces {
            let Some(source) = mesh.faces.get(index) else {
                return Err(BooleanError::InvalidInput("missing face"));
            };
            let mut face = source.clone();
            for id in &mut face.verts {
                let next = if let Some(existing) = remap.get(id) {
                    *existing
                } else {
                    let vertex = mesh
                        .verts
                        .get(*id as usize)
                        .ok_or(BooleanError::InvalidInput("missing vertex"))?;
                    let next = u32::try_from(compact.verts.len())
                        .map_err(|_| BooleanError::InvalidOutput("vertex budget"))?;
                    compact.verts.push(vertex.clone());
                    remap.insert(*id, next);
                    next
                };
                *id = next;
            }
            let next_face = compact.faces.len();
            for (pinned_face, corner) in &mesh.uv_pinned {
                if *pinned_face == index {
                    compact.uv_pinned.insert((next_face, *corner));
                }
            }
            compact.faces.push(face);
        }
        for (a, b) in &mesh.sharp_edges {
            if let (Some(a), Some(b)) = (remap.get(a), remap.get(b)) {
                compact
                    .sharp_edges
                    .insert(if a < b { (*a, *b) } else { (*b, *a) });
            }
        }
        for (a, b) in &mesh.selected_edges {
            if let (Some(a), Some(b)) = (remap.get(a), remap.get(b)) {
                compact
                    .selected_edges
                    .insert(if a < b { (*a, *b) } else { (*b, *a) });
            }
        }
        for (a, b) in &mesh.uv_seams {
            if let (Some(a), Some(b)) = (remap.get(a), remap.get(b)) {
                compact
                    .uv_seams
                    .insert(if a < b { (*a, *b) } else { (*b, *a) });
            }
        }
        result.push(compact);
    }
    Ok(result)
}

pub fn boolean_solid(a: &Mesh, b: &Mesh, op: BooleanOp) -> Result<BooleanSolid, BooleanError> {
    validate_solid_operand(a)?;
    validate_solid_operand(b)?;
    let (mut ta, mut tb) = (a.clone(), b.clone());
    ta.triangulate();
    tb.triangulate();
    let ma = to_provider(&ta).ok_or(BooleanError::InvalidInput("first solid"))?;
    let mb = to_provider(&tb).ok_or(BooleanError::InvalidInput("second solid"))?;
    let result = match op {
        BooleanOp::Union => ma.union(&mb),
        BooleanOp::Difference => ma.difference(&mb),
        BooleanOp::Intersection => ma.intersection(&mb),
    };
    if result.status() != manifold_rust::types::Error::NoError {
        return Err(BooleanError::Kernel);
    }
    if result.is_empty() {
        return Ok(BooleanSolid::Empty);
    }
    let raw = from_provider(&result).ok_or(BooleanError::InvalidOutput("solid result"))?;
    Ok(BooleanSolid::Mesh(Box::new(
        crate::boolean_cleanup::cleanup_boolean_surfaces(&raw, a, b).0,
    )))
}
