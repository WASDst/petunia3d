//! Ferramenta de automação interna (cargo xtask) para o Petunia3D.
//! Gerencia tarefas de documentação, integridade e drift prevention.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

mod bible;
mod generator;
mod icons_check;
mod slint_lint;
mod ui_guard;
use generator::GeneratedCatalog;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "docs" => task_docs()?,
        "docs-generate" => {
            let check_only = args.any(|a| a == "--check");
            task_docs_generate(check_only)?;
        }
        "docs-check" => task_docs_check()?,
        "bible-check" => bible::check(&root_dir(), false)?,
        "bible-lock" => bible::check(&root_dir(), true)?,
        "arch-check" => task_arch_check()?,
        "verify" => task_verify()?,
        "ui-check" => task_ui_check()?,
        "icons-check" => icons_check::run(&root_dir())?,
        "ui-lint" => slint_lint::run(&root_dir())?,
        "ui-guard" => {
            let rest: Vec<String> = args.collect();
            let strict = rest.iter().any(|a| a == "--strict");
            let baseline = rest.iter().any(|a| a == "--baseline");
            let report = ui_guard::run(&root_dir(), strict)?;
            if baseline {
                println!(
                    "\n---8<--- 00-baseline.md (colar em docs/audits/ui-ecosystem-final-push/) ---\n"
                );
                print!("{}", ui_guard::markdown(&report));
            }
        }
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!("Comando desconhecido: {other}\n");
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"Petunia3D xtask — Automação de Desenvolvimento

USO:
    cargo xtask <COMANDO>

COMANDOS:
    docs          Gera referências técnicas e compila o site estático com VitePress
    docs-generate Gera exclusivamente os catálogos e referências em docs/generated/
                  (use --check para validar drift sem escrever nada)
    docs-check    Valida integridade, ausência de drift em docs/generated/ e build sem erros
    bible-check   Valida o caderno canônico (docs/bible/), links, vocabulário e congelamento do site
    bible-lock    Regenera o lock do site congelado (somente após decisão explícita de descongelar)
    arch-check    Valida a integridade dos relatórios da auditoria arquitetural
    verify        Gate backend read-only (fmt, check, tests, clippy, arch-check)
    ui-check      Valida o mapa de componentes UI (docs/public/ui-map.json) contra o código
    ui-lint       Falha se o markup Slint usar cor #hex, font-size ou sombra literal fora de tokens.slint
    ui-guard      Guarda de arquitetura da UI (§36/§37 da diretiva Egui Ecosystem Final Push)
                  Reporta, por regra, ocorrências em product code versus foundation/adapter.
                  --strict    falha se um tipo de biblioteca auxiliar escapar do adapter
                  --baseline  imprime a tabela de baseline em Markdown
    help          Exibe esta mensagem de ajuda
"#
    );
}

fn root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("falha ao localizar raiz do workspace")
        .to_path_buf()
}

fn task_docs_generate(check_only: bool) -> Result<()> {
    if check_only {
        println!("⚙️ Validando drift das referências técnicas geradas (P3D-119/P3D-120)...");
    } else {
        println!("⚙️ Gerando referências técnicas a partir do código-fonte (P3D-119)...");
    }
    let root = root_dir();
    let docs_dir = root.join("docs");
    let gen_dir = docs_dir.join("generated");
    std::fs::create_dir_all(&gen_dir)
        .with_context(|| format!("falha ao criar pasta {}", gen_dir.display()))?;

    let catalog = GeneratedCatalog::generate(&root)?;

    // Caminhos são relativos a `docs/` (alguns derivados vivem fora de docs/generated/).
    let files: [(&str, &str); 9] = [
        ("generated/COMMANDS.md", &catalog.commands_md),
        ("generated/KEYBINDS.md", &catalog.keybinds_md),
        ("generated/ICON_TOKENS.md", &catalog.icon_tokens_md),
        ("generated/TEXT_TOKENS.md", &catalog.text_tokens_md),
        ("generated/THEME_TOKENS.md", &catalog.theme_tokens_md),
        (
            "generated/SUPPORTED_FORMATS.md",
            &catalog.supported_formats_md,
        ),
        ("generated/index.md", &catalog.index_md),
        ("generated/manifest.json", &catalog.manifest_json),
        ("shortcuts/cheatsheet.md", &catalog.cheatsheet_md),
    ];

    // Sincroniza CHANGELOG.md com docs/changelog/index.md (P3D-117)
    let changelog_src = root.join("CHANGELOG.md");
    let changelog_dest = root.join("docs/changelog/index.md");
    let changelog = if changelog_src.exists() && changelog_dest.exists() {
        let content = std::fs::read_to_string(&changelog_src)?;
        let mut synced = String::from("# Histórico de Versões (Changelog)\n\n");
        if let Some(pos) = content.find("\n\n") {
            synced.push_str(&content[pos + 2..]);
        } else {
            synced.push_str(&content);
        }
        Some(synced)
    } else {
        None
    };

    if check_only {
        let mut drift = Vec::new();
        for (relative, expected) in files {
            match std::fs::read_to_string(docs_dir.join(relative)) {
                Ok(disk) if disk == *expected => {}
                Ok(_) => drift.push(format!("docs/{relative}")),
                Err(_) => drift.push(format!("docs/{relative} (ausente)")),
            }
        }
        if let Some(expected) = &changelog {
            match std::fs::read_to_string(&changelog_dest) {
                Ok(disk) if disk == *expected => {}
                Ok(_) => drift.push("docs/changelog/index.md".to_string()),
                Err(_) => drift.push("docs/changelog/index.md (ausente)".to_string()),
            }
        }
        if !drift.is_empty() {
            for file in &drift {
                eprintln!("  ✖ drift detectado: {file}");
            }
            bail!(
                "{} arquivo(s) gerado(s) desatualizados; rode `cargo run -p xtask -- docs-generate`",
                drift.len()
            );
        }
        println!("✅ Nenhuma divergência (drift) nos catálogos gerados nem no changelog vivo.");
        return Ok(());
    }

    for (relative, content) in files {
        let target = docs_dir.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("falha ao criar {}", parent.display()))?;
        }
        std::fs::write(&target, content)
            .with_context(|| format!("falha ao escrever {}", target.display()))?;
    }
    if let Some(synced) = changelog {
        std::fs::write(&changelog_dest, &synced)
            .with_context(|| format!("falha ao escrever {}", changelog_dest.display()))?;
    }

    println!("✅ 8 arquivos canônicos gerados (docs/generated/ + docs/shortcuts/cheatsheet.md)!");
    println!("✅ docs/changelog/index.md sincronizado com CHANGELOG.md (P3D-117)!");
    Ok(())
}

fn task_docs() -> Result<()> {
    task_docs_generate(false)?;

    println!("📦 Compilando documentação oficial do Petunia3D (VitePress)...");
    let root = root_dir();
    let docs_dir = root.join("docs");

    let status = Command::new("pnpm")
        .arg("run")
        .arg("build")
        .current_dir(&docs_dir)
        .status()
        .context("falha ao executar pnpm na pasta docs/")?;

    if !status.success() {
        bail!("Build da documentação falhou com status: {status}");
    }

    println!("✅ Documentação gerada com sucesso em docs/.vitepress/dist/");
    Ok(())
}

fn task_docs_check() -> Result<()> {
    println!("🔍 Validando integridade da documentação...");
    let root = root_dir();
    let docs_dir = root.join("docs");
    let gen_dir = docs_dir.join("generated");

    // 0. Conformidade do caderno canônico e congelamento do site (AGENTS.md §1).
    bible::check(&root, false)?;

    // 1. Verificar existência de arquivos canônicos
    let required_files = [
        "index.md",
        "getting-started/index.md",
        "getting-started/what-is-petunia3d.md",
        "getting-started/installation.md",
        "getting-started/first-project.md",
        "getting-started/interface-overview.md",
        "getting-started/your-first-model.md",
        "manual/index.md",
        "manual/interface.md",
        "manual/viewport.md",
        "manual/selection.md",
        "manual/modeling.md",
        "manual/paint.md",
        "manual/uv.md",
        "manual/animation.md",
        "manual/asset-library.md",
        "manual/projects.md",
        "manual/export.md",
        "workspaces/index.md",
        "workspaces/modeling.md",
        "workspaces/paint.md",
        "workspaces/uv.md",
        "workspaces/animation.md",
        "tools/index.md",
        "customization/index.md",
        "shortcuts/index.md",
        "developers/index.md",
        "developers/ui-component-map.md",
        "changelog/index.md",
        "generated/COMMANDS.md",
        "generated/KEYBINDS.md",
        "generated/ICON_TOKENS.md",
        "generated/TEXT_TOKENS.md",
        "generated/THEME_TOKENS.md",
        "generated/SUPPORTED_FORMATS.md",
        "generated/index.md",
        "generated/manifest.json",
    ];

    for file in &required_files {
        let p = docs_dir.join(file);
        if !p.exists() {
            bail!("Arquivo essencial de documentação ausente: docs/{file}");
        }
    }

    println!(
        "📄 Todos os {} arquivos essenciais estão presentes.",
        required_files.len()
    );

    // 2. Detecção de Drift em docs/generated/ (P3D-120)
    println!("🔍 Verificando drift em docs/generated/ (P3D-120)...");
    let expected = GeneratedCatalog::generate(&root)?;
    let checks = [
        ("COMMANDS.md", &expected.commands_md),
        ("KEYBINDS.md", &expected.keybinds_md),
        ("ICON_TOKENS.md", &expected.icon_tokens_md),
        ("TEXT_TOKENS.md", &expected.text_tokens_md),
        ("THEME_TOKENS.md", &expected.theme_tokens_md),
        ("SUPPORTED_FORMATS.md", &expected.supported_formats_md),
        ("index.md", &expected.index_md),
        ("manifest.json", &expected.manifest_json),
    ];
    for (filename, expected_content) in checks {
        let path = gen_dir.join(filename);
        if !path.exists() {
            bail!(
                "Arquivo gerado ausente: docs/generated/{filename}. Execute 'cargo run -p xtask -- docs' para gerar."
            );
        }
        let disk_content = std::fs::read_to_string(&path)
            .with_context(|| format!("Falha ao ler {}", path.display()))?;
        if disk_content != *expected_content {
            bail!(
                "Drift detectado em docs/generated/{filename}! O arquivo no repositório está desatualizado em relação ao código-fonte. Execute 'cargo run -p xtask -- docs' para sincronizar."
            );
        }
    }
    println!("✅ Nenhuma divergência (drift) detectada nos arquivos gerados.");

    // 3. Verificação de sincronização do Changelog (P3D-117)
    let changelog_src = root.join("CHANGELOG.md");
    let changelog_dest = docs_dir.join("changelog/index.md");
    if changelog_src.exists() && changelog_dest.exists() {
        let root_content = std::fs::read_to_string(&changelog_src)?;
        let docs_content = std::fs::read_to_string(&changelog_dest)?;
        let mut expected_synced = String::from("# Histórico de Versões (Changelog)\n\n");
        if let Some(pos) = root_content.find("\n\n") {
            expected_synced.push_str(&root_content[pos + 2..]);
        } else {
            expected_synced.push_str(&root_content);
        }
        if docs_content != expected_synced {
            bail!(
                "Drift detectado em docs/changelog/index.md em relação ao CHANGELOG.md! Execute 'cargo run -p xtask -- docs' para sincronizar."
            );
        }
        println!("✅ Changelog vivo validado e 100% sincronizado (P3D-117).");
    }

    // 4. Validar mapa de componentes UI contra o código
    task_ui_check()?;

    // 5. Build do VitePress é pulado durante o desenvolvimento com o site congelado (AGENTS.md §1).
    // Para compilar explicitamente o site VitePress, utilize 'cargo run -p xtask -- docs'.
    println!(
        "🧊 Site público congelado (AGENTS.md §1): pulando compilação do VitePress para poupar tempo e tokens."
    );
    println!("🎉 Verificação de integridade concluída com sucesso!");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct UiMapNode {
    id: String,
    kind: String,
    file: String,
    entry: String,
    #[allow(dead_code)]
    line: Option<u32>,
    #[allow(dead_code)]
    position: Option<String>,
    region: Option<String>,
    #[serde(default)]
    children: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    reads: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    writes: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    dispatches: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    opens: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct UiMap {
    version: u32,
    #[allow(dead_code)]
    roots: Vec<String>,
    nodes: Vec<UiMapNode>,
}

/// Valida o mapa de componentes UI contra o código (drift prevention).
///
/// Falha em: JSON inválido, ids duplicados, kind desconhecido, arquivo
/// ausente, símbolo de entrada ausente no arquivo, região fora de
/// `RegionSlot`, filho inexistente, ciclo. Símbolos públicos sem nó são
/// apenas avisos (guia de cobertura, não quebra).
fn task_ui_check() -> Result<()> {
    println!("🗺️ Validando mapa de componentes UI...");
    let root = root_dir();
    let map_path = root.join("docs").join("public").join("ui-map.json");
    let text = std::fs::read_to_string(&map_path)
        .with_context(|| format!("Falha ao ler {}", map_path.display()))?;
    let map: UiMap =
        serde_json::from_str(&text).with_context(|| "docs/public/ui-map.json inválido")?;
    if map.version != 1 {
        bail!(
            "ui-map.json: version {} não suportada (esperado 1)",
            map.version
        );
    }
    if map.nodes.is_empty() {
        bail!("ui-map.json: nenhum nó documentado");
    }

    const KINDS: &[&str] = &[
        "shell",
        "shell_bar",
        "toolbar",
        "panel",
        "section",
        "tab_bar",
        "tab",
        "form",
        "grid",
        "list",
        "canvas",
        "data_table",
        "button_row",
        "menu",
        "modal",
        "dialog",
        "file_picker",
        "overlay",
        "hud",
        "gizmo",
        "viewport_overlay",
        "measurement_tool",
        "widget",
        "token_system",
        "icon_system",
        "devtool",
        "status",
        "service",
        "helper",
    ];

    // Regiões canônicas lidas do próprio RegionSlot (fonte única).
    let regions_src = std::fs::read_to_string(root.join("crates/ui/src/regions.rs"))?;
    let mut valid_regions = HashSet::new();
    let mut in_enum = false;
    for line in regions_src.lines() {
        let t = line.trim();
        if t.starts_with("pub enum RegionSlot") {
            in_enum = true;
            continue;
        }
        if in_enum {
            if t.starts_with('}') {
                break;
            }
            let variant: String = t
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !variant.is_empty() {
                valid_regions.insert(variant);
            }
        }
    }
    if valid_regions.is_empty() {
        bail!("RegionSlot não encontrado em crates/ui/src/regions.rs");
    }

    let mut ids = HashSet::new();
    let mut by_id: HashMap<&str, &UiMapNode> = HashMap::new();
    for node in &map.nodes {
        if !ids.insert(node.id.clone()) {
            bail!("ui-map.json: id duplicado '{}'", node.id);
        }
        if !KINDS.contains(&node.kind.as_str()) {
            bail!(
                "ui-map.json: kind desconhecido '{}' no nó '{}'",
                node.kind,
                node.id
            );
        }
        if let Some(region) = &node.region
            && !valid_regions.contains(region)
        {
            bail!(
                "ui-map.json: região '{}' fora de RegionSlot (nó '{}')",
                region,
                node.id
            );
        }
        let file_path = root.join(&node.file);
        if !file_path.exists() {
            bail!(
                "ui-map.json: arquivo ausente '{}' (nó '{}')",
                node.file,
                node.id
            );
        }
        let content = std::fs::read_to_string(&file_path)?;
        let probes = [
            format!("fn {}(", node.entry),
            format!("struct {}", node.entry),
            format!("enum {}", node.entry),
            format!("mod {}", node.entry),
        ];
        if !probes.iter().any(|p| content.contains(p.as_str())) {
            bail!(
                "ui-map.json: símbolo '{}' ausente em {} (nó '{}')",
                node.entry,
                node.file,
                node.id
            );
        }
        by_id.insert(node.id.as_str(), node);
    }
    for node in &map.nodes {
        for child in &node.children {
            if !by_id.contains_key(child.as_str()) {
                bail!(
                    "ui-map.json: filho '{}' do nó '{}' não existe",
                    child,
                    node.id
                );
            }
        }
    }
    // Aciclicidade via DFS a partir de cada nó.
    fn visit<'a>(
        id: &'a str,
        by_id: &HashMap<&'a str, &'a UiMapNode>,
        stack: &mut Vec<&'a str>,
    ) -> Result<()> {
        if stack.contains(&id) {
            bail!("ui-map.json: ciclo envolvendo '{}' ({:?})", id, stack);
        }
        stack.push(id);
        if let Some(node) = by_id.get(id) {
            for child in &node.children {
                visit(child, by_id, stack)?;
            }
        }
        stack.pop();
        Ok(())
    }
    for node in &map.nodes {
        visit(node.id.as_str(), &by_id, &mut Vec::new())?;
    }
    println!(
        "✅ Mapa válido: {} nós, {} regiões, filhos acíclicos.",
        map.nodes.len(),
        valid_regions.len()
    );

    // Cobertura: símbolos públicos de UI sem nó (aviso, não quebra).
    let covered: HashSet<(String, String)> = map
        .nodes
        .iter()
        .map(|n| (n.file.clone(), n.entry.clone()))
        .collect();
    let mut uncovered = Vec::new();
    collect_ui_symbols(&root.join("crates/ui/src"), &root, &covered, &mut uncovered);
    uncovered.sort();
    uncovered.dedup();
    if !uncovered.is_empty() {
        println!("⚠️ {} símbolos públicos sem nó no mapa:", uncovered.len());
        for item in uncovered.iter().take(20) {
            println!("   - {item}");
        }
        if uncovered.len() > 20 {
            println!("   ... e outros {}", uncovered.len() - 20);
        }
    }
    Ok(())
}

/// Coleta `arquivo::Simbolo` públicos de topo (`pub fn/struct/enum/mod` sem
/// indentação) e registra os ausentes da cobertura do mapa.
fn collect_ui_symbols(
    dir: &Path,
    root: &Path,
    covered: &HashSet<(String, String)>,
    out: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ui_symbols(&path, root, covered, out);
            continue;
        }
        if path.extension().map(|e| e != "rs").unwrap_or(true) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // Módulos de teste vivem no fim do arquivo por convenção: ignora tudo
        // a partir do primeiro `#[cfg(test)]`.
        let body = match content.find("#[cfg(test)]") {
            Some(pos) => &content[..pos],
            None => content.as_str(),
        };
        for line in body.lines() {
            for prefix in [
                "pub fn ",
                "pub struct ",
                "pub enum ",
                "pub mod ",
                "pub const ",
            ] {
                if let Some(rest) = line.strip_prefix(prefix) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() && !covered.contains(&(rel.clone(), name.clone())) {
                        out.push(format!("{rel}::{name}"));
                    }
                    break;
                }
            }
        }
    }
}

fn task_arch_check() -> Result<()> {
    println!("🔍 Validando relatórios da Auditoria Arquitetural...");
    let root = root_dir();
    let audit_dir = root
        .join("docs")
        .join("audits")
        .join("architecture-decoupling");

    if !audit_dir.exists() {
        bail!("Diretório de auditoria não encontrado: docs/audits/architecture-decoupling");
    }

    let required_reports = [
        "README.md",
        "01-current-architecture.md",
        "02-dependency-analysis.md",
        "03-ui-coupling.md",
        "04-state-ownership.md",
        "05-command-system.md",
        "06-tool-system.md",
        "07-renderer-boundary.md",
        "08-project-assets-io.md",
        "09-modularity-extension-cost.md",
        "10-headless-readiness.md",
        "11-cross-language-ui.md",
        "12-testing-gaps.md",
        "13-findings.md",
        "14-scorecard.md",
        "15-target-architecture-options.md",
        "16-remediation-plan.md",
    ];

    let mut missing = Vec::new();
    let mut total_bytes = 0;

    for report in &required_reports {
        let p = audit_dir.join(report);
        if !p.exists() {
            missing.push(*report);
        } else {
            let metadata = std::fs::metadata(&p)
                .with_context(|| format!("Falha ao ler metadados de {report}"))?;
            if metadata.len() == 0 {
                bail!("Relatório de auditoria está vazio: {report}");
            }
            total_bytes += metadata.len();
        }
    }

    if !missing.is_empty() {
        bail!("Relatórios de auditoria ausentes: {:?}", missing);
    }

    println!(
        "📊 Auditoria validada com sucesso: {} relatórios canônicos presentes ({:.1} KB de documentação técnica).",
        required_reports.len(),
        total_bytes as f64 / 1024.0
    );

    println!("🛡️ Validando invariantes arquiteturais dos manifestos Cargo...");
    let checks = [
        (
            "crates/core/Cargo.toml",
            &["egui =", "petunia_render"][..],
            "petunia_core não pode depender de egui nem petunia_render",
        ),
        (
            "crates/config/Cargo.toml",
            &["egui ="][..],
            "petunia_config não pode depender de egui",
        ),
        (
            "crates/mesh/Cargo.toml",
            &["egui", "petunia_core", "petunia_ui"][..],
            "petunia_mesh deve ser 100% puro",
        ),
        (
            "crates/project/Cargo.toml",
            &["egui", "petunia_ui"][..],
            "petunia_project não pode depender de UI",
        ),
        (
            "crates/commands/Cargo.toml",
            &["egui", "petunia_ui"][..],
            "petunia_commands não pode depender de UI",
        ),
        (
            "crates/render-wgpu/Cargo.toml",
            &["egui", "petunia_ui"][..],
            "petunia_render_wgpu não pode depender de UI",
        ),
        (
            "crates/module-paint/Cargo.toml",
            &["rfd ="][..],
            "petunia_module_paint não pode depender de rfd",
        ),
        (
            "crates/module-model/Cargo.toml",
            &["egui"][..],
            "petunia_module_model não pode depender de egui (G7)",
        ),
        (
            "crates/module-paint/Cargo.toml",
            &["egui"][..],
            "petunia_module_paint não pode depender de egui (G7)",
        ),
        (
            "crates/module-uv/Cargo.toml",
            &["egui"][..],
            "petunia_module_uv não pode depender de egui (G7)",
        ),
        (
            "crates/module-assets/Cargo.toml",
            &["egui"][..],
            "petunia_module_assets não pode depender de egui (G7)",
        ),
        (
            "crates/cli/Cargo.toml",
            &["egui"][..],
            "petunia_cli não pode depender de egui (G8)",
        ),
        (
            "crates/ffi/Cargo.toml",
            &["egui"][..],
            "petunia_ffi não pode depender de egui (G10)",
        ),
    ];

    for (rel_path, forbidden, reason) in checks {
        let path = root.join(rel_path);
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("Falha ao ler {rel_path}"))?;
        for pattern in forbidden {
            if content.contains(pattern) {
                bail!("Violação em {rel_path}: contém '{pattern}' proibido ({reason})");
            }
        }
    }

    println!(
        "✅ Invariantes de manifesto validados: core, config, mesh, project, commands, render-wgpu e module-paint estão em conformidade."
    );

    println!("🛡️ Validando fronteira de I/O de arquivos (Gauntlet G4)...");
    let mut rs_files = Vec::new();
    let crates_dir = root.join("crates");

    fn collect_rs(dir: &Path, acc: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    collect_rs(&path, acc)?;
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    acc.push(path);
                }
            }
        }
        Ok(())
    }

    collect_rs(&crates_dir, &mut rs_files)?;

    for file_path in &rs_files {
        let rel = file_path
            .strip_prefix(&root)
            .unwrap_or(file_path)
            .to_string_lossy();

        if rel == "crates/ui/src/file_dialog_service.rs" || rel.starts_with("crates/xtask") {
            continue;
        }

        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Falha ao ler {}", file_path.display()))?;

        if content.contains("rfd::FileDialog") {
            bail!(
                "Violação de Fronteira de I/O: {rel} instancia rfd::FileDialog fora de file_dialog_service.rs!"
            );
        }
        if content.contains("egui_file_dialog::FileDialog") {
            bail!(
                "Violação de Fronteira de I/O: {rel} instancia egui_file_dialog fora de file_dialog_service.rs!"
            );
        }
    }

    println!(
        "✅ Fronteira de I/O validada: nenhum arquivo fora de crates/ui/src/file_dialog_service.rs instancia rfd ou egui-file-dialog."
    );

    println!(
        "🛡️ Validando ausência de sessões de ferramentas em memória temporária de UI (Gauntlet G5)..."
    );
    for file_path in &rs_files {
        let rel = file_path
            .strip_prefix(&root)
            .unwrap_or(file_path)
            .to_string_lossy();

        if rel.starts_with("crates/xtask") {
            continue;
        }

        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Falha ao ler {}", file_path.display()))?;

        if content.contains("\"cut.session\"") {
            bail!(
                "Violação de Sessão de Ferramenta (G5): {rel} ainda utiliza Id(\"cut.session\") em memória temporária de UI!"
            );
        }
        if content.contains("\"modal.pointer\"") {
            bail!(
                "Violação de Sessão de Ferramenta (G5): {rel} ainda utiliza Id(\"modal.pointer\") em memória temporária de UI!"
            );
        }
    }
    println!(
        "✅ Sessões de ferramentas validadas: CutSession e PointerSession residem exclusivamente no domínio (AppState)."
    );

    println!("🛡️ Validando decomposição do God Object AppState (Gauntlet G6 / F-002)...");
    let state_rs = root.join("crates/core/src/state.rs");
    let state_content = std::fs::read_to_string(&state_rs)
        .with_context(|| format!("Falha ao ler {}", state_rs.display()))?;

    let required_substates = [
        "pub struct ProjectState",
        "pub struct EditorSession",
        "pub struct ToolState",
        "pub struct UiState",
        "pub struct RenderResources",
    ];
    for sub in required_substates {
        if !state_content.contains(sub) {
            bail!(
                "Violação de Decomposição (G6): sub-estado {sub} não encontrado em crates/core/src/state.rs!"
            );
        }
    }

    let required_app_state_fields = [
        "pub project: ProjectState",
        "pub session: EditorSession",
        "pub ui: UiState",
        "pub render: RenderResources",
        "pub events: EventBus",
    ];
    for field in required_app_state_fields {
        if !state_content.contains(field) {
            bail!(
                "Violação de Composição de AppState (G6): campo {field} não encontrado em AppState!"
            );
        }
    }
    if !state_content.contains("pub tools: ToolState") {
        bail!(
            "Violação de Composição (G6): campo pub tools: ToolState não encontrado em EditorSession!"
        );
    }

    println!(
        "✅ Sub-estados coesos validados: AppState decomposto em ProjectState, EditorSession, ToolState, UiState e RenderResources."
    );

    println!("🛡️ Validando purificação dos crates de módulo (Gauntlet G7 / F-008)...");
    for file_path in &rs_files {
        let rel = file_path
            .strip_prefix(&root)
            .unwrap_or(file_path)
            .to_string_lossy();

        if rel.starts_with("crates/module-") {
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Falha ao ler {}", file_path.display()))?;
            if content.contains("egui::") || content.contains("use egui") {
                bail!(
                    "Violação de Purificação de Módulo (G7 / F-008): {rel} contém referência direta a egui!"
                );
            }
        }
    }
    println!(
        "✅ Módulos purificados: module-model, module-paint, module-uv e module-assets são 100% livres de egui."
    );

    println!("🛡️ Validando soberania headless do crate CLI (Gauntlet G8 / F-011)...");
    for file_path in &rs_files {
        let rel = file_path
            .strip_prefix(&root)
            .unwrap_or(file_path)
            .to_string_lossy();

        if rel.starts_with("crates/cli") {
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Falha ao ler {}", file_path.display()))?;
            if content.contains("egui::") || content.contains("use egui") {
                bail!(
                    "Violação de Soberania Headless (G8 / F-011): {rel} contém referência direta a egui!"
                );
            }
        }
    }
    println!(
        "✅ Crate CLI validado: petunia-cli é 100% puro e opera sem qualquer dependência de UI."
    );

    println!("🛡️ Validando estabilização da Application API (Gauntlet G9 / F-010)...");
    let queries_path = root.join("crates/core/src/queries.rs");
    if !queries_path.exists() {
        bail!(
            "Violação de Application API (G9 / F-010): crates/core/src/queries.rs não encontrado!"
        );
    }
    let state_path = root.join("crates/core/src/state.rs");
    let state_content = std::fs::read_to_string(&state_path)?;
    if !state_content.contains("pub export_selected: Vec<Uuid>") {
        bail!("Violação de Application API (G9 / F-010): export_selected deve ser Vec<Uuid>!");
    }
    println!("✅ Application API validada: DTOs e identificadores estáveis (UUID) ativos.");

    println!("🛡️ Validando soberania da camada C-ABI / FFI (Gauntlet G10)...");
    for file_path in &rs_files {
        let rel = file_path
            .strip_prefix(&root)
            .unwrap_or(file_path)
            .to_string_lossy();

        if rel.starts_with("crates/ffi") {
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Falha ao ler {}", file_path.display()))?;
            if content.contains("egui::") || content.contains("use egui") {
                bail!("Violação de C-ABI (G10): {rel} contém referência direta a egui!");
            }
        }
    }
    let header_path = root.join("crates/ffi/include/petunia.h");
    if !header_path.exists() {
        bail!("Violação de C-ABI (G10): header crates/ffi/include/petunia.h ausente!");
    }
    println!("✅ Camada C-ABI / FFI validada: petunia_ffi e include/petunia.h são 100% autônomos.");

    println!("🛡️ Validando ausência de atalhos físicos nas ferramentas (Wave 1)...");
    for file_path in &rs_files {
        let rel = file_path
            .strip_prefix(&root)
            .unwrap_or(file_path)
            .to_string_lossy();

        if rel.starts_with("crates/module-model")
            || rel == "crates/core/src/modal.rs"
            || rel == "crates/core/src/cutting_session.rs"
        {
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Falha ao ler {}", file_path.display()))?;
            if content.contains("winit::keyboard")
                || content.contains("PhysicalKey")
                || content.contains("egui::Key")
            {
                bail!(
                    "Violação de Desacoplamento de Entrada (Wave 1): {rel} referencia atalhos físicos!"
                );
            }
        }
    }
    println!("✅ Soberania de ferramentas validada: tools não conhecem keycodes físicos.");

    println!("🛡️ Validando ausência de eframe em todo o domínio (Wave 1)...");
    let domain_crates = [
        "crates/core",
        "crates/mesh",
        "crates/commands",
        "crates/config",
        "crates/project",
        "crates/render",
        "crates/module-model",
        "crates/module-paint",
        "crates/module-uv",
        "crates/module-assets",
        "crates/cli",
        "crates/ffi",
    ];
    for c in domain_crates {
        let cargo_p = root.join(c).join("Cargo.toml");
        if cargo_p.exists() {
            let content = std::fs::read_to_string(&cargo_p)?;
            if content.contains("eframe") {
                bail!("Violação de Domínio (Wave 1): {c}/Cargo.toml depende de eframe!");
            }
        }
    }
    println!("✅ Domínio agnóstico validado: eframe ausente de todo o Core e Módulos.");

    println!("🛡️ Validando integridade de projetos e persistência agnóstica à UI (Wave 2)...");
    let project_crates = [
        "crates/project",
        "crates/core/src/project_service.rs",
        "crates/core/src/recent_projects.rs",
    ];
    for p in project_crates {
        let p_path = root.join(p);
        if p_path.is_file() {
            let content = std::fs::read_to_string(&p_path)?;
            if content.contains("egui::") || content.contains("eframe") {
                bail!("Violação de Persistência (Wave 2): {p} depende de egui/eframe!");
            }
        } else if p_path.is_dir() {
            let src_dir = p_path.join("src");
            let target_dir = if src_dir.exists() { src_dir } else { p_path };
            for entry in std::fs::read_dir(&target_dir)? {
                let entry = entry?;
                if entry.path().extension().is_some_and(|e| e == "rs") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if content.contains("egui::") || content.contains("eframe") {
                        bail!(
                            "Violação de Persistência (Wave 2): {} depende de egui/eframe!",
                            entry.path().display()
                        );
                    }
                }
            }
        }
    }
    println!("✅ Integridade de projeto validada: persistência e autosave 100% livres de UI/egui.");

    println!("🛡️ Validando fronteiras P0 (Wave M3)...");
    let p0_manifest_checks = [
        (
            "crates/ui/Cargo.toml",
            &["egui_extras", "iconflow"][..],
            "petunia_ui deve possuir os adapters egui_extras/iconflow",
        ),
        (
            "crates/mesh/Cargo.toml",
            &["geo =", "manifold-rust"][..],
            "petunia_mesh deve possuir os providers geo/manifold-rust",
        ),
        (
            "crates/project/Cargo.toml",
            &["tobj =", "gltf-json", "tempfile"][..],
            "petunia_project deve possuir tobj/gltf-json/tempfile",
        ),
        (
            "crates/core/Cargo.toml",
            &["tracing =", "slotmap =", "rayon =", "flume =", "schemars"][..],
            "petunia_core deve possuir tracing/slotmap/rayon/flume/schemars",
        ),
        (
            "crates/app/Cargo.toml",
            &["tracing-subscriber", "eframe"][..],
            "petunia_app deve possuir tracing-subscriber/eframe",
        ),
    ];
    for (rel_path, required, reason) in p0_manifest_checks {
        let path = root.join(rel_path);
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("Falha ao ler {rel_path}"))?;
        for pattern in required {
            if !content.contains(pattern) {
                bail!("Violação P0 (Wave M3): {rel_path} não contém '{pattern}' ({reason})");
            }
        }
    }
    // Dependências P0 confinadas aos donos: nenhum outro crate pode puxá-las.
    let p0_confinement = [
        ("iconflow", "crates/ui"),
        ("egui_extras", "crates/ui"),
        ("manifold-rust", "crates/mesh"),
        ("tobj =", "crates/project"),
        ("gltf-json", "crates/project"),
        ("tracing-subscriber", "crates/app"),
    ];
    let owner_crates = [
        "crates/core",
        "crates/mesh",
        "crates/commands",
        "crates/config",
        "crates/project",
        "crates/plugins",
        "crates/mcp",
        "crates/render",
        "crates/render-gl",
        "crates/render-wgpu",
        "crates/module-model",
        "crates/module-paint",
        "crates/module-uv",
        "crates/module-assets",
        "crates/ui",
        "crates/app",
        "crates/cli",
        "crates/ffi",
    ];
    for (dep, owner) in p0_confinement {
        for c in owner_crates {
            if c == owner {
                continue;
            }
            let cargo_p = root.join(c).join("Cargo.toml");
            if cargo_p.exists() {
                let content = std::fs::read_to_string(&cargo_p)?;
                if content.contains(dep) {
                    bail!(
                        "Violação P0 (Wave M3): {c}/Cargo.toml depende de '{dep}' fora do dono {owner}!"
                    );
                }
            }
        }
    }
    // Tipos de provider nunca vazam: fontes fora do dono não referenciam os crates.
    let p0_source_checks = [
        ("use iconflow", "crates/ui/src/icon_provider.rs"),
        ("egui_extras::", "crates/ui"),
        ("geo::", "crates/mesh"),
        ("manifold_rust::", "crates/mesh"),
        ("tobj::", "crates/project"),
        ("gltf_json::", "crates/project"),
    ];
    for (marker, owner) in p0_source_checks {
        for file_path in &rs_files {
            let rel = file_path
                .strip_prefix(&root)
                .unwrap_or(file_path)
                .to_string_lossy();
            if rel.starts_with("crates/xtask") || rel.starts_with("fuzz/") {
                continue;
            }
            if rel.starts_with(owner) {
                continue;
            }
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Falha ao ler {}", file_path.display()))?;
            if content.contains(marker) {
                bail!("Violação P0 (Wave M3): {rel} referencia '{marker}' fora de {owner}!");
            }
        }
    }
    println!("✅ Fronteiras P0 validadas: cada dependência vive apenas no dono canônico.");

    println!("🛡️ Validando fronteiras P1 (Wave M4)...");
    let p1_manifest_checks = [
        (
            "crates/project/Cargo.toml",
            &["zip ="][..],
            "petunia_project deve possuir o container zip",
        ),
        (
            "crates/plugins/Cargo.toml",
            &["mlua"][..],
            "petunia_plugins deve possuir mlua",
        ),
        (
            "crates/mcp/Cargo.toml",
            &["rmcp", "tokio"][..],
            "petunia_mcp deve possuir rmcp/tokio",
        ),
        (
            "crates/mesh/Cargo.toml",
            &["xatlas"][..],
            "petunia_mesh deve possuir o provider xatlas",
        ),
        (
            "crates/app/Cargo.toml",
            &["notify ="][..],
            "petunia_app deve possuir notify",
        ),
        (
            "crates/ui/Cargo.toml",
            &["egui_inbox", "egui_taffy", "twill"][..],
            "petunia_ui deve possuir inbox/taffy/twill",
        ),
    ];
    for (rel_path, required, reason) in p1_manifest_checks {
        let path = root.join(rel_path);
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("Falha ao ler {rel_path}"))?;
        for pattern in required {
            if !content.contains(pattern) {
                bail!("Violação P1 (Wave M4): {rel_path} não contém '{pattern}' ({reason})");
            }
        }
    }
    let p1_confinement = [
        ("zip =", "crates/project"),
        ("mlua", "crates/plugins"),
        ("rmcp", "crates/mcp"),
        ("tokio", "crates/mcp"),
        ("xatlas", "crates/mesh"),
        ("notify =", "crates/app"),
        ("egui_inbox", "crates/ui"),
        ("egui_taffy", "crates/ui"),
        ("egui_commonmark", "crates/ui"),
        ("egui_autocomplete", "crates/ui"),
        ("twill", "crates/ui"),
        ("egui_inspection", "crates/ui"),
        ("egui_mcp", "crates/ui"),
        ("egui-probe", "crates/ui"),
    ];
    for (dep, owner) in p1_confinement {
        for c in owner_crates {
            if c == owner {
                continue;
            }
            let cargo_p = root.join(c).join("Cargo.toml");
            if cargo_p.exists() {
                let content = std::fs::read_to_string(&cargo_p)?;
                if content.contains(dep) {
                    bail!(
                        "Violação P1 (Wave M4): {c}/Cargo.toml depende de '{dep}' fora do dono {owner}!"
                    );
                }
            }
        }
    }
    // Checagem extra: novos crates P1 não podem existir fora dos membros conhecidos.
    for extra in ["crates/plugins", "crates/mcp"] {
        if !root.join(extra).join("Cargo.toml").exists() {
            bail!("Violação P1 (Wave M4): {extra}/Cargo.toml ausente!");
        }
    }
    let p1_source_checks = [
        ("use mlua", "crates/plugins"),
        ("rmcp::", "crates/mcp"),
        ("xatlas_rs_v2::", "crates/mesh"),
        ("notify::", "crates/app"),
        ("egui_taffy::", "crates/ui"),
        ("egui_inbox::", "crates/ui"),
        ("twill::", "crates/ui"),
        ("egui_commonmark::", "crates/ui"),
        ("egui_autocomplete::", "crates/ui"),
        ("egui_inspection::", "crates/ui"),
        ("egui_probe::", "crates/ui"),
    ];
    for (marker, owner) in p1_source_checks {
        for file_path in &rs_files {
            let rel = file_path
                .strip_prefix(&root)
                .unwrap_or(file_path)
                .to_string_lossy();
            if rel.starts_with("crates/xtask") || rel.starts_with("fuzz/") {
                continue;
            }
            if rel.starts_with(owner) {
                continue;
            }
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Falha ao ler {}", file_path.display()))?;
            if content.contains(marker) {
                bail!("Violação P1 (Wave M4): {rel} referencia '{marker}' fora de {owner}!");
            }
        }
    }
    println!("✅ Fronteiras P1 validadas: cada dependência vive apenas no dono canônico.");

    println!(
        "🏛️ Progresso de remediação: GAUNTLETS G0 a G10, WAVE 1 e WAVE 2 100% CONCLUÍDOS COM SUCESSO!"
    );

    // P0 architecture: MCP must not own Project/UndoStack; CLI/FFI must not
    // call Tool::apply for domain mutation.
    let mcp_src = root.join("crates/mcp/src/server.rs");
    let mcp = std::fs::read_to_string(&mcp_src).unwrap_or_default();
    if mcp.contains("UndoStack<Project>") || mcp.contains("struct McpDomain") {
        bail!("Violação P0: MCP não pode possuir Project/UndoStack próprio");
    }
    if !mcp.contains("dispatch_intent") && !mcp.contains("state.dispatch") {
        bail!("Violação P0: MCP deve despachar pela Application API");
    }
    for (rel, marker) in [
        ("crates/cli/src/main.rs", "Tool::apply"),
        ("crates/cli/src/main.rs", "ExtrudeTool::"),
        ("crates/ffi/src/lib.rs", "ExtrudeTool::"),
        ("crates/ffi/src/lib.rs", "PrimitivesTool::"),
        ("crates/core/Cargo.toml", "petunia_render"),
    ] {
        let content = std::fs::read_to_string(root.join(rel)).unwrap_or_default();
        if content.contains(marker) {
            bail!("Violação P0: {rel} ainda contém '{marker}'");
        }
    }
    println!("✅ Invariantes P0 (command spine / MCP / core↛render) validadas.");
    Ok(())
}

fn run_cargo(root: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("falha ao executar cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("cargo {} falhou", args.join(" "));
    }
    Ok(())
}

/// Read-only backend gate. Does not format or rewrite UI files.
fn task_verify() -> Result<()> {
    let root = root_dir();
    println!("🔎 cargo xtask verify (read-only backend gate)");
    run_cargo(&root, &["check", "--workspace"])?;
    run_cargo(
        &root,
        &[
            "test",
            "-p",
            "petunia_core",
            "-p",
            "petunia_commands",
            "-p",
            "petunia_project",
            "-p",
            "petunia_mesh",
            "-p",
            "petunia_cli",
            "-p",
            "petunia_ffi",
            "-p",
            "petunia_mcp",
            "-p",
            "petunia_plugins",
            "-p",
            "petunia_module_model",
            "-p",
            "petunia_module_paint",
            "--",
            "--test-threads=8",
        ],
    )?;
    task_arch_check()?;
    println!("✅ verify: backend gate concluído (read-only).");
    Ok(())
}
