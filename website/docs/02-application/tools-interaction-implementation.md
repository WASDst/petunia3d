# Ferramentas compartilhadas — proposta e matriz de implementação

> **Status documental: proposta consolidada em 2026-10-15, a partir da análise solicitada pelo usuário.** Esta entrega documenta comportamentos e trabalhos futuros; não inicia sua implementação funcional. Qt e Slint permanecem fora do escopo de alteração. O estado de cada feature está obrigatoriamente preenchido na matriz e corresponde ao tracker.

## 1. Intenção e autoridade

Reduzir trocas repetitivas entre seleção e transformação, corrigir operações que atingem conteúdo indevido e tornar os atalhos previsíveis nos workspaces MODEL, PAINT, UV e ANIMATE.

Esta é a página de referência para a proposta transversal e seu plano de implementação. Os contratos de dados e algoritmos continuam nos capítulos de cada domínio. O [registro de decisões](../00-philosophy/decision-register.md) aponta para esta proposta sem declarar sua implementação ou aceitação de UX.

Os estados versionados têm uma única autoridade: `website/progress/tasks.json`, conforme o [contrato de acompanhamento](../17-reimplementation/index.md). A matriz abaixo é sua apresentação detalhada para as tarefas H01–H18. Não manter uma segunda interpretação de status.

**Escopo autorizado nesta entrega:** documentação, navegação, planejamento, status e validação de consistência. **Trabalhos funcionais futuros:** Core, Application, Config e módulos de ferramentas, na branch funcional autorizada. Não editar Qt/Slint, desenhar GUI, publicar o site, mudar formatos de projeto ou incorporar recursos adiados como requisito deste plano.

## 2. Fontes e baseline da análise

### Evidência local

- Auditoria: `ANALISE-FERRAMENTAS-2026-10-15.md`, na raiz do repositório.
- Reproduções e logs: `evidence/tools-audit-2026-10-15/{repro.rs,repro-results.txt,core-tests.txt,snapshot.json}`.
- Checkout auditado: `/mnt/sda3/petunia3d-qt-integration`, branch `codex/qt-integration`, HEAD `095ea728edef22e73c13bed53ad96e741178e999` **mais WIP local**. Os hashes no snapshot delimitam a evidência; o SHA sozinho não representa os arquivos exercitados.
- Resultado da auditoria anterior: **417 testes existentes pass**; harness sintético com **1 pass / 16 fail**. Há reproduções relacionadas à mesma causa e dois testes de cobertura de atalhos proposta; não são 16 bugs independentes.
- Ambiente daquela execução: Linux, Cargo/Rustc 1.99.0 de `/usr/bin`, `CARGO_BUILD_JOBS=1`; configuração declarada no repositório: Rust 1.98.1. Qt/Slint, eventos físicos de desktop, Windows e aceitação nativa: `not run`.

Estes resultados pertencem à auditoria anterior, não foram repetidos na entrega documental. Cada implementação futura deve reproduzir o caso no checkout atual, acrescentar a regressão permanente adequada e registrar nova evidência.

### Contratos internos

- [Application e Commands](./commands-events-services.md): intents, contexto restrito, single writer, validação, eventos e Undo.
- [Transforms, origin e pivot](../03-geometry/transforms-origin-pivot-gizmos.md): mundo/local, domínio de objeto/componente e manipulação.
- [PAINT — arquitetura](../06-paint/architecture-decision.md) e [UX](../06-paint/workspace-ux.md): layers, restrição de pintura e precedência do brush.
- [UV — arquitetura](../07-uv/architecture-decision.md) e [UX](../07-uv/workspace-ux.md): FaceCorner, Sync explícito, pins e transformação.
- [ANIMATE — UX](../10-animation/workspace-ux.md): procedural-first, preview, posing e progressão de capacidades.
- [Protocolo de implementação](../16-code-agents/implementation-protocol.md): evidência, gates, mudança mínima e handoff.

### Referências públicas de interação

| Referência                                                                                                                                                                                                                                                        | Comportamento que informa a proposta                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Blender — Tweak e seleção](https://docs.blender.org/manual/sl/4.5/interface/selecting.html)                                                                                                                                                                      | Tweak permite movimentar um item por arrasto; seleção por área e composição de seleção possuem gestos próprios.                                                               |
| [Cinema 4D — Modeling Axis](https://help.maxon.net/c4d/2026/en-us/Content/html/TOOLLIVESELECTION-MDATA_AXIS_GROUP.html)                                                                                                                                           | Seleção e transformação compartilham configuração de eixo e pivô.                                                                                                             |
| [Maya — Tool Box](https://help.autodesk.com/cloudhelp/2024/ENU/Maya-Basics/files/GUID-B345E162-0149-4E09-AC98-48DCFC227F33.htm)                                                                                                                                   | Q/W/E/R representam Select/Move/Rotate/Scale; perfis familiares precisam preservar a associação efetiva.                                                                      |
| [SketchUp — Move](https://help.sketchup.com/en/sketchup/moving-entities-around) e [Push/Pull](https://help.sketchup.com/es/sketchup/pushing-and-pulling-shapes-3d)                                                                                                | Referência espacial, inferências, precisão numérica e confirmação ajudam a operar diretamente sobre o alvo.                                                                   |
| [Substance Painter — ferramentas](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/painting/paint-tools/tool-list) e [Polygon Fill](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/painting/paint-tools/polygon-fill) | Picker temporário e distinção entre pintura, máscara, projeção e preenchimento por domínio. Polygon Fill pinta máscara; não equivale integralmente ao Fill de cor do Petunia. |
| [Blender — UV Selection](https://docs.blender.org/manual/zh-hans/4.5/editors/uv/selecting.html)                                                                                                                                                                   | Seleção UV diferencia corners/arestas/faces/ilhas e possui sincronização explícita com o 3D.                                                                                  |
| [Blender — keyframes](https://docs.blender.org/manual/id/4.5/animation/keyframes/editing.html) e [Maya — Dope Sheet](https://help.autodesk.com/cloudhelp/2026/ENU/Maya-Animation/files/GUID-231A20E8-5EBD-476D-88BC-704901A5E682.htm)                             | Autoria no frame e edição temporal exigem alvos explícitos para propriedades, chaves e regiões.                                                                               |

A comparação foi documental, sem inspeção de código externo e sem estudo com participantes. A combinação de seleção, movimento no alvo e box no vazio é uma proposta para o Petunia; não é cópia exata nem superioridade de UX comprovada.

## 3. Lacunas que orientam a execução

Os problemas de maior impacto são: exclusão de chaves vizinhas por tolerância fixa de 0.05 s; keying da transformação de repouso em vez da pose avaliada; operações atingindo clipes inativos; e Fill ampliando seleção vazia ou alvo inválido para a textura inteira.

Na configuração, o parser rejeita colchetes literais e perde Numpad+ no round-trip; bindings removidos reaparecem ao herdar defaults; conflitos internos aos perfis podem escolher ações pelo nome; e os dois detectores de conflitos divergem sobre contextos compartilhados. A Application ainda não executa todas as ações tratadas nos adapters.

Para interação, já existem `ToolSession`, `ModalOp`, projeção de transformações, seleção UV por corners, `StrokeSampler`, `StrokeBuffer`, dirty tiles, `AnimatePreview` e comandos transacionais. Faltam políticas e sessões compartilhadas que conectem essas capacidades de forma consistente. **Reutilizar e refatorar essas bases antes de considerar reescrita.**

## 4. Contrato proposto de seleção dinâmica

Select permanece a ferramenta persistente. Mover, girar e escalar podem ser operações temporárias sobre o alvo selecionado, preservando uma opção de transformação persistente conforme o perfil/preferência existente.

1. Clique seleciona sem transformar. Pressionar sobre item selecionado preserva a multisseleção até a classificação do gesto.
2. Arrastar item selecionado além do limiar move o conjunto; arrastar item não selecionado seleciona esse alvo e move somente ele. Cancelar restaura conteúdo e seleção anteriores.
3. Arrastar no vazio inicia seleção por área; a ação explícita Box Select força esse gesto inclusive sobre geometria.
4. Modificadores semânticos de estender/subtrair seleção prevalecem sobre movimento direto. Não fixar Ctrl/Shift na geometria ou duplicar o keymap.
5. Arrastar uma alça respeita eixo/plano, orientação e pivô. Movimento livre 3D usa plano da câmera passando pela âncora/pivô, com conversão mundo/local correta.
6. Usar o limiar já existente de 4 px lógicos, configurável em 2–16 px, para separar clique e arrasto.
7. Soltar confirma um único Undo de mutação; Esc cancela e restaura exatamente a fonte. G/R/S temporários retornam à ferramenta de origem depois de confirmar/cancelar, compatibilizando o perfil e a preferência de ativação.
8. Oferecer operação sem manter botão pressionado: iniciar por ação semântica/teclado, clicar/mover/clicar e digitar valores. Teclado, foco e constraints não podem depender de arrasto.
9. Preview usa uma fonte fixa, um alvo estável e revisão validada. Troca de documento/objeto, Undo/Redo ou edição concorrente não pode aplicar preview obsoleto sobre outro alvo.

### Limites por superfície

| Superfície       | Alvo e limite da operação                                                                                                                                                                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MODEL            | Objetos ou componentes conforme o domínio; não trocar domínio implicitamente. Object Mode altera Transform, Edit Mode altera geometria local conforme o contrato de transforms.                                                                                        |
| UV               | Corners/arestas/faces/ilhas em coordenadas UV. Não alterar posições 3D. Preservar Sync explícito e as regras distintas de pins em movimento manual e solvers.                                                                                                          |
| PAINT            | Select escolhe alvo/máscara; brush/eraser consomem o gesto de pintura. Somente decal explicitamente escolhido pode receber movimento de decal. Seleção de faces para máscara não move malha. Movimento de pixels requer contrato próprio, fora desta proposta inicial. |
| ANIMATE viewport | Manipular controle/osso como pose; distinguir repouso, pose avaliada, draft, clipe e layer. Não alterar a malha de repouso nem inserir chave automaticamente sem política explícita.                                                                                   |
| ANIMATE timeline | Seleção de chave, playhead, região temporal e osso são estados distintos. Mover uma chave altera seu tempo no clipe alvo; Delete não deve cair em exclusão geométrica.                                                                                                 |

## 5. Contratos de Paint, UV, Animation e atalhos

**Paint:** scope sem alvo válido devolve erro contextual ou no-change, sem promoção silenciosa para objeto inteiro. Todas as entradas de Fill devem respeitar a mesma restrição de faces, layer lock e transação. Picker temporário retorna à ferramenta anterior; Clone informa ausência de origem. Preservar a proteção atual por seleção parcial até decisão explícita sobre sua UX; motivo de bloqueio deve ser consultável no domínio.

**UV:** transformação contínua abre sessão com seleção e UVs originais fixadas; updates não geram novos checkpoints; confirmação produz um Undo e cancelamento restaura a fonte. Ações de seleção e G/R/S devem chegar à Application como intenções do domínio UV, com constraints e entrada numérica apropriadas. Unwrap/Pack/Density/Stitch/Relax continuam ações discretas quando não exigem gesto contínuo.

**Animation:** identificar clipe, skeleton, osso e chave explicitamente. Exclusão não usa janela fixa capaz de alcançar quadros vizinhos. Keying preserva a pose avaliada segundo a política autoral; se layers/procedural tornarem o resultado ambíguo, resolver ou rejeitar com motivo, nunca gravar repouso como substituto silencioso. IK manual precisa resolver a cadeia de fato; um toggle ou solver isolado não fecha essa feature.

**Atalhos:** parser e exportação devem ser reversíveis; desativação é persistente e distinta de ausência em override. Resolver o mapa efetivo dos perfis sem colisão arbitrária e usar a mesma regra de contexto nos detectores. Ações desconhecidas devem produzir resultado explícito. Prioridade proposta: edição de texto/valor ativo → sessão/modal ativo → superfície focada → workspace → ações compartilhadas. A integração futura deverá respeitar comandos de navegação e foco; não se introduz fallback físico como substituto de binding removido.

## 6. Matriz de implementação iterativa

**Status de implementação é obrigatório em todas as linhas.** Somente `TODO`, `IN PROGRESS` e `DONE` são permitidos. Esta coluna representa o desenvolvimento do delta delimitado da feature; a base de código existente é descrita separadamente e não transforma um TODO em feature ausente.

- `TODO`: desenvolvimento do delta ainda não iniciado, mesmo que a auditoria já exista ou haja base parcial reutilizável.
- `IN PROGRESS`: implementação iniciada, ou gate necessário ainda aberto. Bloqueios ficam na evidência com motivo e próximo passo; não criar um quarto status.
- `DONE`: todos os checkpoints do escopo concluídos com evidência e gates aplicáveis executados. Compilar, escrever testes ou registrar a proposta não basta.

Checkpoints e percentual permanecem no [painel de acompanhamento](#/progress). H01–H18 começam em `TODO` porque esta entrega é documental. Tarefas anteriores como A04, V01 ou N01 preservam seus históricos e escopos; os novos IDs acompanham os deltas desta auditoria, sem substituí-las ou declarar frontend aceito.

<!-- BEGIN TOOL IMPLEMENTATION MATRIX -->
| ID  | Prioridade | Feature / delta                                 | Status de implementação | Base atual / evidência                                        | Critério de aceite                                                                                                                                                                                                                                                                                                        |
| --- | ---------- | ----------------------------------------------- | ----------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| H01 | P1         | Keyframes com precisão por frame                | TODO                    | N-A; tolerância fixa remove vizinhas a 30 fps.                | Excluir apenas a chave alvo em 24/30/60 fps, preservando vizinhas e Undo/Redo. Definir identidade/precisão temporal sem tolerância fixa que alcance outro frame.                                                                                                                                                          |
| H02 | P1         | Keying da pose efetivamente avaliada            | TODO                    | N-B; inserção sem draft pode gravar repouso.                  | Preservar a pose avaliada ao inserir chave, com e sem draft. Resolver espaço autoral, procedural/layers e erro contextual sem fallback silencioso para repouso.                                                                                                                                                           |
| H03 | P1         | Autoria restrita ao clipe explícito             | TODO                    | N-C; seleção de active_clip não delimita todos os comandos.   | Inserir, excluir, interpolar e limpar somente no clip_id validado para skeleton/osso. Cobrir dois clipes do mesmo skeleton, alvo obsoleto, Undo/Redo e round-trip.                                                                                                                                                        |
| H04 | P1         | Fill com scope e alvo válidos                   | TODO                    | P-A; seleção vazia ou face inválida ampliam Fill.             | Seleção vazia e alvo inválido retornam erro/no-change sem pintar objeto inteiro. Cobrir scopes Face/SelectedFaces/Island/Object com semente válida e uma transação.                                                                                                                                                       |
| H05 | P1         | Restrição única em todas as entradas de Fill    | TODO                    | P-B; API compatível Fill ignora máscara.                      | Aplicar máscara e layer lock também na API compatível, sem divergência 2D/3D. Consolidar ou depreciar caminhos legados somente após cobrir callers e compatibilidade.                                                                                                                                                     |
| H06 | P2         | Parser e round-trip de atalhos                  | TODO                    | K-A/B; colchetes e Numpad+ falham no parser/retorno.          | Aceitar colchetes literais, Shift+colchetes e tecla Numpad+. Garantir parse/export/reload dos bindings e rejeição explícita de entradas inválidas.                                                                                                                                                                        |
| H07 | P2         | Persistência de atalhos desativados             | TODO                    | K-C; defaults restauram binding removido.                     | Distinguir binding desativado de ação ausente em override. Salvar/recarregar preservando desativação e compatibilidade de perfis antigos.                                                                                                                                                                                 |
| H08 | P2         | Perfis efetivos e conflitos coerentes           | TODO                    | K-D/E; Maya W colide e detectores divergem.                   | Resolver todos os perfis efetivos sem colisão arbitrária; preservar aliases semânticos legítimos. Unificar conflitos em global/view/window/workspace e provar Maya W e demais colisões auditadas.                                                                                                                         |
| H09 | P2         | Executor compartilhado de ações semânticas      | TODO                    | K-F; handlers diferem entre Application e adapters.           | Mapear ações de seleção, transformação e Paint por contratos compartilhados. Retornar resultado/erro para ação desconhecida e manter validação, permissões e Undo.                                                                                                                                                        |
| H10 | P2         | Seleção dinâmica e movimento direto MODEL       | TODO                    | ToolSession/ModalOp existem; política de Select ausente.      | Distinguir clique/arrasto/box; preservar multisseleção e modificadores semânticos. Validar alvo/revisão, limiar, mundo/local, cancelamento exato e Undo único sem mudar domínio.                                                                                                                                          |
| H11 | P2         | G/R/S temporários e operação sem arrasto        | TODO                    | Transformação existe; retorno temporário é proposta.          | Retornar à ferramenta de origem após confirmar/cancelar, compatível com perfil e preferência de ativação. Cobrir valores digitados, eixos/planos, snap e clique/mover/clique sem botão mantido.                                                                                                                           |
| H12 | P2         | Sessão contínua de transformação UV             | TODO                    | V01 parcial; comandos discretos e previews diretos existem.   | Fixar UVs/seleção/revisão de origem e atualizar preview sem checkpoints por evento. Commit único, Esc exato, pins/Sync preservados e nenhuma alteração de posições 3D.                                                                                                                                                    |
| H13 | P2         | Ações e atalhos contextuais UV                  | TODO                    | Sem binding padrão UV G/R/S; kernels reutilizáveis.           | Definir ações de seleção/move/rotate/scale UV com handlers na Application. Respeitar superfície focada, remapeamento, constraints e entrada numérica UV.                                                                                                                                                                  |
| H14 | P2         | Atalhos de transporte e chaves ANIMATE          | TODO                    | Space resolve para ferramenta anterior no keymap global.      | Criar ações de play/pause, passo/frame e autoria de chaves no contexto apropriado. Separar foco de timeline/viewport/texto e evitar Delete de geometria ou comandos globais indevidos.                                                                                                                                    |
| H15 | P2         | Posing e IK manual por intenção de alvo         | TODO                    | Solvers existem; toggle não comprova manipulação IK.          | Resolver cadeia ao mover controle/alvo IK com conversão mundo/local e alvo estável. Distinguir repouso/pose/draft, preview/commit/cancel e keying explícito sem mutação da malha de repouso.                                                                                                                              |
| H16 | P2         | Picker temporário e origem explícita de Clone   | TODO                    | Kernels Paint testados; lifecycle comum permanece parcial.    | Amostragem temporária retorna ao brush anterior sem criar Undo de pintura. Clone exige ou solicita origem de forma explícita e compartilha restrições do stroke.                                                                                                                                                          |
| H17 | P2         | Estado e diagnóstico das ferramentas Paint      | TODO                    | Máscara parcial pode bloquear mesmo com toggle desligado.     | Expor motivos tipados para falta de alvo/UV, máscara, layer lock e origem Clone. Preservar restrições atuais e transações de brush/formas/projeção/decal sem mover geometria por seleção de máscara.                                                                                                                      |
| H18 | P2         | Gramática consistente nas famílias de modelagem | IN PROGRESS                    | Tools/comandos/previews existem; aceite por família pendente. | Cobrir Extrude/Inset/Bevel/Push-Pull com precondições, preview, precisão e limites de topologia. Cobrir Primitives/DRAW/Poly Pen/Shape Builder e Knife/Slice/Loop Cut com fases claras, cancelamento e Undo. Manter Merge/Connect/Dissolve/Subdivide como ações contextuais quando discretas, com regressões por família. |
<!-- END TOOL IMPLEMENTATION MATRIX -->

### Atualização obrigatória e preservação do histórico

1. Ao iniciar uma feature, atualizar sua tarefa em `website/progress/tasks.json` para `IN PROGRESS` e a célula correspondente nesta matriz **no mesmo conjunto de alterações**.
2. Atualizar checkpoints, evidência, data e critério restante a cada entrega. Evidência registra revisão/WIP, ambiente, comandos e resultados `not run / pass / fail / blocked`.
3. Antes de usar `DONE`, verificar cada gate do escopo. Aceite do núcleo não representa aceite de frontend; gates futuros de integração permanecem separados.
4. Preservar IDs, critérios e evidências anteriores. Acrescentar etapas complementares quando necessário; não apagar linha concluída, reiniciar contagem ou reduzir escopo para melhorar percentual.
5. Regressão reabre a tarefa em `IN PROGRESS` e o checkpoint afetado, preservando evidência histórica e acrescentando a falha atual.
6. Executar `node website/scripts/verify-progress.cjs`: o verificador rejeita status vazio/inválido, status diferente do tracker, linha duplicada ou tarefa H sem linha correspondente. Não editar apenas a tabela ou apenas o JSON.

## 7. Sequência e gates de aceite

1. **Integridade primeiro:** H01–H05 corrigem alvo temporal, clipe, pose e scope de pintura.
2. **Ações previsíveis:** H06–H09 consolidam parser, persistência, perfis e executor semântico.
3. **Seleção direta:** H10–H11 reutilizam gesto/transação e cobrem multisseleção, box, precisão e cancelamento.
4. **Domínios específicos:** H12–H17 completam UV, transporte/posing e Paint sem mutação cruzada de domínio.
5. **Famílias de modelagem:** H18 aplica a gramática às capacidades já existentes, preservando limitações reais de topologia e comportamento.

Gates funcionais por tarefa:

- Regressões sintéticas no núcleo: 24/30/60 fps e chaves vizinhas; dois clipes do mesmo skeleton; pose avaliada antes/depois de keying; Fill com seleção vazia, alvo inválido, máscara e layer lock.
- Keymaps: colchetes e Numpad+, round-trip, binding removido, todos os perfis efetivos e colisões contextuais. IDs de ação não existentes hoje, como `uv.move`, precisam de contrato e handler, não apenas TOML.
- Gestos: fonte fixa, threshold, multisseleção, transformações de objeto/local, entradas não finitas, snap/constraints, stale target/revision, cancelamento e um Undo por gesto.
- Persistência/compatibilidade quando afetadas: Undo/Redo e save/load com mesmo conteúdo autoral; nenhuma migração de formato implícita.
- Teclado e acessibilidade: fluxo sem arrasto, valores digitados, composição de seleção e prioridades de foco. Aceite físico nos frontends permanece pendente de tarefa separada autorizada.

Executar apenas os testes afetados com os limites de jobs do projeto. Registrar comandos reais; não inventar um gate verde porque há código/teste. Quando uma implementação futura alcançar viewport/renderização, acrescentar evidência visual e de plataforma conforme o protocolo.

## 8. Verificação desta entrega documental — D03

Intenção: consolidar proposta, matriz e regra de atualização obrigatória. Fontes: auditoria local e contratos citados acima. Gap: as propostas existiam num relatório de auditoria, sem tarefas delimitadas e matriz canônica validada contra o tracker.

Changed: esta página, entradas de navegação, referências nos capítulos pertinentes, tarefas H01–H18 e D03, e verificação da coluna de status. Nenhum código de ferramenta, Qt ou Slint foi alterado.

| Gate documental                                                           | Resultado                                                             |
| ------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Sintaxe de `website/app.js`, `website/progress/progress.js` e verificador | pass                                                                  |
| `node website/scripts/verify-progress.cjs`                                | pass: 87 processos, 70 documentos e 18 linhas de matriz consistentes  |
| `node --test website/tests/progress.test.cjs`                             | pass: 6 testes, zero falhas                                           |
| `python3 website/scripts/verify-agent-docs.py`                            | pass: 70 rotas e catálogo/links de agentes consistentes               |
| Contraprovas: status vazio, divergência, duplicação e linha ausente       | pass: 6 rejeições esperadas, incluindo quarto status e coluna ausente |
| Links internos e diff dos arquivos desta entrega                          | pass: links relativos existentes e diff sem erros de whitespace       |
| Testes Rust, GUI nativa, a11y física, Windows e GPU                       | not run; nenhuma implementação funcional nesta entrega                |

Verificação documental em Linux, 2026-10-15, na branch/HEAD mais WIP citados na §2. Os 68 processos preexistentes e suas evidências foram preservados. Sem build Rust ou diretório efêmero de agente criado; disco com 40 GB livres (84%) e 18 GiB de RAM disponível, swap 3.3/4.0 GiB. Não houve publicação do site.

Risks: status requer atualização junto à implementação; o validador comprova consistência de registros, não funcionamento de feature. Dados da auditoria pertencem ao WIP e toolchain registrados. A UX proposta ainda precisa de validação contextual com uso real. Site não publicado; sem commit nesta branch de integração Qt.

Next checkpoint: D03 DONE encerra somente documentação e consistência, após os gates documentais acima. H01–H18 permanecem `TODO`. Iniciar futura implementação pela precisão temporal e pelo clipe explícito (H01–H03), na branch funcional autorizada, preservando a restrição atual de não alterar Qt/Slint.
