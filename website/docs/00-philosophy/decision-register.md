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

## Guideline visual aprovado em 2026-10-11

Spectrum é a referência principal para a hierarquia de atenção da GUI. O usuário aprovou pílulas seletivas, color wheel preciso, ícones sucintos, superfícies neutras e relevo suave, preservando o macro-layout. O contrato visual canônico está em [Design system — guideline visual](../09-ui/design-system-guideline.md), acompanhado por U08. Aprovação de aparência não fecha aceitação nativa, acessibilidade, performance ou plataforma.

## Governança de mudanças

Toda revisão estrutural posterior documenta:
- problema concreto e evidência de código;
- opções avaliadas e razão para a escolha mais simples;
- impacto em documentos legados, API pública, Windows/Linux e acessibilidade;
- migração Reuse/Refactor/Move/Rewrite;
- testes e condições de rollback;
- atualização do manifest/site e status da implementação.

Decisão sem evidência de viabilidade não vira conclusão de testes. O caderno em **website/docs/** é a referência desta branch; **docs/bible/** é legado histórico e não recebe novas especificações desta refatoração.

## ADR-C10 — base funcional única e integração Qt (2026-10-13)

O usuário autorizou trazer o adapter Qt para o repositório da implementação funcional, evitando duas cópias independentes do motor. `refactor/architecture-foundation` é a linha funcional; `codex/qt-integration` trabalha em worktree do mesmo repositório. As features compartilhadas vivem em Core/Application/Geometry/Project/Renderer; Qt é consumidor desses contratos, não uma segunda implementação do domínio. Slint permanece a entrada padrão durante a integração. Aceitação de produção Qt, paridade, Windows/Linux, a11y e performance continuam independentes deste transporte. Protocolo e evidências: [Integração Qt](../09-ui/qt-integration.md).

## Proposta registrada — ferramentas e seleção dinâmica (2026-10-15)

O usuário solicitou documentar a auditoria de ferramentas/atalhos e a proposta de seleção com movimento direto, incluindo matriz com status de implementação sempre preenchido. A referência transversal é [Ferramentas compartilhadas — proposta e matriz de implementação](../02-application/tools-interaction-implementation.md); H01–H18 acompanham os deltas e D03 acompanha somente a entrega documental. Trata-se de proposta registrada e plano de execução, sem declarar UX ratificada, correções implementadas ou frontend aceito. A restrição atual de não alterar Qt/Slint permanece vigente. Os contratos existentes de Paint, UV, Animation e Commands continuam autoritativos em suas responsabilidades.

## ADR-C11 — Abandono e remoção total do Slint e consolidação em Qt 6 + petunia_editor (2026-10-15)

Por solicitação explícita do usuário ("analisar a implementação para abandono e remoção total da GUI em slint e de suas dependencias para não pesar mais o projeto"), a avaliação de substituição/viabilidade do Slint foi concluída com a decisão de **No-Go** e remoção definitiva de toda a infraestrutura Slint (`slint`, `slint-build`, crates `i-slint-*` e `crates/ui-slint/`) do repositório e do workspace Cargo.

Decisões arquiteturais e operacionais:
1. **Controlador neutro desacoplado (`crates/editor` / `petunia_editor`):** A máquina de estados, o gerenciamento de sessões de ferramentas, projeções de viewport e intents do editor (anteriormente `SlintUiBridge`) foram extraídos para o crate neutro `crates/editor` (`petunia_editor`). Este crate não depende de Slint, Qt, FFI ou GPU, servindo como o controlador headless único do produto.
2. **Purga completa das dependências Slint:** O crate `crates/ui-slint/` foi excluído do repositório, assim como todas as suas regras de lint e dependências no `Cargo.toml` raiz e `Cargo.lock`. Isso alivia o tempo de compilação, o consumo de disco e de memória durante o desenvolvimento.
3. **Frontend Qt 6 primário (`crates/ui-cxxqt`):** `crates/ui-cxxqt` passa a ser o frontend padrão do executável principal (`src/main.rs`), consumindo diretamente o `petunia_editor`. A execução da GUI passa a ocorrer via `petunia_ui_cxxqt::widgets::run()`.
4. **Isolamento de camadas (L6):** A UI em C++/Qt Widgets e QML permanece estritamente na camada de apresentação (L6), comunicando-se com `petunia_editor` através de DTOs e despachando comandos transacionais para a camada Application.

## Editor contextual, DRAW/POLY e ShapeBuilder avançado — 2026-10-15

O usuário aprovou incorporar SEL-001–007, VSP-001–006/MF-001–008 e RIG-001–012 do pacote de especificação ao produto. O [contrato integrado](../02-application/contextual-editor-contract.md) concilia essas decisões com os capítulos de domínio e a [matriz complementar H01–H33](../02-application/tools-interaction-implementation.md). Aprovação não comprova implementação.

Seleção usa gizmo integrado e **arrasto protegido**, substituindo a proposta inicial de mover ao arrastar geometria; modos globais de componentes deixam de ser requisito do fluxo, mas componentes e filtros permanecem. Pivô do objeto é sua origem local persistente; centro de transformação e Cursor3D são referências independentes, assim como navegação. Editar origem preserva o quadro atual, não promete preservar toda a animação. DRAW é preset planar de POLY, com retorno livre, ferramentas e perfis compartilhados. ShapeBuilder avança em composição reeditável, múltiplos sólidos e construção sobre superfícies, após a fundação de dois operandos.

Execução: núcleo/editor compartilhado primeiro, apresentação Qt/GL e gates visuais depois; sem restaurar Slint ou modificar GUI neste lote. Trabalho na branch funcional autorizada, sem merge em main. Histórico D03 e demais evidências permanece preservado; D04 cobre apenas a consolidação documental.

## ADR-C12 — Estratégia de testes em camadas e prevenção de regressão (Padrão QA Qt Creator / QTestLib) — 2026-10-15

Após análise da experiência de QA e automação E2E do Qt Creator IDE, o projeto ratifica formalmente a adoção dos princípios de garantia de qualidade demonstrados na prática (testes holísticos de ponta a ponta sob a perspectiva do usuário, prevenção contínua de regressões com reprodução obrigatória em testes sintéticos, fuzzing com dados hostis e bateria estruturada em camadas).

Decisões de engenharia e viabilidade de ferramentas:
1. **Inviabilidade do Squish GUI Tester:** Avaliado e rejeitado para este repositório. Embora funcional para a The Qt Company internamente, trata-se de ferramenta proprietária de alto custo (licenciamento corporativo por assento e por nó de CI), incompatível com o modelo aberto do Petunia3D e excessivamente pesada para os budgets estritos de CI e memória da máquina de desenvolvimento.
2. **Automação nativa aberta via `QTestLib` offscreen:** A automação de GUI adota o framework nativo e gratuito `QTestLib` do próprio Qt (`QTest::mouseClick`, `QTest::keyClicks`, `QSignalSpy`). A execução é isolada e serial em modo offscreen (`QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1`), dispensando displays virtuais pesados e garantindo acionamento real de widgets e atalhos.
3. **Fluxos de autoria E2E em memória (`petunia_editor::EditorController`):** Devido à arquitetura desacoplada L6 ("Everything is a Command"), os fluxos complexos de autoria (MODEL, DRAW, POLY, PAINT, UV, ANIMATE) são testados ponta a ponta no controlador neutro Rust (`editor_parity.rs`), com execução determinística em microssegundos e imune a flakiness de GUI.
4. **Proteção da Viewport OpenGL por Golden Frames FBO:** A integridade de shaders e rendering 3D no `QtGLWidget` (`QOpenGLWidget`) é protegida por capturas offscreen de FBO comparadas contra imagens PNG de referência com tolerância controlada.
5. **Prevenção contínua de regressões ("Prove"):** Toda correção de crash ou bug deve vir acompanhada de teste automatizado sintético que reproduzia a falha antes da correção.
6. **Acessibilidade como gate funcional e canal de teste:** Toda superfície expõe semântica para `QAccessible` e suporta ciclo completo de teclado (`F6`/`Tab`), viabilizando auditoria a11y contínua e automação externa gratuita via APIs de acessibilidade do SO (AT-SPI / pywinauto).


## ADR-C13 — Operation Graph: operações persistentes e re-editáveis (2026-10-17) — PROPOSTO

Direção aprovada pelo usuário em 2026-10-17; o contrato detalhado está em [Operation Graph](../03-geometry/operation-graph.md) e **não está implementado**. Reabre de forma controlada a restrição "sem node graph" da V1 (modifiers, geradores, materiais), preservando a deferência de GPU shader graph.

1. **Escopo inicial:** booleana e materiais/texturas. Nó de operação ligado ao objeto selecionado, com parâmetros, operação e saída; a saída liga a um objeto ou a outro nó.
2. **Simplicidade:** nenhum nó é necessário para começar; a mesma receita tem vista de Pilha (Inspector) e de Grafo (Gaveta). Tudo é comando, com Undo, teclado e a11y.
3. **Reuso:** extrair o núcleo genérico de `SurfaceRecipe`; `Graph(output)` como nova fonte de geometria avaliada e não editável até Bake; `BooleanOpCmd` vira Aplicar/Bake; `ModifierKind` migra para cadeia de nós.
4. **Modularidade:** `NodeDescriptor` estático com portas e parâmetros tipados; UI gerada do descriptor; extensão Lua posterior sob E01/E02; nó desconhecido carrega como nó fantasma.
5. **Pendências:** semântica de ligar a mais de um objeto (compartilhar × por objeto); formato de persistência e versão de schema; limites de avaliação.

## ADR-C14 — Shell v2 da GUI Qt: regiões fixas, layout versionado e construção em paralelo (2026-10-17) — PROPOSTO

Direção aprovada pelo usuário em 2026-10-17; detalhes em [Shell v2](../09-ui/shell-v2-proposal.md) e evidências em [dossiê](../09-ui/gui-audit-dossier.md). **Não implementado.**

1. **Regiões fixas por padrão** (AppBar, Rail, Viewport Header, Work Surface, Estrutura, Propriedades, Gaveta, Status) com layout do usuário persistido e versionado, conforme `newdocs/design-system`; sem nova biblioteca de dock (KDDockWidgets/Qt-ADS fora de escopo). Substitui a recomendação de KDDockWidgets da matriz Qt.
2. **Novo shell em paralelo**, atrás de `PETUNIA_SHELL=v2`, reaproveitando componentes sadios; troca única na paridade, remoção do legado depois.
3. **Onde cada coisa mora:** Rail, HUD, ação contextual, propriedade, nó/pilha, diálogo (regra em Shell v2 §3). Domínio de seleção só no Viewport Header; ações contextuais flutuantes (≤ 4 + Mais).
4. **Uma fonte por responsabilidade:** tokens, ações, ferramentas, seções de propriedade e strings por id; ponte tipada e seccionada por revisão.
5. **Mapeamento de ids:** UI-001…015 do pacote `newdocs` passam a ser rastreados por este ADR, sem aprovação retroativa de UI-011…015 (guardrails).
6. **Supera** os trechos dependentes de Slint em `09-ui` e nas rotas de `AGENTS.md`, cuja reescrita é pendência (U10).

## ADR-C15 — Shell v2: refinamento de uso (2026-10-18) — APROVADO PELO USUÁRIO

Pedido do usuário após uso real do shell v2; detalhes e evidências em [refinamento do Shell v2](../09-ui/shell-v2-refinement.md). Verificação visual e de plataforma continuam abertas.

1. **Booleanas só por nós.** Fuse/Cut/Intersect diretos, o operando global e Keep Parts foram removidos; o nó Boolean do Operation Graph é o único caminho e **Make Editable** (`graph_bake`) substitui o "Aplicar". **Supera** o item 3 do [ADR-C13](#adr-c13--operation-graph-operacoes-persistentes-e-re-editaveis-2026-10-17--proposto) (`BooleanOpCmd` como Aplicar/Bake). Join passa a fundir a seleção no objeto ativo.
2. **Um só contorno de seleção, no passe GL.** O contorno 2D projetado em Rust e pintado por QPainter foi removido (ficava para trás ao arrastar); o passe GL usa a cor de seleção de cada parte e o hover do objeto.
3. **Estrutura com grupos.** Grupos são as collections do documento (um nível); arrastar e soltar, Ctrl+G/Ctrl+Shift+G e isolamento são comandos com Undo. Hierarquia aninhada com transformação herdada segue em G09.
4. **Rail mínimo.** Só transformar, 3D Cursor e criação; operações de modelagem são ações contextuais agrupadas na barra flutuante.
5. **Navegação e gizmos.** Cubo de navegação (faces, arestas, cantos, bússola, início, projeção) no lugar das esferas de eixo; gizmos de transformação reformulados com tamanho e espessura nas Preferências.

## ADR-C16 — Shell v2: roteiro de Poly, Paint, UV, Animate, Nós e Assets (2026-10-18) — APROVADO PELO USUÁRIO

Debate técnico entre o usuário e o assistente (postura definida em `AGENTS.md`, "Postura técnica e processo de decisão"); o usuário aceitou as recomendações do assistente. Roteiro e status em [roteiro do Shell v2](../09-ui/shell-v2-roadmap.md).

1. **Cor de seleção: o mais específico vence** (objeto > grupo > global), com a opção **"forçar cor do grupo"** por grupo. Motivo: com a cor do grupo mandando sempre, o objeto ativo dentro do grupo ficaria igual aos irmãos. A cor do grupo é metadado do documento (campo opcional, sem quebrar arquivos antigos).
2. **O pivô é a origem do objeto** (zero local do TRS); mover o pivô é um *rebase* que **não muda a geometria no mundo** (verificado com tolerância antes de gravar) e tem Undo exato. **Correção do próprio ADR em 2026-10-18:** a proposta inicial era "um deslocamento de pivô separado, sem regravar a malha"; ao ler `origin.rs` ficou claro que o modelo canônico já atende o objetivo (Undo exato e preservar a cena) e que um segundo conceito de pivô complicaria gizmo, snap, animação e exportação. O rebase **recusa**, com motivo, objeto bloqueado, primitiva paramétrica ("Tornar editável" antes), com modificadores ou com rig. **Editar pivô** é uma sessão: liga o Mover, Enter confirma, Esc desfaz exatamente os passos da sessão e escolher outra ferramenta confirma. Menus de botão direito **Pivô** (resetar para o centro da geometria, base, origem do mundo, 3D Cursor, ponto sob o mouse, centro do vértice/aresta/face selecionado) e **Cursor 3D**.
3. **Uma só ferramenta "Projeção"**, que substitui "projeção" e "decal". A diferença real é o ciclo de vida: **manter editável** (decal) ou **aplicar na textura** (bake).
4. **Operações animadas só como parâmetros de nós.** Extrude, inset, push/pull por keyframe exigem a operação como nó com parâmetros animáveis (topologia que muda no tempo não interpola). Vértices, arestas e faces editados à mão são animados por morph targets (topologia estável).
5. **Registro de propriedades animáveis antes da timeline.** A timeline estilo motion design e a **aba de Curvas de animação** (editor de curvas) são visões do mesmo registro tipado (id, tipo, faixa, unidade); nenhuma das duas cria um modelo de dados próprio.
6. **Seletor de cor único** (`PetuniaColorPanel`) em todo o app: roda maior, outros modos, RGB/HSL com alfa, hex, paletas com criar/editar/apagar. A opacidade do brush passa a viver nele (a duplicata do painel de Paint sai junto, não antes).
7. **Efeitos como sub-camadas** da camada, ambos com máscara, com cache por camada e invalidação por tile (orçamento de hardware modesto).
8. **Nós como grafo acíclico (DAG)** com portas tipadas, cache por nó e detecção de ciclos; um nó de objeto pode alimentar várias operações sem nó especial. UV e pintura à mão **não** viram nós.
9. **`3D-Nav-Viz5.gif`** é só referência visual do usuário: fica fora do repositório para sempre (excluído localmente em `.git/info/exclude`), sem entrada em `ATTRIBUTION.md`.
