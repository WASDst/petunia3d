//! Petunia MCP boundary (P1-04 `rmcp` + P1-05 `tokio`).
//!
//! Flow enforced here:
//!
//! ```text
//! MCP transport (stdio)
//!     ↓
//! Petunia MCP adapter (this crate)
//!     ↓
//! policy (deny-by-default authoring) + validation (registry, bounded args,
//! stale ObjectIds, revision preconditions)
//!     ↓
//! Application API (`AppState::dispatch_intent`)
//!     ↓
//! CommandDispatcher → transaction → domain
//! ```
//!
//! The server never owns a parallel Document/Undo stack. Tokio lives only
//! inside this crate (`serve_stdio`); no Tokio type crosses into Core.

pub mod server;

pub use rmcp::ErrorData as McpError;
pub use server::{
    AuditEntry, McpPolicy, PetuniaMcp, serve_stdio, serve_stdio_blocking, serve_stdio_with,
};
