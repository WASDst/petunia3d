# Visual e acessibilidade — Implementation-vs-Spec Gap Matrix (Onda 6, parte 1)

Escopo: primeira entrega da Onda 6 do [ADR 007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md)
(visual premium, preferências de acessibilidade e testes com usuários).
Autoridade: [capítulo 05](../bible/foundations/05-viewport-shading-modos-visualizacao.md)
("Studio light padrão previsível", aparência por workspace),
[capítulo 46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md)
§2.4, §3.4 e §8.4–8.5, [constituição 03](../bible/constitution/03-invariantes-de-ui-ux-design-system-e-acessib.md).

## Auditoria antes da mudança (30/09)

| Aspecto | Encontrado | Classificação |
| --- | --- | --- |
| Tinta translúcida de hover e seleção | Hover 18% (ciano), seleção 32% (cor de seleção) na camada de seleção do renderer | COMPLIANT (preservado) |
| Luz de estúdio | Direção fixa no mundo (`LIGHT_DIR`): orbitando para trás, a forma fica só com luz ambiente; o X-Ray usava outra constante no shader | FUNCTIONAL_BUT_DIFFERENT |
| Raio do snap ajustável | `SnapSettings::radius_pixels` existia, sem controle na UI | PARTIALLY_COMPLIANT |
| Testes de aparência | Sem GPU no CI de nuvem: testes wgpu eram pulados | MISSING |
| Roteiro de teste com usuários | Inexistente | MISSING |

## Requisitos

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 1 | Luz de estúdio previsível e legível ao orbitar | `petunia_render::scene::studio_light_for_camera` (luz por cima do ombro de quem olha) usada pelo wgpu (sólido, textura e X-Ray pelo mesmo uniforme) e pelo viewport por software. Testes com leitura de pixels: `studio_light_following_the_camera_reads_the_same_from_any_side` (GPU) e `software_studio_light_follows_the_camera`. | COMPLIANT (2026-09-30) |
| 2 | "Rotação rápida da luz" / escolha do usuário | Preferência "Luz de estúdio acompanha a câmera" (padrão ligada; desligada = fixa no mundo), salva em disco. Teste `studio_light_preference_reaches_the_render_state`. | PARTIALLY_COMPLIANT — girar a luz por arrasto ainda não existe. |
| 3 | Raio do snap ajustável (cursores de área, Findlater et al. 2010) | Slider "Raio do snap" (4–48 px) em Configurações; salvo e restaurado; valor fora da faixa volta ao padrão. Teste `snap_radius_is_an_accessibility_setting_with_real_effect` (20 px encaixa com 30, não com 12). | COMPLIANT |
| 4 | Aparência verificável sem GPU física | Testes wgpu executam com Vulkan por software (Mesa lavapipe, `mesa-vulkan-drivers`); sem adaptador eles continuam pulando sem falhar. | PARTIALLY_COMPLIANT — o CI precisa instalar o pacote para rodar de fato. |
| 5 | Teste com usuários | [`user-test-protocol.md`](user-test-protocol.md): 7 decisões em aberto (Q1–Q7), 3 grupos (incluindo limitação motora), 7 tarefas, SEQ/SUS e critérios de decisão. | PARTIALLY_COMPLIANT — roteiro pronto; sessões não realizadas. |

## Parte 2 (2026-09-30)

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 6 | Linhas de largura constante em pixels, com antisserrilhado | As arestas viram faixas de dois triângulos expandidas no vertex shader (`WIDE_LINE_WGSL`), com largura em px lógicos × escala da tela; o MSAA 4× suaviza as bordas; os buffers não dependem da câmera. A pipeline `LineList` antiga das arestas foi removida (a grade continua em `LineList`). Teste por pixels `wireframe_edges_have_constant_pixel_width` (largura segue a configuração e não muda com o zoom). | COMPLIANT (2026-09-30) |
| 7 | Aparência por modo (capítulo 05) | `Mesh::to_classified_edges` (borda, não manifold ou dobra > 30°) + `EdgeMode`: DRAW = só arestas de feição; POLY = todas, as comuns mais finas e claras e as de feição reforçadas; PAINT/UV = faces limpas com o overlay opcional. Testes `classified_edges_mark_borders_and_creases_only` e, por pixels, `draw_reads_shape_and_poly_reads_topology`. | PARTIALLY_COMPLIANT — viewport por software mantém o comportamento antigo (pontos e plano de trabalho: ver parte 3). |

## Parte 3 (2026-09-30)

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 8 | Pontos visíveis no domínio Point (POLY) | Já existia: a camada de seleção desenha um disco de 3,5–5,5 px por ponto não selecionado no domínio Point (`update_selection_layer`). A parte 2 o listou como pendente por engano; nenhuma mudança de código. | COMPLIANT (auditado em 2026-09-30) |
| 9 | Plano de trabalho em destaque no DRAW (cap. 05) | `WorkplaneOverlay` + `append_workplane`: recorte translúcido com grade de 4 + 4 células e eixos reforçados, centrado na origem do perfil, com tamanho proporcional à altura visível (lê igual em qualquer zoom) e empurrado levemente para a câmera para não disputar profundidade com a face. Aparece no DRAW com a ferramenta de desenho quando o plano está decidido (travado ou perfil em edição); no automático, antes do 1º clique, o destaque da face sob o cursor já mostra o candidato. Testes `draw_highlights_the_workplane_only_once_it_is_decided` e, por pixels, `workplane_highlight_tints_the_plane_under_the_camera`. | PARTIALLY_COMPLIANT — viewport por software sem o destaque. |

## Parte 4 (2026-09-30)

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 10 | Seleção de objetos visível no viewport, com contorno de largura constante (cap. 46 §8.4, Rong & Tan) | Auditoria: no domínio Object o viewport GPU não destacava os objetos selecionados (`Selection.assets` nem chegava ao renderer; só componentes tinham camada). Delta: máscara dos selecionados (`Rg8Unorm`, R = selecionado, G = ativo) gravada antes do passe principal e composição em tela cheia que pinta os pixels a até N px da máscara, com a borda suavizada pela distância. N = espessura da seleção (1–4 px lógicos) × escala da tela; o ativo usa a cor de seleção e os demais a mesma cor mais escura. Busca direta no raio em vez de jump flooding: para 1–4 px é mais simples e barata; JFA só compensa para raios grandes. Só no domínio Object e no workspace de modelagem. Testes `object_outline_follows_the_object_selection_only_in_the_object_domain` e, por pixels, `selected_objects_get_a_constant_width_outline` (largura igual ao afastar a câmera; interior intacto). | PARTIALLY_COMPLIANT — o contorno aparece por cima de objetos que estejam na frente (sem teste de profundidade); não aparece no shading Wireframe (sem faces na máscara) nem no viewport por software. |

## Pendências registradas

- Contorno de seleção: esconder a parte encoberta por outros objetos; Wireframe
  e viewport por software.
- Matcap; oclusão ambiente (GTAO).
- Guias de aresta do domínio Edge ainda em `LineList` de 1 px.
- Aparência por modo e plano de trabalho no viewport por software.
- Coalescer renders por quadro (pendência da Onda 1).
- Girar a luz de estúdio por arrasto; filtragem de tremor e "lupa motora".
- Instalar Vulkan por software no CI para os testes de aparência.
