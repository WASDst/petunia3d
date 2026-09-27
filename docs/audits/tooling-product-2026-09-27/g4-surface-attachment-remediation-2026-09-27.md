# Remediação G4 — fundação headless de SurfaceAttachment

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** representação persistente e transacional de `P3D-158` para pontos
> de spline, sem superfície de produto pós-V1
>
> **Validação:** executada somente após código, provas e documentação do
> conjunto estarem completos

## 1. Autoridade, sequência e limite do gate

Este checkpoint é o quinto pacote da remediação operacional (`G0` a `G4`) e
implementa a fundação que o roadmap de Shape-first/Hair chama de
`SurfaceAttachment headless`. A numeração operacional inclui os gates de PAINT
executados antes das fundações pós-V1; ela não renumera as fases `S0–S4` e
`H0–H8` nem os gates conceituais do roadmap.

Fontes canônicas aplicadas:

- `docs/bible/specs/p3d-158-surface-attachment-foundation.md`;
- `docs/bible/specs/p3d-161-spline-core.md`;
- `docs/bible/foundations/38-pos-v1-low-poly-hair-mesh-morphs-character.md`;
- `docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`;
- `docs/bible/foundations/34-arquitetura-modular-rust-safety.md`.

O gate entrega authoring data, queries geométricas e Commands headless. Não
expõe novos controles no Slint, não amplia o egui legado, não migra Decal ainda
e não declara transforms parent/child implementados quando o modelo atual de
`Asset` aplica transformações diretamente à geometria.

## 2. Implementation-vs-Spec Gap Matrix do G4

| ID | Estado anterior | Delta implementado | Estado após o pacote |
| :--- | :---: | :--- | :---: |
| HA-02 | `MISSING` | Attachment persistente por target/triângulo/bariêntricas, estados explícitos, queries e Commands | `PARTIALLY_COMPLIANT`; fundação headless concluída para authoring mesh e pontos de spline, consumidores/UI restantes pendentes |
| ATT-DATA | `MISSING` | `SurfaceTriangleHandle`, bariêntricas, offset normal, rotação tangente, revisão/fingerprint e último ponto mundial | `COMPLIANT` |
| ATT-STATE | `MISSING` | `Valid`, `NeedsReattach`, `MissingTarget` e `Invalid`, sem reprojeção silenciosa | `COMPLIANT` |
| ATT-QUERY | `RUDIMENTARY`/`DUPLICATED` | Raycast determinístico, avaliação de frame/UV, slide, rotate, reproject e detach keep-world em um contrato de projeto | `COMPLIANT` para authoring mesh |
| ATT-CMD | `MISSING` | Attach/Detach/Slide/Reproject/Rotate de ponto de spline via dispatcher, rollback e Undo/Redo | `COMPLIANT` para API headless |
| ATT-PERSIST | `MISSING` | Campo aditivo opcional em `SplinePoint`, round-trip `.petunia` e default de JSON anterior | `COMPLIANT` |
| ATT-TRANSFORM | `MISSING` | Nenhum pseudo-transform foi introduzido | `MISSING`; `Asset` ainda não possui hierarchy/world transform persistente |
| ATT-EVALUATED | `MISSING` | Fingerprint usa a triangulação autoral usada por renderer/picking | `PARTIALLY_COMPLIANT`; attachments ainda não referenciam output avaliado de modifiers |
| ATT-CONSUMERS | `MISSING` | `Project::resolved_spline` torna o primeiro consumidor canônico e conversão resolve attachments antes de bake | `PARTIALLY_COMPLIANT`; Decal, Path Paint, accessories e Hair ainda não foram migrados |
| HA-01 | `PARTIALLY_COMPLIANT` | `SplinePoint` passa a aderir a superfície e bloqueia move mundial ambíguo | `PARTIALLY_COMPLIANT`; overlay/editor Slint e Profile/generators continuam pendentes |
| HA-03 | `MISSING` | Root/guide agora têm as duas fundações de dados necessárias, sem criar um Hair paralelo | `MISSING`; H0 continua condicionado a Profile/generator/overlay |

## 3. Representação e ownership

`petunia_project` é o único owner do contrato persistente. O attachment contém
o UUID do alvo, o índice da face e do triângulo real de render/picking,
coordenadas baricêntricas, offset normal e rotação no plano tangente. UV é um
resultado derivado por face-corner e não faz parte da âncora geométrica.

`source_topology_revision` preserva contexto de diagnóstico e
`source_topology_fingerprint` decide validade estrutural. Usar somente a revisão
global invalidaria attachments quando qualquer outro asset mudasse; usar somente
contagens deixaria conectividade obsoleta passar. O fingerprint inclui índices
de faces e a decomposição efetiva em triângulos, mas ignora seleção, cor, UV e
movimentos que preservam essa decomposição.

O antigo tipo transitório de `viewport_query` foi removido. Core apenas reexporta
o contrato de projeto e mantém `AttachmentValidity` como alias de compatibilidade,
eliminando dois donos para o mesmo conceito.

## 4. Semântica geométrica

O pipeline de consulta é:

```text
ray mundial finito
  → assets autorais visíveis e desbloqueados
  → triangulação compartilhada com renderer/picking
  → Möller–Trumbore + bariêntricas
  → handle persistente + fingerprint
  → frame posição/normal/tangente/bitangente + UV opcional
```

Tangentes usam derivadas UV quando o mapa é não degenerado e caem para uma
aresta geométrica projetada no plano. `tangent_rotation` gira o frame ao redor
da normal. `normal_offset` desloca somente o resultado derivado. Em UV seams, a
face e o triângulo identificam explicitamente o lado, enquanto a posição
geométrica permanece baricêntrica.

Uma mudança somente de posições reavalia o mesmo triângulo. Uma alteração de
conectividade ou da triangulação autoral produz `NeedsReattach`; alvo removido
produz `MissingTarget`. Nos dois casos, `detach_keep_world` preserva o último
ponto mundial conhecido, mas nenhuma API escolhe outra superfície sem ação
explícita.

## 5. Transações e integração com Spline Core

Cinco Commands tipados cobrem attach, detach, slide, reproject explícito e
rotação. Todos declaram `ProjectChanges::SPLINES`; o dispatcher continua dono
único de snapshot, rollback, histórico, dirty state e revisão. Rays inválidos,
targets ausentes/bloqueados, attachment obsoleto e no-op são recusados antes de
criar histórico sempre que podem ser conhecidos por `can_execute`.

`MoveSplinePointCmd` não desloca mais silenciosamente o fallback de um ponto
aderido enquanto a avaliação continua em outra posição. O usuário/consumer deve
usar slide/reproject ou detach. `ConvertSplineToPolylineCmd` resolve os pontos
aderidos contra o projeto antes de amostrar e remove os attachments do resultado
baked, evitando conversão a partir de posições fallback antigas.

## 6. Persistência e compatibilidade

`SplinePoint.attachment` é um campo `Option` aditivo com `serde(default)`. JSON
de spline anterior ao G4 carrega os pontos como detached. O round-trip do
projeto `.petunia` preserva target, handle, pesos, offsets e último ponto
mundial.

Dados não finitos ou pesos fora do simplex são rejeitados na API autoral e
descartados somente na fronteira defensiva de normalização de arquivo. Estado
derivado (`SurfaceAttachmentStatus` e `SurfaceFrame`) não é serializado.

## 7. Performance, segurança e limites conscientes

- o raycast não aloca uma malha triangulada intermediária e mantém desempate
  determinístico pela ordem autoral;
- o raycast ainda é linear no total de faces; overlay/draw-on-surface deve
  introduzir cache/BVH por revisão antes de consultas contínuas em cenas grandes;
- o fingerprint é `O(faces + corners)` e roda em operações explícitas de
  authoring; consumidores por frame devem cacheá-lo por revisão antes de Hair;
- queries rejeitam `NaN`, infinito, direção degenerada e distância não positiva;
- UV auxiliar não finito é descartado sem contaminar o frame geométrico;
- faces/índices inválidos não causam acesso fora de faixa na triangulação;
- assets bloqueados não recebem attach/slide/reproject; ocultar depois de attach
  não destrói a referência já existente;
- attachment usa a mesh autoral, não `evaluated_mesh`; aderir a outputs de
  modifiers exige IDs/remap estáveis e não será simulado por índice frágil;
- transform parent/child permanece `MISSING`, porque o projeto ainda não possui
  o contrato persistente necessário. Geometria transformada em-place funciona,
  mas não satisfaz esse item do DoD canônico.

## 8. Provas automatizadas do conjunto

As provas adicionadas cobrem:

- ray/triangle com reconstrução baricêntrica e rejeição de não finitos;
- fingerprint estável sob posição/cor/UV/seleção e sensível à conectividade;
- raycast, soma dos pesos, posição, normal, tangente e UV;
- deformação somente de posições e revisão global de outro domínio sem falsa
  invalidação;
- `NeedsReattach`, `MissingTarget` e detach com fallback mundial;
- reprojeção determinística com offset/rotação preservados;
- lados distintos de UV seam por face-corner;
- visibilidade, lock e rays inválidos;
- round-trip `.petunia` e JSON anterior sem campo attachment;
- attach/rotate/slide/detach/reproject com histórico transacional e no-op sem
  checkpoint;
- bloqueio de move mundial ambíguo e conversão de spline pela posição resolvida.

## 9. Próximo gate autorizado

O próximo gate é `S0/S3`: Profile persistente e Sweep referenciando recursos por
ID, com generator evaluation/cache e Bake/Make Editable transacional. Ele deve
reusar `SplineResource`, `SurfaceAttachment` e os frames comuns existentes.

Overlay/picking Slint de curvas vem depois do contrato headless do Profile. H0
(`Curve → Ribbon Mesh`) só inicia quando o generator puder referenciar
Spline/Profile persistentes e publicar output derivado sem transformar UI em
owner de geometria.

## 10. Gates de encerramento

| Gate | Resultado |
| :--- | :---: |
| `cargo fmt --all -- --check` | PASSOU |
| `cargo check --workspace` | PASSOU |
| 830 testes de Mesh/Project/Core/MODEL/PAINT/Slint | PASSARAM |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSOU |
| `cargo run -p xtask -- docs-check` | PASSOU |
| `cargo run -p xtask -- bible-check` | PASSOU |
| `cargo run -p xtask -- ui-guard --strict` | PASSOU |

Contagem final: `petunia_mesh` 137, `petunia_project` 126, `petunia_core`
190, `petunia_module_model` 1, `petunia_module_paint` 30 e
`petunia_ui_slint` 346. Os comandos Cargo usaram
`RUSTUP_TOOLCHAIN=1.98.1`.

Após a revisão final de robustez, os 316 testes de `petunia_project` e
`petunia_core` foram repetidos sobre o diff definitivo e passaram novamente.

O `docs-check` confirmou as 250 páginas do caderno, 406 links internos, 813
arquivos do site público congelado e changelogs sincronizados. Os 357 símbolos
públicos ainda fora do mapa de componentes permanecem inventário não bloqueante.
O `ui-guard` não encontrou violação de confinamento; os 302 candidatos listados
continuam limitados à UI egui legada e já estavam documentados.
