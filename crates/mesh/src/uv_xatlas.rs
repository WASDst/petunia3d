//! Generic UV unwrap fallback provider (`xatlas-rs-v2`, P1-06).
//!
//! Petunia native deterministic/projection workflows (`project_planar` and
//! friends) remain first-class. This provider is the generic fallback for
//! arbitrary meshes: it runs xatlas charts on a triangulated copy and writes
//! averaged per-corner UVs back into the Petunia [`Mesh`]. No xatlas type
//! ever becomes the canonical project representation.
//!
//! Provider note: `xatlas-rs` 0.1.3 ships Windows-only pre-generated bindings
//! (unusable on Linux) and its `generate_bindings` path fails against modern
//! libclang, so this boundary uses `xatlas-rs-v2` 0.1.4 — the maintained
//! evolution from the same upstream repository.

use crate::Mesh;

/// xatlas input limits (hostile-mesh class).
const MAX_VERTS: usize = 200_000;
const MAX_TRIS: usize = 400_000;

/// Fallback unwrap failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UnwrapError {
    /// Nothing to unwrap (empty mesh).
    #[error("mesh is empty")]
    Empty,
    /// Mesh exceeds provider limits.
    #[error("mesh exceeds xatlas fallback limits")]
    TooLarge,
    /// Non-finite coordinates or bad indices.
    #[error("mesh has invalid geometry")]
    InvalidGeometry,
    /// Provider produced no usable UVs.
    #[error("xatlas produced no UVs")]
    NoOutput,
}

/// Unwraps `mesh` in place with the xatlas fallback.
///
/// The mesh is triangulated first when needed; quad topology is therefore
/// not preserved on this path (documented). Returns the number of charts.
pub fn unwrap_fallback(mesh: &mut Mesh) -> Result<usize, UnwrapError> {
    if mesh.verts.is_empty() || mesh.faces.is_empty() {
        return Err(UnwrapError::Empty);
    }
    mesh.triangulate();
    if mesh.verts.len() > MAX_VERTS || mesh.faces.len() > MAX_TRIS {
        return Err(UnwrapError::TooLarge);
    }
    if mesh
        .verts
        .iter()
        .any(|v| !v.pos.iter().all(|x| x.is_finite()))
    {
        return Err(UnwrapError::InvalidGeometry);
    }
    let count = mesh.verts.len() as u32;
    let mut positions = Vec::with_capacity(mesh.verts.len() * 3);
    for v in &mesh.verts {
        positions.extend_from_slice(&v.pos);
    }
    let mut indices = Vec::with_capacity(mesh.faces.len() * 3);
    for face in &mesh.faces {
        if face.verts.len() != 3 || face.verts.iter().any(|i| *i >= count) {
            return Err(UnwrapError::InvalidGeometry);
        }
        indices.extend_from_slice(&face.verts);
    }

    let decl = xatlas_rs_v2::MeshDecl {
        vertex_position_data: xatlas_rs_v2::MeshData::Contiguous(&positions),
        index_data: Some(xatlas_rs_v2::IndexData::U32(&indices)),
        face_count: mesh.faces.len() as u32,
        ..Default::default()
    };
    // Declared after `decl` so it drops first (it borrows the buffers).
    let mut atlas = xatlas_rs_v2::Xatlas::new();
    atlas
        .add_mesh(&decl)
        .map_err(|_| UnwrapError::InvalidGeometry)?;
    atlas.generate(
        &xatlas_rs_v2::ChartOptions::default(),
        &xatlas_rs_v2::PackOptions {
            padding: 2,
            bilinear: true,
            resolution: 1024,
            ..Default::default()
        },
    );
    let (atlas_w, atlas_h) = (atlas.width().max(1) as f32, atlas.height().max(1) as f32);
    let meshes = atlas.meshes();
    let output = meshes.first().ok_or(UnwrapError::NoOutput)?;
    if output.index_array.len() != mesh.faces.len() * 3 {
        return Err(UnwrapError::NoOutput);
    }

    // UV por **canto** (não por vértice): o xatlas divide vértices nas costuras
    // entre charts; calcular a média por vértice colapsaria as costuras e
    // sobreporia charts. As UVs saem em texels do atlas, então normalizamos.
    let mut new_uv: Vec<Vec<[f32; 2]>> = Vec::with_capacity(mesh.faces.len());
    for tri in output.index_array.as_chunks::<3>().0 {
        let mut corners = Vec::with_capacity(3);
        for &oi in tri {
            let vert = output
                .vertex_array
                .get(oi as usize)
                .ok_or(UnwrapError::NoOutput)?;
            if !vert.uv.iter().all(|u| u.is_finite()) {
                return Err(UnwrapError::NoOutput);
            }
            corners.push([
                (vert.uv[0] / atlas_w).clamp(0.0, 1.0),
                (vert.uv[1] / atlas_h).clamp(0.0, 1.0),
            ]);
        }
        new_uv.push(corners);
    }
    let charts = output.chart_array.len();
    for (face, uv) in mesh.faces.iter_mut().zip(new_uv) {
        face.uv = uv;
    }
    Ok(charts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_unwraps_with_charts_and_finite_uvs() {
        let mut mesh = Mesh::cube(2.0);
        let charts = unwrap_fallback(&mut mesh).unwrap();
        assert!(charts >= 1);
        assert!(mesh.faces.iter().all(|f| f.verts.len() == 3));
        for face in &mesh.faces {
            assert_eq!(face.uv.len(), 3);
            for uv in &face.uv {
                assert!(uv.iter().all(|u| u.is_finite()));
            }
        }
    }

    #[test]
    fn empty_and_degenerate_inputs_are_rejected() {
        assert_eq!(
            unwrap_fallback(&mut Mesh::default()).unwrap_err(),
            UnwrapError::Empty
        );
        let mut bad = Mesh::cube(1.0);
        bad.verts[0].pos[0] = f32::NAN;
        assert_eq!(
            unwrap_fallback(&mut bad).unwrap_err(),
            UnwrapError::InvalidGeometry
        );
    }
}
