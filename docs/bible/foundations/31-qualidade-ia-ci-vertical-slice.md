# 31 — Qualidade, IA, CI e Vertical Slice de Conformance da Stack Rust

<aside>
🧪

A stack Rust e a UI Baseline V1 estão congeladas; este capítulo define a **fatia vertical de conformance** que deve provar, em implementação real, que os contratos dos capítulos 27–36 funcionam juntos com qualidade mensurável. Desenvolvimento assistido por IA continua sendo processo verificável, não geração de código sem feedback.

</aside>

# Objetivo

A principal vantagem esperada de Rust/egui/wgpu não é apenas segurança ou performance: é permitir um ciclo rápido e verificável para **um desenvolvedor + agentes de IA**, especialmente na UI — área que já causou maior dificuldade prática no projeto.

# Ferramentas obrigatórias

Baseline de qualidade:

```
cargo fmt
cargo check
cargo clippy
cargo test
cargo xtask verify
proptest
cargo-fuzz
Criterion
tracing
license/dependency audit
```

Warnings relevantes do projeto devem falhar no CI. Não usar `allow` global para silenciar uma classe inteira de problemas sem rationale.

# Pirâmide de testes Rust

## Unit

Math, IDs, Profile helpers, topology primitives, serializers, brush kernels.

## Command tests

Cada command mutável deve verificar resultado, invariants, Undo, Redo, rollback e IDs/remaps.

## Property tests

Gerar inputs/sequências e validar invariantes topológicas e documentais.

## Provider contracts

Manifold/xatlas/import/export passam por suites independentes dos vendors.

## Golden

Project serialization, migrations, triangulation fixtures, exporters, MCP schemas, Lua descriptors e diagnostics importantes.

## Round-trip

`.petunia save→load`, migration→save→reload, import/export quando aplicável, Undo/Redo sequences.

## UI/visual

Petunia Components sobre egui, keyboard/focus, AccessKit tree, `egui_kittest`, screenshot comparisons e flows reais.

# Geometry invariants

Em debug/tests verificar sistematicamente:

```
half-edge cycles close
twin relation is symmetric
next/previous coherent
faces reference valid edges
vertices reference valid halfedges
no references to deleted keys
positions are finite
no NaN/Inf
triangulation triangles map to valid source FaceId
```

# proptest

Exemplos de propriedades:

```
valid Profile → Extrude → valid manifold/open mesh according to contract
valid mesh → Weld allowed points → invariants hold
Connect → Undo → semantic equality with original
random local edit sequence → never leaves dangling IDs
save/load → semantic equality
```

# Fuzz targets

Prioridade:

- `.petunia` manifest/document loader;
- ZIP/file boundaries;
- image decode adapters;
- OBJ/glTF inputs;
- Profile/polygon processing;
- triangulation;
- Connect matching;
- Cut/Slice adapters;
- Boolean/UV conversion boundaries;
- plugin manifest;
- MCP input schema.

# Concurrency tests

Jobs carregam base revision. Testar explicitamente resultado stale:

```
revision 10 snapshot → start job
main document becomes revision 11
job 10 returns
→ result is rejected/marked stale
```

Não testar apenas o happy path.

# egui + IA

A stack deve aproveitar `egui_kittest`, `egui_inspection` e `egui_mcp` em builds de desenvolvimento:

```
read Figma/Notion spec
→ implement/reuse Petunia Component
→ cargo xtask ui-test
→ compile/run development build
→ inspect AccessKit tree
→ send mouse/keyboard input
→ capture screenshot
→ compare against approved reference
→ fix
→ repeat
```

A definição de pronto de UI não pode ser “compilou”. Inspection/MCP de UI deve ficar desabilitado em release e restrito a loopback/configuração de desenvolvimento quando ativo.

# Visual regression

Para componentes/telas congelados:

- screenshots em tamanho e UI scale conhecidos;
- tolerância definida para pequenas diferenças de rasterização;
- comparação estrutural/tokens junto da imagem;
- atualizar golden visual somente com decisão explícita.

# Accessibility validation

UI tests devem verificar quando aplicável:

- role;
- accessible name;
- selected/checked/expanded/disabled state;
- keyboard activation;
- focus visibility;
- focus order;
- Escape/Enter consistency;
- nenhum trap em toolbar/segmented control;
- Parts tree e Context fields presentes na accessibility tree.

# AI-friendly architecture

Agentes devem encontrar módulos pequenos e nomes explícitos. Regras:

- evitar arquivos gigantes sem necessidade;
- evitar abstrações genéricas antes de uso real;
- interfaces/traits somente em boundaries reais;
- invariants documentadas perto do código;
- tests próximos da feature;
- commands semanticamente nomeados;
- TODOs com contexto, condição de conclusão e dependências;
- nenhuma dependência de conhecimento implícito do autor;
- evitar macros/metaprogramming que escondam data flow essencial;
- evitar abreviações internas obscuras;
- usar error types/codes estruturados para permitir feedback automático.

A arquitetura de código completa é definida no capítulo 34.

# Observabilidade

Usar `tracing` com spans por command/job/provider. Em debug, conseguir responder:

```
qual command rodou?
qual revision iniciou?
qual provider foi usado?
houve fallback?
quanto demorou?
qual cache foi invalidado?
qual transaction foi commitada?
```

# CI inicial

Matrix obrigatória:

```
Windows x86_64
Linux x86_64
```

Stages:

```
format
→ clippy
→ unit/command/property tests
→ provider contracts
→ project round-trip/migrations
→ export goldens
→ plugin/MCP tests
→ UI component tests
→ license/security checks
→ debug/release build
```

# Vertical slice obrigatório de conformance

Implementar na mesma arquitetura para provar a baseline final:

```
launch Petunia egui/eframe shell
→ egui-wgpu custom viewport rendering
→ create cube primitive
→ select a face
→ Extrude
→ Undo/Redo
→ import/reference image
→ create Profile
→ project reference texture
→ save .petunia
→ close/reload .petunia
→ export GLB
→ perform one Fuse via manifold-rust
→ perform Auto UV via native route/xatlas fallback
→ paint one stroke on model
→ load simple Lua plugin/command
→ call one equivalent operation through MCP
```

# UI slice

O vertical slice não pode usar uma UI genérica temporária como única prova. Precisa implementar um recorte real do Petunia Design System:

- workspace pills;
- Parts;
- Context;
- floating toolbar;
- viewport controls;
- ao menos Button, IconButton, SplitButton, SegmentedControl, NumberField e Panel;
- dark tokens da UI Baseline Final V1;
- keyboard/focus/accessibility metadata.

# Critérios de falha estrutural

## egui/UI stack

Uma falha só pode disparar **novo ADR de reabertura** se o vertical slice demonstrar bloqueadores graves/estruturais reproduzíveis em:

```
focus/keyboard routing
menus/popups
Parts tree
numeric fields
HiDPI
AccessKit/accessibility
custom viewport composition
custom styling/Design System fidelity
IME/text input
Linux/Windows behavior
CPU cost recorrente da immediate-mode UI
```

Problemas corrigíveis, tuning visual ou crates auxiliares substituíveis não reabrem a stack por si só. Corrigir adapter/dependency/implementação primeiro; reabertura exige custo estrutural recorrente que comprometa requisito normativo da UI.

## Veto de wgpu

Reabrir renderer se a integração `egui-wgpu`/custom render pass for impraticável, instável a ponto de quebrar desenvolvimento frequente ou impedir targets obrigatórios.

## Veto de geometry

Reabrir provider, não a linguagem inteira, se manifold/xatlas/adapters falharem. Preferir substituir uma dependência isolada antes de abandonar Rust.

# Métricas de desenvolvimento assistido por IA

Registrar no vertical slice:

```
tempo de implementação
build incremental
LOC própria
LOC glue/FFI
número de tentativas do agente
build/test failures
regressões de UI
intervenções humanas
bugs de ownership/concurrency
fidelidade ao Figma
```

Essas métricas servem como evidência de conformance e, somente diante de bloqueador estrutural, como material para um novo ADR.

# Knowledge map para agentes

Não usar um único `AGENTS.md` gigantesco como depósito da especificação. O repositório deve possuir um mapa hierárquico, por exemplo:

```
AGENTS.md
  → architecture map
  → build/test commands
  → links para documentação canônica

docs/architecture/
  domain.md
  geometry.md
  commands.md
  rendering.md
  plugins.md
  testing.md

crates/petunia-geometry/AGENTS.md
crates/petunia-render/AGENTS.md
...
```

O arquivo raiz orienta; documentos próximos do código detalham invariants locais. Os agentes e o CI (`xtask`) devem manter esse mapa sincronizado com o Livro Vivo sem duplicar toda a documentação em cada arquivo.

# Regra para agentes

Nenhum agente pode declarar feature concluída sem rodar a verificação prevista para aquela camada. Para mudanças de UI, teste visual/interativo; para geometry, invariants/property tests; para I/O, round-trip; para provider, contract tests.

Além disso, agentes devem respeitar a cadeia **Tool → Command → Algorithm → Data**, nunca introduzir Tool→Tool, mutação direta da half-edge pela UI, lógica paralela em Lua/MCP, `Arc<Mutex<ApplicationState>>` global ou `.clone()` usado apenas para calar borrow checker. Violações arquiteturais contam como falha mesmo que o código compile.

# Definition of Done arquitetural

Além da validação funcional, uma feature só pode ser considerada concluída quando seu comportamento de domínio está implementado, o command tipado existe quando aplicável, transaction/Undo e errors estão definidos, testes da camada e de interoperabilidade passam, UI/Lua/MCP reutilizam o mesmo command sem duplicar lógica e `cargo xtask verify` passa.

Ver [34 — Arquitetura Modular Explícita, Rust Safety e Representação em Código](34-arquitetura-modular-rust-safety.md) para o contrato completo de modularidade e Rust safety.

# Regra final

**A baseline final não é considerada implementada apenas porque foi aprovada no papel; ela atinge conformance quando um fluxo real Petunia atravessa UI → commands → geometry → renderer → persistence → extensibility com qualidade mensurável.**