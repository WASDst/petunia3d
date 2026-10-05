# Sistema de Temas e Pacotes de Ícones — Petunia3D

O **Petunia3D** implementa um ecossistema declarativo, extensível e orientado a tokens para temas visuais e conjuntos de ícones, permitindo personalização visual completa sem recompilação.

---

## 1. Arquitetura do Sistema de Temas

O gerenciamento de temas é implementado em `crates/config/src/theme.rs` e consumido pela camada de apresentação em `crates/ui/src/tokens.rs`.

### 1.1 `ThemeToken` e Cores Semânticas
Em vez de cores hardcoded nos componentes, a interface referencia o enum `ThemeToken`:
- **Canvas & Backgrounds**: `bg_canvas`, `bg_panel`, `bg_header`, `bg_viewport`, `bg_sidebar`.
- **Containers & Surfaces**: `surface_dark`, `surface_input`, `surface_active`, `surface_hover`.
- **Text & Content**: `text_primary`, `text_secondary`, `text_muted`.
- **Borders & Dividers**: `border`, `border_focus`.
- **Accents & States**: `accent`, `accent_hover`, `selection`, `error`, `warning`, `success`.

### 1.1.1 Tokens no shell Slint (produção)

No frontend Slint, `crates/ui-slint/src/theme.rs` copia as cores do tema ativo
para o global `DesignTokens` de `crates/ui-slint/ui/tokens.slint`. Esse arquivo
é o **único** lugar com valores literais:

- cores semânticas do tema (`surface*`, `text-*`, `accent`, `selection`…);
- cores de sobreposição da viewport, independentes do tema (`hud-text`,
  `measure`, `marker-pivot`, `cursor-*`, `uv-*`);
- tipografia `font-caption` 10 · `font-small` 11 · `font-default` 12 ·
  `font-strong` 13 · `font-exceptional` 14 (cap. 36);
- elevação em dois níveis com sombra (`elevation-1-*` painel/HUD,
  `elevation-2-*` popover/flutuante) e `glow-blur` para estado armado;
- o global `ColorPresets`, com as paletas rápidas de pintura e de etiqueta.

`cargo run -p xtask -- ui-lint` falha se outro `.slint` usar cor `#hex`,
`font-size` ou `drop-shadow-blur` literal (strings e comentários são
ignorados). O job `UI Slint` executa essa guarda.

### 1.2 Estrutura de Diretório de Temas
Os temas residem em `assets/themes/<theme_id>/`:

```
assets/themes/
├── petunia-dark/
│   ├── manifest.toml
│   └── theme.toml
├── petunia-light/
│   ├── manifest.toml
│   └── theme.toml
├── petunia-capuccino/
│   ├── manifest.toml
│   └── theme.toml
└── petunia-tokyo-nights/
    ├── manifest.toml
    └── theme.toml
```

#### `manifest.toml`
```toml
id = "petunia-dark"
name = "Petunia Dark (Blender Pro)"
version = "1.0.0"
author = "Petunia3D Team"
license = "MIT"
description = "Tema escuro profissional inspirado na paleta canônica do Blender."
```

#### `theme.toml`
Valores hexadecimais em formato `#RRGGBB` ou `#RRGGBBAA`:
```toml
[colors]
bg_canvas = "#1d1d1d"
bg_panel = "#2d2d2d"
bg_header = "#242424"
bg_viewport = "#393939"
bg_sidebar = "#282828"
surface_dark = "#1e1e1e"
surface_input = "#181818"
surface_active = "#4772b3"
surface_hover = "#3e3e3e"
text_primary = "#e1e1e1"
text_secondary = "#a0a0a0"
text_muted = "#707070"
border = "#3f3f3f"
border_focus = "#4772b3"
accent = "#4772b3"
accent_hover = "#5c8cd6"
selection = "#4772b3"
error = "#e05252"
warning = "#e5a93c"
success = "#52b86a"
```

### 1.3 Resiliência e Fallback
Se qualquer arquivo for corrompido, faltar campos ou não puder ser lido, o `ThemeRegistry` faz fallback automático e seguro para a paleta canônica `Petunia Dark`, garantindo que a aplicação nunca entre em estado de erro ou tela branca.

---

## 2. Sistema de Pacotes de Ícones

O sistema de ícones é implementado em `crates/ui/src/icon_registry.rs` e `crates/ui/src/icons.rs`.

### 2.1 Hierarquia de Renderização (Cascading Fallback)
1. **Asset Rasterizado/SVG do Pacote Ativo**: Textura renderizada em alta resolução com canal alfa e tingimento dinâmico.
2. **Desenho Vetorial do Petunia**: Funções analíticas em `icons.rs` com stroke balanceado (~1.8–1.9px) e paleta cromática canônica inspirada no Blender.
3. **Glifo Unicode / Text Badge**: Representação tipográfica compacta garantida em qualquer plataforma.

### 2.2 Pacotes Suportados
- **Petunia**: Conjunto oficial com fidelidade pixel-perfect ao `Blender.svg`.
- **Phosphor**: Estética técnica e limpa.
- **Tabler**: Formas geométricas precisas para ferramentas 3D.
- **Iconoir**: Desenho minimalista monocromático.
- **Lucide**: Visual contemporâneo com curvas uniformes.

---

## 3. Seleção em Tempo de Execução

O usuário pode alterar o tema e o pacote de ícones instantaneamente pelo modal de Configurações (`⚙ Config` no canto superior direito do cabeçalho), com pré-visualização ao vivo dos tokens e do grid de ferramentas.
