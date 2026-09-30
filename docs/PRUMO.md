# Prumo — Petunia3D (Intent Router)

Roteador central de intenção e mapa canônico de navegação do projeto **Petunia3D** para humanos e agentes autônomos.

> 🧊 **Site público de documentação CONGELADO** até o fim do desenvolvimento do projeto
> (`docs/.vitepress/**`, `docs/index.md`, `docs/public/**`, `docs/image-references/**`,
> workflow de deploy). A fonte única da verdade é o **Livro Vivo** em
> [`docs/bible/`](bible/index.md). Ver [`AGENTS.md`](../AGENTS.md) §1.

---

## 0. Fonte Canônica (Livro Vivo)

- [Petunia3D — Livro Vivo](bible/index.md) — raiz canônica: visão, escopo, arquitetura, UI Baseline, stack e governança.
- [Especificações P3D — Readiness e Gauntlet Waves](bible/especificacoes-p3d-readiness-gauntlet-waves.md) — catálogo P3D-001 a P3D-174, epics, readiness e waves.
- [Status de implementação das especificações](bible/status/p3d-implementation-status.md) — artefato operacional (não é autoridade documental).
- [Auditoria de conformidade documental](audits/bible-conformance/README.md) — o que em `docs/` divergia do caderno e as sessões de correção.

---

## Estado Operacional Canônico

- [Estado Atual do Projeto](../PROJECT_STATE.md) — Fase ativa, metas e ordem de recuperação
- [`prumo.json`](../prumo.json) — Manifesto canônico do projeto e perfis tecnológicos
- [Contratos de Documentação (M5)](contracts/bindings.json) — Mapeamento formal de conformidade de documentação

---

## 1. Quero Usar o Produto (Usuário / Artista 3D)

- [Visão Geral e Introdução](../README.md) — O que é o Petunia3D e como começar
- [Manual de Uso e Navegação](manual/usage.md) — Controles de viewport, modos de seleção e atalhos
- [Workspaces e Design System](ui/README.md) — workspaces MODEL, PAINT e UV e seus fluxos
- [Guia de Instalação e Requisitos](manual/installation.md) — Instalação via Cargo, requisitos de GPU e binários
- [Referência de Atalhos](../../assets/keybinds/petunia.toml) — Arquivo TOML configurável de atalhos de teclado

---

## 2. Quero Desenvolver e Contribuir (Engenheiro de Software)

- [Padrões de Código e Engenharia](development/coding-standards.md) — Diretrizes de Rust, `rustfmt` e `clippy`
- [Estratégia de Testes e Conformance](development/testing-strategy.md) — Pirâmide de testes, gates e validações
- [Plano e evidências de interação premium](development/premium-interaction-plan.md) — auditoria atual e critérios pendentes
- [Plano de Context, workspaces e viewport](development/workspace-inspector-implementation-plan.md) — arquitetura compartilhada e próximo recorte MODEL/viewport
- [Relatório de Evidências Gauntlet](GAUNTLET.md) — Histórico de rodadas de validação, benchmarks e métricas
- [Governança do Repositório](governance/repository-governance.md) — Políticas de branch, commits convencionais e PRs
- [Contrato de Segurança e Modelo de Confiança](security/security-contract.md) — Limites de confiança e sanitização de I/O
- [Modelagem de Ameaças (STRIDE)](security/threat-model.md) — Análise de riscos para modelador desktop

---

## 3. Arquitetura e Decisões de Engenharia

- [Topologia e Grafo de Crates](ARCHITECTURE.md) — Arquitetura de 14 crates acíclicos e render-on-demand
- [Visão Geral de Arquitetura](architecture/overview.md) — Camadas de domínio, aplicação e adaptadores
- [Contrato de Clean Code](architecture/clean-code-contract.md) — Princípios de separação de responsabilidades
- [Registros de Decisões Arquiteturais (ADRs)](architecture/adr/README.md):
  - [ADR 001: Linha de Base Arquitetural](architecture/adr/001-architecture-baseline.md)
  - [ADR 32: Migração da Baseline Odin para Rust](bible/foundations/32-adr-odin-para-rust.md)
- [Petunia3D — Livro Vivo](bible/index.md) — 258 páginas canônicas (00–16, 01–46, P3D-001–174, seções A–O, adendos):
  - [Workflow Shape-First](bible/foundations/02-workflow-modelagem-shape-first.md)
  - [Geometria e Topologia](bible/foundations/03-geometry-core-faces-topologia.md)
  - [Combine, Fuse, Weld e Personagens](bible/foundations/04-combine-fuse-weld-personagens.md)
  - [Viewport, Shading e Modos de Visualização](bible/foundations/05-viewport-shading-modos-visualizacao.md)
  - [Arquitetura Modular Explícita, Rust Safety](bible/foundations/34-arquitetura-modular-rust-safety.md)
  - [Design System Visual: Tokens, Hierarquia e Estados](bible/foundations/24-design-system-tokens-estados.md)
  - [UI Baseline Final V1, Temas e Plugin Panels](bible/foundations/36-ui-baseline-temas-plugin-panels.md)
  - [MCP API, Automação e Integração com Agentes de IA](bible/foundations/11-mcp-api-automacao-agentes.md)

---

## 4. Quero Operar e Suportar (Operador / Release)

- [Ciclo de Vida de Instalação](operations/installation-lifecycle.md) — Procedimentos de atualização, rollback e desinstalação
- [Guia de Deploy e Empacotamento](operations/deployment.md) — Compilação release, validação de dependências e distribuição
- [Observabilidade e Diagnósticos](operations/observability.md) — Logs `RUST_LOG`, diagnóstico de drivers e benchmarks
- [Referência da Linha de Comando (CLI)](reference/cli.md) — Variáveis `PETUNIA_BACKEND`, flags e códigos de saída

---

## 5. Sou um Agente de IA (Protocolo Lean Progressive Context)

1. Leia [`ENTRYPOINT.md`](../ENTRYPOINT.md), [`PROJECT_STATE.md`](../PROJECT_STATE.md) e este `docs/PRUMO.md`.
2. Identifique a meta ativa em `.ai/goals/`.
3. Carregue **apenas o contexto mínimo suficiente** para a tarefa específica.
4. Respeite as barreiras arquiteturais: `core` não conhece crates de módulos concretos; `render-gl` e `render-wgpu` são os únicos que tocam GPU.
5. Nunca enfraqueça os critérios de aceitação de testes nem suprima erros em silêncio.
6. Mantenha sincronizados código, testes e documentação através de deltas.
7. Registre evidências e telemetria antes de finalizar a execução.

---

## Metas e Inteligência

- **Metas do Projeto**: Localizadas em `.ai/goals/<fase>/` (ex: `.ai/goals/P00/`).
- **Inteligência Durável**: Métricas operacionais e consumo em `.prumo/history/project-intelligence.json`.
