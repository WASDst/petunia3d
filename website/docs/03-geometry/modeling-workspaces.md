# Workspaces de modelagem

> **Status: aprovado.**

DRAW deixa de ser um motor geométrico separado.

DRAW e POLY são **presets de apresentação e navegação do mesmo Modeling Engine**, com ferramentas e representações compartilhadas. Ativar o preset planar é explícito; retornar à vista livre preserva documento e seleção.

## Princípio

```text
Modeling Engine
├── Mesh
├── Splines
├── Workplane
├── Poly Pen
├── Bezier Pen
├── Primitives
├── Knife
├── Push/Pull
├── Shape Builder
├── Booleans
└── topology operations
```

Os presets podem priorizar a apresentação, sem restringir capacidades:
- ferramentas priorizadas;
- navegação;
- projeção;
- overlays;
- defaults;
- layout.

Eles não determinam um segundo modelo geométrico.

## DRAW

DRAW torna-se um perfil planar:

- projeção ortográfica;
- câmera normal ao workplane;
- pan e zoom ativos;
- orbit livre desabilitado;
- troca rápida entre planos;
- alinhamento a face/plano local;
- apresentação orientada a shapes e curvas.

## POLY

POLY usa:
- orbit livre;
- perspectiva/ortográfica;
- overlays topológicos;
- edição direta de componentes;
- o mesmo workplane quando ferramentas precisarem.

## Workplane global

Workplane deixa de ser conceito exclusivo de DRAW.

Ele é usado por:
- Poly Pen;
- Bezier Pen;
- Circle;
- Rectangle/Plane;
- Polygon;
- Arc;
- primitives;
- snapping;
- transformações;
- medidas;
- Shape Builder.

## Curvas persistentes

Unificação de workspaces não implica converter curvas imediatamente em mesh.

Spline/Profile continuam sendo recursos persistentes quando oferecem valor paramétrico para:
- edição de handles;
- extrude;
- revolve;
- sweep;
- paths;
- hair/guides futuros;
- animação futura.

O `ProfileState` transitório duplicado deve desaparecer gradualmente em favor dos recursos persistentes e das Tool Sessions.

## Selection

Não haverá segunda taxonomia global exclusiva de DRAW.

Mesh mantém Object / Vertex / Edge / Face como identidades de alvo e filtros. O fluxo contextual aprovado dispensa trocar obrigatoriamente entre modos globais; sua implementação é rastreada em H22.

Spline e outros objetos podem oferecer componentes contextuais próprios sem criar outro modo global concorrente.
