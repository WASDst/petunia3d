# Image Reference & Photo Projection

> **Status: proposta recomendada; aguarda aprovação.**

## Objetivo de produto

Transformar referência + projeção em uma das features de identidade do Petunia3D.

O fluxo principal deve permitir que um iniciante use uma foto, blueprint ou desenho para:

```text
Import Reference
→ Align / Calibrate
→ Trace / Build
→ Project Photo
→ Refine / Paint
→ Export
```

Sem exigir que ele configure manualmente:

- câmera de projeção;
- Image Texture nodes;
- UV Project modifier;
- material intermediário;
- projector object;
- bake pipeline.

Casos prioritários:

- prédios e fachadas;
- casas/interiores simples;
- props;
- móveis;
- veículos low-poly;
- caixas/embalagens;
- placas;
- reconstrução estilizada de objetos por foto;
- blockout baseado em blueprint.

## Diagnóstico da implementação atual

### ReferenceImage

Hoje vive em `core::state` e contém diretamente:

```text
name
width / height
rgba pixels
axis
offset
size
opacity
visible
locked
rotation
xray
revision
```

Problemas:

- não possui ID estável;
- pixels ficam dentro do estado Core;
- só representa planos alinhados aos eixos;
- possui somente offset normal + size + rotação no plano;
- não possui posição 2D livre no plano;
- não possui crop;
- não possui calibração de escala;
- não possui câmera/perspectiva persistente.

## Bug estrutural de persistência

As referências atuais ficam em:

```text
ProjectState.refs: Vec<ReferenceImage>
```

mas `save_project()` serializa somente:

```text
state.project.project
```

Portanto ReferenceImage ainda não é realmente recurso autoral do Document, apesar da documentação antiga dizer que é.

Isso precisa ser corrigido antes de Photo Projection ser considerada confiável.

## Reference rendering atual

O renderer usa um quad derivado de:

```text
RefAxis
offset
size
image aspect
in-plane rotation
```

É suficiente para blueprint ortográfico simples.

Não é suficiente para:

- referência custom em plano arbitrário;
- foto perspectiva;
- crop/calibration;
- photo matching.

## Project From Reference atual

O comando atual converte `RefAxis` em dois vetores de plano e chama:

```rust
mesh.project_from_view(right, up, origin)
```

O algoritmo:

- faz dot product de todos os vertices contra right/up;
- encontra bounds da Mesh;
- normaliza esses bounds para 0..1;
- substitui UVs.

Isso significa que o comando atual **não projeta realmente a imagem de referência como enquadrada na viewport**.

O tamanho visual da referência e seu framing não são a autoridade do UV.

Além disso:

- não suporta perspectiva;
- não usa crop real;
- não preserva framing exato;
- não é adequado como feature flagship.

Classificação:

```text
Project From Reference atual
→ REUSE concept/name
→ REPLACE projection math/integration
```

## Um núcleo de projeção bom já existe

`module-paint::projection` contém uma base forte:

- `DepthBuffer` com `view_proj` real;
- clipping no near plane;
- visibility/occlusion;
- back-face filtering;
- screen→image mapping;
- bilinear sampling;
- per-face filtering;
- UV texel rasterization;
- 1 px controlled dilation;
- dirty tile output.

Classificação:

```text
projection math / depth / bake core
→ STRONG REUSE
```

O código deve migrar para uma API compartilhada de projection, não continuar propriedade exclusiva de Paint.

## Decisão conceitual central

Separar três ações que hoje tendem a ser confundidas:

```text
Reference Image
→ guia visual para modelar

Project UV From Reference
→ escreve FaceCorner UV

Project Photo
→ usa a ReferenceView como projector e compõe a imagem numa TextureResource
```

`Project Photo` é a feature principal para iniciantes.

`Project UV From Reference` continua ferramenta técnica de UV.

## Por que Project Photo não deve depender de reescrever UV

O motor de projection já consegue:

1. rasterizar cada texel da UV existente;
2. recuperar sua posição 3D;
3. projetar essa posição na ReferenceView;
4. testar visibilidade/oclusão;
5. amostrar a foto;
6. gravar no PaintDocument/TextureResource.

Logo uma projeção perspectiva pode ser correta por texel sem transformar a perspectiva diretamente em UV linear.

Isso evita parte dos artefatos típicos de projection UV perspectiva em geometria low-poly.

## Arquitetura alvo

```text
Document
├ TextureResources
├ ReferenceSets
│  └ ReferenceViews
├ SceneObjects
├ Materials
└ PaintDocuments
   └ ReferenceProjectionLayers
```

## ReferenceImage versus ReferenceView

Separar conteúdo de imagem de sua colocação/projeção.

### Conteúdo

Reutilizar TextureResource:

```rust
ReferenceView {
    image: TextureId,
    ...
}
```

A foto é sRGB por default.

Não duplicar RGBA dentro de ReferenceView.

## ReferenceSet

```rust
pub struct ReferenceSet {
    pub id: ReferenceSetId,
    pub name: String,
    pub views: Vec<ReferenceViewId>,
}
```

Um projeto pode ter:

```text
House References
├ Front
├ Right
├ Top
└ Perspective 01
```

ou:

```text
Prop References
├ Front
├ Side
└ Back
```

## ReferenceView

Direção:

```rust
pub struct ReferenceView {
    pub id: ReferenceViewId,
    pub name: String,
    pub image: TextureId,
    pub role: ReferenceRole,
    pub projection: ReferenceProjection,
    pub crop: ImageCrop,
    pub display: ReferenceDisplay,
    pub revision: Revision,
}
```

## ReferenceRole

```text
Front
Back
Left
Right
Top
Bottom
Custom
Perspective
```

`Side` legado migra para Left ou Right explicitamente.

Role organiza UX; não deve ser usado como matemática secreta quando existe Projection explícita.

## ReferenceProjection

```rust
pub enum ReferenceProjection {
    Orthographic(OrthographicReference),
    Perspective(PerspectiveReference),
}
```

## OrthographicReference

Representar um frame de plano verdadeiro:

```text
origin
right
up
normal
world height
```

Aspect ratio vem da imagem/crop.

Slots Front/Back/etc. apenas geram defaults desse frame.

Depois o usuário pode mover/rotacionar/calibrar a referência sem perder identidade.

## PerspectiveReference

Persistir uma câmera de referência independente da Editor Camera.

Direção:

```text
camera transform
vertical FOV
aspect
near/far derived
lens shift / principal point when needed
```

Não usar a Camera transitória do EditorSession como autoridade persistente.

## ImageCrop

Crop precisa ser authoring metadata:

```text
min UV
max UV
```

Projetar uma foto cropada não deve exigir editar pixels destrutivamente.

## ReferenceDisplay

Pode guardar preferências úteis ao projeto:

- opacity;
- visible;
- locked;
- xray/front overlay;
- mirror X/Y quando necessário.

Esses campos não entram em Material/Export; existem para o workflow de referência.

## Persistência

ReferenceSet e ReferenceView migram para `petunia-project::Document`.

São:

- salvos no `.petunia`;
- incluídos no autosave;
- undoable;
- acessíveis a CLI/MCP/plugins por IDs tipados.

Reference images são embutidas por default via TextureResource para evitar projetos quebrados ao mover pastas.

Original filename/source path pode permanecer somente como metadata.

## Reference Manager

Preservar a base existente de cards e seis slots, mas evoluir de 'lista de imagens' para 'Reference Set'.

Cada card mostra:

```text
thumbnail
name
role
projection type
scale/calibration state
visibility
lock
opacity
```

Ações rápidas:

```text
[View] [Align] [Trace] [Project Photo]
```

## Import UX

Ao importar imagem perguntar somente o necessário:

```text
Use as:

○ Blueprint / Orthographic
○ Perspective Photo
○ Free Reference
```

Não mostrar parâmetros de câmera antes de o usuário escolher Perspective.

## Blueprint / Orthographic flow

Fluxo mínimo:

```text
Import
→ choose Front/Side/Top/Custom
→ place
→ optional Set Scale
→ Trace / Model
```

### Set Scale

Feature de alto valor e baixo custo:

1. clicar dois pontos conhecidos na imagem;
2. digitar distância real;
3. Petunia calcula world scale da ReferenceView.

Exemplo:

```text
click two corners of a door
→ 0.90 m
→ entire blueprint calibrated
```

Isso é extremamente útil para arquitetura e props.

## Reference Origin

Permitir escolher um ponto da imagem como:

```text
World Origin / Reference Origin
```

Com isso Front/Side/Top de um mesmo set podem compartilhar um ponto sem exigir posicionamento numérico manual.

## Quick alignment helpers

Para blueprints, oferecer:

- center image;
- align ground/baseline;
- align to world origin;
- flip horizontal;
- rotate 90°;
- copy scale from another view.

Essas ações são mais úteis para iniciantes que campos XYZ genéricos.

## Perspective Match

Para fotos, criar um modo guiado específico.

Objetivo:

```text
Photo
→ camera calibration
→ model overlays photo correctly
→ trace/build
→ project same photo
```

## Perspective Match V1

Começar com **2-point perspective**, ideal para prédios, casas, móveis e props com eixos dominantes.

Passos progressivos:

```text
1. Match horizontal direction A
2. Match horizontal direction B
3. Set origin
4. Set scale
5. Done
```

A UI mostra dois pares de guias de fuga, mas somente o passo atual recebe destaque forte.

Horizon é derivado automaticamente.

## Vertical handling

Default:

```text
Keep verticals vertical
```

que cobre fotos arquitetônicas razoavelmente niveladas.

Modo avançado futuro pode permitir terceiro vanishing point para câmera inclinada.

Não começar com calibração fotogramétrica genérica.

## Match quality feedback

Exibir feedback simples:

```text
Perspective match
✓ Directions converge
✓ Horizon resolved
○ Scale not set
```

Não mostrar matriz de câmera ou focal length técnica por default.

## Use Current View

Também permitir:

```text
Create Perspective Reference From Current View
```

Útil quando o usuário alinha o modelo visualmente por conta própria.

A câmera atual é copiada para uma ReferenceView persistente; não fica vinculada à editor camera.

## Trace Mode

Ativar uma experiência focada:

- ReferenceView fica alinhada e bloqueada;
- câmera entra na reference view;
- mesh pode ficar semi-transparente/wire overlay;
- DRAW usa Workplane apropriado;
- Reference opacity fica acessível no header;
- usuário pode alternar rapidamente Photo ↔ Model.

Não criar um workspace novo; é um modo/contexto dentro de MODEL.

## Trace & Build

Workflow flagship:

```text
Import Photo
→ Match / Calibrate
→ Trace facade/profile
→ Extrude / Push-Pull
→ Project Photo
```

A mesma ReferenceView dirige todas as etapas.

## Project Photo

Essa é a ação principal de textura.

Ela cria ou atualiza uma layer não destrutiva no PaintDocument:

```rust
ReferenceProjectionLayer {
    reference: ReferenceViewId,
    target: ProjectionTarget,
    opacity: f32,
    blend: Normal,
    backface_cull: bool,
    occlusion: ProjectionOcclusion,
}
```

O tipo exato pode variar, mas a relação persistente é obrigatória.

## ProjectionTarget

V1 recomendada:

```text
Visible Surface
Selected Faces
Facing Surface
```

### Visible Surface

Default para iniciante.

O z-buffer da própria geometria decide o que a foto consegue ver.

### Selected Faces

Limita a projeção a uma seleção autoral.

Se a topology mudar e o face-set não puder ser remapeado, layer entra em `NeedsReview`.

### Facing Surface

Filtro por ângulo entre surface normal e projector direction.

Útil para aplicar uma fachada sem atingir laterais muito inclinadas.

## Occlusion scope

Default:

```text
Target Object only
```

Outros objetos da cena não devem criar buracos inesperados na projeção.

Opção avançada futura:

```text
Selected Objects / Scene
```

## Live projection

Enquanto a layer estiver viva:

```text
move/calibrate ReferenceView
→ projection reevaluates

```
edit target geometry
→ projection reevaluates

```
change crop
→ projection reevaluates
```

Isso transforma Reference Projection em ferramenta de authoring, não em bake descartável.

## Evaluation dependencies

Cache key conceitual:

```text
ReferenceView revision
+ source TextureResource revision
+ target ObjectId
+ evaluated geometry revision
+ object/world transform revision
+ UV revision
+ PaintDocument resolution
+ projection layer revision
```

Sem hash integral de pixels/mesh por frame.

## Reutilizar o projection engine atual

Promover para Geometry/Paint shared service algo equivalente a:

```text
ProjectionEvaluator
├ Projector transform
├ depth/visibility cache
├ surface texel projection
├ image sampling
└ dirty region result
```

`DepthBuffer`, near clipping, visibility test e bake texel logic atuais devem ser preservados/refatorados.

## Reference projector mapping

Para cada texel alvo:

```text
UV texel
→ surface world position
→ ReferenceProjection view-projection
→ image coordinates
→ visibility test
→ sample source image
→ composite
```

Perspective e Orthographic usam o mesmo pipeline.

## Não depender do viewport atual

`Project Photo` usa a ReferenceView persistente, não a câmera que o usuário deixou no viewport.

Isso torna Undo, export, headless e MCP determinísticos.

## Existing UV requirement

Para compor numa TextureResource raster, o target precisa ter UV estável.

Fluxo:

```text
valid UV?
├ yes → project
└ no  → offer Auto UV / Generator UV / Cancel
```

Não fazer unwrap silencioso.

## Material creation

Se o objeto não tiver material/canal BaseColor configurado, `Project Photo` pode criar em uma única transaction:

```text
Material
+ PaintDocument
+ output TextureResource
+ ReferenceProjectionLayer
```

Mas a preview/confirmation deve deixar claro que isso será criado.

Não voltar ao legado `Asset.texture`.

## Photo lighting

Fotos normalmente já contêm luz e sombra.

Oferecer escolha simples:

```text
Appearance
● Keep Photo Lighting
○ Relight With Material
```

`Keep Photo Lighting` pode usar Unlit.

`Relight With Material` usa Standard PBR e aceita que a foto ainda contém iluminação baked.

Isso evita dupla iluminação inesperada.

## Resolution

Antes de aplicar, mostrar resolução estimada do resultado:

```text
Texture: 1024 × 1024
Projected area: ~680 px wide
Quality: Good
```

Se a foto possui mais detalhe que a textura alvo comporta, sugerir aumentar resolução.

Não alterar automaticamente sem confirmação.

## Project From Reference UV

Manter como ferramenta técnica, mas renomear na UI para:

```text
Project UV From Reference
```

para diferenciá-la de:

```text
Project Photo
```

Nova implementação deve usar o framing real da ReferenceView.

Não normalizar simplesmente mesh bounds para 0..1.

## Project From View UV

Também renomear:

```text
Project UV From View
```

Continua útil para workflow custom.

## Sem fallback silencioso

O comando atual `Project From Reference` pode cair para current view quando não existe referência.

Remover esse comportamento.

Se não há ReferenceView ativa:

```text
No active reference
[Choose Reference] [Project From View]
```

A intenção do usuário nunca muda silenciosamente.

## Multi-view projection

Essa é a extensão mais valiosa após Single Reference Projection.

Um ReferenceSet pode dirigir várias projection layers:

```text
Front  → front visible surfaces
Right  → right visible surfaces
Top    → top visible surfaces
```

Mesmo sem blending automático, isso já é muito poderoso para prédios/props.

## Auto Multi-View

Recomendado como feature flagship V1.x.

Para cada texel/surface sample e cada ReferenceView:

1. está dentro do image bounds?
2. é visível pelo projector?
3. está front-facing?
4. calcular facing score;
5. escolher a melhor referência.

Score básico:

```text
max(0, dot(surface_normal, direction_to_projector))
```

Pode receber peso de distância/borda depois.

## Multi-view blend

Primeira versão de Auto Multi-View pode usar winner-takes-all.

Depois, feather entre as duas melhores views perto de transições.

Não começar com optical-flow/photo-consistency/fotogrametria.

## Seam correction

Depois da projeção multi-view, oferecer ferramentas pequenas:

- Feather Boundary;
- Clone/Heal via Paint quando existir;
- Paint over seam;
- choose source view for selected region.

A correção manual é preferível a um algoritmo pesado escondendo decisões.

## Projection masks

ReferenceProjectionLayer pode usar máscara derivada de:

- face target;
- visibility;
- facing angle;
- optional paint mask futura.

Não criar um segundo mask subsystem agora.

## Difference from Decal

Decal e Reference Projection reutilizam matemática de projection, mas têm intenções distintas.

```text
Decal
→ detalhe localizado
→ placement sobre surface

Reference Projection
→ texturização ampla guiada por camera/reference
→ source view calibrada
```

Não fundir os dois conceitos no modelo de produto.

## Difference from photogrammetry

Petunia não tenta reconstruir automaticamente depth/mesh de várias fotos.

O usuário continua criando a forma.

O sistema ajuda com:

- camera match;
- tracing;
- scale;
- projection;
- multi-view texture assignment.

Isso mantém resultado previsível e low-poly.

## Architecture / building workflow

Exemplo:

```text
1. Add Front Blueprint
2. Set Scale: door = 0.90 m
3. Trace facade outline
4. Extrude depth
5. Add Perspective/Side Photo
6. Match Photo
7. Project Photo → Visible Surface
8. Add Side projection
9. Paint seams/details
```

Sem abrir UV Editor se os generators/Auto UV já fornecerem UV adequada.

## Prop workflow

```text
Front photo
+ Side photo
→ calibrate common scale
→ trace
→ extrude / shape
→ Auto Multi-View
→ Paint cleanup
```

## Beginner UX principle

Expor ações em linguagem de intenção:

```text
Align Photo
Set Scale
Trace
Project Photo
Add Side Photo
Fix Seam
```

e deixar termos como:

```text
view-projection matrix
camera intrinsics
FaceCorner UV
depth buffer
```

apenas na implementação/docs técnicos.

## Comparison lessons

### Blender

`Project from View` e UV Project são poderosos, mas o usuário normalmente precisa entender View/Camera + UV + Material/Image Texture e há limitações conhecidas em projeção perspectiva low-poly.

Petunia deve reduzir isso para uma ação contextual baseada numa ReferenceView persistente.

### SketchUp

Match Photo demonstra o valor de vanishing points + origin + scale para arquitetura.

Petunia deve preservar o poder, mas usar um wizard progressivo em vez de apresentar todas as guias simultaneamente.

### Shapr3D

Import Image tem UX simples de move/scale/rotate/opacity e é ótima referência para tracing.

Petunia deve ir além reutilizando a mesma ReferenceView também para Photo Projection.

## Undo

Regras:

- import reference = 1 Undo;
- calibration gesture = 1 Undo;
- perspective match confirm = 1 Undo;
- reference transform gesture = 1 Undo;
- create Project Photo layer = 1 Undo;
- changing projection parameters = 1 Undo per gesture;
- Bake/Flatten = 1 Undo.

Preview/calibration dragging não cria histórico por mouse move.

## Export

ReferenceViews são authoring metadata.

Export glTF/OBJ não precisa incluir imagens de referência.

ReferenceProjectionLayers são compostas para as TextureResources exportadas.

Opcionalmente Petunia-native metadata pode manter provenance/reference info, mas não é requisito de interoperabilidade.

## MCP / Plugins

IDs estáveis permitem comandos determinísticos:

```text
reference.import
reference.set_scale
reference.match_perspective
reference.align
projection.create
projection.set_target
projection.bake
```

Nenhum comando depende de 'first visible reference'.

## Performance

Projection é authoring evaluation, não operação por frame.

Cachear depth/projector data por ReferenceView + target geometry revisions.

Live preview pode usar:

- resolução menor;
- selected/visible bounds;
- delayed final recomposition após gesture.

Commit/idle usa qualidade final.

Mesma filosofia Preview/Final dos Generators.

## Performance tiers

Low tier:

- projection preview half/quarter resolution;
- final bake unchanged;
- no automatic feather multi-view.

Qualidade final do documento não depende do tier.

## Migração recomendada

1. introduzir ReferenceViewId/ReferenceSetId;
2. mover references para Document/petunia-project;
3. substituir embedded RGBA por TextureId;
4. migrar RefAxis placement para OrthographicReference frame;
5. manter six-slot Reference Manager como presets;
6. fazer renderer consumir RenderReferenceImage neutra;
7. extrair projection engine compartilhado de module-paint;
8. corrigir Project UV From Reference para usar exact ReferenceView framing;
9. remover fallback silencioso para current view;
10. introduzir ReferenceProjectionLayer no PaintDocument;
11. implementar Blueprint Set Scale;
12. implementar Trace Mode;
13. implementar Project Photo ortográfico;
14. implementar PerspectiveReference + guided 2-point Match;
15. implementar Project Photo perspectiva;
16. integrar material/PaintDocument creation transaction;
17. adicionar multi-view manual;
18. adicionar Auto Multi-View winner-takes-all;
19. adicionar feather somente depois de profiling/UX validation.

## Gates obrigatórios

- references sobrevivem save/load/autosave;
- replacing image preserva ReferenceView identity;
- duplicate project is self-contained;
- ortho reference render framing == projection framing;
- changing reference size/position changes live projection consistently;
- perspective match reproduces reference camera;
- Project Photo does not depend on current editor camera;
- selected-face projection never paints other faces;
- self-occluded surfaces are not painted;
- photo projection preserves target UV topology;
- reference source image is never destructively modified;
- one gesture = one Undo;
- export output equals PaintDocument composite;
- Project UV From Reference and Project Photo remain distinct commands.

## Não objetivos V1

Não adicionar:

- automatic 3D reconstruction;
- Structure from Motion;
- dense photogrammetry;
- neural depth estimation;
- automatic object segmentation;
- optical-flow blending;
- camera lens distortion solver completo;
- arbitrary 3-point camera calibration no primeiro slice;
- cloud photo processing.

## Regra anti-bloat

A feature deve ser poderosa porque reutiliza a mesma ReferenceView em toda a jornada:

```text
REFERENCE
→ ALIGN
→ TRACE
→ BUILD
→ PROJECT
→ PAINT
```

Não porque tenta virar um software de fotogrametria.