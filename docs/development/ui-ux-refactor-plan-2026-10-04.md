# Plano de refatoração de UI/UX — shell Slint (2026-10-04)

> **Status: PROPOSTA.** Nada aqui altera o caderno canônico. Os pontos que
> divergem de `docs/bible/` ou de ADRs estão listados em
> [§12 Decisões pendentes](#12-decisões-pendentes-do-responsável-do-produto) e só
> valem depois de aprovação explícita e registro no caderno.
> Autoridade: caps. [23](../bible/foundations/23-macroarquitetura-interface.md),
> [24](../bible/foundations/24-design-system-tokens-estados.md),
> [25](../bible/foundations/25-biblioteca-componentes-interacao.md),
> [36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md),
> [46](../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md);
> ADRs [004](../architecture/adr/004-inspector-translucido-alca-modifiers.md),
> [005](../architecture/adr/005-modulos-inspector-dock-float-pin.md),
> [007](../architecture/adr/007-workspaces-draw-poly-e-gramatica-unica.md).

## TL;DR

- **O problema não é a arquitetura de produto, é a execução visual.** O caderno
  já pede viewport-first, Inspector contextual, toolbar contextual, densidade
  progressiva e accent com um só significado. O código não entrega isso.
- **Hoje:** ~125 controles visíveis ao abrir POLY, 7 regiões de ferramentas
  competindo, accent roxo em ~15 lugares ao mesmo tempo, 60 textos com 8–9 px,
  150 cores fixas no markup, rótulos técnicos crus (`ConnectedPixels`,
  `BrightnessContrast`) e texto cortado.
- **Meta:** ≤ 45 controles visíveis ao abrir, 3 zonas de atenção, roxo só para
  "ativo/foco", textos ≥ 11 px em controles, zero hex fora dos tokens.
- **Como:** 9 fases incrementais (F0–F8). Primeiro tokens e componentes (base),
  depois shell, Inspector, ferramentas, PAINT e diálogos. Sem big-bang.
- **Próximo passo:** o responsável do produto aprova ou ajusta as 10 decisões de
  [§12](#12-decisões-pendentes-do-responsável-do-produto). A F1 pode começar sem
  elas.

---

## Andamento

| Fase | Estado | Registro |
|---|---|---|
| F1 — Tokens v2 + lint | Concluída em 2026-10-04 (testes, clippy e captura POLY/PAINT) | ver [§F1](#f1--registro-de-implementação) |
| F2 — Componentes + galeria | Concluída em 2026-10-04 | ver [§F2](#f2--registro-de-implementação) |
| F3 — Shell | Concluída em 2026-10-04, com D1 parcial | ver [§F3](#f3--registro-de-implementação) |
| F4 — Inspector | Concluída em 2026-10-04 (Quick Actions movida na F5) | ver [§F4](#f4--registro-de-implementação) |
| F5 — Ferramentas | Concluída em 2026-10-05 (F5b: grupos com flyout) | ver [§F5](#f5--registro-de-implementação) |
| F6 — PAINT | Concluída em 2026-10-05 | ver [§F6](#f6--registro-de-implementação) |
| Demais fases | Não iniciadas | aguardam §12 onde indicado |

---

## 1. Escopo

**Dentro:** o shell Slint (`crates/ui-slint/`), seus tokens, componentes,
layout, textos visíveis, diálogos e a organização do código `.slint` e do
bridge.

**Fora:**

- gramática de ferramenta, snapping e viewport 3D — já pesquisados e
  decididos no cap. 46 / ADR 007; este plano só muda **onde** e **como** os
  controles aparecem;
- site de documentação (congelado, AGENTS.md §1);
- UI egui legada (sem features novas);
- Animate (só herda os tokens e componentes novos).

## 2. Diagnóstico

### 2.1 Por tela (capturas de 2026-10-04, 1800 × 1012)

| Tela | Problema observado | Efeito para o usuário |
|---|---|---|
| Todas | 7 grupos de ferramentas espalhados: trilho esquerdo, trilho direito, 4 grupos flutuantes no topo da viewport, barra inferior | O olho não sabe para onde ir; nada parece mais importante |
| Todas | Roxo (accent) em pins, pílulas do trilho direito, CTA "Assign material", workspace, ferramenta, perfil de material, botões "Grid" | Accent perde o sentido de "ativo"; a tela parece barulhenta |
| Todas | Ícones de 11–14 px sem rótulo, alvos pequenos | Exige memorizar ícones; ruim para iniciantes e motor fino |
| Todas | Trilho direito com 6 pílulas roxas aparece **junto** do Inspector aberto | Navegação duplicada; o ADR 004 previa o trilho só no estado recolhido |
| POLY/DRAW | Inspector com 6 cartões empilhados, cada um com borda forte e sombra | Parede de painéis; viewport perde protagonismo |
| POLY/DRAW | Cabeçalho de seção com chevron acima do título e título centralizado | Parece quebrado; leitura em zigue-zague |
| POLY/DRAW | Parts: slider "Row size", lupa solta, ícone de ordenação e "Separate to new part" sempre visíveis | Controles raros ocupando lugar de controles frequentes |
| POLY/DRAW | Material: perfil como segmented de 5 opções que **transborda** o painel (`BR Standa…`, `Glass / Transparen…`); "Emission stre…" cortado | Texto ilegível; aparência de bug |
| POLY/DRAW | Object diz "Select an object" enquanto Transform mostra valores e Parts mostra "Cube" | Estado contraditório; o usuário não sabe o que está selecionado |
| POLY/DRAW | Quick actions e Modifiers como seções fixas no Inspector | Inspector longo, com rolagem logo ao abrir |
| POLY | Barra inferior com 16 ícones + lixeira, independentemente da seleção | Contraria o cap. 23 ("contextualidade antes de volume de botões") |
| PAINT | Mesmas ferramentas no trilho esquerdo **e** na barra inferior (pincel, borracha, conta-gotas, balde, seleção) | Duplicação; dúvida sobre qual usar |
| PAINT | Rótulos crus de enum: `ConnectedPixels`, `SelectedFaces`, `UvIsland`, `FirstObject`, `BrightnessContrast`, `HueSaturation` | Linguagem de programador na cara do artista |
| PAINT | Status em português (`Ferramenta ativa: brush`) com o resto em inglês | Vazamento de i18n (`lib.rs:1450` monta a frase com string fixa e id cru) |
| Status bar | `Shift+MM…` truncado; "Nothing under the cursor" ocupa o lugar mais nobre | Dica útil escondida, ruído em destaque |
| Preferências | Título da seção, ~50 px vazios, controles centralizados; botões de escala sem 175 % (o cap. 36 exige); "Accent / Selection Color" com laranja como padrão; rodapé `Theme: petunia-dark · Keymap: petunia-default` | Espaço desperdiçado; mistura **accent da UI** (violeta) com **cor de seleção 3D** (laranja); texto de depuração |
| Reference Images | 6 cartões altos em lista, cada um repetindo a mesma frase longa | Precisa rolar para ver 6 slots que caberiam numa grade 3 × 2 |

### 2.2 Métricas do código (commit `326d0a2`)

| Métrica | Valor | Regra |
|---|---|---|
| Linhas de `ui/app.slint` | 13 560 | — (monólito) |
| Linhas de `src/lib.rs` | 14 915 | — (monólito) |
| Propriedades de `PetuniaSlintShell` | 693 | — |
| Callbacks de `PetuniaSlintShell` | 288 | — |
| Textos com `font-size` 8 px / 9 px | 4 / 56 | cap. 36: "nunca 8 px"; escala começa em 10 |
| Textos com 10 px | 205 | 10 = Caption; não deveria ser o tamanho dominante |
| Cores `#hex` literais em `app.slint` | 150 | cap. 24 / AGENTS §3: zero hardcode de cor |
| Valores distintos de `drop-shadow-blur` | 8 (3–16 px) | cap. 24: hierarquia por borda, não sombra pesada |
| Fontes de ícone | Lucide (`@lucide`) + `petunia_icons.slint` | cap. 36 rev. 2026-09-30: um único pack |
| Fonte de texto | nenhuma definida (depende do sistema) | — |
| `view_layout.rs` | suporta `Quad` (4 vistas) | cap. 36: quad-view fora de escopo |

### 2.3 Implementation-vs-Spec Gap Matrix

| Requisito do caderno | Estado | Evidência |
|---|---|---|
| Viewport-first (cap. 23/36) | `PARTIALLY_COMPLIANT` | Medida de 480 × 360 respeitada, mas 7 regiões de controle e 6 cartões opacos dominam a leitura |
| Toolbar contextual (cap. 23) | `FUNCTIONAL_BUT_DIFFERENT` | Barra inferior existe, mas mostra o conjunto inteiro do workspace, não o da seleção |
| Viewport control bar LEFT/CENTER/RIGHT (cap. 23) | `FUNCTIONAL_BUT_DIFFERENT` | 4 ilhas soltas em vez de uma barra com 3 zonas |
| Inspector contextual "um painel, um contexto" (cap. 23) | `PARTIALLY_COMPLIANT` | Seções fixas; Object mostra vazio quando há objeto no Transform |
| Rail colapsado do Inspector (ADR 004 §3) | `DUPLICATED` | `app.slint:10747` desenha as pílulas com o Inspector visível |
| Accent = active/selected/focus (cap. 24/36) | `FUNCTIONAL_BUT_DIFFERENT` | Accent usado em pins, CTA, pílulas e filtros simultaneamente |
| Tipografia 10–14, nunca 8 (cap. 36) | `BROKEN` | 60 ocorrências de 8–9 px |
| Zero hardcode de cor (AGENTS §3) | `BROKEN` | 150 hex no markup |
| i18n obrigatório (cap. 36 rev. 2026-09-30 §6) | `PARTIALLY_COMPLIANT` | `Tr.t` amplamente usado; vazamentos em `lib.rs:1450` e enums crus em PAINT |
| Pack único de ícones (cap. 36 §7) | `PARTIALLY_COMPLIANT` | Lucide ainda é a fonte principal |
| UI scale 100/125/150/175/200 (cap. 36) | `PARTIALLY_COMPLIANT` | Preferências não oferecem 175 % |
| Accent violeta vs. seleção laranja (tokens) | `BROKEN` | Preferência "Accent / Selection Color" funde os dois tokens |
| Motion 90–140 ms + reduced-motion (cap. 36) | `COMPLIANT` | Global `Motion` em `tokens.slint` |
| Overlay LIFO / Esc (AGENTS §0.1) | `COMPLIANT` | `overlay.rs` + testes |
| Command Palette (cap. 36) | `PARTIALLY_COMPLIANT` | Existe; pouco visível (ícone de lupa entre 8 ícones) |

### 2.4 Causas-raiz

1. **Sem hierarquia de atenção.** Toda ação recebe um ícone permanente; nada é
   rebaixado para menu, palette ou estado contextual.
2. **Accent sem disciplina.** O roxo é usado como "estado ligado" de qualquer
   toggle, então nada se destaca.
3. **Tokens incompletos.** Faltam tokens de tipografia, elevação, tamanho de
   ícone e superfícies selecionadas; o markup compensa com valores literais.
4. **Monólito.** Um componente de 693 propriedades torna cada ajuste visual caro
   e arriscado; componentes "públicos" (cap. 25) são reimplementados localmente.
5. **Enum → UI direto.** O view-model publica identificadores internos como
   texto visível.

## 3. Referências de mercado — o que aprender

> Fonte primária de interação: cap. 46. Aqui o foco é **apresentação**.

| Referência | O que faz bem | Aplicar no Petunia | Não adotar |
|---|---|---|---|
| **Plasticity** | Chrome mínimo; parâmetros do comando aparecem num diálogo flutuante **só enquanto o comando roda**; outliner e propriedades discretos à direita; teclado primeiro | Card "Última operação" como único lugar de parâmetros transitórios; Inspector sem controles de operação | Kernel NURBS; RMB como confirmação (cap. 46 §3.6) |
| **Shapr3D** | Poucas ferramentas visíveis, agrupadas por categoria com rótulo; ações aparecem perto da seleção; alvos grandes | Trilho com grupos e flyouts; barra contextual com rótulo; alvo mínimo 28 px | Paradigma touch-only |
| **Figma (UI3)** | Toolbar no **rodapé** do canvas; painel de propriedades com rótulo à esquerda e campo à direita; canvas quase sem moldura | Barra contextual inferior como "ilha" única; grade rótulo/campo no Inspector | Painéis 100 % flutuantes por padrão |
| **Spline** | Ferramentas de 3D com estética de app moderno: superfícies planas, uma cor de destaque, tipografia clara | Superfície única do Inspector, separadores finos em vez de cartões | Visual de "brinquedo" (cap. 23: nem infantil, nem Blender amputado) |
| **Nomad Sculpt** | Popovers grandes e legíveis a partir de botões com rótulo; menus curtos | Popovers para Overlays, Shading e Câmera em vez de ilhas de ícones | — |
| **Cinema 4D / Modo** | Chips de modificador iguais em toda ferramenta | Zona CENTER da view bar com chips Pivot · Orientação · Snap · Simetria · Suave (cap. 46 §7) | Densidade de menus |

Padrão comum aos "convidativos": **pouca coisa permanente, uma cor de destaque,
rótulos onde o ícone é ambíguo e parâmetros que aparecem quando são úteis**.

## 4. Princípios de design (verificáveis)

1. **Três zonas de atenção.** Ferramentas (esquerda), contexto (direita),
   ações da seleção (rodapé da viewport). Tudo o resto vive em menu, popover ou
   palette.
2. **Accent é escasso.** Roxo preenchido só em: workspace ativo, ferramenta
   ativa, item selecionado de um segmented, botão primário (no máximo 1 por
   região). Toggles ligados usam ícone/texto em accent sobre fundo neutro.
3. **Ícone ambíguo leva rótulo.** Se 3 de 5 pessoas não nomeiam o ícone no teste
   do iniciante, ele ganha texto.
4. **Raro vai para o "⋯".** Controle usado em < 10 % das sessões sai da
   superfície principal (ex.: tamanho de linha da lista Parts).
5. **Nenhum texto cortado.** Se não cabe, o componente troca de forma
   (segmented → dropdown), nunca trunca o rótulo.
6. **Vazio explica e convida.** Estado vazio diz o que fazer e oferece a ação.
7. **Linguagem do artista.** Nenhum id, enum ou chave técnica visível.

## 5. Shell-alvo

### 5.1 DRAW / POLY

```
┌─ Top bar 40 ────────────────────────────────────────────────────────────────┐
│ ◆ Arquivo Editar Ver Janela   Sem título • Salvo   [DRAW|POLY|PAINT]  ↶ ↷ [⌕ Buscar comando…  Ctrl K] ⚙ │
├──┬──────────────────────────────────────────────────────────────┬──────────┤
│▣ │ ┌ View bar (uma ilha, 3 zonas) ──────────────────────────────┐│ INSPECTOR│
│✎ │ │ Objeto Face Aresta Ponto │ Pivô Snap Simetria │ Persp▾ ◐▾ ⧉▾││ Cube   ⋯ │
│⬚ │ └────────────────────────────────────────────────────────────┘│──────────│
│──│                                                               │▾ Parts   │
│↔ │                                                               │▾ Transf. │
│⟳ │                       VIEWPORT                                │▾ Material│
│⤢ │                                                               │▾ Objeto  │
│──│                                                               │          │
│＋│         ┌ Última operação (transitório) ┐                     │          │
│  │         └───────────────────────────────┘                     │          │
│  │      ┌ Barra contextual: ações da seleção, com rótulo ┐       │          │
│  │      │ ⇧ Extrudar  ▢ Inset  ◠ Round Edge  ✂ Cut  ⋯ Mais │      │          │
│  │      └─────────────────────────────────────────────────┘       │          │
├──┴──────────────────────────────────────────────────────────────┴──────────┤
│ [LMB] Selecionar  [MMB] Orbitar  [⇧ MMB] Mover vista   ·   1 objeto · 12 tris   ● Salvo │
└────────────────────────────────────────────────────────────────────────────┘
```

**Top bar.** Esquerda: menus + nome do projeto e estado de gravação. Centro:
workspaces (segmented com rótulo). Direita: Desfazer, Refazer, **campo de busca
de comandos** visível (porta de entrada para tudo que saiu da superfície) e
Preferências. Abrir/Salvar/Referências/Dividir vista saem dos ícones e ficam em
menus, atalhos e palette.

**Trilho de ferramentas (44 px).** Grupos separados por divisor: Seleção ·
Transformar · Ferramentas do workspace · Criar. Cada grupo mostra a última
ferramenta usada e abre **flyout** com as variantes (caixa/laço; cubo/esfera/
cilindro). Ícone 20 px, alvo 36 px, tooltip com nome + atalho + uma frase.
Preferência opcional "Mostrar rótulos" expande o trilho para ~168 px
(ver D6).

**View bar (uma ilha no topo da viewport)** — aplica o cap. 23 literalmente:

- LEFT — o que estou editando: seleção `Objeto | Face | Aresta | Ponto` (POLY) ou
  `Forma | Curva | Ponto | Região` (DRAW), com rótulo quando houver largura.
- CENTER — chips de modificador da ferramenta (cap. 46 §7). Só aparecem quando a
  ferramenta usa.
- RIGHT — como estou vendo: câmera (`Persp ▾`, com vistas e "Dividir vista"),
  sombreamento (popover com os 4 modos-base) e **Overlays ▾** (popover com
  checkboxes nomeados). Substitui os 3 grupos de ícones atuais.

**Barra contextual (rodapé da viewport).** Mostra só ações válidas para a
seleção atual, com rótulo, no máximo 6 + "Mais". Sem seleção: dica curta
("Clique numa face para extrudar") + ação de criação. Excluir fica no "Mais" e
no teclado, não como lixeira permanente.

**Status bar (22 px).** Esquerda: dicas do mouse/teclado com glifos de tecla,
nunca truncadas (somem da direita para a esquerda por prioridade). Direita:
resumo da seleção, estatísticas, gravação. Remove "Nothing under the cursor".

### 5.2 Inspector

- **Uma superfície**, não seis cartões. Seções separadas por fio de 1 px e
  título; sombra só no painel inteiro (elevação 1). Seção flutuante (ADR 005)
  continua possível e ganha elevação 2.
- **Cabeçalho de contexto** fixo no topo: ícone do tipo + nome editável do item
  selecionado + menu "⋯" (Duplicar, Excluir, Separar em nova parte, Salvar como
  asset). Sem seleção: "Nada selecionado" + 2 ações sugeridas.
- **Cabeçalho de seção**: chevron e título alinhados à esquerda numa linha; pin e
  flutuar aparecem **no hover** dentro de "⋯". Pin ligado = ícone pequeno
  neutro, nunca fundo roxo.
- **Grade rótulo/campo**: coluna de rótulo fixa (~76 px), campos ocupam o resto.
- **Parts**: campo de busca com placeholder (a lupa entra no campo); contagem no
  título ("Parts · 1"); linha = ícone, nome, etiqueta de cor; olho e cadeado no
  hover ou quando ativos. Tamanho de linha, ordenação e filtros no "⋯".
- **Transform**: três linhas `Posição / Rotação / Escala` × `X Y Z`; o eixo é
  uma barra colorida de 2 px à esquerda do campo, não uma letra vermelha;
  unidade visível (`m`, `°`); arrastar no rótulo faz scrub.
- **Material**: linha única `● Nome do material ▾` (lista, novo, duplicar,
  remover no menu); **Perfil** vira dropdown; cor base = um swatch que abre o
  seletor com a paleta; Rugosidade e Metálico como sliders com valor; Emissão e
  Albedo dentro de "Avançado" (fechado). "Atribuir material" só aparece quando a
  seleção não usa o material exibido, como botão secundário.
- **Objeto**: dados do objeto (tipo, contagens, origem). **Modifiers** vira
  subseção de Objeto (ver D2). **Quick actions** sai do Inspector: as ações vão
  para a barra contextual, o RMB e a palette (ver D2).
- **Trilho de pílulas** só existe com o Inspector recolhido (ADR 004 §3); pílula
  ativa usa marcador lateral, não preenchimento roxo (ver D1).

### 5.3 PAINT

- **Trilho esquerdo** = ferramentas de pintura (Pincel, Borracha, Conta-gotas,
  Balde, Seleção, Formas, Caminho, Projeção). Fonte única.
- **Barra contextual inferior** = ajustes do **pincel ativo**: cor, tamanho,
  força, suavização, simetria X/Y/Z, preset ▾. Deixa de repetir ferramentas.
- **Inspector PAINT**: `Camadas` (primeira, a mais usada; "Adicionar efeito ▾"
  dentro dela) → `Pincel` → `Preenchimento e projeção` → `Superfície`
  ("Preparar superfície": desdobrar, empacotar ilhas; fechada por padrão).
- **Canvas 2D** sai do Inspector e vira a divisão 3D/2D ou painel flutuante já
  previstos na revisão de 2026-09-30.
- **Rótulos humanos** (catálogo `TextId`): Pixels conectados · Face · Faces
  selecionadas · Ilha UV · Objeto; Superfície · Tela; Nenhuma · Primeiro objeto ·
  Primeira face; Pixelizar · Posterizar · Inverter · Granulado · Níveis · Brilho
  e contraste · Matiz e saturação.

### 5.4 Preferências

- Barra lateral com **busca** no topo.
- Linha padrão: rótulo + descrição curta à esquerda, controle à direita;
  grupos com título 13 px e 16 px de espaço — sem os vãos de ~50 px atuais.
- Escala da UI: 100 / 125 / 150 / **175** / 200 %.
- Separar **Cor de destaque da interface** (violeta, token `accent`) de **Cor
  de seleção na viewport** (laranja, token `selection`), cada um com redefinir.
- Rodapé de depuração sai; informação de tema/keymap vai para "Sobre".

### 5.5 Imagens de referência

- Grade 3 × 2 de blocos compactos: rótulo da vista (`Frente +Z`), área de soltar
  com miniatura, opacidade e menu "⋯" (trocar, remover, travar).
- Uma frase de ajuda no topo, não seis.
- Arrastar arquivo para o bloco carrega; "Limpar tudo" vai para o "⋯" do
  cabeçalho com confirmação.

### 5.6 Primeira execução e estados vazios

- Home do cap. 23 (`Novo`, `Abrir`, `Recuperar sessão`, `Recentes`, `Ajustes`).
- Cena nova mostra uma dica não modal: "Desenhe um retângulo no chão e puxe
  para cima" com link para o tour de 3 passos; some após a primeira operação.
- Todo painel vazio segue o princípio 6.

## 6. Design System v2 (tokens)

Mantém nomes e semântica dos caps. 24/36; **acrescenta** o que falta. Valores
são ponto de partida (tuning permitido pelo cap. 36).

### 6.1 Cor — dark

| Token | Valor | Uso |
|---|---|---|
| `surface.canvas` | `#0F1012` | fundo da viewport |
| `surface.panel` | `#17181B` | Inspector, trilhos, top bar |
| `surface.raised` | `#1E2024` | campos, popovers, card flutuante |
| `surface.hover` | `#262930` | hover |
| `surface.pressed` | `#2E323A` | pressionado |
| `surface.selected` | `accent` 16 % | linha/célula selecionada |
| `border.subtle` / `border` / `border.strong` | `#24272D` / `#2E323A` / `#3D424C` | divisores / campos / foco não-accent |
| `text.primary` / `secondary` / `muted` / `disabled` | `#ECEEF2` / `#A9B0BB` / `#7A818D` / `#555B66` | — |
| `accent` / `accent.hover` / `accent.subtle` | `#B58CFF` / `#C4A3FF` / `#B58CFF29` | ativo, foco, primário |
| `selection.viewport` / `selection.active` | `#E96A00` / `#FFAF29` | **só** geometria 3D |

Light e High Contrast derivam pelos mesmos nomes (cap. 36 rev. 2026-09-30 §5).
Todo par texto/superfície ≥ 4,5:1; ícone/borda funcional ≥ 3:1 — verificado por
teste, não a olho.

### 6.2 Tipografia

- Fonte embarcada (ver D4): **Inter** para UI, **JetBrains Mono** para campos
  numéricos e atalhos. Ambas OFL, boa leitura de 11–13 px, cobertura pt-BR.
- Escala do cap. 36 sem mudanças, com regra de uso:
  `caption 10` só badges e unidades · `small 11` rótulos e dicas ·
  `default 12` controles e valores · `strong 13` títulos de seção ·
  `exceptional 14` títulos de diálogo.
- Peso: 400 corpo, 600 títulos. Nada abaixo de 10 px.

### 6.3 Espaço, tamanho, forma

- Espaço: `4 · 8 · 12 · 16 · 24` (subconjunto da escala do cap. 24).
- Controle: `24` compacto (ícone em lista) · `28` padrão · `32` primário.
- Ícone: `16` em controles, `20` em trilhos; traço 1,75 (cap. 36 §7).
- Raio: o do cap. 36 (4 / 5 / 8 / 10 / pill).
- **Elevação — 3 níveis** em vez de 8 sombras:
  `0` plano · `1` painel (borda + sombra 8 px / 25 %) · `2` popover e flutuante
  (borda + sombra 16 px / 40 %).

### 6.4 Estados obrigatórios por componente

`default · hover · pressed · focus-visible · selected · disabled · loading`
(cap. 24). Foco = anel de 2 px `focus-ring`, sempre visível por teclado.

## 7. Componentes (cap. 25)

| Ação | Componentes |
|---|---|
| **Manter e padronizar** | `NumericField`, `Vector3Field`, `ColorSwatch`, `RichTooltip`, `MenuDropdown`, `ContextMenuItem`, `PetuniaSlider` |
| **Unificar** | `TopAction` + `ToolButton` + `ViewportBarButton` + `ViewportSvgButton` → `IconButton` (variantes `ghost / toggle / tool`); `WorkspaceSegment` + segmented locais → `Segmented` |
| **Novos** | `Dropdown`, `Popover`, `ContextBar`, `ViewBar`, `ToolGroup` (com flyout), `PropertyRow` (rótulo/campo), `EmptyState`, `KeyHint`, `SearchField`, `Toast` padronizado |
| **Refazer** | `InspectorSection` (cabeçalho em linha, ações no hover, sem cartão), `ReferenceSlotCard` (bloco de grade) |
| **Remover da superfície** | `PartsRowSizeControl` (vai para "⋯"), `QuickActionsBody` (vira fonte de dados da barra contextual) |

Cada componente: tokens apenas, 7 estados, `accessible-role`/`label`, exemplo
na **galeria** (`crates/ui-slint/examples/gallery.rs`) e teste.

## 8. Arquitetura de código

1. **Dividir o monólito** sem mudar comportamento:

   ```
   ui/
     tokens.slint            cores, tipo, espaço, elevação, motion
     components/             IconButton, Segmented, Dropdown, PropertyRow…
     shell/                  TopBar, ToolRail, ViewBar, ContextBar, StatusBar
     inspector/              Parts, Transform, Material, Object, Modifiers
     workspaces/             draw_poly.slint, paint.slint (animate.slint já existe)
     dialogs/                Preferences, References, CommandPalette, Home
     app.slint               composição apenas
   ```

2. **Globals por domínio** em vez de 693 propriedades no `Window`:
   `ShellState`, `ToolState`, `InspectorState`, `PaintState`, `ViewportState`,
   cada uma com seus callbacks. O Rust acessa por `app.global::<T>()`. O
   `lib.rs` se divide no mesmo eixo (`bridge/inspector.rs`, `bridge/paint.rs`…).
   `UiIntent` continua a única fronteira com o domínio (AGENTS §0.1).
3. **Ferramentas por dados.** Trilho e barra contextual recebem
   `[ToolEntry { command-id, icon-id, label, shortcut, group, enabled }]` gerado
   do Command Registry. Uma ferramenta nova não toca markup.
4. **Rótulos por catálogo.** O view-model nunca publica `Debug`/enum como texto;
   cada enum visível tem `TextId` e teste de paridade en/pt-BR.
5. **Guarda automática** (`xtask ui-lint`, estende o `ui-guard`):
   falha com `#hex` fora de `tokens.slint`, `font-size` < 10 px, `Text { text:
   "literal" }` e `drop-shadow` fora dos tokens de elevação.
6. **Regressão visual (spike na F0).** Renderizar o shell com o renderer de
   software do Slint em 1280 × 800 e 1920 × 1080 por workspace e comparar com
   referências. Se o spike falhar, cair para captura manual registrada.

## 9. Acessibilidade

- Teclado: F6 entre regiões, Tab dentro, setas em segmented/listas (cap. 36).
- Alvo mínimo 28 px (24 só em listas densas com espaçamento).
- Contraste verificado por teste (§6.1).
- Modo "Mostrar rótulos" no trilho; tooltips com atraso curto e sem bloquear.
- Reduced-motion já existe; manter em todo componente novo.
- Linguagem simples nas dicas (frase curta, verbo primeiro).

## 10. Roteiro

Cada fase é um PR pequeno, com commit/push (AGENTS §7), gates do AGENTS §4 e
**limpeza de build ao final** (`cargo clean -p petunia_ui_slint` ou remoção do
`target/` do worktree).

| Fase | Entrega | Critério de aceite | Depende |
|---|---|---|---|
| **F0 — Fundação** | Aprovação de §12; spike de captura por software; baseline de métricas (contagem de controles, hex, fontes) gravada | Métricas reproduzíveis por script; decisão registrada no caderno | — |
| **F1 — Tokens v2 + lint** | Tokens novos; troca de 150 hex e de 8–9 px por tokens; 3 níveis de elevação; `xtask ui-lint` | `ui-lint` verde; zero hex fora de tokens; zero texto < 10 px; testes Slint verdes | — |
| **F2 — Componentes + galeria** | Divisão de `app.slint`; `IconButton`, `Segmented`, `Dropdown`, `Popover`, `PropertyRow`, `EmptyState`; galeria | Galeria mostra 7 estados por componente; nenhum comportamento alterado (testes existentes verdes) | F1 |
| **F3 — Shell** | Top bar enxuta + busca; View bar única com 3 zonas; status bar; trilho direito só recolhido | Controles visíveis em POLY ≤ 70; nenhum rótulo truncado em 1280 × 800 | F2, D1 |
| **F4 — Inspector** | Superfície única; cabeçalho de contexto; Parts/Transform/Material/Object novos; estados vazios coerentes | Material sem texto cortado em 280 px; Object e Transform nunca contraditórios (teste) | F2, D2, D3 |
| **F5 — Ferramentas** | Trilho por dados com grupos/flyouts; barra contextual por seleção; modo rótulos | Controles visíveis em POLY ≤ 45; barra muda com a seleção (teste) | F3, D6 |
| **F6 — PAINT** | Sem duplicação trilho/barra; barra de pincel; Inspector reordenado; rótulos humanos; fix `lib.rs:1450` | Zero enum cru visível (teste varre o view-model); paridade i18n verde | F5 |
| **F7 — Diálogos** | Preferências, Referências, Palette, Home | 175 % disponível; accent ≠ seleção; 6 referências sem rolagem em 1280 × 800 | F2, D5, D8 |
| **F8 — Acessibilidade e teste** | Auditoria WCAG 2.2 AA; teste do iniciante (`user-test-protocol.md`); polimento de motion | Metas da §11 atingidas ou desvios registrados | F3–F7 |

Ordem sugerida de PRs: F1 → F2 → F3 → F4 → F5 → F6 → F7 → F8. F0 corre em
paralelo à F1.

## 11. Métricas de sucesso

| Métrica | Hoje | Meta |
|---|---|---|
| Controles visíveis ao abrir POLY (1800 × 1012) | ~125 (contagem da captura) | ≤ 45 |
| Regiões com ferramentas | 7 | 3 (+ top bar) |
| Elementos com fundo accent em repouso | ~15 | ≤ 3 (workspace, ferramenta, seleção de segmented) |
| Texto < 10 px | 60 | 0 |
| `#hex` fora de `tokens.slint` | 150 | 0 |
| Rótulos truncados em 1280 × 800 | ≥ 6 | 0 |
| Enums/ids crus visíveis | ≥ 10 | 0 |
| Teste do iniciante: criar forma, extrudar, pintar sem ajuda | não medido | ≥ 4 de 5 participantes |

## 12. Decisões pendentes do responsável do produto

Cada item conflita ou toca o caderno/ADRs. Recomendação em **negrito**.

| # | Decisão | Conflito | Recomendação |
|---|---|---|---|
| D1 | Trilho de pílulas só com Inspector recolhido; pílula ativa sem fundo roxo | Implementação diverge do ADR 004 §3; alinhar | **Aprovar** (corrige para o ADR) |
| D2 | Quick actions sai do Inspector; Modifiers vira subseção de Objeto | ADR 004 §6 e ADR 005 §1 listam 6 seções; cap. 36 congela só 4 | **Aprovar** e emendar ADR 005 |
| D3 | Inspector como superfície única opaca (sem 6 cartões com sombra) | ADR 004 §1 pede alfa 0,9 no painel | **Manter alfa 0,9 no painel**, remover cartões e sombras das seções |
| D4 | Fontes embarcadas Inter + JetBrains Mono | Nova dependência de asset (cap. 18) | **Aprovar** (OFL, sem código) |
| D5 | Separar preferência de accent da cor de seleção 3D | Correção de bug contra tokens | **Aprovar** |
| D6 | Modo opcional "Mostrar rótulos" com trilho ~168 px | Cap. 36 fixa trilho em 40–46 px | **Aprovar como preferência desligada por padrão** |
| D7 | Não expor `ViewLayout::Quad` | `view_layout.rs` suporta 4 vistas; cap. 36 limita a 2 | **Remover `Quad` ou mantê-lo só em teste** |
| D8 | Adicionar 175 % nas preferências | Cap. 36 já exige | **Aprovar** (correção) |
| D9 | Campo de busca de comandos visível na top bar | Nenhum; reforça a Command Palette | **Aprovar** |
| D10 | Lixeira deixa de ser botão permanente na barra contextual | Nenhum no caderno | **Aprovar** (Delete no teclado, RMB e "Mais") |

## 13. Riscos

| Risco | Mitigação |
|---|---|
| Regressão funcional ao dividir o monólito | F2 sem mudança de comportamento; testes existentes como rede; PRs pequenos |
| Fontes aumentam o binário | Subconjunto latino + pesos 400/600 apenas |
| Barra contextual "esconde" ações | Palette e RMB sempre têm tudo; teste do iniciante mede descoberta |
| Spike de captura por software não reproduz o WGPU | A captura cobre só o chrome; viewport validada à parte (matrizes de viewport) |
| Escopo cresce para "redesign total" | Cada fase tem critério de aceite fechado; itens novos vão para backlog |

## 14. Higiene de build

Pedido do responsável (2026-10-04): **limpar compilações desnecessárias depois
de usá-las**. Ao final de cada fase:

```bash
du -sh target
cargo clean -p petunia_ui_slint
```

Quando o worktree for descartado, remover o `target/` inteiro dele. Registrar a
limpeza no relatório da fase.

## F1 — registro de implementação

Feito em 2026-10-04, sem depender das decisões da §12.

- `tokens.slint`: tokens de tipografia, elevação (2 níveis com sombra + brilho
  de "armado"), cores de sobreposição da viewport/HUD e o global
  `ColorPresets` com as paletas rápidas.
- `app.slint` e `animate.slint`: 150 cores `#hex` trocadas por tokens; todos os
  `font-size` em px viraram `DesignTokens.font-*`; os 60 textos de 8–9 px
  sobem para 10 px (`font-caption`); as 8 variações de sombra viraram
  `elevation-1` (blur ≤ 10 px) ou `elevation-2` (blur ≥ 12 px).
- Texto branco sobre accent nos toggles de simetria, alvo e máscara do PAINT
  passou a `on-accent` (escuro), que tem contraste maior sobre o violeta.
- `xtask ui-lint` (`crates/xtask/src/slint_lint.rs`) com testes; ligado ao job
  `UI Slint` do CI.
- Fora do escopo da F1, já anotado para fases seguintes: o texto fixo
  `(Insert)` no aviso de edição de pivô e as letras de eixo `X/Y/Z`, `UV`,
  `U →`, `V ↑` (F6, i18n); presets de seleção ainda comparados por string hex
  (F7, junto da D5).

## F2 — registro de implementação

Feito em 2026-10-04. Nenhum comportamento muda.

- `app.slint` dividido (13 560 → ~10 100 linhas): `types.slint` (structs
  públicas, reexportadas para o Rust), `components/feedback.slint`
  (`RichTooltip`), `components/controls.slint` (botões, campos, menus),
  `components/paint_canvas.slint`, `inspector/sections.slint` (seções e corpos
  do Inspector) e `dialogs/references.slint`. Os imports foram calculados por
  uso; o shell ficou em `app.slint` porque depende de ~700 propriedades da
  janela (globals por domínio são a etapa seguinte, §8.2).
- Componentes novos em `components/base.slint`: `IconButton` (ghost / toggle /
  tool), `Segmented` (setas ←/→), `DropdownButton` (só o gatilho; a lista é
  `MenuDropdown` via `OverlayStack`), `PropertyRow`, `EmptyState`, `KeyHint` e
  `CommandSearchField`. Tokens novos: `accent-subtle`, `icon-small/control/rail`.
- Galeria: `gallery.slint` + `cargo run -p petunia_ui_slint --example gallery`
  (`PETUNIA_LANG=pt-BR` para português). Textos em `[gallery]` nos dois
  catálogos; teste `gallery_receives_translations`.
- `ui-lint` passa a varrer subpastas de `ui/`.

## F3 — registro de implementação

Feito em 2026-10-04 por pedido do responsável ("prossiga com F2 e F3"), o que
aplica as recomendações de **D1** e **D9**.

- **Top bar:** saem os ícones Parts, Abrir, Salvar, Busca e Referências. Entra
  o `CommandSearchField` ("Buscar comandos…", abre a palette). Ficam Desfazer,
  Refazer e Preferências. Abrir/Salvar continuam em Arquivo e nos atalhos;
  **View → Imagens de referência (F4)** e **Window → Parts** são itens novos
  (teste `top_bar_actions_moved_to_menus_stay_reachable`).
- **View bar única:** as três ilhas (câmera, sombreamento/auxiliares, domínio)
  viram uma barra centrada na área livre da viewport, com LEFT = domínio de
  seleção, CENTER = pivô/snap/proporcional, RIGHT = projeção (rótulo
  Persp/Ortho), enquadrar, redefinir, dividir, 4 modos de sombreamento, raio-X
  e opções. Toggles usam `IconButton` "toggle" (fundo suave); o botão de
  referências sai da barra (menu + F4).
- **Status bar:** a mensagem neutra fica discreta (destaque só para
  aviso/erro/sucesso); as dicas viram chips `KeyHint` (LMB, MMB, Shift+MMB,
  Roda) que somem da direita para a esquerda abaixo de 1240/1100 px, em vez
  do texto truncado.
- **D1 (parcial):** pílula de seção aberta = `accent-subtle` + marcador lateral
  de 3 px, ícone em accent; trilho de 56 → 50 px (pílula mantém 36 px, contrato do ADR 004 coberto por teste de gestos). **Pendente para a F4:** o
  trilho é hoje o único controle que abre/fecha o painel (hover e pílulas);
  escondê-lo com o painel aberto exige antes o botão de recolher no cabeçalho
  de contexto do Inspector.
- **Resíduos registrados:** nome do projeto na top bar (o view-model não expõe
  o nome; F7); popovers de Câmera/Overlays (precisam de entrada no
  `OverlayStack`; F5); textos fixos em português no HUD da ferramenta ativa
  (`"Arraste o corte…"`, `"Mova o mouse…"`) e o padrão `"Ready"` (F6, i18n).
- **Achados da captura (2026-10-04):** (1) o Rust sempre publica uma dica de
  contexto ("Selection: Object · LMB Select · …"), então os chips `KeyHint`
  quase nunca aparecem; o texto agora cabe sem truncar, mas a dica estruturada
  (lista de pares tecla/ação no view-model) fica para a F5. (2) O diálogo
  "Recover unsaved work?" não bloqueia cliques na viewport: um clique atrás
  dele desmarcou o objeto. Bug preexistente, fora do escopo da F3.

## F4 — registro de implementação

Feito em 2026-10-04 por pedido do responsável ("prossiga"), aplicando as
recomendações de **D2** e **D3** e completando a **D1**. Registrado no cap. 36
("Revisão de baseline — 2026-10-04") e nas emendas dos ADRs 004 e 005.

- **Superfície única (D3):** o painel do Inspector é um retângulo translúcido
  (alfa 0,9) com elevação 2; `InspectorSection` ficou plana (fio no topo, sem
  cartão/sombra), título 13 px alinhado à esquerda — corrigido o bug em que o
  chevron esticava e empurrava o título para o centro.
- **Pin sob demanda:** aparece no hover do cabeçalho, com foco ou ligado;
  ligado = ícone neutro.
- **Cabeçalho de contexto:** ícone + nome do objeto (ou "Nada selecionado"),
  Duplicar e Excluir (saíram do corpo de Objeto) e **Recolher Inspector**.
- **D1 completa:** o trilho de pílulas virou sobreposição na borda direita da
  viewport (a viewport ganha 50 px) e só aparece com o painel recolhido; o
  painel aberto o cobre. Gizmo XYZ, view bar e transporte do Animate usam a
  nova folga `right-clearance`. "Recolher" fecha e solta o pin de todas as
  seções (`collapse_inspector`, teste `collapse_inspector_closes_and_unpins_every_section`).
- **D2:** Modifiers virou subseção de Object (sem pílula; teste de gestos
  atualizado para 4 pílulas). Quick Actions continua no Inspector até a barra
  contextual da F5 recebê-la — nada é removido sem destino.
- **Material:** o perfil virou grade 2 colunas com rótulo acima; os cinco
  nomes cabem inteiros.
- **Sem seleção, nada contraditório:** Transform só aparece com objeto
  selecionado; sem seleção, um `EmptyState` explica o que fazer (teste
  `inspector_never_shows_transform_values_without_a_selection`). O clique real
  no botão Recolher é coberto por `collapse_inspector_button_is_clickable_and_returns_to_the_rail`.
- **Correção da F3:** os chips da status bar dependiam de `root.width`, o que
  criava *binding loop* com o layout (aviso do Slint, risco de pânico); agora o
  grupo recorta à direita sem consultar a largura.
- **Resíduos para a F5+:** grade rótulo/campo (`PropertyRow`) nas seções
  Transform/Material, barra de eixo no lugar da letra colorida, opções raras de
  Parts (tamanho de linha, ordenação) num menu "⋯", Quick Actions na barra
  contextual.

## F5 — registro de implementação

Feito em 2026-10-05 com autorização do responsável para todas as fases
restantes; aplica **D2** (parte final), **D6** e **D10**.

- **Barra contextual por seleção:** em POLY, cada ferramenta só aparece no
  domínio de seleção em que faz sentido (`poly-tool-visible` em `app.slint`):
  objeto → Push/Pull e Subdivide; face → Extrude, Push/Pull, Inset,
  Subdivide, Spin, Dissolve; aresta → Round Edge, Connect, Make Face,
  Subdivide, Spin, Dissolve; ponto → Merge, Make Face, Dissolve. Poly Pen,
  Knife, Loop Cut e Slice aparecem sempre. DRAW, PAINT e UV não mudam. Teste
  `poly_context_bar_follows_the_selection_domain`.
- **Vocabulário:** o botão da barra dizia "Bevel" (`sl.bevel`); agora diz
  "Round Edge" em en e pt-BR, como pede o cap. 13.
- **Ações rápidas (D2):** saíram do Inspector (seção e pílula removidas) e
  moram na barra contextual, com rótulo. O botão de ajuste abre um cartão não
  modal acima da barra para personalizar (mesmo `QuickActionsBody`).
- **Lixeira (D10):** deixou de ser botão permanente da barra; excluir continua
  pela tecla Delete, pelo menu do botão direito e pelo cabeçalho do Inspector.
- **Modo "Mostrar nomes" (D6):** preferência `show_tool_labels`, desligada por
  padrão, persistida em `preferences.toml` e exposta em Preferências. Ligada,
  o trilho vai de 44 px para 148 px e `ToolButton`/`ViewportSvgButton` mostram
  o nome ao lado do ícone (o plano previa ~168 px; 148 px já cabe "Lasso
  Select" e "Asset Library"). Teste `show_tool_labels_preference_reaches_the_view_model`.
- **Dica em repouso:** a frase fixa em inglês "Selection: Object · LMB
  Select…" saiu; em repouso a dica fica vazia e os gestos aparecem como chips
  traduzidos na status bar (teste do HUD atualizado).
- **F5b — grupos com flyout:** novo `ToolGroup` (`components/controls.slint`)
  recebe uma lista `[RailTool]` (id, rótulo, atalho, descrição, ícone,
  ativo), mostra a variação ativa ou a última usada e abre o flyout pelo
  botão direito ou pela marca de canto; Esc ou clique fora fecham. Em
  DRAW/POLY, Seleção+Laço e Cursor 3D+Medir viraram dois grupos; Adicionar
  Cubo/Esfera/Cilindro saíram do trilho de POLY (estão no menu Adicionar); as
  cinco ferramentas de DRAW que se repetiam na barra contextual saíram dela.
  O trilho de POLY passou de 14 para 9 controles. O resto do trilho continua
  declarado à mão; a contagem total de controles é medida na F0.

## F6 — registro de implementação

Feito em 2026-10-05.

- **Fonte única de ferramentas:** o trilho do PAINT é declarado por dados
  (Seleção, grupo Pincel/Aerógrafo, Borracha, Conta-gotas, Balde, grupo
  Formas Linha/Retângulo/Elipse, grupo Gradiente linear/radial, Seleção UV);
  as 11 ferramentas repetidas na barra contextual saíram dela.
- **Barra de pincel:** a barra contextual do PAINT mostra Isolar, a cor
  atual, tamanho −/+, opacidade −/+ e simetria X/Y/Z.
- **Inspector reordenado:** Camadas → Efeitos → Paleta → Pincel →
  Preenchimento e projeção → Canvas → Preparar superfície (fechada) →
  Camadas de textura (fechada; hoje é só um indicador fixo, ponto para a
  conversa sobre o PAINT).
- **Rótulos humanos:** `paint-label()` traduz os ids de escopo de
  preenchimento, projeção, trava e efeitos (17 chaves `sl.paint_opt_*` em en e
  pt-BR); o escopo de preenchimento virou coluna para "Pixels conectados" não
  cortar.
- **i18n do status:** "Ferramenta ativa: {tool}" e "Modo de seleção:
  {domain}" vêm de `TextId` (`tools.active`, `selection.mode`,
  `selection.domain_*`); antes eram frases fixas em português com id cru.
- **Fora desta fase:** o canvas 2D continua como seção e painel flutuante; a
  revisão profunda do PAINT fica para a conversa pedida pelo responsável.
