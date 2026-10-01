# Gramática única de ferramenta (`ToolSession`) — Implementation-vs-Spec Gap Matrix (Onda 2)

Escopo: Onda 2 do [ADR 007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md).
Autoridade: [constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md)
(ciclo `ToolSession`, gesto atômico, "Última operação", contrato numérico),
[constituição 03](../bible/constitution/03-invariantes-de-ui-ux-design-system-e-acessib.md)
(navegação, RMB, alvos), [capítulo 36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
(entrada), [P3D-083](../bible/specs/p3d-083-tool-properties.md),
[P3D-092](../bible/specs/p3d-092-petunia-default.md) e
[P3D-131](../bible/specs/p3d-131-modal-tool-feedback-system.md). Diagnóstico de
origem: [capítulo 46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md), seção 2.2.

## Ciclo de cada ferramenta antes × depois

| Ferramenta | Antes (29/09) | Depois (Onda 2) |
| --- | --- | --- |
| Move / Rotate / Scale (ferramenta) | Persistente; arrasto após 4 px fixos no Slint; clique seleciona; sem "Última operação" | `ToolSession` no core decide clique × arrasto com limiar configurável; clique seleciona; arrasto opera; confirmar registra "Última operação" |
| Gizmo (alças) | Arrasto próprio (`is-gizmo-drag`) | Mesmo `ToolSession` (alvo `Handle`); clicar-mover-clicar na alça quando a preferência está ativa |
| Extrude / Extrude Individual / Inset / Round Edge / Push-Pull | Tecla abria operação imediata; arrasto vertical incremental 0,5 × mundo/px; E em Object selecionava tudo; sem alça | Tecla escolhe a ferramenta persistente e troca o domínio de forma visível; arrastar sobre uma face (ou aresta) a seleciona e opera; o valor segue o cursor pela normal projetada, estável sob zoom; Shift = precisão; confirmar = 1 Undo + "Última operação" |
| Modal por teclado (duplo toque / preferência "instant") | Segue o mouse; clique confirma; RMB cancela | Mantido como estado `Latched` do perfil Blender; RMB não cancela mais |
| Loop Cut / Slice / Knife / Profile | Navegação suspensa; roda mudava contagem | Navegação liberada; roda sempre faz zoom; Ctrl+roda muda a contagem; ciclos próprios até as Ondas 4 e 5 |

## Requisitos

| # | Requisito | Estado antes | Evidência e delta | Estado após |
| --- | --- | --- | --- | --- |
| 1 | Uma máquina de estados de gesto no core, sem teclas físicas | MISSING | `crates/core/src/tool_session.rs`: `ToolSession` (`Idle/Pressed/Dragging/Latched`), `ToolEffect`, `DragFrame`, `drag_value`, `LastOperation`; 15 testes de domínio. | PARTIALLY_COMPLIANT — `Collecting` (vários cliques) fica para as Ondas 4/5. |
| 2 | Clique seleciona; arrasto opera; limiar configurável (2–16 px) | PARTIALLY_COMPLIANT | Limiar fixo de 4 px no Slint só para Move/Rotate/Scale. Agora vem de `UserPreferences::drag_threshold_px`, com controle em Configurações. | PARTIALLY_COMPLIANT — sem teste de hardware. |
| 3 | Arrastar em qualquer lugar seguindo o cursor | FUNCTIONAL_BUT_DIFFERENT | Extrude/Inset/Round Edge/Push-Pull usavam delta vertical. Agora `drag_value` projeta o cursor na normal (Extrude/Push-Pull), usa distância ao pivô (Inset/Round Edge) e razão de distâncias (Scale). | PARTIALLY_COMPLIANT — faltam alças visuais dedicadas (seta na normal) e captura nativa. |
| 4 | Gesto atômico + "Última operação" ajustável no mesmo Undo | MISSING | `AppState::commit_modal_gesture` e `adjust_last_operation` (desfaz, reaplica e confirma: profundidade do histórico igual). Card no `ToolCard` com `NumericField`. | PARTIALLY_COMPLIANT — ajuste por texto; arrastar o campo ainda não reaplica (custo de reaplicar a cada evento). |
| 5 | Clicar-mover-clicar (WCAG 2.5.7) | MISSING | Preferência `click_move_click`: clicar numa alça a prende ao ponteiro até o próximo clique. | PARTIALLY_COMPLIANT — só alças do gizmo de transformação nesta onda. |
| 6 | Valor digitado vence o mouse e não vaza | BROKEN | Onda 1 corrigiu o buffer; o gesto paramétrico ignora o mouse enquanto houver texto. | COMPLIANT — core `typed_value_wins_over_the_pointer_and_never_leaks`; bridge `typed_value_wins_over_drag_and_shows_in_the_hud` (2026-09-30). |
| 7 | RMB nunca cancela | FUNCTIONAL_BUT_DIFFERENT | `viewport_context_triage` cancelava qualquer sessão; o `TouchArea` limpava o gesto no meio do arrasto. Agora RMB é ignorado durante gesto e abre o menu fora dele. | COMPLIANT — `right_button_never_cancels_an_active_gesture`, `viewport_right_click_never_cancels_an_active_transform`, core `secondary_button_never_cancels` (2026-09-30). |
| 8 | Navegação nunca suspensa; roda sempre zoom | BROKEN | `mouse_navigation_suspended` removido; guardas do Slint removidas; `viewport_ctrl_scroll` para contagens e raio proporcional. | PARTIALLY_COMPLIANT — sem teste com trackpad. |
| 9 | Escada do Esc | PARTIALLY_COMPLIANT | Esc cancela o gesto, depois fecha overlays e, por fim, devolve a ferramenta persistente para Select. | COMPLIANT — `escape_ladder_returns_a_persistent_tool_to_select`, core `escape_ladder_cancels_the_gesture_then_exits_the_tool` (2026-09-30). |
| 10 | Sem efeitos ocultos | BROKEN | E/I/Ctrl+B em Object selecionavam tudo; Esc apagava a primitiva recém-criada. Agora a troca de domínio é visível, nada é selecionado sozinho e Esc mantém a primitiva (Undo remove). | COMPLIANT — `tool_key_selects_the_persistent_tool_without_opening_an_operation`, `escape_keeps_a_newly_created_primitive`, `test_smart_contextual_selection_mode_switching` (2026-09-30). |
| 11 | Textos visíveis por `TextId` | PARTIALLY_COMPLIANT | Novos textos (`tool_grammar.*`, `preferences.*`) em en e pt-BR. Textos antigos do HUD e dos cards seguem fixos em inglês. | PARTIALLY_COMPLIANT. |

## Pendências registradas

- ~~`Space` = ferramenta anterior~~ → implementado (2026-09-30): `Space` ocioso alterna com a
  ferramenta anterior (a do workspace atual; senão Select); durante uma sessão ele continua
  confirmando. O micro-inspector passou para `Shift+Space`. Ação de keymap `global.previous_tool`.
- Um único buffer numérico: o bridge ainda usa `modal_text`; o buffer de
  `ToolSession::text` está pronto para frontends futuros (MCP/Lua).
- Loop Cut, Slice, Knife e Profile migram para `Collecting` nas Ondas 4 e 5.
- Gizmo 3D em passo GPU próprio e alças ≥ 24 px para Extrude/Inset/Round Edge: Onda 6.

## Testes do contrato antigo atualizados de propósito

Estes testes codificavam comportamentos que o ADR 007 substituiu e foram
reescritos para o contrato novo (não são regressões):
`viewport_right_click_never_cancels_an_active_transform` (antes: RMB cancela),
`viewport_right_click_opens_selection_menu_without_session` (retorno = menu aberto),
`modeling_tool_shortcut_double_tap_behavior` e
`inset_and_bevel_shortcuts_support_single_and_double_tap` (1 toque escolhe a
ferramenta; duplo toque segue o ponteiro), `proportional_editing_radius_adjustment_and_falloff`
e `wheel_zooms_even_with_a_tool_modal_open` (roda = zoom; Ctrl+roda = raio),
`test_smart_contextual_selection_mode_switching` (sem selecionar tudo),
`gizmo_center_and_plane_hit_testing` e `test_magnetic_snap_marker_projection`
(codificavam os bugs corrigidos na Onda 1).

Falha preexistente corrigida: `test_viewport_context_menu_modeling_actions_and_dismissal`
esperava "Normais recalculadas (suave)", mas o commit `41b3902` transformou o
Shade Smooth em comando do core com status "Smooth shading"; o teste já falhava
no branch de hardening.
