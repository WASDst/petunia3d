# P3D-171 — Animation Ghosts & Trajectories

<aside>
👻

Estado: **SPEC DRAFT (2026-09-30)** · Prioridade: P2 · Era 3. Camada 3 (Posar) do Animate ([cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)). Overlay de **leitura**: nunca altera o documento.

</aside>

## Objetivo

Mostrar, sobre o viewport, as poses de frames vizinhos (**Ghosts**) e o caminho percorrido por ossos escolhidos (**Motion Trails**), para que o artista julgue tempo, espaçamento e arcos sem abrir editores de curvas.

## Ghosts

Silhueta translúcida da malha (ou dos ossos) nas poses anteriores e seguintes ao frame atual.

| Modo | Comportamento |
| --- | --- |
| Off | sem ghosts |
| Past | apenas frames anteriores |
| Future | apenas frames seguintes |
| Both | anteriores e seguintes |
| Selected Frames | somente os frames selecionados na timeline |
| Keys Only | restringe qualquer modo acima aos keyframes/pose markers |

Controles: quantidade de ghosts antes e depois, **passo** (a cada N frames), queda de opacidade com a distância temporal e **Bones Only**.

- **Funciona sobre Motions vivos:** ghosts amostram `pose(t)` (P3D-170), não dependem de haver keys.
- **Orçamento:** o número máximo de ghosts é limitado; acima do orçamento de triângulos o modo cai automaticamente para **Bones Only**, com aviso. Meta a validar por benchmark em PC modesto (P3D-126).
- **Acessibilidade:** passado e futuro **não se distinguem só por cor**. Além dos tokens `ghost.past` e `ghost.future`, usam-se contorno (sólido versus tracejado) e opacidade, e o alto contraste (cap. 36) redefine os tokens.

## Motion Trails

Caminho de um ou mais ossos/juntas ao longo de um intervalo, com marcas de frame a cada N frames e clique para saltar ao frame. V1 é **somente leitura**; editar o caminho arrastando fica como candidato posterior.

## Reference ghost

Quando há Reference Frames ([P3D-172](p3d-172-reference-image-sequence.md)), o frame de referência pode aparecer como camada de comparação com o mesmo mecanismo de opacidade.

## Boundaries

| Camada | Responsabilidade |
| --- | --- |
| Dados | as **configurações** de ghosts/trails são estado de sessão/preferência por workspace (como o layout do cap. 36), **não** dados do documento |
| Algorithm | amostra `pose(t)` por frame e produz matrizes de skinning e pontos de trilha |
| Renderer | recebe um `GhostDescriptor` neutro (matrizes + estilo); `PetuniaRenderer` não conhece Slint nem egui |
| UI | `CommandId` (`view.ghosts.toggle`, `view.ghosts.mode`, `view.trails.toggle`…), `TextId`, `IconId`, `ThemeToken`; nenhuma tecla física fixa |

## Não objetivos

Editar a trajetória diretamente, simulação, ghosts de múltiplos assets, ghosts de cena.

## Testes / DoD

1. Cada modo seleciona os frames corretos (incluindo Keys Only e limites do clipe/loop).
2. Ghosts de um Motion vivo e de um clipe com keys coincidem com `pose(t)`.
3. O orçamento aplica o fallback para Bones Only e sinaliza.
4. Ghosts não alteram revisão nem histórico do documento.
5. Alto contraste e leitura sem cor (captura verificada em janela real).
6. Trails: pontos e marcas de frame corretos; clique salta ao frame.
7. Teste do renderer: descritor neutro, sem tipo de UI.

## Dependências

P3D-135, P3D-170 (para Motions vivos), P3D-012/P3D-010 (shading e overlays), cap. 36 (acessibilidade).
