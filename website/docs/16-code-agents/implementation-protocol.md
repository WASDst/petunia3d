# Protocolo de implementação para LLMs

> **Status: obrigatório para tarefas de implementação nesta branch.** Derivado da filosofia Petunia3D e do processo de Prumo; a documentação do Prumo orienta papeis, mas não autoriza acessos ou merges.

## Contrato de entrada mínimo

```yaml
task:
  id: "<issue or task identifier>"
  goal: "<observable user value>"
  branch: "refactor/architecture-foundation"
  baseline_sha: "<confirmed HEAD>"
  canonical_docs: ["website/docs/..."]
  affected_paths: ["crates/..."]
  constraints: ["preserve working behavior", "Windows/Linux", "accessibility"]
  non_goals: ["unrelated cleanup", "second renderer/frontend"]
  acceptance: ["behavior", "regression", "integration", "a11y when relevant"]
  execution_gates: ["relevant cargo tests", "UI/GL/manual gates if changed"]
  permission_scope: "only explicitly authorized operations"
```

Não tratar placeholders como valores: preencher via Git/source antes de editar. Se branch divergir, parar commits à branch inesperada.

## Etapa A — Reconhecimento

1. Confirmar estado Git, branch, HEAD, arquivos modificados e jobs de CI existentes.
2. Ler [protocolo de navegação](./reading-navigation.md) e capítulos canônicos pertinentes.
3. Localizar implementação, callers, boundary, testes e serialização relevante.
4. Construir Gap Matrix. Anotar o que pode ser **preservado integralmente**.
5. Definir riscos cross-cutting: undo, save/load, rendering, security, a11y, performance, platform.
6. Escrever plano pequeno orientado a *vertical slice*, com checkpoints independentes.

**Não iniciar reescrita de módulo porque parece monolítico sem medir risco de decomposição.**

## Etapa B — Design mínimo

A arquitetura de cada mudança explicita:
- input/outputs tipados e erros;
- owner de estado e revisão;
- boundary de UI e domain;
- sessão/transaction/Undo;
- cache e invalidações;
- compatibilidade de formato/API;
- condições de cancel/stale e falha;
- acessibilidade e comunicação de feedback.

Para nova ferramenta:
```text
Physical input
→ Slint gesture router / keymap semantic action
→ UiIntent / typed DTO
→ Application ToolSession + Commands/Queries
→ pure Geometry/Paint/UV/Animation algorithm
→ Document transaction (single writer)
→ events / invalidations / evaluated render state
→ Slint accessible feedback
```

O código não deve criar nova rota `Slint callback → mesh.faces[..]`.

## Etapa C — Implementação

- Criar teste de regressão que caracterize o bug sempre que possível.
- Alterar o menor conjunto coeso de arquivos; preservar API e contrato autoral.
- Preferir refatoração com equivalência e testes, nunca big-bang.
- Nomear conceitos para reduzir ambiguidade; newtypes e erros estruturados em boundaries.
- Evitar `unwrap` em entrada externa, `unsafe` sem justificativa, hardcode de keymap na Geometry e duplicação de constantes sem tokens.
- Executar gate focado e corrigir causas; não apagar asserções, não silenciar diagnósticos para obter verde.
- Alterações de projeto/formatos requerem migrations/fixtures e round-trip.
- Fechar operações transacionais e previews com cancel e Undo coerentes.

## Etapa D — Verificação por responsabilidade

| Área alterada | Gates necessários |
|---|---|
| Rust puro | fmt, build/check, testes unitários/regressão, Clippy do crate |
| Geometry / tools | invariants, topology, FaceCorner, preview/commit/cancel/Undo |
| Project / asset | legacy round-trip, corrupt input, atomic save, ID/revisions |
| Slint | keymaps remapeados, gestures, focus/hover/active/disabled, UI scale |
| Viewport OpenGL | FBO/texture sem readback/frame, resize/HiDPI, picking, state restore |
| Paint/UV | strokes, layers, selection, pressure/scale quando aplicável, UV invariants |
| Animation | procedural preview, transport, rig/pose, persistência e reversal |
| Lua/MCP | capabilities, deny-by-default, trust boundaries, transactions, stale IDs |
| Docs/site | manifest registration, routes, link validation, no legacy authority |
| Release | Linux/Windows, low-end, smoke e independent reviewer evidence |

**Campos não executados ficam `not run`**, não pass. Testes manuais são distintos de CI.

## Etapa E — Revisão independente

Quando disponível, repassar diff/contextpack para `tester` e `reviewer`, anexando `accessibility-reviewer` para UI e `security-reviewer` para entradas externas/capabilities. Revisão verifica comportamentos e regressões, não só estilo.

Para PR/branch CI, investigar **o primeiro erro primário** e não repetir mudanças cosméticas para acalmar um lint. Se o check passar com stubs, considerar `implemented, unverified`.

## Etapa F — Documentação e handoff

- Atualizar capítulo canônico quando decisões, UX, contrato público ou expectativas mudarem.
- Registrar status real da implementação (Approved/In migration/Verified).
- Garantir links do site e catálogo.
- Revisar diff por escopo, credenciais e mudanças inesperadas.
- Commit apenas na branch autorizada e somente com permissão. Nunca merge na main por inferência.
- Finalizar com [formato de evidência](./evidence-handoffs.md).

## Anti-patterns proibidos

Refactor que apaga funcionalidades; implementar tudo em `callbacks.rs`; solucionar bug com segunda fonte de verdade; acessar Geometry a partir de widget; fingir testes via mock único; duplicar engine de DRAW; criar UI "acessível" separada em vez de semântica comum; exagerar abstrações para futuro hipotético; rodar scripts externos sem revisão; alterar direitos/capabilities para "facilitar"; simular benchmarks.
