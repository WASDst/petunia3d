# Application — separação de estado

> **Status: aprovado**

## DocumentSession

O atual `ProjectState` já representa em grande parte uma sessão de documento, mas mistura responsabilidades.

Destino:

```rust
pub struct DocumentSession {
    document: Document,
    history: DocumentHistory,
    file_path: Option<DocumentPath>,
}
```

Não pertencem a `DocumentSession`:

- recent projects;
- opções de exportação;
- câmera;
- UI;
- input;
- tool state;
- preferências globais.

## Dirty state

`is_dirty: bool` não deve competir com `UndoStack::is_dirty()`.

O estado modificado deve ser derivado de uma única autoridade de histórico/revisão.

## EditorSession

```rust
pub struct EditorSession {
    workspace: Workspace,
    selection: SelectionState,
    viewport: ViewportSession,
    tools: ToolSessionManager,
}
```

### SelectionState

`SelectionDomain` é a única autoridade de domínio de seleção.

`SelectMode` será removido progressivamente.

IDs e índices semânticos devem receber newtypes quando isso aumentar segurança e legibilidade.

### ViewportSession

Contém estado de sessão da viewport, como:

- Camera;
- 3D Cursor;
- shading;
- grid;
- overlays;
- X-Ray;
- snapping;
- transform orientation;
- pivot.

Camera não pertence ao Core mínimo.

### Tools

`ToolState` será decomposto em:

```text
ToolSessionManager
├── ActiveTool
├── ActiveToolSession
└── ToolSettings
```

Tool Settings persistem entre usos.

Tool Sessions existem somente durante a operação ativa.
