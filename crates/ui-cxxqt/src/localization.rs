//! Reuse the canonical locale catalog for native Qt labels; extra native chrome
//! strings belong to this adapter, not to the document or modeling engine.
use serde_json::{Map, Value, json};
pub(crate) fn pack(locale: &petunia_config::I18n) -> Value {
    let english = petunia_config::I18n::embedded_only("en");
    let portuguese = petunia_config::I18n::embedded_only("pt-BR");
    let mut map = Map::new();
    for key in english.keys() {
        let value = locale.t(&key);
        map.insert(english.t(&key), json!(&value));
        map.insert(portuguese.t(&key), json!(value));
    }
    let native: Value = serde_json::from_str(include_str!("../assets/native-translations.json"))
        .expect("embedded native locale catalog");
    for (source, translations) in native.as_object().expect("native locale object") {
        if let Some(value) = translations
            .get(locale.lang())
            .or_else(|| translations.get("en"))
        {
            map.insert(source.clone(), value.clone());
        }
    }
    Value::Object(map)
}
