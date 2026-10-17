//! Host utilities for the editor session.

use crate::selection_color_has_contrast;
use petunia_core::AppState;

#[allow(unused_imports)]
pub use petunia_application::{DesktopHost as AppDesktopHost, SessionLockGuard};

pub struct DesktopHost;

impl DesktopHost {
    pub fn apply_preferences(state: &mut AppState, preferences: &petunia_config::UserPreferences) {
        let mut adjusted_prefs = preferences.clone();
        if !selection_color_has_contrast(adjusted_prefs.selection_rgb) {
            adjusted_prefs.selection_rgb = petunia_config::UserPreferences::default().selection_rgb;
        }
        petunia_application::DesktopHost::apply_preferences(state, &adjusted_prefs);
    }
}
