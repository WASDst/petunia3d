//! OBJ import behind the headless Importer contract (`tobj`, P0-06).
//!
//! Petunia-controlled OBJ *writing* stays in `petunia_mesh` (deterministic,
//! dependency-free). OBJ *reading* goes through `tobj` here so malformed
//! external files get validated once, at this boundary, before touching the
//! domain: finite coordinates, sane sizes, triangulated faces.

use petunia_mesh::{Face, Mesh, Vertex};

/// Limits guarding against hostile files (zip-bomb class).
const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;

/// Import failure with a stable, user-presentable message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ObjImportError {
    /// The file could not be parsed as OBJ.
    #[error("OBJ parse error: {0}")]
    Parse(String),
    /// The file parsed but violates domain invariants.
    #[error("OBJ validation error: {0}")]
    Validation(String),
    /// The file exceeds [`MAX_VERTICES`] / [`MAX_FACES`].
    #[error("OBJ exceeds size limits")]
    TooLarge,
}

/// Imports the first model of an OBJ document into a Petunia [`Mesh`].
///
/// Faces are triangulated by the loader; quads are therefore not preserved on
/// this path (documented, not silent: triangulation is inherent to `tobj`).
pub fn import_obj_bytes(data: &[u8]) -> Result<Mesh, ObjImportError> {
    let mut reader = std::io::BufReader::new(data);
    let options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };
    let (models, _) = tobj::load_obj_buf(&mut reader, &options, |_| {
        Ok((Vec::new(), Default::default()))
    })
    .map_err(|e| ObjImportError::Parse(e.to_string()))?;
    let model = models
        .into_iter()
        .next()
        .ok_or_else(|| ObjImportError::Validation("OBJ contains no models".to_string()))?;
    mesh_from_tobj(&model.mesh)
}

fn mesh_from_tobj(mesh: &tobj::Mesh) -> Result<Mesh, ObjImportError> {
    if mesh.positions.len() / 3 > MAX_VERTICES || mesh.indices.len() / 3 > MAX_FACES {
        return Err(ObjImportError::TooLarge);
    }
    if !mesh.positions.len().is_multiple_of(3) {
        return Err(ObjImportError::Validation(
            "position buffer is not a multiple of 3".to_string(),
        ));
    }
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(ObjImportError::Validation(
            "index buffer is not a multiple of 3".to_string(),
        ));
    }
    let mut out = Mesh::default();
    let (vert_chunks, _) = mesh.positions.as_chunks::<3>();
    for chunk in vert_chunks {
        let [x, y, z] = *chunk;
        if ![x, y, z].iter().all(|v| v.is_finite()) {
            return Err(ObjImportError::Validation(
                "non-finite vertex position".to_string(),
            ));
        }
        out.verts.push(Vertex {
            pos: [x, y, z],
            color: [0.75, 0.75, 0.78],
            selected: false,
        });
    }
    if out.verts.is_empty() {
        return Err(ObjImportError::Validation(
            "OBJ contains no vertices".to_string(),
        ));
    }
    let count = out.verts.len() as u32;
    let (tri_chunks, _) = mesh.indices.as_chunks::<3>();
    for tri in tri_chunks {
        let [a, b, c] = *tri;
        if a >= count || b >= count || c >= count {
            return Err(ObjImportError::Validation(
                "face index out of bounds".to_string(),
            ));
        }
        let mut face = Face::new(vec![a, b, c]);
        face.uv = vec![[0.0, 0.0]; 3];
        out.faces.push(face);
    }
    if out.faces.is_empty() {
        return Err(ObjImportError::Validation(
            "OBJ contains no faces".to_string(),
        ));
    }
    petunia_mesh::validate_invariants(&out)
        .map_err(|e| ObjImportError::Validation(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBE_OBJ: &str = "o Cube\n\
        v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\n\
        v -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\n\
        f 1 2 3\nf 1 3 4\nf 5 6 7\nf 5 7 8\n";

    #[test]
    fn valid_obj_imports_triangulated_mesh() {
        let mesh = import_obj_bytes(CUBE_OBJ.as_bytes()).unwrap();
        assert_eq!(mesh.verts.len(), 8);
        assert_eq!(mesh.faces.len(), 4);
        assert!(mesh.faces.iter().all(|f| f.verts.len() == 3));
    }

    #[test]
    fn malformed_obj_is_a_parse_error() {
        let err = import_obj_bytes(b"this is not an obj \x00\x01").unwrap_err();
        assert!(matches!(
            err,
            ObjImportError::Parse(_) | ObjImportError::Validation(_)
        ));
    }

    #[test]
    fn empty_obj_is_a_validation_error() {
        assert_eq!(
            import_obj_bytes(b"o Empty\n").unwrap_err(),
            ObjImportError::Validation("OBJ contains no vertices".to_string())
        );
    }

    #[test]
    fn quad_is_triangulated_not_preserved() {
        let quad = "o Q\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n";
        let mesh = import_obj_bytes(quad.as_bytes()).unwrap();
        assert_eq!(mesh.faces.len(), 2);
    }
}
