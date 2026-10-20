# Shell v2 — roteiro de Poly, Paint, UV, Animate, Nós e Assets (2026-10-18)

> Status: **aprovado** ([ADR-C16](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuario)). Ondas A e B concluídas (2026-10-19); Onda C em andamento: C1–C3 entregues em 2026-10-19. Referências de outros softwares (Blender, Photoshop, Affinity, Cavalry, Vegas) servem de **referência de ergonomia**, nunca de código ou recurso copiado.

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
| B8  | Linguagem visual da viewport e da GUI: guia de estilo e passada de polimento; **aproveitar o espaço** (painéis e campos sem tamanho desnecessário) e **alinhar todo elemento ao visual geral** (pedido de 2026-10-19) | [x] DONE (guia aprovado; tokens nos guias de snap; frentes B12–B14) |
| B9  | Menus em torta (pie): Shift+A primitivas, vista da câmera, domínio de seleção (objeto/vértice/aresta/face), com teclado e alternativa em lista | [x] DONE (código e testes) |
| B10 | Seleção de vértice/aresta/face fiel ao que se vê: respeitar a oclusão no Sólido e selecionar através no Raio-X/Wireframe (política explícita, controle no cabeçalho, estudo de referência) | [x] DONE (política Automática aprovada; 3 defeitos corrigidos com testes de regressão) |
| B11 | Gizmo da ferramenta Selecionar configurável nas Preferências: mostrar mover, escalar e/ou rotacionar | [x] DONE (código e testes) |
| B12 | Seleção e overlays suaves na vista: contorno fino do objeto, pontos pequenos de vértice, arestas finas, face com tinta leve, sem diagonais de triangulação nem halos sobrepostos | [x] DONE (métricas novas; capturas GL antes/depois) |
| B13 | Painel direito denso e organizado: tamanho de cada campo e de cada módulo (o rótulo acima continua, por decisão do usuário) | [x] DONE (rótulo acima mantido; campos e módulos densos) |
| B14 | Paint compacto: seletor de cor em modo compacto no painel e menu de botão direito mais denso | [x] DONE (seletor de cor compacto e menus densos; trilho único mantido — ver nota) |
| B15 | Orientação de transformação no cabeçalho da vista: Global, Local, Normal e Vista | [x] DONE (Global, Local, Normal, Vista; Cursor fica de fora — ver nota) |

## Onda C — grandes

| ID  | Item                                                                                                     | Status   |
|-----|----------------------------------------------------------------------------------------------------------|----------|
| C1  | Camadas: opções da camada ativa no painel, efeitos como sub-camadas, máscara em camada e efeito         | [x] DONE (código e testes) |
| C2  | Paint: ferramenta única "Projeção"; pintar só a seleção; isolar para pintura (fantasma/raio-x)          | [x] DONE (código e testes) |
| C3  | Gradiente como camada de preenchimento editável (tipos como opção do painel)                            | [x] DONE (código e testes) |
| C4  | Nós: DAG com portas tipadas, picker de objeto, menu ao arrastar uma saída, booleana com operando visível/fantasma | [x] DONE (código e testes) |
| C5  | Animate: bones/rigs/templates como módulo; timeline estilo motion design; **aba de Curvas**; morph targets | [ ] TODO |
| C6  | Assets: galeria nova (grade, busca, tags, arrastar para a viewport) e salvar grupos (depende de G09)    | [~] IN PROGRESS (grade entregue; tags/arrastar/salvar grupos dependem de P04 e G09) |
| C7  | Grupo especial **Referência**, sempre o primeiro da lista de Partes, com girar, mover no eixo da referência, escalar, ocultar e apagar (no painel de Partes só ocultar e apagar) | [x] DONE (código e testes) |
| C8  | Modos das operações com vários elementos selecionados: extrudar a região (manifold), faces individuais, ao longo das normais; idem para inset/bevel, escolhidos no HUD da ferramenta | [x] DONE (código e testes) |

## Gap

- Hoje `DeleteAssetCmd` não limpa grupos vazios; o grupo só guarda nome (`Project.collections: Vec<String>`).
- O menu de botão direito só tem "Origem para o cursor"; o pivô é calculado na projeção, sem deslocamento próprio persistido.
- ~~O seletor de cor é um diálogo pequeno com confirmação; a opacidade do brush é repetida no painel de Paint.~~ Resolvido na Onda B (B1/A8).
- ~~`view.frame_selection` só agendava `camera_frame` e nada o consumia no shell Qt.~~ Corrigido: o tick do shell avança a animação, e com movimento reduzido salta direto ao alvo.
- A timeline não tem registro de propriedades nem editor de curvas.

## Notas de design — pedidos de 2026-10-19

- **B6 eixos e grade.** O estado já tem `GridSettings` (tamanho, subdivisões, opacidade) e `show_grid`: falta expor na interface (Preferências e um botão no cabeçalho da vista). O eixo Y vertical atravessando o objeto é ruído visual: oculto por padrão, X/Z no chão, e todos os eixos com teste de profundidade (não aparecem por dentro dos objetos), cada um ligável nos overlays.
- **B7 trava de valores.** Referência de ergonomia: o cadeado por canal ao lado de cada campo de transformação (Blender). A trava é dado do objeto (por canal), não da interface: gizmo, modal, campos e scripts precisam respeitá-la, com o motivo exibido quando recusam.
- **B8 linguagem visual.** Antes de polir peça por peça, um guia de estilo com referências e capturas (antes/depois), aprovado com o usuário; depois a passada no shell inteiro. É alicerce: a Onda C cria muita interface e deve nascer no estilo novo.
- **B9 menus em torta.** O shell antigo tinha um `PetuniaPieMenu` (removido na U19); o novo é um componente do shell v2: abre no ponteiro, escolhe por direção ou número, Esc cancela, navegável por teclado e com nomes acessíveis, respeitando movimento reduzido; cada torta tem a mesma lista também em menu comum.
- **B10 seleção fiel ao que se vê (defeito relatado em 2026-10-19).** Gap verificado no código: o picking usa `select_through`, que por decisão anterior ([câmera e seleção](../01-core/camera-and-selection.md)) é independente de `show_xray` e do Wireframe, e a exposição no Qt (H33) nunca foi feita. Resultado: no Raio-X a pessoa vê os componentes de trás e não consegue selecioná-los. Que o Sólido "não respeita o que é visível" ainda precisa de reprodução. Hipóteses: o teste de visibilidade é por ponto (vértice, amostras da aresta, todos os vértices da face na caixa), com tolerância relativa de `1e-5`, e falha na silhueta e em faces parcialmente cobertas. **Recomendação, a debater:** o modelo continua separando render e picking, mas a política padrão vira **Automática**, ou seja, segue o que se vê (Sólido respeita a oclusão; Raio-X e Wireframe selecionam através). Há também um controle de três estados no cabeçalho da vista (Automática, Só visível, Através), anunciado ao leitor de tela, e Alt+Z continua só visual. Isso emenda a regra de `camera-and-selection.md`, então entra no decision register depois de aprovado. **Referências de ergonomia, por conhecimento geral e ainda a validar com uso real:** no Blender, clique, caixa e laço respeitam a oclusão no Sólido e selecionam através com o Raio-X ligado (o Wireframe tem Raio-X próprio). No Maya há a opção de seleção baseada na câmera (só o visível). No 3ds Max há "Ignore Backfacing" no Editable Poly. Nada disso foi consultado na fonte nesta sessão. Gates: matriz de teste (domínio × clique/caixa/laço × Sólido/Raio-X/Wireframe × política) com cubo ortográfico e perspectivo, silhueta e face parcialmente coberta, mais um teste de regressão que reproduza o relato antes da correção.
- **B11 gizmo do Selecionar.** Hoje o Selecionar mostra mover, girar e escalar juntos (A16). A preferência escolhe quais alças aparecem (qualquer combinação, inclusive nenhuma), persiste em `UserPreferences` como as outras do gizmo e vale só para a ferramenta Selecionar (Mover/Girar/Escalar continuam com a própria alça).
- **Referência visual de 2026-10-19 (capturas do Blender enviadas pelo usuário).** Comparação feita sobre as imagens e as nossas capturas do harness (`target/shell-captures/shell_POLY.png`, `shell_PAINT.png`, `wave_b_extrude_preview.png`). É ergonomia, sem copiar ícone, cor ou código (clean-room). Pontos observados:
  - **B12 seleção.** Na referência, o objeto selecionado tem só a silhueta fina em laranja, sem arestas internas. Em edição, vértices são pontos pequenos (escuros; selecionados claros), arestas são finas e só a selecionada ganha cor, e a face selecionada recebe uma tinta translúcida leve. Não aparece diagonal de triangulação nem traço com halo grosso. Na nossa vista, a prévia mostra as diagonais da triangulação e o gizmo do Selecionar empilha mover, girar e escalar (ver B11). O alvo é a hierarquia do [guia](visual-language.md): seleção ativa > ferramenta > hover > guias > grade. Diagonais só com "triangulação" ligada.
  - **B13 painel direito.** A referência usa uma coluna de rótulos alinhada à direita e campos numa grade única. O rótulo do grupo aparece uma vez ("Location X", depois só "Y" e "Z"), e a última coluna é fixa para cadeado e chave. As linhas são baixas e regulares, as seções vêm recolhidas por padrão (Delta Transform aninhado) e o agrupamento é feito por espaço, não por bordas. No nosso painel, os rótulos ficam acima dos campos (duas linhas por vetor), os controles são altos, há fundos e bordas por grupo, e o Pivô ocupa uma linha inteira. Proposta: rótulo à esquerda quando a coluna passa de cerca de 300 px, XYZ na mesma linha enquanto couber (o `PetuniaVector3Field` já empilha quando estreita), altura de controle densa por padrão e coluna de ícones alinhada.
  - **B14 Paint.** A referência mostra as vistas 2D e 3D lado a lado, cada uma com um trilho fino próprio, uma prateleira de pincéis embaixo (miniaturas, categorias e busca) e, no painel, só o pincel ativo, Tamanho e Força com o botão de pressão. O menu de botão direito é denso: linhas baixas, separadores por grupo, atalho alinhado à direita e submenu com seta. No nosso Paint, a roda de cor grande toma o painel, o canvas 2D fica vazio e o trilho junta 2D e 3D. Proposta: modo compacto do `PetuniaColorPanel` (roda menor, canais recolhíveis), prateleira de pincéis e densidade de menu definida em `PetuniaTheme`.
  - **B15 orientação.** A referência oferece Global, Local, Normal, Gimbal, Vista, Cursor e Pai, mais "+" para criar uma orientação. O núcleo já tem `TransformOrientation` (Global, Local, Normal, View, Auto, Custom), sem nada exposto no Qt. Recomendação: Global, Local, Normal, Vista e Cursor (o Cursor 3D existe). Gimbal fica para a C5 (curvas de Euler) e Pai entra só quando houver hierarquia. Custom ("+") já tem `custom_transform_frame` no núcleo e pode vir depois, e Auto precisa ter a semântica revista antes de aparecer.
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

Fechamento da Onda B (B8, B10–B15) e correção do enquadramento (F/Home), 2026-10-19, testes no fim (pedido do usuário). Mesmo ambiente.

| Gate                                                                                   | Resultado                                                              | Status  |
|----------------------------------------------------------------------------------------|------------------------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                                           | sem diferenças                                                         | pass    |
| `cargo clippy -D warnings` (8 crates, `--all-targets --all-features`)                  | 0 avisos (1 lint do teste novo corrigido)                              | pass    |
| `cargo test --no-fail-fast` (7 crates, `--all-features`, `PETUNIA_REQUIRE_GL=1`)       | 1628 testes, 0 falhas, GL sem SKIPPED                                  | pass    |
| `cargo test -p petunia_ui_cxxqt -- --test-threads=1` (offscreen)                       | 51 testes, 0 falhas (inclui os 2 do enquadramento em `editor_parity`)  | pass    |
| `shell_v2` (harness): 315 verificações, 10 novas                                       | 0 reprovadas em 5 execuções seguidas                                   | pass    |
| `xtask docs-generate --check`; `verify-progress.cjs`; `progress.test.cjs`              | sem drift; consistentes; 6/6                                           | pass    |
| Janela real com GPU (seleção no Raio-X, gizmo, painel denso, Paint compacto)            | não executado: precisa do usuário no app                               | not run |
| Windows, leitor de tela, alto contraste, 150%/200%                                     | não executado                                                          | not run |

Defeitos reais achados e corrigidos neste fechamento, cada um com teste que falhava antes:
- **Raio-triângulo sem folga:** o raio vazava pela diagonal compartilhada dos quads (125 de 1600 raios no teste sintético). As cópias em `projection.rs` e `section_analysis.rs` agora delegam à única função corrigida.
- **Gizmo do Selecionar:** engolia o clique em vértices e arestas vizinhos (o hover mostrava um componente e o clique ia para a alça).
- **Gizmo sem componentes:** aparecia em vértice/aresta/face sem nada selecionado.
- **`camera_frame` não avançava** no shell Qt (F/Home não reenquadravam). Corrigido pela sessão paralela e incluído aqui.

Teste ajustado (por honestidade): `vertex_domain_draws_round_markers_of_constant_size` exigia mais de 40 px no disco de hover, o que codificava o halo antigo. O limite passou a refletir o ponto aprovado de 3,5 px (mais de 20 px), e a verificação de tamanho constante em qualquer zoom ficou intacta.

Correção de teste feita no caminho (por honestidade): a primeira versão do check do eco do pincel apontava o centro da vista, mas checks anteriores giram a câmera e movem o cubo e o ponto caía fora do objeto. O check passou a partir de um ponto da textura visível e fazer a ida e volta (erro medido: 0,00 px). A lógica Rust não mudou, e um teste de bridge isolado já passava.

Onda C, parte 1 (C1–C3) e correção do wireframe nas bordas, 2026-10-19, testes no fim (pedido do usuário). Mesmo ambiente; decisões em [ADR-C16, item 16](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuário).

| Gate                                                                                          | Resultado                                                                 | Status  |
|-----------------------------------------------------------------------------------------------|---------------------------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                                                  | sem diferenças                                                            | pass    |
| `cargo clippy -D warnings` (core, project, module-paint, render, editor; `--all-targets`)     | 0 avisos (4 lints nos arquivos novos corrigidos)                          | pass    |
| `cargo test --lib` (project, module-paint, core, render, editor `animation-workspace`)        | 293 + 125 + 210 + 40 + 148 testes, 0 falhas                               | pass    |
| `cargo check -p petunia_ui_cxxqt`                                                             | C++/Qt compila (painéis de camada, efeitos, projeção e prateleira)        | pass    |
| `cargo test -p petunia_ui_cxxqt --test editor_parity -- --test-threads=1` (offscreen)          | 24 testes, 0 falhas                                                       | pass    |
| `cargo test -p petunia_ui_cxxqt --test shell_v2 -- --test-threads=1` (offscreen)               | 323 verificações, 0 reprovadas (8 novas: bindings dos painéis, máscara, sub-efeito, gradiente, isolar) | pass    |
| Janela real com GPU (painéis novos, prateleira de pincéis, wireframe nas bordas)              | não executado: precisa do usuário no app                                  | not run |
| Windows, leitor de tela, alto contraste, 150%/200%                                            | não executado                                                             | not run |

Testes novos da entrega (todos passando): máscara da camada com inversão; ordem e opacidade dos sub-efeitos; gradiente com dados hostis (reparo de paradas e geometria); `composite_tiles == composite` com gradiente + sub-efeito + máscara; reparo de pilha hostil (índice ativo, efeitos, grupos órfãos/ciclo, alvo órfão) com idempotência; mescla recusando ajuste embaixo; alvo do pincel escolhendo canvas e chave de traço; operações de gradiente/efeito/máscara (limites, clamps, recusas); isolamento de pintura fantasma preservando o Raio-X; normais por aresta (borda aberta, cubo fechado, não-manifold) e layout das faixas no `edge_bands`.

**Defeito achado e corrigido no próprio gate de interface:** o controle "Isolar para pintura" da seção Projeção compilava, mas enviava `{"id": <booleano>}` — o comando `paint.isolate` é ação de keymap, não comando registrado, então o clique não fazia nada. A verificação de harness `paint:isolate-ghost-toggles-the-mode` nasceu reprovando (0 -> 0) e só fechou depois de expor `toggle_paint_isolate` como ação de request (o keybind usa o mesmo método). Também foi conferido, por verificação de harness, que toda chave ligada pelos três painéis novos existe no snapshot (`paint:panels-have-no-missing-bindings`).

Nota de infraestrutura: o vertex shader novo (`edge_line.vert`) entra na lista de fontes do teste de shaders, que compila cada fonte — sem isso o arquivo passaria despercebido.

Onda C, parte 2 — C8 (modos das operações com vários elementos), 2026-10-19. Decisões em [ADR-C16, item 18](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuário).

| Gate                                                                                 | Resultado                                                             | Status  |
|--------------------------------------------------------------------------------------|-----------------------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                                         | sem diferenças                                                      | pass    |
| `cargo clippy -D warnings` (mesh, core, editor, config, module-model, cli, ui-cxxqt)  | 0 avisos                                                             | pass    |
| `cargo test --lib` (mesh, core, editor `animation-workspace`, config)                 | 272 + 211 + 148 + 27 testes, 0 falhas                                | pass    |
| `cargo test -p petunia_editor --test operation_mode_c8`                               | 4 testes, 0 falhas (modos publicados, escolha na ferramenta)         | pass    |
| `cargo test -p petunia_ui_cxxqt --test editor_parity` (offscreen)                      | 25 testes, 0 falhas                                                  | pass    |
| `cargo test -p petunia_ui_cxxqt --test shell_v2` (offscreen)                           | 326 verificações, 0 reprovadas (2 novas: modos no HUD)             | pass    |
| `cargo test -p petunia_ui_cxxqt --test ui_i18n_catalog`                                | 3 testes, 0 falhas                                                   | pass    |
| Janela real com GPU (escolha do modo com o mouse e pelo teclado)                      | não executado: precisa do usuário no app                           | not run |

O que mudou (C8): a extrusão tem **três modos** — Região (a seleção sobe como um bloco, malha fechada), **Faces individuais** (cada face vira o seu prisma, mas todas sobem na mesma direção) e **Ao longo das normais** (cada face sobe pela sua própria normal). O Inset tem **dois** (Região e Individual). A escolha fica no HUD da ferramenta, num controle segmentado que só aparece quando a operação tem modo, e vale para as operações seguintes (é lembrada com as demais preferências da ferramenta).

Defeito encontrado e corrigido no caminho: a normal da região era normalizada duas vezes na extrusão individual, o que fazia "individual" divergir da "região" nos últimos bits da posição (1e-8) — a normal agora é normalizada uma vez só, com teste que compara as duas direções.

Onda C, parte 3 — C7 (grupo Referência nas Partes), 2026-10-19.

| Gate                                                        | Resultado                                             | Status  |
|-------------------------------------------------------------|-------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                | sem diferenças                                        | pass    |
| `cargo clippy -D warnings` (crates afetados)                | 0 avisos                                              | pass    |
| `cargo test -p petunia_ui_cxxqt --test shell_v2` (offscreen) | 327 verificações, 0 reprovadas (1 nova do C7)        | pass    |
| `cargo test -p petunia_ui_cxxqt --test ui_i18n_catalog`     | 3 testes, 0 falhas                                    | pass    |
| Janela real com GPU (grupo, olho, apagar pelo menu)         | não executado: precisa do usuário no app              | not run |

O que mudou (C7): as imagens de referência ganham um **grupo próprio, sempre no topo da lista de Partes** — com o olho (mostrar/ocultar) e o cadeiro na linha, e o botão direito com "Apagar referência". Girar, mover no eixo e escalar continuam no painel de Referências (o grupo em Partes é o atalho, não uma segunda verdade). O grupo só aparece quando há imagem carregada; slots vazios não viram linha.

Onda C, parte 4 — C4 (nós: portas tipadas, picker de objeto, menu ao arrastar uma saída, booleana com operando fantasma), 2026-10-19.

| Gate                                                            | Resultado                                                          | Status  |
|-----------------------------------------------------------------|--------------------------------------------------------------------|---------|
| `cargo fmt --all -- --check`                                    | sem diferenças                                                     | pass    |
| `cargo clippy -D warnings` (core, mesh, ui-cxxqt)                | 0 avisos                                                           | pass    |
| `cargo test -p petunia_core --lib`                              | 212 testes, 0 falhas (1 novo: nó criado já ligado)                 | pass    |
| `cargo test -p petunia_mesh --lib`                              | 283 testes, 0 falhas (11 dos morph targets)                        | pass    |
| `cargo test -p petunia_ui_cxxqt --test shell_v2` (offscreen)     | 328 verificações, 0 reprovadas (1 nova do C4)                     | pass    |
| `cargo test -p petunia_ui_cxxqt --test ui_i18n_catalog`          | 3 testes, 0 falhas                                                 | pass    |
| Janela real com GPU (arrastar fio, picker, fantasma)             | não executado: precisa do usuário no app                           | not run |

O que mudou (C4): as **portas tipadas** já existiam no grafo de materiais (entrada/saída por tipo, recusadas quando o tipo não bate). Esta entrega fecha as três peças que faltavam:

- **Menu ao arrastar uma saída**: soltar um fio no vazio abre o menu de nós; no grafo de materiais, o nó novo **nasce ligado** à saída arrastada, na posição da soltura, com a entrada compatível escolhida pelo tipo (um tipo incompatível é recusado e não deixa nó órfão).
- **Picker de objeto**: o alvo de uma operação é escolhido numa lista **buscável** (o antigo menu "Aplicar em…" virou o picker) — e soltar um fio no vazio no grafo de objetos abre o mesmo picker.
- **Booleana com operando visível/fantasma**: o objeto que serve de ferramenta fica escondido no documento; no grafo ele aparece **esmaecido**, com a dica no tooltip dizendo por quê (não só pela cor).

Onda C, parte 5 — C6 (galeria de assets) e C5 (Animate), 2026-10-19.

**C6 — galeria (parcial).** A biblioteca de assets passa a ser uma **grade de miniaturas** (a miniatura é o que distingue um asset do outro), mantendo a busca e o clique para inserir. O que **não** foi entregue e por quê:

| Parte | Situação | Motivo |
|---|---|---|
| Grade de miniaturas | entregue | — |
| Buscar | já existia | — |
| Arrastar para a viewport | pendente | a viewport ainda não aceita soltar assets; falta o pedido de inserção no ponto de soltura |
| Tags | pendente | não existe modelo de metadados de asset (P04 está TODO) |
| Salvar grupos | pendente | depende de **G09** (hierarquia/coleções/prefabs), que está TODO |

**C5 — Animate (parcial).** Entregue nesta onda: os **morph targets** (blend shapes) no crate de malha — contrato de dados, mistura linear (`p = p_base + Σ peso·(alvo − p_base)`), recusas tipadas e 11 testes. Ficam de fora: a **aba de Curvas** (o modelo do Animate publica as trilhas e os tempos dos keyframes, mas não os *valores* das curvas — sem eles não há o que desenhar; falta a amostragem da curva no modelo e o editor de gráfico), a **timeline estilo motion design** (a barra de transporte e o dope sheet existem; a barra de tempo em motion design, com camadas e marcador de quadro, é reescrita de tela) e **bones/rigs/templates como módulo** (reorganização do `crates/editor/src/animate.rs` em crate próprio — trabalho de forma, sem valor novo para quem usa).

## Risks

- A cor por grupo muda o formato do documento: o campo é opcional (arquivos antigos abrem sem migração) e a validação rejeita valores inválidos.
- Mover o pivô regrava as coordenadas locais da malha de forma compensada (a geometria no mundo não muda); por isso o rebase recusa primitiva paramétrica, modificadores, rig e objeto bloqueado, e a interface mostra o motivo.
- Animar vértices à mão depende de topologia estável; operações que mudam topologia entram só por nós (ADR-C16, item 4).

## Next

Conferência do usuário no app real: seleção no Raio-X e no Sólido, gizmo do Selecionar, painel denso, Paint compacto, orientação, diálogos no Wayland, tortas, cadeados, grade, e — da Onda C parte 1 — os painéis de camada/efeitos/gradiente, a prateleira de pincéis e o wireframe nas bordas. A Onda C já tem C1–C3 e C8; depois dela vêm C4 (nós), C5 (que consome o registro animável), C6 (galeria de assets) e C7 (grupo Referência).
