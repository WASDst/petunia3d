# Slint Reassessment & GUI Architecture

> **Status: proposta recomendada; aguarda aprovação.**
>
> Este capítulo reabre **somente a escolha do toolkit/host da GUI**. As decisões já aprovadas de Application separada da UI e de renderer 3D OpenGL 3.3 permanecem válidas. Nenhuma alteração na arquitetura do renderer será feita antes da aprovação desta proposta.

## Conclusão executiva

Vale a pena insistir no Slint.

Mas não vale a pena continuar crescendo a implementação atual da mesma forma.

A recomendação é:

```text
SLINT
→ manter como candidato principal
→ preservar o grande investimento já existente
→ refatorar shell/bridge agressivamente
→ migrar viewport 3D para OpenGL
→ executar gates objetivos de UX/a11y/performance

EGUI
→ congelar
→ manter como fallback arquivado fora do branch ativo
→ nenhuma feature nova
→ documentação de restauração
```

Não manter dois frontends ativos em paralelo no longo prazo.

---

# Evidência do código atual

Slint já é o frontend padrão do binário:

```text
petunia3d
├ default → petunia_ui_slint
└ --legacy-egui → petunia_app / petunia_ui
```

A implementação Slint já contém:

- shell completo;
- top bar;
- workspaces;
- tool rail;
- viewport;
- Inspector;
- Parts;
- Asset Library;
- Command Palette;
- temas;
- i18n;
- UI scale;
- reduced motion;
- high contrast;
- OverlayStack;
- Reference Manager;
- Paint;
- UV;
- Animate experimental;
- split viewport;
- WGPU viewport;
- software fallback;
- centenas de testes.

Isso não é um protótipo descartável.

---

# O principal problema atual não é Slint

O problema é o crescimento monolítico.

Na branch atual:

```text
ui/app.slint        ≈ 708 KB
src/lib.rs          ≈ 694 KB
src/callbacks.rs    ≈ 254 KB
src/tests.rs        ≈ 452 KB
src/view_model.rs   ≈ 79 KB
```

Esses arquivos concentram responsabilidades demais.

A conclusão não deve ser:

```text
Slint não funciona
```

e sim:

```text
a boundary Slint ↔ Application ficou grande demais
+
o shell precisa ser decomposto
```

---

# O que já está bom e deve ser preservado

## Design tokens

Já existem:

- cores semânticas;
- axis colors;
- selection colors;
- surface hierarchy;
- motion tokens;
- spacing;
- control sizes;
- typography;
- reduced motion.

A arquitetura é correta.

A calibração visual precisa mudar.

## Componentes

Já existem componentes reutilizáveis reais:

- IconButton;
- Segmented;
- DropdownButton;
- PropertyRow;
- NumericField;
- Vector3Field;
- ToolButton;
- InspectorSection;
- RichTooltip;
- Search;
- sliders;
- paint canvas.

Não reimplementar isso do zero.

## Inspector accordion

`InspectorSection` já possui:

- expand/collapse;
- animação;
- keyboard activation;
- accessibility role;
- expanded state;
- pin;
- toggle-all;
- focus ring.

A ideia da captura do Unity de propriedades em accordion é, portanto, uma evolução natural do código atual.

## Accessibility foundation

Já existem:

- `accessible-role`;
- `accessible-label`;
- accessible state;
- FocusScope;
- Enter/Space activation;
- high contrast;
- reduced motion;
- UI scale.

Isso é uma vantagem importante para a filosofia do Petunia.

Mas a cobertura ainda é incompleta e deve virar gate, não promessa.

---

# Problemas que precisam ser corrigidos antes de apostar definitivamente no Slint

## 1. Monólitos

`app.slint`, `lib.rs` e `callbacks.rs` precisam ser quebrados por responsabilidade.

## 2. Acoplamento de domínio

`petunia_ui_slint` hoje depende diretamente de:

- petunia_core/AppState;
- petunia_mesh;
- petunia_project;
- module-model;
- module-paint;
- module-uv;
- render-wgpu.

Isso conflita com a nova arquitetura.

A UI deve depender principalmente de:

```text
Application Queries / View Models
Application Commands / Intents
neutral Render boundary
```

Não de Mesh ou módulos internos.

## 3. Viewport WGPU

A implementação Slint atual usa `petunia_render_wgpu`.

Isso **não** significa que manter Slint exige manter WGPU.

A decisão de renderer continua:

```text
Petunia Renderer
→ OpenGL 3.3 Core
```

Precisamos substituir somente o adapter do viewport.

## 4. Typography/accessibility

Tokens atuais ainda incluem 10 e 11 px.

A nova baseline deve ser:

```text
minimum ordinary UI text = 12 logical px
panel/strong             = 13
important                = 14
larger accessibility     = UI scale
```

10/11 px podem existir apenas em casos excepcionais não essenciais, se testes de legibilidade justificarem. A preferência é eliminar seu uso comum.

## 5. Keyboard regions

A documentação exige:

```text
F6 / Shift+F6
→ cycle major regions

Tab / Shift+Tab
→ controls inside current region
```

A implementação ainda não possui cobertura completa.

Isso entra como gate obrigatório.

## 6. Raw TouchArea

Controles ad-hoc precisam ser reduzidos.

Regra futura:

> Todo controle público interativo usa Petunia Component com foco, ação de teclado, role, name, state e hit target definidos.

---

# Slint + OpenGL

Manter Slint não reabre a decisão de usar WGPU.

Arquitetura alvo:

```text
DesktopHost / Slint winit backend
        │
        ├── Slint shell
        │
        └── current OpenGL context
                │
                ├── Petunia OpenGL Renderer
                │    └── viewport FBO texture
                │
                └── Slint renderer
                     └── displays borrowed GL texture
```

O viewport 3D continua sendo propriedade do Petunia Renderer.

Slint apenas apresenta sua textura e encaminha input.

## Regra

Nunca:

```text
OpenGL viewport
→ glReadPixels every frame
→ CPU image
→ Slint
```

O gate exige compartilhamento de textura GPU/contexto ou caminho equivalente sem readback por frame.

## Render order

Direção:

```text
Slint rendering notifier / current GL context
→ update Petunia viewport FBO when dirty
→ expose GL texture to Slint Image
→ Slint composites shell
→ present
```

O protótipo deve validar state restoration entre Petunia GL e Slint/FemtoVG.

---

# Leitura das referências visuais enviadas

A captura do Unity é usada como **referência de hierarquia e densidade**, não como skin para copiar.

A interpretação aprovada para discussão é:

```text
┌──────────────────── Header ──────────────────────────┐
│ menus/project       workspace/context      global    │
├──────┬──────────────────────────────┬────────────────┤
│ Tool │                              │ Outliner/Parts │
│ Rail │                              ├────────────────┤
│      │          VIEWPORT            │ Properties     │
│      │                              │                │
│      │                              │ Transform   ▾  │
│      │                              │ Geometry    ▾  │
│      │    Context Action Bar        │ Material    ▾  │
│      │                              │ Modifiers   ▾  │
├──────┴──────────────────────────────┴────────────────┤
│ Assets / Prefabs drawer                             │
└──────────────────────────────────────────────────────┘
```

---

# Header

Não copiar os botões Play/Pause do Unity.

O estilo pode inspirar:

- baixa altura;
- poucos níveis visuais;
- distribuição clara;
- agrupamentos sem excesso de cards.

Conteúdo Petunia:

## Left

```text
Petunia / File / Edit / View
Project Name
Save State
```

## Center

Contexto de trabalho de alto nível.

Não sobrecarregar com controles do viewport.

## Right

```text
Undo / Redo
Command Search
optional validation/performance indicator
Preferences
```

Shading, camera, overlays, snapping e pivot continuam na barra contextual da viewport, não no header global.

---

# Left Tool Rail

Manter a ideia atual.

```text
Select
Transform
Shape tools
Poly tools
Create
Paint tools according to workspace
```

Regras:

- 40–48 logical px compacta;
- icon + tooltip;
- modo opcional com labels;
- agrupamento por divisores;
- flyout para famílias;
- ferramenta ativa clara;
- não duplicar ações da contextual bar.

A captura do Unity não deve nos empurrar para ícones minúsculos sem rótulos.

---

# Viewport

A viewport permanece o centro da aplicação.

Manter:

- overlays;
- gizmos;
- Reference;
- Workplane;
- HUD;
- contextual interaction;
- futura OpenGL architecture.

A barra flutuante contextual inferior permanece.

Ela deve mostrar somente ações válidas para:

```text
current selection
+
active tool/session
```

Máximo recomendado:

```text
5–6 primary actions
+ More
```

---

# Right Column

A principal mudança da referência enviada é boa e deve ser considerada.

## Upper right: Outliner / Parts

Mover Parts/Outliner para uma região própria acima das propriedades.

Isso reabre a antiga decisão de colocar Parts como primeira seção do Inspector.

A nova proposta é:

```text
Right Column
├ Scene / Parts
└ Properties
```

Vantagens:

- hierarquia sempre visível;
- seleção e properties ficam espacialmente relacionadas;
- reduz rolagem do Inspector;
- aproxima workflow de editores profissionais sem copiar o Unity;
- libera Inspector para propriedades reais.

## Outliner controls

Header compacto:

```text
Scene / Parts         Search
Filter       Sort      More
```

Rows:

- disclosure;
- icon/type;
- name;
- optional color tag;
- visibility;
- lock;
- state badges somente quando necessários.

Search por:

- name;
- type;
- tag;
- collection.

---

# Properties abaixo

Visual limpo, uma superfície contínua.

Sem stack de cards independentes.

## Fixed/common sections

Quando aplicável:

```text
Transform
Geometry Source
Material
```

`Transform` fica próximo do topo e é persistente para SceneObject.

## Contextual sections

Exemplos:

```text
Modifiers
Generator
Paint
Reference Projection
UV
Rig
Animation
Plugin Properties
```

Só aparecem quando relevantes.

## Accordion

Cada section possui:

- entire header clickable;
- chevron;
- title;
- optional summary;
- optional enabled toggle;
- optional menu;
- keyboard focus;
- accessible expanded state;
- persisted UI open/closed state.

Não usar cor como única indicação.

---

# "Components" sem ECS

A ideia da imagem é útil, mas não precisamos chamar o conceito de Components nem adotar ECS.

Arquitetura:

```text
PropertySectionRegistry
├ Built-in section descriptors
└ Plugin section descriptors
```

Conceitualmente:

```rust
PropertySectionDescriptor {
    id,
    title,
    icon,
    visibility_query,
    fields/actions,
    order,
}
```

A Application fornece DTOs e commands.

Slint renderiza usando apenas Petunia Components.

Plugins **não recebem raw Slint UI access**.

Isso permite um botão como:

```text
+ Add
  ├ Modifier
  ├ Material Feature
  ├ Reference Projection
  └ Plugin Extension
```

sem ECS e sem um segundo framework de propriedades.

Nome público recomendado:

```text
Properties / Features
```

e não necessariamente "Components".

---

# Asset / Prefab drawer

Manter a gaveta inferior retrátil como na anotação.

Header:

```text
Assets / Prefabs
[Search................] [Type] [Tags] [Favorites] [⋯]
```

Filtros:

- name;
- type;
- tags;
- color tags;
- favorites;
- collection/source.

Views:

```text
Grid
List
```

Rows/cards devem ser virtualizados quando necessário.

Drag:

```text
Asset / Prefab
→ viewport
→ placement preview
→ click
→ SceneObject
```

Não usar Asset Library como filesystem explorer genérico.

---

# Split Viewports

A feature é viável e já possui base no Slint atual.

Hoje:

- existe segunda câmera;
- documento é compartilhado;
- secondary view é cached;
- secondary view é atualmente software;
- edição fica apenas na main view.

A nova arquitetura OpenGL melhora isso.

## V1

Suportar:

```text
Single
Split Vertical
Split Horizontal
```

No máximo 2 views inicialmente.

Cada view:

- própria Camera;
- próprio viewport rect;
- próprio FBO color/depth;
- mesmo scene/render cache;
- mesma Selection/Document;
- ViewId explícito em input.

## GPU sharing

```text
SceneObject / RenderMesh cache
        ↓ once
shared VBO / IBO / textures
        ├ View A camera → FBO A
        └ View B camera → FBO B
```

Não duplicar scene buffers por view.

## Scheduling

Active view:

```text
interactive → full update rate
```

Passive view:

```text
nothing changed → zero render
scene changed → one render
active gesture → freeze or throttle
gesture ends → final refresh
```

Recomendação de primeira implementação:

```text
during heavy drag/paint
→ passive view max 10 Hz or frozen

on release
→ immediate final render
```

## Active viewport

Click numa viewport:

```text
ViewId becomes active
```

Depois disso navigation/tool rays usam sua Camera.

Isso é melhor que manter a segunda view permanentemente read-only.

A primeira etapa pode liberar apenas navigation + selection antes de liberar todas as tools, se necessário.

## Performance rule

Split é feature opcional.

Se hardware tier baixo ou viewport pequeno:

- reduzir MSAA/passive quality;
- reduzir passive render resolution temporariamente;
- nunca reduzir qualidade final/export;
- não duplicar geometry evaluation.

Quad view continua fora do primeiro slice.

---

# Responsividade

O macro-layout deve ser fixed-but-adaptive, não docking irrestrito.

Desktop largo:

```text
Tool Rail | Viewport | Right Column
                    | Outliner
                    | Properties
Assets Drawer
```

Janela estreita:

1. Asset drawer recolhe;
2. Outliner pode recolher para header/tab;
3. Properties vira right drawer;
4. Tool rail continua compacta;
5. viewport mantém prioridade.

Não esconder funcionalidades sem uma rota acessível para reabri-las.

---

# Acessibilidade — novo hard gate

Slint permanece candidato principal **porque** a fundação pode ser aproveitada, mas só será confirmado após esta matriz.

## Typography

- texto operacional >= 12 logical px;
- UI scale 100/125/150/175/200;
- fonte clara e legível;
- sem truncar labels essenciais.

## Pointer targets

- mínimo operacional 28 px;
- preferir 32 px para ações recorrentes;
- áreas de hit podem ser maiores que a geometria visual;
- gizmos seguem regra própria de hit target.

## Keyboard

Obrigatório:

```text
F6 / Shift+F6
→ Top Header
→ Tool Rail
→ Viewport
→ Outliner
→ Properties
→ Assets
→ Context Bar

Tab / Shift+Tab
→ dentro da região

Enter / Space
→ activate

Esc
→ close/cancel topmost context
→ restore previous focus
```

## Screen reader semantics

Toda ação interativa precisa fornecer:

- role;
- name;
- state;
- value quando aplicável;
- default action;
- focus order coerente.

## Color

Nenhum estado crítico depende somente de cor.

Exemplos:

```text
selected
→ color + outline/icon/state

axis
→ color + X/Y/Z

error
→ color + icon + text
```

## Reduced motion

Mantém o sistema atual.

Animação decorativa pode ser removida.

Feedback funcional de tool/animation timeline não é escondido.

---

# Refatoração estrutural do Slint

Não fazer rewrite big-bang.

## Markup alvo

```text
ui/
├ app.slint                 # composition only
├ shell/
│  ├ header.slint
│  ├ tool_rail.slint
│  ├ right_column.slint
│  ├ asset_drawer.slint
│  └ status_bar.slint
├ viewport/
│  ├ viewport.slint
│  ├ view_bar.slint
│  ├ context_bar.slint
│  └ overlays.slint
├ panels/
│  ├ outliner.slint
│  ├ properties.slint
│  ├ material.slint
│  └ ...
├ components/
├ dialogs/
├ tokens.slint
└ types.slint
```

`app.slint` deve voltar a ser composição, não implementação de todo o produto.

## Rust bridge alvo

```text
src/
├ lib.rs                    # bootstrap/public API only
├ bridge/
│  ├ application.rs
│  ├ commands.rs
│  ├ viewport.rs
│  ├ outliner.rs
│  ├ properties.rs
│  ├ assets.rs
│  └ accessibility.rs
├ view_model/
├ viewport/
└ tests/
```

Evitar outro arquivo central de centenas de KB.

---

# Boundary da UI

Direção:

```text
Slint
↓ UiIntent / View DTO
Application
↓ Commands / Queries
Document / Geometry
```

Proibido no destino:

```text
Slint component
→ mutate Mesh directly
```

ou:

```text
UI bridge
→ call module internals because convenient
```

---

# Egui como fallback congelado

A ideia é recomendada.

Mas a forma correta não é manter egui em evolução dentro do mesmo branch.

## Estratégia

Quando esta proposta for aprovada:

1. criar branch/tag de preservação do último estado egui utilizável;
2. registrar SHA exato;
3. registrar como executar;
4. registrar features existentes e limitações;
5. congelar: sem features novas;
6. manter somente fixes indispensáveis enquanto o gate Slint não terminou;
7. depois do gate Slint, retirar egui do build graph do branch principal;
8. se Slint fracassar estruturalmente, restaurar a partir do snapshot.

Nome sugerido:

```text
archive/egui-fallback-2026-10
```

O nome exato não é importante; o commit é.

## Não duplicar código na documentação

Git já é o arquivo.

A documentação guarda:

- branch/tag;
- commit;
- setup;
- build command;
- feature matrix;
- known gaps;
- passos para reativar.

---

# Go / No-Go do Slint

Slint só se torna decisão definitiva depois de passar um milestone de resgate.

## Gate 1 — OpenGL viewport

- Petunia OpenGL renderer funciona no shell Slint;
- mesma textura GPU chega ao shell sem readback por frame;
- resize/HiDPI correto;
- picking coordinates corretas;
- Render-on-demand preservado.

## Gate 2 — performance

Em hardware de referência low-end:

- UI idle praticamente sem trabalho contínuo;
- active viewport responsivo;
- no runaway allocations;
- resize não trava;
- Split2 compartilha scene buffers;
- passive viewport não rouba frame budget durante gestures.

## Gate 3 — accessibility

- keyboard-only para fluxo principal;
- F6 regions;
- focus restore;
- screen-reader tree auditada;
- high contrast;
- reduced motion;
- 100–200% scale;
- ordinary text >= 12px;
- critical state never color-only.

## Gate 4 — shell/UX

- header;
- tool rail;
- viewport;
- context bar;
- Outliner/Parts;
- Properties;
- Asset/Prefab drawer;
- command search;
- menus/dialogs.

Sem overflow em tamanhos suportados.

## Gate 5 — authoring critical path

Pelo menos:

```text
create object
select
transform
draw shape
Push/Pull
undo/redo
save/load
material
reference image
project photo basic path
```

deve estar utilizável no Slint antes de remover egui do branch ativo.

---

# Critérios de abandono do Slint

Não trocar toolkit por preferência estética.

Trocar apenas se o milestone provar um bloqueador estrutural, por exemplo:

- integração OpenGL confiável impossível na matriz Windows/Linux alvo;
- input/focus/accessibility incapaz de cumprir os contratos;
- custo/performance de composição inviável no hardware mínimo;
- bugs de toolkit reproduzíveis sem workaround aceitável;
- bridge continuar exigindo complexidade significativamente maior mesmo após modularização.

"Visual ainda precisa de polish" não é critério de abandono.

---

# Decisões recomendadas

1. Reabrir a antiga escolha egui como frontend final.
2. Slint volta a ser o **candidato principal recomendado**.
3. OpenGL 3.3 continua sendo o renderer 3D final; WGPU não volta como requisito.
4. Criar um adapter Slint + OpenGL em vez de manter o viewport WGPU.
5. Fazer um milestone de resgate antes de declarar Slint definitivo.
6. Não adicionar novas features ao egui.
7. Criar snapshot branch/tag do egui após aprovação desta decisão.
8. Após gates Slint, retirar egui do build graph ativo.
9. Reorganizar o shell seguindo a macro-hierarquia da referência Unity sem copiar sua estética.
10. Parts/Outliner ganha região própria no topo da coluna direita.
11. Properties fica abaixo com accordions.
12. Transform permanece seção comum; outras seções são contextuais.
13. Implementar PropertySectionRegistry/descriptor, sem ECS.
14. Asset/Prefab drawer inferior é preservado e melhorado com search/tags/favorites.
15. Contextual bottom bar permanece na viewport.
16. Split2 vertical/horizontal entra como feature oficial; shared GPU resources + render-on-demand.
17. Quad view permanece fora do primeiro slice.
18. Slint markup e Rust bridge serão decompostos incrementalmente, não reescritos.
19. Accessibility passa a ser hard gate, com texto >= 12px e region keyboard navigation.
20. Só abandonar Slint por bloqueador estrutural medido, usando egui arquivado como fallback.

## Regra anti-rewrite

```text
Preserve Slint components, flows and tests
→ simplify layout
→ decouple bridge
→ replace viewport adapter
→ validate
```

e não:

```text
delete Slint
→ rebuild every panel in another toolkit
```
