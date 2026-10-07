# Renderer e Viewport Pipeline

> **Status: proposta recomendada; aguarda aprovação.**

## Decisão já fechada

O alvo do Petunia3D é:

```text
winit + glutin + glow/OpenGL + egui + egui_glow
```

Baseline:

```text
OpenGL 3.3 Core
```

Extensões GL 4.x podem ser usadas somente por capability gate e nunca como requisito funcional.

Não manter WGPU como backend alternativo no destino final.

## Diagnóstico atual

Hoje existem três fontes de comportamento:

### render-gl

- OpenGL 3.3 funcional;
- bootstrap glutin;
- mesh/textures/reference images/grid/edges básicos;
- VBOs persistentes;
- recebe `&AppState` inteiro;
- consulta Project, Camera, Theme, Selection e flags diretamente;
- Smooth está fixo como `false` em caminho crítico;
- várias features visuais maduras do WGPU não possuem paridade.

### render-wgpu

É hoje a implementação mais rica de viewport:

- MSAA;
- linhas de largura constante;
- EdgeMode DRAW/POLY;
- selection overlays;
- outline de objeto em pixels;
- workplane overlay;
- screen-space gizmo pass;
- Matcap;
- GTAO/ambient occlusion;
- texture dirty updates;
- revisioned buffers;
- X-Ray;
- reference images;
- pixel tests.

Essas ideias devem ser preservadas, mas não o backend WGPU.

### petunia-render

Hoje é quase apenas tipos compartilhados. No destino ele se torna o renderer OpenGL concreto.

## Estratégia de migração

Não apagar WGPU no início.

Usá-lo como **oráculo de comportamento visual temporário** enquanto cada feature é portada para o novo OpenGL renderer.

```text
define neutral render boundary
        ↓
port OpenGL baseline
        ↓
port visual features from WGPU
        ↓
pixel/behavior parity gates
        ↓
remove WGPU
```

WGPU não recebe features novas durante a migração, exceto correções necessárias para manter o branch utilizável.

## Boundary principal

O renderer não recebe:

- AppState;
- Document;
- EditorSession;
- SelectionState;
- egui Context;
- filesystem;
- preferences store.

Ele recebe somente um snapshot neutro preparado para renderização.

Direção:

```rust
pub struct RenderFrameInput<'a> {
    pub camera: CameraMatrices,
    pub viewport: PhysicalViewport,
    pub settings: ViewportRenderSettings,
    pub objects: &'a [RenderObject],
    pub overlays: &'a RenderOverlays,
    pub references: &'a [RenderReferenceImage],
}
```

O tipo exato pode variar; a regra de dependência não.

## Quem prepara RenderFrameInput

`Application/Editor query layer` resolve:

```text
SceneObject
→ Geometry Evaluation Service
→ modifiers
→ pose/skin
→ SurfaceShading + Sharp
→ corner normals
→ transforms/material references
→ RenderObject
```

O renderer apenas consome.

Não chamar `asset.evaluated_mesh()` de dentro do renderer.

## RenderObject

Direção:

```rust
pub struct RenderObject {
    pub object_id: ObjectId,
    pub transform: Mat4,
    pub mesh: RenderMeshHandleOrData,
    pub material: RenderMaterialRef,
    pub visible: bool,
    pub selected: bool,
    pub active: bool,
}
```

Selection é reduzida a flags/overlay data necessários para desenho; o renderer não conhece `SelectionState`.

## RenderMesh

Boundary já fechada em Geometry:

```rust
pub struct RenderVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub texture_coordinates: [f32; 2],
}

pub struct RenderMesh {
    pub vertices: Vec<RenderVertex>,
    pub indices: Vec<u32>,
}
```

RenderMesh é derivado e não persistente.

Ele já incorpora:

- triangulação canônica;
- FaceCorner UV;
- corner normals;
- vertex color;
- splits necessários de render.

## Mesh GPU cache

Não usar fingerprint integral da cena.

Cache por objeto/mesh:

```text
ObjectId
+ render-geometry revision
+ shading revision
+ material binding revision
```

Câmera nunca invalida vertex/index buffers.

Mudança de seleção nunca reconstrói a mesh GPU.

## GPU resources

Manter por objeto:

- VBO;
- IBO;
- vertex/index counts;
- last uploaded revision.

Prune por ObjectId removido.

`glBufferSubData`/orphaning pode ser usado quando profiling justificar; começar simples e correto.

## Texture cache

Texturas usam IDs/revisions estáveis.

Dirty rectangles de Paint devem ser preservados da infraestrutura WGPU atual.

```text
TextureId/ObjectId
+ texture revision
+ optional dirty rects
```

Não hashear pixels inteiros por frame.

## Context ownership

DesktopHost possui:

- Window;
- glutin context/surface;
- swap/present;
- resize;
- DPI;
- OpenGL capability detection;
- egui integration.

`petunia-render` recebe um contexto OpenGL válido e gerencia apenas recursos/passes gráficos.

O atual `render-gl/bootstrap.rs` deve migrar conceitualmente para DesktopHost.

## Um único contexto OpenGL

Viewport 3D e egui usam o mesmo contexto.

Ordem:

```text
begin frame
→ render viewport 3D
→ render viewport overlays GPU
→ egui paint
→ present
```

O renderer restaura apenas os estados GL que realmente compartilha com egui; evitar reset global caro e implícito.

## Render passes V1

Pipeline recomendado:

```text
1. clear / depth
2. reference images behind
3. optional depth prepass
4. opaque surfaces
5. x-ray/translucent surfaces
6. topology/feature edges
7. selection/component overlays
8. workplane/grid/guides
9. selection outline
10. reference images x-ray/front
11. screen-space gizmo/tool overlays
```

Nem todo passo precisa emitir draw call se estiver vazio.

## MSAA

Preservar MSAA.

Configuração:

```text
Off / 2x / 4x / 8x quando suportado
```

Default recomendado: 4x quando capability/performance tier permitir.

Não assumir que o framebuffer padrão possui o sample count desejado.

O viewport pode usar FBO multisample próprio + resolve.

Isso também torna screenshots/pixel tests previsíveis.

## Linhas

Não depender de `glLineWidth` para edges de viewport.

Drivers tratam line width de forma inconsistente.

Preservar a solução WGPU:

```text
edge segment
→ screen-aware triangle band
→ constant logical-pixel width
```

Implementação OpenGL pode expandir:

- em vertex shader;
- ou CPU para overlay batches simples.

O contrato visual é largura constante em pixels lógicos.

## EdgeMode

Preservar conceito:

```text
DRAW    → Features
POLY    → Topology
PAINT   → Overlay
UV      → Overlay
```

Mas `EdgeMode` pertence ao input de viewport, não ao backend.

Feature Edge derivada continua separada de Sharp Edge autoral.

## Selection overlays

Component overlays precisam de buffers independentes da mesh principal:

- points;
- selected vertices;
- selected edges;
- selected faces;
- hover/preselection.

Trocar seleção não deve reuploadar RenderMesh.

Overlay data pode ser reconstruída de forma incremental por selection revision.

## Object outline

Portar o comportamento WGPU de outline em largura constante.

OpenGL 3.3 suporta abordagem simples:

```text
selected objects
→ mask FBO
→ fullscreen dilation/neighbor search
→ composite
```

Raio pequeno em pixels não precisa Jump Flooding.

Não usar scaled-mesh outline como solução principal porque falha em objetos côncavos/escala não uniforme.

## Gizmos e tool overlays

Gizmo deve continuar GPU e sem depth test quando precisa permanecer legível.

UI/Application produz primitives neutras:

```text
ScreenLine
ScreenPolygon
WorldLine
WorldPoint
```

Renderer tessela/desenha.

Hit testing continua usando a mesma geometria lógica no Application/UI, não leitura de framebuffer.

## Workplane

Portar o workplane overlay já existente no WGPU:

- fill translúcido;
- grid;
- axes;
- tamanho aparente estável;
- leve depth bias quando necessário.

Renderer recebe somente WorkplaneOverlay pronto.

## Grid

Grid global e Workplane grid são overlays diferentes.

Grid settings vêm no RenderFrameInput.

Não consultar preferences/theme globalmente dentro do renderer.

## Tema e cores

Resolver tokens antes da chamada ao renderer.

Renderer recebe:

```text
ViewportPalette
```

com cores RGBA concretas.

Isso remove `ThemeRegistry::global()` do render-gl.

## Face Orientation

Portar para OpenGL alvo.

Pode ser shader variant/uniform que colore:

- front-facing;
- back-facing.

Usar `gl_FrontFacing` quando compatível com o passe.

Não recalcular face orientation na CPU.

## Triangulation overlay

Triangulação continua derivada da mesma tesselação usada por render/picking.

Overlay usa diagonais internas derivadas sem alterar authoring Mesh.

## X-Ray

Preservar como overlay/viewport feature, não novo Shading mode.

Regra:

- alpha configurado;
- depth read/write policy explícita;
- seleção through coerente com Application;
- não contaminar Material/Rendered semanticamente.

## Matcap

Portar.

É barato e de alto valor para leitura de forma.

Pode ser procedural como no WGPU atual; não exige textura externa.

Matcap é opção de Solid shading, não material do objeto.

## Ambient Occlusion

Preservar como recurso opcional, mas com tier.

OpenGL 3.3 suporta FBO/depth textures/fullscreen passes suficientes.

V1 recomendada:

- half resolution;
- depth-based AO;
- bilateral/depth-aware blur simples;
- somente Solid/Material Preview;
- desabilitado em X-Ray;
- desligável;
- low-performance tier pode iniciar desligado.

Não exigir compute shaders.

O GTAO atual do WGPU serve como referência de comportamento, não precisa ser portado linha por linha.

## Material Preview e Rendered

A arquitetura dos passes deve permitir os dois, mas o contrato visual precisa ser honesto.

### Solid

- studio light;
- object/base display color;
- optional Matcap;
- optional AO.

### Material Preview

- material base color/texture;
- roughness/metallic somente quando o shader realmente suportar;
- studio environment/light.

### Rendered

- luzes reais da cena suportadas pelo Petunia;
- sem prometer ray tracing/sombras que não existem.

Materiais detalhados serão fechados no domínio seguinte.

## Reference images

Preservar dois passes:

- behind geometry;
- X-Ray/front overlay.

Texture upload usa revision, não FNV dos pixels por frame.

RefAxis e placement são resolvidos fora do renderer em quad/world transform neutro quando possível.

## Render-on-demand

Preservar como requisito.

Frame é solicitado quando:

- camera muda;
- scene/render revision muda;
- selection/hover overlay muda;
- animation/paint/tool preview está ativo;
- UI precisa redesenhar.

Mouse parado sem mudança não deve reconstruir nem redesenhar continuamente.

## Performance tiers

Sem criar renderer diferente.

Um conjunto simples de settings:

```text
Low
Balanced
High
```

pode resolver defaults para:

- MSAA;
- AO;
- shadow quality futura;
- reference filtering;
- outline quality.

Manual overrides continuam possíveis onde fizer sentido.

Não ramificar lógica de produto inteira por tier.

## Capability detection

No startup registrar:

- GL version;
- GLSL version;
- vendor/renderer;
- max texture size;
- max samples;
- texture formats relevantes;
- anisotropy/extensões opcionais.

Se GL 3.3 Core não estiver disponível:

→ erro de inicialização claro.

Não existe backend WGPU fallback no destino final.

## Debug messages

Em debug/dev, habilitar `KHR_debug` quando disponível.

Mensagens de driver devem ir para diagnostics/log, com rate limiting se necessário.

Não expor spam técnico ao usuário final.

## Ownership de shaders

GLSL fica no crate `petunia-render`.

Preferir arquivos `.vert/.frag` ou módulos claramente separados em vez de strings enormes intercaladas com lógica Rust.

Shaders recebem nomes de passes, inputs e uniforms explícitos.

## Estado GL

Cada pass declara o estado necessário:

- depth test/write;
- blend;
- culling;
- stencil;
- framebuffer;
- program;
- VAO.

Evitar depender do estado deixado pelo passe anterior.

Um pequeno `RenderStateCache` pode evitar chamadas redundantes depois que o pipeline estiver correto.

Não criar command buffer abstrato.

## Estrutura alvo do crate

```text
petunia-render
├── renderer.rs
├── frame_input.rs
├── mesh.rs
├── gpu_cache.rs
├── texture_cache.rs
├── passes/
│   ├── surface.rs
│   ├── edges.rs
│   ├── selection.rs
│   ├── outline.rs
│   ├── references.rs
│   ├── workplane.rs
│   ├── ao.rs
│   └── overlays.rs
├── shaders/
└── diagnostics.rs
```

Não criar trait de backend: haverá um backend.

## O que reutilizar do render-gl

**REUSE / REFACTOR:**

- glow resource handling;
- shader compile/link error handling;
- persistent VBO idea;
- texture upload basics;
- reference image drawing;
- glutin bootstrap logic como referência para DesktopHost.

**REWRITE AT BOUNDARY:**

- `draw(&AppState, ...)`;
- global scene fingerprints;
- theme lookup;
- project traversal;
- direct evaluated_mesh calls;
- overlay hashing ligado a AppState.

## O que portar do render-wgpu

**PORTAR COMPORTAMENTO/ALGORITMO, NÃO API WGPU:**

- constant-pixel edges;
- EdgeMode;
- selection point/edge/face overlays;
- object outline;
- workplane overlay;
- GPU gizmo/screen overlays;
- Matcap;
- AO;
- MSAA behavior;
- texture dirty updates;
- revision-based invalidation;
- pixel/visual tests e fixtures.

## Ordem de migração

1. criar RenderFrameInput/RenderMesh neutros;
2. fazer Geometry/Application produzir RenderObjects;
3. mover GL context ownership para DesktopHost;
4. mover renderer concreto para `petunia-render`;
5. portar surface + texture + references;
6. portar corner normals/Flat-Smooth;
7. portar constant-pixel edges + EdgeMode;
8. portar selection overlays + workplane;
9. portar outline + GPU gizmos;
10. portar Face Orientation/X-Ray/triangulation;
11. portar MSAA próprio;
12. portar Matcap;
13. portar AO opcional;
14. migrar Paint dirty texture updates;
15. rodar parity/pixel suite;
16. remover `render-wgpu`, `render-gl` e backend selection code.

## Gates para remover WGPU

WGPU só sai quando OpenGL passar:

- Solid/Material visual parity dentro de tolerância;
- Flat/Smooth/Sharp;
- selection hover/active;
- constant-pixel edges em DRAW/POLY;
- X-Ray;
- Workplane/Grid;
- Object outline;
- Face Orientation;
- Reference Images;
- Gizmos;
- HiDPI;
- MSAA;
- Matcap;
- AO quando habilitado;
- Paint texture updates;
- render-on-demand;
- screenshots/pixel tests essenciais.

Não exigir pixel-identidade entre APIs; exigir comportamento e tolerância visual definida.

## Regra anti-bloat

Um renderer OpenGL concreto, uma scene/render boundary e poucos passes explícitos.

Não criar:

- RenderGraph genérico;
- multi-backend trait;
- ECS renderer;
- command buffer próprio;
- material node graph dentro do renderer;
- abstração de GPU hipotética.

O renderer deve ser simples de rastrear no código e previsível para humanos e agentes.