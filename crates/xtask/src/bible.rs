//! `bible-check` — conformidade do caderno canônico (`docs/bible/`) e das regras
//! de documentação do projeto (P3D-120).
//!
//! Verifica:
//! 1. completude e continuidade do caderno (00–16, 01–45, P3D-001–174, seções A–O, adendos);
//! 2. integridade dos links internos do caderno;
//! 3. ausência de caminhos mortos (`petunia3d-livro-vivo/`, `petunia-full-book/`);
//! 4. vocabulário de usuário (Point/Round Edge) nos manuais;
//! 5. congelamento do site público (hash dos arquivos congelados).

use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const CONSTITUTION_CHAPTERS: std::ops::RangeInclusive<u32> = 0..=16;
const FOUNDATIONS_CHAPTERS: std::ops::RangeInclusive<u32> = 1..=45;
const P3D_RANGE: std::ops::RangeInclusive<u32> = 1..=174;
const SECTION_LETTERS: &str = "ABCDEFGHIJKLMNO";

const DEAD_PATHS: [&str; 2] = ["petunia3d-livro-vivo/", "petunia-full-book/"];

/// Diretórios de documentação de usuário que devem falar a linguagem amigável.
const USER_FACING_DIRS: [&str; 3] = ["manual", "getting-started", "tools"];

/// Termos técnicos que não podem ser apresentados como vocabulário primário.
const FORBIDDEN_USER_TERMS: [(&str, &str); 3] = [
    (
        "Modo de Edição",
        "use o modelo unificado de seleção `Object / Face / Edge / Point`",
    ),
    (
        "Chanfro",
        "use **Round Edge** na linguagem amigável e `Bevel` apenas como termo técnico",
    ),
    (
        "Material Preview",
        "a V1 usa os modos-base `Wireframe / Solid / Textured / Silhouette` (cap. 36)",
    ),
];

/// Glifos usados como pseudo-ícones de UI. Iconografia é endereçada por `IconId`:
/// nenhuma página de usuário deve apresentar emoji/glifo no lugar do ícone real
/// (P3D-088: ausência de placeholders quadrado/círculo/emoji).
/// Emoji decorativos de heading não são pseudo-ícones de UI; a lista cobre apenas os
/// glifos que apareciam como rótulo/ícone de controle nas páginas de usuário.
const FORBIDDEN_GLYPHS: [&str; 17] = [
    "⬝", "╱", "▨", "➕", "▾", "🔒", "○", "●", "◐", "☼", "👁", "📦", "⚡", "⛶", "⇲", "⌖", "🗑",
];

const FROZEN_LOCK: &str = "docs/audits/bible-conformance/frozen-site.lock.json";
const FROZEN_SKIP: [&str; 3] = ["dist", "cache", "node_modules"];

pub fn check(root: &Path, update_lock: bool) -> Result<()> {
    let bible = root.join("docs/bible");
    if !bible.exists() {
        bail!("caderno canônico ausente: docs/bible/");
    }

    check_structure(&bible)?;
    check_links(&bible)?;
    check_dead_paths(root)?;
    check_vocabulary(root)?;
    check_frozen_site(root, update_lock)?;

    println!("🎉 Caderno canônico, links, vocabulário e congelamento do site conformes.");
    Ok(())
}

fn count_md(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                .count()
        })
        .unwrap_or(0)
}

fn parse_number(name: &str, width: usize) -> Option<u32> {
    name.get(..width)?.parse::<u32>().ok()
}

fn check_structure(bible: &Path) -> Result<()> {
    println!("📚 Verificando completude do caderno (docs/bible/)...");

    let mut problems: Vec<String> = Vec::new();

    // Constituição 00–16
    let constitution_dir = bible.join("constitution");
    let found: BTreeSet<u32> = std::fs::read_dir(&constitution_dir)
        .with_context(|| format!("falha ao ler {}", constitution_dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| parse_number(&e.file_name().to_string_lossy(), 2))
        .collect();
    for n in CONSTITUTION_CHAPTERS {
        if !found.contains(&n) {
            problems.push(format!("capítulo ausente: constitution/{n:02}"));
        }
    }

    // Fundamentos 01–45
    let foundations_dir = bible.join("foundations");
    let found: BTreeSet<u32> = std::fs::read_dir(&foundations_dir)
        .with_context(|| format!("falha ao ler {}", foundations_dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| parse_number(&e.file_name().to_string_lossy(), 2))
        .collect();
    for n in FOUNDATIONS_CHAPTERS {
        if !found.contains(&n) {
            problems.push(format!("capítulo ausente: foundations/{n:02}"));
        }
    }

    // Especificações P3D-001 a P3D-174
    let specs_dir = bible.join("specs");
    let found: BTreeSet<u32> = std::fs::read_dir(&specs_dir)
        .with_context(|| format!("falha ao ler {}", specs_dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_prefix("p3d-")
                .and_then(|rest| rest.get(..3))
                .and_then(|digits| digits.parse::<u32>().ok())
        })
        .collect();
    for n in P3D_RANGE {
        if !found.contains(&n) {
            problems.push(format!("especificação ausente: specs/p3d-{n:03}"));
        }
    }

    // Seções A–O
    let sections_dir = bible.join("sections");
    let present: Vec<String> = std::fs::read_dir(&sections_dir)
        .with_context(|| format!("falha ao ler {}", sections_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    for letter in SECTION_LETTERS.chars() {
        let prefix = format!("section-{}-", letter.to_ascii_lowercase());
        if !present.iter().any(|name| name.starts_with(&prefix)) {
            problems.push(format!("seção ausente: sections/{prefix}*"));
        }
    }

    // Adendos e raízes
    let addenda = count_md(&bible.join("addenda"));
    if addenda < 3 {
        problems.push(format!(
            "adendos insuficientes em addenda/ (encontrados {addenda}, esperados 3)"
        ));
    }
    for required in ["index.md", "especificacoes-p3d-readiness-gauntlet-waves.md"] {
        if !bible.join(required).exists() {
            problems.push(format!("página de entrada ausente: docs/bible/{required}"));
        }
    }

    if !problems.is_empty() {
        for p in &problems {
            eprintln!("  ✖ {p}");
        }
        bail!(
            "caderno canônico incompleto ({} problema(s))",
            problems.len()
        );
    }

    let total = walk_md(bible).len();
    println!(
        "✅ Caderno completo: {total} páginas markdown (00–16, 01–45, P3D-001–174, A–O, adendos)."
    );
    Ok(())
}

fn walk_md(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            // O check global não deve percorrer os artefatos de build/deps:
            // filtrar após walk_md(root) ainda atravessa árvores enormes.
            if matches!(
                entry.file_name().to_str(),
                Some("target" | "node_modules" | ".git")
            ) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn check_links(bible: &Path) -> Result<()> {
    println!("🔗 Verificando links internos do caderno...");
    let mut broken = Vec::new();
    let mut total = 0usize;

    for path in walk_md(bible) {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("falha ao ler {}", path.display()))?;
        for target in markdown_links(&content) {
            if target.starts_with("http") {
                continue;
            }
            total += 1;
            let clean = target.split('#').next().unwrap_or(&target).to_string();
            let decoded = percent_decode(&clean);
            let candidate = path.parent().unwrap_or(bible).join(&decoded);
            if !candidate.exists() {
                broken.push(format!(
                    "{} → {}",
                    path.strip_prefix(bible).unwrap_or(&path).display(),
                    clean
                ));
            }
        }
    }

    if !broken.is_empty() {
        for b in broken.iter().take(20) {
            eprintln!("  ✖ link quebrado: {b}");
        }
        bail!("{} link(s) interno(s) quebrado(s) no caderno", broken.len());
    }

    println!("✅ {total} links internos verificados, nenhum quebrado.");
    Ok(())
}

fn markdown_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = rest[..end].trim();
        if target.ends_with(".md") || target.contains(".md#") {
            links.push(target.to_string());
        }
    }
    links
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        let percent_escape = if bytes[i] == b'%' && i + 2 < bytes.len() {
            u8::from_str_radix(&input[i + 1..i + 3], 16).ok()
        } else {
            None
        };
        if let Some(value) = percent_escape {
            out.push(value as char);
            i += 3;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn check_dead_paths(root: &Path) -> Result<()> {
    println!("🚫 Verificando caminhos mortos de documentação...");
    let mut hits = Vec::new();
    for path in walk_md(root) {
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
        if relative.starts_with("docs/node_modules") || relative.starts_with("target") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for dead in DEAD_PATHS {
            if content.contains(dead) {
                hits.push(format!("{relative} referencia `{dead}`"));
            }
        }
    }
    if !hits.is_empty() {
        for hit in hits.iter().take(20) {
            eprintln!("  ✖ {hit}");
        }
        bail!(
            "{} referência(s) a caminho morto de documentação",
            hits.len()
        );
    }
    println!("✅ Nenhum caminho morto referenciado.");
    Ok(())
}

fn check_vocabulary(root: &Path) -> Result<()> {
    println!("🗣️  Verificando vocabulário de usuário...");
    let mut hits = Vec::new();
    for dir in USER_FACING_DIRS {
        for path in walk_md(&root.join("docs").join(dir)) {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if content.contains("STATUS DA ESPECIFICAÇÃO") || content.contains("legacy") {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            for (term, guidance) in FORBIDDEN_USER_TERMS {
                if content.contains(term) {
                    hits.push(format!(
                        "{relative} usa o termo técnico `{term}` como vocabulário primário — {guidance}"
                    ));
                }
            }
            for glyph in FORBIDDEN_GLYPHS {
                if content.contains(glyph) {
                    hits.push(format!(
                        "{relative} usa o glifo `{glyph}` como pseudo-ícone — iconografia usa `IconId`"
                    ));
                }
            }
        }
    }
    if !hits.is_empty() {
        for hit in hits.iter().take(20) {
            eprintln!("  ✖ {hit}");
        }
        bail!("{} violação(ões) de vocabulário de usuário", hits.len());
    }
    println!("✅ Vocabulário de usuário conforme (Point / Round Edge).");
    Ok(())
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn frozen_files(root: &Path) -> Vec<PathBuf> {
    let targets = [
        PathBuf::from("docs/.vitepress"),
        PathBuf::from("docs/index.md"),
        PathBuf::from("docs/public"),
        PathBuf::from("docs/package.json"),
        PathBuf::from("docs/vercel.json"),
        PathBuf::from("docs/image-references"),
        PathBuf::from(".github/workflows/docs.yml"),
    ];
    let mut files = Vec::new();
    for target in targets {
        let path = root.join(&target);
        if path.is_file() {
            files.push(target);
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let mut stack = vec![path];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.filter_map(|e| e.ok()) {
                let child = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if child.is_dir() {
                    if !FROZEN_SKIP.contains(&name.as_str()) {
                        stack.push(child);
                    }
                } else if child.is_file() {
                    files.push(child.strip_prefix(root).unwrap_or(&child).to_path_buf());
                }
            }
        }
    }
    files.sort();
    files
}

fn check_frozen_site(root: &Path, update_lock: bool) -> Result<()> {
    let lock_path = root.join(FROZEN_LOCK);
    let mut entries: Vec<(String, u64)> = Vec::new();
    for relative in frozen_files(root) {
        let bytes = std::fs::read(root.join(&relative))
            .with_context(|| format!("falha ao ler {}", relative.display()))?;
        entries.push((
            relative.to_string_lossy().replace('\\', "/"),
            fnv1a64(&bytes),
        ));
    }
    let serialized = serde_json::to_string_pretty(&serde_json::json!({
        "description": "Hash dos arquivos do site público de documentação, congelado até o fim do projeto (AGENTS.md §1). Regenere com `cargo run -p xtask -- bible-lock` apenas após decisão explícita de descongelar.",
        "algorithm": "fnv1a64",
        "files": entries.iter().map(|(path, hash)| serde_json::json!({ "path": path, "hash": format!("{hash:016x}") })).collect::<Vec<_>>(),
    }))?;

    if update_lock {
        std::fs::write(&lock_path, serialized + "\n")
            .with_context(|| format!("falha ao escrever {}", lock_path.display()))?;
        println!(
            "🔒 Lock do site atualizado: {FROZEN_LOCK} ({} arquivos).",
            entries.len()
        );
        return Ok(());
    }

    println!("🧊 Verificando congelamento do site de documentação...");
    let Ok(current) = std::fs::read_to_string(&lock_path) else {
        bail!("lock ausente ({FROZEN_LOCK}); rode `cargo run -p xtask -- bible-lock`");
    };
    let Ok(expected) = serde_json::from_str::<serde_json::Value>(&current) else {
        bail!("lock inválido: {FROZEN_LOCK}");
    };
    let Some(expected_files) = expected.get("files").and_then(|value| value.as_array()) else {
        bail!("lock sem campo `files`: {FROZEN_LOCK}");
    };

    let expected: Vec<(String, String)> = expected_files
        .iter()
        .filter_map(|entry| {
            Some((
                entry.get("path")?.as_str()?.to_string(),
                entry.get("hash")?.as_str()?.to_string(),
            ))
        })
        .collect();

    let mut changes = Vec::new();
    for (path, hash) in &entries {
        let hash = format!("{hash:016x}");
        match expected
            .iter()
            .find(|(expected_path, _)| expected_path == path)
        {
            Some((_, expected_hash)) if *expected_hash == hash => {}
            Some(_) => changes.push(format!("modificado: {path}")),
            None => changes.push(format!("adicionado: {path}")),
        }
    }
    for (path, _) in &expected {
        if !entries.iter().any(|(current, _)| current == path) {
            changes.push(format!("removido: {path}"));
        }
    }

    if !changes.is_empty() {
        for change in changes.iter().take(20) {
            eprintln!("  ✖ {change}");
        }
        bail!(
            "{} alteração(ões) no site público, que está CONGELADO até o fim do projeto (ver AGENTS.md §1)",
            changes.len()
        );
    }

    println!(
        "✅ Site público intacto e congelado ({} arquivos).",
        entries.len()
    );
    Ok(())
}
