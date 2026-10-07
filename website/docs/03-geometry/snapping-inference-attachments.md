# Snapping, Inference, Workplane e Surface Attachments

> **Status: aprovado.**

## Diagnóstico atual

Há três caminhos parcialmente concorrentes:

1. `snap.rs` — snapping por distância em unidades de mundo, focado em uma Mesh;
2. `inference.rs` — snapping em pixels de tela, múltiplas fontes, oclusão, midpoint, edge, face, guides e grid;
3. `SplineSnapSettings / snap_spline_position` — snapping próprio de Spline em distância de mundo.

O segundo é a fundação correta para interação de viewport.

## Decisão recomendada

Ter **um único Inference Engine** para interação.

Ele atende Mesh, Spline, PlanarShape, Workplane/Grid, Poly Pen, Bezier Pen, transforms, Shape Builder e ferramentas futuras.

`snap.rs` deixa de ser engine e preserva apenas helpers matemáticos úteis de grid/increment até serem realocados.

`SplineSnapSettings` e `snap_spline_position` são aposentados após migração dos consumidores.

## Tolerância

Snapping interativo usa pixels lógicos, nunca unidades de mundo.

Grid/Increment continuam matemáticos em unidades do Workplane.

## Smart Snap explícito

O estado atual usa `SnapTarget::Grid` também como modo automático. Isso deve ser separado.

```rust
pub enum SnapMode {
    Smart,
    Grid,
    Increment,
    Point,
    Edge,
    Face,
}
```

## Prioridade

Preservar a prioridade atual:

```text
Point / Midpoint / Intersection / FaceCenter
    ↓
OnEdge
    ↓
Axis / Parallel / Perpendicular
    ↓
Angle
    ↓
OnFace
    ↓
Grid
```

Dentro da mesma classe vence o candidato mais próximo em pixels; profundidade desempata.

## Proveniência semântica

O resultado não pode conter somente posição.

```rust
pub struct SnapHit {
    pub position: WorldPosition,
    pub kind: SnapKind,
    pub pixel_distance: f32,
    pub reference: SnapReference,
}
```

`SnapReference` pode identificar:

- Grid/Guide virtual;
- Object + VertexIndex;
- Object + EdgeKey + edge parameter;
- Object + FaceIndex + barycentric/triangle reference;
- Spline + SplinePointId;
- Spline segment + parameter;
- visual-only evaluated surface quando não houver identidade autoral estável.

Isso permite que Poly Pen e Bezier Pen usem o mesmo hit tanto para feedback quanto para a operação, sem refazer picking.

## Stable versus visual-only

Nem toda geometria visível possui identidade autoral persistente.

### Stable authoring target

Exemplos: vertex/edge/face de EditableMesh, SplinePoint e Spline segment.

Pode ser usado para split edge, attachment persistente, reutilização de vertex e edição de componente.

### Visual-only target

Exemplos: faces criadas somente por Array/Mirror/Thickness ainda não aplicados e Generator result transitório.

Pode ser usado para posição, alinhamento, medidas e transform snapping, mas não para referência topológica persistente.

Ferramentas que exigem identidade estável filtram candidatos visual-only.

## Tool filters

A preferência global não substitui necessidades da ferramenta.

A consulta combina user SnapMode/settings com tool-specific mask/requirements.

Exemplos:

- Move → position-only candidates allowed;
- Poly Pen split → stable Mesh edge required;
- Bezier Pen on Surface → stable surface required;
- DRAW planar → Workplane grid + points + edges + inference guides.

## Workplane

Workplane é a autoridade para snapping planar.

Ele fornece origin, horizontal axis, vertical axis, normal e grid spacing/orientation.

DRAW usa os eixos do Workplane para Grid, Axis inference, angle inference e parallel/perpendicular references.

Não arredondar X/Y/Z globais quando o Workplane estiver inclinado.

## Inference guides

Manter e completar:

- Axis;
- Parallel;
- Perpendicular;
- Angle increment;
- Midpoint;
- Face center;
- Intersection.

A referência pode vir do último segmento criado, edge destacado/selecionado, Workplane axes ou transform constraint.

## Spline snapping

Spline entra no mesmo prepared scene.

Targets: control point, evaluated segment nearest point, endpoint e midpoint quando fizer sentido.

Segment reference deve preferir IDs dos pontos extremos + parâmetro, evitando depender apenas de índice de array.

## Prepared scene / acceleration

A ideia atual de `SnapAccel` é boa: preparar uma vez por gesto/revisão e consultar muitas vezes.

Evoluir para uma estrutura com identidade dos candidatos.

Não reconstruir edges/triangles/BVH em cada pointer event.

## Spatial queries compartilhadas

Hoje snapping, viewport picking e SurfaceAttachment constroem/consultam geometria espacial por caminhos diferentes.

Criar uma fundação pequena de spatial queries reutilizável:

```text
Evaluated Scene Geometry
        ↓
Prepared Spatial Data
├ points/edges
├ triangle BVH
├ bounds
└ provenance
```

Consumidores: object/component picking, snap occlusion, surface raycast, Bezier draw-on-surface e future Surface Conform.

Não criar scene graph espacial ou framework genérico além disso.

## Ownership por camada

### Geometry
- Triangle BVH;
- ray/triangle;
- nearest point helpers;
- grid/increment math;
- attachment frame math.

### Application / Editor
- Inference Engine;
- Camera projection;
- Workplane;
- tool masks;
- prepared scene;
- user/editor settings.

### Document / Project
- persistent SurfaceAttachment data;
- references por ObjectId;
- authoring topology revisions.

## SurfaceAttachment — target

Migrar target UUID/Asset para `ObjectId`.

Attachment persistente continua exigindo `ObjectGeometry::Mesh`.

## SurfaceAttachment — triangle reference

O formato atual usa `FaceIndex + triangle_index`, mas a triangulação de n-gons é derivada e pode variar com geometria.

Após FaceCorner, preferir referência explícita ao triângulo autoral usado no attachment:

```rust
pub struct SurfaceTriangleReference {
    pub face: FaceIndex,
    pub vertices: [VertexIndex; 3],
}
```

A barycentric coordinate é relativa a esses três vertices, não a uma posição ordinal numa triangulação recalculada.

## Revisions

SurfaceAttachment não usa revisão global do projeto.

EditableMesh deve distinguir pelo menos geometry revision e topology revision.

Mudanças somente de posição mantêm attachment válido e fazem ele acompanhar a superfície.

Mudanças de conectividade/índices tentam remap explícito; se ambíguo, estado vira `NeedsReattach`.

Evitar fingerprint integral da Mesh como mecanismo principal de avaliação.

## TopologyResult e attachments

```text
Geometry Operation
     ↓
TopologyResult / ElementRemap
     ↓
Application
     ├ update SelectionState
     └ update SurfaceAttachments when unambiguous
```

Se face/vertices envolvidos forem removidos ou splitados sem correspondência única, usar `NeedsReattach`.

Nunca reprojetar silenciosamente.

## Transform

Attachment guarda referência em geometria local e avalia em world:

```text
authoring triangle local
      ↓ barycentric
local SurfaceFrame
      ↓ SceneObject world transform
world SurfaceFrame
```

Transform do objeto não invalida attachment.

Normal/tangent devem ser transformadas corretamente inclusive com scale não uniforme.

## Modifiers

Topology-preserving modifiers como Bend/Twist/Taper/Stretch podem participar da avaliação do attachment porque mantêm identidade dos componentes.

Topology-changing modifiers não oferecem automaticamente superfície persistente endereçável.

Na primeira versão, não criar attachment em região apenas gerada por modifier; quando a referência não puder ser preservada explicitamente, exigir Apply/Make Editable.

## Bezier Pen on Surface

```text
pointer
  ↓
Inference Engine
  ↓ stable face SnapHit
  ↓
SurfaceAttachment
  ↓
SplinePoint
```

Sem segundo raycast.

## Poly Pen

SnapHit semântico permite:

```text
Point hit → reuse VertexIndex
Edge hit  → split EdgeKey at parameter
Empty/workplane → create new vertex
```

A ferramenta não precisa inferir novamente qual componente estava sob o cursor.

## Accessibility

Preservar shape + color + text label, nunca somente cor; radius configurável; feedback do target kind; guides legíveis e Reduced Motion.

Quando um target visual-only não puder executar uma ação persistente, explicar o motivo em vez de falhar silenciosamente.

## Migração recomendada

1. tornar `inference.rs` o engine canônico;
2. introduzir proveniência semântica em SnapHit;
3. adicionar ObjectId/world transforms às prepared sources;
4. integrar Spline targets;
5. conectar Workplane diretamente ao grid/guides;
6. mover BVH/helpers neutros para Geometry;
7. compartilhar prepared spatial data com picking/surface raycast;
8. migrar Poly Pen e Bezier Pen;
9. migrar SurfaceAttachment para ObjectId + topology revision;
10. remover engine legado `snap.rs` e `SplineSnapSettings` após migração.

## Regra anti-bloat

Um engine de inferência, um sistema de spatial queries e um contrato de attachment.

Não criar engines separados por workspace ou por ferramenta.


## Decisões fechadas

1. `inference.rs` é o engine canônico de snapping/inference.
2. `snap.rs` deixa de ser engine; apenas helpers matemáticos úteis sobrevivem.
3. `SplineSnapSettings` e `snap_spline_position` serão aposentados.
4. Smart Snap é modo explícito, separado de Grid.
5. `SnapHit` carrega proveniência semântica.
6. Candidatos distinguem stable authoring targets de visual-only evaluated targets.
7. Workplane é a autoridade para Grid/Axis/Angle inference planar.
8. Spline participa do mesmo Inference Engine.
9. `SnapAccel` é preservado e evolui para carregar identidade.
10. BVH/picking/snap/surface raycast compartilham uma fundação espacial pequena.
11. `TriangleBvh` pertence conceitualmente a Geometry.
12. SurfaceAttachment usa `ObjectId`.
13. Attachment deixa de depender de `triangle_index` derivado e passa a referenciar o triângulo autoral explicitamente.
14. EditableMesh distingue revision geral de `topology_revision`.
15. `TopologyResult` também atualiza attachments quando o remap for inequívoco.
16. Alterações ambíguas levam a `NeedsReattach`; não há reprojeção silenciosa.
