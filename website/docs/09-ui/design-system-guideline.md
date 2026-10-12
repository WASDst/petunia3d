# Design system — guideline visual do Petunia3D

> **Approved, in migration — 2026-10-11.** Direção aprovada pelo usuário nesta retomada: fundamentos de edição do Spectrum, pílulas seletivas, ícones sucintos e profundidade suave inspirada nas referências fornecidas. A composição do software de referência não é uma especificação para o layout do Petunia. Implementação acompanhada por U08; aceite visual, acessibilidade e plataforma permanecem gates independentes.

## 1. Intenção e limites

O conteúdo produzido recebe a maior atenção. A moldura é neutra, legível e compacta; o acento identifica escolhas e o relevo ajuda a reconhecer controles. O [macro-layout e as regiões semânticas](./workspaces-feedback-accessibility.md) continuam com a mesma autoridade.

Este capítulo governa o acabamento dos componentes existentes: cores, tipografia, raios, ícones, bordas, relevo e estados. Não introduz outro frontend, reorganização das regiões, registries, efeitos de GPU ou uma nova fonte de estado autoral. `DesignTokens` é a projeção visual; os temas canônicos continuam em `petunia_config` e nos packs oficiais.

## 2. Superfícies e cores

- Painéis e campos usam cinzas neutros; claro e escuro compartilham a mesma linguagem. Isso mantém uma moldura estável ao trabalhar com materiais, PAINT e UV.
- Lilás é o acento padrão da interface; a preferência de acento e o tema High Contrast continuam disponíveis.
- Selected, Active e hover da geometria preservam seus códigos funcionais próprios. Cor de aviso/erro não é decoração.
- Campos, moldura, overlays e conteúdo se distinguem por luminosidade, borda e posição já existente, sem multiplicar cartões em cada seção.
- A amostra de cor mostra o RGB real. Gradientes de acabamento nunca modificam a amostra, o canvas, a textura ou a cor autoral.
- Contraste de texto operacional: pelo menos 4,5:1 nas combinações suportadas. Foco/estados essenciais precisam de indicador reconhecível e alternativa à cor. High Contrast usa superfícies sólidas e bordas explícitas.

## 3. Formas e relevo

| Uso | Tratamento |
|---|---|
| Escolha curta, workspace e opção segmentada | Pílula compacta; raio limitado à metade da menor dimensão. Item ativo tem preenchimento distinto. |
| Botão de ferramenta | Cantos suaves; ícone preserva silhueta. Estado ativo, hover e foco continuam distinguíveis. |
| Campo numérico, texto e lista | Cantos menores, borda legível e superfície discretamente encaixada. |
| Painel fixo | Tom neutro e borda sutil; sombra não é necessária em cada seção. |
| Menu, popover e overlay | Elevação localizada sobre o conteúdo; foco de teclado não depende da sombra. |

O relevo é curto e consistente: borda superior discretamente iluminada, borda inferior mais escura e diferenças pequenas de tom. Pressed pode parecer levemente rebaixado. Não usar refração, backdrop blur nem animação contínua para obter essa sensação. O acabamento passivo não recebe input, não entra na árvore semântica como controle e não altera a posição do conteúdo.

## 4. Tipografia, ícones e densidade

- Caption/auxiliar operacional: **12 logical px**; pequeno: 12; principal: 13; destaque: 14; excepcional: 16. Escala da UI e métricas reais ainda precisam de aceite em 100–200%.
- Campo técnico permanece legível e digitável; não compensar falta de espaço reduzindo letras abaixo do mínimo. Clipping e rótulos em português são regressões a verificar.
- Pesos Regular/Medium predominam; títulos de seção têm hierarquia moderada. Preservar números e unidades necessários à precisão.
- Família de ícones consistente, traço uniforme e ajuste óptico de tamanho. Ícones de controle em torno de 16 px e de ferramenta em torno de 20 px, dentro de alvos operacionais de pelo menos 28 px.
- Tooltip e nome semântico complementam a silhueta. O foco visível é independente de ferramenta ativa e de seleção de objeto.
- Espaçamento segue os tokens 4/8/12; pílulas não exigem aumentar todas as margens nem alterar a geografia do editor.

## 5. Seletores de cor

O padrão de referência é anel de matiz com área triangular de saturação/valor, marcador visível e amostras precisas. O seletor e as entradas numéricas representam uma única escolha de cor; preto, branco e cores escuras precisam ser alcançáveis. RGB/HEX e alternativas de teclado devem continuar disponíveis quando o contexto as oferece.

Reconhecimento do input fica no componente; conversão e validação são funções puras Rust. Hover não deve confirmar cor. Cancel/restauração preservam o contrato do contexto que abriu o picker. Escolhas autorais precisam seguir Commands/Undo; um avanço visual não comprova essa migração de domínio.

O slice inicial alcança o popover real de cor de seleção da parte. Outros controles de cor serão migrados progressivamente, sem afirmar que já usam o novo picker. A rota legada `set_part_selection_color` ainda precisa de comando tipado/Undo e é uma dívida explícita do boundary.

## 6. Estados e movimento

Cada componente distingue default, hover, pressed, active/selected, foco e disabled. O anel de foco permanece claro sobre superfícies com relevo; a ferramenta ativa continua identificável enquanto outro campo recebe foco.

Transições de acabamento são breves e locais. As durações existentes 100/140/180 ms continuam controladas pelo token de movimento reduzido. Feedback de edição e valor numérico não aguarda animação. High Contrast remove o relevo opcional; a preferência Reduced Motion remove transições, preservando os indicadores estáticos.

## 7. Referências e aceitação

Referências de fundamentos: [Spectrum — Attention hierarchy](https://spectrum.adobe.com/foundations/composition/attention-hierarchy), [Spectrum — Editing contexts](https://spectrum.adobe.com/foundations/color/background-layers), [Spectrum — Grays](https://spectrum.adobe.com/foundations/color/grays), [Fluent — States and focus](https://fluent2.microsoft.design/color), [Fluent — Materials](https://fluent2.microsoft.design/material). São referências de design, não bibliotecas a instalar ou autoridade para mudar contratos do editor.

Antes de fechar U08: conferir componentes representativos nos temas claro/escuro/High Contrast, inglês/português, escalas 100–200%, mouse/teclado, foco, overflow e contraste. Para o picker, verificar HSV/RGB/HEX e markers, drag/release/cancel, cores acromáticas/escuras e interação numérica. Compilação, testes comportamentais e aceite nativo são evidências separadas.

Estado e comandos efetivamente executados desta retomada ficam no [Slint Rescue §28](./slint-rescue-plan.md#28-retomada-codex-guideline-visual-e-application-2026-10-11). Nenhum gate manual ou percentual de implementação é inferido deste guideline aprovado.
