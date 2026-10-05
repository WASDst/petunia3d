//! Formato `.petunia`.
//!
//! Canonical V1 (ch. 16): ZIP container
//! ```text
//! project.petunia
//! ├ manifest.json
//! ├ document.json
//! ├ textures/
//! ├ references/
//! └ thumbnails/
//! ```
//!
//! Loader also accepts the legacy postcard blob (`PETUNIA\0` + version +
//! Project) and migrates it into the live Document. New saves always write
//! the ZIP container. `.pkg` remains a separate portable package.

use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};

use super::{
    AnimationAsset, AnnotationItem, Asset, Light, Material, MeasurementItem, Project, Skeleton,
    SplineResource,
};
use crate::io_atomic::atomic_write;

pub const PROJECT_VERSION: u32 = 1;
const MAGIC: &[u8; 8] = b"PETUNIA\0";
const ZIP_MAGIC: [u8; 2] = [0x50, 0x4B]; // "PK"
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 4096;
/// Limite do `document.json`. Igual ao limite total descomprimido: quem grava
/// e quem lê usam o mesmo teto, então nenhum save bem-sucedido é irrecuperável.
const MAX_JSON_BYTES: usize = MAX_UNCOMPRESSED_BYTES as usize;

#[derive(Debug, Serialize, Deserialize)]
struct PetuniaFile {
    magic: [u8; 8],
    version: u32,
    project: Project,
}

#[derive(Debug, Deserialize)]
struct LegacyPetuniaFileBeforeSplines {
    magic: [u8; 8],
    version: u32,
    project: LegacyProjectBeforeSplines,
}

#[derive(Debug, Deserialize)]
struct LegacyPetuniaFileWithSplines {
    magic: [u8; 8],
    version: u32,
    project: LegacyProjectWithSplines,
}

#[derive(Debug, Deserialize)]
struct LegacyProjectWithSplines {
    id: uuid::Uuid,
    name: String,
    assets: Vec<Asset>,
    active: usize,
    palette: Vec<[f32; 3]>,
    collections: Vec<String>,
    annotations: Vec<AnnotationItem>,
    annotation_groups: Vec<String>,
    measurements: Vec<MeasurementItem>,
    annotations_visible: bool,
    annotations_locked: bool,
    measurements_visible: bool,
    materials: Vec<Material>,
    lights: Vec<Light>,
    skeletons: Vec<Skeleton>,
    animations: Vec<AnimationAsset>,
    topology_revision: u64,
    position_revision: u64,
    selection_revision: u64,
    material_revision: u64,
    texture_revision: u64,
    transform_revision: u64,
    normal_revision: u64,
    uv_revision: u64,
    color_revision: u64,
    spline_revision: u64,
    splines: Vec<SplineResource>,
}

impl LegacyProjectWithSplines {
    fn into_project(self) -> Project {
        Project {
            id: self.id,
            name: self.name,
            assets: self.assets,
            active: self.active,
            history_selection: Vec::new(),
            palette: self.palette,
            collections: self.collections,
            annotations: self.annotations,
            annotation_groups: self.annotation_groups,
            measurements: self.measurements,
            annotations_visible: self.annotations_visible,
            annotations_locked: self.annotations_locked,
            measurements_visible: self.measurements_visible,
            materials: self.materials,
            lights: self.lights,
            skeletons: self.skeletons,
            animations: self.animations,
            topology_revision: self.topology_revision,
            position_revision: self.position_revision,
            selection_revision: self.selection_revision,
            material_revision: self.material_revision,
            texture_revision: self.texture_revision,
            transform_revision: self.transform_revision,
            normal_revision: self.normal_revision,
            uv_revision: self.uv_revision,
            color_revision: self.color_revision,
            spline_revision: self.spline_revision,
            splines: self.splines,
            procedural_revision: 0,
            profiles: Vec::new(),
            path_generators: Vec::new(),
            smooth_shaded_assets: Vec::new(),
            rig_roles: Vec::new(),
            ik_chains: Vec::new(),
            motions: Vec::new(),
            prefabs: Vec::new(),
            prefab_links: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LegacyProjectBeforeSplines {
    id: uuid::Uuid,
    name: String,
    assets: Vec<Asset>,
    active: usize,
    palette: Vec<[f32; 3]>,
    collections: Vec<String>,
    annotations: Vec<AnnotationItem>,
    annotation_groups: Vec<String>,
    measurements: Vec<MeasurementItem>,
    annotations_visible: bool,
    annotations_locked: bool,
    measurements_visible: bool,
    materials: Vec<Material>,
    lights: Vec<Light>,
    skeletons: Vec<Skeleton>,
    animations: Vec<AnimationAsset>,
    topology_revision: u64,
    position_revision: u64,
    selection_revision: u64,
    material_revision: u64,
    texture_revision: u64,
    transform_revision: u64,
    normal_revision: u64,
    uv_revision: u64,
    color_revision: u64,
}

impl LegacyProjectBeforeSplines {
    fn into_project(self) -> Project {
        Project {
            id: self.id,
            name: self.name,
            assets: self.assets,
            active: self.active,
            history_selection: Vec::new(),
            palette: self.palette,
            collections: self.collections,
            annotations: self.annotations,
            annotation_groups: self.annotation_groups,
            measurements: self.measurements,
            annotations_visible: self.annotations_visible,
            annotations_locked: self.annotations_locked,
            measurements_visible: self.measurements_visible,
            materials: self.materials,
            lights: self.lights,
            skeletons: self.skeletons,
            animations: self.animations,
            topology_revision: self.topology_revision,
            position_revision: self.position_revision,
            selection_revision: self.selection_revision,
            material_revision: self.material_revision,
            texture_revision: self.texture_revision,
            transform_revision: self.transform_revision,
            normal_revision: self.normal_revision,
            uv_revision: self.uv_revision,
            color_revision: self.color_revision,
            spline_revision: 0,
            splines: Vec::new(),
            procedural_revision: 0,
            profiles: Vec::new(),
            path_generators: Vec::new(),
            smooth_shaded_assets: Vec::new(),
            rig_roles: Vec::new(),
            ik_chains: Vec::new(),
            motions: Vec::new(),
            prefabs: Vec::new(),
            prefab_links: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestV1 {
    format: String,
    version: u32,
    name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("I/O: {0}")]
    Io(String),
    #[error("formato inválido: {0}")]
    Format(String),
    #[error("versão {0} não suportada (máx {1})")]
    Version(u32, u32),
    #[error("arquivo excede o limite de tamanho ({0} bytes)")]
    TooLarge(u64),
}

/// Salva o projeto no disco utilizando escrita atômica segura (P3D-001).
pub fn save(project: &Project, path: &std::path::Path) -> Result<(), ProjectError> {
    save_atomic(project, path)
}

/// Serializes `project` as a normalized ZIP V1 container (P3D-001).
pub fn encode_zip(project: &Project) -> Result<Vec<u8>, ProjectError> {
    let mut normalized = project.clone();
    normalized.validate();
    let project = &normalized;
    let manifest = ManifestV1 {
        format: "petunia".into(),
        version: PROJECT_VERSION,
        name: project.name.clone(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| ProjectError::Format(e.to_string()))?
        .into_bytes();
    let document_json =
        serde_json::to_vec(project).map_err(|e| ProjectError::Format(e.to_string()))?;
    if document_json.len() > MAX_JSON_BYTES {
        return Err(ProjectError::TooLarge(document_json.len() as u64));
    }

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("manifest.json", opts)
            .map_err(|e| ProjectError::Format(e.to_string()))?;
        zip.write_all(&manifest_json)
            .map_err(|e| ProjectError::Io(e.to_string()))?;
        zip.start_file("document.json", opts)
            .map_err(|e| ProjectError::Format(e.to_string()))?;
        zip.write_all(&document_json)
            .map_err(|e| ProjectError::Io(e.to_string()))?;
        zip.finish()
            .map_err(|e| ProjectError::Format(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

/// Salva o projeto de forma atômica e resiliente a falhas.
pub fn save_atomic(project: &Project, path: &std::path::Path) -> Result<(), ProjectError> {
    let bytes = encode_zip(project)?;
    atomic_write(path, &bytes).map_err(|e| ProjectError::Io(e.to_string()))
}

pub fn load(path: &std::path::Path) -> Result<Project, ProjectError> {
    let meta = std::fs::metadata(path).map_err(|e| ProjectError::Io(e.to_string()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(ProjectError::TooLarge(meta.len()));
    }
    let bytes = std::fs::read(path).map_err(|e| ProjectError::Io(e.to_string()))?;
    load_bytes(&bytes)
}

/// Parses a project from raw bytes (fuzz boundary: never panics on hostile input).
pub fn load_bytes(bytes: &[u8]) -> Result<Project, ProjectError> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ProjectError::TooLarge(bytes.len() as u64));
    }
    if bytes.len() >= 2 && bytes[0] == ZIP_MAGIC[0] && bytes[1] == ZIP_MAGIC[1] {
        return load_zip(bytes);
    }
    load_legacy_postcard(bytes)
}

fn load_zip(bytes: &[u8]) -> Result<Project, ProjectError> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| ProjectError::Format(e.to_string()))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(ProjectError::Format("too many zip entries".into()));
    }
    let mut uncompressed: u64 = 0;
    let mut document = None;
    let mut manifest_version = PROJECT_VERSION;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ProjectError::Format(e.to_string()))?;
        let name = entry.name().to_string();
        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
            return Err(ProjectError::Format(format!("path traversal: {name}")));
        }
        let size = entry.size();
        uncompressed = uncompressed.saturating_add(size);
        if uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(ProjectError::TooLarge(uncompressed));
        }
        if name == "manifest.json" {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| ProjectError::Format(e.to_string()))?;
            let manifest: ManifestV1 =
                serde_json::from_str(&buf).map_err(|e| ProjectError::Format(e.to_string()))?;
            if manifest.version > PROJECT_VERSION {
                return Err(ProjectError::Version(manifest.version, PROJECT_VERSION));
            }
            manifest_version = manifest.version;
        } else if name == "document.json" {
            if size as usize > MAX_JSON_BYTES {
                return Err(ProjectError::TooLarge(size));
            }
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| ProjectError::Format(e.to_string()))?;
            document = Some(buf);
        }
    }
    let _ = manifest_version;
    let doc = document.ok_or_else(|| ProjectError::Format("document.json ausente".into()))?;
    let mut project: Project =
        serde_json::from_slice(&doc).map_err(|e| ProjectError::Format(e.to_string()))?;
    project.validate();
    Ok(project)
}

fn load_legacy_postcard(bytes: &[u8]) -> Result<Project, ProjectError> {
    let file: PetuniaFile = match postcard::from_bytes(bytes) {
        Ok(file) => file,
        Err(current_error) => {
            if let Ok(legacy) = postcard::from_bytes::<LegacyPetuniaFileWithSplines>(bytes) {
                if legacy.magic != *MAGIC {
                    return Err(ProjectError::Format("magic inválido".into()));
                }
                if legacy.version > PROJECT_VERSION {
                    return Err(ProjectError::Version(legacy.version, PROJECT_VERSION));
                }
                let mut project = legacy.project.into_project();
                project.validate();
                return Ok(project);
            }
            let legacy: LegacyPetuniaFileBeforeSplines =
                postcard::from_bytes(bytes).map_err(|legacy_error| {
                    ProjectError::Format(format!("{current_error}; legacy layouts: {legacy_error}"))
                })?;
            if legacy.magic != *MAGIC {
                return Err(ProjectError::Format("magic inválido".into()));
            }
            if legacy.version > PROJECT_VERSION {
                return Err(ProjectError::Version(legacy.version, PROJECT_VERSION));
            }
            let mut project = legacy.project.into_project();
            project.validate();
            return Ok(project);
        }
    };
    if file.magic != *MAGIC {
        return Err(ProjectError::Format("magic inválido".into()));
    }
    if file.version > PROJECT_VERSION {
        return Err(ProjectError::Version(file.version, PROJECT_VERSION));
    }
    let mut project = file.project;
    project.validate();
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;
    use serde::ser::SerializeStruct;

    struct LegacyProjectV1<'a>(&'a Project);
    struct LegacyProjectWithSplinesV1<'a>(&'a Project);

    impl serde::Serialize for LegacyProjectV1<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let project = self.0;
            let mut state = serializer.serialize_struct("Project", 25)?;
            state.serialize_field("id", &project.id)?;
            state.serialize_field("name", &project.name)?;
            state.serialize_field("assets", &project.assets)?;
            state.serialize_field("active", &project.active)?;
            state.serialize_field("palette", &project.palette)?;
            state.serialize_field("collections", &project.collections)?;
            state.serialize_field("annotations", &project.annotations)?;
            state.serialize_field("annotation_groups", &project.annotation_groups)?;
            state.serialize_field("measurements", &project.measurements)?;
            state.serialize_field("annotations_visible", &project.annotations_visible)?;
            state.serialize_field("annotations_locked", &project.annotations_locked)?;
            state.serialize_field("measurements_visible", &project.measurements_visible)?;
            state.serialize_field("materials", &project.materials)?;
            state.serialize_field("lights", &project.lights)?;
            state.serialize_field("skeletons", &project.skeletons)?;
            state.serialize_field("animations", &project.animations)?;
            state.serialize_field("topology_revision", &project.topology_revision)?;
            state.serialize_field("position_revision", &project.position_revision)?;
            state.serialize_field("selection_revision", &project.selection_revision)?;
            state.serialize_field("material_revision", &project.material_revision)?;
            state.serialize_field("texture_revision", &project.texture_revision)?;
            state.serialize_field("transform_revision", &project.transform_revision)?;
            state.serialize_field("normal_revision", &project.normal_revision)?;
            state.serialize_field("uv_revision", &project.uv_revision)?;
            state.serialize_field("color_revision", &project.color_revision)?;
            state.end()
        }
    }

    impl serde::Serialize for LegacyProjectWithSplinesV1<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let project = self.0;
            let mut state = serializer.serialize_struct("Project", 27)?;
            state.serialize_field("id", &project.id)?;
            state.serialize_field("name", &project.name)?;
            state.serialize_field("assets", &project.assets)?;
            state.serialize_field("active", &project.active)?;
            state.serialize_field("palette", &project.palette)?;
            state.serialize_field("collections", &project.collections)?;
            state.serialize_field("annotations", &project.annotations)?;
            state.serialize_field("annotation_groups", &project.annotation_groups)?;
            state.serialize_field("measurements", &project.measurements)?;
            state.serialize_field("annotations_visible", &project.annotations_visible)?;
            state.serialize_field("annotations_locked", &project.annotations_locked)?;
            state.serialize_field("measurements_visible", &project.measurements_visible)?;
            state.serialize_field("materials", &project.materials)?;
            state.serialize_field("lights", &project.lights)?;
            state.serialize_field("skeletons", &project.skeletons)?;
            state.serialize_field("animations", &project.animations)?;
            state.serialize_field("topology_revision", &project.topology_revision)?;
            state.serialize_field("position_revision", &project.position_revision)?;
            state.serialize_field("selection_revision", &project.selection_revision)?;
            state.serialize_field("material_revision", &project.material_revision)?;
            state.serialize_field("texture_revision", &project.texture_revision)?;
            state.serialize_field("transform_revision", &project.transform_revision)?;
            state.serialize_field("normal_revision", &project.normal_revision)?;
            state.serialize_field("uv_revision", &project.uv_revision)?;
            state.serialize_field("color_revision", &project.color_revision)?;
            state.serialize_field("spline_revision", &project.spline_revision)?;
            state.serialize_field("splines", &project.splines)?;
            state.end()
        }
    }

    #[derive(serde::Serialize)]
    struct LegacyPetuniaFileV1<'a> {
        magic: [u8; 8],
        version: u32,
        project: LegacyProjectV1<'a>,
    }

    #[derive(serde::Serialize)]
    struct LegacyPetuniaFileWithSplinesV1<'a> {
        magic: [u8; 8],
        version: u32,
        project: LegacyProjectWithSplinesV1<'a>,
    }

    #[test]
    fn save_load_roundtrip() {
        let mut p = Project::new();
        p.add("Plane", Mesh::plane(1.0));
        let mut spline = crate::SplineResource::from_polyline(
            "Cable path",
            &[[0.0, 0.0, 0.0], [1.0, 0.5, 0.0], [2.0, 0.0, 0.0]],
            false,
        );
        let target = p.assets[1].id;
        let hit = crate::project_ray_to_surface_target(
            &p,
            target,
            [0.0, 2.0, 0.0],
            [0.0, -1.0, 0.0],
            10.0,
        )
        .unwrap()
        .unwrap();
        let point_id = spline.points[0].id;
        spline
            .set_attachment(
                point_id,
                Some(hit.attachment),
                hit.frame.position.map(f64::from),
            )
            .unwrap();
        let spline_id = spline.id;
        p.add_spline(spline).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("petunia_test_roundtrip.petunia");
        save(&p, &path).unwrap();
        let q = load(&path).unwrap();
        assert_eq!(q.assets.len(), 2);
        assert_eq!(q.assets[1].name, "Plane");
        assert_eq!(q.assets[1].id, p.assets[1].id);
        assert_eq!(q.splines.len(), 1);
        assert_eq!(q.splines[0].id, spline_id);
        assert_eq!(q.splines[0].points, p.splines[0].points);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..2], b"PK");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn compound_profiles_svg_sources_and_surface_paths_survive_zip() {
        let mut project = Project::new();
        let curve = crate::SplineResource::from_polyline(
            "Outline",
            &[
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ],
            true,
        );
        let mut profile =
            crate::ProfileResource::new("Hole", curve.id, crate::ProfileWorkplane::default());
        profile
            .holes
            .push(vec![[-0.2, -0.2], [-0.2, 0.2], [0.2, 0.2], [0.2, -0.2]]);
        project.add_spline(curve).unwrap();
        project.add_profile(profile.clone()).unwrap();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="red"/></svg>"#;
        let decal = crate::DecalLayer::from_svg(svg, 16, [0.5, 0.5], 0.2).unwrap();
        let mut stack = crate::PaintLayerStack::new();
        stack.add_layer(crate::PaintLayer::new_decal("SVG", decal));
        project.assets[0].paint_stack = Some(stack);
        let target = project.assets[0].id;
        let hit = crate::project_ray_to_surface_target(
            &project,
            target,
            [0.0, 0.0, 5.0],
            [0.0, 0.0, -1.0],
            10.0,
        )
        .unwrap()
        .unwrap();
        let mut path = crate::SplineResource::from_polyline(
            "Paint path",
            &[[0.0, 0.0, 0.5], [0.1, 0.0, 0.5]],
            false,
        );
        path.points[0].attachment = Some(hit.attachment);
        let path_id = path.id;
        project.add_spline(path).unwrap();
        let loaded = load_bytes(&encode_zip(&project).unwrap()).unwrap();
        assert_eq!(loaded.get_profile(profile.id).unwrap().holes, profile.holes);
        assert_eq!(
            loaded.get_spline(path_id).unwrap().points[0].attachment,
            Some(hit.attachment)
        );
        let layer = loaded.assets[0]
            .paint_stack
            .as_ref()
            .unwrap()
            .active()
            .unwrap();
        let crate::paint_layers::LayerKind::Decal(decal) = &layer.kind else {
            panic!("lost SVG decal");
        };
        assert_eq!(decal.source_svg.as_deref(), Some(svg));
        assert!(crate::evaluate_surface_attachment(&loaded, &hit.attachment).is_ok());
    }

    #[test]
    fn profile_and_path_generator_roundtrip() {
        let mut project = Project::new();
        let profile_spline = crate::SplineResource::from_polyline(
            "Square curve",
            &[
                [-0.5, -0.5, 0.0],
                [0.5, -0.5, 0.0],
                [0.5, 0.5, 0.0],
                [-0.5, 0.5, 0.0],
            ],
            true,
        );
        let profile = crate::ProfileResource::new(
            "Square",
            profile_spline.id,
            crate::ProfileWorkplane::default(),
        );
        let profile_id = profile.id;
        project.add_spline(profile_spline).unwrap();
        project.add_profile(profile).unwrap();
        let path = crate::SplineResource::from_polyline(
            "Guide",
            &[[0.0, 0.0, 0.0], [0.0, 0.0, 2.0]],
            false,
        );
        let path_id = path.id;
        project.add_spline(path).unwrap();
        let generator = crate::PathGenerator::sweep(
            "Square sweep",
            path_id,
            profile_id,
            crate::SweepGeneratorParameters::default(),
        );
        let generator_id = generator.id;
        project.add_path_generator(generator).unwrap();

        let bytes = encode_zip(&project).unwrap();
        let loaded = load_bytes(&bytes).unwrap();

        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.profiles[0].id, profile_id);
        assert_eq!(loaded.path_generators.len(), 1);
        assert_eq!(loaded.path_generators[0].id, generator_id);
        let mut cache = crate::PathGeneratorEvaluationCache::default();
        assert!(
            loaded
                .evaluate_path_generator(
                    generator_id,
                    crate::PathGeneratorQuality::Final,
                    &mut cache,
                )
                .is_ok()
        );
    }

    #[test]
    fn rejects_garbage() {
        let dir = std::env::temp_dir();
        let path = dir.join("petunia_test_garbage.petunia");
        std::fs::write(&path, b"lixo total").unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn hostile_project_normalized() {
        use super::super::{Asset, Canvas};
        use petunia_mesh::{Face, Mesh, Vertex};
        let bad_mesh = Mesh {
            verts: vec![Vertex::new(0.0, 0.0, 0.0), Vertex::new(1.0, 0.0, 0.0)],
            faces: vec![Face {
                verts: vec![0, 1, 77],
                uv: vec![[0.0, 0.0]],
                selected: false,
                material_slot: None,
            }],
            selected_edges: Default::default(),
            uv_seams: Default::default(),
            uv_pinned: Default::default(),
        };
        let bad = super::super::Project {
            id: uuid::Uuid::new_v4(),
            name: "bad".into(),
            assets: vec![Asset {
                id: uuid::Uuid::new_v4(),
                name: "bad".into(),
                mesh: bad_mesh.into(),
                visible: true,
                locked: false,
                collection: None,
                base_color: [f32::NAN, 0.0, 0.0],
                texture: Some(Canvas {
                    w: 0,
                    h: 999999,
                    pixels: vec![].into(),
                }),
                material_id: None,
                skeleton_id: None,
                skin_data: None,
                favorite: false,
                tags: vec![],
                modifiers: vec![],
                origin: None,
                paint_stack: None,
                eval_cache: None,
                parametric: None,
                selection_overlay_color: None,
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }],
            active: 42,
            palette: vec![],
            collections: vec![],
            ..Default::default()
        };
        let dir = std::env::temp_dir();
        let path = dir.join("petunia_test_hostile.petunia");
        save(&bad, &path).unwrap();
        let q = load(&path).unwrap();
        assert!(!q.assets.is_empty());
        assert!(q.active < q.assets.len());
        let a = &q.assets[0];
        assert!(a.mesh.faces.iter().all(|f| f.uv.len() == f.verts.len()));
        assert!(a.base_color.iter().all(|x| x.is_finite()));
        if let Some(cv) = &a.texture {
            assert_eq!(cv.pixels.len(), (cv.w * cv.h * 4) as usize);
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_future_version_rejected() {
        let manifest = serde_json::json!({
            "format": "petunia",
            "version": 999,
            "name": "future"
        });
        let document = serde_json::to_vec(&Project::new()).unwrap();
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("manifest.json", opts).unwrap();
            zip.write_all(manifest.to_string().as_bytes()).unwrap();
            zip.start_file("document.json", opts).unwrap();
            zip.write_all(&document).unwrap();
            zip.finish().unwrap();
        }
        let bytes = cursor.into_inner();
        let res = load_bytes(&bytes);
        assert!(matches!(res, Err(ProjectError::Version(999, 1))));
    }

    #[test]
    fn legacy_postcard_still_loads() {
        let p = Project::new();
        let file = PetuniaFile {
            magic: *MAGIC,
            version: 1,
            project: p.clone(),
        };
        let bytes = postcard::to_allocvec(&file).unwrap();
        let loaded = load_bytes(&bytes).unwrap();
        assert_eq!(loaded.assets.len(), p.assets.len());
        assert_eq!(loaded.assets[0].name, "Cube");
    }

    #[test]
    fn postcard_layout_before_splines_loads_with_additive_defaults() {
        let project = Project::new();
        let bytes = postcard::to_allocvec(&LegacyPetuniaFileV1 {
            magic: *MAGIC,
            version: 1,
            project: LegacyProjectV1(&project),
        })
        .unwrap();

        let loaded = load_bytes(&bytes).unwrap();

        assert_eq!(loaded.assets.len(), project.assets.len());
        assert_eq!(loaded.assets[0].id, project.assets[0].id);
        assert_eq!(loaded.assets[0].name, project.assets[0].name);
        assert!(loaded.splines.is_empty());
        assert_eq!(loaded.spline_revision, 0);
    }

    #[test]
    fn postcard_layout_with_splines_loads_with_procedural_defaults() {
        let mut project = Project::new();
        let spline = crate::SplineResource::from_polyline(
            "Legacy guide",
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            false,
        );
        let spline_id = spline.id;
        project.add_spline(spline).unwrap();
        project.spline_revision = 7;
        let bytes = postcard::to_allocvec(&LegacyPetuniaFileWithSplinesV1 {
            magic: *MAGIC,
            version: 1,
            project: LegacyProjectWithSplinesV1(&project),
        })
        .unwrap();

        let loaded = load_bytes(&bytes).unwrap();

        assert_eq!(loaded.splines.len(), 1);
        assert_eq!(loaded.splines[0].id, spline_id);
        assert_eq!(loaded.spline_revision, 7);
        assert!(loaded.profiles.is_empty());
        assert!(loaded.path_generators.is_empty());
        assert_eq!(loaded.procedural_revision, 0);
    }

    #[test]
    fn document_without_spline_fields_defaults_to_empty_storage() {
        let mut document = serde_json::to_value(Project::new()).unwrap();
        let object = document.as_object_mut().unwrap();
        object.remove("splines");
        object.remove("spline_revision");
        object.remove("profiles");
        object.remove("path_generators");
        object.remove("procedural_revision");

        let loaded: Project = serde_json::from_value(document).unwrap();

        assert!(loaded.splines.is_empty());
        assert_eq!(loaded.spline_revision, 0);
        assert!(loaded.profiles.is_empty());
        assert!(loaded.path_generators.is_empty());
        assert_eq!(loaded.procedural_revision, 0);
    }

    #[test]
    fn prefab_library_roundtrips_and_old_documents_default_to_empty() {
        let mut p = Project::new();
        let cube_id = p.assets[0].id;
        let prefab_id = p.create_prefab(&[cube_id], Some("Crate")).unwrap();
        assert!(p.set_prefab_favorite(prefab_id, true));
        let instance = p.instantiate_prefab(prefab_id, [3.0, 0.0, 0.0])[0];

        let path = std::env::temp_dir().join("petunia_test_prefab_roundtrip.petunia");
        save(&p, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.prefabs.len(), 1);
        assert_eq!(loaded.prefabs[0].name, "Crate");
        assert!(loaded.prefabs[0].favorite);
        assert_eq!(loaded.prefab_instance_count(prefab_id), 1);
        assert!(loaded.assets.iter().any(|a| a.id == instance));
        let _ = std::fs::remove_file(&path);

        let mut document = serde_json::to_value(&p).unwrap();
        let object = document.as_object_mut().unwrap();
        object.remove("prefabs");
        object.remove("prefab_links");
        let old: Project = serde_json::from_value(document).unwrap();
        assert!(old.prefabs.is_empty() && old.prefab_links.is_empty());
    }

    #[test]
    fn test_project_metadata_and_asset_tags_roundtrip() {
        let mut p = Project::new();
        p.name = "My Adventure".to_string();
        let mut cube_asset = Asset::new("Hero", Mesh::cube(1.5));
        cube_asset.favorite = true;
        assert!(cube_asset.add_tag("character"));
        assert!(cube_asset.add_tag("protagonist"));
        let hero_id = cube_asset.id;
        p.assets.push(cube_asset);

        let dir = std::env::temp_dir();
        let path = dir.join("petunia_test_tags_roundtrip.petunia");
        save(&p, &path).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.id, p.id);
        assert_eq!(loaded.name, "My Adventure");
        let hero = loaded.assets.iter().find(|a| a.id == hero_id).unwrap();
        assert!(hero.favorite);
        assert!(hero.has_tag("character"));
        assert!(hero.has_tag("protagonist"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_atomic_save_preserves_original_on_failure() {
        let mut original = Project::new();
        original.name = "Original Intact".to_string();
        let dir =
            std::env::temp_dir().join(format!("petunia_atomic_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("project.petunia");

        save(&original, &path).unwrap();
        let valid_bytes = std::fs::read(&path).unwrap();

        let invalid_path = path.join("sub_project.petunia");
        let mut corrupt_attempt = Project::new();
        corrupt_attempt.name = "Corrupt".to_string();
        let err = save(&corrupt_attempt, &invalid_path);
        assert!(err.is_err(), "Deve falhar ao tentar salvar sob um arquivo");

        let current_bytes = std::fs::read(&path).unwrap();
        assert_eq!(current_bytes, valid_bytes);
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.name, "Original Intact");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn canvas_pixels_serialize_as_compact_base64() {
        let canvas = crate::Canvas::new(8, 8, [1, 2, 3, 4]);
        let json = serde_json::to_string(&canvas).unwrap();
        assert!(
            json.contains("\"pixels\":\""),
            "esperado string base64: {json}"
        );
        assert!(
            json.len() < 8 * 8 * 4 * 2,
            "JSON deve ser compacto: {} bytes",
            json.len()
        );
        let back: crate::Canvas = serde_json::from_str(&json).unwrap();
        assert_eq!(back, canvas);
    }

    #[test]
    fn canvas_pixels_legacy_number_array_still_loads() {
        let json = r#"{"w":2,"h":1,"pixels":[1,2,3,4,5,6,7,8]}"#;
        let canvas: crate::Canvas = serde_json::from_str(json).unwrap();
        assert_eq!(canvas.pixels, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn canvas_pixels_postcard_layout_is_unchanged() {
        let canvas = crate::Canvas::new(4, 4, [9, 8, 7, 6]);
        let bytes = postcard::to_allocvec(&canvas).unwrap();
        let back: crate::Canvas = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, canvas);
        let mut legacy = Vec::new();
        legacy.extend(postcard::to_allocvec(&canvas.w).unwrap());
        legacy.extend(postcard::to_allocvec(&canvas.h).unwrap());
        legacy.extend(postcard::to_allocvec(&canvas.pixels).unwrap());
        assert_eq!(bytes, legacy);
    }

    #[test]
    fn saved_project_with_large_textures_reloads() {
        // Regressão D-01: o save aceitava até 256 MiB de JSON, mas o loader
        // recusava mais de 32 MiB. Um projeto de várias texturas grandes
        // precisa sobreviver a save -> load.
        let mut p = Project::new();
        p.add("Plane", Mesh::plane(1.0));
        let mut noise = crate::Canvas::new(1024, 1024, [0, 0, 0, 255]);
        for (i, byte) in noise.pixels.iter_mut().enumerate() {
            *byte = (i.wrapping_mul(2654435761) >> 13) as u8;
        }
        for asset in p.assets.iter_mut().skip(1).take(1) {
            asset.texture = Some(noise.clone());
        }
        let bytes = encode_zip(&p).unwrap();
        let back = load_bytes(&bytes).unwrap();
        assert_eq!(
            back.assets[1].texture.as_ref().unwrap().pixels,
            noise.pixels
        );
    }

    #[test]
    fn oversized_input_is_rejected() {
        let huge = vec![0u8; (MAX_FILE_BYTES as usize) + 1];
        assert!(matches!(load_bytes(&huge), Err(ProjectError::TooLarge(_))));
    }
}
