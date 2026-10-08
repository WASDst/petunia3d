# Paint Architecture

> **Status: proposta recomendada; aguarda aprovação.**

## Diagnóstico atual

O subsistema atual já possui funcionalidades valiosas:

- Pixel/Soft Brush;
- Eraser;
- Flood Fill;
- Eyedropper;
- Line/Rectangle;
- vertex color paint;
- pintura 3D por raycast + UV;
- selection isolation;
- raster layers;
- blend modes;
- effects;
- decals;
- SVG decals;
- surface-projected decals;
- dirty tiles;
- partial composition;
- partial GPU texture uploads.

O problema principal é a integração atual:

```text
PaintLayerStack
    ↓ composite
Asset.texture
    ↓ copy/sync
Material.albedo_texture
    ↓ upload
Renderer
```

Há três representações do mesmo resultado e PaintModule ainda depende diretamente de AppState.

## Decisão principal

Paint vira um authoring subsystem que produz TextureResource:

```text
PaintDocument
     ↓
layer composition
     ↓
TextureResource
     ↓
Material channel
     ↓
Renderer
```

SceneObject não mantém cópia paralela da textura.

## PaintDocument

```rust
pub struct PaintDocument {
    pub id: PaintDocumentId,
    pub name: String,
    pub target: PaintTarget,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<PaintLayer>,
    pub output_texture: TextureId,
    pub revision: Revision,
}
```

## PaintTarget

```rust
pub enum PaintTarget {
    MaterialChannel {
        object: ObjectId,
        material_slot: MaterialSlotIndex,
        channel: PaintChannel,
    },
}
```

O target object explícito permite surface decals e projection paint sem depender do objeto ativo global.

## PaintChannel

```rust
pub enum PaintChannel {
    BaseColor,
    Roughness,
    Metallic,
    Emission,
    Height,
    Opacity,
}
```

Normal maps podem ser consumidos pelo Material, mas direct normal painting não entra na primeira versão sem brush vetorial próprio.

Fluxo recomendado inicialmente: paint Height → Generate Normal Map, ou importar normal map existente.

## Color versus scalar channels

BaseColor/Emission usam color picker e RGBA/RGB.

Roughness/Metallic/Height/Opacity usam valor escalar 0..1 e eyedropper escalar.

O engine pode reutilizar Canvas RGBA durante a migração, mas a API não deve assumir que todo canal é cor.

## PaintSession

Estado transitório:

```rust
pub struct PaintSession {
    pub document: PaintDocumentId,
    pub active_layer: PaintLayerId,
    pub brush: BrushSettings,
    pub stroke: Option<PaintStrokeSession>,
    pub isolate_selection: bool,
}
```

`active_layer` sai da semântica persistente da PaintLayerStack.

## BrushSettings

BrushSettings sai do minimal Core e pertence a Paint/Application.

Descriptor inclui type, radius, hardness/falloff, strength, spacing, color/value e pressure mapping futuro.

## Stroke transaction

```text
pointer down
→ begin PaintStrokeSession
→ many dabs
→ accumulate dirty tiles
→ pointer up
→ one Undo transaction
```

Cancel restaura os tiles capturados antes do stroke.

## Tile system

Preservar fortemente:

```text
TILE_SIZE = 32
DirtyTiles
composite_tiles
TextureDirtyRect
partial GPU upload
```

Classificação: REUSE.

## Undo por tile

Strokes grandes não devem depender de snapshot completo do Project.

Direção:

```rust
pub struct PaintStrokeDelta {
    pub document: PaintDocumentId,
    pub tiles: Vec<PaintTileDelta>,
}
```

Primeira implementação pode manter before/after comprimido por tile.

## Layer stack

Preservar stable layer ID, order, visibility, opacity, raster, decal, effects, merge down e deterministic composition.

Separar active_layer da stack persistente.

## Blend modes

Produto V1 recomendado:

```text
Normal
Multiply
Screen
Add
```

Os algoritmos já existem; não adicionar dezenas de modos antes de necessidade real.

## Effects

Preservar Pixelate, Posterize, Invert, Grain, Levels, Brightness/Contrast e Hue/Saturation.

Effects continuam CPU/non-destructive dentro do PaintDocument, não shaders de Material.

## Composite cache

PaintDocument materializa seu resultado diretamente em `output_texture: TextureId`.

```text
layers changed
→ dirty tiles
→ composite into TextureResource
→ texture revision++
→ TextureUpdate
```

Não existe Canvas cache duplicado no SceneObject.

## Material integration

Para iniciar Paint em um canal:

1. resolve SceneObject.materials[slot];
2. resolve Material;
3. garante TextureResource para o canal;
4. PaintDocument escreve nesse recurso;
5. Material continua apontando para o mesmo TextureId.

## Recursos compartilhados

Se dois objetos compartilham Material/Texture, pintar altera ambos.

A UI deve indicar claramente:

```text
This material/texture is shared by 2 objects.

[Paint Shared]   [Make Unique]
```

Não duplicar silenciosamente e não modificar objetos relacionados sem feedback.

## UV requirement

Texture Paint 3D exige UV válida.

```text
valid UV?
├ yes → paint
└ no  → offer Create UVs / Auto Unwrap
```

Nunca criar UV silenciosamente.

## 3D paint query

```text
pointer ray
→ Evaluated Surface hit
→ FaceCorner UV
→ texel
→ PaintStrokeSession
```

Reutilizar a fundação espacial de Geometry/Application. Paint não cria raycast próprio.

## Modifiers e generated surfaces

Raster Paint pode operar numa surface avaliada quando existe UV determinística.

Mirror/Array podem compartilhar UV entre cópias; pintar uma cópia pode afetar todas. Isso deve ser visível e previsível.

Para pintura única por cópia: Apply/Make Editable + UV adequada.

## Selection isolation

`isolate_selection` consulta SelectionState da Application.

O PaintDocument não persiste seleção de faces.

Generated-only faces sem identidade autoral não participam de persistent selection isolation na V1.

## Decal

Preservar DecalLayer, mas substituir DecalAnchor world-space pelo contrato de SurfaceAttachment aprovado.

Transform do SceneObject não quebra o decal.

Topology ambiguity leva a NeedsReattach, nunca reprojeção silenciosa.

## UV-space decal

Decal sem SurfaceAttachment continua válido como 2D decal no atlas: center_uv + scale_uv + rotation.

## SVG decals

Preservar source SVG, rerasterization, aspect ratio e deterministic bake.

Não descartar o SVG source depois da rasterização.

## Decal variants

Preservar dados/algoritmos existentes, mas sua UI não bloqueia Paint V1. Integração temporal será fechada em Animation.

## Projection Paint

Preservar conceito, mas operações não recebem AppState.

Paint operations recebem PaintDocument + PaintSurfaceContext + parâmetros explícitos.

## Vertex Paint

Vertex Color Paint continua separado de Texture Paint.

Opera diretamente em EditableMesh vertex color, um Undo por stroke, sem PaintLayerStack inicialmente.

Generator vivo requer Make Editable para Vertex Paint.

## Eyedropper

É channel-aware:

- BaseColor/Emission → color;
- scalar channel → scalar;
- vertex paint → vertex/interpolated color.

Não usar uma única função que presume RGBA.

## Resolution

Preservar presets 256, 512, 1024 e 2048.

4096 só entra quando hardware/memory policy permitir e o usuário escolher.

Defaults sugeridos: Low 512, Balanced 1024, High opcional 2048.

## Resize

Resize é operação explícita: resample raster layers, rerasterize SVG quando possível, preservar effects paramétricos e registrar um Undo.

Performance tier nunca redimensiona documento automaticamente.

## Color space

BaseColor/Emission são canais de cor; Roughness/Metallic/Height/Opacity são data/linear.

Composição precisa respeitar a política de color space definida no domínio Materials.

## Brush preview

Brush preview é overlay transitório; não altera TextureResource.

Application produz cursor/footprint e Renderer apenas desenha.

## Ownership arquitetural

Project/Document: PaintDocument, PaintLayer data, decal source, output TextureId, target references.

Geometry: UV/barycentric helpers, spatial queries, FaceCorner e SurfaceAttachment math.

Application: PaintSession, BrushSettings, pointer lifecycle, selection isolation, commands/undo e shared-resource warnings.

Renderer: recebe TextureResource atualizada e dirty regions; não conhece PaintLayer.

## Migração recomendada

1. introduzir PaintDocumentId/PaintChannel/PaintTarget;
2. separar active_layer da persistent stack;
3. criar TextureResource output;
4. remover Asset.texture como composite cache;
5. remover sync de cópias para Material.albedo_texture;
6. migrar PaintModule para operações sem AppState;
7. mover stroke lifecycle para Application ToolSession;
8. implementar tile-delta Undo;
9. migrar 3D paint para spatial query compartilhada;
10. migrar DecalAnchor para SurfaceAttachment/local frame;
11. adaptar color/scalar channels;
12. integrar UX de recursos compartilhados;
13. migrar TextureUpdate para TextureId;
14. manter SurfaceRecipe e decal animation fora do critical path.

## Regra anti-bloat

Paint deve continuar com a gramática:

```text
choose channel
→ choose layer
→ paint
```

A arquitetura interna pode ser robusta sem expor complexidade de Photoshop/Substance para tarefas simples.
