# Remediação G0 — transações, revisões, histórico e cache

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** fechamento conjunto de `TX-01`, `TX-02`, `TX-03`, `RV-01` e
> `CA-01` do dossiê de ferramentas
>
> **Validação:** concluída após a implementação integral do conjunto G0

## 1. Autoridade e critérios

Este pacote aplica incrementalmente os contratos de transação e invalidação já
definidos pelo Livro Vivo:

- um gesto contínuo corresponde a uma transação e cancelar não cria histórico;
- operação inválida restaura o documento e não entra no histórico;
- o documento permanece `single-writer`;
- mudanças invalidam somente os domínios e caches dependentes;
- caches derivados não são autoridade e precisam representar toda a origem;
- ferramentas interativas preservam `preview → commit/cancel → Undo/Redo`.

Referências normativas principais:

- `docs/bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md`;
- `docs/bible/foundations/09-arquitetura-governanca-tecnica.md`;
- `docs/bible/foundations/16-documento-formato-undo-recovery.md`;
- `docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`;
- `docs/bible/foundations/34-arquitetura-modular-rust-safety.md`.

## 2. Implementation-vs-Spec Gap Matrix do G0

| ID | Estado anterior | Delta implementado | Estado após o pacote |
| :--- | :---: | :--- | :---: |
| TX-01 | `BROKEN` | Checkpoint passa a receber o snapshot anterior; gestos de Paint, decal e Profile Volume têm boundary explícito | `COMPLIANT` nos fluxos auditados |
| TX-02 | `DUPLICATED` | `CommandDispatcher` vira o único owner de checkpoint, sincronização e publicação dos comandos; comandos não repetem o lifecycle | `COMPLIANT` |
| TX-03 | `BROKEN` | Orçamento do histórico usa estimativa profunda das capacidades retidas pelo `Project` | `COMPLIANT` para snapshots atuais |
| RV-01 | `BROKEN` | Invalidação passa a declarar domínios de mudança e revisões independentes | `COMPLIANT` nos caminhos migrados |
| CA-01 | `BROKEN` | Chave do cache de modifiers representa todo o conteúdo autoral da malha e a pilha de modifiers | `COMPLIANT` funcionalmente |

O dossiê original permanece um snapshot histórico. Esta página registra a
remediação posterior sem reescrever os achados que motivaram o trabalho.

## 3. Arquitetura entregue

### 3.1 Ownership transacional único

`CommandDispatcher::dispatch` agora executa o boundary completo de um comando
destrutivo:

1. valida precondições;
2. rejeita concorrência com modal, preview de malha ou stroke ativo;
3. captura projeto e seleção anteriores;
4. congela primitivas paramétricas dentro da mesma transação;
5. executa o comando;
6. restaura tudo em caso de erro;
7. grava exatamente um checkpoint anterior em sucesso;
8. sincroniza seleção e publica somente os domínios declarados.

As implementações de comando deixaram de chamar `checkpoint`,
`sync_selection` e `emit_mesh_changed` por conta própria. A operação contextual
de excluir/dissolver também foi consolidada em um único comando, impedindo que a
tentativa de dissolução e o fallback de exclusão criem duas entradas.

Comandos que não alteram o documento, como salvar, criar um novo projeto por
fluxo dedicado e persistir o asset ativo, não recebem uma transação destrutiva
artificial. Alterações de metadados declaram `ProjectChanges::NONE` quando não
afetam caches visuais.

### 3.2 Gestos e previews atômicos

Os seguintes fluxos agora capturam o estado anterior no início e finalizam uma
única transação:

- stroke de Paint e pintura por cor de vértice;
- fill no canvas 2D;
- transformação interativa de decal;
- criação shape-first por extrude, revolve ou sweep de perfil;
- previews modais e previews de malha compartilhados pelo Core.

`Escape` restaura o snapshot anterior, preserva os contadores monotônicos de
revisão e não adiciona histórico. Confirmar registra o snapshot anterior, nunca
o estado já mutado. Atualizações de preview continuam visíveis durante o gesto,
mas o commit não publica uma segunda invalidação redundante.

### 3.3 Revisões de domínio

`ProjectChanges` formaliza os domínios:

- topologia;
- posições;
- normais;
- seleção;
- UVs;
- cores de vértice;
- materiais;
- texturas;
- transforms.

O projeto mantém um contador monotônico para cada domínio. `undo`, `redo` e
cancelamentos rebaseiam o snapshot restaurado sobre o relógio atual, evitando
que um estado antigo reutilize uma revisão observada anteriormente. O renderer
inclui normais, UVs e cores em seu fingerprint, inclusive no fallback legado de
projetos sem revisões.

Os contadores já existentes mantêm a ordem serializada anterior; os novos foram
acrescentados ao fim da representação para não reinterpretar projetos postcard
legados. Após um restore, somente o domínio efetivamente publicado avança; os
demais preservam o relógio corrente sem invalidação artificial.

Os comandos de seleção, UV, normais, pintura, material, textura e transform
publicam mudanças específicas. `GEOMETRY` continua como agrupamento de
compatibilidade para operadores topológicos que alteram múltiplos atributos.

### 3.4 Histórico com orçamento realista

`Project::estimated_bytes` deixou de medir apenas o tamanho superficial da
estrutura. A estimativa passa a incluir capacidades retidas por:

- assets, malhas e vetores internos de faces;
- seleções, seams, pins e slots de material;
- texturas, canvases e pilhas de pintura;
- materiais e todos os canais de textura;
- skeletons, bones, skin weights e animações;
- coleções, anotações, medições e strings;
- cache de avaliação retido no snapshot.

O objetivo é fazer o limite de memória do Undo refletir o custo dominante dos
snapshots atuais. O passo futuro continua sendo compartilhar blobs imutáveis e
usar diffs por região/tile, não sofisticar indefinidamente a contabilidade de
clones integrais.

### 3.5 Cache correto de modifiers

A chave de avaliação de modifiers agora mistura a pilha de modifiers com uma
assinatura completa da malha fonte: posições, cores, seleção, topologia que
define as normais derivadas, UVs, materiais por face, arestas selecionadas,
seams e pins. Duas
malhas com as mesmas contagens, mas conteúdo diferente, não reutilizam mais o
mesmo resultado derivado.

Essa solução fecha o bug de correção de `CA-01`. Ela ainda é `O(n)` para gerar a
assinatura e deve ser tratada como ponte segura até que a mutação da malha tenha
ownership suficiente para manter uma revisão autoral por asset sem risco de
esquecimento.

### 3.6 Composição de Paint como operação de domínio

A recomposição da pilha de Paint foi movida para `Project`, permitindo que os
comandos de decal sejam completos também em execução headless. A UI deixou de
terminar silenciosamente uma operação de domínio depois que o dispatcher já
havia fechado a transação.

## 4. Provas adicionadas

Foram adicionados testes para:

- uma seleção alterar apenas a revisão de seleção;
- UV Stitch/Relax criar uma entrada por comando e alterar apenas UV;
- Flip Normals alterar apenas a revisão de normais;
- Dissolve e exclusão contextual produzirem um único roundtrip cada;
- o orçamento do histórico observar dados alocados fora da struct superficial;
- o cache de modifiers distinguir fontes com contagens idênticas;
- alterações diretas de UV invalidarem o fingerprint legado;
- fill de cor de vértice avançar somente a revisão de cor e ser desfazível;
- mutações da pilha de Paint gerarem um único roundtrip;
- decal interativo confirmar uma vez e cancelar sem histórico;
- fill 2D ser uma única transação;
- Profile Volume cancelar exatamente para o estado original e confirmar com um
  único Undo/Redo.

## 5. Limites conscientes e próximo gate

Este pacote não declara resolvidos achados fora do G0:

- `PA-01` e `PA-02`: Paint ainda possui cópias integrais e composição por CPU;
- `UI-01`: o catálogo semântico e o keymap ainda precisam convergir;
- `UI-02`: promessas visuais precisam corresponder ao shader real;
- `UV-01`: a superfície UV ainda precisa obedecer ao congelamento canônico;
- `AX-01`: navegação regional e semântica acessível continuam incompletas;
- `HA-01`: Spline Core e Surface Attachment ainda são pré-condições de Hair.

Também permanecem mutações diretas da UI fora dos fluxos críticos auditados. O
próximo gate arquitetural deve migrá-las gradualmente para Commands, sem uma
reescrita ampla e sem expandir a superfície egui legada.

## 6. Gates de encerramento

Os testes são executados somente depois que código, testes e documentação deste
conjunto estiverem completos, conforme solicitado para o roadmap.

| Gate | Resultado |
| :--- | :---: |
| `cargo fmt --all -- --check` | PASSOU |
| `cargo check --workspace` | PASSOU |
| testes de Project/Core/Paint/UV/Slint | PASSARAM |
| `cargo clippy ... --all-targets -- -D warnings` | PASSOU |
| `cargo run -p xtask -- docs-check` | PASSOU |
| `cargo run -p xtask -- bible-check` | PASSOU |
| `cargo run -p xtask -- ui-guard --strict` | PASSOU |

O `docs-check` preservou os 813 arquivos do site público congelado e reportou,
como informação não bloqueante, símbolos públicos ainda ausentes do mapa de
componentes. O `ui-guard` também manteve confinados os tipos egui e listou os
cheiros já existentes na crate legada; nenhum deles foi introduzido ou ampliado
por este pacote Slint/Core.
