# Workspace POLY — Poly Pen — Implementation-vs-Spec Gap Matrix (Onda 5, parte 1)

Escopo: primeira entrega da Onda 5 do [ADR 007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md)
— a ferramenta "quase universal" de edição de componente do POLY.
Autoridade: [P3D-075](../bible/specs/p3d-075-vertical-tool-toolbar.md) (trilho
POLY com Poly Pen), [capítulo 46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md)
§1.2 ("POLY também desenha, no nível de componente") e §4 (Polygon Pen do
Cinema 4D como modelo) e [constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md)
(ferramenta persistente, `Collecting`, gesto = 1 Undo, escada do Esc).

## Auditoria antes da mudança (30/09)

| Aspecto | Encontrado | Classificação |
| --- | --- | --- |
| Pré-seleção de componentes no hover | `hover_component` com o mesmo hit test do clique (Ondas 2–3) | COMPLIANT (preservado) |
| Ferramenta Poly Pen | Inexistente; `P3D-075` a lista no trilho POLY | MISSING |
| Mover elemento sem selecionar antes | Só com Move/gizmo sobre a seleção existente | MISSING |
| Extrudar aresta de borda | Extrude exige faces | MISSING |
| Desenhar polígono ponto a ponto | Só Make Face a partir de seleção | MISSING |
| Derreter ponto | `dissolve_vertices` existe (comando Dissolve) | FUNCTIONAL_BUT_DIFFERENT |

## Requisitos

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 1 | Ferramenta persistente na gramática única | `GrammarTool::PolyPen` (`poly_pen`) na mesma `ToolSession`; botão no trilho POLY; `ModelingMode::Poly` a oferece, DRAW não. | COMPLIANT |
| 2 | Hover destaca o elemento que o gesto vai usar | `poly_pen_target`: ponto > aresta > face, com as tolerâncias em pixels e a oclusão do picking de seleção. | COMPLIANT |
| 3 | Arrastar ponto/aresta/face move o elemento, sem selecionar antes | O elemento arrastado vira a seleção (visível, como no C4D) e o Move segue o cursor; 1 Undo; snap da Onda 3. Teste `poly_pen_drag_moves_the_point_under_the_cursor`. | COMPLIANT |
| 4 | Ctrl-arrastar aresta extruda | `Mesh::extrude_edge` (só arestas de borda, quad com a orientação do vizinho) + `AppState::begin_poly_pen_edge_extrude` (prelúdio + Move = 1 Undo, `Esc` exato). Testes `extruding_a_border_edge_adds_a_consistent_quad`, `edge_extrude_gesture_is_one_undo_and_escape_restores`, `poly_pen_ctrl_drag_extrudes_a_border_edge`. | PARTIALLY_COMPLIANT — arestas internas são recusadas com mensagem; o modificador ainda é Ctrl fixo no bridge (keymap pendente). |
| 5 | Cliques desenham um polígono (`Collecting`) | `Mesh::add_pen_polygon` reaproveita pontos existentes, orienta pelo vizinho ou para a câmera e recusa resultado não manifold; Enter ou o 1º ponto fecham; Backspace remove o último; Esc limpa antes de sair da ferramenta; prévia até o cursor. Testes `poly_pen_draws_a_polygon_as_one_undo`, `poly_pen_collecting_follows_the_escape_and_backspace_ladder`, `polygon_on_a_border_edge_follows_the_neighbor_winding`. | COMPLIANT |
| 6 | Ctrl-clique derrete o ponto | `AppState::poly_pen_melt_point` (1 Undo). Teste `poly_pen_ctrl_click_melts_a_point`. | COMPLIANT |
| 7 | Textos por `TextId` | `tools.poly_pen`, `tools.poly_pen_hint` (en/pt-BR). | COMPLIANT |

## Segunda entrega (04/10/2026)

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 8 | Modos Points/Edges/Polygons (cap. 46 §4) | `PolyPenMode` (Auto/Points/Edges/Polygons) no card da ferramenta (`BrushChip`), `UiIntent::SetPolyPenMode`; o hover e o clique seguem o modo: Points só pontos (clique coleta, alternativo derrete), Edges só arestas (clique insere ponto, alternativo extruda), Polygons faces e arestas de borda. Auto preserva o comportamento anterior. Teste `poly_pen_modes_filter_the_hover_and_the_click`. | COMPLIANT |
| 9 | Pintar faces arrastando | `AppState::begin_poly_pen_strip`/`poly_pen_strip_to`/`finish_poly_pen_strip`/`cancel_poly_pen_strip` (`PenStrip`): arrastar a partir de uma aresta de borda no modo Polygons cria um quad por passo do tamanho da aresta, seguindo o cursor no plano de desenho; gesto = 1 Undo, quads pintados selecionados, `Esc` restaura. Testes `painting_a_strip_is_one_undo_and_escape_restores`, `strips_only_start_on_border_edges`, `poly_pen_polygons_mode_paints_quads_from_a_border_edge`. | COMPLIANT |
| 4 | Modificador alternativo pelo keymap | Seção `[pointer]` do keymap (`pointer.alternate` = Ctrl, `pointer.extend` = Shift por padrão; `Keybinds::pointer_modifier`, exportação TOML). O bridge resolve os dois no press; o texto de ajuda mostra o modificador configurado. Testes `pointer_modifiers_have_defaults_and_roundtrip_through_toml`, `poly_pen_alternate_modifier_comes_from_the_keymap`. | COMPLIANT |
| 10 | Ponto novo no vazio usa o plano de trabalho | `poly_pen_plane_point`: com o plano de trabalho travado (cap. 05), pontos novos e a pintura caem nele; sem ele, no plano de frente para a câmera pelo último ponto. | COMPLIANT |
| 11 | Ícone dedicado | `tool_poly_pen` (outline e filled) no pacote Petunia, `PetuniaIcons.tool-poly-pen` no trilho POLY. | COMPLIANT |

## Pendências registradas

- ~~Ícone dedicado de caneta no pacote Petunia~~ → implementado (2026-10-04).
- ~~Modificador "ação alternativa" resolvido pelo keymap~~ → implementado
  (2026-10-04, seção `[pointer]`).
- ~~Clique numa aresta para subdividi-la~~ → implementado (2026-09-30): sem polígono em coleta, clicar numa aresta insere um ponto nela (`Mesh::split_edge`, todas as faces da aresta, UV interpolada, costura dividida; `AppState::poly_pen_split_edge`, 1 Undo, o ponto novo fica selecionado). Com pontos já coletados o clique continua adicionando ponto ao polígono.
- ~~Pintar faces arrastando; modos Points/Edges/Polygons~~ → implementado (2026-10-04).
- ~~Novo ponto no vazio integrado ao plano de trabalho~~ → implementado (2026-10-04).
- `assets/icons/petunia-dual/generate.py` falha antes desta mudança
  (`Unmapped command draw.exclude`); o ícone novo foi gravado com as funções do
  próprio gerador.
- Captura nativa e teste com usuários.
