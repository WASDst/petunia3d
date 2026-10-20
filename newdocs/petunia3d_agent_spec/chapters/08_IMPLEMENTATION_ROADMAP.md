# 08 — Roadmap executável por vertical slices e dependências

**Finalidade:** orientar planning/issue/PR do code agent. Não implementar tudo em um único PR. Prioridade mede benefício para experiência + risco + dependências; não é declaração de conclusão.

## 8.1 Política operacional

Cada slice: (1) baseline/contratos e gap; (2) teste de caracterização ou regressão; (3) domínio pequeno e commands; (4) ViewModel; (5) Qt/GL; (6) Undo/cancel e save/load quando necessário; (7) acessibilidade; (8) evidência funcional/manual e docs/tracker. Executar gating de recursos antes de iniciar outro domínio.

Sem push/merge automático. Reconciliar docs e flows de CI primeiro. Dados de projeto existentes são prioritários; nenhuma migração incompatível sem fixtures/backups e aprovação explícita.

## 8.2 Matriz de épicos

| Fase | Épico / Id sugerido | Dependência | Entregável mínimo verificável | Prioridade |
|---|---|---|---|---|
| 0 | **DOC-QT-001** Fonte da verdade e CI Qt | main atual | AGENTS/README/decisions/CI/tracker coerentes, sem Slint como produção | P0 |
| 0 | **BASE-001** Inventário & testes base | DOC-QT | gap matrix de seleção, gizmo, render, rig; testes críticos caracterizados | P0 |
| 1 | **SEL-CORE-001** Picking/active element | BASE | candidate typed, hit priority, filters, active element | P0 |
| 1 | **SEL-TRANS-001** Transactions | SEL-CORE | protected drag, begin/preview/commit/cancel, 1 Undo | P0 |
| 1 | **SEL-GIZMO-001** Gizmo universal | SEL-TRANS | Move/Rotate/Scale+Pivot com hit separado do visual | P0 |
| 1 | **SEL-SNAP-001** Snap Engine | SEL-TRANS | Pivot/Point-to-Point auto + transforms suaves default | P0 |
| 1 | **VSP-CLAY-001** Clay Soft + silhouette | BASE | modo de trabalho e contorno leve funcional no Qt GL | P0 |
| 1 | **VSP-DIM-001** Bounds e medida | SEL-CORE | world/local/evaluated, frame legível e escala em preview | P0 |
| 2 | **SEL-MULTI-001** Heterogeneous resolver | SEL-GIZMO | subcomponent dedup, individual origins e hierarchy policy | P1 |
| 2 | **VSP-DEPTH-001** Depth comparison | VSP-DIM | axis delta/distances + quality label | P1 |
| 2 | **VSP-COMP-001** Companion manual | VSP-CLAY | Qt second camera synchronized document/selection | P1 |
| 2 | **VSP-SEC-001** Section visualization | VSP-COMP | 1 clipping plane non-destructive + flip, normal view | P1 |
| 2 | **MF-001** Feedback de operações | SEL-TRANS | Extrude/Inset/Bevel etc. use common preview contract | P1 |
| 3 | **VSP-SEC-002** Geometry section | VSP-SEC-001 | contornos múltiplos/holes/open + labels validado | P1 |
| 3 | **VSP-COMP-002** Smart suggestions | VSP-COMP-001 | heuristic projection + opt-in auto | P1 |
| 3 | **VSP-THICK-001** Thickness probe | VSP-SEC-002 | explicit & directional method + quality | P2 |
| 3 | **VSP-PROF-001** Organic profiles | VSP-SEC-002 | manual centerline + stations stable frames | P2 |
| 3 | **RIG-MAN-001** Manual Rig Editor | BASE | joint/hierarchy/weights/pose with Undo | P1 |
| 3 | **RIG-AUTO-001** Assisted autorig | RIG-MAN | landmark/preset preview/correction/commit | P1 |
| 4 | **SSG-001** Structural Shape Graph | VSP-PROF/RIG-AUTO | serializable parts with stable IDs+connections | P2 |
| 4 | **RIG-LIVE-001** Incremental Rigging | SSG/RIG-AUTO | localized rig delta, override/lock policy | P2 |
| 4 | **RIG-ADV-001** Advanced weights/pose | RIG-LIVE | solver improvements, deformation QA & export | P2 |
| 5 | **VSP-ADV-001** Surface Flow/contours | VSP-CLAY | curvature/temporal coherence when justified | P3 |
| 5 | **RIG-AI-001** Optional provider | RIG-MAN/RIG-AUTO | offline baseline intact, plugin isolated | P3 |

## 8.3 Fase 0: saneamento após migração Qt

1. Conferir `AGENTS.md`, `website/docs/00-philosophy/decision-register.md` (procurar ADR-C11), `website/docs/manifest.json`, `16-code-agents` e `Cargo.toml` atual.
2. Atualizar docs que dizem Slint atual; remover/rotular claramente históricos sem destruir proveniência.
3. Substituir CI `ui-slint.yml` por gate Qt/Editor com Qt 6 dev e OpenGL headless adequado. Preservar demais gates. Se CI requer Qt instalado, configurar explicitamente no runner.
4. Corrigir artefatos gerados drift com `xtask docs-generate` e revisar diff.
5. Corrigir 15 deadlinks no VitePress (não ignorá-los em massa).
6. Adicionar link deste pacote ao caderno canônico, **se usuário autorizar edit de repo**; reformatar como documentos do `website/docs/`/manifest conforme padrão local.
7. Auditar `SlintUiBridge` como nome legado e outras fronteiras; não promover renames arriscados sem caso.

**Definição de pronto da fase 0:** README/AGENTS/decisions coerentes com Qt + todos os workflows essenciais configurados e verdes ou impedimentos claramente evidenciados; snapshot baseline e tracker atualizados; compilação sozinha não certifica Qt nativa.

## 8.4 Guias de fatiamento por feature

### Selection/Gizmo

Primeiro separação de hit test vs render; depois transação segura; depois frame de pivô; então heterogênea e snap. Testar no ambiente GL Qt com zoom/HiDPI e cancel.

### Petunia Clay

Começar Soft com iluminação independente e silhouette pass, sem trocar de material do documento. Extrair parâmetros em Rust DTO, UI Qt com preferências persistentes. Cavity macro e MatCap em segunda rodada; evitar exigir versão GL >3.3.

### Volume/Companion

Reusar bounds disponíveis e retorno semântico. Disponibilizar painel de medida acessível antes de dezenas de overlays. Para Companion reutilizar viewport secundária existente, confirmar câmera independente do PiP. Introduzir sugestão contextual somente após resolução manual estável.

### Section/Thickness/Profile

Clipping GL leve primeiro; num segundo PR extração de contornos geométricos e holes; somente depois thickness/organic profiles. Section Only temporário e logs explicativos; shapes/malhas abertas não recebem preenchimento fictício.

### Smart Rig

Auditar primeiramente rig+IK+LBS+export/undo existente. Rig Editor manual vem antes de autorig live: corrigir pesos e joints precisa estar disponível independentemente de automação. Depois assisted landmark/weight preview. SSG só após migrar persistência e provar round-trip; incremental com simulações de resize/add/remove, bloqueios manuais e transações.

## 8.5 Acceptance por slice

Cada entrega precisa especificar: objetivo observável; paths afetados; comandos/queries; testes de domínio; teste de bridge; teste GL/manual quando pertinente; impacto no arquivo `.petunia`; comportamento keyboard/a11y; benchmark se GPU/cálculo pesado; docs/revisão/CI; bugs ou limitações descobertas.

### Exemplo de task card

```yaml
task:
  id: "VSP-COMP-001"
  goal: "Abrir uma Companion View ortográfica com câmera própria e seleção sincronizada"
  baseline_sha: "<git rev-parse HEAD em execução>"
  branch: "<branch autorizada pelo usuário>"
  code_map: ["crates/ui-cxxqt/src/render/opengl_viewport.rs", "crates/ui-cxxqt/cpp/petunia_main_window.cpp", "crates/editor/src/view_model.rs"]
  invariants:
    - "câmera principal não muda"
    - "mesmo documento, uma autoridade de seleção"
    - "Qt GL context/thread correto"
    - "fecha ou fixa sem perder seleção"
  tests:
    - "selection and preview visible in both views"
    - "reset/resize/DPI/context recreation"
    - "keyboard and focus"
    - "low-end performance"
  non_goals: ["reabrir gizmo", "trocar OpenGL por WGPU", "implementar rig"]
  permission_scope: "sem push/merge até autorização"
```

## 8.6 Riscos e mitigação

- **Fragmentação de UI/contexto:** DTOs semânticos e única authority; não duplicar hit/selection em Rust e Qt.
- **Performance:** caches por revisão, atualização diferenciada fast/deferred, segundo viewport apenas quando necessário.
- **Geometria ambígua:** quality methods e reason, nunca false exactness.
- **Topologia/rig quebrados:** estáveis refs e versioned migrations, testes de mesh modifier/pose/export.
- **Usuário iniciante sobrecarregado:** perfis adaptativos, não intrusão, comparação empírica de UX.
- **CI herdada Slint:** saneamento de workflows/docs em fase zero.
- **Escopo inflado:** vertical slice pequeno, sem framework genérico novo sem necessidade.
