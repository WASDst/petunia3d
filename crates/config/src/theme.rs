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
    fn default() -> Self {
        Self {
            bg_canvas: "#17181c".into(),
            bg_header: "#202126".into(),
            bg_panel: "#24252a".into(),
            bg_panel_header: "#292a30".into(),
            bg_surface: "#30323a".into(),
            bg_surface_hover: "#3b3d47".into(),
            bg_surface_active: "#474a56".into(),
            text_primary: "#f0f2f5".into(),
            text_secondary: "#a3a7b5".into(),
            text_muted: "#6e7280".into(),
            text_active: "#ffffff".into(),
            accent_blue: "#3b82f6".into(),
            accent_orange: "#f97316".into(),
            accent_hover: "#60a5fa".into(),
            accent_border: "#2563eb".into(),
            border_subtle: "#32343c".into(),
            border_strong: "#434652".into(),
            border_focus: "#3b82f6".into(),
            status_info: "#38bdf8".into(),
            status_warning: "#fbbf24".into(),
            status_error: "#f87171".into(),
            status_success: "#4ade80".into(),
            resolved: OnceLock::new(),
        }
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
    pub const OFFICIAL_V1_THEME_IDS: [&'static str; 2] = ["petunia-dark", "petunia-high-contrast"];

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
            text_active: "#ffffff".into(),
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
}
