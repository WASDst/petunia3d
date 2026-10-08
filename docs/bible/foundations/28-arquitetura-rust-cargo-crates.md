# 28 — Arquitetura Rust, Cargo Workspace e Fronteiras entre Crates

<aside>
🏗️

Esta página define a arquitetura física do repositório Rust. O objetivo é obter modularidade suficiente para testes e agentes de IA sem fragmentar o projeto em dezenas de crates difíceis de navegar. A representação e as regras de implementação dentro dessas fronteiras são normatizadas pelo capítulo 34 — **Explicit Modular Data Architecture**.

</aside>

# Princípio de dependência

As camadas internas não dependem das externas. UI, renderer, Lua e MCP são adapters/serviços ao redor do domínio.

```
petunia-ui ─────┐
petunia-plugins ├→ petunia-core → petunia-geometry
petunia-mcp ────┘

petunia-render ← RenderSnapshot
petunia-io ← Project/Export models

apps/petunia = composition root
```

Regra absoluta:

```
petunia-core      → NO egui, NO eframe, NO wgpu, NO mlua, NO rmcp
petunia-geometry  → NO egui, NO eframe, NO wgpu
petunia-render    → NO egui, NO eframe
petunia-ui        → pode conhecer egui/eframe/egui-wgpu e UI adapters
apps/petunia      → conecta tudo
```

Se egui for substituído no futuro, Geometry, Document, File Format, plugin contracts e renderer não devem precisar ser reescritos. Workspaces devem depender prioritariamente de Petunia Components/adapters, reduzindo o alcance de uma eventual troca de toolkit.

# Estrutura do workspace

```
petunia3d/
├ Cargo.toml
├ Cargo.lock
├ rust-toolchain.toml
├ rustfmt.toml
├ clippy.toml
├ deny.toml
├ apps/
│  └ petunia/
├ crates/
│  ├ petunia-core/
│  ├ petunia-geometry/
│  ├ petunia-render/
│  ├ petunia-ui/
│  ├ petunia-io/
│  ├ petunia-plugins/
│  └ petunia-mcp/
├ design/
│  ├ figma/
│  ├ tokens/
│  └ references/
├ assets/
│  ├ icons/
│  ├ branding/
│  └ default/
├ locales/
│  ├ en-US.json
│  └ pt-BR.json
├ fixtures/
│  ├ meshes/
│  ├ profiles/
│  ├ projects/
│  └ exports/
├ plugins/
│  └ examples/
├ shaders/
└ xtask/
```

# petunia-core

Responsável por conceitos de domínio/aplicação sem detalhes gráficos. **Domain e Application são camadas lógicas dentro desta crate inicialmente**, não novas crates obrigatórias.

Estrutura interna sugerida:

```
petunia-core/src/
├ domain/
│  ├ document/
│  ├ objects/
│  ├ materials/
│  ├ textures/
│  ├ references/
│  └ ids/
├ application/
│  ├ commands/
│  ├ transactions/
│  ├ history/
│  ├ selection/
│  └ registry/
└ lib.rs
```

Responsabilidades:

- IDs de domínio compartilhados;
- Document;
- assets/parts hierarchy;
- selection;
- materials/references/textures metadata;
- Command Registry;
- Application API;
- Transactions;
- Undo/Redo/history;
- session state;
- revision numbers;
- command descriptors e schemas abstratos.

Não executa rendering e não sabe como um botão é desenhado. Uma futura separação física em `petunia-domain`/`petunia-application` só acontece se houver benefício arquitetural comprovado.

# petunia-geometry

Responsável por:

- `PetuniaMesh`;
- half-edge topology;
- Profiles e WorkPlanes;
- triangulation cache;
- normals;
- Extrude/Push-Pull;
- Inset;
- Cut/Slice;
- Fuse adapter;
- Connect/Bridge/Weld;
- Bevel de 1 segmento;
- Mirror;
- Revolve;
- UV generation/adapter;
- geometry validation.

Dependências geométricas ficam aqui ou atrás de traits daqui.

# petunia-render

Responsável por:

- wgpu context/device/queue;
- pipelines;
- WGSL shaders;
- GPU mesh caches;
- camera rendering;
- reference images;
- wireframe;
- selection overlays;
- profiles/workplane overlays;
- gizmos;
- snap hints;
- custom viewport rendering integrado ao render pass wgpu via adapter egui-wgpu;
- dirty-region texture uploads.

Recebe `RenderSnapshot`. **Não recebe `&mut Document`.**

# petunia-ui

Responsável por:

- egui/eframe shell;
- Petunia Design System;
- foundation/tokens;
- Petunia Components;
- UI adapters para crates auxiliares;
- Model/Paint/UV workspace composition;
- PartsTree;
- ContextPanel;
- AssetLibrary;
- toolbar/viewport controls;
- AccessKit/accessibility semantics;
- keyboard focus e presentation layer;
- adapter `egui-wgpu` para custom viewport rendering.

A UI envia intents/commands; não chama primitives de half-edge diretamente. Workspaces não devem depender de crates auxiliares de terceiros quando um adapter Petunia puder encapsulá-las.

# petunia-io

Responsável por:

- `.petunia` ZIP;
- JSON/Serde;
- migrations;
- atomic save;
- recovery snapshots;
- image codecs;
- GLB/glTF;
- OBJ;
- ExportModel;
- import adapters.

O authoring model não usa structs específicas de glTF como representação interna.

# petunia-plugins

Responsável por:

- mlua/Lua 5.4;
- PluginHost;
- manifest;
- capabilities;
- sandbox;
- um Lua State por plugin;
- command registration;
- subscriptions/event dispatch;
- lifecycle enable/disable/unload.

# petunia-mcp

Responsável por:

- rmcp;
- MCP tools/resources;
- schemas;
- permissions;
- tradução MCP ↔ Command Registry;
- Tokio runtime isolado;
- channels até a Application/Main thread.

Não possui o Document.

# apps/petunia

É o composition root. Deve conter o mínimo possível de lógica:

```
create application services
create document/session
create renderer
create UI
create PluginHost
create MCP service
wire channels/callbacks
run main loop
```

# xtask

Criar tooling Rust próprio para padronizar comandos frequentes:

```
cargo xtask check
cargo xtask test
cargo xtask verify
cargo xtask ui-test
cargo xtask fixtures
cargo xtask package
cargo xtask licenses
```

Agentes de IA devem usar `cargo xtask verify` como definição automatizada mínima de “pronto”.

# Command Registry

O Command Registry é a fronteira comum entre UI, Lua e MCP.

Descriptor conceitual:

```
command_id
label_key
description_key
argument_schema
result_schema
accepted_contexts
required_capabilities
undoable
previewable
lua_exposable
mcp_exposable
help_manual_id
```

Internamente, commands permanecem Rust tipados. JSON/schema existe apenas em fronteiras dinâmicas.

Exemplo conceitual:

```rust
pub enum AppCommand {
    Extrude(ExtrudeCommand),
    Move(MoveCommand),
    Fuse(FuseCommand),
    Cut(CutCommand),
}
```

# Single source para UI/Lua/MCP

```
Rust command + metadata
          │
          ├→ UI presentation
          ├→ Lua binding
          └→ MCP tool schema
```

Evitar manter três APIs manuais que inevitavelmente divergirão.

# RenderSnapshot

Renderer não lê o documento vivo. A Application layer cria uma representação somente de leitura:

```
RenderSnapshot
├ meshes/render buffers source
├ transforms
├ materials
├ references
├ selection
├ active profile/workplane
├ overlays
└ camera
```

Isso reduz acoplamento, races e acesso acidental ao authoring state.

# ExportModel

Também criar representação explícita de export:

```
ExportModel
├ triangulated meshes
├ materials
├ textures
├ transforms
└ metadata necessária ao exporter
```

Exporters não devem adaptar diretamente a half-edge mesh viva para cada formato.

# Regra contra over-modularization

Não criar um crate por classe ou por comando. Uma nova crate só existe quando possui fronteira arquitetural real, dependências significativamente diferentes ou necessidade de compilação/teste isolado.

Modules são a unidade barata de encapsulamento. Crates são boundaries físicos de dependência/compilação. Uma feature pode ser altamente modular sem ganhar uma crate própria.

# Error model

Crates de domínio usam `thiserror` para erros tipados. `anyhow` fica restrito a composition root, CLI/tools e fronteiras onde contexto adicional é útil.

Nunca mostrar diretamente `Debug`/stack trace técnico ao usuário final. O core deve retornar código/contexto estruturado, por exemplo:

```
GeometryError::IncompatibleBoundaries
→ error key: geometry.connect.incompatible_boundaries
→ structured context: selected loops/counts
→ UI/localization chooses user-facing message
```

Isso permite logs técnicos e mensagens amigáveis coexistirem.

# i18n

Preservar a decisão de internacionalização por tokens JSON:

```
locales/
├ en-US.json
└ pt-BR.json
```

Exemplo:

```json
{
  "command.model.extrude": "Extrude",
  "command.model.cut": "Cut",
  "panel.parts": "Parts"
}
```

Strings de produto não ficam espalhadas em widgets/workspaces Rust. A UI resolve labels por tokens/chaves i18n através de um `I18nModel`/adapter centralizado. Tests devem incluir labels longos e expansão de idioma.

# Naming/Clean Code

Priorizar nomes explícitos sobre abreviações. Traits representam fronteiras reais (`UvUnwrapProvider`, `ProjectStorage`, `Exporter`) e não abstrações especulativas. Funções pequenas e coesas; comentários explicam invariants/rationale, não repetem o código.

Aplicar integralmente o capítulo 34: evitar `ctx/mgr/svc/cfg/doc/rev/obj/geom/tex/sel/cmd` quando nomes completos forem naturais; permitir abreviações canônicas de domínio (`UV`, `GPU`, `CPU`, `UUID`, `MCP`, `PBR`, `RGB`). Não usar traits, generics, `Arc<Mutex<_>>`, `Rc<RefCell<_>>`, macros ou `.clone()` como mecanismos automáticos para contornar um desenho de ownership ruim.

# Arquitetura de código especializada

A página [34 — Arquitetura Modular Explícita, Rust Safety e Representação em Código](34-arquitetura-modular-rust-safety.md) complementa esta arquitetura física e tem precedência para decisões de representação interna, ownership, `struct/enum/trait`, DOD/ECS, Tool/Command/Algorithm, safety, modularidade de features e regras para agentes de IA.

[35 — egui, Petunia Components, UI Adapters e Tooling de Desenvolvimento](35-egui-components-adapters-tooling.md) define Petunia Components, adapters, crates auxiliares egui, testing/inspection e fronteiras específicas da UI.

# Regra final

**Poucas crates, responsabilidades fortes, dependências em uma direção e nenhum framework externo vazando para o domínio.**