# ADR 004 — Inspector translúcido, alça por proximidade e stack de modifiers

Status: aceito em 2026-09-24 por autorização explícita do responsável do produto.
Autoridade: [Livro Vivo, capítulo 36](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
(revisão MODEL de 2026-09-23) e [ADR 003](003-model-parts-in-inspector.md).
Este ADR registra o motivo; não cria uma segunda baseline.

## Contexto

Após uso, o produto exigiu: sidebar com a viewport visível através do fundo,
alça de recolhimento que só aparece por proximidade (clique recolhe, sem textos
"Inspetor"/seta/"MODEL"), rail colapsado legível, seções Material/Object
coerentes com o fluxo do legado egui, modifiers com aparência de stack Blender
e painel único de ferramenta fora do canto inferior.

## Decisão

1. Painel e rail usam `surface-raised` com alfa 0.9; seções mantêm fundo opaco.
2. Faixa de proximidade (14px) na borda do painel revela a alça; clique alterna
   o recolhimento. O header com título/seta/pill de workspace sai.
3. Rail colapsado vira 5 pílulas de ícone (Parts, Transform, Material, Object,
   Modifiers) com hover e Tooltip; clique expande e abre a seção.
4. Títulos de seção passam a usar `TextId` (fim do hardcode).
5. Material segue o fluxo legado: slot + gerenciar (Assign/New/Duplicate/Remove)
   antes de editar (Surface → Emission → Advanced → Albedo).
6. Object perde a microlinha redundante; Quick Actions ganham seção própria.
7. Modifiers: Add no topo, toggle com ícone Monitor, ChevronUp/Down + X;
   semântica top-aplica-primeiro preservada (eval sequencial no Core).
8. Tool Options + Operation HUD viram um card único dentro da viewport
   (topo-esquerda, como o popover legado); oculto em repouso.

## Divergências das referências (registradas, não aplicadas em silêncio)

- O manual Blender 5.2 descreve a Sidebar como região **opaca** (sem
  translucidez) e mantém **dois** elementos inferiores (Status Bar persistente
  + Adjust Last Operation transitório). O produto optou pela translucidez e
  pelo card único; vale para o shell Slint.
- Ícones de pílula usam o set Lucide disponível (ex.: Monitor no lugar do
  monitor/camera do Blender); duplicar ícones Blender seria violação de asset.

## Consequências

- ADR 003 continua válido; este ADR refina apenas apresentação e organização.
- Slots de extensão continuam controlados; nenhuma seção central muda de ordem.
- PAINT e UV não são redesenhados por este ADR.

## Emenda 2026-10-04

O rail de pílulas do §3 volta a ser exclusivo do estado recolhido: ele passa a
ser sobreposto à borda direita da viewport e o painel aberto o cobre (antes os
dois apareciam juntos). O rail tem 4 pílulas (Parts, Transform, Material,
Object) e mais Quick Actions até a F5; Modifiers vive dentro de Object. Ver a
emenda de mesma data no [ADR 005](005-modulos-inspector-dock-float-pin.md).

## Correção de interação 2026-10-07

O pedido de corrigir a reabertura e a sobreposição delimita dois estados:
prévia de hover mantém o rail recolhido visível e clicável, com o flyout ao lado;
clique abre a seção persistente. A animação não pode atravessar o rail nem
interceptar seus cliques. Recolher retorna às quatro pílulas. A ordem das seções
e o painel como superfície única são preservados.
