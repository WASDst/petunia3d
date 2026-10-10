# DRAW / POLY — unificação aprovada do Modeling Engine

> **Status: aprovado em 2026-10-08.** Esta decisão ratifica [Workspaces de modelagem](./modeling-workspaces.md), que já aprovava a unificação, e substitui o status anterior de proposta deste capítulo. **Não significa que a migração esteja concluída.**

## Decisão de produto

DRAW e POLY são **dois perfis de interação** sobre um **único Modeling Engine**. Permanecem identificáveis e acessíveis na GUI porque servem a fluxos mentais diferentes; não existem dois sistemas de geometria, duas seleções ou duas versões de Poly Pen. O objetivo é tornar a criação por desenho confortável a iniciantes sem penalizar a modelagem poligonal.

- **DRAW:** authoring planar, câmera ortográfica normal ao Workplane, pan e zoom, inferências e feedback de formas/curvas. Orbit livre fica desativado *enquanto o perfil planar está ativo*; trocar de perfil ou desativar o alinhamento não altera geometria.
- **POLY:** modelagem espacial, câmera livre em perspectiva ou ortográfica, edição de componentes e overlays topológicos.
- **Ambos:** mesmo documento, ObjectId, SelectionState, Workplane, snapping, Command Dispatcher, ToolSession e Undo/Redo.
- **MODEL:** domínio maior que agrega os perfis DRAW/POLY. Nenhum dos perfis cria projeto, sessão ou modo de edição global concorrente.

## Uma autoridade por dado

| Responsabilidade | Dono canônico | Não criar |
|---|---|---|
| Geometria mesh autoral | Geometry / EditableMesh | segunda mesh autoral de DRAW |
| Curvas | SplineResource persistente | BezierPath autoral concorrente |
| Forma planar | PlanarShape com contornos Spline | região paralela persistente em ProfileState |
| Superfície gerada | Geometry Source / Generator Evaluation | mesh + descriptor autorais simultâneos |
| Seleção | Application / SelectionState | SelectionDomain separado para DRAW |
| Plano ativo | EditorSession / Workplane | plano estático fixo no módulo DRAW |
| Preferências de ferramenta | ToolSettings | valores soltos duplicados por workspace |
| Gesto em execução | ToolSession | mutações por pointer diretamente em UI |
| Preview e overlays | Evaluation + render contracts | mesh transitória gravada no Document |
| Histórico | Command/Undo Dispatcher | Undo local ao workspace |

**Curvas não são meshes disfarçadas.** Spline e PlanarShape permanecem editáveis e paramétricos. Apenas operações que exigem mesh avaliam uma superfície; Make Editable/Bake é ato explícito.

## Gramática de ferramentas

Poly Pen edita mesh e Bezier Pen edita spline: **compartilham input, snapping, workplane, parâmetros e lifecycle, não algoritmo de geometria**.

- Ferramentas comuns: primitives, Knife/Cut, Push/Pull, extrusão, Shape Builder, snapping, transforms e materiais, conforme capacidades do objeto selecionado.
- DRAW prioriza Rectangle, Circle, Arc, Polygon, Bezier Pen, Trace, formas fechadas, extrusão a partir de perfil e cotas.
- POLY prioriza componentes Vertex/Edge/Face, Poly Pen, loops, operações de topologia e gizmos.
- A mesma operação pode ser acessada nos dois perfis; a interface informa quando um tipo de geometria exige conversão explícita.
- Comandos reutilizam os mesmos IDs, parâmetros, validação e Undo, independentemente de onde foram acionados.
- Mudança de perfil não faz Bake, Save, Commit de ferramenta modal, troca material nem move a câmera silenciosamente.

## Navegação planar

Planar Navigation usa a normal do Workplane ativo, com câmera ortográfica alinhada, pan e zoom. Presets cardinais, Align to Face e planos locais são ações distintas, reversíveis e visíveis.

1. Entrar em DRAW salva o estado anterior da câmera na ViewportSession, sem criar segunda câmera autoral.
2. Ativar alinhamento planar ajusta a vista, não transforma objetos nem o Workplane persistido de um PlanarShape.
3. Mudar Workplane não move câmera automaticamente; uma ação explícita **Align View to Workplane** alinha, mantendo a regra já aprovada em Snapping.
4. Sair de DRAW restaura a navegação anterior quando solicitado pelo comportamento de troca de perfil, sem perda de zoom/estado e sem afetar outras viewports.
5. Cada split viewport guarda sessão própria; planos e visibilidade compartilham apenas o que for decisão de usuário explícita.
6. Pointer e teclado são traduzidos por keymap para intents semânticos; Alt não é exceção codificada na ferramenta.

## Selection e operação contextual

Object/Vertex/Edge/Face permanecem os modos globais para mesh; pontos e segmentos de spline são seleção **contextual** da ferramenta, sem um modo global "Shape/Curve/Region" competindo com Mesh. A seleção ativa sempre explicita o tipo, origem e as capacidades da operação; não tenta convertê-la automaticamente.

Hover, active, selected, locked, disabled e invalid seguem o contrato visual único. NumericField, click-move-click, zoom, snapping e undo não mudam de semântica ao alternar DRAW/POLY.

## Sessão e commits

1. UI reconhece gesto → UiIntent semântico.
2. Application inicia ToolSession com Workplane, selection, parâmetros e revisão do Document congelados para o gesto.
3. Geometry executa queries/algoritmos puros e retorna preview + diagnósticos.
4. Confirmar produz Command/Transaction único e invalida caches derivados.
5. Cancelar/Escape retorna ao snapshot anterior, sem registro no Undo.
6. Se a fonte foi modificada por outra operação, descartar preview obsoleto, explicar conflito e exigir reavaliação.

## Plano de migração incremental

| Passo | Estratégia | Critério |
|---|---|---|
| 1. Inventory | Mapear pontos de duplicação em ProfileState, Poly Pen, curves, tool callbacks | tabela Reuse/Refactor/Move/Rewrite |
| 2. Workplane | Reutilizar motor de inference e unificar Workplane | snapping e coord. consistentes |
| 3. Curves | Bezier Pen passa a autorar SplineResource | sem pipeline ProfileState → BezierPath |
| 4. Planar shapes | Representação persistente PlanarShape + holes como SplineId | round-trip com IDs |
| 5. Commands | Unificar IDs de ferramentas, undo e intents | mesma operação nos dois perfis |
| 6. Navigation | DRAW como navegação planar em ViewportSession | ida/volta previsível |
| 7. Cleanup | Deprecar estado autoral paralelo apenas após consumidores migrados | ausência de regressão funcional |

Não remover ferramentas, parâmetros, saved projects ou layouts para simplificar essa migração. Qualquer conversão de documento legado precisa ser explícita, versionada e testada.

## Gates obrigatórios

- DRAW → POLY → DRAW preserva geometria, splines, IDs, seleção e Undo.
- Mesmo comando produz mesma geometria com mesmos parâmetros nos dois perfis.
- Editar curvas não as converte em mesh sem ordem.
- Splines com holes, formas em Workplane local e assets legados sobreviverão a save/load.
- Keymaps remapeados, viewport split, caneta, mouse, teclado e click-move-click não divergem.
- High Contrast, leitura de foco, nomes e estados acessíveis permanecem funcionais.
- Testar planar alinhado a face oblíqua e troca para câmera livre sem movimento implícito de objeto.
- Performance dos previews permanece aceitável em hardware mínimo; medir antes de otimizar.

## Não objetivos

Sem ECS, editor CAD geral com constraints, segundo scene graph, segundo Geometry Engine ou reescrita integral de ferramentas saudáveis. Não remover o nome DRAW/POLY da UI: unificação interna não implica forçar um fluxo único ao usuário.

## Decisões fechadas

1. **DRAW/POLY: perfis, não motores nem documentos separados.**
2. **Planar Navigation** é a interação principal do DRAW e não altera Workplane ao alinhar a câmera.
3. Curvas persistem como SplineResource; formas persistem como PlanarShape.
4. Selection, commands, snapping, sessions, IDs, render e Undo são compartilhados.
5. Migração incremental com gates por vertical slice; remover duplicação somente após equivalência comprovada.
