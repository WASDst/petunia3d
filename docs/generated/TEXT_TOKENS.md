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

Total de chaves de localização cadastradas: **815**.

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
| `animate.auto_rig` | Auto-Rig | Auto-Rig |
| `animate.first_frame` | First frame | Primeiro frame |
| `animate.frame` | Frame | Frame |
| `animate.humanoid` | Humanoid | Humanoide |
| `animate.last_frame` | Last frame | Último frame |
| `animate.pause` | Pause | Pausar |
| `animate.play` | Play | Reproduzir |
| `animate.tip_first` | Jump to First Frame · Shift+Left | Ir ao Primeiro Frame · Shift+Left |
| `animate.tip_last` | Jump to Last Frame · Shift+Right | Ir ao Último Frame · Shift+Right |
| `animate.tip_next` | Step 1 Frame Forward · Right | Avançar 1 Frame · Right |
| `animate.tip_play` | Play / Pause Animation · Space | Reproduzir / Pausar Animação · Space |
| `animate.tip_prev` | Step 1 Frame Backward · Left | Voltar 1 Frame · Left |
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
| `file.import_obj` | Import OBJ… | Importar OBJ… |
| `file.new` | New project | Novo projeto |
| `file.open_project` | Open project… | Abrir projeto… |
| `file.quit` | Quit | Sair |
| `file.save` | Save | Salvar |
| `file.save_as` | Save as… | Salvar como… |
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
| `preferences.click_move_click` | Drag without holding the button (click, move, click) | Arrastar sem segurar o botão (clicar, mover, clicar) |
| `preferences.click_move_click_hint` | Click a handle: it follows the mouse until the next click. | Clique numa alça: ela segue o mouse até o próximo clique. |
| `preferences.colorblind_axes` | Colorblind axes differentiation (X, Y, Z labels) | Diferenciação não-cromática de eixos (Rótulos X, Y, Z) |
| `preferences.double_tap_interval` | Double-tap shortcut interval (ms) | Intervalo de duplo toque de atalho (ms) |
| `preferences.drag_threshold` | Distance before a drag starts (px) | Distância para começar a arrastar (px) |
| `preferences.multiselection_measure_tag` | Display average measure tag on multiple edge selection | Exibir tag de média na multiseleção de arestas |
| `preferences.reduced_motion` | Reduced motion (disable viewport animations) | Redução de movimento (desativa animações do viewport) |
| `preferences.snap_radius` | Snap radius (px) | Raio do snap (px) |
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
| `snap_kind.axis_x` | Along X axis | Paralelo ao eixo X |
| `snap_kind.axis_y` | Along Y axis | Paralelo ao eixo Y |
| `snap_kind.axis_z` | Along Z axis | Paralelo ao eixo Z |
| `snap_kind.grid` | Grid | Grade |
| `snap_kind.midpoint` | Midpoint | Ponto médio |
| `snap_kind.on_edge` | On edge | Na aresta |
| `snap_kind.on_face` | On face | Na face |
| `snap_kind.point` | Point | Ponto |
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
| `ui.save_active_as_asset` | Save Active as Asset | Salvar ativo como asset |
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
| `ws.model` | MODEL | MODEL |
| `ws.paint` | PAINT | PAINT |
| `ws.uv` | UV | UV |

