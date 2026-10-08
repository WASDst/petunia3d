# 27 — Stack Rust Canônica: Rust + egui + wgpu

<aside>
🦀

Esta página define a **technical baseline final do Petunia3D** para implementação: **Rust 2024 + egui + eframe + egui-wgpu + wgpu + Geometry Core próprio em Rust**. A stack substitui a baseline anterior centrada em Odin e adota uma UI Rust-first permissiva, diretamente alinhada ao renderer wgpu. O vertical slice técnico do capítulo 31 é agora um **teste de conformance/integração**; reabrir a stack exige bloqueador estrutural comprovado + ADR explícito.

</aside>

# Objetivo da stack

A stack deve permitir que um único desenvolvedor, fortemente assistido por agentes de IA, construa e mantenha um modelador low-poly desktop com:

- interface altamente customizada e fiel ao Design System Petunia;
- viewport 3D moderno e cross-platform;
- Geometry Core previsível e seguro;
- modelagem direta e profile-based;
- UV automático e projeção de referência;
- pintura simples sobre textura;
- Undo/Redo transacional;
- formato de projeto versionado;
- plugins Lua;
- MCP nativo;
- testes automatizados extensos;
- Windows e Linux como targets obrigatórios iniciais.

A arquitetura deve minimizar glue entre linguagens. **Rust é a linguagem de primeira classe de quase todo o executável. Lua permanece a linguagem pública de plugins.** A única dependência estrutural que pode continuar usando implementação C/C++ por baixo é xatlas, isolada atrás de provider.

# Licença do projeto e política de dependências

**Petunia3D será licenciado sob MIT.** Isso é uma restrição arquitetural, não apenas uma decisão de publicação.

Sempre que houver alternativas tecnicamente equivalentes, preferir dependências sob:

```
MIT
Apache-2.0
MIT OR Apache-2.0
BSD-2-Clause
BSD-3-Clause
ou licença permissiva equivalente explicitamente aprovada
```

Licenças customizadas, copyleft forte, source-available ou restrições comerciais exigem decisão arquitetural explícita antes da adoção.

`cargo-deny`/tooling equivalente deve validar a allowlist de licenças no CI.

# Stack resumida

| Área | Baseline | Papel |
| --- | --- | --- |
| Linguagem | Rust 2024 Edition | Aplicação, core, geometry, renderer, I/O, plugins host, MCP. |
| Build | Cargo workspace | Build, testes, dependências, tooling. |
| UI | egui 0.36.x | Immediate-mode UI, input, layout, painting, accessibility tree e shell desktop. |
| Desktop host | eframe 0.36.x | Window/app integration e execução desktop. |
| UI ↔ GPU | egui-wgpu 0.36.x | Integração oficial egui/wgpu e custom rendering dentro de regiões da UI. |
| GPU | wgpu 30.x | Viewport, overlays, gizmos e uploads de textura. |
| Shaders | WGSL | Shaders únicos cross-backend. |
| Math | glam | Vec2/Vec3/Quat/Mat4 e matemática 2D/3D. |
| IDs | slotmap | Handles generacionais tipados. |
| 2D Geometry | geo | Profile operations e polygon predicates/booleans quando apropriado. |
| 3D Boolean | manifold-rust | Fuse/Cut complexo. |
| Auto UV | Petunia native + xatlas fallback | UV determinística/projeção + unwrap genérico. |
| Picking | ray/triangle próprio inicialmente | Seleção e Paint on Model; BVH somente se profiling justificar. |
| Images | image com codecs explícitos | Decode/encode de texturas/referências. |
| Painting | TextureBitmap próprio | Buffer CPU, Undo e dirty regions. |
| glTF/GLB | gltf + gltf-json | Import/export principal. |
| OBJ | writer pequeno próprio + tobj para import | Formato secundário; export controlado pelo Petunia e import atrás de Importer. |
| Serialize | Serde + serde_json | Document/project data. |
| Container | zip | Arquivo de projeto versionado. |
| Plugins | Lua 5.4 + mlua | Extension API pública. |
| MCP | rmcp + Tokio isolado | Servidor MCP Rust nativo. |
| CPU jobs | Rayon | Boolean/UV/validation/export work. |
| Channels | flume | Comunicação service/worker → single writer. |
| Errors | thiserror; anyhow nas bordas | Erros de domínio tipados + contexto de app. |
| Logging | tracing | Observabilidade estruturada. |
| Schemas | schemars + Serde | Command/MCP/plugin schemas. |
| Property tests | proptest | Invariantes topológicas e commands. |
| Benchmarks | Criterion | Performance baseada em fixtures reais. |
| Fuzzing | cargo-fuzz | Loaders, geometry boundaries e parsers. |

# Por que egui

A UI do Petunia é uma ferramenta técnica/dcc-like com grande quantidade de controls contextuais, viewport próprio e forte integração com input. egui se encaixa nesse perfil porque:

- é Rust-first e permissivo (`MIT OR Apache-2.0`);
- integra diretamente com wgpu pelo `egui-wgpu` oficial;
- permite custom rendering em uma região da UI, reduzindo glue de viewport;
- possui AccessKit como base de accessibility/inspection;
- possui ecossistema ativo de widgets para tree, tiling, tables, dialogs e testing;
- permite construir o Design System Petunia em uma camada própria sem vazar o toolkit para o core;
- é usado em aplicações profissionais/visualizers Rust com renderer próprio, oferecendo referência arquitetural real.

Immediate mode não autoriza misturar estado de domínio com estado de apresentação. O Document continua single-writer e a UI somente observa view models/snapshots e dispara Commands.

# UI como camada própria

A regra de dependência é:

```
Petunia workspaces/screens
        ↓
Petunia Components
        ↓
Petunia UI adapters
        ↓
egui / eframe / selected egui crates
```

Não espalhar chamadas cruas de egui por todo o projeto. **A UI de produto deve passar por Petunia Components/adapters como regra arquitetural**; egui cru fica restrito à implementação desses boundaries e a devtools explicitamente delimitadas. Componentes próprios incluem:

```
PetuniaButton
PetuniaIconButton
PetuniaSplitButton
PetuniaSegmentedControl
PetuniaNumberField
PetuniaPanel
PetuniaPanelHeader
PetuniaContextSection
PetuniaTree
PetuniaAssetCard
PetuniaWorkspacePill
PetuniaTooltip
PetuniaToolbar
PetuniaModal
```

Widgets externos complexos entram atrás de adapters próprios, preservando substituibilidade.

# Design System

O Design System Petunia é autoridade visual. egui fornece interação/layout/semântica; não define a identidade do produto.

A camada `foundation` deve centralizar:

```
colors
typography
spacing
radii
strokes
metrics
motion
icons
interaction states
focus visuals
```

`egui::Style`, `Visuals`, `Spacing`, `WidgetVisuals` e custom painting são configurados a partir desses tokens.

# Viewport: integração direta egui-wgpu

A arquitetura preferida do viewport é:

```
egui layout
    ↓ allocate viewport Rect
Petunia viewport widget/adapter
    ↓ egui-wgpu custom callback
PetuniaRenderer
    ↓ wgpu RenderPass / resources
GPU
```

O renderer continua em `petunia-render`; `petunia-ui` apenas fornece o retângulo, input/view state e adapter de integração.

Não fazer o Geometry Core depender de egui. Não fazer `PetuniaRenderer` depender de objetos de estado mutável da UI além de um adapter estreito na fronteira de renderização.

# Accessibility

Accessibility usa a árvore AccessKit exposta pelo egui/host.

Todo componente Petunia customizado deve publicar semântica apropriada quando aplicável:

```
role
accessible name
description
enabled/disabled
selected/checked/expanded
keyboard activation
focus state
```

Uma customização visual não pode eliminar a semântica do controle.

# Testabilidade da UI

UI é código testável. A baseline de desenvolvimento deve usar:

- `egui_kittest` para semantic queries, interaction tests e render/snapshot tests;
- `egui_inspection` em builds de desenvolvimento para inspeção externa da árvore/interface;
- `egui_mcp` em ambiente de desenvolvimento para permitir agentes inspecionarem, interagirem e capturarem screenshots;
- visual regression com tolerância explicitamente controlada.

Serviços de inspection/MCP de UI ficam **desabilitados em builds de distribuição** e, quando ativos, restritos a loopback/configuração de desenvolvimento.

# Ecossistema UI inicial

A lista abaixo **não é uma lista de dependências automáticas do Cargo**. O capítulo 35 é a autoridade para classificação detalhada e adapters.

## BASELINE

- `egui`;
- `eframe`;
- `egui-wgpu`;
- `egui_extras`.

## BASELINE

- `egui_ltreeview` para Parts tree, atrás de `PetuniaTreeAdapter`;
- `egui_tiles` para engine de layout/painéis controlados, atrás de `PetuniaLayoutAdapter`;
- `egui-lucide` como pack genérico principal de ícones;

## OPTIONAL / REFERENCE

- `egui-file-dialog` permanece optional; fluxo padrão usa dialogs nativos do SO;
- `egui-phosphor` permanece REFERENCE/MONITOR.

## DEV TOOL

- `egui_kittest` — obrigatório em dev/tests para semântica, interação e snapshots;
- `egui_inspection` — somente development build;
- `egui_mcp` — somente development build e acesso local/loopback;
- `egui_json_tree`, `egui_probe`, `puffin_egui` e tooling semelhante conforme necessidade de diagnóstico.

## OPTIONAL / FUTURE

- `egui_animation`;
- `egui_form`;
- `egui_dnd`;
- `egui_commonmark`;
- `egui_table`;
- `egui-data-table`;
- `egui-snarl`.

Itens `REFERENCE/MONITOR` ou `REJECTED/NOT RELEVANT` não entram no baseline e são catalogados no capítulo 35.

Detalhes, critérios, wrappers e promoção entre categorias são normatizados no capítulo específico do ecossistema egui.

# Política para crates auxiliares de UI

Uma crate de UI só entra se resolver interação/infraestrutura não trivial que seria custo real reproduzir.

Antes de adicionar:

1. licença permissiva compatível com a política MIT do projeto;
2. compatibilidade com a linha egui pinada;
3. atividade/manutenção;
4. targets Windows/Linux;
5. accessibility behavior quando relevante;
6. impacto de build/dependências;
7. possibilidade de isolamento atrás de adapter;
8. snapshot/interaction tests próprios do Petunia.

Uma biblioteca de componentes nunca se torna autoridade visual do Petunia.

# Rust como linguagem central

Rust deverá ser usado para:

- Application Core;
- Document Model;
- Command Registry;
- Geometry Core;
- Renderer;
- caches;
- UI/application presentation;
- I/O;
- import/export orchestration;
- plugin host;
- MCP adapter/server;
- build tooling/xtask.

Não criar um core Rust e depois espalhar C++, Go ou outra linguagem pela aplicação sem uma necessidade comprovada.

# Política de unsafe

Regra default para crates de domínio:

```rust
#![forbid(unsafe_code)]
```

Aplicar onde viável em `petunia-core`, `petunia-geometry`, `petunia-io`, lógica de plugins e MCP. Dependências externas podem internamente utilizar unsafe, mas isso não autoriza unsafe arbitrário no nosso código.

Se uma integração nativa realmente exigir unsafe, concentrá-la em módulo/crate de fronteira pequeno, documentado e testado.

# Dependências deliberadamente fora da baseline

- Bevy/engine completa;
- ECS genérico;
- Skia para painting V1;
- CAD/B-Rep/OpenCascade;
- Assimp como importer universal;
- banco de dados para o documento;
- engine de render offline;
- runtime web/CEF;
- Tokio dentro do Geometry/Application Core;
- remesh/retopology automática;
- segundo toolkit UI concorrente no build normal.

# Targets de plataforma

Baseline obrigatória:

- Windows x86_64;
- Linux x86_64, com Wayland como cenário de teste importante.

macOS deve permanecer possível arquiteturalmente, mas não precisa bloquear o primeiro milestone se não houver infraestrutura de CI/teste suficiente.

# Cargo workspace — baseline orientativa

O workspace real deve pinar versões compatíveis e versionar `Cargo.lock`.

```toml
[workspace]
resolver = "2"

[workspace.package]
edition = "2024"

[workspace.dependencies]
# UI / GPU
egui = "0.36"
eframe = { version = "0.36", default-features = false, features = ["wgpu"] }
egui-wgpu = "0.36"
egui_extras = "0.36"
wgpu = "30"
bytemuck = { version = "1", features = ["derive"] }

# Domain / math / IDs
glam = { version = "0", features = ["serde"] }
slotmap = { version = "1", features = ["serde"] }
smallvec = { version = "1", features = ["serde"] }
indexmap = { version = "2", features = ["serde"] }
uuid = { version = "1", features = ["serde", "v4"] }

# Geometry
geo = "0.33"
manifold-rust = "0.13"
xatlas_rs_v2 = "0.1"

# Data / I/O
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1"
zip = "8"
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }

# Plugins
mlua = { version = "0.12", features = ["lua54", "vendored", "serde"] }

# Runtime services
rayon = "1"
flume = "0"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
rmcp = "<pin compatible>"

# Diagnostics
thiserror = "2"
anyhow = "1"
tracing = "0.1"
```

Crates auxiliares egui devem ser adicionadas somente quando a feature correspondente entrar no vertical slice/produto e sempre pinadas na linha compatível com egui 0.36.x.

# Política de version/update

Updates de dependência exigem:

```
read release notes
→ verify egui/eframe/egui-wgpu/wgpu compatibility
→ cargo check/test/clippy
→ provider contract tests
→ UI semantic/snapshot tests
→ viewport GPU tests
→ license/security check
→ update Cargo.lock
→ record material delta
```

Não fazer bulk update cego em dependencies centrais.

# Toolchain Rust

Versionar `rust-toolchain.toml`. Preferir stable Rust compatível com as crates pinadas. Nightly só pode ser introduzido para ferramenta isolada e nunca como requisito silencioso do aplicativo.

# Build profiles

Manter pelo menos:

```
dev       → debug rápido e assertions/invariants
release   → distribuição
profiling → símbolos/instrumentation suficiente para medir hotspots
```

# Princípio final

**Uma linguagem principal, UI Rust-first permissiva, renderer wgpu próprio, fronteiras estreitas e bibliotecas especializadas apenas para problemas que não criam identidade ao Petunia.**