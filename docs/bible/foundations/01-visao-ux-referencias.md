# 01 — Visão, UX e Referências

O usuário deve conseguir começar por uma imagem de referência e sentir que está **desenhando o modelo**, em vez de precisar aprender topologia antes de conseguir criar uma forma reconhecível.

## Câmera contextual

| Contexto | Projeção padrão | Comportamento |
| --- | --- | --- |
| Front / Side / Top | Ortográfica | Traçado e alinhamento sem distorção de perspectiva. |
| Reference / Trace | Ortográfica | O plano de desenho fica travado na referência; a câmera continua navegável (orbitar, deslocar, aproximar). |
| Orbit | Perspectiva | Inspeção natural do volume. |
| Draw on Face | Câmera inalterada por padrão | A face vira o plano de trabalho (folha 2D). "Olhar para o plano" alinha a câmera sob comando ou por preferência; a câmera nunca se move sozinha. |

## Reference Sets

Um projeto pode associar imagens às vistas Front, Side, Back e Top. A troca de vista ativa automaticamente a referência correspondente.

### Recursos

- Opacity da referência.
- Lock de posição e escala.
- Overlay / X-Ray.
- Centerline visual.
- Silhouette Mode.
- Difference/Comparison overlay como evolução futura.
- Na V1, múltiplas referências usam alinhamento manual previsível de posição/escala/rotação. **Landmark Alignment** (Head, Hip, Feet etc.) fica para V1.x como evolução de calibração multi-view.

## Linguagem espacial amigável

O modo iniciante pode apresentar **Width / Height / Depth** em vez de exigir X/Y/Z o tempo todo. O modo técnico continua oferecendo os eixos convencionais.

## Seleção contextual

Evitar obrigar o iniciante a compreender imediatamente uma separação rígida entre Object Mode e Edit Mode. Desde a revisão de 2026-09-29 ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)), o nível de trabalho é escolhido pelo workspace: **DRAW** expõe **Shape · Curve · Point · Region**; **POLY** expõe **Object · Face · Edge · Point**, além de inferência contextual por clique/duplo clique.

## Smart Snap

Por padrão, snapping contextual detecta Vertex, Edge, Midpoint, Center, Grid, centerline da referência e interseções de profiles, com inferência de direção (eixos, paralelo, perpendicular) no estilo snap-dragging/SketchUp. Cada snap é mostrado com forma, cor **e rótulo**, nunca só por cor ([capítulo 45](45-pesquisa-interacao-modelagem-referencias.md)). Configuração avançada permanece disponível.

## Filosofia de nomenclatura

Priorizar termos diretos como Draw, Cut, Push/Pull, Round Edge e Split. Tooltips podem apresentar o termo técnico equivalente, como Bevel, para facilitar transferência de conhecimento para outros softwares.

# Direção de interface documentada

A fase de UI/UX passa a ter documentação especializada. Esta página continua definindo a intenção de uso e referências; detalhes visuais e de componentes devem ser lidos nos capítulos seguintes:

- [22 — Referência de Interface: Análise do Figma Blender UI Redesign](22-referencia-interface-figma.md) — observações confirmadas da referência Figma e limites da análise.
- [23 — Macroarquitetura da Interface Petunia3D](23-macroarquitetura-interface.md) — aplicação dos princípios ao Petunia: viewport-first, Parts, Context, workspaces e painéis.
- [24 — Design System Visual: Tokens, Hierarquia e Estados](24-design-system-tokens-estados.md) — tokens e regras de consistência visual.
- [25 — Biblioteca de Componentes e Contratos de Interação](25-biblioteca-componentes-interacao.md) — componentes reutilizáveis e critérios de aceite.
- [26 — Figma → Implementação, Assets e Fundamentos de Acessibilidade](26-figma-implementacao-acessibilidade.md) — handoff, assets, controles reais, scaling e fundamentos de acessibilidade.

## Regra de precedência

A referência visual nunca deve reintroduzir conceitos de Blender que contrariem a filosofia de simplicidade do Petunia. Em especial, não usar `Object/Edit Mode`, taxonomia de Properties de Render/World/ViewLayer ou Timeline no workspace Model apenas porque existem no concept analisado.