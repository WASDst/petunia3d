# `crates/ui-slint/ui/`

Definições de interface declarativa na linguagem Slint (`.slint`):
- `app.slint`: Layout completo do shell, componentes visuais (Top bar, Tool Tray, Viewport, Parts, Asset Drawer, Inspector, Command Palette, Modais), e contratos de dados reativos bidirecionais.
- `animate.slint`: Componentes do workspace Animate (`AnimSlider` com draft/commit, cartões de Motion, chips, corpos do Inspector e transporte in-canvas). Só lê propriedades e emite `action(ação, argumento, valor)`; todo texto vem do Rust por `TextId`.
- `tokens.slint`, `petunia_icons.slint`: tokens de design e ícones próprios.
