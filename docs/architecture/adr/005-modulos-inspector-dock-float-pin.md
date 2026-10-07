# ADR 005 — Módulos independentes do Inspector: ancorar, flutuar e fixar por seção

Status: aceito em 2026-09-24 por autorização explícita do responsável do produto.
Autoridade: [Livro Vivo, capítulo 36](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
(revisão MODEL de 2026-09-23), [ADR 003](003-model-parts-in-inspector.md) e
[ADR 004](004-inspector-translucido-alca-modifiers.md).
Este ADR registra o motivo; não cria uma segunda baseline.

## Contexto

Após a revisão do Inspector (ADR 004), o produto pediu que **cada módulo do painel
direito seja independente**: abrir/recolher, ancorar ou flutuar e fixar conteúdo,
com persistência própria por módulo. E que os painéis possam ser **flutuantes de
verdade** — arrastáveis dentro da própria tela, sem janela de sistema.

A restrição do caderno é explícita: o shell é viewport-first, sem docking
irrestrito e sem janelas OS. Logo, "flutuante" aqui significa *card sobreposto
dentro do canvas*, com posição persistida, e não uma `Window` do sistema
operacional. Registro essa divergência de terminologia porque "floating panel"
em várias ferramentas é exatamente uma janela OS — o que a V1 proíbe.

## Decisão

1. **Ancoragem por seção.** As seis seções (`Parts`, `Transform`, `Material`,
   `Object`, `Modifiers`, `Quick Actions`) mantêm um layout próprio
   (`SectionLayout`: `docked`, `x`, `y`, `pin_open`, `pinned_asset`). O estado
   canônico é a ordem do Inspector; `dock` é o estado inicial, portanto o
   baseline anterior é o fallback.
2. **Corpo definido uma vez.** Cada corpo virou componente Slint
   (`PartsBody`, `TransformBody`, `MaterialBody`, `ObjectBody`, `ModifiersBody`,
   `QuickActionsBody`) instanciado na coluna ancorada e no card flutuante. Não há
   cópia de markup nem de callback por seção.
3. **Camada flutuante em `Window`.** Um `for` sobre `section-states` cria um
   `FloatSectionCard` por seção não ancorada, posicionado em `(x, y)`. O card
   expõe arraste pelo header, pin aberto, pin de asset e retorno à coluna.
4. **Arraste in-canvas com clamp.** O header acumula o delta do ponteiro e
   emite `section-moved`; o clamp mantém o card dentro do viewport
   (`x ≥ 0`, `y ≥ 40`, folga de 80px nas bordas).
5. **Pin duplo, como pedido.** `pin_open` mantém a seção aberta contra
   "recolher tudo" e contra o recolhimento do painel; `pinned_asset` fixa a
   seção a um UUID de asset. São eixos independentes: um card pode estar
   fixado a um asset e continuar recolhível.
6. **Pin de asset redireciona a operação, não só a leitura.** Material,
   Object e Modifiers resolvem o asset exibido por `section_asset()` (UUID
   fixado com fallback na seleção ativa); mutações de modifier localizam o
   dono por `find_modifier_owner` e aplicam no asset exibido, não no ativo.
7. **Persistência por módulo.** `UserPreferences.section_layouts` guarda os
   seis layouts; o startup restaura via `restore_section_layouts` e cada
   transição grava. UUID inválido, string longa ou asset inexistente caem no
   comportamento seguro (seleção ativa) em vez de quebrar a UI.
8. **Texto por `TextId`.** Dock, arraste, pin aberto, pin de asset e unpin têm
   `TextId` em en/pt-BR; nenhum rótulo de affordance ficou hardcoded.
9. **Ordem e teclado preservados.** Reancorar devolve a seção à ordem canônica;
   o toggle de recolher-tudo é reverso de mão única (`close_all = all open`) e
   ignora seções com pin aberto.

## Divergências das referências (registradas, não aplicadas em silêncio)

- O Blender usa *panels flutuantes* de verdade: uma `Window` flutuante que
  pairs out of editor, perde foco e fecha. A V1 do Petunia proíbe janelas OS
  (cap. 36) e exige determinismo de layout; a decisão é card in-canvas, com
  posição persistida e sem "pairs out" implícito.
- O C4D separa janelas em abas por padrão; o Petunia mantém um único shell.
- Nenhum docking irrestrito foi introduzido: só existem dois estados por seção
  (ancorada ou flutuante), sem provedor de snap para áreas de drop.

## Consequências

- ADR 003 e 004 continuam válidos; este ADR só troca a unidade de controle de
  "painel monolítico" para "módulo com estado próprio".
- O bridge ganhou `UiIntent` de seção; a fronteira toolkit-neutra segue intacta
  (Slint só fala com o bridge, o Core não conhece `SectionLayout`).
- O clamp usa folga fixa de 80px, não a largura real do card: com Inspector
  largo, o card pode encostar na borda direita. Registrado como resíduo na
  matriz de gaps, não escondido.
- Ações de criação de modifier passam a poder nascer em um asset diferente do
  ativo; o Undo continua transacional por comando.

## Emenda 2026-10-04 — refatoração de UI (F4)

Aprovada pelo responsável do produto ao autorizar as fases F3 e F4 do
[plano de UI](../../development/ui-ux-refactor-plan-2026-10-04.md)
(decisões D1, D2 e D3, §12). Registrada no cap. 36, "Revisão de baseline —
2026-10-04".

1. **Modifiers vira subseção de Object (D2).** A seção deixa de ter pílula e
   cartão próprios; o corpo `ModifiersBody` é o mesmo, instanciado dentro de
   Object quando há objeto selecionado. O `SectionLayout` de `modifiers`
   continua persistido por compatibilidade, sem efeito visual.
2. **Quick Actions sai do Inspector (D2), em duas etapas.** Nesta emenda a
   seção permanece no lugar até a barra contextual da F5 recebê-la; nada é
   removido antes de ter destino.
3. **Superfície única (D3).** O painel ancorado é um só retângulo translúcido
   (alfa 0,9, ADR 004 §1) com elevação 2; as seções são planas, separadas por
   fio, sem cartão nem sombra. O card flutuante de uma seção desancorada
   continua com elevação própria.
4. **Pin visível sob demanda.** O botão de pin aparece no hover do cabeçalho,
   com foco ou quando ligado; ligado é um ícone neutro, não um fundo accent.
5. **Recolher Inspector (D1).** O cabeçalho de contexto ganha "Recolher", que
   fecha **e solta o pin** de todas as seções. Divergência registrada em
   relação ao §5 deste ADR: o pin continua protegendo a seção contra o
   recolhimento automático (fim do peek) e contra "recolher tudo" (Alt+clique),
   mas não contra este comando explícito — sem isso, como o clique na pílula
   já fixa a seção, o painel nunca recolheria.
