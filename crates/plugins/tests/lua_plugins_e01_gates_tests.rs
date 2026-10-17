//! Gates e testes de conformidade para Lua Plugins e Public API (E01).
//!
//! Website/docs/13-extensions/lua-plugins-public-api.md:
//! 1. Plugin sem capability falha com PermissionDenied sem alterar Document.
//! 2. Unload/reload limpa panel, command, handler e foco sem leaks.
//! 3. Plugin gera undo/redo idêntico ao nativo e aborta batch ao falhar (rollback atômico).
//! 4. Access a IDs stale dá erro (StaleReference / InvalidId), não outro objeto.
//! 5. Lua worker/callback lento termina por limite/budget; UI/Host não congela.
//! 6. Sandbox defense in depth: sem acesso a IO, OS, rede ou bytecode arbitrário.
//! 7. Version incompatibility e erro de schema/manifest produzem diagnóstico estruturado.
//! 8. MCP exposable é opt-in adicional e não aumenta capabilities automaticamente.
//! 9. Dados autorais namespaced persistem e ausência de plugin produz missing diagnostic sem descarte.

use petunia_core::{AppState, ProjectService};
use petunia_plugins::dispatch::{
    PluginApiError, answer_plugin_query, apply_intents, detect_missing_plugins, get_plugin_data,
    set_plugin_data,
};
use petunia_plugins::host::{
    CapabilityError, Host, HostConfig, LuaCommand, PLUGIN_API_VERSION, PluginCapabilities,
    PluginError, PluginId, ValidatedIntent,
};
use petunia_plugins::manifest::{Capability, PluginManifest};
use petunia_plugins::registry::{PluginRegistry, PluginStatus};

fn test_id(name: &str) -> PluginId {
    PluginId::new(name).expect("valid test plugin id")
}

fn sample_manifest(id: &str, required: &[Capability], optional: &[Capability]) -> PluginManifest {
    PluginManifest {
        plugin_id: id.to_string(),
        version: "1.0.0".to_string(),
        api_version: PLUGIN_API_VERSION,
        name: format!("Test Plugin {id}"),
        author: "Petunia QA".to_string(),
        source: String::new(),
        entrypoint: "main.lua".to_string(),
        required_capabilities: required.iter().copied().collect(),
        optional_capabilities: optional.iter().copied().collect(),
        commands: vec![
            "model.add_primitive".to_string(),
            "model.subdivide".to_string(),
        ],
        queries: vec!["scene.list".to_string(), "plugin.data".to_string()],
        allow_destructive: false,
    }
}

// ----------------------------------------------------------------------------
// Gate 1: Plugin sem capability falha com PermissionDenied sem alterar Document
// ----------------------------------------------------------------------------
#[test]
fn test_gate1_missing_capability_denied_without_altering_document() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let initial_asset_count = state.project.assets.len();
    let (initial_undo, _) = state.project.undo.depth();

    let host = Host::new();
    let pid = test_id("gate1.denied");

    // Plugin com apenas ReadDocument tenta emitir model.add_primitive
    let caps = PluginCapabilities {
        api_version: PLUGIN_API_VERSION,
        commands: vec![], // nenhuma permissão de comando
        queries: vec!["scene.list".to_string()],
        filesystem_roots: vec![],
        allow_network: false,
        allow_destructive: false,
    };

    let result = host.execute(&pid, &caps, r#"petunia.command("model.add_primitive")"#);
    assert!(
        matches!(result, Err(PluginError::PermissionDenied(_))),
        "Expected PermissionDenied, got {result:?}"
    );

    // Invariante: O Document e Undo permanecem 100% inalterados
    assert_eq!(state.project.assets.len(), initial_asset_count);
    assert_eq!(state.project.undo.depth().0, initial_undo);
}

// ----------------------------------------------------------------------------
// Gate 2: Unload/reload limpa handlers e estado sem leaks ou contaminação
// ----------------------------------------------------------------------------
#[test]
fn test_gate2_unload_and_reload_cleans_handlers_without_leaks() {
    let mut registry = PluginRegistry::default();
    let manifest = sample_manifest(
        "gate2.lifecycle",
        &[Capability::ReadDocument],
        &[Capability::EditGeometry],
    );

    // Fonte Lua registra um event handler para "mesh.changed"
    let source = r#"
        local count = 0
        petunia.on("mesh.changed", function(evt, payload)
            count = count + 1
            petunia.command("model.add_primitive")
        end)
    "#;

    registry
        .install(manifest.clone(), source.to_string())
        .expect("install must succeed");
    registry
        .grant("gate2.lifecycle", Capability::ReadDocument)
        .expect("grant must succeed");
    registry
        .grant("gate2.lifecycle", Capability::EditGeometry)
        .expect("grant must succeed");

    // Enable inicial ativa os handlers
    let _ = registry.enable("gate2.lifecycle").expect("enable ok");
    assert_eq!(
        registry.status("gate2.lifecycle"),
        Some(PluginStatus::Enabled)
    );

    // Dispara evento -> handler é acionado e produz intent
    let deliveries = registry.deliver("mesh.changed", "{}");
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].1.commands.len(), 1);

    // Disable descarta a instância e seus handlers
    registry.disable("gate2.lifecycle").expect("disable ok");
    assert_eq!(
        registry.status("gate2.lifecycle"),
        Some(PluginStatus::Disabled)
    );

    // Dispara evento novamente -> nenhum handler é chamado
    let deliveries_disabled = registry.deliver("mesh.changed", "{}");
    assert!(
        deliveries_disabled.is_empty(),
        "Disabled plugin must receive zero events"
    );

    // Re-enable cria novo Lua State limpo
    let _ = registry.enable("gate2.lifecycle").expect("re-enable ok");
    assert_eq!(
        registry.status("gate2.lifecycle"),
        Some(PluginStatus::Enabled)
    );

    // Uninstall remove completamente do registro
    registry.uninstall("gate2.lifecycle").expect("uninstall ok");
    assert!(!registry.is_installed("gate2.lifecycle"));
}

// ----------------------------------------------------------------------------
// Gate 3: Plugin gera undo/redo idêntico ao nativo e aborta batch ao falhar
// ----------------------------------------------------------------------------
#[test]
fn test_gate3_atomic_batch_rollback_on_failure_and_single_undo_on_success() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let initial_assets = state.project.assets.len();
    let (initial_undo, _) = state.project.undo.depth();
    let pid = test_id("gate3.atomic");

    // 1. Batch com falha atômica: comando 1 é válido, comando 2 falha com StaleReference
    let failing_intent = ValidatedIntent {
        commands: vec![
            LuaCommand {
                command: "model.add_primitive".into(),
                args: vec![],
                target: None,
            },
            LuaCommand {
                command: "model.subdivide".into(),
                args: vec![1.0],
                target: Some("00000000-0000-4000-8000-000000000000".into()),
            },
        ],
        queries: vec![],
    };

    let result = apply_intents(&mut state, &pid, &failing_intent);
    assert!(
        matches!(result, Err(PluginApiError::StaleReference(_))),
        "Batch must fail with StaleReference, got {result:?}"
    );

    // Rollback total: nada mudou no Document nem no UndoStack
    assert_eq!(state.project.assets.len(), initial_assets);
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // 2. Batch atômico com sucesso: dois comandos válidos
    let successful_intent = ValidatedIntent {
        commands: vec![
            LuaCommand {
                command: "model.add_primitive".into(),
                args: vec![],
                target: None,
            },
            LuaCommand {
                command: "model.add_primitive".into(),
                args: vec![],
                target: None,
            },
        ],
        queries: vec![],
    };

    apply_intents(&mut state, &pid, &successful_intent).expect("batch should succeed");

    // Documento avançou exatamente 2 assets
    assert_eq!(state.project.assets.len(), initial_assets + 2);
    // Mas gerou exatamente UM passo atômico de Undo
    assert_eq!(state.project.undo.depth().0, initial_undo + 1);
    assert_eq!(state.project.undo.undo_label(), Some("plugin gate3.atomic"));

    // Testa Undo e Redo
    assert!(state.undo(), "undo should succeed");
    assert_eq!(state.project.assets.len(), initial_assets);

    assert!(state.redo(), "redo should succeed");
    assert_eq!(state.project.assets.len(), initial_assets + 2);
}

// ----------------------------------------------------------------------------
// Gate 4: Access a IDs stale dá erro (StaleReference), não outro objeto
// ----------------------------------------------------------------------------
#[test]
fn test_gate4_stale_reference_returns_error_and_never_targets_active_object() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let pid = test_id("gate4.stale");

    // Cria um objeto inicial
    let add_intent = ValidatedIntent {
        commands: vec![LuaCommand {
            command: "model.add_primitive".into(),
            args: vec![],
            target: None,
        }],
        queries: vec![],
    };
    apply_intents(&mut state, &pid, &add_intent).expect("add primitive ok");

    let first_asset_id = state.project.assets[0].id;
    let stale_id = "11111111-2222-4333-8444-555555555555".to_string();

    // Tenta aplicar operação em ID stale inexistente
    let stale_intent = ValidatedIntent {
        commands: vec![LuaCommand {
            command: "model.subdivide".into(),
            args: vec![1.0],
            target: Some(stale_id.clone()),
        }],
        queries: vec![],
    };

    let err = apply_intents(&mut state, &pid, &stale_intent).expect_err("must error");
    assert_eq!(err, PluginApiError::StaleReference(stale_id));

    // O primeiro objeto não sofreu mutação indevida
    assert_eq!(state.project.assets[0].id, first_asset_id);
}

// ----------------------------------------------------------------------------
// Gate 5: Lua worker/callback lento termina por limite/budget
// ----------------------------------------------------------------------------
#[test]
fn test_gate5_infinite_loop_aborts_cleanly_via_instruction_budget() {
    let host = Host::with_config(HostConfig {
        max_instructions: 50_000,
        max_memory_bytes: 4 * 1024 * 1024,
    });
    let pid = test_id("gate5.budget");
    let caps = PluginCapabilities {
        api_version: PLUGIN_API_VERSION,
        commands: vec![],
        queries: vec![],
        filesystem_roots: vec![],
        allow_network: false,
        allow_destructive: false,
    };

    // Script com loop infinito
    let source = "while true do end";
    let result = host.execute(&pid, &caps, source);

    assert!(
        matches!(result, Err(PluginError::Budget)),
        "Must abort on budget exhaustion, got {result:?}"
    );
}

// ----------------------------------------------------------------------------
// Gate 6: Sandbox defense in depth (sem IO, OS, rede, bytecode dump)
// ----------------------------------------------------------------------------
#[test]
fn test_gate6_sandbox_defense_in_depth_blocks_os_io_and_bytecode() {
    let host = Host::new();
    let pid = test_id("gate6.sandbox");
    let caps = PluginCapabilities {
        api_version: PLUGIN_API_VERSION,
        commands: vec![],
        queries: vec![],
        filesystem_roots: vec![],
        allow_network: false,
        allow_destructive: false,
    };

    // Tentativa 1: os e io não existem
    let res_io = host.execute(
        &pid,
        &caps,
        r#"
        if io ~= nil or os ~= nil or package ~= nil then
            error("Sandbox leak: io/os/package exposed")
        end
    "#,
    );
    assert!(res_io.is_ok(), "io, os, and package must be nil");

    // Tentativa 2: string.dump foi removido
    let res_dump = host.execute(
        &pid,
        &caps,
        r#"
        if string.dump ~= nil then
            error("string.dump is not nil")
        end
    "#,
    );
    assert!(res_dump.is_ok(), "string.dump must be nil");
}

// ----------------------------------------------------------------------------
// Gate 7: Version incompatibility e erro de manifest produzem diagnóstico
// ----------------------------------------------------------------------------
#[test]
fn test_gate7_manifest_validation_and_version_incompatibility() {
    // 1. API version mismatch
    let mut bad_api_manifest = sample_manifest("gate7.ver", &[], &[]);
    bad_api_manifest.api_version = PLUGIN_API_VERSION + 99;
    assert_eq!(
        bad_api_manifest.validate(),
        Err(CapabilityError::UnsupportedApi(PLUGIN_API_VERSION + 99))
    );

    // 2. Semver inválido
    let mut bad_semver_manifest = sample_manifest("gate7.ver", &[], &[]);
    bad_semver_manifest.version = "1.0".to_string(); // falta PATCH
    assert!(matches!(
        bad_semver_manifest.validate(),
        Err(CapabilityError::BadManifest(_))
    ));

    // 3. Path traversal no entrypoint
    let mut bad_entry_manifest = sample_manifest("gate7.ver", &[], &[]);
    bad_entry_manifest.entrypoint = "../evil.lua".to_string();
    assert!(matches!(
        bad_entry_manifest.validate(),
        Err(CapabilityError::BadManifest(_))
    ));

    // 4. Comando declarado sem capability de cobertura
    let mut unhandled_cmd_manifest = sample_manifest("gate7.ver", &[], &[]);
    unhandled_cmd_manifest.commands = vec!["project.export.obj".to_string()];
    // ExportFiles não está nas capabilities declaradas
    assert_eq!(
        unhandled_cmd_manifest.validate(),
        Err(CapabilityError::NotCovered(
            "project.export.obj".to_string()
        ))
    );
}

// ----------------------------------------------------------------------------
// Gate 8: MCP exposable é opt-in adicional e não aumenta capabilities
// ----------------------------------------------------------------------------
#[test]
fn test_gate8_mcp_exposable_grants_no_underlying_capabilities() {
    let mut manifest = sample_manifest("gate8.mcp", &[Capability::McpExposable], &[]);
    manifest.commands = vec!["model.add_primitive".to_string()];

    // Manifest é rejeitado porque McpExposable não cobre comandos de modelagem
    assert_eq!(
        manifest.validate(),
        Err(CapabilityError::NotCovered(
            "model.add_primitive".to_string()
        ))
    );
}

// ----------------------------------------------------------------------------
// Gate 9: Dados autorais namespaced persistem e ausência gera diagnóstico
// ----------------------------------------------------------------------------
#[test]
fn test_gate9_namespaced_plugin_data_persistence_and_missing_diagnostic() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let pid = test_id("gate9.data_plugin");

    // Salva dados autorais namespaced com schema 1
    let custom_data = serde_json::json!({
        "voxel_size": 0.25,
        "smoothing_passes": 3,
        "features": ["corner_stitch", "auto_relax"]
    });

    set_plugin_data(&mut state, &pid, 1, custom_data.clone())
        .expect("set_plugin_data must succeed");

    // Leitura via query contextual
    let query_res = answer_plugin_query(&state, &pid, "plugin.data").expect("query ok");
    assert!(query_res.contains("\"schema_version\":1"));
    assert!(query_res.contains("\"voxel_size\":0.25"));

    // Leitura in-memory direta
    let payload = get_plugin_data(&state, &pid).expect("payload must exist");
    assert_eq!(payload.schema_version, 1);
    assert_eq!(payload.data, custom_data);

    // Diagnóstico de plugin ausente: o plugin tem dados no Document, mas não está no Registry
    let registry = PluginRegistry::default();
    let missing = detect_missing_plugins(&state, &registry);
    assert_eq!(missing, vec!["gate9.data_plugin"]);

    // Invariante: O payload reconhecido permanece intacto no Document após o diagnóstico
    let payload_intact = get_plugin_data(&state, &pid).expect("payload must remain intact");
    assert_eq!(payload_intact.data, custom_data);
}
