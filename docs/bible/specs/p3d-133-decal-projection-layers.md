# P3D-133 — Decal & Projection Layers

<aside>
🧩

MVP aprovado por decisão do responsável em 01/10/2026 (P3D-165).

</aside>

## Objetivo

Adicionar PNG/JPEG/SVG como layer projetada e reposicionável sobre o modelo.

## Requisitos

Import, move, rotate, scale, opacity, mask/clip, visibility e persistência. Decal permanece não destrutivo enquanto for uma layer.

## Arquitetura

Asset original + projection descriptor + raster/cache. SVG pode ser rasterizado/cacheado para composição mantendo source/metadata. Compartilhar matemática de projection/reference quando útil sem misturar UX.

## Dependências

P3D-055, P3D-061, P3D-132, P3D-125.

## Testes / DoD

Formatos válidos/inválidos, transform, seams, save/load, missing source e bake/export quando aplicável.

## Reconciliação de implementação (02/10/2026)

SVG fonte/cache e importação DRAW/PAINT integrados; Projection commita em raster com preview temporário/oclusão/restrições; Stencil modula alpha/luma. Decal live usa o descriptor UV existente; manipulação live por attachment é evolução futura. Código compilado não certifica gates ou aceite manual. Limites e roteiro: [matriz MVP](../../development/paint-draw-mvp-completion-gap-matrix.md).
