# Crate `petunia_ui_slint` (`crates/ui-slint/`)

**Único frontend ativo de produção** do Petunia3D, construído com [Slint](https://slint.dev/) 1.18.
A decisão de 2026-10-08 mantém Slint em consolidação incremental e transforma egui em
fallback congelado. Enquanto a separação física do legado não for concluída,
`--legacy-egui` / `PETUNIA_LEGACY_EGUI=1` são apenas compatibilidade temporária.
Novas features de UI pertencem exclusivamente a esta crate. Ver
[ADR 009](../../docs/architecture/adr/009-slint-producao-egui-fallback-congelado.md).

- **Shell declarativo (`ui/app.slint`)**: Top bar com navegação exclusiva dos workspaces (`MODEL`, `PAINT`, `UV` e, com a feature `animation-workspace`, `ANIMATE`), alternador de domínio de seleção (Object, Point, Edge, Face), Tool Tray contextual, painel Parts, gaveta inferior para Asset Library, e painéis de Context Inspector adaptados ao workspace ativo.
- **Viewport central interativo**: composição direta da textura WGPU no Slint via WGPU 30, sem readback síncrono GPU→CPU por frame, com software viewport fallback resiliente (`viewport_soft.rs`).
- **Bridge de intents reativo (`lib.rs`)**: Comunicação desacoplada entre a UI Slint e os domínios de `petunia_core`, `petunia_commands` e `petunia_project`. O tipo `UiIntent` é a única fronteira que traduz ações do shell para operações de domínio.
- **Command Palette & dispatcher canônico**: busca, disabled reasons, foco, seleção por teclado e execução através do `CommandDispatcher` do core.
- **Persistência assíncrona (`files.rs`)**: Diálogos nativos assíncronos via RFD para carregar e salvar arquivos de projeto `.petunia`.
- **Camada de design tokens e temas (`theme.rs`)**: Sincronização em tempo de execução com o `ThemeRegistry` do Petunia (`petunia-dark` e `petunia-high-contrast`).
- **Gerenciador de Overlays (`overlay.rs`)**: Pilha LIFO identificada por superfície, pin de Parts, Escape/click-away determinísticos e restauração contextual.

O fluxo Cut/Knife ainda depende de conectar coordenadas de viewport à sessão de
corte existente. Essa pendência permanece explícita em vez de apresentar um
botão visual como operação concluída.

## Validação

```bash
cargo test -p petunia_ui_slint --lib          # testes unitários e do bridge (inclui o Animate)
cargo test -p petunia_ui_slint --features animation-workspace --lib --test animate_shell   # workspace Animate no shell real
cargo clippy -p petunia_ui_slint --all-targets -- -D warnings
cargo fmt -p petunia_ui_slint -- --check
```
