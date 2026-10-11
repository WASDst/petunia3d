//! Preferências persistidas e opções de apresentação do editor.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Define o modo de confirmação das ferramentas paramétricas.
    pub fn set_tool_activation(&mut self, id: &str) -> bool {
        let Some(mode) = petunia_core::ToolActivation::from_id(id) else {
            return false;
        };
        if self.state.session.tools.tool_activation == mode {
            return false;
        }
        self.state.session.tools.tool_activation = mode;
        self.state.set_status(match mode {
            petunia_core::ToolActivation::Drag => "Tools confirm on pointer release (click + drag)",
            petunia_core::ToolActivation::Instant => {
                "Tools follow the pointer; click or Enter confirms, Esc cancels"
            }
        });
        self.state.mark_dirty();
        true
    }

    /// Preferência de input da aplicação, sem criar undo nem sujar o modelo.
    pub fn set_invert_vertical_drag(&mut self, invert: bool) -> bool {
        if self.state.ui.invert_vertical_drag == invert {
            return false;
        }
        self.state.ui.invert_vertical_drag = invert;
        let status_id = if invert {
            petunia_config::text_id::UI_VERTICAL_DRAG_INVERTED
        } else {
            petunia_config::text_id::UI_VERTICAL_DRAG_NORMAL
        };
        self.state.set_status(self.state.t_id(status_id));
        true
    }

    /// Diferenciação não-cromática de eixos para acessibilidade e daltonismo.
    pub fn set_colorblind_axes(&mut self, enabled: bool) -> bool {
        if self.state.ui.colorblind_axes == enabled {
            return false;
        }
        self.state.ui.colorblind_axes = enabled;
        self.preferences.colorblind_axes = enabled;
        self.state.mark_dirty();
        true
    }

    /// Redução de movimento para usuários com sensibilidade vestibular / labirintite.
    pub fn set_reduced_motion(&mut self, enabled: bool) -> bool {
        if self.state.ui.reduced_motion == enabled {
            return false;
        }
        self.state.ui.reduced_motion = enabled;
        self.preferences.reduced_motion = enabled;
        self.state.mark_dirty();
        true
    }

    /// Exibe tag flutuante com a média de medidas na multiseleção de arestas.
    pub fn set_multiselection_measure_tag(&mut self, enabled: bool) -> bool {
        if self.state.ui.multiselection_measure_tag == enabled {
            return false;
        }
        self.state.ui.multiselection_measure_tag = enabled;
        self.preferences.multiselection_measure_tag = enabled;
        self.state.mark_dirty();
        true
    }

    /// Intervalo máximo em milissegundos para duplo toque de tecla de ferramenta.
    pub fn set_double_tap_interval_ms(&mut self, interval_ms: u64) -> bool {
        let clamped = interval_ms.min(2000);
        if self.preferences.double_tap_interval_ms == clamped {
            return false;
        }
        self.preferences.double_tap_interval_ms = clamped;
        self.state.mark_dirty();
        true
    }

    pub fn set_selection_color_hex(&mut self, value: &str) -> bool {
        let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_INVALID);
            self.state.set_status(message);
            return false;
        }
        let mut rgb = [0u8; 3];
        for (index, channel) in rgb.iter_mut().enumerate() {
            let Some(piece) = hex.get(index * 2..index * 2 + 2) else {
                return false;
            };
            let Ok(parsed) = u8::from_str_radix(piece, 16) else {
                return false;
            };
            *channel = parsed;
        }
        if !selection_color_has_contrast(rgb) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_LOW_CONTRAST);
            self.state.set_status(message);
            return false;
        }
        if colors_too_close(rgb, self.effective_accent_rgb()) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_COLORS_TOO_CLOSE);
            self.state.set_status(message);
            return false;
        }
        if self.state.ui.selection_rgb == rgb {
            return false;
        }
        self.state.ui.selection_rgb = rgb;
        true
    }

    /// Cor de destaque do tema ativo, sem a preferência do usuário.
    pub(crate) fn theme_accent_rgb(&self) -> [u8; 3] {
        let theme = petunia_config::ThemeRegistry::global()
            .get_theme(&self.state.ui.active_theme_id)
            .cloned()
            .unwrap_or_default();
        let [r, g, b, _] = theme
            .colors
            .get_token_color(petunia_config::ThemeToken::AccentBlue)
            .0;
        [r, g, b]
    }

    /// Cor de destaque em uso: a do usuário ou a do tema.
    pub fn effective_accent_rgb(&self) -> [u8; 3] {
        self.preferences
            .accent_rgb
            .unwrap_or_else(|| self.theme_accent_rgb())
    }

    /// Define a cor de destaque da interface (`#RRGGBB`); vazio volta ao tema.
    /// Recusa uma cor parecida demais com a da seleção na viewport (D5).
    pub fn set_accent_color_hex(&mut self, value: &str) -> bool {
        let value = value.trim();
        if value.is_empty() {
            if self.preferences.accent_rgb.is_none() {
                return false;
            }
            self.preferences.accent_rgb = None;
            self.state.mark_dirty();
            return true;
        }
        let Some(rgb) = parse_hex_rgb(value) else {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_INVALID);
            self.state.set_status(message);
            return false;
        };
        if colors_too_close(rgb, self.state.ui.selection_rgb) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_COLORS_TOO_CLOSE);
            self.state.set_status(message);
            return false;
        }
        if self.preferences.accent_rgb == Some(rgb) {
            return false;
        }
        self.preferences.accent_rgb = Some(rgb);
        self.state.mark_dirty();
        true
    }

    /// Define a cor customizada de overlay de seleção de uma parte (`None` para usar a geral).
    pub fn set_part_selection_color(&mut self, asset_id_str: &str, color: Option<[u8; 3]>) -> bool {
        if let Ok(id) = uuid::Uuid::parse_str(asset_id_str) {
            let res = self
                .state
                .project
                .project
                .set_asset_selection_overlay_color(id, color);
            if res {
                self.state.mark_dirty();
            }
            res
        } else {
            false
        }
    }

    pub fn set_selection_thickness(&mut self, thickness: f32) -> bool {
        if !thickness.is_finite() {
            return false;
        }
        let thickness = thickness.clamp(1.0, 6.0);
        if (self.state.ui.selection_thickness - thickness).abs() < f32::EPSILON {
            return false;
        }
        self.state.ui.selection_thickness = thickness;
        true
    }

    pub fn set_language(&mut self, lang: &str) -> bool {
        if self.state.ui.i18n.lang == lang {
            return false;
        }
        self.state.ui.i18n.set_lang(lang);
        self.state.mark_dirty();
        true
    }

    pub fn set_ui_scale(&mut self, scale: f32) -> bool {
        if !scale.is_finite() {
            return false;
        }
        let scale = scale.clamp(1.0, 2.0);
        if (self.preferences.ui_scale - scale).abs() < 0.001 {
            return false;
        }
        self.preferences.ui_scale = scale;
        self.state.mark_dirty();
        true
    }

    /// Define o estilo do tema de ícones ("outline" ou "filled").
    pub fn set_icon_theme(&mut self, theme: &str) -> bool {
        let theme_normalized = match theme {
            "filled" => "filled",
            _ => "outline",
        };
        if self.preferences.icon_theme == theme_normalized {
            return false;
        }
        self.preferences.icon_theme = theme_normalized.to_string();
        self.state.mark_dirty();
        true
    }

    /// Overlay persisted layouts onto the runtime section states.
    /// Called once at startup with the loaded preferences; tests pass
    /// hand-built preferences instead of touching disk.
    /// Sobrepõe os layouts persistidos aos estados das seções.
    /// Chamado uma vez na inicialização com as preferências carregadas; testes
    /// passam preferências construídas à mão em vez de tocar o disco.
    pub fn restore_section_layouts(&mut self, preferences: &petunia_config::UserPreferences) {
        self.preferences = preferences.clone();
        if !preferences.theme_id.is_empty() {
            self.state.ui.active_theme_id = preferences.theme_id.clone();
        }
        self.tool_session
            .set_drag_threshold_px(preferences.drag_threshold_px);
        self.state.session.snap_settings.radius_pixels =
            petunia_core::clamp_snap_radius(preferences.snap_radius_px);
        self.tool_session
            .set_click_move_click(preferences.click_move_click);
        self.state.profile.workplane_prefer_ground = preferences.workplane_prefer_ground;
        if !preferences.active_keymap_id.is_empty() {
            self.state.ui.active_keymap_id = preferences.active_keymap_id.clone();
            self.state.ui.keybinds =
                petunia_config::Keybinds::load_profile(&preferences.active_keymap_id);
        }
        self.section_layouts = section_layout::restore_section_layouts(preferences);
    }

    pub fn set_keymap_profile(&mut self, profile_id: &str) -> bool {
        self.activate_keymap_profile(profile_id)
    }

    /// Refresh the cached preferences from live UI state (section layouts are
    /// owned by the cache itself and left untouched here).
    /// Atualiza o cache de preferências a partir do estado vivo da UI (os layouts
    /// pertencem ao próprio cache e ficam intocados aqui).
    pub(crate) fn sync_preferences_from_state(&mut self) {
        self.preferences.invert_vertical_drag = self.state.ui.invert_vertical_drag;
        self.preferences.colorblind_axes = self.state.ui.colorblind_axes;
        self.preferences.reduced_motion = self.state.ui.reduced_motion;
        self.preferences.multiselection_measure_tag = self.state.ui.multiselection_measure_tag;
        self.preferences.selection_rgb = self.state.ui.selection_rgb;
        self.preferences.selection_thickness = self.state.ui.selection_thickness;
        self.preferences.model_quick_actions = self.state.ui.model_quick_actions.clone();
        self.preferences.active_keymap_id = self.state.ui.active_keymap_id.clone();
        self.preferences.theme_id = self.state.ui.active_theme_id.clone();
    }

    /// Persist runtime section layouts; failures surface as status, never panic.
    /// Persiste os layouts das seções; falhas viram status, nunca pânico.
    pub(crate) fn persist_section_layouts(&mut self) {
        self.sync_preferences_from_state();
        for id in petunia_config::InspectorSectionId::all() {
            let layout = self.section_layouts[section_layout::section_index(id)].clone();
            self.preferences.set_section_layout(id, layout);
        }
        let path = self
            .preferences_path_override
            .clone()
            .unwrap_or_else(petunia_config::UserPreferences::default_path);
        if let Err(error) = self.preferences.save_to_path(&path) {
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_SECTION_LAYOUT_SAVE_FAILED),
                &[("error", format!("{error}"))],
            ));
        }
    }

    /// Distância, em px lógicos, que separa clique de arrasto.
    pub fn set_drag_threshold_px(&mut self, pixels: f32) -> bool {
        if !pixels.is_finite() {
            return false;
        }
        let (minimum, maximum) = petunia_core::DRAG_THRESHOLD_RANGE;
        let pixels = pixels.clamp(minimum, maximum);
        if (self.preferences.drag_threshold_px - pixels).abs() < f32::EPSILON {
            return false;
        }
        self.preferences.drag_threshold_px = pixels;
        self.tool_session.set_drag_threshold_px(pixels);
        self.state.mark_dirty();
        true
    }

    /// Raio do snap em px lógicos (acessibilidade motora).
    pub fn set_snap_radius_px(&mut self, pixels: f32) -> bool {
        if !pixels.is_finite() {
            return false;
        }
        let pixels = petunia_core::clamp_snap_radius(pixels);
        if (self.preferences.snap_radius_px - pixels).abs() < f32::EPSILON {
            return false;
        }
        self.preferences.snap_radius_px = pixels;
        self.state.session.snap_settings.radius_pixels = pixels;
        self.state.mark_dirty();
        true
    }

    /// Clicar numa alça a faz seguir o ponteiro até o próximo clique.
    pub fn set_click_move_click(&mut self, enabled: bool) -> bool {
        if self.preferences.click_move_click == enabled {
            return false;
        }
        self.preferences.click_move_click = enabled;
        self.tool_session.set_click_move_click(enabled);
        self.state.mark_dirty();
        true
    }

    /// Mostrar a Home ao abrir o programa sem projeto.
    pub fn set_show_home_on_start(&mut self, enabled: bool) -> bool {
        if self.preferences.show_home_on_start == enabled {
            return false;
        }
        self.preferences.show_home_on_start = enabled;
        true
    }

    /// Trilho de ferramentas com nomes ao lado dos ícones (plano de UI, D6).
    pub fn set_show_tool_labels(&mut self, enabled: bool) -> bool {
        if self.preferences.show_tool_labels == enabled {
            return false;
        }
        self.preferences.show_tool_labels = enabled;
        self.state.mark_dirty();
        true
    }

    /// Luz de estúdio presa à câmera (padrão) ou fixa no mundo.
    pub fn set_studio_light_follows_camera(&mut self, follows: bool) -> bool {
        if self.preferences.studio_light_follows_camera == follows {
            return false;
        }
        self.preferences.studio_light_follows_camera = follows;
        self.state.render.mark_dirty();
        self.state.mark_dirty();
        true
    }

    /// Matcap procedural no Solid (preferência persistida).
    pub fn set_viewport_matcap(&mut self, enabled: bool) -> bool {
        if self.preferences.viewport_matcap == enabled {
            return false;
        }
        self.preferences.viewport_matcap = enabled;
        self.state.render.mark_dirty();
        self.state.mark_dirty();
        true
    }

    /// Oclusão ambiente (GTAO) no Solid e no Material (preferência persistida).
    pub fn set_viewport_ambient_occlusion(&mut self, enabled: bool) -> bool {
        if self.preferences.viewport_ambient_occlusion == enabled {
            return false;
        }
        self.preferences.viewport_ambient_occlusion = enabled;
        self.state.render.mark_dirty();
        self.state.mark_dirty();
        true
    }

    /// Plano automático do Draw: favorecer o chão (SketchUp) ou a vista (Modo/C4D).
    pub fn set_workplane_prefer_ground(&mut self, enabled: bool) -> bool {
        if self.preferences.workplane_prefer_ground == enabled {
            return false;
        }
        self.preferences.workplane_prefer_ground = enabled;
        self.state.profile.workplane_prefer_ground = enabled;
        self.state.mark_dirty();
        true
    }
}
