//! Gerenciador de imagens de referência.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Abre o painel flutuante do Gerenciador de Referências (P3D-013).
    /// Não é modal: não prende foco nem bloqueia Esc além do topo da pilha.
    /// Fica pinado porque se trabalha com ele aberto: click-away não fecha,
    /// só Esc ou o botão de fechar.
    pub fn open_reference_manager(&mut self) -> bool {
        self.reference_manager_open = true;
        self.overlays.push(OverlayEntry {
            id: OverlayId::ReferenceManager,
            kind: OverlayKind::FloatingPanel,
            pinned: true,
            dismiss_on_escape: true,
            dismiss_on_click_away: false,
        });
        self.state.set_status(self.state.t("sl.reference_images"));
        true
    }

    /// Fecha o painel do Gerenciador de Referências.
    pub fn close_reference_manager(&mut self) -> bool {
        self.reference_manager_open = false;
        self.overlays.remove(OverlayId::ReferenceManager);
        true
    }

    /// Alterna a visibilidade do painel do Gerenciador de Referências.
    pub fn toggle_reference_manager(&mut self) -> bool {
        if self.reference_manager_open {
            self.close_reference_manager()
        } else {
            self.open_reference_manager()
        }
    }

    /// Carrega uma imagem de referência para o slot correspondente (ou custom).
    pub fn load_reference_slot(
        &mut self,
        axis_str: &str,
        name: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => petunia_core::RefAxis::Front,
        };
        let axis_key = axis_str.to_lowercase();
        self.reference_thumbnails
            .insert(axis_key, (width, height, rgba.clone()));
        if axis_str == "custom" {
            petunia_core::project_service::ProjectService::add_reference_image(
                &mut self.state,
                name,
                width,
                height,
                rgba,
            );
        } else {
            petunia_core::project_service::ProjectService::set_reference_slot(
                &mut self.state,
                axis,
                name,
                width,
                height,
                rgba,
            );
        }
        self.state.mark_dirty();
        true
    }

    /// Remove a imagem de referência associada ao slot indicado.
    pub fn remove_reference_slot(&mut self, axis_str: &str) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        self.reference_thumbnails.remove(&axis_str.to_lowercase());
        if let Some(idx) = self.state.project.refs.iter().position(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            petunia_core::project_service::ProjectService::remove_reference(&mut self.state, idx);
            true
        } else {
            false
        }
    }

    /// Alinha a câmera do viewport à vista ortográfica correspondente ao slot.
    pub fn align_reference_view(&mut self, axis_str: &str) -> bool {
        let preset = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::ViewPreset::Front,
            "back" => petunia_core::ViewPreset::Back,
            "left" => petunia_core::ViewPreset::Left,
            "right" => petunia_core::ViewPreset::Right,
            "top" => petunia_core::ViewPreset::Top,
            "bottom" => petunia_core::ViewPreset::Bottom,
            _ => return false,
        };
        self.state.session.camera.set_preset(preset);
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_VIEW_ALIGNED_TO),
            &[("0", preset.title().to_string())],
        ));
        self.state.mark_dirty();
        true
    }

    /// Alterna a visibilidade da imagem de referência no slot.
    pub fn toggle_reference_visible(&mut self, axis_str: &str) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        if let Some(r) = self.state.project.refs.iter_mut().find(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            r.visible = !r.visible;
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Alterna o bloqueio de edição/transformação da imagem de referência.
    pub fn toggle_reference_lock(&mut self, axis_str: &str) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        if let Some(r) = self.state.project.refs.iter_mut().find(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            r.locked = !r.locked;
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Ajusta um parâmetro de calibração fina da referência (opacity, size, offset, rotation).
    pub fn set_reference_param(&mut self, axis_str: &str, param: &str, val: f32) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        if let Some(r) = self.state.project.refs.iter_mut().find(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            match param {
                "opacity" => r.opacity = val.clamp(0.0, 1.0),
                "size" => r.size = val.clamp(0.1, 50.0),
                "offset" => r.offset = val.clamp(-50.0, 50.0),
                "rotation" => r.rotation = val.clamp(-180.0, 180.0),
                _ => return false,
            }
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Limpa todas as referências da cena.
    pub fn clear_all_references(&mut self) -> bool {
        petunia_core::project_service::ProjectService::clear_references(&mut self.state);
        self.reference_thumbnails.clear();
        true
    }
}
