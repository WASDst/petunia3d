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


## Q06 — implementação de paridade Qt Widgets

Escopo autorizado em 2026-10-13: implementar todos os fluxos nativos no worktree
`petunia3d-qt-integration`, branch `codex/qt-integration`. Por instrução explícita do
usuário, os testes/gates começam **somente depois** da implementação completa,
sem testes intermediários por workspace. Qt Quick continua um preview separado.
### Owner e despacho

`QtEditor` adapta o controlador funcional `petunia_editor::EditorController<QtViewportCapture>` sem
abrir uma janela Slint. Há uma única Application, Document, seleção e pilha Undo.
Qt Widgets lê o DTO via `editor_snapshot` e envia ações que são convertidas em
UiIntent/AnimateIntent ou métodos tipados existentes; não mantém cópias autorais
locais de camadas, modificadores ou keyframes. Core/Geometry permanecem sem Qt
ou Slint. A dívida transitória anterior (controlador residir em `ui-slint`) foi quitada
em 2026-10-15 com a extração de `crates/editor` (`petunia_editor`) e a eliminação
definitiva de `crates/ui-slint` e de todo o ecossistema Slint.

Referências e cores de seleção por objeto passaram a Commands compartilhados,
com histórico, locks e validação de revisão para referências. Idioma persistido
é opcional: preferências antigas não sobrescrevem o idioma explicitamente
escolhido pelo host. Abrir valida o arquivo completo antes de substituir a
sessão e limpa seleção/history/previews que apontavam para o documento anterior.
O marcador de sessão Qt usa criação exclusiva e remoção apenas pelo seu owner.

O editor nativo fica em armazenamento local à thread, sem `unsafe impl Send`.
QApplication fixa a thread da GUI; chamadas FFI de workers são recusadas e não
criam outro documento da interface. Antes dessa fixação, clientes de teste
headless possuem controladores independentes por thread. Recursos GL permanecem
por superfície/contexto. Ponteiros da ABI C têm contratos `unsafe` explícitos;
os wrappers Widgets verificam a criação na thread de QApplication.

Os catálogos embutidos `en` e `pt-BR` aparecem mesmo sem arquivos de locale no
diretório de execução; `en-US` mantém compatibilidade com hosts existentes. As
docas PAINT/UV/ANIMATE dividem canvas e controles horizontalmente, preservando o
viewport. A barra usa SVGs canônicos outline/filled, cor do tema e nomes completos
para ferramentas sem ícone. O build Widgets requer também o módulo Qt 6 Svg.

Referências são autoradas em `Project.references`: a lista de compatibilidade
é normalizada antes do snapshot transacional e sincronizada após comandos e
Undo/Redo. Salvar usa `ProjectService`, incluindo a paleta; abrir restaura a
paleta e invalida caches no novo documento. A primeira chave de um rig vazio
preserva o playhead escolhido, em vez de ser gravada implicitamente no frame 0.

O timer nativo consulta avanço de animação, decalque, airbrush e autosave. Sem
mudanças, a resposta não reconstrói o DTO e a janela não solicita redesenho.
Pressão, tilt e borracha do tablet são temporários e não reescrevem o preset,
inclusive durante acúmulo contínuo de tinta. Isso não substitui medição em
hardware de baixo desempenho ou teste com stylus físico.

### Cobertura implementada, sujeita aos gates finais

| Fluxo | Superfície Qt e caminho funcional |
|---|---|
| Shell, docas e workspaces | QMainWindow, docas reais, páginas destacáveis, contexto responsivo, DTO → UiIntent |
| Seleção, outliner e biblioteca | UUIDs estáveis, multiseleção, rename, locks/visibilidade, box/laço, prefabs/favoritos/thumbnails/placement |
| MODEL, DRAW e POLY | Pointer ToolSession, previews, confirmação/cancelamento, primitivos, Loop Cut, perfis/curvas/workplanes/volume, extensão DRAW |
| Inspector e materiais | TRS por parser/Command, pivô/origem/snap/proporcional, slots/texturas/PBR, modificadores e ações rápidas |
| PAINT | Pixels reais do canvas, brushes/presets/masks, camadas/effects, superfície SVG/stencil, decal/variants/playback, PiP e tablet input |
| UV | Projeção/seams/pins/pack/relax/stitch, seleção, gestos com preview e commit único/revision check, campos numéricos e textura |
| ANIMATE | Creature/rig/motion/style/fit/params, avaliação/posing, ossos/IK, transporte, keyframes, camadas, bake, dope sheet/curvas/onion/trails |
| Projeto e referências | Save/open/import/export reais, recent/home/recovery, slots RGBA, parâmetros/locks/alinhamento/Undo |
| Preferências e acesso | Idiomas/temas/escala, reduced motion, keymap profiles/captura/conflitos, F6 e restauração de foco, nomes nativos |
| Viewport/lifecycle | Contexto GL e recursos por superfície/thread, split com câmera independente, PiP com câmera primária; CPU fallback reutilizado |

O fallback CPU reutiliza Software3dViewport e limita o raster a 1280×720,
preservando coordenadas lógicas. Não introduz outro algoritmo de geometria ou
renderer. Seus limites de shading/referências são os do fallback existente.
Nenhum benchmark de performance ou aceite em hardware é inferido dessa decisão.

### Verificação final

Executável nativo: `cargo run -p petunia_ui_cxxqt --bin petunia3d-qt`.
`editor_parity` verifica comportamento do documento/histórico/pintura/arquivos e
revisões. Desde U19 (2026-10-17) a janela real é o Shell v2: o harness offscreen
`shell_v2` exercita regiões, foco F6, temas, densidade, largura estreita, layout,
HUD, Operation Graph e captura screenshots; `qt_parity_acceptance` (shell legado)
foi removido junto com `PetuniaMainWindow`. Usar XDG_CONFIG_HOME/XDG_DATA_HOME isolados e
registrar plataforma, backend, comandos, resultados e limitações. Offscreen/CPU
não certifica OpenGL físico, multimonitor, leitor de tela, stylus real ou Windows.
Q06 só pode ser DONE quando todos os gates aplicáveis tiverem evidência.

Evidência de execução em 2026-10-14 (`qt-parity-qa-20261013`):
- `cargo check --locked -p petunia_ui_cxxqt --all-targets`: pass.
- `cargo test --locked -p petunia_ui_cxxqt --test editor_parity`: pass (17 pass, 0 fail, 0.45s).
- `QT_QPA_PLATFORM=offscreen cargo test --locked -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1 --skip test_widgets_studio_window_creation`: pass (10 pass, 0 fail).
- `QT_QPA_PLATFORM=offscreen cargo run -p petunia_ui_cxxqt --example qt_parity_acceptance /tmp/petunia_qt_acceptance`: pass (50 checks executados e aprovados, report `pass: true`, 9 capturas PNG salvas).
- Barras flutuantes adaptativas implementadas: PetuniaToolOptionsBar (top view bar) e PetuniaFloatingContextBar (bottom contextual bar com model/draw/paint/uv/animate).
- Seletor de cores nativo implementado: PetuniaColorWheel e PetuniaColorPickerPopover (anel cônico de matiz, gradiente de saturação/valor, entradas numéricas H/S/V, campo HEX e paleta de amostras).
- Limites mantidos: janela nativa, GPU física, tablet físico e Windows permanecem not run.
- Acrescentado em 2026-10-14: `cargo fmt -p petunia_ui_cxxqt -- --check` pass; `bridge_test` 11/11 pass; `editor_parity` 17/17 pass; `qt_parity_acceptance` mantém `pass: true` com 50 checks; binário `petunia3d-qt` inicia e permanece estável ≥20 s com `QT_QPA_PLATFORM=offscreen`.
- Ajustes de UI pós-auditoria: cabeçalho consolidado (workspaces, undo/redo, busca e preferências) sem seletor de domínio duplicado; tool rail em modo ícone-only (44px); outliner sem cabeçalho com colunas fixas e ícones de visibilidade/bloqueio; barras flutuantes reposicionadas com clamping anti-sobreposição; validação de stylus/GPU/Windows permanece `not run`.

## Q07 — abandono do Slint, extração do petunia_editor e consolidação Qt (2026-10-15)

Escopo autorizado pelo usuário em 2026-10-15: abandono e remoção total da GUI em Slint e de suas dependências para desonerar o projeto.

### Arquitetura consolidada

1. **`crates/editor` (`petunia_editor`):** Controlador de domínio neutro e headless, contendo a máquina de estados do editor, sessões de ferramentas, projeções de viewport, sincronização de outliner, materiais, decalques, presets e animação. Possui zero dependências de Slint, Qt, FFI ou GPU.
2. **Remoção de `crates/ui-slint`:** O crate e todas as dependências Slint (`slint`, `slint-build`, `i-slint-*`) foram removidos do workspace e do repositório, com `Cargo.lock` e `Cargo.toml` purgados.
3. **`crates/ui-cxxqt` como frontend primário:** `crates/ui-cxxqt` agora depende exclusivamente de `petunia_editor` para todas as funções de controle e snapshot. `src/main.rs` despacha para `petunia_ui_cxxqt::widgets::run()`.
4. **Desacoplamento garantido (L6):** Os widgets C++/Qt permanecem estritamente na camada de apresentação; toda lógica e mutação de documento trafegam via `petunia_editor` e comandos da camada Application.

### Verificação

- `CARGO_BUILD_JOBS=2 cargo test -p petunia_editor --lib --features animation-workspace`: pass (116 testes executados e aprovados, zero falhas).
- `CARGO_BUILD_JOBS=2 cargo check -p petunia_ui_cxxqt --all-targets`: pass (zero erros).
- `QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt --test editor_parity -- --test-threads=1`: pass (21 testes executados e aprovados, zero falhas).
- `QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1 --skip test_widgets_studio_window_creation`: pass (10 testes executados e aprovados, zero falhas).
- `CARGO_BUILD_JOBS=2 cargo check --workspace`: pass (todos os crates do workspace compilando de forma limpa).
- `CARGO_BUILD_JOBS=2 cargo test -p petunia_core -p petunia_mesh -p petunia_project -p petunia_application -p petunia_render`: pass (todos os testes de domínio, geometria, renderização e aplicação passando).
- Dependências Slint no `Cargo.lock`: zero referências a crates Slint.
