# Slint Rescue — Plano de Refatoração Incremental

> **Status: em execução desde 2026-10-08.**
>
> Este plano implementa as decisões aprovadas em [Slint Reassessment & GUI Architecture](./slint-reassessment.md) e [Workspaces, Feedback Visual e Acessibilidade](./workspaces-feedback-accessibility.md).

## 1. Objetivo

Reduzir o monólito atual do frontend Slint sem reescrever comportamento funcional nem criar uma segunda arquitetura paralela.

O Rescue segue quatro regras:

1. **extração incremental, não big-bang**;
2. **uma única fonte de estado** — componentes extraídos recebem projeções e emitem intents/callbacks;
3. **nenhum acesso novo a Mesh/Project internals na camada declarativa**;
4. **cada fase precisa preservar comportamento observável antes da próxima**.

## 2. Estado inicial observado

No início desta fase:
- `app.slint` tinha aproximadamente 10.774 linhas;
- Header, Tool Rail, viewport, workspaces, Inspector, overlays e toast estavam compostos no mesmo arquivo;
- componentes base já existiam em `components/`;
- `animate.slint` já demonstrava uma separação melhor entre UI declarativa e intents;
- o bridge Rust ainda permanece grande e será tratado em fase posterior.

## 3. Estrutura alvo

```text
ui/
├ app.slint
├ shell/
│  ├ header.slint
│  ├ tool_rail.slint
│  ├ right_column.slint
│  ├ workspace_drawer.slint
│  └ status_bar.slint
├ viewport/
│  ├ viewport.slint
│  ├ view_bar.slint
│  ├ context_bar.slint
│  └ overlays.slint
├ workspaces/
│  ├ model/
│  ├ paint/
│  ├ uv/
│  └ animate/
├ panels/
├ components/
├ dialogs/
├ tokens.slint
└ types.slint
```

O objetivo não é atingir essa árvore em uma única mudança; ela é direção de decomposição.

## 4. Fase 0 — contratos e baseline

**Status: concluída para início do Rescue.**

Fechado:
- Slint frontend principal;
- OpenGL 3.3 Core como renderer final;
- egui congelado como fallback externo;
- macro-layout;
- contratos de workspace;
- feedback visual;
- acessibilidade;
- PAINT/UV/ANIMATE UX.

Ainda obrigatório antes de remover egui:
- gates de performance;
- viewport OpenGL no Slint;
- auditoria de acessibilidade;
- fluxo autoral principal.

## 5. Fase 1 — Shell estrutural

**Status: em execução.**

### Concluído

#### Header
Extraído para:

`ui/shell/header.slint`

Responsabilidades:
- menus globais;
- seletor de workspace;
- Undo/Redo;
- Command Search;
- Settings.

O componente:
- não conhece documento;
- não conhece mesh;
- recebe estado projetado;
- emite callbacks semânticos;
- preserva i18n/tokens existentes.

#### Tool Rail
Extraído para:

`ui/shell/tool_rail.slint`

Responsabilidades:
- ferramentas contextuais de MODEL/DRAW/POLY/PAINT/UV/ANIMATE;
- ToolGroups/flyouts;
- entrada para Asset Library.

O estado continua em `PetuniaSlintShell`.
O Tool Rail não ganha estado autoral próprio.

#### Status Toast
Extraído para:

`ui/components/feedback.slint::StatusToast`

Toast passa a ser componente transversal.
Erros críticos continuam proibidos de depender apenas de toast.

### Resultado intermediário

`app.slint` caiu progressivamente de aproximadamente 10.774 para aproximadamente 9.330 linhas sem remover feature de domínio. A pequena variação após a extração veio da nova boundary semântica de modificadores; o roteador pesado continua fora do monólito.

### Extrações adicionais concluídas

- Right Column shell com `@children`: chrome, rail, hover/peek, header e scroll fora do monólito; conteúdo de workspace ainda permanece no chamador;
- Workspace Drawer genérico;
- Asset Library/Prefabs extraída como conteúdo do drawer;
- Status Bar persistente;
- Context Bar da viewport extraída como host com `@children`;
- View Bar extraída como host com `@children`;
- overlays passivos de seleção, Animate e previews de ferramenta extraídos para `viewport/overlays.slint`;
- guias/previews/stencil em `viewport/guides.slint`, cursor/cotas/snap em `cursor_hud.slint`, resumos em `status_hud.slint` e conteúdo de operação em `operation_hud.slint`; ver execução/gates da wave em §19;
- `PetuniaViewportHost` extraído com superfície, resize lógico/físico, HiDPI e composição da imagem GPU;
- `ViewportInputRouter` extraído: o antigo `TouchArea` de ~400 linhas saiu do monólito, mantendo apenas recognition state efêmero e callbacks semânticos;
- `drag-threshold-px` agora controla a promoção click → drag/box/transform/lasso;
- Orbit/Pan/Zoom convergem para `UiIntent::ViewportGesture`; Orbit preserva pivot por seleção/Cursor;
- fases de tool pointer usam `ViewportPointerPhase::{Press,Move,Release,Cancel}` internamente no Rust;
- pointer semantics do keymap são projetadas para o router via masks, incluindo Maya `Alt+LMB → Orbit`;
- bug de escopo de `pointer.adjust` no scroll corrigido, exceção física de Alt removida da gramática de ferramentas e cancelamento do pointer desacoplado do menu contextual;
- `callbacks.rs` caiu de ~6.069 para ~5.487 linhas com navigation/selection/hover/transform/tool pointer/Split View delegados a `bridge/viewport.rs`.

### Próximas extrações desta fase

1. validar compilação do shell extraído;
2. validar overlays/HUD extraídos na wave U01 (§19), incluindo equivalência visual/foco/teclado;
3. definir `ViewportInputIntent`/boundary antes de mover pointer handling;
4. somente depois, decompor picking/gizmos/tool sessions.

A viewport é deliberadamente posterior porque concentra input, picking, overlays e tool sessions.

## 6. Fase 2 — Right Column

**Status: em execução.**

Separar:
- Structure;
- Properties;
- comportamento responsive/collapse;
- split vertical interno.

O shell não deve saber o conteúdo detalhado de cada Property Section.

Meta e componentes introduzidos:

```text
RightColumn
├ StructureHost
└ PropertiesHost
```

- `StructureHost` e `PropertiesHost` exportados em `ui/shell/right_column.slint`.
- Workspaces MODEL e ANIMATE reestruturados sob `StructureHost` e `PropertiesHost`.
- `AnimateInspector` extraído para `ui/animate.slint`.
- `ModelInspector` extraído para `ui/workspaces/model/inspector.slint`, mantendo Parts em Structure e Transform/Material/Object/Combine em Properties. A extração preserva os corpos existentes; não implementa o split independente nem muda ownership de seleção, materiais, primitivas ou Undo.
- `PaintInspector` extraído para `ui/workspaces/paint/inspector.slint`, preservando controles, 70 propriedades (incluindo seis bindings de ida/volta) e 50 callbacks. O inspector ainda contém as seções existentes; Layers/Properties independentes e Work Surface 2D pertencem às próximas fases. Gates em §19.
- `UvInspector` extraído para `ui/workspaces/uv/inspector.slint`, composto pelo shell com as mesmas propriedades e callbacks. Os hosts são transitórios: operações UV ainda ocupam `StructureHost`; sua migração para Properties/Context Bar e a lista de Islands pertencem à fase de workspace.
- Split vertical independente escrito na wave U02 (§20): cabeçalhos, scrolls, resize/collapse e memória de sessão por workspace. Compilação, regressões e aceite nativo ainda não executados; não considerar Fase 2 verificada.
- Conteúdo recebe propriedades projetadas pelo shell; registry não foi introduzido.

## 7. Fase 3 — Workspace Drawer

Generalizar a antiga Asset Library:

```text
MODEL   → Assets / Prefabs
PAINT   → Assets / Brush Presets / Palette
UV      → auxiliar / diagnostics quando necessário
ANIMATE → Timeline / Motion Stack
```

O container pertence ao shell.
O conteúdo pertence ao workspace.

**Implementação em andamento (2026-10-09):** conteúdo com abas por workspace escrito na wave U03 (§20), preservando rotas Assets/Prefabs. PAINT migra presets/paleta; UV projeta diagnóstico; ANIMATE reutiliza transporte procedural/Motion Stack. Não equivale a timeline completa de keyframes nem a aceite visual.

## 8. Fase 4 — Viewport

Somente após o shell externo estar estável.

Extrair:
- viewport container;
- resize/HiDPI boundary;
- selection overlays;
- tool overlays;
- View Bar;
- Context Bar;
- split view host.

Não mover regra de geometria ou tool session para Slint.

## 9. Fase 5 — Workspaces

### MODEL
Separar DRAW/POLY presentation sem duplicar transform/navigation.

### PAINT
- Layers como Structure;
- Canvas como Work Surface;
- Properties contextuais;
- 3D/2D/Split/PiP.

### UV
- UV Editor promovido a Work Surface;
- split 3D/UV;
- Islands/UV Health.

### ANIMATE
- preservar `AnimateIntent`;
- Creature/Motion Stack em Structure;
- Timeline drawer;
- procedural-first.

## 10. Fase 6 — Rust bridge

Depois que as fronteiras visuais estiverem reais, decompor o Rust:

```text
src/
├ lib.rs
├ bridge/
│  ├ application.rs
│  ├ commands.rs
│  ├ viewport.rs
│  ├ outliner.rs
│  ├ properties.rs
│  ├ assets.rs
│  └ accessibility.rs
├ view_model/
└ viewport/
```

Regra:
não substituir um `lib.rs` enorme por um `bridge.rs` enorme.

## 11. Fase 7 — UI boundary

Convergir para:

```text
Slint
↓ UiIntent / DTO
Application
↓ Commands / Queries
Document / Geometry / Paint / UV / Animation
```

Proibir novos caminhos:
```text
Slint callback → mesh.faces[...]
```

## 12. Fase 8 — OpenGL viewport

Integrar o renderer final aprovado:

```text
petunia-render
→ FBO
→ GL texture
→ Slint/FemtoVG composition
```

Gates:
- sem `glReadPixels` por frame;
- resize;
- HiDPI;
- picking coordinates;
- state restoration;
- render-on-demand;
- hardware mínimo.

## 13. Fase 9 — acessibilidade

Antes do Go/No-Go:
- F6/Shift+F6 entre regiões;
- Tab dentro da região;
- focus restore;
- dialog focus trap;
- roles/names/states;
- 100–200% scale;
- High Contrast;
- Reduced Motion;
- click-move-click;
- hit targets;
- screen-reader audit.

## 14. Fase 10 — Go/No-Go

Slint só é abandonado por bloqueador estrutural reproduzível depois da modularização.

Não são motivos suficientes:
- preferência estética;
- polish incompleto;
- código legado ainda não migrado.

São bloqueadores possíveis:
- integração GL final inviável nas plataformas alvo;
- input/focus/a11y incapaz de cumprir contrato;
- composição impossível no hardware mínimo;
- bugs do toolkit sem workaround aceitável;
- boundary significativamente mais complexa mesmo após decomposição.

## 15. Regra de commits

Preferir commits pequenos por fronteira:
- extrair componente;
- compor no shell;
- limpar imports;
- atualizar documentação;
- validar.

Não misturar extração estrutural com redesign visual significativo no mesmo commit.

## 16. Estado atual

Os arquivos locais já possuem:
- ShellHeader, ToolRail, StatusToast e regiões independentes da viewport;
- `ViewportInputRouter` com thresholds configuráveis e masks oriundas do keymap;
- `bridge/viewport.rs` para navigation, selection, hover, transform, tool pointer e Split View;
- `bridge/model.rs` para DRAW/Profile e `bridge/paint.rs` para input de pintura;
- testes declarativos de regressão do keymap no `viewport_gestures.rs`.

No ponto de inspeção desta rodada, `StructureHost` e `PropertiesHost` foram introduzidos em `right_column.slint`, `AnimateInspector` foi extraído para `animate.slint`, e as seções de MODEL e ANIMATE foram delimitadas semanticamente sob esses hosts. As propriedades `precision-mode` e `snap-mode` foram corrigidas para `in-out property` em `app.slint` para conformidade com a suite de testes.

### Checkpoint UV — retomada de 2026-10-08

**Intenção:** fechar a extração do inspetor UV iniciada na sessão OpenCode `ses_ee2102b0bffeRU05Hib0pMUoUH`, preservando comportamento antes de outra extração.

**Fontes:** este plano (§6 e §15), [UV Workspace](../07-uv/workspace-ux.md) (§2, §7, §8 e §21), `ui/app.slint`, `ui/workspaces/uv/inspector.slint` e `src/callbacks.rs` do crate `petunia_ui_slint`.

| Requisito | Gap observado | Estado |
|---|---|---|
| Conteúdo UV fora do shell | `UvInspector` recebe 16 propriedades / 12 callbacks; `app.slint` passou de 9.354 para 8.933 linhas | Verified nos testes headless abaixo; visual pendente |
| Equivalência da extração | Referências a propriedades, callbacks, traduções e nomes acessíveis comparadas ao bloco original recuperado da sessão; alinhamento do botão Unwrap e espaçamento de 10px restaurados | Inspeção estática realizada |
| Right Column completa | Hosts delimitam conteúdo, sem split independente; operações UV ainda não estão na região final | In migration |
| UV como Work Surface | Canvas permanece 256×256 no Inspector | Pending, Fase 5 |
| Isolamento de workspaces | A extração anterior aninhou PAINT na condição MODEL; limites MODEL/Properties, bloco PAINT e blocos posteriores restaurados a partir da baseline | Corrigido; teste de clique PAINT: pass |
| Versionamento | Metadados Git recuperados do remoto autorizado; arquivos ocultos ausentes restaurados; baseline `ecbbcfeee59953ea3c69e3218b9b281abdc38271` na branch `refactor/architecture-foundation` | Branch e HEAD verificados |

**Changed nesta retomada:** restauração de alinhamento/espaçamento em `ui/workspaces/uv/inspector.slint`, espaçamento ANIMATE e limites de MODEL/PAINT em `ui/app.slint`; regressões em `tests/uv_shell.rs` exercitam clique/arrasto no canvas, origem V inferior, término do arrasto, botões Move/Scale/Rotate após troca de workspace e resize, e acesso independente às operações PAINT. O seletor do flyout em `tests/viewport_gestures.rs` agora aponta para `RightColumnShell::inspector-flyout`, nome qualificado emitido pelo compilador após a extração; as asserções de interação permanecem.

**Verification nesta retomada (Linux, Rust 1.98.1 / Slint 1.18.0, backend de testes headless):**

| Comando realmente executado | Resultado |
|---|---|
| `cargo fmt -p petunia_ui_slint -- --check` | pass |
| `target/debug/xtask ui-lint` | pass, 23 arquivos |
| `git diff --check` | pass |
| `CARGO_BUILD_JOBS=2 cargo test -p petunia_ui_slint --config profile.test.package.petunia_ui_slint.debug=0 --lib --test uv_shell --test viewport_gestures --test animate_shell` | pass: 508 biblioteca + 3 UV/PAINT + 15 gestos + 11 ANIMATE = 537 testes, zero falhas/ignorados |
| `CARGO_BUILD_JOBS=2 cargo clippy -p petunia_ui_slint --profile test --config profile.test.package.petunia_ui_slint.debug=0 --all-targets -- -D warnings` | pass, 3m27s, zero warnings |

O override de `debug=0` reduz símbolos de depuração Rust apenas no crate UI; não remove metadados de busca Slint nem asserções. A primeira tentativa de testes com o profile padrão foi interrompida para limitar artefatos; nenhum pass é atribuído a ela. Caches incrementais antigos fora do build ativo foram limpos, liberando aproximadamente 12 GB. Manifesto e presença dos links canônicos conferidos. A suíte com `--features animation-workspace`, aceitação visual, GL, Windows, screen reader e low-end permanecem **not run** neste checkpoint.

**Risks:** testes headless não comprovam estética, integração GL, Windows, screen reader ou hardware modesto. Não houve alteração de algoritmo, formato de projeto ou caminho de Undo; os testes de domínio existentes continuam necessários. Git está restaurado, mas commit/push e SHA de entrega são informados somente após execução.

**Next checkpoint:** extrair o conteúdo MODEL em um incremento separado, preservando callbacks e as regressões de gestos/seleção. A promoção UV para Work Surface continua na Fase 5; o gate completo Slint Rescue permanece aberto.

**Bugs estruturais detectados pelo gate:** o compilador Slint 1.18 não permite `@children` dentro de elementos condicionais e não permite acesso a `parent` em bindings do componente raiz. O Rescue mantém `RightColumnShell` e `WorkspaceDrawer` estruturalmente presentes com visibilidade controlada; posicionamento relativo ao parent de `ContextBar`, `ViewBar` e `PetuniaViewportHost` pertence ao `app.slint`. `ViewportInputRouter::clear-gesture` foi explicitado como função pública para o shell. O gate confirmou `cargo fmt --check` e `ui-lint` passando nas revisões recentes; Clippy/compilação e testes só serão marcados como aprovados quando a execução correspondente terminar sem erro.

Após os checkpoints da coluna direita: **dividir o registro da viewport em módulos coesos sem transformar `bridge/viewport.rs` em novo monólito**, preservar Rust para picking, ToolSession e gizmo geometry, e evoluir para DTOs/intents explícitos de select/hover/box/lasso. A gramática semântica específica de PAINT requer outra rodada para clone/decal/straight stroke.

### Checkpoint MODEL — 2026-10-09

**Intent:** U01, extrair o conteúdo MODEL em um slice estrutural sobre `40ad949fb3b7a7d392d402bf4da55f5c28769621`, branch `refactor/architecture-foundation`. Preservar contratos públicos do shell, gestos, validação numérica e comportamento de outros workspaces.

**Sources:** este plano §6/§15; [Workspaces, Feedback e Acessibilidade](./workspaces-feedback-accessibility.md); `ui/app.slint`, `ui/inspector/sections.slint`, `ui/shell/right_column.slint` e testes reais do crate Slint.

| Requisito | Gap antes | Changed / limite |
|---|---|---|
| Conteúdo MODEL fora do shell | Bloco de 303 linhas em `app.slint`, com Parts/Transform/Material/Object/modifiers/Combine | Movido para `workspaces/model/inspector.slint`; mesmos componentes, traduções, tokens e condições |
| Projeções e intents | Bindings/callbacks eram diretos no shell | `ModelInspector` recebe 119 propriedades, incluindo seis bindings de ida/volta, e encaminha 49 callbacks; `selection-domain` passa a ser explicitamente projetado |
| Uma fonte de estado | Shell/Rust mantêm estado e execução | Não introduz seleção/material/primitive/modifier próprios no componente; contrato público do Window preservado |
| Equivalência estrutural | Risco de perder callbacks/retornos ou aninhar outro workspace | Corpo transferido comparado à baseline, exceto whitespace e `root.selection-domain`; encaminhamento completo conferido; bloco PAINT e cauda do shell preservados |
| Interações efetivas | Extração estrutural não era coberta por teste MODEL específico | `tests/model_shell.rs` exercita input físico, query two-way, ações Parts, valor/retorno de validação Transform, selection domain, Material e Combine |
| CI do slice | Integração não executava `uv_shell` e ainda não tinha `model_shell`; push limitado a main | Q03: ambos incluídos na integração com animation-workspace; branch autorizada incluída no trigger. Execução remota continua distinta de alteração do YAML |

**Changed:** `app.slint` passa de 8.933 para 8.802 linhas; `ModelInspector` tem 481 linhas incluindo a interface explícita. Imports de corpos exclusivamente MODEL saem do shell. Nenhum algoritmo, schema, renderer ou serviço de domínio foi alterado.

**Verification:** Linux, Rust 1.98.1, Slint 1.18.0, backend de testes headless; fontes do slice sobre a baseline acima. Gates concluídos:

| Comando realmente executado | Resultado |
|---|---|
| `cargo fmt -p petunia_ui_slint -- --check` | pass |
| `target/debug/xtask ui-lint` | pass, 24 arquivos Slint |
| `CARGO_BUILD_JOBS=1 cargo test -p petunia_ui_slint --config profile.test.package.petunia_ui_slint.debug=0 --test model_shell --test uv_shell --test viewport_gestures --test animate_shell --test ui_metrics` | pass: 4 MODEL + 11 ANIMATE + 3 UV/PAINT + 15 gestos + 1 métricas = 34, zero falhas/ignorados |
| `CARGO_BUILD_JOBS=1 cargo test -p petunia_ui_slint --config profile.test.package.petunia_ui_slint.debug=0 --lib` | pass: 508, zero falhas/ignorados; build 6m46s |
| `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_ui_slint --profile test --config profile.test.package.petunia_ui_slint.debug=0 --all-targets -- -D warnings` | pass, zero warnings; 7m38s |
| `node website/scripts/verify-progress.cjs` e `node --test website/tests/progress.test.cjs` | pass: 66 processos / 66 documentos e 6 testes do acompanhamento |
| `python3 website/scripts/verify-agent-docs.py` | pass: 66 rotas, 15 páginas de agentes, catálogo 39/189/20 |
| `git diff --check` | pass |
| Comparação estrutural e revisão do diff | pass: corpo MODEL igual, exceto indentação e `root.selection-domain`; todos os 119/49 encaminhamentos, seis bindings bidirecionais e retornos bool conferidos; API pública/markup pré-MODEL e PAINT/ANIMATE/UV/cauda iguais à baseline |

**Total Rust executado: 542 testes**, zero falhas/ignorados. O primeiro build com dois jobs teve somente o compilador de testes de unidade interrompido por orçamento de memória; a biblioteca pôde terminar e foi reutilizada. Essa tentativa interrompida não recebe pass. Os testes de teclado usam os eventos públicos `WindowEvent`, sem habilitar APIs internas do backend. O override `debug=0` afeta símbolos Rust do crate UI, preservando metadados Slint do backend headless e asserções.

Identidade das fontes testadas (SHA-256), conferida novamente antes do commit:

- `ui/app.slint`: `6a186422169ae2ace489375a1efca52fb1627d1efd1e6e076179090d10304397`.
- `ui/workspaces/model/inspector.slint`: `18b201b0cf0dbd8fffcc29afcb5be3f31b84d4d0b1ef09eb8c652dfdc8a8789d`.
- `tests/model_shell.rs`: `d1d27b79eb9d7bc75036f6044ad2ca72705fc7bf90913c50bdea96975b0c9722`.

**Acompanhamento:** U01 passa a **IN PROGRESS (050%)**, três de seis checkpoints; Q03 a **IN PROGRESS (025%)**, somente inclusão das suítes no workflow concluída. A cobertura de branch foi ampliada, mas auditoria completa de paths/toolchain, CI remota e demais gates de qualidade continuam pendentes. A suíte com `--features animation-workspace`, validação visual/manual, screen reader, GL, Windows e low-end permanecem **not run** neste slice. SHA do commit de entrega informado no handoff; não inferir push/CI a partir dos resultados locais.

**Risks:** cabeçalhos acessíveis e controles existentes são preservados, mas testes headless não comprovam equivalência visual, screen reader, Windows, GL ou low-end. Esses gates permanecem abertos na aceitação final U01/Rescue; não presumir Go a partir da extração.

**Next checkpoint:** conteúdo PAINT em slice separado, depois overlays/HUD restantes e aceitação final. A extração MODEL só aumenta o percentual U01 após os gates focados; U01 não vira DONE enquanto houver checkpoints abertos.

## 19. Wave U01 — PAINT e HUDs (2026-10-09)

**Intent:** concluir a implementação estrutural restante sobre `0883bfbedc2b1c81ed692da5d455fed2dcb19e0a`, branch `refactor/architecture-foundation`. Conforme instrução do usuário, implementar toda a wave antes de executar testes; a bateria começa somente após PAINT, overlays/HUD, regressões e integração estarem escritos.

**Sources:** §5 Shell estrutural e §6 Inspector deste plano; PAINT Workspace §2 (capacidades preservadas); Viewport Input Boundary; protocolo de implementação §C–F.

**Gap:** MODEL/UV/ANIMATE extraídos e previamente verificados; PAINT e guias/HUD ainda inline; gate visual/foco/teclado final parcial. A extração estrutural não implementa o novo macro-layout PAINT nem fecha o Rescue Go/No-Go.

**Changed:** `workspaces/paint/inspector.slint` projeta Canvas, Layers, Effects, Fill/Projection, Decal e Brush; bindings de ida/volta preservam painel do canvas e transforms do decal. A classificação de pincel permanece função única no shell. `viewport/guides.slint` mantém guias e previews antes do input; `cursor_hud.slint` mantém cursor, pivô, snap, cotas, medidas, eixos e pílula após o input; `status_hud.slint` mantém resumos após ViewBar. `operation_hud.slint` extrai conteúdo passivo, preservando o ToolCard interativo no host e sua posição. Nenhum componente novo recebe input ou ownership do documento. Regressões físicas `paint_shell`/`viewport_hud` e inclusão no workflow fazem parte da wave.

**Verification (bateria iniciada somente após implementar toda a wave):**

| Gate | Resultado |
|---|---|
| Comparação de corpos/projeções com a baseline; API/funções públicas e tail após PAINT preservados | pass |
| `cargo fmt -p petunia_ui_slint -- --check` e `git diff --check` | pass |
| `target/debug/xtask ui-lint` | pass, 29 arquivos |
| Testes de integração default abaixo | pass, 39 testes |
| `cargo test … --lib` | pass, 508 testes |
| `cargo clippy … --all-targets -- -D warnings` | pass, todos os targets |
| Validador do tracker, seis testes Node e guard documental | pass |
| Feature `animation-workspace`, CI remota e aceitação visual/nativa | not run |

```bash
CARGO_BUILD_JOBS=1 cargo test -p petunia_ui_slint --config profile.test.package.petunia_ui_slint.debug=0 \
  --test paint_shell --test viewport_hud --test model_shell --test uv_shell \
  --test viewport_gestures --test animate_shell --test ui_metrics
```

Bateria unitária e Clippy executados sequencialmente, no mesmo target, com um job:

```bash
CARGO_BUILD_JOBS=1 cargo test -p petunia_ui_slint --config profile.test.package.petunia_ui_slint.debug=0 --lib
CARGO_BUILD_JOBS=1 cargo clippy -p petunia_ui_slint --all-targets -- -D warnings
node website/scripts/verify-progress.cjs
node --test website/tests/progress.test.cjs
python3 website/scripts/verify-agent-docs.py
```

Unitários: **508 pass**; integração: **39 pass**; total: **547 testes Rust pass**, sem ignorados/filtrados nos resultados das suítes. Clippy: **pass** com `-D warnings`. Logs: `/tmp/petunia-wave-unit.log` e `/tmp/petunia-wave-clippy.log`. Ambiente: Linux x86_64, Rust 1.98.1, Slint 1.18.0; override `debug=0` apenas no crate UI em test, sem remover metadados/asserções. Configuração de CI foi atualizada, mas não executada remotamente. Após os gates, sessões incrementais antigas/sem lock foram removidas; caches mais recentes e qualquer sessão ativa preservados.

A primeira execução terminou em **fail**: o helper do teste emitia `PointerMoved` novamente antes do release e buscava a visibilidade da layer como button embora o contrato existente seja checkbox. Corrigido o harness, preservando asserções e código de produção, a execução consolidada passou: PAINT 3, HUD 2, MODEL 4, UV/PAINT 3, ANIMATE 11, gestures 15, métricas 1. Logs: `/tmp/petunia-wave-integration.log` (tentativa inicial) e `/tmp/petunia-wave-integration-final.log` (pass).

Painel local `http://localhost:8080/#/progress`: reload + filtro IN PROGRESS mostra U01 (083%, 5/6) e Q03 (025%, 1/4), com links canônicos; console sem warnings/errors. Captura local `petunia-u01-wave-checkpoint.jpg`. Esse check do site não substitui o gate visual da aplicação Slint.

PAINT projeta 70 propriedades, 50 callbacks e seis bindings de ida/volta. Guias/HUD são projeções sem input, comparadas à baseline com suas condições e posições preservadas. O shell passa de 8.802 para 7.145 linhas. U01 avança para **IN PROGRESS (083%)** (cinco de seis checkpoints); a aceitação final ainda está aberta.

**Risks:** testes headless não comprovam equivalência visual, leitor de tela, GL, Windows ou desempenho low-end. Layout/ownership completo dos workspaces, DTOs/intents e decisão Go/No-Go pertencem a outros processos.

**Next checkpoint:** implementação completa e bateria local aprovada. Registrar aceitação visual/nativa de layout, foco, teclado/F6, temas/escala e leitores de tela; completar a configuração `animation-workspace` e gates de plataforma quando executados. U01 permanece em andamento até existir prova da aceitação final; o Rescue Go/No-Go continua aberto.


Fontes da wave (SHA-256, arquivos verificados):

```text
5b45468b0cb9355377e2fae5e898f27b637cdbb26fe0853508c79dd7b2260a75  crates/ui-slint/ui/app.slint
5ee45e0127f55ff84895c53b91fcd77b5d3abf75bd8f81e43d00fbf87bb4809f  crates/ui-slint/ui/workspaces/paint/inspector.slint
bcf2106b6a6bc70bb34ff5a7f2ba073424a61158a6f3b558c7c342ab6bbac8d8  crates/ui-slint/ui/viewport/guides.slint
c80999450006e3de03a2d9536f2d56541f84f48fa514e8e823abc7ffdee1387f  crates/ui-slint/ui/viewport/cursor_hud.slint
05c1837d266af168f3ad14368fd85b1160f838e38f9776cceeccad11b2684026  crates/ui-slint/ui/viewport/status_hud.slint
e85d0011cff8a38b9fb725b1b7592f60606d7dc3dfb858cb592b103085671206  crates/ui-slint/ui/viewport/operation_hud.slint
69ec5cfe88df9636c5479591f72606998591b99d21aa7e459bf03a1eef998088  crates/ui-slint/tests/paint_shell.rs
cc3701e6d8745e29180bf1e534e4ed8230b20ff40bb51a89b3979a3ab5607a1f  crates/ui-slint/tests/viewport_hud.rs
```

## 20. Wave U02/U03 — regiões independentes e drawer (2026-10-09)

**Intent:** continuar a refatoração da UI sobre `1edfb2495e6b897b45adf148af019c1705fcc498`, branch `refactor/architecture-foundation`. Após iniciar o fechamento U01, o usuário orientou: “prossiga com as outras implementações, principalmente a refatoração da UI, deppois testamos”. A bateria desta wave e o aceite nativo ficam adiados; implementar não autoriza marcar gates pass.

**Sources:** §6–7 deste plano; Workspaces, Feedback e Acessibilidade §2–6, §19–20; PAINT UX §2–3 e palette/presets; UV UX §2–3 e Health; ANIMATE UX §1–4 e procedural transport; protocolo de implementação §C–F e contrato do tracker.

**Gap / Changed:**

| Processo | Antes | Código escrito nesta wave | Limite |
|---|---|---|---|
| U02 | Hosts em uma coluna com scroll comum | `InspectorSplitLayout` + `InspectorPane`; scroll próprio por região, proporção limitada, resize por ponteiro/setas/Home/End, collapse por Enter/Espaço/ação acessível; quatro workspaces conectados | Não compilado/testado; UV ainda usa operações em Structure e canvas pequeno em Properties |
| U02 | Layout sem memória por workspace | `inspector_layout.rs` guarda proporção/collapse em slots tipados de sessão; bridge valida e sincroniza apenas três propriedades por mudança | Não serializa layout no projeto; não altera Document/Undo/revisão |
| U03 | Drawer com Assets/Prefabs em todos os workspaces | `WorkspaceDrawerContent`: abas com teclado, memória local da aba por workspace, close/Esc; filtros/callbacks Assets preservados | Abertura/altura seguem o estado existente do drawer; restauração regional de foco pertence a U07 |
| U03 | Presets/paleta no Inspector PAINT | Corpos/ações migrados para `workspaces/paint/drawer.slint`, presets e paleta em abas próprias | Novo macro-layout Canvas 2D/3D/PiP não está neste slice |
| U03 | Conteúdo UV/ANIMATE inferior ausente | UV diagnostics projetado + Pack/Equalize; ANIMATE playhead/play/pause/Keep Live/Apply Now + seleção/duplicação/remoção de motions | Diagnóstico acionável por ilha, keyframes/curves e blending completo continuam pendentes |
| U06 | F6 sempre enviado à pill, mesmo com rail oculto no painel aberto | Token positivo entregue ao header Structure quando expandido; rail só recebe token quando recolhido; zero não força foco | Correção limitada, sem alegar navegação completa de regiões/Shift+F6 ou reader |
| U08 | Chips/swatches/alça sem teclado ou alvo adequado | `BrushChip` e `ColorSwatch` com foco/ação semântica/Enter/Espaço; alças com ações de incremento/decremento; alvos de 28px; texto operacional dos novos hosts/drawer e chips usa token de 12px | Auditoria global de estados/temas/escala, nomes/estados globais e leitores de tela aberta |

MODEL preserva Parts/Transform/Material/Object/Combine e mantém headers de Properties alcançáveis no painel aberto. PAINT usa Layers em Structure e decal em Properties; ANIMATE passa Creature + picker/stack para Structure, mantendo parâmetros em Properties. Contratos autorais, intents, renderers e schemas não são reescritos. O transporte flutuante não aparece simultaneamente com o transporte do drawer aberto.

O divisor inferior usa a faixa persistida existente (132–520px), limita a altura visual à janela e calcula arraste em coordenadas absolutas para evitar realimentação quando a própria alça se move. Em janelas muito baixas e layouts estreitos, a usabilidade ainda requer validação real.

**Verification:**

| Gate / evidência | Estado real |
|---|---|
| Antes desta wave: inicialização da aplicação pelo `cargo run` no terminal do usuário (fontes de `1edfb24`), terminal “Petunia3D window ready” | pass somente para startup; não comprova esta wave nem layout/teclado |
| Antes do adiamento: `CARGO_BUILD_JOBS=1 cargo test -p petunia_ui_slint --config profile.test.package.petunia_ui_slint.debug=0 --test shell_focus` | fail: 1 pass / 1 fail; Enter não chegou ao callback esperado durante travessia dos workspaces; log `/tmp/petunia-u01-focus.log` |
| Revisão de código da rota F6 | Identificado token para rail oculto quando expandido; correção escrita, comportamento pós-correção não executado |
| Formatação Rust com `cargo fmt -p petunia_ui_slint` | Aplicada como edição; `--check` não executado nesta wave |
| Revisão de whitespace com `git diff --check` | pass após remover espaço residual da movimentação do bloco PAINT; não é teste funcional |
| Compilação/check, testes unitários/integração, Clippy, ui-lint, validadores site/tracker | not run após orientação de adiamento |
| Aceite visual/nativo, Linux/Windows, escala/temas, AT-SPI/leitor, GL, low-end e CI remota | not run |

Regressões escritas, **não executadas**: três unitários de memória/validação, `shell_focus` (3), `inspector_split` (2), `workspace_drawer` (3). Cobrem input físico, roteamento de ações, limites, collapse, recuperação de layout/filtros e teclado. Configuração CI inclui as três suítes; YAML alterado não comprova execução remota. Os 547 pass da wave anterior continuam vinculados apenas às fontes anteriores.

**Acompanhamento:** U01 continua IN PROGRESS (083%) com aceite final aberto; U02/U03/U06/U08 ficam IN PROGRESS (000%) sem checkpoints novos marcados. Q03 continua IN PROGRESS (025%) pela configuração já entregue, sem inferir gates novos. Nenhum denominador foi alterado.

**Risks:** toda a wave é `implemented, unverified`; compilação pode revelar diagnósticos e regressões precisam confirmar a integração. A11y inclui melhorias locais, sem conformidade global presumida. O renderer efetivamente iniciado na baseline continua WGPU histórico; isso não implementa o OpenGL de destino.

**Next checkpoint:** prosseguir nos slices de UI autorizados; quando iniciar a fase de testes, compilar e executar a bateria inteira das fontes finais, começando pelas três suítes novas, depois integrações existentes/lib/Clippy/ui-lint/guard do site. Aceite nativo/plataforma permanece evidência separada. Não fechar U01/U02/U03/U06/U08 apenas pela presença dos arquivos.

### Correção de compilação Slint após retomada

O `cargo run` iniciado pelo usuário durante a entrega encontrou **fail** no markup: `WorkspaceDrawer` redeclarava `maximum-height`, propriedade reservada do Slint. Renomeado para `drawer-height-limit` sobre o commit `9e8998adcd6626f380354def0f9dc102f24973b9`.

O build script já compilado foi executado diretamente, com `OUT_DIR` isolado em `/tmp/petunia-u02-u03-slint`, `CARGO_MANIFEST_DIR` do crate, `PROFILE=debug` e `TARGET=x86_64-unknown-linux-gnu`. Resultado **pass** (exit 0) para compilação Slint/geração de Rust, sem warnings/errors no log `/tmp/petunia-u02-u03-slint.log`. Nenhum teste nem rustc da aplicação foi iniciado por essa verificação. Artefato temporário de geração pode ser removido após uso; o log permanece.

```bash
# Executado a partir de crates/ui-slint, usando o build script produzido pelo cargo run do usuário:
OUT_DIR=/tmp/petunia-u02-u03-slint \
CARGO_MANIFEST_DIR=/home/raillen/Documentos/petunia3d-refact/petunia3d-refact/crates/ui-slint \
PROFILE=debug TARGET=x86_64-unknown-linux-gnu \
../../target/debug/build/petunia_ui_slint-1c1dad84ce22d80d/build-script-build
```

Isso comprova somente que o compilador Slint aceita o markup corrigido. Compilação completa Rust, regressões, layout/foco e aceite nativo permanecem **not run** depois da correção, conforme o adiamento do usuário. A falha inicial do `cargo run` e a regressão anterior `shell_focus` não recebem pass retroativo.

## 21. Slice U04 — layout bridge e viewport DTOs (2026-10-09)

Layout/sections extraídos para `bridge/shell_layout.rs`; select/box/lasso seguem `UiIntent::ViewportSelection`, hover segue DTO At/Clear e tool pointer recebe fase/px lógicos/modificadores tipados. `bridge/viewport.rs` faz conversão/refresh, `bridge/viewport_actions.rs` executa as consultas e a gramática existentes. Sem novo owner de Document/Geometry, renderer ou frontend. API antiga preservada. Registro de unidades, código movido, regressões não executadas, check e limites: [Viewport Input Boundary §22](./viewport-input-boundary.md#22-slice-u04--dtos-e-execucao-no-bridge-2026-10-09). U04 IN PROGRESS (000%); decomposição total e gates ainda abertos.


Check de compilação do conjunto U02/U03/U04: `CARGO_BUILD_JOBS=1 cargo check -p petunia_ui_slint --lib` **pass**, exit 0 em 3m20s, sem warnings (`/tmp/petunia-u04-check.log`). O check default inclui o markup corrigido; não é execução dos testes, feature animation-workspace, link/startup, aceite visual/reader/GL/Windows. U02/U03/U04/U06/U08 continuam em andamento, sem novos checkpoints comportamentais concluídos.

## 22. Slice U06/U07 — navegação regional (2026-10-09)

Ciclo F6/Shift+F6 com sete entradas, skip de regiões indisponíveis (incluindo Context Bar vazio em ANIMATE), captura de binding antes da navegação, bloqueio de overlays e retorno regional escrito. Header/Tools/Work Surface/Context usam anchors sem TouchArea; Structure/Properties/Drawer reutilizam entradas existentes. Estado de apresentação no shell; nenhum novo owner autoral. Detalhes e limites: [Workspaces §23](./workspaces-feedback-accessibility.md#23-slice-u06u07--entradas-regionais-e-retorno-de-foco-2026-10-09). Tab confinado, focus trap completo, memória por controle e reader ainda pendentes. U06/U07 IN PROGRESS (000%); testes adiados pelo usuário.

Compilação das fontes finais: `CARGO_BUILD_JOBS=1 cargo check -p petunia_ui_slint --lib` **pass**, exit 0 em 2m04s, sem warnings (`/tmp/petunia-u06-check-final.log`). Linux x86_64, Rust 1.98.1, default library. Quatro regressões preparadas, **not run**; feature/runtime/aceite nativo/reader/Windows pendentes. Identidade das fontes e limites em Workspaces §23.

## 23. Slice U06/U07 — foco dos modais Settings/Palette (2026-10-09)

Boundary de Tab/Shift+Tab usando o percurso nativo, foco inicial nas buscas, retorno aos dois invocadores reais do Header, captura de binding antes do controle e precedência de Escape modal sobre ferramenta. Click-away trata somente o topo/pin; Add popover passa a integrar OverlayStack. O topo é projetado pelo bridge para o Slint. Conteúdo, authoring, formatos e renderer preservados. Gap, arquivos, evidência, limites de role Slint e próximo checkpoint: [Workspaces §24](./workspaces-feedback-accessibility.md#24-slice-u06u07--contencao-de-dois-modais-e-invocadores-header-2026-10-09). U06/U07 IN PROGRESS (000%); dez regressões escritas, not run por adiamento do usuário; compilação final default --lib pass, exit 0 em 2m15s, sem warnings; Linux x86_64/Rust 1.98.1. Log `/tmp/petunia-u07-check-final.log`; identidade das fontes registrada em Workspaces §24.

## 24. Slice U07 — item de menu acionável e retorno ao invocador do popover de criação (2026-10-09)

`ContextMenuItem` passa a ser acionável por teclado (Enter/Espaço, anel de foco, ação semântica) sem mudar o caminho de pointer. O popover de criação lembra o botão do Tool Rail como invocador e devolve o foco a ele quando fecha; o atalho global (`model.primitives`) continua sem invocador de controle. Popover segue não modal: F6 e atalhos de ferramenta permanecem ativos. Draft de texto sobrevive; caret não é restaurável porque o Slint 1.18 expõe `cursor-position-byte-offset` somente como propriedade `out` interna de teste. Gap, arquivos, evidência e limites: [Workspaces §25](./workspaces-feedback-accessibility.md#25-slice-u07--item-de-menu-acionavel-e-retorno-ao-invocador-do-popover-de-criacao-2026-10-09). U07 IN PROGRESS; duas regressões escritas, **not run**; compilação `--lib --tests` pass, exit 0 em 4m19s, sem warnings; log `/tmp/petunia-u07-menus-check.log`.
