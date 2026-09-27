# Revisão de produto por workspace e superfície

## 1. MODEL

### 1.1 O que já é sólido

O maior ativo técnico do projeto é o conjunto de operadores geométricos em
`petunia_mesh` e comandos correspondentes em `petunia_core`. A suíte observada
exercita invariantes topológicas, seleção, operações destrutivas e vários casos
de borda. O editor não depende apenas de botões cenográficos.

| Capacidade | Avaliação | Observação |
| :--- | :---: | :--- |
| Seleção Point/Edge/Face/Object | `PARTIALLY_COMPLIANT` | Domínios e preselection existem; autoridade/revisão ainda se divide entre mesh e sessão |
| Move/Rotate/Scale | `COMPLIANT` | Modal e confirmação/cancelamento estão presentes; input precisa passar integralmente pelo keymap |
| Extrude/Individual | `PARTIALLY_COMPLIANT` | Operação e modal existem; fluxo transacional e presets estão na bridge, não em sessão neutra completa |
| Inset | `COMPLIANT` | Operador e parâmetros relevantes existem |
| Round Edge (Bevel) | `PARTIALLY_COMPLIANT` | Geometria e segmentos existem; feedback de clamp/overlap e linguagem pública precisam consolidar |
| Loop Cut | `PARTIALLY_COMPLIANT` | Comando/operação existem; preview e escolha de loop precisam testes visuais mais fortes |
| Knife/Slice | `PARTIALLY_COMPLIANT` | Fundação funcional; sessão/overlay ainda depende do frontend e dos caminhos de input |
| Subdivide/Dissolve/Delete | `PARTIALLY_COMPLIANT` | Algoritmos existem; undo duplicado/tardio torna o ciclo de produto inseguro |
| Merge/Weld/Connect | `COMPLIANT` funcional | Cobertura algorítmica boa; preservar vocabulário de usuário `Connect` |
| Join/Separate/Keep Parts | `PARTIALLY_COMPLIANT` | Operações existem em níveis diferentes; tornar resultado e ownership explícitos |
| Fuse/Cut (booleans) | `PARTIALLY_COMPLIANT` | Provider manifold isolado é boa fronteira; preflight e mensagens de falha ainda devem amadurecer |
| Mirror/Symmetrize | `PARTIALLY_COMPLIANT` | Modifiers e ações existem; cache de avaliação é incorreto e ToolRegistry não os representa uniformemente |
| Normals/diagonal/orientation | `PARTIALLY_COMPLIANT` | Funções existem; Flat/Smooth visual não é propriedade persistente correta |

### 1.2 Shape-first atual

Há fundações reais e úteis:

- perfil com pontos e nós Bézier;
- escolha de workplane;
- preview de volume e Depth Handle;
- primitivas com parâmetros editáveis;
- revolve;
- sweep simples;
- geração low-poly e caminho de tornar editável.

O problema não é a matemática isolada. É o ciclo de vida. O perfil ainda funciona
principalmente como estado de sessão e lógica de bridge, não como um recurso de
projeto serializável com ID, revisão, commands e dependências. Com isso:

- editar um ponto pode não seguir a mesma disciplina de undo de editar malha;
- Draw-on-Face não tem attachment persistente;
- Sweep e futuro Hair tenderiam a criar outra estrutura de curva paralela;
- modifiers/generators não recebem invalidação precisa;
- export/bake não compartilham um contrato uniforme.

### 1.3 Melhorias de usabilidade MODEL

1. **Feedback de operação no viewport:** mostrar valor atual, eixo/plano,
   unidade, clamp e motivo de recusa próximo ao cursor, não só na status bar.
2. **Preflight visível:** self-intersection de perfil, face degenerada, non-
   manifold e boolean sem interseção devem ser diagnosticados antes do commit.
3. **Last Operation coerente:** toda ação paramétrica recente deve poder ajustar
   parâmetros até outra ação destrutiva, com botão explícito `Make Editable`.
4. **Snap explicável:** indicar alvo e tipo (`Point`, midpoint, grid, surface),
   com cor/forma que não dependa apenas de cor.
5. **Densidade low-poly:** presets semânticos e contagem prevista de faces antes
   de confirmar Revolve/Sweep/Hair.
6. **Seleção oclusa:** feedback claro quando X-Ray muda a elegibilidade de pick;
   separar aparência wireframe de política de seleção.
7. **Vocabulário:** UI pública usa Point, Round Edge, Fuse, Cut, Connect, Keep
   Parts, Join e Project From Reference/View; termos técnicos ficam em docs
   avançadas/diagnóstico.

### 1.4 Riscos estruturais MODEL

- deleção/reindexação em vetores exige cuidado com IDs efêmeros;
- adjacency é reconstruída por operadores em vez de mantida como estrutura
  autoral/cache comum;
- `evaluated_mesh()` clona malha por consulta;
- cache atual pode ficar stale após mudança de posições/conectividade com mesmas
  contagens;
- ToolRegistry legado e sessões Slint não descrevem a mesma lista de tools;
- commands nem sempre são o único caminho para algoritmos.

## 2. PAINT

### 2.1 Inventário funcional

| Capacidade | Implementação observada | Avaliação |
| :--- | :--- | :---: |
| Vertex color 3D | Paint e eyedrop por vértice | `COMPLIANT` básico |
| Brush raster | Hard/soft, opacity, hardness, flow e falloff | `PARTIALLY_COMPLIANT` |
| Eraser | Alpha/strength no canvas | `PARTIALLY_COMPLIANT` |
| Eyedropper | Canvas e vertex paths | `COMPLIANT` básico |
| Fill | Canvas e escopos UV/seleção | `PARTIALLY_COMPLIANT`; undo precisa correção |
| Shapes | Linha e retângulo | `PARTIALLY_COMPLIANT` |
| Gradient | Linear | `PARTIALLY_COMPLIANT` |
| Symmetry | Dabs espelhados | `PARTIALLY_COMPLIANT`; composição é repetida |
| 3D paint por UV | Face hit → barycentric UV → pixel | `COMPLIANT` básico |
| Layers/groups | Ordem, opacity, lock, visibility, grupos | `PARTIALLY_COMPLIANT`; transação/custo |
| Effects | Estruturas e composição básica | `RUDIMENTARY` frente a masks/canais |
| Decal | Layer, transform e bake | `PARTIALLY_COMPLIANT`; drag undo incorreto |
| Reference bake | Comando presente | `PARTIALLY_COMPLIANT` |
| Palette | Import/export e cores recentes | `COMPLIANT` básico |
| UV bleed/dilate | Operação disponível | `COMPLIANT` básico |

### 2.2 Problemas de experiência

1. **Stroke não é determinístico entre dispositivos.** Passos fixos no Slint
   fazem o resultado depender de poll rate, zoom e resolução. `spacing` precisa
   ser respeitado pelo sampler comum.
2. **Feedback de custo está ausente.** Resolução, layers e simetria podem tornar
   um stroke caro sem aviso; a UI deve exibir tamanho de canvas, memória estimada
   e estado de composição.
3. **Canais materiais não são claros.** A UI/material sugere PBR, mas a pintura
   efetiva e o renderer concentram-se em cor/albedo. O usuário precisa saber o
   canal ativo e quais canais a V1 realmente suporta.
4. **Layers misturam ação e cache.** `Asset.texture` e
   `Material.albedo_texture` recebem cópias; o usuário vê uma pilha, mas a
   implementação mantém múltiplos raster owners.
5. **Feedback 2D/3D deve ser equivalente.** Cursor, raio, hardness, spacing,
   simetria, seams e área afetada precisam usar a mesma linguagem visual.
6. **Fill necessita preview/limite.** Escopos grandes deveriam mostrar alvo e
   permitir cancelamento/job quando exceder orçamento.

### 2.3 Arquitetura de pintura recomendada

```text
PointerSample
  → StrokeSampler (spacing, pressure, smoothing, symmetry)
  → DabBatch
  → PaintOperation (layer/channel/UV tiles)
  → DirtyTiles + undo delta
  → Compositor incremental
  → DirtyRects de GPU
  → Viewport/Canvas observam revisão
```

Regras:

- uma amostra consolidada gera no máximo uma composição por layer/canal;
- undo armazena tiles anteriores comprimidos/deduplicados, não o projeto inteiro;
- 2D e 3D compartilham sampler e operação;
- zoom do canvas é apresentação, não reamostragem CPU;
- o compositor é independente de Slint/WGPU;
- preview do brush não altera pixels.

## 3. UV

### 3.1 O engine deve ser preservado

`petunia_module_uv` oferece um conjunto coerente para V1:

- unwrap automático por `xatlas`;
- cube/project from view/reference;
- pack islands com padding;
- seams;
- pin/unpin;
- move/scale/rotate;
- stitch;
- relax;
- texel density e diagnósticos.

Essas funções são valiosas para PAINT e não devem ser removidas com a superfície
UV independente.

### 3.2 O workspace deve ser retirado da navegação principal

A decisão atual do Livro Vivo é manter UV como utilitário de preparação dentro
de PAINT. A experiência recomendada:

1. PAINT → `Preparar superfície`;
2. painel temporário/estado de tarefa com `Unwrap`, `Seams`, `Pins`, `Pack`,
   `Density`, `Check Stretch`;
3. preview 2D opcional, fechado por default;
4. `Concluir` retorna à pintura preservando câmera/brush;
5. erros de UV oferecem ação direta, não exigem trocar de workspace.

### 3.3 Débitos técnicos UV

- seleção UV deve ter IDs próprios por corner/UV vertex/island;
- `Face.selected` não deve servir simultaneamente a topologia e UV;
- transform UV é um gesture transacional único;
- view-model não deve serializar toda a malha como string SVG em cada sync;
- diagnósticos de overlap/stretch/out-of-bounds devem gerar overlays estruturados;
- pack/unwrap em malhas grandes precisam job/cancel/progress, não bloquear UI;
- density deve explicitar unidade e resolução alvo.

## 4. Viewport e feedback visual

### 4.1 Pontos fortes

- grid adaptativo;
- câmera com presets, frame selection/all e projeção;
- camadas separadas de geometria, linhas, seleção e referências;
- preselection por domínio;
- overlays de triangulação, wire, face orientation e UV checker;
- render-on-demand/fingerprint evita reconstrução geométrica em vários frames de
  câmera/hover;
- referências têm upload condicionado a hash;
- X-Ray e seleção oclusa são conceitos visíveis.

### 4.2 Problemas atuais

- modos principais não correspondem ao contrato canônico;
- Material Preview e Rendered têm descrições falsas;
- Flat/Smooth não persiste por objeto;
- ausência de Silhouette/Reference como modo principal;
- seleção explícita não tem revisão integrada de forma inequívoca ao early-return;
- wireframe é desenhado como linhas sem estratégia robusta de largura/AA em todas
  as GPUs;
- não há orçamento/LOD para overlays em cenas grandes;
- não há indicação padronizada de operação inválida, preview aproximado ou job
  pendente.

### 4.3 Linguagem de feedback recomendada

| Estado | Feedback mínimo |
| :--- | :--- |
| Hover elegível | contorno/elemento destacado sem mutar seleção |
| Selecionado | cor + espessura/forma distinta; não depender só de cor |
| Snap ativo | marcador do alvo, tipo e distância |
| Modal em curso | valor, unidade, eixo/plano e instruções confirmar/cancelar |
| Operação inválida | preview vermelho/hachurado + motivo objetivo |
| Cálculo assíncrono | spinner discreto, progresso quando mensurável e cancel |
| Preview aproximado | badge `Preview`; commit recalcula resultado final |
| Attachment perdido | marcador na raiz + ação `Reattach` |
| Custo excessivo | estimativa de faces/memória antes de commit |

## 5. GUI, navegação e design system

### 5.1 O que funciona

- shell reconhecível e orientado ao viewport;
- MODEL com criação à esquerda, Inspector à direita e Assets abaixo;
- componentes visuais reutilizáveis já existem;
- dark theme oficial coerente;
- toolbar/menus/inspector têm hierarquia visual clara;
- OverlayStack e modais formam uma base melhor que popups ad-hoc;
- view-model separa parte da leitura de estado da declaração Slint.

### 5.2 O que reduz profissionalismo

1. **Texto e idioma inconsistentes:** strings literais em inglês e português
   impedem localização consistente.
2. **Copy promete tecnologia:** tooltips devem explicar resultado visual, não
   dizer “PBR/raytraced” quando isso não existe.
3. **Controles ad-hoc:** `Rectangle + TouchArea` repetidos não herdam estados de
   foco, disabled, tooltip e acessibilidade de um componente canônico.
4. **Arquivos gigantes:** tornam revisão visual, ownership e regressão mais
   difíceis.
5. **Render em cascata:** muitos callbacks sincronizam view-model e viewport
   imediatamente; uma transação de UI deveria coalescer updates.
6. **Densidade excessiva:** controles de 22–32 px funcionam com mouse preciso,
   mas exigem hit targets acessíveis e modo de escala robusto.

### 5.3 Modularização sem reescrita

```text
ui-slint/
  shell/               # top bar, regions, overlay host
  model/               # view model + intents MODEL
  paint/               # canvas/layers/brush projections
  uv_utility/          # preparação de superfície dentro de PAINT
  viewport/            # toolbar, feedback, adapter GPU
  settings/            # preferences/keymap/theme
  commands/            # projeção do catálogo canônico
  accessibility/       # focus regions and semantic helpers
```

Os componentes `.slint` podem ser importados por arquivo sem quebrar os nomes
públicos atuais. A extração deve ocorrer uma vertical slice por vez, com testes
de snapshot estrutural e nenhuma mudança de layout incidental.

## 6. Configurações

### 6.1 Implementado de forma útil

- tema dark e high contrast;
- escala de UI;
- reduced motion;
- esquema de eixos para daltonismo;
- perfis de keymap;
- preferências persistidas;
- opções de viewport e navegação relevantes.

### 6.2 Gaps

- cheat sheet de shortcuts é literal e pode divergir do keymap ativo;
- atalhos físicos fora do keymap tornam o seletor de perfil parcialmente falso;
- faltam os perfis canônicos previstos que só devem ser adicionados após a
  consolidação da autoridade;
- settings declara conformidade acessível sem auditoria completa;
- algumas opções visuais não têm contraparte persistente por projeto/objeto;
- não há página de diagnóstico clara para GPU, limites, memória de undo, canvas e
  caches;
- reset por seção e diff para defaults devem ser padronizados.

### 6.3 Configurações profissionais recomendadas

1. **Interface:** scale, density preset, reduced motion, tooltip delay, language.
2. **Input:** perfil, busca por comando, remap/conflict detector, import/export.
3. **Viewport:** AA/quality preset, overlays, selection-through, reference
   quality, fallback seguro por GPU.
4. **Paint:** default resolution, memory warning, spacing/pressure curve, tile
   cache budget.
5. **Files:** autosave/recovery e caminhos; apenas quando serviço real existir.
6. **Diagnostics:** adapter/backend, VRAM estimate, scene stats, undo bytes,
   texture bytes, cache hits/rebuilds; sem opções placebo.

## 7. Acessibilidade

### 7.1 Estado positivo

- escala 100–200%;
- tema high contrast;
- reduced motion;
- eixos com labels/forma, não apenas cor;
- uso parcial de roles/labels;
- foco visual em componentes canônicos;
- design desktop com regiões previsíveis.

### 7.2 Prioridades

1. implementar F6/Shift+F6 entre Top Bar, criação, viewport, Inspector, Assets e
   status/contexto;
2. Tab/Shift+Tab fica dentro da região atual;
3. Escape fecha somente o topo da OverlayStack e restaura foco ao invocador;
4. converter controles ad-hoc para componentes que forneçam semântica padrão;
5. expor selected/checked/expanded/value/min/max;
6. garantir alternativa de teclado para drag, reorder, splitter, color swatch e
   canvas actions;
7. validar contraste de todos os tokens/estados, inclusive overlays;
8. permitir hit target maior que o desenho visual;
9. não anunciar WCAG AA antes de auditoria assistiva com AccessKit/leitor real;
10. testar 200% scale em viewport mínimo canônico sem cortar ações essenciais.

## 8. Ordem de produto recomendada

1. integridade de undo/transações;
2. honestidade de UI e contrato de shading;
3. performance e determinismo PAINT;
4. autoridade única de command/keymap e navegação acessível;
5. UV como utilitário PAINT;
6. modularização da bridge/UI;
7. Spline Core + SurfaceAttachment;
8. expansão shape-first;
9. Hair Clump incremental;
10. somente depois, canais avançados e operações de cena mais pesadas.
