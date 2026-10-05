# Petunia3D — Mapa de Navegação

Roteador central de intenção e mapa de navegação do projeto **Petunia3D** para humanos e agentes.

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

## Estado Operacional

- [Estado Atual do Projeto](../PROJECT_STATE.md) — fase ativa, metas e ordem de recuperação.
- [Regras para agentes](../AGENTS.md) — normas obrigatórias para qualquer code agent.

---

## 1. Quero Usar o Produto (Usuário / Artista 3D)

- [Visão Geral e Introdução](../README.md) — o que é o Petunia3D e como começar
- [Manual de Uso e Navegação](manual/usage.md) — controles de viewport, modos de seleção e atalhos
- [Workspaces e Design System](ui/README.md) — workspaces e seus fluxos (baseline atual no cap. 36 do Livro Vivo)
- [Guia de Instalação e Requisitos](manual/installation.md) — instalação via Cargo, requisitos de GPU e binários
- [Perfis de Atalhos](../assets/keymaps/README.md) — perfis TOML configuráveis de atalhos de teclado

---

## 2. Quero Desenvolver e Contribuir (Engenheiro de Software)

- [Padrões de Código e Engenharia](development/coding-standards.md) — diretrizes de Rust, `rustfmt` e `clippy`
- [Estratégia de Testes e Conformance](development/testing-strategy.md) — pirâmide de testes, gates e validações
- [Plano e evidências de interação premium](development/premium-interaction-plan.md) — auditoria atual e critérios pendentes ([evidências](audits/premium-evidence/))
- [Plano de Context, workspaces e viewport](development/workspace-inspector-implementation-plan.md) — arquitetura compartilhada e recorte de viewport
- [Relatório de Evidências Gauntlet](GAUNTLET.md) — histórico de rodadas de validação, benchmarks e métricas
- [Governança do Repositório](governance/repository-governance.md) — políticas de branch, commits convencionais e PRs
- [Contrato de Segurança e Modelo de Confiança](security/security-contract.md) — limites de confiança e sanitização de I/O
- [Modelagem de Ameaças (STRIDE)](security/threat-model.md) — análise de riscos para modelador desktop

---

## 3. Arquitetura e Decisões de Engenharia

- [Topologia e Grafo de Crates](ARCHITECTURE.md) — grafo de crates acíclico e render-on-demand
- [Visão Geral de Arquitetura](architecture/overview.md) — camadas de domínio, aplicação e adaptadores
- [Contrato de Clean Code](architecture/clean-code-contract.md) — princípios de separação de responsabilidades
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

- [Ciclo de Vida de Instalação](operations/installation-lifecycle.md) — procedimentos de atualização, rollback e desinstalação
- [Guia de Deploy e Empacotamento](operations/deployment.md) — compilação release, validação de dependências e distribuição
- [Observabilidade e Diagnósticos](operations/observability.md) — logs `RUST_LOG`, diagnóstico de drivers e benchmarks
- [Referência da Linha de Comando (CLI)](reference/cli.md) — variáveis `PETUNIA_BACKEND`, flags e códigos de saída

---

## 5. Sou um Agente de IA

1. Leia [`AGENTS.md`](../AGENTS.md), [`PROJECT_STATE.md`](../PROJECT_STATE.md) e este `docs/NAVIGATION.md`.
2. Abra a página relevante do [Livro Vivo](bible/index.md) e, depois, o código e os testes envolvidos.
3. Carregue **apenas o contexto mínimo suficiente** para a tarefa específica.
4. Respeite as barreiras arquiteturais: domínio não conhece toolkit de UI; apenas os adapters de render tocam GPU.
5. Nunca enfraqueça os critérios de aceitação de testes nem suprima erros em silêncio.
6. Mantenha sincronizados código, testes e documentação.
