# Core — triagem do código atual

> **Status: diagnóstico aprovado para orientar migração.**

## Reutilizar

### Handles generacionais

`handles.rs` possui uma boa separação entre UUID persistente e handle geracional de sessão. A implementação deve ser preservada, com revisão de nomenclatura/localização quando necessário.

### Matemática neutra de viewport

`LogicalRect`, transformações screen/NDC, ray e conversões de viewport já estão desacopladas de egui e são recuperáveis.

## Reutilizar com refatoração

### Camera

A matemática atual de perspectiva, ortográfica, presets, orbit, pan, zoom, ray e preservação de framing é útil. A implementação será preservada inicialmente, mas nomes abreviados e documentação insuficiente serão corrigidos.

### Selection

O conceito de seleção será preservado, mas `SelectionDomain` será a autoridade única. `SelectMode` será removido gradualmente.

### EventBus

A fila simples é deliberadamente preferível a frameworks reativos pesados. O contrato será refinado para distinguir intenção de comando de fato ocorrido.

### Queries

A ideia de DTOs neutros para UI, CLI, MCP, plugins e FFI é boa. Queries serão separadas de mutações e deixarão de depender de um God State.

## Refatorar fortemente ou substituir a estrutura

- `ToolState` monolítico.
- `UiState` dentro do Core.
- `command.rs` monolítico.
- comandos que recebem `&mut AppState` irrestrito.
- módulos com acesso irrestrito ao estado inteiro.

O objetivo é preservar algoritmos e comportamento comprovado, não perpetuar contêineres arquiteturalmente ruins.
