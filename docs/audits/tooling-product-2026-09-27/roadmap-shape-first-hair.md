# Roadmap técnico — Shape-first evoluído e Low-Poly Hair

> **Status:** proposta operacional derivada para planejamento. Hair permanece
> pós-V1/pós-GA conforme o capítulo 38. Esta página não altera o escopo congelado.

## 1. Tese arquitetural

Shape-first e Hair não devem ser dois sistemas independentes. Ambos precisam das
mesmas quatro fundações:

1. **Spline Core** para paths/profiles;
2. **SurfaceAttachment** para aderir authoring à malha;
3. **Generator evaluation** versionado e cacheável;
4. **Bake/Make Editable** transacional e previsível.

Construir Hair diretamente dentro de `SlintUiBridge` ou como outro conjunto de
vetores temporários criaria uma terceira implementação de curva e repetiria os
problemas atuais de commands, revisões e undo.

## 2. Pré-condições obrigatórias

Antes de H0:

- TX-01/TX-02/TX-03 corrigidos;
- `DirtyDomains` implantado;
- cache de modifiers/generators usa revisions corretas;
- picking e overlays aceitam entidades de authoring que não são mesh components;
- IDs persistentes/generacionais disponíveis para curve, control point,
  attachment e generator;
- serialization versionada e round-trip testado;
- commands são o único caminho de mutação;
- jobs aplicam resultado somente com revision compatível.

Sem isso, cada novo control point ou alteração de largura pode clonar projeto,
reavaliar toda a cena e criar histórico incorreto.

## 3. Modelo de dados proposto

Os nomes abaixo são uma proposta de implementação para os contratos P3D-158,
P3D-160 e P3D-161; os tipos definitivos devem seguir os IDs do projeto.

```rust
pub struct SplineResource {
    pub id: SplineId,
    pub points: Vec<SplinePoint>,
    pub closed: bool,
    pub interpolation: SplineInterpolation,
    pub revision: u64,
}

pub struct SplinePoint {
    pub id: SplinePointId,
    pub position: DVec3,
    pub handle_in: DVec3,
    pub handle_out: DVec3,
    pub handle_mode: HandleMode,
    pub attachment: Option<SurfaceAttachment>,
}

pub struct SurfaceAttachment {
    pub target: ObjectId,
    pub triangle: TriangleHandle,
    pub barycentric: DVec3,
    pub normal_offset: f64,
    pub tangent_rotation: f64,
    pub source_topology_revision: u64,
    pub status: AttachmentStatus,
}

pub struct HairClumpGenerator {
    pub id: GeneratorId,
    pub guide: SplineId,
    pub profile: HairProfile,
    pub root_width: f32,
    pub tip_width: f32,
    pub thickness: f32,
    pub segments: u16,
    pub taper: f32,
    pub twist: f32,
    pub curvature: f32,
    pub mirror: Option<MirrorParameters>,
    pub revision: u64,
}
```

### Regras

- a spline é authoring data, não mesh renderizável;
- samples, arc-length table e frames são caches descartáveis;
- `HairClumpGenerator` referencia a spline por ID;
- output mesh é derivado e cacheado por revisões;
- root attachment é compartilhado com decal/path paint;
- topology revision incompatível marca `NeedsReattach`;
- nenhuma reprojeção escolhe silenciosamente outra área;
- Bake gera um novo estado/asset normal e registra perda de editabilidade.

## 4. Spline Core

### 4.1 Escopo mínimo P3D-161

- polyline e Bézier;
- open/closed;
- add/move/delete point;
- break/align handles;
- reverse;
- tangent evaluation;
- arc-length table;
- resample por distância;
- parallel-transport frame;
- snapping;
- attachment opcional por ponto;
- serialization;
- Commands/Undo;
- Bake/Convert.

### 4.2 Cache

```text
Spline revision
  → arc-length table
  → samples at requested spacing/count
  → tangents
  → stable frames
  → generator-dependent mesh
```

A edição de um ponto invalida somente a spline e seus dependents. Não deve marcar
topologia de todas as meshes do projeto nem reconstruir texturas.

### 4.3 Estabilidade geométrica

Parallel transport deve ser a baseline para evitar flips de Frenet em trechos
quase retos. Casos de teste:

- linha reta;
- curva em S;
- loop fechado;
- tangent quase paralelo ao up inicial;
- corner agudo;
- reverse;
- segmentos degenerados/repetidos;
- escala extrema;
- determinismo save/load.

## 5. SurfaceAttachment

### 5.1 Contrato de comportamento

| Mudança | Resultado esperado |
| :--- | :--- |
| Transform do objeto alvo | attachment acompanha sem reprojetar |
| Somente posições, topologia compatível | reavaliar triangle/frame usando IDs válidos |
| Topologia incompatível | `NeedsReattach`; manter último estado para diagnóstico |
| Undo da mudança topológica | attachment volta a `Valid` se handle/revision corresponder |
| Reattach explícito | raycast/projeção determinística em command único |
| UV seam | posição geométrica continua estável; UV auxiliar pode ter lado explícito |
| Alvo removido | `MissingTarget`, nunca dangling reference |

### 5.2 APIs

- `project_ray_to_surface`;
- `evaluate_world_frame`;
- `evaluate_tangent_frame`;
- `slide_attachment`;
- `rotate_about_normal`;
- `validate_attachment`;
- `reattach_nearest` somente por ação explícita;
- `detach_keep_world`.

### 5.3 Reuso

Implementar uma única vez para:

- roots de Hair;
- control points de Surface Path;
- decals;
- Path Paint;
- accessories/sockets;
- futuros surface conform/details.

## 6. Shape-first 2.0

### S0 — Profile como recurso persistente

- migrar perfil transitório para `SplineResource` planar;
- armazenar workplane/orientation e constraints;
- preview de volume referencia Profile ID;
- edição de ponto é command;
- save/load preserva authoring;
- `Make Editable` usa contrato P3D-160.

**Aceite:** criar perfil, salvar, reabrir, editar pontos e atualizar volume sem
perder IDs nem criar novo objeto.

### S1 — Diagnóstico e constraints

- close/open/reverse;
- grid/point/edge snapping;
- horizontal/vertical/equal-length simples;
- self-intersection e segmentos degenerados;
- preview de winding/normals/caps;
- custo previsto de faces.

**Não objetivo:** solver CAD geral.

### S2 — Draw-on-Face

- control points usam SurfaceAttachment;
- overlay mostra aderência e offset;
- topology change marca pontos inválidos;
- usuário pode reattach por ponto ou path;
- perfil pode gerar extrude/cut/decal/path paint conforme comando escolhido.

### S3 — Sweep/Revolve compartilhados

- Sweep recebe `ProfileId + SplineId + parameters`;
- resampling e frames vêm do Spline Core;
- caps, twist, taper e segments são parâmetros do generator;
- Revolve usa Profile e eixo explícito;
- preview low resolution durante drag, resultado final no commit;
- Bake exibe vertices/tris e dependências removidas.

### S4 — Repeat/Stretch along path

Somente após Sweep estável:

- repeat asset along path;
- stretch segment along path;
- end/corner rules;
- instancing quando possível;
- sem introduzir node graph.

## 7. Hair incremental

As fases seguem o capítulo 38, com gates técnicos explícitos.

### H0 — Curve → Ribbon Mesh

**Escopo:** uma guide spline livre, profile Ribbon, width constante, preview e
Bake.

**Reuso:** Spline Core, resampling e parallel transport.

**Aceite:** nenhuma torção súbita nos fixtures; alterar um ponto atualiza somente
o generator; undo/redo e save/load determinísticos.

### H1 — Width, Segments e Taper

- root/tip width;
- segment count ou target spacing;
- taper curve simples;
- estimativa de vertices/tris antes do commit;
- presets Low/Medium/Custom orientados a game-ready.

**Aceite:** alterar segments não muda a forma global além da tesselação dentro de
tolerância definida por fixture.

### H2 — Solid Clumps / Profiles

- Triangle;
- Diamond;
- Box;
- low-poly Tube;
- thickness e normal orientation;
- caps configuráveis.

Perfis devem ser enums/recursos de generator, não cinco algoritmos independentes.

### H3 — Attach to Scalp

- root usa SurfaceAttachment;
- guide pode começar no tangent frame do scalp;
- normal offset e tangent rotation;
- badge `Needs Reattach` e ação explícita;
- transform do scalp atualiza guide/output sem bake.

### H4 — Mirror / Symmetry

- mirror não duplica authoring silenciosamente;
- opção live referencia guide fonte e plano;
- `Make Unique` cria guide independente;
- warning quando attachment espelhado não encontra superfície válida.

### H5 — Draw Hair Tool

Fluxo:

1. selecionar scalp/parte;
2. arrastar root → tip sobre ou a partir da superfície;
3. amostrar pointer em world/surface space;
4. simplificar/resample a curva;
5. preview Ribbon de baixa densidade;
6. ajustar width/taper/segments no Inspector;
7. confirmar uma transação.

Feedback:

- root marker;
- path e control points;
- direção root→tip;
- seção/profile ghost;
- contagem prevista;
- attachment válido/inválido;
- mirror preview.

### H6 — Clusters e variações

- um HairAsset contém vários guides/clumps;
- seleção de guide e seleção de asset são distintas;
- parâmetros podem ser shared defaults + override local;
- seed determinística;
- bounds/culling por cluster;
- um draw/batch por material/profile quando viável.

### H7+ — somente após profiling e uso real

Comb/Smooth/Cut/Lengthen, Curl/Wave, interpolação e braids permanecem posteriores.
Cada um precisa justificar UX, formato persistente, custo e export. Nenhum é
pré-requisito de um Hair Designer útil.

## 8. Commands e transações

Conjunto mínimo:

- `CreateSpline`;
- `AddSplinePoint`;
- `MoveSplinePoint`;
- `SetSplineHandles`;
- `DeleteSplinePoint`;
- `ReverseSpline`;
- `AttachSplinePoint`;
- `ReattachSplinePoint`;
- `CreateHairClump`;
- `SetHairParameters`;
- `SetHairMirror`;
- `BakeGenerator`;
- `MakeGeneratorUnique`.

Gestures de drag agrupam updates em uma transação. O dispatcher grava uma entrada
no commit. Preview nunca grava centenas de snapshots.

## 9. Renderer, picking e overlays

### Authoring overlay

O renderer recebe uma descrição neutra:

```text
Curve polyline
Control points
Selected point
Handle lines
Root marker
Attachment status
Profile cross-sections opcional
Generator bounds/cost warning
```

Esses itens não entram em `Mesh::verts/faces` e não reutilizam flags de seleção
topológica.

### Picking

- ID pass ou CPU picking por segmentos/pontos conforme profiling;
- tolerância em logical px convertida explicitamente;
- prioridade contextual entre point/curve/surface;
- hover não altera documento;
- picking cacheado por curve revision e camera.

### Output mesh

- cache por `guide_revision + parameters_revision + attachment_target_revision`;
- bounds calculados junto ao output;
- upload somente do clump/cluster alterado;
- sem reevaluar todos os generators em camera-only frame;
- Bake reutiliza exatamente o mesmo evaluator do preview final.

## 10. UI/UX Hair

### Criação à esquerda

Uma única entrada `Hair` abre escolhas progressivas:

- Draw Hair;
- Hair from Selected Curve;
- Add Clump to Hair Asset.

Não criar uma toolbar separada extensa.

### Inspector à direita

Ordem sugerida dentro do shell canônico:

1. Parts — Hair Asset/Clumps;
2. Transform;
3. Hair — Profile, Width, Taper, Segments, Twist;
4. Attachment — target/status/reattach;
5. Material;
6. Object/Generator — Bake/Make Editable.

### Acessibilidade

- todos os parâmetros editáveis por teclado;
- point selection aparece na árvore acessível como lista do guide;
- draw gesture tem alternativa `Add Point` + edição numérica;
- status de attachment não depende somente de vermelho/verde;
- reduced motion remove animações de interpolação do preview;
- F6 navega para viewport/Inspector e restaura o ponto de edição.

## 11. Performance e memória

O caderno proíbe metas arbitrárias antes de fixtures reais. Portanto, os gates
são relativos e instrumentados:

- counters de spline sample/frame cache hit/miss;
- tempo de evaluate por clump e cluster;
- vertices/tris derivados;
- bytes CPU/GPU por Hair Asset;
- número de buffers atualizados por edição;
- frame de câmera não reavalia Hair;
- editar um guide não reavalia siblings independentes;
- preview reduz densidade quando evaluator excede um frame perceptível;
- jobs usam snapshot + revision check + cancel.

Fixtures mínimas:

1. 1 Ribbon simples;
2. 32 clumps curtos;
3. 128 clumps agrupados;
4. curvas com corners/loops;
5. scalp deformado sem mudança topológica;
6. topology change que invalida attachments;
7. mirror com lado sem superfície correspondente.

## 12. Testes e Definition of Done

### Matemática

- arc length/tangent/resampling;
- frame stability/reverse/closed loop;
- profile triangulation/normals/caps;
- taper/twist determinísticos.

### Attachment

- barycentric round-trip;
- parent transform;
- position-only update;
- topology invalidation;
- explicit reattach;
- save/load;
- UV seam.

### Transação

- gesture gera uma entrada;
- cancel restaura A sem entrada;
- `A → B → undo A → redo B`;
- stale job nunca commita;
- Bake é atômico.

### Produto

- keyboard-only path;
- tree/Inspector/viewport selection sincronizada;
- high contrast/reduced motion/200% scale;
- export do baked mesh;
- live vs baked equivalence dentro de tolerância.

## 13. Sequência de execução recomendada

| Gate | Entrega | Dependência | Resultado |
| :--- | :--- | :--- | :--- |
| G0 | Transações/revisões/cache corrigidos | atual | fundação segura |
| G1 | Spline Core headless | G0 | curvas compartilhadas |
| G2 | SurfaceAttachment headless | G0/G1 | aderência persistente |
| G3 | Profile persistente + Sweep migrado | G1/G2 | shape-first consolidado |
| G4 | Overlay/picking de curvas no Slint | G1 | edição visual acessível |
| G5 | H0–H2 | G1/G3/G4 | clumps livres low-poly |
| G6 | H3–H5 | G2/G5 | Hair desenhado no scalp |
| G7 | H6 + profiling | G6 | Hair Assets escaláveis |
| G8 | H7+ selecionado por uso | G7 | expansão controlada |

Essa ordem pavimenta Hair ao melhorar imediatamente Profile, Sweep,
Draw-on-Face, decals e Path Paint, evitando infraestrutura exclusiva para uma
feature pós-V1.
