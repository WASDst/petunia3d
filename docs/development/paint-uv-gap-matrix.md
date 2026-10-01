# PAINT / UV — Implementation-vs-Spec Gap Matrix (2026-09-30)

Escopo: auditoria aprofundada do workspace PAINT (com "Preparar superfície"/UV)
contra P3D-055 a P3D-065, P3D-132 a P3D-134 e os capítulos 12, 15, 42 e 44, motivada
por três defeitos relatados pelo responsável do produto: pintar uma face pinta as
outras; "Mask to selection" e a trava de pincel não valem nem no 2D nem no 3D; e
faltam ferramentas previstas no caderno.

## 1. Auditoria antes da mudança

| Aspecto | Encontrado | Classificação |
| --- | --- | --- |
| UV padrão das primitivas | `Mesh::project_planar()` projeta cada face no plano do eixo dominante da normal: faces opostas (frente/trás, topo/base, esquerda/direita) caem **no mesmo retângulo** da textura. Pintar uma face pinta a oposta e, no cubo, todas as faces ocupam a mesma área `0..1`. **Causa raiz do "pintar uma face pinta todas".** | BROKEN |
| Auto Unwrap (xatlas) | `unwrap_fallback` gravava UVs **em texels do atlas** (sem normalizar) e **fazia a média por vértice** entre as costuras, colapsando os charts; a pintura usava `rem_euclid(1.0)` e "dava a volta". | BROKEN |
| Pincel 3D | Carimbava um disco **no espaço da textura** ao redor do UV do ponto de impacto: pegava qualquer face vizinha no atlas e ignorava a distância real na superfície. O tamanho do carimbo (texels) não era o do anel de preview (pixels de tela → mundo). | BROKEN |
| Mask to selection (P3D-132) | O parâmetro `isolate` só barrava o ponto de impacto (`face_hit_uv`); o disco de tinta vazava para as faces não selecionadas. Sem seleção, a máscara ligada pintava tudo. No canvas 2D nem era consultada. | BROKEN |
| Trava de pincel (`BrushLock`) e escopo (`FillScope`) | Existiam no estado e na UI, mas só o caminho de vértices os lia. Nenhum dos caminhos de textura (3D, 2D, forma, gradiente, balde) os respeitava. | BROKEN |
| Acúmulo de opacidade | Cada dab misturava sobre o pixel já misturado: dabs sobrepostos escureciam além da "força" (sem teto por traço). | FUNCTIONAL_BUT_DIFFERENT |
| Ferramenta Fill no PAINT | Ao **escolher** a ferramenta, `fill_selection` pintava os vértices de todo o objeto com a cor atual (efeito colateral do id `fill`, que é compartilhado com o balde). | BROKEN |
| Oclusão | `pick_face_hit` testava só a malha ativa: pintava através de outros objetos. | BROKEN |
| Seleção no PAINT | Inexistente: não dava para trocar de objeto nem escolher a parte a pintar. | MISSING |
| Isolar objeto no PAINT | Existia só no menu de contexto do Outliner e em `Numpad /`. | PARTIALLY_COMPLIANT |

## 2. Requisitos e entrega

| # | Requisito (fonte) | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 1 | UV sem sobreposição por padrão (P3D-062, cap. 15) | `Mesh::layout_uv_charts`: charts por normal (40°), projeção no plano médio, densidade uniforme, empacotamento por prateleiras e ajuste final ao quadrado `0..1`; determinístico, preserva quads. Todas as primitivas e o OBJ sem UV usam o layout. Testes `default_planar_projection_overlaps_but_chart_layout_does_not`, `chart_layout_stays_inside_the_unit_square_and_keeps_quads`, `layout_is_deterministic`. | COMPLIANT (2026-09-30) |
| 2 | Auto Unwrap correto (P3D-064) | UV **por canto** a partir do `index_array` do xatlas, normalizada pelo tamanho do atlas (padding 2 px, bilinear). | COMPLIANT (sem teste visual) |
| 3 | Pincel 3D segue a superfície (P3D-062) | `Mesh::rasterize_face_near` + `PaintModule::stamp_dabs_3d`: pincel **esférico** de raio de mundo (o mesmo do anel: `world_radius_for_px`), por face candidata; densidade de texel local define o custo; nunca alcança a face oposta (`dot(normal) < -0,5`) nem faces sem relação geométrica. Testes `spherical_dab_on_one_face_never_reaches_the_opposite_face`, `painting_one_face_does_not_paint_the_others`. | COMPLIANT |
| 4 | Máscara/isolamento por seleção (P3D-132) | `PaintRestriction` (faces elegíveis por traço) → candidatas no 3D e máscara de texels `CoverageMask` no 2D. Seleção vazia com a máscara ligada **não pinta** (a UI avisa). Testes `test_paint_mask_selection_isolation`, `mask_with_nothing_selected_paints_nothing`, `mask_restricts_the_2d_canvas_to_the_selected_faces`. | COMPLIANT |
| 5 | Trava de pincel (`FirstFace`/`SelectedFaces`) e escopos de preenchimento | Mesma restrição; balde, gradiente e formas devolvem os texels fora da máscara (`restricted_edit`). A semente 2D revela a face pelo texel (`faces_at_texel`). | COMPLIANT |
| 6 | Opacidade × fluxo (P3D-057) | `StrokeBuffer`: `mistura(base, cor, cobertura)`; *flow* acumula, *strength* é o teto; Airbrush acumula até 1. Teste `overlapping_flow_does_not_darken_past_the_strength_cap`. | COMPLIANT |
| 7 | Sem efeito colateral ao escolher Fill | O preenchimento de vértices só roda fora do PAINT. | COMPLIANT |
| 8 | Pintar não atravessa a cena | `paint_pick` descarta pontos cobertos por outros objetos (`ViewportSceneQuery::point_visible`). | COMPLIANT |
| 9 | Ferramenta de seleção no PAINT | Tool `select`: clique em outro objeto o torna ativo; clique no ativo seleciona a face (Shift soma; duplo clique seleciona a ilha UV); vazio limpa. Atalho `V`. | COMPLIANT (sem captura nativa) |
| 10 | Isolar objeto (hide/unhide) no PAINT | Botão no trilho + `Numpad /`/`/`; o isolamento acompanha a troca de objeto (`refresh_isolation`). | COMPLIANT (sem captura nativa) |

## 3. Ferramentas previstas — estado real

Fonte: P3D-055 (adendo), P3D-133, P3D-134, capítulo 44 e capítulo 42.

| Item | Situação |
| --- | --- |
| Brush / Pixel / Airbrush / Eraser / Fill / Picker | Existem; restrição por máscara e trava agora valem em todos. |
| Line / Rectangle | Existem (cor sólida). |
| **Círculo/Elipse** (cap. 44) | **Novo** (2026-09-30): elipse preenchida inscrita entre dois pontos. |
| **Gradient** | Linear existia **substituindo** os pixels; agora compõe por cima, com a opacidade do pincel e a cor final transparente. **Novo: radial.** Cores editáveis e dithering continuam fora. |
| Face / UV Island Fill | Existe (`FillScope`), restrito pela máscara. |
| **Smudge, Blur, Dodge, Burn, Spray, Clone** | **Novos** (2E). Clone: origem por Ctrl+clique. Sem pressão de tablet. |
| Ponta, ângulo, achatamento, mistura, estabilizador, jitter, espalhamento, presets | **Novos** (2E): `BrushStyle`/`BrushPreset`. |
| Decal (P3D-133) | Camada de decal transformável (existia só com um quadrado amarelo de teste). **Novo:** importar PNG/JPEG pelo botão de imagem da pilha de camadas (decodificação validada, lado maior limitado a 512 texels, proporção preservada, 1 Undo; arquivo inválido não muda o documento). **Faltam** SVG (exigiria `resvg` como dependência direta), máscara/clip, fonte ausente e bake/export. |
| Projection / Stencil | `MISSING`. |
| Clone / Patch (cópia de região com máscara) | O pincel Clone cobre o uso básico; Patch (remendo com costura) segue `MISSING`. |
| Path Paint | `MISSING` (o Spline Core existe; falta a ferramenta). |
| Effect Stack (P3D-134) | Pixelate, Posterize, Invert, **Grain, Levels (agora na UI), Brightness/Contrast e Hue/Saturation** — todos existiam no domínio; Levels não era oferecido na UI. |
| Surface Recipe (P3D-113) | Grafo headless existe; sem editor visual (por decisão). |

## 4. Pendências registradas

- Captura nativa e teste com usuários do fluxo de seleção/máscara.
- O caminho GPU da textura pintada nunca foi exercitado com GPU real neste ambiente.
- Sem tablet: a pressão não entra no motor (gancho previsto em `StrokeSampler`/`PointStabilizer`).
- Smudge no 3D acompanha só o dab original (a simetria repete o carimbo, não o borrão) e ignora saltos entre charts.
- Custo do pincel 3D: percorre todas as faces por dab; para malhas > 20 mil faces
  falta um índice espacial (grade uniforme por revisão).
