//! Dados autorais de plugins no Document (website/docs/13-extensions/
//! lua-plugins-public-api.md, "Estado, dados e compatibilidade").
//!
//! Cada plugin tem um namespace próprio com schema versionado. O payload é
//! JSON opaco para o host: é preservado byte a byte quando o plugin está
//! ausente (diagnóstico de plugin faltando, nunca apagar) e nunca é migrado
//! silenciosamente por uma atualização do plugin.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Payload de um plugin: versão do schema declarada por ele e o JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginPayload {
    pub schema_version: u32,
    pub data: serde_json::Value,
}

/// Namespaces por `plugin_id`, em ordem determinística.
pub type PluginDataStore = BTreeMap<String, PluginPayload>;

/// Limite de um payload serializado (dados autorais, não cache).
pub const MAX_PLUGIN_PAYLOAD_BYTES: usize = 1024 * 1024;

/// Escrita recusada de payload de plugin.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PluginDataError {
    #[error("bad plugin namespace '{0}'")]
    BadNamespace(String),
    #[error("plugin payload has {0} bytes (max {MAX_PLUGIN_PAYLOAD_BYTES})")]
    TooLarge(usize),
}

/// Única porta de escrita: namespace com a mesma forma de `PluginId`
/// (`1..=64` de `[a-zA-Z0-9._-]`) e payload serializado dentro do limite.
pub fn put_payload(
    store: &mut PluginDataStore,
    plugin_id: &str,
    payload: PluginPayload,
) -> Result<(), PluginDataError> {
    let valid = !plugin_id.is_empty()
        && plugin_id.len() <= 64
        && plugin_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !valid {
        return Err(PluginDataError::BadNamespace(plugin_id.to_string()));
    }
    let size = serde_json::to_vec(&payload.data).map_or(usize::MAX, |bytes| bytes.len());
    if size > MAX_PLUGIN_PAYLOAD_BYTES {
        return Err(PluginDataError::TooLarge(size));
    }
    store.insert(plugin_id.to_string(), payload);
    Ok(())
}

/// Plugins com dados no documento que não estão em `installed`: o host os
/// reporta como "missing plugin" sem tocar nos payloads.
pub fn missing_plugins<'a>(
    store: &'a PluginDataStore,
    installed: impl Fn(&str) -> bool + 'a,
) -> impl Iterator<Item = &'a str> + 'a {
    store
        .keys()
        .map(String::as_str)
        .filter(move |id| !installed(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_plugins_are_reported_without_dropping_payloads() {
        let mut store = PluginDataStore::new();
        let payload = PluginPayload {
            schema_version: 2,
            data: serde_json::json!({ "tiles": [1, 2, 3] }),
        };
        store.insert("acme.tiles".into(), payload.clone());
        store.insert("acme.paint".into(), payload.clone());
        let missing: Vec<_> = missing_plugins(&store, |id| id == "acme.paint").collect();
        assert_eq!(missing, vec!["acme.tiles"]);
        assert_eq!(store["acme.tiles"], payload);
    }

    #[test]
    fn writes_validate_namespace_and_size() {
        let mut store = PluginDataStore::new();
        let small = PluginPayload {
            schema_version: 1,
            data: serde_json::json!({ "ok": true }),
        };
        assert!(put_payload(&mut store, "acme.tiles", small.clone()).is_ok());
        assert!(matches!(
            put_payload(&mut store, "../evil", small),
            Err(PluginDataError::BadNamespace(_))
        ));
        let huge = PluginPayload {
            schema_version: 1,
            data: serde_json::Value::String("x".repeat(MAX_PLUGIN_PAYLOAD_BYTES)),
        };
        assert!(matches!(
            put_payload(&mut store, "acme.tiles", huge),
            Err(PluginDataError::TooLarge(_))
        ));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn payloads_round_trip_through_json() {
        let mut store = PluginDataStore::new();
        store.insert(
            "acme.tiles".into(),
            PluginPayload {
                schema_version: 1,
                data: serde_json::json!({ "a": 1 }),
            },
        );
        let text = serde_json::to_string(&store).unwrap();
        let back: PluginDataStore = serde_json::from_str(&text).unwrap();
        assert_eq!(back, store);
    }
}
