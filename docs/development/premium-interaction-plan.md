# Interação premium — plano e evidências

Status: em implementação; os critérios do `GAUNTLET_PREMIUM_PROMPT.md` continuam integrais.
Referência visual e de interação: sistema Petunia (Livro Vivo 23, 24, 36) e capítulo 46 (ADR 007, 2026-09-29). Histórico: até essa data a referência canônica era [`docs/image-references/Blender.svg`](../../docs/image-references/Blender.svg) (catálogo de 268 elementos extraídos em [`docs/image-references/extracted/`](../../image-references/extracted/)).

## Auditoria inicial — 2026-09-12

O runtime não tem sessão modal, gizmo manipulável nem hover de componentes.
Extrude, Inset, Bevel e Push/Pull mutam no `on_activate`; o histórico não
permite cancelar uma prévia. Picking de vértices/arestas usa tolerância em
unidades de mundo e fallback sem limite em pixels. A documentação histórica
de convergência não comprova o novo padrão premium.

## Ordem de implementação

1. Transação modal em `core`: snapshot possuído pela sessão; prévias sempre
   derivadas do original; confirmação com um checkpoint; cancelamento exato.
   Adaptadores de ferramentas apenas solicitam início. Testes de cancelamento,
   repetição de prévias, undo/redo e entradas não finitas.
2. Integração `ui/app`: mouse, números, eixos/planos, snapping, HUD e atalhos;
   impedir que painéis/comandos alterem o projeto durante uma sessão.
3. Picking compartilhado com oclusão e tolerância em pixels, hover e gizmos.
   Testes de profundidade, perspectiva/ortográfica, restrições e eventos egui.
4. Corrigir invariantes geométricas identificadas em extrusão; depois ampliar
   inset, bevel segmentado, loop cut, knife e slice interativos.
5. Pintura contínua e navegação, validação hostil e revisão independente.
6. Executar testes workspace, clippy estrito, fmt e smoke; medir caminho dourado
   e desempenho gráfico antes de declarar convergência.

## Limites e riscos

Não alterar o formato persistido nem adicionar dependências. Sessões são estado
efêmero do editor, fora de `.petunia`. `core` permanece independente dos módulos
concretos. `ui` traduz eventos; `mesh` implementa geometria. Snapshot tem custo
proporcional à malha/projeto; 60 FPS precisa de medição, não de inferência.

O workspace real contém 14 crates internos e o pacote executável raiz. O número
13 no prompt e na documentação anterior é divergente do manifesto existente.

## Documentação afetada

Atualizar manual, arquitetura, relatório Gauntlet, estado do projeto e inteligência
Prumo com resultados observados e lacunas explícitas. Auditorias independentes:
interação/gizmos, ferramentas, renderização, topologia e arquitetura/segurança.

## Resultado da primeira rodada

Implementados e integrados: sessões modais para G/R/S/E/I/P/Ctrl+B, HUD,
restrições globais/eixos/planos, números e snapping; gizmos; hover e picking;
loop cut com preview e slide; knife por segmentos na mesma face; slice por
arrasto; pintura de vértices com undo por traço; câmera e atalhos revisados.

As revisões encontraram e corrigiram falhas adicionais em cancelamento, redraw,
ordem da atualização wgpu, geometria da extrusão e buffers de entrada da faca.
O bootstrap gráfico passa a propagar erros recuperáveis. Novos testes exercitam
transações de domínio e eventos egui sem simular resultados geométricos.

## Critérios premium ainda não comprovados

- Caminho dourado com artista em menos de dois minutos; benchmark de 60 FPS
  durante manipulação e 0 FPS ocioso no build final, em GL e wgpu.
- Picking com oclusão entre assets e modo objeto com seleção de qualquer asset.
- Gizmos medidos em uso real, incluindo câmera quase paralela ao eixo/plano,
  pivô personalizável e alternância entre eixos locais/globais.
- Extrusão individual/Alt+E, inset métrico com proteção de auto-interseção,
  bevel multiaresta arredondado com scroll de segmentos.
- Knife atravessando várias faces sem cliques intermediários; caps complexas
  com furos; push/pull volumétrico universal por arrasto direto de face.
- Pintura de textura UV no viewport, pincel conformado a superfícies curvas,
  balde contextual de face e ferramentas UV de um clique.
- ZERO PANIC completo em todas as falhas de recursos de GPU e limites de
  arquivos externos; medição de AA, shading e desempenho dos overlays.

As notas técnicas provisórias estão em `docs/GAUNTLET.md`. Nenhum critério foi
rebaixado: esta rodada entrega funções utilizáveis, mas não encerra a missão
premium nem valida a conclusão histórica de qualidade máxima.

## Achado da captura gráfica

A primeira captura real, após os testes sem GPU, revelou que GL e wgpu ainda
mapeavam NDC para a janela inteira, enquanto UI/picking usavam o painel central.
Foi corrigido com `core::viewport::PhysicalViewport`: bordas arredondadas em
pixels, conversão top-left→bottom-left para GL e viewport/scissor nos dois
backends. O clear GL desativa o scissor herdado de egui antes de limpar o frame.
Testes cobrem deslocamento de painéis, DPI e resize; a captura anterior fica
preservada em `.prumo/history/premium/startup-gl-before-viewport-fix.png`.

## Evidência automatizada final

- `cargo fmt --all -- --check`: passou.
- `cargo test --workspace`: **128 testes passaram**, incluindo domínio,
  entrada egui, câmera/picking e contrato de viewport físico.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou sem avisos.
- Logs completos: `.prumo/history/premium/tests.log` e `clippy.log`.

Nenhuma dependência externa foi adicionada. O grafo de 14 crates internos mais
executável raiz foi conferido acíclico. O teste de smoke e a captura GL do binário final estão registrados abaixo.


- `cargo run -- --smoke-test`: **SMOKE OK**, saída 0; log em
  `.prumo/history/premium/smoke.log`.
- Captura real OpenGL: saída 0 em Mesa 26.1.7 / Intel HD Graphics 4000,
  contexto 4.2 Core. O retângulo central contém e centraliza o cubo após a
  correção. Captura final: `.prumo/history/premium/startup-gl.png`;
  diagnóstico: `startup-gl.log` no mesmo diretório.
- O gerenciador de janelas entregou framebuffer 678×739 apesar da solicitação
  1280×800. Essa captura também mostra dívida visual em janela estreita:
  textos do rodapé sobrepostos e ícones com fallback. Não é uma certificação
  de acabamento premium, nem teste manual de arrasto ou medição de FPS.

Os gates automatizados estão verdes; a missão premium continua aberta nos
critérios listados acima. Não houve commit, publicação ou alteração de dependências.
