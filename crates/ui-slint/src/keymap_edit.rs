//! Edição de atalhos: perfis do usuário, captura de tecla e restauração (P3D-090/091).
//!
//! O markup só mostra linhas prontas ([`KeymapActionRow`]) e emite intenções
//! (capturar, restaurar, criar/apagar perfil). Toda regra mora aqui e em
//! `petunia_config::keybinds`:
//!
//! - perfis canônicos são somente leitura: a primeira edição cria uma cópia
//!   `user-*` no diretório de configuração e a ativa;
//! - a captura aceita uma combinação (Ctrl/Shift/Alt + tecla); `Escape` cancela;
//! - conflitos não bloqueiam (o usuário decide), mas são sinalizados na linha e
//!   no status.

use std::sync::Arc;

use petunia_config::Keybinds;
use petunia_config::keybinds::{Binding, Mods2};

use crate::{PetuniaViewport, SlintUiBridge};

/// Linha de perfil na lista de perfis de atalhos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapProfileRow {
    pub id: String,
    pub name: String,
    pub custom: bool,
    pub active: bool,
}

/// Linha de ação na lista de atalhos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapActionRow {
    pub action: String,
    pub label: String,
    pub shortcut: String,
    pub conflict: bool,
    pub capturing: bool,
}

/// Listas prontas para o markup (perfis e ações), em cache por revisão.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapSnapshot {
    pub profiles: Vec<KeymapProfileRow>,
    pub rows: Vec<KeymapActionRow>,
}

/// Estado de edição de atalhos no shell (não é documento).
#[derive(Default)]
pub struct KeymapEditor {
    /// Última fotografia das listas e a revisão em que foi feita.
    pub cache: std::cell::RefCell<Option<(u32, Arc<KeymapSnapshot>)>>,
    /// Muda a cada alteração; o shell só reconstrói as listas quando ele muda.
    pub revision: u32,
    /// Ação esperando a próxima combinação de teclas.
    pub capturing: Option<String>,
    /// Diretório dos perfis do usuário (testes substituem).
    pub dir_override: Option<std::path::PathBuf>,
}

/// Texto legível derivado do id (`model.push_pull` → `model · push pull`).
pub fn action_label(action: &str) -> String {
    let (namespace, name) = action.split_once('.').unwrap_or(("", action));
    let name = name.replace('_', " ");
    if namespace.is_empty() {
        name
    } else {
        format!("{namespace} · {name}")
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    fn keymap_dir(&self) -> std::path::PathBuf {
        self.keymap_editor
            .dir_override
            .clone()
            .unwrap_or_else(Keybinds::user_profiles_dir)
    }

    fn bump_keymap_revision(&mut self) {
        self.keymap_editor.revision = self.keymap_editor.revision.wrapping_add(1);
        self.state.mark_dirty();
    }

    /// Perfis disponíveis (canônicos e do usuário) com o ativo marcado.
    pub fn keymap_profile_rows(&self) -> Vec<KeymapProfileRow> {
        let active = &self.state.ui.active_keymap_id;
        Keybinds::all_profiles_in(&self.keymap_dir())
            .into_iter()
            .map(|p| KeymapProfileRow {
                active: &p.id == active,
                id: p.id,
                name: p.name,
                custom: p.custom,
            })
            .collect()
    }

    /// Ações conhecidas: as dos defaults mais as do perfil atual, ordenadas.
    fn known_keymap_actions(&self) -> Vec<String> {
        let mut all = Keybinds::defaults().bound_actions();
        for action in self.state.ui.keybinds.bound_actions() {
            if !all.contains(&action) {
                all.push(action);
            }
        }
        for (action, _) in crate::paint_surface_tools::SURFACE_ACTIONS
            .iter()
            .chain(crate::draw_extensions::DRAW_ACTIONS.iter())
        {
            if *action != "draw.import_svg" && !all.iter().any(|a| a == action) {
                all.push((*action).into());
            }
        }
        all.sort();
        all
    }

    fn build_keymap_rows(&self) -> Vec<KeymapActionRow> {
        let keybinds = &self.state.ui.keybinds;
        self.known_keymap_actions()
            .into_iter()
            .map(|action| {
                let binding = keybinds.binding_for(&action);
                KeymapActionRow {
                    label: crate::paint_surface_tools::SURFACE_ACTIONS
                        .iter()
                        .chain(crate::draw_extensions::DRAW_ACTIONS.iter())
                        .find(|(id, _)| *id == action)
                        .map_or_else(
                            || action_label(&action),
                            |(_, key)| self.state.t_id(petunia_config::TextId::new(key)),
                        ),
                    shortcut: binding.map(Binding::to_shortcut_string).unwrap_or_default(),
                    conflict: binding
                        .is_some_and(|b| !keybinds.conflicts_for(&action, b).is_empty()),
                    capturing: self.keymap_editor.capturing.as_deref() == Some(action.as_str()),
                    action,
                }
            })
            .collect()
    }

    /// Listas de perfis e ações; só são refeitas quando a revisão muda.
    pub fn keymap_snapshot(&self) -> (u32, Arc<KeymapSnapshot>) {
        let revision = self.keymap_editor.revision;
        let mut cache = self.keymap_editor.cache.borrow_mut();
        if let Some((cached, snapshot)) = cache.as_ref()
            && *cached == revision
        {
            return (revision, Arc::clone(snapshot));
        }
        let snapshot = Arc::new(KeymapSnapshot {
            profiles: self.keymap_profile_rows(),
            rows: self.build_keymap_rows(),
        });
        *cache = Some((revision, Arc::clone(&snapshot)));
        (revision, snapshot)
    }

    /// Garante um perfil editável: perfis canônicos viram uma cópia `user-*`.
    pub fn ensure_editable_keymap(&mut self) -> bool {
        let active = self.state.ui.active_keymap_id.clone();
        if Keybinds::is_custom_profile(&active) {
            return true;
        }
        let base = Keybinds::all_profiles_in(&self.keymap_dir())
            .into_iter()
            .find(|p| p.id == active)
            .map_or(active.clone(), |p| p.name);
        self.create_keymap_profile(&format!("{base} (custom)"))
    }

    /// Cria um perfil do usuário a partir dos atalhos atuais e o ativa.
    pub fn create_keymap_profile(&mut self, name: &str) -> bool {
        let name = name.trim();
        let name = if name.is_empty() {
            "Custom keymap"
        } else {
            name
        };
        // Nome livre: acrescenta um número enquanto o id já existir.
        let existing: Vec<String> = Keybinds::all_profiles_in(&self.keymap_dir())
            .into_iter()
            .map(|p| p.id)
            .collect();
        let mut candidate = name.to_string();
        let mut n = 2;
        while existing.contains(&petunia_config::keybinds::sanitize_profile_id(&candidate)) {
            candidate = format!("{name} {n}");
            n += 1;
        }
        let dir = self.keymap_dir();
        match self
            .state
            .ui
            .keybinds
            .save_user_profile_in(&dir, &candidate, "")
        {
            Ok(id) => {
                self.state.ui.active_keymap_id = id.clone();
                self.preferences.active_keymap_id = id;
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_KEYMAP_PROFILE_CREATED),
                    &[("candidate", format!("{candidate}"))],
                ));
                self.bump_keymap_revision();
                true
            }
            Err(error) => {
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_KEYMAP),
                    &[("error", format!("{error}"))],
                ));
                false
            }
        }
    }

    /// Apaga um perfil do usuário; se era o ativo, volta ao Petunia Padrão.
    pub fn delete_keymap_profile(&mut self, id: &str) -> bool {
        if !Keybinds::delete_user_profile_in(&self.keymap_dir(), id) {
            return false;
        }
        if self.state.ui.active_keymap_id == id {
            self.state.ui.active_keymap_id = "petunia-default".to_string();
            self.state.ui.keybinds =
                Keybinds::load_profile_in("petunia-default", &self.keymap_dir());
            self.preferences.active_keymap_id = "petunia-default".to_string();
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_KEYMAP_PROFILE_DELETED),
        );
        self.bump_keymap_revision();
        true
    }

    /// Ativa um perfil (canônico ou do usuário) lendo `dir` do bridge.
    pub fn activate_keymap_profile(&mut self, id: &str) -> bool {
        if self.state.ui.active_keymap_id == id {
            return false;
        }
        self.state.ui.active_keymap_id = id.to_string();
        self.state.ui.keybinds = Keybinds::load_profile_in(id, &self.keymap_dir());
        self.preferences.active_keymap_id = id.to_string();
        self.keymap_editor.capturing = None;
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_KEYMAP_PROFILE_ACTIVATED),
            &[("id", format!("{id}"))],
        ));
        self.bump_keymap_revision();
        true
    }

    fn persist_active_keymap(&mut self) -> bool {
        let active = self.state.ui.active_keymap_id.clone();
        if !Keybinds::is_custom_profile(&active) {
            return false;
        }
        let dir = self.keymap_dir();
        let name = Keybinds::all_profiles_in(&dir)
            .into_iter()
            .find(|p| p.id == active)
            .map_or(active.clone(), |p| p.name);
        match self.state.ui.keybinds.save_user_profile_in(&dir, &name, "") {
            Ok(_) => true,
            Err(error) => {
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_KEYMAP),
                    &[("error", format!("{error}"))],
                ));
                false
            }
        }
    }

    /// Começa a esperar a combinação para `action` (cria a cópia editável se preciso).
    pub fn begin_keymap_capture(&mut self, action: &str) -> bool {
        if !self.known_keymap_actions().iter().any(|a| a == action) {
            return false;
        }
        if !self.ensure_editable_keymap() {
            return false;
        }
        self.keymap_editor.capturing = Some(action.to_string());
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_PRESS_THE_NEW_SHORTCUT_ESC_CANCELS),
        );
        self.bump_keymap_revision();
        true
    }

    /// Cancela a captura em andamento.
    pub fn cancel_keymap_capture(&mut self) -> bool {
        if self.keymap_editor.capturing.take().is_none() {
            return false;
        }
        self.bump_keymap_revision();
        true
    }

    /// Recebe a combinação capturada e a grava na ação pendente.
    pub fn capture_keymap_key(&mut self, text: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        let Some(action) = self.keymap_editor.capturing.clone() else {
            return false;
        };
        let Some(key) = crate::input::key_code_from_slint(text) else {
            // Modificador sozinho ou tecla sem mapeamento: segue esperando.
            return true;
        };
        let binding = Binding {
            key,
            mods: Mods2 { ctrl, shift, alt },
        };
        let conflicts = self.state.ui.keybinds.conflicts_for(&action, &binding);
        self.state
            .ui
            .keybinds
            .set_binding(action.clone(), binding.clone());
        self.keymap_editor.capturing = None;
        if self.persist_active_keymap() {
            let shortcut = binding.to_shortcut_string();
            self.state.set_status(if conflicts.is_empty() {
                format!("{} = {shortcut}", action_label(&action))
            } else {
                format!(
                    "{} = {shortcut} (also used by {})",
                    action_label(&action),
                    conflicts.join(", ")
                )
            });
        }
        self.bump_keymap_revision();
        true
    }

    /// Remove o atalho de uma ação (fica sem tecla).
    pub fn clear_keymap_binding(&mut self, action: &str) -> bool {
        if !self.ensure_editable_keymap() {
            return false;
        }
        if self.state.ui.keybinds.remove_binding(action).is_none() {
            return false;
        }
        self.persist_active_keymap();
        self.bump_keymap_revision();
        true
    }

    /// Restaura o atalho padrão Petunia de uma ação.
    pub fn reset_keymap_binding(&mut self, action: &str) -> bool {
        if !self.ensure_editable_keymap() {
            return false;
        }
        let default = Keybinds::defaults().binding_for(action).cloned();
        match default {
            Some(binding) => self
                .state
                .ui
                .keybinds
                .set_binding(action.to_string(), binding),
            None => {
                self.state.ui.keybinds.remove_binding(action);
            }
        }
        self.persist_active_keymap();
        self.bump_keymap_revision();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlaceholderViewport;
    use petunia_core::AppState;

    fn bridge_with_dir(dir: &std::path::Path) -> SlintUiBridge<PlaceholderViewport> {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.keymap_editor.dir_override = Some(dir.to_path_buf());
        bridge
    }

    #[test]
    fn labels_are_derived_from_the_action_id() {
        assert_eq!(action_label("model.push_pull"), "model · push pull");
        assert_eq!(action_label("undo"), "undo");
    }

    #[test]
    fn editing_a_builtin_profile_creates_a_user_copy_and_persists_the_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut bridge = bridge_with_dir(dir.path());
        assert_eq!(bridge.state.ui.active_keymap_id, "petunia-default");

        assert!(bridge.begin_keymap_capture("model.extrude"));
        // a cópia já é o perfil ativo e está em disco
        let active = bridge.state.ui.active_keymap_id.clone();
        assert!(active.starts_with("user-"), "ativo = {active}");
        assert!(dir.path().join(format!("{active}.toml")).exists());

        // captura: Ctrl+Shift+X vira o atalho de Extrude
        assert!(bridge.route_shortcut("x", true, true, false));
        assert_eq!(
            bridge
                .state
                .ui
                .keybinds
                .shortcut_for("model.extrude")
                .as_deref(),
            Some("Ctrl+Shift+X")
        );
        assert!(bridge.keymap_editor.capturing.is_none());

        // recarregar o perfil do disco devolve a mudança
        let reloaded = petunia_config::Keybinds::load_profile_in(&active, dir.path());
        assert_eq!(
            reloaded.shortcut_for("model.extrude").as_deref(),
            Some("Ctrl+Shift+X")
        );
        // o perfil embutido continua intacto
        let builtin = petunia_config::Keybinds::load_profile_in("petunia-default", dir.path());
        assert_eq!(builtin.shortcut_for("model.extrude").as_deref(), Some("E"));
    }

    #[test]
    fn escape_cancels_a_capture_without_changing_the_binding() {
        let dir = tempfile::tempdir().unwrap();
        let mut bridge = bridge_with_dir(dir.path());
        assert!(bridge.begin_keymap_capture("model.extrude"));
        assert!(bridge.route_shortcut("Escape", false, false, false));
        assert!(bridge.keymap_editor.capturing.is_none());
        assert_eq!(
            bridge
                .state
                .ui
                .keybinds
                .shortcut_for("model.extrude")
                .as_deref(),
            Some("E")
        );
    }

    #[test]
    fn a_shortcut_used_elsewhere_is_flagged_but_applied() {
        let dir = tempfile::tempdir().unwrap();
        let mut bridge = bridge_with_dir(dir.path());
        assert!(bridge.begin_keymap_capture("model.extrude"));
        // "I" já é model.inset
        assert!(bridge.route_shortcut("i", false, false, false));
        let (_, snapshot) = bridge.keymap_snapshot();
        let row = snapshot
            .rows
            .iter()
            .find(|r| r.action == "model.extrude")
            .unwrap();
        assert_eq!(row.shortcut, "I");
        assert!(row.conflict);
    }

    #[test]
    fn reset_restores_the_default_and_delete_returns_to_the_default_profile() {
        let dir = tempfile::tempdir().unwrap();
        let mut bridge = bridge_with_dir(dir.path());
        assert!(bridge.begin_keymap_capture("model.extrude"));
        assert!(bridge.route_shortcut("q", false, false, false));
        assert!(bridge.reset_keymap_binding("model.extrude"));
        assert_eq!(
            bridge
                .state
                .ui
                .keybinds
                .shortcut_for("model.extrude")
                .as_deref(),
            Some("E")
        );
        let active = bridge.state.ui.active_keymap_id.clone();
        assert!(bridge.delete_keymap_profile(&active));
        assert_eq!(bridge.state.ui.active_keymap_id, "petunia-default");
        assert!(!dir.path().join(format!("{active}.toml")).exists());
        // perfis embutidos não podem ser apagados
        assert!(!bridge.delete_keymap_profile("blender"));
    }

    #[test]
    fn snapshot_lists_user_profiles_after_the_builtin_ones() {
        let dir = tempfile::tempdir().unwrap();
        let mut bridge = bridge_with_dir(dir.path());
        assert!(bridge.create_keymap_profile("Minha caneta"));
        let (_, snapshot) = bridge.keymap_snapshot();
        assert_eq!(snapshot.profiles.len(), 9);
        let last = snapshot.profiles.last().unwrap();
        assert!(last.custom && last.active);
        assert_eq!(last.name, "Minha caneta");
    }

    #[test]
    fn space_swaps_to_the_previous_tool_and_workspace_keys_switch_workspaces() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(crate::UiIntent::SetActiveTool("move".into()));
        bridge.apply(crate::UiIntent::SetActiveTool("rotate".into()));
        assert!(bridge.route_shortcut("Space", false, false, false));
        assert_eq!(bridge.state.session.tools.active_tool, "move");
        assert!(bridge.route_shortcut("Space", false, false, false));
        assert_eq!(bridge.state.session.tools.active_tool, "rotate");

        assert!(bridge.route_shortcut("3", true, false, false));
        assert_eq!(bridge.state.workspace, petunia_core::Workspace::Paint);
        assert_eq!(bridge.state.session.tools.active_tool, "brush");
        assert!(bridge.route_shortcut("1", true, false, false));
        assert_eq!(bridge.state.workspace, petunia_core::Workspace::Model);
        assert_eq!(bridge.modeling_mode, crate::ModelingMode::Draw);
        assert!(bridge.route_shortcut("2", true, false, false));
        assert_eq!(bridge.modeling_mode, crate::ModelingMode::Poly);
        assert!(bridge.route_shortcut("4", true, false, false));
        assert_eq!(bridge.state.workspace, petunia_core::Workspace::Uv);
    }
}
