# 09 — Matriz de testes e aceite

**Princípio:** `Approved != Implemented != Verified`. Testes sugeridos são especificação, não execuções realizadas. Na entrega de code agents, reportar `pass/fail/blocked/not run` para cada conjunto exercitado.

## 9.1 Seleção / gizmo

| Test ID | Setup e ação | Resultado esperado |
|---|---|---|
| SEL-T01 | Drag sobre face sem alça | não inicia Move; mantém geometria |
| SEL-T02 | Pointer vazio + drag | box selection; modo crossing/window configurável |
| SEL-T03 | Hover sobre vertex/edge/face próximos | candidato correto, destaque coerente e sem flicker |
| SEL-T04 | Alternar Select Through | oclusão/picking alterado só quando ativado |
| SEL-T05 | Gizmo + mesh coexistem na tela | hit de handle explícito prevalece |
| SEL-T06 | Esc durante Move numérico | rollback exato, Undo stack não incrementada |
| SEL-T07 | Face e aresta sobrepostas selecionadas | deduplicação de vertex updates |
| SEL-T08 | Pai + filho selecionados | nenhuma dupla transformação hierárquica |
| SEL-T09 | Shared/Individual rotate | centros/arranjos mudam segundo política |
| SEL-T10 | Auto snap pivot; move smooth | snap só nos contextos decididos |
| SEL-T11 | Local/World/Normal/View e frame congelado | orientação previsível durante drag |
| SEL-T12 | Mesh + shape paramétrico | sem conversão silenciosa; undo atômico |
| SEL-T13 | Zoom extremo e HiDPI | gizmo com hit targets estáveis |
| SEL-T14 | Hover toolbar/controles de frame | overlays não bloqueiam seleção involuntariamente |

## 9.2 Percepção espacial / render

| Test ID | Setup e ação | Resultado esperado |
|---|---|---|
| VSP-T01 | Clay Soft vs material | documento/materiais idênticos ao sair |
| VSP-T02 | orbitar busto | sombreamento/contornos legíveis, sem flashing |
| VSP-T03 | fundo claro/escuro | seleção e contornos distinguíveis |
| VSP-T04 | scale de torso olhando frontal | dimensão relevante e preview original legíveis |
| VSP-T05 | rotacionar objeto com bbox | world/local diferença correta |
| VSP-T06 | multi selection | bounds agregada sem duplicar geometria |
| VSP-T07 | source mesh vs evaluated modifier | medida rotulada por representação |
| VSP-T08 | zero dimension + scale to dimension | erro estável sem divisão por zero |
| VSP-T09 | unidades genéricas da cena | não inventar cm/m |
| VSP-T10 | pinned measure + topology edit | remap válido ou invalidação clara |
| VSP-T11 | Depth Comparison ponto-ponto | distância euclidiana e delta por eixo corretos |
| VSP-T12 | contato vs interseção | tangência não classificada automaticamente como volume comum |
| VSP-T13 | Depth X-Ray vs Select Through | visibilidade não altera picking sem ação |
| VSP-T14 | Companion View manual | câmera principal inalterada, seleção sincronizada |
| VSP-T15 | Companion durante undo/cancel | ambas as views restauradas corretamente |
| VSP-T16 | Suggested companion | sugestão contextual, não popups repetitivos |
| VSP-T17 | Qt GL resize/context recreate | recursos liberados/recriados sem crash |

## 9.3 Section, Thickness e Organic Profiles

| Test ID | Setup e ação | Resultado esperado |
|---|---|---|
| SEC-T01 | cortar cubo manifold | região preenchida/contorno correto |
| SEC-T02 | cortar plane open mesh | linhas, sem face interior inventada |
| SEC-T03 | cortar toro com furo | anéis internos mantidos, sem fechar buraco |
| SEC-T04 | plane coincide with face/edge/vertex | sem segmentos duplicados/loop falso |
| SEC-T05 | cut intersects 3 disconnected limbs | componentes separados, medidas por componente |
| SEC-T06 | move/rotate/flip plane | update e cancel determinísticos |
| SEC-T07 | clipping vs geometry split | clipping não altera documento nem seleção de face sintética |
| SEC-T08 | section object anchored/world anchored | transforma segundo semântica escolhida |
| SEC-T09 | cross section hollow shell | não confunde parede com diâmetro externo |
| THI-T01 | paired surface measurement | references e unidades explicitadas |
| THI-T02 | directional ray curved surface | método/estimativa e possível ambiguidade rotulados |
| THI-T03 | thin heatmap com limiar configurado | escala/legenda acessíveis e sem valores arbitrários |
| ORG-T01 | 3 stations em antebraço | width/depth variam segundo contorno real |
| ORG-T02 | path curvo cauda | frames sem flips espontâneos |
| ORG-T03 | bifurcação torso+braço | segmentos distintos, sem single axial line falsa |
| ORG-T04 | parametric profile edit | comando explícito, histórico e undo preservados |

## 9.4 Modeling Feedback

| Test ID | Operação | Resultado esperado |
|---|---|---|
| MF-T01 | Extrude positivo/negativo | distância signed, ghost, cap/sides preview |
| MF-T02 | Inset colapsando topologia | diagnóstico antes de commit e sem crash |
| MF-T03 | Bevel além do raio válido | feedback de constraint e solução sem corromper mesh |
| MF-T04 | Boolean open operands | incompatibilidade explícita, sem autocorreção oculta |
| MF-T05 | Mirror torso assimétrico | preview sem impor simetria automática |
| MF-T06 | Sculpt stroke | brush footprint e valor de influência corretos |
| MF-T07 | Remesh com skin bind | alerta remap necessário antes de commit |
| MF-T08 | partial selection incompatible | targets afetados/ignorados explicitamente, confirmação |

## 9.5 Smart Rig

| Test ID | Setup e ação | Resultado esperado |
|---|---|---|
| RIG-T01 | desativar auto | bones/weights/animações preservados |
| RIG-T02 | manual create/reparent joint | skeleton válido e Undo/Redo |
| RIG-T03 | rig preset com landmark manual | preview editável antes de bind |
| RIG-T04 | pose test e Esc | restaura rest/current corretamente |
| RIG-T05 | live append cauda duas vezes no mesmo event | no duplicate bones via idempotência |
| RIG-T06 | move locked joint via upstream resize | lock preservado ou conflito explícito |
| RIG-T07 | change material only | no weight solver/incremental graph change |
| RIG-T08 | merge/Remesh de mesh skinned | weights rebind/remap ou erro claro, nunca indices reutilizados |
| RIG-T09 | IK cadeia degenerada | no panic/NaN, erro estável |
| RIG-T10 | paint weights manual mask + live | override preservado |
| RIG-T11 | glTF export + reload | joints, bind, weights, clips correspondem |
| RIG-T12 | save/load .petunia projeto legado | preserva dados antigos sem perda |
| RIG-T13 | modelo criatura não humanoide | modo manual funcional sem rig template |
| RIG-T14 | preview com 2 poses e alteração base | explicit Rest/Pose/Corrective semantics |

## 9.6 Matriz de plataforma e acesso

| Categoria | Casos de aceite |
|---|---|
| Linux | Qt 6 nativo, GL 3.3, software fallback, X11/Wayland conforme suporte |
| Windows | Qt nativo, resolução/HiDPI, input tablets e GL availability |
| GPU modesta | sem AO avançado, gizmo/seleção/medidas continuam funcionais |
| Foco teclado | F6/atalhos configurados, tab order, fallback input numérico, Esc |
| Leitor de tela | AccessibleMeasurement/Selection/RigModel, informação não só GL |
| Redução de movimento | sem transições/depth pulses importantes obrigatórios |
| Persistência | preferences vs session vs document annotations separadas |
| Segurança | entrada malformada, NaN, negative scale, topologia inválida, huge model |

## 9.7 Evidências recomendadas por PR

- Testes unitários de serviço Rust e regras de transação (incluindo property/fuzz quando útil).
- Fixtures geométricas sintéticas: cube, torus, open plane, hollow cylinder, connected/unconnected limb, 3-part quadruped, skinned mesh with modifier.
- Testes de integração CXX-Qt com `bridge_test`/`editor_parity` e focused commands.
- Capturas de viewport Qt nativo com estado `before`, `preview`, `after`, `cancel`, em 100% e escala HiDPI; metas de interação não podem ser comprovadas apenas por screenshot.
- Testes de perda de contexto GL/resize/múltiplas viewports com evidência.
- Benchmark reproduzível de 1k, 100k e 1M triângulos (somente se ambiente comportar) e cargas de rig; não inventar FPS.
- Logs com SHA, OS, versões Qt/Rust, comandos completos, retorno, link ou path da evidência; estados `pass / fail / blocked / not run`.
