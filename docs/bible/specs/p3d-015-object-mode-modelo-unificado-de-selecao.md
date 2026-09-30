# P3D-015 — Object Mode / Modelo Unificado de Seleção

<aside>
🧩

Decisão consolidada: **não expor Object Mode e Edit Mode como dois mundos separados** · Prioridade: P0.

</aside>

## Objetivo

Unificar a interação em quatro domínios: **Object / Face / Edge / Point** (`Point` é o termo público para vértice). Object permite editar o objeto completo; os demais operam componentes da mesh. Desde a revisão de 2026-09-29 ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)), esses domínios pertencem ao workspace **POLY**; o workspace **DRAW** usa `Shape / Curve / Point / Region`.

## UX

A troca de domínio é feita pelo segmented control do workspace e por atalhos do keymap (por exemplo, 1–4). `Tab` **não** alterna domínio: ele navega controles (capítulo 36). A UI mostra claramente o domínio ativo e não exige ritual de entrar/sair de Edit Mode. Implementação pendente: o código ainda usa `Tab` para alternar.

## Auditoria

Mapear o atual `Object Mode`, `Edit Mode`, seleção, tools, keymaps, header, shelf e commands. Preservar operações funcionais enquanto o estado público é simplificado.

## Arquitetura

Pode existir estado interno de edição para implementação, mas ele não deve dominar a UX nem duplicar comportamento. O estado semântico deve ser algo equivalente a `SelectionDomain::{Object, Vertex, Edge, Face}`.

## Dependências

P3D-016–020, P3D-074, P3D-076, P3D-090.

## Testes / DoD

Troca por UI/atalho, manutenção do último domínio por workspace, tools corretas por domínio, seleção sincronizada entre DRAW e POLY, undo e ausência de duplicação de Point/Edge/Face em regiões diferentes.