# Registro de decisões — arquitetura, escopo e execução

> **Status: consolidação aprovada em 2026-10-08.** Usuário autorizou o fechamento das decisões faltantes segundo a filosofia previamente estabelecida. Este registro distingue **decisão de projeto**, **implementação** e **evidência de teste**.

## Princípios inegociáveis

- Reuse → Refactor → Move → Rewrite como último recurso; jamais big-bang rewrite.
- Core/Geometry/Document/Application/Renderer/UI com ownership explícito e uma única autoridade de dados.
- Low-poly, Shape-first, authoring intuitivo e sem complexidade de DCC generalista desnecessária.
- Windows/Linux, GPU modesta, funcionamento offline e arquivo de projeto portátil.
- Acessibilidade para teclado, leitores de tela e neurodivergência é hard gate.
- Slint + OpenGL 3.3 é plano aprovado **condicionado ao Rescue Go/No-Go**; egui é fallback congelado, não frontend mantido em paralelo.
- UI, plugins e MCP emitem intents/commands; nenhum altera Mesh/Document diretamente.
- Não criar funcionalidades especulativas para preencher arquitetura genérica.

## Ledger de decisões fechadas nesta rodada

| ID | Área | Decisão | Documento |
|---|---|---|---|
| ADR-C01 | DRAW/POLY | Perfis sobre um mesmo motor; DRAW planar e POLY espacial | [Unificação DRAW/POLY](../03-geometry/draw-unification-proposal.md) |
| ADR-C02 | Document | Authoring persistente separado de EditorSession e Infrastructure | [Project/Persistência](../11-project/project-persistence.md) |
| ADR-C03 | Assets | SceneObject é instância, Asset é reuso, stores tipadas com IDs | [Assets](../11-project/assets-resource-management.md) |
| ADR-C04 | Import/Export | Pipeline único em snapshot; GLB principal, OBJ secundário; validação | [Interchange](../12-interchange/import-export-game-ready.md) |
| ADR-C05 | Extensões | Lua 5.4 via mlua e capabilities; módulos internos não são plugins | [Lua Plugins](../13-extensions/lua-plugins-public-api.md) |
| ADR-C06 | Automação | MCP é adapter semântico da Application com autorização e Undo | [MCP](../13-extensions/mcp-automation-agents.md) |
| ADR-C07 | Qualidade | Gates com execução/evidência obrigatórias; Slint/a11y/GL são hard gates | [Testing](../14-quality/testing-ci-conformance.md) |
| ADR-C08 | Performance | single writer, jobs versionados, caches derivados e render-on-demand | [Performance](../14-quality/performance-jobs-budgets.md) |
| ADR-C09 | Platform | Windows/Linux; GL 3.3; compatibilidade e release evidence | [Distribuição](../15-platform/build-distribution-compatibility.md) |

## Contratos de produtos já aprovados anteriormente

- **Modeling/Geometry:** ObjectGeometry canônica, FaceCorner UV, Shape Builder, Spline/PlanarShape, generators, modifier stack pequena, snapping/inference, transforms, normals, validation/cleanup.
- **Renderer:** OpenGL 3.3, glow, FBO/texture e integração Slint sem readback por frame.
- **Materials/Paint/UV:** stores e bindings tipados; layers, decals animados, paint 3D/2D, UV Editor work surface.
- **Reference/Projection:** ReferenceSets/Views, Trace & Build, Perspective Match, Project Photo e multi-view projection.
- **UI:** macro-layout, Regions/F6, feedback, numeric fields, tooltips, keyboard, reduced motion, high contrast.
- **Animation:** procedural-first; rig/posing/keyframes são capacidades incrementais, não promessas de implementação atual.

## Decisões de escopo — o que fica explicitamente ADIADO

Estas não são "pendências de arquitetura sem resposta"; são **não objetivos da baseline**:

| Assunto | Decisão | Condição para reabrir |
|---|---|---|
| Extensão de Modifier Stack | Manter pequena; não absorver Boolean/Shape Builder | algoritmo existente + valor ao usuário + baixo custo + teste |
| Graph Editor completo, Tweak Layers e Ghosts | Fases futuras de ANIMATE | procedural UX consolidada + dados de rig/clip e casos reais |
| GPU procedural shader/editor de nodes complexo | Fora da baseline | requisitos demonstrados sem bloat |
| FBX como formato principal | Fora da V1 | demanda real e integração viável |
| Native third-party plugin / DLL | Fora da V1 | modelo de segurança, ABI e suporte sustentável |
| Marketplace e cloud/login | Fora da V1 | objetivo concreto sem prejudicar offline-first |
| ECS, scene graph universal, abstraction-first | Não adotados | evidência de insuficiência dos contratos existentes |
| Segundo frontend/renderer de produção | Não manter egui/WGPU em paridade | falha estruturada no gate Slint/GL, documentada |
| Limites numéricos de FPS/memória | Não inventar; estabelecer por medição | benchmarking reproduzível em hardware-alvo |

## Estado de implementação é independente

Aprovar ADR-C01...C09 **não** informa que um crate, feature, UI ou pipeline está pronto. Estado real de entrega deve usar:

- **Proposed:** ideia ainda não ratificada.
- **Approved:** decisão de produto/arquitetura.
- **In migration:** trabalho de código iniciado, sem gate completo.
- **Implemented, unverified:** existe código, ainda carece de evidência.
- **Verified:** testes automatizados/manuais requeridos comprovadamente passam.
- **Blocked:** impedimento reproduzível documentado.

**Nunca usar "concluído" por mera presença de documentação, teste escrito, PR aberto ou compilação parcial.**

## Sequência de execução

1. Fechar Slint Rescue com equivalência de interface/gestos, boundaries e OpenGL.
2. Concluir migração Project/Asset/Document e single-writer Application.
3. Consolidar geometry DRAW/POLY, FaceCorner, generators e modelagem shape-first.
4. Validar Paint/UV/Reference Projection e primeiro fluxo Animate procedural.
5. Implementar adapters de extensibilidade/interchange sem duplicar domínio.
6. Executar gates Windows/Linux, low-end, persistência, a11y e export.
7. Só então considerar release/go-no-go e expansões de escopo.

**Ordem indica dependências prioritárias, não licença para apagar recursos existentes ou bloquear trabalho independente.**

## Governança de mudanças

Toda revisão estrutural posterior documenta:
- problema concreto e evidência de código;
- opções avaliadas e razão para a escolha mais simples;
- impacto em documentos legados, API pública, Windows/Linux e acessibilidade;
- migração Reuse/Refactor/Move/Rewrite;
- testes e condições de rollback;
- atualização do manifest/site e status da implementação.

Decisão sem evidência de viabilidade não vira conclusão de testes. O caderno em **website/docs/** é a referência desta branch; **docs/bible/** é legado histórico e não recebe novas especificações desta refatoração.
