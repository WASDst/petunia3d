# Workspaces, Feedback Visual e Acessibilidade

> **Status: aprovado em 2026-10-08.**
>
> Este capítulo complementa [Slint Reassessment & GUI Architecture](./slint-reassessment.md). Ele não redefine o toolkit: Slint permanece o frontend principal da refatoração e OpenGL 3.3 Core permanece o renderer 3D de destino. Aqui fechamos como o shell se especializa por workspace e qual gramática visual/interativa todos os workspaces devem compartilhar.

## 1. Princípio central: gramática espacial estável, conteúdo contextual

Petunia3D não terá uma GUI completamente diferente em cada workspace. O usuário aprende uma única geografia:

```text
┌─────────────────────────────────────────────────────────────┐
│ Header global                                                │
├──────┬──────────────────────────────┬────────────────────────┤
│      │                              │ STRUCTURE              │
│ Tool │                              │ "o que existe aqui?"   │
│ Rail │       WORK SURFACE           ├────────────────────────┤
│      │                              │ PROPERTIES             │
│      │                              │ "como é o atual?"       │
│      │ Contextual Action Bar        │                        │
├──────┴──────────────────────────────┴────────────────────────┤
│ WORKSPACE DRAWER / AUXILIARY SURFACE                        │
└─────────────────────────────────────────────────────────────┘
```

As posições permanecem previsíveis. O conteúdo e a função mudam conforme o domínio.

Isso evita dois extremos:
- um shell MODEL-first que comprime PAINT/UV/ANIMATE no Inspector;
- quatro aplicativos visualmente diferentes dentro do mesmo produto.

## 2. Regiões semânticas

### Header global
Contém apenas contexto global e navegação de alto nível:
- arquivo/projeto;
- undo/redo;
- workspaces;
- preferências;
- comandos globais;
- estado de documento quando necessário.

Não é local para ferramentas específicas do workspace.

### Tool Rail
Ferramentas que mudam o comportamento direto do ponteiro/cursor na work surface.

Regra:
- ferramenta persistente → Tool Rail;
- ação pontual → Context Bar, Properties, menu ou Command Palette;
- variações de uma mesma família → ToolGroup/flyout;
- rótulos podem ser exibidos permanentemente via preferência de acessibilidade.

### Work Surface
É a área produtiva principal. Pode ser:
- viewport 3D;
- canvas 2D;
- UV Editor;
- timeline/curve editor;
- composição 3D + 2D.

A work surface nunca deve ser reduzida artificialmente para caber em um Inspector.

### Structure
Responde:

> "Quais entidades relevantes existem neste domínio?"

Exemplos:
- MODEL → Scene / Parts;
- PAINT → Layers;
- UV → UV Maps / Islands;
- ANIMATE → Creature / Motion Stack / Rig hierarchy.

### Properties
Responde:

> "Quais propriedades e operações pertencem ao contexto atualmente selecionado?"

Implementação alvo:
- `PropertySectionDescriptor`;
- `PropertySectionRegistry`;
- DTOs de leitura vindos da Application;
- intents/commands na escrita;
- nenhuma dependência de ECS.

### Contextual Action Bar
Ações de alta frequência da operação atual:
- confirmar/cancelar;
- parâmetros modais;
- opções rápidas;
- snap/constraint relevantes;
- status legível da ferramenta.

Não deve duplicar permanentemente o Inspector.

### Workspace Drawer
Região inferior opcional e redimensionável.

Conteúdo padrão por workspace:
- MODEL → Assets / Prefabs;
- PAINT → Assets / Brush Presets / Palette;
- UV → normalmente recolhido; Assets ou diagnósticos conforme contexto;
- ANIMATE → Timeline / Motion Stack.

"Asset Library" deixa de ser semanticamente obrigatório como único conteúdo inferior.

## 3. Matriz de workspaces

| Workspace | Work Surface | Structure | Properties | Drawer |
|---|---|---|---|---|
| DRAW | 3D viewport shape-first | Scene / Parts | Shape / Generator / Transform / Material | Assets / Prefabs |
| POLY | 3D viewport topology-first | Scene / Parts | Geometry / Transform / Modifiers / Material | Assets / Prefabs |
| PAINT | 3D, 2D ou composição 3D+2D | Layers | Target / Brush / Layer / Projection / Effects | Assets / Brushes / Palette |
| UV | 3D + UV Editor em split | UV Maps / Islands | Selection / Unwrap / Projection / Pack / Density / Health | recolhido por padrão |
| ANIMATE | 3D viewport + transport | Creature / Motions / Rig quando necessário | Motion / Rig / Advanced | Timeline / Motion Stack |

## 4. PAINT: regra estrutural

PAINT não é um Inspector de MODEL com brushes extras.

O estado atual já possui:
- brush/pixel/eraser/fill/picker;
- line/rectangle/gradient;
- spray/smudge/blur/dodge/burn/clone;
- brush presets;
- symmetry;
- selection mask;
- texture/vertex target;
- layers, groups, decals, opacity, visibility, lock, merge;
- effects;
- canvas 2D;
- UV overlay;
- PiP 2D/3D;
- projection e fill scopes.

A refatoração deve reorganizar, não reimplementar sem necessidade.

### Structure
`Layers` vira a estrutura primária do workspace.

### Properties
Contextual:
- brush ativo → Brush/Stroke;
- layer ativa → Layer Properties;
- decal layer → Decal;
- fill → Fill/Projection;
- material target → Channel/Material context.

### Work Surface
Canvas 2D deixa de ser miniatura funcional presa ao Inspector.

Estados oficiais iniciais:
- 3D;
- 2D;
- 3D + 2D;
- 2D + 3D PiP.

## 5. UV: regra estrutural

O editor UV atual de 256×256 no Inspector é considerado uma limitação de apresentação, não o destino arquitetural.

O workspace abre por padrão com split redimensionável:

```text
3D Viewport  |  UV Editor
    ~55%     |    ~45%
```

Cada lado pode futuramente ser maximizado.

### Structure
- UV Maps / Sets;
- Islands.

### Properties
- Selection;
- Unwrap;
- Projection;
- Pack;
- Texel Density;
- UV Health.

### UV Health
Diagnósticos deixam de ser números passivos. Sempre que possível devem ser acionáveis:
- overlap;
- stretch;
- zero-area;
- out-of-range;
- density mismatch;
- seam/split inconsistency.

Clicar no problema deve selecionar ou enquadrar o alvo quando tecnicamente viável.

## 6. ANIMATE: regra estrutural

ANIMATE preserva a direção procedural-first existente.

O sistema atual `Slint → AnimateIntent → Application/commands → domínio` é referência para os demais módulos.

Não transformar a superfície inicial em um clone simplificado do Blender.

### Structure
- Creature;
- Motion Stack;
- Rig hierarchy apenas quando relevante.

### Properties
- Motion;
- Style;
- parâmetros universais;
- Rig/controle selecionado;
- Advanced.

### Drawer
Timeline/Motion Stack é o conteúdo inferior padrão.

Keyframes, curves e edição fina são superfícies avançadas; não devem dominar a experiência inicial.

## 7. Gramática de estados visuais

Todo controle deve distinguir semanticamente:

### Default
Disponível e inativo.

### Hover
"O que será atingido se eu agir agora?"
- não muda documento;
- não equivale a seleção;
- deve ser sutil.

### Pressed
Pointer/key activation em andamento.

### Focus
Foco de teclado.
- independente de hover;
- ring visível;
- não depende exclusivamente de cor.

### Selected
Pertence ao conjunto selecionado.

### Active
Elemento primário/current dentro de uma seleção ou contexto.

### Armed
Ferramenta esperando o próximo gesto.

### Live
Sessão/modal procedural dirigindo preview.

### Disabled
Relevante, porém indisponível.
- permanece identificável;
- expõe motivo quando não óbvio;
- não some apenas porque falta uma pré-condição.

### Invalid / Warning / Error / Success
Semântica própria e nunca comunicada apenas por cor.

## 8. Listas

Outliner, Layers, UV Islands, Assets, Motions, Timeline tracks e Command Palette usam o mesmo contrato:

```text
hover
+ selected
+ active/current
+ keyboard focus
```

Esses estados podem coexistir e precisam ser distinguíveis.

Listas potencialmente longas usam `ListView`/virtualização.

## 9. Ícones e rótulos

### Tool Rail
Ícone pode ser a superfície padrão quando:
- semântica é estável;
- tooltip rico existe;
- `accessible-label` existe.

Preferência "Mostrar nomes das ferramentas" expande o rail e é tratada como recurso oficial de acessibilidade.

### Menus e ações destrutivas
Texto visível é obrigatório.

### Ações universais compactas
Fechar, visibilidade, lock, undo/redo podem usar ícone isolado, sempre com nome semântico e tooltip quando aplicável.

### Ferramentas raras/ambíguas
Preferir ícone + texto.

## 10. Tooltip

`RichTooltip` é o componente canônico.

Estrutura:

```text
Título                         Atalho
Descrição curta
Gesto / dica contextual
```

Tooltip explica; não substitui:
- label obrigatório;
- erro persistente;
- disabled reason;
- status crítico.

## 11. NumericField

Preservar e consolidar o comportamento atual:

```text
click          → edição digitada
drag horizontal→ scrub
Shift + drag   → fine scrub
Enter          → commit
Esc            → restaura original/cancela
```

O parser pode continuar aceitando expressões e operações relativas.

A roda do mouse NÃO altera valor apenas por estar sobre o campo. Wheel rola o painel por padrão. Ajuste por wheel exige foco explícito ou gesto deliberado futuro.

## 12. Sliders

Slider serve exploração; entrada numérica serve precisão.

Quando precisão for relevante:

```text
Opacity     ─────●────   72%
```

Slider e NumericField representam uma única fonte de valor.

Drag produz preview; release/commit deve formar um único Undo quando a operação for autoral.

## 13. Scroll

Política global:
- painel/lista → scroll;
- viewport 3D → zoom;
- canvas 2D → zoom;
- timeline → navegação temporal conforme gesture map.

Controles filhos não sequestram wheel acidentalmente.

Evitar nested scroll.

## 14. Feedback da viewport

Toda ferramenta deve conseguir projetar os seguintes estados sem inventar uma linguagem própria:

- hover candidate;
- selected;
- active;
- snap candidate;
- snap accepted;
- eixo/plano restrito;
- armed;
- live preview;
- valor numérico digitado;
- resultado válido;
- resultado inválido;
- warning;
- commit;
- cancel.

A mesma semântica usa tokens/ícones/labels coerentes entre ferramentas.

## 15. Comunicação: status, toast, notification e dialog

### Status Bar
Contexto contínuo:
- ferramenta atual;
- gesto esperado;
- valor/estado corrente;
- progresso leve.

### Inline Message
Problema ou aviso ligado diretamente ao controle/feature.

### Toast
Evento transitório:
- saved;
- exported;
- operação concluída;
- warning não bloqueante.

Erro crítico nunca existe apenas em toast.

### Notification / Problems
Evento persistente e revisável:
- import parcial;
- asset ausente;
- plugin falhou;
- recurso incompatível;
- múltiplos warnings.

### Dialog
Somente para decisão realmente bloqueante ou com consequência relevante:
- mudanças não salvas;
- recuperação;
- overwrite;
- ação destrutiva irreversível;
- migração de arquivo.

## 16. OverlayStack

Preservar os tipos atuais:
- Tooltip;
- Popover;
- ContextMenu;
- FloatingPanel;
- Drawer;
- Modal.

Regras adicionais obrigatórias:
1. lembrar invocador;
2. mover foco para a superfície aberta;
3. modal prende foco;
4. Escape fecha apenas o topo dismissible;
5. fechamento restaura foco ao invocador quando ainda válido;
6. click-away nunca confirma operação destrutiva;
7. painéis pinados não fecham por click-away;
8. **popover não modal não rouba foco** (decisão §26): enquanto está aberto, o shell é dono de um índice de highlight — o mesmo padrão da barra de menus —, as setas movem o destaque, Enter invoca o item destacado e o foco permanece no invocador.

A regra 2 vale para superfícies modais (Modal) e para o foco inicial já existente em listas/segmented. Popovers e flyouts não modais adotam a regra 8: sem entrada automática de foco, sem contenção de Tab e sem bloquear F6/atalhos — é o que os mantém não modais.

## 17. Badges

Badge comunica estado compacto, nunca decoração.

Exemplos válidos:
- `SNAP · Vertex`;
- `PRECISION`;
- `LIVE`;
- `READ ONLY`;
- `SHARED`;
- `UNSAVED`;
- `3 WARNINGS`.

Badge nunca depende apenas de cor.

## 18. Acessibilidade — contrato de produto

Acessibilidade permanece hard gate.

### Navegação regional

Ordem alvo de `F6`:

```text
Header
→ Tool Rail
→ Work Surface
→ Structure
→ Properties
→ Workspace Drawer
→ Context Bar
```

`Shift+F6` percorre ao contrário.

`Tab/Shift+Tab` navega dentro da região atual.

### Requisitos
- foco sempre visível;
- roles/names/states corretos;
- Enter/Space em controles focáveis;
- setas em segmented/listas quando apropriado;
- foco restaurado após overlays;
- High Contrast;
- Reduced Motion;
- UI Scale 100–200%;
- HiDPI;
- keymap configurável;
- labels persistentes opcionais para tools;
- hit targets adequados;
- drag threshold configurável;
- click-move-click como alternativa formal a drag contínuo;
- tamanho ajustável de gizmos/handles;
- feedback não dependente apenas de cor;
- IME e entrada de texto real testados.

Não declarar conformidade WCAG enquanto navegação regional, árvore acessível e testes assistivos não forem auditados.

## 19. Dívidas concretas observadas

### Controles ad-hoc
Ainda existem muitos `Rectangle + TouchArea` implementando botões/toggles/list rows.

Direção:
- migrar progressivamente para componentes base;
- não fazer big-bang rewrite;
- cada componente base precisa implementar default/hover/pressed/focus/selected/disabled e semântica acessível.

### Tipografia
Tokens atuais ainda incluem 10/11 px.

Texto funcional normal deve começar em **12 logical px**. Tamanhos menores ficam restritos a informação realmente auxiliar e precisam passar verificação de legibilidade/escala.

### Navegação F6
O slice U06/U07 (§23) escreve o ciclo de entrada F6/Shift+F6 por sete regiões com disponibilidade e proteção de overlays. Tab confinado, memória exata por controle e validação assistiva permanecem pendentes; não considerar navegação completa verificada.

### Focus management
`OverlayStack` já controla LIFO e dismiss, porém a restauração/trap de foco precisa entrar no contrato e nos testes.

## 20. Gates

Esta arquitetura só é considerada implementada quando:

1. as mesmas regiões mantêm significado previsível em todos os workspaces;
2. PAINT não depende de Canvas/Layer editor comprimido no Inspector;
3. UV Editor é work surface de primeira classe;
4. ANIMATE mantém workflow procedural-first;
5. estados visuais são consistentes e não color-only;
6. disabled reason é exposto quando necessário;
7. todos os controles reutilizáveis têm teclado + semântica;
8. F6/Shift+F6 percorre regiões;
9. overlays restauram foco;
10. UI Scale/High Contrast/Reduced Motion funcionam em todas as superfícies;
11. toast não é a única representação de erro importante;
12. listas longas são virtualizadas;
13. wheel não modifica NumericField incidentalmente;
14. viewport feedback segue uma gramática única.

## 21. Decisão final

Petunia3D terá **workspaces especializados sobre uma gramática espacial comum**.

A GUI não é MODEL com painéis trocados, nem quatro aplicativos independentes.

A especialização ocorre no conteúdo de Tool Rail, Work Surface, Structure, Properties e Workspace Drawer; interação, feedback e acessibilidade permanecem compartilhados e previsíveis.

## 22. Implementação da wave U02/U03 (2026-10-09)

O split Structure/Properties e o drawer por workspace estão escritos sobre `1edfb24`, com estado efêmero de layout no bridge e abas locais no shell. MODEL conserva Parts/propriedades; PAINT separa Layers/decal e migra presets/paleta para o drawer; ANIMATE separa Creature/Motion Stack dos parâmetros; UV mantém seu conteúdo transitório até a promoção da Work Surface.

Scrolls independentes, resize/collapse, teclado das alças/abas/chips/swatches e destino F6 para a região visível foram escritos. **Compilação, testes e aceite nativo adiados pelo usuário: not run.** Não declarar conformidade assistiva, fechamento da navegação regional nem DONE a partir deste registro. Gap, arquivos, regressão inicial fail e próximo checkpoint: [Slint Rescue §20](./slint-rescue-plan.md#20-wave-u02-u03-regioes-independentes-e-drawer-2026-10-09).

## 23. Slice U06/U07 — entradas regionais e retorno de foco (2026-10-09)

**Intent / Sources:** executar a ordem regional de §18 e corrigir o roteamento de foco/teclado com overlays, sobre `36266107236163f63b4c584bc0cd28b8ae95afca`, branch `refactor/architecture-foundation`. Testes de comportamento continuam adiados pelo usuário.

**Gap:** F6 alternava somente viewport/Inspector, ignorava Shift+F6, processava F6 antes de capturar bindings e podia entregar foco a entradas de fundo em overlays. Painéis que sumiam por resize/close não tinham recuperação regional explícita. U02 disponibilizou cabeçalhos independentes, reaproveitados aqui.

**Changed:**

| Região / responsabilidade | Implementação escrita |
|---|---|
| Ordem | Header → Tool Rail → Work Surface → Structure → Properties → Drawer → Context Bar, com ciclo inverso Shift+F6. Estado inicial Work Surface; primeiro F6 avança para a próxima região disponível |
| Disponibilidade | Structure usa a pill no rail fechado ou o header no painel aberto; Properties somente no painel expandido; ambos omitidos em compact/hidden. Drawer somente quando aberto; Context Bar omitida em ANIMATE, onde ainda não tem ações |
| Entradas | `RegionFocusAnchor` com nome traduzido, papel groupbox, ring e FocusScope; sem TouchArea. Header Enter abre File; Tools Enter seleciona Select; Work Surface preserva atalhos/confirm de ferramenta. Pane headers e aba selecionada do drawer reaproveitados |
| Segurança de foco | F6 bloqueado em Home/Recovery, Settings, Command Palette, References, menu e contexto. Captura de binding tem precedência, permitindo gravar F6 e setas antes da navegação de Settings. Entradas novas, pills e pane headers são desabilitados durante overlays; global shortcut routing também respeita o bloqueio |
| Retorno | Fechar o conjunto de overlays restaura a região lembrada; se indisponível, Work Surface. Resize compact/ocultar inspector/fechar drawer/trocar para workspace sem a região recupera Work Surface. Sair de Structure fecha o peek aberto pelo teclado, preservando hover/pin |
| Estado | Apenas apresentação efêmera no shell; sem novo owner de Document/Undo, sem persistir IDs de foco no projeto, sem novo toolkit ou renderer |

Os sete IDs de apresentação (0–6) são documentados no shell, sem registry especulativo. `f6-rail-token`/`f6-in-rail` permanecem como projeções de compatibilidade. Foco de viewport fica acima da imagem e abaixo dos overlays/conteúdo, preservando ordem visual existente.

**Limites explícitos:** esta etapa implementa **entradas regionais**, não uma conclusão de todo U06/U07. A navegação nativa de Tab continua existente: confinamento por região/modal ainda não foi implementado. A memória registra a última região de navegação, não o controle exato nem todas as transições de foco por mouse. Focus trap completo, restauração de draft/caret, IME, nomes/disabled reasons globais, leitores de tela e audit Linux/Windows continuam pendentes. Bloquear o roteador global e as entradas regionais não equivale a desabilitar todos os controles de fundo de todos os overlays.

**Verification:** compilação intermediária do markup pelo build script isolado: pass, exit 0 (`/tmp/petunia-u06-slint.log`), antes dos últimos ajustes de bloqueio/ring. `CARGO_BUILD_JOBS=1 cargo check -p petunia_ui_slint --lib` nas fontes finais **pass**, exit 0 em 2m04s, sem warnings (`/tmp/petunia-u06-check-final.log`), Linux x86_64, toolchain Rust 1.98.1, features default. Esse check não compila os alvos de teste nem confirma link/startup/feature animation-workspace. `cargo fmt -p petunia_ui_slint` aplicado e revisão `git diff --check` pass. Nenhum teste executado nesta etapa.

`tests/shell_focus.rs` foi atualizado para o contrato novo de sete regiões (o antigo ciclo de duas regiões foi substituído pela decisão documentada, preservando prova física de Enter/callback). Quatro cenários preparados: ciclo direto/inverso em quatro workspaces/três temas e rotas reais; skip/resize/close; bloqueio/retorno de overlays; captura de F6/setas em bindings. O cenário de disponibilidade também cobre o Context Bar vazio em ANIMATE. Todos **not run**. O fail histórico de U01/F6 permanece histórico, sem pass retroativo. Suíte já está incluída na CI configurada; execução remota não inferida.

Fontes finais desta etapa (SHA-256):

- `crates/ui-slint/ui/app.slint`: `7ca8d2b8e090816d138c7f3dbefcbcd833cb763c9c43c04745f593a82e2f4b61`
- `crates/ui-slint/ui/shell/focus_anchor.slint`: `bd8924d10ef2ebb8e9cdd728edd67cb2d30433c592eb86201fcb93d9e9e25746`
- `crates/ui-slint/tests/shell_focus.rs`: `61e2fc9de9c05674512a4496e82c2662909c3d1d108699d8c5c6e381e87a087a`

**Risks:** check de compilação não comprova ordem nativa de Tab, foco visual, bubbling sob controles, reader, GL ou low-end. Widgets de overlay existentes ainda precisam do focus trap próprio. U06/U07 permanecem IN PROGRESS (000%), sem mudar o denominador ou alegar conformidade WCAG.

**Next checkpoint:** implementar Tab regional/trap de overlays e memória de foco por controle em slice próprio. Na fase de testes posterior, executar shell_focus, viewport_gestures, shell suites/lib/feature e validar foco/teclado/temas/escala/AT-SPI no app nativo.

## 24. Slice U06/U07 — contenção de dois modais e invocadores Header (2026-10-09)

**Intent / Sources:** implementar a entrada/contenção de foco em Settings e Command Palette e a restauração dos seus dois invocadores no Header. Fontes: §16 OverlayStack, §18 navegação/teclado, Slint Rescue §§5/13 e implementação em `overlay.rs`, `app.slint`, `shell/header.slint`, `callbacks.rs`. Baseline `cba9f38f73d7c3b8f7a78915909edb10e2cb6b84`, branch `refactor/architecture-foundation`. Os testes permanecem adiados pelo usuário.

**Gap matrix:**

| Requisito | Realidade antes deste slice | Delta |
|---|---|---|
| F6 regional | Entradas/proteção/retorno regional escritos e compilados em §23; gates comportamentais not run | Preservar; não declarar verificado |
| Abrir modal e focar conteúdo | Ausente em Settings/Palette | Entrada no TextInput de busca; ring na Palette |
| Tab/Shift+Tab no modal | Percurso sem contenção | Boundary com guards e extremos reais, usando traversal nativo |
| Tab em controles fora do modal | Roteador global podia consumir Tab | Rejeitar Tab comum quando foco está em controle; preservar o binding de ferramenta quando foco está na viewport |
| Memória de invocador | Somente região F6 | Restaurar os botões concretos Search/Settings no Header; demais invocadores seguem fallback regional |
| Capturar bindings | Bubble posterior a TextInput/Enter local | Capture no ancestor antes do controle/trap; Escape cancela captura antes de dismiss |
| Modal versus cancelamento de ferramenta | Bridge podia cancelar gesto/pontos antes de fechar modal | Topo modal recebe Escape antes da gramática de ferramenta |
| Click-away/pin | Podia fechar menu e camada superior no mesmo evento | Somente topo da pilha; pinned/non-dismissible bloqueia passagem |
| Add popover | Flag local fora da pilha | Ambos os escritores (rail e comando) usam `set_add_menu_open`, com entrada Popover em OverlayStack |

**Changed:** `shell/modal_focus_boundary.slint` envolve os cards existentes de Settings/Palette. O componente não desenha guards, não adiciona TouchArea nem enumera controles ou dados de documento. Capture registra sentido de Tab (incluindo Backtab) e intercepta Escape/F6 do modal ativo; guards agendam a transferência de foco ao primeiro/último controle real fornecido pelo chamador. Um Timer de 1 ms, ativo somente durante essa transferência, evita mudar foco dentro de focus-gained antes de o toolkit publicar foco/semântica. Isso não comprova o comportamento assistivo, que permanece pendente. Search/close na Palette; close do Header/close do footer em Settings. O foco inicial em Settings permanece na busca, acessível em ambos os sentidos. Os footers usam IconButton, tokens e labels traduzidos. Settings recebe scrim/click-away com a política já existente no bridge; drag/resize, categorias, keymap e demais conteúdos são reaproveitados. Home abre Settings pela intenção existente, evitando a flag isolada da pilha.

`OverlayId::presentation_id()` e `ShellViewModel.overlay_top_id` projetam o topo atual de OverlayStack, sem pilha concorrente no Slint. O sync aplica a projeção antes das flags. Entre Settings/Palette, somente o topo recebe entrada/trap e elevação visual; a outra camada não toma foco ao ser sincronizada. O default declarativo do ID existe somente para uso do shell sem bridge; a execução de produção sobrescreve-o com a projeção real.

O Header lembra quais dos seus dois botões abriram a superfície. Ao fechar o último overlay bloqueante, o shell restaura esse controle e limpa a memória; quando não há invocador Header, conserva o fallback regional de §23 (agora também após uma abertura anterior ao primeiro F6). Nenhuma referência de UI é persistida no projeto; Document/Undo/serialização, picking e renderer não mudam. As entradas/extremos referenciados pelo boundary precisam permanecer visíveis e habilitados; não usar um comando destrutivo como entrada padrão.

**Verification:** compilação intermediária pelo build script Slint isolado **pass** após corrigir a tentativa de usar o role `dialog`, indisponível em Slint 1.18. `CARGO_BUILD_JOBS=1 cargo check -p petunia_ui_slint --lib` intermediário **pass**, exit 0 em 4m02s, sem warnings (`/tmp/petunia-u07-check.log`). A compilação das fontes finais, após adiar a transferência dos guards, **pass**, exit 0 em 2m15s, sem warnings (`/tmp/petunia-u07-check-final.log`). Ambiente Linux x86_64, Rust 1.98.1; features default e library apenas. Não inclui link/startup, alvos de teste, feature animation-workspace nem aceite nativo. A tentativa inicial de Timer falhou pela propriedade repeat inexistente; removida, usando running ligado apenas à transferência pendente. `cargo fmt -p petunia_ui_slint` aplicado; `git diff --check` pass. Nenhum teste/gate comportamental executado nesta etapa.

Seis novas regressões headless em `tests/shell_focus.rs`: entrada/traversal nos extremos da Palette; Backtab/Enter no footer Settings; retorno a cada invocador Header com reabertura física via Enter; captura de texto/Enter/Tab/Escape sem editar query; Tab de ferramenta na viewport versus Tab nativo no Header; alternância do topo projetado entre os dois modais mantendo queries. O helper avança dois ticks de mock time para iniciar bindings e concluir o Timer, sem sleep real. Quatro regressões do bridge: precedência modal sobre pontos de ferramenta + projeção; pin/uma camada por click-away; modal non-dismissible; Add popover/drawer. Todas **not run**, incluindo compilação dos alvos de teste. Regressões anteriores não recebem pass retroativo.

Fontes desta etapa (SHA-256):

- `crates/ui-slint/ui/app.slint`: `44d1a0b79c63c0247a430d5570dd0a6a9307081dd607c55e4efd0ec7fd2769d7`
- `crates/ui-slint/ui/shell/modal_focus_boundary.slint`: `0d53f4e7183400cfd85d994a65540cb803fda4c54ae92d2715746d5c7f36f3f9`
- `crates/ui-slint/ui/shell/header.slint`: `e14fb81c53d0d0faad89aed26b813eff910ff68ed3ff9b0c314d3adc98372fd8`
- `crates/ui-slint/src/overlay.rs`: `03739c196af8104e48bd0aa4b3a19ef2a708e54d55ec107dff075c149630321a`
- `crates/ui-slint/src/lib.rs`: `6861ebd7df7021a41549cdf2170708456ddc6c30fcdb54d94a5579f64287f727`
- `crates/ui-slint/tests/shell_focus.rs`: `404c69fd01a38b2548950a180546998606c642f055af0cde5b1e1d507d527f01`

**Risks / limites:** nenhum comportamento foi executado nem observado no app nativo nesta etapa. Traversal dos guards, árvore/role acessível, bubbling, caret/IME, high contrast, escala/resize, Windows/Linux assistivo e integração final precisam de gates posteriores. Slint 1.18 não fornece `AccessibleRole::Dialog`: as superfícies usam groupbox nomeado, sem alegar role modal no leitor de tela. Este slice cobre Settings/Palette e os dois botões Header, não Home/Recovery, menus/flyouts, References, painéis pinados, memória de todo controle ou draft/caret em overlays aninhados. Tab regional completo continua pendente; o binding de ferramenta existente na Work Surface é preservado até esse contrato ser migrado. A elevação/foco entre outros tipos de overlay ainda precisa de integração ampla. U06/U07 continuam IN PROGRESS (000%), sem novos checkpoints comprovados.

**Next checkpoint:** estender entrada/trap seguro a Home/Recovery, manter References como FloatingPanel, e restaurar controles internos em overlays aninhados sem perder draft/caret. Na fase de testes autorizada depois da implementação, executar shell_focus/lib/viewport_gestures/feature e documentar aceitação nativa com teclado, pointer, tema/escala e AT-SPI/Windows. Nenhum teste é liberado automaticamente por uma compilação verde.

## 25. Slice U07 — item de menu acionável e retorno ao invocador do popover de criação (2026-10-09)

**Intent / Sources:** cumprir §16 (lembrar invocador; fechamento restaura foco ao invocador quando ainda válido) e reduzir a lacuna de teclado em menus para o popover de criação e os itens de menu de contexto. Fontes: §16 OverlayStack, §18 acessibilidade, implementação em `ui/components/controls.slint`, `ui/shell/tool_rail.slint`, `ui/app.slint`, `tests/shell_focus.rs`. Baseline `f0fcd0b`, branch `refactor/architecture-foundation`. Testes seguem adiados pelo usuário.

**Gap matrix:**

| Requisito | Realidade antes deste slice | Delta |
|---|---|---|
| Item de menu acionável por teclado | `ContextMenuItem` só tinha `TouchArea`; nenhum `FocusScope`, sem `accessible-action-default` | Item focável com Enter/Espaço, anel de foco e ação semântica |
| Memória de invocador por controle | Somente os dois botões do Header | Botão de criação do Tool Rail lembrado ao abrir o popover |
| Restauração ao invocador | Fallback regional apenas | Fechamento do popover devolve foco ao botão que o abriu, via epoch |

**Changed:** `ContextMenuItem` ganhou `item-focus` (`FocusScope` com `key-pressed` para Enter/Espaço), `forward-focus`, `accessible-action-default` e o anel de foco dos outros controles; o caminho de pointer é o mesmo `TouchArea`, agora dentro do escopo. Isso torna acionáveis por teclado os itens de menus que já são abertos por um controle focável (overflow do Context Bar, flyout do trilho) e pela ação semântica de leitor de tela; a árvore acessível continua `button` com `label`, `enabled` e `checked`.

O Tool Rail declara `add-focus-token`; o botão de criação observa esse epoch e só chama `focus()` quando o chamador o incrementa. O shell guarda `add-menu-invoker` (1 = botão do trilho; 0 = atalho global) e `rail-add-focus-token`. Ao abrir pelo botão, a memória é marcada; quando `add-menu-open` volta a falso, o shell limpa a memória e incrementa o epoch uma única vez. Quando o popover foi aberto pelo atalho (`model.primitives`) não há controle invocador e o foco permanece onde estava, preservando o comportamento anterior; se o workspace/modo mudar com o popover aberto, o botão de criação deixa de existir e também não há destino de retorno. Nenhuma referência de UI é persistida no projeto; Document/Undo/serialização, picking e renderer não mudam.

**Verification:** `cargo check -p petunia_ui_slint --lib --tests` **pass**, exit 0 em 4m19s, sem warnings (`/tmp/petunia-u07-menus-check.log`), após a última edição de fonte; repetido depois do `cargo fmt` **pass**, exit 0 em 31s, sem warnings (`/tmp/petunia-u07-menus-check2.log`). `cargo fmt -p petunia_ui_slint -- --check` passou depois de formatar `tests/workspace_drawer.rs`: o trecho reparado em §24 tinha ficado sem formatação, e o reparo não muda comportamento. `node website/scripts/verify-progress.cjs` **pass** (66 processos, 66 documentos, estados/checkpoints/evidências consistentes), `node website/tests/progress.test.cjs` **pass** (6/6) e `python3 website/scripts/verify-agent-docs.py` **pass** (66 rotas, 15 páginas de agente, links/catálogo consistentes); `git diff --check` limpo. Ambiente Linux x86_64, features default, library e alvos de teste. Duas regressões novas em `tests/shell_focus.rs`: `creation_popover_returns_focus_to_its_rail_invoker` (abre pelo botão real, fecha pela flag sincronizada e prova o retorno ao invocador reabrindo com Enter) e `context_menu_items_are_actionable_by_keyboard_and_semantic_action` (item exposto como `button` responde à ação semântica e fecha o menu). Ambas **not run**, incluindo execução dos alvos de teste. Compilação verde não é prova de comportamento; regressões anteriores não recebem pass retroativo.

Fontes desta etapa (SHA-256):

- `crates/ui-slint/ui/app.slint`: `88de4a53a7b3e9126fd78663d0a8998fb7941472a1326e06bc92fdb0d1e5a2b8`
- `crates/ui-slint/ui/shell/tool_rail.slint`: `be7363a5e415f2b133cb5d81a64b31f960294798f7274d811eb3e4f2bf10c4a7`
- `crates/ui-slint/ui/components/controls.slint`: `6f0a92dc48dcfc3d3049bb3c35941c742e8ed57bd6eb65d7d65829185baf91a5`
- `crates/ui-slint/tests/shell_focus.rs`: `6dccba6397321793ea665066c64059bda468aaafb919cb8a7be08b61ab163211`

**Risks / limites:** nenhum comportamento foi executado nem observado no app nativo. O popover de criação continua **não modal**: não entra em `regional-focus-blocked`, então F6 e os atalhos de ferramenta seguem ativos com ele aberto (os itens anunciam atalhos numéricos). Não há entrada automática de foco no popover, nem navegação por setas nem contenção de Tab; o padrão highlight-based da barra de menus não foi estendido. O menu de contexto da viewport/Outliner ainda só abre por pointer, então o ganho de teclado vale para menus cujo invocador já é focável. Memória de caret não é implementável nesta versão: `TextInput` expõe `cursor-position-byte-offset` apenas como `out property` interna, documentada somente para testes (`i-slint-compiler-1.18.0/builtin_elements.rs`, comentário "Internal, undocumented property, only exposed for tests"), sem setter; o draft de texto sobrevive por binding, mas a posição do cursor não é restaurável pelo app. Traversal de guards, árvore/role acessível, IME, high contrast, escala/resize, Windows/Linux assistivo e aceite nativo seguem pendentes. U07 continua IN PROGRESS; nenhum checkpoint é marcado como concluído.

**Next checkpoint:** decidir a entrada de foco em popovers não modais (unificar com o padrão highlight-based dos menus) e dar caminho de teclado ao flyout de grupo e ao menu de contexto; depois executar shell_focus/lib/viewport_gestures/feature na fase autorizada e registrar aceitação nativa com teclado, pointer, tema/escala e AT-SPI/Windows.

## 26. Slice U07 — popovers não modais por highlight e teclado no flyout e no menu de contexto (2026-10-09)

**Intent / Sources:** fechar a decisão de entrada de foco em popovers não modais e dar caminho de teclado ao flyout de grupo e ao menu de contexto do Outliner. Fontes: §16 (nova regra 8), §18 (teclado/roles/foco visível), implementação em `ui/components/controls.slint`, `ui/shell/tool_rail.slint`, `ui/app.slint`, `ui/inspector/sections.slint`, `tests/shell_focus.rs`. Baseline `90254cd`, branch `refactor/architecture-foundation`. Testes seguem adiados pelo usuário.

**Decisão:** popover não modal **não rouba foco**. Ele opera pelo mesmo padrão highlight-based da barra de menus: o shell é dono de um índice, setas movem o destaque, Enter invoca o item destacado, Escape fecha o topo dismissible e o foco permanece no controle invocador. Consequências pretendidas: F6 e atalhos continuam ativos (caráter não modal preservado), Tab ainda pode entrar nos itens focáveis e não há restauração artificial de foco porque ele nunca mudou de dono. Enquanto o popup está aberto, é o invocador focado que entrega setas/Enter/Espaço ao popup; Escape não é interceptado e segue para o topo da pilha.

**Gap matrix:**

| Requisito | Realidade antes deste slice | Delta |
|---|---|---|
| Operar o popover de criação por teclado | Abria/fechava por Enter, sem navegação nem ação por destaque | Down/Up movem o highlight; Enter cria o destacado; foco fica no botão |
| Escape do popover de criação | Já fechava pelo topo da pilha (§24: `OverlayStack`/`handle_escape`) | Preservado: a UI não intercepta Esc de camadas da pilha |
| Abrir o flyout de grupo por teclado | Só pointer (botão direito ou marca de canto) | Shift+F10/tecla Menu no botão focado abrem o flyout |
| Operar o flyout por teclado | Itens focáveis apenas | Down/Up movem o highlight; Enter ativa a variação; Escape fecha |
| Menu de contexto do Outliner por teclado | Só botão direito na linha | Linhas focáveis: Enter/Espaço seleciona; Shift+F10/Menu abrem o menu ancorado na linha |
| Pintura do destaque | `highlighted` existia só em `ContextMenuItem` (menus) | `ToolButton` ganhou `highlighted`; popover e flyout projetam o índice do shell |
| Foco/restauração | Invocador do popover já existia (§25) | Fluxo novo não muda o dono do foco; sem roubo nem restauração |

**Changed:** `ToolButton` ganhou `highlighted`, `popup-open`, `popup-step(int)` e `popup-invoke`; no `tool-focus`, o ramo de popup entrega setas/Enter/Espaço ao chamador e o ramo normal aceita Shift+F10 ou `Key.Menu` chamando `secondary-clicked`. Escape não é interceptado no controle: camadas da pilha fecham pelo topo em `OverlayStack` (§24) e o flyout, que é local da UI, já era fechado pelo ramo `rail-flyout` do shell. `ToolGroup` recebe `flyout-open` e os dois callbacks do flyout e os repassa ao botão interno. O Tool Rail recebe `flyout` (id aberto) e quatro callbacks (popover + flyout); as sete ToolGroups do trilho e o grupo `cut` do Context Bar passam `flyout-open: root.rail-flyout == "<id>"`.

O shell guarda `add-menu-highlight` e `rail-flyout-highlight` (in-out), a lista `add-menu-ids` na ordem dos dez botões e as funções `add-menu-step/invoke` e `rail-flyout-step/invoke` (mesma matemática do `menu-step`, incluindo o pulo para o último item em Up a partir de -1). `changed rail-flyout` e `changed add-menu-open` resetam o destaque; os dez botões do popover e os itens do flyout projetam o índice. Nenhum callback novo no root: o estado continua de apresentação do shell.

As linhas do Outliner do Inspector (`inspector/sections.slint`) e do drawer compacto (`app.slint`) ganharam um `FocusScope` cobrindo a linha, com anel de foco, Enter/Espaço selecionando e Shift+F10/Menu emitindo `scene-context-requested` ancorado na linha; o clique passa a focar a linha. Document/Undo, formatos, picking e renderer não mudam.

**Verification:** `cargo check -p petunia_ui_slint --lib --tests` **pass**, exit 0 em 5m36s (`/tmp/petunia-u07-popovers-check.log`), repetido após o reparo do Esc **pass**, exit 0 em 4m02s (`/tmp/petunia-u07-popovers-repair.log`) e reexecutado na revisão **pass**, exit 0 em 1m29s (`/tmp/petunia-u07-popovers-review.log`), sem warnings nos três, cobrindo todo o markup e os alvos de teste. `cargo fmt -p petunia_ui_slint` aplicado; `cargo fmt -p petunia_ui_slint -- --check` **pass**. Ambiente Linux x86_64, Rust 1.98.1, features default, library e alvos de teste. Três regressões novas em `tests/shell_focus.rs`: `creation_popover_highlight_navigates_keeps_focus_and_escape_dismisses` (Down/Up, Enter cria e fecha; Escape é emulado pelo caminho do bridge e fecha sem criar; o invocador reabre por Enter), `tool_group_flyout_opens_by_keyboard_and_invokes_the_highlighted_variation` (Shift+F10 abre, down até a segunda variação, Enter ativa e o foco continua no grupo) e `outliner_row_opens_its_context_menu_from_the_keyboard` (linha focada, Shift+F10 ancora o menu na linha, Enter segue selecionando e Escape fecha o topo). Todas **not run**, incluindo execução dos alvos de teste — a fase de testes segue adiada. Compilação verde não é prova de comportamento; regressões anteriores não recebem pass retroativo.

Fontes desta etapa (SHA-256):

- `crates/ui-slint/ui/components/controls.slint`: `cf7440ef7c3c0770e87e823f5a50f617d707b5ae23637136a5d5a7303767db20`
- `crates/ui-slint/ui/shell/tool_rail.slint`: `c3b4213428087a32d329b8b5535b15c9ff5be040a468a254df709cc380bd0c39`
- `crates/ui-slint/ui/app.slint`: `651dca82d38a60d1d74b060d249242e3e49cc66b04cf1c38364fcbe5a4c1d1a8`
- `crates/ui-slint/ui/inspector/sections.slint`: `6c5473400acb9aaf812d3fd3a6905a0fd8f7ba3a61f5a8ee3d9882c83fe7f3a0`
- `crates/ui-slint/tests/shell_focus.rs`: `25aa704897bc11435c58363223386633f1f3f261052b18c7aa9aae85f1d27dd4`

**Risks / limites:** nenhum comportamento foi executado nem observado no app nativo. A barra de menus não mudou; o menu de contexto ganhou apenas a abertura por teclado — setas dentro dele e navegação por setas entre linhas do Outliner continuam pendentes (os itens já são operáveis por Tab/Enter/Espaço desde §25). `add-menu-ids` duplica a ordem dos botões do popover; o refactor data-driven fica para o design system (U08). Enquanto o popup/flyout está aberto, o invocador focado consome setas/Enter/Espaço — é o mecanismo que substitui a entrada de foco; Escape segue para o shell/bridge (topo da pilha). No flyout, Enter/Espaço sem destaque mantém o flyout aberto em vez de ativar a variação mostrada (padrão da barra de menus). Linhas do Outliner focadas consomem Enter/Espaço (selecionar) enquanto mantêm o foco. Popover e flyout continuam não modais (F6/atalhos ativos); menu de contexto e barra de menus continuam bloqueando a navegação regional. Memória de caret segue não restaurável (herdado de §25). Traversal de guards, árvore/role acessível, IME, high contrast, escala/resize, Windows/Linux assistivo e aceite nativo seguem pendentes. U07 continua IN PROGRESS; nenhum checkpoint é marcado como concluído.

**Next checkpoint:** executar shell_focus/lib/viewport_gestures/feature na fase autorizada e registrar aceitação nativa com teclado, pointer, tema/escala e AT-SPI/Windows; depois avaliar setas dentro do menu de contexto (provável refactor data-driven) e navegação por setas entre linhas do Outliner.
