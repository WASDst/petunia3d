# Dossiê de auditoria da GUI Qt — 2026-10-17

> **Status:** diagnóstico (não é decisão). Evidência por **leitura estática** do checkout `refactor/architecture-foundation` com árvore suja; **nada foi compilado nem executado** (`not run`). Números de linha vêm de três auditorias paralelas e já apresentam pequena deriva (ex.: `petunia_overlay_cards.cpp:726` → `:796`); reconfirmar a cada fatia. Itens marcados ✔ foram reverificados por leitura direta. A proposta que responde a este dossiê está em [Shell v2](shell-v2-proposal.md) e [Operation Graph](../03-geometry/operation-graph.md).

Abreviações: `mw` = `crates/ui-cxxqt/cpp/petunia_main_window.cpp`, `ep` = `petunia_editor_panel.cpp`, `oc` = `petunia_overlay_cards.cpp`, `gl` = `petunia_gl_widget.cpp`.

## 1. Intent e fontes

- **Intent:** avaliar a GUI Qt Widgets contra a filosofia do projeto ([AGENTS.md](../../AGENTS.md), [registro de decisões](../00-philosophy/decision-register.md)), contra o pacote `newdocs/` (design system UI-001 e especificação SEL/VSP/MF/RIG) e contra as referências de produto (Plasticity, Rhino, Shapr3D).
- **Fontes:** `crates/ui-cxxqt/{cpp,src,tests,examples,assets}`; `website/docs/09-ui/*`, `02-application/*`, `03-geometry/*`; `website/progress/tasks.json`; `newdocs/`.
- **Limites:** nenhuma execução de GUI, nenhum screenshot novo; afirmações sobre foco visível e contraste estão marcadas "confirmar visualmente".

## 2. Resultado em uma página

| #  | Achado                                                                                                         | Gravidade |
|----|----------------------------------------------------------------------------------------------------------------|-----------|
| 1  | Operações paramétricas (booleana, modifiers, projeção, UV) vivem como abas/botões one-shot, não como operações re-editáveis | Estrutural |
| 2  | `PetuniaMainWindow` (3030 linhas, 46 métodos, 127 `connect`, 118 `petuniaRequest`) e `PetuniaEditorPanel` (2211 linhas, 7 modos por `if (mode == …)`) concentram a UI | Estrutural |
| 3  | DRAW é tratado como POLY ✔: Rust publica `"DRAW"/"POLY"`, C++ compara `ws == "Model" && mode == "Draw"` (`oc:796`, `oc:1301`) | Bug no caminho principal |
| 4  | Ids de comando usados pela UI que não existem no core ✔ (`view.shade_smooth/flat`, `model.origin_to_*`)       | Bug         |
| 5  | ~2,1k linhas de widgets compilados e nunca instanciados ✔ (`PetuniaPaintPanel` etc.)                           | Dívida      |
| 6  | Tema, i18n e a11y não são uma camada: ✔ 424 literais hex, 105 `setStyleSheet`, densidade inerte, ~618 strings fora do catálogo | Qualidade  |
| 7  | Ponte: snapshot JSON completo por request, polling de 33 ms, duas APIs (JSON + 75 `extern "C"` legados)       | Desempenho/arquitetura |
| 8  | Núcleo ainda não oferece o que a UI "deveria" projetar (grafo/pilha de operações; G12/G13/G14/P01 TODO)       | Dependência |
| 9  | Tracker e docs: itens DONE sem superfície Qt; docs 09-ui ainda em Slint; conflitos newdocs × website/docs     | Governança  |

## 3. Mapa real do shell (as-built)

```text
[logo][menus Arquivo…Ajuda][DRAW|POLY|PAINT|UV|ANIMATE]      [↶][↷][Buscar Ctrl+K][⚙]
┌────┬──────────────────────────────────────────────────┬─────────────────────────┐
│Rail│ ToolOptionsBar (domínio+snap+câmera+shading)     │ Outliner "Partes" ≥300px│
│44px│ PrimCard/LoopCard (20,20)  MicroInsp (20,160)    │─────────────────────────│
│    │ HUD (20,360)   [QOpenGLWidget + overlays pintados]│ Inspector MODEL (8 abas)│
│    │            FloatingContextBar (por workspace)    │  Transf|Material|Modif|  │
│    │                                                  │  Ações|Ferram|Perfil|    │
├────┴──────────────────────────────────────────────────┤  Ops DRAW|Booleans       │
│ Dock inferior oculto: Paint | UV | Timeline (EditorPanel por modo)               │
└──────────────────────────────────────────────────────────────────────────────────┘
```

- 5 docks, 1 painel flutuante, 6 widgets de overlay, ~8 overlays pintados no GL, 3 popups, ~7 diálogos.
- O Inspector é **sempre o MODEL**, mesmo em PAINT, UV e ANIMATE. O Rail de UV e ANIMATE tem apenas `select`; PAINT lista 21 ferramentas (cerca de metade sem ícone); extrude/bevel/inset não aparecem em nenhum rail.
- `drawViewportHud` retorna cedo sempre que há `ToolOptionsBar` irmã (`gl:215`): código efetivamente morto.
- O layout é guardado em três lugares (`QSettings`, `PetuniaDockManager`, Rust).

## 4. Matriz de colocação de features

Legenda de "casa certa": **Rail** · **HUD** (parâmetros vivos no viewport) · **Ação** (context action) · **Propriedade** · **Nó/Pilha** · **Diálogo**. Regra de decisão em [Shell v2 §3](shell-v2-proposal.md).

| Feature                      | Hoje                                                                   | Casa certa                                   | Nó? | Status      |
|------------------------------|------------------------------------------------------------------------|----------------------------------------------|-----|-------------|
| Primitivas                   | menu + popup do rail + pie + `PetuniaAddMenu` morto (4 listas)         | Rail "Criar" + HUD; objeto = nó Primitive    | sim | [ ] TODO    |
| Parâmetros de primitiva      | `PetuniaPrimitiveCard` genérico, sem `sync()`                          | HUD + seção Propriedades                     | sim | [ ] TODO    |
| Loop Cut                     | card + ContextBar + aba + MicroInspector + wheel (4 superfícies)       | Rail + HUD único                             | —   | [ ] TODO    |
| Extrude/Inset/Bevel/PushPull | ContextBar + combo em aba + campo "Valor da operação"                  | Rail/Ação + HUD com `PetuniaNumberField`     | —   | [ ] TODO    |
| Booleana                     | aba "Booleans", botões `fuse/cut/intersect` destrutivos               | Nó Boolean + Bake                            | sim | [ ] TODO    |
| Mirror/Symmetry/Array/Subdiv | aba "Modif" (`QFormLayout`); Array ausente; Subdiv one-shot            | Pilha/Nós                                    | sim | [ ] TODO    |
| Geradores                    | ausentes na GUI                                                        | Nó fonte + Make Editable                     | sim | [ ] TODO    |
| Shading/normais              | botões com ids inexistentes ✔                                          | Ação + seção Geometria                       | —   | [ ] TODO    |
| Pivot/origem                 | 3 implementações, 1 viva; ids `model.origin_to_*` inexistentes ✔       | Seção Transform + Ação                       | —   | [ ] TODO    |
| Materiais/texturas/efeitos   | aba Material + "Camadas" (~9 controles/camada) + 22 botões de superfície | Nós de superfície + Layers em Estrutura    | sim | [ ] TODO    |
| Projeção/stencil/decal       | abas de botões                                                         | HUD + nó Projection                          | sim | [ ] TODO    |
| UV unwrap/pack/relax/stitch  | botões sem parâmetros                                                  | Ação + Propriedades UV + HUD                 | —   | [ ] TODO    |
| Referência/projeção de foto  | 4 lugares                                                              | Dock Reference + Propriedades + nó           | sim | [ ] TODO    |
| Animação/rig                 | abas Criatura/Motion/Pose/Timeline; hit-test do dopesheet no widget    | Estrutura + Gaveta Timeline                  | —   | [ ] TODO    |
| Domínio de seleção           | ToolOptionsBar; teclas 1/2/3 pulam Vértice                             | Viewport Header (único)                      | —   | [ ] TODO    |
| Câmera/projeção              | duas stores; barra secundária sem toggle legado                        | Viewport Header                              | —   | [ ] TODO    |
| Exportar/Importar            | FFI legado + request (2 caminhos), sem Import no menu                  | AppBar > File, diálogos assíncronos          | —   | [ ] TODO    |

## 5. Violações de arquitetura (AGENTS.md: "UI fina")

- **Lógica em widget:** mapeamento kind→parâmetro (`mw:443-460`), clamp de pincel (`mw:275-288`), máquina de câmera (`mw:1838-1905`), hit-test do dopesheet (`ep:1946-1979`), menu de contexto do viewport com 20 ids crus (`gl:522-560`).
- **Estado duplicado:** `m_currentProjectPath`, `m_activeWorkspace` (dois escritores), `m_modeBadge` só atualizado em `setSelectionDomain`, projeção em duas stores, três stores de layout.
- **Código morto:** `PetuniaPaintPanel` ✔, `PetuniaUvPanel`, `PetuniaTimelineWidget`, `PetuniaAssetLibraryWidget`, `PetuniaPreferencesDialog`, `PetuniaReferenceDialog`; `refreshModifiers`; 19 declarações FFI não chamadas; harness de aceitação de 237 linhas dentro de `mw`.
- **Modais bloqueantes em contextos reentrantes:** `dialog.exec()` dentro do slot de sync (`mw:2169-2182`, disparado pelo `QTimer` de 33 ms) e `QMenu::exec` dentro de `mousePressEvent`.
- **Event filter global:** `qApp->installEventFilter(this)` com `findChildren<QAction*>` por tecla (`mw:2398-2561`).
- **Desperdício por sync:** `applyPreferences` 2× por sync limpando o cache de ícones; `petuniaLocalize` varre filhos em cada sync.
- **Drag-and-drop do outliner** habilitado como `InternalMove` sem handler: a reordenação visual é revertida no próximo sync (`mw:978-981`, `mw:1624-1628`).

## 6. Tema, i18n e acessibilidade

- `petunia_theme.h` não alimenta QSS vivo; só `PetuniaVector3Field` usa `axisColor`, uma vez, no construtor. Ícones padrão em `#eeeeee` falham em tema claro; `density` é lida apenas no diálogo morto e na Gallery.
- ~991 literais visíveis em `cpp/*.cpp`, ~618 ausentes do `native-translations.json` (limite superior); rótulos derivados de ids (`ep:914,928,1023`); `tr()` sobre string dinâmica (`mw:2686`).
- **A11y:** 30 `setAccessibleName` e 3 `setAccessibleDescription` em ~18k linhas; ✔ 0 nomes em `oc`; 0 `setTabOrder`; `:focus` definido só para campos de texto/combos; foco do `PetuniaNumberField` provavelmente invisível (confirmar visualmente); F6 exclui o header quando a `ToolOptionsBar` está visível; foco restaurado em 4 diálogos apenas; pie menu e color wheel só com mouse; Space duplicado (MicroInspector × Reproduzir/Pausar); atalhos fixos em menus e tooltips; `#777777` sobre `#202020` ≈ 3,7:1 (< 4,5:1).
- **Layout:** chrome mínimo ≈ 650 px (duas docks ≥ 300 px + rail), deixando ≈ 200 px de viewport a 850 px de largura.

## 7. Ponte Rust ↔ Qt

- Um request JSON único; cada ação não-`snapshot` devolve snapshot completo (`shellviewmodel` ≈ 587 `insert(`) e o C++ pede outro (`syncFromApplication`, 57 chamadas); `paintGL`, `paintEvent` e caminhos de hover também pedem snapshot.
- 132 variantes de `UiIntent` contra 268 braços diretos em `dispatch_controller`; 75 funções `extern "C"` legadas, ~40 ainda em uso; retorno `bool` ignorado (`mw:439`).
- O snapshot é `serde_json::Value` sem tipo; o C++ lê `state["k"]` com default silencioso, o que permite bugs como `"Draw"` × `"DRAW"` ✔.
- ~147 ids de comando do core nunca referenciados pela GUI (acessíveis apenas pela paleta).
- Entrada numérica: `number()` em `ep:411-426` descarta o texto da expressão digitada e envia só o valor; expressões funcionam apenas no caminho XYZ (`ep:546`).

## 8. Testes: o que realmente cobrem

| Suíte                         | Cobre                                                         | Lacuna                                                           |
|-------------------------------|---------------------------------------------------------------|------------------------------------------------------------------|
| `tests/editor_parity.rs` (21) | fluxos de documento via `QtEditor::request`                   | nenhum widget C++ é executado                                    |
| `tests/bridge_test.rs` (11)   | FFI legado (`petunia_qt_app_*`)                               | protege superfície morta; um teste passa vazio sem display       |
| `tests/gallery_test.rs` (1)   | `setValue/setMixed/setValues` e troca de tema                 | não testa Enter/Esc/scrub, a11y, contraste ou visual             |
| aceitação nativa (`mw:2794+`) | troca de abas, pick, F6, capturas                             | fora do `cargo test`; não cobre cards/rail/outliner/i18n; não pegaria o bug DRAW |

As contagens citadas no backlog (17/10/50) não coincidem com o código atual (21/11/24 chamadas `check(`). **Todos esses números são leitura estática; nenhuma suíte foi executada nesta auditoria.**

## 9. Documentação: conflitos e defasagens

| Tema                  | `website/docs`                                                    | `newdocs`                                                  | Resolução proposta                                  |
|-----------------------|-------------------------------------------------------------------|------------------------------------------------------------|-----------------------------------------------------|
| Nó/grafo              | "sem node graph"; três inserções tardias citam "futuro Sistema de Nós" | `ModifierStack` só como padrão nomeado               | ADR-C13 (Operation Graph)                           |
| Docking               | "fixed-but-adaptive"; KDDockWidgets "prioritário"                 | layout persistente; preferir `PetuniaDockManager`; Qt-ADS só em spike | ADR-C14: regiões fixas + layout versionado, sem lib nova |
| Context actions       | 5–6 + More                                                        | 3–4 + More                                                 | 4 + More, validar em protótipo                      |
| Tamanhos              | corpo 13, alvo ≥ 28, rail 40–48/44                                | corpo 14, controle 32/36/40, rail 44–52                    | tokens por densidade                                |
| Domínio de seleção    | ToolOptionsBar                                                    | SVG no viewport header; texto no rail                      | Viewport Header                                     |
| Barra contextual      | três posições distintas                                           | idem                                                       | flutuante, reposicionável, nunca sobre o gizmo      |
| Assistência visual    | 3 perfis                                                          | 4 (Organic)                                                | 3 perfis + preset Organic                           |
| Slint residual        | `AGENTS.md:122,131`, register `:12,:98`, U09, E02, R02            | —                                                          | reescrever 09-ui para Qt; fechar U09                |
| Tool options          | —                                                                 | pede, não define região                                    | HUD de operação + barra contextual (ADR-C14)        |
| IDs                   | ADR-C01…C12, U01…                                                 | UI-001…015 (colide visualmente com U0x)                    | mapear UI-0xx → ADR-C14                             |

## 10. Tracker: itens DONE que pedem checkpoint "superfície Qt"

| ID                     | Situação no tracker | Observação da auditoria                                                                 | Ação proposta      |
|------------------------|---------------------|-----------------------------------------------------------------------------------------|--------------------|
| Q06                    | DONE 7/7            | evidência com revisão "working tree"; janela nativa, GPU, tablet e Windows `not run`     | anotar/reabrir     |
| Q07                    | DONE                | evidência com revisão "working tree"                                                    | anotar             |
| H21, H23, H29–H33      | DONE                | 0 referências a Companion/Clay/Section/Volume/Smart Rig em `crates/ui-cxxqt`            | checkpoint Qt      |
| H19                    | DONE 6/6            | docs ainda dizem IN PROGRESS/TODO                                                       | reconciliar        |
| G05, G10–G14           | TODO                | contradiz H* DONE                                                                       | reconciliar        |
| AD-06 (KDDockWidgets)  | DONE (matriz Qt)    | aparece só em comentário (`petunia_dock_manager.h:12`)                                  | reabrir            |
| AD-07 (QuickQanava)    | DONE (matriz Qt)    | `NodeGraphCanvas.qml` com nós fixos, sem dependência, sem dispatch, só no preview       | reabrir            |

## 11. Top 15 ofensores (ordem de prioridade)

1. DRAW tratado como POLY (`oc:796,1301` × `lib.rs`): a barra DRAW nunca aparece.
2. Inspector MODEL com 8 abas estáticas misturando transform, material, modifiers, ferramentas, perfil, DRAW e booleanas.
3. Booleana, subdivisão, UV, superfície/stencil e efeitos como botões one-shot.
4. `PetuniaEditorPanel` god class dirigida por strings.
5. ~2,1k linhas de widgets mortos e membros sem uso.
6. Tokens de tema ignorados (424 hex, 105 `setStyleSheet`, QSS posicional `%1…%15`).
7. Ponte: snapshot completo duplicado + polling.
8. Dois caminhos de comando (JSON e FFI legado) com falhas silenciosas.
9. Fios soltos: ids inexistentes, expressão descartada, cards sem `sync()`, badge dessincronizado, cor de material caindo na linha 0 (`mw:1285-1287`).
10. Teclado e foco: sem `:focus` visível, 0 nomes nos overlays, pie/color wheel só com mouse, F6 sem header.
11. Rail errado: UV/ANIMATE com um item, PAINT com 21 (metade sem ícone), ferramentas de modelagem ausentes.
12. ~600 strings fora do catálogo e rótulos derivados de ids.
13. Diálogos modais abertos a partir do sync.
14. Estado de layout/modo/câmera duplicado em 3 stores.
15. Testes que dão falso conforto (vazio sem display, gallery sem comportamento, aceitação fora do CI).

## 12. Riscos e próximos passos

- Este dossiê é leitura estática: antes de qualquer correção, obter **baseline visual offscreen** (DRAW/POLY/PAINT/UV/ANIMATE × claro/escuro/alto contraste × 100–200 %) e executar as suítes sob orçamento (`CARGO_BUILD_JOBS=2`, `QT_QPA_PLATFORM=offscreen`, `-- --test-threads=1`).
- Próximo checkpoint: Fase F0 do [roteiro Shell v2](shell-v2-proposal.md) (tarefa U10 no tracker).
