//! Host utilities for the editor session.

use crate::selection_color_has_contrast;
use petunia_core::AppState;

pub struct DesktopHost;

impl DesktopHost {
    pub fn apply_preferences(state: &mut AppState, preferences: &petunia_config::UserPreferences) {
        if let Some(language) = &preferences.language {
            state.ui.i18n.set_lang(language);
        }
        state.ui.invert_vertical_drag = preferences.invert_vertical_drag;
        state.ui.colorblind_axes = preferences.colorblind_axes;
        state.ui.reduced_motion = preferences.reduced_motion;
        state.ui.multiselection_measure_tag = preferences.multiselection_measure_tag;
        state.ui.selection_rgb = if selection_color_has_contrast(preferences.selection_rgb) {
            preferences.selection_rgb
        } else {
            petunia_config::UserPreferences::default().selection_rgb
        };
        state.ui.selection_thickness = preferences.selection_thickness.clamp(1.0, 6.0);
        state.ui.model_quick_actions = preferences.model_quick_actions.clone();
        if !preferences.theme_id.is_empty() {
            state.ui.active_theme_id = preferences.theme_id.clone();
        }
        if !preferences.active_keymap_id.is_empty() {
            state.ui.active_keymap_id = preferences.active_keymap_id.clone();
            state.ui.keybinds =
                petunia_config::Keybinds::load_profile(&preferences.active_keymap_id);
        }
    }
}
