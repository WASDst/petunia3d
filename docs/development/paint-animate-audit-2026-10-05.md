# PAINT e ANIMATE — auditoria e plano de correção (05/10/2026)

Implementation-vs-Spec Gap Matrix pedida pelo responsável do produto depois
da rodada DRAW/POLY. Autoridade: [P3D-055](../bible/specs/p3d-055-paint-workspace.md),
[P3D-056](../bible/specs/p3d-056-pixel-brush.md), [P3D-057](../bible/specs/p3d-057-soft-brush.md),
[P3D-132](../bible/specs/p3d-132-paint-masks-face-selection-isolation.md),
[P3D-133](../bible/specs/p3d-133-decal-projection-layers.md),
[P3D-156](../bible/specs/p3d-156-decals-surface-details.md),
capítulos [39](../bible/foundations/39-pos-v1-decals-surface-details-facial-atlas.md),
[44](../bible/foundations/44-pos-v1-surface-paint-toolbox.md) e
[45](../bible/foundations/45-pos-v1-animate-acessivel-animacao-procedural.md),
[P3D-066](../bible/specs/p3d-066-animation-workspace.md) e P3D-169–174.

Método: leitura do código e dos caminhos executáveis (bridge Slint →
`petunia_module_paint` → `petunia_project`), com as queixas relatadas como
hipóteses a confirmar. Cada linha aponta a causa no código.

## 1. Decalques

| # | Requisito | Encontrado (causa) | Estado |
|---|---|---|---|
| D1 | Resolução adequada (P3D-156: "detalhe corretamente baked") | A textura do objeto nasce com **256×256** (`module-paint/src/lib.rs`, `ensure_*`), o PNG é reduzido a 512 px na importação e o decalque é composto **dentro** dessa textura com amostragem *nearest* (`PaintLayerStack::composite`). Um decalque de escala UV 0,3 ocupa ~77 texels. | BROKEN |
| D2 | Ficar onde foi colocado no 3D ("surface or UV attachment", cap. 39) | O decalque só existe em espaço UV (`center_uv`, `scale_uv`); entra no centro do UV (0,5; 0,5) e não sob o cursor. Um retângulo em UV atravessa costuras e ilhas: parte aparece em outro lugar do modelo; a proporção segue a densidade de texels de cada ilha, não a superfície. | FUNCTIONAL_BUT_DIFFERENT |
| D3 | Manipulação no 3D (mover/girar/escalar visual) | Só por modificadores no arrasto (`decal_drag_to`): sem Shift move o centro para o UV do impacto, Shift escala por arrasto vertical, Ctrl gira por arrasto horizontal; sem alças, sem gizmo, sem valor digitado, sem prévia do contorno no 3D; fora da gramática única (ciclo próprio). | RUDIMENTARY |
| D4 | Decalques animados (cap. 39: Decal Set, `variant_index`, step animation) | Inexistente: `DecalLayer` tem uma imagem só; nenhum track de variante no Animate. | MISSING |
| D5 | Desempenho ao mover | Cada evento recompõe a textura inteira (O(W×H) por camada de decalque), sem recorte pela caixa do decalque. | RUDIMENTARY |
| D6 | SVG nítido | Fonte SVG guardada e re-rasterizável (`rerasterize`); limitado pelo D1. | PARTIALLY_COMPLIANT |
| D7 | Bake para BaseColor, save/load | `bake_decal_to_raster` e serialização com campo opcional existem; herdam D1/D2. | PARTIALLY_COMPLIANT |

## 2. Pintura e seleção

| # | Requisito | Encontrado (causa) | Estado |
|---|---|---|---|
| S1 | Pintar respeita a seleção (P3D-132) | A restrição só existe com a opção "mascarar pela seleção" ligada, e ela nasce **desligada** (`state.rs`, `paint_isolate_selection: false`). Selecionar faces no PAINT não restringe nada até o usuário achar a opção. | FUNCTIONAL_BUT_DIFFERENT |
| S2 | Nenhuma tinta fora da área permitida, inclusive em costuras | Pincel 3D (`stamp_dabs_3d`) pula faces não permitidas, mas rasteriza as permitidas com sangria de `BLEED_PX` em UV **sem** máscara de cobertura: a sangria pinta texels de faces vizinhas não selecionadas quando as ilhas UV se encostam. O 2D aplica a máscara (com 1 px de sangria). | BROKEN |
| S3 | Seleção visível e editável no 2D | O canvas 2D mostra o UV, mas não destaca as faces selecionadas nem permite selecionar nele. | MISSING |

## 3. Pincéis e representação na viewport

| # | Requisito | Encontrado (causa) | Estado |
|---|---|---|---|
| B1 | Cursor mostra tamanho | Anel de `brush-size × 16 px` no 3D, igual ao carimbo; círculo plano de tela, mesmo onde a superfície inclinada transforma a pegada em elipse. | PARTIALLY_COMPLIANT |
| B2 | Cursor mostra dureza | Ausente (sem anel interno do núcleo duro). | MISSING |
| B3 | Cursor mostra opacidade/força | Preenchimento fixo de 14%, independe de força e opacidade. | MISSING |
| B4 | Cursor em todos os pincéis | Só Brush/Eraser/Airbrush/Pixel; Smudge/Blur/Dodge/Burn/Spray/Clone sem cursor; canvas 2D **sem cursor nenhum**. | PARTIALLY_COMPLIANT |
| B5 | Tamanho em px de tela no 2D e no 3D (BrushSettings) | No 2D o raio é `size_px × 0,5` em **texels** (`stamp_dabs_2d`): com zoom o mesmo tamanho pinta áreas diferentes no 2D e no 3D. | FUNCTIONAL_BUT_DIFFERENT |
| B6 | Dureza vale para Soft/Eraser/Airbrush (P3D-057) | `dab_falloff`: a Borracha usa rampa linear fixa (`1 − t`) e ignora a dureza. | BROKEN |
| B7 | Opacidade × fluxo, presets, estilos, Smudge/Blur/Dodge/Burn/Spray/Clone | Implementados (`StrokeBuffer`, `BrushStyle`, presets). | COMPLIANT (preservar) |

## 4. ANIMATE

| # | Requisito | Encontrado | Estado |
|---|---|---|---|
| A1 | Workspace acessível no build | Pill ANIMATE atrás da feature `animation-workspace`, desligada por padrão (ADR 006: só aparece quando aceito). | COMPLIANT com a decisão; bloqueia teste com usuários |
| A2 | Camada 1: rig + Motion procedural com prévia (P3D-170) | `animate.rs` + `animate_session.rs` + geradores de `motion.rs`; testes `animate_shell`. | PARTIALLY_COMPLIANT (sem teste do iniciante) |
| A3 | Rig Roles & IK (P3D-169) | `rig_roles.rs`, `ik.rs`. | PARTIALLY_COMPLIANT |
| A4 | Ghosts & Trajectories (P3D-171) | Só um rastro em `ik.rs`; sem overlay no shell. | RUDIMENTARY |
| A5 | Reference Image Sequence (P3D-172) | Nada. | MISSING |
| A6 | Secondary Motion & Ragdoll (P3D-173) | Nada. | MISSING |
| A7 | Layered Animation (P3D-174) | Nada (sem camadas/peso por clip). | MISSING |
| A8 | Camada "Refinar": keyframes/timeline no shell Slint | Keyframes existem no modelo (`animation.rs`, glTF); timeline só no egui legado. | MISSING no Slint |
| A9 | Export de animação | `gltf_rig.rs` escreve clips. | PARTIALLY_COMPLIANT |

## 5. Plano de correção (ordem)

1. **Seleção que vale (S1, S2)**: seleção de faces no PAINT restringe a pintura
   por padrão (a opção vira "pintar fora da seleção"); sangria do 3D limitada a
   texels de calha (sem dono) ou de faces permitidas.
2. **Pincel legível (B1–B6)**: cursor com núcleo duro e preenchimento pela força,
   em todos os pincéis, no 3D e no 2D; tamanho em px de tela no 2D; dureza na
   Borracha.
3. **Decalque de superfície (D1, D2, D5)**: âncora 3D (ponto, normal, tangente,
   largura em mundo) projetada nos texels pela posição 3D de cada texel
   (`rasterize_face_near` já entrega a posição), amostragem bilinear, recorte
   pela caixa; textura com resolução escolhível (256–4096) e padrão maior.
4. **Manipulação de decalque (D3)**: alças na gramática única (mover pela
   superfície, girar, escalar, largura digitada), contorno projetado no 3D.
5. **Decalque animado (D4)**: Decal Set com variantes e track `variant_index`
   (step) no Animate; bake de flipbook como fallback de export.
6. **ANIMATE (A4–A8)**: ghosts/trajetórias no viewport, timeline simples de
   keyframes no Slint, camadas aditivas; secundário e sequência de referência
   depois.

Cada item fecha com testes e atualização desta matriz.

## 6. Entregas

| Item | Entrega | Estado após |
|---|---|---|
| S1 | Política escolhida pelo responsável (opção c, 05/10): seleção **parcial** de faces restringe sozinha; a opção do painel ("Sempre mascarar") mantém a máscara mesmo com tudo ou nada selecionado. `selection_masks_paint` em `module-paint/src/engine.rs`. Teste `partial_selection_restricts_painting_without_the_toggle`. | COMPLIANT |
| S2 | `PaintRestriction::mask`: a sangria de 1 px só cai em calha (texel sem face dona); o pincel 3D passa a consultar a mesma máscara do 2D. Teste `restriction_mask_never_bleeds_into_unselected_neighbors`. | COMPLIANT |
| B2–B4 | `BrushCursor` compartilhado: anel externo = tamanho, anel interno = núcleo duro (dureza; ausente no Pixel), preenchimento = opacidade × fluxo, ponto central; em todos os pincéis de traço (Brush, Eraser, Airbrush, Pixel, Smudge, Blur, Dodge, Burn, Spray, Clone), no viewport e no canvas 2D. | COMPLIANT — pegada elíptica em superfície inclinada (B1) segue como círculo de tela. |
| B5 | Fim da dualidade `canvas_brush` × `paint_radius` no uso: o 2D usa o descriptor do viewport (tipo pela ferramenta, tamanho pelo slider em px de tela) convertido para texels pela escala em que a textura aparece (`canvas_brush_settings`). Teste `canvas_brush_uses_the_viewport_descriptor_in_screen_pixels`. | COMPLIANT |
| B6 | Borracha segue a dureza (`dab_falloff`); só o Pixel é sempre duro. Teste `eraser_follows_hardness_like_the_soft_brush`. | COMPLIANT |
| — | Canvas 2D deixava de respeitar a proporção da textura (imagem esticada até o widget); agora cabe sem deformar. | Corrigido |
| D2 (dados) | `DecalAnchor` (ponto, normal, tangente, largura e alcance em mundo) em `DecalLayer.anchor`: projetor ortogonal no espaço do objeto; cada texel recebe a cor pela posição 3D (`surface_samples`, via `rasterize_face_near`), só em faces voltadas para o projetor; atravessa costuras e mantém a proporção da imagem. Composição completa, por tiles e bake usam a malha do asset; serialização com campo opcional (projetos antigos abrem como decalque UV). Testes `surface_decal_*`. | PARTIALLY_COMPLIANT — a importação ainda cria decalque UV no centro do atlas; colocar sob o cursor fica para a etapa de manipulação (D3). |
| D1 (parcial) | Amostragem bilinear no lugar de *nearest* nos decalques UV e de superfície. | PARTIALLY_COMPLIANT — resolução da textura (256 px) ainda não é escolhível. |
| D7 | Bake usa o tamanho da textura do asset (antes caía em 512×512 porque a camada ativa, o próprio decalque, não tem canvas). | Corrigido |
| D2 (UI) | Decalque novo (Adicionar, importar PNG/SVG) nasce fixado no ponto do objeto sob o centro da vista (ou o ponto visto na direção do centro do objeto), largura = 30% do maior lado. Arrasto no viewport: mover refixa sob o cursor, Shift muda a largura em mundo, Ctrl gira; Esc restaura sem histórico. Inspector mostra "Largura" no lugar de Posição/Escala UV; o contorno de prévia é o retângulo do projetor. Testes `new_decals_are_attached_to_the_surface_under_the_view_center`, `surface_decal_drag_moves_scales_rotates_and_cancels`. | COMPLIANT — alças visuais na gramática única seguem em D3. |
| D1 | Textura de pintura nasce com 1024 px (era 256) e a resolução é escolhível no painel Canvas (256 / 512 / 1024 / 2048), reamostrando todas as camadas numa entrada do histórico. Teto do `Canvas` sobe de 1024 para 2048 (`Canvas::MAX_SIDE`, também na importação glTF e na sangria de exportação). Decalques importados até 1024 px (eram 512). Teste `paint_texture_resolution_is_choosable_and_undoable`. | COMPLIANT — 4096 fica de fora: com o orçamento de histórico de 256 MiB, um snapshot (~192 MiB) deixaria um único passo de undo. |
