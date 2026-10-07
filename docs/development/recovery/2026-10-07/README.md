# Fila preservada da retomada unificada

Esta pasta reúne os sete conjuntos de trabalho não commitado encontrados nas
worktrees em 07/10/2026. A branch `codex/unified-gradual-recovery` é o ponto único
de retomada. O `manifest.json` registra o commit-base, arquivos novos e SHA-256
de cada patch e arquivo tar. Os patches estão comprimidos em `.patch.gz`; o manifesto também registra o hash dos bytes descomprimidos, idênticos ao diff original. Descomprimir antes da leitura e reconciliação. As worktrees de origem não são mais necessárias
para ler ou recuperar esses deltas, mas são mantidas durante a validação.

**São snapshots de recuperação, não código ativo nem uma segunda autoridade
documental.** Textos canônicos dentro de um patch só valem após reconciliação e
edição no caderno. Nunca aplicar a fila inteira automaticamente: os snapshots
partem de bases diferentes e alguns já foram recuperados na main.

| Snapshot | Ordem / estado | Critério de incorporação |
|---|---|---|
| `paint-ui` | Próximo após POLY e UI commitada | Máscara/grupo/escopo e biblioteca; preservar Decal Set, âncora 3D, tiles e canvas atuais. |
| `material-channels` | Composição compartilhada com PAINT, depois fechar W1 | Unificar canal + máscara + grupos; UI/export ainda faltam. |
| `draw-gestures` | Loop Cut incorporado parcialmente; demais deltas pendentes | Criação por gesto, planos/nós/parede com furo e seção em sólidos booleanos; não regredir ToolSession, keymap e cache. |
| `profile-parts` | Reconciliar com DRAW atual | Incorporar somente funcionalidades/testes não cobertos pelos patches mais recentes. |
| `ui-contracts` | Depois do shell consolidado | Adaptar golden MODEL/UV para DRAW/POLY/PAINT e novas seções antes de ativar. |
| `svg-hardening` | Equivalente na main | Não reaplicar; comparar se surgir um requisito ainda não coberto. |
| `projection-stencil` | Equivalente na main | Não reaplicar versões anteriores às correções atuais. |

Procedimento de cada item:

1. Verificar hashes e ler a matriz do requisito no caderno vigente.
2. Comparar patch + arquivos novos com a árvore atual; registrar COMPLIANT,
   DUPLICATED ou delta funcional por requisito.
3. Trazer somente o delta, com testes de regressão, sem sobrescrever arquivos
   inteiros de uma base antiga nem escolher um lado de conflito em silêncio.
4. Validar comportamento, persistência/Undo quando aplicável e gates do projeto.
5. Atualizar matriz e estado; commit, push e verificar SHA remoto.
6. Manter o snapshot até a equivalência/integração estar comprovada.

O plano operacional e os recibos vivem em
`docs/development/unified-recovery-gap-matrix-2026-10-07.md` e `PROJECT_STATE.md`.
W2/W3 começam após W1; W4/W5, Animate e extensibilidade seguem o plano canônico.
Features egui exclusivas permanecem no histórico como legado, sem expansão da
superfície de produção nem reabertura do site congelado.
