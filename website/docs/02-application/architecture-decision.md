# Application — decisão arquitetural

> **Status: aprovado**

A camada Application passa a ser a responsável por coordenar o funcionamento do editor sem depender de GUI, janela, GPU ou APIs de plataforma.

## Estrutura principal

```rust
pub struct Application {
    document: DocumentSession,
    editor: EditorSession,
    commands: CommandDispatcher,
    events: ApplicationEventQueue,
}
```

## Decisões aprovadas

1. `ProjectState` evolui para `DocumentSession`.
2. `Project` evolui conceitualmente para `Document`, com migração gradual para reduzir quebra de código.
3. Dirty state terá uma única fonte de verdade baseada no histórico/revisão.
4. `EditorSession` permanece, mas será decomposto internamente em Selection, Viewport e Tools.
5. `ToolState` monolítico será substituído por Tool Settings persistentes e uma única Active Tool Session efêmera.
6. `app::Core` deixa de existir como conceito; teremos `Application` e `DesktopHost`.
7. `ModuleRegistry` será aposentado gradualmente para funcionalidades internas como Model, Paint, UV e Assets.
8. Autosave será separado entre política de aplicação e mecanismo de persistência/filesystem.

## Regra de dependência

```text
DesktopHost
├── Window/Input
├── UI
├── OpenGL
├── Filesystem/Preferences
└── Application
     ├── DocumentSession
     ├── EditorSession
     ├── Commands
     └── Events
```

A Application não conhece `winit`, egui, OpenGL nem filesystem concreto.
