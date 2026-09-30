# P3D-076 — Contextual Tool Shelf

<aside>
🧩

Estado: **parcial e duplicando conceitos** · Prioridade: P0.

</aside>

## Objetivo

Shelf flutuante inferior dedicada a ações contextuais, não a repetir Selection Domain.

## Decisão

Remover Point/Edge/Face da shelf. A shelf mostra **commands** de um disparo do contexto atual; ferramentas persistentes (Extrude, Inset, Round Edge, Loop Cut, Cut) vivem no trilho do workspace (P3D-075, revisão de 2026-09-29). Em POLY, a shelf pode mostrar Connect, Subdivide, Merge, Dissolve, Flip Diagonal etc.; em DRAW, Converter em polígonos, Fechar perfil, Mirror etc. Paint/UV/Animation exibem conjuntos próprios.

## UX

Grupos com separadores, icon+label em largura ampla, icon-only + tooltip em largura curta e overflow menu quando necessário. Nunca overlap.

## Dependências

P3D-015, P3D-077, P3D-079, P3D-083.

## Testes / DoD

Todos os domínios/workspaces relevantes, resize, overflow, commands corretos e sem duplicação de estado.