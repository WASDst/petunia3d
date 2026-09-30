//! Tradução por chave para o shell Slint (`Tr.t("chave")`).
//!
//! O catálogo é o mesmo dos `TextId` (`assets/locales/*.toml`). O Slint é
//! single-threaded; o idioma ativo vive em um `thread_local` e o global `Tr`
//! recebe `revision` novo a cada troca para reavaliar os bindings.

use std::cell::RefCell;

use petunia_config::I18n;
use slint::ComponentHandle;

use crate::{PetuniaSlintShell, Tr};

thread_local! {
    static CATALOG: RefCell<Option<I18n>> = const { RefCell::new(None) };
}

/// Traduz `key` no idioma instalado; sem catálogo, devolve a própria chave.
pub fn lookup(key: &str) -> String {
    CATALOG.with(|c| match c.borrow().as_ref() {
        Some(i18n) => i18n.t(key),
        None => key.to_string(),
    })
}

/// Instala `lang` (idempotente) e liga o callback `Tr.lookup` na janela.
pub fn install(window: &PetuniaSlintShell, lang: &str) {
    let changed = CATALOG.with(|c| {
        let mut slot = c.borrow_mut();
        match slot.as_ref() {
            Some(current) if current.lang == lang => false,
            _ => {
                *slot = Some(I18n::load(lang));
                true
            }
        }
    });
    let tr = window.global::<Tr>();
    if changed {
        tr.on_lookup(|key| lookup(key.as_str()).into());
        tr.set_revision(tr.get_revision().wrapping_add(1).max(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP_SLINT: &str = include_str!("../ui/app.slint");
    const EN_TOML: &str = include_str!("../../../assets/locales/en.toml");
    const PT_TOML: &str = include_str!("../../../assets/locales/pt-BR.toml");

    /// Toda chave usada em `Tr.t("...")` existe, não vazia, em en e pt-BR.
    #[test]
    fn every_slint_translation_key_exists_in_both_locales() {
        let en = I18n::parse(EN_TOML);
        let pt = I18n::parse(PT_TOML);
        let mut keys = Vec::new();
        let mut rest = APP_SLINT;
        while let Some(pos) = rest.find("Tr.t(\"") {
            rest = &rest[pos + 6..];
            if let Some(end) = rest.find('"') {
                keys.push(rest[..end].to_string());
            }
        }
        for key in &keys {
            assert!(en.get(key).is_some_and(|v| !v.is_empty()), "en sem {key}");
            assert!(
                pt.get(key).is_some_and(|v| !v.is_empty()),
                "pt-BR sem {key}"
            );
        }
    }

    #[test]
    fn lookup_falls_back_to_key_without_catalog() {
        assert_eq!(lookup("sl.__missing__"), "sl.__missing__");
    }
}
