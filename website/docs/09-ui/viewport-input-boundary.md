# Viewport Input Boundary

> **Status: aprovado em 2026-10-08 e em migração incremental.**
>
> Este capítulo define a fronteira entre input bruto do Slint, reconhecimento de gesto, intents de viewport e tool/domain sessions. Ele complementa [Slint Rescue](./slint-rescue-plan.md) e [Workspaces, Feedback Visual e Acessibilidade](./workspaces-feedback-accessibility.md).

## 1. Problema observado

A viewport Slint acumulou responsabilidades demais em um único `TouchArea`:

- hover;
- click;
- drag threshold;
- orbit/pan/zoom;
- box select;
- lasso;
- gizmo drag;
- transform drag;
- modal tools;
- tool grammar;
- Paint strokes;
- Loop Cut scrub;
- Draw depth handle;
- click-move-click/latched gesture;
- cursor placement;
- context menu.

Isso funcionou como integração pragmática, mas tornou `app.slint` responsável por reconhecer gesto **e** conhecer detalhes demais sobre o que cada ferramenta faz.

A refatoração não deve mover domínio para Slint nem mover eventos brutos diretamente para Geometry.

## 2. Boundary aprovada

```text
OS / Slint PointerEvent
        ↓
ViewportInputRouter
(raw input → gesture recognition)
        ↓
semantic callbacks / UiIntent
        ↓
SlintUiBridge / Application
        ↓
ToolSession / Selection / Camera / Paint / Geometry
```

Cada nível possui responsabilidade específica.

## 3. Nível 1 — raw input

Slint conhece:
- pointer position;
- button;
- wheel;
- modifiers;
- pressed/released/cancel;
- hover;
- tamanho lógico da viewport.

Raw input **não é command de domínio**.

Evitar interfaces como:

```text
Geometry::pointer_down(...)
Mesh::mouse_move(...)
```

## 4. Nível 2 — gesture recognition

`ViewportInputRouter` pode manter estado efêmero:

- press origin;
- last pointer position;
- moved-since-down;
- pending box select;
- box select;
- pending lasso;
- lasso path;
- orbit;
- pan;
- pending transform;
- gizmo drag;
- painting;
- tool gesture;
- precision modifier;
- snap modifier.

Esse estado:
- não é serializado;
- não entra no Project;
- não constitui fonte de verdade de uma ferramenta;
- pode desaparecer ao cancelar/fechar a UI.

O roteador decide **qual gesto está acontecendo**, não qual alteração geométrica deve ocorrer.

## 5. Nível 3 — semantic viewport intents

Depois do reconhecimento, a UI emite ações com significado.

Categorias atuais:

### Navigation
- Orbit;
- Pan;
- Zoom.

Essas ações convergem para:

```rust
UiIntent::ViewportGesture(ViewportGesture::...)
```

### Hover / Selection
- viewport hover;
- hover clear;
- select;
- box select;
- lasso select.

### Cursor
- place 3D cursor.

### Transform
- begin;
- update;
- end.

### Gizmo
- hover;
- drag begin;
- drag end.

### Paint
- stroke begin;
- stroke update;
- stroke end.

### Tool grammar
- Press;
- Move;
- Release;
- Cancel.

No Rust, tool grammar usa `ViewportPointerPhase` em vez de números mágicos. A API Slint gerada ainda envia `int` temporariamente; a conversão acontece na boundary.

### Modal/parameter gestures
- tool modal hover;
- scrub;
- apply;
- Loop Cut scrub/apply;
- profile depth.

## 6. Nível 4 — domain/application

Somente Rust/Application decide:

- picking real;
- seleção autoral;
- target de gizmo;
- edição de mesh;
- criação de ToolSession;
- snapping geométrico;
- commit/cancel;
- Undo;
- PaintDocument;
- UV;
- modifiers;
- project mutation.

Regra absoluta:

```text
ViewportInputRouter
≠
Geometry controller
```

## 7. Coordenadas

A boundary precisa declarar qual espaço cada callback usa.

### Normalized viewport coordinates
`0..1`:

- selection;
- box select;
- cursor placement;
- hover quando o consumidor faz picking normalizado.

### Logical viewport pixels
- tool grammar;
- transform manipulation;
- gizmo hit;
- Paint brush;
- modal scrub;
- display-space handles.

### Physical pixels
Não devem vazar para ferramenta/domain.

São usados apenas na boundary de renderer/HiDPI quando necessário.

## 8. Modificadores

Shift/Ctrl/Alt não devem significar diretamente uma ação de domínio dentro de Geometry.

Fluxo:

```text
physical modifier
→ keymap / interaction grammar
→ semantic meaning
```

Exemplos:
- Shift pode significar precision ou extend dependendo do gesto;
- Ctrl pode significar snap, subtract ou uma ação configurada;
- Alt pode significar orbit em keymap Maya ou alternate tool behavior.

A Application/tool session interpreta o significado de ferramenta quando necessário.

## 9. Drag threshold

O threshold pertence ao recognition layer.

Ele diferencia:
- click;
- click + drag;
- box select;
- transform drag;
- lasso start.

A preferência de acessibilidade `drag-threshold-px` deve futuramente substituir os literais 3/4 px restantes no router.

Isso é requisito da próxima limpeza.

## 10. Click-Move-Click

`click-move-click` continua recurso formal de acessibilidade.

Não deve ser implementado como fork completo das ferramentas.

A mesma ToolSession precisa aceitar:
- drag contínuo;
- gesture latched / click-move-click.

O input layer muda a forma de alimentar a sessão; o domínio permanece igual.

## 11. Cancel

`Esc` é o cancel canônico de ToolSession.

RMB:
- sem gesto → context menu;
- Shift+RMB conforme contrato atual pode posicionar cursor;
- durante gesto não deve substituir arbitrariamente Esc como cancel.

Pointer cancel do sistema:
- converte para `ViewportPointerPhase::Cancel`;
- limpa estado efêmero;
- manda cancel para sessão quando necessário.

## 12. Navigation invariant

Navegação de câmera deve permanecer independente da ferramenta.

Regra já adotada:
- wheel → zoom;
- MMB → orbit;
- Shift+MMB → pan;
- Maya keymap pode mapear Alt+LMB para orbit.

Ferramenta armada não sequestra wheel comum.

Ctrl+wheel pode ser um canal contextual explicitamente contratado.

## 13. Orbit

Orbit possui semântica adicional:
- seleção ativa pode definir pivot;
- Cursor 3D pode definir pivot;
- caso contrário mantém o target atual.

Por isso todos os caminhos de Orbit agora convergem por:

```text
UiIntent::ViewportGesture(
    ViewportGesture::Orbit { ... }
)
```

e reutilizam a implementação de `orbit_viewport`.

Isso elimina a divergência anterior onde um callback direto possuía comportamento de pivot diferente do `UiIntent`.

## 14. Tool pointer phases

Contrato Rust:

```rust
ViewportPointerPhase::Press
ViewportPointerPhase::Move
ViewportPointerPhase::Release
ViewportPointerPhase::Cancel
```

A interface Slint continua temporariamente emitindo:
- 0;
- 1;
- 2;
- 3.

Esses números são compatibilidade da boundary, não semântica permitida dentro da lógica da ferramenta.

Migração futura pode substituir o callback gerado por um enum Slint equivalente quando isso não aumentar a complexidade da API.

## 15. ViewportInputRouter

Arquivo:

`ui/viewport/input_router.slint`

Responsabilidade atual:
- extrair o antigo `viewport-touch` do `app.slint`;
- manter gesture state transitório;
- renderizar box/lasso diretamente associados ao recognition state;
- emitir os mesmos callbacks existentes.

Não possui:
- Project;
- Mesh;
- SelectionState;
- ToolSession;
- Commands;
- Undo.

## 16. Estado que ainda permanece fora do router

Ainda fica no `PetuniaSlintShell` por enquanto:
- estado apresentado do gizmo;
- tool modal presentation;
- HUDs;
- cursor 3D overlay;
- handles interativos externos;
- viewport context menu;
- View Bar / Context Bar state;
- domínio selecionado;
- active tool/workspace.

Isso evita uma extração grande demais.

## 17. Próxima migração

### Passo A — concluído
Os thresholds de promoção de click → drag/box/transform/lasso agora recebem `drag-threshold-px` da preferência de acessibilidade. O espaçamento interno de amostragem do caminho do lasso permanece separado, pois não representa intenção de drag.

### Passo B — concluído para a gramática global da viewport
Ações globais de pointer agora usam `[pointer]` no keymap: `precision`, `snap`, `extend`, `subtract`, `pan`, `cursor_place`, `adjust`, `loop` e `orbit_left`.

O perfil Maya declara `orbit_left = "Alt"`; o Slint não testa mais `active-keymap-id == "maya"`.

A regressão de scroll foi corrigida: `scroll-event` calcula `pointer.adjust` usando os modificadores **do próprio evento de scroll**, sem acessar a variável local de `pointer-event`. `Ctrl` não é mais fixo para o ajuste contextual; um perfil pode remapear a ação para `Alt`, `Shift`, combinações ou nenhuma tecla.

Da mesma forma, a exceção da gramática de ferramentas que testava `event.modifiers.alt` agora consulta a ação semântica `pointer.loop`. `pointer.orbit_left` tem precedência sobre a sessão de ferramenta com gesto latched para preservar Alt+LMB no perfil Maya. Um cancelamento nativo não deve abrir menu contextual: somente RMB sem gesto tem essa responsabilidade.

Modificadores crus ainda atravessam a boundary para ToolSession e PAINT quando a interpretação pertence à própria ferramenta. A rodada específica de PAINT deve substituir Shift/Ctrl semânticos de clone/decal/straight stroke por ações de pointer próprias antes de considerar esse domínio totalmente migrado.

### Passo C — escrito, testes pendentes (slice U04)
DTOs/intents Rust ligados à produção em §22; compilação default pass, testes not run. Escopo:
- select;
- hover;
- box/lasso;
- pointer tool session.

### Passo D — em execução

`bridge/viewport.rs` já concentra:
- navigation + resize;
- selection / cursor / contextual wheel;
- hover;
- transforms + gizmo drag;
- tool pointer;
- Split View.

`callbacks.rs` caiu de aproximadamente 6.069 para aproximadamente 5.487 linhas. Outros grupos devem migrar apenas quando houver fronteira temática clara.

### Passo E
Somente depois considerar mover picking orchestration.

Picking geométrico continua Rust.

## 18. Testes obrigatórios

O gate de viewport deve cobrir:

- click selection;
- click sem drag não vira box select;
- drag acima do threshold vira box;
- lasso;
- Orbit;
- Pan;
- wheel Zoom;
- Ctrl+wheel;
- Maya Alt+LMB;
- transform instant;
- transform por drag;
- gizmo drag;
- Paint begin/update/end;
- tool grammar Press/Move/Release/Cancel;
- Escape;
- click-move-click;
- context menu;
- cursor placement;
- High DPI coordinate mapping.

Os testes existentes em `viewport_gestures.rs` continuam sendo a base de regressão. Foram adicionados cenários declarativos para `pointer.adjust` remapeado por evento de scroll, Petunia versus Maya Alt+LMB, `pointer.pan` remapeado, `precision`, `snap`, `extend` e colocação de Cursor 3D. **Estes cenários foram registrados no código; o resultado de execução do gate CI deve ser consultado antes de declará-los aprovados.**

## 19. Acessibilidade

Input alternativo não é camada paralela.

Teclado, click-move-click, hit targets maiores e drag threshold configurável devem produzir os mesmos semantic intents que mouse/pen.

Isso evita manter:
- "modo acessível" separado;
- tool implementation duplicada;
- comportamento divergente.

## 20. Decisão final

A viewport terá **um recognition layer de UI fino** e **uma camada semântica Rust forte**.

Slint pode decidir se o usuário clicou, arrastou, orbitou ou iniciou um lasso.

Slint não decide como modificar mesh, como resolver snapping geométrico, como aplicar uma ToolSession ou como registrar Undo.

Essa boundary é a condição para continuar decompondo a viewport sem reconstruir o acoplamento em outro arquivo.


## 21. Progresso da modularização Rust

A boundary deixou de existir apenas no markup:

- `src/bridge/viewport.rs` registra navegação, resize, seleção, Cursor 3D, hover, transform/gizmo e contexto;
- `src/bridge/model.rs` recebe gestos de Profile/DRAW;
- `src/bridge/paint.rs` recebe strokes 3D do PAINT;
- `render_viewport`, `viewport_render_state`, `apply_viewport_gesture`, resize e orbit foram removidos do `lib.rs` raiz e vivem no módulo de viewport.

Os algoritmos grandes de picking/seleção permanecem no Rust e serão extraídos apenas quando houver uma boundary de query/service clara; não serão movidos mecanicamente só para reduzir LOC.

## 22. Slice U04 — DTOs e execução no bridge (2026-10-09)

**Intent / Sources:** tornar os callbacks Slint de select/hover/box/lasso/tool pointer adapters de DTOs, conforme §3–8 deste capítulo e Slint Rescue §10–11. Baseline `fbe5aa7` na branch `refactor/architecture-foundation`; U02/U03 já escritos, aceite comportamental ainda adiado pelo usuário.

**Gap:** callbacks de box/lasso ainda executavam seleção e montavam status lendo malha/sessão diretamente; tool pointer usava fase inteira para decidir refresh; layout/sections permanecia espalhado em `callbacks.rs`/`lib.rs`. Picking, parser lasso, keymap e ToolSession existentes são reutilizáveis.

**Changed:**

| Boundary | Contrato implementado |
|---|---|
| NormalizedViewportPoint | Origem superior esquerda; finitude de coordenadas/projeção NDC; capture fora de 0..1 preservado para box/point, sem clamp incidental |
| LogicalViewportPoint | Pixels lógicos, valores finitos, coordenadas de capture negativas permitidas; sem DPI físico em ferramentas |
| ViewportSelectionIntent | Point / Box / Lasso; `UiIntent::ViewportSelection` despacha no bridge; Replace/Add/Subtract preserva precedência de subtract quando ambos flags estavam ativos |
| ViewportLassoPolygon | Reusa parser único: 131.072 bytes / 4.096 pontos, pelo menos três pontos, sem NaN/Inf; clamp à borda e NDC existentes preservados |
| ViewportHoverIntent | At / Clear; escolha hover de Loop Cut versus componente no bridge; throttle e footprint PAINT existentes preservados |
| ViewportToolPointerIntent | Press / Move / Release / Cancel em px lógicos; callback rejeita fase desconhecida; Move é comparado por enum, sem `phase == 1` |
| PhysicalPointerModifiers | Shift/Ctrl/Alt descritos como teclas físicas; keymap Application continua resolvendo precision/snap/extend/alternate antes de ToolSession |
| shell_layout.rs | Conexões e métodos de layout/sections movidos, mantendo nomes públicos, persistência e regras existentes; projeção de split continua limitada a três propriedades |
| viewport_actions.rs | Execução da seleção/status/hover e gramática typed; preserva consultas, picking, ToolSession, commit/cancel/Undo existentes |

O callback público Slint permanece compatível (int/float/bool), mas é convertido antes de executar. `tool_pointer`/`tool_pointer_ex` antigos também permanecem disponíveis; o adapter Rust legado conserva o fallback histórico de fase desconhecida para Cancel, enquanto a entrada Slint agora recusa códigos desconhecidos. Não criar cancel tácito a partir de entrada inválida.

Métodos de layout/sections foram movidos por equivalência; esta extração não altera a política histórica de persistência/dirty das seções. DTOs não são serializados no Project. Core/Geometry não recebem dependência de UI/GPU; migração de ownership para Application continua processo separado.

Contagem de fontes: `lib.rs` 16.994 → 16.779 linhas; `callbacks.rs` 5.540 → 5.401. Novos módulos pequenos por responsabilidade: shell_layout, viewport_actions e viewport_intents. Os callbacks de seleção deixam de ler malha/sessão para executar seleção ou compor status.

**Verification:** formatação aplicada (`cargo fmt -p petunia_ui_slint`), revisão de source e `git diff --check` pass; `CARGO_BUILD_JOBS=1 cargo check -p petunia_ui_slint --lib` **pass**, exit 0, 3m20s, sem warnings. Log `/tmp/petunia-u04-check.log`; Linux x86_64, Rust 1.98.1, configuração default, library somente; o comando não compila nem executa testes e não verifica a feature animation-workspace. Regressões escritas (três de validação/unidades/fases + uma de integração da seleção com queries reais), **not run**. Bateria lib/integração/Clippy/ui-lint/feature animation-workspace, runtime/native/reader/GL/Windows/low-end: **not run** por adiamento do usuário. O check aceita a UI U02/U03 corrigida e a boundary U04, sem comprovar comportamento, link/startup da aplicação ou aceite assistivo.

**Risks:** esta é decomposição incremental, sem alegar conclusão de todo U04. Ainda existem outros métodos de domínio/aplicação em `lib.rs`, callbacks gerais e entrada PAINT com modificadores crus (U05). DTO escrito não comprova runtime; testes de seleção/Undo/keymap remapeado/cancel precisam confirmar equivalência.

**Next checkpoint:** check default registrado; continuar os slices autorizados de UI/bridge, especialmente F6 regional/foco e promoção das work surfaces. Executar as regressões somente na fase de testes posterior. U04 permanece IN PROGRESS (000%) até existir evidência suficiente dos critérios/gates completos.


Fontes de DTOs/módulos no check (SHA-256):

```text
349a39b2bbc519dd7294be05fea44490d1275674e5f43a8ff8bb6510a6830a5f  src/viewport_intents.rs
1c057e847564bffe232dd3a8a3f29da0caa4aa3f207c0fc254cffc4e69a0bc26  src/bridge/viewport_actions.rs
cd26e41bd87b24f1d235c0abe77d4346e04a90d219c0ea29d362253d38e96a36  src/bridge/shell_layout.rs
```
