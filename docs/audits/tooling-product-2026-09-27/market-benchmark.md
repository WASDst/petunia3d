# Benchmark de mercado — padrões aplicáveis sem descaracterizar o Petunia3D

> **Consulta:** 2026-09-27.  
> **Regra:** fontes externas informam ergonomia e engenharia; não substituem o
> Livro Vivo. O objetivo não é transformar Petunia3D em Blender, Maya, Substance,
> Houdini, XGen ou ZBrush.

## 1. Resumo

| Produto | Padrão maduro observado | O que adotar | O que não copiar |
| :--- | :--- | :--- | :--- |
| Blender | operações modais com parâmetros, overlays e edição UV abrangente | confirmação/cancelamento previsível, last operation, diagnósticos de bevel/UV | modo Rendered, densidade de menus, clone de atalhos |
| Maya | toolkit contextual, preselection, seleção refinada e UV Toolkit | filtro/feedback de seleção, operações UV agrupadas por tarefa | complexidade de modes/marking menus na V1 |
| Substance 3D Painter | layers, masks, channels e performance guiada por stack | composição incremental, canal ativo explícito, masks simples quando houver orçamento | pipeline multicanal completo antes de estabilizar PAINT V1 |
| Houdini | guides, resample, interpolate, masks e cadeia procedural | dados de guide explícitos, attachment e modifiers previsíveis | grafo procedural geral/nodes como pré-requisito |
| Maya XGen | splines interativos ancorados e grooming por guides | draw/edit guide on surface, comprimento/segments controláveis | sistema completo de grooming/render de fios |
| ZBrush | ações contextuais rápidas e curve brushes gerando forma | gesto direto para clumps low-poly, taper/profile e preview imediato | linguagem opaca de ícones e comportamento difícil de descobrir |

## 2. MODEL — Blender, Maya e ZBrush

### 2.1 Operações modais e feedback

A documentação oficial do
[Blender Bevel](https://docs.blender.org/manual/id/4.5/modeling/meshes/editing/edge/bevel.html)
mostra uma operação com largura, segmentos, profile, clamp overlap, affect e
modos de cálculo de width. O valor para Petunia3D não é copiar todas as opções;
é o padrão de interação:

- preview durante o gesto;
- parâmetros alteráveis sem iniciar outra ferramenta;
- clamp/overlap explícito;
- resultado previsível para vertex/edge;
- ajuste posterior da última operação.

**Gap Petunia:** Round Edge existe, mas os estados de clamp, degeneração e custo
não formam um contrato visual tão claro. A bridge conhece parte dos parâmetros,
e o renderer/overlay não apresenta um diagnóstico unificado.

**Recomendação:** manter poucos parâmetros V1 — Width, Segments, Profile e Clamp
— com feedback próximo ao cursor e Inspector sincronizado. Opções avançadas
ficam progressivas, não permanentemente expostas.

### 2.2 Seleção e contexto

O
[Maya Modeling Toolkit](https://help.autodesk.com/cloudhelp/ENU/MayaCRE-Modeling/files/GUID-7EC09146-E5E8-449E-83D7-BFE7C780C32D.htm)
centraliza modos de seleção e opções de componentes para reduzir troca mental.
Petunia3D já tem vantagem potencial: vocabulário menor e workspaces mais
restritos.

**Adotar:**

- preselection independente de seleção;
- sinalização de seleção oclusa/X-Ray;
- filtro por domínio e estado disabled contextual;
- feedback de soft/proportional influence;
- ação disponível somente quando `can_execute` explicar o porquê.

**Não adotar:** proliferação de modes globais e menus contextuais profundos. O
Petunia deve continuar viewport-first e baseado em ferramentas claras.

### 2.3 Modelagem contextual e curvas

As páginas oficiais do ZBrush para
[ZModeler/Polygon Actions](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/modeling-basics/creating-meshes/zmodeler/zmodeler-actions/polygon-actions/polygon-actions.html),
[Curve Brushes](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpting-brushes/curve-brushes/curve-brushes.html)
e
[Curve Alpha](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/sculpting/sculpting-brushes/curve-alpha/curve-alpha.html)
mostram dois padrões valiosos para o foco estilizado do Petunia:

1. ação depende do elemento sob o cursor e produz preview imediato;
2. uma curva pode instanciar/esticar uma forma com taper e perfil.

**Aplicação Petunia:** Draw-on-Face e Hair Ribbon devem parecer extensões naturais
do shape-first: desenhar uma linha/curva, ajustar poucos handles, largura/taper e
confirmar um generator low-poly. O resultado continua editável até `Make
Editable`.

**Evitar:** esconder o significado em combinações de ícone + hover sem texto. O
Petunia deve manter nome, tooltip, Command Palette e Inspector.

## 3. UV — Blender e Maya

A documentação oficial de
[Blender UV Editing](https://docs.blender.org/manual/sr/4.5/modeling/meshes/uv/editing.html)
e do
[Maya UV Toolkit](https://help.autodesk.com/cloudhelp/ENU/MayaCRE-Modeling/files/GUID-73BF7546-3BF2-44F3-9192-15A024CEC173.htm)
apresenta um conjunto convergente: selection, transform, cut/sew, unfold/relax,
pin, straighten/alignment, layout/pack e texel density.

O engine Petunia já cobre uma parcela relevante: seams, pins, unwrap, transform,
stitch, relax, pack e density. O gap maior é produto/escala:

- seleção UV precisa ser independente e estável;
- overlap/stretch/out-of-range precisam overlays claros;
- operações longas precisam progress/cancel;
- o editor não deve reconstruir uma string SVG global a cada mudança;
- a utilidade deve aparecer no fluxo PAINT, conforme o caderno.

**Diferenciação recomendada:** em vez de um workspace UV generalista, oferecer um
assistente `Preparar superfície` com três estágios: `Cut`, `Unwrap`, `Pack &
Check`. O modo avançado expõe pins/density/relax sem dominar a UI principal.

## 4. PAINT — Substance 3D Painter

### 4.1 Layer stack

A documentação oficial da Adobe sobre
[Layer Stack](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/interface/layer-stack/layer-stack)
descreve layers, groups, masks, blending e channels como sistema central. A
página de
[Geometry Mask](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/interface/layer-stack/geometry-mask)
mostra o valor de limitar rapidamente onde um layer atua.

Petunia3D já tem layers/groups/opacity/visibility/lock/effects/decal. As lacunas
prioritárias não são adicionar dezenas de blend modes; são:

1. garantir undo correto;
2. remover cópias integrais por dab;
3. explicitar canal ativo e escopo;
4. fornecer mask simples por seleção/parte quando o compositor incremental
   estiver pronto;
5. manter a pilha como única autoridade do resultado, com cache derivado.

### 4.2 Brush e spacing

A documentação oficial do
[Paint Brush](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/painting/paint-tools/paint-brush)
organiza parâmetros de brush e material de maneira previsível. O aprendizado
principal é que spacing/flow/pressure pertencem ao motor de stroke, não ao loop
de eventos de uma UI específica.

O módulo Petunia já possui `BrushSettings::stroke_dabs`, mas o frontend Slint
não o usa consistentemente. Consolidar esse sampler oferece ganho imediato de
qualidade e performance, sem ampliar escopo.

### 4.3 Performance da pilha

A própria Adobe publica
[orientações de performance para layer management](https://experienceleague.adobe.com/en/docs/substance-3d-painter/using/technical-support/performance-guidelines/layer-management),
o que reforça que layers, masks e resoluções têm custo acumulativo. Para
Petunia3D, a conclusão é arquitetural:

- mostrar memória estimada por texture set;
- compor somente tiles alterados;
- limitar/avisar resolução e quantidade de efeitos caros;
- permitir flatten/bake explícito e reversível por undo;
- medir tempo de compositor e upload separadamente.

**Não copiar agora:** material painting multicanal completo, smart materials,
bakers complexos e dezenas de filtros. Esses recursos multiplicariam o custo de
um compositor que ainda copia o canvas integralmente.

## 5. Hair — Houdini e Maya XGen

### 5.1 Padrões comuns

As páginas oficiais de
[Houdini Guide Groom](https://www.sidefx.com/docs/houdini/nodes/sop/guidegroom.html),
[Houdini Groom](https://www.sidefx.com/docs/houdini/fur/groom.html),
[Houdini Guide Deform](https://www.sidefx.com/docs/houdini/nodes/sop/guidedeform.html)
e
[Maya XGen Interactive Groom Splines](https://help.autodesk.com/cloudhelp/2026/ENU/Maya-CharEffEnvBuild/files/GUID-D25EF979-1444-4DB7-90AD-08A83F5DC6CE.htm)
convergem em conceitos:

- guides são dados editáveis distintos da geometria final;
- roots são relacionados à superfície;
- resampling/segments controlam resolução;
- comprimento, direção, smooth, clump/noise e masks são modificadores;
- deformações da superfície precisam propagar aos guides;
- geração/interpolação de hairs é separada da edição dos guides.

### 5.2 Tradução correta para Petunia3D

Petunia3D deve aplicar esses conceitos a **mesh clumps estilizados**, não a
milhões de fios:

```text
GuideSpline
  + SurfaceAttachment(root)
  + Width/Taper/Profile/Segments
  + optional Mirror/Clump modifiers
  → HairClumpGenerator
  → evaluated low-poly mesh
  → explicit Bake/Make Editable
```

Isso preserva o diferencial shape-first e limita o problema a um generator
geométrico previsível.

### 5.3 O que rejeitar

- dependência de um node graph geral;
- simulação física como requisito de V1;
- renderer especializado de strands;
- groom layers/masks avançados antes de attachment robusto;
- um objeto por fio/clump, que destruiria performance e usabilidade;
- comportamento procedural implícito sem Bake/Make Editable.

## 6. Matriz de adoção

| Prática de mercado | Valor | Custo | Decisão |
| :--- | :---: | :---: | :--- |
| Preview modal + last operation | Alto | Médio | Adotar já em MODEL |
| Clamp/diagnóstico de operação | Alto | Baixo/Médio | Adotar já |
| UV Toolkit completo como workspace | Médio | Alto | Rejeitar; usar utility PAINT |
| Seleção UV independente | Alto | Médio | Adotar |
| Stroke sampler unificado | Alto | Médio | Adotar imediatamente |
| Masks simples por seleção/parte | Alto | Médio | Adotar após compositor incremental |
| Multicanal PBR completo | Médio | Muito alto | Adiar |
| Guides + surface attachment | Alto | Médio/Alto | Adotar como fundação Hair |
| Interpolação de milhões de hairs | Baixo para proposta | Muito alto | Rejeitar |
| Curve-driven mesh ribbon/clump | Muito alto | Médio | Adotar |
| Node graph procedural geral | Baixo para V1 | Muito alto | Rejeitar |
| Context action por hover | Alto | Médio | Adotar com nomes/tooltips claros |

## 7. Diferenciação profissional do Petunia3D

O produto não vence pela quantidade de comandos. Ele pode vencer por reduzir o
caminho entre intenção e forma:

1. **Create → Shape → Refine → Paint** sem mudar para uma linguagem CAD.
2. **Generators pequenos e visíveis**, com custo previsto e Bake explícito.
3. **Draw-on-surface consistente** para perfis, cortes, decals e Hair.
4. **Feedback honesto**, sem nomes de tecnologia que o renderer não implementa.
5. **Low-poly como decisão de produto**, com density/segments/budget sempre
   visíveis.
6. **UV como assistência**, não como um segundo editor obrigatório.
7. **Acessibilidade por arquitetura**, com commands/keymap/focus region únicos.

Essa direção aproveita padrões maduros do mercado sem herdar sua complexidade
histórica.
