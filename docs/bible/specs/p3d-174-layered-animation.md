# P3D-174 — Layered Animation (Tweak Layers e blend de Motions)

<aside>
🧅

Estado: **SPEC DRAFT (2026-09-30)** · Prioridade: P3 · Era 3 (fase F4 do [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)). É o que permite **ajustar um Motion procedural sem quebrá-lo**.

</aside>

## Objetivo

Combinar a base (Motion vivo ou clipe) com **ajustes aditivos** feitos à mão, e fazer transições entre dois Motions, mantendo tudo simples e sem NLA.

## Camadas

| Tipo | Função |
| --- | --- |
| **Base** | um Motion Recipe (P3D-170) ou um clipe |
| **Tweak (aditiva)** | *offset* sobre a base: rotação delta, translação delta e razão de escala por osso |
| **Override** | substitui a pose dos ossos cobertos, com peso |
| **Wiggle** | secondary motion (P3D-173) aplicado por cima |

- Cada camada tem **peso**, **solo/mute** e **máscara por grupo de papéis** (Upper Body, Legs, Tail…), que usam os papéis do P3D-169.
- **Limite V1: 4 camadas** e um botão **Add Tweak**. Sem NLA, sem graph editor, sem state machine.
- Composição: `pose = base(t) ⊕ Σ tweaks`, onde ⊕ é composição de deltas (quaternion delta, soma de translação, produto de escala), determinística e independente de ordem entre tweaks aditivos disjuntos.

## Blend de Motions

Transição entre dois Motions (por exemplo Walk → Run) por **crossfade com fase alinhada** para ciclos, e um controle contínuo **Speed** que interpola parâmetros de Walk a Run quando ambos são do mesmo gerador. A transição usa `pose(t)` dos dois lados — nunca cópias de keys.

## Flatten

**Apply Now** achata a pilha em `BoneTrack`s editáveis (P3D-160), com relatório e Undo atômico. Camadas Wiggle são cozidas antes (P3D-173).

## Boundaries

`AnimationLayerStack` no projeto; avaliador puro num módulo sem UI; comandos transacionais (`layer.add`, `.remove`, `.set_weight`, `.set_mask`, `.reorder`, `.flatten`) com Undo/Redo, registrados em `canonical()`; UI por `TextId`/`IconId`/`ThemeToken`.

## Não objetivos

NLA sofisticado, graph editor, state machines e blend trees de gameplay (papel da game engine, P3D-143).

## Testes / DoD

1. Composição aditiva: base + tweak nulo = base; tweaks disjuntos comutam.
2. Máscaras por papel afetam só os ossos do grupo.
3. Blend: continuidade na transição e fase alinhada em ciclos.
4. Flatten ≈ avaliação viva dentro da tolerância; Undo/Redo por hash.
5. Serialização e migração do `.petunia`; determinismo por hash.
6. Limite de 4 camadas com mensagem clara.

## Dependências

P3D-170, P3D-169, P3D-173, P3D-160, P3D-067.
