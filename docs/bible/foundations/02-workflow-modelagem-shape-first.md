# 02 — Workflow de Modelagem Shape-First

# Dois caminhos de criação igualmente válidos

O Petunia3D não é exclusivamente profile-based. O usuário escolhe o caminho mais simples para o objeto.

```mermaid
flowchart LR
    A["Draw / Profile"] --> C["Extrude / Push-Pull"]
    B["3D Primitive"] --> D["Direct Edit"]
    C --> E["Polygon Mesh"]
    D --> E
    E --> F["Face / Edge / Point editing"]
```

# Draw / Profile

## V1

- Line
- Polyline / Polygon Pen
- Rectangle
- Circle com sides explícitos
- Polygon
- Arc com segments explícitos

**Star não faz parte do Core V1.** Pode ser adicionada posteriormente como conveniência V1.x ou por plugin, sem aumentar a superfície inicial de criação.

A Pen Tool inicial gera linhas poligonais. Bézier verdadeira fica para uma evolução posterior, evitando que o projeto dependa de um sistema completo de curvas paramétricas. Quando a curva livre entrar no workspace DRAW, a direção preferida é interpolar os pontos clicados sem alças (κ-curves, [capítulo 46](46-pesquisa-interacao-modelagem-referencias.md)), sempre discretizada com contagem de segmentos explícita.

O desenho do Petunia é **planar 3D modeling**, não desenho 2D ambíguo: cada Profile está associado a um Work Plane conhecido no espaço 3D. O sistema conhece origem, eixos do plano, normal, posição e contexto de operação. Isso elimina a necessidade de inferir livremente onde um stroke existe no espaço.

### Interação de perfis no DRAW — revisão de 2026-10-03

Escolher Rectangle ou Circle arma uma ferramenta persistente, sem criar um
perfil imediatamente. Rectangle usa primeiro canto → canto oposto; Circle usa
centro → raio. Arrastar e clicar-mover-clicar seguem a gramática da constituição
11, com prévia sem mutação do documento, confirmação transacional e Esc.

Perfis são editáveis antes de qualquer volume: Select arrasta o corpo ou os
pontos/alças; Move, Rotate e Scale operam no perfil selecionado. Opções refletem
a forma selecionada (dimensões e segmentos quando aplicáveis). Editar livremente
um nó pode descaracterizar uma primitiva e desativar sua regeneração dimensional.

Auto, Ground, Face e View são compartilhados entre as ferramentas 2D. Face usa
a face selecionada ou aguarda um clique na viewport; não substitui uma escolha
sem face por Ground. Trocar explicitamente o plano de um perfil selecionado
preserva sua identidade e geometria local e permite Undo. Iniciar/cancelar uma
forma preserva o frame de um plano travado. A câmera só se alinha por comando.

## Profile → Volume

Ao fechar uma forma válida, o software mostra preenchimento e uma **Depth Handle**. Arrastar a alça — ou arrastar em qualquer lugar com a ferramenta Push/Pull — gera a extrusão imediatamente, com valor digitável (constituição 11).

No workspace DRAW, regiões fechadas (inclusive as formadas por perfis que se cruzam no mesmo plano) são destacadas no hover e podem ser empurradas diretamente. Sobre uma face existente, puxar para fora soma volume e empurrar para dentro corta, por operação topológica local (imprint + extrusão).

## Low-poly durante a criação

Circle, Arc, Cylinder, Sphere e Bevel expõem densidade geométrica como parte do estilo. Presets possíveis: Very Low, Low, Medium e Custom.

# Primitivas volumétricas

Primitivas são essenciais porque frequentemente são o caminho mais rápido.

## Conjunto Core V1

- Box / Cube
- Plane
- Cylinder
- Cone
- Sphere low-poly / UV Sphere
- Icosphere
- Capsule

As primitivas podem permanecer como **generators paramétricos** enquanto o usuário altera dimensões e segmentos. Quando precisar de edição topológica arbitrária, usa Make Editable / Convert to Mesh.

# Draw on Face

Com Draw ativo, a face plana sob o cursor vira o plano de trabalho. A câmera **não** é movida automaticamente; o comando "Olhar para o plano" (ou uma preferência) a alinha quando o usuário quiser. Um profile desenhado nela pode:

- Splitar a face.
- Ser puxado para fora como detalhe.
- Ser empurrado para dentro como recess/hole.
- Servir como região para extrusão.

# Revolve e Sweep

**Revolve 360° faz parte do Core V1**. Ele transforma um Profile em volume rotacional com quantidade explícita de segmentos radiais; partial/helical revolve e perfis variáveis permanecem fora da V1 conforme capítulo 14.

**Simple Sweep é Official Extension / V1.x**, não Core V1 e não sweep CAD completo. O contrato detalhado está fechado no capítulo 14: `closed polygon Profile + open polyline Path`, framing por parallel transport, densidade geométrica explícita, caps controlados e falha previsível diante de auto-intersections problemáticas. O objetivo continua ser cabos, tubos, galhos, chifres e formas semelhantes com baixa densidade geométrica controlável.

**Loft fica fora do escopo oficial atual.** Correspondence entre múltiplos profiles, vertex matching, twist, resampling e demais edge cases não justificam o custo para o objetivo do Petunia. Loft poderá existir futuramente via plugin.

# História simples e não destrutiva

Sem construir uma árvore CAD completa, determinados objetos podem manter uma pequena pilha procedural:

```
Profile
↓
Extrude
↓
Mirror
↓
Bevel
```

Cada estágio pode ser ajustável até o usuário decidir converter para mesh editável. No workspace DRAW, formas permanecem paramétricas; "Converter em polígonos" é a passagem explícita, reversível por Undo, para edição de componente no workspace POLY. A história procedural deve permanecer pequena; operações mesh arbitrárias continuam apoiadas pelo Undo/Redo transacional normal.
