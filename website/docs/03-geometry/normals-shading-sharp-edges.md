# Normals, Shading e Sharp Edges

> **Status: proposta recomendada; aguarda aprovação.**

## Diagnóstico atual

Hoje há quatro conceitos parcialmente misturados:

- `Shading::Wireframe/Solid/MaterialPreview/Rendered` é modo de visualização do viewport;
- Flat/Smooth é persistido separadamente em `Project::smooth_shaded_assets`;
- não há Sharp Edges autorais persistentes;
- o renderer usa um threshold fixo de 30° para classificar feature edges visuais.

Além disso:

- render Smooth calcula média simples de face normals por vertex;
- export calcula média ponderada por área;
- nenhum dos dois respeita Sharp Edges;
- `recalculate_normals()` na verdade altera winding das faces;
- normals não precisam ser persistidas no authoring mesh.

## Regra principal

Separar três conceitos:

```text
Viewport Shading Mode
≠
Surface Shading Policy
≠
Face Orientation / Winding
```

### Viewport Shading Mode

Permanece estado do EditorSession:
- Wireframe;
- Solid;
- Material Preview;
- Rendered.

Não é propriedade geométrica do objeto.

### Surface Shading Policy

É propriedade persistente do SceneObject/geometry appearance:

```rust
pub enum SurfaceShading {
    Flat,
    Smooth,
}
```

Flat permanece default.

### Face Orientation

É determinada pelo winding autoral das faces. Não existe normal armazenada para corrigir.

## SurfaceShading no SceneObject

Eliminar a lista paralela `Project::smooth_shaded_assets` e mover a intenção para o objeto.

Isso elimina lookup global por UUID, simplifica duplicação/prefab, torna serialização local ao objeto e evita lista stale após delete/remap.

## Sharp Edges

Adicionar informação autoral explícita à EditableMesh:

```rust
pub struct EditableMesh {
    pub mesh: Shared<Mesh>,
    pub sharp_edges: HashSet<EdgeKey>,
    pub revision: Revision,
    pub topology_revision: Revision,
}
```

A localização física final pode mudar durante a migração, mas semanticamente Sharp pertence à geometria autoral.

Sharp Edge:
- separa smoothing groups;
- não precisa ser UV seam;
- não muda topologia;
- não muda triangulação;
- não é subdivision crease.

## Sharp não é feature edge

O threshold visual atual de 30° pode continuar existindo para overlays.

```text
Feature edge
→ derivada de boundary/non-manifold/dihedral angle
→ ajuda visual

Sharp edge
→ decisão autoral persistente
→ afeta normals
```

Não misturar os dois conceitos.

## Normals são derivadas

Não persistir face normals, vertex normals ou custom split normals.

Derivar a partir de positions, face winding, SurfaceShading e Sharp Edges.

Custom split normals editáveis ficam fora da V1.

## Corner normals

Smooth com Sharp Edge não pode ser representado por uma única normal por VertexIndex.

O mesmo vertex pode precisar de normals diferentes em faces adjacentes. A saída correta é por face-corner.

Isso combina naturalmente com FaceCorner.

## Algoritmo canônico

### Flat

Cada corner usa a normal geométrica da face.

### Smooth

Para cada face-corner:
1. iniciar na face incidente;
2. encontrar faces do one-ring conectadas ao mesmo vertex;
3. atravessar apenas edges que não estejam marcadas Sharp;
4. acumular face normals do smoothing island;
5. ponderar por corner angle;
6. normalizar.

Corner-angle weighting é o baseline recomendado porque reduz dependência do tamanho desigual das faces, funciona bem em low-poly e evita diferença Render × Export.

## Uma implementação compartilhada

Criar Geometry API equivalente a:

```rust
compute_corner_normals(
    mesh: &Mesh,
    shading: SurfaceShading,
    sharp_edges: &HashSet<EdgeKey>,
) -> CornerNormals
```

Render e Export consomem exatamente esse resultado.

Remover algoritmos independentes de `to_triangles_smooth` e `compute_normals` após migração.

## Render boundary

A representação já aprovada de RenderVertex é adequada para split normals.

Quando duas faces compartilham VertexIndex mas têm normals diferentes, o renderer produz RenderVertices distintos.

Não duplicar authoring vertices por causa de shading.

## Export

### glTF

O exporter duplica vertices somente na representação de export onde UV ou normal divergem por face-corner.

### OBJ

OBJ permite índices separados de position/uv/normal; emitir normal indices por face-corner.

## Geometry tools usam normals geométricas

Extrude, Push/Pull, Workplane from Face, Knife, SurfaceAttachment e Face Orientation usam `face_normal()` geométrica baseada em winding.

Nunca usar shading normal para decisões topológicas.

## Mark Sharp / Clear Sharp

Adicionar commands explícitos `Mark Sharp` e `Clear Sharp` para EdgeKeys selecionadas.

São operações de documento e entram no Undo.

## Smooth by Angle

Não criar Auto Smooth como estado mágico recalculado continuamente.

Criar command de conveniência `Smooth by Angle…`:
1. objeto vira `SurfaceShading::Smooth`;
2. calcula dihedral angle de cada internal edge;
3. edges acima do threshold são marcadas Sharp;
4. boundary edges funcionam naturalmente como border de smoothing;
5. grava Sharp flags explicitamente.

O threshold é parâmetro do comando, não propriedade semântica obrigatória permanente.

## Generators

Generators podem fornecer defaults úteis de SurfaceShading/Sharp:

- Box/Wedge: Flat por default;
- Cylinder/Cone: sides smooth, cap boundary Sharp;
- Sphere/Torus/Capsule: Smooth pode ser default;
- Extrude: cap/side boundary Sharp quando necessário;
- Revolve: profile corners podem gerar Sharp rings;
- Sweep: profile corners podem gerar Sharp longitudinal edges.

Depois de Make Editable, isso vira authoring state normal.

## Modifiers

Topology-preserving deformers preservam Sharp Edge identity e recalculam normals no resultado.

Mirror/Symmetry/Array/Thickness/Subdivide precisam propagar/remapear Sharp explicitamente.

## Shape Builder / Boolean

Quando SurfaceOrigin é conhecida, herdar Sharp quando a boundary corresponde a source edge conhecida.

Edges novas usam política determinística; nunca copiar EdgeKey antiga sem remap comprovado.

## Bevel

Bevel direto preserva Sharp em edges não afetadas, remove/remapeia Sharp da edge consumida e define a política das novas boundaries explicitamente.

## Recalculate Normals deve ser renomeado

O método atual propaga winding e usa signed volume; ele não recalcula normals persistidas.

Separar comandos:

### Flip Faces
Inverte winding das faces selecionadas.

### Orient Consistently
Propaga winding consistente por componente conectado. Funciona para meshes abertas e fechadas sem afirmar qual lado é fora.

### Orient Outward
Somente para closed manifold components: primeiro orienta consistentemente, depois usa signed volume para decidir exterior.

Para open mesh, Orient Outward é recusado ou exige referência explícita.

## Face Orientation overlay

Preservar. Ele visualiza winding/front/back geométrico, não Smooth/Sharp shading.

Deve funcionar no renderer OpenGL alvo.

## Normal overlay

Adicionar somente como ferramenta avançada/debug:
- Face normals;
- opcionalmente Corner/Shading normals.

Não é requisito de fluxo básico.

## Cache/revisions

CornerNormals são derivados e podem ser cached por:

```text
geometry revision
+ topology revision
+ SurfaceShading
+ sharp-edge revision
```

Mudança de UV/material não precisa invalidar normals.

## Validation

Sharp edge set precisa validar:
- ambos vertices existem;
- EdgeKey existe na topology autoral;
- sem self-edge.

Topology operations retornam remap/invalidation para Sharp edges assim como UV seams.

Não apagar Sharp silenciosamente.

## Relação com UV seams

Sharp e Seam são independentes:

```text
Sharp only
Seam only
Sharp + Seam
Neither
```

A UI pode oferecer ação de conveniência futura `Mark Sharp + Seam`, mas o modelo não os acopla.

## Migração recomendada

1. introduzir SurfaceShading no SceneObject;
2. migrar `Project::smooth_shaded_assets`;
3. introduzir Sharp Edge set com EdgeKey;
4. implementar `compute_corner_normals` canônico;
5. fazer OpenGL renderer consumir corner normals;
6. fazer OBJ/glTF exporters consumirem o mesmo cálculo;
7. adicionar Mark/Clear Sharp;
8. adicionar Smooth by Angle como command;
9. reorganizar Flip / Orient Consistently / Orient Outward;
10. adaptar generators/modifiers/topology remaps;
11. remover cálculo Smooth duplicado antigo.

## Testes obrigatórios

- cube Flat;
- cube Smooth sem Sharp;
- cube Smooth com todas edges Sharp = visualmente Flat;
- cylinder Smooth com cap boundaries Sharp;
- uneven face areas sem diferença Render/Export;
- UV seam sem Sharp continua smooth;
- Sharp sem UV seam continua split normal;
- Mark/Clear Sharp Undo/Redo;
- topology operation remapeia Sharp;
- Mirror/Array/Thickness/Subdivide propagam Sharp corretamente;
- glTF vertex split em hard-normal boundary;
- OBJ normal indices por corner;
- negative scale + winding/export;
- open mesh Orient Consistently;
- closed mesh Orient Outward;
- Face Orientation independente de SurfaceShading.

## Regra anti-bloat

Persistir somente decisões de authoring:
- winding;
- SurfaceShading;
- Sharp Edge flags.

Normals, smoothing islands e RenderVertices permanecem derivados.

Sem custom split normals, subdivision crease weights ou normal-editing tools na V1.
