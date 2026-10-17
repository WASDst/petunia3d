//! Completude de tradução do Shell v2: todo `tr("...")` dos widgets novos e todo
//! rótulo publicado pelo editor (catálogo de ferramentas e ações contextuais)
//! precisa existir em `assets/native-translations.json` com `pt-BR` e `en`.
use petunia_core::{SelectionDomain, Workspace};
use petunia_editor::{ModelingMode, context_actions, tool_catalog};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::PathBuf};

fn catalog() -> serde_json::Map<String, Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/native-translations.json");
    serde_json::from_str::<Value>(&fs::read_to_string(path).expect("catálogo legível"))
        .expect("catálogo JSON")
        .as_object()
        .expect("objeto")
        .clone()
}

/// Decodifica as escapes simples de um literal C++ (`\t`, `\n`, `\"`, `\\`).
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Literais `tr("...")` de um arquivo C++.
fn tr_literals(source: &str) -> Vec<String> {
    let bytes: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 < bytes.len() {
        let is_tr = bytes[i] == 't'
            && bytes[i + 1] == 'r'
            && bytes[i + 2] == '('
            && (i == 0 || !(bytes[i - 1].is_alphanumeric() || bytes[i - 1] == '_'));
        if is_tr {
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == '"' {
                let mut k = j + 1;
                let mut raw = String::new();
                while k < bytes.len() && bytes[k] != '"' {
                    if bytes[k] == '\\' && k + 1 < bytes.len() {
                        raw.push(bytes[k]);
                        k += 1;
                    }
                    raw.push(bytes[k]);
                    k += 1;
                }
                // só literais isolados: tr("...") fechando com ')'
                let mut m = k + 1;
                while m < bytes.len() && bytes[m].is_whitespace() {
                    m += 1;
                }
                if m < bytes.len() && bytes[m] == ')' {
                    out.push(unescape(&raw));
                }
                i = k;
            }
        }
        i += 1;
    }
    out
}

fn shell_sources() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cpp/shell");
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).expect("cpp/shell") {
        let path = entry.expect("entrada").path();
        if path.extension().and_then(|e| e.to_str()) == Some("cpp")
            && !path.file_name().unwrap().to_string_lossy().contains("harness")
        {
            files.push((
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).expect("fonte"),
            ));
        }
    }
    files
}

fn has_both(catalog: &serde_json::Map<String, Value>, key: &str) -> bool {
    catalog.get(key).is_some_and(|entry| {
        entry["pt-BR"].as_str().is_some_and(|v| !v.is_empty())
            && entry["en"].as_str().is_some_and(|v| !v.is_empty())
    })
}

#[test]
fn every_shell_string_is_in_the_native_catalog() {
    let catalog = catalog();
    let mut missing = BTreeSet::new();
    let mut total = 0;
    for (file, source) in shell_sources() {
        for literal in tr_literals(&source) {
            total += 1;
            if !has_both(&catalog, &literal) {
                missing.insert(format!("{file}: {literal:?}"));
            }
        }
    }
    assert!(total > 100, "extração de tr() falhou ({total} literais)");
    assert!(
        missing.is_empty(),
        "textos sem tradução em native-translations.json (rode tools/i18n_sync.py):\n{}",
        missing.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn every_editor_published_label_is_translated() {
    let catalog = catalog();
    let mut labels = BTreeSet::new();
    for (workspace, mode) in tool_catalog::all_contexts() {
        for tool in tool_catalog::tools_for(workspace, mode) {
            labels.insert(tool.label.to_string());
        }
    }
    for workspace in [Workspace::Model, Workspace::Uv] {
        for mode in [ModelingMode::Draw, ModelingMode::Poly] {
            for domain in [
                SelectionDomain::Object,
                SelectionDomain::Vertex,
                SelectionDomain::Edge,
                SelectionDomain::Face,
            ] {
                for action in context_actions::context_actions(workspace, mode, domain) {
                    labels.insert(action.label.to_string());
                }
            }
        }
    }
    let missing: Vec<_> = labels.iter().filter(|label| !has_both(&catalog, label)).collect();
    assert!(missing.is_empty(), "rótulos do editor sem tradução: {missing:?}");
}

#[test]
fn tool_and_action_icons_exist_in_the_icon_pack() {
    // O qrc embute todos os SVGs de `petunia-outline` por id plano (nome do arquivo).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/petunia-outline/svg");
    let mut stems = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("diretório de ícones").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("svg") {
                stems.insert(path.file_stem().unwrap().to_string_lossy().into_owned());
            }
        }
    }
    let has = |icon: &str| stems.contains(icon);
    let mut missing = BTreeSet::new();
    for (workspace, mode) in tool_catalog::all_contexts() {
        for tool in tool_catalog::tools_for(workspace, mode) {
            if !has(tool.icon) {
                missing.insert(tool.icon.to_string());
            }
        }
    }
    for workspace in [Workspace::Model, Workspace::Uv] {
        for mode in [ModelingMode::Draw, ModelingMode::Poly] {
            for domain in [
                SelectionDomain::Object,
                SelectionDomain::Vertex,
                SelectionDomain::Edge,
                SelectionDomain::Face,
            ] {
                for action in context_actions::context_actions(workspace, mode, domain) {
                    if !has(action.icon) {
                        missing.insert(action.icon.to_string());
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "ícones ausentes do pacote petunia-outline: {missing:?}");
}
