//! Validação dos packs de ícones (`assets/icons/petunia-{outline,filled}`).
//!
//! Regras (cap. 36, revisão 2026-09-30): grade 24×24, arte monocromática em
//! `currentColor` (sem cores fixas), traços só nas larguras permitidas e cada
//! ícone presente nos dois packs. Isso mantém os ícones tingíveis por tema.

use anyhow::{Result, bail};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Larguras de traço permitidas: padrão + ênfases pontuais.
const ALLOWED_STROKES: [&str; 5] = ["1.75", "2", "2.2", "2.5", "3.2"];
/// Cores permitidas: só `currentColor`, `none` e preto/branco dentro de máscaras.
const ALLOWED_COLORS: [&str; 5] = ["currentColor", "none", "#000", "#fff", "transparent"];

fn svg_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            svg_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "svg") {
            out.push(path);
        }
    }
}

fn attr_values<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let needle = format!(" {name}=\"");
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(&needle) {
        rest = &rest[pos + needle.len()..];
        if let Some(end) = rest.find('"') {
            found.push(&rest[..end]);
        }
    }
    found
}

pub fn run(root: &Path) -> Result<()> {
    println!("🎨 Validando packs de ícones...");
    let mut problems = Vec::new();
    let mut names: Vec<BTreeSet<String>> = Vec::new();
    for variant in ["outline", "filled"] {
        let base = root.join(format!("assets/icons/petunia-{variant}/svg"));
        let mut files = Vec::new();
        svg_files(&base, &mut files);
        if files.is_empty() {
            bail!("pack petunia-{variant} sem SVGs em {}", base.display());
        }
        let mut set = BTreeSet::new();
        for file in &files {
            let name = file.file_stem().unwrap().to_string_lossy().to_string();
            set.insert(name.clone());
            let text = std::fs::read_to_string(file)?;
            if !text.contains("viewBox=\"0 0 24 24\"") {
                problems.push(format!("{variant}/{name}: viewBox diferente de 0 0 24 24"));
            }
            for attr in ["fill", "stroke", "color"] {
                for value in attr_values(&text, attr) {
                    if !ALLOWED_COLORS.contains(&value) {
                        problems.push(format!(
                            "{variant}/{name}: {attr}=\"{value}\" fixo (use currentColor)"
                        ));
                    }
                }
            }
            for width in attr_values(&text, "stroke-width") {
                if !ALLOWED_STROKES.contains(&width) {
                    problems.push(format!(
                        "{variant}/{name}: stroke-width {width} fora do padrão"
                    ));
                }
            }
        }
        names.push(set);
    }
    for missing in names[0].symmetric_difference(&names[1]) {
        problems.push(format!("{missing}: presente em apenas um dos packs"));
    }
    if !problems.is_empty() {
        for p in problems.iter().take(40) {
            eprintln!("  - {p}");
        }
        bail!("{} problema(s) nos ícones", problems.len());
    }
    println!(
        "✅ {} ícones × 2 variantes conformes (24 px, currentColor, traços padronizados).",
        names[0].len()
    );
    Ok(())
}
