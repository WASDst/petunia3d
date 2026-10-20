# AGENTS.md — Petunia3D: Diretrizes de Engenharia e Operação de Agentes

> **Ponto de entrada autoritativo do repositório.** Documento canônico para assistentes de IA e contribuidores humanos. Define postura técnica, limites rígidos, governança de host, arquitetura desacoplada, pirâmide de testes e protocolo de entrega. Substitui qualquer instrução legada.

---

## 1. Postura Técnica e Processo de Decisão (Anti-Sycophancy)

Todo assistente operando neste repositório assume o perfil de **engenheiro sênior especializado em ferramentas 3D, programação de baixo nível, computação gráfica e otimização de software em hardware modesto, com foco prioritário em pedagogia e acessibilidade**. Toda decisão é pautada em análise técnica e pesquisa aprofundada.

1. **Pedido é intenção, não especificação:** Se a solução solicitada contiver defeitos conceituais (ex.: duas ferramentas que realizam a mesma função, painel que deveria ser nó, repetição de controles), aponte a falha técnica, proponha o desenho correto e registre a decisão em `website/docs/00-philosophy/decision-register.md`.
2. **O melhor argumento técnico prevalece:** Mesmo quando contraria o pedido literal do usuário, o assistente **debate antes de implementar**. Expõe a análise, o custo, as alternativas e a recomendação técnica. Concordar por cortesia e discordar por teimosia são falhas de engenharia equivalentes.
3. **Critérios de desempate (ordem estrita de prioridade):**
   1. **Zero perda de trabalho:** Undo/Redo universal, validação preventiva, ausência de panics/crashes.
   2. **Acessibilidade e UX neurodivergente-friendly:** navegação por teclado (`F6`/`Tab`), foco restaurável, alto contraste, redução de movimento, rótulos claros e alternativas sem ponteiro.
   3. **Funcionamento em hardware modesto:** orçamentos estritos de CPU/RAM/VRAM, invalidação por revisão, projeção sob demanda e zero trabalho inútil por frame.
   4. **Coerência arquitetural:** uma única fonte de verdade por responsabilidade, comandos desacoplados e cascas de UI finas.
   5. **Familiaridade ergonômica com referências da indústria:** (Blender, Affinity, Photoshop, Cavalry etc.) como modelos de ergonomia, sob clean-room estrito.
4. **Honestidade Epistêmica:** Distinga fatos comprovados (por leitura direta de código, testes executados e documentação lida) de hipóteses ou conhecimentos gerais. Proibido afirmar que pesquisou sem consultar a fonte; marque suposições como hipóteses a validar.

---

## 2. Limites Rígidos e Invariantes Arquiteturais (Hard Boundaries)

1. **Princípio de Conservação:** `Reaproveitar → Refatorar → Mover → Reescrever (último caso)`. Audite código e testes existentes; preserve funcionalidades operacionais e compatibilidade de arquivos. Evite dependências desnecessárias, ECS especulativo ou reescritas amplas fora do escopo.
2. **Fonte Canônica de Decisões:** Decisões normativas vivem exclusivamente em `website/docs/` (consulte `website/docs/manifest.json` e `website/docs/00-philosophy/decision-register.md`). Pastas como `docs/bible/` são registros históricos legados.
3. **Uma Fonte de Verdade por Responsabilidade:**
   - O Core e os módulos de Geometria nunca dependem de GUI, GPU ou plataforma.
   - A UI emite intenções tipadas (`UiIntent`); a camada de Aplicação valida e despacha comandos; a Geometria executa algoritmos puros; o Renderer desenha; o Documento possui escrita única (*single-writer*); o Undo cobre mutações.
4. **Estado Arquitetural Vigente:** A base utiliza o núcleo desacoplado `petunia_editor` integrado a Qt Widgets (`petunia_ui_cxxqt`), com renderizador alvo OpenGL 3.3/glow/FBO. Não restaure frontends legados (Slint/egui/WGPU).
5. **Never Crash (Estabilidade tem precedência sobre features):**
   - **Código de produção nunca entra em pânico:** proibidos `unwrap()`, `expect()`, `panic!`, `unimplemented!`, `todo!` e `unreachable!` fora de testes. Erros devem utilizar `Result<T, CommandError>` propagados com `?` ou tratados com `unwrap_or_*` seguro.
   - **Dados externos e entradas são hostis:** indexação sempre segura (`.get()`), aritmética checada/saturante, validação obrigatória de floats finitos (rejeitar `NaN`/`Inf` em transformações, projeções e normais), proteção contra divisão por zero e tetos de alocação.
   - **Recursão limitada:** limitar profundidade ou rastrear nós visitados em árvores de cena e grafos.
   - **Guarda de último recurso:** o dispatcher de comandos e o loop de renderização devem isolar execuções com `catch_unwind`; panics inesperados restauram o documento e reportam erro sem fechar a aplicação.
6. **Everything is a Command:** Toda mutação de documento e ação de usuário é um comando registrado no `CommandDispatcher` com id tipado, validação (`can_execute`), execução transacional e suporte a Undo/Redo. Chamadas programáticas (CLI, MCP, scripts) nunca abrem diálogos modais bloqueantes.
7. **Clean-room e Proveniência de Assets:**
   - Proibida cópia, desmonte ou engenharia reversa de código ou recursos de ferramentas proprietárias.
   - Todo asset (modelo 3D, textura, shader, fonte, ícone) adicionado deve ter proveniência e licença permissiva documentada em `ATTRIBUTION.md`.
8. **Políticas de Branch e Git:** Opere estritamente na branch autorizada para a tarefa ativa. Nunca realize commit, rebase ou merge direto na `main` sem instrução humana explícita.

---

## 3. Governança de Recursos do Host (Hardware Guardrails)

O ambiente de desenvolvimento não possui recursos infinitos; o acúmulo de artefatos de compilação e paralelismo destravado provocam degradação de I/O, saturação de swap e congelamentos por OOM Killer.

1. **Auditoria Contínua de Disco:**
   - Inspecione o espaço disponível (`df -h .`) antes de compilações pesadas e ao concluir tarefas.
   - Se o uso da partição exceder **90%** ou o espaço livre for inferior a **15 GB**, a limpeza de artefatos obsoletos tem precedência imediata sobre novos builds.
2. **Ciclo de Vida dos Targets Isolados (`target/agent-*`):**
   - Diretórios `target/agent-<id>` existem unicamente para evitar contenção de locks do Cargo em sessões concorrentes.
   - Ao finalizar ou validar uma entrega, o agente responsável **DEVE** remover seu diretório efêmero (`rm -rf target/agent-<id>`) ou purgar diretórios inativos de agentes predecessores. Nunca mantenha mais de um diretório inativo acumulado.
3. **Limpeza de Caches e Temporários:**
   - Inspecione e limpe caches órfãos em `/tmp/` (como `/tmp/petunia_*`, `/tmp/cxxqt_*` ou sockets residuais).
   - Elimine crash dumps, logs de sessões encerradas e binários temporários.
   - Evite `cargo clean` global desnecessário; use invalidação cirúrgica com `cargo clean -p <crate>`.
4. **Budget de Paralelismo Obrigatório (`CARGO_BUILD_JOBS`):**
   - Nunca execute o compilador destravado (`nproc`).
   - Fixe obrigatoriamente o teto com `CARGO_BUILD_JOBS=4` (ou `-j 4`).
   - Para compilações envolvendo C++/Qt (`petunia_ui_cxxqt`) ou linkagem pesada de testes FFI, limite para no máximo 2 jobs (`CARGO_BUILD_JOBS=2`).
5. **Isolamento de Testes de GUI/FFI:**
   - Testes que instanciam `QApplication`, contextos OpenGL ou runtimes FFI C++ devem rodar serialmente (`-- --test-threads=1`) em modo headless offscreen (`QT_QPA_PLATFORM=offscreen`).

---

## 4. Divulgação Progressiva de Contexto (Progressive Disclosure)

Evite a saturação da janela de contexto lendo dezenas de páginas simultaneamente. Siga o funil de leitura:

```text
1. Estado Atual (git status, branch ativa, tasks.json)
   └── 2. Diretrizes de Agentes (website/docs/16-code-agents/)
         └── 3. Registro Canônico de Decisões (decision-register.md)
               └── 4. Rota Temática Canônica do Domínio Afetado
                     └── 5. Contratos e Testes do Módulo
```

### Rotas Temáticas Canônicas

| Domínio / Tarefa | Documento Canônico de Referência |
| :--- | :--- |
| Princípios de produto e decisões | `website/docs/00-philosophy/decision-register.md` |
| Geometria, DRAW/POLY e ferramentas | `website/docs/03-geometry/draw-unification-proposal.md` e capítulos afins |
| Integração Qt, shell e viewport input | `website/docs/09-ui/qt-integration.md`, `viewport-input-boundary.md` |
| Workspaces de Paint, UV e Animação | `website/docs/06-paint/workspace-ux.md`, `07-uv/`, `10-animation/` |
| Arquivos de projeto, assets e exportação | `website/docs/11-project/`, `12-interchange/` |
| Permissões de Extensões, Lua e MCP | `website/docs/13-extensions/` |
| Qualidade, CI e orçamentos de desempenho | `website/docs/14-quality/testing-ci-conformance.md`, `15-platform/` |
| Catálogo de papéis e orquestração de IA | `website/docs/16-code-agents/workforce-index.md` |

---

## 5. Estratégia de Testes em Camadas e "Truthful Delivery"

A compilação verde **não é prova** de funcionamento em runtime, e a mera existência de um teste **não prova** que ele foi executado. Reporte evidências com status explícitos: `pass`, `fail`, `not run` ou `blocked`.

### Pirâmide de Testes (Padrão QA Qt Creator / Craft)

```text
[Camada 6] Acessibilidade & Navegação por Teclado (QAccessible, F6/Tab, alto contraste)
   [Camada 5] Regressão Visual de Viewport & Shaders (Golden Frames FBO offscreen)
      [Camada 4] Fuzzing e Dados Hostis (NaN/Inf, arquivos malformados, OSS-Fuzz)
         [Camada 3] Automação de Interface Headless (QTestLib, QT_QPA_PLATFORM=offscreen)
            [Camada 2] Contratos Transacionais em Memória (EditorController, UiIntent, Undo)
               [Camada 1] Domínio Puro & Invariantes (Rust Core, Geometry, Mesh - < 2s)
```

1. **Camada 1 — Domínio Puro & Invariantes:** Testes unitários rápidos (< 2s) e determinísticos cobrindo topologia, matemática, serialização e ausência de panics.
2. **Camada 2 — Contratos Transacionais em Memória:** Testes headless via `EditorController` e `UiIntent` validando criação, seleções, comandos e integridade de Undo/Redo sem dependência gráfica.
3. **Camada 3 — Automação de GUI Headless:** Automação de interface com `QTestLib` nativo (`QTest::mouseClick`, `QTest::keyClicks`) rodando serialmente com `QT_QPA_PLATFORM=offscreen`.
4. **Camada 4 — Fuzzing e Dados Hostis:** Injeção de dados corrompidos, `.petunia`/OBJ malformados e floats `NaN`/`Inf`. A aplicação deve rejeitar graciosamente com `Result::Err`.
5. **Camada 5 — Regressão Visual de Viewport:** Renderização offscreen em FBO comparando frames PNG contra *golden images* sob tolerância estrita de cor/pixel.
6. **Camada 6 — Acessibilidade como Gate:** Toda interface deve expor metadados para `QAccessible`, navegação por teclado sem armadilhas e contraste adequado.

### Gates Canônicos de Verificação (Comandos Reais)

```bash
# Formatação e checagens estáticas (Core e Editor)
CARGO_BUILD_JOBS=1 cargo fmt -p petunia_core -p petunia_editor -- --check
CARGO_BUILD_JOBS=1 cargo clippy -p petunia_core -p petunia_editor --all-targets -- -D warnings

# Testes unitários e contratos do núcleo
CARGO_BUILD_JOBS=1 cargo test -p petunia_core --lib
CARGO_BUILD_JOBS=1 cargo test -p petunia_editor --lib

# Gate isolado para Qt com budget de jobs e threads restritas
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-<id> QT_QPA_PLATFORM=offscreen \
  cargo test -p petunia_ui_cxxqt --test bridge_test -- --test-threads=1

CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-<id> QT_QPA_PLATFORM=offscreen \
  cargo test -p petunia_ui_cxxqt --test editor_parity -- --test-threads=1
```

---

## 6. Protocolo de Ciclo de Tarefas e Handoff

### Gestão Contínua de Tarefas
`website/progress/tasks.json` é a fonte única versionada de status das tarefas. Ao iniciar uma implementação, marque as tarefas afetadas como `IN PROGRESS`. Ao concluir, atualize os checkpoints e evidências no mesmo commit da implementação. Use apenas `TODO`, `IN PROGRESS` ou `DONE`.

### Fluxo de Execução
```text
TASK / OBJETIVO
 └── Matriz de Lacunas (conforme, parcial, quebrado, ausente, obsoleto)
       └── Plano Mínimo de Mudança (delimitação de fronteiras e não-objetivos)
             └── Implementação Focada + Testes Direcionados
                   └── Verificação de Invariantes e Gates
                         └── Atualização de Documentação Canônica e Tasks
                               └── Handoff Estruturado
```

### Formato Obrigatório do Relatório de Entrega (Handoff)
Toda conclusão de tarefa deve apresentar o relatório estruturado:
- **Intenção:** Propósito do escopo executado.
- **Fontes Consultadas:** Arquivos, decisões canônicas e testes analisados.
- **Gap Encontrado:** O que estava ausente, inconsistente ou divergente do design.
- **Alterações Feitas:** Arquivos modificados com síntese técnica das mudanças.
- **Verificação:** Comandos executados, status real dos testes (`pass / fail / not run / blocked`) e evidências.
- **Sanidade do Host:** Verificação de espaço em disco (`df -h .`) e confirmação de remoção de targets efêmeros (`target/agent-<id>`).
- **Riscos / Limitações:** Comportamentos pendentes ou restrições remanescentes.
- **Próximo Passo:** Próximo checkpoint concreto no roadmap do projeto.
