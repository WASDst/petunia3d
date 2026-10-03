# PAINT/DRAW — reconciliação do MVP (02/10/2026)

Escopo autorizado: corrigir booleanos; Shape Builder poligonal (curvas fiéis em V1); SVG decal/perfil, Projection/Stencil e Path Paint no MVP; completar adicionais aprovados. Preservar aparência da UI. Gates finais aguardam instrução do responsável.

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

Evidência da inspeção: HEAD inicial 4a41951; SVG 0e1c83e e Path Paint 8645a8f em worktrees separados; Projection sem commit. Nenhum gate executado nesta retomada.

## Superfície de uso

As ações novas estão na paleta de comandos e podem receber atalhos no perfil do usuário. DRAW expõe parâmetros numéricos nos controles existentes. PAINT usa a layer de decal selecionada como fonte de Projection, Stencil e stamps. Enter aplica Path/Projection; Posicionar stencil usa a mesma manipulação e Enter retorna ao brush; Esc cancela o gesto antes de remover o stencil. Nenhuma reformulação visual foi incluída.

## Limites explícitos do MVP

- Shape Builder reconstrói polígonos; curvas fiéis ficam para V1. Furos são loops do perfil composto: contorno, seleção, persistência e extrusão funcionam; edição de nós individuais desses loops fica para V1. Revolve/Sweep recusam perfis compostos, sem preencher furos silenciosamente.
- SVG DRAW importa contornos Bézier, sem interpretar estilo/fill-rule como sólidos compostos. Clipping/filtros são respeitados na rasterização de decal, não na extração de splines. Fonte SVG e furos persistem em ZIP/JSON; o escritor postcard legado conserva seu layout antigo, não guarda a fonte SVG e recusa perfis com furos para não perder geometria.
- Projection é temporária em espaço de tela e commita raster; Stencil modula o brush pelo alpha ou luminância. Decal live mantém transformação UV existente. Attachments de Path invalidam se a topologia mudar; não há reprojeção silenciosa.
- Path é poligonal no shell. Ribbon usa ponta quadrada contínua, stamps seguem a tangente local. Não há editor live de repintura automática do caminho após commit. Alpha lock bloqueia áreas transparentes da layer alvo; projeções e stamps em layer nova exigem alpha lock desligado.
- Booleano preserva a geometria e pode manter triângulos onde quads não são seguros. Transferência raster limitada a 1024 px; a pilha é consolidada somente quando muda o atlas. Canais são amostrados das superfícies de origem; parâmetros de shader permanecem os de A. Não se cria retopologia ou sistema novo de materiais por face.
- Trace é silhouette bounded (alpha ou luminância), sem tracing de cor/curvas. Mirror é de criação, não constraint live. Simplify/Resample são explícitos e podem mudar a curva.

## Evidência de validação

Compilação focalizada de `petunia_ui_slint --lib` realizada para integração; novas regressões escritas para Undo/rollback, furos, textura booleana, densidade e persistência. Execução de testes, gates completos, captura nativa e aceite manual **aguardam instrução do responsável**. Compilação não substitui esses critérios. Ver [roteiro manual](paint-draw-mvp-manual-2026-10-02.md).

Path também coleta anchors por clique/arrasto no canvas UV, com preview UV sem ligar segmentos entre charts diferentes; o commit usa o mesmo caminho/restrições/Undo do modelo 3D. A interação Projection continua na viewport; clicks do canvas nesse modo não pintam um brush por engano.
