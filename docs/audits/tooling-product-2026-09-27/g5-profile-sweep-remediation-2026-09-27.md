# Remediação G5 — Profile persistente, Sweep derivado e Bake transacional

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** fundação headless de `S0/S3`, `P3D-160` e `P3D-168`, sem ampliar
> a superfície de produto Slint antes do gate de overlay/authoring visual
>
> **Validação:** executada somente após código, provas e documentação do
> conjunto estarem completos

## 1. Autoridade e limite do gate

Este checkpoint sucede a fundação de `SurfaceAttachment` e implementa o próximo
delta comprovado do roadmap Shape-first/Hair: Profile deixa de existir apenas
como estado transitório de ferramenta e passa a ter uma representação
persistente, enquanto Sweep ganha um gerador vivo referenciado por IDs, cache
derivado e Bake/Make Editable atômico.

Fontes canônicas aplicadas:

- `docs/bible/specs/p3d-160-bake-flatten-derived-asset-contract.md`;
- `docs/bible/specs/p3d-161-spline-core.md`;
- `docs/bible/specs/p3d-168-procedural-path-generators.md`;
- `docs/bible/foundations/02-workflow-modelagem-shape-first.md`;
- `docs/bible/foundations/40-pos-v1-spline-core-procedural-path-generators.md`;
- `docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`;
- `docs/bible/foundations/34-arquitetura-modular-rust-safety.md`.

O gate é deliberadamente headless. O editor Profile já exposto no Slint continua
transitório e não foi conectado parcialmente ao novo contrato, pois dois owners
simultâneos de authoring seriam piores que uma migração explícita no gate visual.
O egui legado não recebeu expansão.

## 2. Implementation-vs-Spec Gap Matrix do G5

| ID | Estado anterior | Delta implementado | Estado após o pacote |
| :--- | :---: | :--- | :---: |
| S0-DATA | `MISSING` | `ProfileResource` persistente referencia uma `SplineResource` planar fechada e preserva workplane, espessura e revisão | `COMPLIANT` para domínio headless |
| S0-EDIT | `PARTIALLY_COMPLIANT` | Pontos do Profile reutilizam IDs e Commands de Spline; criação/deleção do par Profile+Spline é transacional | `PARTIALLY_COMPLIANT`; constraints e editor visual persistente permanecem pendentes |
| S0-PERSIST | `MISSING` | JSON `.petunia`, normalização defensiva e migração postcard intermediária | `COMPLIANT` |
| S0-UI | `FUNCTIONAL_BUT_DIFFERENT` | Nenhuma ligação parcial foi adicionada | `FUNCTIONAL_BUT_DIFFERENT`; `ProfileState` Slint ainda é transitório e será substituído no gate visual |
| S1-DIAG | `RUDIMENTARY` | Diagnósticos tipados publicam vértices, triângulos, orçamento, dependências e warnings de preview/caps | `PARTIALLY_COMPLIANT`; winding, self-intersection e constraints visuais continuam pendentes |
| S3-REFS | `MISSING` | `PathGenerator` referencia `ProfileId + SplineId + parameters` | `COMPLIANT` para Sweep headless |
| S3-EVAL | `RUDIMENTARY` | Polyline preserva corners; Bézier usa arc-length; RMF/miter/caps reutilizam `petunia_mesh`; preview usa spacing reduzido | `COMPLIANT` para os parâmetros implementados |
| S3-CACHE | `MISSING` | Cache inclui conteúdo do generator, Profile, curva de perfil e path resolvido, inclusive attachments | `COMPLIANT` |
| S3-BAKE | `MISSING` | Avalia final, cria `Asset` estático e remove somente o generator em uma transação com Undo/Redo | `COMPLIANT` para execução síncrona limitada por orçamento |
| S3-PARAM | `RUDIMENTARY` | Sampling, tolerance, preview multiplier, caps, miter, miter limit e vertex budget são persistentes | `PARTIALLY_COMPLIANT`; twist, taper, segment override e Revolve persistente não foram simulados |
| P3D-160 | `PARTIALLY_COMPLIANT` | Bake de generator publica custo e equivalência live/baked; recursos compartilhados permanecem reutilizáveis | `PARTIALLY_COMPLIANT`; job assíncrono, cancelamento e conflito de revisão para bake pesado seguem pendentes |
| P3D-168 | `RUDIMENTARY` | Primeira primitiva oficial, Sweep Profile Along Path, passa a ser persistente, determinística e bakeável | `PARTIALLY_COMPLIANT`; Repeat, Stretch e End/Corner Rules permanecem `MISSING` |
| HA-03 | `MISSING` | Hair agora pode depender de Profile/Path/Generator estáveis sem criar outro sistema de curvas | `MISSING` como produto; H0 ainda depende do overlay Slint e de um generator Ribbon dedicado |
| A11Y/UI | `MISSING` neste fluxo | Nenhuma UI incompleta ou inacessível foi publicada | `MISSING`; foco, teclado, feedback de seleção e alto contraste pertencem ao próximo gate Slint |

## 3. Representação e ownership

O contrato persistente vive exclusivamente em `petunia_project`:

```text
ProfileResource
  ├─ ProfileId
  ├─ SplineId planar fechado (ownership exclusivo)
  ├─ workplane ortonormal persistente
  ├─ wall_thickness
  └─ revision

PathGenerator
  ├─ GeneratorId
  ├─ ProfileId
  ├─ Path SplineId
  ├─ SweepGeneratorParameters
  └─ revision
```

Profile não duplica control points. A curva planar é a mesma
`SplineResource` usada pelo restante da arquitetura, logo add/move/delete,
handles, reverse, save/load e Undo continuam com um único contrato. O Profile
possui a spline de seção de modo exclusivo; o path é um recurso compartilhado.
Deleções que deixariam referências pendentes são recusadas no domínio e nos
Commands.

O workplane é validado e ortonormalizado por Gram-Schmidt, preservando a
orientação da normal. Pontos e handles do Profile precisam permanecer no plano
local `Z=0`, o loop precisa estar fechado e attachments de superfície são
recusados no perfil. Paths continuam aceitando `SurfaceAttachment` e são
resolvidos antes da avaliação.

`wall_thickness` é persistido para a evolução shape-first, mas Sweep recusa
espessura positiva neste gate. Isso evita tratar dois contornos concatenados
como um polígono simples e produzir caps/topologia incorretos silenciosamente.

## 4. Avaliação geométrica e cache

O pipeline derivado é:

```text
GeneratorId
  → valida ProfileId/SplineId e parâmetros
  → resolve attachments do path
  → preserva pontos exatos de polylines
  → resample arc-length de Bézier conforme Preview/Final
  → estima vértices e aplica orçamento antes de gerar mesh
  → valida triangulação do perfil quando caps são pedidos
  → Sweep com parallel-transport frames + miter limitado
  → Mesh derivada + diagnostics
```

O cache não depende apenas de revisions públicas. A chave inclui fingerprints
de conteúdo para permanecer correta diante de carga de arquivo, edição direta
defensiva e mudança de geometria do target de um attachment. No último caso, o
path resolvido muda mesmo que a spline autoral preserve seus pontos fallback.

Preview multiplica os spacings configurados para Bézier; Final usa os valores
autorais. Polylines não são artificialmente subdivididas, preservando corners
agudos para o miter. O resultado usa a implementação RMF compartilhada em
`petunia_mesh`, sem um segundo sistema de frames dentro do generator.

## 5. Performance e limites de segurança

- o orçamento padrão é 262.144 vértices e o teto global é 1.000.000;
- multiplicação de contagens usa overflow check antes de alocação da mesh;
- coordenadas `f64` que não cabem em `f32` são recusadas na fronteira de mesh;
- spacing, tolerance, preview multiplier e miter limit rejeitam zero, `NaN` e
  infinito;
- caps pedidos exigem triangulação válida antes da criação das laterais;
- cache hit não reconstrói samples, frames ou mesh;
- snapshots de Undo contabilizam nomes e os novos vetores persistentes;
- fingerprints são lineares no número de control points e adequados a authoring,
  mas não substituem um dependency graph para milhares de generators;
- Bake atual é síncrono e limitado pelo orçamento. Bakes maiores devem migrar
  para snapshot/job com cancelamento e commit condicionado à revisão antes de o
  teto ser ampliado.

## 6. Persistência e compatibilidade

Os campos `procedural_revision`, `profiles` e `path_generators` foram anexados
depois de `splines`, mantendo a disciplina append-only do schema. JSON anterior
usa defaults vazios.

Postcard é posicional e não pode depender de `serde(default)` para campos finais
ausentes. O loader agora reconhece explicitamente três layouts:

1. layout atual com Profile/Generator;
2. layout intermediário com Spline persistente, anterior a este gate;
3. layout legado anterior a Spline.

Novas gravações continuam usando o ZIP JSON canônico; as variantes postcard são
somente fronteiras de leitura/migração.

## 7. Commands e semântica transacional

Foram adicionados Commands tipados para:

- criar Profile e sua spline planar em uma única entrada;
- excluir Profile e sua spline owned sem deixar dependências;
- criar/excluir PathGenerator;
- atualizar parâmetros de Sweep com no-op recusado;
- Bake/Make Editable do generator.

O dispatcher continua sendo o único owner de snapshot, rollback, histórico,
dirty state e publicação de revisões. `ProjectChanges::PROCEDURAL` separa
invalidação de authoring derivado da topologia estática. Operações somente de
curvas/procedural recebem feedback `CurveEdit`; Bake publica também `GEOMETRY`
e invalida a mesh ativa.

Bake preserva Profile e Path porque são recursos reutilizáveis, remove apenas o
generator vivo e cria um `Asset` convencional. Undo restaura exatamente o
generator e remove a malha baked; Redo repete o estado materializado.

## 8. Provas automatizadas do conjunto

As provas adicionadas cobrem:

- normalização/handedness do workplane;
- rejeição de Profile aberto, não planar ou attached;
- Sweep determinístico com path aberto, corner e caps;
- IDs de dependência e contagens de diagnóstico;
- hit/miss e invalidação por edição do path;
- invalidação quando a superfície de um path attached deforma;
- rejeição por orçamento antes da geração;
- round-trip ZIP JSON de Profile/Generator e avaliação após load;
- migração postcard anterior a Spline e do layout intermediário com Spline;
- defaults de JSON anterior aos campos procedurais;
- criação, update, no-op, bloqueio de deleção, Bake, Undo e Redo;
- equivalência entre mesh live final e asset baked.

## 9. Limitações explícitas e próximo gate

O gate não declara S0/S3 completos como produto. Permanecem:

- substituir `ProfileState` transitório do Slint por seleção de `ProfileId`;
- overlay/picking de curva e handles no viewport Slint;
- foco, teclado, narration/status acessível e feedback de hover/seleção;
- diagnostics visuais de winding, caps, custo e dependências removidas;
- twist, taper, override de segments e Revolve persistente;
- execução assíncrona para Bake acima do orçamento interativo;
- Repeat/Stretch/End-Corner Rules de `P3D-168`.

O próximo gate autorizado é o overlay/editor Slint de curvas e a migração
controlada do Profile visual para IDs persistentes. H0 (`Curve → Ribbon Mesh`)
vem depois dessa superfície, reutilizando o mesmo cache e contrato de Bake.

## 10. Gates de encerramento

| Gate | Resultado |
| :--- | :---: |
| `cargo fmt --all -- --check` | PASSOU |
| `cargo check --workspace` | PASSOU |
| 839 testes de Mesh/Project/Core/MODEL/PAINT/Slint | PASSARAM |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSOU |
| `cargo run -p xtask -- docs-check` | PASSOU |
| `cargo run -p xtask -- bible-check` | PASSOU |
| `cargo run -p xtask -- ui-guard --strict` | PASSOU |

Contagem final: `petunia_mesh` 137, `petunia_project` 134, `petunia_core`
191, `petunia_module_model` 1, `petunia_module_paint` 30 e
`petunia_ui_slint` 346. Os comandos Cargo usaram
`RUSTUP_TOOLCHAIN=1.98.1`.

O primeiro ciclo afetado detectou apenas um fixture novo que criava
`Project::default()` sem asset-alvo antes de testar attachment; o fixture foi
corrigido para declarar a superfície explicitamente e os 325 testes de Project
e Core foram repetidos integralmente sobre o diff final.

O `docs-check` confirmou as 250 páginas do caderno, 406 links internos, 813
arquivos do site público congelado e changelogs sincronizados. Os 357 símbolos
públicos ainda fora do mapa de componentes permanecem inventário não bloqueante.
O `ui-guard` não encontrou violação de confinamento; os 302 candidatos listados
continuam limitados à UI egui legada e já estavam documentados.
