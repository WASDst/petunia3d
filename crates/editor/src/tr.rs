//! Tradução por chave para o editor (`Tr.t("chave")`).

use petunia_config::I18n;
use std::cell::RefCell;

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

/// Preenche os marcadores `{nome}` de uma mensagem traduzida.
pub fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut text = template.to_string();
    for (key, value) in args {
        text = text.replace(&format!("{{{key}}}"), value);
    }
    text
}

/// Define o catálogo ativo de tradução.
pub fn set_locale(lang: &str) {
    CATALOG.with(|c| {
        *c.borrow_mut() = Some(I18n::load(lang));
    });
}
