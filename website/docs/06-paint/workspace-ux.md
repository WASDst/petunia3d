# PAINT Workspace — UX e Comportamento

> **Status: aprovado em 2026-10-08.**
>
> Este documento especializa o contrato global de [Workspaces, Feedback Visual e Acessibilidade](../09-ui/workspaces-feedback-accessibility.md) para o workspace PAINT. A arquitetura de dados continua definida em [Paint Architecture](./architecture-decision.md) e o sistema de decal em [Decals e Animated Decals](./decals-animated-decals.md).

## 1. Objetivo

PAINT deve permitir que um usuário iniciante comece a pintar imediatamente, mas não limitar usuários avançados a um único raster simples.

A progressão é:

```text
escolher alvo/canal
→ escolher layer
→ escolher ferramenta
→ pintar
→ ajustar layer/brush
→ revisar 3D/2D
```

A arquitetura interna pode ser rica sem tornar a primeira experiência semelhante a Photoshop/Substance.

## 2. O que já existe e deve ser reaproveitado

A implementação Slint atual já expõe ou conecta:
- Select;
- Brush / Pixel;
- Eraser;
- Eyedropper;
- Fill;
- Line / Rectangle;
- gradients;
- Airbrush / Spray;
- Smudge / Blur;
- Dodge / Burn;
- Clone;
- brush size / opacity / hardness;
- flow / spacing / smoothing;
- jitter / scatter;
- tip shape / angle / roundness;
- blend modes;
- presets;
- symmetry X/Y/Z;
- texture vs vertex target;
- selection mask;
- Layers;
- layer groups;
- decal layers;
- visibility / lock / move / merge / delete;
- layer opacity;
- effects;
- Canvas 2D;
- UV overlay;
- pixel grid;
- zoom/pan;
- 2D selection;
- Picture-in-Picture;
- projection/fill scopes.

Portanto o problema principal é **hierarquia e densidade da interface**, não ausência geral de funcionalidade.

## 3. Macro-layout

```text
┌──────────────────────────────────────────────────────────────────┐
│ Header                                                           │
├──────┬───────────────────────────────┬────────────────────────────┤
│ Tool │                               │ LAYERS                     │
│ Rail │        PAINT SURFACE          │                            │
│      │      3D / 2D / SPLIT          ├────────────────────────────┤
│      │                               │ PROPERTIES                 │
│      │ Context Bar                   │ Brush / Layer / Projection │
├──────┴───────────────────────────────┴────────────────────────────┤
│ Assets / Brush Presets / Palette                                 │
└──────────────────────────────────────────────────────────────────┘
```

## 4. Tool Rail

Ordem sugerida por frequência e modelo mental:

### Selection
- Select.

### Paint
- Brush;
- Pixel Brush como variação;
- Eraser.

### Sampling / Fill
- Eyedropper;
- Fill.

### Shapes
- Line;
- Rectangle;
- futuras Ellipse/Shape tools se o motor consolidar suporte.

### Gradient
- Linear;
- Radial.

### Effects brushes
Flyout:
- Airbrush;
- Spray;
- Smudge;
- Blur;
- Dodge;
- Burn;
- Clone.

Ferramentas menos frequentes ficam em ToolGroup/flyout para reduzir altura e carga visual.

O rail não deve duplicar parâmetros do brush.

## 5. Work Surface

PAINT possui duas superfícies legítimas:
- 3D Surface Paint;
- 2D Texture Canvas.

### Modos iniciais

#### 3D
Default quando o usuário está pintando no modelo.

#### 2D
Canvas ocupa a superfície principal.

#### Split
3D e 2D simultâneos com divisor redimensionável.

#### 2D + 3D PiP
Preservar o PiP já existente para tarefas onde a textura precisa dominar.

O PiP:
- mostra 3D vivo;
- tem affordance clara de retorno;
- pode ser movido no futuro;
- nunca intercepta brush strokes fora de seu bounds;
- permanece acessível por teclado.

## 6. Canvas 2D

Preservar do componente atual:
- fit da textura;
- zoom;
- pan;
- pixelated rendering;
- UV overlay;
- pixel grid;
- selection rectangle;
- brush cursor;
- stroke callbacks.

Melhorias contratuais:
- zoom contínuo/steps previsíveis;
- `Frame Texture` para reenquadrar;
- pan com MMB e alternativa por teclado;
- status de zoom visível;
- overlay UV independente do modo de seleção;
- indicador de pixel grid somente quando útil;
- cursor nunca some por estar sobre fundo de cor semelhante.

## 7. Brush Cursor

O cursor atual é uma boa base e deve continuar mostrando:
- diâmetro;
- hardness;
- força/opacidade aproximada;
- erase;
- centro;
- achatamento/orientação sobre superfície inclinada.

Estados adicionais:
- fora do alvo pintável → cursor neutral/invalid;
- layer locked → cursor disabled + motivo;
- sem UV para Texture Paint → cursor blocked + mensagem persistente;
- seleção mascarando área → footprint continua visível, mas região bloqueada deve ser legível;
- clone source ausente → estado armed específico.

O cursor não pode depender apenas de cor.

## 8. Structure = Layers

PAINT usa Layers como estrutura primária, não Outliner.

Linha canônica:

```text
[disclosure] [type] Layer Name        [eye] [lock]
```

Ações secundárias devem preferir hover/context menu em vez de sete botões sempre visíveis.

### Estados
- hover;
- selected;
- active;
- focused;
- locked;
- hidden;
- shared/read-only quando necessário.

### Ações
- add raster layer;
- add group;
- add decal;
- duplicate;
- rename;
- visibility;
- lock;
- reorder por drag + alternativa por teclado;
- merge down;
- remove.

Merge/Delete não devem ocupar espaço permanente em cada linha.

### Reorder
Drag and drop precisa ter:
- insertion indicator;
- target group highlight;
- auto-scroll;
- cancel com Esc;
- alternativa `Move Up/Down` via keyboard/context menu.

## 9. Properties — ordem contextual

O painel inferior direito não mostra todas as opções de PAINT de uma vez.

### 9.1 Target
Sempre visível quando relevante:
- Object;
- Material Slot;
- Channel;
- Texture / Vertex Paint.

Se dois objetos compartilham Material/Texture:
- mostrar badge `SHARED`;
- explicar impacto;
- oferecer ação explícita de Make Unique quando suportada;
- nunca duplicar silenciosamente.

### 9.2 Brush
Quando uma ferramenta de brush estiver ativa:
- Size;
- Opacity/Strength;
- Hardness;
- Flow.

Esses são controles primários.

### 9.3 Stroke
Accordion secundário:
- Spacing;
- Smoothing;
- Size Jitter;
- Opacity Jitter;
- Scatter;
- Spray Density;
- Tip;
- Angle;
- Roundness;
- Blend.

Progressive disclosure evita um painel enorme no primeiro contato.

### 9.4 Color / Value
Canal-aware:
- BaseColor/Emission → color picker;
- Roughness/Metallic/Height/Opacity → scalar field;
- Vertex Paint → color.

Nunca mostrar color picker RGB como representação falsa de canal escalar.

### 9.5 Symmetry
X / Y / Z como toggles nomeados/tooltip.
No viewport, exibir plano(s) de simetria quando ativo e útil.

### 9.6 Fill
Somente com Fill ativo:
- scope;
- tolerance quando existir;
- target;
- selection isolation.

### 9.7 Projection
Somente quando relevante:
- Surface;
- Screen Space;
- referência/projection source;
- lock/alignment;
- ações de bake.

### 9.8 Layer Properties
Quando layer é o contexto primário:
- opacity;
- blend;
- effects;
- mask;
- decal properties;
- group properties.

## 10. Context Bar

A barra contextual abaixo da Work Surface deve mostrar apenas operações de altíssima frequência.

### Brush
```text
Size | Opacity | Color/Value | Symmetry | Channel
```

### Fill
```text
Scope | Projection | Color/Value
```

### Decal
```text
Move | Scale | Rotate | Variant | Confirm
```

### Selection
```text
Mode | Add/Subtract | Island | Clear
```

Ela não substitui Properties; funciona como quick access.

## 11. Canais

UI deve refletir o contrato de PaintChannel:

### Color
- BaseColor;
- Emission.

### Scalar
- Roughness;
- Metallic;
- Height;
- Opacity.

Direct Normal Painting não entra na primeira versão.

Se Normal Map estiver disponível como material input:
- importar;
- visualizar;
- gerar a partir de Height quando o fluxo estiver implementado.

## 12. Layer types

### Raster
Pintável diretamente.

### Group
Organização e composição.

### Decal
Usa o contrato de SurfaceAttachment/local frame quando surface decal.

### Effect
Paramétrico; não recebe brush strokes diretamente.

A UI usa ícones e labels diferentes. Não depender apenas do nome da layer.

## 13. Decals dentro de PAINT

Decal continua parte de Layers.

Selecionar uma Decal Layer:
- muda Properties para Decal;
- mostra handles relevantes no 3D;
- mantém a imagem/source identificável;
- variants ficam na mesma seção;
- Surface vs UV decal é visível.

Animated Decal não cria um segundo sistema visual. Ele acrescenta:
- animation source;
- frame/variant timing;
- playback preview quando aplicável;
- integração posterior com ANIMATE.

## 14. Effects

Effects devem aparecer como stack da layer, não como lista global solta.

Cada effect:
- enable/disable;
- reorder quando permitido;
- parâmetros;
- reset;
- remove.

Preview é live; commit segue política de Undo do domínio.

## 15. Brush Presets

O drawer inferior pode abrir a aba `Brush Presets`.

Preset mostra:
- thumbnail/tip;
- nome;
- categoria/tags futuros.

Aplicar preset:
- muda somente settings pertencentes ao preset;
- não muda target/layer silenciosamente.

Salvar preset deve ser ação explícita.

## 16. Palette

Palette não precisa ocupar permanentemente o Inspector.

Abrir no drawer inferior ou popover persistente.

Suporta:
- swatches;
- import/export;
- presets;
- histórico recente futuro.

Color picker e Palette são complementares.

## 17. Feedback visual específico

### Stroke
Durante stroke:
- footprint;
- preview direto;
- nenhuma toast por dab/stroke.

### Commit
Stroke finaliza um único Undo.

### Locked layer
Não iniciar stroke.
Mostrar:
- cursor disabled;
- inline/status reason;
- opção de desbloquear quando apropriado.

### No UV
Texture Paint exige UV válida.

Mostrar estado persistente:
```text
Texture Paint requires UVs
[Open UV Workspace]
```

Não fazer auto-unwrap silencioso.

### Shared resource
Mostrar badge + inline warning antes da primeira mutação relevante.

### Projection invalid
Mostrar motivo no contexto da ferramenta; não apenas toast.

## 18. Selection e máscaras

Selection em PAINT pode controlar:
- faces/surface;
- UV region;
- island;
- object.

`Mask Selection` deve ser visualmente evidente na viewport/canvas.

Quando máscara está ativa, regiões fora dela recebem overlay sutil ou outro sinal não destrutivo.

## 19. Input

### 3D
- LMB → tool action;
- MMB/orbit/pan conforme keymap;
- wheel → zoom;
- navigation nunca muda brush settings incidentalmente.

### 2D
- LMB → stroke/select;
- MMB → pan;
- wheel → zoom;
- atalhos resolvidos por keymap.

### Pen tablet
Arquitetura deve reservar:
- pressure;
- tilt quando suportado;
- eraser tip.

Não declarar suporte completo até teste em hardware real.

## 20. Acessibilidade no PAINT

Além do contrato global:

- Brush Size pode ser ajustado por NumericField/teclado;
- Opacity/Hardness/Flow nunca dependem apenas de slider;
- todos os swatches têm nome/valor acessível;
- Layer reorder possui alternativa a drag;
- Canvas possui comando de frame/reset zoom;
- PiP acessível sem ponteiro;
- brush cursor tem alternativa de tamanho/contraste;
- opção de aumentar cursor do brush independentemente da UI;
- lock/visibility/active não são comunicados apenas por ícone/cor;
- grupos expõem expanded/collapsed semanticamente;
- tool labels permanentes funcionam no rail de PAINT;
- reduced motion não remove informação de stroke/commit;
- High Contrast mantém UV wire, selection, brush cursor e warning distinguíveis.

## 21. Empty states

### Nenhum objeto pintável
```text
Select a paintable object
```

### Nenhum PaintDocument
```text
Create Paint Layer
```

### Sem UV para Texture Paint
```text
Texture Paint requires UVs
[Open UV Workspace]
```

### Layer bloqueada
```text
Layer is locked
[Unlock]
```

Empty state deve dizer o próximo passo, não apenas informar ausência.

## 22. Responsive behavior

Quando largura diminuir:
1. labels do Tool Rail podem recolher se a preferência permitir;
2. Context Bar move ações secundárias para overflow;
3. Structure/Properties mantêm largura mínima legível;
4. Work Surface recebe prioridade;
5. Split 3D/2D pode migrar temporariamente para tabs/alternância quando ambas ficarem inutilizáveis;
6. nunca reduzir canvas a uma miniatura funcional apenas para preservar painéis.

## 23. Performance

- brush preview é overlay transitório;
- dirty tiles/partial composition permanecem;
- partial GPU upload permanece objetivo;
- listas de layer grandes usam virtualização;
- thumbnail/preset rendering é lazy;
- 2D/3D simultâneos não forçam atualização quando unchanged;
- idle permanece render-on-demand.

## 24. Migração de UI recomendada

1. extrair `PaintWorkspaceViewModel`;
2. separar Layers de Properties;
3. mover Canvas 2D do Inspector para Work Surface;
4. criar Surface Mode 3D/2D/Split/PiP;
5. substituir botões ad-hoc de Layers por componentes base;
6. criar `PaintContextBar`;
7. reorganizar Brush em Basic + Stroke/Advanced;
8. tornar Channel/Target first-class;
9. levar Presets/Palette ao Workspace Drawer;
10. consolidar warnings de UV/shared/locked em inline feedback;
11. testar teclado/foco/High Contrast/UI Scale;
12. preservar as funcionalidades atuais durante a migração.

## 25. Gates

PAINT só é considerado migrado quando:

- Layers é a Structure primária;
- Canvas 2D é Work Surface de primeira classe;
- 3D/2D/Split/PiP funcionam sem duplicar estado;
- Brush básico cabe sem scroll excessivo em altura comum;
- parâmetros avançados usam disclosure;
- channel-aware controls não mentem sobre scalar/color;
- sem UV produz CTA e nunca auto-unwrap silencioso;
- shared resources são avisados;
- layer reorder é acessível sem drag;
- NumericField e sliders obedecem o contrato global;
- stroke = um Undo;
- High Contrast e UI Scale preservam cursor/UV/selection;
- nenhuma feature já funcional é descartada apenas para simplificar a GUI.

## 26. Decisão final

PAINT deixa de ser uma coleção de controles dentro do Inspector e passa a ser um workspace de primeira classe.

A prioridade é:

```text
Layers à direita
+ pintura no centro
+ propriedades contextuais
+ controles rápidos na Context Bar
+ presets/palette/assets no drawer
```

O motor atual será reaproveitado e reorganizado progressivamente; não haverá rewrite funcional sem necessidade comprovada.
