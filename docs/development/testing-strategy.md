# Estratégia de Testes e Garantia de Qualidade — Petunia3D

Este documento codifica o **ciclo rigoroso de testes, quality gates e conformance** para o Petunia3D. Nenhum commit é mesclado sem passar por todas as barreiras de verificação.

---

## 1. Test Levels (Níveis de Teste)

1. **Testes Unitários de Malha e Operações (`crates/mesh`)**:
   - Geometria computacional determinística: adição de vértices, indexação de faces, subdivisão, extrusão, triangulação e projeção UV.
   - Primitivas padrão: Cubo, Cilindro, Esfera UV, Ico-esfera, Cone, Plano e Toro com topologia válida e orientações normais corretas.
2. **Testes de Persistência e Limites de Confiança (`crates/project`)**:
   - Testes com entradas hostis: arquivos OBJ com índices fora de limites (OOB), vértices com NaN/infinito, arquivos `.petunia` corrompidos e sem assets.
   - Validação da integridade transacional de serialização binária e exportação glTF 2.0 / GLB.
3. **Testes de Undo/Redo e Comandos (`crates/commands`)**:
   - Gravação de snapshots, reversão exata de estado de malha e verificação de integridade pós-desfazer/refazer.
4. **Testes de Conformance Gráfica e Shaders (`crates/render`, `crates/render-gl`, `crates/render-wgpu`)**:
   - Validação de shaders GLSL 330 contra hardware de referência.
   - Testes de render-on-demand: garantia de que estados sem eventos (`idle`) geram zero redesenhos de frame na GPU.

---

## 2. Quality Gates (Portais de Qualidade)

Para aprovação de qualquer alteração no repositório:

| Gate | Comando | Critério de Aceitação |
|------|---------|------------------------|
| **Testes da Workspace** | `cargo test --workspace` | 100% dos testes aprovados; a contagem mínima é a do build atual, nunca um número fixo no documento. |
| **Check de Compilação** | `cargo check` | Sem erros em todo o workspace. |
| **Linter Estático** | `cargo clippy --workspace --all-targets -- -D warnings` | 0 avisos (warnings são erro). |
| **Formatação Canônica** | `cargo fmt --all --check` | 100% de conformidade com o `rustfmt`. |
| **Trust Boundary Sanitization** | `cargo test -p petunia_project -p petunia_mesh` | Rejeição sem pânico de malhas degeneradas e arquivos malformados. |
| **Caderno canônico** | `cargo run -p xtask -- bible-check` | Completude do Livro Vivo, links, vocabulário de usuário e site congelado. |
| **Drift de referências** | `cargo run -p xtask -- docs-generate --check` | CommandIds, keybinds, IconIds, TextIds, ThemeTokens e changelog sem drift. |
| **Mapa de UI** | `cargo run -p xtask -- ui-check` | `docs/public/ui-map.json` coerente com o código (ids, arquivos, símbolos, aciclicidade). |
| **Architecture checks** | `cargo run -p xtask -- arch-check` | Auditoria arquitetural íntegra (P3D-122). |

### Gates de documentação e QA por feature (P3D-114–125)

Além dos gates acima, a feature só é `DONE` quando estes itens aplicáveis existem e
foram verificados:

| P3D | Item | Evidência exigida |
| :--- | :--- | :--- |
| 114 | Tooltips completos | texto/atalho em todo controle relevante, via `TextId` |
| 115 | Ajuda contextual `?` | tópico de ajuda alcançável do contexto atual |
| 116 | Website de documentação | publicável a partir do repositório (site congelado na fase atual) |
| 117 | Changelog vivo | `CHANGELOG.md` sincronizado com `docs/changelog/index.md` + migration notes |
| 118 | Screenshots atualizados | **captura real da implementação**, nunca mockup |
| 119 | Referência automática de tokens | catálogos gerados sem drift |
| 120 | Docs Check | `docs-check` + `bible-check` verdes |
| 121 | UI Regression Tests | testes de interação/foco e snapshots nos fluxos afetados |
| 122 | Architecture Checks | fronteiras de crate e dependências validadas |
| 123 | Testes de ferramentas | uma Tool nova entra com teste de comportamento e de undo |
| 124 | Testes de import/export | fixtures de conformance por formato (`gltf`/OBJ) |
| 125 | Asset/Icon caching | parse/rasterização cacheados, sem download em runtime |

---

## 3. Conformance Strategy (Estratégia de Conformidade)

- **Padrão gráfico**: a stack final é **Rust 2024 + egui + eframe + egui-wgpu + wgpu** (capítulos 27 e 36). O viewport é integrado por `egui-wgpu` (custom callback/RenderPass) e o backend OpenGL existe apenas como caminho de compatibilidade — ele **não** é o baseline contratual.
- **Vertical slice**: o recorte do capítulo 31 funciona como teste de conformance/integração da baseline; falha localizada corrige adapter/dependência/implementação, e só bloqueador estrutural comprovado dispara novo ADR.
- **Exportação Game-Ready**: O formato glTF 2.0 (`.glb`) deve ser aceito sem avisos pelo validador oficial Khronos glTF Validator e importável diretamente na Godot Engine 4.x e Unity.
- **Isolamento de Cfg/Plataforma**: Validação contínua do fallback de backend (`PETUNIA_BACKEND=gl` vs `PETUNIA_BACKEND=wgpu`).

---

## 4. Evidence Expectations (Expectativas de Evidências)

- **Relatórios Gauntlet**: Todas as mudanças estruturais relevantes devem atualizar o documento canônico `docs/GAUNTLET.md` com métricas empíricas:
  - Tempo de startup até o primeiro frame exibido (meta: $\le 500$ ms).
  - Pegada de memória RAM residente (VmRSS) em idle (meta: $\le 100$ MB).
  - Taxa de quadros estável em cena teste de modelagem ($\ge 60$ FPS em hardware modesto).
  - Número exato de testes executados e aprovados.
- **Logs Operacionais**: Em caso de falha de renderização, logs estruturados via `RUST_LOG=wgpu_hal=debug,petunia=debug` devem ser anexados à evidência da tarefa.
