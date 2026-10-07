# Object e Geometry Source

> **Status: proposta recomendada; aguarda aprovação.**

## Diagnóstico atual

O atual `Asset` acumula responsabilidades diferentes:

- objeto/instância de cena;
- owner de `Mesh`;
- transform;
- visibility/lock/collection;
- material e textura;
- Paint stack;
- rig/skin;
- modifier stack;
- primitive descriptor paramétrico;
- evaluation cache.

Além disso, o termo Asset também é usado para biblioteca/reutilização, criando ambiguidade entre conteúdo reutilizável e objeto colocado na cena.

## Direção recomendada

Renomear conceitualmente o atual `Asset` para `SceneObject`.

Na UI, o termo visível pode continuar sendo **Part** quando isso for mais natural.

Reservar **Asset** para conteúdo reutilizável/importado/biblioteca.

## Geometry Source

Um SceneObject possui uma única fonte geométrica autoral:

```rust
pub enum ObjectGeometry {
    Mesh(EditableMesh),
    Spline(SplineId),
    PlanarShape(PlanarShapeId),
    Generator(GeneratorId),
}
```

A forma final dos nomes/tipos será congelada somente após a implementação dos primeiros vertical slices.

### EditableMesh

```rust
pub struct EditableMesh {
    pub mesh: Shared<Mesh>,
    pub revision: Revision,
}
```

A revisão explícita substitui hashing integral da mesh como mecanismo principal de invalidação.

## Uma fonte de verdade

Não manter:

```text
Mesh + PrimitiveDescriptor
Mesh + Generator
```

como duas autoridades simultâneas.

Objeto paramétrico:

```text
SceneObject
└── ObjectGeometry::Generator
```

Objeto convertido:

```text
SceneObject
└── ObjectGeometry::Mesh
```

## Make Editable

```text
Generator
   ↓ evaluate Final
Mesh
   ↓ replace geometry source
EditableMesh
```

Após a conversão, a relação paramétrica é removida de forma explícita.

## Evaluation

A avaliação não pertence ao objeto persistente.

```text
ObjectGeometry
     ↓
Geometry Evaluation
     ↓
optional modifiers
     ↓
optional deformation/pose
     ↓
render / picking / export
```

Caches pertencem à sessão/evaluation service e são indexados por IDs + revisions.

## Geometry não-mesh

Spline e PlanarShape não devem ser artificialmente convertidos em Mesh somente para existir na cena.

Render/picking podem receber representações derivadas próprias para:
- curve lines;
- control points;
- planar fill/outline.

OBJ/glTF exportam somente objetos que resolvem para surface Mesh. Export vetorial é um caminho separado.

## Topology-bound features

Para evitar remapping complexo na primeira arquitetura:

### Permitido em Generator
- material;
- transform;
- auto UV produzida pelo generator;
- preview/render/export da mesh avaliada.

### Exige Make Editable inicialmente
- Paint raster/vertex vinculado à topologia;
- edição UV manual persistente;
- skin weights/rig binding por vertex;
- edição direta Vertex/Edge/Face.

Essa regra pode ser relaxada futuramente somente quando existir remapping robusto comprovado.

## Selection

Seleção de objeto passa a usar `ObjectId`, não `AssetId`.

O termo `AssetId` fica reservado para conteúdo reutilizável quando necessário.

Component selection continua contextual:
- Mesh → Vertex / Edge / Face;
- Spline → points/segments através da ferramenta/contexto de spline;
- PlanarShape → edição de seus contours/splines.

Não criar uma segunda taxonomia global de DRAW.

## SceneObject mínimo

Direção:

```rust
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub geometry: ObjectGeometry,
    pub transform: Transform,
    pub visible: bool,
    pub locked: bool,
    // appearance / paint / rig bindings permanecem separados por capacidade.
}
```

Durante a migração, material/Paint/rig podem permanecer temporariamente no mesmo struct para reduzir churn; a separação física pode ocorrer nos respectivos capítulos.

## Document

Direção conceitual:

```text
Document
├── SceneObjects
├── Splines
├── PlanarShapes
├── GeometryGenerators
├── Materials
├── Skeletons
├── Animation
└── other authored resources
```

`active` object e selection não pertencem ao Document; pertencem ao `EditorSession`.

## Coordinate space

Regra recomendada:

- geometry autoral é armazenada em espaço local;
- `SceneObject.transform` é a autoridade para local → world;
- workplane persistente de um PlanarShape é relativo ao espaço autoral/local;
- workplane ativo do EditorSession é transitório em espaço de edição/world;
- generators que dependem de outros objetos devem resolver explicitamente transforms entre source e generated object.

Não permitir transform implícito duplicado em Mesh + Profile/Generator.

## Modifiers

Modifiers são ortogonais à fonte geométrica.

Não decidir neste capítulo se a stack atual permanece como produto.

Enquanto existir:

```text
Geometry Source
    ↓
base evaluated mesh
    ↓
Modifiers
    ↓
final evaluated mesh
```

Nunca fazer Modifier virar outro Geometry Source.

## Migração pragmática

1. Introduzir IDs/tipos e `ObjectGeometry`.
2. Migrar primitive parametric para Generator.
3. Fazer render/picking/export consultarem um único GeometryEvaluation service.
4. Mover evaluation cache para DocumentSession/Application.
5. Migrar Profiles → PlanarShapes e conectar objetos de cena.
6. Remover `Asset::parametric`, `Asset::eval_cache` e APIs duplicadas.
7. Renomear `Asset` → `SceneObject` após consumidores estarem migrados.
8. Remover `Project.active` da persistência e usar EditorSession.

## Regra anti-bloat

Não criar store/graph genérico universal para todos os tipos de recurso antes de haver necessidade concreta.

O objetivo é uma fonte geométrica explícita por objeto e uma única porta de avaliação, não um ECS nem um scene graph abstrato.
