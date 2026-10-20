# Roteiro de teste manual do Shell v2 (o que checar ao abrir o app)

> Status: **roteiro vivo** (2026-10-19). Cada item é algo que **ninguém** pode verificar por automação: precisa de olho, mouse e teclado no app real. Os gates automatizados (`cargo test`, `shell_v2` offscreen) já cobrem o resto — este arquivo é a parte que fica com a pessoa.
>
> Como usar: siga em ordem (cada bloco depende do anterior). Marque o que falhar com uma captura; isso vira a próxima fila.

## 0. Antes de tudo (2 min)

1. Abra o app e **confirme que ele abre sem janela extra** e que a viewport mostra o cubo.
2. **Redimensione a janela** de 1280 px até 1920 px: nenhuma região pode sumir, sobrepor ou ficar cortada.
3. **Tab** a partir do topo: o foco tem de andar em ordem previsível (barra → trilho → viewport → painéis → rodapé), sempre **visível**.
4. Ative **movimento reduzido** nas Preferências: nada pode depender de animação.

## 1. Importação (a que estava quebrada)

5. **Arquivo → Importar → Modelo 3D** → traga um **OBJ com `.mtl` + textura PNG**: a cor e a textura precisam aparecer.
6. Repita com um **`.glb`** (textura embutida) e um **`.gltf`** (textura externa): mesmo resultado.
7. Traga um **OBJ com dois objetos**: devem entrar **dois** assets, cada um com nome próprio.
8. Traga um arquivo que **não é modelo**: deve aparecer recusa clara, não sucesso vazio.
9. **Ctrl+Z** depois de importar: o asset precisa sumir (import é desfazível).

## 2. Paint — Camadas (C1)

10. No painel **Camada ativa**: mude **opacidade** e **modo de mistura** e veja o efeito na pintura.
11. **Alvo do pincel → Máscara da camada**: pinte; a parte pintada da máscara deve **limitar** a camada.
12. Em **Efeitos**: adicione "Inverter cores" → a camada inverte; ligue/desligue; mude a **opacidade do efeito**; adicione **máscara no efeito**.
13. **Máscara da camada**: criar → **inverter** (checkbox) → remover.

## 3. Paint — Projeção e isolamento (C2)

14. Deve existir **uma só** ferramenta "Projeção" (sem "Decal" separado).
15. **Só onde está selecionado**: pintar fora da seleção não pinta nada.
16. **Isolar para pintura**: os outros objetos ficam em **Raio‑X (fantasma)**, não escondidos; ao **desligar**, o Raio‑X que você tinha antes volta.

## 4. Gradiente (C3)

17. **Efeitos → Nova camada de gradiente (linear)**: a camada aparece na Estrutura.
18. Edite **Tipo** (linear/radial/angular/refletido/diamante), **paradas** (posição e cor) e **Início/Fim** X e Y.

## 5. Modos das operações (C8)

19. Selecione várias faces → **Extrudir** → troque o modo no **HUD**:
    - **Região**: a seleção sobe junto e a malha continua fechada;
    - **Faces individuais**: cada face se separa, todas na mesma direção;
    - **Ao longo das normais**: cada face na sua própria normal.
20. **Inset**: modo **Região** e **Individual**. O modo fica **lembrado** para a próxima operação.

## 6. Nós (C4)

21. No grafo de **materiais**: arraste de uma saída e **solte no vazio** → abre o menu de nós; o nó novo nasce **ligado**, onde você soltou. Solte num tipo incompatível: tem de **recusar** sem deixar nó órfão.
22. No grafo de **objetos**, solte um fio no vazio → abre o **picker de objeto**: busque por texto, use setas e Enter.
23. Crie uma **Booleana**: o objeto-ferramenta (o operando) aparece **esmaecido** no grafo; passe o mouse para ler a dica.

## 7. Referência (C7)

24. Carregue uma imagem de referência (painel **Referências**) e veja a lista de **Partes**: o grupo **"Referência"** aparece **no topo**, com a imagem dentro.
25. O **olho** da linha mostra/esconde; o **botão direito → Apagar referência** remove.
26. Girar, mover no eixo e escalar continuam no painel de Referências.

## 8. Wireframe e viewport

27. No **Sólido**, as arestas de trás **não "estouram"** nas bordas. No **Wireframe** tudo aparece (é Raio‑X por definição); arestas abertas continuam aparecendo.

## 9. Assets (C6, parcial)

28. A biblioteca de assets é uma **grade de miniaturas**; a busca e o clique para inserir continuam funcionando.

## 10. Fechamento

29. **Salvar e reabrir** o projeto: as camadas, máscaras, efeitos, gradiente e o modo de extrusão precisam voltar.
30. Se algo falhou, anote **o que** (passo), **como reproduzir** e **uma captura** — é isso que vira a próxima fila.

## O que ainda não existe (para não procurar)

- **Aba de Curvas** no Animate (C5) — a curva ainda não é editável.
- **Arrastar asset para a viewport**, **tags** e **salvar grupos** na galeria (C6) — dependem de P04/G09.
- **Timeline estilo motion design** (C5).
