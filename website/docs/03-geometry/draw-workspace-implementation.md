# DRAW — integração de formas, volumes e interface Qt

> Implementação iniciada em 2026-10-18; branch `codex/qt-integration`, base `92c22caa23f96f85f18e72a5e962a1d8831694db`. Gates automatizados executados em 2026-10-18 (ver [Gates executados](#gates-executados-em-2026-10-18)); gates manuais de plataforma continuam pendentes. Este registro complementa os contratos aprovados; não transforma propostas em decisões.

## Autoridade e limites

A decisão mais recente de frontend é ADR-C11/Q07: Qt 6 Widgets apresenta o estado do editor neutro. DRAW e POLY continuam compartilhando documento, seleção de malhas, geometria e Undo. O nome legado `SlintUiBridge` não introduz um frontend Slint.

Contratos de geometria: [unificação DRAW/POLY](./draw-unification-proposal.md), [splines e formas](./splines-planar-shapes.md), [snap e workplane](./snapping-inference-workplane.md), [geradores](./generators-implementation.md), [Shape Builder](./shape-builder-3d.md) e [modelagem direta](./direct-modeling-semantics.md). Orientação de GUI: documentos vigentes de `09-ui`, com o redesign visual aprovado em 11/10. A proposta H10/H11 não é tratada como decisão ratificada.

## Estado autoral e interação

`PlanarShape` referencia uma spline externa e splines internas. `ProfileResource` e `ProfileWorkplane` são aliases de compatibilidade. Anéis legados e espessura legada são promovidos a contornos explícitos na fronteira de comandos/carregamento, sem preservar dois autores para o mesmo anel.

`GeometryGenerator` persiste Primitive, Extrude, Revolve ou Sweep. `Asset::source_mesh` apresenta o resultado derivado; `asset.mesh` fica vazio em novos objetos gerados. O cache não é serializado. Falha de avaliação esvazia o resultado e mantém a fonte com diagnóstico. `MakeGeneratedEditableCmd` é a conversão explícita, transacional e reversível.

Gestos de curvas, dimensões e transformações usam `ProfileAuthoringPreview` fora do documento. O gesto acumula mudanças numa origem fixa, confirma com um comando e cancela descartando a prévia. Revisões obsoletas rejeitam a confirmação. A edição paramétrica de volumes também usa prévia externa e uma confirmação.

O plano de construção e a câmera salva pertencem à sessão de cada viewport. Selecionar outro plano não gira a câmera; Olhar para o plano é explícito. Entrar na navegação DRAW captura a câmera e usa o frame completo, incluindo rotação dentro do plano. Sair restaura a câmera correspondente.

A seleção ativa de forma, contorno, ponto e segmento pertence à sessão comum do editor e é restaurada com o histórico. O Inspector permite editar posições, tangentes e modos Broken/Aligned/Mirrored com valores numéricos; trocar DRAW/POLY preserva fontes, seleção e Undo.

Os controles Qt apresentam formas, contornos, contexto de pontos/segmentos/regiões, desenho interativo de primitivas planares, plano, inferência, parâmetros e confirmação de volumes. Campos numéricos com scrub usam início/prévia/confirmação/cancelamento. Sweep exige escolha explícita do caminho; curvas abertas criadas em DRAW são elegíveis e preservam seu plano salvo.

Smart Snap tem escolha própria, separada de Grid e Increment. Malhas e contornos persistentes entram na mesma consulta em pixels, com proveniência de vértice/aresta/face/ponto/segmento. Alvos derivados indicam estabilidade limitada. O passo angular é configurável. A trava de guia tem modificador remapeável e ação persistente acessível.

Shape Builder de sólidos aceita exatamente dois operandos. A preparação calcula AOnly, Intersection e BOnly e separa componentes conectados. A interação consulta BVHs prontas, com ciclo de regiões internas e X-ray temporário. Unite/Remove/Extract confirmam uma transação e preservam objetos sem regiões alteradas. Resultado booleano vazio é válido; falha do kernel é distinta. Há limites explícitos para triângulos, componentes, amostras e overlays. Faces herdadas conservam UV por corner e referências aos materiais e à pintura de origem numa tabela local do resultado. O renderer monta um atlas derivado limitado a 4096² para apresentar essa tabela; isso não modifica os UV autorais nem cria materiais no projeto. O GLB resolve as superfícies em partes numa cópia de exportação.

## Matriz complementar

Os registros anteriores permanecem em seus documentos e no tracker. Esta tabela acompanha apenas esta integração.

| Contrato / IDs          | Implementação                                              | Status/check (implementação) | Gates automatizados (2026-10-18) |
|-------------------------|------------------------------------------------------------|------------------------------|----------------------------------|
| G05/G06 — curvas/formas | Contornos persistentes, edição contextual e prévia externa | IN PROGRESS                  | pass                             |
| G07 — workplane/snap    | Frame por viewport, Smart e proveniência, trava de guia    | IN PROGRESS                  | pass                             |
| G14 — Shape Builder     | Sessão de dois sólidos, regiões/BVH e commit               | IN PROGRESS                  | pass                             |
| G12 — geradores         | Fonte persistente, caches derivados, conversão explícita   | IN PROGRESS                  | pass                             |
| G15 — Push/Pull         | Região livre cria extrusão; mantém contrato de face        | IN PROGRESS                  | pass                             |
| H18 — interface Qt      | Controles de formas, planos, volumes e composição          | IN PROGRESS                  | pass (offscreen); manual pending |

## Verificação e riscos

Testes foram escritos para fontes, persistência ZIP, holes, altura negativa, eixos de revolução, sweep explícito, sessões de viewport, sólidos separados, bloqueios e integração Qt/Undo/cancelamento. Por solicitação do usuário, execução e compilação são feitas somente na fase final. A existência desses testes não confirma seu resultado.

### Gates executados em 2026-10-18

Ambiente: Linux, Rust 1.98.1 (`rust-toolchain.toml`), `CARGO_BUILD_JOBS=1`, `QT_QPA_PLATFORM=offscreen` para Qt. Árvore de trabalho sobre `92c22caa`, ainda não commitada.

| Gate                                                                                                              | Resultado                      |
|-------------------------------------------------------------------------------------------------------------------|--------------------------------|
| `cargo fmt --all -- --check`                                                                                      | pass                           |
| `cargo clippy --all-targets -- -D warnings` (config, mesh, project, core, module_model, render, editor, ui_cxxqt) | pass                           |
| `cargo test` (config, mesh, project, core, module_model, render, editor)                                          | pass — 1167 pass, 0 fail       |
| `editor_parity -- --test-threads=1`                                                                               | pass — 33/33                   |
| `bridge_test -- --test-threads=1 --skip test_widgets_studio_window_creation`                                      | pass — 10/10                   |
| `cargo run --example qt_parity_acceptance` (XDG isolado)                                                          | pass — 62 checks, 12 de DRAW   |
| Inspeção das capturas `draw-points`, `draw-shape`, `draw-volume-preview`, `draw-generated`                        | pass, com as pendências abaixo |

Regressão encontrada e corrigida no fechamento: um clique sem arrasto do Draw Profile criava o nó só na prévia e nunca o confirmava, porque apenas o caminho de arrasto chamava `profile_pointer_up`. O teste `draw_point_gesture_never_writes_authoring_until_release_and_cancels_cleanly` cobre o caso.

Pendências visuais observadas nas capturas:

- A grade de referência tem extensão fixa de 20 m (`crates/render/src/scene.rs`); quando a câmera planar enquadra uma forma longe da origem, parte da viewport fica sem grade. Comportamento anterior a esta mudança.
- A aba Gerador desabilita, mas não oculta, os campos de outros tipos de gerador (por exemplo, eixo e ângulo de revolução num Extrude).
- A barra de status mostra "No selection" em inglês com a interface em pt-BR; o campo "Nome da forma" corta o texto.

Aceitação nativa visual em GPU real, Windows, leitor de tela, DPI e hardware modesto exigem evidência própria. O gate Q07 anterior não substitui os gates desta mudança. A permanência dos decodificadores e APIs legados é compatibilidade, não evidência de que a migração inteira dos capítulos G05–G15 esteja concluída.
