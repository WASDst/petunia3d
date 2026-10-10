# Geometry — decisão arquitetural

> **Status: aprovado**

## Decisões aprovadas

1. A `Mesh` indexada permanece como representação autoral canônica.
2. Half-Edge permanece uma representação derivada de topologia; conceitualmente será tratada como `HalfEdgeTopology`, não como segunda malha autoral.
3. Selection sai completamente da Mesh e pertence à Application.
4. Operações de seleção pertencem à Application; queries topológicas permanecem em Geometry.
5. Índices recebem newtypes gradualmente, incluindo `VertexIndex`, `FaceIndex` e `EdgeKey`.
6. Faces suportam oficialmente triangles, quads e n-gons.
7. UV continua sendo face-corner, nunca um único atributo global por vertex.
8. Triangulação continua derivada e compartilhada entre render e picking.
9. `TopologyResult` e remapping são preservados e fortalecidos.
10. Validação estrita e reparo de input externo tornam-se operações distintas.
11. Knife, Poly Pen, Loop Cut, Boolean cleanup, Half-Edge e Weld são prioritariamente reutilizados.
12. Simplificação é dividida entre Contour Simplification e Mesh Cleanup.

## Ferramentas de simplificação

- Douglas–Peucker já existe e deve migrar para Geometry.
- Merge by Distance/Weld já existe e deve ser reutilizado.
- Adaptive Resample geral deve ser consolidado a partir das infraestruturas existentes.
- Limited Dissolve continua sendo gap real.

## Regra de dados inválidos

Dados internos inválidos representam bug e devem falhar de forma explícita.

Dados externos inválidos podem passar por sanitização/reparo controlado com relatório.
