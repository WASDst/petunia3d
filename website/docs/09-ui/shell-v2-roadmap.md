# Shell v2 — roteiro de Poly, Paint, UV, Animate, Nós e Assets (2026-10-18)

> Status: **aprovado** ([ADR-C16](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuario)). Onda A concluída; Onda B implementada até B9 (2026-10-19), com B8 aguardando a aprovação do guia de estilo e B10/B11 registrados em 2026-10-19; a Onda C vem depois. Referências de outros softwares (Blender, Photoshop, Affinity, Cavalry, Vegas) servem de **referência de ergonomia**, nunca de código ou recurso copiado.

## Intent

Transformar as observações de uso do usuário em um plano por onda: o que é pequeno e concreto (A), o que é alicerce compartilhado (B) e o que é grande (C).

## Sources

Pedido do usuário de 2026-10-18, [refinamento do Shell v2](shell-v2-refinement.md), [Operation Graph](../03-geometry/operation-graph.md), [Paint](../06-paint/workspace-ux.md), [UV](../07-uv/workspace-ux.md), [Animação](../10-animation/workspace-ux.md).

## Onda A — pequenas e concretas

| ID  | Item                                                                                          | Status          |
|-----|-----------------------------------------------------------------------------------------------|-----------------|
| A1  | Apagar ou tirar o último membro de um grupo remove o grupo (um passo de Undo)                  | [x] DONE (código e testes; gates no fim) |
| A2  | Renomear grupo descobrível (F2 e duplo clique) e cor de seleção por grupo (+ forçar)          | [x] DONE (código e testes; gates no fim) |
| A3  | Pivô: origem do objeto (rebase sem mudar a geometria), Editar pivô em sessão (Enter/Esc) e menu de botão direito | [x] DONE (código e testes; gates no fim) |
| A4  | Menu de botão direito do 3D Cursor (mouse, seleção, pivô do objeto, origem do mundo, enquadrar) | [x] DONE (código e testes; gates no fim) |
| A5  | Nós: navegação infinita (roda/trackpad, botão direito, Espaço, Alt+setas), zoom no cursor e "Enquadrar tudo" (botão, Home, F) | [x] DONE (código e testes; gates no fim) |
| A6  | Paint: botão no canvas 2D para alternar o wireframe UV sobre a pintura (a UV já o mostra)      | [x] DONE (código e testes; gates no fim) |
| A7  | UV: mover várias ilhas de uma vez (pressionar numa selecionada mantém tudo) e caixa de seleção | [x] DONE (código e testes; gates no fim) |
| A8  | Remover a opacidade duplicada do Paint                                                         | [x] DONE (a opacidade é o alfa do seletor de cor embutido no pincel) |

### Onda A, segunda leva — relatos de uso de 2026-10-18 (tarde)

| ID  | Item                                                                                                              | Status          |
|-----|-------------------------------------------------------------------------------------------------------------------|-----------------|
| A9  | Medida: a ferramenta não tinha gesto (`active_measurement` nunca era escrito); dois cliques com prévia, persistência e Undo | [x] DONE (código e testes) |
| A10 | Arrastar sobre o valor **ou** sobre o nome de qualquer campo numérico ajusta o número (`PetuniaScrub`, global)    | [x] DONE (código e testes) |
| A11 | Objeto oculto continua selecionável (Partes, nós) e transformável (gizmo, campos, teclado)                        | [x] DONE (código e testes) |
| A12 | Nós: caixa de seleção, multisseleção, Ctrl+G agrupa e renomear grupo de nós                                       | [x] DONE (código e testes) |
| A13 | Legenda de tipos do canvas de nós fixa na vista (não anda com o pan)                                              | [x] DONE (código e testes) |
| A14 | Imagens de referência importadas na orientação certa (UV do quad vinha invertido em V)                            | [x] DONE (código e testes) |
| A15 | Opacidade global do cubo de navegação nas Preferências                                                            | [x] DONE (código e testes) |
| A16 | Cursor 3D e gizmo acompanham o objeto durante o arrasto                                                           | [x] DONE (`ee93e35a`) |

## Onda B — alicerces

| ID  | Item                                                                                       | Status   |
|-----|--------------------------------------------------------------------------------------------|----------|
| B1  | `PetuniaColorPanel` único (roda, modos, RGB/HSL+alfa, hex, paletas CRUD) e cor fixa no rail | [x] DONE (código e testes) |
| B2  | Cursor do brush no 3D e no 2D (dureza, opacidade, jitter) e posição espelhada 3D↔2D         | [x] DONE (código e testes) |
| B3  | Quadro de prévia de ferramentas na viewport (extrude, inset, bevel, knife) com HUD numérico  | [x] DONE (código e testes) |
| B4  | Registro de propriedades animáveis (tipo, faixa, unidade)                                    | [x] DONE (contrato e testes; a GUI o consome na C5) |
| B5  | Todo diálogo/modal movível e redimensionável (Preferências, Referências, primitivas, cor), com posição e tamanho lembrados | [x] DONE (código e testes) |
| B6  | Overlays da vista: eixo Y oculto por padrão e eixos sem atravessar objetos (teste de profundidade, esmaecidos); grade com extensão e tamanho do ladrilho configuráveis e botão de ligar/desligar no cabeçalho da vista | [x] DONE (código e testes) |
| B7  | Travar valores: cadeado pequeno dentro do próprio campo, à direita; trava por canal (posição/rotação/escala × X/Y/Z) respeitada por gizmo, modal e teclado | [x] DONE (código e testes) |
| B8  | Linguagem visual da viewport e da GUI: estudo de referência, guia de estilo e passada de polimento (ícones, espaçamento, feedback de seleção/hover/ferramenta, gizmos, overlays, tipografia) | [-] IN PROGRESS (guia proposto em [linguagem visual](visual-language.md), aguarda aprovação) |
| B9  | Menus em torta (pie): Shift+A primitivas, vista da câmera, domínio de seleção (objeto/vértice/aresta/face), com teclado e alternativa em lista | [x] DONE (código e testes) |
| B10 | Seleção de vértice/aresta/face fiel ao que se vê: respeitar a oclusão no Sólido e selecionar através no Raio-X/Wireframe (política explícita, controle no cabeçalho, estudo de referência) | [ ] TODO (pedido de 2026-10-19; decisão de política com o usuário) |
| B11 | Gizmo da ferramenta Selecionar configurável nas Preferências: mostrar mover, escalar e/ou rotacionar | [ ] TODO (pedido de 2026-10-19) |

## Onda C — grandes

| ID  | Item                                                                                                     | Status   |
|-----|----------------------------------------------------------------------------------------------------------|----------|
| C1  | Camadas: opções da camada ativa no painel, efeitos como sub-camadas, máscara em camada e efeito         | [ ] TODO |
| C2  | Paint: ferramenta única "Projeção"; pintar só a seleção; isolar para pintura (fantasma/raio-x)          | [ ] TODO |
| C3  | Gradiente como camada de preenchimento editável (tipos como opção do painel)                            | [ ] TODO |
| C4  | Nós: DAG com portas tipadas, picker de objeto, menu ao arrastar uma saída, booleana com operando visível/fantasma | [ ] TODO |
| C5  | Animate: bones/rigs/templates como módulo; timeline estilo motion design; **aba de Curvas**; morph targets | [ ] TODO |
| C6  | Assets: galeria nova (grade, busca, tags, arrastar para a viewport) e salvar grupos (depende de G09)    | [ ] TODO |
| C7  | Grupo especial **Referência**, sempre o primeiro da lista de Partes, com girar, mover no eixo da referência, escalar, ocultar e apagar (no painel de Partes só ocultar e apagar) | [ ] TODO |
| C8  | Modos das operações com vários elementos selecionados: extrudar a região (manifold), faces individuais, ao longo das normais; idem para inset/bevel, escolhidos no HUD da ferramenta | [ ] TODO |

## Gap

- Hoje `DeleteAssetCmd` não limpa grupos vazios; o grupo só guarda nome (`Project.collections: Vec<String>`).
- O menu de botão direito só tem "Origem para o cursor"; o pivô é calculado na projeção, sem deslocamento próprio persistido.
- ~~O seletor de cor é um diálogo pequeno com confirmação; a opacidade do brush é repetida no painel de Paint.~~ Resolvido na Onda B (B1/A8).
- `view.frame_selection` só agenda `camera_frame` e nada o consome no shell Qt: enquadrar pode não mover a câmera (achado nos testes da Onda B, tarefa separada).
- A timeline não tem registro de propriedades nem editor de curvas.

## Notas de design — pedidos de 2026-10-19

- **B6 eixos e grade.** O estado já tem `GridSettings` (tamanho, subdivisões, opacidade) e `show_grid`: falta expor na interface (Preferências e um botão no cabeçalho da vista). O eixo Y vertical atravessando o objeto é ruído visual: oculto por padrão, X/Z no chão, e todos os eixos com teste de profundidade (não aparecem por dentro dos objetos), cada um ligável nos overlays.
- **B7 trava de valores.** Referência de ergonomia: o cadeado por canal ao lado de cada campo de transformação (Blender). A trava é dado do objeto (por canal), não da interface: gizmo, modal, campos e scripts precisam respeitá-la, com o motivo exibido quando recusam.
- **B8 linguagem visual.** Antes de polir peça por peça, um guia de estilo com referências e capturas (antes/depois), aprovado com o usuário; depois a passada no shell inteiro. É alicerce: a Onda C cria muita interface e deve nascer no estilo novo.
- **B9 menus em torta.** O shell antigo tinha um `PetuniaPieMenu` (removido na U19); o novo é um componente do shell v2: abre no ponteiro, escolhe por direção ou número, Esc cancela, navegável por teclado e com nomes acessíveis, respeitando movimento reduzido; cada torta tem a mesma lista também em menu comum.
- **B10 seleção fiel ao que se vê (defeito relatado em 2026-10-19).** Gap verificado no código: o picking usa `select_through`, que por decisão anterior ([câmera e seleção](../01-core/camera-and-selection.md)) é independente de `show_xray` e do Wireframe, e a exposição no Qt (H33) nunca foi feita. Resultado: no Raio-X a pessoa vê os componentes de trás e não consegue selecioná-los. Que o Sólido "não respeita o que é visível" ainda precisa de reprodução. Hipóteses: o teste de visibilidade é por ponto (vértice, amostras da aresta, todos os vértices da face na caixa), com tolerância relativa de `1e-5`, e falha na silhueta e em faces parcialmente cobertas. **Recomendação, a debater:** o modelo continua separando render e picking, mas a política padrão vira **Automática**, ou seja, segue o que se vê (Sólido respeita a oclusão; Raio-X e Wireframe selecionam através). Há também um controle de três estados no cabeçalho da vista (Automática, Só visível, Através), anunciado ao leitor de tela, e Alt+Z continua só visual. Isso emenda a regra de `camera-and-selection.md`, então entra no decision register depois de aprovado. **Referências de ergonomia, por conhecimento geral e ainda a validar com uso real:** no Blender, clique, caixa e laço respeitam a oclusão no Sólido e selecionam através com o Raio-X ligado (o Wireframe tem Raio-X próprio). No Maya há a opção de seleção baseada na câmera (só o visível). No 3ds Max há "Ignore Backfacing" no Editable Poly. Nada disso foi consultado na fonte nesta sessão. Gates: matriz de teste (domínio × clique/caixa/laço × Sólido/Raio-X/Wireframe × política) com cubo ortográfico e perspectivo, silhueta e face parcialmente coberta, mais um teste de regressão que reproduza o relato antes da correção.
- **B11 gizmo do Selecionar.** Hoje o Selecionar mostra mover, girar e escalar juntos (A16). A preferência escolhe quais alças aparecem (qualquer combinação, inclusive nenhuma), persiste em `UserPreferences` como as outras do gizmo e vale só para a ferramenta Selecionar (Mover/Girar/Escalar continuam com a própria alça).
- **C8 modos de operação.** O núcleo já tem `ModalKind::ExtrudeIndividual`; faltam o modo "ao longo das normais", o mesmo leque para inset e bevel, e a escolha visível no HUD da ferramenta (não escondida em atalho).

## Design — aba de Curvas da timeline

- **Mesma fonte da timeline:** os canais do editor de curvas são as propriedades do registro animável (B4); selecionar uma propriedade na timeline a mostra nas curvas.
- **Interpolação por chave:** constante, linear, Bézier com tangentes (livres, alinhadas, automáticas) e atalhos de suavização; modo "normalizar" para ver curvas de escalas diferentes juntas.
- **Edição:** seleção múltipla de chaves, mover no tempo e no valor, caixa de transformação (escala em volta de um pivô), copiar e colar, repetir (loop) e ajuste de ordem.
- **Acessibilidade e hardware modesto:** valor e tempo da chave com entrada numérica digitada; navegação de chave a chave pelo teclado; amostragem das curvas só do trecho visível e invalidação por revisão (sem recalcular por quadro).

## Verification

Onda A, executada em 2026-10-18 no fim da implementação (árvore de trabalho sobre `743273e1`). Ambiente: Linux, `RUSTUP_TOOLCHAIN=1.98.1`, `CARGO_BUILD_JOBS=2`, Qt offscreen, GL por EGL headless.

| Gate                                                                                          | Resultado                                                         | Status  |
|-----------------------------------------------------------------------------------------------|-------------------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                                                  | sem diferenças                                                    | pass    |
| `cargo clippy -D warnings` (core, editor, render, config, mesh, project, application, ui-cxxqt) | 0 avisos                                                          | pass    |
| `cargo test --no-fail-fast` (7 crates, `--all-features`, `PETUNIA_REQUIRE_GL=1`)              | 89 binários, 1570 testes, 0 falhas, nenhum SKIPPED (+51 novos)    | pass    |
| `cargo test -p petunia_ui_cxxqt -- --test-threads=1` (offscreen)                              | contratos, i18n, bridge, editor_parity e `shell_v2` verdes        | pass    |
| `shell_v2` (harness): 231 verificações, 42 novas desta onda e do conserto do gizmo            | 0 reprovadas; binário rodado 5 vezes seguidas sem falha           | pass    |
| `xtask docs-generate --check`                                                                 | sem drift                                                         | pass    |
| `verify-progress.cjs` e `progress.test.cjs`                                                   | 135 processos consistentes; 6/6                                   | pass    |
| Janela real com GPU (menus de Pivô/Cursor, canvas de nós, UV, árvore de Partes)               | conferido pelo usuário no app (Linux), 2026-10-19: "tudo funcionou" | pass    |
| Windows, leitor de tela, alto contraste, escalas 150%/200%                                    | não executado                                                     | not run |

Segunda leva (A9–A15), 2026-10-19: Rust 92 binários, 1602 testes, 0 falhas, GL sem SKIPPED; clippy `-D warnings` limpo; Qt offscreen 49 testes, 0 falhas; `shell_v2` com 258 verificações, 0 reprovadas, em 5 execuções seguidas; `docs-generate --check` sem drift. Defeitos reais achados pelos testes no caminho: o Ctrl+G de Partes deixava o grupo antigo vazio (agora removido) e, no canvas de nós, o filtro de teclas do shell mandava Ctrl+G para "agrupar objetos" (agora o canvas declara a tecla como sua).

Correções de teste feitas no caminho (registradas por honestidade): a primeira versão do check de órbita só olhava depois de soltar o botão e passava sem a correção; o check de "clique reduz a seleção UV" contava ilhas em vez de faces e partia de uma seleção que o Undo já tinha desfeito.

Onda B (B1–B7, B9) e A8, 2026-10-19, testes só no fim da onda (pedido do usuário). Ambiente igual ao da Onda A.

| Gate                                                                                          | Resultado                                                                 | Status  |
|-----------------------------------------------------------------------------------------------|---------------------------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                                                  | sem diferenças (depois de formatar o teste novo)                          | pass    |
| `cargo clippy -D warnings` (8 crates, `--all-targets --all-features`)                         | 0 avisos                                                                  | pass    |
| `cargo test --no-fail-fast` (7 crates, `--all-features`, `PETUNIA_REQUIRE_GL=1`)              | 1615 testes, 0 falhas, GL sem SKIPPED; + ida e volta 3D↔UV no bridge      | pass    |
| `cargo test -p petunia_ui_cxxqt -- --test-threads=1` (offscreen)                              | 49 testes, 0 falhas (contratos, i18n, bridge, editor_parity, `shell_v2`)  | pass    |
| `shell_v2` (harness): 303 verificações, 45 novas da Onda B                                    | 0 reprovadas em 5 execuções seguidas                                      | pass    |
| `xtask docs-generate --check`                                                                 | sem drift (regenerado pelos atalhos novos Alt+V e Alt+D)                  | pass    |
| `verify-progress.cjs`, `progress.test.cjs`, `verify-agent-docs.py`                            | 135 processos consistentes; 6/6; links e catálogo consistentes            | pass    |
| Janela real com GPU (Wayland: mover/redimensionar diálogos, tortas, cadeados, eco do pincel)  | não executado: precisa do usuário no app                                  | not run |
| Windows, leitor de tela, alto contraste, escalas 150%/200%                                    | não executado                                                             | not run |

Correção de teste feita no caminho (por honestidade): a primeira versão do check do eco do pincel apontava o centro da vista, mas checks anteriores giram a câmera e movem o cubo e o ponto caía fora do objeto. O check passou a partir de um ponto da textura visível e fazer a ida e volta (erro medido: 0,00 px). A lógica Rust não mudou, e um teste de bridge isolado já passava.

## Risks

- A cor por grupo muda o formato do documento: o campo é opcional (arquivos antigos abrem sem migração) e a validação rejeita valores inválidos.
- Mover o pivô regrava as coordenadas locais da malha de forma compensada (a geometria no mundo não muda); por isso o rebase recusa primitiva paramétrica, modificadores, rig e objeto bloqueado, e a interface mostra o motivo.
- Animar vértices à mão depende de topologia estável; operações que mudam topologia entram só por nós (ADR-C16, item 4).

## Next

Conferência do usuário no app real (diálogos no Wayland, tortas, cadeados, eco do pincel, grade). Depois, aprovação do [guia de estilo](visual-language.md) (B8) e abertura da Onda C, começando por C5 (que consome o registro animável) e C8 (modos no HUD de prévia).
