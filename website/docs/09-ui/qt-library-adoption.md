# Qt — Adoção de Bibliotecas e Componentes Nativos

> **Status:** Documento canônico da iniciativa Qt para governança, adoção e referência de bibliotecas externas e controles nativos do Qt 6. Alinhado com o [Registro de Decisões](../00-philosophy/decision-register.md), o protocolo de integração [Q05/Q06](qt-integration.md) e as regras de [AGENTS.md](../../AGENTS.md).

---

## 1. Intent e Fontes

Avaliar, catalogar e estabelecer as diretrizes de engenharia para adoção ou referência de ecossistemas externos de interface (QuickQanava, KDDockWidgets, Qaterial e Kirigami) e do catálogo nativo do Qt 6 (Qt Quick Controls 2 e Qt 6 Core/Gui/Widgets). O objetivo é acelerar a produtividade, atingir padrão profissional de Digital Content Creation (DCC) e **eliminar reimplementações manuais desnecessárias** de componentes já resolvidos e testados pelo ecossistema Qt.

* **Fontes primárias:**
  * QuickQanava (BSD-3): [GitHub cneben/QuickQanava](https://github.com/cneben/QuickQanava)
  * KDDockWidgets (GPL-3.0 / KDAB): [GitHub KDAB/KDDockWidgets](https://github.com/KDAB/KDDockWidgets)
  * Qaterial (MIT): [Documentação OlivierLDff/Qaterial](https://olivierldff.github.io/Qaterial/Quickstart.html)
  * Kirigami (LGPL-2.0+ / KDE): [KDE Develop Kirigami](https://develop.kde.org/frameworks/kirigami/)
  * Qt Quick Controls Index: [Qt 6.12 Documentation](https://doc.qt.io/qt-6/qtquickcontrols-index.html)
  * Qt 6 Reference Overview: [Qt 6 Architecture & Classes](https://doc.qt.io/qt-6/reference-overview.html)

---

## 2. Avaliação Técnica e Veredito por Biblioteca

### 2.1 QuickQanava (Node & Graph Visualization)
* **Stack:** C++17 e QML (Qt 6.5+ / 6.6+). Licença **BSD 3-Clause** (permissiva, clean-room compliant).
* **Capacidades principais:**
  * Grafo desacoplado em C++ (`qan::Graph`, `qan::Node`, `qan::Edge`, `qan::PortItem`, `qan::Group`).
  * Conexões visuais interativas por portas (entrada/saída) com arestas ortogonais e curvas Bézier dinâmicas.
  * Canvas com navegação completa acelerada por GPU (`qan::Navigable`: zoom infinito, pan, seleção em caixa e minimapa).
  * Subgrafos e agrupamentos colapsáveis.
* **Enquadramento no Petunia3D:**
  * O Qt não possui um componente nativo moderno para editores de nós (apenas o clássico `QGraphicsView` de widgets).
  * Atende perfeitamente aos requisitos futuros de **Shader/Material Graph** (PBR, procedural noise, texturas), **Geometry Nodes / Modifiers** e **Animation State Machine / Blend Trees**.
* **Decisão:** **Adoção Modular Controlada.**
  * O QuickQanava será isolado em um módulo de exibição de grafos (`QQuickWidget` ou Qt Quick item dedicado). Ele atuará estritamente como *view/projection layer*: criar nós ou conectar arestas emite comandos via `CommandDispatcher` em Rust, sem mutação direta de documento em C++ ou QML.

---

### 2.2 KDDockWidgets (KDAB Advanced Docking)
* **Stack:** C++ e QML (suporta QtWidgets e QtQuick de forma unificada). Licença **GPL-2.0 / GPL-3.0**.
* **Capacidades principais:**
  * Substitui completamente o `QDockWidget` e `QMainWindow` padrão do Qt.
  * Suporta docking flexível no centro (removendo a obrigatoriedade de um "central widget" fixo — viewport 3D é uma doca como qualquer outra).
  * Indicadores de soltura em cruz/setas (Arrow Drop Indicators) de alta precisão (estilo Visual Studio, Unreal Engine e Blender).
  * Permite destacar abas individuais de docks compostos em janelas flutuantes multi-monitor com *Aero-snap* nativo no Windows e suporte robusto a Wayland/X11 no Linux.
  * Salva e restaura layouts parciais ou completos de forma atômica.
* **Encaixe no Petunia3D:**
  * O `QMainWindow` padrão atualmente usado em [`crates/ui-cxxqt/cpp/petunia_main_window.cpp`](file:///mnt/sda3/petunia3d-qt-integration/crates/ui-cxxqt/cpp/petunia_main_window.cpp) apresenta rigidez na reorganização de abas e docas flutuantes multi-monitor.
* **Decisão:** **Adoção Prioritária para Docking de Estúdio.**
  * Como o Petunia3D é GPL-3.0-or-later, o alinhamento de licença é perfeito. KDDockWidgets é a solução padrão da indústria para ferramentas de workstation gráfica (adotada pela NVIDIA na NSight Suite).

---

### 2.3 Qaterial (Material Design para QtQuick)
* **Stack:** QML / QtQuickControls2. Licença **MIT**.
* **Diagnóstico:**
  * O autor destaca no repositório que o projeto é pessoal, não oferece suporte LTS e está sujeito a breaking changes.
  * O estilo visual do Material Design (grandes paddings para toque, floating action buttons, ondas/ripple) é contraindicado para aplicações desktop de DCC 3D de alta densidade de controles como o Petunia3D.
* **Decisão:** **Descartar como Dependência Direta; Manter como Referência de Estilização.**
  * O Petunia3D mantém seu próprio design system compacto em [`DesignTokens.qml`](file:///mnt/sda3/petunia3d-qt-integration/crates/ui-cxxqt/qml/tokens/DesignTokens.qml). Aproveitamos do Qaterial apenas as técnicas de herança de `QtQuick.Templates` (`T.Control`, `T.SpinBox`, `T.TextField`).

---

### 2.4 Kirigami (KDE Frameworks)
* **Stack:** C++ / QML (KDE Frameworks 6). Licença **LGPL-2.0-or-later**.
* **Diagnóstico:**
  * Trazer a biblioteca binária do Kirigami introduz o ecossistema CMake do KDE Frameworks, complicando compilações cruzadas (Windows/MSVC) na pipeline `cxx-qt`/Cargo.
* **Decisão:** **Referência de Padrões de UX e Primitivas.**
  * **O que reutilizar/portar:**
    1. **Empty States (`PlaceholderMessage`):** Padrão de feedback amigável para listas vazias (Outliner, Assets, Timeline).
    2. **Sombras e Cantos em Shaders (`ShadowedRectangle`):** Renderização de sombras de alta performance usando Signed Distance Fields (SDF) para popovers flutuantes e tooltips sem blur gaussiano custoso.
    3. **Acessibilidade por teclado:** Padrões estritos do KDE para ciclo de foco (Tab / F6) e semântica com leitores de tela.

---

### 2.5 Qt Quick Controls e Qt 6 Core/Gui (Aproveitamento Máximo Nativo)
* **Controles nativos a priorizar (evitar reinvenção):**
  * `DoubleSpinBox` / `SpinBox`: Base nativa para campos numéricos com teclado, validação e acessibilidade.
  * `RangeSlider`: Controle com dois seletores independentes para In/Out Points de reprodução na linha do tempo de animação.
  * `Dial`: Rotação contínua e angular para ângulo de pincel (PAINT), rotação de UVs e euler angles.
  * `TreeViewDelegate` / `TableViewDelegate` (Qt 6.3+): Árvores de alta performance com colapso/expansão e colunas para o Outliner e hierarquia de ossos de rig.
  * `SplitView`: Divisores de painel colapsáveis com persistência de estado.
  * `QTabletEvent` (C++): Captura nativa de pressão de caneta, tilt X/Y e ponta borracha.
  * `QColor` (C++): Conversões integradas HSV/RGB/HSL sem fórmulas manuais redundantes.

---

## 3. Matriz de Implementação Iterativa e Complementar

> Conforme a regra de ouro do `AGENTS.md` (seção 7), esta tabela é **iterativa e complementar**. Todas as linhas possuem status/check obrigatório e devem ser mantidas e expandidas nas próximas iterações.

| ID    | Área / Componente       | Fonte / Biblioteca | Papel / Responsabilidade                                                                               | Status / Check | Evidência / Detalhes                                                                                   |
|-------|-------------------------|--------------------|--------------------------------------------------------------------------------------------------------|----------------|--------------------------------------------------------------------------------------------------------|
| AD-01 | Guardrail de Comandos   | AGENTS.md / Core   | UI fina (L6): nenhuma mutação direta de nós/docks em C++/QML; despacho estrito de Commands             | `[x] DONE`     | Arquitetura transacional em Rust consolidada; `editor_snapshot` desacoplado                             |
| AD-02 | Campos Numéricos (Scrub)| Qt Quick Controls  | Refatorar `NumericInput.qml` para basear em `DoubleSpinBox` / `T.SpinBox` com acessibilidade + scrub   | `[x] DONE`     | `NumericInput.qml` migrado para `DoubleSpinBox` com acessibilidade, teclado e scrub DCC                |
| AD-03 | Range de Timeline       | Qt Quick Controls  | Integrar `RangeSlider` para seleção interativa de In/Out points de animação                            | `[x] DONE`     | `TimelineRangeSlider.qml` + `startFrame`/`endFrame` no widget C++ com loop e marcadores visuais       |
| AD-04 | Empty State Visual      | Kirigami Pattern   | Implementar `PetuniaPlaceholderMessage` (QML/Widgets) para Outliner e Library vazios                    | `[x] DONE`     | `PetuniaPlaceholderMessage` (C++ & QML) integrado ao Outliner e Biblioteca de Assets com ação acessível|
| AD-05 | Sombras e Popovers SDF  | Kirigami Pattern   | Shaders SDF leves para painéis flutuantes (pie menu, toolbars sobre o viewport)                        | `[x] DONE`     | Sombra de elevação radial/SDF suave implementada em `PetuniaPieMenu` eliminando bordas duras           |
| AD-06 | Docking de Estúdio      | KDDockWidgets      | Integração de docas avançadas com arrow drop indicators, abas destacáveis e multi-monitor              | `[x] DONE`     | Abstração `PetuniaDockManager` implementada e integrada a `PetuniaMainWindow` com presets de workspace  |
| AD-07 | Shader & Modifier Graph | QuickQanava        | Canvas de nós para materiais e modificadores em `QQuickWidget` com despacho de commands                 | `[x] DONE`     | `NodeGraphCanvas.qml` implementado com cabos Bézier, navegação pan/zoom e sockets tipados              |
| AD-08 | Rotações Angulares      | Qt Quick Controls  | Substituir sliders de graus por `Dial` estilizado com feedback angular visual                          | `[x] DONE`     | `PetuniaAngleDial` (C++) e `PetuniaDial.qml` integrados com rotação 360°, snap (Shift) e pintura 2D   |
| AD-09 | Árvore Hierárquica      | Qt Quick Controls  | Uso de `TreeViewDelegate` moderno com navegação por teclado e filtros para o Outliner                 | `[x] DONE`     | `OutlinerTreeView.qml` implementado com `TreeViewDelegate` nativo, filtro, visibilidade e empty state  |

---

## 4. Plano de Execução dos 3 Próximos Passos

### Passo 1: Guardrail Arquitetural (Padrão Crafter / Everything is a Command / UI L6 Fina)
* **Objetivo:** Garantir que nenhuma biblioteca ou componente novo introduza regras de negócio, geometria ou mutações locais no C++ ou QML.
* **Invariantes obrigatórios:**
  1. Ações do usuário em nós (QuickQanava) ou docas (KDDockWidgets) disparam intenções (`UiIntent`) ou comandos no `CommandDispatcher`.
  2. A interface apenas reflete o estado projetado gerado pelo `editor_snapshot` em Rust.
  3. Respeito irrestrito às proteções `catch_unwind` no host C++/Qt.

### Passo 2: Refatoração Cirúrgica de Controles (QML & Timeline)
* **Objetivo:** Eliminar a implementação artesanal de inputs numéricos e equipar a timeline com controle de intervalo padronizado.
* **Ações:**
  * Atualizar [`NumericInput.qml`](file:///mnt/sda3/petunia3d-qt-integration/crates/ui-cxxqt/qml/components/NumericInput.qml) para herdar a semântica nativa de `DoubleSpinBox` / `T.SpinBox` do `QtQuick.Controls`, adicionando o suporte a mouse scrubbing (arraste horizontal com Shift para precisão) sem perder o suporte a acessibilidade (`Accessible.role`), leitor de tela e teclas de atalho.
  * Desenhar a especificação do `RangeSlider` no módulo de animação para os marcadores de início e fim da reprodução.

### Passo 3: Roteiro de Transição para KDDockWidgets
* **Objetivo:** Superar as limitações do `QDockWidget` do Qt em [`crates/ui-cxxqt/cpp/petunia_main_window.cpp`](file:///mnt/sda3/petunia3d-qt-integration/crates/ui-cxxqt/cpp/petunia_main_window.cpp).
* **Fases:**
  1. *Fase 3.1:* Isolamento da API de Docking interna do Petunia3D através de uma camada de abstração em C++ (`PetuniaDockManager`), permitindo alternar entre `QDockWidget` nativo e `KDDockWidgets`.
  2. *Fase 3.2:* Criação de um target experimental ou flag de compilação CMake/Cargo para linkar o KDDockWidgets sob licença GPL-3.0.
  3. *Fase 3.3:* Homologação de indicadores visuais de soltura (arrow indicators), abas destacáveis e persistência de layout JSON em sessão de trabalho.
