# Qt — integração sobre a base funcional única

> Status: consolidação do código/build/histórico concluída no escopo Q05, autorizada pelo usuário em 2026-10-13. Integração funcional e aceitação Qt continuam abertas; não certifica paridade nem substituição de produção.

## Intent

Consolidar o frontend Qt com as features atuais sem copiar o motor antigo nem manter dois históricos de produto. O repositório em `/run/media/raillen/3854B63454B5F52A/petunia3d-refact` é a base funcional. O checkout Qt em `/run/media/raillen/3854B63454B5F52A/petunia3d-qt-integration` é um worktree desse repositório, branch `codex/qt-integration`.

## Sources

[Registro de decisões](../00-philosophy/decision-register.md#adr-c10--base-funcional-unica-e-integracao-qt-2026-10-13), [protocolo](../16-code-agents/implementation-protocol.md), [tracker](../17-reimplementation/index.md). Código: `crates/ui-cxxqt`, Core/Application/Project/Render compartilhados e workspace Cargo.

## Gap

| Responsabilidade | Estado inicial | Delta desta integração |
|---|---|---|
| Histórico e features | dois repositórios relacionados, ambos com alterações locais | snapshot recuperável + branch Qt no repositório funcional |
| Domínio compartilhado | pasta Qt baseada em features antigas | usar exatamente o snapshot funcional atual |
| Frontend Qt | crate e C++/QML em outro checkout | portar o crate, seus arquivos novos e dependências Cargo |
| Paridade/aceitação | adapter parcial e alegações históricas de 100% | manter IN PROGRESS; nenhum teste histórico é prova deste conjunto |

## Changed

Snapshots locais (índice temporário, sem mudar índice/HEAD dos checkouts originais):

- features: `a8d60cd72ab28ef7d430279b1ed3100ee5d98eae`, pai `641a7d1ec6dfe3efc486a919d7936f56030d53f2`; branch de preservação `codex/snapshot-features-20261013` no repositório funcional;
- Qt: `be18831dddc997e652a8888b0123ec221246d196`, pai `304e3c9c266c01923f73182bcd643b844496a34e`; branch de preservação `codex/snapshot-qt-20261013` no repositório Qt antigo.

Esses commits preservam trabalho preexistente não verificado; não atribuem sua autoria nem seus gates a esta integração. Backup externo com patches, hashes e arquivos modificados em `/home/raillen/.codex/visualizations/2026/10/13/01a13594-f708-7151-b213-9488ef2bc02d/integration-backup`.

Transferência selecionada: `crates/ui-cxxqt/**`, registro no workspace e lockfile com dependências Qt. Nenhum Core/Project/Mesh/Slint da pasta antiga foi sobreposto na base funcional. O lockfile de origem preserva as versões já presentes na base e adiciona a árvore Qt; gates locked confirmarão resolução.

## Verification

- Snapshot consistente antes/depois da captura: pass.
- `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/petunia_cxxqt_target cargo check --locked --offline -p petunia_ui_cxxqt --all-targets`: pass, Linux, Rust 1.98.1, Qt 6.12.0, 1m20s.
- `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/petunia_cxxqt_target QT_QPA_PLATFORM=offscreen cargo test --locked --offline -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1 --skip test_widgets_studio_window_creation`: pass (10 testes, zero falhas, 1 filtrado; build 1m57s). O teste filtrado de janela permanece not run.
- Package/features/dependencies do executável raiz e `src/main.rs` comparados ao snapshot: pass, inalterados. Slint/CLI/MCP mantidos como entradas existentes.
- `node website/scripts/verify-progress.cjs`: pass (67 processos, 68 documentos).
- `node --test website/tests/progress.test.cjs`: pass (6 testes).
- `python3 website/scripts/verify-agent-docs.py`: pass (68 rotas).
- `git diff --check` no worktree Qt: pass. No checkout funcional: aviso em `crates/ui-slint/ui/animate.slint:1459` (blank line at EOF), pertencente ao trabalho funcional preservado; não corrigido nesta consolidação.
- 1329 arquivos funcionais comparados byte a byte com snapshot: pass, zero diferenças; 50 paths Qt preservados.
- GUI nativa, stylus, GL, HiDPI, Windows, leitor de tela, low-end e paridade: not run. Clippy Qt e bateria Rust transversal: not run neste transporte.

## Risks

O módulo importado inclui melhorias locais de PAINT/UV/Animate/assets/modifiers ainda sem aceitação neste conjunto. Se as APIs novas divergirem, adaptar somente o adapter Qt. Não enfraquecer testes nem sobrepor a implementação funcional para fazer o Qt compilar. A pasta Qt antiga fica como origem preservada; não deve receber novas features do domínio.

## Next checkpoint — fluxo contínuo

1. Trabalhar features compartilhadas na linha funcional; fechar mudanças em commits identificáveis antes de atualizar o worktree Qt.
2. No worktree Qt, verificar `git status`; preservar alterações locais antes de sincronizar. Com worktree limpo, integrar os novos commits de `refactor/architecture-foundation` com `git merge refactor/architecture-foundation`. Não fazer cópias manuais de Core/Project/Mesh.
3. Adaptar Qt a Commands/Queries/sessões atuais e rodar gates focados serialmente. Testar efeitos reais, Undo/Redo e save/load, não apenas setters/getters.
4. Integrar de volta somente o conjunto Qt revisado. O pedido desta rodada autoriza a consolidação local, não publicação/merge em main nem descontinuação Slint.
5. O snapshot funcional desta rodada inclui trabalho ainda não commitado na linha funcional. Quando esse trabalho for commitado, integrar o commit real na branch Qt; o snapshot anterior continua recuperável e a igualdade do conteúdo deve ser revista.

## Como executar

Qt Widgets experimental:

```bash
cargo run -p petunia_ui_cxxqt --example cxxqt_widgets_studio
```

Qt Quick preview separado:

```bash
cargo run -p petunia_ui_cxxqt --example cxxqt_preview
```

Gate focado (Qt 6 e toolchain C++ necessários):

```bash
CARGO_BUILD_JOBS=1 cargo check --locked -p petunia_ui_cxxqt --all-targets
QT_QPA_PLATFORM=offscreen cargo test --locked -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1
```

Os testes Qt usam estado global e devem ser executados serialmente. Offscreen não valida o QOpenGLWidget em janela nativa nem hardware de tablet. Respeitar qualquer Cargo já ativo; um único target directory para os gates desta rodada.

Correção incidental de formato: N01 trazia três checkpoints com texto livre no campo evidence, rejeitado pelo validador. Os textos foram preservados integralmente em entradas evidence referenciadas por ID; status/checkpoints não foram promovidos e os relatos históricos não foram reexecutados nesta integração.

## Entrega local e continuidade

O módulo Qt e integração de Cargo/docs/tracker foram instalados também no checkout funcional, sem trocar seu HEAD ou índice nem commitá-lo junto com trabalho de outros agentes. A branch `codex/qt-integration` tem commit próprio sobre o snapshot funcional. Os dois checkouts pertencem ao mesmo repositório Git; a pasta antiga foi preservada com `QT_WORKTREE_MOVED.md`.

Os gates Rust acima valem para a base funcional capturada no snapshot, não para alterações posteriores feitas por outro agente no checkout funcional. Quando a linha funcional receber seu commit real, sincronizar esse commit e repetir os gates afetados. O backup contém logs `qt-check.log` e `qt-tests.log`, arquivos/diffs, snapshot JSON e arquivo standalone do módulo Qt.
