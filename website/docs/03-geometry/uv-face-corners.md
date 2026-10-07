# UV — Face Corners

> **Status: direção aprovada; migração será planejada antes da implementação.**

A representação atual guarda arrays paralelos de vertices e UVs por face:

```rust
Face {
    verts: Vec<VertexIndex>,
    uv: Vec<TextureCoordinates>,
}
```

Isso representa seams corretamente, mas exige preservar manualmente a invariante de comprimento e sincronizar dois vetores em praticamente toda operação topológica.

## Direção

Migrar para um modelo explícito de face-corner quando a migração puder ser feita preservando comportamento e compatibilidade de arquivos:

```rust
pub struct FaceCorner {
    pub vertex: VertexIndex,
    pub texture_coordinates: TextureCoordinates,
}

pub struct Face {
    pub corners: Vec<FaceCorner>,
    pub material_slot: Option<MaterialSlotIndex>,
}
```

## Benefícios esperados

- UV e vertex index deixam de poder perder sincronização.
- Inserir/remover/splitar corners vira uma única operação estrutural.
- Knife, Loop Cut, Poly Pen, Dissolve, Bevel e Boolean cleanup ficam mais difíceis de implementar incorretamente.
- Seams e edição UV ficam semanticamente mais claras.
- A API fica mais legível para humanos e agentes.

## Custo

A migração é de custo médio-alto porque toca operações geométricas, import/export, serialização, UV e testes. Entretanto é uma mudança localizada dentro do domínio Geometry e faz mais sentido durante esta refatoração do que depois que novas ferramentas forem construídas sobre o formato atual.

A migração deve ser incremental e acompanhada de compatibilidade do formato de projeto ou conversão explícita na camada de persistência.
