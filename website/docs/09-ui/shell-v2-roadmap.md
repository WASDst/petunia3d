# Shell v2 — roteiro de Poly, Paint, UV, Animate, Nós e Assets (2026-10-18)

> Status: **aprovado** ([ADR-C16](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuario)). A Onda A está em implementação; B e C seguem depois dela. Referências de outros softwares (Blender, Photoshop, Affinity, Cavalry, Vegas) servem de **referência de ergonomia**, nunca de código ou recurso copiado.

## Intent

Transformar as observações de uso do usuário em um plano por onda: o que é pequeno e concreto (A), o que é alicerce compartilhado (B) e o que é grande (C).

## Sources

Pedido do usuário de 2026-10-18, [refinamento do Shell v2](shell-v2-refinement.md), [Operation Graph](../03-geometry/operation-graph.md), [Paint](../06-paint/workspace-ux.md), [UV](../07-uv/workspace-ux.md), [Animação](../10-animation/workspace-ux.md).

## Onda A — pequenas e concretas

| ID  | Item                                                                                          | Status          |
|-----|-----------------------------------------------------------------------------------------------|-----------------|
| A1  | Apagar ou tirar o último membro de um grupo remove o grupo (um passo de Undo)                  | [-] IN PROGRESS |
| A2  | Renomear grupo descobrível (F2 e duplo clique) e cor de seleção por grupo (+ forçar)          | [-] IN PROGRESS |
| A3  | Pivô: deslocamento não destrutivo, Editar pivô modal (Enter/Esc) e menu de botão direito       | [-] IN PROGRESS |
| A4  | Menu de botão direito do 3D Cursor (origem, seleção, ponto sob o mouse, origem para o cursor)  | [-] IN PROGRESS |
| A5  | Nós: navegação infinita (pan em qualquer direção, zoom) e "Enquadrar tudo"                    | [-] IN PROGRESS |
| A6  | Paint/UV: alternar wireframe sobre a pintura 2D                                                | [-] IN PROGRESS |
| A7  | UV: mover várias ilhas e partes de uma vez                                                     | [-] IN PROGRESS |
| A8  | Remover a opacidade duplicada do Paint                                                         | [ ] TODO (depende de B1: a opacidade passa para o seletor de cor) |

## Onda B — alicerces

| ID  | Item                                                                                       | Status   |
|-----|--------------------------------------------------------------------------------------------|----------|
| B1  | `PetuniaColorPanel` único (roda, modos, RGB/HSL+alfa, hex, paletas CRUD) e cor fixa no rail | [ ] TODO |
| B2  | Cursor do brush no 3D e no 2D (dureza, opacidade, jitter) e posição espelhada 3D↔2D         | [ ] TODO |
| B3  | Quadro de prévia de ferramentas na viewport (extrude, inset, bevel, knife) com HUD numérico  | [ ] TODO |
| B4  | Registro de propriedades animáveis (tipo, faixa, unidade)                                    | [ ] TODO |

## Onda C — grandes

| ID  | Item                                                                                                     | Status   |
|-----|----------------------------------------------------------------------------------------------------------|----------|
| C1  | Camadas: opções da camada ativa no painel, efeitos como sub-camadas, máscara em camada e efeito         | [ ] TODO |
| C2  | Paint: ferramenta única "Projeção"; pintar só a seleção; isolar para pintura (fantasma/raio-x)          | [ ] TODO |
| C3  | Gradiente como camada de preenchimento editável (tipos como opção do painel)                            | [ ] TODO |
| C4  | Nós: DAG com portas tipadas, picker de objeto, menu ao arrastar uma saída, booleana com operando visível/fantasma | [ ] TODO |
| C5  | Animate: bones/rigs/templates como módulo; timeline estilo motion design; **aba de Curvas**; morph targets | [ ] TODO |
| C6  | Assets: galeria nova (grade, busca, tags, arrastar para a viewport) e salvar grupos (depende de G09)    | [ ] TODO |

## Gap

- Hoje `DeleteAssetCmd` não limpa grupos vazios; o grupo só guarda nome (`Project.collections: Vec<String>`).
- O menu de botão direito só tem "Origem para o cursor"; o pivô é calculado na projeção, sem deslocamento próprio persistido.
- O seletor de cor é um diálogo pequeno com confirmação; a opacidade do brush é repetida no painel de Paint.
- A timeline não tem registro de propriedades nem editor de curvas.

## Design — aba de Curvas da timeline

- **Mesma fonte da timeline:** os canais do editor de curvas são as propriedades do registro animável (B4); selecionar uma propriedade na timeline a mostra nas curvas.
- **Interpolação por chave:** constante, linear, Bézier com tangentes (livres, alinhadas, automáticas) e atalhos de suavização; modo "normalizar" para ver curvas de escalas diferentes juntas.
- **Edição:** seleção múltipla de chaves, mover no tempo e no valor, caixa de transformação (escala em volta de um pivô), copiar e colar, repetir (loop) e ajuste de ordem.
- **Acessibilidade e hardware modesto:** valor e tempo da chave com entrada numérica digitada; navegação de chave a chave pelo teclado; amostragem das curvas só do trecho visível e invalidação por revisão (sem recalcular por quadro).

## Verification

Por onda, quando entregue. A Onda A: ver o registro de evidência `shell-v2-roadmap-a-2026-10-18` em `website/progress/tasks.json`.

## Risks

- A cor por grupo muda o formato do documento: o campo é opcional (arquivos antigos abrem sem migração) e a validação rejeita valores inválidos.
- Mover o pivô sem regravar a malha exige que gizmo, snap e animação leiam o pivô do objeto, e não o centro da caixa.
- Animar vértices à mão depende de topologia estável; operações que mudam topologia entram só por nós (ADR-C16, item 4).

## Next

Concluir a Onda A, rodar os gates no fim e só então abrir B1 e B4, que destravam o resto.
