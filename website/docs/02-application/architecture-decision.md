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

## Integração incremental com Slint — 2026-10-11

O caminho normal do `DesktopHost` constrói e configura uma `Application`, depois a transfere para `SlintUiBridge::from_application`. O adapter `ApplicationState` projeta o mesmo `AppState` para os consumidores existentes de `bridge.state`; não cria outro documento, histórico ou relógio. A entrada de compatibilidade `SlintUiBridge::new` adota a sessão recebida por `Application::from_state`.

Comandos, Undo/Redo e a política de autosave passam pela Application. O host resolve o catálogo de ferramentas configurado em disco e fornece o mecanismo concreto de autosave; o catálogo nativo construído pela Application é puro. Ferramentas e projeções ainda usam acesso mutável transitório ao AppState. Esse adapter não conclui a decomposição de Document/EditorSession, a aposentadoria de ModuleRegistry nem a migração de todos os intents.

O estado de implementação, os comandos de verificação e os gates pendentes estão registrados no [Slint Rescue §28](../09-ui/slint-rescue-plan.md#28-retomada-codex-guideline-visual-e-application-2026-10-11), em A02/A04/U04. A arquitetura aprovada acima permanece o alvo; a presença do adapter não demonstra aceite comportamental ou de plataforma.
