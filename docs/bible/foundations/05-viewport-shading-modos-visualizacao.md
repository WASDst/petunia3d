# 05 — Viewport, Shading e Modos de Visualização

## Decisão principal

O Petunia3D terá quatro modos de visualização centrais, sempre acessíveis por um único controle compacto:

- **Textured** — textura/material aplicado; modo padrão para avaliação artística.
- **Solid** — forma sem textura, com iluminação simples; ideal para avaliar volumes e silhueta.
- **Wireframe** — arestas visíveis e transparência estrutural; ideal para alinhamento, topology e referências.
- **Silhouette / Reference** — modelo reduzido à silhueta, com referência controlável ao fundo; modo assinatura do Petunia para tracing e comparação visual.

## Overlays, não novos modos

Para evitar bloat, informações técnicas entram como overlays combináveis:

- Wire overlay sobre Solid/Textured.
- Triangulation preview.
- Vertex / edge / face selection highlights.
- Face orientation.
- UV checker / distortion preview.
- Reference opacity / x-ray.

## Flat e Smooth

Flat/Smooth não serão modos de viewport separados; serão propriedades de shading. O projeto deve suportar:

- Flat shading por padrão para estética low-poly.
- Smooth shading **suportado no Core V1 como propriedade secundária**, não como modo de viewport separado.
- Sharp edges / hard edges explícitos.
- Visualização simples de normals somente em modo avançado.

## Iluminação de trabalho

Manter extremamente simples:

- Studio light padrão previsível.
- Rotação rápida da luz.

Implementação (2026-09-30, [ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md) Onda 6):
a studio light acompanha a câmera por padrão (por cima do ombro de quem olha,
como no Plasticity e no Cinema 4D), de modo que a forma continua legível de
qualquer lado; a preferência "Luz de estúdio acompanha a câmera" volta para a
luz fixa no mundo. Girar a luz por arrasto continua pendente — ver
[`wave6-visual-accessibility-gap-matrix.md`](../../development/wave6-visual-accessibility-gap-matrix.md).
- **Unlit suportado na V1** como opção secundária para conferir textura pura, sem substituir os quatro modos-base.
- Sem sistema completo de renderização na área principal de modelagem.

## Princípio de UX

O usuário deve trocar a forma de enxergar o modelo sem mudar de workspace ou configurar dezenas de opções. O software adapta a visualização ao contexto: desenhar sobre referência favorece ortográfica + silhouette/reference; orbitar favorece perspective + solid/textured; editar topology ativa overlays relevantes.

## Aparência por workspace

Desde a revisão de 2026-09-29 ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)), os dois workspaces de criação usam os mesmos modos-base com defaults de overlay diferentes:

- **DRAW:** leitura de forma — iluminação de estúdio ou matcap, arestas de feição nítidas, plano de trabalho em destaque, regiões tingidas no hover e cotas visíveis.
- **POLY:** leitura de topologia — flat shading, todas as arestas finas com arestas de feição reforçadas, pontos visíveis no domínio Point, pré-seleção clara e componentes de trás ocultos fora do X-Ray.

Em ambos: imagem renderizada em pixels físicos com antisserrilhado, linhas de largura constante em pixels e pré-seleção do elemento sob o cursor antes do clique ([capítulo 45](45-pesquisa-interacao-modelagem-referencias.md)).

# Apresentação na UI

A apresentação dos modos de viewport deve seguir a linguagem de controles compactos documentada na análise do Figma, mas com semântica Petunia.

## Controle principal

Preferir um **segmented control único** para os quatro modos mutuamente exclusivos:

```
Wireframe | Solid | Textured | Silhouette/Reference
```

Não copiar `Rendered` do Blender.

## Opções secundárias

Lighting, Unlit, Wire Overlay, Triangulation, Face Orientation e demais overlays ficam em controles separados ou dropdown acoplado, evitando multiplicar modos principais.

## Estado

O modo ativo deve ser visualmente evidente por combinação de background/border/indicador, não apenas pela cor do ícone. O controle precisa também expor estado selecionado semanticamente ao toolkit de acessibilidade.

## Relação com a referência visual

Ver [22 — Referência de Interface: Análise do Figma Blender UI Redesign](22-referencia-interface-figma.md) para o componente `view3d_shading` observado e [25 — Biblioteca de Componentes e Contratos de Interação](25-biblioteca-componentes-interacao.md) para o contrato do View Mode Control.