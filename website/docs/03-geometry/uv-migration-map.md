# Mapa de migração UV para FaceCorner

> **Status: aprovado. Documento de execução.**

A migração deve acontecer por slices pequenos, preservando comportamento. O objetivo não é apenas renomear campos; é tornar `FaceCorner` a unidade estrutural autoral de cada face.

## 1. Foundation / tipos

Adicionar:

```rust
pub struct TextureCoordinates([f32; 2]);

pub struct FaceCorner {
    pub vertex: VertexIndex,
    pub texture_coordinates: TextureCoordinates,
}

pub struct Face {
    pub corners: Vec<FaceCorner>,
    pub material_slot: Option<MaterialSlotIndex>,
}
```

Fornecer inicialmente helpers de compatibilidade para reduzir churn durante a transição:

- `Face::vertex_indices()`;
- `Face::texture_coordinates()`;
- `Face::corner(index)`;
- `Face::corner_mut(index)`;
- construtores explícitos para faces sem UV inicial e faces com UV.

Evitar manter `verts` e `uv` como duas fontes de verdade após o corte final.

## 2. Geometry core

### Triangulation
Atualizar `face_triangle_corners`, normal, centroid, fingerprint e demais queries para acessar `corner.vertex`.

### Topology / Half-Edge
`HalfEdgeTopology::from_mesh` passa a consumir diretamente cada `FaceCorner`.
A conversão de volta deve reconstruir corners completos.

### Operações que reorganizam faces
Migrar primeiro as operações estruturalmente simples:

- reverse winding;
- rotate face;
- triangulate;
- flip diagonal;
- face construction helpers.

A operação sobre winding deve virar simplesmente:

```rust
face.corners.reverse();
```

## 3. Operações topológicas críticas

Cada operação abaixo deve preservar/interpolar `FaceCorner::texture_coordinates` junto com a topologia:

- Poly Pen;
- Split Edge;
- Knife / multi-face Knife;
- Loop Cut;
- Bevel;
- Inset;
- Extrude;
- Dissolve;
- Weld / Merge by Distance;
- Imprint;
- Slice;
- Boolean result conversion;
- Boolean cleanup;
- Shape Builder;
- future Limited Dissolve.

### Gate
Para cada operação:
1. topologia válida;
2. quantidade de corners consistente;
3. UV finita;
4. seams preservadas quando aplicável;
5. material slot preservado;
6. resultado determinístico quando já existe essa garantia.

## 4. UV subsystem

Migrar:

- `uv.rs`;
- `uv_tools.rs`;
- `uv_layout.rs`;
- `uv_xatlas.rs`;
- island detection;
- unwrap;
- relax;
- pack;
- projection;
- density/health diagnostics.

Os algoritmos deixam de indexar `face.uv[corner]` e passam a consumir `face.corners[corner].texture_coordinates`.

## 5. Pins e seams

### Seams
`uv_seams` continua sendo baseada em `EdgeKey`, pois seam é propriedade de uma aresta geométrica.

### Pins
Hoje `uv_pinned` usa `(face_index, corner_index)`.

Criar newtype explícito:

```rust
pub struct FaceCornerIndex {
    pub face: FaceIndex,
    pub corner: CornerIndex,
}
```

O storage de pins passa a usar esse tipo.

## 6. Project / Document

Atualizar:

- fingerprints;
- estimated size;
- snapshots/history;
- equality/diagnostics relevantes;
- serialization helpers.

### Compatibilidade de arquivo

A camada de persistência precisa carregar o formato antigo:

```text
Face { verts, uv }
```

e convertê-lo para:

```text
Face { corners }
```

durante o load.

Não exigir que o documento em memória carregue os dois formatos.

Se o formato binário versionado exigir bump de versão, adicionar migração explícita e fixture de arquivo antigo.

## 7. Importers

### OBJ
Construir diretamente `FaceCorner` a partir do par vertex/texture-coordinate do OBJ.

### glTF
`TEXCOORD_0` continua mapeando para UV por face-corner.

### Outros importadores
Qualquer formato sem UV cria `TextureCoordinates::ZERO` ou equivalente explícito.

## 8. Exporters

OBJ/glTF/GLB devem extrair:

- position via `corner.vertex`;
- UV via `corner.texture_coordinates`.

Garantir que seams continuem gerando vertices expandidos distintos quando necessário no formato de saída.

## 9. Surface attachments

`surface_attachment.rs` usa triangulação + UV por corner para reconstrução/interpolação.

Atualizar para consumir corners diretamente e manter:
- barycentric attachment;
- reattachment validation;
- UV lookup;
- comportamento de Paint/Decal.

Esse é um gate crítico porque attachments persistentes dependem de correspondência geométrica correta.

## 10. Paint

Migrar consumidores em `module-paint` e ferramentas de superfície:

- selected face UV;
- UV island masks;
- projection;
- paint attachments;
- stencil/decal bake;
- pixel-to-surface mapping quando consumir UV.

Paint não deve conhecer detalhes internos adicionais da Face; preferir queries de Geometry quando possível.

## 11. UI / UV workspace

Migrar:
- UV viewport;
- checker;
- health diagnostics;
- island visualization;
- seleção de corners;
- paint/UV overlays.

UI deve receber DTOs/queries, não manipular `FaceCorner` diretamente quando isso puder ser evitado.

## 12. Render

Durante a transição, render extraction deve ser o único adaptador principal.

```text
Authoring Mesh
    ↓
FaceCorner
    ↓
triangulation
    ↓
RenderVertex
```

Atualizar:
- OpenGL extraction;
- software viewport enquanto existir;
- wgpu legado enquanto permanecer na árvore;
- texture-coordinate hashing/fingerprint temporário.

A arquitetura alvo continua sendo OpenGL único; não expandir trabalho no backend legado além do necessário para manter a branch compilável durante a migração.

## 13. MCP / FFI / plugins

Auditar APIs que exponham faces ou UVs.

Não vazar índices internos de corner sem necessidade. Se a API pública precisar endereçar UV corner, usar contrato explícito e versionado.

## 14. Testes obrigatórios

Adicionar/ajustar fixtures para:

- seam compartilhando o mesmo vertex com UV diferente;
- Knife atravessando seam;
- Loop Cut interpolando UV;
- Bevel com UV;
- Boolean + cleanup preservando material/UV quando herdável;
- OBJ roundtrip;
- glTF/GLB roundtrip;
- save/load de arquivo antigo;
- surface attachment;
- Paint em ilha UV;
- xatlas unwrap;
- reverse winding;
- n-gon côncavo;
- undo/redo após operação topológica.

## Ordem recomendada

```text
Types
  ↓
Geometry read-only queries
  ↓
Simple face mutations
  ↓
Critical topology operations
  ↓
UV subsystem
  ↓
Project migration
  ↓
Import/export
  ↓
Attachments/Paint
  ↓
UI/Render adapters
  ↓
Remove compatibility helpers
```

A migração termina somente quando não existir mais acesso autoral paralelo a `verts + uv`.
