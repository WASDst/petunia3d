# Matriz de Implementação e Correção — estado real pós-G6 (2026-09-29)

> **Natureza:** registro operacional derivado; não substitui o Livro Vivo.
>
> Base: thread Codex `01a0e429…` (27–29/09/2026, 40 MB, 6.753 eventos) verificada contra o código em `main`
> (`0daebf3`, 3 arquivos de Inspector não commitados). Verificação **estática**: nada foi compilado/rodado.

## 0. TL;DR

- O Codex fez: dossiê de auditoria → **G0** (transações/revisões) → **G1** (Paint stroke/composição) → **G2** (upload GPU regional) → **G3** (Spline Core) → **G4** (SurfaceAttachment) → **G5** (Profile+Sweep persistentes) → **G6** (authoring visual do Profile). Tudo publicado; último commit da trilha: `2d4390d`.
- O que o código confirma: G0 (parcial), G1 e G2 são reais e alcançáveis no produto. G3/G4/G5 são **fundação headless**: só testes e o fluxo Profile os consomem. G6 está implementado mas **sem gates registrados**.
- O que o código **contradiz**: "839 testes/workspace" (é subconjunto de 6 crates); "single owner transacional" (≈120 chamadas diretas restam); Settings da UI afirma WCAG/AccessKit/"zero hardcode" (falso); Hair **não começou**.
- Próximo passo real do roadmap: **G6.1** (endurecer Profile) → **H0 Curve→Ribbon Mesh**. Mas há 6 bugs de correção e 4 gargalos que valem antes.

> Nota de numeração: os "G0–G6" do Codex são gates de *remediação*; o roadmap Hair tem outra sequência G0–G8 (S0–S4, H0–H8). Não confundir.

## 1. Linha do tempo da thread × estado no código

| Gate (Codex) | Entrega alegada | Commit | Veredito no código | Observação |
| :-- | :-- | :-- | :-: | :-- |
| Dossiê | 5 docs (`docs/audits/tooling-product-2026-09-27/`) | `c8060c2` | VERIFICADO | Achados majoritariamente confirmados (ver §2) |
| G0 | Dispatcher dono único de checkpoint/rollback; `ProjectChanges`; budget profundo; cache de modifiers | `c7d71c6` | **PARCIAL** | Vale só p/ `Command`; 4 comandos não-destrutivos + ~120 call sites diretos (ui-slint 34 `checkpoint`, 47 `emit_*`) |
| G1 | `StrokeSampler`, `DirtyTiles`, canvas nativo | `25d8956` | VERIFICADO | Airbrush/legado egui divergem; undo de stroke ainda é snapshot de `Project` inteiro |
| G2 | Upload `write_texture` por sub-região | `500cdf4` | VERIFICADO | Alinhamento de bytes_per_row OK; refs de imagem ainda hashadas todo frame |
| G3 | `SplineResource`, cache, comandos, snap | `f32eafc` | VERIFICADO headless | Snap e `SplineEvaluationCache` sem consumidor em produção; comandos não registrados em `canonical()` |
| G4 | `SurfaceAttachment` + 5 comandos | `a3d4b6a` | VERIFICADO headless | **Zero alcance de UI**; O(pontos×faces) por avaliação |
| G5 | Profile/Sweep persistentes, cache, Bake, migração | `71060e9` | VERIFICADO headless | UI "Sweep" ainda usa `build_sweep_mesh` legado |
| G6 | Editor Profile em `ProfileId+SplineId` | `2d4390d` (título diz Inspector) | IMPLEMENTADO, **sem evidência de gate** | Doc §gates vazio; suíte Slint completa/clippy/docs-check pendentes segundo o próprio Codex |
| Inspector redesign | peek/pin flutuante | `0daebf3` + WIP | **WIP inconsistente** | ver §3 (regressão provável) |

Testes reais (`#[test]`): core 192, project 135, mesh 140, ui-slint 351, module-paint 30, egui-ui 354, demais ≤5 → ~1.303 no workspace. "839" = soma de 6 crates numa foto anterior.

## 2. Matriz de Implementação e Correção

Legenda estado: COMPLIANT · PARTIAL · HEADLESS (existe sem alcance de produto) · BROKEN · MISSING · FALSE (doc/UI afirma o que o código não faz).
Prioridade: **P0** integridade/perda de dados · **P1** correção/perf visível · **P2** dívida · **P3** polimento.
Esforço: S (<1 dia) · M (1–3 d) · L (>3 d).

### 2.1 Integridade, comandos, revisões (domínio)

| ID | Área | Estado real | Evidência | Correção | Pri | Esf |
| :-- | :-- | :-: | :-- | :-- | :-: | :-: |
| D-01 | Save/Load assimétrico | **BROKEN** | `format.rs:31-33` grava até 256 MiB de JSON; loader rejeita >32 MiB (`MAX_JSON_BYTES`); canvases como arrays JSON (~14 MB/1024²) | Canvases em blobs binários no ZIP; validar reabertura pós-save; alinhar limites | **P0** | M |
| D-02 | Dono único de transação (TX-02) | PARTIAL | `AddPrimitiveCmd:1395`, `BooleanOpCmd:2835`, `JoinObjectsCmd:2884`, `SaveActiveAsAssetCmd:2627` não-destrutivos; ui-slint 34 `checkpoint` | Migrar os 4 comandos; substituir call sites da bridge por `dispatch` | P1 | L |
| D-03 | Undo duplo-clone | perf | `command.rs:266` clona `original`, `checkpoint_sized(&original)` clona de novo (`commands/lib.rs:82`) | `push(original)` por valor | P1 | S |
| D-04 | Falha de dispatch deixa sessão mutada | BROKEN | `freeze_active_primitive_for_command` já consumiu `primitive_session` no rollback | Snapshot/restore da sessão junto do projeto | P1 | S |
| D-05 | Undo sem no-op check | BROKEN | `UvStitch`, `Dissolve` sem mudança criam entrada + bump | Guard de no-op no dispatcher | P2 | S |
| D-06 | Redo ilimitado / eviction O(n) / budget estourável | PARTIAL | `evict_to_budget` só aparado undo; `Vec::remove(0)`; mantém ≥1 entrada | `VecDeque`, trim de redo | P2 | S |
| D-07 | API `UndoStack::checkpoint` não dimensionada | armadilha | `commands/lib.rs:217` `size_of_val` | Tornar privada/removê-la | P2 | S |
| D-08 | Revisões granulares (RV-01) | PARTIAL | `emit_mesh_changed→GEOMETRY` bumpa 5 domínios; `Command::changes()` default GEOMETRY (41/99 overrides); undo/redo = `ALL`; fingerprint global (`render_revision.rs:81`) | Overrides por comando; revisão por asset; undo com changeset | P1 | M |
| D-09 | Cache de modifiers (CA-01) | PARTIAL | chave correta (hash de conteúdo) mas cache **morto**: `evaluated_mesh(&self)` não escreve; hash O(V+F)+clone a cada chamada (render, picking, export) | Cache no owner por revisão de asset; `evaluated_mesh` devolve `Arc`/`Cow` | **P1** | M |
| D-10 | Fingerprint O(V+F) por frame com revisões 0 | perf | `render_revision.rs:147-194` | Nunca deixar revisão 0 (inicializar ao carregar) | P1 | S |
| D-11 | Spline `resample` quadrático | perf | `spline.rs:~597` varredura linear por amostra, até 262.144² | Busca binária em `cumulative_lengths` | P1 | S |
| D-12 | Invariantes de Profile não validados | BROKEN | `Add/Move/SetHandles/AttachSplinePointCmd` sem `validate_authoring` (só `UpdateSplineCmd`) | Validar em todos os comandos de spline de Profile | P1 | S |
| D-13 | Revision de spline manual/rebobinada na UI | smell | `ui-slint/lib.rs:3575,3605,3509,6678,3860` | Comando bumpa a revisão; remover hacks | P2 | S |
| D-14 | Drag muta direto (fora de Command) | PARTIAL | G6 admite; inserção+drag | Preview transacional (modal) real | P2 | M |
| D-15 | Attachment sem alcance de UI | HEADLESS | nenhum uso em ui-slint/render | UI de Attach/Slide/Reproject (H3) | P2 | L |
| D-16 | `surface_attachment_status` O(F)/ponto | perf | `topology_fingerprint()` por ponto; `project_ray` sem BVH | Cachear fingerprint por revisão de topologia; BVH | P1 | M |
| D-17 | Falha de 1 attachment derruba o generator inteiro | design | `resolved_spline` | Degradar por ponto + diagnóstico | P2 | S |
| D-18 | Sweep ignora workplane; Bezier perde cantos | BROKEN | `evaluate_sweep` só lê xy; resample por spacing | Aplicar workplane; amostrar vértices duros | P1 | M |
| D-19 | Cache de generator de 1 entrada | perf | `PathGeneratorEvaluationCache` | LRU pequeno | P3 | S |
| D-20 | Dois modelos de Profile | DUPLICATED | `ProfileState` (`state.rs:141`) + `draw_profile.rs` + recursos persistentes; adapter clonado por frame | Remover owner transitório (G6.1) | P1 | M |
| D-21 | Comandos G3–G5 não registrados em `canonical()` | MISSING | sem palette/keymap/MCP | Registrar com `CommandSpec` | P2 | S |
| D-22 | Código morto domínio | HD-01 | `UvModule::stitch/relax`, `snap_spline_position`, `SplineEvaluationCache`, `SplineResource::estimated_bytes`, `source_topology_revision`, `evaluated_mesh_cached` (só testes) | Remover ou ligar; `#[allow(dead_code)]` 19 | P3 | S |

### 2.2 PAINT

| ID | Item | Estado | Evidência | Correção | Pri | Esf |
| :-- | :-- | :-: | :-- | :-- | :-: | :-: |
| PA-B1 | **Ativar Fill pinta vértices** | **BROKEN** | `ui-slint/lib.rs:1153` `SetActiveTool("fill")` chama `fill_selection` sem checar workspace; sem seleção recolore tudo + clone + undo | Só executar no clique/Apply, nunca ao selecionar ferramenta | **P0** | S |
| PA-B2 | Tile composite com base oculta/opacidade 0 | BROKEN | `paint_layers.rs:661` só limpa tile em `li==0` | Limpar tile sempre antes do 1º layer visível | **P1** | S |
| PA-B3 | Borda esquerda/topo acumula | BROKEN | `(cx+dx).max(0)` (`module-paint/lib.rs:444,501,528`) | Clip em vez de clamp | P1 | S |
| PA-B4 | Camada bloqueada não é respeitada | BROKEN | brush/fill/gradient/shape ignoram `locked`; decal editável | Guard central em `canvas_mut` | P1 | S |
| PA-B5 | Alpha "over" incorreto | BROKEN | `blend_pixels:208-240` | Fórmula Porter-Duff | P1 | S |
| PA-B6 | Effect layer: tile ≠ full | BROKEN | `paint_layers.rs:~795` vs `571`; sem teste de equivalência | Unificar + teste tile==full | P1 | S |
| PA-B7 | Canvas 2D stale após stroke 3D | BUG | `callbacks.rs:1875` sem `publish_canvas_image` | Publicar no end | P2 | S |
| PA-B8 | Painel 2D mostra só layer ativa; efeitos → `None` | PARTIAL | `render_paint_canvas` | Mostrar composto | P2 | S |
| PA-B9 | Brush 3D mistura unidades | BUG | `size_px=radius*16` usado em texel e passo de tela | Separar raio UV e espaçamento em tela | P2 | S |
| PA-B10 | Reupload possível omitido | edge | `sync_asset_textures ~1930` | Forçar full se sem update declarado | P3 | S |
| PA-P1 | Undo de stroke = clone de `Project` | perf | `state.rs:2984`; diff completo em `finish` | Diffs por tile (critério G1 não entregue) | **P1** | L |
| PA-P2 | Composição full em fill/gradient/shape/layer-op; Pixelate ⇒ full por dab | perf | `project/lib.rs:1009` | Recompor por região; Pixelate tile-safe | P1 | M |
| PA-P3 | `view_model()` + UV strings + cópia da layer por evento de pointer | perf | `callbacks.rs:1845-1885` | View-model incremental; throttle por frame | **P1** | M |
| PA-P4 | Picking O(faces) sem BVH; usa malha base, não avaliada | perf/BUG | `projection.rs:1601` | BVH por asset (compartilhar com D-16) | P1 | M |
| PA-P5 | Referências hashadas por frame | perf | `render-wgpu:2054-2090` FNV ~16 MB/frame (2048²) | Revisão monotônica | **P1** | S |
| PA-F1 | Layer groups | STUB | flag em raster 1×1; compositor ignora `group_id` | Compor grupos de verdade | P2 | M |
| PA-F2 | Máscaras de layer | MISSING | sem campo | Adicionar `mask` (V1 Substance-like) | P2 | L |
| PA-F3 | Blend modes | PARTIAL | 4 modos, sem UI | +Overlay/Soft Light/Color; UI | P3 | M |
| PA-F4 | Gradiente | PARTIAL | só linear, cor final fixa, sem preview | Preview + 2 cores | P3 | S |
| PA-F5 | Multicanal (rough/metal/normal) | RUDIMENTARY | só albedo | Canais V1 após pipeline | P3 | L |
| PA-D1 | Código morto | DEAD | `paint_screen_space`, `fill_scope`, `is_paintable`; GL legado hasha texturas | Remover/ligar | P3 | S |

### 2.3 Viewport / Renderer

| ID | Item | Estado | Evidência | Correção | Pri | Esf |
| :-- | :-- | :-: | :-- | :-- | :-: | :-: |
| VP-01 | Modos de shading ≠ cap. 05 e copy enganosa | **FALSE** | `app.slint:5352-5353` promete PBR/ray-traced/sombras; renderer: 1 luz difusa | Corrigir copy agora; depois reconciliar enum (Textured/Solid/Wireframe/Silhouette) | **P1** | S |
| VP-02 | Flat/Smooth não persiste nem é honrado | STUB | `render-wgpu:1374` `smooth=false` fixo | Propriedade por asset + normais por vértice | P1 | M |
| VP-03 | Sem culling/dirty por asset/batching | RUDIMENTARY | rebuild de todos os assets por qualquer mudança | Buffers por asset + frustum coarse | P1 | L |
| VP-04 | Camera/hover clona malha 2× | perf | `update_selection_layer:1616,1723` | Pular em Object domain; cache avaliado (D-09) | **P1** | S |
| VP-05 | Vertex paint reconstrói tudo por evento | perf | `color_revision` no fingerprint global | Domínio próprio de cor | P2 | S |
| VP-06 | Upload por retângulo | perf | 1 `write_texture` por rect | Cap por frame / merge | P3 | S |
| VP-07 | Viewport soft clona por frame | perf | `viewport_soft.rs:301` | Cache | P3 | S |
| VP-08 | Controles de viewport | MISSING | sem FOV/clip, bookmarks, FPS, histórico de undo; grid = texto estático; snap target sem UI | Painel de Viewport/Snap | P2 | M |

### 2.4 UI / UX / Acessibilidade / Input

| ID | Item | Estado | Evidência | Correção | Pri | Esf |
| :-- | :-- | :-: | :-- | :-- | :-: | :-: |
| UI-B1 | **WIP Inspector inconsistente** | **BROKEN (provável)** | diff local: `open` default→false, UI lê `pin_open`; `on_section_open_toggled` (`lib.rs:6642`) ainda alterna `open`; `tests.rs:8273` espera `open=true`; `restore_section_layouts` ignora `open` salvo | Unificar `open`/`pin_open`, atualizar teste, decidir migração de prefs, **antes de commitar** | **P0** | S |
| AX-01 | Alegações falsas nas Settings | **FALSE** | `app.slint:10843-10846, 11019, 11061` ("Zero strings", "AccessKit ativada", "Tab/Shift+Tab", "WCAG 2.2 AA") | Trocar por estado honesto | **P1** | S |
| AX-02 | Escala de UI não faz nada | **FALSE** | `ui-scale` só destaca botões; nada consome | Ligar a fator de layout/fonte | **P1** | M |
| AX-03 | Teclado: 249 TouchArea × 7 FocusScope | BROKEN | Settings/rail/pin sem foco; 9 `key-pressed` | Componentes base focáveis (`Button`, `Tab`, `Pill`) | P1 | L |
| AX-04 | Inspector peek/pin só por hover | BROKEN | `InspectorPill` sem FocusScope; pin 22 px | Atalho + foco + alvo ≥32 px | P1 | M |
| AX-05 | Modal sem `dialog`, trap e retorno de foco | MISSING | 0 `accessible-role: dialog` | `OverlayStack` gerencia foco | P1 | M |
| AX-06 | F6/Shift+F6 | MISSING | só string "F6" | Ordem de regiões | P2 | M |
| AX-07 | Alto contraste parcial | PARTIAL | 171 hex literais; `apply_theme` ~15 tokens e sobrescreve `selection` com azul | Tokens completos; auditoria de contraste | P2 | M |
| AX-08 | Texto 9–10 px (234 usos), `control-height` 28 px não usado | PARTIAL | | Mínimos em token | P2 | M |
| IN-01 | Bypass físico antes do keymap | BROKEN | `route_shortcut:9508`; W hardcoded; `CommandId` (46) ≠ core (IDs string); `OpenProject` é stub (`lib.rs:9281`) | Gesto→`CommandId` único; `CommandSpec` projetado | P1 | L |
| IN-02 | Aba Teclado é texto estático | STUB | `app.slint:11007` | Editor de atalhos real | P2 | M |
| UI-05 | Zero hardcode de texto | BROKEN | 221 literais `text:` (~210 únicos) + 215 `set_status` literais; PT/EN misturados; 0 `@tr` | Projetar `TextId` em Slint | P2 | L |
| UI-06 | Vocabulário canônico | BROKEN | "Bevel" na toolbar/locale vs "Round Edge" | Renomear | P2 | S |
| UV-01 | Workspace UV público | OBSOLETE | pill `app.slint:651`; 59 checagens `active-workspace` | Mover p/ "Preparar superfície" em PAINT | P1 | M |
| UV-02 | Editor UV via strings SVG | perf | `build_uv_editor:5997`, trunca 2000 faces; roda a cada pointer event | Paths estruturados + retained | P1 | L |
| UV-03 | Seleção UV acoplada a `Face.selected` | BROKEN | `uv_editor_click:6118` | IDs de UV próprios | P2 | M |
| UV-04 | `uv_move_selected` vs comentário | BUG | `lib.rs:6165` | Alinhar semântica | P3 | S |
| UI-M1 | Settings: Plugins/MCP estáticos | FALSE | "Ativo" fixo | Dados vivos ou remover | P2 | S |
| MOD-01 | Ferramentas com lacunas | PARTIAL | Mirror só modifier; `symmetrize/weld` sem botão; Bevel 1 segmento convexo | Expor; bevel multi-seg | P2 | M |
| AR-01 | Monólitos | RUDIMENTARY | app.slint 11.234; lib.rs 11.666; callbacks 4.762; tests 8.408 | Fatiar por feature | P2 | L |
| AR-02 | `Arc<Mutex<Bridge>>` | PARTIAL | 242 locks, falha silenciosa (`if let Ok`) | Fila de intents; erro visível | P2 | L |
| MO-LEG | egui legado | DEBT | `crates/ui` 28.7k LOC + app 2.7k + GL; `ToolRegistry` só p/ legado | Plano de aposentadoria | P3 | L |

Panic risk baixo (0 `unwrap` em callbacks; `panic=abort`); `unsafe` só em ffi (58) e render-gl (21).

### 2.5 Shape-first / Hair (estado)

| Item | Estado | Próximo |
| :-- | :-: | :-- |
| S0 Profile persistente | COMPLIANT (editor) / DUPLICATED (owner transitório) | G6.1: inserção 100% Command, remover `ProfileState`, UI para listar/reabrir Profiles, nudge/foco por teclado |
| S3 Sweep/Revolve/Extrude vivos | PARTIAL | UI de Sweep passar ao generator persistente; Extrude/Revolve como generators |
| Spline Core (P3D-161) | HEADLESS | overlay já existe via Profile; editor genérico de curvas |
| SurfaceAttachment (P3D-158) | HEADLESS | UI + BVH + malha avaliada |
| H0 Curve→Ribbon Mesh | **MISSING** | iniciar só após G6.1 + D-11/D-16/D-18 |
| H1–H7 | MISSING | conforme roadmap |

## 3. Ordem recomendada (sem big-bang)

1. **Sprint A — "não perder trabalho / não mentir"** (todos S): D-01, PA-B1, UI-B1 (antes de commitar o WIP), AX-01, VP-01 (copy), D-12, D-03, D-04.
2. **Sprint B — correção de Paint**: PA-B2…B6, B7, B9, PA-P5, VP-04, D-10, D-11.
3. **Sprint C — performance estrutural**: D-09 (cache avaliado) → D-08 (revisões por asset) → VP-03; PA-P1/P2/P3; picking BVH (PA-P4+D-16).
4. **Sprint D — acessibilidade/UX**: AX-02…AX-06, IN-01/IN-02, UV-01/02.
5. **Sprint E — G6.1 + H0**: D-20, D-14, D-18, D-21, depois Hair Ribbon.

Cada sprint: testes só ao fechar o conjunto (regra do usuário), `fmt/check/clippy`, `docs-check`, `bible-check`, `ui-guard --strict`, medição antes/depois para toda alegação de performance; site VitePress permanece congelado.

## 4. Verificação (para quando houver implementação)

- Reproduzir PA-B1 (selecionar Fill sem seleção → entradas de undo/vértices alterados) e UI-B1 (`cargo test -p petunia_ui_slint --lib section_layout`).
- Teste de round-trip save→load com projeto texturizado >32 MiB (D-01).
- Testes de equivalência tile==full (PA-B2/B6), `do→undo→redo` por hash (D-02/D-05).
- Bench: frames com 1/100/1000 assets; eventos de pointer em Paint com UV overlay ligado; referência 2048².
- Rodar suíte **completa** do workspace (a "839" do Codex é subconjunto) e registrar resultado real no relatório G6.

## 5. Ressalvas

- Análise estática; itens "read from code" carecem de repro/profiling.
- Os 3 arquivos não commitados do Inspector não foram tocados.
