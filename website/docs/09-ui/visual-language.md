# Linguagem visual da vista e da GUI (B8, aprovada em 2026-10-19)

> Status: **aprovada pelo usuário em 2026-10-19** ([ADR-C16](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuario), itens B8 e 15). Referências de outros programas servem só de ergonomia: nenhum ícone, cor, código ou captura deles entra no repositório.

## Intent

Dar à vista 3D e ao shell uma hierarquia visual única e previsível. Hoje cada overlay escolhe sua cor, espessura e rótulo, e o resultado é "cru" (relato do usuário de 2026-10-19). A Onda C cria muita interface nova e precisa já nascer neste estilo.

## Sources

Relato do usuário de 2026-10-19 ("o visual e o feedback da viewport e da GUI estão crus"), [roteiro do Shell v2](shell-v2-roadmap.md), tokens de `PetuniaTheme` (`crates/ui-cxxqt/cpp/petunia_theme.h`), restrições de acessibilidade do [AGENTS.md](../16-code-agents/index.md) (contraste, movimento reduzido, alternativas ao ponteiro).

## Princípios

1. **Hierarquia por importância, não por cor.** Do mais forte ao mais fraco: a seleção ativa, depois a ferramenta em curso (prévia, seta, valor), depois o hover, a seleção inativa, os guias (snap, eixos) e por fim a grade. Uma camada nunca compete com a de cima.
2. **Nada depende só de cor.** Cada estado também muda forma, traço ou texto: tracejado para o que é eco ou prévia, sólido para o que é real, preenchimento para o que está ativo e ícone que muda de forma (cadeado aberto ou fechado).
3. **Halo escuro sob todo traço de overlay.** Um traço claro de 1,5 a 2 px sobre um halo preto translúcido de 4 px lê bem sobre fundo claro, escuro e textura.
4. **O número mora perto da operação.** Valores da ferramenta aparecem junto da âncora ou da seta, não num canto (quadro de prévia, B3). O cartão do canto fica para os parâmetros completos.
5. **Sem animação obrigatória.** Nada informa só por movimento. Com movimento reduzido nada se anima, e o shell novo não usa animação nenhuma.
6. **Tokens, não literais.** Cores, espaços, raios e fontes saem de `PetuniaTheme`, porque o alto contraste e a escala de 150% e 200% precisam funcionar sem exceções por widget.

## Escala da vista 3D

| Camada                 | Traço                         | Cor (token)                    | Exemplo                                   | Status      |
|------------------------|-------------------------------|--------------------------------|-------------------------------------------|-------------|
| Seleção ativa          | 2 px sólido + halo            | `accentSelectionActive`        | contorno do objeto ativo                  | [x] existe  |
| Componentes de edição  | vértice 2 px (sel. 3 px), aresta 1 px | escuro; seleção na cor de seleção; face 22% + contorno | domínios vértice/aresta/face (GL) | [x] B12     |
| Ferramenta em curso    | 2 px sólido + halo, seta      | `accentPrimary`                | quadro de prévia (B3)                     | [x] B3      |
| Eco e prévia           | 1,5 px tracejado + halo       | branco 80% / preto 60%         | eco do pincel 3D↔2D (B2), knife           | [x] B2/B3   |
| Hover                  | ponto 3,5 px / traço 1,5–2,5 px | ciano 90%, face 12%          | pré-seleção de vértice/aresta/face        | [x] B12     |
| Guias                  | 1,5 px sólido + halo          | `accentPrimary`                | snap, eixo de restrição, prévias          | [x] B8      |
| Eixos do mundo         | 1 px, 62% da cor do eixo      | cores de eixo, com depth test  | X/Z no chão, Y oculto (B6)                | [x] B6      |
| Grade                  | 1 px, maior a cada 10         | cinza × opacidade da grade     | grade configurável (B6)                   | [x] B6      |

## Escala da GUI

| Elemento               | Regra                                                                                     | Status      |
|------------------------|-------------------------------------------------------------------------------------------|-------------|
| Campos numéricos       | quadro do campo é o alvo; prefixo de eixo colorido; cadeado à direita, discreto até travar | [x] B7      |
| Diálogos               | moldura própria (barra de título, borda 1 px `mid`), arrastar, redimensionar e teclado    | [x] B5      |
| Menus em torta         | disco central com o título, anel guia pontilhado e fatia em destaque (forma e cor)        | [x] B9      |
| Cor                    | um seletor só (`PetuniaColorPanel`) em todo o app                                          | [x] B1      |
| Ícones                 | traço 1,75 px num grid de 24, cantos e pontas arredondados (conjunto `petunia-outline`)   | [x] auditado: 287/287 conformes; `select_edge` usa 3,2 px de propósito (aresta em destaque) |
| Espaçamento            | múltiplos de `space(1)`; rótulo acima (decisão do usuário), margens e espaços de `space(1)`, campos em `fieldHeight()` (28 px) | [x] B13     |
| Tipografia             | Body para valores, Caption secundária para rótulos do painel; negrito só em títulos       | [x] B13     |

## Gap

- ~~Hover, guias e rótulos de snap usavam `QPalette::Highlight`.~~ Agora usam tokens do tema com halo (B8).
- As cores do cursor do pincel continuam literais (herdadas) e ainda precisam virar tokens de overlay para o alto contraste.
- ~~Ícones com traço e cantos diferentes~~: hipótese desfeita pela auditoria de 2026-10-19 (287 SVGs, todos com 1,75 px, `round` e grid 24).

## Verification

Capturas GL reais (EGL headless) do modo de edição antes e depois em `target/render-captures/` (teste `edit_mode_overlays_capture_for_visual_review`). Capturas do shell em `target/shell-captures/` (`shell_POLY_dense`, `shell_PAINT_compact`) e checks `panel:*`, `color:brush-panel-is-compact`. Resultados no [roteiro](shell-v2-roadmap.md#verification).

## Risks

- Mudar a hierarquia de cores pode atrapalhar quem já se acostumou ao visual atual. A ideia é fazer em passos pequenos, um tema de cada vez, sempre com o alto contraste conferido.

## Next

Tokens de overlay para o cursor do pincel; a Onda C nasce com estas métricas.
