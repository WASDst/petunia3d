# UV — Face Corners

> **Status: aprovado para implementação durante a refatoração de Geometry.**

A representação atual usa arrays paralelos:

```rust
pub struct Face {
    pub verts: Vec<VertexIndex>,
    pub uv: Vec<TextureCoordinates>,
}
```

Embora correta para seams, ela exige a invariante manual:

```text
verts.len() == uv.len()
```

Essa invariante aparece repetidamente em Knife, Loop Cut, Bevel, Imprint, Boolean cleanup, UV tools, import/export, attachments e render extraction.

## Modelo alvo

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

## Benefícios

- vertex e UV de corner tornam-se atomicamente ligados;
- inserir, remover, inverter e reorganizar uma face opera uma única coleção;
- elimina uma classe inteira de bugs de arrays fora de sincronia;
- simplifica operações topológicas que interpolam UV;
- torna seams, attachments e triangulação mais explícitos;
- melhora significativamente a auditabilidade por humanos e agentes.

## Regras

- UV continua sendo **por face-corner**, nunca por vertex global.
- A migração não altera a capacidade de representar seams.
- Não converteremos todas as APIs externas de uma vez; adaptadores temporários são aceitáveis durante a migração.
- Compatibilidade de arquivos existentes é obrigatória.
- Nenhuma operação geométrica será considerada migrada até seus testes de UV e topologia continuarem verdes.

O roteiro transversal está em [Mapa de migração UV](uv-migration-map.md).
