# Shell v2 — proposta de refatoração da GUI Qt

> **Status:** proposta. A direção foi **aprovada pelo usuário em 2026-10-17** em três pontos (sistema de nós simples e modular; regiões fixas por padrão considerando `newdocs`; novo shell em paralelo reaproveitando o atual). Os detalhes abaixo ainda precisam virar decisão formal (ADR-C14 e [ADR-C13](../00-philosophy/decision-register.md)) e **não estão implementados**. Evidências em [gui-audit-dossier.md](gui-audit-dossier.md). O sistema de nós está em [Operation Graph](../03-geometry/operation-graph.md).

## 1. Princípios

- **Referências:** Plasticity (canvas dominante, chrome mínimo, HUD de operação), Shapr3D (ações contextuais), Rhino (precisão numérica e propriedades técnicas). São princípios, não clonagem de UI (`newdocs/design-system` §3).
- **Mantém:** Qt 6 Widgets + CXX-Qt + OpenGL; `UiIntent` → Application → `CommandDispatcher`; UI fina; clean-room. Sem QML no shell de produção.
- **Uma fonte de verdade por responsabilidade:** tokens (`petunia_config`), ações (`ActionRegistry`), ferramentas (`ToolCatalog` no Rust), seções de propriedade (`PropertySectionRegistry`), strings (ids estáveis).
- **Contrato de todo componente** (`newdocs` UI-001 §6.1): id, role, text/tooltip id, estados (hover, focus-visible, pressed, selected, mixed, disabled, invalid, busy, read-only), `accessible_name/description`, sincronização com sinais bloqueados, hook de teste; só emite eventos semânticos, nunca muta o documento.
- **Estratégia (decisão do usuário):** shell novo construído em paralelo atrás de `PETUNIA_SHELL=v2`, com troca única na paridade. Dentro do v2 não há big-bang: cada região entra com Gallery e testes.

## 2. Regiões (gramática fixa + layout do usuário versionado)

```text
┌ AppBar: menus · projeto●dirty · [DRAW|POLY|PAINT|UV|ANIMATE] · ↶↷ · Buscar · ⚙ ───────────┐
├──┬─ Viewport Header: Projeção | Display | Overlays | Navegação | Domínio de seleção ─┬────┤
│R │ ┌ Work Surface (viewport 3D | UV 2D | Paint 2D | split) ───────────────────────┐  │Estr│
│a │ │  Operation HUD (ancorado ao handle ativo) · Ações contextuais (≤4 + Mais)    │  │utu │
│i │ │  Gizmo e overlays com prioridade e anti-colisão                              │  │ra  │
│l │ └──────────────────────────────────────────────────────────────────────────────┘  │────│
│  │                                                                                   │Prop│
├──┴─ Gaveta (recolhível): Assets · Timeline · UV Islands · Nós ───────────────────────┴────┤
│ Status: ferramenta · seleção · precisão/snap · tarefa em segundo plano                    │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

- Regiões fixas; o usuário redimensiona, recolhe e fixa; destacar painel é exceção.
- **Layout persistido** com `workspace_id`, `layout_schema_version`, ids estáveis de dock, tamanhos relativos + mínimos em px lógicos, `user_modified`, backup do layout anterior e recuperação de ids ausentes, schema obsoleto ou monitor indisponível. **`applyWorkspacePreset()` nunca sobrescreve layout do usuário.** Mudar tema ou layout nunca suja o documento 3D.
- **Sem nova biblioteca de dock.** KDDockWidgets/Qt-ADS ficam fora; a persistência de `PetuniaDockManager` é reduzida ao necessário.
- **Estrutura / Propriedades por workspace** (matriz canônica em [workspaces-feedback-accessibility](workspaces-feedback-accessibility.md)):

| Workspace  | Estrutura                  | Propriedades                                                       | Gaveta                     |
|------------|----------------------------|--------------------------------------------------------------------|----------------------------|
| DRAW/POLY  | Cena / Partes              | Shape · Generator · Geometry · Transform · **Operações** · Material | Assets · **Nós**           |
| PAINT      | Layers                     | Target · Brush · Layer · Projection · Effects                      | Assets · Brushes · **Nós** |
| UV         | UV Maps / Islands          | Selection · Unwrap · Projection · Pack · Health                    | recolhida                  |
| ANIMATE    | Creature / Motions / Rig   | Motion · Rig · Advanced                                            | Timeline                   |

- **Rail** vem de `ToolCatalog` (Rust), por workspace, nunca de listas em C++; rótulo completo + atalho no tooltip; modo expandido com texto (fim de `P…h`).
- **Viewport Header** contém só controles de vista (projeção, display, overlays, navegação) e o **domínio de seleção**, que não é duplicado em outro lugar.
- **Ações contextuais:** barra flutuante perto da seleção, reposicionável e fixável, até 4 ações + "Mais", filtrada por capacidade, nunca sobre o gizmo nem bloqueando picking; muda após troca de seleção, nunca durante o arrasto.

## 3. Regra de decisão: "onde isto mora"

1. Tem parâmetros que o usuário vai querer editar depois de confirmar? → **Nó/Pilha** (seção Operações).
2. É um gesto vivo, com valores durante a operação? → **HUD** (`PetuniaNumberField` compartilhado; confirmar cria ou edita o nó; Esc restaura; um gesto = um Undo).
3. É um comando pontual sobre a seleção? → **Ação contextual** (via resolvedor de capacidades; nunca filtra em silêncio; mostra o motivo se desabilitada).
4. É estado do objeto selecionado? → **Propriedade** (seção do registry).
5. É modo persistente de entrada de ponteiro? → **Rail**.
6. É decisão bloqueante (salvar, recuperar, destrutivo irreversível)? → **Diálogo**. Nunca aberto a partir de sync/timer e nunca por chamada programática (CLI, MCP, scripts).

## 4. Camada C++

| Módulo             | Responsabilidade                                                                                                     |
|--------------------|----------------------------------------------------------------------------------------------------------------------|
| `ShellWindow`      | apenas layout e regiões; sem lógica de domínio                                                                       |
| `RegionHost`       | contêiner de região com estado recolhido/fixado e persistência                                                       |
| `RegionNavigator`  | F6/Shift+F6 na ordem canônica (Header → Rail → Work Surface → Estrutura → Propriedades → Gaveta → Barra contextual), Tab dentro da região, restauração de foco |
| `ViewModelStore`   | DTO tipado por seção com `revision`; widgets assinam a seção e fazem diff                                            |
| `ActionRegistry`   | única fonte para menus, paleta, rail, barra contextual, atalhos e tooltips derivados do keymap                       |
| `ThemeService`     | snapshot imutável → QSS/`QPalette` gerados uma vez por troca; cache de ícones por (tema, DPI, tamanho, estado)       |
| `OverlayManager`   | prioridade gizmo > seleção > valor principal > snap > avisos > secundárias; âncoras 3D com histerese; interativo × click-through |
| controllers finos  | um por região, emitindo `UiIntent`; nenhuma regra de geometria                                                       |

## 5. Camada Rust (editor e ponte)

- `ShellViewModel` **tipado e seccionado por revisão**; o request devolve só as seções alteradas; eventos de mudança no lugar do polling de 33 ms.
- Via única de `intent/command` com `Result` visível na UI; migrar as ~40 chamadas legadas e remover os 75 `extern "C"` ao final.
- `ToolCatalog`, `PropertySectionRegistry`, capacidades contextuais e motivos de desabilitação nascem em `petunia_editor` (testáveis headless — Camada 2).
- Modos e ids como enums compartilhados ou constantes geradas (impede `"Draw"` × `"DRAW"`); teste de paridade "todo id usado em C++ existe no registry".

## 6. Reaproveitamento

| Manter e portar                                                                                                                                                   | Descartar ou reescrever                                                                                                                         |
|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `PetuniaGlWidget` (viewport e gizmos pintados), `PetuniaNumberField`, `PetuniaVector3Field`, `PropertyGroup`, `PetuniaTheme` (tornar real), `PetuniaIcons` (com cache), `PetuniaCommandPalette`, `ColorWheel` e `AngleDial` (+ teclado), `PetuniaEditorCanvas` (Paint 2D), `UvCanvas`, `DopesheetCanvas` (sem hit-test no widget), Component Gallery | `PetuniaMainWindow`, `PetuniaEditorPanel` (dividir por feature), `ToolOptionsBar`, `FloatingContextBar` monolítica, `PrimitiveCard`, `LoopCutCard`, `MicroInspector`, `OperationHud` atuais, seis widgets mortos, `FloatingPanel` em lambda, harness de aceitação em código de produção |

## 7. Roteiro

| Fase | Entrega                                                                                                                                         | IDs do tracker | Status       |
|------|-------------------------------------------------------------------------------------------------------------------------------------------------|----------------|--------------|
| F0   | Verdade e limpeza: baseline visual offscreen, bugs DRAW/ids/expressão/modais, remoção de código morto, teste "ids ↔ registry", ADRs, reabrir DONE inflados | U10            | [x] DONE     |
| F1   | Fundações: `ThemeService`, catálogo por ids (+ pseudo-locale), `ActionRegistry`, contrato de componente, Gallery como gate, foco visível e nomes acessíveis | U11, U12       | [x] DONE     |
| F2   | Ponte tipada: `ShellViewModel` seccionado, eventos, via única, `ToolCatalog`, `PropertySectionRegistry`, capacidades                           | U13, A08       | [x] DONE     |
| F3   | Shell v2 esqueleto atrás de flag: AppBar, Rail, Viewport Header, regiões, F6, layout versionado, Outliner por UUID, fatia **Transform**          | U14            | [x] DONE     |
| F4   | HUD e ações contextuais: `OperationHud` único, resolvedor de capacidades, anti-colisão, rail por catálogo                                       | U15            | [x] DONE     |
| F5   | Núcleo de operações (só Rust, paralelo desde F1): ver [Operation Graph](../03-geometry/operation-graph.md)                                      | O01, O02       | [x] DONE     |
| F6   | UI de nós: Pilha no Inspector → Gaveta "Nós" → materiais/texturas → descriptors                                                                 | O03, O04       | [x] DONE     |
| F7   | Workspaces PAINT, UV e ANIMATE no v2                                                                                                            | U16–U18        | [x] DONE     |
| F8   | Swap: v2 vira padrão, remoção do shell legado, do FFI legado e dos testes legados; fechar U01–U09 antigos                                       | U19            | [ ] TODO     |
| F9   | Gates finais: QTestLib offscreen, golden frames FBO, a11y (AT-SPI), matriz manual Linux/Windows/GPU baixa                                       | Q08            | [ ] TODO     |

**Regra de fatia:** inventário → contrato → componente na Gallery → teste Camada 2 → widget → teste QTestLib offscreen → screenshot → remover o legado correspondente → atualizar `tasks.json` no mesmo commit → handoff (Intent / Sources / Gap / Changed / Verification / Risks / Next checkpoint).

## 8. Verificação e gates

- Rust: `CARGO_BUILD_JOBS=2 cargo fmt`/`clippy -D warnings`; `cargo test -p petunia_core --lib`; `-p petunia_editor --features animation-workspace --lib`; `-p petunia_project`; `-p petunia_mesh`. Todo bug ganha teste de regressão antes da correção (inclui `"Draw"` × `"DRAW"`).
- Qt: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-<id> QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt --test editor_parity -- --test-threads=1` e `--test bridge_test … --skip test_widgets_studio_window_creation`; novos testes QTestLib por componente (commit/Esc/scrub, foco, nomes, troca de tema sem perder edição).
- Visual: render offscreen/golden PNG por workspace, tema e DPI. Compilação verde não é prova de UI funcional.
- Higiene: `df -h .` e `free -h` antes e depois; remover `target/agent-<id>` ao entregar.
- Manual, com ambiente registrado: GPU física, Windows, leitor de tela, tablet, baixa performance. Sem execução, o status é `not run`.
- Teste de uso (10 min, iniciante): selecionar, escalar por cota, achar modifiers, trocar pincel, mover ilha UV, adicionar booleana. Só se pode afirmar "ajuda iniciantes" depois de executado.

## 9. Riscos e pendências

| Risco / pergunta                                                                                         | Mitigação                                                                |
|----------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| Gaveta "Nós" sem núcleo vira demo de novo                                                                | UI só após testes headless do núcleo (F5)                                |
| Período sem UI utilizável no v2                                                                          | shell legado permanece padrão até F8; Gallery e testes desde F1          |
| Branch: AGENTS autoriza Qt em `codex/qt-integration`; checkout atual é `refactor/architecture-foundation` | confirmar branch antes de qualquer commit; nunca `main`                  |
| Árvore suja com trabalho em andamento (Gallery, NumberField, plugins)                                    | organizar commit/stash como primeiro passo da F0                         |
| Números de UX (UV ≥ 55–65 %, dock 300–360 px, 3–4 ações) são hipóteses                                  | validar em protótipo e registrar no decision-register                    |
| Orçamento de RAM/disco com C++/Qt                                                                        | `CARGO_BUILD_JOBS=2`, `target/agent-<id>` efêmero                        |
