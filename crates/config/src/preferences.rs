//! Preferências do usuário independentes do documento e do toolkit.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Inspector section with independent dock/float/pin state.
/// Seção do Inspector com estado independente de dock/flutuação/pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InspectorSectionId {
    Parts,
    Transform,
    Material,
    Object,
    Modifiers,
    QuickActions,
}

impl InspectorSectionId {
    /// All sections in canonical Inspector order.
    /// Todas as seções na ordem canônica do Inspector.
    pub const fn all() -> [Self; 6] {
        [
            Self::Parts,
            Self::Transform,
            Self::Material,
            Self::Object,
            Self::Modifiers,
            Self::QuickActions,
        ]
    }

    /// Stable persistence key. Never rename: stored on disk.
    /// Chave estável de persistência. Nunca renomear: gravada em disco.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Parts => "parts",
            Self::Transform => "transform",
            Self::Material => "material",
            Self::Object => "object",
            Self::Modifiers => "modifiers",
            Self::QuickActions => "quick_actions",
        }
    }
}

/// Dock/float/pin state of one Inspector section, persisted per module.
fn default_open() -> bool {
    false
}

/// Estado de dock/flutuação/pin de uma seção do Inspector, persistido por módulo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SectionLayout {
    /// Docked in the Inspector column when true, floating card when false.
    /// Ancorado na coluna do Inspector quando verdadeiro, card flutuante quando falso.
    pub docked: bool,
    /// Floating card position in logical px, clamped to [0, 8192] on load.
    /// Posição do card flutuante em px lógicos, limitada a [0, 8192] ao carregar.
    pub x: f32,
    /// See `x`. / Ver `x`.
    pub y: f32,
    /// Keep the section open: ignores collapse-all and panel collapse.
    /// Mantém a seção aberta: ignora recolher-tudo e recolhimento do painel.
    pub pin_open: bool,
    /// Asset UUID text this section is pinned to; follows selection when `None`.
    /// Stored as text because `config` must not depend on `project`/`uuid`
    /// (dependency direction: domain owns identity, config only persists it).
    /// Texto do UUID do asset ao qual a seção está fixada; segue a seleção quando `None`.
    /// Guardado como texto porque `config` não pode depender de `project`/`uuid`
    /// (direção de dependência: o domínio detém a identidade, config só persiste).
    pub pinned_asset: Option<String>,
    /// Whether the section is expanded/visible (`true`) or collapsed into a pill (`false`).
    /// Se a seção está expandida/visível (`true`) ou recolhida em pílula (`false`).
    #[serde(default = "default_open")]
    pub open: bool,
}

impl Default for SectionLayout {
    fn default() -> Self {
        Self {
            docked: true,
            x: 12.0,
            y: 56.0,
            pin_open: false,
            pinned_asset: None,
            open: false,
        }
    }
}

impl SectionLayout {
    /// Clamp coordinates to finite viewport range and drop oversized ids.
    /// Limita coordenadas à faixa finita da viewport e descarta ids longos demais.
    pub fn sanitized(mut self) -> Self {
        if !self.x.is_finite() {
            self.x = Self::default().x;
        }
        if !self.y.is_finite() {
            self.y = Self::default().y;
        }
        self.x = self.x.clamp(0.0, 8192.0);
        self.y = self.y.clamp(0.0, 8192.0);
        if self
            .pinned_asset
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 64)
        {
            self.pinned_asset = None;
        }
        self
    }
}

fn default_theme_id() -> String {
    "petunia-dark".to_string()
}

/// Limiar padrão de arrasto; igual a `petunia_core::DEFAULT_DRAG_THRESHOLD_PX`.
const fn default_drag_threshold_px() -> f32 {
    4.0
}

/// Igual a `petunia_core::DEFAULT_SNAP_RADIUS_PIXELS`.
const fn default_snap_radius_px() -> f32 {
    12.0
}

const fn default_true() -> bool {
    true
}

fn default_icon_theme() -> String {
    "outline".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserPreferences {
    /// Inverte apenas a resposta vertical das ferramentas de manipulação.
    pub invert_vertical_drag: bool,
    /// Cor do destaque de seleção, RGB sRGB de 8 bits.
    pub selection_rgb: [u8; 3],
    /// Espessura visual em pixels lógicos (1..=6).
    pub selection_thickness: f32,
    /// Escala da interface (1.0 = 100%, até 2.0 = 200%).
    pub ui_scale: f32,
    /// Ações rápidas preferidas do Inspector MODEL.
    pub model_quick_actions: Vec<String>,
    /// Intervalo máximo em milissegundos para duplo toque de tecla de ferramenta entrar em modo modal (0 desativa o temporizador).
    pub double_tap_interval_ms: u64,
    /// Distância, em px lógicos, que separa um clique de um arrasto (2–16).
    pub drag_threshold_px: f32,
    /// Clicar numa alça a faz seguir o ponteiro até o próximo clique, sem
    /// segurar o botão (alternativa a arrastar, WCAG 2.2 — 2.5.7).
    pub click_move_click: bool,
    /// Plano automático do Draw sem face sob o cursor: `true` usa o chão, a
    /// menos que a câmera esteja quase na horizontal; `false` usa o plano do
    /// mundo mais de frente para a vista.
    pub workplane_prefer_ground: bool,
    /// Luz de estúdio acompanha a câmera (Plasticity/C4D); `false` a mantém
    /// fixa no mundo.
    #[serde(default = "default_true")]
    pub studio_light_follows_camera: bool,
    /// Matcap procedural no Solid (aparência de argila que segue a vista).
    #[serde(default)]
    pub viewport_matcap: bool,
    /// Oclusão ambiente (GTAO) no Solid e no Material.
    #[serde(default = "default_true")]
    pub viewport_ambient_occlusion: bool,
    /// Raio do snap em px lógicos (4–48). Maior ajuda quem tem tremor ou pouca
    /// precisão (Findlater et al., 2010: cursores de área).
    #[serde(default = "default_snap_radius_px")]
    pub snap_radius_px: f32,
    /// Diferenciação não-cromática de eixos para acessibilidade e daltonismo.
    pub colorblind_axes: bool,
    /// Redução de movimento para usuários com sensibilidade vestibular / labirintite.
    pub reduced_motion: bool,
    /// Trilho de ferramentas com o nome ao lado do ícone (plano de UI, D6).
    /// Desligado por padrão: o trilho padrão continua estreito.
    pub show_tool_labels: bool,
    /// Cor de destaque da interface (botões ativos, foco). `None` usa a do
    /// tema. Separada da cor de seleção na viewport (plano de UI, D5).
    pub accent_rgb: Option<[u8; 3]>,
    /// Projetos abertos ou salvos por último, do mais recente ao mais antigo
    /// (Home, cap. 23). No máximo `RECENT_PROJECTS_LIMIT`.
    pub recent_projects: Vec<String>,
    /// Mostra a Home ao abrir o programa sem projeto.
    pub show_home_on_start: bool,
    /// Exibe tag flutuante com a média de medidas na multiseleção de arestas.
    pub multiselection_measure_tag: bool,
    /// Perfil ativo de atalhos de teclado ("petunia-default", "blender", "maya", "3ds-max", etc.).
    pub active_keymap_id: String,
    /// Tema de cores ativo (`petunia-dark`, `petunia-light`, `petunia-high-contrast`
    /// ou um tema externo). Id desconhecido cai no dark ao aplicar.
    #[serde(default = "default_theme_id")]
    pub theme_id: String,
    /// Tema dos ícones da interface ("outline" ou "filled").
    /// Icon theme of the user interface ("outline" or "filled").
    #[serde(default = "default_icon_theme")]
    pub icon_theme: String,
    /// Dock/float/pin por módulo do Inspector, chaveado por `InspectorSectionId::as_str`.
    /// Chaves desconhecidas são descartadas ao carregar; módulos ausentes usam o padrão.
    /// Dock/float/pin per Inspector module, keyed by `InspectorSectionId::as_str`.
    /// Unknown keys are dropped on load; missing modules use the default.
    pub section_layouts: BTreeMap<String, SectionLayout>,
}

/// Quantos projetos recentes a Home lista.
pub const RECENT_PROJECTS_LIMIT: usize = 8;

impl UserPreferences {
    /// Põe `path` no topo dos recentes, sem repetir e respeitando o limite.
    pub fn push_recent_project(&mut self, path: &str) {
        self.recent_projects.retain(|existing| existing != path);
        self.recent_projects.insert(0, path.to_string());
        self.recent_projects.truncate(RECENT_PROJECTS_LIMIT);
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            invert_vertical_drag: false,
            selection_rgb: [233, 106, 0],
            selection_thickness: 2.0,
            ui_scale: 1.0,
            model_quick_actions: Vec::new(),
            double_tap_interval_ms: 350,
            drag_threshold_px: default_drag_threshold_px(),
            click_move_click: false,
            workplane_prefer_ground: false,
            studio_light_follows_camera: true,
            viewport_matcap: false,
            viewport_ambient_occlusion: true,
            snap_radius_px: default_snap_radius_px(),
            colorblind_axes: false,
            reduced_motion: false,
            show_tool_labels: false,
            accent_rgb: None,
            recent_projects: Vec::new(),
            show_home_on_start: true,
            multiselection_measure_tag: true,
            active_keymap_id: "petunia-default".to_string(),
            theme_id: default_theme_id(),
            icon_theme: "outline".to_string(),
            section_layouts: BTreeMap::new(),
        }
    }
}

impl UserPreferences {
    /// Diretório de configuração do usuário (`…/petunia3d`).
    pub fn config_dir() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(std::env::temp_dir);
        base.join("petunia3d")
    }

    pub fn default_path() -> PathBuf {
        Self::config_dir().join("preferences.toml")
    }

    pub fn load_from_path(path: &Path) -> io::Result<Self> {
        let source = fs::read_to_string(path)?;
        let mut preferences: Self = toml::from_str(&source)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if !preferences.selection_thickness.is_finite()
            || !(1.0..=6.0).contains(&preferences.selection_thickness)
        {
            preferences.selection_thickness = Self::default().selection_thickness;
        }
        if !preferences.snap_radius_px.is_finite()
            || !(4.0..=48.0).contains(&preferences.snap_radius_px)
        {
            preferences.snap_radius_px = default_snap_radius_px();
        }
        if !preferences.ui_scale.is_finite() || !(1.0..=2.0).contains(&preferences.ui_scale) {
            preferences.ui_scale = 1.0;
        }
        if preferences.double_tap_interval_ms > 2000 {
            preferences.double_tap_interval_ms = 2000;
        }
        if !preferences.drag_threshold_px.is_finite()
            || !(2.0..=16.0).contains(&preferences.drag_threshold_px)
        {
            preferences.drag_threshold_px = Self::default().drag_threshold_px;
        }
        if preferences.active_keymap_id.is_empty() {
            preferences.active_keymap_id = "petunia-default".to_string();
        }
        if preferences.theme_id.trim().is_empty() {
            preferences.theme_id = default_theme_id();
        }
        if !matches!(preferences.icon_theme.as_str(), "outline" | "filled") {
            preferences.icon_theme = "outline".to_string();
        }
        let mut seen = HashSet::new();
        preferences
            .model_quick_actions
            .retain(|id| !id.is_empty() && id.len() <= 64 && seen.insert(id.clone()));
        preferences.model_quick_actions.truncate(6);
        let known: HashSet<&str> = InspectorSectionId::all()
            .iter()
            .map(|id| id.as_str())
            .collect();
        preferences
            .section_layouts
            .retain(|key, _| known.contains(key.as_str()));
        for layout in preferences.section_layouts.values_mut() {
            *layout = std::mem::take(layout).sanitized();
        }
        Ok(preferences)
    }

    /// Layout persistido do módulo, ou o padrão quando ausente.
    /// Persisted module layout, or the default when missing.
    pub fn section_layout(&self, id: InspectorSectionId) -> SectionLayout {
        self.section_layouts
            .get(id.as_str())
            .cloned()
            .unwrap_or_default()
    }

    /// Grava o layout do módulo já sanitizado.
    /// Stores the module layout, sanitized first.
    pub fn set_section_layout(&mut self, id: InspectorSectionId, layout: SectionLayout) {
        self.section_layouts
            .insert(id.as_str().to_string(), layout.sanitized());
    }

    pub fn load() -> Self {
        Self::load_from_path(&Self::default_path()).unwrap_or_default()
    }

    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let source = toml::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let temporary = path.with_extension(format!("toml.{}.tmp", std::process::id()));
        fs::write(&temporary, &source)?;
        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            #[cfg(windows)]
            Err(_) => {
                // Windows não substitui sempre o destino de rename; fallback
                // de melhor esforço para gravações subsequentes.
                let result = fs::write(path, source);
                let _ = fs::remove_file(&temporary);
                result
            }
            #[cfg(not(windows))]
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to_path(&Self::default_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_round_trip_and_missing_field_default() {
        let path =
            std::env::temp_dir().join(format!("petunia-preferences-{}.toml", std::process::id()));
        let preferences = UserPreferences {
            invert_vertical_drag: true,
            selection_rgb: [32, 180, 240],
            selection_thickness: 3.5,
            ui_scale: 1.25,
            model_quick_actions: vec!["model.fuse".to_string()],
            colorblind_axes: false,
            reduced_motion: false,
            show_tool_labels: true,
            accent_rgb: Some([91, 140, 255]),
            recent_projects: vec!["/tmp/a.petunia".to_string()],
            show_home_on_start: false,
            multiselection_measure_tag: true,
            active_keymap_id: "blender".to_string(),
            theme_id: "petunia-light".to_string(),
            icon_theme: "outline".to_string(),
            double_tap_interval_ms: 300,
            drag_threshold_px: 6.0,
            click_move_click: true,
            workplane_prefer_ground: true,
            studio_light_follows_camera: false,
            viewport_matcap: true,
            viewport_ambient_occlusion: false,
            snap_radius_px: 20.0,
            section_layouts: BTreeMap::from([(
                InspectorSectionId::Material.as_str().to_string(),
                SectionLayout {
                    docked: false,
                    x: 100.0,
                    y: 200.0,
                    pin_open: true,
                    pinned_asset: None,
                    open: true,
                },
            )]),
        };
        preferences.save_to_path(&path).unwrap();
        assert_eq!(UserPreferences::load_from_path(&path).unwrap(), preferences);
        let updated = UserPreferences {
            selection_thickness: 5.0,
            ..preferences
        };
        updated.save_to_path(&path).unwrap();
        assert_eq!(UserPreferences::load_from_path(&path).unwrap(), updated);
        fs::write(
            &path,
            "selection_thickness = nan\ninvert_vertical_drag = true\n",
        )
        .unwrap();
        let sanitized = UserPreferences::load_from_path(&path).unwrap();
        assert_eq!(sanitized.selection_thickness, 2.0);
        assert!(sanitized.invert_vertical_drag);
        fs::remove_file(&path).unwrap();
        assert_eq!(
            toml::from_str::<UserPreferences>("").unwrap(),
            UserPreferences::default()
        );
    }

    #[test]
    fn section_layout_round_trip_per_module() {
        let path = std::env::temp_dir().join(format!(
            "petunia-sections-roundtrip-{}.toml",
            std::process::id()
        ));
        let mut preferences = UserPreferences::default();
        assert_eq!(
            preferences.section_layout(InspectorSectionId::Parts),
            SectionLayout::default()
        );
        preferences.set_section_layout(
            InspectorSectionId::Object,
            SectionLayout {
                docked: false,
                x: 300.0,
                y: 120.0,
                pin_open: true,
                pinned_asset: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
                open: true,
            },
        );
        preferences.save_to_path(&path).unwrap();
        let loaded = UserPreferences::load_from_path(&path).unwrap();
        assert_eq!(
            loaded.section_layout(InspectorSectionId::Object),
            preferences.section_layout(InspectorSectionId::Object)
        );
        assert!(!loaded.section_layout(InspectorSectionId::Object).docked);
        assert_eq!(
            loaded.section_layout(InspectorSectionId::Parts),
            SectionLayout::default()
        );
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn section_layout_round_trip_covers_all_six_modules() {
        let path =
            std::env::temp_dir().join(format!("petunia-sections-all-{}.toml", std::process::id()));
        let mut preferences = UserPreferences::default();
        for (index, id) in InspectorSectionId::all().iter().enumerate() {
            preferences.set_section_layout(
                *id,
                SectionLayout {
                    docked: index % 2 == 0,
                    x: 10.0 * index as f32,
                    y: 20.0 * index as f32,
                    pin_open: index % 2 == 1,
                    pinned_asset: None,
                    open: true,
                },
            );
        }
        preferences.save_to_path(&path).unwrap();
        let loaded = UserPreferences::load_from_path(&path).unwrap();
        assert_eq!(loaded.section_layouts.len(), 6);
        assert_eq!(loaded, preferences);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn section_layout_sanitized_holds_for_every_f32_bit_class() {
        // Deterministic sweep, no RNG and no new dependencies: every exponent
        // class (zeros, subnormals, normals, infinities, NaN payloads) crossed
        // with representative mantissas, both signs.
        // Varredura determinística, sem RNG e sem dependências novas: toda classe
        // de expoente (zeros, subnormais, normais, infinitos, payloads NaN) cruzada
        // com mantissas representativas, ambos os sinais.
        let mantissas = [
            0x0u32, 0x1, 0x2, 0x100, 0x200000, 0x400000, 0x555555, 0x7FFFFF, 0xAAAAAA,
        ];
        let mut cases = 0u32;
        for exponent in 0x00u32..=0xFF {
            for mantissa in mantissas {
                for sign in [0x0u32, 0x80000000] {
                    let raw = sign | (exponent << 23) | mantissa;
                    // Skip signaling NaN payloads: they trap on some targets when
                    // materialized, and quiet NaNs already cover the class.
                    // Pula payloads NaN sinalizadores: eles armadilham em alguns alvos
                    // ao materializar, e os NaNs quietos já cobrem a classe.
                    if exponent == 0xFF && mantissa != 0 && mantissa & 0x400000 == 0 {
                        continue;
                    }
                    let value = f32::from_bits(raw);
                    let layout = SectionLayout {
                        x: value,
                        y: -value,
                        ..SectionLayout::default()
                    }
                    .sanitized();
                    assert!(
                        layout.x.is_finite() && (0.0..=8192.0).contains(&layout.x),
                        "x escaped sanitize for bits {raw:#010x}"
                    );
                    assert!(
                        layout.y.is_finite() && (0.0..=8192.0).contains(&layout.y),
                        "y escaped sanitize for bits {raw:#010x}"
                    );
                    cases += 1;
                }
            }
        }
        assert!(cases > 4000, "sweep must cover thousands of patterns");
        // Byte length decides: multibyte content counts against the 64-byte cap.
        // Tamanho em bytes decide: conteúdo multibyte conta no limite de 64 bytes.
        for bytes in [0usize, 1, 36, 64, 65, 100] {
            let layout = SectionLayout {
                pinned_asset: Some("x".repeat(bytes)),
                ..SectionLayout::default()
            }
            .sanitized();
            let kept = layout.pinned_asset.as_ref().map(String::len);
            if bytes == 0 || bytes > 64 {
                assert_eq!(kept, None, "byte-len {bytes} must be dropped");
            } else {
                assert_eq!(kept, Some(bytes), "byte-len {bytes} must be kept");
            }
        }
        let kept = SectionLayout {
            pinned_asset: Some("é".repeat(32)),
            ..SectionLayout::default()
        }
        .sanitized();
        assert_eq!(kept.pinned_asset.as_ref().map(String::len), Some(64));
        let dropped = SectionLayout {
            pinned_asset: Some("é".repeat(33)),
            ..SectionLayout::default()
        }
        .sanitized();
        assert_eq!(dropped.pinned_asset, None);
    }

    #[test]
    fn section_layout_load_sanitizes_coordinates_and_keys() {
        let path = std::env::temp_dir().join(format!(
            "petunia-sections-sanitize-{}.toml",
            std::process::id()
        ));
        fs::write(
            &path,
            "[section_layouts.material]\ndocked = false\nx = nan\ny = -50.0\npin_open = true\npinned_asset = \"\"\n\
             [section_layouts.unknown_module]\ndocked = false\n",
        )
        .unwrap();
        let loaded = UserPreferences::load_from_path(&path).unwrap();
        let material = loaded.section_layout(InspectorSectionId::Material);
        assert!(!material.docked);
        assert_eq!((material.x, material.y), (12.0, 0.0));
        assert!(material.pin_open);
        assert_eq!(material.pinned_asset, None);
        assert!(!loaded.section_layouts.contains_key("unknown_module"));
        fs::remove_file(&path).unwrap();
    }
}
