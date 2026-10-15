# Análise de estado — features do Petunia3D (excluindo a GUI)

> **Documento associado a este worktree e à sua branch.**
>
> | Campo | Valor |
> |---|---|
> | Worktree do documento | `/run/media/raillen/3854B63454B5F52A/petunia3d-qt-integration` |
> | **Branch associada** | **`codex/qt-integration`** |
> | HEAD associado | `095ea728edef22e73c13bed53ad96e741178e999` — `refactor(qt): consolidate frontend on shared functional repository` (2026-10-13 12:40:45 -0300) |
> | Worktree irmão (linha funcional) | `/run/media/raillen/3854B63454B5F52A/petunia3d-refact` @ `641a7d1e` — `refactor/architecture-foundation` |
> | Data da análise | 2026-10-13 |
> | Escopo | Features não-GUI: Core, Application/Document, Geometry, Project/Assets, Renderer, Materials/Paint/UV, Animation, Interchange, Extensions (Lua/MCP), Quality/Perf |
> | Status do arquivo | Não commitado neste worktree (o worktree está com 30 entradas de WIP) |

## Onde as medições foram executadas

As medições de código e a execução da suíte foram feitas no **worktree funcional** (`refactor/architecture-foundation` @ `641a7d1e`), porque é a linha onde o tracker e o código não-GUI são mantidos. A equivalência com a branch deste worktree foi conferida arquivo a arquivo:

| Caminho | Estado entre `refactor/architecture-foundation` e `codex/qt-integration` |
|---|---|
| `crates/mesh/**` | **idêntico** (inclui `crates/mesh/src/bevel.rs`, linha 515) |
| `crates/project/src/**` | idêntico |
| `crates/render/src/**` | idêntico |
| `crates/core/src/**` | **difere**: `lib.rs` e arquivo extra `reference_commands.rs` (só no Qt) |
| `website/progress/tasks.json` | difere: 67 tarefas no funcional, **68** aqui — a extra é **Q06 “Paridade funcional Qt com o editor atual” (IN PROGRESS)** |

Consequência: toda a análise de Geometry/Project/Renderer vale igualmente para `codex/qt-integration`; a parte de Core pode ter um delta pequeno aqui (`reference_commands.rs`). A falha de teste descrita abaixo **existe nas duas branches**.

---

## TL;DR

1. **O motor não-GUI está muito mais construído do que o tracker sugere.** ~115k linhas em 16 crates (core, mesh, project, render, plugins, mcp, module-*), com kernels reais: booleano via `manifold-rust`, bevel/inset/loop-cut/knife/imprint, unwrap via `xatlas`, autosave com gerações, export GLB/OBJ, host Lua com capabilities, adapter MCP com 9 tools.
2. **O que falta é, na maior parte, migração de contrato e verificação — não feature nova.** Das 53 tarefas não-GUI: **2 DONE**, **21 IN PROGRESS**, **30 TODO**; vários TODO têm baseline dizendo “Parcial: código existe em `arquivo:linha`”.
3. **Ausências reais confirmadas por busca no código** (contrato aprovado, tipo inexistente): Collections/parenting (G09), `ObjectId`/`GeometrySource` (P01), `MaterialId`/`TextureId`/`TextureResource` (M01), `PerspectiveReference`/multi-view (F03), jobs canceláveis/coalescing (Q01), release/packaging (Q04).
4. **Há uma regressão vermelha no motor:** `cargo test` não-GUI = **1222 pass / 1 fail**. O fail é `petunia_mesh::bevel::tests::unsupported_multiple_edges_and_nonfinite_width_are_atomic` (G16, commit `9c79f99`).

---

## Método e fontes

| Fonte | Uso |
|---|---|
| `website/progress/tasks.json` (fonte única) | 67 processos, checkpoints, evidências, datas e gates |
| `website/docs/manifest.json` | inventário canônico por domínio |
| `website/docs/00-philosophy/decision-register.md` | ADR-C01…C10, escopo adiado, sequência de execução |
| `website/docs/16-code-agents/implementation-protocol.md` | formato de gap matrix, gates por responsabilidade |
| leitura direta de `crates/**` (ripgrep + leitura de módulos) | presença/ausência de tipos e contratos |
| `cargo test --workspace --exclude petunia_ui_slint --exclude petunia_ui_cxxqt --exclude petunia3d --no-fail-fast` | única prova de execução deste relatório |

Contagem de estados no tracker funcional: 4 DONE, 32 IN PROGRESS, 31 TODO (67). Não-GUI: 2 DONE, 21 IN PROGRESS, 30 TODO (53).

---

## Estado por domínio (não-GUI)

| Domínio | Existe no código (evidência) | Lacuna principal | IDs |
|---|---|---|---|
| **Core / Application** | `AppState`, `EditorSession`, `ViewportSession`, `CommandDispatcher`, `CommandContext`, `CommandEffects`, `SelectionDomain`, `core/src/jobs.rs` | `UiState` ainda dentro do Core (`core/src/state.rs:1260`); `AppState` god-object; `ModuleRegistry` não aposentado; `DesktopHost` mora no `ui-slint` | A01, A02, A04, A06, A07 |
| **Geometry (mesh)** | `bevel`, `inset`, `loop_cut`, `knife`, `imprint`, `connect`, `ops` (revolve/sweep/extrude), `boolean` + `boolean_cleanup`, `validation`/`cleanup`, `uv_xatlas/relax/stitch/layout`, `shading` | FaceCorner não é transversal (`verts`+`uv` paralelos ainda); weld/remap OK, mas UV/material/provenance/sharp por corner pendente | G01–G04, G16–G18 |
| **DRAW/POLY, splines, workplane** | `poly_pen.rs`, `draw_profile.rs`, `core/src/inference.rs`, `core/src/snap.rs`, `project/src/spline.rs`, `profile.rs` | Dois motores/profile transitório; SplineId/PlanarShape não unificados; Workplane global por gesto ausente | G05, G06, G07 |
| **Hierarquia / attachments** | `prefab.rs`, `surface_attachment.rs`, `object_transform.rs`, `bvh.rs` | Sem `CollectionId`/parent-child (só `Asset.collection: Option<String>`); attachment por triângulo/index sem remap/`NeedsReattach` | G08, G09 |
| **Project / Document** | `format.rs` (ZIP V1 + postcard legado), `io_atomic` (`sync_all` antes do rename), `autosave` (5 gerações, coalescido por revisão), `pipeline` (OBJ/glTF/GLB/PKG), `import_gltf`/`import_obj` com limites, `game_ready` validator | Snapshot autoral vs EditorSession incompleto; migrations de schema/IDs; round-trip completo; `Asset` ainda acumula `mesh`+`texture`+`base_color` | P01–P04, I01, I02 |
| **Renderer** | `render/src/{frame,extract,scene,overlays,gl/*}` — GL 3.3/glow, dirty rects, caches por revisão, renderer sem `AppState` | Gates de resize/HiDPI/picking em GPU e paridade visual | R01–R03 |
| **Materials / Paint / UV** | `material.rs`, `paint_layers.rs` (layers, decals, variantes), `module-paint` (2D/3D, projeção, dirty tiles), `module-uv` + kernels UV | Sem `MaterialId`/`TextureId`/`TextureResource`; `Canvas` embutido; decal IO/sequence; UV Work Surface | M01, M02, M04, M05, V01–V03 |
| **Animation** | `core/src/animate_session.rs`, `project/{animation,motion,rig,posing,ik}.rs`, `bake_motion`, poses transacionais | Suíte `animation-workspace` e fluxo de produção | **N01 DONE** / N02 |
| **Extensions (Lua/MCP)** | `plugins/{host,registry,instance,manifest,dispatch}` (Lua 5.4, deny-by-default, sandbox só texto), `mcp/src/server.rs` (9 tools, `expected_revision`, batch atômico, audit redigido) | Ligação do host à Application; aprovação por request; painéis Slint | E01–E03 |
| **Quality / Perf** | `crates/*/benches` (`command_spine`, `core_jobs`, `mesh_ops`), `fuzz/`, CI `ui-slint.yml` | Jobs canceláveis/progress/stale, low-end medido, gates Windows, supply-chain, release | Q01–Q04 |

---

## Lacunas por tipo de trabalho

### A. Só falta executar/registrar gates (9 tarefas — linha de chegada)

Não-GUI: `G01`, `G02`, `A05`, `G17`, `G18`, `V02`. GUI (fora do escopo, citadas para contexto): `U02`, `R01`, `R03`.
Nestas o código e os testes já estão escritos; falta rodar e registrar evidência. **Maior retorno por esforço do projeto.**

### B. Finalizar sobre código existente

- **Application/Document** (`A01`, `A02`, `A04`, `A06`, `A07`; `P01`–`P04`): tirar `UiState` do Core, injetar clock/filesystem/watchers, aposentar `ModuleRegistry`, separar settings persistentes de sessão, `ObjectId`/Geometry source, stores de asset com Relink/Missing Dependencies.
- **Geometry** (`G03`–`G15`): FaceCorner transversal, DRAW/POLY num motor, Spline/PlanarShape, Workplane/inference unificado, attachments com ID estável, transforms origin/pivot, generators com contrato único, evaluation service (Base→Modified→Deformed), Shape Builder, Push/Pull.
- **Paint/UV/Reference** (`M01`, `M02`, `M04`, `M05`; `V01`, `V03`; `F01`, `F02`): IDs tipados de material/textura, PaintDocument/Channel, decal IO/sequence, promoção do UV Editor a Work Surface, ReferenceSets persistentes e Project Photo.
- **Adapters e periferia** (`I01`, `I02`, `E01`, `E03`, `Q01`, `Q03`): consolidação de pipeline, staging/limits/refs externas, ligação do host Lua, aprovação por request no MCP, jobs versionados.

### C. Contratos aprovados sem código correspondente

`G09` (Collections/parenting), `P01` (`ObjectId`/`GeometrySource`), `M01` (`MaterialId`/`TextureId`/`TextureResource`), `F03` (`PerspectiveReference`/multi-view), `Q01` (jobs canceláveis), `Q04` (release/packaging), `E02` (painéis de plugin).

### D. Fora da baseline — não é dívida obrigatória

Graph Editor completo, modifiers extras, FBX como formato principal, plugin nativo/DLL, marketplace e cloud/login, segundo backend de produção. Decisão em `decision-register.md` → “Decisões de escopo — o que fica explicitamente ADIADO”.

---

## Verificação executada

```bash
cd /run/media/raillen/3854B63454B5F52A/petunia3d-refact
cargo test --workspace --exclude petunia_ui_slint --exclude petunia_ui_cxxqt --exclude petunia3d --no-fail-fast
# log: /tmp/petunia/nongui-tests.log  (exit capturado dentro do log como EXIT=101)
```

| Métrica | Valor |
|---|---|
| Resultado | **1222 passed / 1 failed / 0 ignored** |
| Exit status | **101** |
| Suítes | 49 binários de teste + 14 doc-test suites (63 linhas de resultado) |
| Suítes com falha | 1 (`petunia_mesh` lib) |

Distribuição por suíte (passou/total):

| Suíte | Resultado |
|---|---|
| `petunia_project` | 243 pass, 0 fail (+ `pipeline_tests` 16, `material_tests` 8, `modifier_stack` 5, `motion_tests` 22, `posing_tests` 12, `save_tree` 3, `gltf_rig_roundtrip` 9, `fuzz_corpus` 3) |
| `petunia_mesh` | **253 pass, 1 fail** (+ `interactive_geometry_tests` 4, `proptest_invariants` 6, `loop_cut_directions` 2, `subdivide_cuts` 3, `triangulation_tests` 3) |
| `petunia_core` | 179 pass (+ 16 integrações: `command_tests` 52, `animate_session_tests` 19, `state_decomposition_tests` 15, `project_service_tests` 13, `shading_and_save_tests` 10, `rig_command_tests` 8, `adapter_interop` 8, `cutting_session_tests` 6, `motion_command_tests` 6, `fit_rig_tests` 6, `queries_tests` 5, `prefab_tests` 3, `proptest_jobs` 3, `snapshots` 3, `primitive_pipeline_tests` 2, `brush_radius_tests` 2) |
| `petunia_module_paint` | 121 pass (+ `stroke_coverage` 1) |
| `petunia_render` | 34 pass + `gl_pixels` 23 pass (pixel GL em Linux) |
| `petunia_config` | 24 pass |
| `petunia_plugins` | 21 pass |
| `xtask` | 17 pass |
| `petunia_commands` | 13 pass |
| `petunia_mcp` | 8 pass |
| `petunia_module_model` | 7 pass |
| `petunia_ffi` | `c_abi_tests` 7 pass |
| `petunia_cli` | `headless_integration` 5 pass |
| `petunia_application` | 4 pass |
| `petunia_module_uv` | 4 pass |
| `petunia_module_assets` | 1 pass |

### Falha única (G16 — Inset/Bevel)

```text
bevel::tests::unsupported_multiple_edges_and_nonfinite_width_are_atomic
panicked at crates/mesh/src/bevel.rs:515:9
assertion `left == right` failed
  left: (0, 1)
 right: (0, 2)
```

- **Causa:** em `Mesh::bevel_edges` (`crates/mesh/src/ops.rs:970`), a etapa 1 lista corretamente todas as arestas inválidas, mas a etapa 2 (falha de **interação entre arestas adjacentes**) faz `report.unsupported_edges.push(edge)` e retorna com **uma** aresta, enquanto o contrato/teste exige o conjunto inteiro.
- **Atomicidade preservada:** nada é mutado (o retorno acontece antes de `*self = working`); o que diverge é apenas o relatório devolvido.
- **Pré-existente:** `crates/mesh` está limpo no git e o arquivo é idêntico nos dois worktrees → a falha existe no HEAD `641a7d1e` e no HEAD `095ea728`, não vem do WIP de integração Qt.
- **Não corrigida nesta análise:** é mudança de comportamento de kernel com contrato documentado (`website/docs/03-geometry/direct-modeling-semantics.md`); exige decisão explícita.

### Gates não executados

`ui-slint`, `ui-cxxqt` e o binário `petunia3d` não foram executados. `ui-slint` **não** foi compilado por este comando (nenhum crate de `crates/*/Cargo.toml` depende de `petunia_ui_slint`); a espera inicial de ~15 min no lock do Cargo foi de outro processo já em execução no ambiente. Windows, GPU low-end, screen reader, janela nativa e CI remota: **`not run`**.

---

## Riscos

1. **Evidência “escrito, não compilado / testes not run”** em vários checkpoints (G01, G16, G17, P02, P03, I02, E01, E03). Checkpoint concluído com essa ressalva é `implemented, unverified`, não DONE — e a única falha encontrada agora está justamente num kernel com esse histórico.
2. **O tracker superestima a lacuna nas partes TODO** cujo baseline diz “Parcial/Pendente” e aponta arquivo existente: parece feature ausente, mas é migração/re-baseline.
3. **WIP não commitado:** no worktree funcional (37 arquivos + `crates/ui-cxxqt/` untracked) e neste worktree Qt (30 entradas). Enquanto isso não é fechado, o HEAD não representa o que realmente existe.
4. **Fronteira Core↔Application ainda vazando** (`UiState` no Core, `ModuleRegistry` ativo): migrações novas herdam a dívida.

---

## Próximo checkpoint (recomendação)

**Fechar a linha de chegada (bucket A), começando pela única falha vermelha.** Corrigir o relatório de `bevel_edges` (etapa 2 devolvendo o conjunto recusado) devolve o motor não-GUI a verde e destrava 6 processos que só esperam gates — muito mais barato que iniciar G09/M01/F03.

**Próximo passo concreto:** corrigir `crates/mesh/src/ops.rs:970` (`bevel_edges`, etapa 2) e rodar `cargo test -p petunia_mesh --lib`, registrando a evidência em G16/`tasks.json` no mesmo diff.

---

## Reprodução

```bash
D=/run/media/raillen/3854B63454B5F52A/petunia3d-refact   # linha funcional (medido)
Q=/run/media/raillen/3854B63454B5F52A/petunia3d-qt-integration  # worktree desta branch

# associação de branch/worktree
git -C "$Q" -c safe.directory="$Q" rev-parse --abbrev-ref HEAD   # codex/qt-integration
git -C "$Q" -c safe.directory="$Q" rev-parse HEAD                # 095ea728…

# suíte não-GUI (evidência do relatório)
cd "$D" && cargo test --workspace --exclude petunia_ui_slint --exclude petunia_ui_cxxqt \
  --exclude petunia3d --no-fail-fast

# falha isolada
cd "$D" && cargo test -p petunia_mesh --lib bevel::tests::unsupported_multiple_edges

# equivalência entre worktrees
diff -rq "$D/crates/mesh" "$Q/crates/mesh"
```

## Limites deste documento

- Medições de teste foram feitas **apenas em Linux** (Mesa/zink) e apenas no escopo não-GUI; nada aqui comprova GPU, Windows, acessibilidade ou janela nativa.
- A análise comparou documentação canônica (`website/docs/`) com código e com o tracker; **não** reauditou cada checkpoint marcado como concluído.
- Este arquivo não foi commitado. Nenhum código foi alterado.
