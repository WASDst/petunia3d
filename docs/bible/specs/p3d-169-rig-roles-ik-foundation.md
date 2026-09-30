# P3D-169 — Rig Roles & IK Foundation

<aside>
🦴

Estado: **SPEC DRAFT (2026-09-30); dados de Rig Roles, solvers de IK e comandos transacionais implementados (F0.4–F0.6); sem registro em `canonical()` (D-21) nem UI** · Prioridade: P2 · Era 3. Fundação headless do Animate procedural ([cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)). Sem UI própria: consumida por P3D-170, P3D-173 e pelo retarget (P3D-138).

</aside>

## Objetivo

Dar **semântica** aos ossos e oferecer **IK mínimo** para que geradores, retarget e pés plantados funcionem em qualquer rig (humanoide, quadrúpede, multi-leg, serpente, peixe, pássaro) sem depender de nomes de osso.

## Rig Roles

Um `RigRoleMap` associa `BoneId → RigRole`. Papéis são dados, não código por espécie:

| Grupo | Papéis (exemplos) |
| --- | --- |
| Corpo | `Root`, `Hips`, `Spine(n)`, `Chest`, `Neck`, `Head`, `Jaw` |
| Membros | `Leg(i).Thigh`, `Leg(i).Shin`, `Leg(i).Foot`, `Arm(i).Upper`, `Arm(i).Lower`, `Arm(i).Hand`, com `i` = índice de membro |
| Cadeias | `Tail(n)`, `Spine(n)` de serpente/peixe, `Tentacle(n)`, `Wing(i).Segment(n)` |
| Acessórios | `Wiggle(n)` para cabelo/capa/orelha (consumido por P3D-173) |

- Presets (P3D-136) preenchem o mapa; o Auto-Rig (P3D-137) também.
- O usuário pode **atribuir ou corrigir papéis** manualmente ("Roles"), com validação imediata.
- O mapa é recurso do rig (mesmo ciclo de vida do `Skeleton`), com IDs estáveis (P3D-108) e serialização versionada no `.petunia`.

## Contrato de rig (Rig Requirements)

Cada gerador declara os papéis de que precisa (`RigRequirements`). A validação devolve **erro estruturado e legível**, nunca uma animação quebrada. Exemplo: o gait de N pernas exige ao menos duas cadeias `Leg(i)` completas (coxa, canela, pé).

## IK

| Solver | Uso | Notas |
| --- | --- | --- |
| **Two-bone analítico** | Pernas e braços | Pole vector; **soft IK** perto da extensão total para evitar "estalo" do joelho |
| **FABRIK** | Cadeias longas (cauda, tentáculo, serpente) | Preserva comprimento de osso; limites angulares opcionais; máximo de iterações e tolerância explícitos |
| **Look-at / Aim** | Cabeça, olhar, mira | Um osso, peso e limites |

Um `IkChain` é dado: `{ id, bones (por papel), solver, pole, weight, revision }`. Alvos vêm do gerador (P3D-170) ou de um alvo manual.

## Foot planting

- O gerador fornece a **fase de contato** por perna; o IK mantém o pé no alvo durante o contato (sem deslizar).
- V1: plano de chão (`Y = 0` no espaço do asset). Terreno, raycast contra cena e conteúdo de level ficam **fora** (P3D-128).

## Boundaries

| Camada | Responsabilidade |
| --- | --- |
| Data | `RigRoleMap`, `IkChain` no projeto; sem tipos de UI |
| Algorithm | solvers puros e deterministas (`f32`, sem alocação por frame no caminho quente) |
| Command | `rig.assign_role`, `rig.clear_role`, `rig.add_ik_chain`, `rig.set_ik_target`, `rig.remove_ik_chain` — transacionais, com Undo/Redo; registrados em `CommandDispatcher::canonical()` |
| UI | `TextId`, `IconId` e `ThemeToken` para "Roles"; nenhuma tecla física (keymap) |

## Não objetivos

Stack geral de constraints, blending IK/FK arbitrário, IK de corpo inteiro, física (P3D-173).

## Erros e estados inválidos

Alvo inalcançável → clamp na extensão máxima com aviso; pole colinear → pole alternativo determinístico; cadeia com osso de comprimento zero → erro; papel duplicado → erro com os dois ossos citados.

## Testes / DoD

1. Two-bone alcança o alvo dentro da tolerância e preserva o comprimento dos ossos.
2. Alvo inalcançável estende sem flip; soft IK sem descontinuidade.
3. FABRIK converge, preserva comprimentos e respeita limites.
4. Papéis: atribuição válida/inválida, duplicados, rigs incompletos com mensagem clara.
5. Round-trip `.petunia` do `RigRoleMap` e das `IkChain`; migração retrocompatível.
6. Determinismo: mesma entrada → mesmo hash de pose.
7. Undo/Redo dos comandos por hash de projeto; nenhum no-op cria histórico.
8. `arch-check`: nenhum tipo de UI no módulo.

## Dependências

P3D-135, P3D-136, P3D-108, P3D-100/101 (Commands). Amplia o escopo "constraints" excluído por P3D-067 **somente** ao conjunto acima ([emenda](p3d-067-animacao-simples.md)).
