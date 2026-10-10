# Evidência, revisão e handoff entre LLMs

> O objetivo é permitir que um segundo agente assuma o trabalho **sem acreditar em resultados fabricados, sem reler o repositório inteiro e sem perder decisões**.

## Status de implementação versus execução

**Estado de implementação:** `approved`, `in migration`, `implemented-unverified`, `verified`, `blocked`.

**Estado de cada gate:** `pass`, `fail`, `not run`, `blocked`, `not applicable`. `pass` exige execução na SHA informada, com comando e resultado. `not applicable` exige justificativa.

`Compiled` é diferente de `tested`; `tested` é diferente de `usable`; `usable` não prova `accessible`; `accessible` manual não significa certificação formal.

## Contrato de saída normalizado

```yaml
handoff:
  repository: "WASDst/petunia3d"
  branch: "refactor/architecture-foundation"
  head_sha: "<actual current commit>"
  task_id: "<task>"
  intent: "<desired user observable outcome>"
  doc_sources:
    - path: "website/docs/..."
      heading: "..."
      decision: "approved|in migration"
  implementation:
    status: "implemented-unverified"
    changed_paths: ["crates/..."]
    behavior: ["what genuinely changed"]
    deferred: ["explicitly untouched features"]
  verification:
    - gate: "cargo fmt -p petunia_ui_slint -- --check"
      status: "not run"
      result: "No toolchain available in this environment"
      sha: "<HEAD>"
    - gate: "Manual screen reader (Windows)"
      status: "not run"
      result: "Needs a real AT environment"
      sha: "<HEAD>"
  risks:
    - severity: "P1"
      cause: "..."
      mitigation: "..."
  next:
    owner_role: "tester"
    smallest_action: "<single concrete action>"
    prerequisite: "<exact path/command>"
```

Preencher com dados reais, não publicar um YAML contendo placeholders sem explicar que são placeholders.

## Reviewer checklist

| Pergunta | Prova esperada |
|---|---|
| Mudança respeita decisões? | cite capítulo/heading canônico |
| Código é de fato alcançável? | call graph e runtime entrypoint |
| Há fonte duplicada de estado? | ownership por type/field |
| Cancel/Undo consistem? | teste exercitado com assert de before/after |
| Não há regressão de arquivos? | fixture round-trip/migration |
| Entrada externa é segura? | teste para arquivo corrompido, path e capability |
| UI tem estados e semântica? | keyboard/screen reader/contrast/evidence |
| FPS/memória melhoraram? | benchmark com ambiente e comparação |
| Claims de pronto são verdadeiros? | gates por SHA, não mock ou promessa |
| Foi modificada só branch permitida? | git status/log/head antes/depois |

## Escala de severidade

- **P0:** corrupção de projeto, bypass de autorização, crash comum, regressão severa a11y, operação destrutiva silenciosa.
- **P1:** bloqueio de fluxo principal, comportamento divergente dos contratos, testes relevantes quebrados, estado inconsistente.
- **P2:** ergonomia/perf moderada, documentação parcialmente desatualizada, melhorias localizadas.
- **P3:** refinamento cosmético e conveniência pós-baseline.

Escalonamento não permite passar por cima de instruções explícitas de segurança ou autoridade.

## Como fazer checkpoint para nova conversa

1. Escreva estado atual, não histórico narrado em excesso.
2. Cite **HEAD + branch** e diff/commit se houver.
3. Cite 2–5 arquivos canônicos relevantes.
4. Liste gates realizados com status, jobs URL e bloqueios.
5. Deixe apenas uma próxima ação concreta de maior prioridade.
6. Não afirme "nenhuma pendência" se faltam testes manuais.

## Exemplo de relatório humano legível

```text
Task: Refatorar Input Boundary — pointer cancel
Branch: refactor/architecture-foundation
HEAD: [SHA confirmado]
Decisão: viewport-input-boundary.md § [heading]
Gap inicial: BROKEN — cancel não encerrava gesture
Mudanças: router.slint e teste de keymap
Passou: [comando + SHA + link do runner]
Não executado: Windows HiDPI, screen reader
Risco: lógica de foco em context menu ainda sem teste manual
Próximo: tester executar [cenário] em [plataforma]
```

O relato acima é **modelo**, não resultado de teste real desta sessão.
