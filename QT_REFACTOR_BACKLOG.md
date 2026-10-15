# Backlog de Refatoração da UI Qt (Petunia3D)

> **Documento de Acompanhamento Contínuo — Branch `codex/qt-integration`**  
> Criado em: 2026-10-15  
> Regra de governança: Toda nova correção, melhoria solicitada ou refinamento de interface solicitada pelo usuário deve ser registrada nesta tabela com ID sequencial (`QT-XX`), status (`[ ]` Pendente, `[-]` Em Progresso, `[x]` Concluído), categoria, descrição detalhada e arquivos afetados.

---

## 1. Tabela Mestra de Features e Refatorações

| ID    | Check | Feature / Tarefa                                | Categoria      | Descrição & Critérios de Aceite                                                                                                                                           | Arquivos Afetados                                            |
|-------|:-----:|-------------------------------------------------|----------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------|
| QT-00 |  [x]  | Transforms compactos em linha única (XYZ)       | Ergonomia / UI | Posição, Rotação, Escala, Cursor 3D e Poses com campos X, Y, Z agrupados na mesma linha horizontal com tags coloridas (#ff6e6e, #69db7c, #4dabf7).                       | `petunia_editor_panel.h`, `petunia_editor_panel.cpp`         |
| QT-00 |  [x]  | Remoção de caixas/bordas duras de ícones        | Estilo / Tema  | Botões de ferramentas na Tool Rail, barra flutuante inferior e barra de opções superior transparentes e sem bordas opacas cinzas (#292929/#404040).                      | `petunia_main_window.cpp`, `petunia_overlay_cards.cpp`       |
| QT-00 |  [x]  | Barra de ferramentas inferior dinâmica          | Ferramentas    | Context bar inferior adaptativa conforme domínio ativo (Object, Vertex, Edge, Face), exibindo apenas operações contextuais relevantes.                                    | `petunia_overlay_cards.h`, `petunia_overlay_cards.cpp`       |
| QT-00 |  [x]  | Densidade compacta no Outliner (Parts)          | Outliner / UI  | Altura de linha reduzida para 22px, ícones 14px, colunas de visibilidade/lock em 20px e slider de densidade 18-36px.                                                      | `petunia_main_window.h`, `petunia_main_window.cpp`           |
| QT-00 |  [x]  | Top Header Bar unificado                        | Layout / Shell | Faixa superior única: [Logo Petunia] [Menus] [Workspaces Centralizados] [Desfazer/Refazer] [Busca Ctrl+K] [Configurações].                                                | `petunia_main_window.cpp`                                    |
| QT-01 |  [x]  | Barra lateral esquerda focada e limpa           | Tool Rail      | Conter estritamente: Seleção (Box/Lasso), Transforms (Move/Rotate/Scale), Cursor 3D, Medidas e Primitivas (popup). Ferramentas de modelagem movidas para barra flutuante. | `petunia_main_window.cpp`                                    |
| QT-02 |  [x]  | Feedbacks visuais na viewport e gizmo moderno   | Viewport / GL  | Realce nítido de hover/seleção em vértices, arestas e faces; gizmo de transformação redesenhado com eixos RGB limpos, flechas sólidas e planos visuais XY/XZ/YZ.         | `opengl_viewport.rs`, `petunia_gl_widget.cpp`                |
| QT-03 |  [x]  | Renderização e manipulação do 3D Cursor na tela | Viewport / 3D  | Desenhar retículo visual do 3D Cursor no viewport; permitir posicionamento interativo com clique/arrasto no espaço 3D quando a ferramenta Cursor 3D estiver ativa.        | `opengl_viewport.rs`, `petunia_gl_widget.cpp`                |
| QT-04 |  [x]  | Poly Pen em projeção ortográfica e planos       | Modelagem / GL | Corrigir unproject/raycast da Poly Pen nas vistas ortográficas (Front, Top, Right, etc.) e em planos específicos de projeção.                                            | `petunia_gl_widget.cpp`, `opengl_viewport.rs`                |
| QT-05 |  [x]  | Redesenho do controle de Pivot no Inspector     | Inspector / UI | Layout compacto [Dropdown Origem Pivot] [Reset Pivot]. Distinção formal estrita: Pivot pertence ao objeto; 3D Cursor pertence ao mundo.                                  | `petunia_editor_panel.cpp`                                   |
| QT-06 |  [x]  | Remoção do 3D Cursor do Transform de Objeto     | Inspector / UI | Eliminar campos/botões de 3D Cursor da aba Transform do objeto no Inspector (mantendo como ferramenta global independente).                                               | `petunia_editor_panel.cpp`                                   |
| QT-07 |  [x]  | Sistema de docking e janelas flutuantes livres  | Docking / UX   | Abas destacadas devem ser janelas flutuantes normais no SO (não travadas), com redocking flexível, reposicionamento livre e agrupamento arbitrário em abas.              | `petunia_editor_panel.cpp`, `petunia_main_window.cpp`        |
| QT-08 |  [x]  | Primitivas como submenu popup na Tool Rail      | Ferramentas    | Remover aba "Primitive" do Inspector; botão de primitiva na barra lateral abre submenu popup instantâneo para inserção de Cube, Cylinder, Sphere, Plane, etc.            | `petunia_main_window.cpp`, `petunia_editor_panel.cpp`        |
| QT-09 |  [x]  | Remoção do painel View do Inspector             | Inspector / UI | Remover aba "View" do Inspector; todas as configurações de visualização e sombreamento (Wireframe, Solid, MatCap, X-Ray) consolidadas na barra superior da viewport.     | `petunia_editor_panel.cpp`, `petunia_overlay_cards.cpp`      |
| QT-10 |  [x]  | Revisão e validação de Draw Operator e Profile  | DRAW Workspace | Auditar paridade, funcionamento e persistência dos operadores de DRAW, perfis de curva, extrusões de perfil e simetria.                                                   | `editor.rs`, `petunia_editor_panel.cpp`                      |
| QT-11 |  [x]  | Suporte a cena vazia (Cubo padrão apagável)     | Cena / Modelo  | Permitir deletar o cubo inicial sem restrições, mantendo a cena funcional com 0 objetos (sem crash, com feedback "No Selection").                                         | `state.rs`, `editor.rs`, `petunia_main_window.cpp`           |
| QT-12 |  [x]  | Aprimoramentos no Outliner (Parts)              | Outliner       | Ícones desabilitados escurecidos (dimmed); botão/ação Isolar (Solo); Tag de cor por parte; multi-seleção com Ctrl/Shift; drag-and-drop para pastas e hierarquias.        | `petunia_main_window.cpp`                                    |
| QT-13 |  [x]  | Unificação da visão 2:1 no seletor de câmera    | Viewport / UI  | Integrar a visão 2:1 junto às opções de projeção de câmera (Persp / Orto / 2:1 Split), em vez de botão isolado avulso.                                                    | `petunia_overlay_cards.cpp`, `petunia_overlay_cards.h`       |
| QT-14 |  [x]  | Controle de sensibilidade de zoom e ajustes     | Preferências   | Acelerar velocidade de aproximação inicial de zoom; adicionar slider de velocidade de zoom; seletor de cor com Color Wheel; atalhos de teclado configuráveis.            | `petunia_gl_widget.cpp`, `petunia_main_window.cpp`           |
| QT-15 |  [x]  | Painéis flutuantes móveis e redimensionáveis    | Janelas / UX   | Painéis como Imagens de Referência, Paleta de Cores e Inspetores auxiliares como janelas flutuantes livres, redimensionáveis e não-bloqueantes.                           | `petunia_main_window.cpp`, `petunia_editor_panel.cpp`        |
| QT-16 |  [x]  | Pie Menu radial com atalho Shift+A              | Ergonomia / UX | Ao pressionar Shift+A na viewport 3D, abrir menu radial/circular na posição do mouse com acesso rápido a criação de primitivas e operadores contextuais.                  | `petunia_gl_widget.cpp`, `petunia_pie_menu.h/.cpp`           |

---

## 2. Detalhamento dos Requisitos e Critérios de Aceite

### QT-01 — Barra lateral esquerda focada e limpa
* **Problema:** A barra vertical esquerda acumulou ferramentas de modelagem (Extrude, Inset, Bevel, Knife, etc.) misturadas com ferramentas de seleção, transformação e navegação.
* **Solução:**
  - Manter exclusivamente: `Select Box`, `Lasso Select`, `Move`, `Rotate`, `Scale`, `3D Cursor`, `Measure`, e o disparador de `Primitives`.
  - Remover ferramentas de modelagem direta (Extrude, Inset, Bevel, Knife, Subdivide, etc.) da barra vertical; elas devem residir unicamente na barra flutuante inferior adaptativa.
* **Critérios de Aceite:** Barra lateral enxuta contendo apenas ferramentas globais de seleção, manipulação espacial, medição e adição.

---

### QT-02 — Feedbacks visuais na viewport e gizmo moderno
* **Problema:** Falta feedback visual claro ao passar o mouse ou selecionar sub-elementos (vértices com ponto discreto demais, arestas sem iluminação de hover, faces sem overlay translúcido colorido). O gizmo de transformação possui estética rústica.
* **Solução:**
  - Implementar highlight dinâmico de hover para vértices (halo circular), arestas (linha iluminada de 3px) e faces (preenchimento suave translúcido com cor de seleção).
  - Modernizar o gizmo de transformação em `crates/ui-cxxqt/src/render/opengl_viewport.rs`: eixos RGB puros com pontas de cone bem desenhadas para translação, anéis com arcos para rotação, cubos nas pontas para escala, além de alças de plano (quadrados XY, XZ, YZ) no centro.
* **Critérios de Aceite:** Manipulação intuitiva com resposta visual em tempo real para cada componente sob o cursor e gizmo esteticamente comparável a DCCs modernos.

---

### QT-03 — Renderização e manipulação do 3D Cursor na tela
* **Problema:** O 3D Cursor não é visível na viewport nem pode ser reposicionado com o mouse.
* **Solução:**
  - Renderizar na viewport o ícone/retículo clássico do 3D Cursor (círculo tracejado vermelho/branco com eixos cruzados) na posição de coordenadas do documento.
  - Ao selecionar a ferramenta `3D Cursor` na barra lateral, cliques e arrastos do botão esquerdo na viewport projetam raycast no plano de grade ou nas faces dos objetos em cena, despachando `set_cursor_3d(x, y, z)`.
* **Critérios de Aceite:** O cursor 3D é visível em qualquer ângulo e interativo via mouse na viewport.

---

### QT-04 — Poly Pen em projeção ortográfica e planos
* **Problema:** A ferramenta Poly Pen só funciona corretamente em perspectiva, falhando ao desenhar em vistas ortográficas ou planos fixos.
* **Solução:**
  - Atualizar o cálculo de `unproject` e interseção de plano para contemplar `projection_matrix` ortográfica sem divisão por perspectiva $w$, projetando o clique no plano de corte da câmera ativa ou na grade correspondente ao plano de visualização (Top = XY/XZ, Front = XZ/XY, Right = YZ).
* **Critérios de Aceite:** Criação de pontos e polígonos funcional em Perspectiva, Top, Front, Right e vistas ortográficas arbitrárias.

---

### QT-05 — Redesenho do controle de Pivot no Inspector
* **Problema:** O controle de pivot possui múltiplos botões espalhados e mistura conceitos de Pivot com 3D Cursor.
* **Solução:**
  - Redesenhar a seção de Pivot no Inspector:
    $$\text{[Dropdown Origem Pivot: Mediana | Base | Cursor 3D | Seleção | Geometria]} \quad \text{[Reset Pivot]}$$
  - Deixar clara a distinção conceitual no código e na interface: **Pivot é local do objeto**; **3D Cursor é uma coordenada global do mundo**.
* **Critérios de Aceite:** Linha compacta com dropdown e botão de reset, desacoplada do 3D Cursor.

---

### QT-06 — Remoção do 3D Cursor do Transform de Objeto
* **Problema:** Os campos e botões de 3D Cursor estavam duplicados dentro do bloco Transform do objeto no Inspector.
* **Solução:**
  - Remover as entradas de 3D Cursor da aba Transform do objeto.
  - O cursor 3D passa a ser gerenciado unicamente pela ferramenta dedicada no Tool Rail e pelo atalho contextual correspondente.
* **Critérios de Aceite:** Nenhuma menção a 3D Cursor dentro do painel Transform do objeto selecionado.

---

### QT-07 — Sistema de docking e janelas flutuantes livres
* **Problema:** Ao destacar uma aba do Inspector, a janela ficava presa ou limitada a redocks automáticos rígidos.
* **Solução:**
  - Configurar as janelas flutuantes (`QDockWidget` ou janela nativa desacoplada) com `Qt::Window` e flags completas (`QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable`).
  - Permitir movimentação livre fora da janela principal, redimensionamento sem travas e capacidade de agrupar em abas (tabbed docking) onde o usuário desejar.
* **Critérios de Aceite:** Abas destacadas podem flutuar livremente em múltiplos monitores e ser reposicionadas ou reagrupadas livremente.

---

### QT-08 — Primitivas como submenu popup na Tool Rail
* **Problema:** A criação de primitivas ocupava uma aba inteira no Inspector.
* **Solução:**
  - Remover a aba "Primitive" do Inspector.
  - Na barra lateral esquerda (Tool Rail), criar botão de ferramenta de Primitivas que, ao ser clicado, abre um menu suspenso (QMenu / Popup com ícones) listando Cube, Cylinder, UV Sphere, Plane, Torus, Cone, etc. Ao clicar no item, o objeto é instanciado na cena e o menu se fecha.
* **Critérios de Aceite:** Inserção rápida de primitivas direto da barra lateral sem poluir o Inspector.

---

### QT-09 — Remoção do painel View do Inspector
* **Problema:** Existência de uma aba "View" no Inspector redundante com a barra de ferramentas de topo da viewport.
* **Solução:**
  - Remover aba "View" do Inspector.
  - Garantir que todos os seletores de visualização (Shading: Wireframe, Solid, MatCap, X-Ray; Câmera: Persp, Orto, etc.) estejam presentes na barra superior de opções da viewport.
* **Critérios de Aceite:** Inspector reservado a propriedades contextuais do objeto e ferramentas; controle de visualização unificado na viewport.

---

### QT-10 — Revisão e validação de Draw Operator e Profile
* **Problema:** Falta clareza e testes de cobertura sobre o comportamento das ferramentas do workspace DRAW e edição de perfis.
* **Solução:**
  - Auditar operadores de perfil (`draw_profile.rs`, comandos de Draw, curvas e extrusões).
  - Validar reatividade dos seletores de modo de desenho e espelhamento/simetria de curvas no Qt.
* **Critérios de Aceite:** Fluxo completo de criação e modificação de perfis operante e sem erros no workspace DRAW.

---

### QT-11 — Suporte a cena vazia (Cubo padrão apagável)
* **Problema:** O cubo inicial é protegido ou não pode ser deletado quando é o único objeto da cena.
* **Solução:**
  - Permitir a deleção do último objeto restante na cena, suportando documento com contagem de objetos igual a zero.
  - Tratar na viewport, outliner e inspector o estado de cena vazia (exibir grade, ocultar propriedades de objeto, exibir texto de apoio "Cena vazia — Pressione Shift+A ou use a barra de ferramentas para adicionar um objeto").
* **Critérios de Aceite:** Deletar o cubo inicial resulta em cena vazia sem crashes ou exceções, permitindo adicionar novos objetos em seguida.

---

### QT-12 — Aprimoramentos no Outliner (Painel de Parts)
* **Problema:** Ícones desabilitados somem completamente; faltam recursos essenciais como Solo/Isolar, marcação de cores, multi-seleção consistente e hierarquia drag-and-drop.
* **Solução:**
  - Manter ícones desabilitados visíveis com opacidade reduzida (`opacity: 0.35` / cor escura).
  - Adicionar ação de "Isolar" (Solo / Shift+H) para focar apenas no objeto selecionado.
  - Implementar Tag de Cor (Color Badge / etiqueta com seletor de paleta para identificação visual rápida de partes).
  - Suportar multi-seleção nativa (`QAbstractItemView::ExtendedSelection`) com Ctrl e Shift.
  - Habilitar Drag & Drop (`dragEnabled: true`, `dropIndicatorShown: true`) para reordenação, agrupamento em pastas e criação de relação pai-filho.
* **Critérios de Aceite:** Outliner completo, responsivo e ergonômico conforme padrões de produção 3D.

---

### QT-13 — Unificação da visão 2:1 no seletor de câmera
* **Problema:** A visão 2:1 é um botão isolado que não reflete seu papel como modo de câmera/projeção.
* **Solução:**
  - Integrar o modo 2:1 em um seletor agrupado de projeção na barra de opções da viewport: `[Persp] [Orto] [2:1 Split]`.
* **Critérios de Aceite:** Troca de modo de projeção reunida em um controle único e coerente.

---

### QT-14 — Controle de sensibilidade de zoom e ajustes nas preferências
* **Problema:** Zoom inicial lento/travando ao aproximar da cena; ausência de controle de velocidade de zoom; diálogo de preferências carece de color wheel e configuração de atalhos.
* **Solução:**
  - Ajustar a curva de velocidade de zoom em `petunia_gl_widget.cpp` para aceleração adaptativa proporcional à distância do ponto focal.
  - Adicionar controle de `Velocidade do Zoom` (Zoom Sensitivity) nas preferências.
  - Integrar Color Wheel / `QColorDialog` interativo para `Accent Color` e `Overlay Selection Color`.
  - Exibir tabela editável de atalhos de teclado no diálogo de preferências.
* **Critérios de Aceite:** Navegação de zoom fluida em qualquer escala de distância e diálogo de preferências completo com seleção de cores visual e atalhos.

---

### QT-15 — Painéis flutuantes móveis e redimensionáveis (Imagens de Referência)
* **Problema:** Certos painéis secundários (como Imagens de Referência) ficam fixos ou sem liberdade de redimensionamento e posicionamento.
* **Solução:**
  - Tornar o painel de Imagens de Referência uma janela flutuante independente (`QDialog` não-modal ou `QDockWidget` desencaixado), redimensionável e móvel.
* **Critérios de Aceite:** Painel de referências pode ser posicionado livremente e mantido aberto ao lado da viewport.

---

### QT-16 — Pie Menu radial com atalho Shift+A
* **Problema:** Adicionar novos objetos requer navegar em menus de topo ou barras laterais, quebrando o fluxo de modelagem ágil.
* **Solução:**
  - Implementar Pie Menu radial ativado pelo atalho `Shift+A` na viewport 3D.
  - O menu surge centrado na coordenada do mouse na viewport, com fatias radiais para: Cube, Cylinder, UV Sphere, Plane, Torus, Cone, Empty, etc., selecionáveis com movimento direcional do mouse ou clique.
* **Critérios de Aceite:** Pressionar Shift+A na viewport abre menu circular no cursor do mouse e insere a primitiva escolhida no local do cursor 3D.

---

## 3. Registro de Execução e Checkpoints

| Data | Tarefas Alteradas | Evidências e Comandos | Status |
|---|---|---|---|
| 2026-10-15 | QT-00 (Transforms compactos, ícones frameless, context bar dinâmica, outliner 22px, header unificado) | `cargo test -p petunia_ui_cxxqt --test editor_parity` (17/17 pass)<br>`cargo run -p petunia_ui_cxxqt --example qt_parity_acceptance` (50/50 pass)<br>Screenshot [`qt_refined_v2.png`](file:///home/raillen/.gemini/antigravity/brain/f047b0f5-1db9-470d-8dd1-7c7927e4aee7/qt_refined_v2.png) | Concluído |
| 2026-10-15 | Criação do Backlog Mestre de Refatoração (QT-01 a QT-16) | Estruturação das 16 novas demandas do usuário em tabela com critérios técnicos de aceite e regras de governança contínua. | Concluído |
| 2026-10-15 | QT-01 a QT-16 (Refatoração Integral: Tool Rail focada, Gizmo moderno RGB, 3D Cursor tela/interativo, Ortho Poly Pen, Pivot desacoplado, Docking livre, Primitives popup/pie menu, View consolidada, Cena vazia, Outliner polish, Projeção unificada 2:1, Zoom sensitivity, Pie Menu Shift+A) | `cargo test -p petunia_ui_cxxqt --test editor_parity` (17/17 pass)<br>`cargo test -p petunia_ui_cxxqt --test bridge_test` (10/10 pass)<br>`cargo run -p petunia_ui_cxxqt --example qt_parity_acceptance` (50/50 pass)<br>Screenshots [`qt_refined_v3.png`](file:///home/raillen/.gemini/antigravity/brain/f047b0f5-1db9-470d-8dd1-7c7927e4aee7/qt_refined_v3.png) e `/tmp/qt_acceptance/*.png` | Concluído |
