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
 
O estado modificado é derivado de uma única autoridade de histórico/revisão (`UndoStack::is_dirty()`), sincronizada com cada salvamento e carregamento. Comandos não-destrutivos (como seleção) não alteram o dirty state nem criam checkpoints; comandos destrutivos avançam a pilha e marcam dirty atomicamente. Desfazer até a versão salva restaura o dirty state para `false`.
 
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
 
### Tools e ciclo de vida de ToolSession
 
`ToolState` decompõe explicitamente configurações persistentes e sessões ativas:
 
```rust
pub struct ToolSettingsSnapshot {
    pub draw_mode: DrawMode,
    pub default_radius: f32,
    pub primitive_kind: PrimitiveKind,
    pub knife_cut_through: bool,
    pub extrude_depth: f32,
    pub bevel_offset: f32,
    pub inset_thickness: f32,
}

pub enum ActiveToolSessionKind {
    None,
    Modal,
    Primitive,
    Cut,
    RegionPush,
    UvTransform,
    PaintStroke,
}
```
 
- **Settings persistentes:** `ToolSettingsSnapshot` retém opções de ferramentas entre diferentes invocações e trocas de workspace sem interferir nem ser corrompido por operações efêmeras em andamento.
- **Sessões ativas efêmeras:** `ActiveToolSessionKind` identifica formalmente operações interativas ativas (Modal, Primitive, Cut, RegionPush, UvTransform, PaintStroke).
- **Cancelamento limpo:** `cancel_active_session()` em `ToolState` e `AppState` encerra qualquer rascunho ou transformação interativa sem mutação no documento e com zero poluição na pilha de Undo (0 entradas).
- **Preview/Commit/Cancel:** manipulações interativas operam sobre prévias isoladas do documento canônico; o commit atômico gera exatamente uma única entrada de Undo.

