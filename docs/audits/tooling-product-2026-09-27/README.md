# Petunia3D — Dossiê de ferramentas, produto e evolução técnica

> **Data do snapshot:** 2026-09-27  
> **Revisão Git final de referência:** `05b2ead` (`main`)
> **Escopo observado:** implementação Slint/WGPU publicada nessa revisão; a
> auditoria começou em `819ab77` e acompanhou o delta até o checkpoint final
> **Natureza:** auditoria operacional derivada; não substitui o Livro Vivo em
> `docs/bible/`  
> **Regra de execução:** corrigir incrementalmente; este dossiê não autoriza
> reescrita total nem reabertura da UI Baseline V1

> **Estado pós-auditoria:** os gates G0–G3 estão registrados separadamente. O G3
> eleva `HA-01` a `PARTIALLY_COMPLIANT` com a fundação headless de Spline Core;
> attachment, UI, Profile persistente, generators e Hair continuam pendentes.

## 1. Veredito executivo

O Petunia3D já possui uma base de editor 3D significativamente mais real do que
um protótipo visual: há operadores de malha testados, seleção por domínio,
operações paramétricas, perfis shape-first, pintura raster, pilha de camadas,
unwrap por `xatlas`, edição UV, renderer WGPU persistente, temas, preferências e
uma interface Slint extensa. Os pontos fortes estão no núcleo geométrico, na
separação do renderer e na quantidade de comportamento automatizado por testes.

O principal risco já não é "falta de funcionalidades". É a perda de coerência
entre várias implementações parcialmente sobrepostas:

1. a UI Slint executa regras de domínio e transações por caminhos próprios;
2. o `CommandDispatcher` também grava histórico e invalida o projeto;
3. alguns comandos repetem internamente essas mesmas responsabilidades;
4. pintura e viewport publicam cópias integrais onde o modelo já conhece regiões
   sujas;
5. os contratos visuais e de input prometem mais do que o renderer e o keymap
   realmente entregam;
6. uma superfície UV independente continua visível apesar da decisão canônica de
   mantê-la como utilitário dentro de PAINT;
7. a fundação de curvas/attachment necessária para Hair ainda não existe como
   recurso persistente e transacional.

O resultado é um produto funcional, porém com risco alto de regressões de undo,
latência crescente e comportamento divergente entre mouse, atalhos, menus e
workspaces. Adicionar Hair antes de estabilizar transações, revisões e curvas
compartilhadas ampliaria exatamente esses gargalos.

## 2. Decisões recomendadas

### Imediatas — integridade antes de expansão

1. **Tornar o dispatcher o único dono de transação.** Cada ação destrutiva deve
   produzir exatamente um snapshot/registro de undo e exatamente um conjunto de
   revisões de domínio.
2. **Corrigir checkpoints pós-mutação.** Operações de pilha de pintura,
   decal, fill 2D e delete/dissolve precisam capturar o estado anterior ou usar
   comandos reversíveis.
3. **Separar revisão de seleção, topologia, posições, UV, pintura e material.**
   `emit_mesh_changed()` não pode invalidar topologia e posições para qualquer
   alteração.
4. **Retirar afirmações visuais não implementadas.** A UI não deve anunciar PBR,
   roughness, ray tracing ou sombras enquanto o shader não os executar.
5. **Aplicar o congelamento UV.** Preservar algoritmos e editor UV, mas apresentá-
   los como `Preparar superfície` dentro de PAINT; não como workspace principal.

### Próximo ciclo — performance e consistência

1. Fazer o stroke produzir uma lista de tiles/retângulos sujos e compor/enviar à
   GPU uma vez por amostra consolidada, não uma vez por dab espelhado.
2. Eliminar o upscale CPU do canvas 2D; escalar a textura na apresentação.
3. Corrigir o cache de modifiers para usar revisões reais da fonte, não apenas
   contagens de vértices/faces.
4. Consolidar o registro de comandos da UI e o `CommandDispatcher` em um catálogo
   semântico único, com keymap como única tradução de input físico.
5. Dividir `SlintUiBridge`, `callbacks.rs` e `app.slint` por feature sem alterar o
   layout canônico.

### Fundação para Shape-first e Hair

1. Implementar `P3D-161` como recurso de spline serializável, versionado e
   independente de UI.
2. Implementar `P3D-158` para attachment por triângulo + coordenadas
   baricêntricas, com estado explícito `NeedsReattach`.
3. Reusar resampling por comprimento de arco e frames de transporte paralelo
   para Sweep, perfis e Hair Clump.
4. Entregar Hair em incrementos: Ribbon → taper/segments → attachment → mirror →
   draw-on-surface → clusters → bake. Não construir um clone de XGen/Houdini.

## 3. Achados críticos

| ID | Achado | Severidade | Estado |
| :--- | :--- | :---: | :---: |
| TX-01 | Checkpoints de várias ações são gravados após a mutação | P0 | BROKEN |
| TX-02 | UV stitch/relax e dissolve podem registrar undo no comando e no dispatcher | P0 | DUPLICATED |
| TX-03 | Estimativa genérica do histórico mede só o tamanho superficial de `Project` | P0 | BROKEN |
| PA-01 | Cada dab pode clonar/recompor canvas, stack e textura integral | P1 | PARTIALLY_COMPLIANT |
| PA-02 | Produção Slint ignora spacing canônico em strokes importantes | P1 | BROKEN |
| PA-03 | Canvas 2D faz upscale CPU e pode alocar cerca de 64 MiB em 256² × 16 | P1 | BROKEN |
| PA-04 | Upload de textura varre e envia o canvas inteiro | P1 | BROKEN |
| UI-01 | Dois catálogos de comandos e atalhos físicos fora do keymap | P1 | DUPLICATED |
| UI-02 | UI anuncia PBR/ray tracing/sombras ausentes no shader | P1 | FUNCTIONAL_BUT_DIFFERENT |
| UV-01 | Workspace UV independente contradiz o addendum canônico mais recente | P1 | OBSOLETE |
| RV-01 | Invalidação genérica marca topologia/posição para UV, seleção e pintura | P1 | BROKEN |
| CA-01 | Cache de modifiers não inclui revisão/conteúdo da malha de origem | P1 | BROKEN |
| AX-01 | F6/Shift+F6 e cobertura teclado/leitor de tela são incompletos | P1 | MISSING |
| HA-01 | Spline persistente e SurfaceAttachment compartilhados ainda não existem | P1 | MISSING |

O catálogo completo, com evidência física, impacto e correção sugerida, está em
[`findings.md`](./findings.md).

## 4. Scorecard de maturidade

As notas abaixo são indicadores direcionais do snapshot, não novos requisitos de
produto nem substitutos para os critérios do caderno.

| Área | Nota | Diagnóstico resumido |
| :--- | :---: | :--- |
| Núcleo geométrico MODEL | 7,5/10 | Bom conjunto de operadores e testes; representação autoral e sessões ainda fragmentadas |
| Workflow shape-first | 6,5/10 | Perfis, workplanes, depth, revolve e sweep existem; recurso paramétrico durável ainda incompleto |
| PAINT | 5,0/10 | Camadas e ferramentas funcionam; custo de cópia, spacing e undo impedem escala profissional |
| UV algorítmico | 6,5/10 | Unwrap, pins, seams, stitch, relax e density existem |
| UV como produto | 3,5/10 | Superfície independente está obsoleta e seleção compete com seleção de malha |
| Viewport/renderer | 5,0/10 | Cache de buffers e overlays são bons; shading prometido e shading real divergem |
| UI/UX Slint | 6,0/10 | Shell amplo e coerente visualmente; monólitos, hardcodes e caminhos duplicados elevam risco |
| Acessibilidade | 5,0/10 | Alto contraste, escala e reduced motion existem; navegação regional e semântica são incompletas |
| Input/keymap | 4,0/10 | Perfis existem, mas atalhos físicos e catálogo paralelo quebram a autoridade do keymap |
| Undo/transações | 3,0/10 | Infraestrutura existe, porém ownership duplicado e checkpoints tardios ameaçam integridade |
| Performance previsível | 4,5/10 | Render-on-demand é um avanço; pintura e avaliação de assets continuam O(n) com cópias integrais |
| Prontidão para Hair | 3,0/10 | Matemática de sweep/RMF ajuda; persistência, attachment, revisões e overlays de curva faltam |

## 5. Estrutura do dossiê

- [`gap-matrix.md`](./gap-matrix.md) — matriz Implementation-vs-Spec com o
  vocabulário obrigatório do projeto.
- [`findings.md`](./findings.md) — achados técnicos priorizados, reprodução,
  impacto e direção de correção.
- [`workspace-review.md`](./workspace-review.md) — avaliação de MODEL, PAINT,
  UV, viewport, GUI, configurações e acessibilidade.
- [`market-benchmark.md`](./market-benchmark.md) — comparação com documentação
  oficial de Blender, Maya, Substance 3D Painter, Houdini, XGen e ZBrush.
- [`roadmap-shape-first-hair.md`](./roadmap-shape-first-hair.md) — arquitetura e
  sequência recomendada para shape-first e Hair.
- [`verification.md`](./verification.md) — método, comandos, resultados e limites
  da auditoria.
- [`g0-remediation-2026-09-27.md`](./g0-remediation-2026-09-27.md) — fechamento
  do primeiro conjunto prioritário: transações, revisões, histórico e cache de
  modifiers.
- [`g1-paint-remediation-2026-09-27.md`](./g1-paint-remediation-2026-09-27.md) —
  remediação do stroke, batch simétrico, composição por tiles e canvas 2D nativo.
- [`g2-paint-gpu-remediation-2026-09-27.md`](./g2-paint-gpu-remediation-2026-09-27.md) —
  fechamento do upload regional Paint→Slint/WGPU, sem rebuild geométrico.
- [`g3-spline-core-remediation-2026-09-27.md`](./g3-spline-core-remediation-2026-09-27.md) —
  fundação persistente/transacional de splines e frames compartilhados com Sweep.

## 6. Fontes canônicas principais

- [`docs/bible/foundations/02-workflow-modelagem-shape-first.md`](../../bible/foundations/02-workflow-modelagem-shape-first.md)
- [`docs/bible/foundations/05-viewport-shading-modos-visualizacao.md`](../../bible/foundations/05-viewport-shading-modos-visualizacao.md)
- [`docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`](../../bible/foundations/19-concorrencia-memoria-caches-performance.md)
- [`docs/bible/foundations/34-arquitetura-modular-rust-safety.md`](../../bible/foundations/34-arquitetura-modular-rust-safety.md)
- [`docs/bible/foundations/36-ui-baseline-temas-plugin-panels.md`](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
- [`docs/bible/foundations/38-pos-v1-low-poly-hair-mesh-morphs-character.md`](../../bible/foundations/38-pos-v1-low-poly-hair-mesh-morphs-character.md)
- [`docs/bible/specs/p3d-158-surface-attachment-foundation.md`](../../bible/specs/p3d-158-surface-attachment-foundation.md)
- [`docs/bible/specs/p3d-161-spline-core.md`](../../bible/specs/p3d-161-spline-core.md)

## 7. Limites e regra de interpretação

Esta auditoria combina leitura estática, testes unitários e validação WGPU
headless. Não inclui sessão formal de usabilidade com participantes, medição em
GPU integrada/discreta representativa, captura de frame com RenderDoc nem
benchmark estatístico de projetos grandes. As conclusões de complexidade,
alocação e cópia são evidências de código; valores de tempo de frame devem ser
confirmados com profiling antes de estabelecer metas numéricas.

O benchmark de mercado identifica padrões úteis, não requisitos. Quando uma
prática externa conflita com o Livro Vivo, prevalece o Livro Vivo. Em especial,
o Petunia3D não deve copiar docking irrestrito, o modo Rendered do Blender nem a
complexidade procedural de Houdini/XGen.
