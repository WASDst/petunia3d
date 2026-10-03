# Pesquisa — próximos passos de PAINT e DRAW (2026-10-01)

Status na publicação (01/10): **pesquisa e análise, antes da implementação**. A shortlist da seção 5 foi autorizada pelo responsável e implementada em 02/10; [estado atual e limites](paint-draw-mvp-completion-gap-matrix.md). As tabelas de pesquisa continuam referências, não promessa de entregar todas as propostas. Pedido do responsável
do produto: (a) o que mais vale implementar em PAINT e DRAW fora das listas atuais;
(b) por que os booleanos saem em triângulos; (c) esforço de um *Shape Builder* 2D;
(d) SVG como decalque, Projection/Stencil e Path Paint. Método: leitura do caderno
(caps. 03, 04, 42, 43, 44; P3D-127, 156, 158, 165) e do código, uma sonda descartável
no kernel booleano e pesquisa externa (fontes no fim). Esforço: **S** ≤ 2 dias,
**M** 3–5 dias, **L** ≥ 1 semana (uma pessoa, incluindo testes e documentação).

## 1. Booleanos resultam em triângulos

### O que o caderno realmente diz

| Fonte | Regra |
| --- | --- |
| Cap. 04 ("Política normativa de Fuse e Cut") | O resultado booleano recebe **apenas cleanup seguro: degenerados, weld de pontos coincidentes e dissolução coplanar quando comprovadamente segura**. **Sem remesh nem retopologia automática.** Preview e Undo obrigatórios. "Smart Fuse" (cleanup + simplificação opcional, sempre visível) fica como evolução futura. |
| Cap. 03 | O authoring aceita **triângulo, quad e n-gon**; a triangulação é cache derivado. Booleano é fornecedor substituível, não fundamento. |
| Cap. 04 (Connect) | Dois contornos com contagens diferentes viram **faixa mista de quads e triângulos** (programação dinâmica sobre `n × m`, minimizando custo); 1:1 gera quads "sempre que a orientação permitir". |
| P3D-127 | Sem remesh como atalho técnico. |

Ou seja: o caderno **não promete quads** no resultado booleano — promete **dissolução coplanar**
(que produz faces maiores, inclusive n-gons) e proíbe remesh. "Quads onde recuperáveis" é a leitura
compatível; "só quads" não é (e contradiz o cap. 03).

### O que o código faz hoje

`AppState::apply_boolean` triangula as duas malhas, chama `boolean_meshes` (Manifold) e **grava o
resultado cru**: nenhuma dissolução coplanar, nenhum weld/cleanup, UV `[0,0]` em todos os cantos
(a pintura sobre o resultado fica inutilizável), `material_slot` e cores perdidos.

Sonda descartável (arquivo apagado; caixa 4×1×4 menos cilindro de 24 lados):

| Resultado | Valor |
| --- | --- |
| Faces | **112 triângulos, 0 quads** |
| Parede do furo | 24 grupos coplanares de **2 triângulos cada** — eram 24 quads antes do kernel |
| Tampa de cima e de baixo | 1 grupo coplanar de **28 triângulos** cada (anel: 4 cantos + 24 pontos do furo) |

A imagem enviada é exatamente o caso das tampas: o kernel triangula o anel por restrições e deixa
**leques longos** a partir de um único vértice (triângulos finos), em vez de uma faixa equilibrada.

### Proposta: `BooleanCleanup` (compatível com o caderno)

1. **Seguro**: weld por tolerância pequena e remoção de degenerados (já previstos).
2. **Regiões coplanares**: agrupar triângulos adjacentes por plano (normal e offset com tolerância) —
   é o passo clássico de simplificação do CSG exato.
3. **Contornos da região**: arestas que aparecem uma vez no grupo formam ciclos; ciclo contido em outro = furo.
4. **Por região**:
   - 1 contorno → **uma face n-gon** (ou quad, se tiver 4 pontos);
   - 2 contornos (anel, o caso da imagem) → **reusar `bridge_boundaries` do Connect**: faixa de quads e
     triângulos com custo mínimo (sem leques; com contagens parecidas dá quase só quads);
   - ≥ 3 contornos → fallback: manter a triangulação do kernel (marcada no preview).
5. **Pares de triângulos** que eram um quad (paredes de cilindro, lados de caixa) → **Tris to Quads
   conservador**: coplanares, união convexa, quase retangular, sem cruzar costura/material (o Blender usa
   limite de ângulo de 40° e exige quad convexo; add-ons de limpeza pós-booleano usam a mesma ordem:
   weld → dissolução limitada → tris para quads).
6. **UV/material/cor**: herdar do triângulo de origem (o Manifold pode carregar o id da face original) e,
   quando não der, rodar `layout_uv_charts` (já existe) — hoje o resultado perde tudo isso.
7. **Preview** com contagem antes/depois, Undo único e botão "Manter triangulação do kernel".

**Esforço: M (4–6 dias).** O que **não** vira quad: contornos com contagens muito diferentes
(ex.: 4 cantos × 24 pontos do furo) — ali sobram triângulos, sem inventar vértices (regra "sem remesh").
**Decisão pedida:** confirmar "quads onde recuperáveis, n-gon/faixa mista nos anéis" como o critério.

## 2. Shape Builder 2D (estilo Illustrator)

**Como funciona (Illustrator):** ao entrar, o aplicativo analisa as formas sobrepostas em faces e arestas
(arranjo planar); passar o mouse destaca a face; **arrastar** funde as faces tocadas, **Alt+arrastar** apaga,
**clique** extrai a face como forma independente.

**O que já temos:** `planar_regions` (arranjo planar com furos), `region_at` (hit-test), o ciclo de ferramenta
(`ToolSession`), destaque de região no hover e a crate `geo` já dependência do `petunia_mesh`
(`BooleanOps`: união, diferença, interseção; resultado `MultiPolygon` com furos).

**Trabalho que falta:**

| Parte | Esforço |
| --- | --- |
| Ferramenta: arrastar sobre regiões (caminho do ponteiro → regiões tocadas), Alt = apagar, clique = extrair | S–M |
| Operação: união das regiões tocadas / remoção / extração com `geo` e escrita de volta como perfis | S |
| Substituir os perfis de origem pelos resultados em **1 Undo**, com preview | S |
| **Fidelidade de curva**: hoje o arranjo trabalha em polilinhas tesseladas; o resultado perde as Béziers | L (separado) |

**Resposta direta:** um **MVP poligonal é M (3–5 dias)** e reaproveita quase tudo. Curvas fiéis (dividir os
nós de Bézier nas interseções, como o Illustrator faz ao "reconstruir o máximo possível dos caminhos
originais") somam outra semana. **Recomendo implementar o MVP e deixar curvas para uma segunda etapa**,
com o aviso "o resultado vira polilinha" visível. A mesma base entrega de graça os comandos **Unir / Subtrair /
Interseção / Excluir** (Pathfinder) sobre formas selecionadas.

## 3. SVG decalque, Projection/Stencil e Path Paint

O caderno já define as três (P3D-133, 156, 158, 165, cap. 43/44): **Decal = entidade viva reutilizável;
Projection = interação temporária que *commita* numa camada; Path Paint reaproveita o Spline Core e o
`SurfaceAttachment`; `Surface Manipulator` é compartilhado entre Decal e Projection; vocabulário
Apply / Keep Live / Bake.**

### SVG como decalque — **M (3–4 dias)**
- **Rasterização**: `resvg 0.48.1` **já está no `Cargo.lock`** (vem pelo Slint) com licença Apache-2.0 OR MIT
  (a MPL-2.0 era só até a 0.44; o `deny.toml` aceita as duas). Adicioná-lo como dependência direta do crate de
  projeto não traz versão nova. `usvg` não carrega fontes por padrão: **texto em SVG não renderiza** (aviso claro).
- **Modelo**: guardar o **SVG de origem** (bytes) + raster em cache; reraster quando a resolução do alvo muda.
  Campo novo com `#[serde(default)]` — projetos antigos continuam abrindo.
- **Segurança**: SVG é entrada não confiável — limite de tamanho (ex.: 2 MB), sem referências externas, limite
  de dimensão raster, erro vira mensagem sem tocar o documento (já é o padrão do importador de imagem).
- **Observação**: o decalque atual é uma **camada raster** com transformação UV; o P3D-156/158 prevê
  `DecalInstance` com `SurfaceAttachment` (aderir à superfície, espelhar, biblioteca). SVG é a ocasião de decidir se
  migramos para a entidade viva (**+M**) ou mantemos a camada.

### Projection / Stencil — **M (4–6 dias)**
Referências: no Blender, o *stencil* projeta a imagem **da câmera** e só pinta dentro dos limites do
estêncil (mover/escalar/rotacionar na tela, opacidade); *view plane* × *area plane*; no Substance Painter,
projeção e estêncil são interação de tela.
- Manipulador na viewport (mover, girar, escalar, opacidade, espelhar) — **o mesmo do decalque** (Surface Manipulator).
- **Commit**: para cada texel visível por uma face elegível, projetar o ponto de mundo no plano do estêncil e
  amostrar a imagem. Precisa de oclusão (a cena de consultas já é cacheada) e de corte de faces de costas.
  **A máscara/restrição que implementei nesta rodada vale aqui sem mudança.**
- Duas formas de uso: **pintar através do estêncil** (pincel limitado pela imagem) e **Aplicar projeção** (de uma vez).
- *Keep Live* continua sendo o Decal; Projection é sempre destrutiva por camada (como o caderno define).

### Path Paint — **M (3–5 dias) para Stroke; +M para Ribbon/Filled**
Referência (Substance Painter): ferramentas *Paint along path*, *Ribbon path*, *Filled path*, *Erase/Smudge along path*.
- O usuário clica pontos **sobre a superfície** (raycast → `SurfaceAttachment`, que já existe em `petunia_project`);
  a spline é amostrada por espaçamento (`samples_by_spacing`, já existe) e cada amostra vira um dab do motor 3D novo.
- Herda tudo: tamanho, fluxo, jitter, estilo de ponta, simetria, máscara, trava, borrão/clone ao longo do caminho.
- Pré-visualização antes de confirmar; 1 Undo. **Ribbon** (largura variável) e **Filled** (região entre o caminho) são a parte L.

### Ordem sugerida do pacote 3
SVG decalque → Path Paint (Stroke) → Projection/Stencil (depende do manipulador, que o SVG ajuda a fechar).

## 4. O que mais vale fazer — PAINT (fora das listas)

Critério: alto valor para **low-poly/jogos**, custo baixo sobre o que já existe, e **sem virar Substance/Photoshop**
(cap. 43/44). Fontes entre parênteses.

| # | Proposta | Por quê (evidência) | Esforço | Prioridade |
| --- | --- | --- | --- | --- |
| P1 | **Lock Alpha** (só pinta onde já há tinta) | Blockbench e Aseprite; trivial com o buffer de traço | S | Alta |
| P2 | **Linha reta com Shift** e **pixel-perfect** no traço livre | Blockbench (Shift = linha), Aseprite (pixel-perfect) — essencial em pixel art | S | Alta |
| P3 | **Pintura em "wrap"/tileável** + prévia do tile | Aseprite (modo tiled, grade 3×3) — texturas repetíveis | S–M | Média |
| P4 | **Rampa de paleta / shading ink** (clarear/escurecer *dentro da paleta*) | Aseprite (ink de sombreamento); guia low-poly: poucas cores base + sombras e luzes da mesma rampa | M | Alta (liga com a Palette P3D-148 que já existe) |
| P5 | **Dithering** (padrões Bayer) no pincel e no gradiente | O cap. 44 já cita "dithering/quantização futuro"; Aseprite | S–M | Média |
| P6 | **Painel "saúde do UV"**: overlay de checker, densidade de texel com normalização, ilhas que se sobrepõem ou minúsculas, padding em *px na resolução final* | Substance/Blender/C4D: padding de 4–16 px conforme a resolução e o mip; já existem `uv_diagnostics`, `texel_density` e o checker — falta expor no PAINT | M | **Alta** (evita o bug que corrigimos hoje voltar em silêncio) |
| P7 | **Dilatação (bleed) na exportação** em px configurável + aviso de mip | Substance ("dilation + transparency") | S | Alta |
| P8 | **Máscara por Material/Ilha** no pincel (além da seleção de faces) | Substance (geometry masks por ID/ilha) | S–M | Média |
| P9 | **Máscaras de camada** (não destrutivas) | P3D-132 prevê "se a arquitetura de camadas justificar" | M–L | Média |
| P10 | **Pintar canais**: roughness/metallic/emissão em cinza | P3D-062 prevê seleção de canal; hoje só Albedo | M | Média |
| P11 | **Clone a partir de outra camada/imagem de referência** | Blender (*clone from paint slot*) | S–M | Média |
| P12 | **Prévia de orçamento**: tamanho de textura por plataforma, nº de slots, overdraw de decal | Cap. 43 ("texture size budget presets", "material slot count warning") | S–M | Média |
| P13 | **Assar AO / altura para uma camada** (multiplicar) para sombreado estilizado | Substance (AO assado); P3D-160 (bake) | L | Baixa (só se o cap. 43 liberar) |
| P14 | **Sombra/AO por vértice** (cor de vértice) para low-poly sem textura | P3D-150 (vertex color) já existe | M | Baixa–média |

**Não recomendo:** pressão de tablet (o Slint não entrega pressão hoje), pintura PBR completa, pincéis de
escultura, qualquer remesh.

## 5. O que mais vale fazer — DRAW (fora das listas)

Referências: Plasticity (Offset Planar Curve/Region, Fillet, Trim, Split, Slot, polígono regular, arco, elipse),
Shapr3D/Fusion (restrições, cotas, espelho, padrão, offset, trim), Illustrator (Shape Builder, Pathfinder).

| # | Proposta | Por quê | Esforço | Prioridade |
| --- | --- | --- | --- | --- |
| D1 | **Shape Builder + Unir/Subtrair/Interseção** (item 2) | Núcleo do desenho de formas; base já pronta | M | **Alta** |
| D2 | **Arredondar cantos** (fillet/chamfer 2D) com raio numérico por ponto | Plasticity (*Fillet Curve/Vertex*): "reto → curva" com controle, em vez de só alças | S–M | **Alta** |
| D3 | **Espelhar na criação** (simetria em torno de um eixo do plano) | Fusion/Shapr3D (espelho com simetria): desenhar meia silhueta de personagem/prop | S–M | **Alta** |
| D4 | **Formas prontas**: polígono regular (N lados), retângulo arredondado, elipse, arco de 3 pontos, slot | Plasticity; hoje só retângulo e círculo | S cada | Alta |
| D5 | **Vetorizar a referência** (contorno da imagem de referência → perfil, com tolerância e simplificação) | O fluxo "shape-first" do cap. 02 parte de imagens de referência; `vtracer` (Rust, tem modo polígono) ou marching squares + Douglas–Peucker (sem dependência) | M | **Alta** |
| D6 | **Importar SVG como perfis** (caminhos → splines) | Reusa o parser do SVG decalque; `usvg` entrega os segmentos Bézier | M | Alta (junto com SVG decalque) |
| D7 | **Valor digitado por segmento** (comprimento/ângulo ao desenhar) | Fusion/SketchUp; o `ToolSession` já tem valor digitado | S–M | Alta |
| D8 | **Reduzir pontos / "Curva → N segmentos"** (Douglas–Peucker; low-poly pede contagem controlada) | Hoje a suavização é por tolerância, não por nº de segmentos | S | Alta |
| D9 | **Offset de curva/região** (contorno paralelo, cantos redondos/lineares) | Plasticity (*Offset Planar Curve/Region*, *Slot*); `i_overlay`, que o `geo` usa, tem API de offset (a verificar na versão fixada) | M | Média |
| D10 | **Trim / Split** em interseções | Plasticity (*Trim*, *Split Segment*); o arranjo planar já divide os segmentos | M | Média |
| D11 | **Extrusão com conicidade (draft) e torção; extrusão simétrica; chanfro no topo** | Plasticity; cap. 43 (Simple Deform) | M | Média |
| D12 | **Loft entre dois perfis** | Plasticity (*Loft*); Sweep e Revolve já existem | M–L | Média |
| D13 | **Plano por 3 pontos / por face + aresta** | Pendência da Onda 4 | S–M | Média |
| D14 | **Restrições e cotas completas** (estilo CAD) | Shapr3D/Fusion — **não recomendo**: contraria "sem virar CAD/DCC universal" | L | Não |

## 6. Decisões pedidas (máximo 3)

1. **Booleano**: aceitar "quads onde recuperáveis; n-gon e faixa mista nos anéis; nunca remesh" como critério?
   *(Recomendo sim; é o que o cap. 04 permite. "Só quads" exigiria vértices novos, que o caderno proíbe.)*
2. **Shape Builder**: MVP poligonal primeiro (M) e curvas fiéis depois (L)? *(Recomendo.)*
3. **Pacote inicial** a implementar quando você liberar. *Recomendo, nesta ordem:*
   **(a)** `BooleanCleanup` (item 1) → **(b)** Shape Builder MVP + Unir/Subtrair (D1) → **(c)** SVG decalque + importar SVG
   como perfil (D6) → **(d)** Path Paint (Stroke) → **(e)** Projection/Stencil. Em paralelo, os "S" de maior valor:
   P1, P2, P7, D2, D3, D4, D7, D8 e o painel de saúde do UV (P6).

## Fontes

- Blender — [Tris to Quads e limpeza pós-booleano (QuadFix Lite)](https://github.com/quadfix-tools/quadfix-lite), [Cleaning Up Boolean-Heavy Meshes](https://blenderartists.org/t/tutorial-cleaning-up-boolean-heavy-meshes-into-usable-quads-with-quadify-pre/1617574)
- CSG: [Exact predicates, exact constructions and combinatorics for mesh CSG](https://arxiv.org/pdf/2405.12949), [Concise Plane Arrangements for Low-Poly Surface and Volume Modelling](https://arxiv.org/pdf/2404.06154), [Polygon with holes](https://en.wikipedia.org/wiki/Polygon_with_holes)
- Shape Builder: [Illustrator Shape Builder Tool (guia)](https://illustratorhow.com/shape-builder-tool/), [Building Objects from Objects](http://www.adobeillustratorsmartnotes.com/illustrator/combining-objects/combining-objects.html)
- Polígonos 2D em Rust: [geo `BooleanOps`](https://docs.rs/geo/latest/geo/algorithm/bool_ops/trait.BooleanOps.html), [iOverlay](https://github.com/iShape-Rust/iOverlay)
- SVG: [resvg](https://crates.io/crates/resvg) (Apache-2.0 OR MIT desde a 0.45), [resvg/usvg/tiny-skia](https://deepwiki.com/linebender/resvg)
- Pintura 3D: [Blender — Texture Paint Tools](https://docs.blender.org/manual/en/2.80/sculpt_paint/texture_paint/tools.html), [Blender — Texture & Texture Mask](https://docs.blender.org/manual/en/latest/sculpt_paint/brush/texture.html), [Substance 3D Painter — Polygon fill](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/painting/paint-tools/polygon-fill), [Anchor point](https://helpx.adobe.com/substance-3d-painter/features/effects/anchor-point.html), [Substance — release notes](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/release-notes/all-changes)
- Pixel art / low-poly: [Aseprite](https://www.aseprite.org/release-notes/12), [Blockbench Wiki](https://blockbench.net/wiki/guides/blockbench-overview-tips/), [Lospec — Blockbench](https://lospec.com/software/blockbench)
- UV/textura para jogos: [Texel density e padding (Cinema 4D)](https://novedge.com/blogs/design-news/cinema-4d-tip-efficient-uv-island-packing-and-texel-density-workflow), [UV unwrapping para jogos](https://nastyrodent.com/uv-unwrapping-for-games/), [Texture inpainting e costuras de UV](https://www.tripo3d.ai/blog/texture-inpainting-uv-seams)
- Sketch/CAD: [Plasticity — Sketch](https://doc.plasticity.xyz/sketch), [Offset Planar Curve](https://doc.plasticity.xyz/sketch/offset-planar-curve), [Sketch Commands](https://doc.plasticity.xyz/all-commands/sketch-commands), [Shapr3D — constraints](https://www.shapr3d.com/product/cad-constraints), [Fusion 360 — Sketch Constraints](https://lecture.nakayasu.com/en/docs/fusion360/fusion360-sketch-constraints/)
- Vetorização: [vtracer](https://github.com/visioncortex/vtracer)
