# Path Paint (domínio e bridge Slint)

Núcleo do Path Paint do workspace PAINT (P3D-158, P3D-160/161, P3D-062; fundações 43/44):
o usuário clica pontos sobre a superfície e o pincel é pintado ao longo do caminho
projetado na malha. Inclui linha reta (Shift+clique) e o filtro *pixel perfect*.
Integração Slint em `paint_surface_tools.rs`: ToolSession compartilhada, cliques/arrasto, preview, Enter aplica e Esc cancela. A paleta oferece Stroke, Ribbon e Repeated Stamp. As ações podem receber atalhos no perfil do usuário.

Código: `crates/module-paint/src/path_paint.rs` e `surface_commands.rs`.

## API pública (re-exportada por `petunia_module_paint`)

- `sample_path(nodes, closed, smooth, spacing, max_samples) -> Vec<Vec3>`:
  poligonal ou Catmull-Rom centrípeta reamostrada em passos iguais de comprimento
  de arco. Inclui o primeiro e (aberto) o último nó; fechado dá a volta com passo
  ajustado. NaN/inf e nós repetidos são descartados; `spacing` inválido vira 1/100
  do comprimento; nunca passa de `max_samples`.
- `SurfaceHit { face, position }`, `snap_to_surface(mesh, point, max_distance, face_allowed)`:
  ponto mais próximo nas faces permitidas (empate: menor face).
- `project_path_onto_surface(mesh, samples, max_distance, face_allowed) -> Vec<SurfaceHit>`:
  pré-processa a malha uma vez (caixas por face + grade uniforme), descarta
  amostras sem superfície e colapsa duplicatas consecutivas.
- `line_pixels(a, b)` (Bresenham inclusivo) e `pixel_perfect(points)` (remove o
  pixel do meio de cantos em L, estilo Aseprite).
- `PaintModule::paint_surface_path(state, nodes, closed, smooth, settings, isolate_selection) -> usize`
  e `PaintModule::paint_surface_line(state, a, b, settings, isolate_selection) -> usize`.
  Devolvem o número de dabs carimbados (0 se nada foi pintado). Os métodos são
  declarados em `path_paint.rs` (bloco `impl PaintModule`), para manter o `lib.rs`
  com apenas `mod` e re-exports.

## Comportamento de `paint_surface_path`

- Espaçamento: `size_px * spacing` (px de tela) convertido por
  `AppState::world_radius_for_px` (a mesma conversão do dab 3D), na menor
  profundidade do caminho para não abrir buracos.
- Um único traço: `begin_paint_stroke` ... `finish_paint_stroke(false)`, logo
  **uma entrada de Undo** e o buffer do traço limita a opacidade. Se o chamador
  já abriu um traço, ele é reaproveitado e não é finalizado aqui. Com modal ou
  preview ativo (traço não abre) nada é pintado.
- Restrição (seleção, `isolate_selection`, trava de pincel) resolvida uma vez e
  usada tanto ao colar as amostras quanto no carimbo (`paint_mesh_3d_batch_with_settings`).
- Oclusão não é tratada: quem chama escolhe os nós sobre a superfície visível.
- Alcance de colagem: `max(0,25 * diagonal da malha, 4 * raio de mundo do pincel)`.

## Limites

- Máximo de 8192 nós de entrada e 4096 dabs por caminho (`MAX_PATH_NODES`, `MAX_PATH_DABS`).
- `line_pixels` devolve vazio para retas com mais de 2^20 px de lado.
- Pincéis de forma (`Line/Rectangle/Ellipse`) não são aceitos (devolve 0).
- O retorno de dabs não inclui as cópias da simetria.
- Cada dab ainda percorre todas as faces (custo do motor 3D existente); não foi otimizado aqui.

## Persistência e transação

`PaintPathCmd` avalia todos os anchors antes de pintar. Salva uma spline com `SurfaceAttachment` nos pontos, preservando alvo/triângulo/barycentric/fingerprint. Mudança de topologia retorna erro e rollback; o draft nunca migra silenciosamente para outro objeto. A avaliação em lote calcula o fingerprint uma vez por alvo.

Stroke usa o brush ativo; Ribbon usa ponta quadrada e espaçamento contínuo sem alterar a preferência do brush. Repeated Stamp usa a imagem do decal selecionado, orientada pela tangente, em layer raster própria (até 256 stamps). Nenhum modo altera o decal fonte. Restrições de face/seleção e layer/asset lock são comuns ao motor. Caminho sem pixels elegíveis não cria layer/spline/Undo.

O shell usa caminho poligonal. A spline salva registra os anchors do commit; editar a spline não repinta automaticamente o raster. Alpha lock em layer nova transparente impede Projection/Stamps. Linhas Shift 2D e 3D são traços únicos; pixel perfect incremental remove cantos também na composição/simetria.

## Testes

Regressões existentes e novas (não executadas nesta rodada; gates adiados):
`cargo test -p petunia_module_paint --lib path_paint`, `surface_commands`, `projection`. Cobrem: espaçamento
igual (poligonal e círculo fechado, erro < 2%), inclusão de pontas, snap e
`face_allowed`, caminho sobre aresta de cubo (pinta as duas faces, nunca a oposta),
uma entrada de Undo por caminho, restrição por seleção e trava, pixel perfect em
escada, entradas degeneradas/NaN/enormes, determinismo e tempo (2000 amostras em
5k faces).

Path também coleta anchors por clique/arrasto no canvas UV, com preview UV sem ligar segmentos entre charts diferentes; o commit usa o mesmo caminho/restrições/Undo do modelo 3D. A interação Projection continua na viewport; clicks do canvas nesse modo não pintam um brush por engano.
