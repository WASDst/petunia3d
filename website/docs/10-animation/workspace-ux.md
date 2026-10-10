# ANIMATE Workspace — UX e Comportamento

> **Status: aprovado em 2026-10-08 para a direção de UX.**
>
> O estado implementado atual cobre principalmente a experiência procedural-first (Creature + Motion + preview). Timeline de keyframes, Ghosts/Trails, Tweak Layers e Reference Frames são expansão posterior e **não devem ser documentados como concluídos**.

## 1. Objetivo

ANIMATE deve permitir criar movimento útil antes de exigir conhecimento de rigging/keyframes.

Modelo mental:

```text
Creature
→ Fit to Model
→ choose Motion
→ adjust Style / parameters
→ preview
→ keep live or bake/apply
```

Edição manual entra progressivamente.

## 2. O que já existe

Implementado/ligado na base atual:
- workspace Animate atrás de feature;
- `AnimateSession` neutra;
- `AnimateIntent`;
- criaturas/rig presets;
- Humanoid;
- Quadruped;
- MultiLeg;
- Serpent;
- Fish;
- Bird;
- Motion catalog;
- Biped Cycle;
- Gait;
- Serpentine;
- Idle Breath;
- Styles;
- parâmetros universais/advanced;
- Stepped;
- Root Motion quando suportado;
- Fit to Model;
- Show Bones;
- pose preview;
- malha deformada de preview;
- play/pause;
- playhead scrub;
- Keep Live / Apply Now;
- disabled reason para Motion incompatível;
- teclado e semântica acessível melhores que a média do shell.

Isso é a fundação e deve ser preservado.

## 3. O que NÃO está fechado como implementado

Não tratar como pronto:
- timeline de keyframes completa no Slint;
- curve/graph editor;
- Ghosts;
- motion trails;
- pose library completa;
- Tweak Layers;
- blend de múltiplos Motions como UX final;
- Reference Frames;
- secondary motion/wiggle completo;
- catálogo alvo de todos os geradores futuros.

Esses itens entram progressivamente.

## 4. Macro-layout

```text
┌──────────────────────────────────────────────────────────────────┐
│ Header                                                           │
├──────┬───────────────────────────────┬────────────────────────────┤
│ Tool │                               │ CREATURE / MOTION STACK    │
│ Rail │       3D ANIMATION VIEW       ├────────────────────────────┤
│      │                               │ PROPERTIES                 │
│      │ Transport / Context Bar       │ Motion / Rig / Advanced    │
├──────┴───────────────────────────────┴────────────────────────────┤
│ TIMELINE / MOTION STACK DRAWER                                  │
└──────────────────────────────────────────────────────────────────┘
```

A viewport continua central. Timeline não toma a tela inteira por padrão.

## 5. Tool Rail

O rail não deve virar uma lista de todos os comandos de animação.

### Fase procedural atual
Pode mostrar:
- Select/Pose;
- Motion picker shortcut/families;
- Fit/Attach quando necessário.

### Futuro posing
Ferramentas:
- Select Bone/Control;
- Move;
- Rotate;
- IK Target;
- Pose brush/ferramentas somente quando realmente implementadas.

Motions do catálogo não precisam permanecer duplicados simultaneamente no Rail e Inspector. Deve existir uma localização primária.

## 6. Structure

### Creature
Mostra criaturas/rigs existentes e qual está ativa.

### Motion Stack
Mostra Motions ligados à criatura:
- nome;
- type/generator;
- enabled;
- selected;
- live/baked state;
- incompatibility warning.

### Rig hierarchy
Só aparece quando o usuário entra em edição/posing avançado.

Não expor hierarchy completa para alguém que só quer aplicar Walk.

## 7. Properties

### Creature
- preset/type;
- Fit to Model;
- linked models;
- rig health/status.

### Motion
- Style;
- principais parâmetros;
- Speed/duration quando aplicável;
- Energy/amplitude e equivalentes;
- Stepped;
- Root Motion;
- generator-specific parameters.

### Advanced
Progressive disclosure.
O atual `Advanced` é um bom padrão e deve ser preservado.

### Rig/Control
Quando bone/control estiver selecionado:
- transform;
- constraints/IK úteis;
- role;
- limits quando implementados.

## 8. Motion picker

A implementação atual de cards/chips é boa conceitualmente.

Regras:
- Motion compatível → acionável;
- Motion incompatível → visível disabled;
- tooltip/description explica por quê;
- nunca esconder todo o catálogo e deixar o usuário sem entender possibilidades.

Ao escolher um Motion pela primeira vez:
- cria;
- seleciona;
- inicia preview conforme regra atual;
- não exige navegar por múltiplos diálogos.

## 9. Styles

Styles são presets de parâmetros, não tipos opacos de animação.

Ao clicar:
- parâmetros mudam;
- UI continua mostrando os valores resultantes;
- usuário pode ajustar depois;
- ao divergir, estado pode virar `Custom`.

## 10. Sliders e parâmetros

Preservar preview live + commit único.

Porém sliders importantes também precisam de entrada numérica conforme contrato global.

```text
Energy       ─────●────  0.72
```

Keyboard:
- arrows;
- Home/End quando semanticamente válido;
- typing via NumericField/componente combinado futuro.

## 11. Transport

O atual transport in-canvas é uma boa direção.

Conteúdo:
- play/pause;
- playhead;
- time;
- loop quando necessário;
- Keep Live;
- Apply/Bake.

Deve permanecer próximo da viewport e não escondido em painel lateral.

### Sem Motion
Transport pode permanecer visível em estado reduzido/disabled com orientação, ou ocultar controles irrelevantes preservando clareza.

## 12. Keep Live vs Apply/Bake

A diferença precisa ser entendível sem conhecer internals.

Sugestão de linguagem de produto:
- **Keep Live** → mantém Motion procedural editável;
- **Bake / Apply** → converte para animação editável/exportável conforme arquitetura.

Tooltip explica implicação.

Nunca destruir recipe sem confirmação/Undo quando a operação perder editabilidade.

## 13. Timeline

### Fase inicial
O drawer inferior representa principalmente:
- duração;
- playhead;
- blocos de Motion;
- markers simples;
- futura camada de tweak.

Não começar expondo dezenas de tracks de bone.

### Fase de posing/keyframes
Quando keyframes forem implementados:
- tracks por control/bone somente sob demanda;
- seleção hierárquica;
- keys focáveis;
- move/scale temporal;
- box select;
- snapping temporal;
- keyboard alternatives.

### Curve Editor
Superfície avançada opcional, não default.

## 14. Motion Stack e layers

A evolução deve permitir:
- base Motion procedural;
- Tweak Layer aditiva;
- pose corrections;
- secondary motion.

Modelo mental:

```text
Walk (live)
+ Hand correction
+ Head look
+ Tail secondary
```

Mas a UI só apresenta layers quando a infraestrutura existir.

Não criar controles placeholder.

## 15. Posing

O objetivo futuro é permitir pose direta no viewport sem abandonar o procedural-first.

Fluxo:
```text
select control
→ move/rotate or IK
→ preview
→ key/add tweak
```

Usar a mesma máquina de gesto do MODEL:
- hover;
- active handle;
- drag/click-move-click;
- numeric;
- constraint;
- commit/cancel;
- Undo único.

## 16. Viewport feedback

Além da gramática global:

### Bones
- hierarchy/links;
- selected;
- active;
- constrained;
- IK target;
- warning.

### Motion preview
- mesh deformada;
- skeleton opcional;
- root path futuro;
- contact/foot plant hints quando úteis.

### Invalid rig
Não apenas texto no Inspector:
- badge `RIG ISSUE`;
- inline reason;
- ação para diagnosticar/fixar quando possível.

## 17. Ghosts e Trails — futuro

Quando implementados:

### Ghosts
- número limitado;
- before/after;
- opacity configurável;
- reduced-motion não os desliga, pois são informação estática;
- performance budget.

### Trails
- trajetória do control/root;
- opção de mostrar selected only;
- densidade adaptativa.

Não declarar concluído nesta fase.

## 18. Reference Frames — futuro

Referência de animação pode reutilizar a arquitetura de Image Reference:
- image sequence/video frames quando suportado;
- overlay/side reference;
- frame offset;
- opacity;
- sync com playhead.

Deve ser ferramenta de apoio, não pipeline de vídeo embutido completo.

## 19. Decals animados

Animated Decal integra-se ao ANIMATE sem criar um segundo editor.

Quando o decal tiver variantes/flipbook:
- track simples de `variant_index`;
- stepped by default;
- preview no mesmo transport;
- bake/export quando necessário.

A autoria visual continua em PAINT; timing pertence a ANIMATE.

## 20. Empty states

### Sem criatura
```text
Choose a creature type
[Humanoid] [Quadruped] [Bird] ...
```

### Criatura sem modelo
```text
Fit this creature to a model to preview deformation
[Fit to Selected Model]
```

### Sem Motion
```text
Choose a Motion to start animating
```

### Motion incompatível
Não virar empty state genérico. Mostrar o Motion disabled com reason.

## 21. Acessibilidade

ANIMATE deve ser referência do sistema porque seus componentes já possuem boa base semântica.

Requisitos:
- Motion cards focáveis;
- disabled reason acessível;
- transport completamente operável por teclado;
- playhead ajustável sem drag;
- timeline futura navegável por teclado;
- posing com click-move-click;
- handles ampliáveis;
- bones/controls não dependem só de cor;
- playback não dispara animação visual decorativa fora da cena;
- reduced-motion afeta GUI, não a animação que o usuário está editando;
- opção de pausar previews automáticos;
- status temporal anunciado de forma não excessivamente verbosa para screen reader.

## 22. Responsive behavior

Em janela estreita:
- Structure e Properties podem alternar por tabs;
- Transport permanece acessível;
- Timeline reduz altura antes de sacrificar viewport;
- Motion picker migra de grid para lista;
- nomes não são truncados sem tooltip.

## 23. Performance

- pose preview cacheada por conteúdo;
- playhead não recria rig;
- skeleton overlay não força rebuild de mesh;
- timeline virtualizada;
- ghosts/trails futuros possuem budget explícito;
- quando playback para, voltar a render-on-demand quando possível.

## 24. Migração recomendada

1. preservar `AnimateIntent` como fronteira;
2. remover duplicação Rail vs Motion picker;
3. separar Structure de Properties;
4. transformar Creature/Motion list em Structure;
5. manter transport in-canvas;
6. criar Drawer de Timeline/Motion Stack mínimo;
7. adicionar NumericField aos parâmetros de precisão;
8. integrar focus/F6;
9. só depois implementar posing/keyframes;
10. Ghosts/Trails/Reference/Tweak Layers em fases posteriores.

## 25. Gates da fase procedural

- iniciante consegue Creature → Motion → preview sem documentação externa;
- Motion incompatível explica motivo;
- Fit to Model é reversível/Undoable;
- sliders previewam sem poluir Undo;
- commit cria histórico coerente;
- Keep Live/Bake são compreensíveis;
- keyboard-only cobre o fluxo;
- High Contrast mantém bones/controls legíveis;
- nenhuma feature futura aparece como falsa implementação.

## 26. Decisão final

ANIMATE continua **procedural-first, progressive-disclosure e viewport-first**.

O objetivo não é competir com um DCC tradicional em quantidade de editores expostos, mas permitir chegar a movimento útil rapidamente e revelar posing, keyframes, layers e curves conforme a necessidade cresce.


## Checkpoint de apresentação U02/U03 — 2026-10-09

O drawer recebe transporte/playhead procedural e Motion Stack usando as ações existentes. Creature/Motion picker ficam em Structure, parâmetros em Properties. Timeline completa de keyframes, curves, blending e expansão procedural permanecem pendentes. **Código escrito, não compilado/testado:** bateria e aceite nativo adiados pelo usuário. Registro e limites: [Slint Rescue §20](../09-ui/slint-rescue-plan.md#20-wave-u02u03--regioes-independentes-e-drawer-2026-10-09).
