---
title: Cheatsheet de Atalhos (Perfil Petunia)
description: Atalhos do perfil canônico Petunia, gerados a partir do keymap (P3D-090, P3D-119)
---

<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->

# Cheatsheet de Atalhos — perfil `petunia-default`

> **Perfil canônico:** `Petunia` é o preset default do produto. Os demais presets
> oficiais são `Petunia Simple`, `Petunia Notebook`, `Blender-like`,
> `Blender-like Notebook`, `Maya-like`, `3ds Max-like` e `Cinema 4D-like`.
> A tabela completa de todos os perfis está em
> [Catálogo de Perfis e Atalhos](../generated/KEYBINDS.md).

> Nenhuma ferramenta depende de tecla física como regra de negócio: os binds
> apontam para `CommandId` e podem ser remapeados por perfil, com detecção de
> conflito e import/export em JSON versionado.

## Sistema & Arquivos

| Ação | Atalho |
| :--- | :---: |
| `global.command_palette` | <kbd>Ctrl+P</kbd> |
| `global.cycle_mode` | <kbd>Tab</kbd> |
| `global.help` | <kbd>H</kbd> |
| `global.micro_inspector` | <kbd>Shift+Space</kbd> |
| `global.previous_tool` | <kbd>Space</kbd> |
| `global.redo` | <kbd>Ctrl+Shift+Z</kbd> |
| `global.rename` | <kbd>F2</kbd> |
| `global.reset_camera` | <kbd>Home</kbd> |
| `global.save_project` | <kbd>Ctrl+S</kbd> |
| `global.toggle_projection` | <kbd>O</kbd> |
| `global.toggle_wireframe` | <kbd>Z</kbd> |
| `global.undo` | <kbd>Ctrl+Z</kbd> |

## Modelagem & Transformação

| Ação | Atalho |
| :--- | :---: |
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
| `model.make_face` | <kbd>F</kbd> |
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
| `model.shape_builder` | <kbd>Shift+M</kbd> |
| `model.slice` | <kbd>Shift+K</kbd> |
| `model.subdivide` | <kbd>Alt+W</kbd> |
| `model.tool_select` | <kbd>W</kbd> |
| `model.transform` | <kbd>T</kbd> |

## Pintura

| Ação | Atalho |
| :--- | :---: |
| `paint.airbrush` | <kbd>Shift+B</kbd> |
| `paint.color_picker` | <kbd>I</kbd> |
| `paint.eraser` | <kbd>E</kbd> |
| `paint.fill` | <kbd>G</kbd> |
| `paint.gradient` | <kbd>Shift+G</kbd> |
| `paint.isolate` | <kbd>/</kbd> |
| `paint.line` | <kbd>L</kbd> |
| `paint.paint` | <kbd>B</kbd> |
| `paint.rectangle` | <kbd>R</kbd> |
| `paint.select` | <kbd>V</kbd> |

## UV

| Ação | Atalho |
| :--- | :---: |
| `uv.toggle_seam` | <kbd>U</kbd> |

## Visualização & Câmera

| Ação | Atalho |
| :--- | :---: |
| `view.frame_all` | <kbd>Home</kbd> |
| `view.frame_selection` | <kbd>F</kbd> |
| `view.reset_camera` | <kbd>Shift+Home</kbd> |
| `view.toggle_projection` | <kbd>O</kbd> |
| `view.toggle_wireframe` | <kbd>Z</kbd> |
| `view.toggle_xray` | <kbd>Alt+Z</kbd> |

## Navegação e foco (contrato de input)

Estas entradas são contrato da UI Baseline V1, não binds de keymap:

| Entrada | Ação |
| :--- | :--- |
| `LMB` | selecionar/operar |
| `Shift + LMB` | adicionar/alternar seleção |
| `RMB` | context menu |
| `MMB` | orbit |
| `Shift + MMB` | pan |
| wheel/pinch | zoom |
| `Esc` | cancelar |
| `Enter` | confirmar operação pendente |
| `F6` / `Shift+F6` | navegar regiões principais |
| `Tab` / `Shift+Tab` | navegar controles dentro da região |

