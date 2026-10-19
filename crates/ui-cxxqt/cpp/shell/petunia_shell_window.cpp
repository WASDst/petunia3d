#include "petunia_shell_window.h"

#include "../petunia_dialog_chrome.h"
#include "petunia_pie_menu.h"

#include "../petunia_command_palette.h"
#include "../petunia_editor_panel.h"
#include "../petunia_gl_widget.h"
#include "../petunia_home_dialog.h"
#include "../petunia_icons.h"
#include "../petunia_recovery_dialog.h"
#include "../petunia_theme.h"
#include "petunia_align_popover.h"
#include "petunia_app_bar.h"
#include "petunia_context_actions.h"
#include "petunia_drawer.h"
#include "petunia_i18n.h"
#include "petunia_node_canvas.h"
#include "petunia_node_page.h"
#include "petunia_operation_hud.h"
#include "petunia_properties_panel.h"
#include "petunia_region_navigator.h"
#include "petunia_segmented.h"
#include "petunia_status_bar.h"
#include "petunia_structure_panel.h"
#include "petunia_tool_rail.h"
#include "petunia_viewport_header.h"
#include "petunia_workspace_sections.h"

#include <QAbstractItemView>
#include <QAbstractSpinBox>
#include <QApplication>
#include <QCloseEvent>
#include <QContextMenuEvent>
#include <QComboBox>
#include <QCursor>
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
    if (m_nodePage && !m_nodePage->parentWidget())
        delete m_nodePage;
    if (m_assetsHost && !m_assetsHost->parentWidget())
        delete m_assetsHost;
    if (m_timelinePage && !m_timelinePage->parentWidget())
        delete m_timelinePage;
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
    connect(m_context, &PetuniaContextActions::uiActionRequested, this,
            [this](const QString &target, QWidget *anchor) {
                if (target == QLatin1String("ui.align"))
                    openAlign(anchor);
            });
    connect(m_viewport, &PetuniaGlWidget::uiActionRequested, this, [this](const QString &target) {
        if (target == QLatin1String("ui.align"))
            openAlign(nullptr);
    });
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
    updateRegions();

    connect(m_appBar, &PetuniaAppBar::localAction, this, &PetuniaShellWindow::triggerLocalAction);
    connect(m_structure, &PetuniaStructurePanel::createRequested, this,
            [this] { m_controller.command(QStringLiteral("model.add_cube")); });
    connect(m_structure, &PetuniaStructurePanel::alignRequested, this, [this](QWidget *anchor) { openAlign(anchor); });
    connect(m_rail, &PetuniaToolRail::expandedChanged, this, [this](bool) { scheduleLayoutSave(); });
    connect(m_properties, &PetuniaPropertiesPanel::sectionToggled, this, [this](const QString &, bool) { scheduleLayoutSave(); });
    connect(m_properties, &PetuniaPropertiesPanel::materialNodesRequested, this, [this] { openNodeEditor(true); });
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
    for (auto *splitter : {m_body, m_middle, m_right, m_surface})
        connect(splitter, &QSplitter::splitterMoved, this, [this] { scheduleLayoutSave(); });
    connect(m_header, &PetuniaViewportHeader::surfaceModeRequested, this, [this](const QString &mode) {
        auto layout = m_layoutStore.load(m_workspace);
        applySurfaceMode(mode, layout[PetuniaLayoutStore::kSurfaceRatio].toDouble());
        scheduleLayoutSave();
    });
}

int PetuniaShellWindow::focusedRegion() const {
    return m_navigator ? m_navigator->regionOf(QApplication::focusWidget()) : -1;
}

namespace {
/// Memória do invocador (U07): guarda o widget com foco ao abrir um diálogo e o
/// devolve ao fechar, se ainda estiver visível.
struct PetuniaFocusReturn {
    QPointer<QWidget> invoker{QApplication::focusWidget()};
    ~PetuniaFocusReturn() {
        if (!invoker || !invoker->isVisible())
            return;
        // A janela do invocador volta a ser a ativa: sem isso o foco fica só
        // "guardado" nela e o teclado (F6, atalhos) não chega a lugar nenhum.
        invoker->window()->activateWindow();
        invoker->setFocus(Qt::OtherFocusReason);
    }
};
} // namespace

PetuniaNodeCanvas *PetuniaShellWindow::nodeCanvas() const {
    return m_nodePage ? m_nodePage->canvas() : nullptr;
}

void PetuniaShellWindow::openAlign(QWidget *anchor) {
    if (!m_align)
        m_align = new PetuniaAlignPopover(&m_controller, this);
    m_align->popupAt(anchor ? anchor : static_cast<QWidget *>(m_viewArea));
}

void PetuniaShellWindow::openNodeEditor(bool material) {
    if (!m_nodePage)
        return;
    m_nodePage->setDomain(material ? PetuniaNodeCanvas::Domain::Material : PetuniaNodeCanvas::Domain::Operations);
    m_drawer->setCurrentPage(QStringLiteral("nodes"));
    m_drawer->setOpen(true);
    m_nodePage->canvas()->setFocus(Qt::OtherFocusReason);
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

    if (m_nodePage)
        m_nodePage->sync(state);
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
    if (workspace == QLatin1String("ANIMATE") && m_timelinePage) {
        m_timelineCanvas->sync(state);
        m_timelinePage->sync(state);
    } else if (auto *canvas = m_canvases.value(workspace)) {
        canvas->sync(state);
    }

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
    if (m_timelinePage)
        m_timelinePage->setParent(nullptr);
    if (m_assetsHost)
        m_assetsHost->setParent(nullptr);
    if (m_nodePage)
        m_nodePage->setParent(nullptr);
    auto *old = takeCentralWidget();
    delete old;
    qDeleteAll(m_canvases);
    delete m_timelinePage; // leva o dope sheet junto
    m_timelinePage = nullptr;
    m_timelineCanvas = nullptr;
    delete m_assetsHost;
    delete m_nodePage;
    m_nodePage = nullptr;
    m_canvases.clear();
    m_assetsHost = nullptr;
    m_assetsPanel = nullptr;
    m_workspace.clear();
    delete m_align; // volta a nascer, já traduzido, no próximo uso
    m_align = nullptr;
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
        if (m_timelinePage)
            m_drawer->addPage(QStringLiteral("timeline"), tr("Linha do tempo"), QStringLiteral("timeline"), m_timelinePage);
    } else {
        if (!m_assetsHost) {
            m_assetsHost = new QWidget(this);
            auto *layout = new QVBoxLayout(m_assetsHost);
            layout->setContentsMargins(0, 0, 0, 0);
        }
        m_drawer->addPage(QStringLiteral("assets"), tr("Assets"), QStringLiteral("asset_library"), m_assetsHost);
        if (m_controller.isModeling()) {
            if (!m_nodePage)
                m_nodePage = new PetuniaNodePage(&m_controller, this);
            m_drawer->addPage(QStringLiteral("nodes"), tr("Nós"), QStringLiteral("link"), m_nodePage);
        }
    }
    applyLayout(workspace);
}

QWidget *PetuniaShellWindow::workspaceCanvas(const QString &workspace) const {
    if (workspace == QLatin1String("ANIMATE"))
        return m_timelineCanvas;
    return m_canvases.value(workspace);
}

/// Superfícies 2D nativas do workspace: canvas de pintura e editor UV vivem na
/// área de trabalho; o dope sheet do ANIMATE fica na página da linha do tempo.
/// As propriedades desses workspaces são seções nativas (Propriedades), sem o
/// painel legado.
void PetuniaShellWindow::ensureSpecialized(const QString &workspace) {
    auto refreshOnEdit = [this](PetuniaEditorCanvas *canvas) {
        connect(canvas, &PetuniaEditorCanvas::editorChanged, this, [this] { m_controller.refresh(); });
    };
    if (workspace == QLatin1String("ANIMATE")) {
        if (m_timelinePage)
            return;
        m_timelineCanvas = new PetuniaEditorCanvas(workspace, this);
        refreshOnEdit(m_timelineCanvas);
        m_timelinePage = new PetuniaTimelinePage(&m_controller, m_timelineCanvas, this);
        m_timelinePage->hide();
        m_timelineCanvas->sync(m_controller.state());
        m_timelinePage->sync(m_controller.state());
        return;
    }
    if ((workspace != QLatin1String("PAINT") && workspace != QLatin1String("UV")) || m_canvases.contains(workspace))
        return;
    auto *canvas = new PetuniaEditorCanvas(workspace, this);
    canvas->hide();
    refreshOnEdit(canvas);
    m_canvases.insert(workspace, canvas);
    canvas->sync(m_controller.state());
}

void PetuniaShellWindow::arrangeSurface(const QString &workspace) {
    for (const auto &id : {QStringLiteral("PAINT"), QStringLiteral("UV")})
        if (auto *canvas = m_canvases.value(id))
            if (canvas->parentWidget() == m_surface)
                canvas->hide();
    m_viewArea->show();
    if (workspace == QLatin1String("PAINT") || workspace == QLatin1String("UV")) {
        if (auto *canvas = m_canvases.value(workspace)) {
            // PAINT: canvas à direita da vista; UV: editor à esquerda (foco principal).
            m_surface->insertWidget(workspace == QLatin1String("UV") ? 0 : 1, canvas);
            canvas->show();
        }
    }
    updateRegions();
}

void PetuniaShellWindow::applySurfaceMode(const QString &mode, double ratio) {
    auto *canvas = m_canvases.value(m_workspace);
    if (!canvas) {
        m_viewArea->show();
        return;
    }
    canvas->setVisible(mode != QLatin1String("3d"));
    m_viewArea->setVisible(mode != QLatin1String("2d"));
    if (mode == QLatin1String("split")) {
        const int total = std::max(m_surface->width(), 800);
        const int view = int(total * std::clamp(ratio, 0.15, 0.85));
        // A ordem dos widgets no splitter muda por workspace (UV: editor primeiro).
        QList<int> sizes;
        for (int i = 0; i < m_surface->count(); ++i) {
            auto *w = m_surface->widget(i);
            sizes << (w == canvas ? total - view : w == m_viewArea ? view : (w->isVisible() ? w->width() : 0));
        }
        m_surface->setSizes(sizes);
    }
    m_header->setSurfaceMode(mode);
    updateRegions();
}

void PetuniaShellWindow::updateRegions() {
    // F6 percorre também a superfície 2D quando ela está visível.
    QList<QWidget *> regions{m_appBar, m_rail};
    if (m_viewArea->isVisible())
        regions << m_viewport;
    if (auto *canvas = m_canvases.value(m_workspace); canvas && canvas->isVisible())
        regions << canvas;
    regions << m_structure << m_properties << m_drawer << m_context;
    m_navigator->setRegions(regions);
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
    if (workspace == QLatin1String("PAINT") || workspace == QLatin1String("UV"))
        applySurfaceMode(layout[PetuniaLayoutStore::kSurfaceMode].toString(),
                         layout[PetuniaLayoutStore::kSurfaceRatio].toDouble());

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
        // O QSplitter guarda os tamanhos antigos: sem isto a altura da gaveta
        // aberta vira um vão vazio abaixo da barra em vez de voltar à superfície.
        m_middle->setSizes({std::max(0, m_middle->height() - bar), bar});
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
        // Logo após abrir, a gaveta ainda tem a altura da barra recolhida: essa
        // medida não é a preferência do usuário e não pode sobrescrevê-la.
        if (midSizes.size() == 2 && midSizes[1] >= m_drawer->barHeight() + 80)
            layout[PetuniaLayoutStore::kDrawerHeight] = midSizes[1];
    }
    layout[PetuniaLayoutStore::kSections] = m_properties->sectionStates();
    if (m_workspace == QLatin1String("PAINT") || m_workspace == QLatin1String("UV")) {
        layout[PetuniaLayoutStore::kSurfaceMode] = m_header->surfaceMode()->current();
        auto *canvas = m_canvases.value(m_workspace);
        if (canvas && canvas->isVisible() && m_viewArea->isVisible()) {
            const int total = canvas->width() + m_viewArea->width();
            if (total > 0)
                layout[PetuniaLayoutStore::kSurfaceRatio] = double(m_viewArea->width()) / total;
        }
    }
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
        m_hud->move(m_hud->placement(m_viewArea->size(), margin));
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
        // Popup não-modal: devolve o foco ao invocador quando fecha.
        connect(m_palette, &QDialog::finished, this, [this] {
            if (m_paletteInvoker && m_paletteInvoker->isVisible()) {
                m_paletteInvoker->window()->activateWindow();
                m_paletteInvoker->setFocus(Qt::OtherFocusReason);
            }
        });
    }
    m_paletteInvoker = QApplication::focusWidget();
    const QPoint center = mapToGlobal(rect().center()) - QPoint(m_palette->width() / 2, m_palette->height() / 2);
    m_palette->move(center);
    m_palette->show();
}

void PetuniaShellWindow::openPreferences() {
    PetuniaFocusReturn focusReturn;
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
    PetuniaDialogChrome::install(&dialog, QStringLiteral("preferences"));
    petuniaLocalize(&dialog);
    dialog.exec();
    m_controller.request(QStringLiteral("preferences_save"));
}

void PetuniaShellWindow::openReferences() {
    PetuniaFocusReturn focusReturn;
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Referências"));
    dialog.resize(640, 600);
    dialog.setMinimumSize(360, 360);
    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new PetuniaEditorPanel(QStringLiteral("REFERENCES"), &dialog);
    panel->setTransport([this](const QString &a, const QJsonObject &args) { return m_controller.request(a, args); });
    layout->addWidget(panel);
    PetuniaDialogChrome::install(&dialog, QStringLiteral("references"));
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

bool PetuniaShellWindow::handlePieShortcut(QKeyEvent *key, QWidget *target) {
    if (!qobject_cast<PetuniaGlWidget *>(target) || key->isAutoRepeat())
        return false;
    const auto &state = m_controller.state();
    const auto map = state["shortcut_map"].toObject();
    const QKeySequence pressed(key->keyCombination());
    auto matches = [&](const char *action) {
        const QString text = map.value(QLatin1String(action)).toString();
        return !text.isEmpty() && QKeySequence(text, QKeySequence::PortableText) == pressed;
    };
    // Com uma operação em curso (G/R/S, primitiva, loop cut) a tecla é da operação.
    if (state["tool_modal_active"].toBool() || state["primitive_active"].toBool())
        return false;
    const bool model = state["workspace"].toString().compare(QLatin1String("Model"), Qt::CaseInsensitive) == 0;
    QString kind;
    if (model && matches("model.primitives"))
        kind = QStringLiteral("add");
    else if (matches("view.pie_view"))
        kind = QStringLiteral("view");
    else if (model && matches("model.pie_select_domain"))
        kind = QStringLiteral("select");
    if (kind.isEmpty())
        return false;
    openPie(kind, QCursor::pos());
    return true;
}

PetuniaPieMenu *PetuniaShellWindow::openPie(const QString &kind, const QPoint &globalPos) {
    QVector<PetuniaPieItem> items;
    QString title;
    auto command = [this](const QString &id) { return [this, id] { m_controller.command(id); }; };
    if (kind == QLatin1String("add")) {
        title = tr("Adicionar");
        struct P { const char *id; const char *label; const char *icon; };
        const P prims[] = {{"cube", "Cubo", "cube"},         {"sphere", "Esfera", "sphere"},
                           {"cylinder", "Cilindro", "cylinder"}, {"cone", "Cone", "cone"},
                           {"torus", "Toro", "torus"},       {"plane", "Plano", "plane"},
                           {"icosphere", "Icosfera", "icosphere"}, {"capsule", "Cápsula", "capsule"}};
        for (const auto &prim : prims)
            items.append({QLatin1String(prim.id), tr(prim.label), QLatin1String(prim.icon),
                          command(QStringLiteral("model.add_%1").arg(QLatin1String(prim.id)))});
    } else if (kind == QLatin1String("view")) {
        title = tr("Vista");
        struct V { const char *id; const char *label; const char *command; const char *icon; };
        const V views[] = {{"top", "Topo", "view.top", "view_top"},
                           {"right", "Direita", "view.right", "view_right"},
                           {"front", "Frente", "view.front", "view_front"},
                           {"iso", "Isométrica", "view.isometric_ne", "view_perspective"},
                           {"bottom", "Base", "view.bottom", "view_bottom"},
                           {"frame", "Enquadrar seleção", "view.frame_selection", "frame_selection"},
                           {"back", "Trás", "view.back", "view_back"},
                           {"left", "Esquerda", "view.left", "view_left"}};
        for (const auto &view : views)
            items.append({QLatin1String(view.id), tr(view.label), QLatin1String(view.icon),
                          command(QLatin1String(view.command))});
    } else if (kind == QLatin1String("select")) {
        title = tr("Selecionar");
        struct D { const char *id; const char *label; const char *icon; };
        const D domains[] = {{"object", "Objeto", "mode_object"}, {"vertex", "Vértice", "select_vertex"},
                             {"edge", "Aresta", "select_edge"},   {"face", "Face", "select_face"}};
        for (const auto &domain : domains) {
            const QString id = QLatin1String(domain.id);
            items.append({id, tr(domain.label), QLatin1String(domain.icon),
                          [this, id] { m_controller.request(QStringLiteral("domain"), {{"id", id}}); }});
        }
    } else {
        return nullptr;
    }
    auto *pie = new PetuniaPieMenu(title, this);
    pie->setItems(items);
    const QPointer<QWidget> invoker = QApplication::focusWidget();
    connect(pie, &PetuniaPieMenu::closed, this, [invoker] {
        if (invoker && invoker->isVisible())
            invoker->setFocus(Qt::PopupFocusReason);
    });
    pie->popup(globalPos);
    return pie;
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
    // Shift+F10 abre o menu de contexto do widget com foco (como a tecla Menu).
    if (key->key() == Qt::Key_F10 && (key->modifiers() & Qt::ShiftModifier)) {
        QWidget *focus = QApplication::focusWidget();
        if (!focus)
            return true;
        QWidget *receiver = focus;
        QPoint pos = focus->rect().center();
        if (auto *view = qobject_cast<QAbstractItemView *>(focus)) {
            receiver = view->viewport();
            const QRect item = view->visualRect(view->currentIndex());
            pos = item.isValid() ? item.center() : receiver->rect().center();
        }
        QContextMenuEvent menu(QContextMenuEvent::Keyboard, pos, receiver->mapToGlobal(pos));
        QCoreApplication::sendEvent(receiver, &menu);
        return true;
    }
    // Esc e Home pertencem à vista/canvas (cancelar operação, enquadrar).
    // Também +/−/setas no canvas 2D: zoom, pan e nudge UV antes dos atalhos globais.
    auto *canvas = qobject_cast<PetuniaEditorCanvas *>(target);
    if ((canvas && canvas->ownsKey(key)) ||
        (qobject_cast<PetuniaGlWidget *>(target) && key->key() == Qt::Key_Escape))
        return QMainWindow::eventFilter(watched, event);

    // O canvas de nós é dono das suas teclas (setas, Enter, Delete, Shift+A): sem isso
    // Delete apagaria o objeto 3D. Acordes Ctrl (desfazer, salvar…) seguem globais,
    // exceto Ctrl+setas, que movem o nó.
    if (qobject_cast<PetuniaNodeCanvas *>(target)) {
        const bool arrow = key->key() >= Qt::Key_Left && key->key() <= Qt::Key_Down;
        // Ctrl+G agrupa os NÓS (não os objetos da cena), Ctrl+A seleciona os nós, F2 renomeia
        // o grupo: o canvas declara essas teclas como suas (PetuniaNodeCanvas::ownsKey).
        if (!(key->modifiers() & Qt::ControlModifier) || arrow || PetuniaNodeCanvas::ownsKey(key))
            return QMainWindow::eventFilter(watched, event);
    }

    const bool editable = qobject_cast<QLineEdit *>(target) || qobject_cast<QAbstractSpinBox *>(target) ||
                          qobject_cast<QTextEdit *>(target) || qobject_cast<QPlainTextEdit *>(target);
    const bool fileChord = key->modifiers() & Qt::ControlModifier;
    if (editable && !fileChord)
        return QMainWindow::eventFilter(watched, event);
    if (!editable && handlePieShortcut(key, target))
        return true;
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
