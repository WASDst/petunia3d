//! Tema externo (TOML) e registro centralizado de temas do Petunia3D.
//! Suporta tokens semânticos (`ThemeToken`), os temas oficiais embutidos de V1 e
//! temas personalizados criados por usuários em subpastas contendo
//! `manifest.toml` e `theme.toml`.
//!
//! **Conjunto oficial de V1 (capítulo 36):** `petunia-dark` é o tema completo
//! oficial e `petunia-high-contrast` é a variação oficial de acessibilidade.
//! Qualquer outro tema é declaração externa — pack em `themes/<id>/` do diretório
//! do usuário ou `.petunia-theme` — nunca código embutido (P3D-085).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

/// Cor RGBA pura (0-255).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColorRgba(pub [u8; 4]);

impl ColorRgba {
    pub const WHITE: Self = Self([255, 255, 255, 255]);
    pub const BLACK: Self = Self([0, 0, 0, 255]);
    pub const TRANSPARENT: Self = Self([0, 0, 0, 0]);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    pub const fn to_rgba_u8(&self) -> [u8; 4] {
        self.0
    }

    pub fn to_rgba_f32(&self) -> [f32; 4] {
        [
            self.0[0] as f32 / 255.0,
            self.0[1] as f32 / 255.0,
            self.0[2] as f32 / 255.0,
            self.0[3] as f32 / 255.0,
        ]
    }
}

impl From<[u8; 4]> for ColorRgba {
    fn from(arr: [u8; 4]) -> Self {
        Self(arr)
    }
}

impl From<ColorRgba> for [u8; 4] {
    fn from(c: ColorRgba) -> Self {
        c.0
    }
}

/// Tokens canônicos de cores do sistema de design do Petunia3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeToken {
    BgCanvas,
    BgHeader,
    BgPanel,
    BgPanelHeader,
    BgSurface,
    BgSurfaceHover,
    BgSurfaceActive,
    TextPrimary,
    TextSecondary,
    TextMuted,
    TextActive,
    AccentBlue,
    AccentOrange,
    AccentHover,
    AccentBorder,
    BorderSubtle,
    BorderStrong,
    BorderFocus,
    StatusInfo,
    StatusWarning,
    StatusError,
    StatusSuccess,
}

const ALL_THEME_TOKENS: [ThemeToken; 22] = [
    ThemeToken::BgCanvas,
    ThemeToken::BgHeader,
    ThemeToken::BgPanel,
    ThemeToken::BgPanelHeader,
    ThemeToken::BgSurface,
    ThemeToken::BgSurfaceHover,
    ThemeToken::BgSurfaceActive,
    ThemeToken::TextPrimary,
    ThemeToken::TextSecondary,
    ThemeToken::TextMuted,
    ThemeToken::TextActive,
    ThemeToken::AccentBlue,
    ThemeToken::AccentOrange,
    ThemeToken::AccentHover,
    ThemeToken::AccentBorder,
    ThemeToken::BorderSubtle,
    ThemeToken::BorderStrong,
    ThemeToken::BorderFocus,
    ThemeToken::StatusInfo,
    ThemeToken::StatusWarning,
    ThemeToken::StatusError,
    ThemeToken::StatusSuccess,
];

/// Metadados de manifesto do tema (`manifest.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeFile {
    pub theme: ThemeManifest,
}

/// Tema completo do Petunia3D.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Theme {
    #[serde(skip)]
    pub manifest: Option<ThemeManifest>,
    pub colors: ThemeColors,
    pub shell: ShellColors,
    pub font: ThemeFont,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub bg_canvas: String,
    pub bg_header: String,
    pub bg_panel: String,
    pub bg_panel_header: String,
    pub bg_surface: String,
    pub bg_surface_hover: String,
    pub bg_surface_active: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    pub text_active: String,
    pub accent_blue: String,
    pub accent_orange: String,
    pub accent_hover: String,
    pub accent_border: String,
    pub border_subtle: String,
    pub border_strong: String,
    pub border_focus: String,
    pub status_info: String,
    pub status_warning: String,
    pub status_error: String,
    pub status_success: String,
    /// Cache puramente derivado. Temas são tratados como valores imutáveis após
    /// carregamento; assim os hexadecimais são parseados no máximo uma vez.
    #[serde(skip)]
    resolved: OnceLock<HashMap<ThemeToken, ColorRgba>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeFont {
    pub family: String,
    pub size: f32,
}

impl Default for ThemeFont {
    fn default() -> Self {
        Self {
            family: "proportional".into(),
            size: 14.0,
        }
    }
}

impl Default for ThemeColors {
    /// Paleta do `petunia-dark` (baseline cap. 36). Os nomes `accent_blue` e
    /// `accent_orange` são históricos: hoje carregam o accent floral (violeta)
    /// e a cor de seleção (laranja), respectivamente.
    fn default() -> Self {
        Self {
            bg_canvas: "#101114".into(),
            bg_header: "#17181c".into(),
            bg_panel: "#1d1f23".into(),
            bg_panel_header: "#22252a".into(),
            bg_surface: "#25282e".into(),
            bg_surface_hover: "#30343b".into(),
            bg_surface_active: "#3a3f48".into(),
            text_primary: "#edf0f4".into(),
            text_secondary: "#aeb5c0".into(),
            text_muted: "#8a919e".into(),
            text_active: "#101114".into(),
            accent_blue: "#b58cff".into(),
            accent_orange: "#e96a00".into(),
            accent_hover: "#c9a8ff".into(),
            accent_border: "#9b6df0".into(),
            border_subtle: "#2e3238".into(),
            border_strong: "#454b56".into(),
            border_focus: "#d1b8ff".into(),
            status_info: "#6cb6ff".into(),
            status_warning: "#e5bd67".into(),
            status_error: "#ef6b73".into(),
            status_success: "#73d59b".into(),
            resolved: OnceLock::new(),
        }
    }
}

/// Tokens de shell que não fazem parte dos 22 `ThemeToken` legados e são
/// consumidos apenas pela UI Slint (`DesignTokens`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellToken {
    SelectionActive,
    Disabled,
    HoverHighlight,
    Scrim,
    Shadow,
    ShadowSubtle,
    HudSurface,
    HudBorder,
}

/// Cores de shell por tema. Valores em hex `#rrggbb` ou `#rrggbbaa`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellColors {
    pub selection_active: String,
    pub disabled: String,
    pub hover_highlight: String,
    pub scrim: String,
    pub shadow: String,
    pub shadow_subtle: String,
    pub hud_surface: String,
    pub hud_border: String,
}

impl Default for ShellColors {
    fn default() -> Self {
        Self {
            selection_active: "#ffaf29".into(),
            disabled: "#565d68".into(),
            hover_highlight: "#7ddcff".into(),
            scrim: "#00000099".into(),
            shadow: "#00000088".into(),
            shadow_subtle: "#00000055".into(),
            hud_surface: "#14141ee6".into(),
            hud_border: "#3a3f48".into(),
        }
    }
}

impl ShellColors {
    fn hex_for(&self, token: ShellToken) -> (&str, &str) {
        let d = |t: ShellToken| match t {
            ShellToken::SelectionActive => "#ffaf29",
            ShellToken::Disabled => "#565d68",
            ShellToken::HoverHighlight => "#7ddcff",
            ShellToken::Scrim => "#00000099",
            ShellToken::Shadow => "#00000088",
            ShellToken::ShadowSubtle => "#00000055",
            ShellToken::HudSurface => "#14141ee6",
            ShellToken::HudBorder => "#3a3f48",
        };
        let v = match token {
            ShellToken::SelectionActive => &self.selection_active,
            ShellToken::Disabled => &self.disabled,
            ShellToken::HoverHighlight => &self.hover_highlight,
            ShellToken::Scrim => &self.scrim,
            ShellToken::Shadow => &self.shadow,
            ShellToken::ShadowSubtle => &self.shadow_subtle,
            ShellToken::HudSurface => &self.hud_surface,
            ShellToken::HudBorder => &self.hud_border,
        };
        (v.as_str(), d(token))
    }

    /// Resolve o token; valor inválido cai no default escuro.
    pub fn get(&self, token: ShellToken) -> ColorRgba {
        let (v, fallback) = self.hex_for(token);
        Theme::hex(v)
            .or_else(|| Theme::hex(fallback))
            .unwrap_or(ColorRgba::WHITE)
    }
}

impl ThemeColors {
    fn hex_for_token(&self, token: ThemeToken) -> &str {
        match token {
            ThemeToken::BgCanvas => &self.bg_canvas,
            ThemeToken::BgHeader => &self.bg_header,
            ThemeToken::BgPanel => &self.bg_panel,
            ThemeToken::BgPanelHeader => &self.bg_panel_header,
            ThemeToken::BgSurface => &self.bg_surface,
            ThemeToken::BgSurfaceHover => &self.bg_surface_hover,
            ThemeToken::BgSurfaceActive => &self.bg_surface_active,
            ThemeToken::TextPrimary => &self.text_primary,
            ThemeToken::TextSecondary => &self.text_secondary,
            ThemeToken::TextMuted => &self.text_muted,
            ThemeToken::TextActive => &self.text_active,
            ThemeToken::AccentBlue => &self.accent_blue,
            ThemeToken::AccentOrange => &self.accent_orange,
            ThemeToken::AccentHover => &self.accent_hover,
            ThemeToken::AccentBorder => &self.accent_border,
            ThemeToken::BorderSubtle => &self.border_subtle,
            ThemeToken::BorderStrong => &self.border_strong,
            ThemeToken::BorderFocus => &self.border_focus,
            ThemeToken::StatusInfo => &self.status_info,
            ThemeToken::StatusWarning => &self.status_warning,
            ThemeToken::StatusError => &self.status_error,
            ThemeToken::StatusSuccess => &self.status_success,
        }
    }

    fn resolve_uncached(&self, token: ThemeToken) -> ColorRgba {
        Theme::hex(self.hex_for_token(token)).unwrap_or_else(|| {
            let defaults = ThemeColors::default();
            Theme::hex(defaults.hex_for_token(token)).unwrap_or(ColorRgba::WHITE)
        })
    }

    /// Converte um token canônico na cor correspondente deste tema em formato RGBA puro.
    /// O mapa inteiro é resolvido apenas na primeira consulta do tema.
    pub fn get_token_color_rgba(&self, token: ThemeToken) -> ColorRgba {
        let resolved = self.resolved.get_or_init(|| {
            ALL_THEME_TOKENS
                .into_iter()
                .map(|theme_token| (theme_token, self.resolve_uncached(theme_token)))
                .collect()
        });
        resolved.get(&token).copied().unwrap_or(ColorRgba::WHITE)
    }

    /// Alias conveniente para obter a cor do token em ColorRgba.
    pub fn get_token_color(&self, token: ThemeToken) -> ColorRgba {
        self.get_token_color_rgba(token)
    }
}

impl Theme {
    /// Carrega o tema pelo identificador. Se não encontrar, tenta carregar o tema Petunia Dark.
    pub fn load_by_id(id: &str) -> Self {
        let registry = ThemeRegistry::global();
        registry.get_theme(id).cloned().unwrap_or_default()
    }

    /// Carrega o tema padrão do sistema (`petunia-dark`).
    pub fn load() -> Self {
        Self::load_by_id("petunia-dark")
    }

    pub fn hex(s: &str) -> Option<ColorRgba> {
        let s = s.trim().trim_start_matches('#');
        if s.len() == 6 && s.is_ascii() {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(ColorRgba::rgb(r, g, b))
        } else if s.len() == 8 && s.is_ascii() {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(ColorRgba::new(r, g, b, a))
        } else {
            None
        }
    }
}

/// Registro central de descoberta e carregamento de temas instalados.
pub struct ThemeRegistry {
    themes: HashMap<String, Theme>,
    manifests: Vec<ThemeManifest>,
}

/// Snapshot global imutável. Leituras no hot path fazem somente clone de `Arc`;
/// varredura de disco/TOML ocorre no primeiro acesso ou em reload explícito.
static GLOBAL_THEME_REGISTRY: LazyLock<RwLock<Arc<ThemeRegistry>>> =
    LazyLock::new(|| RwLock::new(Arc::new(ThemeRegistry::load_all())));

impl ThemeRegistry {
    fn load_all() -> Self {
        let mut registry = Self {
            themes: HashMap::new(),
            manifests: Vec::new(),
        };
        registry.scan_and_load();
        registry
    }

    /// Obtém o snapshot global já carregado, sem I/O nem parsing por chamada.
    pub fn global() -> Arc<Self> {
        match GLOBAL_THEME_REGISTRY.read() {
            Ok(registry) => Arc::clone(&registry),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// Reescaneia temas fora do hot path e troca o snapshot de forma atômica
    /// para leitores subsequentes. Usado pelo file watcher/hot reload.
    pub fn reload_global() {
        let reloaded = Arc::new(Self::load_all());
        match GLOBAL_THEME_REGISTRY.write() {
            Ok(mut registry) => *registry = reloaded,
            Err(poisoned) => *poisoned.into_inner() = reloaded,
        }
    }

    /// Retorna a lista de manifestos de todos os temas válidos encontrados.
    pub fn available(&self) -> &[ThemeManifest] {
        &self.manifests
    }

    /// Busca um tema carregado pelo ID.
    pub fn get_theme(&self, id: &str) -> Option<&Theme> {
        self.themes
            .get(id)
            .or_else(|| self.themes.get("petunia-dark"))
    }

    /// Escaneia pastas de temas em busca de `manifest.toml` e `theme.toml`.
    pub fn scan_and_load(&mut self) {
        // Permite reload idempotente sem duplicar manifestos built-in.
        self.themes.clear();
        self.manifests.clear();

        // 1. Carrega os 4 temas built-in garantidos em memória
        self.register_builtin_themes();

        // 2. Escaneia diretórios do disco (assets/themes e diretório local)
        let candidate_dirs = ["assets/themes", "themes"];
        for dir_name in candidate_dirs {
            let path = PathBuf::from(dir_name);
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let subpath = entry.path();
                    if subpath.is_dir() {
                        self.try_load_theme_folder(&subpath);
                    }
                }
            }
        }
    }

    fn try_load_theme_folder(&mut self, folder: &Path) {
        let manifest_path = folder.join("manifest.toml");
        let theme_path = folder.join("theme.toml");

        if !manifest_path.exists() || !theme_path.exists() {
            return;
        }

        // Tenta ler manifest
        let manifest_text = match fs::read_to_string(&manifest_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "[Petunia3D ThemeRegistry] Aviso: Falha ao ler '{:?}': {e}",
                    manifest_path
                );
                return;
            }
        };

        let manifest: ThemeManifest = match toml::from_str::<ThemeFile>(&manifest_text) {
            Ok(tf) => tf.theme,
            Err(_) => match toml::from_str::<ThemeManifest>(&manifest_text) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "[Petunia3D ThemeRegistry] Aviso: manifest.toml inválido em '{:?}': {e}. Usando fallback Petunia Dark.",
                        folder
                    );
                    return;
                }
            },
        };

        // Tenta ler theme.toml
        let theme_text = match fs::read_to_string(&theme_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "[Petunia3D ThemeRegistry] Aviso: Falha ao ler '{:?}': {e}",
                    theme_path
                );
                return;
            }
        };

        let mut theme: Theme = match toml::from_str(&theme_text) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "[Petunia3D ThemeRegistry] Aviso: theme.toml inválido em '{:?}': {e}. Usando fallback Petunia Dark.",
                    folder
                );
                return;
            }
        };

        theme.manifest = Some(manifest.clone());

        // Atualiza ou insere
        if let Some(existing) = self.manifests.iter_mut().find(|m| m.id == manifest.id) {
            *existing = manifest.clone();
        } else {
            self.manifests.push(manifest.clone());
        }
        self.themes.insert(manifest.id, theme);
    }

    /// IDs oficiais de V1 (capítulo 36). Nenhum outro tema é embutido: temas
    /// adicionais são declarações externas carregadas do diretório do usuário.
    pub const OFFICIAL_V1_THEME_IDS: [&'static str; 3] =
        ["petunia-dark", "petunia-light", "petunia-high-contrast"];

    fn register_builtin_themes(&mut self) {
        // Petunia Dark — tema completo oficial da V1 (default).
        let dark_manifest = ThemeManifest {
            id: "petunia-dark".into(),
            name: "Petunia Dark".into(),
            version: "1.0.0".into(),
            author: Some("Petunia3D Team".into()),
            description: Some("Tema escuro canônico padrão".into()),
        };
        let dark_theme = Theme {
            manifest: Some(dark_manifest.clone()),
            ..Default::default()
        };
        self.manifests.push(dark_manifest.clone());
        self.themes.insert("petunia-dark".into(), dark_theme);

        // Petunia Light — tema claro oficial (revisão cap. 36, 2026-09-30):
        // neutros quentes, sem branco puro, para conforto visual prolongado.
        let light_manifest = ThemeManifest {
            id: "petunia-light".into(),
            name: "Petunia Light".into(),
            version: "1.0.0".into(),
            author: Some("Petunia3D Team".into()),
            description: Some("Tema claro suave, neutros quentes sem branco puro".into()),
        };
        let light_colors = ThemeColors {
            bg_canvas: "#dcdad3".into(),
            bg_header: "#e9e7e1".into(),
            bg_panel: "#efede8".into(),
            bg_panel_header: "#e6e4de".into(),
            bg_surface: "#f6f5f1".into(),
            bg_surface_hover: "#e3e1da".into(),
            bg_surface_active: "#d6d3ca".into(),
            text_primary: "#24262b".into(),
            text_secondary: "#4c515b".into(),
            text_muted: "#626772".into(),
            text_active: "#ffffff".into(),
            accent_blue: "#7c4dd6".into(),
            accent_orange: "#d45a00".into(),
            accent_hover: "#6a3cc4".into(),
            accent_border: "#5b2fb0".into(),
            border_subtle: "#d2cfc6".into(),
            border_strong: "#b3afa4".into(),
            border_focus: "#5b2fb0".into(),
            status_info: "#1f6fb2".into(),
            status_warning: "#8a5f00".into(),
            status_error: "#c93b45".into(),
            status_success: "#2f8a55".into(),
            ..Default::default()
        };
        let light_shell = ShellColors {
            selection_active: "#c98a00".into(),
            disabled: "#a9a69c".into(),
            hover_highlight: "#0a8fb5".into(),
            scrim: "#2b271f66".into(),
            shadow: "#3a2f1f33".into(),
            shadow_subtle: "#3a2f1f1f".into(),
            hud_surface: "#f5f3eeee".into(),
            hud_border: "#cfccc2".into(),
        };
        let light_theme = Theme {
            manifest: Some(light_manifest.clone()),
            colors: light_colors,
            shell: light_shell,
            font: ThemeFont::default(),
        };
        self.manifests.push(light_manifest.clone());
        self.themes.insert("petunia-light".into(), light_theme);

        // Petunia High Contrast — variação oficial de acessibilidade da V1.
        let hc_manifest = ThemeManifest {
            id: "petunia-high-contrast".into(),
            name: "Petunia High Contrast".into(),
            version: "1.0.0".into(),
            author: Some("Petunia3D Team".into()),
            description: Some("Variação oficial de acessibilidade com contraste máximo".into()),
        };
        let hc_colors = ThemeColors {
            bg_canvas: "#000000".into(),
            bg_header: "#0a0a0a".into(),
            bg_panel: "#121212".into(),
            bg_panel_header: "#1a1a1a".into(),
            bg_surface: "#1f1f1f".into(),
            bg_surface_hover: "#2e2e2e".into(),
            bg_surface_active: "#3d3d3d".into(),
            text_primary: "#ffffff".into(),
            text_secondary: "#e6e6e6".into(),
            text_muted: "#b8b8b8".into(),
            text_active: "#000000".into(),
            accent_blue: "#00b0ff".into(),
            accent_orange: "#ffb000".into(),
            accent_hover: "#4dd0ff".into(),
            accent_border: "#ffffff".into(),
            border_subtle: "#6b6b6b".into(),
            border_strong: "#a0a0a0".into(),
            border_focus: "#ffd400".into(),
            status_info: "#57c7ff".into(),
            status_warning: "#ffd400".into(),
            status_error: "#ff6b6b".into(),
            status_success: "#6ee7a8".into(),
            ..Default::default()
        };

        let hc_theme = Theme {
            manifest: Some(hc_manifest.clone()),
            colors: hc_colors,
            shell: ShellColors {
                selection_active: "#ffd400".into(),
                disabled: "#8a8a8a".into(),
                hud_surface: "#000000f2".into(),
                hud_border: "#a0a0a0".into(),
                ..Default::default()
            },
            font: ThemeFont::default(),
        };
        self.manifests.push(hc_manifest.clone());
        self.themes.insert("petunia-high-contrast".into(), hc_theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_v1_themes_are_registered() {
        let registry = ThemeRegistry::global();
        let _ = registry.available();

        for id in ThemeRegistry::OFFICIAL_V1_THEME_IDS {
            let theme = registry.get_theme(id);
            assert!(theme.is_some(), "Theme {id} must be registered");
            let theme = theme.unwrap();
            assert_eq!(theme.manifest.as_ref().unwrap().id, id);
        }
    }

    #[test]
    fn test_high_contrast_is_a_discoverable_official_theme() {
        let registry = ThemeRegistry::global();
        assert!(
            registry
                .available()
                .iter()
                .any(|m| m.id == "petunia-high-contrast"),
            "High Contrast must be listed as an official accessibility variation"
        );
    }

    #[test]
    fn test_unknown_theme_id_falls_back_to_petunia_dark() {
        let theme = Theme::load_by_id("tema-que-nao-existe");
        assert_eq!(theme.manifest.as_ref().unwrap().id, "petunia-dark");
    }

    #[test]
    fn test_theme_registry_global_reuses_snapshot() {
        let first = ThemeRegistry::global();
        let second = ThemeRegistry::global();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn test_theme_tokens_resolve_valid_colors() {
        let registry = ThemeRegistry::global();
        for manifest in registry.available() {
            let theme = registry.get_theme(&manifest.id).unwrap();
            for token in [
                ThemeToken::BgCanvas,
                ThemeToken::BgHeader,
                ThemeToken::BgPanel,
                ThemeToken::BgSurface,
                ThemeToken::TextPrimary,
                ThemeToken::AccentBlue,
                ThemeToken::BorderSubtle,
            ] {
                let c = theme.colors.get_token_color(token);
                assert_ne!(c, ColorRgba::TRANSPARENT);
            }
            assert!(theme.colors.resolved.get().is_some());
        }
    }

    fn relative_luminance(c: ColorRgba) -> f32 {
        let [r, g, b, _] = c.to_rgba_f32();
        let to_linear = |v: f32| -> f32 {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
    }

    fn contrast_ratio(c1: ColorRgba, c2: ColorRgba) -> f32 {
        let l1 = relative_luminance(c1);
        let l2 = relative_luminance(c2);
        let (bright, dark) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (bright + 0.05) / (dark + 0.05)
    }

    #[test]
    fn test_wcag_22_contrast_ratios() {
        let registry = ThemeRegistry::global();
        for manifest in registry.available() {
            let theme = registry.get_theme(&manifest.id).unwrap();
            let text = theme.colors.get_token_color(ThemeToken::TextPrimary);
            let canvas = theme.colors.get_token_color(ThemeToken::BgCanvas);
            let surface = theme.colors.get_token_color(ThemeToken::BgSurface);
            let panel = theme.colors.get_token_color(ThemeToken::BgPanel);

            let ratio_canvas = contrast_ratio(text, canvas);
            let ratio_surface = contrast_ratio(text, surface);
            let ratio_panel = contrast_ratio(text, panel);

            assert!(
                ratio_canvas >= 4.5,
                "Theme {} TextPrimary vs BgCanvas contrast ratio {:.2} < 4.5 (WCAG AA)",
                manifest.id,
                ratio_canvas
            );
            assert!(
                ratio_surface >= 4.5,
                "Theme {} TextPrimary vs BgSurface contrast ratio {:.2} < 4.5 (WCAG AA)",
                manifest.id,
                ratio_surface
            );
            assert!(
                ratio_panel >= 4.5,
                "Theme {} TextPrimary vs BgPanel contrast ratio {:.2} < 4.5 (WCAG AA)",
                manifest.id,
                ratio_panel
            );
        }
    }

    #[test]
    fn test_light_theme_is_soft_not_pure_white() {
        let theme = ThemeRegistry::global()
            .get_theme("petunia-light")
            .cloned()
            .expect("petunia-light must be registered");
        for token in [
            ThemeToken::BgCanvas,
            ThemeToken::BgHeader,
            ThemeToken::BgPanel,
            ThemeToken::BgSurface,
        ] {
            let c = theme.colors.get_token_color(token);
            assert_ne!(c, ColorRgba::WHITE, "{token:?} must not be pure white");
            assert!(
                relative_luminance(c) < 0.92,
                "{token:?} must stay soft (luminance < 0.92)"
            );
        }
    }

    #[test]
    fn test_text_on_accent_meets_aa_in_every_theme() {
        let registry = ThemeRegistry::global();
        for manifest in registry.available() {
            let theme = registry.get_theme(&manifest.id).unwrap();
            let on = theme.colors.get_token_color(ThemeToken::TextActive);
            let accent = theme.colors.get_token_color(ThemeToken::AccentBlue);
            let ratio = contrast_ratio(on, accent);
            assert!(
                ratio >= 4.5,
                "{} on-accent contrast {ratio:.2} < 4.5",
                manifest.id
            );
        }
    }

    #[test]
    fn test_secondary_and_muted_text_meet_aa_on_panel() {
        let registry = ThemeRegistry::global();
        for manifest in registry.available() {
            let theme = registry.get_theme(&manifest.id).unwrap();
            let panel = theme.colors.get_token_color(ThemeToken::BgPanel);
            for token in [ThemeToken::TextSecondary, ThemeToken::TextMuted] {
                let ratio = contrast_ratio(theme.colors.get_token_color(token), panel);
                assert!(
                    ratio >= 4.5,
                    "{} {token:?} on panel {ratio:.2} < 4.5",
                    manifest.id
                );
            }
        }
    }
}
