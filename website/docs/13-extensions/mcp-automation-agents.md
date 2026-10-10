# MCP, automação, agentes e workflows

> **Status: aprovado em 2026-10-08 como contrato alvo.** Os endpoints/commands efetivamente disponíveis devem ser verificados contra o adapter de código antes de documentação pública de suporte.

## Papel

MCP é um **adapter oficial da Application API**, não mecanismo de automação de cliques, não executor genérico de scripts e não uma segunda linguagem de mutação do projeto. O server Rust (integração existente) expõe schemas tipados de Tool/Resource e usa sessão do aplicativo ou CLI/headless conforme lifecycle autorizado.

Camadas:

1. MCP transport/server: autenticação/configuração de trust scope, schemas, timeouts e cancelamento.
2. MCP adapter: converte requests em queries/commands estáveis da Application.
3. Application: autoriza capabilities, coordena sessions, transactions e revisões.
4. Domain/Services: executa geometria, documento e export sem dependência de MCP.
5. Response: resultado estruturado com ObjectIds, diagnostics, warnings, revision e action status.

Nenhum cliente pode escrever diretamente em Document, Mesh ou cache.

## Contrato das Tools

Categorias, **não lista afirmada como implementada**:
- Read: scene summary, list objects, inspect properties, selection, asset references, UV/material/rig info permitida;
- Authoring: add primitive, transform, tool command, assign material, create reference, project photo **apenas quando serviço correspondente existir**;
- Validation: model diagnostics, geometry/UV checks, game-ready inspect;
- I/O: open/save/import/export com paths e escopos aprovados;
- Visual: viewport snapshot quando GUI/runtime fornecer, não obrigatório no headless;
- Batch: lista tipada de comandos com preconditions e política atomic.

Cada Tool publica name, version/schema, description, input/output types, mutability, required capabilities, Undo semantics, idempotency guidance, preconditions e errors.

## Security: trust boundaries

- Server não abre listener de rede pública por padrão; transport seguro/local/stdio conforme host e configuração explícita.
- Aprovação humana para operações sensíveis (escrita, filesystem fora de projeto, overwrite/export externo), com allowlist de caminhos e escopos.
- Nunca expor token, credenciais, env secret ou documento de outra sessão por query implícita.
- Filesystem paths canonicalizados/normalizados; conter traversal, symlinks perigosos, oversized inputs e decompression bombs.
- Limites configuráveis de payload, contagem de operações, duração e consumo de recursos.
- Usuário pode revisar logs e revogar permissão; erros não ecoam conteúdo sensível desnecessário.
- Cliente remoto ou plugin não herda permissões do usuário desktop automaticamente.
- Process/command execution arbitrary **não** é capability de MCP V1.

## Revisions, concurrency e stale results

Reads podem usar snapshot imutável. Mutations exigem revision precondition/optimistic check quando houver concorrência. Resultados de worker que não correspondam à revisão esperada são descartados ou explicitamente reavaliados; nunca sobrescrevem estado mais recente silenciosamente.

Um agente não contorna ToolSession ou Undo. Comandos longos retornam progress/status/cancel contract onde implementado; sobrecarga de requests é controlada, evitando bloquear UI.

## Batch, dry run e Undo

- **Dry Run/Preview** quando o comando suportar: valida intent, pré-condições, riscos e escopo, mas não altera Document.
- Batch com atomic=true: all-or-nothing e **uma entrada de Undo**.
- Non-atomic batch precisa opt-in inequívoco, retorno itemizado e diagnóstico do que foi realmente aplicado.
- Resource/Material shared edits informam alcance e confirmação quando houver risco de propagar mudanças.
- Comandos perigosos não recebem "auto repair" ou bake silencioso.
- Todas as mutações geram eventos pós-commit pela Application.

## Public API e Plugins

MCP não deve expor APIs privadas de plugins. Extensões registram Tools MCP **somente** quando manifest é mcp_exposable e o usuário aprova capabilities da Tool e do plugin. Capability de editar Geometry no plugin não vira permissão global para agentes sem concessão independente.

Um plugin quebrado é suspenso sem derrubar o MCP server; Tool schema incompatível é recusada com erro claro. Versionamento da API permite client compatibility checks.

## UX de autorização

No Slint:
- painel de sessões/clients conectados e permissões;
- detalhe de request com operação, targets, efeitos, paths e quantidade;
- opções Approve Once / Scoped Allow / Deny quando política exigir;
- notificações por mudança autoral, com Undo acessível;
- histórico de operações com erros e exports redacted;
- cancelamento e desconexão sem perda de Undo.

Toda decisão é navegável por teclado, com foco explícito, contrastes e leitura de status por screen reader; agentes não ganham capacidade de suprimir confirmação do usuário.

## Conformance

- Unauthenticated/unapproved mutation negada, sem mudança do snapshot.
- Invalid schema, stale ObjectId, revision mismatch e file traversal retornam erros distintos.
- Batch atomic falho reverte por inteiro; Undo/Redo persistem após um commit bem-sucedido.
- Cancelamento de job não aplica resultado obsoleto.
- CLI/headless funcional sem dependência de Slint/renderer.
- Plugin expose off = Tool ausente; capability revogada = request bloqueado.
- Várias solicitações concorrentes não corrompem Document.
- Logs não revelam tokens, paths externos não autorizados ou dados sensíveis.

## Decisões fechadas

MCP é adaptador semântico da Application; capabilities com deny-by-default; revisões explícitas; mutations undoáveis e auditáveis; confirmação quando risco; sem automação de cliques como regra; sem exec arbitrário; plugins opt-in por capability dupla.
