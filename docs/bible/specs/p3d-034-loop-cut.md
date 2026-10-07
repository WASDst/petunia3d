# P3D-034 — Loop Cut

<aside>
🧩

Estado: **parcialmente implementado; hover e seção pós-booleana, integração de 2026-10-07; QA nativo pendente** · Prioridade: P1.

</aside>

## Objetivo

Inserir edge loops em topologia compatível sem prometer resultado onde não há caminho de loop válido.

## Auditoria

Validar detecção do loop, preview, posição/slide se existir, seleção resultante e undo.

## Regras

Em topologia incompatível, ação fica disabled ou retorna erro/hint claro; não “chuta” um corte.

## Interação da UI Slint

Escolher Loop Cut arma a ferramenta. Passar o ponteiro sobre uma aresta ou face
de um quad ring elegível desenha uma prévia sem mutar a malha. A roda faz zoom;
Ctrl+roda ou os controles ajustam `Cuts` (1–32), conforme constituição 11.
Clique inicia a sessão transacional para slide; Enter confirma, Esc cancela.

Resultados booleanos podem conter triângulos e n-gons sem um percurso de
quads. Em uma superfície fechada, conectada e manifold, o contrato de
compatibilidade admite uma **seção perpendicular à aresta escolhida**: a prévia
explicita o plano e o slide, e a confirmação insere loops de seção na superfície,
sem tampa interna. Esse caminho triangula o snapshot, preserva UVs e materiais
e produz uma entrada de Undo. Não infere um ring arbitrário entre poles.

Superfícies abertas sem ring de quads, componentes desconectados e topologia
não manifold continuam incompatíveis; não produzem um corte parcial.
O preview nativo ainda precisa de QA visual e comparação de
oclusão/topologias complexas.

## Dependências

P3D-018, P3D-041, P3D-123.

## Testes / DoD

Quads regulares, boundaries, poles/ngons, cancel e regressão. Fuse, Cut e
Intersection: prévia sem mutação, seção fechada/manifold, slide, UVs/materiais,
seleção resultante, um Undo e recusa de snapshot obsoleto.
