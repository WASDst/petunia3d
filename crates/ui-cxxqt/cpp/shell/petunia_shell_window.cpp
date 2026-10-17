#include "petunia_shell_window.h"

#include "../petunia_command_palette.h"
#include "../petunia_editor_panel.h"
#include "../petunia_gl_widget.h"
#include "../petunia_home_dialog.h"
#include "../petunia_icons.h"
#include "../petunia_recovery_dialog.h"
#include "../petunia_theme.h"
#include "petunia_app_bar.h"
#include "petunia_context_actions.h"
#include "petunia_drawer.h"
#include "petunia_i18n.h"
#include "petunia_node_canvas.h"
#include "petunia_operation_hud.h"
#include "petunia_properties_panel.h"
#include "petunia_region_navigator.h"
#include "petunia_status_bar.h"
#include "petunia_structure_panel.h"
#include "petunia_tool_rail.h"
#include "petunia_viewport_header.h"

#include <QAbstractSpinBox>
#include <QApplication>
#include <QCloseEvent>
#include <QComboBox>
#include <QDialog>
#include <QFileDialog>
#include <QFileInfo>
#include <QFrame>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QKeyEvent>
#include <QLineEdit>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSettings>
#include <QSplitter>
#include <QStackedWidget>
#include <QTextEdit>
#include <QTimer>
#include <QVBoxLayout>
#include <algorithm>

namespace {
QString projectDisplayName(const QString &path) {
    return path.isEmpty() ? QStringLiteral("Sem título") : QFileInfo(path).completeBaseName();
}
} // namespace

PetuniaShellWindow::PetuniaShellWindow(QWidget *parent, bool interactive)
    : QMainWindow(parent), m_interactive(interactive) {
    setObjectName(QStringLiteral("PetuniaShellWindow"));
    resize(1360, 860);
    setMinimumSize(800, 560);

    // 1. Sessão e primeiro estado: o idioma e o tema são conhecidos antes de
    //    construir os widgets (os textos `tr` já nascem traduzidos).
    m_controller.start();
    const auto &first = m_controller.state();
    petuniaSetTranslations(petuniaRequest(QStringLiteral("translations")).value(QStringLiteral("translations")).toObject());
    auto &theme = PetuniaTheme::instance();
    theme.setUiScale(first.value(QStringLiteral("ui_scale")).toDouble(1.0));
    theme.setReducedMotion(first.value(QStringLiteral("reduced_motion")).toBool());
    theme.updateFromSnapshot(first.value(QStringLiteral("theme_colors")).toObject(),
                             first.value(QStringLiteral("current_theme")).toString());
    theme.applyToApplication();
    connect(&theme, &PetuniaTheme::changed, this, [] { PetuniaTheme::instance().applyToApplication(); });

    buildUi();
    qApp->installEventFilter(this);
    connect(&m_controller, &PetuniaShellController::stateChanged, this, &PetuniaShellWindow::onState);
    m_layoutTimer.setSingleShot(true);
    m_layoutTimer.setInterval(400);
    connect(&m_layoutTimer, &QTimer::timeout, this, &PetuniaShellWindow::captureLayout);

    QSettings settings(QStringLiteral("Petunia3D"), QStringLiteral("QtShell"));
    restoreGeometry(settings.value(QStringLiteral("geometry")).toByteArray());
    onState(m_controller.state());
    if (m_interactive && m_controller.flag(QStringLiteral("show_home_on_start")))
        QTimer::singleShot(0, this, &PetuniaShellWindow::openHome);
}

PetuniaShellWindow::~PetuniaShellWindow() {
    m_controller.stop();
    // Páginas fora da gaveta ficam sem pai (clearPages): este é o único dono delas.
    if (m_nodeCanvas && !m_nodeCanvas->parentWidget())
        delete m_nodeCanvas;
    if (m_assetsHost && !m_assetsHost->parentWidget())
        delete m_assetsHost;
}

void PetuniaShellWindow::buildUi() {
    auto *central = new QWidget(this);
    central->setObjectName(QStringLiteral("PetuniaShellCentral"));
    auto *vbox = new QVBoxLayout(central);
    vbox->setContentsMargins(0, 0, 0, 0);
    vbox->setSpacing(0);

    m_appBar = new PetuniaAppBar(&m_controller, central);
    vbox->addWidget(m_appBar);
    auto *line = new QFrame(central);
    line->setProperty("role", QStringLiteral("separator"));
    line->setFixedHeight(1);
    vbox->addWidget(line);

    auto *bodyRow = new QWidget(central);
    auto *row = new QHBoxLayout(bodyRow);
    row->setContentsMargins(0, 0, 0, 0);
    row->setSpacing(0);
    m_rail = new PetuniaToolRail(&m_controller, bodyRow);
    row->addWidget(m_rail);
    auto *railLine = new QFrame(bodyRow);
    railLine->setProperty("role", QStringLiteral("separator"));
    railLine->setFixedWidth(1);
    row->addWidget(railLine);

    m_body = new QSplitter(Qt::Horizontal, bodyRow);
    m_body->setChildrenCollapsible(false);
    m_body->setHandleWidth(PetuniaTheme::instance().space(1) + 1);
    row->addWidget(m_body, 1);
    vbox->addWidget(bodyRow, 1);

    // Coluna central: cabeçalho + superfície de trabalho; gaveta embaixo.
    m_middle = new QSplitter(Qt::Vertical, m_body);
    m_middle->setChildrenCollapsible(false);
    m_middle->setMinimumWidth(PetuniaTheme::instance().scaled(470));
    m_middle->setHandleWidth(PetuniaTheme::instance().space(1) + 1);
    m_workSurface = new QWidget(m_middle);
    auto *surfaceLayout = new QVBoxLayout(m_workSurface);
    surfaceLayout->setContentsMargins(0, 0, 0, 0);
    surfaceLayout->setSpacing(0);
    m_header = new PetuniaViewportHeader(&m_controller, m_workSurface);
    surfaceLayout->addWidget(m_header);
    m_surface = new QSplitter(Qt::Horizontal, m_workSurface);
    m_surface->setChildrenCollapsible(false);
    surfaceLayout->addWidget(m_surface, 1);

    m_viewArea = new QWidget(m_surface);
    m_viewArea->setObjectName(QStringLiteral("PetuniaViewArea"));
    auto *viewLayout = new QVBoxLayout(m_viewArea);
    viewLayout->setContentsMargins(0, 0, 0, 0);
    m_viewport = new PetuniaGlWidget(m_viewArea);
    m_viewport->setShellChrome(true);
    m_viewport->setAccessibleName(tr("Vista 3D"));
    m_viewport->setFocusPolicy(Qt::StrongFocus);
    viewLayout->addWidget(m_viewport);
    connect(m_viewport, &PetuniaGlWidget::selectionChanged, &m_controller, &PetuniaShellController::refresh);
    m_hud = new PetuniaOperationHud(&m_controller, m_viewArea);
    m_context = new PetuniaContextActions(&m_controller, m_viewArea);
    m_viewArea->installEventFilter(this);
    m_surface->addWidget(m_viewArea);

    m_secondaryArea = new QWidget(m_surface);
    auto *secondaryLayout = new QVBoxLayout(m_secondaryArea);
    secondaryLayout->setContentsMargins(0, 0, 0, 0);
    m_secondary = new PetuniaGlWidget(m_secondaryArea);
    m_secondary->setSurfaceId(2);
    m_secondary->setShellChrome(true);
    m_secondary->setAccessibleName(tr("Vista 3D secundária"));
    secondaryLayout->addWidget(m_secondary);
    connect(m_secondary, &PetuniaGlWidget::selectionChanged, &m_controller, &PetuniaShellController::refresh);
    m_surface->addWidget(m_secondaryArea);
    m_secondaryArea->hide();

    m_drawer = new PetuniaDrawer(m_middle);
    m_middle->addWidget(m_workSurface);
    m_middle->addWidget(m_drawer);
    m_middle->setStretchFactor(0, 1);
    m_middle->setStretchFactor(1, 0);
    m_middle->setCollapsible(0, false);
    m_middle->setCollapsible(1, false);

    // Coluna direita: Estrutura (partes) e Propriedades.
    m_right = new QSplitter(Qt::Vertical, m_body);
    m_right->setChildrenCollapsible(false);
    m_right->setMinimumWidth(PetuniaTheme::instance().scaled(240));
    m_right->setHandleWidth(PetuniaTheme::instance().space(1) + 1);
    m_structure = new PetuniaStructurePanel(&m_controller, m_right);
    m_properties = new PetuniaPropertiesPanel(&m_controller, m_right);
    m_right->addWidget(m_structure);
    m_right->addWidget(m_properties);
    m_body->addWidget(m_middle);
    m_body->addWidget(m_right);
    m_body->setStretchFactor(0, 1);
    m_body->setStretchFactor(1, 0);

    m_status = new PetuniaStatusBar(this);
    setStatusBar(m_status);
    connect(&m_controller, &PetuniaShellController::errorReported, m_status, &PetuniaStatusBar::showProblem);
    setCentralWidget(central);

    m_navigator = new PetuniaRegionNavigator(this);
    m_navigator->setRegions({m_appBar, m_rail, m_viewport, m_structure, m_properties, m_drawer, m_context});

    connect(m_appBar, &PetuniaAppBar::localAction, this, &PetuniaShellWindow::triggerLocalAction);
    connect(m_structure, &PetuniaStructurePanel::createRequested, this,
            [this] { m_controller.command(QStringLiteral("model.add_cube")); });
    connect(m_rail, &PetuniaToolRail::expandedChanged, this, [this](bool) { scheduleLayoutSave(); });
    connect(m_properties, &PetuniaPropertiesPanel::sectionToggled, this, [this](const QString &, bool) { scheduleLayoutSave(); });
    connect(m_drawer, &PetuniaDrawer::openChanged, this, [this](bool open) {
        if (m_assetsHost && open && !m_assetsPanel && m_drawer->currentPageId() == QLatin1String("assets")) {
            m_assetsPanel = new PetuniaEditorPanel(QStringLiteral("ASSETS"), m_assetsHost);
            m_assetsPanel->setTransport([this](const QString &a, const QJsonObject &args) { return m_controller.request(a, args); });
            m_assetsHost->layout()->addWidget(m_assetsPanel);
        }
        if (!m_applyingLayout) {
            // Guarda o estado atual (aberto/fechado) antes de reaplicar: senão o layout
            // salvo (gaveta fechada) desfaz a ação do usuário.
            captureLayout();
            applyLayout(m_workspace);
            scheduleLayoutSave();
        }
    });
    for (auto *splitter : {m_body, m_middle, m_right})
        connect(splitter, &QSplitter::splitterMoved, this, [this] { scheduleLayoutSave(); });
}

int PetuniaShellWindow::focusedRegion() const {
    return m_navigator ? m_navigator->regionOf(QApplication::focusWidget()) : -1;
}

void PetuniaShellWindow::syncNow() {
    m_controller.refresh();
    onState(m_controller.state());
}

// ── Estado ──────────────────────────────────────────────────────────────────

void PetuniaShellWindow::onState(const QJsonObject &state) {
    if (state.isEmpty())
        return;
    auto &theme = PetuniaTheme::instance();
    theme.setUiScale(state["ui_scale"].toDouble(1.0));
    theme.setReducedMotion(state["reduced_motion"].toBool());
    theme.updateFromSnapshot(state["theme_colors"].toObject(), state["current_theme"].toString());

    const QString language = state["active_language"].toString();
    if (!m_language.isEmpty() && language != m_language) {
        // O idioma mudou: refaz as regiões (cada texto nasce traduzido) e reaplica o layout.
        m_language = language;
        petuniaSetTranslations(petuniaRequest(QStringLiteral("translations")).value(QStringLiteral("translations")).toObject());
        retranslateUi();
        return;
    }
    m_language = language;

    const QString workspace = m_controller.workspaceId();
    if (workspace != m_workspace)
        onWorkspaceChanged(workspace);

    if (m_nodeCanvas)
        m_nodeCanvas->sync(state);
    m_appBar->sync(state);
    m_rail->sync(state);
    m_header->sync(state);
    m_structure->sync(state);
    m_properties->sync(state);
    m_hud->sync(state);
    // Durante uma operação, os controles da operação têm prioridade sobre as ações da seleção.
    m_context->setSuppressed(!m_hud->activeSections().isEmpty());
    m_context->sync(state);
    m_status->sync(state);
    if (auto *panel = m_specialized.value(workspace))
        panel->sync(state);

    m_viewport->setExternalState(state);
    m_viewport->update();
    const bool split = state["split_enabled"].toBool() && m_controller.isModeling();
    if (m_secondaryArea->isVisible() != split) {
        m_secondaryArea->setVisible(split);
        if (split)
            m_surface->setSizes({1, 1});
    }
    if (split) {
        m_secondary->setExternalState(state);
        m_secondary->update();
    }

    m_projectPath = state["project_path"].toString();
    setWindowTitle(QStringLiteral("Petunia3D — %1[*]").arg(projectDisplayName(m_projectPath)));
    setWindowModified(!state["saved"].toBool(true));
    repositionOverlays();
    openHostDialogs(state);
}

void PetuniaShellWindow::retranslateUi() {
    // Canvases e hospedeiros podem estar fora da árvore (gaveta/abas): solta-os antes
    // de destruir a central e libera cada um exatamente uma vez.
    for (auto *canvas : std::as_const(m_canvases))
        canvas->setParent(nullptr);
    if (m_assetsHost)
        m_assetsHost->setParent(nullptr);
    if (m_nodeCanvas)
        m_nodeCanvas->setParent(nullptr);
    auto *old = takeCentralWidget();
    delete old;
    qDeleteAll(m_canvases);
    qDeleteAll(m_specialized);
    delete m_assetsHost;
    delete m_nodeCanvas;
    m_nodeCanvas = nullptr;
    m_canvases.clear();
    m_specialized.clear();
    m_assetsHost = nullptr;
    m_assetsPanel = nullptr;
    m_workspace.clear();
    buildUi();
    onState(m_controller.state());
}

void PetuniaShellWindow::onWorkspaceChanged(const QString &workspace) {
    if (!m_workspace.isEmpty())
        captureLayout();
    m_workspace = workspace;
    ensureSpecialized(workspace);
    arrangeSurface(workspace);

    m_drawer->clearPages();
    if (workspace == QLatin1String("ANIMATE")) {
        if (auto *timeline = m_canvases.value(QStringLiteral("ANIMATE")))
            m_drawer->addPage(QStringLiteral("timeline"), tr("Linha do tempo"), QStringLiteral("timeline"), timeline);
    } else {
        if (!m_assetsHost) {
            m_assetsHost = new QWidget(this);
            auto *layout = new QVBoxLayout(m_assetsHost);
            layout->setContentsMargins(0, 0, 0, 0);
        }
        m_drawer->addPage(QStringLiteral("assets"), tr("Assets"), QStringLiteral("asset_library"), m_assetsHost);
        if (m_controller.isModeling()) {
            if (!m_nodeCanvas)
                m_nodeCanvas = new PetuniaNodeCanvas(&m_controller, this);
            m_drawer->addPage(QStringLiteral("nodes"), tr("Nós"), QStringLiteral("link"), m_nodeCanvas);
        }
    }
    applyLayout(workspace);
}

void PetuniaShellWindow::ensureSpecialized(const QString &workspace) {
    if (workspace != QLatin1String("PAINT") && workspace != QLatin1String("UV") && workspace != QLatin1String("ANIMATE"))
        return;
    if (m_specialized.contains(workspace))
        return;
    auto *panel = new PetuniaEditorPanel(workspace, this);
    panel->hide();
    panel->setTransport([this](const QString &a, const QJsonObject &args) { return m_controller.request(a, args); });
    m_specialized.insert(workspace, panel);
    if (auto *canvas = panel->takeCanvas())
        m_canvases.insert(workspace, canvas);
    if (auto *tabs = panel->takeTabs())
        m_properties->setHosted(workspace, tabs);
    panel->sync(m_controller.state());
}

void PetuniaShellWindow::arrangeSurface(const QString &workspace) {
    for (const auto &id : {QStringLiteral("PAINT"), QStringLiteral("UV")})
        if (auto *canvas = m_canvases.value(id))
            if (canvas->parentWidget() == m_surface)
                canvas->hide();
    if (workspace == QLatin1String("PAINT")) {
        if (auto *canvas = m_canvases.value(workspace)) {
            m_surface->insertWidget(1, canvas);
            canvas->show();
            const int w = std::max(m_surface->width(), 800);
            m_surface->setSizes({w * 45 / 100, w * 55 / 100});
        }
    } else if (workspace == QLatin1String("UV")) {
        if (auto *canvas = m_canvases.value(workspace)) {
            m_surface->insertWidget(0, canvas);
            canvas->show();
            const int w = std::max(m_surface->width(), 800);
            m_surface->setSizes({w * 62 / 100, w * 38 / 100});
        }
    }
}

// ── Layout ──────────────────────────────────────────────────────────────────

void PetuniaShellWindow::applyLayout(const QString &workspace) {
    if (workspace.isEmpty())
        return;
    m_applyingLayout = true;
    const auto layout = m_layoutStore.load(workspace);
    // A coluna direita nunca passa de ~36% da janela: a vista mantém largura útil.
    const int right = std::min(layout[PetuniaLayoutStore::kRightWidth].toInt(), int(width() * 0.36));
    const int totalW = std::max(m_body->width(), right + 400);
    m_body->setSizes({totalW - right, right});
    const double ratio = layout[PetuniaLayoutStore::kStructureRatio].toDouble();
    const int totalH = std::max(m_right->height(), 400);
    m_right->setSizes({int(totalH * ratio), int(totalH * (1.0 - ratio))});
    m_right->setVisible(!layout[PetuniaLayoutStore::kRightCollapsed].toBool());
    m_rail->setExpanded(layout[PetuniaLayoutStore::kRailExpanded].toBool());
    m_properties->applySectionStates(layout[PetuniaLayoutStore::kSections].toObject());

    const bool open = layout[PetuniaLayoutStore::kDrawerOpen].toBool();
    const int drawerH = layout[PetuniaLayoutStore::kDrawerHeight].toInt();
    const int bar = m_drawer->barHeight();
    m_drawer->setOpen(open);
    if (m_drawer->isOpen()) {
        m_drawer->setMinimumHeight(bar + 80);
        m_drawer->setMaximumHeight(QWIDGETSIZE_MAX);
        const int totalMid = std::max(m_middle->height(), drawerH + 240);
        m_middle->setSizes({totalMid - drawerH, drawerH});
    } else {
        m_drawer->setMinimumHeight(bar);
        m_drawer->setMaximumHeight(bar);
    }
    m_applyingLayout = false;
}

void PetuniaShellWindow::captureLayout() {
    if (m_workspace.isEmpty() || m_applyingLayout)
        return;
    auto layout = m_layoutStore.load(m_workspace);
    const auto bodySizes = m_body->sizes();
    if (bodySizes.size() == 2 && m_right->isVisible() && bodySizes[1] > 0)
        layout[PetuniaLayoutStore::kRightWidth] = bodySizes[1];
    const auto rightSizes = m_right->sizes();
    if (rightSizes.size() == 2 && rightSizes[0] + rightSizes[1] > 0)
        layout[PetuniaLayoutStore::kStructureRatio] = double(rightSizes[0]) / double(rightSizes[0] + rightSizes[1]);
    layout[PetuniaLayoutStore::kRightCollapsed] = !m_right->isVisible() && !m_right->isVisibleTo(this);
    layout[PetuniaLayoutStore::kRailExpanded] = m_rail->expanded();
    layout[PetuniaLayoutStore::kDrawerOpen] = m_drawer->isOpen();
    if (m_drawer->isOpen()) {
        const auto midSizes = m_middle->sizes();
        if (midSizes.size() == 2 && midSizes[1] > 0)
            layout[PetuniaLayoutStore::kDrawerHeight] = midSizes[1];
    }
    layout[PetuniaLayoutStore::kSections] = m_properties->sectionStates();
    m_layoutStore.save(m_workspace, layout);
}

void PetuniaShellWindow::scheduleLayoutSave() {
    if (!m_applyingLayout)
        m_layoutTimer.start();
}

void PetuniaShellWindow::showEvent(QShowEvent *event) {
    QMainWindow::showEvent(event);
    if (m_firstShow) {
        m_firstShow = false;
        // Os splitters só conhecem seu tamanho real depois do primeiro show.
        QTimer::singleShot(0, this, [this] {
            applyLayout(m_workspace);
            arrangeSurface(m_workspace);
        });
    }
}

void PetuniaShellWindow::resizeEvent(QResizeEvent *event) {
    QMainWindow::resizeEvent(event);
    repositionOverlays();
}

void PetuniaShellWindow::repositionOverlays() {
    if (!m_viewArea)
        return;
    const auto &theme = PetuniaTheme::instance();
    const int margin = theme.space(3);
    if (m_hud && m_hud->isVisible()) {
        m_hud->adjustSize();
        m_hud->move(margin, margin);
        m_hud->raise();
    }
    if (m_context && m_context->isVisible()) {
        m_context->setCompact(m_viewArea->width() - 2 * margin < m_context->fullWidth());
        m_context->adjustSize();
        const int w = m_context->width();
        const int x = std::clamp((m_viewArea->width() - w) / 2, margin, std::max(margin, m_viewArea->width() - w - margin));
        const int y = std::max(margin, m_viewArea->height() - m_context->height() - margin);
        m_context->move(x, y);
        m_context->raise();
    }
}

// ── Ações locais e diálogos ─────────────────────────────────────────────────

void PetuniaShellWindow::triggerLocalAction(const QString &id) {
    if (id == QLatin1String("file.new"))
        newProject();
    else if (id == QLatin1String("file.new_empty")) {
        if (confirmDiscard())
            m_controller.request(QStringLiteral("project.new_empty"));
    } else if (id == QLatin1String("file.open"))
        openProject();
    else if (id == QLatin1String("file.save"))
        saveProject();
    else if (id == QLatin1String("file.save_as"))
        saveProjectAs();
    else if (id == QLatin1String("file.export_glb"))
        exportTo(QStringLiteral("glb"));
    else if (id == QLatin1String("file.export_obj"))
        exportTo(QStringLiteral("obj"));
    else if (id == QLatin1String("app.palette"))
        openPalette();
    else if (id == QLatin1String("app.preferences"))
        openPreferences();
    else if (id == QLatin1String("app.references"))
        openReferences();
    else if (id == QLatin1String("app.assets"))
        openAssets();
    else if (id == QLatin1String("app.home"))
        openHome();
    else if (id == QLatin1String("app.recovery"))
        openRecovery();
    else if (id == QLatin1String("app.about"))
        showAbout();
    else if (id == QLatin1String("panel.rail_labels"))
        m_rail->setExpanded(!m_rail->expanded());
    else if (id == QLatin1String("panel.right")) {
        m_right->setVisible(!m_right->isVisibleTo(this));
        scheduleLayoutSave();
    } else if (id == QLatin1String("panel.drawer"))
        m_drawer->setOpen(!m_drawer->isOpen());
    else if (id == QLatin1String("layout.reset")) {
        m_layoutStore.reset(m_workspace);
        applyLayout(m_workspace);
    } else if (id == QLatin1String("layout.reset_all")) {
        m_layoutStore.resetAll();
        applyLayout(m_workspace);
    }
}

bool PetuniaShellWindow::confirmDiscard() {
    if (m_controller.state().value(QStringLiteral("saved")).toBool(true))
        return true;
    const auto answer = QMessageBox::question(this, tr("Alterações não salvas"),
                                              tr("Salvar as alterações antes de continuar?"),
                                              QMessageBox::Save | QMessageBox::Discard | QMessageBox::Cancel,
                                              QMessageBox::Save);
    if (answer == QMessageBox::Cancel)
        return false;
    if (answer == QMessageBox::Discard)
        return true;
    return saveProject();
}

void PetuniaShellWindow::newProject() {
    if (confirmDiscard())
        m_controller.request(QStringLiteral("project.new"));
}

void PetuniaShellWindow::openProject() {
    if (!confirmDiscard())
        return;
    const QString path = QFileDialog::getOpenFileName(
        this, tr("Abrir projeto Petunia3D"), QString(), tr("Projetos Petunia (*.petunia *.json);;Todos os arquivos (*)"));
    if (!path.isEmpty())
        m_controller.request(QStringLiteral("project.open"), {{"path", path}});
}

bool PetuniaShellWindow::saveProject() {
    if (m_projectPath.isEmpty())
        return saveProjectAs();
    const auto result = m_controller.request(QStringLiteral("project.save"), {{"path", m_projectPath}});
    return result.value(QStringLiteral("ok")).toBool(true);
}

bool PetuniaShellWindow::saveProjectAs() {
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Salvar projeto Petunia3D"), m_projectPath.isEmpty() ? QStringLiteral("projeto.petunia") : m_projectPath,
        tr("Projetos Petunia (*.petunia);;Todos os arquivos (*)"));
    if (path.isEmpty())
        return false;
    const auto result = m_controller.request(QStringLiteral("project.save"), {{"path", path}});
    return result.value(QStringLiteral("ok")).toBool(true);
}

void PetuniaShellWindow::exportTo(const QString &kind) {
    const bool glb = kind == QLatin1String("glb");
    const QString path = QFileDialog::getSaveFileName(
        this, glb ? tr("Exportar glTF binário") : tr("Exportar Wavefront OBJ"),
        glb ? QStringLiteral("modelo.glb") : QStringLiteral("modelo.obj"),
        glb ? tr("glTF binário (*.glb)") : tr("Wavefront OBJ (*.obj);;Todos os arquivos (*)"));
    if (!path.isEmpty())
        m_controller.request(glb ? QStringLiteral("export_glb") : QStringLiteral("export_obj"), {{"path", path}});
}

void PetuniaShellWindow::openPalette() {
    if (!m_palette) {
        m_palette = new PetuniaCommandPalette(this);
        connect(m_palette, &PetuniaCommandPalette::commandSelected, this,
                [this](const QString &id) { m_controller.command(id); });
    }
    const QPoint center = mapToGlobal(rect().center()) - QPoint(m_palette->width() / 2, m_palette->height() / 2);
    m_palette->move(center);
    m_palette->show();
}

void PetuniaShellWindow::openPreferences() {
    const QPointer<QWidget> invoker = QApplication::focusWidget();
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Preferências"));
    dialog.resize(680, 680);
    dialog.setMinimumSize(360, 360);
    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new PetuniaEditorPanel(QStringLiteral("SETTINGS"), &dialog);
    panel->setTransport([this](const QString &a, const QJsonObject &args) { return m_controller.request(a, args); });
    layout->addWidget(panel);
    auto *close = new QPushButton(tr("Fechar"), &dialog);
    layout->addWidget(close);
    connect(close, &QPushButton::clicked, &dialog, &QDialog::accept);
    petuniaLocalize(&dialog);
    dialog.exec();
    if (invoker && invoker->isVisible())
        invoker->setFocus();
    m_controller.request(QStringLiteral("preferences_save"));
}

void PetuniaShellWindow::openReferences() {
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Referências"));
    dialog.resize(640, 600);
    dialog.setMinimumSize(360, 360);
    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new PetuniaEditorPanel(QStringLiteral("REFERENCES"), &dialog);
    panel->setTransport([this](const QString &a, const QJsonObject &args) { return m_controller.request(a, args); });
    layout->addWidget(panel);
    petuniaLocalize(&dialog);
    dialog.exec();
}

void PetuniaShellWindow::openAssets() {
    m_drawer->setCurrentPage(QStringLiteral("assets"));
    m_drawer->setOpen(true);
}

void PetuniaShellWindow::openHome() {
    const QPointer<QWidget> invoker = QApplication::focusWidget();
    PetuniaHomeDialog dialog(this);
    QStringList names, paths;
    for (const auto &value : m_controller.state()["recent_projects"].toArray()) {
        const auto pair = value.toArray();
        names << pair[0].toString();
        paths << pair[1].toString();
    }
    dialog.setRecentProjects(names, paths);
    dialog.setRecoveryAvailable(m_controller.flag(QStringLiteral("recovery_available")));
    dialog.setShowOnStart(m_controller.flag(QStringLiteral("show_home_on_start")));
    connect(&dialog, &PetuniaHomeDialog::newProjectRequested, this, &PetuniaShellWindow::newProject);
    connect(&dialog, &PetuniaHomeDialog::openProjectRequested, this, &PetuniaShellWindow::openProject);
    connect(&dialog, &PetuniaHomeDialog::recoverProjectRequested, this, &PetuniaShellWindow::openRecovery);
    connect(&dialog, &PetuniaHomeDialog::settingsRequested, this, &PetuniaShellWindow::openPreferences);
    connect(&dialog, &PetuniaHomeDialog::recentProjectSelected, this, [this](const QString &path) {
        if (confirmDiscard())
            m_controller.request(QStringLiteral("project.open"), {{"path", path}});
    });
    petuniaLocalize(&dialog);
    dialog.exec();
    if (invoker && invoker->isVisible())
        invoker->setFocus();
    m_controller.request(QStringLiteral("set_show_home_on_start"), {{"enabled", dialog.showOnStart()}});
    m_controller.request(QStringLiteral("preferences_save"));
}

void PetuniaShellWindow::openRecovery() {
    if (!m_controller.flag(QStringLiteral("recovery_available"))) {
        QMessageBox::information(this, tr("Recuperação"), tr("Nenhum snapshot de recuperação disponível."));
        return;
    }
    PetuniaRecoveryDialog dialog(this);
    const auto info = m_controller.state()["recovery"].toObject();
    dialog.setSessionDetails(tr("Sessão recuperável"), info["name"].toString(), info["path"].toString());
    connect(&dialog, &PetuniaRecoveryDialog::recoverRequested, this, [this] {
        if (confirmDiscard())
            m_controller.request(QStringLiteral("recover_pending"));
    });
    connect(&dialog, &PetuniaRecoveryDialog::discardRequested, this,
            [this] { m_controller.request(QStringLiteral("discard_pending_recovery")); });
    connect(&dialog, &PetuniaRecoveryDialog::keepRequested, this,
            [this] { m_controller.request(QStringLiteral("keep_saved_project")); });
    petuniaLocalize(&dialog);
    dialog.exec();
}

void PetuniaShellWindow::showAbout() {
    QMessageBox::about(this, tr("Sobre o Petunia3D"),
                       tr("Petunia3D — modelagem 3D acessível e de baixo consumo.\nInterface Qt Widgets (Shell v2)."));
}

void PetuniaShellWindow::openHostDialogs(const QJsonObject &state) {
    const bool palette = state["command_search_visible"].toBool();
    const bool settings = state["settings_visible"].toBool();
    const bool references = state["reference_manager_open"].toBool();
    const bool assets = state["asset_library_visible"].toBool();
    if (!m_interactive || !(palette || settings || references || assets) || m_hostDialogPending)
        return;
    m_hostDialogPending = true;
    // Consumido uma vez; os diálogos abrem pelo laço de eventos, nunca de dentro do sync.
    petuniaRequest(QStringLiteral("consume_host_requests"));
    QTimer::singleShot(0, this, [this, palette, settings, references, assets] {
        if (palette)
            openPalette();
        else if (settings)
            openPreferences();
        else if (references)
            openReferences();
        else if (assets)
            openAssets();
        m_hostDialogPending = false;
    });
}

void PetuniaShellWindow::closeEvent(QCloseEvent *event) {
    if (!confirmDiscard()) {
        event->ignore();
        return;
    }
    captureLayout();
    QSettings settings(QStringLiteral("Petunia3D"), QStringLiteral("QtShell"));
    settings.setValue(QStringLiteral("geometry"), saveGeometry());
    m_controller.request(QStringLiteral("preferences_save"));
    event->accept();
}

// ── Teclado ─────────────────────────────────────────────────────────────────

bool PetuniaShellWindow::handleShortcut(QKeyEvent *key) {
    const auto map = m_controller.state()["shortcut_map"].toObject();
    const QKeySequence pressed(key->keyCombination());
    auto matches = [&](const char *action) {
        const QString text = map.value(QLatin1String(action)).toString();
        return !text.isEmpty() && QKeySequence(text, QKeySequence::PortableText) == pressed;
    };
    if (matches("global.save_project") || pressed == QKeySequence(QKeySequence::Save)) {
        saveProject();
        return true;
    }
    if (matches("global.open_project") || pressed == QKeySequence(QKeySequence::Open)) {
        openProject();
        return true;
    }
    if (pressed == QKeySequence(Qt::CTRL | Qt::Key_K) || pressed == QKeySequence(Qt::Key_F3) ||
        matches("global.command_palette") || pressed == QKeySequence(Qt::CTRL | Qt::Key_P)) {
        openPalette();
        return true;
    }
    return false;
}

bool PetuniaShellWindow::eventFilter(QObject *watched, QEvent *event) {
    if (watched == m_viewArea && event->type() == QEvent::Resize) {
        repositionOverlays();
        return false;
    }
    if (event->type() != QEvent::KeyPress)
        return QMainWindow::eventFilter(watched, event);

    auto *target = qobject_cast<QWidget *>(watched);
    const bool ours = target && (target == this || isAncestorOf(target));
    if (!ours || qApp->activeModalWidget() || qApp->activePopupWidget())
        return QMainWindow::eventFilter(watched, event);
    auto *key = static_cast<QKeyEvent *>(event);

    if (key->key() == Qt::Key_F6) {
        m_navigator->cycle(key->modifiers() & Qt::ShiftModifier ? -1 : 1, QApplication::focusWidget());
        return true;
    }
    // Esc e Home pertencem à vista/canvas (cancelar operação, enquadrar).
    if ((qobject_cast<PetuniaEditorCanvas *>(target) && (key->key() == Qt::Key_Escape || key->key() == Qt::Key_Home)) ||
        (qobject_cast<PetuniaGlWidget *>(target) && key->key() == Qt::Key_Escape))
        return QMainWindow::eventFilter(watched, event);

    // O canvas de nós é dono das suas teclas (setas, Enter, Delete, Shift+A): sem isso
    // Delete apagaria o objeto 3D. Acordes Ctrl (desfazer, salvar…) seguem globais,
    // exceto Ctrl+setas, que movem o nó.
    if (qobject_cast<PetuniaNodeCanvas *>(target)) {
        const bool arrow = key->key() >= Qt::Key_Left && key->key() <= Qt::Key_Down;
        if (!(key->modifiers() & Qt::ControlModifier) || arrow)
            return QMainWindow::eventFilter(watched, event);
    }

    const bool editable = qobject_cast<QLineEdit *>(target) || qobject_cast<QAbstractSpinBox *>(target) ||
                          qobject_cast<QTextEdit *>(target) || qobject_cast<QPlainTextEdit *>(target);
    const bool fileChord = key->modifiers() & Qt::ControlModifier;
    if (editable && !fileChord)
        return QMainWindow::eventFilter(watched, event);
    if (handleShortcut(key))
        return true;
    if (editable)
        return QMainWindow::eventFilter(watched, event);

    const QString text = petuniaKeyText(key);
    if (text.isEmpty())
        return QMainWindow::eventFilter(watched, event);
    const bool secondary = target == m_secondary;
    if (secondary)
        petunia_qt_view_activate(2);
    const auto reply = petuniaRequest(QStringLiteral("shortcut"),
                                      {{"text", text},
                                       {"ctrl", bool(key->modifiers() & Qt::ControlModifier)},
                                       {"shift", bool(key->modifiers() & Qt::ShiftModifier)},
                                       {"alt", bool(key->modifiers() & Qt::AltModifier)}});
    if (secondary)
        petunia_qt_view_activate(1);
    if (reply.value(QStringLiteral("changed")).toBool()) {
        m_controller.refresh();
        return true;
    }
    return QMainWindow::eventFilter(watched, event);
}
