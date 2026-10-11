//! Manifest e capabilities nomeadas (website/docs/13-extensions/
//! lua-plugins-public-api.md, "Manifest e permissões").
//!
//! Negar por padrão: cada command/query declarado precisa estar coberto por
//! uma capability **concedida**. Read não implica write; filesystem não implica
//! network; UI panel não implica acesso ao Document; `mcp_exposable` é opt-in
//! adicional e nunca concede nada.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::host::{CapabilityError, PLUGIN_API_VERSION, PluginCapabilities, PluginId};

/// Capabilities conhecidas pela API V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    RegisterCommands,
    RegisterUiPanel,
    RegisterViewportOverlay,
    ReadDocument,
    ReadSelection,
    EditGeometry,
    EditUv,
    EditMaterials,
    ImportFiles,
    ExportFiles,
    Filesystem,
    McpExposable,
}

impl Capability {
    /// Prefixos de command id que a capability libera.
    fn command_prefixes(self) -> &'static [&'static str] {
        match self {
            Self::EditGeometry => &["model.", "global.undo", "global.redo"],
            Self::EditUv => &["uv."],
            Self::EditMaterials => &["material.", "paint."],
            Self::ImportFiles => &["project.import"],
            Self::ExportFiles => &["project.export"],
            _ => &[],
        }
    }

    /// Query names que a capability libera.
    fn queries(self) -> &'static [&'static str] {
        match self {
            Self::ReadDocument => &["scene.list", "scene.hierarchy", "scene.tools"],
            Self::ReadSelection => &["scene.selection"],
            _ => &[],
        }
    }

    fn covers_command(self, command: &str) -> bool {
        self.command_prefixes()
            .iter()
            .any(|prefix| command == *prefix || command.starts_with(prefix))
    }
}

/// Manifest de um plugin Lua.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin_id: String,
    /// Versão do plugin, `MAJOR.MINOR.PATCH`.
    pub version: String,
    /// Versão da API do host exigida.
    pub api_version: u32,
    pub name: String,
    #[serde(default)]
    pub author: String,
    /// Origem declarada (URL/repositório) — informativa, nunca resolvida.
    #[serde(default)]
    pub source: String,
    /// Arquivo Lua de entrada, relativo ao pacote do plugin.
    pub entrypoint: String,
    #[serde(default)]
    pub required_capabilities: BTreeSet<Capability>,
    #[serde(default)]
    pub optional_capabilities: BTreeSet<Capability>,
    /// Command ids que o plugin pode pedir.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Queries que o plugin pode pedir.
    #[serde(default)]
    pub queries: Vec<String>,
    /// Comandos destrutivos (delete/discard/overwrite) exigem opt-in explícito.
    #[serde(default)]
    pub allow_destructive: bool,
}

/// Versão semântica mínima `MAJOR.MINOR.PATCH`.
fn is_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 9 && p.chars().all(|c| c.is_ascii_digit()))
}

/// Entrypoint relativo, sem `..`, sem raiz e terminando em `.lua`.
fn is_safe_entrypoint(entry: &str) -> bool {
    !entry.is_empty()
        && entry.len() <= 128
        && entry.ends_with(".lua")
        && !entry.starts_with('/')
        && !entry.starts_with('\\')
        && !entry.contains(':')
        && entry
            .split(['/', '\\'])
            .all(|part| part != ".." && !part.is_empty())
}

impl PluginManifest {
    /// Valida forma e coerência do manifest, independente das concessões.
    pub fn validate(&self) -> Result<PluginId, CapabilityError> {
        let id = PluginId::new(&self.plugin_id)?;
        if self.api_version != PLUGIN_API_VERSION {
            return Err(CapabilityError::UnsupportedApi(self.api_version));
        }
        if !is_semver(&self.version) {
            return Err(CapabilityError::BadManifest(format!(
                "version '{}' is not MAJOR.MINOR.PATCH",
                self.version
            )));
        }
        if self.name.trim().is_empty() || self.name.len() > 128 {
            return Err(CapabilityError::BadManifest("name".into()));
        }
        if !is_safe_entrypoint(&self.entrypoint) {
            return Err(CapabilityError::BadManifest(format!(
                "entrypoint '{}'",
                self.entrypoint
            )));
        }
        if let Some(both) = self
            .required_capabilities
            .intersection(&self.optional_capabilities)
            .next()
        {
            return Err(CapabilityError::BadManifest(format!(
                "{both:?} is both required and optional"
            )));
        }
        // Tudo que o plugin pode pedir precisa estar ao alcance do que declara.
        let declared: BTreeSet<Capability> = self
            .required_capabilities
            .union(&self.optional_capabilities)
            .copied()
            .collect();
        self.check_coverage(&declared)?;
        Ok(id)
    }

    /// Capabilities efetivas: obrigatórias + opcionais concedidas.
    pub fn effective(&self, granted: &BTreeSet<Capability>) -> BTreeSet<Capability> {
        self.required_capabilities
            .iter()
            .chain(
                self.optional_capabilities
                    .iter()
                    .filter(|c| granted.contains(c)),
            )
            .copied()
            .collect()
    }

    /// Lista de permissões de execução para um conjunto efetivo: só os
    /// commands/queries cobertos por ele (o resto fica negado em tempo de chamada).
    pub fn capabilities_for(&self, effective: &BTreeSet<Capability>) -> PluginCapabilities {
        PluginCapabilities {
            api_version: self.api_version,
            commands: self
                .commands
                .iter()
                .filter(|cmd| effective.iter().any(|cap| cap.covers_command(cmd)))
                .cloned()
                .collect(),
            queries: self
                .queries
                .iter()
                .filter(|q| {
                    effective
                        .iter()
                        .any(|cap| cap.queries().contains(&q.as_str()))
                })
                .cloned()
                .collect(),
            filesystem_roots: Vec::new(),
            allow_network: false,
            allow_destructive: self.allow_destructive,
        }
    }

    fn check_coverage(&self, declared: &BTreeSet<Capability>) -> Result<(), CapabilityError> {
        for cmd in &self.commands {
            if !declared.iter().any(|cap| cap.covers_command(cmd)) {
                return Err(CapabilityError::NotCovered(cmd.clone()));
            }
        }
        for query in &self.queries {
            if !declared
                .iter()
                .any(|cap| cap.queries().contains(&query.as_str()))
            {
                return Err(CapabilityError::NotCovered(query.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PluginManifest {
        PluginManifest {
            plugin_id: "acme.uv_tools".into(),
            version: "1.2.0".into(),
            api_version: PLUGIN_API_VERSION,
            name: "UV Tools".into(),
            author: "Acme".into(),
            source: String::new(),
            entrypoint: "main.lua".into(),
            required_capabilities: BTreeSet::from([Capability::ReadDocument]),
            optional_capabilities: BTreeSet::from([Capability::EditUv]),
            commands: vec!["uv.pack_islands".into()],
            queries: vec!["scene.list".into()],
            allow_destructive: false,
        }
    }

    #[test]
    fn valid_manifest_passes_and_parses_from_json() {
        let m = manifest();
        assert!(m.validate().is_ok());
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"read_document\""));
        let back: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn commands_need_a_declared_capability() {
        let mut m = manifest();
        m.commands.push("model.extrude".into());
        assert_eq!(
            m.validate(),
            Err(CapabilityError::NotCovered("model.extrude".into()))
        );
    }

    #[test]
    fn optional_capabilities_only_apply_when_granted() {
        let m = manifest();
        let denied = m.capabilities_for(&m.effective(&BTreeSet::new()));
        assert!(denied.commands.is_empty());
        assert_eq!(denied.queries, vec!["scene.list".to_string()]);
        let granted = m.capabilities_for(&m.effective(&BTreeSet::from([Capability::EditUv])));
        assert_eq!(granted.commands, vec!["uv.pack_islands".to_string()]);
    }

    #[test]
    fn mcp_exposable_grants_nothing() {
        let mut m = manifest();
        m.optional_capabilities = BTreeSet::from([Capability::McpExposable]);
        assert!(matches!(m.validate(), Err(CapabilityError::NotCovered(_))));
    }

    #[test]
    fn unsafe_entrypoints_and_versions_are_rejected() {
        for entry in [
            "../evil.lua",
            "/abs.lua",
            "C:\\x.lua",
            "main.txt",
            "a//b.lua",
        ] {
            let mut m = manifest();
            m.entrypoint = entry.into();
            assert!(m.validate().is_err(), "{entry}");
        }
        let mut m = manifest();
        m.version = "1.2".into();
        assert!(m.validate().is_err());
    }
}
