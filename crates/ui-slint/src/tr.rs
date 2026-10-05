//! Tradução por chave para o shell Slint (`Tr.t("chave")`).
//!
//! O catálogo é o mesmo dos `TextId` (`assets/locales/*.toml`). O Slint é
//! single-threaded; o idioma ativo vive em um `thread_local` e o global `Tr`
//! recebe `revision` novo a cada troca para reavaliar os bindings.

use std::cell::RefCell;

use petunia_config::I18n;
use slint::ComponentHandle;

use crate::{ComponentGallery, PetuniaSlintShell, Tr};

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

/// Liga `Tr.lookup` na galeria de componentes. Cada janela tem a própria
/// instância do global, então a ligação é sempre refeita (sem o atalho
/// idempotente de [`install`]).
pub fn install_gallery(window: &ComponentGallery, lang: &str) {
    CATALOG.with(|c| {
        let mut slot = c.borrow_mut();
        if slot.as_ref().is_none_or(|current| current.lang != lang) {
            *slot = Some(I18n::load(lang));
        }
    });
    let tr = window.global::<Tr>();
    tr.on_lookup(|key| lookup(key.as_str()).into());
    tr.set_revision(tr.get_revision().wrapping_add(1).max(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::{Path, PathBuf};

    const EN_TOML: &str = include_str!("../../../assets/locales/en.toml");
    const PT_TOML: &str = include_str!("../../../assets/locales/pt-BR.toml");

    /// Todos os `.slint` sob `ui/` (a UI pode ser dividida em vários arquivos).
    fn slint_sources(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(dir).expect("ui/ legível");
        for entry in entries {
            let path = entry.expect("entrada de ui/").path();
            if path.is_dir() {
                slint_sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "slint") {
                out.push(path);
            }
        }
    }

    /// Toda chave usada em `Tr.t("...")` existe, não vazia, em en e pt-BR.
    #[test]
    fn every_slint_translation_key_exists_in_both_locales() {
        let en = I18n::parse(EN_TOML);
        let pt = I18n::parse(PT_TOML);
        let mut files = Vec::new();
        slint_sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
            &mut files,
        );
        assert!(
            files.iter().any(|f| f.ends_with("app.slint")),
            "app.slint deve ser varrido"
        );
        let mut keys = Vec::new();
        for file in &files {
            let source = std::fs::read_to_string(file).expect("slint legível");
            let mut rest = source.as_str();
            while let Some(pos) = rest.find("Tr.t(\"") {
                rest = &rest[pos + 6..];
                if let Some(end) = rest.find('"') {
                    keys.push(rest[..end].to_string());
                }
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
    fn gallery_receives_translations() {
        let Ok(gallery) = ComponentGallery::new() else {
            // Ambiente headless sem backend de janela.
            return;
        };
        install_gallery(&gallery, "pt-BR");
        let tr = gallery.global::<Tr>();
        assert_eq!(tr.invoke_t("gallery.solid".into()), "Sólido");
    }

    #[test]
    fn lookup_falls_back_to_key_without_catalog() {
        assert_eq!(lookup("sl.__missing__"), "sl.__missing__");
    }
}
