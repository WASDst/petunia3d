# Scene Hierarchy, Parts, Collections e Transforms

> **Status: aprovado.**

## Diagnóstico atual

A cena atual usa:
- `Vec<Asset>` como ordem principal;
- `collection: Option<String>` por objeto;
- `Vec<String>` para collections;
- `position / rotation / scale` no Asset;
- reordenação linear no Outliner;
- nenhum parent/child real entre objetos.

O Outliner é chamado de hierárquico, mas a hierarquia real hoje é apenas organização por collections e seções especiais.

## Regra fundamental

Separar duas relações que não devem ser confundidas:

### Collection
Organização visual/lógica.

Não afeta transform.

Mover um objeto para uma Collection nunca altera posição, rotação, escala ou parenting.

### Parent
Relação espacial.

O transform local do filho é relativo ao parent.

Parenting deve ser uma ação explícita.

## SceneObject

Direção:

```rust
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub geometry: ObjectGeometry,
    pub transform: Transform,
    pub parent: Option<ObjectId>,
    pub collection: Option<CollectionId>,
    pub visible: bool,
    pub locked: bool,
}
```

Não armazenar `children: Vec<ObjectId>` no objeto.

Children são derivados consultando `parent`, evitando duas fontes de verdade.

## Transform

Introduzir um tipo único:

```rust
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}
```

A representação interna de rotação pode evoluir para quaternion onde fizer sentido, mas a API/UI pode continuar expondo Euler em graus.

## Local e World

Regra:

```text
object.local_transform
        ↓
parent world transform
        ↓
object.world_transform
```

```text
world(object) = world(parent) × local(object)
```

Objeto raiz:

```text
world(object) = local(object)
```

Geometry permanece em espaço local.

## Parenting

### Parent
Ao parentear preservando a aparência visual:

```text
new_local = inverse(parent_world) × old_world
```

Assim o objeto não "salta" no viewport.

### Unparent
Também preserva world transform:

```text
new_local = old_world
```

### Cycle prevention
É proibido criar:
- self-parent;
- ciclo indireto;
- parent inexistente.

A validação deve ser explícita antes do commit.

## Collection

Migrar de strings soltas para identidade estável:

```rust
pub struct Collection {
    pub id: CollectionId,
    pub name: String,
}
```

`SceneObject.collection` usa `Option<CollectionId>`.

Isso elimina bugs de rename por string.

### Escopo inicial
Collection é plana.

Não adicionar nested collections inicialmente.

Pastas aninhadas só entram se uso real demonstrar necessidade.

## Outliner

O Outliner pode apresentar:

```text
Scene
├── Collection: Environment
│   ├── Tree
│   └── Rock
├── Collection: Character
│   └── Body
│       ├── Eye.L
│       └── Eye.R
└── Unsorted
    └── Guide Curve
```

Mas Collection e Parent continuam conceitos diferentes.

A apresentação pode combinar os dois sem misturar suas semânticas.

## Reordering

Reordenar no Outliner não deve depender da posição no `Vec<SceneObject>`.

A ordem visual deve ser persistida separadamente ou derivada por um `display_order` estável.

No início, um inteiro `order` por objeto/collection é suficiente.

Não usar índice do Vec como identidade.

## Selection

Seleção usa `ObjectId`.

Parenting e Collection não alteram seleção automaticamente, exceto quando o comando explicitamente criar/remover objetos.

## Visibility e Lock

Visibility/Lock são propriedades do SceneObject.

Collections podem futuramente oferecer override de UI, mas a primeira versão não precisa propagar estado hierárquico complexo.

Evitar:
- tri-state visibility;
- inherited lock;
- layer masks;
na primeira arquitetura.

## Transform de múltipla seleção

Para múltiplos objetos:
- gizmo usa pivot definido pelo EditorSession;
- cada objeto recebe transformação em world;
- resultado é convertido de volta para local usando seu parent.

Isso evita comportamento incorreto em objetos com parents diferentes.

## Generators e parenting

Generators referenciam seus source objects por ObjectId.

A avaliação resolve os world transforms das fontes e converte para o espaço local do objeto gerado:

```text
source local
→ source world
→ generated object local
```

Parenting não muda o contrato do generator.

## SurfaceAttachment

SurfaceAttachment deve apontar para ObjectId.

Ao avaliar:
1. resolve target object;
2. resolve sua Geometry;
3. aplica world transform;
4. avalia attachment na superfície correspondente.

Mudança de transform do target não invalida topologia do attachment.

Mudança topológica continua usando revision/remap.

## Prefabs

O sistema atual de Prefab já acerta uma decisão importante: Prefab é separado da cena.

Manter esse princípio.

Entretanto a implementação atual copia `Asset` e desloca vertices diretamente.

No modelo novo, Prefab deve preservar:
- SceneObjects;
- transforms locais relativos ao root/pivot do prefab;
- geometry sources;
- relações parent/child internas.

Instanciar um prefab cria novos ObjectIds e remapeia referências internas.

### PrefabLink
`asset_id` evolui para `object_id`.

Não transformar Prefab em live scene graph obrigatório.

Link/revision continua simples:
- prefab source;
- instantiated objects;
- stale revision indication.

## Groups

Não criar um tipo `GroupObject` inicialmente.

Se o usuário quiser apenas organização:
→ Collection.

Se quiser transformação conjunta persistente:
→ Parent object / explicit parenting.

Se posteriormente houver necessidade real de Empty/Pivot, adicionar um `SceneObject` sem Geometry, em vez de criar outro sistema.

## Empty / Pivot

Não necessário no primeiro corte.

Mas a arquitetura permite futuramente:

```rust
ObjectGeometry::None
```

ou um `SceneObjectKind::Empty`.

Só introduzir quando parenting, rig helpers ou layout demonstrarem necessidade concreta.

## Import glTF

glTF possui scene hierarchy real.

Importação deve:
- criar SceneObjects;
- preservar node transforms;
- preservar parent relations quando relevantes;
- não aplicar/bakear todos os transforms nos vertices por padrão.

OBJ continua plano por não possuir scene graph equivalente.

## Export

glTF exporta hierarchy/transforms quando possível.

OBJ resolve world transforms e exporta geometry final, pois OBJ não oferece scene hierarchy equivalente.

## Migração recomendada

1. introduzir `Transform`, `ObjectId`, `CollectionId`;
2. converter collection string → CollectionId com compatibilidade no loader;
3. adicionar `parent: Option<ObjectId>`;
4. mover `Project.active` para EditorSession;
5. atualizar Outliner para IDs estáveis;
6. implementar parent/unparent preservando world transform;
7. adaptar multi-selection transforms;
8. adaptar generators e SurfaceAttachment;
9. migrar Prefab;
10. atualizar glTF import/export.

## Regra anti-bloat

Não criar:
- ECS;
- arbitrary component system;
- nested collection graph;
- visibility inheritance complexa;
- constraints genéricas;
- scene graph com múltiplos parents.

A cena começa como uma árvore simples de parenting + collections planas independentes.


## Decisões fechadas

1. Collections são organização e nunca afetam transforms.
2. Parenting é uma relação espacial explícita e independente de Collection.
3. `SceneObject` possui `parent: Option<ObjectId>`; children são derivados.
4. Geometry permanece em espaço local; world transform deriva da cadeia de parents.
5. Parent/Unparent preservam world transform.
6. Self-parent e ciclos são proibidos antes do commit.
7. Collections usam `CollectionId`, não Strings.
8. Collections permanecem planas inicialmente.
9. Ordem visual do Outliner deixa de depender da posição do objeto no `Vec`.
10. Multi-selection opera em world space e converte o resultado para o local de cada parent.
11. SurfaceAttachment aponta para `ObjectId`.
12. Prefabs preservam objetos, transforms e parenting; não deslocam vertices diretamente.
13. Não criar `GroupObject`, ECS ou component system genérico nesta etapa.
14. glTF preserva hierarchy/transforms; OBJ resolve transforms para geometria exportada.
