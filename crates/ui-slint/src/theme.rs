//! Integração de temas e tokens semânticos com o ThemeRegistry canônico do Petunia3D.
//!
//! Atualiza as propriedades dinâmicas do singleton `DesignTokens` em tempo de execução.

use petunia_config::{ThemeRegistry, ThemeToken};
use slint::ComponentHandle;

use crate::{DesignTokens, PetuniaSlintShell};

/// Aplica o tema pelo seu identificador (ex.: "petunia-dark", "petunia-high-contrast")
/// às propriedades do global `DesignTokens` na janela Slint.
pub fn apply_theme(window: &PetuniaSlintShell, theme_id: &str) {
    let registry = ThemeRegistry::global();
    let theme = registry.get_theme(theme_id).cloned().unwrap_or_default();
    let tokens = window.global::<DesignTokens>();

    let to_slint = |token: ThemeToken| -> slint::Color {
        let c = theme.colors.get_token_color(token);
        slint::Color::from_argb_u8(c.0[3], c.0[0], c.0[1], c.0[2])
    };

    tokens.set_canvas(to_slint(ThemeToken::BgCanvas));
    tokens.set_surface(to_slint(ThemeToken::BgSurface));
    tokens.set_surface_raised(to_slint(ThemeToken::BgSurfaceActive));
    tokens.set_surface_hover(to_slint(ThemeToken::BgSurfaceHover));
    tokens.set_border(to_slint(ThemeToken::BorderSubtle));
    tokens.set_border_strong(to_slint(ThemeToken::BorderStrong));
    tokens.set_text_primary(to_slint(ThemeToken::TextPrimary));
    tokens.set_text_secondary(to_slint(ThemeToken::TextSecondary));
    tokens.set_text_muted(to_slint(ThemeToken::TextMuted));
    tokens.set_accent(to_slint(ThemeToken::AccentBlue));
    tokens.set_focus_ring(to_slint(ThemeToken::BorderFocus));
    tokens.set_success(to_slint(ThemeToken::StatusSuccess));
    tokens.set_warning(to_slint(ThemeToken::StatusWarning));
    tokens.set_danger(to_slint(ThemeToken::StatusError));
    tokens.set_selection(to_slint(ThemeToken::AccentOrange));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_registry_has_canonical_themes() {
        let registry = ThemeRegistry::global();
        assert!(registry.get_theme("petunia-dark").is_some());
        assert!(registry.get_theme("petunia-high-contrast").is_some());
    }

    #[test]
    fn apply_theme_resolves_tokens_without_panic() {
        let Ok(window) = PetuniaSlintShell::new() else {
            // Ambiente de teste headless sem GPU/display
            return;
        };
        apply_theme(&window, "petunia-dark");
        apply_theme(&window, "petunia-high-contrast");
    }
}
