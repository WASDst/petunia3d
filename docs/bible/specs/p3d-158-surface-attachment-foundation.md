# P3D-158 — Surface Attachment Foundation

<aside>
📌

Estado: **fundação aprovada; uso por Path Paint antecipado para o MVP em 01/10/2026**. Uma única representação de attachment deve ser reutilizada por Decals, Hair roots, Path Paint, Surface Conform, accessories e detalhes aderidos.

</aside>

# Objetivo

Representar uma posição aderida a uma superfície de forma estável mesmo quando o objeto é transformado e, dentro de limites definidos, quando sua geometria derivada muda.

# Modelo de dados

```rust
pub struct SurfaceAttachment {
    pub target: ObjectId,
    pub triangle: TriangleHandle,
    pub barycentric: Vec3,
    pub normal_offset: f32,
    pub tangent_rotation: f32,
}
```

A representação pode manter dados auxiliares de UV/tangent quando necessário, mas triangle + barycentric permanece a âncora geométrica principal para o mesmo revision topology.

# Consumidores

- DecalInstance;
- HairRoot / guide start;
- Surface Path control point;
- Surface Conform references;
- optional accessory/socket placement;
- projected labels/patches.

# Reattachment

Mudança de transform preserva attachment. Mudança de topology pode invalidar handles; o sistema deve marcar `Needs Reattach` e oferecer reprojection explícita, nunca escolher silenciosamente uma região distante.

# API

Queries: project ray to surface, evaluate world frame, evaluate tangent frame, slide attachment, rotate around normal, reproject, validate.

# Undo / persistência

Attachment é authoring data versionado no `.petunia`; operações de slide/reproject são Commands transacionais.

# Testes / DoD

Raycast fixtures, barycentric round-trip, transform parent/child, save/load, invalidation por topology revision, reprojection determinística e attachments em UV seams.