# H19 — primitivas em frame local, 2026-10-15

## Intent

Continuar a separação entre origem persistente do objeto e Cursor3D na criação/regeneração de primitivas. Escopo Core/editor compartilhado, sem mudanças no frontend Qt ou Slint.

## Sources

Branch `refactor/architecture-foundation`, baseline `93c4ed782a64ae374bc8646c12635455c71d6f61`. Contratos: `website/docs/02-application/contextual-editor-contract.md` (Pivô do objeto, centro, Cursor3D e câmera), `03-geometry/transforms-origin-pivot-gizmos.md` (Object Mode transforma frame, Edit Mode transforma geometria), matriz H19 e tracker versionado.

Callers: `UiIntent::AddPrimitive` → comando registrado `model.add_*` → `CommandContext::begin_primitive`; adaptação numérica compartilhada → `update_active_primitive`; reedição durante criação → `update_primitive`. Nenhum widget ou algoritmo de GUI recebe regra de geometria.

## Gap

| Requisito / lacuna observada                               | Classificação inicial    | Plano mínimo / limite                                                                    | Status de implementação |
| ---------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------- | ----------------------- |
| Criação assava cursor nos vértices e Asset.origin          | FUNCTIONAL_BUT_DIFFERENT | Descritor local + translação do frame; remover cursor_offset da sessão.                  | DONE                    |
| Reedição podia apagar rotação/escala assadas               | BROKEN                   | Preservar TRS moderno; validar fonte legada; recusar reconstrução ambígua.               | DONE                    |
| Valores não finitos e falhas criavam histórico/contexto    | PARTIALLY_COMPLIANT      | Preparar/validar antes de checkpoint e mutação.                                          | DONE                    |
| Sessão podia continuar válida com outro objeto ativo       | BROKEN                   | Validar identidade ativa e fonte paramétrica; adapter e Cancel respeitam contexto atual. | DONE                    |
| Rebase de origem paramétrica/skin e normalização do legado | PARTIALLY_COMPLIANT      | Manter proteções; não inventar compensação ou converter para editável em silêncio.       | TODO                    |

## Changed

- A criação captura o cursor uma vez no TRS e mantém a malha do descritor em coordenadas locais. A sessão deixou de armazenar `cursor_offset`.
- Regeneração mantém UUID, origem local zero e TRS atual. Fontes legadas compatíveis conservam sua translação anterior; fontes assadas/livres incompatíveis são recusadas antes de trocar malha ou criar histórico.
- Parâmetros não finitos, frames inválidos, objetos bloqueados e rig/skin são recusados. Edição de parâmetros durante transformações é bloqueada para não ser sobrescrita pelo snapshot modal.
- A validade da criação exige alvo ativo/paramétrico e o checkpoint no topo. Adapter numérico e Cancel respeitam essa validade.
- Dez regressões Core e duas de adapter exercitam criação, trinta atualizações por espécie, G/R/S real, reedição, Undo/Redo/cancel e save/load nativo. Exemplo CPU compara reedição com malha local + TRS explícitos.

H19 permanece IN PROGRESS e preserva checkpoints anteriores. Nenhum campo novo de persistência, dependência ou alteração de Cargo.lock.

## Verification

Todos os gates Rust abaixo terminaram com exit code 0 nas fontes congeladas de `source-snapshot.json`. As três imagens foram inspecionadas: aumento da largura conserva orientação e colocação; imagem reeditada e referência coincidem.

| Gate                                     | Resultado | Evidência / limite                                                              |
| ---------------------------------------- | --------- | ------------------------------------------------------------------------------- |
| Core + Project + Editor                  | pass      | `tests.txt`: 700 testes / 16 suítes, animation-workspace; 12 regressões novas.  |
| Clippy all-targets, três crates          | pass      | `clippy.txt`, `-D warnings`, animation-workspace.                               |
| fmt                                      | pass      | `fmt.txt`, três crates.                                                         |
| Diagnóstico CPU                          | pass      | `visual.txt`, `pixels.json`: reedição 18.721 pixels; referência 0; 800 × 600.   |
| Tracker/matriz e seis testes             | pass      | `progress.txt`, `progress-tests.txt`.                                           |
| Documentos e catálogo de agentes         | pass      | `agent-docs.txt`.                                                               |
| Qt/GPU físico/Windows/DPI/leitor de tela | not run   | Imagens CPU não estabelecem aceite nativo.                                      |
| Revisão independente                     | not run   | Revisão local do diff e testes automatizados; nenhum segundo revisor utilizado. |

Jobs=1 e testes serializados devido ao swap quase cheio. As fontes Rust ficam congeladas durante cada execução de Cargo.

A primeira compilação focal expôs uso de um wrapper inexistente apenas no novo teste; o teste passou a acessar o CommandContext público. O cenário G/R/S inicialmente solicitava escala global não uniforme depois de rotação, introduzindo shear: a recusa era correta. O cenário usa escala antes de rotação para testar um TRS válido, mantendo escala negativa/não uniforme; a regressão anterior de recusa atômica de shear em `origin_frame` permanece. Uma primeira execução ampliada passou 700 testes; a revisão final alinhou Cancel à mesma validação de alvo ativo. A suíte ampliada foi repetida sobre esse ajuste: 700 pass em 16 suítes. Resultados intermediários não são o gate final.

Comandos executados (sem contar repetições como testes únicos):

```bash
CARGO_BUILD_JOBS=1 cargo test -p petunia_core -p petunia_project -p petunia_editor --features petunia_editor/animation-workspace --lib --test primitive_frame --test primitive_frame_bridge --test origin_frame --test command_tests --test primitive_pipeline_tests --test prefab_tests --test project_service_tests --test uv_transform_session --test keyframe_integrity --test save_tree --test modifier_stack --test pipeline_tests --test contextual_interactions -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_project -p petunia_editor --features petunia_editor/animation-workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_project -p petunia_editor -- --check
CARGO_BUILD_JOBS=1 cargo run -p petunia_editor --features animation-workspace --example primitive_frame_feedback -- evidence/primitive-frame-2026-10-15
node website/scripts/verify-progress.cjs
node --test website/tests/progress.test.cjs
python3 website/scripts/verify-agent-docs.py
```

Clippy aguardou um build Qt externo no mesmo target; o processo alheio não foi interrompido nem usado como evidência de aceite Qt. Nenhuma fonte Rust foi alterada durante os gates finais. Hashes em `source-snapshot.json`.

## Risks

Arquivos antigos com deformação assada não têm frame de geração inequívoco; o caminho seguro recusa regeneração destrutiva. A conversão explícita para editável mantém a superfície. Rebase de origem de fontes paramétricas, skin e hierarquia continua separado. Qt nativo, GPU físico, Windows, DPI, leitor de tela e revisão independente: not run.

## Next checkpoint

H19 mantém quatro de seis checkpoints comprovados; os dois gates amplos continuam abertos. Prosseguir com compensação de fontes autorais para rebase e alvos de perfil/mistos H22/H23, preservando este slice. Reedição ainda usa o adapter de compatibilidade Core; a migração de toda a API para intents tipadas/Application não é encerrada por estas regressões. Nenhum push, merge em main ou publicação de site.

Disco inicial 92 GiB livres / 62%; ver `environment.txt` e `resources-final.txt` para auditoria final. Nenhum target de agente criado/retido nem clean global. `newdocs/` permanece do usuário, com 20 hashes originais preservados. Qt/Slint, Cargo.toml e Cargo.lock não têm alterações desta etapa.
