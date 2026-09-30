# P3D-173 — Secondary Motion (Wiggle) & Simple Ragdoll

<aside>
🌀

Estado: **SPEC DRAFT (2026-09-30)** · Prioridade: P3 · Era 3 (fase F5 do [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)). Simulação **leve e determinística**, sem competir com suítes de física.

</aside>

## Objetivo

Dar vida a caudas, cabelo, capas, orelhas e antenas com **um toggle e poucos sliders** (**Wiggle**), e oferecer uma queda simples (**Ragdoll Fall**) para o Motion *Die*.

## Wiggle (spring chains)

Cadeia de ossos marcados com papel `Wiggle(n)` (P3D-169) que segue o corpo com atraso e retorna à pose original.

| Controle (user) | Parâmetro técnico |
| --- | --- |
| Stiffness | rigidez da mola |
| Damping | amortecimento |
| Gravity | influência da gravidade |
| Reach / Radius | raio de colisão simples (esferas/cápsulas do próprio rig) |

- Integração semi-implícita ou verlet com **passo fixo**; sem dependência do frame rate da tela.
- **Scrub-safe:** simulação é *stateful*; o resultado é cacheado de forma determinística a partir de checkpoints (frame 0 ou último checkpoint), de modo que arrastar a timeline dê sempre a mesma pose. Em loops há *warm-up* para o fim casar com o início.
- Modos: **Live** (prévia) e **Apply Now** (bake para keys, P3D-160). A exportação glTF é **sempre baked** — glTF não tem spring bones.

## Ragdoll Fall (simples)

Queda com corpos rígidos aproximados por cápsulas e juntas com limites, para o Motion *Die*. Determinístico por seed; o resultado é convertido em keys por **Apply Now** e a simulação não é reexecutada na reprodução.

## Boundaries

`WiggleChain`/`RagdollSetup` são dados no projeto; o solver é algoritmo puro num módulo sem UI; `Commands` transacionais (`wiggle.add_chain`, `wiggle.set_param`, `wiggle.bake`, `motion.ragdoll_bake`) com Undo/Redo e registro em `canonical()`.

## Não objetivos

Cloth, groom por fios, fluidos, colisão com a cena, física de corpo rígido geral (cap. 08: "fora do centro").

## Testes / DoD

1. Determinismo por hash; passo fixo independente do frame rate.
2. **Scrub:** avançar, voltar e saltar produzem a mesma pose (cache/checkpoints).
3. Loop com warm-up: fim ≈ início dentro de tolerância.
4. Colisão simples não atravessa o corpo em casos de teste.
5. Estabilidade numérica com parâmetros extremos (sem NaN, sem explosão).
6. Apply Now ≈ simulação dentro da tolerância; Undo/Redo por hash.
7. Ragdoll: cápsulas respeitam limites de junta; resultado reprodutível.

## Dependências

P3D-169, P3D-170, P3D-160, P3D-135.
