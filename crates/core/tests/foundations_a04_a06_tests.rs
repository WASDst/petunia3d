//! Testes de conformidade arquitetural para Foundations A04 e A06:
//! - A04: Commands/Queries/Events com DTOs, diagnostics e invalidações pós-commit.
//! - A06: ToolSession/Undo/dirty com isolamento de settings, preview/commit/cancel e autoridade única.

use petunia_core::command::{
    Command, CommandContext, CommandError, CommandExecutionReport, DuplicateSelectionCmd,
    SelectAllCmd,
};
use petunia_core::modal::ModalKind;
use petunia_core::state::{ActiveToolSessionKind, AppState, ToolActivation, ToolSettingsSnapshot};
use petunia_core::uv_transform_session::UvTransformKind;
use petunia_mesh::Mesh;
use petunia_project::ProjectChanges;

struct DiagnosticTestCmd;

impl Command for DiagnosticTestCmd {
    fn label(&self) -> &'static str {
        "Diagnostic Test"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY
    }

    fn execute(&self, ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
        ctx.add_diagnostic("diagnóstico de topologia verificado com sucesso");
        ctx.set_status("Operação de teste concluída");
        ctx.mark_dirty();
        Ok(())
    }
}

struct FailingPanicCmd;

impl Command for FailingPanicCmd {
    fn label(&self) -> &'static str {
        "Failing Panic"
    }

    fn execute(&self, _ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
        panic!("simulated failure inside command");
    }
}

#[test]
fn test_a04_command_execution_report_and_invalidation_dto() {
    let mut state = AppState::default();
    let initial_revision = state.project.project.revision_id();
    let (initial_undo, initial_redo) = state.project.undo.depth();
    assert_eq!(initial_undo, 0);
    assert_eq!(initial_redo, 0);

    // 1. Despacho não-destrutivo com relatório detalhado
    let report: CommandExecutionReport = state
        .dispatch_detailed(&SelectAllCmd)
        .expect("SelectAllCmd deve executar com sucesso");

    assert_eq!(report.label, "select all");
    assert!(report.changes.contains(ProjectChanges::SELECTION));
    assert_eq!(report.undo_depth, 0);
    assert_eq!(report.redo_depth, 0);
    assert!(report.effects.redraw);

    // 2. Comando destrutivo com mensagens de diagnóstico (A04)
    let diag_report = state
        .dispatch_detailed(&DiagnosticTestCmd)
        .expect("DiagnosticTestCmd deve executar com sucesso");

    assert_eq!(diag_report.label, "Diagnostic Test");
    assert!(diag_report.changes.contains(ProjectChanges::GEOMETRY));
    assert_eq!(diag_report.undo_depth, 1);
    assert_eq!(diag_report.redo_depth, 0);
    assert_eq!(
        diag_report.effects.status.as_deref(),
        Some("Operação de teste concluída")
    );
    assert_eq!(
        diag_report.effects.diagnostics,
        vec!["diagnóstico de topologia verificado com sucesso".to_string()]
    );
    assert_ne!(diag_report.revision, initial_revision);

    // 3. Comando registrado com despacho detalhado
    let registered_report = state
        .dispatch_command_detailed("model.select_all")
        .expect("model.select_all registrado deve executar");
    assert_eq!(registered_report.label, "select all");
    assert_eq!(registered_report.undo_depth, 1);

    // 4. Isolamento contra falhas e panics: restauração de documento sem avanço de histórico
    let pre_panic_revision = state.project.project.revision_id();
    let pre_panic_depth = state.project.undo.depth();

    let err = state.dispatch_detailed(&FailingPanicCmd);
    assert!(
        err.is_err(),
        "Comando que entra em pânico deve retornar Err"
    );
    assert_eq!(
        state.project.undo.depth(),
        pre_panic_depth,
        "Histórico de Undo não deve avançar após falha"
    );
    assert_eq!(
        state.project.project.revision_id(),
        pre_panic_revision,
        "Revisão do documento não deve avançar após falha"
    );
}

#[test]
fn test_a06_tool_settings_snapshot_isolation() {
    let mut state = AppState::default();

    // Capturar configurações padrão
    let snapshot: ToolSettingsSnapshot = state.tool_settings_snapshot();
    assert_eq!(snapshot.extrude_dist, 0.5);
    assert_eq!(snapshot.tool_activation, ToolActivation::Drag);

    // Modificar configurações na sessão
    state.session.tools.extrude_dist = 2.5;
    state.session.tools.bevel_amount = 0.8;
    state.session.tools.paint_radius = 1.6;
    state.session.tools.brush_hardness = 0.75;
    state.session.tools.tool_activation = ToolActivation::Instant;

    assert_eq!(state.session.tools.extrude_dist, 2.5);
    assert_eq!(state.session.tools.tool_activation, ToolActivation::Instant);

    // Restaurar snapshot e conferir isolamento
    state.restore_tool_settings(&snapshot);
    assert_eq!(state.session.tools.extrude_dist, 0.5);
    assert_eq!(state.session.tools.bevel_amount, 0.15);
    assert_eq!(state.session.tools.paint_radius, 0.8);
    assert_eq!(state.session.tools.brush_hardness, 0.0);
    assert_eq!(state.session.tools.tool_activation, ToolActivation::Drag);
}

#[test]
fn test_a06_active_tool_session_kinds_and_cancel() {
    let mut state = AppState::default();

    // 1. Estado inicial inativo
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::None
    );
    assert!(!state.session.tools.is_session_active());
    assert!(!state.cancel_active_tool_session());

    // 2. Sessão Modal ativa
    state
        .begin_modal(ModalKind::Move)
        .expect("begin_modal deve ter sucesso");
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::Modal
    );
    assert!(state.session.tools.is_session_active());

    // Cancelar sessão modal
    assert!(state.cancel_active_tool_session());
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::None
    );
    assert!(!state.session.tools.is_session_active());
    assert_eq!(
        state.project.undo.depth().0,
        0,
        "Cancelamento não polui Undo"
    );

    // 3. Sessão Cut ativa
    let cube = Mesh::cube(1.0);
    state.session.tools.cut_session = Some(petunia_core::CutSession::new(cube));
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::Cut
    );
    assert!(state.cancel_active_tool_session());
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::None
    );

    // 4. Sessão UvTransform ativa (com cantos UV selecionados)
    let mesh = state.project.active_mesh().cloned().expect("malha ativa");
    state.session.uv_selection.set_faces(&mesh, [0]);
    state
        .begin_uv_transform(UvTransformKind::Move)
        .expect("begin_uv_transform deve ter sucesso");
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::UvTransform
    );
    assert!(state.cancel_active_tool_session());
    assert_eq!(
        state.session.tools.active_session_kind(),
        ActiveToolSessionKind::None
    );
}

#[test]
fn test_a06_single_authority_dirty_state_lifecycle() {
    let mut state = AppState::default();

    // Começa sem alterações não salvas
    assert!(!state.is_dirty(), "Estado inicial deve ser limpo");

    // Selecionar e duplicar (operação destrutiva que gera checkpoint de Undo)
    state
        .dispatch(&SelectAllCmd)
        .expect("SelectAll deve executar");
    state
        .dispatch(&DuplicateSelectionCmd)
        .expect("DuplicateSelection deve executar");
    assert!(
        state.is_dirty(),
        "Após comando destrutivo, is_dirty deve ser true"
    );

    // Desfazer volta ao ponto limpo
    assert!(state.undo());
    assert!(
        !state.is_dirty(),
        "Após desfazer até o ponto salvo, is_dirty deve ser false"
    );

    // Refazer torna dirty novamente
    assert!(state.redo());
    assert!(
        state.is_dirty(),
        "Após refazer, is_dirty deve voltar a ser true"
    );
}
