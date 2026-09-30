# P3D-170 — Procedural Motion Generators

<aside>
🚶

Estado: **SPEC DRAFT (2026-09-30); Biped Cycle, Gait, Serpentine Wave, Idle Breath, Styles, Stepped, Apply Now e comandos implementados no domínio (F1), sem UI; Wing Flap, Reaction, Action e Look/Aim pendentes** · Prioridade: P2 · Era 3. **Primeira experiência do Animate** ([cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md), decisão D1). Criaturas são cidadãs de primeira classe (D2).

</aside>

## Objetivo

Permitir que quem nunca animou escolha um **Motion** (andar, correr, respirar, atacar…) e o ajuste com poucos controles em linguagem comum, obtendo um movimento coerente, em loop quando aplicável e exportável.

## Motion Recipe

Dado serializável, avaliado como **função pura**:

```
pose(t) = Generator(rig, roles, params, style, seed, t)
```

| Campo | Descrição |
| --- | --- |
| `id`, `name` | identidade estável (P3D-108) |
| `generator` | tipo do gerador (ver tabela) |
| `params` | valores tipados com faixa, padrão, unidade e rótulo por `TextId` |
| `style` | preset de parâmetros (Cartoon, Heavy, Stiff, Floaty, Stepped) |
| `seed` | variação determinística |
| `duration`, `loop` | ciclo em segundos; `pose(0) == pose(T)` para geradores cíclicos |
| `root_mode` | `In Place` (padrão) ou `Root Motion` |
| `revision` | invalidação de cache |

## Geradores iniciais

| Gerador | Cobre | Papéis exigidos |
| --- | --- | --- |
| **Biped Cycle** | Walk, Run, Jump | `Hips`, `Spine`, `Leg(0..1)` completas; braços opcionais |
| **Gait (N pernas)** | Quadrúpede (walk/trot/run), aranha, escorpião, inseto | ≥ 2 `Leg(i)` completas. **Um só gerador**, parametrizado por número de pernas e **padrão de passo** (alternado, tetrapod, onda) |
| **Serpentine Wave** | Serpente, peixe, cauda, tentáculo | cadeia `Spine(n)`/`Tail(n)` com ≥ 3 ossos |
| **Wing Flap** | Pássaro, inseto, morcego | `Wing(i)` e corpo; modos Flap e Glide |
| **Idle Breath** | Respirar e balançar | `Spine`/`Chest` |
| **Reaction** | Hit, Die (queda procedural) | corpo; `Die` pode usar ragdoll (P3D-173) |
| **Action** | Attack (bote/golpe), Wave, Eat | conforme ação; cabeça/braços/mandíbula |
| **Look / Aim** | Olhar e mirar | `Head`/`Neck` (IK look-at de P3D-169) |

## Controles universais

**Speed, Energy, Weight, Stride, Lean, Smoothness.** São a camada 1 da UX; os parâmetros específicos de cada gerador (fase por perna, altura do passo, amplitude da onda…) ficam em "Advanced". Rótulos e descrições por `TextId`, com tooltip (P3D-114).

## Styles

Presets que apenas preenchem parâmetros (nunca lógica): **Cartoon** (overshoot e antecipação), **Heavy**, **Stiff**, **Floaty** e **Stepped** — quantização temporal a 12/15 fps com *hold*, para estética retrô. `Stepped` é um filtro sobre `pose(t)`, não um gerador separado.

## Presets de rig adicionais

P3D-136 passa a cobrir também **serpente, peixe e pássaro** (além de humanoide, quadrúpede e multi-leg), ainda como templates de dados e totalmente editáveis, cada um com papéis preenchidos.

## Keep Live e Apply Now

O Motion permanece **vivo** no projeto. **Apply Now** converte em `BoneTrack`s editáveis (P3D-160): amostragem em fps escolhido, **redução de keys** por tolerância (por canal), relatório do que mudou e Undo atômico. A conversão nunca é silenciosa.

## Boundaries

| Camada | Responsabilidade |
| --- | --- |
| Data | `MotionRecipe` no projeto |
| Algorithm | módulo de animação dedicado (sem UI), no padrão de `module-paint`/`module-uv`; o core conhece só um trait de gerador |
| Command | `motion.add`, `motion.remove`, `motion.duplicate`, `motion.set_param`, `motion.set_style`, `motion.apply_now` — transacionais, com Undo/Redo, registrados em `canonical()` (palette, keymap e MCP) |
| UI | Motion picker, sliders e Styles no Inspector do Animate (cap. 36 / ADR 006), tudo por `TextId`/`IconId`/`ThemeToken` |

## Performance

Avaliação por frame sem alocação no caminho quente; cache por `(revision, t quantizado)`. Meta a validar por benchmark em PC modesto (P3D-126); nenhuma alegação de desempenho antes da medição.

## Não objetivos

Motion matching, geração por IA, aprendizado por reforço, state machines de gameplay, blend trees complexas (o blend entre dois Motions é tratado em P3D-174).

## Testes / DoD

1. **Determinismo:** mesma receita + seed → mesmo hash de poses.
2. **Loop:** `pose(0) == pose(T)` dentro de tolerância nos geradores cíclicos.
3. **Faixas:** parâmetros fora da faixa são limitados e reportados.
4. **Contrato de rig:** rig sem os papéis exigidos → erro estruturado, sem animação parcial.
5. **Pés plantados:** deslizamento de pé abaixo de tolerância durante o contato (Biped e Gait com 2, 4, 6 e 8 pernas).
6. **Comprimento de osso** preservado pelo IK.
7. **Apply Now:** `sample(clip)` ≈ `pose(t)` dentro da tolerância declarada; redução de keys respeita o limite; Undo/Redo por hash.
8. **Serialização e migração** do `.petunia`.
9. **Stepped:** quantização correta e hold determinístico.

## Dependências

P3D-169, P3D-135, P3D-136, P3D-160, P3D-108, P3D-100/101.
