# Módulos internos, Lua Plugins e Public Extension API

> **Status: aprovado em 2026-10-08 para a arquitetura alvo.** Preserva princípios do sistema de plugins existente e adapta o contrato de UI à estratégia Slint; não declara paridade de todos os adapters implementada.

## Três categorias, três contratos

| Categoria | Exemplo | Responsabilidade |
|---|---|---|
| Módulos nativos internos | Model, Paint, UV, Assets | Crates/features compiladas; não são plugins runtime |
| Provedores oficiais | Boolean, Auto UV, codecs | Adapters internos atrás de traits tipadas |
| Community Plugins | Lua 5.4 + mlua | API estável de commands, queries, events, panels e capabilities |

Não criar um ModuleRegistry genérico com trait para *tudo*; o plano aprovado em Application já elimina seu papel como fundamento de Model/Paint/UV. Providers internos não são argumento automático para permitir native DLL de terceiros na V1.

## Boundary canônica

Lua Plugin → Host API versionada → Application Query/Command/Transaction → Document/Geometry/Services.

- Plugin não lê Mesh interna, index numérico cru, GPU, raw pointer, Slint internals, renderer state ou AppState mutável.
- UI de plugins emite UiIntents e binding com DTOs; nunca muta documento em callback de render.
- Public API usa IDs tipados/referências com validação de lifecycle, erros estruturados e opções explícitas.
- Events são **fatos depois do commit**, não instruções disfarçadas.
- Todo plugin compartilha undo/redo/cancel com ferramentas nativas.
- Stable API evita expor classes internas e garante compatibilidade por semver/capabilities.

## Runtime Lua

- **Lua 5.4 via mlua** como extensão pública inicial.
- Um Lua State isolado por plugin; sem acesso irrestrito a stdlib IO, OS, process spawn ou network.
- Sandbox **defense in depth**: capabilities host + validação de paths + execution budgets + limites de memória + cancelamento/timeouts.
- Lua embutido não é fronteira de segurança perfeita contra código arbitrário hostil. Não prometer isolamento de processo: plugins de origem não confiável exigem confirmação e políticas de execução mais restritivas.
- Erro em plugin não deve corromper documento; host aborta transaction e desabilita a extensão com diagnóstico quando necessário.
- Unload deve liberar commands, event subscriptions, panels, actions e caches namespaced.
- Sem hot reload "mágico": recarregar ocorre por lifecycle explícito, com restauração apenas de estado declarativo seguro.

## Manifest e permissões

Campos mínimos: plugin_id, version, api_version/capability_version, name, author/source, entrypoint, required_capabilities, optional_capabilities, dependencies/supported app range quando necessário. O host resolve compatibilidade antes de ativar.

Capabilities inicialmente conhecidas:
- register_commands, register_ui_panel, register_viewport_overlay;
- read_document, read_selection;
- edit_geometry, edit_uv, edit_materials;
- import_files, export_files;
- filesystem (scoped);
- mcp_exposable.

**Negar por padrão** operações sensíveis. Concessão de read não implica write; filesystem não implica network; UI panel não implica acesso ao Document. Permissões devem ser revogáveis com efeito observável.

## API e comandos públicos

- Commands semânticos (create, transform, modify, import, export) recebem DTOs estáveis e retornam IDs/resultados tipados.
- Query retorna snapshot consistente com revision; nunca referências mutáveis de longa duração.
- Erros incluem InvalidId, StaleReference, PermissionDenied, ValidationFailed, VersionMismatch, Busy, Cancelled e Unsupported.
- Batch atômico executa com commit único ou aborta inteiro; operações parciais exigem contrato não atômico explícito e disclosure.
- Commands de plugin devem indicar Undo label, scope autoral, preconditions e progress/cancel.
- Mensageria/event dispatch protege contra eventos recursivos ilimitados e mutações durante draw.

## Plugin Panels com Slint

O host Slint compõe **descritores de componentes públicos**, não markup Slint fornecida como código compilável runtime e nem referências diretas à árvore do shell.

Regiões V1: left, right, bottom, sujeitas ao layout do workspace. Plugins **não substituem** viewport central e não abrem janelas arbitrárias como padrão. Descriptor:
- panel_id + plugin_id;
- label/icon/help;
- preferred_region e allowed_regions;
- min/preferred size, singleton e visibility default;
- context requirements;
- controls e binding a DTOs/action IDs versionados.

Panels:
- herdam tokens/tema, escalabilidade 100–200%, High Contrast e Reduced Motion;
- foco/Tab/F6, keyboard activation e semântica de acessibilidade fornecidos pelo host;
- não podem fornecer RGB locais arbitrários, interceptar atalhos reservados sem mapping ou esconder focus ring;
- guardam estado de visualização namespaced, não Document paralelo;
- atualizam-se por eventos/invalidation, nunca scan de cena por frame.

**Regra de implementação:** preservar código de host e dispatch existente sempre que saudável. O Slint bridge é adapter, não API pública congelada no seu markup atual.

## Overlay de viewport

Plugin pode declarar overlays passivos ou ações semânticas registradas. O host renderiza conteúdo vetorial/handles via interfaces limitadas; plugin não recebe context GL, shader injection ou GPU resources brutos na API V1. Overlays precisam de hit-test e regras de prioridade para não quebrar selection, tool sessions, performance ou acessibilidade.

## Estado, dados e compatibilidade

- Configuração do plugin pertence a namespace/versão próprios e preferências apropriadas.
- Dados autorais de plugin dentro do Document exigem schema versionado, validação e forma de carregar sem execução do plugin quando seguro.
- Ausência de plugin deve produzir **missing plugin/resource diagnostic** sem apagar payload reconhecido.
- Atualização de plugin não migra arquivo de usuário silenciosamente.
- Crash/unload não mantém callbacks/handles stale e não contamina outras sessões.

## Testes de conformidade

1. Plugin sem capability falha com PermissionDenied sem alterar Document.
2. Unload/reload limpa panel, command, handler e foco sem leaks.
3. Plugin gera undo/redo idêntico ao nativo e aborta batch ao falhar.
4. Access a IDs stale dá erro, não outro objeto.
5. Lua worker/callback lento termina por limite/cancel quando aplicável; UI não congela.
6. Panels funcionam com teclado, alto contraste e escala; desabilitar UI não quebra registro.
7. Version incompatibility e erro de schema produzem diagnóstico.
8. MCP exposable é opt-in adicional e não aumenta capabilities automaticamente.

### Matriz de implementação e conformidade (E01)

| Requisito / Gate | Status | Evidência / Teste | Escopo & Garantia |
|---|---|---|---|
| Gate 1: PermissionDenied sem tocar Document | `[x] DONE` | `test_gate1_missing_capability_denied_without_altering_document` | Comando não concedido rejeitado; Document e Undo inalterados |
| Gate 2: Unload/reload limpo sem leaks | `[x] DONE` | `test_gate2_unload_and_reload_cleans_handlers_without_leaks` | Disable/uninstall dropa instâncias, handlers e assinaturas; re-enable cria state novo |
| Gate 3: Atomic Batch e Rollback em falha | `[x] DONE` | `test_gate3_atomic_batch_rollback_on_failure_and_single_undo_on_success` | Batch atômico via `AppState::transaction`: 1 undo rotulado no sucesso; rollback total em falha |
| Gate 4: Stale reference / ID inexistente | `[x] DONE` | `test_gate4_stale_reference_returns_error_and_never_targets_active_object` | Target inexistente falha com `PluginApiError::StaleReference`, nunca atinge objeto ativo |
| Gate 5: Instruction budget e timeouts | `[x] DONE` | `test_gate5_infinite_loop_aborts_cleanly_via_instruction_budget` | Loops infinitos interrompidos por teto de instruções sem travar aplicação |
| Gate 6: Sandbox defense in depth | `[x] DONE` | `test_gate6_sandbox_defense_in_depth_blocks_os_io_and_bytecode` | `io`, `os`, `package`, bytecode binário e dump suprimidos do ambiente |
| Gate 7: Version incompatibility & manifest | `[x] DONE` | `test_gate7_manifest_validation_and_version_incompatibility` | Mismatch de API version, semver malformado, path traversal rejeitados com diagnóstico |
| Gate 8: MCP exposable restrito | `[x] DONE` | `test_gate8_mcp_exposable_grants_no_underlying_capabilities` | `McpExposable` é opt-in sem concessão automática de capabilities de escrita |
| Gate 9: Dados autorais namespaced | `[x] DONE` | `test_gate9_namespaced_plugin_data_persistence_and_missing_diagnostic` | `set_plugin_data`, schema versionado, persistência e detecção de missing plugins sem descarte |


## Não objetivos V1

Marketplace, plugins binários arbitrários, API de shaders GPU, macro runtime de Slint/markup livre, rede irrestrita e ECS universal. São ampliações que exigem requisitos e segurança próprios.

## Decisões fechadas

Lua como API de terceiros; nativos internos separados; UI e MCP são adapters de comandos/queries; capabilities revogáveis; um state por plugin; isolamento e budgets; schema versionado; zero mutações durante render; UX e acessibilidade herdadas do host.
