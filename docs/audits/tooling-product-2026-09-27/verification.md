# Verificação, método e limites

## 1. Snapshot auditado

- branch: `main`;
- commit final de referência: `05b2ead`;
- commit no início da auditoria: `819ab77`;
- data: 2026-09-27;
- toolchain requerido pelo workspace: Rust `1.98.1`;
- delta de código acompanhado durante a auditoria e consolidado em `05b2ead`:
  - `crates/render-wgpu/src/lib.rs`;
  - `crates/ui-slint/src/callbacks.rs`;
  - `crates/ui-slint/src/lib.rs`;
  - `crates/ui-slint/src/tests.rs`;
  - `crates/ui-slint/ui/app.slint`.

Essas alterações de produto foram produzidas em paralelo à auditoria, foram
lidas para avaliar o estado atual e não foram implementadas como parte do
dossiê. Como o código evoluiu durante a leitura, os achados usam símbolos/fluxos
como evidência e `05b2ead` como âncora final reproduzível.

## 2. Método

### 2.1 Reconciliação

1. leitura de `AGENTS.md`, `ENTRYPOINT.md`, `prumo.json`, `docs/PRUMO.md` e estado
   do projeto;
2. leitura das páginas canônicas sobre arquitetura, UI, input, viewport,
   performance, MODEL, PAINT, UV, spline, attachment, bake e Hair;
3. inventário de crates, entrypoints, comandos, modules, view-model, callbacks,
   renderer e testes;
4. rastreio de fluxos alcançáveis a partir do shell Slint;
5. classificação com o vocabulário obrigatório da matriz;
6. comparação com documentação oficial dos produtos de referência;
7. execução de testes e gates no toolchain fixado.

### 2.2 Critério de realidade

- presença de arquivo/tipo não conta como feature alcançável;
- teste do host egui não comprova alcance no Slint;
- string/tooltips não comprovam capacidade do renderer;
- estado em view-model não comprova persistência/undo;
- cache só conta se a chave representar todas as dependências;
- performance só é declarada como medida quando há counter/benchmark; nos outros
  casos, o relatório descreve complexidade, cópia e alocação observáveis.

## 3. Métricas do snapshot

| Hotspot | Linhas/ocorrências |
| :--- | ---: |
| `crates/ui-slint/ui/app.slint` | 11.441 linhas |
| `crates/ui-slint/src/lib.rs` | 11.210 linhas |
| `crates/ui-slint/src/callbacks.rs` | 4.766 linhas |
| `crates/ui-slint/src/tests.rs` | 7.745 linhas |
| `crates/core/src/command.rs` | 3.926 linhas |
| `crates/core/src/state.rs` | 3.253 linhas |
| `crates/mesh/src/ops.rs` | 2.634 linhas |
| `crates/module-paint/src/lib.rs` | 2.092 linhas |
| `TouchArea` em `app.slint` | 250 |
| `accessible-role` em `app.slint` | 80 |
| `accessible-label` em `app.slint` | 83 |
| cores hex literais em `app.slint` | 133 |
| `render_viewport(` em callbacks | 125 |
| `.lock()` em callbacks | 241 |

As contagens são indicadores de hotspots, não diagnósticos isolados. Por
exemplo, um parent semântico pode cobrir mais de um `TouchArea`; por isso a
conclusão de acessibilidade também usou inspeção dos controles.

## 4. Gates executados

Todos os comandos abaixo foram executados com `RUSTUP_TOOLCHAIN=1.98.1`, porque
o `stable` ativo no ambiente era anterior ao `rust-version` do workspace.

### Formatação e compilação

```bash
cargo fmt --all -- --check
cargo check --workspace
```

Resultado: **verde**.

### Testes relevantes

```bash
cargo test \
  -p petunia_mesh \
  -p petunia_core \
  -p petunia_module_model \
  -p petunia_module_paint \
  -p petunia_module_uv \
  -p petunia_render_wgpu \
  -p petunia_ui_slint \
  --lib
```

| Crate | Resultado |
| :--- | ---: |
| `petunia_mesh` | 113 passed |
| `petunia_core` | 98 passed |
| `petunia_module_model` | 1 passed |
| `petunia_module_paint` | 27 passed |
| `petunia_module_uv` | 5 passed |
| `petunia_render_wgpu` | 2 passed |
| `petunia_ui_slint` | 331 passed |
| **Total** | **577 passed; 0 failed** |

Os testes WGPU `wgpu_viewport_initializes_or_skips_when_no_gpu` e
`hover_updates_selection_without_rebuilding_scene_geometry` passaram no snapshot
final.

### Clippy

```bash
cargo clippy \
  -p petunia_ui_slint \
  -p petunia_render_wgpu \
  -p petunia_core \
  -p petunia_mesh \
  -p petunia_module_model \
  -p petunia_module_paint \
  -p petunia_module_uv \
  --all-targets -- -D warnings
```

Resultado: **verde**.

### Guards do projeto

```bash
cargo run -p xtask -- docs-check
cargo run -p xtask -- bible-check
cargo run -p xtask -- ui-guard --strict
```

Resultado: **verde** nos três.

Observações não bloqueantes já reportadas pelas ferramentas:

- `docs-check` listou 357 símbolos públicos sem nó no mapa de componentes;
- `ui-guard --strict` listou 302 ocorrências candidatas a migração na UI egui
  legada, mas nenhuma violação de confinamento;
- o VitePress não foi compilado, corretamente, porque o site público está
  congelado.

## 5. Por que testes verdes não anulam os achados

A suíte demonstra boa cobertura de algoritmos e muitos fluxos de bridge. Os
achados críticos estão em propriedades ainda não afirmadas pelos testes:

- um teste pode verificar que delete/dissolve alterou contagens sem executar
  undo/redo e comparar conteúdo;
- layer mutations podem validar composição sem provar que o checkpoint capturou
  o estado anterior;
- decal pode ser transformado e baked sem verificar undo do drag;
- fill pode alterar pixels sem testar uma entrada única de histórico;
- cache de modifiers pode ser testado somente quando contagens mudam;
- shading pode ser “distinto e alcançável” sem ser PBR/ray-traced;
- testes de preferências não equivalem a navegação integral por leitor de tela.

Os primeiros testes de remediação devem ser escritos para falhar nesses
contratos antes de alterar a implementação.

## 6. Verificações ainda necessárias

### Visual/usabilidade

- sessão guiada em 1280×720, 1440×900 e 4K/200%;
- keyboard-only de abrir projeto até modelar/pintar/salvar;
- NVDA/Orca/VoiceOver conforme plataformas suportadas;
- contraste por estado e tema;
- focus trap/restore em overlays aninhados;
- tablet/pressure quando houver backend suportado.

### Performance

- puffin/tracing de stroke com 256², 1K, 2K e múltiplas layers;
- bytes alocados por sample/dab;
- tempo de compositor vs upload;
- scene fixtures com muitos assets/modifiers;
- cache hit/miss e geometry rebuild count;
- GPU capture para overdraw, linhas e bind groups;
- memória real de undo após strokes e operações destrutivas.

### Robustez

- fault injection em jobs/import/export;
- cancelamento e stale revision;
- projetos hostis/corrompidos;
- recovery após ação interrompida;
- round-trip de generators futuros;
- testes de `NeedsReattach` após mudança topológica.

## 7. Reprodutibilidade e manutenção do dossiê

Este diretório é um snapshot datado. Não deve ser editado para aparentar que
achados históricos nunca existiram. Correções futuras devem:

1. referenciar o ID do achado no commit/PR;
2. adicionar teste de regressão;
3. registrar `RESOLVED` em um novo checkpoint ou apêndice datado;
4. manter o Livro Vivo como autoridade de requisitos;
5. não alterar o site congelado para publicar este dossiê antes da etapa final.
