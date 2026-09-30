# Snapping, pré-seleção e plano de trabalho — Implementation-vs-Spec Gap Matrix (Onda 3)

Escopo: Onda 3 do [ADR 006](../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md).
Parte 1: snapping. Parte 2: pré-seleção no Draw e plano de trabalho automático.
Autoridade: [P3D-040](../bible/specs/p3d-040-sistema-de-snap.md) (direção
aprovada em 2026-09-29), [P3D-131](../bible/specs/p3d-131-modal-tool-feedback-system.md),
[constituição 03](../bible/constitution/03-invariantes-de-ui-ux-design-system-e-acessib.md)
(nunca só cor) e [capítulo 45](../bible/foundations/45-pesquisa-interacao-modelagem-referencias.md),
seções 2.2, 2.3 e 3.3.

## Antes × depois

| Aspecto | Antes (30/09, Onda 2) | Depois (Onda 3, parte 1) |
| --- | --- | --- |
| Tolerância | 0,35 unidade de mundo (`SnapSettings::snap_distance`); inalcançável afastado, "gruda" aproximado | `SnapSettings::radius_pixels` (padrão 12 px lógicos, faixa 4–48) medido na tela: mesmo tamanho em qualquer zoom |
| Passadas por gesto | Move encaixava duas vezes: bridge (`project.active_mesh()`, a malha **em prévia**) e core (`update_modal`) | Uma passada no bridge (`screen_snap`) contra a malha de origem congelada; o core só registra o tipo (`update_modal_snapped`) e nunca encaixa sozinho |
| Alvos | Um por vez (Grid/Increment/Vertex/Edge/Face); face só no centroide | `SnapKind`: Ponto, Ponto médio, Na aresta, Guia de eixo X/Y/Z, Na face (ponto sob o cursor), Grade; prioridade fixa ponto > aresta > guia > face > grade |
| Autoencaixe | Vértices que se moviam eram alvos de si mesmos | `ModalOp::moving_vertices` exclui a geometria em movimento (e as faces dela da oclusão) |
| Oclusão | Nenhuma | Pontos e arestas atrás da malha só com raio-X/wireframe (reuso de `picking::occluded`) |
| Inferência | Nenhuma | Guias paralelas aos eixos a partir da âncora (Bier 1986; SketchUp; Dynamic Guides do C4D); com grade, a distância ao longo da guia anda no passo da grade |
| Perfil (Draw) | Grade fixa de 0,25 em `profile_screen_to_plane`, sem UI para ligar | Mesma passada: pontos/arestas da malha projetados no plano, guias `right`/`up` a partir do último ponto e grade do plano (`grid_spacing`) |
| Feedback | `is_snapped` + marcador âmbar; HUD "Snap Active" | `ToolFeedback::snap_kind`; marcador com forma (quadrado = ponto, círculo = demais) e **rótulo** traduzido (`snap_kind.*`); HUD mostra o tipo |
| Reaplicar "Última operação" | `reapply` chamava `update_modal`, que podia reencaixar o valor ajustado | O core não encaixa: o valor digitado é aplicado exatamente |

## Requisitos (P3D-040, direção aprovada)

| # | Requisito | Estado antes | Evidência e delta | Estado após |
| --- | --- | --- | --- | --- |
| 1 | Tolerância em pixels; o mais próximo vence | FUNCTIONAL_BUT_DIFFERENT | `crates/core/src/inference.rs` (`snap_screen`, `clamp_snap_radius`). Teste `tolerance_is_measured_in_pixels_at_any_zoom` (zoom 2 e 50). | COMPLIANT (2026-09-30) |
| 2 | Tipos: extremidade, médio, centro, na aresta, na face projetada, interseção, grade do plano | PARTIALLY_COMPLIANT | Ponto, médio, na aresta, na face (ponto projetado) e grade do plano implementados; testes `points_beat_edges_and_edges_beat_faces`, `grid_is_the_last_resort`. **Centro de face e interseção ainda faltam.** | PARTIALLY_COMPLIANT |
| 3 | Inferência de direção: eixos, paralelo/perpendicular à aresta, passos de 15° | MISSING | Guias de eixo a partir da âncora (mundo no Move; `right`/`up` do plano no Draw), quantizadas pela grade. Testes `axis_guide_infers_direction_from_anchor`, `axis_guide_steps_by_grid_spacing`, `profile_snap_infers_workplane_axis_from_last_point`. **Paralelo/perpendicular à aresta e 15° ainda faltam.** | PARTIALLY_COMPLIANT |
| 4 | Forma + cor + rótulo; nunca só cor | BROKEN | `SnapMarkerModel { label, round }`, `snap-marker-label` no `app.slint` com `DesignTokens`; `SnapKind::text_id` (en/pt-BR). Teste `test_magnetic_snap_marker_projection`. | PARTIALLY_COMPLIANT — sem captura nativa; o âmbar do marcador ainda é literal antigo, não token. |
| 5 | Trava por segurar `Shift` ou alternar por teclas; estado no HUD | PARTIALLY_COMPLIANT | Sem mudança nesta entrega (travas de eixo existentes). | PARTIALLY_COMPLIANT |
| 6 | `is_snapped` = encaixou de fato | COMPLIANT (Onda 1) | Mantido; agora vem do chamador (`feedback_reports_snap_only_when_the_caller_snapped`). | COMPLIANT |
| 7 | Uma única passada de snap por gesto | DUPLICATED | Snap removido de `AppState::update_modal`; bridge usa `screen_snap`. | COMPLIANT (2026-09-30) |
| 8 | Sem jitter; zoom/DPI; cancel | PARTIALLY_COMPLIANT | Tolerância em px lógicos (independe do fator de escala); desempate por profundidade determinístico. Sem teste de hardware. | PARTIALLY_COMPLIANT |

## Pendências registradas

- Raio do snap: campo em Configurações (`preferences.snap_radius` já tem TextId).
- Snap de Extrude/Push-Pull para alturas de outros pontos (inferência ao longo
  da normal) e snap entre objetos diferentes (hoje só a malha ativa).
- Centro de face, interseção de arestas, paralelo/perpendicular a aresta, 15°.
- Custo: a passada percorre todos os vértices e arestas da malha ativa a cada
  movimento; o orçamento do capítulo 45 (≤ 16,7 ms até 50 mil triângulos) exige
  um índice espacial (BVH/grade de tela) antes da Onda 6.
- Snap continua desligado por padrão (`SnapSettings::enabled = false`); o
  capítulo 01 pede snapping contextual por padrão, decisão a registrar junto com
  a pré-seleção (parte 2).

## Testes do contrato antigo atualizados de propósito

- `modal::tests::feedback_reports_snap_only_when_the_caller_snapped` substitui
  `feedback_reports_snap_only_when_a_target_attracted_the_point`: o core não
  encaixa mais sozinho (P3D-040, "uma única passada").
- `test_magnetic_snap_marker_projection`: `update_modal` sem tipo não mostra
  marcador; com `update_modal_snapped(.., Some(Point))` mostra forma e rótulo.

---

# Parte 2 — pré-seleção e plano de trabalho (2026-09-30)

Autoridade adicional: [capítulo 01](../bible/foundations/01-visao-ux-referencias.md)
(câmera contextual: "a câmera nunca se move sozinha"),
[capítulo 02](../bible/foundations/02-workflow-modelagem-shape-first.md)
(Draw on Face: a face sob o cursor vira o plano; "Olhar para o plano"),
[constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md)
("Idle → pré-seleção + dica") e capítulo 45 (tabela de decisões: plano
"automático pela vista, face sob o cursor, … travado; câmera não é forçada").

## Auditoria antes da mudança

| Aspecto | Encontrado (30/09) | Classificação |
| --- | --- | --- |
| Pré-seleção de componentes (Point/Edge/Face/Object) | `hover_component` usa o mesmo hit test do clique, em px, com oclusão | COMPLIANT (preservado) |
| Pré-seleção no Draw | O ramo `draw_profile` só guardava a posição do ponteiro: nada indicava onde o clique cairia | MISSING |
| Câmera ao ativar o Draw | `profile_align_camera_to_workplane` a cada ativação e ao escolher Face: vista e projeção trocadas sem pedido | BROKEN (cap. 01/02) |
| Plano automático | Face **selecionada** → Face; vista alinhada a eixo → plano da câmera; senão chão | PARTIALLY_COMPLIANT |
| Plano travado | Não existia: o rótulo Chão/Face/Vista era deduzido da normal e da seleção | MISSING |
| Textos do card | "Plane:", "Ground", "Face", "View" e status em inglês fixo | BROKEN (constituição 03, `TextId`) |

## Requisitos

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 9 | A câmera nunca se move sozinha; "Olhar para o plano" sob comando | Ativação usa `profile_capture_current` (sem câmera); intent `ProfileLookAtPlane` e botão no card. Teste `draw_never_moves_the_camera_on_its_own`. | COMPLIANT (2026-09-30) |
| 10 | Plano automático pela vista | `profile_capture_world_plane_for_view`: plano do mundo cuja normal é o eixo dominante da direção da câmera, voltado para ela, pelo 3D Cursor; reavaliado no 1º clique. Testes `world_plane_for_view_faces_the_camera_and_is_right_handed`, `auto_workplane_follows_the_view_when_there_is_no_face`. | COMPLIANT |
| 11 | Face sob o cursor vira o plano (Draw on Face) | `resolve_auto_workplane_at` no 1º clique de um perfil novo usa o mesmo picking de Face (com oclusão da cena); `profile_capture_face_index`. Teste `auto_workplane_uses_the_face_under_the_cursor_on_the_first_click`. | PARTIALLY_COMPLIANT — o perfil ainda gera volume como asset separado (sem imprint/Push-Pull na face hospedeira: Onda 4). |
| 12 | Plano travado | `ProfileState::{workplane_kind, workplane_locked}`; Chão/Face/Vista travam, Auto destrava; o tipo sobrevive a `clear()` e é recapturado. Testes `locked_workplane_ignores_the_face_under_the_cursor`, `locked_kind_is_recaptured_after_clear`. | COMPLIANT |
| 13 | Pré-seleção no Draw ("o usuário vê o que o clique fará") | `update_profile_preselection`: com Auto e sem perfil, destaca a face que viraria o plano e mostra o marcador "Na face"; depois, o ponto encaixado que o clique criaria (mesma passada do clique). `snap_marker_model` une operação e pré-seleção. | PARTIALLY_COMPLIANT — sem captura nativa; sem medição do orçamento de 1 quadro. |
| 14 | Textos por `TextId` | `ui.profile_plane*`, `ui.profile_look_at_plane`, `tool_grammar.draw_ready`, `workplane_set`, `workplane_auto`, `no_face_selected` (en/pt-BR). | COMPLIANT para o card e os status tocados |

## Pendências da parte 2

- Plano a partir de 3 pontos ou de face + aresta, e plano médio entre faces
  paralelas (Plasticity); "plano pelo ponto sob o cursor" em vez do 3D Cursor.
- ~~Em perspectiva comum, a regra do eixo dominante escolhe um plano vertical~~ →
  resolvido por preferência (2026-09-30, pedido do responsável do produto):
  Configurações → "Plano automático favorece o chão"
  (`UserPreferences::workplane_prefer_ground`, padrão desligado = estilo
  Modo/C4D). Ligada, o chão vence sempre que a câmera está inclinada ao menos
  20° (`GROUND_BIAS_MIN_PITCH_DEGREES`); abaixo disso o chão ficaria "de lado"
  e volta o plano vertical. A face sob o cursor continua vencendo nos dois
  modos. Testes `ground_preference_wins_unless_the_camera_is_almost_level`,
  `workplane_ground_preference_is_a_setting`. O padrão a manter ainda deve ser
  validado com usuários (Onda 6).
- Pré-seleção de regiões fechadas no Draw (Onda 4) e da ferramenta Move com snap
  antes do arrasto.
- O caminho legado `module-model::profile_screen_to_plane` (egui) mantém a grade
  0,25; fora do escopo (crate legado).
