# Remediação G7 — endurecimento de domínio, viewport e UI

> **Data:** 2026-09-29
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** itens das seções 1, 2.1, 2.3 e 2.4 de
> `implementation-correction-matrix-2026-09-29.md`

## 1. Autoridade

Contratos aplicados: cap. 05 (Flat/Smooth por objeto, sem "Rendered" prometido),
cap. 13 (vocabulário **Round Edge**), cap. 19 (revisões, caches, orçamento de
histórico), cap. 34 (ownership transacional) e cap. 36 (acessibilidade e
teclado). Nenhuma decisão de produto nova foi tomada em silêncio; onde o código
e o caderno divergem, a UI passou a dizer somente o que o código faz.

## 2. Implementation-vs-Spec Gap Matrix do G7

| ID | Estado anterior | Delta implementado | Estado depois |
| :-- | :-: | :-- | :-: |
| UI-B1 | `BROKEN` | Inspector "rail-first": default recolhido, a UI lê `open`; pílula e pin mantêm `open`/`pin_open` juntos; teste de default atualizado | `COMPLIANT` |
| D-01 | `BROKEN` | `Canvas.pixels` em base64 no JSON (lê o array legado; postcard inalterado); teto único de 256 MiB para salvar e abrir; regressão save→load com textura grande | `COMPLIANT` |
| D-03 | perf | `UndoStack::push_sized` recebe o snapshot por valor; dispatcher e `ProjectState::checkpoint` não clonam duas vezes | `COMPLIANT` |
| D-04 | `BROKEN` | dispatch que falha restaura `primitive_session` e `last_primitive` | `COMPLIANT` |
| D-05 | `BROKEN` | `CommandError::NoChange`: UV stitch/relax e Dissolve sem efeito não criam histórico nem avançam revisões | `COMPLIANT` para os três comandos |
| D-06 | `PARTIALLY_COMPLIANT` | `VecDeque`; redo obedece o orçamento em bytes | `COMPLIANT` |
| D-07 | armadilha | `checkpoint`/`undo`/`redo` sem tamanho só existem em testes | `COMPLIANT` |
| D-10 | perf | `ProjectState::new/reset` e carga de arquivo avançam as revisões, então o hash de conteúdo por frame do fingerprint não roda em uso normal | `COMPLIANT` |
| D-09 / VP-04 | perf | `Asset::evaluated_mesh_ref` (empréstimo sem cópia e sem hash quando não há modifier) nos caminhos de render, seleção, picking e viewport soft; `evaluated_mesh` também sai cedo | `PARTIALLY_COMPLIANT`: com modifiers ainda recalcula (cache por revisão de asset pendente) |
| D-11 | perf | amostragem por comprimento de arco com busca binária (O(log n)) | `COMPLIANT` |
| D-12 | `BROKEN` | Add/Move/SetHandles validam o Profile numa cópia; Attach é recusado em spline de Profile | `COMPLIANT` |
| D-13 | smell | `UpdateSplineCmd`/`UpdateProfileCmd` avançam a revisão; hacks removidos da UI | `COMPLIANT` |
| D-18 | `BROKEN` | Bézier com handles nulos usa os vértices (cantos preservados). A alegação "Sweep ignora workplane" foi **retirada**: o perfil é local ao plano por desenho | `COMPLIANT` para o caso reportado |
| D-22 | morto | removidos `UvModule::stitch/relax` (duplicavam os comandos) | `PARTIALLY_COMPLIANT` |
| VP-01 | `FALSE` | descrições de Material/Rendered dizem o que o shader faz (base color, textura, emissão, uma luz direcional; sem sombras nem ray tracing) | `COMPLIANT` para a copy; reconciliação de enum com o cap. 05 continua pendente |
| VP-02 | `STUB` | `Project::smooth_shaded_assets` + `SetShadeSmoothCmd` (`model.shade_smooth`/`model.shade_flat`), Undo/Redo, persistência, renderer honra por objeto | `COMPLIANT` |
| PA-P5 | perf | referências enviam à GPU por `ReferenceImage::revision`, sem FNV por frame | `COMPLIANT` |
| AX-01 | `FALSE` | Settings deixou de afirmar WCAG 2.2 AA, AccessKit, "zero hardcode", plugins e MCP ativos | `COMPLIANT` |
| AX-02 | `FALSE` | a escala 100–200 % agora despacha `ScaleFactorChanged` sobre a escala do sistema | `PARTIALLY_COMPLIANT`: não verificado visualmente em janela real |
| AX-04 | `BROKEN` | pílulas e botão de pin focáveis por Tab, Enter/Espaço, ação padrão acessível, anel de foco, pin com 28 px | `PARTIALLY_COMPLIANT` |
| AX-05 | `MISSING` | abas do Settings com papel `tab`, estado marcado e navegação por setas ↑/↓ | `PARTIALLY_COMPLIANT`: sem armadilha de foco nem papel de diálogo |
| AX-06 | `MISSING` | F6 alterna o foco entre viewport e régua do Inspector | `PARTIALLY_COMPLIANT`: duas regiões apenas |
| AX-07 | `PARTIALLY_COMPLIANT` | token `selection` usa `AccentOrange` em vez de `AccentBlue` | `PARTIALLY_COMPLIANT` |
| IN-01 | `BROKEN` | normalização de teclas do teclado numérico em `numpad_key` (tabela única, testada) | `PARTIALLY_COMPLIANT`: a cadeia de teclas físicas ainda precede o keymap |
| UI-06 | `BROKEN` | "Bevel" → "Round Edge" em locales (pt-BR/en), toolbar e mensagens | `COMPLIANT` |
| UI-M1 | `FALSE` | abas Plugins e MCP sem "Ativo" fixo | `COMPLIANT` |

## 3. Itens não fechados neste gate (com motivo)

| ID | Motivo |
| :-- | :-- |
| D-02 | ~120 chamadas diretas de `checkpoint`/`emit_*` na bridge: migração por vertical slice, não em lote |
| D-08 | revisão por asset e `changes()` por comando exigem medição e desenho do changeset do renderer |
| D-14 / D-20 | dependem do G6.1 (Profile só por Command; remover `ProfileState`) |
| D-15 / D-16 | Attachment não tem UI; BVH e cache de fingerprint entram junto com H3 |
| D-17 / D-19 / D-21 | decisão de design: degradação por ponto, LRU e registro de comandos paramétricos no catálogo |
| VP-03 | buffers por asset e frustum culling: mudança grande do renderer, precisa benchmark antes/depois |
| VP-05…08 | vertex paint, uploads por retângulo, viewport soft e painel de Viewport/Snap: pertencem à conversa de UX |
| AX-03 / AX-08 | 249 `TouchArea` × 7 `FocusScope` e fontes de 9–10 px: exigem componentes base focáveis |
| IN-01 (resto) / IN-02 | autoridade única de keymap e editor de atalhos |
| UI-05 | migração de ~210 textos literais para `TextId` |
| UV-01…03 | decisão: UV será unificado ao PAINT (próxima conversa) |
| AR-01 / AR-02 / MO-LEG | fatiamento dos monólitos, fila de intents e aposentadoria do egui |

## 4. Gates de encerramento

Resultados reais, sem extrapolar:

| Gate | Resultado |
| :-- | :-- |
| `cargo fmt --all -- --check` | passou (após `cargo fmt --all`) |
| `cargo check --workspace --all-targets` | passou |
| `cargo clippy --workspace --all-targets -- -D warnings` | passou |
| Testes `commands`, `config`, `mesh`, `project`, `core` | passaram (incluindo 5 testes novos em `command_tests.rs`, 4 em `format.rs`, 2 em `commands`) |
| Testes `petunia_ui_slint --lib` | 344 passaram, antes das últimas mudanças de foco/F6/numpad em `.slint`/`lib.rs` |
| Testes após as últimas mudanças de UI, `render-wgpu` e `docs-check`/`bible-check`/`ui-guard --strict` | **não executados**: a máquina ficou sobrecarregada por builds concorrentes de outras sessões (load médio > 100) e a recompilação completa não terminou |
| Verificação visual (UI scale, F6, foco de pílulas) | **não executada**: exige abrir a janela |

A afirmação anterior "839 testes do workspace" continua sendo um subconjunto de
seis crates; o total de `#[test]` no repositório é de aproximadamente 1.300.
