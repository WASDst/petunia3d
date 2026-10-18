# Shell v2 — refinamento de uso (2026-10-18)

> Status: implementado no código e coberto por testes automatizados; **verificação visual em uso real, em Windows e com leitor de tela continua pendente**. Decisão registrada no [ADR-C15](../00-philosophy/decision-register.md#adr-c15--shell-v2-refinamento-de-uso-2026-10-18--aprovado-pelo-usuario).

## Intent

Corrigir o que o uso real do shell v2 mostrou: contorno de seleção "fantasma", painel de Partes sem arrastar nem grupos, pivô invisível, painéis poluídos, booleana direta duplicando o nó, controles grandes demais, gizmos e navegação pouco legíveis e nós sem identidade visual.

## Sources

Pedido do usuário de 2026-10-18 (13 observações com imagens de referência), [Shell v2](shell-v2-proposal.md), [Operation Graph](../03-geometry/operation-graph.md), `crates/render/src/gl/outline.rs`, `crates/editor/src/projection.rs`, `crates/ui-cxxqt/cpp/shell/`, `crates/ui-cxxqt/cpp/petunia_nav_cube.*`.

## Gap

Causa do "fantasma" (item 1): não era o renderer GL. Um contorno 2D antigo, calculado em Rust como caminho de texto (`compute_selection_overlay`) e pintado por `QPainter` por cima do GL, só era refeito quando o view-model era recalculado. Ao arrastar um gizmo ou girar a câmera ele ficava para trás. Também usava cores fixas, o que explica a cor da parte ser ignorada (item 2).

| #  | Observação                                         | Estado inicial                                                    | Entrega                                                                                  | Status        |
|----|----------------------------------------------------|-------------------------------------------------------------------|------------------------------------------------------------------------------------------|---------------|
| 1  | Overlay de seleção não segue o objeto              | contorno 2D projetado e desatualizado                             | removido; só o passe GL, com máscara RGBA por parte, ativo e hover                       | [x] DONE (código e testes; visual pendente) |
| 2  | Partes: arrastar, vários, Ctrl+G, isolar, cor, alinhar | lista plana, sem DnD, alinhar em submenu longo                  | árvore com grupos, DnD, Ctrl/Shift+clique, Ctrl+G/Ctrl+Shift+G, coluna de isolar, painel de alinhar | [x] DONE (código e testes; visual pendente) |
| 3  | Pivô visível e editável                            | só o 3D Cursor                                                    | marcador de pivô e de origem; modo Editar pivô com indicação, botão e Esc                | [x] DONE (código e testes; visual pendente) |
| 4  | Painel lateral poluído                             | botões de largura total, estatísticas duplicadas                  | botões em linha, ícones, rótulos curtos, seções menores                                  | [x] DONE (código; visual pendente) |
| 5  | Booleana só por nós                                | comandos e estado globais de Fuse/Cut/Intersect                   | removidos; Join por seleção; Make Editable do grafo                                      | [x] DONE      |
| 6  | Botões e menus da viewport                         | cabeçalho com texto e altura confortável                          | cabeçalho só com ícones, altura compacta, menus estilizados                              | [x] DONE (código; visual pendente) |
| 7  | Navegador 3D moderno                               | esferas de eixo (e mapeamento Y/Z trocado)                        | cubo com faces, arestas, cantos, bússola, início e menu de projeção                      | [x] DONE (código e testes; visual pendente) |
| 8  | Gizmos reformulados e configuráveis                | hastes simples, tamanho fixo                                      | novo desenho e `gizmo_size`/`gizmo_thickness` nas Preferências                           | [x] DONE (código e testes; visual pendente) |
| 9  | Nós com identidade visual                          | cabeçalho neutro                                                  | cor, ícone, papel, portas e fios por tipo, legenda                                       | [x] DONE (código; visual pendente) |
| 10 | Coerência geral                                    | —                                                                 | tokens e alturas compactas compartilhados (`compactControlHeight`)                       | [x] DONE      |
| 11 | Rail só transformar, cursor e criação              | rail com todas as operações                                       | catálogo enxuto; operações na barra contextual, agrupadas                                | [x] DONE      |
| 12 | Vértices/arestas com problemas                     | relato sem passos                                                 | testes de seleção por clique e de operações em objeto esticado e girado; bug de dissolver corrigido; ver Verification    | [-] IN PROGRESS (demais casos dependem de passos do relato) |
| 13 | Referências de UI                                  | imagens anexas                                                    | navegador (cubo e bússola), gizmos (setas, arcos, planos) e nós (cores e tipos)          | [x] DONE      |

## Changed

- **Renderer:** `RenderOutline` guarda uma entrada por objeto (`OutlineEntry`) com a cor já resolvida; a máscara do contorno é RGBA8 e o shader pega a cor do vizinho mais próximo. Cor da parte (`selection_overlay_color`) para ativo, selecionados e hover; a cor da parte entra na chave dos overlays de componentes.
- **Editor/ponte:** removidos `SelectionOverlayModel`, `compute_selection_overlay` e o `drawSelectionOutline` do Qt. Novos pedidos `scene_group`, `scene_ungroup`, `scene_drop`, `group_*`, `isolate_part`, `set_gizmo_size`, `set_gizmo_thickness`.
- **Núcleo:** comandos `scene.group`, `scene.ungroup`, `MoveAssetsCmd`, `RenameGroupCmd` (todos com Undo de uma entrada); isolamento passa a valer para a seleção inteira; `model.join` funde a seleção no ativo. Removidos `BooleanOpCmd`, `apply_boolean`, `boolean_texture` e os campos `boolean_*` da sessão.
- **Qt:** `PetuniaNavCube` (cubo), `PetuniaAlignPopover`, `PetuniaStructurePanel` com árvore e DnD controlado (o Qt nunca move linhas; o gesto vira `scene_drop`), cabeçalho e barra contextual compactos, gizmo e pivô desenhados em `PetuniaGlWidget`, nós coloridos.
- **Catálogos:** rail com transformar, 3D Cursor e criação; `context_actions` com grupos (`GROUP_ORDER`) e as operações de modelagem como ferramentas contextuais.

## Verification

Executado em 2026-10-18, no fim da implementação, sobre a árvore de trabalho com base em `85a7ed3b` (ainda sem commit no momento da execução). Ambiente: Linux, `RUSTUP_TOOLCHAIN=1.98.1`, `CARGO_BUILD_JOBS=2`, Qt offscreen, GL por EGL headless.

| Gate                                                                                     | Resultado                                              | Status |
|------------------------------------------------------------------------------------------|--------------------------------------------------------|--------|
| `cargo fmt --all -- --check`                                                             | sem diferenças                                         | pass   |
| `cargo clippy -D warnings` (core, editor, render, config, mesh, project, application, ui-cxxqt) | 0 avisos                                          | pass   |
| `cargo test` (core, config, editor, project, mesh, render, application; `--all-features`, `PETUNIA_REQUIRE_GL=1`) | 85 binários, 1519 passaram, 0 falharam, 0 ignorados, nenhum SKIPPED | pass   |
| `cargo test -p petunia_ui_cxxqt -- --test-threads=1` (offscreen; pula `test_widgets_studio_window_creation`) | 48 passaram, 0 falharam; `shell_v2` (harness offscreen) passou | pass   |
| `cargo run -p xtask -- docs-generate --check`                                            | sem drift depois de regenerar `docs/generated/`        | pass   |
| `node website/scripts/verify-progress.cjs` e `node --test website/tests/progress.test.cjs` | 132 processos consistentes; 6/6 testes                 | pass   |
| Olhar o resultado em janela real com GPU (contorno ao arrastar, cubo, pivô, árvore)     | não executado                                          | not run |
| Windows, leitor de tela, alto contraste, escalas 150%/200%                               | não executado                                          | not run |

Regressões provadas por teste: o dissolver de arestas vizinhas gerava face com vértice repetido (3 dos 6 testes de `dissolve_edges_validity` falham sem a correção em `Mesh::dissolve_selected`); `primitive_frame` falhava já no HEAD por comparar JSON de `HashSet` e passou a ordenar os conjuntos antes de comparar.

Limites: `ICON_TOKENS.md` gerado saiu vazio, porque o gerador ainda procura o catálogo Slint removido; fica como tarefa separada. O teste `test_widgets_studio_window_creation` continua fora da rodada, como nos gates do AGENTS.md.

### Correção posterior: gizmo parado ao mover a câmera (2026-10-18)

Relato do usuário depois do commit `bbf2d941`: ao orbitar ou dar pan/zoom, o gizmo ficava na tela em vez de seguir o objeto. Mesma família do "fantasma": gizmo, pivô e 3D Cursor são projetados em pixels por Rust (`compute_gizmo`) e o shell v2 reaproveita o snapshot que a janela empurra só quando o editor muda (`m_externalState`). Gestos de câmera do próprio `PetuniaGlWidget` (órbita, pan, zoom, roda, ações do cubo) não geravam snapshot novo. Agora esses gestos marcam `m_cameraDirty` e `refreshOverlayState()` refaz o snapshot no próximo quadro. Teste de regressão no harness (`viewport:gizmo-follows-the-orbit-mid-drag` e `...-pan-mid-drag`): arrasto real do botão do meio (e Shift para pan), conferido **no meio do gesto**, comparando o overlay desenhado com um snapshot novo. Soltar o botão já refazia o estado, então checar só depois escondia o defeito (primeira versão do teste passava sem a correção; foi reescrita). Provado: sem a correção as duas verificações falham; com ela passam. O pivô e o 3D Cursor usam o mesmo snapshot e foram corrigidos junto. Limite: uma trava isolada do `cargo test` sob carga concorrente, sem causa identificada (7 execuções diretas do binário passaram).

## Risks

- Grupos são `collections` de um nível: hierarquia aninhada com transformação herdada continua em G09.
- O isolamento guarda visibilidades por índice; por isso reordenar a cena durante o isolamento é recusado.
- O cubo de navegação e os gizmos só foram validados por geometria e pixels em ambiente headless; falta olhar o resultado numa janela real, em Windows e em alto contraste.
- O item 12 (vértices e arestas) veio sem passos de reprodução; os testes novos cobrem seleção e operações em objeto esticado e girado, mas não esgotam o relato.

## Next

Rodar a aplicação, conferir o contorno ao arrastar gizmos, o cubo, o pivô e a árvore de Partes, e listar casos concretos de vértices/arestas que ainda falhem (passo a passo) para virarem testes de regressão.
