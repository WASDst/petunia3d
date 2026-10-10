# Shape Builder 3D

> **Status: aprovado. Direção estratégica e arquitetura sólida inicial fechadas.**

O Shape Builder 3D deve ser uma feature diferenciadora do Petunia3D por transformar composição geométrica em interação direta, sem introduzir um segundo kernel CAD.

## Princípio

A ferramenta não inventa uma nova matemática booleana.

Ela cria uma camada de interação sobre capacidades existentes:

- planar arrangement;
- regions;
- imprint;
- Push/Pull;
- Boolean Union;
- Boolean Difference;
- Boolean Intersection;
- Boolean cleanup.

## Dois caminhos internos, uma experiência

### Planar / Surface Shape Builder

Usa regiões coplanares, holes, imprint e Push/Pull.

### Solid Shape Builder

Usa o kernel boolean existente e apresenta suas partes de forma visual.

---

# Proposta detalhada para Solid Shape Builder

> **Status: aprovado para a primeira implementação sólida.**

## 1. Dois operandos por sessão

A primeira versão trabalha com exatamente dois objetos sólidos ativos:

```text
A + B
```

Não tenta construir uma partição volumétrica arbitrária de N objetos ao mesmo tempo.

Isso evita crescimento combinatório e mantém picking, preview, Undo e preservação de atributos compreensíveis.

Um terceiro objeto pode ser incorporado em um gesto posterior.

## 2. Três regiões booleanas fundamentais

Para dois sólidos, derivar:

```text
A_ONLY   = A \ B
OVERLAP  = A ∩ B
B_ONLY   = B \ A
```

Essas três regiões formam o vocabulário sólido básico.

Não é necessário criar um novo kernel de regiões volumétricas.

## 3. Componentes desconectados

Cada resultado booleano pode conter múltiplos componentes desconectados.

A Geometry deverá oferecer uma query de componentes conectados por faces:

```rust
connected_face_components(mesh: &Mesh) -> Vec<MeshComponent>
```

Cada componente visualmente separado pode virar uma região interativa.

Isso é uma query derivada; não introduz identidade persistente de componente no documento.

## 4. Identidade transitória da região

Uma região de preview pertence somente à Tool Session.

Exemplo conceitual:

```rust
pub struct SolidRegionCandidate {
    pub source: SolidRegionSource,
    pub component_index: ComponentIndex,
    pub preview_mesh: Mesh,
    pub bounds: Bounds3,
}

pub enum SolidRegionSource {
    FirstOnly,
    Intersection,
    SecondOnly,
}
```

Nada disso é persistido no arquivo .petunia.

## 5. Picking

Picking deve acontecer sobre as meshes de preview derivadas.

Pipeline:

```text
pointer ray
   ↓
candidate bounds
   ↓
triangle hit
   ↓
nearest visible SolidRegionCandidate
```

O hover destaca a região inteira correspondente ao componente conectado, não somente o triângulo atingido.

## 6. Preview

Ao iniciar a sessão:

1. capturar A e B;
2. calcular A_ONLY, OVERLAP e B_ONLY;
3. executar boolean cleanup;
4. separar componentes conectados;
5. construir cache de picking/bounds;
6. renderizar como preview transitório.

O documento não muda durante hover ou seleção de regiões.

## 7. Estado da ferramenta

```rust
pub struct SolidShapeBuilderSession {
    pub first_operand: AssetId,
    pub second_operand: AssetId,
    pub candidates: Vec<SolidRegionCandidate>,
    pub selected_regions: HashSet<SolidRegionIndex>,
    pub intent: ShapeBuilderIntent,
}
```

O cache deve ser reconstruído somente quando os operandos/revisões mudarem.

## 8. Intenções iniciais

Manter somente três ações públicas:

### Unite
As regiões tocadas entram no resultado final como um único objeto quando a união for válida.

### Remove
As regiões tocadas deixam de fazer parte do resultado.

### Extract
As regiões tocadas tornam-se objeto(s) independentes.

A UI não deve expor diretamente A_ONLY / OVERLAP / B_ONLY; isso é implementação.

## 9. Modelo de seleção por máscara

Internamente, a sessão mantém quais regiões sobreviverão.

Exemplo:

```text
A_ONLY   keep
OVERLAP  remove
B_ONLY   keep
```

produz duas partes externas sem a interseção.

Outro gesto:

```text
A_ONLY   keep
OVERLAP  keep
B_ONLY   keep
```

equivale visualmente a Union.

Isso permite uma gramática única sem criar dezenas de operadores.

## 10. Resultado

No commit:

- combinar apenas as regiões marcadas para manter;
- aplicar cleanup;
- reconstruir assets necessários;
- produzir remap/changes explícitos;
- transferir material/UV/paint quando possível;
- registrar um único Undo.

## 11. Atributos

A origem da região é conhecida:

- FirstOnly deriva principalmente de A;
- SecondOnly deriva principalmente de B;
- Intersection possui superfícies herdadas dos dois operandos e superfícies novas de corte.

Reutilizar a infraestrutura atual de Boolean material/texture transfer em vez de duplicá-la.

FaceCorner torna essa transferência mais segura porque vertex + UV de corner permanecem unidos estruturalmente.

## 12. Ambiguidade e falha

Recusar claramente quando:

- operandos não forem sólidos adequados;
- boolean kernel falhar;
- resultado for vazio;
- topologia resultante for inválida;
- número de componentes ultrapassar orçamento seguro;
- preview exceder orçamento configurado.

Nunca tentar “consertar” silenciosamente com remesh global.

## 13. Performance

Não recalcular boolean a cada movimento do mouse.

```text
tool begin / operand changed
        ↓
calculate candidate regions once
        ↓
hover only performs picking
        ↓
gesture changes selection mask
        ↓
commit reuses cached result when valid
```

Essa separação é essencial para hardware low-end.

## 14. Multi-object

Não fazer partição combinatória de vários objetos na V1.

Fluxo:

```text
A + B
→ commit
→ Result + C
→ commit
```

A interação continua rápida e previsível.

Multi-object verdadeiro só deve ser considerado se uso real demonstrar necessidade.

## 15. Relação com Planar Shape Builder

A experiência visual deve ser a mesma:

```text
hover region
→ highlight
→ drag/click
→ preview
→ commit
```

Mas o backend pode ser diferente:

```text
Planar → arrangement/regions
Solid  → boolean decomposition
```

A uniformidade é de interação, não de algoritmo.

## 16. Não objetivos

- B-Rep;
- CAD constraint solver;
- volumetric arrangement genérico;
- histórico boolean procedural obrigatório;
- remeshing global automático;
- N-object boolean partition na primeira versão;
- novo kernel CSG;
- persistência de regiões transitórias.

## Critério de sucesso

O usuário deve conseguir combinar formas complexas sem pensar em “Union/Difference/Intersection” na maior parte do tempo, enquanto a implementação continua sendo uma camada pequena e testável sobre os kernels existentes.


## Decisões fechadas — Solid Shape Builder V1

As seguintes regras são normativas:

1. A primeira versão sólida trabalha com **dois operandos por sessão**.
2. A decomposição fundamental é **A_ONLY / INTERSECTION / B_ONLY**.
3. Componentes desconectados de cada resultado são tratados como regiões distintas.
4. Regiões sólidas são **transitórias da Tool Session** e nunca persistem no documento.
5. Booleans e cleanup são recalculados quando os operandos/revisões mudarem; **movimento do mouse faz apenas picking/seleção de região**.
6. A linguagem pública inicial possui somente **Unite, Remove e Extract**.

Expansões futuras só entram após medição de uso real. Multi-object combinatório, novos kernels, B-Rep e remeshing global permanecem explicitamente fora do escopo inicial.

# Implementação detalhada do Solid Shape Builder

> **Status: aprovado; complementa as seis regras V1 anteriores.**

## Resultado booleano vazio

O wrapper boolean atual trata Mesh vazia como erro. Isso precisa mudar porque `A ∩ B = ∅` e `A − A = ∅` são resultados válidos.

Direção:

```rust
pub enum BooleanSolid {
    Empty,
    Mesh(Mesh),
}
```

Kernel error continua separado de resultado vazio.

## Pré-condição de Solid

Cada operand precisa resolver para surface Mesh finita, estruturalmente válida, manifold, fechada e dentro do budget. Não executar repair automático. Objetos skinned/rigged ficam fora do Solid Shape Builder V1.

## Session space

Usar o espaço local do primeiro operand como espaço de cálculo.

```text
A local → session space
B local → B world → inverse(A world) → session space
```

Transform singular é erro. Determinant negativo exige correção de winding ao materializar a Mesh.

## Broad phase

Se os bounds não se sobrepõem:

```text
A_ONLY = A
INTERSECTION = Empty
B_ONLY = B
```

Sem chamar o kernel.

## Decomposição dedicada

Criar uma operação Geometry de alto nível que prepare/triangule/converta os dois operands uma única vez e derive:

```text
first_only   = A − B
intersection = A ∩ B
second_only  = B − A
```

Evitar repetir conversão Petunia → provider três vezes.

## BooleanPartition

Direção conceitual:

```rust
pub struct BooleanPartition {
    pub first_only: Vec<SolidRegion>,
    pub intersection: Vec<SolidRegion>,
    pub second_only: Vec<SolidRegion>,
}
```

Cada resultado é validado, recebe boolean cleanup, é separado em componentes edge-connected e compactado.

## Connected face components

Adicionar `connected_face_components(mesh)` com edge → incident faces + BFS/DFS. Componentes que apenas tocam por um vertex continuam separados. Essa query também serve a Separate Loose Parts, imports e diagnostics.

## SolidRegion

Região transitória:

```rust
pub struct SolidRegion {
    pub id: SolidRegionId,
    pub class: SolidRegionClass,
    pub mesh: Mesh,
    pub bounds: Bounds3,
    pub provenance: RegionProvenance,
}
```

IDs existem somente durante ToolSession.

## Proveniência mínima

Não criar provenance graph CAD. Por face avaliada basta algo equivalente a:

```rust
pub enum SurfaceOrigin {
    First(FaceIndex),
    Second(FaceIndex),
    Generated,
}
```

`SourceIndex` e `surface_sources` atuais são uma boa fundação para isso.

## UV

Não relayoutar toda a Mesh só porque o resultado mistura faces de A e B.

- face com origem conhecida → preservar/interpolar UV da face-fonte para FaceCorner;
- face sem origem confiável → UV planar determinística apenas para aquela ilha;
- xatlas permanece fallback explícito, não efeito colateral obrigatório.

Preservar material + UV de cada origem é preferível a destruir os dois layouts.

## Materials

Faces herdadas preservam material do source object/face. Ao consolidar objetos, construir a material table do resultado e remapear slots de A/B deterministicamente. Surface `Generated` usa política explícita do resultado primário; não criar material novo automaticamente.

## RegionPlan — ownership transitório

O keep/remove mask decide volume, mas não identidade de SceneObject. Para preservar objetos intocados, usar um plano transitório de ownership.

No início:

```text
Group A owns: A_ONLY + INTERSECTION
Group B owns: B_ONLY + INTERSECTION
```

A interseção pode pertencer aos dois grupos somente dentro da ToolSession para reproduzir os dois objetos originais sobrepostos.

### Remove

Remove as referências das regiões tocadas de todos os grupos.

### Extract

Remove as regiões tocadas dos source groups e cria output group(s) novos.

### Unite

Remove as regiões tocadas dos grupos existentes e cria um único output group contendo-as.

Regiões não tocadas preservam identidade e comportamento dos operands.

## Preservação de Geometry Source

Se o output group de um operand não mudou, não tocar no SceneObject: preservar ObjectId, Generator, modifiers e transform.

Se mudou, materializar apenas esse resultado como EditableMesh. Isso é uma operação destrutiva explícita do Shape Builder, não o freeze paramétrico silencioso proibido anteriormente.

## Output objects

Objetos existentes modificados voltam para seu próprio local space. Objetos novos de Unite/Extract usam inicialmente o session-space transform do primeiro operand.

## Commit de grupos

- uma região → usar Mesh compacta cached;
- várias regiões disjuntas num mesmo grupo → podem permanecer como loose parts de um único objeto;
- regiões que compartilham boundary e foram `Unite` → executar union final uma única vez no commit para remover surfaces internas.

Nenhum reboolean acontece durante hover.

## Fast paths

Quando o group corresponde exatamente a uma expressão conhecida:

```text
A_ONLY + INTERSECTION = original A
B_ONLY + INTERSECTION = original B
A_ONLY                = cached A−B
B_ONLY                = cached B−A
INTERSECTION          = cached A∩B
```

`Unite` dos três calcula `A ∪ B` uma única vez no commit.

## Hover e picking

Cada SolidRegion recebe bounds + triangle BVH. Pointer query testa bounds, depois triangles, ordena hits por depth e destaca a região inteira.

## Regiões internas

Para `B` completamente dentro de `A`, o overlap pode estar oculto. O picking deve coletar todos os region hits do ray.

V1:
- nearest hit é o hover padrão;
- Tab/atalho acessível percorre regiões sob o cursor;
- X-Ray Regions temporário revela regiões ocultas.

Não exigir section plane apenas para selecionar volumes internos.

## Feedback visual

Não depender somente de cor. Combinar outline, fill translúcido e label/icon contextual: `+ Unite`, `− Remove`, `↗ Extract`. Reduced Motion remove animação, não informação.

## Gesture sampling

Drag usa spacing em screen space e deduplica `SolidRegionId`. Mouse move só faz picking; nenhuma evaluation geométrica ocorre durante o gesto.

## Session key

Capturar ObjectIds, Geometry Source revisions, Modifier revisions, transforms/world revisions e Evaluation Quality. Se dependency mudar externamente, rebuild seguro ou cancel com diagnóstico.

## Budget

Usar `ShapeBuilderBudget` interno com limites de input triangles, decomposed triangles, components e custo de final union. Não expor esses knobs na UI comum. Excedeu → diagnóstico; nunca remesh automático.

## Undo

Hover e gestures não criam checkpoints. Commit cria uma única transaction com objects alterados/removidos/criados e seleção final. Cancel descarta partition + RegionPlan.

## Selection pós-commit

- Unite novo → selecionar resultado;
- Extract → selecionar extraídos;
- Remove → preservar o primeiro objeto sobrevivente relevante;
- múltiplos outputs explícitos podem ficar multi-selected.

Component selection antiga nunca sobrevive a topology substituída.

## Fixtures obrigatórias

1. cubes parcialmente sobrepostos;
2. disjuntos;
3. idênticos;
4. B contido em A;
5. A contido em B;
6. contato apenas por face;
7. contato apenas por edge;
8. contato apenas por vertex;
9. operand com loose parts fechadas;
10. solid côncavo;
11. non-manifold recusado;
12. open mesh recusada;
13. transforms/parenting;
14. negative scale;
15. materiais diferentes;
16. UVs diferentes;
17. múltiplos connected components.

## Testes de intenção

- Remove overlap remove a região compartilhada de todos os grupos;
- Extract overlap cria novo SceneObject e retira overlap dos source groups;
- Unite all equivale geometricamente a `A ∪ B`;
- Cancel mantém Document semanticamente idêntico;
- operand paramétrico intocado permanece Generator.

## Não objetivos adicionais

- persistent face IDs do kernel;
- CSG history;
- N-solid partition arbitrária;
- reboolean em pointer move;
- voxel/remesh;
- B-Rep;
- automatic retopology;
- semantic feature recognition.

O diferencial deve vir da interação e previsibilidade, não do acúmulo de subsistemas.


## Decisões detalhadas aprovadas

1. Boolean distingue resultado vazio de erro.
2. Solid Shape Builder usa decomposição dedicada que prepara os operandos uma única vez.
3. Connected face components vira query reutilizável de Geometry.
4. SolidRegion carrega proveniência mínima First / Second / Generated.
5. UV e material são preservados por face sempre que houver proveniência; não há relayout global obrigatório.
6. RegionPlan transitório preserva ownership e ObjectId de objetos não tocados.
7. Geometry Source paramétrica intocada permanece paramétrica; apenas outputs efetivamente alterados são materializados.
8. O espaço local do primeiro operand é o session space.
9. Picking pode percorrer múltiplos hits para regiões internas, com cycling e X-Ray Regions.
10. Hover/drag executam somente picking; booleans pesados acontecem na preparação ou commit.
11. Shape Builder usa budget interno e falha explicitamente sem remesh automático.
