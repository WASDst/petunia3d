# Shell v2 — roteiro de Poly, Paint, UV, Animate, Nós e Assets (2026-10-18)

> Status: **aprovado** ([ADR-C16](../00-philosophy/decision-register.md#adr-c16--shell-v2-roteiro-de-poly-paint-uv-animate-nós-e-assets-2026-10-18--aprovado-pelo-usuario)). A Onda A está em implementação; B e C seguem depois dela. Referências de outros softwares (Blender, Photoshop, Affinity, Cavalry, Vegas) servem de **referência de ergonomia**, nunca de código ou recurso copiado.

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
| A8  | Remover a opacidade duplicada do Paint                                                         | [ ] TODO (depende de B1: a opacidade passa para o seletor de cor) |

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
| B1  | `PetuniaColorPanel` único (roda, modos, RGB/HSL+alfa, hex, paletas CRUD) e cor fixa no rail | [ ] TODO |
| B2  | Cursor do brush no 3D e no 2D (dureza, opacidade, jitter) e posição espelhada 3D↔2D         | [ ] TODO |
| B3  | Quadro de prévia de ferramentas na viewport (extrude, inset, bevel, knife) com HUD numérico  | [ ] TODO |
| B4  | Registro de propriedades animáveis (tipo, faixa, unidade)                                    | [ ] TODO |
| B5  | Todo diálogo/modal movível e redimensionável (Preferências, Referências, primitivas, cor), com posição e tamanho lembrados | [ ] TODO |

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
| Janela real com GPU (menus de Pivô/Cursor, canvas de nós, UV, árvore de Partes)               | não executado                                                     | not run |
| Windows, leitor de tela, alto contraste, escalas 150%/200%                                    | não executado                                                     | not run |

Segunda leva (A9–A15), 2026-10-19: Rust 92 binários, 1602 testes, 0 falhas, GL sem SKIPPED; clippy `-D warnings` limpo; Qt offscreen 49 testes, 0 falhas; `shell_v2` com 258 verificações, 0 reprovadas, em 5 execuções seguidas; `docs-generate --check` sem drift. Defeitos reais achados pelos testes no caminho: o Ctrl+G de Partes deixava o grupo antigo vazio (agora removido) e, no canvas de nós, o filtro de teclas do shell mandava Ctrl+G para "agrupar objetos" (agora o canvas declara a tecla como sua).

Correções de teste feitas no caminho (registradas por honestidade): a primeira versão do check de órbita só olhava depois de soltar o botão e passava sem a correção; o check de "clique reduz a seleção UV" contava ilhas em vez de faces e partia de uma seleção que o Undo já tinha desfeito.

## Risks

- A cor por grupo muda o formato do documento: o campo é opcional (arquivos antigos abrem sem migração) e a validação rejeita valores inválidos.
- Mover o pivô regrava as coordenadas locais da malha de forma compensada (a geometria no mundo não muda); por isso o rebase recusa primitiva paramétrica, modificadores, rig e objeto bloqueado, e a interface mostra o motivo.
- Animar vértices à mão depende de topologia estável; operações que mudam topologia entram só por nós (ADR-C16, item 4).

## Next

Concluir a Onda A, rodar os gates no fim e só então abrir B1 e B4, que destravam o resto.
