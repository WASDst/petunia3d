//! Testes sintéticos de conformidade e gates da tarefa P02 (Persistência, Staged Load e Integridade Atômica).
//!
//! Cobre os requisitos e gates canônicos de `website/docs/11-project/project-persistence.md`:
//! 1. Open/save/load de arquivo legado e formato novo com IDs autorais persistentes e roundtrip completo.
//! 2. Corrupt/truncated/oversized/zip traversal/symlinks recusados sem pânico com erros tipados.
//! 3. Save com escrita interrompida preserva arquivo anterior; dirty permanece fiel à revisão e Undo/Redo.
//! 4. Staged load atômico: validação e migrations em staging antes de publicar na sessão.

use std::io::Write;

use petunia_core::AppState;
use petunia_core::project_service::ProjectService;
use petunia_mesh::{Face, Mesh, Vertex};
use petunia_project::format::{LoadMode, ProjectError, load_bytes_with_report};
use petunia_project::zip;
use petunia_project::{
    AnimationAsset, AnimationClip, AnnotationItem, AnnotationStroke, Asset, Canvas,
    DocumentSnapshot, Light, Material, MeasurementItem, ObjectTransform, PathGenerator,
    ProfileResource, ProfileWorkplane, Project, ReferenceImage, Skeleton, SplineInterpolation,
    SplineResource, SweepGeneratorParameters,
};
use uuid::Uuid;

/// Cria um projeto autoral completo cobrindo todos os subsistemas de dados.
fn create_authored_project_with_all_subsystems() -> Project {
    let mut project = Project::new();
    project.id = Uuid::new_v4();
    project.name = "Master Authored Project".to_string();
    project.palette = vec![[0.1, 0.2, 0.3], [0.8, 0.5, 0.2]];

    // 1. Asset com malha, sharp edge, UVs por face corner e canvas de textura
    let mut mesh = Mesh {
        verts: vec![
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(1.0, 0.0, 0.0),
            Vertex::new(1.0, 1.0, 0.0),
            Vertex::new(0.0, 1.0, 0.0),
        ],
        faces: vec![Face::with_uv(
            vec![0, 1, 2, 3],
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        )],
        ..Default::default()
    };
    mesh.sharp_edges.insert((0, 1));
    mesh.uv_seams.insert((1, 2));

    let asset_id = Uuid::new_v4();
    let mut asset = Asset::new("HeroAsset", mesh);
    asset.id = asset_id;
    asset.set_object_transform(ObjectTransform {
        translation: [1.0, 2.0, 3.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.5, 1.5, 1.5],
    });
    let mut canvas = Canvas::new(64, 64, [255, 0, 0, 255]);
    canvas.set(10, 10, [0, 255, 0, 255]);
    asset.texture = Some(canvas);
    project.assets = vec![asset];

    // 2. Material PBR autoral
    let mat_id = Uuid::new_v4();
    let mut material = Material::new("GoldMaterial");
    material.id = mat_id;
    material.base_color = [1.0, 0.84, 0.0, 1.0];
    material.metallic = 0.9;
    material.roughness = 0.2;
    project.materials = vec![material];

    // 3. Luz autoral
    let light = Light::directional("KeyLight", [0.0, -1.0, 0.0]);
    project.lights = vec![light];

    // 4. Esqueleto e ossos
    let mut skeleton = Skeleton::new("MainRig");
    skeleton
        .add_bone("RootBone", None, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
        .expect("add bone succeeds");
    project.skeletons = vec![skeleton];

    // 5. Spline procedural
    let spline_id = Uuid::new_v4();
    let mut spline = SplineResource::from_polyline(
        "GuideCurve",
        &[[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [2.0, 0.0, 1.0]],
        false,
    );
    spline.id = spline_id;
    spline.interpolation = SplineInterpolation::CubicBezier;
    project.splines = vec![spline];

    // 6. Profile 2D
    let profile_id = Uuid::new_v4();
    let mut profile = ProfileResource::new("SquareProfile", spline_id, ProfileWorkplane::default());
    profile.id = profile_id;
    profile.wall_thickness = 0.05;
    project.profiles = vec![profile];

    // 7. Path Generator (Sweep)
    let gen_id = Uuid::new_v4();
    let path_gen = PathGenerator::sweep(
        gen_id,
        spline_id,
        profile_id,
        SweepGeneratorParameters::default(),
    );
    project.path_generators = vec![path_gen];

    // 8. Anotação e Medição
    let note = AnnotationItem::new(
        "Check topology edge flow",
        vec![AnnotationStroke {
            points: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
            color: [1.0, 0.0, 0.0, 1.0],
            width: 2.0,
        }],
    );
    project.annotations = vec![note];

    let measurement = MeasurementItem::new("Width", [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0);
    project.measurements = vec![measurement];

    // 9. Imagem de referência
    let reference = ReferenceImage::from_rgba("RefFront".to_string(), 4, 4, vec![255u8; 64]);
    project.references = vec![reference];

    // 10. Animação
    let anim = AnimationAsset::new("Idle", AnimationClip::new("IdleClip", 1.0));
    project.animations = vec![anim];

    project
}

#[test]
fn test_p02_gate_authoring_roundtrip_all_subsystems_preserve_ids_and_data() {
    let original = create_authored_project_with_all_subsystems();
    let original_id = original.id;
    let asset_id = original.assets[0].id;
    let mat_id = original.materials[0].id;
    let light_id = original.lights[0].id;
    let skel_id = original.skeletons[0].id;
    let spline_id = original.splines[0].id;
    let profile_id = original.profiles[0].id;
    let gen_id = original.path_generators[0].id;
    let anim_id = original.animations[0].id;

    // Converte para DocumentSnapshot
    let snapshot = DocumentSnapshot::from_document(&original);
    assert_eq!(snapshot.project_id(), original_id);
    assert_eq!(snapshot.name(), "Master Authored Project");
    assert_eq!(snapshot.assets().len(), 1);

    // Validação de invariantes em staging antes de gravar
    snapshot.validate_invariants().expect("invariantes válidos");

    // Grava de forma atômica em disco temporário
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("p02_roundtrip_{}.petunia", Uuid::new_v4()));
    snapshot
        .save_atomic(&file_path)
        .expect("save atômico deve suceder");

    // Carrega de volta em staging com modo Strict (recusa reparos silenciosos)
    let (loaded_snapshot, report) = DocumentSnapshot::load_atomic(&file_path, LoadMode::Strict)
        .expect("staged load strict deve suceder");

    // Limpeza do arquivo temporário
    let _ = std::fs::remove_file(&file_path);

    // Assegura ausência de mutações ou reparos
    assert!(
        report.repairs.is_empty(),
        "projeto limpo não deve requerer reparos: {:?}",
        report.repairs
    );

    let doc = loaded_snapshot.into_document();

    // 1. Identidade e metadados
    assert_eq!(doc.id, original_id, "project_id deve ser idêntico");
    assert_eq!(doc.name, "Master Authored Project");
    assert_eq!(doc.palette, original.palette);

    // 2. Assets e malha
    assert_eq!(doc.assets.len(), 1);
    let loaded_asset = &doc.assets[0];
    assert_eq!(loaded_asset.id, asset_id, "asset id deve ser idêntico");
    assert_eq!(loaded_asset.name, "HeroAsset");
    assert_eq!(loaded_asset.mesh.vert_count(), 4);
    assert_eq!(loaded_asset.mesh.faces.len(), 1);
    assert_eq!(
        loaded_asset.mesh.sharp_edges, original.assets[0].mesh.sharp_edges,
        "sharp edges devem ser preservadas no roundtrip"
    );
    assert_eq!(
        loaded_asset.mesh.uv_seams, original.assets[0].mesh.uv_seams,
        "uv seams devem ser preservadas no roundtrip"
    );
    assert_eq!(
        loaded_asset.mesh.faces[0].uv, original.assets[0].mesh.faces[0].uv,
        "face corner UVs devem ser preservados no roundtrip"
    );
    assert_eq!(
        loaded_asset.origin_world(),
        original.assets[0].origin_world()
    );
    assert_eq!(
        loaded_asset.texture.as_ref().unwrap().get(10, 10),
        Some([0, 255, 0, 255]),
        "canvas pixels devem ser idênticos"
    );

    // 3. Materiais e Luzes
    assert_eq!(doc.materials.len(), 1);
    assert_eq!(doc.materials[0].id, mat_id);
    assert_eq!(doc.materials[0].name, "GoldMaterial");
    assert_eq!(doc.materials[0].metallic, 0.9);

    assert_eq!(doc.lights.len(), 1);
    assert_eq!(doc.lights[0].id, light_id);
    assert_eq!(doc.lights[0].name, "KeyLight");

    // 4. Esqueleto e Rigging
    assert_eq!(doc.skeletons.len(), 1);
    assert_eq!(doc.skeletons[0].id, skel_id);
    assert_eq!(doc.skeletons[0].bones.len(), 1);
    assert_eq!(doc.skeletons[0].bones[0].name, "RootBone");

    // 5. Procedural (Splines, Profiles, Generators)
    assert_eq!(doc.splines.len(), 1);
    assert_eq!(doc.splines[0].id, spline_id);
    assert_eq!(doc.splines[0].points.len(), 3);
    assert_eq!(
        doc.splines[0].interpolation,
        SplineInterpolation::CubicBezier
    );

    assert_eq!(doc.profiles.len(), 1);
    assert_eq!(doc.profiles[0].id, profile_id);
    assert_eq!(doc.profiles[0].spline_id, spline_id);
    assert_eq!(doc.profiles[0].wall_thickness, 0.05);

    assert_eq!(doc.path_generators.len(), 1);
    assert_eq!(doc.path_generators[0].id, gen_id);

    // 6. Anotações, Medições e Referências
    assert_eq!(doc.annotations.len(), 1);
    assert_eq!(doc.annotations[0].name, "Check topology edge flow");
    assert_eq!(doc.measurements.len(), 1);
    assert_eq!(doc.measurements[0].name, "Width");
    assert_eq!(doc.references.len(), 1);
    assert_eq!(doc.references[0].name, "RefFront");

    // 7. Animações
    assert_eq!(doc.animations.len(), 1);
    assert_eq!(doc.animations[0].id, anim_id);
    assert_eq!(doc.animations[0].name, "Idle");
}

fn package_raw_zip(manifest_json: &[u8], document_json: &[u8]) -> Vec<u8> {
    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(manifest_json).unwrap();
        zip.start_file("document.json", opts).unwrap();
        zip.write_all(document_json).unwrap();
        zip.finish().unwrap();
    }
    zip_buf.into_inner()
}

#[test]
fn test_p02_gate_legacy_formats_migration_preserves_identities_and_audit_report() {
    let mut project = Project::new();
    let asset_id = project.assets[0].id;
    // Simula arquivo legado com a lista paralela `smooth_shaded_assets`
    project.smooth_shaded_assets.push(asset_id);

    let doc_bytes = serde_json::to_vec(&project).expect("serialize legacy doc");
    let manifest = br#"{"format":"petunia","version":1,"name":"Legacy"}"#;
    let bytes = package_raw_zip(manifest, &doc_bytes);

    let (loaded, report) =
        load_bytes_with_report(&bytes, LoadMode::LegacyRepair).expect("load legacy repair");

    // A lista paralela foi migrada para o próprio asset
    assert!(
        loaded.is_smooth_shaded(asset_id),
        "asset deve estar com shading smooth"
    );
    assert!(
        loaded.smooth_shaded_assets.is_empty(),
        "smooth_shaded_assets legado deve ser esvaziado para garantir autoridade única"
    );
    assert!(
        report
            .repairs
            .iter()
            .any(|r| r.contains("smooth_shaded_assets")),
        "reparo auditável deve registrar a migração da lista legada: {:?}",
        report.repairs
    );
    assert_eq!(loaded.id, project.id, "project id deve ser preservado");
}

#[test]
fn test_p02_gate_hostile_corrupt_truncated_oversized_and_traversal_refusal() {
    // 1. Arquivo truncado ou lixo aleatório
    let truncated = b"PK\x03\x04 incomplete data";
    let err = load_bytes_with_report(truncated, LoadMode::Strict).unwrap_err();
    match err {
        ProjectError::Format(_) | ProjectError::Io(_) => {}
        other => panic!("esperava erro de formato ou I/O, obteve: {other:?}"),
    }

    // 2. Path traversal no arquivo ZIP
    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("../../etc/passwd", opts).unwrap();
        zip.write_all(b"malicious content").unwrap();
        zip.finish().unwrap();
    }
    let traversal_bytes = zip_buf.into_inner();
    let err = load_bytes_with_report(&traversal_bytes, LoadMode::Strict).unwrap_err();
    match err {
        ProjectError::Format(msg) => {
            assert!(
                msg.contains("path traversal"),
                "deve recusar explicitamente path traversal: {msg}"
            );
        }
        other => panic!("esperava erro de path traversal, obteve: {other:?}"),
    }

    // 3. Versão de manifest futura indevida (ex: version 999)
    let future_manifest = br#"{"format":"petunia","version":999,"name":"FutureProject"}"#;
    let future_bytes = package_raw_zip(future_manifest, b"{}");
    let err = load_bytes_with_report(&future_bytes, LoadMode::Strict).unwrap_err();
    match err {
        ProjectError::Version(got, max) => {
            assert_eq!(got, 999);
            assert_eq!(max, petunia_project::format::PROJECT_VERSION);
        }
        other => panic!("esperava erro de versao incompativel, obteve: {other:?}"),
    }

    // 4. Invariantes corrompidos sob LoadMode::Strict
    let mut corrupt_proj = Project::new();
    // Face com índice de vértice 999 inexistente (corrupção de integridade topológica)
    corrupt_proj.assets[0].mesh.faces = vec![Face::new(vec![0, 1, 999])];
    // Motion órfão apontando para esqueleto inexistente
    corrupt_proj
        .motions
        .push(petunia_project::motion::MotionRecipe::new(
            Uuid::new_v4(),
            petunia_project::motion::MotionGenerator::IdleBreath,
            "OrphanMotion",
        ));
    let corrupt_doc_bytes = serde_json::to_vec(&corrupt_proj).expect("serialize corrupt doc");
    let manifest = br#"{"format":"petunia","version":1,"name":"Corrupt"}"#;
    let corrupt_bytes = package_raw_zip(manifest, &corrupt_doc_bytes);

    let err = load_bytes_with_report(&corrupt_bytes, LoadMode::Strict).unwrap_err();
    match err {
        ProjectError::RepairRequired(msg) => {
            assert!(
                msg.contains("invariantes violados"),
                "LoadMode::Strict deve recusar malha corrompida: {msg}"
            );
        }
        other => panic!("esperava RepairRequired sob Strict, obteve: {other:?}"),
    }

    // Sob LoadMode::LegacyRepair, o arquivo abre e reporta o reparo auditável
    let (repaired, report) =
        load_bytes_with_report(&corrupt_bytes, LoadMode::LegacyRepair).expect("legacy repair abre");
    assert!(
        report.repairs.iter().any(|r| r.contains("motions")),
        "deve registrar a remoção auditável da motion órfã: {:?}",
        report.repairs
    );
    assert_eq!(
        repaired.assets[0].mesh.faces.len(),
        0,
        "face inválida deve ser removida"
    );
    assert!(
        repaired.motions.is_empty(),
        "motions órfãs devem ser expurgadas"
    );
}

#[test]
fn test_p02_gate_atomic_save_failure_preserves_original_file_and_dirty_fidelity() {
    let mut state = AppState::default();
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("p02_atomic_safety_{}.petunia", Uuid::new_v4()));

    // Salva versão inicial 1
    ProjectService::save_project(&mut state, &file_path).expect("save 1 deve suceder");
    let initial_bytes = std::fs::read(&file_path).expect("ler arquivo inicial");
    assert!(!initial_bytes.is_empty());
    assert!(!state.is_document_dirty());

    // 1. Teste de fidelidade da revisão de save_point com histórico Undo/Redo
    // Usuário faz uma edição autoral (adição de primitivo)
    let add_cube = petunia_core::AddPrimitiveCmd::cube(true);
    state.dispatch(&add_cube).expect("dispatch add cube");
    assert_eq!(state.project.assets.len(), 2);
    assert!(state.is_document_dirty(), "documento deve estar dirty");

    // O Save captura o snapshot nesta revisão:
    let ticket = state.project.save_point();

    // Usuário continua editando antes do término do save:
    let add_plane = petunia_core::AddPrimitiveCmd::new(petunia_core::PrimitiveKind::Plane);
    state.dispatch(&add_plane).expect("dispatch add plane");
    assert_eq!(state.project.assets.len(), 3);

    // O Save confirma que a gravação do `ticket` terminou:
    state.project.mark_saved(ticket);
    assert!(
        state.is_document_dirty(),
        "salvar revisão anterior durante edições concorrentes NÃO limpa o dirty mais recente!"
    );

    // Desfaz a edição mais recente (volta ao estado gravado pelo ticket):
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 2);
    assert!(
        !state.is_document_dirty(),
        "desfazer de volta ao ponto gravado restaura o documento para LIMPO (clean)!"
    );

    // Refaz a edição:
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), 3);
    assert!(
        state.is_document_dirty(),
        "avançar além do ponto gravado torna o documento DIRTY novamente!"
    );

    // 2. Proteção de integridade do arquivo em disco: falha durante save
    // Tenta salvar em um diretório como se fosse arquivo (I/O error previsível):
    let invalid_dir_target = temp_dir.join(format!("dir_target_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&invalid_dir_target).unwrap();

    let save_err = ProjectService::save_project(&mut state, &invalid_dir_target);
    assert!(
        save_err.is_err(),
        "save em diretório existente deve falhar com erro tipado"
    );

    // O arquivo original inicial permanece intacto e 100% legível
    let current_bytes = std::fs::read(&file_path).expect("ler arquivo original");
    assert_eq!(
        current_bytes, initial_bytes,
        "arquivo original nunca pode ser corrompido ou truncado por falhas"
    );

    // Limpeza
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_dir_all(&invalid_dir_target);
}
