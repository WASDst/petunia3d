# 08 — Roadmap Pós-GA Aprovado

<aside>
🚀

Roadmap aprovado para o período posterior à consolidação do backlog principal. Nenhum item desta página deve atropelar o GA Hardening nem descaracterizar o Petunia3D como ferramenta focada em criação de assets low-poly para jogos.

</aside>

# Regra de entrada pós-GA

Uma funcionalidade futura deve reforçar pelo menos uma destas metas: facilitar criação de assets low-poly, reduzir trabalho entre Petunia e game engines, reutilizar o Core/Commands/API existentes, preservar modularidade e manter boa performance em PCs modestos. Recursos que empurrem o produto para DCC universal devem virar outro produto ou módulo externo.

# Era 0 — GA Hardening

Antes de novas features grandes, executar uma fase sem expansão de escopo focada em confiabilidade, compatibilidade de projetos, migrations, performance, UX, testes, documentação e encerramento dos débitos arquiteturais críticos. A cadeia `abrir → modelar → pintar → organizar → salvar → fechar → reabrir → exportar` deve ser confiável antes do pós-GA.

# Era 1 — The Low-Poly Asset Tool

Prioridade em recursos diretamente ligados ao pipeline de assets de games: **Decals / Surface Details**, Asset Validator, collision authoring, sockets, LODs, palettes, texture atlas, vertex colors, batch processing, portable project packages, tool presets e command recipes/macros. Decals foram promovidos para esta era por oferecerem alto valor simultaneamente para personagens, roupas, props, veículos e environments com custo de implementação moderado.

# Era 1.5 — Non-Destructive Asset Authoring

Após a base game-ready, introduzir as fundações que multiplicam produtividade sem mudar a identidade do produto: **Modifier Stack**, **Surface Attachment**, **Parametric Asset Properties**, **Bake/Flatten Contract**, **Spline Core**, Parts Hierarchy/Linked Instances e Procedural Path Generators asset-local.

A experiência continua shape-first e direta. Non-destructive authoring é uma camada opcional: ações como Mirror/Array/Bend podem oferecer `Apply Now` ou `Keep Live`. Modifiers deformadores devem ter representação didática no viewport com cage/capture box, eixo/spine, origem, limits e handles diretos. O usuário deve conseguir compreender visualmente onde e como a deformação atua antes de abrir parâmetros avançados.

# Era 2 — Surface & Lookdev

Aprofundar Material/Paint/UV com masks, Surface Paint Toolbox, decal channels avançados, effect stack e baking multi-channel quando custo/benefício justificar. **Surface Recipes** são a evolução controlada de texturing nodes: presets/effects primeiro, graph apenas em Advanced. Generator Recipes somente após Spline Core e Modifier foundation; Geometry Nodes genérico permanece fora.

# Era 3 — Characters & Animation

Evoluir Skeleton/Rig Core, skinning, rig presets, retargeting, auto-rig e Animation Asset Library, **com animação procedural primeiro** (revisão de 2026-09-30, [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md) e [ADR 006](../../architecture/adr/006-workspace-animate-pos-v1.md)).

Ordem interna revisada: (F0) Rig Roles, IK e glTF com skin/animação → (F1) Motion Generators headless, com criaturas de primeira classe → (F2) workspace **Animate** pós-V1 com escolha de Motion e sliders → (F3) timeline, Ghosts e Trails → (F4) Reference Frames (batch de imagens) e Tweak Layers → (F5) Wiggle, ragdoll simples e Animation Asset Library. O auto-rig continua exigindo rig manual sólido e edição posterior. **Vídeo, estimativa de pose, mocap e geração por IA não fazem parte da Era 3.**

# Era 4 — Programmable & Intelligent Petunia

Public Extension API independente do runtime, Lua como primeiro adapter, Command Recipes, MCP, Internal Agent Panel e AI-Assisted Modeling. IA deve operar sobre Commands/Tools/API sem simular cliques de UI.

# Era 5 — Ecosystem

Petunia3D continua independente. Map Editor e Game Engine devem ser produtos separados que compartilham formatos, bibliotecas e módulos neutros. Integrações entram por bridge, export profiles e futuramente Live Asset Link.

# Recursos aprovados

- P3D-144 — Asset Validator & Game Readiness.
- P3D-145 — Collision Authoring.
- P3D-146 — Sockets & Attachment Points.
- P3D-147 — LOD Manager.
- P3D-148 — Palette Manager.
- P3D-149 — Texture Atlas Builder.
- P3D-150 — Vertex Color Painting.
- P3D-151 — Batch Asset Processor.
- P3D-152 — Portable Project Package.
- P3D-153 — Tool Presets.
- P3D-154 — Command Recipes / Macros.
- P3D-155 — Live Asset Link.
- **P3D-156 — Decals & Surface Details.**
- **P3D-157 — Modifier Stack & Visual Deformer UX.**
- **P3D-158 — Surface Attachment Foundation.**
- **P3D-159 — Parametric Asset Properties.**
- **P3D-160 — Bake, Flatten & Derived Asset Contract.**
- **P3D-161 — Spline Core.**
- **P3D-162 — Simple Morph Targets.**
- **P3D-163 — Low-Poly Hair Designer.**
- **P3D-164 — Surface Recipes & Simple Nodes.**
- **P3D-165 — Surface Paint Toolbox.**
- **P3D-166 — Parts Hierarchy & Linked Instances.**
- **P3D-167 — Asset States & Variant Composition.**
- **P3D-168 — Procedural Path Generators.**
- **P3D-169 — Rig Roles & IK Foundation** (SPEC DRAFT, 2026-09-30).
- **P3D-170 — Procedural Motion Generators** (SPEC DRAFT).
- **P3D-171 — Animation Ghosts & Trajectories** (SPEC DRAFT).
- **P3D-172 — Reference Image Sequence** (SPEC DRAFT).
- **P3D-173 — Secondary Motion & Simple Ragdoll** (SPEC DRAFT).
- **P3D-174 — Layered Animation** (SPEC DRAFT).

# Recursos complementares aprovados como candidatos

- **Parametric Decal Sets / Facial Atlases** como evolução de P3D-156 + P3D-159.
- Origin & Pivot Presets e hinge workflow.
- Bounding Box / Dimension Tools e Game Scale Guides.
- Naming Rules para engine/export.
- Material Variants integradas às Parametric Asset Properties.
- Pose Library.
- Material Presets.
- Brush Library.
- Procedural Primitive Presets.
- Quick Symmetry.
- Dithering Preview.
- Retro Renderer Preview Profiles.
- Baking simples, inicialmente AO/Normal/Curvature/Vertex AO, reutilizado por Surface Recipes.
- Texture budget, material-slot, alpha/overdraw e draw-call diagnostics dentro do Game Ready Inspector.
- Package format local para Theme/Icon Pack/Keymap/Plugin/Material Preset/Brush Preset/Rig Preset sem marketplace obrigatório.

# Recursos deliberadamente fora do centro

Sculpting avançado, remesh, fluid simulation, cloth completa, particles complexo, compositor, render farm, video editor, CAD paramétrico completo, Geometry Nodes equivalente ao Blender, scene editor completo, terrain/world editor embutido, game engine dentro do Petunia, marketplace/login/cloud obrigatório.

# Regra para agentes

Quando detalhar ou implementar qualquer item pós-GA, primeiro confirmar que ele continua respeitando a Constituição, que não cria um sistema paralelo aos Commands/Core/API e que pode permanecer isolado do workflow principal quando o usuário não precisar dele.

[P3D-144 — Asset Validator & Game Readiness](../specs/p3d-144-asset-validator-game-readiness.md)

[P3D-145 — Collision Authoring](../specs/p3d-145-collision-authoring.md)

[P3D-146 — Sockets & Attachment Points](../specs/p3d-146-sockets-attachment-points.md)

[P3D-147 — LOD Manager](../specs/p3d-147-lod-manager.md)

[P3D-148 — Palette Manager](../specs/p3d-148-palette-manager.md)

[P3D-149 — Texture Atlas Builder](../specs/p3d-149-texture-atlas-builder.md)

[P3D-150 — Vertex Color Painting](../specs/p3d-150-vertex-color-painting.md)

[P3D-151 — Batch Asset Processor](../specs/p3d-151-batch-asset-processor.md)

[P3D-152 — Portable Project Package](../specs/p3d-152-portable-project-package.md)

[P3D-153 — Tool Presets](../specs/p3d-153-tool-presets.md)

[P3D-154 — Command Recipes / Macros](../specs/p3d-154-command-recipes-macros.md)

[P3D-155 — Live Asset Link](../specs/p3d-155-live-asset-link.md)

[P3D-156 — Decals & Surface Details](../specs/p3d-156-decals-surface-details.md)

# Contratos transversais pós-V1

## Visual-first Modifier UX

Modifiers deformadores não podem ser apenas sliders que produzem uma forma inesperada. Quando selecionados, Bend/Twist/Taper/Stretch devem exibir uma **Deformer Cage / Capture Box** ajustada à região afetada, eixo/spine, origin, Start/End bounds e handles próprios. Bend mostra arco-guia; Twist mostra anéis/indicação helicoidal; Stretch mostra handles longitudinais e volume preview; Taper mostra frames de seção; Array mostra ghost copies; Thickness mostra shell/normal offset; Mirror mostra plane + ghost result.

Viewport handles e campos do Inspector operam sobre os mesmos Commands e parâmetros. `Show Guides: Always / When Selected / Never` respeita acessibilidade e preferência do usuário. A referência conceitual vem de deformers visuais usados em DCCs como Houdini e Maya, simplificados para a linguagem visual do Petunia.

## Surface Manipulator

Fundação compartilhável para Decals, Projection e outras operações surface-relative: slide por raycast, tangent-space movement, rotate around normal, scale, mirror, offset e preview.

## Surface Attachment

Uma representação única de attachment evita Decal/Hair/Conform/Path Paint implementarem âncoras incompatíveis. Topology revision invalida de forma explícita e oferece reprojection; nada é reanexado silenciosamente a outra região.

## Parametric Asset Properties

Morph weights, Decal Sets, Material Variants, Modifier/Generator parameters e visibility podem ser expostos por uma interface semântica comum para Inspector, Presets, Animation, MCP e export adapters.

## Apply / Keep Live / Bake / Flatten

Vocabulário uniforme em todo o produto. Preview de operações destrutivas informa impacto e dependências antes do commit.

## Scope Unit anti-bloat

Toda feature declara uma unidade de escopo:

- `Asset-local` → candidato natural ao Petunia;
- `Project-library` → candidato natural ao Petunia;
- `Scene/World` → outro produto/editor;
- `Runtime/Game Logic` → game engine.

Teste de produto: **se o resultado é um asset exportável isoladamente, provavelmente cabe; se o resultado é um level/world, não cabe.**

# Ordem de implementação pós-GA revisada

```
GA Hardening
→ Game Ready Inspector + Pivot/Scale/Validator quick wins
→ Decals M0/M1 + Surface Manipulator
→ Surface Attachment + Bake/Flatten contracts
→ Modifier Stack foundation
→ Mirror Live / Array / Thickness / Simple Deform visual-first
→ Parts Hierarchy + Linked Instances
→ Spline Core
→ Cable / Pipe / Fence / Wall generators
→ Parametric Asset Properties
→ Simple Morph Targets
→ Low-Poly Hair Designer
→ Surface Paint Toolbox expansion
→ Surface Recipes / simple nodes
→ Parametric/animated Decal Sets
→ Asset States / deeper Character Authoring
```

# Regra para implementação por agentes

Cada P3D pós-V1 deve passar por `SPEC READY` antes do código e declarar: objetivo, non-goals, data model, dependencies, Commands/API, viewport/Inspector UX, visual feedback, Undo, persistência, invalidation, performance budget, export/bake semantics, accessibility, tests/fixtures e acceptance criteria. Nenhum agente deve criar um sistema paralelo a Geometry Core, Texture/Layer Core, Commands ou project serialization.

[P3D-157 — Modifier Stack & Visual Deformer UX](../specs/p3d-157-modifier-stack-visual-deformer-ux.md)

[P3D-158 — Surface Attachment Foundation](../specs/p3d-158-surface-attachment-foundation.md)

[P3D-159 — Parametric Asset Properties](../specs/p3d-159-parametric-asset-properties.md)

[P3D-160 — Bake, Flatten & Derived Asset Contract](../specs/p3d-160-bake-flatten-derived-asset-contract.md)

[P3D-161 — Spline Core](../specs/p3d-161-spline-core.md)

[P3D-162 — Simple Morph Targets](../specs/p3d-162-simple-morph-targets.md)

[P3D-163 — Low-Poly Hair Designer](../specs/p3d-163-low-poly-hair-designer.md)

[P3D-164 — Surface Recipes & Simple Nodes](../specs/p3d-164-surface-recipes-simple-nodes.md)

[P3D-165 — Surface Paint Toolbox](../specs/p3d-165-surface-paint-toolbox.md)

[P3D-166 — Parts Hierarchy & Linked Instances](../specs/p3d-166-parts-hierarchy-linked-instances.md)

[P3D-167 — Asset States & Variant Composition](../specs/p3d-167-asset-states-variant-composition.md)

[P3D-168 — Procedural Path Generators](../specs/p3d-168-procedural-path-generators.md)

[P3D-169 — Rig Roles & IK Foundation](../specs/p3d-169-rig-roles-ik-foundation.md)

[P3D-170 — Procedural Motion Generators](../specs/p3d-170-procedural-motion-generators.md)

[P3D-171 — Animation Ghosts & Trajectories](../specs/p3d-171-animation-ghosts-trajectories.md)

[P3D-172 — Reference Image Sequence](../specs/p3d-172-reference-image-sequence.md)

[P3D-173 — Secondary Motion & Simple Ragdoll](../specs/p3d-173-secondary-motion-simple-ragdoll.md)

[P3D-174 — Layered Animation](../specs/p3d-174-layered-animation.md)
