# P3D-160 — Bake, Flatten & Derived Asset Contract

<aside>
🔥

Estado: **fundação pós-V1 aprovada**. Decals, modifiers, generators e recipes precisam de um vocabulário comum para transformar authoring live em resultados portáveis.

</aside>

# Vocabulário

- **Apply**: torna uma operação geométrica live parte da base mesh quando semanticamente apropriado.
- **Bake**: materializa informação derivada em mesh, texture ou channel de destino.
- **Flatten**: combina uma pilha de layers/effects/surface details em representação estática equivalente.
- **Keep Live**: preserva authoring não destrutivo.

# Objetivo

Evitar que cada sistema invente export/bake próprios.

# Casos

- Modifier Stack → Apply to Mesh;
- Decals → Bake BaseColor/Normal/Roughness;
- Generator → Bake procedural result to regular Parts/Mesh;
- Surface Recipe → Bake to texture/channel;
- Linked Instance → Make Unique/Flatten;
- Asset State → export selected state/preset.

# Preview

Toda operação destrutiva mostra impacto: tris/vertices, texture resolution, removed live dependencies e warnings.

# Transaction

Bake pesado trabalha em snapshot/job e somente commita se revision ainda for válida. Cancelamento obrigatório quando aplicável.

# Export profiles

`Portable Static` favorece bake/flatten; `Editable/Engine Adapter` pode preservar metadata/variants suportadas.

# Testes / DoD

Equivalence tests live vs baked, cancellation, revision conflict, Undo when feasible, atomic output e diagnostics para perda de editabilidade.

# Extensão pós-V1 (2026-09-30) — Animação

O vocabulário aplica-se a animação: **Motion Recipe** e pilhas de camadas ficam **vivos** (*Keep Live*) até o usuário escolher **Apply Now**, que amostra em fps escolhido, **reduz keys** por tolerância, gera `BoneTrack`s editáveis, reporta o que mudou e é atômico no Undo ([P3D-170](p3d-170-procedural-motion-generators.md), [P3D-173](p3d-173-secondary-motion-simple-ragdoll.md), [P3D-174](p3d-174-layered-animation.md)). Simulações (Wiggle, ragdoll) são sempre cozidas na exportação.
