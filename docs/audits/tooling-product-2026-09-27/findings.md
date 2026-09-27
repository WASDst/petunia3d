# Catálogo técnico de achados

## 1. Critério de severidade

| Nível | Critério |
| :--- | :--- |
| **P0** | Pode corromper expectativa de undo/redo, exceder orçamento de memória sem controle ou tornar uma ação destrutiva não reversível |
| **P1** | Divergência arquitetural/funcional relevante, gargalo provável ou promessa de produto incorreta |
| **P2** | Dívida de manutenção, inconsistência ou custo que cresce com a expansão do produto |
| **P3** | Polimento, telemetria ou oportunidade sem impacto imediato de integridade |

Confiança **alta** significa que o fluxo foi observado diretamente no código ou
teste. Confiança **média** significa que o custo/efeito decorre da estrutura do
código, mas precisa de profiling ou sessão interativa para quantificação.

## 2. Resumo priorizado

| ID | Achado | Severidade | Confiança |
| :--- | :--- | :---: | :---: |
| TX-01 | Checkpoints pós-mutação tornam undo incorreto em ações públicas | P0 | Alta |
| TX-02 | Comando, dispatcher e bridge repetem checkpoint/invalidação | P0 | Alta |
| TX-03 | Budget de undo subestima snapshots genéricos de `Project` | P0 | Alta |
| PA-01 | Pipeline de dab amplifica cópias e recomposição integral | P1 | Alta |
| PA-02 | Produção Slint ignora spacing canônico em strokes importantes | P1 | Alta |
| PA-03 | Canvas 2D faz upscale CPU e grandes alocações por publicação | P1 | Alta |
| PA-04 | Upload de textura varre e envia o canvas inteiro | P1 | Alta |
| RV-01 | Invalidação de domínio é ampla e revisão de seleção é incompleta | P1 | Alta |
| CA-01 | Cache de modifiers pode devolver geometria obsoleta | P1 | Alta |
| VP-01 | Shading/descrições da UI excedem o renderer e divergem do caderno | P1 | Alta |
| VP-02 | Avaliação de cena escala por asset sem culling/batching efetivo | P1 | Média |
| IN-01 | Catálogos de comando e roteamento físico de teclas são duplicados | P1 | Alta |
| AX-01 | Navegação regional e semântica acessível são incompletas | P1 | Alta |
| UV-01 | Workspace UV público está obsoleto | P1 | Alta |
| UV-02 | Modelo visual UV e seleção não escalam com a complexidade | P2 | Alta/Média |
| AR-01 | Bridge/callbacks/Slint concentram responsabilidades demais | P2 | Alta |
| AR-02 | `Arc<Mutex<SlintUiBridge>>` produz contenção e falha silenciosa | P2 | Alta |
| MO-01 | ToolRegistry legado e fluxo Slint representam duas arquiteturas | P2 | Alta |
| MO-02 | Half-edge não é a representação autoral principal | P2 | Alta |
| HD-01 | APIs e caminhos de transição ampliam superfície morta/duplicada | P2 | Média/Alta |

## 3. Integridade, comandos e undo

### TX-01 — Checkpoints pós-mutação

- **Estado:** `BROKEN`
- **Severidade:** P0
- **Confiança:** alta
- **Evidência:**
  - `crates/ui-slint/src/lib.rs::mutate_paint_stack()` chama `mutate(stack)` e
    somente depois `state.checkpoint(label)`.
  - `crates/ui-slint/src/lib.rs::delete_or_dissolve_selection()` clona `before`,
    altera a malha, ignora `before` e então chama `checkpoint`.
  - `crates/ui-slint/src/lib.rs::decal_drag_end()` grava o checkpoint ao fim do
    drag, quando o transform já foi aplicado repetidamente.
  - o fill 2D chama `PaintModule::canvas_fill_scoped()` sem abrir uma sessão de
    stroke nem gravar explicitamente o estado anterior.

**Comportamento esperado:** Undo restaura exatamente o estado antes do gesto
confirmado.

**Comportamento provável:** o topo do histórico contém o mesmo estado já
mutado; o primeiro Undo não altera nada ou restaura uma etapa diferente da
pretendida. No caso de fill, a ação pode não ter entrada de histórico.

**Impacto:** quebra a confiança básica do editor. Para modelagem e pintura, uma
ação visualmente válida mas não reversível é risco de perda de trabalho, não
apenas defeito de UX.

**Correção recomendada:**

1. retirar checkpoints manuais da bridge;
2. representar cada ação como comando/gesto transacional;
3. para gestures, capturar `before` em `begin`, atualizar preview sem histórico e
   registrar uma única entrada em `commit`;
4. `cancel` restaura o snapshot/delta inicial sem gerar entrada;
5. adicionar testes que comparem conteúdo antes/depois/undo/redo, não somente
   contagens após a ação.

**Critério de aceite:** para add/remove/reorder de layer, opacity, decal drag,
fill e delete/dissolve, a sequência `do → undo → redo` deve produzir três hashes
determinísticos: `A → B → A → B`, com uma única entrada no histórico.

### TX-02 — Ownership triplo de transação e invalidação

- **Estado:** `DUPLICATED`
- **Severidade:** P0
- **Confiança:** alta
- **Evidência:**
  - `CommandDispatcher::dispatch()` clona o `Project`, executa, registra
    `checkpoint_sized`, sincroniza seleção e chama `emit_mesh_changed()` para
    comando destrutivo.
  - `UvStitchCmd`, `UvRelaxCmd` e `DissolveCmd` clonam/checkpointam e invalidam
    novamente dentro de `execute()`.
  - `SlintUiBridge::execute_core_command()` e outros wrappers fazem invalidações
    adicionais para alguns IDs.

**Impacto funcional:** uma ação pode consumir duas entradas de undo, disparar
eventos repetidos e avançar revisões mais de uma vez. Undo pode exigir dois
passos e observers podem fazer trabalho duplicado.

**Impacto de performance:** antes de um comando destrutivo, o dispatcher já
clona o projeto para rollback; ao checkpointar, outra cópia pode ser retida. O
comando interno pode adicionar uma terceira cópia.

**Correção recomendada:** `Command::execute()` deve somente validar/aplicar a
operação e retornar um `ChangeSet`/`DirtyDomains`. O dispatcher deve ser o único
dono de rollback, undo, revisões, events e dirty flag.

**Critério de aceite:** teste paramétrico de todos os comandos destrutivos exige
delta de exatamente `+1` no tamanho lógico do histórico e revisões somente nos
domínios declarados.

### TX-03 — Budget de undo superficial

- **Estado:** `BROKEN`
- **Severidade:** P0
- **Confiança:** alta
- **Evidência:** `crates/commands/src/lib.rs::estimate_bytes()` usa
  `std::mem::size_of_val(value)`. Esse valor não inclui heap de `Vec`, pixels,
  nomes, materiais, layers ou meshes. O dispatcher usa `Project::estimated_bytes`,
  mas checkpoints genéricos ainda aparecem em core e módulos.

**Impacto:** um histórico configurado para 256 MiB pode reter múltiplos projetos
com texturas grandes sem que o budget perceba o custo. Em PAINT, o erro cresce
rapidamente com número de layers e resolução.

**Correção recomendada:** proibir `UndoStack<Project>::checkpoint()` sem tamanho
profundo; expor somente `checkpoint_sized` para agregados, ou substituir
snapshots integrais por deltas/comandos reversíveis. O estimador profundo deve
contar capacidade efetivamente retida e evitar dupla contagem de recursos
compartilhados.

## 4. Revisões, cache e renderer

### RV-01 — Invalidação ampla e seleção incompleta

- **Estado:** `BROKEN`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:**
  - `AppState::emit_mesh_changed()` chama `bump_topology()` e
    `bump_positions()` incondicionalmente.
  - alterações de UV, paint, material e seleção passam por caminhos que usam
    essa função.
  - `AppState::sync_selection()` sincroniza sessão/eventos, mas não avança
    diretamente a revisão de seleção.
  - o early-return WGPU reage a hover/câmera/domínio e ao fingerprint de cena;
    uma mudança de seleção que não altera esses sinais pode depender de uma
    invalidação indireta.

**Impacto:** mudanças baratas provocam reevaluations/rebuilds caros, enquanto
uma seleção explícita pode não renovar a camada no momento correto. O modelo de
revisões deixa de comunicar a causa real.

**Correção recomendada:** definir um `DirtyDomains` bitset ou `ChangeSet` com
`TOPOLOGY`, `POSITIONS`, `NORMALS`, `UV`, `SELECTION`, `MATERIAL`, `PAINT`,
`ASSET_LIST`, `REFERENCE`. O renderer consome somente os domínios necessários.

**Teste necessário:** selecionar por clique mantendo câmera e hover constantes;
verificar que `selection_rebuilds` aumenta e `mesh_rebuilds` não.

### CA-01 — Chave incorreta do cache de modifiers

- **Estado:** `BROKEN`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:** `Asset::evaluated_mesh()` e `evaluated_mesh_cached()` usam como
  chave apenas uma combinação de `verts.len()`, `faces.len()` e
  `modifier_hash()`.

**Falha:** mover vértices ou alterar conectividade mantendo as mesmas contagens
não muda a chave. Se o cache foi preenchido, a malha avaliada anterior pode ser
devolvida.

**Situação atual:** o cache parece pouco usado no caminho principal; isso reduz
a frequência hoje, mas transforma o defeito em armadilha para a otimização que
Hair/modifiers inevitavelmente exigirão.

**Correção recomendada:** chave = `source_geometry_revision +
modifier_stack_revision + evaluation_context`. Invalidar explicitamente no
owner do asset, sem hash O(n) a cada frame.

### VP-01 — Contrato de shading divergente e copy enganosa

- **Estado:** `FUNCTIONAL_BUT_DIFFERENT` / `BROKEN` na descrição
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:**
  - core: `Wireframe`, `Solid`, `MaterialPreview`, `Rendered`; default `Solid`;
  - cap. 05: Textured default, Solid, Wireframe, Silhouette/Reference e “não
    copiar Rendered do Blender”;
  - `app.slint` descreve Material como PBR/roughness e Rendered como ray-traced ou
    iluminação realista com sombras;
  - shader observado usa base color/textura/emission e iluminação direcional
    ambiente+difusa; `roughness`, `metallic`, normal map, alpha, sombras e ray
    tracing não participam do resultado;
  - renderer fixa `smooth = false` e `unlit = false` globalmente, usando Unlit
    apenas por perfil material.

**Impacto:** o usuário escolhe um modo com expectativa errada e desenvolvedores
podem expandir uma direção explicitamente rejeitada pelo caderno.

**Correção recomendada:** primeiro corrigir labels/tooltips e reconciliar a
enumeração com a autoridade canônica. Só depois ampliar shader. Flat/Smooth deve
ser propriedade persistida por objeto; Unlit continua opção secundária.

### VP-02 — Escala de avaliação da cena

- **Estado:** `RUDIMENTARY`
- **Severidade:** P1
- **Confiança:** média até profiling
- **Evidência:** o update WGPU percorre todos os assets visíveis, chama
  `evaluated_mesh()` (que clona/evalua) e materializa vetores de triângulos e
  linhas. A camada de seleção também consulta geometria avaliada. Não foi
  encontrado frustum culling por bounds, batching por material nem instancing.

**Impacto projetado:** número de objetos, modifiers e futuros Hair Clumps elevam
custo CPU e alocações antes mesmo do draw. O gargalo tende a aparecer em cenas
com muitos assets pequenos.

**Correção recomendada:** antes de instancing complexo, implementar:

1. bounds/revisão por asset;
2. cache avaliado correto;
3. frustum culling coarse;
4. buffers/ranges atualizados somente para assets sujos;
5. counters e cenários de benchmark.

## 5. PAINT

### PA-01 — Amplificação de cópias por dab

- **Estado:** `PARTIALLY_COMPLIANT`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:** `PaintModule::composite_active_tiles()` clona o
  `PaintLayerStack` ativo para contornar empréstimos, compõe no `Asset.texture` e
  clona o canvas resultante para `Material.albedo_texture`. A função de brush
  chama composição após a aplicação; simetria invoca o caminho por dab
  espelhado.

**Impacto:** com layers/resolução maiores, uma amostra do pointer pode causar
várias cópias integrais de canvases mais composição. O custo é proporcional ao
conteúdo do stack, não à área alterada.

**Correção recomendada:**

- mutation retorna `DirtyTiles`;
- simetria agrega todos os dabs antes de compor;
- compositor lê stack sem cloná-lo, escreve em cache separado e faz uma passagem;
- material referencia o mesmo recurso lógico/handle do canvas composto, não uma
  cópia de pixels;
- upload recebe retângulos/tile rows.

### PA-02 — Spacing ignorado no frontend de produção

- **Estado:** `BROKEN`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:** o módulo oferece `BrushSettings::stroke_dabs()` e o egui legado
  o usa. O Slint interpola stroke 3D por passos fixos de coordenada lógica e o
  canvas 2D por passos fixos de pixel, chamando o brush diretamente.

**Impacto:** densidade varia com zoom, resolução e taxa de eventos; strokes podem
ficar mais escuros, serrilhados ou caros conforme o dispositivo de input.

**Correção recomendada:** um `StrokeSampler` neutro acumula distância e produz
dabs no espaço escolhido (UV/texture ou world/screen estabilizado). Slint e egui
apenas entregam samples; ambos usam o mesmo algoritmo.

### PA-03 — Upscale CPU do canvas 2D

- **Estado:** `BROKEN`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:** `SlintUiBridge::render_paint_canvas()` cria um
  `SharedPixelBuffer` nas dimensões `canvas × zoom` e replica pixels no CPU.

**Exemplo:** canvas 256×256 com zoom 16 produz 4096×4096×4 bytes, cerca de 64 MiB
por buffer, sem contar cópias intermediárias e o objeto de imagem.

**Impacto:** picos de memória, cópia e latência em pan/zoom/stroke; risco de
stutter exatamente durante interação.

**Correção recomendada:** publicar a textura em resolução nativa e usar scaling
do item de imagem/canvas. Para pixel art, selecionar filtro nearest. Cursor,
checker e guides devem ser overlays vetoriais.

### PA-04 — Hash e upload integrais da textura

- **Estado:** `BROKEN`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:** `Renderer::sync_asset_textures()` calcula FNV sobre todos os
  pixels e executa `queue.write_texture` para o canvas completo quando o hash
  muda.

**Impacto:** qualquer dab de poucos pixels varre e transfere toda a textura. O
hash evita upload quando nada mudou, mas não torna a alteração incremental.

**Correção recomendada:** revisão monotônica elimina hash O(n); `DirtyRects`
define uploads parciais com alinhamento de rows quando necessário. Consolidar
retângulos adjacentes e limitar quantidade por frame.

## 6. Input, UI e acessibilidade

### IN-01 — Comandos e atalhos com duas autoridades

- **Estado:** `DUPLICATED`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:**
  - `crates/ui-slint/src/commands.rs` define enum/descriptor próprios;
  - `crates/core/src/command.rs` define o dispatcher canônico com IDs diferentes;
  - `SlintUiBridge::route_shortcut()` trata teclas físicas, eixos, workspaces,
    números e menus antes/depois do keymap;
  - tooltips/settings exibem combinações literais.

**Exemplos de drift:** `global.save_project` vs `file.save`,
`select.mode_point` vs `select.domain_vertex`, `uv.unwrap` vs
`uv.unwrap_auto`.

**Impacto:** remap não é autoridade; a mesma ação pode estar habilitada no menu e
indisponível no teclado, e descrições ficam obsoletas.

**Correção recomendada:** core/application fornece `CommandSpec { id, text_id,
icon_id, contexts, enabled, checked }`; keymap resolve `PhysicalGesture` para ID;
UI projeta menu/palette/tooltip a partir do mesmo catálogo.

### AR-01 — Monólitos e fan-out de callbacks

- **Estado:** `RUDIMENTARY`
- **Severidade:** P2
- **Confiança:** alta
- **Evidência quantitativa:**
  - `app.slint`: 11.441 linhas;
  - `ui-slint/src/lib.rs`: 11.210 linhas;
  - `ui-slint/src/callbacks.rs`: 4.766 linhas;
  - callbacks contêm 125 chamadas a `render_viewport()`.

**Impacto:** alterações pequenas atravessam shell, bridge e callbacks; ownership
fica difícil de auditar; sincronizações redundantes surgem naturalmente.

**Correção incremental:** extrair por vertical slice, começando por Paint,
Viewport e Command Projection. Cada controller expõe view-model imutável +
intents, sem conhecer componentes Slint concretos. Não reescrever o shell.

### AR-02 — Mutex como barramento da UI

- **Estado:** `PARTIALLY_COMPLIANT`
- **Severidade:** P2
- **Confiança:** alta
- **Evidência:** callbacks têm 241 `.lock()` sobre o bridge; muitos usam
  `if let Ok(...)`, descartando a ação quando o mutex está poisoned.

**Impacto:** contenção no thread de UI, repetição, tratamento de erro invisível e
risco de callback sem feedback. O lock mascara a ausência de uma fila de intents
e owner explícito.

**Correção recomendada:** callbacks enviam intents para o owner do editor;
projeções retornam ao shell em lote. Enquanto a migração não ocorrer, centralizar
lock/error reporting e nunca falhar silenciosamente.

### AX-01 — Cobertura acessível incompleta

- **Estado:** `PARTIALLY_COMPLIANT`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência quantitativa:** 250 `TouchArea`, 80 `accessible-role`, 83
  `accessible-label`. A contagem não exige relação 1:1, mas inspeção mostra
  controles ad-hoc sem foco/ação de teclado. F6 é parseado, mas não implementa
  navegação de regiões.

**Pontos positivos:** high contrast oficial, escala de UI, reduced motion,
indicadores de eixo para color-blind e uso parcial de roles/labels.

**Gaps:**

- F6/Shift+F6 ausente;
- foco/restauração de modal sem cobertura sistemática;
- estados selected/expanded/disabled nem sempre expostos;
- controles de 28–32 px sem hit target ampliado para uso touch/tremor;
- settings declara WCAG/AccessKit mais amplamente que a implementação comprovada;
- cores literais podem ignorar alto contraste.

**Correção recomendada:** criar um teste/inventário de componentes interativos:
nome acessível, papel, estado, foco, ativação por teclado, ordem e contraste.
Validar com leitor de tela real antes de alegar conformidade.

## 7. UV e MODEL

### UV-01 — Superfície UV obsoleta

- **Estado:** `OBSOLETE`
- **Severidade:** P1
- **Confiança:** alta
- **Evidência:** `app.slint` ainda mostra pill/workspace UV e editor dedicado. O
addendum mais recente do cap. 36 e `P3D-063` determinam remover a superfície
independente durante o congelamento de PAINT, preservando UV como utilitário.

**Correção recomendada:** manter `petunia_module_uv` e o editor, mas abri-lo a
partir de PAINT em `Preparar superfície`, com retorno explícito ao contexto de
pintura. Não apagar algoritmos.

### UV-02 — Seleção e renderização UV não escalam

- **Estado:** `RUDIMENTARY`
- **Severidade:** P2
- **Confiança:** alta para arquitetura, média para limite percebido
- **Evidência:** seleção UV coexiste com flags `Face.selected`; o view-model gera
  representação SVG/string ampla da malha e aplica limites para controlar custo.

**Impacto:** conflitos entre seleção topológica e UV; custo de serialização e
reconstrução cresce com número de faces, com truncamento de feedback em malhas
grandes.

**Correção recomendada:** IDs de UV vertex/edge/island próprios, buffers retidos
e atualização por revisão. O renderer 2D deve receber arrays/paths estruturados,
não uma única string regenerada.

### MO-01 — ToolRegistry legado vs produção Slint

- **Estado:** `OBSOLETE`/`DUPLICATED`
- **Severidade:** P2
- **Confiança:** alta
- **Evidência:** `petunia_module_model::ToolRegistry` serve majoritariamente ao
  host egui/legado, enquanto Slint controla tools por bridge, strings, comandos e
  sessões próprias. Algumas ferramentas tipadas nem entram no registry default.

**Impacto:** novas features podem ser implementadas no lugar errado ou duas
vezes. Testes do registry não comprovam alcance no produto principal.

**Correção recomendada:** congelar a API legado; documentar mapa de alcance;
remover junto com `--legacy-egui`. Novas tools devem usar sessão neutra + comando.

### MO-02 — Half-edge auxiliar, não autoral

- **Estado:** `FUNCTIONAL_BUT_DIFFERENT`
- **Severidade:** P2
- **Confiança:** alta
- **Evidência:** a maior parte dos operadores edita `Vec<Vertex>`, `Vec<Face>` e
  `HashSet` de arestas; `HalfEdgeMesh` é usado sobretudo para conversão,
  validação ou casos localizados.

**Impacto:** operações locais complexas precisam reconstruir adjacency e manter
índices após deleções. Hair em si não exige half-edge, mas boolean/connect/knife
e modifiers crescentes podem aumentar o custo e o risco topológico.

**Direção recomendada:** não fazer migração big-bang. Medir operadores com maior
custo/fragilidade; introduzir um kernel/topology transaction ou adjacency cache
para eles. A representação serializada pode continuar compacta e indexada.

## 8. Código morto, duplicado e dívida de transição

### HD-01 — Superfícies sem alcance claro

- **Estado:** `DUPLICATED`/`OBSOLETE`
- **Severidade:** P2
- **Confiança:** média/alta
- **Candidatos observados:**
  - `PaintModule::paint_screen_space()` não aparece no caminho Slint principal;
  - ToolRegistry MODEL e grande parte do UI egui permanecem somente por
    compatibilidade;
  - catálogos de comandos, mensagens e atalhos têm versões core/UI;
  - `evaluated_mesh_cached()` tem pouco uso produtivo enquanto
    `evaluated_mesh()` clona repetidamente;
  - rotas UV públicas coexistem com a decisão de congelamento.

**Regra:** candidato não significa remoção imediata. Primeiro gerar relatório de
call graph/feature reachability para os dois entrypoints (`petunia3d` e
`--legacy-egui`), adicionar deprecation e só remover quando o critério de
aposentadoria do legado estiver satisfeito.

## 9. Sequência segura de remediação

1. **G0 — testes de integridade:** reproduções de TX-01/TX-02/TX-03.
2. **G1 — dono transacional:** dispatcher + `ChangeSet`; remover checkpoints dos
   comandos/bridge.
3. **G2 — revisões:** DirtyDomains e cache de modifiers correto.
4. **G3 — PAINT:** StrokeSampler único, composição/upload incremental e canvas
   nativo.
5. **G4 — command/keymap projection:** autoridade única, F6 e remoção de copies
   literais de shortcut.
6. **G5 — contrato visual:** modos de shading, labels e propriedades persistentes.
7. **G6 — UV/product:** mover superfície para PAINT e separar seleção UV.
8. **G7 — modularização Slint:** controllers por feature, sem mudar UX.
9. **G8 — spline/attachment:** pré-requisitos shape-first/Hair.

Cada gate deve terminar com testes focados, workspace check/clippy aplicável,
guards arquiteturais e uma medição antes/depois para qualquer alegação de
performance.
