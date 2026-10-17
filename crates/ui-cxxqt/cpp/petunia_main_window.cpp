#include <cstddef>
#include <cstdint>
#include <vector>
extern "C" size_t petunia_qt_software_frame_rgba(uint64_t, uint32_t, uint32_t,
                                                 uint8_t *, size_t, uint32_t *,
                                                 uint32_t *);
#include "petunia_asset_library_widget.h"
#include "petunia_command_palette.h"
#include "petunia_home_dialog.h"
#include "petunia_main_window.h"
#include "petunia_overlay_cards.h"
#include "petunia_paint_panel.h"
#include "petunia_preferences_dialog.h"
#include "petunia_recovery_dialog.h"
#include "petunia_reference_dialog.h"
#include "petunia_timeline_widget.h"
#include "petunia_uv_panel.h"
#include "petunia_pie_menu.h"
#include "petunia_placeholder_message.h"
#include "petunia_dock_manager.h"
#include "petunia_icons.h"
#include "petunia_theme.h"
#include "petunia_number_field.h"
#include <QScrollArea>

#include <QAbstractSpinBox>
#include <QAccessible>
#include <QApplication>
#include <QButtonGroup>
#include <QColorDialog>
#include <QComboBox>
#include <QDoubleSpinBox>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonDocument>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPalette>
#include <QPointer>
#include <QScrollArea>
#include <QSettings>
#include <QShortcut>
#include <QSignalBlocker>
#include <QSplitter>
#include <QStatusBar>
#include <QSurfaceFormat>
#include <QSvgRenderer>
#include <QTextEdit>
#include <QThread>
#include <QTimer>
#include <QToolButton>
#include <QVBoxLayout>

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
bool petunia_qt_app_get_asset_rotation(int index, float *rx, float *ry,
                                       float *rz);
bool petunia_qt_app_set_asset_rotation(int index, float rx, float ry, float rz);
bool petunia_qt_app_get_asset_scale(int index, float *sx, float *sy, float *sz);
bool petunia_qt_app_set_asset_scale(int index, float sx, float sy, float sz);
bool petunia_qt_app_get_asset_color(int index, float *r, float *g, float *b);
bool petunia_qt_app_set_asset_color(int index, float r, float g, float b);
bool petunia_qt_app_get_scene_stats(int *verts_out, int *tris_out);
void petunia_qt_app_set_selection_domain(int domain);
void petunia_qt_app_frame_selection();
void petunia_qt_camera_set_preset(int preset);
void petunia_qt_camera_toggle_projection();
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
bool petunia_qt_app_get_modifier_info(int asset_index, int mod_index,
                                      char *name_buf, int max_len,
                                      bool *enabled, int *axis, float *weld);
bool petunia_qt_app_add_modifier(int asset_index, int kind, int axis,
                                 float weld);
bool petunia_qt_app_toggle_modifier(int asset_index, int mod_index,
                                    bool enabled);
bool petunia_qt_app_remove_modifier(int asset_index, int mod_index);
bool petunia_qt_app_apply_modifier(int asset_index, int mod_index);
bool petunia_qt_app_move_modifier(int asset_index, int mod_index, bool up);

bool petunia_qt_app_get_material_pbr(int asset_index, float *roughness,
                                     float *metallic, float *emission_strength,
                                     int *profile);
bool petunia_qt_app_set_material_pbr(int asset_index, float roughness,
                                     float metallic, float emission_strength,
                                     int profile);

int petunia_qt_app_get_surface_shading(int asset_index);
bool petunia_qt_app_set_surface_shading(int asset_index, int shading);

bool petunia_qt_app_quick_action(const char *action);
}

PetuniaMainWindow::PetuniaMainWindow(QWidget *parent) : QMainWindow(parent) {
    setupUi();
    qApp->installEventFilter(this);
    QSettings settings("Petunia3D", "QtStudio");
    restoreGeometry(settings.value("geometry").toByteArray());
    syncFromApplication();
    m_frameClock.start();
    auto *timer = new QTimer(this);
    timer->setInterval(33);
    connect(timer, &QTimer::timeout, this, [this] {
        const auto dt = m_frameClock.restart() / 1000.;
        const auto result = petuniaRequest("tick", {{"dt", dt}});
        if (result["changed"].toBool() || !result["ok"].toBool())
            syncEditorState(result);
    });
    timer->start();
    petuniaRequest("session_start");
    if (m_editorState["show_home_on_start"].toBool())
        QTimer::singleShot(0, this, &PetuniaMainWindow::openHomeDialog);
}

PetuniaMainWindow::~PetuniaMainWindow() { petuniaRequest("session_end"); }

void PetuniaMainWindow::setupUi() {
    resize(1280, 800);
    setWindowTitle(QStringLiteral("Petunia3D — Qt Studio[*]"));
    setDockOptions(QMainWindow::AnimatedDocks | QMainWindow::AllowNestedDocks |
                   QMainWindow::AllowTabbedDocks);

    // Contêiner Central: Divisor de Viewports com barras flutuantes adaptativas
    auto *centralContainer = new QWidget(this);
    auto *centralLayout = new QVBoxLayout(centralContainer);
    centralLayout->setContentsMargins(0, 0, 0, 0);
    centralLayout->setSpacing(0);

    // Divisor de Viewport (Suporte a Vista Dividida / Split View)
    m_viewportSplitter = new QSplitter(Qt::Horizontal, centralContainer);
    centralLayout->addWidget(m_viewportSplitter, 1);

    // Viewport Primária OpenGL 3.3
    auto *primaryContainer = new QWidget(m_viewportSplitter);
    auto *primaryLayout = new QVBoxLayout(primaryContainer);
    primaryLayout->setContentsMargins(0, 0, 0, 0);

    m_primaryStack = new QStackedWidget(primaryContainer);
    m_viewport = new PetuniaGlWidget(m_primaryStack);
    connect(m_viewport, &PetuniaGlWidget::selectionChanged, this,
            &PetuniaMainWindow::syncFromApplication);
    m_primaryStack->addWidget(m_viewport);
    m_primaryPaintCanvas = new PetuniaEditorCanvas("PAINT", m_primaryStack);
    m_primaryStack->addWidget(m_primaryPaintCanvas);
    connect(m_primaryPaintCanvas, &PetuniaEditorCanvas::editorChanged, this,
            &PetuniaMainWindow::syncFromApplication);
    m_pipViewport = new PetuniaGlWidget(m_primaryPaintCanvas);
    m_pipViewport->setSurfaceId(3);
    m_pipViewport->resize(240, 160);
    m_pipViewport->move(8, 8);
    connect(m_pipViewport, &PetuniaGlWidget::selectionChanged, this,
            &PetuniaMainWindow::syncFromApplication);
    primaryLayout->addWidget(m_primaryStack);
    m_viewportSplitter->addWidget(primaryContainer);

    // Barras Flutuantes Adaptativas na Viewport (Top View Bar e Bottom Context Bar)
    m_toolOptionsBar = new PetuniaToolOptionsBar(m_primaryStack);
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::selectionDomainChanged, this,
            &PetuniaMainWindow::setSelectionDomain);
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::cameraPresetRequested, this,
            &PetuniaMainWindow::setPrimaryCameraPreset);
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::planarNavigationToggled, this, [this]() {
        petuniaRequest("command", {{"id", "view.planar_navigation"}});
        syncFromApplication();
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::frameSelectionRequested, this, [this]() {
        petuniaRequest("command", {{"id", "view.frame_selection"}});
        syncFromApplication();
    });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::snapToggled, this,
            [this](bool enabled) {
                emit actionTriggered("tool.toggle_snap");
                setStatusMessage(enabled ? QStringLiteral("Snap ativado")
                                         : QStringLiteral("Snap desativado"));
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::proportionalToggled, this,
            [this](bool enabled) {
                emit actionTriggered("tool.toggle_proportional");
                setStatusMessage(
                    enabled ? QStringLiteral("Edição proporcional ativada")
                            : QStringLiteral("Edição proporcional desativada"));
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::shadingModeChanged, this,
            [this](const QString &mode) {
                if (m_viewport)
                    m_viewport->setShadingMode(mode);
                if (m_secondaryViewport)
                    m_secondaryViewport->setShadingMode(mode);
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::xrayToggled, this,
            [this](bool) {
                petuniaRequest("command", {{"id", "view.toggle_xray"}});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::matcapToggled, this,
            [this](bool enabled) {
                petuniaRequest("set_viewport_matcap", {{"enabled", enabled}});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::ambientOcclusionToggled,
            this, [this](bool enabled) {
                petuniaRequest("set_viewport_ambient_occlusion",
                               {{"enabled", enabled}});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::faceOrientationToggled,
            this, [this](bool) {
                petuniaRequest("face_orientation", {});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::uvCheckerToggled, this,
            [this](bool) {
                petuniaRequest("uv_checker", {});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::actionTriggered, this,
            &PetuniaMainWindow::actionTriggered);
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::snapTargetChanged, this,
            [this](const QString &id) {
                petuniaRequest("snap_target", {{"id", id}});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::proportionalRadiusChanged,
            this, [this](double value) {
                petuniaRequest("proportional_radius", {{"value", value}});
                syncFromApplication();
            });
    connect(m_toolOptionsBar, &PetuniaToolOptionsBar::pivotChanged, this,
            [this](const QString &id) {
                petuniaRequest("set_pivot_point", {{"id", id}});
                syncFromApplication();
            });

    m_contextBar = new PetuniaFloatingContextBar(m_primaryStack);
    connect(m_contextBar, &PetuniaFloatingContextBar::actionTriggered, this,
            [this](const QString &action) {
                petuniaRequest(action);
                syncFromApplication();
            });
    connect(m_contextBar, &PetuniaFloatingContextBar::commandExecuted, this,
            [this](const QString &cmd) {
                petuniaRequest("command", {{"id", cmd}});
                syncFromApplication();
            });
    connect(m_contextBar, &PetuniaFloatingContextBar::toolChanged, this,
            [this](const QString &tool) {
                petuniaRequest("tool", {{"id", tool}});
                syncFromApplication();
            });
    connect(m_contextBar, &PetuniaFloatingContextBar::brushSizeDelta, this,
            [this](float delta) {
                double current = m_editorState["brush_size"].toDouble(16.0);
                double next = qBound(1.0, current + delta, 2048.0);
                petuniaRequest("brush_size", {{"value", next}});
                syncFromApplication();
            });
    connect(m_contextBar, &PetuniaFloatingContextBar::brushOpacityDelta, this,
            [this](float delta) {
                double current = m_editorState["brush_opacity"].toDouble(1.0);
                double next = qBound(0.05, current + delta, 1.0);
                petuniaRequest("brush_opacity", {{"value", next}});
                syncFromApplication();
            });
    connect(m_contextBar, &PetuniaFloatingContextBar::symmetryToggled, this,
            [this](const QString &axis, bool val) {
                petuniaRequest(QStringLiteral("set_paint_symmetry_") + axis, {{"val", val}});
                syncFromApplication();
            });
    connect(m_contextBar, &PetuniaFloatingContextBar::colorPickerRequested, this,
            &PetuniaMainWindow::pickMaterialColor);
    connect(m_contextBar, &PetuniaFloatingContextBar::animateActionTriggered, this,
            [this](const QString &action) {
                if (action == "toggle_play")
                    petuniaRequest("animate", {{"id", "toggle-play"}});
                else if (action == "first")
                    petuniaRequest("animate", {{"id", "seek"}, {"value", 0.0}});
                else if (action == "prev_key")
                    petuniaRequest("animate", {{"id", "prev-keyframe"}});
                else if (action == "next_key")
                    petuniaRequest("animate", {{"id", "next-keyframe"}});
                else if (action == "insert_key")
                    petuniaRequest("animate", {{"id", "insert-keyframe"}});
                syncFromApplication();
            });

    // Cards Sobrepostos Flutuantes na Viewport Primária
    m_primitiveCard = new PetuniaPrimitiveCard(m_viewport);
    m_primitiveCard->move(20, 20);

    m_loopCutCard = new PetuniaLoopCutCard(m_viewport);
    m_loopCutCard->move(20, 20);

    m_microInspector = new PetuniaMicroInspector(m_viewport);
    m_microInspector->move(20, 160);
    connect(m_microInspector, &PetuniaMicroInspector::toolSelected, this,
            [this](const QString &toolId) {
                petuniaRequest("tool", {{"id", toolId}});
                syncFromApplication();
            });
    connect(m_microInspector, &PetuniaMicroInspector::modelingActionTriggered,
            this, &PetuniaMainWindow::actionTriggered);

    m_operationHud = new PetuniaOperationHud(m_viewport);
    m_operationHud->move(20, 360);

    // Viewport Secundária Dividida
    m_secondaryViewportContainer = new QWidget(m_viewportSplitter);
    auto *secondaryLayout = new QVBoxLayout(m_secondaryViewportContainer);
    secondaryLayout->setContentsMargins(0, 0, 0, 0);
    secondaryLayout->setSpacing(2);

    auto *secondaryBar = new QWidget(m_secondaryViewportContainer);
    secondaryBar->setFixedHeight(28);
    secondaryBar->setStyleSheet(QStringLiteral(
        "background-color: #202020; border-bottom: 1px solid #383838;"));
    auto *secondaryBarLayout = new QHBoxLayout(secondaryBar);
    secondaryBarLayout->setContentsMargins(4, 2, 4, 2);
    secondaryBarLayout->setSpacing(4);

    const struct {
        const char *id;
        const char *label;
    } presets[] = {{"persp", "Persp"}, {"front", "Frente"}, {"back", "Trás"},
                   {"left", "Esq"},    {"right", "Dir"},    {"top", "Topo"},
                   {"iso21", "2:1"}};
    for (const auto &p : presets) {
        auto *btn = new QPushButton(QString::fromUtf8(p.label), secondaryBar);
        btn->setFixedHeight(22);
        btn->setStyleSheet(QStringLiteral(
            "QPushButton { background: #282828; color: #bbb; border: 1px solid "
            "#404040; border-radius: 3px; font-size: 10px; padding: 0 6px; }"
            "QPushButton:hover { background: #353535; color: #fff; }"));
        connect(btn, &QPushButton::clicked, this,
                [this, id = QString::fromUtf8(p.id)]() {
                    setSecondaryCameraPreset(id);
                });
        secondaryBarLayout->addWidget(btn);
    }
    secondaryBarLayout->addStretch();

    auto *closeSplitBtn = new QPushButton(QStringLiteral("✕"), secondaryBar);
    closeSplitBtn->setFixedSize(20, 20);
    closeSplitBtn->setCursor(Qt::PointingHandCursor);
    closeSplitBtn->setStyleSheet(
        QStringLiteral("QPushButton { background: transparent; color: #888; "
                       "border: none; font-weight: bold; }"
                       "QPushButton:hover { color: #fff; background: #333; "
                       "border-radius: 2px; }"));
    connect(closeSplitBtn, &QPushButton::clicked, this,
            &PetuniaMainWindow::toggleSplitView);
    secondaryBarLayout->addWidget(closeSplitBtn);

    secondaryLayout->addWidget(secondaryBar);

    m_secondaryViewport = new PetuniaGlWidget(m_secondaryViewportContainer);
    m_secondaryViewport->setSurfaceId(2);
    connect(m_secondaryViewport, &PetuniaGlWidget::selectionChanged, this,
            &PetuniaMainWindow::syncFromApplication);
    secondaryLayout->addWidget(m_secondaryViewport);

    m_viewportSplitter->addWidget(m_secondaryViewportContainer);
    m_secondaryViewportContainer->hide();

    setCentralWidget(centralContainer);

    // Popover / Menu Adicionar Primitivas (Shift+A)
    m_addMenu = new PetuniaAddMenu(this);
    connect(m_addMenu, &PetuniaAddMenu::primitiveSelected, this,
            [this](const QString &id) {
                emit actionTriggered(QStringLiteral("model.add_%1").arg(id));
                showPrimitiveCard(id);
            });

    m_pieMenu = new PetuniaPieMenu(this);
    connect(m_pieMenu, &PetuniaPieMenu::primitiveSelected, this,
            [this](const QString &id) {
                emit actionTriggered(QStringLiteral("model.add_%1").arg(id));
                showPrimitiveCard(id);
            });

    auto *shiftAShortcut =
        new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_A), this);
    connect(shiftAShortcut, &QShortcut::activated, this,
            &PetuniaMainWindow::openAddMenu);

    auto *spaceShortcut = new QShortcut(QKeySequence(Qt::Key_Space), this);
    connect(spaceShortcut, &QShortcut::activated, this,
            &PetuniaMainWindow::toggleMicroInspector);

    setupDocks();
    setupMenuBar();
    setupWorkspaceBar();
    setupToolRail();
    setupStatusBar();

    connect(this, &PetuniaMainWindow::actionTriggered, this,
            [this](const QString &action) {
                QByteArray utf8 = action.toUtf8();
                if (action == "file.open") {
                    openProject();
                    return;
                }
                if (action == "file.save") {
                    saveProject();
                    return;
                }
                if (action == "file.save_as") {
                    saveProjectAs();
                    return;
                }
                if (action.startsWith("tool:"))
                    petuniaRequest("tool", {{"id", action.mid(5)}});
                else
                    petunia_qt_app_execute_command(utf8.constData());
                syncFromApplication();
            });

    connect(m_primitiveCard, &PetuniaPrimitiveCard::parameterChanged, this,
            [this](float size, int sides, int rings) {
                auto vm = petuniaRequest("snapshot");
                const auto kind = vm["primitive_kind"].toString();
                if (kind == "box" || kind == "cube" || kind == "plane") {
                    for (const auto &param :
                         QStringList{"width", "height", "depth"})
                        petuniaRequest("update_primitive_param_float",
                                       {{"param", param}, {"value", size}});
                } else
                    petuniaRequest("update_primitive_param_float",
                                   {{"param", "radius"}, {"value", size}});
                petuniaRequest("update_primitive_param_float",
                               {{"param", "sides"}, {"value", sides}});
                petuniaRequest("update_primitive_param_float",
                               {{"param", "rings"}, {"value", rings}});
                syncFromApplication();
            });
    connect(m_loopCutCard, &PetuniaLoopCutCard::loopCutChanged, this,
            [this](int cuts, float slide, float) {
                petuniaRequest("set_loop_cut_count", {{"cuts", cuts}});
                petuniaRequest("set_loop_cut_slide", {{"slide", slide}});
                syncFromApplication();
            });
    syncFromApplication();
}

void PetuniaMainWindow::setupMenuBar() {
    this->menuBar()->hide();
    m_headerMenuBar = new QMenuBar(this);
    m_headerMenuBar->setObjectName(QStringLiteral("HeaderMenuBar"));
    auto *menuBar = m_headerMenuBar;

    // 1. Menu Arquivo
    auto *fileMenu = menuBar->addMenu(QStringLiteral("&Arquivo"));
    auto *newAct = fileMenu->addAction(QStringLiteral("Novo Projeto"));
    newAct->setShortcut(QKeySequence::New);
    connect(newAct, &QAction::triggered, this, &PetuniaMainWindow::newProject);

    auto *newEmptyAct = fileMenu->addAction(QStringLiteral("Nova Cena Vazia"));
    newEmptyAct->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_N));
    connect(newEmptyAct, &QAction::triggered, this, [this]() {
        petuniaRequest("project.new_empty");
        syncFromApplication();
    });

    auto *openAct = fileMenu->addAction(QStringLiteral("Abrir..."));
    openAct->setShortcut(QKeySequence::Open);
    connect(openAct, &QAction::triggered, this,
            &PetuniaMainWindow::openProject);

    fileMenu->addSeparator();
    auto *saveAct = fileMenu->addAction(QStringLiteral("Salvar"));
    saveAct->setShortcut(QKeySequence::Save);
    connect(saveAct, &QAction::triggered, this,
            &PetuniaMainWindow::saveProject);

    auto *saveAsAct = fileMenu->addAction(QStringLiteral("Salvar Como..."));
    saveAsAct->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_S));
    connect(saveAsAct, &QAction::triggered, this,
            &PetuniaMainWindow::saveProjectAs);

    fileMenu->addSeparator();
    auto *exportMenu = fileMenu->addMenu(QStringLiteral("&Exportar"));
    auto *exportGltfAct =
        exportMenu->addAction(QStringLiteral("GLTF / GLB (*.glb)..."));
    connect(exportGltfAct, &QAction::triggered, this,
            &PetuniaMainWindow::exportGltf);

    auto *exportObjAct =
        exportMenu->addAction(QStringLiteral("Wavefront OBJ (*.obj)..."));
    connect(exportObjAct, &QAction::triggered, this,
            &PetuniaMainWindow::exportObj);

    // 2. Menu Editar
    auto *editMenu = menuBar->addMenu(QStringLiteral("&Editar"));
    auto *undoAct = editMenu->addAction(QStringLiteral("Desfazer"));
    undoAct->setProperty("editorAction", "undo");
    undoAct->setShortcut(QKeySequence::Undo);
    connect(undoAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_undo();
        syncFromApplication();
    });

    auto *redoAct = editMenu->addAction(QStringLiteral("Refazer"));
    redoAct->setProperty("editorAction", "redo");
    redoAct->setShortcut(QKeySequence::Redo);
    connect(redoAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_redo();
        syncFromApplication();
    });

    editMenu->addSeparator();
    auto *dupAct = editMenu->addAction(QStringLiteral("Duplicar"));
    dupAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_D));
    connect(dupAct, &QAction::triggered, this,
            [this]() { emit actionTriggered("edit.duplicate"); });

    auto *delAct = editMenu->addAction(QStringLiteral("Deletar"));
    delAct->setShortcut(QKeySequence::Delete);
    connect(delAct, &QAction::triggered, this,
            [this]() { emit actionTriggered("edit.delete"); });

    editMenu->addSeparator();
    auto *paletteAct =
        editMenu->addAction(QStringLiteral("Paleta de Comandos..."));
    paletteAct->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_P));
    connect(paletteAct, &QAction::triggered, this,
            &PetuniaMainWindow::openCommandPalette);

    auto *paletteF3 = new QAction(this);
    paletteF3->setShortcut(QKeySequence(Qt::Key_F3));
    connect(paletteF3, &QAction::triggered, this,
            &PetuniaMainWindow::openCommandPalette);
    addAction(paletteF3);

    auto *paletteCtrlK = new QAction(this);
    paletteCtrlK->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_K));
    connect(paletteCtrlK, &QAction::triggered, this,
            &PetuniaMainWindow::openCommandPalette);
    addAction(paletteCtrlK);

    editMenu->addSeparator();
    auto *prefAct = editMenu->addAction(QStringLiteral("Preferências..."));
    prefAct->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_Comma));
    connect(prefAct, &QAction::triggered, this,
            &PetuniaMainWindow::openPreferences);

    // 3. Menu Seleção
    auto *selectMenu = menuBar->addMenu(QStringLiteral("&Seleção"));
    auto *selAll = selectMenu->addAction(QStringLiteral("Selecionar Tudo"));
    selAll->setShortcut(QKeySequence::SelectAll);
    connect(selAll, &QAction::triggered, this,
            [this]() { emit actionTriggered("select.all"); });

    auto *selNone = selectMenu->addAction(QStringLiteral("Limpar Seleção"));
    selNone->setShortcut(QKeySequence(Qt::ALT | Qt::Key_A));
    connect(selNone, &QAction::triggered, this,
            [this]() { emit actionTriggered("select.clear"); });

    auto *selInv = selectMenu->addAction(QStringLiteral("Inverter Seleção"));
    selInv->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_I));
    connect(selInv, &QAction::triggered, this,
            [this]() { emit actionTriggered("select.invert"); });

    // 4. Menu Adicionar
    auto *addMenu = menuBar->addMenu(QStringLiteral("&Adicionar"));
    auto *addMenuPopAct =
        addMenu->addAction(QStringLiteral("Menu de Primitivas..."));
    addMenuPopAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_A));
    connect(addMenuPopAct, &QAction::triggered, this,
            &PetuniaMainWindow::openAddMenu);

    auto *assetLibAct =
        addMenu->addAction(QStringLiteral("Biblioteca de Modelos & Assets..."));
    assetLibAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_F1));
    connect(assetLibAct, &QAction::triggered, this,
            &PetuniaMainWindow::openAssetLibrary);

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

    // 5. Menu Objeto
    auto *objectMenu = menuBar->addMenu(QStringLiteral("&Objeto"));
    auto *alignMenu = objectMenu->addMenu(QStringLiteral("&Alinhar"));

    auto *alignCenterX = alignMenu->addAction(QStringLiteral("Centralizar em X"));
    connect(alignCenterX, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_center_x");
    });

    auto *alignCenterY = alignMenu->addAction(QStringLiteral("Centralizar em Y"));
    connect(alignCenterY, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_center_y");
    });

    auto *alignCenterZ = alignMenu->addAction(QStringLiteral("Centralizar em Z"));
    connect(alignCenterZ, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_center_z");
    });

    alignMenu->addSeparator();

    auto *alignMinX = alignMenu->addAction(QStringLiteral("Alinhar à Esquerda (Min X)"));
    connect(alignMinX, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_min_x");
    });

    auto *alignMaxX = alignMenu->addAction(QStringLiteral("Alinhar à Direita (Max X)"));
    connect(alignMaxX, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_max_x");
    });

    auto *alignMinY = alignMenu->addAction(QStringLiteral("Apoiar na Base (Min Y)"));
    connect(alignMinY, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_min_y");
    });

    auto *alignMaxY = alignMenu->addAction(QStringLiteral("Alinhar ao Topo (Max Y)"));
    connect(alignMaxY, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_max_y");
    });

    alignMenu->addSeparator();

    auto *alignMinZ = alignMenu->addAction(QStringLiteral("Alinhar ao Fundo (Min Z)"));
    connect(alignMinZ, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_min_z");
    });

    auto *alignMaxZ = alignMenu->addAction(QStringLiteral("Alinhar à Frente (Max Z)"));
    connect(alignMaxZ, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.align_max_z");
    });

    alignMenu->addSeparator();

    auto *distribX = alignMenu->addAction(QStringLiteral("Distribuir Espaçamento em X"));
    connect(distribX, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.distribute_objects_x");
    });

    auto *distribY = alignMenu->addAction(QStringLiteral("Distribuir Espaçamento em Y"));
    connect(distribY, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.distribute_objects_y");
    });

    auto *distribZ = alignMenu->addAction(QStringLiteral("Distribuir Espaçamento em Z"));
    connect(distribZ, &QAction::triggered, this, [this]() {
        emit actionTriggered("model.distribute_objects_z");
    });

    // 6. Menu Ferramentas
    auto *toolsMenu = menuBar->addMenu(QStringLiteral("&Ferramentas"));
    auto *refAct = toolsMenu->addAction(
        QStringLiteral("Gerenciador de Imagens de Referência..."));
    connect(refAct, &QAction::triggered, this,
            &PetuniaMainWindow::openReferenceManager);

    toolsMenu->addSeparator();
    auto *snapAct = toolsMenu->addAction(QStringLiteral("Snap ao Grid"));
    snapAct->setShortcut(QKeySequence(Qt::SHIFT | Qt::Key_Tab));
    connect(snapAct, &QAction::triggered, this,
            [this]() { emit actionTriggered("tool.toggle_snap"); });

    // 6. Menu Exibir
    auto *viewMenu = menuBar->addMenu(QStringLiteral("&Exibir"));
    viewMenu->addAction(m_outlinerDock->toggleViewAction());
    viewMenu->addAction(m_inspectorDock->toggleViewAction());
    viewMenu->addAction(m_timelineDock->toggleViewAction());
    viewMenu->addAction(m_paintDock->toggleViewAction());
    viewMenu->addAction(m_uvDock->toggleViewAction());
    viewMenu->addSeparator();

    auto *splitViewAct =
        viewMenu->addAction(QStringLiteral("Dividir Vista 3D"));
    splitViewAct->setShortcut(QKeySequence(Qt::CTRL | Qt::ALT | Qt::Key_Q));
    connect(splitViewAct, &QAction::triggered, this,
            &PetuniaMainWindow::toggleSplitView);

    auto *microInspAct = viewMenu->addAction(QStringLiteral("Micro Inspetor"));
    microInspAct->setShortcut(QKeySequence(Qt::Key_Space));
    connect(microInspAct, &QAction::triggered, this,
            &PetuniaMainWindow::toggleMicroInspector);

    viewMenu->addSeparator();

    auto *frameAct = viewMenu->addAction(QStringLiteral("Focar Seleção"));
    frameAct->setShortcut(QKeySequence(Qt::Key_F));
    connect(frameAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_frame_selection();
        if (m_viewport)
            m_viewport->update();
    });

    auto *resetCam = viewMenu->addAction(QStringLiteral("Redefinir Câmera"));
    resetCam->setShortcut(QKeySequence(Qt::Key_Home));
    connect(resetCam, &QAction::triggered, this, [this]() {
        petunia_qt_camera_reset();
        if (m_viewport)
            m_viewport->update();
    });

    viewMenu->addSeparator();

    auto *resetLayoutAct = viewMenu->addAction(QStringLiteral("Restaurar Layout Deste Workspace"));
    connect(resetLayoutAct, &QAction::triggered, this, [this]() {
        QSettings settings("Petunia3D", "QtStudio");
        settings.remove("workspace/" + m_activeWorkspace);
        if (m_dockManager) {
            m_dockManager->applyWorkspacePreset(m_activeWorkspace);
        }
        setStatusMessage(tr("Layout do workspace redefinido para o padrão"));
    });

    auto *resetAllLayoutsAct = viewMenu->addAction(QStringLiteral("Restaurar Todos os Layouts de Fábrica"));
    connect(resetAllLayoutsAct, &QAction::triggered, this, [this]() {
        QSettings settings("Petunia3D", "QtStudio");
        settings.remove("workspace");
        if (m_dockManager) {
            for (const auto &ws : {"DRAW", "POLY", "PAINT", "UV", "ANIMATE"}) {
                m_dockManager->applyWorkspacePreset(ws);
            }
            m_dockManager->applyWorkspacePreset(m_activeWorkspace);
        }
        setStatusMessage(tr("Todos os layouts de workspace redefinidos"));
    });

    // 7. Menu Ajuda
    auto *helpMenu = menuBar->addMenu(QStringLiteral("&Ajuda"));
    auto *homeAct =
        helpMenu->addAction(QStringLiteral("Tela Inicial (Home)..."));
    connect(homeAct, &QAction::triggered, this,
            &PetuniaMainWindow::openHomeDialog);

    auto *recAct = helpMenu->addAction(QStringLiteral("Recuperar Sessão..."));
    connect(recAct, &QAction::triggered, this,
            &PetuniaMainWindow::openRecoveryDialog);

    helpMenu->addSeparator();

    auto *aboutAct = helpMenu->addAction(QStringLiteral("Sobre o Petunia3D"));
    connect(aboutAct, &QAction::triggered, this, [this]() {
        QMessageBox::about(
            this, QStringLiteral("Sobre o Petunia3D"),
            QStringLiteral("<h3>Petunia3D — Qt Studio</h3><p>Editor 3D · "
                           "GPL-3.0-or-later</p>"));
    });
}

void PetuniaMainWindow::setupWorkspaceBar() {
    auto *wsBar = addToolBar(QStringLiteral("Workspaces"));
    wsBar->setObjectName(QStringLiteral("WorkspaceToolBar"));
    wsBar->setMovable(false);

    // 1. Logo Petunia sem escrita
    auto *logoLabel = new QLabel(wsBar);
    logoLabel->setPixmap(PetuniaIcons::get("logo").pixmap(20, 20));
    logoLabel->setContentsMargins(6, 0, 8, 0);
    wsBar->addWidget(logoLabel);

    // 2. Barra de Ferramentas / Menus
    if (m_headerMenuBar) {
        m_headerMenuBar->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Preferred);
        wsBar->addWidget(m_headerMenuBar);
    }

    // 3. Spacer expansível à esquerda
    auto *leftSpacer = new QWidget(wsBar);
    leftSpacer->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    wsBar->addWidget(leftSpacer);

    // 4. Workspaces no Centro (DRAW, POLY, PAINT, UV, ANIMATE)
    auto *wsContainer = new QFrame(wsBar);
    wsContainer->setObjectName(QStringLiteral("WorkspacePillGroup"));
    auto *wsLayout = new QHBoxLayout(wsContainer);
    wsLayout->setContentsMargins(2, 2, 2, 2);
    wsLayout->setSpacing(2);

    const QStringList wsList = {"DRAW", "POLY", "PAINT", "UV", "ANIMATE"};
    auto *wsGroup = new QButtonGroup(this);
    for (const auto &ws : wsList) {
        auto *btn = new QPushButton(ws, wsContainer);
        btn->setProperty("workspaceId", ws);
        btn->setCheckable(true);
        btn->setChecked(ws == m_activeWorkspace);
        wsGroup->addButton(btn);
        connect(btn, &QPushButton::clicked, this, [this, ws]() {
            setWorkspace(ws);
            emit workspaceChanged(ws);
        });
        wsLayout->addWidget(btn);
    }
    wsBar->addWidget(wsContainer);

    // 5. Spacer expansível à direita para manter workspaces perfeitamente centrados
    auto *rightSpacer = new QWidget(wsBar);
    rightSpacer->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    wsBar->addWidget(rightSpacer);

    // 6. Refazer e Desfazer
    auto *undoBtn = new QPushButton(PetuniaIcons::get("undo"), QString(), wsBar);
    undoBtn->setToolTip(QStringLiteral("Desfazer (Ctrl+Z)"));
    undoBtn->setFixedSize(26, 26);
    connect(undoBtn, &QPushButton::clicked, this, [this]() {
        petuniaRequest("undo");
        syncFromApplication();
    });
    wsBar->addWidget(undoBtn);

    auto *redoBtn = new QPushButton(PetuniaIcons::get("redo"), QString(), wsBar);
    redoBtn->setToolTip(QStringLiteral("Refazer (Ctrl+Y)"));
    redoBtn->setFixedSize(26, 26);
    connect(redoBtn, &QPushButton::clicked, this, [this]() {
        petuniaRequest("redo");
        syncFromApplication();
    });
    wsBar->addWidget(redoBtn);

    // 7. Barra de Busca
    auto *searchBtn = new QPushButton(PetuniaIcons::get("search"), QStringLiteral(" Buscar... (Ctrl+K)"), wsBar);
    searchBtn->setObjectName(QStringLiteral("HeaderSearchBtn"));
    searchBtn->setFixedHeight(26);
    connect(searchBtn, &QPushButton::clicked, this, &PetuniaMainWindow::openCommandPalette);
    wsBar->addWidget(searchBtn);

    // 8. Botão de Configurações
    auto *prefBtn = new QPushButton(PetuniaIcons::get("settings"), QString(), wsBar);
    prefBtn->setToolTip(QStringLiteral("Preferências (Ctrl+,)"));
    prefBtn->setFixedSize(26, 26);
    connect(prefBtn, &QPushButton::clicked, this, &PetuniaMainWindow::openPreferences);
    wsBar->addWidget(prefBtn);

    // Atalhos ergonômicos para seleção de domínio (1, 2, 3)
    auto *scObj = new QShortcut(QKeySequence(Qt::Key_1), this);
    connect(scObj, &QShortcut::activated, this, [this]() { setSelectionDomain(0); });

    auto *scEdge = new QShortcut(QKeySequence(Qt::Key_2), this);
    connect(scEdge, &QShortcut::activated, this, [this]() { setSelectionDomain(2); });

    auto *scFace = new QShortcut(QKeySequence(Qt::Key_3), this);
    connect(scFace, &QShortcut::activated, this, [this]() { setSelectionDomain(3); });
}

void PetuniaMainWindow::setupToolRail() {
    m_toolRailBar = addToolBar(QStringLiteral("Ferramentas"));
    m_toolRailBar->setObjectName(QStringLiteral("ToolRailBar"));
    m_toolRailBar->setOrientation(Qt::Vertical);
    m_toolRailBar->setToolButtonStyle(Qt::ToolButtonIconOnly);
    m_toolRailBar->setIconSize(QSize(20, 20));
    addToolBar(Qt::LeftToolBarArea, m_toolRailBar);
    m_toolRailBar->setMovable(false);

    auto addRailTool = [this](const QString &iconName, const QString &tooltip,
                              const QString &cmd,
                              const QKeySequence &shortcut = QKeySequence()) {
        auto *act = m_toolRailBar->addAction(PetuniaIcons::get(iconName), QString());
        act->setToolTip(tooltip);
        if (!shortcut.isEmpty()) {
            act->setShortcut(shortcut);
        }
        connect(act, &QAction::triggered, this,
                [this, cmd]() { emit actionTriggered(cmd); });
        return act;
    };

    addRailTool(QStringLiteral("move"), QStringLiteral("Selecionar / Mover (W)"),
                QStringLiteral("tool:select"));
    addRailTool(QStringLiteral("extrude"), QStringLiteral("Extrusão (E)"),
                QStringLiteral("tool:extrude"), QKeySequence(Qt::Key_E));
    addRailTool(QStringLiteral("inset"), QStringLiteral("Inset de Faces (I)"),
                QStringLiteral("tool:inset"), QKeySequence(Qt::Key_I));
    addRailTool(QStringLiteral("bevel"), QStringLiteral("Bevel / Chanfro (Ctrl+B)"),
                QStringLiteral("tool:bevel"),
                QKeySequence(Qt::CTRL | Qt::Key_B));

    auto *loopCutAct = addRailTool(
        QStringLiteral("loop_cut"), QStringLiteral("Loop Cut (Ctrl+R)"),
        QStringLiteral("tool:loop_cut"), QKeySequence(Qt::CTRL | Qt::Key_R));
    connect(loopCutAct, &QAction::triggered, this,
            &PetuniaMainWindow::showLoopCutCard);

    addRailTool(QStringLiteral("subdivide"), QStringLiteral("Subdividir Malha"),
                QStringLiteral("model.subdivide"));

    m_toolRailBar->addSeparator();

    addRailTool(
        QStringLiteral("duplicate"), QStringLiteral("Duplicar Objeto (Shift+D)"),
        QStringLiteral("edit.duplicate"), QKeySequence(Qt::SHIFT | Qt::Key_D));
    addRailTool(QStringLiteral("delete"), QStringLiteral("Deletar Seleção (Del)"),
                QStringLiteral("edit.delete"), QKeySequence(Qt::Key_Delete));

    m_toolRailBar->addSeparator();

    auto *frameAct = m_toolRailBar->addAction(
        PetuniaIcons::get("frame_selection"), QString());
    frameAct->setToolTip(QStringLiteral("Focar Seleção na Câmera (F)"));
    connect(frameAct, &QAction::triggered, this, [this]() {
        petunia_qt_app_frame_selection();
        if (m_viewport)
            m_viewport->update();
    });
}

class PetuniaOutlinerProxyModel : public QSortFilterProxyModel {
public:
    using QSortFilterProxyModel::QSortFilterProxyModel;
    bool filterSelectedOnly{false};

    void setFilterSelectedOnly(bool enabled) {
        if (filterSelectedOnly != enabled) {
            filterSelectedOnly = enabled;
            invalidate();
        }
    }

protected:
    bool filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const override {
        if (filterSelectedOnly) {
            auto *model = qobject_cast<QStandardItemModel *>(sourceModel());
            if (model) {
                auto *item = model->item(sourceRow, 0);
                if (item && !item->data(Qt::UserRole + 1).toBool())
                    return false;
            }
        }
        return QSortFilterProxyModel::filterAcceptsRow(sourceRow, sourceParent);
    }
};

void PetuniaMainWindow::setupDocks() {
    m_dockManager = new PetuniaDockManager(this, this);

    m_outlinerDock = new QDockWidget(tr("Partes"), this);
    m_outlinerDock->setObjectName("OutlinerDock");
    auto *container = new QWidget(m_outlinerDock);
    auto *layout = new QVBoxLayout(container);
    layout->setContentsMargins(4, 4, 4, 4);
    layout->setSpacing(4);

    // Search & Action Bar (Search, Filter, Sort, Separate)
    auto *searchRow = new QHBoxLayout();
    searchRow->setContentsMargins(0, 0, 0, 0);
    searchRow->setSpacing(3);

    m_outlinerSearch = new QLineEdit(container);
    m_outlinerSearch->setPlaceholderText(tr("Buscar partes"));
    m_outlinerSearch->setAccessibleName(tr("Buscar partes"));
    searchRow->addWidget(m_outlinerSearch, 1);

    m_outlinerFilterBtn = new QPushButton(container);
    m_outlinerFilterBtn->setCheckable(true);
    m_outlinerFilterBtn->setFixedSize(26, 26);
    m_outlinerFilterBtn->setIcon(PetuniaIcons::get(QStringLiteral("filter")));
    m_outlinerFilterBtn->setIconSize(QSize(16, 16));
    m_outlinerFilterBtn->setToolTip(tr("Exibir apenas partes selecionadas"));
    m_outlinerFilterBtn->setAccessibleName(tr("Show selected parts only"));
    searchRow->addWidget(m_outlinerFilterBtn);

    m_outlinerSortBtn = new QPushButton(container);
    m_outlinerSortBtn->setCheckable(true);
    m_outlinerSortBtn->setFixedSize(26, 26);
    m_outlinerSortBtn->setIcon(PetuniaIcons::get(QStringLiteral("list_view")));
    m_outlinerSortBtn->setIconSize(QSize(16, 16));
    m_outlinerSortBtn->setToolTip(tr("Ordenar partes por nome (A-Z)"));
    m_outlinerSortBtn->setAccessibleName(tr("Sort parts by name"));
    searchRow->addWidget(m_outlinerSortBtn);

    m_outlinerSeparateBtn = new QPushButton(container);
    m_outlinerSeparateBtn->setFixedSize(26, 26);
    m_outlinerSeparateBtn->setIcon(PetuniaIcons::get(QStringLiteral("separate")));
    m_outlinerSeparateBtn->setIconSize(QSize(16, 16));
    m_outlinerSeparateBtn->setToolTip(tr("Separar seleção para nova parte (P)"));
    m_outlinerSeparateBtn->setAccessibleName(tr("Separate to new part"));
    connect(m_outlinerSeparateBtn, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("model.separate_selection"));
    });
    searchRow->addWidget(m_outlinerSeparateBtn);

    layout->addLayout(searchRow);

    m_outlinerTree = new QTreeView(container);
    m_outlinerTree->setAccessibleName(tr("Partes da cena"));
    m_outlinerTree->setMinimumWidth(230);
    m_outlinerTree->setRootIsDecorated(false);
    m_outlinerTree->setIndentation(0);
    m_outlinerTree->setSelectionMode(QAbstractItemView::ExtendedSelection);
    m_outlinerTree->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_outlinerTree->setContextMenuPolicy(Qt::CustomContextMenu);
    m_outlinerTree->setDragEnabled(true);
    m_outlinerTree->setAcceptDrops(true);
    m_outlinerTree->setDropIndicatorShown(true);
    m_outlinerTree->setDragDropMode(QAbstractItemView::InternalMove);
    m_outlinerTree->setStyleSheet(QStringLiteral("QTreeView::item { height: 26px; }"));
    m_outlinerTree->setIconSize(QSize(16, 16));

    auto *outlinerDelShortcut = new QShortcut(QKeySequence(Qt::Key_Delete), m_outlinerTree);
    connect(outlinerDelShortcut, &QShortcut::activated, this, [this]() {
        petuniaRequest("command", {{"id", "edit.delete"}});
        syncFromApplication();
    });
    auto *outlinerBkspShortcut = new QShortcut(QKeySequence(Qt::Key_Backspace), m_outlinerTree);
    connect(outlinerBkspShortcut, &QShortcut::activated, this, [this]() {
        petuniaRequest("command", {{"id", "edit.delete"}});
        syncFromApplication();
    });
    auto *outlinerSoloShortcut = new QShortcut(QKeySequence(Qt::SHIFT | Qt::Key_H), m_outlinerTree);
    connect(outlinerSoloShortcut, &QShortcut::activated, this, [this]() {
        petuniaRequest("isolate");
        syncFromApplication();
    });
    auto *outlinerVisShortcut = new QShortcut(QKeySequence(Qt::Key_H), m_outlinerTree);
    connect(outlinerVisShortcut, &QShortcut::activated, this, [this]() {
        for (const auto &row : m_outlinerTree->selectionModel()->selectedRows()) {
            auto src = m_outlinerProxy->mapToSource(row);
            auto *item = m_outlinerModel->item(src.row(), 0);
            if (item) {
                petuniaRequest("visibility", {{"id", item->data(Qt::UserRole).toString()}});
            }
        }
        syncFromApplication();
    });
    auto *outlinerLockShortcut = new QShortcut(QKeySequence(Qt::Key_L), m_outlinerTree);
    connect(outlinerLockShortcut, &QShortcut::activated, this, [this]() {
        for (const auto &row : m_outlinerTree->selectionModel()->selectedRows()) {
            auto src = m_outlinerProxy->mapToSource(row);
            auto *item = m_outlinerModel->item(src.row(), 0);
            if (item) {
                petuniaRequest("lock", {{"id", item->data(Qt::UserRole).toString()}});
            }
        }
        syncFromApplication();
    });
    auto *outlinerRenameShortcut = new QShortcut(QKeySequence(Qt::Key_F2), m_outlinerTree);
    connect(outlinerRenameShortcut, &QShortcut::activated, this, [this]() {
        auto current = m_outlinerTree->currentIndex();
        if (current.isValid()) {
            m_outlinerTree->edit(current.siblingAtColumn(0));
        }
    });
    auto *outlinerDupShortcut = new QShortcut(QKeySequence(Qt::CTRL | Qt::Key_D), m_outlinerTree);
    connect(outlinerDupShortcut, &QShortcut::activated, this, [this]() {
        for (const auto &row : m_outlinerTree->selectionModel()->selectedRows()) {
            auto src = m_outlinerProxy->mapToSource(row);
            auto *item = m_outlinerModel->item(src.row(), 0);
            if (item) {
                petuniaRequest("scene_context", {{"id", item->data(Qt::UserRole).toString()}, {"target", "duplicate"}});
            }
        }
        syncFromApplication();
    });

    m_outlinerModel = new QStandardItemModel(this);
    m_outlinerModel->setHorizontalHeaderLabels(
        {tr("Nome"), tr("Visível"), tr("Bloqueio")});

    auto *proxyModel = new PetuniaOutlinerProxyModel(this);
    proxyModel->setSourceModel(m_outlinerModel);
    proxyModel->setFilterCaseSensitivity(Qt::CaseInsensitive);
    proxyModel->setFilterKeyColumn(0);
    m_outlinerProxy = proxyModel;
    m_outlinerTree->setModel(m_outlinerProxy);
    layout->addWidget(m_outlinerTree, 1);

    m_outlinerPlaceholder = new PetuniaPlaceholderMessage(container);
    m_outlinerPlaceholder->setTitle(tr("Cena Vazia"));
    m_outlinerPlaceholder->setExplanation(tr("Nenhum objeto na cena. Adicione primitivas para começar a modelar."));
    m_outlinerPlaceholder->setActionText(tr("+ Adicionar Primitiva"));
    connect(m_outlinerPlaceholder, &PetuniaPlaceholderMessage::actionTriggered, this, &PetuniaMainWindow::openAddMenu);
    layout->addWidget(m_outlinerPlaceholder, 1);
    m_outlinerPlaceholder->hide();

    // Row Size Slider (Accessible: "Row size", range 18-36, default 22)
    auto *rowSizeContainer = new QWidget(container);
    auto *rowSizeLayout = new QHBoxLayout(rowSizeContainer);
    rowSizeLayout->setContentsMargins(2, 0, 2, 0);
    rowSizeLayout->setSpacing(6);

    auto *rowSizeLabel = new QLabel(tr("Row size"), rowSizeContainer);
    rowSizeLabel->setStyleSheet(QStringLiteral("color: #888888; font-size: 11px;"));
    rowSizeLayout->addWidget(rowSizeLabel);

    m_outlinerRowSizeSlider = new QSlider(Qt::Horizontal, rowSizeContainer);
    m_outlinerRowSizeSlider->setObjectName(QStringLiteral("model-inspector-parts-row-height"));
    m_outlinerRowSizeSlider->setAccessibleName(tr("Row size"));
    m_outlinerRowSizeSlider->setAccessibleDescription(tr("Tamanho das linhas da árvore de partes"));
    m_outlinerRowSizeSlider->setRange(18, 36);
    m_outlinerRowSizeSlider->setSingleStep(1);
    m_outlinerRowSizeSlider->setValue(22);
    m_outlinerRowSizeSlider->setMinimumWidth(100);
    rowSizeLayout->addWidget(m_outlinerRowSizeSlider, 1);

    auto *rowSizeValLabel = new QLabel(QStringLiteral("22"), rowSizeContainer);
    rowSizeValLabel->setStyleSheet(QStringLiteral("color: #888888; font-size: 11px; min-width: 16px;"));
    rowSizeLayout->addWidget(rowSizeValLabel);

    connect(m_outlinerRowSizeSlider, &QSlider::valueChanged, this,
            [this, rowSizeValLabel](int h) {
                m_outlinerRowHeight = h;
                rowSizeValLabel->setText(QString::number(h));
                m_outlinerTree->setStyleSheet(
                    QStringLiteral("QTreeView::item { height: %1px; }").arg(h));
            });

    layout->addWidget(rowSizeContainer);
    rowSizeContainer->hide();

    m_outlinerTree->header()->setSectionResizeMode(0, QHeaderView::Stretch);
    m_outlinerTree->header()->setSectionResizeMode(1, QHeaderView::Fixed);
    m_outlinerTree->header()->resizeSection(1, 26);
    m_outlinerTree->header()->setSectionResizeMode(2, QHeaderView::Fixed);
    m_outlinerTree->header()->resizeSection(2, 26);
    m_outlinerTree->setHeaderHidden(true);

    connect(m_outlinerSearch, &QLineEdit::textChanged, m_outlinerProxy,
            &QSortFilterProxyModel::setFilterFixedString);
    connect(m_outlinerFilterBtn, &QPushButton::toggled, this, [this](bool checked) {
        if (auto *p = static_cast<PetuniaOutlinerProxyModel *>(m_outlinerProxy)) {
            p->setFilterSelectedOnly(checked);
        }
    });
    connect(m_outlinerSortBtn, &QPushButton::toggled, this, [this](bool checked) {
        if (checked) {
            m_outlinerProxy->sort(0, Qt::AscendingOrder);
        } else {
            m_outlinerProxy->sort(-1);
        }
    });
    connect(m_outlinerTree, &QTreeView::customContextMenuRequested, this,
            &PetuniaMainWindow::onOutlinerContextMenu);
    connect(m_outlinerTree, &QTreeView::clicked, this,
            [this](const QModelIndex &proxy) {
                if (m_syncingOutliner)
                    return;
                auto src = m_outlinerProxy->mapToSource(proxy);
                const auto id = m_outlinerModel->item(src.row(), 0)
                                    ->data(Qt::UserRole)
                                    .toString();
                auto selectedRows = m_outlinerTree->selectionModel()->selectedRows();
                if (src.column() == 1) {
                    if (selectedRows.size() > 1 && selectedRows.contains(proxy.siblingAtColumn(0))) {
                        for (const auto &pRow : selectedRows) {
                            auto sRow = m_outlinerProxy->mapToSource(pRow);
                            auto *it = m_outlinerModel->item(sRow.row(), 0);
                            if (it) petuniaRequest("visibility", {{"id", it->data(Qt::UserRole).toString()}});
                        }
                    } else {
                        petuniaRequest("visibility", {{"id", id}});
                    }
                } else if (src.column() == 2) {
                    if (selectedRows.size() > 1 && selectedRows.contains(proxy.siblingAtColumn(0))) {
                        for (const auto &pRow : selectedRows) {
                            auto sRow = m_outlinerProxy->mapToSource(pRow);
                            auto *it = m_outlinerModel->item(sRow.row(), 0);
                            if (it) petuniaRequest("lock", {{"id", it->data(Qt::UserRole).toString()}});
                        }
                    } else {
                        petuniaRequest("lock", {{"id", id}});
                    }
                }
                syncFromApplication();
            });
    connect(m_outlinerTree->selectionModel(),
            &QItemSelectionModel::selectionChanged, this, [this] {
                if (m_syncingOutliner)
                    return;
                for (int r = 0; r < m_outlinerModel->rowCount(); ++r) {
                    auto *item = m_outlinerModel->item(r, 0);
                    if (item) {
                        const auto pIdx = m_outlinerProxy->mapFromSource(m_outlinerModel->index(r, 0));
                        const bool sel = pIdx.isValid() && m_outlinerTree->selectionModel()->isRowSelected(pIdx.row(), QModelIndex());
                        item->setData(sel, Qt::UserRole + 1);
                    }
                }
                QJsonArray ids;
                for (const auto &row :
                     m_outlinerTree->selectionModel()->selectedRows()) {
                    auto src = m_outlinerProxy->mapToSource(row);
                    ids.append(m_outlinerModel->item(src.row(), 0)
                                   ->data(Qt::UserRole)
                                   .toString());
                }
                auto current = m_outlinerProxy->mapToSource(
                    m_outlinerTree->currentIndex());
                QString active;
                if (current.isValid())
                    active = m_outlinerModel->item(current.row(), 0)
                                 ->data(Qt::UserRole)
                                 .toString();
                petuniaRequest("scene_selection",
                               {{"ids", ids}, {"active", active}});
                syncFromApplication();
            });
    connect(m_outlinerModel, &QStandardItemModel::itemChanged, this,
            [this](QStandardItem *item) {
                if (m_syncingOutliner || item->column() != 0)
                    return;
                petuniaRequest("rename",
                               {{"id", item->data(Qt::UserRole).toString()},
                                {"name", item->text()}});
                syncFromApplication();
            });
    m_outlinerDock->setWidget(container);
    addDockWidget(Qt::RightDockWidgetArea, m_outlinerDock);
    m_dockManager->registerExistingDock(QStringLiteral("OutlinerDock"), m_outlinerDock);

    auto panelDock = [this](const QString &title, const QString &name,
                            const QString &mode, Qt::DockWidgetArea area) {
        auto *panel = new PetuniaEditorPanel(mode, this);
        auto *dock = m_dockManager->registerDock(name, title, panel, area);
        m_editorPanels.append(panel);
        connect(panel, &PetuniaEditorPanel::editorChanged, this,
                &PetuniaMainWindow::syncFromApplication);
        return dock;
    };
    m_inspectorDock = panelDock(tr("Inspector"), "InspectorDock", "MODEL",
                                Qt::RightDockWidgetArea);
    m_paintDock = panelDock(tr("Pintura"), "PaintDock", "PAINT",
                            Qt::BottomDockWidgetArea);
    m_uvDock = panelDock(tr("UV"), "UvDock", "UV", Qt::BottomDockWidgetArea);
    m_timelineDock = panelDock(tr("Animação"), "TimelineDock", "ANIMATE",
                               Qt::BottomDockWidgetArea);
    m_paintDock->hide();
    m_uvDock->hide();
    m_timelineDock->hide();
    m_outlinerDock->setMinimumWidth(300);
    m_inspectorDock->setMinimumWidth(300);
    resizeDocks({m_outlinerDock, m_inspectorDock}, {240, 450}, Qt::Vertical);
    resizeDocks({m_outlinerDock, m_inspectorDock}, {320, 320}, Qt::Horizontal);
}

void PetuniaMainWindow::setupStatusBar() {
    auto *sb = statusBar();
    m_modeBadge = new QLabel(QStringLiteral(" OBJECT "), this);
    m_modeBadge->setStyleSheet(QStringLiteral(
        "background-color: #b58cff; color: #121212; font-weight: bold; "
        "border-radius: 3px; padding: 2px 6px;"));
    sb->addWidget(m_modeBadge);

    m_statusLabel = new QLabel(QStringLiteral("Pronto"), this);
    sb->addWidget(m_statusLabel, 1);

    m_statsLabel =
        new QLabel(QStringLiteral("Vértices: 0 | Triângulos: 0"), this);
    m_statsLabel->setStyleSheet(QStringLiteral(
        "color: #888888; font-family: monospace; margin-right: 12px;"));
    sb->addPermanentWidget(m_statsLabel);

    m_selectionLabel =
        new QLabel(QStringLiteral("Nenhum objeto selecionado"), this);
    sb->addPermanentWidget(m_selectionLabel);
}

void PetuniaMainWindow::setSelectionDomain(int domain) {
    m_activeDomain = domain;
    petunia_qt_app_set_selection_domain(domain);

    switch (domain) {
    case 1:
        m_modeBadge->setText(QStringLiteral(" VERTEX "));
        m_modeBadge->setStyleSheet(QStringLiteral(
            "background-color: #4fc3f7; color: #121212; font-weight: bold; "
            "border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(
            QStringLiteral("Modo de Seleção: Vértice (Nível Sub-Objeto)"));
        break;
    case 2:
        m_modeBadge->setText(QStringLiteral(" EDGE "));
        m_modeBadge->setStyleSheet(QStringLiteral(
            "background-color: #81c784; color: #121212; font-weight: bold; "
            "border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Aresta"));
        break;
    case 3:
        m_modeBadge->setText(QStringLiteral(" FACE "));
        m_modeBadge->setStyleSheet(QStringLiteral(
            "background-color: #ffb74d; color: #121212; font-weight: bold; "
            "border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Face"));
        break;
    default:
        m_modeBadge->setText(QStringLiteral(" OBJECT "));
        m_modeBadge->setStyleSheet(QStringLiteral(
            "background-color: #b58cff; color: #121212; font-weight: bold; "
            "border-radius: 3px; padding: 2px 6px;"));
        setStatusMessage(QStringLiteral("Modo de Seleção: Objeto B-Rep"));
        break;
    }

    if (m_viewport) {
        m_viewport->update();
    }
}

void PetuniaMainWindow::pickMaterialColor() {
    auto selection = m_outlinerTree->selectionModel()->selectedRows();
    int row = selection.isEmpty()
                  ? 0
                  : m_outlinerProxy->mapToSource(selection.first()).row();

    float r = 0.8f, g = 0.8f, b = 0.8f;
    petunia_qt_app_get_asset_color(row, &r, &g, &b);
    QColor initialColor = QColor::fromRgbF(r, g, b);

    PetuniaColorPickerPopover picker(this);
    picker.setTitle(QStringLiteral("Cor Base do Material"));
    picker.setColor(initialColor);
    if (picker.exec() == QDialog::Accepted) {
        const QColor chosen = picker.color();
        petunia_qt_app_set_asset_color(row, static_cast<float>(chosen.redF()),
                                       static_cast<float>(chosen.greenF()),
                                       static_cast<float>(chosen.blueF()));
        updateDirtyState(petunia_qt_app_is_dirty());
        syncFromApplication();
    }
}

void PetuniaMainWindow::onOutlinerContextMenu(const QPoint &pos) {
    const auto proxy = m_outlinerTree->indexAt(pos);
    if (!proxy.isValid())
        return;
    const auto src = m_outlinerProxy->mapToSource(proxy);
    const auto id =
        m_outlinerModel->item(src.row(), 0)->data(Qt::UserRole).toString();
    petuniaRequest("select", {{"id", id}});
    QMenu menu(this);
    auto *rename = menu.addAction(tr("Renomear (F2)"));
    auto *duplicate = menu.addAction(tr("Duplicar (Ctrl+D)"));
    auto *remove = menu.addAction(tr("Excluir (Del)"));
    menu.addSeparator();
    auto *toggleVis = menu.addAction(tr("Alternar visibilidade (H)"));
    auto *toggleLock = menu.addAction(tr("Bloquear / Desbloquear (L)"));
    auto *isolate = menu.addAction(tr("Isolar (Shift+H)"));
    menu.addSeparator();
    auto *save = menu.addAction(tr("Salvar como asset"));
    auto *selectionColor = menu.addAction(tr("Cor de seleção da parte"));
    auto *defaultColor = menu.addAction(tr("Usar cor de seleção global"));
    menu.addSeparator();
    auto *up = menu.addAction(tr("Mover acima"));
    auto *down = menu.addAction(tr("Mover abaixo"));
    menu.addSeparator();
    auto *alignSub = menu.addMenu(tr("Alinhar"));
    auto *aCenterX = alignSub->addAction(tr("Centralizar em X"));
    auto *aCenterY = alignSub->addAction(tr("Centralizar em Y"));
    auto *aCenterZ = alignSub->addAction(tr("Centralizar em Z"));
    alignSub->addSeparator();
    auto *aMinX = alignSub->addAction(tr("Esquerda (Min X)"));
    auto *aMaxX = alignSub->addAction(tr("Direita (Max X)"));
    auto *aMinY = alignSub->addAction(tr("Base (Min Y)"));
    auto *aMaxY = alignSub->addAction(tr("Topo (Max Y)"));
    auto *aMinZ = alignSub->addAction(tr("Fundo (Min Z)"));
    auto *aMaxZ = alignSub->addAction(tr("Frente (Max Z)"));
    alignSub->addSeparator();
    auto *aDistX = alignSub->addAction(tr("Distribuir em X"));
    auto *aDistY = alignSub->addAction(tr("Distribuir em Y"));
    auto *aDistZ = alignSub->addAction(tr("Distribuir em Z"));

    auto *action = menu.exec(m_outlinerTree->viewport()->mapToGlobal(pos));
    if (action == rename)
        m_outlinerTree->edit(proxy);
    else if (action == duplicate)
        petuniaRequest("scene_context", {{"id", id}, {"target", "duplicate"}});
    else if (action == remove)
        petuniaRequest("scene_context", {{"id", id}, {"target", "delete"}});
    else if (action == toggleVis)
        petuniaRequest("visibility", {{"id", id}});
    else if (action == toggleLock)
        petuniaRequest("lock", {{"id", id}});
    else if (action == isolate)
        petuniaRequest("isolate");
    else if (action == save)
        petuniaRequest("save_asset");
    else if (action == selectionColor) {
        PetuniaColorPickerPopover picker(this);
        picker.setTitle(QStringLiteral("Cor de Seleção"));
        picker.setColor(QColor(QStringLiteral("#e96a00")));
        if (picker.exec() == QDialog::Accepted) {
            const auto c = picker.color();
            petuniaRequest(
                "part_selection_color",
                {{"id", id},
                 {"color", QJsonArray{c.red(), c.green(), c.blue()}}});
        }
    } else if (action == defaultColor)
        petuniaRequest("part_selection_color",
                       {{"id", id}, {"color", QJsonValue()}});
    else if (action == up || action == down)
        petuniaRequest("scene_move",
                       {{"id", id}, {"delta", action == up ? -1 : 1}});
    else if (action == aCenterX)
        emit actionTriggered("model.align_center_x");
    else if (action == aCenterY)
        emit actionTriggered("model.align_center_y");
    else if (action == aCenterZ)
        emit actionTriggered("model.align_center_z");
    else if (action == aMinX)
        emit actionTriggered("model.align_min_x");
    else if (action == aMaxX)
        emit actionTriggered("model.align_max_x");
    else if (action == aMinY)
        emit actionTriggered("model.align_min_y");
    else if (action == aMaxY)
        emit actionTriggered("model.align_max_y");
    else if (action == aMinZ)
        emit actionTriggered("model.align_min_z");
    else if (action == aMaxZ)
        emit actionTriggered("model.align_max_z");
    else if (action == aDistX)
        emit actionTriggered("model.distribute_objects_x");
    else if (action == aDistY)
        emit actionTriggered("model.distribute_objects_y");
    else if (action == aDistZ)
        emit actionTriggered("model.distribute_objects_z");
    syncFromApplication();
}

void PetuniaMainWindow::syncFromApplication() {
    syncEditorState(petuniaRequest("snapshot"));
}

void PetuniaMainWindow::refreshModifiers() {
    if (!m_modifiersListLayout)
        return;

    QLayoutItem *item;
    while ((item = m_modifiersListLayout->takeAt(0)) != nullptr) {
        if (item->widget())
            delete item->widget();
        delete item;
    }

    auto selection = m_outlinerTree->selectionModel()->selectedRows();
    int row = selection.isEmpty()
                  ? 0
                  : m_outlinerProxy->mapToSource(selection.first()).row();
    int modCount = petunia_qt_app_get_modifier_count(row);

    if (modCount == 0) {
        auto *emptyLabel = new QLabel(
            QStringLiteral("Nenhum modificador ativo"), m_modifiersContainer);
        emptyLabel->setStyleSheet(QStringLiteral(
            "color: #777777; font-style: italic; font-size: 11px;"));
        m_modifiersListLayout->addWidget(emptyLabel);
        return;
    }

    for (int i = 0; i < modCount; ++i) {
        char nameBuf[64] = {0};
        bool enabled = true;
        int axis = 0;
        float weld = 0.001f;
        if (!petunia_qt_app_get_modifier_info(row, i, nameBuf, sizeof(nameBuf),
                                              &enabled, &axis, &weld)) {
            continue;
        }

        auto *card = new QFrame(m_modifiersContainer);
        card->setStyleSheet(QStringLiteral(
            "QFrame { background-color: #242424; border: 1px solid #3c3c3c; "
            "border-radius: 4px; padding: 4px; }"));
        auto *cardLayout = new QVBoxLayout(card);
        cardLayout->setContentsMargins(6, 6, 6, 6);
        cardLayout->setSpacing(4);

        auto *headerRow = new QHBoxLayout();
        auto *enableCheck = new QCheckBox(QString::fromUtf8(nameBuf), card);
        enableCheck->setChecked(enabled);
        enableCheck->setStyleSheet(
            QStringLiteral("font-weight: bold; color: #b58cff;"));
        connect(enableCheck, &QCheckBox::toggled, this,
                [this, row, i](bool checked) {
                    petunia_qt_app_toggle_modifier(row, i, checked);
                    if (m_viewport)
                        m_viewport->update();
                });
        headerRow->addWidget(enableCheck, 1);

        auto *applyBtn = new QPushButton(QStringLiteral("Aplicar"), card);
        applyBtn->setCursor(Qt::PointingHandCursor);
        connect(applyBtn, &QPushButton::clicked, this, [this, row, i]() {
            petunia_qt_app_apply_modifier(row, i);
            setStatusMessage(
                QStringLiteral("Modificador aplicado à malha base"));
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
        auto *weldSpin = new PetuniaNumberField(card);
        weldSpin->setRange(0.0001, 1.0);
        weldSpin->setSingleStep(0.001);
        weldSpin->setValue(static_cast<double>(weld));
        weldSpin->setSuffix(QStringLiteral("m"));
        optsRow->addWidget(weldLabel);
        optsRow->addWidget(weldSpin);

        cardLayout->addLayout(optsRow);
        m_modifiersListLayout->addWidget(card);
    }
}

void PetuniaMainWindow::refreshMaterialPbr() {
    if (!m_roughnessSlider || !m_metallicSlider || !m_emissionSpin ||
        !m_profileCombo)
        return;
    m_syncingMaterial = true;

    auto selection = m_outlinerTree->selectionModel()->selectedRows();
    int row = selection.isEmpty()
                  ? 0
                  : m_outlinerProxy->mapToSource(selection.first()).row();

    float rough = 0.5f, metal = 0.0f, emission = 0.0f;
    int profile = 0;
    if (petunia_qt_app_get_material_pbr(row, &rough, &metal, &emission,
                                        &profile)) {
        m_roughnessSlider->setValue(static_cast<int>(rough * 100.0f));
        m_roughnessLabel->setText(QString::number(rough, 'f', 2));
        m_metallicSlider->setValue(static_cast<int>(metal * 100.0f));
        m_metallicLabel->setText(QString::number(metal, 'f', 2));
        m_emissionSpin->setValue(static_cast<double>(emission));
        int profIdx = m_profileCombo->findData(profile);
        if (profIdx >= 0)
            m_profileCombo->setCurrentIndex(profIdx);
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
        connect(m_commandPalette, &PetuniaCommandPalette::commandSelected, this,
                [this](const QString &cmd) {
                    emit actionTriggered(cmd);
                    syncFromApplication();
                });
    }
    QPoint center =
        mapToGlobal(rect().center()) -
        QPoint(m_commandPalette->width() / 2, m_commandPalette->height() / 2);
    m_commandPalette->move(center);
    m_commandPalette->show();
}

void PetuniaMainWindow::setWorkspace(const QString &workspace) {
    if (workspace == m_activeWorkspace && m_editorState["workspace"].isString())
        return;
    QSettings settings("Petunia3D", "QtStudio");
    settings.setValue("workspace/" + m_activeWorkspace, m_dockManager ? m_dockManager->saveState() : saveState());
    petuniaRequest("workspace", {{"id", workspace}});
    if (workspace == "DRAW" || workspace == "POLY")
        petuniaRequest("modeling_mode", {{"id", workspace}});
    m_activeWorkspace = workspace;
    const auto saved = settings.value("workspace/" + workspace).toByteArray();
    if (!saved.isEmpty()) {
        if (m_dockManager)
            m_dockManager->restoreState(saved);
        else
            restoreState(saved);
    } else if (m_dockManager) {
        m_dockManager->applyWorkspacePreset(workspace);
    } else {
        m_paintDock->setVisible(workspace == "PAINT");
        m_uvDock->setVisible(workspace == "UV");
        m_timelineDock->setVisible(workspace == "ANIMATE");
        m_inspectorDock->setVisible(workspace == "DRAW" || workspace == "POLY");
    }
    if (saved.isEmpty()) {
        auto *dock = workspace == "PAINT"     ? m_paintDock
                     : workspace == "UV"      ? m_uvDock
                     : workspace == "ANIMATE" ? m_timelineDock
                                              : nullptr;
        if (dock) {
            int targetH = (workspace == "UV") ? qMax(300, int(height() * 0.58))
                                              : qMax(190, height() / 3);
            resizeDocks({dock}, {targetH}, Qt::Vertical);
        }
    }
    syncFromApplication();
}

void PetuniaMainWindow::updateSelectedCount(int count) {
    int domain = petunia_qt_app_get_selection_domain();
    if (count <= 0) {
        switch (domain) {
        case 1:
            m_selectionLabel->setText(
                QStringLiteral("Nenhum vértice selecionado"));
            break;
        case 2:
            m_selectionLabel->setText(
                QStringLiteral("Nenhuma aresta selecionada"));
            break;
        case 3:
            m_selectionLabel->setText(
                QStringLiteral("Nenhuma face selecionada"));
            break;
        default:
            m_selectionLabel->setText(
                QStringLiteral("Nenhum objeto selecionado"));
            break;
        }
    } else {
        switch (domain) {
        case 1:
            m_selectionLabel->setText(
                QStringLiteral("%1 vértice(s) selecionado(s)").arg(count));
            break;
        case 2:
            m_selectionLabel->setText(
                QStringLiteral("%1 aresta(s) selecionada(s)").arg(count));
            break;
        case 3:
            m_selectionLabel->setText(
                QStringLiteral("%1 face(s) selecionada(s)").arg(count));
            break;
        default:
            m_selectionLabel->setText(
                QStringLiteral("%1 objeto(s) selecionado(s)").arg(count));
            break;
        }
    }
}

void PetuniaMainWindow::updateDirtyState(bool isDirty) {
    setWindowModified(isDirty);
}

void PetuniaMainWindow::setStatusMessage(const QString &message) {
    m_statusLabel->setText(message);
}

void PetuniaMainWindow::populateOutliner(const QStringList &objectNames,
                                         const QList<bool> &visibilities) {
    Q_UNUSED(objectNames);
    Q_UNUSED(visibilities);
    if (!m_outlinerModel)
        return;
    m_syncingOutliner = true;
    const auto items = m_editorState["scene_items"].toArray();
    const bool isEmpty = items.isEmpty();
    if (m_outlinerPlaceholder) {
        m_outlinerPlaceholder->setVisible(isEmpty);
    }
    m_outlinerTree->setVisible(!isEmpty);

    bool rebuild = m_outlinerModel->rowCount() != items.size();
    for (int i = 0; !rebuild && i < items.size(); ++i)
        rebuild = m_outlinerModel->item(i, 0)->data(Qt::UserRole).toString() !=
                  items[i].toObject()["id"].toString();
    if (rebuild) {
        m_outlinerModel->removeRows(0, m_outlinerModel->rowCount());
        for (const auto &v : items) {
            auto o = v.toObject();
            auto *name = new QStandardItem(o["name"].toString());
            name->setData(o["id"].toString(), Qt::UserRole);
            auto *eye = new QStandardItem;
            eye->setEditable(false);
            auto *lock = new QStandardItem;
            lock->setEditable(false);
            m_outlinerModel->appendRow({name, eye, lock});
        }
    }
    const QSignalBlocker block(m_outlinerTree->selectionModel());
    m_outlinerTree->selectionModel()->clearSelection();
    for (int i = 0; i < items.size(); ++i) {
        auto o = items[i].toObject();
        auto *name = m_outlinerModel->item(i, 0);
        name->setText(o["name"].toString());
        name->setData(o["selected"].toBool(), Qt::UserRole + 1);
        name->setEditable(!o["locked"].toBool());
        name->setToolTip(o["active"].toBool() ? tr("Objeto ativo")
                                              : tr("Objeto da cena"));

        QString iconColor = QStringLiteral("#b58cff");
        if (o.contains("selection_color") && o["selection_color"].isArray()) {
            auto arr = o["selection_color"].toArray();
            iconColor = QColor(arr[0].toInt(), arr[1].toInt(), arr[2].toInt()).name();
        }
        name->setIcon(PetuniaIcons::get(QStringLiteral("cube"), iconColor));

        auto *eye = m_outlinerModel->item(i, 1);
        if (o["visible"].toBool()) {
            eye->setIcon(PetuniaIcons::get("eye", "#cccccc"));
            eye->setText(QString());
        } else {
            eye->setIcon(PetuniaIcons::get("eye_hidden", "#555555"));
            eye->setText(QString());
        }
        eye->setTextAlignment(Qt::AlignCenter);
        eye->setToolTip(o["visible"].toBool() ? tr("Visível") : tr("Oculto (clique para exibir)"));

        auto *lock = m_outlinerModel->item(i, 2);
        if (o["locked"].toBool()) {
            lock->setIcon(PetuniaIcons::get("lock", "#cccccc"));
            lock->setText(QString());
        } else {
            lock->setIcon(PetuniaIcons::get("unlock", "#444444"));
            lock->setText(QString());
        }
        lock->setTextAlignment(Qt::AlignCenter);
        lock->setToolTip(o["locked"].toBool() ? tr("Bloqueado") : tr("Livre (clique para bloquear)"));

        auto proxy =
            m_outlinerProxy->mapFromSource(m_outlinerModel->index(i, 0));
        if (o["selected"].toBool())
            m_outlinerTree->selectionModel()->select(
                proxy, QItemSelectionModel::Select | QItemSelectionModel::Rows);
        if (o["active"].toBool())
            m_outlinerTree->selectionModel()->setCurrentIndex(
                proxy, QItemSelectionModel::NoUpdate);
    }
    m_syncingOutliner = false;
}

void PetuniaMainWindow::openPreferences() {
    const QPointer<QWidget> invokingFocus = qApp->focusWidget();
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Preferências"));
    dialog.resize(650, 650);
    dialog.setMinimumSize(320, 320);
    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new PetuniaEditorPanel("SETTINGS", &dialog);
    layout->addWidget(panel);
    auto *close = new QPushButton(tr("Fechar"), &dialog);
    layout->addWidget(close);
    connect(close, &QPushButton::clicked, &dialog, &QDialog::accept);
    connect(panel, &PetuniaEditorPanel::editorChanged, this,
            &PetuniaMainWindow::syncFromApplication);
    petuniaLocalize(&dialog);
    dialog.exec();
    if (invokingFocus && invokingFocus->isVisible())
        invokingFocus->setFocus();
    petuniaRequest("preferences_save");
    syncFromApplication();
}

void PetuniaMainWindow::openReferenceManager() {
    if (!m_referenceDialog) {
        m_referenceDialog = new QDialog(this, Qt::Window | Qt::CustomizeWindowHint |
                                              Qt::WindowTitleHint | Qt::WindowCloseButtonHint |
                                              Qt::WindowMaximizeButtonHint);
        m_referenceDialog->setWindowTitle(tr("Referências"));
        m_referenceDialog->resize(600, 560);
        m_referenceDialog->setMinimumSize(320, 320);
        auto *layout = new QVBoxLayout(m_referenceDialog);
        auto *panel = new PetuniaEditorPanel("REFERENCES", m_referenceDialog);
        layout->addWidget(panel);
        connect(panel, &PetuniaEditorPanel::editorChanged, this,
                &PetuniaMainWindow::syncFromApplication);
        petuniaLocalize(m_referenceDialog);
    }
    m_referenceDialog->show();
    m_referenceDialog->raise();
    m_referenceDialog->activateWindow();
}

void PetuniaMainWindow::openHomeDialog() {
    const QPointer<QWidget> invokingFocus = qApp->focusWidget();
    PetuniaHomeDialog dialog(this);
    QStringList names, paths;
    for (const auto &v : m_editorState["recent_projects"].toArray()) {
        auto pair = v.toArray();
        names << pair[0].toString();
        paths << pair[1].toString();
    }
    dialog.setRecentProjects(names, paths);
    dialog.setRecoveryAvailable(m_editorState["recovery_available"].toBool());
    dialog.setShowOnStart(m_editorState["show_home_on_start"].toBool());
    connect(&dialog, &PetuniaHomeDialog::newProjectRequested, this,
            &PetuniaMainWindow::newProject);
    connect(&dialog, &PetuniaHomeDialog::openProjectRequested, this,
            &PetuniaMainWindow::openProject);
    connect(&dialog, &PetuniaHomeDialog::recoverProjectRequested, this,
            &PetuniaMainWindow::openRecoveryDialog);
    connect(&dialog, &PetuniaHomeDialog::settingsRequested, this,
            &PetuniaMainWindow::openPreferences);
    connect(&dialog, &PetuniaHomeDialog::recentProjectSelected, this,
            [this](const QString &path) {
                if (confirmDiscard()) {
                    petuniaRequest("project.open", {{"path", path}});
                    syncFromApplication();
                }
            });
    petuniaLocalize(&dialog);
    dialog.exec();
    if (invokingFocus && invokingFocus->isVisible())
        invokingFocus->setFocus();
    petuniaRequest("set_show_home_on_start",
                   {{"enabled", dialog.showOnStart()}});
    petuniaRequest("preferences_save");
}

void PetuniaMainWindow::openRecoveryDialog() {
    const QPointer<QWidget> invokingFocus = qApp->focusWidget();
    if (!m_editorState["recovery_available"].toBool()) {
        QMessageBox::information(
            this, tr("Recuperação"),
            tr("Nenhum snapshot de recuperação disponível."));
        return;
    }
    PetuniaRecoveryDialog dialog(this);
    auto info = m_editorState["recovery"].toObject();
    dialog.setSessionDetails(tr("Sessão recuperável"), info["name"].toString(),
                             info["path"].toString());
    connect(&dialog, &PetuniaRecoveryDialog::recoverRequested, this, [this] {
        if (confirmDiscard())
            petuniaRequest("recover_pending");
        syncFromApplication();
    });
    connect(&dialog, &PetuniaRecoveryDialog::discardRequested, this, [this] {
        petuniaRequest("discard_pending_recovery");
        syncFromApplication();
    });
    connect(&dialog, &PetuniaRecoveryDialog::keepRequested, this, [this] {
        petuniaRequest("keep_saved_project");
        syncFromApplication();
    });
    petuniaLocalize(&dialog);
    dialog.exec();
    if (invokingFocus && invokingFocus->isVisible())
        invokingFocus->setFocus();
}

void PetuniaMainWindow::openAddMenu() {
    if (m_pieMenu) {
        m_pieMenu->execAt(QCursor::pos());
    } else if (m_addMenu) {
        m_addMenu->exec(QCursor::pos());
    }
}

void PetuniaMainWindow::toggleSplitView() {
    if (m_secondaryViewportContainer) {
        bool show = !m_secondaryViewportContainer->isVisible();
        m_secondaryViewportContainer->setVisible(show);
        setStatusMessage(
            show ? QStringLiteral("Vista 3D dividida ativada")
                 : QStringLiteral("Vista 3D individual restaurada"));
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

void PetuniaMainWindow::showOperationHud(const QString &opName,
                                         const QString &details) {
    if (m_operationHud) {
        m_operationHud->showFeedback(opName, details);
    }
}

void PetuniaMainWindow::setPrimaryCameraPreset(const QString &preset) {
    PetuniaViewScope scope(1);
    setStatusMessage(QStringLiteral("Câmera / Projeção: %1").arg(preset));
    if (preset == QStringLiteral("front"))
        petunia_qt_camera_set_preset(1);
    else if (preset == QStringLiteral("back"))
        petunia_qt_camera_set_preset(2);
    else if (preset == QStringLiteral("right"))
        petunia_qt_camera_set_preset(3);
    else if (preset == QStringLiteral("left"))
        petunia_qt_camera_set_preset(4);
    else if (preset == QStringLiteral("top"))
        petunia_qt_camera_set_preset(5);
    else if (preset == QStringLiteral("iso21")) {
        petunia_qt_camera_set_preset(8);
        if (!m_editorState["is_orthographic"].toBool())
            petuniaRequest("command", {{"id", "view.toggle_projection"}});
    } else if (preset == QStringLiteral("ortho")) {
        if (!m_editorState["is_orthographic"].toBool()) {
            petunia_qt_camera_toggle_projection();
            petuniaRequest("command", {{"id", "view.toggle_projection"}});
        }
    } else { // persp
        if (m_editorState["is_orthographic"].toBool()) {
            petunia_qt_camera_toggle_projection();
            petuniaRequest("command", {{"id", "view.toggle_projection"}});
        }
        petunia_qt_camera_set_preset(0);
    }

    if (m_viewport) {
        m_viewport->update();
    }
    syncFromApplication();
}

void PetuniaMainWindow::setSecondaryCameraPreset(const QString &preset) {
    PetuniaViewScope scope(2);
    setStatusMessage(QStringLiteral("Câmera / Projeção: %1").arg(preset));
    if (preset == QStringLiteral("front"))
        petunia_qt_camera_set_preset(1);
    else if (preset == QStringLiteral("back"))
        petunia_qt_camera_set_preset(2);
    else if (preset == QStringLiteral("right"))
        petunia_qt_camera_set_preset(3);
    else if (preset == QStringLiteral("left"))
        petunia_qt_camera_set_preset(4);
    else if (preset == QStringLiteral("top"))
        petunia_qt_camera_set_preset(5);
    else if (preset == QStringLiteral("iso21"))
        petunia_qt_camera_set_preset(8);
    else if (preset == QStringLiteral("ortho")) {
        if (!m_editorState["is_orthographic"].toBool())
            petuniaRequest("command", {{"id", "view.toggle_projection"}});
    } else { // persp
        if (m_editorState["is_orthographic"].toBool())
            petuniaRequest("command", {{"id", "view.toggle_projection"}});
        petunia_qt_camera_set_preset(0);
    }

    if (m_viewport) {
        m_viewport->update();
    }
    if (m_secondaryViewport) {
        m_secondaryViewport->update();
    }
    syncFromApplication();
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
    applyPreferences(m_editorState);
}

void PetuniaMainWindow::newProject() {
    if (!confirmDiscard())
        return;
    petuniaRequest("project.new");
    m_currentProjectPath.clear();
    syncFromApplication();
}

void PetuniaMainWindow::openProject() {
    if (!confirmDiscard())
        return;
    QString path = QFileDialog::getOpenFileName(
        this, QStringLiteral("Abrir Projeto Petunia3D"), QString(),
        QStringLiteral(
            "Projetos Petunia (*.petunia *.json);;Todos os Arquivos (*)"));
    if (!path.isEmpty()) {
        if (petunia_qt_app_load_project(path.toUtf8().constData())) {
            m_currentProjectPath = path;
            setStatusMessage(QStringLiteral("Projeto carregado: %1").arg(path));
            syncFromApplication();
        } else {
            QMessageBox::warning(
                this, QStringLiteral("Erro"),
                QStringLiteral("Falha ao abrir o arquivo de projeto."));
        }
    }
}

void PetuniaMainWindow::saveProject() {
    if (m_currentProjectPath.isEmpty()) {
        saveProjectAs();
        return;
    }
    if (petunia_qt_app_save_project(
            m_currentProjectPath.toUtf8().constData())) {
        setStatusMessage(QStringLiteral("Projeto salvo com sucesso: %1")
                             .arg(m_currentProjectPath));
        updateDirtyState(false);
    } else {
        QMessageBox::warning(
            this, QStringLiteral("Erro"),
            QStringLiteral("Falha ao salvar o projeto no disco."));
    }
}

void PetuniaMainWindow::saveProjectAs() {
    QString path = QFileDialog::getSaveFileName(
        this, QStringLiteral("Salvar Projeto Petunia3D"),
        m_currentProjectPath.isEmpty() ? QStringLiteral("projeto.petunia")
                                       : m_currentProjectPath,
        QStringLiteral("Projetos Petunia (*.petunia);;Todos os Arquivos (*)"));
    if (!path.isEmpty()) {
        m_currentProjectPath = path;
        saveProject();
    }
}

void PetuniaMainWindow::exportGltf() {
    QString path = QFileDialog::getSaveFileName(
        this, QStringLiteral("Exportar Modelo 3D (GLTF/GLB)"),
        QStringLiteral("modelo.glb"), QStringLiteral("Binary GLTF (*.glb)"));
    if (!path.isEmpty()) {
        if (petunia_qt_app_export_gltf(path.toUtf8().constData())) {
            setStatusMessage(
                QStringLiteral("Modelo GLTF exportado com sucesso: %1")
                    .arg(path));
        } else {
            QMessageBox::warning(
                this, QStringLiteral("Erro"),
                QStringLiteral("Falha ao exportar modelo GLTF."));
        }
    }
}

void PetuniaMainWindow::exportObj() {
    QString path = QFileDialog::getSaveFileName(
        this, QStringLiteral("Exportar Wavefront OBJ"),
        QStringLiteral("modelo.obj"),
        QStringLiteral("Wavefront OBJ (*.obj);;Todos os Arquivos (*)"));
    if (!path.isEmpty()) {
        if (petunia_qt_app_export_obj(path.toUtf8().constData())) {
            setStatusMessage(
                QStringLiteral("Modelo OBJ exportado com sucesso: %1")
                    .arg(path));
        } else {
            QMessageBox::warning(
                this, QStringLiteral("Erro"),
                QStringLiteral("Falha ao exportar modelo OBJ."));
        }
    }
}

void PetuniaMainWindow::openAssetLibrary() {
    const QPointer<QWidget> invokingFocus = qApp->focusWidget();
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Biblioteca do projeto"));
    dialog.resize(650, 650);
    dialog.setMinimumSize(320, 320);
    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new PetuniaEditorPanel("ASSETS", &dialog);
    layout->addWidget(panel);
    auto *close = new QPushButton(tr("Fechar"), &dialog);
    layout->addWidget(close);
    connect(close, &QPushButton::clicked, &dialog, &QDialog::accept);
    connect(panel, &PetuniaEditorPanel::editorChanged, this,
            &PetuniaMainWindow::syncFromApplication);
    petuniaLocalize(&dialog);
    dialog.exec();
    if (invokingFocus && invokingFocus->isVisible())
        invokingFocus->setFocus();
    petuniaRequest("preferences_save");
    syncFromApplication();
}

extern "C" {
static int s_argc = 1;
static char s_app_name[] = "petunia3d";
static char *s_argv[] = {s_app_name, nullptr};
bool petunia_qt_claim_ui_thread();

void *petunia_qt_app_create() {
    if (!petunia_qt_claim_ui_thread())
        return nullptr;
    QSurfaceFormat fmt;
    fmt.setVersion(3, 3);
    fmt.setProfile(QSurfaceFormat::CoreProfile);
    QSurfaceFormat::setDefaultFormat(fmt);

    return new QApplication(s_argc, s_argv);
}

int petunia_qt_app_exec(void *app) {
    if (app) {
        return static_cast<QApplication *>(app)->exec();
    }
    return 0;
}

void petunia_qt_app_destroy(void *app) {
    delete static_cast<QApplication *>(app);
}

void *petunia_main_window_create() {
    if (!qApp || QThread::currentThread() != qApp->thread())
        return nullptr;
    return new PetuniaMainWindow();
}

void petunia_main_window_show(void *win) {
    if (win) {
        static_cast<PetuniaMainWindow *>(win)->show();
    }
}

void petunia_main_window_destroy(void *win) {
    delete static_cast<PetuniaMainWindow *>(win);
}
}

void PetuniaMainWindow::syncEditorState(const QJsonObject &state) {
    if (state.isEmpty())
        return;
    m_editorState = state;
    applyPreferences(state);
    m_currentProjectPath = state["project_path"].toString();
    m_statusLabel->setProperty("snapshotDerived", true);
    m_selectionLabel->setProperty("snapshotDerived", true);
    m_statsLabel->setProperty("snapshotDerived", true);
    const auto priorStatus = m_statusLabel->text();
    setStatusMessage(
        state["error"].toString(state["status_message"].toString()));
    if (m_statusLabel->text() != priorStatus) {
        QAccessibleEvent announcement(m_statusLabel,
                                      state["status_severity"].toInt() == 3
                                          ? QAccessible::Alert
                                          : QAccessible::NameChanged);
        QAccessible::updateAccessibility(&announcement);
    }
    m_selectionLabel->setText(state["selection_summary"].toString());
    m_statsLabel->setText(state["scene_stats"].toString());
    updateDirtyState(!state["saved"].toBool());
    populateOutliner({}, {});
    for (auto *panel : m_editorPanels)
        if (panel->isVisible())
            panel->sync(state);
    const auto mode = state["workspace"].toString() == "Model"
                          ? state["modeling_mode"].toString()
                          : state["workspace"].toString().toUpper();
    m_activeWorkspace = mode;
    updateToolRail(mode, state);
    const bool paintPip = mode == "PAINT" && state["paint_pip"].toBool();
    m_primaryStack->setCurrentWidget(
        paintPip ? static_cast<QWidget *>(m_primaryPaintCanvas)
                 : static_cast<QWidget *>(m_viewport));
    m_pipViewport->setVisible(paintPip);
    if (paintPip) {
        m_primaryPaintCanvas->sync(state);
        m_pipViewport->update();
    }
    for (auto *button : findChildren<QPushButton *>())
        if (QStringList{"DRAW", "POLY", "PAINT", "UV", "ANIMATE"}.contains(
                button->property("workspaceId").toString())) {
            QSignalBlocker block(button);
            button->setChecked(button->property("workspaceId").toString() ==
                               mode);
        }
    if (m_domainObjBtn) {
        const auto domain = state["selection_domain"].toString();
        m_domainObjBtn->setChecked(domain == "Object");
        m_domainVertBtn->setChecked(domain == "Vertex");
        m_domainEdgeBtn->setChecked(domain == "Edge");
        m_domainFaceBtn->setChecked(domain == "Face");
    }
    m_toolOptionsBar->sync(state);
    if (m_contextBar)
        m_contextBar->sync(state);
    repositionFloatingBars();
    if (state["operation_hud_active"].toBool()) {
        QStringList lines;
        for (const auto &v : state["operation_hud_lines"].toArray())
            lines << v.toString();
        showOperationHud(state["operation_hud_title"].toString(),
                         lines.join(" · "));
    } else
        m_operationHud->clearFeedback();
    const auto position = state["position"].toArray(),
               rotation = state["rotation"].toArray();
    if (position.size() == 3 && rotation.size() == 3)
        m_microInspector->setCoordinates(
            position[0].toDouble(), position[1].toDouble(),
            position[2].toDouble(), rotation[0].toDouble(),
            rotation[1].toDouble(), rotation[2].toDouble());
    m_microInspector->setMeasureData(
        state["measure_distance"].toDouble(), state["measure_dx"].toDouble(),
        state["measure_dy"].toDouble(), state["measure_dz"].toDouble(),
        state["measure_angle_deg"].toDouble());
    m_primitiveCard->setVisible(state["primitive_active"].toBool());
    m_loopCutCard->setVisible(state["loop_cut_active"].toBool());
    for (auto *action : findChildren<QAction *>()) {
        if (action->property("editorAction") == "undo")
            action->setEnabled(state["can_undo"].toBool());
        if (action->property("editorAction") == "redo")
            action->setEnabled(state["can_redo"].toBool());
    }
    if (state["command_search_visible"].toBool() ||
        state["settings_visible"].toBool() ||
        state["reference_manager_open"].toBool() ||
        state["asset_library_visible"].toBool()) {
        petuniaRequest("consume_host_requests");
        if (state["command_search_visible"].toBool())
            openCommandPalette();
        else if (state["settings_visible"].toBool())
            openPreferences();
        else if (state["reference_manager_open"].toBool())
            openReferenceManager();
        else if (state["asset_library_visible"].toBool())
            openAssetLibrary();
    }
    applyPreferences(state);
    if (m_viewport)
        m_viewport->update();
    if (m_secondaryViewport && m_secondaryViewport->isVisible())
        m_secondaryViewport->update();
}

void PetuniaMainWindow::showEvent(QShowEvent *event) {
    QMainWindow::showEvent(event);
    if (m_outlinerDock && m_inspectorDock) {
        resizeDocks({m_outlinerDock, m_inspectorDock}, {310, 310}, Qt::Horizontal);
    }
    repositionFloatingBars();
}

void PetuniaMainWindow::repositionFloatingBars() {
    if (!m_primaryStack)
        return;
    const int w = m_primaryStack->width();
    const int h = m_primaryStack->height();
    if (w < 40 || h < 40)
        return;

    if (m_toolOptionsBar && m_toolOptionsBar->isVisible()) {
        m_toolOptionsBar->adjustSize();
        const int barW = m_toolOptionsBar->width();
        const int maxX = qMax(8, w - barW - 90);
        const int x = qBound(8, (w - barW) / 2, maxX);
        m_toolOptionsBar->move(x, 8);
        m_toolOptionsBar->raise();
    }
    if (m_contextBar && m_contextBar->isVisible()) {
        m_contextBar->adjustSize();
        const int barW = m_contextBar->width();
        const int barH = m_contextBar->height();
        const int x = qBound(8, (w - barW) / 2, qMax(8, w - barW - 8));
        const int y = qMax(8, h - barH - 12);
        m_contextBar->move(x, y);
        m_contextBar->raise();
    }
}

void PetuniaMainWindow::applyPreferences(const QJsonObject &state) {
    const auto colors = state["theme_colors"].toObject();
    PetuniaTheme::instance().updateFromSnapshot(colors, state["current_theme"].toString());
    PetuniaIcons::clearCache();
    const auto signature = state["current_theme"].toString() +
                           state["accent_color_hex"].toString() +
                           QString::number(state["ui_scale"].toDouble()) +
                           QString::number(state["reduced_motion"].toBool()) +
                           state["active_language"].toString();
    if (signature == m_themeSignature)
        return;
    m_themeSignature = signature;
    petuniaSetTranslations(
        petuniaRequest("translations")["translations"].toObject());
    const auto color = [&](const char *key, const char *fallback) {
        return QColor(
            colors.value(key).toString(QString::fromLatin1(fallback)));
    };
    QPalette palette;
    palette.setColor(QPalette::Window, color("bg_panel", "#202020"));
    palette.setColor(QPalette::WindowText, color("text_primary", "#eeeeee"));
    palette.setColor(QPalette::Base, color("bg_canvas", "#121212"));
    palette.setColor(QPalette::AlternateBase, color("bg_surface", "#292929"));
    palette.setColor(QPalette::Text, color("text_primary", "#eeeeee"));
    palette.setColor(QPalette::Button, color("bg_surface", "#292929"));
    palette.setColor(QPalette::ButtonText, color("text_primary", "#eeeeee"));
    palette.setColor(QPalette::Highlight,
                     QColor(state["accent_color_hex"].toString(
                         colors["accent_blue"].toString("#b58cff"))));
    palette.setColor(QPalette::HighlightedText,
                     color("text_active", "#121212"));
    palette.setColor(QPalette::Mid, color("border_subtle", "#404040"));
    palette.setColor(QPalette::ToolTipBase, color("bg_surface", "#292929"));
    palette.setColor(QPalette::ToolTipText, color("text_primary", "#eeeeee"));
    palette.setColor(QPalette::Disabled, QPalette::Text,
                     color("text_muted", "#777777"));
    palette.setColor(QPalette::Disabled, QPalette::ButtonText,
                     color("text_muted", "#777777"));
    qApp->setPalette(palette);

    const QString bgCanvas = color("bg_canvas", "#121212").name();
    const QString bgHeader = color("bg_header", "#202020").name();
    const QString bgPanel = color("bg_panel", "#202020").name();
    const QString bgSurface = color("bg_surface", "#292929").name();
    const QString bgSurfaceHover = color("bg_surface_hover", "#353535").name();
    const QString bgSurfaceActive = color("bg_surface_active", "#404040").name();
    const QString textPrimary = color("text_primary", "#eeeeee").name();
    const QString textSecondary = color("text_secondary", "#b5b5b5").name();
    const QString textMuted = color("text_muted", "#777777").name();
    const QString textActive = color("text_active", "#121212").name();
    const QString accentColor = palette.color(QPalette::Highlight).name();
    const QString accentOrange = color("accent_orange", "#e96a00").name();
    const QString borderSubtle = color("border_subtle", "#404040").name();
    const QString borderStrong = color("border_strong", "#5a5a5a").name();
    const QString borderFocus = color("border_focus", "#b58cff").name();

    setStyleSheet(QStringLiteral(
        "QMainWindow { background-color: %1; color: %2; }\n"
        "QMenuBar { background-color: %3; color: %2; border-bottom: 1px solid %4; padding: 2px 4px; font-weight: 500; }\n"
        "QMenuBar::item { background: transparent; padding: 4px 8px; border-radius: 4px; }\n"
        "QMenuBar::item:selected { background-color: %5; color: %2; }\n"
        "QMenuBar::item:pressed { background-color: %6; }\n"
        "QMenu { background-color: %7; color: %2; border: 1px solid %4; border-radius: 6px; padding: 4px; }\n"
        "QMenu::item { padding: 5px 24px 5px 12px; border-radius: 4px; font-size: 12px; }\n"
        "QMenu::item:selected { background-color: %8; color: %9; font-weight: bold; }\n"
        "QMenu::separator { height: 1px; background-color: %4; margin: 4px 6px; }\n"
        "QToolBar { background-color: %3; border-bottom: 1px solid %4; spacing: 4px; padding: 2px 6px; }\n"
        "QToolBar#WorkspaceToolBar { spacing: 6px; padding: 2px 8px; }\n"
        "QToolBar#WorkspaceToolBar QPushButton { background: transparent; color: %12; border: none; border-radius: 4px; padding: 3px 6px; min-height: 22px; }\n"
        "QToolBar#WorkspaceToolBar QPushButton:hover { background-color: %5; color: %2; }\n"
        "QToolBar#WorkspaceToolBar QPushButton:pressed { background-color: %6; }\n"
        "QPushButton#HeaderSearchBtn { background-color: %7; color: %12; border: 1px solid %4; border-radius: 13px; padding: 3px 12px; font-size: 11px; min-height: 22px; }\n"
        "QPushButton#HeaderSearchBtn:hover { border-color: %15; color: %2; background-color: %5; }\n"
        "QMenuBar#HeaderMenuBar { background: transparent; border: none; font-size: 12px; }\n"
        "QMenuBar#HeaderMenuBar::item { background: transparent; padding: 4px 8px; color: %12; border-radius: 4px; }\n"
        "QMenuBar#HeaderMenuBar::item:selected { background-color: %5; color: %2; }\n"
        "QMenuBar#HeaderMenuBar::item:pressed { background-color: %6; }\n"
        "QToolBar#ToolRailBar { background-color: %10; border-right: 1px solid %4; border-bottom: none; spacing: 3px; padding: 4px; min-width: 44px; max-width: 44px; }\n"
        "QToolBar#ToolRailBar[expanded=\"true\"] { min-width: 140px; max-width: 180px; }\n"
        "QToolBar#ToolRailBar QToolButton { background: transparent; color: %2; border: none; border-radius: 6px; font-weight: bold; min-width: 32px; max-width: 32px; min-height: 32px; max-height: 32px; padding: 2px; }\n"
        "QToolBar#ToolRailBar[expanded=\"true\"] QToolButton { min-width: 124px; max-width: 160px; min-height: 28px; max-height: 28px; text-align: left; padding: 4px 8px; font-weight: 500; font-size: 11px; }\n"
        "QToolBar#ToolRailBar QToolButton:hover { background-color: %5; }\n"
        "QToolBar#ToolRailBar QToolButton:checked { background-color: %8; color: %9; }\n"
        "QToolBar#ToolRailBar QToolButton:pressed { background-color: %11; }\n"
        "QFrame#WorkspacePillGroup { background-color: %7; border: 1px solid %4; border-radius: 14px; }\n"
        "QFrame#WorkspacePillGroup QPushButton { background: transparent; color: %12; border: none; border-radius: 12px; padding: 3px 14px; font-weight: bold; font-size: 12px; min-height: 22px; }\n"
        "QFrame#WorkspacePillGroup QPushButton:hover { color: %2; background-color: %5; }\n"
        "QFrame#WorkspacePillGroup QPushButton:checked { background-color: %8; color: %9; }\n"
        "QPushButton, QToolButton { background-color: %7; color: %2; border: 1px solid %4; border-radius: 4px; padding: 3px 8px; min-height: 20px; font-size: 12px; }\n"
        "QPushButton:hover, QToolButton:hover { background-color: %5; border-color: %13; }\n"
        "QPushButton:pressed, QToolButton:pressed { background-color: %6; }\n"
        "QPushButton:checked, QToolButton:checked { background-color: %8; color: %9; font-weight: bold; border-color: %8; }\n"
        "QPushButton:disabled, QToolButton:disabled { color: %14; background-color: %10; border-color: %4; }\n"
        "QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox { background-color: %7; color: %2; border: 1px solid %4; border-radius: 4px; padding: 2px 4px; min-height: 20px; font-size: 11px; selection-background-color: %8; selection-color: %9; }\n"
        "QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus { border: 1px solid %15; }\n"
        "QComboBox::drop-down { subcontrol-origin: padding; subcontrol-position: top right; width: 18px; border-left: 1px solid %4; }\n"
        "QComboBox QAbstractItemView { background-color: %7; color: %2; border: 1px solid %4; selection-background-color: %8; selection-color: %9; border-radius: 4px; }\n"
        "QDockWidget { color: %2; font-weight: bold; }\n"
        "QDockWidget::title { background-color: %10; text-align: left; padding: 5px 8px; border-bottom: 1px solid %4; font-size: 11px; }\n"
        "QTabWidget::pane { border: 1px solid %4; background-color: %10; }\n"
        "QTabBar::tab { background-color: transparent; color: %12; border: none; border-bottom: 2px solid transparent; padding: 4px 5px; min-width: 22px; min-height: 20px; font-weight: bold; font-size: 11px; margin-right: 1px; }\n"
        "QTabBar::tab:hover:!selected { background-color: %5; color: %2; border-radius: 4px; }\n"
        "QTabBar::tab:selected { background-color: %7; color: %8; border-bottom: 2px solid %8; border-radius: 4px; }\n"
        "QTreeView { background-color: %10; color: %2; border: 1px solid %4; border-radius: 4px; selection-background-color: transparent; outline: none; }\n"
        "QTreeView::item { height: 26px; padding: 2px 6px; border: none; font-size: 11px; }\n"
        "QTreeView::item:hover { background-color: %5; border-radius: 4px; }\n"
        "QTreeView::item:selected { background-color: rgba(181, 140, 255, 0.22); color: #ffffff; border-radius: 4px; font-weight: 500; }\n"
        "QHeaderView::section { background-color: %7; color: %12; border: none; border-bottom: 1px solid %4; padding: 4px 8px; font-weight: bold; font-size: 11px; }\n"
        "QScrollBar:vertical { background: transparent; width: 8px; margin: 0px; }\n"
        "QScrollBar::handle:vertical { background: %4; border-radius: 4px; min-height: 24px; }\n"
        "QScrollBar::handle:vertical:hover { background: %13; }\n"
        "QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height: 0px; background: none; }\n"
        "QScrollBar::add-page:vertical, QScrollBar::sub-page:vertical { background: none; }\n"
        "QScrollBar:horizontal { background: transparent; height: 8px; margin: 0px; }\n"
        "QScrollBar::handle:horizontal { background: %4; border-radius: 4px; min-width: 24px; }\n"
        "QScrollBar::handle:horizontal:hover { background: %13; }\n"
        "QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal { width: 0px; background: none; }\n"
        "QScrollBar::add-page:horizontal, QScrollBar::sub-page:horizontal { background: none; }\n"
        "QSlider::groove:horizontal { height: 4px; background: %4; border-radius: 2px; }\n"
        "QSlider::sub-page:horizontal { background: %8; border-radius: 2px; }\n"
        "QSlider::handle:horizontal { width: 14px; height: 14px; margin: -5px 0; background: %2; border: 1px solid %4; border-radius: 7px; }\n"
        "QSlider::handle:horizontal:hover { background: #ffffff; border-color: %8; }\n"
        "QCheckBox, QRadioButton { color: %2; spacing: 6px; }\n"
        "QCheckBox::indicator, QRadioButton::indicator { width: 14px; height: 14px; background-color: %7; border: 1px solid %4; border-radius: 3px; }\n"
        "QRadioButton::indicator { border-radius: 7px; }\n"
        "QCheckBox::indicator:hover, QRadioButton::indicator:hover { border-color: %8; }\n"
        "QCheckBox::indicator:checked, QRadioButton::indicator:checked { background-color: %8; border-color: %8; }\n"
        "QGroupBox { color: %12; font-weight: bold; font-size: 11px; border: 1px solid %4; border-radius: 5px; margin-top: 12px; padding-top: 10px; }\n"
        "QGroupBox::title { subcontrol-origin: margin; subcontrol-position: top left; left: 8px; padding: 0 4px; background-color: %10; }\n"
        "QMainWindow::separator { background-color: %4; width: 3px; height: 3px; border: none; margin: 0px; }\n"
        "QMainWindow::separator:hover { background-color: %8; }\n"
        "QStatusBar { background-color: %3; color: %12; border-top: 1px solid %4; padding: 2px 4px; }\n"
    )
        .arg(bgCanvas, textPrimary, bgHeader, borderSubtle)
        .arg(bgSurfaceHover, bgSurfaceActive, bgSurface, accentColor)
        .arg(textActive, bgPanel, accentOrange, textSecondary)
        .arg(borderStrong, textMuted, borderFocus));
    QFont font = qApp->font();
    font.setPointSizeF(12 * state["ui_scale"].toDouble(1));
    qApp->setFont(font);
    petuniaLocalize(this);
    setDockOptions(
        QMainWindow::AllowNestedDocks | QMainWindow::AllowTabbedDocks |
        (state["reduced_motion"].toBool() ? QMainWindow::DockOptions()
                                          : QMainWindow::AnimatedDocks));
}
bool PetuniaMainWindow::confirmDiscard() {
    if (!petunia_qt_app_is_dirty())
        return true;
    const auto answer = QMessageBox::question(
        this, tr("Alterações não salvas"),
        tr("Salvar as alterações antes de continuar?"),
        QMessageBox::Save | QMessageBox::Discard | QMessageBox::Cancel,
        QMessageBox::Save);
    if (answer == QMessageBox::Cancel)
        return false;
    if (answer == QMessageBox::Discard)
        return true;
    saveProject();
    return !petunia_qt_app_is_dirty();
}
void PetuniaMainWindow::closeEvent(QCloseEvent *event) {
    if (!confirmDiscard()) {
        event->ignore();
        return;
    }
    QSettings settings("Petunia3D", "QtStudio");
    settings.setValue("geometry", saveGeometry());
    settings.setValue("workspace/" + m_activeWorkspace, saveState());
    petuniaRequest("preferences_save");
    petuniaRequest("session_end");
    event->accept();
}
bool PetuniaMainWindow::eventFilter(QObject *watched, QEvent *event) {
    auto *target = qobject_cast<QWidget *>(watched);
    const bool ours = target && (target == this || isAncestorOf(target));
    if (ours && (watched == m_primaryStack || watched == m_viewport) &&
        event->type() == QEvent::Resize) {
        repositionFloatingBars();
    }
    if (ours && event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);
        if ((qobject_cast<PetuniaEditorCanvas *>(target) &&
             (key->key() == Qt::Key_Escape || key->key() == Qt::Key_Home)) ||
            (qobject_cast<PetuniaGlWidget *>(target) &&
             key->key() == Qt::Key_Escape))
            return QMainWindow::eventFilter(watched, event);
    }

    const bool editable = target && (qobject_cast<QLineEdit *>(target) ||
                                     qobject_cast<QAbstractSpinBox *>(target) ||
                                     qobject_cast<QTextEdit *>(target));
    if (ours && !qApp->activeModalWidget() && !qApp->activePopupWidget() &&
        (event->type() == QEvent::ShortcutOverride ||
         event->type() == QEvent::KeyPress)) {
        auto *key = static_cast<QKeyEvent *>(event);
        const auto seq = QKeySequence(key->keyCombination())
                             .toString(QKeySequence::PortableText);
        QString action;
        for (const auto &v :
             m_editorState["keymap_snapshot"].toObject()["rows"].toArray()) {
            const auto row = v.toObject();
            if (QKeySequence(row["shortcut"].toString()) ==
                QKeySequence(key->keyCombination())) {
                action = row["action"].toString();
                break;
            }
        }
        // An explicit editor binding takes priority. Free native menu chords
        // still work (for example Ctrl+A); accepting every ShortcutOverride
        // would suppress those QAction shortcuts before Qt could deliver them.
        QAction *nativeFallback = nullptr;
        if (action.isEmpty() && !editable) {
            for (auto *candidate : findChildren<QAction *>()) {
                if (candidate->isEnabled() && candidate->isVisible() &&
                    !candidate->shortcut().isEmpty() &&
                    candidate->shortcut() ==
                        QKeySequence(key->keyCombination())) {
                    nativeFallback = candidate;
                    break;
                }
            }
        }
        if (nativeFallback) {
            if (event->type() == QEvent::KeyPress) {
                nativeFallback->trigger();
                return true;
            }
            return QMainWindow::eventFilter(watched, event);
        }
        const bool fileAction =
            action == "global.save_project" || action == "global.open_project";
        if (event->type() == QEvent::ShortcutOverride &&
            (!editable || fileAction)) {
            event->accept();
            return true;
        }
        if (event->type() == QEvent::KeyPress && key->key() != Qt::Key_F6 &&
            (!editable || fileAction)) {
            if (action == "global.save_project") {
                saveProject();
                return true;
            }
            if (action == "global.open_project") {
                openProject();
                return true;
            }
            const auto text = petuniaKeyText(key);
            if (target == m_secondaryViewport)
                petunia_qt_view_activate(2);
            auto state = petuniaRequest(
                "shortcut",
                {{"text", text},
                 {"ctrl", bool(key->modifiers() & Qt::ControlModifier)},
                 {"shift", bool(key->modifiers() & Qt::ShiftModifier)},
                 {"alt", bool(key->modifiers() & Qt::AltModifier)}});
            petunia_qt_view_activate(1);
            if (state["changed"].toBool()) {
                syncEditorState(state);
                return true;
            }
        }
    }

    if (event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);
        auto *widget = qobject_cast<QWidget *>(watched);
        if (widget && (widget == this || isAncestorOf(widget)) &&
            !qApp->activeModalWidget() && key->key() == Qt::Key_F6) {
            QList<QWidget *> regions;
            if (m_toolOptionsBar && m_toolOptionsBar->isVisible())
                regions << m_toolOptionsBar;
            else if (m_headerMenuBar && m_headerMenuBar->isVisible())
                regions << m_headerMenuBar;
            else if (menuBar() && menuBar()->isVisible())
                regions << menuBar();

            if (m_toolRailBar && m_toolRailBar->isVisible())
                regions << m_toolRailBar;

            if (m_primaryStack && m_primaryStack->currentWidget())
                regions << m_primaryStack->currentWidget();
            else if (m_viewport)
                regions << m_viewport;

            if (m_outlinerDock && m_outlinerDock->isVisible()) {
                if (m_outlinerSearch && m_outlinerSearch->isVisible())
                    regions << m_outlinerSearch;
                else if (m_outlinerTree)
                    regions << m_outlinerTree;
            }

            if (m_inspectorDock && m_inspectorDock->isVisible() && m_inspectorDock->widget())
                regions << m_inspectorDock->widget();

            for (auto *dock : {m_paintDock, m_uvDock, m_timelineDock}) {
                if (dock && dock->isVisible() && dock->widget()) {
                    regions << dock->widget();
                    break;
                }
            }

            if (m_contextBar && m_contextBar->isVisible())
                regions << m_contextBar;

            for (auto *dock : findChildren<QDockWidget *>())
                if (dock->isFloating() && dock->isVisible() && dock->widget() && !regions.contains(dock->widget()))
                    regions << dock->widget();
            int current = -1;
            for (int i = 0; i < regions.size(); ++i)
                if (regions[i] &&
                    (regions[i] == qApp->focusWidget() ||
                     regions[i]->isAncestorOf(qApp->focusWidget())))
                    current = i;
            const int direction = key->modifiers() & Qt::ShiftModifier ? -1 : 1;
            for (int n = 1; n <= regions.size(); ++n) {
                const int i = (current + direction * n + regions.size() * 2) %
                              regions.size();
                auto *region = regions[i];
                if (!region || !region->isVisible() || !region->isEnabled())
                    continue;
                QWidget *focus = region;
                if (region->focusPolicy() == Qt::NoFocus) {
                    for (auto *child : region->findChildren<QWidget *>())
                        if (child->isVisible() && child->isEnabled() &&
                            child->focusPolicy() != Qt::NoFocus) {
                            focus = child;
                            break;
                        }
                }
                focus->setFocus(Qt::TabFocusReason);
                return true;
            }
        }
    }
    return QMainWindow::eventFilter(watched, event);
}

void PetuniaMainWindow::updateToolRail(const QString &mode,
                                       const QJsonObject &state) {
    if (!m_toolRailBar)
        return;
    if (m_toolRailMode != mode ||
        m_toolRailBar->property("activeLanguage").toString() !=
            state["active_language"].toString() ||
        m_toolRailBar->property("iconColor").toString() !=
            palette().color(QPalette::WindowText).name() ||
        m_toolRailBar->property("iconTheme").toString() !=
            state["icon_theme"].toString()) {
        m_toolRailMode = mode;
        m_toolRailBar->setProperty("activeLanguage",
                                   state["active_language"].toString());
        m_toolRailBar->setProperty(
            "iconColor", palette().color(QPalette::WindowText).name());
        m_toolRailBar->setProperty("iconTheme", state["icon_theme"].toString());
        qDeleteAll(m_toolRailBar->actions());
        m_toolRailBar->clear();
        QStringList tools;
        if (mode == "DRAW")
            tools = {"select",        "move",      "rotate",
                     "scale",         "transform", "draw_profile",
                     "shape_builder", "cursor",    "measure"};
        else if (mode == "POLY")
            tools = {"select",    "lasso_select", "move",     "rotate",
                     "scale",     "transform",    "cursor",   "measure",
                     "primitive"};
        else if (mode == "PAINT")
            tools = {"select",     "brush",      "pixel",
                     "airbrush",   "eraser",     "smudge",
                     "blur",       "dodge",      "burn",
                     "spray",      "clone",      "picker",
                     "fill",       "line",       "rectangle",
                     "ellipse",    "gradient",   "gradient_radial",
                     "path_paint", "projection", "decal"};
        else
            tools = {"select"};
        const QHash<QString, QString> labels{
            {"select", tr("Selecionar")},
            {"move", tr("Mover")},
            {"rotate", tr("Rotacionar")},
            {"scale", tr("Escalar")},
            {"transform", tr("Transformar")},
            {"draw_profile", tr("Desenhar perfil")},
            {"shape_builder", "Shape Builder"},
            {"cursor", "Cursor 3D"},
            {"measure", tr("Medir")},
            {"primitive", tr("Adicionar Primitiva")},
            {"lasso_select", tr("Laço")},
            {"poly_pen", "Poly Pen"},
            {"extrude", tr("Extrudar")},
            {"push_pull", "Push/Pull"},
            {"inset", "Inset"},
            {"bevel", "Bevel"},
            {"cut", tr("Faca")},
            {"loop_cut", "Loop Cut"},
            {"slice", tr("Cortar")},
            {"brush", tr("Pincel")},
            {"eraser", tr("Borracha")},
            {"picker", tr("Conta-gotas")},
            {"fill", tr("Preencher")},
            {"line", tr("Linha")},
            {"rectangle", tr("Retângulo")},
            {"ellipse", tr("Elipse")}};
        const QHash<QString, QString> icons{{"select", "select_box"},
            {"move", "move"},
            {"rotate", "rotate"},
            {"scale", "scale"},
            {"transform", "transform"},
            {"cursor", "cursor_3d"},
            {"measure", "measure"},
            {"primitive", "cube"},
            {"draw_profile", "draw_profile"},
            {"shape_builder", "shape_layer"},
            {"lasso_select", "select_lasso"},
            {"poly_pen", "tool_poly_pen"},
            {"extrude", "extrude"},
            {"push_pull", "pushpull"},
            {"inset", "inset"},
            {"bevel", "bevel"},
            {"cut", "knife"},
            {"loop_cut", "loop_cut"},
            {"slice", "slice"},
            {"brush", "paint_brush"},
            {"eraser", "paint_eraser"},
            {"smudge", "paint_smudge"},
            {"clone", "paint_clone"},
            {"picker", "paint_picker"},
            {"fill", "paint_fill"},
            {"line", "paint_line"},
            {"rectangle", "paint_rect"},
            {"ellipse", "shape_circle"},
            {"decal", "paint_decal"}};
        const QHash<QString, QString> shortcuts{
            {"select", "W"},
            {"move", "G"},
            {"rotate", "R"},
            {"scale", "S"},
            {"cursor", "Shift+S"},
            {"measure", "M"},
            {"lasso_select", "Q"},
            {"poly_pen", "P"},
            {"extrude", "E"},
            {"inset", "I"},
            {"bevel", "Ctrl+B"},
            {"cut", "K"},
            {"loop_cut", "Ctrl+R"},
            {"brush", "B"},
            {"eraser", "E"},
            {"picker", "I"}
        };
        int i = 0;
        for (const auto &tool : tools) {
            if (tool == "move" || tool == "cursor" || tool == "primitive") {
                m_toolRailBar->addSeparator();
            }
            auto *action = m_toolRailBar->addAction(QString::number(++i));
            action->setCheckable(tool != "primitive");
            action->setProperty("toolId", tool);
            action->setProperty(
                "toolLabel",
                labels.value(tool,
                             tr(qPrintable(QString(tool).replace('_', ' ')))));
            QString tip = action->property("toolLabel").toString();
            if (shortcuts.contains(tool)) {
                tip += QStringLiteral(" (%1)").arg(shortcuts.value(tool));
            }
            action->setToolTip(tip);
            if (icons.contains(tool)) {
                const auto theme =
                    state["icon_theme"] == "filled" ? "filled" : "outline";
                QFile resource(QString(":/toolbar/") + theme + "/" +
                               icons.value(tool) + ".svg");
                if (resource.open(QIODevice::ReadOnly)) {
                    auto svg = resource.readAll();
                    svg.replace(
                        "currentColor",
                        palette().color(QPalette::WindowText).name().toUtf8());
                    QSvgRenderer renderer(svg);
                    if (renderer.isValid()) {
                        QPixmap pixmap(96, 96);
                        pixmap.fill(Qt::transparent);
                        QPainter painter(&pixmap);
                        renderer.render(&painter);
                        action->setIcon(QIcon(pixmap));
                    }
                }
                if (action->icon().isNull()) {
                    action->setIcon(PetuniaIcons::get(icons.value(tool)));
                }
            }
            if (tool == "primitive") {
                connect(action, &QAction::triggered, this, [this, action]() {
                    auto *btn = qobject_cast<QToolButton *>(m_toolRailBar->widgetForAction(action));
                    QMenu menu(this);
                    menu.setStyleSheet(QStringLiteral(
                        "QMenu { background-color: #202020; border: 1px solid #444444; color: #eeeeee; font-size: 12px; padding: 4px; }"
                        "QMenu::item { padding: 6px 18px; border-radius: 4px; }"
                        "QMenu::item:selected { background-color: #b58cff; color: #121212; font-weight: bold; }"
                    ));
                    const struct { const char *id; const char *name; const char *icon; } prims[] = {
                        {"cube", "Cubo (Cube)", "cube"},
                        {"cylinder", "Cilindro (Cylinder)", "cylinder"},
                        {"sphere", "Esfera UV (Sphere)", "sphere"},
                        {"plane", "Plano (Plane)", "plane"},
                        {"cone", "Cone", "cone"},
                        {"torus", "Toro (Torus)", "torus"},
                    };
                    for (const auto &p : prims) {
                        auto *item = menu.addAction(PetuniaIcons::get(p.icon), QString::fromUtf8(p.name));
                        connect(item, &QAction::triggered, this, [this, id = p.id]() {
                            emit actionTriggered(QStringLiteral("model.add_%1").arg(id));
                            showPrimitiveCard(id);
                        });
                    }
                    QPoint pos = btn ? btn->mapToGlobal(QPoint(btn->width() + 2, 0)) : QCursor::pos();
                    menu.exec(pos);
                });
            } else {
                connect(action, &QAction::triggered, this, [this, tool] {
                    petuniaRequest("tool", {{"id", tool}});
                    syncFromApplication();
                });
            }
        }
        m_toolRailBar->addSeparator();
        auto *toggleAct = m_toolRailBar->addAction(QString());
        toggleAct->setProperty("toolId", "toggle_rail_labels");
        connect(toggleAct, &QAction::triggered, this, [this]() {
            bool current = petuniaRequest("snapshot")["show_tool_labels"].toBool();
            petuniaRequest("set_show_tool_labels", {{"enabled", !current}});
            syncFromApplication();
        });
    }
    const bool expanded = state["show_tool_labels"].toBool();
    if (m_toolRailBar->property("expanded").toBool() != expanded) {
        m_toolRailBar->setProperty("expanded", expanded);
        m_toolRailBar->style()->unpolish(m_toolRailBar);
        m_toolRailBar->style()->polish(m_toolRailBar);
    }
    for (auto *action : m_toolRailBar->actions()) {
        const auto toolId = action->property("toolId").toString();
        if (toolId == "toggle_rail_labels") {
            action->setIcon(PetuniaIcons::get(expanded ? "chevron_left" : "chevron_right"));
            action->setText(expanded ? tr("Recolher barra") : QString());
            action->setToolTip(expanded ? tr("Recolher barra de ferramentas (apenas ícones)")
                                        : tr("Expandir barra de ferramentas (exibir rótulos)"));
            if (auto *button = qobject_cast<QToolButton *>(
                    m_toolRailBar->widgetForAction(action))) {
                button->setToolButtonStyle(expanded ? Qt::ToolButtonTextBesideIcon
                                                    : Qt::ToolButtonIconOnly);
                button->setAccessibleName(action->toolTip());
            }
            continue;
        }
        const QSignalBlocker blocker(action);
        action->setChecked(toolId == state["active_tool"].toString());
        const auto label = action->property("toolLabel").toString();
        action->setText(label);
        if (auto *button = qobject_cast<QToolButton *>(
                m_toolRailBar->widgetForAction(action))) {
            button->setToolButtonStyle(expanded || action->icon().isNull()
                                           ? Qt::ToolButtonTextBesideIcon
                                           : Qt::ToolButtonIconOnly);
            button->setAccessibleName(label);
            button->setAccessibleDescription(action->toolTip());
        }
    }
}

// Used by the final native acceptance executable, on the QApplication thread.
extern "C" const char *petunia_qt_native_acceptance(const char *directory) {
    static QByteArray output;
    QJsonArray checks;
    bool passed = true;
    auto check = [&](const QString &name, bool ok) {
        checks.append(QJsonObject{{"name", name}, {"pass", ok}});
        passed &= ok;
    };
    const auto circle =
        petuniaPath("M 10 10 m -4 0 a 4 4 0 1 0 8 0 a 4 4 0 1 0 -8 0");
    check("shared-svg-relative-arcs",
          circle.contains(QPointF(10, 10)) &&
              qAbs(circle.boundingRect().width() - 8.) < 0.01 &&
              qAbs(circle.boundingRect().height() - 8.) < 0.01);
    petuniaRequest("set_show_home_on_start", {{"enabled", false}});
    petuniaRequest("set_show_tool_labels", {{"enabled", false}});
    petuniaRequest("keep_saved_project");
    PetuniaMainWindow window;
    window.resize(1280, 800);
    window.show();
    auto flush = [&] {
        QApplication::processEvents();
        window.syncFromApplication();
        window.repaint();
        QApplication::processEvents();
    };
    flush();
    const bool expectCpu =
        QGuiApplication::platformName() == "offscreen" ||
        QGuiApplication::platformName() == "minimal" ||
        qEnvironmentVariableIsSet("PETUNIA_VIEWPORT_SOFTWARE");
    const auto backend = window.viewport()->renderingBackend();
    check("viewport-backend-" + backend,
          backend == (expectCpu ? "cpu" : "opengl"));
    bool selectionIcon = false;
    for (auto *action : window.findChildren<QAction *>())
        if (action->property("toolId") == "select")
            selectionIcon = !action->icon().pixmap(24, 24).isNull();
    check("canonical-tool-icon", selectionIcon);
    auto selectImage = [&] {
        for (auto *action : window.findChildren<QAction *>())
            if (action->property("toolId") == "select")
                return action->icon().pixmap(24, 24).toImage();
        return QImage();
    };
    petuniaRequest("set_icon_theme", {{"theme", "outline"}});
    flush();
    const auto outlineIcon = selectImage();
    petuniaRequest("set_icon_theme", {{"theme", "filled"}});
    flush();
    check("native-icon-theme-changes-pixels", !outlineIcon.isNull() &&
                                                  !selectImage().isNull() &&
                                                  outlineIcon != selectImage());
    petuniaRequest("set_icon_theme", {{"theme", "outline"}});
    flush();
    for (const auto &mode :
         QStringList{"DRAW", "POLY", "PAINT", "UV", "ANIMATE"}) {
        QPushButton *button = nullptr;
        for (auto *candidate : window.findChildren<QPushButton *>())
            if (candidate->property("workspaceId").toString() == mode)
                button = candidate;
        check("native-tab-" + mode, button != nullptr);
        if (button)
            button->click();
        flush();
        check("workspace-button-state-" + mode, button && button->isChecked());
        if (mode == "PAINT") {
            petuniaRequest("add_paint_layer");
            petuniaRequest("tool", {{"id", "brush"}});
            petuniaRequest("brush_size", {{"value", 24}});
            petuniaRequest("paint_color", {{"r", 0.1}, {"g", 0.5}, {"b", 0.9}});
            petuniaRequest("paint_stroke",
                           {{"x", 0.3}, {"y", 0.5}, {"phase", 0}});
            petuniaRequest("paint_stroke",
                           {{"x", 0.7}, {"y", 0.5}, {"phase", 1}});
            petuniaRequest("paint_stroke",
                           {{"x", 0.7}, {"y", 0.5}, {"phase", 2}});
        }
        if (mode == "ANIMATE") {
            auto a = petuniaRequest("snapshot")["animate"].toObject();
            const auto preset = a["creature_presets"].toArray();
            if (!preset.isEmpty())
                petuniaRequest("animate",
                               {{"id", "add-creature"},
                                {"arg", preset.first().toObject()["id"]}});
            petuniaRequest("animate", {{"id", "fit-to-model"}});
            a = petuniaRequest("snapshot")["animate"].toObject();
            for (const auto &v : a["catalog"].toArray())
                if (v.toObject()["available"].toBool()) {
                    petuniaRequest("animate", {{"id", "add-motion"},
                                               {"arg", v.toObject()["id"]}});
                    break;
                }
            const auto bones = petuniaRequest("snapshot")["animate"]
                                   .toObject()["bones"]
                                   .toArray();
            if (!bones.isEmpty())
                petuniaRequest("animate",
                               {{"id", "select-bone"},
                                {"arg", bones.first().toObject()["id"]}});
            petuniaRequest("animate", {{"id", "seek"}, {"value", 0.5}});
            petuniaRequest("animate", {{"id", "insert-keyframe"}});
        }
        flush();
        const auto snapshot = petuniaRequest("snapshot");
        const QString expected = (mode == "DRAW" || mode == "POLY") ? "Model"
                                 : mode == "PAINT"                  ? "Paint"
                                 : mode == "UV"                     ? "Uv"
                                                                    : "Animate";
        check("document-workspace-" + mode,
              snapshot["workspace"].toString().compare(
                  expected, Qt::CaseInsensitive) == 0);
        check("capture-" + mode,
              window.grab().save(QString::fromUtf8(directory) + "/" + mode +
                                 ".png"));
        check("viewport-space-" + mode,
              window.centralWidget()->height() >= window.height() / 3);
        if (mode == "PAINT") {
            petuniaRequest("toggle_paint_pip");
            flush();
            auto *stack = window.findChild<QStackedWidget *>();
            check("paint-primary-canvas",
                  stack && qobject_cast<PetuniaEditorCanvas *>(
                               stack->currentWidget()));
            check("capture-paint-pip",
                  window.grab().save(QString::fromUtf8(directory) +
                                     "/paint-pip.png"));
            petuniaRequest("toggle_paint_pip");
            flush();
        }
    }
    window.setWorkspace("POLY");
    window.toggleSplitView();
    flush();
    check("capture-split",
          window.grab().save(QString::fromUtf8(directory) + "/split.png"));
    window.toggleSplitView();
    flush();
    petuniaRequest("tool", {{"id", "select"}});
    petuniaRequest("domain", {{"id", "object"}});
    petuniaRequest("clear_selection");
    flush();
    auto click = [&](const QPointF &at) {
        auto *view = window.viewport();
        QMouseEvent down(QEvent::MouseButtonPress, at,
                         QPointF(view->mapToGlobal(at.toPoint())),
                         Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(view, &down);
        QMouseEvent up(QEvent::MouseButtonRelease, at,
                       QPointF(view->mapToGlobal(at.toPoint())), Qt::LeftButton,
                       Qt::NoButton, Qt::NoModifier);
        QApplication::sendEvent(view, &up);
        flush();
    };
    click(QPointF(window.viewport()->width() / 2.,
                  window.viewport()->height() / 2.));
    check("native-pointer-picks",
          petuniaRequest("snapshot")["selected_count"].toInt() > 0);
    click(QPointF(2, 2));
    check("native-pointer-clears",
          petuniaRequest("snapshot")["selected_count"].toInt() == 0);
    QKeyEvent selectAll(QEvent::KeyPress, Qt::Key_A, Qt::ControlModifier,
                        QString(QChar(1)));
    QApplication::sendEvent(window.viewport(), &selectAll);
    flush();
    check("native-control-shortcut",
          petuniaRequest("snapshot")["selected_count"].toInt() > 0);
    {
        QLineEdit field(&window);
        field.setText("numeric input");
        field.show();
        field.setFocus();
        QKeyEvent editSelect(QEvent::KeyPress, Qt::Key_A, Qt::ControlModifier,
                             QString(QChar(1)));
        QApplication::sendEvent(&field, &editSelect);
        check("native-editing-keeps-control-a",
              field.selectedText() == "numeric input");
    }
    auto *before = QApplication::focusWidget();
    QKeyEvent focus(QEvent::KeyPress, Qt::Key_F6, Qt::NoModifier);
    QApplication::sendEvent(&window, &focus);
    check("F6-focus-region", QApplication::focusWidget() != nullptr &&
                                 QApplication::focusWidget() != before);
    auto docks = window.findChildren<QDockWidget *>();
    if (!docks.isEmpty()) {
        docks.first()->setFloating(true);
        flush();
        check("native-floating-dock", docks.first()->isFloating());
        docks.first()->setFloating(false);
        flush();
    }
    window.resize(850, 600);
    flush();
    check("capture-responsive",
          window.grab().save(QString::fromUtf8(directory) + "/responsive.png"));
    petuniaRequest("set_language", {{"lang", "en"}});
    flush();
    check("english-preference",
          petuniaRequest("snapshot")["active_language"] == "en");
    check("capture-en",
          window.grab().save(QString::fromUtf8(directory) + "/english.png"));
    petuniaRequest("set_language", {{"lang", "pt-BR"}});
    flush();
    QJsonObject bindings;
    for (const auto &mode : QStringList{"MODEL", "PAINT", "UV", "ANIMATE",
                                        "SETTINGS", "REFERENCES", "ASSETS"}) {
        PetuniaEditorPanel panel(mode);
        const auto missing = panel.missingBindings(petuniaRequest("snapshot"));
        bindings[mode] = QJsonArray::fromStringList(missing);
        check("native-bindings-" + mode, missing.isEmpty());
    }
    uint32_t w = 0, h = 0;
    auto bytes =
        petunia_qt_software_frame_rgba(1, 400, 300, nullptr, 0, &w, &h);
    check("software-frame-sized", bytes == size_t(w) * h * 4 && w > 0 && h > 0);
    std::vector<uint8_t> pixels(bytes);
    if (bytes)
        petunia_qt_software_frame_rgba(1, 400, 300, pixels.data(), bytes, &w,
                                       &h);
    bool varied = false;
    for (size_t i = 4; i + 3 < pixels.size(); i += 4)
        if (pixels[i] != pixels[0] || pixels[i + 1] != pixels[1] ||
            pixels[i + 2] != pixels[2]) {
            varied = true;
            break;
        }
    check("software-renders-scene", varied);
    output =
        QJsonDocument(QJsonObject{{"pass", passed},
                                  {"checks", checks},
                                  {"missing_bindings", bindings},
                                  {"platform", QGuiApplication::platformName()},
                                  {"viewport_backend", backend}})
            .toJson(QJsonDocument::Indented);
    return output.constData();
}
