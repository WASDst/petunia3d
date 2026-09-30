# Petunia3D — modelador low-poly shape-first (Rust + Slint + WGPU)

Modelador 3D nativo focado em criação rápida de assets low-poly para games:
desenhe a silhueta sobre a referência, gere a malha, ajuste/transforme,
prepare UV, pinte e exporte — sem exigir domínio prévio de um DCC generalista.

A interface padrão é o shell Slint. O domínio geométrico permanece desacoplado
da UI e é acionado por comandos transacionais com Undo/Redo. A UI egui existe
apenas como legado de transição.

## Rodar

```bash
cargo run --release
cargo run --release -- --legacy-egui
PETUNIA_LEGACY_EGUI=1 cargo run --release
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

- **Produção:** shell declarativo em Slint 1.18 (`crates/ui-slint/`), com
  viewport WGPU compartilhado e fallback de software quando não há GPU
  compatível.
- **Legado:** UI egui (`crates/ui/`) e host em `crates/app/`, acessíveis apenas
  por `--legacy-egui` ou `PETUNIA_LEGACY_EGUI=1`; não recebem novas
  funcionalidades de produto.
- **Contratos visuais toolkit-neutros:** viewport-first, Parts/Context/Asset
  Library, workspaces `MODEL / PAINT / UV`, tokens semânticos,
  keyboard/focus/accessibility. Detalhes em [`docs/ui/README.md`](docs/ui/README.md).

## Layout

```
petunia3d/
├── src/main.rs            # binário fino (Slint por padrão, egui legado opcional)
├── crates/
│   ├── core/              # estado, câmera, comandos, undo/redo, módulos
│   ├── mesh/              # malha, primitivas, operações, UV e topologia
│   ├── commands/          # pilha de undo/redo
│   ├── config/            # i18n, keymaps, temas e ferramentas
│   ├── project/           # Asset(UUID)/Project, `.petunia`, pipeline OBJ/glTF/GLB/pkg
│   ├── render/            # tipos e matemática compartilhada de cena
│   ├── render-gl/         # backend OpenGL do host legado
│   ├── render-wgpu/       # backend WGPU compartilhado
│   ├── module-model/      # ferramentas de modelagem
│   ├── module-paint/      # pintura 2D/3D, camadas e efeitos
│   ├── module-uv/         # projeções, ilhas e transformações UV
│   ├── module-assets/     # biblioteca de assets
│   ├── ui-slint/          # frontend de produção em Slint
│   ├── ui/                # frontend egui legado
│   ├── app/               # host legado, backends e loop render-on-demand
│   ├── cli/               # CLI headless para automação e pipelines
│   ├── ffi/               # camada C-ABI e `include/petunia.h`
│   ├── mcp/               # fronteira MCP para agentes
│   ├── plugins/           # sistema de plugins Lua
│   └── xtask/             # automação e prevenção de drift documental
├── assets/                # locales, keymaps, temas e ícones
└── docs/                  # manuais, referência e Livro Vivo (SSOT)
```

## Documentação e fonte única da verdade

- Roteador do projeto: [`docs/PRUMO.md`](docs/PRUMO.md).
- **Livro Vivo (SSOT):** [`docs/bible/index.md`](docs/bible/index.md), atualmente
  com catálogo até `P3D-174`.
- Interface e contratos visuais: [`docs/ui/README.md`](docs/ui/README.md).
- Auditoria atual da interface Slint:
  [`docs/ui/slint-modern-audit.md`](docs/ui/slint-modern-audit.md).

## Qualidade e status

Os portões normativos — formato, `check`, testes relevantes, Clippy,
`arch-check`, `docs-check`, `bible-check` e `ui-guard --strict` — estão
definidos em [`AGENTS.md`](AGENTS.md). Para mudanças na UI Slint, consulte
também a matriz de gaps em `docs/ui/slint-modern-audit.md` antes de considerar
uma funcionalidade concluída.

## Contribuindo

Leia [`CONTRIBUTING.md`](CONTRIBUTING.md) e o guia completo em
[`docs/developers/contributing.md`](docs/developers/contributing.md).
Resumo: TDD, `clippy`/`fmt` limpos, CHANGELOG + docs no mesmo PR,
i18n en/pt-BR em paridade.

## Licença

MIT — veja [`LICENSE`](LICENSE). Se o projeto te ajuda, considere apoiar em
[ko-fi.com/raillen](https://ko-fi.com/raillen).
