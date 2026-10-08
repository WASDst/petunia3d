# 29 — Geometry Core, Renderer, UV e Painting na Stack Rust

<aside>
📐

Esta página detalha como a stack Rust implementa as funcionalidades centrais do Petunia sem usar uma engine completa. O authoring core é nosso; bibliotecas externas resolvem problemas especializados atrás de providers.

</aside>

# PetuniaMesh próprio

O authoring mesh é uma estrutura half-edge própria e não um wrapper direto de crate externo.

Estrutura conceitual:

```
Mesh
├ SlotMap<VertexId, VertexTopology>
├ SlotMap<HalfEdgeId, HalfEdgeTopology>
├ SlotMap<EdgeId, EdgeTopology>
├ SlotMap<FaceId, FaceTopology>
├ MeshAttributes
└ MeshRevision
```

`slotmap` fornece keys generacionais; cada domínio deve declarar keys distintas para impedir mistura acidental de IDs.

## Topologia

A conectividade deve permanecer mínima e independente dos atributos sempre que isso simplificar lifecycle e DOD:

```
VertexTopology
└ outgoing_half_edge

HalfEdgeTopology
├ origin
├ twin
├ next
├ previous
├ face
└ edge

EdgeTopology
└ half_edge

FaceTopology
└ half_edge
```

Position, UV, material e sharpness não são obrigatoriamente campos desses structs topológicos.

# Attributes separados

Não inflar os elementos topológicos com todos os dados possíveis. Attributes e dados deriváveis ficam separados quando apropriado:

```
Topology
VertexPositions
CornerUV0 / HalfEdgeUV0
SharpEdges
MaterialByFace
future attributes
```

UV deve poder ser corner/face-vertex data quando seams exigirem valores distintos para um mesmo vertex geométrico. A representação concreta precisa preservar essa possibilidade sem duplicar vertices authoring apenas para armazenar UV.

Selection é estado de aplicação/viewport, não propriedade permanente obrigatória da topology.

# IDs e lifecycle

IDs antigos nunca devem resolver silenciosamente para elementos novos. Edits destrutivos retornam remaps quando necessário. APIs públicas recebem handles tipados e validam existência/revision.

# Triangulation Cache

Authoring aceita triangles/quads/n-gons; renderer e export usam triangles derivados.

```
PetuniaMesh
    ↓
TriangulationCache
    ├→ Renderer
    ├→ Picking
    ├→ Boolean provider
    └→ Export
```

Cada triangle derivado mantém o `FaceId` de origem para seleção e diagnostics.

# Revision/caches

Cada mesh possui revision. Caches registram a revisão fonte:

```
TriangulationCache
NormalCache
GpuMeshCache
PickingCache
UvAnalysisCache
```

Se a revisão relevante mudou, o cache é reconstruído/refeito. Preferir revision counters e dirty domains simples a um dependency graph sofisticado.

# Dirty domains

Distinguir ao menos:

```
TopologyDirty
PositionsDirty
UVDirty
MaterialDirty
TextureDirty
TransformDirty
ReferenceDirty
```

Mover um objeto não deve invalidar triangulation; pintar uma textura não deve reconstruir topology.

# Profiles e WorkPlanes

Profiles pertencem a um domínio 2D localizado em 3D:

```
Profile
├ WorkPlane
├ contours
│  ├ outer
│  └ holes
└ metadata

WorkPlane
├ origin Vec3
├ axis_u Vec3
├ axis_v Vec3
└ normal Vec3
```

Pontos do Profile são Vec2; o WorkPlane resolve sua posição 3D.

# `geo` como kernel 2D auxiliar

Usar `geo` somente onde remove complexidade concreta: predicates, intersections e polygon operations de Profile. Não transformar tipos `geo` no formato de documento público.

Fluxo:

```
Petunia Profile
→ adapter geo::Polygon
→ operation
→ validated Petunia Profile
```

# Profile triangulation

Profiles fechados e validados são triangulados para gerar caps. Para V1, preferir algoritmo Earcut/triangulação já disponível pela stack 2D. Inputs auto-intersectantes ou inválidos devem falhar com diagnóstico claro em vez de depender de comportamento indefinido do triangulador.

# Operações locais primeiro

Extrude, Push/Pull, Inset, Slice, Connect, Weld, Bevel 1 segment e Revolve são operações do nosso Geometry Core. Não usar Boolean como atalho para toda edição.

# Fuse/Cut complexo

`manifold-rust` é provider para operações volumétricas que realmente exigem Boolean.

```
Fuse/Cut command
→ tentar operação local quando apropriado
→ se interseção volumétrica arbitrária: triangulation snapshot
→ manifold-rust
→ output triangles
→ reconstruct PetuniaMesh
→ safe cleanup
→ validate
→ preview/commit
```

Não aplicar remesh global. Cleanup permitido continua restrito a degenerates, coincident vertices seguros, safe coplanar cleanup e validation.

# UV pipeline

Auto UV possui três rotas:

```
Known generator?
  yes → deterministic native UV
  no ↓
Reference/View projection applicable?
  yes → projected UV
  no ↓
xatlas fallback
```

Primitives/Profile Extrude devem gerar UV previsível quando a parametrização é conhecida. `Project From Reference` usa a câmera/reference conhecida e não precisa de unwrap genérico.

# xatlas

Usar wrapper Rust apenas atrás de `UvUnwrapProvider`. Nenhum tipo xatlas pode aparecer na Application API ou no formato `.petunia`.

O provider recebe uma mesh/snapshot convertida e retorna islands/UVs em tipos Petunia.

# Renderer próprio sobre wgpu

Não usar Bevy/Fyrox/engine. Renderer mínimo:

```
PetuniaRenderer
├ GpuContext
├ ViewportRenderer
├ MeshRenderer
├ GridRenderer
├ ReferenceRenderer
├ OverlayRenderer
├ GizmoRenderer
└ TextureUploader
```

# Render passes

Baseline simples:

1. Reference/Grid;
2. opaque/textured mesh;
3. wire/topology overlay;
4. selection highlights;
5. Profiles/WorkPlane;
6. gizmos/snap hints.

Silhouette é variante do pipeline de mesh/visualization; não requer render engine separado.

Não criar GI, shadow system avançado, path tracing ou pós-processamento de engine.

# WGSL

Shaders versionados pelo projeto:

```
mesh.wgsl
grid.wgsl
wireframe.wgsl
selection.wgsl
reference.wgsl
silhouette.wgsl
gizmo.wgsl
uv_checker.wgsl
```

# egui ↔ wgpu

A integração deve permanecer isolada entre `petunia-ui` e `petunia-render`. A estratégia preferida é **custom rendering direto via `egui-wgpu`**:

```
egui layout
→ allocate viewport Rect
→ Petunia viewport adapter
→ egui-wgpu custom callback / RenderPass
→ PetuniaRenderer
→ wgpu
```

egui controla layout, clipping e input da região; `PetuniaRenderer` continua responsável pela renderização 3D. `petunia-render` não possui dependência de egui/eframe e expõe apenas a interface necessária para registrar/preparar/renderizar o snapshot no pass recebido pelo adapter.

# Picking

Começar com a solução mais simples que satisfaz low-poly:

```
screen cursor
→ camera ray
→ iterate/render triangles
→ Möller-Trumbore/intersection
→ nearest triangle
→ source FaceId
```

Não adicionar BVH antes de profiling indicar necessidade. Quando necessário, introduzir `PickingBackend` e estrutura BVH derivada, nunca tornar BVH parte da authoring mesh.

# Painting data model

`TextureBitmap` é tipo de domínio próprio:

```
width
height
pixel format
pixels
revision
```

Não usar `wgpu::Texture`, `egui::TextureHandle` ou um tipo da crate `image` como modelo persistente do documento.

# Paint on Model pipeline

```
pointer
→ raycast
→ triangle
→ barycentric coordinates
→ interpolate UV
→ texture coordinates
→ Brush operation
→ TextureBitmap
→ DirtyRect
→ partial GPU upload
```

# Paint V1

Ferramentas mínimas próprias sobre `TextureBitmap`:

- Pixel Pencil;
- Brush;
- Eraser;
- Fill;
- Color Picker;
- Line;
- Rectangle;
- Palette + recent colors;
- Pixel Grid contextual.

**Symmetry Painting fica V1.x**, conforme o contrato funcional do capítulo 15. Não adotar Skia/Krita-like engine na V1.

## Paint 2D e 3D compartilham estado

O editor 2D opcional e o Paint on Model são duas apresentações do mesmo domínio:

```
Paint 3D ─┐
          ├→ Paint Commands → TextureBitmap → Undo tile diffs → DirtyRect → GPU upload
Paint 2D ─┘
```

Não duplicar bitmap, histórico, palette ou estado persistente por editor. UI state como zoom/pan do canvas 2D permanece efêmero no `petunia-ui`; conteúdo raster e transactions continuam no domínio de textura. Alterações feitas em qualquer superfície invalidam a mesma `TextureBitmap.revision` e aparecem na outra imediatamente.

# Texture dirty regions

Cada stroke acumula dirty rectangle/tile range. O renderer atualiza apenas a região alterada via GPU upload quando possível. Um stroke inteiro gera uma única operação de Undo lógica.

# Normal/tangent

Normals são derivados do authoring mesh. Flat/Smooth/Sharp obedecem às regras já fechadas. Tangents/MikkTSpace só entram quando normal maps exigirem, evitando dependência prematura.

# Export optimization

`meshopt` é optional/future provider para export optimization, cache ordering ou LOD. Não modificar authoring topology automaticamente.

# Relação com a arquitetura de código

A representação desta página deve obedecer [34 — Arquitetura Modular Explícita, Rust Safety e Representação em Código](34-arquitetura-modular-rust-safety.md). Em especial: algorithms geométricos não conhecem UI/Lua/MCP/Undo; Tools não chamam Tools; long-lived relationships usam IDs; e DOD é aplicado somente onde simplifica o fluxo de dados ou hot paths reais. A integração específica da UI/viewport segue [35 — egui, Petunia Components, UI Adapters e Tooling de Desenvolvimento](35-egui-components-adapters-tooling.md).

# Regra final

**Petunia possui seus dados e suas operações essenciais; providers especializados entram apenas quando evitam reinventar algoritmos difíceis.**