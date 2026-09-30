# Current Project State

- Project: **Petunia3D**
- Prumo: **0.5.1**
- Current phase: **Wave 11 — Extensibility, Plugins & Automation (Ativa)**
- Canonical Specification & SSOT: [`docs/bible/`](docs/bible/index.md) (155 P3D specs, 17 capítulos constitucionais, 15 seções, 3 adendos e 36 capítulos de fundação unificados)
- Referência de UI e interação (ADR 006, 2026-09-29): o próprio sistema Petunia ([cap. 23](docs/bible/foundations/23-macroarquitetura-interface.md), [24](docs/bible/foundations/24-design-system-tokens-estados.md), [36](docs/bible/foundations/36-ui-baseline-temas-plugin-panels.md)) e a pesquisa de interação do [cap. 45](docs/bible/foundations/45-pesquisa-interacao-modelagem-referencias.md). `docs/image-references/Blender.svg` deixou de ser referência canônica e permanece apenas como material histórico.
- Decisão de produto 2026-09-29 ([ADR 006](docs/architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)): workspaces **DRAW + POLY** no lugar de MODEL e gramática única de ferramenta (persistente, arrastar, valor digitado, clicar-mover-clicar, RMB = menu). Implementação em ondas no branch `design/draw-poly-interaction`: Onda 1 (viewport em pixels físicos + MSAA 4× e bugs comprovados) e Onda 2 (`ToolSession` no core; ferramentas persistentes; "Última operação"; RMB = menu; navegação nunca suspensa) com testes, clippy e gates verdes em 2026-09-30; captura nativa e teste com usuários pendentes ([matriz Onda 1](docs/development/viewport-crispness-gap-matrix.md), [matriz Onda 2](docs/development/tool-session-gap-matrix.md)).
- Current implementation status: **Conformidade em revisão**. A alegação histórica de Waves 0–10 totalmente concluídas não certifica o frontend Slint. Ver [plano de paridade](docs/development/viewport-parity-plan.md) e [matriz de gaps](docs/development/viewport-gap-matrix.md). Nesta rodada, 169 testes Slint, 22 Paint e 4 UV passaram; a paridade visual e a reprodução manual seguem pendentes.
- **Frontend de produção**: `petunia_ui_slint` (Slint 1.18) — shell declarativo, 169 testes unitários, bridge de intents, viewport WGPU/software fallback. UI egui (`crates/ui/`) arquivada como legado de transição (`--legacy-egui` / `PETUNIA_LEGACY_EGUI=1`).
- Context methodology: **Lean Progressive Context (LPC)**
- Last updated: `2026-09-23`
- Current planning focus: [Context/workspaces/viewport](docs/development/workspace-inspector-implementation-plan.md), com próxima discussão restrita a MODEL e viewport. O plano é proposta, não implementação nem reabertura silenciosa das decisões de Paint/UV.

### Retomada e publicação de 23/09/2026

O ambiente retomado contém o checkpoint `53ec324` e alterações posteriores
preservadas na árvore de trabalho. Os hashes locais `4364977` e `956eca0`,
informados na conversa anterior, não estão disponíveis nos objetos nem no
reflog desse ambiente. O primeiro checkpoint foi publicado pela integração
GitHub como `522b1f1`, com a árvore de arquivos idêntica à original.

O estado disponível inclui seleção múltipla/retangular, transformações
projetadas, campos numéricos transacionais, histórico com identidades de estado
e o algoritmo ampliado de Connect. Handles de plano/livre, escolha do anel de
Loop Cut por hover e clipping completo do fallback software não estão presentes
nesta retomada e permanecem pendentes no plano. Não se alega recuperação integral
dos dois commits ausentes. Os checkpoints de 22/09 não tinham aceite; a
validação de 23/09 está registrada no plano de paridade.

## Current status

The premium audit supersedes the previous claim that all required operations were
complete. *(Histórico: até 2026-09-29, o alvo visual era o layout 1920×1080 de `Blender.svg`; o ADR 006 retirou essa referência canônica.)* The visual target for final production was formalised as the 1920×1080
layout of [`Blender.svg`](docs/image-references/Blender.svg),
with 268 component SVGs cataloged and verified in [`docs/image-references/extracted/index.html`](docs/image-references/extracted/index.html).

The first implementation round delivers transactional modal transforms,
extrusion/inset/bevel previews, gizmos, component hover, quad loop cut, segmented
knife input, planar slicing, atomic paint strokes and corrected camera navigation.
The second implementation round delivers the native vector icon engine ([`crates/ui/src/icons.rs`](crates/ui/src/icons.rs)),
accessible `tool_button` with compact/expanded layouts, explicit orthographic camera
controls with 6 orthogonal presets, numeric tool property fields ([`crates/ui/src/tool_fields.rs`](crates/ui/src/tool_fields.rs)),
and responsive panel refactoring.
The third implementation round (Gauntlet Loop R2) delivers the interactive Navigation
Orientation Gizmo ([`crates/ui/src/nav_gizmo.rs`](crates/ui/src/nav_gizmo.rs)) with 6 depth-sorted world axes,
zoom/pan/perspective action handles, 3D Cursor overlay with `Shift+RMB` placement,
contextual RMB menus for sub-elements and objects, and direct Viewport Shading mode selectors.
The fourth implementation round delivers the Canonical Desktop UI Architecture matching [`Blender.svg`](docs/image-references/Blender.svg):
- Design tokens centralizados ([`crates/ui/src/tokens.rs`](crates/ui/src/tokens.rs))
- Gerenciador de ícones rasterizados embutidos com tingimento reativo ([`crates/ui/src/app_icons.rs`](crates/ui/src/app_icons.rs)) consumindo os PNGs reais do Figma em [`assets/ui/icons/toolbar/`](assets/ui/icons/toolbar/)
- Macroestrutura de 6 níveis componentizada: Main Header ([`crates/ui/src/main_header.rs`](crates/ui/src/main_header.rs)), 3D View Bar ([`crates/ui/src/viewport_bar.rs`](crates/ui/src/viewport_bar.rs)), Toolbar 40px ([`crates/ui/src/toolbar.rs`](crates/ui/src/toolbar.rs)), Outliner hierárquico ([`crates/ui/src/outliner.rs`](crates/ui/src/outliner.rs)), Painel de Propriedades modular com abas ([`crates/ui/src/properties_panel.rs`](crates/ui/src/properties_panel.rs)), Timeline de animação ([`crates/ui/src/timeline.rs`](crates/ui/src/timeline.rs)) e Status Bar ([`crates/ui/src/status_bar.rs`](crates/ui/src/status_bar.rs)).
The fifth implementation round delivers modern egui ecosystem integration and specialized crates under strict Petunia Design System sovereignty:
- Registro centralizado de ícones (`IconRegistry`, `PetuniaIcon`) em [`crates/ui/src/icon_registry.rs`](crates/ui/src/icon_registry.rs) consumindo 9 PNGs de toolbar e 15 PNGs de abas de properties extraídos do Figma ([`assets/ui/icons/properties/`](assets/ui/icons/properties/)), com decodificação alfa e tingimento dinâmico.
- Componentes soberanos de UI (`PetuniaToolbarButton`, `PetuniaPropertyTabButton`, `PetuniaWorkspacePill`, `PetuniaSearchBox`) em [`crates/ui/src/widgets.rs`](crates/ui/src/widgets.rs).
- Integração bidirecional de gizmos 3D via `transform-gizmo-egui` em [`crates/ui/src/transform_gizmo_integration.rs`](crates/ui/src/transform_gizmo_integration.rs).
- Hierarquia de cena moderna com `egui_ltreeview` em [`crates/ui/src/outliner.rs`](crates/ui/src/outliner.rs) com busca instantânea e toggles de visibilidade.
- Sistema de docking e layout de múltiplos painéis com `egui_tiles` e `PetuniaTilesBehavior` em [`crates/ui/src/tiles_workspace.rs`](crates/ui/src/tiles_workspace.rs).
- Diálogos de arquivos de sistema multiplataforma com `egui-file-dialog` em [`crates/ui/src/file_dialog_service.rs`](crates/ui/src/file_dialog_service.rs).
- Suíte de testes UI headless com `egui_kittest` em [`crates/ui/tests/kittest_ui_flows.rs`](crates/ui/tests/kittest_ui_flows.rs).
The sixth implementation round delivers UI consolidation, dead controls cleanup, Outliner redesign, and Blender-authentic high-visibility vector icons:
- Novo sistema vetorial de ícones na toolbar ([`crates/ui/src/icons.rs`](crates/ui/src/icons.rs), [`crates/ui/src/icon_registry.rs`](crates/ui/src/icon_registry.rs)) com traço reforçado (2.0px–2.6px) e paleta multicolorida autêntica do Blender (3D Cursor com anel vermelho e segmentos brancos, eixos cardeais RGB para Move/Scale/Rotate/Transform, lápis ciano com madeira dourada para Annotate, régua amarela com miras para Measure, cubo isométrico laranja com badge '+' para Add Primitive, e realces em amarelo/laranja vivo para ferramentas de modelagem).
- Redesenho completo do Outliner ([`crates/ui/src/outliner.rs`](crates/ui/src/outliner.rs)): árvore reativa aos assets reais do projeto (`project.assets`), toggle funcional de visibilidade `👁` sincronizado com pipelines de renderização WebGPU e OpenGL, menu de contexto (RMB) para duplicar/deletar e Galeria Rápida de Primitivas instanciadas na coordenada exata do 3D Cursor.
- Eliminação de botões sem funcionalidade: remoção dos botões mortos de abas de properties (`Render Engine`, `Output`, `View Layer`, `Scene`, `World`, `Collection`), pills estáticos de cena no header, menu fictício `Render` e painel de Timeline.
- Reorganização da Viewport Bar em 5 clusters semânticos com 4 botões esféricos de sombreamento (`○ Wire`, `● Solid`, `◐ Material`, `☼ Render`).
- Correção de atalhos e seleção: `Tab` e teclas numéricas `0..=4` com propagação sem perda no winit/egui e unificação para 4 alvos de seleção (`Objeto [Tab/0]`, `Vértice [1]`, `Aresta [2]`, `Face [3]`).
The seventh implementation round delivers interactive 3D measurement and grease pencil annotation, floating viewport bar refinement, dedicated asset library drawer, dynamic theme and icon pack systems, 8 canonical keymap profiles, and TOML i18n:
- Ferramenta interativa de régua e medição 3D ([`crates/ui/src/measurement.rs`](crates/ui/src/measurement.rs)): snapping magnético a vértices de malhas ativas, graduações métricas, e badge flutuante com distância euclidiana e deltas cartesianos ($\Delta X, \Delta Y, \Delta Z$).
- Ferramenta interativa de anotação e rascunho 3D ([`crates/ui/src/annotation.rs`](crates/ui/src/annotation.rs)): rascunho grease-pencil com projeção em superfícies de malhas ativas e plano do 3D Cursor.
- Barra da viewport aperfeiçoada ([`crates/ui/src/viewport_bar.rs`](crates/ui/src/viewport_bar.rs)): botão explícito `[➕ Add+ ▾]`, esferas flutuantes compactas de shading estilo Blender (`○`, `●`, `◐`, `☼`) e botões de seleção otimizados sem sobreposição.
- Gaveta dedicada da biblioteca de assets ([`crates/ui/src/asset_library_drawer.rs`](crates/ui/src/asset_library_drawer.rs)): gerenciamento de modelos, pré-visualizações, métricas de polígonos e instanciação no 3D Cursor, com clara distinção entre salvar asset (no projeto) e salvar o projeto (.petunia).
- Sistema de temas orientado a tokens ([`crates/config/src/theme.rs`](crates/config/src/theme.rs), [`crates/ui/src/tokens.rs`](crates/ui/src/tokens.rs)): 4 temas nativos (`petunia-dark`, `petunia-light`, `petunia-capuccino`, `petunia-tokyo-nights`) e carregamento via TOML com fallback seguro.
- Sistema de pacotes de ícones ([`crates/ui/src/icon_registry.rs`](crates/ui/src/icon_registry.rs)): 5 pacotes (`Petunia`, `Phosphor`, `Tabler`, `Iconoir`, `Lucide`) com cascading fallback e espessura refinada para 1.8–1.9px.
- 8 perfis canônicos de teclado ([`crates/config/src/keybinds.rs`](crates/config/src/keybinds.rs)): mapeamentos completos para Blender, Maya, 3ds Max, Cinema 4D, Notebooks e detecção automática de conflitos.
- Modal de configurações centralizado ([`crates/ui/src/settings_modal.rs`](crates/ui/src/settings_modal.rs)) e internacionalização em TOML (`pt-BR`, `en`).
The eighth implementation round delivers the UI Reorganization, Hierarchy Consolidation & Ergonomics Refinement:
- Contextual Modeling Shelf ([`crates/ui/src/contextual_shelf.rs`](crates/ui/src/contextual_shelf.rs)): cápsula flutuante na base da viewport reagindo ao modo ativo (`Model+Edit`, `Model+Object`, `Paint`, `UV`, `Animate`) com escudo de eventos de mouse para prevenir interferência no raycasting tridimensional.
- Retractable Asset Browser ([`crates/ui/src/asset_browser.rs`](crates/ui/src/asset_browser.rs)): painel retrátil à esquerda (220–340px) com filtro por categoria, busca padronizada com `PetuniaSearchBox`, cartões de modelo e salvamento direto na biblioteca interna.
- Viewport Bar em 6 clusters semânticos ([`crates/ui/src/viewport_bar.rs`](crates/ui/src/viewport_bar.rs)): dropdown de modo (`Object` vs `Edit`), alvos de seleção (`⬝ Vértice`, `╱ Aresta`, `▨ Face`) exibidos **exclusivamente** no modo de edição, menus com `➕ Add+ ▾` e menus contextuais de malha/objeto, transform/pivot, snapping/proportional editing, overlays/x-ray e esferas de shading.
The ninth implementation round delivers UI Enhancements & Interactions (Vertex Hover Demarcation, Toolbar Edit Tools, WGPU X-Ray, Reference Images, Outliner Collections/Lock/Isolate, and Vibrant Properties Tabs):
- Demarcação visual de vértices em modo de edição com hover dourado e anel halo ciano brilhante (`#64dcff`) em [`crates/ui/src/viewport_interaction.rs`](crates/ui/src/viewport_interaction.rs).
- Expansão dinâmica da toolbar esquerda ao entrar em Edit Mode com as 9 ferramentas de modelagem de malha em [`crates/ui/src/toolbar.rs`](crates/ui/src/toolbar.rs).
- Pipeline WebGPU e picking X-Ray com translucidez da malha (`alpha ~ 0.45`) e arestas sem oclusão de profundidade em [`crates/render-wgpu/src/lib.rs`](crates/render-wgpu/src/lib.rs) e atalho `Alt+Z`.
- Reintegração de imagens de referência no menu `➕ Add+ ▾`, na contextual shelf (`🖼 Referência`) e no Outliner em [`crates/ui/src/outliner.rs`](crates/ui/src/outliner.rs).
- Suporte a pastas/coleções (`📁 Coleções`), bloqueio (`🔒 Lock`) e isolamento (`⌖ Isolar` / `Numpad /`) no Outliner e no motor de transformação modal.
- Abas de propriedades ampliadas para `32x28px` com container emoldurado e cores semânticas vibrantes do Blender (Tool `#3169e3`, Object `#e67e22`, Modifiers `#00a8ff`, Data `#2ecc71`, Material `#e84393`) em [`crates/ui/src/properties_panel.rs`](crates/ui/src/properties_panel.rs).
- Descongestionamento da sidebar direita: redução para estritamente `Outliner` e `Properties`, com inspector de transform em grid triaxial RGB e cor de material unificada com paleta do projeto.
- Especialização da toolbar vertical esquerda: restrita às 8 ferramentas primárias de interação contínua.
- Barra de status inferior reorganizada em 3 blocos: identidade de projeto (`● Salvo` / `○ Não salvo`) e dicas à esquerda, mensagens no centro, e telemetria de cena agregada (`Tris`, `Verts`, `Objs`, `ms`) à direita.
The tenth implementation round delivers complete Annotations & Measurements Architecture with Undo/Redo (Ctrl+Z), Dedicated Outliner Collections, Subgrouping, Strict Confinement, and Transform Properties:
- Migração de anotações e medidas para o domínio de dados persistente [`petunia_project::Project`](crates/project/src/lib.rs), eliminando o descarte no histórico e garantindo suporte total a Undo/Redo (`Ctrl+Z` e `Ctrl+Shift+Z`) com checkpoints transacionais.
- Coleção dedicada `📝 Anotações` no topo do Outliner ([`crates/ui/src/outliner.rs`](crates/ui/src/outliner.rs)) com cor ciano (`#00d2d3`), alternância coletiva e individual de visibilidade (`👁`) e bloqueio (`🔒`), criação de subgrupos internos (`📁 Subgrupo`) e confinamento estrito impedindo que anotações sejam movidas para coleções de malhas 3D.
- Coleção dedicada `📏 Medidas` no topo do Outliner ([`crates/ui/src/outliner.rs`](crates/ui/src/outliner.rs)) com cor amarela (`#feca57`) e controles estritamente limitados a visibilidade e exclusão.
- Inspetor de propriedades e transformações de anotações no Painel de Propriedades ([`crates/ui/src/properties_panel.rs`](crates/ui/src/properties_panel.rs)): campos de nome, visibilidade, bloqueio, subgrupo, aparência de traço (cor e espessura), e seção completa de Transformação (Location X/Y/Z, Rotation X/Y/Z em graus, Scale X/Y/Z e redefinição).
- Manipulação tridimensional por Gizmos no Viewport ([`crates/ui/src/viewport_interaction.rs`](crates/ui/src/viewport_interaction.rs)): cálculo em tempo real de matriz de transformação (`item.transform_matrix()`), renderização de gizmo no centro da anotação, arraste por eixos/planos, cancelamento por `Escape` e gravação de checkpoint no término.
The eleventh implementation round delivers Viewport Axis Locking Indications & Controls across 3 synchronized layers:
- Linhas-guia 3D infinitas projetadas no espaço da cena (`modal_viewport.rs`, `viewport_interaction.rs`): linhas atravessando o pivô com halo de brilho e cores canônicas do Blender (`AXIS_X` vermelho `#e03c42`, `AXIS_Y` verde `#62c934`, `AXIS_Z` azul `#3182f6`) e quad translúcido sombreado para planos (`Shift+X/Y/Z`).
- HUD flutuante dinâmico de alta visibilidade (`modal_viewport.rs`): pill com moldura na cor do eixo, badges semânticos `[ 🔒 EIXO X ]` e dicas contextuais de atalhos.
- Controles interativos na Viewport Bar (`viewport_bar.rs`): grupo `🔒 [ X ] [ Y ] [ Z ]` no cluster de transformação com preenchimento sólido colorido e badge dinâmico (`[ 🔒 Eixo X ]`), com suporte a travamento direto ou pré-configuração.
- Sincronização centralizada no domínio (`state.rs`, `modal.rs`): controle de estado unificado em `AppState.locked_axes`, herança em `begin_modal`, `toggle_axis_lock`, `is_axis_locked` e reset limpo em `commit_modal` e `cancel_modal`.
The twelfth implementation round delivers the Official Petunia3D Living Documentation Website, xtask Automation & Drift Prevention, and GitHub Actions CI/CD:
- Website estático completo com VitePress e Mermaid em [`docs/`](docs/): landing page oficial, Primeiros Passos (6 capítulos), Manual do Usuário (11 capítulos), Workspaces (5 guias), Catálogo de Ferramentas (19 ferramentas), Personalização (temas, ícones, i18n, keymaps), Atalhos (Cheatsheet e Blender), Portal do Desenvolvedor (8 guias) e Changelog sincronizado.
- Crate de automação [`crates/xtask`](crates/xtask) integrado no workspace (`cargo xtask docs` e `cargo xtask docs-check`) validando determinismo editorial e build do VitePress.
- Pipeline de deploy contínuo em [`.github/workflows/docs.yml`](.github/workflows/docs.yml) publicando automaticamente no GitHub Pages a cada commit na branch `main`.
The thirteenth implementation round delivers Deep Interface Revision, Canonical Vector Iconography, Deduplicated Controls, and Unified Menus:
- Erradicação total de emojis Unicode em favor de mais de 20 ícones vetoriais canônicos desenhados via egui Painter ([`crates/ui/src/icons.rs`](crates/ui/src/icons.rs), [`crates/ui/src/icon_registry.rs`](crates/ui/src/icon_registry.rs)) com respeito estrito a DPI e tokens semânticos de tema.
- Fonte Única da Verdade para modos de seleção: remoção de botões duplicados da contextual shelf e centralização estrita no cabeçalho do viewport com atalhos numéricos (`1`, `2`, `3`).
- Sistema de menus unificado com `PetuniaMenuItem` ([`crates/ui/src/widgets.rs`](crates/ui/src/widgets.rs)) no padrão profissional do Blender `[Ícone] Rótulo ... [Atalho] ›` e busca dinâmica de atalhos (`Keybinds::shortcut_for`).
- Reorganização da Viewport Bar em 7 clusters responsivos com botões de visibilidade, travamento de eixos, projeção e esferas de sombreamento canônicas.
The fourteenth implementation round delivers Core V1 & Interactive Geometry Refinement (Gauntlet Loop):
- Triangulation Inspection Overlay: extração determinística de wireframe de suporte de diagonais internas (`Mesh::triangulation_wireframe`), overlay renderizado com profundidade nos backends OpenGL e WebGPU, e botão comutador na Viewport Bar.
- Flip Diagonal: inversão de diagonais de quads via rotação cíclica e Delaunay edge-flip em aresta compartilhada por triângulos (`Mesh::flip_diagonal`, `FlipDiagonalCmd`).
- Revolve 360° Selection: revolução procedimental de perfis conectados/abertos em 360° ao redor dos eixos coordenados (`Mesh::revolve_selection`, `RevolveCmd`).
- Extrude Individual Faces: extrusão desacoplada por face gerando topos disjuntos e anéis de paredes sem fusão de arestas vizinhas (`Mesh::extrude_individual`, `ExtrudeIndividualCmd`, `Alt+E`).
- Multi-Segment Rounded Bevel: chanfro de aresta com multi-segmentos e curvatura de filete em arco circular (`Mesh::bevel_selected_segments`), com costura topológica manifold e estrito fechamento.
- Guarded Metric Inset: inset métrico protegido com amortecimento dinâmico step-down prevenindo inversão de normais e auto-interseções topológicas.
Final gates: 100% dos testes passando em todo o workspace (incluindo 60 testes em `petunia_mesh`, 42 unitários e 11 de comandos em `petunia_core`, 88 unitários e 17 de fluxos em `petunia_ui`, totalizando 277+ testes); cargo clippy estrito (`-D warnings`), `cargo fmt --check`, `cargo run -p xtask -- docs-check` e `cargo run -p xtask -- arch-check` com 100% de conformidade.
The fifteenth implementation round delivers Wave 6 (Scene, Assets, Outliner & Inspector) and UI/Viewport Polish:
- Seleção multi-objeto no Viewport 3D no modo Objeto por cálculo de distância euclidiana da câmera e suporte a toggle com Shift.
- Responsividade imediata do Outliner sem arrasto fantasma, seleção direta por rótulo, botão dedicado de exclusão (`PetuniaIcon::Delete`) e suporte nativo a teclas `Delete`, `Backspace` e `Shift+D`.
- Mecanismo transparente de reancoragem de painéis com botão `⇲ Dock` e banner contextual na barra lateral.
- Estabilidade dimensional anti-expansão horizontal em 100% das janelas modais.
- Unificação Canônica da Documentação (Single Source of Truth) sob `docs/bible/`, integrando 155 especificações P3D, 17 capítulos constitucionais, 15 seções temáticas, 3 adendos e 36 capítulos de fundação ao VitePress com links limpos e rastreáveis.
The sixteenth implementation round delivers the complete Stack Modernization (Waves M1, M2, M3, and M4):
- Wave M1 (Toolchain): Rust 2024 edition, resolver 2, workspace inheritance, and strict Clippy zero-warnings compliance.
- Wave M2 (Host & Graphics): egui 0.36, wgpu 30, and dual-backend rendering (WGPU + GL) fully stabilized and tested.
- Wave M3 (P0 Dependencies): tracing, slotmap, rayon/flume jobs, tempfile atomic IO, manifold-rust 3D booleans, geo profiles, tobj/gltf-json parsers, egui_extras image/svg, and eframe host adapter.
- Wave M4 (P1 Features & Ecosystem Hardening): versioned package format (.pkg zip container), sandboxed Lua plugins (`petunia_plugins` with mlua), Model Context Protocol server (`petunia_mcp` with rmcp 3.3 over stdio), generic UV unwrapping (`xatlas-rs-v2`), live filesystem watch reloading (`WatchService`), UI enhancements (taffy flex layout, twill styles, devtools loopback with inspection and egui_mcp bridge, criterion benchmarks), CLI and desktop MCP stdio endpoints (`petunia3d --mcp-stdio`, `petunia-cli mcp`), and full cargo-deny supply-chain compliance.

The seventeenth implementation round delivers UI/UX Polish, Theming, Icon System & Viewport Ergonomics (Resolution of 13 Critical Defects):
- Propagação integral de temas e tokens para todos os componentes egui e cor de limpeza do viewport em OpenGL e WebGPU.
- Tradução (i18n) completa com cobertura total em `pt-BR.toml` e `en.toml` para ferramentas, shelf contextual, menus e viewport.
- Resolução e preview autêntico dos pacotes de ícones (`Petunia`, `Phosphor`, `Tabler`, `Iconoir`, `Lucide`) com motor vetorial nativo em egui Painter eliminando glifos quebrados/tofu.
- Redesenho da contextual modeling shelf com cálculo dinâmico irrestrito e botão de referência abrindo o modal do Reference Manager.
- Gerenciador de Imagens de Referência redesenhado com cartões responsivos, miniatura clicável para picker e sliders de ajuste fino.
- Dropdown menus e popovers contidos em largura (~148-165px) com visual reativo a temas.
- Popover de configuração de grade 3D (`⚙`) com sliders para tamanho, subdivisões, opacidade e eixos isométricos.
- Sincronização de overlays de eixos e bússola/HUD com os renderizadores OpenGL e WGPU.
- Modo X-Ray com alfa translucidez (0.45) e arestas sem oclusão de profundidade.
- Snapping magnético na viewport durante translações.
- Translação livre de objetos e vértices no plano da câmera (`ModalConstraint::Free`).
- Interceptação antecipada da tecla `Tab` no loop Winit prevenindo consumo pela navegação de foco do egui.
- Reorientação do gizmo de transformação ao alternar entre eixos Global e Local (`local_axes_for_mesh`).

The eighteenth implementation round delivers Wayland Crash Elimination & In-Canvas File Dialog System:
- Eliminação do segmentation fault no Wayland / Mesa Intel (`SEGV_MAPERR` em `wl_proxy_destroy` no `smithay-clipboard`) via `SafeDisplayTarget` que contorna a thread instável de seleção primária delegando a manipulação de clipboard com segurança para o `arboard`.
- Implementação de encerramento seguro (`exiting` e `CloseRequested`) em `WgpuApp` e `GlApp`, garantindo liberação limpa de contextos gráficos e janelas antes da finalização do event loop.
- Migração integral para o sistema de diálogos in-canvas com `egui-file-dialog` em `file_dialog_service.rs`, eliminando completamente as chamadas a seletores nativos do SO (`rfd` / portal DBus `ashpd`), warnings de `zbus::proxy` e abrindo seletores diretamente na interface da aplicação para imagens de referência, projetos e paletas.

The nineteenth implementation round delivers Wave 7 (Materials, Texture, UV & Paint Engine — P3D-050 a P3D-065, P3D-132 a P3D-134, P3D-140):
- **Modelo Canônico de Materiais e Canais PBR (`crates/project/src/material.rs`)**:
  - Modelo unificado Single Source of Truth: `ShaderProfile` (`Pbr`, `Unlit`, `Toon`, `Glass`, `Emissive`), `AlphaMode` (`Opaque`, `Mask`, `Blend`), canais de textura (`Albedo`, `Normal`, `Roughness`, `Metallic`, `Emission`, `Height`), conversão bidirecional Roughness ↔ Glossiness e validação/sanitização contra valores não finitos.
  - Vínculo direto de slots de material em faces de malha (`Face.material_slot`) e ativos de cena (`Asset.material_id`), com normalização e reparo transparente de projetos legados em `Project::validate()`.
- **UV Workspace & Algoritmos de Desdobramento (`crates/module-uv`, `crates/mesh/src/uv.rs`)**:
  - Algoritmos de projeção UV: Mapeamento Planar (`project_planar`), Mapeamento Cúbico / Box nos 6 planos cardeais (`project_cube`) e Auto Unwrap não-destrutivo integrado ao `xatlas-rs-v2` (`unwrap_auto`).
  - Transformações UV no workspace: translação, escala e rotação incremental de 90° para ilhas e vértices UV com botões dedicados na UI.
- **Motor de Pintura 2D & Projeção 3D sobre Malha (`crates/module-paint/src/lib.rs`)**:
  - 5 pincéis canônicos: Pixel Brush rígido, Soft Brush com atenuação quadrática suave, Borracha com atenuação de canal alfa, Flood Fill com tolerância de cor e Conta-gotas (Eyedropper).
  - Pintura 3D contínua sobre a superfície da malha no viewport interativo via projeção baricêntrica de coordenadas UV.
  - Isolamento de seleção e máscaras de pintura (`isolate_selection` / P3D-132), restringindo a aplicação de pincéis exclusivamente às faces selecionadas da malha.
- **Pilha Unificada de Camadas, Decalques e Efeitos (`crates/module-paint/src/lib.rs`)**:
  - `PaintLayerStack` determinístico com suporte a camadas raster com modos de mesclagem (`Normal`, `Multiply`, `Add`, `Screen`), decalques parametrizados (`DecalLayer` com coordenadas UV, escala, rotação em radianos e opacidade — P3D-133) e pilha de efeitos não-destrutivos (`PaintEffect::Pixelate`, `PaintEffect::Posterize` e `PaintEffect::Invert` — P3D-134).
- **Hub de Materiais no Painel de Propriedades (`crates/ui/src/properties_panel.rs`)**:
  - Interface visual completa de edição de materiais: seleção de shader profile, seletor RGBA de cor base sincronizado, sliders de roughness e metálico, escala de normal map, cor e intensidade de emissão, controles de canal alfa com cutoff, textura de albedo e swatches da paleta do projeto.
- **Integração dos Renderers WGPU & OpenGL aos Materiais (`crates/render-wgpu`, `crates/render-gl`)**:
  - Sincronização automática das propriedades de materiais, texturas de albedo e perfis de sombreamento nos pipelines gráficos do WebGPU (`render-wgpu`) e OpenGL Desktop 3.3+ (`render-gl`).

The twentieth implementation round delivers Wave 8 (Import, Export & Delivery Pipeline — P3D-068 a P3D-072, P3D-124):
- **Pipeline de Entrega e Registro de Formatos (`crates/project/src/pipeline.rs`)**:
  - Matriz desacoplada de capacidades (`FormatCapabilities`), enum canônico `FileFormat` (`Obj`, `Gltf`, `Glb`, `Pkg`), e opções configuráveis de exportação/importação (`ExportOptions`, `ImportOptions`).
  - Traits modulares de adaptadores `FormatExporter` e `FormatImporter` desacoplados de UI.
  - Implementação completa de `DeliveryPipeline` registrando adaptadores para Wavefront OBJ, glTF 2.0 (JSON), glTF 2.0 Binary (.glb) e Petunia Package (.pkg zip container).
  - Suporte completo a exportação individual (`export_single_asset` / P3D-068), exportação múltipla (`export_multiple_assets` / P3D-069), exportação em lote com tolerância a falhas parciais (`batch_export` / P3D-070), e importadores com preservação de topologia (`import_file` / P3D-071).
  - 14 testes de integração e conformance cobrindo round-trips, integridade de buffers binários, sanitização contra geometria corrompida (NaNs/infinities) e resiliência contra arquivos maliciosos ou corrompidos (`pipeline_tests.rs` / P3D-124).
- **Delegação Limpa no Core (`crates/core/src/project_service.rs`)**:
  - `ProjectService` orquestra exportação e importação via `DeliveryPipeline` sem expor detalhes internos de baixo nível e preservando backward compatibility de erros de domínio.

The twenty-first implementation round delivers Wave 9 (Documentation, QA, Release & GA Hardening — P3D-116 a P3D-121, P3D-126 a P3D-130):
- **Catálogo Automático de Referência & Tokens (`crates/xtask/src/generator.rs` / P3D-119)**:
  - Gerador determinístico de documentação canônica a partir do código: `COMMANDS.md`, `KEYBINDS.md`, `ICON_TOKENS.md`, `TEXT_TOKENS.md`, `THEME_TOKENS.md` e `SUPPORTED_FORMATS.md`, com manifesto JSON validado e 8 testes unitários de idempotência bit a bit.
- **Detecção Contínua de Drift e Sincronização do Changelog (`crates/xtask/src/main.rs` / P3D-117, P3D-120)**:
  - Quality gate permanente `cargo xtask docs-check` com verificação de drift contra referências geradas e garantia de que `docs/changelog/index.md` reflete fielmente `CHANGELOG.md`.
  - Build oficial do VitePress validado com 36 arquivos essenciais e zero links/tabelas quebradas.
- **Regressão de UI Headless com `egui_kittest` (`crates/ui/tests/kittest_ui_flows.rs` / P3D-121)**:
  - 19 testes headless de interação cobrindo alternância de modos de seleção (`Vertex`, `Edge`, `Face`), shelf contextual dinâmica em todos os workspaces (`Model`, `Paint`, `UV`) e colapso responsivo.
- **Hardening de GA e Estabilidade para Computadores Modestos (P3D-126 a P3D-130)**:
  - Verificação de ausência de vazamentos de memória e conformidade estrita com o baseline desktop low-poly.

The twenty-second implementation round delivers Wave 10 (Animation & Rigging — P3D-066, P3D-067, P3D-135, P3D-136, P3D-137, P3D-138, P3D-139):
- **Skeleton & Rig Core (`crates/project/src/rig.rs` / P3D-135)**:
  - Estrutura completa de ossos e articulações (`Bone`), esqueleto (`Skeleton`) e transformações tridimensionais (`Transform3D`).
  - Prevenção estrita de ciclos na árvore hierárquica, reatribuição de pais, cálculo determinístico de matrizes de repouso (Bind Pose) e matrizes inversas (`inverse_bind_matrix`).
  - Pesos de deformação por vértice (`VertexSkinWeight`) com normalização garantida (soma = 1.0) e algoritmo de deformação Linear Blend Skinning (`SkinData::deform_mesh`).
- **Animação Simples & Trilha de Keyframes (`crates/project/src/animation.rs` / P3D-067)**:
  - Trilhas de ossos (`BoneTrack`) com keyframes de translação, rotação e escala.
  - Interpolação linear contínua e interpolação esférica quaternion (`Slerp`).
  - Amostragem temporal contínua (`sample_pose` e `sample_skinning_matrices`) com suporte a repetição (looping) e framerates configuráveis (24/30/60 FPS).
- **Presets Canônicos de Rigging (`crates/project/src/animation.rs` / P3D-136)**:
  - Presets modulares de templates de dados: Humanoide bípede proporcional (15+ ossos), Quadrúpede de 4 patas com cauda, e criatura Multi-Leg parametrizada (aranhas/escorpiões).
- **Auto-Rig & Auto-Skin Heurístico (`crates/project/src/animation.rs` / P3D-137)**:
  - Dimensionamento proporcional automático (`auto_fit_humanoid`) adaptando o esqueleto à caixa delimitadora (Bounding Box) do modelo tridimensional ativo.
  - Cálculo geométrico automático de pesos de deformação (`compute_auto_skin_weights`) com atenuação quadrática suave em relação ao segmento do osso e atribuição dos 4 ossos mais influentes por vértice.
- **Retargeting de Animações Externas (`crates/project/src/animation.rs` / P3D-138)**:
  - Perfil de mapeamento semântico `RetargetProfile` (padrão Mixamo para Petunia Humanoid) e retargeting desacoplado de clipes.
- **Biblioteca de Ativos de Animação (`crates/project/src/animation.rs` / P3D-139)**:
  - Estrutura `AnimationAsset` integrada ao documento do projeto, com biblioteca nativa contendo os clipes de animação canônicos `Humanoid_Idle` e `Humanoid_Walk`.
- **Animation Workspace & Painel de UI (`crates/ui/src/modules_ui/animation_ui.rs`, `crates/ui/src/contextual_shelf.rs` / P3D-066)**:
  - Painel lateral no `Workspace::Animate` contendo gestão de Armatures, Inspetor de Ossos com coordenadas [X, Y, Z] e comprimento, catálogo de clipes e controles de transporte com gravação de keyframes.
  - Shelf contextual inferior com botões dedicados de presets rápidos, Auto-Rig, scrubbing de frames e transporte de reprodução.
  - 20 testes headless de fluxo UI (`kittest_ui_flows.rs`) e 47 testes unitários de domínio passando com 100% de conformidade.

Evidence is preserved in `.prumo/history/premium/`, [`docs/GAUNTLET.md`](docs/GAUNTLET.md), [`docs/GAUNTLET_HANDOFF.md`](docs/GAUNTLET_HANDOFF.md), [`docs/audits/stack-modernization/`](docs/audits/stack-modernization/) and [`docs/bible/`](docs/bible/index.md).

## Next action

**Prioridade de trabalho solicitada em 23/09/2026:** discutir e refinar somente
MODEL e viewport, começando pelas fases 1–4 do
[plano de Context/workspaces](docs/development/workspace-inspector-implementation-plan.md).
O plano completo registra PAINT/UV para preservar a arquitetura compartilhada,
mas não autoriza sua implementação nesta rodada.

**Iniciativa Paint (decisão 2026-09-16)** — permanece no roadmap canônico antes
da Wave 11. Branches
`paint/core-engine` (motor, descriptors, effects, graph headless) e
`paint/ui-redesign` (layout mini-Photoshop, painel Layers/Brush/Effects).

Fases da iniciativa, em ordem:

1. **Congelar UV-editing na UI V1**: remover a pill UV temporariamente; preservar
   `module-uv` como utilitário de "Preparar superfície" dentro do Paint (fluxo
   paint-first, P3D-063/064/065). Redesign completo do workspace UV adiado para
   o ciclo pós-Paint.
2. **Brush engine unificado**: descriptor `BrushSettings` (`size_px`, `hardness`,
   `strength`, `flow`, `spacing`) substituindo a dualidade `canvas_brush` ×
   `paint_radius`; stroke engine baseado em dabs; novo pincel **Airbrush**
   (P3D-055/056/057).
3. **Effect Stack com presets do capítulo 42**: UX "Add Effect" na pilha de
   camadas; nodes a implementar — Grain/Noise, Levels/Threshold,
   Brightness/Contrast, Hue/Saturation (P3D-134).
4. **Surface Recipe graph headless (P3D-113)**: DAG, sockets tipados, avaliador
   determinístico e cache por node; sem editor visual nesta fase.
5. **Layout Paint "mini Photoshop"**: canvas 2D central + prévia 3D; painel
   direito com Layers (drag-and-drop), Brush e Effects; dentro do shell
   congelado do cap. 36 (`MODEL / PAINT` preservado).

Após a estabilização da iniciativa Paint, retomar a **Wave 11
(Extensibility, Plugins & Automation)** cobrindo P3D-110, P3D-111, P3D-112,
P3D-141, P3D-142 e P3D-154.

### Frontend Slint — gaps conhecidos (2026-09-20)

O shell Slint (`crates/ui-slint/`) é o frontend de produção com 55 testes
unitários verdes. Gaps conhecidos em relação ao caderno (capítulos 23/36):

- **Transform modal**: sendo corrigido transacionalmente (scrubbing, commit,
  cancel — testes `transform_scrub_*` e `escape_cancels_transform` já verdes).
- **Delete/visibility/lock diretos**: corrigidos (`scene_item_visibility_and_lock_toggles`,
  `primitive_creation_and_deletion_updates_scene`, `delete_is_dirty_and_undo_restores_the_asset`).
- **Overlay identity LIFO**: corrigido (`overlay_stack_handles_escape_in_lifo_order`,
  `modal_identity_preserves_lifo_when_multiple_modals_are_open`,
  `closing_one_drawer_does_not_close_or_leave_a_ghost_for_another`).
- **Viewport resize**: corrigido (`viewport_resize_updates_backend_and_camera_aspect`).
- **Registry centralizado de ícones**: ausente no Slint; a Command Palette já
  consulta e executa o `CommandDispatcher` canônico do core.
- **Keymap profiles**: ausente — os 8 perfis canônicos do capítulo 36 ainda
  vivem somente em `petunia_config::keybinds` sem bridge no shell Slint.
- **i18n TOML**: ausente — `pt-BR.toml`/`en.toml` existem no legado egui mas
  não foram portados para o Slint.
- **Fast path GPU**: bloqueado — `viewport_gpu.rs` ainda faz staging allocation,
  GPU→CPU readback e espera síncrona por frame antes de criar `slint::Image`.
- **Layout flex/grid complexo**: em progresso — o shell usa layout declarativo
  nativo Slint, mas painéis como Properties/Outliner ainda precisam de
  refinamento de responsividade e densidade.

Esses gaps estão registrados na [auditoria Slint](docs/ui/slint-modern-audit.md).

## Recovery order

1. `ENTRYPOINT.md` or platform adapter.
2. `prumo.json` and this state file.
3. `docs/PRUMO.md` and the premium interaction plan.
4. Historical goals in `.ai/goals/` (their DONE state does not close premium work).
5. Only relevant canonical docs, symbols and tests.
