# Remediação G3 — fundação headless de Spline Core

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** fundação persistente e transacional de `P3D-161`, sem superfície
> de produto pós-V1
>
> **Validação:** executada somente após código, provas e documentação do
> conjunto estarem completos

## 1. Autoridade, sequência e limite do gate

Este checkpoint é o quarto pacote da remediação operacional (`G0` a `G3`) e
corresponde à primeira fundação do roadmap de Shape-first/Hair. O nome `G3` não
altera a numeração das fases `S0–S4` e `H0–H8` do roadmap.

Fontes canônicas aplicadas:

- `docs/bible/specs/p3d-161-spline-core.md`;
- `docs/bible/foundations/40-pos-v1-spline-core-procedural-path-generators.md`;
- `docs/bible/foundations/38-pos-v1-low-poly-hair-mesh-morphs-character.md`;
- `docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`;
- `docs/bible/foundations/34-arquitetura-modular-rust-safety.md`.

O gate entrega o núcleo headless necessário para os próximos passos, mas não
declara `P3D-161` integralmente concluído. `SurfaceAttachment`, edição visual,
Profile persistente, generators e Bake to Mesh continuam em gates próprios.
Nenhum controle foi acrescentado ao Slint e nenhuma feature foi criada no egui
legado.

## 2. Implementation-vs-Spec Gap Matrix do G3

| ID | Estado anterior | Delta implementado | Estado após o pacote |
| :--- | :---: | :--- | :---: |
| HA-01 | `RUDIMENTARY` | `SplineResource` persistente com IDs, revisões, polyline/Bézier, open/closed, handles, avaliação e comandos | `PARTIALLY_COMPLIANT` com `P3D-161` completo somente em seu núcleo headless sem attachment/UI/Bake mesh |
| SPL-DATA | `MISSING` | Recurso e pontos serializáveis, IDs estáveis, validação, reverse e domínio de revisão próprio | `COMPLIANT` |
| SPL-EVAL | `DUPLICATED`/`RUDIMENTARY` | Comprimento de arco, tangentes, resampling e parallel transport compartilhado com Sweep | `COMPLIANT` |
| SPL-CACHE | `MISSING` | Cache transitório por ID, revisão, fingerprint geométrico, tolerância e spacing | `COMPLIANT` |
| SPL-CMD | `MISSING` | Create/Delete/Add/Move/Delete Point/Handles/Open-Close/Reverse/Convert pelo dispatcher e Undo/Redo | `COMPLIANT` para API headless |
| SPL-SNAP | `MISSING` | Snap determinístico para grid e pontos, com settings validados | `PARTIALLY_COMPLIANT`; edge/surface snap depende das queries e de attachment |
| SPL-ATTACH | `MISSING` | Nenhuma persistência improvisada foi adicionada sobre o tipo transitório existente | `MISSING`; próximo gate é `P3D-158` |
| SPL-BAKE | `MISSING` | Conversão amostrada para polyline existe | `PARTIALLY_COMPLIANT`; Bake to Mesh pertence ao operador/generator que fornecer profile e regras |
| MO-04 | `PARTIALLY_COMPLIANT` | Sweep deixa de manter uma segunda implementação de RMF | `PARTIALLY_COMPLIANT`; ainda não referencia `SplineId + ProfileId` persistentes |
| S0-PROFILE | `MISSING` | Nenhuma migração prematura do `ProfileState` transitório | `MISSING`; deve consumir esta fundação em gate posterior |
| HA-03 | `MISSING` | A matemática necessária a guides passa a existir sem criar Hair paralelo | `MISSING`; H0 só inicia após `P3D-158` e generator evaluation |

## 3. Arquitetura entregue

### 3.1 Documento e ownership

`petunia_project` passa a possuir `SplineResource` e `SplinePoint`. O recurso é
authoring data, não mesh nem estado de widget. Samples, tabelas e frames não são
serializados; vivem em `SplineEvaluationCache`, que pode ser descartado e
reconstruído a qualquer momento.

`ProjectChanges::SPLINES` e `spline_revision` isolam a invalidação de paths das
revisões de topologia, posições, UV, cores, materiais e texturas. Comandos de
spline não emitem `MeshChanged` e não forçam reconstrução das meshes atuais.

### 3.2 Geometria comum para Sweep e Hair futuro

`petunia_mesh::compute_parallel_transport_frames` é a implementação única de
frames por double reflection em precisão `f64`. O Sweep existente converte sua
entrada `Vec3` na fronteira e reutiliza o mesmo algoritmo que o cache de spline.

Casos degenerados procuram vizinhos distintos sem mudar a cardinalidade do
path. Loops recebem compensação distribuída de twist. A baseline evita Frenet
frames e, portanto, a instabilidade conhecida em trechos quase retos.

### 3.3 Avaliação e cache

O pipeline derivado é:

```text
SplineResource
  → tesselação adaptativa de Bézier ou polyline
  → ArcLengthTable
  → samples uniformes por distância
  → parallel-transport frames
  → consumidor futuro (Sweep/Profile/Hair/generator)
```

As chaves incluem revisão e fingerprint somente dos control points/handles. O
fingerprint evita reutilização obsoleta quando Undo cria uma ramificação com a
mesma revisão local; seu custo é `O(control points)`, não `O(samples)` nem
`O(output mesh)`. Mudança de nome não invalida avaliação geométrica.

### 3.4 Transações

Os comandos tipados usam o `CommandDispatcher` como único owner de rollback,
checkpoint, dirty state e publicação de revisão. `can_execute` rejeita recurso
ausente, índice/ID inválido, valor não finito e no-op conhecido antes de clonar o
documento. Falha de avaliação restaura o snapshot e não entra no histórico.

Os comandos não foram registrados no catálogo visual: `P3D-161` é pós-V1 e o
contrato de interação/`TextId`/`IconId` ainda precisa de gate próprio. Expor
labels literais agora criaria superfície pública antes da especificação de UX.

## 4. Persistência e compatibilidade

O schema `.petunia` V1 recebe `splines` e `spline_revision` como campos aditivos
com defaults vazio/zero. Projetos ZIP/JSON sem esses campos abrem sem migração
manual.

O layout postcard legado é posicional. Por isso os campos novos foram anexados
após todos os campos V1 existentes, em vez de inserir `splines` próximo de
`assets`. Uma fixture serializa explicitamente o layout anterior e exige que o
loader atual restaure assets e aplique os defaults de spline. O número de versão
permanece `1`, pois a mudança é retrocompatível e não altera semântica dos
campos existentes.

Snapshots de Undo passam a incluir capacidade dos recursos, nomes e control
points no orçamento profundo do projeto.

## 5. Segurança, limites e determinismo

- posições, handles, parâmetros, tolerância, spacing e settings de snap exigem
  valores finitos;
- uma avaliação é limitada a 262.144 pontos para impedir explosão de memória em
  tolerância/spacing hostil;
- a recursão adaptativa de Bézier tem profundidade máxima fixa;
- paths inteiramente degenerados falham de forma recuperável;
- caches são derivados e não entram em save, Undo ou schema;
- IDs duplicados são rejeitados na API autoral e normalizados somente na
  fronteira defensiva de carregamento;
- save/load preserva IDs, handles, interpolação e fechamento;
- os frames são determinísticos para a mesma entrada.

## 6. Provas automatizadas executadas

As provas do conjunto cobrem:

- comprimento de arco e resampling uniforme;
- avaliação/tangente Bézier e reverse geometricamente equivalente;
- modos Broken/Aligned/Mirrored;
- linha reta, curva em S, corner agudo, loop fechado, pontos repetidos, vertical
  quase paralela ao guide inicial e escalas mistas;
- invalidação de cache por revisão e por conteúdo após restore;
- limites de avaliação e rejeição de `NaN`/spacing patológico;
- snap para ponto/grid e settings inválidos;
- round-trip do recurso e do projeto `.petunia`;
- abertura de JSON e postcard anteriores aos campos de spline;
- orçamento profundo do histórico;
- comando `do → undo → redo`, uma entrada por ação, revisão somente de spline e
  no-op sem histórico;
- regressão do Sweep sobre a implementação compartilhada de frames.

## 7. Próximo gate autorizado

O próximo passo do roadmap é `P3D-158 SurfaceAttachment`, não Hair UI. Ele deve
substituir/reconciliar o attachment transitório de viewport por um contrato
persistente compartilhado com estado `Valid/NeedsReattach/MissingTarget`, sem
reprojeção silenciosa. Depois disso, o Profile persistente e o Sweep por IDs
podem consumir `SplineResource`; somente então H0 (`Curve → Ribbon Mesh`) fica
tecnicamente seguro.

## 8. Gates de encerramento

| Gate | Resultado |
| :--- | :---: |
| `cargo fmt --all -- --check` | PASSOU |
| `cargo check --workspace` | PASSOU |
| 818 testes de Mesh/Project/Core/MODEL/PAINT/Slint | PASSARAM |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSOU |
| `cargo run -p xtask -- docs-check` | PASSOU |
| `cargo run -p xtask -- bible-check` | PASSOU |
| `cargo run -p xtask -- ui-guard --strict` | PASSOU |

Contagem final: `petunia_mesh` 135, `petunia_project` 119, `petunia_core`
187, `petunia_module_model` 1, `petunia_module_paint` 30 e
`petunia_ui_slint` 346. Os comandos Cargo usaram
`RUSTUP_TOOLCHAIN=1.98.1`.

O `docs-check` confirmou as 250 páginas do caderno, 406 links internos, 813
arquivos do site público congelado e changelogs sincronizados. Os 357 símbolos
públicos ainda fora do mapa de componentes permanecem inventário não bloqueante.
O `ui-guard` não encontrou violação de confinamento; os 302 candidatos listados
continuam limitados à UI egui legada e já estavam documentados.
