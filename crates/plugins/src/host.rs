//! Capability-limited Lua 5.4 host.
//!
//! See crate docs for the trust model. This file implements declaration,
//! validation, sandbox construction and budgeted execution.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use mlua::{HookTriggers, Lua, StdLib, VmState};
use serde::{Deserialize, Serialize};

/// Plugin API version this host implements. Scripts declaring anything else
/// are rejected before execution.
pub const PLUGIN_API_VERSION: u32 = 1;
/// Max script source size (64 KiB).
pub const MAX_SCRIPT_BYTES: usize = 64 * 1024;
/// Max VM instructions per execution (instruction budget / timeout strategy).
pub const MAX_INSTRUCTIONS: u32 = 1_000_000;
/// Max heap bytes per execution.
pub const MAX_MEMORY_BYTES: usize = 8 * 1024 * 1024;
/// Max recorded intents per execution.
pub const MAX_INTENTS: usize = 256;

/// Stable plugin identity: `1..=64` chars of `[a-zA-Z0-9._-]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(String);

impl PluginId {
    /// Validates the identity string.
    pub fn new(id: &str) -> Result<Self, CapabilityError> {
        if id.is_empty() || id.len() > 64 {
            return Err(CapabilityError::BadIdentity(id.to_string()));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(CapabilityError::BadIdentity(id.to_string()));
        }
        Ok(Self(id.to_string()))
    }

    /// The validated identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Declared capabilities. Everything defaults to denied; the script gets
/// exactly what is listed here and nothing else.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilities {
    /// Required [`PLUGIN_API_VERSION`].
    pub api_version: u32,
    /// Command ids the script may request (validated at call time).
    #[serde(default)]
    pub commands: Vec<String>,
    /// Query names the script may request.
    #[serde(default)]
    pub queries: Vec<String>,
    /// Read-only filesystem roots (reserved: no fs access is granted yet;
    /// roots are validated and recorded for the future path).
    #[serde(default)]
    pub filesystem_roots: Vec<PathBuf>,
    /// Network policy: always denied (no socket library exists in the sandbox).
    #[serde(default)]
    pub allow_network: bool,
    /// Destructive operations (delete, overwrite) policy.
    #[serde(default)]
    pub allow_destructive: bool,
}

impl PluginCapabilities {
    /// Validates declarations: API version, id shapes, root sanity.
    pub fn validate(&self) -> Result<(), CapabilityError> {
        if self.api_version != PLUGIN_API_VERSION {
            return Err(CapabilityError::UnsupportedApi(self.api_version));
        }
        if self.commands.len() > 64 || self.queries.len() > 64 {
            return Err(CapabilityError::TooManyDeclarations);
        }
        for cmd in &self.commands {
            if !is_command_shape(cmd) {
                return Err(CapabilityError::UnknownCommand(cmd.clone()));
            }
            if is_destructive(cmd) && !self.allow_destructive {
                return Err(CapabilityError::DestructiveDenied(cmd.clone()));
            }
        }
        for query in &self.queries {
            if !is_command_shape(query) {
                return Err(CapabilityError::UnknownQuery(query.clone()));
            }
        }
        for root in &self.filesystem_roots {
            if !root.is_absolute() {
                return Err(CapabilityError::RelativeRoot(root.display().to_string()));
            }
        }
        if self.allow_network {
            return Err(CapabilityError::NetworkDenied);
        }
        Ok(())
    }
}

fn is_command_shape(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_'))
}

fn is_destructive(id: &str) -> bool {
    id.contains("delete") || id.contains("discard") || id.contains("overwrite")
}

/// Capability declaration failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    /// Bad plugin identity string.
    #[error("bad plugin identity '{0}'")]
    BadIdentity(String),
    /// Unsupported plugin API version.
    #[error("unsupported plugin API version {0} (host speaks {PLUGIN_API_VERSION})")]
    UnsupportedApi(u32),
    /// Malformed command id.
    #[error("unknown command '{0}'")]
    UnknownCommand(String),
    /// Malformed query name.
    #[error("unknown query '{0}'")]
    UnknownQuery(String),
    /// Destructive command without `allow_destructive`.
    #[error("destructive command '{0}' denied by policy")]
    DestructiveDenied(String),
    /// Relative filesystem root.
    #[error("filesystem root must be absolute: '{0}'")]
    RelativeRoot(String),
    /// Network can never be granted.
    #[error("network access is always denied")]
    NetworkDenied,
    /// Declaration lists too long.
    #[error("too many capability declarations")]
    TooManyDeclarations,
    /// Manifest malformado (versão, nome, entrypoint, capabilities).
    #[error("bad manifest: {0}")]
    BadManifest(String),
    /// Command/query declarado sem capability que o cubra.
    #[error("'{0}' is not covered by any declared capability")]
    NotCovered(String),
}

/// A command request recorded from script execution, validated but not yet
/// dispatched. Dispatch (permissions, undo, validation) stays with the app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LuaCommand {
    /// Command id, member of declared `commands`.
    pub command: String,
    /// Numeric args in command-defined order.
    pub args: Vec<f64>,
    /// Objeto alvo (id estável). Ausente = objeto ativo; id inexistente é
    /// erro `StaleReference` no dispatch, nunca outro objeto.
    #[serde(default)]
    pub target: Option<String>,
}

/// Validated script output: intents to dispatch + query names to answer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidatedIntent {
    /// Recorded command requests, in script order.
    pub commands: Vec<LuaCommand>,
    /// Requested query names, in script order.
    pub queries: Vec<String>,
}

/// Execution failure.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Capability declaration invalid.
    #[error("capability: {0}")]
    Capability(#[from] CapabilityError),
    /// Script-level failure (syntax, runtime, budget).
    #[error("lua: {0}")]
    Lua(String),
    /// Instruction budget exhausted (timeout strategy).
    #[error("instruction budget exhausted")]
    Budget,
    /// Too many recorded intents.
    #[error("intent limit exceeded")]
    TooManyIntents,
    /// Command/query pedido sem capability concedida (ou já revogada).
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// Reentrada: um handler tentou disparar outra entrega no mesmo plugin.
    #[error("plugin is busy")]
    Busy,
}

/// Host configuration: budgets shared by every execution.
#[derive(Clone, Debug)]
pub struct HostConfig {
    /// Max VM instructions per execution.
    pub max_instructions: u32,
    /// Max heap bytes per execution.
    pub max_memory_bytes: usize,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            max_instructions: MAX_INSTRUCTIONS,
            max_memory_bytes: MAX_MEMORY_BYTES,
        }
    }
}

/// The Lua plugin host. Stateless across executions: build once, execute many.
#[derive(Clone, Debug, Default)]
pub struct Host {
    config: HostConfig,
}

impl Host {
    /// Creates a host with default budgets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a host with explicit budgets.
    pub fn with_config(config: HostConfig) -> Self {
        Self { config }
    }

    /// Validates declarations, runs `source` in a sandbox with budgets, and
    /// returns the validated intents. Never panics on hostile scripts.
    pub fn execute(
        &self,
        id: &PluginId,
        capabilities: &PluginCapabilities,
        source: &str,
    ) -> Result<ValidatedIntent, PluginError> {
        capabilities.validate()?;
        let sandbox = Sandbox::new(&self.config, Rc::new(RefCell::new(capabilities.clone())))?;
        sandbox.run_chunk(id, source)?;
        Ok(sandbox.take_output())
    }

    /// Budgets em uso.
    pub fn config(&self) -> &HostConfig {
        &self.config
    }
}

/// Endurece a stdlib carregada antes de qualquer código de plugin.
const SANDBOX_PRELUDE: &str = r#"
local raw_load = load
load = function(chunk, name, _mode, env)
    return raw_load(chunk, name, "t", env)
end
string.dump = nil
"#;

/// Prefixo das recusas de capability dentro do Lua (mapeadas para
/// [`PluginError::PermissionDenied`]).
const DENIED: &str = "permission denied: ";

/// Um Lua State isolado com a API `petunia` instalada e budgets por entrada.
/// Usado tanto pela execução avulsa quanto pela instância persistente.
pub(crate) struct Sandbox {
    pub(crate) lua: Lua,
    spent: Rc<RefCell<u32>>,
    intents: Rc<RefCell<Vec<LuaCommand>>>,
    queries: Rc<RefCell<Vec<String>>>,
    pub(crate) capabilities: Rc<RefCell<PluginCapabilities>>,
}

impl Sandbox {
    pub(crate) fn new(
        config: &HostConfig,
        capabilities: Rc<RefCell<PluginCapabilities>>,
    ) -> Result<Self, PluginError> {
        let libs = StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH;
        let lua =
            Lua::new_with(libs, Default::default()).map_err(|e| PluginError::Lua(e.to_string()))?;
        lua.set_memory_limit(config.max_memory_bytes)
            .map_err(|e| PluginError::Lua(e.to_string()))?;

        let spent = Rc::new(RefCell::new(0u32));
        let budget = config.max_instructions;
        let spent_hook = Rc::clone(&spent);
        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(10_000),
                ..Default::default()
            },
            move |_, _| {
                let mut used = spent_hook.borrow_mut();
                *used = used.saturating_add(10_000);
                if *used > budget {
                    return Err(mlua::Error::RuntimeError(
                        "instruction budget exhausted".into(),
                    ));
                }
                Ok(VmState::Continue)
            },
        )
        .map_err(|e| PluginError::Lua(e.to_string()))?;

        let intents: Rc<RefCell<Vec<LuaCommand>>> = Rc::new(RefCell::new(Vec::new()));
        let queries: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let petunia = lua
            .create_table()
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        {
            let intents = Rc::clone(&intents);
            let caps = Rc::clone(&capabilities);
            let record =
                lua
                    .create_function(
                        move |_,
                              (command, args, target): (
                            String,
                            Option<Vec<f64>>,
                            Option<String>,
                        )| {
                            if !caps.borrow().commands.iter().any(|c| c == &command) {
                                return Err(mlua::Error::RuntimeError(format!(
                                    "{DENIED}{command}"
                                )));
                            }
                            let mut guard = intents.borrow_mut();
                            if guard.len() >= MAX_INTENTS {
                                return Err(mlua::Error::RuntimeError(
                                    "intent limit exceeded".into(),
                                ));
                            }
                            guard.push(LuaCommand {
                                command,
                                args: args.unwrap_or_default(),
                                target,
                            });
                            Ok(true)
                        },
                    )
                    .map_err(|e| PluginError::Lua(e.to_string()))?;
            petunia
                .set("command", record)
                .map_err(|e| PluginError::Lua(e.to_string()))?;
        }
        {
            let queries = Rc::clone(&queries);
            let caps = Rc::clone(&capabilities);
            let ask = lua
                .create_function(move |_, name: String| {
                    if !caps.borrow().queries.iter().any(|q| q == &name) {
                        return Err(mlua::Error::RuntimeError(format!("{DENIED}{name}")));
                    }
                    queries.borrow_mut().push(name);
                    Ok("{}".to_string())
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            petunia
                .set("query", ask)
                .map_err(|e| PluginError::Lua(e.to_string()))?;
        }
        lua.globals()
            .set("petunia", petunia)
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        // O "safe mode" do mlua não impede `load` de aceitar bytecode, e
        // bytecode malicioso derruba a VM: só texto, e sem `string.dump`.
        lua.load(SANDBOX_PRELUDE)
            .set_name("=petunia-sandbox")
            .set_mode(mlua::chunk::ChunkMode::Text)
            .exec()
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        Ok(Self {
            lua,
            spent,
            intents,
            queries,
            capabilities,
        })
    }

    /// Zera o budget de instruções para uma nova entrada no plugin.
    pub(crate) fn reset_budget(&self) {
        *self.spent.borrow_mut() = 0;
    }

    /// Executa um chunk de código-fonte (texto, nunca bytecode) com budget novo.
    pub(crate) fn run_chunk(&self, id: &PluginId, source: &str) -> Result<(), PluginError> {
        if source.len() > MAX_SCRIPT_BYTES {
            return Err(PluginError::Lua("script exceeds size limit".to_string()));
        }
        self.reset_budget();
        self.lua
            .load(source)
            .set_name(format!("@{}", id.as_str()))
            .set_mode(mlua::chunk::ChunkMode::Text)
            .exec()
            .map_err(|e| map_lua_error(&e))
    }

    /// Intents e queries gravados desde a última coleta.
    pub(crate) fn take_output(&self) -> ValidatedIntent {
        ValidatedIntent {
            commands: std::mem::take(&mut *self.intents.borrow_mut()),
            queries: std::mem::take(&mut *self.queries.borrow_mut()),
        }
    }
}

pub(crate) fn map_lua_error(error: &mlua::Error) -> PluginError {
    let text = error.to_string();
    if text.contains("instruction budget exhausted") {
        PluginError::Budget
    } else if let Some(rest) = text.split(DENIED).nth(1) {
        let what = rest
            .split(|c: char| c.is_whitespace() || c == '\n')
            .next()
            .unwrap_or(rest);
        PluginError::PermissionDenied(what.to_string())
    } else if text.contains("intent limit exceeded") {
        PluginError::TooManyIntents
    } else {
        PluginError::Lua(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> PluginCapabilities {
        PluginCapabilities {
            api_version: PLUGIN_API_VERSION,
            commands: vec!["model.extrude".to_string(), "global.undo".to_string()],
            queries: vec!["scene.list".to_string()],
            filesystem_roots: vec![],
            allow_network: false,
            allow_destructive: false,
        }
    }

    fn id() -> PluginId {
        PluginId::new("test.plugin").unwrap()
    }

    #[test]
    fn valid_script_records_intents_in_order() {
        let host = Host::new();
        let out = host
            .execute(
                &id(),
                &capabilities(),
                r#"
                petunia.command("model.extrude", { 0.5 })
                petunia.command("global.undo")
                local scene = petunia.query("scene.list")
                "#,
            )
            .unwrap();
        assert_eq!(
            out.commands,
            vec![
                LuaCommand {
                    command: "model.extrude".to_string(),
                    args: vec![0.5],
                    target: None,
                },
                LuaCommand {
                    command: "global.undo".to_string(),
                    args: vec![],
                    target: None,
                },
            ]
        );
        assert_eq!(out.queries, vec!["scene.list".to_string()]);
    }

    #[test]
    fn ungranted_command_is_rejected_at_call_time() {
        let host = Host::new();
        let err = host
            .execute(&id(), &capabilities(), r#"petunia.command("model.delete")"#)
            .unwrap_err();
        assert!(matches!(err, PluginError::PermissionDenied(ref what) if what == "model.delete"));
    }

    #[test]
    fn sandbox_has_no_io_os_or_network() {
        let host = Host::new();
        for probe in [
            r#"assert(io == nil)"#,
            r#"assert(os == nil)"#,
            r#"assert(package == nil)"#,
            r#"assert(require == nil)"#,
            r#"assert(socket == nil)"#,
            // Bytecode nunca é aceito (pode derrubar a VM).
            r#"assert(string.dump == nil)"#,
            r#"assert(load("\27Lua") == nil)"#,
        ] {
            host.execute(&id(), &capabilities(), probe).unwrap();
        }
    }

    #[test]
    fn infinite_loop_hits_instruction_budget() {
        let host = Host::with_config(HostConfig {
            max_instructions: 50_000,
            max_memory_bytes: MAX_MEMORY_BYTES,
        });
        let err = host
            .execute(&id(), &capabilities(), r#"while true do end"#)
            .unwrap_err();
        assert!(matches!(err, PluginError::Budget));
    }

    #[test]
    fn declarations_are_validated_before_execution() {
        let host = Host::new();
        let mut bad_api = capabilities();
        bad_api.api_version = 999;
        assert!(matches!(
            host.execute(&id(), &bad_api, "").unwrap_err(),
            PluginError::Capability(CapabilityError::UnsupportedApi(999))
        ));
        let mut bad_cmd = capabilities();
        bad_cmd.commands = vec!["rm -rf".to_string()];
        assert!(matches!(
            host.execute(&id(), &bad_cmd, "").unwrap_err(),
            PluginError::Capability(_)
        ));
        let mut destructive = capabilities();
        destructive.commands = vec!["model.delete".to_string()];
        assert!(matches!(
            host.execute(&id(), &destructive, "").unwrap_err(),
            PluginError::Capability(CapabilityError::DestructiveDenied(_))
        ));
        let mut network = capabilities();
        network.allow_network = true;
        assert!(matches!(
            host.execute(&id(), &network, "").unwrap_err(),
            PluginError::Capability(CapabilityError::NetworkDenied)
        ));
        assert!(PluginId::new("evil id!").is_err());
    }
}
