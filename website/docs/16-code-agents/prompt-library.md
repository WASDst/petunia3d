# Biblioteca de prompts — implementação e auditoria

> Prompts de trabalho, não instruções para ignorar autorizações. Substitua placeholders por dados **confirmados** no repositório. A descrição da tarefa não supera as decisões canônicas nem permite merge em main.

## Padrão universal de preenchimento

**Campos obrigatórios:** `[TASK_ID]`, `[OUTCOME]`, `[CANONICAL_DOC]`, `[TARGET_PATHS]`, `[BEHAVIOR]`, `[NON_GOALS]`, `[GATES]`. Não assuma status de implementação a partir da documentação.

### P0. Prompt principal — construir uma feature

```text
Atue como coordenador de implementação do Petunia3D.
Repositório: WASDst/petunia3d.
Branch ÚNICA permitida: refactor/architecture-foundation. Nunca alterar main ou fazer merge.

Objetivo: [OUTCOME]   Task: [TASK_ID]
Documentos canônicos: [CANONICAL_DOC]
Limites: [TARGET_PATHS], [NON_GOALS]
Comportamento esperado: [BEHAVIOR]
Gates obrigatórios: [GATES]

Leia AGENTS.md → website/docs/16-code-agents/index.md →
reading-navigation.md → implementation-protocol.md → decision-register
→ capítulos relevantes → código, call sites, testes e manifests.
Não leia o caderno inteiro nem use docs/bible como autoridade desta branch.

Monte uma Gap Matrix COMPLIANT / PARTIALLY_COMPLIANT /
FUNCTIONAL_BUT_DIFFERENT / RUDIMENTARY / STUB / BROKEN /
DUPLICATED / MISSING / OBSOLETE. Preserve o que funciona.
Proponha menor vertical slice, mapeie ownership, Undo, IDs, revisões,
segurança, a11y, performance e compatibilidade.
Implemente de modo incremental sem duplicar domínio nem quebrar projetos.
Adicione testes relevantes; rode verificações disponíveis. Não enfraqueça
gates para obter verde. Atualize docs canônicos se contrato mudou.
Registre commit SHA, paths, comandos/resultados, gates NOT RUN e riscos.
Se houver blocker, documente exatamente o blocker e a mitigação segura.
```

### P1. Auditoria de implementação versus documentação

```text
Investigue [SUBSISTEMA] sem presumir que a documentação corresponde ao código.
Leia apenas os contratos do domínio em website/docs/, callers reais,
manifests, testes e runtime entrypoints. Trace o caminho UI/CLI/MCP →
Application → Domain → Document/Renderer e retorno.
Produza: Gap Matrix com evidência por arquivo/símbolo; inventário de código
morto/duplicado; risco por severidade; classificação Reuse/Refactor/Move/
Rewrite; correções priorizadas P0/P1/P2; testes que faltam.
Não implemente enquanto atuar como reviewer/explorer sem permissão write.
Não use nomes de arquivos ou mocks como única prova de funcionalidade.
```

### P2. Bug fix com regressão

```text
Reproduza o bug [SYMPTOM] com [STEPS] no commit [SHA].
Encontre o primeiro ponto onde o estado observado diverge do contrato
[CANONICAL_DOC]. Crie teste de reprodução (ou documente por que não é
automatizável). Corrija a causa no menor owner correto, sem workaround
em camadas vizinhas. Valide normal + erro + Cancel + Undo quando aplicável.
Teste keymaps/HiDPI/focus se UI; stale revisions se worker; malformed
inputs se IO. Entregue antes/depois e resultado real dos testes.
```

### P3. Geometry — Shape-first / DRAW / POLY

```text
Revisar e implementar [TOOL] segundo website/docs/03-geometry/
draw-unification-proposal.md e [GEOMETRY_CHAPTER].
DRAW/POLY devem compartilhar documento, ToolSession, snapping,
Geometry Source e Undo; DRAW é perfil planar, não outro motor.
Curvas persistem como SplineResource, formas como PlanarShape e Mesh
autoral mantém FaceCorner UV. Defina preview, commit/cancel, remapping,
topology validation, Make Editable explícito e input numérico.
Teste faces tri/quad/ngon, splines/holes, non-manifold, rollback,
save/load e alternância DRAW↔POLY. Não criar ECS ou Mesh paralela.
```

### P4. Slint — UI, estados e acessibilidade

```text
Implementar [UI_FLOW] na arquitetura Slint atual, de acordo com
website/docs/09-ui/slint-rescue-plan.md,
workspaces-feedback-accessibility.md e viewport-input-boundary.md.
Inspecione controles base e callbacks antes de criar novo widget.
Manter Structure/Properties/Drawer por workspace e viewport-first.
Criar estados default/hover/pressed/focused/selected/disabled com
feedback não dependente só de cor. Nome/descrição acessível, teclado,
F6, focus restore, High Contrast, Reduced Motion, 100-200% scale,
NumericField/slider consistentes, click-move-click quando gestual.
Emitir intents semânticos, nunca mesh edit em Slint. Rodar UI lint,
regressão de gestos, testes de foco e smoke manual identificado.
```

### P5. Viewport GL / performance low-end

```text
Trabalhar na integração Slint + petunia-render OpenGL 3.3 para [FLOW].
Estudar website/docs/04-renderer/architecture-decision.md e
14-quality/performance-jobs-budgets.md.
Preservar GPU ownership, FBO/GL texture sem readback por frame,
state restoration, resize/HiDPI, picking coords, render-on-demand.
Split2 compartilha recursos; vista passiva não compete em gestures.
Testes com host de referência real (Linux/Windows/low-end) e logs.
Não adicionar WGPU como backend de produção nem alegar FPS sem medição.
```

### P6. PAINT / UV / Project Photo

```text
Implementar [FEATURE] a partir de 06-paint/architecture-decision.md,
06-paint/workspace-ux.md, 07-uv/architecture-decision.md,
07-uv/workspace-ux.md e 08-reference-projection/architecture-decision.md,
selecionando apenas capítulos realmente relacionados.
Preservar layers, canais/UV FaceCorner, referência, projection bake,
SurfaceAttachment e shared-resource warnings. Preview não altera
Document; stroke = um Undo; alterações de textura usam dirty tiles.
PAINT 2D/3D/Split e UV Work Surface têm UI e foco previsíveis.
NUNCA fazer auto-unwrap, Bake, remesh ou destructive projection silenciosos.
```

### P7. ANIMATE procedural-first

```text
Atuar sobre [ANIMATE_FLOW] segundo 10-animation/workspace-ux.md.
Preservar Creature → Motion → Preview/Transport como caminho principal.
Separar autoral, evaluated pose e ToolSession; ajustes durante preview
não poluem Undo. Indicar funcionalidade não implementada honestamente:
Graph Editor, Tweak Layers, trails e ghosts não são assumidos prontos.
Teste timeline transitions, cancel/replay, compatibility, skin/rig
e acessibilidade das propriedades e do feedback.
```

### P8. File format / Assets / Import / Export

```text
Modificar [IO_FEATURE] segundo 11-project/project-persistence.md,
assets-resource-management.md e 12-interchange/import-export-game-ready.md.
Staging + strict validation, IDs estáveis, revisão e dirty únicos,
atomic save, recovery independente, library resource refs explícitas.
Import externo é untrusted; testar path traversal, oversized, formatos
corrompidos e cancelamento. Export resolve Geometry em snapshot sem
modificar authoring; warnings de perda/bake precisam de opt-in.
Provar round-trip e migração de fixtures legadas em Linux/Windows.
```

### P9. Lua Plugin / MCP Security

```text
Implementar [PUBLIC_ACTION] somente por Application Commands/Queries
conforme 13-extensions/lua-plugins-public-api.md e
mcp-automation-agents.md. Nenhum acesso direto a Mesh interna, Slint,
GL ou file IO fora de capability. Declarar schemas, IDs, scopes,
authorization, Undo, stale revision, errors, timeout/cancel e audit.
Escrever testes de PermissionDenied, revogação, rollback de batch,
plugin unload e paths não autorizados. Não introduzir exec arbitrário.
```

### P10. Revisão de segurança + acessibilidade independente

```text
Atue como reviewer independente do changeset [SHA/DIFF] com critérios
[CANONICAL_DOC]. Não modifique código neste papel. Faça análise de
trust boundaries, dependencies, stale state, Undo e falhas. Em UI,
inspecione keyboard navigation, F6, focus restoration, screen reader
semantics, contrast, scale, reduced motion, cognitive load e nomes.
Entregue achados em P0/P1/P2 com paths, reprodução, esperado, observado,
provas, status pass/fail/not-tested e ações recomendadas.
Não confundir ausência de ferramenta de teste com aprovação.
```

### P11. Refatoração limpa / código morto

```text
Audite [MODULE] e identifique código OBSOLETE versus usado por
consumidores ativos, formatos legados e feature gates. Confirme
callers e tests antes de remover. Separe Move/Refactor de mudanças
comportamentais. Remova duplicação após provar contrato equivalente,
com rollback planejado. Prefira nomes explícitos, ownership claro,
erros tipados e menos dependências. Não refatore áreas adjacentes
por estética ou para diminuir LOC isoladamente.
```

### P12. Documentação para LLM / drift

```text
Atue como documentation-maintainer. Compare [CHANGED_CONTRACT] com
website/docs/manifest.json, decision-register.md e código real.
Atualize uma única página canônica por fato, use links relativos,
títulos estáveis, status de implementação honesto e linguagem acessível.
Verifique links/caminhos, navegação, conteúdo repetido, antigas
afirmações egui/WGPU, referências a docs/bible e gates incorretos.
Gere ContextPack pequeno, uma nota de migração e changelog pertinente.
Nunca apresente especulação como comportamento já implementado.
```

### P13. CI com falha e revisão de evidências

```text
Analisar [WORKFLOW_RUN] no commit [HEAD].
Confirmar checkout/branch e a primeira etapa que realmente falhou.
Ler logs completos pertinentes; separar falha de infra de erro do código.
Corrigir causa mínima e retestar no novo HEAD; não invocar sucesso de
run cancelada, de outra SHA ou de outro conjunto de features.
Mostrar matriz: format, lint, clippy, unit, integration, UI visual,
OpenGL real, a11y, Linux/Windows — PASS/FAIL/NOT RUN/BLOCKED.
```

## Como combinar prompts

Use P0 como envelope e **apenas um prompt de domínio** (P3–P9), mais P10 para revisão quando necessário. P1/P2 são alternativas de investigação; P12/P13 entram no fechamento. O agente não precisa receber toda esta página repetida no contexto — bastam o ID e o trecho escolhido.
