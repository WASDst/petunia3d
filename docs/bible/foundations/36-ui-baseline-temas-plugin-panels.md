# 36 — UI Baseline Final V1, Temas e Plugin Panels

<aside>
🎨

Este capítulo congela a **UI Baseline Final V1** do Petunia3D e define duas superfícies oficiais de extensibilidade visual: **Theme Extensions declarativas** e **Plugin Panels Lua sobre Petunia Components**. Ajustes finos posteriores podem calibrar valores, mas não reabrem automaticamente a arquitetura, o layout principal ou a stack.

</aside>

# Status

**UI BASELINE FINAL V1 — APPROVED.**

**Decisão 2026-09-20: Slint é o frontend de produção.** A crate `petunia_ui_slint`
(`crates/ui-slint/`) é a interface principal do Petunia3D, construída com Slint 1.18.
O binário `petunia3d` executa o shell Slint por padrão; a UI egui (`crates/ui/`) é
legado de transição acessível via `--legacy-egui` / `PETUNIA_LEGACY_EGUI=1`.

Os princípios, medidas, workspaces e contratos de acessibilidade deste capítulo são
**toolkit-neutros** — aplicam-se independentemente do frontend. A migração para Slint
preserva esses contratos; o domínio (`petunia_core`, `petunia_commands`, `petunia_project`,
`petunia_config`) não conhece Slint nem egui.

O vertical slice do capítulo 31 continua obrigatório como **teste de conformance e
integração**. Reabertura da arquitetura exige bloqueador estrutural real + ADR explícito.

# Princípios congelados

- viewport-first;
- shell profissional simplificado, nunca “Blender amputado” nem aplicativo infantil;
- trilho de ferramentas do workspace à esquerda; em DRAW e POLY, `Parts` é a
  primeira seção do `Inspector` à direita, seguida por `Transform`, `Material` e
  `Object`;
  `Asset Library` permanece inferior;
- painéis semi-flutuantes, retráteis e redimensionáveis dentro de limites explícitos;
- `DRAW / POLY / PAINT / UV` como workspaces V1 (revisão de 2026-09-29, ADR 006; antes `MODEL / PAINT / UV`);
- toolbar contextual dentro do viewport;
- seleção explícita por workspace: POLY `Object / Face / Edge / Point`; DRAW `Shape / Curve / Point / Region`;
- uma única gramática de ferramenta para todas as ferramentas (constituição 11);
- `Wireframe / Solid / Textured / Silhouette` como modos-base;
- overlays como composição sobre modo-base;
- Command Registry como fonte de ações, shortcuts, plugins e command palette;
- Petunia Components como única linguagem visual pública da aplicação;
- AccessKit + keyboard/focus como parte do contrato, não acabamento tardio;
- nenhum docking irrestrito na V1.

# Shell e medidas de referência V1

Valores são em **logical px** antes de UI scaling.

| Elemento | Baseline | Regra |
| --- | --- | --- |
| Top bar | 40 | menus/projeto à esquerda, workspace pills ao centro, ações globais à direita |
| Trilho de ferramentas DRAW/POLY | 40–46 | ferramentas persistentes do workspace, sem duplicar o Inspector |
| Inspector DRAW/POLY | 320–360 default; 280–460 | Parts/Transform/Material/Object; seções recolhíveis, lista de Parts virtualizada e rolagem vertical |
| Asset Library | 176 default; 120–360 | resize vertical + collapse |
| Panel Header | 28 | um contrato único |
| Control | 28 | 30–32 somente quando a hierarquia justificar |
| Status Strip | ~22 | hints contextuais + stats/save/validation |
| Gutter estrutural | 8 | 4 apenas entre controles intimamente relacionados |

Preservar aproximadamente `480 × 360` logical px de viewport antes de ceder mais espaço a painéis.

## Breakpoints desktop

- `>= 1280`: shell completo;
- `1024–1279`: Asset Library inicia recolhida;
- `< 900`: Inspector pode atuar como drawer temporário à direita para preservar viewport; Parts não volta à lateral esquerda.

# Layout e docking

O shell Slint (`crates/ui-slint/ui/app.slint`) usa **layout declarativo nativo**
(grid, horizontal, vertical, flex) com constraints Slint (`preferred-width`,
`min-width`, `max-width`, `horizontal-stretch`, `vertical-stretch`). O Petunia
controla o grafo de regiões. Capacidade interna de tabs/tiles não implica docking
livre ao usuário.

> **Histórico (legado egui):** a stack anterior usava `egui_tiles` como baseline
> de macro-layout atrás de `PetuniaLayoutAdapter`. Essa crate permanece no workspace
> para retrocompatibilidade da UI legado, mas não recebe novas features. A
> responsividade no Slint é expressa em unidades lógicas e constraints declarativas;
> cálculo manual de larguras em Rust (`if available_width < N`) é proibido.

O Petunia controla o grafo de regiões. Capacidade interna de tabs/tiles não implica docking livre ao usuário.

Baseline:

```
Top Bar
└ Main Workspace
  ├ Left Region    → trilho de ferramentas do workspace + extension panels autorizados
  ├ Center Region  → Viewport / workspace editor
  ├ Right Region   → Inspector + extension panels autorizados; DRAW/POLY: Parts → Transform → Material → Object
  └ Bottom Region  → Asset Library + extension panels autorizados
```

Regras:

- core panels não podem ser removidos permanentemente por plugin;
- resize só existe em divisores autorizados;
- double-click em divisor restaura medida default;
- estado de layout é persistido por workspace;
- plugin panels entram apenas em **extension slots** controlados;
- nenhuma janela flutuante arbitrária na V1.

## Revisão de baseline MODEL — 2026-09-23

Esta revisão foi aprovada explicitamente pelo responsável do produto para resolver
a sobreposição da gaveta Parts com a barra de primitivas. A regra anterior de
`Parts` à esquerda fica substituída apenas no workspace MODEL: a barra esquerda
contém ações de criação, enquanto a lista Parts integra o Inspector direito.
O Inspector mantém cabeçalho fixo, corpo com rolagem vertical e seções que
recolhem sem reservar altura. Em larguras compactas, o acesso à lista pode usar
um drawer controlado à direita; ele não é uma segunda lista independente.
Os slots de extensão permanecem controlados e nenhum plugin pode deslocar as
seções centrais nem reintroduzir docking livre. PAINT e UV mantêm contratos
próprios até revisão explícita.

## Revisão de workspaces DRAW e POLY — 2026-09-29

Aprovada explicitamente pelo responsável do produto
([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md);
pesquisa no [capítulo 45](45-pesquisa-interacao-modelagem-referencias.md)).
O workspace MODEL é substituído por dois workspaces que compartilham documento,
seleção de objeto, câmera, snapping, gramática de ferramenta e Inspector:

- **DRAW** — nível de forma (perfis, regiões, volumes paramétricos, desenho sobre
  faces, Push/Pull). Seleção `Shape / Curve / Point / Region`.
- **POLY** — nível de componente. Seleção `Object / Face / Edge / Point`.

As medidas, regiões e regras da revisão de 2026-09-23 valem para os dois. O
trilho esquerdo passa a conter as ferramentas persistentes do workspace; o
Inspector é o mesmo. A conversão de forma (DRAW) para malha editável (POLY) é
explícita e reversível por Undo.

**Decisão aprovada, implementação pendente.** Até que DRAW e POLY estejam
funcionais, a UI mantém a pill MODEL, conforme a regra "workspace não
implementado não aparece como pill desabilitada". `Blender.svg` e o manual do
Blender deixam de ser referências canônicas desta baseline.

# Design System Final V1

## Tipografia

```
Caption      10
UI Small     11
UI Default   12
Panel/Strong 13
Exceptional  14
```

Nunca usar 8 px como baseline de texto interativo.

## UI scaling

Presets oficiais:

```
100%
125%
150%
175%
200%
```

A escala do sistema/HiDPI continua respeitada. Layout usa unidades lógicas.

## Radius

```
radius.control = 4
radius.segment = 5
radius.panel   = 8
radius.window  = 10
radius.pill    = full
```

## Accent

Baseline Petunia usa **violeta floral** em torno de `#B58CFF` como ponto inicial de implementação. Ajustes de luminância/contraste são tuning permitido sem mudar a semântica.

Accent significa principalmente:

```
selected
active
current
focus emphasis
```

Warning/error/success usam famílias semânticas próprias.

## Motion

- hover/menu: ~80–100 ms;
- collapse/panel: ~140 ms;
- transição estrutural: máximo ~180 ms;
- reduced-motion obrigatório;
- nenhuma animação contínua puramente decorativa.

## Tema oficial

Dark é o tema completo oficial da V1. High Contrast é variação oficial de acessibilidade. Light pode surgir depois sem alterar a arquitetura porque todos os estilos usam tokens semânticos.

# Theme Extension API

Usuários podem **criar, editar, importar, exportar e compartilhar temas** sem escrever plugin ou código executável.

Formato compartilhável:

```
my-theme.petunia-theme
├ theme.toml
├ tokens.json
├ preview.png        optional
├ fonts/             optional, subject to validation
└ icons/             optional, only supported theme-safe overrides
```

`.petunia-theme` é um ZIP versionado e **puramente declarativo**.

## Manifest conceitual

```toml
id = "community.midnight_petunia"
name = "Midnight Petunia"
version = "1.0.0"
petunia_theme_api_version = "1"
extends = "petunia.dark"
author = "..."
```

## Herança

Temas devem herdar de uma base:

```
petunia.dark
petunia.high_contrast
community.other_theme   optional when dependency is available
```

Resolver tokens em cadeia e detectar ciclos/dependências ausentes.

## Tokens extensíveis

Temas podem substituir tokens semânticos, incluindo:

- `surface.*`;
- `border.*`;
- `text.*`;
- `accent.*`;
- `selection.*`;
- `state.*`;
- `shadow.*`;
- `radius.*` dentro de ranges suportados;
- motion dentro dos limites de acessibilidade;
- typography family/weight quando o font resource for válido;
- icon mapping somente em slots genericamente tematizáveis.

## Tokens protegidos

Um theme **não pode** reduzir ou quebrar silenciosamente:

- minimum hit areas;
- focus semantics;
- accessible names/roles;
- keyboard behavior;
- layout topology do shell;
- tamanho mínimo do viewport;
- contraste mínimo exigido pelo modo High Contrast;
- security indicators e destructive-action semantics.

Spacing/metrics estruturais não são completamente livres: o usuário pode escolher profiles de densidade e overrides dentro de ranges validados, mas um theme não recebe poder para transformar o shell em um layout incompatível.

## Theme Manager

Settings → Appearance deve oferecer:

- theme picker;
- preview;
- `Create Theme from Current`;
- duplicate/rename;
- edit semantic tokens;
- import/export `.petunia-theme`;
- reset token;
- contrast/accessibility warnings;
- live preview/hot reload quando seguro.

Mudança de tema nunca altera documento `.petunia`.

# Temas e plugins

Um Community Plugin pode **bundlar** um `.petunia-theme` opcional ou registrar um theme package declarativo, mas não pode executar código durante resolução de tokens.

Preferência:

```
pure theme → .petunia-theme
functional extension + optional theme → .petunia-plugin containing theme package
```

# Plugin Panels — decisão V1

Community Plugins podem criar **novos painéis completos**, porém somente através da `Petunia UI Extension API`.

A regra anterior “sem UI nativa arbitrária” passa a significar:

> **Sem acesso cru a Slint, egui, wgpu, Painter, raw input, ponteiros ou markup
> arbitrário.** Plugins podem compor interfaces ricas usando componentes públicos
> e estáveis do Petunia, independentemente do frontend de produção.

# Panel Registry

Plugin registra um descriptor:

```
panel_id
plugin_id
title
icon
preferred_region
allowed_regions
minimum_size
preferred_size
singleton
visible_by_default
context_requirements
help/manual id
```

Regiões V1 permitidas:

```
left
right
bottom
```

`center/viewport replacement` e janelas flutuantes arbitrárias não fazem parte da Community Plugin API V1.

# Extension slots

Core regions fornecem slots controlados:

```
LEFT
  Trilho de ferramentas do workspace
  Plugin Panel A
  Plugin Panel B

RIGHT
  Inspector (Parts → Transform → Material → Object)
  Plugin Panel C

BOTTOM
  Asset Library
  Plugin Panel D
```

Painéis adicionais podem aparecer como tabs/stack controlados pelo Petunia. O usuário escolhe visibilidade em `View → Panels` ou Command Palette e pode mover um plugin panel somente entre suas regiões permitidas.

# Petunia UI Extension API

A API Lua expõe um builder seguro, conceitualmente:

```lua
petunia.ui.register_panel({
  id = "example.generator",
  title = "Roof Generator",
  preferred_region = "right",
  render = function(ui, context)
    ui:section("Shape", function(section)
      section:number_field("Width", state.width)
      section:number_field("Height", state.height)
      section:select("Style", state.style, styles)
      section:button("Generate", "example.generate_roof")
    end)
  end
})
```

A sintaxe concreta pode evoluir; o contrato semântico é normativo.

## Componentes públicos iniciais

Plugin panels podem compor, quando aplicável:

```
Text / Label / Heading
Icon / Image from packaged assets
Divider / Spacer
Row / Column
ScrollRegion
Section / CollapsibleSection
Button / IconButton / ToggleButton / SplitButton
SegmentedControl
TextField / SearchField / NumberField / Slider
Checkbox / Choice / Select
PropertyRow
List / Tree through bounded adapter
InlineMessage / Progress / EmptyState
Tooltip / HelpLink
CommandButton
```

Não expor custom `Painter` nem shader/GPU hooks pelo panel API V1. Viewport overlays continuam um extension point separado e permissionado.

# Event model de Plugin Panels

Plugin panel não deve varrer o Document inteiro a cada frame.

Fluxo preferido:

```
Document/selection event after commit
→ plugin subscription
→ plugin-scoped view state update
→ panel invalidation/repaint
→ render declarative Petunia Components
```

O host fornece `PanelContext` read-only com informações explicitamente permitidas. Mutação continua passando por Command/Application API.

# State de painel

UI state de plugin é namespaced por `plugin_id + panel_id`.

Pode persistir:

- open/closed;
- selected tab/filter;
- local form values quando autorizado;
- user preferences do panel;
- region/size dentro dos limites permitidos.

Nunca persiste raw handles internos como estado durável sem validação generacional.

# Capabilities de UI

Capabilities passam a ser mais granulares:

```
register_ui_panel
register_viewport_overlay
register_commands
read_selection
read_document
edit_geometry
edit_uv
edit_materials
import_files
export_files
filesystem
mcp_exposable
```

Theme package puramente declarativo não recebe capability de execução.

# Segurança e isolamento

- um Lua State por plugin;
- sem acesso a toolkit de UI (`egui::Ui`, `slint::ComponentHandle`), wgpu ou raw window handle;
- sem filesystem/networking sem capability;
- render errors ficam confinados ao plugin panel;
- repeated failures podem suspender apenas aquele panel/plugin;
- instruction/memory budgets continuam válidos durante callbacks de UI;
- nenhuma mutação do Document durante render; ações produzem Commands/events;
- unload remove panels, commands, subscriptions e state associado sem deixar referências quebradas.

# Acessibilidade de Plugin Panels

O host garante semantics dos componentes Petunia. O plugin deve fornecer textos de domínio adequados:

- panel title;
- accessible labels de campos icon-only/customizados;
- descriptions/help quando necessário;
- mensagens de erro compreensíveis.

Um plugin não pode desabilitar focus ring, keyboard activation ou semantics estruturais dos componentes públicos.

# Temas aplicados a Plugin Panels

Plugin panels **herdam automaticamente o tema atual**.

Plugin não escolhe RGB/hex local para componentes normais. Quando precisar expressar semântica, usa variants:

```
normal
accent
success
warning
danger
muted
```

Isso garante que um Community Plugin continue legível em Petunia Dark, High Contrast e temas criados pelo usuário.

# i18n de plugins

Plugins podem embutir bundles de tradução namespaced. Panel metadata e labels devem poder usar localization keys; fallback explícito para a língua do manifest.

# Crates UI — estado atual (2026-09-20)

**Frontend de produção:** `petunia_ui_slint` (Slint 1.18) em `crates/ui-slint/`.
Componentes declarativos em `ui/app.slint` (`TopAction`, `ToolButton`,
`NumericField`, `Vector3Field`, `InspectorSection`, `ColorSwatch`) são a
linguagem visual pública. Ícones via `lucide-slint` (`=1.47.0`).

**Legado de transição:** `crates/ui/` (egui 0.36) e `crates/app/` (host egui)
permanecem no workspace para retrocompatibilidade, acessíveis via
`--legacy-egui` / `PETUNIA_LEGACY_EGUI=1`. Não recebem novas features de
product UI. As crates egui especializadas abaixo são relevantes somente para
o legado:

- `egui_extras`, `egui_ltreeview`, `egui_tiles`, `egui_taffy`, `egui_dnd`,
  `egui_animation` → **LEGADO** (baseline da UI egui arquivada);
- `iconflow` com packs oficiais (`tabler`, `iconoir`, `phosphor`, `lucide`) +
  pack **Petunia** para conceitos 3D → usado no legado;
- `egui-file-dialog` → **LEGADO**; o Slint usa `rfd::AsyncFileDialog` em
  `files.rs`;
- `egui_kittest`, `egui_inspection`, `egui_mcp` → **LEGADO** (dev tools da
  UI egui).

Ferramentas de domínio como Extrude/Bevel/Cut/Fuse/Connect/UV usam ícones
próprios Petunia quando Lucide não representar semanticamente a operação.

# Input e navigation baseline

- LMB: clique sem movimento seleciona; arrasto além do limiar opera a ferramenta
  ativa (alça ou "haul"); clicar-mover-clicar é alternativa a todo arrasto;
- Shift+LMB: add/toggle selection;
- RMB: context menu — nunca cancela;
- MMB: orbit;
- Shift+MMB: pan;
- wheel/pinch: zoom — sempre, inclusive com ferramenta ativa; contagens de
  ferramenta usam `+`/`−` ou Ctrl+wheel;
- navegação nunca é suspensa por ferramenta;
- Esc: cancela em escada (arrasto → ferramenta → Select);
- Enter: confirm quando há operação pendente;
- F6 / Shift+F6: navegar regiões principais;
- Tab / Shift+Tab: navegar controles dentro da região;
- arrows: segmented/tree/navigation contextual;
- Space/Enter: ativação acessível do controle focado.

Click fora nunca confirma silenciosamente uma operação pendente de vários passos. Um gesto de arrasto concluído é uma transação desfazível, ajustável no card "Última operação" (constituição 11).

# Shortcuts

Preset default: **Petunia**.

Presets oficiais adicionais:

```
Blender
Blender — No Numpad
Maya
3ds Max
Cinema 4D
```

Remapping é orientado por `CommandId`, com busca, captura de teclas, detecção de conflito, reset por command/categoria/preset e import/export em JSON versionado.

# Workspaces finais V1

## DRAW

Viewport dominante + trilho de ferramentas de forma à esquerda (desenho de
perfis, Push/Pull, Revolve, primitivas, plano de trabalho) + Inspector direito
(`Parts → Transform → Material → Object`) + Asset Library inferior. Aparência do
viewport para leitura de forma (capítulo 05).

## POLY

Mesma estrutura de DRAW, com trilho de ferramentas de componente (Move/Rotate/
Scale, Extrude, Inset, Round Edge, Loop Cut, Cut, Poly Pen etc.) e aparência do
viewport para leitura de topologia (capítulo 05).

> Implementação (2026-09-30): o seletor do topo mostra `DRAW · POLY · PAINT · UV`.
> DRAW e POLY são modos do mesmo workspace de modelagem (mesmos documento,
> seleção, câmera, snapping e Inspector); muda o trilho de ferramentas. O
> conjunto de seleção `Shape / Curve / Point / Region` do DRAW e a aparência de
> viewport por modo (capítulo 05) ainda estão pendentes — ver
> [`draw-regions-gap-matrix.md`](../../development/draw-regions-gap-matrix.md).

## PAINT

Viewport 3D dominante; Context apresenta brush/material/texture. **Editor 2D de textura faz parte da V1 como painel/split opcional, fechado por padrão**, compartilhando a mesma TextureBitmap, palette, Pixel Grid e Undo/Redo do Paint 3D; não se transforma em editor de imagem generalista.

## UV

Split UV 2D + viewport 3D, default aproximado `55/45`, redimensionável e com seleção sincronizada.

Workspace não implementado não aparece como pill desabilitada.

### Adendo 2026-09-16 — iniciativa Paint (decisão registrada)

Durante a iniciativa Paint, a **pill UV sai temporariamente da UI V1**. O
workspace de edição UV é congelado porque o `uv_ui` alterna `f.selected`, a
mesma flag das máscaras de pintura (P3D-132), e o próprio P3D-063 exigia
design conjunto com Materials/Paint. O módulo `module-uv` **permanece** como
utilitário: projeções Planar, Box/Cúbica e Auto Unwrap (xatlas) são expostas
dentro do Paint como "Preparar superfície" (P3D-065, fluxo paint-first). O
redesign completo do workspace UV (P3D-063/064/065) fica marcado para o ciclo
**pós-Paint**. O shell `MODEL / PAINT` permanece; nenhum outro princípio deste
capítulo é reaberto pela decisão.

# Viewport adapter final

O viewport adapter é **toolkit-neutro**. `PetuniaRenderer` (`crates/render/`)
não conhece Slint nem egui; o adapter é a única fronteira que conhece ambos.

**Slint (produção):**

```
Slint TouchArea
→ viewport_gpu.rs (textura WGPU compartilhada → slint::Image)
→ PetuniaRenderer
→ wgpu
```

**egui (legado):**

```
egui layout
→ allocate viewport Rect/input/clip
→ PetuniaViewportAdapter
→ egui-wgpu custom callback / RenderPass
→ PetuniaRenderer
→ wgpu
```

Repaint é event-driven em idle e contínuo somente quando interação/motion/job
visual exigir.

V1 possui um viewport 3D principal por workspace. Multi-viewport/quad-view
fica para evolução posterior.

# Testes e conformance

UI pronta significa, no mínimo:

- component semantic/interaction tests;
- keyboard/focus tests;
- AccessKit tree checks;
- 100/150/200% visual snapshots;
- dark + high contrast;
- theme load/inheritance/invalid-token tests;
- plugin panel registration/unload/failure isolation tests;
- plugin panel accessibility tests;
- permissions/capability tests;
- layout persistence tests;
- screenshot visual regression;
- Windows + Linux conformance;
- viewport integration/HiDPI/popups/input tests.

Alteração de golden é mudança visual consciente, nunca atualização automática silenciosa.

# Tuning versus mudança arquitetural

Depois deste capítulo, mudanças como `248 → 256 px` ou pequeno ajuste de luminância são **tuning** quando mantêm contrato, hierarquia e acessibilidade.

Exigem nova decisão/ADR quando alterarem:

- toolkit principal;
- grafo estrutural do shell;
- exposição de docking irrestrito;
- raw UI/GPU access para community plugins;
- linguagem pública de plugins;
- invariantes de accessibility;
- escopo funcional do produto.

# Regra final

**Petunia deve ser personalizável sem ser fragmentável.** Usuários podem mudar a aparência profundamente e plugins podem acrescentar painéis e workflows, mas o aplicativo continua dono de seus componentes, acessibilidade, comandos, segurança e arquitetura de layout.
