---
title: Tokens de Temas Visuais
description: Catálogo canônico de tokens visuais ThemeToken e paletas do Design System (P3D-119)
---

<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->

# Catálogo Canônico de Tokens de Temas (`ThemeToken`)

> **Single Source of Truth (P3D-085, P3D-119)**
> O Petunia Design System define tokens semânticos universais para eliminar cores hardcoded e garantir contraste, acessibilidade e flexibilidade estética.

## Matriz Comparativa de Cores por Tema

> Temas oficiais de V1 (capítulo 36): **Petunia Dark** (completo, default) e **Petunia High Contrast** (variação oficial de acessibilidade). Temas adicionais são packs declarativos do usuário (`themes/<id>/` ou `.petunia-theme`).

| Token Semântico | Função no Design | Petunia Dark | Petunia High Contrast |
| :--- | :--- | :---: | :---: |
| `ThemeToken::BgCanvas` | Fundo geral da área de visualização 3D (Canvas) | `#121212` | `#DDDDDD` | `#000000` |
| `ThemeToken::BgHeader` | Barra de menu principal superior e cabeçalho da aplicação | `#191919` | `#E7E7E7` | `#0A0A0A` |
| `ThemeToken::BgPanel` | Fundo das barras laterais (Toolbar, Outliner, Properties) | `#202020` | `#EEEEEE` | `#121212` |
| `ThemeToken::BgPanelHeader` | Cabeçalho e divisores de seções dos painéis laterais | `#252525` | `#E4E4E4` | `#1A1A1A` |
| `ThemeToken::BgSurface` | Superfície de widgets, caixas de entrada e botões em repouso | `#292929` | `#F5F5F5` | `#1F1F1F` |
| `ThemeToken::BgSurfaceHover` | Superfície de widgets com realce de cursor (hover) | `#353535` | `#E3E3E3` | `#2E2E2E` |
| `ThemeToken::BgSurfaceActive` | Superfície de widgets em estado ativo/pressionado | `#404040` | `#D6D6D6` | `#3D3D3D` |
| `ThemeToken::TextPrimary` | Texto de máxima ênfase (títulos, etiquetas principais) | `#EEEEEE` | `#252525` | `#FFFFFF` |
| `ThemeToken::TextSecondary` | Texto de média ênfase (descrições, valores numéricos) | `#B5B5B5` | `#505050` | `#E6E6E6` |
| `ThemeToken::TextMuted` | Texto atenuado e atalhos secundários | `#9E9E9E` | `#5F5F5F` | `#B8B8B8` |
| `ThemeToken::TextActive` | Texto sobre fundo de destaque (seleção ativa) | `#121212` | `#FFFFFF` | `#000000` |
| `ThemeToken::AccentBlue` | Cor de destaque principal (seleção de objetos e foco) | `#B58CFF` | `#7C4DD6` | `#00B0FF` |
| `ThemeToken::AccentOrange` | Cor de destaque secundária (transformações e alertas) | `#E96A00` | `#D45A00` | `#FFB000` |
| `ThemeToken::AccentHover` | Realce sobre botões ou elementos de destaque | `#C9A8FF` | `#6A3CC4` | `#4DD0FF` |
| `ThemeToken::AccentBorder` | Bordas de destaque e anéis de foco ativo | `#9B6DF0` | `#5B2FB0` | `#FFFFFF` |
| `ThemeToken::BorderSubtle` | Divisores sutis entre seções e painéis | `#343434` | `#D0D0D0` | `#6B6B6B` |
| `ThemeToken::BorderStrong` | Bordas pronunciadas de caixas de diálogo e popups | `#505050` | `#B0B0B0` | `#A0A0A0` |
| `ThemeToken::BorderFocus` | Anel de foco acessível de teclado e widgets | `#D1B8FF` | `#5B2FB0` | `#FFD400` |
| `ThemeToken::StatusInfo` | Mensagens informativas e telemetria | `#6CB6FF` | `#1F6FB2` | `#57C7FF` |
| `ThemeToken::StatusWarning` | Alertas de geometria não conforme ou limites | `#E5BD67` | `#8A5F00` | `#FFD400` |
| `ThemeToken::StatusError` | Erros críticos de I/O, formato corrompido ou colisão | `#EF6B73` | `#C93B45` | `#FF6B6B` |
| `ThemeToken::StatusSuccess` | Confirmação de salvamento, exportação e snapshots | `#73D59B` | `#2F8A55` | `#6EE7A8` |

