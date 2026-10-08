# 35 — egui, Petunia Components, UI Adapters e Tooling de Desenvolvimento

<aside>
🧩

Este capítulo define **como o Petunia3D usa egui sem deixar o toolkit dominar a arquitetura ou o visual do produto**. egui fornece fundação de interação/layout/accessibility; Petunia Components define identidade; crates auxiliares entram atrás de adapters quando resolvem infraestrutura real. Esta página complementa os capítulos 24–28 e 31.

</aside>

# Princípio central

A cadeia normativa da UI é:

```
Petunia workspaces/screens
        ↓
Petunia Components
        ↓
Petunia UI adapters
        ↓
egui / eframe / egui-wgpu / selected ecosystem crates
```

Não espalhar widgets de terceiros diretamente pelos workspaces. Um workspace deve depender do contrato `PetuniaButton`, `PetuniaTree`, `PetuniaNumberField`, `PetuniaPanel` etc., não de uma crate auxiliar específica.

# O que egui deve resolver

Delegar ao egui:

- event/input handling;
- immediate-mode layout;
- text editing/rendering primitives;
- focus/keyboard routing;
- scroll areas;
- menus/popups;
- core widgets;
- accessibility tree via AccessKit;
- painting 2D;
- desktop host via eframe;
- integração GPU via egui-wgpu.

# O que continua pertencendo ao Petunia

- Design System e tokens;
- anatomia final dos componentes;
- estilo visual;
- comportamento específico de DCC/modelagem;
- Command binding;
- viewport 3D;
- gizmos 3D finais;
- Tool state;
- menus semânticos do produto;
- accessibility labels/semantics do domínio;
- regras de focus/keyboard do Petunia;
- visual regression baselines.

# Estrutura recomendada de `petunia-ui`

```
petunia-ui/
├ foundation/
│  ├ tokens.rs
│  ├ colors.rs
│  ├ typography.rs
│  ├ spacing.rs
│  ├ radii.rs
│  ├ strokes.rs
│  ├ metrics.rs
│  ├ motion.rs
│  └ icons.rs
│
├ components/
│  ├ button.rs
│  ├ icon_button.rs
│  ├ split_button.rs
│  ├ segmented_control.rs
│  ├ number_field.rs
│  ├ panel.rs
│  ├ panel_header.rs
│  ├ context_section.rs
│  ├ tree.rs
│  ├ menu.rs
│  ├ modal.rs
│  ├ tooltip.rs
│  ├ toast.rs
│  ├ asset_card.rs
│  └ workspace_pill.rs
│
├ adapters/
│  ├ tree.rs
│  ├ tiles.rs
│  ├ file_dialog.rs
│  ├ table.rs
│  ├ markdown.rs
│  └ icons.rs
│
├ workspaces/
│  ├ model/
│  ├ paint/
│  └ uv/
│
└ devtools/
   ├ inspection.rs
   ├ probe.rs
   ├ json.rs
   ├ profiler.rs
   └ logs.rs
```

Uma dependency abandonada deve ser substituível principalmente dentro de `adapters/`.

# Petunia Components — baseline

Componentes fundamentais próprios:

```
PetuniaButton
PetuniaIconButton
PetuniaSplitButton
PetuniaSegmentedControl
PetuniaNumberField
PetuniaTextField
PetuniaSelect
PetuniaCheckbox
PetuniaSwitch
PetuniaPanel
PetuniaPanelHeader
PetuniaContextSection
PetuniaTree
PetuniaToolbar
PetuniaWorkspacePill
PetuniaAssetCard
PetuniaTooltip
PetuniaModal
PetuniaToast
PetuniaMenu
PetuniaCommandPalette
```

Cada componente deve definir:

```
visual states
semantic role
accessible label/description contract
focus behavior
keyboard behavior
pointer behavior
minimum hit area
layout metrics
token dependencies
snapshot fixtures
interaction tests
```

# Design System sobre egui

`egui::Style`, `Visuals`, `Spacing`, `WidgetVisuals` e Painter são implementação; não são o design system em si.

Fluxo:

```
Figma/Notion tokens
      ↓
PetuniaTokens
      ↓
PetuniaThemeAdapter
      ↓
egui Style/Visuals
      +
custom Petunia Components
```

Tokens devem usar nomes semânticos, por exemplo:

```
surface.canvas
surface.panel
surface.panel_hover
border.subtle
border.focus
text.primary
text.secondary
accent.primary
selection.background
error.surface
spacing.panel_gap
radius.control
metric.toolbar_height
```

Não espalhar valores RGB/px diretamente por workspaces.

# Ecossistema — classificação

Toda crate/projeto do ecossistema deve receber **uma classe de adoção explícita**. As classes normativas são:

- **BASELINE** — dependency aprovada para o baseline corrente quando a área correspondente é construída;
- **BASELINE CANDIDATE** — forte candidata, mas não deve entrar no Cargo baseline antes da validação específica;
- **OPTIONAL** — pode ser adotada somente se uma feature real justificar o custo;
- **DEV TOOL** — permitida apenas em development/test/tooling, sem exposição em distribuição normal por padrão;
- **FUTURE** — reservada a feature/milestone posterior; não bloquear V1;
- **REFERENCE/MONITOR** — estudar/acompanhar implementação, ergonomia ou evolução; não é dependency baseline;
- **REJECTED/NOT RELEVANT** — avaliada e não deve ser adotada para o uso indicado, salvo nova decisão explícita com rationale.

A classificação é de **adoção**, não de qualidade geral da biblioteca. `REFERENCE/MONITOR` e `REJECTED/NOT RELEVANT` podem continuar sendo fontes de estudo sem contaminar a stack.

> **Revisão vigente.** A *Egui Ecosystem Final Push Directive* (§17, §21 e §43)
> atualiza as classificações abaixo depois de medir o custo de resolver layout à
> mão no product code: raw egui fica para **micro-layout simples**, e Taffy,
> Tiles, DnD, Motion e o ecossistema de tabelas/forms/async passam a ter papel de
> base nos product paths. A ordem normativa de compatibilidade é
> **release crates.io → upstream atual → revision pin → fork mínimo Petunia →
> implementação local/defer** (§18).

| Crate/projeto | Uso Petunia | Status |
| --- | --- | --- |
| `egui_extras` | image loaders, TableBuilder, layouts auxiliares | **BASELINE** |
| `egui_ltreeview` | Parts tree hierárquica | **BASELINE** |
| `egui_tiles` | engine de macro-layout controlado (`PetuniaLayoutAdapter`, sem docking livre) | **BASELINE** |
| `egui_taffy` | layout responsivo complexo / flex / wrap / grid (`PetuniaTaffyLayout`) | **BASELINE** |
| `egui-file-dialog` | browser/dialogs customizados | **OPTIONAL** |
| `egui_kittest` | semantic/interaction/snapshot UI tests | **DEV TOOL** |
| `egui_inspection` | inspeção externa/accessibility tree/input/screenshot | **DEV TOOL** |
| `egui_mcp` | controle da UI por agentes no ambiente de desenvolvimento | **DEV TOOL** |
| `iconflow` | carga dos packs genéricos oficiais (`tabler`, `iconoir`, `phosphor`, `lucide`) atrás do `IconRegistry` | **BASELINE** |
| pack **Petunia** | ícones de domínio 3D sem equivalente genérico (Extrude, Inset, Round Edge, Loop Cut, shading, pivot) | **BASELINE** |
| `egui-lucide` | pack genérico isolado — superado por `iconflow` | **REFERENCE/MONITOR** |
| `egui-phosphor` | referência/alternativa de iconografia | **REFERENCE/MONITOR** |
| `egui_animation` | motion/collapse/easing (`PetuniaMotion`) | **BASELINE** |
| `egui_dnd` | reorderable lists (`PetuniaDragAdapter`) | **BASELINE** |
| `egui_form` | forms/validation complexos (`PetuniaFormAdapter` após gate) | **P1 — adotar em product paths** |
| `egui_table` | tabelas (`PetuniaTableAdapter` após gate) | **P1 — adotar em product paths** |
| `egui_virtual_list` | coleções grandes virtualizadas | **P1 — adotar em product paths** |
| `egui_suspense` | estados async/loading declarativos | **P1 — adotar em product paths** |
| `egui-notify` | notificações (fork mínimo Petunia, §19.3) | **P1-patched** |
| `egui_commonmark` | Markdown/help/release notes | OPTIONAL |
| `egui-data-table` | tabelas editáveis avançadas | **FUTURE** |
| `egui-snarl` | node editor futuro | **FUTURE** |
| `egui_json_tree` | diagnostics/JSON inspector | DEV TOOL |
| `egui_probe` | painéis de debug derivados de structs | DEV TOOL |
| `puffin_egui` | profiler embutido | **DEV TOOL** |
| `tracing-egui` | viewer de tracing/logs | **DEV TOOL** |
| `egui_code_editor` | futuro editor Lua/dev console | **FUTURE** |

## REFERENCE/MONITOR e rejeições explícitas

- `egui_dock` — **REFERENCE/MONITOR** como comparação de docking/IDE; não é baseline porque o shell do Petunia deve ser mais controlado.
- `egui-elements` — **REFERENCE/MONITOR** para theme architecture e component anatomy.
- `egui-elegance` — **REFERENCE/MONITOR** para controles compostos, tokens, snapshots e accessibility tests.
- `egui-components` — **REFERENCE/MONITOR** para implementações de componentes complexos; não deve se tornar o Design System Petunia.
- `egui_component` — **REFERENCE/MONITOR** enquanto estiver em linha/estágio incompatível ou imaturo para a baseline pinada.
- `egui-material3` — **REFERENCE/MONITOR** para component anatomy; **REJECTED/NOT RELEVANT como Design System/tema do Petunia**.
- `catppuccin-egui` — **REFERENCE/MONITOR** para mapping de cores; **REJECTED/NOT RELEVANT como tema final**.
- `egui-gizmo` — **REFERENCE/MONITOR** para ergonomia/algoritmos; **REJECTED/NOT RELEVANT como gizmo final de produção** na baseline atual, pois o gizmo Petunia deve integrar renderer, picking e snapping próprios.

Rejeição é **escopada ao uso indicado**. Uma implementação rejeitada como dependency de produção ainda pode permanecer como referência técnica sem virar fallback silencioso.

# `egui_extras`

Considerar parte natural da baseline por ser extensão oficial/coordenada com a linha egui.

Usos:

- image loaders para PNG/JPEG/SVG/WebP quando habilitados;
- thumbnails da Asset Library;
- reference images;
- tabelas simples;
- strips/layout helpers.

Não transformar `egui_extras` em dependência de core/domain.

# Parts — `egui_ltreeview`

É a implementação **BASELINE** atrás de `PetuniaTreeAdapter` para a árvore `Parts` porque fornece:

- directory/leaf nodes;
- single/multi selection;
- keyboard navigation;
- drag-and-drop;
- context menus;
- large-tree culling;
- styling customizável.

Criar `PetuniaTreeAdapter` que converte `PartsViewModel` para a crate. Workspaces/painéis não usam a crate diretamente.

Seleção continua sendo estado da Application/UI e não propriedade persistente da topology.

# Layout — `egui_tiles`

É **BASELINE** atrás de `PetuniaLayoutAdapter`. Foi escolhida sobre docking totalmente livre porque consegue representar containers horizontal/vertical/grid/tabs sem obrigar o Petunia a expor todos esses graus de liberdade ao usuário.

> **Status (2026-09-16):** a crate ainda **não é dependência** do workspace e o
> `PetuniaLayoutAdapter` ainda não existe — o macro-layout atual é o dock próprio
> controlado. A integração está na Wave 2 da *Egui Ecosystem Final Push Directive*
> (§22, §31, §44). O capítulo 36 registra a mesma decisão.

O layout **dentro** de um painel ou painel-flutuante é problema separado: para
além de micro-layout simples (colunas/linhas diretas permitidas em built-in),
responsividade, flex, wrap e grid pertencem ao `PetuniaTaffyLayout`, nunca a
`available_width()` comparado com um número em product code.

Uso possível:

```
Parts | Viewport | Context
          ↓
      Asset Library
```

Petunia controla:

- quais regiões existem;
- min/max size;
- collapse rules;
- persistência de ratios permitidos;
- se tabs/docking são visíveis;
- reset workspace layout.

A existência da capacidade de docking na crate **não autoriza** adicionar docking irrestrito à V1.

# `egui_dock`

Alternativa madura para um modelo mais parecido com IDE. Manter como **REFERENCE/MONITOR** para comparação; não é fallback automático nem baseline, porque o Petunia quer shell mais controlado.

# File browser — `egui-file-dialog`

Permanece **OPTIONAL**, não baseline. O fluxo padrão V1 usa dialogs nativos do sistema operacional para Open/Save/Import/Export. `egui-file-dialog` pode ser adotada posteriormente quando um workflow realmente exigir browser integrado/custom filesystem:

- open/save;
- file/folder selection;
- multi-selection;
- criação de diretório;
- keyboard navigation;
- custom right panel;
- custom filesystem quando necessário.

Ainda é permitido utilizar native OS file dialogs onde a experiência for melhor. A escolha deve ser por workflow, não dogma.

# Numeric controls

`egui::DragValue` é a fundação preferida para `PetuniaNumberField` em parâmetros DCC:

```
click/type
+
horizontal drag to adjust
+
range/step/speed
+
unit-aware formatter
```

O wrapper Petunia adiciona tokens, label, units, reset/default, validation, accessible name e drag semantics coerentes.

# Menus, popups e modal

Preferir widgets nativos egui quando suficientes e encapsular:

```
egui menu → PetuniaMenu
egui Modal → PetuniaModal
popup/context menu → PetuniaPopup/PetuniaContextMenu
```

Não adicionar dependency apenas para reproduzir um componente que o core atual já implementa adequadamente.

# Command Palette

Uma command palette pode usar `egui_palette` ou implementação pequena própria sobre Command Registry.

Regra:

```
fuzzy search result
→ CommandId
→ CommandDispatcher
```

Nunca mapear palette diretamente para callback de botão.

# UI kits: política geral

UI kits externos servem principalmente para **estudar implementação, ergonomia e component anatomy**. Não adotar seu visual como identidade do Petunia.

## `egui-elements`

Referência forte para:

- theme architecture;
- component visuals;
- modal/combo/text controls;
- live theme editing.

Permanece **REFERENCE/MONITOR**; só pode virar dependency seletiva após reclassificação explícita e validação de compatibilidade, licença e dependency cost.

## `egui-elegance`

Referência forte para:

- SegmentedControl;
- Tabs;
- Switch;
- TextInput;
- Toast;
- Tooltip;
- SortableList;
- theme/tokens;
- snapshot/accessibility tests.

Usar principalmente como estudo e eventualmente wrapper seletivo.

## `egui-components`

Coleção ampla inspirada/portada do ecossistema gpui com components como Accordion, Alert, Badge, Breadcrumb, Button, Card, Dialog, Form, Menu, NumberInput, Popover, Resizable, Sidebar, Table, Tabs, Tree etc.

Monitorar compatibilidade com a linha egui corrente. Pode fornecer boas implementações de componentes complexos, mas não deve se tornar o Design System Petunia.

## `egui_component`

Arquitetura de styling desacoplado é interessante e alinhada ao Petunia. Enquanto permanecer alpha/linha egui diferente, tratar como **REFERENCE/MONITOR**, não baseline.

## `egui-material3`

Material 3 não é o visual do Petunia. A biblioteca é referência para componentes complexos como ActionSheet, Badge, Breadcrumb, Timeline, Toolbar, TreeView e DataTable. Features pesadas não devem ser importadas apenas para reaproveitar um widget.

## `catppuccin-egui`

Referência de mapping de paleta/semantic colors. Não usar como tema final.

# Icons

A estratégia é híbrida:

```
ícones genéricos
→ um pack permissivo selecionado

ícones de modelagem Petunia
→ SVGs próprios
```

Candidatos genéricos:

- Lucide: visual leve/minimalista;
- Phosphor: catálogo amplo e coerente;
- Material Symbols: fallback de catálogo grande, não identidade principal.

Escolher **um** pack principal depois de teste visual. Não carregar três icon packs na baseline.

Ícones específicos como Extrude, Bevel, Slice, Fuse, Connect, Face/Edge/Point, seams e UV permanecem assets próprios do Petunia quando o pack genérico não comunicar bem o domínio.

# Notifications/toasts

Preferir `PetuniaToast` próprio sobre infraestrutura pequena. `egui-notify`/`egui-toast` podem ser usados quando atualizados para a linha pinada e quando reduzirem complexidade real.

Toasts não substituem dialogs para decisões destrutivas/importantes.

# Tabelas

Hierarquia de escolha:

```
Tabela simples
→ egui_extras::TableBuilder

Dataset muito grande/virtualizado
→ egui_table

Tabela editável avançada
→ egui-data-table, somente se houver requisito real
```

Possíveis usos futuros: Asset Library list mode, validator results, plugin manager, developer/MCP inspectors.

# Node graph — `egui-snarl`

É candidato para futura visualização em nós (textura/efeitos/automação), nunca requisito do Core V1. Encapsular em official extension/module para evitar que graph types contaminem Document Core antes de a feature existir.

# Gizmo 3D

`egui-gizmo` é referência valiosa de ergonomia/algoritmo, mas não baseline devido à diferença de linha egui e porque o gizmo final deve integrar profundamente com o renderer/picking Petunia.

Regra preferida:

```
PetuniaRenderer / GizmoRenderer
→ draw + hit test + snap
```

Podemos usar uma implementação externa em spike inicial, mas o produto final não deve depender dela sem reavaliação.

# Motion — `egui_animation`

Pode apoiar:

- collapse de painéis;
- hover easing;
- workspace transition;
- toolbar contextual;
- Asset Library expand/collapse.

Sempre respeitar `Reduced Motion`. Motion é detalhe de apresentação, nunca fonte de estado de domínio.

# Async/UI jobs

O Application Core permanece síncrono. Não introduzir async por conveniência de widget.

- `egui_inbox` pode ser útil apenas para mensagens locais de apresentação;
- `egui-async` fica restrito a futuros serviços realmente assíncronos, como update checker ou rede.

Boolean/Auto UV/geometry continuam em Rayon/jobs + channels/revision check.

# Forms — `egui_form`

Pode apoiar validation de Settings, Export Settings e configuração de plugins, mas os dados permanecem structs Rust tipadas. A crate não substitui `UserSettings`/domain types por um mapa genérico.

# Dev tool — `egui_probe`

Permitido para developer/debug panels onde geração automática é desejável:

```
MeshDebugSettings
RendererDebugSettings
SnapDebugState
ProviderDiagnostics
```

Não usar para UI pública final porque UI de produto exige hierarquia, textos e comportamento deliberados.

# Dev tool — JSON inspector

`egui_json_tree` é útil para:

- manifest/projeto durante debugging;
- plugin persisted state;
- MCP payloads;
- Command schemas;
- settings/config inspection.

Não expor informação sensível em builds/reports públicos sem sanitização.

# Profiling e logs

`puffin_egui` é candidato para painel de profiling em builds dev/profiling.

`tracing-egui` ou viewer próprio pode apresentar eventos `tracing` se a crate escolhida estiver suficientemente madura/compatível.

Esses painéis não entram na distribuição normal por default.

# Markdown/help

`egui_commonmark` é candidato para:

- Quick Start;
- What's New;
- release notes;
- plugin help;
- documentação embutida curta.

O manual canônico pode continuar externo/web; Markdown embutido não deve duplicar toda a documentação.

# Code editor futuro

`egui_code_editor` pode apoiar Lua plugin editor/dev console futuramente. Não adicionar editor de código à V1 apenas porque a crate existe.

# Bibliotecas pesquisadas e rejeitadas como relevantes

## eHMI

Foco em HMI/robótica com gauges/bars/toggles. Não resolve uma necessidade Petunia que o core/UI própria não cubra. **Não adotar.**

## longbridge-candlesticks

Biblioteca de domínio financeiro/trading, não componente egui relevante para o Petunia. **Fora da stack.**

# AccessKit e componentes customizados

Todo wrapper customizado deve manter/inserir `WidgetInfo`/semântica equivalente correta.

Exemplo conceitual:

```
PetuniaIconButton
├ custom painter
├ hover/press visuals
├ CommandId
├ tooltip/help
└ AccessKit role/name/state
```

Visual customizado sem semântica é bug.

# `egui_kittest` como Definition of Done de componente

Cada componente principal deve possuir, quando aplicável:

- query por role/name;
- keyboard activation;
- pointer click/drag;
- disabled/selected/expanded state;
- focus behavior;
- screenshot em estado normal/hover/focus/disabled;
- scaling fixtures;
- labels longos/i18n.

Golden visual só muda por decisão explícita de Design System.

# `egui_inspection` + `egui_mcp`

Fluxo de agentes:

```
read Figma/Notion
→ implement Petunia Component
→ cargo xtask ui-test
→ launch development build
→ inspect AccessKit tree
→ mouse/keyboard interaction
→ screenshot
→ compare reference
→ fix
→ repeat
```

Regras de segurança:

- feature/build flag de desenvolvimento;
- loopback only por default;
- nunca habilitado silenciosamente em release;
- não expor controle remoto para rede pública;
- screenshots/logs tratados como dados de desenvolvimento.

# Figma handoff com egui

Figma continua sendo visual truth; não há export de UI de produção.

Pipeline:

```
Figma variables/styles/components
→ audited design tokens/component specs
→ source-controlled Rust tokens
→ Petunia Components
→ semantic/snapshot tests
```

Um extractor pode gerar material intermediário, mas a UI final é Rust/egui e segue os contratos do Notion.

# Critério de adoção de crate

Antes de promover qualquer crate para baseline:

1. licença compatível com MIT/permissiva;
2. versão compatível com egui pinado;
3. manutenção ativa suficiente;
4. API compreensível por agentes;
5. Windows/Linux;
6. accessibility preservável;
7. dependências transitivas aceitáveis;
8. adapter Petunia possível;
9. testes próprios cobrindo nosso uso;
10. benefício maior que escrever wrapper/implementação pequena própria.

# Regra anti-bloat

Não adicionar 20 crates de UI de uma vez. O catálogo deste capítulo é uma **biblioteca de decisões**, não uma lista de dependências obrigatórias.

Baseline cresce por demanda real:

```
need tree
→ adopt tree adapter

need huge table
→ evaluate egui_table then

need node graph
→ evaluate egui-snarl then
```

# Regra final

> **egui fornece infraestrutura; Petunia Components fornece identidade. Crates auxiliares resolvem problemas locais atrás de adapters e nunca definem o produto.**
> 

# Theme Extensions e Plugin Panels

A camada `petunia-ui` também é a autoridade de implementação para as superfícies públicas definidas no capítulo 36:

- `.petunia-theme` declarativo → `PetuniaThemeLoader` → `PetuniaTokens` → `PetuniaThemeAdapter`;
- `Plugin Panel descriptor` → `PetuniaPanelRegistry` → `PetuniaUiBuilder` → Petunia Components → egui.

Plugins **não recebem tipos egui/wgpu**. Theme packages não executam Lua durante resolução de tokens. Ambos devem permanecer atrás de contratos versionados e testáveis, preservando substituibilidade do toolkit e consistência visual.