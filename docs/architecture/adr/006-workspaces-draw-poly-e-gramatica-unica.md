# ADR 006 — Workspaces DRAW e POLY e gramática única de ferramenta

Status: aceito em 2026-09-29 por autorização explícita do responsável do produto.
Autoridade: [Livro Vivo, capítulo 36](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md)
(revisão de workspaces de 2026-09-29) e [constituição 11](../../bible/constitution/11-contrato-de-mesh-selection-tools-e-undo.md).
Pesquisa de apoio: [capítulo 45](../../bible/foundations/45-pesquisa-interacao-modelagem-referencias.md).
Este ADR registra o motivo; não cria uma segunda baseline.

## Contexto

O responsável do produto avaliou que seguir a filosofia do Blender deixou a
modelagem poligonal inconsistente e a modelagem por desenho ainda pior. A
auditoria de 2026-09-29 (capítulo 45, seção 2) confirmou:

- a documentação prática tratava `Blender.svg` como referência visual canônica e
  o manual do Blender como referência de interação;
- não existe camada comum de ferramenta: cinco mecanismos de sessão com regras
  diferentes de confirmar, cancelar, digitar valores e navegar;
- o desenho não permite orbitar, digitar valores, inferir alinhamentos nem
  reeditar o perfil, e gera volume como asset separado;
- o viewport renderiza em pixels lógicos, sem antisserrilhado, e refaz trabalho
  proporcional à cena inteira a cada movimento do mouse.

A pesquisa comparou Plasticity, Cinema 4D, Modo e SketchUp e a literatura de
manipulação direta, snapping, esboço e acessibilidade motora.

## Decisão

1. **Workspaces `DRAW` e `POLY` substituem `MODEL`.** O conjunto V1 passa a ser
   `DRAW / POLY / PAINT / UV`.
   - `DRAW` trabalha no nível de forma: perfis, regiões, volumes paramétricos,
     desenho sobre faces e Push/Pull.
   - `POLY` trabalha no nível de componente: `Point / Edge / Face`.
   - Os dois compartilham documento, seleção de objeto, câmera, snapping,
     gramática de ferramenta e o Inspector (`Parts → Transform → Material →
     Object`). Somente o trilho esquerdo de ferramentas, o conjunto de seleção e
     a aparência do viewport mudam.
   - Passar de uma forma de DRAW para edição em POLY é explícito e reversível
     ("Converter em polígonos"). Nada é convertido em silêncio.
   - Enquanto DRAW e POLY não estiverem funcionais, a UI continua mostrando o
     workspace atual (regra "workspace não implementado não aparece").
2. **Gramática única de ferramenta (`ToolSession`).** Todas as ferramentas
   seguem o mesmo ciclo:
   - ferramenta persistente: permanece ativa depois de cada uso;
   - clique sem movimento seleciona em qualquer ferramenta; arrasto além do
     limiar executa a ferramenta, pela alça ou em qualquer lugar do viewport
     ("haul");
   - cada gesto é uma transação; o valor continua ajustável no card "Última
     operação" até outra mudança no documento, sem criar entradas extras de Undo;
   - valor digitado durante o gesto (vence o mouse); depois do gesto, ajuste
     no campo do card "Última operação" — digitar fora de gesto não captura
     teclas, para não criar modo oculto;
   - **clicar-mover-clicar** como alternativa a todo arrasto (WCAG 2.2, 2.5.7) e
     limiar de arrasto configurável;
   - navegação sempre disponível durante qualquer ferramenta; a roda sempre faz
     zoom;
   - **RMB sempre abre menu de contexto; `Esc` cancela** em escada (arrasto →
     ferramenta → Select); `Enter` confirma o que estiver pendente.
3. **Modal estilo Blender** (tecla → segue o mouse → clique) passa a ser um
   estado (`Latched`) da mesma máquina, oferecido no perfil de teclas Blender e
   pela preferência de acessibilidade "arrastar sem segurar". Não é o padrão.
4. **Referências:** `Blender.svg` e o manual do Blender deixam de ser
   referências canônicas. A referência visual é o próprio sistema Petunia
   (capítulos 23, 24 e 36); as referências de interação estão no capítulo 45.

## Consequências

- Revisados no caderno: capítulos 01, 02, 05, 23, 36; constituição 03 e 11;
  P3D-015, 040, 073, 075, 076, 083, 092, 095 e 131.
- O `Tab` deixa de alternar domínio de seleção; volta a navegar controles.
- A contradição "RMB cancela" (P3D-131) × "RMB = menu" (capítulo 36) fica
  resolvida a favor do menu.
- ADRs 003, 004 e 005 continuam válidos; onde dizem "MODEL", passam a valer
  para DRAW e POLY.
- Implementação em ondas, cada uma com matriz de lacunas própria:
  1. viewport nítido e fluido + bugs comprovados;
  2. núcleo `ToolSession` e migração de Move/Rotate/Scale, Extrude, Inset,
     Round Edge e Push/Pull;
  3. picking, pré-seleção, snapping com inferência e plano de trabalho;
  4. workspace DRAW;
  5. workspace POLY;
  6. visual premium, preferências de acessibilidade e testes com usuários.
- Até a conclusão das ondas, o código diverge desta decisão; as páginas
  revisadas marcam "decisão aprovada, implementação pendente".

## Critérios de aceite

- Nenhuma ferramenta migrada tem ciclo próprio: começar, ajustar, digitar,
  confirmar, cancelar e navegar funcionam igual em todas.
- Um gesto confirmado gera exatamente uma entrada de Undo; ajustar a "Última
  operação" não aumenta o histórico; `Esc` restaura o estado exato.
- Toda operação por arrasto também funciona por clicar-mover-clicar e por valor
  digitado; alvos de alça têm pelo menos 24×24 px lógicos.
- RMB nunca cancela; orbitar, deslocar e aproximar funcionam durante qualquer
  ferramenta.
- A pill DRAW ou POLY só aparece quando o workspace estiver funcional.
- Gates de CI, capturas nativas em escala 100% e 150% e registro na matriz de
  lacunas; nenhum item vira COMPLIANT sem evidência.
