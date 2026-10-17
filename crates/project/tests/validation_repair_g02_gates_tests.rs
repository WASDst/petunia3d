//! Validation, Repair, Cleanup e Import Boundaries — G02 Gates Tests.
//! Website doc: website/docs/03-geometry/validation-repair-cleanup.md
//!
//! Gates obrigatórios:
//! 1. Decisão 12: OBJ mantém postura estrita (rejeição de NaN/Inf, out-of-bounds e degenerates sem auto-repair).
//! 2. Decisão 13: glTF opera de forma tolerante com warnings explícitos no ImportReport.
//! 3. Decisão 14: Arquivos .petunia atuais com dados corruptos falham explicitamente em LoadMode::Strict;
//!    LoadMode::LegacyRepair migra e audita reparos no LoadReport.
//! 4. Decisões 1 & 2: Validação estrita nunca modifica dados; repair pertence a fronteiras externas e emite relatório.

use base64::Engine;
use petunia_mesh::validation::{
    MeshValidationError, RepairPolicy, repair_external_mesh, validate_invariants,
};
use petunia_mesh::{Face, Mesh, Vertex};
use petunia_project::Project;
use petunia_project::format::{LoadMode, ProjectError, encode_zip, load_bytes_with_report};
use petunia_project::import_gltf::import_scene_bytes;
use petunia_project::import_obj::{ObjImportError, import_obj_bytes};
use std::io::{Cursor, Read, Write};

#[test]
fn test_g02_gate_obj_strict_posture() {
    // 1. OBJ válido com triângulos e quads importa com sucesso e passa em validate_invariants
    let valid_obj = "o ValidQuad\n\
        v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 1.0 1.0 0.0\nv 0.0 1.0 0.0\n\
        f 1 2 3 4\n";
    let mesh = import_obj_bytes(valid_obj.as_bytes()).expect("Import OBJ válido");
    assert_eq!(mesh.verts.len(), 4);
    assert_eq!(mesh.faces.len(), 2, "Quad triangulado na leitura");
    assert!(
        validate_invariants(&mesh).is_ok(),
        "Malha importada deve passar em validate_invariants"
    );

    // 2. OBJ com vértice não-finito (NaN) é estritamente rejeitado com ObjImportError::Validation
    let nan_obj = "o HostileNaN\n\
        v NaN 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
        f 1 2 3\n";
    let err = import_obj_bytes(nan_obj.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ObjImportError::Validation(ref msg) if msg.contains("non-finite")),
        "Esperava rejeição estrita de NaN, veio {err:?}"
    );

    // 3. OBJ com índice de vértice fora dos limites é estritamente rejeitado (no parse ou validação)
    let oob_obj = "o HostileOob\n\
        v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
        f 1 2 999\n";
    let err = import_obj_bytes(oob_obj.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ObjImportError::Parse(ref msg) | ObjImportError::Validation(ref msg) if msg.contains("out of bounds")),
        "Esperava rejeição estrita de índice fora dos limites, veio {err:?}"
    );

    // 4. OBJ com face degenerada (vértices repetidos f 1 1 2) é estritamente rejeitado
    let degen_obj = "o HostileDegen\n\
        v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\n\
        f 1 1 2\n";
    let err = import_obj_bytes(degen_obj.as_bytes()).unwrap_err();
    assert!(
        matches!(err, ObjImportError::Validation(_)),
        "Esperava rejeição de face degenerada no OBJ, veio {err:?}"
    );
}

#[test]
fn test_g02_gate_gltf_tolerant_with_explicit_warnings() {
    // Constrói um glTF sintético contendo 3 posições (36 B) e 4 índices u16 (8 B, incompleto: 4 não é múltiplo de 3)
    let mut bin = Vec::new();
    for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for x in p {
            bin.extend_from_slice(&x.to_le_bytes());
        }
    }
    // 4 índices: [0, 1, 2, 0]
    bin.extend_from_slice(&[0u8, 0, 1, 0, 2, 0, 0, 0]);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bin);

    // O documento contém duas primitivas: uma com modo 0 (POINTS, ignorada com warning) e
    // uma com modo 4 (TRIANGLES, mas com índices incompletos que geram warning).
    let doc = format!(
        r#"{{
        "asset": {{"version": "2.0"}},
        "scenes": [{{"nodes": [0]}}],
        "nodes": [{{"mesh": 0}}],
        "meshes": [{{
            "name": "TolerantMesh",
            "primitives": [
                {{"attributes": {{"POSITION": 0}}, "mode": 0}},
                {{"attributes": {{"POSITION": 0}}, "indices": 1, "mode": 4}}
            ]
        }}],
        "buffers": [{{"byteLength": 44, "uri": "data:application/octet-stream;base64,{b64}"}}],
        "bufferViews": [
            {{"buffer": 0, "byteOffset": 0, "byteLength": 36, "target": 34962}},
            {{"buffer": 0, "byteOffset": 36, "byteLength": 8, "target": 34963}}
        ],
        "accessors": [
            {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0,0.0,0.0], "max": [1.0,1.0,0.0]}},
            {{"bufferView": 1, "componentType": 5123, "count": 4, "type": "SCALAR"}}
        ]
    }}"#
    );

    let scene = import_scene_bytes(doc.as_bytes(), None, "test", true, 1.0, false).expect("import");
    assert_eq!(scene.meshes.len(), 1);
    assert_eq!(
        scene.meshes[0].1.faces.len(),
        1,
        "O triângulo válido foi preservado"
    );

    // Decisão 13: a sanitização tolerante não foi silenciosa — warnings explícitos emitidos!
    assert!(
        !scene.warnings.is_empty(),
        "Importador glTF deve registrar warnings explícitos sobre faces/atributos reparados"
    );
    assert!(
        scene
            .warnings
            .iter()
            .any(|w| w.contains("primitiva(s) de pontos/linhas ignorada(s)")
                || w.contains("incompletos")),
        "Warnings devem relatar anomalias da malha: {:?}",
        scene.warnings
    );

    // E a malha resultante final cumpre estritamente todos os invariantes
    assert!(
        validate_invariants(&scene.meshes[0].1).is_ok(),
        "Malha após reparo tolerante de importação deve cumprir validate_invariants"
    );
}

#[test]
fn test_g02_gate_petunia_strict_load_fails_on_corrupt_data() {
    // 1. Projeto válido salvo round-trip limpo em LoadMode::Strict
    let mut project = Project::new();
    project.name = "StrictValidationTest".to_string();
    let valid_bytes = encode_zip(&project).expect("encode_zip válido");

    let (loaded_clean, report_clean) = load_bytes_with_report(&valid_bytes, LoadMode::Strict)
        .expect("Strict load de arquivo íntegro");
    assert_eq!(loaded_clean.name, "StrictValidationTest");
    assert!(
        report_clean.repairs.is_empty(),
        "Sem reparos em arquivo íntegro"
    );

    // 2. Criação de arquivo .petunia com malha corrompida:
    // Injeta um índice de vértice fora dos limites [0, 1, 999] em uma face
    let cursor = Cursor::new(&valid_bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("abrir zip");
    let mut document_json = Vec::new();
    let mut manifest_json = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        if entry.name() == "document.json" {
            entry.read_to_end(&mut document_json).unwrap();
        } else if entry.name() == "manifest.json" {
            entry.read_to_end(&mut manifest_json).unwrap();
        }
    }

    let mut doc_value: serde_json::Value =
        serde_json::from_slice(&document_json).expect("parse document.json");

    // Injeta índice de vértice fora dos limites na primeira face
    doc_value["assets"][0]["mesh"]["faces"][0]["verts"] = serde_json::json!([0, 1, 999]);

    let corrupt_doc_bytes = serde_json::to_vec(&doc_value).unwrap();

    let mut corrupt_zip_buf = Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut corrupt_zip_buf);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip_writer.start_file("manifest.json", options).unwrap();
        zip_writer.write_all(&manifest_json).unwrap();
        zip_writer.start_file("document.json", options).unwrap();
        zip_writer.write_all(&corrupt_doc_bytes).unwrap();
        zip_writer.finish().unwrap();
    }
    let corrupt_bytes = corrupt_zip_buf.into_inner();

    // Decisão 14: Em LoadMode::Strict, o carregamento do arquivo com dados corrompidos
    // FALHA explicitamente com ProjectError::RepairRequired!
    let strict_result = load_bytes_with_report(&corrupt_bytes, LoadMode::Strict);
    assert!(
        matches!(strict_result, Err(ProjectError::RepairRequired(ref msg)) if msg.contains("invariantes")),
        "LoadMode::Strict deve recusar malha com índice fora dos limites em vez de reparar em silêncio: {strict_result:?}"
    );

    // Em LoadMode::LegacyRepair, o arquivo corrompido é recuperado com reparo auditado
    let (repaired_project, _repair_report) =
        load_bytes_with_report(&corrupt_bytes, LoadMode::LegacyRepair)
            .expect("LegacyRepair deve carregar com sanitização controlada");

    // A malha recuperada teve a face inválida removida e agora cumpre validate_invariants
    let active_mesh = repaired_project.active_mesh().expect("active mesh");
    assert!(
        validate_invariants(active_mesh).is_ok(),
        "Malha após LegacyRepair deve passar em validate_invariants"
    );
}

#[test]
fn test_g02_gate_validation_never_modifies_data_and_repair_is_explicit() {
    // Decisão 1: validate_invariants nunca muta o dado, apenas inspeciona
    let mut mesh = Mesh::default();
    mesh.verts.push(Vertex::new(f32::NAN, 1.0, 2.0));
    mesh.verts.push(Vertex::new(1.0, 0.0, 0.0));
    mesh.verts.push(Vertex::new(0.0, 1.0, 0.0));
    mesh.faces.push(Face::new(vec![0, 1, 2]));

    // Antes da validação
    assert!(mesh.verts[0].pos[0].is_nan());

    let err = validate_invariants(&mesh).unwrap_err();
    assert!(matches!(
        err,
        MeshValidationError::NonFiniteVertexPosition { index: 0, .. }
    ));

    // Após validação estrita, o vértice continua inalterado (sem auto-repair)
    assert!(
        mesh.verts[0].pos[0].is_nan(),
        "validate_invariants é estritamente de leitura (sem mutação)"
    );

    // Decisão 2: o reparo pertence exclusivamente a repair_external_mesh e produz relatório auditável
    let report = repair_external_mesh(&mut mesh, RepairPolicy::TolerantImport);
    assert!(!report.is_clean());
    assert_eq!(report.vertices_sanitized, 1);
    assert_eq!(mesh.verts[0].pos[0], 0.0, "Reparo externo higienizou o NaN");

    // Agora passa em validate_invariants
    assert!(validate_invariants(&mesh).is_ok());
}
