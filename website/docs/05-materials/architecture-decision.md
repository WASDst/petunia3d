# Materials, Textures e Paint Binding

> **Status: aprovado.**

## Diagnóstico atual

O projeto já possui uma base útil de material, mas hoje existem várias autoridades concorrentes para aparência.

No Asset atual:

```text
base_color
texture: Option<Canvas>
material_id
paint_stack
Face.material_slot
```

Ao mesmo tempo, `Material` armazena diretamente seis `Canvas`:

- albedo;
- normal;
- roughness;
- metallic;
- emission;
- height.

Isso produz duplicação de pixels, bindings pouco claros e caminhos legados que precisam sincronizar Asset.texture com Material.albedo_texture.

Também há uma inconsistência entre:
- `Asset.material_id`, que representa apenas um material;
- `Face.material_slot`, que permite múltiplos materiais por face;
- ausência de uma tabela explícita de slots por objeto.

## Objetivo

Separar quatro conceitos:

```text
Material
→ parâmetros de superfície e referências de textura

TextureResource
→ pixels/imagem e revisão

MaterialBinding
→ quais materiais um SceneObject usa

Paint
→ ferramenta/processo que modifica ou produz TextureResources
```

Nenhum desses conceitos pertence ao renderer.

## MaterialId e TextureId

Usar IDs tipados:

```rust
pub struct MaterialId(Uuid);
pub struct TextureId(Uuid);
pub struct MaterialSlotIndex(u16);
```

UUID permanece como representação persistente; newtypes evitam misturar IDs semanticamente diferentes.

## Material Store

Document mantém materiais como recursos independentes:

```text
Document
├ SceneObjects
├ Materials
├ Textures
└ ...
```

Não embutir cópia de Material em SceneObject.

Objetos referenciam materiais.

## Texture Resource

Material não deve possuir pixels diretamente.

Direção:

```rust
pub struct TextureResource {
    pub id: TextureId,
    pub name: String,
    pub image: TextureImage,
    pub revision: Revision,
}
```

`TextureImage` continua inicialmente simples e CPU-owned.

A implementação pode reaproveitar `Canvas` durante a migração, mas Canvas deixa de ser campo embutido de cada Material.

## Texture storage

Não criar sistema de streaming virtual, sparse texture ou asset database complexa na V1.

Pixels permanecem autocontidos no documento/projeto quando essa for a política do formato.

Imported external paths podem ser preservados como metadata/origin, mas o documento salvo deve saber reconstruir o recurso sem depender silenciosamente de um arquivo que desapareceu.

## Material model inicial

O modelo atual declara:

```text
PBR
Unlit
Toon
Glass
Emissive
```

mas o renderer não possui paridade robusta para todos.

Não manter uma promessa arquitetural maior que a implementação.

Recomendação V1:

```rust
pub enum MaterialModel {
    Standard,
    Unlit,
}
```

### Standard

Metallic/Roughness PBR simples:

- Base Color;
- Roughness;
- Metallic;
- Normal;
- Emission;
- Opacity/Alpha.

### Unlit

Base Color + texture + alpha, sem iluminação.

## Toon

Não apagar o conceito definitivamente, mas remover de requisito central da V1.

Toon pode voltar como MaterialModel quando houver:
- shader real;
- parâmetros definidos;
- export behavior explícito;
- testes visuais.

Não implementar como simples label que cai no mesmo shader Standard.

## Emissive

Emission não precisa ser `ShaderProfile::Emissive`.

É propriedade do Standard material:

```text
emission_color
emission_strength
emission texture
```

Um material puramente emissivo é apenas Standard com contribuição emissiva dominante.

Isso reduz um profile artificial.

## Glass

`Glass` atual promete mais do que o renderer entrega.

Glass real implica pelo menos:
- transmission;
- IOR;
- refraction policy;
- sorting/transparency;
- environment/reflection behavior.

Portanto Glass sai da V1 canônica.

Enquanto não houver implementação real, materiais transparentes usam AlphaMode e não se chamam Glass fisicamente correto.

## AlphaMode

Preservar:

```rust
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}
```

Mas documentar limitações.

### Opaque
Pipeline principal.

### Mask
Alpha test/cutoff no fragment shader.

### Blend
Pass transparente separado.

V1 pode ordenar por objeto/primitive depth aproximado; não prometer order-independent transparency.

## Material

Direção:

```rust
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub model: MaterialModel,

    pub base_color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub normal_scale: f32,
    pub emission_color: [f32; 3],
    pub emission_strength: f32,

    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,

    pub textures: MaterialTextures,
    pub revision: Revision,
}
```

## MaterialTextures

Direção explícita, não mapa stringly-typed:

```rust
pub struct MaterialTextures {
    pub base_color: Option<TextureBinding>,
    pub normal: Option<TextureBinding>,
    pub roughness: Option<TextureBinding>,
    pub metallic: Option<TextureBinding>,
    pub emission: Option<TextureBinding>,
    pub height: Option<TextureBinding>,
}
```

Height pode ser authoring data mesmo antes de displacement real existir no renderer.

A UI precisa rotular honestamente quando um canal é armazenado/exportado mas não afeta o viewport.

## TextureBinding

Uma binding é mais que TextureId:

```rust
pub struct TextureBinding {
    pub texture: TextureId,
    pub uv_set: UvSetIndex,
    pub sampler: TextureSampler,
}
```

V1 usa somente UV set 0, mas o tipo evita assumir isso para sempre.

Não adicionar transformação UV por material antes de haver necessidade real.

## Color space

Definir semanticamente:

```text
Base Color → sRGB
Emission   → sRGB
Normal     → linear/data
Roughness  → linear/data
Metallic   → linear/data
Height     → linear/data
```

A conversão sRGB ↔ linear precisa ser consistente entre import, sampling CPU, renderer e export.

Não multiplicar bytes sRGB como se fossem valores lineares.

Esse é um bug de qualidade visual, não uma otimização.

## Sampler

Começar pequeno:

```rust
pub struct TextureSampler {
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    pub filter: TextureFilter,
}
```

V1:
- Repeat;
- Clamp;
- Linear;
- Nearest.

Anisotropy é capability/quality option do renderer, não parte obrigatória da definição autoral.

## Material Binding por objeto

Substituir `SceneObject.material_id` único por slots explícitos:

```rust
pub struct MaterialBinding {
    pub slots: Vec<MaterialId>,
}
```

SceneObject:

```rust
pub struct SceneObject {
    // ...
    pub materials: MaterialBinding,
}
```

Slot 0 é o material principal/default.

## Face material slot

Face continua armazenando somente:

```text
MaterialSlotIndex
```

e nunca MaterialId direto.

Assim Geometry continua independente do Document material store.

```text
Face.material_slot = 2
        ↓
SceneObject.materials[2]
        ↓
MaterialId
        ↓
Material
```

## Default slot

Recomendação:

- toda surface face resolve para um slot válido;
- ausência legada de slot equivale a slot 0;
- durante a migração, `Option<usize>` pode permanecer temporariamente;
- modelo final prefere `MaterialSlotIndex` com default 0.

Isso reduz branches no renderer/export.

## Slot removal

Remover material slot é operação explícita.

Se faces usam o slot:
- UI oferece remap para outro slot;
- command atualiza faces via Geometry/Application transaction.

Nunca deslocar índices silenciosamente e deixar faces apontando para outro material.

## Shared materials

Dois objetos podem usar o mesmo MaterialId.

Editar o material altera ambos.

UI deve tornar isso visível:

```text
Material: Red Plastic
Used by 3 objects
[Make Unique]
```

`Make Unique` duplica Material com novo MaterialId e troca somente o slot do objeto atual.

## Shared textures

Materiais diferentes também podem referenciar o mesmo TextureId.

Isso evita duplicação de:
- imagens importadas;
- decals baked;
- painted outputs reutilizados.

## Paint não pertence ao Material struct

O Material aponta para o resultado de textura.

Paint é um authoring workflow separado.

Não armazenar `PaintLayerStack` dentro de Material.

Também não deixar SceneObject.texture competir com o Material.

## Boundary do Paint

A arquitetura recomendada para o capítulo de Paint será:

```text
Paint Session
     ↓
Paint Document / Layer Stack
     ↓
composite
     ↓
TextureResource
     ↓
Material channel
```

O detalhe de ownership da layer stack será fechado no domínio Paint.

Neste capítulo fica aprovada a regra:

> o renderer e o Material consomem TextureResource; eles não conhecem brush, layer, decal ou paint session.

## Legacy Asset.texture

`Asset.texture` é uma duplicação de `Material.albedo_texture`.

Migrar:

```text
Asset.texture
→ TextureResource
→ Material base_color binding
```

Depois remover o campo.

Nenhum caminho novo deve gravar pixels simultaneamente em Asset.texture e Material.

## Legacy Asset.base_color

`Asset.base_color` também duplica Material.base_color.

Migrar para:
- material do slot 0; ou
- viewport display color separado, se houver necessidade real de cor de objeto sem Material.

Não manter o mesmo campo com duas semânticas.

## Display Color

Se o modo Solid precisar de cor independente do Material Preview, usar propriedade explícita:

```rust
pub struct ViewportDisplay {
    pub color: [f32; 4],
}
```

ou equivalente no SceneObject.

Não reutilizar Material.base_color implicitamente para tudo.

Essa decisão pode ser finalizada no capítulo de UI/viewport appearance.

## PaintLayerStack atual

A implementação contém valor real:

- raster layers;
- opacity/visibility;
- blend modes;
- effects;
- decals;
- SVG source preservation;
- deterministic composition.

Classificação:

```text
layer/effect algorithms → REUSE
ownership/integration   → REFACTOR
```

Não jogar fora esse trabalho.

## Blend modes

O comentário atual diz que Multiply/Add/Screen existem sem UI dedicada.

Isso é aceitável como implementação interna, mas o produto não deve declarar uma feature como pronta apenas porque a enum variant existe.

Quando expostos na UI:
- devem possuir visual parity;
- tests de composição;
- export/bake semantics.

## SurfaceRecipe

`SurfaceRecipe` atual é um graph de processamento de textura CPU, não um material shader graph.

Preservar essa distinção.

Ele pode sobreviver como:
- procedural texture recipe;
- effect recipe;
- bake/export tool.

Não conectar nodes diretamente ao shader do renderer na V1.

Avaliação:

```text
SurfaceRecipe
→ CPU evaluation/cache
→ TextureResource
→ Material
```

Não:

```text
SurfaceRecipe
→ runtime GPU node material
```

Isso mantém o renderer simples.

## Normal maps

Normal texture requer tangent space coerente.

Não basta amostrar RGB.

Precisamos de uma etapa derivada:

```text
RenderMesh
+ UV
+ corner normal
→ tangent/bitangent basis
```

Tangents são derivados, não persistentes no authoring Mesh.

Para glTF export:
- exportar tangents quando gerados ou deixar consumer recalcular conforme contrato;
- nunca usar tangent inconsistente com FaceCorner splits.

## Height

Height texture inicialmente:
- pode ser pintada;
- pode ser exportada como recurso quando formato/workflow suportar;
- pode alimentar future bump/parallax.

Não deslocar geometria implicitamente.

Displacement real é feature futura explícita.

## PBR renderer V1

Material Preview deve suportar de verdade:

- base color factor × texture;
- metallic;
- roughness;
- normal map;
- emission;
- alpha mask/blend.

Não precisamos implementar:
- clearcoat;
- sheen;
- anisotropy;
- subsurface;
- transmission;
- iridescence;
- volume.

Isso é suficiente para um modelador enxuto e exportável para glTF.

## Solid versus Material Preview

### Solid

Não precisa respeitar Material completo.

Usa:
- viewport display color;
- studio light/matcap;
- AO;
- geometry shading.

### Material Preview

Usa Material + TextureResource + PBR simplificado.

Isso evita que material pesado prejudique edição básica.

## Rendered

Pode inicialmente usar o mesmo material model de Material Preview com luzes reais da cena.

Não inventar um segundo sistema de material para Rendered.

## Material revisions

Cada Material possui revision própria.

Cada TextureResource possui revision própria.

Alterar:
- roughness → invalida material uniforms/binding;
- texture pixels → invalida texture upload;
- UV → invalida RenderMesh;
- selection → não invalida nenhum dos anteriores.

Evitar `project.material_revision` global como mecanismo principal.

Uma revisão global pode permanecer temporariamente para migração/compatibilidade.

## Texture revisions e dirty rects

Preservar a infraestrutura já existente de dirty texture updates.

Direção:

```rust
pub struct TextureUpdate {
    pub texture_id: TextureId,
    pub revision: Revision,
    pub dirty_regions: Vec<TextureDirtyRect>,
}
```

Não usar ObjectId/AssetId como identidade de textura depois da migração.

## Validation

Aplicar a mesma filosofia já aprovada para Geometry.

`Material::validate()` mutante atual deve ser dividido.

### Strict validation
Não modifica:
- finite factors;
- ranges;
- referenced TextureIds exist;
- alpha cutoff valid;
- no invalid sampler enum/state.

### Legacy/import repair
Pode:
- clamp glTF values conforme spec;
- substituir campos legados conhecidos;
- reportar texture decode failures;
- preencher defaults documentados.

Sempre com Import/Repair report.

## Texture validation

Strict:
- width/height > 0;
- dimensions under project limits;
- byte length matches format;
- supported format.

`Canvas::validate()` mutante atual vira repair de boundary/legacy, não rotina interna geral.

## Import glTF

A base atual é boa e deve ser preservada.

Migrar:
- glTF images → TextureResources;
- glTF materials → Materials com TextureIds;
- glTF primitive material index → SceneObject MaterialBinding slot;
- Face.material_slot → MaterialSlotIndex local ao objeto.

Não espelhar albedo em `Asset.texture`.

## Export glTF

Export:
- agrupa primitives por material slot;
- resolve MaterialId por MaterialBinding;
- escreve textures uma vez por TextureId;
- deduplica shared TextureResources;
- respeita sRGB/data channel semantics.

Isso melhora arquivos exportados e evita imagens duplicadas.

## OBJ/MTL

OBJ:
- material slots viram `usemtl`;
- MTL representa subset compatível;
- propriedades não representáveis geram ExportReport warning, não sumiço silencioso.

## Shape Builder / Boolean

A decisão anterior de provenance encaixa diretamente aqui.

Ao combinar objects:
- faces preservam MaterialSlotIndex de origem semanticamente;
- output SceneObject constrói uma nova MaterialBinding;
- slots A/B são remapeados deterministicamente;
- Materials não são duplicados, apenas IDs são reutilizados.

## Generators

Generator faces usam slots locais.

V1 recomendada:
- slot 0 = material principal;
- Extrude pode futuramente expor cap/side slot assignments;
- não criar materiais automaticamente.

Se cap/side assignments entrarem, apontam para MaterialSlotIndex existente.

## Modifiers

Modifiers preservam MaterialSlotIndex das faces descendentes quando possível.

Faces novas:
- Thickness side walls usam slot configurável ou slot 0;
- Subdivide herda source face slot;
- Mirror/Array copiam slots.

## Texture memory

Para hardware fraco:
- GPU upload sob demanda;
- mipmaps geradas quando usadas no viewport;
- textures fora de uso podem ser evictadas da GPU sem remover CPU/document data;
- limite de import já existente deve continuar.

Não implementar virtual texture.

## Undo

Editar pixels grandes por snapshot completo pode ficar caro.

A arquitetura de Paint deverá evoluir para tile/dirty-region undo quando necessário.

Mas Material parameter changes usam commands pequenos.

Não bloquear este redesign esperando um undo perfeito de Paint.

## Migração recomendada

1. introduzir MaterialId / TextureId / MaterialSlotIndex;
2. introduzir TextureResource store;
3. migrar Material canvases para TextureBindings;
4. introduzir SceneObject MaterialBinding;
5. migrar Face.material_slot para slot local explícito;
6. migrar Asset.texture/base_color;
7. adaptar glTF import/export;
8. adaptar RenderFrameInput/material cache;
9. migrar Paint output para TextureResource;
10. preservar/reencaixar PaintLayerStack;
11. manter SurfaceRecipe como CPU texture authoring;
12. remover ShaderProfiles não implementados;
13. dividir Material/Canvas validation em strict + repair.

## Não objetivos V1

Não adicionar:
- material node graph GPU;
- arbitrary custom shaders;
- glass/refraction fisicamente correta;
- subsurface;
- clearcoat;
- procedural GPU materials;
- UDIM;
- virtual textures;
- multi-UV editing avançado;
- displacement geométrico automático.

## Regra anti-bloat

Um material Standard simples e exportável, um modelo Unlit, recursos de textura reutilizáveis e um Paint pipeline separado.

A riqueza visual deve vir primeiro de:
- boas UVs;
- boas texturas;
- Paint acessível;
- PBR básico correto;
- iluminação de viewport legível.

Não da quantidade de knobs de shader.


## Decisões fechadas

1. Material referencia TextureResources em vez de armazenar Canvas diretamente.
2. SceneObject usa MaterialBinding com slots locais.
3. Face armazena MaterialSlotIndex local, não MaterialId.
4. V1 canônica usa Standard + Unlit.
5. Emission permanece propriedade do Standard; Glass/Toon saem da promessa central até terem implementação real.
6. TextureResource possui identidade/revision próprias e pode ser compartilhada.
7. Asset.texture e Asset.base_color deixam de competir com Material.
8. Paint produz TextureResource; Material e Renderer não conhecem layers/brushes.
9. SurfaceRecipe permanece authoring CPU de textura, não material node graph GPU.
10. Normal/tangent continuam dados derivados no boundary de render/export.
11. Material/Texture validation segue strict validation + explicit repair.
