//! Integração de temas e tokens semânticos com o ThemeRegistry canônico do Petunia3D.

#[cfg(test)]
mod tests {
    use petunia_config::ThemeRegistry;

    #[test]
    fn theme_registry_has_canonical_themes() {
        let registry = ThemeRegistry::global();
        assert!(registry.get_theme("petunia-dark").is_some());
        assert!(registry.get_theme("petunia-high-contrast").is_some());
        assert!(registry.get_theme("petunia-light").is_some());
    }
}
