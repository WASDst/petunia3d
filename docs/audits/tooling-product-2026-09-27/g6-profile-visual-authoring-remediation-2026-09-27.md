# Remediação G6 — Authoring visual persistente de Profile no Slint

> **Data:** 2026-09-27
>
> **Natureza:** registro de implementação derivado; não substitui o Livro Vivo
>
> **Escopo:** migração incremental do editor Profile de `ProfileState` transitório
> para `ProfileId + SplineId`, preservando o overlay e o layout Slint existentes

## 1. Autoridade e decisão do gate

Este checkpoint executa o delta visual deixado explícito pelo G5. As fontes
canônicas aplicadas são `P3D-161`, as fundações 02, 34, 36 e 40, e os contratos
de transação/ownership já adotados pelo projeto.

Não foi criado um segundo editor de curvas. O `Path` Slint, os callbacks de
ponteiro, o hit-test em pixels lógicos, os cards de workplane e os controles de
volume existentes foram preservados. A mudança é de autoridade: pontos,
handles, fechamento, workplane e espessura pertencem agora aos recursos
persistentes; `ProfileState` conserva apenas parâmetros de sessão ainda não
modelados como generator, como depth, ângulo de revolve e tolerância visual.

## 2. Implementation-vs-Spec Gap Matrix

| Contrato | Antes | Delta comprovado | Depois |
| :--- | :---: | :--- | :---: |
| S0-UI-OWNER | `FUNCTIONAL_BUT_DIFFERENT` | bridge guarda `ProfileId`; curva é resolvida por `SplineId` | `COMPLIANT` para o editor Profile atual |
| S0-POINT-ID | `BROKEN` | seleção e hit-test deixam índices e usam `SplinePoint::id` | `COMPLIANT` |
| S0-OPEN-DRAFT | `BROKEN` | `ProfileResource` aceita draft vazio/aberto; generator exige fechamento | `COMPLIANT` |
| S0-ADD | `PARTIALLY_COMPLIANT` | primeiro ponto cria Profile+Spline; demais usam `AddSplinePointCmd` | `COMPLIANT` |
| S0-MOVE | `PARTIALLY_COMPLIANT` | preview muta a spline, restaura A e despacha um comando no pointer-up | `COMPLIANT` |
| S0-HANDLES | `PARTIALLY_COMPLIANT` | Broken/Aligned/Mirrored mapeiam para Sharp/Smooth/Symmetric | `COMPLIANT` para mouse/Alt |
| S0-CLOSE | `FUNCTIONAL_BUT_DIFFERENT` | fechamento usa `SetSplineClosedCmd` e mantém o recurso | `COMPLIANT` |
| S0-PRIMITIVES | `DUPLICATED` | Rectangle/Circle atualizam uma spline persistente em uma transação | `COMPLIANT` |
| S0-SMOOTH | `DUPLICATED` | smooth/sharp atualizam o recurso por `UpdateSplineCmd` | `COMPLIANT` |
| S0-WALL | `FUNCTIONAL_BUT_DIFFERENT` | espessura atualiza `ProfileResource` por `UpdateProfileCmd` | `COMPLIANT` |
| S0-EXIT | `BROKEN` | trocar ferramenta/Escape encerra edição sem apagar o Profile | `COMPLIANT` |
| S0-VOLUME | `PARTIALLY_COMPLIANT` | preview lê snapshot derivado dos recursos e commit preserva o Profile | `COMPLIANT` para o gerador de mesh legado |
| S0-UNDO | `BROKEN` | inserção e drag produzem uma entrada por gesto; recursos sobrevivem ao volume | `COMPLIANT` |
| A11Y-HIT | `COMPLIANT` | alvo continua com 16 px lógicos e destaque de seleção existente | `COMPLIANT`, preservado |
| A11Y-KEYBOARD | `PARTIALLY_COMPLIANT` | Close, Generate, Delete, Enter e Escape continuam roteáveis | `PARTIALLY_COMPLIANT`; navegação entre pontos/handles ainda falta |
| HA-03 | `MISSING` | Hair pode reutilizar o mesmo owner visual/persistente de curvas | `PARTIALLY_COMPLIANT` como fundação; Ribbon ainda não implementado |

## 3. Ownership e fluxo transacional

```text
Slint pointer/key intent
  → SlintUiBridge (ProfileId + PointId selecionado)
  → Command (Create/Add/Move/Handles/Close/Update/Delete)
  → SplineResource / ProfileResource
  → ProjectChanges::SPLINES ou PROCEDURAL
  → overlay derivado do recurso persistente
```

Um arraste existente usa o mesmo padrão modal do restante do editor:

1. captura ponto e revision originais no pointer-down;
2. publica previews absolutos sobre a spline, sem criar histórico por frame;
3. no pointer-up captura B, restaura A e despacha `MoveSplinePointCmd` ou
   `SetSplineHandlesCmd` uma única vez;
4. no-op não cria entrada.

Inserção cria sua entrada no pointer-down. O arraste contínuo do novo ponto
atualiza os handles dentro da mesma ação; Undo remove Profile+Spline quando era
o primeiro ponto e Redo restaura também os handles finais. IDs não mudam durante
o gesto, portanto deleção e seleção não dependem de posição no vetor.

## 4. Compatibilidade shape-first

O G5 exigia equivocadamente que todo `ProfileResource` já fosse fechado e
tivesse três pontos. Isso tornava impossível persistir o estado intermediário
prescrito por `P3D-161`. O contrato foi reconciliado:

- authoring aceita zero ou mais pontos e spline aberta;
- planaridade, finitude, ownership e ausência de attachment continuam
  invariantes do Profile;
- Sweep/PathGenerator rejeita explicitamente profile aberto ou com menos de
  três pontos;
- o preview de Extrude fecha por Command quando há três pontos suficientes;
- sair da ferramenta preserva o draft em vez de apagá-lo silenciosamente.

Rectangle e Circle não substituem o `ProfileId` ativo. Eles substituem a
geometria da spline em uma entrada de Undo, evitando recursos órfãos e a antiga
duplicação entre `points` e `nodes`.

## 5. UI, feedback e acessibilidade

- nenhum componente, cor, ícone ou layout novo foi introduzido;
- o overlay existente continua desenhando curva, handles e âncoras em screen
  space, mas deriva tudo da spline persistente;
- seleção usa UUID e continua destacada mesmo após reordenação/deleção;
- o HUD informa que Escape encerra a edição, em vez de prometer cancelamento
  destrutivo;
- Delete continua disponível por comando/context menu;
- Enter/Generate continua promovendo o profile fechado ao preview de volume;
- workplane Ground/Face/View encerra o contexto ativo e inicia outro sem apagar
  o recurso anterior.

Gap remanescente: o editor ainda não oferece foco sequencial entre anchors e
handles, nudge por teclado, anúncio estruturado de coordenadas/modo de handle,
hover diferenciado nem preferência configurável de tamanho do handle. Esses
itens permanecem necessários para paridade keyboard-only/screen-reader.

## 6. Performance e segurança

- movimento de ponteiro não clona `Project` nem adiciona snapshots ao Undo;
- hit-test e geração do path visual permanecem O(n) no número de control points;
- tesselação continua adaptativa e só ocorre ao montar o view model/preview;
- operações em lote usam uma cópia da spline, não uma sequência de Commands;
- validação f64/planar ocorre no domínio antes de aceitar recurso persistente;
- geração continua usando adapter derivado para o builder legado, sem manter um
  segundo owner de pontos;
- `ProfileId` inválido após Undo é tratado como ausência, sem indexar memória ou
  reaproveitar índice de outro recurso.

Para curvas muito grandes, o próximo limite é reconstruir a string completa do
`Path` a cada frame de ponteiro. Antes de Hair de alta densidade, o overlay deve
ganhar dirty ranges ou buffers de instância para anchors/handles.

## 7. Provas do conjunto

O pacote adiciona ou atualiza provas para:

- Profile aberto persistente válido e rejeição como dependência de Sweep;
- Commands de atualização de spline/profile com Undo/Redo;
- desenho, fechamento e volume usando os recursos persistentes;
- Rectangle/Circle, smooth/sharp, wall thickness, revolve e sweep;
- seleção/hit-test/drag/delete por UUID;
- um único Undo para inserção com handle arrastado;
- persistência após trocar de ferramenta;
- cancelamento do preview de volume sem apagar Profile;
- commit de volume preservando Profile e desfazendo somente a mesh gerada.

## 8. Resíduos e próximo gate

1. remover ou restringir as APIs de authoring transitório de `module-model`
   depois que o egui legado deixar de depender delas;
2. extrair o bloco Profile do monólito `SlintUiBridge` para um controller sem
   mudar o contrato público;
3. implementar seleção/nudge/handle-mode por teclado e feedback acessível;
4. persistir Extrude/Revolve como generators vivos, substituindo o adapter de
   mesh legado;
5. iniciar Hair H0 com `Curve → Ribbon Mesh`, reutilizando Spline, overlay,
   cache, IDs, Undo e Bake existentes.

## 9. Gates de encerramento

Os resultados são preenchidos somente após o conjunto completo, conforme a
regra do usuário: format, check, testes relevantes/workspace, clippy,
`docs-check`, `bible-check` e `ui-guard --strict`.
