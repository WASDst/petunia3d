# Remediação G2 — upload regional de textura no WGPU

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** fechamento de `PA-04` no frontend de produção Slint/WGPU
>
> **Validação:** executada somente após código, provas e documentação do conjunto

## 1. Autoridade e critérios

Este pacote fecha o caminho incremental iniciado no G1 conforme os contratos já
definidos pelo Livro Vivo:

- paint stroke transfere somente tiles/regiões modificados quando o backend
  permite atualização de subresource;
- revisões monotônicas evitam varrer pixels para descobrir mudanças conhecidas;
- buffers e handles GPU continuam exclusivos do renderer;
- caches e invalidações transitórias não entram no documento persistido;
- alteração de textura não reconstrói buffers de geometria;
- o bridge Slint traduz estado de sessão para o adapter sem tornar o domínio
  dependente de toolkit ou WGPU.

Referências normativas principais:

- `docs/bible/foundations/19-concorrencia-memoria-caches-performance.md`;
- `docs/bible/foundations/28-arquitetura-rust-cargo-crates.md`;
- `docs/bible/foundations/29-geometry-renderer-uv-painting-rust.md`;
- `docs/bible/foundations/34-arquitetura-modular-rust-safety.md`.

## 2. Implementation-vs-Spec Gap Matrix do G2

| ID | Estado anterior | Delta implementado | Estado após o pacote |
| :--- | :---: | :--- | :---: |
| PA-04 | `PARTIALLY_COMPLIANT` após G1 | `DirtyTiles` vira `TextureDirtyRect`, atravessa `RenderResources` e `PetuniaViewport`, e alimenta `queue.write_texture` por subregião sem FNV integral | `COMPLIANT` no frontend Slint/WGPU de produção |
| RV-PAINT-GPU | `FUNCTIONAL_BUT_DIFFERENT` | Fingerprints de geometria e textura foram separados; revisão de textura não invalida triangulação/VBO | `COMPLIANT` para a fronteira Paint→WGPU |
| ARC-PAINT-GPU | `PARTIALLY_COMPLIANT` | Invalidação é estado transitório do runtime; documento não contém handle, buffer ou estado WGPU | `COMPLIANT` |
| LEGACY-GL | `PARTIALLY_COMPLIANT` | OpenGL legado preserva cache-hit geométrico, mas ainda detecta conteúdo por FNV e reenvia o canvas integral | `OBSOLETE` para expansão; dívida aceita na superfície `--legacy-egui` |

O estado `COMPLIANT` de `PA-04` é deliberadamente restrito ao frontend oficial.
O AGENTS §0.1 proíbe expandir o produto no egui legado; migrar o protocolo de
dirty regions para esse caminho aumentaria superfície obsoleta sem beneficiar o
binário padrão.

## 3. Arquitetura entregue

### 3.1 Invalidação transitória e neutra

`TextureUpdate` identifica o asset, dimensões do canvas e uma lista opcional de
`TextureDirtyRect`. `None` representa upload integral. O tipo vive no Core como
mensagem transitória de runtime, sem serialização e sem qualquer conceito de
texture handle, queue, bind group ou backend.

`RenderResources` agrega atualizações por asset. Regiões são:

1. recortadas às dimensões válidas;
2. combinadas horizontal e verticalmente quando contíguas ou sobrepostas;
3. promovidas a atualização integral quando cobrem todo o canvas;
4. dominadas por uma invalidação integral já pendente.

Isso limita chamadas por frame sem ampliar a área transferida. Mudança de
dimensão também conserva fallback integral, pois exige recriar o recurso GPU.

### 3.2 Fronteira Slint sem vazamento de WGPU

`PetuniaViewport::queue_texture_updates()` transporta as mensagens antes do
frame. O backend de software pode descartá-las porque lê o documento diretamente;
o `WgpuViewport` as encaminha ao renderer. A fila é drenada uma vez pelo bridge,
impedindo replay acidental no frame seguinte.

Esse contrato mantém:

- domínio independente de Slint;
- adapter independente da persistência;
- renderer sem acesso mutável ao documento;
- troca/fallback de backend segura, pois a criação de um slot sempre faz upload
  integral inicial.

### 3.3 Fingerprints separados

`SceneFingerprint` passa a expor `mesh`, `textures` e `refs_layout`. A revisão e
os pixels de textura saem do hash de geometria. Dimensões e associação do recurso
continuam no fingerprint de mesh porque alteram ranges e bindings.

O fingerprint de textura usa `texture_revision` e metadados do recurso em
O(assets), sem ler pixels. Projetos antigos com revisão zero continuam corretos:
a primeira criação do slot sempre faz upload integral. Mutação direta posterior
sem publicar revisão é rejeitada pelo contrato de domínio, em vez de reintroduzir
uma varredura O(n) por frame. Assim, câmera e paint não causam triangulação,
alocação de vertices, recriação de buffers ou hash integral do canvas.

### 3.4 Upload regional e fallback conservador

Para cada retângulo, o renderer empacota somente suas linhas RGBA em um scratch
reutilizado entre frames e chama `queue.write_texture` com `origin` e `extent`
regionais. Não há FNV do canvas no caminho WGPU de asset texture.

Upload integral permanece correto e limitado a:

- primeira criação do slot;
- resize/recriação do canvas;
- alteração genérica sem dirty regions declaradas;
- cobertura agregada de todo o canvas.

Invalidações integrais incluem canvas próprio do asset e albedo herdado do
material. Slots invisíveis ou inexistentes não exigem trabalho imediato; se
voltarem a ser necessários, sua criação faz upload integral.

Invalidações regionais também alcançam outros assets que herdam o mesmo albedo
compartilhado e não possuem textura própria. Isso evita que uma instância mostre
pixels novos enquanto outra continue com o slot GPU anterior.

## 4. Telemetria e provas

O WGPU expõe contadores monotônicos de diagnóstico:

- chamadas de upload;
- bytes enviados;
- uploads integrais;
- uploads parciais;
- rebuilds geométricos já existentes.

As provas automatizadas cobrem:

- clipping, deduplicação, união de regiões e dominância do upload integral;
- publicação transitória e incremento único da revisão de textura;
- propagação regional aos consumidores de um material compartilhado;
- revisão de textura alterar `textures`, mas não `mesh`, e mutação não publicada
  permanecer invisível por contrato;
- batch de Paint publicar regiões correspondentes aos dirty tiles;
- upload WGPU de uma região 4 × 3 enviar exatamente 48 bytes em uma chamada;
- a mesma edição não incrementar rebuilds de geometria nem uploads integrais;
- hosts sem adaptador WGPU pularem somente a prova GPU, mantendo as provas de
  domínio executáveis.

## 5. Limites conscientes

Este gate não muda o schema persistente e não unifica `Asset.texture` com
`Material.albedo_texture`. O G1 já reduz a sincronização entre ambos às linhas
sujas, mas os dois canvases ainda ocupam memória própria. Uma migração para
recurso compartilhado exige decisão de ownership/schema e não foi acoplada à
correção de upload.

Também permanecem fora deste gate:

- profiling estatístico em GPUs integradas e discretas representativas;
- captura RenderDoc para medir custo de driver e sincronização;
- mipmaps e compressão de textura;
- caminho OpenGL/egui legado, que continua conservador e integral;
- upload incremental do canvas 2D apresentado pelo próprio Slint.

O próximo conjunto prioritário pode atacar `IN-01`/catálogo-keymap ou iniciar a
fundação `P3D-161` de Spline Core. Hair não deve avançar para superfície antes de
Spline e `P3D-158` SurfaceAttachment existirem como recursos headless.

## 6. Gates de encerramento

Os testes são executados somente depois que código, provas e documentação deste
conjunto estão completos, conforme solicitado para o roadmap.

| Gate | Resultado |
| :--- | :---: |
| `cargo fmt --all -- --check` | PASSOU |
| `cargo check --workspace` | PASSOU |
| 665 testes de Core/Paint/Slint/WGPU e regressão Project/UV | PASSARAM |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSOU |
| `cargo run -p xtask -- docs-check` | PASSOU |
| `cargo run -p xtask -- bible-check` | PASSOU |
| `cargo run -p xtask -- ui-guard --strict` | PASSOU |

Os comandos Cargo usaram `RUSTUP_TOOLCHAIN=1.98.1`, versão exigida por
`rust-toolchain.toml`. O `docs-check` preservou os 813 arquivos do site público
congelado e manteve como inventário não bloqueante os 357 símbolos públicos sem
nó no mapa. O `ui-guard` não encontrou vazamento de tipos egui; apenas listou os
cheiros preexistentes e confinados da crate legado.
