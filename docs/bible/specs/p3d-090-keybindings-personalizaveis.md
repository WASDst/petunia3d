# P3D-090 — Keybindings personalizáveis

<aside>
🧩

Estado: **implementação precisa auditoria** · Prioridade: P1.

</aside>

## Objetivo

Toda ação configurável por `CommandId` + keymap/context, sem tools conhecendo teclas físicas.

## Modelo

InputTrigger estruturado (keyboard/mouse/wheel) é serializado em strings TOML apenas na fronteira. Suportar múltiplos bindings e unbound explícito.

## Contextos

Global, Viewport, Selection Domain, Outliner, Properties, Paint, UV, Timeline, TextInput e Modal, com prioridade definida.

## Dependências

P3D-091–099, P3D-100.

## Testes / DoD

Parsing/serialization, runtime switch, text input blocking, modal context, persistence e menus/tooltips atualizando shortcuts.

## Estado de implementação (2026-09-30)

Perfis canônicos embutidos no binário e **perfis do usuário** (`<config>/petunia3d/keymaps/user-*.toml`): criar a partir do atual, editar por captura de tecla (Esc cancela), restaurar o padrão por ação, apagar; editar um perfil canônico cria uma cópia. Conflitos (P3D-091) sinalizados na linha e no status. Novas ações: `global.previous_tool` (`Space`), `global.micro_inspector` (`Shift+Space`), `window.workspace_*` (`Ctrl+1..5`) e as ações `paint.*`. Evidência em [`keymap-shell-gap-matrix`](../../development/keymap-shell-gap-matrix.md).