# Especificações P3D — Readiness e Gauntlet Waves

<aside>
📘

**Consolidação documental — 2026-09-15.** A antiga **Petunia3D — Implementation Bible** foi absorvida pelo **Petunia3D — Livro Vivo**. Esta página agora é o **hub de Especificações P3D, Readiness e Gauntlet Waves** do único caderno canônico. Referências internas antigas ao nome “Implementation Bible” devem ser interpretadas como referências a este capítulo.

</aside>

# Papel deste capítulo

O **Livro Vivo** é a única raiz documental canônica do Petunia3D. Este capítulo preserva o catálogo funcional detalhado, os epics, contratos de readiness e o protocolo de execução que antes viviam em um segundo caderno.

A separação de responsabilidades após o merge é:

- **Livro Vivo:** visão, produto, escopo, arquitetura, UI/UX, stack, políticas técnicas e decisões normativas.
- **Este capítulo:** especificações P3D, epics, readiness, auditoria de lacunas e execução por Gauntlet Waves.
- **Repositório, testes e comportamento executável:** evidência do estado real da implementação.
- **Planilha/backlog operacional:** índice resumido de status e prioridade; não substitui a especificação.

# Regra de autoridade após o merge

Quando houver conflito documental, prevalece a decisão **mais recente e explicitamente normativa**, respeitando a hierarquia especializada do Livro Vivo. Em particular:

1. ADRs e decisões explícitas posteriores prevalecem sobre baselines históricas.
2. O capítulo 34 é a autoridade especializada de representação em código, ownership, modularidade e Rust safety.
3. O capítulo 36 é a autoridade especializada da UI Baseline vigente.
4. O capítulo 12 governa o escopo funcional; o roadmap pós-GA desta página complementa e atualiza candidatos antigos quando houver promoção explícita.
5. Os capítulos 19 e 20 governam concorrência/performance e testes/conformance.
6. As páginas P3D detalham o comportamento da feature sem poder contradizer as invariantes normativas acima.

Contradições normativas ainda não resolvidas devem ser registradas como contradição; não podem ser escondidas por uma escolha silenciosa do agente.

# Reconciliação obrigatória entre especificação e código

Antes de mudanças significativas, produzir uma **Implementation-vs-Spec Gap Matrix**. Cada requisito relevante deve ser classificado como:

`COMPLIANT`, `PARTIALLY_COMPLIANT`, `FUNCTIONAL_BUT_DIFFERENT`, `RUDIMENTARY`, `STUB`, `BROKEN`, `DUPLICATED`, `MISSING`, `OBSOLETE`.

Regras:

- preservar `COMPLIANT`;
- corrigir somente o delta comprovado em `PARTIALLY_COMPLIANT`;
- mudar `FUNCTIONAL_BUT_DIFFERENT` apenas quando a divergência violar contrato normativo ou houver ganho demonstrável;
- consolidar `DUPLICATED` em um caminho canônico;
- remover `OBSOLETE` apenas após provar ausência de consumidores e possuir cobertura/rollback adequados;
- evitar big-bang rewrites; reescrita total exige evidência de que a arquitetura atual impede a correção incremental.

# Readiness e Definition of Done

Uma P3D P0/P1 entra em implementação definitiva somente após `SPEC READY`. Conforme aplicável, a especificação deve conter objetivo, escopo/non-goals, auditoria do código, dependências, modelo de estado/dados, boundaries Core/Application/UI/Renderer, Commands/TextIds/IconIds/ThemeTokens/keymap, Undo/persistência, estados inválidos/erros, performance, segurança, testes, documentação, critérios de aceitação e riscos de migração/rollback.

`IMPLEMENTATION READY` acrescenta: baseline tests executados, bloqueadores conhecidos, arquivos/módulos reais mapeados, architecture checks relevantes definidos e nenhuma decisão estrutural crítica deixada implícita.

Estados recomendados: `IDEA/ROADMAP → SPEC DRAFT → SPEC READY → AUDITED → IMPLEMENTATION READY → IN PROGRESS → VALIDATION → DONE`, além de `ABSORBED` e `RULE`. `DONE` exige evidência, não apenas código presente.

# Escopo do catálogo P3D consolidado

O catálogo preservado cobre **P3D-001 a P3D-174**. A antiga descrição que terminava em P3D-155 ficou obsoleta após a expansão aprovada do roadmap pós-GA.

- P3D-016 permanece como registro histórico; sua UX pública foi absorvida por P3D-015.
- P3D-131–143 formalizam feedback modal, Paint avançado, rig/animação, materiais avançados, AI e bridge futura.
- P3D-144–155 iniciam o toolkit pós-GA game-ready.
- P3D-156–168 expandem o pós-GA com Decals, Modifier Stack, Surface Attachment, Parametric Asset Properties, Bake/Flatten, Spline Core, Morph Targets, Low-Poly Hair, Surface Recipes, Surface Paint Toolbox, Parts Hierarchy, Asset States e Procedural Path Generators.
- P3D-169–174 (2026-09-30) especificam o **Animate procedural-first**: Rig Roles/IK, Motion Generators, Ghosts/Trails, Reference Image Sequence, Secondary Motion e Layered Animation ([cap. 45](foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)).

# Fontes redundantes preservadas após o merge

As páginas 00–07 abaixo são mantidas **integralmente como linhagem documental e referência histórica**, mas suas regras redundantes foram absorvidas pelas autoridades canônicas do Livro Vivo. Elas não formam um segundo nível de governança concorrente.

- 00 + 04 → consolidados principalmente nos capítulos 09 e 34 do Livro Vivo.
- 01 + 05 + 06 → consolidados principalmente no capítulo 13 e neste capítulo 39.
- 02 → consolidado no capítulo 20 e no Master Prompt de Waves.
- 03 → consolidado na UI Baseline do capítulo 36.
- 07 → preservado como histórico de evolução; o roadmap pós-GA abaixo é a autoridade atual para promoções pós-GA.

[00 — Constituição do Projeto e Diretiva para Agentes](constitution/00-constituicao-do-projeto-e-diretiva-para-agent.md)

[01 — Protocolo de Especificação P3D e Prompt para LLM](constitution/01-protocolo-de-especificacao-p3d-e-prompt-para.md)

[02 — Gauntlet Loop, Quality Gates e Definition of Done](constitution/02-gauntlet-loop-quality-gates-e-definition-of.md)

[03 — Invariantes de UI/UX, Design System e Acessibilidade](constitution/03-invariantes-de-ui-ux-design-system-e-acessib.md)

[04 — Invariantes de Arquitetura, Modularidade e Core Agnóstico à UI](constitution/04-invariantes-de-arquitetura-modularidade-e-co.md)

[05 — Política de Documentação, Screenshots e Changelog](constitution/05-politica-de-documentacao-screenshots-e-chang.md)

[06 — Governança do Backlog, Status e Prioridades](constitution/06-governanca-do-backlog-status-e-prioridades.md)

[07 — Roadmap, Adendos e Critérios para Novos Módulos](constitution/07-roadmap-adendos-e-criterios-para-novos-modul.md)

# Catálogo funcional por epics

As páginas abaixo continuam sendo a especificação funcional detalhada das features. Elas devem ser lidas junto das invariantes normativas do Livro Vivo e do protocolo de readiness acima.

[A — Project & Files](sections/section-a-project-files.md)

[B — Viewport & Navigation](sections/section-b-viewport-navigation.md)

[C — Reference Workflow](sections/section-c-reference-workflow.md)

[D — Selection, Transform & Modeling](sections/section-d-selection-transform-modeling.md)

[E — Assets, Scene & Inspector](sections/section-e-assets-scene-inspector.md)

[F — Surface, Materials, Paint & UV](sections/section-f-surface-materials-paint-uv.md)

[G — Animation & Rigging](sections/section-g-animation-rigging.md)

[H — Import & Export](sections/section-h-import-export.md)

[I — UI Shell & Professional UX](sections/section-i-ui-shell-professional-ux.md)

[J — Customization, i18n & Keymaps](sections/section-j-customization-i18n-keymaps.md)

[K — Architecture & Modularity](sections/section-k-architecture-modularity.md)

[L — Plugins, Automation & AI](sections/section-l-plugins-automation-ai.md)

[M — Help, Documentation & QA](sections/section-m-help-documentation-qa.md)

[N — Performance & Project Philosophy](sections/section-n-performance-project-philosophy.md)

[O — Future Product Integration](sections/section-o-future-product-integration.md)

# Contratos operacionais e transversais ativos

Estas páginas não são um segundo caderno: são subespecificações especializadas deste capítulo único.

[08 — Roadmap Pós-GA Aprovado](constitution/08-roadmap-pos-ga-aprovado.md)

[09 — Shared 3D Foundation, Map Editor e Game Engine](constitution/09-shared-3d-foundation-map-editor-e-game-engin.md)

[10 — Convenções Espaciais, Unidades e Coordenadas](constitution/10-convencoes-espaciais-unidades-e-coordenadas.md)

[11 — Contrato de Mesh, Selection, Tools e Undo](constitution/11-contrato-de-mesh-selection-tools-e-undo.md)

[12 — Contrato de Materiais, Texturas, UV e Color Pipeline](constitution/12-contrato-de-materiais-texturas-uv-e-color-p.md)

[13 — Jobs, Concorrência, Diagnósticos, Segurança e Trust Boundaries](constitution/13-jobs-concorrencia-diagnosticos-seguranca-e.md)

[14 — Release, Compatibilidade, Distribuição e Critérios de GA](constitution/14-release-compatibilidade-distribuicao-e-crit.md)

[15 — Auditoria Final de Lacunas e Readiness Matrix](constitution/15-auditoria-final-de-lacunas-e-readiness-matrix.md)

[16 — Master Prompt de Implementação por Gauntlet Waves](constitution/16-master-prompt-de-implementacao-por-gauntlet-w.md)

# Regra final

A documentação unificada deve reduzir duplicação sem apagar rationale. Conteúdo absorvido permanece acessível como histórico; conteúdo normativo possui uma única autoridade explícita. Novas decisões devem ser registradas no **Livro Vivo**, e novas P3Ds/execução devem entrar neste hub sem recriar uma segunda “Bible” paralela.