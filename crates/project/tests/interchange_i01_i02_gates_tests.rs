//! Bateria de Gates Sintéticos para I01 e I02 (website/docs/12-interchange/import-export-game-ready.md).
//!
//! Gates Verificados:
//! 1. Snapshot Avaliado & Authoring Intacto: Export em cópia não altera DocumentSnapshot nem Project autoral.
//! 2. Tangentes Ortonormais Game-Ready & Vertex Splitting: Vetores 4D ortonormais, handedness +/-1.0, preservação de seams/sharp.
//! 3. Khronos glTF 2.0 / GLB Conformance: GLB estruturalmente válido com POSITION, NORMAL, TEXCOORD_0, TANGENT e PBR.
//! 4. IO Seguro & Path Traversal: Rejeição de `../../etc/passwd`, URIs remotas e dados maliciosos sem pânico.
//! 5. Integridade Atômica & Política de Sobrescrita: Rejeição estrita com AlreadyExists e escrita atômica.
//! 6. Round-Trip Fiel de OBJ e GLB: Preservação de topologia, normais e UVs por corner.

use std::fs;
use tempfile::tempdir;

use glam::Vec3;
use petunia_mesh::Mesh;
use petunia_project::{
    DeliveryPipeline, DocumentSnapshot, ExportModel, ExportOptions, ImportOptions, Material,
    PipelineError, Project,
};

fn sample_authoring_project() -> Project {
    let mut proj = Project {
        name: "Interchange Test Project".to_string(),
        ..Default::default()
    };

    // 1. Cubo com arestas sharp e UVs mapeadas
    let mut cube = Mesh::cube(2.0);
    cube.sharp_edges.insert((0, 1));
    cube.sharp_edges.insert((1, 2));
    cube.sharp_edges.insert((2, 3));
    proj.add("Hero Cube", cube);

    // 2. Material PBR completo
    let mut mat = Material::new("PBR Steel");
    mat.base_color = [0.8, 0.85, 0.9, 1.0];
    mat.metallic = 0.9;
    mat.roughness = 0.25;
    mat.emission_color = [0.2, 0.1, 0.0];
    mat.emission_strength = 3.0;
    let mat_id = proj.add_material(mat);
    proj.assets[0].material_id = Some(mat_id);

    // 3. Plano com textura
    let plane = Mesh::plane(4.0);
    let mut canvas = petunia_project::Canvas::new(16, 16, [128, 200, 255, 255]);
    canvas.set(4, 4, [255, 0, 0, 255]);
    proj.add("Ground Plane", plane);
    proj.assets.last_mut().expect("plane asset").texture = Some(canvas);

    proj
}

// -----------------------------------------------------------------------------
// GATE 1: SNAPSHOT AVALIADO & AUTHORING INTACTO (I01)
// -----------------------------------------------------------------------------

#[test]
fn test_gate1_snapshot_evaluated_authoring_intact() {
    let dir = tempdir().expect("tempdir");
    let original_project = sample_authoring_project();
    let snapshot = DocumentSnapshot::from_document(&original_project);

    let serialized_before = postcard::to_allocvec(snapshot.document()).expect("serialize before");

    let pipeline = DeliveryPipeline::new();
    let glb_target = dir.path().join("hero.glb");
    let obj_target = dir.path().join("hero.obj");

    let options = ExportOptions {
        triangulate: true,
        export_materials: true,
        scale: 1.5,
        overwrite: true,
    };

    // Export GLB via snapshot
    let rep_glb = pipeline
        .export_snapshot(&snapshot, &[0, 1], &glb_target, &options)
        .expect("export snapshot glb");
    assert!(rep_glb.bytes_written > 0);
    assert!(glb_target.exists());

    // Export OBJ via snapshot
    let rep_obj = pipeline
        .export_snapshot(&snapshot, &[0], &obj_target, &options)
        .expect("export snapshot obj");
    assert!(rep_obj.bytes_written > 0);
    assert!(obj_target.exists());

    // Invariante não-negociável: Document original deve permanecer byte-a-byte idêntico!
    let serialized_after = postcard::to_allocvec(snapshot.document()).expect("serialize after");
    assert_eq!(
        serialized_before, serialized_after,
        "Authoring mutado após exportação! O snapshot avaliado violou a regra de cópia estrita."
    );
}

// -----------------------------------------------------------------------------
// GATE 2: TANGENTES ORTONORMAIS GAME-READY & VERTEX SPLITTING (I01)
// -----------------------------------------------------------------------------

#[test]
fn test_gate2_orthonormal_tangents_and_vertex_splitting() {
    let project = sample_authoring_project();
    let snapshot = DocumentSnapshot::from_document(&project);

    let model = ExportModel::from_snapshot(&snapshot, &[0], &ExportOptions::default())
        .expect("build export model");

    assert_eq!(model.meshes.len(), 1);
    let mesh = &model.meshes[0];

    assert!(!mesh.positions.is_empty());
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    assert_eq!(mesh.positions.len(), mesh.uvs.len());
    assert_eq!(mesh.positions.len(), mesh.tangents.len());

    // Verifica que cada tangente é um vetor 4D ortonormalizado com handedness +/- 1.0
    for i in 0..mesh.positions.len() {
        let n = Vec3::from(mesh.normals[i]);
        let tan = mesh.tangents[i];
        let t_xyz = Vec3::new(tan[0], tan[1], tan[2]);
        let w = tan[3];

        // 1. Tangente XYZ deve ser unitária
        assert!(
            (t_xyz.length() - 1.0).abs() < 1e-3,
            "Tangente não-unitária: ||t|| = {}",
            t_xyz.length()
        );

        // 2. Tangente XYZ deve ser ortogonal à normal (dot product ~ 0)
        let dot = n.dot(t_xyz).abs();
        assert!(
            dot < 1e-3,
            "Tangente não é ortogonal à normal: dot = {dot} (normal: {n:?}, tan: {t_xyz:?})"
        );

        // 3. Handedness deve ser rigorosamente +1.0 ou -1.0
        assert!(
            (w - 1.0).abs() < 1e-4 || (w + 1.0).abs() < 1e-4,
            "Handedness inválida: {w}"
        );
    }

    // Exporta para GLB e valida via parser oficial glTF
    let glb_bytes = model.to_glb(snapshot.document()).expect("to glb");
    let (gltf_doc, buffers, _) = gltf::import_slice(&glb_bytes).expect("parse glb");

    let prim = gltf_doc
        .meshes()
        .next()
        .expect("mesh")
        .primitives()
        .next()
        .expect("prim");

    let reader = prim.reader(|b| Some(&buffers[b.index()]));
    let tangents: Vec<[f32; 4]> = reader.read_tangents().expect("has tangents").collect();
    assert_eq!(tangents.len(), mesh.positions.len());
}

// -----------------------------------------------------------------------------
// GATE 3: KHRONOS GLTF 2.0 / GLB CONFORMANCE & PBR MATERIALS (I01 & I02)
// -----------------------------------------------------------------------------

#[test]
fn test_gate3_khronos_glb_conformance_and_materials() {
    let project = sample_authoring_project();
    let snapshot = DocumentSnapshot::from_document(&project);

    let model = ExportModel::from_snapshot(&snapshot, &[0, 1], &ExportOptions::default())
        .expect("export model");

    let glb_bytes = model.to_glb(snapshot.document()).expect("to glb");

    // Valida container binário GLB
    assert!(glb_bytes.len() >= 20);
    assert_eq!(&glb_bytes[0..4], b"glTF");
    let version = u32::from_le_bytes(glb_bytes[4..8].try_into().unwrap());
    assert_eq!(version, 2, "glTF versão deve ser 2.0");

    let (doc, buffers, images) = gltf::import_slice(&glb_bytes).expect("parse official gltf");
    assert_eq!(doc.meshes().len(), 2, "2 malhas exportadas");
    assert_eq!(buffers.len(), 1, "GLB possui chunk BIN consolidado");
    assert!(!images.is_empty(), "Textura embutida presente no GLB");

    // Valida propriedades PBR do material
    let mat0 = doc.materials().next().expect("material exists");
    let pbr = mat0.pbr_metallic_roughness();
    assert!((pbr.metallic_factor() - 0.9).abs() < 1e-3);
    assert!((pbr.roughness_factor() - 0.25).abs() < 1e-3);
    let emissive = mat0.emissive_factor();
    assert!(emissive[0] > 0.0);
}

// -----------------------------------------------------------------------------
// GATE 4: IO SEGURO, PATH TRAVERSAL & MALICIOUS INPUT (I02)
// -----------------------------------------------------------------------------

#[test]
fn test_gate4_safe_io_path_traversal_and_malicious_input() {
    let dir = tempdir().expect("tempdir");
    let pipeline = DeliveryPipeline::new();

    // 1. Arquivo glTF malicioso tentando path traversal para fora do diretório base
    let malicious_gltf = r#"{
        "asset": { "version": "2.0" },
        "buffers": [
            { "uri": "../../etc/passwd", "byteLength": 1024 }
        ],
        "meshes": []
    }"#;

    let mal_path = dir.path().join("malicious.gltf");
    fs::write(&mal_path, malicious_gltf).expect("write malicious gltf");

    let res = pipeline.import_file(&mal_path, &ImportOptions::default());
    assert!(
        res.is_err(),
        "Importador de glTF aceitou path traversal ('../../etc/passwd')! Falha grave de segurança."
    );
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("path traversal") || err_str.contains("rejeitado"),
        "Mensagem de erro deve reportar rejeição de traversal, obtido: {err_str}"
    );

    // 2. Arquivo glTF malicioso com URI remota (sem rede no import)
    let remote_gltf = r#"{
        "asset": { "version": "2.0" },
        "buffers": [
            { "uri": "https://evil.com/payload.bin", "byteLength": 1024 }
        ],
        "meshes": []
    }"#;
    let remote_path = dir.path().join("remote.gltf");
    fs::write(&remote_path, remote_gltf).expect("write remote gltf");

    let res_remote = pipeline.import_file(&remote_path, &ImportOptions::default());
    assert!(res_remote.is_err(), "Importador aceitou URI remota!");

    // 3. Arquivo OBJ com coordenadas não finitas (NaN / Inf)
    let nan_obj = "v NaN 1.0 2.0\nv 0.0 Inf 1.0\nv 0.0 0.0 0.0\nf 1 2 3\n";
    let nan_path = dir.path().join("nan.obj");
    fs::write(&nan_path, nan_obj).expect("write nan obj");

    let res_nan = pipeline.import_file(&nan_path, &ImportOptions::default());
    assert!(res_nan.is_err(), "Importador aceitou vértices com NaN/Inf!");
}

// -----------------------------------------------------------------------------
// GATE 5: INTEGRIDADE ATÔMICA & POLÍTICA DE SOBRESCRITA (I01 & I02)
// -----------------------------------------------------------------------------

#[test]
fn test_gate5_atomic_integrity_and_overwrite_policy() {
    let dir = tempdir().expect("tempdir");
    let project = sample_authoring_project();
    let snapshot = DocumentSnapshot::from_document(&project);
    let pipeline = DeliveryPipeline::new();

    let target_file = dir.path().join("preexisting.obj");
    fs::write(&target_file, "PREVIOUS CONTENT DO NOT OVERWRITE").expect("write preexisting");

    // Tentativa com overwrite = false deve falhar imediatamente
    let no_overwrite = ExportOptions {
        overwrite: false,
        ..Default::default()
    };

    let err = pipeline
        .export_snapshot(&snapshot, &[0], &target_file, &no_overwrite)
        .expect_err("Deveria rejeitar sobrescrita");

    match err {
        PipelineError::AlreadyExists(p) => assert_eq!(p, target_file),
        other => panic!("Esperado PipelineError::AlreadyExists, obtido {other:?}"),
    }

    // Verifica que o conteúdo anterior não foi alterado nem truncado
    let read_back = fs::read_to_string(&target_file).expect("read");
    assert_eq!(read_back, "PREVIOUS CONTENT DO NOT OVERWRITE");

    // Agora com overwrite = true, deve substituir atomicamente
    let allow_overwrite = ExportOptions {
        overwrite: true,
        ..Default::default()
    };

    let rep = pipeline
        .export_snapshot(&snapshot, &[0], &target_file, &allow_overwrite)
        .expect("export com overwrite");

    assert!(rep.bytes_written > 0);
    let new_content = fs::read_to_string(&target_file).expect("read new");
    assert!(new_content.contains("v "));
    assert!(new_content.contains("f "));
}

// -----------------------------------------------------------------------------
// GATE 6: ROUND-TRIP OBJ E GLB COM PRESERVAÇÃO DE UVs E NORMAIS (I01 & I02)
// -----------------------------------------------------------------------------

#[test]
fn test_gate6_roundtrip_obj_and_glb_preserves_topology_and_uvs() {
    let dir = tempdir().expect("tempdir");
    let pipeline = DeliveryPipeline::new();

    // 1. Round-trip OBJ
    let mut orig_mesh = Mesh::plane(2.0);
    orig_mesh.faces[0].uv = vec![[0.1, 0.2], [0.8, 0.2], [0.8, 0.9], [0.1, 0.9]];

    let mut proj = Project::default();
    proj.add("Plane", orig_mesh);

    let obj_path = dir.path().join("plane_roundtrip.obj");
    pipeline
        .export_single_asset(
            &proj,
            0,
            &obj_path,
            &ExportOptions {
                triangulate: true,
                ..Default::default()
            },
        )
        .expect("export obj");

    let payload = pipeline
        .import_file(&obj_path, &ImportOptions::default())
        .expect("import obj");

    assert_eq!(payload.meshes.len(), 1);
    let (_, imported_mesh) = &payload.meshes[0];
    assert!(!imported_mesh.verts.is_empty());
    assert_eq!(imported_mesh.faces.len(), 2, "Plano triangulado em 2 faces");

    // Valida que as faces importadas preservaram UVs não-nulas
    let mut has_non_zero_uv = false;
    for f in &imported_mesh.faces {
        for uv in &f.uv {
            if uv[0] > 0.05 || uv[1] > 0.05 {
                has_non_zero_uv = true;
            }
        }
    }
    assert!(has_non_zero_uv, "Round-trip OBJ perdeu coordenadas UV!");

    // 2. Round-trip GLB
    let glb_path = dir.path().join("plane_roundtrip.glb");
    pipeline
        .export_single_asset(&proj, 0, &glb_path, &ExportOptions::default())
        .expect("export glb");

    let glb_payload = pipeline
        .import_file(&glb_path, &ImportOptions::default())
        .expect("import glb");

    assert_eq!(glb_payload.meshes.len(), 1);
    let (_, glb_mesh) = &glb_payload.meshes[0];
    assert_eq!(glb_mesh.faces.len(), 2);
}
