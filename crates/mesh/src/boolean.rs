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

/// A successful solid operation can legitimately contain no volume.
#[derive(Clone, Debug)]
pub enum BooleanSolid {
    Empty,
    Mesh(Box<Mesh>),
}

/// Two validated, triangulated operands prepared once for a partition session.
/// Provider handles remain private and never enter document serialization.
pub struct PreparedBooleanOperands {
    a: Manifold,
    b: Manifold,
}

/// Disjoint-membership cells before splitting edge-connected components.
#[derive(Clone, Debug)]
pub struct BooleanPartition {
    pub first_only: BooleanSolid,
    pub overlap: BooleanSolid,
    pub second_only: BooleanSolid,
}

/// Regiões disjuntas dos dois operandos (o que o Shape Builder consome).
#[derive(Clone, Debug)]
pub struct SolidPartition {
    pub regions: Vec<SolidRegion>,
}

/// Particiona dois sólidos em regiões disjuntas (com proveniência).
///
/// Recusa operandos abertos ou não-manifoldes; a subtração é feita uma vez,
/// sem reparo ou remesh.
pub fn partition_solids(first: &Mesh, second: &Mesh) -> Result<SolidPartition, BooleanError> {
    let prepared = PreparedBooleanOperands::new(first, second)?;
    let regions = prepared.decompose_regions(first, second)?;
    Ok(SolidPartition { regions })
}

impl PreparedBooleanOperands {
    pub fn new(a: &Mesh, b: &Mesh) -> Result<Self, BooleanError> {
        let a = to_provider(a).ok_or(BooleanError::InvalidInput("mesh A"))?;
        let b = to_provider(b).ok_or(BooleanError::InvalidInput("mesh B"))?;
        if a.status() != manifold_rust::types::Error::NoError {
            return Err(BooleanError::InvalidInput(
                "mesh A is not a closed manifold",
            ));
        }
        if b.status() != manifold_rust::types::Error::NoError {
            return Err(BooleanError::InvalidInput(
                "mesh B is not a closed manifold",
            ));
        }
        Ok(Self { a, b })
    }

    pub fn evaluate(&self, op: BooleanOp) -> Result<BooleanSolid, BooleanError> {
        let result = match op {
            BooleanOp::Union => self.a.union(&self.b),
            BooleanOp::Difference => self.a.difference(&self.b),
            BooleanOp::Intersection => self.a.intersection(&self.b),
        };
        solid_from_provider(&result)
    }

    pub fn partition(&self) -> Result<BooleanPartition, BooleanError> {
        Ok(BooleanPartition {
            first_only: self.evaluate(BooleanOp::Difference)?,
            overlap: self.evaluate(BooleanOp::Intersection)?,
            second_only: solid_from_provider(&self.b.difference(&self.a))?,
        })
    }

    /// Decomposes the two operands into edge-connected `SolidRegion`s with provenance,
    /// material slots and UV preservation.
    pub fn decompose_regions(&self, a: &Mesh, b: &Mesh) -> Result<Vec<SolidRegion>, BooleanError> {
        let partition = self.partition()?;
        let source_index = crate::boolean_cleanup::SourceIndex::from_operands(a, b);
        let mut regions = Vec::new();
        let mut next_id = 1u32;

        for (class, solid) in [
            (SolidRegionClass::FirstOnly, partition.first_only),
            (SolidRegionClass::Intersection, partition.overlap),
            (SolidRegionClass::SecondOnly, partition.second_only),
        ] {
            let components = solid.connected_regions()?;
            for mut mesh in components {
                let mut provenance = Vec::with_capacity(mesh.faces.len());
                for face in &mut mesh.faces {
                    if face.verts.len() >= 3 {
                        let v0 = mesh.verts.get(face.verts[0] as usize);
                        let v1 = mesh.verts.get(face.verts[1] as usize);
                        let v2 = mesh.verts.get(face.verts[2] as usize);
                        if let (Some(v0), Some(v1), Some(v2)) = (v0, v1, v2) {
                            let tri = [
                                crate::boolean_cleanup::dvec(v0.vec()),
                                crate::boolean_cleanup::dvec(v1.vec()),
                                crate::boolean_cleanup::dvec(v2.vec()),
                            ];
                            let origin = source_index.locate(tri);
                            match origin {
                                Some(crate::boolean_cleanup::Origin { side: 0, face: fa }) => {
                                    provenance.push(SurfaceOrigin::First(fa));
                                    face.material_slot =
                                        a.faces.get(fa as usize).and_then(|f| f.material_slot);
                                    face.uv = face
                                        .verts
                                        .iter()
                                        .map(|&v| {
                                            if let Some(v_ref) = mesh.verts.get(v as usize) {
                                                crate::boolean_cleanup::source_uv(
                                                    a,
                                                    fa as usize,
                                                    v_ref.vec(),
                                                )
                                            } else {
                                                [0.0, 0.0]
                                            }
                                        })
                                        .collect();
                                }
                                Some(crate::boolean_cleanup::Origin { side: 1, face: fb }) => {
                                    provenance.push(SurfaceOrigin::Second(fb));
                                    face.material_slot =
                                        b.faces.get(fb as usize).and_then(|f| f.material_slot);
                                    face.uv = face
                                        .verts
                                        .iter()
                                        .map(|&v| {
                                            if let Some(v_ref) = mesh.verts.get(v as usize) {
                                                crate::boolean_cleanup::source_uv(
                                                    b,
                                                    fb as usize,
                                                    v_ref.vec(),
                                                )
                                            } else {
                                                [0.0, 0.0]
                                            }
                                        })
                                        .collect();
                                }
                                _ => {
                                    provenance.push(SurfaceOrigin::Generated);
                                    face.material_slot =
                                        a.faces.first().and_then(|f| f.material_slot);
                                    if face.uv.len() != face.verts.len() {
                                        face.uv = vec![[0.0, 0.0]; face.verts.len()];
                                    }
                                }
                            }
                        } else {
                            provenance.push(SurfaceOrigin::Generated);
                        }
                    } else {
                        provenance.push(SurfaceOrigin::Generated);
                    }
                }
                regions.push(SolidRegion::new(next_id, class, mesh, provenance));
                next_id = next_id.saturating_add(1);
            }
        }

        Ok(regions)
    }
}

/// Class of a solid region in a two-operand partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SolidRegionClass {
    FirstOnly,
    Intersection,
    SecondOnly,
}

/// Provenance of a face in a solid region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SurfaceOrigin {
    First(u32),
    Second(u32),
    Generated,
}

/// A disjoint solid region resulting from decomposing prepared boolean operands.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SolidRegion {
    pub id: u32,
    pub class: SolidRegionClass,
    pub mesh: Mesh,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub provenance: Vec<SurfaceOrigin>,
}

impl SolidRegion {
    pub fn new(
        id: u32,
        class: SolidRegionClass,
        mesh: Mesh,
        provenance: Vec<SurfaceOrigin>,
    ) -> Self {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for v in &mesh.verts {
            for i in 0..3 {
                if v.pos[i] < min[i] {
                    min[i] = v.pos[i];
                }
                if v.pos[i] > max[i] {
                    max[i] = v.pos[i];
                }
            }
        }
        if min[0].is_infinite() {
            min = [0.0; 3];
            max = [0.0; 3];
        }
        Self {
            id,
            class,
            mesh,
            bounds_min: min,
            bounds_max: max,
            provenance,
        }
    }
}

/// Transient ownership plan for solid regions before commit.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RegionPlan {
    /// Output groups containing region IDs.
    /// Index 0 = Group A (default owner of FirstOnly + Overlap).
    /// Index 1 = Group B (default owner of SecondOnly + Overlap).
    /// Index 2+ = Extracted or separate output groups.
    pub groups: Vec<Vec<u32>>,
}

impl RegionPlan {
    /// Creates the initial ownership plan from a set of solid regions.
    /// Group 0 (A) owns FirstOnly + Overlap.
    /// Group 1 (B) owns SecondOnly + Overlap.
    pub fn initial(regions: &[SolidRegion]) -> Self {
        let mut group_a = Vec::new();
        let mut group_b = Vec::new();
        for r in regions {
            match r.class {
                SolidRegionClass::FirstOnly => group_a.push(r.id),
                SolidRegionClass::SecondOnly => group_b.push(r.id),
                SolidRegionClass::Intersection => {
                    group_a.push(r.id);
                    group_b.push(r.id);
                }
            }
        }
        Self {
            groups: vec![group_a, group_b],
        }
    }

    /// Removes region IDs from all groups.
    pub fn remove(&mut self, region_ids: &[u32]) {
        for group in &mut self.groups {
            group.retain(|id| !region_ids.contains(id));
        }
    }

    /// Removes region IDs from existing groups and creates a new output group containing them.
    /// Returns the index of the newly created group.
    pub fn extract(&mut self, region_ids: &[u32]) -> usize {
        self.remove(region_ids);
        self.groups.push(region_ids.to_vec());
        self.groups.len().saturating_sub(1)
    }

    /// Merges specified regions into a single target group, removing them from other groups.
    pub fn unite(&mut self, region_ids: &[u32]) -> usize {
        self.remove(region_ids);
        self.groups.push(region_ids.to_vec());
        self.groups.len().saturating_sub(1)
    }

    /// Unites all regions into Group 0 (equivalent to A ∪ B) and empties other groups.
    pub fn unite_all(&mut self, regions: &[SolidRegion]) {
        self.groups = vec![regions.iter().map(|r| r.id).collect()];
    }

    /// Commits the plan into output meshes.
    /// Each group with regions is converted into an output `Mesh`.
    /// Groups with 0 regions produce `None`.
    pub fn commit(&self, regions: &[SolidRegion]) -> Result<Vec<Option<Mesh>>, BooleanError> {
        let mut outputs = Vec::with_capacity(self.groups.len());
        for group in &self.groups {
            if group.is_empty() {
                outputs.push(None);
                continue;
            }
            if group.len() == 1 {
                let id = group[0];
                let Some(region) = regions.iter().find(|r| r.id == id) else {
                    return Err(BooleanError::InvalidOutput("referenced region not found"));
                };
                outputs.push(Some(region.mesh.clone()));
                continue;
            }
            // Multiple regions in this group: combine them.
            let mut combined = Mesh::default();
            let mut first = true;
            for &id in group {
                let Some(region) = regions.iter().find(|r| r.id == id) else {
                    return Err(BooleanError::InvalidOutput("referenced region not found"));
                };
                if first {
                    combined = region.mesh.clone();
                    first = false;
                } else {
                    let mut a = combined.clone();
                    let mut b = region.mesh.clone();
                    a.triangulate();
                    b.triangulate();
                    match boolean_solids(&a, &b, BooleanOp::Union) {
                        Ok(BooleanSolid::Mesh(m)) => {
                            combined = *m;
                        }
                        Ok(BooleanSolid::Empty) => {}
                        Err(_) => {
                            // Fallback concatenation of loose parts
                            let v_offset = u32::try_from(combined.verts.len()).map_err(|_| {
                                BooleanError::InvalidOutput("combined mesh too large")
                            })?;
                            combined.verts.extend(region.mesh.verts.iter().cloned());
                            for f in &region.mesh.faces {
                                let mut face = f.clone();
                                for idx in &mut face.verts {
                                    *idx = idx.saturating_add(v_offset);
                                }
                                combined.faces.push(face);
                            }
                        }
                    }
                }
            }
            outputs.push(Some(combined));
        }
        Ok(outputs)
    }
}

impl BooleanSolid {
    /// Regions share edges, not just a single touching vertex.
    pub fn connected_regions(&self) -> Result<Vec<Mesh>, BooleanError> {
        match self {
            Self::Empty => Ok(Vec::new()),
            Self::Mesh(mesh) => crate::boolean_partition::edge_connected_components(mesh),
        }
    }
}

/// Success with zero volume is separate from malformed input/kernel failures.
pub fn boolean_solids(a: &Mesh, b: &Mesh, op: BooleanOp) -> Result<BooleanSolid, BooleanError> {
    PreparedBooleanOperands::new(a, b)?.evaluate(op)
}

fn solid_from_provider(result: &Manifold) -> Result<BooleanSolid, BooleanError> {
    if result.status() != manifold_rust::types::Error::NoError {
        return Err(BooleanError::Kernel);
    }
    if result.is_empty() {
        return Ok(BooleanSolid::Empty);
    }
    from_provider(result)
        .map(|mesh| BooleanSolid::Mesh(Box::new(mesh)))
        .ok_or(BooleanError::InvalidOutput("mesh"))
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
    // Compatibility API requires a concrete mesh. New region workflows use
    // boolean_solids so a valid empty result is not mistaken for a failure.
    match boolean_solids(a, b, op)? {
        BooleanSolid::Mesh(mesh) => Ok(*mesh),
        BooleanSolid::Empty => Err(BooleanError::InvalidOutput("mesh")),
    }
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
    let vertex_capacity = mesh.verts.len().checked_mul(3)?;
    let face_capacity = mesh.faces.len().checked_mul(3)?;
    let count = u32::try_from(mesh.verts.len()).ok()?;
    let mut vert_properties = Vec::with_capacity(vertex_capacity);
    for v in &mesh.verts {
        if !v.pos.iter().all(|x| x.is_finite()) {
            return None;
        }
        vert_properties.extend_from_slice(&v.pos);
    }
    let mut tri_verts = Vec::with_capacity(face_capacity);
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
    let count = u32::try_from(mesh.verts.len()).ok()?;
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

const MAX_SOLID_TRIANGLES: usize = 200_000;
const MAX_SOLID_COMPONENTS: usize = 128;


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
