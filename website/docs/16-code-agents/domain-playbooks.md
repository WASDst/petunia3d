# Playbooks por domínio — rota rápida para tarefas

> Cada linha é um ponto de entrada, não substitui a leitura do capítulo canônico correspondente. Se skill Prumo sugerir stack divergente, **a stack e os contratos Petunia3D prevalecem**.

| Tipo de trabalho | Leia primeiro | Agents recomendados | Skills inicialmente relevantes | Evidência-chave |
|---|---|---|---|---|
| Shape-first / Geometry / Drawing | [DRAW/POLY](../03-geometry/draw-unification-proposal.md), [Shape Builder](../03-geometry/shape-builder-3d.md) | explorer, architect, engine-engineer, tester | lang-rust, architecture-quality, clean-code, physics-collision quando couber | determinismo, topologia, Undo, legacy |
| Snapping / input / gizmo | [Inference](../03-geometry/snapping-inference-workplane.md), [Viewport Input](../09-ui/viewport-input-boundary.md) | editor-engineer, ui-component-engineer, accessibility-reviewer | input-handling, keyboard-accessibility, focus-management | keymaps, cancel, cursor, HiDPI |
| Slint GUI / Design System | [Slint Rescue](../09-ui/slint-rescue-plan.md), [Feedback+A11y](../09-ui/workspaces-feedback-accessibility.md) | ux-architect, design-system-engineer, ui-component-engineer, accessibility-reviewer | accessibility, cognitive-clarity, design-system, design-tokens, interaction-design | teclado/F6, focus, 100–200%, contraste |
| OpenGL / viewport / shader | [Renderer](../04-renderer/architecture-decision.md), [Performance](../14-quality/performance-jobs-budgets.md) | renderer-engineer, engine-engineer, performance-agent | rendering-3d, shaders, lang-rust, lang-glsl, benchmarking | GPU ownership, resize, state restore, low-end |
| PAINT / decals | [Paint](../06-paint/architecture-decision.md), [Decals](../06-paint/decals-animated-decals.md), [PAINT UX](../06-paint/workspace-ux.md) | editor-engineer, engine-engineer, ux-architect, tester | editor-tooling, asset-pipeline, color-science, accessibility | stroke Undo, layers, dirty tiles, channels |
| UV / FaceCorners | [UV architecture](../07-uv/architecture-decision.md), [UV UX](../07-uv/workspace-ux.md) | engine-engineer, editor-engineer, quality-reviewer | applied-mathematics-dsp, testing-quality, performance-native | unwrap/pins/seams/island selection/round-trip |
| Reference Projection | [Reference/Projection](../08-reference-projection/architecture-decision.md) | editor-engineer, renderer-engineer, ux-architect | rendering-3d, interaction-design, color-science | preview/project/bake não destrutivo |
| ANIMATE / rigs | [ANIMATE UX](../10-animation/workspace-ux.md) | engine-engineer, motion-designer, accessibility-reviewer | motion-library, physics-collision, input-handling | procedural-first, Undo/preview, timeline honesty |
| Project / Assets / Export | [Project](../11-project/project-persistence.md), [Assets](../11-project/assets-resource-management.md), [Interchange](../12-interchange/import-export-game-ready.md) | architect, backend-engineer, security-reviewer, tester | serialization, asset-pipeline, filesystem-security, untrusted-project-security | atomic save, recovery, malformed data, IDs |
| Lua Plugins / MCP | [Lua API](../13-extensions/lua-plugins-public-api.md), [MCP](../13-extensions/mcp-automation-agents.md) | security-architect, backend-engineer, reviewer | plugin-architecture, plugin-security, mcp-security, api-contract-testing | deny-by-default, rollback, stale IDs |
| CI / distribution / performance | [Conformance](../14-quality/testing-ci-conformance.md), [Platforms](../15-platform/build-distribution-compatibility.md) | devops-engineer, release-verifier, performance-agent | ci-cd, github-ci-debug, release-engineering, supply-chain-security | commands, jobs, versions, hardware matrix |
| Docs for agents | [Navegação](./reading-navigation.md), [Index](./index.md) | documentation-maintainer, reviewer | documentation-for-llms, lean-progressive-context, cognitive-clarity | links, canon, token budget, no drift |

Para cada linha, os **links dos arquivos das Skills e Agents** são fornecidos no [Catálogo Prumo](./workforce-index.md); não se presumem instalados localmente.

## Sobre padrões gerais e exceções

\`game-engine-architecture\` contém material sobre ECS; isso **não autoriza introduzir ECS** no Petunia3D. Skills \`frontend-web\` ou \`playwright-ui\` podem ajudar o **site de documentação**, mas não são base da UI Slint. \`screen-reader\`, \`zoom-reflow\` e WCAG precisam de adaptações verificadas às APIs nativas Slint/Windows/Linux; não tratar guia de DOM/ARIA como implementação desktop automática.

## Quando acrescentar especialista

- **Acessibilidade:** sempre que mudar UI, viewport input, estados, diálogo, tutoriais ou texto.
- **Segurança:** sempre que entrar arquivo externo, script/plugin, MCP, path, import ou export.
- **Performance:** quando mudar hot path, renderer, cache, concurrency, paint stroke ou layout crítico.
- **Documentation:** quando mudar contrato, arquitetura, semântica, compatibilidade ou novo comportamento.
- **Design/Research:** quando ferramenta alterar fluxo mental, descoberta, naming ou carga cognitiva.
