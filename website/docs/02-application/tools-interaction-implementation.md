# Ferramentas compartilhadas — proposta e matriz de implementação

> **Status documental: proposta consolidada em 2026-10-15, a partir da análise solicitada pelo usuário.** Implementação incremental autorizada após consolidação do plano; evidências anteriores permanecem delimitadas por revisão. Qt e Slint permanecem fora do escopo de alteração. O estado de cada feature está obrigatoriamente preenchido na matriz e corresponde ao tracker.

## 1. Intenção e autoridade

Reduzir trocas repetitivas entre seleção e transformação, corrigir operações que atingem conteúdo indevido e tornar os atalhos previsíveis nos workspaces MODEL, PAINT, UV e ANIMATE.

Esta é a página de referência para a proposta transversal e seu plano de implementação. Os contratos de dados e algoritmos continuam nos capítulos de cada domínio. O [registro de decisões](../00-philosophy/decision-register.md) aponta para esta proposta sem declarar sua implementação ou aceitação de UX.

Os estados versionados têm uma única autoridade: `website/progress/tasks.json`, conforme o [contrato de acompanhamento](../17-reimplementation/index.md). A matriz abaixo é sua apresentação detalhada para as tarefas H01–H33. Não manter uma segunda interpretação de status.

**Escopo atual:** núcleo, editor compartilhado, Config e módulos de ferramentas na branch funcional; integração visual Qt permanece posterior. A [especificação contextual](./contextual-editor-contract.md) amplia o plano sem declarar recursos implementados. Alterações persistidas exigem migração e round-trip. Não publicar o site nem alterar Qt/Slint nesta etapa.

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
2. **SEL-004 — arrasto protegido:** arrastar geometria não inicia transformação. Mover usa alça explícita ou comando temporário; cancelar restaura a fonte. Esta decisão substitui o movimento direto proposto na entrega D03, cujo registro histórico é preservado.
3. Arrastar no vazio inicia seleção por área; a ação explícita Box Select força esse gesto inclusive sobre geometria.
4. Modificadores semânticos de estender/subtrair seleção mantêm sua função própria. Não fixar Ctrl/Shift na geometria ou duplicar o keymap.
5. Arrastar uma alça respeita eixo/plano, orientação e pivô. Movimento livre 3D usa plano da câmera passando pela âncora/pivô, com conversão mundo/local correta.
6. Usar o limiar já existente de 4 px lógicos, configurável em 2–16 px, para separar clique e arrasto.
7. Soltar confirma um único Undo de mutação; Esc cancela e restaura exatamente a fonte. G/R/S temporários retornam à ferramenta de origem depois de confirmar/cancelar, compatibilizando o perfil e a preferência de ativação.
8. Oferecer operação sem manter botão pressionado: iniciar por ação semântica/teclado, clicar/mover/clicar e digitar valores. Teclado, foco e constraints não podem depender de arrasto.
9. Preview usa uma fonte fixa, um alvo estável e revisão validada. Troca de documento/objeto, Undo/Redo ou edição concorrente não pode aplicar preview obsoleto sobre outro alvo.

### Limites por superfície

| Superfície       | Alvo e limite da operação                                                                                                                                                                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MODEL            | Seleção contextual identifica objetos/componentes, com filtros explícitos. Transformar objeto altera Transform; transformar componentes altera geometria local. Modos globais não são requisito do fluxo novo.                                                         |
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

Checkpoints e percentual permanecem no [painel de acompanhamento](#/progress). H01–H18 iniciaram em `TODO` na entrega documental D03. A execução atual atualiza somente os deltas iniciados; H19–H33 são complementares. Tarefas anteriores como A04, V01 ou N01 preservam seus históricos e escopos; os novos IDs acompanham os deltas desta auditoria, sem substituí-las ou declarar frontend aceito.

<!-- BEGIN TOOL IMPLEMENTATION MATRIX -->
| ID  | Prioridade | Feature / delta                                 | Status de implementação | Base atual / evidência                                        | Critério de aceite                                                                                                                                                                                                                                                                                                        |
| --- | ---------- | ----------------------------------------------- | ----------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| H01 | P1         | Keyframes com precisão por frame                | DONE                    | N-A; tolerância fixa remove vizinhas a 30 fps.                | Excluir apenas a chave alvo em 24/30/60 fps, preservando vizinhas e Undo/Redo. Definir identidade/precisão temporal sem tolerância fixa que alcance outro frame.                                                                                                                                                          |
| H02 | P1         | Keying da pose efetivamente avaliada            | DONE                    | N-B; inserção sem draft pode gravar repouso.                  | Preservar a pose avaliada ao inserir chave, com e sem draft. Resolver espaço autoral, procedural/layers e erro contextual sem fallback silencioso para repouso.                                                                                                                                                           |
| H03 | P1         | Autoria restrita ao clipe explícito             | DONE                    | N-C; seleção de active_clip não delimita todos os comandos.   | Inserir, excluir, interpolar e limpar somente no clip_id validado para skeleton/osso. Cobrir dois clipes do mesmo skeleton, alvo obsoleto, Undo/Redo e round-trip.                                                                                                                                                        |
| H04 | P1         | Fill com scope e alvo válidos                   | DONE                    | P-A; seleção vazia ou face inválida ampliam Fill.             | Seleção vazia e alvo inválido retornam erro/no-change sem pintar objeto inteiro. Cobrir scopes Face/SelectedFaces/Island/Object com semente válida e uma transação.                                                                                                                                                       |
| H05 | P1         | Restrição única em todas as entradas de Fill    | DONE                    | P-B; API compatível Fill ignora máscara.                      | Aplicar máscara e layer lock também na API compatível, sem divergência 2D/3D. Consolidar ou depreciar caminhos legados somente após cobrir callers e compatibilidade.                                                                                                                                                     |
| H06 | P2         | Parser e round-trip de atalhos                  | DONE                    | K-A/B; colchetes e Numpad+ falham no parser/retorno.          | Aceitar colchetes literais, Shift+colchetes e tecla Numpad+. Garantir parse/export/reload dos bindings e rejeição explícita de entradas inválidas.                                                                                                                                                                        |
| H07 | P2         | Persistência de atalhos desativados             | DONE                    | K-C; defaults restauram binding removido.                     | Distinguir binding desativado de ação ausente em override. Salvar/recarregar preservando desativação e compatibilidade de perfis antigos.                                                                                                                                                                                 |
| H08 | P2         | Perfis efetivos e conflitos coerentes           | IN PROGRESS             | K-D/E; Maya W colide e detectores divergem.                   | Resolver todos os perfis efetivos sem colisão arbitrária; preservar aliases semânticos legítimos. Unificar conflitos em global/view/window/workspace e provar Maya W e demais colisões auditadas.                                                                                                                         |
| H09 | P2         | Executor compartilhado de ações semânticas      | IN PROGRESS             | K-F; handlers diferem entre Application e adapters.           | Mapear ações de seleção, transformação e Paint por contratos compartilhados. Retornar resultado/erro para ação desconhecida e manter validação, permissões e Undo.                                                                                                                                                        |
| H10 | P2         | Seleção integrada e arrasto protegido MODEL     | IN PROGRESS             | ToolSession/ModalOp existem; política de Select ausente.      | SEL-004 substitui a proposta de movimento ao arrastar geometria: transformação por alça ou comando; vazio inicia box; fonte fixa, cancelamento e Undo único.                                                                                                                                                              |
| H11 | P2         | G/R/S temporários e operação sem arrasto        | TODO                    | Transformação existe; retorno temporário é proposta.          | Retornar à ferramenta de origem após confirmar/cancelar, compatível com perfil e preferência de ativação. Cobrir valores digitados, eixos/planos, snap e clique/mover/clique sem botão mantido.                                                                                                                           |
| H12 | P2         | Sessão contínua de transformação UV             | IN PROGRESS             | V01 parcial; comandos discretos e previews diretos existem.   | Fixar UVs/seleção/revisão de origem e atualizar preview sem checkpoints por evento. Commit único, Esc exato, pins/Sync preservados e nenhuma alteração de posições 3D.                                                                                                                                                    |
| H13 | P2         | Ações e atalhos contextuais UV                  | IN PROGRESS             | Sem binding padrão UV G/R/S; kernels reutilizáveis.           | Definir ações de seleção/move/rotate/scale UV com handlers na Application. Respeitar superfície focada, remapeamento, constraints e entrada numérica UV.                                                                                                                                                                  |
| H14 | P2         | Atalhos de transporte e chaves ANIMATE          | IN PROGRESS             | Space resolve para ferramenta anterior no keymap global.      | Criar ações de play/pause, passo/frame e autoria de chaves no contexto apropriado. Separar foco de timeline/viewport/texto e evitar Delete de geometria ou comandos globais indevidos.                                                                                                                                    |
| H15 | P2         | Posing e IK manual por intenção de alvo         | TODO                    | Solvers existem; toggle não comprova manipulação IK.          | Resolver cadeia ao mover controle/alvo IK com conversão mundo/local e alvo estável. Distinguir repouso/pose/draft, preview/commit/cancel e keying explícito sem mutação da malha de repouso.                                                                                                                              |
| H16 | P2         | Picker temporário e origem explícita de Clone   | TODO                    | Kernels Paint testados; lifecycle comum permanece parcial.    | Amostragem temporária retorna ao brush anterior sem criar Undo de pintura. Clone exige ou solicita origem de forma explícita e compartilha restrições do stroke.                                                                                                                                                          |
| H17 | P2         | Estado e diagnóstico das ferramentas Paint      | TODO                    | Máscara parcial pode bloquear mesmo com toggle desligado.     | Expor motivos tipados para falta de alvo/UV, máscara, layer lock e origem Clone. Preservar restrições atuais e transações de brush/formas/projeção/decal sem mover geometria por seleção de máscara.                                                                                                                      |
| H18 | P2         | Gramática consistente nas famílias de modelagem | TODO                    | Tools/comandos/previews existem; aceite por família pendente. | Cobrir Extrude/Inset/Bevel/Push-Pull com precondições, preview, precisão e limites de topologia. Cobrir Primitives/DRAW/Poly Pen/Shape Builder e Knife/Slice/Loop Cut com fases claras, cancelamento e Undo. Manter Merge/Connect/Dissolve/Subdivide como ações contextuais quando discretas, com regressões por família. |
| H19 | P2         | Pivô do objeto e centro de transformação        | IN PROGRESS             | 688 testes pass; rigs/geradores/legado completo pendentes.    | Origem local persistente, compensação da pose atual, centro temporário e conversão mundo/local explícitos.                                                                                                                                                                                                                |
| H20 | P2         | Cursor3D e navegação independentes              | IN PROGRESS             | Baseline 92c22caa; ver contrato contextual.                   | Modo explícito de navegação; pan e enquadramento não movem cursor; cópia única para centro ou pivô.                                                                                                                                                                                                                       |
| H21 | P2         | DRAW como preset de POLY                        | IN PROGRESS             | Baseline 92c22caa; ver contrato contextual.                   | Ferramentas e perfis compartilhados; vista planar ortográfica e retorno livre preservam documento e seleção.                                                                                                                                                                                                              |
| H22 | P2         | Seleção contextual e alvos heterogêneos         | IN PROGRESS             | Baseline 92c22caa; ver contrato contextual.                   | SEL-002/006: filtros, ActiveElement, resolução sem duplicidade e hierarquias sem dupla transformação.                                                                                                                                                                                                                     |
| H23 | P2         | Gizmo universal, orientação e snap              | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | SEL-001/003/005/007: centro, exibição e orientação independentes; frame congelado e snap contextual.                                                                                                                                                                                                                      |
| H24 | P2         | ShapeBuilder planar com operandos explícitos    | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | Regiões com furos, seleção de fontes explícita, preservação de fontes não afetadas e transação única.                                                                                                                                                                                                                     |
| H25 | P2         | ShapeBuilder sólido de dois operandos           | IN PROGRESS             | Baseline 92c22caa; ver contrato contextual.                   | Partição A-only/overlap/B-only, componentes por aresta, resultado vazio, Unite/Remove/Extract e proveniência UV/material.                                                                                                                                                                                                 |
| H26 | P2         | Composição reeditável e múltiplos sólidos       | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | Fontes estáveis e receita persistente; regiões transitórias, revisão, orçamento e cancelamento; sem bake silencioso.                                                                                                                                                                                                      |
| H27 | P2         | Construção sobre superfícies                    | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | Plano de face, imprint, furos e Push/Pull contextual; superfícies curvas como extensão posterior.                                                                                                                                                                                                                         |
| H28 | P2         | Feedback comum de operações                     | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | MF-001–008: preview de fonte fixa, ghost, parâmetros, revisão e incompatibilidades antes de confirmar.                                                                                                                                                                                                                    |
| H29 | P2         | Dimensões e comparação espacial                 | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | VSP-003/004: frames World/Local/Custom/Structural, source/evaluated e qualidade; escala dimensional separada de consulta.                                                                                                                                                                                                 |
| H30 | P2         | Section, Thickness e Organic Profiles           | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | VSP-005: seção geométrica separada de clipping/split, contornos abertos/furos, espessura direcional e estações manuais.                                                                                                                                                                                                   |
| H31 | P2         | Smart Rig manual e assistido                    | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | RIG-001/002/004/008/009/010/011: edição manual, preview revisável, bind transacional e pose explícita.                                                                                                                                                                                                                    |
| H32 | P2         | Smart Rig estrutural e Live opcional            | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | RIG-003/005/006/007/012: identidades estáveis, overrides, atualização localizada idempotente e desligar sem apagar rig.                                                                                                                                                                                                   |
| H33 | P2         | Integração visual e acessibilidade posterior    | TODO                    | Baseline 92c22caa; ver contrato contextual.                   | VSP-001/002/004, SEL-007: Clay, Companion View, gizmo, feedback acessível e gates Qt/GL/Windows/Linux.                                                                                                                                                                                                                    |
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

Changed: esta página, entradas de navegação, referências nos capítulos pertinentes, tarefas H01–H33 e D03, e verificação da coluna de status. Nenhum código de ferramenta, Qt ou Slint foi alterado.

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

Next checkpoint: D03 DONE encerra somente documentação e consistência, após os gates documentais acima. H01–H33 permanecem `TODO`. Iniciar futura implementação pela precisão temporal e pelo clipe explícito (H01–H03), na branch funcional autorizada, preservando a restrição atual de não alterar Qt/Slint.

## 9. Checkpoint funcional — 2026-10-15

**Intent:** iniciar o plano pelo núcleo/editor, preservando GUI Qt e histórico. **Sources:** baseline `92c22caa`, especificação incorporada no [contrato contextual](./contextual-editor-contract.md), auditoria anterior delimitada por revisão e código/callers atuais. **Gap:** as reproduções anteriores foram convertidas em regressões permanentes; H19–H33 ampliam o plano sem declarar features prontas.

**Changed:** H01–H07 DONE no escopo delimitado de correções do núcleo. H08/H09/H10/H12/H13/H14/H20/H21/H22/H25 continuam IN PROGRESS com checkpoints/evidência; demais etapas permanecem TODO. D04 DONE cobre documentação. Seleção permanece protegida; o gizmo novo só habilita alvos suportados, sem transformar a mesh subjacente quando o perfil está selecionado. Modos globais ainda são filtros legados: a seleção contextual/heterogênea completa é H22, não encerrada por mostrar o gizmo em Select.

**Verification:** Linux, Rust/Cargo 1.99.0, `CARGO_BUILD_JOBS=1`, testes serializados. Lote integrado: **776 pass** em 17 suítes. Após revisão e correções de validação, Editor/Mesh/Paint: **522 pass** em sete suítes; esses números incluem testes repetidos e não devem ser somados como casos únicos. Clippy `--all-targets -D warnings` e fmt dos sete crates afetados: **pass**. Manifesto do pacote: **20 hashes pass**. Logs, comandos, ambiente, hashes e limites ficam em `evidence/contextual-tools-2026-10-15/`.

**Risks:** Qt nativo, Windows, leitor de tela, DPI/GPU físicos e integração completa dos gestos UV: **not run**. Imagem do gizmo é evidência de projeção headless, não captura Qt. H19 continua TODO: `Asset.origin`, metadados legados e Object Mode ainda precisam da migração segura de frame local, skin e dependências; a separação de navegação não encerra isso. ShapeBuilder sólido ainda não tem sessão/publicação completa. Fallbacks físicos remanescentes exigem revisão de atalhos.

**Next checkpoint:** H19 (origem autoritativa/compensação e rejeição de fontes incompatíveis) e H22/H23 (alvos tipados de perfil/mistos e frame/gizmo), mantendo H24–H27 e Smart Rig em etapas próprias. Nenhum merge em main ou publicação do site.

### Revisão de histórico e alças

H03 foi reaberto durante a revisão: desfazer a criação do primeiro clipe deixava a sessão com referência obsoleta. A reconciliação agora remove somente essa referência em Undo/Redo/resolve, sem escolher outro clipe arbitrariamente. `history-tests.txt`: **342 pass**, incluindo sete regressões de keyframes, 20 de Animate e quatro de UV; o status DONE foi restaurado com evidência nova, preservando a anterior. A autoria direta com ID inválido continua rejeitada. Clique em alça abaixo do limiar também tem regressão: não seleciona a geometria por trás.


## Continuação H19 sobre 045f8857 — 2026-10-15

**Intent / Sources:** aplicar a origem persistente do [contrato contextual](./contextual-editor-contract.md) e do [contrato de transformações](../03-geometry/transforms-origin-pivot-gizmos.md) pelo núcleo/editor; Qt/Slint preservados.

**Gap / Changed:** o rebase estático move o frame e compensa posições locais, preservando a superfície atual. G/R/S e reset de objetos normalizados usam TRS; os dados assados mantêm compatibilidade explícita. Os comandos de origem e instanciação usam o dispatcher, com Undo único e proteção de fontes vivas. Snap congela geometria e frame; separação e prefabs conservam a colocação em mundo. Limites de precisão, shear, referência obsoleta, rig/paramétrico incompatível e falhas parciais são recusados.

**Verification:** 688 testes pass em 14 suítes com `petunia_editor/animation-workspace`, incluindo 20 cenários de origem/frame e dois de snap no editor. Relatório e gates complementares em `evidence/origin-frame-2026-10-15/README.md`. Os checkpoints complementam os dois gates amplos ainda abertos; H19 permanece **IN PROGRESS**.

**Risks / Next checkpoint:** completar compensação de fontes/rig, migração de produtores/formatos legados e edição de orientação da origem. O arrasto de origem atual suporta Move. Seleção heterogênea/frame de gizmo H22/H23 e integração/aceite nativo H33 continuam separados. Nenhuma compilação ou imagem CPU substitui gates de Qt/GPU físico, Windows ou leitor de tela.
