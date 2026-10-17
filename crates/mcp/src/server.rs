//! Tools-only MCP server over stdio.
//!
//! MCP is an adapter (website/docs/13-extensions/mcp-automation-agents.md):
//! requests validate, then call the Application API (`AppState::dispatch_intent`
//! and `AppState::transaction`). It does not own a parallel Document/Undo.
//!
//! - **Deny by default:** reads and validation are always available; every
//!   mutation needs [`McpPolicy::allow_authoring`], decided by the host.
//! - **ObjectIds and revisions:** DTOs carry stable object ids and the
//!   document revision; mutations accept `expected_revision` and fail with
//!   `revision_mismatch` instead of overwriting newer state.
//! - **Atomic batch / dry run:** `execute_batch` is all-or-nothing with one
//!   Undo entry; `dry_run` validates without touching the Document.
//! - **Distinct errors:** `data.code` is `permission_denied`, `bad_shape`,
//!   `not_allowlisted`, `stale_reference`, `revision_mismatch`, `too_large`
//!   or `command_failed`.
//! - **Audit:** a bounded, redacted log (operation, command, outcome — never
//!   args or paths).

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use petunia_core::{
    AddPrimitiveCmd, AppState, CommandError, CommandIntent, PrimitiveKind, ProjectService,
    SceneItemContract, UndoCmd,
};
use rmcp::ErrorData as McpError;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{ServiceExt, tool, tool_router, transport::stdio};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Max commands in one batch.
pub const MAX_BATCH_COMMANDS: usize = 64;
/// Max retained audit entries.
pub const MAX_AUDIT_ENTRIES: usize = 256;

/// Authorization policy of an MCP session. Everything that mutates the
/// Document is denied unless the host enabled authoring explicitly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct McpPolicy {
    /// Authoring: mutations through Application commands.
    pub allow_authoring: bool,
    /// Export: export scene or meshes.
    pub allow_export: bool,
    /// Plugins: allow invoking tools provided by approved plugins.
    pub allow_plugins: bool,
    /// Stable session identifier.
    pub session_id: String,
}

impl Default for McpPolicy {
    fn default() -> Self {
        Self::read_only()
    }
}

impl McpPolicy {
    /// Read/validate only.
    pub fn read_only() -> Self {
        Self {
            allow_authoring: false,
            allow_export: true,
            allow_plugins: false,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Reads plus undoable authoring through Application commands.
    pub fn authoring() -> Self {
        Self {
            allow_authoring: true,
            allow_export: true,
            allow_plugins: true,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Host-side decision for stdio launches: authoring only when
    /// `PETUNIA_MCP_ALLOW_AUTHORING=1` was set explicitly by the user.
    pub fn from_env() -> Self {
        let allow_authoring = std::env::var("PETUNIA_MCP_ALLOW_AUTHORING").as_deref() == Ok("1");
        let allow_export = std::env::var("PETUNIA_MCP_ALLOW_EXPORT").as_deref() != Ok("0");
        let allow_plugins = std::env::var("PETUNIA_MCP_ALLOW_PLUGINS").as_deref() == Ok("1");
        Self {
            allow_authoring,
            allow_export,
            allow_plugins,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn grant_authoring(&mut self) {
        self.allow_authoring = true;
    }

    pub fn revoke_authoring(&mut self) {
        self.allow_authoring = false;
    }

    pub fn grant_export(&mut self) {
        self.allow_export = true;
    }

    pub fn revoke_export(&mut self) {
        self.allow_export = false;
    }

    pub fn grant_plugins(&mut self) {
        self.allow_plugins = true;
    }

    pub fn revoke_plugins(&mut self) {
        self.allow_plugins = false;
    }
}

/// Status summary of an active MCP session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct SessionStatusDto {
    pub session_id: String,
    pub allow_authoring: bool,
    pub allow_export: bool,
    pub allow_plugins: bool,
    pub revision: u64,
    pub audit_count: usize,
}

/// One redacted audit record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct AuditEntry {
    /// Tool name.
    pub operation: String,
    /// Command id when the tool ran one (never args or paths).
    pub command: Option<String>,
    /// Stable outcome code (`ok`, `dry_run`, or an error code).
    pub outcome: String,
    /// Document revision after the operation.
    pub revision: u64,
}

/// Registration details of an approved plugin in this MCP session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct McpPluginRegistration {
    pub plugin_id: String,
    pub name: String,
    pub is_mcp_exposable: bool,
    pub allowed_commands: Vec<String>,
}

/// Shared Application session behind the tool boundary.
struct McpSession {
    state: AppState,
    policy: McpPolicy,
    audit: VecDeque<AuditEntry>,
    plugins: BTreeMap<String, McpPluginRegistration>,
}

impl McpSession {
    fn new(policy: McpPolicy) -> Self {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        Self {
            state,
            policy,
            audit: VecDeque::new(),
            plugins: BTreeMap::new(),
        }
    }

    fn revision(&self) -> u64 {
        self.state.project.project.revision_id()
    }

    fn record(&mut self, operation: &str, command: Option<&str>, outcome: &str) {
        if self.audit.len() >= MAX_AUDIT_ENTRIES {
            self.audit.pop_front();
        }
        let revision = self.revision();
        self.audit.push_back(AuditEntry {
            operation: operation.to_string(),
            command: command.map(str::to_string),
            outcome: outcome.to_string(),
            revision,
        });
    }

    /// Fails (and audits) when authoring is not allowed.
    fn require_authoring(&mut self, operation: &str) -> Result<(), McpError> {
        if self.policy.allow_authoring {
            return Ok(());
        }
        self.record(operation, None, "permission_denied");
        Err(coded(
            "permission_denied",
            format!("{operation}: authoring is not allowed in this MCP session"),
        ))
    }

    /// Fails (and audits) when export is not allowed.
    fn require_export(&mut self, operation: &str) -> Result<(), McpError> {
        if self.policy.allow_export {
            return Ok(());
        }
        self.record(operation, None, "permission_denied");
        Err(coded(
            "permission_denied",
            format!("{operation}: export is not allowed in this MCP session"),
        ))
    }

    /// Fails (and audits) when plugin tools are not allowed.
    fn require_plugins(&mut self, operation: &str) -> Result<(), McpError> {
        if self.policy.allow_plugins {
            return Ok(());
        }
        self.record(operation, None, "permission_denied");
        Err(coded(
            "permission_denied",
            format!("{operation}: plugin tools are not allowed in this MCP session"),
        ))
    }

    /// Fails (and audits) when the document moved past `expected`.
    fn require_revision(&mut self, operation: &str, expected: Option<u64>) -> Result<(), McpError> {
        let current = self.revision();
        match expected {
            Some(expected) if expected != current => {
                self.record(operation, None, "revision_mismatch");
                Err(coded(
                    "revision_mismatch",
                    format!("expected revision {expected}, document is at {current}"),
                ))
            }
            _ => Ok(()),
        }
    }
}

/// Error with a stable machine-readable code in `data.code`.
fn coded(code: &'static str, message: impl Into<String>) -> McpError {
    McpError::invalid_params(message.into(), Some(serde_json::json!({ "code": code })))
}

/// Stable code for an Application command error.
fn command_error_code(error: &CommandError) -> &'static str {
    match error {
        CommandError::StaleReference(_) | CommandError::InvalidAssetIndex(_) => "stale_reference",
        CommandError::UnknownCommand(_) => "not_allowlisted",
        CommandError::UnknownPrimitive(_) => "bad_shape",
        CommandError::NoChange(_) => "no_change",
        _ => "command_failed",
    }
}

/// Parameters for [`PetuniaMcp::add_primitive`].
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct AddPrimitiveParams {
    /// Primitive kind: `"cube"`, `"plane"`, `"sphere"`, …
    pub kind: String,
    /// Edge size in world units (0.01..=100). Reserved; V1 uses default mesh.
    pub size: f64,
    /// Fail with `revision_mismatch` if the document is not at this revision.
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

/// Parameters for [`PetuniaMcp::export_obj`].
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ExportObjParams {
    /// Asset index in scene order (legacy; prefer `id`).
    #[serde(default)]
    pub index: Option<usize>,
    /// Stable object id (UUID) from `list_assets`.
    #[serde(default)]
    pub id: Option<String>,
}

/// One command intent (validate, execute or batch item).
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct IntentParams {
    /// Command id, e.g. `"model.extrude"`.
    pub command: String,
    /// Optional target object UUID (for `model.add_primitive`: the kind).
    #[serde(default)]
    pub asset_id: Option<String>,
    /// Numeric args in command-defined order.
    #[serde(default)]
    pub args: Vec<f64>,
}

/// Parameters for [`PetuniaMcp::execute_intent`].
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ExecuteIntentParams {
    #[serde(flatten)]
    pub intent: IntentParams,
    /// Fail with `revision_mismatch` if the document is not at this revision.
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

/// Parameters for [`PetuniaMcp::execute_batch`].
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct BatchParams {
    /// Commands applied in order as one transaction (max 64).
    pub commands: Vec<IntentParams>,
    /// Fail with `revision_mismatch` if the document is not at this revision.
    #[serde(default)]
    pub expected_revision: Option<u64>,
    /// Validate everything without changing the Document.
    #[serde(default)]
    pub dry_run: bool,
}

/// Validation report returned by [`PetuniaMcp::validate_intent`].
#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
pub struct IntentReport {
    /// Whether the intent is well-formed and registered.
    pub valid: bool,
    /// Stable reason code (`"ok"`, `"bad_shape"`, `"not_allowlisted"`,
    /// `"stale_reference"`).
    pub reason: String,
    /// Document revision after the call.
    pub revision: u64,
}

/// Result of [`PetuniaMcp::execute_batch`].
#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
pub struct BatchReport {
    /// Commands applied (0 on dry run).
    pub applied: usize,
    pub dry_run: bool,
    /// Document revision after the call.
    pub revision: u64,
}

/// Asset summary DTO returned by scene tools.
#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
pub struct AssetSummaryDto {
    /// Stable object id (UUID).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Always `"object"` on this boundary.
    pub kind: String,
    /// Visibility flag.
    pub visible: bool,
}

/// Parameters for [`PetuniaMcp::execute_plugin_tool`].
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct PluginToolParams {
    /// Target plugin id.
    pub plugin_id: String,
    /// Command to execute on behalf of the plugin.
    pub command: String,
    /// Optional target asset id.
    #[serde(default)]
    pub asset_id: Option<String>,
    /// Command args.
    #[serde(default)]
    pub args: Vec<f64>,
    /// Fail with `revision_mismatch` if the document is not at this revision.
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

/// Result of [`PetuniaMcp::execute_plugin_tool`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct PluginToolReport {
    pub success: bool,
    pub reason: String,
    pub revision: u64,
}

/// Petunia MCP server. Clone shares the same Application session.
#[derive(Clone)]
pub struct PetuniaMcp {
    session: Arc<Mutex<McpSession>>,
}

impl std::fmt::Debug for PetuniaMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PetuniaMcp").finish_non_exhaustive()
    }
}

impl PetuniaMcp {
    /// Creates a **read-only** server with a fresh Application session.
    pub fn new() -> Self {
        Self::with_policy(McpPolicy::read_only())
    }

    /// Creates a server with an explicit authorization policy.
    pub fn with_policy(policy: McpPolicy) -> Self {
        Self {
            session: Arc::new(Mutex::new(McpSession::new(policy))),
        }
    }

    /// Host API: updates the session authorization policy.
    pub async fn update_policy(&self, policy: McpPolicy) {
        let mut session = self.session.lock().await;
        session.policy = policy;
    }

    /// Host API: retrieves the current policy.
    pub async fn get_policy(&self) -> McpPolicy {
        let session = self.session.lock().await;
        session.policy.clone()
    }

    /// Host API: registers an approved plugin in this MCP session.
    pub async fn register_plugin(&self, manifest: &petunia_plugins::PluginManifest) {
        let mut session = self.session.lock().await;
        let is_mcp_exposable = manifest
            .required_capabilities
            .contains(&petunia_plugins::Capability::McpExposable)
            || manifest
                .optional_capabilities
                .contains(&petunia_plugins::Capability::McpExposable);
        session.plugins.insert(
            manifest.plugin_id.clone(),
            McpPluginRegistration {
                plugin_id: manifest.plugin_id.clone(),
                name: manifest.name.clone(),
                is_mcp_exposable,
                allowed_commands: manifest.commands.clone(),
            },
        );
    }

    /// Host API: unregisters a plugin from this MCP session.
    pub async fn unregister_plugin(&self, plugin_id: &str) {
        let mut session = self.session.lock().await;
        session.plugins.remove(plugin_id);
    }

    fn asset_summaries(state: &AppState) -> Vec<AssetSummaryDto> {
        state
            .project
            .assets
            .iter()
            .map(|asset| AssetSummaryDto {
                id: asset.id.to_string(),
                name: asset.name.clone(),
                kind: "object".to_string(),
                visible: asset.visible,
            })
            .collect()
    }

    fn intent_well_formed(command: &str, asset_id: Option<&str>, args: &[f64]) -> bool {
        let asset_ok = asset_id
            .is_none_or(|id| uuid::Uuid::parse_str(id).is_ok() || PrimitiveKind::parse(id).is_ok());
        !command.is_empty()
            && command.len() <= 64
            && command
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
            && args.len() <= 16
            && args.iter().all(|a| a.is_finite())
            && asset_ok
    }

    fn allowlisted(state: &AppState, command: &str) -> bool {
        state.commands.contains(command)
            || matches!(
                command,
                "model.add_primitive" | "model.add_cube" | "model.scale" | "model.scale_selection"
            )
    }

    /// Static checks shared by validate, execute and dry-run: shape,
    /// allowlist and that an explicit target exists. `Err` holds the code.
    fn check_intent(state: &AppState, intent: &IntentParams) -> Result<(), &'static str> {
        if !Self::intent_well_formed(&intent.command, intent.asset_id.as_deref(), &intent.args) {
            return Err("bad_shape");
        }
        if !Self::allowlisted(state, &intent.command) {
            return Err("not_allowlisted");
        }
        let creates = matches!(
            intent.command.as_str(),
            "model.add_primitive" | "model.add_cube"
        );
        if let Some(id) = intent.asset_id.as_deref().filter(|_| !creates) {
            let found = uuid::Uuid::parse_str(id)
                .ok()
                .and_then(|uuid| state.project.find(uuid));
            if found.is_none() {
                return Err("stale_reference");
            }
        }
        Ok(())
    }

    fn to_command_intent(intent: IntentParams) -> CommandIntent {
        CommandIntent {
            command: intent.command,
            asset_id: intent.asset_id,
            args: intent.args,
        }
    }
}

impl Default for PetuniaMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(server_handler)]
impl PetuniaMcp {
    /// Lists scene assets as `{id, name, kind, visible}` DTOs.
    #[tool(description = "List scene assets (id, name, kind, visible).")]
    pub async fn list_assets(&self) -> Result<Json<Vec<AssetSummaryDto>>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(Self::asset_summaries(&session.state)))
    }

    /// Current document revision (for `expected_revision` preconditions).
    #[tool(description = "Current document revision, used as expected_revision precondition.")]
    pub async fn document_revision(&self) -> Result<Json<u64>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(session.revision()))
    }

    /// Adds a primitive through the Application command spine.
    #[tool(description = "Add a primitive to the scene (undoable; requires authoring).")]
    pub async fn add_primitive(
        &self,
        Parameters(params): Parameters<AddPrimitiveParams>,
    ) -> Result<Json<AssetSummaryDto>, McpError> {
        if !params.size.is_finite() || !(0.01..=100.0).contains(&params.size) {
            return Err(coded("bad_shape", "size must be finite within 0.01..=100"));
        }
        let kind = PrimitiveKind::parse(&params.kind).map_err(|e| {
            coded(
                "bad_shape",
                format!("unknown primitive '{}': {e}", params.kind),
            )
        })?;
        let mut session = self.session.lock().await;
        session.require_authoring("add_primitive")?;
        session.require_revision("add_primitive", params.expected_revision)?;
        let result = session.state.dispatch(&AddPrimitiveCmd {
            kind,
            name: Some(format!("MCP {}", params.kind)),
            at_cursor: true,
        });
        if let Err(error) = result {
            let code = command_error_code(&error);
            session.record("add_primitive", Some("model.add_primitive"), code);
            return Err(coded(code, error.to_string()));
        }
        session.record("add_primitive", Some("model.add_primitive"), "ok");
        let asset = session
            .state
            .project
            .active()
            .ok_or_else(|| coded("command_failed", "no active asset after add"))?;
        Ok(Json(AssetSummaryDto {
            id: asset.id.to_string(),
            name: asset.name.clone(),
            kind: "object".to_string(),
            visible: asset.visible,
        }))
    }

    /// Undoes the last Application mutation.
    #[tool(description = "Undo the last scene mutation (requires authoring).")]
    pub async fn undo(&self) -> Result<Json<bool>, McpError> {
        let mut session = self.session.lock().await;
        session.require_authoring("undo")?;
        let undone = session.state.dispatch(&UndoCmd).is_ok();
        session.record(
            "undo",
            Some("global.undo"),
            if undone { "ok" } else { "no_change" },
        );
        Ok(Json(undone))
    }

    /// Exports one asset as OBJ text (bounded by scene content).
    #[tool(description = "Export one scene asset as OBJ text by id (or legacy index).")]
    pub async fn export_obj(
        &self,
        Parameters(params): Parameters<ExportObjParams>,
    ) -> Result<String, McpError> {
        let mut session = self.session.lock().await;
        session.require_export("export_obj")?;
        let assets = &session.state.project.assets;
        let asset = match (&params.id, params.index) {
            (Some(id), _) => uuid::Uuid::parse_str(id)
                .ok()
                .and_then(|uuid| assets.iter().find(|a| a.id == uuid))
                .ok_or_else(|| coded("stale_reference", format!("no object with id {id}")))?,
            (None, Some(index)) => assets.get(index).ok_or_else(|| {
                coded(
                    "stale_reference",
                    format!("asset index {index} out of range"),
                )
            })?,
            (None, None) => return Err(coded("bad_shape", "pass `id` or `index`")),
        };
        Ok(petunia_project::export::export_obj(asset))
    }

    /// Validates a command intent against the Application registry without executing it.
    #[tool(description = "Validate a command intent (id, target, args) without executing.")]
    pub async fn validate_intent(
        &self,
        Parameters(params): Parameters<IntentParams>,
    ) -> Result<Json<IntentReport>, McpError> {
        let session = self.session.lock().await;
        let (valid, reason) = match Self::check_intent(&session.state, &params) {
            Ok(()) => (true, "ok"),
            Err(code) => (false, code),
        };
        Ok(Json(IntentReport {
            valid,
            reason: reason.to_string(),
            revision: session.revision(),
        }))
    }

    /// Executes a validated command intent through the Application API.
    #[tool(
        description = "Execute a command intent through the Application command spine (requires authoring)."
    )]
    pub async fn execute_intent(
        &self,
        Parameters(params): Parameters<ExecuteIntentParams>,
    ) -> Result<Json<IntentReport>, McpError> {
        let mut session = self.session.lock().await;
        if let Err(code) = Self::check_intent(&session.state, &params.intent) {
            session.record("execute_intent", Some(params.intent.command.as_str()), code);
            return Ok(Json(IntentReport {
                valid: false,
                reason: code.to_string(),
                revision: session.revision(),
            }));
        }
        session.require_authoring("execute_intent")?;
        session.require_revision("execute_intent", params.expected_revision)?;
        let command = params.intent.command.clone();
        let intent = Self::to_command_intent(params.intent);
        match session.state.dispatch_intent(&intent) {
            Ok(()) => {
                session.record("execute_intent", Some(command.as_str()), "ok");
                Ok(Json(IntentReport {
                    valid: true,
                    reason: "ok".to_string(),
                    revision: session.revision(),
                }))
            }
            Err(error) => {
                let code = command_error_code(&error);
                session.record("execute_intent", Some(command.as_str()), code);
                Err(coded(code, error.to_string()))
            }
        }
    }

    /// Atomic batch: all commands as one transaction and one Undo entry, or
    /// nothing. `dry_run` validates only.
    #[tool(
        description = "Run up to 64 command intents atomically (one Undo entry, all-or-nothing); dry_run validates only."
    )]
    pub async fn execute_batch(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<Json<BatchReport>, McpError> {
        let mut session = self.session.lock().await;
        if params.commands.is_empty() || params.commands.len() > MAX_BATCH_COMMANDS {
            session.record("execute_batch", None, "too_large");
            return Err(coded(
                "too_large",
                format!("batch must have 1..={MAX_BATCH_COMMANDS} commands"),
            ));
        }
        for intent in &params.commands {
            if let Err(code) = Self::check_intent(&session.state, intent) {
                session.record("execute_batch", Some(intent.command.as_str()), code);
                return Err(coded(code, format!("{}: {code}", intent.command)));
            }
        }
        session.require_revision("execute_batch", params.expected_revision)?;
        if params.dry_run {
            session.record("execute_batch", None, "dry_run");
            return Ok(Json(BatchReport {
                applied: 0,
                dry_run: true,
                revision: session.revision(),
            }));
        }
        session.require_authoring("execute_batch")?;
        let count = params.commands.len();
        let intents: Vec<CommandIntent> = params
            .commands
            .into_iter()
            .map(Self::to_command_intent)
            .collect();
        let result = session.state.transaction("mcp batch", |state| {
            for intent in &intents {
                match state.dispatch_intent(intent) {
                    Ok(()) | Err(CommandError::NoChange(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        });
        match result {
            Ok(()) => {
                session.record("execute_batch", None, "ok");
                Ok(Json(BatchReport {
                    applied: count,
                    dry_run: false,
                    revision: session.revision(),
                }))
            }
            Err(error) => {
                let code = command_error_code(&error);
                session.record("execute_batch", None, code);
                Err(coded(code, error.to_string()))
            }
        }
    }

    /// Redacted audit log of this session (newest last).
    #[tool(description = "Redacted audit log of this MCP session (operation, command, outcome).")]
    pub async fn audit_log(&self) -> Result<Json<Vec<AuditEntry>>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(session.audit.iter().cloned().collect()))
    }

    /// Exposes the scene as query DTOs (mirrors Application Queries shape).
    #[tool(description = "Scene items as query DTOs (name, kind, visible).")]
    pub async fn scene_items(&self) -> Result<Json<Vec<SceneItemContract>>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(
            session
                .state
                .query_scene_hierarchy()
                .assets
                .into_iter()
                .map(|asset| SceneItemContract {
                    name: asset.name,
                    kind: "object".to_string(),
                    visible: asset.visible,
                })
                .collect(),
        ))
    }

    /// Current session status (session_id, permissions, revision, audit count).
    #[tool(description = "Get status and capabilities of current MCP session.")]
    pub async fn session_status(&self) -> Result<Json<SessionStatusDto>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(SessionStatusDto {
            session_id: session.policy.session_id.clone(),
            allow_authoring: session.policy.allow_authoring,
            allow_export: session.policy.allow_export,
            allow_plugins: session.policy.allow_plugins,
            revision: session.revision(),
            audit_count: session.audit.len(),
        }))
    }

    /// Execute a tool provided by an approved mcp_exposable plugin.
    #[tool(
        description = "Execute an approved plugin tool (requires allow_plugins and mcp_exposable)."
    )]
    pub async fn execute_plugin_tool(
        &self,
        Parameters(params): Parameters<PluginToolParams>,
    ) -> Result<Json<PluginToolReport>, McpError> {
        let mut session = self.session.lock().await;
        session.require_plugins("execute_plugin_tool")?;
        session.require_revision("execute_plugin_tool", params.expected_revision)?;

        let reg = match session.plugins.get(&params.plugin_id) {
            Some(r) => r.clone(),
            None => {
                session.record("execute_plugin_tool", None, "not_installed");
                return Err(coded(
                    "not_installed",
                    format!(
                        "plugin '{}' is not registered in this MCP session",
                        params.plugin_id
                    ),
                ));
            }
        };

        if !reg.is_mcp_exposable {
            session.record("execute_plugin_tool", None, "plugin_not_exposable");
            return Err(coded(
                "plugin_not_exposable",
                format!("plugin '{}' is not marked mcp_exposable", params.plugin_id),
            ));
        }

        if !reg.allowed_commands.contains(&params.command) {
            session.record(
                "execute_plugin_tool",
                Some(&params.command),
                "not_allowlisted",
            );
            return Err(coded(
                "not_allowlisted",
                format!(
                    "command '{}' is not exposed by plugin '{}'",
                    params.command, params.plugin_id
                ),
            ));
        }

        session.require_authoring("execute_plugin_tool")?;
        let cmd_intent = CommandIntent {
            command: params.command.clone(),
            asset_id: params.asset_id,
            args: params.args,
        };

        match session.state.dispatch_intent(&cmd_intent) {
            Ok(()) => {
                session.record("execute_plugin_tool", Some(&params.command), "ok");
                Ok(Json(PluginToolReport {
                    success: true,
                    reason: "ok".to_string(),
                    revision: session.revision(),
                }))
            }
            Err(err) => {
                let code = command_error_code(&err);
                session.record("execute_plugin_tool", Some(&params.command), code);
                Err(coded(code, err.to_string()))
            }
        }
    }
}

/// Serves the Petunia MCP server over stdio with an explicit policy. The
/// Tokio runtime stays inside this call; cancel by closing the transport.
pub async fn serve_stdio_with(policy: McpPolicy) -> Result<(), McpError> {
    let service = PetuniaMcp::with_policy(policy)
        .serve(stdio())
        .await
        .map_err(|e| McpError::internal_error(format!("serve: {e}"), None))?;
    service
        .waiting()
        .await
        .map_err(|e| McpError::internal_error(format!("waiting: {e}"), None))?;
    Ok(())
}

/// [`serve_stdio_with`] using the host decision from the environment
/// ([`McpPolicy::from_env`]): read-only unless authoring was enabled.
pub async fn serve_stdio() -> Result<(), McpError> {
    serve_stdio_with(McpPolicy::from_env()).await
}

/// Blocking stdio entry for embedding binaries: builds a current-thread
/// Tokio runtime internally, so hosts never name a Tokio type.
pub fn serve_stdio_blocking() -> Result<(), McpError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| McpError::internal_error(format!("runtime: {e}"), None))?
        .block_on(serve_stdio())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn author() -> PetuniaMcp {
        PetuniaMcp::with_policy(McpPolicy::authoring())
    }

    fn code(error: &McpError) -> Option<&str> {
        error.data.as_ref()?.get("code")?.as_str()
    }

    fn intent(command: &str, asset_id: Option<&str>, args: Vec<f64>) -> IntentParams {
        IntentParams {
            command: command.to_string(),
            asset_id: asset_id.map(str::to_string),
            args,
        }
    }

    #[tokio::test]
    async fn list_starts_with_starter_scene_and_ids() {
        let server = PetuniaMcp::new();
        let items = server.list_assets().await.unwrap().0;
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i.kind == "object"));
        assert!(items.iter().all(|i| uuid::Uuid::parse_str(&i.id).is_ok()));
    }

    #[tokio::test]
    async fn mutations_are_denied_by_default() {
        let server = PetuniaMcp::new();
        let before = server.list_assets().await.unwrap().0.len();
        let error = server
            .add_primitive(Parameters(AddPrimitiveParams {
                kind: "cube".to_string(),
                size: 2.0,
                expected_revision: None,
            }))
            .await
            .err()
            .expect("expected an error");
        assert_eq!(code(&error), Some("permission_denied"));
        assert!(server.undo().await.is_err());
        assert_eq!(server.list_assets().await.unwrap().0.len(), before);
        let log = server.audit_log().await.unwrap().0;
        assert!(log.iter().all(|e| e.outcome == "permission_denied"));
    }

    #[tokio::test]
    async fn add_undo_roundtrip() {
        let server = author();
        let before = server.list_assets().await.unwrap().0.len();
        let added = server
            .add_primitive(Parameters(AddPrimitiveParams {
                kind: "cube".to_string(),
                size: 2.0,
                expected_revision: None,
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(added.name, "MCP cube");
        assert_eq!(server.list_assets().await.unwrap().0.len(), before + 1);
        assert!(server.undo().await.unwrap().0);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before);
        assert!(!server.undo().await.unwrap().0);
    }

    #[tokio::test]
    async fn invalid_primitive_params_are_rejected() {
        let server = author();
        for (kind, size) in [("not-a-prim", 1.0), ("cube", f64::NAN)] {
            let error = server
                .add_primitive(Parameters(AddPrimitiveParams {
                    kind: kind.to_string(),
                    size,
                    expected_revision: None,
                }))
                .await
                .err()
                .expect("expected an error");
            assert_eq!(code(&error), Some("bad_shape"));
        }
        let missing = server
            .export_obj(Parameters(ExportObjParams {
                index: Some(9999),
                id: None,
            }))
            .await
            .expect_err("expected an error");
        assert_eq!(code(&missing), Some("stale_reference"));
    }

    #[tokio::test]
    async fn export_by_id_and_validate_codes() {
        let server = PetuniaMcp::new();
        let first = server.list_assets().await.unwrap().0.remove(0);
        let obj = server
            .export_obj(Parameters(ExportObjParams {
                index: None,
                id: Some(first.id.clone()),
            }))
            .await
            .unwrap();
        assert!(obj.contains('v'));

        for (params, want) in [
            (intent("model.extrude", None, vec![0.5]), "ok"),
            (intent("model.frobnicator", None, vec![]), "not_allowlisted"),
            (intent("rm -rf", None, vec![]), "bad_shape"),
            (
                intent(
                    "model.extrude",
                    Some("00000000-0000-4000-8000-000000000000"),
                    vec![0.5],
                ),
                "stale_reference",
            ),
        ] {
            let report = server.validate_intent(Parameters(params)).await.unwrap().0;
            assert_eq!(report.reason, want);
            assert_eq!(report.valid, want == "ok");
        }
    }

    #[tokio::test]
    async fn execute_intent_uses_application_commands_and_revisions() {
        let server = author();
        let before = server.list_assets().await.unwrap().0.len();
        let revision = server.document_revision().await.unwrap().0;
        let report = server
            .execute_intent(Parameters(ExecuteIntentParams {
                intent: intent("model.add_primitive", Some("Plane"), vec![]),
                expected_revision: Some(revision),
            }))
            .await
            .unwrap()
            .0;
        assert!(report.valid);
        assert_ne!(report.revision, revision);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before + 1);

        // A precondição antiga agora falha sem aplicar nada.
        let stale = server
            .execute_intent(Parameters(ExecuteIntentParams {
                intent: intent("model.add_primitive", Some("Plane"), vec![]),
                expected_revision: Some(revision),
            }))
            .await
            .err()
            .expect("expected an error");
        assert_eq!(code(&stale), Some("revision_mismatch"));
        assert_eq!(server.list_assets().await.unwrap().0.len(), before + 1);
    }

    #[tokio::test]
    async fn batches_are_atomic_with_one_undo_entry_and_dry_run_changes_nothing() {
        let server = author();
        let before = server.list_assets().await.unwrap().0.len();
        let add = intent("model.add_primitive", Some("Plane"), vec![]);

        let dry = server
            .execute_batch(Parameters(BatchParams {
                commands: vec![add.clone(), add.clone()],
                expected_revision: None,
                dry_run: true,
            }))
            .await
            .unwrap()
            .0;
        assert!(dry.dry_run);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before);

        let report = server
            .execute_batch(Parameters(BatchParams {
                commands: vec![add.clone(), add.clone()],
                expected_revision: None,
                dry_run: false,
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(report.applied, 2);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before + 2);
        assert!(server.undo().await.unwrap().0);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before);

        // Um alvo obsoleto recusa o batch inteiro antes de aplicar qualquer coisa.
        let error = server
            .execute_batch(Parameters(BatchParams {
                commands: vec![
                    add,
                    intent(
                        "model.extrude",
                        Some("00000000-0000-4000-8000-000000000000"),
                        vec![0.5],
                    ),
                ],
                expected_revision: None,
                dry_run: false,
            }))
            .await
            .err()
            .expect("expected an error");
        assert_eq!(code(&error), Some("stale_reference"));
        assert_eq!(server.list_assets().await.unwrap().0.len(), before);
    }

    #[tokio::test]
    async fn audit_never_records_args() {
        let server = author();
        let _ = server
            .execute_intent(Parameters(ExecuteIntentParams {
                intent: intent("model.extrude", None, vec![0.123456]),
                expected_revision: None,
            }))
            .await;
        let log = server.audit_log().await.unwrap().0;
        let text = serde_json::to_string(&log).unwrap();
        assert!(!text.contains("0.123456"));
        assert!(
            log.iter()
                .any(|e| e.command.as_deref() == Some("model.extrude"))
        );
    }
}
