# Petunia3D — Refatoração arquitetural

Este site é o caderno vivo da revisão completa do Petunia3D.

A meta não é reescrever por reescrever. A meta é recuperar uma arquitetura pequena, compreensível e sustentável, preservando o máximo de código existente que seja coerente, legível, testável e tecnicamente saudável.

## Prioridades

- Windows e Linux primeiro.
- Hardware low-end como requisito real.
- Menos dependências e menos backends simultâneos.
- Uma única fonte de verdade por responsabilidade.
- Separação clara entre Core, Geometry, Project, Application, Render e UI.
- Acessibilidade e neurodivergência desde a arquitetura.
- Código autoexplicativo, documentação instrutiva e nomes completos.
- Newtypes e aliases quando tornarem contratos mais explícitos.
- Reescrita somente quando corrigir a estrutura existente custar mais que substituir.

## Ordem inicial

1. Core.
2. Application e sessões.
3. Geometry e topologia.
4. Commands, Undo/Redo e Tool Sessions.
5. Project, persistência e assets.
6. Renderização.
7. Paint e UV.
8. Plugins, Lua e MCP.
9. GUI e Petunia Design System.
10. Build, testes e consolidação final.

## Decisões consolidadas — 2026-10-08

A etapa de decisões arquiteturais está fechada para a baseline descrita no [Registro de decisões](./00-philosophy/decision-register.md). **A implementação continua incremental e ainda depende de testes e gates**.

### Contratos transversais

- [DRAW/POLY — um Modeling Engine](./03-geometry/draw-unification-proposal.md): dois perfis, sem geometria/seleção duplicadas.
- [Project e Recovery](./11-project/project-persistence.md): dados autorais, versões, save atômico e recuperação.
- [Assets e Library](./11-project/assets-resource-management.md): IDs e recursos compartilhados sem segundo SceneObject.
- [Import, Export e Game-Ready](./12-interchange/import-export-game-ready.md): validação e export em cópia.
- [Lua Plugins](./13-extensions/lua-plugins-public-api.md) e [MCP](./13-extensions/mcp-automation-agents.md): capacidades autorizadas sobre Application API.
- [Testing & Conformance](./14-quality/testing-ci-conformance.md), [Performance](./14-quality/performance-jobs-budgets.md) e [Distribuição](./15-platform/build-distribution-compatibility.md): gates mensuráveis.

A decisão Slint + OpenGL permanece **aprovada como direção, não como gate final validado**. A GUI e os workspaces continuam seguindo os contratos já documentados.

