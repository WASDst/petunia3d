//! Guarda de tokens do shell Slint (`crates/ui-slint/ui/*.slint`).
//!
//! Regras (AGENTS.md §3, cap. 24/36 e plano de UI 2026-10-04 §8.5): fora de
//! `tokens.slint`, nenhum markup usa cor `#hex`, `font-size` em px literal ou
//! `drop-shadow-blur` em px literal. Valores dentro de strings (ex.: o hex de
//! um preset enviado ao Rust) e comentários não contam.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

/// Arquivo onde os valores literais são permitidos.
const TOKENS_FILE: &str = "tokens.slint";

#[derive(Debug, PartialEq, Eq)]
struct Violation {
    line: usize,
    rule: &'static str,
    text: String,
}

/// Remove comentário de linha e o conteúdo de strings, preservando o resto.
fn code_only(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_str = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
                out.push('"');
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if c == '"' {
            in_str = true;
        }
        out.push(c);
    }
    out
}

fn has_hex_color(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.iter().enumerate().any(|(i, &b)| {
        if b != b'#' {
            return false;
        }
        let digits = bytes[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        matches!(digits, 3 | 4 | 6 | 8)
    })
}

/// `prop: <número>px` com valor literal.
fn has_literal_px(code: &str, prop: &str) -> bool {
    let Some(pos) = code.find(prop) else {
        return false;
    };
    let rest = code[pos + prop.len()..].trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    let rest = rest.trim_start();
    let digits = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .count();
    digits > 0 && rest[digits..].starts_with("px")
}

fn check_source(source: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let code = code_only(line);
        let mut push = |rule| {
            found.push(Violation {
                line: index + 1,
                rule,
                text: line.trim().to_string(),
            })
        };
        if has_hex_color(&code) {
            push("cor #hex fora de tokens.slint");
        }
        if has_literal_px(&code, "font-size") {
            push("font-size literal (use DesignTokens.font-*)");
        }
        if has_literal_px(&code, "drop-shadow-blur") {
            push("drop-shadow-blur literal (use DesignTokens.elevation-*)");
        }
    }
    found
}

/// Todos os `.slint` sob `dir`, inclusive subpastas (`components/`, `inspector/`…).
fn slint_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            slint_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "slint") {
            out.push(path);
        }
    }
}

pub fn run(root: &Path) -> Result<()> {
    println!("🎨 Validando tokens do shell Slint...");
    let mut files = Vec::new();
    slint_files(&root.join("crates/ui-slint/ui"), &mut files);
    files.retain(|p| p.file_name().is_some_and(|n| n != TOKENS_FILE));
    files.sort();

    let mut total = 0;
    for path in &files {
        let source = std::fs::read_to_string(path)?;
        let rel = path.strip_prefix(root).unwrap_or(path).display();
        for v in check_source(&source) {
            eprintln!("  ✖ {rel}:{} — {}: {}", v.line, v.rule, v.text);
            total += 1;
        }
    }
    if total > 0 {
        bail!("{total} violação(ões) de token no shell Slint");
    }
    println!(
        "✅ {} arquivo(s) .slint sem cor, fonte ou sombra literal.",
        files.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(src: &str) -> Vec<&'static str> {
        check_source(src).into_iter().map(|v| v.rule).collect()
    }

    #[test]
    fn flags_literal_values() {
        assert_eq!(rules("background: #14141ef0;").len(), 1);
        assert_eq!(rules("color: #fff;").len(), 1);
        assert_eq!(rules("font-size: 9px;").len(), 1);
        assert_eq!(rules("drop-shadow-blur: 12px;").len(), 1);
    }

    #[test]
    fn accepts_tokens_strings_and_comments() {
        assert!(rules("background: DesignTokens.hud-surface;").is_empty());
        assert!(rules("font-size: DesignTokens.font-small;").is_empty());
        assert!(rules("drop-shadow-blur: DesignTokens.elevation-1-blur;").is_empty());
        assert!(rules(r##"clicked => { root.set("#E96A00"); }"##).is_empty());
        assert!(rules("// laranja #E96A00").is_empty());
        assert!(rules(r##"text: "\"#abc\" ok";"##).is_empty());
    }

    #[test]
    fn reports_line_numbers() {
        let found = check_source("a: b;\ncolor: #ffffff;\n");
        assert_eq!(found[0].line, 2);
    }
}
