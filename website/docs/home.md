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
