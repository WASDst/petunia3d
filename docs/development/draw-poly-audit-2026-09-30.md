# Auditoria DRAW + POLY — usabilidade e desempenho (2026-09-30)

Escopo: reanálise dos dois workspaces de modelagem já entregues (DRAW e POLY, que
compartilham o workspace interno `Model`) buscando defeitos de funcionalidade,
gargalos de desempenho e atritos de uso. Método: leitura do bridge
(`crates/ui-slint`), do core (`petunia_core`) e dos módulos de modelagem; cruzamento
com as matrizes das Ondas 1–6 do ADR 007. **Não houve profiling com GPU/captura nativa
neste ambiente**: o que é "medido" abaixo é custo assintótico lido no código ou teste;
o que é estimativa está dito.

## 1. Desempenho

| # | Achado | Custo antes | Ação | Estado |
| --- | --- | --- | --- | --- |
| P1 | `ViewportSceneQuery::new` (triangula todos os objetos visíveis, incluindo modifiers) era reconstruída a **cada** movimento do mouse (hover), a cada teste de oclusão e a cada dab de pintura | O(triângulos da cena) por evento | Cache por chave de geometria (`scene_cache.rs`: topologia, posições, transformações, splines, procedural, visibilidade/bloqueio). Cores, texturas e seleção não invalidam | **Corrigido** |
| P2 | Pincel 3D: `face_normal` (Newell) por face **por dab**, sem descarte espacial | O(faces × dabs) por evento, com 8× na simetria total | Normal, centro e raio de cada face calculados uma vez por chamada; descarte por esfera da face antes da rasterização | **Corrigido** (sem índice espacial: acima de ~20 mil faces ainda é linear) |
| P3 | Cache de regiões/contornos do DRAW chaveado pelo `revision_clock` **inteiro**: pintar (cor/textura) o invalidava | Recalcula o arranjo planar (O(n²) por plano) a cada dab | Chave só com `spline`/`procedural` + quantidade de perfis | **Corrigido** |
| P4 | `view_model()` reconstrói centenas de `String` e `sync_window_properties` reatribui todas as propriedades a cada evento | Alocações proporcionais ao tamanho do shell por evento | Não alterado. Recomendação: *dirty flags* por seção do view-model (viewport, inspector, listas) e `SharedString` interning. Exige medição com o app aberto | **Aberto** |
| P5 | Cada callback de ponteiro renderiza o viewport na hora (sem coalescer por quadro) | Em hover com mouse de alta taxa, N renders/quadro | Não alterado (pendência da Onda 1). Recomendação: marcar "render pendente" e consumir num único `Timer` de 60 Hz | **Aberto** |
| P6 | `pick_face_hit` e picking de componentes são lineares nas faces da malha ativa | O(faces) por evento | Aceitável até ~20 mil faces. Recomendação: grade uniforme/BVH reconstruída por revisão de geometria (reaproveita a chave de P1) | **Aberto** |
| P7 | Snap com inferência percorre todos os vértices e arestas da malha ativa a cada movimento | O(V+E) por evento; orçamento do cap. 46 (≤ 16,7 ms em 50 mil triângulos) não comprovado | Não alterado. Mesma recomendação de índice espacial | **Aberto** |

## 2. Usabilidade e correção

| # | Achado | Ação | Estado |
| --- | --- | --- | --- |
| U1 | Forma desenhada sumia ao clicar fora (só aparecia no hover) | Regiões e contornos sempre visíveis no DRAW; reativar pelo contorno | **Corrigido** |
| U2 | Perfil fechado não aceitava novos pontos; não havia como curvar/endireitar um ponto isolado | Inserção na aresta (sem mudar a curva), duplo clique reto↔curva, botão age no ponto selecionado | **Corrigido** |
| U3 | `Space` abria o micro-inspector (ADR 007: "ferramenta anterior") | `Space` = ferramenta anterior; micro-inspector em `Shift+Space` | **Corrigido** |
| U4 | Sem tecla direta de workspace | `Ctrl+1..5` | **Corrigido** |
| U5 | Trocar para o PAINT mantinha a ferramenta de modelagem; escolher Fill no PAINT pintava os vértices do objeto todo | Normalização de ferramenta ao trocar de workspace; Fill só pinta vértices fora do PAINT | **Corrigido** |
| U6 | Painéis flutuantes travavam ao arrastar, não aceitavam tamanho e cortavam valores | Arrasto estável, alça de redimensionar | **Corrigido** |
| U7 | Sliders com `value: expr` perdiam o binding no primeiro arrasto | `PetuniaSlider.controlled` | **Corrigido** |
| U8 | Alças de Extrude/Inset/Round Edge abaixo de 24 px (WCAG 2.5.8, ADR 007) | — | **Aberto** (visual, com o agente de UI) |
| U9 | Um único buffer numérico: o bridge usa `modal_text`; `ToolSession::text` está pronto e sem uso | — | **Aberto** (migração do bridge; baixo risco, alto toque) |
| U10 | Loop Cut, Slice, Knife e Profile ainda têm ciclo próprio (não passam por `ToolSession`) | — | **Aberto** (Ondas 4/5 do ADR 007; escopo grande) |
| U11 | Poly Pen: sem "clique na aresta para subdividir" | `Mesh::split_edge` + `poly_pen_split_edge` (1 Undo) + clique na aresta sem polígono em coleta | **Corrigido** |
| U12 | Snap só considera a malha ativa; sem snap por normal | — | **Aberto** |
| U13 | Contorno de seleção sem teste de profundidade; ausente em Wireframe e no viewport por software | — | **Aberto** (renderer) |
| U14 | Entrada do DRAW: conjunto de seleção Shape/Curve/Point/Region, plano por 3 pontos e "Converter em polígonos" | — | **Aberto** (dependem das formas paramétricas, cap. 02) |

## 3. Ordem recomendada do que ficou aberto

1. **Índice espacial por revisão de geometria** (P6, P7, e o pincel 3D): uma grade uniforme
   reaproveitando a chave de P1 resolve três itens de uma vez e é o que sustenta o orçamento de 50 mil triângulos.
2. **Coalescer render por quadro** (P5) e **dirty flags do view-model** (P4): medir primeiro com o app aberto
   (`puffin` já está nas funções do renderer).
3. **U9 → U10**: um buffer numérico e depois a migração de Loop Cut/Slice/Knife/Profile para `ToolSession`;
   é o que de fato cumpre o critério "nenhuma ferramenta tem ciclo próprio" do ADR 007.
4. **U8** com o agente de UI (alças ≥ 24 px) e **U13** com o renderer.

## Execução de 04/10/2026

Os itens 1–4 desta ordem (índice espacial, render coalescido e dirty flags, buffer numérico único e migração para `ToolSession`, alças) foram tratados na rodada registrada em [`draw-poly-perf-tools-2026-10-04.md`](draw-poly-perf-tools-2026-10-04.md); alças ≥ 24 px e gizmo em passo GPU continuam pendentes.
