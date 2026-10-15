//! Outliner, renomear, filtros de assets e prefabs.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_asset_query(&mut self, query: &str) -> bool {
        if self.asset_query == query {
            return false;
        }
        self.asset_query = query.to_string();
        true
    }

    pub fn set_parts_query(&mut self, query: &str) -> bool {
        if self.parts_query == query {
            return false;
        }
        self.parts_query = query.to_string();
        true
    }

    pub fn set_parts_selected_only(&mut self, enabled: bool) -> bool {
        if self.parts_selected_only == enabled {
            return false;
        }
        self.parts_selected_only = enabled;
        true
    }

    pub fn set_parts_sort_by_name(&mut self, enabled: bool) -> bool {
        if self.parts_sort_by_name == enabled {
            return false;
        }
        self.parts_sort_by_name = enabled;
        true
    }

    pub fn set_asset_only_favorites(&mut self, only: bool) -> bool {
        if self.asset_only_favorites == only {
            return false;
        }
        self.asset_only_favorites = only;
        true
    }

    /// Prefabs da biblioteca filtrados/ordenados pela busca atual, com miniatura em cache.
    pub(crate) fn prefab_item_models(&self) -> Vec<view_model::PrefabItemModel> {
        use petunia_project::model_library::{ModelLibraryQuery, ModelLibrarySort};
        let query = ModelLibraryQuery {
            search: self.asset_query.clone(),
            only_favorites: self.asset_only_favorites,
            sort: if self.asset_sort_by_name {
                ModelLibrarySort::NameAsc
            } else {
                ModelLibrarySort::IndexAsc
            },
            ..Default::default()
        };
        let project = &self.state.project;
        let mut thumbs = self.prefab_thumbs.borrow_mut();
        let live: std::collections::HashSet<(uuid::Uuid, u32)> =
            project.prefabs.iter().map(|p| (p.id, p.revision)).collect();
        thumbs.retain(|key, _| live.contains(key));
        project
            .prefab_summaries(&query)
            .into_iter()
            .map(|summary| {
                let prefab = &project.prefabs[summary.original_index];
                let thumbnail = thumbs
                    .entry((prefab.id, prefab.revision))
                    .or_insert_with(|| {
                        thumbnail::render_prefab_thumbnail(prefab).map(std::sync::Arc::new)
                    })
                    .clone();
                view_model::PrefabItemModel {
                    id: summary.id.to_string(),
                    name: summary.name,
                    parts: summary.parts,
                    tris: summary.triangles,
                    verts: summary.vertices,
                    favorite: summary.favorite,
                    instances: summary.instances,
                    tags: summary.tags.join(", "),
                    thumbnail,
                }
            })
            .collect()
    }

    /// Instancia o prefab no 3D Cursor e seleciona as cópias.
    pub fn place_prefab(&mut self, id: &str) -> bool {
        let Ok(prefab) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.sync_viewport_context();
        self.state.instantiate_prefab(prefab, None)
    }

    pub fn delete_prefab(&mut self, id: &str) -> bool {
        uuid::Uuid::parse_str(id).is_ok_and(|prefab| self.state.delete_prefab(prefab))
    }

    pub fn set_prefab_favorite(&mut self, id: &str, favorite: bool) -> bool {
        uuid::Uuid::parse_str(id)
            .is_ok_and(|prefab| self.state.set_prefab_favorite(prefab, favorite))
    }

    pub fn update_prefab(&mut self, id: &str) -> bool {
        uuid::Uuid::parse_str(id)
            .is_ok_and(|prefab| self.state.update_prefab_from_selection(prefab))
    }

    pub fn set_asset_sort_by_name(&mut self, sort_by_name: bool) -> bool {
        if self.asset_sort_by_name == sort_by_name {
            return false;
        }
        self.asset_sort_by_name = sort_by_name;
        true
    }

    pub fn set_asset_thumbnail_size(&mut self, size: f32) -> bool {
        if !size.is_finite() {
            return false;
        }
        let size = size.clamp(48.0, 128.0);
        if (self.state.ui.asset_thumbnail_size - size).abs() < f32::EPSILON {
            return false;
        }
        self.state.ui.asset_thumbnail_size = size;
        true
    }

    pub(crate) fn select_asset_by_id(&mut self, asset: uuid::Uuid) {
        if let Some(index) = self
            .state
            .project
            .assets
            .iter()
            .position(|candidate| candidate.id == asset)
        {
            self.cancel_active_operation();
            self.state.select_object(Some(index), false);
        }
    }

    pub(crate) fn toggle_asset_visibility(&mut self, id: &str) {
        self.apply(UiIntent::ToggleSceneAssetVisibility(id.to_string()));
    }

    pub(crate) fn toggle_asset_lock(&mut self, id: &str) {
        self.apply(UiIntent::ToggleSceneAssetLock(id.to_string()));
    }

    /// Abre a edição inline do nome do ativo selecionado.
    pub fn begin_rename(&mut self) -> bool {
        if self.rename_draft.is_some() {
            return true;
        }
        let Some(asset) = self.state.project.active() else {
            self.state
                .set_status(petunia_core::AssetRenameError::NoActiveAsset.to_string());
            return false;
        };
        self.rename_draft = Some(asset.name.clone());
        self.state.mark_dirty();
        true
    }

    /// Confirma o nome em edição. O domínio decide validade e histórico.
    pub fn commit_rename(&mut self, name: &str) -> bool {
        if self.rename_draft.take().is_none() {
            return false;
        }
        match self.state.rename_active_asset(name) {
            Ok(_) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Abandona a edição sem tocar no documento.
    pub fn cancel_rename(&mut self) -> bool {
        if self.rename_draft.take().is_none() {
            return false;
        }
        self.state.mark_dirty();
        true
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Stable scene row selection shared by native toolkit adapters.
    pub fn select_scene_item(&mut self, id: &str, extend: bool) -> bool {
        if let Some(raw) = id.strip_prefix("profile:") {
            let Ok(id) = uuid::Uuid::parse_str(raw) else {
                return false;
            };
            if !self
                .state
                .project
                .project
                .profiles
                .iter()
                .any(|p| p.id == id)
            {
                return false;
            }
            self.cancel_active_operation();
            let selected = self.activate_profile(id);
            self.state.select_object(None, false);
            self.reset_transform_fields();
            selected
        } else {
            let Some(index) = self
                .state
                .project
                .assets
                .iter()
                .position(|a| a.id.to_string() == id)
            else {
                return false;
            };
            self.cancel_active_operation();
            self.active_profile_id = None;
            self.state.select_object(Some(index), extend);
            self.reset_transform_fields();
            true
        }
    }
}
