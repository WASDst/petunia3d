# Interface e Design System — Petunia3D (`docs/ui/`)

Documentação da **UI Baseline Final V1**, congelada no capítulo 36 do Livro Vivo
([`docs/bible/foundations/36-ui-baseline-temas-plugin-panels.md`](../bible/foundations/36-ui-baseline-temas-plugin-panels.md)).

> **Regra de leitura:** o capítulo 36 é a autoridade. Calibrações pequenas que
> preservam contrato, hierarquia e acessibilidade são **tuning**. Mudanças de
> toolkit, grafo do shell, docking irrestrito, acesso cru de plugins a UI/GPU e
> invariantes de acessibilidade exigem decisão explícita/ADR.

**Frontend de produção (2026-09-20):** `petunia_ui_slint` (Slint 1.18) em
`crates/ui-slint/`. O binário `petunia3d` executa o shell Slint por padrão; a
UI egui (`crates/ui/`) é legado de transição acessível via `--legacy-egui` /
`PETUNIA_LEGACY_EGUI=1`. Os contratos de UX dos capítulos 23 e 36 são
toolkit-neutros — a migração para Slint os preserva.

## 1. Modelo mental do usuário

```
REFERENCE → DRAW/CREATE → SHAPE → PAINT/PROJECT → CHECK → EXPORT
```

Topologia, triangulação e UV continuam acessíveis como escape hatch e aprendizado
progressivo — nunca como pré-requisito para o primeiro asset.

## 2. Princípios congelados

- **viewport-first**: o viewport mantém ~`480 × 360` logical px antes de ceder espaço a painéis;
- shell profissional simplificado — nunca "Blender amputado" nem aplicativo infantil;
- `Parts` à esquerda, `Context` à direita, `Asset Library` inferior;
- painéis semi-flutuantes, retráteis e redimensionáveis dentro de limites explícitos;
- `MODEL / PAINT / UV` como workspaces V1;
- toolbar contextual dentro do viewport;
- seleção explícita `Object / Face / Edge / Point`;
- `Wireframe / Solid / Textured / Silhouette` como modos-base; overlays são composição sobre o modo-base;
- Command Registry como fonte única de ações, shortcuts, plugins e command palette;
- Petunia Components como única linguagem visual pública;
- AccessKit + keyboard/focus fazem parte do contrato, não são acabamento tardio;
- **nenhum docking irrestrito na V1**.

## 3. Shell e medidas de referência

Valores em **logical px**, antes de UI scaling.

| Elemento | Baseline | Regra |
| :--- | :--- | :--- |
| Top bar | `40` | menus/projeto à esquerda, workspace pills ao centro, ações globais à direita |
| Parts | `248` default; `200–400` | recolhível, resize horizontal |
| Context | `288` default; `240–440` | selection/tool-centric |
| Asset Library | `176` default; `120–360` | resize vertical + collapse |
| Panel Header | `28` | um contrato único |
| Control | `28` | 30–32 apenas quando a hierarquia justificar |
| Status Strip | `~22` | hints contextuais + stats/save/validation |
| Gutter estrutural | `8` | `4` somente entre controles intimamente relacionados |

Breakpoints desktop: `≥ 1280` shell completo · `1024–1279` Asset Library inicia
recolhida · `< 1024` Parts/Context podem atuar como drawers/overlays temporários.

Estrutura de regiões:

```
Top Bar
└ Main Workspace
  ├ Left Region    → Parts + extension panels autorizados
  ├ Center Region  → Viewport / workspace editor
  ├ Right Region   → Context + extension panels autorizados
  └ Bottom Region  → Asset Library + extension panels autorizados
```

Regras: core panels não podem ser removidos permanentemente por plugin · resize
só em divisores autorizados · double-click no divisor restaura a medida default ·
estado de layout é persistido por workspace · plugin panels entram apenas em
**extension slots** controlados · nenhuma janela flutuante arbitrária na V1.

## 4. Design System Final V1

**Tipografia:** `Caption 10` · `UI Small 11` · `UI Default 12` · `Panel/Strong 13` · `Exceptional 14`.
Nunca usar 8 px como baseline de texto interativo.

**UI scaling:** presets `100% / 125% / 150% / 175% / 200%`; escala do sistema/HiDPI respeitada; layout em unidades lógicas.

**Radius:** `control 4` · `segment 5` · `panel 8` · `window 10` · `pill full`.

**Accent:** violeta floral em torno de `#B58CFF` como ponto inicial. Accent significa
`selected`, `active`, `current`, `focus emphasis`; warning/error/success têm famílias semânticas próprias.

**Motion:** hover/menu ~`80–100 ms` · collapse/panel ~`140 ms` · transição estrutural
máx. ~`180 ms` · **reduced-motion obrigatório** · nenhuma animação contínua decorativa.

**Tema oficial:** **Dark** é o tema completo da V1. **High Contrast** é variação
oficial de acessibilidade. Light pode surgir depois sem mudar arquitetura, porque
todo estilo usa tokens semânticos.

Todos os valores acima são `ThemeToken` — nunca cor, espaçamento ou fonte hardcoded
em widget.

## 5. Temas declarativos (Theme Extension API)

Usuários criam, editam, importam, exportam e compartilham temas **sem escrever código**:

```
my-theme.petunia-theme
├ theme.toml
├ tokens.json
├ preview.png        opcional
├ fonts/             opcional, sujeito a validação
└ icons/             opcional, somente overrides theme-safe suportados
```

O `.petunia-theme` é um ZIP versionado **puramente declarativo** (não executa código
durante a resolução de tokens) e herda de uma base (`petunia.dark`,
`petunia.high_contrast` ou outro tema, quando disponível), com detecção de ciclos e
dependências ausentes.

**Tokens protegidos** — um tema não pode reduzir silenciosamente: minimum hit areas,
semântica de foco, accessible names/roles, comportamento de teclado, topologia do
shell, tamanho mínimo do viewport, contraste mínimo do modo High Contrast, indicadores
de segurança e semântica de ações destrutivas.

**Theme Manager** (`Settings → Appearance`): theme picker, preview, *Create Theme from
Current*, duplicate/rename, edição de tokens semânticos, import/export
`.petunia-theme`, reset de token, avisos de contraste/acessibilidade e live preview.
Trocar de tema **nunca** altera o documento `.petunia`.

## 6. Plugin Panels

Community Plugins Lua podem registrar **painéis completos**, somente pela
`Petunia UI Extension API`: sem acesso cru a `egui`, `wgpu`, `Painter`, raw input,
ponteiros ou markup arbitrário.

- **Panel Registry**: `panel_id`, `plugin_id`, `title`, `icon`, `preferred_region`,
  `allowed_regions`, `minimum_size`, `preferred_size`, `singleton`,
  `visible_by_default`, `context_requirements`, `help/manual id`.
- **Regiões V1**: `left`, `right`, `bottom`. Substituir o viewport/centro e janelas
  flutuantes arbitrárias **não** fazem parte da Community Plugin API V1.
- **Event model**: o painel não varre o documento a cada frame; ele reage a eventos
  após commit, atualiza seu estado e invalida o repaint.
- **Capabilities**: `register_ui_panel`, `register_viewport_overlay`, `register_commands`,
  `read_selection`, `read_document`, `edit_geometry`, `edit_uv`, `edit_materials`,
  `import_files`, `export_files`, `filesystem`, `mcp_exposable`.
- **Isolamento**: um Lua State por plugin, erros confinados ao painel, sem mutação do
  documento durante render, unload remove painéis/commands/subscriptions.
- **Acessibilidade e tema**: o host garante semantics, foco e herança automática do
  tema atual. Plugins usam variants (`normal`, `accent`, `success`, `warning`,
  `danger`, `muted`) — nunca RGB/hex local.

## 7. Input, foco e acessibilidade

| Entrada | Ação |
| :--- | :--- |
| `LMB` | selecionar/operar |
| `Shift + LMB` | adicionar/alternar seleção |
| `RMB` | context menu |
| `MMB` | orbit |
| `Shift + MMB` | pan |
| wheel/pinch | zoom |
| `Esc` | cancelar |
| `Enter` | confirmar operação pendente |
| `F6` / `Shift+F6` | navegar regiões principais |
| `Tab` / `Shift+Tab` | navegar controles dentro da região |
| setas | segmented/tree/navegação contextual |
| `Space`/`Enter` | ativação acessível do controle focado |

Clicar fora **nunca** confirma silenciosamente operação destrutiva. Foco visível,
reduced-motion e semantics AccessKit são contrato, não polish.

## 8. Temas, ícones e presets (estado do build)

O contrato do caderno define: **Dark oficial** + **High Contrast** de acessibilidade;
temas adicionais entram por `.petunia-theme`; e os pack oficiais de ícones são
**Tabler, Iconoir, Phosphor, Lucide** mais **Petunia Custom Icons** para conceitos 3D
sem representação adequada (P3D-087, P3D-088).

> **Divergência registrada:** o build atual embarca temas extras (Light, Capuccino,
> Tokyo Nights), um pack `future-dark` e um pack Petunia cuja linguagem foi derivada
> de referência externa. Isso é `FUNCTIONAL_BUT_DIFFERENT` em relação ao capítulo 36 e
> está registrado na [auditoria de conformidade](../audits/bible-conformance/README.md);
> os valores válidos continuam sendo os tokens semânticos, não as paletas específicas.

Presets de keymap: default **Petunia**, mais `Petunia Simple`, `Petunia Notebook`,
`Blender-like`, `Blender-like Notebook`, `Maya-like`, `3ds Max-like` e `Cinema 4D-like`.
Remapping é orientado por `CommandId`, com busca, captura de teclas, detecção de
conflito, reset por command/categoria/preset e import/export em JSON versionado.

## 9. Workspaces V1

- **MODEL** — viewport dominante + Parts + Context + Asset Library.
- **PAINT** — viewport 3D dominante; Context apresenta brush/material/texture. O
  editor 2D de textura faz parte da V1 como painel/split **opcional, fechado por
  padrão**, compartilhando a mesma TextureBitmap, palette, Pixel Grid e Undo/Redo do
  Paint 3D — não é um editor de imagem generalista.
- **UV** — split 2D + viewport 3D, default aproximado `55/45`, redimensionável e com
  seleção sincronizada.

Workspace não implementado **não aparece** como pill desabilitada.

## 10. Viewport adapter

O viewport adapter é toolkit-neutro. `PetuniaRenderer` (`crates/render/`) não
conhece Slint nem egui; o adapter é a única fronteira que conhece ambos.

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

Repaint é event-driven em idle e contínuo somente quando
interação/motion/job visual exigir. A V1 tem **um** viewport 3D principal por
workspace; multi-viewport/quad-view é evolução posterior.

## 11. Testes e conformance da UI

UI pronta significa, no mínimo: component semantic/interaction tests · keyboard/focus
tests · AccessKit tree checks · snapshots visuais em `100/150/200%` · dark + high
contrast · theme load/inheritance/invalid-token tests · plugin panel
registration/unload/failure isolation · plugin panel accessibility · permissions/capability
· layout persistence · screenshot visual regression · conformance Windows + Linux ·
viewport integration/HiDPI/popups/input.

Alteração de golden é mudança visual consciente — **nunca** atualização automática silenciosa.

## 12. Referências visuais arquivadas (não normativas)

`docs/image-references/` guarda mockups e capturas usados durante a fase de pesquisa
(incluindo `Blender.svg` e os elementos vetoriais extraídos). Conforme o contrato
documental, esses artefatos são **`FIGMA-CONFIRMED`: evidência visual observada, não
requisito do produto**.

> Mockup **não** constitui prova de implementação (P3D-118). Composição espacial,
> menus e catálogo de ícones do Blender **não** definem a baseline — o capítulo 36
> define. A lista de menus de um DCC genérico (Sculpting, Geometry Nodes, Compositing,
> Particles, Physics, Constraints) está explicitamente fora do produto-base.
