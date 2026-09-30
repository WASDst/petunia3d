# P3D-092 — Petunia Default

<aside>
🧩

Perfil oficial principal · Prioridade: P1.

</aside>

## Objetivo

Keymap equilibrado, memorável e coerente com a filosofia Petunia, sem dependência essencial de numpad.

## Direção inicial

Q Select, G Move, R Rotate, S Scale, T Transform, E Extrude, I Inset, B Bevel, K Knife, Ctrl+R Loop Cut, Shift+D Duplicate, Delete/X, Ctrl+Z/Shift+Z, Ctrl+S/O/N, F Frame Selected e Command Palette configurável.

## Comportamento padrão (2026-09-29, [ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md))

No perfil Petunia, uma tecla de ferramenta **seleciona a ferramenta persistente**; a operação acontece por arrasto (alça ou em qualquer lugar), clicar-mover-clicar ou valor digitado (constituição 11). O comportamento modal estilo Blender (tecla → segue o mouse → clique) não é o padrão; ele existe no perfil Blender-like (P3D-095) e pela preferência de acessibilidade "arrastar sem segurar". As letras acima são direção inicial e podem mudar após validação de UX.

## Regra

A lista final precisa ser validada contra conflicts e UX real; não preservar um shortcut só porque foi sugerido historicamente.

## Dependências

P3D-090–091.

## Testes / DoD

Todos os commands essenciais acessíveis, conflicts zero/aceitos explicitamente e documentação/cheat sheet gerável.