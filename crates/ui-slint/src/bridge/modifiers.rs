//! Pilha de modificadores.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Asset displayed by a section: the pinned asset when set and resolvable,
    /// otherwise the active selection. Fail-safe fallback, never panics.
    /// Asset exibido por uma seção: o asset fixado quando definido e resolvível,
    /// senão a seleção ativa. Fallback fail-safe, nunca pânico.
    pub fn section_asset(
        &self,
        section: petunia_config::InspectorSectionId,
    ) -> Option<&petunia_project::Asset> {
        self.section_layouts[section_layout::section_index(section)]
            .pinned_asset
            .as_deref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .and_then(|id| {
                self.state
                    .project
                    .assets
                    .iter()
                    .find(|asset| asset.id == id)
            })
            .or_else(|| self.state.project.active())
    }

    /// Index of the asset owning a modifier id (any asset, not just the active
    /// one) so sections pinned to another asset act on the displayed stack.
    /// Modifier ids are unique; the active path behaves exactly as before.
    /// Índice do asset dono de um modifier (qualquer asset, não só o ativo)
    /// para seções fixadas agirem na pilha exibida. Ids são únicos; o caminho
    /// ativo comporta-se exatamente como antes.
    pub(crate) fn find_modifier_owner(&self, id: uuid::Uuid) -> Option<usize> {
        self.state
            .project
            .assets
            .iter()
            .position(|asset| asset.modifiers.iter().any(|modifier| modifier.id == id))
    }

    pub fn add_modifier(&mut self, kind: &str) -> bool {
        // Creation follows the displayed Modifiers section: pinned asset when
        // set, active selection otherwise. / A criação segue a seção exibida.
        let asset_index = self
            .section_asset(petunia_config::InspectorSectionId::Modifiers)
            .map(|asset| asset.id)
            .and_then(|id| {
                self.state
                    .project
                    .assets
                    .iter()
                    .position(|asset| asset.id == id)
            });
        let Some(asset_index) = asset_index else {
            return false;
        };
        if self.state.project.assets.get(asset_index).is_none() {
            return false;
        }
        let modifier = match kind {
            "mirror" => petunia_project::ModifierInstance::mirror(0, 0.001),
            "symmetry" => petunia_project::ModifierInstance::symmetry(0, true, 0.001),
            _ => return false,
        };
        self.state.checkpoint("add modifier");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.modifiers.push(modifier);
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if modifier.enabled == enabled {
            return false;
        }
        self.state.checkpoint("toggle modifier");
        if let Some(modifier) = self.state.project.assets[asset_index]
            .modifiers
            .iter_mut()
            .find(|item| item.id == id)
        {
            modifier.enabled = enabled;
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn remove_modifier(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        if !self.state.project.assets[asset_index]
            .modifiers
            .iter()
            .any(|item| item.id == id)
        {
            return false;
        }
        self.state.checkpoint("remove modifier");
        self.state.project.assets[asset_index]
            .modifiers
            .retain(|item| item.id != id);
        self.state.emit_mesh_changed();
        true
    }

    pub fn move_modifier(&mut self, id: &str, direction: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let Some(current) = self.state.project.assets[asset_index]
            .modifiers
            .iter()
            .position(|item| item.id == id)
        else {
            return false;
        };
        let target = if direction < 0 {
            if current == 0 {
                return false;
            }
            current - 1
        } else {
            current + 1
        };
        if target >= self.state.project.assets[asset_index].modifiers.len() {
            return false;
        }
        self.state.checkpoint("reorder modifier");
        self.state.project.assets[asset_index]
            .modifiers
            .swap(current, target);
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_axis(&mut self, id: &str, axis: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let axis_value = (axis.clamp(0, 2)) as usize;
        self.state.checkpoint("change modifier axis");
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        match &mut modifier.kind {
            petunia_project::ModifierKind::Mirror { axis, .. } => *axis = axis_value,
            petunia_project::ModifierKind::Symmetry { axis, .. } => *axis = axis_value,
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_direction(&mut self, id: &str, direction_value: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        self.state.checkpoint("change modifier direction");
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        match &mut modifier.kind {
            petunia_project::ModifierKind::Symmetry {
                positive_to_negative,
                ..
            } => *positive_to_negative = direction_value,
            petunia_project::ModifierKind::Mirror { .. } => return false,
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn apply_modifier(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let Some(asset) = self.state.project.assets.get(asset_index) else {
            return false;
        };
        if !asset.modifiers.iter().any(|item| item.id == id) {
            return false;
        }
        let evaluated = asset.evaluated_mesh();
        self.state.checkpoint("apply modifier");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.mesh = evaluated.into();
            asset.modifiers.retain(|item| item.id != id);
        }
        self.state.emit_mesh_changed();
        true
    }
}
