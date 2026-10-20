//! Project Persistence, Autosave e Crash Recovery — P03 Gates Tests.
//! Website doc: website/docs/11-project/project-persistence.md
//!
//! Gates obrigatórios:
//! 1. Recover/Inspect/Discard: inspeção não toca a sessão aberta; recuperação entra como dirty
//!    e preserva o arquivo original; descarte remove gerações individualmente ou em bloco.
//! 2. Preservação do original: recuperação nunca sobrescreve o arquivo oficial automaticamente.
//! 3. Retenção de 5 gerações e coalescimento por project_id e revisão.
//! 4. Resiliência a falhas/crashes: marcadores de lock determinam unclean shutdown.

use petunia_core::AppState;
use petunia_core::project_service::ProjectService;
use petunia_mesh::Mesh;
use petunia_project::Project;
use petunia_project::autosave::{AutosaveConfig, AutosaveService};
use petunia_project::format;
use std::fs;

#[test]
fn test_p03_gate_recover_inspect_discard_lifecycle() {
    let test_dir = std::env::temp_dir().join(format!("petunia_p03_cycle_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir).expect("criar diretório de teste");
    let official_project_path = test_dir.join("original_work.petunia");

    // 1. Cria projeto original com cubo simples e salva no disco
    let mut initial_project = Project::new();
    initial_project.name = "OriginalProject".to_string();
    format::save_atomic(&initial_project, &official_project_path).expect("salvar projeto oficial");

    // 2. Inicia sessão: grava marcador de lock
    AutosaveService::create_session_lock(Some(&official_project_path), &initial_project.name, 1000)
        .expect("criar session.lock");

    // 3. Modifica projeto (adiciona malha) e executa autosaves com revisões crescentes
    initial_project.add("ExtraMesh", Mesh::cube(2.0));
    let snap_r1 = AutosaveService::perform_snapshot(
        &initial_project,
        Some(&official_project_path),
        1100,
        1,
        10,
        5,
    )
    .expect("autosave r10");

    initial_project.add("SecondMesh", Mesh::plane(4.0));
    let snap_r2 = AutosaveService::perform_snapshot(
        &initial_project,
        Some(&official_project_path),
        1200,
        2,
        20,
        5,
    )
    .expect("autosave r20");

    // 4. Simula Crash: o processo termina sem chamar remove_session_lock
    assert!(AutosaveService::session_lock_path(Some(&official_project_path)).exists());

    // 5. Na inicialização do próximo ciclo: detecta necessidade de recuperação
    let rec_info = AutosaveService::detect_recovery(Some(&official_project_path))
        .expect("recuperação deve ser detectada");
    assert_eq!(rec_info.project_name, "OriginalProject");
    assert_eq!(rec_info.snapshot_path, snap_r2);
    assert!(rec_info.is_newer_than_main);

    let candidates = AutosaveService::list_recovery_generations(Some(&official_project_path));
    assert_eq!(candidates.len(), 2, "Duas gerações salvas");
    assert_eq!(
        candidates[0].revision,
        Some(20),
        "A primeira é a mais recente"
    );

    // 6. Inspect: inspeciona sem tocar no estado da aplicação atual
    let mut app_state = AppState::new("en");
    let original_doc_id = app_state.project.project.id;

    let summary = ProjectService::inspect_recovery(&candidates[0]).expect("inspect_recovery");
    assert_eq!(summary.name, "OriginalProject");
    assert_eq!(summary.asset_count, 3); // Cubo inicial + ExtraMesh + SecondMesh
    assert_eq!(summary.revision, Some(20));

    // Garante que o documento aberto na aplicação não foi tocado pelo inspect
    assert_eq!(app_state.project.project.id, original_doc_id);
    assert_eq!(app_state.project.project.assets.len(), 1);

    // 7. Recover: recupera a geração escolhida para a sessão
    ProjectService::recover_generation(&mut app_state, &candidates[0]).expect("recover_generation");

    // Documento foi carregado e está marcado como dirty para exigir Save explícito
    assert_eq!(app_state.project.project.name, "OriginalProject");
    assert_eq!(app_state.project.project.assets.len(), 3);
    assert!(
        app_state.is_document_dirty(),
        "Documento recuperado deve estar em estado dirty"
    );
    assert_eq!(
        app_state.project.project_path.as_deref(),
        Some(official_project_path.to_str().unwrap())
    );

    // 8. O arquivo original no disco AINDA está com o conteúdo V1 inicial (1 asset)!
    let on_disk_original = format::load(&official_project_path).expect("ler arquivo em disco");
    assert_eq!(
        on_disk_original.assets.len(),
        1,
        "Arquivo oficial no disco não deve ser sobrescrito pelo Recover"
    );

    // 9. Discard: descarta a geração mais antiga individualmente
    AutosaveService::discard_generation(&candidates[1]).expect("discard generation antiga");
    assert!(!snap_r1.exists());
    assert!(snap_r2.exists());

    // 10. Descarte total limpa o lock e os snapshots remanescentes
    AutosaveService::discard_recovery(Some(&official_project_path)).expect("discard total");
    assert!(!snap_r2.exists());
    assert!(!AutosaveService::session_lock_path(Some(&official_project_path)).exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_p03_gate_five_generations_retention_and_coalescing() {
    let test_dir =
        std::env::temp_dir().join(format!("petunia_p03_retention_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&test_dir).expect("criar diretório");
    let proj_path = test_dir.join("retention.petunia");

    let p = Project::new();

    // Grava 8 snapshots rotativos com limite configurado de keep_n = 5
    for i in 1..=8 {
        let _ = AutosaveService::perform_snapshot(&p, Some(&proj_path), 2000 + i, i, i * 10, 5)
            .expect("snapshot rotativo");
    }

    let candidates = AutosaveService::list_recovery_generations(Some(&proj_path));
    assert_eq!(
        candidates.len(),
        5,
        "Política de retenção deve manter no máximo 5 gerações"
    );

    // As gerações retidas são as 5 mais novas (revisões 80, 70, 60, 50, 40)
    let revs: Vec<_> = candidates.iter().map(|c| c.revision.unwrap()).collect();
    assert_eq!(revs, vec![80, 70, 60, 50, 40]);

    // Teste de coalescimento: se o documento está dirty mas a revisão não mudou, decide() recusa novo snapshot
    let mut service = AutosaveService::new(AutosaveConfig {
        enabled: true,
        interval_secs: 60,
        keep_n: 5,
        only_when_dirty: true,
    });

    // Primeiro disparo para a revisão 100 através de tick()
    let step1 = service.tick(100, true, 100, &p, Some(&proj_path));
    assert!(step1.is_some(), "Primeiro disparo deve gravar snapshot");

    // Próximo ciclo 120s depois: ainda dirty, mas mesma revisão 100 -> coalescido (None)
    let step2 = service.decide(220, true, 100);
    assert!(
        step2.is_none(),
        "Ciclos sem alteração de revisão devem ser coalescidos"
    );

    // Quando a revisão avança para 101 -> autoriza novo snapshot
    let step3 = service.decide(220, true, 101);
    assert!(
        step3.is_some(),
        "Avanço de revisão deve autorizar novo snapshot"
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_p03_gate_untitled_documents_isolated_by_project_id() {
    let doc_a = Project::new();
    let doc_b = Project::new();
    assert_ne!(doc_a.id, doc_b.id);

    let dir_a = AutosaveService::autosave_dir_for(None, doc_a.id);
    let dir_b = AutosaveService::autosave_dir_for(None, doc_b.id);
    assert_ne!(
        dir_a, dir_b,
        "Diretórios de untitled devem ser isolados por project_id"
    );

    // Snapshots de doc_a com keep_n = 1 não removem snapshots de doc_b
    let snap_b = AutosaveService::perform_snapshot(&doc_b, None, 100, 1, 1, 1).unwrap();
    let snap_a1 = AutosaveService::perform_snapshot(&doc_a, None, 110, 1, 1, 1).unwrap();
    let snap_a2 = AutosaveService::perform_snapshot(&doc_a, None, 120, 2, 2, 1).unwrap();

    assert!(snap_b.exists(), "Snapshot de B não pode ser removido por A");
    assert!(
        !snap_a1.exists(),
        "Snapshot antigo de A foi podado pelo limite keep_n=1"
    );
    assert!(snap_a2.exists(), "Snapshot mais recente de A permanece");

    let _ = fs::remove_file(&snap_b);
    let _ = fs::remove_file(&snap_a2);
}
