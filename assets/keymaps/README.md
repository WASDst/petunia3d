# Perfis de Mapeamento de Teclado (`assets/keymaps/`)

Este diretório é a **fonte única** dos perfis de keymap do Petunia3D (P3D-090).
Não existe um segundo diretório de keybinds: cada perfil é um único `.toml` aqui.
Os perfis canônicos são **embutidos no binário** (não dependem do diretório de
trabalho). Perfis do usuário vivem em `<config>/petunia3d/keymaps/user-*.toml`
(`~/.config/petunia3d/keymaps` no Linux) e são criados em **Configurações → Teclado**:
editar um perfil canônico cria uma cópia sua; os canônicos nunca são alterados.

## Perfis nativos

1. **`petunia-default.toml`** — perfil canônico do Petunia3D (default).
2. **`petunia-simple.toml`** — conjunto reduzido para quem está começando.
3. **`petunia-notebook.toml`** — adaptado para notebooks sem teclado numérico.
4. **`blender.toml`** — paridade com o padrão clássico do Blender (G/R/S, E, I, Ctrl+B, Shift+A, Tab).
5. **`blender-notebook.toml`** — padrão Blender adaptado a teclados compactos.
6. **`maya.toml`** — convenções do Autodesk Maya (Q/W/E/R).
7. **`3ds-max.toml`** — convenções do Autodesk 3ds Max.
8. **`cinema-4d.toml`** — convenções do Maxon Cinema 4D.

## Estrutura do arquivo

O arquivo tem uma seção `[profile]` com metadados e, em seguida, **uma seção por
namespace de ação**. A chave completa de uma ação é `<namespace>.<ação>` — é
essa string que aparece no Command Registry, na Cheat Sheet e no editor de
atalhos.

```toml
[profile]
id = "petunia-default"
name = "Petunia Padrão"
description = "Mapa de teclas canônico do Petunia3D."

[global]
undo = "Ctrl+Z"
redo = "Ctrl+Shift+Z"
save_project = "Ctrl+S"
cycle_mode = "Tab"

[model]
select_vertex = "1"
select_edge = "2"
select_face = "3"
select_object = "4"
move = "G"
rotate = "R"
scale = "S"
extrude = "E"

[paint]
paint = "B"
```

Ações desconhecidas são ignoradas; ações ausentes caem no default interno. Isso
mantém um perfil parcial válido.

## Resolução e conflitos

`petunia_config::keybinds` parte dos defaults Petunia e aplica por cima o perfil
escolhido: do usuário (`user-*`) ou canônico embutido. Ações novas portanto têm
atalho mesmo em perfis antigos. A detecção de conflitos
(P3D-091) distingue:

- **Exato** — mesmo namespace usando o mesmo atalho;
- **Sobreposição global** — atalho global sombreando um atalho contextual;
- **Tecla reservada** — uso de tecla protegida do sistema.

Os resultados aparecem no editor de atalhos de **Configurações → Teclado**, que
também exporta o mapa atual de volta para TOML.

## Namespaces com ação própria

- `global.*` — desfazer, salvar, `previous_tool` (`Space`), `micro_inspector` (`Shift+Space`);
- `window.*` — paleta de comandos, `workspace_draw/poly/paint/uv/animate` (`Ctrl+1..5`);
- `view.*`, `model.*`, `uv.*`;
- `paint.*` — `select`, `isolate`, `paint`, `airbrush`, `eraser`, `color_picker`, `fill`,
  `gradient`, `gradient_radial`, `line`, `rectangle`, `ellipse`, tamanho e dureza.

`global.*`, `view.*` e `window.*` valem em qualquer workspace; os demais só no seu contexto.
