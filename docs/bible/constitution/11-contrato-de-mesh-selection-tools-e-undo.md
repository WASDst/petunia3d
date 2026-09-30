# 11 — Contrato de Mesh, Selection, Tools e Undo

<aside>
🧰

Contrato compartilhado para P3D-015–041, P3D-083, P3D-100–101 e P3D-131. Evita que cada ferramenta implemente seleção, preview, confirmação e undo de forma diferente.

</aside>

# Selection Domain

A UX pública usa `Object / Face / Edge / Point` no workspace POLY e `Shape / Curve / Point / Region` no workspace DRAW ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)). `Point` é o termo público para vértice. `Tab` não alterna domínio. O estado interno pode ter subestados, mas não deve criar uma segunda verdade pública.

# Selection semantics

Congelar comportamento para replace/add/subtract/toggle, marquee, hidden/locked, X-Ray, seleção via Outliner e seleção sincronizada. Selection é editor/application state; widgets apenas consultam/solicitam mudanças.

# Topologia e invariantes

Auditar o mesh model atual e registrar invariantes: referências válidas, winding, faces degeneradas, edges órfãs, non-manifold quando permitido, normals/tangents e IDs/handles. Toda operação topológica deve validar invariantes após commit em debug/tests.

# Tool lifecycle canônico — gramática única (`ToolSession`)

Revisão de 2026-09-29 ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md); pesquisa no [capítulo 45](../foundations/45-pesquisa-interacao-modelagem-referencias.md)). Todas as ferramentas usam a mesma máquina de estados, implementada uma vez no core:

```
Idle (hover → pré-seleção + dica)
  ├─ press → Pressed ──(solta antes do limiar)──→ clique: seleciona / adiciona ponto
  │                   └─(passa do limiar)──────→ Dragging (alça | haul)
  ├─ atalho, teclado ou preferência "sem segurar" → Latched (segue o mouse; clique termina)
  └─ campo do card "Última operação" → reaplica o gesto confirmado (mesmo Undo)
Dragging/Latched → solta / clica / Enter → Commit → Idle com "Última operação"
                 → Esc → restaura exatamente
Collecting (ferramentas de vários cliques) → clique adiciona · Backspace remove o
  último · Enter, duplo clique ou 1º ponto fecha · Esc sai
```

Regras:

- **Ferramenta persistente:** permanece ativa depois de cada gesto.
- **Clique sem movimento seleciona** em qualquer ferramenta; arrasto além do limiar (em pixels lógicos, configurável) opera a ferramenta, pela alça ou em qualquer lugar do viewport ("haul"). Caixa e laço pertencem à ferramenta Select.
- **Clicar-mover-clicar** (estado `Latched`) é alternativa equivalente a todo arrasto (WCAG 2.2, 2.5.7). O modal estilo Blender (tecla → mouse → clique) é esse mesmo estado.
- **Navegação nunca é suspensa**; a roda sempre faz zoom.
- **RMB abre o menu de contexto e nunca cancela.** `Esc` cancela em escada: arrasto → pontos coletados → ferramenta → Select.
- Sem efeitos ocultos: uma ferramenta sem seleção aplicável mostra uma dica e não seleciona tudo por conta própria.
- Activate/deactivate/cancel não deixam estado fantasma. Troca de tool encerra/cancela explicitamente a operação anterior.

> **Decisão aprovada, implementação em ondas.** Em 2026-09-29 o código ainda tem cinco mecanismos de sessão distintos ([capítulo 45](../foundations/45-pesquisa-interacao-modelagem-referencias.md), seção 2.2). A migração segue a ordem do ADR 006; ferramentas não migradas continuam marcadas como PARTIALLY_COMPLIANT nas matrizes de lacunas.

# Preview vs Commit

Preview é temporário e não gera entradas de Undo. **Cada gesto concluído é uma transação semântica** (1 entrada de Undo). O resultado permanece ajustável no card **"Última operação"** até que outro comando altere o documento: reaplicar restaura o checkpoint do gesto e reexecuta dentro da mesma entrada, sem aumentar o histórico. Cancel restaura exatamente o estado anterior. Uma primitiva criada é um gesto confirmado; `Esc` não a apaga (Undo sim).

# Modal Tool Feedback

P3D-131 fornece guideline/line, delta, axis constraint, snap, numeric input e hints de status de forma reutilizável. A tool descreve feedback; renderer/frontend apresenta sem acoplar algoritmo ao toolkit. O HUD mostra sempre o valor, a unidade, o texto digitado, a trava ativa e o snap que **efetivamente** encaixou.

# Axis, Snap e Numeric Input

X/Y/Z e planos aprovados devem usar a mesma infraestrutura entre Move/Rotate/Scale/Extrude/Inset/Bevel quando semanticamente compatível. Keymaps resolvem teclas; tools consomem intent semântico.

Contrato de entrada numérica:

- dígitos, sinal e separador decimal digitados **durante um gesto** vão para um buffer único do parâmetro principal; não é preciso clicar num campo;
- **depois do gesto**, o valor é ajustado no campo do card "Última operação" (foco por clique ou teclado). Fora de gesto, digitar não captura teclas: atalhos como `1`–`4` (domínio de seleção) continuam valendo, sem modo oculto;
- enquanto o buffer tiver texto, ele vence o mouse; `Backspace` apaga; buffer vazio devolve o controle ao mouse;
- `Tab` troca o parâmetro que recebe o texto enquanto houver gesto ativo (prioridade Modal da constituição 03); fora de gesto, `Tab` navega controles; `Enter` aplica;
- o buffer é **limpo** ao iniciar, confirmar e cancelar qualquer gesto — nenhum valor vaza para a operação seguinte;
- valor inválido ou fora dos limites não altera a prévia nem entra no histórico.

# Undo/Redo

Gestos contínuos são uma transação. Operações topológicas, paint strokes, property drags e transform drags devem declarar boundary de transaction. Redo não depende de widget ou mouse original. Branch após Undo invalida redo de forma previsível.

# Error/invalid-state policy

Operação inválida não corrompe mesh e não entra no history. Se algo puder ser parcialmente aplicado, usar validação/preflight ou transaction rollback.

# Tests obrigatórios

Headless tests por operation; property/invariant tests onde úteis; cancel/confirm; undo/redo; multi-selection; locked/hidden; invalid topology; sequences longas; deterministic replay quando aplicável.

# DoD

Uma nova modeling tool pode reutilizar selection/modal/undo infrastructure sem criar um segundo lifecycle e sem alterar módulos não relacionados.