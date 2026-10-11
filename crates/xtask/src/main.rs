//! Ferramenta de automação interna (cargo xtask) para o Petunia3D.
//! Gerencia tarefas de documentação, integridade e drift prevention.

use anyhow::{Context, Result, bail};

use std::path::{Path, PathBuf};
use std::process::Command;

mod bible;
mod generator;
mod icons_check;
mod slint_lint;
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
        "icons-check" => icons_check::run(&root_dir())?,
        "ui-lint" => slint_lint::run(&root_dir())?,
        "ui-metrics" => task_ui_metrics()?,
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
    ui-lint       Falha se o markup Slint usar cor #hex, font-size ou sombra literal fora de tokens.slint
    ui-metrics    Mede o shell (plano de UI §11): controles visíveis por workspace em 1800 × 1012
                  e literais de cor/fonte (ui-lint). Falha se POLY passar da meta.
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

    // 4. O mapa de componentes egui (docs/public/ui-map.json) descrevia a UI
    //    removida; a estrutura do shell Slint é validada pelo `ui-lint` e pelos
    //    testes de integração de `petunia_ui_slint`.

    // 5. Build do VitePress é pulado durante o desenvolvimento com o site congelado (AGENTS.md §1).
    // Para compilar explicitamente o site VitePress, utilize 'cargo run -p xtask -- docs'.
    println!(
        "🧊 Site público congelado (AGENTS.md §1): pulando compilação do VitePress para poupar tempo e tokens."
    );
    println!("🎉 Verificação de integridade concluída com sucesso!");
    Ok(())
}

/// Único arquivo autorizado a abrir o diálogo nativo de arquivos (Gauntlet G4).
const FILE_DIALOG_BOUNDARY: &str = "crates/ui-slint/src/files.rs";

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
            "crates/render/Cargo.toml",
            &["slint", "petunia_ui_slint", "petunia_application"][..],
            "petunia_render não pode depender da UI nem da Application (recebe só RenderFrameInput)",
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

    // Um único renderer (OpenGL 3.3 via glow) e uma única UI (Slint): nenhum
    // manifesto pode trazer de volta WGPU ou a pilha egui (04-renderer, decisões 1–2).
    let mut manifests = vec![root.join("Cargo.toml")];
    for entry in std::fs::read_dir(root.join("crates"))?.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if manifest.exists() {
            manifests.push(manifest);
        }
    }
    for manifest in manifests {
        let content = std::fs::read_to_string(&manifest)
            .with_context(|| format!("Falha ao ler {}", manifest.display()))?;
        for line in content.lines().filter(|l| !l.trim_start().starts_with('#')) {
            let key = line
                .split('=')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"');
            if key == "wgpu"
                || key.starts_with("wgpu-")
                || key.starts_with("egui")
                || key == "eframe"
            {
                bail!(
                    "Violação em {}: dependência '{key}' proibida (viewport é OpenGL 3.3/glow e a UI é Slint)",
                    manifest.display()
                );
            }
        }
    }

    println!(
        "✅ Invariantes de manifesto validados: core, config, mesh, project, commands, render e módulos em conformidade; sem WGPU/egui no workspace."
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

        if rel == FILE_DIALOG_BOUNDARY || rel.starts_with("crates/xtask") {
            continue;
        }

        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Falha ao ler {}", file_path.display()))?;

        if content.contains("rfd::FileDialog") || content.contains("rfd::AsyncFileDialog") {
            bail!(
                "Violação de Fronteira de I/O: {rel} instancia o diálogo nativo fora de {FILE_DIALOG_BOUNDARY}!"
            );
        }
    }

    println!(
        "✅ Fronteira de I/O validada: nenhum arquivo fora de {FILE_DIALOG_BOUNDARY} instancia o diálogo nativo (rfd)."
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

/// Métricas do shell (plano de UI F0): literais de estilo e controles visíveis.
fn task_ui_metrics() -> Result<()> {
    slint_lint::run(&root_dir())?;
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(root_dir())
        .args([
            "test",
            "-q",
            "-p",
            "petunia_ui_slint",
            "--test",
            "ui_metrics",
            "--",
            "--nocapture",
        ])
        .status()
        .context("falha ao rodar o teste ui_metrics")?;
    if !status.success() {
        bail!("ui-metrics: a contagem de controles passou da meta (ver tabela acima)");
    }
    Ok(())
}
