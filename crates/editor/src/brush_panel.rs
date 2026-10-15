//! Pincel: parâmetros de estilo e presets (iniciativa Paint, 2E).
//!
//! O markup só emite `(nome, valor)` e lê um [`BrushPanelModel`] pronto; as
//! regras (limites, qual ferramenta acompanha qual tipo de pincel, persistência
//! dos presets do usuário) vivem aqui e em `petunia_core::brush`.

use petunia_core::{BRUSH_PX_PER_UNIT, BrushBlend, BrushPreset, BrushTip, BrushType};

use crate::{PetuniaViewport, SlintUiBridge, UiIntent};

/// Valores do painel de pincel para o shell.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BrushPanelModel {
    pub flow: f32,
    pub spacing: f32,
    pub smoothing: f32,
    pub size_jitter: f32,
    pub opacity_jitter: f32,
    pub scatter: f32,
    pub spray_density: f32,
    pub angle: f32,
    pub roundness: f32,
    pub tip: String,
    pub blend: String,
    pub presets: Vec<String>,
    pub active_preset: i32,
    pub clone_source_set: bool,
}

/// Ferramenta de pintura que acompanha um tipo de pincel.
pub fn tool_id_for_brush(kind: BrushType) -> &'static str {
    match kind {
        BrushType::Pixel => "pixel",
        BrushType::Soft => "brush",
        BrushType::Eraser => "eraser",
        BrushType::Airbrush => "airbrush",
        BrushType::Smudge => "smudge",
        BrushType::Blur => "blur",
        BrushType::Dodge => "dodge",
        BrushType::Burn => "burn",
        BrushType::Spray => "spray",
        BrushType::Clone => "clone",
        BrushType::Fill => "fill",
        BrushType::Eyedropper => "picker",
        BrushType::Line => "line",
        BrushType::Rectangle => "rectangle",
        BrushType::Ellipse => "ellipse",
    }
}

fn tip_id(tip: BrushTip) -> &'static str {
    match tip {
        BrushTip::Round => "round",
        BrushTip::Square => "square",
        BrushTip::Diamond => "diamond",
    }
}

fn blend_id(blend: BrushBlend) -> &'static str {
    match blend {
        BrushBlend::Normal => "normal",
        BrushBlend::Multiply => "multiply",
        BrushBlend::Screen => "screen",
        BrushBlend::Add => "add",
        BrushBlend::Darken => "darken",
        BrushBlend::Lighten => "lighten",
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    fn brush_presets_path(&self) -> std::path::PathBuf {
        self.brush_presets_path_override
            .clone()
            .unwrap_or_else(BrushPreset::user_path)
    }

    fn ensure_brush_presets_loaded(&mut self) {
        if !self.brush_presets_loaded {
            self.brush_presets = BrushPreset::load_user_from(&self.brush_presets_path());
            self.brush_presets_loaded = true;
        }
    }

    /// Presets embutidos seguidos dos do usuário.
    pub fn all_brush_presets(&mut self) -> Vec<BrushPreset> {
        self.ensure_brush_presets_loaded();
        let mut all = BrushPreset::builtin();
        all.extend(self.brush_presets.iter().cloned());
        all
    }

    /// Valores atuais do painel (sem I/O: os presets só aparecem depois de carregados).
    pub fn brush_panel_model(&self) -> BrushPanelModel {
        let tools = &self.state.session.tools;
        let style = tools.brush_style;
        let mut names: Vec<String> = BrushPreset::builtin().into_iter().map(|p| p.name).collect();
        names.extend(self.brush_presets.iter().map(|p| p.name.clone()));
        BrushPanelModel {
            flow: tools.brush_flow,
            spacing: tools.brush_spacing,
            smoothing: style.smoothing,
            size_jitter: style.size_jitter,
            opacity_jitter: style.opacity_jitter,
            scatter: style.scatter,
            spray_density: style.spray_density,
            angle: style.angle_deg,
            roundness: style.roundness,
            tip: tip_id(style.tip).to_string(),
            blend: blend_id(style.blend).to_string(),
            presets: names,
            active_preset: self.active_brush_preset.map_or(-1, |i| i as i32),
            clone_source_set: tools.clone_source.is_some(),
        }
    }

    /// Ajusta um parâmetro do pincel por nome. `false` se o nome é desconhecido.
    pub fn set_brush_param(&mut self, name: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let tools = &mut self.state.session.tools;
        match name {
            "flow" => tools.brush_flow = value.clamp(0.0, 1.0),
            "spacing" => tools.brush_spacing = value.clamp(0.01, 1.0),
            "smoothing" => tools.brush_style.smoothing = value.clamp(0.0, 0.95),
            "size_jitter" => tools.brush_style.size_jitter = value.clamp(0.0, 1.0),
            "opacity_jitter" => tools.brush_style.opacity_jitter = value.clamp(0.0, 1.0),
            "scatter" => tools.brush_style.scatter = value.clamp(0.0, 2.0),
            "spray_density" => tools.brush_style.spray_density = value.clamp(0.05, 1.0),
            "angle" => tools.brush_style.angle_deg = value.clamp(-180.0, 180.0),
            "roundness" => tools.brush_style.roundness = value.clamp(0.1, 1.0),
            _ => return false,
        }
        self.active_brush_preset = None;
        self.state.mark_dirty();
        true
    }

    /// Escolhe a ponta (`round`, `square`, `diamond`).
    pub fn set_brush_tip(&mut self, tip: &str) -> bool {
        let tip = match tip {
            "round" => BrushTip::Round,
            "square" => BrushTip::Square,
            "diamond" => BrushTip::Diamond,
            _ => return false,
        };
        self.state.session.tools.brush_style.tip = tip;
        self.active_brush_preset = None;
        self.state.mark_dirty();
        true
    }

    /// Escolhe o modo de mistura da cor do pincel.
    pub fn set_brush_blend(&mut self, blend: &str) -> bool {
        let blend = match blend {
            "normal" => BrushBlend::Normal,
            "multiply" => BrushBlend::Multiply,
            "screen" => BrushBlend::Screen,
            "add" => BrushBlend::Add,
            "darken" => BrushBlend::Darken,
            "lighten" => BrushBlend::Lighten,
            _ => return false,
        };
        self.state.session.tools.brush_style.blend = blend;
        self.active_brush_preset = None;
        self.state.mark_dirty();
        true
    }

    /// Aplica um preset (embutido ou do usuário): ferramenta, números e estilo.
    pub fn apply_brush_preset(&mut self, index: usize) -> bool {
        let all = self.all_brush_presets();
        let Some(preset) = all.get(index) else {
            return false;
        };
        let settings = preset.settings.sanitized();
        self.apply(UiIntent::SetActiveTool(
            tool_id_for_brush(settings.kind).to_string(),
        ));
        self.apply(UiIntent::SetBrushSize(settings.size_px / BRUSH_PX_PER_UNIT));
        let tools = &mut self.state.session.tools;
        tools.brush_hardness = settings.hardness;
        tools.paint_strength = settings.strength;
        tools.brush_flow = settings.flow;
        tools.brush_spacing = settings.spacing;
        tools.brush_style = preset.style.sanitized();
        self.active_brush_preset = Some(index);
        let name = preset.name.clone();
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_BRUSH),
            &[("name", name.to_string())],
        ));
        self.state.mark_dirty();
        true
    }

    /// Salva o pincel atual como preset do usuário (substitui o de mesmo nome).
    pub fn save_brush_preset(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || BrushPreset::builtin().iter().any(|p| p.name == name) {
            return false;
        }
        self.ensure_brush_presets_loaded();
        let preset = BrushPreset {
            name: name.to_string(),
            settings: self.viewport_brush_settings(),
            style: self.state.session.tools.brush_style,
        };
        match self.brush_presets.iter_mut().find(|p| p.name == name) {
            Some(existing) => *existing = preset,
            None => self.brush_presets.push(preset),
        }
        let path = self.brush_presets_path();
        if let Err(error) = BrushPreset::save_user_to(&path, &self.brush_presets) {
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_BRUSH_PRESETS),
                &[("error", format!("{error}"))],
            ));
            return false;
        }
        let builtin = BrushPreset::builtin().len();
        self.active_brush_preset = self
            .brush_presets
            .iter()
            .position(|p| p.name == name)
            .map(|i| i + builtin);
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_BRUSH_SAVED),
            &[("name", name.to_string())],
        ));
        self.state.mark_dirty();
        true
    }

    /// Apaga um preset do usuário (os embutidos não saem).
    pub fn delete_brush_preset(&mut self, index: usize) -> bool {
        self.ensure_brush_presets_loaded();
        let builtin = BrushPreset::builtin().len();
        let Some(user_index) = index.checked_sub(builtin) else {
            return false;
        };
        if user_index >= self.brush_presets.len() {
            return false;
        }
        self.brush_presets.remove(user_index);
        let path = self.brush_presets_path();
        if let Err(error) = BrushPreset::save_user_to(&path, &self.brush_presets) {
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_BRUSH_PRESETS),
                &[("error", format!("{error}"))],
            ));
            return false;
        }
        self.active_brush_preset = None;
        self.state.mark_dirty();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlaceholderViewport;
    use petunia_core::AppState;

    fn bridge(dir: &std::path::Path) -> SlintUiBridge<PlaceholderViewport> {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.brush_presets_path_override = Some(dir.join("brush-presets.json"));
        bridge
    }

    #[test]
    fn params_are_clamped_and_unknown_names_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = bridge(dir.path());
        assert!(b.set_brush_param("smoothing", 5.0));
        assert_eq!(b.state.session.tools.brush_style.smoothing, 0.95);
        assert!(b.set_brush_param("flow", -1.0));
        assert_eq!(b.state.session.tools.brush_flow, 0.0);
        assert!(!b.set_brush_param("nope", 1.0));
        assert!(!b.set_brush_param("flow", f32::NAN));
        assert!(b.set_brush_tip("diamond"));
        assert!(!b.set_brush_tip("star"));
        assert!(b.set_brush_blend("multiply"));
        assert_eq!(b.brush_panel_model().blend, "multiply");
    }

    #[test]
    fn applying_a_preset_switches_tool_and_style() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = bridge(dir.path());
        let spray = BrushPreset::builtin()
            .iter()
            .position(|p| p.name == "Spray can")
            .unwrap();
        assert!(b.apply_brush_preset(spray));
        assert_eq!(b.state.session.tools.active_tool, "spray");
        assert!((b.state.session.tools.brush_style.spray_density - 0.3).abs() < 1e-6);
        assert_eq!(b.brush_panel_model().active_preset, spray as i32);
        let ink = BrushPreset::builtin()
            .iter()
            .position(|p| p.name == "Ink")
            .unwrap();
        assert!(b.apply_brush_preset(ink));
        assert!((b.state.session.tools.brush_style.smoothing - 0.5).abs() < 1e-6);
    }

    #[test]
    fn user_presets_round_trip_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut b = bridge(dir.path());
        b.set_brush_param("scatter", 0.8);
        assert!(b.save_brush_preset("Minha textura"));
        assert!(
            !b.save_brush_preset("Pencil"),
            "nome de preset embutido é reservado"
        );

        let mut again = bridge(dir.path());
        let all = again.all_brush_presets();
        let mine = all.iter().find(|p| p.name == "Minha textura").unwrap();
        assert!((mine.style.scatter - 0.8).abs() < 1e-6);

        let index = all.iter().position(|p| p.name == "Minha textura").unwrap();
        assert!(!again.delete_brush_preset(0), "embutidos não saem");
        assert!(again.delete_brush_preset(index));
        assert!(BrushPreset::load_user_from(&dir.path().join("brush-presets.json")).is_empty());
    }
}
