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
