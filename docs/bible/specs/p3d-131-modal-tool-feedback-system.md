# P3D-131 — Modal Tool Feedback System

<aside>
🧩

Novo item · Infraestrutura compartilhada · Prioridade: P1.

</aside>

## Objetivo

Criar feedback visual reutilizável para ferramentas modais como Extrude, Inset, Bevel e transforms.

## Descriptor proposto

Um modelo neutro como `ToolFeedback`/`OverlayPrimitive` pode conter origem, direção/guia, ponto atual, delta/valor, labels/hints e estado de axis/confirm/cancel. O frontend/render adapter decide como desenhar.

## UX

Linha pontilhada/guia discreta, valor quando útil, sem cobrir a geometria. O HUD mostra valor, unidade, texto digitado, trava ativa e o snap que efetivamente encaixou. A status bar mostra o próximo passo em linguagem simples e as ações do gesto atual (por exemplo: "Arraste ou digite um valor · Esc cancela · RMB menu"), sempre a partir do keymap ativo.

Revisão de 2026-09-29 ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)): **RMB abre menu de contexto e nunca cancela**; cancelar é `Esc`. A cota desenhada respeita o tipo do valor (distância, ângulo em arco, fator de escala). O buffer numérico é limpo ao iniciar, confirmar e cancelar.

## Arquitetura

Tools produzem semântica; não chamam `egui::Painter`. O sistema não deve existir apenas para Extrude.

## Dependências

P3D-029, P3D-031, P3D-032, P3D-083.

## Testes / DoD

Reutilização por múltiplas tools, cancel/confirm, DPI, ausência de alocações pesadas e feedback sincronizado com o estado real da operação.