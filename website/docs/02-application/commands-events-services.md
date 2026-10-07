# Application — Commands, eventos e serviços

> **Status: aprovado**

## Commands

O sistema atual de `Command` e `CommandDispatcher` é uma base valiosa e será preservado.

O principal ajuste é eliminar acesso irrestrito a `AppState`.

Destino inicial:

```rust
pub struct CommandContext<'application> {
    pub document: &'application mut DocumentSession,
    pub editor: &'application mut EditorSession,
}
```

Não criaremos múltiplas hierarquias de command antecipadamente. Contextos menores só serão introduzidos quando houver benefício real.

`ProjectChanges` deve ser reutilizado sempre que possível para comunicar invalidações e mudanças do documento.

## Eventos

A fila simples atual será preservada.

Eventos representam fatos que já aconteceram.

Exemplos:

```text
DocumentOpened
DocumentSaved
SelectionChanged
GeometryChanged
ActiveToolChanged
```

Pedidos de ação como Open, Save, Import ou Export não são eventos; são intents/commands.

## ModuleRegistry

`ModuleRegistry` não será mantido como fundamento para funcionalidades internas.

Model, Paint, UV e Assets são partes nativas do Petunia3D, não plugins runtime.

A remoção será gradual para preservar comportamento existente.

## DesktopHost

Estado de plataforma sai da Application:

- modifier keys;
- mouse buttons;
- last pointer position;
- window;
- winit events;
- OpenGL context;
- egui integration.

O DesktopHost traduz eventos de plataforma para contratos neutros da Application.

## Autosave

A implementação atual possui comportamento útil e será preservada por partes.

Separação:

```text
Application
└── AutosavePolicy
    ├── interval
    ├── retention
    └── only_when_modified

Infrastructure
├── AutosaveStorage
├── SessionLock
├── RecoveryStorage
└── AtomicDocumentWriter
```

A política decide **quando** salvar.

A infraestrutura decide **como e onde** persistir.

## Recent Projects

Recent Projects não pertence ao documento. Será uma única fonte de verdade em preferências/serviços da aplicação ou desktop.

## Side effects

Construtores da Application não devem:

- ler environment variables;
- consultar relógio do sistema;
- criar session locks;
- acessar filesystem;
- iniciar file watchers.

Essas operações pertencem ao bootstrap/host e são injetadas explicitamente.
