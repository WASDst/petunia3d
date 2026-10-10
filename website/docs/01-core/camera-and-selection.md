# Core — câmera e seleção

> **Status: decisões aprovadas**

## Câmera

A câmera deixa de ser responsabilidade do Core mínimo e passa para a camada Application/Editor Session.

Os algoritmos atuais serão preservados quando corretos.

### Presets existentes

Continuamos com vistas em perspectiva, ortográficas cardinais e isométricas verdadeiras.

### Pseudo-isométrico 2:1

Será adicionado um preset específico para a projeção pseudo-isométrica clássica de games 2:1, usando **26,565°**.

Ela deve ser tratada como preset distinto da vista isométrica verdadeira. O nome da API deve refletir explicitamente essa diferença.

Uma opção de naming a validar durante a implementação:

```rust
ViewPreset::PseudoIsometric2To1
```

O preset deverá usar projeção ortográfica e preservar o framing da câmera.

## Seleção

`SelectionDomain` será a autoridade única para o domínio de interação:

```rust
pub enum SelectionDomain {
    Object,
    Vertex,
    Edge,
    Face,
}
```

`SelectMode` é redundante em princípio e uso e será removido gradualmente.

A migração deverá identificar consumidores, substituir conversões, preservar testes e remover o enum apenas quando não houver consumidores.

IDs persistentes deverão usar newtypes quando isso aumentar segurança e legibilidade.
