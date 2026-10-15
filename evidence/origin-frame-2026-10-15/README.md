# H19 — origem persistente / frame local

## Intent

Continuar o plano autorizado pelo núcleo, separando origem do objeto, centro de transformação e Cursor3D. Branch `refactor/architecture-foundation`; base `045f88576a792e3c823b6c0f1a5dd63024ef835c`. Sem alteração da GUI Qt/Slint, merge em main, push ou publicação.

## Sources

- `website/docs/03-geometry/transforms-origin-pivot-gizmos.md`: regra Object Mode / Component Edit e checkpoint H19.
- `website/docs/02-application/contextual-editor-contract.md`: origem local persistente, cópia explícita do cursor, centro/frame independentes.
- `crates/project/src/lib.rs`: `Asset::transform`, `world_matrix`, `world_mesh` e ausência atual de hierarquia.
- `crates/core/src/modal.rs`, `state.rs`, `command.rs`; intents existentes em `crates/editor/src/bridge/application.rs`.
- Tracker `website/progress/tasks.json`, matriz canônica de ferramentas; pacote fornecido em `newdocs/` preservado.

## Gap

| Responsabilidade                  | Baseline                 | Delta desta etapa                                                                     | Status de implementação |
| --------------------------------- | ------------------------ | ------------------------------------------------------------------------------------- | ----------------------- |
| Alteração explícita da origem     | DUPLICATED / BROKEN      | Rebase geometria/frame com preservação em mundo e transação única.                    | IN PROGRESS             |
| G/R/S Object Mode                 | FUNCTIONAL_BUT_DIFFERENT | Alterar o transform autoritativo em vez de assar posições locais.                     | IN PROGRESS             |
| Centros temporários e Cursor3D    | PARTIALLY_COMPLIANT      | Origem, bounding box, origens individuais e cópia de cursor independentes.            | IN PROGRESS             |
| Geradores, skin e offsets legados | PARTIALLY_COMPLIANT      | Rejeição explícita sem bake; preservação dos dados antigos, migração completa futura. | TODO                    |
| GUI, GPU físico e acessibilidade  | Não exercitados aqui     | Gates nativos separados da verificação do núcleo.                                     | not run                 |

## Changed

- `project::origin`: rebase validado antes de qualquer escrita; conservação da superfície em mundo, cópia/posicionamento por frame, classificação de compatibilidade e projeção de metadados.
- `core::origin_commands`: cinco presets registrados no dispatcher e comando explícito por UUID/revisão; wrappers existentes compartilham o mesmo escritor e Undo. Geometry usa todos os vértices; Selection recusa vazio; Bottom usa limites autorais em mundo; Cursor é cópia pontual.
- `core::modal`: G/R/S de objetos normalizados atualiza somente o TRS; arrasto de origem preserva a superfície e parte sempre do snapshot. Alvos e flags do gesto são congelados. Shear é rejeitado antes de publicar qualquer alvo.
- Compatibilidade: dados com offset legado ou TRS identidade e metadados assados preservam o caminho de vértices/metadados, incluindo o cache rígido quando aplicável. Reset legado de fonte procedural/rig é recusado sem conversão automática. Rebase explícito normaliza a origem estática; a migração de todos os produtores permanece aberta.
- Snap usa geometria e frame congelados. A superfície parada permanece como alvo ao editar a origem; outros objetos selecionados em movimento são excluídos. Dois testes do editor exercitam consultas reais de snap.
- Instanciação converge no comando registrado, sem checkpoint duplicado e sem congelar outro asset ativo. Separação herda o frame; prefabs calculam bounds em mundo, movem frames e recusam publicação parcial quando um resultado é inválido.
- Limpar TRS preserva geometria/fontes nos objetos normalizados. A decomposição TRS não usa a translação para tolerar shear; matrizes projetivas e rebases distantes que perdem precisão são recusados.
- Exemplo `crates/editor/examples/origin_frame_feedback.rs`: UiIntent de origem, CPU viewport e comparação de pixels. Os assets diagnósticos originais estão relacionados em `ATTRIBUTION.md`.

H19 permanece IN PROGRESS; os checkpoints verificados são complementares aos dois gates amplos ainda abertos. Nenhum campo novo foi adicionado ao formato persistente ou ao Cargo.lock.

## Verification

Todos os gates abaixo terminaram com exit code 0 nas fontes registradas em `source-snapshot.json`. Jobs limitados a `CARGO_BUILD_JOBS=1`, testes serializados.

| Gate                                  | Resultado | Prova / limite                                                                                                  |
| ------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------- |
| Core + Project + Editor               | pass      | `tests.txt`: 688 pass / 14 suítes, `petunia_editor/animation-workspace`; 20 cenários novos de origem/frame.     |
| Clippy all-targets, três crates       | pass      | `clippy.txt`, animation-workspace habilitado, `-D warnings`.                                                    |
| Cache rígido do renderer              | pass      | `rigid-cache-tests.txt`: 1 pass; extração CPU, sem contexto GL/GPU; Editor filtrado tem 0 testes executados.    |
| fmt                                   | pass      | `fmt.txt`: `cargo fmt -p petunia_core -p petunia_project -p petunia_editor -- --check`.                         |
| Diagnóstico de superfície/frame       | pass      | `visual.txt`, `pixels.json`, três PNGs inspecionados: rebase 0 pixels; Move 78.629 pixels alterados, 800 × 600. |
| Tracker/matriz e seus seis testes     | pass      | `progress.txt`, `progress-tests.txt`: 104 processos, 71 documentos, estados e referências consistentes.         |
| Links e catálogo de agentes           | pass      | `agent-docs.txt`: 71 rotas, 15 páginas de agentes; catálogo 39 / 189 / 20.                                      |
| Qt/GPU físico/Windows/DPI/leitor tela | not run   | Imagem Software3dViewport e teste de extração não estabelecem aceite nativo.                                    |
| Revisão independente                  | not run   | Revisão local do diff/contratos e gates automatizados; nenhum segundo agente ou revisor humano foi usado.       |

Comandos executados, sem somar repetições como testes únicos:

```bash
CARGO_BUILD_JOBS=1 cargo test -p petunia_core -p petunia_project -p petunia_editor --features petunia_editor/animation-workspace --lib --test origin_frame --test command_tests --test primitive_pipeline_tests --test prefab_tests --test project_service_tests --test uv_transform_session --test keyframe_integrity --test save_tree --test modifier_stack --test pipeline_tests --test contextual_interactions -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_project -p petunia_editor --features petunia_editor/animation-workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=1 cargo test -p petunia_render -p petunia_editor --features petunia_editor/animation-workspace --lib rigid_preview -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo run -p petunia_editor --features animation-workspace --example origin_frame_feedback -- evidence/origin-frame-2026-10-15
CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_project -p petunia_editor -- --check
node website/scripts/verify-progress.cjs
node --test website/tests/progress.test.cjs
python3 website/scripts/verify-agent-docs.py
```

A primeira execução ampla falhou por edições durante a compilação, misturando interfaces; não foi usada como gate. Um teste de instanciação que consultava o centro local foi corrigido para verificar origem em mundo e conservação da malha; comando e wrapper foram unificados. Houve ajustes de compilação/lint e uma compilação cancelada antes de corrigir o cache legado. As 688 verificações e os gates finais acima foram repetidos depois de congelar as fontes. Logs intermediários redundantes foram removidos.

Disco: 102 GiB livres / 58% no início, 92 GiB / 62% após builds; RAM disponível final 17 GiB, swap quase cheio. Por isso jobs=1 e test-threads=1. Nenhum `target/agent-*` foi criado ou retido; nenhum clean global. O pacote fornecido continua com os 20 hashes originais. `environment.txt` e `resources-final.txt` registram os limites; Qt/Slint, Cargo.toml e Cargo.lock não têm deltas desta etapa.

## Risks

`Asset.origin` e metadados assados continuam em caminho explícito de compatibilidade, não migração completa; dados antigos com cursor em mundo e transform não identidade não permitem inferência segura. Compensação de bind/animação, hierarquia, geradores/modifiers e todos os produtores legados não estão encerrados. Preservação de pose atual não promete preservar trajetórias completas. Em objetos normalizados, escala global que introduz shear é recusada. Edit Origin suporta Move; orientação da origem permanece pendente. Qt nativo, GPU físico, Windows, DPI e leitor de tela: not run.

## Next checkpoint

Concluir gates do frame estático, registrar limitações e manter checkpoints complementares no tracker. Depois: compensação de fontes autorais e migração legada; seleção heterogênea/frame de gizmo H22/H23 permanecem etapas próprias.
