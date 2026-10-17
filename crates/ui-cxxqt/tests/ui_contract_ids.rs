//! Contratos estáticos entre a UI Qt e o núcleo: a UI só pode referenciar ids de
//! comando que existem no `CommandDispatcher` e deve usar os mesmos ids de modo
//! que o editor publica. Não precisa de display nem de QApplication.
use petunia_core::command::CommandDispatcher;
use std::{collections::BTreeSet, fs, path::PathBuf};

const COMMAND_NAMESPACES: &[&str] = &[
    "model", "uv", "paint", "draw", "edit", "view", "file", "select", "tool", "object", "project",
];

/// Literais que parecem ids, mas pertencem a outra camada (recursos Qt, chaves
/// de configuração ou intents tratados fora do `CommandDispatcher`). Cada
/// entrada exige justificativa; a lista só pode encolher.
const NOT_DISPATCHER_IDS: &[(&str, &str)] = &[
    // Intent local do viewport (`petunia_gl_widget`), não é comando de documento.
    ("view.isolate", "intent de visualização tratado em actionTriggered"),
];

/// Fontes Rust onde um id pode ser tratado (dispatcher, editor, ponte Qt).
fn rust_sources() -> String {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut all = String::new();
    for dir in ["core/src", "editor/src", "ui-cxxqt/src"] {
        let mut stack = vec![crates.join(dir)];
        while let Some(path) = stack.pop() {
            let Ok(read) = fs::read_dir(&path) else { continue };
            for entry in read.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                    all.push_str(&fs::read_to_string(&p).unwrap_or_default());
                }
            }
        }
    }
    all
}

fn cpp_sources() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp");
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).expect("diretório cpp legível") {
        let path = entry.expect("entrada legível").path();
        if matches!(path.extension().and_then(|e| e.to_str()), Some("cpp" | "h")) {
            let text = fs::read_to_string(&path).expect("fonte legível");
            files.push((
                path.file_name().unwrap().to_string_lossy().into_owned(),
                text,
            ));
        }
    }
    files.sort();
    files
}

/// Extrai literais de string C++ simples (`"..."`) de uma linha, ignorando
/// escapes e tudo depois de `//`.
fn string_literals(line: &str) -> Vec<String> {
    let code = line.split("//").next().unwrap_or("");
    let mut out = Vec::new();
    let mut current = String::new();
    let mut inside = false;
    let mut escaped = false;
    for ch in code.chars() {
        if inside {
            if escaped {
                escaped = false;
                current.push(ch);
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                inside = false;
                out.push(std::mem::take(&mut current));
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            inside = true;
        }
    }
    out
}

fn looks_like_command_id(text: &str) -> bool {
    let Some((namespace, name)) = text.split_once('.') else {
        return false;
    };
    COMMAND_NAMESPACES.contains(&namespace)
        && !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[test]
fn every_command_id_used_by_the_qt_ui_is_registered() {
    let dispatcher = CommandDispatcher::canonical();
    // O editor resolve ids por mais de um caminho (dispatcher, extensões de
    // DRAW/PAINT, ações de sessão da ponte). Um id é "tratado" se está
    // registrado ou aparece como literal em core/editor/ponte.
    let rust = rust_sources();
    let exempt: BTreeSet<&str> = NOT_DISPATCHER_IDS.iter().map(|(id, _)| *id).collect();
    let mut unknown = BTreeSet::new();
    for (file, text) in cpp_sources() {
        for (number, line) in text.lines().enumerate() {
            for literal in string_literals(line) {
                if looks_like_command_id(&literal)
                    && !exempt.contains(literal.as_str())
                    && !dispatcher.contains(&literal)
                    && !rust.contains(&format!("\"{literal}\""))
                {
                    unknown.insert(format!("{file}:{} → {literal}", number + 1));
                }
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "ids de comando da UI sem registro no núcleo:\n{}",
        unknown.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn qt_ui_uses_the_mode_ids_published_by_the_editor() {
    // `ModelingMode::id()` publica "DRAW"/"POLY"; a UI nunca pode comparar com
    // grafias alternativas (regressão: `mode == "Draw"` escondia a barra DRAW).
    assert_eq!(petunia_editor::ModelingMode::Draw.id(), "DRAW");
    assert_eq!(petunia_editor::ModelingMode::Poly.id(), "POLY");
    let mut offenders = Vec::new();
    for (file, text) in cpp_sources() {
        for (number, line) in text.lines().enumerate() {
            let lowered_mode_compare = line.contains("mode") || line.contains("Mode");
            for literal in string_literals(line) {
                if lowered_mode_compare && matches!(literal.as_str(), "Draw" | "Poly") {
                    offenders.push(format!("{file}:{} → {literal}", number + 1));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "grafias de modo diferentes de DRAW/POLY:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn snapshot_publishes_draw_mode_for_the_draw_workspace() {
    use petunia_ui_cxxqt::editor::QtEditor;
    let mut editor = QtEditor::default();
    for (id, mode) in [("DRAW", "DRAW"), ("POLY", "POLY"), ("DRAW", "DRAW")] {
        editor
            .request(&serde_json::json!({"action": "workspace", "id": id}))
            .expect("troca de workspace");
        let snapshot = editor.snapshot();
        assert_eq!(snapshot["workspace"], "Model");
        assert_eq!(snapshot["modeling_mode"], mode);
    }
}
