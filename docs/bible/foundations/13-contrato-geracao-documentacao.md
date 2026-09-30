# 13 — Contrato para Geração de Documentação pelo Framework

<aside>
📚

Este capítulo define **como o Project/Prumo Framework deve interpretar o Livro Vivo do Petunia3D** ao gerar documentação, especificações, planos de implementação, tarefas e validações. O objetivo é evitar que hipóteses antigas, itens experimentais ou termos técnicos internos sejam promovidos acidentalmente a requisitos do produto.

</aside>

# Fonte canônica

O conjunto de páginas sob **Petunia3D — Livro Vivo** é a fonte canônica de requisitos e decisões do projeto enquanto não houver uma especificação versionada posterior explicitamente declarada como substituta.

Ao gerar documentação, o framework deve preservar:

- intenção de produto;
- decisões normativas;
- rationale relevante;
- separação entre Core, Official Extension, Community Plugin, V1.x e Experimental;
- nomenclatura de usuário versus nomenclatura técnica interna;
- decisões explicitamente marcadas como fora de escopo.

# Hierarquia de autoridade documental

Em caso de conflito entre páginas, interpretar nesta ordem:

1. **32 — ADR: Migração da Baseline Odin para Rust** para determinar a stack vigente quando houver conflito com documentação anterior.
2. **34 — Arquitetura Modular Explícita, Rust Safety e Representação em Código** para representação de entidades/recursos/topologia, DOD/ECS, Tool→Command→Algorithm→Data, ownership, traits, modularidade, safety e regras de implementação Rust.
3. **27–31 — stack Rust, arquitetura de crates, geometry/rendering, I/O/plugins/MCP e qualidade/vertical slice** para detalhes atuais de implementação; **35** é a autoridade especializada do ecossistema egui/Petunia Components; **36 é a autoridade final para UI Baseline V1, layout, tokens finais, input/focus/accessibility, Theme Extensions e Plugin Panels**; **33** reúne referências técnicas e não supera os contratos normativos.
4. **09 — Arquitetura, Princípios de Decisão e Governança Técnica** para regras arquiteturais gerais que não foram substituídas pelo ADR/capítulo 34.
5. **21 — Baseline Funcional e UI V1 Congeladas, Stack Rust Final** para estado consolidado de produto/stack e fase corrente de implementação/conformance.
6. **12 — Baseline Funcional, Roadmap e Contrato de Escopo** para status de inclusão/versionamento de funcionalidades.
7. **14–20 — contratos técnicos especializados** para comportamento e invariantes; quando mencionarem Odin/Go ou uma implementação antiga, 27–32 prevalecem.
8. **22–26 — fundamentos/referências da interface**; quando algum valor anteriormente estiver marcado como `UI-OPEN`, **36 prevalece** para a baseline final V1. Ajustes posteriores são tuning somente quando preservam o contrato.
9. Capítulos especializados anteriores, como Geometry Core, Combine/Fuse, Viewport e Photo Projection, para comportamento/rationale complementar.
10. **07 — Pesquisa** como rationale/evidência, não requisito automático.
11. Ideias marcadas como futuras, experimentais ou candidatas nunca superam uma decisão normativa posterior.

Se duas decisões normativas posteriores ainda forem incompatíveis, o framework deve sinalizar **contradição** em vez de escolher silenciosamente.

# Vocabulário normativo

## Termos de usuário

A documentação de produto deve preferir:

- **Point** quando o contexto for amigável; `Vertex` pode aparecer como termo técnico/avançado.
- **Round Edge** como linguagem amigável; `Bevel/Chamfer` na documentação técnica.
- **Fuse** para fundir volumes.
- **Cut** para remover volume.
- **Connect** para criar continuidade entre partes.
- **Keep Parts** para preservar partes independentes em uma composição.
- **Join** para um único objeto com ilhas/submeshes separados.
- **Project From Reference** quando houver uma referência cadastrada; **Project From View** para projeção livre/custom.
- **Motion** para um movimento procedural pronto (andar, correr, respirar…); tecnicamente *Motion Recipe / Procedural Motion Generator*. Ajustes em **Speed, Energy, Weight, Stride, Lean** e **Style**.
- **Wiggle** para movimento secundário (cauda, cabelo, capa); tecnicamente *spring bones*.
- **Ghosts** para as silhuetas de poses vizinhas e **Motion Trail** para o caminho de um osso (evitar "Path", já usado por Spline/generators); tecnicamente *pose ghosting / onion skin* e *trajectory / motion path*.
- **Apply Now** para converter um Motion vivo em keys editáveis e **Keep Live** para mantê-lo vivo (P3D-160); tecnicamente *bake to keys*.
- **Tweak** para um ajuste aditivo sobre um Motion; tecnicamente *additive layer*.
- **Reference Frames** para uma sequência de imagens de referência; tecnicamente *ReferenceSequence*.
- **Role** para o papel de um osso no rig; tecnicamente *RigRole*.

## Termos técnicos internos

- `Fuse` pode usar **Union**.
- `Cut` pode usar **Difference**.
- `Intersect` é avançado/posterior, não operação principal da V1.
- `BooleanProvider` é infraestrutura interna/substituível.
- `half-edge` ou estrutura equivalente é detalhe do Geometry Core.

A documentação para iniciantes não deve introduzir CSG/half-edge/UV internals antes que isso seja necessário para uma tarefa concreta.

# Marcadores de status

O framework deve interpretar os seguintes conceitos:

## Core V1

Requisito obrigatório da experiência principal. Deve gerar especificação de implementação, critérios de aceite e testes.

## Official Extension

Capacidade mantida oficialmente, mas isolada do core quando isso reduz acoplamento. Pode vir instalada/ativada por padrão.

## V1.x

Planejada após a primeira baseline funcional. Não bloquear a conclusão do MVP salvo dependência explícita.

## Community Plugin

Extensão opcional. Não gerar requisito para instalação padrão.

## Experimental / Pesquisa

Ideia em avaliação. Gerar documentação de pesquisa/roadmap, nunca task obrigatória de implementação sem promoção explícita.

## Out of Scope

Não implementar no baseline atual. O framework deve tratar sua introdução como mudança de escopo que exige decisão explícita.

# Regras de derivação

O framework **pode derivar e fechar autonomamente decisões técnicas** quando elas estiverem dentro da delegação arquitetural do capítulo 09 e o Livro Vivo já fornecer contexto suficiente. Não deve transformar escolhas de dependência, representação, formato interno, concorrência, segurança, memória, testes ou fronteiras de módulos em perguntas ao usuário apenas por existirem múltiplas soluções tecnicamente válidas.

Quando uma decisão técnica afetar materialmente identidade do produto, escopo funcional, nomenclatura pública, workflow, UI/UX, branding ou acessibilidade, o framework deve parar na fronteira de produto: pode propor/recomendar, mas não promover silenciosamente a mudança.

O framework **pode derivar tarefas técnicas** de uma decisão normativa, mas não pode inventar funcionalidade de produto.

Exemplo permitido:

```
Decisão: toda mutação é transacional.

Tarefas derivadas:
- Transaction API
- rollback
- Undo entry
- testes de falha parcial
```

Exemplo proibido:

```
Decisão: Paint on Model simples.

Inferência indevida:
- adicionar Substance-like procedural painting
```

# Regras para arquitetura gerada

Toda arquitetura proposta pelo framework deve respeitar:

- core pequeno;
- **Explicit Modular Data Architecture** do capítulo 34;
- dados concretos e ownership explícito antes de abstrações;
- DOD seletivo, nunca ECS World universal;
- `Tool → Command → Algorithm → Data` como cadeia funcional;
- Tool não chama Tool;
- módulos desacoplados por contratos estáveis;
- Application API/Command Registry como fronteira comum para UI, plugins e MCP;
- mutations transacionais;
- long-lived relationships por IDs, com handles topológicos generacionais validados;
- Geometry Core sem vazamento de estruturas internas;
- providers externos atrás de interfaces próprias;
- traits apenas em boundaries de substituição/extensão reais;
- single-writer Document + snapshots para renderer/jobs;
- `unsafe` isolado e auditável;
- Cargo features apenas para capabilities aditivas, não como module system;
- facilidade de implementação e manutenção como objetivo explícito;
- não otimizar para escala muito além do foco low-poly sem requisito real.

# Regras para plugins

- API pública de terceiros: **Lua**.
- Plugins não recebem acesso direto à memória/half-edge interna.
- Capabilities declarativas.
- Mutação via transactions/Application API.
- Sem ABI nativo público obrigatório na V1.
- Official native providers são infraestrutura interna, não precedente automático para community native plugins.
- Community Plugins podem registrar **Plugin Panels** somente pela `Petunia UI Extension API` do capítulo 36.
- Plugin Panels não recebem acesso cru a egui, wgpu, Painter, raw input ou markup arbitrário; usam Petunia Components e extension slots controlados.
- Theme Extensions `.petunia-theme` são declarativas e não executam código durante resolução de tokens.
- Um plugin funcional pode embutir um theme package declarativo, mas temas e código do plugin continuam superfícies separadas.
- Acessibilidade, keyboard/focus, current theme e layout estrutural permanecem sob autoridade do host Petunia.

# Regras para MCP

- MCP é adapter sobre a Application API.
- Não controlar UI por automação de cliques como caminho principal.
- Preferir tools semânticas e schemas estruturados.
- Não oferecer execução arbitrária de código como capacidade padrão.
- Respeitar capabilities/permissões.
- Operações editáveis devem gerar transactions/Undo.
- Plugins só são expostos via MCP quando declarados como exponíveis e autorizados.

# Regras para documentação de UX

A documentação deve ensinar o fluxo mais simples primeiro:

```
Reference
→ Draw/Create
→ Shape
→ Paint/Project
→ Check
→ Export
```

Point/Edge/Face, UV manual, triangulação e topology avançada aparecem como escape hatch e aprendizado progressivo, não como pré-requisito para começar.

# Regras para documentação de modelagem

Sempre deixar claro que Petunia não é um editor 2D que adivinha 3D. Profiles são formas planas localizadas em um **Work Plane conhecido no espaço 3D**.

A documentação deve destacar dois caminhos igualmente válidos:

```
Profile → Extrude/Shape
Primitive → Direct Edit
```

Não apresentar Draw/Profile como obrigação quando uma primitiva for mais simples.

# Regras para booleans

Ao gerar documentação de usuário:

- usar Fuse e Cut;
- explicar Union/Difference apenas em documentação avançada/técnica;
- não sugerir Boolean para Extrude/Push-Pull simples que podem ser operações locais;
- não adicionar remesh automático ao fluxo;
- manter preview, validação e Undo como requisitos.

# Regras para UV e textura

Priorizar automação:

- Auto UV;
- Project From Reference/View;
- packing simples;
- texel/pixel density;
- stretch warnings;
- Paint on Model.

Editor UV manual existe para controle/correção, não como rito obrigatório antes da pintura.

# Regras para pesquisa e fontes

Páginas de pesquisa servem para justificar decisões e orientar estudos futuros. Ao transformar pesquisa em documentação técnica:

- diferenciar claramente fato observado, inspiração e decisão adotada;
- não assumir que Petunia precisa reproduzir todas as features de uma referência;
- Shapr3D/Plasticity/MoI são referências principalmente de interação/workflow, não autorização para adotar B-Rep/Parasolid;
- Blockbench/picoCAD/Dust3D/Asset Forge ajudam a avaliar simplicidade e low-poly, mas suas limitações não precisam ser copiadas.

# Saída esperada do framework

Ao gerar um projeto/implementation bible a partir do caderno, produzir pelo menos:

1. visão e princípios;
2. escopo funcional por versão;
3. arquitetura por módulos;
4. contratos/interfaces principais;
5. modelo de dados/documento;
6. geometry/topology invariants;
7. workflows de UX;
8. plugin/MCP contracts;
9. formatos/import/export;
10. estratégia de testes;
11. critérios de aceite;
12. riscos e dependências externas;
13. itens explicitamente fora de escopo;
14. backlog separado para V1.x e Experimental.

# Regra de coerência

Antes de gerar uma nova versão da documentação, o framework deve executar uma etapa de **contradiction/delta check** entre o baseline anterior e o Livro Vivo atual. Mudanças de vocabulário, escopo ou arquitetura devem aparecer explicitamente no delta.

## UI-BASELINE e documentação de interface

A fase de decisão de UI/UX possui fundamentos nos capítulos 22–26, implementação nos capítulos 27–35 e **baseline final normativa no capítulo 36**. `Rust + egui + eframe + egui-wgpu + wgpu` são a plataforma final corrente. O framework deve distinguir rigorosamente três classes de informação:

1. **FIGMA-CONFIRMED** — medidas, componentes, variants e padrões efetivamente observados na referência. Servem como evidência visual, não como requisito automático do produto.
2. **PETUNIA-BASELINE-FINAL** — decisões aprovadas e congeladas, incluindo shell, layout, medidas de referência, typography, accent, motion, gestures, focus, shortcuts, workspace flows, Theme Extensions e Plugin Panels.
3. **TUNING** — calibrações pequenas que preservam arquitetura, hierarquia, semântica e acessibilidade. Tuning não autoriza trocar toolkit, abrir docking irrestrito ou conceder raw UI/GPU access a plugins.

Não gerar novos itens `UI-OPEN/PROTOTYPE` para assuntos fechados no capítulo 36. Reabertura material exige decisão explícita e, quando arquitetural, ADR.

Os capítulos especializados são:

- [22 — Referência de Interface: Análise do Figma Blender UI Redesign](22-referencia-interface-figma.md);
- [23 — Macroarquitetura da Interface Petunia3D](23-macroarquitetura-interface.md);
- [24 — Design System Visual: Tokens, Hierarquia e Estados](24-design-system-tokens-estados.md);
- [25 — Biblioteca de Componentes e Contratos de Interação](25-biblioteca-componentes-interacao.md);
- [26 — Figma → Implementação, Assets e Fundamentos de Acessibilidade](26-figma-implementacao-acessibilidade.md).

O framework **pode gerar scaffolding Rust, Cargo workspace, Petunia Components sobre egui, adapters de crates auxiliares, wgpu renderer skeleton, design tokens, command contracts e tarefas do vertical slice** a partir da baseline atual. Não pode congelar silenciosamente itens UI-OPEN.

O framework também não deve converter automaticamente código React/Tailwind gerado pelo handoff Figma em arquitetura de produção nem introduzir web runtime/CEF. O alvo corrente é **Rust + egui + eframe + egui-wgpu + wgpu**.

É permitido implementar mocks/ports/interfaces headless necessários para testar o core, desde que isso não congele a experiência visual.

# Regras específicas para geração da stack Rust

Ao gerar o projeto, o framework deve:

1. criar Cargo workspace com as crates descritas no capítulo 28, evitando crate por command/feature pequena;
2. aplicar o capítulo 34 como contrato de representação e ownership;
3. impedir egui/eframe de vazar para core/geometry/render;
4. impedir wgpu de vazar para geometry/document;
5. usar Commands tipados/Command Registry como fronteira comum de UI/Lua/MCP;
6. manter algorithms independentes de UI/Undo/Lua/MCP sempre que possível;
7. manter Tokio restrito a `petunia-mcp`/serviços async reais;
8. tratar `manifold-rust` e xatlas como providers/implementações substituíveis;
9. manter `.petunia` e ExportModel independentes dos tipos de vendors;
10. gerar tests junto de cada feature, incluindo interoperability tests entre ferramentas;
11. usar `cargo xtask verify` como verificação integrada e adicionar architecture checks automatizáveis;
12. gerar knowledge map/`AGENTS.md` hierárquico em vez de um único arquivo gigante;
13. executar o vertical slice do capítulo 31 como teste de conformance da baseline final; falha localizada deve corrigir adapter/dependency/implementação, e somente bloqueador estrutural comprovado pode disparar novo ADR.

# Regra final

**O caderno descreve intenção e contratos; o framework transforma isso em implementação sem aumentar silenciosamente o produto.**

# Consolidação — Readiness, backlog e completude documental

Esta seção absorve as regras operacionais antes duplicadas entre o Livro Vivo e a antiga Implementation Bible.

## Gates de readiness

Para P3D P0/P1, `SPEC READY` exige, conforme aplicável: objetivo; escopo e non-goals; auditoria do código; dependências; estado/modelo de dados; boundaries Core/Application/UI/Renderer; Commands/TextIds/IconIds/ThemeTokens/keymap; Undo/persistência; erros/invalid states; performance; segurança; testes; documentação; acceptance criteria; riscos de migração/rollback.

`IMPLEMENTATION READY` exige adicionalmente: baseline tests executados; bloqueadores conhecidos; arquivos/módulos reais mapeados; architecture checks relevantes definidos; nenhuma decisão estrutural crítica deixada implícita.

## Estados canônicos de maturidade

`IDEA/ROADMAP → SPEC DRAFT → SPEC READY → AUDITED → IMPLEMENTATION READY → IN PROGRESS → VALIDATION → DONE`.

Também existem `ABSORBED` para itens preservados historicamente mas incorporados por outra especificação e `RULE` para invariantes transversais. `DONE` só pode ser atribuído com evidência.

## Governança do backlog

- IDs P3D existentes não são renumerados.
- Requisitos absorvidos permanecem registrados para rastreabilidade.
- Novas funcionalidades recebem IDs novos.
- Itens pós-GA não entram no GA sem decisão explícita.
- Dependências locais de uma P3D não substituem a ordem global das Gauntlet Waves.

## Documentação como parte da feature

Mudança user-facing deve avaliar manual, documentação pública, screenshots reais, changelog e referências geradas. Commands, keybinds, `IconId`, `TextId` e `ThemeToken` devem ser gerados/validados automaticamente quando possível, e CI deve detectar drift. Mockup não constitui evidência de implementação.

## Fonte única após o merge

O único caderno canônico é **Petunia3D — Livro Vivo**. O antigo caderno “Implementation Bible” foi absorvido como [Especificações P3D — Readiness e Gauntlet Waves](../especificacoes-p3d-readiness-gauntlet-waves.md). Referências antigas ao nome continuam válidas como alias histórico, mas não criam uma segunda autoridade documental.