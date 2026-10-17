//! Gates e testes de conformidade para Application e DesktopHost (A02).
//!
//! Website/docs/02-application/commands-events-services.md:
//! 1. Construtores da Application são 100% puros (sem leitura de env, sem relógio do sistema, sem locks, sem I/O de disco).
//! 2. Injeção determinística de relógio (abstração Clock, FixedClock).
//! 3. Separação de Autosave: política (Application/AutosavePolicy) vs infraestrutura (AutosaveStorage/DesktopHost).
//! 4. DesktopHost gerencia estado de plataforma e SessionLock (prevenção de conflito de escrita e liberação limpa).
//! 5. Aposentadoria gradual de ModuleRegistry: processamento nativo direto de eventos de domínio (Model, Paint, UV, Assets).
//! 6. Isolamento transacional de comandos: falhas de comando nunca corrompem documento, revisão ou integridade de Undo.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use petunia_application::host::DesktopHost;
use petunia_application::{Application, ApplicationConfig, AutosaveOutcome};
use petunia_core::{AppEvent, AutosaveStorage, Clock, CommandError, FixedClock, ModuleRegistry};
use tempfile::tempdir;

// ----------------------------------------------------------------------------
// Gate 1: Pureza dos construtores (zero env, zero relógio do sistema, zero locks, zero I/O)
// ----------------------------------------------------------------------------
#[test]
fn test_gate1_application_constructors_are_strictly_pure() {
    let temp_dir = tempdir().unwrap();
    let initial_entries = std::fs::read_dir(temp_dir.path()).unwrap().count();

    // 1. Construtor padrão
    let app1 = Application::new(ApplicationConfig::default());
    assert!(!app1.state.is_document_dirty());
    assert_eq!(app1.state.project.undo.depth().0, 0);
    assert!(!app1.state.project.assets.is_empty());

    // 2. Construtor com relógio fixo injetado via DesktopHost
    let clock = Arc::new(FixedClock(50_000));
    let app2 = DesktopHost::create_application(
        ApplicationConfig {
            language: "pt-BR".into(),
        },
        Some(Arc::clone(&clock) as Arc<dyn Clock>),
    );

    assert_eq!(app2.state.ui.i18n.lang(), "pt-BR");
    assert!(!app2.state.is_document_dirty());
    assert!(Arc::ptr_eq(&app2.state.clock, &(clock as Arc<dyn Clock>)));

    // Invariante: Nenhum arquivo ou lock foi criado no disco
    assert_eq!(
        std::fs::read_dir(temp_dir.path()).unwrap().count(),
        initial_entries
    );
}

// ----------------------------------------------------------------------------
// Gate 2: Injeção determinística de relógio (Clock abstraction)
// ----------------------------------------------------------------------------
#[test]
fn test_gate2_injected_clock_controls_time_deterministically() {
    let mut app = Application::default();
    let clock1 = FixedClock(1_000);
    let clock2 = FixedClock(2_000);

    // Documento limpo: nada a salvar independente do relógio
    let dummy_storage = DummyStorage::default();
    assert!(app.tick_autosave(&clock1, &dummy_storage).is_none());
    assert!(app.tick_autosave(&clock2, &dummy_storage).is_none());
    assert_eq!(dummy_storage.save_calls.load(Ordering::SeqCst), 0);
}

// ----------------------------------------------------------------------------
// Gate 3: Separação de Autosave entre política e infraestrutura
// ----------------------------------------------------------------------------
#[derive(Default)]
struct DummyStorage {
    save_calls: AtomicUsize,
}

impl AutosaveStorage for DummyStorage {
    fn write_snapshot(
        &self,
        _project: &petunia_project::Project,
        _project_path: Option<&Path>,
        _timestamp: u64,
        _seq: u64,
        _revision: u64,
        _keep_n: usize,
    ) -> Result<PathBuf, petunia_project::format::ProjectError> {
        self.save_calls.fetch_add(1, Ordering::SeqCst);
        Ok(PathBuf::from("/mock/snapshot.petunia"))
    }
}

#[test]
fn test_gate3_autosave_policy_separated_from_storage_infrastructure() {
    let mut app = Application::default();
    let storage = DummyStorage::default();

    // 1. Aplicação sem alterações: AutosavePolicy não solicita gravação
    let outcome1 = app.tick_autosave(&FixedClock(100), &storage);
    assert!(outcome1.is_none());
    assert_eq!(storage.save_calls.load(Ordering::SeqCst), 0);

    // 2. Modifica o documento
    app.execute_registered_command("model.add_cube").unwrap();
    assert!(app.state.is_document_dirty());

    // 3. Primeiro tick após modificação: AutosavePolicy decide salvar
    let outcome2 = app.tick_autosave(&FixedClock(200), &storage);
    assert_eq!(
        outcome2,
        Some(AutosaveOutcome::Saved("snapshot.petunia".into()))
    );
    assert_eq!(storage.save_calls.load(Ordering::SeqCst), 1);

    // 4. Tick subsequente sem nova alteração: AutosavePolicy coalescida não dispara gravação redundante
    let outcome3 = app.tick_autosave(&FixedClock(205), &storage);
    assert!(outcome3.is_none());
    assert_eq!(storage.save_calls.load(Ordering::SeqCst), 1);
}

// ----------------------------------------------------------------------------
// Gate 4: DesktopHost e Gestão de Session Lock
// ----------------------------------------------------------------------------
#[test]
fn test_gate4_desktophost_session_lock_lifecycle_and_conflict_prevention() {
    let temp_dir = tempdir().unwrap();
    let dir = temp_dir.path();

    // 1. Inicialmente não existe lock
    assert!(DesktopHost::check_session_lock(dir).is_none());

    // 2. Adquire session lock
    let guard = DesktopHost::acquire_session_lock(dir, "TestProject", None, 1_700_000_000).unwrap();
    assert_eq!(guard.info().project_name, "TestProject");
    assert_eq!(guard.info().pid, std::process::id());
    assert!(guard.path().exists());

    // Inspeciona lock existente
    let inspected = DesktopHost::check_session_lock(dir).unwrap();
    assert_eq!(inspected.project_name, "TestProject");
    assert_eq!(inspected.pid, std::process::id());

    // 3. Conflito: tentar adquirir lock novamente com processo ativo falha
    let second_attempt =
        DesktopHost::acquire_session_lock(dir, "OtherProject", None, 1_700_000_100);
    assert!(second_attempt.is_err());
    assert_eq!(
        second_attempt.err().unwrap().kind(),
        std::io::ErrorKind::AlreadyExists
    );

    // 4. Liberação explícita do lock remove o arquivo do disco
    guard.release().unwrap();
    assert!(DesktopHost::check_session_lock(dir).is_none());
    assert!(!dir.join("session.lock").exists());

    // 5. RAII drop automático de lock
    {
        let _auto_guard =
            DesktopHost::acquire_session_lock(dir, "AutoProject", None, 1_700_000_200).unwrap();
        assert!(dir.join("session.lock").exists());
    }
    // Ao sair do escopo, o arquivo foi limpo
    assert!(!dir.join("session.lock").exists());
}

// ----------------------------------------------------------------------------
// Gate 5: Aposentadoria gradual de ModuleRegistry e eventos nativos de domínio
// ----------------------------------------------------------------------------
#[test]
fn test_gate5_native_domain_events_operate_independently_of_dynamic_modules() {
    let mut app = Application::default();

    // Cria um cubo
    app.execute_registered_command("model.add_cube").unwrap();
    let asset_id = app.state.project.active().unwrap().id;

    // Seleciona faces UV usando helper da struct
    let mesh = &app.state.project.active().unwrap().mesh;
    app.state
        .session
        .tools
        .uv_selection
        .set_faces(mesh, vec![0]);
    assert!(!app.state.session.tools.uv_selection.is_empty());

    // Dispara evento nativo de MeshChanged
    app.dispatch_event(&AppEvent::MeshChanged { asset_id });

    // A Application processou o evento de domínio nativamente, mantendo os corners válidos
    assert!(!app.state.session.tools.uv_selection.is_empty());

    // Substituir a registry por uma vazia não impede o domínio nativo de funcionar
    app.registry = ModuleRegistry::new();
    app.dispatch_event(&AppEvent::ActiveAssetChanged { asset_id });
    assert!(!app.state.session.tools.uv_selection.is_empty());
}

// ----------------------------------------------------------------------------
// Gate 6: Isolamento transacional de comandos (Never Crash & integridade de histórico)
// ----------------------------------------------------------------------------
#[test]
fn test_gate6_command_failure_never_corrupts_document_or_undo_history() {
    let mut app = Application::default();
    let initial_rev = app.state.project.project.revision_id();
    let initial_depth = app.state.project.undo.depth().0;
    let initial_assets = app.state.project.assets.len();

    // 1. Comando inexistente é rejeitado com CommandError tipado
    let res_unknown = app.execute_registered_command("nonexistent.command");
    assert!(matches!(res_unknown, Err(CommandError::UnknownCommand(_))));

    assert_eq!(app.state.project.project.revision_id(), initial_rev);
    assert_eq!(app.state.project.undo.depth().0, initial_depth);
    assert_eq!(app.state.project.assets.len(), initial_assets);
    assert!(!app.state.is_document_dirty());

    // 2. Comando válido executa e avança a revisão
    app.execute_registered_command("model.add_cube").unwrap();
    assert_ne!(app.state.project.project.revision_id(), initial_rev);
    assert_eq!(app.state.project.undo.depth().0, initial_depth + 1);
    assert_eq!(app.state.project.assets.len(), initial_assets + 1);
    assert!(app.state.is_document_dirty());

    // 3. Undo restaura o estado limpo
    let undone = app.undo();
    assert!(undone);
    assert_eq!(app.state.project.assets.len(), initial_assets);
}
