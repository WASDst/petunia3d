# Contexto — onde paramos (2026-10-19)

> Arquivo de **retomada**: leia isto primeiro na próxima sessão. Ele é curto de propósito; o detalhe mora nos documentos canônicos.

## Onde estamos

| Item | Valor |
|---|---|
| Repositório | `petunia3d` (github.com/WASDst/petunia3d) |
| Branch de trabalho | `refactor/architecture-foundation` |
| HEAD | `7df47dbd` — `feat(assets): galeria de assets em grade de miniaturas (C6, parte 1)` |
| Remoto | `origin` = `https://github.com/WASDst/petunia3d.git` (branch ainda **não** enviada quando este arquivo foi escrito) |
| main | `92c22caa` — **ancestral** do nosso branch (main não tem nada que a gente não tenha) |
| Toolchain | `1.98.1` (fixado em `rust-toolchain.toml`; o `stable` do ambiente é 1.97.1 e **não compila**) |

## O que foi entregue (Onda C + correções)

| Entrega | Estado | Commit |
|---|---|---|
| Importação de modelos (OBJ+`.mtl`+textura, glTF, GLB, multi-objeto) + menu **Arquivo → Importar** | DONE | `6305bab4` |
| C1 Camadas (efeitos como sub-camadas, máscara em camada e efeito, alvo do pincel) | DONE | `de656ab4` |
| C2 Projeção única, pintar só a seleção, isolar em fantasma | DONE | `de656ab4` |
| C3 Gradiente como camada de preenchimento | DONE | `de656ab4` |
| Correção do wireframe (arestas não "estouram" nas bordas) | DONE | `de656ab4` |
| C7 Grupo "Referência" no topo das Partes | DONE | `aa3d000d` |
| C8 Modos das operações no HUD (Região / Individual / Ao longo das normais) | DONE | `8828fa37` |
| Morph targets (blend shapes) no crate de malha | DONE | `4b8443de` |
| C4 Nós (picker de objeto, menu ao arrastar, booleana com operando fantasma) | DONE | `870c37fc` |
| C6 Galeria de assets em grade | **parcial** | `7df47dbd` |
| C5 Animate (aba de Curvas, timeline motion design, módulo de bones) | **parcial** | `4b8443de` (só morph targets) |
| Ondas D–F (acabamento da UI) | **planejadas** | — |

## Gates da última entrega (todos verdes)

`fmt` limpo · `clippy -D warnings` (9 crates, `--all-targets`) · core 212 · mesh 283 · project 297 · module-paint 125 · module-model 7 · editor 148 · config 27 · ui-cxxqt offscreen: `editor_parity` 25, `shell_v2` **328 verificações**, `ui_i18n_catalog` 3, `import_model` 8 · docs sem drift (`docs-generate --check`, `verify-progress`, `verify-agent-docs`).

## O que está em aberto (e por quê)

1. **C5 — aba de Curvas**: o modelo do Animate publica as trilhas e os *tempos* dos keyframes, mas não os **valores** das curvas; falta a amostragem no modelo e o editor de gráfico.
2. **C5 — timeline estilo motion design**: barra de transporte e dope sheet existem; a barra de tempo em motion design é reescrita de tela.
3. **C5 — bones/rigs/templates como módulo**: reorganização de `crates/editor/src/animate.rs` em crate próprio (forma, sem valor novo).
4. **C6 — arrastar asset para a viewport**: a viewport não aceita soltar assets.
5. **C6 — tags / salvar grupos**: dependem de **P04** (metadados de asset) e **G09** (coleções/prefabs), ambos TODO.
6. **Ondas D–F** (acabamento da UI): o próximo bloco de trabalho — ver [09-ui/shell-v2-polish-waves.md](website/docs/09-ui/shell-v2-polish-waves.md).

## Ambiente (economia de recursos)

- `CARGO_BUILD_JOBS=2` (a swap da máquina está cheia; subir o paralelismo mata o build).
- Disco: 58% usado, ~100 GB livres (o cache incremental foi limpo; vale repetir quando passar de 85%).
- Nenhum `target/agent-*` pendente.

## Próximos passos (ordenados)

1. **Enviar o branch** e abrir o PR da integração (ver [MERGE-PLAN.md](MERGE-PLAN.md)).
2. Rodar o [roteiro de teste manual](website/docs/09-ui/shell-v2-manual-test-checklist.md) no app real e devolver as falhas com captura.
3. **Onda D** (a casca da UI): D1–D7 — é o que o relato "mal polida" pede.
4. Depois: **Onda E** (viewport) e **Onda F** (consistência/a11y).
5. Em paralelo, se sobrar fôlego: a **aba de Curvas** (C5).

## Documentos canônicos (o que ler)

| Assunto | Arquivo |
|---|---|
| Roteiro do Shell v2 (ondas A–F) | `website/docs/09-ui/shell-v2-roadmap.md` |
| Acabamento da UI: Slint × Qt e ondas D–F | `website/docs/09-ui/shell-v2-polish-waves.md` |
| O que testar no app | `website/docs/09-ui/shell-v2-manual-test-checklist.md` |
| Decisões | `website/docs/00-philosophy/decision-register.md` (itens 16–20) |
| Tracker | `website/progress/tasks.json` (U33 Onda C, U34–U36 ondas D–F) |
| Intercâmbio (import/export) | `website/docs/12-interchange/import-export-game-ready.md` |
| Plano de integração das branches | `MERGE-PLAN.md` (raiz) |
