# UV Workspace — UX e Comportamento

> **Status: aprovado em 2026-10-08.**
>
> Este documento especializa o contrato global de [Workspaces, Feedback Visual e Acessibilidade](../09-ui/workspaces-feedback-accessibility.md) para UV. A arquitetura geométrica/algorítmica continua definida em [UV Workspace e Editor](./architecture-decision.md).

## 1. Objetivo

UV deve deixar de parecer uma ferramenta escondida dentro do Inspector.

O usuário precisa perceber imediatamente a relação:

```text
superfície 3D
↔
island UV 2D
↔
textura / checker
```

O workspace deve ensinar essa relação visualmente, sem exigir que o usuário conheça terminologia avançada antes de operar.

## 2. O que já existe e deve ser reaproveitado

A implementação atual já possui:
- seleção UV;
- move;
- scale;
- rotate;
- pins;
- seams;
- Auto Unwrap/xatlas;
- Box/Cube projection;
- View projection;
- Pack;
- texel density;
- Equalize Density;
- Stitch;
- Relax;
- UV hit;
- diagnostics;
- texture background;
- selected faces;
- seam overlay;
- pinned vertices;
- checker/coverage usados por Paint.

A GUI atual já desenha um editor UV funcional em 2D, porém preso a aproximadamente 256×256 dentro do Inspector.

Isso é classificado como **capacidade existente em superfície inadequada**.

## 3. Macro-layout

Default:

```text
┌──────────────────────────────────────────────────────────────────┐
│ Header                                                           │
├──────┬──────────────────────────────┬─────────────────────────────┤
│ Tool │        3D VIEWPORT           │ UV MAPS / ISLANDS          │
│ Rail │                              ├─────────────────────────────┤
│      ├──────────── splitter ────────┤ PROPERTIES                  │
│      │          UV EDITOR           │ Selection / Unwrap / Pack  │
│      │ Context Bar                 │ Density / Health            │
├──────┴──────────────────────────────┴─────────────────────────────┤
│ Auxiliary Drawer — collapsed by default                         │
└──────────────────────────────────────────────────────────────────┘
```

A implementação pode preferir split vertical lado a lado em telas largas e split horizontal em formatos estreitos, desde que:
- ambas as surfaces continuem utilizáveis;
- o divisor seja persistente por workspace;
- cada surface possa ser maximizada.

A proporção inicial recomendada em desktop largo é próxima de **55% 3D / 45% UV**.

## 4. Tool Rail

Ferramentas que alteram interação direta no UV Editor:

### Selection
- Select;
- seleção de Corner/Vertex;
- Edge;
- Face;
- Island.

A UI não precisa mostrar cinco ferramentas separadas se um selector compacto resolver melhor.

### Transform
- Move;
- Rotate;
- Scale.

Devem reutilizar a mesma gramática de transformação do MODEL:
- modal;
- preview;
- numeric input;
- constraint quando aplicável;
- Enter commit;
- Esc cancel;
- um Undo.

### Seam / Pin
- Mark/Clear Seam;
- Pin/Unpin.

Podem aparecer como ações contextuais quando uma seleção compatível existir, sem ocupar permanentemente o rail inteiro.

### Projection / Unwrap
Unwrap e Projection são ações, não pointer tools. Preferir Context Bar/Properties/Command Palette.

## 5. Work Surface 3D

O lado 3D mostra:
- mesh;
- seleção correspondente;
- seams;
- checker temporário;
- stretch/density overlay quando habilitado;
- paint preview quando útil.

### Seleção sincronizada
A preferência `Sync Selection` é explícita:
- ON → seleção UV e 3D se correspondem;
- OFF → cada domínio mantém sua seleção.

A UI deve mostrar claramente o estado de sync, nunca inferi-lo silenciosamente.

## 6. UV Editor

O UV Editor é uma superfície de primeira classe.

Deve fornecer:
- zoom;
- pan;
- frame all;
- frame selected;
- grid 0..1;
- tile bounds;
- textura/checker;
- UV wire;
- selected;
- active;
- seams;
- pins;
- diagnostics overlay;
- transform preview.

O editor não deve assumir que todo projeto ficará eternamente em 0..1, embora UDIM permaneça fora da V1.

## 7. Structure = UV Maps / Islands

### UV Maps / Sets
Quando houver suporte a múltiplos sets, a região Structure hospeda:
- nome;
- active;
- usage/material context;
- add/remove/rename quando suportado.

Enquanto V1 operar apenas no set principal, a UI não inventa controles falsos.

### Islands
Lista derivada, não documento paralelo.

Linha:
```text
Island 12       34 faces     density 512 px/m
```

Permitir:
- select;
- frame;
- lock/fixed para pack quando suportado;
- diagnostics badge;
- busca/filtro futuro.

A lista não deve persistir IDs frágeis como verdade autoral.

## 8. Properties

### Selection
- selection type;
- Sync Selection;
- transform values quando relevante;
- pin state.

### Unwrap
- Auto Unwrap;
- provider/options estritamente necessárias;
- respeitar seams/pins;
- nunca alterar topology autoral.

### Projection
- Planar/View;
- Box/Cube;
- Cylindrical;
- Spherical, quando implementados.

Ferramentas não implementadas não aparecem como controle funcional.

### Pack
- selected only;
- padding em pixels;
- rotate 90°;
- respect fixed/pinned;
- deterministic result.

### Texel Density
- current;
- target;
- Set;
- Match;
- Equalize Selected Islands.

### UV Health
Painel de diagnóstico de primeira classe.

## 9. UV Health

Indicadores:
- Islands;
- overlap;
- zero-area;
- stretch;
- out-of-range;
- texel density variance;
- seam without split;
- split without seam.

Cada problema recebe:
- severidade;
- contagem;
- descrição;
- ação quando possível.

Exemplo:

```text
Overlap              3 islands       [Select]
High Stretch         12 faces        [Show]
Density mismatch     4 islands       [Equalize]
```

Cor é redundante, nunca a única indicação.

## 10. Checker

Checker é overlay temporário de análise, não Material persistente.

A Context Bar pode expor:
- Checker on/off;
- scale;
- Stretch;
- Density.

A mesma opção deve refletir no 3D e UV quando fizer sentido.

## 11. Seams

Seam possui estado visual consistente em 3D e 2D.

Regras:
- Mark/Clear é Undoable;
- Auto Unwrap pode adicionar seam intent explicitamente;
- nunca ocultar que boundaries foram criadas;
- seam sem UV split entra em diagnostics;
- UV split sem seam entra em diagnostics.

## 12. Pins

Pin é constraint de solver.

Visual:
- símbolo/forma distinta;
- não depender apenas de vermelho;
- selecionado e pinned continuam distinguíveis.

Transform direto explícito pode mover pinned corner; o pin continua na nova posição, conforme arquitetura aprovada.

## 13. Stitch e Split

### Stitch
UI deve deixar claro qual lado fica parado e qual move quando essa escolha for relevante.

Não usar média silenciosa dos dois lados como comportamento universal.

### Split UV
Cria descontinuidade UV:
- sem duplicar vertex 3D;
- atualiza seam intent.

## 14. Transformações

Move/Rotate/Scale no UV usam o mesmo contrato de interação do MODEL:
- drag threshold configurável;
- click-move-click como alternativa;
- numeric typing;
- fine mode;
- snap futuro;
- HUD;
- commit/cancel explícitos.

Botões atuais de `←U`, `U→`, `±15°` podem sobreviver como quick nudge em menu/contexto, mas não devem ser o mecanismo primário de transformação.

## 15. Context Bar

Exemplo em seleção de islands:

```text
Island | Sync ON | Pin | Unwrap | Pack | Checker | Density 512
```

Durante transform:

```text
Move · U 0.124 · V -0.032 | Precision | Enter Confirm | Esc Cancel
```

## 16. Feedback de seleção

Estados no UV Editor:
- unselected wire;
- hover candidate;
- selected;
- active;
- pinned;
- seam;
- invalid/diagnostic.

Não reutilizar uma cor para significados incompatíveis.

## 17. Empty states

### Sem mesh
```text
Select a mesh to edit UVs
```

### Mesh sem UV
```text
This mesh has no UV layout
[Unwrap] [Project from View]
```

### Generator não editável
```text
Manual UV editing requires editable geometry
[Make Editable]
```

Sem conversão silenciosa.

## 18. Acessibilidade

Além do contrato global:
- todas as operações têm rota por teclado/Command Palette;
- reorder/list selection não depende de drag;
- zoom possui comandos;
- Frame Selected/All possui comandos;
- pins/seams não dependem apenas de cor;
- diagnostics são legíveis por leitor de tela;
- split possui ajuste por teclado;
- foco informa qual surface está ativa;
- F6 alterna explicitamente 3D e UV como regiões distintas;
- hit targets de corners/edges podem ser ampliados sem alterar precisão geométrica;
- preferências de tamanho de handles afetam UV também.

## 19. Responsive behavior

Em largura confortável:
- 3D + UV simultâneos.

Em largura reduzida:
- split pode virar tabs `3D | UV`;
- manter estado/câmera/zoom de cada superfície;
- nunca reduzir UV Editor a miniatura de Inspector.

Structure/Properties podem colapsar, mas UV continua sendo work surface.

## 20. Performance

- islands/diagnostics são caches derivados por revision;
- listas grandes virtualizadas;
- overlays atualizam sem reconstruir topology;
- checker não cria Material persistente;
- transform preview usa estado transitório;
- idle render-on-demand permanece.

## 21. Migração recomendada

1. extrair `UvWorkspaceViewModel`;
2. promover UV Editor de Inspector para Work Surface;
3. implementar split persistente 3D/UV;
4. criar selector de selection domain UV;
5. mover operações para Properties/Context Bar;
6. criar Structure de Islands;
7. consolidar UV Health;
8. substituir botões de nudge como interação primária por transform modal;
9. adicionar navegação F6 entre as duas surfaces;
10. testar sync ON/OFF, Undo e High Contrast.

## 22. Gates

- UV Editor é work surface;
- Auto Unwrap não altera topology;
- selection sync é explícito;
- seams/pins têm feedback coerente;
- Pack determinístico;
- padding em pixels;
- transform modal tem Undo único;
- diagnostics são acionáveis;
- keyboard-only cobre fluxo principal;
- split funciona em UI Scale 100–200%;
- Paint coverage corresponde ao UV exibido.

## 23. Decisão final

UV será um workspace de relação **3D ↔ 2D**, não um painel técnico secundário.

O editor existente deve ser promovido e reorganizado, preservando seus algoritmos úteis enquanto a arquitetura de seleção/FaceCorner é corrigida conforme o capítulo técnico.


## Checkpoint de apresentação U02/U03 — 2026-10-09

O drawer auxiliar recebe o diagnóstico já projetado pela Application, com ações Pack/Equalize existentes. A promoção do editor UV a Work Surface, lista de Islands e diagnóstico acionável por alvo permanecem pendentes. **Código escrito, não compilado/testado:** bateria e aceite nativo adiados pelo usuário. Registro e limites: [Slint Rescue §20](../09-ui/slint-rescue-plan.md#20-wave-u02u03--regioes-independentes-e-drawer-2026-10-09).
