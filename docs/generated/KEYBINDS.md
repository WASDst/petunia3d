---
title: Catálogo de Perfis e Atalhos de Teclado
description: Referência canônica dos perfis de teclado e atalhos configuráveis (P3D-119)
---

<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->

# Catálogo de Perfis de Atalhos de Teclado (`Keybinds`)

> **Single Source of Truth (P3D-087, P3D-119)**
> O Petunia3D suporta múltiplos perfis de teclado canônicos e remapeamento via arquivos TOML.

## Perfis Canônicos Disponíveis

| ID do Perfil | Nome Amigável | Descrição |
| :--- | :--- | :--- |
| `petunia-default` | **Petunia Padrão** | Atalhos canônicos com acesso direto a ferramentas e navegação ágil |
| `petunia-simple` | **Petunia Simples** | Atalhos minimalistas focados em modelagem rápida sem combinações complexas |
| `petunia-notebook` | **Petunia Notebook** | Otimizado para laptops sem teclado numérico dedicado |
| `blender` | **Blender (Oficial)** | Mapeamento 100% fiel ao padrão do Blender (G/R/S, E, I, Ctrl+B, Shift+A, Tab) |
| `blender-notebook` | **Blender Notebook** | Padrão Blender adaptado para laptops sem teclado numérico |
| `maya` | **Autodesk Maya** | Padrão Maya (Q/W/E/R para Seleção, Mover, Rotacionar e Escalar, F para Frame) |
| `3ds-max` | **Autodesk 3ds Max** | Padrão 3ds Max (Q/W/E/R, Z para Zoom Extents, 1/2/4 para sub-objetos) |
| `cinema-4d` | **Maxon Cinema 4D** | Padrão Cinema 4D (E para Mover, R para Rotacionar, T para Escalar) |


## Mapeamento Canônico Padrão (`petunia-default`)

| Ação Semântica | Atalho Padrão |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Shift+Z</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Delete</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>F</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>Shift+A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>R</kbd> |
| `model.scale` | <kbd>S</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>0</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>T</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |


## Perfis Especializados em Disco

### Perfil: `3ds-max` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Y</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Delete</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>Shift+E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>Z</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>E</kbd> |
| `model.scale` | <kbd>R</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>4</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>4</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>W</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

### Perfil: `blender` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Shift+Z</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>X</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>F</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>Shift+A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>R</kbd> |
| `model.scale` | <kbd>S</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>4</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>G</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

### Perfil: `blender-notebook` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Shift+Z</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Delete</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>F</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>R</kbd> |
| `model.scale` | <kbd>S</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>4</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>G</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

### Perfil: `cinema-4d` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Y</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Backspace</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>D</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>H</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>R</kbd> |
| `model.scale` | <kbd>T</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>4</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>E</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

### Perfil: `maya` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Y</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Delete</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>Ctrl+E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>F</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>E</kbd> |
| `model.scale` | <kbd>R</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>Q</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>W</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

### Perfil: `petunia-notebook` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.redo` | <kbd>Ctrl+Shift+Z</kbd> |
| `global.rename` | <kbd>Ctrl+F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Backspace</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>F</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>R</kbd> |
| `model.scale` | <kbd>S</kbd> |
| `model.select_edge` | <kbd>F2</kbd> |
| `model.select_face` | <kbd>F3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>F4</kbd> |
| `model.select_vertex` | <kbd>F1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>G</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

### Perfil: `petunia-simple` (49 atalhos)

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>F1</kbd> |
| `global.redo` | <kbd>Ctrl+Y</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Space</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |
| `model.bevel` | <kbd>Ctrl+B</kbd> |
| `model.box_select` | <kbd>B</kbd> |
| `model.connect` | <kbd>Ctrl+J</kbd> |
| `model.delete` | <kbd>Delete</kbd> |
| `model.dissolve` | <kbd>X</kbd> |
| `model.draw_profile` | <kbd>Shift+P</kbd> |
| `model.duplicate` | <kbd>Shift+D</kbd> |
| `model.extrude` | <kbd>E</kbd> |
| `model.extrude_individual` | <kbd>Alt+E</kbd> |
| `model.frame_selection` | <kbd>F</kbd> |
| `model.inset` | <kbd>I</kbd> |
| `model.invert_selection` | <kbd>Ctrl+I</kbd> |
| `model.knife` | <kbd>K</kbd> |
| `model.loop_cut` | <kbd>Ctrl+R</kbd> |
| `model.merge` | <kbd>M</kbd> |
| `model.move` | <kbd>G</kbd> |
| `model.primitives` | <kbd>A</kbd> |
| `model.push_pull` | <kbd>P</kbd> |
| `model.rotate` | <kbd>R</kbd> |
| `model.scale` | <kbd>S</kbd> |
| `model.select_edge` | <kbd>2</kbd> |
| `model.select_face` | <kbd>3</kbd> |
| `model.select_linked` | <kbd>L</kbd> |
| `model.select_object` | <kbd>4</kbd> |
| `model.select_vertex` | <kbd>1</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>T</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `uv.toggle_seam` | <kbd>U</kbd> |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |
| `window.command_palette` | <kbd>Ctrl+P</kbd> |
| `window.reference_manager` | <kbd>Shift+R</kbd> |

