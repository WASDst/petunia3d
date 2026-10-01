# Feature svg: SVG como decalque e como fonte de perfil DRAW

Domínio puro em `crates/project/src/svg.rs` (sem UI). Specs: P3D-133 (decalque),
P3D-156/158 (spline core, anexo à superfície), fundações 42-44 (anti-bloat).

## O que faz

1. **Decalque**: `DecalLayer::from_svg` rasteriza o SVG em `Canvas` RGBA
   **straight** (não pré-multiplicado) e guarda o texto em `source_svg`.
   `DecalLayer::rerasterize(max_px)` refaz a imagem em outra resolução.
2. **Perfil/spline DRAW**: `svg_subpaths` / `svg_to_splines` extraem a geometria
   dos caminhos (formas incluídas) como nós Bézier com alças relativas
   (convenção de `SplinePoint`), centrados, com y para cima, escala uniforme.

## API pública

- `MAX_SVG_BYTES` (2 MiB), `MAX_RASTER_PX` (1024), `MAX_SVG_NODES` (20 000)
- `enum SvgError { TooLarge, Parse(String), Empty, ZeroSize, Render }`
- `svg_info(&str) -> Result<SvgInfo { width, height }, SvgError>`
- `rasterize_svg(&str, max_px) -> Result<Canvas, SvgError>`
- `svg_subpaths(&str, fit_size) -> Result<Vec<SvgSubpath>, SvgError>`
- `svg_to_splines(&str, fit_size) -> Result<Vec<SplineResource>, SvgError>`
- `DecalLayer { source_svg: Option<String> }` (`#[serde(default)]`),
  `DecalLayer::from_svg(svg, max_px, center_uv, scale_uv_width)`,
  `DecalLayer::rerasterize(&mut self, max_px) -> Result<bool, SvgError>`

Tudo reexportado em `petunia_project`.

## Decisões e limites

- Dependência: `resvg 0.48.1` com `default-features = false` (já estava no
  `Cargo.lock` via Slint; só ganhamos a aresta do `petunia_project`). Licenças
  Apache-2.0/MIT, cobertas pelo `deny.toml`. Sem `text`, sem `svgz`, sem
  `raster-images`.
- **`<text>` é ignorado**; `<image>` é ignorado (nem arquivo, nem rede, nem
  data URL); SVGZ não é aceito; `<!ENTITY` é recusado.
- Entrada acima de 2 MiB falha com `TooLarge` antes de parsear. `rasterize_svg`
  limita o maior lado a `min(max_px, 1024)` (mínimo 1 px); largura/altura
  gigantes no documento são só escala.
- `Empty` = texto vazio ou documento sem nada desenhável (inclui só `<text>`).
  `ZeroSize` = documento sem tamanho válido, `fit_size` não positivo/não finito
  ou geometria sem extensão. Estouro de `MAX_SVG_NODES` reutiliza `TooLarge`.
- Clip paths, máscaras, filtros e dashes **não** entram na geometria (só na
  rasterização). Caminhos ocultos (`visibility`) são ignorados; preenchimento e
  traço não são distinguidos: o contorno do caminho é a spline.
- Caixa envolvente usa os extremos reais das curvas (não os pontos de controle).
- Nós consecutivos coincidentes e **sem curvatura** são fundidos; uma curva que
  volta ao mesmo ponto com alças é mantida. Subcaminhos com menos de 2 nós são
  descartados; `closed` na spline só vale com 3+ nós (`validate_authoring`).
- As splines são planas (z = 0) e recebem `Uuid` novos a cada chamada.
- Projetos antigos sem `source_svg` abrem com `None`.

## Como testar

```bash
export CARGO_TARGET_DIR=/home/raillen/.cache/p3d-agent-target-wt
cargo test -p petunia_project --lib svg
cargo test -p petunia_project --lib paint_layers
```
