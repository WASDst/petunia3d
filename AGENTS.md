# AGENTS.md — Regras obrigatórias para todos os code agents

> Leia este arquivo **antes** de qualquer tarefa neste repositório. Ele é normativo.
> Em conflito com qualquer outra instrução do repositório, este arquivo prevalece;
> em conflito com o caderno canônico, o caderno prevalece sobre este arquivo.

## 0. Fonte única da verdade

O caderno canônico vive em **`docs/bible/`** — 258 páginas: raiz `index.md`
(Petunia3D — Livro Vivo), `constitution/` (00–16), `foundations/` (01–46),
`specs/` (P3D-001 a P3D-174), `sections/` (A–O), `addenda/` e `status/`.

- Toda decisão de arquitetura, produto, UX, escopo, vocabulário e documentação
  deve sair do caderno. Nenhuma outra documentação pode contradizê-lo.
- A antiga **Implementation Bible** foi absorvida pelo Livro Vivo. Referências a
  esse nome são alias histórico — nunca uma segunda autoridade.
- O espelho em `docs/bible/` é **derivado e verificado**. Não edite uma página
  do caderno em dois lugares: o caderno é o único lugar.
- Hierarquia de autoridade em conflito (cap. 13): `32` (ADR Odin→Rust) → `34`
  (representação/ownership/Rust safety) → `27–31`, `35`, `36` (UI Baseline final)
  → `09` → `21` → `12` → `14–20` → `22–26` → capítulos especializados → `07`
  (pesquisa, não requisito). Contradição não resolvida deve ser **sinalizada**,
  nunca escolhida em silêncio.

## 0.1 Frontend de produção e legado

**Decisão 2026-09-20: Slint é o frontend de produção.** A crate `petunia_ui_slint`
(`crates/ui-slint/`) é a interface principal do Petunia3D. O binário `petunia3d`
executa o shell Slint por padrão; a UI egui foi arquivada e permanece acessível
somente para transição via flag `--legacy-egui` ou variável de ambiente
`PETUNIA_LEGACY_EGUI=1`.

Consequências operacionais:

- **Slint é a superfície de produto**. Novos painéis, componentes e fluxos de UI
  devem ser implementados em `crates/ui-slint/` usando a linguagem declarativa
  `.slint` e o bridge Rust em `lib.rs`.
- **egui é legado de transição**. A crate `crates/ui/` (`petunia_ui`) e o host
  `crates/app/` (`petunia_app`) permanecem no workspace para retrocompatibilidade,
  mas não recebem novas features de product UI. Correções críticas são permitidas;
  expansão de superfície é proibida.
- **Contratos de UX são toolkit-neutros**. Os capítulos 23 e 36 do caderno definem
  a UI Baseline Final V1 em termos de princípios, medidas, workspaces e
  acessibilidade — não de toolkit. A migração para Slint preserva esses contratos.
- **Dominío e domínio visual permanecem desacoplados**. `petunia_core`,
  `petunia_commands`, `petunia_project` e `petunia_config` não conhecem Slint nem
  egui. O bridge (`UiIntent` em `petunia_ui_slint`) é a única fronteira que
  traduz intents do shell para operações de domínio.
- **Viewport adapter é toolkit-neutro**. `PetuniaRenderer` (`crates/render/`) não
  conhece Slint nem egui. O adapter WGPU (`viewport_gpu.rs` no Slint,
  `egui-wgpu` no legado) é a única fronteira que conhece ambos.

### Regras de layout Slint

O shell Slint usa layout declarativo nativo (grid, horizontal, vertical, flex)
definido em `ui/app.slint`. Regras derivadas:

- **Viewport-first**: o viewport ocupa a região central e mantém ~`480 × 360`
  logical px antes de ceder espaço a painéis (capítulo 36).
- **Shell estrutural**: em DRAW e POLY (que substituem MODEL, ADR 007 de
  2026-09-29), trilho de ferramentas do workspace à esquerda e Inspector à direita
  com `Parts → Transform → Material → Object` (revisão de baseline aprovada em
  2026-09-23 no cap. 36); `Asset Library` inferior e `Top Bar` no topo. Nenhum
  docking irrestrito na V1. Até DRAW/POLY estarem funcionais, a UI mantém MODEL.
- **Gramática única de ferramenta** (constituição 11, ADR 007): ferramenta
  persistente, arrastar (alça ou em qualquer lugar), valor digitado,
  clicar-mover-clicar, RMB = menu, Esc cancela, navegação nunca suspensa. Não
  crie um ciclo próprio de sessão para uma ferramenta nova.
- **Componentes reutilizáveis**: componentes Slint declarativos em `ui/app.slint`
  (ex.: `TopAction`, `ToolButton`, `NumericField`, `Vector3Field`,
  `InspectorSection`, `ColorSwatch`) são a linguagem visual pública. Não reimplemente
  controles existentes.
- **Responsividade por layout nativo**: use `preferred-width`, `min-width`,
  `max-width`, `horizontal-stretch`, `vertical-stretch` e constraints Slint.
  Proibido calcular larguras manualmente em Rust (`if available_width < N`).
- **Overlays e modais**: pilha LIFO gerenciada por `OverlayStack` em `overlay.rs`;
  dismiss por `Escape` e click-away. Não crie modais ad-hoc fora da pilha.

### Guard e validação

```bash
cargo test -p petunia_ui_slint --lib          # 55 testes unitários
cargo clippy -p petunia_ui_slint --all-targets -- -D warnings
cargo fmt -p petunia_ui_slint -- --check
```

O `docs-check` e `bible-check` continuam válidos para o caderno canônico. O
`ui-guard` (confinamento de tipos egui) aplica-se somente à crate legado
`crates/ui/`; a crate Slint não está sujeita a esse guard porque não usa egui.

O `docs-check` inclui a validação do mapa de componentes (`docs/public/ui-map.json`)
contra o código. Esse arquivo está no **site público congelado** (AGENTS.md §1):
quando o código muda o símbolo de entrada de um nó, o mapa precisa de uma decisão
explícita de descongelar (`cargo xtask bible-lock`) — não edite o arquivo por
conta própria.

## 1. 🧊 SITE DE DOCUMENTAÇÃO CONGELADO

**Decisão de 2026-09-25: o site público de documentação está congelado novamente até o fim do desenvolvimento de todo o projeto para evitar consumo excessivo de tokens e tempo de compilação.** Nenhum agente deve trabalhar nele até a etapa final.

Congelado (não modificar, não "melhorar", não recompilar continuamente com VitePress):

- `docs/.vitepress/**` — config, nav, `bibleSidebar.ts`, tema, cache, `dist/`;
- `docs/index.md` — página inicial/hero do site;
- `docs/public/**`, `docs/package.json`, `docs/pnpm-lock.yaml`, `docs/vercel.json`;
- `.github/workflows/docs.yml` — publicação/deploy do site;
- `docs/image-references/**` — mockups e capturas usados como referência visual;
- promessa de site: busca, versionamento, i18n do chrome, screenshots publicados.

Motivo: o caderno canônico em `docs/bible/` é a única fonte da verdade; o site VitePress é superfície de apresentação final. Reconstruir e validar o site em cada passo consome muitos tokens e ciclos de compilação. Essa etapa fica reservada para o fechamento do projeto.

## 2. Regra de reconciliação antes de implementar

1. Auditar o código e o comportamento executável antes de mudar qualquer coisa.
2. Classificar cada requisito relevante como `COMPLIANT`, `PARTIALLY_COMPLIANT`,
   `FUNCTIONAL_BUT_DIFFERENT`, `RUDIMENTARY`, `STUB`, `BROKEN`, `DUPLICATED`,
   `MISSING` ou `OBSOLETE` — produzir uma **Implementation-vs-Spec Gap Matrix**.
3. Preservar o que já é `COMPLIANT`. Corrigir apenas o delta comprovado.
4. Reescrita completa só com evidência de que a arquitetura atual impede a
   correção incremental. Nada de big-bang rewrite.

## 3. Invariantes que nunca podem ser violadas

- Core e domínio não dependem de toolkit de UI (`egui`, `Slint`, `eframe`), widgets, cores, ícones ou teclas.
- Strings visíveis usam `TextId`; ícones usam `IconId`; aparência usa `ThemeToken`;
  ações semânticas usam `CommandId`. **Zero hardcode** de texto, ícone, cor ou
  atalho físico em UI pública.
- Input físico é resolvido por keymap. Tools não conhecem teclas como regra de negócio.
- Cadeia funcional: `Tool → Command → Algorithm → Data`. Tool não chama Tool.
- Mutations são transacionais; documento com single-writer; jobs operam em snapshots.
- `unsafe` isolado e auditável.
- UI Baseline V1 congelada, com revisões de 2026-09-23, 2026-09-29 (DRAW / POLY,
  ADR 007) e 2026-09-30 (painéis flutuantes não modais, até 2 viewports opcionais,
  tema claro, Paint+UV unificados; cap. 36): workspaces `DRAW / POLY / PAINT` (a
  edição de UV vive em "Preparar superfície" dentro do PAINT), trilho de
  ferramentas-esquerda / Inspector-direita com Parts como primeira seção /
  Asset Library-abaixo, dark oficial,
  Petunia Components como linguagem visual. Não reintroduzir docking irrestrito,
  clone de Blender, acesso cru a egui/wgpu para plugins nem reabrir `UI-OPEN`.
- Vocabulário de usuário (cap. 13): **Point**, **Round Edge**, **Fuse**, **Cut**,
  **Connect**, **Keep Parts**, **Join**, **Project From Reference/View**.
  `Vertex`, `Bevel`, `Union`, `Difference` são termos técnicos.

## 4. Definition of Done

Uma feature só termina quando, conforme aplicável: comportamento, testes,
arquitetura, UI/tokens, documentação, screenshots, changelog e referências
geradas estiverem sincronizados — e **nenhuma documentação conhecida como
obsoleta permanecer**. Gates obrigatórios: `cargo fmt --check`, `cargo check`,
testes relevantes, `cargo clippy` quando viável, architecture checks,
`cargo run -p xtask -- docs-check`, `cargo run -p xtask -- bible-check` e
`cargo run -p xtask -- ui-guard --strict` (confinamento de tipos de UI).

## 5. Protocolo de contexto

Leia, nesta ordem: este `AGENTS.md` → `ENTRYPOINT.md` → `prumo.json` →
`docs/PRUMO.md` → a página relevante em `docs/bible/` → o código e os testes
envolvidos. Contexto mínimo suficiente, expansão progressiva, ponteiro em vez de
payload. Nunca enfraquecer critérios de aceitação em silêncio.

## 6. Escolha de modelos — sem obrigatoriedade

Não há modelo ou provedor obrigatório para nenhuma função, inclusive para a
sessão primária. Use o modelo disponível e adequado à tarefa, considerando
capacidade, custo e contexto.

Preferências em `prumo.json` e modelos configurados nos agentes são sugestões ou
configurações de execução, não requisitos de autorização. Não interrompa o
trabalho, exija troca de modelo nem solicite confirmação apenas por divergência
entre o modelo da sessão e essas configurações.

A escolha de modelo não altera os requisitos de qualidade, segurança,
conformidade, testes e revisão definidos neste arquivo.

## 7. Destino de publicação autorizado

O repositório de trabalho é **`wasd-lat/petunia3d`**. O usuário autorizou
publicar nele e pediu **commit e push após cada alteração concluída**.
Use a conexão autenticada da conta `wasd-lat` quando houver múltiplas contas.
Verifique o conteúdo remoto após publicar e identifique claramente checkpoints
que ainda aguardam testes; publicação não substitui os critérios de aceite.
