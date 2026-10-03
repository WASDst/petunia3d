# Workspace DRAW — regiões e Push/Pull — Implementation-vs-Spec Gap Matrix (Onda 4, parte 1)

Escopo: primeira entrega da Onda 4 do [ADR 007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md)
— o núcleo geométrico e de interação do DRAW. O pill DRAW, o trilho de
ferramentas de forma e a separação de MODEL em DRAW/POLY ficam para a parte 2
(regra "workspace não implementado não aparece").
Autoridade: [capítulo 02](../bible/foundations/02-workflow-modelagem-shape-first.md)
("Profile → Volume", "Draw on Face"), [capítulo 36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
(workspace DRAW), [constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md)
(gesto atômico, "Última operação", Esc restaura) e
[capítulo 46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md)
(regiões do Plasticity, Push/Pull do SketchUp).

## Auditoria antes da mudança (30/09)

| Aspecto | Encontrado | Classificação |
| --- | --- | --- |
| Regiões fechadas | Só o perfil ativo fechado era reconhecido; perfis que se cruzam não formavam regiões | MISSING |
| Destaque de região no hover | Inexistente | MISSING |
| Profile → Volume | Botões Generate/Revolve criam asset separado; sem arrastar a região | FUNCTIONAL_BUT_DIFFERENT |
| Draw on Face: somar/cortar na face hospedeira | Inexistente: o volume nunca era integrado à face | MISSING |
| Gesto de Push/Pull | Ferramenta persistente da Onda 2, só sobre faces existentes | PARTIALLY_COMPLIANT |

## Requisitos

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 1 | Regiões fechadas, inclusive de perfis que se cruzam no mesmo plano | `crates/mesh/src/arrangement.rs`: arranjo planar (divisão nas interseções, poda de pontas, faces por meia-aresta, furos atribuídos à menor região que os contém). `AppState::profile_region_planes` agrupa perfis coplanares e tessela Bézier. Testes `overlapping_profiles_make_three_regions`, `nested_profile_becomes_a_hole_of_the_outer_region`, `open_line_across_a_profile_splits_it_and_tails_are_ignored`, `crossing_profiles_expose_their_overlap_as_a_region`. | COMPLIANT (2026-09-30) |
| 2 | Regiões destacadas no hover | `region_hit_at` (cache por revisão do documento, oclusão pela malha ativa) + `region_hover_commands`; tinta translúcida com `evenodd` (furos vazados) no Draw (sem perfil aberto) e no Push/Pull. Teste `draw_tool_highlights_a_closed_region_on_hover`. | PARTIALLY_COMPLIANT — sem captura nativa; oclusão só pela malha ativa. |
| 3 | Arrastar a região gera volume, com valor digitável | Push/Pull sobre uma região: `begin_region_gesture` abre Extrude na mesma `ToolSession` (valor segue o cursor, digitação vence o mouse). Teste `push_pull_drag_on_a_region_imprints_and_extrudes_in_one_undo`. | COMPLIANT |
| 4 | Sobre uma face: puxar soma, empurrar corta (imprint + extrusão) | `crates/mesh/src/imprint.rs`: região estritamente dentro de uma face plana vira face própria; o anel é dividido em duas faces por duas pontes válidas (polígonos simples, anti-horários). Testes `imprint_then_extrude_makes_a_closed_boss_and_recess`, `imprint_works_on_a_triangle_host`, `region_on_a_face_is_imprinted_and_extruded_as_one_undo`. | PARTIALLY_COMPLIANT — região que cruza arestas da face, região com furo sobre face e várias faces hospedeiras ficam pendentes. |
| 5 | Região no vazio gera sólido fechado | `region_sheet`: fundo invertido + topo selecionado (até um furo); asset novo "Forma" (`draw.shape_name`). Teste `free_region_becomes_a_closed_solid_in_a_new_asset`. | PARTIALLY_COMPLIANT — empurrar para o lado negativo inverte o sólido (ver pendências). |
| 6 | Gesto = 1 Undo; Esc restaura exatamente; "Última operação" ajustável | Prelúdio no core: `AppState::begin_modal_after_prelude` (commit grava a partir do estado anterior ao imprint; cancelar ou confirmar sem mudança volta a ele) e `LastOperation` guarda o documento pós-prelúdio para reaplicar sem imprint duplicado. Testes `cancel_restores_the_document_exactly`, `last_operation_reapplies_the_region_push_in_the_same_undo`, `escape_during_a_region_push_restores_the_face`. | COMPLIANT |
| 7 | A primitiva paramétrica não é regenerada por cima do imprint | `freeze_active_primitive_for_command` antes do imprint. | COMPLIANT |

## Parte 2 — DRAW e POLY no seletor (2026-09-30)

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 8 | `DRAW / POLY / PAINT / UV` no lugar de MODEL | `ModelingMode { Draw, Poly }` no bridge; o workspace interno continua `Model` (DRAW e POLY compartilham documento, seleção, câmera, snapping e Inspector, como no ADR 007). `WorkspaceSegment` com quatro segmentos; títulos e descrições por `TextId` (`workspace.*`). Teste `draw_and_poly_share_the_modeling_workspace`. | PARTIALLY_COMPLIANT — sem captura nativa. |
| 9 | Trilho de ferramentas de forma no DRAW | Barra contextual filtrada por modo: DRAW = Sketch, Retângulo, Círculo, Push/Pull (sempre visível), Duplicar; POLY = Extrude, Inset, Bevel, Knife, Loop Cut, Subdivide, Merge, Slice, Connect, Make Face, Spin, Dissolve (+ Push/Pull, Duplicar). Trocar de modo devolve para Select uma ferramenta que o trilho novo não oferece; nada é convertido. Teste `switching_mode_drops_a_tool_the_new_rail_does_not_offer`. | PARTIALLY_COMPLIANT — Revolve e plano de trabalho ainda ficam no card do perfil. |
| 10 | Conversão explícita DRAW → POLY | Hoje as formas do DRAW já são malhas (o Push/Pull gera polígonos); "Converter em polígonos" só fará sentido quando as formas forem paramétricas. | MISSING (depende de formas paramétricas) |

**Padrão:** POLY, para preservar o fluxo atual; DRAW como padrão para quem
começa é decisão a validar com usuários (Onda 6).

## Pendências registradas

- **Restante da Onda 4:** conjunto de seleção `Shape / Curve / Point / Region`
  no DRAW, aparência de viewport para leitura de forma (capítulo 05), plano por
  3 pontos/face + aresta, formas paramétricas + "Converter em polígonos".
- Imprint geral: região que cruza as arestas da face (dividir por corte),
  região com furos sobre face, região que atravessa várias faces coplanares.
- ~~Região solta empurrada para o lado negativo inverte o sólido~~ → corrigido:
  a extrusão negativa de uma folha solta inverte as faces e o sólido continua
  voltado para fora (também ao reajustar a "Última operação").
- Perfil consumido: depois do Push/Pull o perfil continua no documento e a
  região volta a aparecer no hover (como no Plasticity); avaliar com usuários.
- Oclusão da região só considera a malha ativa; outros objetos não escondem.
- Custo: o arranjo é O(n²) por plano, recalculado só quando o documento muda
  (cache por `revision_clock`).

---

# Parte 3 — formas persistentes e edição de nós (2026-09-30)

Motivação (responsável do produto): uma forma sem preenchimento sumia da viewport
ao clicar fora dela; faltava inserir pontos depois de fechar e alternar pontos
entre reto e curva, inclusive no retângulo e no círculo.

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 11 | Formas sempre visíveis no DRAW | `draw_shapes.rs`: `region_shapes_commands` (tinta leve de **todas** as regiões fechadas, com furos) e `profile_outline_commands` (contorno dos perfis fora de edição, abertos ou fechados), por cache de revisão. Só no DRAW. | PARTIALLY_COMPLIANT — sem captura nativa |
| 12 | Reativar uma forma | Clicar no contorno de um perfil inativo o torna o perfil em edição (plano de trabalho incluído); não vale enquanto há um perfil aberto sendo desenhado nem com um volume em edição. Ctrl+clique força um ponto novo. | PARTIALLY_COMPLIANT |
| 13 | Inserir nó depois de fechar | Clicar na aresta do perfil **fechado** divide o segmento (`SplineResource::split_segment`: interpolação na polilinha, De Casteljau na Bézier — a forma não muda) e deixa o novo ponto pronto para arrastar. Testes `splitting_a_polyline_segment_keeps_the_shape`, `splitting_a_bezier_segment_preserves_the_curve`. | COMPLIANT (domínio); interação sem captura |
| 14 | Ponto reto ↔ curva | `SplineResource::set_point_curved` (alças alinhadas pela direção dos vizinhos) + duplo clique no nó + botão de curvas (age no ponto selecionado; sem seleção, em todos). Vale em qualquer perfil (polígono, retângulo, círculo). Teste `a_point_toggles_between_straight_and_curved`. | COMPLIANT (domínio); interação sem captura |

## Pendências da parte 3

- Inserir nó em perfil **aberto** só por Ctrl+clique/forma fechada (clicar na aresta de um perfil aberto adiciona ponto novo, para não atrapalhar o desenho de formas que se cruzam).
- Clicar **dentro** da região para reativar a forma ainda não existe (só o contorno).
- Círculo continua sendo um polígono de N pontos; curvar todos os pontos o arredonda, mas não há "círculo paramétrico" editável por raio (depende das formas paramétricas do cap. 02).

## Checkpoint posterior — 02/10/2026 (`4a41951`)

A tabela acima registra a rodada original. O código posterior acrescenta
seleção Shape/Curve/Point/Region no shell, Depth Handle, plano por três
pontos/seleção e imprint com `geo::BooleanOps` para recortar regiões que
cruzam os limites da face. Esses deltas substituem a alegação de ausência
dessas capacidades, mas não encerram automaticamente todas as pendências.

| Requisito | Evidência atual | Classificação nesta auditoria |
| --- | --- | --- |
| Seleção específica DRAW | `app.slint`, `callbacks.rs` e `lib.rs` no commit `4a41951` | PARTIALLY_COMPLIANT — reprodução manual pendente |
| Depth Handle visual/interativo | `draw_shapes.rs` e `app.slint`; teste de projeção adicionado | PARTIALLY_COMPLIANT — testes não executados nesta auditoria |
| Plano por três pontos/seleção | `ProfileWorkplane::from_three_points`, `draw_profile.rs`; testes adicionados | PARTIALLY_COMPLIANT — validação de fluxo pendente |
| Imprint cruzando o limite de uma face | `crates/mesh/src/imprint.rs`, recorte booleano e testes adicionados | PARTIALLY_COMPLIANT — sem confirmação de furos e múltiplas faces hospedeiras |

A inspeção de 02/10 foi documental e estática; não confirma gates, captura
nativa ou teste com usuários. A evidência de compilação do PR #19 antecede
este checkpoint e não valida esses deltas.
