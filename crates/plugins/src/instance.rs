//! Instância persistente de plugin: um Lua State isolado por plugin
//! (website/docs/13-extensions/lua-plugins-public-api.md, "Runtime Lua").
//!
//! O entrypoint roda uma vez no load (fase de registro). Depois disso o host
//! entrega **eventos** — fatos já comitados — aos handlers registrados com
//! `petunia.on(nome, fn)`. Cada entrada no plugin tem budget novo; reentrada é
//! recusada (`Busy`); revogar capabilities vale na chamada seguinte; descartar
//! a instância libera state, handlers e intents (unload limpo).

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use crate::host::{
    HostConfig, PluginCapabilities, PluginError, PluginId, Sandbox, ValidatedIntent, map_lua_error,
};

/// Eventos públicos V1 (fatos pós-commit; payload JSON de snapshot).
/// Nomes refletem só o que o host afirma: mudanças de UV chegam como
/// `mesh.changed` (o fato do core não as distingue).
pub const KNOWN_EVENTS: &[&str] = &[
    "project.loaded",
    "active_object.changed",
    "selection.changed",
    "mesh.changed",
    "texture.changed",
];

/// Máximo de handlers por plugin.
pub const MAX_HANDLERS: usize = 64;

const HANDLERS_KEY: &str = "petunia.handlers";

pub struct PluginInstance {
    id: PluginId,
    sandbox: Sandbox,
    delivering: Cell<bool>,
    subscriptions: Rc<RefCell<BTreeSet<String>>>,
}

impl PluginInstance {
    /// Cria o state, instala `petunia.on` e roda o entrypoint. Os intents do
    /// entrypoint são devolvidos para o host aplicar como um batch.
    pub fn load(
        id: PluginId,
        capabilities: PluginCapabilities,
        config: &HostConfig,
        source: &str,
    ) -> Result<(Self, ValidatedIntent), PluginError> {
        capabilities.validate()?;
        let sandbox = Sandbox::new(config, Rc::new(RefCell::new(capabilities)))?;
        let subscriptions = Rc::new(RefCell::new(BTreeSet::new()));
        Self::install_event_api(&sandbox, Rc::clone(&subscriptions))?;
        let instance = Self {
            id,
            sandbox,
            delivering: Cell::new(false),
            subscriptions,
        };
        instance.sandbox.run_chunk(&instance.id, source)?;
        let output = instance.sandbox.take_output();
        Ok((instance, output))
    }

    fn install_event_api(
        sandbox: &Sandbox,
        subscriptions: Rc<RefCell<BTreeSet<String>>>,
    ) -> Result<(), PluginError> {
        let lua = &sandbox.lua;
        let handlers = lua
            .create_table()
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        lua.set_named_registry_value(HANDLERS_KEY, handlers)
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        let on = lua
            .create_function(move |lua, (event, handler): (String, mlua::Function)| {
                if !KNOWN_EVENTS.contains(&event.as_str()) {
                    return Err(mlua::Error::RuntimeError(format!(
                        "unknown event '{event}'"
                    )));
                }
                let handlers: mlua::Table = lua.named_registry_value(HANDLERS_KEY)?;
                let list: mlua::Table = match handlers.get::<Option<mlua::Table>>(event.as_str())? {
                    Some(list) => list,
                    None => {
                        let list = lua.create_table()?;
                        handlers.set(event.as_str(), list.clone())?;
                        list
                    }
                };
                let total: usize = handlers
                    .pairs::<String, mlua::Table>()
                    .filter_map(Result::ok)
                    .map(|(_, l)| l.raw_len())
                    .sum();
                if total >= MAX_HANDLERS {
                    return Err(mlua::Error::RuntimeError("too many event handlers".into()));
                }
                list.raw_push(handler)?;
                subscriptions.borrow_mut().insert(event);
                Ok(())
            })
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        let petunia: mlua::Table = lua
            .globals()
            .get("petunia")
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        petunia
            .set("on", on)
            .map_err(|e| PluginError::Lua(e.to_string()))
    }

    pub fn id(&self) -> &PluginId {
        &self.id
    }

    /// Eventos assinados, em ordem determinística.
    pub fn subscriptions(&self) -> Vec<String> {
        self.subscriptions.borrow().iter().cloned().collect()
    }

    /// Troca as capabilities efetivas (concessão/revogação). Vale a partir da
    /// próxima chamada do plugin.
    pub fn set_capabilities(&self, capabilities: PluginCapabilities) {
        *self.sandbox.capabilities.borrow_mut() = capabilities;
    }

    /// Entrega um evento (fato pós-commit) aos handlers do plugin, na ordem de
    /// registro, com budget novo. Devolve os intents pedidos pelos handlers para
    /// o host aplicar como um batch. Sem handler para o evento: nada acontece.
    pub fn deliver(&self, event: &str, payload_json: &str) -> Result<ValidatedIntent, PluginError> {
        if self.delivering.replace(true) {
            return Err(PluginError::Busy);
        }
        let result = self.deliver_inner(event, payload_json);
        self.delivering.set(false);
        result
    }

    fn deliver_inner(
        &self,
        event: &str,
        payload_json: &str,
    ) -> Result<ValidatedIntent, PluginError> {
        if !self.subscriptions.borrow().contains(event) {
            return Ok(ValidatedIntent::default());
        }
        let lua = &self.sandbox.lua;
        let handlers: mlua::Table = lua
            .named_registry_value(HANDLERS_KEY)
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        let list: mlua::Table = handlers
            .get(event)
            .map_err(|e| PluginError::Lua(e.to_string()))?;
        self.sandbox.reset_budget();
        for handler in list.sequence_values::<mlua::Function>() {
            let handler = handler.map_err(|e| PluginError::Lua(e.to_string()))?;
            handler
                .call::<()>((event, payload_json))
                .map_err(|e| map_lua_error(&e))?;
        }
        Ok(self.sandbox.take_output())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::PLUGIN_API_VERSION;

    fn caps(commands: &[&str]) -> PluginCapabilities {
        PluginCapabilities {
            api_version: PLUGIN_API_VERSION,
            commands: commands.iter().map(|c| c.to_string()).collect(),
            queries: vec![],
            filesystem_roots: vec![],
            allow_network: false,
            allow_destructive: false,
        }
    }

    fn load(source: &str, commands: &[&str]) -> PluginInstance {
        let id = PluginId::new("test.events").unwrap();
        PluginInstance::load(id, caps(commands), &HostConfig::default(), source)
            .unwrap()
            .0
    }

    #[test]
    fn state_persists_between_events_and_handlers_record_intents() {
        let plugin = load(
            r#"
            local count = 0
            petunia.on("mesh.changed", function(name, payload)
                count = count + 1
                petunia.command("uv.pack_islands", { count })
            end)
            "#,
            &["uv.pack_islands"],
        );
        assert_eq!(plugin.subscriptions(), vec!["mesh.changed".to_string()]);
        plugin.deliver("mesh.changed", "{}").unwrap();
        let second = plugin.deliver("mesh.changed", "{}").unwrap();
        assert_eq!(second.commands[0].args, vec![2.0]);
        assert!(
            plugin
                .deliver("texture.changed", "{}")
                .unwrap()
                .commands
                .is_empty()
        );
    }

    #[test]
    fn revoked_capabilities_apply_on_the_next_call() {
        let plugin = load(
            r#"petunia.on("texture.changed", function() petunia.command("uv.pack_islands") end)"#,
            &["uv.pack_islands"],
        );
        assert_eq!(
            plugin
                .deliver("texture.changed", "{}")
                .unwrap()
                .commands
                .len(),
            1
        );
        plugin.set_capabilities(caps(&[]));
        assert!(matches!(
            plugin.deliver("texture.changed", "{}"),
            Err(PluginError::PermissionDenied(ref what)) if what == "uv.pack_islands"
        ));
    }

    #[test]
    fn unknown_events_are_rejected_at_registration() {
        let id = PluginId::new("test.events").unwrap();
        let result = PluginInstance::load(
            id,
            caps(&[]),
            &HostConfig::default(),
            r#"petunia.on("render.frame", function() end)"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn each_delivery_gets_its_own_budget() {
        let id = PluginId::new("test.events").unwrap();
        let config = HostConfig {
            max_instructions: 200_000,
            ..HostConfig::default()
        };
        let (plugin, _) = PluginInstance::load(
            id,
            caps(&[]),
            &config,
            r#"
            petunia.on("mesh.changed", function()
                for i = 1, 5000 do local _ = i * 2 end
            end)
            petunia.on("texture.changed", function() while true do end end)
            "#,
        )
        .unwrap();
        for _ in 0..20 {
            plugin.deliver("mesh.changed", "{}").unwrap();
        }
        assert!(matches!(
            plugin.deliver("texture.changed", "{}"),
            Err(PluginError::Budget)
        ));
        // O budget estourado não envenena entregas seguintes.
        plugin.deliver("mesh.changed", "{}").unwrap();
    }
}
