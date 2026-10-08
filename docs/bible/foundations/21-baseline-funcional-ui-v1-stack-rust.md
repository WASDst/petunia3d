# 21 — Baseline Funcional e UI V1 Congeladas, Stack Rust Final

<aside>
🔒

O **escopo de produto V1**, o **comportamento funcional/core** e a **UI Baseline Final V1** do Petunia3D estão congelados. A linguagem/core continuam em **Rust 2024** e o viewport em **wgpu**. A decisão de frontend foi refinada em 2026-10-08: **Slint 1.18 é a única UI ativa de produção** e egui é fallback congelado, conforme [ADR 009](../../architecture/adr/009-slint-producao-egui-fallback-congelado.md). O vertical slice do capítulo 31 continua teste obrigatório de conformance/integração. O capítulo 34 rege a arquitetura Rust e o **capítulo 36 rege a UI final, temas e Plugin Panels**; o capítulo 35 é histórico para o fallback egui.

</aside>

# Baseline funcional congelada

- paradigma polygon mesh shape-first/direct-mesh;
- Work Planes e Profiles em espaço 3D conhecido;
- primitives e generators básicos, com **Capsule no Core V1**; Star fica fora do Core V1;
- Extrude/Push-Pull local;
- Cut/Difference fallback;
- Slice planar;
- Fuse/Union provider;
- Connect/Bridge/Weld;
- one-segment Bevel/Chamfer V1;
- Revolve V1;
- Simple Sweep V1.x limitado;
- triangles/quads/n-gons authoring + triangulation cache;
- half-edge/index topology encapsulada;
- normals derivados, Flat/Smooth/Sharp; Flat é default e Smooth é opção secundária V1;
- Unlit como opção secundária V1;
- Auto UV híbrido + xatlas fallback;
- seams, packing, texel density e stretch analysis;
- Project From Reference/View; Landmark Alignment fica V1.x;
- Base Color painting V1 com Palette, recent colors e Pixel Grid;
- editor 2D de textura opcional/fechado por padrão, compartilhando TextureBitmap e Undo com Paint 3D;
- material model glTF-like;
- `.petunia` ZIP + JSON authoring data;
- atomic save, Undo/Redo transacional e recovery snapshots;
- GLB/glTF + OBJ baseline;
- Game Ready Validator.

# Baseline técnica final

Os itens abaixo formam a baseline final e devem ser comprovados pelo vertical slice de conformance:

- **Rust 2024 como core language final**, substituindo Odin conforme ADR 32;
- **Slint 1.18 como UI toolkit ativa de produção; egui/eframe congelados apenas como fallback recuperável**;
- **wgpu 30.x** como backend do viewport, acessado por adapter toolkit-neutro; integração específica do shell Slint fica confinada ao frontend;
- Geometry Core próprio em Rust + `slotmap`;
- `geo` para Profile/polygon 2D quando necessário;
- `manifold-rust` para Fuse/Cut complexo;
- xatlas como Auto UV fallback atrás de provider;
- Lua 5.4 + `mlua` plugin API;
- Application API/Command Registry;
- MCP Rust via `rmcp`, com Tokio isolado;
- single-writer document + worker snapshots;
- testing/conformance strategy;
- extensibilidade como parte da V1;
- **Explicit Modular Data Architecture** do capítulo 34: dados concretos, ownership explícito, DOD seletivo, sem ECS universal, Tool→Command→Algorithm→Data, traits apenas em boundaries reais, long-lived relations por IDs, single-writer, snapshots, `unsafe` isolado e interoperability tests.

# Estado atual da UI

A fase de decisão de UI/UX foi **fechada como UI BASELINE FINAL V1**. Permanecem válidos viewport-first, painéis semi-flutuantes/retráteis, workspace pills reduzidas, `Parts`, `Context` selection-centric, toolbar contextual, segmented controls, overlays, Petunia Components, keyboard/focus/accessibility e viewport atrás de adapter toolkit-neutro. A implementação ativa desses contratos é Slint; integração egui-wgpu pertence apenas ao fallback congelado.

O capítulo 36 congela:

- shell, regiões, medidas de referência e breakpoints;
- layout/collapse/resize sem docking irrestrito;
- typography, UI scale, accent, radius e motion;
- input/gestures, focus order e shortcuts UX;
- fluxo Model/Paint/UV;
- viewport adapter e repaint policy;
- crates UI promovidas à baseline;
- Theme Extensions `.petunia-theme`;
- Plugin Panels Lua em slots controlados.

A partir daqui, pequenas calibrações como ajuste de poucos pixels, luminância ou timing são **tuning**, desde que preservem contrato, hierarquia e acessibilidade. Reabrir toolkit, grafo estrutural do shell, docking irrestrito, raw UI/GPU access de plugins ou invariantes de accessibility exige decisão arquitetural explícita.

## Especificação normativa de UI

Usar os capítulos 22–26 como base de referência/design system e **36 como autoridade final para decisões antes marcadas como UI-OPEN/PROTOTYPE**.

# Regra de mudança

Depois deste marco:

```
nova ideia técnica
→ verificar se resolve requisito existente
→ comparar com baseline
→ registrar delta/rationale
→ implementar somente se benefício superar custo
```

Nova feature de produto não entra por conveniência arquitetural.

# Regra para o framework

Ao gerar implementation bible/tasks:

1. tratar páginas 09, 12 e 21 como normativas para arquitetura/escopo;
2. tratar o **capítulo 34 como autoridade especializada para representação em código, Rust safety, DOD/ECS, Tool/Command/Algorithm e modularidade**;
3. tratar páginas 14–20 como contratos técnicos especializados históricos/funcionais;
4. tratar **27–32 como autoridade atual para a stack Rust e sua implementação**;
5. implementar a UI conforme os capítulos 24–26 e a autoridade final do capítulo 36; ajustes permitidos são tuning, não novos defaults arbitrários;
6. permitir implementação headless/testável do core antes do shell final;
7. exigir contradiction/delta check antes de alterar decisões congeladas;
8. considerar qualquer afirmação anterior de `Odin core` ou `Go MCP sidecar` como superseded pelo ADR 32.

# Fase atual

A fase de decisão de **escopo de produto V1 + core + UI Baseline V1 está encerrada**. A fase corrente é **implementação/conformance do vertical slice**, incluindo validação prática da arquitetura dos capítulos 34–36 em features reais. O vertical slice do capítulo 31 comprova integração, qualidade, performance, accessibility e fidelidade; ele só reabre a arquitetura quando revelar bloqueador estrutural comprovado.

[34 — Arquitetura Modular Explícita, Rust Safety e Representação em Código](34-arquitetura-modular-rust-safety.md) define a arquitetura de código Rust-safe vigente.

[32 — ADR: Migração da Baseline Odin para Rust](32-adr-odin-para-rust.md) registra formalmente o delta Odin → Rust.

# Regra final

**Escopo de produto V1, Core e UI baseline devem permanecer estáveis enquanto implementamos a experiência.** Só reabrir uma decisão congelada quando evidência concreta demonstrar bloqueador estrutural ou quando houver mudança explícita de produto.