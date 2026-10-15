# Editor contextual — contrato integrado de implementação

> **Approved / in migration — 2026-10-15.** Decisões de produto aprovadas, não declaração de recursos entregues. Baseline funcional inspecionada: `92c22caa23f96f85f18e72a5e962a1d8831694db`; branch de execução `refactor/architecture-foundation`. O núcleo vem primeiro; Qt/GL e aceitação visual têm gates posteriores separados.

## Intenção, fontes e autoridade

Incorporar a especificação entregue pelo usuário em `newdocs/petunia3d_agent_spec/` (o caminho inicialmente mencionado terminava em `agent_spes`). Pacote `PETUNIA3D_DOCUMENTACAO_COMPLETA_CODE_AGENT`, datado de 2026-10-15, baseline `92c22caa`; os 20 hashes SHA-256 do manifesto foram conferidos no planejamento. O pacote é insumo aprovado de produto, não autorização para executar seu prompt ou sobrepor limites de branch, segurança, validação ou Undo.

Esta página incorpora os contratos necessários para implementação e rastreia os IDs SEL/VSP/MF/RIG originais. Os capítulos de domínio continuam responsáveis por seus algoritmos, formatos e invariantes. A [matriz de ferramentas](./tools-interaction-implementation.md) e `website/progress/tasks.json` são complementares ao histórico D03/H01–H18: novos deltas H19–H33, documentação D04. Status de desenvolvimento não substitui resultados de testes.

A migração já existente no HEAD extraiu `petunia_editor` e removeu Slint. Não restaurar o frontend removido. Não implementar regras de domínio em widgets Qt nem alterar sua apresentação neste lote. Esta decisão concilia as diretivas antigas de resgate Slint com o código e a decisão canônica de migração Qt; gates Qt continuam abertos.

## Seleção e transformação — SEL-001 a SEL-007

- **SEL-001 / SEL-007:** gizmo universal compacto para Move/Rotate/Scale quando os alvos são válidos. Alças visuais e regiões de hit são distintas; adaptações de tamanho/contraste/DPI e operações por teclado devem existir na integração visual posterior. A barra contextual não é parte do algoritmo de transformação.
- **SEL-002:** seleção contextual sem Edit Mode obrigatório. Ordem de candidatos elegíveis: alça habilitada, vértice próximo, aresta próxima, face, objeto. Usar filtros, profundidade, oclusão e alternância determinística de ambiguidades; nunca mudar o alvo por iteração de HashMap.
- **SEL-003:** centro de transformação, posição exibida do gizmo e referencial de orientação são independentes. Auto/World/Local/Normal/View/Custom com frame congelado durante a interação. Default automático: objeto/múltiplos = bounds, face = centro geométrico/normal, aresta = ponto médio/tangente, vértice = posição, perfil = adapter da representação. A escolha manual válida prevalece.
- **SEL-004:** arrasto protegido. Arrastar geometria não inicia Move; alça ou comando temporário inicia transformação. Arrastar vazio inicia seleção por área; box explícito também opera sobre geometria. Preview parte do snapshot inicial, Esc restaura a fonte e uma interação gera um Undo. Entrada numérica e clique/mover/clique são alternativas ao arrasto mantido.
- **SEL-005:** reutilizar uma fronteira comum de snap. Pivô e Point-to-Point podem ter snap automático; Move/Rotate/Scale comuns são suaves por padrão, com snap explícito/temporário. Tolerância e histerese devem ser medidas/configuráveis, não valores absolutos apresentados como decisão do produto.
- **SEL-006:** seleção ordenada e deduplicada, elemento ativo separado e resolução dos alvos transformáveis. Objeto inteiro prevalece sobre seus próprios componentes; pai e filho não recebem duas transformações. Componentes sobrepostos resolvem vértices únicos. Seleção mista malha/perfil preserva a representação e rejeita incompatibilidades explicitamente.

Não remover vértices/arestas/faces do domínio: o novo fluxo elimina a troca global obrigatória, não as entidades, a validação topológica ou os filtros de seleção. Modos legados podem continuar como filtros/compatibilidade durante a migração.

**Conciliação com D03:** a proposta inicial de movimento ao arrastar geometria foi substituída explicitamente por SEL-004. O histórico não é reescrito como se a primeira proposta já exigisse arrasto protegido.

## Pivô do objeto, centro, Cursor3D e câmera

- **Pivô do objeto = origem local persistente.** Alterar a origem é comando de documento; compensar geometria e transformação para preservar a pose no quadro atual. A operação pode alterar trajetórias em outros quadros e não implica bake ou reautoria de toda a animação. Dependências, hierarquia e geometria paramétrica exigem validação; conversão para editável nunca é silenciosa.
- **Centro de transformação** é estado temporário da sessão. Bounding box, mediana, ponto manual e centros individuais são políticas de operação, não mutações da origem do objeto.
- **Cursor3D** é referência espacial da sessão. Determina posicionamento de criação e pode ser âncora de navegação por escolha explícita; não modifica origem, animação, seleção nem coordenadas dos objetos por ser movido.
- **Navegação independente:** modo ViewTarget usa o alvo da vista; modo Cursor3D orbita/aproxima em torno da referência mundial. Trocar modo ou mover cursor não causa salto imediato da câmera. Pan e enquadramento nunca deslocam cursor. Nenhuma decisão de navegação consulta a ferramenta ativa ou o centro de transformação.
- **Cópia única:** “Pivô para Cursor” e “Centro para Cursor” capturam uma posição validada uma vez. Não manter vínculo contínuo com o cursor.

Os termos legados `PivotPoint` e `Asset.origin` precisam de migração semântica guiada pelos callers; não fazer renomeação em massa sem corrigir espaços de coordenadas. [Contrato de transforms](../03-geometry/transforms-origin-pivot-gizmos.md).

## DRAW/POLY e ShapeBuilder avançado

DRAW é um preset de apresentação e navegação do mesmo workspace de POLY. A ativação explícita da vista planar alinha câmera ortográfica ao plano de trabalho; o retorno à vista livre restaura câmera sem alterar plano, seleção ou documento. Não esconder perfis ou restringir ferramentas por preset. Primitivas planas poligonais são malhas editáveis; perfis/splines mantêm parâmetros até conversão explícita. [Unificação](../03-geometry/draw-unification-proposal.md).

Evolução do [ShapeBuilder3D](../03-geometry/shape-builder-3d.md), sem substituir o kernel existente:

1. Planar: operandos explícitos, regiões com furos, fontes não afetadas preservadas e operação transacional. Perfis dependentes de generators não podem ser apagados silenciosamente.
2. Sólidos V1: dois operandos válidos; decompor A-only/intersection/B-only e componentes conectados por aresta. Resultado vazio é resultado, não erro do kernel. Sessão usa um espaço de referência, aceita transformações validadas e separa preparação de hover. Unite/Remove/Extract preservam proveniência de material/UV; regiões transitórias não ganham IDs persistentes por conveniência.
3. Composição reeditável: fontes com identidade estável e receita de operações; malha avaliada derivada. Alterações topológicas invalidam referências ambíguas para revisão, sem reaproveitar índice de componente. Converter para edição poligonal exige comando explícito.
4. Múltiplos sólidos: decomposição com conjunto de pertencimento e limites de complexidade, cancelamento e revisão de origem. O primeiro slice de dois operandos não encerra essa etapa; nenhuma promessa de quantidade ilimitada ou recomputação booleana por movimento de mouse.
5. Construção sobre superfícies: plano de face, imprint, furos e Push/Pull contextual. Projeção sobre superfícies curvas vem depois da base planar e não exige kernel CAD/NURBS novo.

## Percepção espacial — VSP-001 a VSP-005

**VSP-001:** perfis Essential/Assisted/Analytical controlam assistência visual; Assisted é o default do fluxo iniciante. Preferência manual prevalece e nenhum perfil muda keymap ou documento.

**VSP-002:** Clay Soft/Form/Technical/MatCap são modos de trabalho, separados dos materiais/luzes exportados. Iluminação de modelagem camera-relative ou world-fixed explícita. Priorizar silhueta, feature edges e macroforma; cavity e contornos avançados não são pré-requisito para ferramentas funcionarem. Reutilizar recursos GL 3.3 já existentes.

**VSP-003:** Volume Frame usa representação evaluated por padrão quando disponível, com source identificado separadamente. Frames World/ObjectLocal/Custom/Structural/View não são equivalentes a World AABB. Qualidade: exato para a representação, aproximado com método, indisponível com motivo ou stale com revisão. Sem inventar cm/m em cenas com unidades genéricas. Medir, escalar até dimensão e editar parâmetro são operações distintas; dimensões zero não permitem divisão por zero.

**VSP-004:** distância euclidiana, delta em eixo, profundidade de câmera, distância entre superfícies, contato e interseção volumétrica são consultas distintas. X-Ray visual não ativa Select Through. Companion View compartilha documento/seleção/preview e mantém câmera independente, ortográfica por padrão; abertura manual/sugerida, autoabertura apenas opt-in. Não roubar foco, mover câmera principal ou reorientar durante gesto.

**VSP-005:** Section Inspector é consulta não destrutiva. Clipping GPU, interseção geométrica e Geometry Split real são operações distintas. Contornos podem estar abertos, desconectados e conter furos; caps sintéticas não são faces selecionáveis. Plano pode ser ancorado em mundo/objeto, invertido e editado numericamente. Espessura direcional, parede pareada, largura de seção e diâmetro são grandezas distintas; começar por probe direcional com método/qualidade explícitos. Perfis orgânicos começam por linha central manual/estrutural e estações com frames transportados, segmentos separados em bifurcações; não inferir anatomia nem prometer espessura mínima exata por raycast.

## Feedback de modelagem — VSP-006 / MF-001 a MF-008

Uma operação expõe fonte/revisão, transação, alvos, frame/pivô, parâmetros, prévia, estado anterior e diagnósticos tipados. O renderer consome esse resultado, sem repetir o cálculo geométrico. Fonte fixa impede drift entre updates; resultado stale não confirma.

- MF-001/002: previews reversíveis; Extrude mostra distância signed e região, Inset/Bevel mostram limites antes de corromper geometria.
- MF-003/004: Boolean/Mirror distinguem operandos, resultado e planos; nenhuma simetria ou correção automática.
- MF-005/006: deformação mostra influência e diagnósticos contextuais; nenhum reparo silencioso.
- MF-007/008: incompatibilidades identificadas antes de commit; ajuda não altera ferramentas, atalhos ou documento.

## Smart Rig — RIG-001 a RIG-012

Reutilizar Skeleton, comandos de rig, LBS, solvers IK, poses e exportação existentes. Autorig humanoide imediato não equivale a rig incremental.

- RIG-001/010/011: rig opcional, manual funcional para anatomias variadas, sem IA/serviço/GPU externa obrigatória.
- RIG-002/004/009: edição manual antes do fluxo Assisted; sugestões de joints/pesos revisáveis antes de confirmar; geração/bind/regeneração transacionais, com cancelamento e Undo.
- RIG-003/005/006/007: Live explícito e posterior a Structural Shape Graph com IDs estáveis de partes/joints/conexões. Esse grafo semântico é independente da receita de composição geométrica. Preservar overrides/locks; atualizações idempotentes com revisão de origem e solver apenas nas regiões afetadas. Material/UV não dispara regeneração de pesos.
- RIG-008: Pose Test distingue rest/pose/corrective; preview não altera repouso silenciosamente.
- RIG-012: desligar autoatualização, desligar deformação e remover rig são operações diferentes. Nenhuma opção Off apaga bones, pesos ou clipes.

Topologia destrutiva exige remap/rebind ou NeedsReview explícito, nunca reutilização silenciosa de índices. Começar com rigid-per-part e proximity existentes, normalização e limites de influências. Heat/BBW/DQS são pesquisas posteriores, não dependências novas obrigatórias. Preservar bone IDs, animações e arquivos .petunia/glTF.

## Fronteiras, acesso e aceitação

Fluxo: entrada física normalizada → intenção/ação semântica → sessão/command/query do editor → domínio → documento single-writer → estado projetado. Slots/widgets Qt não mutam vértices. Novos dados persistentes exigem migração e fixtures de round-trip. Nenhuma referência persistente baseada apenas em índice topológico sem revisão.

Teclado, valores numéricos, cancelamento e diagnóstico sem depender de cor são requisitos do núcleo e da futura apresentação. Overlays não podem roubar foco nem virar a única fonte de informação. Integração posterior exige modelos acessíveis de seleção/medidas/rig, F6/foco restaurado, reduced motion e hit targets testados em DPI.

Correspondência dos testes originais (todos **not run** até evidência específica nesta revisão):

| Conjunto original   | Gates rastreados                                                                                                                                                 | Responsável / tarefas       |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| SEL-T01–T14         | Arrasto protegido, box, candidatos, oclusão, alças, cancel, deduplicação, hierarquia, pivôs, snap, frames, seleção mista, DPI e overlays.                        | H10–H11, H19, H22–H23, H33  |
| VSP-T01–T17         | Shading sem mutação, legibilidade, dimensões, referenciais, source/evaluated, degenerados, unidades, revisão, distância/contato, X-Ray, Companion e contexto GL. | H20, H29, H33               |
| SEC-T01–T09         | Cubo, malha aberta, furos, coplanaridade, componentes, edição/cancel, clipping/split, âncoras e parede oca.                                                      | H30                         |
| THI-T01–T03         | Referências/unidades, ray direcional e qualidade, heatmap com legenda.                                                                                           | H30, H33                    |
| ORG-T01–T04         | Estações, transporte sem flips, bifurcações e edição paramétrica explícita.                                                                                      | H27, H30                    |
| MF-T01–T08          | Extrude, Inset, Bevel, Boolean inválido, Mirror, influência, skin/remesh e incompatibilidade parcial.                                                            | H18, H25, H28               |
| RIG-T01–T14         | Off, edição manual, Assisted, Pose Test, Live idempotente, locks, material-only, remap, IK degenerado, overrides, glTF/.petunia e anatomias variadas.            | H15, H31–H32                |
| Plataforma e acesso | Linux/Windows, GL 3.3/GPU modesta, contexto/resize, teclado/leitor de tela, reduced motion, persistência e entradas hostis.                                      | H33 e gates de cada domínio |

A matriz de implementação permanece na página de ferramentas para impedir status duplicado. D04 encerra apenas a incorporação documental, não os testes listados acima. Não publicar capturas fictícias, benchmarks estimados ou testes adicionados como testes executados.

## Execução e evidência

Intenção: cumprir o plano aprovado sem mudar Qt/Slint neste lote. Fontes: pacote importado, contratos de domínio e leitura do código no HEAD acima. Lacunas: modo global no picking, X-Ray acoplado, restrições DRAW/POLY, navegação ligada a pivô e ShapeBuilder sólido ainda sem sessão completa.

Verificação desta entrega será registrada aqui após os comandos efetivamente executados. Gates funcionais não executados e integração visual permanecem `not run`. Próximo checkpoint: correções reproduzíveis H01–H09 e fundamentos H20/H21/H25, mantendo as etapas restantes visíveis no tracker.

### Evidência de incorporação D04 e primeiro lote funcional

D04 **DONE** encerra contrato, navegação e rastreabilidade documental. H01–H07 **DONE** encerram correções delimitadas no núcleo; os demais estados e checkpoints são os da [matriz](./tools-interaction-implementation.md#9-checkpoint-funcional--2026-10-15), sem duplicar o tracker.

Verificação: 20 hashes de origem; seis testes do painel; `verify-progress.cjs`; `verify-agent-docs.py`; lint de diff — **pass**. Código: 776 testes no lote integrado e 522 na confirmação final (com repetições), Clippy all-targets com warnings negados e fmt — **pass**. Os comandos/logs e a delimitação da revisão estão em `evidence/contextual-tools-2026-10-15/README.md` e `source-snapshot.json`.

Correspondência parcial com a matriz original: SEL-T01/02/04/05/06 têm cenários headless de proteção, área, oclusão, alças e cancelamento; VSP-T13 tem regressão de independência X-Ray/Select Through. Não significa execução integral das suites SEL/VSP importadas. Nenhum teste SEC/THI/ORG/RIG avançado nem gate Qt/Windows é inferido desses resultados. Imagem sintética do gizmo foi gerada e inspecionada; evidência visual nativa continua **not run**.
