# Operation Graph — sistema de nós simples e modular

> **Status:** proposta. Direção aprovada pelo usuário em 2026-10-17: sistema de nós **simples de usar**, escopo inicial **booleanas + materiais/texturas**, saída ligável a um objeto ou a outro nó, **registro modular aberto** para nós futuros. Formaliza-se em ADR-C13 (ver [registro de decisões](../00-philosophy/decision-register.md)). **Nada aqui está implementado.** Este documento **reabre**, de forma controlada, a restrição "sem node graph" de [modifiers-evaluation](modifiers-evaluation.md), [generators-redesign](generators-redesign.md) e [materials](../05-materials/architecture-decision.md), cujos itens passam a ser lidos à luz do ADR-C13 (ver §9).

## 1. Intent e fontes

- **Intent:** dar uma casa persistente e re-editável às operações paramétricas hoje apresentadas como botões one-shot (booleana, modifiers, geradores, projeção) e aos materiais/texturas, sem criar um "Blender dentro do Petunia".
- **Fontes:** [object-geometry-source](object-geometry-source.md), [modifiers-evaluation](modifiers-evaluation.md), `crates/project/src/surface_recipe.rs`, `crates/project/src/lib.rs` (`ModifierKind`, `ModifierInstance`), `crates/core/src/modifier_commands.rs`, `crates/core/src/command.rs` (`BooleanOpCmd`), `crates/mesh/src/boolean.rs`, [dossiê da GUI](../09-ui/gui-audit-dossier.md).

## 2. Estado atual (evidência de leitura)

| Peça                                | Onde                                         | Situação                                                                 |
|-------------------------------------|----------------------------------------------|--------------------------------------------------------------------------|
| Grafo tipado de superfície          | `surface_recipe.rs` (939 linhas)             | `NodeSpec`, `SocketType {Color, Value, Texture}`, arestas, `CycleDetected`, `TypeMismatch`, `SocketAlreadyWired`, avaliação CPU com cache |
| Pilha de modifiers                  | `ModifierKind::{Mirror, Symmetry}`           | só dois tipos; `ModifierOp::{Add, SetEnabled, Remove, Move, SetAxis, SetDirection, Apply}` |
| Booleana                            | `BooleanOpCmd` + `boolean_operand` em estado de sessão | destrutiva; exige "escolher operando" antes; sem receita persistente |
| Kernel de booleana                  | `petunia_mesh::boolean` (Manifold)           | valida entradas fechadas/manifold, diagnósticos estáveis                 |
| Fonte de geometria por objeto       | docs `object-geometry-source`                | Mesh · Spline · PlanarShape · Generator; geradores vivos exigem Make Editable para edição |
| Tarefas relacionadas                | G12, G13, G14, P01                           | TODO no tracker                                                          |

## 3. Princípios (simplicidade primeiro)

1. **Nenhum nó é necessário para começar.** Selecionar objeto → *Adicionar operação ▸ Boolean* cria o nó já ligado ao objeto; parâmetros aparecem no Inspector e no HUD. O canvas é opcional.
2. **Duas vistas da mesma receita:** **Pilha** (linear, no Inspector, para iniciantes) e **Grafo** (canvas na Gaveta, para ramificações). Uma receita ramificada mostra na Pilha "ramificada — abrir no grafo".
3. **Tudo é comando** (`graph.add_node`, `remove_node`, `connect`, `disconnect`, `set_param`, `move_node`, `promote_param`, `bake`): transacional, com Undo; um gesto = um Undo; posição do nó é dado de documento com coalescência de movimentos.
4. **Operável por teclado:** selecionar uma saída → Enter → lista de destinos compatíveis; conectar e desconectar sem arrastar; nomes acessíveis por nó e por porta (tipo, estado, valor); nunca entrada só por arrasto.
5. **Erros por nó, não modais**, com ação de correção ("Operando não é sólido fechado" [Mostrar] [Reparar]).
6. **Avaliação assíncrona com revisão:** resultado *stale* nunca confirma; cache por nó chaveado pelas revisões das entradas.
7. **Sem panics e dados hostis:** `Result<_, CommandError>`, floats finitos, limites de profundidade, de número de nós e de tamanho de malha; `catch_unwind` no avaliador.

## 4. Modelo

### 4.1 Domínios e tipos de porta

- **Domínios:** `Geometry` (novo) e `Surface` (o `SurfaceRecipe` atual).
- **Tipos de porta:** `Geometry`, `Object`, `Color`, `Value`, `Texture`, `Material`, `Selection`. Cada tipo tem **cor, forma e rótulo** (nunca só cor).
- Entrada aceita uma ligação, salvo **portas multi-entrada** (ex.: `Tools[N]` da booleana). Uma saída pode alimentar várias entradas.

### 4.2 Encaixe com o que existe (Reuse → Refactor)

1. **Extrair** o núcleo genérico do `surface_recipe.rs` (grafo tipado, validação, ordem topológica, detecção de ciclo, cache) para um módulo/crate neutro (nome proposto `petunia_graph`, sem Qt/GPU). `SurfaceRecipe` continua funcionando, com seus testes verdes e o formato `.petunia` retrocompatível.
2. **Fonte de geometria:** nova variante `Graph(output)`, **avaliada e não editável** até *Make Editable/Bake* (mesma regra dos geradores vivos). Material, UV automático, render e export seguem permitidos; sculpt, paint, UV manual e rig exigem Bake.
3. **Modifiers atuais:** `ModifierKind::{Mirror, Symmetry}` viram uma cadeia linear de nós (migração sem perda; arquivos antigos carregam).
4. **Booleana:** o comando destrutivo `BooleanOpCmd` permanece como **Aplicar/Bake** (avaliar + Make Editable). O estado de sessão `boolean_operand` deixa de existir.

### 4.3 Nó Boolean (escopo inicial)

| Elemento   | Definição                                                                                                  |
|------------|------------------------------------------------------------------------------------------------------------|
| Entradas   | `Base` (Geometry/Object) · `Tools` (multi-entrada, Geometry/Object)                                        |
| Parâmetros | `Operação {União, Diferença, Interseção}` · `Operandos: manter / ocultar`                                  |
| Saída      | `Geometry`                                                                                                 |
| Kernel     | `petunia_mesh::boolean` / `PreparedBooleanOperands` (Manifold), com validação e diagnósticos existentes     |
| Erros      | operando aberto ou não-manifold, resultado vazio legítimo (`BooleanSolid::Empty`), explosão de tamanho     |

### 4.4 Saídas e distribuição

- **Saída de Objeto** (sumidouro): define a geometria avaliada de um Part.
- **Distribuir** (fan-out): liga a mesma saída a vários Parts ou nós, com modo "compartilhar o resultado" ou "aplicar por objeto".
- **Pendência de decisão (ADR-C13):** semântica exata de "ligar a mais de um objeto" (geometria compartilhada × cópia por objeto) e interação com Undo e Bake.

### 4.5 Superfície: materiais e texturas

- O grafo `Surface` ganha `PBR Material` (Base Color, Roughness, Metallic, Emission, Normal, Height) e `Atribuir Material` (slot).
- Permanece a avaliação **CPU → textura → material** (`SurfaceRecipe` → `TextureResource` → `Material`). **Não** conectar nós diretamente ao shader do renderer na V1; GPU shader graph segue deferido ([decision-register](../00-philosophy/decision-register.md)).

## 5. Registro modular de nós

- **`NodeDescriptor`** estático: `id` namespaced (`petunia.boolean`, `<plugin>.nome`), rótulo e ajuda por text-id, categoria, **portas tipadas** (incluindo multi-entrada), **esquema de parâmetros** (tipo, unidade, limites, default — o mesmo que alimenta Inspector, HUD e `PetuniaNumberField`), função `evaluate` pura e determinística, **versão do nó** para migração.
- **UI gerada a partir do descriptor** (Inspector, HUD, canvas, busca "Adicionar nó"): um nó novo = um descriptor + um `evaluate`, **sem código Qt**.
- **Built-ins** em registro estático (sem registry especulativo, regra 6 do AGENTS). **Nós Lua** ficam como ponto de extensão futuro, sob E01/E02: sandbox, capability `register_node` negada por padrão, sem GL nem I/O, sem alterar documento fora de comando.
- **Nó desconhecido** (plugin ausente): carrega como **nó fantasma**, preservando parâmetros e ligações, com avaliação desativada e aviso.
- **Permissões:** UI, Lua e MCP passam pelos mesmos comandos `graph.*`, com validação, Undo e checagem de revisão.

## 6. Catálogo sugerido de nós

| Categoria  | Nós                                                                                                                               | Onda   |
|------------|-----------------------------------------------------------------------------------------------------------------------------------|--------|
| Operação   | Boolean (multi-tool), Mirror, Symmetry, Array (linear/radial), Thickness/Solidify, Subdivide, Bevel por ângulo, Simple Deform (bend/twist/taper), Merge/Join, Decimate/Remesh | N1–N3  |
| Fonte      | Primitive paramétrica (substitui o card de primitivas), Perfil → Extrude/Revolve/Sweep (G12), Shape Builder persistente (H26), Referência de objeto | N2     |
| Seleção    | Por ângulo, por material, por grupo                                                                                               | N3     |
| Superfície | Image Texture, Color, Value, Mix, Tint, Multiply, Gradient, Checker, Noise, Color Ramp, Normal-from-Height, UV Project (box/triplanar/referência), Bake | N1–N3  |
| Utilidade  | Reroute, Frame/Grupo, Nota, Switch, **Parâmetro promovido** (expõe campo do nó no Inspector), Expressão numérica tipada (parser determinístico, sem script) | N1–N2  |
| Futuro     | Instanciar em pontos, Rig/Weights, Valor × tempo (ANIMATE), nós Lua                                                               | N4+    |

## 7. UX

- Fios com **cor + padrão + rótulo** de tipo; autolayout ao inserir; "Inserir nó no fio"; busca rápida (`Shift+A`/paleta); mini-preview do resultado no viewport (ghost) no lugar de thumbnails no nó; resumo de uma linha do nó na Pilha.
- **Editor Qt:** `QGraphicsView/QGraphicsScene` dentro do Widgets, com `QAccessibleInterface` por nó, porta e fio e uma lista de nós navegável por teclado como alternativa ao canvas. Sem QML/QuickQanava: a matriz Qt registra o canvas de nós como DONE, mas é um QML de demonstração sem dependência real nem despacho de comandos (reabrir AD-07).
- **Local:** Gaveta "Nós" em DRAW/POLY/PAINT, expansível a tela cheia. Um workspace "NODES" dedicado só se o uso provar necessidade.

## 8. Roteiro e gates

| ID  | Entrega                                                                                                              | Gates                                                                                  | Status       |
|-----|----------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------|--------------|
| O01 | ADR-C13; extração do núcleo de grafo a partir de `SurfaceRecipe`; `NodeDescriptor`; comandos `graph.*`               | ciclos, tipos, retrocompatibilidade de `.petunia`, fuzz de arquivos hostis, sem panics | [-] IN PROGRESS |
| O02 | Nó Boolean + Saída de Objeto + Bake; migração de Mirror/Symmetry                                                     | paridade com `BooleanOpCmd` atual; Undo; operandos inválidos; resultado vazio          | [x] DONE     |
| O03 | Pilha no Inspector + Gaveta "Nós" + HUD gerados do descriptor                                                        | `EditorController` headless (Camada 2); QTestLib offscreen; teclado/a11y               | [x] DONE     |
| O04 | Materiais/texturas/PBR como nós; catálogo N1                                                                         | visual parity; export/bake; cache por revisão                                          | [x] DONE     |

Cada tarefa fecha com testes de regressão, evidência com SHA e `not run / pass / fail / blocked`.

**Estado verificado (2026-10-17):**

- O01: `NodeDescriptor`, grafo tipado e comandos `OpGraphCmd` com Undo estão implementados e testados (`crates/project/src/op_graph.rs`, `crates/core/src/op_graph_commands.rs`). **Pendente:** o núcleo é um módulo novo; `SurfaceRecipe` mantém seu próprio grafo (extração comum não feita) e não há fuzz de `.petunia` hostil específico do grafo.
- O02: paridade nó Boolean × `BooleanOpCmd` testada (3 operações × limpeza ligada/desligada, mesmas faces e posições). Diferença conhecida: o comando destrutivo transfere textura/UV (`boolean_texture::transfer`) e o nó não. A pilha legada `Asset.modifiers` migra ao carregar (`Project::migrate_legacy_modifiers`): mesma ordem, estado, parâmetros e UUID; parâmetro fora da faixa mantém o legado. Os comandos `AddModifier` (MCP/Lua) ainda escrevem a pilha legada, que migra no próximo carregamento.
- Fan-out: "ligar a mais de um objeto" é inerente (uma saída de operação alimenta N objetos; cada um avalia a operação no próprio frame). Não existe nó "Distribuir" separado.

## 9. Efeito sobre documentos existentes

Os trechos abaixo continuam válidos como **restrição de escopo da V1 do domínio** (sem grafo genérico de shader/GPU, sem plugin framework de operadores), mas deixam de proibir um grafo de operações com registro modular conforme este documento: `modifiers-evaluation.md` (Modifier Stack pequena, recipe ordenada), `generators-redesign.md`, `generators-implementation.md`, `object-geometry-source.md` (sem store/graph genérico), `05-materials` (SurfaceRecipe ≠ shader graph), `06-paint` (inserção sobre futuro "Sistema de Nós") e `qt-library-adoption.md` (AD-07). A reconciliação textual desses documentos é pendência do ADR-C13.
