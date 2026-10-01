# Estudo — viewport com 1, 2 ou 4 vistas (2026-09-30)

Escopo: decidir **se e como** o shell oferece 1, 2 ou 4 vistas 3D sem perder
fluidez. Motivação do responsável do produto: a vista quádrupla ajuda no DRAW
(desenhar na vista ortográfica do plano e conferir as outras ao mesmo tempo).
Este documento é estudo e plano; **não altera a baseline** (cap. 36 permanece
"até 2 viewports opcionais" até a decisão abaixo ser aceita e o capítulo
revisado).

## 1. Estado atual (auditado no código)

| Item | Hoje |
| --- | --- |
| Vista principal | GPU (wgpu) com MSAA 4×, `WgpuViewport::render_frame` — um alvo, uma câmera. |
| Vista secundária | `split_view::SplitView`: **software** (`Software3dViewport`), câmera própria, presets Persp/Front/Back/Left/Right/Top, recusada abaixo de `2 × 480 px`. |
| Cache da secundária | Chave por hash das revisões do documento + câmera + opções; só re-renderiza quando a chave muda. |
| Edição | Só na vista principal (seleção, ferramentas e pintura). A secundária é de navegação. |
| Renderer | `PetuniaRenderer::update(...)` envia **cena + câmera** juntas (buffers de malha, grade, referências e o uniforme da câmera num único passo). |

Consequência: abrir a 2ª vista já custa CPU proporcional a *pixels × triângulos*
a cada mudança do documento; três vistas extras no mesmo caminho triplicariam
isso (e o usuário de "PC modesto", P3D-126, é o alvo).

## 2. Orçamento (o que "sem comprometer performance" significa aqui)

Medidas a cumprir antes de qualquer promoção a `COMPLIANT`:

1. **Vista ativa**: ≤ 16,7 ms por quadro no modelo de referência de 50 mil triângulos
   (capítulo 46) com 1, 2 e 4 vistas abertas.
2. **Vistas passivas**: custo **zero** quando nada mudou (chave de cache por
   vista); máximo de 1 re-render de vista passiva por quadro (round-robin).
3. **Memória**: alvos de cor + profundidade por vista ≤ 2 × (largura × altura × 4 B)
   por vista passiva (sem MSAA nas passivas).
4. **Mínimos**: 480 × 360 px lógicos por vista (capítulo 36); se não couber, o
   layout recua de 4 → 2 → 1 com aviso, nunca empilha vistas ilegíveis.

## 3. Alternativas

| # | Alternativa | Custo de CPU | Custo de GPU | Complexidade | Veredito |
| --- | --- | --- | --- | --- | --- |
| A | Generalizar `SplitView` para `Vec<SplitView>` (3 vistas de software) | **Alto** (×3 a cada edição) | nenhum | Baixa | Só serve como passo 0 (2 vistas). Não escala a 4 em PC modesto. |
| B | Todas as vistas em GPU, **uma cena e N câmeras** (recomendada) | Baixo: a cena sobe uma vez por revisão | N passes baratos (ortográficas sem MSAA) | Média | **Recomendada.** |
| C | Quad com vistas passivas em **resolução reduzida** (50%) + upscale | Baixo | Baixo | Média | Complemento de B para o modo "econômico". |
| D | Janelas de sistema separadas | — | — | — | **Proibido** pelo capítulo 36 (sem janelas de sistema, sem docking). |

## 4. Proposta (alternativa B, em fases)

### F0 — Separar cena de câmera no renderer (base de tudo)
- `PetuniaRenderer::update` passa a ter duas metades: `update_scene(...)`
  (buffers de malha, grade, referências, seleção) e `set_camera(view_id, ...)`
  (uniforme + bind group por vista). A cena só é reenviada quando a revisão do
  documento muda (hoje já existe o contador de revisões).
- Cada vista tem seu par `(cor, profundidade)`; a principal mantém MSAA 4×, as
  passivas não.
- Sem mudança de UI. Critério de saída: teste por pixels com 2 câmeras na mesma cena
  (o de `wireframe_edges_have_constant_pixel_width` já mostra como ler pixels).

### F1 — Modelo de layout no core do shell (toolkit-neutro)
- `ViewLayout { Single, Split2 (lado a lado), Quad }` e
  `ViewSlot { id, preset, camera, resolution_scale }`; funções puras
  `layout_rects(layout, w, h) -> Vec<Rect>` e `downgrade(layout, w, h)` com os
  mínimos da seção 2. Testável sem janela.
- Quad padrão para o DRAW: **Persp (principal) + Top + Front + Right**, com a
  vista ortográfica do plano de trabalho ativo destacada (ADR 007: "a câmera
  nunca se move sozinha" — o destaque só indica, não move).

### F2 — Política de render das vistas passivas
- Chave de cache por vista (mesma ideia do `SplitView::key`) e fila round-robin:
  no máximo uma passiva por quadro; a vista sob o ponteiro tem prioridade.
- Durante um gesto na vista ativa (arrastar, pintar), as passivas congelam a
  última imagem e atualizam ao soltar (ou a 10 Hz), para nunca competir com o
  gesto.
- `resolution_scale` adaptativo: 1,0 → 0,5 se o orçamento do quadro estourar
  duas vezes seguidas; volta a 1,0 depois de 2 s ocioso.

### F3 — Shell (markup Slint)
- Uma região por vista com as mesmas alças de navegação da secundária atual
  (órbita/pan/zoom, presets). O seletor 1/2/4 vira um grupo de 3 botões na barra
  da viewport. Isso é trabalho de interface e segue o agente de UI.
- **Edição em qualquer vista** (desenhar o perfil na vista Top, por exemplo) é
  decisão separada: exige que o bridge resolva o raio pela câmera *da vista sob o
  ponteiro*. Recomendação: liberar primeiro só para o DRAW (perfil, Push/Pull) e
  para o Poly Pen, que já calculam tudo a partir de um raio.

## 5. Riscos

| Risco | Mitigação |
| --- | --- |
| Buffers de cena duplicados por vista | F0 compartilha os buffers; só o uniforme e os alvos são por vista. |
| Latência percebida ao pintar/arrastar | F2 congela passivas durante o gesto. |
| Hit-test ambíguo com 4 vistas | Todo evento carrega o `view_id` da região; o bridge só usa a câmera dessa vista. |
| Fallback sem GPU (software) | Quad só é oferecido com GPU; sem GPU o máximo é Split2 em resolução reduzida. |
| Contradizer a baseline | Exige revisão do capítulo 36 (vistas 1/2/4) com ADR próprio antes de aparecer na UI. |

## 6. Decisão pedida ao responsável do produto

1. Aprovar a revisão do capítulo 36: "1, 2 ou 4 vistas opcionais" (hoje "até 2").
2. Edição em qualquer vista: só DRAW/Poly Pen primeiro (recomendado) ou todas?
3. Quad padrão do DRAW: Persp + Top + Front + Right (recomendado).

Enquanto isso não for decidido, **nenhum código de UI é alterado**; F0 e F1 são
seguras (sem mudança visível) e podem entrar antes da decisão.

## 7. O que já foi entregue desta proposta

- Nada de F0–F3 ainda. Este estudo é o registro da análise; o item segue em
  `PLANNED` na matriz de pendências de [`viewport-gap-matrix.md`](viewport-gap-matrix.md).
