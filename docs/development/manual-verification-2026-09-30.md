# Roteiro de verificação manual — rodada de 2026-09-30

Tudo abaixo passou em teste automático (headless), **mas nada foi visto numa janela real**:
este ambiente não tem captura nativa. Cada linha diz o que fazer e o que deve acontecer;
se algo divergir, anote o passo — é evidência para a matriz correspondente.

## PAINT — seleção, isolar, máscara, restrição

| # | Passo | Esperado |
| --- | --- | --- |
| 1 | Crie dois objetos (ex.: cubo e esfera). Entre no PAINT (`Ctrl+3`). | A ferramenta ativa é o **pincel** (não Select). |
| 2 | Escolha **Select** (`V`) e clique no outro objeto. | O outro objeto vira o ativo ("Painting '…'"); o pincel passa a pintar nele. |
| 3 | Com Select, clique numa face do objeto ativo; `Shift` soma; duplo clique numa face. | Face selecionada; `Shift` acumula; duplo clique seleciona a **ilha UV** inteira. Clicar no vazio limpa. |
| 4 | Clique no botão de olho (**Isolar**) ou aperte `/`. | Só o objeto ativo fica visível; clicar de novo mostra todos. Trocar de objeto com isolamento ligado mantém só o novo visível. |
| 5 | Ligue **Mask to selection**, selecione **uma** face e pinte por cima de várias. | Só a face selecionada recebe tinta (3D **e** no canvas 2D). Sem seleção e com a máscara ligada, **nada** é pintado. |
| 6 | Desligue a máscara e pinte numa face de um cubo. | Só a face pintada (e vizinhas dentro do raio do pincel) muda; a face oposta **não** recebe tinta. |
| 7 | Escolha a ferramenta **Fill** (balde). | Nada muda até você clicar (antes, escolher Fill pintava o objeto todo). |
| 8 | Trave o pincel na "primeira face" e arraste atravessando faces. | A tinta fica só na face onde o traço começou. |
| 9 | Pinte por cima de outro objeto que cobre o ativo. | Não pinta através do objeto da frente. |

## PAINT — pincéis e ferramentas novas

| # | Passo | Esperado |
| --- | --- | --- |
| 10 | Em Brush settings, troque o tipo: Spray, Smudge, Blur, Dodge, Burn, Clone. | Cada um age sobre o que já está pintado (Smudge arrasta, Blur suaviza, Dodge/Burn clareiam/escurecem). |
| 11 | Clone: `Ctrl+clique` na origem; depois arraste em outro ponto. | Copia a textura da origem. Sem origem, o primeiro clique a define. |
| 12 | Ponta quadrada/losango, ângulo, achatamento; jitter/espalhamento; estabilizador. | A pegada muda; o traço fica mais suave com o estabilizador; mesmo traço = mesmo resultado. |
| 13 | Fluxo baixo + opacidade 50%: passe várias vezes no mesmo ponto dentro de **um** traço. | Não escurece além de 50% (flow acumula, opacity limita). |
| 14 | Salve um pincel ("Save brush"), reabra o app. | O preset volta na lista (`brush-presets.json`). |
| 15 | Elipse, Gradiente linear e **radial** (rail). | Elipse inscrita; gradientes esmaecem para transparente **sem apagar** a pintura abaixo. |
| 16 | Camadas → botão de imagem → escolha um PNG/JPEG. | Vira camada de decalque com a proporção da imagem. Arquivo inválido mostra mensagem e não altera nada. |
| 17 | Adicione o efeito **Levels**. | Aparecem os 5 parâmetros e o resultado compõe. |

## DRAW

| # | Passo | Esperado |
| --- | --- | --- |
| 18 | Desenhe um retângulo/círculo e troque de ferramenta. | A forma **continua visível** (tinta leve + contorno). |
| 19 | Com o Sketch, clique no contorno de uma forma antiga. | Ela volta a ser a forma em edição. `Ctrl+clique` força um ponto novo. |
| 20 | Num perfil fechado, clique **na aresta**. | Entra um nó novo sem deformar a forma; dá para arrastá-lo na hora. |
| 21 | Duplo clique num nó; ou selecione o nó e use o botão de curvas. | Alterna **reto ↔ curva** naquele ponto. Sem seleção, o botão age em todos. |

## Atalhos, painéis e controles

| # | Passo | Esperado |
| --- | --- | --- |
| 22 | Troque de ferramenta duas vezes e aperte `Space` (sem estar num gesto). | Alterna com a ferramenta anterior. `Shift+Space` abre o micro-inspector. |
| 23 | `Ctrl+1`, `Ctrl+2`, `Ctrl+3`, `Ctrl+4`. | DRAW, POLY, PAINT, UV. |
| 24 | Configurações → Teclado. | Lista vertical de perfis (nomes inteiros); "Novo perfil a partir do atual". |
| 25 | Clique num atalho e pressione outra combinação; `Esc` cancela. | A ação passa a usar a combinação; conflito aparece em amarelo; editar um perfil embutido cria uma cópia sua. |
| 26 | Arraste o cabeçalho de Configurações / Reference Manager / Canvas. | Move suavemente, sem "travar". |
| 27 | Arraste o canto inferior direito desses painéis; duplo clique no canto. | Redimensiona; duplo clique volta ao padrão. |
| 28 | Configurações → Ferramentas: arraste os sliders (intervalo de duplo toque, limiar de arrasto, raio do snap); reabra a janela. | Acompanham o valor salvo; o texto ao lado confere com a posição. |
| 29 | Poly Pen: clique numa aresta (sem polígono em andamento). | Entra um ponto na aresta (1 Undo). |

## O que **não** foi feito (para não procurar)

- Layout de 1/2/4 vistas (só estudo e modelo puro); SVG como decalque; Projection/Stencil; Path Paint.
- Índice espacial para picking/snap; coalescer render por quadro; alças ≥ 24 px (agente de UI).
- Gates completos (`cargo test --workspace`, clippy, `docs-check`, `bible-check`, `ui-guard`): aguardam o seu "pode rodar".
