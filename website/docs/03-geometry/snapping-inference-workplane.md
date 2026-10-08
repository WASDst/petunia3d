# Snapping, Inference & Workplane

> **Status: proposta recomendada; aguarda aprovação.**

## Objetivo

Transformar snapping e inferência em uma infraestrutura transversal de modelagem, não em comportamento particular de Move ou DRAW.

O sistema deve servir igualmente a:

- Move / Rotate / Scale quando aplicável;
- Poly Pen;
- Bezier Pen;
- Line / Rectangle / Circle / Arc;
- Knife;
- Push/Pull;
- Extrude;
- placement de primitives;
- Shape Builder;
- Trace Mode;
- Reference alignment/calibration;
- futuros transform gizmos.

Regra:

> O cursor deve sempre dizer ao usuário onde a próxima ação acontecerá antes do clique.

---

## Diagnóstico atual

A implementação atual tem duas gerações coexistindo.

### Motor novo: `core::inference`

Este é o caminho tecnicamente correto.

Já possui:

- tolerância em pixels;
- prioridade determinística;
- Point;
- Midpoint;
- On Edge;
- Face Center;
- On Face;
- Intersection;
- Axis guides;
- Parallel;
- Perpendicular;
- Angle step;
- Grid;
- oclusão;
- X-Ray;
- `SnapAccel`;
- `TriangleBvh`;
- exclusão de geometria em movimento;
- suporte estrutural a múltiplas fontes via `SnapSource[]`.

Classificação:

```text
core::inference
→ STRONG REUSE
→ MOVE TO APPLICATION / VIEWPORT QUERY LAYER
```

A matemática e os testes devem ser preservados.

---

## Documentação antiga está defasada

A antiga gap matrix ainda registra como pendentes:

- Face Center;
- Intersection;
- Parallel;
- Perpendicular;
- 15°;
- multi-object;
- spatial acceleration.

O código atual já cobre grande parte disso.

Portanto essa documentação não deve orientar uma reimplementação.

---

## Motor antigo: `core::snap`

Também permanece um segundo sistema:

```text
SnapSettings
SnapTarget
SnapElement
SnapQuery
SnapResult
snap_point(...)
snap_point_to_grid(...)
snap_point_to_vertices(...)
snap_point_to_edges(...)
snap_point_to_faces(...)
```

Ele usa principalmente tolerância em unidades de mundo.

Isso conflita com a decisão já consolidada de snapping interativo em pixels.

Classificação:

```text
core::snap
→ LEGACY / CONSOLIDATE
```

Preservar apenas utilitários matemáticos puros que ainda tenham uso real.

Não manter dois motores públicos de snapping.

---

## Problema de estado duplicado

Hoje EditorSession possui conceitualmente:

```text
snap_enabled
+
SnapSettings.enabled
```

Isso cria duas autoridades para a mesma decisão.

Direção:

```text
EditorSession
└ SnapSessionSettings
   ├ enabled
   ├ radius_pixels
   ├ target policy
   ├ angular step
   └ temporary overrides
```

Uma única autoridade.

---

## Snap não deve ser persistência de documento

Configurações como:

- snap ligado/desligado;
- raio;
- targets;
- angular step;
- preferências de inferência;

são estado/preferência do editor.

Não pertencem ao Document.

---

## Smart Snap como padrão

A direção atual de `enabled = true` está correta.

Mas Smart Snap não deve significar "arredonde tudo para a grade".

Default:

```text
Smart Snap
├ existing points
├ midpoints
├ intersections
├ edges
├ inference guides
├ face
└ workplane grid as weak fallback
```

A grade é o último recurso.

O movimento livre continua livre quando nenhum alvo útil estiver suficientemente próximo.

---

## Hard Grid Snap separado

Quando o usuário realmente quiser quantização rígida:

```text
Ctrl / explicit Grid Lock
→ quantize movement
```

Não confundir:

```text
Smart Snap
≠
Hard Increment Snap
```

---

## Prioridade recomendada

Manter a filosofia atual, refinada:

```text
1. Exact geometric points
   ├ Point
   ├ Midpoint
   ├ Intersection
   └ Face Center

2. Edge

3. Inference guides
   ├ Axis
   ├ Parallel
   └ Perpendicular

4. Angular guide

5. Face

6. Grid
```

Dentro da mesma classe:

1. menor distância em pixels;
2. profundidade resolve empate.

---

## On Face

`OnFace` é diferente de Face Center.

```text
Face Center
→ ponto semântico

On Face
→ posição projetada sob o cursor
```

Ambos devem continuar existindo.

---

## Intersection

O algoritmo atual calcula interseção real entre segmentos 3D candidatos próximos.

Isso é adequado para V1.

Não transformar imediatamente em interseção infinita de linhas.

Se futuramente houver demanda:

```text
Intersection
├ Segment Intersection
└ Extended Guide Intersection
```

mas não precisa entrar agora.

---

## Inferência de direção

A infraestrutura atual de:

- Axis;
- Parallel;
- Perpendicular;
- Angle;

deve virar conceito de primeira classe.

Uma ferramenta fornece:

```text
anchor
reference direction
active workplane
allowed guide families
```

O motor fornece o melhor candidato.

A ferramenta não implementa matemática própria.

---

## Dynamic Guides

A UI deve desenhar uma guia somente quando ela realmente participa da inferência atual.

Exemplo:

```text
●─────────────── cursor
 Parallel
```

ou:

```text
●
│
│ Perpendicular
│
cursor
```

Evitar poluir a viewport com todas as linhas possíveis.

---

## Temporary Inference Lock

O usuário deve conseguir congelar a inferência atual.

Direção recomendada:

```text
Shift hold
→ lock current inference

Shift tap / accessible toggle
→ persistent lock until released
```

O estado aparece no HUD.

Importante para usuários que não conseguem manter modifier keys confortavelmente.

---

## Angle inference

15° continua um bom default.

Mas deve ser configuração:

```text
Angle Step
15°
```

Presets simples:

- 5°;
- 15°;
- 30°;
- 45°;
- 90°.

Não criar arbitrary constraint solver.

---

# Workplane

A arquitetura já aprovou que Workplane é global ao modelador, não específico de DRAW.

Hoje ainda existem:

```text
ProfileState
├ right
├ up
├ origin
├ normal
├ workplane_kind
└ workplane_locked
```

e:

```text
ProfileWorkplane
```

persistente em ProfileResource.

Isso deve convergir.

---

## Workplane value

Direção:

```rust
pub struct Workplane {
    pub origin: [f64; 3],
    pub right: [f64; 3],
    pub up: [f64; 3],
    pub normal: [f64; 3],
}
```

com invariantes:

- frame ortonormal;
- right-handed;
- valores finitos.

O atual `ProfileWorkplane::try_normalized` é boa base para REUSE.

---

## Workplane ownership

Existem dois contextos diferentes:

### Authoring resource

PlanarShape/Profile pode persistir o plano em que foi criado.

```text
PlanarShape
└ Workplane
```

### Editor active workplane

O editor também possui um plano ativo transitório:

```text
EditorSession
└ WorkplaneSession
```

Não são a mesma coisa.

Copiar o plano ativo para um PlanarShape no momento da criação é correto.

Mover depois o active workplane não deve mover shapes existentes.

---

## WorkplaneSession

Direção:

```rust
pub struct WorkplaneSession {
    pub mode: WorkplaneMode,
    pub plane: Workplane,
    pub locked: bool,
}
```

Conceitualmente.

---

## WorkplaneMode

```text
Auto
Ground
View
Face
Selection
Custom
Reference
```

### Auto

Escolhe contexto útil no momento da ação.

### Ground

Plano XZ.

### View

Plano orientado pela vista atual.

### Face

Face sob cursor/selecionada.

### Selection

Derivado de 3 pontos, face + edge ou faces paralelas.

### Custom

Plano manipulado explicitamente.

### Reference

Plano de uma ReferenceView ortográfica.

Esse último é importante para Trace Mode.

---

## Reference Workplane

Quando Trace Mode usa uma ReferenceView ortográfica:

```text
ReferenceView plane
=
Active Workplane
```

Isso garante que:

- Line;
- Rectangle;
- Bezier;
- Poly Pen;
- Set Scale;
- snapping;

usem exatamente o mesmo frame da referência.

Nenhuma conversão duplicada.

---

## Perspective Reference

Uma Perspective Reference não define um único plano 2D universal.

Portanto Trace Mode perspectiva deve escolher explicitamente:

```text
Ground
Face
Custom construction plane
```

e usar a Reference Camera apenas como view.

Não fingir que uma fotografia perspectiva é um Workplane.

---

## Auto Workplane

A heurística atual é boa como base:

1. face relevante sob cursor;
2. caso contrário plano de mundo apropriado à vista;
3. preferência opcional por Ground.

Mas a decisão deve acontecer no início de uma operação/desenho.

Evitar Workplane mudar silenciosamente no meio de um gesto.

---

## Workplane Capture

Ao iniciar uma gesture que depende de plano:

```text
resolve workplane
→ freeze for gesture
→ preview
→ commit/cancel
```

Isso elimina jitter quando cursor cruza diferentes faces.

---

## Camera rule

Preservar decisão anterior:

> mudar Workplane nunca move câmera automaticamente.

Ação explícita:

```text
Look At Workplane
```

faz o alinhamento.

---

## Workplane from selection

A base atual deve ser preservada:

- from three points;
- face + edge.

Adicionar o caso já documentado de:

```text
Midplane between two parallel faces
```

desde que ambas sejam realmente paralelas dentro de tolerância.

---

## Surface-aware drawing

Em modo Auto:

```text
hover face
→ candidate Workplane preview

first click
→ capture Workplane

rest of gesture
→ Workplane frozen
```

Isso combina com a regra:

> o usuário vê o que o clique fará.

---

# Scene-wide snapping

`SnapAccel::build(&[SnapSource])` já suporta múltiplas fontes estruturalmente.

A arquitetura nova deve assumir:

```text
visible eligible SceneObjects
→ Geometry Evaluation
→ Snap Sources
→ SnapAccel
```

Não apenas active mesh.

---

## Geometry stage para snap

Para Object mode:

```text
snap against Modified geometry
```

porque é o que o usuário vê.

Para component editing da EditableMesh:

```text
active object
→ authoring Base mesh

other objects
→ visible Modified geometry
```

Isso segue a decisão de Geometry Evaluation já proposta.

Skin/Pose pode entrar apenas quando necessário para interactive placement; não deve bloquear o primeiro slice.

---

## Object transforms

SnapAccel deve receber posições já resolvidas em world space.

Não armazenar lógica de SceneObject transform dentro do algoritmo de inferência.

Pipeline:

```text
Scene Query
→ evaluate local geometry
→ world transform
→ SnapSource
→ SnapAccel
```

---

## IDs nos hits

O `ScreenSnapHit` atual retorna:

```text
point
kind
pixel_distance
```

Para UX e ferramentas mais robustas precisamos saber a origem.

Direção:

```text
SnapHit
├ position
├ kind
├ source object
├ optional component reference
└ pixel distance
```

Isso permite:

- mostrar "Midpoint · Wall";
- usar a face atingida como novo Workplane;
- associar objetos;
- constraints futuras;
- MCP/debug determinístico.

Não precisa virar dependency graph.

---

## Component reference

Usar referências tipadas quando estáveis:

```text
Point → VertexIndex
Edge → EdgeKey
Face → FaceIndex
```

Para geometria derivada/transitória, o hit pode ter somente source ObjectId + position/kind.

Não persistir handles de snap.

---

# Preselection

Snapping e hover/preselection devem compartilhar Scene Query/acceleration, mas não ser o mesmo conceito.

```text
Preselection
→ what would be selected/acted on

Snap
→ where the action position is attracted
```

Uma face pode estar preselected enquanto o ponto de snap é um midpoint de sua edge.

A UI pode mostrar os dois sem conflitar.

---

## Visual grammar

Cada snap deve ter:

- forma;
- rótulo;
- contraste;
- guia quando aplicável.

Nunca depender somente de cor.

Exemplo:

```text
◇ Midpoint
□ Intersection
○ Point
△ Face Center
──── Parallel
┆ Perpendicular
```

Ícones exatos podem ser ajustados pelo Design System.

---

## HUD

Mostrar somente informação operacional útil:

```text
Snap: Midpoint
Guide: Parallel
Angle: 45°
Workplane: Front Reference
```

Não mostrar todos simultaneamente se não estiverem ativos.

---

# Performance

A estratégia atual é boa:

```text
build SnapAccel
→ reuse through gesture/revision
```

Não reconstruir por pointer event.

Cache key conceitual:

```text
eligible object IDs
+ geometry revisions
+ transforms revisions
+ visibility
+ editing mode
```

---

## Screen-space culling future

Antes de inventar estrutura nova, medir o `SnapAccel` atual.

Se necessário:

1. frustum/object broad phase;
2. screen-space buckets para points/edges;
3. BVH continua para occlusion/faces.

Não adicionar GPU picking para snap sem profiling demonstrar necessidade.

---

# SplineSnapSettings

`petunia_project::SplineSnapSettings` é um segundo helper de snapping geométrico no domínio de Spline.

Se for usado apenas por APIs headless de edição de spline, pode sobreviver como utilitário determinístico específico.

Mas não deve competir com o Smart Snap interativo.

Regra:

```text
interactive viewport snapping
→ Inference Engine

headless constrained spline edit
→ optional small geometry helper
```

Se não houver consumidor real, remover.

---

# Migração

1. congelar `core::inference` como comportamento a preservar;
2. criar testes de caracterização para FaceCenter, Intersection, Parallel, Perpendicular e Angle;
3. atualizar documentação que ainda marca essas features como missing;
4. criar `Workplane` neutro a partir de `ProfileWorkplane`;
5. mover active workplane para EditorSession/WorkplaneSession;
6. migrar PlanarShape/Profile para o mesmo tipo Workplane;
7. remover right/up/origin/normal duplicados de ProfileState;
8. consolidar `snap_enabled` + `SnapSettings.enabled`;
9. migrar callers interativos para uma única API de Inference;
10. aposentar `SnapQuery/SnapResult/snap_point` como motor interativo;
11. adaptar Scene Query para gerar SnapSources world-space de múltiplos objetos;
12. enriquecer SnapHit com source ObjectId/component;
13. congelar Workplane por gesture;
14. integrar Reference Workplane no Trace Mode;
15. adicionar Midplane entre faces paralelas;
16. validar visual grammar/HUD;
17. medir performance antes de otimização adicional.

---

# Gates obrigatórios

- snap tem tolerância visual equivalente em qualquer zoom;
- DPI não altera semanticamente a seleção do candidato;
- moving geometry não snap em si mesma;
- hidden geometry não vence sem X-Ray;
- Point/Midpoint/Intersection/FaceCenter têm prioridade previsível;
- Parallel/Perpendicular/Angle não criam jitter;
- grid permanece fallback em Smart Snap;
- hard increment snap é comportamento separado;
- multi-object snap respeita transformações;
- locked objects podem ser targets, mas não podem ser editados;
- hidden objects nunca são targets;
- active Workplane não muda no meio da gesture;
- mudar Workplane nunca move câmera implicitamente;
- Reference orthographic plane e Trace Workplane coincidem exatamente;
- Perspective Reference nunca é tratada como plano automaticamente;
- snap marker usa forma + texto, nunca só cor;
- one pointer event não reconstrói SnapAccel sem revisão geométrica.

---

# Decisões recomendadas

1. `core::inference` vira a base canônica e é preservado/refatorado, não reescrito.
2. `core::snap` deixa de ser um segundo motor interativo.
3. Smart Snap continua ligado por padrão.
4. Smart Snap não implica hard grid quantization.
5. Prioridade mantém points > edge > guides > angle > face > grid.
6. FaceCenter e OnFace continuam semanticamente separados.
7. Shift suporta hold lock + alternativa toggle acessível.
8. Angle step default 15°, configurável.
9. Workplane vira tipo reutilizável global, derivado do atual ProfileWorkplane.
10. Active Workplane pertence ao EditorSession; shapes persistem sua própria cópia.
11. Workplane modes: Auto, Ground, View, Face, Selection, Custom, Reference.
12. Workplane é congelado por gesture.
13. Camera nunca segue Workplane implicitamente.
14. Orthographic Reference pode ser Workplane; Perspective Reference não.
15. Scene-wide snap usa múltiplos SnapSources em world space.
16. SnapHit recebe source ObjectId e component reference quando possível.
17. Component edit snap usa base mesh do objeto ativo e geometry evaluated dos demais.
18. Preselection e Snap compartilham infraestrutura, mas permanecem conceitos distintos.
19. SnapAccel é cacheado por revisão/gesture.
20. Não introduzir GPU snap, constraint solver ou dependency graph sem necessidade medida.
