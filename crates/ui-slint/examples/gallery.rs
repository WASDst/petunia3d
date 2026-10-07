//! Galeria dos componentes base do design system v2.
//!
//! `cargo run -p petunia_ui_slint --example gallery` (idioma: `PETUNIA_LANG`,
//! padrão `en`; ex.: `PETUNIA_LANG=pt-BR`).

use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let gallery = petunia_ui_slint::ComponentGallery::new()?;
    let lang = std::env::var("PETUNIA_LANG").unwrap_or_else(|_| "en".to_string());
    petunia_ui_slint::tr::install_gallery(&gallery, &lang);
    gallery.run()
}
