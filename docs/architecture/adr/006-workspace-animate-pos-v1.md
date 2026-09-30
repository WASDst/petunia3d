# ADR 006 — Workspace Animate pós-V1 e animação procedural primeiro

Status: **aceito em 2026-09-30** por autorização explícita do responsável do produto (decisões D1–D4 do capítulo 45). O layout do workspace descrito abaixo é **proposta de especificação (SPEC DRAFT)**: não há implementação Slint nem captura de aceite.
Autoridade: [Livro Vivo, capítulo 45](../../bible/foundations/45-pos-v1-animate-acessivel-animacao-procedural.md), [capítulo 36](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md) (adendo pós-V1), [P3D-066](../../bible/specs/p3d-066-animation-workspace.md), [P3D-067](../../bible/specs/p3d-067-animacao-simples.md) e [P3D-169](../../bible/specs/p3d-169-rig-roles-ik-foundation.md)–[P3D-174](../../bible/specs/p3d-174-layered-animation.md).
Este ADR registra o motivo; não cria uma segunda baseline.

## Contexto

O caderno previa Animate (P3D-066/067) como workspace de baixa prioridade (P3), com rig, clipes e timeline, e **excluía** constraints avançadas e simulação. Uma conversa de 2026-09-30 mostrou que:

1. O domínio de rig e clipes existe em `petunia_project`, mas **não alcança o produto**: a UI de Animate vive só no egui legado, atrás da feature `animation-workspace`, e o Slint não a tem.
2. **Animação procedural não constava do roadmap** — "procedural" referia-se só a geometria.
3. O objetivo de produto é que o Animate seja **extremamente fácil para quem não entende de animação**. Uma timeline de keyframes como primeira experiência contradiz esse objetivo.
4. O capítulo 36 lista `MODEL / PAINT / UV` como workspaces **V1** e diz que "workspace não implementado não aparece como pill desabilitada".

## Decisão

1. **Animate é o primeiro workspace pós-V1.** A UI Baseline V1 **não é reaberta**: continua `MODEL / PAINT / UV`. A pill **Animate só aparece quando o workspace estiver implementado e aceito** (F2 do capítulo 45); antes disso o workspace não existe no shell de produção, e a feature `animation-workspace` do egui legado segue como transição, sem receber novas features (AGENTS §0.1).
2. **Procedural primeiro** (D1). A primeira experiência é escolher um **Motion** e ajustar poucos sliders; keyframes, Ghosts e camadas entram por divulgação progressiva. A cadeia é `Tool → Command → Algorithm → Data`: o módulo de animação não conhece UI.
3. **Criaturas de primeira classe** (D2): os geradores usam **papéis de osso** (Rig Roles) e **número de pernas**, nunca nomes de osso nem lógica por espécie.
4. **Sem vídeo e sem ML por enquanto** (D3): a referência de animação é batch de imagens (P3D-172).
5. **Emenda escopada ao P3D-067.** As exclusões "constraints avançadas" e "simulation" continuam valendo, com **três exceções nomeadas**: IK mínimo (P3D-169), Wiggle e Ragdoll simples (P3D-173). Nenhum stack geral de constraints, graph editor, NLA ou state machine é aprovado.
6. **Layout proposto do Animate** (subordinado ao cap. 36; sem docking irrestrito, sem janelas de sistema, um único viewport 3D):

   | Região | Conteúdo |
   | --- | --- |
   | Esquerda | barra de criação do workspace: **Motions** (grade com prévia), Rig e Reference Frames |
   | Centro | viewport dominante com Ghosts/Trails; **Reference Monitor** como cartão flutuante *in-canvas* (padrão do [ADR 005](005-modulos-inspector-dock-float-pin.md)) |
   | Direita | Inspector em seções independentes (dock/float/pin, ADR 005): `Rig → Motion → Style → Layers` |
   | Inferior | Timeline (clip/Motion, keys, filmstrip de referência), no lugar da Asset Library enquanto o workspace estiver ativo; a Animation Asset Library (P3D-139) abre como gaveta |

   **Nota de implementação (F2, 2026-09-30):** o layout acima foi implementado com três desvios — Inspector aberto por padrão no Animate, *Styles* dentro da seção *Motion* (`Creature → Motions → Motion`) e **transporte in-canvas** no lugar da timeline (que é F3). A pill ANIMATE só existe com a feature `animation-workspace`. Falta o aceite visual; ver "Estado da fase F2" no capítulo 45.

7. **Ordem de entrega** (fases F0–F5 do capítulo 45): fundação headless e glTF com skin/animação → Motions headless → shell Animate camadas 1–2 → Posar (timeline, Ghosts) → Reference Frames e camadas → Wiggle e biblioteca.

## Consequências

- **Positivas:** o domínio de rig e clipes passa a ter caminho até o produto; a experiência inicial do Animate é a mais simples possível; criaturas e humanoides usam o mesmo motor; a arquitetura respeita Core agnóstico à UI, comandos transacionais e determinismo.
- **Custos e riscos:** exige dono único de transação para os comandos de rig e clipe (D-02 da matriz de 2026-09-29), glTF com skin e animação (hoje ausente, AN-13) e componentes focáveis no Slint (AX-03) antes do shell; a simulação (Wiggle, ragdoll) precisa de cache determinístico para scrubbing.
- **Não aprovado por este ADR:** implementação de código, prazos, e qualquer alegação de acessibilidade — esta exige o "teste do iniciante" do capítulo 45 medido com artistas.

## Alternativas consideradas

| Alternativa | Por que não |
| --- | --- |
| Keyframe e timeline primeiro (Era 3 original) | Dificulta o objetivo de acessibilidade; entrega valor só a quem já sabe animar |
| Reabrir a UI Baseline V1 para incluir Animate | Viola a baseline congelada sem necessidade; o adendo pós-V1 basta |
| Animações codificadas por espécie (modelo do Dust3D) | Explosão N rigs × M animações; contraria P3D-136 (sem lógica humanoide no core) |
| Vídeo → movimento e geração por IA | Ruidoso, restrito a humanos, custo alto em PC modesto; P3D-129 veda nuvem obrigatória. Adiado (D3) |

## Verificação

Gap Matrix `AN-01…AN-15` no capítulo 45; `bible-check` e `docs-check` como gates documentais; a cada fase, os gates do AGENTS §4 e evidência registrada.
