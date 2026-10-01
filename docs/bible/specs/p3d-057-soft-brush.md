# P3D-057 — Soft Brush

<aside>
🧩

Estado: **em implementação (iniciativa Paint, 2026-09-16)** · Prioridade: P1.
Descriptor unificado `BrushSettings` com hardness/strength/flow/spacing; stroke
engine baseado em dabs.

</aside>

## Objetivo

Pincel suave convencional com size, opacity/strength e falloff simples.

## BrushSettings — hardness / strength / flow / spacing (decisão 2026-09-16)

O Soft Brush consome o mesmo descriptor unificado `BrushSettings` do Pixel
Brush e do Eraser:

- `hardness` — controla o falloff da borda (0 = totalmente suave, 1 = borda
  definida); é o parâmetro que diferencia o Soft do Pixel;
- `strength` — intensidade de cada dab sobre o canal alvo;
- `flow` — taxa de acúmulo ao longo do stroke (diferente de strength, que é
  por-dab);
- `spacing` — distância entre dabs consecutivos ao longo do trajeto.

O stroke engine é baseado em **dabs** (carimbos individuais) avaliados ao longo
do caminho do cursor; a acumulação de `flow × strength` por dab é o que produz
o efeito de pincel contínuo. O mesmo motor serve ao novo **Airbrush** (modo
aditivo contínuo, em implementação).

## Arquitetura

Compartilhar stroke engine com Pixel Brush/Eraser; brush parameters são descriptors, não lógica de widget.

## Dependências

P3D-055, P3D-061, P3D-132.

## Testes / DoD

Pressure somente se suportado, strokes rápidos/lentos, opacity acumulada, mask e undo agrupado por stroke.

## Estado de implementação (2026-09-30)

Opacidade × fluxo: `StrokeBuffer` acumula o fluxo e limita pela opacidade (dabs sobrepostos não escurecem além do teto). Além do descriptor básico há `BrushStyle` (ponta, ângulo, achatamento, mistura, estabilizador, jitter determinístico por semente, espalhamento, densidade do spray) e presets (`BrushPreset`, embutidos e do usuário). Novos pincéis: Smudge, Blur, Dodge, Burn, Spray e Clone. Pressão: sem tablet no shell atual (gancho previsto).