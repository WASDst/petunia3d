//! Ciclo de vida de plugins (website/docs/13-extensions/
//! lua-plugins-public-api.md, "Manifest e permissões", "Runtime Lua").
//!
//! Install valida o manifest; Enable exige as capabilities obrigatórias
//! concedidas pelo usuário e cria um Lua State isolado; Grant/Revoke mudam as
//! permissões com efeito observável (revogar uma obrigatória desabilita o
//! plugin); Disable/Uninstall descartam o state — handlers, assinaturas e
//! intents pendentes somem junto. Não é um ModuleRegistry genérico: só plugins
//! Lua de terceiros passam por aqui.

use std::collections::{BTreeMap, BTreeSet};

use crate::host::{CapabilityError, HostConfig, PluginError, PluginId, ValidatedIntent};
use crate::instance::PluginInstance;
use crate::manifest::{Capability, PluginManifest};

/// Estado observável de um plugin instalado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginStatus {
    Disabled,
    Enabled,
    /// Desabilitado pelo host após erro; reabilitar é ação explícita.
    Faulted,
}

struct Installed {
    manifest: PluginManifest,
    source: String,
    granted: BTreeSet<Capability>,
    status: PluginStatus,
    instance: Option<PluginInstance>,
    last_error: Option<String>,
}

/// Falha de ciclo de vida.
#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error(transparent)]
    Capability(#[from] CapabilityError),
    #[error(transparent)]
    Plugin(#[from] PluginError),
    #[error("plugin '{0}' is not installed")]
    NotInstalled(String),
    #[error("plugin '{0}' is already installed")]
    AlreadyInstalled(String),
    #[error("required capability {0:?} not granted")]
    MissingCapability(Capability),
}

/// Plugins Lua instalados, por id, em ordem determinística.
#[derive(Default)]
pub struct PluginRegistry {
    config: HostConfig,
    plugins: BTreeMap<String, Installed>,
}

impl PluginRegistry {
    pub fn new(config: HostConfig) -> Self {
        Self {
            config,
            plugins: BTreeMap::new(),
        }
    }

    /// Instala (sem executar nada). O manifest precisa ser válido.
    pub fn install(
        &mut self,
        manifest: PluginManifest,
        source: String,
    ) -> Result<PluginId, LifecycleError> {
        let id = manifest.validate()?;
        if self.plugins.contains_key(id.as_str()) {
            return Err(LifecycleError::AlreadyInstalled(id.as_str().to_string()));
        }
        self.plugins.insert(
            id.as_str().to_string(),
            Installed {
                manifest,
                source,
                granted: BTreeSet::new(),
                status: PluginStatus::Disabled,
                instance: None,
                last_error: None,
            },
        );
        Ok(id)
    }

    fn get_mut(&mut self, id: &str) -> Result<&mut Installed, LifecycleError> {
        self.plugins
            .get_mut(id)
            .ok_or_else(|| LifecycleError::NotInstalled(id.to_string()))
    }

    /// Concede uma capability (obrigatória ou opcional). Com o plugin ativo,
    /// vale a partir da próxima chamada.
    pub fn grant(&mut self, id: &str, capability: Capability) -> Result<(), LifecycleError> {
        let plugin = self.get_mut(id)?;
        plugin.granted.insert(capability);
        Self::refresh_capabilities(plugin);
        Ok(())
    }

    /// Revoga uma capability. Revogar uma obrigatória desabilita o plugin
    /// (descarta o state); revogar uma opcional vale na próxima chamada.
    pub fn revoke(&mut self, id: &str, capability: Capability) -> Result<(), LifecycleError> {
        let plugin = self.get_mut(id)?;
        plugin.granted.remove(&capability);
        if plugin.manifest.required_capabilities.contains(&capability) {
            plugin.instance = None;
            plugin.status = PluginStatus::Disabled;
        } else {
            Self::refresh_capabilities(plugin);
        }
        Ok(())
    }

    fn refresh_capabilities(plugin: &mut Installed) {
        if let Some(instance) = &plugin.instance {
            let effective = plugin.manifest.effective(&plugin.granted);
            instance.set_capabilities(plugin.manifest.capabilities_for(&effective));
        }
    }

    /// Ativa: exige todas as obrigatórias concedidas, cria o state e roda o
    /// entrypoint. Devolve os intents do entrypoint (o host aplica como batch).
    pub fn enable(&mut self, id: &str) -> Result<ValidatedIntent, LifecycleError> {
        let config = self.config.clone();
        let plugin = self.get_mut(id)?;
        if let Some(missing) = plugin
            .manifest
            .required_capabilities
            .iter()
            .find(|c| !plugin.granted.contains(c))
        {
            return Err(LifecycleError::MissingCapability(*missing));
        }
        let plugin_id = PluginId::new(&plugin.manifest.plugin_id)?;
        let effective = plugin.manifest.effective(&plugin.granted);
        let capabilities = plugin.manifest.capabilities_for(&effective);
        match PluginInstance::load(plugin_id, capabilities, &config, &plugin.source) {
            Ok((instance, intents)) => {
                plugin.instance = Some(instance);
                plugin.status = PluginStatus::Enabled;
                plugin.last_error = None;
                Ok(intents)
            }
            Err(error) => {
                plugin.instance = None;
                plugin.status = PluginStatus::Faulted;
                plugin.last_error = Some(error.to_string());
                Err(error.into())
            }
        }
    }

    /// Desativa: descarta o state (handlers, assinaturas, intents pendentes).
    pub fn disable(&mut self, id: &str) -> Result<(), LifecycleError> {
        let plugin = self.get_mut(id)?;
        plugin.instance = None;
        plugin.status = PluginStatus::Disabled;
        Ok(())
    }

    /// Remove o plugin. Dados autorais dele no Document são preservados pelo
    /// próprio Document (ver `petunia_project::plugin_data`).
    pub fn uninstall(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.plugins
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| LifecycleError::NotInstalled(id.to_string()))
    }

    pub fn status(&self, id: &str) -> Option<PluginStatus> {
        self.plugins.get(id).map(|p| p.status)
    }

    pub fn last_error(&self, id: &str) -> Option<&str> {
        self.plugins.get(id).and_then(|p| p.last_error.as_deref())
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }

    /// Entrega um evento a todos os plugins ativos que o assinam, em ordem de
    /// id. Um plugin que falha é desabilitado com diagnóstico (Faulted) e os
    /// demais seguem. Devolve `(plugin_id, intents)` para o host aplicar, cada
    /// um como um batch atômico.
    pub fn deliver(&mut self, event: &str, payload_json: &str) -> Vec<(String, ValidatedIntent)> {
        let mut out = Vec::new();
        for (id, plugin) in &mut self.plugins {
            let Some(instance) = &plugin.instance else {
                continue;
            };
            match instance.deliver(event, payload_json) {
                Ok(intents) if !intents.commands.is_empty() || !intents.queries.is_empty() => {
                    out.push((id.clone(), intents));
                }
                Ok(_) => {}
                Err(error) => {
                    plugin.instance = None;
                    plugin.status = PluginStatus::Faulted;
                    plugin.last_error = Some(error.to_string());
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::PLUGIN_API_VERSION;

    fn manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "acme.uv".into(),
            version: "1.0.0".into(),
            api_version: PLUGIN_API_VERSION,
            name: "UV".into(),
            author: String::new(),
            source: String::new(),
            entrypoint: "main.lua".into(),
            required_capabilities: BTreeSet::from([Capability::ReadDocument]),
            optional_capabilities: BTreeSet::from([Capability::EditUv]),
            commands: vec!["uv.pack_islands".into()],
            queries: vec!["scene.list".into()],
            allow_destructive: false,
        }
    }

    const SOURCE: &str =
        r#"petunia.on("mesh.changed", function() petunia.command("uv.pack_islands") end)"#;

    fn installed() -> PluginRegistry {
        let mut registry = PluginRegistry::default();
        registry.install(manifest(), SOURCE.into()).unwrap();
        registry
    }

    #[test]
    fn enable_requires_required_capabilities() {
        let mut registry = installed();
        assert!(matches!(
            registry.enable("acme.uv"),
            Err(LifecycleError::MissingCapability(Capability::ReadDocument))
        ));
        registry.grant("acme.uv", Capability::ReadDocument).unwrap();
        registry.enable("acme.uv").unwrap();
        assert_eq!(registry.status("acme.uv"), Some(PluginStatus::Enabled));
    }

    #[test]
    fn optional_grant_and_revoke_are_observable() {
        let mut registry = installed();
        registry.grant("acme.uv", Capability::ReadDocument).unwrap();
        registry.enable("acme.uv").unwrap();
        // Sem EditUv o handler é recusado e o plugin vira Faulted.
        assert!(registry.deliver("mesh.changed", "{}").is_empty());
        assert_eq!(registry.status("acme.uv"), Some(PluginStatus::Faulted));
        assert!(
            registry
                .last_error("acme.uv")
                .unwrap()
                .contains("permission denied")
        );

        registry.grant("acme.uv", Capability::EditUv).unwrap();
        registry.enable("acme.uv").unwrap();
        let out = registry.deliver("mesh.changed", "{}");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1.commands[0].command, "uv.pack_islands");
    }

    #[test]
    fn revoking_a_required_capability_disables_and_drops_handlers() {
        let mut registry = installed();
        registry.grant("acme.uv", Capability::ReadDocument).unwrap();
        registry.grant("acme.uv", Capability::EditUv).unwrap();
        registry.enable("acme.uv").unwrap();
        registry
            .revoke("acme.uv", Capability::ReadDocument)
            .unwrap();
        assert_eq!(registry.status("acme.uv"), Some(PluginStatus::Disabled));
        assert!(registry.deliver("mesh.changed", "{}").is_empty());
    }

    #[test]
    fn install_twice_and_unknown_ids_fail() {
        let mut registry = installed();
        assert!(matches!(
            registry.install(manifest(), SOURCE.into()),
            Err(LifecycleError::AlreadyInstalled(_))
        ));
        assert!(matches!(
            registry.enable("nope"),
            Err(LifecycleError::NotInstalled(_))
        ));
        registry.uninstall("acme.uv").unwrap();
        assert!(!registry.is_installed("acme.uv"));
    }
}
