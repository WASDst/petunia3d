# P3D-075 — Vertical Tool Toolbar

<aside>
🧩

Estado: **parcial; iconografia/semântica precisa refino** · Prioridade: P1.

</aside>

## Objetivo

Toolbar vertical para tools persistentes de interação, não catálogo de todos os commands.

## Conteúdo típico

Select/Cursor, Move, Rotate, Scale, Universal Transform, Annotate/Measure e outras tools realmente persistentes.

Desde 2026-09-29 ([ADR 007](../../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md)), o trilho é do workspace e todas as ferramentas de operação são persistentes (constituição 11):

- **DRAW:** Select, Linha/Polilinha, Retângulo, Círculo, Arco, Polígono, Push/Pull, Revolve, primitivas, plano de trabalho, Medir.
- **POLY:** Select, Move, Rotate, Scale, Universal, Extrude, Inset, Round Edge, Loop Cut, Cut, Poly Pen, Medir.

## Regras

Commands como Delete/Add Cube não ocupam a toolbar apenas por disponibilidade de espaço. Tool ativa tem estado inequívoco, hitbox adequada e tooltip com keybind atual.

## Dependências

P3D-088, P3D-101, P3D-114.

## Testes / DoD

Icon packs diferentes, DPI, seleção de tool, disabled state e ausência de handlers duplicados.