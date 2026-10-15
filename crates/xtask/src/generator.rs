//! Gerador determinístico de catálogos e referências técnicas (P3D-119).
//! Extrai metadados das fontes canônicas de código (Core, Config, Project, UI, Locales)
//! e produz arquivos Markdown em `docs/generated/`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use petunia_config::keybinds::Keybinds;
use petunia_config::theme::{Theme, ThemeToken};
use petunia_core::command::CommandDispatcher;
use petunia_project::pipeline::{DeliveryPipeline, FileFormat};

/// Arquivos gerados e seus conteúdos produzidos pelo gerador determinístico.
pub struct GeneratedCatalog {
    pub commands_md: String,
    pub keybinds_md: String,
    pub cheatsheet_md: String,
    pub icon_tokens_md: String,
    pub text_tokens_md: String,
    pub theme_tokens_md: String,
    pub supported_formats_md: String,
    pub index_md: String,
    pub manifest_json: String,
}

impl GeneratedCatalog {
    pub fn generate(root_dir: &Path) -> Result<Self> {
        let commands_md = generate_commands_md()?;
        let keybinds_md = generate_keybinds_md(root_dir)?;
        let cheatsheet_md = generate_cheatsheet_md()?;
        let icon_tokens_md = generate_icon_tokens_md(root_dir)?;
        let text_tokens_md = generate_text_tokens_md(root_dir)?;
        let theme_tokens_md = generate_theme_tokens_md()?;
        let supported_formats_md = generate_supported_formats_md()?;
        let index_md = generate_index_md();

        let manifest_files = [
            ("COMMANDS.md", commands_md.len()),
            ("KEYBINDS.md", keybinds_md.len()),
            ("ICON_TOKENS.md", icon_tokens_md.len()),
            ("TEXT_TOKENS.md", text_tokens_md.len()),
            ("THEME_TOKENS.md", theme_tokens_md.len()),
            ("SUPPORTED_FORMATS.md", supported_formats_md.len()),
            ("index.md", index_md.len()),
        ];

        let manifest_json = generate_manifest_json(&manifest_files)?;

        Ok(Self {
            commands_md,
            keybinds_md,
            cheatsheet_md,
            icon_tokens_md,
            text_tokens_md,
            theme_tokens_md,
            supported_formats_md,
            index_md,
            manifest_json,
        })
    }
}

const AUTOGEN_HEADER: &str = r#"<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->"#;

fn generate_commands_md() -> Result<String> {
    let dispatcher = CommandDispatcher::canonical();
    let mut all_meta = dispatcher.all_metadata();
    all_meta.sort_by(|a, b| {
        a.category
            .as_str()
            .cmp(b.category.as_str())
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Catálogo de Comandos & Ferramentas\n");
    out.push_str(
        "description: Catálogo canônico de comandos gerados a partir do CommandDispatcher (P3D-119)\n",
    );
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Catálogo Canônico de Comandos e Ações (`CommandId`)\n\n");
    out.push_str("> **Single Source of Truth (P3D-100, P3D-119)**\n");
    out.push_str(
        "> Todos os comandos do Petunia3D são registrados centralmente no `CommandDispatcher`, ",
    );
    out.push_str("permitindo despacho transacional com histórico (Undo/Redo), Command Palette e telemetria.\n\n");

    out.push_str(&format!(
        "Total de comandos registrados no motor: **{}**.\n\n",
        all_meta.len()
    ));

    out.push_str("## Tabela Geral de Comandos\n\n");
    out.push_str(
        "| ID (`CommandId`) | Rótulo | Categoria | Destrutivo | Tópico Docs | Descrição |\n",
    );
    out.push_str("| :--- | :--- | :---: | :---: | :--- | :--- |\n");

    for meta in &all_meta {
        let is_destr = dispatcher
            .get(&meta.id)
            .map(|c| if c.is_destructive() { "Sim" } else { "Não" })
            .unwrap_or("—");
        let topic = meta
            .docs_topic
            .map(|t| format!("{t:?}"))
            .unwrap_or_else(|| "—".into());

        out.push_str(&format!(
            "| `{}` | **{}** | `{}` | {} | {} | {} |\n",
            meta.id,
            meta.label,
            meta.category.as_str(),
            is_destr,
            topic,
            meta.description
        ));
    }

    out.push('\n');
    Ok(out)
}

fn generate_keybinds_md(root_dir: &Path) -> Result<String> {
    let profiles = Keybinds::available_profiles();
    // O preset canônico vive em `assets/keymaps/petunia-default.toml`; o fallback
    // interno (`Keybinds::default()`) pode estar vazio, então carregamos o perfil
    // do disco para que o catálogo nunca publique uma tabela canônica vazia (P3D-119).
    let mut default_bindings = Keybinds::default().all_bindings();
    if default_bindings.is_empty() {
        default_bindings = Keybinds::load_profile("petunia-default").all_bindings();
    }

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Catálogo de Perfis e Atalhos de Teclado\n");
    out.push_str("description: Referência canônica dos perfis de teclado e atalhos configuráveis (P3D-119)\n");
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Catálogo de Perfis de Atalhos de Teclado (`Keybinds`)\n\n");
    out.push_str("> **Single Source of Truth (P3D-087, P3D-119)**\n");
    out.push_str("> O Petunia3D suporta múltiplos perfis de teclado canônicos e remapeamento via arquivos TOML.\n\n");

    out.push_str("## Perfis Canônicos Disponíveis\n\n");
    out.push_str("| ID do Perfil | Nome Amigável | Descrição |\n");
    out.push_str("| :--- | :--- | :--- |\n");
    for p in &profiles {
        out.push_str(&format!(
            "| `{}` | **{}** | {} |\n",
            p.id, p.name, p.description
        ));
    }
    out.push_str("\n\n");

    out.push_str("## Mapeamento Canônico Padrão (`petunia-default`)\n\n");
    out.push_str("| Ação Semântica | Atalho Padrão |\n");
    out.push_str("| :--- | :---: |\n");
    for (action, shortcut) in default_bindings {
        out.push_str(&format!("| `{action}` | <kbd>{shortcut}</kbd> |\n"));
    }
    out.push_str("\n\n");

    // Verificar se há perfis adicionais em assets/keymaps/*.toml
    let keymaps_dir = root_dir.join("assets").join("keymaps");
    if keymaps_dir.exists() {
        out.push_str("## Perfis Especializados em Disco\n\n");
        let mut profile_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&keymaps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    profile_files.push(stem.to_string());
                }
            }
        }
        profile_files.sort();
        for id in profile_files {
            // `petunia-default` já é publicado como tabela canônica acima.
            if id == "petunia-default" {
                continue;
            }
            let loaded = Keybinds::load_profile(&id);
            let bindings = loaded.all_bindings();
            out.push_str(&format!(
                "### Perfil: `{id}` ({} atalhos)\n\n",
                bindings.len()
            ));
            out.push_str("| Ação | Atalho |\n");
            out.push_str("| :--- | :---: |\n");
            for (action, shortcut) in bindings {
                out.push_str(&format!("| `{action}` | <kbd>{shortcut}</kbd> |\n"));
            }
            out.push('\n');
        }
    }

    Ok(out)
}

/// Gera o cheatsheet de atalhos a partir do perfil canônico `petunia-default`.
///
/// O arquivo deixa de ser curado à mão: assim ele não pode divergir do keymap
/// canônico (P3D-090/P3D-119) e a CI detecta drift.
pub fn generate_cheatsheet_md() -> Result<String> {
    let mut bindings = Keybinds::load_profile("petunia-default").all_bindings();
    if bindings.is_empty() {
        bindings = Keybinds::default().all_bindings();
    }

    let categories = [
        ("global", "Sistema & Arquivos"),
        ("select", "Seleção"),
        ("model", "Modelagem & Transformação"),
        ("paint", "Pintura"),
        ("uv", "UV"),
        ("view", "Visualização & Câmera"),
    ];

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Cheatsheet de Atalhos (Perfil Petunia)\n");
    out.push_str("description: Atalhos do perfil canônico Petunia, gerados a partir do keymap (P3D-090, P3D-119)\n");
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Cheatsheet de Atalhos — perfil `petunia-default`\n\n");
    out.push_str(
        "> **Perfil canônico:** `Petunia` é o preset default do produto. Os demais presets\n\
         > oficiais são `Petunia Simple`, `Petunia Notebook`, `Blender-like`,\n\
         > `Blender-like Notebook`, `Maya-like`, `3ds Max-like` e `Cinema 4D-like`.\n\
         > A tabela completa de todos os perfis está em\n\
         > [Catálogo de Perfis e Atalhos](../generated/KEYBINDS.md).\n\n",
    );
    out.push_str(
        "> Nenhuma ferramenta depende de tecla física como regra de negócio: os binds\n\
         > apontam para `CommandId` e podem ser remapeados por perfil, com detecção de\n\
         > conflito e import/export em JSON versionado.\n\n",
    );

    for (prefix, label) in categories {
        let entries: Vec<&(String, String)> = bindings
            .iter()
            .filter(|(action, _)| action.starts_with(&format!("{prefix}.")))
            .collect();
        if entries.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "## {label}\n\n| Ação | Atalho |\n| :--- | :---: |\n"
        ));
        for (action, shortcut) in entries {
            out.push_str(&format!("| `{action}` | <kbd>{shortcut}</kbd> |\n"));
        }
        out.push('\n');
    }

    out.push_str(
        "## Navegação e foco (contrato de input)\n\n\
         Estas entradas são contrato da UI Baseline V1, não binds de keymap:\n\n\
         | Entrada | Ação |\n| :--- | :--- |\n\
         | `LMB` | selecionar/operar |\n\
         | `Shift + LMB` | adicionar/alternar seleção |\n\
         | `RMB` | context menu |\n\
         | `MMB` | orbit |\n\
         | `Shift + MMB` | pan |\n\
         | wheel/pinch | zoom |\n\
         | `Esc` | cancelar |\n\
         | `Enter` | confirmar operação pendente |\n\
         | `F6` / `Shift+F6` | navegar regiões principais |\n\
         | `Tab` / `Shift+Tab` | navegar controles dentro da região |\n\n",
    );

    Ok(out)
}

/// Catálogo de ícones a partir do global `PetuniaIcons` do shell Slint (fonte
/// única: cada propriedade aponta para o SVG do pack, cuja pasta é o grupo).
fn generate_icon_tokens_md(root_dir: &Path) -> Result<String> {
    let icon_catalog_path = root_dir.join("crates/ui-slint/ui/petunia_icons.slint");
    if !icon_catalog_path.exists() {
        return Ok(String::new());
    }
    let content = std::fs::read_to_string(&icon_catalog_path)
        .with_context(|| format!("Falha ao ler {}", icon_catalog_path.display()))?;

    let mut tokens: Vec<(String, String, String)> = Vec::new();
    for line in content.lines() {
        let Some(rest) = line.trim().strip_prefix("out property <image> ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if name.is_empty() {
            continue;
        }
        let group = rest
            .split("petunia-outline/svg/")
            .nth(1)
            .and_then(|path| path.split('/').next())
            .filter(|group| !group.ends_with(".svg"))
            .unwrap_or("geral")
            .to_string();
        tokens.push((name.clone(), name.replace('-', "_"), group));
    }

    tokens.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Tokens de Ícones Semânticos\n");
    out.push_str("description: Catálogo canônico de identificadores de ícones IconId (P3D-119)\n");
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Catálogo Canônico de Tokens de Ícones (`IconId`)\n\n");
    out.push_str("> **Single Source of Truth (P3D-086, P3D-119)**\n");
    out.push_str(
        "> Ícones no Petunia3D são endereçados por tokens semânticos do global Slint `PetuniaIcons` ",
    );
    out.push_str(
        "(variantes outline/filled trocadas em tempo de execução, sem afetar a lógica).\n\n",
    );

    out.push_str(&format!(
        "Total de tokens declarados em `PetuniaIcons`: **{}**.\n\n",
        tokens.len()
    ));

    out.push_str("| Propriedade Slint | Identificador Textual (`IconId`) | Grupo Semântico |\n");
    out.push_str("| :--- | :--- | :--- |\n");
    for (name, id, group) in tokens {
        out.push_str(&format!("| `PetuniaIcons.{name}` | `{id}` | {group} |\n"));
    }
    out.push('\n');

    Ok(out)
}

fn generate_text_tokens_md(root_dir: &Path) -> Result<String> {
    let en_path = root_dir.join("assets/locales/en.toml");
    let pt_path = root_dir.join("assets/locales/pt-BR.toml");

    let en_text = std::fs::read_to_string(&en_path).unwrap_or_default();
    let pt_text = std::fs::read_to_string(&pt_path).unwrap_or_default();

    let en_val: toml::Value =
        toml::from_str(&en_text).unwrap_or(toml::Value::Table(Default::default()));
    let pt_val: toml::Value =
        toml::from_str(&pt_text).unwrap_or(toml::Value::Table(Default::default()));

    let mut keys: BTreeMap<String, (String, String)> = BTreeMap::new();

    fn extract_keys(
        prefix: &str,
        val: &toml::Value,
        en: bool,
        map: &mut BTreeMap<String, (String, String)>,
    ) {
        if let Some(table) = val.as_table() {
            for (k, v) in table {
                let full_key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                if let Some(s) = v.as_str() {
                    let entry = map
                        .entry(full_key)
                        .or_insert_with(|| ("—".into(), "—".into()));
                    if en {
                        entry.0 = s.to_string();
                    } else {
                        entry.1 = s.to_string();
                    }
                } else if v.is_table() {
                    extract_keys(&full_key, v, en, map);
                }
            }
        }
    }

    extract_keys("", &en_val, true, &mut keys);
    extract_keys("", &pt_val, false, &mut keys);

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Tokens de Tradução e Localização\n");
    out.push_str(
        "description: Catálogo canônico de chaves de internacionalização TextId (P3D-119)\n",
    );
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Catálogo Canônico de Chaves de Localização (`TextId`)\n\n");
    out.push_str("> **Single Source of Truth (P3D-088, P3D-119)**\n");
    out.push_str(
        "> A UI do Petunia3D é 100% internacionalizada. Nenhuma string do usuário é hardcoded; ",
    );
    out.push_str("todas as mensagens passam pelo motor `I18n` com fallback seguro em inglês.\n\n");

    out.push_str(&format!(
        "Total de chaves de localização cadastradas: **{}**.\n\n",
        keys.len()
    ));

    out.push_str("| Chave (`TextId`) | Inglês (`en.toml`) | Português (`pt-BR.toml`) |\n");
    out.push_str("| :--- | :--- | :--- |\n");

    for (k, (en_s, pt_s)) in keys {
        let clean_en = en_s.replace('|', "\\|").replace('\n', " ");
        let clean_pt = pt_s.replace('|', "\\|").replace('\n', " ");
        out.push_str(&format!("| `{k}` | {clean_en} | {clean_pt} |\n"));
    }
    out.push('\n');

    Ok(out)
}

fn generate_theme_tokens_md() -> Result<String> {
    let tokens = [
        (
            ThemeToken::BgCanvas,
            "Fundo geral da área de visualização 3D (Canvas)",
        ),
        (
            ThemeToken::BgHeader,
            "Barra de menu principal superior e cabeçalho da aplicação",
        ),
        (
            ThemeToken::BgPanel,
            "Fundo das barras laterais (Toolbar, Outliner, Properties)",
        ),
        (
            ThemeToken::BgPanelHeader,
            "Cabeçalho e divisores de seções dos painéis laterais",
        ),
        (
            ThemeToken::BgSurface,
            "Superfície de widgets, caixas de entrada e botões em repouso",
        ),
        (
            ThemeToken::BgSurfaceHover,
            "Superfície de widgets com realce de cursor (hover)",
        ),
        (
            ThemeToken::BgSurfaceActive,
            "Superfície de widgets em estado ativo/pressionado",
        ),
        (
            ThemeToken::TextPrimary,
            "Texto de máxima ênfase (títulos, etiquetas principais)",
        ),
        (
            ThemeToken::TextSecondary,
            "Texto de média ênfase (descrições, valores numéricos)",
        ),
        (
            ThemeToken::TextMuted,
            "Texto atenuado e atalhos secundários",
        ),
        (
            ThemeToken::TextActive,
            "Texto sobre fundo de destaque (seleção ativa)",
        ),
        (
            ThemeToken::AccentBlue,
            "Cor de destaque principal (seleção de objetos e foco)",
        ),
        (
            ThemeToken::AccentOrange,
            "Cor de destaque secundária (transformações e alertas)",
        ),
        (
            ThemeToken::AccentHover,
            "Realce sobre botões ou elementos de destaque",
        ),
        (
            ThemeToken::AccentBorder,
            "Bordas de destaque e anéis de foco ativo",
        ),
        (
            ThemeToken::BorderSubtle,
            "Divisores sutis entre seções e painéis",
        ),
        (
            ThemeToken::BorderStrong,
            "Bordas pronunciadas de caixas de diálogo e popups",
        ),
        (
            ThemeToken::BorderFocus,
            "Anel de foco acessível de teclado e widgets",
        ),
        (
            ThemeToken::StatusInfo,
            "Mensagens informativas e telemetria",
        ),
        (
            ThemeToken::StatusWarning,
            "Alertas de geometria não conforme ou limites",
        ),
        (
            ThemeToken::StatusError,
            "Erros críticos de I/O, formato corrompido ou colisão",
        ),
        (
            ThemeToken::StatusSuccess,
            "Confirmação de salvamento, exportação e snapshots",
        ),
    ];

    // Apenas os temas oficiais de V1 (capítulo 36) entram no catálogo canônico:
    // Petunia Dark (default) e Petunia High Contrast (acessibilidade). Temas de
    // usuário são declarações externas e não fazem parte do contrato de tokens.
    let theme_ids = petunia_config::theme::ThemeRegistry::OFFICIAL_V1_THEME_IDS;
    let loaded_themes: Vec<(String, Theme)> = theme_ids
        .iter()
        .map(|id| (id.to_string(), Theme::load_by_id(id)))
        .collect();

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Tokens de Temas Visuais\n");
    out.push_str("description: Catálogo canônico de tokens visuais ThemeToken e paletas do Design System (P3D-119)\n");
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Catálogo Canônico de Tokens de Temas (`ThemeToken`)\n\n");
    out.push_str("> **Single Source of Truth (P3D-085, P3D-119)**\n");
    out.push_str("> O Petunia Design System define tokens semânticos universais para eliminar cores hardcoded ");
    out.push_str("e garantir contraste, acessibilidade e flexibilidade estética.\n\n");

    out.push_str("## Matriz Comparativa de Cores por Tema\n\n");
    out.push_str("> Temas oficiais de V1 (capítulo 36): **Petunia Dark** (completo, default) e ");
    out.push_str("**Petunia High Contrast** (variação oficial de acessibilidade). ");
    out.push_str("Temas adicionais são packs declarativos do usuário (`themes/<id>/` ou `.petunia-theme`).\n\n");
    out.push_str("| Token Semântico | Função no Design | Petunia Dark | Petunia High Contrast |\n");
    out.push_str("| :--- | :--- | :---: | :---: |\n");

    for (token, desc) in tokens {
        let mut row = format!("| `ThemeToken::{token:?}` | {desc} |");
        for (_id, theme) in &loaded_themes {
            let col = theme.colors.get_token_color(token);
            let hex = format!("#{:02X}{:02X}{:02X}", col.0[0], col.0[1], col.0[2]);
            row.push_str(&format!(" `{hex}` |"));
        }
        out.push_str(&row);
        out.push('\n');
    }
    out.push('\n');

    Ok(out)
}

fn generate_supported_formats_md() -> Result<String> {
    let pipeline = DeliveryPipeline::default();

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Formatos 3D Suportados & Capacidades\n");
    out.push_str("description: Matriz de capacidades do pipeline de entrega e importadores/exportadores (P3D-119)\n");
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Matriz de Formatos 3D Suportados (`DeliveryPipeline`)\n\n");
    out.push_str("> **Single Source of Truth (P3D-068 a P3D-072, P3D-124, P3D-119)**\n");
    out.push_str("> O Petunia3D adota um pipeline modular desacoplado da interface gráfica para leitura e emissão de modelos 3D.\n\n");

    out.push_str("## Matriz de Capacidades por Formato\n\n");
    out.push_str("| Formato | Extensão | Importação | Exportação | Materiais PBR | Texturas | Cores p/ Vértice | Múltiplas Malhas | Formato Binário |\n");
    out.push_str("| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |\n");

    for fmt in FileFormat::ALL {
        let has_imp = if pipeline.importer(fmt).is_some() {
            "✅ Sim"
        } else {
            "❌ Não"
        };
        let has_exp = if pipeline.exporter(fmt).is_some() {
            "✅ Sim"
        } else {
            "❌ Não"
        };

        let caps = pipeline
            .exporter(fmt)
            .map(|e| e.capabilities())
            .or_else(|| pipeline.importer(fmt).map(|i| i.capabilities()))
            .unwrap_or(petunia_project::pipeline::FormatCapabilities {
                supports_materials: false,
                supports_textures: false,
                supports_vertex_colors: false,
                supports_multi_mesh: false,
                supports_binary: false,
            });

        let bool_str = |b: bool| if b { "✅" } else { "—" };

        out.push_str(&format!(
            "| **{}** | `.{}` | {} | {} | {} | {} | {} | {} | {} |\n",
            fmt.label(),
            fmt.extension(),
            has_imp,
            has_exp,
            bool_str(caps.supports_materials),
            bool_str(caps.supports_textures),
            bool_str(caps.supports_vertex_colors),
            bool_str(caps.supports_multi_mesh),
            bool_str(caps.supports_binary)
        ));
    }

    out.push_str("\n\n## Opções do Pipeline\n\n");
    out.push_str("- **Triangulação (`triangulate`)**: Converte faces poligonais em triângulos no momento da emissão.\n");
    out.push_str("- **Exportação de Materiais (`export_materials`)**: Emite propriedades PBR (Base Color, Roughness, Metallic, Emission).\n");
    out.push_str("- **Fator de Escala (`scale`)**: Transforma dimensões geométricas uniformemente durante importação ou exportação.\n");
    out.push_str("- **Tolerância a Falhas em Lote (`batch_export`)**: Falhas parciais em assets individuais não interrompem os demais.\n\n");

    Ok(out)
}

fn generate_index_md() -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: Referências Geradas do Código\n");
    out.push_str(
        "description: Catálogos e tabelas geradas deterministicamente pelo xtask (P3D-119)\n",
    );
    out.push_str("---\n\n");
    out.push_str(AUTOGEN_HEADER);
    out.push_str("\n\n# Dados e Referências Geradas do Código (`docs/generated/`)\n\n");
    out.push_str("Esta seção contém tabelas e catálogos técnicos extraídos diretamente das fontes canônicas ");
    out.push_str("do Petunia3D para garantir que a documentação permaneça 100% sincronizada com a implementação.\n\n");

    out.push_str("## Seções Disponíveis\n\n");
    out.push_str("- [Comandos & Ferramentas (`CommandId`)](/generated/COMMANDS): Todos os comandos registrados e suas propriedades.\n");
    out.push_str("- [Perfis e Atalhos de Teclado (`Keybinds`)](/generated/KEYBINDS): 8 perfis canônicos e atalhos mapeados.\n");
    out.push_str("- [Tokens de Ícones Semânticos (`IconId`)](/generated/ICON_TOKENS): Identificadores de iconografia.\n");
    out.push_str("- [Tokens de Tradução e i18n (`TextId`)](/generated/TEXT_TOKENS): Strings e chaves de internacionalização.\n");
    out.push_str("- [Tokens de Temas Visuais (`ThemeToken`)](/generated/THEME_TOKENS): Design system e matriz de cores.\n");
    out.push_str("- [Formatos 3D Suportados (`DeliveryPipeline`)](/generated/SUPPORTED_FORMATS): Matriz de capacidades de import/export.\n\n");

    out.push_str("Para regenerar ou validar estes catálogos:\n");
    out.push_str("```bash\n");
    out.push_str("cargo run -p xtask -- docs        # Regenera catálogos e compila o site\n");
    out.push_str("cargo run -p xtask -- docs-check  # Valida integridade e detecta drift\n");
    out.push_str("```\n");

    out
}

fn generate_manifest_json(files: &[(&str, usize)]) -> Result<String> {
    let mut file_entries = serde_json::Map::new();
    for (name, len) in files {
        file_entries.insert(
            name.to_string(),
            serde_json::json!({
                "size_bytes": len,
            }),
        );
    }

    let manifest = serde_json::json!({
        "schema_version": "1.0",
        "specification": "P3D-119",
        "generator": "petunia-xtask",
        "files": file_entries,
    });

    serde_json::to_string_pretty(&manifest).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("falha ao localizar raiz do workspace")
            .to_path_buf()
    }

    #[test]
    fn test_deterministic_generation_run_twice() {
        let root = test_root_dir();
        let cat1 = GeneratedCatalog::generate(&root).expect("geração 1 falhou");
        let cat2 = GeneratedCatalog::generate(&root).expect("geração 2 falhou");

        assert_eq!(
            cat1.commands_md, cat2.commands_md,
            "COMMANDS.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.keybinds_md, cat2.keybinds_md,
            "KEYBINDS.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.icon_tokens_md, cat2.icon_tokens_md,
            "ICON_TOKENS.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.text_tokens_md, cat2.text_tokens_md,
            "TEXT_TOKENS.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.theme_tokens_md, cat2.theme_tokens_md,
            "THEME_TOKENS.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.supported_formats_md, cat2.supported_formats_md,
            "SUPPORTED_FORMATS.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.index_md, cat2.index_md,
            "index.md deve ser 100% determinístico"
        );
        assert_eq!(
            cat1.manifest_json, cat2.manifest_json,
            "manifest.json deve ser 100% determinístico"
        );
    }

    #[test]
    fn test_commands_catalog_contents() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        assert!(cat.commands_md.contains("# Catálogo Canônico de Comandos"));
        assert!(cat.commands_md.contains("`file.new`"));
        assert!(cat.commands_md.contains("`file.save`"));
        assert!(cat.commands_md.contains("`edit.undo`"));
        assert!(cat.commands_md.contains("`model.extrude`"));
        assert!(cat.commands_md.contains("`model.bevel`"));
        assert!(cat.commands_md.contains("`model.subdivide`"));
        assert!(cat.commands_md.contains("`model.merge`"));
    }

    #[test]
    fn test_keybinds_catalog_contents() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        assert!(cat.keybinds_md.contains("`petunia-default`"));
        assert!(cat.keybinds_md.contains("`blender`"));
        assert!(cat.keybinds_md.contains("`maya`"));
        assert!(cat.keybinds_md.contains("`3ds-max`"));
        assert!(cat.keybinds_md.contains("`cinema-4d`"));
    }

    #[test]
    fn test_icon_tokens_catalog_contents() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        assert!(cat.icon_tokens_md.contains("`PetuniaIcons.select-box`"));
        assert!(cat.icon_tokens_md.contains("`PetuniaIcons.extrude`"));
        assert!(cat.icon_tokens_md.contains("`PetuniaIcons.bevel`"));
        assert!(cat.icon_tokens_md.contains("`select_box`"));
        assert!(cat.icon_tokens_md.contains("`extrude`"));
    }

    #[test]
    fn test_text_tokens_catalog_contents() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        assert!(cat.text_tokens_md.contains("`app.title`"));
        assert!(cat.text_tokens_md.contains("`ui.tools`"));
        assert!(cat.text_tokens_md.contains("`menu.file`"));
    }

    #[test]
    fn test_theme_tokens_catalog_contents() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        assert!(cat.theme_tokens_md.contains("`ThemeToken::BgCanvas`"));
        assert!(cat.theme_tokens_md.contains("`ThemeToken::AccentBlue`"));
        assert!(cat.theme_tokens_md.contains("`ThemeToken::StatusSuccess`"));
        // Catálogo canônico cobre apenas os temas oficiais de V1 (cap. 36).
        assert!(cat.theme_tokens_md.contains("Petunia Dark"));
        assert!(cat.theme_tokens_md.contains("Petunia High Contrast"));
        assert!(
            !cat.theme_tokens_md.contains("Petunia Light"),
            "temas não oficiais não entram no catálogo de tokens"
        );
    }

    #[test]
    fn test_supported_formats_catalog_contents() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        assert!(cat.supported_formats_md.contains("Wavefront OBJ"));
        assert!(cat.supported_formats_md.contains("glTF 2.0 (JSON)"));
        assert!(cat.supported_formats_md.contains("glTF 2.0 Binary (GLB)"));
        assert!(cat.supported_formats_md.contains("Petunia Package (.pkg)"));
    }

    #[test]
    fn test_manifest_json_validation() {
        let root = test_root_dir();
        let cat = GeneratedCatalog::generate(&root).expect("geração falhou");

        let parsed: serde_json::Value =
            serde_json::from_str(&cat.manifest_json).expect("manifest.json deve ser JSON válido");

        assert_eq!(parsed["specification"], "P3D-119");
        assert_eq!(parsed["generator"], "petunia-xtask");
        assert!(
            parsed["files"]["COMMANDS.md"]["size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            parsed["files"]["KEYBINDS.md"]["size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            parsed["files"]["ICON_TOKENS.md"]["size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            parsed["files"]["TEXT_TOKENS.md"]["size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            parsed["files"]["THEME_TOKENS.md"]["size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            parsed["files"]["SUPPORTED_FORMATS.md"]["size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(parsed["files"]["index.md"]["size_bytes"].as_u64().unwrap() > 0);
    }
}
