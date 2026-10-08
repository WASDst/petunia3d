# Transforms, Origin, Pivot & Gizmos

> **Status: proposta recomendada; aguarda aprovação.**

## Objetivo

Fechar uma única gramática para Move, Rotate e Scale em Object Mode e Component Edit, incluindo:

- espaço de transformação;
- pivô;
- origem do objeto;
- hierarquia;
- multi-selection;
- gizmos;
- snapping;
- numeric input;
- apply transforms.

A regra central é:

> Object Mode transforma SceneObject.transform. Edit Mode transforma authoring geometry.

Esses dois caminhos não podem continuar misturados.

---

# Diagnóstico atual

## Object Mode ainda move a Mesh

O modal atual seleciona toda a Mesh no Object Mode e transforma seus vertices.

Depois tenta manter também:

```text
Asset.position
Asset.rotation
Asset.scale
Asset.origin
```

sincronizados.

Isso cria duas autoridades para posição espacial.

Entra em conflito direto com a arquitetura já aprovada:

```text
authoring geometry = local
SceneObject.transform = local → parent/world authority
```

Classificação:

```text
current Object Mode transform
→ REWRITE AT APPLICATION BOUNDARY
```

Os algoritmos de gesture/modal podem ser reutilizados; a mutação final não.

## Edit Mode

Transform de componentes sobre vertices selecionados é conceitualmente correto.

Classificação:

```text
component transform math
→ REUSE + typed selection / world-local conversion
```

## Orientações atuais

Hoje existem apenas:

```text
Global
Local
```

e `Local` no gizmo customizado é inferido de seleção de faces/edges ou até da primeira face da Mesh.

Isso é semanticamente instável.

## Dois sistemas de gizmo

Hoje coexistem:

```text
transform-gizmo-egui
+
Petunia custom gizmo
```

O gizmo próprio já possui:

- tamanho visual estável em logical pixels;
- axis handles;
- plane handles;
- rotation rings;
- scale handles;
- Universal gizmo;
- hit testing próprio;
- integração direta com o modal Petunia.

Manter os dois não traz benefício proporcional.

## Origin e Pivot estão misturados

Hoje:

```text
PivotPoint
→ centro usado por uma transformação
```

mas:

```text
Edit Pivot Mode
→ move Asset.origin
```

Esses conceitos são diferentes.

A UI/API deve separá-los.

---

# Transform canônico

SceneObject continua com um único transform autoral local ao parent:

```rust
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}
```

A matemática operacional usa Quat/Mat4.

Euler degrees podem continuar como representação de UI e, durante migração, serialização.

Não acumular rotações fazendo soma ingênua de Euler quando o gesto pode ser resolvido por quaternion/matrix.

## World transform

```text
world(object)
=
world(parent) × local(object)
```

Sem parent:

```text
world = local
```

---

# Object Mode

Move/Rotate/Scale alteram somente:

```text
SceneObject.transform
```

Nunca vertices de EditableMesh.

Nunca output de Generator.

Nunca pontos de Spline/PlanarShape.

Consequências:

- câmera não invalida geometria;
- mover objeto não invalida RenderMesh;
- Generator não perde parametricidade;
- Paint/UV não são reescritos;
- scene hierarchy funciona corretamente.

---

# Edit Mode

Component transform altera authoring data local.

Para EditableMesh:

```text
Vertex / Edge / Face selection
→ selected VertexIndices
→ local geometry positions
```

Para Spline/PlanarShape:

```text
selected control points
→ authoring points
```

Generator evaluated-only components continuam não editáveis diretamente.

---

# World-space interaction em Edit Mode

O usuário interage visualmente em world space.

Mesmo com objeto rotacionado/parented:

```text
pointer / snap / gizmo
→ desired world-space transform
→ inverse object world transform
→ local authoring positions
```

Isso evita que snapping de cena dependa de coordenadas locais internas.

---

# Transform Orientation

Substituir a enum limitada atual por:

```text
World
Local
Workplane
Normal
View
```

## World

Eixos globais XYZ.

## Local

Object Mode:

```text
SceneObject local axes
```

Edit Mode:

```text
active SceneObject local axes
```

Local **não** significa média arbitrária das normals selecionadas.

## Workplane

Usa o Workplane ativo já aprovado.

Essencial para DRAW, Trace e arquitetura.

## Normal

Edit Mode:

```text
active face / selected region frame
```

Quando não existir orientação normal inequívoca:

```text
Normal orientation unavailable
```

e não fallback silencioso para primeira face.

Object Mode pode desabilitar Normal.

## View

Eixos da câmera atual:

```text
X = camera right
Y = camera up
Z = camera forward
```

Útil para ajustes screen-aligned.

## Fora da V1

Não adicionar:

- Gimbal orientation;
- arbitrary saved custom orientations;
- orientation stack;
- named construction frames.

Workplane já cobre boa parte do caso custom.

---

# Pivot Policy

Pivot é estado do EditorSession.

Não é propriedade do objeto.

V1:

```text
Median
Bounding Box Center
Active Element
3D Cursor
Individual
```

## Median

Média das posições world-space das unidades selecionadas.

## Bounding Box

Centro da AABB world-space da seleção.

## Active Element

Usa:

- active SceneObject em Object Mode;
- active Face/Edge/Vertex quando disponível em Edit Mode.

Se não houver active element válido, o comando deve ficar indisponível ou indicar fallback explícito; não escolher elemento arbitrário.

## 3D Cursor

Usa cursor transitório do editor.

## Individual

Object Mode:

```text
cada SceneObject usa sua própria Object Origin
```

Edit Mode:

```text
cada ilha conectada de componentes selecionados usa seu centro
```

Isso evita duas transformações incompatíveis no mesmo vertex compartilhado.

---

# 3D Cursor

O 3D Cursor permanece ferramenta do EditorSession, não dado semântico obrigatório do Document.

Pode ser restaurado por workspace/session memory, mas comandos headless nunca dependem dele implicitamente.

Operações:

```text
Place Cursor
Cursor to Selection
Cursor to World Origin
Selection to Cursor
```

`Place Cursor` usa Inference Engine.

Quando usado para criar um objeto, a posição resultante é copiada para SceneObject.transform.

---

# Object Origin

Object Origin é o ponto local `(0,0,0)` do SceneObject e a base do seu frame local.

Não manter um segundo conceito opcional:

```text
Asset.origin: Option<Vec3>
```

no destino final.

O transform do SceneObject já localiza a origem no parent/world space.

## Origin ≠ Pivot

```text
Object Origin
→ persistente
→ afeta frame local, parenting e transforms

Transform Pivot
→ transitório/editor
→ só decide centro da operação atual
```

Por isso:

```text
Edit Pivot Mode
```

deve virar:

```text
Edit Origin
```

---

# Edit Origin

Em Edit Origin:

```text
Move gizmo
→ move Object Origin
→ world geometry stays fixed
```

Isso requer compensação nos dados locais.

Para EditableMesh:

```text
new object transform
+ inverse local vertex offset
→ identical world vertex positions
```

Uma gesture = um Undo.

Rotate/Scale Origin ficam fora da primeira versão.

V1 Edit Origin é translacional.

---

# Origin commands

Preservar e clarificar:

```text
Origin to Geometry
Origin to Bottom
Origin to 3D Cursor
Origin to Selection
Geometry to Origin
```

## Origin to Geometry

Move a origem para o centro geométrico sem mover a aparência world-space.

## Origin to Bottom

Move para:

```text
center X/Z of local/world bounds
+ lowest point along chosen up axis
```

Default usa World Up; opção futura Workplane Up apenas se houver necessidade.

É especialmente útil para props assentados no chão.

## Origin to Cursor

Move a origem ao 3D Cursor mantendo geometria visualmente fixa.

## Origin to Selection

Move a origem ao pivot/centro da seleção autoral.

## Geometry to Origin

Move os dados autorais locais para que seu centro coincida com `(0,0,0)` do objeto.

Nesse comando a geometria **realmente se move em world space** se o transform do objeto ficar inalterado.

A UI precisa diferenciar fortemente:

```text
Origin to Geometry
≠
Geometry to Origin
```

---

# Origin e Generators

Mover a origem preservando world geometry implica reescrever o frame autoral.

Para evitar um `geometry_offset` genérico escondido:

```text
Generator / procedural geometry
+ Edit Origin
→ Make Editable & Continue
```

na V1.

O mesmo vale para Origin to Geometry/Bottom/Cursor quando a operação exigiria deslocar output procedural sem semântica definida.

Generators específicos podem ganhar origin/anchor parameters no futuro quando isso fizer sentido ao produto.

---

# Apply Transform

Adicionar operações explícitas:

```text
Apply Scale
Apply Rotation
Apply Rotation & Scale
```

Objetivo:

```text
bake object transform component into authoring geometry
→ reset corresponding SceneObject.transform component
→ world appearance unchanged
```

## EditableMesh

Suportado.

## Spline / PlanarShape

Pode transformar seus pontos autorais quando a operação for bem definida.

## Generator

V1:

```text
Make Editable & Apply
```

Não bakear parametricidade silenciosamente.

## Non-uniform scale

Operações métricas como Bevel/Inset podem produzir medidas visualmente distorcidas quando o objeto possui non-uniform scale.

Antes de uma operação que depende de distância world-space exata:

```text
Non-uniform scale detected
[Apply Scale & Continue] [Use Local Units] [Cancel]
```

O default recomendado é `Apply Scale & Continue` para um EditableMesh.

---

# Negative scale

Negative scale é permitido no Object Mode.

Renderer precisa tratar determinant negativo corretamente em:

- culling/front-facing;
- normal transform;
- Face Orientation.

`Apply Scale` com determinant negativo precisa corrigir winding/normals na authoring Mesh para manter a aparência.

Não usar negative scale como substituto oculto de Mirror Modifier.

---

# Hierarchy e multi-selection

Object transforms são calculados em world space e resolvidos novamente para local transforms.

Para cada objeto selecionado:

```text
desired_world
→ inverse(new_parent_world) × desired_world
→ new local Transform
```

## Parent e child selecionados juntos

Evitar double transform.

Algoritmo:

1. snapshot de todos os world transforms originais;
2. calcular desired world transform de cada objeto selecionado;
3. resolver os novos locals em ordem hierárquica usando o parent world já atualizado;
4. unselected descendants apenas herdam o parent normalmente.

Assim selecionar parent + child e mover ambos não move o child duas vezes.

## Diferentes parents

Multi-selection continua funcionando.

Cada local transform é resolvido contra seu próprio parent.

---

# TRS e shear

O SceneObject aprovado usa Translation + Rotation + Scale, não matriz arbitrária/shear.

Algumas combinações de:

```text
reparenting
+ rotated object
+ non-uniform parent scale
```

podem gerar uma matriz local com shear que não é representável exatamente por TRS.

Nesses casos:

```text
Cannot preserve world transform exactly with this parent scale.
[Apply Parent Scale] [Keep Local Transform] [Cancel]
```

Nunca decompor aproximadamente e alterar o objeto silenciosamente.

Isso refina a regra já aprovada de Parent/Unparent preservar world transform.

---

# Gizmo canônico

Manter o **Petunia custom gizmo** como implementação canônica.

Remover no destino final:

```text
transform-gizmo
transform-gizmo-egui
transform_gizmo_integration.rs
mint dependency quando não houver outro consumidor
```

Motivos:

- já temos hit testing próprio;
- já temos logical-pixel sizing;
- já temos Universal mode;
- precisamos integração íntima com Inference/Workplane/Pivot;
- reduz dependências;
- evita duas gramáticas de interação.

---

# Gizmo tools

Continuam disponíveis:

```text
Move
Rotate
Scale
Transform / Universal
```

Universal é conveniente, mas não precisa ser o único/default.

Usuário pode escolher ferramenta separada ou Universal.

## Move

- axis handles;
- plane handles;
- center = view-plane free move.

## Rotate

- X/Y/Z rings;
- View ring opcional/recomendado;
- angle snap integra Inference/Angle policy.

## Scale

- X/Y/Z handles;
- center = uniform scale;
- plane scale pode entrar somente se UX realmente justificar.

## Universal

Combina handles, mas usa hierarquia visual para não virar ruído.

---

# Gizmo orientation

O gizmo recebe um `TransformFrame` resolvido pela Application:

```text
origin
basis X
basis Y
basis Z
pivot policy
```

A UI não tenta deduzir Local/Normal examinando a Mesh por conta própria.

Isso elimina o comportamento atual de `local_axes_for_mesh()` escolher primeira face.

---

# Gizmo accessibility

Eixos continuam usando cores convencionais, mas nunca somente cor.

Adicionar:

- labels X/Y/Z nas pontas quando houver espaço;
- shapes distintos para Move/Scale;
- hover com texto curto;
- hit target maior que stroke visual;
- high-contrast outline;
- colorblind axis palette opcional;
- tamanho baseado em logical pixels/DPI.

Target mínimo recomendado para hit area: aproximadamente 24 logical px quando viável.

---

# Snapping

Gizmo/modal não implementa snapping.

Fluxo:

```text
pointer gesture
→ requested transform
→ Inference Engine
→ snapped requested transform
→ TransformSession preview
```

Uma única autoridade.

Move usa position snap.

Rotate usa angle snap.

Scale usa increment snap somente quando explicitamente habilitado.

---

# Numeric input

Distinguir:

## Inspector

Campos são valores absolutos do SceneObject local ao parent:

```text
Position X/Y/Z
Rotation X/Y/Z
Scale X/Y/Z
```

## Modal HUD

Entrada digitada durante gesture é delta/valor da operação iniciada:

```text
Move X: 2 m
Rotate Z: 45°
Scale: 1.5×
```

Não misturar os dois modelos.

---

# Units

Move usa unidades do projeto/world.

Rotate usa graus na UI.

Scale é fator adimensional.

Inspector pode aceitar expressões simples de unidade no futuro (`50cm`, `1.2m`) sem mudar a arquitetura de Transform.

---

# TransformSession

Substituir o modal centrado em AppState por session menor:

```rust
pub struct TransformSession {
    pub kind: TransformKind,
    pub orientation: TransformOrientation,
    pub pivot: TransformPivot,
    pub constraint: TransformConstraint,
    pub snapshot: TransformSnapshot,
    pub current: TransformDelta,
}
```

O tipo exato pode variar.

Snapshot contém apenas os objetos/componentes necessários, não o Project inteiro.

---

# Undo

```text
pointer down / command start
→ capture snapshot
→ many previews
→ commit = one command/history entry
→ Esc = exact snapshot restoration
```

Object Mode snapshot = transforms relevantes.

Edit Mode snapshot = posições autorais relevantes.

Não clonar Document inteiro para cada drag no destino final.

---

# Events

Preview de transform não precisa emitir `DocumentChanged` pesado por mouse move.

Separar:

```text
TransformPreviewChanged
TransformCommitted
```

ou revision equivalente dentro do EditorSession.

Commit incrementa document/geometry/transform revisions corretas.

---

# Locked objects

Locked SceneObject:

- pode ser snap target;
- pode ser reference/occluder;
- não pode receber transform;
- não é alterado em multi-selection.

Se seleção contém locked + unlocked:

transforma somente unlocked e UI informa a exclusão.

---

# Migração recomendada

1. caracterizar Move/Rotate/Scale atuais com testes;
2. introduzir TransformFrame/TransformSession na Application;
3. Object Mode passa a alterar somente SceneObject.transform;
4. remover mutação de vertices em Object Mode;
5. resolver multi-selection em world space + parent-local conversion;
6. adicionar detecção de shear/TRS decomposition failure;
7. ampliar TransformOrientation para World/Local/Workplane/Normal/View;
8. remover `local_axes_for_mesh` como autoridade de Local;
9. adicionar Active Element ao Pivot;
10. formalizar Individual por objeto/ilha conectada;
11. separar Object Origin de Transform Pivot;
12. renomear Edit Pivot → Edit Origin;
13. remover Asset.origin no destino final;
14. migrar origin commands para local-geometry compensation;
15. adicionar Apply Scale / Rotation / Rotation & Scale;
16. integrar non-uniform-scale guard às operações métricas;
17. consolidar no custom Petunia gizmo;
18. remover transform-gizmo-egui/mint quando possível;
19. integrar gizmo com Inference Engine;
20. reduzir Undo snapshot ao estado transformado.

---

# Gates obrigatórios

- Object Move não altera RenderMesh/authoring vertices;
- Edit Move não altera SceneObject.transform;
- parented object move preserva world intent;
- selecting parent+child does not double-transform child;
- multi-selection with different parents is deterministic;
- Local orientation follows SceneObject transform, not arbitrary face;
- Normal orientation refuses ambiguity instead of guessing;
- Workplane orientation matches active Workplane exactly;
- View orientation tracks camera basis;
- pivot Median/Bounds/Active/Cursor/Individual have deterministic tests;
- Edit Origin keeps world geometry stationary;
- Origin to Geometry and Geometry to Origin are mathematically distinct;
- Generator is never silently baked by origin/apply operations;
- Apply Scale preserves world appearance;
- negative scale preserves correct front/back rendering;
- shear-producing reparent never approximates silently;
- gizmo size remains usable across DPI/zoom;
- gizmo labels/shapes do not depend only on RGB axes;
- snapping occurs once through Inference;
- one gesture = one Undo;
- Esc exactly restores snapshot.

---

# Decisões recomendadas

1. Object Mode modifica somente SceneObject.transform.
2. Edit Mode modifica somente authoring geometry.
3. World-space interaction é convertida para local no boundary da Application.
4. Orientations V1: World, Local, Workplane, Normal e View.
5. Local significa SceneObject local axes; Normal é orientação de seleção.
6. Pivots V1: Median, Bounding Box, Active Element, 3D Cursor e Individual.
7. 3D Cursor permanece estado do EditorSession.
8. Object Origin e Transform Pivot são conceitos separados.
9. `Edit Pivot` é renomeado para `Edit Origin`.
10. Asset.origin deixa de existir no modelo final; local (0,0,0) é a origem do SceneObject.
11. Origin operations preservam world geometry quando movem a origem.
12. Origin edit de Generator exige Make Editable na V1.
13. Apply Scale/Rotation são operações explícitas e nunca bakeiam Generator silenciosamente.
14. Operações métricas detectam non-uniform scale e oferecem Apply Scale & Continue.
15. Parent/child multi-selection é resolvida por desired world transforms, não transform incremental sobre hierarchy.
16. TRS não ganha shear; casos não representáveis são recusados com ação corretiva.
17. O gizmo próprio do Petunia vira a implementação canônica.
18. transform-gizmo-egui é removido no destino final.
19. Gizmo recebe TransformFrame pronto; não deduz orientation pela Mesh.
20. TransformSession/Undo captura somente o estado necessário, não o Document inteiro.

## Regra anti-bloat

Uma transformação precisa responder somente:

```text
O que está sendo transformado?
Em qual orientação?
Em torno de qual pivot?
Qual constraint/snap?
```

Origin, Workplane, hierarchy e gizmo alimentam essas quatro respostas; não precisam criar subsistemas concorrentes.