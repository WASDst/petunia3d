# `crates/ui-slint/src/`

Módulos Rust do backend e integração do frontend Slint:
- `lib.rs`: Bridge de intenções, view models reativos, loop de aplicação e inicialização do shell.
- `animate.rs`: Bridge do workspace Animate (cap. 45 F2): `AnimateIntent` (o markup emite `(ação, argumento, valor)`), `AnimateViewModel`, tick de reprodução, projeção do esqueleto posado e sincronização **no lugar** das listas/sliders. Compila sempre; só a pill ANIMATE depende da feature `animation-workspace`.
- `commands.rs`: Integração do catálogo de comandos semânticos com a Command Palette.
- `files.rs`: Serviço de diálogo nativo de arquivos assíncrono (abrir/salvar projeto).
- `numeric.rs`: Lógica de scrubbing, fine-stepping e clamping para inputs numéricos de precisão.
- `overlay.rs`: Gerenciamento da pilha de overlays LIFO com suporte a Escape e click-away.
- `theme.rs`: Adaptador dinâmico de tokens de design e registro de temas do Petunia3D.
- `viewport_gpu.rs`: Conexão com pipeline gráfico WGPU e render off-screen exportado para imagem Slint.
- `viewport_soft.rs`: Rasterizador e wireframe de fallback por software para compatibilidade em ambientes sem suporte WGPU/Vulkan.


## Bridge modular em migração

- `bridge/viewport.rs`: navegação, resize/HiDPI, render da viewport, seleção/cursor/context, hover e coordenação de transform/gizmo. Picking e matemática permanecem no bridge/core.
- `bridge/model.rs`: callbacks de Profile/DRAW que acontecem sobre a viewport.
- `bridge/paint.rs`: begin/update/end do traço PAINT sobre a viewport 3D.

`callbacks.rs` continua sendo o agregador temporário, mas não deve receber novos blocos de viewport quando já existir um módulo de bridge responsável.
