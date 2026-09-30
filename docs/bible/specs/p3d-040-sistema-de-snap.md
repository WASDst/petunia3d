# P3D-040 — Sistema de Snap

<aside>
🧩

Estado: **precisa auditoria** · Prioridade: P1.

</aside>

## Objetivo

Snap reutilizável para Grid/Increment/Vertex/Edge/Face conforme suporte real.

## Auditoria

Mapear targets, prioridade, tolerance, visual feedback e integração com Move/Rotate/Scale/Extrude. Identificar stubs.

## Arquitetura

`SnapQuery/SnapResult` neutros, consumidos por tools; input/UI apenas escolhe política. Evitar queries geométricas duplicadas em cada tool.

## Direção aprovada (2026-09-29, [ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md), Onda 3)

- Tolerância em **pixels** da tela (não em unidades de mundo), com o alvo mais próximo sempre vencendo (bubble cursor, [capítulo 45](../foundations/45-pesquisa-interacao-modelagem-referencias.md)).
- Tipos de ponto: extremidade, ponto médio, centro, na aresta, na face (ponto projetado, não só o centroide), interseção e grade do plano de trabalho.
- Inferência de direção: paralelo a eixo, paralelo/perpendicular à aresta sob o cursor e guias pontilhadas "a partir do ponto" (snap-dragging de Bier; SketchUp; Dynamic Guides do Cinema 4D); ângulo em passos de 15°.
- Cada snap mostra forma + cor + **rótulo**; nunca só cor.
- Trava por segurar `Shift` **ou** alternar por teclas (acessível); o estado da trava aparece no HUD.
- `is_snapped` significa que o ponto encaixou, não que o snap está ligado.
- Uma única passada de snap por gesto (sem snap duplo bridge + core).

## Implementação (2026-09-30, Onda 3 parte 1)

Motor em espaço de tela em `crates/core/src/inference.rs` (`snap_screen`,
`SnapKind`, `SnapMask`, `SnapAnchor`, `SnapGrid`), usado pelo Move e pelo desenho
de perfil numa única passada; o core não encaixa em `update_modal`. Estado por
requisito, pendências e evidências em
[`docs/development/snap-inference-gap-matrix.md`](../../development/snap-inference-gap-matrix.md).

## Dependências

P3D-009, P3D-021–029.

## Testes / DoD

Cada target, conflito entre targets, zoom/DPI, cancel e ausência de jitter.