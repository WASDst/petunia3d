# PAINT/DRAW MVP — roteiro de verificação manual (02/10/2026)

Estado: **roteiro, não evidência de execução**. Gates finais aguardam instrução do responsável. [Matriz de implementação/limites](paint-draw-mvp-completion-gap-matrix.md).

## Preparação

Abrir projeto de teste em cópia, criar cubo e plano com UV charts distintos, 256 px e duas layers raster. Na paleta de comandos buscar os rótulos abaixo; ações também podem receber atalhos em Configurações. Salvar/abrir `.petunia` usa ZIP/JSON.

## Booleanos

1. Pintar A vermelho e B azul; Cut de B cruzando A, com cleanup ligado. Verificar paredes/quads e caps seguros, sem exigir só quads num anel de contagens diferentes. Não deve surgir triângulo cruzando o furo nem degenerado.
2. Conferir que áreas de A/B conservam suas cores, normals/roughness/metallic quando presentes, que a pintura pode continuar sobre uma face e não vaza para as outras. Undo restaura ambos os objetos/texturas; Keep Parts mantém B.
3. Repetir com B inteiramente externo e A pintado: UV herdado deve manter a paint stack de A; outro objeto que compartilha seu material não deve mudar.
4. Repetir com cleanup desligado e sem textura: atlas válido ainda permite pintar faces separadas.

## Shape Builder e perfis compostos

1. Dois polígonos sobrepostos: Merge por clique/arrasto; desfazer/refazer deve ser um passo por operação. Exercitar Unite/Subtract/Intersect/Exclude na paleta.
2. Contorno externo + divisória fechada: apagar só a área interna. Ela deve continuar vazia ao mover câmera, salvar/abrir e extrudar.
3. Divisória aberta atravessando forma: Merge das duas células, a divisória interna deve desaparecer; trechos externos sobrevivem.
4. Extrudar perfil com dois furos; olhar pelos furos dos dois lados. Revolve/Sweep devem recusar o perfil composto, sem gerar sólido preenchendo os furos.
5. Clicar no contorno de um furo reativa a forma; handles de nós de furos não fazem parte deste MVP.

## SVG

1. PAINT: importar SVG com alpha, curva e clip; imagem proporcional, fonte/cache persistentes; mover/escalar/rotacionar decal existente. Re-rasterizar 512/1024 pela paleta mantém posição/escala.
2. DRAW: importar SVG com vários subpaths; conferir alças Bézier, workplane atual e um Undo para o lote. A importação é de contornos: fill-rule e clipping não se tornam booleanos de perfis automaticamente.
3. Arquivo inválido, maior que 2 MiB, ENTITY, árvore de referências exponencial: recusa sem alterar documento. `<text>`/`<image>` não são importados; salvar e abrir confirma que source/cache permanecem.

## Projection/Stencil

1. Selecionar decal fonte e executar Projetar decalque. Mover por clique/arrasto, rotacionar com o modificador alternativo, escalar com modificador de extensão; paleta também espelha/rotaciona/escala. Enter cria layer raster; Esc só cancela preview. Fonte permanece.
2. Colocar um objeto entre câmera e alvo: projeção só pinta regiões visíveis. Seleção/FirstFace/SelectedFaces devem limitar o resultado; área inteiramente bloqueada não deixa layer ou Undo vazios.
3. Stencil cria layer raster e retorna ao brush; pintar segue alpha da imagem. Alternar luminância e usar Posicionar stencil para mover/rotacionar/escalar; Enter volta ao brush. Navegar a câmera, alternar 2D/3D e verificar a projeção coerente.
4. Durante stroke, Esc cancela o stroke antes de remover stencil. Trocar workspace/objeto não transporta o draft para outro alvo. Alpha lock em uma layer nova transparente impede pintar; desligá-lo permite Projection/Stamps.

## Path Paint

1. Stroke: marcar/arrastar pontos sobre a superfície, conferir preview; Enter pinta e salva spline com anchors; Undo remove ambos em um passo. Esc descarta draft sem pintar.
2. Ribbon: verificar faixa contínua e que brush original não foi alterado. Repeated Stamp usa o decal selecionado com tangente local em layer própria; fonte não muda.
3. Restringir seleção/face; testar seleção vazia, layer/objeto locked e mudança de topologia antes do commit. Erro não deve deixar metade do resultado. Salvar/abrir confirma anchors; editar a spline depois não repinta automaticamente o raster.

## Shortlist adicional

1. Alpha lock: pintar/erase/smudge numa layer com metade transparente; preservar alpha e área vazia. Desligado, permitir expansão. Dithering e palette lighter/darker seguem a paleta, sem exceder restrições.
2. Pixel perfect: escada 1 px em canvas, inclusive passando entre tiles e com simetria X/Y; cantos removidos devem sumir também da textura composta. Shift+clique liga pontos em 2D/3D com um Undo; trocar objeto não reutiliza endpoint anterior.
3. UV health: densidade dobra ao dobrar resolução e cai pela metade ao dobrar escala da malha; equalizar usa medida linear. Conferir ilhas pequenas, overlap e bleed. Exportar GLB com padding 0 e 2; original e histórico não mudam.
4. DRAW: presets, parâmetros radius/length/angle/sides/tolerance; mirror cria cópia em um Undo; constraints afetam o próximo segmento. Round de um ponto e de perfil inteiro; Simplify/Resample explícitos. Trace usa referência visível (alpha ou luminância) e preserva furos.

## Gates reservados

Após autorização: fmt/check, testes relevantes (SVG, profile_tools, format, boolean_texture, shape_builder, draw_batch, projection, surface_commands, path_paint, uv_tools), clippy, architecture/docs/bible/UI guards. Registrar comando, SHA, resultado e diferenças manuais encontradas. Não usar logs de worktrees antigos como prova deste checkout.

Export bleed restringe a dilatação aos gutters fora das faces UV, preservando pixels/alpha da arte dentro das ilhas. Materiais compartilhados usam a máscara combinada dos consumidores exportados; todos os canais existentes recebem padding em cópia. Verificar também transparências internas de decals SVG.
