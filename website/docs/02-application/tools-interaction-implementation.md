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
| H19 | P2         | Pivô do objeto e centro de transformação        | IN PROGRESS             | 700 testes pass; primitivas locais; rig/legado pendentes.     | Origem local persistente, compensação da pose atual, centro temporário e conversão mundo/local explícitos.                                                                                                                                                                                                                |
| H20 | P2         | Cursor3D e navegação independentes              | DONE                    | NavigationAnchor, órbita/zoom ancorados e Undo isolado ok.    | Modo explícito de navegação; pan e enquadramento não movem cursor; cópia única para centro ou pivô.                                                                                                                                                                                                                       |
| H21 | P2         | DRAW como preset de POLY                        | DONE                    | Presets DRAW/POLY, projeção planar e restauração livre ok.    | Ferramentas e perfis compartilhados; vista planar ortográfica e retorno livre preservam documento e seleção.                                                                                                                                                                                                              |
| H22 | P2         | Seleção contextual e alvos heterogêneos         | DONE                    | SEL-002/006; filtros, resolução e Undo verificados.           | SEL-002/006: filtros, ActiveElement, resolução sem duplicidade e hierarquias sem dupla transformação.                                                                                                                                                                                                                     |
| H23 | P2         | Gizmo universal, orientação e snap              | DONE                    | SEL-001/003/005/007; frame congelado e orientações ok.       | SEL-001/003/005/007: centro, exibição e orientação independentes; frame congelado e snap contextual.                                                                                                                                                                                                                      |
| H24 | P2         | ShapeBuilder planar com operandos explícitos    | DONE                    | Operandos explícitos, furos e Undo testados no delta.         | Regiões com furos, seleção de fontes explícita, preservação de fontes não afetadas e transação única.                                                                                                                                                                                                                     |
| H25 | P2         | ShapeBuilder sólido de dois operandos           | DONE                    | Partição, RegionPlan, proveniência e Undo no delta.           | Partição A-only/overlap/B-only, componentes por aresta, resultado vazio, Unite/Remove/Extract e proveniência UV/material.                                                                                                                                                                                                 |
| H26 | P2         | Composição reeditável e múltiplos sólidos       | DONE                    | Receita persistente, fontes, orçamento e bake explícito ok.   | Fontes estáveis e receita persistente; regiões transitórias, revisão, orçamento e cancelamento; sem bake silencioso.                                                                                                                                                                                                      |
| H27 | P2         | Construção sobre superfícies                    | DONE                    | Plano de face, imprint e Push/Pull verificados.               | Plano de face, imprint, furos e Push/Pull contextual; superfícies curvas como extensão posterior.                                                                                                                                                                                                                         |
| H28 | P2         | Feedback comum de operações                     | DONE                    | Preview fonte fixa, ghost, limites MF-001–008 e guards ok.    | MF-001–008: preview de fonte fixa, ghost, parâmetros, revisão e incompatibilidades antes de confirmar.                                                                                                                                                                                                                    |
| H29 | P2         | Dimensões e comparação espacial                 | DONE                    | Frames, representação e consultas espaciais VSP-003/004.       | VSP-003/004: frames World/Local/Custom/Structural, source/evaluated e qualidade; escala dimensional separada de consulta.                                                                                                                                                                                                 |
| H30 | P2         | Section, Thickness e Organic Profiles           | DONE                    | VSP-005: Section, Thickness e Organic Profiles concluídos.     | VSP-005: seção geométrica separada de clipping/split, contornos abertos/furos, espessura direcional e estações manuais.                                                                                                                                                                                                   |
| H31 | P2         | Smart Rig manual e assistido                    | DONE                    | RIG-001/002/004/008/009/010/011 concluídos com testes.        | RIG-001/002/004/008/009/010/011: edição manual, preview revisável, bind transacional e pose explícita.                                                                                                                                                                                                                    |
| H32 | P2         | Smart Rig estrutural e Live opcional            | DONE                    | RIG-003/005/006/007/012 concluídos com testes.        | RIG-003/005/006/007/012: identidades estáveis, overrides, atualização localizada idempotente e desligar sem apagar rig.                                                                                                                                                                                                   |
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


## Continuação H19 sobre 93c4ed78 — primitivas locais (2026-10-15)

**Intent / Sources:** continuar o contrato de origem persistente na criação/reedição de primitivas, pelo Core/editor compartilhado. Qt e Slint preservados.

**Gap / Changed:** a fábrica deixava cursor nos vértices e no offset legado; a regeneração podia apagar pose assada. Novas primitivas passam a usar descritor local + TRS, sem `cursor_offset` na sessão. Reedição conserva frame e identidade, valida fonte e parâmetros antes do checkpoint e recusa edição assada/livre incompatível, rig e alvo bloqueado. Sessão obsoleta não redireciona o adapter numérico para a primitiva anterior nem permite cancelamento dessa criação por um alvo ativo diferente.

**Verification:** 700 testes pass em 16 suítes com `petunia_editor/animation-workspace`, incluindo dez regressões Core e duas de adapter; fmt e Clippy all-targets `-D warnings` pass. Três imagens Software3dViewport inspecionadas: reedição altera 18.721 pixels e coincide com referência local + TRS (0 pixels de diferença). Relatório, comandos e hashes: `evidence/primitive-frame-2026-10-15/README.md`. O checkpoint complementar mantém H19 **IN PROGRESS**, sem descartar as evidências estáticas anteriores.

**Risks / Next checkpoint:** compensação autoral para rebase paramétrico/skin, orientação da origem e migração do legado ainda pendentes; alvos de perfil/mistos H22/H23 e aceite nativo H33 continuam separados. Imagens CPU não estabelecem aceite Qt/GPU físico, Windows, DPI ou leitor de tela.


## Continuação H22 — Seleção contextual e alvos heterogêneos (SEL-002, SEL-006) (2026-10-15)

**Intent / Sources:** Implementar seleção contextual e alvos heterogêneos (`SelectionRef`, `SelectionFilters`, `ActiveElement`, `SelectionResolver`) no Core e Editor, com resolução sem duplicidade de vértices compartilhados, dominância de objetos sobre subcomponentes, filtragem de raízes em árvores hierárquicas com proteção contra ciclos, exclusão de alvos bloqueados (`locked_targets`) e picking contextual com prioridade tipada.

**Gap / Changed:**
- `crates/core/src/selection.rs`: tipos canônicos `SelectionRef` (`Object`, `Vertex`, `Edge`, `Face`, `Shape`), `SelectionFilters`, `ActiveElement` com tempo/ordem de seleção, e `SelectionResolver` executando deduplicação e dominância em O(n).
- `crates/editor/src/bridge/picking.rs`: picking contextual resolvendo subcomponentes por prioridade tipada (Vertex -> Edge -> Face -> Object) quando filtros estão ativos.
- `crates/core/src/state.rs`: `sync_selection` bidirecional reconciliando itens contextuais, malha ativa e limpando referências obsoletas em Undo/Redo.

**Verification:**
- `crates/core/tests/contextual_selection.rs`: 10 passed; 0 failed.
- `crates/editor/tests/contextual_interactions.rs`: 9 passed; 0 failed.
- `crates/core/src/selection.rs`: 8 unitários passed; 0 failed.
- Clippy e fmt pass.


## Continuação H23 — Gizmo universal, orientação e snap (SEL-001, SEL-003, SEL-005, SEL-007) (2026-10-15)

**Intent / Sources:** Implementar gizmo de transformação universal com separação estrita entre centro de transformação (pivô) e frame de orientação, orientações Global, Local, View, Custom, Normal e Auto, congelamento do frame de orientação durante gestos modais (`ModalOp`), restrições axiais/planares projetadas no frame sem distorção e snap contextual.

**Gap / Changed:**
- `crates/core/src/state.rs`:
  - `TransformFrame`: struct ortonormal (`x_axis`, `y_axis`, `z_axis`) com construtores `global()`, `from_rotation()`, `from_z_axis()`, `from_edge()`, `from_view()`, e métodos `axis(index)` e `plane_normal(index)`.
  - `TransformOrientation`: enum estendido com `Normal`, `View`, `Auto`, `Custom`.
  - `EditorView::calculate_transform_frame`: resolução das orientações canônicas. `Auto` resolve a partir do `ActiveElement` (Face normal, Edge tangente, Vertex normal, Object rotação) ou seleção ativa da malha, com fallback limpo.
  - Sincronização de `custom_transform_frame` entre `EditorSession` e `ViewportSession`.
  - Fallback seguro de `PivotPoint::Cursor3D` para `cursor_3d` quando `transform_center` não estiver inicializado.
- `crates/core/src/modal.rs`:
  - `ModalOp` armazena `frame: TransformFrame` congelado em `begin_modal`, evitando descontinuidades de eixos durante rotação de câmera ou troca de orientação no meio do gesto.
  - Projeções de constraints axiais (`ModalConstraint::Axis(i)`) e planares (`ModalConstraint::Plane(i)`) calculadas com `modal.frame.axis(i)` e produto escalar.
  - Escala restrita ao longo de eixos e planos arbitrários sem cisalhamento.
- `crates/editor/src/projection.rs`:
  - `compute_gizmo` projeta hastes, anéis de rotação e planos alinhados ao `frame` ativo ou congelado.
- `crates/editor/src/bridge/transform.rs`:
  - Arrasto de gizmo e constraints usam `modal.frame.axis(index)`.
- `crates/core/src/command.rs`, `material_commands.rs`, `editor/bridge`:
  - `SetAssetTransformCmd` (`model.set_transform` / `asset.set_transform`) e `SetMaterialPropertiesCmd` (`material.set_properties`) transacionais com checagem de floats finitos, Undo/Redo e integração `UiIntent`.
- `crates/core/tests/universal_gizmo.rs`:
  - 5 testes de integração dedicados cobrindo construtores de frame, independência pivô/orientação, frame customizado, resolução Auto e congelamento durante gestos modais.

**Verification:**
- `crates/core/tests/universal_gizmo.rs`: 5 passed; 0 failed.
- `crates/core --lib`: 186 passed; 0 failed.
- `crates/editor --features animation-workspace --lib`: 126 passed; 0 failed.
- `crates/core --test contextual_selection`: 10 passed; 0 failed.
- `crates/editor --test contextual_interactions`: 9 passed; 0 failed.
- `crates/core --test command_tests`: 60 passed; 0 failed.
- `crates/project --lib`: 244 passed; 0 failed.
- `cargo fmt -p petunia_core -p petunia_editor -- --check`: passed.
- `cargo clippy -p petunia_core -p petunia_editor --all-targets -- -D warnings`: passed.
- Scripts de validação de progresso e documentação: passed.

**Risks / Next checkpoint:**
- Projeção de gizmo verificada matematicamente e headless; apresentação nativa na viewport Qt permanece sob Task H33.


## Continuação H20 — Cursor3D e navegação independentes (2026-10-15)

**Intent / Sources:** Desacoplar rigorosamente o posicionamento do 3D Cursor das operações de navegação de câmera (órbita, pan, zoom, enquadramento), garantindo ancoragem explícita (`NavigationAnchor`), cópia isolada de centro de transformação e proteção total contra mutação do documento ou poluição do histórico de Undo.

**Gap / Changed:**
- `crates/core/src/camera.rs`, `state.rs`: `NavigationAnchor` (`Default`, `Cursor3D`) define explicitamente o ponto de pivô da câmera sem acoplamento a ferramentas ativas ou pivôs de transformação (`PivotPoint`).
- `crates/editor/src/bridge/viewport.rs`: gestos de navegação (`apply_viewport_gesture`, `orbit_viewport`, `pan`, `zoom`) orbitam e aproximam em torno do cursor 3D quando ancorado, e nunca alteram a posição do `cursor_3d`.
- `crates/editor/src/bridge/pivot.rs`: `place_cursor_3d` valida coordenadas, rejeita `NaN`/`Inf` com segurança e faz raycast/snapping magnético; `copy_cursor_to_transform_center` congela o centro de transformação separadamente do cursor.
- `crates/editor/tests/contextual_interactions.rs`: teste dedicado `navigation_decoupling_pan_frame_and_history_invariants_h20` provando que pan, zoom e órbita preservam `cursor_3d`, troca de âncora não move a câmera nem suja o documento, e operações de visualização não criam entradas de Undo.

**Verification:**
- `crates/editor/tests/contextual_interactions.rs`: 11 passed; 0 failed.
- `crates/core --lib`: 186 passed; 0 failed.
- `crates/editor --features animation-workspace --lib`: 126 passed; 0 failed.
- Clippy e fmt pass.


## Continuação H21 — DRAW como preset de POLY (2026-10-15)

**Intent / Sources:** Implementar `ModelingMode::Draw` e `ModelingMode::Poly` como presets de apresentação sobre o mesmo workspace unificado `Model`, compartilhando ferramentas, perfis, picking e extensões sem conversões destrutivas ou perda de seleção ao alternar modos.

**Gap / Changed:**
- `crates/editor/src/lib.rs`: `ModelingMode` com presets `Draw` e `Poly`. Ambos compartilham o mesmo catálogo (`offers_tool` retorna `true`). `Draw` utiliza `EdgeMode::Features` e `Poly` utiliza `EdgeMode::Topology`.
- `crates/editor/src/bridge/application.rs`: `UiIntent::SetModelingMode` alterna entre `Draw` e `Poly` sem alterar a seleção ativa ou sujar o documento de projeto. Ao entrar em DRAW com workplane ativo, a câmera alinha-se ortograficamente.
- `crates/editor/src/bridge/viewport.rs`: em modo DRAW planar, o gesto de órbita livre é redirecionado suavemente para pan planar, garantindo que o gesto do usuário nunca fique inoperante. Ao retornar para POLY, a projeção perspectiva livre e orientação original são restauradas sem qualquer mutação de documento ou criação de passos de Undo.
- `crates/editor/tests/contextual_interactions.rs`: teste dedicado `draw_poly_preset_switching_and_roundtrip_invariants_h21` provando que o chaveamento preserva seleção, malhas ativas, topologia e integridade do histórico de Undo.

**Verification:**
- `crates/editor/tests/contextual_interactions.rs`: 11 passed; 0 failed.
- `crates/editor --lib`: 126 passed; 0 failed.
- Clippy e fmt pass.

**Risks / Next checkpoint:**
- Integração de UI nativa Qt para os botões DRAW/POLY e Cursor3D verificada em paridade de bridge; gates de apresentação visual e eventos de viewport permanecem sob H33.
- Próximos checkpoints: H24 (ShapeBuilder planar com operandos explícitos) e H25 (ShapeBuilder sólido).


## Continuação H24 — ShapeBuilder planar com operandos explícitos (2026-10-15)

**Intent / Sources:** Implementar suporte a operandos explícitos e preservação de regiões com furos no ShapeBuilder planar (`ShapeEditCmd` e `PathfinderOp`), garantindo que perfis e linhas de construção não envolvidos permaneçam intactos no plano, operando sob transação única atômica com suporte integral a Undo/Redo.

**Gap / Changed:**
- `crates/core/src/region_push.rs`: derivado `Clone` em `ProfileGroup`; adicionado `filter_by_operands(&self, operands: &[uuid::Uuid]) -> Self` preservando indexação paralela de `profiles`, `lines` e `shapes` coplanares para perfis selecionados.
- `crates/core/src/context_api.rs`: adicionado `apply_shape_edit_with_operands` delegando ao `CommandContext`.
- `crates/core/src/shape_builder.rs`:
  - Campo `pub operands: Option<Vec<uuid::Uuid>>` em `ShapeEditCmd`, construtores `new`, `with_plane`, `with_operands`, `pathfinder`, `pathfinder_with_operands`.
  - Validação estrita em `Command::can_execute` rejeitando listas vazias ou operações de Pathfinder com menos de 2 operandos sem entrar em pânico.
  - `apply_shape_edit` filtra `ProfileGroup` pelos operandos indicados; formas fechadas não participantes e linhas de construção abertas fora da união são preservadas sem alteração de ID, nome ou atributos.
  - Preservação estrita de furos internos (`profile.holes` ↔ `Region2.holes`).
  - 13 testes unitários em `crates/core/src/shape_builder.rs`, incluindo cobertura de operandos explícitos, preservação de fontes não afetadas, validação de erros e furos.
- `crates/editor/src/shape_builder_tool.rs`: atualizadas invocações de `ShapeEditCmd` para explicitar `operands: None`.
- `crates/editor/tests/contextual_interactions.rs`: teste de integração dedicado `shape_builder_explicit_operands_and_hole_preservation_invariants_h24` provando preservação de formas e furos em fusão parcial e restauração atômica em único passo de Undo.

**Verification:**
- `crates/core/src/shape_builder.rs`: 13 passed; 0 failed.
- `crates/core --lib`: 189 passed; 0 failed.
- `crates/editor/tests/contextual_interactions.rs`: 12 passed; 0 failed.
- `crates/editor --lib`: 126 passed; 0 failed.
- Clippy e fmt pass.

**Risks / Next checkpoint:**
- Próximo checkpoint: H25 (ShapeBuilder sólido de dois operandos: BooleanSolid, partição A-only/overlap/B-only, resultado vazio e proveniência UV/material).


## Continuação H25 — ShapeBuilder sólido de dois operandos (2026-10-15)

**Intent / Sources:** Implementar o ShapeBuilder sólido de dois operandos (`A` e `B`) sobre o kernel booleano e decomposição em componentes por aresta (`crates/mesh/src/boolean_partition.rs`), estruturando o plano de ownership transitório (`RegionPlan`), proveniência de faces, UV e materiais (`crates/mesh/src/boolean_cleanup.rs`), preservação de objetos intocados e integração atômica no despachante de comandos (`SolidShapeEditCmd`).

**Gap / Changed:**
- `crates/mesh/src/solid_builder.rs`:
  - `SolidRegionProvenance` (`FirstOnly`, `Overlap`, `SecondOnly`) e `SolidRegion` com bounding boxes finitos e procedência estrita.
  - `RegionGroupOrigin` (`FirstOperand`, `SecondOperand`, `Created`) e `CommittedObject` (`PreserveFirst`, `PreserveSecond`, `Empty`, `Mesh`).
  - `RegionPlan`: `build` decompondo operandos e componentes conexos por aresta, `remove`, `extract`, `unite`, `is_first_untouched`, `is_second_untouched` e `commit` com fast-path exato para união global e objetos intocados.
- `crates/mesh/src/boolean_cleanup.rs`:
  - `SourceIndex::from_operands` calculando diagonal e tolerância relativa automaticamente.
  - `annotate_mesh_provenance` transferindo coordenadas de textura UV e slots de material para cada componente decomposto.
- `crates/core/src/solid_commands.rs`:
  - `SolidOperation` (`UniteAll`, `RemoveOverlap`, `ExtractOverlap`).
  - `SolidShapeEditCmd` com validação estrita em `can_execute` (rejeitando ausência de operando e auto-referência sem panics).
- `crates/core/src/state.rs`:
  - `apply_solid_shape_builder`: avaliação em coordenadas locais, decomposição, aplicação sobre o `RegionPlan`, commit seguro, criação de novos assets para extração, preservação de objetos intocados e mutação atômica protegida por checkpoint de Undo.
- `crates/core/src/command.rs`: registro de metadados canônicos para `model.shape_builder_solid`, `model.solid_unite`, `model.solid_remove_overlap` e `model.solid_extract_overlap`, além de aliases.
- `crates/editor/tests/contextual_interactions.rs`:
  - Teste de integração `solid_shape_builder_two_operands_and_undo_invariants_h25` validando extração de overlap para novo asset, remoção de overlap com preservação de material slots, união sólida, preservação de objeto não-participante e restauração via Undo em passos atômicos.
- `crates/core/tests/command_tests.rs`:
  - Teste `test_solid_shape_builder_commands_and_undo_redo` validando o ciclo completo no despachante com Undo/Redo roundtrip.
- `crates/mesh/tests/solid_builder_tests.rs`:
  - 10 testes unitários cobrindo partição de cubos, cubos disjuntos, cubos idênticos, cubos contidos, integridade de slots de material e UVs, e rejeição de malhas abertas ou inválidas sem panics.

**Verification:**
- `crates/mesh/tests/solid_builder_tests.rs`: 10 passed; 0 failed.
- `crates/mesh/tests/solid_partition.rs`: 4 passed; 0 failed.
- `crates/mesh --lib`: 261 passed; 0 failed.
- `crates/core/tests/command_tests.rs`: 61 passed; 0 failed.
- `crates/core --lib`: 189 passed; 0 failed.
- `crates/editor/tests/contextual_interactions.rs`: 13 passed; 0 failed.
- `crates/editor --lib`: 126 passed; 0 failed.
- Clippy e fmt pass em todos os alvos afetados.

**Risks / Next checkpoint:**
- Checkpoint concluído: H25.

---

## Continuação H26 — Composição reeditável e múltiplos sólidos (2026-10-16)

**Intent / Sources:** Implementar a composição reeditável e avaliação sequencial de múltiplos sólidos com receitas persistentes (`SolidCompositionRecipe`, schema v1), fontes estáveis (`SolidSource`), passos ativáveis e reordenáveis (`SolidCompositionStep`), limites de orçamento paramétrico (`ShapeBuilderBudget`), detecção automática de invalidação topológica (`CompositionStatus::NeedsEvaluation` / `Invalidated`), garantia estrita de "Sem bake silencioso", e comandos transacionais sob Undo/Redo no despachante (`CreateSolidCompositionCmd`, `BakeSolidCompositionCmd`, `ToggleSolidCompositionStepCmd`, `ReorderSolidCompositionStepCmd`).

**Gap / Changed:**
- `crates/mesh/src/solid_builder.rs`:
  - `ShapeBuilderBudget`: limites configuráveis de `max_input_triangles`, `max_decomposed_triangles`, `max_components` e `max_depth` com métodos `check_input`, `check_decomposed` e `check_depth`.
  - `RegionPlan::build_with_budget(a, b, budget)` integrando checagem estrita de complexidade antes e após decomposição booleana.
  - `evaluate_multi_solid`: avaliador com auto-triangulação intermediária (garantindo compatibilidade com kernels booleanos de 3 vértices por face), anotação de proveniência cumulativa e restrições de orçamento a cada profundidade da composição.
- `crates/project/src/solid_composition.rs`:
  - `SOLID_COMPOSITION_SCHEMA_VERSION = 1`.
  - `SolidSource` (`id`, `name`, `topology_hash`), `SolidCompositionStep` (`id`, `operand_id`, `operation`, `enabled`), `CompositionStatus` (`Valid`, `NeedsEvaluation`, `Invalidated { reason }`).
  - `SolidCompositionRecipe`: estrutura serializável no projeto Petunia3D com `sources`, `steps`, `cached_mesh`, `budget` e `status`. Métodos: `add_step`, `toggle_step`, `reorder_step`, `evaluate`, `bake`, `check_invalidation`.
- `crates/project/src/lib.rs`:
  - Adicionado `composition: Option<SolidCompositionRecipe>` na struct `Asset`.
  - `Asset::is_composition()` e `Asset::bake_composition()` ("Sem bake silencioso" — consome a receita e materializa a malha estática).
  - `Asset::source_mesh_hash(&self) -> u64` tornado público para fingerprinting topológico rápido.
- `crates/core/src/solid_commands.rs`:
  - `CreateSolidCompositionCmd`: cria a composição a partir do asset base e operandos secundários; valida auto-referência e existência dos ativos.
  - `BakeSolidCompositionCmd`: converte explicitamente a receita em malha estática B-Rep editável, consumindo a receita.
  - `ToggleSolidCompositionStepCmd`: ativa/desativa um passo específico da receita e dispara reavaliação.
  - `ReorderSolidCompositionStepCmd`: reordena os passos da receita mantendo determinismo.
- `crates/core/src/state.rs`:
  - Métodos transacionais em `CommandContext`: `create_solid_composition`, `bake_solid_composition`, `toggle_solid_composition_step`, `reorder_solid_composition_steps` com emissão de eventos e marcação dirty.
  - Exposição de métodos equivalentes em `AppState`.
- `crates/core/src/command.rs`:
  - Registro canônico dos comandos: `model.solid_composition_create`, `model.solid_composition_bake`, `model.solid_composition_toggle_step`, `model.solid_composition_reorder_step` e seus aliases.
- Testes e Validação:
  - `crates/mesh/tests/solid_builder_tests.rs`: 12 testes (adicionados testes de budget e evaluate_multi_solid).
  - `crates/project/tests/solid_composition_tests.rs`: 7 testes cobrindo encadeamento de 3 cubos, toggle/reorder, invalidação por mutação geométrica e por remoção de operando, håndling de budget e bake explícito.
  - `crates/core/tests/command_tests.rs`: `test_solid_composition_commands_and_undo_redo` validando o ciclo completo no despachante de comandos.
  - `crates/editor/tests/contextual_interactions.rs`: `solid_composition_multi_solid_and_reedit_invariants_h26` validando integração do editor, ocultação de operandos secundários, bake explícito e reversibilidade integral em múltiplos passos de Undo.

**Verification:**
- `crates/mesh/tests/solid_builder_tests.rs`: 12 passed; 0 failed.
- `crates/project/tests/solid_composition_tests.rs`: 7 passed; 0 failed.
- `crates/project --lib`: 244 passed; 0 failed.
- `crates/core/tests/command_tests.rs`: 62 passed; 0 failed.
- `crates/editor/tests/contextual_interactions.rs`: 14 passed; 0 failed.
- `crates/ui-cxxqt --test editor_parity`: 21 passed; 0 failed (gate Qt offscreen).
- Clippy e fmt: zero warnings e zero erros em todos os pacotes.

**Risks / Next checkpoint:**
- Checkpoint concluído: H27. Próximo checkpoint: H28 (Feedback comum de operações: preview de fonte fixa, ghost, parâmetros, revisão e incompatibilidades antes de confirmar).

---

### Entrega H27 — Construção sobre superfícies (Plano de face, Imprint com furos, Push/Pull contextual) (2026-10-16)

**Intent:**
- Implementar os comandos e fluxos transacionais da Task H27:
  1. Alinhamento de plano de trabalho (`ProfileWorkplane`) à face planar de uma malha autoral (`AlignWorkplaneToFaceCmd` e `AlignWorkplaneToSelectionCmd`), rejeitando graciosamente faces curvas ou não-planares em conformidade com o contrato H27 ("superfícies curvas como extensão posterior").
  2. Imprint de perfis contendo furos (`ImprintProfileCmd`) sobre superfícies coplanares da malha alvo via triangulação `earcut`, subdividindo a face hospedeira e selecionando as regiões internas resultantes.
  3. Push/Pull contextual (`ContextualPushPullCmd`), discriminando se a seleção é um contorno livre de sólido (translada face esticando paredes vizinhas sem criar loop) ou região interna embutida (cria extrusão com paredes laterais).
  4. Suporte irrestrito a Undo/Redo e conformidade rigorosa com a regra "Everything is a Command".

**Sources:**
- `crates/mesh/src/imprint.rs`
- `crates/core/src/surface_commands.rs`
- `crates/core/src/state.rs`
- `crates/core/src/command.rs`
- `crates/core/src/context_api.rs`
- `crates/editor/src/ui_bridge.rs`
- `crates/core/tests/command_tests.rs`
- `crates/editor/tests/contextual_interactions.rs`

**Verification:**
- `crates/mesh/src/imprint.rs`: teste unitário `imprint_region_with_hole_splits_face_with_earcut` (pass).
- `crates/core/tests/command_tests.rs`: `test_surface_construction_commands_and_undo_redo` (pass).
- `crates/editor/tests/contextual_interactions.rs`: 15 passed, 0 failed (incluindo `surface_construction_face_plane_imprint_with_holes_and_push_pull_h27`).
- `crates/ui-cxxqt --test editor_parity`: 21 passed, 0 failed (gate Qt offscreen).
- `cargo clippy`: zero warnings em `petunia_core` e `petunia_editor`.

**Risks / Next checkpoint:**
- Checkpoint concluído: H27. Próximo checkpoint: H28 (Feedback comum de operações: preview de fonte fixa, ghost, parâmetros, revisão e incompatibilidades antes de confirmar).

---

### Entrega H28 — Feedback comum de operações (MF-001 a MF-008) (2026-10-16)

**Intent:**
- Implementar o subsistema de feedback comum e unificado de operações de modelagem (MF-001 a MF-008):
  1. `MF-001`: Projeção unificada `OperationFeedback` (`ModelingFeedback`) expondo fonte/revisão fixa, malha ghost (geometria de origem congelada), malha de prévia ativa, reversibilidade não-destrutiva sem poluição de histórico no cancelamento e transação única de Undo no commit.
  2. `MF-002`: Parâmetros ativos com bounds explícitos e limites de advertência (`APPROACHING_LIMIT`, `EXCEEDED_LIMIT`, `INSET_COLLAPSE_LIMIT`, `BEVEL_OVERLAP_LIMIT`, `EXTRUDE_NEGATIVE`).
  3. `MF-003`: Preflight de operações booleanas identificando incompatibilidades explícitas (detecção de operandos abertos/não-manifold via `is_closed_manifold()` e auto-booleana) antes da confirmação, sem auto-reparo silencioso.
  4. `MF-004`: Preflight de espelhamento/simetria com marcador de plano discreto (`FeedbackMarker::PlaneDisc`) e detecção de assimetria artística sem imposição de correção forçada.
  5. `MF-005`: Preflight de pincéis de escultura e deformação com marcadores de footprint e falloff (`FeedbackMarker::BrushFootprint`).
  6. `MF-006`: Diagnósticos e avisos contextuais não-bloqueantes (`FeedbackWarning`, `WarningSeverity`) sem interrupção de fluxo modal.
  7. `MF-007`: Incompatibilidades estruturais e seleções heterogêneas reportando alvos ignorados (`ignored_targets`, `supported_targets`, aviso de rebind ao fazer remesh em malhas skinned).
  8. `MF-008`: Pureza irrestrita das consultas de feedback — nenhuma consulta altera ferramenta ativa, atalhos, documento ou pilha de Undo.
  9. Safe commit guard: rejeição transacional e restauração automática segura caso a revisão do documento tenha sofrido mutação concorrente durante o gesto (`is_stale == true`).

**Sources:**
- `crates/core/src/modeling_feedback.rs`
- `crates/core/src/modal.rs`
- `crates/core/src/state.rs`
- `crates/core/src/context_api.rs`
- `crates/core/src/lib.rs`
- `crates/editor/src/bridge/tool_preview.rs`
- `crates/core/tests/modeling_feedback_tests.rs`
- `crates/core/tests/command_tests.rs`
- `crates/editor/tests/contextual_interactions.rs`

**Verification:**
- `crates/core/tests/modeling_feedback_tests.rs`: 8 passed, 0 failed.
- `crates/core/tests/command_tests.rs`: `test_modeling_feedback_and_ghost_invariants_h28` (pass).
- `crates/editor/tests/contextual_interactions.rs`: `modeling_feedback_mf001_to_mf008_invariants_h28` (pass).
- `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_editor --all-targets -- -D warnings`: zero warnings.
- `CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_editor -- --check`: clean.

**Risks / Next checkpoint:**
- Checkpoint concluído: H28. Próximo checkpoint: H29 (Dimensões e comparação espacial: VSP-003/004 — frames World/Local/Custom/Structural, source/evaluated e qualidade; escala dimensional separada de consulta).

---

### Entrega H29 — Dimensões e comparação espacial (VSP-003 e VSP-004) (2026-10-16)

**Intent:**
- Implementar o subsistema de medição de volume, dimensões espaciais e consultas comparativas (VSP-003 e VSP-004):
  1. `VSP-003`: `VolumeFrame` com representação `Evaluated` (padrão) e `Source` identificada separadamente. Frames canônicos suportados: `World`, `ObjectLocal`, `Custom` (base ortonormal via Gram-Schmidt), `Structural` (eixos principais de inércia / PCA calculados via autovetores Jacobi 3x3 com fallback automático para `ObjectLocal` em geometrias isotrópicas) e `View` (espaço de câmera). Invariante comprovado: frames não são redutíveis a uma simples `World AABB`.
  2. Qualidade de medição explícita (`MeasurementQuality::Exact`, `Approximate`, `Unavailable`, `Stale`).
  3. Formatação dimensional neutra e configurável (`formatted_dimensions`, `format_measurement`) respeitando unidades de cena e sem inventar sufixos `cm`/`m` em cenas genéricas.
  4. Separação estrita entre medir/consultar, escalar até dimensão (`ScaleToDimensionCmd` / `model.scale_to_dimension`, `transform.scale_to_dimension`) e editar parâmetros. Prevenção comprovada contra divisão por zero em geometrias com dimensão zero (ex: planos ou malhas degeneradas) e integridade transacional de Undo/Redo.
  5. `VSP-004`: Seis consultas espaciais distintas (`SpatialQueryKind`): distância euclidiana (`EuclideanDistance`), delta de componente em eixo no frame (`AxisDelta`), profundidade de câmera (`CameraDepth`), clearance de superfície com par de pontos mais próximos (`SurfaceClearance`), contato sob tolerância (`ContactUnderTolerance`) e interseção volumétrica via SAT de 15 eixos (`VolumetricIntersection`).
  6. Companion View (`CompanionViewState`) com projeção ortográfica independente, opt-in explícito, sem roubo de foco nem alteração da câmera principal da viewport.
  7. Invariante estrito X-Ray vs Select Through: ativação de raio-X visual nunca altera a política de seleção transparente/select through.

**Sources:**
- `crates/core/src/spatial_query.rs`
- `crates/core/src/lib.rs`
- `crates/core/src/command.rs`
- `crates/core/src/context_api.rs`
- `crates/core/src/state.rs`
- `crates/core/src/modal.rs`
- `crates/core/tests/spatial_query_tests.rs`
- `crates/core/tests/command_tests.rs`
- `crates/editor/tests/contextual_interactions.rs`

**Gap:**
- O Petunia3D carecia de um subsistema robusto para medição de caixas delimitadoras orientadas (OBB) em múltiplos frames de referência (especialmente eixos principais estruturais via PCA), consultas analíticas de folga de superfície (clearance) e interseção volumétrica SAT, além de controle ortogonal de visualização auxiliar (Companion View) e separação de X-Ray/Select Through.

**Changed:**
- Criado o módulo `spatial_query.rs` em `crates/core/src/` com todos os tipos, kernels matemáticos (Jacobi 3x3, Gram-Schmidt, SAT 15 eixos, point-to-triangle distance), comando `ScaleToDimensionCmd` e inspetor `SpatialQueryInspector`.
- Registrado `pub mod spatial_query;` e exportados os tipos principais em `crates/core/src/lib.rs`.
- Registrado o comando `model.scale_to_dimension` (e alias `transform.scale_to_dimension`) no despachante canônico em `crates/core/src/command.rs`.
- Integrado `companion_view` no `EditorSession` e expostos métodos de consulta no `EditorView` (`crates/core/src/state.rs`).
- Adicionados métodos de consulta e controle em `AppState` e `CommandContext` (`crates/core/src/context_api.rs`).
- Corrigida atualização de `published_revision` no `update_modal_fast` em `crates/core/src/modal.rs`.
- Desenvolvida suíte unitária completa `crates/core/tests/spatial_query_tests.rs` com 7 testes cobrindo todos os invariantes VSP-003 e VSP-004.
- Adicionados testes de integração em `crates/core/tests/command_tests.rs` e `crates/editor/tests/contextual_interactions.rs`.

**Verification:**
- `crates/core/tests/spatial_query_tests.rs`: 7 passed, 0 failed.
- `crates/core/tests/command_tests.rs`: `test_spatial_dimension_and_comparison_invariants_h29` (pass).
- `crates/editor/tests/contextual_interactions.rs`: `spatial_dimensions_and_companion_view_invariants_h29` (pass).
- `crates/core` unit tests: 190 passed, 0 failed.
- `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_editor --all-targets -- -D warnings`: zero warnings.
- `CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_editor -- --check`: clean.

**Risks / Next checkpoint:**
- Degenerescência de autovalores em geometrias isotrópicas: mitigada pelo fallback automático do frame `Structural` para o frame `ObjectLocal`.
- Checkpoint concluído: H29. Próximo checkpoint: H30 (Section, Thickness e Organic Profiles: VSP-005 — seção geométrica separada de clipping/split, contornos abertos/furos, espessura direcional e estações manuais).

---

### Entrega H30 — Section, Thickness e Organic Profiles (VSP-005) (2026-10-16)

**Intent:**
- Implementar o subsistema analítico de corte geométrico, sonda de espessura de parede e perfis orgânicos (VSP-005):
  1. `Section Inspector`: corte analítico não-destrutivo estritamente desacoplado de GPU clipping (shader discard) e Geometry Split (mutação topológica). Suporta classificação exata de loops abertos (`LoopKind::Open`), fechados (`LoopKind::Closed`), cálculo de área planar (Shoelace/Gauss) e detecção de furos coplanares aninhados (`is_hole = true`) via teste ponto-em-polígono (Ray casting).
  2. `Synthetic Caps`: geração opcional de malha de preenchimento (`SyntheticCap`) para visualização na viewport com invariante estrito `is_selectable = false` (nunca tratadas como faces autorais da malha por picking ou ferramentas de edição).
  3. `Section Plane Control`: plano de corte analítico (`SectionPlane`) com ancoragem canônica no Mundo ou Objeto (`SectionAnchor`), edição numérica de passagem, offset paramétrico e inversão de normal (`flip`). Comandos dedicados registrados no despachante: `model.set_section_plane`, `model.clear_section_plane`, `model.flip_section_plane` e `model.offset_section_plane`.
  4. `Thickness Probe`: sonda analítica direcional de espessura de parede (`ThicknessProbeQuery`, `ThicknessProbeResult`) via traçado de raios Möller-Trumbore em triângulos locais, com classificação explícita de grandeza de parede (`WallType::ParallelWall`, `ObliqueWall`, `NoOpposingWall`) e qualidade de medição (`MeasurementQuality::Exact`, `Approximate`, `Unavailable`).
  5. `Organic Profiles`: estações ao longo de linhas de centro (`OrganicCenterline`, `ProfileStation`) usando Parallel Transport Frame (Bishop Frame / Rotation Minimizing Frame - RMF) via rotação de Rodrigues para garantir matematicamente **zero flips e sem inversão de normais** ao longo de curvas 3D arbitrárias e através de pontos de inflexão ($\kappa = 0$). Suporte a bifurcações (`StationBranch`) como segmentos independentes que herdam suavemente a base de orientação da estação tronco mãe sem inferência anatômica mágica.

**Sources:**
- `crates/core/src/section_analysis.rs`
- `crates/core/src/lib.rs`
- `crates/core/src/command.rs`
- `crates/core/src/context_api.rs`
- `crates/core/src/state.rs`
- `crates/core/tests/section_analysis_tests.rs`

**Gap:**
- O Petunia3D carecia de uma ferramenta analítica de seção transversal geométrica independente do pipeline de renderização, cálculo de espessura de parede de malha com detecção de paralelismo e estações de perfis orgânicos com garantia matemática de estabilidade de eixos sem inversão de normais de Frenet-Serret.

**Changed:**
- Criado o módulo `section_analysis.rs` em `crates/core/src/` com todos os tipos (`SectionAnchor`, `SectionPlane`, `LoopKind`, `SectionContour`, `SyntheticCap`, `SectionResult`, `WallType`, `ThicknessProbeQuery`, `ThicknessProbeResult`, `ProfileStation`, `StationBranch`, `OrganicCenterline`), comandos e inspetor `SectionInspector`.
- Registrado `pub mod section_analysis;` e exportados os tipos e comandos em `crates/core/src/lib.rs`.
- Adicionado campo `active_section: Option<SectionPlane>` em `EditorSession` e métodos de consulta (`query_section`, `probe_thickness`) em `EditorView` (`crates/core/src/state.rs`).
- Adicionados métodos de consulta e controle em `AppState` e `CommandContext` (`crates/core/src/context_api.rs`).
- Registrados comandos `model.set_section_plane`, `model.clear_section_plane`, `model.flip_section_plane`, `model.offset_section_plane` (e aliases `view.*`) em `crates/core/src/command.rs`.
- Desenvolvida suíte unitária completa `crates/core/tests/section_analysis_tests.rs` com 9 testes cobrindo todos os requisitos VSP-005.

**Verification:**
- `crates/core/tests/section_analysis_tests.rs`: 9 passed, 0 failed (0.03s).
- `crates/core/tests/spatial_query_tests.rs`: 10 passed, 0 failed (0.02s).
- `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core --all-targets -- -D warnings`: zero warnings.
- `CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -- --check`: clean.

**Risks / Next checkpoint:**
- Degenerescência de malhas abertas ou raios divergentes: tratada com retorno gracioso de `WallType::NoOpposingWall` e qualidade `MeasurementQuality::Unavailable` sem pânicos.
- Checkpoint concluído: H30. Checkpoint concluído: H30. Próximo checkpoint: H31 (Smart Rig manual e assistido).

---

### Entrega H31 — Smart Rig manual e assistido (RIG-001/002/004/008/009/010/011) (2026-10-16)

**Intent:**
- Implementar as capacidades fundamentais do Smart Rig manual e assistido:
  1. Criação e edição manual de esqueletos e ossos para anatomias arbitrárias (bípede, quadrúpede, multípode/inseto, serpente, peixe, asas, criaturas assimétricas; determinístico local sem suposições anatômicas embutidas). Comandos: `CreateEmptySkeletonCmd`, `AddBoneCmd`, `RemoveBoneCmd`, `ReparentBoneCmd` (com validação estrita anti-ciclos e preservação de rest offsets absolutos), `EditBoneRestPoseCmd` e `RenameBoneCmd`.
  2. Preview de skinning revisável (`SkinningPreviewState`) com overrides manuais por vértice/osso antes do commit, desacoplado de mutações prematuras no documento ou Undo stack.
  3. Bind transacional (`BindRigCmd`, `UnbindRigCmd`) suportando métodos `Proximity`, `RigidPerPart` e `SmoothEnvelope`, com limite rígido de até 4 influências por vértice e normalização invariante $\sum w_i = 1.0$.
  4. Isolamento estrito entre Modo de Pose e Rest Pose: `ResetAllBonePosesCmd` restaura todos os ossos para suas posições de repouso canônicas; `ApplyPoseAsRestCmd` assenta explicitamente a pose de deformação atual como nova Rest Pose de repouso, recalculando as matrizes de bind inversas e deformando a geometria do asset correspondente.
  5. Desativação não-destrutiva de skinning (`SetSkinningEnabledCmd` / `asset.skinning_enabled`) permitindo desligar a deformação de malha instantaneamente sem deletar ossos, pesos de skinning associados ou clipes de animação (RIG-012).

**Sources:**
- `crates/project/src/rig.rs`
- `crates/project/src/lib.rs`
- `crates/project/src/format.rs`
- `crates/core/src/rig_commands.rs`
- `crates/core/src/lib.rs`
- `crates/core/tests/smart_rig_tests.rs`

**Gap:**
- O Petunia3D carecia de comandos transacionais e de domínio para criação manual de anatomias de esqueletos arbitrárias (além de presets humanóides/quadrúpedes), cálculo automático de bind weights por Proximity/Rigid/Smooth com capping e normalização, preview volátil antes do bind, desacoplamento seguro entre poses de teste e rest pose com capacidade de assentar nova rest pose deformando a malha, e toggle não-destrutivo de deformação.

**Changed:**
- Em `crates/project/src/rig.rs`: adicionados `edit_bone_rest_pose`, `rename_bone`, `SkinBindMethod` (`Proximity`, `RigidPerPart`, `SmoothEnvelope`) e função pura `compute_skin_weights` com clamp a max 4 influências e normalização unitária.
- Em `crates/project/src/lib.rs` e `format.rs`: adicionado campo `pub skinning_enabled: bool` ao `Asset` (com default `true` serializável).
- Em `crates/core/src/rig_commands.rs`: implementados comandos `CreateEmptySkeletonCmd`, `AddBoneCmd`, `RemoveBoneCmd`, `ReparentBoneCmd`, `EditBoneRestPoseCmd`, `RenameBoneCmd`, `BindRigCmd`, `UnbindRigCmd`, `ResetAllBonePosesCmd`, `ApplyPoseAsRestCmd`, `SetSkinningEnabledCmd` e a struct `SkinningPreviewState`.
- Em `crates/core/src/lib.rs`: exportados todos os novos tipos e comandos de smart rig.
- Em `crates/core/tests/smart_rig_tests.rs`: desenvolvidos 7 testes cobrindo todas as anatomias, anti-ciclo, reparent, rest pose edit, bind methods, unbind, preview overrides, pose test vs rest pose e desativação não-destrutiva.

**Verification:**
- `crates/core/tests/smart_rig_tests.rs`: 7 passed, 0 failed (0.06s).
- `crates/core/tests/rig_command_tests.rs`: 8 passed, 0 failed (0.08s).
- `crates/project/src/lib.rs`: 246 passed, 0 failed (0.44s).
- `crates/core/src/lib.rs`: 190 passed, 0 failed (1.20s).
- `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_project --all-targets -- -D warnings`: zero warnings.
- `CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_project -- --check`: clean.

**Risks / Next checkpoint:**
- Degenerescência de ossos com comprimento zero ou coordenadas não finitas: rejeitada com `CommandError::InvalidInput` em `can_execute` e nas funções de esqueleto.
- Checkpoint concluído: H31. Próximo checkpoint: H32 (Smart Rig estrutural e Live opcional: RIG-003/005/006/007/012 — identidades estáveis, overrides, atualização localizada idempotente e desligar sem apagar rig).

---

### Entrega H32: Smart Rig estrutural e Live opcional (RIG-003/005/006/007/012)

**Intent:**
- Implementar o subsistema de Smart Rig Estrutural e sincronização Live opcional com conformidade transacional estrita:
  1. Grafo semântico de forma estrutural (`StructuralShapeGraph`) com identidades estáveis (`PartId`, `JointId`, `ConnectionId`) independente e desacoplado de receitas geométricas transitórias (RIG-003).
  2. Preservação de overrides manuais de pesos (`overrides: HashMap<usize, VertexSkinWeight>`) e travas granulares (`locked_parts`, `locked_bones`) em atualizações subsequentes (RIG-005/006).
  3. Solver localizado e estritamente idempotente (`update_structural_rig_localized`): avalia revisões de fonte, recalcula somente partes com geometria mutada e devolve `changed: false` quando o documento não foi alterado.
  4. Isolamento de materiais e UVs: mutações em `material_revision` e `uv_revision` avançam o snapshot do live rig sem recomputar pesos de skinning ou deformação (RIG-007).
  5. Guarda contra mutações topológicas destrutivas: discrepâncias em contagem de vértices acionam explicitamente `RigReviewStatus::NeedsReview(reason)` em vez de reutilização silenciosa de índices corrompidos.
  6. Remapeamento explícito de pesos (`RemapRigWeightsCmd` / `remap_skin_weights`) restaurando `RigReviewStatus::UpToDate`.
  7. Três operações independentes de desligamento no RIG-012: `SetLiveAutoUpdateCmd` (desliga sincronização automática sem desativar deformação), `SetSkinningEnabledCmd` (desativa deformação sem apagar pesos ou ossos) e `UnbindRigCmd` / `RemoveSkeletonCmd` (desvinculação explícita; nenhuma opção "Off" destrói ossos, pesos ou animações acidentalmente).

**Sources:**
- `crates/project/src/rig.rs`
- `crates/project/src/lib.rs`
- `crates/project/src/format.rs`
- `crates/core/src/rig_commands.rs`
- `crates/core/src/lib.rs`
- `crates/core/tests/structural_live_rig_tests.rs`

**Gap:**
- O Petunia3D não dispunha de grafo estrutural desacoplado com identificadores estáveis, persistência de overrides de pesos e travas em atualizações, solver localizado e idempotente de rig, diferenciação entre alterações de UV/material e alterações de geometria, e separação conceitual entre pausar live sync, desligar skinning e remover rig (RIG-012).

**Changed:**
- Em `crates/project/src/rig.rs`: adicionados tipos `PartId`, `JointId`, `ConnectionId`, `ConnectionKind`, `StructuralPart`, `StructuralJoint`, `StructuralConnection`, `StructuralShapeGraph`, `RigReviewStatus`, `LiveRigData`, `StructuralRigUpdateReport` e funções puras `update_structural_rig_localized` e `remap_skin_weights`.
- Em `crates/project/src/lib.rs` e `format.rs`: adicionado campo `pub live_rig: Option<LiveRigData>` ao `Asset`.
- Em `crates/core/src/rig_commands.rs`: implementados comandos `SetLiveAutoUpdateCmd`, `CreateStructuralGraphCmd`, `AddStructuralPartCmd`, `RemoveStructuralPartCmd`, `AddStructuralJointCmd`, `ConnectStructuralPartsCmd`, `SetStructuralPartLockedCmd`, `SetStructuralBoneLockedCmd`, `SetVertexWeightOverrideCmd`, `ClearVertexWeightOverrideCmd`, `UpdateStructuralRigCmd` e `RemapRigWeightsCmd`.
- Em `crates/core/src/lib.rs`: exportados todos os novos tipos e comandos.
- Em `crates/core/tests/structural_live_rig_tests.rs`: desenvolvidos 8 testes sintéticos cobrindo criação de grafo, conexões, juntas, bounds, idempotência, isolamento UV/material, preservação de overrides/locks, detecção de topologia destrutiva com status `NeedsReview`, remapeamento transacional e as 3 operações distintas de desligamento do RIG-012.

**Verification:**
- `crates/core/tests/structural_live_rig_tests.rs`: 8 passed, 0 failed (0.04s).
- `crates/core/tests/smart_rig_tests.rs`: 7 passed, 0 failed (0.04s).
- `crates/core/tests/rig_command_tests.rs`: 8 passed, 0 failed (0.06s).
- `crates/project/src/lib.rs`: 246 passed, 0 failed (0.47s).
- `CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_project --all-targets -- -D warnings`: zero warnings.
- `CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_project -- --check`: clean.

**Risks / Next checkpoint:**
- Topologia destrutiva durante live rig: adequadamente interceptada pelo solver, definindo `RigReviewStatus::NeedsReview` com mensagem detalhada e rejeitando atualizações automáticas silenciosas até que o usuário execute `RemapRigWeightsCmd` ou re-faça o bind intencionalmente.
- Checkpoint concluído: H32. Próximo checkpoint: H33 (Integração visual e acessibilidade posterior: VSP-001/002/004, SEL-007 — Clay, Companion View, gizmo, feedback acessível e gates Qt/GL/Windows/Linux).


