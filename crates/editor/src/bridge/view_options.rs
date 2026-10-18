//! Câmera, presets de vista, split view e opções de shading.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Define o modo de sombreamento da viewport pelo id estável.
    ///
    /// Cada modo corresponde a um pipeline real: Wireframe não preenche,
    /// Solid usa o estúdio da viewport, MaterialPreview amostra a textura e
    /// Rendered usa a luz da cena.
    pub fn set_shading_mode(&mut self, id: &str) -> bool {
        let Some(mode) = petunia_core::Shading::from_id(id) else {
            return false;
        };
        if self.state.shading == mode {
            return false;
        }
        self.state.shading = mode;
        // Material/Rendered dependem de amostrar o material.
        if mode.samples_material() {
            self.state.session.textured = true;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_SHADING),
            &[("0", mode.id().to_string())],
        ));
        self.state.mark_dirty();
        true
    }

    /// Opacidade da geometria em X-Ray.
    pub fn set_xray_opacity(&mut self, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let clamped = opacity.clamp(0.1, 0.9);
        if (clamped - self.state.session.xray_opacity).abs() < f32::EPSILON {
            return false;
        }
        self.state.session.xray_opacity = clamped;
        self.state.mark_dirty();
        true
    }

    /// Alterna a pílula do Micro-Inspector flutuante sob o cursor (Espaço).
    pub fn toggle_micro_inspector(&mut self) -> bool {
        if self.micro_inspector_open {
            self.micro_inspector_open = false;
            self.overlays.remove(OverlayId::MicroInspector);
        } else {
            self.micro_inspector_open = true;
            self.micro_inspector_pos = self.pointer_position;
            self.overlays.push(OverlayEntry {
                id: OverlayId::MicroInspector,
                kind: OverlayKind::FloatingPanel,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            });
        }
        self.state.mark_dirty();
        true
    }

    /// Liga/desliga a viewport dividida. Recusa (com aviso) quando a vista
    /// principal ficaria abaixo de `MIN_SPLIT_VIEW_WIDTH` por metade.
    pub fn toggle_split_view(&mut self) -> bool {
        if self.split.enabled {
            self.split.close();
            return true;
        }
        if self.viewport_size[0] < 2.0 * split_view::MIN_SPLIT_VIEW_WIDTH {
            self.state.set_status(self.state.t("sl.split_too_narrow"));
            return false;
        }
        self.split.open(&self.state.session.camera);
        true
    }

    pub fn set_split_preset(&mut self, id: &str) -> bool {
        use petunia_core::ViewPreset;
        let preset = match id {
            "persp" => ViewPreset::Persp,
            "front" => ViewPreset::Front,
            "back" => ViewPreset::Back,
            "left" => ViewPreset::Left,
            "right" => ViewPreset::Right,
            "top" => ViewPreset::Top,
            "iso21" | "2:1" | "pseudo_iso" => ViewPreset::PseudoIsometric2To1,
            _ => return false,
        };
        self.split.set_preset(preset);
        true
    }

    /// Quadro da vista secundária, apenas quando algo mudou.
    pub fn render_split_view(&mut self) -> bool {
        if !self.split.enabled {
            return false;
        }
        let state = self.viewport_render_state();
        self.split
            .render(&self.state.project, &self.state.project.refs, state)
    }

    pub fn toggle_paint_pip(&mut self) -> bool {
        self.paint_pip = !self.paint_pip;
        self.paint_pip
    }

    /// Alinha a câmera a um eixo a partir do tripé de navegação.
    pub fn snap_view_to_axis(&mut self, axis: &str) -> bool {
        use petunia_core::ViewPreset;
        let preset = match axis {
            "x" => ViewPreset::Right,
            "y" => ViewPreset::Top,
            "z" => ViewPreset::Front,
            _ => return false,
        };
        self.set_view_preset(preset)
    }

    /// Define diretamente o preset de visualização da câmera e notifica o estado.
    pub fn set_view_preset(&mut self, preset: petunia_core::ViewPreset) -> bool {
        self.leave_planar_navigation_for_view_change();
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::command::SetViewPresetCmd(preset))
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_VIEW),
            &[("0", preset.title().to_string())],
        ));
        self.state.mark_dirty();
        true
    }

    /// Alterna a visualização para a vista oposta (ex: Front -> Back, Top -> Bottom).
    pub fn toggle_view_opposite(&mut self) -> bool {
        self.leave_planar_navigation_for_view_change();
        use petunia_core::ViewPreset;
        let next = match self.state.session.camera.view_preset() {
            Some(ViewPreset::Front) => ViewPreset::Back,
            Some(ViewPreset::Back) => ViewPreset::Front,
            Some(ViewPreset::Right) => ViewPreset::Left,
            Some(ViewPreset::Left) => ViewPreset::Right,
            Some(ViewPreset::Top) => ViewPreset::Bottom,
            Some(ViewPreset::Bottom) => ViewPreset::Top,
            _ => {
                self.state.session.camera.planar_basis = None;
                self.state.session.camera.yaw = (self.state.session.camera.yaw
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU);
                self.state.mark_dirty();
                return true;
            }
        };
        self.set_view_preset(next)
    }

    pub fn toggle_face_orientation(&mut self) -> bool {
        self.state.session.show_face_orientation = !self.state.session.show_face_orientation;
        let enabled = self.state.session.show_face_orientation;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Orientação de faces (azul/vermelho) ativada"
        } else {
            "Orientação de faces desativada"
        });
        enabled
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// An explicit camera change enters free navigation, retaining the current
    /// view and the construction plane. Align View re-enters planar navigation.
    pub(crate) fn leave_planar_navigation_for_view_change(&mut self) {
        if self.planar_navigation_active() {
            self.state.session.draw_planar_navigation = false;
            self.state.session.planar_saved_camera = None;
        }
    }
}
