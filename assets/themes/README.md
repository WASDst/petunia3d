# Temas da Interface (`assets/themes/`)

Este diretório contém **apenas os temas oficiais do Petunia3D V1**, conforme o
capítulo 36 do Livro Vivo (*UI Baseline Final V1, Temas e Plugin Panels*).

## Temas oficiais de V1

| ID | Papel | Pasta |
| --- | --- | --- |
| `petunia-dark` | Tema completo oficial e **default** da V1 | [`petunia-dark/`](petunia-dark/) |
| `petunia-light` | Tema claro oficial (revisão 2026-09-30): neutros quentes, sem branco puro | [`petunia-light/`](petunia-light/) |
| `petunia-high-contrast` | Variação oficial de **acessibilidade** | [`petunia-high-contrast/`](petunia-high-contrast/) |

Cada pack é declarativo e contém:

- `manifest.toml` — metadados (`id`, `name`, `version`, `author`, `description`);
- `theme.toml` — tabelas `[colors]` (22 `ThemeToken`), `[shell]` (tokens de shell Slint) e `[font]`.

Os mesmos temas existem como fallback embutido em
`petunia_config::theme::ThemeRegistry`, para que o editor nunca dependa de I/O
para abrir com a aparência correta. **Os valores dos packs em disco e do
fallback embutido devem permanecer idênticos** — essa é a checagem de contrato
do tema oficial.

## Temas não oficiais

Temas adicionais **não** são embutidos no produto. Eles são declarações externas
carregadas do diretório de temas do usuário (P3D-085):

```
<diretório-de-usuário>/themes/<id>/{manifest.toml,theme.toml}
```

Packs prontos para copiar/estudar vivem em [`../theme-examples/`](../theme-examples/).
Um tema parcial é permitido: tokens ausentes caem no default.

## O que um tema **não** pode fazer

- reduzir silenciosamente contraste ou legibilidade de estados semânticos;
- alterar espaçamento métrico estrutural, layout ou densidade do shell;
- executar código — temas são puramente declarativos, sem plugin;
- alterar o documento `.petunia`.

Trocar de tema nunca modifica o projeto aberto.
