# Testes, CI, conformance e gates de aceitação

> **Status: aprovado em 2026-10-08 como contrato alvo.** Não interpretar este documento como relatório de testes executados. Cada gate exige execução, link e evidência em commit/runner.

## Princípio

**Decisão arquitetural aprovada não significa implementação concluída.** Cada feature avança pela cadeia: especificação → código → testes automáticos → validação manual/visual → evidência → aceita. Não declarar conformidade de Slint, GPU, acessibilidade ou export antes da execução correspondente.

O velho programa de CI/documentação tinha baselines egui+wgpu. A autoridade nesta branch é **Slint + OpenGL 3.3** com egui preservado externamente como contingência, sem dual frontend de produção.

## Matriz de testes

| Nível | Domínio | Evidência requerida |
|---|---|---|
| Unit pure | Geometry, UV, Camera math, Snap, Validation | determinismo, corner cases, input inválido |
| Command/Transaction | Model, Paint, Animation, Assets | preview/commit/cancel, Undo/Redo, dirty/revisions |
| Persistence | .petunia, recovery, assets | round-trip, migrations, atomic write, malformed data |
| Interchange | glTF/GLB, OBJ, textures | conformance, import/export fixtures, warnings |
| Security | Lua, MCP, paths, permissions | deny-by-default, stale ID, limits, rollback |
| UI/Interactions | Slint components, workspaces, viewport | gestures remapeados, states, focus, keyboard |
| Renderer | OpenGL viewport+Slint | native texture, resize/HiDPI, picking, state restoration |
| Accessibility | shell + workspaces + overlays | screen reader audit, keyboard-only, high contrast, IME |
| Performance | low-end Linux/Windows | frame times, GPU/CPU memory, idle scheduling, jobs |

Testes devem cobrir invariantes do domínio **antes** de snapshots cosméticos. Golden images são complemento, não garantia de usabilidade.

## Gates de cada mudança

1. Lint/format e build do menor conjunto relevante sem warnings novos.
2. Suite de teste unitário do domínio alterado.
3. Contract tests de Application boundary: UI não muta Geometry diretamente; Renderer não grava Document.
4. Teste de Undo/Redo, Cancel e ausência de side effects onde pertinente.
5. Document/recovery/export fixture quando feature altera schema.
6. Testes com erros/limites para dados externos e adapters.
7. Acessibilidade (keyboard, state announced, labels, focus, reduced motion) para UI.
8. Benchmark antes/depois com cenário, hardware e versão para alegações de performance.
9. Registro de status **not run / pass / fail / blocked** para cada gate; sem inferir pass por compilação.

Testes em CI Linux não substituem smoke manual em Windows e nem teste de mesa fraca.

## Frontend Slint — gates específicos

O workflow **UI Slint** existente verifica formatação do crate, tokens, Clippy e testes de interface/gestos. Comandos úteis:

- cargo fmt -p petunia_ui_slint -- --check
- cargo run -p xtask -- ui-lint
- cargo clippy -p petunia_ui_slint --all-targets -- -D warnings
- cargo test -p petunia_ui_slint --lib
- cargo test -p petunia_ui_slint --features animation-workspace --lib --test animate_shell --test viewport_gestures --test model_shell --test uv_shell

Os nomes/flags devem acompanhar os manifests reais. O workflow atualmente cobre paths específicos e não constitui aprovação universal do workspace. Expandir gates gradualmente após modularizar para evitar bloqueios de memória mascararem problemas.

**Checkpoint Q03 — 2026-10-09:** a extração MODEL adiciona `model_shell` e `uv_shell` ao comando de integração da CI Slint e inclui `refactor/architecture-foundation` no trigger de push. Evidência local e limites em [Slint Rescue — checkpoint MODEL](../09-ui/slint-rescue-plan.md#checkpoint-model-2026-10-09). YAML atualizado não equivale a execução de Actions: CI remota permanece `not run` até haver run/SHA. Cobertura ampla Core/Geometry/Project/security/GL/Windows e pinagem/toolchain permanecem checkpoints distintos.

### Slint Rescue Go/No-Go

Ver [Slint Reassessment](../09-ui/slint-reassessment.md). Não dar Go final enquanto faltarem:
- FBO/GL Texture compositado sem readback por frame;
- resize/HiDPI/picking e contexto único corretos;
- Split2 ativo/passivo sem budget regressivo;
- critical authoring flow funcional;
- teclado/F6/focus recovery/IME e screen reader tree;
- UI Scale 100–200%, High Contrast, Reduced Motion e hit targets adequados;
- perf low-end e Windows/Linux smoke.

Falha isolada de CSS visual ou código ainda monolítico não determina automaticamente No-Go; bloqueador estrutural comprovado após rescue sim.

## Gesto e keymaps

Cobrir:
- click, drag threshold, lasso/box, orbit/pan/zoom;
- remap de pointer, Maya Alt+LMB, Ctrl/Alt/Shift interpretados semanticamente;
- transform, gizmos, modal tool/latched gesture, click-move-click;
- Paint stroke begin/update/end, pointer Cancel, lost focus e Escape;
- coord. normalized/logical/physical em HiDPI e split;
- estado da ferramenta com overlays e screen reader feedback.

Testes não substituem inspeção do código para paths paralelos que contornem o dispatcher.

## Quality Gate de Geometry e Project

Cada operação topológica deve provar:
- validade de triangles/quads/n-gons e índices;
- FaceCorner UV, sharp normals, material slots e topology remapping;
- propriedade de Undo integral;
- cálculo determinístico salvo tolerâncias numéricas explícitas;
- ausência de panic com input ruim;
- equivalência de dados antes/depois do save/load.

Rust fuzz/property tests e sanitizer/Miri entram nos módulos seguros quando tecnicamente viáveis, não por imposição abstrata em todos os crates.

## Segurança e supply chain

- cargo-deny para advisories/licenses/bans/sources quando workflow habilitado.
- Permissões mínimas de CI e pinagem de versões conforme governança; não introduzir downloads runtime obrigatórios.
- Testes de input externo em project, images, SVG, OBJ/GLB e MCP requests.
- Secrets nunca em fixtures públicas ou logs.
- Interdependências de crates não podem inverter grafo acordado.
- Plugins e commands precisam de testes de capability, cancel e revision mismatch.

## Procedimento de regressão manual

Roteiro reproduzível por versão:
1. Abrir projeto limpo e legado.
2. Criar objeto → DRAW shape → Push/Pull → POLY edit → Undo/Redo.
3. Paint/UV, image reference e Project Photo smoke.
4. Salvar, fechar, reabrir; comparar IDs, aparência, layers e UV.
5. Animate procedural preview, pausas e parâmetros.
6. Trocar keymap, escala, temas, teclado only, focus entre F6 regions.
7. Split viewport, maximize/resize, sleep/wake e dispositivo gráfico quando aplicável.
8. Export/validation e reimport.
9. Executar low-end e Linux/Windows com logs e captures reais, não mockups.

## Observabilidade e report

Cada evidência registra: commit SHA, branch, toolchain, OS, GPU/driver, feature flags, command, resultado, run URL, limites e screenshot/log quando apropriado. Um gate pendente fica explicitamente pendente; "implementado" nunca é inferido pela existência de arquivo de teste.

## Decisões fechadas

Contratos testados por boundary; UI/GL/a11y hard gates; nenhuma aprovação sem evidência; rollback/cancel/Undo essenciais; fixtures cross-platform; performance medida; CI da branch e legado egui têm escopos separados.
