# Redesign de Generators

> **Status: aprovado — conceito preservado, implementação atual classificada como insuficiente e sujeita a redesign.**

## Diagnóstico atual

A implementação existente não é uma base completa de generators.

Hoje:
- `PathGeneratorKind` possui apenas `Sweep`;
- Extrude e Revolve existem por caminhos separados e não fazem parte do mesmo sistema persistente;
- `PathGeneratorEvaluationCache` guarda apenas uma avaliação;
- há histórico de bugs de avaliação, workplane e curvas;
- invalidation/dependency tracking é incompleto;
- alguns caminhos existem principalmente em testes/comandos, sem integração de produto robusta;
- o sistema ainda depende de estruturas legadas de Profile/DRAW.

Portanto a classificação não é **REUSE**.

É:

> **REUSE ALGORITHMS / REDESIGN ARCHITECTURE.**

## Por que generators continuam fazendo sentido

Generators são valiosos somente onde preservam authoring de alto nível de forma clara.

Casos iniciais legítimos:

```text
PlanarShape → Extrude → Mesh
PlanarShape → Revolve → Mesh
PlanarShape + Spline → Sweep → Mesh
Primitive Parameters → Primitive Mesh
```

Não transformar toda modelagem do Petunia3D em procedural graph.

## Fronteira

### Direct modeling

Permanece destrutivo/direto quando essa é a experiência natural:

- Poly Pen;
- Knife;
- Bevel;
- Inset;
- mesh-face Extrude;
- Push/Pull sobre mesh;
- Shape Builder após commit;
- topology cleanup.

### Parametric generation

Usado quando existe uma fonte autoral claramente útil para reeditar:

- PlanarShape Extrude;
- PlanarShape Revolve;
- Sweep;
- primitives paramétricas enquanto não convertidas para Mesh editável.

## Modelo alvo mínimo

Evitar hierarquia complexa.

Conceitualmente:

```rust
pub struct GeometryGenerator {
    pub id: GeneratorId,
    pub name: String,
    pub kind: GeometryGeneratorKind,
    pub revision: Revision,
}

pub enum GeometryGeneratorKind {
    Extrude(ExtrudeGenerator),
    Revolve(RevolveGenerator),
    Sweep(SweepGenerator),
    Primitive(PrimitiveGenerator),
}
```

Os detalhes finais dos tipos só devem ser congelados após implementar os três primeiros vertical slices.

## Input por IDs explícitos

Generator não possui cópias escondidas das fontes.

Exemplos:

```text
Extrude
└── PlanarShapeId

Revolve
└── PlanarShapeId

Sweep
├── PlanarShapeId
└── SplineId
```

## Output

O output avaliado é derivado.

```text
authoring sources
      ↓
generator
      ↓
evaluation
      ↓
Mesh + diagnostics
```

A Mesh avaliada não é segunda fonte de verdade.

## Evaluation contract

Toda avaliação deve produzir algo semelhante a:

```rust
pub struct GeneratorEvaluation {
    pub mesh: Mesh,
    pub diagnostics: GeneratorDiagnostics,
    pub source_revisions: SourceRevisions,
}
```

Erros são explícitos; nenhum generator devolve silenciosamente uma mesh antiga quando a fonte atual não pode ser avaliada.

## Cache

Não repetir o cache de uma entrada atual.

O primeiro desenho deve ser simples:

- cache por GeneratorId;
- chave por revision do generator + revisions/fingerprints das fontes;
- Preview e Final podem ter entradas distintas;
- invalidação dirigida por dependency changes;
- orçamento pequeno/configurável;
- LRU somente se profiling demonstrar necessidade.

## Preview e Final

Generators podem ter duas qualidades:

- Preview — mais barato, durante gesto/edição;
- Final — commit/export/idle refinement.

Mas a diferença não pode mudar semanticamente a forma, somente resolução/qualidade.

## UV

Generators conhecidos devem produzir UV previsível diretamente quando possível.

Com `FaceCorner`, Extrude/Revolve/Sweep podem emitir UV por corner no momento da geração, evitando unwrap genérico desnecessário.

Se uma superfície não possuir regra UV confiável, cair para projeção/xatlas explicitamente; nunca gerar UV inválida silenciosamente.

## Materiais e atributos

A avaliação deve declarar regras determinísticas para:
- material das caps;
- material das sides;
- vertex colors quando aplicável;
- UV;
- normals derivadas;
- source attribution útil para future remap.

## Bake / Make Editable

Todo generator precisa de saída explícita:

```text
Make Editable
    ↓
evaluate Final
    ↓
replace generator-backed object
with ordinary Mesh
```

Depois disso, nenhuma dependência paramétrica permanece.

## Undo

Editar parâmetros do generator é uma operação de documento.

A Mesh de preview/cache não entra separadamente no history.

## Falha e degradação

Não aceitar:
- stale cache silencioso;
- fallback para mesh incorreta;
- generator inválido que bloqueia o documento inteiro;
- dependência ausente sem diagnóstico identificável.

Um generator quebrado deve continuar representável no documento e exibir diagnóstico/recovery possível.

## Ordem de reconstrução

1. consolidar `SplineResource`, `Workplane` e `PlanarShape`;
2. implementar Extrude como primeiro vertical slice completo;
3. implementar Revolve sobre o mesmo contrato;
4. migrar/reimplementar Sweep usando algoritmos existentes úteis;
5. integrar cache/invalidation;
6. integrar UV/materials;
7. implementar Make Editable;
8. somente então considerar generators adicionais.

## Regra anti-bloat

Não criar node graph, modifier stack genérica ou framework abstrato antes de Extrude/Revolve/Sweep compartilharem necessidades concretas verificadas.

O sistema de generators deve permanecer pequeno, explícito e orientado ao fluxo shape-first.
