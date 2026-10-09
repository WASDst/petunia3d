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
7. painéis pinados não fecham por click-away.

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
O key parser reconhece F6, porém a navegação regional completa ainda não é considerada implementada.

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

## 21. Implementação da wave U02/U03 (2026-10-09)

O split Structure/Properties e o drawer por workspace estão escritos sobre `1edfb24`, com estado efêmero de layout no bridge e abas locais no shell. MODEL conserva Parts/propriedades; PAINT separa Layers/decal e migra presets/paleta para o drawer; ANIMATE separa Creature/Motion Stack dos parâmetros; UV mantém seu conteúdo transitório até a promoção da Work Surface.

Scrolls independentes, resize/collapse, teclado das alças/abas/chips/swatches e destino F6 para a região visível foram escritos. **Compilação, testes e aceite nativo adiados pelo usuário: not run.** Não declarar conformidade assistiva, fechamento da navegação regional nem DONE a partir deste registro. Gap, arquivos, regressão inicial fail e próximo checkpoint: [Slint Rescue §20](./slint-rescue-plan.md#20-wave-u02u03--regioes-independentes-e-drawer-2026-10-09).
