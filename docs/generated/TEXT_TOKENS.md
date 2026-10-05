---
title: Tokens de Tradução e Localização
description: Catálogo canônico de chaves de internacionalização TextId (P3D-119)
---

<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->

# Catálogo Canônico de Chaves de Localização (`TextId`)

> **Single Source of Truth (P3D-088, P3D-119)**
> A UI do Petunia3D é 100% internacionalizada. Nenhuma string do usuário é hardcoded; todas as mensagens passam pelo motor `I18n` com fallback seguro em inglês.

Total de chaves de localização cadastradas: **1686**.

| Chave (`TextId`) | Inglês (`en.toml`) | Português (`pt-BR.toml`) |
| :--- | :--- | :--- |
| `actions.amount` | Amount | Qtd |
| `actions.angle` | Angle | Ângulo |
| `actions.apply` | Apply | Aplicar |
| `actions.apply_scale` | Apply scale | Aplicar escala |
| `actions.bevel` | Round Edge | Round Edge |
| `actions.cancel` | Cancel | Cancelar |
| `actions.connect` | Bridge Faces | Conectar Faces |
| `actions.cursor_to_origin` | Cursor to World Origin | Cursor para origem global |
| `actions.cursor_to_origin_status` | 3D Cursor centered on the origin | Cursor 3D centralizado na origem |
| `actions.delete` | Delete | Apagar |
| `actions.deselect` | None | Nada |
| `actions.dissolve` | Dissolve Selected | Dissolver Seleção |
| `actions.distance` | Distance | Distância |
| `actions.duplicate` | Duplicate | Duplicar |
| `actions.extrude` | Extrude | Extrudar |
| `actions.factor` | Factor | Fator |
| `actions.flip_normals` | Flip Normals | Inverter Normais |
| `actions.inset` | Inset | Inset |
| `actions.invert` | Invert (Ctrl+I) | Inverter (Ctrl+I) |
| `actions.merge_by_distance` | Merge by distance | Fundir por distância |
| `actions.merge_center` | Merge center | Fundir no centro |
| `actions.merge_distance` | Distance | Distância |
| `actions.mirror` | Mirror | Espelhar |
| `actions.move` | Move | Mover |
| `actions.need_edit` | Requires a component domain (Point / Edge / Face) | Requer um domínio de componente (Point / Edge / Face) |
| `actions.need_selection` | Select geometry first | Selecione a geometria primeiro |
| `actions.no_mesh` | No active mesh | Sem malha ativa |
| `actions.pushpull` | Push/Pull | Push/Pull |
| `actions.recalculate_normals` | Recalc Normals | Recalcular Normais |
| `actions.revolve` | Revolve | Revolver |
| `actions.revolve_need_edges` | Select at least 2 connected edges first. | Selecione ao menos 2 edges conectados. |
| `actions.revolve_open_hint` | Angles below 360° leave the profile open. | Ângulos abaixo de 360° deixam o perfil aberto. |
| `actions.scale` | Scale | Escala |
| `actions.select_all` | All | Tudo |
| `actions.select_linked` | Linked (L) | Conectados (L) |
| `actions.slice` | Slice Plane | Fatiar Plano |
| `actions.slice_cap` | Slice (Cap) | Fatiar (tampa) |
| `actions.slice_x` | Slice X | Fatiar X |
| `actions.slice_y` | Slice Y | Fatiar Y |
| `actions.slice_z` | Slice Z | Fatiar Z |
| `actions.subdivide` | Subdivide | Subdividir |
| `actions.symmetrize` | Symmetrize | Simetrizar |
| `actions.symmetrize_dir` | Direction | Direção |
| `actions.symmetrize_dir_neg` | − to + | − para + |
| `actions.symmetrize_dir_pos` | + to − | + para − |
| `actions.triangulate` | Triangulate | Triangular |
| `actions.weld_eps` | Weld | Solda |
| `animate.add_creature` | Add creature | Adicionar criatura |
| `animate.advanced` | Advanced | Avançado |
| `animate.apply_now` | Apply Now | Aplicar agora |
| `animate.apply_now_tip` | Turn this Motion into an editable clip and remove the live Motion | Transforma o Movimento em um clipe editável e remove o Movimento vivo |
| `animate.auto_rig` | Auto-Rig | Auto-Rig |
| `animate.choice.pattern_alternate` | Alternate | Alternado |
| `animate.choice.pattern_auto` | Automatic | Automático |
| `animate.choice.pattern_lateral` | Side by side | Lado a lado |
| `animate.choice.pattern_wave` | Wave | Onda |
| `animate.creature.bird` | Bird | Pássaro |
| `animate.creature.fish` | Fish | Peixe |
| `animate.creature.humanoid` | Humanoid | Humanoide |
| `animate.creature.multi_leg` | Many legs | Muitas pernas |
| `animate.creature.quadruped` | Four legs | Quadrúpede |
| `animate.creature.serpent` | Serpent | Serpente |
| `animate.duplicate` | Duplicate Motion | Duplicar Movimento |
| `animate.empty_creature` | Start by adding a creature. It comes with bones ready to animate. | Comece adicionando uma criatura. Ela já vem com ossos prontos para animar. |
| `animate.empty_motion` | Pick a Motion on the left to bring your creature to life. | Escolha um Movimento à esquerda para dar vida à criatura. |
| `animate.first_frame` | First frame | Primeiro frame |
| `animate.fit_locked` | Unlock the model first | Destrave o modelo antes |
| `animate.fit_model` | Fit to model | Ajustar ao modelo |
| `animate.fit_model_tip` | Fits this creature's bones to your selected model so the model moves with them | Ajusta os ossos da criatura ao modelo selecionado para o modelo se mover com eles |
| `animate.fit_needs_model` | Select a model with points first | Selecione antes um modelo com pontos |
| `animate.frame` | Frame | Frame |
| `animate.humanoid` | Humanoid | Humanoide |
| `animate.keep_live` | Bake a copy | Gerar cópia |
| `animate.keep_live_tip` | Add an editable clip but keep the live Motion too | Adiciona um clipe editável e mantém também o Movimento vivo |
| `animate.last_frame` | Last frame | Último frame |
| `animate.linked_model` | Linked model | Modelo ligado |
| `animate.motion.biped_cycle` | Walk / Run | Andar / Correr |
| `animate.motion.gait` | Walk (legs) | Andar (pernas) |
| `animate.motion.idle_breath` | Breathe | Respirar |
| `animate.motion.serpentine` | Slither / Swim | Deslizar / Nadar |
| `animate.motion_tip.biped_cycle` | Walk or run on two legs | Andar ou correr sobre duas pernas |
| `animate.motion_tip.gait` | Walk, trot or crawl on four or more legs | Andar, trotar ou rastejar com quatro ou mais pernas |
| `animate.motion_tip.idle_breath` | Breathe and sway in place | Respirar e balançar no lugar |
| `animate.motion_tip.serpentine` | Slither, swim or sway a tail | Deslizar, nadar ou balançar uma cauda |
| `animate.needs_body` | Needs a body bone | Precisa de um osso de corpo |
| `animate.needs_chain` | Needs a longer spine or tail | Precisa de coluna ou cauda mais longa |
| `animate.needs_legs` | Needs at least two legs | Precisa de pelo menos duas pernas |
| `animate.param.arm_swing` | Arm swing | Balanço dos braços |
| `animate.param.duty` | Foot contact | Contato do pé |
| `animate.param.energy` | Energy | Energia |
| `animate.param.head_hold` | Head steady | Cabeça firme |
| `animate.param.lean` | Lean | Inclinação |
| `animate.param.pattern` | Step pattern | Padrão de passo |
| `animate.param.run_blend` | Walk ↔ Run | Andar ↔ Correr |
| `animate.param.smoothness` | Smoothness | Suavidade |
| `animate.param.speed` | Speed | Velocidade |
| `animate.param.spine_wave` | Spine wave | Onda da coluna |
| `animate.param.step_height` | Step height | Altura do passo |
| `animate.param.stride` | Stride | Passada |
| `animate.param.sway` | Sway | Balanço |
| `animate.param.tail_boost` | Tail boost | Reforço da cauda |
| `animate.param.tail_sway` | Tail sway | Balanço da cauda |
| `animate.param.variation` | Variation | Variação |
| `animate.param.wavelength` | Wave length | Comprimento da onda |
| `animate.param.weight` | Weight | Peso |
| `animate.param_tip.arm_swing` | How much the arms swing | Quanto os braços balançam |
| `animate.param_tip.duty` | How long each foot stays on the ground | Por quanto tempo cada pé fica no chão |
| `animate.param_tip.energy` | How big and lively the movement is | Quão grande e animado é o movimento |
| `animate.param_tip.head_hold` | Keeps the head steadier than the body | Mantém a cabeça mais firme que o corpo |
| `animate.param_tip.lean` | Lean the body backward or forward | Inclina o corpo para trás ou para a frente |
| `animate.param_tip.pattern` | Which legs move together | Quais pernas se movem juntas |
| `animate.param_tip.run_blend` | 0 walks, 1 runs | 0 anda, 1 corre |
| `animate.param_tip.smoothness` | Snappy at 0, flowing at 1 | Seco em 0, fluido em 1 |
| `animate.param_tip.speed` | How fast the Motion plays | Quão rápido o Movimento toca |
| `animate.param_tip.spine_wave` | How much the spine bends while walking | Quanto a coluna dobra ao andar |
| `animate.param_tip.step_height` | How high the feet lift | Quão alto os pés levantam |
| `animate.param_tip.stride` | Length of each step | Comprimento de cada passo |
| `animate.param_tip.sway` | Side-to-side body sway | Balanço lateral do corpo |
| `animate.param_tip.tail_boost` | Extra motion toward the tail tip | Movimento extra na ponta da cauda |
| `animate.param_tip.tail_sway` | How much the tail swings | Quanto a cauda balança |
| `animate.param_tip.variation` | Adds small natural differences | Adiciona pequenas diferenças naturais |
| `animate.param_tip.wavelength` | How many bends fit along the body | Quantas curvas cabem ao longo do corpo |
| `animate.param_tip.weight` | Heavier bodies bounce and settle more | Corpos mais pesados quicam e assentam mais |
| `animate.pause` | Pause | Pausar |
| `animate.play` | Play | Reproduzir |
| `animate.playhead` | Playhead | Cabeça de reprodução |
| `animate.remove` | Remove Motion | Remover Movimento |
| `animate.rig_error` | This creature cannot do this Motion. Check its bones. | Esta criatura não consegue fazer este Movimento. Confira os ossos dela. |
| `animate.root_motion` | Move forward | Avançar |
| `animate.root_motion_tip` | The whole body travels instead of walking in place | O corpo todo se desloca em vez de andar no lugar |
| `animate.show_bones` | Show bones | Mostrar ossos |
| `animate.stepped` | Stepped (retro) | Em degraus (retrô) |
| `animate.stepped_tip` | Holds each pose for a moment, like classic 12 fps animation | Segura cada pose por um instante, como na animação clássica de 12 fps |
| `animate.style.cartoon` | Cartoon | Cartoon |
| `animate.style.custom` | Custom | Personalizado |
| `animate.style.floaty` | Floaty | Flutuante |
| `animate.style.heavy` | Heavy | Pesado |
| `animate.style.stiff` | Stiff | Rígido |
| `animate.tip_first` | Jump to First Frame · Shift+Left | Ir ao Primeiro Frame · Shift+Left |
| `animate.tip_last` | Jump to Last Frame · Shift+Right | Ir ao Último Frame · Shift+Right |
| `animate.tip_next` | Step 1 Frame Forward · Right | Avançar 1 Frame · Right |
| `animate.tip_play` | Play / Pause Animation · Space | Reproduzir / Pausar Animação · Space |
| `animate.tip_prev` | Step 1 Frame Backward · Left | Voltar 1 Frame · Left |
| `animate.title_creature` | Creature | Criatura |
| `animate.title_motion` | Motion | Movimento |
| `animate.title_picker` | Motions | Movimentos |
| `animate.title_style` | Style | Estilo |
| `animate.unavailable_no_rig` | Add a creature first | Adicione uma criatura primeiro |
| `animate.workspace_tip` | Bring creatures to life with ready-made Motions | Dê vida a criaturas com Movimentos prontos |
| `app.title` | Petunia3D | Petunia3D |
| `camera.back` | Back | Traseira |
| `camera.bottom` | Bottom | Inferior |
| `camera.frame_hint` | Frame the selection, or the active object when nothing is selected (F / Numpad .). | Enquadrar a seleção ou o objeto ativo quando nada estiver selecionado (F / Numpad .). |
| `camera.free` | Free | Livre |
| `camera.front` | Front | Frente |
| `camera.height_hint` | World height at the view center. Drag the value or double-click to type an exact size. | Altura do enquadramento no centro da vista, em metros. Arraste o valor ou clique duas vezes para digitar uma medida exata. |
| `camera.iso_ne` | Isometric NE | Isométrico NE |
| `camera.iso_nw` | Isometric NW | Isométrico NW |
| `camera.iso_se` | Isometric SE | Isométrico SE |
| `camera.iso_sw` | Isometric SW | Isométrico SW |
| `camera.isometric` | Isometric | Isométrico |
| `camera.left` | Left | Esquerda |
| `camera.orthographic` | Orthographic | Ortográfica |
| `camera.perspective` | Perspective | Perspectiva |
| `camera.projection` | Projection | Projeção |
| `camera.projection_hint` | Change projection while keeping the same framing at the view center. Orbit works in both modes. | Alterar a projeção mantendo o enquadramento no centro da vista. É possível orbitar nos dois modos. |
| `camera.reset` | Reset view | Redefinir vista |
| `camera.reset_hint` | Restore the origin and default zoom while keeping this view and projection (Home). | Voltar à origem e ao zoom inicial mantendo esta vista e projeção (Home). |
| `camera.right` | Right | Direita |
| `camera.top` | Top | Superior |
| `camera.view` | View | Vista |
| `camera.views_hint` | Axis-aligned views use orthographic projection. Ctrl selects the opposite side. | Vistas alinhadas aos eixos usam projeção ortográfica. Ctrl seleciona o lado oposto. |
| `camera.visible_height` | Visible height | Altura visível |
| `command_palette.no_results` | No commands found | Nenhum comando encontrado |
| `command_palette.placeholder` | Type a command or search… | Digite um comando ou busque… |
| `context.delete_collection` | Delete Collection | Apagar Coleção |
| `context.delete_tip` | Delete object (Delete) | Apagar objeto (Delete) |
| `context.export` | Export... | Exportar... |
| `context.hide` | Hide in 3D Viewport | Ocultar na Viewport 3D |
| `context.isolate` | Isolate Object | Isolar Objeto |
| `context.isolate_exit` | Restore Visibility (Exit Isolate) | Restaurar Visibilidade (Sair do Isolamento) |
| `context.isolate_tip` | Isolate Active Object (Numpad /): Hide all others and focus the selected one | Isolar Objeto Ativo (Numpad /): Esconder todos os outros e focar no selecionado |
| `context.isolate_tip_on` | Isolate Active (Numpad /): Restore visibility of all objects | Isolar Ativo (Numpad /): Restaurar visibilidade de todos os objetos |
| `context.lock` | Lock Object | Bloquear Objeto |
| `context.lock_tip` | Lock Object (prevents transform) | Bloquear Objeto (impede transformar) |
| `context.modeling` | Modeling | Modelagem |
| `context.move_to_collection` | Move to Collection | Mover para Coleção |
| `context.new_collection_tip` | Create a new Collection to organize models | Criar nova Coleção para organizar modelos |
| `context.none_root` | None (Root) | Nenhuma (Raiz) |
| `context.object_locked` | Object Locked | Objeto Bloqueado |
| `context.rename` | Rename | Renomear |
| `context.rename_collection` | Rename Collection | Renomear Coleção |
| `context.show` | Show in 3D Viewport | Mostrar na Viewport 3D |
| `context.toggle_col_lock` | Toggle Collection Lock | Alternar Bloqueio da Coleção |
| `context.toggle_col_vis` | Toggle Collection Visibility | Alternar Visibilidade da Coleção |
| `context.unlock` | Unlock Object | Desbloquear Objeto |
| `ctx.extrude_region` | Extrude Region | Extrudar Região |
| `ctx.separate` | Separate Selection | Separar Seleção |
| `density.comfortable` | Comfortable | Confortável |
| `density.compact` | Compact | Compacta |
| `density.label` | Density | Densidade |
| `density.spacious` | Spacious | Espaçosa |
| `display.title` | Display | Exibição |
| `dock.left` | Left | Esquerda |
| `dock.orientation` | Panels layout | Disposição dos painéis |
| `dock.right` | Right | Direita |
| `dock.side` | Dock side | Lado do dock |
| `dock.side_by_side` | Side by side | Lado a lado |
| `dock.stacked` | Stacked | Empilhados |
| `draw.shape_name` | Shape | Forma |
| `edit.redo` | Redo | Refazer |
| `edit.undo` | Undo | Desfazer |
| `empty.no_selection` | Nothing selected | Nada selecionado |
| `empty.no_selection_hint` | Select an object to inspect its properties. | Selecione um objeto para inspecionar suas propriedades. |
| `empty.scene_empty` | Scene is empty — add something: | Cena vazia — adicione algo: |
| `export.empty` | nothing selected | nada selecionado |
| `export.format` | Format | Formato |
| `export.go` | Export… | Exportar… |
| `export.report` | Report | Relatório |
| `export.title` | Export | Exportar |
| `extensions.angle` | Segment angle | Ângulo do segmento |
| `extensions.arc` | Arc through first three points | Arco pelos três primeiros pontos |
| `extensions.dimensions` | Toggle segment dimensions | Alternar dimensões por segmento |
| `extensions.ellipse` | Ellipse profile | Perfil elíptico |
| `extensions.length` | Size / segment length | Tamanho / comprimento do segmento |
| `extensions.mirror` | Toggle mirrored creation | Alternar criação espelhada |
| `extensions.polygon` | Regular polygon | Polígono regular |
| `extensions.radius` | Corner radius | Raio dos cantos |
| `extensions.rectangle` | Rounded rectangle | Retângulo arredondado |
| `extensions.resample` | Convert curve to N segments | Converter curva em N segmentos |
| `extensions.round` | Round shape corners | Arredondar cantos da forma |
| `extensions.sides` | Sides / arc segments | Lados / segmentos do arco |
| `extensions.simplify` | Simplify shape points | Simplificar pontos da forma |
| `extensions.slot` | Slot profile | Perfil de rasgo |
| `extensions.threshold` | Trace threshold (0–254) | Limiar de vetorização (0–254) |
| `extensions.tolerance` | Simplification tolerance | Tolerância de simplificação |
| `extensions.trace` | Trace visible reference silhouette | Vetorizar silhueta da referência visível |
| `file.import_gltf` | Import glTF / GLB… | Importar glTF / GLB… |
| `file.import_obj` | Import OBJ… | Importar OBJ… |
| `file.new` | New project | Novo projeto |
| `file.open_project` | Open project… | Abrir projeto… |
| `file.quit` | Quit | Sair |
| `file.save` | Save | Salvar |
| `file.save_as` | Save as… | Salvar como… |
| `gallery.disabled` | Disabled | Desativado |
| `gallery.edge` | Edge | Aresta |
| `gallery.empty_action` | New shape | Nova forma |
| `gallery.empty_message` | Click a part in the viewport or create a new shape. | Clique numa parte na viewport ou crie uma forma nova. |
| `gallery.empty_state` | Empty state | Estado vazio |
| `gallery.empty_title` | Nothing selected | Nada selecionado |
| `gallery.face` | Face | Face |
| `gallery.ghost` | Ghost action | Ação simples |
| `gallery.hint_orbit` | Orbit | Orbitar |
| `gallery.hint_pan` | Pan view | Mover vista |
| `gallery.hint_select` | Select | Selecionar |
| `gallery.icon_button` | Icon buttons | Botões de ícone |
| `gallery.key_lmb` | LMB | LMB |
| `gallery.key_mmb` | MMB | MMB |
| `gallery.key_palette` | Ctrl K | Ctrl K |
| `gallery.key_shift_mmb` | Shift MMB | Shift MMB |
| `gallery.key_undo` | Ctrl+Z | Ctrl+Z |
| `gallery.labelled` | Extrude | Extrudar |
| `gallery.object` | Object | Objeto |
| `gallery.pbr_standard` | PBR Standard | PBR padrão |
| `gallery.point` | Point | Ponto |
| `gallery.profile` | Profile | Perfil |
| `gallery.properties` | Property rows | Linhas de propriedade |
| `gallery.roughness` | Roughness | Rugosidade |
| `gallery.search_and_hints` | Search and hints | Busca e dicas |
| `gallery.search_placeholder` | Search command… | Buscar comando… |
| `gallery.segmented` | Segmented control | Controle segmentado |
| `gallery.silhouette` | Silhouette | Silhueta |
| `gallery.solid` | Solid | Sólido |
| `gallery.textured` | Textured | Texturizado |
| `gallery.title` | Petunia Components — gallery | Petunia Components — galeria |
| `gallery.toggle_off` | Toggle (off) | Alternar (desligado) |
| `gallery.toggle_on` | Toggle (on) | Alternar (ligado) |
| `gallery.tool_active` | Tool (active) | Ferramenta (ativa) |
| `gallery.tool_idle` | Tool (idle) | Ferramenta (inativa) |
| `gallery.value_short` | V | V |
| `gallery.wireframe` | Wireframe | Aramado |
| `geometry.title` | Geometry | Geometria |
| `geometry.tris` | Triangles | Triângulos |
| `help.body` | MMB orbit • Shift+MMB pan • wheel zoom • Tab mode • Del delete • Home reset • H help • Ctrl+Z/Y undo | MMB orbita • Shift+MMB pan • scroll zoom • Tab modo • Del apaga • Home reseta • H ajuda • Ctrl+Z/Y desfaz |
| `hints.bevel` | Ctrl+B: interactive Round Edge of one supported edge. | Ctrl+B: Round Edge interativo de um edge suportado. |
| `hints.connect` | B: bridge two loops or faces. | B: conecta (bridge) dois loops ou faces. |
| `hints.dissolve` | X: dissolve selected edges/vertices cleanly. | X: dissolve edges/points sem deixar buracos. |
| `hints.draw_profile` | Shift+P: click in ortho view to add points. Click near 1st to close. | Shift+P: clique na vista ortográfica p/ pontos. Perto do 1º fecha. |
| `hints.extrude` | E: extrude selected faces. | E: extruda as faces selecionadas. |
| `hints.inset` | I: inset selected faces. | I: inset nas faces selecionadas. |
| `hints.merge` | M: merge selected into center. | M: funde a seleção no centro. |
| `hints.mirror` | Ctrl+M: mirror + weld. | Ctrl+M: espelha + solda. |
| `hints.paint` | B: click the mesh to paint vertex colors. Alt+click: pick. | B: clique na malha p/ pintar. Alt+clique: conta-gotas. |
| `hints.primitives` | A: add low-poly primitive. | A: adiciona primitiva low-poly. |
| `hints.pushpull` | P: push/pull along normals. | P: empurra/puxa ao longo das normais. |
| `hints.revolve` | Select connected edges, then revolve them around an axis. | Selecione edges conectados e revolva ao redor de um eixo. |
| `hints.select` | Click to select. 1/2/3: point/edge/face. | Clique p/ selecionar. 1/2/3: point/edge/face. |
| `hints.slice` | Shift+K: drag a cutting plane; Enter applies, Esc cancels. | Shift+K: arraste o plano de corte; Enter aplica, Esc cancela. |
| `hints.subdivide` | W: subdivide (loop cut). Triangulate below. | W: subdivide (loop cut). Triangular abaixo. |
| `hints.symmetrize` | Alt+M: copy one side across the axis and weld the seam. | Alt+M: copia um lado para o outro no eixo e solda a costura. |
| `hints.transform` | G/R/S: move, rotate, scale with mouse. Enter applies; Esc cancels. | G/R/S: mover, rotacionar, escalar com mouse. Enter aplica; Esc cancela. |
| `home.new` | New model | Novo modelo |
| `home.no_recent` | No recent projects yet. Projects you open or save appear here. | Nenhum projeto recente ainda. Os projetos que você abrir ou salvar aparecem aqui. |
| `home.open` | Open project… | Abrir projeto… |
| `home.recent` | Recent projects | Projetos recentes |
| `home.recover` | Recover session | Recuperar sessão |
| `home.settings` | Settings | Ajustes |
| `home.show_on_start` | Show this screen at startup | Mostrar esta tela ao abrir |
| `home.subtitle` | Pick up where you left off or start something new. | Continue de onde parou ou comece algo novo. |
| `home.title` | Petunia3D | Petunia3D |
| `inspector.add_component` | Add Component | Adicionar Componente |
| `inspector.go_material` | Material (open tab) | Material (abrir aba) |
| `inspector.go_object` | Transform (open tab) | Transform (abrir aba) |
| `inspector.no_results` | No matching properties. | Nenhuma propriedade correspondente. |
| `inspector.pin` | Pin | Fixar |
| `inspector.pin_tip` | Pin Inspector Keep this object visible in the Inspector while selecting other objects. | Fixar Inspector Mantém este objeto visível no Inspector ao selecionar outros objetos. |
| `inspector.search` | Search properties... | Buscar propriedades... |
| `inspector.tab_material` | Material | Material |
| `inspector.tab_modify` | Modifiers | Modificadores |
| `inspector.tab_object` | Object | Objeto |
| `inspector.tab_selection` | Selection | Seleção |
| `inspector.tool_active` | Active Tool | Ferramenta Ativa |
| `inspector.tool_bevel` | Round Edge tool | Ferramenta Round Edge |
| `inspector.tool_mirror` | Mirror tool | Ferramenta Mirror |
| `inspector.tool_subdivide` | Subdivide tool | Ferramenta Subdivide |
| `inspector.unpin` | Unpin | Soltar |
| `inspector.unpin_tip` | Unpin Inspector Follow the current selection again. | Soltar Inspector Volta a seguir a seleção atual. |
| `keymap.capture` | capture… | capturar… |
| `keymap.conflict_shared` | shares the shortcut with | compartilha o atalho com |
| `keymap.conflict_title` | Shortcut conflicts detected | Conflito de atalhos detectado |
| `keymap.unsupported_key` | Key not supported by the keymap | Tecla não suportada pelo keymap |
| `menu.command_palette` | Command Palette | Paleta de Comandos |
| `menu.edit` | Edit | Editar |
| `menu.file` | File | Arquivo |
| `menu.help` | Help | Ajuda |
| `menu.preferences` | Preferences | Preferências |
| `menu.recent_projects` | Recent Projects | Projetos Recentes |
| `menu.view` | View | Exibir |
| `menu.window` | Window | Janela |
| `modes.edge` | Edge | Edge |
| `modes.edit` | Edit | Edição |
| `modes.face` | Face | Face |
| `modes.object` | Object | Objeto |
| `modes.paint` | Texture Paint | Pintura |
| `modes.vertex` | Point | Point |
| `modifiers.add` | Add | Adicionar |
| `modifiers.add_mirror` | Add Mirror | Adicionar Espelho |
| `modifiers.add_symmetry` | Add Symmetry | Adicionar Simetria |
| `modifiers.axis` | Axis | Eixo |
| `modifiers.empty` | No modifiers in stack. | Sem modificadores na pilha. |
| `modifiers.enable` | Enable modifier | Ativar modificador |
| `modifiers.remove` | Remove modifier | Remover modificador |
| `modifiers.title` | Modifiers | Modificadores |
| `paint.blend_add` | Add | Adicionar |
| `paint.blend_multiply` | Multiply | Multiplicar |
| `paint.blend_normal` | Normal | Normal |
| `paint.blend_screen` | Screen | Tela |
| `paint.brush` | Brush | Pincel |
| `paint.brush_airbrush` | Airbrush | Aerógrafo |
| `paint.brush_eraser` | Eraser | Borracha |
| `paint.brush_fill` | Fill | Preencher |
| `paint.brush_line` | Line | Linha |
| `paint.brush_picker` | Color picker | Conta-gotas |
| `paint.brush_pixel` | Pixel | Pixel |
| `paint.brush_rect` | Rectangle | Retângulo |
| `paint.brush_soft` | Soft | Suave |
| `paint.canvas` | Albedo canvas | Canvas albedo |
| `paint.canvas_hint` | Drag to paint. Ctrl+drag erases. Wheel over UV scales it. | Arraste p/ pintar. Ctrl+arraste apaga. Scroll no UV escala. |
| `paint.canvas_resize` | Resize (keeps content) | Redimensionar (mantém conteúdo) |
| `paint.canvas_size` | Texture size | Tamanho da textura |
| `paint.channel` | Channel | Canal |
| `paint.channel_albedo` | Albedo (Base Color) | Albedo (Cor Base) |
| `paint.channel_emission` | Emission | Emissão |
| `paint.channel_height` | Height | Altura |
| `paint.channel_locked_tip` | V1 paints Albedo only. Other channels arrive in V1.x (P3D-062). | V1 pinta só Albedo. Outros canais chegam na V1.x (P3D-062). |
| `paint.channel_metallic` | Metallic | Metálico |
| `paint.channel_normal` | Normal | Normal |
| `paint.channel_roughness` | Roughness | Rugosidade |
| `paint.clear` | Clear | Limpar |
| `paint.color` | Color | Cor |
| `paint.color_current` | Current color | Cor atual |
| `paint.color_current_tip` | Color used by the brush and the bucket. Adding it to the palette is explicit (+). | Cor usada pelo pincel e pelo balde. Adicionar à paleta é explícito (+). |
| `paint.effect_add` | Add Effect | Adicionar efeito |
| `paint.effect_add_tip` | Adds a new layer with a non-destructive effect | Cria uma camada nova com efeito não-destrutivo |
| `paint.effect_brightness` | Brightness | Brilho |
| `paint.effect_brightness_contrast` | Brightness / Contrast | Brilho / Contraste |
| `paint.effect_cell_size` | Cell size | Tamanho da célula |
| `paint.effect_contrast` | Contrast | Contraste |
| `paint.effect_gamma` | Gamma | Gama |
| `paint.effect_grain` | Grain | Grão |
| `paint.effect_hue` | Hue | Matiz |
| `paint.effect_hue_saturation` | Hue / Saturation | Matiz / Saturação |
| `paint.effect_in_max` | Input max | Máx. de entrada |
| `paint.effect_in_min` | Input min | Mín. de entrada |
| `paint.effect_intensity` | Intensity | Intensidade |
| `paint.effect_invert` | Invert | Inverter |
| `paint.effect_levels` | Levels | Níveis |
| `paint.effect_levels_count` | Levels | Níveis |
| `paint.effect_no_parameters` | No parameters | Sem parâmetros |
| `paint.effect_out_max` | Output max | Máx. de saída |
| `paint.effect_out_min` | Output min | Mín. de saída |
| `paint.effect_pixelate` | Pixelate | Pixelizar |
| `paint.effect_posterize` | Posterize | Posterizar |
| `paint.effect_saturation` | Saturation | Saturação |
| `paint.eraser` | Eraser (hold Ctrl) | Borracha (segure Ctrl) |
| `paint.eraser_hint` | Hold Ctrl while painting to erase | Segure Ctrl pintando p/ apagar |
| `paint.fill` | Fill | Preencher |
| `paint.fill_done` | filled {n} faces | {n} faces preenchidas |
| `paint.fill_sel` | Fill sel | Preencher sel |
| `paint.fill_sel_tip` | Fill the selected faces with the current color | Preenche as faces selecionadas com a cor atual |
| `paint.flow` | Flow | Fluxo |
| `paint.flow_tip` | Paint delivered per dab over time (Airbrush) | Tinta depositada por dab ao longo do tempo (Airbrush) |
| `paint.hardness` | Hardness | Dureza |
| `paint.hardness_tip` | Solid core as a fraction of the radius (0 = fully soft) | Núcleo sólido como fração do raio (0 = totalmente suave) |
| `paint.isolate_faces` | Isolate faces (mask) | Isolar faces (máscara) |
| `paint.isolate_faces_tip` | Confine 3D strokes to the selected faces only | Confinar traço 3D exclusivamente às faces selecionadas |
| `paint.layer_blend` | Blend | Mistura |
| `paint.layer_copy_name` | {name} copy | cópia de {name} |
| `paint.layer_default_name` | Layer {n} | Camada {n} |
| `paint.layer_delete` | Delete layer | Apagar camada |
| `paint.layer_down` | Move down | Descer |
| `paint.layer_duplicate` | Duplicate layer | Duplicar camada |
| `paint.layer_empty` | Single base layer. Add layers for non-destructive detail. | Camada base única. Adicione camadas p/ detalhe não destrutivo. |
| `paint.layer_hide` | Hide layer | Ocultar camada |
| `paint.layer_kind_hint` | Raster layer: the brush paints into this layer. | Camada raster: o pincel pinta dentro dela. |
| `paint.layer_new` | New layer | Nova camada |
| `paint.layer_opacity` | Opacity | Opacidade |
| `paint.layer_show` | Show layer | Mostrar camada |
| `paint.layer_up` | Move up | Subir |
| `paint.layers` | Layers | Camadas |
| `paint.new_canvas` | New 256² | Novo 256² |
| `paint.new_canvas_tip` | Replaces the texture with a new 256² canvas and clears the layers | Substitui a textura por uma tela 256² nova e limpa as camadas |
| `paint.palette_add` | Add | Adicionar |
| `paint.palette_add_tip` | Add current color to palette | Adicionar a cor atual à paleta |
| `paint.palette_clear` | Clear | Limpar |
| `paint.palette_export` | Export | Exportar |
| `paint.palette_export_tip` | Export palette to .gpl | Exportar paleta p/ .gpl |
| `paint.palette_import` | Import | Importar |
| `paint.palette_import_tip` | Import .hex or .gpl palette | Importar paleta .hex ou .gpl |
| `paint.palette_loaded_gameboy` | Game Boy palette loaded | Paleta Game Boy carregada |
| `paint.palette_loaded_pico8` | PICO-8 palette loaded | Paleta PICO-8 carregada |
| `paint.palette_remove` | Remove | Remover |
| `paint.palette_remove_tip` | Remove the current color from the palette | Remove a cor atual da paleta |
| `paint.palette_use` | Use this color | Usar esta cor |
| `paint.pick` | Pick? | Pegar? |
| `paint.pick_hint` | Alt+click the mesh to pick a color | Alt+clique na malha p/ pegar cor |
| `paint.picked` | color picked | cor capturada |
| `paint.pixel_grid` | Pixel grid | Grade de pixels |
| `paint.pixel_grid_tip` | Pixel grid on the 2D canvas (only when zoomed in) | Grade de pixels no canvas 2D (só com zoom suficiente) |
| `paint.prepare_surface` | Prepare surface | Preparar superfície |
| `paint.prepare_surface_hint` | Projection used by the 3D brush when the mesh has no usable UVs. | Projeção usada pelo pincel 3D quando a malha não tem UVs utilizáveis. |
| `paint.radius` | Radius | Raio |
| `paint.size` | Size px | Tam px |
| `paint.size_tip` | Brush diameter in pixels, on the 2D canvas and on the model | Diâmetro do pincel em pixels, na tela 2D e no modelo |
| `paint.spacing` | Spacing | Espaçamento |
| `paint.spacing_tip` | Distance between dabs as a fraction of the diameter | Distância entre dabs como fração do diâmetro |
| `paint.strength` | Strength | Força |
| `paint.strength_tip` | Opacity of each dab | Opacidade de cada dab |
| `paint.surface_auto_unwrap` | Auto Unwrap | Auto Unwrap |
| `paint.surface_auto_unwrap_tip` | Cut the mesh into charts automatically (angle-based) | Corta a malha em charts automaticamente (por ângulo) |
| `paint.surface_box` | Box | Box |
| `paint.surface_box_tip` | Project the six box faces (fastest for hard-surface props) | Projeta as seis faces da caixa (mais rápido para props de hard-surface) |
| `paint.surface_planar` | Planar | Planar |
| `paint.surface_planar_tip` | Re-project the selection from the current view | Reprojeta a seleção a partir da vista atual |
| `paint.surface_unwrapped` | Surface prepared: {n} charts | Superfície preparada: {n} ilhas |
| `paint.tool_section_brushes` | Brushes | Pincéis |
| `paint.tool_section_sample` | Sample | Amostra |
| `paint.tool_section_shapes` | Shapes | Formas |
| `paint.vertex` | Paint on model | Pintura no modelo |
| `pivot.bounds` | Bounding Box Center | Centro da caixa |
| `pivot.cursor` | 3D Cursor | Cursor 3D |
| `pivot.edit_pivot` | Edit Pivot Mode | Modo Ajustar Pivô |
| `pivot.edit_pivot_hint` | Transform only the object's origin/pivot | Transforma apenas a origem/pivô do objeto |
| `pivot.geometry_to_origin` | Geometry to Origin | Geometria para a Origem |
| `pivot.individual` | Individual Origins | Origens individuais |
| `pivot.median` | Median Point | Ponto mediano |
| `pivot.origin_to_bottom` | Origin to Bottom | Origem para a Base |
| `pivot.origin_to_cursor` | Origin to 3D Cursor | Origem para o Cursor 3D |
| `pivot.origin_to_geometry` | Origin to Geometry | Origem para a Geometria |
| `pivot.origin_to_selection` | Origin to Selection | Origem para a Seleção |
| `prefab.deleted` | Prefab '{name}' removed from the library | Prefab '{name}' removido da biblioteca |
| `prefab.instantiated` | Prefab '{name}' placed in the scene | Prefab '{name}' colocado na cena |
| `prefab.renamed` | Prefab renamed to '{name}' | Prefab renomeado para '{name}' |
| `prefab.saved` | Prefab '{name}' saved to the library | Prefab '{name}' salvo na biblioteca |
| `prefab.updated` | Prefab '{name}' updated from the selection | Prefab '{name}' atualizado com a seleção |
| `preferences.click_move_click` | Drag without holding the button (click, move, click) | Arrastar sem segurar o botão (clicar, mover, clicar) |
| `preferences.click_move_click_hint` | Click a handle: it follows the mouse until the next click. | Clique numa alça: ela segue o mouse até o próximo clique. |
| `preferences.colorblind_axes` | Colorblind axes differentiation (X, Y, Z labels) | Diferenciação não-cromática de eixos (Rótulos X, Y, Z) |
| `preferences.double_tap_interval` | Double-tap shortcut interval (ms) | Intervalo de duplo toque de atalho (ms) |
| `preferences.drag_threshold` | Distance before a drag starts (px) | Distância para começar a arrastar (px) |
| `preferences.multiselection_measure_tag` | Display average measure tag on multiple edge selection | Exibir tag de média na multiseleção de arestas |
| `preferences.reduced_motion` | Reduced motion (disable viewport animations) | Redução de movimento (desativa animações do viewport) |
| `preferences.show_tool_labels` | Show names in the tool rail | Mostrar nomes no trilho de ferramentas |
| `preferences.show_tool_labels_hint` | The rail gets wider and each tool shows its name next to the icon. | O trilho fica mais largo e cada ferramenta mostra o nome ao lado do ícone. |
| `preferences.snap_radius` | Snap radius (px) | Raio do snap (px) |
| `preferences.studio_light_follows_camera` | Studio light follows the camera | Luz de estúdio acompanha a câmera |
| `preferences.studio_light_follows_camera_hint` | On: the shape stays readable from any side while you orbit (Plasticity/Cinema 4D style). Off: the light stays fixed in the world. | Ligada: a forma continua legível de qualquer lado ao orbitar (estilo Plasticity/Cinema 4D). Desligada: a luz fica fixa no mundo. |
| `preferences.workplane_from_selection` | Set Workplane from Selection (3 Points / Face) | Plano de Trabalho pela Seleção (3 Pontos / Face) |
| `preferences.workplane_prefer_ground` | Automatic work plane favors the ground | Plano automático favorece o chão |
| `preferences.workplane_prefer_ground_hint` | With no face under the cursor, drawing goes on the ground unless the camera is almost level. Off: the world plane that most faces the view (Modo/Cinema 4D style). | Sem face sob o cursor, o desenho vai para o chão, a menos que a câmera esteja quase na horizontal. Desligado: o plano do mundo mais de frente para a vista (estilo Modo/Cinema 4D). |
| `prims.body_length` | Body Length | Comprimento do Corpo |
| `prims.bottom_radius` | Bottom Radius | Raio da Base |
| `prims.cancel` | Cancel | Cancelar |
| `prims.cap` | Cap | Tampa |
| `prims.cap_both` | Both | Ambas |
| `prims.cap_bottom_only` | Bottom | Base |
| `prims.cap_none` | None | Nenhuma |
| `prims.cap_top_only` | Top | Topo |
| `prims.capsule` | Capsule | Cápsula |
| `prims.circle` | Circle | Círculo |
| `prims.cone` | Cone (8) | Cone (8) |
| `prims.confirm` | Confirm | Confirmar |
| `prims.confirm_hint` | Enter confirms · Esc cancels | Enter confirma · Esc cancela |
| `prims.cube` | Cube | Cubo |
| `prims.cylinder` | Cylinder (8) | Cilindro (8) |
| `prims.depth` | Depth | Profundidade |
| `prims.fill` | Fill | Preenchimento |
| `prims.fill_disc` | Disc | Disco |
| `prims.fill_none` | None | Nenhum |
| `prims.group_basic` | BASIC | BÁSICAS |
| `prims.group_organic` | ORGANIC | ORGÂNICAS |
| `prims.group_round` | ROUND | REDONDAS |
| `prims.height` | Height | Altura |
| `prims.icosphere` | Icosphere | Icoesfera |
| `prims.major_radius` | Major Radius | Raio Maior |
| `prims.minor_radius` | Minor Radius | Raio Menor |
| `prims.plane` | Plane | Plano |
| `prims.radius` | Radius | Raio |
| `prims.reopen` | Last operation… | Última operação… |
| `prims.reset` | Reset | Redefinir |
| `prims.rings` | Rings | Anéis |
| `prims.segments` | Segments | Segmentos |
| `prims.sides` | Sides | Lados |
| `prims.size` | Size | Tamanho |
| `prims.sphere` | Sphere (low) | Esfera (low) |
| `prims.subdivision` | Subdivision Level | Nível de Subdivisão |
| `prims.tip_body_length` | Length of the straight body between the rounded caps. | Comprimento do corpo reto entre as calotas. |
| `prims.tip_caps` | Close the top and bottom with flat caps. | Fecha topo e base com tampas planas. |
| `prims.tip_fill` | Fill the circle with a triangle fan, or keep only the outline. | Preenche o círculo com leque de triângulos ou mantém só o contorno. |
| `prims.tip_major_radius` | Distance from the Torus center to the center of its tube. | Distância do centro do Toro ao centro do tubo. |
| `prims.tip_minor_radius` | Radius of the Torus tube. | Raio do tubo do Toro. |
| `prims.tip_rings` | Rings from bottom to top. Fewer rings read more low-poly. | Anéis da base ao topo. Menos anéis, mais low-poly. |
| `prims.tip_segments` | Segments around the shape. Fewer segments read more low-poly. | Segmentos ao redor da forma. Menos segmentos, mais low-poly. |
| `prims.tip_sides` | Number of sides around the shape. Lower values create a more visibly low-poly result. | Número de lados ao redor da forma. Valores menores deixam o low-poly mais visível. |
| `prims.tip_subdiv` | Each level significantly increases the number of faces in the Icosphere. | Cada nível aumenta muito o número de faces da Icoesfera. |
| `prims.tip_top_radius` | Radius of the upper ring. Set it to zero to create a cone. | Raio do anel superior. Zere para criar um cone. |
| `prims.tip_vertices` | Segments around the circle. | Segmentos ao redor do círculo. |
| `prims.top_radius` | Top Radius | Raio do Topo |
| `prims.torus` | Torus | Toro |
| `prims.tris` | Tris | Tris |
| `prims.vertices` | Segments | Segmentos |
| `prims.wedge` | Wedge | Cunha |
| `prims.width` | Width | Largura |
| `profile.clear` | Clear | Limpar |
| `profile.close` | Close | Fechar |
| `profile.closed` | profile closed | perfil fechado |
| `profile.depth` | Depth | Profundidade |
| `profile.gen_extrude` | Gen Extrude | Gerar Extrude |
| `profile.gen_revolve` | Gen Revolve | Gerar Revolve |
| `profile.generated` | profile mesh generated | malha do perfil gerada |
| `profile.need_closed` | close the profile first (click near 1st point) | feche o perfil antes (clique perto do 1º ponto) |
| `profile.need_points` | draw at least 2 points first | desenhe ao menos 2 pontos |
| `profile.points` | points | pontos |
| `profile.segments` | Revolve segs | Segs revolve |
| `profile.snap` | Snap 0.25 | Snap 0.25 |
| `profile.tris` | tris | tris |
| `profile.undo_pt` | Undo pt | Desfaz pt |
| `props.edges` | edges | edges |
| `props.faces` | tris | tris |
| `props.name` | Name | Nome |
| `props.verts` | verts | verts |
| `refs.add_custom` | Add Unassigned… | Adicionar Avulsa… |
| `refs.add_custom_tooltip` | Load a reference image without pinning to a canonical slot | Carregar uma imagem de referência sem fixá-la a um slot canônico |
| `refs.align_view` | Align 3D camera to this reference angle | Alinhar câmera 3D com este ângulo de referência |
| `refs.back` | Back | Trás |
| `refs.bottom` | Bottom | Fundo |
| `refs.clear_all` | Clear All | Limpar Todas |
| `refs.clear_all_tooltip` | Remove all reference images from scene | Remover todas as imagens de referência da cena |
| `refs.click_to_load` | Click to load | Clique para carregar |
| `refs.fine_tune` | Fine tuning | Ajuste fino |
| `refs.front` | Front | Frente |
| `refs.left` | Left | Esquerda |
| `refs.load` | Load image… | Carregar imagem… |
| `refs.loaded` | reference(s) loaded | referência(s) carregada(s) |
| `refs.lock` | Lock | Bloquear |
| `refs.manage` | Reference Manager… | Gerenciador de Referências… |
| `refs.manager_desc` | Configure independent reference images for the 6 canonical orthographic slots. | Configure imagens de referência independentes para os 6 slots ortográficos canônicos. |
| `refs.manager_title` | Reference Set Manager | Gerenciador de Conjunto de Referências |
| `refs.no_image` | No image bound to this view | Nenhuma imagem vinculada a esta vista |
| `refs.offset` | Offset | Offset |
| `refs.opacity` | Opacity | Opacidade |
| `refs.remove` | Remove reference image | Remover imagem de referência |
| `refs.replace` | Click to replace | Clique para substituir |
| `refs.reset_default` | Reset to default | Redefinir padrão |
| `refs.right` | Right | Direita |
| `refs.rotation` | Rotation | Rotação |
| `refs.side` | Side | Lado |
| `refs.size` | Size | Tamanho |
| `refs.top` | Top | Topo |
| `refs.visible` | Visible | Visível |
| `refs.xray` | X-Ray / Overlay | Raio-X / Sobrepor |
| `scene.f_annotations` | Annotations | Anotações |
| `scene.f_collections` | Collections | Coleções |
| `scene.f_measurements` | Measurements | Medidas |
| `scene.f_refs` | Reference images | Imagens de referência |
| `scene.f_state` | Objects | Objetos |
| `scene.filter` | Filter | Filtro |
| `scene.reset_split` | Auto size | Tamanho automático |
| `scene.search` | Search objects... | Buscar objetos... |
| `scene.title` | Scene | Cena |
| `scene_filter.all` | All | Todos |
| `scene_filter.unlocked` | Unlocked only | Só desbloqueados |
| `scene_filter.visible` | Visible only | Só visíveis |
| `selection.clear` | Clear | Limpar |
| `selection.domain_edge` | Edge | Aresta |
| `selection.domain_face` | Face | Face |
| `selection.domain_object` | Object | Objeto |
| `selection.domain_point` | Point | Ponto |
| `selection.mode` | Selection mode: {domain} | Modo de seleção: {domain} |
| `selection.selected` | selected | selecionados |
| `settings.appearance` | Appearance | Aparência |
| `settings.density` | Interface density | Densidade da interface |
| `settings.examples` | Examples: | Exemplos: |
| `settings.export_glb` | GLB export format (off exports OBJ) | Formato de exportação GLB (desligado exporta OBJ) |
| `settings.export_glb_hint` | Default format for the export dialog | Formato padrão do diálogo de exportação |
| `settings.icons` | Icons | Ícones |
| `settings.icons_desc` | The pack changes the interface icons (menus, panels, transport); the 3D tools keep Petunia vector art. Custom packs arrive through plugins. | O pacote muda os ícones de interface (menus, painéis, transporte); as ferramentas 3D mantêm a arte vetorial Petunia. Pacotes personalizados entram pela via de plugins. |
| `settings.icons_title` | Icon packs | Pacotes de Ícones |
| `settings.import_export` | Import / Export | Importar / Exportar |
| `settings.interface` | Interface | Interface |
| `settings.keymap` | Keymap | Atalhos |
| `settings.language` | Language | Idioma |
| `settings.pack_active` | • Active | • Ativo |
| `settings.pack_desc_iconoir` | Minimalist, geometric look | Visual minimalista e geométrico |
| `settings.pack_desc_lucide` | Refined 2px vector stroke | Traço vetorial refinado de 2px |
| `settings.pack_desc_petunia` | Native icons with Blender style and vector rendering | Ícones nativos com estilo Blender e renderização vetorial |
| `settings.pack_desc_phosphor` | Clean, modern and balanced lines | Linhas limpas, modernas e equilibradas |
| `settings.pack_desc_tabler` | Consistent, technical 24x24 grid | Grade 24x24 consistente e técnica |
| `settings.pack_name_iconoir` | Iconoir (default) | Iconoir (Padrão) |
| `settings.pack_name_lucide` | Lucide Icons | Lucide Icons |
| `settings.pack_name_petunia` | Petunia (own art) | Petunia (Arte própria) |
| `settings.pack_name_phosphor` | Phosphor Icons | Phosphor Icons |
| `settings.pack_name_tabler` | Tabler Icons | Tabler Icons |
| `settings.pack_not_compiled` | · build without extended-icon-packs | · build sem extended-icon-packs |
| `settings.pack_selected` | ✔ Selected | ✔ Selecionado |
| `settings.pack_use` | Use this pack | Usar este pacote |
| `settings.reset_all_layouts` | Reset All UI Layouts | Redefinir Todos os Layouts |
| `settings.reset_workspace` | Reset Current Workspace Layout | Redefinir Layout do Workspace Atual |
| `settings.show_shelf` | Contextual shelf over the viewport | Barra contextual sobre a viewport |
| `settings.title` | Settings | Configurações |
| `shading.smooth` | Smooth | Suave |
| `shading.solid` | Flat | Plano |
| `shading.textured` | Textured | Textura |
| `shading.tip_material` | Material Preview (Z 2) | Prévia de Material (Z 2) |
| `shading.tip_rendered` | Rendered View (Z 8) | Vista Renderizada (Z 8) |
| `shading.tip_solid` | Solid / Clay (Z 6) | Sólido / Clay (Z 6) |
| `shading.tip_wireframe` | Wireframe (Z 4) | Arame (Z 4) |
| `shading.unlit` | Unlit | Sem Luz |
| `shading.wire` | Wireframe | Arame |
| `sl.200_ms` | 200 ms | 200 ms |
| `sl.2d_circle` | 2D Circle | Círculo 2D |
| `sl.2d_planar_boundary_circle_polygon_or_n_gon_d` | 2D planar boundary circle polygon or n-gon disc | Círculo plano 2D: polígono de contorno ou disco n-gon |
| `sl.2d_rectangle` | 2D Rectangle | Retângulo 2D |
| `sl.350_ms` | 350 ms | 350 ms |
| `sl.3d_cursor` | 3D Cursor | Cursor 3D |
| `sl.3d_mesh_modeling_extrusion_loop_cuts_and_boo` | 3D mesh modeling, extrusion, loop cuts, and booleans | Modelagem de malhas 3D, extrusão, loop cuts e booleanas |
| `sl.3d_viewport` | 3D Viewport | Viewport 3D |
| `sl.500_ms` | 500 ms | 500 ms |
| `sl.90_ccw` | ↺ 90° CCW | ↺ 90° anti-horário |
| `sl.90_cw` | ↻ 90° CW | ↻ 90° horário |
| `sl.accessibility` | Accessibility | Acessibilidade |
| `sl.accessibility_inclusion` | Accessibility & Inclusion | Acessibilidade & Inclusão |
| `sl.active` | Active | Ativa |
| `sl.active_effect` | Active effect | Efeito ativo |
| `sl.active_features_high_contrast_theme_ui_scale` | Active features: high contrast (theme), UI scale, reduced motion and axes that don't rely on color. | Recursos ativos: alto contraste (tema), escala de UI, movimento reduzido e eixos sem depender de cor. |
| `sl.add_an_effect_layer` | Add an effect layer | Adicionar camada de efeito |
| `sl.add_cube` | Add Cube | Adicionar cubo |
| `sl.add_cylinder` | Add Cylinder | Adicionar cilindro |
| `sl.add_decal_layer` | Add decal layer | Adicionar camada de decalque |
| `sl.add_group` | Add group | Adicionar grupo |
| `sl.add_layer` | Add layer | Adicionar camada |
| `sl.add_parametric_3d_cube_mesh_with_customizabl` | Add parametric 3D cube mesh with customizable subdivisions | Adiciona um cubo 3D paramétrico com subdivisões ajustáveis |
| `sl.add_parametric_cylinder_with_radial_subdivis` | Add parametric cylinder with radial subdivisions | Adiciona um cilindro paramétrico com subdivisões radiais |
| `sl.add_primitive` | Add Primitive | Adicionar primitiva |
| `sl.add_sphere` | Add Sphere | Adicionar esfera |
| `sl.add_uv_sphere_mesh_with_longitude_and_latitu` | Add UV sphere mesh with longitude and latitude rings | Adiciona uma esfera UV com anéis de longitude e latitude |
| `sl.airbrush` | Airbrush | Aerógrafo |
| `sl.alt_w_subdivide` | Alt+W: Subdivide | Alt+W: Subdividir (Subdivide) |
| `sl.angle` | Angle | Ângulo |
| `sl.angle_2` | Angle:  | Ângulo:  |
| `sl.appearance` | Appearance | Aparência |
| `sl.assistive_technology` | Assistive Technology | Tecnologia Assistiva |
| `sl.autodesk_3ds_max` | Autodesk 3ds Max | Autodesk 3ds Max |
| `sl.autodesk_maya` | Autodesk Maya | Autodesk Maya |
| `sl.automatic_fallback_to_english_when_a_key_is_` | ✓ Automatic fallback to English when a key is not found | ✓ Fallback automático para inglês quando chave não encontrada |
| `sl.automatic_workplane_ground_xz_active_face_or` | • Automatic workplane: Ground (XZ), active Face or Orthographic view | • Workplane automático: Ground (XZ), Face ativa ou Vista ortográfica |
| `sl.automation_mcp` | Automation (MCP) | Automação (MCP) |
| `sl.bake_reference_to_texture` | Bake Reference to Texture | Gerar textura da referência |
| `sl.base_cap_off` | Base Cap: off | Tampa da base: desligada |
| `sl.base_cap_on` | Base Cap: on | Tampa da base: ligada |
| `sl.bevel` | Round Edge | Round Edge |
| `sl.bevel_round_edge_bevel` | Bevel (Round Edge / Bevel) | Bisel (Round Edge / Bevel) |
| `sl.blender_official` | Blender (Official) | Blender (Oficial) |
| `sl.bottom_cap_off` | Bottom Cap: off | Tampa inferior: desligada |
| `sl.bottom_cap_on` | Bottom Cap: on | Tampa inferior: ligada |
| `sl.bottom_radius` | Bottom Radius | Raio inferior |
| `sl.box_click_selection_of_scene_points_edges_an` | Box / click selection of scene points, edges and faces | Seleção por caixa ou clique de Points, arestas e faces da cena |
| `sl.bridge_edge_loops_with_configurable_quad_seg` | Bridge edge loops with configurable quad segments and twist | Conecta loops de arestas com segmentos de quads e torção configuráveis |
| `sl.brush` | Brush | Pincel |
| `sl.brush_angle` | Angle | Ângulo |
| `sl.brush_blend` | Blend | Mistura |
| `sl.brush_blend_add` | Add | Somar |
| `sl.brush_blend_darken` | Darken | Escurecer |
| `sl.brush_blend_lighten` | Lighten | Clarear |
| `sl.brush_blend_multiply` | Multiply | Multiplicar |
| `sl.brush_blend_normal` | Normal | Normal |
| `sl.brush_blend_screen` | Screen | Tela |
| `sl.brush_clone_hint` | Ctrl+click on the surface to set the clone source. | Ctrl+clique na superfície para definir a origem do clone. |
| `sl.brush_clone_ready` | Clone source set. Ctrl+click to change it. | Origem do clone definida. Ctrl+clique para trocar. |
| `sl.brush_flow` | Flow | Fluxo |
| `sl.brush_larger` | Larger brush | Pincel maior |
| `sl.brush_lock` | Brush lock | Bloquear pincel |
| `sl.brush_opacity_jitter` | Opacity jitter | Variação de opacidade |
| `sl.brush_preset_delete` | Delete preset | Apagar preset |
| `sl.brush_preset_save` | Save brush | Salvar pincel |
| `sl.brush_presets` | Presets | Presets |
| `sl.brush_roundness` | Roundness | Achatamento |
| `sl.brush_scatter` | Scatter | Espalhamento |
| `sl.brush_settings` | Brush Settings | Configurações do pincel |
| `sl.brush_size_jitter` | Size jitter | Variação de tamanho |
| `sl.brush_smaller` | Smaller brush | Pincel menor |
| `sl.brush_smoothing` | Stabilizer | Estabilizador |
| `sl.brush_spacing` | Spacing | Espaçamento |
| `sl.brush_spray_density` | Spray density | Densidade do spray |
| `sl.brush_tip` | Tip | Ponta |
| `sl.brush_tip_diamond` | Diamond | Losango |
| `sl.brush_tip_round` | Round | Redonda |
| `sl.brush_tip_square` | Square | Quadrada |
| `sl.brush_type` | Brush type | Tipo de pincel |
| `sl.brush_type_blur` | Blur | Desfoque |
| `sl.brush_type_burn` | Burn | Queimar |
| `sl.brush_type_clone` | Clone | Clone |
| `sl.brush_type_dodge` | Dodge | Clarear |
| `sl.brush_type_pixel` | Pixel | Pixel |
| `sl.brush_type_smudge` | Smudge | Borrar |
| `sl.brush_type_spray` | Spray | Spray |
| `sl.camera_projection` | Camera Projection | Projeção da Câmera |
| `sl.camera_viewpoints_viewport_overlays_and_rend` | Camera viewpoints, viewport overlays and rendering display modes | Pontos de vista da câmera, overlays da viewport e modos de exibição |
| `sl.cancel` | Cancel | Cancelar |
| `sl.cancel_esc` | Cancel (Esc) | Cancelar (Esc) |
| `sl.canvas` | Canvas | Tela |
| `sl.cap_rings` | Cap Rings | Anéis da tampa |
| `sl.capsule` | Capsule | Cápsula |
| `sl.card_gizmo` | CARD / GIZMO | CARD / GIZMO |
| `sl.change_pivot_point_origin_for_transformation` | Change pivot point origin for transformations (Median, Cursor, Bounds) | Muda a origem do pivô das transformações (Mediana, Cursor, Limites) |
| `sl.choose` | Choose | Escolher |
| `sl.circle` | Circle | Círculo |
| `sl.circle_2d` | Circle 2D | Círculo 2D |
| `sl.clamp_overlap_active` | ✓ Clamp Overlap (Active) | ✓ Limitar sobreposição (ativo) |
| `sl.clamp_overlap_off` | Clamp Overlap (Off) | Limitar sobreposição (desligado) |
| `sl.clamp_overlap_prevents_geometry_self_interse` | Clamp Overlap (prevents geometry self-intersection) | Clamp Overlap (evita auto-interseção de geometria) |
| `sl.clean_quads` | Clean quads | Quads limpos |
| `sl.clean_quads_off` | Clean quads: off | Quads limpos: desligado |
| `sl.clean_quads_on` | Clean quads: on | Quads limpos: ligado |
| `sl.clear` | Clear | Limpar |
| `sl.clear_all` | Clear All | Limpar tudo |
| `sl.clear_all_seams` | Clear all seams | Limpa todas as costuras |
| `sl.clear_all_transforms` | Clear All Transforms | Limpar todas as transformações |
| `sl.clear_all_uv_pins` | Clear all UV pins | Limpa todas as fixações UV |
| `sl.clear_location` | Clear Location | Limpar posição |
| `sl.clear_operand` | Clear operand | Limpar operando |
| `sl.clear_pins` | Clear Pins | Limpar fixações |
| `sl.clear_rotation` | Clear Rotation | Limpar rotação |
| `sl.clear_scale` | Clear Scale | Limpar escala |
| `sl.clear_seams` | Clear Seams | Limpar costuras |
| `sl.clear_selection` | Clear Selection | Limpar seleção |
| `sl.click_and_drag_or_press_e_alt_e_to_extrude_s` | Click and drag or press E / Alt+E to extrude selected faces | Clique e arraste ou pressione E / Alt+E para extrudar as faces selecionadas |
| `sl.click_drag` | Click + Drag | Clique + Arrastar |
| `sl.click_edge_to_cut_scroll_to_adjust_cuts` | Click edge to cut · Scroll to adjust cuts | Clique na aresta para cortar · Role para ajustar os cortes |
| `sl.click_to_collapse_section` | Click to collapse section | Clique para recolher a seção |
| `sl.click_to_enter_value_drag_to_adjust_shift_fo` | Click to enter value · Drag to adjust · Shift for precision | Clique para digitar · Arraste para ajustar · Shift para precisão |
| `sl.click_to_expand_section` | Click to expand section | Clique para expandir a seção |
| `sl.click_to_pin_section_open` | Click to pin section open | Clique para fixar a seção aberta |
| `sl.click_to_select_or_modify_active_color` | Click to select or modify active color | Clique para escolher ou alterar a cor ativa |
| `sl.click_to_unpin_section` | Click to unpin section | Clique para desafixar a seção |
| `sl.click_vertices_or_edges_to_draw_cut_path` | Click vertices or edges to draw cut path | Clique em vértices ou arestas para desenhar o caminho do corte |
| `sl.close_asset_library` | Close asset library | Fechar biblioteca de Assets |
| `sl.close_reference_manager` | Close reference manager | Fechar gerenciador de referências |
| `sl.close_scene_drawer` | Close scene drawer | Fechar gaveta da cena |
| `sl.close_settings` | Close settings | Fechar configurações |
| `sl.collapse` | Collapse | Recolher |
| `sl.collapse_and_merge_all_selected_vertices_to_` | Collapse and merge all selected vertices to their median center | Colapsa e mescla todos os vértices selecionados no centro mediano |
| `sl.collapse_inspector` | Collapse Inspector | Recolher Inspector |
| `sl.collapse_inspector_hint` | Close every section and return to the pill rail. Hover a pill to peek again. | Fecha todas as seções e volta ao trilho de pílulas. Passe o mouse numa pílula para espiar de novo. |
| `sl.color_palette` | Color Palette | Paleta de cores |
| `sl.color_picker` | Color Picker | Conta-gotas |
| `sl.color_swatch` | Color Swatch | Amostra de cor |
| `sl.combine` | Combine | Combinar |
| `sl.combine_the_active_part_with_the_operand` | Combine the active part with the operand | Combina a parte ativa com o operando |
| `sl.command_search` | Command search | Busca de comandos |
| `sl.cone` | Cone | Cone |
| `sl.configure_studio_matcaps_cavity_highlights_a` | Configure studio matcaps, cavity highlights and wireframe opacity | Configura matcaps de estúdio, realces de cavidade e opacidade do wireframe |
| `sl.confirm` | Confirm | Confirmar |
| `sl.confirm_cut` | Confirm Cut | Confirmar Cut |
| `sl.confirm_enter` | Confirm (Enter) | Confirmar (Enter) |
| `sl.conical_geometry_tapering_to_a_single_apex_v` | Conical geometry tapering to a single apex vertex | Geometria cônica que afina até um único vértice no topo |
| `sl.connect` | Connect | Connect |
| `sl.connect_loops` | Connect Loops | Connect Loops |
| `sl.create_2d_circle_profile_on_active_workplane` | Create 2D circle profile on active workplane, ready to extrude or revolve | Cria um perfil circular 2D no plano de trabalho ativo, pronto para extrudar ou revolver |
| `sl.create_2d_rectangle_profile_on_active_workpl` | Create 2D rectangle profile on active workplane, ready to extrude | Cria um perfil retangular 2D no plano de trabalho ativo, pronto para extrudar |
| `sl.create_an_identical_clone_of_selected_geomet` | Create an identical clone of selected geometry or parts | Cria um clone idêntico da geometria ou das partes selecionadas |
| `sl.create_face_or_edge_connecting_selected_vert` | Create face or edge connecting selected vertices or edges (F) | Cria uma face ou aresta ligando vértices ou arestas selecionados (F) |
| `sl.ctrl_snap` | Ctrl: Snap | Ctrl: encaixe |
| `sl.ctrl_z_ctrl_y_undo_redo` | Ctrl+Z / Ctrl+Y: Undo / Redo | Ctrl+Z / Ctrl+Y: Desfazer / Refazer |
| `sl.cube` | Cube | Cubo |
| `sl.cursor` | Cursor | Cursor |
| `sl.curve` | Curve | Curva |
| `sl.cut` | Cut | Cut |
| `sl.cut_mesh_along_an_infinite_cutting_plane_thr` | Cut mesh along an infinite cutting plane through selection | Corta a malha por um plano de corte infinito através da seleção |
| `sl.cuts` | Cuts | Cortes |
| `sl.cylinder` | Cylinder | Cilindro |
| `sl.cylindrical_body_capped_by_two_hemispherical` | Cylindrical body capped by two hemispherical domes | Corpo cilíndrico com duas cúpulas hemisféricas nas pontas |
| `sl.default_modeling_parameters` | Default Modeling Parameters | Parâmetros Padrão de Modelagem |
| `sl.deg` | Deg | Graus |
| `sl.delete` | Delete | Excluir |
| `sl.depth` | Depth | Profundidade |
| `sl.disabled` | Disabled | Desativado |
| `sl.disabled_0` | Disabled (0) | Desativado (0) |
| `sl.display_mesh_edges_with_transparent_wirefram` | Display mesh edges with transparent wireframe surfaces | Exibe as arestas da malha com superfícies wireframe transparentes |
| `sl.dissolve` | Dissolve | Dissolver |
| `sl.dissolve_selected_vertices_edges_or_faces_wi` | Dissolve selected vertices, edges or faces without removing surrounding geometry | Dissolve vértices, arestas ou faces selecionados sem remover a geometria ao redor |
| `sl.distance` | Distance:  | Distância:  |
| `sl.document_mutations_go_through_transactional_` | Document mutations go through transactional commands with Undo/Redo. | Mutações do documento passam por comandos transacionais com Undo/Redo. |
| `sl.donut_shaped_revolution_ring_with_major_and_` | Donut-shaped revolution ring with major and minor radii | Anel de revolução em forma de rosca, com raios maior e menor |
| `sl.double_tap_interval_modal_shortcut` | Double-Tap Interval (Modal Shortcut) | Intervalo do Duplo Toque (Atalho Modal) |
| `sl.drag_a_cut_line_across_the_mesh_to_bisect` | Drag a cut line across the mesh to bisect | Arraste uma linha de corte pela malha para dividi-la |
| `sl.drag_direction` | Drag Direction | Direção de Arraste |
| `sl.draw` | DRAW | DRAW |
| `sl.draw_2d_profile_curves_on_ground_face_or_vie` | Draw 2D profile curves on ground, face, or viewplane to extrude or revolve | Desenhe curvas de perfil 2D no chão, em uma face ou no plano da vista para extrudar ou revolver |
| `sl.draw_linear_color_gradients_between_endpoint` | Draw linear color gradients between endpoints across active canvas | Desenha gradientes lineares de cor entre dois pontos na tela ativa |
| `sl.draw_straight_brush_strokes_between_clicked_` | Draw straight brush strokes between clicked endpoints | Desenha pinceladas retas entre os pontos clicados |
| `sl.dual_balanced_off` | Dual Balanced: OFF | Dual balanceado: OFF |
| `sl.dual_balanced_on` | Dual Balanced: ON | Dual balanceado: ON |
| `sl.duplicate` | Duplicate | Duplicar |
| `sl.edge` | Edge | Aresta |
| `sl.edge_loop` | Edge Loop | Loop de arestas |
| `sl.edge_ring` | Edge Ring | Anel de arestas |
| `sl.effects` | Effects | Efeitos |
| `sl.ellipse` | Ellipse | Elipse |
| `sl.ellipse_description` | Fill an ellipse inscribed between two points on the surface | Preenche uma elipse inscrita entre dois pontos da superfície |
| `sl.enabled` | Enabled | Ativado |
| `sl.english_us` | English (US) | English (US) |
| `sl.enter_confirm_esc_cancel` | Enter confirm · Esc cancel | Enter confirma · Esc cancela |
| `sl.enter_confirm_esc_cancel_drag_slide` | Enter confirm · Esc cancel · Drag slide | Enter confirma · Esc cancela · Arraste para deslizar |
| `sl.enter_confirm_esc_cancel_t_toggle_trim` | Enter: Confirm · Esc: Cancel · T: Toggle Trim | Enter: Confirmar · Esc: Cancelar · T: Alternar Aparar |
| `sl.equalize_texel_density` | Equalize Texel Density | Igualar densidade de texels |
| `sl.equalize_texel_density_across_uv_islands` | Equalize texel density across UV islands | Iguala a densidade de texels entre as ilhas UV |
| `sl.erase_pixel_color_values_and_alpha_transpare` | Erase pixel color values and alpha transparency from active texture | Apaga cor e transparência alfa da textura ativa |
| `sl.eraser` | Eraser | Borracha |
| `sl.essential_keyboard_shortcuts` | Essential Keyboard Shortcuts | Atalhos de Teclado Essenciais |
| `sl.expand` | Expand | Expandir |
| `sl.export` | Export | Exportar |
| `sl.extensions_plugins` | Extensions & Plugins | Extensões & Plugins |
| `sl.extrude` | Extrude | Extrude |
| `sl.extrude_preview_live` | Extrude Preview (Live) | Prévia de Extrude (ao vivo) |
| `sl.extrude_region_e` | Extrude Region (E) | Extrude Região (E) |
| `sl.f1_f2_f3_workspaces_model_paint_uv` | F1 / F2 / F3: Workspaces (Model/Paint/UV) | F1 / F2 / F3: Workspaces (Model/Paint/UV) |
| `sl.face` | Face | Face |
| `sl.face_loop` | Face Loop | Loop de faces |
| `sl.face_orientation` | Face Orientation | Orientação das faces |
| `sl.face_orientation_front_back_normals` | Face Orientation (Front/Back normals) | Orientação de Faces (Normais frontais/traseiras) |
| `sl.face_orientation_front_blue_back_red` | Face Orientation: Front (Blue) / Back (Red) | Orientação das faces: frente (azul) / verso (vermelho) |
| `sl.face_orientation_off` | Face Orientation: Off | Orientação das faces: desligada |
| `sl.fill` | Fill | Preencher |
| `sl.fill_bucket` | Fill Bucket | Balde de tinta |
| `sl.fill_disc` | Fill: Disc | Preenchimento: disco |
| `sl.fill_open_loop` | Fill: Open Loop | Preenchimento: laço aberto |
| `sl.fill_or_stroke_rectangular_2d_shapes_on_text` | Fill or stroke rectangular 2D shapes on texture canvas | Preenche ou contorna formas retangulares 2D na tela de textura |
| `sl.fill_projection` | Fill & Projection | Preenchimento e projeção |
| `sl.fill_scope` | Fill scope | Escopo do preenchimento |
| `sl.filled` | Filled | Filled |
| `sl.flat_2d_quad_rectangular_plane_surface_posit` | Flat 2D quad rectangular plane surface positioned on ground plane | Superfície plana retangular 2D posicionada no plano do chão |
| `sl.flip_normals` | Flip Normals | Inverter normais |
| `sl.flood_fill_contiguous_color_areas_on_active_` | Flood fill contiguous color areas on active UV map | Preenche áreas contíguas de cor no mapa UV ativo |
| `sl.focus_camera` | Focus Camera | Focar câmera |
| `sl.frame` | Frame | Enquadrar |
| `sl.frame_and_center_camera_view_on_currently_se` | Frame and center camera view on currently selected geometry | Enquadra e centraliza a câmera na geometria selecionada |
| `sl.frame_selection` | Frame Selection | Enquadrar seleção |
| `sl.free_mode` | FREE MODE | MODO LIVRE |
| `sl.freehand_polygon_loop_lasso_selection` | Freehand polygon loop lasso selection | Seleção livre por laço de polígono |
| `sl.freehand_raster_texture_painting_with_adjust` | Freehand raster texture painting with adjustable size and softness | Pintura raster livre de textura com tamanho e suavidade ajustáveis |
| `sl.fuse` | Fuse | Fuse |
| `sl.g_move_r_rotate_s_scale_shift_precision` | G move · R rotate · S scale · Shift precision | G move · R gira · S escala · Shift precisão |
| `sl.g_move_translate` | G: Move / Translate | G: Mover / Transladar |
| `sl.g_to_move_origin` | · G to move origin | · G para mover a origem |
| `sl.generate_volume_directly_by_dragging_depth_o` | • Generate volume directly by dragging depth or pressing Enter | • Gerar volume direto por arraste de profundidade ou Enter |
| `sl.generative_and_deformer_modifier_stack` | Generative and deformer modifier stack | Pilha de modificadores generativos e deformadores |
| `sl.gradient` | Gradient | Gradiente |
| `sl.gradient_radial` | Radial gradient | Gradiente radial |
| `sl.gradient_radial_description` | Radial gradient from the click to the release point, fading out | Gradiente radial do clique até o ponto de soltar, esmaecendo |
| `sl.grid` | Grid | Grade |
| `sl.ground` | Ground | Chão |
| `sl.ground_grid` | Ground Grid | Grade de Chão (Ground Grid) |
| `sl.hardness` | Hardness | Dureza |
| `sl.height` | Height | Altura |
| `sl.hide` | Hide | Ocultar |
| `sl.hint_orbit` | Orbit | Orbitar |
| `sl.hint_pan` | Pan view | Mover vista |
| `sl.hint_select` | Select | Selecionar |
| `sl.hint_zoom` | Zoom | Zoom |
| `sl.hold_alt_and_click_to_toggle_all_sections` | Hold Alt and click to toggle all sections | Segure Alt e clique para alternar todas as seções |
| `sl.icon_style` | Icon Style | Estilo dos Ícones |
| `sl.icosphere` | Icosphere | Icosfera |
| `sl.import` | Import | Importar |
| `sl.import_decal_image` | Import image as decal | Importar imagem como decalque |
| `sl.individual` | Individual | Individual |
| `sl.individual_alt_e` | Individual (Alt+E) | Individual (Alt+E) |
| `sl.insert` | Insert | Inserir |
| `sl.insert_a_connected_edge_loop_ring_across_qua` | Insert a connected edge loop ring across quad mesh geometry | Insere um anel de arestas conectado em uma malha de quads |
| `sl.inset` | Inset | Inset |
| `sl.inset_new_polygonal_boundary_inside_selected` | Inset new polygonal boundary inside selected faces | Cria um novo contorno poligonal dentro das faces selecionadas |
| `sl.instant_shortcut` | Instant Shortcut | Atalho Instantâneo |
| `sl.interactively_slice_through_edges_and_faces_` | Interactively slice through edges and faces with cutting line | Corta arestas e faces de forma interativa com uma linha de corte |
| `sl.interface_language` | Interface Language | Idioma da Interface |
| `sl.interface_theme` | Interface Theme | Tema da Interface |
| `sl.intersect` | Intersect | Intersectar |
| `sl.invert` | Invert | Inverter |
| `sl.invert_selection` | Invert Selection | Inverter seleção |
| `sl.invert_vertical_mouse_response` | Invert vertical mouse response | Inverter resposta vertical do mouse |
| `sl.isolate` | Isolate | Isolar |
| `sl.join` | Join | Join |
| `sl.join_the_operand_into_the_active_part` | Join the operand into the active part | Une o operando à parte ativa |
| `sl.keep_parts` | Keep Parts | Keep Parts |
| `sl.keep_parts_off` | Keep Parts: off | Keep Parts: desligado |
| `sl.keep_parts_on` | Keep Parts: on | Keep Parts: ligado |
| `sl.keep_the_overlapping_volume` | Keep the overlapping volume | Mantém o volume sobreposto |
| `sl.key_lmb` | LMB | LMB |
| `sl.key_mmb` | MMB | MMB |
| `sl.key_shift_mmb` | Shift+MMB | Shift+MMB |
| `sl.key_wheel` | Wheel | Roda |
| `sl.keyboard_global_shortcuts_work_tab_focus_bet` | Keyboard: global shortcuts work; Tab focus between controls is still partial. | Teclado: atalhos globais funcionam; foco por Tab entre controles ainda é parcial. |
| `sl.keyboard_shortcut_profile` | Keyboard Shortcut Profile | Perfil de Atalhos de Teclado |
| `sl.keyboard_shortcuts` | Keyboard & Shortcuts | Teclado & Atalhos |
| `sl.keymap_custom_badge` | custom | personalizado |
| `sl.keymap_delete_profile` | Delete profile | Apagar perfil |
| `sl.keymap_editor_hint` | Click a shortcut, then press the new keys (Esc cancels). Editing a built-in profile creates your own copy. | Clique num atalho e pressione as novas teclas (Esc cancela). Editar um perfil embutido cria uma cópia sua. |
| `sl.keymap_new_profile` | New profile from current | Novo perfil a partir do atual |
| `sl.keymap_press_key` | Press keys… | Pressione as teclas… |
| `sl.keymap_reset_binding` | Restore default | Restaurar padrão |
| `sl.knife` | Knife | Knife |
| `sl.language_region` | Language & Region | Idioma & Região |
| `sl.layer_locked` | Layer locked | Camada bloqueada |
| `sl.layer_visible` | Layer visible | Camada visível |
| `sl.layers` | Layers | Camadas |
| `sl.layout_truncated_only_the_first_2000_faces_a` | Layout truncated: only the first 2000 faces are drawn. | Layout truncado: apenas as primeiras 2000 faces são desenhadas. |
| `sl.line` | Line | Linha |
| `sl.lit_preview_with_a_single_directional_light_` | Lit preview with a single directional light (no shadows or ray tracing) | Prévia iluminada com uma luz direcional (sem sombras nem ray tracing) |
| `sl.lmb_click_to_place_shift_rmb_in_any_tool` | LMB Click to place · Shift+RMB in any tool | Clique esquerdo para posicionar · Shift+botão direito em qualquer ferramenta |
| `sl.load_image` | Load Image | Carregar imagem |
| `sl.localization_status` | Localization Status | Status de Localização |
| `sl.lock` | Lock | Bloquear |
| `sl.loop_cut` | Loop Cut | Loop Cut |
| `sl.make_face` | Make Face | Criar face |
| `sl.make_face_edge` | Make Face / Edge | Criar face / aresta |
| `sl.manage_scene_parts_groups_and_hierarchy` | Manage scene parts, groups and hierarchy | Gerencie partes, grupos e hierarquia da cena |
| `sl.mark_clear_seam` | Mark / Clear Seam | Marcar / limpar costura |
| `sl.mark_unmark_seams` | Mark/Unmark Seams | Marcar/desmarcar costuras |
| `sl.mask_selection` | Mask Selection: | Máscara de seleção: |
| `sl.mcp_automation` | MCP & Automation | MCP & Automação |
| `sl.measure` | Measure | Medir |
| `sl.measure_distances_and_angles_between_geometr` | Measure distances and angles between geometry elements in 3D | Mede distâncias e ângulos entre elementos da geometria em 3D |
| `sl.measurement_tags_on_multi_edge_selection` | Measurement Tags on Multi-Edge Selection | Tags de Medida em Multiseleção de Arestas |
| `sl.merge` | Merge | Mesclar |
| `sl.merge_center` | Merge Center | Mesclar no centro |
| `sl.merge_layer_down` | Merge layer down | Mesclar com a camada abaixo |
| `sl.micro_inspector` | Micro-Inspector | Micro-Inspector |
| `sl.migration_of_literal_strings_to_textid_in_pr` | Migration of literal strings to TextId in progress. | Migração de textos literais para TextId em andamento. |
| `sl.mode` | Mode | Modo |
| `sl.model` | MODEL | MODELAR |
| `sl.model_workspace` | Model Workspace | Espaço de modelagem |
| `sl.move` | Move | Mover |
| `sl.move_down` | Move Down | Mover para baixo |
| `sl.move_layer_down` | Move layer down | Mover camada para baixo |
| `sl.move_layer_up` | Move layer up | Mover camada para cima |
| `sl.move_up` | Move Up | Mover para cima |
| `sl.move_uv_down` | Move UV down | Mover UV para baixo |
| `sl.move_uv_left` | Move UV left | Mover UV para a esquerda |
| `sl.move_uv_right` | Move UV right | Mover UV para a direita |
| `sl.move_uv_up` | Move UV up | Mover UV para cima |
| `sl.new_open_save_import_export_projects_and_sce` | New, open, save, import/export projects and scene files | Novo, abrir, salvar, importar/exportar projetos e arquivos de cena |
| `sl.no_extension_is_loaded_by_this_interface_in_` | No extension is loaded by this interface in this version. | Nenhuma extensão é carregada por esta interface nesta versão. |
| `sl.no_reference_image_loaded_click_the_box_to_l` | No reference image loaded. Click the box to load blueprints or photo references. | Nenhuma imagem de referência carregada. Clique na caixa para carregar blueprints ou fotos de referência. |
| `sl.non_color_axis_differentiation_color_blindne` | Non-Color Axis Differentiation (Color Blindness) | Diferenciação Não-Cromática de Eixos (Daltonismo) |
| `sl.nothing_selected` | Nothing selected | Nada selecionado |
| `sl.nothing_selected_hint` | Click a part in the viewport or in the Parts list to see its transform, material and data. | Clique numa parte na viewport ou na lista Parts para ver posição, material e dados. |
| `sl.numeric_property_field` | Numeric property field | Campo numérico |
| `sl.obj_import_and_gltf_export_are_native_featur` | OBJ import and glTF export are native features, available from the File menu. | Importar OBJ e exportar glTF são recursos nativos, acessíveis pelo menu Arquivo. |
| `sl.object` | Object | Objeto |
| `sl.object_metadata_visibility_and_display_optio` | Object metadata, visibility and display options | Metadados, visibilidade e opções de exibição do objeto |
| `sl.off` | OFF | DESLIGADO |
| `sl.offset` | Offset | Deslocamento |
| `sl.on` | ON | LIGADO |
| `sl.opacity` | Opacity | Opacidade |
| `sl.opacity_less` | Less opacity | Menos opacidade |
| `sl.opacity_more` | More opacity | Mais opacidade |
| `sl.open_an_existing_project_from_disk_p3d` | Open an existing project from disk (.p3d) | Abre um projeto existente do disco (.p3d) |
| `sl.open_application_preferences_keymaps_and_the` | Open application preferences, keymaps and theme settings | Abre preferências, atalhos e tema do aplicativo |
| `sl.open_or_collapse_the_bottom_project_asset_li` | Open or collapse the bottom project asset library | Abre ou recolhe a biblioteca de Assets do projeto na parte inferior |
| `sl.open_or_collapse_the_scene_parts_hierarchy_o` | Open or collapse the scene parts hierarchy outliner | Abre ou recolhe o Outliner da hierarquia de partes da cena |
| `sl.open_overflow_menu_with_additional_modeling_` | Open overflow menu with additional modeling operations | Abre o menu extra com mais operações de modelagem |
| `sl.open_project` | Open project | Abrir projeto |
| `sl.open_reference_image_manager_to_load_bluepri` | Open Reference Image Manager to load blueprints and background references | Abre o gerenciador de imagens de referência para blueprints e fundos |
| `sl.operand` | Operand | Operando |
| `sl.ortho` | Ortho | Orto |
| `sl.orthographic` | Orthographic | Ortográfica |
| `sl.outline` | Outline | Outline |
| `sl.overlaid_wireframe_overlay` | Overlaid Wireframe Overlay | Wireframe Overlay sobreposto |
| `sl.pack_and_arrange_uv_islands_efficiently_with` | Pack and arrange UV islands efficiently without overlap | Empacota e organiza ilhas UV com eficiência, sem sobreposição |
| `sl.pack_islands` | Pack Islands | Pack Islands |
| `sl.paint` | PAINT | PINTAR |
| `sl.paint_canvas_close` | Close canvas window | Fechar janela do canvas |
| `sl.paint_canvas_open` | Open the canvas in a floating window | Abrir o canvas em janela flutuante |
| `sl.paint_color` | Paint color | Cor da tinta |
| `sl.paint_isolate_description` | Hide every other object so only the active one shows; click again to bring them back | Esconde os outros objetos para mostrar só o ativo; clique de novo para trazê-los de volta |
| `sl.paint_opt_brightness_contrast` | Brightness & contrast | Brilho e contraste |
| `sl.paint_opt_connected_pixels` | Connected pixels | Pixels conectados |
| `sl.paint_opt_face` | Face | Face |
| `sl.paint_opt_first_face` | First face | Primeira face |
| `sl.paint_opt_first_object` | First object | Primeiro objeto |
| `sl.paint_opt_grain` | Grain | Granulado |
| `sl.paint_opt_hue_saturation` | Hue & saturation | Matiz e saturação |
| `sl.paint_opt_invert` | Invert | Inverter |
| `sl.paint_opt_levels` | Levels | Níveis |
| `sl.paint_opt_none` | None | Nenhuma |
| `sl.paint_opt_object` | Object | Objeto |
| `sl.paint_opt_pixelate` | Pixelate | Pixelizar |
| `sl.paint_opt_posterize` | Posterize | Posterizar |
| `sl.paint_opt_screen` | Screen | Tela |
| `sl.paint_opt_selected_faces` | Selected faces | Faces selecionadas |
| `sl.paint_opt_surface` | Surface | Superfície |
| `sl.paint_opt_uv_island` | UV island | Ilha UV |
| `sl.paint_pip` | Paint on the 2D canvas with a live 3D inset | Pintar no canvas 2D com inset 3D ao vivo |
| `sl.paint_pip_back` | Back to the 3D view | Voltar para a vista 3D |
| `sl.paint_select_description` | Switch the object being painted or pick faces to paint; Shift adds, double-click picks a whole UV island | Troca o objeto que está sendo pintado ou escolhe faces para pintar; Shift soma, duplo clique escolhe a ilha UV inteira |
| `sl.paint_workspace` | Paint Workspace | Espaço de pintura |
| `sl.parametric_6_sided_hexahedral_box_with_width` | Parametric 6-sided hexahedral box with width, height and depth | Caixa paramétrica de 6 faces com largura, altura e profundidade |
| `sl.parametric_circular_cylinder_with_top_bottom` | Parametric circular cylinder with top/bottom cap options | Cilindro circular paramétrico com tampas superior/inferior opcionais |
| `sl.part_selection_color` | Part selection color | Cor de seleção da parte |
| `sl.partial_coverage_menus_and_tooltips_are_tran` | Partial coverage: menus and tooltips are translated; several status messages still mix Portuguese and English. | Cobertura parcial: menus e tooltips traduzidos; várias mensagens de status ainda misturam português e inglês. |
| `sl.parts` | Parts | Peças |
| `sl.persp` | Persp | Persp |
| `sl.perspective` | Perspective | Perspectiva |
| `sl.petunia3d` | Petunia3D | Petunia3D |
| `sl.petunia_default` | Petunia Default | Petunia Padrão |
| `sl.pin_or_unpin_uv_vertices_of_the_selected_fac` | Pin or unpin UV vertices of the selected faces | Fixa ou solta os vértices UV das faces selecionadas |
| `sl.pin_parts` | Pin Parts | Fixar peças |
| `sl.pin_section` | Pin section | Fixar seção |
| `sl.pin_section_open` | Pin section open | Fixar seção aberta |
| `sl.pin_unpin_p` | Pin/Unpin (P) | Fixar/soltar (P) |
| `sl.pinned_open` | Pinned open | Fixada aberta |
| `sl.pipe_radius` | Pipe Radius | Raio do tubo |
| `sl.pipe_segments` | Pipe Segments | Segmentos do tubo |
| `sl.pivot` | Pivot | Pivô |
| `sl.place_3d_cursor_as_origin_for_new_objects_an` | Place 3D cursor as origin for new objects and transformations | Posiciona o cursor 3D como origem de novos objetos e transformações |
| `sl.plane` | Plane: | Plano: |
| `sl.plane_2` | Plane | Plano |
| `sl.plugins` | Plugins | Plugins |
| `sl.point` | Point | Point |
| `sl.point_mode_off` | Point Mode (Off) | Modo Point (desligado) |
| `sl.point_vertex_mode` | ✓ Point (Vertex) Mode | ✓ Modo Point (vértice) |
| `sl.poly` | POLY | POLY |
| `sl.polyhedral_geodesic_sphere_composed_of_equil` | Polyhedral geodesic sphere composed of equilateral triangles | Esfera geodésica poliédrica feita de triângulos equiláteros |
| `sl.portugues_brasil` | Português (Brasil) | Português (Brasil) |
| `sl.pos` | Pos:  | Pos:  |
| `sl.position` | Position | Posição |
| `sl.position_rotation_and_scale_coordinates` | Position, rotation and scale coordinates | Coordenadas de posição, rotação e escala |
| `sl.precision_0_1` | PRECISION 0.1× | PRECISÃO 0.1× |
| `sl.prefab_delete` | Delete prefab | Excluir prefab |
| `sl.prefab_empty_hint` | Select objects and choose Save as prefab to reuse them in any scene. | Selecione objetos e use Salvar como prefab para reutilizá-los em qualquer cena. |
| `sl.prefab_empty_title` | No prefabs yet | Nenhum prefab ainda |
| `sl.prefab_favorite` | Favorite | Favoritar |
| `sl.prefab_only_favorites` | Only favorites | Só favoritos |
| `sl.prefab_parts` | parts | peças |
| `sl.prefab_place` | Place in scene | Colocar na cena |
| `sl.prefab_update` | Update from selection | Atualizar com a seleção |
| `sl.prepare_surface` | Prepare surface | Preparar superfície |
| `sl.preview_base_color_texture_and_emission_with` | Preview base color, texture and emission with a simple directional light | Visualiza cor base, textura e emissão com uma luz direcional simples |
| `sl.primitive_properties` | Primitive Properties | Propriedades da primitiva |
| `sl.profile_2d_sketch_draw_profile` | Profile & 2D Sketch (Draw Profile) | Perfil & Esboço 2D (Draw Profile) |
| `sl.project_ref` | Project Ref | Project Ref |
| `sl.project_uv_coordinates_from_active_camera_pe` | Project UV coordinates from active camera perspective | Projeta coordenadas UV a partir da perspectiva da câmera ativa |
| `sl.projection` | Projection | Projeção |
| `sl.prop` | Prop | Prop |
| `sl.pull_new_vertices_and_faces_outward_from_sel` | Pull new vertices and faces outward from selected edges or faces | Puxa novos vértices e faces para fora a partir de arestas ou faces selecionadas |
| `sl.push_or_pull_faces_along_surface_normals_dir` | Push or pull faces along surface normals directly | Empurra ou puxa faces diretamente ao longo das normais da superfície |
| `sl.quick_actions_and_modeling_tools` | Quick actions and modeling tools | Ações rápidas e ferramentas de modelagem |
| `sl.quick_menu_to_create_standard_3d_parametric_` | Quick menu to create standard 3D parametric shapes | Menu rápido para criar formas 3D paramétricas padrão |
| `sl.quick_search_and_execute_any_command_tool_or` | Quick search and execute any command, tool or setting | Busque e execute qualquer comando, ferramenta ou ajuste |
| `sl.r_rotate` | R: Rotate | R: Rotacionar |
| `sl.radius` | Radius | Raio |
| `sl.rail_more_hint` | Right-click or use the corner mark for similar tools. | Botão direito ou a marca no canto mostram ferramentas parecidas. |
| `sl.reapply_the_most_recently_undone_operation` | Reapply the most recently undone operation | Reaplica a última operação desfeita |
| `sl.rect_2d` | Rect 2D | Ret. 2D |
| `sl.rectangle` | Rectangle | Retângulo |
| `sl.redo` | Redo | Refazer |
| `sl.reduced_motion_vestibular_sensitivity` | Reduced Motion (Vestibular Sensitivity) | Redução de Movimento (Sensibilidade Vestibular) |
| `sl.ref_align_view` | Align the camera to this view | Alinhar a câmera a esta vista |
| `sl.ref_clear_confirm` | Remove all reference images? | Remover todas as imagens de referência? |
| `sl.ref_clear_yes` | Remove all | Remover todas |
| `sl.ref_help` | Click a view to load an image; select it to adjust opacity, size and offset. | Clique numa vista para carregar uma imagem; selecione-a para ajustar opacidade, tamanho e deslocamento. |
| `sl.ref_images` | Ref Images | Imagens ref. |
| `sl.ref_remove` | Remove image | Remover imagem |
| `sl.ref_toggle_visible` | Show or hide | Mostrar ou ocultar |
| `sl.reference_images` | Reference Images | Imagens de referência |
| `sl.reference_images_manager_for_background_blue` | Reference Images manager for background blueprints and modeling guides | Gerenciador de imagens de referência para blueprints de fundo e guias de modelagem |
| `sl.region` | Region | Região |
| `sl.relax` | Relax | Relaxar |
| `sl.relax_uv` | Relax UV | Relaxar UV |
| `sl.remove_layer` | Remove layer | Remover camada |
| `sl.remove_selected_points_edges_faces_or_entire` | Remove selected points, edges, faces or entire objects | Remove Points, arestas, faces ou objetos inteiros selecionados |
| `sl.rename` | Rename | Renomear |
| `sl.reset_0_0_0` | Reset (0,0,0) | Resetar (0,0,0) |
| `sl.reset_accent` | Theme default | Padrão do tema |
| `sl.reset_camera_zoom_orientation_and_position_t` | Reset camera zoom, orientation and position to default origin | Restaura zoom, orientação e posição da câmera para a origem padrão |
| `sl.reset_e96a00` | Reset (#E96A00) | Reset (#E96A00) |
| `sl.reset_to_default_color` | Reset to default color | Restaurar cor padrão |
| `sl.reset_view` | Reset view | Resetar vista |
| `sl.resize_active_selection_proportionally_or_al` | Resize active selection proportionally or along specific axes | Redimensiona a seleção ativa de forma proporcional ou por eixo |
| `sl.resize_panel_height` | Resize panel height | Redimensionar altura do painel |
| `sl.revert_the_most_recent_modeling_or_editing_o` | Revert the most recent modeling or editing operation | Reverte a operação de modelagem ou edição mais recente |
| `sl.revolve_selection_around_cursor_or_axis_into` | Revolve selection around cursor or axis into lathe surface | Revolve a seleção em torno do cursor ou de um eixo, criando uma superfície de torno |
| `sl.right_angled_triangular_prism_3d_ramp_mesh` | Right-angled triangular prism 3D ramp mesh | Prisma triangular retângulo 3D, em forma de rampa |
| `sl.ring_radius` | Ring Radius | Raio do anel |
| `sl.ring_segments` | Ring Segments | Segmentos do anel |
| `sl.rings` | Rings | Anéis |
| `sl.rot` | Rot:  | Rot:  |
| `sl.rotate` | Rotate | Girar |
| `sl.rotate_2` | Rotate − | Girar − |
| `sl.rotate_3` | Rotate + | Girar + |
| `sl.rotate_active_selection_around_the_active_pi` | Rotate active selection around the active pivot point | Gira a seleção ativa em torno do pivô ativo |
| `sl.rotate_uv_90_degrees_clockwise` | Rotate UV 90 degrees clockwise | Girar UV 90 graus no sentido horário |
| `sl.rotate_uv_90_degrees_counter_clockwise` | Rotate UV 90 degrees counter-clockwise | Girar UV 90 graus no sentido anti-horário |
| `sl.rotate_uv_clockwise` | Rotate UV clockwise | Girar UV no sentido horário |
| `sl.rotate_uv_counter_clockwise` | Rotate UV counter-clockwise | Girar UV no sentido anti-horário |
| `sl.rotation` | Rotation | Rotação |
| `sl.round_edge_bevel_options` | Round Edge (Bevel) Options | Opções de Round Edge (Bevel) |
| `sl.round_edges_or_vertices_with_smooth_chamfer_` | Round edges or vertices with smooth chamfer bevel fillets | Arredonda arestas ou vértices com chanfros suaves |
| `sl.s_scale` | S: Scale | S: Escalonar |
| `sl.sample_color_from_viewport_pixels_under_mous` | Sample color from viewport pixels under mouse cursor | Amostra a cor do pixel da viewport sob o cursor |
| `sl.save_project` | Save project | Salvar projeto |
| `sl.save_the_current_project_and_scene_to_disk` | Save the current project and scene to disk | Salva o projeto e a cena atuais no disco |
| `sl.saved` | Saved | Salvo |
| `sl.scale` | Scale | Escala |
| `sl.scale_2` | Scale − | Escala − |
| `sl.scale_3` | Scale + | Escala + |
| `sl.scale_uv_down` | Scale UV down | Reduzir escala UV |
| `sl.scale_uv_up` | Scale UV up | Aumentar escala UV |
| `sl.screen_reader_labels_and_roles_exist_only_on` | Screen reader: labels and roles exist only on some controls; WCAG compliance has not been audited yet. | Leitor de tela: rótulos e papéis existem só em parte dos controles; conformidade WCAG ainda não foi auditada. |
| `sl.search_command_placeholder` | Search commands… | Buscar comandos… |
| `sl.search_commands` | Search commands | Buscar comandos |
| `sl.segments` | Segments | Segmentos |
| `sl.select` | Select | Selecionar |
| `sl.select_all` | Select All | Selecionar Tudo |
| `sl.select_and_manipulate_entire_objects_as_disc` | Select and manipulate entire objects as discrete units | Seleciona e manipula objetos inteiros como unidades |
| `sl.select_and_transform_individual_vertices_poi` | Select and transform individual vertices (points) | Seleciona e transforma vértices (Points) individuais |
| `sl.select_and_transform_polygonal_boundary_edge` | Select and transform polygonal boundary edges | Seleciona e transforma arestas de contorno do polígono |
| `sl.select_and_transform_polygonal_planar_faces` | Select and transform polygonal planar faces | Seleciona e transforma faces planas do polígono |
| `sl.select_curve_segment` | Select and manipulate profile curves or segments | Seleciona e manipula curvas e segmentos de perfil |
| `sl.select_entire_shape` | Select and transform the entire 2D shape or volume | Seleciona e transforma a forma 2D inteira ou volume |
| `sl.select_region_face` | Select and extrude planar 2D regions | Seleciona e extruda regiões planares 2D |
| `sl.select_second_part_to_combine` | Select second part to Combine: | Selecione a segunda parte para combinar: |
| `sl.select_uv_vertices_edges_and_polygon_islands` | Select UV vertices, edges and polygon islands | Seleciona vértices, arestas e ilhas de polígonos UV |
| `sl.semi_transparent_surfaces_allowing_selection` | Semi-transparent surfaces allowing selection of occluded geometry | Superfícies semitransparentes que permitem selecionar geometria oculta |
| `sl.separate_selection` | Separate to new part | Criar nova parte da seleção |
| `sl.set_as_boolean_operand` | Set as Boolean Operand | Definir como operando booleano |
| `sl.set_origin` | Set Origin | Definir origem |
| `sl.settings` | Settings | Configurações |
| `sl.settings_about` | About | Sobre |
| `sl.settings_kw_0` | theme color accent scale zoom icons selection highlight | tema cor destaque escala zoom ícones seleção realce |
| `sl.settings_kw_1` | language idioma region | idioma language região |
| `sl.settings_kw_2` | keyboard shortcuts keymap keys | teclado atalhos keymap teclas |
| `sl.settings_kw_3` | accessibility motion contrast axes colorblind | acessibilidade movimento contraste eixos daltonismo |
| `sl.settings_kw_4` | tools drag snap click handles labels rail | ferramentas arrastar snap clique alças nomes trilho |
| `sl.settings_kw_5` | viewport camera light workplane navigation | viewport câmera luz plano navegação |
| `sl.settings_kw_6` | plugins extensions | plugins extensões |
| `sl.settings_kw_7` | automation mcp | automação mcp |
| `sl.settings_kw_8` | about version theme keymap | sobre versão tema keymap |
| `sl.settings_search` | Search settings | Buscar ajustes |
| `sl.shade_flat` | Shade Flat | Sombreamento chapado |
| `sl.shade_smooth` | Shade Smooth | Sombreamento suave |
| `sl.shading` | Shading | Sombreamento |
| `sl.shading_options` | Shading options | Opções de sombreamento |
| `sl.shape` | Shape | Forma |
| `sl.shape_builder` | Shape Builder | Shape Builder |
| `sl.shape_builder_description` | Drag across regions to merge them, Ctrl+drag to delete them, click to extract one | Arraste sobre regiões para fundi-las, Ctrl+arrastar para apagá-las, clique para extrair uma |
| `sl.shape_exclude` | Exclude overlap | Excluir sobreposição |
| `sl.shape_intersect` | Intersect shapes | Interseção de formas |
| `sl.shape_subtract` | Subtract shapes | Subtrair formas |
| `sl.shape_unite` | Unite shapes | Unir formas |
| `sl.shift_a_add_primitive` | Shift+A: Add Primitive | Shift+A: Adicionar Primitiva |
| `sl.shift_precision_0_1` | Shift: Precision 0.1× | Shift: precisão 0,1× |
| `sl.show` | Show | Mostrar |
| `sl.show_texture` | ✓ Show Texture | ✓ Mostrar textura |
| `sl.show_texture_2` | Show Texture | Mostrar textura |
| `sl.sides` | Sides | Lados |
| `sl.size` | Size | Tamanho |
| `sl.sketch` | Sketch | Esboço |
| `sl.sketch_profile` | Sketch / Profile | Esboço / Perfil |
| `sl.slice` | Slice | Slice |
| `sl.slicer_slice_loop_cut` | Slicer (Slice / Loop Cut) | Fatiador (Slice / Loop Cut) |
| `sl.slide` | Slide | Deslizar |
| `sl.smooth_gradual_airbrush_spray_effect_onto_ac` | Smooth gradual airbrush spray effect onto active texture | Aplica um spray suave e gradual de aerógrafo na textura ativa |
| `sl.smooth_unpinned_uv_island_corners_to_minimiz` | Smooth unpinned UV island corners to minimize distortion | Suaviza cantos de ilhas UV não fixados para reduzir distorção |
| `sl.smoothly_deform_neighboring_vertices_with_pr` | Smoothly deform neighboring vertices with proportional falloff curve | Deforma suavemente os vértices vizinhos com curva de queda proporcional |
| `sl.snap` | SNAP | SNAP |
| `sl.snap_2` | Snap | Snap |
| `sl.snap_transformations_to_grid_intervals_verti` | Snap transformations to grid intervals, vertices, edges or faces | Ajusta transformações a intervalos da grade, vértices, arestas ou faces |
| `sl.space_close_esc_cancel` | Space: close · Esc: cancel | Space: fechar · Esc: cancelar |
| `sl.space_floating_micro_inspector` | Space: Floating Micro-Inspector | Espaço: Micro-Inspector flutuante |
| `sl.sphere` | Sphere | Esfera |
| `sl.spin` | Spin | Spin |
| `sl.spin_lathe` | Spin / Lathe | Girar / Torno |
| `sl.split_back` | Back | Trás |
| `sl.split_close` | Close split view | Fechar vista dividida |
| `sl.split_disabled` | Split (Disabled) | Split (Desativado) |
| `sl.split_front` | Front | Frente |
| `sl.split_left` | Left | Esq |
| `sl.split_persp` | Persp | Persp |
| `sl.split_right` | Right | Dir |
| `sl.split_t` | Split (T) | Dividir (T) |
| `sl.split_toggle` | Split viewport | Dividir viewport |
| `sl.split_toggle_hint` | Show a second 3D view to inspect another side of the model | Mostra uma segunda vista 3D para ver outro lado do modelo |
| `sl.split_too_narrow` | The viewport is too narrow to split. | A viewport está estreita demais para dividir. |
| `sl.split_top` | Top | Topo |
| `sl.standard_matte_shaded_surfaces_with_basic_di` | Standard matte shaded surfaces with basic directional lighting | Superfícies foscas padrão com iluminação direcional básica |
| `sl.step_1_0_m_subdivisions_0_1_m` | Step 1.0 m (Subdivisions 0.1 m) | Passo 1.0 m (Subdivisões 0.1 m) |
| `sl.step_90` | Step 90°: | Passo 90°: |
| `sl.stitch` | Stitch | Costurar |
| `sl.stitch_seams` | Stitch Seams | Costurar bordas |
| `sl.subdivide` | Subdivide | Subdividir |
| `sl.subdivide_faces_into_quad_quarters_creating_` | Subdivide faces into quad quarters creating finer geometry | Subdivide faces em quatro quads, criando geometria mais fina |
| `sl.subdivisions` | Subdivisions | Subdivisões |
| `sl.subtract_the_operand_from_the_active_part` | Subtract the operand from the active part | Subtrai o operando da parte ativa |
| `sl.surface_shaders_colors_and_textures` | Surface shaders, colors and textures | Shaders, cores e texturas da superfície |
| `sl.symmetry` | Symmetry: | Simetria: |
| `sl.symmetry_x` | Mirror strokes on X | Espelhar traços em X |
| `sl.symmetry_y` | Mirror strokes on Y | Espelhar traços em Y |
| `sl.symmetry_z` | Mirror strokes on Z | Espelhar traços em Z |
| `sl.tab_selection_mode_v_e_f_obj` | Tab: Selection Mode (V/E/F/Obj) | Tab: Modo de Seleção (V/E/F/Obj) |
| `sl.target` | Target: | Alvo: |
| `sl.target_canonical_vocabulary_point_round_edge` | Target canonical vocabulary: Point, Round Edge, Fuse, Cut, Connect. | Vocabulário canônico alvo: Point, Round Edge, Fuse, Cut, Connect. |
| `sl.texture` | Texture | Textura |
| `sl.texture_layers` | Texture Layers | Camadas de textura |
| `sl.texture_painting_layer_management_and_brush_` | Texture painting, layer management, and brush tools | Pintura de texturas, camadas e pincéis |
| `sl.the_mcp_server_is_a_separate_component_this_` | The MCP server is a separate component; this screen does not start it or monitor its state. | O servidor MCP é um componente separado; esta tela não o inicia nem monitora seu estado. |
| `sl.this_effect_has_no_parameters` | This effect has no parameters. | Este efeito não tem parâmetros. |
| `sl.toggle_between_perspective_3d_projection_and` | Toggle between perspective 3D projection and orthographic view | Alterna entre projeção 3D em perspectiva e vista ortográfica |
| `sl.toggle_panels_layouts_fullscreen_mode_and_ac` | Toggle panels, layouts, fullscreen mode and accessibility options | Alternar painéis, layouts, tela cheia e opções de acessibilidade |
| `sl.toggle_seams_on_the_selected_face` | Toggle seams on the selected face | Alterna as costuras na face selecionada |
| `sl.toggle_the_scene_parts_outliner_and_hierarch` | Toggle the scene parts outliner and hierarchy list | Alterna o Outliner de partes e a lista de hierarquia da cena |
| `sl.tool_confirmation_mode` | Tool Confirmation Mode | Modo de Confirmação de Ferramenta |
| `sl.tool_options` | Tool Options | Opções da ferramenta |
| `sl.tools` | Tools | Ferramentas |
| `sl.top_cap_off` | Top Cap: off | Tampa superior: desligada |
| `sl.top_cap_on` | Top Cap: on | Tampa superior: ligada |
| `sl.top_radius` | Top Radius | Raio superior |
| `sl.torus` | Torus | Toro |
| `sl.transform_quick_readout` | Transform Quick Readout | Leitura rápida de transformação |
| `sl.translate_active_selection_along_axes_or_vie` | Translate active selection along axes or viewport plane | Move a seleção ativa pelos eixos ou pelo plano da viewport |
| `sl.trim_enabled` | Trim (Enabled) | Trim (Ativado) |
| `sl.trim_mode_removes_the_cut_side_instead_of_sp` | Trim mode (removes the cut side instead of splitting) | Modo Trim (remove lado cortado ao invés de split) |
| `sl.trim_t` | Trim (T) | Aparar (T) |
| `sl.ui_scale_zoom` | UI Scale (Zoom) | Escala da UI (Zoom) |
| `sl.un_isolate` | Un-isolate | Desisolar |
| `sl.undo` | Undo | Desfazer |
| `sl.undo_redo_history_and_global_application_pre` | Undo, redo, history and global application preferences | Desfazer, refazer, histórico e preferências globais do app |
| `sl.unfold_3d_mesh_surface_into_flat_2d_uv_layou` | Unfold 3D mesh surface into flat 2D UV layout | Desdobra a superfície da malha 3D em um layout UV 2D plano |
| `sl.universal_3_in_1_gizmo_for_translate_rotate_` | Universal 3-in-1 gizmo for translate, rotate and scale | Gizmo universal 3 em 1 para mover, girar e escalar |
| `sl.unlock` | Unlock | Desbloquear |
| `sl.unpin_parts` | Unpin Parts | Desafixar peças |
| `sl.unpin_section` | Unpin section | Desafixar seção |
| `sl.unsaved` | Unsaved | Não salvo |
| `sl.unwrap` | Unwrap | Unwrap |
| `sl.uv` | UV | UV |
| `sl.uv_checkerboard` | UV Checkerboard | Xadrez UV |
| `sl.uv_checkerboard_off` | UV Checkerboard: Off | Xadrez UV: desligado |
| `sl.uv_checkerboard_on` | UV Checkerboard: On | Xadrez UV: ligado |
| `sl.uv_editor` | UV Editor | Editor UV |
| `sl.uv_operations` | UV Operations | Operações UV |
| `sl.uv_projection_unwrapping_seams_and_island_pa` | UV projection, unwrapping, seams, and island packing | Projeção UV, unwrap, seams e empacotamento de ilhas |
| `sl.uv_select` | UV Select | Seleção UV |
| `sl.uv_sphere_parameterized_by_longitude_segment` | UV sphere parameterized by longitude segments and latitude rings | Esfera UV com segmentos de longitude e anéis de latitude |
| `sl.uv_statistics` | UV Statistics | Estatísticas UV |
| `sl.uv_wireframe_overlay` | ✓ UV Wireframe Overlay | ✓ Sobreposição de wireframe UV |
| `sl.uv_wireframe_overlay_2` | UV Wireframe Overlay | Sobreposição de wireframe UV |
| `sl.uv_workspace` | UV Workspace | Espaço UV |
| `sl.vertex` | Vertex | Vértice |
| `sl.vertices` | Vertices | Vértices |
| `sl.view` | View | Vista |
| `sl.viewport_n_nslint_shell_bootstrap_ngpu_viewp` | VIEWPORT\n\nSlint shell bootstrap\nGPU viewport remains behind PetuniaViewport | VIEWPORT\n\nSlint shell bootstrap\nGPU viewport remains behind PetuniaViewport |
| `sl.viewport_rendering_shading` | Viewport Rendering & Shading | Renderização e Shading do Viewport |
| `sl.viewport_selection_color` | Selection color in the viewport | Cor da seleção na viewport |
| `sl.w_toggle_selection_select_box_select` | W: Toggle Selection (Select / Box Select) | W: Alternar Seleção (Select / Box Select) |
| `sl.wedge` | Wedge | Cunha |
| `sl.weld_coincident_3d_edges_that_have_split_uv_` | Weld coincident 3D edges that have split UV seams | Solda arestas 3D coincidentes que têm seams UV divididas |
| `sl.width` | Width | Largura |
| `sl.workplane_from_selection` | Set Workplane from Selection (3 Points / Face) | Plano de Trabalho pela Seleção (3 Pontos / Face) |
| `sl.workspaces` | Workspaces | Espaços de trabalho |
| `sl.x_ray` | X-Ray | X-Ray |
| `sl.x_ray_enabled` | X-Ray enabled | X-Ray ativado |
| `sl.x_ray_off` | X-Ray off | X-Ray desligado |
| `sl.x_ray_on` | X-Ray on | X-Ray ligado |
| `snap_kind.axis_x` | Along X axis | Paralelo ao eixo X |
| `snap_kind.axis_y` | Along Y axis | Paralelo ao eixo Y |
| `snap_kind.axis_z` | Along Z axis | Paralelo ao eixo Z |
| `snap_kind.grid` | Grid | Grade |
| `snap_kind.midpoint` | Midpoint | Ponto médio |
| `snap_kind.on_edge` | On edge | Na aresta |
| `snap_kind.on_face` | On face | Na face |
| `snap_kind.point` | Point | Ponto |
| `surface.alpha_lock` | Toggle alpha lock | Alternar trava de alfa |
| `surface.apply` | Apply surface paint | Aplicar pintura de superfície |
| `surface.boolean_albedo` | Boolean albedo | Albedo do booleano |
| `surface.cancel` | Cancel surface paint | Cancelar pintura de superfície |
| `surface.choose_decal` | Select a decal layer to use as the source. | Selecione uma camada de decalque para usar como fonte. |
| `surface.dithering` | Toggle ordered dithering | Alternar dithering ordenado |
| `surface.export_padding` | Export bleed (px) | Bleed na exportação (px) |
| `surface.finish_operation` | Confirm or cancel the active operation first. | Confirme ou cancele a operação ativa primeiro. |
| `surface.grow` | Increase projection scale | Aumentar escala da projeção |
| `surface.import_svg` | Import SVG profiles | Importar perfis SVG |
| `surface.mirror` | Mirror projection | Espelhar projeção |
| `surface.needs_reattach` | The surface changed. Reattach the path before applying. | A superfície mudou. Reanexe o caminho antes de aplicar. |
| `surface.palette_darker` | Palette ramp: previous color | Rampa da paleta: cor anterior |
| `surface.palette_lighter` | Palette ramp: next color | Rampa da paleta: próxima cor |
| `surface.palette_normal` | Palette ramp: normal paint | Rampa da paleta: pintura normal |
| `surface.path` | Path Paint | Pintar caminho |
| `surface.path_name` | Paint path | Caminho de pintura |
| `surface.pixel_perfect` | Toggle pixel perfect | Alternar pixel perfeito |
| `surface.project` | Project selected decal | Projetar decalque selecionado |
| `surface.projection_name` | Projection | Projeção |
| `surface.ribbon` | Path Paint: ribbon | Path Paint: faixa |
| `surface.rotate` | Rotate projection 45° | Girar projeção 45° |
| `surface.shrink` | Decrease projection scale | Diminuir escala da projeção |
| `surface.stamps` | Path Paint: repeated decal stamps | Path Paint: carimbos repetidos |
| `surface.stencil` | Paint through selected decal stencil | Pintar através do stencil do decalque |
| `surface.stencil_luma` | Toggle stencil alpha / luminance | Alternar alfa / luminância do stencil |
| `surface.stencil_name` | Stencil paint | Pintura com stencil |
| `surface.stencil_place` | Place stencil (Enter to paint) | Posicionar stencil (Enter para pintar) |
| `surface.svg_resolution_1024` | SVG decal: render at 1024 px | Decalque SVG: rasterizar em 1024 px |
| `surface.svg_resolution_512` | SVG decal: render at 512 px | Decalque SVG: rasterizar em 512 px |
| `surface.uv_details` |  Density: {density} px/unit; tiny islands: {tiny} Export bleed: {padding} px. Leave room between islands for mipmaps. |  Densidade: {density} px/unidade; ilhas minúsculas: {tiny} Bleed de exportação: {padding} px. Reserve espaço entre ilhas para os mipmaps. |
| `surface.uv_health` | Check UV health | Verificar saúde do UV |
| `surface.uv_report` | UV: {islands} islands; {overlaps} overlaps; {zero} zero area faces; {outside} corners outside UV | UV: {islands} ilhas; {overlaps} sobreposições; {zero} faces sem área; {outside} cantos fora do UV |
| `tool_grammar.adjust_hint` | Type a value and press Enter to adjust (same Undo) | Digite um valor e Enter para ajustar (mesmo Undo) |
| `tool_grammar.adjusted` | Last operation adjusted | Última operação ajustada |
| `tool_grammar.draw_ready` | Draw on {plane}: click to add points, drag for curves; with Auto, the face under the cursor becomes the plane | Desenhar em {plane}: clique para adicionar pontos, arraste para curvas; no Auto, a face sob o cursor vira o plano |
| `tool_grammar.expired` | The last operation can no longer be adjusted | A última operação não pode mais ser ajustada |
| `tool_grammar.gesture_hint` | Drag or type a value · Esc cancels · Right button: menu | Arraste ou digite um valor · Esc cancela · Botão direito: menu |
| `tool_grammar.last_operation` | Last operation | Última operação |
| `tool_grammar.needs_edge` | {tool}: click or drag an edge | {tool}: clique ou arraste uma aresta |
| `tool_grammar.needs_face` | {tool}: click or drag a face | {tool}: clique ou arraste uma face |
| `tool_grammar.no_face_selected` | No face selected; using Ground | Nenhuma face selecionada; usando o chão |
| `tool_grammar.primitive_kept` | Primitive kept · Ctrl+Z undoes | Primitiva mantida · Ctrl+Z desfaz |
| `tool_grammar.ready` | {tool}: drag on the mesh or click to select | {tool}: arraste sobre a malha ou clique para selecionar |
| `tool_grammar.workplane_auto` | Automatic work plane: face under the cursor or the world plane most parallel to the view | Plano de trabalho automático: face sob o cursor ou plano do mundo mais paralelo à vista |
| `tool_grammar.workplane_set` | Work plane: {plane} (locked) | Plano de trabalho: {plane} (travado) |
| `tool_properties.bevel_hint` | Supports one manifold convex edge with simple corners; one segment. | Suporta um edge convexo manifold com cantos simples; um segmento. |
| `tool_properties.bevel_width` | Round Edge width | Largura do Round Edge |
| `tool_properties.blocked` | Confirm or cancel the viewport operation before editing these fields. | Confirme ou cancele a operação na viewport antes de editar estes campos. |
| `tool_properties.choose_transform` | Choose a transform handle or use G/R/S. | Escolha uma alça de transformação ou use G/R/S. |
| `tool_properties.cuts` | Cuts | Cortes |
| `tool_properties.displacement` | Displacement | Deslocamento |
| `tool_properties.error_number` | Enter a finite number in every field. | Informe um número finito em cada campo. |
| `tool_properties.extrude_distance` | Extrude distance | Distância de extrusão |
| `tool_properties.inset_factor` | Inset factor | Fator de inset |
| `tool_properties.preview_hint` | Changes preview live. Enter applies; Esc cancels. | As alterações são pré-visualizadas em tempo real. Enter aplica; Esc cancela. |
| `tool_properties.profile_usage` | Click in an orthographic view to add points. Click near the first point to close the profile. | Clique em uma vista ortográfica para adicionar pontos. Clique perto do primeiro ponto para fechar o perfil. |
| `tool_properties.pushpull_distance` | Push/Pull distance | Distância de Push/Pull |
| `tool_properties.rotation_hint` | Angles are degrees, Euler XYZ, around the selection center. | Ângulos em graus, Euler XYZ, em torno do centro da seleção. |
| `tool_properties.rotation_xyz` | XYZ Rotation | Rotação XYZ |
| `tool_properties.scale_xyz` | Scale per axis | Escala por eixo |
| `tool_properties.title` | Tool Properties | Propriedades da ferramenta |
| `tool_properties.universal_transform` | Universal Transform | Transformação unificada |
| `tool_properties.value` | Value | Valor |
| `toolbar.columns` | Columns | Colunas |
| `toolbar.configure` | Configure Toolbar | Configurar Barra |
| `toolbar.family_hint` | Right-click for the tool family menu | Clique com o botão direito para o menu da família |
| `toolbar.move_down` | Move down | Descer |
| `toolbar.move_up` | Move up | Subir |
| `toolbar.one_column` | 1 column | 1 coluna |
| `toolbar.planned` | Planned feature | Recurso planejado |
| `toolbar.reset` | Reset toolbar | Redefinir barra |
| `toolbar.two_columns` | 2 columns | 2 colunas |
| `toolbar.visible` | Visible | Visível |
| `tools.active` | Active tool: {tool} | Ferramenta ativa: {tool} |
| `tools.add_primitive` | Add Primitive | Adicionar primitiva |
| `tools.annotate` | Annotate | Anotar |
| `tools.bevel` | Round Edge | Round Edge |
| `tools.connect` | Connect | Conectar |
| `tools.cursor_3d` | 3D Cursor | Cursor 3D |
| `tools.dissolve` | Dissolve | Dissolver |
| `tools.draw_profile` | Profile | Perfil |
| `tools.eraser` | Eraser | Borracha |
| `tools.extrude` | Extrude | Extrudar |
| `tools.extrude_individual` | Extrude Individual | Extrusão individual |
| `tools.flip_diagonal` | Flip Diagonal | Inverter diagonal |
| `tools.inset` | Inset | Inset |
| `tools.knife` | Knife | Faca |
| `tools.loop_cut` | Loop Cut | Corte em loop |
| `tools.measure` | Measure | Medir |
| `tools.merge` | Merge | Fundir |
| `tools.mirror` | Mirror | Espelho |
| `tools.move` | Move | Mover |
| `tools.paint` | Paint | Pintar |
| `tools.picker` | Picker | Conta-gotas |
| `tools.pivot` | Pivot | Pivô |
| `tools.poly_pen` | Poly Pen | Poly Pen |
| `tools.poly_pen_hint` | Drag a point, edge or face to move it; Ctrl-drag a border edge to extrude; click points to draw a polygon (Enter or first point closes); Ctrl-click a point to melt it | Arraste um ponto, aresta ou face para mover; Ctrl-arraste uma aresta de borda para extrudar; clique pontos para desenhar um polígono (Enter ou o primeiro ponto fecha); Ctrl-clique num ponto para derretê-lo |
| `tools.primitives` | Add | Adicionar |
| `tools.push_pull` | Push/Pull | Push/Pull |
| `tools.pushpull` | Push/Pull | Push/Pull |
| `tools.revolve` | Revolve | Revolução |
| `tools.rotate` | Rotate | Rotacionar |
| `tools.scale` | Scale | Escalar |
| `tools.select` | Select | Seleção |
| `tools.select_box` | Box Select | Seleção em caixa |
| `tools.select_lasso` | Lasso Select | Seleção em laço |
| `tools.slice` | Slice | Fatiar |
| `tools.subdivide` | Cut | Cortar |
| `tools.symmetrize` | Symmetrize | Simetrizar |
| `tools.transform` | Transform | Transformar |
| `tools.uv_project` | Project | Projetar |
| `tools.uv_seam` | Seam | Costura |
| `tools.uv_select` | UV Select | Seleção UV |
| `tools.uv_unwrap` | Unwrap | Desdobrar |
| `transform.dimensions` | Dimensions | Dimensões |
| `transform.link` | Link axes | Ligar eixos |
| `transform.position` | Position | Posição |
| `transform.relative_hint` | Rotation is relative to the current mesh. | Rotação é relativa à malha atual. |
| `transform.reset_origin` | Reset to Origin | Centralizar na Origem |
| `transform.rotation` | Rotation | Rotação |
| `transform.scale` | Scale | Escala |
| `transform.title` | Transform | Transform |
| `transform.unlink` | Unlink axes | Desligar eixos |
| `ui.accent_color` | Interface accent color | Cor de destaque da interface |
| `ui.accent_color_hint` | Active buttons, tools and focus. The selection in the viewport has its own color below. | Botões e ferramentas ativos e foco. A seleção na viewport tem a própria cor, abaixo. |
| `ui.action_cut` | Cut | Cortar |
| `ui.action_fuse` | Fuse | Fundir |
| `ui.action_intersect` | Intersect | Interseção |
| `ui.action_join` | Join | Juntar |
| `ui.action_loop_cut` | Loop Cut | Loop Cut |
| `ui.action_merge` | Merge | Mesclar |
| `ui.action_slice` | Slice | Fatiar |
| `ui.action_subdivide` | Subdivide | Subdividir |
| `ui.active_brush_color` | Active Brush Color | Cor ativa do pincel |
| `ui.active_tool` | Active tool | Ferramenta ativa |
| `ui.albedo_base_color` | Albedo (Base Color) | Albedo (cor base) |
| `ui.assets` | Asset Library | Assets |
| `ui.at_3d_cursor` | at 3D Cursor | no Cursor 3D |
| `ui.close` | Close | Fechar |
| `ui.collapse` | Collapse section | Recolher painel |
| `ui.collapse_inspector` | Collapse inspector | Recolher inspector |
| `ui.colors_too_close` | Accent and selection colors are too similar. Pick colors that are easier to tell apart. | As cores de destaque e de seleção estão parecidas demais. Escolha cores fáceis de distinguir. |
| `ui.decal_bake` | Bake Decal to Layer | Fixar Decalque na Camada |
| `ui.decal_hint` | Drag on 3D surface to place decal. Shift+drag: scale, Ctrl+drag: rotate. | Arraste na superfície 3D para posicionar. Shift+arraste: escala, Ctrl+arraste: rotação. |
| `ui.decal_position` | Position | Posição |
| `ui.decal_rotation` | Rotation | Rotação |
| `ui.decal_scale` | Scale | Escala |
| `ui.decal_transform` | Decal Transform (UV) | Transformação do Decalque (UV) |
| `ui.dock_split_hint` | Drag to resize panels | Arraste para redimensionar os painéis |
| `ui.duplicate` | Duplicate | Duplicar |
| `ui.expand` | Expand section | Expandir painel |
| `ui.expand_inspector` | Expand inspector | Expandir inspector |
| `ui.floating_inspector` | Floating Inspector | Inspector Flutuante |
| `ui.help` | Help | Ajuda |
| `ui.hide_part` | Hide part | Ocultar peça |
| `ui.highlight_thickness` | Selection highlight thickness | Espessura do destaque da seleção |
| `ui.inspector` | Inspector | Inspector |
| `ui.invert_vertical_drag` | Invert up/down for modeling tools | Inverter cima/baixo nas ferramentas de modelagem |
| `ui.language` | Language | Idioma |
| `ui.last_operation` | Last operation | Última operação |
| `ui.lock` | Lock | Bloquear |
| `ui.lock_part` | Lock part | Bloquear peça |
| `ui.loop_cut_hint` | Hover a quad edge ring; scroll to change cuts, click to place, Enter confirms. Esc cancels. | Passe sobre uma aresta de quads; role para mudar cortes, clique para posicionar e Enter confirma. Esc cancela. |
| `ui.material_advanced` | Advanced material settings | Configurações avançadas do material |
| `ui.material_alpha_blend` | Blend | Mistura |
| `ui.material_alpha_cutoff` | Alpha cutoff | Corte alfa |
| `ui.material_alpha_mask` | Mask | Máscara |
| `ui.material_alpha_mode` | Alpha mode | Modo alfa |
| `ui.material_alpha_opaque` | Opaque | Opaco |
| `ui.material_assign` | Assign material | Atribuir material |
| `ui.material_base_color` | Base color | Cor base |
| `ui.material_clear_texture` | Clear texture | Limpar textura |
| `ui.material_create_texture` | Create texture | Criar textura |
| `ui.material_duplicate` | Duplicate material | Duplicar material |
| `ui.material_editor` | Material editor | Editor de material |
| `ui.material_emission` | Emission | Emissão |
| `ui.material_emission_strength` | Emission strength | Intensidade da emissão |
| `ui.material_metallic` | Metallic | Metálico |
| `ui.material_name` | Name | Nome |
| `ui.material_new` | New material | Novo material |
| `ui.material_no_material` | No material assigned to this object. | Nenhum material atribuído a este objeto. |
| `ui.material_no_selection` | Select an object to edit its material. | Selecione um objeto para editar seu material. |
| `ui.material_no_texture` | No texture | Sem textura |
| `ui.material_normal_scale` | Normal scale | Escala do normal |
| `ui.material_profile` | Profile | Perfil |
| `ui.material_profile_emissive` | Emissive | Emissivo |
| `ui.material_profile_glass` | Glass / Transparent | Vidro / Transparente |
| `ui.material_profile_pbr` | PBR Standard | PBR Padrão |
| `ui.material_profile_toon` | Toon / Cel-Shading | Toon / Cel-Shading |
| `ui.material_profile_unlit` | Unlit / Flat | Sem iluminação |
| `ui.material_remove` | Remove material | Remover material |
| `ui.material_roughness` | Roughness | Rugosidade |
| `ui.material_texture_albedo` | Albedo texture | Textura de albedo |
| `ui.mesh` | Mesh | Malha |
| `ui.model_lasso_hint` | Draw a freeform region to select parts or mesh elements. | Desenhe uma região livre para selecionar peças ou elementos da malha. |
| `ui.model_position_hint` | Move the selection using the axes or mouse. | Mova a seleção pelos eixos ou com o mouse. |
| `ui.model_rotate_hint` | Rotate the selection around its pivot. | Gire a seleção ao redor do pivô. |
| `ui.model_scale_hint` | Resize the selection along an axis. | Redimensione a seleção ao longo de um eixo. |
| `ui.model_select_hint` | Click to select; drag on empty space for a selection box. | Clique para selecionar; arraste no espaço vazio para selecionar por caixa. |
| `ui.model_transform_hint` | Move with arrows, scale with squares, rotate with rings. | Mova pelas setas, escale pelos quadrados e gire pelos anéis. |
| `ui.modifier_add_mirror` | Add Mirror | Adicionar Mirror |
| `ui.modifier_add_symmetry` | Add Symmetry | Adicionar Symmetry |
| `ui.modifier_apply` | Apply modifier | Aplicar modificador |
| `ui.modifier_axis` | Axis | Eixo |
| `ui.modifier_direction` | Direction | Direção |
| `ui.modifier_mirror` | Mirror | Espelho |
| `ui.modifier_move_down` | Move modifier down | Mover modificador para baixo |
| `ui.modifier_move_up` | Move modifier up | Mover modificador para cima |
| `ui.modifier_negative_to_positive` | Negative to positive | Negativo para positivo |
| `ui.modifier_positive_to_negative` | Positive to negative | Positivo para negativo |
| `ui.modifier_remove` | Remove modifier | Remover modificador |
| `ui.modifier_symmetry` | Symmetry | Simetria |
| `ui.modifiers_empty` | No modifiers in the stack. | Sem modificadores na pilha. |
| `ui.more` | More… | Mais… |
| `ui.more_actions` | More actions | Mais ações |
| `ui.more_model_tools` | More modeling tools | Mais ferramentas de modelagem |
| `ui.no_tool` | Tool disabled in tools.toml | Ferramenta desligada no tools.toml |
| `ui.no_tool_parameters` | Select Extrude, Inset, Round Edge, Loop Cut or Profile to edit parameters here. | Selecione Extrude, Inset, Round Edge, Loop Cut ou Profile para editar os parâmetros aqui. |
| `ui.numeric_field_hint` | Click to enter an exact value, or drag to adjust. Hold Shift for precision. | Clique para digitar um valor exato ou arraste para ajustar. Segure Shift para precisão. |
| `ui.object_lock` | Lock | Bloqueio |
| `ui.object_name` | Name | Nome |
| `ui.object_no_selection` | Select an object to inspect its data. | Selecione um objeto para inspecionar seus dados. |
| `ui.object_visibility` | Visibility | Visibilidade |
| `ui.outliner` | Outliner | Outliner |
| `ui.pack_islands` | Pack Islands | Empacotar ilhas |
| `ui.parts` | Parts | Peças |
| `ui.parts_annotations` | Annotations | Anotações |
| `ui.parts_collection` | Collection | Coleção |
| `ui.parts_exit_isolate` | Exit isolation | Sair do isolamento |
| `ui.parts_isolate` | Isolate active object | Isolar objeto ativo |
| `ui.parts_measurements` | Measurements | Medições |
| `ui.parts_new_collection` | New collection | Nova coleção |
| `ui.parts_row_size` | Row size | Tamanho das linhas |
| `ui.pivot_hint` | Choose the center used by Move, Rotate, and Scale. | Escolha o centro usado por Move, Rotate e Scale. |
| `ui.place_in_scene` | Place in scene | Colocar na cena |
| `ui.preferences_save_failed` | Could not save preferences | Não foi possível salvar as preferências |
| `ui.primitive_freeze` | Make Editable | Converter em Malha |
| `ui.primitive_freeze_hint` | Freeze parametric primitive into static editable mesh | Congela a primitiva paramétrica em malha estática editável |
| `ui.primitive_frozen_status` | Primitive converted to editable mesh | Primitiva convertida em malha editável |
| `ui.primitive_parametric` | Parametric Primitive | Primitiva Paramétrica |
| `ui.profile_add_circle` | Add Circle | Add Círculo |
| `ui.profile_add_rect` | Add Rect | Add Retângulo |
| `ui.profile_canvas_hint` | Click on canvas to plot vertices or insert a 2D primitive. | Clique no canvas para marcar vértices ou inserir um primitivo 2D. |
| `ui.profile_close` | Close Profile | Fechar Profile |
| `ui.profile_cuts` | Cuts | Cortes |
| `ui.profile_depth` | Depth | Profundidade |
| `ui.profile_generate` | Generate Volume | Gerar volume |
| `ui.profile_hint` | Click to draw on the view plane; click the first point to close. Generate by extrude or revolve. | Clique para desenhar num plano da vista; clique no primeiro ponto para fechar. Depois gere por extrusão ou revolve. |
| `ui.profile_look_at_plane` | Look at plane | Olhar para o plano |
| `ui.profile_plane` | Plane | Plano |
| `ui.profile_plane_auto` | Auto | Auto |
| `ui.profile_plane_face` | Face | Face |
| `ui.profile_plane_ground` | Ground | Chão |
| `ui.profile_plane_view` | View | Vista |
| `ui.profile_points` | Points | Pontos |
| `ui.profile_presets` | 2D Profile Presets | Presets de perfil 2D |
| `ui.profile_revolve` | Revolve | Revolve |
| `ui.profile_sharp_corners` | Sharp Corners | Cantos Retos |
| `ui.profile_smooth_curves` | Smooth Curves | Suavizar Curvas |
| `ui.profile_smoothness` | Smoothness | Suavização |
| `ui.profile_sweep` | Sweep | Sweep |
| `ui.profile_wall_thickness` | Wall Thickness | Espessura de Parede |
| `ui.project_asset_library` | Project Asset Library | Biblioteca de Assets do Projeto |
| `ui.properties` | Properties | Propriedades |
| `ui.push_pull_hint` | Drag vertically or enter a distance to move selected faces without creating side walls. | Arraste verticalmente ou informe a distância para mover as faces selecionadas sem criar paredes. |
| `ui.quick_action_add` | Pin action | Fixar ação |
| `ui.quick_action_customize` | Pin or unpin quick actions | Fixar ou soltar ações rápidas |
| `ui.quick_action_done` | Done | Concluir |
| `ui.quick_action_remove` | Unpin action | Soltar ação |
| `ui.quick_action_reset` | Reset quick actions | Restaurar ações rápidas |
| `ui.quick_actions` | Quick actions | Ações rápidas |
| `ui.recovery_body` | Petunia3D found an autosave snapshot that is newer than the saved project. The previous session did not close cleanly. | O Petunia3D encontrou um snapshot de autosave mais recente que o projeto salvo. A sessão anterior não foi encerrada corretamente. |
| `ui.recovery_discard` | Discard snapshots | Descartar snapshots |
| `ui.recovery_keep` | Open saved project | Abrir projeto salvo |
| `ui.recovery_recover` | Recover snapshot | Recuperar snapshot |
| `ui.recovery_title` | Recover unsaved work? | Recuperar trabalho não salvo? |
| `ui.redock` | Re-dock the Properties panel into the sidebar | Reancorar o Painel de Propriedades na barra lateral |
| `ui.refs` | Reference images | Imagens de referência |
| `ui.rename` | Rename | Renomear |
| `ui.resize_panel_width` | Resize panel width | Redimensionar largura do painel |
| `ui.save_active_as_asset` | Save selection as prefab | Salvar seleção como prefab |
| `ui.search` | Search | Buscar |
| `ui.search_assets` | Search assets | Buscar assets |
| `ui.search_parts` | Search parts | Buscar peças |
| `ui.section_dock` | Dock panel | Ancorar painel |
| `ui.section_drag` | Drag to move | Arrastar para mover |
| `ui.section_pin_asset` | Pin to active asset | Fixar no asset ativo |
| `ui.section_pin_open` | Keep open | Manter aberto |
| `ui.section_unpin_asset` | Unpin asset | Desfixar asset |
| `ui.selected_parts_only` | Show selected parts only | Mostrar apenas peças selecionadas |
| `ui.selection_color` | Selection color (#RRGGBB) | Cor da seleção (#RRGGBB) |
| `ui.selection_color_invalid` | Selection color must use #RRGGBB | A cor da seleção deve usar #RRGGBB |
| `ui.selection_color_low_contrast` | Selection color needs more contrast against the viewport | A cor da seleção precisa de mais contraste com a viewport |
| `ui.show_part` | Show part | Mostrar peça |
| `ui.slice_hint` | Drag in the viewport to define the plane; both sides are kept. Enter confirms, Esc cancels. | Arraste na viewport para definir o plano; os dois lados são mantidos. Enter confirma, Esc cancela. |
| `ui.sort_assets` | Sort assets by name | Ordenar assets por nome |
| `ui.sort_parts` | Sort parts by name | Ordenar peças por nome |
| `ui.stats_faces` | Faces | Faces |
| `ui.stats_selection` | Selection | Seleção |
| `ui.stats_tris` | Tris | Tris |
| `ui.stats_verts` | Verts | Vértices |
| `ui.status_hint` | LMB select   ·   MMB orbit   ·   Esc cancel | LMB seleciona   ·   MMB orbita   ·   Esc cancela |
| `ui.tab_material` | Material | Material |
| `ui.tab_modifiers` | Modifiers | Modificadores |
| `ui.tab_object` | Object | Objeto |
| `ui.tab_parts` | Parts | Peças |
| `ui.tab_transform` | Transform | Transformar |
| `ui.theme` | Theme | Tema |
| `ui.thumbnail_size` | Thumbnail size | Tamanho das miniaturas |
| `ui.tool_card` | Tool | Ferramenta |
| `ui.tool_options` | Tool Options | Opções da ferramenta |
| `ui.tool_options_collapse` | Collapse tool options | Recolher opções da ferramenta |
| `ui.tool_options_expand` | Expand tool options | Expandir opções da ferramenta |
| `ui.tools` | Tools | Ferramentas |
| `ui.tools_menu` | Tools… | Ferramentas… |
| `ui.unlock_part` | Unlock part | Desbloquear peça |
| `ui.unwrap_mesh` | Unwrap Mesh | Desdobrar malha |
| `ui.vertical_drag_inverted` | Vertical tool drag inverted | Arrasto vertical invertido |
| `ui.vertical_drag_normal` | Vertical tool drag normal | Arrasto vertical normal |
| `ui.vertical_tool_drag` | Vertical tool drag | Arrasto vertical das ferramentas |
| `ui.view_gizmo` | View navigation | Navegação da vista |
| `ui.view_gizmo_hint` | Drag to orbit; click an axis to align the view | Arraste para orbitar; clique num eixo para alinhar a vista |
| `ui.view_lit` | Scene lighting | Iluminação da cena |
| `ui.view_lit_hint` | Preview scene lighting and materials together. | Pré-visualiza a iluminação da cena junto com os materiais. |
| `ui.view_material` | Material preview | Prévia de material |
| `ui.view_material_hint` | Preview material colors and textures. | Pré-visualiza as cores e texturas do material. |
| `ui.view_solid` | Solid view | Vista sólida |
| `ui.view_solid_hint` | Show solid geometry with viewport lighting. | Mostra a geometria sólida com a iluminação da viewport. |
| `ui.view_wireframe` | Wireframe view | Vista em arame |
| `ui.view_wireframe_hint` | Show mesh edges as the base shading mode. | Mostra as arestas da malha como modo de visualização base. |
| `ui.visible` | Visible | Visível |
| `ui.wire_overlay` | Wire overlay | Sobreposição de arestas |
| `ui.wire_overlay_hint` | Show mesh edges over the current view mode | Mostra as arestas sobre o modo de visualização atual |
| `ui.xray_opacity` | X-Ray opacity | Opacidade do X-Ray |
| `uv.faces` | Faces | Faces |
| `uv.hint` | Click: select face. Drag: move UVs. Wheel: scale. | Clique: seleciona face. Arraste: move UVs. Scroll: escala. |
| `uv.preview_3d` | 3D preview | Prévia 3D |
| `uv.reproject` | Planar | Planar |
| `uv.scale` | Scale | Escala |
| `uv.selected` | sel faces | faces sel |
| `uv.title` | UV editor | Editor UV |
| `view.frame` | Frame | Enquadrar |
| `view.frame_all` | Frame All | Enquadrar tudo |
| `view.toggle_split` | Split viewport | Dividir viewport |
| `viewport.axis_lock` | Lock axis | Travar eixo |
| `viewport.axis_unlock` | Unlock axis | Destravar eixo |
| `viewport.drop_to_instantiate` | Drop to Instantiate | Soltar para Instanciar |
| `viewport.orientation` | Orientation | Orientação |
| `viewport.overflow` | More viewport tools | Mais ferramentas da viewport |
| `viewport.overlays_tip` | Toggle Overlays display | Alternar exibição de Overlays |
| `viewport.pivot` | Pivot Point | Ponto de Pivô |
| `viewport.prop_tip` | Proportional Editing · O | Edição Proporcional · O |
| `viewport.snap_tip` | Magnetic Snapping · Shift+Tab | Snapping Magnético · Shift+Tab |
| `viewport.tri_tip` | Triangulation inspection (internal diagonals of quads/n-gons) | Inspeção de triangulação (diagonais internas de quads/n-gons) |
| `viewport.xray_tip` | X-Ray / Mesh transparency mode · Alt+Z | Modo Raio-X / Transparência de Malha · Alt+Z |
| `workspace.draw_description` | Shape level: draw profiles on planes and faces, push and pull regions into solids | Nível de forma: desenhe perfis em planos e faces, empurre e puxe regiões em sólidos |
| `workspace.draw_ready` | DRAW: shape tools — Sketch, Push/Pull on regions | DRAW: ferramentas de forma — Sketch, Push/Pull em regiões |
| `workspace.draw_title` | Draw Workspace | Workspace Desenho |
| `workspace.poly_description` | Component level: edit points, edges and faces with extrude, inset, round edge and cuts | Nível de componente: edite pontos, arestas e faces com extrude, inset, round edge e cortes |
| `workspace.poly_ready` | POLY: component tools — points, edges and faces | POLY: ferramentas de componente — pontos, arestas e faces |
| `workspace.poly_title` | Poly Workspace | Workspace Polígonos |
| `ws.animate` | ANIMATE | ANIMATE |
| `ws.model` | MODEL | MODEL |
| `ws.paint` | PAINT | PAINT |
| `ws.uv` | UV | UV |

