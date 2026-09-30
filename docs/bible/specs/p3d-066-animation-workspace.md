# P3D-066 — Animation Workspace

<aside>
🧩

Escopo expandido; não tratar como feature monolítica · Prioridade: P3 → **P2 (revisada em 2026-09-30)** · Estado: **SPEC DRAFT** para o shell Slint pós-V1. Visão, UX e fasing em [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md); decisão de shell em [ADR 006](../../architecture/adr/006-workspace-animate-pos-v1.md).

</aside>

## Objetivo

Workspace de animação low-poly **extremamente fácil para quem não entende de animação**, com recursos profundos disponíveis por divulgação progressiva, construído sobre subsistemas separados e reutilizáveis.

## Decomposição obrigatória

- P3D-135 Skeleton & Rig Core.
- P3D-136 Rig Presets (humanoide, quadrúpede, multi-leg, serpente, peixe, pássaro).
- P3D-137 Auto-Rig.
- P3D-138 Retargeting/compatibilidade externa.
- P3D-139 Animation Asset Library.
- **P3D-169** Rig Roles & IK Foundation.
- **P3D-170** Procedural Motion Generators.
- **P3D-171** Animation Ghosts & Trajectories.
- **P3D-172** Reference Image Sequence.
- **P3D-173** Secondary Motion & Simple Ragdoll.
- **P3D-174** Layered Animation.

## UX (emenda de 2026-09-30)

**Procedural primeiro.** A camada 1 é o estado inicial: escolher rig, escolher um **Motion** com prévia ao vivo, ajustar *Speed / Energy / Weight / Stride / Lean* e um **Style**, exportar. As camadas seguintes (Personalizar, Posar, Refinar) aparecem por seções do Inspector e nunca são pré-requisito. Timeline e painéis de rig/animação aparecem principalmente neste workspace, não permanentemente em MODEL/PAINT. A interface prioriza Motions, clips e keyframes simples, **não dezenas de graph editors**.

## Shell

Animate é o **primeiro workspace pós-V1** (adendo no cap. 36): a pill só aparece quando o workspace estiver implementado e aceito; a UI Baseline V1 `MODEL / PAINT / UV` não é reaberta. Layout proposto no ADR 006 (sem docking irrestrito, sem janelas de sistema, um único viewport).

## Arquitetura

Workspace é frontend; skeleton, papéis, receitas, clips e pesos vivem fora da UI e são serializáveis/testáveis. A feature `animation-workspace` do egui legado permanece só como transição.

## Testes / DoD

Trocar workspace sem perder contexto; Motions, rigs e clips básicos visíveis e editáveis; nenhum acoplamento do core a timeline/widget; **teste do iniciante** do cap. 45 medido com artistas antes de qualquer alegação de acessibilidade.
