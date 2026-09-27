# Matriz Implementation-vs-Spec

> Esta matriz preserva o snapshot original da auditoria. O estado pós-remediação
> dos gates concluídos está registrado em `g0-remediation-2026-09-27.md`,
> `g1-paint-remediation-2026-09-27.md` e
> `g2-paint-gpu-remediation-2026-09-27.md`; `PA-04` passa a `COMPLIANT` no
> frontend Slint/WGPU de produção após o G2. O G3 está registrado em
> `g3-spline-core-remediation-2026-09-27.md` e eleva `HA-01` a
> `PARTIALLY_COMPLIANT`. O G4 está registrado em
> `g4-surface-attachment-remediation-2026-09-27.md` e eleva `HA-02` a
> `PARTIALLY_COMPLIANT` na fundação headless, sem declarar transforms
> hierárquicos, UI ou todos os consumidores concluídos. As linhas abaixo
> preservam o snapshot original; os relatórios de gate são o estado corrente.

## 1. Legenda obrigatória

| Estado | Uso nesta auditoria |
| :--- | :--- |
| `COMPLIANT` | Contrato relevante existe, está alcançável no produto e concorda com o caderno |
| `PARTIALLY_COMPLIANT` | Parte material do contrato existe, mas há lacunas ou caminhos divergentes |
| `FUNCTIONAL_BUT_DIFFERENT` | Há comportamento útil e alcançável, porém o conceito/contrato difere do canônico |
| `RUDIMENTARY` | Existe somente uma fundação pequena, sem o ciclo de produto exigido |
| `STUB` | A superfície existe, mas não realiza a função prometida |
| `BROKEN` | A implementação pode produzir estado, histórico ou feedback incorreto |
| `DUPLICATED` | Há mais de um dono/fluxo para o mesmo conceito, com risco de divergência |
| `MISSING` | O requisito relevante não está implementado |
| `OBSOLETE` | O código/superfície foi superado por decisão canônica posterior |

## 2. Matriz consolidada

| ID | Área/contrato | Estado | Evidência executável | Delta comprovado | Fonte canônica |
| :--- | :--- | :---: | :--- | :--- | :--- |
| ARC-01 | Domínio independente de Slint/egui | `COMPLIANT` | `petunia_core`, `petunia_commands`, `petunia_project` e `petunia_config` não importam Slint | Manter o limite; não mover estado de widget/GPU para o core | Constituição 03; cap. 34 |
| ARC-02 | `Tool → Command → Algorithm → Data` | `PARTIALLY_COMPLIANT` | `CommandDispatcher` e comandos semânticos existem; `SlintUiBridge` ainda chama módulos e muta `Project/Mesh` diretamente | Fazer callbacks emitirem intents/comandos; módulos não devem checkpointar | Constituição 04; cap. 34 |
| ARC-03 | Documento single-writer | `PARTIALLY_COMPLIANT` | `AppState` centraliza o documento; bridge é protegido por `Arc<Mutex<_>>` e callbacks fazem 241 locks | Tornar a fila de intents/event-loop o dono explícito; eliminar falha silenciosa por poison | Caps. 19 e 34 |
| ARC-04 | Mutação transacional única | `BROKEN` | Há checkpoints pós-mutação e checkpoints duplicados dentro de comandos + dispatcher | Um owner de transação; testes de undo/redo por ação pública | Constituição 11; cap. 34 |
| ARC-05 | Revisões por domínio | `BROKEN` | `emit_mesh_changed()` incrementa topologia e posições para alterações heterogêneas | APIs `emit_topology_changed`, `emit_uv_changed`, `emit_paint_changed`, etc. | Caps. 19 e 34 |
| ARC-06 | Catálogo semântico único de comandos | `DUPLICATED` | `ui-slint/src/commands.rs::CommandId` e `core::CommandDispatcher` têm IDs/nomenclaturas diferentes | Catálogo central com metadados projetados para a UI | Constituição 00/04; cap. 36 |
| ARC-07 | Histórico com orçamento real | `BROKEN` | `UndoStack::checkpoint` genérico usa estimativa superficial para `Project` em alguns caminhos | Estimador profundo ou deltas/comandos reversíveis, sem snapshot invisível ao budget | Cap. 19; constituição 11 |
| UI-01 | Slint como frontend de produção | `COMPLIANT` | Shell padrão e bridge vivem em `crates/ui-slint`; egui permanece atrás da transição | Preservar; não criar feature nova em egui | AGENTS §0.1; cap. 36 |
| UI-02 | Shell MODEL: criação esquerda, Inspector direita, Assets abaixo | `COMPLIANT` | Estrutura está presente em `app.slint` | Refinar sem docking irrestrito | Cap. 36 |
| UI-03 | Inspector `Parts → Transform → Material → Object` | `PARTIALLY_COMPLIANT` | Seções existem, mas visibilidade/estado e fluxos paralelos variam por domínio/ferramenta | Teste estrutural e de ordem em larguras suportadas | Cap. 36 |
| UI-04 | Componentes Slint reutilizáveis | `PARTIALLY_COMPLIANT` | Há `TopAction`, `ToolButton`, `NumericField`, `InspectorSection`; múltiplos `Rectangle + TouchArea` ad-hoc repetem controles | Migrar por família, sem big-bang | AGENTS §0.1; cap. 36 |
| UI-05 | Strings públicas via `TextId` | `BROKEN` | Centenas de textos, tooltips e mensagens são literais em Rust/Slint, misturando inglês e português | Projeção de `TextId` para propriedades Slint; zero texto público literal | Constituição 00/03; cap. 36 |
| UI-06 | Ícones via `IconId` | `PARTIALLY_COMPLIANT` | Há `IconSet` e assets semânticos; também há URLs/seleções diretas em componentes | Um registry semântico e fallback testado | Constituição 00/03; cap. 36 |
| UI-07 | Aparência via `ThemeToken` | `PARTIALLY_COMPLIANT` | `DesignTokens` e temas existem; `app.slint` contém 133 cores hex literais | Converter feedback, overlays e estados restantes para tokens | Constituição 00/03; cap. 36 |
| UI-08 | Layout responsivo nativo | `PARTIALLY_COMPLIANT` | Layouts Slint e constraints são amplamente usados; o arquivo monolítico contém dimensões locais rígidas e superfícies extensas | Testar 1280×720, 1440×900, 4K/200%; reduzir medidas duplicadas | Cap. 36 |
| UI-09 | OverlayStack LIFO, Escape e click-away | `PARTIALLY_COMPLIANT` | Infraestrutura de overlay existe e é usada; foco/restauração e modais ad-hoc precisam cobertura | Testes de trap/restauração/foco e pilha aninhada | AGENTS §0.1; cap. 36 |
| UI-10 | Monólitos de apresentação sustentáveis | `RUDIMENTARY` | `app.slint` tem 11.441 LOC; bridge 11.210; callbacks 4.766 | Extrair controllers/projections por feature mantendo contratos públicos | Qualidade operacional derivada |
| IN-01 | Input físico resolvido pelo keymap | `BROKEN` | `route_shortcut()` trata teclas físicas e modos antes do fallback de keymap | Resolver gesto → `CommandId` numa única camada | Constituição 00/04; cap. 36 |
| IN-02 | Perfis de keymap completos | `PARTIALLY_COMPLIANT` | Perfis Petunia/Blender/Maya/3ds Max existem; faltam variantes canônicas e há atalhos fora deles | Acrescentar perfis previstos só após eliminar bypass físico | Cap. 36 |
| IN-03 | F6/Shift+F6 navega regiões principais | `MISSING` | F6 é convertido para string, mas não há comando regional alcançável | Implementar ordem de regiões e retorno de foco | Cap. 36 |
| AX-01 | Alto contraste, escala e reduced motion | `COMPLIANT` | Tema high-contrast, UI scale, e reduced motion persistidos; transições consultam setting | Manter testes e validar contraste real | Cap. 36 |
| AX-02 | Semântica e teclado para controles | `PARTIALLY_COMPLIANT` | 250 `TouchArea`, 80 `accessible-role` e 83 `accessible-label`; vários controles não são focáveis/acionáveis por teclado | Inventário controle a controle; componente interativo deve fornecer papel, nome, estado e ação | Cap. 36; WCAG como benchmark |
| AX-03 | Alegação de conformidade WCAG/AccessKit | `FUNCTIONAL_BUT_DIFFERENT` | Settings afirma cobertura ampla, mas a árvore acessível e navegação regional ainda são incompletas | Trocar alegação por estado honesto até auditoria assistiva | Cap. 36 |
| VP-01 | Modos Textured/Solid/Wireframe/Silhouette-Reference | `FUNCTIONAL_BUT_DIFFERENT` | Implementação expõe Wireframe/Solid/Material Preview/Rendered | Alinhar ao cap. 05 ou registrar reconciliação canônica; não copiar Blender Rendered | Caps. 05 e 36 |
| VP-02 | Default Textured | `FUNCTIONAL_BUT_DIFFERENT` | `Shading::default()` é Solid | Tornar o default canônico quando o modo Textured existir corretamente | Cap. 05 |
| VP-03 | Flat/Smooth como propriedade do objeto | `STUB` | Renderer fixa `smooth = false`; ações de shade apenas recalculam normais/estado efêmero | Persistir propriedade por asset e honrá-la no pipeline | Cap. 05 |
| VP-04 | Unlit secundário | `PARTIALLY_COMPLIANT` | Perfis Unlit/Emissive alteram caminho de triângulos, sem controle de viewport claramente separado | Expor como opção secundária sem criar quinto modo principal | Cap. 05 |
| VP-05 | Material Preview honesto | `FUNCTIONAL_BUT_DIFFERENT` | Textura/base color/emission são usadas; roughness, metallic, normal e alpha não afetam o shader observado | Renomear/limitar descrição ou implementar o contrato material correspondente | Caps. 05, 28 e UI copy |
| VP-06 | Rendered/ray tracing/sombras | `BROKEN` | UI promete iluminação realista/ray-traced e sombras; renderer usa uma luz direcional difusa sem sombras | Remover a promessa e o modo, conforme cap. 05, ou reconciliar formalmente | Cap. 05 |
| VP-07 | Feedback de seleção/preselection | `PARTIALLY_COMPLIANT` | Camada dedicada, hover e domínios existem; revisão explícita de seleção não participa consistentemente do early-return | Incluir `selection_revision` e testes de clique sem mudança de hover | Caps. 15 e 36 |
| VP-08 | Render-on-demand e buffers persistentes | `COMPLIANT` | Fingerprint evita reconstrução em câmera/hover; buffers são persistentes | Preservar counters e medir cenários grandes | Cap. 19 |
| VP-09 | Avaliação/culling/batching de assets | `RUDIMENTARY` | Cada asset visível é avaliado e emitido; não há frustum culling/instancing; seleção pode reevaluar | Cache por revisão, culling coarse e instancing só quando profiling justificar | Cap. 19 |
| MO-01 | Primitivas paramétricas e Make Editable | `PARTIALLY_COMPLIANT` | Primitivas e sessão paramétrica existem; ciclo de edição/serialização varia por ferramenta | Um contrato comum de generator, inspector e bake | Cap. 02; P3D-160 |
| MO-02 | Perfis shape-first + workplane + Depth | `PARTIALLY_COMPLIANT` | Perfis, workplane e volume preview existem e são alcançáveis | Tornar Profile recurso persistente; validação e undo por edição de ponto | Cap. 02 |
| MO-03 | Draw-on-Face | `RUDIMENTARY` | Picking/projeção e workflows próximos existem, sem attachment persistente compartilhado | Construir sobre P3D-158 | Cap. 02; P3D-158 |
| MO-04 | Revolve e Sweep | `PARTIALLY_COMPLIANT` | Algoritmos e fluxo básico existem; caminho/spline não é recurso canônico compartilhado | Reusar P3D-161; controles de twist/taper/caps e diagnóstico | Cap. 02; P3D-161 |
| MO-05 | Representação topológica autoral | `FUNCTIONAL_BUT_DIFFERENT` | Operadores trabalham principalmente sobre vetores/índices/sets; half-edge é auxiliar | Evoluir somente onde invariantes/complexidade comprovarem ganho | Caps. 29 e 34 |
| MO-06 | ToolRegistry MODEL de produção | `OBSOLETE` | Registry é consumido principalmente pelo frontend egui legado; Slint tem outro fluxo | Congelar, documentar e remover com aposentadoria do legado | AGENTS §0.1 |
| PA-01 | Brush/eraser/picker/fill/decal/gradient | `PARTIALLY_COMPLIANT` | Ferramentas existem; sem uniformidade transacional e de spacing | Um engine de stroke compartilhado para 2D/3D/simetria | Cap. 14; specs PAINT |
| PA-02 | Stroke independente da taxa de eventos | `BROKEN` | Slint interpola em passos fixos de tela/pixel e ignora o `spacing` canônico em caminhos principais | Amostrar por distância no espaço correto via `BrushSettings::stroke_dabs` | Cap. 14; qualidade de input |
| PA-03 | Pilha de camadas | `PARTIALLY_COMPLIANT` | Camadas, grupos, efeitos, opacity/lock/visibility existem | Corrigir undo e recomposição incremental; definir máscaras/canais V1 | Cap. 14 |
| PA-04 | Composição e upload incrementais | `BROKEN` | Tiles sujos existem, mas stack/texturas integrais são clonados e upload é integral após hash completo | Dirty rect/tile até compositor e GPU | Caps. 14 e 19 |
| PA-05 | Multicanal de material | `RUDIMENTARY` | Material armazena propriedades, mas pintura efetiva concentra-se em albedo/canvas RGBA | Só expandir após pipeline de composição/revisões; explicitar escopo V1 | Cap. 14; specs de materiais |
| UV-01 | Algoritmos UV básicos | `COMPLIANT` | Unwrap xatlas, seams, pins, transform, stitch, relax e density têm implementação/testes | Preservar como módulo de serviço | P3D-063 |
| UV-02 | Workspace UV independente | `OBSOLETE` | Pill/workspace/editor dedicado seguem visíveis | Mover acesso para utilitário de PAINT; preservar engine | Cap. 36 addendum; P3D-063 |
| UV-03 | Seleção UV independente | `PARTIALLY_COMPLIANT` | Sessão mantém seleção UV, mas `Face.selected` também é reutilizado | Identidade de UV vertex/island separada da seleção topológica | P3D-063 |
| UV-04 | Escalabilidade do editor UV | `RUDIMENTARY` | Modelo projeta geometria para strings SVG e sincronização ampla | Renderização batched/retained e culling de ilhas; medir arquivos grandes | Cap. 19; P3D-063 |
| HA-01 | Spline Core compartilhado | `RUDIMENTARY` | Há Bezier/sweep/RMF locais, mas não recurso serializável com IDs, revisões e comandos | Implementar P3D-161 antes de Hair | P3D-161 |
| HA-02 | SurfaceAttachment | `MISSING` | Não há contrato compartilhado triângulo+bariêntricas com invalidation/reattach | Implementar P3D-158 antes de draw-on-surface/Hair | P3D-158 |
| HA-03 | Hair Clump low-poly | `MISSING` | Nenhum generator/asset Hair alcançável | Roadmap incremental deste dossiê | Cap. 38 |
| HA-04 | Bake procedural explícito | `RUDIMENTARY` | Há conceitos de Make Editable/avaliação, sem fluxo Hair | Usar P3D-160 com undo e confirmação de custo | P3D-160; cap. 38 |

## 3. Reconciliações documentais aplicadas

### UV

O capítulo 36 contém linguagem histórica sobre `MODEL / PAINT / UV`, mas o
addendum datado mais recente, repetido em `P3D-063`, remove a pill UV durante o
congelamento de PAINT e mantém o módulo como utilitário interno. Pela regra de
autoridade temporal explícita dentro da própria página, a superfície UV
independente foi classificada como `OBSOLETE`; os algoritmos UV não foram.

### Viewport

Há referências especializadas a Material Preview/Rendered em páginas e no código,
mas o capítulo 05 define Textured/Solid/Wireframe/Silhouette-Reference e declara
que o produto não deve copiar o Rendered do Blender. Esta auditoria não escolhe
silenciosamente a implementação atual: registra `FUNCTIONAL_BUT_DIFFERENT` e
recomenda reconciliação documental antes de ampliar o renderer.

### Hair

Os algoritmos locais de curva e sweep são evidência útil, mas não satisfazem
`P3D-161`: falta uma entidade compartilhada, serialização, IDs estáveis,
revisões, comandos e attachment. Por isso Spline Core é `RUDIMENTARY`, não
`PARTIALLY_COMPLIANT`, e Hair permanece `MISSING`.
