# Atalhos, painéis e controles do shell — Implementation-vs-Spec Gap Matrix (2026-09-30)

Escopo: P3D-090/091 (keybindings e conflitos), ADR 007 (tabela de decisões:
`Space`), capítulo 36 (painéis flutuantes não modais) e defeitos relatados:
modais que não aceitam tamanho, arrastar painel que "trava", sliders que não
seguem o modelo e nomes de perfil cortados.

## Auditoria antes da mudança

| Aspecto | Encontrado | Classificação |
| --- | --- | --- |
| Carregar perfis | `load_profile` lia `assets/keymaps/<id>.toml` **relativo ao diretório de trabalho**: fora da raiz do repositório o app caía nos defaults internos (e o `petunia-default.toml` — com `select_object = "0"` — contradizia a decisão de V1 e os testes) | BROKEN |
| Perfis do usuário | Prometidos pelo README ("têm precedência"), inexistentes | MISSING |
| Teclas | Sem `/ . , - = ` ; ' \`, setas, PageUp/PageDown, Insert: não dava para ligar atalho nelas | PARTIALLY_COMPLIANT |
| Atalhos do PAINT | Só `paint.paint` estava ligado; os demais eram rótulos (`fmt("paint.eraser", "E")`) sem ação | BROKEN |
| `Space` | Ocioso abria o micro-inspector (decisão do ADR 007: "ferramenta anterior") | MISSING |
| Trocar de workspace pelo teclado | Só Ctrl+PgUp/PgDn (cíclico); nenhuma tecla direta | MISSING |
| Editor de perfis (Configurações → Teclado) | 4 botões fixos de 110 px (nomes cortados) e sem edição de atalhos | BROKEN |
| Arrastar painel flutuante | `PanelDragHandle` somava o deslocamento à posição **do clique** enquanto o próprio painel (e a alça) se movia: o resultado oscilava a cada quadro ("trava") | BROKEN |
| Tamanho dos painéis | Fixo (`min(660px, …)`): valores ficavam cortados | MISSING |
| `PetuniaSlider` | Escrevia em `value`: com `value: expr` (5 usos) o binding de uma via era desfeito no primeiro arrasto e o slider deixava de seguir o modelo | BROKEN |

## Requisitos e entrega

| # | Requisito | Evidência e delta | Estado após |
| --- | --- | --- | --- |
| 1 | Perfis independentes do diretório de trabalho | `EMBEDDED_PROFILES` (`include_str!`); `Keybinds::load_profile_in(id, dir)`; `petunia-default.toml` corrigido (`select_object = "4"`) | COMPLIANT |
| 2 | Perfis do usuário (P3D-090) | `<config>/petunia3d/keymaps/user-*.toml`: `all_profiles_in`, `save_user_profile_in` (escrita atômica), `delete_user_profile_in`, `sanitize_profile_id`; editar um perfil embutido cria a cópia; embutidos não são apagáveis. Testes em `keymap_edit::tests` | COMPLIANT |
| 3 | Conflitos (P3D-091) | `conflicts_for(action, binding)`: mesmo contexto ou global/view/window; sinaliza na linha e no status, não bloqueia | COMPLIANT |
| 4 | Captura de combinação | Clique no atalho → próxima combinação; `Esc` cancela sem fechar Configurações; modificador sozinho segue esperando | COMPLIANT |
| 5 | `Space` = ferramenta anterior (ADR 007) | `global.previous_tool` (`Space`) com a ferramenta do workspace atual; sessões continuam confirmando com `Space`; `global.micro_inspector` = `Shift+Space`. Teste `space_swaps_to_the_previous_tool_and_workspace_keys_switch_workspaces` | COMPLIANT |
| 6 | Teclas de workspace | `window.workspace_draw/poly/paint/uv/animate` = `Ctrl+1..5`; entrar no PAINT ativa o pincel, sair dele tira ferramentas de pintura | COMPLIANT |
| 7 | Atalhos do PAINT ligados | `paint.select` (V), `isolate` (/), `airbrush` (Shift+B), `eraser` (E), `color_picker` (I), `fill` (G), `gradient` (Shift+G), `gradient_radial` (Alt+G), `line` (L), `rectangle` (R), `ellipse` (C) | COMPLIANT |
| 8 | Teclas ampliadas | `KeyCode` com pontuação, setas, PageUp/PageDown e Insert (config + `key_code_from_slint`) | COMPLIANT |
| 9 | Painéis: arrastar sem travar | `PanelDragHandle` usa a posição **atual** + deslocamento relativo (ponto pressionado fica sob o cursor) | COMPLIANT (sem captura nativa) |
| 10 | Painéis: redimensionar | `PanelResizeGrip` (canto inferior direito, duplo clique restaura) em Configurações, Reference Manager e Canvas; limites de 340×260 até a janela | COMPLIANT (sem captura nativa) |
| 11 | Sliders seguem o modelo | `PetuniaSlider.controlled`: não escreve em `value`, mostra o valor arrastado enquanto o ponteiro está preso e emite `changed`/`released` (também em `cancel`); ligado nos 5 usos de binding de uma via | COMPLIANT (sem captura nativa) |
| 12 | Nomes de perfil legíveis | Lista vertical dinâmica (`KeymapProfileEntry`), badge "custom", novo/apagar perfil | COMPLIANT (sem captura nativa) |

## Pendências

- Perfis embutidos ainda não definem `paint.*`/`window.*`; herdam os defaults Petunia.
- A lista de ações do editor mostra o id derivado (`model · push pull`), não um rótulo traduzido por ação.
- Tamanho dos painéis não é persistido entre sessões.
- Atalhos numéricos do teclado (`Numpad`) ainda são tratados à parte (`numpad_key`) e não são rebindáveis.
