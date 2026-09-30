# P3D-136 — Rig Presets — Humanoid, Quadruped e Multi-Leg

<aside>
🧩

Novo item · Prioridade: P3.

</aside>

## Objetivo

Templates de rig reutilizáveis adequados a humanoides, quadrúpedes e criaturas multi-leg como aranhas/escorpiões.

## Princípio

São **templates de dados**, não engines separadas por espécie. Devem definir naming/orientation/hierarchy recomendados e permanecer totalmente editáveis.

## Escopo incremental

1. Humanoid simples.
2. Quadruped simples.
3. Multi-leg genérico com preset demonstrativo para aranha/escorpião.

## Dependências

P3D-135.

## Testes / DoD

Instantiate preset, editar proportions/bones, serialize, aplicar clip compatível quando existir e nenhuma lógica humanoide hardcoded no core.

## Extensão pós-V1 (2026-09-30)

Criaturas são cidadãs de primeira classe ([cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md), decisão D2). O escopo incremental ganha: **4. Serpente/cadeia**, **5. Peixe**, **6. Pássaro/asas**. Todo preset preenche os **Rig Roles** ([P3D-169](p3d-169-rig-roles-ik-foundation.md)) e alimenta os geradores de [P3D-170](p3d-170-procedural-motion-generators.md). O gait de multi-leg é o **mesmo** gerador do quadrúpede, parametrizado por número de pernas.
