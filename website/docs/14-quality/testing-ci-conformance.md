# Testes, CI, conformance e gates de aceitação

> **Status: aprovado em 2026-10-08 como contrato alvo.** Não interpretar este documento como relatório de testes executados. Cada gate exige execução, link e evidência em commit/runner.

## Princípio

**Decisão arquitetural aprovada não significa implementação concluída.** Cada feature avança pela cadeia: especificação → código → testes automáticos → validação manual/visual → evidência → aceita. Não declarar conformidade de UI, GPU, acessibilidade ou export antes da execução correspondente.

Após a consolidação em 2026-10-15 (ADR-C11 e ADR-C12), a autoridade desta branch é **`petunia_editor` (controlador neutro) + Qt Widgets (`petunia_ui_cxxqt`) + OpenGL 3.3/glow/FBO**. A estratégia de garantia de qualidade adota as lições comprovadas de QA do Qt Creator IDE (testes holísticos do ponto de vista do usuário final, prevenção contínua de regressões e fuzzing), adaptadas para o ecossistema aberto e de baixo consumo de recursos do Petunia3D.

## Pirâmide de testes e matriz de conformidade

A bateria de testes do Petunia3D organiza-se em seis camadas complementares:

| Camada | Nível / Domínio | Ferramenta / Execução | Evidência requerida |
|---|---|---|---|
| **L1** | **Unit pure** (Geometry, Mesh, Math, Snap) | `cargo test -p petunia_core --lib` | determinismo, corner cases, validação estrita, floats finitos |
| **L2** | **E2E em memória / Transações** (Model, Paint, UV, Animate) | `petunia_editor::EditorController` (`editor_parity`) | fluxos completos, DTOs, Undo/Redo integral, sem flakiness de GUI |
| **L3** | **Automação nativa de GUI** (Qt Widgets, dialogs, FFI) | `QTestLib` (`QT_QPA_PLATFORM=offscreen bridge_test`) | eventos de mouse/tecla em widgets, atalhos, sincronização FFI |
| **L4** | **Fuzzing & Dados hostis** (Parsers, files, commands) | Sintético / Proptest / OSS-Fuzz pattern | ausência de pânico ("Never Crash"), erro gracioso em `Result::Err` |
| **L5** | **Viewport & Shaders** (OpenGL 3.3 FBO rendering) | Offscreen framebuffer capture + Golden PNGs | conformidade visual, ausência de artefatos, render-on-demand |
| **L6** | **Acessibilidade & Plataforma** (Teclado, F6, high-contrast) | `QAccessible` inspection + AT-SPI / Windows smoke | navegação F6/Tab, foco restaurado, screen reader tree, labels |

Testes devem cobrir invariantes do domínio **antes** de snapshots cosméticos. Golden images são complemento, não garantia de usabilidade.

## Gates de cada mudança

1. Lint/format e build do menor conjunto relevante sem warnings novos.
2. Suite de teste unitário do domínio alterado.
3. Contract tests de Application boundary: UI não muta Geometry diretamente; Renderer não grava Document.
4. Teste de Undo/Redo, Cancel e ausência de side effects onde pertinente.
5. Document/recovery/export fixture quando feature altera schema.
6. Testes com erros/limites para dados externos e adapters.
7. Acessibilidade (keyboard, state announced, labels, focus, reduced motion) para UI.
8. Benchmark antes/depois com cenário, hardware e versão para alegações de performance.
9. Registro de status **not run / pass / fail / blocked** para cada gate; sem inferir pass por compilação.

Testes em CI Linux não substituem smoke manual em Windows e nem teste de mesa fraca.

## Frontend Qt Widgets e petunia_editor — gates específicos

Com a remoção do Slint e consolidação em `petunia_editor` e Qt Widgets (ADR-C11), os gates de interface combinam validação estrita de contrato Rust com automação nativa offscreen via `QTestLib`:
## Controlador de editor — gate de CI atual

**2026-10-18:** com a retirada do Slint (ADR-C11/Q07), o workflow `ui-slint.yml` passou a falhar já no primeiro passo (`package petunia_ui_slint is not a member of the workspace`) sempre que um PR tocava `crates/core/**`. Ele foi substituído por `ui-editor.yml` (**UI Editor**), que cobre o controlador neutro `petunia_editor`, onde agora vivem intents, gestos, sessões e projeção de estado:

- cargo fmt -p petunia_editor -- --check
- cargo clippy -p petunia_editor --features animation-workspace --all-targets -- -D warnings
- cargo test -p petunia_editor --lib
- cargo test -p petunia_editor --features animation-workspace --lib

O adaptador `petunia_ui_cxxqt` exige Qt 6 no runner e não faz parte deste job; seus gates (`bridge_test`, `editor_parity`, `qt_parity_acceptance`) continuam locais, conforme [integração Qt](../09-ui/qt-integration.md). YAML atualizado não equivale a execução: o status remoto vale a partir do primeiro run com SHA.

## Frontend Slint — gates específicos (histórico)

Registro anterior à retirada do Slint; os comandos abaixo não se aplicam mais ao workspace. O workflow **UI Slint** verificava formatação do crate, tokens, Clippy e testes de interface/gestos:

Comandos canônicos de gate:
- `CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_editor -- --check`
- `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_editor --all-targets -- -D warnings`
- `CARGO_BUILD_JOBS=2 cargo test -p petunia_core --lib`
- `CARGO_BUILD_JOBS=2 cargo test -p petunia_editor --features animation-workspace --lib`
- `CARGO_BUILD_JOBS=2 QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1 --skip test_widgets_studio_window_creation`
- `CARGO_BUILD_JOBS=2 QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt --test editor_parity -- --test-threads=1`

### Critérios de aceitação Qt Widgets (Go/No-Go)

Não dar Go final da GUI enquanto faltarem evidências executadas de:
- FBO/GL Texture compositado e redimensionamento correto no `QtGLWidget` (`QOpenGLWidget`);
- Picking determinístico sem leituras redundantes de framebuffer;
- Todos os fluxos de autoria essenciais (MODEL, DRAW, POLY, PAINT, UV, ANIMATE) cobertos por paridade E2E;
- Navegação completa por teclado, ciclo de foco `F6`/`Tab`, recuperação de foco pós-diálogos e árvore `QAccessible`;
- Escala de UI 100–200%, High Contrast, Reduced Motion e alvos de clique adequados;
- Performance medida em hardware de baixo desempenho em Linux e Windows sem regressões de frame time.

## Gesto e keymaps

Cobrir:
- click, drag threshold, lasso/box, orbit/pan/zoom;
- remap de pointer, Maya Alt+LMB, Ctrl/Alt/Shift interpretados semanticamente;
- transform, gizmos, modal tool/latched gesture, click-move-click;
- Paint stroke begin/update/end, pointer Cancel, lost focus e Escape;
- coord. normalized/logical/physical em HiDPI e split;
- estado da ferramenta com overlays e screen reader feedback.

Testes não substituem inspeção do código para paths paralelos que contornem o dispatcher.

## Quality Gate de Geometry e Project

Cada operação topológica deve provar:
- validade de triangles/quads/n-gons e índices;
- FaceCorner UV, sharp normals, material slots e topology remapping;
- propriedade de Undo integral;
- cálculo determinístico salvo tolerâncias numéricas explícitas;
- ausência de panic com input ruim;
- equivalência de dados antes/depois do save/load.

Rust fuzz/property tests e sanitizer/Miri entram nos módulos seguros quando tecnicamente viáveis, não por imposição abstrata em todos os crates.

## Segurança e supply chain

- cargo-deny para advisories/licenses/bans/sources quando workflow habilitado.
- Permissões mínimas de CI e pinagem de versões conforme governança; não introduzir downloads runtime obrigatórios.
- Testes de input externo em project, images, SVG, OBJ/GLB e MCP requests.
- Secrets nunca em fixtures públicas ou logs.
- Interdependências de crates não podem inverter grafo acordado.
- Plugins e commands precisam de testes de capability, cancel e revision mismatch.

## Procedimento de regressão manual

Roteiro reproduzível por versão:
1. Abrir projeto limpo e legado.
2. Criar objeto → DRAW shape → Push/Pull → POLY edit → Undo/Redo.
3. Paint/UV, image reference e Project Photo smoke.
4. Salvar, fechar, reabrir; comparar IDs, aparência, layers e UV.
5. Animate procedural preview, pausas e parâmetros.
6. Trocar keymap, escala, temas, teclado only, focus entre F6 regions.
7. Split viewport, maximize/resize, sleep/wake e dispositivo gráfico quando aplicável.
8. Export/validation e reimport.
9. Executar low-end e Linux/Windows com logs e captures reais, não mockups.

## Observabilidade e report

Cada evidência registra: commit SHA, branch, toolchain, OS, GPU/driver, feature flags, command, resultado, run URL, limites e screenshot/log quando apropriado. Um gate pendente fica explicitamente pendente; "implementado" nunca é inferido pela existência de arquivo de teste.

## Decisões fechadas

Contratos testados por boundary; UI/GL/a11y hard gates; nenhuma aprovação sem evidência; rollback/cancel/Undo essenciais; fixtures cross-platform; performance medida; CI da branch e legado egui têm escopos separados.

## Wave U01 — cobertura adicional (2026-10-09)

`ui-slint.yml` passa a incluir `paint_shell` e `viewport_hud` junto de MODEL/UV/ANIMATE/gestures na configuração `animation-workspace`. As regressões novas exercitam input físico composto, strokes/seleção 2D, IDs/opacity de layers atualizados, efeitos/brush/binding do canvas e geometria/hit testing de HUDs passivos. Execução e limites da wave: [Slint Rescue §19](../09-ui/slint-rescue-plan.md#19-wave-u01--paint-e-huds-2026-10-09). A alteração de configuração não comprova execução remota; Q03 permanece parcial.

## Wave U02/U03 — suítes preparadas (2026-10-09)

`ui-slint.yml` inclui `shell_focus`, `inspector_split` e `workspace_drawer` na integração com `animation-workspace`. A primeira execução de `shell_focus`, anterior à nova implementação, teve **1 pass / 1 fail**. Após o usuário adiar os testes, a correção e as novas regressões foram escritas, sem execução/compilação. Q03 mantém o checkpoint anterior (025%); CI remota e gates desta wave: **not run**. Registro: [Slint Rescue §20](../09-ui/slint-rescue-plan.md#20-wave-u02u03--regioes-independentes-e-drawer-2026-10-09).
