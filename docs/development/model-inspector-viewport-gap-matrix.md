# MODEL Inspector / viewport — Implementation-vs-Spec Gap Matrix (rodada 2)

Escopo: pedido de 23/09/2026, 11 pontos. Auditoria inicial da UI Slint de produção, bridge, Core e renderers. Autoridade: Livro Vivo 05, 23, 24, 36 e decisão explícita do usuário de revisar a baseline para Parts como primeira seção do Inspector direito. O shell continua viewport-first e sem docking irrestrito.

| # | Requisito | Estado inicial | Evidência / delta necessário |
| --- | --- | --- | --- |
| 1 | Parts dentro do Inspector direito; Parts, Transform, Material, Object colapsáveis, responsivos | BROKEN | Parts é drawer separado que sobrepõe o trilho esquerdo; Context não rola, ordem atual é Boolean/Transform/Object/Material e ações soltas. Revisar baseline/ADR e migrar sem perder o acesso em janela compacta. |
| 2 | Transform geral + parâmetros de primitiva; Object info discreta | RUDIMENTARY | Vector3Field transacional existe; `PrimitiveDescriptor` existe somente durante a criação e não persiste no Asset, logo raio/lados não podem virar controles pós-criação sem novo contrato de dados/Undo. Object info ocupa seção grande. |
| 3 | Select único com click/box-drag; Lasso | PARTIALLY_COMPLIANT | `pending-box-select` já dá click/box com Select; botão Box Select duplicado continua na shelf. Não há lasso nem seleção por polígono no Core. |
| 4 | Move, Rotate, Scale e gizmo combinado; ícones claros | PARTIALLY_COMPLIANT | Três ferramentas e gizmos operam; não há gizmo combinado nem iconografia distinta suficiente. |
| 5 | Quatro ícones circulares de shading distinguíveis | FUNCTIONAL_BUT_DIFFERENT | Quatro SVGs novos usam metáfora de cubo; dois modos expostos (`Material`/`Rendered`) divergem dos modos canônicos `Textured`/`Silhouette` e precisam de implementação/ADR, não só renomeação. |
| 6 | Nome + descrição em tooltip de ferramentas, ícones e campos | MISSING | Há accessible-label, mas nenhum Tooltip na UI Slint; usar componente nativo compartilhado, traduções `TextId` e foco de teclado. |
| 7 | Point/Edge legíveis; sem dots poluidores em Face | PARTIALLY_COMPLIANT | Discos Point e faixas Edge existem em GPU/software, mas contraste/tamanho/hit ainda exigem QA; `compute_asset_overlay` desenha dots em todos os centros de face. |
| 8 | Wire overlay independente do modo Wireframe, combinável com X-Ray | BROKEN | Estado `show_wireframe_overlay` já existe e renderers o respeitam, mas botão do popover chama `view.toggle_wireframe`, que alterna o modo-base, não o overlay. |
| 9 | Movimento por atalho previsível como modal do Blender | PARTIALLY_COMPLIANT | G/R/S iniciam sessão modal e mouse move sem clique, mas falta teste com ponteiro inicial longe do pivô, zoom/ortográfica/perspectiva, movimento contínuo, confirmação/cancelamento e mudança de restrição sem salto. |
| 10 | Silhueta/overlay de esfera corretos | BROKEN | Contorno Object usa adjacência front/back com `camera.forward` constante e projeta segmentos sem clipping/oclusão; verificar esfera triangulada e corrigir o algoritmo. |
| 11 | Marca/menus alinhados à esquerda; menu fecha ao click-away | BROKEN | `HorizontalLayout` do chrome não preenche explicitamente a largura; callback `click-away-requested` fecha a pilha, mas não sincroniza `menu-open` ao shell. |

Esta matriz será atualizada por evidência de código, teste headless, captura nativa e gates. Nenhum item vira `COMPLIANT` apenas porque um controle foi desenhado.

## Matriz de ferramentas MODEL/viewport — rodada de interação

Escopo solicitado: Loop Cut com descoberta e preview por hover, wheel sem
interferência de navegação, Push/Pull, Slice, Profile e selector de pivot.
Referências: P3D-027, P3D-029, P3D-034, P3D-083, P3D-131; Livro Vivo 02, 03 e
14. Comportamentos Blender foram conferidos no manual 5.2 para Loop Cut/Slide,
Extrude e Pivot Point. A meta é reaproveitar o modelo mental de preview,
confirmação e cancelamento, respeitando a semântica própria do Petunia.

| Requisito | Estado antes | Evidência e delta | Estado após implementação |
| --- | --- | --- | --- |
| Loop Cut pode iniciar pela ferramenta sem seleção prévia de aresta | BROKEN | `begin_loop_cut()` exigia `selected_edges`; `LoopRing::discover()` já oferece a validação correta. Tool armada passa a descobrir anel pela aresta visível sob o cursor. | PARTIALLY_COMPLIANT — descoberta/seleção apenas em edge quad elegível; falta QA visual nativo com vários tipos de topologia. |
| Preview do corte acompanha hover e não altera documento | BROKEN | `LoopRing::preview()` existia, mas não era projetado pelo VM; mesh só mudava depois do início da sessão. Agora o shell projeta segmentos para overlay e hover não registra Undo nem altera malha. | PARTIALLY_COMPLIANT — coberto por teste do bridge; comparação visual nativa ainda pendente. |
| Wheel ajusta a quantidade de loops | PARTIALLY_COMPLIANT | Scroll já mudava cuts durante sessão; agora também muda cuts durante o preview hover. O limite continua 1–32 e o mesmo evento não chega ao zoom. | PARTIALLY_COMPLIANT — teste de integração Slint valida callback Cuts e ausência de callback Zoom; falta reprodução com dispositivo físico/trackpad. |
| Navegação não rouba gestures durante ferramentas modais/interativas | BROKEN | Handler Slint sempre emitia Orbit/Pan e Zoom; callbacks não filtravam tool ativa. Navegação fica suspensa em Loop Cut, Slice, Profile e modal de ferramenta. | PARTIALLY_COMPLIANT — guard bridge + teste headless de wheel; orbit/pan e touchpad ainda pedem QA de hardware. |
| Loop Cut posiciona, permite slide e confirma/cancela em uma transação | PARTIALLY_COMPLIANT | A sessão de preview/slide/undo já existia, mas dependia de aresta previamente selecionada. Click no edge hover cria sessão; Enter confirma e Esc restaura. | PARTIALLY_COMPLIANT — teste cobre hover→count→place→commit único; falta teste do gesto de slide nativo no novo início por hover. |
| Push/Pull está visível e trabalha com preview transacional | PARTIALLY_COMPLIANT | `ModalKind::PushPull`, kernel e modal parametrizado já existiam; a shelf Slint não expunha a ação. Foi adicionado botão direto e dica localizada. | PARTIALLY_COMPLIANT — integração reutiliza modal existente; testes de kernel/modal já existem, falta teste de gesto do controle recém-exposto. |
| Slice mantém os dois lados por padrão | BROKEN | `CutSession::compute_slice()` chamava `slice_plane(..., true)`, descartando um lado. Agora usa `false`, preservando ambos e sem cap por padrão conforme Livro Vivo 14. | PARTIALLY_COMPLIANT — teste confirma divisão topológica nos dois lados; preview visual atual ainda é guia 2D do plano, não renderização da mesh resultante. |
| Slice bloqueia navegação e cancela sem commit | PARTIALLY_COMPLIANT | Já existia sessão e confirmação no release; Slice agora desativa orbit/pan/zoom enquanto está armada ou arrastada. | PARTIALLY_COMPLIANT — session cancel existente; falta gesture regression com Esc durante arrasto e QA visual do plano. |
| Profile permite desenho, fechamento, preview e saída por extrude/revolve | RUDIMENTARY | Estado e algoritmos existiam no módulo, mas o shell de produção Slint não ativava a captura nem fornecia fluxo de gerar. Adicionados seleção de ferramenta, preview da linha, fechamento explícito, profundidade e ações Generate/Revolve. Extrude e Revolve respeitam frame right/up/normal do Profile. | PARTIALLY_COMPLIANT — teste headless fecha perfil e gera mesh com um Undo; falta precisão do plano sob vistas ortográficas/perspectiva e interação de cancelamento visual. |
| Pivot tem selector acessível no viewport e política atual chega ao transform core | RUDIMENTARY | Enum, cálculo e uso pelo modal já existem no domínio; Slint não mostrava selector. Adicionado popover localizado para Median, Bounds, Cursor e Individual Origins. | PARTIALLY_COMPLIANT — teste valida escolha Cursor e VM; Active Element permanece fora da enum/spec aprovada e casos de seleção múltipla ainda precisam auditoria P3D-027. |

### Fontes Blender consultadas (histórico)

Desde o ADR 007 (2026-09-29), o manual do Blender não é referência de interação; a referência é a constituição 11 e o capítulo 46 do Livro Vivo. As fontes abaixo ficam como registro desta rodada.

- [Loop Cut](https://docs.blender.org/manual/en/latest/modeling/meshes/tools/loop.html)
- [Loop Cut and Slide](https://docs.blender.org/manual/en/latest/modeling/meshes/editing/edge/loopcut_slide.html)
- [Extrude Faces](https://docs.blender.org/manual/en/latest/modeling/meshes/editing/face/extrude_faces.html)
- [Transform Pivot Point](https://docs.blender.org/manual/en/latest/editors/3dview/controls/pivot_point/index.html)

No Blender, o Loop Cut parte de uma aresta sob o cursor em malha elegível, exibe
uma prévia do edge ring, usa a roda para aumentar/reduzir cortes e mantém uma
etapa modal para posicionar/slide antes da confirmação. O Petunia reaproveita
esse ciclo, mas limita a descoberta a quad rings válidos e informa falha para
triângulo/ngon/non-manifold em vez de adivinhar o caminho. Profile continua
sendo uma ferramenta shape-first do Petunia, não uma cópia de uma tool Blender.

## Reconciliação desta rodada

| # | Estado após a implementação | Evidência e limite ainda aberto |
| --- | --- | --- |
| 1 | PARTIALLY_COMPLIANT | Parts foi integrado ao Inspector direito; seções usam altura de conteúdo, scroll e recolhimento independente. Busca/filtro/ordenação/contagem compartilham uma faixa; altura da linha reage ao slider no Inspector e no drawer compacto. Teste headless verifica alinhamento, acessibilidade e resizing nos dois layouts. Capturas nativas em 1034×1012 e 850×700 confirmaram a ordem, Alt+clique e acesso pelo drawer compacto. Faltam lista longa e persistência do estado entre sessões; ainda falta aceite visual nativo para a nova separação de seções. |
| 2 | PARTIALLY_COMPLIANT | Object ficou compacto, com Duplicate/Delete dentro da seção. Parâmetros paramétricos da primitiva continuam ausentes: `PrimitiveDescriptor` não é persistido no Asset; requer contrato de proveniência, regeneração transacional e Undo. |
| 3 | PARTIALLY_COMPLIANT | Select mantém clique/caixa; botão de Box redundante foi removido e Lasso adicionado. Caixa/laço agora detectam arestas que cruzam a região, com testes geométricos. Ainda faltam seleção de região inteiramente dentro de uma face, cobertura de oclusão e teste E2E do gesto. |
| 4 | PARTIALLY_COMPLIANT | Quatro ferramentas e handles distintos do gizmo combinado foram adicionados; teste de picking de move/scale/rotate passou. Falta QA de precisão do gesto e dos estados de foco/hover. |
| 5 | PARTIALLY_COMPLIANT | SVGs circulares distintos foram adicionados aos quatro modos expostos. Nomes/semântica de Material/Rendered versus os modos canônicos ainda exigem reconciliação de produto. |
| 6 | PARTIALLY_COMPLIANT | Componentes reutilizáveis de ação, ferramenta, campo e seção receberam Tooltip; rótulos principais de MODEL foram traduzidos via `TextId`. Controles diretos antigos e descrições de algumas seções ainda precisam de cobertura. |
| 7 | PARTIALLY_COMPLIANT | Dots de Face removidos; Points/Edges ganharam presença visual nos dois renderers. Hit target e legibilidade sob DPI alto/cores customizadas ainda precisam de QA nativo. |
| 8 | COMPLIANT | `view.toggle_wire_overlay` alterna somente o overlay e preserva o modo-base; teste de independência com X-Ray e shading passou. |
| 9 | PARTIALLY_COMPLIANT | Modal por atalho aceita movimento no mesmo gesto; clique de confirmação não vaza para seleção. Teste de gesto cobre o fluxo básico; faltam matriz completa de câmera/restrição e QA nativo. |
| 10 | PARTIALLY_COMPLIANT | Silhueta de esfera usa orientação local de cada face e rejeita projeção inválida; teste de finitude/limites passou. Falta comparação visual em múltiplas câmeras e X-Ray. |
| 11 | PARTIALLY_COMPLIANT | Marca/menus foram alinhados no chrome; click-away sincroniza o estado de menu. Testes de estado passaram; falta QA de monitor pequeno e navegação por teclado. |

`COMPLIANT` acima significa somente o delta específico verificado, não paridade global com Blender/Cinema 4D/Plasticity/Blockbench. O checkpoint continua incremental e as pendências não foram reclassificadas como prontas.

## Rodada 3 — Revisão do Inspector exigida pelo produto (24/09/2026)

Pedido explícito do responsável do produto após uso: sidebar translúcida, alça de
recolhimento por proximidade, rail colapsado em pílulas de ícone, limpeza das
seções Material/Object, stack de modifiers no padrão Blender e painel único de
ferramenta. Referências conferidas no manual Blender 5.2 (Sidebar N-panel,
Properties por ícones, Modifier Stack, Material Slots, Adjust Last Operation) e
no fluxo do legado egui (`crates/ui/src/properties_panel.rs`,
`tool_properties_popover.rs`). Achados que contrariam o pedido foram registrados
em vez de aplicados em silêncio: o Blender **não** usa sidebar translúcida sobre
a viewport (Sidebar é região opaca) nem **um** único painel inferior (Status Bar
persistente + Adjust Last Operation transitório coexistem); a decisão final é do
produto e vale para o shell Slint. Detalhes em
[`ADR 004`](../architecture/adr/004-inspector-translucido-alca-modifiers.md).

| # | Requisito | Estado inicial | Delta aplicado |
| --- | --- | --- | --- |
| R1 | Painel com fundo translúcido (viewport visível através) | BROKEN — `surface`/`surface-raised` opacos | `surface-raised.with-alpha(0.9)` no painel e no rail; seções mantêm fundo opaco para legibilidade |
| R2 | Alça só visível por proximidade; clique recolhe; sem textos "Inspetor"/seta/"MODEL"; módulos afastados da borda | PARTIALLY — chevron permanente + header com 3 textos; padding-right 12px | Faixa de proximidade de 14px revela a alça (chevron em pílula); header removido; padding-right 20px |
| R3 | Rail colapsado legível: pílulas separadas com hover | BROKEN — textos rotacionados colados | 5 pílulas de ícone (Parts/Transform/Material/Object/Modifiers) com hover e Tooltip; clique expande e abre a seção |
| R4 | Títulos de seção via `TextId` | BROKEN — "Transform"/"Material"/"Object" hardcoded | `label-tab-transform/material/object` nos títulos |
| R5 | Material no fluxo do legado (slot + gerenciar primeiro, editar depois) | FUNCTIONAL_BUT_DIFFERENT — ações no rodapé, losango accent inútil | Ações Assign/New/Duplicate/Remove sob a lista de slots; swatch mostra a cor base real; Assign em destaque |
| R6 | Object sem ruído; Quick Actions em seção própria | PARTIALLY — microlinha redundante + customize dentro de Object | Microlinha removida; Quick Actions viram seção própria após Modifiers |
| R7 | Modifiers como stack Blender | FUNCTIONAL_BUT_DIFFERENT — reorder/apply funcionam; visual diverge | Botão Add no topo; toggle com ícone Monitor (viewport); setas viram ChevronUp/Down + X; ordem top-aplica-primeiro preservada |
| R8 | Painel único de ferramenta (params + HUD), fora do canto inferior | BROKEN — Tool Options + HUD simultâneos embaixo | Card único dentro da viewport (topo-esquerda, como o popover legado): params ao vivo durante modal, valores do HUD caso contrário; oculto em repouso |
| R9 | Strings do painel via `TextId` | MISSING — "Cuts", presets e dicas hardcoded | `ui.profile_cuts/presets/add_rect/add_circle/canvas_hint` em en/pt-BR |

## Reconciliação da rodada 3 (24/09/2026)

| # | Estado após a implementação | Evidência e limite ainda aberto |
| --- | --- | --- |
| R1 | PARTIALLY_COMPLIANT | `surface-raised.with-alpha(0.9)` no painel e no rail; seções opacas preservam contraste do texto. Sem captura nativa de aceite ainda. |
| R2 | PARTIALLY_COMPLIANT | Faixa de 14px revela a alça com fade de 120ms; clique recolhe; header removido; padding-right 20px. Falta QA de sensibilidade da faixa em DPI alto. |
| R3 | PARTIALLY_COMPLIANT | 5 `InspectorPill` 36x36 com hover, Tooltip e teste headless de espaçamento (≥8px) e labels. Ícones são do set Lucide (Monitor/Settings), não réplicas Blender. |
| R4 | COMPLIANT | Títulos usam `label-tab-transform/material/object`; nenhum título de seção hardcoded restante. |
| R5 | PARTIALLY_COMPLIANT | Ordem slot → Assign → Surface → Emission → Advanced → Albedo; swatch real; New/Dup/Del como icon-buttons com Tooltip. Falta aceite de fluxo com usuário. |
| R6 | PARTIALLY_COMPLIANT | Microlinha removida; Quick Actions em seção própria colapsável (nada perdido). Ordem Parts→Transform→Material→Object preservada. |
| R7 | PARTIALLY_COMPLIANT | Add no topo, Monitor, Chevrons + X; reorder/apply inalterados e cobertos por testes. Sem drag-grip `::::` (fora do set de ícones). |
| R8 | PARTIALLY_COMPLIANT | Card único em `viewport-region` (12,56); params xor HUD; oculto em repouso; teste headless de presença/ausência. Painel antigo e HUD de baixo removidos. |
| R9 | COMPLIANT | 5 novos `TextId` com en/pt-BR, plumbing no bridge e `docs-generate --check` verde. |

## Rodada 4 — Módulos independentes: ancorar, flutuar e fixar (24/09/2026)

Pedido explícito do responsável do produto após a rodada 3: cada módulo do
Inspector direito passa a ser independente, com abrir/recolher, ancorar ou
flutuar e "pin" duplo (manter aberto + fixar asset), persistência por módulo e
arrasto do painel flutuante em qualquer ponto da tela. Decidido com o produto
que "flutuante" = card sobreposto **dentro do canvas**, persistido e sem janela
do sistema operacional: o cap. 36 proíbe janelas OS e docking irrestrito, e o
termo "floating panel" em Blender/C4D significa justamente o oposto. Detalhes em
[`ADR 005`](../architecture/adr/005-modulos-inspector-dock-float-pin.md).

| # | Requisito | Estado inicial | Delta aplicado |
| --- | --- | --- | --- |
| M1 | Módulo com estado próprio (aberto, ancorado, posição) | PARTIALLY — um `open` booleano por seção, sem posição nem âncora | `SectionLayout` por seção (`docked`, `x`, `y`, `pin_open`, `pinned_asset`); ordem canônica preservada |
| M2 | Card flutuante arrastável em qualquer ponto | MISSING — nenhuma camada flutuante existia | `FloatSectionCard` em `Window`, com `for` sobre `section-states`; arraste pelo header com clamp in-canvas |
| M3 | Corpo único, instanciado ancorado e flutuante | STUB — o corpo vivia inline no painel | 6 componentes `*Body` com `@children`; coluna ancorada e camada flutuante compartilham o mesmo markup |
| M4 | Recolher-tudo sem destruir pin aberto | BROKEN — toggle global ignorava fixação | `toggle_all_sections` pula seções com `pin_open`; semântica de mão única (`close_all` = todas abertas) |
| M5 | Pin duplo: manter aberto + fixar asset | MISSING | Dois controles independentes no header; `pin_open` vence recolher-tudo/painel, `pinned_asset` fixa o UUID exibido |
| M6 | Fixar asset redireciona leitura **e** escrita | MISSING — seção lia e mutava o asset ativo | `section_asset()` (UUID fixado, fallback ativo) em Material/Object/Modifiers; `find_modifier_owner` aplica mutação no asset exibido |
| M7 | Persistência por módulo entre sessões | PARTIALLY — só tema/workspace persistiam | `UserPreferences.section_layouts`; restore no startup e gravação em cada transição |
| M8 | Afordances por `TextId` | MISSING — "Dock"/"Drag"/"Pin" hardcoded no markup | `label-section-dock/drag/pin-open/pin-asset/unpin-asset` em en/pt-BR |
| M9 | Reancorar devolve à ordem canônica | N/A | `section-dock-toggled` volta a seções à ordem; nenhum docking irrestrito foi introduzido |

## Reconciliação da rodada 4 (24/09/2026)

| # | Estado após a implementação | Evidência e limite ainda aberto |
| --- | --- | --- |
| M1 | COMPLIANT | 17 testes de `section_layout` cobrem restore, transições e sanitização; `toggle_all_sections_skips_pinned_open` fixa a semântica de pin aberto. |
| M2 | PARTIALLY_COMPLIANT | `dragging_floating_card_header_reports_clamped_move` valida arraste + clamp; `floating_material_card_anchors_at_state_position` valida ancoragem. Clamp usa folga fixa de 80px em vez da largura real do card: com Inspector largo, o card encosta na borda direita. Sem QA com ponteiro físico. |
| M3 | COMPLIANT | Um `*Body` por seção; teste headless garante cópia única do corpo quando a seção flutua. |
| M4 | PARTIALLY_COMPLIANT | Reverter tudo com `pin_open` respeitado. Falta knob de teclado para o toggle global. |
| M5 | PARTIALLY_COMPLIANT | Dois pins independentes no header e estado persistido; falta pin de asset por clique no Outliner (hoje segue a seleção ativa). |
| M6 | PARTIALLY_COMPLIANT | 4 testes de bridge cobrem leitura (Object/Modifiers/Material) e escrita (`add_modifier` no asset fixado). Falta cobrir toggle/reorder/apply de modifier no asset fixado. |
| M7 | PARTIALLY_COMPLIANT | Roundtrip dos 6 módulos e benchmark de gravação (131µs) da Fase 1; falta testar restauração com janela fora da tela (sanitização já existe, sem teste end-to-end). |
| M8 | COMPLIANT | 5 `TextId` novos em en/pt-BR, `docs-generate --check` verde. |
| M9 | PARTIALLY_COMPLIANT | Reancorar preserva ordem canônica; não há snap de drop nem docking irrestrito, por decisão de V1. |
