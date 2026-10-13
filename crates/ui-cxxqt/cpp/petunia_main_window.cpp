#include "petunia_main_window.h"
#include "petunia_command_palette.h"
#include "petunia_preferences_dialog.h"
#include "petunia_reference_dialog.h"
#include "petunia_paint_panel.h"
#include "petunia_uv_panel.h"
#include "petunia_overlay_cards.h"
#include "petunia_home_dialog.h"
#include "petunia_recovery_dialog.h"
#include "petunia_timeline_widget.h"
#include "petunia_asset_library_widget.h"

#include <QApplication>
#include <QSurfaceFormat>
#include <QMenuBar>
#include <QStatusBar>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QDoubleSpinBox>
#include <QGroupBox>
#include <QHeaderView>
#include <QColorDialog>
#include <QMenu>
#include <QButtonGroup>
#include <QShortcut>
#include <QFileDialog>
#include <QMessageBox>
#include <QComboBox>
#include <QSplitter>

extern "C" {
    bool petunia_qt_app_execute_command(const char *cmd);
    bool petunia_qt_app_undo();
    bool petunia_qt_app_redo();
    int petunia_qt_app_get_asset_count();
    bool petunia_qt_app_get_asset_name(int index, char *buf, int max_len);
    bool petunia_qt_app_get_asset_visible(int index);
    void petunia_qt_app_toggle_visibility(int index, bool visible);
    void petunia_qt_app_select_asset(int index);
    bool petunia_qt_app_get_asset_position(int index, float *x, float *y, float *z);
    bool petunia_qt_app_set_asset_position(int index, float x, float y, float z);
    bool petunia_qt_app_get_asset_rotation(int index, float *rx, float *ry, float *rz);
    bool petunia_qt_app_set_asset_rotation(int index, float rx, float ry, float rz);
    bool petunia_qt_app_get_asset_scale(int index, float *sx, float *sy, float *sz);
    bool petunia_qt_app_set_asset_scale(int index, float sx, float sy, float sz);
    bool petunia_qt_app_get_asset_color(int index, float *r, float *g, float *b);
    bool petunia_qt_app_set_asset_color(int index, float r, float g, float b);
    bool petunia_qt_app_get_scene_stats(int *verts_out, int *tris_out);
    void petunia_qt_app_set_selection_domain(int domain);
    void petunia_qt_app_frame_selection();
    void petunia_qt_camera_set_preset(int preset);
    bool petunia_qt_app_save_project(const char *path);
    bool petunia_qt_app_load_project(const char *path);
    bool petunia_qt_app_export_gltf(const char *path);
    bool petunia_qt_app_export_obj(const char *path);
    bool petunia_qt_app_is_dirty();
    int petunia_qt_app_selected_count();
    int petunia_qt_app_get_active_asset();
    int petunia_qt_app_get_selection_domain();
    void petunia_qt_camera_reset();

    int petunia_qt_app_get_modifier_count(int asset_index);
    bool petunia_qt_app_get_modifier_info(int asset_index, int mod_index, char *name_buf, int max_len, bool *enabled, int *axis, float *weld);
    bool petunia_qt_app_add_modifier(int asset_index, int kind, int axis, float weld);
    bool petunia_qt_app_toggle_modifier(int asset_index, int mod_index, bool enabled);
    bool petunia_qt_app_remove_modifier(int asset_index, int mod_index);
    bool petunia_qt_app_apply_modifier(int asset_index, int mod_index);
    bool petunia_qt_app_move_modifier(int asset_index, int mod_index, bool up);

    bool petunia_qt_app_get_material_pbr(int asset_index, float *roughness, float *metallic, float *emission_strength, int *profile);
    bool petunia_qt_app_set_material_pbr(int asset_index, float roughness, float metallic, float emission_strength, int profile);

    int petunia_qt_app_get_surface_shading(int asset_index);
    bool petunia_qt_app_set_surface_shading(int asset_index, int shading);

    bool petunia_qt_app_quick_action(const char *action);
}

PetuniaMainWindow::PetuniaMainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setupUi();
    applyDarkTheme();
}

PetuniaMainWindow::~PetuniaMainWindow() = default;

void PetuniaMainWindow::setupUi() {
    resize(1280, 800);
    setWindowTitle(QStringLiteral("Petunia3D — Qt Studio[*]"));
    setDockOptions(QMainWindow::AnimatedDocks | QMainWindow::AllowNestedDocks | QMainWindow::AllowTabbedDocks);

    // Contêiner Central: Tool Options Bar no topo + Divisor de Viewports embaixo
    auto *centralContainer = new QWidget(this);
    auto *centralLayout = new QVBoxLayout(centralContainer);
    centralLayout->setContentsMargins(4, 4, 4, 4);
    centralLayout->setSpacing(4);

    // 1. Barra de Opções de Ferramentas / Context Bar / Shading Bar
    m_toolOptionsBar = new PetuniaToolOptionsBar(centralContainer);
    centralLayout->addWidget(m_toolOptionsBar);

    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::snapToggled, this, [this](bool enabled) {
        emit actionTriggered("tool.toggle_snap");
        setStatusMessage(enabled ? QStringLiteral("Snap ativado") : QStringLiteral("Snap desativado"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::proportionalToggled, this, [this](bool enabled) {
        emit actionTriggered("tool.toggle_proportional");
        setStatusMessage(enabled ? QStringLiteral("Edição proporcional ativada") : QStringLiteral("Edição proporcional desativada"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::shadingModeChanged, this, [this](const QString &mode) {
        if (m_viewport) m_viewport->setShadingMode(mode);
        if (m_secondaryViewport) m_secondaryViewport->setShadingMode(mode);
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::xrayToggled, this, [this](bool enabled) {
        setStatusMessage(enabled ? QStringLiteral("Raio-X: Ativado") : QStringLiteral("Raio-X: Desativado"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::matcapToggled, this, [this](bool enabled) {
        setStatusMessage(enabled ? QStringLiteral("MatCap: Ativado") : QStringLiteral("MatCap: Desativado"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::ambientOcclusionToggled, this, [this](bool enabled) {
        setStatusMessage(enabled ? QStringLiteral("Oclusão Ambiente (AO): Ativada") : QStringLiteral("Oclusão Ambiente (AO): Desativada"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::faceOrientationToggled, this, [this](bool enabled) {
        setStatusMessage(enabled ? QStringLiteral("Orientação de Faces: Ativada") : QStringLiteral("Orientação de Faces: Desativada"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::uvCheckerToggled, this, [this](bool enabled) {
        setStatusMessage(enabled ? QStringLiteral("UV Checker: Ativado") : QStringLiteral("UV Checker: Desativado"));
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::actionTriggered, this, &PetuniaMainWindow::actionTriggered);

    // 2. Divisor de Viewport (Suporte a Vista Dividida / Split View)
    m_viewportSplitter = new QSplitter(Qt::Horizontal, centralContainer);
    centralLayout->addWidget(m_viewportSplitter, 1);

    // Viewport Primária OpenGL 3.3
    auto *primaryContainer = new QWidget(m_viewportSplitter);
    auto *primaryLayout = new QVBoxLayout(primaryContainer);
    primaryLayout->setContentsMargins(0, 0, 0, 0);

    m_viewport = new PetuniaGlWidget(primaryContainer);
    connect(m_viewport, &PetuniaGlWidget::selectionChanged, this, &PetuniaMainWindow::syncFromApplication);
    primaryLayout->addWidget(m_viewport);
    m_viewportSplitter->addWidget(primaryContainer);

    // Cards Sobrepostos Flutuantes na Viewport Primária
    m_primitiveCard = new PetuniaPrimitiveCard(m_viewport);
    m_primitiveCard->move(20, 20);

    m_loopCutCard = new PetuniaLoopCutCard(m_viewport);
    m_loopCutCard->move(20, 20);

    m_microInspector = new PetuniaMicroInspector(m_viewport);
    m_microInspector->move(20, 160);
    connect(m_microInspector, &PetuniaMicroInspector::toolSelected, this, [this](const QString &toolId) {
        setStatusMessage(QStringLiteral("Ferramenta ativa: %1").arg(toolId));
    });
    connect(m_microInspector, &PetuniaMicroInspector::modelingActionTriggered, this, &PetuniaMainWindow::actionTriggered);

    m_operationHud = new PetuniaOperationHud(m_viewport);
    m_operationHud->move(20, 360);

    // Viewport Secundária Dividida
    m_secondaryViewportContainer = new QWidget(m_viewportSplitter);
    auto *secondaryLayout = new QVBoxLayout(m_secondaryViewportContainer);
    secondaryLayout->setContentsMargins(0, 0, 0, 0);
    secondaryLayout->setSpacing(2);

    auto *secondaryBar = new QWidget(m_secondaryViewportContainer);
    secondaryBar->setFixedHeight(28);
    secondaryBar->setStyleSheet(QStringLiteral("background-color: #202020; border-bottom: 1px solid #383838;"));
    auto *secondaryBarLayout = new QHBoxLayout(secondaryBar);
    secondaryBarLayout->setContentsMargins(4, 2, 4, 2);
    secondaryBarLayout->setSpacing(4);

    const struct { const char *id; const char *label; } presets[] = {
        {"persp", "Persp"}, {"front", "Frente"}, {"back", "Trás"},
        {"left", "Esq"}, {"right", "Dir"}, {"top", "Topo"}, {"iso21", "2:1"}
    };
    for (const auto &p : presets) {
        auto *btn = new QPushButton(QString::fromUtf8(p.label), secondaryBar);
        btn->setFixedHeight(22);
        btn->setStyleSheet(QStringLiteral(
            "QPushButton { background: #282828; color: #bbb; border: 1px solid #404040; border-radius: 3px; font-size: 10px; padding: 0 6px; }"
            "QPushButton:hover { background: #353535; color: #fff; }"
        ));
        connect(btn, &QPushButton::clicked, this, [this, id = QString::fromUtf8(p.id)]() {
            setSecondaryCameraPreset(id);
        });
        secondaryBarLayout->addWidget(btn);
    }
    secondaryBarLayout->addStretch();

    auto *closeSplitBtn = new QPushButton(QStringLiteral("✕"), secondaryBar);
    closeSplitBtn->setFixedSize(20, 20);
    closeSplitBtn->setCursor(Qt::PointingHandCursor);
    closeSplitBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background: transparent; color: #888; border: none; font-weight: bold; }"
        "QPushButton:hover { color: #fff; background: #333; border-radius: 2px; }"
    ));
    connect(closeSplitBtn, &QPushButton::clicked, this, &PetuniaMainWindow::toggleSplitView);
    secondaryBarLayout->addWidget(closeSplitBtn);

    secondaryLayout->addWidget(secondaryBar);

    m_secondaryViewport = new PetuniaGlWidget(m_secondaryViewportContainer);
    connect(m_secondaryViewport, &PetuniaGlWidget::selectionChanged, this, &PetuniaMainWindow::syncFromApplication);
    secondaryLayout->addWidget(m_secondaryViewport);

    m_viewportSplitter->addWidget(m_secondaryViewportContainer);
    m_secondaryViewportContainer->hide();

    setCentralWidget(centralContainer);

    // Popover / Menu Adicionar Primitivas (Shift+A)
    m_addMenu = new PetuniaAddMenu(this);
    connect(m_addMenu, &PetuniaAddMenu::primitiveSelected, this, [this](const QString &id) {
        emit actionTriggered(QStringLiteral("model.add_%1").arg(id));
        showPrimitiveCard(id);
    });

    auto *shiftAShortcut = new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_A), this);
    connect(shiftAShortcut, &QShortcut::activated, this, &PetuniaMainWindow::openAddMenu);

    auto *spaceShortcut = new QShortcut(QKeySequence(Qt::Key_Space), this);
    connect(spaceShortcut, &QShortcut::activated, this, &PetuniaMainWindow::toggleMicroInspector);

    setupDocks();
    setupMenuBar();
    setupWorkspaceBar();
    setupToolRail();
    setupStatusBar();

    connect(this, &PetuniaMainWindow::actionTriggered, this, [this](const QString &action) {
        QByteArray utf8 = action.toUtf8();
        petunia_qt_app_execute_command(utf8.constData());
        syncFromApplication();
    });

    syncFromApplication();
}

void PetuniaMainWindow::setupMenuBar() {
    auto *menuBar = this->menuBar();

    // 1. Menu Arquivo
    auto *fileMenu = menuBar->addMenu(QStringLiteral("&Arquivo"));
    auto *newAct = fileMenu->addAction(QStringLiteral("Novo Projeto"));
    newAct->setShortcut(QKeySequence::New);
    connect(newAct, &QAction::triggered, this, &PetuniaMainWindow::newProject);

    auto *openAct = fileMenu->addAction(QStringLiteral("Abrir..."));
    openAct->setShortcut(QKeySequence::Open);
    connect(openAct, &QAction::triggered, this, &PetuniaMainWindow::openProject);

    fileMenu->addSeparator();
    auto *saveAct = fileMenu->addAction(QStringLiteral("Salvar"));
    saveAct->setShortcut(QKeySequence::Save);
    connect(saveAct, &QAction::triggered, this, &PetuniaMainWindow::saveProject);

    auto *saveAsAct = fileMenu->addAction(QStringLiteral("Salvar Como..."));
    saveAsAct->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_S));
    connect(saveAsAct, &QAction::triggered, this, &PetuniaMainWindow::saveProjectAs);

    fileMenu->addSeparator();
    auto *exportMenu = fileMenu->addMenu(QStringLiteral("&Exportar"));
    auto *exportGltfAct = exportMenu->addAction(QStringLiteral("GLTF / GLB (*.glb)..."));
    connect(exportGltfAct, &QAction::triggered, this, &PetuniaMainWindow::exportGltf);

    auto *exportObjAct = exportMenu->addAction(QStringLiteral("Wavefront OBJ (*.obj)..."));
    connect(exportObjAct, &QAction::triggered, this, &PetuniaMainWindow::exportObj);

    // 2. Menu Editar
    auto *editMenu = menuBar->addMenu(QStringLiteral("&Editar"));
    auto *undoAct = editMenu->addAction(QStringLiteral("Desfazer"));
    undoAct->setShortcut(QKeySequence::Undo);
    connect(undoAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_undo();
        syncFromApplication();
    });

    auto *redoAct = editMenu->addAction(QStringLiteral("Refazer"));
    redoAct->setShortcut(QKeySequence::Redo);
    connect(redoAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_redo();
        syncFromApplication();
    });

    editMenu->addSeparator();
    auto *dupAct = editMenu->addAction(QStringLiteral("Duplicar"));
    dupAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_D));
    connect(dupAct, &QAction::triggered, this, [this]() { emit actionTriggered("edit.duplicate"); });

    auto *delAct = editMenu->addAction(QStringLiteral("Deletar"));
    delAct->setShortcut(QKeySequence::Delete);
    connect(delAct, &QAction::triggered, this, [this]() { emit actionTriggered("edit.delete"); });

    editMenu->addSeparator();
    auto *paletteAct = editMenu->addAction(QStringLiteral("Paleta de Comandos..."));
    paletteAct->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_P));
    connect(paletteAct, &QAction::triggered, this, &PetuniaMainWindow::openCommandPalette);

    auto *paletteF3 = new QAction(this);
    paletteF3->setShortcut(QKeySequence(Qt::Key_F3));
    connect(paletteF3, &QAction::triggered, this, &PetuniaMainWindow::openCommandPalette);
    addAction(paletteF3);

    auto *paletteCtrlK = new QAction(this);
    paletteCtrlK->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_K));
    connect(paletteCtrlK, &QAction::triggered, this, &PetuniaMainWindow::openCommandPalette);
    addAction(paletteCtrlK);

    editMenu->addSeparator();
    auto *prefAct = editMenu->addAction(QStringLiteral("Preferências..."));
    prefAct->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_Comma));
    connect(prefAct, &QAction::triggered, this, &PetuniaMainWindow::openPreferences);

    // 3. Menu Seleção
    auto *selectMenu = menuBar->addMenu(QStringLiteral("&Seleção"));
    auto *selAll = selectMenu->addAction(QStringLiteral("Selecionar Tudo"));
    selAll->setShortcut(QKeySequence::SelectAll);
    connect(selAll, &QAction::triggered, this, [this]() { emit actionTriggered("select.all"); });

    auto *selNone = selectMenu->addAction(QStringLiteral("Limpar Seleção"));
    selNone->setShortcut(QKeySequence(Qt::ALT | Qt::Key_A));
    connect(selNone, &QAction::triggered, this, [this]() { emit actionTriggered("select.clear"); });

    auto *selInv = selectMenu->addAction(QStringLiteral("Inverter Seleção"));
    selInv->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_I));
    connect(selInv, &QAction::triggered, this, [this]() { emit actionTriggered("select.invert"); });

    // 4. Menu Adicionar
    auto *addMenu = menuBar->addMenu(QStringLiteral("&Adicionar"));
    auto *addMenuPopAct = addMenu->addAction(QStringLiteral("Menu de Primitivas..."));
    addMenuPopAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_A));
    connect(addMenuPopAct, &QAction::triggered, this, &PetuniaMainWindow::openAddMenu);

    auto *assetLibAct = addMenu->addAction(QStringLiteral("Biblioteca de Modelos & Assets..."));
    assetLibAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F1));
    connect(assetLibAct, &QAction::triggered, this, &PetuniaMainWindow::openAssetLibrary);

    addMenu->addSeparator();

    auto *addCube = addMenu->addAction(QStringLiteral("Cubo"));
    connect(addCube, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.add_cube");
        showPrimitiveCard(QStringLiteral("Cubo"));
    });

    auto *addSphere = addMenu->addAction(QStringLiteral("Esfera"));
    connect(addSphere, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.add_sphere");
        showPrimitiveCard(QStringLiteral("Esfera"));
    });

    auto *addCylinder = addMenu->addAction(QStringLiteral("Cilindro"));
    connect(addCylinder, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.add_cylinder");
        showPrimitiveCard(QStringLiteral("Cilindro"));
    });

    auto *addPlane = addMenu->addAction(QStringLiteral("Plano"));
    connect(addPlane, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.add_plane");
        showPrimitiveCard(QStringLiteral("Plano"));
    });

    auto *addCone = addMenu->addAction(QStringLiteral("Cone"));
    connect(addCone, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.add_cone");
        showPrimitiveCard(QStringLiteral("Cone"));
    });

    auto *addCapsule = addMenu->addAction(QStringLiteral("Cápsula"));
    connect(addCapsule, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.add_capsule");
        showPrimitiveCard(QStringLiteral("Cápsula"));
    });

    // 5. Menu Ferramentas
    auto *toolsMenu = menuBar->addMenu(QStringLiteral("&Ferramentas"));
    auto *refAct = toolsMenu->addAction(QStringLiteral("Gerenciador de Imagens de Referência..."));
    connect(refAct, &QAction::triggered, this, &PetuniaMainWindow::openReferenceManager);

    toolsMenu->addSeparator();
    auto *snapAct = toolsMenu->addAction(QStringLiteral("Snap ao Grid"));
    snapAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_Tab));
    connect(snapAct, &QAction::triggered, this, [this]() { emit actionTriggered("tool.toggle_snap"); });

    // 6. Menu Exibir
    auto *viewMenu = menuBar->addMenu(QStringLiteral("&Exibir"));
    viewMenu->addAction(m_outlinerDock->toggleViewAction());
    viewMenu->addAction(m_inspectorDock->toggleViewAction());
    viewMenu->addAction(m_timelineDock->toggleViewAction());
    viewMenu->addAction(m_paintDock->toggleViewAction());
    viewMenu->addAction(m_uvDock->toggleViewAction());
    viewMenu->addSeparator();

    auto *splitViewAct = viewMenu->addAction(QStringLiteral("Dividir Vista 3D"));
    splitViewAct->setShortcut(QKeySequence(Qt::CTRL | Qt::ALT | Qt::Key_Q));
    connect(splitViewAct, &QAction::triggered, this, &PetuniaMainWindow::toggleSplitView);

    auto *microInspAct = viewMenu->addAction(QStringLiteral("Micro Inspetor"));
    microInspAct->setShortcut(QKeySequence(Qt::Key_Space));
    connect(microInspAct, &QAction::triggered, this, &PetuniaMainWindow::toggleMicroInspector);

    viewMenu->addSeparator();

    auto *frameAct = viewMenu->addAction(QStringLiteral("Focar Seleção"));
    frameAct->setShortcut(QKeySequence(Qt::Key_F));
    connect(frameAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_frame_selection();
        if (m_viewport) m_viewport->update();
    });

    auto *resetCam = viewMenu->addAction(QStringLiteral("Redefinir Câmera"));
    resetCam->setShortcut(QKeySequence(Qt::Key_Home));
    connect(resetCam, &QAction::triggered, this, [this]() {
        petunia_qt_camera_reset();
        if (m_viewport) m_viewport->update();
    });

    // 7. Menu Ajuda
    auto *helpMenu = menuBar->addMenu(QStringLiteral("&Ajuda"));
    auto *homeAct = helpMenu->addAction(QStringLiteral("Tela Inicial (Home)..."));
    connect(homeAct, &QAction::triggered, this, &PetuniaMainWindow::openHomeDialog);

    auto *recAct = helpMenu->addAction(QStringLiteral("Recuperar Sessão..."));
    connect(recAct, &QAction::triggered, this, &PetuniaMainWindow::openRecoveryDialog);

    helpMenu->addSeparator();

    auto *aboutAct = helpMenu->addAction(QStringLiteral("Sobre o Petunia3D"));
    connect(aboutAct, &QAction::triggered, this, [this]() {
        QMessageBox::about(
            this,
            QStringLiteral("Sobre o Petunia3D"),
            QStringLiteral("<h3>Petunia3D — Qt Studio</h3>"
                           "<p>Modelador e Editor 3D profissional multiplataforma em Rust e Qt 6.</p>"
                           "<p><b>Arquitetura:</b> OpenGL 3.3 Core Profile • Zero-Copy Framebuffer Blit • QDockWidget Multijanela • QTabletEvent Stylus.</p>")
        );
    });
}

void PetuniaMainWindow::setupWorkspaceBar() {
    auto *wsBar = addToolBar(QStringLiteral("Workspaces"));
    wsBar->setObjectName(QStringLiteral("WorkspaceToolBar"));
    wsBar->setMovable(false);

    // 1. Abas de Workspace do Petunia
    const QStringList wsList = { "DRAW", "POLY", "PAINT", "UV", "ANIMATE" };
    auto *wsGroup = new QButtonGroup(this);
    for (const auto &ws : wsList) {
        auto *btn = new QPushButton(ws, wsBar);
        btn->setCheckable(true);
        btn->setChecked(ws == m_activeWorkspace);
        wsGroup->addButton(btn);
        connect(btn, &QPushButton::clicked, this, [this, ws]() {
            setWorkspace(ws);
            emit workspaceChanged(ws);
        });
        wsBar->addWidget(btn);
    }

    wsBar->addSeparator();

    // 2. Seletores de Domínio de Edição (Object / Vertex / Edge / Face)
    auto *domainGroup = new QButtonGroup(this);
    domainGroup->setExclusive(true);

    m_domainObjBtn = new QPushButton(QStringLiteral("⬡ Objeto"), wsBar);
    m_domainObjBtn->setCheckable(true);
    m_domainObjBtn->setChecked(true);
    m_domainObjBtn->setToolTip(QStringLiteral("Modo Objeto (Atalho 1)"));
    domainGroup->addButton(m_domainObjBtn, 0);
    wsBar->addWidget(m_domainObjBtn);

    m_domainVertBtn = new QPushButton(QStringLiteral("• Vértice"), wsBar);
    m_domainVertBtn->setCheckable(true);
    m_domainVertBtn->setToolTip(QStringLiteral("Modo Vértice (Atalho 1 em POLY)"));
    domainGroup->addButton(m_domainVertBtn, 1);
    wsBar->addWidget(m_domainVertBtn);

    m_domainEdgeBtn = new QPushButton(QStringLiteral("╱ Aresta"), wsBar);
    m_domainEdgeBtn->setCheckable(true);
    m_domainEdgeBtn->setToolTip(QStringLiteral("Modo Aresta (Atalho 2)"));
    domainGroup->addButton(m_domainEdgeBtn, 2);
    wsBar->addWidget(m_domainEdgeBtn);

    m_domainFaceBtn = new QPushButton(QStringLiteral("▱ Face"), wsBar);
    m_domainFaceBtn->setCheckable(true);
    m_domainFaceBtn->setToolTip(QStringLiteral("Modo Face (Atalho 3)"));
    domainGroup->addButton(m_domainFaceBtn, 3);
    wsBar->addWidget(m_domainFaceBtn);

    connect(domainGroup, &QButtonGroup::idClicked, this, &PetuniaMainWindow::setSelectionDomain);

    // Atalhos ergonômicos para seleção de domínio (1, 2, 3)
    auto *scEdge = new QShortcut(QKeySequence(Qt::Key_2), this);
    connect(scEdge, &QShortcut::activated, this, [this]() { m_domainEdgeBtn->click(); });

    auto *scFace = new QShortcut(QKeySequence(Qt::Key_3), this);
    connect(scFace, &QShortcut::activated, this, [this]() { m_domainFaceBtn->click(); });
}

void PetuniaMainWindow::setupToolRail() {
    m_toolRailBar = addToolBar(QStringLiteral("Ferramentas"));
    m_toolRailBar->setObjectName(QStringLiteral("ToolRailBar"));
    m_toolRailBar->setOrientation(Qt::Vertical);
    addToolBar(Qt::LeftToolBarArea, m_toolRailBar);
    m_toolRailBar->setMovable(false);

    auto addRailTool = [this](const QString &text, const QString &tooltip, const QString &cmd, const QKeySequence &shortcut = QKeySequence()) {
        auto *act = m_toolRailBar->addAction(text);
        act->setToolTip(tooltip);
        if (!shortcut.isEmpty()) {
            act->setShortcut(shortcut);
        }
        connect(act, &QAction::triggered, this, [this, cmd]() {
            emit actionTriggered(cmd);
        });
        return act;
    };

    addRailTool(QStringLiteral("↖"), QStringLiteral("Selecionar / Mover (W)"), QStringLiteral("select.all"));
    addRailTool(QStringLiteral("⮁"), QStringLiteral("Extrusão (E)"), QStringLiteral("model.extrude"), QKeySequence(Qt::Key_E));
    addRailTool(QStringLiteral("▣"), QStringLiteral("Inset de Faces (I)"), QStringLiteral("model.inset"), QKeySequence(Qt::Key_I));
    addRailTool(QStringLiteral("⯎"), QStringLiteral("Bevel / Chanfro (Ctrl+B)"), QStringLiteral("model.bevel"), QKeySequence(Qt::CTRL | Qt::Key_B));

    auto *loopCutAct = addRailTool(QStringLiteral("⌗"), QStringLiteral("Loop Cut (Ctrl+R)"), QStringLiteral("model.loop_cut"), QKeySequence(Qt::CTRL | Qt::Key_R));
    connect(loopCutAct, &QAction::triggered, this, &PetuniaMainWindow::showLoopCutCard);

    addRailTool(QStringLiteral("▦"), QStringLiteral("Subdividir Malha"), QStringLiteral("model.subdivide"));

    m_toolRailBar->addSeparator();

    addRailTool(QStringLiteral("⧉"), QStringLiteral("Duplicar Objeto (Shift+D)"), QStringLiteral("edit.duplicate"), QKeySequence(Qt::SHIFT | Qt::Key_D));
    addRailTool(QStringLiteral("✕"), QStringLiteral("Deletar Seleção (Del)"), QStringLiteral("edit.delete"), QKeySequence(Qt::Key_Delete));

    m_toolRailBar->addSeparator();

    auto *frameAct = m_toolRailBar->addAction(QStringLiteral("🎯"));
    frameAct->setToolTip(QStringLiteral("Focar Seleção na Câmera (F)"));
    connect(frameAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_frame_selection();
        if (m_viewport) m_viewport->update();
    });
}

void PetuniaMainWindow::setupDocks() {
    // 1. Doca Esquerda: Cena / Outliner
    m_outlinerDock = new QDockWidget(QStringLiteral("Cena (Outliner)"), this);
    m_outlinerDock->setObjectName(QStringLiteral("OutlinerDock"));
    m_outlinerDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable | QDockWidget::DockWidgetClosable);

    auto *outlinerContainer = new QWidget(m_outlinerDock);
    auto *outlinerLayout = new QVBoxLayout(outlinerContainer);
    outlinerLayout->setContentsMargins(6, 6, 6, 6);

    m_outlinerSearch = new QLineEdit(outlinerContainer);
    m_outlinerSearch->setPlaceholderText(QStringLiteral("Filtrar objetos (Ctrl+F)..."));
    outlinerLayout->addWidget(m_outlinerSearch);

    m_outlinerTree = new QTreeView(outlinerContainer);
    m_outlinerModel = new QStandardItemModel(this);
    m_outlinerModel->setHorizontalHeaderLabels({ QStringLiteral("Objeto"), QStringLiteral("👁"), QStringLiteral("🔒") });

    m_outlinerProxy = new QSortFilterProxyModel(this);
    m_outlinerProxy->setSourceModel(m_outlinerModel);
    m_outlinerProxy->setFilterCaseSensitivity(Qt::CaseInsensitive);
    m_outlinerProxy->setFilterKeyColumn(0);

    connect(m_outlinerSearch, &QLineEdit::textChanged, m_outlinerProxy, &QSortFilterProxyModel::setFilterFixedString);

    m_outlinerTree->setModel(m_outlinerProxy);
    m_outlinerTree->header()->setSectionResizeMode(0, QHeaderView::Stretch);
    m_outlinerTree->header()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    m_outlinerTree->header()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
    m_outlinerTree->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(m_outlinerTree, &QTreeView::customContextMenuRequested, this, &PetuniaMainWindow::onOutlinerContextMenu);

    outlinerLayout->addWidget(m_outlinerTree);

    connect(m_outlinerTree, &QTreeView::clicked, this, [this](const QModelIndex &proxyIdx) {
        if (!proxyIdx.isValid()) return;
        QModelIndex srcIdx = m_outlinerProxy->mapToSource(proxyIdx);
        int row = srcIdx.row();
        int col = proxyIdx.column();
        if (col == 1) { // Coluna de Visibilidade
            bool current = petunia_qt_app_get_asset_visible(row);
            petunia_qt_app_toggle_visibility(row, !current);
        } else {
            petunia_qt_app_select_asset(row);
        }
        syncFromApplication();
    });

    connect(m_outlinerTree->selectionModel(), &QItemSelectionModel::currentChanged,
            this, [this](const QModelIndex &current, const QModelIndex &) {
        if (m_syncingOutliner || !current.isValid()) return;
        QModelIndex srcIdx = m_outlinerProxy->mapToSource(current);
        int row = srcIdx.row();
        petunia_qt_app_select_asset(row);
        syncFromApplication();
    });

    m_outlinerDock->setWidget(outlinerContainer);
    addDockWidget(Qt::LeftDockWidgetArea, m_outlinerDock);

    // 2. Doca Direita: Inspetor de Propriedades e Modificadores
    m_inspectorDock = new QDockWidget(QStringLiteral("Inspetor"), this);
    m_inspectorDock->setObjectName(QStringLiteral("InspectorDock"));
    m_inspectorDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable | QDockWidget::DockWidgetClosable);

    auto *inspectorContainer = new QWidget(m_inspectorDock);
    auto *inspectorLayout = new QVBoxLayout(inspectorContainer);
    inspectorLayout->setContentsMargins(8, 8, 8, 8);

    // 2.1 Seção Transform (Posição, Rotação e Escala)
    auto *transformGroup = new QGroupBox(QStringLiteral("Transform"), inspectorContainer);
    auto *transformLayout = new QFormLayout(transformGroup);

    m_posX = new QDoubleSpinBox(transformGroup);
    m_posX->setRange(-10000.0, 10000.0);
    m_posX->setSingleStep(0.1);
    m_posX->setSuffix(QStringLiteral(" m"));
    transformLayout->addRow(QStringLiteral("Posição X:"), m_posX);

    m_posY = new QDoubleSpinBox(transformGroup);
    m_posY->setRange(-10000.0, 10000.0);
    m_posY->setSingleStep(0.1);
    m_posY->setSuffix(QStringLiteral(" m"));
    transformLayout->addRow(QStringLiteral("Posição Y:"), m_posY);

    m_posZ = new QDoubleSpinBox(transformGroup);
    m_posZ->setRange(-10000.0, 10000.0);
    m_posZ->setSingleStep(0.1);
    m_posZ->setSuffix(QStringLiteral(" m"));
    transformLayout->addRow(QStringLiteral("Posição Z:"), m_posZ);

    m_rotX = new QDoubleSpinBox(transformGroup);
    m_rotX->setRange(-360.0, 360.0);
    m_rotX->setSingleStep(5.0);
    m_rotX->setSuffix(QStringLiteral("°"));
    transformLayout->addRow(QStringLiteral("Rotação X:"), m_rotX);

    m_rotY = new QDoubleSpinBox(transformGroup);
    m_rotY->setRange(-360.0, 360.0);
    m_rotY->setSingleStep(5.0);
    m_rotY->setSuffix(QStringLiteral("°"));
    transformLayout->addRow(QStringLiteral("Rotação Y:"), m_rotY);

    m_rotZ = new QDoubleSpinBox(transformGroup);
    m_rotZ->setRange(-360.0, 360.0);
    m_rotZ->setSingleStep(5.0);
    m_rotZ->setSuffix(QStringLiteral("°"));
    transformLayout->addRow(QStringLiteral("Rotação Z:"), m_rotZ);

    m_scaleX = new QDoubleSpinBox(transformGroup);
    m_scaleX->setRange(0.001, 1000.0);
    m_scaleX->setSingleStep(0.1);
    m_scaleX->setValue(1.0);
    m_scaleX->setSuffix(QStringLiteral("x"));
    transformLayout->addRow(QStringLiteral("Escala X:"), m_scaleX);

    m_scaleY = new QDoubleSpinBox(transformGroup);
    m_scaleY->setRange(0.001, 1000.0);
    m_scaleY->setSingleStep(0.1);
    m_scaleY->setValue(1.0);
    m_scaleY->setSuffix(QStringLiteral("x"));
    transformLayout->addRow(QStringLiteral("Escala Y:"), m_scaleY);

    m_scaleZ = new QDoubleSpinBox(transformGroup);
    m_scaleZ->setRange(0.001, 1000.0);
    m_scaleZ->setSingleStep(0.1);
    m_scaleZ->setValue(1.0);
    m_scaleZ->setSuffix(QStringLiteral("x"));
    transformLayout->addRow(QStringLiteral("Escala Z:"), m_scaleZ);

    auto getSelectedRow = [this]() -> int {
        auto selection = m_outlinerTree->selectionModel()->selectedRows();
        if (selection.isEmpty()) return 0;
        return m_outlinerProxy->mapToSource(selection.first()).row();
    };

    auto onPosChanged = [this, getSelectedRow]() {
        if (m_syncingTransforms) return;
        int row = getSelectedRow();
        petunia_qt_app_set_asset_position(
            row,
            static_cast<float>(m_posX->value()),
            static_cast<float>(m_posY->value()),
            static_cast<float>(m_posZ->value())
        );
        updateDirtyState(petunia_qt_app_is_dirty());
        if (m_viewport) m_viewport->update();
    };

    auto onRotChanged = [this, getSelectedRow]() {
        if (m_syncingTransforms) return;
        int row = getSelectedRow();
        petunia_qt_app_set_asset_rotation(
            row,
            static_cast<float>(m_rotX->value()),
            static_cast<float>(m_rotY->value()),
            static_cast<float>(m_rotZ->value())
        );
        updateDirtyState(petunia_qt_app_is_dirty());
        if (m_viewport) m_viewport->update();
    };

    auto onScaleChanged = [this, getSelectedRow]() {
        if (m_syncingTransforms) return;
        int row = getSelectedRow();
        petunia_qt_app_set_asset_scale(
            row,
            static_cast<float>(m_scaleX->value()),
            static_cast<float>(m_scaleY->value()),
            static_cast<float>(m_scaleZ->value())
        );
        updateDirtyState(petunia_qt_app_is_dirty());
        if (m_viewport) m_viewport->update();
    };

    connect(m_posX, &QDoubleSpinBox::valueChanged, this, onPosChanged);
    connect(m_posY, &QDoubleSpinBox::valueChanged, this, onPosChanged);
    connect(m_posZ, &QDoubleSpinBox::valueChanged, this, onPosChanged);

    connect(m_rotX, &QDoubleSpinBox::valueChanged, this, onRotChanged);
    connect(m_rotY, &QDoubleSpinBox::valueChanged, this, onRotChanged);
    connect(m_rotZ, &QDoubleSpinBox::valueChanged, this, onRotChanged);

    connect(m_scaleX, &QDoubleSpinBox::valueChanged, this, onScaleChanged);
    connect(m_scaleY, &QDoubleSpinBox::valueChanged, this, onScaleChanged);
    connect(m_scaleZ, &QDoubleSpinBox::valueChanged, this, onScaleChanged);

    inspectorLayout->addWidget(transformGroup);

    // 2.2 Seção Modificadores
    auto *modGroup = new QGroupBox(QStringLiteral("Pilha de Modificadores"), inspectorContainer);
    auto *modGroupLayout = new QVBoxLayout(modGroup);
    modGroupLayout->setContentsMargins(6, 6, 6, 6);

    auto *modComboRow = new QHBoxLayout();
    auto *modCombo = new QComboBox(modGroup);
    modCombo->addItem(QStringLiteral("Espelho (Mirror)"), 0);
    modCombo->addItem(QStringLiteral("Simetria (Symmetry)"), 1);
    modComboRow->addWidget(modCombo, 1);

    auto *addModBtn = new QPushButton(QStringLiteral("+ Adicionar"), modGroup);
    addModBtn->setCursor(Qt::PointingHandCursor);
    connect(addModBtn, &QPushButton::clicked, this, [this, modCombo, getSelectedRow]() {
        int row = getSelectedRow();
        int kind = modCombo->currentData().toInt();
        petunia_qt_app_add_modifier(row, kind, 0, 0.001f);
        setStatusMessage(QStringLiteral("Modificador adicionado: %1").arg(modCombo->currentText()));
        refreshModifiers();
        if (m_viewport) m_viewport->update();
    });
    modComboRow->addWidget(addModBtn);
    modGroupLayout->addLayout(modComboRow);

    m_modifiersContainer = new QWidget(modGroup);
    m_modifiersListLayout = new QVBoxLayout(m_modifiersContainer);
    m_modifiersListLayout->setContentsMargins(0, 4, 0, 0);
    m_modifiersListLayout->setSpacing(6);
    modGroupLayout->addWidget(m_modifiersContainer);

    inspectorLayout->addWidget(modGroup);

    // 2.3 Seção Material PBR
    auto *materialGroup = new QGroupBox(QStringLiteral("Material PBR"), inspectorContainer);
    auto *materialLayout = new QFormLayout(materialGroup);
    materialLayout->setContentsMargins(6, 6, 6, 6);

    m_colorBtn = new QPushButton(QStringLiteral("Alterar Albedo"), materialGroup);
    m_colorBtn->setCursor(Qt::PointingHandCursor);
    connect(m_colorBtn, &QPushButton::clicked, this, &PetuniaMainWindow::pickMaterialColor);
    materialLayout->addRow(QStringLiteral("Cor Base:"), m_colorBtn);

    // Rugosidade
    auto *roughRow = new QHBoxLayout();
    m_roughnessSlider = new QSlider(Qt::Horizontal, materialGroup);
    m_roughnessSlider->setRange(0, 100);
    m_roughnessSlider->setValue(50);
    m_roughnessLabel = new QLabel(QStringLiteral("0.50"), materialGroup);
    m_roughnessLabel->setFixedWidth(32);
    roughRow->addWidget(m_roughnessSlider);
    roughRow->addWidget(m_roughnessLabel);
    materialLayout->addRow(QStringLiteral("Rugosidade:"), roughRow);

    // Metacidade
    auto *metalRow = new QHBoxLayout();
    m_metallicSlider = new QSlider(Qt::Horizontal, materialGroup);
    m_metallicSlider->setRange(0, 100);
    m_metallicSlider->setValue(0);
    m_metallicLabel = new QLabel(QStringLiteral("0.00"), materialGroup);
    m_metallicLabel->setFixedWidth(32);
    metalRow->addWidget(m_metallicSlider);
    metalRow->addWidget(m_metallicLabel);
    materialLayout->addRow(QStringLiteral("Metacidade:"), metalRow);

    // Emissão
    m_emissionSpin = new QDoubleSpinBox(materialGroup);
    m_emissionSpin->setRange(0.0, 100.0);
    m_emissionSpin->setSingleStep(0.1);
    m_emissionSpin->setValue(0.0);
    materialLayout->addRow(QStringLiteral("Emissão:"), m_emissionSpin);

    // Perfil de Shader
    m_profileCombo = new QComboBox(materialGroup);
    m_profileCombo->addItem(QStringLiteral("PBR Standard"), 0);
    m_profileCombo->addItem(QStringLiteral("Unlit / Flat"), 1);
    m_profileCombo->addItem(QStringLiteral("Toon / Cel-Shading"), 2);
    m_profileCombo->addItem(QStringLiteral("Glass / Transparent"), 3);
    m_profileCombo->addItem(QStringLiteral("Emissive"), 4);
    materialLayout->addRow(QStringLiteral("Perfil:"), m_profileCombo);

    auto onMaterialPbrChanged = [this, getSelectedRow]() {
        if (m_syncingMaterial) return;
        int row = getSelectedRow();
        float rough = m_roughnessSlider->value() / 100.0f;
        float metal = m_metallicSlider->value() / 100.0f;
        float emission = static_cast<float>(m_emissionSpin->value());
        int prof = m_profileCombo->currentData().toInt();
        m_roughnessLabel->setText(QString::number(rough, 'f', 2));
        m_metallicLabel->setText(QString::number(metal, 'f', 2));
        petunia_qt_app_set_material_pbr(row, rough, metal, emission, prof);
        if (m_viewport) m_viewport->update();
    };

    connect(m_roughnessSlider, &QSlider::valueChanged, this, onMaterialPbrChanged);
    connect(m_metallicSlider, &QSlider::valueChanged, this, onMaterialPbrChanged);
    connect(m_emissionSpin, &QDoubleSpinBox::valueChanged, this, onMaterialPbrChanged);
    connect(m_profileCombo, &QComboBox::currentIndexChanged, this, onMaterialPbrChanged);

    inspectorLayout->addWidget(materialGroup);

    // 2.4 Sombreamento da Superfície (Shading Flat / Smooth)
    auto *shadingGroup = new QGroupBox(QStringLiteral("Sombreamento da Superfície"), inspectorContainer);
    auto *shadingLayout = new QHBoxLayout(shadingGroup);
    shadingLayout->setContentsMargins(6, 6, 6, 6);

    m_flatShadingBtn = new QPushButton(QStringLiteral("Plano (Flat)"), shadingGroup);
    m_flatShadingBtn->setCheckable(true);
    m_smoothShadingBtn = new QPushButton(QStringLiteral("Suave (Smooth)"), shadingGroup);
    m_smoothShadingBtn->setCheckable(true);

    auto *shadingBtnGroup = new QButtonGroup(this);
    shadingBtnGroup->addButton(m_flatShadingBtn, 0);
    shadingBtnGroup->addButton(m_smoothShadingBtn, 1);
    shadingLayout->addWidget(m_flatShadingBtn);
    shadingLayout->addWidget(m_smoothShadingBtn);

    connect(shadingBtnGroup, &QButtonGroup::idClicked, this, [this, getSelectedRow](int id) {
        int row = getSelectedRow();
        petunia_qt_app_set_surface_shading(row, id);
        setStatusMessage(id == 1 ? QStringLiteral("Sombreamento Suave ativado") : QStringLiteral("Sombreamento Plano ativado"));
        if (m_viewport) m_viewport->update();
    });

    inspectorLayout->addWidget(shadingGroup);

    // 2.5 Seção Ações Rápidas (Quick Actions)
    auto *quickGroup = new QGroupBox(QStringLiteral("Ações Rápidas"), inspectorContainer);
    auto *quickGrid = new QGridLayout(quickGroup);
    quickGrid->setContentsMargins(6, 6, 6, 6);
    quickGrid->setSpacing(6);

    auto makeQuickBtn = [this](const QString &text, const char *action) -> QPushButton* {
        auto *btn = new QPushButton(text);
        btn->setCursor(Qt::PointingHandCursor);
        connect(btn, &QPushButton::clicked, this, [this, action, text]() {
            petunia_qt_app_quick_action(action);
            setStatusMessage(QStringLiteral("Ação executada: %1").arg(text));
            syncFromApplication();
        });
        return btn;
    };

    quickGrid->addWidget(makeQuickBtn(QStringLiteral("⛶ Subdividir"), "subdivide"), 0, 0);
    quickGrid->addWidget(makeQuickBtn(QStringLiteral("🪞 Simetrizar"), "symmetrize"), 0, 1);
    quickGrid->addWidget(makeQuickBtn(QStringLiteral("⇄ Inverter Sel."), "invert_selection"), 1, 0);
    quickGrid->addWidget(makeQuickBtn(QStringLiteral("⧉ Selecionar Tudo"), "select_all"), 1, 1);
    quickGrid->addWidget(makeQuickBtn(QStringLiteral("🎯 Origem ao Centro"), "origin_to_geometry"), 2, 0);
    quickGrid->addWidget(makeQuickBtn(QStringLiteral("⤓ Origem à Base"), "origin_to_bottom"), 2, 1);

    inspectorLayout->addWidget(quickGroup);
    inspectorLayout->addStretch();

    m_inspectorDock->setWidget(inspectorContainer);
    addDockWidget(Qt::RightDockWidgetArea, m_inspectorDock);

    // 3. Doca Inferior: Timeline / Animação
    m_timelineDock = new QDockWidget(QStringLiteral("Timeline / Dope Sheet"), this);
    m_timelineDock->setObjectName(QStringLiteral("TimelineDock"));
    m_timelineDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable | QDockWidget::DockWidgetClosable);

    m_timelineWidget = new PetuniaTimelineWidget(m_timelineDock);
    connect(m_timelineWidget, &PetuniaTimelineWidget::frameChanged, this, [this](int f) {
        setStatusMessage(QStringLiteral("Frame de Animação: %1 / 250").arg(f));
        if (m_viewport) m_viewport->update();
    });
    connect(m_timelineWidget, &PetuniaTimelineWidget::playbackToggled, this, [this](bool playing) {
        setStatusMessage(playing ? QStringLiteral("Reprodução ativa (30 fps)") : QStringLiteral("Reprodução pausada"));
    });
    connect(m_timelineWidget, &PetuniaTimelineWidget::keyframeInserted, this, [this](int, int f) {
        setStatusMessage(QStringLiteral("Keyframe registrado no frame %1").arg(f));
        updateDirtyState(true);
        if (m_viewport) m_viewport->update();
    });
    connect(m_timelineWidget, &PetuniaTimelineWidget::keyframeDeleted, this, [this](int, int f) {
        setStatusMessage(QStringLiteral("Keyframe removido do frame %1").arg(f));
        updateDirtyState(true);
        if (m_viewport) m_viewport->update();
    });

    m_timelineDock->setWidget(m_timelineWidget);
    addDockWidget(Qt::BottomDockWidgetArea, m_timelineDock);

    // 4. Doca Especializada: PAINT
    m_paintDock = new QDockWidget(QStringLiteral("Pintura & Texturas (PAINT)"), this);
    m_paintDock->setObjectName(QStringLiteral("PaintDock"));
    m_paintDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable | QDockWidget::DockWidgetClosable);
    m_paintPanel = new PetuniaPaintPanel(m_paintDock);
    m_paintDock->setWidget(m_paintPanel);
    addDockWidget(Qt::RightDockWidgetArea, m_paintDock);
    m_paintDock->hide();

    // 5. Doca Especializada: UV
    m_uvDock = new QDockWidget(QStringLiteral("Editor de Coordenadas UV"), this);
    m_uvDock->setObjectName(QStringLiteral("UvDock"));
    m_uvDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable | QDockWidget::DockWidgetClosable);
    m_uvPanel = new PetuniaUvPanel(m_uvDock);
    m_uvDock->setWidget(m_uvPanel);
    addDockWidget(Qt::LeftDockWidgetArea, m_uvDock);
    m_uvDock->hide();
}

void PetuniaMainWindow::setupStatusBar() {
    auto *sb = statusBar();
    m_modeBadge = new QLabel(QStringLiteral(" OBJECT "), this);
    m_modeBadge->setStyleSheet(QStringLiteral("background-color: #b58cff; color: #121212; font-weight: bold; border-radius: 3px; padding: 2px 6px;"));
    sb->addWidget(m_modeBadge);

    m_statusLabel = new QLabel(QStringLiteral("Pronto"), this);
    sb->addWidget(m_statusLabel, 1);

    m_statsLabel = new QLabel(QStringLiteral("Vértices: 0 | Triângulos: 0"), this);
    m_statsLabel->setStyleSheet(QStringLiteral("color: #888888; font-family: monospace; margin-right: 12px;"));
    sb->addPermanentWidget(m_statsLabel);

    m_selectionLabel = new QLabel(QStringLiteral("Nenhum objeto selecionado"), this);
    sb->addPermanentWidget(m_selectionLabel);
}

void PetuniaMainWindow::setSelectionDomain(int domain) {
    m_activeDomain = domain;
    petunia_qt_app_set_selection_domain(domain);

    switch (domain) {
    case 1:
        m_modeBadge->setText(QStringLiteral(" VERTEX "));
        m_modeBadge->setStyleSheet(QStringLiteral("background-color: #4fc3f7; color: #121212; font-weight: bold; border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Vértice (Nível Sub-Objeto)"));
        break;
    case 2:
        m_modeBadge->setText(QStringLiteral(" EDGE "));
        m_modeBadge->setStyleSheet(QStringLiteral("background-color: #81c784; color: #121212; font-weight: bold; border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Aresta"));
        break;
    case 3:
        m_modeBadge->setText(QStringLiteral(" FACE "));
        m_modeBadge->setStyleSheet(QStringLiteral("background-color: #ffb74d; color: #121212; font-weight: bold; border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Face"));
        break;
    default:
        m_modeBadge->setText(QStringLiteral(" OBJECT "));
        m_modeBadge->setStyleSheet(QStringLiteral("background-color: #b58cff; color: #121212; font-weight: bold; border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Objeto B-Rep"));
        break;
    }

    if (m_viewport) {
        m_viewport->update();
    }
}

void PetuniaMainWindow::pickMaterialColor() {
    auto selection = m_outlinerTree->selectionModel()->selectedRows();
    int row = selection.isEmpty() ? 0 : m_outlinerProxy->mapToSource(selection.first()).row();

    float r = 0.8f, g = 0.8f, b = 0.8f;
    petunia_qt_app_get_asset_color(row, &r, &g, &b);
    QColor initialColor = QColor::fromRgbF(r, g, b);

    QColor chosen = QColorDialog::getColor(initialColor, this, QStringLiteral("Selecionar Cor Base do Material"));
    if (chosen.isValid()) {
        petunia_qt_app_set_asset_color(
            row,
            static_cast<float>(chosen.redF()),
            static_cast<float>(chosen.greenF()),
            static_cast<float>(chosen.blueF())
        );
        updateDirtyState(petunia_qt_app_is_dirty());
        syncFromApplication();
    }
}

void PetuniaMainWindow::onOutlinerContextMenu(const QPoint &pos) {
    QModelIndex proxyIdx = m_outlinerTree->indexAt(pos);
    QMenu menu(this);

    auto *frameAct = menu.addAction(QStringLiteral("🎯 Focar Câmera (F)"));
    connect(frameAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_frame_selection();
        if (m_viewport) m_viewport->update();
    });

    menu.addSeparator();

    auto *dupAct = menu.addAction(QStringLiteral("⧉ Duplicar Objeto (Shift+D)"));
    connect(dupAct, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("edit.duplicate"));
    });

    auto *delAct = menu.addAction(QStringLiteral("✕ Deletar Objeto (Del)"));
    connect(delAct, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("edit.delete"));
    });

    if (proxyIdx.isValid()) {
        QModelIndex srcIdx = m_outlinerProxy->mapToSource(proxyIdx);
        int row = srcIdx.row();
        bool isVis = petunia_qt_app_get_asset_visible(row);
        menu.addSeparator();
        auto *visAct = menu.addAction(isVis ? QStringLiteral("Ocultar Objeto") : QStringLiteral("Exibir Objeto"));
        connect(visAct, &QAction::triggered, this, [this, row, isVis]() {
            petunia_qt_app_toggle_visibility(row, !isVis);
            syncFromApplication();
        });
    }

    menu.exec(m_outlinerTree->viewport()->mapToGlobal(pos));
}

void PetuniaMainWindow::syncFromApplication() {
    int count = petunia_qt_app_get_asset_count();
    QStringList names;
    QList<bool> visibilities;
    char buf[256];

    for (int i = 0; i < count; ++i) {
        if (petunia_qt_app_get_asset_name(i, buf, sizeof(buf))) {
            names.append(QString::fromUtf8(buf));
        } else {
            names.append(QStringLiteral("Objeto %1").arg(i + 1));
        }
        visibilities.append(petunia_qt_app_get_asset_visible(i));
    }

    populateOutliner(names, visibilities);
    updateSelectedCount(petunia_qt_app_selected_count());
    updateDirtyState(petunia_qt_app_is_dirty());

    int verts = 0, tris = 0;
    if (petunia_qt_app_get_scene_stats(&verts, &tris)) {
        m_statsLabel->setText(QStringLiteral("Vértices: %1 | Triângulos: %2").arg(verts).arg(tris));
    }

    if (count > 0 && m_posX && m_posY && m_posZ) {
        m_syncingTransforms = true;
        auto selection = m_outlinerTree->selectionModel()->selectedRows();
        int row = selection.isEmpty() ? 0 : m_outlinerProxy->mapToSource(selection.first()).row();

        float x = 0.0f, y = 0.0f, z = 0.0f;
        if (petunia_qt_app_get_asset_position(row, &x, &y, &z)) {
            m_posX->setValue(static_cast<double>(x));
            m_posY->setValue(static_cast<double>(y));
            m_posZ->setValue(static_cast<double>(z));
        }

        float rx = 0.0f, ry = 0.0f, rz = 0.0f;
        if (petunia_qt_app_get_asset_rotation(row, &rx, &ry, &rz)) {
            m_rotX->setValue(static_cast<double>(rx));
            m_rotY->setValue(static_cast<double>(ry));
            m_rotZ->setValue(static_cast<double>(rz));
        }

        float sx = 1.0f, sy = 1.0f, sz = 1.0f;
        if (petunia_qt_app_get_asset_scale(row, &sx, &sy, &sz)) {
            m_scaleX->setValue(static_cast<double>(sx));
            m_scaleY->setValue(static_cast<double>(sy));
            m_scaleZ->setValue(static_cast<double>(sz));
        }

        float r = 0.8f, g = 0.8f, b = 0.8f;
        if (petunia_qt_app_get_asset_color(row, &r, &g, &b)) {
            QColor col = QColor::fromRgbF(r, g, b);
            QString textCol = col.lightness() > 130 ? QStringLiteral("#121212") : QStringLiteral("#eeeeee");
            m_colorBtn->setStyleSheet(QStringLiteral(
                "background-color: %1; color: %2; font-weight: bold; border-radius: 4px; border: 1px solid #555555; padding: 4px 10px;"
            ).arg(col.name(), textCol));
        }

        m_syncingTransforms = false;
    }

    refreshModifiers();
    refreshMaterialPbr();

    if (m_timelineWidget) {
        m_timelineWidget->syncFromApp();
    }

    if (m_viewport) {
        m_viewport->update();
    }
}

void PetuniaMainWindow::refreshModifiers() {
    if (!m_modifiersListLayout) return;

    QLayoutItem *item;
    while ((item = m_modifiersListLayout->takeAt(0)) != nullptr) {
        if (item->widget()) delete item->widget();
        delete item;
    }

    auto selection = m_outlinerTree->selectionModel()->selectedRows();
    int row = selection.isEmpty() ? 0 : m_outlinerProxy->mapToSource(selection.first()).row();
    int modCount = petunia_qt_app_get_modifier_count(row);

    if (modCount == 0) {
        auto *emptyLabel = new QLabel(QStringLiteral("Nenhum modificador ativo"), m_modifiersContainer);
        emptyLabel->setStyleSheet(QStringLiteral("color: #777777; font-style: italic; font-size: 11px;"));
        m_modifiersListLayout->addWidget(emptyLabel);
        return;
    }

    for (int i = 0; i < modCount; ++i) {
        char nameBuf[64] = {0};
        bool enabled = true;
        int axis = 0;
        float weld = 0.001f;
        if (!petunia_qt_app_get_modifier_info(row, i, nameBuf, sizeof(nameBuf), &enabled, &axis, &weld)) {
            continue;
        }

        auto *card = new QFrame(m_modifiersContainer);
        card->setStyleSheet(QStringLiteral("QFrame { background-color: #242424; border: 1px solid #3c3c3c; border-radius: 4px; padding: 4px; }"));
        auto *cardLayout = new QVBoxLayout(card);
        cardLayout->setContentsMargins(6, 6, 6, 6);
        cardLayout->setSpacing(4);

        auto *headerRow = new QHBoxLayout();
        auto *enableCheck = new QCheckBox(QString::fromUtf8(nameBuf), card);
        enableCheck->setChecked(enabled);
        enableCheck->setStyleSheet(QStringLiteral("font-weight: bold; color: #b58cff;"));
        connect(enableCheck, &QCheckBox::toggled, this, [this, row, i](bool checked) {
            petunia_qt_app_toggle_modifier(row, i, checked);
            if (m_viewport) m_viewport->update();
        });
        headerRow->addWidget(enableCheck, 1);

        auto *applyBtn = new QPushButton(QStringLiteral("Aplicar"), card);
        applyBtn->setCursor(Qt::PointingHandCursor);
        connect(applyBtn, &QPushButton::clicked, this, [this, row, i]() {
            petunia_qt_app_apply_modifier(row, i);
            setStatusMessage(QStringLiteral("Modificador aplicado à malha base"));
            refreshModifiers();
            syncFromApplication();
        });
        headerRow->addWidget(applyBtn);

        auto *delBtn = new QPushButton(QStringLiteral("✕"), card);
        delBtn->setFixedWidth(24);
        delBtn->setCursor(Qt::PointingHandCursor);
        connect(delBtn, &QPushButton::clicked, this, [this, row, i]() {
            petunia_qt_app_remove_modifier(row, i);
            setStatusMessage(QStringLiteral("Modificador removido"));
            refreshModifiers();
            syncFromApplication();
        });
        headerRow->addWidget(delBtn);
        cardLayout->addLayout(headerRow);

        auto *optsRow = new QHBoxLayout();
        auto *axisLabel = new QLabel(QStringLiteral("Eixo:"), card);
        auto *axisCombo = new QComboBox(card);
        axisCombo->addItem(QStringLiteral("X"), 0);
        axisCombo->addItem(QStringLiteral("Y"), 1);
        axisCombo->addItem(QStringLiteral("Z"), 2);
        axisCombo->setCurrentIndex(axis);
        optsRow->addWidget(axisLabel);
        optsRow->addWidget(axisCombo);

        auto *weldLabel = new QLabel(QStringLiteral("Solda:"), card);
        auto *weldSpin = new QDoubleSpinBox(card);
        weldSpin->setRange(0.0001, 1.0);
        weldSpin->setSingleStep(0.001);
        weldSpin->setValue(static_cast<double>(weld));
        weldSpin->setSuffix(QStringLiteral(" m"));
        optsRow->addWidget(weldLabel);
        optsRow->addWidget(weldSpin);

        cardLayout->addLayout(optsRow);
        m_modifiersListLayout->addWidget(card);
    }
}

void PetuniaMainWindow::refreshMaterialPbr() {
    if (!m_roughnessSlider || !m_metallicSlider || !m_emissionSpin || !m_profileCombo) return;
    m_syncingMaterial = true;

    auto selection = m_outlinerTree->selectionModel()->selectedRows();
    int row = selection.isEmpty() ? 0 : m_outlinerProxy->mapToSource(selection.first()).row();

    float rough = 0.5f, metal = 0.0f, emission = 0.0f;
    int profile = 0;
    if (petunia_qt_app_get_material_pbr(row, &rough, &metal, &emission, &profile)) {
        m_roughnessSlider->setValue(static_cast<int>(rough * 100.0f));
        m_roughnessLabel->setText(QString::number(rough, 'f', 2));
        m_metallicSlider->setValue(static_cast<int>(metal * 100.0f));
        m_metallicLabel->setText(QString::number(metal, 'f', 2));
        m_emissionSpin->setValue(static_cast<double>(emission));
        int profIdx = m_profileCombo->findData(profile);
        if (profIdx >= 0) m_profileCombo->setCurrentIndex(profIdx);
    }

    int shading = petunia_qt_app_get_surface_shading(row);
    if (m_flatShadingBtn && m_smoothShadingBtn) {
        m_flatShadingBtn->setChecked(shading == 0);
        m_smoothShadingBtn->setChecked(shading == 1);
    }

    m_syncingMaterial = false;
}

void PetuniaMainWindow::openCommandPalette() {
    if (!m_commandPalette) {
        m_commandPalette = new PetuniaCommandPalette(this);
        connect(m_commandPalette, &PetuniaCommandPalette::commandSelected, this, [this](const QString &cmd) {
            petunia_qt_app_execute_command(cmd.toUtf8().constData());
            syncFromApplication();
        });
    }
    QPoint center = mapToGlobal(rect().center()) - QPoint(m_commandPalette->width() / 2, m_commandPalette->height() / 2);
    m_commandPalette->move(center);
    m_commandPalette->show();
}

void PetuniaMainWindow::setWorkspace(const QString &workspace) {
    m_activeWorkspace = workspace;
    m_statusLabel->setText(QStringLiteral("Workspace alterado para: %1").arg(workspace));

    if (workspace == QStringLiteral("ANIMATE")) {
        m_timelineDock->show();
        m_paintDock->hide();
        m_uvDock->hide();
    } else if (workspace == QStringLiteral("PAINT")) {
        m_paintDock->show();
        m_uvDock->hide();
        m_timelineDock->hide();
    } else if (workspace == QStringLiteral("UV")) {
        m_uvDock->show();
        m_paintDock->hide();
        m_timelineDock->hide();
    } else { // DRAW & POLY
        m_outlinerDock->show();
        m_inspectorDock->show();
        m_paintDock->hide();
        m_uvDock->hide();
        m_timelineDock->hide();
    }
}

void PetuniaMainWindow::updateSelectedCount(int count) {
    int domain = petunia_qt_app_get_selection_domain();
    if (count <= 0) {
        switch (domain) {
        case 1: m_selectionLabel->setText(QStringLiteral("Nenhum vértice selecionado")); break;
        case 2: m_selectionLabel->setText(QStringLiteral("Nenhuma aresta selecionada")); break;
        case 3: m_selectionLabel->setText(QStringLiteral("Nenhuma face selecionada")); break;
        default: m_selectionLabel->setText(QStringLiteral("Nenhum objeto selecionado")); break;
        }
    } else {
        switch (domain) {
        case 1: m_selectionLabel->setText(QStringLiteral("%1 vértice(s) selecionado(s)").arg(count)); break;
        case 2: m_selectionLabel->setText(QStringLiteral("%1 aresta(s) selecionada(s)").arg(count)); break;
        case 3: m_selectionLabel->setText(QStringLiteral("%1 face(s) selecionada(s)").arg(count)); break;
        default: m_selectionLabel->setText(QStringLiteral("%1 objeto(s) selecionado(s)").arg(count)); break;
        }
    }
}

void PetuniaMainWindow::updateDirtyState(bool isDirty) {
    setWindowModified(isDirty);
}

void PetuniaMainWindow::setStatusMessage(const QString &message) {
    m_statusLabel->setText(message);
}

void PetuniaMainWindow::populateOutliner(const QStringList &objectNames, const QList<bool> &visibilities) {
    m_syncingOutliner = true;
    bool needsRebuild = (m_outlinerModel->rowCount() != objectNames.size());
    if (!needsRebuild) {
        for (int i = 0; i < objectNames.size(); ++i) {
            if (m_outlinerModel->item(i, 0)->text() != objectNames[i]
                || (m_outlinerModel->item(i, 1)->text() == QStringLiteral("👁")) != visibilities.value(i, true)) {
                needsRebuild = true;
                break;
            }
        }
    }

    if (needsRebuild) {
        m_outlinerModel->removeRows(0, m_outlinerModel->rowCount());
        for (int i = 0; i < objectNames.size(); ++i) {
            auto *nameItem = new QStandardItem(objectNames[i]);
            nameItem->setEditable(false);

            auto *eyeItem = new QStandardItem(visibilities.value(i, true) ? QStringLiteral("👁") : QStringLiteral("—"));
            eyeItem->setEditable(false);

            auto *lockItem = new QStandardItem(QStringLiteral("🔓"));
            lockItem->setEditable(false);

            m_outlinerModel->appendRow({ nameItem, eyeItem, lockItem });
        }
    }

    int activeIdx = petunia_qt_app_get_active_asset();
    if (activeIdx >= 0 && activeIdx < m_outlinerModel->rowCount()) {
        QModelIndex srcIdx = m_outlinerModel->index(activeIdx, 0);
        QModelIndex proxyIdx = m_outlinerProxy->mapFromSource(srcIdx);
        if (proxyIdx.isValid()) {
            m_outlinerTree->setCurrentIndex(proxyIdx);
            m_outlinerTree->selectionModel()->select(
                proxyIdx,
                QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows
            );
        }
    } else {
        m_outlinerTree->selectionModel()->clearSelection();
    }
    m_syncingOutliner = false;
}

void PetuniaMainWindow::openPreferences() {
    PetuniaPreferencesDialog dlg(this);
    dlg.exec();
}

void PetuniaMainWindow::openReferenceManager() {
    PetuniaReferenceDialog dlg(this);
    dlg.exec();
}

void PetuniaMainWindow::openHomeDialog() {
    if (!m_homeDialog) {
        m_homeDialog = new PetuniaHomeDialog(this);
        connect(m_homeDialog, &PetuniaHomeDialog::newProjectRequested, this, &PetuniaMainWindow::newProject);
        connect(m_homeDialog, &PetuniaHomeDialog::openProjectRequested, this, &PetuniaMainWindow::openProject);
        connect(m_homeDialog, &PetuniaHomeDialog::recoverProjectRequested, this, &PetuniaMainWindow::openRecoveryDialog);
        connect(m_homeDialog, &PetuniaHomeDialog::settingsRequested, this, &PetuniaMainWindow::openPreferences);
        connect(m_homeDialog, &PetuniaHomeDialog::recentProjectSelected, this, [this](const QString &path) {
            if (petunia_qt_app_load_project(path.toUtf8().constData())) {
                m_currentProjectPath = path;
                setStatusMessage(QStringLiteral("Projeto carregado: %1").arg(path));
                syncFromApplication();
            }
        });
    }
    m_homeDialog->exec();
}

void PetuniaMainWindow::openRecoveryDialog() {
    if (!m_recoveryDialog) {
        m_recoveryDialog = new PetuniaRecoveryDialog(this);
        connect(m_recoveryDialog, &PetuniaRecoveryDialog::recoverRequested, this, [this]() {
            emit actionTriggered(QStringLiteral("file.recover_last_session"));
        });
        connect(m_recoveryDialog, &PetuniaRecoveryDialog::discardRequested, this, [this]() {
            emit actionTriggered(QStringLiteral("file.discard_recovery"));
        });
    }
    m_recoveryDialog->exec();
}

void PetuniaMainWindow::openAddMenu() {
    if (m_addMenu) {
        m_addMenu->exec(QCursor::pos());
    }
}

void PetuniaMainWindow::toggleSplitView() {
    if (m_secondaryViewportContainer) {
        bool show = !m_secondaryViewportContainer->isVisible();
        m_secondaryViewportContainer->setVisible(show);
        setStatusMessage(show ? QStringLiteral("Vista 3D dividida ativada") : QStringLiteral("Vista 3D individual restaurada"));
    }
}

void PetuniaMainWindow::toggleMicroInspector() {
    if (m_microInspector) {
        if (m_microInspector->isVisible()) {
            m_microInspector->hide();
        } else {
            m_microInspector->show();
            m_microInspector->raise();
        }
    }
}

void PetuniaMainWindow::showOperationHud(const QString &opName, const QString &details) {
    if (m_operationHud) {
        m_operationHud->showFeedback(opName, details);
    }
}

void PetuniaMainWindow::setSecondaryCameraPreset(const QString &preset) {
    setStatusMessage(QStringLiteral("Vista secundária: %1").arg(preset));
    if (preset == QStringLiteral("front")) petunia_qt_camera_set_preset(1);
    else if (preset == QStringLiteral("back")) petunia_qt_camera_set_preset(2);
    else if (preset == QStringLiteral("right")) petunia_qt_camera_set_preset(3);
    else if (preset == QStringLiteral("left")) petunia_qt_camera_set_preset(4);
    else if (preset == QStringLiteral("top")) petunia_qt_camera_set_preset(5);
    else if (preset == QStringLiteral("iso21")) petunia_qt_camera_set_preset(7);
    else petunia_qt_camera_set_preset(0); // persp

    if (m_secondaryViewport) {
        m_secondaryViewport->update();
    }
}

void PetuniaMainWindow::showPrimitiveCard(const QString &type) {
    if (m_primitiveCard) {
        m_primitiveCard->setPrimitiveType(type);
        m_primitiveCard->raise();
    }
}

void PetuniaMainWindow::showLoopCutCard() {
    if (m_loopCutCard) {
        m_loopCutCard->show();
        m_loopCutCard->raise();
    }
}

void PetuniaMainWindow::applyDarkTheme() {
    setStyleSheet(QStringLiteral(
        "QMainWindow { background-color: #121212; color: #eeeeee; }"
        "QMenuBar { background-color: #202020; color: #eeeeee; border-bottom: 1px solid #404040; }"
        "QMenuBar::item:selected { background-color: #353535; }"
        "QMenu { background-color: #292929; color: #eeeeee; border: 1px solid #404040; }"
        "QMenu::item:selected { background-color: #b58cff; color: #121212; }"
        "QToolBar { background-color: #202020; border-bottom: 1px solid #404040; spacing: 4px; padding: 2px; }"
        "QToolBar#ToolRailBar { border-right: 1px solid #404040; border-bottom: none; spacing: 6px; padding: 4px; }"
        "QToolBar#ToolRailBar QToolButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; font-weight: bold; font-size: 14px; min-width: 32px; min-height: 32px; }"
        "QToolBar#ToolRailBar QToolButton:hover { background-color: #353535; border-color: #b58cff; }"
        "QToolBar#ToolRailBar QToolButton:pressed { background-color: #e96a00; }"
        "QToolBar QPushButton { background-color: #292929; color: #b5b5b5; border: 1px solid #404040; border-radius: 4px; padding: 4px 12px; font-weight: bold; }"
        "QToolBar QPushButton:checked { background-color: #b58cff; color: #121212; border-color: #d1b8ff; }"
        "QToolBar QPushButton:hover:!checked { background-color: #353535; color: #eeeeee; }"
        "QDockWidget { color: #eeeeee; font-weight: bold; }"
        "QDockWidget::title { background-color: #202020; text-align: left; padding: 6px; border: 1px solid #404040; }"
        "QTreeView { background-color: #202020; color: #eeeeee; border: 1px solid #404040; selection-background-color: #e96a00; selection-color: #ffffff; }"
        "QHeaderView::section { background-color: #292929; color: #b5b5b5; border: 1px solid #404040; padding: 2px; }"
        "QLineEdit { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 4px; }"
        "QLineEdit:focus { border: 1px solid #b58cff; }"
        "QGroupBox { color: #b5b5b5; font-weight: bold; border: 1px solid #404040; border-radius: 4px; margin-top: 10px; padding-top: 10px; }"
        "QDoubleSpinBox, QSpinBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 3px; }"
        "QDoubleSpinBox:focus, QSpinBox:focus { border: 1px solid #b58cff; }"
        "QStatusBar { background-color: #202020; color: #b5b5b5; border-top: 1px solid #404040; }"
    ));
}

void PetuniaMainWindow::newProject() {
    petunia_qt_app_execute_command("file.new");
    m_currentProjectPath.clear();
    setStatusMessage(QStringLiteral("Novo projeto iniciado."));
    syncFromApplication();
}

void PetuniaMainWindow::openProject() {
    QString path = QFileDialog::getOpenFileName(
        this,
        QStringLiteral("Abrir Projeto Petunia3D"),
        QString(),
        QStringLiteral("Projetos Petunia (*.petunia *.json);;Todos os Arquivos (*)")
    );
    if (!path.isEmpty()) {
        if (petunia_qt_app_load_project(path.toUtf8().constData())) {
            m_currentProjectPath = path;
            setStatusMessage(QStringLiteral("Projeto carregado: %1").arg(path));
            syncFromApplication();
        } else {
            QMessageBox::warning(this, QStringLiteral("Erro"), QStringLiteral("Falha ao abrir o arquivo de projeto."));
        }
    }
}

void PetuniaMainWindow::saveProject() {
    if (m_currentProjectPath.isEmpty()) {
        saveProjectAs();
        return;
    }
    if (petunia_qt_app_save_project(m_currentProjectPath.toUtf8().constData())) {
        setStatusMessage(QStringLiteral("Projeto salvo com sucesso: %1").arg(m_currentProjectPath));
        updateDirtyState(false);
    } else {
        QMessageBox::warning(this, QStringLiteral("Erro"), QStringLiteral("Falha ao salvar o projeto no disco."));
    }
}

void PetuniaMainWindow::saveProjectAs() {
    QString path = QFileDialog::getSaveFileName(
        this,
        QStringLiteral("Salvar Projeto Petunia3D"),
        m_currentProjectPath.isEmpty() ? QStringLiteral("projeto.petunia") : m_currentProjectPath,
        QStringLiteral("Projetos Petunia (*.petunia);;Todos os Arquivos (*)")
    );
    if (!path.isEmpty()) {
        m_currentProjectPath = path;
        saveProject();
    }
}

void PetuniaMainWindow::exportGltf() {
    QString path = QFileDialog::getSaveFileName(
        this,
        QStringLiteral("Exportar Modelo 3D (GLTF/GLB)"),
        QStringLiteral("modelo.glb"),
        QStringLiteral("Binary GLTF (*.glb);;JSON GLTF (*.gltf);;Todos os Arquivos (*)")
    );
    if (!path.isEmpty()) {
        if (petunia_qt_app_export_gltf(path.toUtf8().constData())) {
            setStatusMessage(QStringLiteral("Modelo GLTF exportado com sucesso: %1").arg(path));
        } else {
            QMessageBox::warning(this, QStringLiteral("Erro"), QStringLiteral("Falha ao exportar modelo GLTF."));
        }
    }
}

void PetuniaMainWindow::exportObj() {
    QString path = QFileDialog::getSaveFileName(
        this,
        QStringLiteral("Exportar Wavefront OBJ"),
        QStringLiteral("modelo.obj"),
        QStringLiteral("Wavefront OBJ (*.obj);;Todos os Arquivos (*)")
    );
    if (!path.isEmpty()) {
        if (petunia_qt_app_export_obj(path.toUtf8().constData())) {
            setStatusMessage(QStringLiteral("Modelo OBJ exportado com sucesso: %1").arg(path));
        } else {
            QMessageBox::warning(this, QStringLiteral("Erro"), QStringLiteral("Falha ao exportar modelo OBJ."));
        }
    }
}

void PetuniaMainWindow::openAssetLibrary() {
    if (!m_assetLibraryWidget) {
        m_assetLibraryWidget = new PetuniaAssetLibraryWidget(this);
        connect(m_assetLibraryWidget, &PetuniaAssetLibraryWidget::assetSpawned, this, [this](const QString &cmd) {
            emit actionTriggered(cmd);
            setStatusMessage(QStringLiteral("Asset inserido na cena: %1").arg(cmd));
        });
    }
    m_assetLibraryWidget->show();
    m_assetLibraryWidget->raise();
    m_assetLibraryWidget->activateWindow();
}

extern "C" {
    static int s_argc = 1;
    static char s_app_name[] = "petunia3d";
    static char* s_argv[] = { s_app_name, nullptr };

    void* petunia_qt_app_create() {
        QSurfaceFormat fmt;
        fmt.setVersion(3, 3);
        fmt.setProfile(QSurfaceFormat::CoreProfile);
        QSurfaceFormat::setDefaultFormat(fmt);

        return new QApplication(s_argc, s_argv);
    }

    int petunia_qt_app_exec(void* app) {
        if (app) {
            return static_cast<QApplication*>(app)->exec();
        }
        return 0;
    }

    void petunia_qt_app_destroy(void* app) {
        delete static_cast<QApplication*>(app);
    }

    void* petunia_main_window_create() {
        return new PetuniaMainWindow();
    }

    void petunia_main_window_show(void* win) {
        if (win) {
            static_cast<PetuniaMainWindow*>(win)->show();
        }
    }

    void petunia_main_window_destroy(void* win) {
        delete static_cast<PetuniaMainWindow*>(win);
    }
}
