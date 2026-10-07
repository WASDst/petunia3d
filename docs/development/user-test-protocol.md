# Protocolo de teste com usuários — DRAW, POLY e acessibilidade (Onda 6)

Escopo: validar com pessoas reais as decisões do [ADR 007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md)
que os testes automáticos não conseguem decidir. Base de método: capítulo 46
§8.5 (acessibilidade motora e cognitiva) e as matrizes das Ondas 1–5.
Este documento é um roteiro de sessão; não cria requisito novo.

## 1. Perguntas que o teste precisa responder

| # | Decisão hoje | Alternativa | Onde está |
| --- | --- | --- | --- |
| Q1 | Workspace de modelagem abre em **POLY** | Abrir em **DRAW** para quem começa | [Onda 4, parte 2](draw-regions-gap-matrix.md) |
| Q2 | Plano automático pelo eixo dominante da vista (estilo Modo/C4D) | Favorecer o chão (estilo SketchUp) — já é preferência | [Onda 3](snap-inference-gap-matrix.md) |
| Q3 | Raio do snap padrão **12 px** | 16–20 px como padrão | [Onda 3](snap-inference-gap-matrix.md), preferência "Raio do snap" |
| Q4 | Luz de estúdio **acompanha a câmera** | Luz fixa no mundo — já é preferência | [Onda 6](wave6-visual-accessibility-gap-matrix.md) |
| Q5 | Perfil continua no documento depois do Push/Pull (a região volta a aparecer no hover) | Consumir o perfil (Plasticity mantém; SketchUp consome) | [Onda 4](draw-regions-gap-matrix.md) |
| Q6 | Ctrl como "ação alternativa" do Poly Pen (extrudar aresta, derreter ponto) | Outro modificador ou menu radial | [Onda 5](poly-pen-gap-matrix.md) |
| Q7 | Snap desligado por padrão (liga com Ctrl ou na barra) | Snap contextual ligado por padrão (capítulo 01) | [Onda 3](snap-inference-gap-matrix.md) |

## 2. Participantes

- **12 pessoas no mínimo**, em três grupos de 4:
  1. nunca modelaram em 3D;
  2. usam outro modelador (Blender, SketchUp, Plasticity, C4D ou Modo);
  3. têm limitação motora nas mãos (tremor, pouca força, amplitude reduzida) ou
     usam mouse adaptado/trackball — grupo exigido pelo capítulo 46 §8.5
     (Findlater et al., 2010; Wobbrock et al., 2011).
- Termo de consentimento, anonimização dos dados e direito de parar a qualquer
  momento. Gravar tela e voz só com autorização explícita.

## 3. Preparação

- Build de release do branch sob teste; mesmo monitor para todos (100% de
  escala) e um segundo teste curto em 150% (captura nativa pendente das
  Ondas 1–6).
- Preferências limpas (arquivo `preferences.toml` removido) para medir os
  padrões; o grupo 3 repete as tarefas 3 e 5 depois de ajustar
  "Distância para começar a arrastar", "Arrastar sem segurar o botão" e
  "Raio do snap".
- Contrabalançar a ordem DRAW/POLY entre participantes (metade começa por
  cada um) para Q1.

## 4. Tarefas (moderador lê em voz alta, sem nomes técnicos)

| # | Tarefa | Mede | Pergunta |
| --- | --- | --- | --- |
| T1 | "Desenhe um retângulo no chão e transforme-o numa caixa de 2 m de altura." | tempo, erros, uso de valor digitado | Q1, Q2 |
| T2 | "Na frente do cubo, desenhe uma janela e afunde-a 20 cm." | sucesso do Draw on Face + Push/Pull, Undos | Q2, Q5 |
| T3 | "Desenhe dois círculos que se cruzam e levante só a parte em comum." | reconhecimento de regiões | Q5 |
| T4 | "Arraste este canto do cubo até encostar neste outro ponto." | acertos de snap, submovimentos | Q3, Q7 |
| T5 | "Com a caneta, crie uma aba a partir desta borda e depois apague este ponto." | descoberta do Poly Pen e do modificador | Q6 |
| T6 | "Gire a vista até olhar o objeto por trás e diga onde está a face que você afundou." | leitura de forma | Q4 |
| T7 | "Desfaça as três últimas ações e refaça uma." | modelo mental de gesto = 1 Undo | geral |

## 5. Métricas

- **Por tarefa:** tempo até concluir, sucesso (sim/parcial/não), número de
  Undo/Esc, número de cliques errados e de pedidos de ajuda.
- **Por tarefa, logo depois:** SEQ (*Single Ease Question*, 1–7).
- **No fim:** SUS (*System Usability Scale*, 10 itens) e três perguntas
  abertas: o que foi mais fácil, o que confundiu, o que faltou.
- **Grupo 3:** mesmas medidas antes e depois do ajuste das preferências;
  registrar que ajuste cada pessoa escolheu (dado para Q3).
- Pensar em voz alta durante as tarefas; o moderador só intervém após 2 min
  sem progresso.

## 6. Critérios de decisão

- Q1: abrir em DRAW se o grupo 1 concluir T1 ao menos 25% mais rápido começando
  por DRAW, sem piorar o grupo 2.
- Q2/Q4/Q7: trocar o padrão se mais de metade dos participantes mudar a
  preferência espontaneamente ou errar a tarefa por causa dela.
- Q3: adotar como padrão a mediana do raio escolhido pelo grupo 3, se ela não
  aumentar erros de snap do grupo 2 em T4.
- Q5/Q6: decidir pela opção com menos pedidos de ajuda; empate mantém o atual.
- Toda decisão volta ao caderno (capítulo 36 ou 45) e à matriz da onda, com os
  dados agregados; dados individuais não entram no repositório.

## 7. Extensão — refatoração de UI (plano 2026-10-04, F8)

Mesmo formato das seções 4–6, aplicado ao shell depois das fases F1–F7. Mede
a meta do plano (§11): **criar forma, extrudar e pintar sem ajuda em ≥ 4 de 5
participantes**.

| # | Tarefa | Mede | Sucesso |
| --- | --- | --- | --- |
| T8 | "Comece um modelo novo." (a partir da Home) | primeira impressão, caminho até a viewport | chega à viewport em < 30 s sem ajuda |
| T9 | "Crie uma caixa e puxe uma das faces para fora." | descoberta da barra contextual por seleção (Extrude só aparece com face) | sem pedido de ajuda |
| T10 | "Pinte uma faixa num lado da caixa, espelhada do outro lado." | barra de pincel (cor, tamanho, simetria) e trilho do PAINT | faixa espelhada sem abrir o Inspector |
| T11 | "Troque a ferramenta de seleção pelo laço." | grupos com flyout no trilho (botão direito / marca de canto) | encontra em < 60 s |
| T12 | "Mude a cor dos botões ativos para azul." | Preferências: busca e cor de destaque | encontra a opção pela busca ou pela aba |

- **Registro:** além das métricas da seção 5, anotar quem usou a busca de
  comandos, o botão direito e o modo "Mostrar nomes no trilho".
- **Decisão:** T9–T11 com sucesso em < 4 de 5 reabrem a decisão correspondente
  (D2, D6/F5b) no plano de UI; resultado agregado vai para o registro da F8.
