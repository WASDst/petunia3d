# Petunia3D — plano de paridade de interface e implementação funcional

Data da reconciliação: 23/09/2026. Base: `f94b888` em `main` de `wasd-lat/petunia3d`.
Fontes: Livro Vivo (`docs/bible/`), dois anexos de especificação da interface,
três capturas de defeitos, código e comportamento do executável. Referência de
interação: Livro Vivo, [constituição 11](../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md) e [capítulo 46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md) (ADR 007, 2026-09-29). O [manual do Blender](https://docs.blender.org/manual/en/latest/modeling/meshes/selecting/introduction.html) deixou de ser referência de interação e fica apenas como comparação histórica.

**Regra de estado:** `Concluído` exige comportamento integrado, teste com
assertivas de resultado, regressão visual real, cancelamento/Undo aplicáveis e
reprodução local. `Em teste` indica código corrigido mas ainda sem aceite. O
registro é atualizado ao fim de cada implementação, junto com a tabela entregue
ao usuário. Não se aceita somente presença de botão, callback ou teste de enum.

Tamanho: P = até 2 dias, M = 3–5 dias, G = 1–2 semanas, XG = várias semanas de
trabalho focado. Estimativas de esforço relativo, não compromisso de prazo.

## Execução atual

Os checkpoints de 22/09/2026 foram publicados como **não testados**. Em 23/09,
após esta rodada de implementação, passaram 22 testes Paint, 4 UV e 169 Slint
com Rust 1.98.1. Um smoke visual real do MODEL/WGPU em 1092×1012 e 1400×900
detectou e confirmou a correção da pill que cobria a viewport e do Inspector
que reservava espaço no layout compacto. Um smoke de PAINT/UV revelou que o
breakpoint de 1100 px ainda oculta os editores 2D na largura inicial. Baixar
o limite sem reestruturar o Inspector força a janela a ~1800 px, então esse
experimento foi revertido; drawer/split compacto e backend software seguem
pendentes. Isso não fecha I04, F04 ou F05.
`cargo check --workspace`, Clippy estrito das crates envolvidas, `arch-check` e
`ui-guard --strict` passaram. `docs-check` e `bible-check` param no lock do site
congelado porque `docs/.vitepress/.env.local` já não existe neste ambiente; o
arquivo não foi recriado. A formatação global anterior ao delta também falhava
em arquivos fora da superfície alterada; foi normalizada em commit mecânico
posterior ao checkpoint funcional.
Matriz detalhada: [viewport-gap-matrix.md](viewport-gap-matrix.md).

## 1. Interface e viewport: paridade antes de novas funções

| ID | Prioridade | Tam. | O que é / critério de aceite | Dep. | Estado e evidência |
|---|---|---:|---|---|---|
| I00 | P0 | M | Auditoria de estado, input, picking, seleção, render, HUD, Undo e screenshots nos dois backends; matriz COMPLIANT/PARTIAL/BROKEN/STUB/MISSING por fluxo. | — | Em andamento: falhas confirmadas no redraw ao mudar modo, hit test aproximado, callbacks cruzados e reconstrução da cena a cada hover. |
| I01 | P0 | M | Estado único de domínio/ferramenta/operação, conversão de seleção coerente e redraw imediato; quatro domínios sem face laranja residual. | I00 | Implementado, não testado: transições de domínio limpam flags incompatíveis e hover; callbacks redesenham o frame. |
| I02 | P0 | G | Mesmo picking para hover e clique; raycast exato do objeto, alvos em pixels para vértice/aresta, oclusão, X-Ray, múltiplos objetos, Shift e área vazia. | I01 | Implementado, não testado: picker compartilhado com o core, oclusão entre objetos visíveis, seleção múltipla/Shift/área e ações contextuais. |
| I03 | P0 | M | Feedback Object/Point/Edge/Face com ativa/hover/ocluída e tamanho fixo de tela; overlays sem diamantes nem volumes indevidos. | I02 | Implementado, não testado: paths em coordenadas preservadas; guias e seleção com profundidade em WGPU/software; tripé confinado em 108 px. |
| I04 | P0 | G | Wire, Solid, Material e Rendered produzem resultados diferentes com material/textura/luz reais; fallback software honesto, paridade e testes de pixels. | I01 | Implementado, não testado: software diferencia Solid/Material/Rendered, amostra textura com UV em perspectiva e usa luz da cena; paridade de pixels pendente. |
| I05 | P0 | M | X-Ray controla transparência, teste de profundidade e alcance de seleção em todos os domínios; opção independente de shading. | I02–I04 | Implementado, não testado: pipelines de seleção WGPU ignoram profundidade apenas com X-Ray; software aplica opacidade e seleção através; matriz completa pendente. |
| I06 | P0 | M | Viewport mantém proporção, responde ao resize e DPI, entrada na coordenada correta, barra e Inspector legíveis de 480 px até 1920 px. | I01 | Parcial: inicialização usa tamanho real do layout; overlays acompanham câmera e resize; pill de pré-seleção não encobre mais a viewport e Inspector oculto não reserva espaço. Smoke WGPU em 1092 e 1400 px; Paint/UV compactos, DPI/extremos e software pendentes. |
| I07 | P1 | M | Grade adaptativa, eixos, cursor, bounds e referências com profundidade/opacidade/hierarquia que não compete com a malha. | I04 | Parcial: grade adaptativa nova; revisão visual e configurações faltam. |
| I08 | P1 | M | Orbit/pan/zoom/frame/ortográfica em torno do pivô, captura do mouse, foco e limiar de drag; clique não inicia transformação. | I02 | Implementado, não testado: limiar de drag, cancelamento, navegação com pivô selecionado e viewport real; smoke de mouse pendente. |
| I09 | P1 | G | Gizmos com picking de handle, tamanho fixo, eixo/plano, hover/drag/lock e feedback legível em zoom e ângulos extremos. | I02,I08 | Parcial, não testado: hastes, cubos de escala e anéis de rotação; handles de plano/livre e orientação Local pendentes no estado retomado. |
| I10 | P1 | G | Operações modais Move/Rotate/Scale/Extrude/Inset/Bevel/Loop Cut/Knife: preview derivado do original, número/eixo/snap, Enter, Esc e um checkpoint. | I01,I02,I09 | Parcial, não testado: G/R/S, entrada numérica, eixo/plano, Shift/Ctrl, seleção múltipla e pivôs por ilha/objeto; revisão de todas as ferramentas pendente. |
| I11 | P1 | M | HUD mostra valores reais, alvo, eixo e dica contextual; barra inferior acompanha o estado e não mente. | I10 | Parcial: HUD e status contextual novos, casos extremos sem smoke. |
| I12 | P1 | M | Painel flutuante por ferramenta: campos corretos, Apply/Cancel, collapse/pin e teclado; não mostrar controles de outra ferramenta. | I10 | Parcial, não testado: NumericField com draft, validação, Enter/Esc, cancelamento de scrub e limiar de drag; pin pendente. |
| I13 | P1 | G | Design system Slint: shell viewport-first, painéis redimensionáveis, tipografia, ícones com legenda, touch/focus e acesso via teclado. | I06 | Parcial: shell e tokens existem; paridade visual/acessibilidade pendente. |
| I14 | P1 | M | Regressão 4 domínios × 4 shadings × X-Ray, cenários das capturas, operações, Undo/Redo, resize e GPU/software. | I01–I13 | Pendente: testes atuais não certificam todas as combinações. |
| I15 | P1 | M | Telemetria, render sob demanda e medição: hover sem triangulação/buffer de malha; UI responsiva em cena simples e densa. | I04,I14 | Implementado, não testado: hover/câmera/domínio atualizam buffers de seleção; navegação sincroniza apenas os overlays; benchmark pendente. |

## 2. Funcionalidades do editor após a paridade de interface

| ID | Prioridade | Tam. | O que é / critério de aceite | Dep. | Estado e evidência |
|---|---|---:|---|---|---|
| F01 | P1 | G | Referência → perfil 2D → forma 3D com preview/cancel/Undo, importação de imagem e transformação editável na viewport Slint. | I08–I14 | Pendente de certificação ponta a ponta. |
| F02 | P1 | G | Primitivas, extrude individual/região, inset, round edge, push/pull, slice, knife e loop cut com topologia válida e entradas precisas. | I10 | Parcial: núcleo e vários fluxos UI ligados; critérios de qualidade topológica ainda abertos. |
| F03 | P1 | G | Connect de loops abertos e contagens diferentes usando quads/triângulos, preview e preservação manifold. | I10,F02 | Parcial, não testado: algoritmo de duas cadeias abertas/anéis fechados e contagens diferentes presente; preview Apply/Cancel ausente no estado retomado. |
| F04 | P1 | G | Paint on model: stroke contínuo, layers, fill por escopo, ferramentas Line/Rectangle, material visível no 3D, Undo por traço. | I04,I10 | Parcial: código recente cobre vários fluxos, sem validação visual real integrada. |
| F05 | P1 | G | UV: unwrap/pacote real, editor 2D, seleção face/ilha, transformações, costuras, texel density e sincronismo com 3D. | I02,I10 | Parcial: picking 2D e transformações recentes; workspace completo não certificado. |
| F06 | P1 | M | Abrir/salvar/importar/exportar com diálogos, modificações não salvas, autosave/recovery e erro recuperável; round-trip por fixture. | I13 | Parcial: bridge e serviços existem; validar no executável. |
| F07 | P1 | G | Completar o contrato ZIP/JSON `.petunia`, manifest e recursos; preservar importação de Postcard legado e round-trip sem perda. | F06 | Parcial: ZIP/JSON e loader Postcard já existem na base; manifest e organização dos recursos precisam ser reconciliados. |
| F08 | P2 | G | Biblioteca de assets de fato independente da cena: catálogo, busca, favoritos, filtros, drag/drop e aplicação; Undo e persistência. | I13,F06 | Parcial: Save Active cria asset de cena, não prova biblioteca completa. |
| F09 | P2 | G | Inspector/contexto, materiais e propriedades reativos ao objeto/seleção/ferramenta, operações transacionais e campos editáveis. | I01,I12 | Parcial; revisão de cada comando visível pendente. |
| F10 | P2 | G | Comandos/keymap/i18n/acessibilidade: sem ações vazias, foco via teclado, menus e busca com atalhos remapeáveis. | I13 | Parcial: catálogo e callbacks existem; auditoria fim a fim pendente. |
| F11 | P2 | XG | Representação authoring com handles/topologia estáveis ou ADR que ajuste a promessa do Livro Vivo; serialização, Undo e plugins seguros. | F02,F07 | Divergência arquitetural confirmada; projeto de migração pendente. |
| F12 | P2 | G | Undo por diffs/escopo, perf em malhas/texturas grandes e história transacional compartilhada com paint/UV/modificadores. | F02,F04,F05 | Parcial, não testado: seleção múltipla no histórico, bytes explícitos no Undo/Redo e identificadores monotônicos do estado salvo; diffs e medições pendentes. |
| F13 | P2 | M | Painéis de plugin, temas, overlays LIFO e API pública compatíveis com o shell Slint e baseline do Livro Vivo. | I13,F10 | Parcial: mecanismos presentes; integração UI não certificada. |
| F14 | P1 | G | Gates workspace/Clippy/fmt/arquitetura/docs, smoke Linux/Windows, cenários visuais e teste com artista; registrar FPS e limites. | I14,F01–F13 | Pendente: nenhuma alegação de qualidade Blender/Plasticity antes da evidência. |

## Evidências exigidas para fechar uma linha

1. Um teste ou reprodução que falhava antes da correção e agora passa no caminho do executável.
2. Captura de janela real nos cenários da especificação, incluindo troca de modo e X-Ray.
3. Operação aplicável testada com confirmar, cancelar, Undo e Redo, com geometria idêntica após cancelar.
4. Nenhuma divergência conhecida entre texto/ícone, interação, estado do domínio e frame renderizado.
5. Resultado de `cargo fmt`, `cargo check`, testes relevantes, `cargo clippy` e gates documentais, com limitações honestas.

## Registro de atualizações

- 21/09/2026: reconciliação do commit `f94b888`. Correções I01/I02/I08/I12/I15 iniciadas;
  estados permanecem **Em teste** até os gates e a reprodução visual.

- 22/09/2026: implementação sem execução de testes conforme orientação do usuário.
  Correção estrutural de `Path.fit`, guias X-Ray, fallback de shading, tamanho
  inicial do layout e sincronização de overlays. Triangulação de polígonos
  côncavos compartilhada entre render e picking; estatísticas contam n−2 tris.
  Checkpoints publicados não representam aceite de qualidade.

- 23/09/2026: conexão GitHub da conta `wasd-lat` autorizada. Primeiro checkpoint
  publicado como `522b1f1`, com árvore idêntica à de `53ec324`. Publicação pela API
  autenticada gera novo hash de commit, sem alterar o conteúdo dos arquivos.
- O ambiente retomado não contém os objetos de `4364977`/`956eca0`. O estado
  disponível foi preservado e reconciliado com esta tabela. Não estão presentes
  os handles de plano/livre, o fluxo de escolha do anel por hover, o clipping
  completo do rasterizador, o preview Apply/Cancel de Connect e a revisão final
  de preservação de atributos em Knife/Loop Cut. Esses itens continuam pendentes.
- O usuário solicitou push seguido de merge na `main`, mantendo os testes para
  o fim da implementação. Este checkpoint não certifica paridade ou qualidade final.
- 23/09/2026: integração Paint/UV corrigida na triangulação côncava, cor de
  pincel/conta-gotas e seleção sincronizada; suíte Slint passou de 158/169 para
  169/169 após atualizar testes que pressupunham semântica anterior de seleção,
  Cut e gizmos. Ver matriz para limites de aceite.
- 23/09/2026: captura real do MODEL/WGPU revelou um scrim de pré-seleção que
  ocupava a altura da viewport e 290 px vazios reservados pelo Inspector oculto.
  As duas causas foram corrigidas no layout declarativo e recapturadas em
  tamanhos compacto e amplo. Smoke posterior de Paint/UV expôs que o breakpoint
  de 1100 px escondia os editores 2D na largura inicial. Reduzir o breakpoint
  sem extrair um drawer expandiu a janela para ~1800 px; a tentativa foi
  revertida e segue como lacuna de layout. Isso não certifica Paint/UV.
