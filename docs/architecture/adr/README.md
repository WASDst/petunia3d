# Architectural Decision Records (`docs/architecture/adr/`)

## O que é este diretório?
Contém os registros formais e imutáveis de decisões arquiteturais significativas tomadas ao longo do projeto Petunia3D.

## Para que serve?
Preserva o contexto histórico, as opções avaliadas, as consequências aceitas e os critérios que justificaram cada decisão estrutural.

## Inventário
- [`001-architecture-baseline.md`](001-architecture-baseline.md): Decisão que estabelece a linha de base de Clean Architecture e separação de domínio.
- [`002-rust-opengl-stack.md`](002-rust-opengl-stack.md): Escolha técnica da stack Rust, OpenGL 3.3 Core Profile (`glow`), fallback `wgpu` e interface `egui`.
- [`003-model-parts-in-inspector.md`](003-model-parts-in-inspector.md): revisão aprovada do shell MODEL, com Parts no Inspector direito e acesso compacto.
- [`004-inspector-translucido-alca-modifiers.md`](004-inspector-translucido-alca-modifiers.md): refinamento do Inspector (translucidez, alça por proximidade, pílulas, Material/Object, modifiers, card único de ferramenta).
- [`005-modulos-inspector-dock-float-pin.md`](005-modulos-inspector-dock-float-pin.md): módulos do Inspector independentes, com card flutuante arrastável in-canvas, pin duplo (aberto + asset) e persistência por seção.
- [`006-workspaces-draw-poly-e-gramatica-unica.md`](006-workspaces-draw-poly-e-gramatica-unica.md): workspaces DRAW e POLY no lugar de MODEL, gramática única de ferramenta (persistente, arrastar, valor digitado, clicar-mover-clicar, RMB = menu) e retirada do Blender como referência canônica.
- [`../bible/foundations/32-adr-odin-para-rust.md`](../bible/foundations/32-adr-odin-para-rust.md): ADR histórica da transição de prototipagem em Odin para Rust (autoridade vigente de stack).
