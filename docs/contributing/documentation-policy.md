# Política de Documentação (Living Docs)

A documentação acompanha o código no mesmo PR — nunca depois. **Nenhuma tarefa é
concluída com documentação conhecida como obsoleta.**

## Fonte única

O caderno canônico é **[`docs/bible/`](../bible/index.md) — Petunia3D Livro Vivo**
(249 páginas): raiz do Livro Vivo, hub de Especificações P3D/Readiness,
`constitution/` (00–16), `foundations/` (01–44), `specs/` (P3D-001 a P3D-168),
`sections/` (A–O), `addenda/` e `status/`.

- A antiga **Implementation Bible** foi absorvida pelo Livro Vivo. O nome continua
  válido como alias histórico, nunca como segunda autoridade.
- Não edite a mesma página em dois lugares. O caderno é o único lugar onde a
  especificação muda; o resto deriva dele.

## Hierarquia de autoridade

Em conflito entre páginas, vale esta ordem (capítulo 13 do caderno):

1. `32` — ADR: Migração da Baseline Odin para Rust (stack vigente).
2. `34` — Arquitetura Modular Explícita, Rust Safety e Representação em Código.
3. `27–31` (stack/arquitetura/qualidade), `35` (ecossistema egui/Petunia Components)
   e **`36`** (UI Baseline Final V1, temas e Plugin Panels).
4. `09` — Arquitetura, Princípios de Decisão e Governança Técnica.
5. `21` — Baseline Funcional e UI V1 Congeladas, Stack Rust Final.
6. `12` — Baseline Funcional, Roadmap e Contrato de Escopo.
7. `14–20` — contratos técnicos especializados.
8. `22–26` — fundamentos/referências de interface (valores `UI-OPEN` antigos são
   superados por `36`).
9. Capítulos especializados anteriores (Geometry Core, Combine/Fuse, Viewport,
   Photo Projection) para comportamento e rationale complementar.
10. `07` — Pesquisa: rationale e evidência, **nunca requisito automático**.
11. Ideias marcadas como futuras/experimentais nunca superam decisão normativa posterior.

Contradição não resolvida deve ser **sinalizada**, não escolhida em silêncio.

## Marcadores de status

Ao documentar ou derivar tarefas, classifique o item como:

| Marcador | Significado |
| :--- | :--- |
| `Core V1` | Requisito obrigatório da experiência principal. |
| `Official Extension` | Mantida oficialmente, isolada do core. |
| `V1.x` | Planejada após a baseline; não bloqueia o MVP. |
| `Community Plugin` | Extensão opcional; não gera requisito de instalação padrão. |
| `Experimental` | Pesquisa/avaliação; gera roadmap, nunca task obrigatória. |
| `Out of Scope` | Fora do baseline; introduzir exige decisão explícita de escopo. |

## Regras

1. **CHANGELOG.md**: toda mudança funcional entra em `[Unreleased]`. Breaking
   changes de schema, formato de projeto, plugin API ou keymaps exigem
   **migration notes**.
2. **Novos atalhos/parâmetros/ferramentas**: atualize `docs/tools/`,
   `docs/shortcuts/` e os locales (`en.toml` e `pt-BR.toml` em paridade exata).
3. **Nova superfície de UI**: adicione o nó em `docs/public/ui-map.json`
   (`cargo run -p xtask -- ui-check` valida ids, arquivos, símbolos e aciclicidade).
4. **Nunca edite arquivos gerados à mão**: `docs/generated/` e `docs/changelog/index.md`
   são derivados do código. Regenere com `cargo run -p xtask -- docs-generate`.
5. **Vocabulário de usuário**: `Point`, `Round Edge`, `Fuse`, `Cut`, `Connect`,
   `Keep Parts`, `Join`, `Project From Reference/View`. `Vertex`, `Bevel`,
   `Union` e `Difference` são termos técnicos — nunca vocabulário primário de manual.
6. **Screenshots** são evidência de implementação, não de intenção. Nunca use
   mockup como prova de funcionalidade (P3D-118).
7. **Sem hardcode em UI pública**: texto usa `TextId`, ícone usa `IconId`, aparência
   usa `ThemeToken`, ação usa `CommandId`.

## Delta check obrigatório

Antes de publicar uma nova versão da documentação, rode uma etapa de
**contradiction/delta check** contra o caderno: mudanças de vocabulário, escopo ou
arquitetura devem aparecer explicitamente no delta — não podem entrar por omissão.

## Site público congelado

O site VitePress está **congelado até o fim do desenvolvimento do projeto**
(`docs/.vitepress/**`, `docs/index.md`, `docs/public/**`, `docs/image-references/**`,
workflow de deploy). Não trabalhe nele. Ver `AGENTS.md` §1 (na raiz do repositório).

## Verificação

```bash
cargo run -p xtask -- bible-check          # caderno, links, vocabulário, site congelado
cargo run -p xtask -- docs-generate --check # drift dos catálogos gerados e changelog
cargo run -p xtask -- ui-check             # mapa da UI contra o código
cargo run -p xtask -- arch-check           # auditoria arquitetural
```
