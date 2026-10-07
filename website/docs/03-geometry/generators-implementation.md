# Generators — implementação detalhada

> **Status: proposta recomendada; complementa o redesign já aprovado.**

## Diagnóstico real da implementação

A base geométrica possui partes boas, mas o produto está fragmentado:

- PrimitiveDescriptor cobre 10 espécies e os builders são reutilizáveis;
- primitivas persistem hoje como `Mesh + parametric descriptor`;
- qualquer edição destrutiva "congela" o descriptor;
- Extrude e Revolve são geração direta a partir do ProfileState transitório;
- Sweep é o único PathGenerator persistente;
- PathGenerator só possui Sweep;
- o cache atual guarda uma única evaluation;
- Sweep expõe vários parâmetros algorítmicos que não deveriam ser parâmetros de produto;
- a UI e o lifecycle dos quatro caminhos são diferentes.

Classificação:

```text
primitive geometry algorithms → REUSE
sweep/frame algorithms        → REUSE
current storage/lifecycle      → REDESIGN
ProfileState integration       → REMOVE
PathGenerator container        → REPLACE
```

## GeneratorKind alvo

Começar somente com quatro famílias:

```rust
pub enum GeometryGeneratorKind {
    Primitive(PrimitiveGenerator),
    Extrude(ExtrudeGenerator),
    Revolve(RevolveGenerator),
    Sweep(SweepGenerator),
}
```

Não adicionar framework genérico maior antes desses quatro funcionarem de ponta a ponta.

## Uma única fonte de verdade

Objeto paramétrico:

```text
SceneObject
└── ObjectGeometry::Generator(GeneratorId)
```

A Mesh gerada é derivada/cache.

Remover o modelo atual:

```text
asset.mesh
+
asset.parametric
```

## Primitive Generator

### Algoritmos a preservar

Preservar:
- Box;
- Plane Mesh;
- Wedge;
- Cylinder/Frustum/Cone;
- Torus;
- Low Sphere;
- Icosphere;
- Capsule;
- helpers como radial_frustum e midpoint cache;
- primitive audit adaptado ao novo strict validation.

### Circle muda de categoria

O Circle atual gera uma Mesh plana. Na arquitetura shape-first, o Circle de criação principal deve gerar:

```text
Spline
→ PlanarShape
```

sobre o Workplane ativo.

O builder de disc/ring Mesh pode permanecer como helper ou comando explícito de "Mesh Disc" se houver necessidade, mas não deve competir com a ferramenta Circle shape-first.

### Plane

Manter `Plane Mesh` porque uma superfície poligonal editável é útil.

Separadamente, Rectangle/Plane Shape cria PlanarShape.

A UI deve deixar a intenção clara sem exigir que o usuário entenda a distinção interna.

## Primitive parameters

Não fazer clamp silencioso dentro de `build()`.

Hoje vários builders fazem:

```text
radius.clamp(...)
segments.clamp(...)
```

No modelo novo:

```text
UI constraints
    ↓
Command validation
    ↓
validated parameters
    ↓
evaluation
```

Valores inválidos retornam `GeneratorError`.

Clamping só é aceitável ao migrar formato legado conhecido, com report.

## Extrude

Input correto:

```text
PlanarShape ObjectId
```

porque Extrude opera sobre uma região preenchida, inclusive com holes.

Contrato conceitual:

```rust
pub struct ExtrudeGenerator {
    pub shape_object: ObjectId,
    pub distance: Length,
    pub cap_start: bool,
    pub cap_end: bool,
}
```

A direção inicial é a normal do Workplane do PlanarShape.

Custom direction pode entrar depois se houver necessidade real.

### Holes

Extrude deve suportar holes desde o primeiro vertical slice.

O caminho atual via region_sheet + extrusion possui algoritmos aproveitáveis.

### Signed distance

Distance pode ser assinada para permitir extrusão em ambos os sentidos sem outro parâmetro booleano.

## Revolve

Revolve não deve exigir PlanarShape fechado.

Input primário:

```text
planar Spline ObjectId
```

Isso representa melhor o lathe clássico:

```text
open profile curve
      ↓
revolve around axis
      ↓
surface/solid
```

Contrato conceitual:

```rust
pub struct RevolveGenerator {
    pub profile_object: ObjectId,
    pub axis: RevolutionAxis,
    pub angle: Angle,
    pub segments: SegmentCount,
    pub cap_start: bool,
    pub cap_end: bool,
}
```

### Axis

Não esconder "local Y" como regra implícita.

Suportar inicialmente:
- Workplane horizontal axis;
- Workplane vertical axis;
- explicit local axis/line.

Custom arbitrary line pode entrar quando a UI de axis picking estiver pronta.

### Profile validation

O profile precisa ser:
- Spline planar;
- pelo menos 2 pontos;
- finito;
- sem self-intersection incompatível com o algoritmo.

Closed spline pode ser aceita quando produzir resultado válido, mas não é requisito.

## Sweep

Inputs:

```text
PlanarShape ObjectId
+
Spline path ObjectId
```

Contrato:

```rust
pub struct SweepGenerator {
    pub profile_object: ObjectId,
    pub path_object: ObjectId,
    pub orientation: SweepOrientation,
    pub cap_start: bool,
    pub cap_end: bool,
}
```

Primeira orientação:

```text
ParallelTransport
```

Reutilizar o algoritmo de RMF/parallel transport atual.

### O que sai dos parâmetros de produto

Parâmetros atuais como:
- profile_spacing;
- path_spacing;
- tolerance;
- preview_spacing_multiplier;
- vertex_budget;
- miter_limit;

não devem aparecer todos como authoring state do usuário.

Eles pertencem principalmente a:

```text
GeneratorEvaluationPolicy
```

O usuário controla forma, não detalhes internos de tesselação.

## Evaluation Policy

Separar parâmetros autorais de política de avaliação:

```rust
pub struct GeneratorEvaluationPolicy {
    pub quality: EvaluationQuality,
    pub vertex_budget: VertexBudget,
    pub curve_tolerance: CurveTolerance,
}
```

Valores reais podem vir de preferências/performance tier.

## Preview e Final

```text
Preview
→ interação

Final
→ idle/commit/export/Make Editable
```

A qualidade muda resolução/tesselação, não intenção geométrica.

Se Preview exceder budget:
- reduzir resolução de forma controlada;
- emitir diagnostic.

Final acima do hard budget:
- falhar explicitamente;
- nunca truncar mesh silenciosamente.

## GeneratorEvaluation

Contrato comum:

```rust
pub struct GeneratorEvaluation {
    pub mesh: Mesh,
    pub diagnostics: GeneratorDiagnostics,
    pub source_revisions: SourceRevisions,
}
```

Opcionalmente, preparar um `GeneratedSurfaceMap` pequeno para material/UV provenance quando houver necessidade concreta.

Não criar provenance graph completo agora.

## Cache

Cache por generator, não uma única entrada global:

```text
GeneratorId
+ generator revision
+ source object revisions
+ source geometry revisions
+ transforms relevantes
+ quality
```

Armazenamento inicial simples:

```text
HashMap<GeneratorEvaluationKey, GeneratorEvaluation>
```

LRU apenas se profiling justificar.

## Dependencies

Cada generator declara ObjectIds fonte.

```text
Primitive
→ []

Extrude
→ [shape_object]

Revolve
→ [profile_object]

Sweep
→ [profile_object, path_object]
```

Não criar dependency graph genérico.

Application pode manter índice reverso simples:

```text
ObjectId → GeneratorIds dependentes
```

para invalidação.

## Coordinate spaces

Sources podem ter transforms diferentes.

Evaluation resolve:

```text
source local
→ source world
→ inverse(generated_object world)
→ generated local
```

Portanto generator output sempre pertence ao espaço local do SceneObject gerado.

## Creation Session

Os quatro generators usam a mesma gramática de criação:

```text
Begin
→ create SceneObject + Generator
→ live Preview
→ edit parameters
→ Confirm = 1 Undo transaction
→ Esc = remove object/generator and restore prior selection
```

Depois de confirmado, o generator continua paramétrico e editável no Inspector.

Isso substitui a diferença atual em que Primitive possui Last Operation, Extrude/Revolve viram Mesh imediatamente e Sweep vive em outro subsystem.

## Post-creation editing

Selecionar objeto gerado mostra seus parâmetros no Inspector.

Alterar parâmetro:

```text
Command
→ generator revision++
→ invalidate evaluation cache
→ preview/final evaluation
→ one undo step for committed edit
```

Não regenerar gravando uma Mesh autoral paralela.

## Direct modeling boundary

Ao iniciar edição Vertex/Edge/Face em objeto gerado:

```text
This geometry is parametric.
Make Editable?
```

Ação explícita:

```text
Make Editable
→ evaluate Final
→ replace ObjectGeometry::Generator with ObjectGeometry::Mesh
→ remove generator if no other owner/reference
→ one Undo
```

Não congelar parametric state silenciosamente no dispatcher como ocorre hoje.

## Modifiers

Generator pode alimentar Modifier Stack:

```text
Generator
→ Base Evaluation
→ Modifiers
→ Render
```

Make Editable inicialmente bakeia generator + modifiers ativos, conforme decisão já aprovada.

## UV por generator

FaceCorner deve ser emitido corretamente durante generation.

### Primitive
- Box/Wedge: deterministic planar charts;
- Cylinder/Cone: side strip + caps;
- Torus: U major angle / V minor angle;
- Sphere/Capsule: longitude/latitude style seam;
- Icosphere: deterministic projection ou xatlas fallback explícito.

### Extrude
- caps: PlanarShape/workplane projection;
- sides: U pelo comprimento do contour, V pela distância de extrusão;
- holes: cada boundary recebe strip própria.

### Revolve
- U: ângulo da revolução;
- V: distância ao longo do profile.

### Sweep
- U: distância/perímetro ao longo do profile;
- V: arc length do path.

Não chamar unwrap genérico para casos com parametrização conhecida.

## Materials

Primeira versão:
- um material herdado do SceneObject é suficiente;
- geradores podem expor slots `Caps` e `Sides` somente quando houver UX concreta.

Evitar criar sistema de material provenance complexo antecipadamente.

## Normals

Normals continuam derivadas.

Generators devem produzir:
- winding consistente;
- topology válida;
- crease/shading hints somente se realmente necessários.

Não persistir custom normals no generator.

## Validation

Toda evaluation termina em:

```text
generate
→ validate_invariants
→ diagnostics
→ cache
```

Nunca em `mesh.validate()` mutante.

## Diagnostics

Exemplos:
- source missing;
- source incompatible;
- profile not planar;
- self-intersection;
- empty result;
- budget exceeded;
- cap impossible;
- degenerate axis;
- path too short.

Um generator inválido permanece no documento com status de erro; não desaparece nem mantém stale mesh fingindo sucesso.

## Vertical slice 1 — Primitive

Objetivo: provar ObjectGeometry::Generator sem dependencies.

Gates:
- create/save/load;
- Inspector edit;
- Preview/Final;
- Render/Picking/Export;
- modifier after generator;
- Make Editable;
- Undo/Redo;
- FaceCorner UV;
- strict validation.

## Vertical slice 2 — Extrude

Prova:
- ObjectId dependency;
- PlanarShape + holes;
- source transform;
- dependency invalidation;
- caps/sides UV;
- source edit updates generated object.

## Vertical slice 3 — Revolve

Prova:
- planar Spline source;
- explicit axis;
- partial angle;
- seam/caps;
- segment count;
- open and closed profile cases.

## Vertical slice 4 — Sweep

Só depois migrar Sweep.

Reusar:
- RMF/parallel transport;
- arc-length sampling;
- existing sweep mesh generation;
- diagnostics úteis.

Remover:
- PathGenerator-only architecture;
- one-entry cache;
- ProfileResource legacy dependency;
- UI algorítmica de spacing/tolerance.

## Test matrix comum

Para cada generator:
- deterministic same input → same mesh;
- no NaN/Inf;
- strict invariants;
- save/load;
- undo/redo;
- source transform;
- source deletion/error state;
- Preview budget;
- Final budget;
- FaceCorner UV finite;
- materials preserved;
- render/picking agreement;
- export;
- Make Editable equivalence.

## Migração

1. implementar novo GeometryGenerator/GeneratorId;
2. migrar PrimitiveDescriptor para PrimitiveGenerator;
3. eliminar `Asset::parametric` e primitive mesh duplication;
4. implementar ExtrudeGenerator;
5. implementar RevolveGenerator;
6. migrar PathGenerator::Sweep;
7. substituir ProfileState dependencies por PlanarShape/Spline;
8. remover PathGenerator container/cache antigos;
9. remover freeze_parametric silencioso;
10. atualizar UI/MCP/plugins/serialization.

## Regra anti-bloat

Generators existem onde preservar a fonte autoral traz valor evidente.

V1 fica em:

```text
Primitive
Extrude
Revolve
Sweep
```

Não adicionar Loft, Boolean history, Bevel generator, node graph ou modifier-generator híbrido antes desses quatro estarem sólidos.
