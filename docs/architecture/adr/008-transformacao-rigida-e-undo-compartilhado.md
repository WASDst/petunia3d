# ADR 008 — Transformação rígida por objeto em tempo de execução e Undo com dados compartilhados

Status: proposto em 2026-10-04 (implementado no branch `claude/draw-poly-perf-tools`,
aguardando os testes do lote e o aceite do responsável do produto).
Autoridade: [Livro Vivo, capítulo 19](../../bible/foundations/19-concorrencia-memoria-caches-performance.md)
(caches e memória), [capítulo 46](../../bible/foundations/46-pesquisa-interacao-modelagem-referencias.md)
(orçamento de 16,7 ms até 50 mil triângulos) e [ADR 007](007-workspaces-draw-poly-e-gramatica-unica.md).
Este ADR registra o motivo; não cria uma segunda baseline.

## Contexto

A auditoria de 2026-10-04 de DRAW/POLY encontrou dois custos estruturais:

1. **Sem matriz por objeto.** `Asset` guarda os vértices já em espaço de mundo
   (os campos `position/rotation/scale` são metadados). Mover um objeto
   reescrevia todos os vértices a cada evento, clonando e validando a malha,
   e o renderer triangulava o objeto de novo por quadro.
2. **Undo por snapshot inteiro.** `UndoStack<Project>` clonava o projeto
   inteiro (malhas e texturas) a cada operação; o mesmo valia para o início de
   cada operação modal e de cada gesto de perfil. Numa cena pesada, poucas
   entradas cabiam no orçamento de 256 MiB e cada confirmação custava
   O(tamanho do projeto).

Tornar a transformação um campo persistente (vértices locais + matriz) mudaria
o formato `.petunia` — inclusive o layout posicional do postcard legado — e
todos os pontos que leem `asset.mesh` em mundo (booleanos, export, pintura,
UV, regiões, medidas). O risco não se justifica na V1.

## Decisão

1. **Matriz rígida em tempo de execução, não persistida.** Durante Move,
   Rotate e Scale no modo objeto, depois da prévia exata do primeiro evento,
   o `core` escreve as posições direto (sem clonar nem validar a malha) e
   publica `AppState::rigid_preview_transforms()` — a matriz original → atual de
   cada objeto em movimento. O renderer WGPU guarda a geometria triangulada no
   início do gesto e só a multiplica pela matriz relativa (posições e normais
   pela inversa-transposta). Os vértices do documento continuam corretos a
   cada evento, então overlays, picking, snap, o backend GL e o viewport por
   software não precisam conhecer a matriz.
2. **`petunia_project::Shared<T>`** (cópia na escrita, `Arc` +
   `Arc::make_mut`) em `Asset.mesh` e `Canvas.pixels`. Clonar o projeto passa a
   custar contadores de referência; a primeira escrita copia só o dado
   alterado, e objetos ou texturas intocados continuam compartilhados entre
   o documento e todos os snapshots. A serialização é transparente: o formato
   do arquivo não muda.
3. Com modifiers, a malha avaliada fica em cache a cada mudança do documento e
   `evaluated_mesh_ref` a empresta em vez de clonar por chamada.

## Consequências

- O custo por evento de mover objetos deixa de incluir clonar, validar e
  triangular; o de confirmar uma operação deixa de copiar o projeto inteiro.
- O orçamento de bytes do histórico mede bytes únicos: `UndoStack` aceita uma
  pegada (`with_footprint`) e `Project::history_footprint` identifica cada
  malha e cada bloco de pixels em `Shared<T>` pelo endereço, contando uma vez
  o bloco presente em vários snapshots; o restante do projeto conta por
  snapshot. Assim, com o mesmo orçamento, cabem muito mais passos quando as
  edições tocam só uma parte da cena.
- Uma matriz persistida por objeto continua possível no futuro, com migração
  de formato própria e um ADR novo.
