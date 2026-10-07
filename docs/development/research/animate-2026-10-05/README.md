# Pesquisa — animação procedural para o Animate (05/10/2026)

Pedido do responsável do produto: estudar a fundo o Dust3D, outros softwares de
animação e de animação procedural, e o repositório UniMate, para orientar o
algoritmo e a funcionalidade do workspace Animate **sem depender de modelo de
IA em tempo de execução**.

Este documento é **pesquisa** (rationale), não requisito: pela regra do cap. 13,
pesquisa não supera decisão normativa. As decisões continuam no
[cap. 45](../../../bible/foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)
e nas specs P3D-169 a P3D-174. Quando o Animate voltar à pauta, os achados abaixo
devem ser reconciliados lá.

Prioridade vigente (05/10/2026): o Animate só volta depois do pacote de waves de
modelagem e superfície ([plano](../../surface-modeling-waves-2026-10-05.md)).

## Material guardado

| Pasta | Conteúdo | Versionado? |
| --- | --- | --- |
| [`sources/dust3d/`](sources/dust3d/) | 17 arquivos do Dust3D (gait, pose sequence, ações, catálogo, secundário, eventos, rig, templates XML) e o `LICENSE` (MIT) | Sim (MIT, aviso preservado) |
| [`sources/unimate/`](sources/unimate/) | README do projeto e do `rig_preprocess`, `review.py`, `annotate.py` e o `LICENSE` (MIT) | Sim (MIT, aviso preservado) |
| `materials-local/` | PDFs: Hecker et al. 2008 (Spore) e Mou et al. 2026 (UniMate, arXiv 2609.05415) | **Não** — o repositório é público e os PDFs têm copyright; ficam só na máquina local (ver `.gitignore`) |

Para recuperar os PDFs em outra máquina, use os links da seção Fontes.

Os arquivos em `sources/` foram copiados de `master` em 05/10/2026 e são
**referência de leitura**. Portar código para o Petunia exige preservar o aviso MIT
do projeto de origem; a adoção prevista é de **ideias e estruturas**, reescritas
sobre o Rig Core do Petunia.

## Resumo

- O caminho mais promissor combina três fontes:
  - **Dust3D:** estilos de movimento como structs de parâmetros nomeados, em "comprimentos de perna";
  - **Spore:** seleção de partes por papel e modos de movimento relativos ao chão ou a outra parte;
  - **UniMate:** pré-processamento do rig (frente do modelo, rótulos, revisão com alertas).
- O modelo de IA do UniMate **não** é aproveitável. O pré-processamento dele é.
- Uma lacuna de produto apareceu: **animação de partes rígidas** (porta, baú, robô),
  sem skinning, como no Blockbench. Ela casa com P3D-166 (hierarquia de partes) e
  com o fluxo de pivô/dobradiça do cap. 43.

## 1. Dust3D (MIT, ativo em 10/2026)

O cap. 45 analisou só o `hurt.cc` da release 1.0.0. O repositório atual tem cerca de
80 clipes bípedes e de 5 a 10 por criatura (quadrúpede, aranha, inseto, serpente,
peixe, pássaro).

| Peça | Como funciona | Arquivo |
| --- | --- | --- |
| Estilo de locomoção | `GaitStyle`: ~40 parâmetros nomeados (fração do ciclo no chão, altura do passo, balanço e giro do quadril, contra-rotação do peito, balanço dos braços, cauda, cabelo). Distâncias em **comprimentos de perna**; tempos em fração do ciclo. Walk e Run preenchem o mesmo struct | `dust3d_animation_biped_gait.h` / `.cc` |
| Pose-alvo por tempo | `PoseSampler: t → PoseSample` (quadril, pitch/yaw/roll, peito e cabeça, alvos de tornozelo, direção dos joelhos, braços relativos ao peito, à cabeça ou ao chão, flags de contato). Um solver comum converte em ossos por IK | `dust3d_animation_biped_pose_sequence.h` |
| Ações como dados | `ActionStyle { kind, duração, amostras }` + `animateAction`: envelopes de fase com `easePose` (ex.: preparar 0–22 %, golpear 22–38 %), referencial cima/frente/esquerda, pernas por IK de dois ossos, braços por direções no espaço do peito com peso, intensidade como parâmetro | `dust3d_animation_biped_action.h` / `.cc` |
| Catálogo | cada clipe declara tipo, duração, amostras, loop, **pose de entrada e de saída** (para encadear) e a função geradora | `dust3d_animation_biped_clip_catalog.h` |
| Clipe exportado | quadros com transformações e matrizes de skin, **eventos** (nome, tempo, osso), root motion `inPlace` com velocidade e direção de movimento, yaw final para clipes de giro | `dust3d_animation_animation_generator.h` |
| IK | analítico de dois ossos com pole vector e **soft IK** exponencial perto da extensão total | `dust3d_animation_common.h` |
| Secundário | Verlet com subpassos (até 240, ~60 Hz); loops "aquecem" ciclos completos antes de gravar; ações assentam na pose inicial | `dust3d_animation_biped_secondary_motion.cc` |
| Eventos | detectados pela cinemática: contato do pé (altura/velocidade), soltura da mão (pico de desaceleração), mão assentando, impacto do corpo | `dust3d_animation_sound_event_detector.h` |
| Rig | o usuário dá nome de osso a **arestas do grafo de modelagem**; o osso sai das cadeias de nós e os pesos vêm do nó que gerou cada vértice; pés no chão (Y = 0); pálpebras geradas | `dust3d_rig_rig_generator.h`, `application_resources_rig_*.xml` |

**Adotar:** estilos com parâmetros nomeados e unidades relativas; ações como tabela
de fases e alvos (não código por ação); catálogo com poses de entrada e saída;
eventos exportados; aquecimento de ciclo no secundário.

**Não adotar:** contrato por **nome** fixo de osso (o Petunia usa papéis, P3D-169);
uma função por ação codificada à mão quando um dado basta. A rigagem por nós não se
aplica (o Petunia não tem grafo de modelagem); o equivalente proposto é marcar
**cadeias de loops de arestas**: o osso passa pelos centros dos anéis e os vértices
de cada anel herdam o peso.

## 2. Spore — Hecker et al., SIGGRAPH 2008

"Real-time Motion Retargeting to Highly Varied User-Created Morphologies". É a
referência mais próxima do problema do Petunia: animar criaturas cuja forma não é
conhecida quando a animação é feita. Lido diretamente no PDF (seções 3 e 4).

| Mecanismo | Descrição |
| --- | --- |
| Consulta de contexto | o animador **descreve** quais partes animar: tipo (*grasper*, *mouth*, *foot*, *spine*, *root*), filtros espaciais (Front/Center/Back, Left/Center/Right, Top/Center/Bottom, relativos ao personagem ou ao conjunto), extremos (FrontMost…) e o modificador *SpineSegment* (sobe até ombro/quadril) |
| Modos de movimento | absoluto; relativo ao repouso; escala por tamanho da criatura ou **comprimento do membro**; **relativo ao chão** (0 = repouso, 1 = chão); **relativo a outra parte** ou a um alvo externo ("mão na boca", "bater palmas", "pegar a fruta"); look-at |
| Generalização | as chaves são guardadas em coordenadas generalizadas; a mesma animação é vista em **vários personagens ao mesmo tempo** durante a edição |
| Ramificação | predicados de três valores (UprightSpine, HasGraspers, HasFeet) limitam a que criaturas a animação se aplica |
| Variantes | uma variante por parte escolhida (qual garra pega) e **espelho sagital** automático |
| Gait N pernas | perna = caminho do pé até a coluna; pernas agrupadas por comprimento, frequências harmonizadas por razões pequenas; *duty factor* e *step trigger* por pé; arco do passo normalizado e escalado pelo comprimento; estilos interpolados pela velocidade e **empilháveis** (mancar); criaturas sem pés rastejam com pseudo-pés |
| IK por partículas | partículas e restrições de comprimento (Jakobsen), duas fases (coluna → membros), coluna por spline quíntica, alvos delegados (a boca move a coluna), falha graciosa, **sem dependência de quadros anteriores** (determinístico) |

**Adotar:** seleção por papel com filtros espaciais; modos relativo ao chão e a outra
parte; prévia em vários rigs; predicados explícitos (já é o "não suportado, com
motivo" do catálogo); gait por grupos de comprimento; coluna por spline e alvos
delegados.

## 3. UniMate (MIT, SIGGRAPH Asia 2026)

Modelo de geração texto → movimento para esqueletos diversos (flow matching sobre
13.769 clipes em 7.430 esqueletos). **O modelo não serve ao Petunia** (D3 do cap. 45:
sem ML por enquanto; exige GPU e dados). O conjunto de dados inclui o Truebones ZOO,
que é comercial e não redistribuível.

O pré-processamento (`rig_preprocess`) é aproveitável sem IA:

| Ideia | Uso no Petunia |
| --- | --- |
| **Facing pair** (as coxas; cabeça/cauda para serpentes e peixes) define a frente | resolve "frente × costas não é inferível" do Fit to model (AN-21) |
| Canonicalização: Y-up, frente +Z, centrado, escala, no chão | normalizar rigs importados antes de aplicar Motions |
| Rotulagem por regras: vocabulário, lados, cadeias de membros, numeração de dedos, rótulos geométricos, correspondência de **árvore de ossos** com templates (Mixamo, Biped, CAT, Rigify) | inferência de Rig Roles (P3D-169) e retarget (P3D-138) |
| Poda de ossos auxiliares, estáticos e pontas (`_End`, `Nub`) | importação de GLB com rig |
| **Revisão com alertas**: rótulo genérico, fora do vocabulário, membros esquerda/direita do lado oposto ao facing pair, pose de cabeça para baixo | painel "Roles" com correção guiada (P3D-137: "indicar falhas, permitir correção antes do bind") |
| Categoria "objeto articulado rígido" | animação de partes rígidas |

## 4. Outras referências

| Fonte | Ideia útil |
| --- | --- |
| Blockbench | **grupo = osso com peso 1** (animação rígida de partes); operações em lote nas chaves (deslocar, escalar, inverter, espelhar); canal de som; marcadores |
| Cascadeur (parte sem ML) | centro de massa com trajetória suave; **arco balístico** no ar; pontos de apoio detectados por raio; área de equilíbrio. Útil como diagnóstico opcional |
| Overgrowth (Rosen, GDC 2014) | poucas poses-chave (ex.: *pass* e *reach* da corrida) misturadas pela distância percorrida |
| runevision (2021–2024) | distância em que o pé sai do chão é quase constante entre velocidades; duração do voo do pé pela passada relativa ao comprimento da perna; parametrização manual venceu PCA |

## 5. Síntese para o algoritmo do Animate

| Ideia | Fonte | Onde entra | Trab. (1–10) | Filosofia |
| --- | --- | --- | --- | --- |
| Referencial do rig (cima, frente pelo facing pair) e unidade "comprimento de perna" | Dust3D, UniMate | P3D-169 / P3D-137 | 3 | Alta |
| Revisão de rig com alertas | UniMate | P3D-137 / P3D-169 | 4 | Alta |
| Marcar ossos por cadeias de loops de arestas | Dust3D (equivalente) | P3D-137 | 5 | Alta |
| Rig de partes rígidas e dobradiça | Blockbench, UniMate | P3D-166 + cap. 43 | 4 | Alta |
| Estilos de movimento como structs de parâmetros nomeados | Dust3D | P3D-170 | 4 | Alta |
| Ações como tabela de fases e alvos | Dust3D | P3D-170 (Reaction, Action) | 5 | Alta |
| Seleção por papel e modos de movimento | Spore | P3D-170 / P3D-174 | 6 | Alta |
| Gait N pernas com grupos por comprimento e rastejar | Spore | P3D-170 | 5 | Alta |
| Coluna por spline e alvos delegados no IK | Spore | P3D-169 | 5 | Alta |
| Eventos de passo e impacto exportados no glTF | Dust3D | AN-13 / P3D-144 | 3 | Alta |
| Poses de entrada e saída para encadear clipes | Dust3D | P3D-139 | 3 | Alta |
| Diagnóstico de física (centro de massa, arco balístico) | Cascadeur | opcional | 5 | Média |

## Fontes

- Dust3D — <https://github.com/huxingyi/dust3d>
- UniMate — <https://github.com/Friedrich-M/UniMate>; artigo <https://arxiv.org/abs/2609.05415>
- Hecker et al. 2008 — <https://www.chrishecker.com/Real-time_Motion_Retargeting_to_Highly_Varied_User-Created_Morphologies>; DOI <https://doi.org/10.1145/1360612.1360626>
- Rosen, GDC 2014 — <https://www.gdcvault.com/play/1020583/Animation-Bootcamp-An-Indie-Approach>
- runevision — <https://blog.runevision.com/2025/01/procedural-creature-progress-2021-2024.html>
- Blockbench — <https://blockbench.net/wiki/guides/blockbench-overview-tips/>
- Cascadeur — <https://cascadeur.com/help/tools/physics_tools/autophysics>, <https://cascadeur.com/help/tools/physics_tools/ballistic_trajectory>

Nota da integração de 07/10/2026: espaços finais em quatro trechos XML/C++ foram normalizados para o gate de whitespace; conteúdo e avisos de licença foram preservados. O original permanece no commit `d8aeb0b`.
