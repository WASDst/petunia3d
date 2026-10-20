# Plano de integração das branches → `main` (2026-10-19)

> Situação medida, não estimada: contagens e conflitos saíram de `git rev-list`, `git merge-tree` e `git diff --stat` nesta data. **Nenhum merge em `main` foi feito** — o `AGENTS.md` exige instrução explícita do usuário para isso (este arquivo é o preparo).

## Panorama

| Branch | Commit | Exclusivos vs `main` | Conteúdo exclusivo | Conflito com `refactor/architecture-foundation` |
|---|---|---|---|---|
| `refactor/architecture-foundation` (nosso) | `7df47dbd` | 84 | Ondas B/C: camadas, pintura, gradiente, nós, referência, modos, importação, morph targets | — |
| `codex/qt-integration` | `d4a8a060` | 1 (mas ~8,5k linhas) | **DRAW**: formas persistentes, geradores reeditáveis, GUI Qt do DRAW | **20 arquivos** |
| `ci/editor-gate` | `f9693e59` | 1 | CI: substitui o gate `ui-slint` pelo gate do `petunia_editor` | **2 arquivos** |
| `claude/practical-wiles-b19d2c` | `9b03a6d0` | 1 | `xtask`: deriva `ICON_TOKENS.md` do pacote de ícones Qt + docs regenerados | 6 arquivos (docs gerados) |
| `shell-v2/onda-a` | `30f96872` | 0 | — | **já contida** no nosso branch |
| `origin/shell-v2/onda-b` | `c7015320` | 0 | — | **já contida** (é a base do nosso branch) |
| `origin/refactor/architecture-foundation` | `1de440ec` | 0 | — | **já contida** (nós somos 21 commits à frente) |

`main` (`92c22caa`) é **ancestral** de todas: não há nada em `main` que as branches não tenham.

## Ordem recomendada

Criar uma branch de integração e trazer **uma branch por vez**, com os gates entre elas:

```bash
git fetch origin
git switch -c integration/main-2026-10-19 main
git merge --no-ff refactor/architecture-foundation   # 1. o trabalho das ondas
# … gates (abaixo) …
git merge --no-ff codex/qt-integration                  # 2. o DRAW (conflitos manuais)
# … gates (abaixo) …
git merge --no-ff ci/editor-gate                       # 3. o CI
git merge --no-ff claude/practical-wiles-b19d2c        # 4. o xtask + docs gerados
# … gates (abaixo) …
git push origin integration/main-2026-10-19
```

### 1. `refactor/architecture-foundation` (nossa) — sem conflito
É a base de tudo: 505 arquivos, ~91k linhas. **Já validada** (gates completos no `CONTEXT.md`).

### 2. `codex/qt-integration` — 20 arquivos em conflito (o trabalho pesado)
| Arquivo | Natureza do conflito | Resolução proposta |
|---|---|---|
| `crates/core/src/state.rs`, `crates/core/src/lib.rs` | o DRAW toca o estado de sessão e as exportações do core, que as ondas também mexeram | **manual**: manter os dois lados (as ondas trazem `extrude_mode`, `import_request`; o DRAW traz geradores/formas) e revalidar com os testes do core |
| `crates/editor/src/bridge/{commands,operations,primitives,tool_gestures,lib}.rs` | o DRAW acrescenta intenções e gestos; as ondas acrescentam comandos de paint/nós | **manual**, por intenção (as ondas B/C não devem ser reescritas) |
| `crates/module-model/src/draw_profile.rs`, `crates/project/src/{format,lib}.rs` | formatos/serialização do DRAW × o novo modelo de camadas | **manual**: manter o formato novo (o `.petunia` tem as camadas) e portar o que o DRAW precisa |
| `crates/ui-cxxqt/cpp/petunia_editor_panel.cpp`, `assets/native-translations.json` | textos e o painel legado | **manual**: regerar o catálogo com `crates/ui-cxxqt/tools/i18n_sync.py` depois de resolver |
| `crates/ui-cxxqt/cpp/petunia_main_window.cpp`, `petunia_overlay_cards.cpp` | **modify/delete**: apagados no nosso branch (a consolidação Qt), modificados no codex | **resolver como delete** (a `main` do shell v2 não usa mais essas janelas) — confirmar com o autor do commit `d4a8a060` |
| `crates/ui-cxxqt/src/render/opengl_viewport.rs` | o viewport do DRAW × o wireframe corrigido (normais por aresta) | **manual**: preservar o descarte de arestas de trás |
| `crates/ui-cxxqt/tests/editor_parity.rs` | testes dos dois lados | **manual**: manter os dois conjuntos de casos |
| `website/progress/tasks.json`, `website/docs/02-application/tools-interaction-implementation.md` | tracker e matriz de ferramentas | **manual**: unificar as entradas (U34–U36 e as do DRAW) |

> **Regra do repo:** o `AGENTS.md` proíbe mesclar em `main` sem instrução explícita do usuário, e proíbe rebase/force-update. O merge em `main` acontece **depois** dos gates e do seu aval.

### 3. `ci/editor-gate` — 2 arquivos
- `.github/workflows/ui-slint.yml`: **modify/delete** (apagado no branch, modificado no nosso) → resolver como **delete** (o gate Slint não existe mais; o substituto é `ui-editor.yml`).
- `website/docs/14-quality/testing-ci-conformance.md`: conflito de texto → unificar as duas descrições de gate.
- `crates/editor/src/animate.rs`: mesclagem **automática** (4 linhas) — rever na leitura.

### 4. `claude/practical-wiles-b19d2c` — docs gerados
- O conflito está em `docs/generated/*` e `docs/shortcuts/cheatsheet.md`, porque o *gerador* mudou.
- **Resolução**: aceitar o `crates/xtask/src/generator.rs` do branch e **regerar** (`cargo run -p xtask -- docs-generate`), descartando o conteúdo conflitante dos arquivos gerados.

## Gates a rodar depois de cada fusão

```bash
export RUSTUP_TOOLCHAIN=1.98.1        # o `stable` do ambiente é 1.97.1 e não compila
export CARGO_BUILD_JOBS=2              # swap cheia: não subir o paralelismo
cargo fmt --all -- --check
cargo clippy -p petunia_core -p petunia_mesh -p petunia_project -p petunia_module_paint \
  -p petunia_module_model -p petunia_editor -p petunia_config -p petunia_cli -p petunia_ui_cxxqt \
  --all-targets -- -D warnings
cargo test -p petunia_core --lib
cargo test -p petunia_editor --features animation-workspace --lib
QT_QPA_PLATFORM=offscreen cargo test -p petunia_ui_cxxqt -- --test-threads=1
cargo run -p xtask -- docs-generate --check
node website/scripts/verify-progress.cjs && python3 website/scripts/verify-agent-docs.py
```

E, no fim, o roteiro de aceite humano: [`website/docs/09-ui/shell-v2-manual-test-checklist.md`](website/docs/09-ui/shell-v2-manual-test-checklist.md).

## Riscos

- A fusão do `codex/qt-integration` toca o **núcleo** (estado, comandos, serialização): é a parte que exige olho humano, não resolution automática.
- Os *docs gerados* só podem ser regenerados **depois** da fusão do `xtask`; antes disso, qualquer `docs-generate` reescreve o formato antigo.
- `main` está atrás de tudo: o PR final será grande (≈100k linhas). Convém revisar por *wave*, não de uma vez.

## Próximo passo

1. Enviar `refactor/architecture-foundation` (`git push`).
2. Abrir a branch de integração e seguir a ordem acima, com os gates entre as fusões.
3. Só então pedir a fusão em `main` (instrução explícita).
