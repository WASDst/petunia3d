# Workspace DRAW — regiões e Push/Pull — Implementation-vs-Spec Gap Matrix (Onda 4, parte 1)

Escopo: primeira entrega da Onda 4 do [ADR 006](../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)
— o núcleo geométrico e de interação do DRAW. O pill DRAW, o trilho de
ferramentas de forma e a separação de MODEL em DRAW/POLY ficam para a parte 2
(regra "workspace não implementado não aparece").
Autoridade: [capítulo 02](../bible/foundations/02-workflow-modelagem-shape-first.md)
("Profile → Volume", "Draw on Face"), [capítulo 36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
(workspace DRAW), [constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md)
(gesto atômico, "Última operação", Esc restaura) e
[capítulo 45](../bible/foundations/45-pesquisa-interacao-modelagem-referencias.md)
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

## Pendências registradas

- **Parte 2 da Onda 4:** pill DRAW (e POLY na Onda 5), trilho de ferramentas de
  forma, aparência de viewport para leitura de forma (capítulo 05), plano por
  3 pontos/face + aresta, "Converter em polígonos".
- Imprint geral: região que cruza as arestas da face (dividir por corte),
  região com furos sobre face, região que atravessa várias faces coplanares.
- Região solta empurrada para o lado negativo: hoje o topo atravessa o fundo
  (sólido invertido); o esperado é extrudar a partir do fundo.
- Perfil consumido: depois do Push/Pull o perfil continua no documento e a
  região volta a aparecer no hover (como no Plasticity); avaliar com usuários.
- Oclusão da região só considera a malha ativa; outros objetos não escondem.
- Custo: o arranjo é O(n²) por plano, recalculado só quando o documento muda
  (cache por `revision_clock`).
