# P3D-067 — Animação simples

<aside>
🧩

Definição original genérica; fechada sobre o Rig Core · Prioridade: P3 → **P2 (revisada em 2026-09-30)**. Emenda escopada aprovada pelo [ADR 006](../../architecture/adr/006-workspace-animate-pos-v1.md).

</aside>

## Objetivo

Fornecer keyframes/clips mínimos para transformações de bones/rigs e playback previsível. Com a decisão "procedural primeiro" ([cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)), keyframes são a **camada 3 (Posar)**: também recebem o resultado de **Apply Now** dos Motions procedurais (P3D-170).

## Escopo inicial sugerido

- timeline simples;
- keyframe add/delete/move;
- playback/loop;
- clip start/end;
- transform channels necessários ao rig;
- interpolation limitada e clara;
- **Ghosts e Motion Trails** (P3D-171);
- **pose markers** alimentados por Reference Frames (P3D-172).

## Não objetivos

Graph editor complexo, NLA sofisticado, state machines/blend trees de gameplay, cloth e fluidos.

### Emenda de 2026-09-30 — exceções nomeadas

Antes, "constraints avançadas" e "simulation" estavam excluídas sem ressalvas. Continuam **excluídas**, com estas exceções nomeadas e limitadas:

| Exceção | Onde | Limite |
| --- | --- | --- |
| IK mínimo (two-bone, FABRIK, look-at) e foot planting | P3D-169 | sem stack geral de constraints |
| Wiggle (spring chains) e Ragdoll Fall simples | P3D-173 | determinístico, cacheado, com Apply Now |
| Layers aditivas (até 4) e blend de Motions | P3D-174 | sem NLA nem graph editor |

## Dependências

P3D-135, P3D-066; para os itens acima, P3D-169–174.

## Testes / DoD

Create/edit/play clip, keyframes, interpolation suportada, undo, save/load e determinismo; Apply Now de Motion procedural produz clip editável equivalente dentro da tolerância declarada.
