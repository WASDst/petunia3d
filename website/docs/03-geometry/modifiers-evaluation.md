# Modifiers e Geometry Evaluation

> **Status: proposta recomendada; aguarda aprovação.**

## Diagnóstico atual

A stack existente possui apenas:
- Mirror;
- Symmetry.

Ela tem boas propriedades:
- é persistente;
- ordenada;
- reordenável;
- pode ser desabilitada;
- não altera a malha-base;
- export já usa o resultado avaliado.

Os principais problemas não estão nos dois algoritmos, mas no pipeline:

- `Asset::evaluated_mesh*` assume que todo objeto possui uma Mesh-base;
- cache vive dentro do objeto persistente;
- invalidação usa hashing integral de conteúdo;
- Pose/Skin ignora assets com modifiers ativos;
- SurfaceAttachment usa diretamente `asset.mesh`, não a geometria visível avaliada;
- Geometry Source, Generator e Modifier ainda não compartilham uma porta única de avaliação.

## Decisão recomendada

Manter uma **Modifier Stack pequena e explícita**.

Não transformar modifiers em:
- node graph;
- plugin framework;
- generator genérico;
- modifier stack estilo Blender com dezenas de operadores.

Mirror e Symmetry são a base inicial.

## Pipeline alvo

```text
ObjectGeometry
     ↓
Base Geometry Evaluation
     ↓
Modifier Stack
     ↓
Modified Mesh
     ↓
Skin / Pose deformation
     ↓
Display / Picking / Export
```

Transform de SceneObject continua fora da geometria local:

```text
local evaluated geometry
        ↓
SceneObject world transform
        ↓
renderer / world queries
```

## Geometry Source

Modifiers não conhecem a origem da geometria.

Recebem uma Mesh e devolvem uma Mesh:

```rust
evaluate_modifier(mesh, modifier) -> Result<Mesh, ModifierError>
```

Portanto funcionam igualmente sobre:
- EditableMesh;
- Primitive Generator;
- Extrude Generator;
- Revolve Generator;
- Sweep Generator;

desde que a Geometry Source produza uma surface Mesh.

Spline ou PlanarShape não recebem modifier de mesh diretamente enquanto não houver um caso de produto comprovado.

## Modifier model

Continuar com enum serializável e explícito:

```rust
pub struct ModifierInstance {
    pub id: ModifierId,
    pub enabled: bool,
    pub kind: ModifierKind,
}

pub enum ModifierKind {
    Mirror(MirrorModifier),
    Symmetry(SymmetryModifier),
}
```

Não criar trait/plugin API para modifiers internos nesta etapa.

## Mirror e Symmetry

Classificação:
- algoritmos: **REUSE + adaptação**;
- integração/cache/pipeline: **REFACTOR**.

A migração para FaceCorner deve garantir:
- vertex colors preservadas;
- material slots preservados;
- UV por corner preservada;
- seams remapeadas quando aplicável;
- pins remapeados quando semanticamente válido;
- topology result/remap explícito quando necessário.

## Cache

Cache sai do SceneObject/Document.

Chave mínima conceitual:

```text
ObjectId
+ Geometry Source revision/dependencies
+ Modifier Stack revision
+ Evaluation Quality
```

Não recalcular hash integral de todos os vertices/faces por frame.

Cache pertence ao Geometry Evaluation Service da sessão.

## Geometry Evaluation Service

Render, picking, export, viewport queries e ferramentas não devem decidir individualmente como obter "a mesh certa".

Uma porta única resolve a geometria.

Conceitualmente:

```rust
evaluate_surface(
    object: ObjectId,
    quality: EvaluationQuality,
) -> Result<EvaluatedSurface, GeometryEvaluationError>
```

O contrato final pode oferecer estágios explícitos quando uma ferramenta precisar da malha-base versus modificada.

## Estágios

Necessidades reais justificam três conceitos:

### Base
Resultado direto da ObjectGeometry.

### Modified
Base + Modifier Stack.

### Deformed
Modified + pose/skin transitório.

Não serializar resultados desses estágios.

## Component editing

Quando `ObjectGeometry::Mesh` possui modifiers:

- Vertex/Edge/Face edit atua na EditableMesh autoral;
- modifier result aparece como preview;
- seleção continua referindo componentes da base;
- regiões criadas somente pelo modifier não recebem identidade autoral.

Não implementar provenance/picking reverso de componentes gerados na primeira versão.

Se o usuário precisar editar a geometria resultante:
→ **Apply/Make Editable**.

## Generator + Modifier

Permitido:

```text
Extrude Generator
      ↓
Mirror
      ↓
render
```

A fonte continua paramétrica.

### Make Editable
Avalia:
- generator;
- todos os modifiers ativos;

e substitui por uma única EditableMesh.

```text
Generator + Modifier Stack
          ↓
      Make Editable
          ↓
      EditableMesh
```

Não tentar "aplicar só Mirror mantendo Extrude paramétrico" na primeira arquitetura. Isso exigiria um estágio intermediário persistente e aumentaria muito a complexidade.

## Apply modifiers em EditableMesh

A operação mínima necessária inicialmente é **Apply All Modifiers**.

Aplicação individual pode ser adicionada depois se uso real justificar.

Apply All:
1. avalia Modified;
2. substitui EditableMesh;
3. limpa stack;
4. registra um único Undo.

## UV manual

EditableMesh com Mirror/Symmetry pode manter UV manual **somente porque esses modifiers possuem regra determinística de propagação de FaceCorner/UV**.

Novos modifiers não ganham automaticamente essa garantia.

Se um futuro modifier não conseguir preservar UV de forma confiável, ele deve:
- invalidar explicitamente UV; ou
- exigir Apply/reunwrap;
nunca manter dados silenciosamente errados.

## Paint

Paint raster baseado em UV pode coexistir com Mirror/Symmetry quando a UV avaliada é preservada deterministicamente.

Paint por topologia/vertex data exige compatibilidade explícita.

Não criar framework genérico de capability. Cada novo modifier deve documentar sua política de atributos antes de entrar no produto.

## Skin / Rig

O bug atual — assets com modifiers simplesmente não serem deformados — deve desaparecer.

Regra inicial:

> Skin binding exige topologia autoral estável.

Portanto modifiers que alteram a quantidade/conectividade de vertices devem ser aplicados antes de criar/bindar SkinData.

Se um objeto já possui SkinData:
- adicionar Mirror/Symmetry topológico é recusado com mensagem clara; ou
- UI oferece Apply/Make Editable antes.

Depois disso:

```text
EditableMesh
   ↓
topology-compatible modifiers (se existirem futuramente)
   ↓
Skin/Pose
```

Não criar Armature Modifier; rigging permanece um domínio próprio.

## SurfaceAttachment

Na primeira arquitetura, attachment persistente exige alvo `ObjectGeometry::Mesh`.

Ele ancora na topologia autoral, não em triangles transitórios criados por Generator/Modifier.

Consequências:
- target Generator → requer Make Editable para attachment persistente;
- região criada apenas por Mirror/Symmetry não recebe attachment persistente;
- transforms do SceneObject continuam acompanhados sem invalidar topologia;
- alteração topológica da EditableMesh usa revision/fingerprint/remap.

Essa restrição evita IDs transitórios de faces avaliadas.

## Picking

### Object picking
Usa a geometria Modified/Deformed visível.

### Component picking
Em edit mode usa a EditableMesh autoral.

Isso mantém coerência entre o que é selecionável persistentemente e o que possui identidade de componente.

## Export

Export estático usa Modified.

Export de rig/animation usa a geometria compatível com Skin/Pose.

Se existir incompatibilidade topológica persistente, export deve falhar com diagnóstico; nunca descartar Skin silenciosamente.

## PoseOverride

O conceito de pose transitória é bom e deve ser preservado, mas deixa de substituir arbitrariamente `asset.evaluated_mesh()`.

O Geometry Evaluation Service passa a coordenar:

```text
Base
→ Modifiers
→ Skin/Pose
```

PoseOverride permanece transitório e fora de Undo/serialization.

## Modifier revision

A stack deve possuir revision explícita ou avançar a revision do SceneObject quando:
- modifier é adicionado/removido;
- ordem muda;
- enabled muda;
- parâmetro muda.

Isso substitui `modifier_hash()` como mecanismo principal de invalidação.

## Future modifiers

Candidatos como Array, Radial Array ou deformers só entram quando houver necessidade de produto clara.

Todo novo modifier precisa responder antes:
1. muda topologia?
2. preserva FaceCorner/UV?
3. preserva material?
4. é compatível com Skin?
5. é compatível com SurfaceAttachment?
6. qual o custo de preview?

Não é necessário criar um type-system complexo para isso; a política pode começar codificada e testada por variant.

## Migração recomendada

1. introduzir Geometry Evaluation Service;
2. mover cache para DocumentSession/Application;
3. adaptar Mirror/Symmetry para FaceCorner/newtypes;
4. trocar hashing integral por revisions;
5. fazer render/picking/export consumirem a porta única;
6. integrar Generator output;
7. definir Base/Modified/Deformed;
8. corrigir Skin/Pose;
9. adaptar SurfaceAttachment às novas regras;
10. remover `Asset::evaluated_mesh*`, `modifier_hash` e `eval_cache`.

## Regra anti-bloat

A stack existe para algumas transformações não destrutivas de alto valor.

Ela não é o centro do modelador e não deve absorver:
- booleans;
- Shape Builder;
- Knife;
- Bevel;
- Loop Cut;
- generators;
- rig;
- animation.

Esses continuam em seus domínios naturais.
