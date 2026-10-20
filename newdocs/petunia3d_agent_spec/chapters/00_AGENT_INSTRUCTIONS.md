# 00 — Instruções normativas para o code agent

> **Leia antes de modificar código.** Esta é uma especificação de implementação progressiva, não um pedido para implementar tudo imediatamente. Contexto-base: `WASDst/petunia3d`, `main` SHA `92c22caa23f96f85f18e72a5e962a1d8831694db` na auditoria em 15/10/2026. Revalide o SHA no começo do trabalho.

## 0.1 Objetivo verificável

Entregar incrementalmente um editor 3D **shape-first**, profissional e acessível, no qual manipular a geometria e compreender sua profundidade/volume seja intuitivo. Usuários iniciantes devem conseguir criar humanoides, animais e criaturas por volumes e evoluir até ferramentas avançadas. A base de rig deve permitir **automático assistido**, **live incremental opt-in** e **inteiramente manual**.

## 0.2 Ordem de trabalho e autoridade

1. Inspecionar `git status --short --branch`, `git rev-parse HEAD`, branch upstream, mudanças não commitadas, worktrees e CI. Não descartar trabalho de outros agentes.
2. Ler o `AGENTS.md` **atual** e `website/docs/16-code-agents/{index,reading-navigation,implementation-protocol}.md`, `website/docs/manifest.json` e `website/docs/00-philosophy/decision-register.md`.
3. Confrontar os documentos com `Cargo.toml`, `src/main.rs`, `crates/ui-cxxqt/`, `crates/editor/`, `crates/render/` e commit de migração Qt. A documentação histórica Slint não prevalece sobre a migração explícita para Qt.
4. Identificar implementação real por símbolo, caminho, callers, testes, entrada FFI, persistência, eventos e invariantes. Criar **gap matrix**: existente/verificado, existente/não verificado, parcial, ausente, bloqueado.
5. Redigir plano de menor mudança por *vertical slice*; **Reuse → Refactor → Move → Rewrite (último recurso)**.
6. Implementar a menor fatia funcional com testes e atualização de docs/tracker. Parar antes de expandir escopo ou refazer arquitetura sem autorização.
7. Relatar SHA, paths, comandos executados, resultados `pass/fail/blocked/not run`, evidências de UI e limitações; não alegar execução que não ocorreu.

**Permissões:** não realizar push/merge/rebase/force push/deploy sem autorização explícita. Não confundir entregar docs com autorização para alterar a `main`. Não remover APIs legadas que ainda possam ter consumidores antes de inventariar chamadas.

## 0.3 Stack e responsabilidades reais a preservar

| Camada | Base observada | Orientação |
|---|---|---|
| Interface principal | Qt 6 Widgets/C++ em `crates/ui-cxxqt/cpp/` | Widgets e acessibilidade nativa, docking, inputs, barras; sem kernels geométricos |
| Bridge | `crates/ui-cxxqt/src/bridge/`, `src/editor.rs`, `cpp/petunia_editor_api.h` | Converter intents/DTOs tipados; validar fronteiras FFI, threads e ownership |
| Editor toolkit-neutro | `crates/editor/` | Tool sessions, overlays semânticos, preferências, input e view models |
| Application/Core | `crates/application/`, `crates/core/` | Autoridade de documento, regras, commands, Undo/Redo, revisão |
| Geometria | `crates/mesh/`, `crates/project/` | Seções, medição, malha, shape, rig, pesos, IK e serialização |
| Renderer | `crates/render/`, `ui-cxxqt/src/render/` | OpenGL 3.3/glow; scene extraction e QOpenGLWidget; fallback existente |
| UI alternativa | arquivos QML/Qt Quick presentes | Não assumir que são a shell principal |

**Não transportar as antigas propostas Qt/QML ou WGPU como requisitos da stack atual.** A arquitetura de capacidades deve ser independente da API gráfica, mas implementações iniciais devem usar OpenGL e Qt Widgets.

## 0.4 Invariantes não negociáveis

- Documento com um só escritor. Ferramentas/GUI/CLI/MCP enviam intents e comandos; nenhuma mutação direta em widget/slot ou renderizador.
- Mutação relevante deve passar por comando validado, transação, undo e redo; cancelamento restaura estado inicial. Preview não modifica silenciosamente o documento persistente.
- `SelectionSet`, `ResolvedTransformTargets`, `ActiveElement`, `Pivot`, `GizmoDisplayLocation` e `Orientation` são conceitos distintos.
- Arrasto da geometria **não a move** por padrão (SEL-004); somente alça/comando explícito.
- Snap automático somente nos contextos aprovados (pivô, Point-to-Point), demais operações suaves por padrão.
- Projeções/medidas informativas não podem interceptar picking da geometria sem mecanismo explícito de interação.
- Inspeção de seção não cria faces reais; apenas `Create Geometry from Section` explícito poderá produzir geometria.
- Referências não sobreviverão à mudança topológica por reutilização de índices: IDs estáveis ou invalidar claramente.
- Geometria avaliada vs original devem ser distinguíveis; medida aproximada rotulada; algoritmo indisponível não fabrica dados.
- Manual override de rig protegido; desligar autorig nunca apaga skeleton existente.
- Nenhuma automação impede edição manual ou altera o material final sem solicitação.
- Windows/Linux, GPU modesta, funcionamento offline, baixo ruído visual, alto contraste, navegação por teclado e redução de movimento são requisitos de aceitação.

## 0.5 Precauções de engenharia

- Evitar `panic!`, `unwrap`, `expect` em produção para entradas não confiáveis; validar NaN/Inf, limites de dimensão, degenerescência, excesso de memória, grafos cíclicos e refs inválidas.
- Não executar cálculos longos no thread UI Qt; jobs por revisão, canceláveis; resultados stale não sobrescrevem o estado atual.
- OpenGL só no contexto e thread corretos, respeitando `QOpenGLWidget` e perda/recriação de contexto.
- Erros atravessando FFI com contrato tipado e propriedade de memória explícita; não emitir exceptions C++ atravessando C ABI.
- Chaves e gestures devem ser semânticas e configuráveis; sem keycodes hardcoded na geometria.
- Fallback gráfico, headless CLI e MCP não devem perder capacidade de domínio ao adicionar UI nova.
- Diagnósticos não devem tratar uma escolha artística como erro anatômico.

## 0.6 Problemas presentes na baseline, para investigação e saneamento

> Achados da leitura GitHub + CI de 2026-10-15. Revalidar ao iniciar. **Não são prova de que a UI Qt não funciona**.

- Commit `584f83573d3049009524b4718c6f1194b58f1b8a` remove Slint, cria crate `petunia_editor`, define `petunia_ui_cxxqt::widgets::run()` como entrada. `main` verificada: `92c22ca`.
- `README.md`, `PROJECT_STATE.md`, `crates/ui-cxxqt/README.md` e trechos de `AGENTS.md` ainda descrevem Slint/Qt experimental ou branches anteriores. Reconciliar canonicamente, sem apagar compromissos de segurança.
- `.github/workflows/ui-slint.yml` ainda executa `cargo fmt -p petunia_ui_slint` e falha porque a crate foi removida; migrar gate para Qt/Editor e checar dependências de runner Qt.
- `bible-check` falhou com 7 artefatos gerados defasados (`docs/generated/*`, shortcuts); `docs-generate` com revisão de diff.
- VitePress falhou por 15 links mortos, majoritariamente navegação e research; corrigir os links reais sem desligar indiscriminadamente a verificação.
- Nome histórico `SlintUiBridge` persiste em código de `petunia_editor`/adapter Qt: renomear com cuidado, após mapear callers, e **não** como prerequisito de funcionalidades se houver risco desproporcional.
- Testes declarados no commit da migração: 116 `petunia_editor`, 21 `editor_parity`, 10 `bridge_test`, `cargo check --workspace`; são relatos do commit, não execução desta documentação.

## 0.7 Comandos de reconhecimento (adapte ao estado real)

```bash
git status --short --branch
git rev-parse HEAD
rg -n 'SlintUiBridge|slint|petunia_ui_cxxqt|QOpenGLWidget|QtViewportRenderer' \
    README.md AGENTS.md Cargo.toml crates src website/docs .github/workflows
rg -n 'Rig|Skin|Pose|Selection|Gizmo|Snap|Section|Thickness|Undo' \
    crates/core crates/project crates/editor crates/render crates/ui-cxxqt
CARGO_BUILD_JOBS=2 cargo check --workspace --locked
CARGO_BUILD_JOBS=2 cargo test -p petunia_editor --lib
CARGO_BUILD_JOBS=2 cargo test -p petunia_ui_cxxqt --test editor_parity -- --test-threads=1
CARGO_BUILD_JOBS=2 cargo test -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1
```

**Não execute tudo cegamente:** conferir recursos livres (`df -h .`, `free -h`), toolchain, Qt nativo, status Git e os gates exigidos pelo protocolo canônico atual. Testes Qt com estado global devem ser serializados quando necessário. Não confundir offscreen com teste nativo de QOpenGLWidget.

## 0.8 Template obrigatório de entrega do agente

```markdown
### Objetivo e baseline
- Branch / HEAD inicial / HEAD final / working tree
### Gap Matrix
- Existente verificado, parcial, ausente, bloqueado (com paths)
### Implementado nesta fatia
- Comandos, UX, APIs, dados, módulos e alterações
### Evidências
- Comando de teste; resultado pass/fail/not run/blocked; logs/prints/capturas
### Compatibilidade
- Undo, save/load, export, Win/Linux, acessibilidade, performance
### Pendências, riscos e próximo slice
- Nenhum componente anunciado como pronto sem gate correspondente
```
