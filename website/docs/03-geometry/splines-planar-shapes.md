# Splines e Planar Shapes

> **Status: aprovado.**

## Autoridade de curvas

`SplineResource` é a única representação autoral persistente de curvas.

Ela já fornece:
- Polyline e Cubic Bézier;
- pontos persistentes com ID;
- handles Broken / Aligned / Mirrored;
- split por De Casteljau;
- avaliação e tangentes;
- arc-length/resample;
- attachments de superfície;
- revision/fingerprint;
- serialização e validação.

## BezierPath legado

`petunia_mesh::curve::BezierPath`, `BezierNode` e `BezierNodeKind` não devem permanecer como um segundo modelo autoral.

Algoritmos úteis podem ser migrados para avaliação/edição de `SplineResource`; o modelo duplicado deve ser aposentado.

## Bezier Pen

Bezier Pen edita diretamente `SplineResource`.

Fluxo:

```text
pointer gesture
    ↓
BezierPenSession
    ↓
commands
    ↓
SplineResource
```

Não haverá pipeline intermediário `ProfileState → BezierPath → SplineResource`.

### Posicionamento inicial

A primeira implementação suporta:
- **Workplane** — ray × plano;
- **Surface** — picking de superfície + `SurfaceAttachment`.

Free-air 3D sem referência espacial não é requisito inicial por ser inerentemente ambíguo com input 2D.

## Workplane

`ProfileWorkplane` evolui para `Workplane`, um contrato geométrico geral.

Ele não pertence exclusivamente ao DRAW/Profile.

É reutilizado por:
- Bezier Pen;
- Poly Pen;
- formas planas;
- primitivas;
- snapping;
- Shape Builder;
- transforms;
- medidas.

### Ownership

`EditorSession.active_workplane` é transitório.

Shapes persistentes podem armazenar o Workplane no qual foram autorados.

## ProfileResource → PlanarShape

O conceito de `ProfileResource` evolui para `PlanarShape`.

```rust
pub struct PlanarShape {
    pub id: PlanarShapeId,
    pub name: String,
    pub outer: SplineId,
    pub holes: Vec<SplineId>,
    pub workplane: Workplane,
}
```

PlanarShape é uma forma preenchida planar composta por Splines.

## Holes

O formato atual:

```text
outer → SplineResource
holes → Vec<Vec<[f64; 2]>>
```

é assimétrico e deve ser removido.

Todos os contornos passam a ser Splines persistentes, permitindo edição Bézier e identidade consistente para outer e holes.

## Wall Thickness

`wall_thickness` deixa de ser propriedade fundamental da forma.

A operação de thickness/offset deve:
- produzir explicitamente um inner contour/hole; ou
- futuramente pertencer a um generator específico quando houver edição paramétrica real.

A forma resultante não deve depender de um parâmetro mágico escondido.

## Primitivas planares

Rectangle, Circle/Ellipse, Polygon, Arc etc. são ferramentas de criação de Spline/PlanarShape.

Um descriptor paramétrico pequeno pode existir enquanto a forma permanecer não editada manualmente; uma edição arbitrária pode invalidá-lo e transformar o resultado em spline comum.

## Poly Pen × Bezier Pen

Não unificar a implementação geométrica.

- Poly Pen edita Mesh.
- Bezier Pen edita Spline.

Compartilham:
- workplane;
- snapping;
- inference;
- pointer grammar;
- numeric input;
- ToolSession lifecycle.
