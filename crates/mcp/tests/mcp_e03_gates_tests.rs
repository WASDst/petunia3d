//! Gates e testes de conformidade para MCP e Automação de Agentes (E03).
//!
//! Website/docs/13-extensions/mcp-automation-agents.md:
//! 1. Aprovações e capabilities por sessão (deny-by-default, grant/revoke dinâmico, session_status).
//! 2. ObjectIds e precondições de revisão (otimismo estrito, stale reference, revision mismatch).
//! 3. Atomic Batch, Dry-run e Rollback total (all-or-nothing, uma entrada de undo, dry run imutável).
//! 4. Redacted Audit Log (privacidade, histórico circular, zero vazamento de args ou paths).
//! 5. Opt-in duplo de Plugins (mcp_exposable exigido + permissão allow_plugins na sessão).
//! 6. Concorrência segura e integridade single-writer (múltiplos callers tokio paralelos).

use std::collections::BTreeSet;

use petunia_mcp::{
    AddPrimitiveParams, BatchParams, ExecuteIntentParams, ExportObjParams, IntentParams, McpPolicy,
    PetuniaMcp, PluginToolParams,
};
use petunia_plugins::{Capability, PluginManifest};
use rmcp::handler::server::wrapper::Parameters;

fn code(err: &rmcp::ErrorData) -> Option<&str> {
    err.data.as_ref()?.get("code")?.as_str()
}

fn unwrap_err<T>(res: Result<T, rmcp::ErrorData>) -> rmcp::ErrorData {
    match res {
        Ok(_) => panic!("expected Err, got Ok"),
        Err(e) => e,
    }
}

// ----------------------------------------------------------------------------
// Gate 1: Capabilities por sessão, deny-by-default e revogação dinâmica
// ----------------------------------------------------------------------------
#[tokio::test]
async fn test_gate1_session_capabilities_deny_by_default_and_dynamic_revocation() {
    let server = PetuniaMcp::new(); // default read_only

    // 1. Consulta session_status inicial
    let status = server.session_status().await.unwrap().0;
    assert!(!status.allow_authoring);
    assert!(status.allow_export);
    assert!(!status.allow_plugins);
    assert!(!status.session_id.is_empty());

    // 2. Mutação é rejeitada com permission_denied
    let add_params = AddPrimitiveParams {
        kind: "cube".to_string(),
        size: 1.0,
        expected_revision: None,
    };
    let err = unwrap_err(server.add_primitive(Parameters(add_params)).await);
    assert_eq!(code(&err), Some("permission_denied"));

    // 3. Concessão dinâmica de authoring
    let mut updated_policy = server.get_policy().await;
    updated_policy.grant_authoring();
    server.update_policy(updated_policy).await;

    // Mutação agora é aceita
    let add_params_allowed = AddPrimitiveParams {
        kind: "cube".to_string(),
        size: 1.0,
        expected_revision: None,
    };
    let added = server
        .add_primitive(Parameters(add_params_allowed))
        .await
        .unwrap()
        .0;
    assert_eq!(added.name, "MCP cube");

    // 4. Revogação de export e authoring
    let mut revoked_policy = server.get_policy().await;
    revoked_policy.revoke_authoring();
    revoked_policy.revoke_export();
    server.update_policy(revoked_policy).await;

    // Mutação bloqueada novamente
    let add_params_blocked = AddPrimitiveParams {
        kind: "cube".to_string(),
        size: 1.0,
        expected_revision: None,
    };
    let err_blocked = unwrap_err(server.add_primitive(Parameters(add_params_blocked)).await);
    assert_eq!(code(&err_blocked), Some("permission_denied"));

    // Exportação bloqueada
    let export_params = ExportObjParams {
        id: Some(added.id),
        index: None,
    };
    let err_export = unwrap_err(server.export_obj(Parameters(export_params)).await);
    assert_eq!(code(&err_export), Some("permission_denied"));
}

// ----------------------------------------------------------------------------
// Gate 2: ObjectIds e Precondições de Revisão
// ----------------------------------------------------------------------------
#[tokio::test]
async fn test_gate2_object_ids_and_revision_preconditions() {
    let server = PetuniaMcp::with_policy(McpPolicy::authoring());
    let initial_rev = server.document_revision().await.unwrap().0;

    // 1. Mutação com expected_revision correto avança o documento
    let add_plane = AddPrimitiveParams {
        kind: "plane".to_string(),
        size: 2.0,
        expected_revision: Some(initial_rev),
    };
    let _added = server.add_primitive(Parameters(add_plane)).await.unwrap().0;

    let new_rev = server.document_revision().await.unwrap().0;
    assert_ne!(new_rev, initial_rev);

    // 2. Mutação com expected_revision obsoleto (stale revision) é rejeitada com revision_mismatch
    let stale_call = AddPrimitiveParams {
        kind: "sphere".to_string(),
        size: 1.5,
        expected_revision: Some(initial_rev),
    };
    let err = unwrap_err(server.add_primitive(Parameters(stale_call)).await);
    assert_eq!(code(&err), Some("revision_mismatch"));

    // Revisão e contagem de assets não mudaram após a rejeição
    assert_eq!(server.document_revision().await.unwrap().0, new_rev);

    // 3. Comando direcionado a ObjectId inexistente falha com stale_reference e não altera a cena
    let stale_target = ExecuteIntentParams {
        intent: IntentParams {
            command: "model.extrude".into(),
            asset_id: Some("00000000-0000-4000-8000-000000000000".into()),
            args: vec![0.5],
        },
        expected_revision: None,
    };
    let report = server
        .execute_intent(Parameters(stale_target))
        .await
        .unwrap()
        .0;
    assert!(!report.valid);
    assert_eq!(report.reason, "stale_reference");
    assert_eq!(server.document_revision().await.unwrap().0, new_rev);
}

// ----------------------------------------------------------------------------
// Gate 3: Atomic Batch, Dry-run e Rollback total
// ----------------------------------------------------------------------------
#[tokio::test]
async fn test_gate3_atomic_batch_dry_run_and_rollback() {
    let server = PetuniaMcp::with_policy(McpPolicy::authoring());
    let initial_assets = server.list_assets().await.unwrap().0.len();
    let initial_rev = server.document_revision().await.unwrap().0;

    let add_cube = IntentParams {
        command: "model.add_primitive".into(),
        asset_id: Some("Cube".into()),
        args: vec![],
    };

    // 1. Dry run valida sem mutar nada
    let dry_batch = BatchParams {
        commands: vec![add_cube.clone(), add_cube.clone()],
        expected_revision: None,
        dry_run: true,
    };
    let dry_report = server.execute_batch(Parameters(dry_batch)).await.unwrap().0;
    assert!(dry_report.dry_run);
    assert_eq!(dry_report.applied, 0);
    assert_eq!(server.list_assets().await.unwrap().0.len(), initial_assets);
    assert_eq!(server.document_revision().await.unwrap().0, initial_rev);

    // 2. Batch atômico com falha em um comando: rollback total
    let failing_batch = BatchParams {
        commands: vec![
            add_cube.clone(),
            IntentParams {
                command: "model.extrude".into(),
                asset_id: Some("00000000-0000-4000-8000-000000000000".into()),
                args: vec![1.0],
            },
        ],
        expected_revision: None,
        dry_run: false,
    };
    let err_batch = unwrap_err(server.execute_batch(Parameters(failing_batch)).await);
    assert_eq!(code(&err_batch), Some("stale_reference"));
    // Nenhum asset foi adicionado
    assert_eq!(server.list_assets().await.unwrap().0.len(), initial_assets);
    assert_eq!(server.document_revision().await.unwrap().0, initial_rev);

    // 3. Batch atômico bem-sucedido gera exatamente 1 entrada de Undo
    let success_batch = BatchParams {
        commands: vec![add_cube.clone(), add_cube.clone()],
        expected_revision: None,
        dry_run: false,
    };
    let success_report = server
        .execute_batch(Parameters(success_batch))
        .await
        .unwrap()
        .0;
    assert_eq!(success_report.applied, 2);
    assert_eq!(
        server.list_assets().await.unwrap().0.len(),
        initial_assets + 2
    );

    // Um único undo reverte ambos os comandos
    let undone = server.undo().await.unwrap().0;
    assert!(undone);
    assert_eq!(server.list_assets().await.unwrap().0.len(), initial_assets);
}

// ----------------------------------------------------------------------------
// Gate 4: Redacted Audit Log (zero vazamento de args ou paths)
// ----------------------------------------------------------------------------
#[tokio::test]
async fn test_gate4_redacted_audit_log_never_leaks_args_or_secrets() {
    let server = PetuniaMcp::with_policy(McpPolicy::authoring());
    let secret_arg = 987.654321;

    let intent_with_arg = ExecuteIntentParams {
        intent: IntentParams {
            command: "model.extrude".into(),
            asset_id: None,
            args: vec![secret_arg],
        },
        expected_revision: None,
    };
    let _ = server.execute_intent(Parameters(intent_with_arg)).await;

    let audit_log = server.audit_log().await.unwrap().0;
    let audit_json = serde_json::to_string(&audit_log).unwrap();

    // Invariante: Argumentos numéricos ou dados sensíveis nunca constam no audit log
    assert!(!audit_json.contains("987.654"));
    assert!(!audit_json.contains("secret"));

    // Mas a operação e o comando estão devidamente auditados
    assert!(audit_log.iter().any(|e| e.operation == "execute_intent"));
}

// ----------------------------------------------------------------------------
// Gate 5: Opt-in duplo de Plugins para automação MCP
// ----------------------------------------------------------------------------
#[tokio::test]
async fn test_gate5_double_opt_in_for_plugin_mcp_tools() {
    let server = PetuniaMcp::with_policy(McpPolicy::authoring());

    // Manifest 1: Plugin NÃO tem Capability::McpExposable
    let unexposed_manifest = PluginManifest {
        plugin_id: "pkg.private_tools".to_string(),
        version: "1.0.0".to_string(),
        api_version: 1,
        name: "Private Tools".to_string(),
        author: "Acme".to_string(),
        source: String::new(),
        entrypoint: "main.lua".to_string(),
        required_capabilities: BTreeSet::from([Capability::ReadDocument]),
        optional_capabilities: BTreeSet::new(),
        commands: vec!["model.add_primitive".to_string()],
        queries: vec!["scene.list".to_string()],
        allow_destructive: false,
    };
    server.register_plugin(&unexposed_manifest).await;

    // Manifest 2: Plugin TEM Capability::McpExposable
    let exposable_manifest = PluginManifest {
        plugin_id: "pkg.public_tools".to_string(),
        version: "1.0.0".to_string(),
        api_version: 1,
        name: "Public Tools".to_string(),
        author: "Acme".to_string(),
        source: String::new(),
        entrypoint: "main.lua".to_string(),
        required_capabilities: BTreeSet::from([Capability::ReadDocument, Capability::McpExposable]),
        optional_capabilities: BTreeSet::new(),
        commands: vec!["model.add_primitive".to_string()],
        queries: vec!["scene.list".to_string()],
        allow_destructive: false,
    };
    server.register_plugin(&exposable_manifest).await;

    // 1. Invocação de plugin não-exposable é rejeitada com plugin_not_exposable
    let call_unexposed = PluginToolParams {
        plugin_id: "pkg.private_tools".to_string(),
        command: "model.add_primitive".to_string(),
        asset_id: Some("Cube".to_string()),
        args: vec![],
        expected_revision: None,
    };
    let err_unexposed = unwrap_err(server.execute_plugin_tool(Parameters(call_unexposed)).await);
    assert_eq!(code(&err_unexposed), Some("plugin_not_exposable"));

    // 2. Invocação de plugin exposable é aceita quando a sessão permite plugins
    let call_exposable = PluginToolParams {
        plugin_id: "pkg.public_tools".to_string(),
        command: "model.add_primitive".to_string(),
        asset_id: Some("Cube".to_string()),
        args: vec![],
        expected_revision: None,
    };
    let report_exposable = server
        .execute_plugin_tool(Parameters(call_exposable))
        .await
        .unwrap()
        .0;
    assert!(report_exposable.success);

    // 3. Se a sessão revogar allow_plugins, a invocação do plugin exposable é negada com permission_denied
    let mut policy = server.get_policy().await;
    policy.revoke_plugins();
    server.update_policy(policy).await;

    let call_revoked = PluginToolParams {
        plugin_id: "pkg.public_tools".to_string(),
        command: "model.add_primitive".to_string(),
        asset_id: Some("Cube".to_string()),
        args: vec![],
        expected_revision: None,
    };
    let err_revoked = unwrap_err(server.execute_plugin_tool(Parameters(call_revoked)).await);
    assert_eq!(code(&err_revoked), Some("permission_denied"));
}

// ----------------------------------------------------------------------------
// Gate 6: Concorrência segura e integridade single-writer
// ----------------------------------------------------------------------------
#[tokio::test]
async fn test_gate6_concurrent_agent_requests_maintain_document_integrity() {
    let server = PetuniaMcp::with_policy(McpPolicy::authoring());
    let initial_count = server.list_assets().await.unwrap().0.len();

    // Spawna 10 tarefas concorrentes adicionando primitivas através de clones do server
    let mut handles = Vec::new();
    for i in 0..10 {
        let s = server.clone();
        handles.push(tokio::spawn(async move {
            let p = AddPrimitiveParams {
                kind: if i % 2 == 0 {
                    "cube".to_string()
                } else {
                    "plane".to_string()
                },
                size: 1.0,
                expected_revision: None,
            };
            s.add_primitive(Parameters(p)).await
        }));
    }

    for h in handles {
        let res = h.await.unwrap();
        assert!(res.is_ok(), "Concurrent request must succeed without panic");
    }

    // A cena avançou exatamente 10 objetos
    assert_eq!(
        server.list_assets().await.unwrap().0.len(),
        initial_count + 10
    );
}
