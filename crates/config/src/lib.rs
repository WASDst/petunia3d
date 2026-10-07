//! Petunia3D — configuração externa (TOML): idioma/i18n, keybinds,
//! ferramentas habilitadas e tema. Nada disso é hardcoded nas features.

pub mod i18n;
pub mod keybinds;
pub mod preferences;
pub mod theme;
pub mod tools;

pub use i18n::{I18n, TextId, text_id};
pub use keybinds::{Binding, ConflictKind, KeyConflict, Keybinds, KeymapProfileInfo};
pub use preferences::{InspectorSectionId, RECENT_PROJECTS_LIMIT, SectionLayout, UserPreferences};
pub use theme::{
    ColorRgba, ShellColors, ShellToken, Theme, ThemeManifest, ThemeRegistry, ThemeToken,
};
pub use tools::load_tools_config;
