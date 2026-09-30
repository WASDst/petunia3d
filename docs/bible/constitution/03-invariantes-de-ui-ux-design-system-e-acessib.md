# 03 — Invariantes de UI/UX, Design System e Acessibilidade

# Princípio

A UI deve parecer uma ferramenta criativa profissional especializada, não um conjunto de widgets egui. A identidade visual pertence ao Petunia, não ao toolkit.

# Hierarquia funcional

Separar claramente Workspace, Selection Domain, Tool, Command, Object Property, Tool Property, Asset e Viewport Setting.

# Regiões

- Header do viewport: seleção/contexto, menus, orientação/pivot, aids e shading.
- Toolbar vertical: tools persistentes do workspace (DRAW ou POLY, ADR 006).
- Shelf contextual: commands/tools do contexto atual, sem duplicar Selection Domain.
- Direita: Inspector (`Parts → Transform → Material → Object`); Tool Properties e "Última operação" separadas do Inspector.
- Assets: browser rápido; Project Model Library: gerenciador amplo separado.

# Tokens e metadata

Todo texto visível usa `TextId`; ícones usam `IconId`; cores/spacing/radius/states usam `ThemeToken`; atalhos exibidos vêm do keymap ativo.

# Toolkit

Slint é o frontend de produção; egui é legado de transição (capítulo 36). Respeitar as capacidades do toolkit. Priorizar spacing, hierarchy, icons, grouping, responsive overflow e states. Não exigir blur/glass/CSS complexo. Caches evitam parse/rasterização por frame.

# Acessibilidade

Hitboxes adequadas (alças e alvos do viewport com pelo menos 24×24 px lógicos), keyboard navigation, foco visível, tooltips, baixa ambiguidade e layouts úteis em 1366×768 até telas maiores. Todo arrasto tem alternativa sem arrastar (clicar-mover-clicar e valor digitado); limiar de arrasto e raio de picking são configuráveis (design baseado em habilidades, capítulo 45).

## Input routing e foco

Definir prioridade explícita: Modal > TextInput/FocusedWidget > painel/workspace > viewport context > Global. Text fields bloqueiam atalhos de tools; modal destrutivo não deixa command global atravessar; drag/drop e pointer capture possuem owner único até release/cancel.

Navegação do viewport (orbitar, deslocar, aproximar) é exceção: permanece disponível durante qualquer ferramenta ou gesto, usando botões e gestos que a ferramenta não consome. RMB abre menu de contexto e nunca cancela; `Esc` cancela (constituição 11).

## Escala e legibilidade

Validar UI scale/DPI, idioma com strings longas, contraste dos estados active/disabled/focus e icon-only controls com tooltip. Não depender apenas de cor para indicar selection/error.

## Motion e feedback

Animações são discretas e opcionais quando necessário; estados críticos permanecem compreensíveis sem animação. Feedback de erro/sucesso não depende exclusivamente de toast efêmero.

## Keyboard-only

Menus, command palette, settings e ações essenciais devem ser alcançáveis por teclado onde o toolkit permitir sem comprometer o viewport. Testes cobrem focus traversal e escape/cancel.