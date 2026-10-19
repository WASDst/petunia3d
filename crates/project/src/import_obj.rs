//! OBJ import behind the headless Importer contract (`tobj`, P0-06).
//!
//! Petunia-controlled OBJ *writing* stays in `petunia_mesh` (deterministic,
//! dependency-free). OBJ *reading* goes through `tobj` here so malformed
//! external files get validated once, at this boundary, before touching the
//! domain: finite coordinates, sane sizes, triangulated faces.
//!
//! The `.mtl` referenced by `mtllib` is read too: without it the model
//! arrives naked (no color, no texture), which is the most common complaint
//! about importers. Material files and texture paths coming from the file are
//! treated as untrusted: they may not escape the model's directory.

use petunia_mesh::{Face, Mesh, Vertex};
use std::cell::RefCell;
use std::path::Path;

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

/// One object of the file (`o`/`g`), with the material it uses.
#[derive(Debug, Clone)]
pub struct ObjObject {
    /// Name from the file (or a generated one).
    pub name: String,
    pub mesh: Mesh,
    /// Index of the object's material in [`ObjImport::materials`].
    pub material: Option<usize>,
}

/// Result of an OBJ import: objects, materials and warnings.
#[derive(Debug, Clone, Default)]
pub struct ObjImport {
    /// One entry per object/material group of the file.
    pub objects: Vec<ObjObject>,
    /// Materials declared in the `.mtl` (index space shared with the faces).
    pub materials: Vec<crate::Material>,
    /// Non-fatal problems (missing `.mtl`, unreadable texture, …).
    pub warnings: Vec<String>,
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

/// Imports an OBJ with its materials: every object of the file, the `.mtl`
/// materials and the textures they reference.
///
/// `base_dir` is the directory of the OBJ file: the `.mtl` and the texture
/// files are looked up there (never outside it).
pub fn import_obj_file(data: &[u8], base_dir: Option<&Path>) -> Result<ObjImport, ObjImportError> {
    let mut reader = std::io::BufReader::new(data);
    let options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };
    let warnings: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let (models, tobj_materials) = tobj::load_obj_buf(&mut reader, &options, |mtl_path| {
        // `mtllib` may point to several files; a missing one is a warning,
        // not a reason to lose the geometry.
        let resolved = match base_dir {
            Some(base) => {
                match crate::external_path::resolve_external_path(base, &mtl_path.to_string_lossy())
                {
                    Ok(path) => path,
                    Err(reason) => {
                        warnings
                            .borrow_mut()
                            .push(format!("material '{}': {reason}", mtl_path.display()));
                        return Ok((Vec::new(), Default::default()));
                    }
                }
            }
            None => mtl_path.to_path_buf(),
        };
        match tobj::load_mtl(&resolved) {
            Ok(pair) => Ok(pair),
            Err(error) => {
                warnings
                    .borrow_mut()
                    .push(format!("material '{}': {error}", mtl_path.display()));
                Ok((Vec::new(), Default::default()))
            }
        }
    })
    .map_err(|e| ObjImportError::Parse(e.to_string()))?;

    let tobj_materials = tobj_materials.map_err(|e| ObjImportError::Parse(e.to_string()))?;
    let mut materials = Vec::with_capacity(tobj_materials.len());
    for material in &tobj_materials {
        materials.push(material_from_tobj(
            material,
            base_dir,
            &mut warnings.borrow_mut(),
        ));
    }

    let mut objects = Vec::with_capacity(models.len());
    for model in models {
        let material = model.mesh.material_id;
        let mesh = mesh_from_tobj(&model.mesh)?;
        let name = if model.name.trim().is_empty() {
            format!("object_{}", objects.len() + 1)
        } else {
            model.name.clone()
        };
        objects.push(ObjObject {
            name,
            mesh,
            material,
        });
    }
    if objects.is_empty() {
        return Err(ObjImportError::Validation(
            "OBJ contains no models".to_string(),
        ));
    }
    Ok(ObjImport {
        objects,
        materials,
        warnings: warnings.into_inner(),
    })
}

/// Converts a `.mtl` material into a Petunia material, decoding its textures.
///
/// `Kd` becomes the base color, `Ns` the roughness (`sqrt(2/(Ns+2))`, the
/// usual conversion) and `map_Kd`/`map_Bump` the albedo/normal textures.
fn material_from_tobj(
    material: &tobj::Material,
    base_dir: Option<&Path>,
    warnings: &mut Vec<String>,
) -> crate::Material {
    let mut out = crate::Material::new(material.name.clone());
    if let Some(diffuse) = material.diffuse {
        let alpha = material.dissolve.unwrap_or(1.0).clamp(0.0, 1.0);
        out.base_color = [diffuse[0], diffuse[1], diffuse[2], alpha];
        if alpha < 1.0 {
            out.alpha_mode = crate::AlphaMode::Blend;
        }
    }
    if let Some(shininess) = material.shininess
        && shininess.is_finite()
        && shininess >= 0.0
    {
        // Conversão usual de Ns para rugosidade (0 = espelhado, 1000 = fosco).
        out.roughness = (2.0 / (shininess + 2.0)).sqrt().clamp(0.04, 1.0);
    }
    if let Some(emissive) = material.emissive {
        out.emission_color = [emissive[0], emissive[1], emissive[2]];
        out.emission_strength = emissive
            .iter()
            .map(|c| c.clamp(0.0, 1.0))
            .fold(0.0f32, f32::max);
    }
    if let Some(texture) = material.diffuse_texture.as_deref() {
        match load_texture(base_dir, texture) {
            Ok(canvas) => out.albedo_texture = Some(canvas),
            Err(reason) => warnings.push(format!("map_Kd '{}': {reason}", texture)),
        }
    }
    if let Some(texture) = material.normal_texture.as_deref() {
        match load_texture(base_dir, texture) {
            Ok(canvas) => out.normal_texture = Some(canvas),
            Err(reason) => warnings.push(format!("map_Bump '{}': {reason}", texture)),
        }
    }
    out.validate();
    out
}

/// Reads a texture referenced by the `.mtl`, refusing paths that leave the
/// model's directory.
fn load_texture(base_dir: Option<&Path>, texture: &str) -> Result<crate::Canvas, String> {
    let Some(base) = base_dir else {
        return Err("textura externa exige o diretório do arquivo".to_string());
    };
    let path = crate::external_path::resolve_external_path(base, texture)?;
    crate::image_decode::decode_image_file(&path)
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

    for uv in &mesh.texcoords {
        if !uv.is_finite() {
            return Err(ObjImportError::Validation(
                "non-finite texture coordinate in OBJ".to_string(),
            ));
        }
    }
    for n in &mesh.normals {
        if !n.is_finite() {
            return Err(ObjImportError::Validation(
                "non-finite normal in OBJ".to_string(),
            ));
        }
    }

    let uv_of = |idx: u32| -> [f32; 2] {
        let i = idx as usize;
        if i * 2 + 1 < mesh.texcoords.len() {
            let u = mesh.texcoords[i * 2];
            let v = mesh.texcoords[i * 2 + 1];
            if u.is_finite() && v.is_finite() {
                return [u, v];
            }
        }
        [0.0, 0.0]
    };

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
        face.uv = vec![uv_of(a), uv_of(b), uv_of(c)];
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
