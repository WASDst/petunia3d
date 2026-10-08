# Decals e Animated Decals

> **Status: aprovado.**

## Diagnóstico atual

O projeto já possui mais infraestrutura de decal do que a arquitetura atual deixa transparecer:

- Decal UV;
- decal projetado em superfície;
- alças Move/Scale/Rotate;
- largura em mundo;
- SVG source preservado;
- rerasterização SVG conforme área ocupada;
- recomposição parcial por tiles;
- Decal Set com variantes;
- `DecalVariantTrack` step-animation;
- playback pelo playhead;
- export de frames + JSON;
- metadata `extras.petunia.decal_sets` no GLB;
- bake estático universal.

Os principais problemas atuais são de modelo:

1. `DecalLayer` duplica a variante ativa em `image`/`source_svg` e novamente em `variants`;
2. cada variante embute um `Canvas`, em vez de reutilizar TextureResource;
3. `DecalAnchor` guarda ponto/normal/tangente em mundo, contrariando o novo SurfaceAttachment local;
4. a trilha possui fps/duração próprios, criando um segundo relógio além da timeline/Animate;
5. edição da animação vive no Inspector do Paint em vez de participar naturalmente da timeline;
6. static/animated/export estão acoplados ao Asset/Material legado.

## Regra principal

Animated Decal **não é outro tipo de layer**.

É a mesma DecalLayer com uma fonte visual que pode variar no tempo.

```text
DecalLayer
├ placement
├ decal set/source
├ layer opacity/blend
└ optional animation track
```

Placement não muda só porque o conteúdo é animado.

## DecalSet como recurso reutilizável

Separar conteúdo do decal de sua instância na layer.

Direção:

```rust
pub struct DecalSet {
    pub id: DecalSetId,
    pub name: String,
    pub variants: Vec<DecalVariant>,
    pub revision: Revision,
}
```

Um mesmo DecalSet pode ser usado por:

- olho esquerdo e direito;
- vários personagens;
- múltiplas layers;
- presets faciais.

Isso reduz memória e torna edição/reuso muito melhores.

## DecalVariant

Variante não embute Canvas duplicado.

Direção:

```rust
pub struct DecalVariant {
    pub id: DecalVariantId,
    pub name: String,
    pub texture: TextureId,
    pub vector_source: Option<DecalVectorSource>,
}
```

`DecalVectorSource` pode guardar SVG source quando houver.

A TextureResource é o raster/cache atual.

Se SVG for rerasterizado, atualiza a TextureResource da variante ou cria nova revisão da mesma identidade.

## Remover `DecalLayer.image` duplicada

A layer não mantém cópia da variante ativa.

Ela guarda apenas:

```text
DecalSetId
rest_variant: DecalVariantId
```

e resolve a TextureResource quando precisa compor.

Isso elimina:

```text
set_variant()
→ copy Canvas into DecalLayer.image
```

como fonte de verdade.

## DecalPlacement

Separar explicitamente os dois tipos existentes:

```rust
pub enum DecalPlacement {
    Uv(UvDecalPlacement),
    Surface(SurfaceDecalPlacement),
}
```

## UV decal

```rust
pub struct UvDecalPlacement {
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
}
```

Continua útil para:

- atlas work;
- UI-like details;
- decals deliberadamente presos à textura.

## Surface decal

Substituir `DecalAnchor` world-space pelo contrato aprovado de SurfaceAttachment.

Direção:

```rust
pub struct SurfaceDecalPlacement {
    pub attachment: SurfaceAttachment,
    pub width: f32,
    pub aspect: f32,
    pub rotation: f32,
    pub depth: f32,
}
```

O frame é avaliado em local space e transformado pelo SceneObject.

Move/Rotate/Parent do objeto não quebram o decal.

Topology change ambígua → `NeedsReattach`.

## Aspect ratio

Por default vem da variante/set.

Se variantes possuem dimensões diferentes, o DecalSet deve definir uma aspect ratio canônica ou exigir compatibilidade.

Recomendação V1:

- todas as variantes de um DecalSet compartilham aspect ratio lógica;
- import adapta/pad variantes incompatíveis ou recusa com diagnóstico;
- trocar variante nunca faz o decal 'pular' de tamanho.

## Manipulação

Preservar a gramática atual porque está boa:

- clique posiciona;
- drag move;
- corner handles escalam;
- top handle gira;
- numeric input;
- Esc cancela;
- um gesto = um Undo.

Na migração egui, reutilizar a interação e não o acoplamento Slint/AppState.

## Recomposição parcial

Preservar fortemente o comportamento atual:

```text
old decal bounds ∪ new decal bounds
→ affected tiles
→ recomposite only those tiles
```

Trocar variante também recompõe somente os tiles cobertos pelo decal.

Isso é essencial para Animated Decals.

## Animated Decal V1

O caso principal é **step animation de variantes**:

```text
Open Eyes
Open Eyes
Blink
Closed
Blink
Open Eyes
```

Sem interpolação entre imagens.

É ideal para:

- olhos;
- bocas;
- expressões;
- damage states;
- LEDs/indicators;
- sprites animados;
- detalhes estilizados.

## Track baseada em identidade, não índice

O modelo atual usa `variant: u32` como índice.

Isso obriga remapear todas as keys ao reordenar/remover variantes.

Trocar por:

```rust
pub struct DecalVariantKey {
    pub time: TimeSeconds,
    pub variant: DecalVariantId,
}
```

ou frame convertido para tempo somente na UI.

VariantId permanece estável mesmo se lista for reordenada.

## Um único relógio

Não manter fps e duração próprios dentro da track quando o decal estiver sendo animado em uma timeline/clip.

Regra:

```text
Timeline/Animation Clip
→ fornece current time
→ DecalVariantTrack.sample(time)
```

A track contém somente keys e interpolation mode.

Para preview standalone fora do Animate, Application pode fornecer um `PreviewPlayback` local sem persistir um segundo relógio no decal.

## Step interpolation

V1 usa apenas:

```text
Step
```

para variant switching.

Cross-fade entre duas textures fica fora da primeira versão porque:

- duplica sampling/composição;
- aumenta custo em Paint;
- complica export;
- não é necessário para facial sprites.

## Opacity animation

Adicionar como extensão de baixo custo:

```text
Decal Opacity Track
```

com Linear ou Step.

Isso permite:

- blink/fade;
- damage indicator;
- appear/disappear;
- efeitos simples.

Pode ser incluído depois do variant track sem novo placement model.

## Transform animation

Não animar reattachment sobre a superfície na V1.

Se quisermos movimento barato depois, animar apenas transform **local ao frame do decal**:

```text
offset U/V
rotation
scale
```

O SurfaceAttachment base permanece fixo.

Assim o decal desliza/oscila localmente sem raycast por frame.

Não animar ObjectId/Face attachment.

## Relação com Animation

Não colocar bone tracks dentro de Paint e não colocar decal internamente dentro de BoneTrack.

A solução recomendada é a timeline enxergar tracks heterogêneas via DTO/Application adapter.

Exemplo visual:

```text
Character_Run
├ Skeleton
│  ├ Hips
│  └ Head
└ Decals
   ├ Eyes.variant
   └ Mouth.variant
```

O modelo de persistência pode continuar com track específica de decal até o domínio Animation ser redesenhado.

Não criar um generic property-animation framework completo agora.

## Decal Animation Clip Binding

Para sincronizar animação facial com um clip de personagem, a track precisa saber a qual timeline/clip pertence.

Recomendação:

```rust
pub struct DecalAnimationBinding {
    pub clip: AnimationClipId,
    pub track: DecalVariantTrack,
}
```

ou equivalente quando AnimationClip ganhar IDs tipados.

Sem binding, decal fica estático em `rest_variant`.

Standalone looping local pode existir como preview/preset, mas não deve competir com a timeline final.

## Facial Decal Set

Animated Decals têm um caso de produto forte: rostos simples estilizados.

Um preset pode oferecer:

```text
Eyes
├ Open
├ Blink
├ Closed
├ Happy
└ Angry

Mouth
├ Neutral
├ Smile
├ Frown
├ A
├ E
└ O
```

Isso não exige rig facial complexo.

## Não criar um sistema facial separado

Eyes/Mouth são simplesmente DecalSets + tracks.

Presets podem organizar nomes e atalhos, mas a arquitetura continua genérica.

Isso permite usar o mesmo sistema para:

- placas;
- telas;
- damage decals;
- LEDs;
- sprites.

## Import de sequence

Feature de baixo custo recomendada:

```text
Import Image Sequence…
```

Arquivos ordenados:

```text
eye_000.png
eye_001.png
eye_002.png
```

viram:

```text
DecalSet variants
+ generated Step keys
```

Não criar outro tipo `ImageSequenceDecal`.

## Import de spritesheet

Também é viável, mas um pouco mais complexo.

V1 opcional:

- rows/columns;
- frame count;
- ordering;
- padding/margin.

Internamente cada frame pode virar região/variant lógica referenciando a mesma TextureResource atlas.

Isso economiza GPU memory/uploads.

## Atlas-backed variants

Para eficiência futura, permitir que uma variant seja:

```text
TextureId + optional UV rectangle
```

Assim um spritesheet pode ser um único TextureResource.

Não obrigar todas as variants a textures separadas.

Direção:

```rust
pub struct DecalImageRef {
    pub texture: TextureId,
    pub region: Option<TextureRegion>,
}
```

## Playback no editor

Quando o variant ativo muda:

1. resolve nova image ref;
2. calcula bounds do decal;
3. marca somente tiles afetados;
4. recompõe output TextureResource;
5. publica TextureUpdate dirty rectangles;
6. render-on-demand solicita frame.

Se a variant não mudou entre dois frames, nenhum trabalho de Paint é feito.

## Performance budget

Animated Decals são baratos quando:

- poucas layers animadas;
- baixa fps de troca;
- recomposição parcial;
- set/atlas compartilhado.

Adicionar diagnostics simples:

```text
Animated decals: 6
Variant switches/sec: 18
Affected texture area/sec: 1.4 MPix
```

Não impor um número mágico muito cedo.

Warn quando a composição real medida exceder budget do performance tier.

## Render strategy

Na V1, Animated Decal continua sendo **texture-authoring composition**, não um projector shader runtime.

Ou seja:

```text
variant changed
→ PaintDocument partial composite
→ TextureResource update
→ normal material rendering
```

Isso preserva renderer simples e export/bake previsível.

Não criar pass de projected decals no OpenGL renderer apenas para animação.

## Quando GPU decals poderiam fazer sentido

Somente se profiling mostrar que CPU partial composition é insuficiente para muitos decals simultâneos.

Mesmo nesse caso, avaliar um compositor GPU dedicado antes de transformar decal em scene renderer feature.

Não antecipar essa complexidade.

## Bake static

Static decal oferece:

```text
Bake to Raster Layer
```

Resultado:

- composite atual vira pixels raster;
- live decal layer é removida ou desabilitada conforme comando;
- um Undo.

Preservar comportamento existente.

## Bake animated

Animated decal não pode ser flattenado em uma única imagem sem perder informação.

Oferecer ações explícitas:

```text
Bake Current Frame
Convert to Static Variant
Export Sequence
Export Flipbook
```

Merge Down em animated decal deve pedir uma dessas decisões, não perder animação silenciosamente.

## Export universal

glTF/GLB core não representa generic texture swapping como decal animation portátil.

Manter fallback universal:

```text
current/rest frame baked into material texture
```

Assim qualquer viewer exibe aparência válida.

## Petunia metadata

Preservar e versionar metadata em `extras.petunia`, mas atualizar para IDs e recursos novos.

Metadata pode conter:

- DecalSet identity/name;
- variants;
- image references/atlas regions;
- Step keys;
- placement metadata quando útil;
- clip binding.

Não serializar JSON string dentro de JSON se puder armazenar objeto estruturado no exporter novo.

## Export Sequence

Exportar:

```text
frame_000.png
frame_001.png
...
animation.json
```

é simples e universal para ferramentas externas.

## Export Flipbook

Feature recomendada de alto valor:

```text
flipbook.png
flipbook.json
```

ou atlas metadata equivalente.

Pode conter somente frames efetivamente distintos.

Para step animation com frames repetidos, metadata guarda durations e evita duplicar pixels.

## glTF export

Recomendação:

1. bake rest/current appearance no PBR material padrão;
2. incluir DecalSet metadata Petunia;
3. incluir variant images/atlas quando export mode pedir metadata completa;
4. emitir warning de compatibilidade:

```text
Animated decal exported as static appearance for standard glTF viewers.
Petunia animation metadata was preserved.
```

Nunca fingir que glTF core reproduzirá a animação.

## Engine adapters

Futuramente adapters podem converter DecalSet para:

- Godot AnimatedTexture/atlas workflow;
- Unity sprite/flipbook material;
- custom engine shader;
- web sprite atlas.

Esses adapters ficam em IO/export, não no core de Paint.

## Animated SVG

SVG source continua estático por variant.

Animated SVG/CSS/SMIL não entra na V1.

Para animação vetorial:

- múltiplas SVG variants;
- step animation entre variants.

Isso preserva rerasterização nítida sem implementar um browser SVG animation engine.

## Channel support

DecalLayer vive dentro de um PaintDocument que já define o channel.

Logo o mesmo decal system pode operar sobre:

- BaseColor;
- Emission;
- Roughness;
- Metallic;
- Height;
- Opacity.

A source é interpretada conforme o tipo do canal.

Não guardar PaintChannel dentro de cada decal.

## Blend/opacity

Blend mode e opacity permanecem propriedades do PaintLayer.

Animated variant não altera blend mode automaticamente.

Isso evita que cada frame seja uma layer completa.

## Shared DecalSet

Editar um DecalSet compartilhado afeta todas as instances.

UI deve mostrar:

```text
Decal Set: Eyes
Used by 4 instances
[Make Unique]
```

Mesma filosofia de Material/Texture compartilhado.

## Undo

Operações:

- placement gesture → 1 Undo;
- add/remove/rename/reorder variant → 1 Undo;
- add/remove/move key → 1 Undo;
- set rest variant → 1 Undo;
- bake → 1 Undo.

Playback não entra no Undo.

## Validation

Strict validation:

- DecalSet não vazio quando referenciado;
- variant IDs únicas;
- referenced TextureIds existem;
- region dentro da texture;
- placement finito;
- width/scale > 0;
- SurfaceAttachment válido ou `NeedsReattach` explícito;
- keys ordenáveis e com variant IDs existentes;
- duplicate-time keys não ambíguas.

Legacy repair:

- converter `image + variants` para DecalSet;
- gerar IDs estáveis novos durante migration;
- converter world-space DecalAnchor para SurfaceAttachment quando possível;
- se impossível, preservar como legacy UV/static ou reportar necessidade de reattach;
- converter index keys para variant IDs.

## Accessibility/UX

Variantes devem ter:

- nome;
- thumbnail;
- índice visual opcional;
- teclado para anterior/próxima;
- labels além de cor.

Timeline:

- keys grandes o suficiente para aquisição;
- step tracks visualmente diferentes de curves;
- current variant textual;
- Reduced Motion não desliga playback autoral; afeta somente animações decorativas da UI.

## Migração recomendada

1. introduzir DecalSetId/DecalVariantId;
2. migrar variant Canvas para TextureResource/image ref;
3. remover `DecalLayer.image` duplicada;
4. introduzir DecalPlacement enum;
5. migrar DecalAnchor para SurfaceAttachment;
6. converter variant_index para rest_variant ID;
7. converter track index-based para ID-based;
8. remover fps/duração persistidos da track quando timeline/clip for autoridade;
9. criar timeline adapter para decal tracks;
10. manter partial tile recomposition;
11. migrar export metadata para recursos/IDs novos;
12. adicionar Import Image Sequence;
13. adicionar Flipbook export;
14. adicionar atlas-backed variants quando necessário;
15. portar UI para egui/Petunia UI Foundation.

## Testes obrigatórios

- static UV decal save/load;
- surface decal follows object transform;
- topology change → NeedsReattach;
- SVG rerasterization preserves sharpness;
- switching variant changes only affected tiles;
- same variant on next frame does zero recomposition;
- variant reorder does not break keys;
- deleting referenced variant is explicit/remaps safely;
- shared DecalSet updates all instances;
- Make Unique isolates instance;
- global playhead samples correct step key;
- playback does not add Undo;
- Bake Current Frame loses animation only by explicit command;
- GLB static fallback is correct;
- metadata preserves variant track;
- Sequence export deterministic;
- Flipbook export deduplicates repeated frames;
- low-end tier remains responsive with measured decal budget.

## Não objetivos V1

Não adicionar:

- arbitrary cross-fade texture animation;
- video textures;
- animated SVG engine;
- GPU projected decal renderer;
- per-frame surface reattachment;
- deformable decal mesh;
- full facial rig system;
- procedural lip-sync AI;
- generic property animation framework.

## Regra anti-bloat

O fluxo precisa permanecer:

```text
Add Decal
→ Place
→ Add Variants
→ Key Variant Changes
→ Play
```

Animated Decals devem oferecer animação facial e detalhes vivos sem transformar Paint em After Effects ou o renderer em um sistema de deferred decals.

## Decisões fechadas

1. Static e Animated Decal compartilham o mesmo DecalLayer.
2. Conteúdo reutilizável vira DecalSet.
3. Variantes usam DecalVariantId + TextureId/TextureRegion, não Canvas embutido.
4. DecalLayer deixa de duplicar a imagem da variante ativa.
5. Placement é explícito: UV ou Surface.
6. Surface Decal usa SurfaceAttachment, não DecalAnchor em world space.
7. Variantes compartilham área/aspect lógico estável.
8. Variant tracks usam IDs estáveis, não índices.
9. Timeline/clip fornece o relógio; a track não mantém um segundo FPS autoral.
10. Variant animation V1 usa Step.
11. Timeline consome decal tracks por adapter sem generic property-animation framework.
12. Facial animation é workflow/preset sobre DecalSet.
13. Import Image Sequence entra como feature de baixo custo.
14. Spritesheet/atlas é extensão natural usando TextureRegion.
15. Playback preserva partial tile recomposition.
16. Animated decal nunca é flattenado silenciosamente.
17. Export mantém static fallback + metadata Petunia, com Sequence/Flipbook explícitos.
18. Opacity animation pode vir na segunda etapa; moving SurfaceAttachment fica fora da V1.
