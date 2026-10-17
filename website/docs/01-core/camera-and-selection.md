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

### Ownership e isolamento transacional

- A câmera reside exclusivamente em `EditorSession` e é projetada em `ViewportSession`.
- `DocumentSession` e `ProjectState` não contêm campos de câmera; mutações de navegação (orbit, pan, zoom, frame) não alteram o hash de revisão do documento e não geram entradas na pilha de Undo.
- Alinhamentos canônicos e isométricos são despachados como comandos transacionais via `SetViewPresetCmd`, emitindo requisição de redesenho sem marcar o documento como modificado.
- Operações numéricas rejeitam floats não-finitos (`NaN`/`Inf`), preservando a altura visível anterior e garantindo estabilidade matemática contínua das matrizes de projeção.
- Gates de câmera e regressões automatizadas validados em `crates/core/tests/camera_foundations_a07_tests.rs`.


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

## Navegação e seleção explícitas — checkpoint contextual 2026-10-15

O [contrato contextual](../02-application/contextual-editor-contract.md) separa navegação, centro de transformação e origem persistente. `NavigationAnchor::ViewTarget` é o default; `Cursor3D` é escolha explícita da sessão. Orbitar no modo Cursor3D transforma olho e alvo em torno da âncora sem reenquadrar antes do gesto; zoom preserva a posição projetada da referência. Pan/frame não movem cursor; trocar modo não altera câmera imediatamente. Ferramenta ativa e `PivotPoint` não decidem navegação.

`select_through` é política de picking da sessão, independente de `show_xray` e Wireframe. Clique, hover, regiões e candidatos de snap usam essa política; a renderização mantém suas opções visuais próprias. `ViewportSession` projeta e restaura ambas as escolhas. A exposição Qt e seus gates continuam em H33; APIs headless não comprovam acesso físico no frontend.
