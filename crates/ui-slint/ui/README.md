# `crates/ui-slint/ui/`

Definições de interface declarativa na linguagem Slint (`.slint`):
- `app.slint`: composição/orquestração de `PetuniaSlintShell` e contratos de dados com o Rust. Durante o Slint Rescue, regiões independentes são extraídas progressivamente sem alterar os contratos públicos.
- `types.slint`: structs públicas trocadas com o Rust (`CommandItem`, `SceneItem`, `MenuEntry`…).
- `shell/header.slint`: header global isolado (menus, workspace e ações globais), sem dependência de documento/mesh.
- `shell/tool_rail.slint`: Tool Rail contextual isolado; recebe projeções/arrays e emite ações, enquanto `app.slint` continua dono do estado.\n- `tokens.slint`: tokens de design (`DesignTokens`, `ColorPresets`, `Motion`, `Tr`). Único arquivo com cores, fontes e sombras literais (`cargo run -p xtask -- ui-lint`).
- `components/base.slint`: componentes base do design system v2 (`IconButton`, `Segmented`, `DropdownButton`, `PropertyRow`, `EmptyState`, `KeyHint`, `CommandSearchField`).
- `components/controls.slint`: controles existentes do shell (`TopAction`, `ToolButton`, `NumericField`, `Vector3Field`, `MenuDropdown`…).
- `components/feedback.slint`: feedback transversal (`RichTooltip`, `StatusToast`). `components/paint_canvas.slint`: superfície do canvas 2D.
- `inspector/sections.slint`: `InspectorSection`, `InspectorPill` e os corpos das seções (Parts, Transform, Material, Object, Modifiers, Quick Actions).
- `dialogs/references.slint`: cartão de slot do Gerenciador de Referências.
- `gallery.slint`: galeria dos componentes base (`cargo run -p petunia_ui_slint --example gallery`).
- `animate.slint`: componentes do workspace Animate. Só lê propriedades e emite `action(ação, argumento, valor)`; todo texto vem do Rust por `TextId`.
- `petunia_icons.slint`: ícones próprios.

- `shell/right_column.slint`: host transitório da coluna direita; controla chrome, rail, hover/peek, header e scroll via `@children`, preparando `Structure + Properties`.
- `shell/workspace_drawer.slint`: container inferior reutilizável por workspace; posicionamento/splitter separados do conteúdo.
- `shell/status_bar.slint`: barra de status persistente; contexto contínuo separado de toast.
- `viewport/context_bar.slint`: moldura da Context Bar da viewport com conteúdo injetado por workspace.
- `workspaces/model/asset_library.slint`: conteúdo da Asset Library/Prefabs desacoplado da geometria do drawer.
- `viewport/view_bar.slint`: chrome da View Bar, com controles específicos ainda fornecidos por `@children`.
- `viewport/overlays.slint`: overlays passivos de seleção, skeleton e previews; não recebe input nem altera domínio.
- `viewport/viewport.slint`: host da viewport principal; owns superfície, resize/HiDPI e imagem GPU, enquanto input/picking/tool sessions permanecem no chamador.
- `viewport/input_router.slint`: recognition layer de pointer/gesture; mantém apenas estado efêmero e emite callbacks semânticos, sem acesso a Project/Mesh/ToolSession.
- A preferência `drag-threshold-px` alimenta diretamente o recognition layer da viewport; click→drag não usa mais threshold fixo no shell.