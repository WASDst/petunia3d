# UV Workspace e Editor

> **Status: aprovado.**

## Contexto já aprovado

A representação autoral de UV continua sendo per-face-corner:

```text
Face
└ FaceCorner
   ├ VertexIndex
   └ TextureCoordinates
```

Seams usam `EdgeKey`. Pins usam `FaceCornerIndex`. Este capítulo define somente o editor/workspace e os algoritmos.

## Diagnóstico atual

A base já possui Planar/Box/View Projection, xatlas Auto Unwrap, seams, pins, Stitch, Relax, islands, Pack, texel density, overlap/stretch diagnostics e coverage masks para Paint.

Os principais problemas são:

1. seleção UV atual é face-based;
2. `UvModule` depende de `AppState`;
3. pins usam tuplas cruas;
4. pack atual é simples e desperdiça espaço;
5. Relax atual é Laplacian local e não é um solver real de stretch;
6. `unwrap_fallback()` chama `mesh.triangulate()` e destrói quads/n-gons autorais.

O item 6 é um bug arquitetural importante e precisa ser corrigido antes de considerar Auto Unwrap seguro.

## Regra principal

UV é edição de atributo da mesma geometria, não um segundo mesh persistente.

```text
EditableMesh topology
      ↓
FaceCorner UV
      ↓
UV Editor
```

Não criar `UvMesh` autoral paralelo.

## Workspace

```text
3D Viewport             UV Editor
────────────             ─────────
Mesh/Surface             Islands/Corners
Seams                    Pins
Selection                Checker/Texture
Paint preview            Density/Stretch
```

As duas views compartilham Document e Application.

## UV Selection

Substituir `uv_selected: HashSet<face>` por seleção UV explícita:

```rust
pub struct UvSelectionState {
    pub mode: UvSelectionMode,
    // representação interna pode ser compacta/derivada
}
```

V1:

```text
Corner
Edge
Face
Island
```

UV vertex não precisa de um conceito separado: é um FaceCorner ou conjunto de corners coincidentes.

## Sync Selection

Adicionar opção explícita `Sync Selection`.

Quando ligada, seleção 3D e UV são sincronizadas semanticamente. Quando desligada, a seleção UV é independente.

Não manter o comportamento atual de espelhar seleção globalmente via evento sem controle do usuário.

## UV Edge

Uma UV edge não é a mesma coisa que `EdgeKey`.

Uma única edge 3D pode gerar duas UV edges quando existe uma seam.

Direção:

```rust
pub struct UvEdge {
    pub face: FaceIndex,
    pub corner_a: CornerIndex,
    pub corner_b: CornerIndex,
}
```

Mark/Clear Seam continua atuando no `EdgeKey` 3D.

## Islands

`UvIsland` é derivada e não persistente.

Island = componente conectado por edges sem seam e com continuidade UV compatível.

Diagnostics deve detectar inconsistências como:

```text
UV split without seam
Seam without UV split
```

Sem auto-repair silencioso.

## Pins

Pins usam `FaceCornerIndex`.

Regra importante: solver/unwrap/relax respeitam pins, mas um direct transform feito pelo usuário sobre um pinned corner pode movê-lo; ele continua pinned na nova posição.

Isso é mais intuitivo que o comportamento atual em que Move/Scale/Rotate simplesmente ignoram qualquer corner pinned.

## UV Transform

V1:

```text
Move
Rotate
Scale
```

Reutilizar modal transforms da Application, com numeric input e snapping. Um gesto = um Undo.

## UV Snapping

Snap 2D pequeno e próprio do UV editor:

```text
Grid
Pixel
Corner
Edge
```

Pixel snapping depende da resolução da TextureResource ativa e distingue center/boundary quando necessário.

## Background e Checker

Background do editor:

```text
Checker
Active Texture
Neutral
```

Checker também pode ser mostrado temporariamente no 3D viewport para avaliar stretch. Não vira Material persistente.

## Stretch

Adicionar overlay derivado.

V1 começa com `Area Stretch` robusto. `Angle Stretch` entra quando o cálculo estiver validado.

Cor não é a única informação: painel/status mostra métricas numéricas.

## Texel Density

Tratar explicitamente como px/world-unit.

Operações:

```text
Measure Density
Set Density
Match Density
Equalize Selected Islands
```

Usar a resolução real da TextureResource/canal ativo, não somente `texture_w`.

## Projection

Preservar:

- Planar;
- Box;
- From View.

Adicionar com baixo custo:

- Cylindrical;
- Spherical.

Cada operação atua na face selection, ou no objeto inteiro quando nada estiver selecionado.

## Auto Unwrap / xatlas

xatlas permanece provider genérico, mas nunca pode mudar a topology autoral.

Fluxo correto:

```text
authoring faces
→ canonical derived triangulation
→ triangle-corner → original FaceCorner provenance
→ xatlas
→ map UV output back to original FaceCorners
→ topology unchanged
```

Quads e n-gons permanecem quads/n-gons.

Tipos do provider não vazam para Project/Application.

## Auto Unwrap e seams

Recomendação: chart boundaries produzidas pelo Auto Unwrap atualizam/derivam seam flags explicitamente.

Isso deixa o resultado reeditável e previsível.

Seams manuais compatíveis são preservadas; o provider pode adicionar novas.

## Native solvers

Não implementar LSCM/ABF++ próprio agora.

V1 usa:

```text
Planar
Box
Cylindrical
Spherical
xatlas Auto Unwrap
Relax
```

## Relax

O `relax_uv()` atual não deve ser apresentado como "minimize stretch": ele faz smoothing local e pode encolher islands.

Classificação: **REPLACE/UPGRADE**.

A nova versão precisa:
- operar por island;
- respeitar pins;
- respeitar seam boundaries;
- minimizar stretch de forma documentada;
- expor apenas iterations/strength simples.

Preferir solver existente/leve quando viável.

## Stitch

O Stitch atual move os dois lados para a média da edge. Isso pode distorcer duas islands.

Nova regra:

```text
active/stationary side
+
moving side
→ align shared edge
→ merge matching corners
→ clear seam
```

Para casos ambíguos, preview ou recusa segura.

## Split UV

Adicionar `Split UV`:

- cria descontinuidade UV ao longo da seleção;
- marca seam correspondente;
- não duplica vertex 3D.

FaceCorner torna isso barato e natural.

## Pack

O pack atual fica como fallback simples.

O pack V1 deve ter:
- selected-only;
- deterministic result;
- optional 90° rotation;
- fixed/pinned islands quando solicitado;
- normalized 0..1;
- padding em pixels.

Preferir provider de packing robusto, inclusive xatlas quando adequado.

## Padding

A UI usa pixels:

```text
Padding: 4 px
```

Application converte para UV a partir da resolução ativa.

## Overlap

Overlap não é automaticamente corrupção.

Casos válidos incluem mirrored/stacked UV.

Diagnostics informa overlap e Paint avisa que várias surfaces podem receber o mesmo texel.

Pack não cria overlap acidental.

## UV Health

Consolidar:

```text
Islands
Overlap %
Zero-area faces
Out-of-range corners
Mean stretch
Max stretch
Texel density
```

Cada item pode oferecer `Select` e somente oferece `Fix` quando a correção for inequívoca.

UV fora de 0..1 é diagnostic, não validation error.

## Multi-material

O background/canal ativo vem do `MaterialSlotIndex` selecionado.

Texel Density usa a resolução da TextureResource do slot/canal atual.

## Paint integration

UV Editor e Paint compartilham as mesmas FaceCorner UVs.

```text
Paint channel selected
→ UV Editor shows that TextureResource
```

Coverage masks são derivados e cacheáveis por UV revision + dimensions + selection mask.

## Generators

Generator vivo pode ter UV derivada visível e diagnosticável.

Manual UV edit exige `Make Editable`.

Paint pode operar sobre Generator quando houver UV estável, conforme a arquitetura de Paint.

Não criar UV override paralelo sobre Generator na V1.

## Modifiers

Topology-preserving modifiers mantêm UV por identidade.

Topology-changing modifiers devem produzir/propagar UV determinística.

Manual edit continua atuando no authoring source, não em corners evaluated-only.

## Undo

Um Undo por:
- transform UV modal;
- Mark/Clear Seam;
- Pin/Unpin;
- Split/Stitch;
- Pack;
- Unwrap;
- Projection.

Nunca checkpoint por mouse move.

## Performance

Cachear derivados por:

```text
UV revision
+ topology revision
```

Inclui island graph, bounds, overlap, stretch e coverage masks.

## Ownership

### Geometry
Projections, islands, seams/pins, overlap/stretch, texel density, pack/unwrap adapters, coverage masks e UV hit primitives.

### Application
UvSelectionState, sync policy, transforms, commands/Undo, material/channel context e UV snap settings.

### UI
2D UV viewport, checker/texture background, toolbar, health e interaction feedback.

### Renderer
Somente overlays 3D de checker/diagnostics quando solicitados.

## Migração recomendada

1. concluir FaceCorner migration;
2. introduzir FaceCornerIndex/CornerIndex;
3. substituir uv_selected face-based por UvSelectionState;
4. mover selection sync para Application;
5. corrigir xatlas para não alterar authoring topology;
6. atualizar seams de chart boundaries;
7. remover AppState dos comandos UV;
8. corrigir pin semantics;
9. substituir/renomear Relax atual;
10. melhorar Stitch;
11. adicionar Split UV;
12. adicionar Cylindrical/Spherical;
13. melhorar Pack com pixel padding;
14. consolidar UV Health;
15. integrar TextureResource/channel;
16. portar UI para egui/Petunia UI Foundation.

## Gates

- Auto Unwrap não altera face count/topology;
- quad/n-gon continua autoral;
- seams/pins permanecem previsíveis;
- Sync off mantém seleção UV independente;
- Sync on mantém 3D↔UV coerente;
- Pack determinístico;
- padding em pixels correto;
- Paint coverage corresponde ao UV Editor;
- glTF/OBJ UV roundtrip;
- Undo/Redo completo.

## Não objetivos V1

Sem:
- UDIM;
- edição multi-UV-set;
- live unwrap contínuo;
- solver acadêmico próprio;
- UV sculpt brush;
- stacking manager complexo;
- procedural UV node graph.

## Regra anti-bloat

```text
Mark Seams
→ Unwrap
→ Pack
→ Check Density/Stretch
→ Paint
```

Ferramentas extras existem para corrigir casos específicos, não para transformar UV em outro aplicativo dentro do Petunia.


## Decisões fechadas

1. UV continua per-face-corner; não existe UvMesh persistente paralelo.
2. UV Selection suporta Corner, Edge, Face e Island.
3. Sync Selection entre 3D e UV é opção explícita.
4. UV Edge é distinta de EdgeKey topológica.
5. UvIsland é derivada e não persistente.
6. Pins são constraints de solver; direct transform explícito pode movê-los.
7. Auto Unwrap/xatlas nunca altera topology autoral.
8. Chart boundaries do Auto Unwrap atualizam seam intent de forma explícita.
9. Relax atual será substituído/atualizado; não será apresentado como stretch minimization enquanto não o fizer.
10. Stitch usa stationary/moving side; não média os dois lados por padrão.
11. Split UV entra como operação explícita sem duplicar vertex 3D.
12. Cylindrical e Spherical Projection entram como ferramentas de baixo custo.
13. Pack usa padding em pixels e resultado determinístico.
14. Texel Density vira ferramenta de primeira classe.
15. UV Health consolida overlap/stretch/zero-area/out-of-range/density.
16. UV fora de 0..1 é diagnostic, não erro.
17. UV Editor mostra TextureResource/canal de Material ativo.
18. Generator pode expor UV derivada e Paint, mas manual UV edit exige Make Editable.
19. Undo é por gesto/comando, nunca por mouse move.
