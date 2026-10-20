# Acabamento da UI: comparação Slint (antiga) × Qt (atual) e as ondas D–F

> Status: **proposta de ondas de polimento** (2026-10-19), a partir do relato do usuário ("perceber o quão mal polida ainda está nossa UI") e da comparação entre a UI Slint removida e o shell Qt atual. As ondas D–F entram no [roteiro do Shell v2](shell-v2-roadmap.md). A [linguagem visual](visual-language.md) (B8, aprovada) é o régua: as ondas não inventam um estilo novo, elas aplicam o que já foi aprovado.

## Intent

Tornar a interface **acabada**, não apenas funcional. A estrutura do shell atual está certa (regiões, painéis, HUD, paleta); o que falta é densidade, hierarquia, ancoragem e consistência — e é isso que se vê no app aberto.

## Sources

- UI Slint removida em `584f8357` (`crates/ui-slint/ui/app.slint` e `components/controls.slint`, lidos no histórico).
- Shell Qt atual: `crates/ui-cxxqt/cpp/shell/` (app bar, trilho, painéis, HUD, gaveta) e as capturas reais em `target/shell-captures/` (`shell_POLY.png`, `shell_PAINT.png`, `shell_POLY_dense.png`).
- Captura do app em uso (1800×1000) enviada pelo usuário em 2026-10-19.
- [Linguagem visual](visual-language.md) (B8, aprovada pelo usuário) e [dossiê de auditoria da GUI](gui-audit-dossier.md).

## Comparação: o que a UI Slint tinha e o shell Qt não tem

| Capacidade (fonte: `app.slint` removido) | Slint (antiga) | Qt (atual) | Situação |
|---|---|---|---|
| Painéis flutuantes com alça de arrastar e redimensionar (`PanelDragHandle`, `PanelResizeGrip`) | sim | **não** (só diálogos, B5) | perdido |
| Trilho agrupado por papel (`ToolGroup`, `RailTool` com rótulo) | sim | ícones soltos, sem rótulo nem grupo | degradado |
| Cartões de ferramenta no contexto (`ToolCard`, `ToolGroup`) | sim | HUD único com seções | parecido |
| Barra de viewport com botões próprios (`ViewportBarButton`) | sim | barra flutuante genérica | degradado |
| Chips de pincel e passos (`BrushChip`, `BrushStepper`) | sim | painel do pincel em campos | parecido |
| Tela inicial com projetos recentes (`HomeTexts`, `RecentProject`) | sim | **não** no shell novo | perdido |
| Ações rápidas em overlay (`quick-actions-open`) | sim | paleta de comandos | trocado |
| Painel de Referências e painel de Paint canvas como diálogo | sim | gaveta/drawer + diálogo | trocado |
| Ações por arrastar com *preview* e moldura (`PanelDragHandle`) | sim | HUD de operação | trocado |

**Leitura honesta:** a UI Slint era **funcionalmente mais rica** (painéis manipuláveis, tela inicial, trilho agrupado). O shell Qt ganhou em coerência e acessibilidade, mas **perdeu densidade e manipulação** — e é isso que a captura de hoje denuncia: uma tela enorme, com o painel direito recolhido, um cartão de contexto solto no meio inferior e nada indicando hierarquia.

## Gap observado na captura do app (1800×1000)

1. **O vazio domina.** Em tela grande o painel direito fica recolhido (só a coluna de ícones) e a viewport cresce sem limite; o trabalho acontece no canto.
2. **O cartão de contexto flutua.** A barra "Duplicate · Align · Join · Separate · More" fica solta no rodapé da viewport, sem âncora visível na seleção.
3. **A barra superior não tem hierarquia.** Menus, *pills* de workspace, undo/redo, busca e engrenagem disputam o mesmo peso visual.
4. **O trilho não diz nada.** Só ícones; não há rótulo no hover nem agrupamento por papel.
5. **Os painéis flutuantes não são manipuláveis** (perdido do Slint) e o layout guardado não distingue "painel aberto" de "painel vazio".
6. **Textos de sistema em inglês no app pt‑BR** (ex.: "No operations. Use '+' to add a boolean…") — a tradução do *snapshot* não cobre as dicas longas.
7. **A viewport é o única coisa "bonita"**: grade fina, eixos discretos, cubo pequeno — o配件 (toolbar, cube, HUD) flutua sem casa.

## Onda D — Densidade, hierarquia e ancoragem (a casca)

| ID | Item | Critério de aceite |
|---|---|---|
| D1 | Barra superior em três zonas com pesos distintos: navegação (menus), contexto (workspaces), ações (undo/redo, busca, ajustes); menus reduzidos de 8 para 5 (Selection→Edit, Add+Object→Object, Tools→Draw) | captura antes/depois + check de hierarquia |
| D2 | Painel direito **aberto por padrão** acima de 1440 px e recolhido abaixo, com botão explícito (nunca "some" sozinho) | check de layout por largura |
| D3 | Cartão de contexto ancorado à seleção/âncora (com linha-guia), não flutuando no rodapé | captura com seleção ativa |
| D4 | Trilho com rótulo no hover, agrupamento por papel (Desenhar · Modelar · Pintar) e marcador da ferramenta ativa | check de acessibilidade do trilho |
| D5 | Espaçamento, altura de campo e tipografia 100% por token (nenhum literal novo) | `grep` de literais + captura |
| D6 | Estados vazios **com ação** (não só frase) em Partes, Propriedades, Operações e Material | check de estado vazio |
| D7 | Textos de sistema (dicas longas, mensagens de estado) traduzidos no snapshot, não hardcoded em inglês | `ui_i18n_catalog` + teste de idioma |

## Onda E — Viewport legível e sem ruído

| ID | Item | Critério de aceite |
|---|---|---|
| E1 | Grade mais sutil por padrão (opacidade e espessura por token) e visível só quando a câmera se move | captura antes/depois |
| E2 | Gizmo: contraste do anel, tamanho e "caixa" de arraste coerentes com a régua de 2 px + halo | captura GL |
| E3 | Seleção sem arestas internas (só a silhueta) e face com tinta leve, como a referência aprovada | captura GL + métricas |
| E4 | Accessories da viewport (toolbar, cube, HUD) num agrupamento só, translúcidos, com sombra e ocultados quando ocioso | captura |
| E5 | HUD da operação com a linguagem do quadro de prévia (B3): mesma tipografia, mesma escala | captura |

## Onda F — Consistência, estados e acessibilidade

| ID | Item | Critério de aceite |
|---|---|---|
| F1 | Um seletor de cor só em todo o app (B1 conferido no app inteiro) | `grep` + captura |
| F2 | Toda torta tem a lista equivalente em menu comum, com o mesmo texto | check de paridade |
| F3 | Foco visível e ordem de foco previsível em todos os painéis e cartões novos | tabulação gravada |
| F4 | Alto contraste e 150%/200% sem exceção em nenhum painel | capturas em 3 escalas |
| F5 | Movimento reduzido respeitado em toda animação nova (nenhuma animação obrigatória) | `reduced_motion` |
| F6 | Leitor de tela: nome, estado e motivo de desabilitado nos controles novos (picker, HUD de modo, grupo Referência) | `QAccessible` |
| F7 | Recusas explicadas em linguagem de usuário, com o motivo e o próximo passo | captura de recusa |

## Verification

- Capturas do shell em `target/shell-captures/` (offscreen, `shell_v2`) antes e depois de cada item; checagens do harness (`shell_v2`) cobrindo o que é verificável sem olho.
- Contraste e escala: capturas a 100%/150%/200% e em tema claro.
- Foco: tabulação gravada em cada painel novo.
- **Não executado no repositório**: tela real com GPU, Windows, leitor de tela.

## Risks

- Mexer na hierarquia da barra superior e no painel direito muda o que a pessoa já aprendeu. Mitigação: um item por passo, com captura antes/depois e o gate do harness verde a cada passo.
- A densidade pode conflitar com a decisão do usuário de manter o rótulo **acima** do campo (B13). As ondas D e F respeitam essa decisão; o que muda é a consistência, não a posição.

## Next

Ordem sugerida: **D1–D7** (a casca, que é o que mais aparece), depois **E1–E5** (a viewport), depois **F1–F7** (consistência e a11y). Cada item fecha com captura e check do harness.
