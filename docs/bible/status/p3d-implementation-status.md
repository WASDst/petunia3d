# Status de implementação das especificações P3D

> Artefato operacional. **Não é uma segunda autoridade documental**: a especificação
> canônica de cada P3D é a página correspondente no caderno (`specs/`). Este índice
> preserva a anotação de wave/status que existia no espelho antigo e deve ser
> reconciliado com evidência executável antes de qualquer promoção para `DONE`
> (Livro Vivo, readiness e taxonomia de estados).

> **Revalidação parcial (2026-09-30):** as linhas abaixo continuam sendo as anotações de 11/09 e **não** foram revalidadas. Onde a auditoria posterior encontrou defeitos, a evidência está nas matrizes de lacunas — P3D-062/064/132 (Paint/UV) em [`paint-uv-gap-matrix`](../../development/paint-uv-gap-matrix.md); P3D-090/091 (atalhos) em [`keymap-shell-gap-matrix`](../../development/keymap-shell-gap-matrix.md). Nenhuma linha foi promovida ou rebaixada aqui sem evidência executável.

| P3D | Wave | Status anotado (2026-09-11, não revalidado) | Contrato anotado | Página canônica |
| :--- | :--- | :--- | :--- | :--- |
| `001` | Wave 2 | `COMPLIANT (Sistema de Projetos .petunia Postcard)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-001-sistema-de-projetos.md) |
| `002` | Wave 2 | `COMPLIANT (Autosave e Recuperação Atômica)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-002-autosave-e-recuperacao.md) |
| `003` | Wave 2 | `COMPLIANT (Biblioteca de Modelos por Projeto)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-003-biblioteca-de-modelos-por-projeto.md) |
| `004` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-004-viewport-3d.md) |
| `005` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-005-navegacao-3d.md) |
| `006` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-006-vistas-ortograficas.md) |
| `007` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-007-navigation-gizmo.md) |
| `008` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-008-frame-selected-frame-all.md) |
| `009` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-009-grid-3d.md) |
| `010` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-010-overlays.md) |
| `011` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-011-x-ray.md) |
| `012` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-012-shading-modes.md) |
| `013` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-013-imagens-de-referencia.md) |
| `014` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-014-referencias-ortograficas.md) |
| `015` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-015-object-mode-modelo-unificado-de-selecao.md) |
| `016` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-016-edit-mode-absorvido-por-p3d-015.md) |
| `017` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-017-vertex-select.md) |
| `018` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-018-edge-select.md) |
| `019` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-019-face-select.md) |
| `020` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-020-object-selection.md) |
| `021` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-021-move.md) |
| `022` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-022-rotate.md) |
| `023` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-023-scale.md) |
| `024` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-024-universal-transform.md) |
| `025` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-025-axis-constraints.md) |
| `026` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-026-transform-orientation.md) |
| `027` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-027-pivot-system.md) |
| `028` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-028-add-primitives.md) |
| `029` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-029-extrude.md) |
| `030` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-030-multi-extrude.md) |
| `031` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-031-inset.md) |
| `032` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-032-bevel.md) |
| `033` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-033-knife-cut.md) |
| `034` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-034-loop-cut.md) |
| `035` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-035-subdivide.md) |
| `036` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-036-merge.md) |
| `037` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-037-split-separate.md) |
| `038` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-038-mirror-simples.md) |
| `039` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-039-proportional-editing.md) |
| `040` | Wave 5 | `COMPLIANT (Selection Domain, Transform & Modeling Core)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-040-sistema-de-snap.md) |
| `041` | Wave 2 | `COMPLIANT (Assets & Primitives)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-041-undo-redo.md) |
| `042` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-042-asset-browser.md) |
| `043` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-043-busca-e-filtros-de-assets.md) |
| `044` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-044-drag-drop-de-assets.md) |
| `045` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-045-thumbnails-configuraveis.md) |
| `046` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-046-outliner.md) |
| `047` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-047-visibility-lock.md) |
| `048` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-048-inspector-properties.md) |
| `049` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-049-transform-inspector.md) |
| `068` | Wave 8 | `COMPLIANT (Import, Export & Delivery Pipeline)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-068-export-individual.md) |
| `069` | Wave 8 | `COMPLIANT (Import, Export & Delivery Pipeline)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-069-export-multiplo.md) |
| `070` | Wave 8 | `COMPLIANT (Import, Export & Delivery Pipeline)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-070-batch-export.md) |
| `071` | Wave 8 | `COMPLIANT (Import, Export & Delivery Pipeline)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-071-importadores-modulares.md) |
| `072` | Wave 8 | `COMPLIANT (Import, Export & Delivery Pipeline)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-072-exportadores-modulares.md) |
| `073` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-073-workspace-system.md) |
| `074` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-074-viewport-header-contextual.md) |
| `075` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-075-vertical-tool-toolbar.md) |
| `076` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-076-contextual-tool-shelf.md) |
| `077` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-077-menus-profissionais.md) |
| `078` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-078-paineis-retrateis-e-redimensionaveis.md) |
| `079` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-079-layout-responsivo-desktop.md) |
| `080` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-080-status-bar.md) |
| `081` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-081-command-palette.md) |
| `082` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-082-context-menus.md) |
| `083` | Wave 6 | `COMPLIANT (Scene, Assets, Outliner & Inspector)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-083-tool-properties.md) |
| `084` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-084-design-system-tokenizado.md) |
| `085` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-085-temas-customizados.md) |
| `086` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-086-icon-packs.md) |
| `087` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-087-packs-oficiais-de-icones.md) |
| `088` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-088-petunia-custom-icons.md) |
| `089` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-089-sistema-completo-de-traducoes.md) |
| `090` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-090-keybindings-personalizaveis.md) |
| `091` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-091-deteccao-de-conflitos.md) |
| `092` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-092-petunia-default.md) |
| `093` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-093-petunia-simple.md) |
| `094` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-094-petunia-notebook.md) |
| `095` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-095-blender-like.md) |
| `096` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-096-blender-like-notebook.md) |
| `097` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-097-maya-like.md) |
| `098` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-098-3ds-max-like.md) |
| `099` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-099-cinema-4d-like.md) |
| `100` | Wave 0 / Wave 1 | `COMPLIANT (Arquitetura Spine & Invariantes)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-100-command-system.md) |
| `101` | Wave 1 | `COMPLIANT (Tools Modulares & Intent)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-101-tools-modulares.md) |
| `102` | Wave 1 | `COMPLIANT (Core Agnóstico à UI)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-102-core-agnostico-a-ui.md) |
| `103` | Wave 1 | `COMPLIANT (Separação Core / Application / UI)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-103-separacao-core-application-ui.md) |
| `104` | Wave 1 | `COMPLIANT (Headless Readiness & CLI)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-104-headless-readiness.md) |
| `105` | Wave 1 | `COMPLIANT (Renderer Boundary WGPU / GL)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-105-renderer-desacoplado-da-ui.md) |
| `106` | Wave 0 / Wave 1 | `COMPLIANT (Arquitetura Spine & Invariantes)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-106-frontends-intercambiaveis.md) |
| `107` | Wave 0 / Wave 1 | `COMPLIANT (Arquitetura Spine & Invariantes)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-107-api-cross-language-ready.md) |
| `108` | Wave 1 | `COMPLIANT (IDs Estáveis Uuid)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-108-ids-estaveis.md) |
| `109` | Wave 1 | `COMPLIANT (Sincronização de Estado & EventBus)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-109-events-state-synchronization.md) |
| `114` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-114-tooltips-completos.md) |
| `115` | Wave 3 | `COMPLIANT (UI Infrastructure, Customization & Input)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-115-ajuda-contextual.md) |
| `122` | Wave 0 / Wave 1 | `COMPLIANT (Architecture Checks Automatizados)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-122-architecture-checks.md) |
| `123` | Wave 5 | `COMPLIANT (Testes de Ferramentas de Modelagem)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-123-testes-de-ferramentas.md) |
| `124` | Wave 8 | `COMPLIANT (Testes de Import/Export & Delivery Pipeline)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-124-testes-de-import-export.md) |
| `125` | Wave 4 | `COMPLIANT (Viewport, Navigation & Reference Workflow)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-125-asset-icon-caching.md) |
| `126` | Wave 0 | `COMPLIANT (Performance Baseline & PC Modesto)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-126-performance-para-pcs-modestos.md) |
| `127` | Transversal | `COMPLIANT (Invariantes de Filosofia do Projeto)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-127-no-remesh.md) |
| `128` | Transversal | `COMPLIANT (Invariantes de Filosofia do Projeto)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-128-sem-composicao-de-cenas-complexa.md) |
| `129` | Transversal | `COMPLIANT (Invariantes de Filosofia do Projeto)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-129-sem-login-obrigatorio.md) |
| `130` | Transversal | `COMPLIANT (Invariantes de Filosofia do Projeto)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-130-customizacao-sem-quebrar-core.md) |
| `131` | Wave 5 | `COMPLIANT (Modal Tool Feedback & Snapping)` | Verificado contra a suíte de testes automatizados e o código-fonte canônico. | [spec](../specs/p3d-131-modal-tool-feedback-system.md) |

## Specs em `SPEC DRAFT` — Animate procedural-first (2026-09-30)

Sem implementação; estados abaixo são de **especificação**, não de código. Visão, Gap Matrix `AN-01…AN-15` e fasing em [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md); decisão de shell em [ADR 006](../../architecture/adr/006-workspace-animate-pos-v1.md).

| P3D | Fase | Estado | Nota | Página canônica |
| :--- | :--- | :--- | :--- | :--- |
| `066` | F2 | `SPEC DRAFT` (emenda 2026-09-30) | Animate pós-V1; domínio existe, **sem UI no Slint** | [spec](../specs/p3d-066-animation-workspace.md) |
| `067` | F3 | `SPEC DRAFT` (emenda 2026-09-30) | exceções nomeadas: IK, Wiggle/Ragdoll, Layers | [spec](../specs/p3d-067-animacao-simples.md) |
| `169` | F0 | `SPEC DRAFT` | Rig Roles & IK Foundation | [spec](../specs/p3d-169-rig-roles-ik-foundation.md) |
| `170` | F1 | `SPEC DRAFT` | Procedural Motion Generators | [spec](../specs/p3d-170-procedural-motion-generators.md) |
| `171` | F3 | `SPEC DRAFT` | Ghosts & Trajectories | [spec](../specs/p3d-171-animation-ghosts-trajectories.md) |
| `172` | F4 | `SPEC DRAFT` | Reference Image Sequence (batch de imagens) | [spec](../specs/p3d-172-reference-image-sequence.md) |
| `173` | F5 | `SPEC DRAFT` | Secondary Motion & Simple Ragdoll | [spec](../specs/p3d-173-secondary-motion-simple-ragdoll.md) |
| `174` | F4 | `SPEC DRAFT` | Layered Animation | [spec](../specs/p3d-174-layered-animation.md) |
