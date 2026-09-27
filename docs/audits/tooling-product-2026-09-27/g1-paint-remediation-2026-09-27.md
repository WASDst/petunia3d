# Remediação G1 — stroke e composição incremental de Paint

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** fechamento conjunto de `PA-01`, `PA-02` e `PA-03` de
> `findings.md`, com avanço parcial de `PA-04`
>
> **Validação:** executada somente após código, provas e documentação do conjunto

## 1. Autoridade e critérios

Este pacote aplica incrementalmente os contratos já definidos pelo Livro Vivo:

- strokes raster são uma transação e registram diffs por tile;
- a densidade do stroke decorre do descriptor do brush, não da frequência do
  dispositivo de input;
- composição e transferência devem ser proporcionais à região alterada;
- caches derivados não são autoridade do documento;
- zoom é responsabilidade da apresentação e não deve multiplicar o raster no
  CPU;
- o domínio continua independente de Slint e do adapter WGPU.

Referências normativas principais:

- `docs/bible/foundations/12-baseline-funcional-roadmap-escopo.md`;
- `docs/bible/foundations/14-workspace-paint-camadas-uv-integrado.md`;
- `docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`;
- `docs/bible/foundations/34-arquitetura-modular-rust-safety.md`;
- `docs/bible/foundations/44-pos-v1-surface-paint-toolbox.md`.

## 2. Implementation-vs-Spec Gap Matrix do G1

Os IDs abaixo seguem `findings.md`. A matriz geral do snapshot usa agrupamentos
de produto mais amplos; esta página evita reinterpretar silenciosamente os dois
catálogos.

| ID | Estado anterior | Delta implementado | Estado após o pacote |
| :--- | :---: | :--- | :---: |
| PA-01 | `PARTIALLY_COMPLIANT` | Uma amostra agrega dabs e simetrias; mutação produz `DirtyTiles`; compositor empresta a stack; albedo copia apenas linhas sujas | `PARTIALLY_COMPLIANT` avançado: amplificação integral por dab removida; canvases persistidos ainda não são um handle compartilhado |
| PA-02 | `BROKEN` | `StrokeSampler` acumula distância residual e alimenta os caminhos Slint 2D/3D com o `spacing` canônico | `COMPLIANT` no frontend de produção |
| PA-03 | `BROKEN` | Imagem publicada permanece na resolução nativa; zoom, recorte, nearest e grid são apresentação Slint/vetorial | `COMPLIANT` |
| PA-04 | `BROKEN` | `DirtyTiles` agora atravessa mutação, composição e espelho de material | `PARTIALLY_COMPLIANT`: o adapter WGPU ainda faz hash e upload do canvas completo |

## 3. Arquitetura entregue

### 3.1 StrokeSampler neutro e incremental

`petunia_core::StrokeSampler` mantém o último sample e a distância restante até
o próximo dab. Um segmento curto não força pintura no fim de cada evento; seu
residual é carregado para o segmento seguinte. Assim, o mesmo caminho reamostrado
em uma ou várias notificações produz posições equivalentes.

O Core não conhece ponteiro, viewport, Slint, UV ou ferramenta física. O bridge
escolhe o espaço:

- canvas 2D: pixels da textura;
- pintura 3D: pixels lógicos da viewport, seguidos de raycast e projeção UV.

O `BrushSettings::stroke_dabs()` de compatibilidade usa a mesma fundação, mas a
garantia completa entre eventos vem de uma instância persistente do sampler.
O frontend egui permanece legado e não recebeu expansão de superfície.

### 3.2 Mutação consolidada e DirtyTiles

O caminho raster passa a executar:

1. receber todos os dabs emitidos pelo sampler para o pointer event;
2. expandir as cópias de simetria sem duplicar o eixo central;
3. carimbar todos os pixels na camada ativa;
4. agregar, ordenar e deduplicar `DirtyTiles` de 32 × 32;
5. recompor uma vez;
6. publicar uma revisão de textura.

Simetria 2D agrega até quatro posições por dab. Simetria 3D agrega até oito hits,
projeta todos para UV e só então entra no mesmo batch raster. Vertex Paint também
recebe os hits em lote e publica uma revisão de cor por amostra.

Fill marca o canvas inteiro, pois seu alcance não pode ser representado pelo raio
do dab. Dabs fora das dimensões não criam tiles fantasmas.

### 3.3 Compositor e material sem clones integrais por dab

`composite_active_tiles()` deixa de clonar `PaintLayerStack` para satisfazer o
borrow checker: stack e cache composto são emprestados como campos disjuntos do
asset. Em seguida, quando o material já possui albedo compatível, somente os
intervalos RGBA das linhas cobertas pelos tiles são copiados.

O fallback integral permanece deliberadamente limitado a:

- primeira criação do albedo derivado;
- mudança de dimensões;
- composição completa de stack não tileável, como efeito Pixelate.

`Asset.paint_stack` continua canônico; `Asset.texture` e
`Material.albedo_texture` continuam caches derivados. Torná-los um único handle
persistente exige migração de schema e ownership de recursos, portanto não foi
misturado a este gate incremental.

### 3.4 Canvas Slint em resolução nativa

`render_paint_canvas()` aloca somente `canvas.w × canvas.h × 4` e copia cada byte
uma vez para o `SharedPixelBuffer`. Em um canvas 256² no zoom 16, o payload cai do
antigo buffer de aproximadamente 64 MiB para 256 KiB.

O item `Image` agora:

- escala e recorta o raster no quadro de 256 px;
- usa `image-rendering: pixelated`;
- mantém o zoom centrado;
- converte input do quadro para coordenadas do raster escalado;
- desenha a grade contextual como `Path`, sem alterar os pixels publicados;
- escala o overlay UV junto com o raster.

Isso também corrige o comportamento anterior no qual o CPU ampliava a imagem e
o Slint a reduzia novamente para o mesmo quadro, sem zoom visual efetivo.

## 4. Provas adicionadas

Foram adicionados testes para:

- equivalência de dabs entre um segmento longo e vários pointer events;
- carregamento de residual menor que um passo;
- rejeição de coordenadas não finitas sem corromper a sessão;
- batch 2D com simetria publicar exatamente uma revisão de textura;
- tiles retornarem ordenados, únicos e limitados ao canvas;
- cache composto e albedo derivado permanecerem byte a byte sincronizados;
- stroke 2D Slint rápido e segmentado produzirem a mesma textura;
- zoom 4× continuar publicando uma imagem na resolução nativa;
- grade vetorial existir apenas quando habilitada e visível.

## 5. Limites conscientes e próximo gate

Este pacote não declara o pipeline GPU incremental. `Renderer::sync_asset_textures`
ainda calcula FNV sobre todos os pixels e chama `queue.write_texture` para o
canvas completo quando a revisão visual muda. Fechar `PA-04` exige transportar
regiões sujas por uma fronteira transitória até o adapter WGPU e definir o
consumo/limpeza dessas regiões sem colocar estado de GPU no documento.

Também permanecem:

- dois canvases persistidos para textura do asset e albedo do material, embora a
  sincronização interativa agora seja regional;
- uma cópia nativa do canvas ao publicar uma nova `slint::Image`;
- caminhos legados egui baseados em `stroke_dabs()` por segmento;
- efeitos não tileáveis com recomposição integral correta por definição.

O próximo pacote de performance de Paint deve fechar `PA-04` com dirty regions
consumíveis pelo renderer e métricas de bytes enviados. Depois disso, o roadmap
pode avançar para coerência de catálogo/keymap ou fundações Shape-first/Hair sem
carregar este gargalo para novos geradores.

## 6. Gates de encerramento

Os testes são executados somente depois que código, testes e documentação deste
conjunto estão completos, conforme solicitado para o roadmap.

| Gate | Resultado |
| :--- | :---: |
| `cargo fmt --all -- --check` | PASSOU |
| `cargo check --workspace` | PASSOU |
| 548 testes de Core/Paint/Slint e regressão Project/UV | PASSARAM |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSOU |
| `cargo run -p xtask -- docs-check` | PASSOU |
| `cargo run -p xtask -- bible-check` | PASSOU |
| `cargo run -p xtask -- ui-guard --strict` | PASSOU |

Os comandos Cargo foram executados com `RUSTUP_TOOLCHAIN=1.98.1`, versão exigida
por `rust-toolchain.toml`; o ambiente da sessão sobrescrevia inicialmente o
toolchain para `stable` 1.97.1. O `docs-check` preservou os 813 arquivos do site
público congelado e reportou apenas o inventário não bloqueante de símbolos ainda
ausentes do mapa. O `ui-guard` manteve todos os tipos egui confinados e listou os
cheiros preexistentes da crate legado, sem introdução de nova superfície egui.
