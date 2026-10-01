//! Keybinds configuráveis (TOML) — spec §17.
//!
//! ```toml
//! [global]
//! save_project = "Ctrl+S"
//! undo = "Ctrl+Z"
//! [model]
//! extrude = "E"
//! ```
//! Ações desconhecidas no TOML são ignoradas (não quebra).

use std::collections::HashMap;
use std::fs;

use self::winit_keys::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods2 {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Binding {
    pub key: KeyCode,
    pub mods: Mods2,
}

/// Nomes de teclas físicas (layout-independente).
fn parse_key(name: &str) -> Option<KeyCode> {
    let n = name.trim();
    if n.len() == 1 {
        let c = n.chars().next()?.to_ascii_uppercase();
        return match c {
            'A'..='Z' => Some(match c {
                'A' => KeyCode::KeyA,
                'B' => KeyCode::KeyB,
                'C' => KeyCode::KeyC,
                'D' => KeyCode::KeyD,
                'E' => KeyCode::KeyE,
                'F' => KeyCode::KeyF,
                'G' => KeyCode::KeyG,
                'H' => KeyCode::KeyH,
                'I' => KeyCode::KeyI,
                'J' => KeyCode::KeyJ,
                'K' => KeyCode::KeyK,
                'L' => KeyCode::KeyL,
                'M' => KeyCode::KeyM,
                'N' => KeyCode::KeyN,
                'O' => KeyCode::KeyO,
                'P' => KeyCode::KeyP,
                'Q' => KeyCode::KeyQ,
                'R' => KeyCode::KeyR,
                'S' => KeyCode::KeyS,
                'T' => KeyCode::KeyT,
                'U' => KeyCode::KeyU,
                'V' => KeyCode::KeyV,
                'W' => KeyCode::KeyW,
                'X' => KeyCode::KeyX,
                'Y' => KeyCode::KeyY,
                _ => KeyCode::KeyZ,
            }),
            '/' => Some(KeyCode::Slash),
            '.' => Some(KeyCode::Period),
            ',' => Some(KeyCode::Comma),
            '-' => Some(KeyCode::Minus),
            '=' => Some(KeyCode::Equal),
            '`' => Some(KeyCode::Backquote),
            ';' => Some(KeyCode::Semicolon),
            '\'' => Some(KeyCode::Quote),
            '\\' => Some(KeyCode::Backslash),
            '0'..='9' => Some(match c {
                '0' => KeyCode::Digit0,
                '1' => KeyCode::Digit1,
                '2' => KeyCode::Digit2,
                '3' => KeyCode::Digit3,
                '4' => KeyCode::Digit4,
                '5' => KeyCode::Digit5,
                '6' => KeyCode::Digit6,
                '7' => KeyCode::Digit7,
                '8' => KeyCode::Digit8,
                _ => KeyCode::Digit9,
            }),
            _ => None,
        };
    }
    Some(match n {
        "Up" | "ArrowUp" => KeyCode::Up,
        "Down" | "ArrowDown" => KeyCode::Down,
        "Left" | "ArrowLeft" => KeyCode::Left,
        "Right" | "ArrowRight" => KeyCode::Right,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Insert" => KeyCode::Insert,
        "Slash" => KeyCode::Slash,
        "Period" => KeyCode::Period,
        "Comma" => KeyCode::Comma,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        "Backquote" => KeyCode::Backquote,
        "Semicolon" => KeyCode::Semicolon,
        "Quote" => KeyCode::Quote,
        "Backslash" => KeyCode::Backslash,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "Tab" => KeyCode::Tab,
        "Space" => KeyCode::Space,
        "Delete" | "Del" => KeyCode::Delete,
        "Backspace" => KeyCode::Backspace,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Escape" | "Esc" => KeyCode::Escape,
        "Enter" => KeyCode::Enter,
        "BracketLeft" | "[" => KeyCode::BracketLeft,
        "BracketRight" | "]" => KeyCode::BracketRight,
        _ => return None,
    })
}

/// `"Ctrl+Shift+Z"` -> Binding.
pub fn parse_binding(s: &str) -> Option<Binding> {
    let mut mods = Mods2::default();
    let mut key = None;
    for part in s.split('+') {
        match part.trim() {
            "Ctrl" | "Control" => mods.ctrl = true,
            "Shift" => mods.shift = true,
            "Alt" => mods.alt = true,
            other => key = parse_key(other),
        }
    }
    key.map(|key| Binding { key, mods })
}

impl Binding {
    pub fn to_shortcut_string(&self) -> String {
        let mut parts = Vec::new();
        if self.mods.ctrl {
            parts.push("Ctrl");
        }
        if self.mods.shift {
            parts.push("Shift");
        }
        if self.mods.alt {
            parts.push("Alt");
        }
        parts.push(self.key.as_str());
        parts.join("+")
    }
}

/// Mapa ação -> binding, ex. `"model.extrude"`.
#[derive(Debug, Default, Clone)]
pub struct Keybinds {
    map: HashMap<String, Binding>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeymapProfileInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// `true` para perfis criados pelo usuário (editáveis, salvos em disco).
    #[serde(default)]
    pub custom: bool,
}

/// Perfis canônicos embutidos no binário (`assets/keymaps`, P3D-090): o app não
/// depende do diretório de trabalho para achar seus atalhos.
const EMBEDDED_PROFILES: [(&str, &str); 8] = [
    (
        "petunia-default",
        include_str!("../../../assets/keymaps/petunia-default.toml"),
    ),
    (
        "petunia-simple",
        include_str!("../../../assets/keymaps/petunia-simple.toml"),
    ),
    (
        "petunia-notebook",
        include_str!("../../../assets/keymaps/petunia-notebook.toml"),
    ),
    (
        "blender",
        include_str!("../../../assets/keymaps/blender.toml"),
    ),
    (
        "blender-notebook",
        include_str!("../../../assets/keymaps/blender-notebook.toml"),
    ),
    ("maya", include_str!("../../../assets/keymaps/maya.toml")),
    (
        "3ds-max",
        include_str!("../../../assets/keymaps/3ds-max.toml"),
    ),
    (
        "cinema-4d",
        include_str!("../../../assets/keymaps/cinema-4d.toml"),
    ),
];

/// Prefixo dos ids de perfis do usuário (evita colidir com os canônicos).
pub const USER_PROFILE_PREFIX: &str = "user-";

/// Id de perfil seguro para nome de arquivo: minúsculas, dígitos e `-`.
pub fn sanitize_profile_id(name: &str) -> String {
    let mut id = String::new();
    let mut last_dash = true;
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            id.push('-');
            last_dash = true;
        }
    }
    let id = id.trim_matches('-').to_string();
    let id = if id.is_empty() {
        "keymap".to_string()
    } else {
        id
    };
    if id.starts_with(USER_PROFILE_PREFIX) {
        id
    } else {
        format!("{USER_PROFILE_PREFIX}{id}")
    }
}

impl Keybinds {
    /// Retorna a lista de todos os 8 perfis canônicos disponíveis.
    pub fn available_profiles() -> Vec<KeymapProfileInfo> {
        let canonical = |id: &str, name: &str, description: &str| KeymapProfileInfo {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            custom: false,
        };
        vec![
            canonical(
                "petunia-default",
                "Petunia Padrão",
                "Atalhos canônicos com acesso direto a ferramentas e navegação ágil",
            ),
            canonical(
                "petunia-simple",
                "Petunia Simples",
                "Atalhos minimalistas focados em modelagem rápida sem combinações complexas",
            ),
            canonical(
                "petunia-notebook",
                "Petunia Notebook",
                "Otimizado para laptops sem teclado numérico dedicado",
            ),
            canonical(
                "blender",
                "Blender (Oficial)",
                "Mapeamento 100% fiel ao padrão do Blender (G/R/S, E, I, Ctrl+B, Shift+A, Tab)",
            ),
            canonical(
                "blender-notebook",
                "Blender Notebook",
                "Padrão Blender adaptado para laptops sem teclado numérico",
            ),
            canonical(
                "maya",
                "Autodesk Maya",
                "Padrão Maya (Q/W/E/R para Seleção, Mover, Rotacionar e Escalar, F para Frame)",
            ),
            canonical(
                "3ds-max",
                "Autodesk 3ds Max",
                "Padrão 3ds Max (Q/W/E/R, Z para Zoom Extents, 1/2/4 para sub-objetos)",
            ),
            canonical(
                "cinema-4d",
                "Maxon Cinema 4D",
                "Padrão Cinema 4D (E para Mover, R para Rotacionar, T para Escalar)",
            ),
        ]
    }

    /// Diretório dos perfis do usuário (`<config>/petunia3d/keymaps`).
    pub fn user_profiles_dir() -> std::path::PathBuf {
        crate::preferences::UserPreferences::config_dir().join("keymaps")
    }

    /// Canônicos + perfis do usuário em `dir`, nessa ordem (usuário por nome).
    pub fn all_profiles_in(dir: &std::path::Path) -> Vec<KeymapProfileInfo> {
        let mut all = Self::available_profiles();
        let mut custom = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else {
                    continue;
                };
                if all.iter().any(|p| p.id == id) {
                    continue;
                }
                let (name, description) = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| toml::from_str::<toml::Value>(&text).ok())
                    .and_then(|v| {
                        let profile = v.get("profile")?.as_table()?.clone();
                        Some((
                            profile.get("name")?.as_str()?.to_string(),
                            profile
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string(),
                        ))
                    })
                    .unwrap_or_else(|| (id.clone(), String::new()));
                custom.push(KeymapProfileInfo {
                    id,
                    name,
                    description,
                    custom: true,
                });
            }
        }
        custom.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        all.extend(custom);
        all
    }

    /// Todos os perfis do usuário atual.
    pub fn all_profiles() -> Vec<KeymapProfileInfo> {
        Self::all_profiles_in(&Self::user_profiles_dir())
    }

    /// `true` se `profile_id` é um perfil do usuário (editável).
    pub fn is_custom_profile(profile_id: &str) -> bool {
        !EMBEDDED_PROFILES.iter().any(|(id, _)| *id == profile_id)
    }

    fn profile_source(profile_id: &str, user_dir: &std::path::Path) -> Option<String> {
        if Self::is_custom_profile(profile_id)
            && let Ok(text) = fs::read_to_string(user_dir.join(format!("{profile_id}.toml")))
        {
            return Some(text);
        }
        if let Some((_, text)) = EMBEDDED_PROFILES.iter().find(|(id, _)| *id == profile_id) {
            return Some((*text).to_string());
        }
        None
    }

    fn apply_profile_toml(&mut self, text: &str) {
        let Ok(v) = toml::from_str::<toml::Value>(text) else {
            return;
        };
        let Some(table) = v.as_table() else {
            return;
        };
        for (section, inner) in table {
            if section == "profile" {
                continue;
            }
            if let Some(m) = inner.as_table() {
                for (action, val) in m {
                    if let Some(s) = val.as_str()
                        && let Some(b) = parse_binding(s)
                    {
                        self.map.insert(format!("{section}.{action}"), b);
                    }
                }
            }
        }
    }

    /// Carrega um perfil pelo ID: embutido (canônico) ou do usuário em `user_dir`.
    ///
    /// Sempre parte dos defaults Petunia, então ações novas ganham atalho mesmo
    /// em perfis antigos. Perfil desconhecido = só os defaults.
    pub fn load_profile_in(profile_id: &str, user_dir: &std::path::Path) -> Self {
        let mut kb = Self::defaults();
        if let Some(text) = Self::profile_source(profile_id, user_dir) {
            kb.apply_profile_toml(&text);
        }
        kb
    }

    /// Carrega um perfil específico pelo ID (canônico ou do usuário).
    pub fn load_profile(profile_id: &str) -> Self {
        Self::load_profile_in(profile_id, &Self::user_profiles_dir())
    }

    /// Salva `self` como perfil do usuário. O id é derivado do nome; devolve-o.
    pub fn save_user_profile_in(
        &self,
        user_dir: &std::path::Path,
        name: &str,
        description: &str,
    ) -> std::io::Result<String> {
        let id = sanitize_profile_id(name);
        fs::create_dir_all(user_dir)?;
        let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let mut out = format!(
            "[profile]\nid = \"{id}\"\nname = \"{}\"\ndescription = \"{}\"\n\n",
            escape(name.trim()),
            escape(description.trim())
        );
        out.push_str(&self.export_to_toml());
        let tmp = user_dir.join(format!("{id}.toml.tmp"));
        fs::write(&tmp, out)?;
        fs::rename(&tmp, user_dir.join(format!("{id}.toml")))?;
        Ok(id)
    }

    /// Salva no diretório do usuário atual.
    pub fn save_user_profile(&self, name: &str, description: &str) -> std::io::Result<String> {
        self.save_user_profile_in(&Self::user_profiles_dir(), name, description)
    }

    /// Apaga um perfil do usuário (os canônicos não podem ser apagados).
    pub fn delete_user_profile_in(user_dir: &std::path::Path, profile_id: &str) -> bool {
        Self::is_custom_profile(profile_id)
            && fs::remove_file(user_dir.join(format!("{profile_id}.toml"))).is_ok()
    }

    /// Ações (além de `action`) que já usam `binding` num contexto que colide
    /// com o dela: mesmo contexto, ou global/view/window contra qualquer um.
    pub fn conflicts_for(&self, action: &str, binding: &Binding) -> Vec<String> {
        let context = |a: &str| a.split('.').next().unwrap_or("global").to_string();
        let shared = |c: &str| matches!(c, "global" | "view" | "window");
        let mine = context(action);
        let mut out: Vec<String> = self
            .map
            .iter()
            .filter(|(other, b)| {
                other.as_str() != action && *b == binding && {
                    let theirs = context(other);
                    theirs == mine || shared(&theirs) || shared(&mine)
                }
            })
            .map(|(other, _)| other.clone())
            .collect();
        out.sort();
        out
    }

    /// Ações com atalho, ordenadas (base da tela de edição).
    pub fn bound_actions(&self) -> Vec<String> {
        let mut v: Vec<String> = self.map.keys().cloned().collect();
        v.sort();
        v
    }

    /// Binding de uma ação, se houver.
    pub fn binding_for(&self, action: &str) -> Option<&Binding> {
        self.map.get(action)
    }

    pub fn load() -> Self {
        Self::load_profile("petunia-default")
    }

    /// Procura ação pelo (key, mods). Retorna ex. `"model.extrude"`.
    pub fn find(&self, key: KeyCode, mods: Mods2) -> Option<&str> {
        self.map
            .iter()
            .find(|(_, b)| b.key == key && b.mods == mods)
            .map(|(a, _)| a.as_str())
    }

    /// Resolve within the active workspace before global/view/window actions.
    /// Unordered map iteration must not route a Model shortcut to Paint.
    pub fn find_in_context(&self, key: KeyCode, mods: Mods2, context: &str) -> Option<&str> {
        self.map
            .iter()
            .filter(|(action, binding)| {
                binding.key == key
                    && binding.mods == mods
                    && (action.starts_with(&format!("{context}."))
                        || action.starts_with("global.")
                        || action.starts_with("view.")
                        || action.starts_with("window."))
            })
            .min_by(|(a, _), (b, _)| {
                (!a.starts_with(&format!("{context}.")), a.as_str())
                    .cmp(&(!b.starts_with(&format!("{context}.")), b.as_str()))
            })
            .map(|(action, _)| action.as_str())
    }

    /// Retorna a representação textual do atalho para uma ação (ex: `"model.extrude"` -> `"E"`).
    pub fn shortcut_for(&self, action: &str) -> Option<String> {
        self.map.get(action).map(|b| b.to_shortcut_string())
    }

    /// Alias ergonômico para shortcut_for.
    pub fn format_shortcut(&self, action: &str) -> Option<String> {
        self.shortcut_for(action)
    }

    /// Retorna a lista de todas as ações e seus atalhos formatados como string, ordenados.
    pub fn all_bindings(&self) -> Vec<(String, String)> {
        let mut list: Vec<(String, String)> = self
            .map
            .iter()
            .map(|(action, binding)| (action.clone(), binding.to_shortcut_string()))
            .collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list
    }

    /// Adiciona ou atualiza um atalho para uma ação.
    pub fn set_binding(&mut self, action: impl Into<String>, binding: Binding) {
        self.map.insert(action.into(), binding);
    }

    /// Remove um atalho de uma ação.
    pub fn remove_binding(&mut self, action: &str) -> Option<Binding> {
        self.map.remove(action)
    }

    /// Exporta o mapa de atalhos para formato TOML legível agrupado por seções.
    pub fn export_to_toml(&self) -> String {
        use std::collections::BTreeMap;
        let mut sections: BTreeMap<&str, BTreeMap<&str, String>> = BTreeMap::new();

        for (action, binding) in &self.map {
            let mut parts = action.splitn(2, '.');
            let section = parts.next().unwrap_or("global");
            let subaction = parts.next().unwrap_or(action);
            sections
                .entry(section)
                .or_default()
                .insert(subaction, binding.to_shortcut_string());
        }

        let mut out = String::new();
        for (sec, entries) in sections {
            out.push_str(&format!("[{sec}]\n"));
            for (act, key) in entries {
                out.push_str(&format!("{act} = \"{key}\"\n"));
            }
            out.push('\n');
        }
        out
    }

    /// Detecta conflitos de atalho com discernimento de contexto (P3D-091).
    /// Conflitos exatos e sobreposições de escopo global são apontados,
    /// enquanto contextos disjuntos (ex: model vs paint) não colidem.
    pub fn detect_conflicts(&self) -> Vec<KeyConflict> {
        let mut conflicts = Vec::new();
        let items: Vec<(&String, &Binding)> = self.map.iter().collect();

        // 1. Teclas protegidas do sistema
        for (act, bind) in &self.map {
            if bind.key == KeyCode::Escape && !bind.mods.ctrl && !bind.mods.alt {
                conflicts.push(KeyConflict {
                    action_a: act.clone(),
                    action_b: "system.escape".to_string(),
                    shortcut: bind.to_shortcut_string(),
                    kind: ConflictKind::Reserved,
                });
            }
        }

        // 2. Colisões entre ações
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                let (act_a, bind_a) = items[i];
                let (act_b, bind_b) = items[j];
                if bind_a == bind_b {
                    let ctx_a = act_a.split('.').next().unwrap_or("global");
                    let ctx_b = act_b.split('.').next().unwrap_or("global");

                    if ctx_a == ctx_b {
                        conflicts.push(KeyConflict {
                            action_a: act_a.clone(),
                            action_b: act_b.clone(),
                            shortcut: bind_a.to_shortcut_string(),
                            kind: ConflictKind::Exact,
                        });
                    } else if ctx_a == "global" || ctx_b == "global" {
                        conflicts.push(KeyConflict {
                            action_a: act_a.clone(),
                            action_b: act_b.clone(),
                            shortcut: bind_a.to_shortcut_string(),
                            kind: ConflictKind::ContextOverlap,
                        });
                    }
                    // Contextos disjuntos (ex: "model" e "paint") não colidem!
                }
            }
        }
        conflicts
    }

    /// Defaults "Petunia" (iguais aos atalhos documentados na UI).
    pub fn defaults() -> Self {
        let pairs = [
            ("global.undo", "Ctrl+Z"),
            ("global.redo", "Ctrl+Shift+Z"),
            ("global.save_project", "Ctrl+S"),
            ("global.command_palette", "Ctrl+P"),
            ("global.toggle_wireframe", "Z"),
            ("global.help", "H"),
            ("global.reset_camera", "Home"),
            ("global.toggle_projection", "O"),
            ("global.cycle_mode", "Tab"),
            ("global.rename", "F2"),
            ("view.frame_all", "Home"),
            ("view.frame_selection", "F"),
            ("view.reset_camera", "Shift+Home"),
            ("view.toggle_projection", "O"),
            ("view.toggle_wireframe", "Z"),
            ("view.toggle_xray", "Alt+Z"),
            ("window.command_palette", "Ctrl+P"),
            ("window.reference_manager", "Shift+R"),
            ("model.select_vertex", "1"),
            ("model.select_face", "3"),
            ("model.select_edge", "2"),
            ("model.select_object", "4"),
            ("uv.toggle_seam", "U"),
            ("model.transform", "T"),
            ("model.move", "G"),
            ("model.box_select", "B"),
            ("model.tool_select", "W"),
            ("model.rotate", "R"),
            ("model.scale", "S"),
            ("model.make_face", "F"),
            ("model.extrude", "E"),
            ("model.extrude_individual", "Alt+E"),
            ("model.push_pull", "P"),
            ("model.inset", "I"),
            ("model.bevel", "Ctrl+B"),
            ("model.subdivide", "Alt+W"),
            ("model.merge", "M"),
            ("model.primitives", "A"),
            ("model.draw_profile", "Shift+P"),
            ("model.slice", "Shift+K"),
            ("model.knife", "K"),
            ("model.loop_cut", "Ctrl+R"),
            ("model.connect", "Ctrl+J"),
            ("model.dissolve", "X"),
            ("model.invert_selection", "Ctrl+I"),
            ("model.select_linked", "L"),
            ("model.delete", "Delete"),
            ("model.duplicate", "Shift+D"),
            ("global.previous_tool", "Space"),
            ("global.micro_inspector", "Shift+Space"),
            ("window.workspace_draw", "Ctrl+1"),
            ("window.workspace_poly", "Ctrl+2"),
            ("window.workspace_paint", "Ctrl+3"),
            ("window.workspace_uv", "Ctrl+4"),
            ("window.workspace_animate", "Ctrl+5"),
            ("paint.select", "V"),
            ("paint.isolate", "/"),
            ("paint.paint", "B"),
            ("paint.airbrush", "Shift+B"),
            ("paint.eraser", "E"),
            ("paint.color_picker", "I"),
            ("paint.fill", "G"),
            ("paint.gradient", "Shift+G"),
            ("paint.line", "L"),
            ("paint.rectangle", "R"),
            ("paint.size_decrease", "["),
            ("paint.size_increase", "]"),
            ("paint.hardness_decrease", "Shift+["),
            ("paint.hardness_increase", "Shift+]"),
        ];
        let mut kb = Self::default();
        for (a, s) in pairs {
            if let Some(b) = parse_binding(s) {
                kb.map.insert(a.to_string(), b);
            }
        }
        kb
    }
}

/// Tipos de conflito de atalhos detectados no sistema (P3D-091).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConflictKind {
    /// Mesmo contexto de trabalho usando o mesmo atalho.
    Exact,
    /// Atalho global sobrepõe atalho de modo contextual específico.
    ContextOverlap,
    /// Uso de tecla protegida de sistema (ex: Escape sem modificadores).
    Reserved,
}

impl ConflictKind {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Exact => "Conflito Exato (mesmo contexto)",
            Self::ContextOverlap => "Sobreposição Global (shadowing)",
            Self::Reserved => "Tecla Reservada do Sistema",
        }
    }
}

/// Detalhes de um conflito detectado entre duas ações ou com o sistema.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyConflict {
    pub action_a: String,
    pub action_b: String,
    pub shortcut: String,
    pub kind: ConflictKind,
}

// Os tipos winit são convertidos em `app`; aqui ficam tipos próprios para
// o crate `config` não depender de `winit`.
pub mod winit_keys {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[allow(missing_docs)]
    pub enum KeyCode {
        KeyA,
        KeyB,
        KeyC,
        KeyD,
        KeyE,
        KeyF,
        KeyG,
        KeyH,
        KeyI,
        KeyJ,
        KeyK,
        KeyL,
        KeyM,
        KeyN,
        KeyO,
        KeyP,
        KeyQ,
        KeyR,
        KeyS,
        KeyT,
        KeyU,
        KeyV,
        KeyW,
        KeyX,
        KeyY,
        KeyZ,
        Digit0,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        Tab,
        Space,
        Delete,
        Backspace,
        Home,
        End,
        Escape,
        Enter,
        BracketLeft,
        BracketRight,
        Up,
        Down,
        Left,
        Right,
        PageUp,
        PageDown,
        Insert,
        Slash,
        Period,
        Comma,
        Minus,
        Equal,
        Backquote,
        Semicolon,
        Quote,
        Backslash,
    }

    impl KeyCode {
        pub fn as_str(&self) -> &'static str {
            match self {
                KeyCode::KeyA => "A",
                KeyCode::KeyB => "B",
                KeyCode::KeyC => "C",
                KeyCode::KeyD => "D",
                KeyCode::KeyE => "E",
                KeyCode::KeyF => "F",
                KeyCode::KeyG => "G",
                KeyCode::KeyH => "H",
                KeyCode::KeyI => "I",
                KeyCode::KeyJ => "J",
                KeyCode::KeyK => "K",
                KeyCode::KeyL => "L",
                KeyCode::KeyM => "M",
                KeyCode::KeyN => "N",
                KeyCode::KeyO => "O",
                KeyCode::KeyP => "P",
                KeyCode::KeyQ => "Q",
                KeyCode::KeyR => "R",
                KeyCode::KeyS => "S",
                KeyCode::KeyT => "T",
                KeyCode::KeyU => "U",
                KeyCode::KeyV => "V",
                KeyCode::KeyW => "W",
                KeyCode::KeyX => "X",
                KeyCode::KeyY => "Y",
                KeyCode::KeyZ => "Z",
                KeyCode::Digit0 => "0",
                KeyCode::Digit1 => "1",
                KeyCode::Digit2 => "2",
                KeyCode::Digit3 => "3",
                KeyCode::Digit4 => "4",
                KeyCode::Digit5 => "5",
                KeyCode::Digit6 => "6",
                KeyCode::Digit7 => "7",
                KeyCode::Digit8 => "8",
                KeyCode::Digit9 => "9",
                KeyCode::F1 => "F1",
                KeyCode::F2 => "F2",
                KeyCode::F3 => "F3",
                KeyCode::F4 => "F4",
                KeyCode::F5 => "F5",
                KeyCode::F6 => "F6",
                KeyCode::F7 => "F7",
                KeyCode::F8 => "F8",
                KeyCode::F9 => "F9",
                KeyCode::F10 => "F10",
                KeyCode::F11 => "F11",
                KeyCode::F12 => "F12",
                KeyCode::Tab => "Tab",
                KeyCode::Space => "Space",
                KeyCode::Delete => "Delete",
                KeyCode::Backspace => "Backspace",
                KeyCode::Home => "Home",
                KeyCode::End => "End",
                KeyCode::Escape => "Escape",
                KeyCode::Enter => "Enter",
                KeyCode::BracketLeft => "[",
                KeyCode::BracketRight => "]",
                KeyCode::Up => "Up",
                KeyCode::Down => "Down",
                KeyCode::Left => "Left",
                KeyCode::Right => "Right",
                KeyCode::PageUp => "PageUp",
                KeyCode::PageDown => "PageDown",
                KeyCode::Insert => "Insert",
                KeyCode::Slash => "/",
                KeyCode::Period => ".",
                KeyCode::Comma => ",",
                KeyCode::Minus => "-",
                KeyCode::Equal => "=",
                KeyCode::Backquote => "`",
                KeyCode::Semicolon => ";",
                KeyCode::Quote => "'",
                KeyCode::Backslash => "\\",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    #[allow(missing_docs)]
    pub struct Mods {
        pub ctrl: bool,
        pub shift: bool,
        pub alt: bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_8_keymap_profiles_load_validly() {
        let profiles = Keybinds::available_profiles();
        assert_eq!(profiles.len(), 8);

        for p in profiles {
            let kb = Keybinds::load_profile(&p.id);
            let bindings = kb.all_bindings();
            assert!(
                !bindings.is_empty(),
                "Profile {} must contain bindings",
                p.id
            );
        }
    }

    #[test]
    fn test_conflict_detection_logic() {
        let mut kb = Keybinds::default();
        // Exact conflict within same context
        kb.set_binding("model.extrude", parse_binding("E").unwrap());
        kb.set_binding("model.push_pull", parse_binding("E").unwrap());

        // Disjoint contexts: model vs paint (not a conflict!)
        kb.set_binding("paint.brush", parse_binding("B").unwrap());
        kb.set_binding("model.connect", parse_binding("B").unwrap());

        // Context overlap: global shadows model
        kb.set_binding("global.save_project", parse_binding("Ctrl+S").unwrap());
        kb.set_binding("model.scale_special", parse_binding("Ctrl+S").unwrap());

        // Reserved key: Escape without modifiers
        kb.set_binding("model.cancel_op", parse_binding("Escape").unwrap());

        let conflicts = kb.detect_conflicts();
        assert_eq!(conflicts.len(), 3); // 1 exact, 1 overlap, 1 reserved

        let exact = conflicts
            .iter()
            .find(|c| c.kind == ConflictKind::Exact)
            .unwrap();
        assert_eq!(exact.shortcut, "E");

        let overlap = conflicts
            .iter()
            .find(|c| c.kind == ConflictKind::ContextOverlap)
            .unwrap();
        assert_eq!(overlap.shortcut, "Ctrl+S");

        let reserved = conflicts
            .iter()
            .find(|c| c.kind == ConflictKind::Reserved)
            .unwrap();
        assert_eq!(reserved.shortcut, "Escape");
    }

    #[test]
    fn test_rebinding_and_toml_export() {
        let mut kb = Keybinds::default();
        kb.set_binding("model.extrude", parse_binding("Shift+E").unwrap());
        assert_eq!(
            kb.shortcut_for("model.extrude"),
            Some("Shift+E".to_string())
        );

        let toml_str = kb.export_to_toml();
        assert!(toml_str.contains("[model]"));
        assert!(toml_str.contains("extrude = \"Shift+E\""));

        kb.remove_binding("model.extrude");
        assert_eq!(kb.shortcut_for("model.extrude"), None);
    }
}
