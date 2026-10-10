# Prumo Workforce — índice completo e verificável

> **Fonte oficial**: [poppy-lat/prumo](https://github.com/poppy-lat/prumo) · [diretório workforce](https://github.com/poppy-lat/prumo/tree/main/src/prumo/resources/workforce) · [snapshot verificado](https://github.com/poppy-lat/prumo/tree/e213260c99d22c89bf31890ec595a89725031c71/src/prumo/resources/workforce) em 2026-10-08.
>
> Inventário integral: **39 agents, 189 skills, 20 recipes**. Cada entrada abaixo ou nas páginas vinculadas aponta ao arquivo real `AGENT.md`, `SKILL.md` ou `RECIPE.md` + manifesto correspondente. **Não afirmar que os papéis estão instalados ou ativos localmente apenas por constarem neste índice.**

## Como usar

1. Leia [Protocolo](./reading-navigation.md) e [Orquestração](./agent-orchestration.md).
2. Selecione **o menor subconjunto** apropriado à tarefa. Regras do Petunia3D prevalecem sobre orientações genéricas de skills.
3. Abra o arquivo `AGENT.md` ou `SKILL.md` individual e suas referências/checklists quando necessários.
4. Verifique manifesto, permissões e execução antes de instalar/copiar/rodar scripts. Nada é executado automaticamente.
5. Reporte quais versões/roles foram realmente usados na entrega.

## Todos os agentes

[**39 agentes, com link individual e manifesto**](./agents-catalog.md) — arquitetos, implementadores, testes, revisão, engines, Slint/UI, acessibilidade, design, segurança, documentação e especialidades.

## Todas as skills (189)

| Catálogo | Especialidade |
|---|---|
| [Engenharia, qualidade, documentação e workflows](./skills-foundations.md) | Clean Code, arquitetura, debugging, context engineering, docs for LLMs, Git, CI, agents |
| [Acessibilidade, neurodivergência, UX e design](./skills-experience.md) | Cognition, Focus, Keyboard, Screen Reader, WCAG, ergonomia, design system, identidade |
| [Editor, engine, renderer, multimídia](./skills-engine-graphics.md) | Geometry/editor patterns, rendering, shaders, assets, Paint, animation, real-time |
| [Segurança, privacidade e release](./skills-security-delivery.md) | MCP/plugin, trust boundaries, file/project handling, isolamento e threat model |
| [Linguagens, compiladores e especialidades](./skills-languages-specialized.md) | Rust, Slint-adjacent, GLSL, Lua, TypeScript e toda a biblioteca de linguagens |

## Todas as receitas

[**20 recipes, com link individual e schema**](./recipes-catalog.md) — feature-standard, bug-fix, architecture-change, engine-renderer, ui-feature, ui-review, docs e complementares.

## Seleção recomendada para Petunia3D

- **Base comum:** `grounded-implementation`, `implementation-reality-verification`, `clean-code`, `code-quality`, `testing-quality`, `lang-rust` e `documentation-for-llms` conforme tarefa.
- **Acessibilidade e neurodivergência:** `accessibility`, `cognitive-clarity`, `keyboard-accessibility`, `focus-management`, `screen-reader`, `motion-accessibility`, `zoom-reflow`, `design-psychology`. Sempre relevantes para UI, mas não executar todos os scripts web no desktop.
- **UI/design:** `design-system`, `design-tokens`, `component-specification`, `interaction-design`, `visual-qa`, `icon-system`, `typography-system`.
- **Geometry/render:** `game-engine-architecture`, `editor-tooling`, `rendering-3d`, `shaders`, `lang-glsl`, `performance-native`, `benchmarking`. Contrato Petunia **proíbe introduzir ECS sem necessidade justificada**.
- **IA, automation, code agents:** `prumo-navigation`, `lean-progressive-context`, `prompt-engineering`, `agentic-workflow-design`, `orchestration-multi-agent`, `mcp-security`.

## Política de integridade do inventário

O inventário foi obtido dos arquivos reais do Git tree da branch `main` do Prumo na revisão `e213260c99d22c89bf31890ec595a89725031c71`, verificando existência de `AGENT.md`, `SKILL.md`, `RECIPE.md` e metadados. Fonte machine-readable: [workforce-catalog.json](./workforce-catalog.json). O `main` do Prumo pode mudar: para reproduzir, use o snapshot SHA; para consultar a versão mais recente, abra os links individuais em `main`.

**Não confundir**: Prumo define papéis/processo; Petunia3D define produto, arquitetura, permissões concretas e critérios de aceitação. Nenhum agent de Prumo ganha permissão de tocar `main` do Petunia3D.
