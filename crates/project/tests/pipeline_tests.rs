//! Testes exaustivos do Pipeline de Entrega, Importação e Exportação Modular (Wave 8 — P3D-068 a P3D-072, P3D-124).

use std::fs;

use petunia_mesh::Mesh;
use petunia_project::{
    DeliveryPipeline, ExportOptions, FileFormat, ImportOptions, Material, PipelineError, Project,
};
use tempfile::tempdir;

fn sample_test_project() -> Project {
    let mut proj = Project::default();
    proj.add("Cube Asset", Mesh::cube(1.0));
    proj.add("Plane Asset", Mesh::plane(2.0));
    let mut mat = Material::new("Gold PBR");
    mat.base_color = [1.0, 0.84, 0.0, 1.0];
    mat.metallic = 1.0;
    mat.roughness = 0.1;
    mat.emission_color = [0.1, 0.1, 0.0];
    mat.emission_strength = 2.0;
    let mat_id = proj.add_material(mat);
    proj.assets[0].material_id = Some(mat_id);
    proj
}

// -----------------------------------------------------------------------------
// P3D-068: EXPORT INDIVIDUAL COM VALIDAÇÃO E RELATÓRIO
// -----------------------------------------------------------------------------

#[test]
fn test_export_single_asset_obj() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let out_path = dir.path().join("single_cube.obj");
    let options = ExportOptions {
        triangulate: true,
        export_materials: false,
        scale: 2.0,
        overwrite: true,
    };

    let report = pipeline
        .export_single_asset(&proj, 0, &out_path, &options)
        .expect("export obj");

    assert_eq!(report.asset_name, "Cube Asset");
    assert_eq!(report.output_path, out_path);
    assert_eq!(report.format, FileFormat::Obj);
    assert!(report.bytes_written > 0);
    assert!(out_path.exists());

    let content = fs::read_to_string(&out_path).expect("read obj");
    assert!(content.contains("v "));
    assert!(content.contains("f "));
}

#[test]
fn test_export_single_asset_glb_with_materials() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let out_path = dir.path().join("single_cube.glb");
    let options = ExportOptions::default();

    let report = pipeline
        .export_single_asset(&proj, 0, &out_path, &options)
        .expect("export glb");

    assert_eq!(report.format, FileFormat::Glb);
    assert!(report.bytes_written > 0);
    assert!(out_path.exists());

    let bytes = fs::read(&out_path).expect("read glb");
    assert_eq!(&bytes[0..4], b"glTF");

    // Valida com o crate gltf oficial
    let (doc, buffers, _) = gltf::import_slice(&bytes).expect("parse glb");
    assert_eq!(doc.meshes().len(), 1);
    assert_eq!(buffers.len(), 1);

    // Valida material PBR emitido
    let mat = doc.materials().next().expect("material exists");
    let pbr = mat.pbr_metallic_roughness();
    assert!((pbr.metallic_factor() - 1.0).abs() < 1e-4);
    assert!((pbr.roughness_factor() - 0.1).abs() < 1e-4);
    let base_color = pbr.base_color_factor();
    assert!((base_color[0] - 1.0).abs() < 1e-4);
    assert!((base_color[1] - 0.84).abs() < 1e-3);
}

#[test]
fn test_export_single_asset_pkg() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let out_path = dir.path().join("single_cube.pkg");
    let options = ExportOptions::default();

    let report = pipeline
        .export_single_asset(&proj, 0, &out_path, &options)
        .expect("export pkg");

    assert_eq!(report.format, FileFormat::Pkg);
    assert!(out_path.exists());
    assert!(report.bytes_written > 0);
}

#[test]
fn test_export_single_asset_overwrite_policy() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let out_path = dir.path().join("file.obj");
    fs::write(&out_path, "pre-existing").expect("write");

    let no_overwrite = ExportOptions {
        overwrite: false,
        ..Default::default()
    };

    let err = pipeline
        .export_single_asset(&proj, 0, &out_path, &no_overwrite)
        .expect_err("should reject overwrite");

    match err {
        PipelineError::AlreadyExists(p) => assert_eq!(p, out_path),
        other => panic!("expected AlreadyExists, got {other:?}"),
    }

    let allow_overwrite = ExportOptions {
        overwrite: true,
        ..Default::default()
    };
    pipeline
        .export_single_asset(&proj, 0, &out_path, &allow_overwrite)
        .expect("should allow overwrite");
}

// -----------------------------------------------------------------------------
// P3D-069 & P3D-070: MULTI-EXPORT & BATCH EXPORT
// -----------------------------------------------------------------------------

#[test]
fn test_export_multiple_assets_obj() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let target_dir = dir.path().join("multi_out");
    let options = ExportOptions::default();

    let report = pipeline
        .export_multiple_assets(&proj, &[0, 1], &target_dir, FileFormat::Obj, &options)
        .expect("multi export");

    assert!(report.all_succeeded());
    assert_eq!(report.succeeded.len(), 2);
    assert_eq!(report.failed.len(), 0);
    assert!(report.total_bytes > 0);

    assert!(target_dir.join("Cube_Asset.obj").exists());
    assert!(target_dir.join("Plane_Asset.obj").exists());
}

#[test]
fn test_batch_export_entire_project_glb() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let target_dir = dir.path().join("batch_out");
    let options = ExportOptions::default();

    let report = pipeline
        .batch_export(&proj, &target_dir, FileFormat::Glb, &options)
        .expect("batch export");

    assert!(report.all_succeeded());
    assert_eq!(report.succeeded.len(), 2);
    assert!(target_dir.join("Cube_Asset.glb").exists());
    assert!(target_dir.join("Plane_Asset.glb").exists());
}

#[test]
fn test_batch_export_partial_failure_tolerance() {
    let dir = tempdir().expect("tempdir");
    let mut proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    // Adiciona asset corrupto com coordenadas NaN
    let mut corrupted_mesh = Mesh::plane(1.0);
    corrupted_mesh.verts[0].pos = [f32::NAN, 0.0, 0.0];
    proj.add("Corrupted Asset", corrupted_mesh);

    let target_dir = dir.path().join("resilient_out");
    let options = ExportOptions::default();

    // Exporta todos os 3 assets (0=Cube, 1=Plane, 2=Corrupted)
    let report = pipeline
        .batch_export(&proj, &target_dir, FileFormat::Glb, &options)
        .expect("batch export");

    assert!(!report.all_succeeded());
    assert_eq!(report.succeeded.len(), 2);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].0, "Corrupted Asset");

    // Os dois assets válidos continuam tendo sido exportados com sucesso!
    assert!(target_dir.join("Cube_Asset.glb").exists());
    assert!(target_dir.join("Plane_Asset.glb").exists());
}

// -----------------------------------------------------------------------------
// P3D-071: MODULAR IMPORT COM VALIDATION
// -----------------------------------------------------------------------------

#[test]
fn test_import_obj_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let file_path = dir.path().join("roundtrip.obj");
    pipeline
        .export_single_asset(&proj, 0, &file_path, &ExportOptions::default())
        .expect("export");

    let payload = pipeline
        .import_file(&file_path, &ImportOptions::default())
        .expect("import");

    assert_eq!(payload.meshes.len(), 1);
    let (name, mesh) = &payload.meshes[0];
    assert_eq!(name, "roundtrip");
    assert!(mesh.vert_count() >= 8);
    assert!(mesh.face_count() >= 6);
}

#[test]
fn test_import_pkg_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let file_path = dir.path().join("package.pkg");
    pipeline
        .export_single_asset(&proj, 0, &file_path, &ExportOptions::default())
        .expect("export pkg");

    let payload = pipeline
        .import_file(&file_path, &ImportOptions::default())
        .expect("import pkg");

    assert_eq!(payload.meshes.len(), 1);
    assert_eq!(payload.materials.len(), 1);
    assert_eq!(payload.materials[0].name, "Gold PBR");
}

#[test]
fn test_import_with_scaling() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();

    let file_path = dir.path().join("cube_scaled.obj");
    pipeline
        .export_single_asset(&proj, 0, &file_path, &ExportOptions::default())
        .expect("export");

    let options = ImportOptions {
        scale: 3.0,
        ..Default::default()
    };
    let payload = pipeline
        .import_file(&file_path, &options)
        .expect("import with scale");

    let (_, mesh) = &payload.meshes[0];
    let max_coord = mesh
        .verts
        .iter()
        .flat_map(|v| v.pos)
        .map(|c| c.abs())
        .fold(0.0f32, f32::max);

    // Cubo de tamanho 1.0 (de -0.5 a 0.5) escalado por 3x deve ter coordenadas até 1.5
    assert!((max_coord - 1.5).abs() < 1e-4);
}

// -----------------------------------------------------------------------------
// P3D-072: CAPABILITIES MATRIX & PLUGGABLE ADAPTERS
// -----------------------------------------------------------------------------

#[test]
fn test_format_capabilities_matrix() {
    let pipeline = DeliveryPipeline::new();

    let obj_caps = pipeline.capabilities(FileFormat::Obj).expect("obj caps");
    assert!(!obj_caps.supports_materials);
    assert!(!obj_caps.supports_textures);
    assert!(!obj_caps.supports_binary);

    let glb_caps = pipeline.capabilities(FileFormat::Glb).expect("glb caps");
    assert!(glb_caps.supports_materials);
    assert!(glb_caps.supports_textures);
    assert!(glb_caps.supports_binary);
    assert!(glb_caps.supports_multi_mesh);

    let pkg_caps = pipeline.capabilities(FileFormat::Pkg).expect("pkg caps");
    assert!(pkg_caps.supports_materials);
    assert!(pkg_caps.supports_binary);
}

// -----------------------------------------------------------------------------
// P3D-124: RESILIENT ERROR HANDLING ON HOSTILE / CORRUPT DATA
// -----------------------------------------------------------------------------

#[test]
fn test_hostile_obj_corrupt_data() {
    let dir = tempdir().expect("tempdir");
    let pipeline = DeliveryPipeline::new();

    let corrupt_path = dir.path().join("corrupt.obj");
    fs::write(&corrupt_path, "v 1.0 2.0 3.0\nf 999 998 997\n").expect("write");

    // Tobis/tobj ou mesh validator detectará índice fora de limites
    let res = pipeline.import_file(&corrupt_path, &ImportOptions::default());
    assert!(res.is_err());
}

#[test]
fn test_hostile_pkg_corrupt_zip() {
    let dir = tempdir().expect("tempdir");
    let pipeline = DeliveryPipeline::new();

    let corrupt_path = dir.path().join("corrupt.pkg");
    fs::write(&corrupt_path, b"NOT_A_ZIP_FILE_AT_ALL").expect("write");

    let res = pipeline.import_file(&corrupt_path, &ImportOptions::default());
    assert!(res.is_err());
}

#[test]
fn test_hostile_gltf_malformed_json() {
    let dir = tempdir().expect("tempdir");
    let pipeline = DeliveryPipeline::new();

    let corrupt_path = dir.path().join("corrupt.gltf");
    fs::write(&corrupt_path, b"{ invalid json ").expect("write");

    let res = pipeline.import_file(&corrupt_path, &ImportOptions::default());
    assert!(res.is_err());
}

#[test]
fn batch_names_never_collide_inside_one_export() {
    let dir = tempdir().expect("tempdir");
    let mut proj = Project::new();
    proj.add("Crate", Mesh::cube(1.0));
    proj.add("Crate", Mesh::cube(2.0));
    proj.add("crate", Mesh::plane(1.0));
    let target_dir = dir.path().join("dupes");
    let report = DeliveryPipeline::new()
        .batch_export(
            &proj,
            &target_dir,
            FileFormat::Obj,
            &ExportOptions::default(),
        )
        .expect("batch export");
    assert!(report.all_succeeded());
    let mut names: Vec<String> = report
        .succeeded
        .iter()
        .map(|r| {
            r.output_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    names.sort();
    assert_eq!(names.len(), 3);
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    assert_eq!(names.len(), 3, "case-insensitive collision: {names:?}");
}

#[test]
fn unsupported_options_are_reported_not_silently_dropped() {
    let dir = tempdir().expect("tempdir");
    let proj = sample_test_project();
    let pipeline = DeliveryPipeline::new();
    let scaled = ExportOptions {
        scale: 2.0,
        ..ExportOptions::default()
    };
    let glb = pipeline
        .export_single_asset(&proj, 0, &dir.path().join("a.glb"), &scaled)
        .expect("glb");
    assert!(glb.warnings.iter().any(|w| w.contains("scale")));
    let obj = pipeline
        .export_single_asset(
            &proj,
            0,
            &dir.path().join("a.obj"),
            &ExportOptions::default(),
        )
        .expect("obj");
    assert!(obj.warnings.iter().any(|w| w.contains("materials")));
}
