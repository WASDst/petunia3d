# Direct Modeling — Push/Pull, Extrude, Inset e Bevel

> **Status: aprovado.**

## Objetivo

Fechar uma semântica simples e previsível para as operações que alteram forma diretamente.

O usuário precisa conseguir responder sem conhecer arquitetura interna:

```text
Push/Pull       → quero empurrar/puxar esta superfície
Extrude Faces   → quero criar nova topologia a partir destas faces
Extrude Shape   → quero um sólido paramétrico vindo desta forma 2D
Inset           → quero criar uma borda interna
Bevel           → quero quebrar/arredondar esta quina
```

Essas operações não devem ser variações ocultas do mesmo comando.

---

## Diagnóstico atual

### Extrude Faces

`Mesh::extrude_selected()` já possui uma base boa:

- extruda uma região selecionada compartilhando o novo cap;
- cria paredes somente no contorno;
- aceita distância zero para preview;
- preserva UV da tampa;
- cria faixa UV nas paredes;
- possui `TopologyResult` wrapper;
- funciona com n-gons;
- possui testes.

Classificação:

```text
Extrude kernel
→ STRONG REUSE + FaceCorner/material/provenance refactor
```

### Extrude Individual

`extrude_individual()` também já existe.

Cada face selecionada ganha seus próprios vertices/cap/walls.

Classificação:

```text
REUSE + attribute/provenance refactor
```

### Push/Pull

O `Mesh::push_pull()` atual apenas:

```text
selected faces
→ average normal
→ translate selected vertices
```

e **não cria faces**.

Já `region_push` faz:

```text
profile region
→ imprint
→ begin Extrude modal
```

Portanto atualmente existem duas semânticas diferentes chamadas Push/Pull.

Isso precisa ser resolvido.

### Inset

O kernel atual é métrico e já possui proteção contra colapso/inversão.

Mas trabalha face por face.

Isso é bom para `Individual`, mas não representa corretamente o caso principal de uma seleção coplanar de várias faces como uma única região.

### Bevel

A implementação atual já possui:

- multi-segment;
- clamp overlap;
- vertex bevel;
- preview modal;
- kernel transacional por edge.

Mas o kernel de edge ainda possui restrições fortes de topologia e o multi-edge recompõe a seleção sequencialmente.

Não deve ser tratado como Bevel geral completamente maduro ainda.

---

# Três extrusões, três intenções

## 1. Extrude Faces

Operação direta de EditableMesh.

Entrada:

```text
selected authoring faces
```

Saída:

```text
duplicated cap
+ side walls
+ topology remap/provenance
```

É deliberadamente topológica.

Mesmo que o resultado visual pudesse ser obtido apenas movendo uma face, Extrude cria a nova faixa lateral.

## 2. Extrude Individual

Também operação direta.

```text
selected faces
→ independent prisms/caps
```

Faces vizinhas não compartilham o novo cap.

## 3. Extrude Shape

É Generator.

```text
PlanarShape
→ GeometryGenerator::Extrude
→ derived Mesh
```

É não destrutivo e paramétrico.

Na UI usar `Extrude Shape` ou `Create Extrusion` quando houver risco de ambiguidade.

No código:

```text
GeometryGenerator::Extrude
```

Não chamar Generator de `Face Extrude`.

---

# Push/Pull

Push/Pull deve ser uma ferramenta contextual de manipulação direta, não apenas outro nome para Extrude.

## Regra

```text
click/drag a visible face or planar region
→ move that surface along its normal
→ preserve the simplest valid topology
```

## Caso A — face de contorno de um sólido

Exemplo: tampa de um cubo.

```text
Push/Pull top face upward
→ move top face
→ adjacent side faces stretch
→ no unnecessary new edge loop
```

Isso é diferente de Extrude Faces:

```text
Extrude top face
→ duplicate cap
→ create a new side ring
```

Essa diferença deve ser visível e intencional.

## Caso B — região imprinted dentro de uma face

Exemplo: desenhar uma janela na fachada e empurrar para dentro.

```text
planar region
→ imprint
→ create cap + side walls
→ move cap inward/outward
```

Aqui Push/Pull precisa criar topologia porque a região não era uma superfície móvel independente.

Pode reutilizar o Extrude kernel internamente.

## Caso C — região livre de PlanarShape

Esse é o principal fluxo shape-first:

```text
draw closed shape
→ hover region
→ Push/Pull
→ create GeometryGenerator::Extrude
```

Por default, uma região livre produz uma extrusão paramétrica viva.

Isso conecta diretamente DRAW ao Generator redesign já aprovado.

### Consequência

Editar a PlanarShape atualiza a extrusão.

Alterar Height atualiza a extrusão.

`Make Editable` converte o resultado final para EditableMesh.

## Caso D — open sheet

Push/Pull em uma folha aberta pode oferecer:

```text
Create Thickness
```

mas não deve inferir isso silenciosamente.

V1 pode recusar e sugerir Extrude/Thickness quando não existir volume/topologia suficiente para uma manipulação inequívoca.

---

# Push/Pull para dentro

Empurrar uma região imprinted para dentro cria um pocket válido:

```text
facade
→ imprint window
→ push inward
→ recessed cavity
```

Enquanto não ocorrer interseção com outra superfície, o resultado é simples e manifold.

## Collision limit

Quando o cap alcança outra parte da mesma superfície/solid:

```text
safe collision
→ clamp preview
→ show `Reached Surface`
```

Não atravessar geometria silenciosamente.

## Auto Cut Through

Extensão recomendada por ser especialmente útil para arquitetura/props:

```text
imprinted region
→ push inward
→ encounters one unambiguous opposite wall
→ [Cut Through]
```

Quando o caso for inequívoco, o Petunia pode usar o Boolean/Cut kernel existente para abrir o vão.

Exemplos:

- janela;
- porta;
- ventilação;
- furo retangular;
- passagem.

Não executar Cut Through quando múltiplas superfícies/volumes tornam o resultado ambíguo.

Primeiro slice pode apenas clamp + botão explícito; auto-completion vem depois.

---

# Push/Pull snapping

Durante o gesto, a distância escalar ao longo da normal pode usar o Inference Engine.

Targets úteis:

```text
Point height
Edge height
Face plane
Grid increment
typed distance
```

Exemplo:

```text
pull cabinet top
→ hover another shelf
→ Snap: Face
→ exact same height
```

O snap continua sendo uma única passada externa à Geometry.

---

# Extrude Faces

## Direction

Default:

```text
single/coplanar region
→ common normal
```

Para seleção não coplanar:

```text
Extrude Region
→ area-weighted selection normal
```

Se a direção resultante for degenerada/quase zero:

```text
Choose Axis / Extrude Individual
```

Não usar uma normal arbitrária.

## Constraints

Extrude pode ser restrita a:

- normal;
- X/Y/Z;
- inferred direction quando semanticamente válido;
- numeric distance.

Plane constraint não deve produzir direção degenerada sem feedback.

## Negative Extrude

Distância negativa é válida.

Ela não deve inverter toda a Mesh.

Somente winding/normal das novas faces que realmente exigirem correção é ajustada pelo kernel.

Qualquer caminho atual que reverta todas as faces por causa de extrusão negativa deve ser removido.

---

# Extrude Shape Generator

Primeiro Generator de uso diário recomendado:

```rust
GeometryGenerator::Extrude {
    source: PlanarShapeId,
    distance: f32,
    cap_start: bool,
    cap_end: bool,
    // material assignments simples futuramente
}
```

## Holes

PlanarShape com holes deve produzir extrusão com furos corretamente.

Exemplo:

```text
outer rectangle
+ inner rectangle hole
→ wall/frame extrusion
```

Isso é essencial para shape-first.

## Preview/Final

Extrude é barato e pode normalmente usar a mesma geometria em Preview e Final.

Não inventar quality decimation onde não há ganho.

## Make Editable

```text
Generator Extrude
→ evaluate final
→ apply modifiers
→ EditableMesh
```

conforme decisões anteriores.

---

# Direct editing de Generators

Face edit em Generator não deve alterar uma Mesh derivada e perder parâmetros silenciosamente.

Fluxo:

```text
user invokes Push/Pull / Face Extrude / Inset / Bevel on Generator
→ `Parametric object`
→ [Make Editable & Continue] [Cancel]
```

`Make Editable & Continue` pode ser uma única transaction:

```text
generator
→ bake
→ execute requested direct operation
```

Undo restaura o Generator original.

Isso mantém o fluxo rápido sem conversão destrutiva invisível.

---

# Inset

## Default: Region

Quando faces selecionadas formarem regiões coplanares conectadas:

```text
selection
→ coplanar connected regions
→ inset each region boundary
```

Faces internas entre faces selecionadas não devem criar bordas de inset.

Isso é a semântica esperada para pisos, painéis, fachadas e hard-surface.

## Individual mode

Manter opção:

```text
Inset Mode
● Region
○ Individual
```

`Individual` reutiliza o comportamento face-by-face atual.

## Multiple islands

Várias regiões coplanares desconectadas são processadas independentemente na mesma operação.

Não unir islands à distância.

## Holes

Region Inset deve respeitar loops internos.

Outer boundary move para dentro.

Hole boundaries movem para fora da região preenchida.

## Metric

Preservar a decisão atual:

```text
Inset distance = world units
```

Não factor relativo ao tamanho da face.

## Clamp

Preservar a proteção atual contra:

- edge collapse;
- normal inversion;
- self-intersection simples.

Quando o valor solicitado é reduzido:

```text
Requested 0.50 m
Applied 0.37 m · Limit reached
```

Não reduzir silenciosamente.

---

# Bevel

## Uma ferramenta

Não separar Chamfer e Bevel em duas ferramentas.

```text
Segments = 1
→ Chamfer

Segments > 1
→ Rounded Bevel
```

## Base a preservar

Preservar:

- metric width;
- segments;
- clamp overlap;
- vertex bevel;
- modal preview.

## Kernel atual não é suficiente como geral

O edge bevel atual é mais seguro em:

- manifold edge;
- convex edge;
- endpoints com topologia limitada.

Multi-edge ainda depende de aplicar edges sequencialmente.

Direção:

```text
selected edge set
→ validate whole set
→ build bevel result transactionally
→ publish only if whole requested operation is valid
```

Não deixar metade da seleção bevelada e metade ignorada sem consentimento.

## Unsupported topology

Default:

```text
Unsupported edges found
→ no mutation
→ highlight/select unsupported
```

Um futuro `Skip Unsupported` pode existir como ação explícita.

## Segments

V1 recomendada:

```text
1–8 na UI comum
```

Valores maiores podem ser digitados dentro de um limite seguro se houver necessidade.

Não há razão para manter o clamp atual de 4 como limite conceitual.

## Profile

V1 usa perfil circular/fillet padrão.

Não expor profile curve customizada inicialmente.

---

# Attributes e provenance

Todas as operações diretas que mudam topology precisam devolver provenance suficiente.

## UV

Com FaceCorner:

### Extrude

- cap preserva os FaceCorners da fonte;
- side walls recebem faixa UV contínua por boundary loop;
- múltiplos boundary loops são tratados separadamente.

### Inset

- inner face recebe UV pela mesma parametrização da source face;
- ring corners são interpolados da source face.

### Bevel

- novos corners interpolam UV das faces/edges contribuintes;
- nunca copiar UV por VertexIndex assumindo unicidade.

## Material slots

Regra geral:

```text
new face with one semantic source
→ inherit that source material slot
```

Para nova face derivada de múltiplas faces:

```text
all contributors same slot
→ inherit

mixed slots
→ deterministic fallback + diagnostic
```

Não perder material silenciosamente.

Para Extrude, side walls normalmente herdam o material da face/boundary que as originou.

## Vertex color

Novos vertices interpolam source colors conforme o mesmo provenance geométrico.

## Sharp/Shading

Operações preservam Sharp flags existentes quando a edge possui descendente inequívoca.

Novas edges não são marcadas Sharp automaticamente apenas por serem novas.

Feature Edge derivada continua cuidando da leitura visual.

Bevel pode futuramente oferecer shading helper específico, mas isso não entra no kernel base.

---

# TopologyResult

O atual wrapper baseado principalmente em contagens é insuficiente para Paint, attachments e seleção.

Estender a infraestrutura já aprovada para carregar:

```text
vertex remap
face remap
created vertices
created edges
created faces
deleted elements
source/provenance when unambiguous
dirty domains
diagnostics
```

Não precisa existir um mega-framework de history.

É apenas o contrato da edição geométrica.

## Selection after operation

Application usa o resultado para selecionar o próximo elemento útil:

```text
Extrude
→ new cap selected

Inset
→ inner region selected

Bevel
→ bevel strip/affected edges according to tool policy

Push/Pull
→ manipulated face/region remains active
```

Selection deixa de morar dentro da Mesh conforme arquitetura já aprovada.

---

# SurfaceAttachment

Operação topológica tenta remapear attachment somente quando provenance é inequívoca.

Exemplo:

```text
face survives as cap descendant
→ remap
```

Quando a surface original é dividida em múltiplas candidatas sem correspondência única:

```text
NeedsReattach
```

Não escolher uma face arbitrariamente.

---

# Paint

Raster Paint continua associado a TextureResource/UV.

Uma operação topológica:

- preserva pixels existentes;
- preserva UV quando possível;
- cria UV determinística para faces novas;
- não repinta automaticamente novas surfaces.

Se novas faces usam uma região UV que sobrepõe conteúdo existente, diagnostics pode recomendar:

```text
Repack UV
Paint new surfaces
```

Não executar automaticamente.

---

# Last Operation

Depois do commit de:

- Extrude;
- Inset;
- Bevel;
- Generator creation;

Application pode manter um `Last Operation` transitório com parâmetros ajustáveis.

Isso não é Modifier.

```text
commit Bevel
→ Last Operation { Width, Segments, Clamp }
→ parameter edit rebuilds from same before snapshot
```

Quando outra edição incompatível ocorre, o painel deixa de ser reeditável.

Isso oferece UX não destrutiva imediata sem criar history node graph.

---

# Undo / ToolSession

Todas seguem a gramática já aprovada:

```text
begin gesture
→ preview from fixed source
→ numeric/snapped updates
→ commit = one Undo
→ Esc = exact restoration
```

Não empilhar Undo por preview.

Para `Make Editable & Continue`, bake + direct edit podem ser uma única operação de Undo.

---

# Kernels versus tools

Geometry expõe operações sem UI.

Application decide:

- hover target;
- seleção;
- modal lifecycle;
- snapping;
- prompts;
- Make Editable;
- Undo;
- Last Operation.

Uma direção possível:

```text
Geometry
├ extrude_region
├ extrude_individual
├ move_face_region
├ inset_region
├ inset_individual
└ bevel_edges

Application
├ PushPullSession
├ ExtrudeSession
├ InsetSession
└ BevelSession
```

Os nomes exatos podem variar.

Não passar AppState aos kernels.

---

# Migração recomendada

1. caracterizar extrude/inset/bevel atuais com testes;
2. introduzir FaceCorner/provenance no resultado;
3. corrigir side material inheritance de Extrude;
4. remover qualquer full-mesh winding flip em negative extrude;
5. separar semanticamente Push/Pull de Extrude;
6. implementar `move_face_region` para boundary faces;
7. manter imprint + extrude para embedded planar regions;
8. fazer free PlanarShape Push/Pull criar Extrude Generator;
9. implementar Generator Extrude com holes;
10. adicionar `Make Editable & Continue`;
11. migrar Inset default para coplanar Region;
12. preservar Individual Inset;
13. substituir multi-edge sequential Bevel por edge-set transaction;
14. enriquecer TopologyResult;
15. conectar operation result à SelectionState;
16. conectar remap ao SurfaceAttachment;
17. conectar snap escalar do Push/Pull ao Inference Engine;
18. adicionar clamp/collision feedback;
19. adicionar Cut Through explícito para caso unambíguo;
20. portar Last Operation para Application architecture.

---

# Gates obrigatórios

- Push/Pull de face externa não cria edge loop desnecessário;
- Extrude Faces sempre cria topology nova;
- free PlanarShape Push/Pull cria Generator, não mesh duplicada;
- host-face region Push/Pull cria pocket manifold;
- negative push não inverte a Mesh inteira;
- collision não atravessa geometry silenciosamente;
- Extrude Individual mantém faces independentes;
- Inset Region ignora edges internas da seleção;
- Inset Individual mantém comportamento por face;
- Bevel é transacional para edge-set;
- FaceCorner UV invariants permanecem válidos;
- material slots são preservados deterministicamente;
- vertex colors são interpolados;
- TopologyResult remapeia seleção/attachments quando possível;
- unsupported topology não publica resultado parcial por default;
- one gesture = one Undo;
- cancel restaura exatamente o source;
- Generator direct edit nunca faz bake silencioso.

---

# Decisões recomendadas

1. Push/Pull e Extrude deixam de ser sinônimos.
2. Push/Pull é contextual e preserva a topologia mais simples.
3. Extrude Faces é operação topológica explícita.
4. Extrude Individual permanece separada.
5. Extrude Shape é Generator paramétrico.
6. Push/Pull de PlanarShape livre cria Extrude Generator por default.
7. Push/Pull de região sobre Mesh usa imprint + extrusion/pocket.
8. Face boundary Push/Pull move a superfície sem criar loop novo.
9. Push/Pull para ao colidir; Cut Through é explícito e só em caso inequívoco.
10. Push/Pull participa do Inference Engine para alturas/alinhamentos.
11. Negative Extrude nunca inverte a Mesh inteira.
12. Inset default vira Region; Individual continua disponível.
13. Inset continua métrico.
14. Bevel continua uma ferramenta; Segments=1 equivale a Chamfer.
15. Bevel edge-set deve ser transacional.
16. Bevel UI comum suporta 1–8 segments; profile custom fica fora da V1.
17. Direct editing em Generator usa `Make Editable & Continue`, nunca bake silencioso.
18. FaceCorner UV, MaterialSlot, vertex color e provenance são parte do contrato da operação.
19. TopologyResult é enriquecido em vez de criar um history/geometry framework novo.
20. Last Operation oferece reedição imediata sem transformar operações diretas em modifiers.

## Regra anti-bloat

Cada ferramenta responde a uma intenção diferente:

```text
Push/Pull
→ manipulate what I see

Extrude Faces
→ create topology

Extrude Shape
→ keep the shape parametric

Inset
→ create an inner boundary

Bevel
→ soften/break an edge
```

Se essas intenções continuarem claras, o Petunia ganha poder sem exigir que o iniciante entenda a implementação.

## Decisões fechadas

1. Push/Pull e Extrude não são sinônimos.
2. Push/Pull é contextual e preserva a topologia mais simples.
3. Extrude Faces é operação topológica explícita.
4. Extrude Individual permanece separada.
5. Extrude Shape é Generator paramétrico.
6. Push/Pull de PlanarShape livre cria Extrude Generator por default.
7. Push/Pull de região sobre Mesh usa imprint + extrusion/pocket.
8. Face boundary Push/Pull move a superfície sem criar loop novo.
9. Push/Pull para ao colidir; Cut Through é explícito e somente em caso inequívoco.
10. Push/Pull participa do Inference Engine para alturas/alinhamentos.
11. Negative Extrude nunca inverte a Mesh inteira.
12. Inset default é Region; Individual continua disponível.
13. Inset continua métrico.
14. Bevel continua uma ferramenta; Segments=1 equivale a Chamfer.
15. Bevel edge-set deve ser transacional.
16. Bevel UI comum suporta 1–8 segments; profile custom fica fora da V1.
17. Direct editing em Generator usa Make Editable & Continue, nunca bake silencioso.
18. FaceCorner UV, MaterialSlot, vertex color e provenance fazem parte do contrato da operação.
19. TopologyResult é enriquecido em vez de criar um history/geometry framework novo.
20. Last Operation oferece reedição imediata sem transformar operações diretas em modifiers.
