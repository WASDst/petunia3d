# 44 — Pós-V1: Surface Paint Toolbox além do Brush

<aside>
🖌️

O Paint Workspace V1 permanece pequeno. A decisão do responsável em 01/10/2026 antecipou SVG decal/perfil, Projection/Stencil e Path Paint para o **MVP** (P3D-133/158/165). As demais propostas desta página permanecem uma evolução **pós-V1** de ferramentas de pintura e projeção voltadas a assets low-poly, sem transformar o Petunia em Substance Painter, Photoshop ou editor de mapas.

</aside>

# Baseline preservada

A V1 continua com:

- Brush/Pencil;
- Eraser;
- Fill;
- Picker;
- Layers;
- Masks/selection isolation conforme baseline;
- Paint on Model + editor 2D opcional.

As ferramentas abaixo são incrementais e dependem da fundação Paint/UV estar estável.

# Tier 1 — Alto valor / baixa complexidade

| Tool | Uso | Dificuldade |
| --- | --- | --- |
| Decal | Surface Detail editável na viewport | 3–5/10 conforme modo |
| Line / Shape | linhas, retângulos, círculos e formas simples | 2–3/10 |
| Gradient | gradiente linear/radial simples | 2–3/10 |
| Face / UV Island Fill | preencher rapidamente face, material ou ilha UV | 3/10 |

## Line / Shape

Útil para:

- faixas de roupa;
- painéis;
- bordas pintadas;
- olhos/bocas geométricos simples;
- sinalização;
- padrões low-poly/pixel-art.

A mesma tool deve funcionar no modelo 3D e na textura 2D quando semanticamente possível.

## Gradient

Inputs públicos mínimos:

- linear / radial;
- start/end;
- colors;
- opacity;
- optional dithering/quantization futuro para estética retro.

# Tier 2 — Surface workflows

## Projection / Stencil

Aplicar uma textura/material através de projeção manipulável na viewport.

Controles simples:

- move;
- rotate;
- scale;
- opacity;
- projection depth/falloff opcional avançado;
- commit to active layer ou keep live quando representado como Surface Detail.

Dificuldade estimada: **4/10** sobre a infraestrutura de Decal/Surface Manipulator.

## Clone / Patch

Copiar informação de textura de uma região para outra para corrigir seams, remover artefatos e repetir detalhes.

Dificuldade: **4–5/10**.

Guardrail: clone opera sobre TextureResources/layers existentes; não cria um raster subsystem paralelo.

# Tier 3 — Path Paint após Spline Core

Inspirado no valor observado em ferramentas modernas de texturing, mas simplificado.

```
Spline / Path
  ↓
Stroke Profile
  ↓
Paint Along Surface
```

Usos:

- costuras;
- zíperes pintados;
- stripes;
- rachaduras;
- fios desenhados como textura;
- trims pintados;
- cicatrizes lineares;
- linhas de painel.

Modes iniciais:

- Stroke;
- Ribbon;
- optional repeated stamp.

Pontos permanecem aderidos à superfície e podem ser editados antes do bake.

Dificuldade: **4/10 depois do Spline Core**; significativamente maior se implementado isoladamente.

# Ferramentas de baixa prioridade

## Smudge

Útil, porém menos alinhado ao foco low-poly e adiciona custo de filtro/brush sampling. Manter como candidato posterior, dificuldade ~4–5/10.

## Advanced Warp Painting

Não prioritário. Warp avançado pode existir somente quando decals/projection demonstram demanda real.

# Relação com Decals

`Decal` e `Projection` compartilham Surface Manipulator e raycast/attachment. A diferença pública:

- **Decal** = detalhe persistente/editável/reutilizável;
- **Projection** = forma temporária de aplicar uma imagem/material à layer;
- **Brush** = stroke livre.

# Relação com UV

Face/Island Fill e seleção por UV devem usar a mesma UV representation e selection semantics. Não duplicar seleção no Paint.

# Undo e persistência

- strokes e fills raster: transaction + tile diffs;
- live decals/path details: entity/property transactions;
- projection commit: raster transaction;
- preview não altera estado permanente até commit.

# Evidência de pesquisa externa

Ferramentas atuais do Substance 3D Painter separam Brush, Projection, Polygon Fill, Clone e Path em tools distintas, com toolbar contextual; Projection possui manipulação direta na viewport, e Path permite pontos editáveis aderidos à superfície. Blockbench também oferece shapes, gradient, bucket e ferramentas rápidas de pintura 2D/3D, reforçando que algumas ferramentas pequenas trazem alto valor sem exigir um sistema procedural completo.

# Regra anti-bloat

Não copiar todo o toolbox de programas de texturing. Uma nova Paint Tool entra quando:

1. resolve um workflow recorrente de asset;
2. pode compartilhar o mesmo Texture/Layer/Undo core;
3. não exige scene/world context;
4. pode ser explicada por interação visual direta.

# Ordem recomendada

```
V1 Paint foundation
→ Decal
→ Line / Shape
→ Gradient
→ Face / UV Island Fill
→ Projection / Stencil
→ Clone / Patch
→ Path Paint after Spline Core
→ optional Smudge / advanced warp
```

# Decisão

Expandir Paint por **tools pequenas e contextuais**, não por um editor 2D/3D generalista. Decal continua first-class tool; Path reaproveita Spline Core; todas as ferramentas permanecem asset-local.