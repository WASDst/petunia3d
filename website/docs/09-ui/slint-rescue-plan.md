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

`app.slint` caiu de aproximadamente 10.774 para aproximadamente 10.332 linhas sem remover feature de domínio.

### Próximas extrações desta fase

1. Right Column shell;
2. Workspace Drawer;
3. status/context shell;
4. overlays de alto nível;
5. depois Viewport shell.

A viewport é deliberadamente posterior porque concentra input, picking, overlays e tool sessions.

## 6. Fase 2 — Right Column

Separar:
- Structure;
- Properties;
- comportamento responsive/collapse;
- split vertical interno.

O shell não deve saber o conteúdo detalhado de cada Property Section.

Meta:

```text
RightColumn
├ StructureHost
└ PropertiesHost
```

Conteúdo por workspace é projetado por view-model/registry.

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

Concluído nesta execução:
- `ShellHeader`;
- `ToolRail`;
- `StatusToast`;
- documentação do diretório atualizada.

Próximo alvo: **Right Column / Structure + Properties**, começando pela extração do container sem mover ainda o conteúdo dos workspaces.
