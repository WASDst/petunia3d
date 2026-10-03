# P3D-165 — Surface Paint Toolbox

<aside>
🖌️

Estado: **MVP aprovado por decisão do responsável (01/10/2026)**. Expandir Paint por ferramentas pequenas sobre a mesma Texture/Layer/Undo foundation, sem criar mini-Photoshop/Substance.

</aside>

# Tier 1

Decal Tool, Line/Shape, Gradient, Face/UV Island Fill.

# Tier 2

Projection/Stencil com viewport manipulator e Clone/Patch.

# Tier 3

Path Paint após Spline Core: Stroke, Ribbon e repeated stamp aderidos à superfície.

# Decal vs Projection vs Brush

Decal permanece entidade live/reutilizável; Projection é interação temporária que commita em layer; Brush cria stroke livre.

# Shared Surface Manipulator

Move/slide on surface, rotate around normal, scale, mirror, opacity e preview são compartilháveis entre Decal e Projection, usando P3D-158.

# Undo

Raster operations usam tile diffs; live surface entities usam property transactions; preview não modifica estado final antes do commit.

# Testes / DoD

3D/2D consistency, seams, masks, undo, layer order, projection preview, path surface attachment e save/load.
# Revisão de escopo de 01/10/2026

SVG como decalque e como fonte de perfis DRAW, Projection/Stencil e Path Paint fazem parte do MVP. O Shape Builder começa poligonal; reconstrução fiel das curvas fica para V1. A decisão amplia a entrega sem alterar a separação entre entidades live, projeções temporárias e strokes. Implementação e validação devem ser classificadas separadamente; código isolado sem bridge não certifica entrega.
