# P3D-172 — Reference Image Sequence (Reference Frames)

<aside>
🎞️

Estado: **SPEC DRAFT (2026-09-30)** · Prioridade: P2 · Era 3. Decisão D3 do [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md): a referência de animação é **batch de imagens**. Vídeo, tracking, estimativa de pose e mocap estão **fora de escopo**. O foco desta página é a **UX**.

</aside>

## Objetivo

Deixar o artista importar uma pasta (ou várias) de imagens como **quadros de referência** ligados à timeline, compará-los com a pose atual e marcar quais são as poses-chave, sem nenhum conhecimento técnico de animação.

## Modelo

`ReferenceSequence` estende o `ReferenceSet`/`ReferenceImage` de [P3D-013](p3d-013-imagens-de-referencia.md):

| Campo | Descrição |
| --- | --- |
| `frames` | lista ordenada de imagens (recurso do projeto com ID estável, embutida ou vinculada) |
| `mapping` | tempo → imagem: *N frames do projeto por imagem*, *ajustar ao tamanho do clipe*, ou taxa própria; `hold` e `loop` |
| `offset` | deslocamento no tempo |
| `view` | vista associada (Front/Back/Left/Right/Top/Bottom/livre), reutilizando os slots do P3D-013 |
| `transform` | posição, escala, rotação e espelhamento no viewport |
| `opacity`, `lock`, `xray` | mesmas propriedades do P3D-013 |
| `key_marks` | frames marcados como **key pose** |

Dados de projeto com IDs estáveis; o renderer recebe descritores neutros e cacheia texturas. Seletor de arquivos e layout ficam no frontend.

## Importação (UX)

1. **Add Reference Frames…** aceita **arrastar uma pasta**, **arrastar vários arquivos** ou o seletor de arquivos.
2. **Ordenação natural** (`frame2` antes de `frame10`; zeros à esquerda; prefixos comuns). A ordem pode ser corrigida por arrastar na filmstrip ou por teclado.
3. **Verificações com mensagem clara, nunca silenciosas:** lacunas na numeração, duplicatas, tamanhos diferentes (oferece normalizar) e formato não suportado (lista quais foram ignorados).
4. **Mapeamento em uma pergunta:** "Quantos frames do projeto por imagem?" com atalhos *1 por imagem*, *ajustar ao clipe atual* e *personalizado*; a prévia mostra o resultado.
5. **Trabalho em segundo plano:** miniaturas e decodificação em jobs que operam sobre snapshot (cap. 13/19). A UI não trava; há progresso e cancelamento.
6. **Vídeo:** não é importado. A ajuda contextual explica como extrair frames com uma ferramenta externa e, então, importar a pasta.

## Timeline e viewport

- **Filmstrip** sincronizada acima da timeline: miniaturas na mesma escala do tempo, frame atual destacado, marcas de key pose e arraste para reordenar.
- **Marcar key poses:** com o frame selecionado, um comando marca/desmarca a **key pose**; a timeline ganha *pose markers* e é possível pular de uma marca à próxima. Nada é criado no rig automaticamente.
- **Comparação:** opacidade, **hold-to-compare** (segurar para ver só a referência), modo *x-ray*/overlay e **Match View** (alinha a câmera ortográfica à vista da sequência).
- **Reference Monitor:** cartão flutuante *in-canvas* (mesmo padrão de [ADR 005](../../architecture/adr/005-modulos-inspector-dock-float-pin.md)) com o frame corrente, para quem prefere não sobrepor a malha. **Sem multi-viewport** (cap. 36).
- **Reference ghost:** o frame pode entrar como camada de comparação junto dos Ghosts ([P3D-171](p3d-171-animation-ghosts-trajectories.md)).

## Orçamento e robustez (PCs modestos, P3D-126)

Decodificação preguiçosa com cache LRU e **dimensão máxima de textura configurável** (padrão sugerido: 1024 px); só os frames vizinhos ao atual ficam residentes. Arquivo ausente → estado *missing* com **relink** (como P3D-003/P3D-139), nunca crash. Limites de contagem e de memória com erro legível.

## Acessibilidade

Reordenar, marcar key pose, mudar mapeamento e navegar pela filmstrip funcionam por teclado; miniaturas têm rótulo acessível (nome e número do frame); nada depende só de cor.

## Comandos (`CommandId`, Undo/Redo, Command Palette)

`reference.sequence.import`, `.reorder`, `.remove_frames`, `.set_mapping`, `.set_view`, `.toggle_key_mark`, `.match_view`, `.relink`. Todos transacionais; texto por `TextId`, ícones por `IconId`.

## Candidato posterior (não faz parte desta spec)

**Pose Pins:** pinos 2D sobre o frame de referência que viram alvos de IK no plano da vista, para completar a pose. É assistência **sem ML**, mas amplia o escopo; só entra após F4 e decisão explícita.

## Não objetivos

Decodificar vídeo, tracking, estimativa de pose, mocap, rotoscopia automática, edição de imagem.

## Testes / DoD

1. Ordenação natural: `frame2 < frame10`, zeros à esquerda, prefixos mistos, empates estáveis.
2. Detecção de lacunas, duplicatas e tamanhos divergentes com mensagens corretas.
3. Matemática de mapeamento: razão de fps, hold, loop, offset e "ajustar ao clipe" (limites e arredondamento).
4. Round-trip do `.petunia`; arquivo ausente → *missing* + relink.
5. Batch de **500 imagens** carrega sem bloquear a UI; a memória respeita o orçamento (evicção LRU verificada).
6. Latência de scrub dentro do orçamento medido em PC modesto.
7. Undo/Redo de todos os comandos por hash.
8. Fluxo por teclado completo e rótulos acessíveis.
9. Usabilidade: importar uma pasta, mapear, marcar 3 key poses e comparar com a pose, **sem documentação** (teste com artistas antes de alegar acessibilidade).

## Dependências

P3D-013, P3D-014, P3D-003/P3D-139 (relink), P3D-066/P3D-067 (timeline), P3D-171, cap. 13/19 (jobs), P3D-126.
