# Orquestração, workforce e separação de papéis

> **Status: política operacional para a branch.** [Prumo Workforce](https://github.com/poppy-lat/prumo/tree/main/src/prumo/resources/workforce/) é um conjunto externo de papéis, skills e receitas. Referenciar um `AGENT.md` não instala nem executa automaticamente esse agent.

## Filosofia

Selecione **menos agentes, melhores gates**. Um agent responsável conduz cada Task, especialistas só entram por risco específico, reviewer/tester verifica independentemente quando possível. Evite um swarm que replique arquivos e produza consenso sem executar código.

### Fluxo padrão

```text
Human Goal / task autorizada
    ↓
Explorer — mapa de arquivos, estado real, fontes canônicas
    ↓
Architect — plano mínimo, restrições, APIs e migração (se estrutural)
    ↓
Implementer / Specialist — mudança estreita com testes
    ↓
Tester — regressão e evidência independente
    ↓
Reviewer — diff, invariantes, dependências, riscos
    ├ Accessibility Reviewer — sempre que interface muda
    ├ Security Reviewer — quando input externo/permissions muda
    ├ Performance Agent — quando hot path/renderer/cache muda
    └ Quality Reviewer — quando conformidade e integração mudam
    ↓
Documentation Maintainer — contrato/site/manifests
    ↓
Coordinator/Human — verifica gates e decide aceitação/release
```

Prumo nomeia `architect`, `implementer`, `tester`, `reviewer`, `explorer` e `documentation-maintainer` como papéis. O Petunia3D adiciona **gates de domínio**; não muda permissões internas desses papéis por inferência.

## Seleção e responsabilidade

| Papel | Quando chamar | Saída | Limite |
|---|---|---|---|
| explorer | início de tarefa não trivial | mapa de código + ContextPack | não decide produto |
| architect / systems-architect | muda ownership/contracts | ADR pequeno, grafo e migração | não supõe user approval para mudar escopo |
| implementer | Task com plano aprovado | diff mínimo + testes | não se autoaprova |
| engine-engineer | Geometry, tools, jobs e engine | algoritmo/contrato implementado | sem UI como dono de domínio |
| editor-engineer | Slint workflows, sessions e UX | interação + bridge coerentes | sem mutação diretamente no widget |
| renderer-engineer | GL, GPU, viewport | pass, texture/state/caches corretos | sem segundo renderer |
| ux-architect / design-system-engineer | mudança perceptível de fluxo | interaction states e design contracts | sem reinventar padrões a cada painel |
| accessibility-reviewer | toda mudança de UI | relatório com evidência e severidade | papel review não edita código |
| tester / reviewer | fim de cada vertical slice | evidência, bugs e review do diff | preferir independência |
| security-reviewer / security-architect | IO, plugins, MCP, injection | threat model e findings | não relaxa capabilities |
| performance-agent | hot paths, buffers, jobs, low-end | benchmark reprodutível | não inventa números |
| documentation-maintainer | mudança em contrato/site | pages canônicas alinhadas | não contradiz decisão ratificada |
| release-verifier | build/distribuição | matriz de plataformas | só declara verified com gates reais |

## Skills de base e complementares

**Rotina mínima recomendada por tipo:**
- Código Rust: `lang-rust`, `clean-code`, `code-quality`, `grounded-implementation`, `testing-quality`.
- Arquitetura: `architecture-quality`, `dependency-management`, `refactoring`, `implementation-reality-verification`.
- GUI: `accessibility`, `keyboard-accessibility`, `screen-reader`, `focus-management`, `cognitive-clarity`, `design-psychology`.
- Docs para LLM: `documentation-for-llms`, `lean-progressive-context`, `documentation`, `prumo-navigation`.
- Segurança: `secure-coding`, `filesystem-security`, `mcp-security`, `plugin-security`, `untrusted-project-security`.
- Performance/GL: `rendering-3d`, `shaders`, `performance-native`, `benchmarking`.
- Visual/identidade: `design-system`, `design-tokens`, `typography-system`, `color-science`, `icon-system`.

"Rotina mínima" **não quer dizer ativar todas** em toda Task. Cada task usa as relevantes. Nenhuma skill substitui documentos canônicos. Consulte links individuais no [índice completo](./workforce-index.md).

## Receitas do Prumo

`feature-standard` é referência-base para mudanças comuns. Outras:
- `bug-fix` para regressões;
- `architecture-change` para boundary/ADR;
- `engine-renderer` para GL/viewport;
- `ui-feature` / `ui-review` para shell e acessibilidade;
- `documentation-refactor` para doc drift;
- `security-review` para trust boundaries;
- `release` apenas com solicitação real de release.

Todas as 20 recipes estão linkadas no [catálogo](./recipes-catalog.md), inclusive especialidades não centrais ao Petunia3D.

## Controle de permissão (crítico)

Prumo registra `allowed_capabilities`, `required_capabilities`, `permissions` e `review_requirement` em manifests. Isso **descreve o papel**, não concede autorização real ao assistant/CLI. Compare com:
- permissão explícita do usuário;
- escopo do repositório e branch;
- política de execução atual;
- features/capabilities do host.

Um `accessibility-reviewer` é auditor, não implementador; um `implementer` não pode declarar independência por revisar sua própria alteração. Agentes do Prumo podem ter diretrizes `never commit, push, merge`; **respeite as restrições por papel**, e faça commits apenas via coordenador/função autorizada, jamais auto-merge.

## Paralelismo seguro

É adequado paralelizar **análises independentes** (Geometry vs. A11y; IO vs. Security) quando compartilharem referência imutável e cada saída for rastreável. Não permita vários writers concorrentes para o mesmo arquivo/branch sem orquestração que controle commits e revisões; resolver conflitos com controle de head, não force push.

Nunca misturar diff de Slint Rescue, geometry migration e formatos do projeto em um único changeset gigante sem necessidade.

## Handoffs duráveis

Todo repasse contém Task ID, goal/acceptance, links canônicos, commit SHA, paths reais, decisão mínima, comandos e status, riscos, nenhuma permissão implícita e gate ainda não executado.

A conclusão real é *implemented → reachable → exercised → evidenced → verified → accepted → released*. Somente evidência relevante ao HEAD move a etapa; nenhuma mensagem "Done" substitui testes.
