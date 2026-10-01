# P3D-055 — Paint Workspace

<aside>
🧩

Estado: **workspace rudimentar/não funcional** · Prioridade: P0.

</aside>

## Objetivo

Reconstruir Paint como workspace contextual simples e poderoso para low-poly, em espírito de “mini Photoshop” sem virar editor 2D generalista.

## UI desejada

- Layers com reorder drag-and-drop, visibility e opacity.
- Brush/Pencil/Eraser/Fill/Picker.
- Painel de efeitos simples.
- Isolamento/máscaras via P3D-132.
- Decals via P3D-133.
- Effect Stack via P3D-134.
- Tool Properties adaptadas à tool ativa.

## Arquitetura

Paint opera sobre texture/layer resources do P3D-050; strokes são transacionais e headless-testable quando possível. Workspace não cria cópia própria do material.

## Dependências

P3D-050, P3D-056–062, P3D-132–134.

## Testes / DoD

Pintura funcional no modelo, undo, save/load, layers, resize/DPI, seleção de canal e ausência de vazamento de estado entre workspaces.

## Adendo pós-V1 — evolução do toolbox

A baseline V1 `Brush/Pencil/Eraser/Fill/Picker` permanece congelada. Após estabilização do Paint Core, evoluir incrementalmente conforme `44 — Pós-V1: Surface Paint Toolbox além do Brush`:

```
Decal
→ Line / Shape
→ Gradient
→ Face / UV Island Fill
→ Projection / Stencil
→ Clone / Patch
→ Path Paint após Spline Core
```

Princípios:

- cada tool reutiliza TextureResources/Layers/Masks/Undo existentes;
- Decal e Projection reutilizam Surface Manipulator;
- Path Paint reutiliza Spline Core;
- nenhuma ferramenta cria scene/world authoring;
- toolbar e Tool Properties permanecem contextuais para não aumentar carga cognitiva.

## Iniciativa Paint (decisão 2026-09-16) — em implementação

A iniciativa Paint redefine o escopo imediato do workspace. Tudo abaixo está
**em implementação** nas branches `paint/core-engine` e `paint/ui-redesign`;
nada é reivindicado como entregue.

### Layout "mini Photoshop" dentro do shell congelado (cap. 36)

O Paint respeita o shell `MODEL / PAINT` do capítulo 36 (sem docking livre, sem
UV pill na V1 durante a iniciativa). O canvas 2D ocupa o centro com prévia 3D
ao lado; o painel direito concentra **Layers** (com drag-and-drop), **Brush** e
**Effects**. A antiga superfície dedicada de edição UV sai da UI V1 (ver
P3D-063/064); o utilitário de projeção de `module-uv` aparece dentro do Paint
como "Preparar superfície".

### BrushSettings unificado

Novo descriptor único `BrushSettings` substitui a dualidade `canvas_brush`
(px) × `paint_radius` (metros):

- `size_px` — tamanho em **pixels de tela** (estilo Photoshop);
- `hardness` — dureza da borda (0–1);
- `strength` — intensidade por dab;
- `flow` — fluxo acumulado ao longo do stroke;
- `spacing` — espaçamento entre dabs.

O modelo é o mesmo para Pixel, Soft, Eraser e o novo **Airbrush** (pincel
aditivo contínuo, em implementação). A antiga dualidade é deprecada em favor
do descriptor único; P3D-056 e P3D-057 detalham os parâmetros.

### Effect Stack com presets do capítulo 42

A pilha de efeitos (P3D-134) ganha UX de presets "Add Effect" na pilha de
camadas. Nodes iniciais confirmados:

- **Já existentes no modelo**: Pixelate, Posterize, Invert.
- **Novos a implementar**: Grain/Noise, Levels/Threshold, Brightness/Contrast,
  Hue/Saturation (lista de nodes do capítulo 42).

### Surface Recipe graph (P3D-113) — headless primeiro

O modelo de dados do Surface Recipe graph começa **headless**: DAG, sockets,
avaliador determinístico e cache. **Sem editor visual** nesta fase; o editor
gráfico de nodes fica para o ciclo pós-Paint, quando a pilha de efeitos estiver
estável.

## Estado de implementação (2026-09-30)

PAINT tem ferramenta **Select** (troca de objeto, face e ilha UV) e **Isolar objeto**; entrar no workspace ativa o pincel. Do adendo pós-V1: Elipse e Gradiente (linear e radial, compondo por cima) existem; Decal por importação, Projection/Stencil e Path Paint seguem ausentes. Detalhes em [`paint-uv-gap-matrix`](../../development/paint-uv-gap-matrix.md).