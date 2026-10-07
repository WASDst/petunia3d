# Retomada unificada — reconciliação e correções de POLY (07/10/2026)

Autoridade: AGENTS.md, P3D-034, P3D-048, capítulos 23/36 e ADRs 004/005/007.
Pedido: reunir trabalho pendente em uma worktree e retomar incrementalmente;
primeiro corrigir Loop Cut horizontal e recolher/restaurar o Inspector sem
sobrepor ou bloquear os ícones.

Worktree de integração: `codex/unified-gradual-recovery`, baseada em `ae81c9a`.
As sete árvores alteradas foram preservadas antes da integração em patches
binários e arquivos dos itens não rastreados, com manifesto e hashes, fora do
repositório e numa fila portátil dentro desta worktree. Nenhuma worktree de origem é descartada durante a reconciliação.

## Implementation-vs-Spec Gap Matrix

| Requisito | Estado inicial | Evidência / delta | Aceite |
|---|---|---|---|
| Loop Cut em ambas as orientações | PARTIALLY_COMPLIANT | Anéis de quads e cortes perpendiculares sucessivos já passam no kernel. Hover limita a face a quatro cantos e o fallback tenta só a aresta mais próxima; triângulos/n-gons interrompem a direção que cruza os caps. | Prévia, clique, slide, cancel e um Undo em cortes horizontais e verticais, inclusive após um corte anterior. |
| Inspector recolhe e reabre por clique | BROKEN (relato reproduzível a validar) | Shell F4 oculta o rail pelo estado de hover que o próprio rail dispara. O hover também altera a posição da superfície sob o ponteiro; teste com relógio controlado reproduziu sobreposição durante a animação em Parts. | Eventos reais de ponteiro: recolher, reabrir pelas pílulas, trocar seção e repetir. |
| Ícones acessíveis sem sobreposição | BROKEN (relato reproduzível a validar) | Rail fica na mesma borda ocupada pelo flyout; opacidade sem desativação explícita pode deixar hit targets invisíveis. | Alvos com retângulos disjuntos no peek, sem interceptação por painel oculto, larguras 1280 e 1024 px, movimento normal e reduzido. |
| UI F2–F8/F0 na mesma base dos PRs #32–34 | PARTIALLY_COMPLIANT | Cinco commits isolados, 30 commits da main ausentes; 9 arquivos conflitantes. | Preservar ToolSession, caches, cursor/canvas, decalques de superfície e variantes; compilação, testes e gates. |
| PAINT: máscaras, grupos e biblioteca | PARTIALLY_COMPLIANT, local | Código e testes em outra worktree; base anterior ao PR #34. | Composição unificada, projetos antigos, Undo e shell; entrega independente após os bugs POLY. |
| Materiais/canais W1 | PARTIALLY_COMPLIANT, local | Renderer/domínio escritos; UI/export ainda pendentes. | Canal + máscara no mesmo esquema; fechar controles, mapas e round-trip. |
| Patches DRAW / Parts planares / contratos | PARTIALLY_COMPLIANT, local | Árvores antigas e recursos parcialmente recuperados na main. | Incorporar somente delta não coberto, preservar dados e adaptar goldens. |
| SVG/Projection/Path Paint recuperados | DUPLICATED | Equivalentes presentes na main, com correções posteriores. | Registrar equivalência; não reaplicar versões antigas. |
| Features egui isoladas | OBSOLETE para produção | Frontend de produção é Slint; expansão egui proibida. | Preservar histórico, sem transportar novas superfícies para o legado. |

## Processo de entrega

1. Confirmar requisito no caderno e classificar estado com evidência de código.
2. Reproduzir a falha com evento/teste pertinente antes da correção.
3. Aplicar delta pequeno, preservando partes conformes e transações.
4. Executar regressões, fmt/check/clippy e guards pertinentes.
5. Registrar resultado, limitações e aceite visual pendente; commit e push.
6. Verificar SHA remoto e atualizar a fila antes de iniciar a próxima etapa.

Não promover item a concluído por existir código ou por passar apenas compilação.
Não reconstruir o site congelado. Alterações de persistência exigem cobertura
combinada de campos antigos e novos antes de publicação como entrega validada.

## Fila inicial

POLY Loop Cut + Inspector → integração UI commitada → PAINT local → canais W1
→ deltas DRAW/Parts e contratos → fechar W1 → W2 → W3 → W4/W5 → Animate
→ plugins/automação. Bugs confirmados têm prioridade dentro de cada etapa.

## Validação

- Malha: `cargo test -p petunia_mesh --lib`: **186 aprovados**, zero falhas. Inclui direções perpendiculares sucessivas, cilindro, cubo triangulado e seções Union/Difference/Intersection, sem perder material, UV ou fechamento.
- `cargo fmt --all -- --check`, `git diff --check`, `docs-check`, `bible-check`, `arch-check`, `ui-lint` e `ui-guard --strict`: aprovados na árvore reconciliada. O site congelado foi preservado (813 arquivos); avisos do mapa/guard legado não são aceite visual Slint.
- Shell: **508 testes unitários + 11 de gestos + 1 de métricas aprovados**. O Loop Cut mantém a orientação do hover ao clique nos callbacks de produção e confirma uma única entrada de Undo. O Inspector passou recolher/reabrir Parts, Transform, Material e Object em 1280/1024 px com movimento normal/reduzido.
- Regressão comprovada do Inspector: com o relógio avançado 40 ms, o teste falhava porque o flyout sobrepunha Parts durante a animação. Depois da correção de posição e rail, passou junto com a suíte inteira; o fade permanece.
- Integração da malha: mais **2 testes**, cobrindo cubo/cilindro/cone/esfera/toro nas duas famílias e Union/Difference/Intersection nos dois eixos, aprovados.
- Configuração: **23 testes aprovados**, incluindo locale parity, catálogo, preferências e contraste. Projeto: **222 aprovados**, incluindo persistência, decalques e exportação.
- Métricas: DRAW **45**, POLY **45**, PAINT **40** controles em 1800×1012; POLY respeita o limite de 45.
- `cargo check -p petunia3d -p petunia_ui_slint -p petunia_mesh -p petunia_config --all-targets`: aprovado, incluindo o host e a galeria.
- Clippy da malha/configuração e Slint (`--all-targets`) aprovado com `-D warnings`. O fixture de callbacks usa a mesma exceção local de Arc do host, pois a imagem Slint permanece na thread da UI.
- Guias en/pt-BR sincronizados para superfície/aresta compatível e Ctrl+roda. Rerun final: 508 + 11 + 1 aprovados, zero falhas; guards e whitespace/fmt aprovados. Total das baterias pertinentes: **953 testes aprovados**.
- QA nativo com GPU, oclusão e sólidos complexos permanece pendente. Backend headless não substitui esse aceite.

## Consolidação de histórico e limite do checkpoint

Os cinco commits exclusivos de Slint (`2a047d9`) e o planejamento/referências
commitados em `d8aeb0b` estão reconciliados com `ae81c9a`. Os arquivos do plano
estão ativos nesta worktree; os deltas WIP das sete árvores ficam na fila
portátil com hashes originais e dos arquivos comprimidos. Nenhuma fonte foi
apagada nem nenhum WIP promovido automaticamente a feature disponível.

As duas regressões POLY têm implementação e cobertura automatizada aprovada.
P3D-034 permanece PARTIALLY_COMPLIANT para o DoD completo enquanto faltarem QA
nativo de oclusão/topologias complexas e captura com GPU. Os controles do
Inspector foram verificados por eventos de ponteiro e bounds, incluindo 40 ms
no meio da transição; o aceite visual nativo geral permanece pendente.
