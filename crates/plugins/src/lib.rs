//! Capability-limited Lua 5.4 plugin host (P1-03 / E01, `mlua`).
//!
//! Website/docs/13-extensions/lua-plugins-public-api.md. Plugins receive
//! Petunia API contracts, never raw UI/GPU handles, application state, or
//! unrestricted filesystem:
//!
//! ```text
//! PluginManifest (named capabilities, deny by default)
//!     ↓ PluginRegistry: install → grant → enable / revoke / disable
//! PluginInstance: one isolated Lua 5.4 state per plugin
//!     (no io/os/package, text-only `load`, instruction + memory budgets)
//!     ↓ petunia.command / petunia.query / petunia.on(event)
//! ValidatedIntent (commands checked against live capabilities)
//!     ↓ apply_intents: one atomic Application transaction per batch
//! Document / Undo shared with native tools
//! ```

pub mod dispatch;
pub mod host;
pub mod instance;
pub mod manifest;
pub mod registry;

pub use dispatch::{
    PluginApiError, answer_plugin_query, answer_query, apply_intents, detect_missing_plugins,
    get_plugin_data, set_plugin_data,
};
pub use host::{
    CapabilityError, Host, HostConfig, LuaCommand, PluginCapabilities, PluginError, PluginId,
    ValidatedIntent,
};
pub use instance::{KNOWN_EVENTS, PluginInstance};
pub use manifest::{Capability, PluginManifest};
pub use registry::{LifecycleError, PluginRegistry, PluginStatus};
