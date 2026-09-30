//! glTF JSON boundary (`gltf-json`, P0-05).
//!
//! glTF *export* already produces binary GLB in [`crate::export`]. This module
//! owns the JSON side of the contract: parsing and validating the glTF JSON
//! chunk of external files, headless and independent from file dialogs and UI.
//! Buffer/attribute decoding stays behind the same boundary as the format
//! matures; today the boundary guarantees structural validation plus a
//! [`GltfSummary`] that importers and tests can rely on.

/// Structural summary of a validated glTF JSON document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GltfSummary {
    /// `asset.version`, e.g. `"2.0"`.
    pub version: String,
    /// Number of scenes.
    pub scenes: usize,
    /// Number of nodes.
    pub nodes: usize,
    /// Number of meshes.
    pub meshes: usize,
    /// Number of buffers (external or GLB chunk references).
    pub buffers: usize,
}

/// glTF JSON import failure with a stable message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GltfImportError {
    /// The document is not valid JSON / glTF.
    #[error("glTF JSON parse error: {0}")]
    Parse(String),
    /// The document parses but violates structural expectations.
    #[error("glTF validation error: {0}")]
    Validation(String),
}

/// Skin/animação de um GLB (cap. 45 F0). Ver [`crate::gltf_rig::import_rig`].
pub use crate::gltf_rig::import_rig;
use gltf_json::validation::Validate;
use petunia_mesh::{Face, Mesh, Vertex};

/// Imported GLB meshes (name, mesh).
pub type GlbMeshes = Vec<(String, Mesh)>;

/// Imports a GLB (or glTF with embedded buffers) into meshes.
pub fn import_glb_bytes(
    data: &[u8],
    name_hint: &str,
    triangulate: bool,
    scale: f32,
) -> Result<GlbMeshes, GltfImportError> {
    let gltf = gltf::Gltf::from_slice(data).map_err(|e| GltfImportError::Parse(e.to_string()))?;
    let blob = gltf.blob.as_deref();
    let mut meshes = Vec::new();
    for (mi, mesh) in gltf.meshes().enumerate() {
        let mut out = Mesh::default();
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| if buffer.index() == 0 { blob } else { None });
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let base = out.verts.len() as u32;
            for pos in positions {
                let mut v = Vertex::new(pos[0] * scale, pos[1] * scale, pos[2] * scale);
                if !v.pos.iter().all(|x| x.is_finite()) {
                    v.pos = [0.0, 0.0, 0.0];
                }
                out.verts.push(v);
            }
            if let Some(indices) = reader.read_indices() {
                let idx: Vec<u32> = indices.into_u32().collect();
                for tri in idx.chunks(3) {
                    if tri.len() == 3 {
                        out.push_face(Face::new(vec![base + tri[0], base + tri[1], base + tri[2]]));
                    }
                }
            }
        }
        if triangulate {
            out.triangulate();
        }
        out.validate();
        if !out.verts.is_empty() {
            let name = mesh
                .name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{name_hint}_{mi}"));
            meshes.push((name, out));
        }
    }
    if meshes.is_empty() {
        return Err(GltfImportError::Validation(
            "GLB does not contain mesh primitives".into(),
        ));
    }
    Ok(meshes)
}

/// Parses and structurally validates a glTF JSON document.
///
/// Rejects documents without an `asset.version`, with an unsupported major
/// version, or with inconsistent mesh/buffer counts. Returns a [`GltfSummary`]
/// for the caller to drive buffer decoding and scene construction.
pub fn parse_gltf_json(data: &[u8]) -> Result<GltfSummary, GltfImportError> {
    let text = std::str::from_utf8(data).map_err(|e| GltfImportError::Parse(e.to_string()))?;
    let root: gltf_json::Root =
        serde_json::from_str(text).map_err(|e| GltfImportError::Parse(e.to_string()))?;
    let mut failures: Vec<String> = Vec::new();
    root.validate(&root, gltf_json::Path::new, &mut |path, error| {
        failures.push(format!("{}: {error:?}", path().as_str()));
    });
    if let Some(first) = failures.into_iter().next() {
        return Err(GltfImportError::Validation(first));
    }

    let version = root.asset.version.clone();
    let major = version
        .split('.')
        .next()
        .and_then(|n| n.parse::<u32>().ok())
        .ok_or_else(|| GltfImportError::Validation(format!("bad asset.version '{version}'")))?;
    if major != 2 {
        return Err(GltfImportError::Validation(format!(
            "unsupported glTF major version {major}"
        )));
    }
    Ok(GltfSummary {
        version,
        scenes: root.scenes.len(),
        nodes: root.nodes.len(),
        meshes: root.meshes.len(),
        buffers: root.buffers.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_GLTF: &str = r#"{
        "asset": { "version": "2.0" },
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
        "buffers": [{ "byteLength": 36 }],
        "bufferViews": [{ "buffer": 0, "byteLength": 36 }],
        "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 1.0] }]
    }"#;

    #[test]
    fn valid_gltf_summarizes_structure() {
        let summary = parse_gltf_json(MINIMAL_GLTF.as_bytes()).unwrap();
        assert_eq!(summary.version, "2.0");
        assert_eq!(summary.scenes, 1);
        assert_eq!(summary.nodes, 1);
        assert_eq!(summary.meshes, 1);
        assert_eq!(summary.buffers, 1);
    }

    #[test]
    fn malformed_json_is_a_parse_error() {
        let err = parse_gltf_json(b"{ not json").unwrap_err();
        assert!(matches!(err, GltfImportError::Parse(_)));
    }

    #[test]
    fn missing_asset_is_rejected() {
        let err = parse_gltf_json(b"{}").unwrap_err();
        assert!(matches!(
            err,
            GltfImportError::Parse(_) | GltfImportError::Validation(_)
        ));
    }

    #[test]
    fn unsupported_major_version_is_rejected() {
        let doc = MINIMAL_GLTF.replace("\"2.0\"", "\"1.0\"");
        assert_eq!(
            parse_gltf_json(doc.as_bytes()).unwrap_err(),
            GltfImportError::Validation("unsupported glTF major version 1".to_string())
        );
    }
}
