# DRAW — regressões de interação reportadas em 03/10/2026

Autoridade: [workflow shape-first](../bible/foundations/02-workflow-modelagem-shape-first.md), constituição 11 e relato visual do usuário após PR #27. Escopo: nove defeitos relatados; site público congelado.

| Requisito | Estado inicial | Evidência / delta necessário |
| --- | --- | --- |
| Dica ocultada sobre painel | BROKEN | HUD não depende do hover livre da viewport |
| Parede completa na extrusão | BROKEN | `effective_points` concatena laços externo/interno num polígono único; usar região com furo |
| Opções por forma selecionada | BROKEN | Profile inclui threshold/radius/angle/tolerance globais sem relação com retângulo |
| Retângulo/círculo por arrasto | FUNCTIONAL_BUT_DIFFERENT | Botões chamam criação imediata, sem ToolSession |
| Mover/escalar perfis 2D | PARTIALLY_COMPLIANT | Bridge possui comandos e gestos; roteamento do shell e seleção precisam de regressão integrada |
| Nós e manipulação na viewport | BROKEN | Caminho de nó só aceita draw_profile; perfil fechado roteia arrasto para depth |
| Plano Face comum às ferramentas | BROKEN | Botão exige face já selecionada e falha muda o plano para Ground; precisa permitir escolher face na viewport |
| Recolher opções | BROKEN | `visible=false` do conteúdo mantém preferred-height; card precisa constraint de altura recolhida |
| Loop Cut após booleana | PARTIALLY_COMPLIANT | Ring exige quads em todo percurso; booleanas geram n-gons/triângulos e o hover ignora erros |
| Ctrl + atalho | PARTIALLY_COMPLIANT | Auditar eventos reais, normalização do teclado e prioridade de gesto/keymap |

Critérios: ferramenta persistente, arrasto/clicar-mover-clicar pelo ToolSession, plano conhecido sem mover câmera, uma entrada Undo por gesto, Esc restaura, edição possível sem volume, nenhuma mutação ao apenas escolher ferramenta. Evidência final será adicionada após testes.


## Comportamento implementado

- Retângulo: primeiro canto → arrasto até o canto oposto. Círculo: centro → raio. A escolha da ferramenta não cria dados. ToolSession fornece limiar, clicar-mover-clicar, confirmação e Esc; o documento só muda no commit.
- Select edita nós/alças e arrasta o corpo do perfil; Move/Rotate/Scale operam no perfil selecionado. Nenhum volume é necessário. Retângulo/círculo mostram dimensões e círculo mostra segmentos; Pen mantém controles de fechamento/curvas/volume pertinentes.
- Auto/Ground/View/Face são compartilhados. Face usa a seleção existente ou arma uma escolha explícita na viewport; clique vazio continua aguardando, Esc cancela e câmera não muda.
- Parede extrudada usa laço externo + furo, sem concatenar os dois como um polígono simples. Teste em ambos os windings confirma 8 paredes e incidência 2 em toda aresta.
- Loop Cut mantém o percurso de quads. Na ausência dele, um sólido fechado e conectado admite seção normal à aresta escolhida, com cuts/slide/balanced e interpolação de atributos. Esse caminho triangula o snapshot para tratar n-gons côncavos; não adiciona tampa interna. Patches abertos, componentes desconectados e malhas não manifold continuam com falha previsível, sem mutação. A prévia é visível antes da confirmação.

## Evidência parcial

`cargo test -j 1 -p petunia_mesh -p petunia_module_model --lib`: 184 + 7 aprovados.
Regressões novas: parede completa/fechada; três operações booleanas com prévia,
slide, fechamento, material, loops selecionados e recusa de snapshot obsoleto.
Resultados do shell e gates finais serão registrados ao concluir.

## Aceite visual nativo pendente

O ambiente de validação não possui display gráfico; não foram produzidas
capturas nativas. O backend de teste do Slint exercita eventos de ponteiro e
teclado com o bridge de produção, mas não certifica aparência/oclusão nativa.

1. Rectangle/Circle: escolher a ferramenta não cria dados; arrastar ou
   clicar-mover-clicar cria uma forma no plano escolhido. Repetir compõe outra forma.
2. Antes de extrudar, Select move o corpo e edita nós; Move/Rotate/Scale operam
   na forma. Esc restaura a edição; Undo desfaz um gesto inteiro.
3. Alternar Auto/Ground/View/Face em Pen, Rectangle e Circle. Face sem seleção
   aguarda um clique na face; a câmera permanece imóvel. Trocar o plano de uma
   forma selecionada mantém a identidade; Undo restaura o plano.
4. Confirmar opções de Rectangle/Circle/Pen e ausência de parâmetros alheios.
5. Passar sobre Inspector/opções oculta a dica; recolher o card deixa só o
   cabeçalho. Expandir recupera controles e altura.
6. Inspecionar as quatro paredes de um retângulo 2 × 1,5, depth 2,7 e wall
   thickness 0,15. Girar a câmera para conferir paredes internas e externas.
7. Na vista frontal, o corpo move em Select e a alça de profundidade separada
   gera uma prévia de volume; Esc cancela essa prévia.
8. Aplicar Fuse/Cut/Intersection em sólidos sobrepostos; Loop Cut mostra a
   seção, permite cuts/slide e confirma com um Undo. Conferir oclusão e UVs.
9. Ctrl+R, Ctrl+B e Ctrl+Z chegam ao keymap; soltar Ctrl não mantém snap preso.
