# Petunia3D · Icon System

Dois packs first-party de SVG independentes, `petunia-outline` e `petunia-filled`, com o mesmo conjunto de `IconId`. A arte usa uma grade de 24 × 24, canvas transparente, formas ópticas de aproximadamente 20 px, traço principal de 1,8 px, extremidades arredondadas e geometria isométrica recorrente para operações 3D. Os desenhos foram criados para o Petunia3D; não contêm arquivos copiados de Blender, Lucide, Tabler ou outros fornecedores.

## Entrega

- `../petunia-outline/` e `../petunia-filled/`: cada pack tem `manifest.toml`, `icons.toml` e SVGs próprios em `svg/<categoria>/<id>.svg`.
- `catalog.json`: IDs, nomes, categorias e marcação de conceitos de roadmap.
- `command-map.json`: mapeamento explícito dos `CommandId` catalogados e dos comandos invocados no shell Slint para um `IconId`.
- [`gallery.html`](gallery.html): galeria local para alternar variantes, filtrar por categoria e procurar por ID/nome. Abra como arquivo local com um navegador.
- [`preview.png`](preview.png): amostra comparativa das duas variantes para revisão rápida no GitHub.
- `generate.py`: fonte editável da geometria e gerador determinístico das duas variantes e catálogos. Reexecute após qualquer alteração.

```bash
python3 assets/icons/petunia-dual/generate.py
```

Não edite SVGs gerados isoladamente. O gerador verifica cobertura dos IDs da registry egui, dos SVGs referenciados pelo shell Slint (exceto o logotipo), de `assets/tools.toml` e dos comandos do catálogo/Slint. Além disso, analisa o XML e exige geometria distinta para as variantes.

## Matriz de reconciliação

| Contrato | Estado anterior | Resultado desta entrega |
| --- | --- | --- |
| `IconId` da registry e tools configuradas | **PARTIALLY_COMPLIANT**: poucos SVGs mapeados em `assets/icons/petunia/icons.toml`; demais fallback/PNG/glifos | **COMPLIANT no asset**: os dois manifests mapeiam todos os IDs conhecidos no momento da geração |
| Comandos e ferramentas do shell Slint | **PARTIALLY_COMPLIANT**: SVGs especializados no shell, sem variantes | **COMPLIANT no asset**: `command-map.json` resolve comandos de MODEL, PAINT, UV, View e File |
| Shape-first, malha, UV e pintura | **RUDIMENTARY no pack**: arte dispersa, inclusive PNGs históricos | **COMPLIANT no asset**: pictogramas dedicados em famílias coerentes e duas variantes |
| Seletor de pack na UI Slint | **MISSING**: shell usa SVGs estáticos e Lucide; não há resolver dinâmico por `IconId` | **MISSING**: integração runtime é uma tarefa separada de UI; esta entrega fornece os assets e mapeamentos sem mudar o shell |

O estado `planned` no catálogo indica somente uma família de símbolos para roadmap (animação, rig, cabelo, cloth e partículas); a presença de qualquer ícone não afirma que uma funcionalidade esteja implementada. Um conceito pode ter aliases de compatibilidade com IDs distintos, por exemplo `tool_extrude` e `extrude`; ambos compartilham a mesma geometria deliberadamente.

## Uso

Um resolver de `IconId` deve carregar `icons.toml` do pack escolhido e usar o `petunia` canônico como fallback. O caminho relativo em `icons.toml` é resolvido a partir da pasta do próprio pack. O SVG especifica a cor padrão clara para o tema dark atual; seus grupos usam `currentColor` para que o loader possa trocar a cor conforme o tema. Detalhes internos da variante filled usam o recorte escuro `#252735` para legibilidade na superfície dark atual; o tratamento de contraste para temas claros e High Contrast deve acompanhar a integração do resolver na UI. Hitbox, tooltip e accessible name pertencem ao botão da UI, não ao desenho.

Os packs oficiais genéricos (`lucide`, `tabler`, `iconoir`, `phosphor`) não são alterados. Este trabalho segue P3D-086–088 e a UI Baseline V1 sem copiar os assets de outros softwares.
