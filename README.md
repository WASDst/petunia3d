# Petunia3D — modelador low-poly shape-first (Rust + Slint + OpenGL)

Modelador 3D nativo focado em criação rápida de assets low-poly para games:
desenhe a silhueta sobre a referência, gere a malha, ajuste/transforme,
prepare UV, pinte e exporte — sem exigir domínio prévio de um DCC generalista.

A interface é o shell Slint com viewport OpenGL 3.3. O domínio geométrico
permanece desacoplado da UI e é acionado por comandos transacionais com
Undo/Redo.

## Rodar

```bash
cargo run --release
PETUNIA_VIEWPORT_SOFTWARE=1 cargo run --release   # viewport por software
PETUNIA_MSAA=1 cargo run --release                # sem MSAA (hardware fraco)
cargo run -p petunia-cli -- help
```

Os atalhos pertencem a perfis configuráveis em `assets/keymaps/`; a referência
completa está em [`docs/input/keymaps.md`](docs/input/keymaps.md).

## Workspaces e capacidades

- **MODEL** — primitivas procedurais, Draw Profile, extrusão, Push/Pull,
  Inset, Bevel, Subdivide, Mirror, Merge, Symmetrize e referências
  ortográficas, atrás do `CommandDispatcher` canônico.
- **PAINT** — pincéis descritivos, camadas, efeitos, pintura 2D/3D e paleta do
  projeto.
- **UV** — projeções planar/cúbica/automática, empacotamento de ilhas,
  transformações de UV e projeção a partir da vista.
- **Command Palette** — busca e execução a partir do catálogo canônico em
  [`docs/generated/COMMANDS.md`](docs/generated/COMMANDS.md).

Exportação e importação passam pelo pipeline de entrega: OBJ, glTF/GLB e
pacotes `.pkg`. O arquivo nativo `.petunia` é um contêiner ZIP versionado com
UUIDs persistentes.

## Frontend e renderização

- **UI:** shell declarativo em Slint 1.18 (`crates/ui-slint/`), compondo com
  FemtoVG/OpenGL.
- **Viewport:** renderer OpenGL 3.3 Core (glow) em `crates/render/`, no mesmo
  contexto do Slint, desenhando num FBO próprio (MSAA + resolve) importado sem
  readback. Fallback de software quando não há OpenGL 3.3. Decisão e passes em
  [`website/docs/04-renderer/architecture-decision.md`](website/docs/04-renderer/architecture-decision.md).
- **Contratos visuais toolkit-neutros:** viewport-first, Parts/Context/Asset
  Library, workspaces `MODEL / PAINT / UV`, tokens semânticos,
  keyboard/focus/accessibility. Detalhes em [`docs/ui/README.md`](docs/ui/README.md).

## Layout

```
petunia3d/
├── src/main.rs            # binário fino (shell Slint)
├── crates/
│   ├── core/              # estado, câmera, comandos, undo/redo, módulos
│   ├── mesh/              # malha, primitivas, operações, UV e topologia
│   ├── commands/          # pilha de undo/redo
│   ├── config/            # i18n, keymaps, temas e ferramentas
│   ├── project/           # Asset(UUID)/Project, `.petunia`, pipeline OBJ/glTF/GLB/pkg
│   ├── application/       # sessão, ações e serviços da aplicação
│   ├── render/            # boundary RenderFrameInput + renderer OpenGL 3.3
│   ├── module-model/      # ferramentas de modelagem
│   ├── module-paint/      # pintura 2D/3D, camadas e efeitos
│   ├── module-uv/         # projeções, ilhas e transformações UV
│   ├── module-assets/     # biblioteca de assets
│   ├── ui-slint/          # frontend em Slint (shell, bridge, viewport GL)
│   ├── cli/               # CLI headless para automação e pipelines
│   ├── ffi/               # camada C-ABI e `include/petunia.h`
│   ├── mcp/               # fronteira MCP para agentes
│   ├── plugins/           # sistema de plugins Lua
│   └── xtask/             # automação e prevenção de drift documental
├── assets/                # locales, keymaps, temas e ícones
└── docs/                  # manuais, referência e Livro Vivo (SSOT)
```

## Documentação e fonte única da verdade

- Mapa de navegação do projeto: [`docs/NAVIGATION.md`](docs/NAVIGATION.md).
- **Livro Vivo (SSOT):** [`docs/bible/index.md`](docs/bible/index.md), atualmente
  com catálogo até `P3D-174`.
- Interface e contratos visuais: [`docs/ui/README.md`](docs/ui/README.md).
- Auditoria atual da interface Slint:
  [`docs/ui/slint-modern-audit.md`](docs/ui/slint-modern-audit.md).

## Qualidade e status

Os portões normativos — formato, `check`, testes relevantes, Clippy,
`arch-check`, `docs-check`, `bible-check` e `ui-lint` — estão
definidos em [`AGENTS.md`](AGENTS.md). Para mudanças na UI Slint, consulte
também a matriz de gaps em `docs/ui/slint-modern-audit.md` antes de considerar
uma funcionalidade concluída.

## Contribuindo

Leia [`CONTRIBUTING.md`](CONTRIBUTING.md) e o guia completo em
[`docs/developers/contributing.md`](docs/developers/contributing.md).
Resumo: TDD, `clippy`/`fmt` limpos, CHANGELOG + docs no mesmo PR,
i18n en/pt-BR em paridade.

## Licença

GNU GPL v3 ou posterior (GPL-3.0-or-later) — veja [`LICENSE`](LICENSE). Se o projeto te ajuda, considere apoiar em
[ko-fi.com/raillen](https://ko-fi.com/raillen).
