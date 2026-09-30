# P3D-135 — Skeleton & Rig Core

<aside>
🧩

Novo subsistema fundamental de animação · Prioridade: P3.

</aside>

## Objetivo

Núcleo mínimo de skeleton/rig independente da UI.

## Modelo mínimo

`SkeletonId`, `BoneId/JointId`, hierarchy parent/children, local/rest transform, bind pose, skin weights e constraints mínimas somente quando justificadas.

## Arquitetura

Skinning/render é adapter sobre os dados do rig; timeline e rig widgets não possuem a verdade do skeleton. IDs estáveis permitem retargeting, animation library e APIs.

## Segurança/robustez

Evitar ciclos na hierarquia, weights inválidos e bones órfãos; erros estruturados.

## Testes / DoD

Criar hierarchy, reparent válido/inválido, bind pose, weights normalizados, save/load, undo e aplicação headless de transforms.

## Extensão pós-V1 (2026-09-30)

O modelo passa a incluir **Rig Roles** e **IK chains** como dados do rig ([P3D-169](p3d-169-rig-roles-ik-foundation.md)). Geradores e retarget consomem **papéis**, nunca nomes de osso. Erro estruturado quando um contrato de rig não é satisfeito.
