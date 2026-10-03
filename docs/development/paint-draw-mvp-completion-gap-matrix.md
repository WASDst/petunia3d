# PAINT/DRAW — reconciliação do MVP (02/10/2026)

Escopo autorizado: corrigir booleanos; Shape Builder poligonal (curvas fiéis em V1); SVG decal/perfil, Projection/Stencil e Path Paint no MVP; completar adicionais aprovados. Preservar aparência da UI. Gates da integração original estavam pendentes; a retomada de 03/10 está autorizada a validar a conclusão.

| Requisito | Estado observado antes da retomada | Delta implementado (inspeção; aceite pendente) |
| --- | --- | --- |
| Boolean cleanup | PARTIALLY_COMPLIANT | UVs e cores seguem a triangulação real, inclusive quads não afins; preservar albedo e canais PBR no atlas novo, reorientando normais; material independente; manter paint stack quando UV herdado |
| Shape Builder / Pathfinder | BROKEN | Furos persistentes, Delete/XOR sem recriar vazios, Merge recorta divisórias abertas; formas não afetadas mantêm identidade; extrusão composta |
| SVG decal/perfil | PARTIALLY_COMPLIANT (worktree isolado) | Recuperado parser limitado e persistência compatível; importação DRAW/decal Slint; rasterização 512/1024 na paleta |
| Projection/Stencil | RUDIMENTARY (rascunho isolado) | Preview temporário, mover/rotacionar/escalar/espelhar; depth da cena; alpha/luma stencil; commit restrito em layer própria e 1 Undo |
| Path Paint | PARTIALLY_COMPLIANT (núcleo isolado) | ToolSession, preview, anchors persistentes, Stroke/Ribbon/Repeated Stamp e 1 Undo; invalidação explícita |
| Alpha lock; linha; pixel perfect; rampas/dithering | MISSING | Motor, linhas Shift 2D/3D, pixel perfect incremental/simétrico e ações localizadas na paleta/keymap |
| UV health; export bleed | PARTIALLY_COMPLIANT | Densidade linear corrigida, ilhas pequenas/overlap/bleed; export GLB com padding 0–64 em cópia |
| Round corners; mirror; presets; dimensões; simplify; trace | MISSING | Round no ponto ou perfil; mirror de criação; polygon/rounded rectangle/ellipse/arc/slot; comprimento/ângulo; simplify/resample; trace com furos; 1 Undo por lote |
| Documentação MVP | OBSOLETE | P3D-133/158/165, cap. 44, matrizes, feature docs, changelog e estado reconciliados |

Evidência da inspeção: HEAD inicial 4a41951; SVG 0e1c83e e Path Paint 8645a8f em worktrees separados; Projection sem commit. Nenhum gate executado na retomada original de 02/10.

## Superfície de uso

As ações novas estão na paleta de comandos e podem receber atalhos no perfil do usuário. DRAW expõe parâmetros numéricos nos controles existentes. PAINT usa a layer de decal selecionada como fonte de Projection, Stencil e stamps. Enter aplica Path/Projection; Posicionar stencil usa a mesma manipulação e Enter retorna ao brush; Esc cancela o gesto antes de remover o stencil. Nenhuma reformulação visual foi incluída.

## Limites explícitos do MVP

- Shape Builder reconstrói polígonos; curvas fiéis ficam para V1. Furos são loops do perfil composto: contorno, seleção, persistência e extrusão funcionam; edição de nós individuais desses loops fica para V1. Revolve/Sweep recusam perfis compostos, sem preencher furos silenciosamente.
- SVG DRAW importa contornos Bézier, sem interpretar estilo/fill-rule como sólidos compostos. Clipping/filtros são respeitados na rasterização de decal, não na extração de splines. Fonte SVG e furos persistem em ZIP/JSON; o escritor postcard legado conserva seu layout antigo, não guarda a fonte SVG e recusa perfis com furos para não perder geometria.
- Projection é temporária em espaço de tela e commita raster; Stencil modula o brush pelo alpha ou luminância. Decal live mantém transformação UV existente. Attachments de Path invalidam se a topologia mudar; não há reprojeção silenciosa.
- Path é poligonal no shell. Ribbon usa ponta quadrada contínua, stamps seguem a tangente local. Não há editor live de repintura automática do caminho após commit. Alpha lock bloqueia áreas transparentes da layer alvo; projeções e stamps em layer nova exigem alpha lock desligado.
- Booleano preserva a geometria e pode manter triângulos onde quads não são seguros. Transferência raster limitada a 1024 px; a pilha é consolidada somente quando muda o atlas. Canais são amostrados das superfícies de origem; parâmetros de shader permanecem os de A. Não se cria retopologia ou sistema novo de materiais por face.
- Rampas seguem a ordem da paleta definida pelo usuário, sem ordenar cores silenciosamente por luminância.
- Trace é silhouette bounded (alpha ou luminância), sem tracing de cor/curvas. Mirror é de criação, não constraint live. Simplify/Resample são explícitos e podem mudar a curva.

## Evidência de validação

Compilação dos executáveis unitários das seis crates abaixo concluída com **exit 0** em checkout isolado. Código publicado no commit `e5e1452dfa734fbaa1c21a7d464e64e738134fdf`; checkout de conferência no commit `4a0af068f984e7dd6359ec7e75bbc6bda383b0bc`, com árvore idêntica `60cc841eecc963ba5e3af4729ddc85b04044be11`. A repetição após corrigir a dependência JSON do teste Slint e coletar os triângulos na regressão de Projection terminou em 3m41s.

```sh
cargo test --no-run --lib -j 1 \
  -p petunia_mesh -p petunia_project -p petunia_core \
  -p petunia_module_model -p petunia_module_paint -p petunia_ui_slint
```

Usado cache Cargo separado via `CARGO_TARGET_DIR`. Resultado: seis executáveis unitários gerados, **zero casos de teste executados nesta conferência**. Novas regressões cobrem Undo/rollback, furos, máscaras/alpha, anchors, textura booleana, densidade e persistência. A execução de testes e gates foi autorizada na retomada de 03/10; os resultados atuais são registrados na matriz DRAW. Captura nativa e aceite manual permanecem pendentes; compilação não substitui esses critérios. Ver [roteiro manual](paint-draw-mvp-manual-2026-10-02.md).

Catálogos e changelog gerados com `cargo run -p xtask -- docs-generate` (exit 0), sem compilar o site público congelado. Entrega no [PR #26](https://github.com/wasd-lat/petunia3d/pull/26), integrado à main; a integração não substitui o aceite. Os commits seguintes a esse registro alteram somente documentação.

Path também coleta anchors por clique/arrasto no canvas UV, com preview UV sem ligar segmentos entre charts diferentes; o commit usa o mesmo caminho/restrições/Undo do modelo 3D. A interação Projection continua na viewport; clicks do canvas nesse modo não pintam um brush por engano.

Export bleed restringe a dilatação aos gutters fora das faces UV, preservando pixels/alpha da arte dentro das ilhas. Materiais compartilhados usam a máscara combinada dos consumidores exportados; todos os canais existentes recebem padding em cópia. Verificar também transparências internas de decals SVG.

## Retomada de 03/10/2026

As correções de seleção DRAW/PAINT, seleção visível por caixa/laço, trilhos e
reedição dimensional de retângulos/círculos estão na atualização da
[matriz DRAW](draw-regions-gap-matrix.md). Essa rodada executa os gates do
checkout atual; a compilação isolada histórica acima permanece identificada
como evidência anterior. Captura e aceite manual seguem no roteiro.
