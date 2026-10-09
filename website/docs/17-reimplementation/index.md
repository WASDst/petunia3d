# Acompanhamento da reimplementação

> **Status: contrato operacional aprovado pelo pedido do usuário em 2026-10-09.** Este módulo acompanha a execução dos contratos de `website/docs/` na branch `refactor/architecture-foundation`. A tabela não substitui decisões de produto nem evidências de teste.

[Abrir a tabela de processos](#/progress)

## Fonte única

`website/progress/tasks.json` é a fonte versionada dos processos, checkpoints, estados, datas e evidências. O site lê esse arquivo diretamente, sem build, servidor de escrita ou status privado em localStorage. Alterar código sem atualizar a tarefa correspondente deixa a entrega incompleta.

Cada tarefa tem ID estável, etapa, descrição do delta, documento principal, leituras complementares, checkpoints de aceitação e estado. Todos os capítulos do caderno inicial estão associados a processos; catálogos e prompts de agentes orientam o processo D02, sem se tornarem features do editor.

## Três estados

| Estado | Regra |
|---|---|
| `TODO` | Migração/gates ainda não iniciados; zero checkpoints concluídos. Código legado pode existir e deve ser preservado. |
| `IN PROGRESS (000%)` | Trabalho iniciado com gates ainda abertos. O percentual é calculado pelos checkpoints concluídos e exibido com três dígitos. |
| `DONE` | Todos os checkpoints do escopo delimitado concluídos, cada um com evidência; gates aplicáveis executados. |

Percentual = `floor(100 × checkpoints concluídos / total de checkpoints)`. Exemplo: dois de seis = `IN PROGRESS (033%)`; trabalho iniciado sem checkpoint concluído = `IN PROGRESS (000%)`. Todos têm peso igual: o número mede critérios atendidos, **não tempo, esforço ou percentual global do produto**. Não arredondar para 100% com critério aberto.

Uma tarefa pode voltar de DONE para IN PROGRESS quando uma regressão invalidar a evidência. Reabrir o checkpoint afetado e explicar a causa. Não criar quarto estado para bloqueios: manter IN PROGRESS e registrar impedimento, próximo passo e gates `blocked` na evidência do capítulo.

## Atualização obrigatória a cada implementação

1. Antes de implementar, localizar os IDs afetados, confirmar documento, código e critérios. Marcar IN PROGRESS quando iniciar; atualizar `updated` na tarefa e no arquivo.
2. Ao entregar um slice, marcar somente os checkpoints comprovados. Anexar referência à entrada de `evidence` com revisão/ambiente, resumo e documento que registra os comandos realmente executados, resultados e limites. Não usar aprovação de ADR como prova.
3. Se houver mudança de escopo, dividir/adicionar tarefa com ID novo e critérios verificáveis. Preservar IDs existentes; não apagar trabalho entregue nem alterar o denominador apenas para melhorar a porcentagem.
4. Registrar no capítulo canônico os gates `not run / pass / fail / blocked`, a revisão validada, riscos e próximo checkpoint. Gates manuais necessários ainda abertos impedem DONE no processo que os exige.
5. Atualizar status, checkpoints, evidência e datas **no mesmo conjunto de alterações da implementação**. Se nada puder ser concluído, atualizar baseline/impedimento/data mesmo que o percentual permaneça.
6. Executar os validadores abaixo, revisar o diff e incluir IDs/status no handoff. A tabela é mantida pelos contribuidores/agentes; o site calcula a apresentação, não detecta automaticamente se uma feature funciona.

## Formato dos dados

Cada checkpoint contém `title`, `completed` (booleano) e `evidence` (ID da prova ou null). Um checkpoint concluído requer prova existente. `documents` começa pelo `document` principal e aponta apenas para rotas registradas no manifesto. A evidência contém `revision`, `document` e `summary`; detalhes extensos ficam no capítulo vinculado.

Exemplo ilustrativo: ao executar o primeiro de quatro checkpoints, mudar o estado para IN PROGRESS e marcar aquele checkpoint como concluído com prova; a interface apresentará `IN PROGRESS (025%)`. Não editar um percentual separado: ele é derivado.

## Validação

```bash
node --check website/app.js
node --check website/progress/progress.js
node website/scripts/verify-progress.cjs
node --test website/tests/progress.test.cjs
python3 website/scripts/verify-agent-docs.py
git diff --check
```

O validador verifica IDs, estados/checkpoints, datas, evidências, existência de documentos e cobertura das rotas Markdown. A CI executa esses gates em alterações de `website/**` e `AGENTS.md`; isso não comprova os gates Rust/GL/manual do editor.

## Snapshot inicial e limites

Inventário inicial: 65 páginas / 18.162 linhas, baseline `bafd02b15fcc006a32f783236e0c87ec74354f51`. Os contratos foram agrupados em 65 processos de migração/verificação mais T01, o próprio painel. Não são 65 funcionalidades ausentes: Lua, MCP, save atômico, export e vários kernels já existem.

U01 inicia com dois de seis checkpoints comprovados: extração ANIMATE e UV/isolamento PAINT. A prova está no [Slint Rescue §16](../09-ui/slint-rescue-plan.md#checkpoint-uv-retomada-de-2026-10-08): 537 testes headless e gates Rust do checkpoint anterior. Equivalência visual final e migrações dos demais domínios continuam abertas. Outros processos começam conservadoramente em TODO até confirmação específica; seu campo baseline registra implementação recuperável, lacuna observada ou conformidade a auditar.

A ordem prioritária segue o [registro de decisões](../00-philosophy/decision-register.md#sequencia-de-execucao); G01/G02 são prioridades de integridade propostas pela inspeção do código. Expansões adiadas (Graph Editor completo, extra modifiers, FBX principal, cloud/marketplace, segundo backend) não são dívida obrigatória da baseline.

## Evidência da entrega do painel — T01

Intenção: integrar processos vinculados aos documentos e atualização contínua. Fontes: registro de decisões, [protocolo de implementação](../16-code-agents/implementation-protocol.md) e inventário por domínio. Gap: o site tinha leitura/navegação, mas não uma tabela versionada com estados.

Changed: módulo `website/progress/`, navegação, dados, protocolo, validadores e CI. Nenhuma compilação Rust é necessária para este slice do site.

Verification — 2026-10-09, Linux, Node.js local e navegador Codex/IAB, arquivos desta entrega sobre a baseline acima:

| Gate executado | Resultado |
|---|---|
| `node --check website/app.js` e `node --check website/progress/progress.js` | pass |
| `node website/scripts/verify-progress.cjs` | pass: 66 processos e 66 documentos cobertos; IDs, três estados, datas e evidências consistentes |
| `node --test website/tests/progress.test.cjs` | pass: 6 testes, zero falhas/ignorados; percentuais, estados, dados inválidos, evidência fora do escopo e filtros |
| `python3 website/scripts/verify-agent-docs.py` | pass: 66 rotas, 15 páginas de agentes, inventário 39/189/20 consistente |
| `git diff --check` | pass |
| Navegador local `http://localhost:8080/#/progress` | pass: carga dos processos, busca sem acentos, filtros combinados, reset, estado vazio, permalink U01, checkpoints e ida/volta ao documento/protocolo |
| Layout e teclado | pass: viewport padrão e 390×844; corpo sem overflow horizontal, tabela com scroll próprio, Tab de busca para Status e Enter em checkpoints |
| Console durante o fluxo | zero erros capturados |

Risks/limites: verificação visual no tema claro e navegador local. Auditoria formal de leitor de tela, tema escuro, Windows e CI remota: not run neste slice; não são prova implícita. O painel registra fatos de execução fornecidos pelos contribuidores, e o validador não comprova que a evidência humana é verdadeira. O site não foi publicado.

Next checkpoint: atualizar os IDs afetados na próxima implementação; U01 continua com MODEL/PAINT/overlays e aceitação final em aberto. T01 pode encerrar somente o escopo do painel acima; isso não fecha o Slint Rescue nem os gates de produto.
