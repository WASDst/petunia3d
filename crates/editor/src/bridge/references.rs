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
        let Some(axis) = reference_axis(axis_str) else {
            return false;
        };
        let mut image =
            petunia_project::ReferenceImage::from_rgba(name, width, height, rgba.clone());
        image.axis = axis;
        let op = if axis_str == "custom" {
            petunia_core::ReferenceOp::Append(image)
        } else {
            petunia_core::ReferenceOp::SetSlot(image)
        };
        if !self.dispatch_reference(axis, op) {
            return false;
        }
        self.reference_thumbnails
            .insert(axis_str.to_lowercase(), (width, height, rgba));
        true
    }

    /// Remove a imagem de referência associada ao slot indicado.
    pub fn remove_reference_slot(&mut self, axis_str: &str) -> bool {
        let Some(axis) = reference_axis(axis_str) else {
            return false;
        };
        let ok = self.dispatch_reference(axis, petunia_core::ReferenceOp::Remove);
        if ok {
            self.reference_thumbnails.remove(&axis_str.to_lowercase());
        }
        ok
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
        let Some(axis) = reference_axis(axis_str) else {
            return false;
        };
        let Some(r) = self
            .state
            .project
            .refs
            .iter()
            .find(|r| reference_axis_matches(r.axis, axis))
        else {
            return false;
        };
        let value = !r.visible;
        self.dispatch_reference(axis, petunia_core::ReferenceOp::Visible(value))
    }

    /// Alterna o bloqueio de edição/transformação da imagem de referência.
    pub fn toggle_reference_lock(&mut self, axis_str: &str) -> bool {
        let Some(axis) = reference_axis(axis_str) else {
            return false;
        };
        let Some(r) = self
            .state
            .project
            .refs
            .iter()
            .find(|r| reference_axis_matches(r.axis, axis))
        else {
            return false;
        };
        let value = !r.locked;
        self.dispatch_reference(axis, petunia_core::ReferenceOp::Locked(value))
    }

    /// Ajusta um parâmetro de calibração fina da referência (opacity, size, offset, rotation).
    pub fn set_reference_param(&mut self, axis_str: &str, param: &str, val: f32) -> bool {
        let Some(axis) = reference_axis(axis_str) else {
            return false;
        };
        self.dispatch_reference(
            axis,
            petunia_core::ReferenceOp::Parameter {
                field: param.into(),
                value: val,
            },
        )
    }

    /// Limpa todas as referências da cena.
    pub fn clear_all_references(&mut self) -> bool {
        let result = self.dispatch_reference(
            petunia_core::RefAxis::Front,
            petunia_core::ReferenceOp::Clear,
        );
        if result {
            self.reference_thumbnails.clear();
        }
        result
    }
}

fn reference_axis(value: &str) -> Option<petunia_core::RefAxis> {
    Some(match value.to_lowercase().as_str() {
        "front" | "custom" => petunia_core::RefAxis::Front,
        "back" => petunia_core::RefAxis::Back,
        "left" => petunia_core::RefAxis::Left,
        "right" => petunia_core::RefAxis::Right,
        "top" => petunia_core::RefAxis::Top,
        "bottom" => petunia_core::RefAxis::Bottom,
        _ => return None,
    })
}
fn reference_axis_matches(a: petunia_core::RefAxis, b: petunia_core::RefAxis) -> bool {
    a == b || (a == petunia_core::RefAxis::Side && b == petunia_core::RefAxis::Right)
}
impl<V: PetuniaViewport> SlintUiBridge<V> {
    fn dispatch_reference(
        &mut self,
        axis: petunia_core::RefAxis,
        op: petunia_core::ReferenceOp,
    ) -> bool {
        let revision = self
            .state
            .project
            .refs
            .iter()
            .find(|r| reference_axis_matches(r.axis, axis))
            .map(|r| r.revision);
        match self.state.dispatch(&petunia_core::EditReferenceCmd {
            axis,
            expected_revision: revision,
            op,
        }) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }
}
