# Changelog

Todas as alterações notáveis deste projeto são documentadas neste arquivo.
O formato baseia-se no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) e adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Unreleased] — Frontend Declarativo Slint & Modern UI

### DRAW/POLY — desempenho e ferramentas (04/10/2026)

- Arrasto e hover redesenham só a viewport e os overlays; a janela completa sincroniza no máximo a cada 50 ms. Seleção não retriangula a cena; o WGPU guarda a geometria por objeto.
- BVH de triângulos no picking, na seleção por caixa/laço, na oclusão de regiões e no snap; prévia incremental de Move/Rotate/Scale/Push, Extrude, Inset e Bevel; matriz rígida por objeto no modo objeto; Undo com malhas e texturas compartilhadas (ADR 008).
- Gramática única para Knife, Loop Cut, Slice, Draw Profile e os modais por teclado; buffer numérico único.
- DRAW: domínios Curve/Point/Region reais; Knife atravessa várias faces; imprint em várias faces, cruzando arestas e com furos; Inset métrico; plano médio entre faces paralelas; snap ligado por padrão entre todos os objetos, com centro de face, interseção, paralelo, perpendicular e 15°.
- 176 mensagens de status passam a usar `TextId` (en/pt-BR). Medição opcional por `PETUNIA_PROFILE`/`PETUNIA_FRAME_TIMING` e benchmarks de consultas da viewport. Matriz: `docs/development/draw-poly-perf-tools-2026-10-04.md`.

### CI, gates e tokens de cor (04/10/2026)

- `ui-guard.yml` voltou a ser YAML válido (o comando `gallery::` era lido como chave e o workflow nunca rodava).
- `deny.toml`: aceita `GPL-3.0-only`/`GPL-3.0-or-later` (licença do projeto e opção GPL do Slint) e registra `RUSTSEC-2026-0206` (`rustybuzz`, transitivo via `usvg`/`resvg`).
- Workflow UI Slint sem debuginfo, sem incremental e com 2 jobs, para não esgotar a memória do runner.
- `cargo clippy --workspace --all-targets -D warnings` sem erros em `petunia_mesh`.
- Shell Slint: 90 cores fixas viram `DesignTokens`; sombras e fundos de HUD seguem o tema (o tema claro deixa de herdar HUD escuro) e overlays do viewport usam tokens `overlay-*` com os mesmos valores.
- `PROJECT_STATE.md` e `AGENTS.md` alinhados ao estado real (próximas ações, gaps do Slint, contagem de testes).

### Repositório — remoção do Prumo (04/10/2026)

- Removida a camada de orquestração Prumo: `.agents/`, `.ai/`, `.prumo/`, `prumo.json`, `ENTRYPOINT.md`, `GEMINI.md`, `docs/contracts/` e scripts do próprio Prumo em `scripts/`. Código, Livro Vivo e `docs/` preservados.
- Evidências da rodada premium movidas para `docs/audits/premium-evidence/`; roteador renomeado para `docs/NAVIGATION.md` com links corrigidos.
- `AGENTS.md`, `PROJECT_STATE.md`, guias de contribuição, testes e deploy deixam de exigir `prumo doctor`/`prumo validate`; gates passam a ser os do `xtask`. Caps. 13, 31 e 34 do Livro Vivo citam agentes/CI em vez do Prumo.
- Corrigido drift entre `CHANGELOG.md` e `docs/changelog/index.md` causado por link relativo.

### DRAW/POLY/PAINT — seleção e perfis reeditáveis (03/10/2026)

- Perfis 2D selecionáveis pelo contorno/interior; mover, girar e escalar com cancelamento e um Undo por gesto.
- Retângulos e círculos novos conservam identificação paramétrica em ZIP/JSON e permitem reeditar dimensões/segmentos após selecionar a forma. Edição livre que descaracteriza a primitiva desativa a regeneração dimensional; formatos legados recusam perda de metadados.
- Seleção por caixa/laço considera oclusão e área projetada sem seleção através; PAINT oferece seleção de objeto/parte inteira e faces.
- Ferramentas de edição poligonal removidas do trilho esquerdo duplicado; teste de integração atualizado para cores por parte.
- Falhas de comandos restauram projeto e seleção sem recalcular a seleção nem incrementar sua revisão; regressões de Projection/Path mantêm rollback exato e histórico vazio.
- SVG: parse, rasterização e destruição da árvore usam worker com pilha definida, preservando o limite de profundidade e evitando estouro em SVG válido aninhado. Fixtures GPU serializam ciclos nativos de dispositivo para permitir execução paralela do restante da suíte.

### PAINT/DRAW — integração MVP (02/10/2026)

- SVG como decal com fonte/cache, re-rasterização 512/1024, parser com limites de expansão e importação de contornos DRAW em um Undo. Compatibilidade dos layouts postcard antigos; novos dados persistem em ZIP/JSON.
- Projection/Stencil: preview temporário, move/rotate/scale/mirror, alpha/luma, oclusão da cena, restrições de pintura, layer raster própria e Undo. Posicionar stencil reutiliza a mesma gramática; Enter retorna ao brush.
- Path Paint integrado ao Slint: Stroke, Ribbon e Repeated Stamp com tangente local, anchors de superfície persistentes, validação em lote e rollback de anchors inválidos/áreas bloqueadas.
- Alpha lock, linha Shift no canvas/modelo, pixel perfect incremental e simétrico, palette ramp e dithering; densidade de texel linear corrigida, diagnóstico de ilhas pequenas e export GLB com bleed configurável em cópia.
- DRAW: round numérico de canto/perfil, mirror de criação, presets poligonais (polígono regular, rounded rectangle, ellipse, arc por três pontos, slot), comprimento/ângulo, simplify/resample e trace de silhouette com furos; comandos em lote mantêm um Undo.
- Shape Builder: Merge não deixa divisórias reabrirem a região; Delete/XOR/Cut persistem furos; contornos dos furos ficam acessíveis na viewport; extrusão suporta múltiplos furos. Revolve/Sweep recusam esses perfis explicitamente. Curvas fiéis e handles de furos ficam para V1.
- Booleanos: cores interpoladas em pontos novos, preservação de paint stack quando UV é herdado e transferência de albedo/PBR ao mudar atlas, incluindo reorientação de mapas de normais. O material resultante não altera ativos que compartilham o original. Sem remesh; triângulos permanecem onde não há cleanup seguro.
- Novas regressões para persistência, Undo/rollback, furos, textura, normais e densidade. Compilação focalizada para integração; testes/gates finais e aceite manual pendentes por instrução do responsável. Sem redesign da UI; somente controles funcionais existentes.

### Shape Builder: preenchimento e furos persistentes (02/10/2026)
- Delete de área cercada, Cut com furo e XOR preservam vazios após recalcular regiões; furos pertencem à forma composta no documento.
- Formas que sobrevivem inteiras mantêm identidade, nome e curvas. Testes de XOR exigem a área correta e perfis JSON antigos abrem sem furos.
- Checkpoint por inspeção; gates aguardam liberação do responsável.


### DRAW: domínios específicos, Depth Handle, Workplane por 3 pontos e Imprint com corte booleano (01/10/2026)
- **Domínios de seleção específicos do DRAW (`Shape / Curve / Point / Region`)**: na barra de domínios, quando em modo DRAW, a seleção expõe os termos e ícones canônicos do Cap. 11/23 (Shape = 1, Curve = 2, Point = 3, Region = 4), preservando os domínios de POLY (Object, Point, Edge, Face).
- **Depth Handle visual e interativa**: ao fechar uma forma no DRAW, projeta no centroide da região uma alça 3D (haste, diamante no topo e cota métrica) com detecção direta de clique/arraste, permitindo puxar a extrusão diretamente com o cursor no modo Select ou Draw.
- **Workplane customizado por 3 pontos e seleção**: `ProfileWorkplane::from_three_points` e `from_face_and_edge`, permitindo criar planos de trabalho alinhados passando por 3 vértices selecionados ou por uma face com aresta guia, com item correspondente no menu de contexto do viewport.
- **Imprint flexível com recorte booleano**: `imprint_boolean_cut` via `geo::BooleanOps` permite que regiões que cruzam bordas da face hospedeira fatiem a geometria com precisão, eliminando o erro de face externa quando o perfil transborda.
- **Composição de múltiplos shapes**: formas 2D fechadas sucessivas criam novos perfis para composição via Shape Builder sem sobrescrita indesejada.

### Persistência de transform, isolamento de trilhos DRAW/POLY e formas 2D (01/10/2026)
- **Persistência de Transforms**: `Asset` ganha os campos persistentes `position`, `rotation` e `scale`. O modal de transformação no espaço de objeto atualiza esses campos, e os inputs de Transform refletem os valores persistidos do objeto ativo ao trocar ou re-selecionar ativos. Adicionados comandos semânticos canônicos `model.clear_location` (`Alt+G`), `model.clear_rotation` (`Alt+R`), `model.clear_scale` (`Alt+S`) e `model.clear_all_transforms` (`Alt+Backspace`) no menu de contexto (RMB sobre o objeto e Outliner).
- **Separação estrita de trilhos DRAW vs POLY**: a barra de ferramentas lateral esquerda separa totalmente as ferramentas exclusivas de DRAW (Sketch Profile, Shape Builder, 2D Rect, 2D Circle, Push-Pull) das ferramentas de POLY (Primitivas 3D, Extrude, Inset, Bevel, Poly Pen, Loop Cut, Slice). Nenhuma ferramenta de DRAW vaza para a barra de POLY e vice-versa.
- **Formas 2D em Parts e seleção/exclusão direta**: os perfis 2D (`ProfileResource`) passam a ser listados em Parts com identificador `profile:{id}` (com contagem de pontos e tag 2D), permitindo selecionar qualquer forma sem volume pela lista ou clicando diretamente no seu contorno na viewport. A tecla `Del` ou a opção de exclusão agora apaga perfis 2D via `DeleteProfileCmd`, eliminando shapes "fantasmas" que ficavam órfãos.

### Cor de overlay de seleção por parte e separar parte da seleção (01/10/2026)
- **Overlay customizado por parte**: cada `Asset` ganha o campo persistente opcional `selection_overlay_color: Option<[u8; 3]>`. Ao criar partes, herdam `None` (usam a cor global do projeto); na lista de partes (Inspector e Outliner flutuante), após o botão de bloqueio (lock), um quadrado colorido exibe a cor ativa da parte e abre popover com roda de cores circular HSV interativa (`crates/ui-slint/src/color_wheel.rs`), presets rápidos, input hex e botão de restaurar cor padrão (`None`).
- **Renderização com cores de parte**: os renderers WGPU, software e a camada de overlay 2D do Slint usam a cor da parte ativa com fallback transparente para a cor global.
- **Criar parte a partir da seleção**: botão "Criar nova parte da seleção" adicionado à seção Parts e atalho contextual no menu do viewport (`P` / `model.separate_selection` / `SeparateSelectionCmd`), permitindo subdividir malhas complexas em partes independentes preservando materiais, UVs e histórico transacional de Undo/Redo.
- **Testes**: testes unitários para a roda de cores HSV, testes de integração de separação de geometria por faces e por vértices com ciclo de Undo/Redo.

### Correção: toggle do wireframe overlay (01/10/2026)
- **Causa-raiz**: no renderer WGPU, `edge_mode == Topology` (modo POLY, o padrão) forçava todas as arestas finas sempre visíveis — o toggle virava no-op justamente no workspace padrão. Apurado por eliminação (comando, sync de propriedades, `TouchArea`, cache de fingerprint e viewport software, todos conformes) + leitura do código.
- **Fix** (`petunia_render_wgpu::edge_overlay_visible`, cap. 05 "Overlays, não novos modos" + Princípio de UX): o toggle é o mestre das arestas finas em qualquer modo; DRAW/POLY contribuem com as arestas de feição como leitura de forma/topologia. Default (overlay ON) pixel-idêntico ao anterior; OFF agora limpa as finas em todos os modos.
- **Checkmark do menu View**: o item `view.toggle_wireframe` (modo de shading) exibia o estado do overlay — parecia travado. Agora reflete `shading == Wireframe`.
- **Testes**: predicado puro com matriz overlay×modo×feição (headless), pixels do viewport software on/off, check do menu vs shading/overlay, e o teste GPU `draw_reads_shape_and_poly_reads_topology` atualizado para o novo contrato (POLY sem overlay esconde a aresta plana; com overlay mostra a topologia).

### Importação glTF/GLB com texturas (01/10/2026)
- **Geometria completa**: `import_scene_bytes` lê posições, índices, `TEXCOORD_0` (UV por canto), `COLOR_0` (cor por vértice) e transformações de nós (baked em espaço de mundo); `TriangleStrip`/`Fan` convertidos, pontos/linhas ignorados com aviso. Limites anti-hostis (1M verts/faces, 256 MiB buffer, 64 MiB imagem).
- **Materiais PBR + texturas**: baseColor/metallic/roughness/emissive, `AlphaMode`/`alpha_cutoff`/`normal_scale`; `baseColor`→albedo, `metallicRoughness`→roughness+metallic, `normal` e `emissive` decodificadas via `image` (PNG/JPEG, ≤1024, `data:` URIs e arquivos externos relativos ao `.gltf`). Falhas viram avisos, nunca panic nem drop silencioso.
- **Pipeline + projeto**: `GltfImporter`/`GlbImporter` retornam `materials` + `mesh_materials` (slots por face preservados); `import_file` resolve `.bin`/imagens externos via diretório base; `ProjectService::import_file_pipeline` vincula `material_id`, espelha albedo em `Asset.texture` e remapa slots. `.gltf` agora importa malhas de verdade (antes só validava).
- **Superfícies**: Slint já filtra `obj/gltf/glb` e usa o pipeline (ganha texturas sem mudar código); egui legado ganha menu "Import glTF / GLB…", comando `file.import_gltf`, `TextId` + locales en/pt-BR, FFI `petunia_import_gltf`; CLI `convert`/`info` aceita `.glb`/`.gltf`.
- **Testes**: round-trip com textura (albedo 8×8 preservado, slot 0), `.gltf` com buffer `data:` embutido, modo hostil ignorado, CLI `.petunia`→`.glb`→info e `.gltf` externo (`.bin`+`.png`)→`.petunia`→`.glb`.

### Paint/UV, DRAW, atalhos e brush — correções de fundação (30/09/2026)
- **Pintar uma face pintava as outras (causa raiz)**: o UV padrão das primitivas (`project_planar`) sobrepunha faces opostas na mesma área da textura. Todas as primitivas e o OBJ sem UV passam a usar `Mesh::layout_uv_charts` (charts por normal, densidade uniforme, sem sobreposição, determinístico, preserva quads). O Auto Unwrap (xatlas) gravava UV em texels e fazia média por vértice (colapsava as costuras); agora é por canto e normalizado.
- **Pincel 3D esférico no espaço de mundo**: mesmo raio do anel de preview, por face candidata (`Mesh::rasterize_face_near`); nunca alcança a face oposta nem faces sem relação geométrica; ignora superfícies encobertas por outros objetos.
- **Máscara e trava de pincel funcionam de verdade** (P3D-132): `PaintRestriction` (faces elegíveis por traço) vale no 3D, no canvas 2D, no balde, no gradiente e nas formas; seleção vazia com a máscara ligada não pinta; "primeira face" trava o traço. `StrokeBuffer`: *flow* acumula e *strength* é o teto (sem escurecer por sobreposição).
- **Efeito colateral removido**: escolher a ferramenta Fill no PAINT pintava os vértices de todo o objeto.
- **PAINT**: ferramenta **Select** (troca de objeto, face, ilha UV com duplo clique; `V`) e **Isolar objeto** (esconde/mostra os outros, acompanha a troca de objeto; `/`). Entrar no PAINT sempre ativa o pincel.
- **Brush modernizado (2E)**: pincéis Smudge, Blur, Dodge, Burn, Spray e Clone (Ctrl+clique define a origem); ponta redonda/quadrada/losango com ângulo e achatamento; modos de mistura (multiply, screen, add, darken, lighten); estabilizador; jitter de tamanho e opacidade; espalhamento; fluxo e espaçamento no painel; presets embutidos e do usuário (`brush-presets.json`). Novas ferramentas: Elipse e Gradiente radial; gradientes compõem por cima (a transparência final preserva a pintura) e usam a opacidade do pincel; efeito Levels disponível na pilha.
- **DRAW**: regiões fechadas e contornos de perfis ficam **sempre visíveis**; clicar no contorno reativa a forma; clicar na aresta de um perfil fechado **insere um nó sem mudar a curva** (De Casteljau) e já deixa arrastar; duplo clique ou o botão de curvas alterna **ponto reto ↔ curva** (um ponto ou todos, também em retângulo e círculo). Ctrl+clique força um ponto novo sobre uma forma existente.
- **Atalhos**: `Space` = ferramenta anterior (`Shift+Space` = micro-inspector); `Ctrl+1..5` troca de workspace; paint.* (select, isolate, airbrush, eraser, picker, fill, gradient, line, rectangle, ellipse, gradient_radial); teclas ampliadas (pontuação: barra, ponto, vírgula, menos, igual, crase, ponto e vírgula, aspas e contrabarra; setas, PageUp/PageDown, Insert). Perfis embutidos no binário (não dependem do diretório de trabalho); **perfis do usuário** (criar, editar por captura de tecla, restaurar, apagar) em `<config>/petunia3d/keymaps`; editar um perfil embutido cria uma cópia; conflitos sinalizados. `petunia-default`: `select_object = "4"`.
- **Shell**: arrastar painéis flutuantes não trava mais (a posição era recalculada a partir do clique enquanto o painel se movia); alça de redimensionar (duplo clique restaura) em Configurações, Reference Manager e Canvas; `PetuniaSlider` com modo `controlled` (escrever em `value` desfazia bindings de uma via: sliders de Configurações, opacidade de camada e parâmetros de efeito deixavam de seguir o modelo); lista de perfis de atalhos vertical (nomes não são mais cortados).
- **Desempenho**: cache da cena de consultas (picking, hover, oclusão, pintura) por chave de geometria — antes era refeita a cada movimento do mouse; o pincel 3D calcula normais/centros/raios das faces uma vez por chamada.
- **Decal por importação** (P3D-133): botão de imagem na pilha de camadas importa PNG/JPEG como decalque (validado, ≤ 512 texels, proporção preservada, 1 Undo). **Poly Pen**: clicar numa aresta (sem polígono em coleta) insere um ponto nela (`Mesh::split_edge`, 1 Undo).
- **Layout 1/2/4 vistas (estudo)**: `docs/development/multi-viewport-study.md` e o modelo puro `view_layout` (retângulos, recuo 4→2→1, agendador de vistas passivas) com testes; **sem mudança visível** até a decisão de produto.
- **Testes**: UV/layout (6), spline (3), engine de pintura (restrição, esfera 3D, ponta, spray, dodge/burn, blur, clone, smudge, jitter, elipse, gradiente), atalhos (6), brush panel (3), layout de vistas (6). Quatro testes antigos codificavam o bug (UV planar sobreposta; `0` como atalho de objeto) e foram atualizados de propósito. **Sem captura nativa**; nenhum gate completo executado nesta rodada.

### Animate F2b — malha deformada no viewport e Fit to model (AN-19/AN-21) (30/09/2026)
- **Domínio** (`petunia_project::posing`): `PoseOverride` (malhas deformadas transitórias, nunca serializadas), `skin_mesh`/`posed_meshes` (Linear Blend Skinning com id de osso → índice), `fit_skeleton_to_bounds` (qualquer criatura ao volume do modelo, com giro de 90° quando o eixo longo difere) e `compute_blended_skin_weights` (osso mais próximo domina, mistura só perto das articulações).
- **Comando**: `FitRigToActiveAssetCmd` liga o modelo ativo à criatura em foco em um passo de Undo; repetir não cria histórico; Motions, papéis e IK seguem valendo (o rig mantém o id). `AnimatePreview::pose_override` alimenta o renderer sem tocar o documento.
- **Renderers**: `PetuniaViewport::set_pose_override`; software e wgpu desenham a malha deformada no lugar do repouso. O wgpu só reconstrói buffers quando a revisão do override muda, e o bridge só avança a revisão quando a geometria muda. O viewport é re-renderizado em tick de reprodução, scrub e draft de slider quando há modelo ligado.
- **UI**: botão **Fit to model** (em destaque enquanto nada está ligado; desabilitado com o motivo — sem criatura, sem modelo, modelo travado) e linha "Modelo ligado".
- **Testes**: 12 de domínio (LBS por id, repouso = identidade, fit de todos os presets, giro de eixo, pesos, caminhada sem explosão), 6 de comandos, 5 de bridge/software/pixels e 1 de shell. **O caminho wgpu não foi exercitado com GPU** (sem adaptador no ambiente): há teste puro da revisão e um teste de GPU pulado sem adaptador.
- **Não incluído**: assets com modifiers (ficam em repouso), correção de volume além do LBS, pintura de pesos, inversão frente/costas e avaliação visual.

### Animate F2 — workspace Animate no Slint, camadas 1–2 (P3D-066/170) (30/09/2026)
- **Sessão neutra** (`petunia_core::animate_session`): `AnimateSession` (criatura, Motion, playhead, play, draft de slider), `AnimatePreview` (avaliador em cache **por conteúdo** e amostragem da pose), `motion_catalog` (o que cada rig suporta, motivo como enum) e ações `animate_*` em `AppState` via comandos transacionais. `RigPresetKind` ganha serpente, peixe e pássaro; `BipedCycle` passa a dizer "faltam pernas" antes de "falta osso de corpo".
- **Shell Slint** (`animate.rs`, `ui/animate.slint`): pill **ANIMATE** (só com a feature `animation-workspace`, ADR 006), Inspector `Creature → Motions → Motion` (Styles + sliders universais + Advanced, Stepped, Root Motion) aberto por padrão no Animate, bandeja com um botão por Motion e transporte in-canvas (play/pause, scrub, **Bake a copy** = Keep Live, **Apply Now** = converter). Esqueleto posado projetado como `Path` (independente de wgpu/software).
- **UX**: um clique cria, seleciona e toca; Motions impossíveis ficam desabilitados com o motivo; arrastar um slider é *draft* ao vivo e soltar confirma **um** passo de Undo (teclado confirma a cada passo); listas e sliders atualizam **no lugar** (foco e arrasto não se perdem). Textos por `TextId`/`animate.<grupo>.<id>` em en e pt-BR, cobertos por teste.
- **Correções**: `petunia_ui_slint` **não compilava** com `--features animation-workspace` (`match` de `Workspace` não exaustivo em Ctrl+PgUp/PgDn); o ciclo de workspaces agora inclui ANIMATE só com a feature e o teste correspondente acompanha.
- **Testes**: 16 no bridge (view-model, gesto de slider, tick, pose projetada, cache, locales) e 11 no shell real (`tests/animate_shell.rs`: cliques, arrasto, teclado, acessibilidade, layout sem rolagem). Eles acharam quatro defeitos de markup antes da entrega (FocusScope engolindo cliques, z-order do transporte, sobreposição com a barra contextual, painel maior que a janela).
- **Não incluído**: malha deformada no viewport (só o esqueleto posado), atalhos do Animate, timeline com keys/Ghosts (F3), aceite visual e "teste do iniciante" — **sem captura de tela**. `viewport_gestures::viewport_shortcut_drag_parametric_hover_and_navigation_gesture` já falhava no HEAD anterior (gesto do tripé) e segue falhando, independente do Animate.

### Contorno de seleção de objetos — Onda 6, parte 4 (30/09/2026)
- **Objetos selecionados agora aparecem no viewport**: antes, no modo de objetos, nada indicava quais estavam selecionados. Agora cada um ganha um contorno de largura constante em pixels, com a borda suavizada; o ativo usa a cor de seleção e os demais a mesma cor mais escura. A espessura segue a preferência de espessura da seleção.
- **Verificado por pixels**: largura do anel medida, igual ao afastar a câmera, e o interior do objeto intacto.
- **Escopo honesto**: o contorno ainda aparece por cima de objetos que estejam na frente, e não aparece no shading Wireframe nem no viewport por software.

### Plano de trabalho em destaque — Onda 6, parte 3 (30/09/2026)
- **O plano de trabalho aparece no DRAW** (capítulo 05): um recorte translúcido com grade, centrado na origem do desenho e do mesmo tamanho na tela em qualquer zoom. Aparece quando o plano está decidido (travado, ou com um perfil em edição); antes do primeiro clique no modo automático, o destaque da face sob o cursor continua mostrando o candidato.
- **Verificado por pixels** no renderer real, e as regras de quando aparecer por teste do bridge.
- **Correção de registro**: os pontos do domínio Point já eram desenhados como discos; a parte 2 os listou como pendentes por engano.

### Paint e viewport dividida (30/09/2026)
- **Ícones com arte própria**: 14 IDs que só reaproveitavam outro ícone (Shade Flat/Smooth, Isolate, Normals Overlay, Flip Normals, Backface Culling, Bounding Box, Maximize, Minimize, Origin, Selection Center, Paint, Reference Manager, Object Mesh) ganharam desenho distinto nas duas variantes. Restam aliases intencionais (abas de dados, `view_*` = `shading_*`).
- **Ícones booleanos redesenhados**: Fuse (união sólida), Cut (união com mordida e contorno tracejado do volume removido), Intersect (interseção destacada com arcos tracejados) e Join (duas partes ligadas por ponte) agora são distinguíveis a 16–20 px; antes eram quatro variações do mesmo par quadrado+círculo.
- **PiP do Paint**: botão no Canvas do Inspector troca os papéis — o canvas 2D ocupa a viewport e o 3D vira um inset ao vivo (clique para voltar); ao iniciar um traço, a câmera 3D enquadra a face cujo UV foi tocado. Limite: com o PiP aberto os cards flutuantes do Inspector ficam cobertos.
- **Ícones**: traço padronizado em 1,75 px nos dois packs (antes 1,4 a 1,8), novo `cargo run -p xtask -- icons-check` (grade 24, só `currentColor`, larguras permitidas, paridade outline/filled) e remoção dos 26 SVGs legados não usados do shell Slint.
- **Paint+UV unificado (primeiro passo)**: a pill UV saiu da barra de workspaces (MODEL | PAINT); "Preparar superfície" (Unwrap e Pack Islands) entra no Inspector do PAINT. O código do workspace UV ainda existe por trás de atalhos e será removido quando o wireframe UV do canvas cobrir a edição de ilhas.
- **Pincel coerente**: `BRUSH_PX_PER_UNIT` é a única relação entre o slider "Size" e o pincel; o carimbo 3D, o clique na malha e o pincel do canvas 2D usam o mesmo diâmetro (o 2D estava preso em 4 px). Anel de preview na viewport PAINT com o mesmo diâmetro e cursor do sistema escondido nas ferramentas de pincel.
- **Canvas 2D flutuante**: `PaintCanvasSurface` (raster, wireframe UV, grade, pan com botão do meio/direito, zoom por roda) no Inspector e em janela arrastável sincronizada com a viewport 3D.
- **Viewport dividida (opcional)**: menu View ou botão da barra da viewport abre uma segunda vista 3D lado a lado com câmera própria (Persp/Frente/Trás/Esq/Dir/Topo), renderizada pelo caminho de software. É vista de navegação: edição continua na principal; documento, undo e seleção são compartilhados. Recusada abaixo de 2×480 px. Divisão vertical, PiP 2D/3D com auto-enquadramento e canvas 2D como divisão da viewport seguem pendentes.

### Redesenho da GUI: temas, i18n, movimento, painéis flutuantes e Prefabs (30/09/2026)
- **Revisão de baseline (cap. 36, 2026-09-30)**: painéis flutuantes não modais, até 2 viewports opcionais, Asset Library dupla, Paint+UV unificados com Texture Graph futuro, tema claro oficial, i18n obrigatório e sistema de ícones único. A divisão de viewport, o Paint+UV unificado e o Texture Graph estão **decididos, não implementados**.
- **Temas**: novo `petunia-light` (neutros quentes, sem branco puro, WCAG AA), dark alinhado ao accent violeta do baseline, High Contrast com texto legível sobre accent. `ShellColors`/`ShellToken` cobrem os tokens de shell; o tema escolhido persiste em `preferences.toml` (`theme_id`). Testes de contraste e de contrato pack-em-disco == embutido. O fundo do viewport 3D ainda não segue o tema.
- **i18n**: 633 literais do `app.slint` migrados para `Tr.t("sl.*")` (en e pt-BR, mesma paridade) com teste-guarda de chaves. Restam textos montados em Rust (status) e atalhos físicos escritos à mão.
- **Movimento e feedback**: tokens `Motion` (100/140/180 ms, respeita movimento reduzido), hover/pressed/foco animados, seções do Inspector com altura animada, slider Petunia próprio, toast de status por severidade, menus com separadores/itens desativados/check e navegação por teclado.
- **Painéis flutuantes**: Reference Manager e Settings deixam de ser modais (sem scrim), arrastáveis pelo cabeçalho, duplo clique recentraliza; Reference Manager compacto à esquerda.
- **Ícones**: o gerador transforma os recortes falsos (`#252735`) em máscaras SVG reais; os packs ficam monocromáticos em `currentColor` e o Slint os tinge por token (legíveis no tema claro).
- **Asset Library com Prefabs**: novo modelo `Prefab`/`PrefabLink` no projeto (persistido, retrocompatível), separado da cena; "Salvar seleção como prefab" não cria objetos na cena, instanciar é desfazível, instâncias desatualizadas são detectáveis. A gaveta mostra miniaturas renderizadas, busca, favoritos, ordenação, colocar/atualizar/excluir. O workspace de gestão completo (catálogos, tags, renomear inline) permanece pendente.
- **Correções**: teste desatualizado de Smooth shading; `viewport_shortcut_drag_parametric_hover_and_navigation_gesture` já falhava antes desta rodada e segue vermelho.

### Arestas nítidas e aparência por modo — Onda 6, parte 2 (30/09/2026)
- **Arestas de largura constante**: as linhas do modelo têm a mesma espessura em pixels em qualquer zoom, distância ou escala de tela, com bordas suavizadas (antes eram linhas de 1 px físico).
- **DRAW lê forma, POLY lê topologia** (capítulo 05): no DRAW aparecem só as arestas de feição (bordas e dobras acima de 30°); no POLY aparecem todas, as comuns mais finas e as de feição reforçadas; PAINT e UV mantêm as faces limpas.
- **Verificado por pixels** no renderer real (Vulkan por software): largura medida e visibilidade de cada tipo de aresta por modo.
- **Escopo honesto**: pontos visíveis no domínio Point, destaque do plano de trabalho, contorno de seleção por jump flooding e matcap ficam pendentes.

### Visual e acessibilidade — Onda 6, parte 1 (30/09/2026)
- **Luz de estúdio que acompanha a câmera**: a forma continua clara e legível ao orbitar para qualquer lado (como no Plasticity e no Cinema 4D); a preferência "Luz de estúdio acompanha a câmera" volta à luz fixa no mundo. X-Ray e viewport por software usam a mesma luz.
- **Raio do snap nas Configurações** (4–48 px): quem tem tremor ou pouca precisão pode aumentar a área de encaixe.
- **Aparência testada por pixels**: os testes do renderer rodam com Vulkan por software (lavapipe) e medem o brilho do modelo visto de frente e de trás.
- **Roteiro de teste com usuários**: `docs/development/user-test-protocol.md` reúne as 7 decisões em aberto das Ondas 3–6, tarefas, métricas (SEQ, SUS) e critérios de decisão, com um grupo de pessoas com limitação motora.
- **Escopo honesto**: linhas de largura constante com AA, contorno por jump flooding, matcap, aparência por modo e girar a luz por arrasto ficam pendentes. Matriz em `docs/development/wave6-visual-accessibility-gap-matrix.md`.

### Poly Pen — Onda 5, parte 1 (30/09/2026)
- **Poly Pen no trilho POLY** (modelo: Polygon Pen do Cinema 4D): passar o mouse destaca ponto, aresta ou face; arrastar move o elemento sem selecionar antes.
- **Ctrl-arrastar uma aresta de borda** cria uma face nova a partir dela (um gesto, um Undo; `Esc` restaura).
- **Desenhar polígonos**: cada clique adiciona um ponto (ou reaproveita um existente); Enter ou clicar no primeiro ponto fecha; Backspace remove o último; `Esc` limpa. A face nova segue a orientação dos vizinhos ou fica de frente para a câmera; resultados não manifold são recusados.
- **Ctrl-clique** num ponto o derrete.
- **Escopo honesto**: ícone próprio, modificador pelo keymap, subdividir aresta por clique e pintar faces ficam pendentes. Matriz em `docs/development/poly-pen-gap-matrix.md`.

### DRAW e POLY no seletor — Onda 4, parte 2 (30/09/2026)
- **Seletor `DRAW · POLY · PAINT · UV`**: MODEL vira dois modos do mesmo workspace de modelagem — mesmo documento, seleção, câmera e Inspector; muda o trilho de ferramentas.
- **Trilho por modo**: DRAW mostra Sketch, Retângulo, Círculo, Push/Pull e Duplicar; POLY mostra as ferramentas de componente (Extrude, Inset, Bevel, Knife, Loop Cut e demais). Trocar de modo nunca converte nem seleciona nada; uma ferramenta que não existe no novo trilho volta para Select.
- **Correção**: empurrar uma região solta para o lado negativo agora gera um sólido voltado para fora.
- **Escopo honesto**: seleção `Shape / Curve / Point / Region` no DRAW, aparência de viewport por modo, plano por 3 pontos e formas paramétricas com "Converter em polígonos" continuam pendentes; padrão POLY a validar com usuários.

### Regiões e Push/Pull do DRAW — Onda 4, parte 1 (30/09/2026)
- **Regiões fechadas como no Plasticity**: perfis que se cruzam no mesmo plano formam regiões (arranjo planar com furos); a região sob o cursor recebe tinta translúcida no Draw e no Push/Pull.
- **Push/Pull na região**: arrastar uma região com Push/Pull gera volume na hora, com o mesmo gesto das outras ferramentas (valor segue o cursor, digitação vence o mouse, "Última operação" ajustável).
- **Draw on Face**: uma região desenhada dentro de uma face é gravada nela (imprint); puxar para fora soma volume, empurrar para dentro cria um rebaixo. No vazio, a região vira um sólido fechado num objeto novo ("Forma").
- **Um gesto, um Undo**: imprint e extrusão entram como uma única entrada de histórico; `Esc` restaura o documento exatamente; reajustar a "Última operação" não duplica o imprint.
- **Escopo honesto**: pill DRAW e trilho de forma (parte 2), imprint de região que cruza arestas ou tem furos sobre face, e extrusão negativa de região solta ficam pendentes. Matriz em `docs/development/draw-regions-gap-matrix.md`.

### Pré-seleção e plano de trabalho — Onda 3, parte 2 (30/09/2026)
- **A câmera nunca se move sozinha**: ativar o Draw ou escolher o plano Face não troca mais a vista nem a projeção; o novo botão "Olhar para o plano" faz isso sob comando.
- **Plano automático**: no 1º clique de um perfil novo, a face sob o cursor vira o plano (Draw on Face); sem face, o plano do mundo mais paralelo à vista, pelo 3D Cursor.
- **Plano travado**: Chão, Face e Vista travam o plano até voltar para Auto; o card mostra a pílula Auto e textos traduzidos (en/pt-BR).
- **Preferência de plano automático**: Configurações → "Plano automático favorece o chão". Ligada (estilo SketchUp), o desenho sem face sob o cursor vai para o chão, a menos que a câmera esteja quase na horizontal (< 20°); desligada (padrão, estilo Modo/Cinema 4D), usa o plano do mundo mais de frente para a vista.
- **Pré-seleção no Draw**: o hover mostra a face que viraria o plano (marcador "Na face") e, depois, o ponto encaixado que o clique criaria.
- **Escopo honesto**: plano por 3 pontos/face + aresta, regiões no hover e imprint na face hospedeira ficam para a Onda 4; a regra do eixo dominante em perspectiva será validada com usuários. Matriz em `docs/development/snap-inference-gap-matrix.md`.

### Snapping com inferência — Onda 3, parte 1 (30/09/2026)
- **Tolerância em pixels**: o snap passa a medir a distância na tela (padrão 12 px, faixa 4–48), com o mesmo alcance em qualquer zoom; antes era 0,35 unidade de mundo.
- **Uma passada por gesto**: o Move encaixava duas vezes (bridge contra a malha em prévia e core); agora o bridge faz uma única consulta contra a malha de origem, ignora a geometria que se move e o core apenas registra o que encaixou. Reajustar a "Última operação" não reencaixa mais o valor digitado.
- **Tipos nomeados com prioridade**: Ponto, Ponto médio, Na aresta, Guia de eixo X/Y/Z, Na face (ponto sob o cursor, não o centroide) e Grade; pontos atrás da malha só com raio-X/wireframe.
- **Inferência de direção**: guias paralelas aos eixos a partir da âncora (snap-dragging); com grade, o comprimento ao longo da guia anda no passo da grade.
- **Desenho de perfil**: usa a mesma passada (pontos da malha projetados no plano, guias do plano a partir do último ponto, grade do plano); a grade fixa de 0,25 foi removida.
- **Nunca só cor**: marcador com forma por tipo e rótulo traduzido (en/pt-BR); o HUD mostra o tipo encaixado.
- **Escopo honesto**: centro de face, interseção, paralelo/perpendicular a aresta, passos de 15°, snap entre objetos, campo de raio em Configurações, índice espacial e pré-seleção universal ficam pendentes. Matriz em `docs/development/snap-inference-gap-matrix.md`.

### Gramática única de ferramenta — Onda 2 (30/09/2026)
- **`ToolSession` no core**: uma máquina de estados (`Idle/Pressed/Dragging/Latched`) decide clique × arrasto com limiar configurável; clicar-mover-clicar em alças (WCAG 2.2, 2.5.7); valor digitado vence o mouse; testes de domínio sem toolkit.
- **Ferramentas persistentes**: Extrude, Inset, Round Edge e Push/Pull deixam de abrir operação ao apertar a tecla; arrastar sobre uma face (ou aresta) a seleciona e opera seguindo o cursor pela normal projetada, estável sob zoom. Move/Rotate/Scale e o gizmo passam pela mesma máquina.
- **"Última operação"**: cada gesto é 1 Undo e continua ajustável no card da ferramenta, reaplicado dentro da mesma entrada de histórico.
- **Contrato de entrada**: RMB nunca cancela (abre menu fora de gesto); navegação nunca é suspensa; a roda sempre faz zoom e Ctrl+roda ajusta contagens; Esc em escada (gesto → overlays → ferramenta volta para Select); Esc mantém a primitiva recém-criada; nada é selecionado sozinho.
- **Preferências de acessibilidade**: distância para começar a arrastar (2–16 px) e "arrastar sem segurar o botão".
- **Escopo honesto**: Loop Cut, Slice, Knife e Profile ainda usam ciclos próprios (Ondas 4/5); `Space` = ferramenta anterior e alças dedicadas de Extrude/Inset ficam pendentes. Matriz em `docs/development/tool-session-gap-matrix.md`.

### Viewport nítido e bugs comprovados — Onda 1 (30/09/2026)
- **Nitidez**: a imagem 3D passa a ser renderizada em pixels físicos (HiDPI e UI scale), com MSAA 4× resolvido na textura exibida; larguras de linha continuam em px lógicos.
- **Fluidez**: mouse parado sobre o mesmo alvo não reconstrói o view model nem redesenha o frame.
- **Correções**: buffer numérico não vaza mais entre operações (E → 2 → Enter → E → 5 extrudava 25); card e Cancelar da faca funcionam (`cut`); hover do gizmo clareia; anel de View Roll clicável onde é desenhado; `is_snapped` significa "encaixou"; cota linear só para distâncias (não para graus/fatores); HUD mostra o valor digitado.
- **Escopo honesto**: coalescer renders por quadro, linhas de largura constante com AA e contorno por jump flooding ficam para ondas seguintes. Matriz em `docs/development/viewport-crispness-gap-matrix.md`.

### Decisão DRAW + POLY e gramática única de ferramenta — Onda 0 (29/09/2026)
- **Pesquisa registrada**: novo capítulo 46 do Livro Vivo com a análise de Plasticity (incluindo o código aberto inicial), Cinema 4D, Modo e SketchUp, a literatura acadêmica aplicável (snap-dragging, beautification, κ-curves, bubble cursor, cursores de área para deficiência motora, WCAG 2.2) e o diagnóstico da implementação Slint.
- **ADR 007 aceito**: workspaces **DRAW** (nível de forma) e **POLY** (nível de componente) substituem MODEL; todas as ferramentas passam a seguir uma gramática única — persistente, arrastar pela alça ou em qualquer lugar, valor digitado, clicar-mover-clicar, "Última operação" ajustável, RMB = menu, Esc cancela, navegação nunca suspensa.
- **Referência canônica**: `Blender.svg` e o manual do Blender deixam de ser referências; caminhos absolutos quebrados (`Projetos/simple3d-modeling`) foram convertidos em caminhos relativos.
- **Escopo honesto**: somente documentação. O código ainda exibe MODEL e mantém cinco mecanismos de sessão; as páginas revisadas marcam "decisão aprovada, implementação pendente".
### Animate F1 — geradores de movimento procedural (P3D-170) (30/09/2026)
- **`MotionRecipe`** (`Project::motions`, append-only) avaliado por `MotionEvaluator` como `pose(t)` puro e determinístico (mesma receita ⇒ mesma pose); parâmetros com faixa (`speed`, `energy`, `weight`, `stride`, `lean`, `smoothness`, `variation` + específicos), **Styles** (Cartoon, Heavy, Stiff, Floaty), **Stepped** (quantização temporal), In Place/Root Motion e contrato de rig via Rig Roles com erro legível.
- **Geradores**: `BipedCycle` (walk↔run, pés plantados por IK, braços contralaterais, lean/torção), `Gait` de N pernas (alternado, sequência lateral, onda; alternating tetrapod para aranhas; ondulação de coluna e cauda), `Serpentine` (serpente, peixe, cauda) e `IdleBreath`. Frente/lateral vêm do próprio rig.
- **Apply Now** (`bake_motion`): amostra ciclos inteiros e reduz chaves por canal (Ramer–Douglas–Peucker) com relatório do que mudou; canais no repouso não geram chaves. O glTF exporta Motions **sempre baked**; os que não podem ser exportados aparecem como aviso no relatório.
- **Comandos** transacionais: `AddMotionCmd`, `SetMotionParamCmd`, `SetMotionStyleCmd`, `UpdateMotionCmd`, `DuplicateMotionCmd`, `RemoveMotionCmd`, `ApplyMotionNowCmd` (Keep Live ou converter em clipe); não tocam revisões de render.
- **Testes** (22 no projeto + 6 no core): fechamento de loop, determinismo, seed/variation, só rotações (exceto raiz), comprimentos de osso, pés plantados (walk e run), aranha 8 pernas, quadrúpede, onda com defasagem, sentido de joelho/braços/lean, extremos sem NaN, bake, persistência e export.
- **Não incluído**: Wing Flap, Reaction, Action, Look/Aim e UI. Valores padrão sem avaliação visual.

### Animate F1.1 — presets de serpente, peixe e pássaro (30/09/2026)
- `RigPreset::serpent(segments, scale)`, `fish(scale)` (serpente de 9 segmentos + nadadeiras) e `bird(scale)` (coluna horizontal, pernas de 3 ossos, asas `Wing_n.L/R`, cauda), como templates de dados (P3D-136).
- Inferência de papéis passa a reconhecer `Spine_n` numerado e `Wing_n.L/R` (`RigRole::Wing`); Mixamo `Spine1/Spine2` continua igual.

### Animate F0.6 — comandos transacionais de rig, papéis, IK e clipes (30/09/2026)
- **`petunia_core::rig_commands`**: presets de rig (humanoide, quadrúpede, multi-leg), Auto-Rig do asset ativo (fit + pesos), remoção de esqueleto em cascata (papéis, IK, vínculos de skin), atribuir/limpar/redetectar papéis, adicionar/editar/remover cadeias de IK, criar/remover clipes e gravar/apagar chaves de osso (valores locais absolutos, rotação normalizada).
- **Contrato**: `can_execute` sem efeitos, `NoChange` sem histórico, rollback do dispatcher, `do → undo → redo` idêntico por hash e nenhuma revisão de render alterada (`changes() = NONE`). Import glTF pelo `ProjectService` liga esqueleto, pesos, papéis e clipes e é desfazível.
- **Fora do escopo**: registro em `canonical()` (palette/keymap/MCP) depende da decisão D-21; edição de ossos segue no egui legado.
- **Testes**: 8 de comandos e 1 de importação pelo serviço. O teste do Slint `test_viewport_context_menu_modeling_actions_and_dismissal` estava desatualizado desde o G7 (mensagem de `model.shade_smooth`) e foi alinhado; suíte Slint 345/345 verde.

### Animate F0.5 — IK mínimo (P3D-169) (30/09/2026)
- **Solvers puros** (`petunia_project::ik`): `solve_two_bone` (analítico com pole vector e soft IK exponencial), `solve_fabrik` (raiz fixa, comprimentos preservados) e look-at; alvos inalcançáveis/próximos demais são limitados sem NaN.
- **Aplicação na pose**: `solve_chain` altera somente as rotações locais dos ossos da cadeia (swing até a nova direção, preservando a torção) com mistura por `weight`; `Skeleton::world_pose_matrices` exposto.
- **Dados**: `IkChain` (`Project::ik_chains`, append-only) com validação (ossos existentes e contíguos) e `IkChain::two_bone_leg` a partir dos Rig Roles; poda em `validate()`.
- **Testes**: alcance, comprimentos, pole, continuidade do soft IK, FABRIK/look-at no esqueleto, pesos, determinismo e persistência. Limites angulares e comandos ficam para a sequência da F0.

### Animate F0.4 — Rig Roles (P3D-169, dados) (30/09/2026)
- **Domínio** (`petunia_project::rig_roles`): `RigRole` (Root, Hips, Spine(n), Chest, Neck, Head, Jaw, Leg/Arm por membro e parte, Wing, Tail(n), Tentacle, Wiggle(n)) e `RigRoleMap` com atribuição validada (osso existente, papel único por osso — o erro nomeia o dono), pernas completas, cadeias de coluna/cauda e `RigRequirement` com mensagens legíveis.
- **Inferência por nomes**: presets Petunia (humanoide, quadrúpede, multi-leg com 2–16 pernas), Mixamo e nomes genéricos; nomes desconhecidos ficam sem papel.
- **Projeto**: `Project::rig_roles` (append-only, `serde(default)`; arquivos antigos abrem sem o campo), `add_skeleton` infere os papéis, `validate()` poda ossos/esqueletos ausentes; `assign_rig_role`/`clear_rig_role`/`infer_rig_roles`.
- **glTF**: papéis em `skin.extras.petunia.roles` (por nome de osso); import restaura ou infere para rigs externos.

### Animate F0.2/F0.3 — glTF com skin, joints e animação (30/09/2026)
- **Export** (`petunia_project::gltf_rig`, integrado a `export_gltf`): nós de joint com TRS de repouso, `skin` com `inverseBindMatrices`, `JOINTS_0`/`WEIGHTS_0` por vértice, `animations` por osso (translation/rotation/scale; LINEAR ou STEP) e `extras.petunia` para round-trip. Sem rig o GLB é idêntico ao anterior. Avisos em `export_report` quando a malha avaliada não casa com os pesos (skin omitida, nunca silenciosa).
- **Import** (`import_rig`, `Project::add_imported_rig`, `ImportPayload.rig`): esqueleto a partir da IBM (com rotação de repouso), pesos `JOINTS_0` u8/u16, clipes (fps/loop/duração via extras), `scale` aplicado; CUBICSPLINE convertido e ancestrais/escala/canais ignorados geram **avisos**. O `ProjectService` liga esqueleto, pesos e clipes ao projeto.
- **Rig Core**: `compute_bind_pose_matrices` respeita a rotação de repouso (rigs externos).
- **Testes**: 9 novos de round-trip (nomes, hierarquia, heads, bind, pesos por vértice, poses amostradas, rotação de repouso, escala, pipeline) + GLB externo montado à mão + export (skin, pesos, animações, tempos inválidos, skin omitida).
- **Registrado**: AN-17 (pesos guardam IDs, consumidores tratam como índice); o import de malha ainda ignora UV/normais.

### Animate F0.1 — correção do bind pose do Rig Core (30/09/2026)
- **AN-16 corrigido**: `Skeleton::compute_bind_pose_matrices` tratava `head` (absoluto) como offset relativo ao pai, acumulando a hierarquia; ossos fora da origem não voltavam à identidade em repouso. Agora o bind é `translate(head)` e `local_transform.translation` guarda o offset de repouso (`head − head_do_pai`), a mesma convenção dos joints do glTF.
- **Amostragem**: `AnimationClip::sample_pose` usa `BoneTrack::sample_transform_over`, e canais sem keyframes herdam o repouso (uma trilha só de rotação não solta mais o osso do pai). O clipe `Humanoid_Idle` passou a respirar em torno do repouso e o botão "Inserir Pose Key" do egui legado grava o repouso do osso.
- **Testes**: repouso ⇒ skinning identidade em hierarquia deslocada, offsets após reparent/remoção e amostragem só-rotação.

### Animate procedural-first — documentação (30/09/2026)
- **Decisão de produto registrada**: Animate acessível a quem não entende de animação, com **animação procedural primeiro**, **criaturas de primeira classe** (humanoide, quadrúpede, multi-leg, serpente, peixe, pássaro) e referência de animação por **batch de imagens** (sem vídeo, sem ML). Sem alteração de código.
- **Caderno**: novo capítulo 45 (visão, UX em quatro camadas, Gap Matrix `AN-01…AN-15`, análise do Dust3D, pesquisa externa e fases F0–F5); novas specs **P3D-169** (Rig Roles & IK), **P3D-170** (Procedural Motion Generators), **P3D-171** (Ghosts & Trajectories), **P3D-172** (Reference Image Sequence), **P3D-173** (Secondary Motion & Ragdoll) e **P3D-174** (Layered Animation), todas em `SPEC DRAFT`.
- **Emendas**: P3D-066/067 (exceções nomeadas a "constraints" e "simulation"), P3D-135/136/138/139/013/160, cap. 08 (Era 3), cap. 13 (vocabulário de animação), adendo pós-V1 no cap. 36 e ADR 006 (`docs/architecture/adr/006-workspace-animate-pos-v1.md`). A UI Baseline V1 `MODEL / PAINT / UV` **não** foi reaberta.
- **Achados registrados**: o domínio de rig/clipes existe em `petunia_project`, mas não alcança o Slint (AN-07) e o glTF não exporta skin/animação (AN-13).
- **Gates**: `xtask bible-check` passa a validar P3D-001–174 e fundamentos 01–45.

### Iconografia vetorial Petunia oficial e temas configuráveis (28/09/2026)
- **Integração nativa no shell Slint**: Os 249 ícones vetoriais SVG oficiais criados para o Petunia3D foram compilados em catálogo declarativo dinâmico (`PetuniaIcons` em `petunia_icons.slint`), fornecendo suporte unificado a duas variantes estéticas: `outline` (traço) e `filled` (preenchido).
- **Troca dinâmica e reatividade imediata**: Alternância de estilo em tempo de execução via seletor em Configurações -> Aparência, com preview dos dois estilos e callback reativo conectado ao singleton global Slint sem reinicialização.
- **Persistência em preferências**: Adicionado campo `icon_theme` em `UserPreferences` (`preferences.toml`) com valor padrão `"outline"` e sanitização segura em caso de dados desconhecidos.
- **Adoção em controles de produto**: Primitivas 3D, shelf de modelagem, operações booleanas, modos de shading, ferramentas de transformação, domínios de seleção, top bar, cabeçalhos do Inspector e pílulas recolhidas atualizados para consumir os ícones de `PetuniaIcons`.

### Gate G4 — Fundação Headless de SurfaceAttachment (27/09/2026)
- **Âncora geométrica persistente**: pontos de spline podem referenciar target, triângulo real de render/picking e coordenadas baricêntricas, com offset normal, orientação tangente e round-trip `.petunia` retrocompatível.
- **Estados sem reprojeção silenciosa**: conectividade incompatível produz `NeedsReattach`, alvo removido produz `MissingTarget` e detach preserva o último ponto mundial conhecido.
- **Queries e transações**: raycast/frame/UV seam, slide, rotate e reprojeção explícita são compartilhados no domínio; Commands tipados integram Undo/Redo e rejeitam no-op, target bloqueado e move mundial ambíguo.
- **Escopo honesto**: a fundação opera sobre a mesh autoral; transforms parent/child, outputs avaliados de modifiers, migração de Decal, UI Slint, Profile/generators e Hair permanecem em gates posteriores.

### Gate G3 — Fundação Headless de Spline Core (27/09/2026)
- **Authoring persistente**: `SplineResource` adiciona polyline/Bézier, paths abertos/fechados, control points com IDs/handles, revisão própria e round-trip `.petunia` retrocompatível.
- **Avaliação compartilhada**: comprimento de arco, tangentes, resampling e frames de transporte paralelo em `f64` passam a alimentar tanto splines quanto o Sweep existente, removendo RMF duplicado.
- **Transações e cache**: comandos tipados cobrem o ciclo autoral com Undo/Redo e invalidação exclusiva de splines; caches derivados usam revisão + fingerprint geométrico e limites contra avaliações patológicas.
- **Escopo honesto**: SurfaceAttachment, overlay Slint, Profile persistente, generators e Hair continuam nos gates seguintes; `HA-01` fica `PARTIALLY_COMPLIANT`, não concluído.

### Gate G2 — Upload Regional de Textura WGPU (27/09/2026)
- **Dirty regions transitórias**: tiles alterados atravessam Core e o contrato neutro de viewport sem persistir estado de GPU no documento.
- **Upload proporcional**: o WGPU elimina o FNV integral por stroke, combina regiões contíguas e envia somente as linhas RGBA afetadas, com fallback integral seguro para criação, resize e invalidações sem região.
- **Cache desacoplado**: revisões de textura não invalidam buffers de geometria; fingerprints e telemetria separam rebuilds, chamadas e bytes de upload.

### Gate G1 — Stroke e Composição Incremental de Paint (27/09/2026)
- **Stroke determinístico**: `StrokeSampler` neutro preserva a distância residual entre pointer events; PAINT 2D e 3D passam a respeitar `BrushSettings::spacing` independentemente da taxa de eventos.
- **Batch e Dirty Tiles**: dabs de uma amostra, inclusive simetrias 2D/3D, são agregados antes de uma única composição e publicação de revisão; a stack deixa de ser clonada no caminho parcial.
- **Sincronização incremental**: o albedo derivado copia somente as linhas dos tiles alterados quando dimensões e recurso já existem, mantendo fallback integral apenas para inicialização/resize.
- **Canvas nativo**: Slint recebe RGBA na resolução original; zoom, recorte, nearest-neighbor e grade vetorial passam à apresentação, removendo o pico de cerca de 64 MiB do antigo upscale 256² × 16.

### Gate G0 — Integridade Transacional, Revisões e Cache (27/09/2026)
- **Transações e Undo/Redo**: `CommandDispatcher` passa a ser o owner único do lifecycle de comandos destrutivos; Paint, decal, delete/dissolve e Profile Volume usam snapshots anteriores e um único roundtrip, com cancelamento sem histórico.
- **Revisões Granulares**: novos domínios para topologia, posições, normais, seleção, UV, cores, materiais, texturas e transforms reduzem invalidações indevidas e permanecem monotônicos após restore.
- **Histórico e Cache**: orçamento de Undo considera dados heap do projeto; cache de modifiers identifica todo o conteúdo da malha fonte, não apenas contagens.
- **Domínio Headless**: composição derivada da pilha de Paint pertence ao projeto, tornando comandos de decal completos sem pós-processamento na UI.

### Melhorias de Modelagem, Primitivas, Área de Transferência e Visualização de Viewport (27/09/2026)
- **Ciclo de Vida de Primitivas & Descarte de Card Sem Trava**:
  - Finalização e confirmação automática da sessão de criação de primitivas (`finalize_primitive_session`) ao clicar fora em espaço vazio ou selecionar objetos/elementos no viewport, ao arrastar gizmos ou ao acionar atalhos/ferramentas.
  - Eliminação de sobreposição e bloqueio mútuo de cliques entre o card de propriedades da primitiva e o Operation HUD na viewport.
- **Dissolução e Exclusão Contextual (`Delete` / `Backspace`)**:
  - Em Edit Mode (`Point`, `Edge`, `Face`), acionar `Delete` ou `Backspace` tenta primeiramente a dissolução limpa (`mesh.dissolve_selected()`) para evitar furos na malha; caso a topologia não permita dissolução (ex.: face isolada ou borda aberta), executa a remoção (`mesh.delete_selected()`).
  - Em Object Mode, aciona a exclusão do asset ativo da cena.
- **Área de Transferência Geométrica e Duplicação (`Ctrl+C`, `Ctrl+V`, `Ctrl+D`)**:
  - `Ctrl+C` e `Ctrl+V` em sub-elementos (vértices, arestas, faces selecionadas ou sob hover) copiam a geometria isolada e colam como um novo objeto independente na cena nomeado `{objeto}_part`.
  - `Ctrl+C` e `Ctrl+V` em Object Mode copiam o objeto inteiro e colam uma duplicata (`{objeto}_copy`).
  - `Ctrl+D` / `Shift+D` duplica contextualmente a geometria selecionada dentro da malha ativa em Edit Mode e o asset completo em Object Mode.
- **Opções de Extrusão e Alternância de Modo no Card de Ferramentas**:
  - Exibição de ações de extrusão por Região (`model.extrude`) e Faces Individuais (`model.extrude_individual`) no card de ferramentas.
  - Alternância dinâmica entre Região e Faces Individuais durante o modal interativo preservando o deslocamento atual, além de botões para presets rápidos (+0.2, +0.5, +1.0 e Inverter sinal).
- **Correção de Z-Fighting e Wireframe em Malhas Densas**:
  - Remoção de offsets arbitrários manuais de coordenadas de vértices em Y (`+0.001`), eliminando arestas soltas, quebras e cintilações em superfícies verticais ou anguladas.
  - Aplicação de viés de profundidade canônico em espaço de projeção (`clip.z -= 0.0001 * clip.w`) no vertex shader de linhas (`LINE_WGSL`), mantendo a visualização nítida em qualquer densidade e ângulo.

### Sprint E: Finalização Canônica dos Workspaces MODEL, PAINT e UV (26/09/2026)
- **Workspace MODEL**:
  - **Dissolução Limpa de Topologia (`model.dissolve` / `X`)**: Implementação de dissolução não-destrutiva de faces, arestas e vértices coplanares ou de grau 2/fan, preservando a contiguidade 2-manifold da malha. Registrado no catálogo de comandos e atalhos globais.
  - **Ponte entre Loops e Fronteiras (`model.connect`)**: Algoritmo de conexão e ponte (*Bridge Edge Loops*) entre pares de laços de arestas ou faces desacopladas com suporte a múltiplos segmentos intermediários e torção (*twist*).
  - **Revolução em Torno de Eixos (`model.spin`)**: Ferramenta de *Spin/Lathe* de arestas e perfis selecionados ao redor dos eixos coordenados principais com definição de passos angulares e centro de rotação.
- **Workspace PAINT**:
  - **Pintura de Cor por Vértice (`paint_vertex_color_3d` / P3D-150)**: Suporte completo à pintura 3D de atributos de cor por vértice na geometria com queda esférica cúbica suave (*spherical falloff*), complementando a pintura em texturas e decalques.
  - **Modo de Destino e Máscara de Seleção**: Seleção de alvo entre Textura e Cor de Vértice (`paint_target_vertex`) e alternância de máscara de isolamento por seleção (`paint_mask_selection`), restringindo a área de aplicação das pinceladas às faces ativas.
  - **Ferramenta de Gradiente Linear 2D (`canvas_gradient_linear` / P3D-165)**: Interação de âncora e arraste vetorial em 2D na textura ativa para projeção de gradientes suaves entre a cor primária e secundária da paleta.
- **Workspace UV**:
  - **Costura de Costuras UV (`uv.stitch`)**: Ferramenta de junção e solda de arestas correspondentes separadas no espaço UV (*Stitch Seams*), respeitando vértices ancorados (*pinned*).
  - **Relaxamento de Coordenadas UV (`uv.relax`)**: Algoritmo de suavização Laplaciana iterativa com amortecimento de contorno para redução de distorções geométricas e equalização de estiramento de UV.
  - **Manipulação Tátil Direta no UV Editor**: Suporte a arraste tátil 2D nas ilhas e sub-elementos do editor UV Slint integrado aos atalhos de translação.

### Sprint D: Ergonomia Unificada de Ferramentas, Navegação de Domínio e Medição Viewport (25/09/2026)
- **Filosofia Unificada de Ativação de Ferramentas (1 Toque Card/Gizmo vs. 2 Toques Modo Livre)**:
  - Todas as ferramentas de modelagem (`model.extrude`, `model.inset`, `model.bevel`, `model.push_pull`, `model.extrude_individual`) agora seguem a mesma ergonomia das ferramentas de transformação (`move`, `rotate`, `scale`).
  - 1 toque no atalho ativa a ferramenta com seu card de opções e gizmos de manipulação direta no viewport, mantendo o ponteiro do mouse livre.
  - 2 toques rápidos dentro do intervalo configurável (`double_tap_interval_ms`, padrão 350ms) acionam o **Modo Livre** modal rápido (estilo Blender), no qual o movimento do mouse altera a distância/quantidade interativamente, `LMB`/`Enter` confirma e `RMB`/`Escape` cancela sem sujar a pilha de histórico.
- **Feedback Visual na Viewport & Cursores Contextuais**:
  - Cursor contextual inteligente na viewport adaptado para cada operação (`ns-resize` para extrusão e push/pull, `nesw-resize` para chanfro/bevel e escala, `nwse-resize` para inserção/inset, `grabbing` para rotação, `crosshair` para faca/slice/loop-cut/medição/desenho).
  - HUD Pill flutuante no topo central da viewport indicando a ferramenta ativa, o modo operacional (`[CARD / GIZMO]` vs `[MODO LIVRE]`) e as instruções de interação e cancelamento.
- **Ciclo de Domínio com `Tab` & Remoção de Tecla Obsoleta**:
  - O atalho `Tab` agora alterna perfeitamente entre o modo de Objeto (`Object`) e o último domínio de sub-elemento utilizado (`Point`, `Edge` ou `Face`).
  - Remoção da tecla `0` para seleção de objeto em prol de `4` e `Tab`.
- **Wireframe Overlay Padrão & Tag de Medição de Múltiplas Arestas**:
  - `show_wireframe_overlay` agora é habilitado por padrão em toda a aplicação.
  - Medição de arestas selecionadas: quando múltiplas arestas estão selecionadas, uma única tag flutuante no ponto médio/interseção exibe a média dos comprimentos (`Ø X.XXXm`), evitando poluição visual.
  - Toggle nas Preferências (`multiselection_measure_tag`) com internacionalização em PT-BR e EN.
- **Primitivas Paramétricas Persistentes**:
  - Suporte completo a re-edição paramétrica de primitivas no Inspector após a criação, congelamento transparente em malha estática em operações destrutivas e ação explícita de congelamento com preservação de histórico.

### Sprint C: Decalques 3D Interativos & Textura/Pintura Não-Destrutiva (25/09/2026)
- **Manipulador 3D Interativo de Decalque na Viewport (`petunia_ui_slint`)**:
  - Manipulação direta sobre a malha 3D via projeção baricêntrica reversa (`Mesh::uv_to_world`): o decalque acompanha a geometria da superfície em tempo real.
  - Arraste interativo de posicionamento com raycast sobre as faces da malha ativa (`face_hit_uv`), convertendo as coordenadas de clique em espaço UV paramétrico.
  - Modificadores táteis no viewport: `Shift + Arraste` para escala uniforme proporcional em tempo real e `Ctrl + Arraste` para rotação suave em graus.
  - Suporte completo a cancelamento atômico via `Escape` (revertendo coordenadas de centro, escala e rotação prévias) e histórico transacional de undo/redo (`checkpoint("decal transform")`) ao soltar o clique.
  - Renderização vetorial reativa na viewport Slint (`decal_preview_commands`): retículo central, perímetro subdividido conformado à curvatura de superfícies 3D curvas e indicador direcional de orientação (topo da estampa) em semi-transparência de destaque.
- **Estrutura Não-Destrutiva de Camadas e Texturas (`petunia_project` & `petunia_module_paint`)**:
  - Preservação estrita da pilha de camadas ativa (`Asset.paint_stack` / `DecalLayer`) como autoridade canônica persistente, mantendo o canvas original intacto sem compressões ou perdas raster destrutivas prematuras.
  - Recomposição instantânea em tempo real no cache de textura composto (`Asset.texture`) e sincronização reativa com o material Albedo.
  - Ação explícita de fixação (`Bake Decal to Layer` / `UiIntent::BakeActiveDecal`): a rasterização destrutiva só ocorre mediante solicitação direta e deliberada do usuário ou na exportação final.
- **Seção do Decalque no Inspector Slint & Internacionalização**:
  - Painel de controle no Inspector com campos numéricos para Posição (U, V), Escala (U, V) e Rotação (Deg) sem valores ou rótulos hardcoded.
  - Inclusão dos TextIDs canônicos (`UI_DECAL_TRANSFORM`, `UI_DECAL_POSITION`, `UI_DECAL_SCALE`, `UI_DECAL_ROTATION`, `UI_DECAL_BAKE`, `UI_DECAL_HINT`) nos dicionários oficiais em português brasileiro e inglês (`assets/locales/`).
  - Bateria de testes automatizados unitários e de integração (`test_decal_live_interactive_drag_manipulator_and_preview_commands`, `decal_layer_creation_and_rendering`, `decal_layer_transform_and_bake_workflow`).

### Sprint B: Simple Sweep Completo com Rotation Minimizing Frames (RMF) (25/09/2026)
- **Varredura 3D com Rotation Minimizing Frames (`petunia_mesh::sweep`)**:
  - Implementação de algoritmo de varredura (*Sweep*) baseado na técnica de *Double Reflection RMF* (Wang et al., 2008), garantindo transporte paralelo de 4ª ordem e eliminando singularidades ou torções anômalas (*flipping/gimbal lock*) ao longo de caminhos 3D curvos arbitrários.
  - Compensação contínua de torção angular residual em caminhos guia fechados (*twist compensation*), distribuindo suavemente o ângulo de fechamento ao longo de toda a espinha.
  - Alinhamento de seções transversais ao plano bissetor em cantos vivos (*mitering*) com contenção de esquadria (*miter limit clamping*), prevenindo autointerseções e picos infinitos em quinas agudas.
  - Geração de tampas trianguladas (*end caps*) automáticas em caminhos abertos com perfil fechado via algoritmo de *ear clipping*.
  - Suporte completo a perfis 2D abertos (fitas/superfícies) e fechados (tubos/dutos), incluindo compatibilidade total com perfis ocos (*Wall Thickness*).
- **Extração Flexível de Caminhos Guia & Integração de Modelagem (`petunia_module_model::draw_profile`)**:
  - Reconhecimento automático de cadeias de arestas selecionadas (`selected_edges`) ou vértices selecionados na malha ativa para guiar a extrusão 3D.
  - Fallback automático para curva 3D suave padrão calculada em relação ao plano de visão/câmera ativo quando não houver seleção.
  - Integração da operação `generate_sweep` com histórico transacional de undo/redo e notificações de status localizadas.
- **Integração na Interface Slint & Internacionalização**:
  - Botão de ação *Sweep* integrado no ToolCard da ferramenta Draw Profile na interface Slint.
  - Novas propriedades reativas, callbacks e `TextId::UI_PROFILE_SWEEP` com traduções completas para português brasileiro e inglês (`assets/locales/`).
  - Testes unitários e de integração abrangentes em `petunia_mesh` e `petunia_ui_slint`.

### Sprint A: Draw Profile com Sistema Bézier Completo e Perfis Ocos (25/09/2026)
- **Sistema Bézier Cúbico 2D (`BezierNode`, `BezierPath`)**:
  - Implementação de nós de controle com tipos de curvatura canônicos (`BezierNodeKind::Sharp`, `Smooth`, `Symmetric`, `Free`), suportando alças independentes ou tangentes contínuas ($G^1/C^1$).
  - Algoritmo adaptativo de tesselação baseado na formulação de De Casteljau com critério recursivo de linearidade de corda (*flatness*), garantindo fidelidade geométrica visual sem sobrecarga poligonal.
  - Conversão bidirecional entre SVG Path (`M`, `C`, `L`, `Z`) e representações de curvas 2D.
  - Alternância rápida na interface entre nós retos e nós suavizados (*Smooth Curves* / *Sharp Corners*), com geração automática de alças tangentes proporcionais à distância entre nós vizinhos.
- **Hollow Profile & Wall Thickness (Perfis Ocos com Espessura de Parede)**:
  - Algoritmo de offset de polígonos fechados (`offset_polygon`) com bissetrizes normais e contenção de esquadria (*miter clamping*) a 3× para prevenção de autointerseções em cantos agudos.
  - Geração de perfis de casca oca (`create_hollow_profile`), costurando o laço externo com o laço interno em ordem reversa e gerando geometria 3D oca sólida em extrusão direta (`generate_extrude`) e revolução (`generate_revolve`).
  - Preview interativo em tempo real na viewport Slint (`profile_preview_commands`), projetando simultaneamente o contorno externo e o contorno oco interno durante a edição.
- **Integração Completa na UI Slint**:
  - Novos controles táteis no ToolCard de Draw Profile: campos numéricos para *Wall Thickness* e *Smoothness*, botão dinâmico para suavização e cantos retos.
  - TextIDs e traduções completas em inglês e português brasileiro sem strings hardcoded (`UI_PROFILE_WALL_THICKNESS`, `UI_PROFILE_SMOOTH_CURVES`, `UI_PROFILE_SHARP_CORNERS`, `UI_PROFILE_SMOOTHNESS`).
  - Bateria de testes unitários automatizados validando tesselação Bézier, offset de polígonos, perfis ocos e integração no shell Slint.

### Acessibilidade de Tooltips Ricos Dinâmicos e Rollover de Menus da Barra Superior (25/09/2026)
- **Atalhos Dinâmicos Derivados do Keymap Ativo (`ShortcutsModel` & `ShortcutsEntry`)**:
  - Eliminação de atalhos físicos hardcoded na interface Slint. O modelo reativo `ShortcutsModel` extrai dinamicamente todas as combinações de teclas a partir de `state.ui.keybinds` (`keybinds.format_shortcut(...)`).
  - Qualquer personalização feita nas configurações de atalhos ou troca de perfil de keymap (ex: Blender, Maya, Petunia Padrão, Notebook) reflete-se **instantaneamente** nas badges `<kbd>` dos tooltips em todos os painéis e ferramentas.
- **Tooltips Ricos Modernos (`RichTooltip`)**: Implementação de componente universal com cabeçalho em destaque, badge estético estilizado em fonte mono para teclas de atalho (`shortcut`), micro-explicação contextual em texto secundário (`description`) e rodapé com dicas de interação e gestos (`hint`), seguindo as melhores práticas e padrões de UX de softwares como Blender, VS Code e Figma.
- **Cobertura Completa em Toda a Interface**: Adicionados rótulos, atalhos físicos e explicações detalhadas em:
  - Ferramentas de modelagem, seleção e medição do painel esquerdo (Select `W`, Lasso `Shift+Space`, 3D Cursor `Shift+RMB`, Measure `M`, Move `G`, Rotate `R`, Scale `S`, Transform `T`, Add Primitive `Shift+A`, Cube, Sphere).
  - Ferramentas de pintura (Brush `B`, Eraser `E`, Color Picker `I`, Fill `G`, Line `D`, Rectangle `U`, Airbrush `Shift+B`) e UV (UV Select `W`, Unwrap `U`, Pack Islands `Ctrl+P`, Project Ref `Alt+U`).
  - Ações do topo (Undo `Ctrl+Z`, Redo `Ctrl+Shift+Z`, Parts `F4`, Open Project `Ctrl+O`, Save `Ctrl+S`, Search `Ctrl+P`, Settings `Ctrl+,`).
  - Seletor de workspace central (MODEL `F1`, PAINT `F2`, UV `F3`).
  - Pílulas flutuantes da viewport (Projection `Numpad 5`, Frame `Numpad .`, Reset View `Home`, Wireframe `Shift+Z`, Solid `Z`, Material `Alt+Z`, Rendered `Ctrl+Alt+Z`, X-Ray `Alt+Z`, Shading Options, Pivot `.`, Snap `Shift+Tab`, Proportional Editing `O`).
  - Domínios de seleção independentes (Object `0`, Point `1`, Edge `2`, Face `3`).
  - Primitivas 3D da galeria (Cube, Sphere, Cylinder, Plane, Cone, Capsule, Torus, Circle, Icosphere, Wedge) e ferramentas da barra inferior (Duplicate, Extrude, Push/Pull, Inset, Bevel, Knife, Loop Cut, Profile Revolve, Subdivide, Merge Center, Slice, Delete).
  - Campos numéricos e eixos X, Y, Z (`NumericField`, `Vector3Field`) com instruções táteis de arraste (scrubbing), precisão via `Shift` e edição por teclado.
- **Rollover Fluido nos Menus do Topo**: Ao abrir qualquer menu suspenso ("File", "Edit", "View", "Window"), sobrepor o mouse sobre qualquer outro menu fecha automaticamente o menu anterior e abre o novo menu de forma imediata (sem cliques adicionais), replicando o comportamento de desktop moderno do macOS, Windows, VS Code e Blender.

### Ergonomia de Ferramentas, Atalhos e Precisão MODEL/PAINT/UV (25/09/2026, Sprints 1–3)
- Atalhos universais de domínio de seleção `1` (Point), `2` (Edge), `3` (Face) e `4`/`0` (Object) no workspace MODEL com alternância direta e sem atritos.
- Ajuste interativo do número de cortes de Loop Cut através da roda do mouse (gesture Zoom) no estado de preview e suspensão contextual de navegação.
- Ferramenta Slice com restrição angular em incrementos de 15° ao segurar `Ctrl`/snap e indicador direcional de seta perpendicular no preview visual demonstrando o lado preservado vs ceifado durante o modo Trim.
- Modo Edit Pivot persistente com atalhos `D` e `Insert`, card dedicado no HUD da viewport ("Edit Pivot Mode") e saída limpa via `Escape`.
- Alternância de costura UV com atalho direto `U` para arestas selecionadas ou dentro do workspace UV.
- Preenchimento de ilhas UV (`FillScope::UvIsland`) com sangria de textura (UV Bleed / dilation de 2px) em direção a pixels adjacentes transparentes, prevenindo frestas não texturizadas nas costuras em 3D.
- Diagnóstico de orientação no UV Checkerboard com faixas de destaque no eixo U (quente/avermelhado) e no eixo V (frio/azulado) no WGPU e no renderizador software.
- Mesclagem de camadas de pintura ("Merge Down") no workspace PAINT com composição determinística de Raster, Decalques projetados e Efeitos, botão dedicado na UI Slint e seleção ativa atualizada.
- Perfil 2D (Draw Profile) com ângulo de revolução (Lathe/Revolve) customizável (`Mesh::revolve_angle`) para geração de setores parciais com fechamento automático de tampas de extremidade e snap magnético ampliado para 0.25.
- Operações Booleanas (Fuse, Cut, Intersect) com inferência automática de operando secundário quando o projeto possui exatamente dois objetos.
- Overlay visual do raio de influência da Edição Proporcional (Soft Selection): círculo pontilhado projetado em tempo real na viewport (`compute_proportional_circle`) durante transformações modais, atualizando dinamicamente com o redimensionamento por roda do mouse.
- Ferramenta Round Edge (Bevel) com opção de Clamp Overlap (`bevel_clamp_overlap`): previne autointerseções e achatamentos ao limitar a distância do chanfro à metade da aresta adjacente mais curta, com alternância no ToolCard da viewport.
- Pintura 3D com descarte de polígonos invertidos (Backface / Angle Culling): algoritmo `pick_face_hit_cull` e `ray_triangle_cull` desconsiderando normais voltadas para longe da câmera, impedindo que o pincel 3D vaze ou suje superfícies opostas.
- Equalização da densidade de texels nas ilhas UV (`Mesh::equalize_texel_density`): normaliza as escalas das ilhas em torno de seus centróides UV com base na média geométrica da densidade global ($\sqrt{\sum A_{UV} / \sum A_{3D}}$), com ação direta no painel de ferramentas UV.
- Controle de Dureza do Pincel (Brush Hardness): ajuste contínuo de 0 a 100% no painel de pintura com atalhos `[` / `]` (tamanho) e `Shift+[` / `Shift+]` (dureza), permitindo transições ultra-suaves de gradiente ou bordas sólidas nítidas.
- Deslocamento e Nudge por Setas do Teclado (`Left`, `Right`, `Up`, `Down`): movimentação milimétrica de elementos selecionados no modo Edição, do objeto no modo Objeto e de faces no editor UV, com passo reduzido a 0.01 via `Shift` e snap de 1.0 via `Ctrl`.
- Chanfro por Vértice / Ponto (*Vertex Bevel*): algoritmo transacional `bevel_vertex` em malhas 2-manifold fechadas substituindo o vértice de canto por vértices de corte ao longo de arestas incidentes e costurando uma face poligonal de tampa (cap), com suporte completo em `BevelCmd`, no modal interativo de Bevel e botão toggle "Point (Vertex) Mode" nas opções de Round Edge da viewport.
- Chanfro de Múltiplos Elementos em Lote (*Multi-edge / Multi-vertex Bevel Batch*): capacidade de chanfrar simultaneamente conjuntos de arestas disjuntas/paralelas ou múltiplos vértices em uma única transação atômica (`Mesh::bevel_selected_full` e `Mesh::bevel_selected_vertex`), com rastreamento geométrico 3D de coordenadas, tolerância no modal (`ModalKind::Bevel` para $\ge 1$ vértices) e rollback garantido sem geometria intermediária corrompida em caso de falha de adjacência.
- Pintura Simétrica em Tempo Real (*Symmetry / Mirror Painting*): espelhamento de pinceladas, carimbos e borracha ao longo dos eixos X, Y e Z no espaço 3D (via projeção tridimensional e busca baricêntrica de coordenadas UV opostas em `PaintModule::paint_mesh_3d_with_settings`) e no canvas 2D de textura (`canvas_brush_with_symmetry`), com seletores visuais táteis X/Y/Z no painel de ferramentas PAINT da interface Slint.
- Fixação de Vértices UV (*UV Vertex Pinning*): ancoragem atômica de vértices e cantos de coordenadas UV (`Mesh::uv_pinned`), preservando coordenadas originais imutáveis durante transformações de translação, rotação e escala no espaço UV (`UvModule`) e projeções de mapa UV (Planar e Cube), com visualização de marcadores em forma de diamante no editor UV da UI Slint, botões de ação Pin/Clear e atalhos rápidos `P` (alternar fixação de faces/cantos selecionados) e `Alt+P` (limpar todas as fixações).
- Cortes de Anel Duplamente Balanceados (*Dual Balanced Loop Cuts*): modo de distribuição simétrica (`LoopDistribution::Balanced`) que posiciona pares de cortes simetricamente em torno do ponto médio (0.5) do anel de quads, com deslize interativo que expande ou pinça os pares de forma espelhada. Mínimo de 2 cortes forçado ao ativar o modo; funciona com qualquer número de cortes ≥ 2. Toggle dedicado "Dual Balanced" no ToolCard de Loop Cut da viewport Slint, com suporte completo em `LoopCutCmd`, `CutSession` e na cadeia de preview/commit do `SlintUiBridge`.
- Transferidor Visual de Ângulo (*Protractor / Angle Arc Overlay*): projeção em tempo real na viewport (`compute_protractor`) de um mostrador circular e setor fatiado com marcações radiais a cada 15° e 45° durante a rotação modal (`ModalKind::Rotate`), exibindo a varredura angular e facilitando o alinhamento visual de rotações.
- Desbloqueio e retomada da documentação contínua: sincronização automática do site VitePress e do caderno canônico (`docs/bible/`) via xtask sem travas de lockfile estático.

### Módulos independentes do Inspector: ancorar, flutuar e fixar (24/09/2026, ADR 005)
- Cada módulo do Inspector direito (Parts, Transform, Material, Object, Modifiers, Quick Actions) tem estado próprio: ancorado ou flutuante, posição, "manter aberto" e asset fixado, com persistência individual por módulo.
- Card flutuante arrastável dentro da viewport (arrasto pelo header, clamp nas bordas), com camada de cards sobre o canvas — sem janelas do sistema operacional, conforme a V1.
- Pin duplo: "manter aberto" sobrevive ao recolher-tudo e ao recolhimento do painel; "fixar asset" fixa a seção a um asset e redireciona leitura **e** escrita (Material, Object e Modifiers operam no asset exibido, não no ativo).
- Corpo de cada seção definido uma única vez e instanciado na coluna ancorada e no card flutuante; sem duplicação de markup ou callbacks.
- Afordances de dock, arraste e pin localizadas via `TextId` (en/pt-BR).

### Decal 3D Projection, Layer Baking & Modal Tool Feedback (24/09/2026, Master MVP)
- Manipulação paramétrica de decalques de pintura: suporte a transformação no espaço UV (`PaintLayerStack::set_decal_transform`) e rasterização destrutiva controlada (`PaintLayerStack::bake_decal_to_raster`).
- Comandos transacionais de domínio `SetDecalTransformCmd` e `BakeDecalCmd` com histórico de desfazer/refazer completo no dispatcher de comandos.
- Controles de UI para decalques no painel de camadas do workspace PAINT (Center U/V, Scale U/V, Rotation em graus e botão de Bake para textura raster).
- Feedback visual de ferramentas modais (`ToolFeedback`) conectado ao enlaçamento de arrasto (`compute_drag_link`) com linhas pontilhadas de projeção e indicação de snap magnético no HUD.
- Clamping de posicionamento de cards flutuantes (`FloatSectionCard`) vinculado às dimensões da janela e árvore de acessibilidade Slint com semântica padronizada `groupbox`.

### Outliner / Parts Hierarchy & UV Editor Seams (24/09/2026, Master MVP)
- Reordenação transacional de assets na hierarquia (`ReorderAssetCmd`, `Project::reorder_asset`) com persistência da seleção ativa e suporte a desfazer/refazer.
- Ações completas no menu de contexto de peças/assets: Isolar / Desisolar (`scene.isolate` / `scene.un_isolate`), Mover para Cima (`scene.move_up`) e Mover para Baixo (`scene.move_down`).
- Exibição direta da densidade da malha em cada item de Parts (`{verts}v · {tris}t`).
- Sincronização visual bidirecional do editor UV: arestas de costura (`uv_seams`) e faces selecionadas são projetadas em subcaminhos dedicados para destaque no canvas Slint.

### Revisão do Inspector MODEL (24/09/2026, ADR 004)
- Painel e rail colapsado com fundo translúcido (viewport visível através); seções mantêm fundo opaco para legibilidade.
- Alça de recolhimento revelada por proximidade na borda do painel; header com "Inspetor"/seta/"MODEL" removido; respiro maior na borda direita.
- Rail colapsado em 5 pílulas de ícone espaçadas com hover e Tooltip; títulos de seção via `TextId`.
- Material no fluxo do legado (slot + Assign/New/Duplicate/Remove antes de editar); Object sem microlinha; Quick Actions em seção própria.
- Modifiers no padrão Blender (Add no topo, toggle Monitor, ChevronUp/Down + X, top-aplica-primeiro).
- Tool Options + Operation HUD unificados num card único dentro da viewport (topo-esquerda), oculto em repouso.

### Fechamento V1 MODEL/PAINT — Sprint 4 (24/09/2026)
- Proportional Editing fim a fim: toggle `O`, raio pela roda, falloff selecionável e kernel aplicado no commit modal.
- Snap magnético nos gestos modais (Grid, Increment, Vertex, Edge, Face), toggle `Shift+Tab` e seletor de alvo na viewport.
- Profile com presets Retângulo/Círculo; UV com botões dedicados de 90° CW/CCW; Paint com botão Add Decal (posicionador 3D segue Era 1).
- Overlays Face Orientation (frente azul/verso vermelho) e UV Checkerboard no WGPU e no fallback software; backend GL legado não renderiza os overlays.
- Lasso com filtro de oclusão verificado e pivot Individual Origins verificado em objeto e componente; matriz V1 atualizada (MODEL 43/45, PAINT 27/30).

### Interação de modelagem MODEL (23/09/2026)
- Loop Cut na UI Slint agora arma sem seleção prévia, encontra edge rings válidos por hover, desenha preview no viewport e ajusta Cuts pela roda sem acionar zoom; click inicia a sessão transacional e Enter/Esc confirma/cancela.
- Orbit, pan e zoom da viewport ficam suspensos durante Loop Cut, Slice, Profile e ferramentas modais com captura do mouse.
- Shelf MODEL expõe Push/Pull, Slice, Loop Cut e Profile; Profile oferece prévia, fechamento, profundidade, geração por extrusão e revolve.
- Selector de pivot localizado expõe Median Point, Bounding Box Center, 3D Cursor e Individual Origins.
- Slice passa a manter os dois lados da malha sem cap por padrão, conforme o contrato canônico.

### Implementado — MODEL Inspector e viewport (23/09/2026)
- Baseline MODEL revisada com autorização do produto: `Parts → Transform → Material → Object` no Inspector direito, com cabeçalho fixo, corpo rolável, seções recolhíveis e acesso compacto à direita. ADR 003 e matriz de gaps registram os limites.
- Parts agrupa busca, filtro, ordenação e contagem numa faixa; o ajuste de altura da linha atualiza a lista ao vivo nos layouts normal e compacto. Seções do Inspector usam superfície elevada e borda semântica mais visível; teste headless cobre a interação e os dois layouts.
- Select combina clique/caixa e ganhou Lasso; a seleção por região detecta arestas cruzadas sem depender do ponto médio. Quatro ferramentas de transformação incluem gizmo combinado com handles próprios de mover, girar e escalar.
- Ícones circulares de shading, overlay de wire independente do modo Wireframe, feedback de seleção Point/Edge mais legível e silhueta de esfera corrigida. Face deixou de exibir pontos centrais que poluíam a leitura.
- Menus do topo alinham-se à esquerda e fecham ao perder foco; o gesto modal por atalho aceita movimento imediato e não deixa o clique de confirmação selecionar o objeto subjacente.
- Parâmetros pós-criação das primitivas, cobertura completa de tooltips e paridade premium de viewport **ainda não estão concluídos**; ver `docs/development/model-inspector-viewport-gap-matrix.md`.

### Planejado — arquitetura de Context e workspaces (23/09/2026)
- Plano incremental de shell rolável e seções compartilhadas, seguido por hierarquia MODEL e deduplicação das barras da viewport; PAINT/UV ficam documentados para fase posterior, sujeitos ao Livro Vivo.

### Corrigido — integração Paint/UV e viewport (23/09/2026)
- A projeção de tinta 3D e o picking UV usam triangulação correta de polígonos côncavos; o Paint não alcança o vazio da concavidade.
- A cor escolhida no Slint chega ao pincel real; o conta-gotas 3D amostra a textura composta e mantém os dois caminhos de cor sincronizados.
- Shift e clique vazio no editor UV mantêm a seleção de UV e a seleção 3D coerentes; transformações UV sem alvo não alteram toda a malha.
- A pill de pré-seleção deixa de cobrir a viewport inteira; no layout compacto, o Inspector oculto deixa de reservar 290 px vazios.
- A suíte Slint voltou a passar (169/169) com provas alinhadas à seleção por domínio, ao Cut multissegmento e ao encurtamento visual dos eixos projetados. Paint (22/22) e UV (4/4) também passaram; aceite visual ainda pendente.

### Adicionado
- **Frontend Slint (`petunia_ui_slint`)**: nova crate no workspace implementando o shell declarativo moderno em Slint 1.18 (`crates/ui-slint/`).
  - Top bar com alternador de domínio de seleção (`Object`, `Point`, `Edge`, `Face`) alinhado ao vocabulário canônico (AGENTS.md §3).
  - Icon Rail para seleção de workspaces `MODEL`, `PAINT` e `UV`.
  - Tool Shelf contextual vertical (44px) integrada com ações de edição e pintura.
  - Viewport central reativo com render off-screen WGPU (`viewport_gpu.rs`) e software viewport fallback (`viewport_soft.rs`) para compatibilidade universal (ex.: GPUs antigas / Ivy Bridge Mesa).
  - Context Inspector adaptado por workspace (Transform com scrubbing e fine-stepping numérico, contagem de polígonos, paleta de pintura e camadas, UV unwrapping e densidade).
  - Scene Outliner hierárquico lateral e gaveta inferior expansível para Asset Library.
  - Command Palette modal com busca difusa e atalhos globais de teclado (`Ctrl+Z`, `Ctrl+Y`, `Ctrl+S`, `Ctrl+O`, `Ctrl+K`, `Shift+D`, `Tab`, `1`/`2`/`3`/`4`).
  - Diálogos de arquivo assíncronos (`files.rs`) com RFD para salvar e abrir projetos `.petunia`.
  - Sincronização dinâmica de tokens com o `ThemeRegistry` (`petunia-dark` e `petunia-high-contrast`).
  - Suporte completo de a11y via AccessKit.
  - Crate legada `petunia_ui` preservada e acessível via flag `--legacy-egui` ou env `PETUNIA_LEGACY_EGUI=1`.

## [Unreleased] — Iniciativa Paint (decisões canônicas 2026-09-16)

### Documentação — emenda canônica no Livro Vivo
- **Paint Workspace (P3D-055)**: registrada a decisão de redesenhar o Paint como "mini Photoshop" dentro do shell congelado do cap. 36 — canvas 2D central + prévia 3D, painel direito com Layers (drag-and-drop), Brush e Effects; `module-uv` preservado como utilitário de "Preparar superfície" (fluxo paint-first, P3D-065).
- **BrushSettings unificado**: novo descriptor único (`size_px` em pixels de tela, `hardness`, `strength`, `flow`, `spacing`) substituindo a dualidade `canvas_brush` (px) × `paint_radius` (metros); novo pincel **Airbrush** (aditivo contínuo). Stroke engine baseado em dabs (P3D-056, P3D-057).
- **Effect Stack (P3D-134)**: UX de presets "Add Effect" na pilha de camadas; nodes iniciais consolidados — Pixelate/Posterize/Invert (já no modelo) + Grain/Noise, Levels/Threshold, Brightness/Contrast, Hue/Saturation (lista do cap. 42, a implementar).
- **Surface Recipe graph (P3D-113)**: modelo de dados começa headless (DAG, sockets, avaliador determinístico, cache); editor visual de nodes adiado para o ciclo pós-Paint.
- **UV Workspace congelado (P3D-063/064/065)**: edição UV sai da UI V1 durante a iniciativa Paint (conflito de `f.selected` com máscaras P3D-132; P3D-063 já exigia design conjunto com Materials/Paint); redesign completo marcado para pós-Paint. Shell `MODEL / PAINT` preservado; capítulo 36 não reaberto.
- **Branches de implementação**: `paint/core-engine` (motor, descriptors, effects, graph headless) e `paint/ui-redesign` (layout mini-Photoshop, painel Layers/Brush/Effects).
- **Drift conhecido no mapa de componentes**: `docs/public/ui-map.json` (site congelado) aponta sete nós para símbolos extintos desde o commit base `cbc267c` (não é deste trabalho); `docs-check` para nesse passo e a decisão de descongelar está registrada em `docs/audits/paint/00-ui-map-drift.md`.
- **Integração no `main`**: as branches de Paint entraram em `main` por fast-forward (`cbc267c` → `d8bbf57`) — `paint/core-engine` já era ancestral e o commit do redesenho de UI veio junto. Na árvore integrada a suíte completa passa: `cargo test --workspace` (61 alvos, 789 testes, 0 falhas), `clippy --workspace --all-targets -D warnings`, `fmt --check`, `arch-check`, `ui-guard --strict` e `bible-check`; só `docs-check` segue vermelho no passo do mapa congelado.
- **Todas as branches ativas no `main`**: a onda 7 do ecossistema egui (`ui/ecosystem-final-push`, `bd2ecad`) também passa a ser ancestral de `main` (`30cdbae`). O conteúdo de código dela já estava integrado desde o snapshot `607a362` que a branch de Paint trazia, então o merge não muda um byte da árvore — ela sai idêntica a `7085ee5`, já validada; o único conflito foi nos arquivos gerados (`docs/generated/TEXT_TOKENS.md`, `manifest.json`), resolvido com o lado do `main` (589 chaves, número conferido contra `assets/locales/{en,pt-BR}.toml`).

### Implementado — workspace PAINT (layout mini Photoshop)

O módulo de pintura deixou de ser "um painel na coluna da direita" e passou a ser uma **superfície fatiada pelas regiões do shell** (`workspaces::WorkspaceLayoutProfile` diz o que cada região mostra).

- **Ferramentas na coluna esquerda**: os oito pincéis (Pixel, Soft, Airbrush, Eraser, Line, Rectangle, Fill, Eyedropper) saem do painel de propriedades e viram a paleta do workspace, em três grupos (Pincéis · Formas · Amostra) desenhados pela mesma grade do adapter do Model.
- **Centro lado a lado**: viewport 3D à esquerda e tela 2D à direita, com **divisória arrastável** — a tela 2D é o paine `PetuniaPane::PaintCanvas` do `adapters::tile_layout` (o produto não compõe mais o centro com `egui::Panel`), então a divisória, os mínimos das duas superfícies (a viewport nunca cede abaixo de `MIN_VIEWPORT_WIDTH`; numa janela estreita quem cede é a tela) e a largura persistida saem do adapter, junto com o resto do macro-layout; o produto só declara o perfil do workspace. As duas visíveis ao mesmo tempo — pinta-se no 2D vendo o resultado no 3D — e a barra de contexto continua sendo chrome da coluna 3D. A tela tem xadrez de transparência, grade de pixels contextual, traço interpolado por `stroke_dabs`, balde e conta-gotas no clique.
- **Pincel em cartão flutuante (§34)**: o novo adapter `adapters::popup::PetuniaPopup` cria a superfície flutuante com o contrato que faltava — **perde o foco no clique fora e no Escape**, sem exigir Enter/Espaço. Dois modos: `Panel` (recolhe para o cabeçalho e volta pelo chevron) e `Menu` (fecha). O cartão do pincel usa `Panel`: recolhe quando o usuário toca a tela/canvas, e a paleta o reabre ao escolher a ferramenta (`PetuniaPopup::reveal`). O corpo vem do módulo de pintura — não há segunda cópia dos controles.
- **Dock do PAINT = Camadas (topo) + Inspector (base)**: as seções passam a ter o rótulo do conteúdo do workspace, e a árvore de objetos (Outliner) vira uma seção *Scene* recolhível dentro da lista de camadas — uma rolagem só para as duas.
- **Camadas como camadas de pintura**: modo de mistura (Normal/Multiply/Add/Screen) e opacidade da camada ativa em um bloco de propriedades, lista reordenável por arrasto (`PetuniaDragList`, sem setas ↑/↓), olho por linha, duplicar (com id novo) e resumo inline quando opacidade/mistura saem do padrão; os parâmetros do efeito aparecem só para a camada ativa.
- **Hierarquia de cor**: em cima a **cor atual** (um seletor, com o hex ao lado), embaixo a **paleta curada**, com `+`/`Remover` explícitos. Mudar a cor não escreve na paleta — antes cada quadro com o seletor aberto empurrava um tom intermediário (`changed()` dispara a cada arrasto) e a paleta virava um borrão de tons que o usuário nunca escolheu; os botões da paleta são botões de cor, não um seletor em cascata por cor.
- **Preparar superfície**: as projeções (Planar · Box · Auto Unwrap) ficam no cartão de pincel, no lugar do editor UV que a V1 aposentou; o workspace UV legado cai na mesma composição, sem editor interativo.
- **Traço e preview coerentes**: o anel na viewport é a pegada real do dab (`BrushPreviewStyle` + `brush_world_radius`) e o arrasto com F ajusta o **tamanho em pixels** — o mesmo número que o carimbo usa.
- **Atalhos por keymap**: `[`/`]` ajustam tamanho e `Shift+[`/`Shift+]` a dureza; nada de tecla física no código de UI.

## [Unreleased] — UI/UX, Responsiveness, Performance & Architecture Remediation (Waves 0–9)

### Adicionado
- **DnD, motion e validação de formulário (Wave 7 · §49/§51)**: três crates entram no grafo com path real — `egui_dnd` (lista reordenável), `egui_animation` (motion) e `egui_form` (validação por campo), todas confinadas em adapter/foundation e cobertas por `ui-guard`.
- **Ícones 100% Petunia**: toda a interface volta ao estilo próprio (`icons.rs`) — o pacote padrão renderiza a arte vetorial Petunia e ela passa a vencer o glifo genérico na cadeia de resolução; 13 desenhos novos no mesmo estilo (brush/eraser/bucket/picker/linha/retângulo, pin, step/jump do transporte, perspectiva/ortográfica) e teste de cobertura que garante arte Petunia (vetor ou PNG) para todo `PetuniaIcon`; pacotes `iconflow` continuam opcionais nas Preferências (o desenho muda, o significado nunca).

### Alterado
- **Reordenar a paleta por arrasto (Wave 7 · §49)**: o menu de configuração da toolbar troca as setas ↑/↓ por um grip arrastável (`PetuniaDragList`); a ordem é aplicada no drop e gravada por um escritor único (`apply_toolbar_config`), sem `mark_dirty` por frame. O grip é a área de aquisição do gesto: o motor só inicia o arrasto com o ponteiro sobre ele (ou após 250ms de pressão) — medido e registrado no ledger.
- **Motion com backend real (Wave 7 · §49)**: `foundation::motion` passa a interpolar (`reveal`, `animate`, `position`, `section`) sobre `egui_animation`, com `seconds()` como única conversão de unidade — passar milissegundos deixava a transição ~1000× mais lenta em silêncio; um teste de taxa falha se a unidade regredir. Primeiro path: a busca inline do Outliner e do Inspector **abre** animando a altura. Com `animation_time == 0` no tema tudo vira troca instantânea.
- **Validação de keymap pelo contrato (Wave 7 · §51)**: a aba de atalhos de Settings deixa de desenhar o banner de conflitos à mão (3 cores literais) e passa a usar `PetuniaValidationReport` + `PetuniaFormSession` + `error_summary_titled`: erro por campo após o gesto de confirmar (`reveal_errors`), resumo inline sempre, tokens semânticos novos `ACCENT_ERROR`/`ACCENT_ERROR_SOFT`, e i18n de título/mensagem (`keymap.conflict_title`, `keymap.conflict_shared`).

### Corrigido & Aprimorado
- **Paleta de ferramentas e menu de família (Wave 6 · §48)**: a paleta deixa de decidir o próprio arranjo com breakpoints (`available_width() < 90` / `>= 100`) — ela declara a célula só-ícone e a célula que comporta o rótulo (`PetuniaToolGridSpec`) e recebe a largura resolvida de cada célula (`PetuniaToolbarButton::width`). No caminho, um defeito real: o grupo split (Seleção/Transformação) desenhava a seta do menu da família **fora do paine**, invisível e inalcançável pelo mouse, porque a coluna de 48px só comportava o ícone; o arranjo virou `plan_group(cell_width)` (com o menu também no clique secundário) e o mínimo da coluna passou a ser derivado do controle (ícone + vão + seta + moldura = 74px), coberto por `tests/toolbar_fit.rs`.
- **Settings e Reference Manager (Wave 6 · §48)**: contrato `PetuniaForm` (`field`, `toggle`, `section`) substitui rótulo+controle montados à mão e os `size(13.0)`/`11.5` repetidos por aba; o gerenciador de referências troca `grid_columns`/`grid_card_width` por `PetuniaColumnSpec` e o breakpoint de 560px por uma esteira flex com `SpaceBetween`. `available_width` de produto caiu de 18 para 14 e `spacing_mut` de 48 para 37.
- **Pintura — UI e arte**: pincéis em linhas de largura total com `PetuniaToolbarButton` (ícone dentro da seleção; o `horizontal` aninhado em `horizontal_wrapped` escondia Linha/Retângulo); ícones de pintura desenhados na arte Petunia; i18n completo do painel (paleta, pincéis, canais, máscara, grade, nome de camada e status) com paridade en/pt-BR; canais PBR nomeados nos dois locales.
- **Render GL**: textura de asset preta — `let h = fnv1a(...)` sombreava a altura e o `tex_sub_image_2d` subia com altura-lixo; upload agora via `tex_image_2d` com dados após o hash renomeado.
- **Toolbar**: PNGs transparentes voltam a ser a arte canônica das ferramentas (os SVGs Figma carregam fundo escuro embutido, ilegíveis a 20px).
- **Settings — Ícones**: aba localizada (título, descrição, badges, botão e exemplos) com nomes/descrições de pacote traduzíveis por id e fallback para o manifest.
- **Nav gizmo**: o toggle de projeção usa o ícone real de perspectiva/ortográfica em vez de um quadrado genérico.
- **Renderers por revisão (Wave 1)**: buffers WGPU/OpenGL só reconstróem com mudança de geometria (fingerprint de cena); órbita de câmera atualiza só uniforms; VBOs GL persistentes por asset; sem create/delete por draw; contadores de telemetria e escopos puffin.
- **Dock direito dividido (Wave 2)**: Outliner e Inspector independentes com divisor arrastável persistente, colapso independente e `UiRegions` como fonte única de retângulos do shell.
- **Workspaces reais (Wave 3)**: perfis de composição Model/Paint/UV/Animate (paletas, centro split no UV, Timeline no Animate), transição com memória de layout e paleta de pose no Animate.
- **Shelf responsiva (Wave 4)**: comandos com prioridade, medição por galley, cápsula exata, modos Full/Compact/Overflow/Pill; menus com largura de conteúdo; ComboBoxes com larguras locais.
- **Reference Manager e Settings (Wave 5)**: grade determinística, ajuste fino colapsável, `modal_sizes` seguro, texturas sem leak, abas Interface e Import/Export sem placebo.
- **Ícones convergidos (Wave 6)**: glifos iconflow reais por pacote para utilitários, arte Petunia para ferramentas, `PetuniaIconButton` canônico, emoji erradicado do código UI.
- **i18n e a11y (Wave 7)**: `TextId` tipado, paridade en/pt-BR em CI, pseudo-locale, switch com redesenho, locales embutidos de reserva, foco por Tab, matriz de escala.
- **Criação de primitivas (Wave 8)**: sessão `PrimitiveCreationSession` com cartão Last Operation (6 espécies, regeneração ao vivo, transação única de undo, Esc cancela).
- **Convergência (Wave 9)**: removidos `tiles_workspace.rs`, `app_icons.rs` e PNGs inalcançáveis da toolbar; pilots P1 com gate (`flex_layout`, `inbox_bridge`) mantidos com veredito no ledger.
- **Primitivas V1 (gauntlet dedicado)**: dez espécies validadas (Cube/Box W/H/D, Plane, Wedge, Cylinder/Cone via gerador frustum com tampas, Circle/Disc, Torus, UV Sphere com polos soldados, Icosphere 0–3, Capsule parametrizada); defaults low-poly; menu Add agrupado; sessão + cartão com Reset e estatísticas; i18n en/pt-BR completa das superfícies; pipeline save/load/OBJ/GLB por espécie.
- **Mapa da interface**: `docs/public/ui-map.json` (195 nós: componentes, posições, interdependências) + `docs/developers/ui-component-map.md`, validados por `cargo xtask ui-check` dentro do `docs-check` (versão anterior congelada em `ui-component-map.legacy.md`).
- **Modelagem — gaps funcionais/a11y/i18n**: `symmetrize` (copia +eixo/−eixo com solda, `SymmetrizeCmd`/`SymmetrizeTool`), merge by distance (`WeldCmd` + slider log), Tool tab com estados desabilitados e motivos, anéis de foco + `widget_info` nos controles custom (domínio, snap/prop, overlays, X-Ray, sombreamento, pills), tooltips de sombreamento e toggles i18n en/pt-BR, Slice XYZ e revolve com hint de perfil aberto + nome `Revolve` sem `360°` fixo.
- **Shell — header/dock/responsivo**: header sem brand com pills centralizadas (3 colunas); `PetuniaMenuButton` com seta vetorial no lugar dos glifos `▾`/`›`/`↑`/`↓`/`✕`/`✓`/`●`/`│` (tofu em fontes sem bloco geométrico); viewport bar responsiva (overflow sob a seta + 2 linhas); status bar em 3 zonas rígidas + correção da invasão do asset browser (ScrollArea limitada com `FOOTER_RESERVE`, teste `shell_layout_tests`); toolbar configurável (ordem/visibilidade/1–2 colunas, mirror/merge/symmetrize, resets); dock com lado E/D + empilhado/lado a lado, rail vertical de abas e inspector em cartões ECS com Add Component (só destinos reais); avaliação de crates externos documentada (sem dependências novas).
- **Ecossistema egui — Final Push wave 5 (Top Bar e macro-layout)**: a Top Bar deixa de fingir centralização com `ui.columns(3, …)` — o produto declara três zonas e o `adapters::top_bar` mede cada faixa por sonda, decide o overflow pelo rank e distribui em três caixas (laterais de largura igual ⇒ centro geométrico; fallback `Drifting` declarado quando a esquerda não cabe na metade). No caminho, `plan_row` passou a derrubar a faixa de menor rank **até caber** (antes um guloso item a item escondia rank alto e mostrava rank baixo com larguras diferentes) e a seta de acesso só aparece com algo oculto. O shell ganhou `PetuniaLayoutAdapter` + `egui_tiles` 0.17.1 (`adapters::tile_layout`): `PetuniaShellLayout` como contrato durável (nunca a árvore da crate), cinco painéis com slot autorizado, nenhum fechável/arrastável (sem docking livre), mínimo de viewport aplicado antes da árvore e larguras devolvidas em pixels para o produto persistir. Duas armadilhas do `egui_tiles` medidas: shares são normalizadas pela soma (dock de 300px media 231.8px) e um `Panel` do egui 0.36 funciona dentro do `Ui` de um tile (o que torna a migração do shell incremental). A troca dos painéis de região pela árvore fica para a Wave 5b, declarada em `docs/audits/ui-ecosystem-final-push/04-wave5-topbar-shell.md`.
- **Ecossistema egui — Final Push waves 1–4 (confinamento e contratos)**: `ui-guard` (§36/§37, 17 regras, `--strict` na CI) mede product code × foundation/adapter; `foundation/` e `adapters/` como única superfície das crates auxiliares; inspector sem breakpoints (`columns` + `PetuniaColumnSpec`, `clamped_width`, `fill_remaining`); barra da viewport sem álgebra de largura — `PetuniaResponsiveToolbar` mede cada faixa por sonda, decide o overflow pelo rank declarado e desenha a linha no taffy (`measured_widths`, `text_w` e o limiar de 40px foram deletados). Duas armadilhas do `egui_taffy` 0.14 medidas e corrigidas: `flex_basis` percentual resolve para zero (grade nunca dividia a linha) e o `Ui` de cada item herda o layout do pai (linha de 24px media 101px). Registro completo em `docs/audits/ui-ecosystem-final-push/`.
- **Sidebar direita redesenhada (Scene + Inspector contextual)**: sem rail de ícones (abas textuais por contexto via `InspectorContext`: vazio/cena, objeto, seleção de componentes, anotação, medida); Scene adaptativa (AUTO por conteúdo até 38% ou MANUAL pelo divisor, duplo-clique volta ao AUTO) com busca expansível (Ctrl+F), funil de filtros e colapso; árvore `egui_ltreeview` com foco seguindo seleção, rename inline (F2/Enter/duplo-clique, buffer vazio + dica, undo), menu com Export real e Delete sob demanda; header do inspector com rename/pin funcional (ref segura, fallback gracioso)/busca/olho/cadeado + resumo de tipo; Transform redesenhado (Position absoluta, Rotation relativa, Scale por eixo com link, Dimensions absolutas, campos responsivos, undo de 1 nível por gesto via `EditSession`, sem campos placebo); seções leves com resumo colapsado; bloco da operação modal temporário com Aplicar/Cancelar; busca achatando seções; densidade global (Compacta/Confortável/Espaçosa) nas Preferências; `UiDensity`/`SceneFilter`/pin/split no `UiState` (sessão, como o dock); sem dependências novas.

## [0.32.0] - 2026-09-14 — Wave 10: Animation & Rigging (P3D-066, P3D-067, P3D-135 a P3D-139)

### Implementado & Aprimorado
- **Skeleton & Rig Core Desacoplado (`crates/project/src/rig.rs`, P3D-135)**:
  - Sistema fundamental de esqueletos e ossos (`Bone`, `Skeleton`) 100% puro e independente da UI.
  - Transformações tridimensionais canônicas (`Transform3D`) com translação, rotação em quaternions e escala, com suporte a lerp e slerp.
  - Prevenção estrita de ciclos na árvore de ossos (`HierarchyCycleDetected`), reatribuição dinâmica de pais e validação de nomes únicos.
  - Cálculo determinístico de matrizes de repouso (Bind Pose) e matrizes inversas de bind (`inverse_bind_matrix`).
  - Pesos de deformação por vértice (`VertexSkinWeight`) com normalização garantida (soma = 1.0) e algoritmo de deformação Linear Blend Skinning (`SkinData::deform_mesh`).
- **Animação Simples & Trilha de Keyframes (`crates/project/src/animation.rs`, P3D-067)**:
  - Trilhas de ossos (`BoneTrack`) com suporte a keyframes de translação, rotação e escala no tempo contínuo.
  - Interpolação linear contínua e interpolação esférica quaternion (`Slerp`).
  - Clipes de animação (`AnimationClip`) com controle de framerate configurável (24/30/60 FPS), suporte a repetição contínua (looping) e amostragem de poses e matrizes de skinning.
- **Presets Canônicos de Rigging (`crates/project/src/animation.rs`, P3D-136)**:
  - Templates de dados pré-configurados prontos para uso: Humanoide bípede proporcional (15+ ossos), Quadrúpede de 4 patas com cauda, e criatura Multi-Leg parametrizada (aranhas/escorpiões).
- **Auto-Rig & Auto-Skinning Heurístico (`crates/project/src/animation.rs`, P3D-137)**:
  - Dimensionamento proporcional automático (`auto_fit_humanoid`) adaptando o esqueleto à caixa delimitadora (Bounding Box) do modelo ativo.
  - Cálculo geométrico automático de pesos de deformação (`compute_auto_skin_weights`) com atenuação quadrática suave e atribuição dos 4 ossos mais influentes por vértice.
- **Retargeting de Animações Externas (`crates/project/src/animation.rs`, P3D-138)**:
  - Perfil de mapeamento semântico `RetargetProfile` (padrão Mixamo para Petunia Humanoid) e retargeting desacoplado de clipes.
- **Biblioteca de Ativos de Animação (`crates/project/src/animation.rs`, P3D-139)**:
  - Estrutura `AnimationAsset` integrada ao documento do projeto, com biblioteca nativa contendo os clipes canônicos `Humanoid_Idle` e `Humanoid_Walk`.
- **Animation Workspace & Painel de UI (`crates/ui/src/modules_ui/animation_ui.rs`, `crates/ui/src/contextual_shelf.rs`, P3D-066)**:
  - Painel lateral no `Workspace::Animate` contendo gestão de Armatures, Inspetor de Ossos com coordenadas [X, Y, Z] e comprimento, catálogo de clipes e controles de transporte com gravação de keyframes.
  - Shelf contextual inferior com botões dedicados de presets rápidos, Auto-Rig, scrubbing de frames e transporte de reprodução.
  - 20 testes headless de fluxo UI (`kittest_ui_flows.rs`) e 47 testes unitários de domínio passando com 100% de conformidade.

## [0.31.0] - 2026-09-14 — Wave 9: Documentation, QA, Release & GA Hardening (P3D-116 a P3D-121, P3D-126 a P3D-130)

### Implementado & Aprimorado
- **Gerador Determinístico de Referências Técnicas do Código (`crates/xtask/src/generator.rs`, P3D-119)**:
  - Implementação de gerador automático e determinístico de catálogos canônicos em `docs/generated/`:
    - `COMMANDS.md`: Catálogo completo de comandos do `CommandDispatcher`, categorias (`File`, `Edit`, `Model`, `Select`, `View`, `Tools`, `Window`, `Help`), flag destrutivo, tópicos de documentação e descrições.
    - `KEYBINDS.md`: Referência dos 8 perfis canônicos (`petunia-default`, `blender`, `maya`, `3ds-max`, `cinema-4d`, etc.) e mapeamento completo de atalhos.
    - `ICON_TOKENS.md`: Mapeamento de tokens semânticos `PetuniaIcon` e identificadores textuais `IconId` organizados por grupos de interface.
    - `TEXT_TOKENS.md`: Catálogo de internacionalização extraído de `en.toml` e `pt-BR.toml` com chaves semânticas `TextId`.
    - `THEME_TOKENS.md`: Matriz de design tokens `ThemeToken` comparando valores hexadecimais entre os 4 temas canônicos (`petunia-dark`, `petunia-light`, `petunia-capuccino`, `petunia-tokyo-nights`).
    - `SUPPORTED_FORMATS.md`: Matriz de capacidades do `DeliveryPipeline` para OBJ, glTF, GLB e PKG.
    - `index.md`: Portal de navegação para referências geradas.
    - `manifest.json`: Manifesto de integridade e metadados com verificação de tamanho de bytes.
  - Subcomando dedicado `cargo xtask docs-generate` para regeneração rápida sem compilar o VitePress.
- **Detecção de Divergência e Quality Gate Contínuo (`crates/xtask/src/main.rs`, P3D-120)**:
  - Extensão do `cargo xtask docs-check` com verificação de drift: compara em memória o conteúdo gerado com os arquivos rastreados no repositório, falhando com instruções claras de correção se houver desatualização.
  - Validação estrita de 36 arquivos essenciais de documentação e build completo do VitePress sem erros.
- **Ampliação da Suíte de Testes de Regressão de UI Headless (`crates/ui/tests/kittest_ui_flows.rs`, P3D-121)**:
  - 19 testes automatizados com `egui_kittest` cobrindo fluxos essenciais sem depender de temporizações frágeis:
    - Alternância e estabilidade de modos de seleção (`SelectMode::Vertex`, `Edge`, `Face`) e `EditMode` (`Object` vs `Edit`).
    - Renderização da barra contextual horizontal (`Contextual Modeling Shelf`) nos diferentes workspaces (`Model`, `Paint`, `Uv`, `Animate`) e retração graciosa em viewports estreitos.
    - Testes de estabilidade multi-frame (10 frames) em modais e gavetas para prevenir regressões de expansão horizontal.
- **Garantia de Qualidade & Invariantes de Release (P3D-126 a P3D-130)**:
  - Suíte de 8 testes de unidade dedicados em `crates/xtask/src/generator.rs` validando determinismo, presença de identificadores e conformidade de schemas.
  - Zero warnings no Clippy (`-D warnings`) em todos os alvos do workspace.
  - Formatação uniforme com `cargo fmt --check`.
  - Conformidade estrita das fronteiras arquiteturais e GA boundaries (No Remesh, Sem login obrigatório, Domínio puro).

## [0.30.0] - 2026-09-14 — Wave 8: Import, Export & Delivery Pipeline (P3D-068 a P3D-072, P3D-124)

### Implementado & Aprimorado
- **Pipeline Unificado de Entrega e Conversão de Formatos (`crates/project/src/pipeline.rs`, P3D-068 a P3D-072)**:
  - Arquitetura desacoplada e modular `DeliveryPipeline` com registro de exportadores e importadores para formatos 3D: Wavefront OBJ, glTF 2.0 (JSON), glTF 2.0 Binário (.glb) e Petunia Package (.pkg).
  - Matriz de capacidades declaradas (`FormatCapabilities` / P3D-071, P3D-072) indicando de forma explícita suporte a materiais PBR, texturas embutidas, vertex colors, múltiplas malhas e binário.
  - Opções padronizadas de exportação (`ExportOptions`) e importação (`ImportOptions`), controlando triangulação de malhas, inclusão de materiais, fator de escala uniforme e política de sobrescrita segura.
- **Export Individual com Validação & Relatório Detalhado (P3D-068)**:
  - Método `export_single_asset` retornando `ExportReport` estruturado com nome do ativo, caminho absoluto, formato, contagem de bytes escritos e avisos não-fatais.
  - Verificação prévia de sobrescrita com erro tipado `PipelineError::AlreadyExists`.
- **Export Múltiplo & Batch Export Determinístico com Tolerância a Falhas (P3D-069, P3D-070)**:
  - Métodos `export_multiple_assets` e `batch_export` com sanitização consistente de nomes de arquivo (`sanitize_name`).
  - Geração de `BatchExportReport` agregando sucessos e falhas parciais, assegurando que falha em um ativo com geometria corrupta não interrompe ou cancela o processamento dos ativos válidos subsequentes.
- **Importadores Modulares com Validação e Normalização (P3D-071, P3D-124)**:
  - Extração de `save_package_bytes` e `open_package_bytes` em `crates/project/src/package.rs`, permitindo serialização e desserialização in-memory de pacotes versionados sem overhead de arquivos temporários.
  - Conversão transparente de formatos externos para `ImportPayload` com malhas limpas e materiais preservados.
  - Blindagem contra entradas hostis: rejeição segura de OBJs com índices corrompidos, pacotes ZIP truncados e arquivos glTF malformados, retornando erros tipados sem causar pânico.
- **Integração do Exportador GLB aos Canais de Materiais PBR (P3D-050, P3D-072)**:
  - Atualização de `export_gltf` em `crates/project/src/export.rs` para extrair as propriedades reais do `Material` associado ao ativo: cor base linear, rugosidade (`roughnessFactor`), metacidade (`metallicFactor`) e emissão escalada por intensidade (`emissiveFactor`).
- **Integração Headless no ProjectService (`crates/core/src/project_service.rs`)**:
  - Exposição de `export_asset_pipeline`, `export_multiple_pipeline`, `batch_export_pipeline` e `import_file_pipeline` com conversão bidirecional de erros (`From<PipelineError> for ProjectServiceError`).
  - Atualização dos métodos tradicionais `export_obj`, `export_all_obj_to_dir` e `export_glb` para delegar diretamente ao pipeline unificado.
- **Garantia de Qualidade & Conformidade**:
  - Suíte de 14 testes de integração dedicados em `crates/project/tests/pipeline_tests.rs` cobrindo round-trips, matriz de capacidades, escala e testes de estresse contra arquivos corrompidos (P3D-124).
  - Testes de integração adicionais no `ProjectService` (`crates/core/tests/project_service_tests.rs`).
  - 100% de aprovação em todos os testes, zero avisos no Clippy (`-D warnings`), formatação uniforme (`cargo fmt --check`), e validações `xtask arch-check` e `xtask docs-check` aprovadas.

## [0.29.0] - 2026-09-14 — Wave 7: Materials, Texture, UV & Paint Engine (P3D-050 a P3D-065, P3D-132 a P3D-134, P3D-140)

### Implementado & Aprimorado
- **Modelo Canônico de Materiais e Canais PBR (P3D-050 a P3D-054, P3D-140)**:
  - Criação de `crates/project/src/material.rs` com modelo unificado: `ShaderProfile` (`Pbr`, `Unlit`, `Toon`, `Glass`, `Emissive`), `AlphaMode` (`Opaque`, `Mask`, `Blend`), canais de textura (`Albedo`, `Normal`, `Roughness`, `Metallic`, `Emission`, `Height`) e conversão bidirecional Roughness ↔ Glossiness.
  - Vínculo direto de slots de material em faces de malha (`material_slot`) e ativos de cena (`material_id`), com garantia de Single Source of Truth no `Project`.
  - Normalização automática de arquivos de projeto legados em `Project::validate()` para compatibilidade sem corrupção de schema.
- **UV Workspace & Algoritmos de Desdobramento (P3D-063, P3D-064, P3D-065)**:
  - Algoritmos de projeção UV: Mapeamento Planar (`project_planar`), Mapeamento Cúbico 6 planos (`project_cube`) e Auto Unwrap não-destrutivo integrado ao `xatlas-rs-v2` (`unwrap_auto`).
  - Transformações UV no workspace: translação, escala e rotação incremental de 90° para ilhas UV.
- **Motor de Pintura 2D & Projeção 3D sobre Malha (P3D-055 a P3D-062, P3D-132)**:
  - 5 modos canônicos de pincel: Pixel Brush rígido, Soft Brush com atenuação quadrática suave, Borracha com atenuação do canal alfa, Flood Fill com tolerância de cor e Conta-gotas (Eyedropper).
  - Pintura contínua sobre a malha 3D via projeção baricêntrica de coordenadas UV no viewport interativo.
  - Máscaras de pintura e isolamento de seleção (`isolate_selection` / P3D-132), restringindo a área de pintura exclusivamente às faces selecionadas.
- **Pilha Unificada de Camadas, Decalques & Efeitos (P3D-061, P3D-133, P3D-134)**:
  - `PaintLayerStack` determinístico com suporte a camadas raster, decalques parametrizados (`DecalLayer` com projeção UV, escala, rotação e opacidade) e pilha de efeitos não-destrutivos (`PaintEffect::Pixelate`, `PaintEffect::Posterize` e `PaintEffect::Invert`).
- **Hub de Materiais no Painel de Propriedades (P3D-048, P3D-050)**:
  - Interface completa de edição de materiais: seleção de shader profile, seletor RGBA de cor base sincronizado, sliders de roughness e metálico, escala de normal map, cor e intensidade de emissão, controles de canal alfa e integração com a paleta do projeto.
- **Integração dos Renderers WGPU & OpenGL aos Materiais (P3D-051, P3D-140)**:
  - Sincronização automática das propriedades de materiais, texturas de albedo e perfis de sombreamento nos pipelines gráficos do WebGPU (`render-wgpu`) e OpenGL Desktop 3.3+ (`render-gl`).
- **Qualidade & Confiabilidade**:
  - 100% de aprovação na suíte de testes com zero avisos no Clippy (`-D warnings`) em todo o workspace.
  - Verificação de arquitetura (`xtask arch-check`) e documentação oficial (`xtask docs-check` com VitePress) 100% compliant.

## [0.28.1] - 2026-09-14 — Wayland Crash Elimination & In-Canvas File Dialog System


### Corrigido & Aprimorado
- **Eliminação de Falha de Segmentação no Wayland (`crates/app/src/lib.rs`)**:
  - Resolução definitiva do segmentation fault (`SEGV_MAPERR` em `wl_proxy_destroy` no `smithay-clipboard`) em ambientes Wayland / Mesa Intel (Ivy Bridge / Gen 7).
  - Implementado `SafeDisplayTarget` no `egui-winit::State::new`, desativando a inicialização da thread instável do `smithay-clipboard` e delegando as operações de clipboard de forma segura para o `arboard`.
  - Implementado tratamento atômico de encerramento (`exiting` e `CloseRequested`) no `WgpuApp` e `GlApp`, desalocando instâncias gráficas e janelas antes da finalização do loop de eventos.
- **Sistema Integral de Diálogos de Arquivos In-Canvas (`crates/ui/src/file_dialog_service.rs`, `crates/ui/src/reference_manager.rs`, `crates/ui/src/lib.rs`)**:
  - Eliminação completa de chamadas ao seletor nativo do sistema operacional (`rfd::FileDialog` / DBus portal `ashpd`), que gerava erros de `zbus::proxy` e não conformidade visual.
  - O Gerenciador de Imagens de Referência, o menu principal e os eventos de importação/exportação de paleta agora utilizam exclusivamente o explorador de arquivos integrado (`egui-file-dialog`), operando diretamente dentro da janela da aplicação com tema, filtros por extensão e responsividade idêntica.

## [0.28.0] - 2026-09-14 — UI/UX Polish, Theming, Icon System & Viewport Ergonomics (13 Critical Defects Resolved)

### Corrigido & Aprimorado
- **1. Sistema de Temas e Tokenização Reativa**:
  - Propagação integral dos temas e tokens para todos os componentes egui e sincronização com as cores de limpeza de viewport no OpenGL e WebGPU.
- **2. Internacionalização Completa (i18n)**:
  - Adição de chaves ausentes em `pt-BR.toml` e `en.toml` para ferramentas, botões de ação, prateleira contextual, menus e barras de viewport.
- **3. Sistema de Ícones e Pacotes Dinâmicos (Icon Packs)**:
  - Resolução dos pacotes de ícones (`Petunia`, `Phosphor`, `Tabler`, `Iconoir`, `Lucide`) com substituição de glifos ausentes por motor vetorial nativo em egui Painter, eliminando completamente os glifos corrompidos ("quadrados/tofu").
  - Mapeamento completo de glifos Phosphor para todas as ferramentas e comutadores.
  - Pré-visualização autêntica de cada pacote de ícones nas opções do modal de configurações (`paint_pack`).
- **4. Prateleira Contextual Flutuante (Modeling Shelf)**:
  - Correção de cálculo de largura da cápsula de fundo (`measured_w` em retângulo irrestrito), eliminando o corte de bordas e desalinhamento visual.
  - O botão de imagem de referência agora abre o Gerenciador de Imagens de Referência (`Reference Manager`) em vez do seletor direto de arquivos.
- **5. Gerenciador de Imagens de Referência (Reference Manager)**:
  - Redesenho responsivo em formato de cartões (`horizontal_wrapped`) com caixa de miniatura clicável que abre o seletor de arquivos e sliders intuitivos de ajuste de escala, rotação, opacidade e offset.
- **6. Menus Dropdown e Popovers**:
  - Redimensionamento e contenção da largura dos popovers e dropdowns para 148–165px com estilização reativa ao tema ativo.
- **7. Grade 3D e Ajustes do Viewport**:
  - Adicionado botão de engrenagem (`⚙`) ao lado do checkbox da grade, exibindo menu suspenso com sliders para tamanho, subdivisões, opacidade e guias isométricas.
- **8. Sincronização de Overlays**:
  - Toggles de eixos mundiais e bússola/HUD de navegação integrados e sincronizados com os backends de renderização.
- **9. Translucidez no Modo X-Ray**:
  - Pipeline de visualização X-Ray com alfa translucidez (`0.45`) e arestas sem oclusão nos renderizadores OpenGL e WGPU.
- **10. Snapping Magnético na Viewport**:
  - Implementação de magnetismo automático ao mover vértices e objetos na viewport com controle visual.
- **11. Movimentação Livre da Câmera (Free Movement)**:
  - Suporte à translação irrestrita no plano da câmera (`ModalConstraint::Free`) com cálculo em tempo real.
- **12. Interceptação da Tecla Tab**:
  - Interceptação preventiva da tecla `Tab` no loop do Winit antes da navegação de foco nativa do egui, garantindo alternância consistente entre Modo Objeto e Modo Edição.
- **13. Orientação Global vs Local do Gizmo**:
  - Orientação do gizmo de transformação reativa ao toggle Global/Local através de detecção dos eixos da malha (`local_axes_for_mesh`).

## [0.27.1] - 2026-09-14 — Master Implementation Gauntlet: Viewport Picking, Outliner Polish & Documentation Unification

### Adicionado
- **Unificação Integral da Documentação (Single Source of Truth) (`docs/bible/`)**:
  - Consolidação e migração de todos os 191 documentos da **Implementation Bible** (`implement-bible/`) e 38 capítulos de fundação narrativa (hoje em `docs/bible/foundations/`) para a estrutura canônica e rastreável dentro de `docs/bible/`.
  - Higienização completa de todos os nomes de arquivos em slugs kebab-case previsíveis (eliminação dos hashes hexadecimais Notion).
  - Resolução automática e determinística de 195 links internos relativos entre especificações, capítulos constitucionais (00 a 16), seções temáticas (A a O) e adendos.
  - Atualização dos status canônicos de cada especificação P3D (P3D-001 a P3D-049 marcadas como `COMPLIANT` conforme as entregas provadas das Waves 0 a 6).
  - Integração da Bíblia diretamente à barra de navegação e sidebar do VitePress com índice mestre estruturado em `docs/bible/index.md`.
- **Botão de Exclusão Direta e Atalhos no Outliner (`crates/ui/src/outliner.rs`, `crates/config/src/keybinds.rs`, `crates/app/src/lib.rs`)**:
  - Adicionado botão de ícone de lixeira (`PetuniaIcon::Delete`) com tooltip descritivo em cada linha de asset no Outliner.
  - Habilitados atalhos de teclado `Delete`, `Backspace` e `Shift+D` diretamente no Outliner e no loop de eventos do Winit.
- **Botão de Reancoragem de Painéis Flutuantes (`crates/ui/src/properties_panel.rs`, `crates/ui/src/lib.rs`)**:
  - Adicionado botão `⇲ Dock` de alta visibilidade no cabeçalho do Properties Inspector quando em modo flutuante.
  - Adicionado banner contextual com botão `[Reancorar]` na barra lateral direita quando o inspetor estiver destacado.

### Corrigido
- **Seleção de Múltiplos Objetos no Viewport 3D (`crates/app/src/lib.rs`)**:
  - Corrigido o picking em modo Objeto (`handle_pick`), que agora itera por todos os assets visíveis e desbloqueados da cena, selecionando o objeto mais próximo da câmera com suporte a alternância cumulativa via tecla `Shift`.
- **Responsividade e Estabilidade do Outliner (`crates/ui/src/outliner.rs`)**:
  - Desativado o drag-and-drop de nós de árvore (`allow_drag_and_drop(false)`), eliminando o atraso de cliques e a criação de fantasmas visuais de arrasto.
  - Substituição de labels simulados por `ui.selectable_label(...)` nativo com reação imediata a cliques.

## [0.27.0] - 2026-09-14 — Master Implementation Gauntlet: Wave 6 (Scene, Assets, Outliner & Inspector)

### Adicionado
- **P3D-042 / P3D-043 / P3D-044 / P3D-045 — Asset Browser, Filtragem & Drag-and-Drop Workflow (`crates/ui/src/asset_browser.rs`, `crates/ui/src/viewport_interaction.rs`, `crates/core/src/command.rs`)**:
  - Novo comando semântico de domínio `model.instantiate_asset` (`InstantiateAssetCmd`) e método auxiliar `AppState::instantiate_asset_by_id`, permitindo criar instâncias independentes de modelos catalogados na biblioteca em coordenadas arbitrárias ou no 3D Cursor.
  - Suporte de primeira classe para Drag-and-Drop (`egui::DragAndDrop`) arrastando miniaturas do Asset Browser diretamente para o Viewport 3D.
  - Interseção por raycast contra o plano de chão $Y=0$ (`modal_viewport::plane_point`), exibição de indicador de alvo e anel no viewport e instanciação exata no ponto de soltura com limpeza de payload.
  - Controle configurável de tamanho de miniaturas (`state.ui.asset_thumbnail_size`, 32px a 128px) com slider de zoom dinâmico e layouts responsivos para visualização em lista ou cartões expandidos.
  - Filtragem multi-critério: busca textual abrangendo nome, tags e coleção, além de abas de categoria (Props, Chars, Env).
- **P3D-046 / P3D-047 / P3D-082 — Outliner & Context Menus Padronizados (`crates/ui/src/outliner.rs`, `crates/ui/src/widgets.rs`)**:
  - Modernização completa dos menus de contexto da árvore do Outliner e dos nós de objetos e coleções com o widget `PetuniaMenuItem`, ícones semânticos vetoriais e badges de atalho.
  - Bloqueio rigoroso de assets bloqueados (`locked`), impedindo interação de gizmo, transformações e seleções acidentais no viewport.
- **P3D-048 / P3D-078 — Inspector Destacável em Janela Flutuante (`crates/ui/src/lib.rs`, `crates/ui/src/properties_panel.rs`)**:
  - Novo estado de UI `inspector_detached: bool` com botão de alternância estilizado no cabeçalho das abas de propriedades (`PetuniaIcon::Maximize` / `Minimize`).
  - Quando ancorado: divide o painel lateral direito com o Outliner.
  - Quando destacado: expande o Outliner para a altura total da barra lateral e abre o Properties Inspector em janela flutuante, redimensionável e móvel (`egui::Window`).
- **P3D-049 — Live Transform Inspector (`crates/ui/src/properties_panel.rs`)**:
  - Inspeção contínua e edição interativa de Location X, Y, Z e Scale diretamente sincronizados com o centróide real da geometria ativa (`mesh.selection_center()`).
  - Atualização em tempo real da malha com checkpoint atômico único de histórico no evento `drag_stopped()`.
  - Ação "Reset to Origin" centralizando o objeto e seus vértices de volta à origem do mundo `[0.0, 0.0, 0.0]`.
- **P3D-082 / P3D-083 — Menus de Contexto do Viewport & Segregação de Propriedades de Ferramentas (`crates/ui/src/nav_gizmo.rs`, `crates/ui/src/viewport_interaction.rs`, `crates/ui/src/properties_panel.rs`)**:
  - Menu de contexto RMB do Viewport totalmente adaptativo ao domínio de seleção ativo (`Object`, `Vertex`, `Edge`, `Face`), utilizando `PetuniaMenuItem`.
  - Aba "tool" dedicada para parâmetros de ferramentas ativas de modelagem e transformação isolada das propriedades de objetos.
- **Qualidade & Testes**:
  - Adicionados testes de comandos unitários para instanciação de assets e undo.
  - Adicionados novos fluxos de teste de UI com `egui_kittest` (totalizando 14 fluxos de integração).
  - 88/88 testes de UI e 14/14 fluxos kittest aprovados.
  - `cargo fmt`, `cargo clippy -D warnings`, `xtask arch-check` e `xtask docs-check` aprovados com 100% de conformidade.

### Corrigido
- **Resolução de Conflito de LayerId ao Destacar/Ancorar o Inspector (`crates/ui/src/lib.rs`, `crates/ui/src/tool_fields.rs`)**:
  - Corrigido o pânico `Widget changed layer_id during the frame from Background to Middle` que ocorria ao clicar para destacar o painel de propriedades para uma janela flutuante.
  - Implementado snapshot de frame (`was_detached`) no `right_panel`, garantindo que o inspector nunca seja desenhado simultaneamente no painel lateral (`Background`) e na janela flutuante (`Middle`) no mesmo frame de transição.
  - Isolamento de IDs de widgets filhos da janela flutuante através de `ui.push_id("detached_inspector", ...)`.
  - Escopo de IDs de widgets numéricos de ferramentas (`tool_fields`) associados dinamicamente ao container pai (`ui.id().with(...)`), eliminando colisões de identificadores estáticos globais entre camadas.
  - Adicionados testes de regressão de transição e isolamento de camadas em `crates/ui/tests/kittest_ui_flows.rs`.
- **Resolução de Expansão Horizontal Infinita em Janelas Modais / Flutuantes (`crates/ui`)**:
  - Eliminado o ciclo de feedback de redimensionamento horizontal infinito em todas as janelas modais (`Settings`, `Asset Library Drawer`, `Reference Set Manager`, `Properties Inspector Detached`, `Command Palette`, `Recovery Dialog`).
  - Aplicada restrição estrita de teto dimensional `.max_size(...)` amarrada à resolução de tela (`ctx.screen_rect()`) em todas as instâncias de `egui::Window`.
  - Configurado `auto_shrink([true, false])` em áreas de rolagem verticais (`ScrollArea::vertical()`), prevenindo que o layout solicite expansão horizontal descontrolada.
  - Corrigido o widget `petunia_search_box`: alocação explícita de bounding box para o ícone de busca e largura desejada protegida contra estiramento abusivo de contêineres pais.
  - Corrigida a alocação de células de grid em `reference_manager.rs`, substituindo `ui.available_width() * 0.48` por largura de coluna fixa determinística (`min_col_width(240.0)`).
  - Adicionados 4 novos testes de estabilidade multi-frame com `egui_kittest` cobrindo 10 quadros consecutivos sem mutação cumulativa de largura.

---

## [0.26.0] - 2026-09-14 — Master Implementation Gauntlet: Wave 5 (Selection, Transform & Modeling Core)

### Adicionado
- **P3D-015 / P3D-016 — Domínio Unificado de Seleção e Mapeamento de Modos (`crates/core/src/selection.rs`, `crates/core/src/state.rs`, `crates/ui/src/viewport_bar.rs`)**:
  - Introdução do enum `SelectionDomain { Object, Vertex, Edge, Face }` unificando a interação e eliminando a segregação estrita entre modos.
  - Alternância rápida via tecla `Tab` entre Object e o último domínio de componente (`Vertex`/`Edge`/`Face`), com atalhos numéricos diretos `0` (Object), `1` (Vertex), `2` (Edge), `3` (Face).
  - Controle segmentado de 4 pílulas com ícones vetoriais dedicados (`PetuniaIcon::ModeObject`, `SelectVertex`, `SelectEdge`, `SelectFace`) na barra superior do Viewport.
  - Rastreamento robusto de seleção de arestas `edges: Vec<(u32, u32)>` na estrutura canônica `Selection`.
  - Novos comandos semânticos registrados no dispatcher: `select.domain_object`, `select.domain_vertex`, `select.domain_edge`, `select.domain_face`, `select.cycle_domain`.
- **P3D-026 / P3D-027 — Orientações de Transformação e Pontos de Pivô Fortemente Tipados (`crates/core/src/state.rs`, `crates/ui/src/viewport_bar.rs`)**:
  - `TransformOrientation { Global, Local }` com suporte de primeira classe no domínio e seleção via dropdown no Viewport Bar.
  - `PivotPoint { MedianPoint, BoundingBoxCenter, Cursor3D, IndividualOrigins }` com cálculo matemático puro via `AppState::calculate_pivot(&self, pivot: PivotPoint) -> Vec3`.
  - Integração no cálculo de pivô em operações modais de transformação (`ModalKind::Move`, `Rotate`, `Scale`).
- **P3D-037 — Separação de Seleção (`crates/core/src/command.rs`)**:
  - Comando semântico `model.separate_selection` (`SeparateSelectionCmd`) extraindo elementos de malha selecionados para um novo asset independente na cena com preservação do histórico de Undo.
- **P3D-039 — Motor de Edição Proporcional (`crates/core/src/proportional.rs`, `crates/ui/src/viewport_bar.rs`, `crates/core/src/modal.rs`)**:
  - `ProportionalFalloff { Smooth, Linear, Sphere, Sharp, Constant }` com funções matemáticas puras de atenuação (curva Hermite cúbica, decaimento linear, perfil esférico, etc.).
  - `ProportionalSettings { enabled, radius, falloff }` mantido na sessão do editor.
  - Botão segmentado com ícone de círculos concêntricos e popover dropdown com seleção de curva de decaimento e slider de raio de influência no Viewport Bar.
  - Deformação suave em tempo real de vértices não selecionados durante operações modais (`Move`).
- **P3D-040 — Motor de Snapping Magnético Geométrico e de Grade (`crates/core/src/snap.rs`, `crates/ui/src/viewport_bar.rs`, `crates/core/src/modal.rs`)**:
  - `SnapTarget { Grid, Increment, Vertex, Edge, Face }` e `SnapSettings { enabled, target, element, grid_spacing, snap_distance }`.
  - Algoritmos matemáticos headless puros para atração a coordenadas de grade, vértices, arestas e faces por projeção vetorial e proximidade.
  - Botão mestre com ícone de ímã, atalho canônico `Shift+Tab` e popover de configuração de alvo, espaçamento de grade e distância de snap no Viewport Bar.
  - Aplicação automática de snapping durante transações modais.
- **P3D-131 — Sistema de Feedback de Ferramentas Modais (`crates/core/src/modal_feedback.rs`, `crates/ui/src/modal_viewport.rs`, `crates/ui/src/status_bar.rs`)**:
  - Descritor desacoplado `ToolFeedback` informando origem, ponto atual, linha-guia 3D, texto de delta numérico, restrições e indicador magnético `[SNAP]`.
  - Overlay 3D desenhando a linha-guia no viewport com destaque em âmbar quando atraído magneticamente.
  - Exibição dinâmica da magnitude e atalhos contextuais da ferramenta ativa na Status Bar inferior.
- **Qualidade & Testes**:
  - Testes unitários para cálculo de pivôs, falloff proporcional, snapping a grid/elementos e feedback modal.
  - Bateria completa de testes de UI (88/88) e integração Kittest (7/7) 100% aprovados.
  - Zero warnings no Clippy (`cargo clippy --workspace -- -D warnings`), conformidade total com `xtask arch-check` e `xtask docs-check`.

---

## [0.25.0] - 2026-09-13 — Master Implementation Gauntlet: Wave 4 (Viewport, Navigation & Reference Workflow)

### Adicionado
- **P3D-004 / P3D-006 — Projeção e Vistas Canônicas & Isométricas (`crates/core/src/camera.rs`, `crates/core/src/command.rs`)**:
  - Implementação dos presets isométricos canônicos para workflows de jogos e visualização técnica: `IsometricNE`, `IsometricNW`, `IsometricSE`, `IsometricSW` com ângulo de inclinação matematicamente exato ($\arcsin(\tan(30^\circ)) \approx 35.264^\circ$).
  - Detecção e classificação em tempo real da orientação da câmera via `Camera::nominal_view(&self) -> (&'static str, f32, f32)` com normalização angular e tolerância a drift.
  - Registro canônico de comandos semânticos de navegação no `CommandDispatcher`: `view.front`, `view.back`, `view.left`, `view.right`, `view.top`, `view.bottom`, `view.isometric_ne`, `view.isometric_nw`, `view.isometric_se`, `view.isometric_sw`.
- **P3D-005 / P3D-075 — Navigation HUD no Viewport (`crates/ui/src/nav_gizmo.rs`, `crates/ui/src/viewport_interaction.rs`)**:
  - Badge translúcido moderno de visualização nominal exibido no canto superior esquerdo do Viewport 3D com leitura clara do nome da vista e ângulos (ex: `Front Ortho · Pitch +0° Yaw +0°` ou `Isometric NE`).
  - Flag de configuração `show_nav_hud: bool` no `EditorSession` e comando `view.toggle_nav_hud` para ativar/desativar exibição.
- **P3D-008 — Frame Selected & Frame All (`crates/core/src/state.rs`, `crates/core/src/command.rs`)**:
  - Implementação pura de `AppState::frame_all(&mut self)` que computa a caixa delimitadora (AABB) agregada de todos os assets e referências visíveis na cena.
  - Comando semântico `view.frame_all` (`FrameAllCmd`) registrado no dispatcher e mapeado para a tecla `Home` nos keybinds padrão.
  - Invariante rigorosamente garantido: enquadramentos de câmera (`frame_selection` e `frame_all`) animam o viewport sem modificar o dirty state do projeto.
- **P3D-013 / P3D-014 — Gerenciador de Conjuntos de Referências (Reference Sets) (`crates/ui/src/reference_manager.rs`, `crates/core/src/project_service.rs`)**:
  - Nova janela utilitária modal `Reference Set Manager` (`window.reference_manager`, atalho `Shift+R`).
  - 6 slots ortográficos canônicos independentes: `Front`, `Back`, `Left`, `Right`, `Top` e `Bottom`.
  - Pré-visualização em miniaturas geradas por textura cacheada na GPU, nome e dimensões em pixels.
  - Controles de calibração fina independentes por referência: opacidade (0.0..1.0), escala/tamanho (0.1..20.0), offset (-20.0..20.0), rotação (-180°..180°), bloqueio de edição (`locked`) e modo X-Ray.
  - Botão de alinhamento rápido da câmera 3D com o ângulo de referência do slot selecionado.
  - Ações de substituição individual de imagem sem recriação do conjunto e remoção atômica por slot.
  - Métodos puros no `ProjectService`: `set_reference_slot`, `remove_reference` e `clear_references`.
- **P3D-009 / P3D-010 — Popover de Overlays no Viewport Context Bar (`crates/ui/src/viewport_bar.rs`, `crates/render-wgpu/src/lib.rs`)**:
  - Botão segmentado de alternância geral de overlays integrado a um popover dropdown compacto (`▾`).
  - Opções centralizadas no popover: Grade 3D (Grid), Eixos Mundiais (Axes), Cursor 3D, Aramado (Wireframe Overlay), Triangulação (Diagonais) e Navigation HUD.
  - Acesso direto ao Gerenciador de Referências no popover.
  - Renderizador WebGPU e controle de cena adaptados para respeitar `show_overlays` e `show_grid` sem consumo de draw calls desnecessárias.
- **Qualidade & Testes**:
  - Testes unitários para projeções isométricas, detecção de vistas nominais, `frame_all` e ciclo de vida de slots de referência.
  - Testes de fluxo UI com `egui_kittest` cobrindo o Gerenciador de Referências e o Navigation HUD.
  - 100% de conformidade com todos os Quality Gates do workspace.

---

## [0.24.0] - 2026-09-13 — Master Implementation Gauntlet: Wave 3 (UI Infrastructure, Customization & Input)

### Adicionado
- **P3D-081 — Command Palette (`crates/ui/src/command_palette.rs`, `crates/core/src/command.rs`)**:
  - Overlay centralizado elegante de busca e execução rápida (`Ctrl+P` / `Ctrl+Shift+P`).
  - Busca fuzzy e por tokens sobre o catálogo canônico de comandos.
  - Navegação fluida por teclado (`ArrowDown`, `ArrowUp`, `Enter`, `Escape`) e auto-focus no campo de busca.
  - Exibição de categoria funcional por pílulas visuais (`File`, `Edit`, `Model`, `View`, `Tools`, `Window`, `Help`, `Select`).
  - Validação contextual estrita (`can_execute`): comandos desabilitados exibem a razão legível (ex: `Requires Edit mode`, `Nothing to undo`) sem execução indevida e com zero lógica de domínio na camada de apresentação.
- **P3D-077 — Menus Padronizados & Submenus Profissionais (`crates/ui/src/widgets.rs`, `crates/ui/src/main_header.rs`)**:
  - Componente `PetuniaMenuItem` aprimorado com renderização de atalhos dinâmicos alinhados e setas de submenu (`›`).
  - Novos widgets canônicos `PetuniaMenuCheckboxItem` (com indicador visual `✓`) e `PetuniaMenuRadioItem` (com indicador de ponto preenchido).
  - Submenu de Projetos Recentes no menu Arquivo (`File`), consumindo o histórico persistente de `state.project.recent_projects`.
  - Migração de todos os menus superiores para widgets e design tokens canônicos, eliminando controles genéricos.
- **P3D-084 / P3D-085 — Design Tokens, Temas & Customização**:
  - Submenu de temas integrado no menu Janela com seleção imediata dos temas canônicos: Petunia Dark, Petunia Light, Capuccino e Tokyo Nights.
- **P3D-089 — i18n & Sincronização de Locales (`assets/locales/en.toml`, `pt-BR.toml`)**:
  - Eliminação de condicionais ad-hoc (`if lang == "en" ...`) no cabeçalho em favor de chaves padronizadas (`menu.window`, `menu.command_palette`, `menu.preferences`, `menu.recent_projects`, `command_palette.*`).
  - Sincronização estrita de todas as chaves entre inglês e português do Brasil.
- **P3D-090..099 — Keymaps, Detecção de Conflitos e Rebinding (`crates/config/src/keybinds.rs`, `crates/ui/src/settings_modal.rs`)**:
  - Sistema de detecção de conflitos context-aware: diferencia colisões exatas (`Exact`), sobreposições com contexto global (`ContextOverlap`), preservando contextos disjuntos (`model` vs `paint`) sem falsos positivos.
  - Proteção de teclas reservadas (`Reserved` para `Escape`).
  - Métodos utilitários de remapeamento (`set_binding`, `remove_binding`, `export_to_toml`).
  - Inclusão canônica de `global.command_palette` (`Ctrl+P`).
- **P3D-114 / P3D-115 — Documentação Contextual e Help Topics (`crates/core/src/docs.rs`)**:
  - Enum `DocsTopic` centralizando 15 tópicos e mapeamento para URLs canônicas (`https://petunia3d.org/docs/...`).
  - Menu de Ajuda (`Help`) com links diretos para documentação online e abertura no navegador padrão do sistema.
  - Associação de tópicos de documentação nos metadados de comandos (`CommandMetadata`).
- **Qualidade & Testes**:
  - Testes unitários para catálogo de comandos, busca, atalhos e validação contextual.
  - Testes de UI sem pânico para Command Palette e menus padronizados.
  - 100% de conformidade com todos os Quality Gates do workspace.

---

## [0.23.0] - 2026-09-13 — Master Implementation Gauntlet: Wave 2 (Project Integrity & Asset Foundation)

### Adicionado
- **P3D-001 — Sistema de Projetos, Escrita Atômica e Dirty State (`crates/project/`, `crates/core/`)**:
  - `save_atomic`: Gravação atômica segura via arquivo temporário oculto `.tmp.{uuid}` no mesmo diretório pai, garantia de persistência física em disco com `sync_all()` e substituição atômica por `rename`. Caso ocorra qualquer falha durante a gravação, o arquivo original do usuário permanece 100% intacto.
  - Formato versionado `.petunia`: Cabeçalho canônico com magic bytes `PETUNIA\0`, `PROJECT_VERSION: u32 = 1` e validação de confiança (M2/M3/M4) com rejeição explícita de versões futuras incompatíveis via `ProjectError::Version`.
  - Rastreamento determinístico de `dirty state`: `ProjectState::is_dirty` e `AppState::is_document_dirty()` desacoplados do dirty de renderização da GPU. Mutações destrutivas marcam o documento como alterado; `save_project` marca como limpo; `Undo` até o ponto do save restaura o clean state sem intervenção manual.
  - `RecentProjects` (`crates/core/src/recent_projects.rs`): Gerenciador persistente de projetos recentes com limite configurável, ordenação temporal e higienização automática de caminhos ausentes no disco (`prune_missing`).
  - `save_as_project`: Salvamento para novo destino com atualização do caminho ativo e marcação limpa.
- **P3D-002 — Autosave & Recuperação de Sessão (`crates/project/src/autosave.rs`, `crates/ui/src/recovery_dialog.rs`, `crates/app/src/lib.rs`)**:
  - `AutosaveService`: Serviço puro de aplicação para snapshots periódicos, desacoplado de taxas de quadros ou componentes de UI.
  - Snapshots rotativos seguros em `.petunia/autosave/autosave-{timestamp}-{seq}.petunia`, com política de retenção dos últimos $N$ snapshots (default 5).
  - Invariante de integridade: Autosave **nunca** sobrescreve o arquivo principal e **nunca** limpa o dirty state do projeto.
  - Marcador de sessão ativa (`session.lock`): Registro de PID, timestamp e projeto em execução para detecção confiável de unclean shutdown ou crash.
  - `RecoveryDialog`: Diálogo modal claro apresentando decisão explícita entre `Recover Project` (carrega snapshot como dirty sem sobrescrever o oficial), `Open Saved Version` e `Discard Recovery`.
  - Remoção garantida do `session.lock` em clean shutdown em ambos os loops WebGPU e OpenGL do `petunia_app`.
- **P3D-003 — Biblioteca de Modelos por Projeto (`crates/project/src/model_library.rs`)**:
  - `ModelLibraryService`: Serviço unificado de consulta, busca textual, filtragem e ordenação consumido tanto pelo `Asset Browser` quanto pela `Project Model Library`.
  - Suporte a tags flexíveis (`tags: Vec<String>`) com normalização para case-insensitivity e agregação de tags com contagem por projeto.
  - Suporte a favoritos (`favorite: bool`, `toggle_favorite`).
  - Modos de ordenação parametrizados: Nome A–Z / Z–A, Contagem de Triângulos, Vértices e Ordem de Adição.
- **P3D-041 — Undo / Redo com Saved State Tracking (`crates/commands/src/lib.rs`)**:
  - `UndoStack`: Rastreamento de `clean_version` e `current_version`. Desfazer alterações até o ponto salvo em disco restaura automaticamente o clean state (`is_clean() == true`).
- **P3D-108 — IDs Estáveis em todo o Ciclo de Vida**:
  - Preservação estrita de identificadores `Uuid` para assets (`Asset.id`), anotações (`AnnotationItem.id`), medições (`MeasurementItem.id`) e metadados de projeto (`Project.id`).
  - Métodos utilitários de acesso por identidade estável: `find_by_id`, `find_by_id_mut`, `remove_by_id` e `duplicate_by_id`.
- **Governança Arquitetural & Testes Automatizados**:
  - Novo gate arquitetural `project_and_persistence_must_not_depend_on_egui` adicionado a `tests/architecture_fitness.rs` e `crates/xtask/src/main.rs`.
  - Testes de failure injection para salvamento atômico, rejeição de formatos futuros e corrompidos, ciclo de vida do autosave e recuperação.
  - Total de testes no workspace elevado de 282 para **292 testes** (100% passando).

---

## [0.22.0] - 2026-09-13 — Master Implementation Gauntlet: Wave 1 (Architecture Spine)

### Adicionado
- **Consolidação de Comandos Canônicos (`crates/core/src/command.rs`, `crates/core/src/lib.rs`)**:
  - `SelectLinkedCmd`: Comando para seleção limpa de componentes conectados (ilhas de malha) no mesh ativo (`model.select_linked`).
  - `BoxSelectCmd`: Comando para seleção de área por frustum/retângulo 2D normalizado no viewport.
  - `ToggleLockAssetCmd`: Comando transacional com Undo/Redo para alternar estado de bloqueio de assets.
  - `ToggleVisibilityAssetCmd`: Comando transacional com Undo/Redo para alternar visibilidade de assets.
  - `SetAssetCollectionCmd`: Comando transacional com Undo/Redo para atribuir ou remover um asset de uma coleção organizacional.
  - `ToggleCollectionVisibilityCmd` & `ToggleCollectionLockCmd`: Comandos transacionais com Undo/Redo para operações em lote sobre coleções.
- **Desacoplamento e Convergência de UI (`crates/ui/src/outliner.rs`, `crates/ui/src/lib.rs`, `crates/app/src/lib.rs`)**:
  - Eliminação de mutações diretas do Outliner: toggles de cadeado (lock), olho (visibilidade) e movimentação para coleções agora despacham comandos canônicos via `state.dispatch`.
  - Box select do viewport migrado para despacho semântico via `BoxSelectCmd`.
  - Atalho de teclado `model.select_linked` roteado através de `SelectLinkedCmd`.
- **Governança Automatizada de Invariantes (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - `tools_must_not_depend_on_physical_keycodes`: Validação estrita de que ferramentas de modelagem não referenciam `PhysicalKey`, `winit::keyboard` ou `egui::Key`.
  - `core_and_domain_must_not_depend_on_eframe`: Validação estrita de ausência de `eframe` em todos os 12 crates de domínio e infraestrutura neutra.
  - Gate `cargo run -p xtask -- arch-check` atualizado confirmando Wave 1 concluída com 100% de sucesso.
- **Suíte de Testes Automatizados (`crates/core/tests/command_tests.rs`)**:
  - 3 novos testes de integração cobrindo o ciclo transacional e Undo/Redo para os comandos de seleção e operações de outliner/coleções.
  - Total de testes no workspace elevado para 282 testes unitários, integração e UI, todos verdes.

---

## [0.21.0] - 2026-09-13 — Core V1 & Interactive Geometry Refinement (Gauntlet Loop)

### Adicionado
- **Core V1: Triangulation Inspection & Flip Diagonal (`crates/mesh/`, `crates/core/`)**:
  - `Mesh::triangulation_wireframe(&self)`: Extração determinística de todas as diagonais internas de corte fan em polígonos quadrangulares e n-gons para renderização de wireframe de suporte.
  - `Mesh::flip_diagonal(&mut self)`: Inversão inteligente de diagonal suportando:
    1. Quads selecionados: rotação cíclica do loop de vértices e UVs (`rotate_left(1)`), alternando a diagonal fan preservando winding, plano e normais;
    2. Aresta selecionada compartilhada por dois triângulos adjacentes (Delaunay Edge Flip).
  - `FlipDiagonalCmd`: Comando transacional no `CommandDispatcher` com histórico completo de Undo/Redo.
- **Core V1: Revolve 360° Selection (`crates/mesh/src/ops.rs`, `crates/core/src/command.rs`)**:
  - `Mesh::revolve_selection(&mut self, segments, angle_deg, axis, center)`: Revolução procedimental de perfis conectados/abertos selecionados em torno de qualquer eixo coordenado ($X, Y, Z$) com fechamento cíclico perfeito em revoluções de 360°.
  - `RevolveCmd`: Comando transacional parametrizado com `segments`, `angle_deg`, `axis` e `center`.
- **Refinamento de Ferramentas Geométricas Interativas (Gauntlet Loop)**:
  - **Extrude Individual Faces (`Alt+E`)**:
    - `Mesh::extrude_individual(&mut self, dist)`: Replicação desacoplada de vértices por face selecionada, gerando anéis de parede independentes e topos disjuntos sem colapso de arestas compartilhadas.
    - `ExtrudeIndividualCmd` e `ExtrudeTool::apply_individual(&mut state)` com suporte a atalho padrão da indústria `Alt+E`.
  - **Multi-segment Rounded Bevel**:
    - `bevel_edge_segments` em `crates/mesh/src/bevel.rs`: Chanfro transacional com $N \ge 1$ segmentos e perfil de curvatura circular em arco de filete ($\text{bulge}(t) = (1 - (2t-1)^2) \times 0.4142$).
    - Costura topológica automática de vértices intermediários nas faces de canto e extremidade, garantindo que o resultado permaneça fechado, 2-manifold e estritamente planar nas faces laterais.
  - **Guarded Metric Inset contra Auto-Interseção**:
    - `Mesh::inset_selected(&mut self, factor)` reforçado com verificação dinâmica de inversão de normais no polígono interno e amortecimento step-down para garantir estabilidade topológica sob fatores extremos.
- **Renderização e Interface do Usuário (`crates/ui/`, `crates/render-gl/`, `crates/render-wgpu/`, `crates/app/`)**:
  - Renderização de linhas de triangulação em tom ciano/azul suave (`[0.3, 0.65, 0.95]`) com offset de profundidade em ambos os backends OpenGL (`petunia_render_gl`) e WebGPU (`petunia_render_wgpu`).
  - Toggle dedicado de Inspeção de Triangulação na barra de contexto do Viewport (`crates/ui/src/viewport_bar.rs`) ao lado dos toggles de Overlays e Raio-X.
  - Itens de menu `Extrude Individual (Alt+E)`, `Flip Diagonal` e `Revolve Selection` integrados ao menu contextual `Mesh ▾`.
  - Atalho `Alt+E` para `model.extrude_individual` adicionado aos keybinds padrões e perfis `petunia.toml`, `petunia-default.toml` e `blender.toml`.
- **Suíte de Testes Automatizados**:
  - `crates/mesh/tests/triangulation_tests.rs`: 3 testes unitários para wireframe de triangulação e flip diagonal.
  - `crates/mesh/tests/interactive_geometry_tests.rs`: 4 testes de conformidade para extrude individual, revolve 360°, bevel multi-segmentos e inset protegido.
  - `crates/core/tests/command_tests.rs`: Testes de execução headless e Undo/Redo para `FlipDiagonalCmd`, `RevolveCmd` e `ExtrudeIndividualCmd`.

---

## [0.20.0] - 2026-09-13 — Architectural Decoupling: C-ABI / FFI Layer (Gauntlet G10, Cross-Language Frontends)

### Adicionado
- **Biblioteca Dinâmica e Estática C-ABI / FFI (`crates/ffi/`)**:
  - `petunia_ffi`: Compilado como `cdylib` (`libpetunia_ffi.so` / `.dll` / `.dylib`) e `rlib` para interoperabilidade direta com linguagens externas sem dependência de `egui`.
  - Header canônico C/C++ (`crates/ffi/include/petunia.h`): Definições de tipos opacos (`PetuniaContext`), códigos de retorno (`PETUNIA_OK`, `PETUNIA_ERR_*`) e protótipos de funções exportadas com documentação Doxygen.
  - Funções de ciclo de vida seguro de contexto: `petunia_context_create(lang)` e `petunia_context_destroy(ctx)`.
  - Funções de I/O de projeto e conversão de formatos: `petunia_new_project`, `petunia_load_project`, `petunia_save_project`, `petunia_import_obj`, `petunia_export_obj`, `petunia_export_glb`.
  - Funções de comandos de modelagem: `petunia_add_primitive`, `petunia_undo`, `petunia_redo`, `petunia_select_all`, `petunia_clear_selection`, `petunia_delete_selection`, `petunia_duplicate_selection`, `petunia_extrude_selection`, `petunia_subdivide_selection`, `petunia_scale_selection`.
  - Funções de consultas semânticas e telemetria: `petunia_get_asset_count`, `petunia_get_scene_summary`, `petunia_get_active_asset_name`, `petunia_get_active_asset_stats`.
  - Manipulação por UUIDs canônicos: `petunia_set_active_asset_by_id`, `petunia_delete_asset_by_id`.
  - Serialização JSON de DTOs para clientes de alto nível (Python, C#, JS): `petunia_query_scene_hierarchy_json`, `petunia_query_selection_details_json`, `petunia_query_tool_status_json`.
  - Diagnóstico seguro de erros: `petunia_last_error_message(buffer, len)`.
- **Suíte de Testes C-ABI de Ponta a Ponta (`crates/ffi/tests/c_abi_tests.rs`)**:
  - 7 testes automatizados exercitando segurança com ponteiros nulos, criação/destruição de contexto, despacho de comandos com undo/redo, consultas JSON, exportações para OBJ e GLB (com validação de magic bytes `glTF`) e manipulação por UUIDs estáveis.
- **Governança de Invariantes em CI (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - Validação estrita de que `crates/ffi` não depende de `egui` e de que o header C canônico está presente.
  - `cargo run -p xtask -- arch-check` atualizado confirmando a conclusão dos 11 Gauntlets arquiteturais (G0 a G10).

---

## [0.19.0] - 2026-09-13 — Architectural Decoupling: Application API Stabilization (Gauntlet G9, F-010)

### Adicionado
- **Application Query API e DTOs Imutáveis (`crates/core/src/queries.rs`)**:
  - `SceneObjectDto`: Snapshot imutável de objeto de cena contendo `id: Uuid`, `name: String`, contagem de vértices, faces, visibilidade, travamento e se é o objeto ativo.
  - `SceneHierarchyDto`: DTO de visão geral da cena contendo lista de `SceneObjectDto`, `active_id: Option<Uuid>`, total global de vértices e faces.
  - `SelectionDetailsDto`: Snapshot da seleção corrente contendo contagem de vértices/arestas/faces selecionados, centro ponderado da seleção 3D (`selection_center: Option<Vec3>`) e modo de seleção.
  - `ToolStatusDto`: Snapshot do estado atual da ferramenta ativa contendo nome da ferramenta, se há operação modal ativa e mensagem de status do editor.
- **Métodos de Consulta e Manipulação por Identificador Estável em `AppState`**:
  - `state.query_scene_hierarchy() -> SceneHierarchyDto`
  - `state.query_selection_details() -> SelectionDetailsDto`
  - `state.query_tool_status() -> ToolStatusDto`
  - `state.set_active_asset_by_id(id: Uuid) -> bool`
  - `state.delete_asset_by_id(id: Uuid) -> bool`
  - `state.find_asset_by_id(id: Uuid) -> Option<&Asset3D>`
  - `state.find_asset_by_id_mut(id: Uuid) -> Option<&mut Asset3D>`
- **Resiliência a Reordenação de Ativos por UUIDs (`crates/core/src/state.rs`)**:
  - Migração de `export_selected: Vec<usize>` para `export_selected: Vec<Uuid>`, eliminando fragilidade de índices instáveis em exportações seletivas.
  - Método auxiliar `export_selected_indices(&self) -> Vec<usize>` que resolve dinamicamente os índices com base na ordem atual de ativos no projeto.
- **Suíte de Testes Automatizados da Application API (`crates/core/tests/queries_tests.rs`)**:
  - 5 testes cobrindo geração de DTOs, cálculos de centro de seleção, resiliência de UUIDs frente a remoções intermediárias de objetos e ativação/remoção segura por identificadores únicos.
- **Governança de Invariantes em CI (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - Validação estrita da presença dos DTOs de queries e do armazenamento por `Uuid` em `export_selected` e na árvore do `outliner`.

---

## [0.18.0] - 2026-09-13 — Architectural Decoupling: Headless Sovereignty (Gauntlet G8, F-011)

### Adicionado
- **Utilitário de Linha de Comando `petunia-cli` (`crates/cli/`)**:
  - `petunia-cli new <arquivo.petunia> [primitiva]`: Cria projetos limpos com primitivas canônicas (`Cube`, `Plane`, `Sphere`, `Cylinder8`, `Capsule`).
  - `petunia-cli info <arquivo>`: Inspeciona assets, hierarquia, totais de vértices/faces e coleções de arquivos `.petunia` e `.obj`.
  - `petunia-cli convert <entrada> <saida>`: Converte formatos 3D bidirecionalmente entre `.petunia`, `.obj` e `.glb`.
  - `petunia-cli transform <entrada.petunia> <saida> [opções]`: Aplica pipelines de comandos e ferramentas (`--select-all`, `--extrude`, `--subdivide`, `--scale`, `--add-primitive`) sem interface gráfica.
  - `petunia-cli bench`: Benchmark integrado que executa um ciclo completo de 5 primitivas, extrusão, undo/redo, salvamento e exportações em menos de 80 milissegundos.
- **Suíte de Testes de Integração Headless (`crates/cli/tests/headless_integration.rs`)**:
  - 5 testes automatizados de ponta a ponta validando persistência, despacho de comandos com undo/redo, ferramentas de modelagem, exportação para OBJ e GLB (com validação de magic bytes `glTF`) e tempo de resposta (< 25ms).
- **Governança de Fitness Arquitetural em CI (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - Validação estrita de que `crates/cli` não depende nem referencia `egui`.
  - Validação integrada ao comando `cargo run -p xtask -- arch-check`.

---

## [0.17.0] - 2026-09-13 — Architectural Decoupling: Module Crates Purification (Gauntlet G7, F-008)

### Adicionado
- **Módulo de Apresentação de Módulos em UI (`crates/ui/src/modules_ui/`)**:
  - `model_ui.rs`: Renderização de painéis contextuais para todas as 15 ferramentas de modelagem (`Select`, `Transform`, `Primitives`, `Extrude`, `Inset`, `Bevel`, `PushPull`, `Subdivide`, `Slice`, `Mirror`, `Connect`, `Merge`, `Dissolve`, `DrawProfile`, `Paint`).
  - `paint_ui.rs`: Painel de pintura com controle de textura, paleta e pincel interativo 2D.
  - `uv_ui.rs`: Canvas 2D interativo com projeção ortográfica UV, renderização de polígonos UV e detecção de clique/arrasto.
- **Governança Automatizada de Invariantes em Fitness e CI (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - `module_crates_must_not_depend_on_egui`: Valida que `module-model`, `module-paint`, `module-uv` e `module-assets` não possuem dependência de `egui` em seus manifestos `Cargo.toml`.
  - `module_crates_sources_must_not_reference_egui`: Varredura recursiva de código-fonte garantindo ausência de `use egui` ou `egui::` em todos os crates `module-*`.
  - Integrado ao comando `cargo run -p xtask -- arch-check`.

### Modificado
- **Purificação Completa de Crates de Módulo (`module-*`)**:
  - `crates/module-model`: Removido método `ui()` do trait `Tool` e de todas as implementações. Exposição pública de métodos de serviço geométrico (`PrimitivesTool::add_primitive`, `TransformTool::apply_*`, `ExtrudeTool::apply`, `InsetTool::apply`, `BevelTool::apply`, `PushPullTool::apply`, `SubdivideTool::apply_*`, `SliceTool::apply_slice`, `MirrorTool::apply`, `ConnectTool::apply`, `MergeTool::apply`, `DissolveTool::apply`, `draw_profile::generate_*`). Removida a dependência `egui` do `Cargo.toml`.
  - `crates/module-paint`: Removido estado efêmero de UI (`canvas_tex`) da struct `PaintModule`. Removido método `ui()`. Removida a dependência `egui` do `Cargo.toml`.
  - `crates/module-uv`: Removido método `ui()`. Exposto cálculo puro de interseção e amostragem via `uv_hit`. Removida a dependência `egui` do `Cargo.toml`.
  - `crates/module-assets`: Removido método `ui()`. Removida a dependência `egui` do `Cargo.toml`.
- **Desacoplamento do Painel de Propriedades (`crates/ui/src/properties_panel.rs`)**:
  - Abas `Paint`, `Uv` e `Tool` delegadas para `modules_ui::{paint_ui, uv_ui, model_ui}` sem acoplamento a métodos internos de UI dos módulos de domínio.

---

## [0.16.0] - 2026-09-13 — Architectural Decoupling: AppState God Object Decomposition (Gauntlet G6, F-002)

### Adicionado
- **Segregação do God Object `AppState` em Sub-estados Coesos (`crates/core/src/state.rs`)**:
  - `ProjectState`: Modelo de domínio puro, histórico transacional undo/redo, paleta de cores e imagens de referência (`project`, `undo`, `palette`, `refs`, `project_path`, `export_selected`, `export_gltf`).
  - `EditorSession`: Câmera de visualização, transformações, modos de seleção, visibilidade de overlays e configurações de visualização de viewport (`selection`, `mode`, `workspace`, `select_mode`, `shading`, `textured`, `camera`, `camera_frame`, `cursor_3d`, `locked_axes`, `snap_enabled`, `proportional_editing`, `transform_orientation`, `pivot_point`, `show_overlays`, `show_xray`).
  - `ToolState`: Sessões de ferramentas ativas e parâmetros voláteis (`active_tool`, `gizmo_mode`, `modal`, `pending_modal`, `pointer_session`, `cut_session`, `mesh_preview`, `paint_*`, `extrude_dist`, `profile`, `uv_selected`, anotações e medições).
  - `UiState`: Estado de apresentação visual, preferências de interface e modais (`viewport_rect`, `viewport_pixels_per_point`, `outliner_search`, `properties_tab`, `show_settings`, `settings_tab`, `show_asset_library`, `show_asset_browser`, `show_help`, `show_perf`, `active_theme_id`, `active_icon_pack_id`, `active_keymap_id`, `i18n`, `keybinds`, `timeline_*`, `status`, `box_select_start`, `pending_pick`, `context_menu_pos`).
  - `RenderResources`: Telemetria de backend de renderização e dirty flags de GPU (`dirty`, `canvas_dirty`, `backend_name`, `stats`).
- **Delegação Ergonômica transparente via `Deref` e `DerefMut`**:
  - `AppState -> EditorSession -> ToolState`: Permite que acessos a propriedades de sessão e ferramentas permaneçam ergonômicos sem quebras de compatibilidade sintática (`state.mode`, `state.camera`, `state.active_tool`).
  - `ProjectState -> Project`: Permite acesso direto aos métodos do modelo de malha (`state.project.assets`, `state.project.active_mesh()`).
- **Suíte de Testes de Desacoplamento de Estado (`crates/core/tests/state_decomposition_tests.rs`)**:
  - 6 testes unitários que exercitam mutações simultâneas de sub-estados isolados sem locks globais, validando a autonomia de ciclo de vida de `ProjectState`, `EditorSession`, `ToolState`, `UiState` e `RenderResources`.
- **Governança Automatizada de Invariantes em Fitness e CI (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - Validação estrita de que nenhum campo de apresentação (ex: `viewport_rect`, `stats`, `show_settings`, `keybinds`, `canvas_dirty`) vaza para as estruturas de domínio (`ProjectState`, `EditorSession`, `ToolState`).

### Modificado
- **Erradicação do Dual-Storage de Paletas em `crates/module-paint`**:
  - A paleta de cores reside exclusivamente no modelo de domínio (`state.project.palette`).
- **Alinhamento de Chamadas em todo o Workspace**:
  - Atualização dos módulos `crates/module-assets`, `crates/module-model`, `crates/module-uv`, `crates/render-gl`, `crates/ui` e `crates/app` para consumir os sub-estados segregados.

---

## [0.15.0] - 2026-09-13 — Architectural Decoupling: Tool Sessions Decoupling (Cutting, Modal & Math Normalization) (Gauntlet G5)

### Adicionado
- **Máquina de Estado Pura de Ferramentas de Corte `CutSession` (`crates/core/src/cutting_session.rs`)**:
  - Encapsula o ciclo de vida transacional das ferramentas `Knife`, `Slice` e `Loop Cut` sem qualquer dependência de `egui`.
  - Campos neutros: `source` (malha original), `anchor: Option<[f32; 2]>`, `edge_start: Option<EdgePoint>`, `ring: Option<LoopRing>`, `cuts: usize`, `sliding: bool`.
  - Métodos canônicos: `adjust_cuts(delta)`, `cut_knife_segment(...)`, `compute_slice(...)`, `compute_loop_slide(current_x)`, `preview_loop_lines(slide)`, `apply_loop_cut(slide)`.
  - 6 testes unitários e de integração headless em `crates/core/tests/cutting_session_tests.rs`.
- **Máquina de Estado Pura de Interação por Ponteiro `PointerSession` (`crates/core/src/modal.rs`)**:
  - Buffer de entrada numérica via teclado com parsing seguro (`parse_numeric()`), âncora de tela agnóstica (`[f32; 2]`) e rastreamento de última posição.
  - Limpeza atômica automática vinculada a `commit_modal()` e `cancel_modal()` no domínio.
- **Campos de Sessão de Primeira Classe em `AppState` (`crates/core/src/state.rs`)**:
  - `pub cut_session: Option<CutSession>` e `pub pointer_session: Option<PointerSession>`.
  - Integração com `finish_mesh_preview`: encerramento de preview (por commit ou cancelamento) reseta atomicamente `cut_session`.
- **Normalização Matemática de Viewport em `petunia_core::viewport` (`crates/core/src/viewport.rs`)**:
  - Métodos geométricos canônicos em `LogicalRect`: `screen_to_ndc`, `ndc_to_screen`, `project_point`, `ray`.
  - Funções canônicas de desprojeção e snapping: `unproject_to_surface_or_cursor_plane` e `unproject_cursor_or_vertex_snap`.
- **Governança de Sessões de UI em Fitness e CI (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - Scanner automatizado que rejeita a reintrodução de `Id::new("cut.session")` ou `Id::new("modal.pointer")` em memória temporária de UI.

### Modificado
- **Erradicação de Memória Temporária de UI em `crates/ui/src/cutting.rs`**:
  - Remoção de `struct CutSession` privada e chamadas `ctx.data_mut` (`insert_temp`, `get_temp`, `remove`).
  - A ferramenta agora opera diretamente sobre `state.cut_session`, delegando o cálculo do plano de corte, deslizamento de loop e aplicação de anéis aos métodos puros do domínio.
- **Erradicação de Memória Temporária de UI em `crates/ui/src/modal_viewport.rs`**:
  - Remoção de `struct PointerSession` privada e chamadas `ctx.data_mut`.
  - `start_handle` e o loop `draw` agora operam exclusivamente com `state.pointer_session`.
- **Deduplicação Matemática em `crates/ui/src/annotation.rs` e `crates/ui/src/measurement.rs`**:
  - Remoção de implementações duplicadas de projeção de tela, conversão NDC e raycasting manual em favor das funções canônicas de `petunia_core::viewport`.
- **`LoopRing` e `RingFace` em `crates/mesh/src/loop_cut.rs`**:
  - Derivação de `Debug` implementada em `LoopRing` e `RingFace`.

---

## [0.14.0] - 2026-09-13 — Architectural Decoupling: Pure ProjectService & File I/O Boundary (Gauntlet G4)

### Adicionado
- **Serviço Puro de Aplicação `ProjectService` (`crates/core/src/project_service.rs`)**:
  - Implementação de serviço de aplicação canônico desacoplado de interfaces gráficas para ciclo de vida de projeto:
    - `ProjectService::new_project`: reinicialização de sessão e sincronização transacional de estado.
    - `ProjectService::load_project` e `ProjectService::save_project`: persistência determinística com versionamento `.petunia`.
    - `ProjectService::import_obj` e `ProjectService::export_obj`: importação e exportação de malhas Wavefront OBJ com criação de checkpoint de undo.
    - `ProjectService::export_glb`: exportação glTF binário com empacotamento nativo.
    - `ProjectService::export_all_obj_to_dir`: exportação em lote para diretório com sanitização de nomes de arquivo.
    - `ProjectService::import_palette` e `ProjectService::export_palette`: importação e exportação de paletas nos formatos GIMP Palette (`.gpl`) e hexadecimal (`.hex`).
    - `ProjectService::add_reference_image`: inserção de imagens de referência na cena com tipagem neutra.
  - Tipagem de erro estruturada `ProjectServiceError` (`Io`, `Format`, `AssetNotFound`, `Export`, `InvalidPalette`).
  - 9 novos testes unitários e de integração headless em `crates/core/tests/project_service_tests.rs`.
- **Eventos de Solicitação de Diálogo de Paleta no EventBus (`crates/core/src/events.rs`)**:
  - Adição de `AppEvent::RequestImportPalette` e `AppEvent::RequestExportPalette`.
  - Tratamento assíncrono e desacoplado no loop de despacho do `Core` (`crates/app/src/lib.rs`).
- **Governança Automatizada de I/O e Isolamento de Diálogos (`tests/architecture_fitness.rs`, `crates/xtask/src/main.rs`)**:
  - Teste automatizado garantindo que `petunia_module_paint` não depende de `rfd`.
  - Scanner de código estático garantindo que nenhuma função fora de `crates/ui/src/file_dialog_service.rs` instancia `rfd::FileDialog` ou `egui_file_dialog::FileDialog`.

### Modificado
- **Purificação de `petunia_module_paint` (`crates/module-paint/Cargo.toml`, `crates/module-paint/src/lib.rs`)**:
  - Remoção completa da dependência externa `rfd`.
  - Substituição de chamadas a diálogos nativos nos botões de importação e exportação de paleta por emissão de eventos no `EventBus`.
  - Funções utilitárias puras `import_palette_file` e `export_palette_file` delegando diretamente ao `ProjectService`.
- **Centralização Canônica de Diálogos de Arquivo (`crates/ui/src/file_dialog_service.rs`)**:
  - `PetuniaFileDialogService` agora delega todas as ações de domínio confirmadas ao `ProjectService`.
  - Adicionadas funções auxiliares no módulo `native` (`pick_project_file`, `pick_save_project_file`, `pick_obj_file`, `pick_export_obj_file`, `pick_export_glb_file`, `pick_folder`, `pick_image_file`, `pick_palette_import_file`, `pick_palette_export_file`).
- **Remediação de `crates/ui/src/lib.rs`**:
  - Funções `open_project_dialog`, `save_project_dialog`, `import_obj_dialog`, `export_dialog`, `refs_section` e `pick_and_add_reference_image` refatoradas para utilizar `file_dialog_service` e `ProjectService`.
  - Remoção das importações de `petunia_mesh::Mesh` e `petunia_project::format` do escopo raiz da UI.

---

## [0.13.0] - 2026-09-13 — Architectural Decoupling: Core Purification, Fitness Governance, Command System, and UI Direct Mutation Extraction (Gauntlets G0, G1, G2, G3)

### Adicionado
- **Governança Arquitetural Contínua (Gauntlet G0)**:
  - Testes de fitness automatizados em `tests/architecture_fitness.rs` garantindo que `petunia_core`, `petunia_config`, `petunia_mesh`, `petunia_project`, `petunia_commands` e `petunia_render_wgpu` nunca dependam do `egui` ou `petunia_ui`.
  - Automação integrada em `cargo xtask arch-check` para validação em CI e pre-commit de manifestos e relatórios canônicos de auditoria.
  - 17 relatórios canônicos de auditoria arquitetural profunda em `docs/audits/architecture-decoupling/` (121+ KB).
- **Purificação Total do Core e Configuração (Gauntlet G1)**:
  - Remoção de 100% das dependências e símbolos de `egui` de `crates/core` e `crates/config`.
  - Introdução do tipo agnóstico `LogicalRect` em `petunia_core::viewport` substituindo `egui::Rect`.
  - Extração de `apply_theme` para `crates/ui/src/theme_adapter.rs`.
  - Isolamento de handles de textura específicos de backend gráfico em `petunia_ui` e `petunia_module_paint`.
- **Fundação do Sistema de Comandos e Dispatcher (Gauntlet G2)**:
  - Trait genérica e desacoplada `Command` em `petunia_commands::Command` e `petunia_core::command::Command`.
  - Despachante transacional `CommandDispatcher` com registry dinâmico, auto-checkpointing de Undo/Redo antes de mutações destrutivas e disparo determinístico de eventos e flags de sujeira (`mark_dirty()`, `emit_mesh_changed()`).
  - Implementação de 8 comandos canônicos essenciais (`AddPrimitiveCmd`, `DuplicateAssetCmd`, `DeleteAssetCmd`, `DeleteSelectionCmd`, `DuplicateSelectionCmd`, `SelectAllCmd`, `ClearSelectionCmd`, `InvertSelectionCmd`).
  - Suíte completa de 7 testes de integração headless em `crates/core/tests/command_tests.rs` validando Undo/Redo roundtrip e uma sessão de modelagem completa sem carregar qualquer backend de interface gráfica.
- **Extração de Mutações Diretas da Camada UI (Gauntlet G3)**:
  - Novos comandos canônicos de malha: `SubdivideSelectionCmd`, `MergeCenterCmd` e `FlipNormalsCmd`.
  - Eliminação de mutações diretas de mesh e chamadas manuais a `state.checkpoint()` nos painéis de UI:
    - `crates/ui/src/properties_panel.rs`: Ações de duplicar e deletar agora despacham `DuplicateSelectionCmd` e `DeleteSelectionCmd`.
    - `crates/ui/src/outliner.rs`: Exclusão e duplicação de assets e adição de primitivas migradas para `DeleteAssetCmd`, `DuplicateAssetCmd` e `AddPrimitiveCmd`.
    - `crates/ui/src/viewport_bar.rs`: Menus `Select ▾`, `Add ▾`, `Object ▾` e `Mesh ▾` migrados para despachar comandos.
    - `crates/ui/src/contextual_shelf.rs`: Ações da shelf flutuante (duplicação, subdivisão e merge) migradas para o despachante.
    - `crates/ui/src/nav_gizmo.rs`: Ações do menu contextual da viewport (subdivide, flip normals, duplicate) migradas para comandos.
    - `crates/ui/src/asset_browser.rs` e `crates/ui/src/asset_library_drawer.rs`: Operações de assets migradas para comandos.
    - `crates/module-model/src/select.rs`: Ações da ferramenta de seleção migradas para comandos.
    - `crates/app/src/lib.rs`: Atalhos de teclado em `Core::on_key` (`model.delete`, `model.duplicate`, `model.select_all`, `model.deselect_all`, `model.invert_selection`) conectados diretamente ao despachante semântico.
  - Aprovação integral dos 88 testes de UI (incluindo 5 fluxos `egui_kittest`) e total de 215 testes da workspace.

## [0.12.0] - 2026-09-13 — Deep Interface Revision: Canonical Vector Iconography, Deduplicated Controls, Unified Menus, and Blender-Standard Properties & Outliner

### Adicionado
- **Iconografia Vetorial Canônica e Eliminação Total de Emojis (`crates/ui/src/icons.rs`, `crates/ui/src/icon_registry.rs`)**:
  - Implementação de mais de 20 novos ícones vetoriais procedurais via egui Painter (`draw_eye_open`, `draw_eye_closed`, `draw_lock_locked`, `draw_lock_unlocked`, `draw_duplicate`, `draw_trash`, `draw_add`, `draw_annotate`, `draw_measure`, `draw_reference_image`, `draw_primitive_cube`, `draw_primitive_cylinder`, `draw_primitive_sphere`, `draw_primitive_plane`, `draw_primitive_cone`, `draw_primitive_capsule`, `draw_chevron_right`, `draw_filter`, `draw_collection`, `draw_object`).
  - Mapeamento completo no enum semântico `PetuniaIcon` e no `IconRegistry`.
  - Erradicação de todos os emojis Unicode na interface (`🧊`, `🕸`, `📋`, `🗑`, `📝`, `🖼`, `⌖`, `⚪`, `🛢`, `▭`, `▲`, `🔒`, `🔓`, `⚙`, `📦`, `🔍`, `➕`, `🎯`) em favor de ícones vetoriais nítidos que respeitam o DPI, tema ativo e tokens de cor.
- **Widgets Padronizados de Menu e Ação (`crates/ui/src/widgets.rs`, `crates/config/src/keybinds.rs`)**:
  - `PetuniaMenuItem`: Layout profissional padrão Blender `[Ícone] Rótulo ... [Atalho] ›` com alinhamento dinâmico e separadores estilizados (`petunia_menu_separator`).
  - Método `shortcut_for(&self, action: &str)` no `Keybinds` para resolução em tempo de execução dos atalhos do perfil ativo.
  - `petunia_action_button`: Botão compacto de ação com ícone vetorial opcional, feedback hover/active e suporte a estilo perigoso/destrutivo.
- **Reorganização Estrutural da Barra Superior da Viewport (`crates/ui/src/viewport_bar.rs`)**:
  - Divisão em 7 clusters funcionais responsivos:
    - *Cluster 1*: Seletor de Modo de Interação (Object / Edit / Paint) com pílula de destaque.
    - *Cluster 2*: Modos de Seleção de Malha (Vértice, Aresta, Face) com atalhos numéricos canônicos `1`, `2`, `3`.
    - *Cluster 3*: Transformação, Pivot e Travamento de Eixos (`X`, `Y`, `Z`).
    - *Cluster 4*: Controles de Câmera da Viewport (Vistas axiais, Projeção, Enquadramento, Reset).
    - *Cluster 5*: Snapping Magnético e Edição Proporcional.
    - *Cluster 6*: Overlays e Modo Raio-X.
    - *Cluster 7*: Modos de Sombreamento esféricos estilo Blender (Wireframe, Solid, Material Preview, Rendered).
- **Outliner e Painel de Propriedades Refinados (`crates/ui/src/outliner.rs`, `crates/ui/src/properties_panel.rs`)**:
  - Outliner: Remoção de botões textuais volumosos no cabeçalho em favor de botões de ícone compactos; ícones vetoriais por tipo de nó; botões reutilizáveis de visibilidade (`👁`) e bloqueio (`🔒`).
  - Properties: Substituição das 5 cores arco-íris das abas por tokens semânticos (`tokens::ACCENT_BLUE`); inspetor de Transform completo (Location X/Y/Z, Rotation X/Y/Z em graus, Scale); botão de duplicar e ação de deletar estilizada em vermelho.
- **Unificação e Fonte Única da Verdade (`crates/ui/src/contextual_shelf.rs`)**:
  - Remoção de controles duplicados de seleção de vértices/arestas/faces da shelf flutuante contextual, centralizando os modos exclusivamente no cabeçalho do viewport.
  - Eliminação de rótulos bilíngues de depuração (`Posição (Location)`, `Escala (Scale)`) em prol de nomenclatura limpa e consistente.

## [0.11.0] - 2026-09-13 — Petunia3D Living Documentation Website, xtask Automation, and GitHub Actions CI/CD

### Adicionado
- **Website Oficial de Documentação com VitePress (`docs/`)**:
  - Portal estático completo, ultraveloz, responsivo e com busca local offline integrado via VitePress e plugin Mermaid.
  - Landing page oficial (`docs/index.md`) com hero dinâmico, proposta de valor, grade de recursos e diagramas de fluxo de criação.
  - Seção **Primeiros Passos (`docs/getting-started/`)**: Introdução conceitual, requisitos, instalação/compilação, ciclo de vida de projetos `.petunia`, tour da interface e tutorial prático de 15 minutos modelando um caixote estilizado.
  - Seção **Manual do Usuário (`docs/manual/`)**: 11 capítulos detalhados cobrindo interface, viewport 3D, linhas-guia de travamento de eixos, modos de seleção, modelagem shape-first, pintura, mapeamento UV, animação, biblioteca de assets, projetos e exportação.
  - Seção **Workspaces (`docs/workspaces/`)**: Guias completos dos 4 espaços de trabalho (Modeling, Paint, UV, Animation).
  - Seção **Catálogo de Ferramentas (`docs/tools/`)**: Documentação exaustiva das 19 ferramentas de criação, modelagem, medição e anotação.
  - Seção **Personalização (`docs/customization/`)**: Documentação dos 4 temas visuais em TOML, 5 pacotes de ícones vetoriais, internacionalização (i18n) e 8 perfis de keymaps.
  - Seção **Atalhos (`docs/shortcuts/`)**: Tabela mestre condensada (Cheatsheet) e guia de equivalência 1:1 para usuários do Blender.
  - Seção **Portal do Desenvolvedor (`docs/developers/`)**: Macroarquitetura de 15 crates modulares, pipeline de renderização híbrido WebGPU/OpenGL, padrão de comandos transacionais, plugins dinâmicos, protocolo MCP, estratégia de testes e guia de contribuição.
  - Seção **Changelog (`docs/changelog/`)**: Espelho interativo sincronizado com o histórico de versões.
- **Crate de Automação de Tarefas e Prevenção de Drift (`crates/xtask`)**:
  - Utilitário Rust integrado no workspace (`cargo xtask docs` e `cargo xtask docs-check`).
  - Validação estrita de integridade de todos os 28 arquivos canônicos e build determinístico do VitePress.
- **Pipeline CI/CD no GitHub Actions (`.github/workflows/docs.yml`)**:
  - Compilação automatizada com Node 20, pnpm 9 e deploy contínuo para o GitHub Pages.

## [0.10.0] - 2026-09-13 — Viewport Axis Locking: 3D Guide Lines, Real-Time HUD, and Viewport Bar Controls

### Adicionado
- **Linhas-Guia 3D Infinitas no Viewport (`crates/ui/src/modal_viewport.rs`, `crates/ui/src/viewport_interaction.rs`)**:
  - Renderização de linhas-guia 3D brilhantes atravessando o pivô da seleção de ponta a ponta da tela quando um eixo cartesiano é travado (`X`, `Y`, `Z`).
  - Cores canônicas de alta visibilidade do Blender (`AXIS_X` vermelho `#e03c42`, `AXIS_Y` verde `#62c934`, `AXIS_Z` azul `#3182f6`).
  - Efeito halo/glow (`Stroke(6.0px)`) com núcleo sólido (`Stroke(2.0px)`) garantindo legibilidade perfeita sobre qualquer geometria ou grid de fundo.
  - Suporte completo a planos coordenados (`Shift+X` para YZ, `Shift+Y` para XZ, `Shift+Z` para XY): traçado simultâneo dos dois eixos do plano e polígono translúcido estilizado preenchendo a região de transformação.
  - Ativação imediata também ao arrastar eixos em gizmos de malha e anotações.
- **HUD Flutuante de Alta Visibilidade no Viewport (`crates/ui/src/modal_viewport.rs`)**:
  - Cápsula/pill estilizada acompanhando o cursor de edição com fundo translúcido escuro e borda na cor do eixo travado.
  - Badge semântico de status: `[ 🔒 EIXO X ]`, `[ 🔒 EIXO Y ]`, `[ 🔒 EIXO Z ]`, `[ 🔒 PLANO YZ (Shift+X) ]`, `[ 🔒 PLANO XZ (Shift+Y) ]`, `[ 🔒 PLANO XY (Shift+Z) ]`, ou `[ 🔓 LIVRE ]`.
  - Exibição de valores numéricos digitados diretamente e guia de atalhos (`X/Y/Z: travar eixo · Shift: plano · Ctrl: snap`).
- **Controles e Indicadores de Eixo na Barra da Viewport (`crates/ui/src/viewport_bar.rs`)**:
  - Grupo dedicado no Cluster 3 de Transformação: `🔒 [ X ] [ Y ] [ Z ]`.
  - Botões interativos de alternância rápida com preenchimento sólido colorido quando ativos e estado neutro quando livres.
  - Badge dinâmico estilizado (`[ 🔒 Eixo X ]`, etc.) indicando o travamento ativo para feedback inequívoco com um único relance.
  - Capacidade bidirecional: alternar eixos durante a edição ou pré-configurar eixos antes de iniciar uma transformação modal.
- **Sincronização de Estado e Arquitetura no Núcleo (`crates/core/src/state.rs`, `crates/core/src/modal.rs`)**:
  - Campo `locked_axes: [bool; 3]` integrado no `AppState` com métodos `is_axis_locked`, `active_axis_constraint_label` e `toggle_axis_lock`.
  - Herança automática de restrições em `begin_modal` e sincronização bidirecional em tempo de execução.
  - Limpeza e reset limpo ao finalizar ou cancelar operações modais (`commit_modal` / `cancel_modal`).

## [0.9.0] - 2026-09-13 — Annotations & Measurements: Undo/Redo (Ctrl+Z), Dedicated Outliner Collections, Subgrouping, Strict Confinement, and Transform Properties

### Adicionado
- **Undo/Redo Transacional para Anotações e Medidas (`Ctrl+Z` / `Ctrl+Shift+Z`) (`crates/project/src/lib.rs`, `crates/core/src/state.rs`, `crates/ui/src/annotation.rs`, `crates/ui/src/measurement.rs`)**:
  - Migração de `annotations` e `measurements` de estruturas transitórias soltas para o domínio de dados persistente `Project`.
  - Checkpoint automático a cada traço finalizado, medição completada ou item excluído via `state.checkpoint()`.
  - Desfazer e refazer completos, imediatos e estáveis com `Ctrl+Z` e `Ctrl+Shift+Z` restaurando perfeitamente os traços e réguas.
- **Coleção Especializada `📝 Anotações` no Topo do Outliner (`crates/ui/src/outliner.rs`)**:
  - Posicionamento canônico no topo da árvore de cena, acima das coleções de malhas.
  - Identidade visual distinta com ícone `📝` e cor ciano característica (`#00d2d3`).
  - Controles coletivos e por item: alternância de visibilidade (`👁` / `⊘`) e alternância de bloqueio (`🔒` / `🔓`).
  - Suporte a múltiplos subgrupos internos (`📁 Subgrupo`) com menus contextuais para mover anotações entre subgrupos ou para a raiz da coleção.
  - **Confinamento Estrito**: Anotações residem exclusivamente na coleção de Anotações e não podem ser movidas ou mescladas em coleções de malhas 3D.
- **Coleção Especializada `📏 Medidas` no Outliner (`crates/ui/src/outliner.rs`)**:
  - Posicionamento canônico no topo da árvore de cena com ícone `📏` e cor amarela de destaque (`#feca57`).
  - Controles estritamente limitados a ocultar/exibir (`👁` / `⊘`) e exclusão (`🗑` / `X`), sem suporte a bloqueio ou transformações, conforme especificado.
- **Inspetor e Propriedades de Transformação de Anotações no Painel de Propriedades (`crates/ui/src/properties_panel.rs`)**:
  - Exibição automática das propriedades ao selecionar uma anotação na árvore ou após desenhá-la.
  - Campos de identificação (nome editável, visibilidade, bloqueio e atribuição de subgrupo).
  - Aparência do traço: seletor de cor RGBA e controle deslizante de espessura de traço.
  - **Seção de Transformação Completa**:
    * Posição (Location): `X`, `Y`, `Z` com badges semânticos coloridos.
    * Rotação (Rotation): `X`, `Y`, `Z` em graus de Euler.
    * Escala (Scale): `X`, `Y`, `Z` com alcance de `0.01..=100.0`.
    * Botão de redefinição de transformação (`↺ Redefinir Transformação`).
  - Ação de exclusão direta com botão `🗑 Deletar Anotação`.
- **Manipulação Direta por Gizmo no Viewport 3D (`crates/ui/src/viewport_interaction.rs`)**:
  - Gizmos tridimensionais (Mover, Rotacionar, Escalar) acoplados ao centro geométrico da anotação selecionada.
  - Suporte a arrasto de eixos e planos com cancelamento por `Escape` e gravação de checkpoint transacional ao soltar o mouse.
  - Respeito integral ao bloqueio individual da anotação e bloqueio global da coleção.

## [0.8.0] - 2026-09-12 — UI Enhancements & Interactions: Vertex Hover Demarcation, Toolbar Edit Tools, WGPU X-Ray, Reference Images, Outliner Collections/Lock/Isolate, and Vibrant Properties Tabs

### Adicionado
- **Demarcação Visual de Vértices no Modo de Edição (`crates/ui/src/viewport_interaction.rs`)**:
  - Quando em `EditMode::Edit` com `SelectMode::Vertex`, todos os vértices da malha ativa são desenhados de forma proeminente (pontos laranjas para selecionados, pontos escuros com contorno claro para não selecionados).
  - Hover dinâmico sobre vértices desenha um ponto interno dourado e um anel/halo externo ciano brilhante (`#64dcff`), garantindo feedback imediato de que o modo de vértices está ativo e indicando qual vértice será selecionado antes do clique.
- **Ferramentas de Modelagem na Barra de Ferramentas Esquerda (`crates/ui/src/toolbar.rs`)**:
  - Ao entrar em `EditMode::Edit`, a barra vertical esquerda expande automaticamente para exibir a paleta completa de 9 ferramentas de modelagem de malha (`Extrude`, `Inset`, `Bevel`, `Loop Cut`, `Knife`, `Push/Pull`, `Slice`, `Subdivide`, `Draw Profile`), em paralelo com a barra flutuante inferior.
- **Renderização e Picking em Modo Raio-X (`crates/render-wgpu/src/lib.rs`, `crates/app/src/lib.rs`, `crates/core/src/state.rs`)**:
  - Pipeline de shader WGSL `fs_xray` com translucidez (`alpha ~ 0.45`), `depth_write_enabled: false` e comparação de profundidade `LessEqual`.
  - Pipeline de arestas X-Ray sem teste de oclusão de profundidade (`CompareFunction::Always`), permitindo que as arestas sejam visíveis através de qualquer geometria.
  - O algoritmo de picking passa a considerar `state.show_xray`, permitindo selecionar vértices, arestas e faces ocultos atrás da superfície quando o modo Raio-X estiver ativado.
  - Atalho canônico `Alt+Z` para alternar modo Raio-X.
- **Reintegração Completa de Imagens de Referência (`crates/ui/src/lib.rs`, `viewport_bar.rs`, `contextual_shelf.rs`, `outliner.rs`)**:
  - Abertura assíncrona/nativa via `pick_and_add_reference_image` disponível tanto no menu `➕ Add+ ▾` da viewport bar quanto na barra contextual flutuante de baixo (`🖼 Referência`).
  - Seção dedicada `🖼 Imagens de Referência` no Outliner com controle de visibilidade (`👁` / `⊘`), alternância de Raio-X (`⚡`) e remoção.
- **Hierarquia de Pastas / Coleções no Outliner (`crates/project/src/lib.rs`, `crates/ui/src/outliner.rs`)**:
  - Capacidade de criar pastas/coleções (`📁 Coleções`) no cabeçalho do Outliner através do botão `📁+ Pasta`.
  - Suporte a agrupar modelos em coleções, renomeação inline, exclusão com retorno automático de itens à raiz, e alternância em lote de visibilidade e bloqueio.
  - Menu contextual nos objetos: `📁 Mover para Coleção ▾` (listando coleções existentes e raiz).
- **Bloqueio (`Lock`) e Isolamento (`Isolate`) de Modelos (`crates/project/src/lib.rs`, `crates/core/src/state.rs`, `crates/core/src/modal.rs`, `crates/ui/src/outliner.rs`)**:
  - Campo `asset.locked: bool` persistido no projeto.
  - Botão `🔒` / `🔓` no Outliner para fixar objetos, impedindo qualquer transformação modal (`ModalError::ActiveLocked`), manipulação por gizmo ou menus contextuais no viewport.
  - Botão e modo `⌖ Isolar` (atalho `Numpad /` ou `/`) que oculta temporariamente todos os outros modelos mantendo apenas o selecionado em visão local, com restauração perfeita do estado de visibilidade anterior ao desativar.
- **Abas de Propriedades Ampliadas e Coloridas Semanticamente (`crates/ui/src/widgets.rs`, `crates/ui/src/properties_panel.rs`)**:
  - Botões de categoria ampliados para `32x28px`, emoldurados em container estilizado (`tokens::BG_PANEL_HEADER`).
  - Cores semânticas vibrantes inspiradas no Blender:
    * `Tool`: Azul canônico (`#3169e3`)
    * `Object`: Laranja característico (`#e67e22`)
    * `Modifiers`: Azul-celeste (`#00a8ff`)
    * `Data`: Verde (`#2ecc71`)
    * `Material`: Magenta / Rosa (`#e84393`)
  - Indicador inferior de seleção ativa e realce refinado ao passar o cursor.

## [0.7.0] - 2026-09-12 — UI Reorganization & Ergonomics Refinement: Contextual Modeling Shelf, Retractable Asset Browser, Clean Two-Panel Sidebar & Viewport Bar 6 Clusters

### Adicionado
- **Barra Contextual Horizontal do Viewport (`crates/ui/src/contextual_shelf.rs`)**:
  - Cápsula flutuante na base inferior do Viewport 3D reagindo dinamicamente ao workspace e ao modo ativo (`Model + Edit`, `Model + Object`, `Paint`, `UV`, `Animate`).
  - Em `Model + Edit`: botões de seleção de malha (`⬝ Vértice`, `╱ Aresta`, `▨ Face`), comandos essenciais (`Extrude`, `Inset`, `Bevel`, `Loop Cut`, `Knife`) e operações topológicas (`Subdivide`, `Merge`).
  - Em `Model + Object`: atalhos de transformação (`Move`, `Rotate`, `Scale`) e primitivas rápidas (`Cubo`, `Esfera`, `Cilindro`, `Plano`) e duplicar objeto.
  - Em `Paint`: ferramentas de pincel, apagador, conta-gotas, ajuste de raio e chip da cor ativa.
  - Em `Animate`: timeline transport player (`◀◀`, `▶ Play / ⏸ Pausa`, `▶▶`) e seletor de frame.
  - Contenção e blindagem de eventos de ponteiro para evitar disparar raycasting de seleção 3D acidental durante cliques e ajustes na shelf.
- **Painel Lateral Retrátil de Navegação de Assets (`crates/ui/src/asset_browser.rs`)**:
  - Painel lateral dedicado à esquerda (220–340px) acionado pelo botão `[📦 Assets]` do cabeçalho superior.
  - Filtro por categorias (`Todos`, `Props`, `Personagens`, `Cenário`) e busca instantânea com `petunia_search_box`.
  - Cards detalhados com contagem de vértices e triângulos, swatch de cor e ações rápidas (`➕ Instanciar`, `🎯 Ativar`, `📋 Duplicar`, `🗑 Deletar`).
  - Botão de rodapé para salvar o modelo ativo atual diretamente na biblioteca do projeto (`state.save_active_as_asset()`).
- **Exportação Canônica nos Menus de Sistema (`crates/ui/src/main_header.rs`, `crates/ui/src/lib.rs`)**:
  - Reclassificação de `Export` de workspace para itens canônicos de menu: `Arquivo -> Exportar OBJ (.obj)...` e `Arquivo -> Exportar GLB (.glb)...`.
  - Introdução do workspace `ANIMATE` nas abas superiores: `[ MODEL ] [ PAINT ] [ UV ] [ ANIMATE ]`.

### Modificado
- **Reorganização Estrutural da Barra Superior da Viewport (`crates/ui/src/viewport_bar.rs`)**:
  - 6 clusters semânticos rigorosamente separados:
    1. Dropdown de Modo (`[ Object Mode ▾ ]` vs `[ Edit Mode ▾ ]`) com alvos contextuais (`⬝ Vértice`, `╱ Aresta`, `▨ Face`) exibidos **exclusivamente** em modo de edição.
    2. Menus rápidos com ícones (`👁 View ▾`, `▢ Select ▾`, `➕ Add+ ▾`) e menu contextual reativo (`🧊 Object ▾` ou `🕸 Mesh ▾`).
    3. Orientação de transformação (`Global`, `Local`, etc.) e Ponto de Pivô (`Median Point`, `3D Cursor`, etc.).
    4. Botões de Snapping Magnético (`🧲 Snap`) e Edição Proporcional (`◎ Prop`).
    5. Diagnóstico de cena (`⊞ Overlays`, `⧉ X-Ray`).
    6. 4 Modos de sombreamento esféricos canônicos do Blender (`○`, `●`, `◐`, `☼`).
  - Interceptação de atalhos de teclado globais (Tab para alternar Object/Edit, e 1/2/3 para alvos de vértice/aresta/face) tratada com fallback e garantia direta no egui sem conflitos de foco.
- **Descongestionamento e Limpeza da Sidebar Direita (`crates/ui/src/outliner.rs`, `crates/ui/src/properties_panel.rs`)**:
  - Redução estrita para apenas 2 componentes verticais: `Outliner` e `Properties`.
  - Remoção de galerias duplicadas, criação solta de primitivas no outliner e seção avulsa de exportação.
  - Aba `Material` no painel de propriedades agora abriga com exclusividade a cor base e a paleta interativa de swatches do projeto.
  - Aba `Object` refinada com identidade do objeto ativo e inspector `▾ Transform` com grid tri-axial e rótulos coloridos RGB (X, Y, Z).
- **Especialização da Barra de Ferramentas Vertical Esquerda (`crates/ui/src/toolbar.rs`)**:
  - Foco exclusivo nas 8 ferramentas primárias e persistentes de interação: `Select Box`, `3D Cursor`, `Move`, `Rotate`, `Scale`, `Transform`, `Measure` e `Annotate`.
  - Operações transitórias de modelagem de malha movidas para a Contextual Modeling Shelf.
- **Refatoração da Barra de Status Inferior (`crates/ui/src/status_bar.rs`)**:
  - Organizada em 3 blocos limpos:
    * Esquerda: indicador de projeto (`● Salvo` / `○ Não salvo`), nome do arquivo e atalhos de mouse/ferramenta ativa.
    * Centro: mensagens operacionais e feedbacks do sistema com truncate.
    * Direita: métricas agregadas da cena (`Tris: {} │ Verts: {} │ Objs: {} │ {:.1}ms │ v0.6.0`) e botões de Undo (`↩`) e Redo (`↪`).

## [0.6.0] - 2026-09-12 — Interactive Measurement & Annotation, Viewport Floating Bar, Asset Drawer, Theme & Icon Packs, Keymaps & TOML i18n

### Adicionado
- **Ferramenta Interativa de Régua e Medição 3D (`crates/ui/src/measurement.rs`, `viewport_interaction.rs`)**:
  - Medição espacial 3D com clique e arraste no viewport (`Tool::Measure`, atalho `M`).
  - Snapping magnético inteligente a vértices de malhas ativas com indicador circular visual.
  - Projeção de planos cartesianos e ray intersection tridimensional.
  - Régua com marcações métricas de graduação a cada 0.1 e 1.0 unidades.
  - Badge flutuante de medição exibindo distância euclidiana precisa e decomposição nos eixos cartesianos ($\Delta X, \Delta Y, \Delta Z$).
  - Cancelamento e limpeza instantânea via tecla `Delete` ou `Esc`.
- **Ferramenta Interativa de Anotação e Rascunho 3D (`crates/ui/src/annotation.rs`, `viewport_interaction.rs`)**:
  - Rascunho à mão livre em espaço tridimensional (`Tool::Annotate`, atalho `D`).
  - Projeção contínua sobre a superfície da malha ativa ou sobre o plano de referência do 3D Cursor.
  - Renderização fluida de strokes com espessura variável e suavização de pontos.
  - Limpeza e remoção de anotações via tecla `Delete` ou `Esc`.
- **Gaveta / Modal Dedicado da Biblioteca de Assets (`crates/ui/src/asset_library_drawer.rs`, `main_header.rs`)**:
  - Nova gaveta/janela flutuante dedicada (`📦 Assets` no header) para visualização e gerenciamento de assets do projeto.
  - Diferenciação conceitual e funcional explícita:
    * *Salvar Modelo Ativo como Asset*: Registra/salva a malha ativa diretamente na biblioteca interna do projeto (`state.project.assets`).
    * *Salvar Projeto*: Persiste o arquivo `.petunia` completo (cena, assets, materiais, paleta e configurações).
  - Cards visuais para cada modelo com contagem de vértices/faces, chips de cor e botões de ação: Instanciar no 3D Cursor (`➕ Instanciar`), Editar (`🎯 Editar`), Duplicar (`📋 Duplicar`) e Remover (`🗑 Remover`).
- **Novo Sistema Dinâmico de Temas Baseado em Tokens (`crates/config/src/theme.rs`, `crates/ui/src/tokens.rs`)**:
  - Suporte completo a temas declarativos em TOML com `manifest.toml` e `theme.toml`.
  - Mapeamento universal de tokens semânticos (`ThemeToken` e `ThemeColors`).
  - 4 temas nativos distribuídos: `petunia-dark`, `petunia-light`, `petunia-capuccino` e `petunia-tokyo-nights`.
  - Varredura dinâmica de diretórios (`assets/themes/`) com fallback tolerante a falhas para a paleta canônica Petunia Dark.
- **Sistema Aberto de Pacotes de Ícones (`crates/ui/src/icon_registry.rs`, `assets/icons/`)**:
  - Suporte modular a múltiplos pacotes de ícones via `manifest.toml` e `icons.toml`.
  - 5 pacotes estruturados: `Petunia`, `Phosphor`, `Tabler`, `Iconoir` e `Lucide`.
  - Cascading fallback seguro: raster/SVG do pacote -> desenho vetorial nativo Petunia -> glifo Unicode.
- **8 Perfis Canônicos de Teclado e Análise de Conflitos (`crates/config/src/keybinds.rs`, `assets/keymaps/`)**:
  - 8 perfis TOML completos: `Petunia Padrão`, `Petunia Simplificado`, `Petunia Notebook`, `Blender`, `Blender Notebook`, `Maya`, `3ds Max` e `Cinema 4D`.
  - Motor de análise e detecção automática de conflitos/colisões de atalhos em tempo de execução com alertas visuais.
- **Internacionalização (i18n) Declarativa via TOML (`crates/config/src/lib.rs`, `assets/locales/`)**:
  - Dicionários completos em TOML para `pt-BR` e `en` cobrindo todas as strings da interface.
  - Carregamento e troca dinâmica sem necessidade de reinicialização.
- **Modal Centralizado de Configurações (`crates/ui/src/settings_modal.rs`, `main_header.rs`)**:
  - Modal com 4 abas ergonômicas: Aparência (seleção de temas e chips de tokens de cor ao vivo), Ícones (seleção de pacotes e preview em grade), Idioma (pt-BR / en-US) e Teclado (troca de perfil, busca de atalhos e monitor de conflitos).

### Modificado
- **Refinamento Óptico dos Ícones da Toolbar (`crates/ui/src/icons.rs`)**:
  - Ajuste suave na espessura do traço vetorial de ~2.4–2.6px para ~1.8–1.9px, garantindo visual refinado e elegante sem perder a legibilidade ou as cores canônicas do Blender.
- **Reorganização Sem Sobreposição da Viewport Bar (`crates/ui/src/viewport_bar.rs`)**:
  - Botão de adição explicitado como `[➕ Add+ ▾]`.
  - Botões de Shading condensados em esferas compactas de 22x22px estilo Blender flutuantes (`○`, `●`, `◐`, `☼`).
  - Botões de seleção compactados (`🧊 Objeto`, `⬝ Vértice`, `╱ Aresta`, `▨ Face`) com atalhos transferidos para tooltips ricos, eliminando colisões horizontais mesmo em viewports estreitos.

## [0.5.0] - 2026-09-12 — UI Consolidation, Outliner Redesign, Dead Controls Cleanup & Blender Vector Icons

### Adicionado
- **Novo Sistema Vetorial de Ícones com Traço Reforçado e Paleta Autêntica do Blender (`crates/ui/src/{icons.rs, icon_registry.rs, app_icons.rs}`)**:
  - Traços reforçados de 1.5px para 2.0px–2.6px com renderização subpixel nítida contra fundos escuros da UI.
  - Paleta multicolorida fiel ao Blender:
    * `3D Cursor`: Anel circular vermelho vibrante com segmentos tracejados brancos e mira vazada.
    * `Transladar (Move)`: Eixos cardeais RGB (+X vermelho, +Y verde, +Z azul) com pontas de seta triangulares sólidas.
    * `Rotacionar (Rotate)`: Anéis elípticos cardeais tridimensionais em RGB com seta de rotação.
    * `Escalar (Scale)`: Hastes tridimensionais RGB terminadas em cubos sólidos preenchidos.
    * `Transformação Combinada (Transform)`: Gizmo unificado com setas de translação, cubos de escala e arco amarelo de rotação.
    * `Seleção em Caixa (Select Box)`: Retângulo de seleção azul/ciano translúcido com ponteiro branco de contorno escuro.
    * `Anotação (Annotate)`: Lápis Grease Pencil com corpo ciano, ponteira de madeira dourada e grafite escuro sobre traçado desenhado.
    * `Régua e Medição (Measure)`: Régua diagonal amarela com graduações pretas e miras de medição azul-claras.
    * `Adicionar Primitivas (Add Primitive)`: Cubo isométrico sombreado nos três tons de laranja clássicos do Blender com badge circular `+`.
    * `Ferramentas de Modelagem`: Traços reforçados de 2.0px–2.4px com destaques em laranja e amarelo vivo para `extrude`, `inset`, `bevel`, `loop_cut`, `knife`, `pushpull`, `slice`, `subdivide` e `draw_profile`.
- **Redesenho Completo e Funcional do Outliner (`crates/ui/src/outliner.rs`)**:
  - Integração direta e reativa com `state.project.assets`.
  - Botão de visibilidade funcional (`👁`) sincronizado com `asset.visible` e respeitado nos backends WebGPU e OpenGL.
  - Menu de contexto (RMB) para Duplicar (`Shift+D`) e Deletar (`Delete`).
  - Galeria rápida de primitivas integradas (`Cubo`, `Esfera`, `Cilindro`, `Plano`, `Cone`, `Cápsula`) instanciadas com precisão na coordenada do 3D Cursor (`state.cursor_3d`).
- **Atalhos e Modelo de Seleção Unificado (`crates/app/src/lib.rs`, `crates/config`)**:
  - Correção da propagação de eventos no `WgpuApp` e `GlApp`: teclas `Tab` e `0..=4` não são mais descartadas pelo egui quando nenhum campo de texto possui foco ativo (`!wants_keyboard_input()`).
  - Simplificação para 4 alvos explícitos de seleção: Objeto (`Tab` / `0`), Vértice (`1`), Aresta (`2`), Face (`3`).
- **Reorganização Semântica da Viewport Bar (`crates/ui/src/viewport_bar.rs`)**:
  - 5 clusters distintos com divisores visuais claros: Alvo de Seleção, Menus Compactos (`👁 View ▾`, `▢ Select ▾`, `+ Add ▾`), Transformação e Snapping, Toggles de Exibição (`Overlays`, `X-Ray`) e Sombreamento em 4 botões esféricos estilo Blender (`○ Wire`, `● Solid`, `◐ Material`, `☼ Render`).

### Removido
- **Limpeza de Controles e Botões Mortos**:
  - Remoção dos botões sem funcionalidade abaixo do Outliner (`Render Engine`, `Output`, `View Layer`, `Scene`, `World`, `Collection`).
  - Remoção dos pills estáticos de cena (`Scene`, `ViewLayer`) e do menu fictício `Render` do cabeçalho superior.
  - Remoção do painel inferior de Timeline, recuperando 100% da altura útil da viewport 3D.

## [0.4.0] - 2026-09-12 — Modernized egui Infrastructure, Specialized Crates & Sovereign Design System

### Adicionado
- **Soberania do Petunia Design System (`crates/ui/src/{tokens.rs, widgets.rs}`)**: Preservação estrita da identidade visual (`Blender.svg`), tokens de cores, métricas e comportamentos interativos sobrepondo qualquer estilo padrão de crates externas. Componentes atômicos: `PetuniaToolbarButton`, `PetuniaPropertyTabButton`, `PetuniaWorkspacePill` e `PetuniaSearchBox`.
- **Registro Centralizado de Ícones (`crates/ui/src/icon_registry.rs`)**:
  - `IconRegistry` e enum `PetuniaIcon` com suporte a 9 ícones de toolbar (PNGs 256x256 RGBA) e 15 ícones de abas de propriedades (PNGs 22x22 RGBA) extraídos diretamente do Figma (`assets/ui/icons/properties/`).
  - Decodificação de PNG embutido com preservação do canal alfa e tingimento dinâmico conforme estado de interação (repouso `#BCBCBC`, hover `#FFFFFF`, ativo `#3169E3`).
  - Fallback automático para ícones vetoriais de grade 24x24 e glifos tipográficos (Phosphor).
- **Integração de Gizmos de Transformação 3D (`transform-gizmo-egui`)**:
  - Módulo `crates/ui/src/transform_gizmo_integration.rs` com conversão bidirecional entre `petunia_core::camera::Camera` e `transform_gizmo::math::Transform`.
  - Mapeamento de modos de manipulação (`Translate`, `Rotate`, `Scale`) e orientação (`Global`, `Local`).
- **Navegação Hierárquica da Cena (`egui_ltreeview`)**:
  - Refatoração do Outliner (`crates/ui/src/outliner.rs`) adotando `egui_ltreeview::TreeView` estilizado com tokens Petunia.
  - Suporte a seleção de nós (`OutlinerNodeId`), expansão persistente, filtragem instantânea de nós via `petunia_search_box` e toggles de visibilidade e renderização.
- **Sistema de Layout e Docking Multi-Painel (`egui_tiles`)**:
  - Módulo `crates/ui/src/tiles_workspace.rs` gerenciando `egui_tiles::Tree<PetuniaPane>` estruturado na árvore canônica de visualização (`Blender.svg`): Toolbar à esquerda, Viewport 3D ao centro, Outliner superior direito, Propriedades inferior direito e Timeline inferior.
  - Implementação de `PetuniaTilesBehavior` customizando barras de abas, fundos, abas ativas, hover e strokes de redimensionamento em estrita conformidade com os tokens Petunia.
- **Serviço de Diálogos de Arquivos Multiplataforma (`egui-file-dialog`)**:
  - Módulo `crates/ui/src/file_dialog_service.rs` desacoplado, suportando Open Project (`.petunia`), Save Project, Save Project As, Import OBJ, Export OBJ e Export GLB.
- **Suíte de Testes de Fluxo UI Headless (`egui_kittest`)**:
  - Configuração de `dev-dependencies` e ativação da feature `accesskit` para interoperabilidade completa com `egui-winit`.
  - Bateria de testes de integração em `crates/ui/tests/kittest_ui_flows.rs` cobrindo fluxos de cabeçalho principal, barra de contexto, toolbar com contextualização de modos, árvore de outliner e layout de tiles.
  - Suíte completa de 64 testes automatizados em `petunia_ui` (59 unitários + 5 de fluxo kittest), 100% de aprovação e zero warnings em `cargo clippy --workspace --all-targets -- -D warnings`.

## [0.3.0] - 2026-09-12 — Canonical Desktop UI Architecture (`Blender.svg` Golden Reference)

### Adicionado
- **Design Tokens Canônicos (`crates/ui/src/tokens.rs`)**: Centralização da paleta Dark Theme profissional do Blender (`#121212`, `#1a1a1a`, `#202020`, `#2d2d2d`, `#3169e3`), raios de curvatura de controles/pílulas e métricas de layout.
- **Gerenciador de Ícones Nativos com Tingimento Dinâmico (`crates/ui/src/app_icons.rs`)**: Embutimento em tempo de compilação dos 9 PNGs transparentes extraídos do Figma (`assets/ui/icons/toolbar/*.png`), tingimento dinâmico com base no estado do botão (repouso `#BCBCBC`, hover `#FFFFFF`, ativo `#3169E3` com ícone branco) e fallback vetorial seamlessly integrado.
- **Cabeçalho Superior Principal (`crates/ui/src/main_header.rs`)**: Menus do sistema (File, Edit, Render, Window, Help), branding Petunia3D e abas de workspaces em pílulas arredondadas.
- **Barra de Contexto do Viewport 3D (`crates/ui/src/viewport_bar.rs`)**: Seletor de modo com badges coloridos (Object/Edit/Paint), botões de seleção de componentes (Vértice [1], Aresta [2], Face [3]), orientação de transformação, ponto de pivô, snapping magnético, edição proporcional e os 4 modos canônicos de sombreamento (Wireframe, Solid, Material, Render).
- **Barra Lateral de Ferramentas (`crates/ui/src/toolbar.rs`)**: Toolbar redimensionável com suporte a largura dinâmica (modo compacto de 48px ou expandido até 240px com ícones e rótulos de ferramentas); contextualização onde ferramentas de modelagem de malha (`extrude`, `inset`, `bevel`, `loop_cut`, etc.) aparecem exclusivamente no modo de edição (`EditMode::Edit`), com normalização defensiva para `select` no modo de objeto (`EditMode::Object`).
- **Cabeçalhos Redimensionáveis (`crates/ui/src/{main_header.rs, lib.rs, viewport_bar.rs}`)**: Cabeçalho principal do sistema e barra de contexto do viewport agora possuem altura redimensionável com limites definidos em `tokens.rs` e alinhamento vertical centralizado de todos os itens.
- **Painel Outliner Hierárquico (`crates/ui/src/outliner.rs`)**: Cabeçalho com modo de visualização, filtro de busca instantânea, botão de nova coleção e árvore hierárquica da cena com toggles de visibilidade e renderização.
- **Painel de Propriedades Modular (`crates/ui/src/properties_panel.rs`)**: Barra de navegação por abas (`Tool`, `Render`, `Output`, `Scene`, `World`, `Object`, `Modifiers`, `Data`, `Material`) com seções sanfonadas, campos de transformação coloridos por eixo (X vermelho, Y verde, Z azul) e integração com ferramentas ativas.
- **Painel de Timeline de Animação (`crates/ui/src/timeline.rs`)**: Controles de transporte completo (`|<<`, `<|`, `Play/Pause`, `|>`, `>>|`), contador numérico de frames, intervalo (Start/End) e régua de scrubbing temporal com cursor interativo.
- **Barra de Status Inferior (`crates/ui/src/status_bar.rs`)**: Dicas contextuais dos botões do mouse, mensagens de status do sistema e telemetria de malha e desempenho em tempo real.
- **Documentação de Diretórios**: READMEs estruturados em `assets/ui/icons/` e `assets/ui/icons/toolbar/` e atualização da arquitetura em `crates/ui/src/README.md`.

## [0.2.0] - 2026-09-12

### Adicionado
- Estrutura topológica Half-Edge (`HalfEdgeMesh`) com detecção de 2-variedades, anomalias de 1-anel via BFS e relatório de defeitos (`TopologyReport`).
- Operações geométricas avançadas: Método de Newell para normais poligonais, triangulação em leque para N-gons arbitrários ($N \ge 3$), fatiamento planar com fechamento de tampas e soldagem de costuras (`cut_edge_cache`), varredura ao longo de polilinhas com RMF (`sweep`), ponte entre loops de faces com minimização cíclica de distância (`connect_loops`), dissolução de arestas/vértices (`dissolve_selected`), inversão e recálculo unificado de normais.
- Modos de sombreamento no pipeline de renderização: Flat, Smooth (normais interpoladas por vértice) e Unlit em OpenGL 3.3 Core e WebGPU (WGSL).
- Suporte a 6 planos ortogonais de imagens de referência (Front, Back, Left, Right, Top, Bottom) com ângulo de rotação arbitrário e modo X-Ray em split-pass (renderizado após a geometria sólida com bypass de profundidade).
- Otimização de barramento PCIe no WebGPU com cache de hash FNV-1a para uploads de textura sob demanda.
- Ferramentas de interface e modelagem: Slice, Connect e Dissolve na barra de ferramentas esquerda com área de rolagem vertical responsiva e ativação não-destrutiva; seleção por caixa (`box_select`) com descarte de vértices atrás da câmera; inversão completa de seleção sincronizada (`invert_selection`).
- Preservação do índice do asset ativo na remoção de assets precedentes em `Project::remove`.
- Sincronização automática de paleta ativa com a paleta do projeto em operações de `undo()` e `redo()`.
- Cobertura expandida para 48 testes automatizados sem falhas e 0 warnings no Clippy com `-D warnings`.
- Validação contínua do ciclo de vida da aplicação com teste de fumaça headless (`petunia3d --smoke-test`).

## [0.1.0] - 2026-09-12

### Adicionado
- Inicialização da estrutura canônica do Prumo v0.5.
- Configuração do manifesto `prumo.json` e orquestração `.ai/`.
- Definição da hierarquia de documentação canônica em `docs/`.
- Contrato estrito de arquitetura em `docs/architecture/clean-code-contract.md`.
- Estratégia de testes exaustivos em `docs/development/testing-strategy.md` (unitários, integração, conformidade, segurança SAST/secrets, performance/stress, UI).
- Política de documentação mandatória com `README.md` explicativo em cada diretório do projeto.

## 2026-09-12 — Premium viewport, second implementation round (UI, Ortho Camera & Vector Icons)

- Implementada câmera ortográfica explícita (`Projection::Ortho`) com 6 vistas predefinidas (`Front`, `Back`, `Right`, `Left`, `Top`, `Bottom`), atalhos de Numpad e controle contínuo de altura de enquadramento.
- Criado motor de ícones vetoriais nativos em grade 24x24 (`crates/ui/src/icons.rs`) e componente acessível `tool_button` com variantes compacta (40x40) e expandida.
- Implementados campos de preenchimento numérico direto para ferramentas de transformação e modelagem (`crates/ui/src/tool_fields.rs`) sincronizados com a máquina de estados modal.
- Interface responsiva com adaptação a janelas estreitas, status bar aprimorada e tokens de tema com conformidade de contraste WCAG 2.2 AA.
- Suíte de testes expandida para 130 testes automatizados com 100% de aprovação, Clippy com 0 warnings e formatação canônica.

## 2026-09-12 — Premium viewport, first implementation round

- Added transactional modal previews, viewport HUD, exact input, axis/plane
  constraints, snapping, transform gizmos and visible-component hover/picking.
- Added quad loop preview/slide, welded knife segments, planar slice gestures,
  atomic vertex-paint strokes and contextual brush radius/eyedropper input.
- Fixed region extrusion topology, closed single-edge bevel, capped slice
  half-space semantics, Top/Bottom camera math and viewport input/redraw ordering.
- Standardized 1/2/3/4 selection, G/R/S, P, Ctrl+R, K/Shift+K and camera shortcuts.
- Added independent review and domain/egui regression tests. Historical premium
  convergence claims are superseded; remaining criteria are explicit in the plan.
