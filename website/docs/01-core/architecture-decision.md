# Core — decisão arquitetural

> **Status: aprovado**

O Petunia3D adotará um **Core mínimo**.

## Responsabilidade

O Core fornece tijolos neutros e fundamentais. Ele não representa a aplicação inteira.

```text
petunia-core
├── identifiers
├── handles
├── shared contracts
├── neutral primitive types
└── math primitives
```

A camada **Application** passa a coordenar o funcionamento do editor:

```text
petunia-application
├── DocumentSession
├── EditorSession
├── Camera
├── Selection
├── Commands
├── CommandDispatcher
├── History
├── Tool Sessions
├── Queries
└── Events
```

> Core fornece os tijolos. Application determina como o editor funciona. Project representa o documento. Geometry transforma geometria. UI apresenta. Render desenha.

## Consequências

- `UiState` sai do Core.
- Camera pertence à sessão de edição/Application.
- Commands deixam de receber acesso irrestrito ao universo inteiro quando contextos menores forem possíveis.
- ToolState monolítico será decomposto.
- I/O não acontece implicitamente na construção de estado de domínio.
