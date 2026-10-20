# Primeiro lote funcional do editor contextual — 2026-10-15

## Intent

Executar o plano em entregas incrementais começando pelo núcleo, sem alterar Qt/Slint. Esta entrega encerra as correções H01–H07 e a documentação D04; não encerra todo o plano. Branch `refactor/architecture-foundation`, base `92c22caa23f96f85f18e72a5e962a1d8831694db`. A branch main não foi alterada nem houve push/deploy.

## Sources

- `AGENTS.md`, registro de decisões, manifesto e protocolo em `website/docs/`.
- `website/docs/02-application/{tools-interaction-implementation,contextual-editor-contract}.md` e tracker `website/progress/tasks.json`.
- Pacote do usuário `newdocs/petunia3d_agent_spec/`: 20 arquivos conferidos por SHA-256; cópia de seu manifesto em `spec-manifest.json`. SVGs de origem não foram importados como assets de produto.
- Auditoria histórica `evidence/tools-audit-2026-10-15/`, cuja revisão é anterior; não reutilizada como prova deste código. `source-snapshot.json` delimita os arquivos finais desta entrega.

## Gap

Clipes/chaves podiam atingir alvos errados, Fill podia ampliar o escopo, parser e persistência de bindings falhavam, handlers físicos sobrepunham perfis. DRAW restringia ferramentas/perfis; navegação consultava pivô/ferramenta; X-Ray alterava picking; Select não disponibilizava gizmo. ShapeBuilder sólido ainda não tinha separação de resultado vazio e partição preparada.

## Changed

- H01–H07 DONE: exclusão de um tempo de chave, autoria restrita a clipe validado, amostragem da pose avaliada e rejeição explícita de keying ambíguo por layers; Fill limitado e API compatível respeitando máscara/lock; parser, export/reload e tombstones `false` para bindings desativados.
- H08/H09 IN PROGRESS: conflitos e perfis efetivos corrigidos, falhas UnknownAction/Rejected explícitas e handlers de seleção/UV/Animate. Fallbacks físicos remanescentes continuam abertos.
- H10 IN PROGRESS: Select com gizmo mesh e arrasto protegido, box por gesto, cancel e modificadores. Perfis selecionados não habilitam gizmo sobre a mesh subjacente. H22/H23 ainda devem resolver alvos contextuais/heterogêneos e frame/gizmo completo.
- H12/H13 IN PROGRESS: UV preview fora do documento, fonte/seleção/revisão fixas, valores absolutos, um commit/Undo, cancel/Undo sem mutação, pins/Sync e stale guards. `UvModule` encaminha ao preview ou ao comando; editor projeta preview e recebe G/R/S/Enter/Esc.
- H14 IN PROGRESS: transporte/autoria Animate antes de Delete geométrico, taxa da timeline validada. Foco físico continua pendente.
- H20/H21 IN PROGRESS: navegação explícita e centro copiado do cursor uma vez; DRAW/POLY compartilham ferramentas/perfis e restauram câmera. H19, migração da origem persistente, continua TODO.
- H25 IN PROGRESS: BooleanSolid Empty/Mesh, operandos preparados, partição e componentes por aresta; sessão/publicação de regiões ainda ausente. H24/H26/H27 e demais recursos avançados continuam TODO.
- D04 DONE: contratos SEL/VSP/MF/RIG incorporados, matriz H19–H33 complementar, operação documentada sobre editor neutro/Qt atual, sem restaurar Slint.

## Verification

Linux x86_64; Rust/Cargo 1.99.0 instalado localmente (o manifest declara Rust 1.98.1). Builds com `CARGO_BUILD_JOBS=1`, testes com `--test-threads=1`. Nenhuma dependência nova nem alteração de Cargo.lock.

| Gate                                                   | Resultado                                 | Evidência                                                   |
| ------------------------------------------------------ | ----------------------------------------- | ----------------------------------------------------------- |
| Lote integrado, sete crates / 17 suítes                | pass — 776 testes                         | `final-tests.txt`                                           |
| Confirmação final Editor/Mesh/Paint / sete suítes      | pass — 522 testes; inclui casos repetidos | `final-focused-tests.txt`                                   |
| História Core/Editor após revisão de Undo/Redo         | pass — 342 testes, com repetições         | `history-tests.txt`                                         |
| Clique protegido em alça abaixo do limiar              | pass — regressão focada                   | `handle-click-test.txt`                                     |
| Seleção final, incluindo clique/área/alças/perfis      | pass — 8 regressões                       | `final-selection-tests.txt`                                 |
| Clippy sete crates, todos os targets, warnings negados | pass                                      | `clippy.txt`                                                |
| Formatação sete crates                                 | pass                                      | `fmt.txt`                                                   |
| Manifesto importado                                    | pass — 20 hashes                          | `spec-manifest.json`                                        |
| Matriz/tracker/documentos                              | pass — 104 processos / 71 documentos      | validação documental da entrega                             |
| Testes do painel                                       | pass — 6 testes                           | `website/tests/progress.test.cjs`                           |
| Projeção visual headless                               | pass — imagem gerada e inspecionada       | `select-source.png`, `select-gizmo.svg`, `select-gizmo.png` |
| Qt nativo / Windows / leitor de tela / DPI-GPU físico  | not run                                   | gate posterior H33                                          |

Não somar os lotes como testes únicos; há repetições. H03 foi reaberto na revisão de histórico, corrigido e validado com nova evidência antes de retornar a DONE. O segundo lote confirma alterações finais de alças/perfis e do backend booleano. A imagem combina um frame real do viewport CPU com os paths projetados pelo editor, rasterizados por librsvg; não é captura Qt.

Comandos executados com sucesso:

```bash
CARGO_BUILD_JOBS=1 cargo test --locked -p petunia_config -p petunia_core -p petunia_application -p petunia_editor -p petunia_module_paint -p petunia_module_uv -p petunia_mesh --features petunia_editor/animation-workspace --lib --test keymap_integrity --test keyframe_integrity --test uv_transform_session --test animate_session_tests --test rig_command_tests --test fill_integrity --test stroke_coverage --test contextual_interactions --test action_integrity --test solid_partition --quiet -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo test --locked -p petunia_editor -p petunia_mesh -p petunia_module_paint --features petunia_editor/animation-workspace --lib --test contextual_interactions --test solid_partition --test fill_integrity --test stroke_coverage -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo clippy --locked -p petunia_config -p petunia_core -p petunia_application -p petunia_editor -p petunia_module_paint -p petunia_module_uv -p petunia_mesh --features petunia_editor/animation-workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=1 cargo fmt -p petunia_config -p petunia_core -p petunia_application -p petunia_editor -p petunia_module_paint -p petunia_module_uv -p petunia_mesh -- --check
CARGO_BUILD_JOBS=1 cargo run --locked -p petunia_editor --example contextual_feedback --quiet -- evidence/contextual-tools-2026-10-15
rsvg-convert evidence/contextual-tools-2026-10-15/select-gizmo.svg -o evidence/contextual-tools-2026-10-15/select-gizmo.png
node website/scripts/verify-progress.cjs
node --test website/tests/progress.test.cjs
python3 website/scripts/verify-agent-docs.py
git diff --check
```

Os lints encontrados durante a implementação foram corrigidos, sem desativar regras ou reduzir asserções. Fixtures Paint receberam apenas ajustes mecânicos para Clippy. As compilações iniciais com erros de desenvolvimento não são gates pass; os logs acima registram os gates finais.

O caso final de história usa `CARGO_BUILD_JOBS=1 cargo test --locked -p petunia_core --test keyframe_integrity --test animate_session_tests --test uv_transform_session -p petunia_editor --features petunia_editor/animation-workspace --lib --test contextual_interactions -- --test-threads=1`; o teste de alça usa o filtro `clicking_a_select_handle_below_drag_threshold_keeps_selection` na lib do editor com `animation-workspace`.

Recursos: antes dos builds, 109 GB livres / 55% ocupado e 17 GiB disponíveis, swap 3.3/4.0 GiB. Depois, 102 GB livres / 58% ocupado e 18 GiB disponíveis; swap aproximadamente 4/4 GiB. O teto de um job foi mantido. Nenhum `target/agent-*` foi criado ou ficou para limpeza; nenhuma limpeza global do cache compartilhado foi executada.

## Risks

- Implementação integral do plano permanece aberta; status/checkpoints detalham o que falta. Não inferir recursos SEC/THI/ORG/Smart Rig da incorporação documental.
- `Asset.origin`, metadados legados e Object Mode ainda precisam da migração autoritativa de frame local, compensação de skin/dependências e política de animação (H19).
- Perfil/malha mistos, auto picking, ActiveElement, orientação/snap completos e integração física UV ainda exigem trabalho. Qt não recebeu novos controles de navegação/centro neste lote.
- A partição booleana não é a sessão ShapeBuilder sólida: faltam RegionPlan, hover/BVH, publicação UV/material, broad-phase, orçamento e cancelamento. Nenhuma receita persistente ou migração de formato foi introduzida.
- A revisão de diff foi local; revisão independente e plataformas nativas não executadas. CI antiga ainda pode referenciar o crate Slint removido pela baseline; não foi validada remotamente.

## Next checkpoint

Concluir H19 e os adapters/alvos H22/H23 antes de ampliar gestos mistos; continuar H24–H27 com operandos explícitos, sessão sólida e composição persistente. Manter integração visual e acessibilidade nativa em H33. Recuperar branch/HEAD/diff e confirmar os hashes/evidências antes de continuar. O pacote original `newdocs/` foi preservado sem modificações e permanece fora deste commit.
