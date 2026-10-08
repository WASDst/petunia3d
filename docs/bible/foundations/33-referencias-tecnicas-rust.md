# 33 — Biblioteca de Referências Técnicas da Stack Rust

<aside>
📚

Biblioteca de referências técnicas usadas para fundamentar e implementar a working stack Rust do Petunia3D. Esta página é **fonte de pesquisa**, não substitui os contratos normativos dos capítulos 27–32, a arquitetura de código do capítulo 34 nem a arquitetura egui/Petunia Components do capítulo 35.

</aside>

# UI — egui / eframe / egui-wgpu

- [egui](https://docs.rs/egui/latest/egui/)
- [egui repository](https://github.com/emilk/egui)
- [eframe](https://docs.rs/eframe/latest/eframe/)
- [egui-wgpu](https://docs.rs/egui-wgpu/latest/egui_wgpu/)
- [egui accessibility](https://github.com/emilk/egui/blob/main/docs/accessibility.md)
- [egui_kittest](https://docs.rs/egui_kittest/latest/egui_kittest/)
- [egui third-party crates wiki](https://github.com/emilk/egui/wiki/3rd-party-egui-crates)

Objetivo de consulta: immediate-mode architecture, desktop host, custom wgpu rendering, AccessKit, testing e ecossistema de componentes.

# egui — componentes e infraestrutura selecionados

- [egui_extras](https://docs.rs/egui_extras/latest/egui_extras/) — image loaders, TableBuilder e layouts auxiliares.
- [egui_ltreeview](https://docs.rs/egui_ltreeview/latest/egui_ltreeview/) — Parts/tree.
- [egui_tiles](https://docs.rs/egui_tiles/latest/egui_tiles/) — tiling/layout/panels.
- [egui_dock](https://docs.rs/egui_dock/latest/egui_dock/) — alternativa de docking.
- [egui-file-dialog](https://docs.rs/egui-file-dialog/latest/egui_file_dialog/) — file/folder dialogs integrados.
- [egui_form](https://docs.rs/egui_form/latest/egui_form/) — forms/validation.
- [egui_dnd](https://docs.rs/egui_dnd/latest/egui_dnd/) — reorderable lists.
- [egui-snarl](https://docs.rs/egui-snarl/latest/egui_snarl/) — node graph futuro.
- [egui_json_tree](https://docs.rs/egui_json_tree/latest/egui_json_tree/) — JSON diagnostics.
- [egui_probe](https://docs.rs/egui-probe/latest/egui_probe/) — generated debug panels.
- [puffin_egui](https://docs.rs/puffin_egui/latest/puffin_egui/) — profiler UI.
- [egui_commonmark](https://docs.rs/egui_commonmark/latest/egui_commonmark/) — Markdown/help.

Estas crates são avaliadas segundo o capítulo 35; referência não implica dependency obrigatória.

# egui — UI kits/estilo como referência

- [egui-elements](https://docs.rs/egui-elements/latest/egui_elements/)
- [egui-elegance](https://docs.rs/egui-elegance/latest/elegance/)
- [egui-components](https://docs.rs/egui-components/latest/egui_components/)
- [egui_component](https://docs.rs/egui_component/latest/egui_component/)
- [egui-material3](https://docs.rs/egui-material3/latest/egui_material3/)
- [catppuccin-egui](https://docs.rs/catppuccin-egui/latest/catppuccin_egui/)

Objetivo: estudar component anatomy, theme/style architecture e interactions. **Nenhum kit externo define o visual Petunia.**

# GPU — wgpu

- [wgpu crate documentation](https://docs.rs/wgpu/latest/wgpu/)
- [wgpu Backend enum](https://docs.rs/wgpu/latest/wgpu/enum.Backend.html)

Objetivo: device/queue/resources, cross-platform backends, rendering/texture APIs e headless/test possibilities.

# Math e IDs

- [glam](https://docs.rs/glam/latest/glam/)
- [slotmap](https://docs.rs/slotmap/latest/slotmap/)

Objetivo: matemática vetorial/matrizes e handles generacionais tipados/persistentes.

# Geometry 2D / Profiles

- [geo](https://docs.rs/geo/latest/geo/)
- [geo BooleanOps](https://docs.rs/geo/latest/geo/algorithm/bool_ops/trait.BooleanOps.html)
- [lyon_tessellation](https://docs.rs/lyon_tessellation/latest/lyon_tessellation/)
- [earcutr](https://docs.rs/earcutr/)

`geo` é baseline auxiliar para Profiles. Lyon/Earcut são referências/alternativas específicas; não adicionar simultaneamente sem necessidade.

# Geometry 3D / Boolean

- [manifold-rust](https://docs.rs/manifold-rust/)
- [Manifold upstream](https://github.com/elalish/manifold)

Objetivo: Fuse/Union e Cut/Difference robustos quando operações locais não bastarem.

# UV

- [xatlas upstream](https://github.com/jpcy/xatlas)
- [xatlas_rs_v2](https://docs.rs/xatlas-rs-v2/latest/xatlas_rs_v2/)

Objetivo: generic Auto UV fallback. xatlas permanece isolado atrás de `UvUnwrapProvider`.

# Mesh optimization / tangents — opcionais

- [meshoptimizer](https://github.com/zeux/meshoptimizer)
- [meshopt Rust](https://docs.rs/meshopt/latest/meshopt/)
- [mikktspace Rust](https://docs.rs/mikktspace/latest/mikktspace/)

Não são baseline obrigatória de authoring. Usar apenas quando export optimization/normal maps justificarem.

# Images/Paint

- [image](https://docs.rs/image/latest/image/)
- [imageproc](https://docs.rs/imageproc/latest/imageproc/)

A crate `image` é adapter de codecs. `imageproc` é referência/futuro; o Paint Core usa `TextureBitmap` próprio.

# glTF / asset interchange

- [gltf](https://docs.rs/gltf/latest/gltf/)
- [gltf-json](https://docs.rs/gltf-json/latest/gltf_json/)
- [ufbx](https://ufbx.github.io/getting-started/) — referência futura para FBX, fora da V1.

# Serialization/container

- [Serde](https://serde.rs/)
- [serde_json](https://docs.rs/serde_json/latest/serde_json/)
- [zip](https://docs.rs/zip/latest/zip/)

Objetivo: `.petunia` ZIP + JSON e migrations testáveis.

# Lua

- [mlua](https://docs.rs/mlua/latest/mlua/)
- [Lua 5.4 Reference Manual](https://www.lua.org/manual/5.4/)

Objetivo: um Lua State por plugin, sandbox/capabilities e API pública Lua.

# MCP

- [Official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP specification](https://modelcontextprotocol.io/)

Objetivo: tools/resources schemas e protocol compliance sem reimplementar wire protocol manualmente.

# Concurrency / channels / diagnostics

- [Rayon](https://docs.rs/rayon/latest/rayon/)
- [flume](https://docs.rs/flume/latest/flume/)
- [tracing](https://docs.rs/tracing/latest/tracing/)
- [thiserror](https://docs.rs/thiserror/latest/thiserror/)
- [anyhow](https://docs.rs/anyhow/latest/anyhow/)
- [schemars](https://docs.rs/schemars/latest/schemars/)

# Testing / quality

- [proptest](https://docs.rs/proptest/latest/proptest/)
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html)

Objetivo: property testing, benchmarks e fuzzing de inputs/boundaries.

# Rust architecture, API design e safety

- [Rust API Guidelines — Checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Rust API Guidelines — Naming](https://rust-lang.github.io/api-guidelines/naming.html)
- [Rust API Guidelines — Future Proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo Resolver / Features](https://doc.rust-lang.org/cargo/reference/resolver.html)
- [std:](https://doc.rust-lang.org/std/marker/trait.Send.html):marker:[:Send](https://doc.rust-lang.org/std/marker/trait.Send.html)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/patterns/)
- [Borrow Clone anti-pattern](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html)
- [Miri](https://github.com/rust-lang/miri/)
- [RustBelt](https://plv.mpi-sws.org/rustbelt/popl18/)

Objetivo: API evolution, ownership, newtypes, error contracts, features, safe boundaries, anti-patterns e validação de `unsafe`.

# Projetos arquiteturais de referência

- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html) — architecture invariants, single mutable state/snapshots e boundaries explícitas.
- [Zed](https://github.com/zed-industries/zed) — estudo de grande Cargo workspace, crates por responsabilidade e editor Rust real.
- [Rerun](https://rerun.io/docs/concepts/logging-and-ingestion/entity-component) — composição inspirada em ECS/data-oriented sem obrigar o Petunia a adotar ECS universal.
- [Bevy ECS Guide](https://github.com/bevyengine/bevy/blob/main/examples/ecs/ecs_guide.rs) — referência para benefícios de composição/data locality; não dependency e não arquitetura-base do Petunia.
- [Graphite](https://graphite.art/) — Rust editor architecture/message passing como estudo, não dependency.
- [Graphite editor codebase overview](https://graphite.art/volunteer/guide/codebase-overview/editor-structure/) — dispatcher/messages/document architecture.
- [QMeshLab](https://github.com/cnr-isti-vclab/QMeshLab) — referência C++/Qt alternativa para editor de mesh, útil como comparação arquitetural.

# Desenvolvimento assistido por IA

- [OpenAI — How OpenAI uses Codex](https://openai.com/business/guides-and-resources/how-openai-uses-codex/)
- [OpenAI — Harness engineering](https://openai.com/index/harness-engineering/)
- [GitHub — Repository custom instructions](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions-in-your-ide/add-repository-instructions-in-your-ide)
- [egui release/tooling history](https://github.com/emilk/egui/releases)
- [egui_kittest](https://docs.rs/egui_kittest/latest/egui_kittest/) — semantic/snapshot UI tests
- [egui inspection sources](https://github.com/emilk/egui/tree/main/crates) — inspection/MCP tooling da linha atual

Objetivo: knowledge map hierárquico, comandos verificáveis, contexto persistente, instruções locais próximas do código e UI verification por accessibility tree/input/screenshots.

# Papers/conceitos geométricos e interação

Continuam relevantes as referências acadêmicas já catalogadas nos capítulos de pesquisa do Petunia, especialmente sketch-based modeling, simple geometry + projected imagery e robust mesh arrangements. Esta página não duplica toda a biblioteca acadêmica; ela centraliza a stack Rust.

# Política de uso das referências

O framework/agente deve:

1. ler primeiro os contratos do Livro Vivo;
2. consultar documentação oficial da dependência antes de inventar API;
3. pin versions no código real;
4. não assumir que exemplos da latest documentation correspondem à versão pinada;
5. registrar workarounds específicos e remover quando upstream corrigir;
6. nunca promover uma biblioteca de referência/opcional para baseline sem decisão explícita.

# Regra final

**Referências ajudam a implementar os contratos; não definem o produto.**