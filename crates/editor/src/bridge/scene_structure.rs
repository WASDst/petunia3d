//! Estrutura da cena: grupos, arrastar e soltar, renomear grupo e isolar uma
//! parte. Cada ação é um comando do núcleo (com Undo); aqui só se traduz a
//! intenção da lista em comandos.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    fn run_scene_command(&mut self, command: &dyn petunia_core::Command) -> bool {
        match self.state.dispatch(command) {
            Ok(()) => true,
            Err(petunia_core::CommandError::NoChange(_)) => false,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// **Agrupar** (Ctrl+G): a seleção vira um grupo novo.
    pub fn group_selection(&mut self) -> bool {
        self.run_scene_command(&petunia_core::GroupSelectionCmd::default())
    }

    /// **Desagrupar** (Ctrl+Shift+G): dissolve o grupo nomeado ou o do ativo.
    pub fn ungroup(&mut self, name: Option<&str>) -> bool {
        self.run_scene_command(&petunia_core::UngroupCmd {
            collection: name.map(str::to_string),
        })
    }

    /// Soltar objetos na lista: reordena antes de `before` (ou no fim) e troca o
    /// grupo conforme `collection` (`Some(None)` tira do grupo).
    pub fn drop_scene_items(
        &mut self,
        ids: &[String],
        before: Option<&str>,
        collection: Option<Option<String>>,
    ) -> bool {
        let parse = |raw: &str| uuid::Uuid::parse_str(raw).ok();
        let ids: Vec<uuid::Uuid> = ids.iter().filter_map(|raw| parse(raw)).collect();
        if ids.is_empty() {
            return false;
        }
        let before = before.and_then(parse);
        self.run_scene_command(&petunia_core::MoveAssetsCmd {
            ids,
            before,
            collection,
        })
    }

    pub fn rename_group(&mut self, from: &str, to: &str) -> bool {
        self.run_scene_command(&petunia_core::RenameGroupCmd {
            from: from.to_string(),
            to: to.to_string(),
        })
    }

    /// Cor de seleção do grupo; `None` a remove. `force` faz o grupo vencer a cor
    /// própria de cada objeto (por padrão o objeto vence).
    pub fn set_group_color(&mut self, name: &str, color: Option<[u8; 3]>, force: bool) -> bool {
        self.run_scene_command(&petunia_core::SetGroupColorCmd {
            name: name.to_string(),
            color,
            force,
        })
    }

    pub fn toggle_group_visibility(&mut self, name: &str) -> bool {
        self.run_scene_command(&petunia_core::ToggleCollectionVisibilityCmd {
            collection: name.to_string(),
        })
    }

    pub fn toggle_group_lock(&mut self, name: &str) -> bool {
        self.run_scene_command(&petunia_core::ToggleCollectionLockCmd {
            collection: name.to_string(),
        })
    }

    /// Seleciona os objetos do grupo (substitui a seleção).
    pub fn select_group(&mut self, name: &str) -> bool {
        let indices: Vec<usize> = self
            .state
            .project
            .assets
            .iter()
            .enumerate()
            .filter(|(_, asset)| asset.collection.as_deref() == Some(name))
            .map(|(index, _)| index)
            .collect();
        if indices.is_empty() {
            return false;
        }
        self.cancel_active_operation();
        self.active_profile_id = None;
        self.state.select_object(None, false);
        for index in indices {
            self.state.select_object(Some(index), true);
        }
        self.reset_transform_fields();
        true
    }

    /// Isola a parte da linha: ela entra na seleção (se ainda não estava) e o
    /// isolamento liga/desliga para a seleção inteira.
    pub fn isolate_part(&mut self, id: &str) -> bool {
        if !self.state.session.isolate_active
            && let Ok(uuid) = uuid::Uuid::parse_str(id)
            && !self.state.session.selection.assets.contains(&uuid)
            && let Some(index) = self.state.project.find(uuid)
        {
            self.cancel_active_operation();
            self.state.select_object(Some(index), false);
        }
        self.apply(UiIntent::ToggleIsolateActiveAsset);
        true
    }
}
