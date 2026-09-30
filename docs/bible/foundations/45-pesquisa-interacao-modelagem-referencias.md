# 45 — Pesquisa: Interação de Modelagem, Referências de Mercado e Literatura

<aside>
🔬

**Capítulo de pesquisa (autoridade equivalente ao capítulo 07).** Registra a
investigação de 2026-09-29 sobre Plasticity, Cinema 4D, Modo e SketchUp, a
literatura acadêmica aplicável e o diagnóstico da implementação Slint. As
decisões normativas resultantes estão no
[ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)
e nas páginas revisadas do caderno (capítulos 01, 02, 05, 23, 36; constituição 03
e 11; P3D-015, 040, 073, 075, 076, 083, 092, 095, 131). Em conflito, essas páginas
prevalecem sobre este capítulo.

</aside>

# Resumo

- **Pergunta do produto:** seguir a filosofia do Blender deixou a modelagem
  poligonal inconsistente e a modelagem por desenho pior. Vale separar em
  workspaces "3D Draw" e "Polygon Modelling"?
- **Resposta:** sim, separar por **nível de abstração** — `DRAW` (forma: perfis,
  regiões, volumes paramétricos) e `POLY` (componente: Point, Edge, Face) — com
  documento, seleção de objeto, câmera, snapping, Inspector e **uma única
  gramática de ferramenta** compartilhados.
- **Causa raiz real:** não é o workspace, é a gramática. Hoje há cinco
  mecanismos de sessão diferentes, cada um com suas regras de confirmar e
  cancelar. Separar workspaces sem unificar a gramática só dividiria a
  inconsistência em duas.
- **Viewport:** a imagem 3D é renderizada em pixels lógicos, sem antisserrilhado
  e com linhas de 1 px; o custo por movimento do mouse cresce com a cena inteira.
  A sensação de nitidez e fluidez do Plasticity depende primeiro de corrigir isso.
- **Direção de interação aprovada:** ferramenta persistente + arrastar (alça ou
  "haul"), gesto atômico com "Última operação" ajustável, valor digitado a
  qualquer momento e alternativa clicar-mover-clicar para quem não consegue
  arrastar. O modal estilo Blender continua disponível no perfil de teclas Blender.

# 1. A pergunta e a resposta

## 1.1 Por que separar

O caderno já descreve duas camadas de uso: "Object/Shape level: caminho padrão e
mais simples" e edição de Point/Edge/Face como escape hatch avançado
([03](03-geometry-core-faces-topologia.md), [06](06-escopo-essencial.md)). Na
implementação, as duas camadas disputam o mesmo trilho de ferramentas e a mesma
gramática herdada do Blender.

| Workspace | Nível | Sensação de referência | O usuário vê |
|---|---|---|---|
| **DRAW** (Desenhar) | Forma: perfis, regiões, volumes paramétricos | Plasticity, Shapr3D, SketchUp | Formas, curvas, regiões, faces como "papel" |
| **POLY** (Polígonos) | Componente: pontos, arestas, faces | Cinema 4D, Modo | `Point · Edge · Face` e topologia |
| PAINT, UV | inalterados | — | — |

Fundamentos:

- **Interface em camadas** reduz carga para iniciantes sem esconder poder
  (Shneiderman 2003; McGrenere, Baecker & Booth 2002; Carroll & Carrithers 1984).
- **Modos grandes e visíveis não geram os erros de modo** descritos por Raskin
  (2000) e por Sellen, Kurtenbach & Buxton (1992). O problema são modos
  invisíveis, como o `Tab` Object↔Edit, que o caderno já rejeita
  ([23](23-macroarquitetura-interface.md)).
- **Precedentes:** Crocotile (Draw/Edit) e Blockbench (Edit/Paint), citados no
  [capítulo 07](07-pesquisa-referencias-lacunas.md); Cinema 4D separa splines e
  geradores da malha editável ("Make Editable"); Modo separa layouts por fase.

## 1.2 Condições para não fragmentar o fluxo

1. Um documento, uma seleção de objeto, uma câmera, um snapping, uma gramática
   de ferramenta e o mesmo Inspector (`Parts → Transform → Material → Object`).
2. DRAW desenha sobre qualquer face, de forma ou de malha. Uma região fechada
   sobre uma face de malha vira Push/Pull local (imprint + extrusão), como no
   SketchUp.
3. A passagem DRAW → POLY é explícita e reversível por Undo ("Converter em
   polígonos"). Formas permanecem paramétricas enquanto estão em DRAW.
4. POLY também desenha, no nível de componente (Poly Pen, Cut).

Alternativa considerada e não escolhida: um único MODEL com sub-modo segmentado
`Desenhar | Polígonos`. O efeito de UX é quase o mesmo, mas é menos descobrível.

# 2. Diagnóstico da implementação (29/09/2026)

Evidência coletada por leitura de código no branch
`fix/hardening-domain-viewport-ui` entre os commits `7cdff74` e `f0d7f25`.
Números de linha são aproximados, porque o branch recebeu commits durante a
coleta. Itens marcados **(verificado)** foram conferidos diretamente.

## 2.1 Âncoras Blender na documentação

- `Blender.svg` era a "Canonical UI Golden Reference" em `PROJECT_STATE.md`,
  `docs/GAUNTLET.md` e `docs/development/premium-interaction-plan.md`.
- O manual do Blender era a "referência de interação" em
  `docs/development/viewport-parity-plan.md` e nas matrizes de lacunas do MODEL.
- O keymap padrão Petunia usava as letras do Blender (`G/R/S/E/I`, P3D-092) e
  `Tab` para alternar domínio (P3D-015). O ciclo "Preview/Modal" e o texto
  "RMB/Esc Cancel" (constituição 11, P3D-131) contradiziam "RMB = menu de
  contexto" do [capítulo 36](36-ui-baseline-temas-plugin-panels.md).
- A entrada numérica era citada na constituição 11, mas nenhuma spec definia
  como ela funciona.

## 2.2 Ferramentas: cinco mecanismos de sessão

1. `ModalOp` do core (`crates/core/src/modal.rs`): Move/Rotate/Scale, Extrude,
   Extrude Individual, Inset, Bevel, Push/Pull. Clona o `Project` inteiro ao
   iniciar, inclusive pixels de textura.
2. Sessões ad-hoc no bridge Slint para Loop Cut, Slice e Knife
   (`crates/ui-slint/src/lib.rs`).
3. Clone bruto do projeto para gerar volume a partir do perfil.
4. Um comando por ponto no perfil (cada ponto é uma entrada de Undo).
5. Sessão de primitiva (`crates/core/src/primitive_session.rs`).

A entrada passa por ~20 flags booleanas num único `TouchArea`
(`crates/ui-slint/ui/app.slint`) e por uma cadeia fixa de teclas avaliada antes
do keymap.

| Sintoma | Onde |
|---|---|
| Confirmar e cancelar mudam por ferramenta: soltar confirma Extrude e o slide do Loop Cut, mas não o Slice; Space ora confirma, ora abre o micro-inspector; RMB cancela quase tudo, mas abre menu no Profile; Esc **apaga** a primitiva recém-criada | `ui-slint/src/lib.rs` (`cancel_active_operation`, roteamento de Space/Esc) |
| Extrude/Inset/Bevel/Push-Pull usam arrasto vertical incremental, não seguem o cursor e **não têm alças**; só Move/Rotate/Scale têm gizmo | `lib.rs` (`scrub_tool_modal`), `projection.rs` |
| O buffer numérico não é limpo ao iniciar, confirmar ou cancelar: E → "2" → Enter → E → "5" extruda 25 **(verificado)** | `lib.rs` (`modal_text`, `commit_tool_modal`, `cancel_tool_modal`) |
| O valor digitado num modal de ferramenta não aparece no HUD e é sobrescrito pelo próximo movimento | `lib.rs` (`fill_operation_hud`) |
| A linha de cota soma `pivot + components`, mas em Rotate os componentes são graus e em Scale são fatores | `core/src/modal.rs`, `ui-slint/src/projection.rs` |
| `is_snapped` significa "snap ligado", não "encaixou"; Move é encaixado duas vezes; raio de 0,35 em unidades de mundo; snap de face só no centroide | `core/src/state.rs`, `core/src/snap.rs` |
| Navegação travada durante Profile, Loop Cut armado, Slice e modais; Profile força câmera ortográfica | `lib.rs`, `callbacks.rs`, `module-model/src/draw_profile.rs` |
| Knife: o card testa `"knife"`, mas a ferramenta se chama `"cut"`; o botão Cancelar envia `view.cancel_active`, comando inexistente **(verificado)** | `ui/app.slint`, `core/src/command.rs` |
| E/I/Ctrl+B no domínio Object selecionam tudo silenciosamente | `lib.rs` (`begin_tool_modal`) |
| "Ajustar última operação" só existe para primitiva; comandos sem parâmetros (Spin fixo em 360°/16/Y) | `core/src/command.rs` |
| A cada movimento do mouse: picking na CPU, `view_model()` completo, ~470 chamadas `set_*` e render, mesmo sem mudança de hover | `ui-slint/src/callbacks.rs` |

## 2.3 Desenho: por que é pior que o poligonal

- Sem orbitar durante o desenho, sem valor digitado, sem inferência, sem cotas
  de comprimento e ângulo, sem regiões, sem desenho do eixo de revolução.
- Fechar o perfil usa 0,25 unidade de mundo, não pixels; o snap de 0,25 não é
  alcançável pela UI.
- Um tremor de 3 px transforma um clique em ponto Bézier; dígitos trocam o
  domínio de seleção; Delete sem ponto selecionado apaga o objeto.
- O volume é gerado por botões (Generate/Revolve) como **asset separado**, nunca
  somado ou cortado da face hospedeira. A Depth Handle do
  [capítulo 02](02-workflow-modelagem-shape-first.md) não existe.
- Existe `ProfileResource`/`SplineResource` persistente, mas sem vínculo com a
  malha gerada; o perfil não é reeditável.

## 2.4 Viewport

- Imagem 3D em pixels lógicos: borrada em telas HiDPI ou com escala de UI acima
  de 100% **(verificado)**.
- Sem MSAA; todas as linhas são `LineList` de 1 px sem antisserrilhado.
- Uma luz Lambert fixa no mundo; sem specular, matcap ou oclusão ambiente;
  mismatch sRGB em texturas e referências.
- Um clique de seleção ou cada passo de arrasto re-triangula todos os assets e
  recria buffers, porque `selection_revision` entra no fingerprint da malha
  **(verificado)**.
- Render síncrono dentro de ~60 callbacks; sem redraw sob demanda; animação de
  câmera impossível.
- Picking na CPU sem BVH; a cena é re-triangulada a cada movimento do mouse.
- O hover do gizmo **escurece** em vez de clarear **(verificado)**; o anel de
  roll é clicável num raio (~113 px) diferente do desenhado (~85 px).
- Contorno de objeto em SVG na CPU, sem oclusão, truncado em 4 000 segmentos.

# 3. Plasticity

Fontes: manual oficial, notas de versão 2026.1 e o código aberto das primeiras
versões (`github.com/nkallen/plasticity`, TypeScript + three.js).

## 3.1 Modelo de comando

- Cada comando é um fluxo interativo que envolve uma **fábrica** de geometria.
  Enquanto o usuário mexe no gizmo ou no diálogo, a fábrica é atualizada; só no
  fim ela faz commit. Gizmos, diálogos e fábricas são "recursos" que terminam ou
  cancelam juntos (`src/command/Command.ts`).
- Alguns comandos terminam na primeira interação (Move); outros permanecem para
  refinamento até OK (Fillet).
- Atalhos **dentro** do comando aparecem no diálogo: em Extrude, `F` (freestyle),
  `V` (pivô), `D` (distância), `A` (ângulo), `I` (individual), `Q/W` (união,
  diferença) (doc: Extrude).

## 3.2 Uma máquina de estados para gizmo e teclado

`src/command/AbstractGizmo.ts` define `GizmoStateMachine` com estados `none`,
`hover`, `dragging` e `command`. O mesmo código atende:

- o fluxo tradicional hover → down → drag → up; e
- o fluxo modal por teclado (atalho → move → clique), "blender modal-style".

Durante `dragging`/`command`, um `KeyboardInterpreter` acumula dígitos e o valor
digitado substitui o do mouse. A versão 2026.1 generalizou isso: `Tab` abre a
entrada de valor em Move, Rotate, Scale, Extrude e similares.

**Lição para o Petunia:** a consistência vem de um único ciclo de interação, não
de regras por ferramenta.

## 3.3 Snapping e planos de construção

- Snaps nomeados: início, fim, meio, mais próximo, interseção, centro, 1/4, 3/4,
  perpendicular, tangente; ponto de controle, região, aresta, face e centro de
  face. Uma dica textual aparece sobre o alvo.
- `Ctrl` ativa temporariamente; `Shift+X` desativa snaps indesejados.
- Planos de construção a partir de 2–4 pontos, de face + aresta, da vista, ou
  plano médio entre faces paralelas. `Space` alinha a vista ao plano;
  `Shift+Space` cria o plano **sem mover a câmera**.
- 2026.1: "2D Snapping" projeta todos os pontos no plano ativo.
- O `PointPicker` restringe o ponto a plano, reta ou arestas e mantém "straight
  snaps" (inferência de eixos a partir do último ponto).

## 3.4 Aparência do viewport (código aberto)

`src/visual_model/RenderedSceneBuilder.ts`:

- arestas como linhas "grossas" em espaço de tela (`LineSegments2`/`LineMaterial`),
  1,5 px normais, 2 px selecionadas (amarelo) e em hover (branco), desenhadas por
  cima da profundidade;
- variante separada para arestas ocultas;
- faces em matcap; hover com tinta translúcida de 10% e seleção de 20%;
- região fechada destacada a 50% no hover.

A versão 1.1 adicionou medidas na tela (comprimento, ângulo, raio), discretas o
bastante para ficarem sempre ligadas; 1.3 adicionou menus radiais; 1.4, órbita
com snap para vista ortogonal segurando `Alt`.

## 3.5 Seleção

Arrastar da direita para a esquerda seleciona o que toca; da esquerda para a
direita, o que está contido (cores diferentes). `Alt+clique` seleciona loops e
relacionados; `Ctrl+1…5` converte entre pontos, arestas, faces, sólidos e grupos.

## 3.6 Não adotar

Kernel NURBS/B-Rep (o [índice](../index.md) proíbe um "mini-Plasticity"
B-Rep) e RMB como confirmação.

# 4. Cinema 4D

Fontes: help.maxon.net (S22 a 2026).

- **Arrastar em qualquer lugar.** Extrude: "arraste para a esquerda ou direita
  dentro do viewport"; não é preciso clicar no objeto. Inset idem.
- **Bevel em dois níveis:** primeiro arrasto geral ajusta Offset; depois alças
  coloridas ajustam Offset e Depth separadamente.
- **Matrix Extrude:** modificadores no arrasto (`Shift` tamanho, `Ctrl` rotação,
  `Alt` Z).
- **Polygon Pen:** "ferramenta quase universal" para editar geometria sem
  selecionar antes. Passe o mouse até o elemento destacar, clique e pinte. Modos
  Points/Edges/Polygons; `Ctrl`-arrastar aresta extruda; `Ctrl`-clique derrete;
  clique do meio subdivide a aresta; Esc apaga a prévia.
- **Spline Pen:** clique cria ponto; arrastar puxa tangente; `Ctrl` estende a
  partir da ponta; Chamfer arredonda cantos por arrasto.
- **Geradores paramétricos** (Extrude, Lathe, Sweep, Loft) com splines como
  filhos, até "Make Editable".
- **Workplane:** modos Locked, Planar (plano do mundo pela vista), Camera, Axis
  (segue a seleção) e **Interactive** (o plano se posiciona em cada polígono sob
  o cursor). "Align Workplane to Selection".
- **Snapping:** 2D (visual), 3D (profundidade correta) e Auto (3D em
  perspectiva, 2D em ortográfica); Dynamic Guides (guias temporárias a partir de
  pontos importantes, com ângulos definidos); Quantize (passos de
  movimento, rotação e escala).

**Adotar:** ferramentas persistentes com arrasto em qualquer lugar; Polygon Pen
como modelo do Poly Pen; Workplane interativo; guias dinâmicas; `Space` para a
ferramenta anterior. **Não adotar:** a quantidade de modos de workplane e menus.

# 5. Modo

Fontes: learn.foundry.com. A Foundry encerrou o desenvolvimento em novembro de
2024 (Modo 17.1 é a última versão); o Modo é referência de design, não
concorrente.

- **Tool Pipe:** uma ferramenta é a composição de operação × centro de ação ×
  eixo × falloff × snapping × simetria. Rotate + falloff linear = Twist; Move +
  centro de elemento + falloff radial = Soft Move.
- **Centros de ação** separados do pivô da malha; "Automatic" coloca a alça no
  centro da seleção alinhada ao mundo ou ao Work Plane.
- **Hauling vs. alças:** arrastar longe das alças ajusta o valor principal
  ("haul"). Clique+arrasto reinicia; `Shift`+arrasto continua; arrasto do meio
  clona o anterior. Com "Select Through", clicar longe das alças seleciona em vez
  de "haul".
- **Sticky keys:** segurar a tecla da ferramenta ativa-a temporariamente;
  soltar volta ao estado anterior.
- **Work Plane** que se orienta para ficar quase paralelo à vista.
- Pré-seleção no hover como feedback padrão.

**Adotar:** modificadores iguais para todas as ferramentas (chips Pivot ·
Orientação · Snap · Simetria · Suave); haul; pré-seleção. **Adaptar:** "Select
Through" vira regra fixa (clique sem movimento sempre seleciona). **Não
adotar:** o pipeline exposto como painel técnico.

# 6. SketchUp e Shapr3D

O SketchUp não estava na lista original, mas é a prova de que desenho +
polígono pode ser acessível: o núcleo é de faces e arestas planas.

- **Motor de inferência** com cor e **texto** ("Ponto médio", "Na face", "No
  eixo vermelho"); magenta indica paralelo/perpendicular.
- **Travas por `Shift` (segurar) ou pelas setas (alternar):** ↑ azul, ← verde,
  → vermelho, ↓ paralelo/perpendicular. Com as setas é possível orbitar e digitar
  medidas enquanto a trava está ativa; a alternância é mais acessível que segurar
  uma tecla.
- **Caixa de medidas:** não se clica no campo; basta digitar durante ou depois
  do gesto (`3m`, `100,300` para retângulo). Redigitar depois da operação corrige
  o último resultado enquanto nenhuma outra ferramenta for usada.
- **Push/Pull** em faces planas; desenhar sobre uma face a divide.

Shapr3D ([capítulo 08](08-photo-projection-trace-project.md)): desenho em
planos e faces, regiões detectadas automaticamente e Extrude que decide somar ou
subtrair pelo sentido.

# 7. Síntese: adotar, adaptar, não adotar

| Tema | Decisão Petunia | Origem |
|---|---|---|
| Ciclo de ferramenta | Máquina única: arrastar, clicar-mover-clicar e teclado no mesmo código | Plasticity `GizmoStateMachine` |
| Confirmação | Gesto atômico; "Última operação" ajustável até outra mudança no documento | C4D, SketchUp |
| Valor principal | Arrastar em qualquer lugar (haul) ou na alça | C4D, Modo |
| Valor exato | Digitar durante o gesto; depois, no campo do card "Última operação" (sem capturar teclas fora de gesto, para não criar modo oculto com atalhos como `1`–`4`) | SketchUp, Plasticity 2026.1 |
| Modificadores | Chips Pivot · Orientação · Snap · Simetria · Suave, iguais em todas as ferramentas | Modo Tool Pipe |
| Snapping | Tipos nomeados com forma + cor + texto; inferência de direção; trava por segurar ou alternar | SketchUp, Plasticity, C4D Dynamic Guides |
| Plano de trabalho | Automático pela vista, face sob o cursor, a partir de face/pontos/vista, travado; câmera não é forçada | C4D Interactive Workplane, Plasticity CPlane |
| Desenho | Regiões fechadas detectadas e empurráveis; Push/Pull soma ou corta | Plasticity, Shapr3D, SketchUp |
| Edição de componente | Poly Pen universal; pré-seleção no hover | C4D Polygon Pen, Modo |
| Viewport | Linhas de largura constante com AA; tinta translúcida para hover/seleção; matcap; cotas | Plasticity |
| RMB | Sempre menu de contexto | [Capítulo 36](36-ui-baseline-temas-plugin-panels.md) |
| Kernel | Polígonos (sem NURBS/B-Rep) | [Índice](../index.md), [03](03-geometry-core-faces-topologia.md) |

# 8. Literatura acadêmica

## 8.1 Interação e manipulação direta

- **Sutherland (1963), *Sketchpad*.** Rubber-banding e restrições geométricas:
  a origem do desenho interativo com precisão.
- **Shneiderman (1983), *Direct Manipulation*; Hutchins, Hollan & Norman (1985).**
  Golfos de execução e de avaliação: o usuário precisa ver o efeito antes e
  durante a ação, e poder desfazê-lo. Fundamenta a prévia contínua.
- **Bier & Stone (1986), *Snap-Dragging*; Bier (1990), *Snap-dragging in three
  dimensions* (I3D, DOI 10.1145/91394.91446).** Função de gravidade + objetos de
  alinhamento criados sob demanda + transformações contínuas, com menos comandos
  que as alternativas da época. É a base teórica do motor de inferência.
- **Conner et al. (1992), *Three-Dimensional Widgets*.** Alças e gizmos como
  objetos de primeira classe que codificam restrições.
- **Beaudouin-Lafon (2000), *Instrumental Interaction*.** A ferramenta é um
  instrumento entre usuário e objeto; ativação, manipulação e reação precisam ser
  descritas de modo uniforme. Modelo conceitual do `ToolSession`.
- **Sellen, Kurtenbach & Buxton (1992), *The Prevention of Mode Errors Through
  Sensory Feedback*; Raskin (2000), *The Humane Interface*.** Modos mantidos
  fisicamente (quasimodos) e visíveis reduzem erros; estado oculto os provoca.
- **Kurtenbach & Buxton (1993), marking menus.** Menus radiais como camada
  opcional para usuários frequentes.
- **Oh & Stuerzlinger (2005).** Mover objetos deslizando sobre superfícies é mais
  rápido para novatos que alças por eixo.
- **Schmidt, Singh & Balakrishnan (2008), *Sketching and Composing Widgets for 3D
  Manipulation*.**
- **Shoemake (1992), *ARCBALL*; Chen, Mountford & Sellen (1988); Khan et al.
  (2008), *ViewCube*; Fitzmaurice et al. (2008), *Safe 3D Navigation*.**
  Rotação previsível, órbita em torno do ponto sob o cursor e gizmo de navegação.
- **MacKenzie & Ware (1993), *Lag as a Determinant of Human Performance*.**
  Atrasos de ~75 ms já degradam o apontamento. Orçamento: pré-seleção em até um
  quadro.

## 8.2 Desenho, esboço e curvas

- **Igarashi et al. (1997), *Interactive Beautification* (UIST, sistema
  Pegasus).** Um traço livre é "arrumado" inferindo restrições (paralelo,
  perpendicular, simetria, congruência) e oferecendo candidatos. O trabalho
  seguinte observa que candidatos demais atrapalham. Para o Petunia: "Arrumar
  traço" opcional com no máximo três candidatos, determinístico, sem IA.
- **Zeleznik, Herndon & Hughes (1996), *SKETCH*; Igarashi et al. (1999), *Teddy*;
  Igarashi & Hughes (2001), *A Suggestive Interface for 3D Drawing*.**
  Sugestões contextuais, poucas e claras.
- **Bae, Balakrishnan & Singh (2008), *ILoveSketch*; Schmidt et al. (2009),
  *Analytic Drawing of 3D Scaffolds*.** Esboço 3D apoiado em andaimes e planos.
- **Yan et al. (2017), *κ-Curves: Interpolation at Local Maximum Curvature*
  (SIGGRAPH, DOI 10.1145/3072959.3073692).** Curvas quadráticas por partes que
  passam pelos pontos clicados, com máximos de curvatura só nos pontos, sem
  laços nem cúspides. É a ferramenta Curvatura do Illustrator: curva sem alças
  Bézier. Candidata para a "Curva" do DRAW, discretizada com contagem explícita
  de segmentos.
- **Levien & Séquin (2009)**, splines interpolantes "mais justas"; **Yuksel,
  Schaefer & Keyser (2011)**, Catmull-Rom centrípeta (sem cúspides).
- **de Berg et al., *Computational Geometry*** (arranjos planares/DCEL).
  Detecção de regiões fechadas formadas por perfis que se cruzam no mesmo plano.

## 8.3 Malha e booleans

- **Baumgart (1975)**, winged-edge; **Mäntylä (1988)**, operadores de Euler;
  **Botsch et al. (2010)**, *Polygon Mesh Processing*.
- **Zhou, Grinspun, Zorin & Jacobson (2016)**, *Mesh Arrangements for Solid
  Geometry*; **Cherchi et al. (2022)**, *Interactive and Robust Mesh Booleans*.
  Base para o provider de Fuse/Cut do [capítulo 07](07-pesquisa-referencias-lacunas.md).

## 8.4 Viewport

- **Saito & Takahashi (1990)**, G-buffers: arestas por descontinuidade de normal
  e profundidade.
- **Gooch et al. (1998)**, iluminação para ilustração técnica; **Sloan et al.
  (2001)**, *The Lit Sphere* (matcap).
- **Bærentzen et al. (2006, 2008)**, wireframe antisserrilhado em passo único e
  remoção de linhas ocultas.
- **Rong & Tan (2006)**, *Jump Flooding*: contorno de seleção de largura
  constante.
- **Jimenez et al. (2016)**, GTAO: oclusão ambiente barata para ler volume.

## 8.5 Acessibilidade motora e cognitiva

- **Grossman & Balakrishnan (2005), *The Bubble Cursor* (CHI, DOI
  10.1145/1054972.1055012).** O alvo mais próximo é sempre o selecionável; o
  desempenho segue a lei de Fitts. Para o Petunia: picking pelo elemento mais
  próximo dentro de um raio em pixels.
- **Findlater et al. (2010), *Enhanced Area Cursors* (UIST, DOI
  10.1145/1866029.1866055).** Para 12 participantes com deficiência motora, duas
  variantes reduziram o tempo em 19%, as submovimentações corretivas em 45% e os
  erros em até 82% em relação ao cursor pontual. Para o Petunia: raio ajustável e
  "lupa motora" em regiões densas.
- **Wobbrock et al. (2011), *Ability-Based Design*; Trewin et al. (2006), *Steady
  Clicks*; Wobbrock et al. (2009), *The Angle Mouse*.** O sistema se adapta à
  habilidade da pessoa: limiar de arrasto, tempo de clique e filtragem de tremor
  configuráveis.
- **WCAG 2.2:** 2.5.7 *Dragging Movements* (toda função por arrasto precisa de
  alternativa com um ponteiro sem arrastar); 2.5.8 *Target Size* (≥ 24×24 px);
  1.4.1 *Use of Color*.
- **Grossman & Fitzmaurice (2010), *ToolClips*.** Dicas de ferramenta com vídeo
  curto melhoram a compreensão.

# 9. Indagações adicionais e respostas

1. **O problema é o workspace ou a gramática?** A gramática (seção 2.2).
2. **Esperar confirmação ou gesto atômico?** Gesto atômico: soltar registra uma
   transação; o valor continua ajustável no card "Última operação".
3. **Como conciliar arrastar em qualquer lugar com seleção por caixa?** Clique
   sem movimento sempre seleciona; arrasto executa a ferramenta; caixa e laço só
   na ferramenta Select ou por modificador. O cursor mostra o que será arrastado.
4. **Orbitar no meio de uma operação?** Sempre. A roda sempre faz zoom;
   contagens usam `+`/`−` ou `Ctrl`+roda.
5. **O que faz o RMB?** Sempre abre o menu de contexto; cancelar é `Esc`.
6. **É preciso NURBS?** Não. Segmentos de reta, arco e curva κ com contagem de
   segmentos explícita; a qualidade vem de inferência, regiões e feedback.
7. **E quem tem tremor ou pouca força?** Clicar-mover-clicar, valor digitado,
   limiar e raio ajustáveis, travas por alternância.
8. **O usuário vê o que o clique fará?** Pré-seleção universal em até um quadro.
9. **A câmera deve se mover sozinha?** Não por padrão (desorientação e conforto
   vestibular); há um comando "Olhar para o plano".
10. **Os monólitos atrapalham?** `ui-slint/ui/app.slint` e `ui-slint/src/lib.rs`
    têm mais de 11 mil linhas cada e espalham lógica de ferramenta. A máquina de
    estados precisa viver no core, com adaptador fino.
11. **O `Blender.svg` continua referência?** Não. A referência é o próprio
    sistema Petunia (capítulos 23, 24, 36) e este capítulo.
12. **Existe orçamento de desempenho?** Ocioso = zero renders; pré-seleção ≤ 16,7
    ms até 50 mil triângulos; prévia ≤ 33 ms até 20 mil; mover objeto = atualizar
    a matriz.

# 10. Arquitetura resultante (resumo)

O contrato normativo está no ADR 006 e na [constituição 11](../constitution/11-contrato-de-mesh-selection-tools-e-undo.md).

```
Idle (hover → pré-seleção + dica)
  ├─ press → Pressed ──(solta antes do limiar)──→ clique: seleciona / adiciona ponto
  │                   └─(passa do limiar)──────→ Dragging (alça | haul)
  ├─ atalho, teclado ou preferência "sem segurar" → Latched (segue o mouse; clique termina)
  └─ campo do card "Última operação" → reaplica o gesto confirmado (mesmo Undo)
Dragging/Latched: prévia viva · Tab troca campo · dígitos → buffer · eixo/plano travam
  ├─ solta / clica / Enter → Commit (1 transação) → Idle com "Última operação"
  └─ Esc → restaura exatamente
Collecting (linha, polígono, corte): clique adiciona · Backspace remove o último ·
  Enter, duplo clique ou 1º ponto fecha · Esc sai
```

Sequência de implementação: Onda 1 (viewport nítido e fluido + bugs
comprovados), Onda 2 (`ToolSession`), Onda 3 (picking, snapping, plano de
trabalho), Onda 4 (DRAW), Onda 5 (POLY), Onda 6 (visual premium, acessibilidade e
testes com usuários).

# 11. Fontes

Plasticity
- Manual: <https://doc.plasticity.xyz/>
- Construction Planes: <https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/construction-plane>
- Snap: <https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/snap>
- Selecting Objects: <https://doc.plasticity.xyz/plasticity-essentials/selecting-objects>
- Extrude: <https://doc.plasticity.xyz/solid/extrude>
- Move: <https://doc.plasticity.xyz/common/move.en>
- Notas 2026.1: <https://doc.plasticity.xyz/release-notes/whats-new-2026.1>
- Código aberto inicial: <https://github.com/nkallen/plasticity> (`src/command/Command.ts`,
  `src/command/AbstractGizmo.ts`, `src/command/KeyboardInterpreter.ts`,
  `src/command/point-picker/PointPicker.ts`, `src/editor/snaps/SnapManager.ts`,
  `src/visual_model/RenderedSceneBuilder.ts`)

Cinema 4D
- Polygon Pen: <https://help.maxon.net/c4d/s22/us/html/TOOLPOLYPEN.html>
- Extrude: <https://help.maxon.net/c4d/en-us/Content/html/TOOLEXTRUDE.html>
- Bevel: <https://help.maxon.net/c4d/s22/us/html/XBEVELTOOL.html>
- Spline Pen: <https://help.maxon.net/c4d/r25/en-us/Content/html/TOOLSPLINEPEN.html>
- Chamfer: <https://help.maxon.net/c4d/2026/en-us/Content/html/TOOLSPLINECHAMFER.html>
- Workplane Modes: <https://help.maxon.net/c4d/2026/en-us/Content/html/51902.html>
- Snap: <https://help.maxon.net/c4d/2026/en-us/Content/html/DMODELING-SNAP_SETTINGS.html>

Modo
- Tool Pipe: <https://learn.foundry.com/modo/content/help/pages/modo_interface/viewports/utility/tool_pipe.html>
- Action Centers: <https://learn.foundry.com/modo/content/help/pages/modeling/action_centers.html>
- Standard Tool Controls: <https://learn.foundry.com/modo/17.0/content/help/pages/modo_interface/standard_tool_controls.html>
- Modeling mindset: <https://learn.foundry.com/modo/content/help/pages/modeling/modo_mindset.html>
- Work Plane: <https://learn.foundry.com/modo/902/content/help/pages/modeling/workplane.html>
- Encerramento: <https://www.foundry.com/news-and-awards/foundry-winds-down-modo-development>

SketchUp
- Drawing basics: <https://help.sketchup.com/en/sketchup/introducing-drawing-basics-and-concepts>
- Moving entities: <https://help.sketchup.com/en/sketchup/moving-entities-around>

Acadêmicas com DOI verificado
- Bier (1990): <https://doi.org/10.1145/91394.91446>
- Grossman & Balakrishnan (2005): <https://doi.org/10.1145/1054972.1055012>
- Findlater et al. (2010): <https://doi.org/10.1145/1866029.1866055>
- Yan et al. (2017): <https://doi.org/10.1145/3072959.3073692>
- Igarashi — Pegasus: <https://www-ui.is.s.u-tokyo.ac.jp/~takeo/research/pegasus/pegasus.html>
- Igarashi & Hughes (2001): <https://www-ui.is.s.u-tokyo.ac.jp/~takeo/papers/chateau.pdf>
- WCAG 2.2: <https://www.w3.org/TR/WCAG22/>

As demais obras são citadas por autor, ano e título; os DOIs não foram
verificados nesta pesquisa e por isso não foram incluídos.
