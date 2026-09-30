# P3D-083 — Tool Properties

<aside>
🧩

Estado: **boundary ainda fraca/incompleta** · Prioridade: P0.

</aside>

## Objetivo

Configurações da tool ativa separadas de propriedades do objeto.

## Exemplos

Primitive parameters, Bevel amount, Extrude mode, brush size/strength. Elas podem aparecer como popover, small panel ou região contextual, sem entrar no Object Inspector.

## Arquitetura

Tool metadata/state expõe parâmetros editáveis por descriptor/API; UI não acessa internals arbitrários. Mudanças que afetam operação modal integram preview/undo corretamente.

## Card "Última operação" (2026-09-29, [ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md))

Depois de cada gesto, o card da ferramenta mostra os parâmetros da operação recém-confirmada, editáveis por campo numérico ou valor digitado. Editar reaplica a operação dentro da mesma entrada de Undo (constituição 11). O card expira quando outro comando altera o documento. Chips de contexto iguais para todas as ferramentas: Pivot · Orientação · Snap · Simetria · Suave.

## Dependências

P3D-048, P3D-101, P3D-131.

## Testes / DoD

Troca de tool, defaults, cancel/confirm, persistência somente quando apropriada e nenhum vazamento de object properties.