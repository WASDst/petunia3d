# 18 — Stack Técnica, Dependências e Fronteiras de Integração

<aside>
🛠️

Esta página é o **índice normativo da stack atual**. A baseline Odin registrada anteriormente foi substituída pelo ADR do capítulo 32. Desde a decisão de 2026-10-08, a stack de produto vigente é **Rust 2024 + Slint 1.18 + wgpu + Geometry Core próprio em Rust**. egui/eframe/egui-wgpu permanecem apenas no fallback congelado e não definem novas features. A política de frontend e reavaliação está no [ADR 009](../../architecture/adr/009-slint-producao-egui-fallback-congelado.md); detalhes funcionais seguem os capítulos 27–36.

</aside>

# Status atual

**FINAL BASELINE adotada para implementação.** O vertical slice do capítulo 31 é teste obrigatório de conformance/integração de UI, viewport, geometry, persistência, export, Lua e MCP. Ele não reabre automaticamente a escolha da stack; somente bloqueador estrutural comprovado + ADR explícito autoriza reavaliação.

# Stack principal

```
Rust 2024
├ Slint 1.18
├ wgpu 30.x
├ shell declarativo `petunia_ui_slint`
├ viewport adapter toolkit-neutro + fallback software
├ WGSL
├ glam
├ slotmap
├ geo
├ manifold-rust
├ xatlas fallback via wrapper/provider
├ image
├ Serde + serde_json + zip
├ gltf + gltf-json
├ mlua + Lua 5.4
├ rmcp + Tokio isolado no MCP
├ Rayon + flume
└ tracing / thiserror / schemars / test tooling
```

# Decisão de linguagem

Rust substitui Odin como linguagem principal do aplicativo. Rust deve concentrar Application Core, Geometry Core, renderer, I/O, plugin host, MCP e tooling. Lua continua sendo a linguagem pública de Community Plugins.

A meta é evitar a antiga combinação Odin + C/C++ providers + Go MCP quando existirem alternativas Rust suficientemente robustas.

# UI

**Slint 1.18 é a toolkit ativa de produção V1.** A UI obedece aos capítulos 22–26, à autoridade final do capítulo 36 e ao ADR 009: componentes reais e acessíveis, viewport-first, workspaces e painéis contextuais, tokens, themes e Plugin Panels sobre Design System próprio.

Slint deve ficar isolado em `petunia_ui_slint`. **UI de produto em workspaces usa Petunia Components/controles declarativos e adapters próprios como fronteira obrigatória**; callbacks emitem `UiIntent` e não carregam regras de domínio. Nenhum domínio interno depende de Slint nem de egui. O capítulo 35 permanece como referência histórica do frontend egui congelado, não como autoridade da UI ativa.

# Renderer

**wgpu 30.x** é a baseline do viewport. Não usar engine completa. `petunia-render` é um renderer pequeno próprio sobre wgpu + WGSL.

A integração com a UI passa pelo adapter de viewport do frontend Slint e pelo `PetuniaRenderer`, mantendo renderer e domínio toolkit-neutros. O caminho GPU é preferido; fallback software permanece disponível. Qualquer alegação de zero-copy ou ausência de readback deve ser comprovada pela implementação e profiling, conforme ADR 009.

# Geometry

O authoring model permanece **PetuniaMesh próprio**, half-edge, com IDs generacionais tipados via `slotmap`. `geo` auxilia Profile/polygon 2D. `manifold-rust` fornece Fuse/Cut complexo. xatlas permanece fallback de Generic Auto UV atrás de `UvUnwrapProvider`.

Não adotar Bevy/ECS/engine, B-Rep/CAD ou uma third-party mesh como autoridade do documento.

# I/O

`.petunia` continua ZIP + JSON authoring data, agora implementado com Serde/serde_json/zip. GLB/glTF usa `gltf`/`gltf-json`; OBJ permanece secundário, com writer próprio para export e `tobj` como parser baseline de import atrás de `Importer`. Image codecs são adapters, não tipos de domínio.

# Plugins

Lua 5.4 continua a API pública. Host Rust usa `mlua`, um Lua State por plugin, capabilities, sandbox e mutations via Command/Application API.

A extensibilidade visual V1 possui duas superfícies oficiais, normatizadas no capítulo 36:

- **Theme Extensions** `.petunia-theme`, puramente declarativas e compartilháveis;
- **Plugin Panels**, registrados por Lua através da `Petunia UI Extension API`, compostos exclusivamente por Petunia Components e extension slots controlados.

Community Plugins não recebem acesso cru a egui/wgpu/Painter/raw input. Painéis herdam tema, keyboard/focus e accessibility semantics do host.

# MCP

A stack Rust elimina a necessidade arquitetural do antigo sidecar Go. `petunia-mcp` passa a ser Rust sobre `rmcp`; Tokio fica restrito ao módulo MCP. Requests entram na Application thread por channel e nunca recebem `&mut Document` diretamente.

# Concorrência

Preservar o contrato anterior: **single-writer document + immutable snapshots/jobs + revision check**. Na stack Rust, Rayon é o pool de CPU jobs e flume é a opção baseline para channels. Não espalhar async/Tokio pelo core.

# Build

Cargo workspace governa o projeto. `Cargo.lock` do aplicativo é versionado. Um `xtask` padroniza check/test/verify/package/licenses/fixtures/UI tests.

# Segurança

Preferir `#![forbid(unsafe_code)]` nas crates de domínio. Unsafe inevitável de integração deve ser pequeno, encapsulado e acompanhado de contract/fuzz tests.

Dependências entram somente após avaliação de licença, manutenção, targets, segurança, build, API e substituibilidade.

# Documentos normativos da stack

- [27 — Stack Rust Canônica](27-stack-rust-canonica.md) — stack Rust vigente; trechos egui são históricos e subordinados ao ADR 009.
- [28 — Arquitetura Rust, Cargo Workspace e Fronteiras entre Crates](28-arquitetura-rust-cargo-crates.md) — crates e dependency boundaries.
- [29 — Geometry Core, Renderer, UV e Painting na Stack Rust](29-geometry-renderer-uv-painting-rust.md) — geometry, rendering, UV e painting.
- [30 — I/O, Projeto .petunia, Lua Plugins e MCP na Stack Rust](30-io-petunia-lua-mcp-rust.md) — I/O, plugins e MCP.
- [31 — Qualidade, IA, CI e Vertical Slice de Conformance da Stack Rust](31-qualidade-ia-ci-vertical-slice.md) — qualidade, IA, CI e vertical slice.
- [32 — ADR: Migração da Baseline Odin para Rust](32-adr-odin-para-rust.md) — ADR Odin → Rust.
- [33 — Biblioteca de Referências Técnicas da Stack Rust](33-referencias-tecnicas-rust.md) — biblioteca de referências oficiais e projetos/papers relacionados à stack.
- [34 — Arquitetura Modular Explícita, Rust Safety e Representação em Código](34-arquitetura-modular-rust-safety.md) — **arquitetura de código Rust-safe**, representação de entidades/recursos/topologia, DOD/ECS seletivo, Tool→Command→Algorithm→Data, modularidade e regras para agentes.
- [35 — egui, Petunia Components, UI Adapters e Tooling de Desenvolvimento](35-egui-components-adapters-tooling.md) — **referência histórica do fallback egui**; os princípios toolkit-neutros de Petunia Components continuam úteis, mas a UI ativa é regida pelo capítulo 36 + ADR 009.

# Regra contra bloat

Não adicionar engines, frameworks ou crates redundantes antes de necessidade mensurável. Providers complexos ficam atrás de traits e não vazam tipos externos para Application API/document format.

**Não usar ECS universal como fundamento do aplicativo.** DOD é seletivo; Document permanece explícito e Geometry usa sua estrutura especializada. Traits, generics e dynamic dispatch devem seguir a política do capítulo 34, e não uma regra genérica de “interface para tudo”.

# Regra final

**A stack atual é Rust-first. O produto continua sendo a autoridade: qualquer biblioteca pode ser substituída se comprometer simplicidade, UX, previsibilidade ou manutenção.**