//! Guarda de tokens do shell Slint (`crates/ui-slint/ui/*.slint`).
//!
//! Regras (AGENTS.md §3, cap. 24/36 e plano de UI 2026-10-04 §8.5): fora de
//! `tokens.slint`, nenhum markup usa cor `#hex`, `font-size` ou
//! `drop-shadow-blur` em px literal, nem `duration` literal (que ignoraria o
//! movimento reduzido). Valores dentro de strings (ex.: o hex de um preset
//! enviado ao Rust) e comentários não contam.

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
    has_literal_unit(code, prop, "px")
}

/// `duration: <número>ms|s` literal: animação que ignora `Motion.reduced`
/// (WCAG 2.3.3; plano de UI F8).
fn has_literal_duration(code: &str) -> bool {
    has_literal_unit(code, "duration", "ms") || has_literal_unit(code, "duration", "s")
}

/// `prop: <número><unidade>` com valor literal.
fn has_literal_unit(code: &str, prop: &str, unit: &str) -> bool {
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
    digits > 0 && rest[digits..].starts_with(unit)
}

/// Eixo principal de um box layout do Slint.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn index(self) -> usize {
        match self {
            Axis::Horizontal => 0,
            Axis::Vertical => 1,
        }
    }

    fn stretch_property(self) -> &'static str {
        match self {
            Axis::Horizontal => "horizontal-stretch",
            Axis::Vertical => "vertical-stretch",
        }
    }
}

/// Bloco `{ … }` aberto durante a varredura.
enum Block {
    /// Elemento cujo tipo é um box layout (`HorizontalLayout { … }` ou
    /// `component X inherits VerticalLayout { … }`).
    Layout {
        line: usize,
        axis: Axis,
        alignment: Option<String>,
        /// Stretch do próprio layout quando ele é filho de outro layout.
        stretch: [Option<bool>; 2],
        /// Por filho direto, na ordem: estica no eixo principal?
        children: Vec<bool>,
    },
    /// Qualquer outro elemento (tipo em Maiúscula).
    Element { stretch: [Option<bool>; 2] },
    /// Callback, `animate`, `function`, `if` de código…
    Other,
}

/// Layout cujo `alignment` ≠ `stretch` convive com um filho elástico no eixo
/// principal: em Slint 1.18 o solver dá a cada filho o tamanho preferido e
/// ignora o stretch (`solve_box_layout`), como na linha do Outliner em que o
/// centro do clique caía em "Ocultar".
struct IgnoredStretch {
    line: usize,
    alignment: String,
    /// Há irmãos depois do primeiro filho elástico (botões à direita etc.).
    has_trailing_sibling: bool,
}

/// Tipo do bloco a partir do texto antes de `{`.
fn classify(head: &str) -> Block {
    let head = head.trim();
    let first = head.split_whitespace().next().unwrap_or("");
    if head.contains("=>") || matches!(first, "animate" | "function" | "pure" | "public") {
        return Block::Other;
    }
    let ident = head
        .rsplit(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .next()
        .unwrap_or("");
    let axis = match ident {
        "HorizontalLayout" => Some(Axis::Horizontal),
        "VerticalLayout" => Some(Axis::Vertical),
        _ => None,
    };
    match axis {
        Some(axis) => Block::Layout {
            line: 0,
            axis,
            alignment: None,
            stretch: [None; 2],
            children: Vec::new(),
        },
        None if ident.starts_with(|c: char| c.is_ascii_uppercase()) => {
            Block::Element { stretch: [None; 2] }
        }
        None => Block::Other,
    }
}

/// `alignment: x` ou `<eixo>-stretch: n` aplicados ao bloco do topo.
fn apply_statement(statement: &str, top: Option<&mut Block>) {
    let Some((name, value)) = statement.split_once(':') else {
        return;
    };
    let (name, value) = (name.trim(), value.trim());
    match top {
        Some(Block::Layout { alignment, .. }) if name == "alignment" => {
            *alignment = Some(value.to_string());
        }
        Some(Block::Element { stretch } | Block::Layout { stretch, .. }) => {
            // Só fator literal > 0: expressões (`a ? 0 : 1`) não contam.
            let positive = value.parse::<f32>().is_ok_and(|v| v > 0.0);
            for axis in [Axis::Horizontal, Axis::Vertical] {
                if name == axis.stretch_property() {
                    stretch[axis.index()] = Some(positive);
                }
            }
        }
        _ => {}
    }
}

/// Encerra o bloco do topo e registra o resultado no pai/achados.
fn close_block(stack: &mut Vec<Block>, found: &mut Vec<IgnoredStretch>) {
    let Some(block) = stack.pop() else {
        return;
    };
    let main_axis_stretch = |parent_axis: Axis| match &block {
        Block::Element { stretch } | Block::Layout { stretch, .. } => {
            stretch[parent_axis.index()].unwrap_or(false)
        }
        _ => false,
    };
    if let Some(Block::Layout { axis, children, .. }) = stack.last_mut() {
        if !matches!(block, Block::Other) {
            children.push(main_axis_stretch(*axis));
        }
    }
    if let Block::Layout {
        line,
        alignment: Some(alignment),
        children,
        ..
    } = block
    {
        if alignment != "stretch" {
            if let Some(first) = children.iter().position(|s| *s) {
                found.push(IgnoredStretch {
                    line,
                    alignment,
                    has_trailing_sibling: first + 1 < children.len(),
                });
            }
        }
    }
}

/// Varre a estrutura de chaves e devolve layouts com stretch ignorado.
fn ignored_stretch(source: &str) -> Vec<IgnoredStretch> {
    let mut stack: Vec<Block> = Vec::new();
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let mut segment = String::new();
        for c in code_only(line).chars() {
            match c {
                '{' => {
                    let mut block = classify(&segment);
                    if let Block::Layout { line, .. } = &mut block {
                        *line = index + 1;
                    }
                    stack.push(block);
                    segment.clear();
                }
                '}' | ';' => {
                    apply_statement(&segment, stack.last_mut());
                    segment.clear();
                    if c == '}' {
                        close_block(&mut stack, &mut found);
                    }
                }
                _ => segment.push(c),
            }
        }
    }
    found
}

/// Política: quais combinações viram violação de lint.
///
/// Não abrir exceção para títulos no fim: a área de texto e de clique também
/// deve preencher a sobra, mesmo quando não há botão à direita.
fn ignores_declared_stretch(alignment: &str, _has_trailing_sibling: bool) -> bool {
    alignment != "stretch"
}

fn check_source(source: &str) -> Vec<Violation> {
    let mut found: Vec<Violation> = ignored_stretch(source)
        .into_iter()
        .filter(|hit| ignores_declared_stretch(&hit.alignment, hit.has_trailing_sibling))
        .map(|hit| Violation {
            line: hit.line,
            rule: "alignment ≠ stretch ignora o horizontal/vertical-stretch do filho",
            text: format!("alignment: {}", hit.alignment),
        })
        .collect();
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
        if has_literal_duration(&code) {
            push("duration literal (use Motion.*, que respeita movimento reduzido)");
        }
        if has_raw_interpolation(line) {
            push("interpolação sem \\{ (o Slint mostra `{expr}` literalmente; use `\\{expr}`)");
        }
    }
    found
}

/// String com `{ident…` sem a barra: o Slint só interpola `\{expr}` e mostraria
/// o texto cru (ex.: "({root.items.length})" na Asset Library).
fn has_raw_interpolation(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let (mut in_str, mut escaped) = (false, false);
    for (i, &c) in chars.iter().enumerate() {
        if !in_str {
            if c == '/' && chars.get(i + 1) == Some(&'/') {
                return false;
            }
            in_str = c == '"';
            continue;
        }
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            in_str = false;
        } else if c == '{' && chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase()) {
            return true;
        }
    }
    false
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
        assert_eq!(rules("animate x { duration: 150ms; }").len(), 1);
    }

    #[test]
    fn accepts_tokens_strings_and_comments() {
        assert!(rules("background: DesignTokens.hud-surface;").is_empty());
        assert!(rules("font-size: DesignTokens.font-small;").is_empty());
        assert!(rules("drop-shadow-blur: DesignTokens.elevation-1-blur;").is_empty());
        assert!(rules("animate x { duration: Motion.fast; }").is_empty());
        assert!(rules(r##"clicked => { root.set("#E96A00"); }"##).is_empty());
        assert!(rules("// laranja #E96A00").is_empty());
        assert!(rules(r##"text: "\"#abc\" ok";"##).is_empty());
    }

    #[test]
    fn reports_line_numbers() {
        let found = check_source("a: b;\ncolor: #ffffff;\n");
        assert_eq!(found[0].line, 2);
    }

    /// (linha, alignment, há irmão depois do filho elástico)
    fn stretch_hits(src: &str) -> Vec<(usize, String, bool)> {
        ignored_stretch(src)
            .into_iter()
            .map(|h| (h.line, h.alignment, h.has_trailing_sibling))
            .collect()
    }

    #[test]
    fn flags_interpolation_without_backslash() {
        assert!(has_raw_interpolation(r#"text: "({root.items.length})";"#));
        assert!(has_raw_interpolation(r#"text: "{item.parts} parts";"#));
        assert!(!has_raw_interpolation(r#"text: "(\{root.items.length})";"#));
        assert!(!has_raw_interpolation(r#"text: "{}";"#), "chaves sem identificador");
        assert!(!has_raw_interpolation(r#"x: 1; // "{root.a}" em comentário"#));
        assert!(!has_raw_interpolation(r#"if a { b: "ok"; }"#), "chave de bloco fora de string");
    }

    #[test]
    fn finds_rows_whose_stretch_child_is_ignored() {
        // Linha do Outliner: nome elástico + botão Ocultar à direita.
        let row = "HorizontalLayout {\n    alignment: center;\n    Text { horizontal-stretch: 1; }\n    Rectangle { width: 24px; }\n}\n";
        assert_eq!(stretch_hits(row), vec![(1, "center".into(), true)]);
        // Espaçador de uma linha e componente que herda o layout.
        let spacer = "component Bar inherits HorizontalLayout {\n    spacing: 8px; alignment: start;\n    Text {}\n    Rectangle { horizontal-stretch: 1; }\n    Text {}\n}\n";
        assert_eq!(stretch_hits(spacer), vec![(1, "start".into(), true)]);
        // Eixo vertical: o stretch que conta é o do eixo principal.
        let rail = "VerticalLayout {\n    alignment: start;\n    Rectangle { height: 32px; }\n    Rectangle { vertical-stretch: 1; }\n    Rectangle { height: 32px; }\n}\n";
        assert_eq!(stretch_hits(rail), vec![(1, "start".into(), true)]);
        // Título como último filho: o scanner relata, a política decide.
        let header = "HorizontalLayout {\n    alignment: start;\n    Rectangle { width: 16px; }\n    Text { horizontal-stretch: 1; }\n}\n";
        assert_eq!(stretch_hits(header), vec![(1, "start".into(), false)]);
    }

    #[test]
    fn ignores_layouts_that_honour_stretch() {
        let stretch = "HorizontalLayout {\n    Text { horizontal-stretch: 1; }\n    Rectangle { width: 24px; }\n}\n";
        assert!(
            stretch_hits(stretch).is_empty(),
            "alignment padrão é stretch"
        );
        let explicit = "HorizontalLayout { alignment: stretch; Text { horizontal-stretch: 1; } }";
        assert!(stretch_hits(explicit).is_empty());
        // Eixo cruzado e expressões não contam.
        let cross = "HorizontalLayout { alignment: center; Text { vertical-stretch: 1; } }";
        assert!(stretch_hits(cross).is_empty());
        let expr =
            "VerticalLayout { alignment: start; Rectangle { vertical-stretch: c ? 0 : 1; } }";
        assert!(stretch_hits(expr).is_empty());
        // Neto elástico dentro de um filho não é filho direto.
        let nested =
            "HorizontalLayout { alignment: center; Rectangle { Text { horizontal-stretch: 1; } } }";
        assert!(stretch_hits(nested).is_empty());
        // Callbacks e strings com chaves não abrem elementos.
        let noise = "HorizontalLayout {\n    alignment: center;\n    clicked => { x = \"{\"; }\n    Text { text: \"\\{a}\"; }\n}\n";
        assert!(stretch_hits(noise).is_empty());
    }

    #[test]
    fn retains_both_stretch_axes_independently_of_statement_order() {
        let row = "HorizontalLayout { alignment: center; Rectangle { horizontal-stretch: 1; vertical-stretch: 0; } }";
        assert_eq!(stretch_hits(row), vec![(1, "center".into(), false)]);
        let reverse = "HorizontalLayout { alignment: center; Rectangle { vertical-stretch: 0; horizontal-stretch: 1; } }";
        assert_eq!(stretch_hits(reverse), vec![(1, "center".into(), false)]);
        let column = "VerticalLayout { alignment: start; Rectangle { vertical-stretch: 1; horizontal-stretch: 0; } }";
        assert_eq!(stretch_hits(column), vec![(1, "start".into(), false)]);
    }

    #[test]
    fn checks_direct_layout_children_without_promoting_grandchild_stretch() {
        let direct = "HorizontalLayout { alignment: center; VerticalLayout { horizontal-stretch: 1; Text {} } Rectangle {} }";
        assert_eq!(stretch_hits(direct), vec![(1, "center".into(), true)]);
        let grandchild = "HorizontalLayout { alignment: center; VerticalLayout { Text { horizontal-stretch: 1; } } }";
        assert!(stretch_hits(grandchild).is_empty());
        let expression = "HorizontalLayout { alignment: center; VerticalLayout { horizontal-stretch: condition ? 0 : 1; } }";
        assert!(stretch_hits(expression).is_empty());
    }

    #[test]
    fn policy_flags_buttons_that_drift_off_the_edge() {
        assert!(ignores_declared_stretch("center", true));
        assert!(ignores_declared_stretch("start", true));
        assert!(ignores_declared_stretch("end", false));
        assert!(ignores_declared_stretch("start", false));
        assert!(!ignores_declared_stretch("stretch", true));
    }
}
