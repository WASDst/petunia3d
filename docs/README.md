# Documentação Canônica do Projeto Petunia3D (`docs/`)

## O que é este diretório?
A pasta `docs/` contém a fonte canônica da verdade do Petunia3D: o **Livro Vivo** (`docs/bible/`) e a documentação derivada de engenharia, produto, arquitetura, testes, manuais e governança.

## Fonte única
O caderno canônico é **[`docs/bible/`](bible/index.md) — Petunia3D Livro Vivo** (257 páginas): raiz do Livro Vivo, hub de Especificações P3D/Readiness, `constitution/` (00–16), `foundations/` (01–45), `specs/` (P3D-001 a P3D-174), `sections/` (A–O) e `addenda/`. Toda outra documentação deve obedecer a ele. A antiga *Implementation Bible* foi absorvida e não é uma segunda autoridade.

## 🧊 Site de documentação congelado
O site público (VitePress) está **congelado até o fim do desenvolvimento do projeto**: `docs/.vitepress/**`, `docs/index.md`, `docs/public/**`, `docs/image-references/**` e o workflow de deploy não devem ser trabalhados. Ver [`AGENTS.md`](../AGENTS.md) §1.

## Para que serve?
Implementa o princípio de **Documentação Canônica Viva**: o repositório é autossuficiente e todo o conhecimento técnico essencial reside diretamente no código, no caderno e em arquivos Markdown padronizados e versionados.

## Roteador Central
Consulte [`NAVIGATION.md`](NAVIGATION.md) como ponto de entrada principal para navegação guiada por intenção (usuário, desenvolvedor, operador, agente).

## Inventário Completo de Documentação
- [`NAVIGATION.md`](NAVIGATION.md): Roteador central de intenção (mapa mestre de navegação).
- [`ARCHITECTURE.md`](ARCHITECTURE.md): Grafo de crates, regras de dependência, eventos e render-on-demand.
- [`GAUNTLET.md`](GAUNTLET.md): Histórico de validações independentes, estabilidade, benchmarks e evidências.
- `architecture/`: Arquitetura do sistema, boundaries, contratos de Clean Code e ADRs.
- `product/`: Visão de produto, proposta de valor, público-alvo e limites de escopo.
- [`development/premium-interaction-plan.md`](development/premium-interaction-plan.md): plano e evidências da rodada premium.
- `development/`: Padrões de código, diretrizes de Rust e estratégia de testes com quality gates.
- `operations/`: Procedimentos de deploy, ciclo de vida de instalação, observabilidade e telemetria.
- `reference/`: Referência da CLI, variáveis de ambiente, códigos de saída e parâmetros.
- `manual/`: Manuais de instalação passo a passo e guia prático de uso/atalhos do viewport.
- `ui/`: Design system, tokens, especificação dos 4 workspaces e fluxos de tela.
- `security/`: Modelagem de ameaças (STRIDE), contratos de segurança e trust boundaries de I/O.
- `governance/`: Políticas de branches, commits convencionais e regras de PR.
- `bible/`: **Caderno canônico completo** (Livro Vivo + hub de Especificações P3D/Readiness + 257 páginas).
- `audits/`: Auditorias de conformidade e evidências (inclui [`audits/bible-conformance/`](audits/bible-conformance/README.md)).
