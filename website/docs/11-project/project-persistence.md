# Project, Document, persistência e recuperação

> **Status: aprovado em 2026-10-08 para arquitetura alvo.** Contrato de implementação; não declara que o formato/serviços atuais já cumprem todos os requisitos.

## Responsabilidades

- **Document:** scene objects, fontes geométricas, splines, shapes, materiais, texturas, references, paint, UV, rigs, clips, ligações e metadata autoral.
- **DocumentSession:** Document aberto, identidade, caminho, histórico e revisão/dirty derivados. Não contém janela, seleção ativa, zoom, ferramentas ou recentes.
- **EditorSession:** workspace, seleção, ViewportSession, ToolSessionManager e preferências de edição transitórias.
- **ProjectService:** operações orquestradas Open, Save, Save As, New, Close, Recover; coordena command boundaries, snapshots, validação e erros.
- **Infrastructure:** atomic writer, codec, locks, paths, import/export, recovery storage e observabilidade; não decide geometria.
- **Application:** único coordenador de mutations, Commands, Transactions e eventos pós-commit.

Esta separação complementa [separação de estado](../02-application/state-separation.md) e [Commands e services](../02-application/commands-events-services.md).

## Documento não é arquivo

Document é modelo de domínio, não estrutura ligada diretamente a Serde/UI/ZIP. Um **DocumentSnapshot** imutável passa por serializer/loader de infraestrutura. Formato de armazenamento não determina os tipos internos de Geometry. Caches GPU, caches triangulation, thumbnails regeneráveis e estados de ferramentas não são persistência autoral.

O arquivo nativo usa extensão **.petunia**, com container e esquema versionados. Preservar o decoder atual quando compatível; não reescrever o projeto existente como novo container até haver migração e round-trip validados.

### Schema mínimo e identidade

- format_version: compatibilidade do arquivo; app_version: diagnóstico de origem, não chave exclusiva de leitura.
- project_id estável, object IDs persistentes e IDs de recursos estáveis após save/load.
- Tabelas explícitas de objetos e dependências; refs inválidas geram erro ou reparo auditável.
- Manifest identifica recursos internos/externos, encoding e versões/feature flags indispensáveis.
- Posições/transforms de authoring seguem convenção RH, Y-up e metros; conversões são explícitas em import/export.
- Handles topológicos runtime são reconstruídos; nunca prometer persistência binária de slotmap/index.
- Preferências de UI global, câmera transitória e seleção não entram no Document apenas para facilitar serializer.
- Metadados opcionais não devem conter caminhos secretos, tokens ou dados privados desnecessários.

## Recursos e dependências

Cada referência de recurso usa ID tipado e resolução determinística. Content hashes ajudam detectar duplicação/integridade, **mas não substituem identidade autoral**. Recursos embutidos são preferidos para portabilidade; referência externa deve ser explícita, resolvível e diagnosticada quando ausente.

Recursos compartilhados não são silenciosamente copiados ao salvar. Alterar material, imagem ou DecalSet compartilhados exige sinalização de escopo na UI; Make Unique é operação explícita. Ver [Assets e Library](./assets-resource-management.md).

## Load: pipeline de confiança

1. Abrir em worker com limites de tamanho, profundidade, contagem de elementos e recursos.
2. Validar versão, container, caminhos, hashes e payloads antes de construir domínio.
3. Deserializar formato externo **para DTO de importação**, não para Document mutável publicado.
4. Aplicar migrations determinísticas em staging; preserve IDs quando possível; registrar mudanças.
5. Validar topologia, attachments, texturas e referências; distinguir **Strict** de **Legacy Repair**.
6. Construir DocumentSnapshot consistente e publicá-lo de uma só vez na Application.
7. Só depois inicializar sessão, UI e caches derivados. Falha não substitui documento corrente.
8. Resultados/requests associados a versão obsoleta não podem sobrescrever sessão mais recente.

Não criar um segundo autor da verdade por "import tolerant" dentro de Geometry. Migrar e reparar são interfaces controladas com logs, limites e opções explícitas.

## Save: integridade atômica

1. Validar invariantes do snapshot correspondente à revisão escolhida.
2. Serializar para staging temporário no mesmo filesystem/diretório de destino sempre que possível.
3. Finalizar escrita e sincronização conforme suporte do SO; usar replace/rename atômico quando disponível.
4. Não destruir o arquivo anterior em falha; tratar permission denied, disco cheio, path inválido e interrupção.
5. Atualizar saved revision **somente se** a operação confirmar êxito para aquela revisão.
6. Salvar uma revisão anterior durante edições concorrentes **não limpa** o dirty mais recente.
7. Save As solicita escolha/confirmacão de destino; nunca altera o caminho da sessão antes de êxito.

Um writer por destino, coordenado pelo host. Não bloquear interface durante compressão; capturar snapshot de forma segura, preferindo dados grandes imutáveis/reutilizáveis quando claro.

## Undo/Redo, dirty e transações

- Undo/Redo pertence ao DocumentSession/Command Dispatcher, não ao serializer.
- **Uma operação do usuário → uma transação coerente**: transform drag, paint stroke, boolean, edição procedural ou batch atômico.
- Cancel/Escape descarta previews transitórios; não marca documento dirty.
- Salvar não limpa Undo; dirty é comparação de revisão/saved checkpoint, não boolean mutável competindo com história.
- Migrations destrutivas e Auto Repair pedem confirmação ou produzem cópia de segurança conforme risco.
- Undo não deve incluir mero hover, foco, câmera ou navegação, exceto ações explicitamente autorais.

## Autosave e Crash Recovery

A política inicial mantém a baseline já especificada de **2 minutos para documento dirty e cinco gerações**, configuráveis/desativáveis. Recovery é snapshot separado do arquivo do usuário; nunca substitui o original automaticamente.

- Coalescer ciclos concorrentes do mesmo documento, identificar por project_id e revision.
- Recover em inicialização exibe data/revisão, origem e ações **Recover / Inspect / Discard**.
- Restore abre sessão isolada ou nova cópia recuperada; sobrescrita do arquivo principal exige Save explícito.
- Recovery temporário é removido apenas segundo política de fechamento/salvamento limpo, nunca em falha de disco.
- Não persistir tokens, keymaps globais, estado de tela ou paths sem consentimento no pacote autoral.
- Eventuais locks de sessão são geridos pelo DesktopHost/Infrastructure, não por construtores puros.

## Conflitos e compatibilidade

Versionamento usa formatos major/minor ou migrações explícitas equivalentes; **versão desconhecida que comprometa integridade falha com orientação**, sem aceitar silenciosamente perda de dados. Campos desconhecidos devem ser preservados quando seguro e houver suporte no container; caso contrário, abortar import ou pedir conversão.

Salvar projeto de versão superior em versão antiga nunca é automático. Pacote portátil copia recursos através de operação explícita **Package Project**, sem caminhos absolutos embutidos e com auditoria de missing assets.

## Gates de conformidade

- Open/save/load de arquivo legado e formato novo com IDs autorais persistentes.
- Corrupt/truncated/oversized/zip traversal/symlinks recusados sem pânico.
- Save com escrita interrompida preserva arquivo anterior; dirty permanece fiel à revisão.
- Undo após save, save durante edição, crashes e recovery por múltiplas gerações.
- Materiais, Paint, UV, splines, animation, decals, references e geometry sources round-trip.
- Projeto portátil abre em Linux e Windows com paths normalizados e ausência de resource leaks.

## Decisões fechadas

Document autoral único; EditorSession separado; serialização e recovery pertencem à Infrastructure; staged load; atomic save; IDs estáveis e migrações explícitas; cache nunca é autoridade; recovery nunca sobrescreve o projeto silenciosamente.
