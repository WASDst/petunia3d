# Performance, memória, concorrência e budgets

> **Status: aprovado em 2026-10-08 para arquitetura alvo.** Metas numéricas só devem ser congeladas depois de medir hardware de referência, não inventadas nesta especificação.

## Premissa de produto

Petunia3D prioriza criação low-poly acessível em **Windows, Linux e hardware modesto**. Boa performance é requisito de arquitetura, não argumento para perda silenciosa de qualidade geométrica, dados, Undo ou acessibilidade.

## Ownership e threads

- **Document single-writer** em Application. Nenhum worker/renderer/plugin escreve diretamente no Document.
- Jobs pesados recebem snapshot imutável, revision e cancel token; retornam resultado tipado, progress e diagnostics.
- Application faz commit apenas após conferir revision/preconditions; resultados stale não sobrescrevem edição posterior.
- OpenGL context é possuído pelo DesktopHost/renderer com lifecycle e thread ownership explícitos.
- UI thread não bloqueia em bake, boolean caro, global UV pack, image decode ou export.
- Nunca criar scheduler genérico nem paralelização automática de toda operação simples antes de profiling.

## Caches e revisões

| Mudança | Invalidar | Não invalidar |
|---|---|---|
| Transform de SceneObject | world bounds/render transform/picking derivado conforme necessidade | topology/triangulation autoral |
| Vertex edit | normals, geometry GPU, spatial acceleration e derived attrs afetados | textures sem alteração |
| Edge/Face topology | triangulation, remapping, picking, shading, UV analysis | preferências de UI |
| Paint stroke | composite tiles/dirty rects, texture upload parcial | geometria toda |
| Material edit | bindings, shader inputs, affected objects | topology sem mudança |
| Reference image | reference texture/projection dependents | unrelated SceneObjects |
| Frame/pose | skinned/evaluated buffers/active viewport | base authored mesh |
| Workspace UI | layout/DTO projections | caches Geometry desnecessários |

Cashes GPU são descartáveis; IDs + revisions dão chaves estáveis. Evitar calcular hash integral da Mesh em cada frame. Grandes blobs podem usar Arc/copy-on-write somente quando simplificarem ownership e houver benefício.

## Renderer e frame scheduling

Renderer único alvo: petunia-render/OpenGL 3.3 Core, glow, FBO texture apresentada pelo Slint/FemtoVG sem readback por frame. Regras:
- render-on-demand: scene, viewport ou overlay dirty desencadeiam frame; UI idle não roda GPU continuamente;
- estados GL declarados/restaurados pelos passes, sem vazamentos para Slint;
- 1 contexto compartilhado quando possível; Split2 compartilha scene resources, não duplicate full GPU cache;
- viewport passiva atualiza no necessário, com throttle durante gestos da ativa;
- resize/HiDPI reallocate explicitamente, sem thrashing permanente;
- perf tiers ativam/desativam overlay/render extras de modo explicável, preservando selection, editing e legibilidade;
- Matcap, AO, preview expensive e antialiasing são adaptativos e nunca hard requirements de hardware;
- textures têm uploads por dirty rect; evitar realloc full texture para brush update pequeno.

## Jobs e cancelamento

Estados mínimos: Queued → Running → Completed / Failed / Cancelled / Stale.

- Progress incrementa em passos reais; job desconhecido usa status indeterminado, sem porcentagem falsa.
- Cancel impede commit de trabalho obsoleto, limpa temp resources e não corrompe Undo.
- Prioridades explícitas: input/viewport responsivo > preview > processamento background não urgente.
- Coalescer work repetido (e.g. drag preview, recovery writer, invalidations).
- Erro de GPU/worker precisa de diagnóstico e recovery path, sem loop de retries infinito.
- Background work não ocupa CPU/GPU permanentemente se cena ociosa.

## Budgets observáveis — sem números fictícios

Definir hardware-alvo e medir antes de formalizar thresholds:

| Métrica | Cenário | Critério qualitativo V1 |
|---|---|---|
| UI idle CPU/GPU | cena estática | praticamente sem trabalho contínuo |
| Input latency | mouse, pen, keyboard | interação acompanha gesture sem travamentos |
| Active viewport frame time | orbit, transform | responsivo no tier alvo |
| Passive viewport overhead | Split2 | não prejudica active viewport |
| GPU peak memory | scene low-poly + texture | cabe em dispositivo de referência |
| Save/load latency | projeto típico e legado | progress e cancel/feedback claros quando longo |
| Autosave peak | documento dirty | não congela input |
| Paint stroke | 2D/3D with layers | dirty tiles, sem upload full desnecessário |
| Undo memory | geometria/paint operações comuns | bounded policy transparente |
| Large invalid inputs | import, image, path | rejeição sem OOM/panic |

Antes/depois sempre registrar hardware, GPU driver, resolução, DPI, versão, corpus e feature flags. Limites concretos serão aprovados por benchmark real, não extrapolação de workstation.

## Low-end & accessibility

UI Scale 200%, high contrast, reduced motion, legibility e larger targets **não** podem ser removidos para ganhar FPS. Preferir reduzir AO/MSAA, frequência de passive viewport, qualidade de thumbnail ou tamanho de cache, mantendo sinalização de estado e picking correto.

## Performance regressions: gates

- Benchmark repetível para Geometry, snap, evaluated mesh, paint layers, GL upload e resize.
- Perfil CPU/allocs e timers GPU quando disponíveis e seguros; não interpretar FPS sozinho.
- Stress de abertura/fechamento, switch de workspace, stale job, context loss e GPU error.
- Memória estabiliza após criar/apagar objetos, snapshots e texturas; sem crescimento linear contínuo.
- Debug/release e flags documentados; não declarar melhoria sem comparação válida.

## Decisões fechadas

Single writer; jobs snapshot-based com revision/cancel; caches por ID/revision; render-on-demand; GL texture direto no Slint; perf tiers sem degradar acessibilidade; budgets numéricos dependem de medição reprodutível.
