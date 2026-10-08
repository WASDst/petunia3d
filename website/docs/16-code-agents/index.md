# Code Agents — guia de entrada

> **Status: diretivas operacionais aprovadas em 2026-10-08.** A documentação canônica de produto/arquitetura permanece nos capítulos anteriores; esta seção ensina como agentes a encontram e a implementam, sem criar uma segunda fonte de verdade.

## Objetivo em três linhas

Um agente deve **entender a decisão aprovada**, **inspecionar a implementação real** e **modificar somente a diferença necessária**. Cada entrega deixa código, testes e documentação coerentes, com evidência verificável e sem inventar gates verdes. Ações são executadas apenas na branch autorizada.

## Comece nesta ordem

1. [AGENTS.md na raiz](https://github.com/WASDst/petunia3d/blob/refactor/architecture-foundation/AGENTS.md) — escopo, branch, segurança e regras curtas.
2. [Como ler e navegar](./reading-navigation.md) — hierarquia de autoridade, contexto progressivo, conflitos e rastreabilidade.
3. [Protocolo de implementação](./implementation-protocol.md) — gap matrix, plano mínimo, implementação, testes e handoff.
4. Consulte **somente o domínio envolvido** no [Registro de decisões](../00-philosophy/decision-register.md) e no manifesto deste site.
5. [Playbooks por domínio](./domain-playbooks.md) — mapa de especialidades, dependências e gates.
6. [Prompts operacionais](./prompt-library.md) — templates de task, correção, UI, Paint, Geometry, GL, a11y, documentação e revisão.
7. [Orquestração e papéis](./agent-orchestration.md) — Prumo agents, seleção responsável, review independente.
8. [Catálogo completo do Prumo](./workforce-index.md) — todos os Agents, Skills e Recipes com links individuais e proveniência.
9. [Evidências e entrega](./evidence-handoffs.md) — formato de saída, critérios de aceitação e continuidade entre sessões.

## Regras que resolvem 90% dos erros

- **Documento aprovado ≠ código já implementado.** Confirme chamadas reais, wiring, features, manifests e testes.
- **Não invente stack.** Slint + OpenGL 3.3 são a direção aprovada; não expandir egui/WGPU por conveniência.
- **Não replique domínio na UI.** Tool → Intent/Command → Application → Geometry/Project/Paint/UV/Animation; renderer consome, não muta.
- **Não faça big-bang rewrite.** Corrija menor delta testável, preserve projetos antigos.
- **Não degrade acessibilidade para aumentar FPS.** Foco, teclado, labels, contraste, motion, escala e input alternativo são requisitos.
- **Não execute habilidades externas às cegas.** Leia \`SKILL.md\` ou \`AGENT.md\` e examine scripts; manifest descreve sugestão de capacidades, não consentimento automático.
- **Não declare testes não executados como aprovados.** \`not run\`, \`blocked\` e evidência são estados honestos.

## Como usar esta seção

**Para uma implementação:** [Navegar](./reading-navigation.md) → [Playbook](./domain-playbooks.md) → [Prompt](./prompt-library.md) → [Protocolo](./implementation-protocol.md) → [Entrega](./evidence-handoffs.md).

**Para uma auditoria:** inspeção do código + [matriz de lacunas](./implementation-protocol.md) + roles \`explorer\`, \`reviewer\`, \`tester\` e especialistas pertinentes, sem escrever código no papel read-only.

**Para trabalho especializado:** [Índice do Prumo](./workforce-index.md). Abra **somente** os arquivos individuais pertinentes; catálogo não é autorização para executar todos os agentes.

## Atalhos normativos

- [Filosofia](../00-philosophy/refactor-principles.md) · [Decisões](../00-philosophy/decision-register.md)
- [Slint Rescue](../09-ui/slint-rescue-plan.md) · [Viewport Input](../09-ui/viewport-input-boundary.md)
- [Conformance e CI](../14-quality/testing-ci-conformance.md) · [Performance](../14-quality/performance-jobs-budgets.md)
