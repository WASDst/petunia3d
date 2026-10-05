# DRAW/POLY — desempenho e ferramentas (04/10/2026)

Implementation-vs-Spec Gap Matrix da rodada que executou os itens 1–9 da
análise de 04/10/2026 (branch `claude/draw-poly-perf-tools`). Autoridade:
[ADR 007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md),
[ADR 008](../architecture/adr/008-transformacao-rigida-e-undo-compartilhado.md),
capítulos [01](../bible/foundations/01-visao-ux-referencias.md),
[02](../bible/foundations/02-workflow-modelagem-shape-first.md) e
[46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md),
constituição 11.

Os testes foram executados ao fim do lote, por instrução do responsável do
produto; o resultado está na seção "Validação". Captura nativa e teste com
usuários continuam pendentes.

## Desempenho

| # | Requisito | Antes | Entrega |
|---|---|---|---|
| 1 | Evento de arrasto/hover barato | `BROKEN`: view model inteiro, ~550 propriedades e listas recriadas por evento; hover do Draw Profile sempre redesenhava | `refresh.rs`: viewport e overlays por evento, janela completa no máximo a cada 50 ms com passada final garantida; listas só trocam se o conteúdo mudou; hover do Draw Profile só redesenha quando algo mudou |
| 2 | GPU só refaz o que mudou | `BROKEN`: seleção ou qualquer edição retriangulava todos os objetos | Seleção fora da impressão digital da malha (GL a mantém nas arestas); WGPU com geometria por objeto em cache por chave de desenho |
| 3 | Medição | `MISSING` na UI | Escopos `puffin` (`PETUNIA_PROFILE=1`), tempos p50/p95/máx por etapa (`PETUNIA_FRAME_TIMING=1`), benchmarks `criterion` de cena, oclusão, caixa e snap em ~50 mil triângulos (`cargo bench -p petunia_core --bench viewport_queries`) |
| 4 | Índice espacial | `MISSING`: picking, caixa/laço e snap lineares (caixa O(V×T)) | `core::bvh::TriangleBvh` na cena de consultas, no picking de componentes (cena em cache, sem triangular por evento) e no snap (`SnapAccel` por gesto/revisão) |
| 5 | Prévia incremental | `RUDIMENTARY`: clone + operação inteira por evento | Move/Rotate/Scale/Push escrevem só vértices afetados; Extrude monta a topologia uma vez; Inset/Bevel/Extrude Individual interpolam após verificação de linearidade e são recalculados exatos no commit; gesto de perfil não clona o projeto por evento |
| 9 | Matriz por objeto e Undo | `MISSING` | Matriz rígida em tempo de execução no modo objeto + `Shared<T>` (cópia na escrita) em malhas e texturas; cache da malha avaliada com modifiers (ADR 008) |

## Ferramentas

| # | Requisito | Antes | Entrega |
|---|---|---|---|
| 6 | Gramática única (ADR 007) | `PARTIALLY_COMPLIANT`: Knife, Loop Cut, Slice, Draw Profile e modais por teclado com ciclo próprio; dois buffers numéricos | Todos pelo `ToolSession`; modais por teclado como sessão travada; buffer numérico único no `ToolSession` |
| 7 | Domínios do DRAW | `FUNCTIONAL_BUT_DIFFERENT`: Curve/Point/Region eram arestas/pontos/faces de malha | Curve = segmento do perfil (Move move os dois nós), Point = nó, Region = região (Push/Pull nela) |
| 7 | Mensagens com `TextId` | `BROKEN`: ~200 `set_status` fixos, EN e PT misturados | 176 `TextId` em `[status]` (en/pt-BR) com marcadores nomeados |
| 8 | Knife | `PARTIALLY_COMPLIANT`: uma face por segmento | Atravessa as faces entre dois cliques pelo plano de visão |
| 8 | Imprint | `PARTIALLY_COMPLIANT`: só dentro de uma face, sem furos | Cruzando arestas, várias faces coplanares e com furos, sem junções em T |
| 8 | Snap (cap. 01) | `PARTIALLY_COMPLIANT`: desligado, só malha ativa | Ligado por padrão (arredondamento rígido só com Ctrl), todos os objetos visíveis, centro de face, interseção, paralelo, perpendicular, passos de 15° no DRAW |
| 8 | Inset métrico | `MISSING` | Distância em mundo com limite anti-interseção |
| 8 | Plano de trabalho | `PARTIALLY_COMPLIANT` (face + aresta já existia) | Plano médio entre duas faces paralelas |
| 8 | Oclusão de região | `PARTIALLY_COMPLIANT`: só malha ativa | Qualquer objeto visível (BVH da cena) |
| 8 | Pontos por perfil | 512 | 4096 |

## Validação (04/10/2026, ao fim do lote)

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D warnings`: limpos.
- `cargo test --workspace`: 1847 testes aprovados, 0 falhas (a última falha,
  `petunia_app` `key_extrude_starts_preview_without_mutating`, era intermitente
  por roteamento aleatório de atalhos no legado e foi corrigida; 3 execuções
  seguidas verdes). `petunia_ui_slint` com `animation-workspace` (lib,
  `animate_shell`, `viewport_gestures`): aprovado.
- `xtask docs-check` (inclui `bible-check` e `ui-check`), `ui-guard --strict`,
  `arch-check`, `docs-generate --check`: aprovados.
- Testes atualizados por mudança de comportamento pedida: snap ligado por
  padrão (pré-condição "sem snap" explícita onde o teste mede geometria exata),
  Inset métrico, rótulo Round Edge (cap. 13).
- Correções encontradas pelos testes: Move com eixo usava a translação
  "encaixada" sem alvo (valor digitado ignorado); arredondamento na grade por
  padrão no Slint e no legado egui; avisos de clippy.

## Pendências conhecidas

- ~~Textos do HUD fora das mensagens de status~~: resolvido em 04/10/2026
  (42 `TextId` em `[hud]`).
- ~~Orçamento de bytes do histórico somando snapshots inteiros~~: resolvido em
  04/10/2026 (bytes únicos por bloco compartilhado, ADR 008).
- Matriz por objeto persistida no arquivo: fora da V1 (ADR 008).
- Poly Pen (modos Points/Edges/Polygons, pintar faces arrastando), alças
  ≥ 24 px e gizmo em passo GPU, matcap/GTAO: seguem nas matrizes anteriores.
- Captura nativa e teste com usuários de todas as ferramentas.
