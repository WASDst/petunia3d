# Viewport nítido e fluido + bugs comprovados — Implementation-vs-Spec Gap Matrix (Onda 1)

Escopo: Onda 1 do [ADR 006](../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md).
Autoridade: Livro Vivo [05](../bible/foundations/05-viewport-shading-modos-visualizacao.md),
[36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md) (HiDPI, repaint
event-driven), [constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md)
(contrato numérico), [P3D-040](../bible/specs/p3d-040-sistema-de-snap.md) e
[P3D-131](../bible/specs/p3d-131-modal-tool-feedback-system.md). Diagnóstico de
origem: [capítulo 45](../bible/foundations/45-pesquisa-interacao-modelagem-referencias.md), seção 2.
Auditoria revalidada em 2026-09-29 contra `f0d7f25`. Nenhum item vira COMPLIANT
sem teste ou captura.

| # | Requisito | Estado antes | Evidência e delta | Estado após |
| --- | --- | --- | --- | --- |
| 1 | Imagem 3D nítida em HiDPI e UI scale > 100% | BROKEN | O alvo WGPU era criado em px lógicos (`self.width / 1px`) e esticado pelo Slint. Agora o alvo usa px físicos (`1phx` no Slint e `window().scale_factor()` no primeiro frame); câmera, picking e overlays continuam em px lógicos; bandas e discos de seleção usam a altura lógica. A troca de monitor ou de UI scale recria o alvo porque a medida física muda. Backend de CPU continua em px lógicos. | PARTIALLY_COMPLIANT — teste `gpu_viewport_renders_in_physical_pixels_while_ui_math_stays_logical` e `cpu_viewport_keeps_logical_pixels`; falta captura nativa em 100% e 150% (a UI Slint não tem modo `--screenshot`). |
| 2 | Antisserrilhado | MISSING | Todas as pipelines e o depth usavam 1 amostra. Agora `Renderer::with_sample_count` usa 4 amostras no viewport Slint, com alvo multisample resolvido na textura exibida (`StoreOp::Discard` nas amostras). `Renderer::new` segue com 1 amostra para o legado egui. | PARTIALLY_COMPLIANT — frame renderizado em GPU real com MSAA 4× sem erro de validação do wgpu (`wgpu_viewport_initializes_or_skips_when_no_gpu` executou, não pulou); MSAA não substitui linhas de largura constante com AA (Onda 6). |
| 3 | Não redesenhar com o mouse parado sobre o mesmo alvo | BROKEN | O callback de hover ignorava o retorno "mudou?" e refazia view model completo, ~470 `set_*` e o frame GPU a cada movimento. Agora retorna cedo quando nada mudou; o elástico do perfil e a linha da faca continuam retornando "mudou". | PARTIALLY_COMPLIANT — coalescer renders por quadro (flag "sujo" + um render por quadro) fica pendente: ~60 pontos chamam `render_viewport()` de forma síncrona e a mudança exige reestruturar os callbacks; entra no laço de quadros da Onda 6. Teste: `hovering_the_same_empty_spot_requests_no_redraw`. |
| 4 | Hover do gizmo dá presença ao eixo | BROKEN | `transparentize(0.18)` escurecia o eixo sob o cursor. Agora `brighter(0.35)`. | PARTIALLY_COMPLIANT — sem captura. |
| 5 | Anel de View Roll clicável onde é desenhado | BROKEN | Desenho a 72×1,18 ≈ 85 px e hit-test a 96×1,18 ≈ 113 px. Uma constante (`GIZMO_VIEW_ROLL_RADIUS`) serve aos dois; faixa clicável de 24 px (WCAG 2.5.8). | COMPLIANT — `view_roll_ring_is_clickable_on_the_drawn_radius`, `gizmo_center_and_plane_hit_testing` (2026-09-30). |
| 6 | Buffer numérico não vaza entre operações | BROKEN | `modal_text` não era limpo em `begin/commit/cancel_tool_modal`: E → 2 → Enter → E → 5 extrudava 25. Agora é limpo nos três pontos; enquanto houver texto, o arrasto não sobrescreve o valor; o HUD mostra `Input`. | COMPLIANT — `typed_value_does_not_leak_into_the_next_tool_modal`, `typed_value_wins_over_drag_and_shows_in_the_hud` (2026-09-30). |
| 7 | Card da faca e Cancelar | BROKEN | O domínio ativa a faca como `cut`, mas o card, o cursor e o botão testavam `knife`; Cancelar enviava `view.cancel_active`, que não existe; o botão Cut reiniciava a sessão. Agora usam `cut`, `model.knife_cancel` e `model.knife_apply`. | PARTIALLY_COMPLIANT — `knife_card_actions_apply_and_cancel_the_cut_session`; textos do card ainda sem `TextId`. |
| 8 | `is_snapped` significa "encaixou" | BROKEN | Era `snap_enabled`. `ModalOp` registra se o último update encaixou; o marcador de snap só aparece nesse caso. | COMPLIANT — core `feedback_reports_snap_only_when_a_target_attracted_the_point`; bridge `test_magnetic_snap_marker_projection` (2026-09-30). |
| 9 | Cota respeita o tipo do valor | BROKEN | Somava `pivot + components` mesmo quando eram graus (Rotate) ou fatores (Scale). Agora `ModalOp::current_point()` só existe para Move e Extrude/Push-Pull; Rotate, Scale, Inset e Bevel mostram o valor no HUD, sem cota linear e sem linha-guia de mundo. | COMPLIANT — core `rotation_and_scale_have_no_world_point_or_guide_line`; bridge `dimension_annotation_only_draws_linear_distances` (2026-09-30). |

## Pendências registradas

- Linhas de largura constante com AA, contorno por jump flooding, iluminação
  relativa à câmera e grade infinita: Onda 6.
- Buffers indexados por asset, matriz por objeto e flags de seleção em buffer
  pequeno: Onda 3.
- Snap duplo (bridge + core) e tolerância em px: Onda 3 (P3D-040).
