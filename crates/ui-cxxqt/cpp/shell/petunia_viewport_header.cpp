#include "petunia_viewport_header.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"
#include "petunia_segmented.h"

#include <QActionGroup>
#include <QFrame>
#include <QHBoxLayout>
#include <QMenu>
#include <QResizeEvent>
#include <QSignalBlocker>

namespace {
constexpr int kCompactBelow = 760;
}

PetuniaViewportHeader::PetuniaViewportHeader(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaViewportHeader"));
    setAccessibleName(tr("Controles da vista"));
    m_layout = new QHBoxLayout(this);

    m_domain = new PetuniaSegmented(this);
    m_domain->setItems({{"object", {}, tr("Objeto"), QStringLiteral("mode_object")},
                        {"vertex", {}, tr("Vértice"), QStringLiteral("select_vertex")},
                        {"edge", {}, tr("Aresta"), QStringLiteral("select_edge")},
                        {"face", {}, tr("Face"), QStringLiteral("select_face")}});
    m_domain->setGroupLabel(tr("Domínio de seleção"));
    connect(m_domain, &PetuniaSegmented::activated, this, [this](const QString &id) {
        m_controller->request(QStringLiteral("domain"), {{"id", id}});
    });
    m_layout->addWidget(m_domain);

    m_domainSeparator = new QFrame(this);
    m_domainSeparator->setProperty("role", QStringLiteral("separator"));
    m_domainSeparator->setFixedWidth(1);
    m_layout->addWidget(m_domainSeparator);

    m_snap = makeToggle(QStringLiteral("snap_magnet"), tr("Snap"), tr("Ativar snap"));
    connect(m_snap, &QToolButton::clicked, this, [this] {
        if (!m_syncing)
            m_controller->request(QStringLiteral("toggle_snap_enabled"));
    });
    m_proportional = makeToggle(QStringLiteral("proportional_editing"), tr("Proporcional"), tr("Edição proporcional"));
    connect(m_proportional, &QToolButton::clicked, this, [this] {
        if (!m_syncing)
            m_controller->request(QStringLiteral("toggle_proportional_editing"));
    });
    m_layout->addWidget(m_snap);
    m_layout->addWidget(m_proportional);
    m_layout->addStretch(1);

    m_projection = new PetuniaSegmented(this);
    m_projection->setItems({{"persp", tr("Persp."), tr("Projeção em perspectiva"), QStringLiteral("view_perspective")},
                            {"ortho", tr("Orto"), tr("Projeção ortográfica"), QStringLiteral("view_orthographic")}});
    m_projection->setGroupLabel(tr("Projeção"));
    m_projection->button(QStringLiteral("persp"))->setProperty("fullText", tr("Persp."));
    m_projection->button(QStringLiteral("ortho"))->setProperty("fullText", tr("Orto"));
    connect(m_projection, &PetuniaSegmented::activated, this, [this](const QString &id) {
        const bool ortho = m_controller->flag(QStringLiteral("is_orthographic"));
        if ((id == QLatin1String("ortho")) != ortho)
            m_controller->command(QStringLiteral("view.toggle_projection"));
    });
    m_layout->addWidget(m_projection);

    m_display = makeMenuButton(QStringLiteral("shading_solid"), tr("Exibição"), tr("Modo de exibição"));
    m_overlays = makeMenuButton(QStringLiteral("overlays"), tr("Overlays"), tr("Overlays da vista"));
    m_camera = makeMenuButton(QStringLiteral("camera"), tr("Câmera"), tr("Câmera e navegação"));
    m_split = makeToggle(QStringLiteral("split_view"), tr("Dividir"), tr("Dividir a vista 3D"));
    connect(m_split, &QToolButton::clicked, this, [this] {
        if (!m_syncing)
            m_controller->request(QStringLiteral("toggle_split_view"));
    });
    m_layout->addWidget(m_display);
    m_layout->addWidget(m_overlays);
    m_layout->addWidget(m_camera);
    m_layout->addWidget(m_split);

    buildDisplayMenu();
    buildOverlaysMenu();
    buildCameraMenu();
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaViewportHeader::applyMetrics);
    applyMetrics();
}

QToolButton *PetuniaViewportHeader::makeMenuButton(const QString &icon, const QString &label, const QString &accessible) {
    auto *button = new QToolButton(this);
    button->setProperty("role", QStringLiteral("header"));
    button->setIcon(PetuniaIcons::get(icon));
    button->setText(label);
    button->setToolTip(accessible);
    button->setAccessibleName(accessible);
    button->setPopupMode(QToolButton::InstantPopup);
    button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    return button;
}

QToolButton *PetuniaViewportHeader::makeToggle(const QString &icon, const QString &label, const QString &tip) {
    auto *button = new QToolButton(this);
    button->setProperty("role", QStringLiteral("header"));
    button->setCheckable(true);
    button->setIcon(PetuniaIcons::get(icon));
    button->setText(label);
    button->setToolTip(tip);
    button->setAccessibleName(tip);
    button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    return button;
}

QString PetuniaViewportHeader::shortcutText(const QString &action) const {
    const auto shortcut = m_controller->shortcut(action);
    return shortcut.isEmpty() ? QString() : QLatin1Char('\t') + shortcut;
}

void PetuniaViewportHeader::buildDisplayMenu() {
    auto *menu = new QMenu(m_display);
    auto *group = new QActionGroup(menu);
    struct Mode { const char *id; const char *label; };
    for (const auto &mode : {Mode{"solid", "Sólido"}, Mode{"wireframe", "Wireframe"},
                             Mode{"material", "Prévia de material"}, Mode{"rendered", "Renderizado"}}) {
        auto *action = menu->addAction(tr(mode.label));
        action->setCheckable(true);
        group->addAction(action);
        const QString id = QLatin1String(mode.id);
        connect(action, &QAction::triggered, this, [this, id] {
            m_controller->request(QStringLiteral("set_shading_mode"), {{"id", id}});
        });
        m_shadingActions.insert(id, action);
    }
    m_display->setMenu(menu);
}

void PetuniaViewportHeader::buildOverlaysMenu() {
    auto *menu = new QMenu(m_overlays);
    struct Overlay { const char *id; const char *label; const char *stateKey; const char *command; const char *request; };
    const Overlay overlays[] = {
        {"xray", "Raio-X", "show_xray", "view.toggle_xray", nullptr},
        {"wire", "Wireframe sobre o sólido", "show_wire_overlay", "view.toggle_wire_overlay", nullptr},
        {"orientation", "Orientação das faces", "show_face_orientation", nullptr, "toggle_face_orientation"},
        {"checker", "Xadrez de UV", "show_uv_checker", nullptr, "toggle_uv_checker"},
    };
    for (const auto &overlay : overlays) {
        auto *action = menu->addAction(tr(overlay.label));
        action->setCheckable(true);
        action->setProperty("stateKey", QLatin1String(overlay.stateKey));
        const QString command = overlay.command ? QLatin1String(overlay.command) : QString();
        const QString request = overlay.request ? QLatin1String(overlay.request) : QString();
        connect(action, &QAction::triggered, this, [this, command, request] {
            if (m_syncing)
                return;
            if (!command.isEmpty())
                m_controller->command(command);
            else
                m_controller->request(request);
        });
        m_overlayActions.insert(QLatin1String(overlay.id), action);
    }
    m_overlays->setMenu(menu);
}

void PetuniaViewportHeader::buildCameraMenu() {
    auto *menu = new QMenu(m_camera);
    struct View { const char *label; const char *command; const char *keymap; };
    const View views[] = {
        {"Frente", "view.front", "view.camera_front"}, {"Trás", "view.back", "view.camera_back"},
        {"Direita", "view.right", "view.camera_right"}, {"Esquerda", "view.left", "view.camera_left"},
        {"Topo", "view.top", "view.camera_top"}, {"Base", "view.bottom", "view.camera_bottom"},
        {"Isométrica", "view.isometric_ne", "view.camera_iso_2_1"},
    };
    for (const auto &view : views) {
        auto *action = menu->addAction(tr(view.label));
        action->setProperty("keymapAction", QLatin1String(view.keymap));
        const QString command = QLatin1String(view.command);
        connect(action, &QAction::triggered, this, [this, command] { m_controller->command(command); });
    }
    menu->addSeparator();
    struct Nav { const char *label; const char *command; const char *keymap; };
    const Nav navs[] = {
        {"Enquadrar seleção", "view.frame_selection", "view.frame_selection"},
        {"Enquadrar tudo", "view.frame_all", "view.frame_all"},
        {"Restaurar câmera", "view.reset_camera", "view.reset_camera"},
    };
    for (const auto &nav : navs) {
        auto *action = menu->addAction(tr(nav.label));
        action->setProperty("keymapAction", QLatin1String(nav.keymap));
        const QString command = QLatin1String(nav.command);
        connect(action, &QAction::triggered, this, [this, command] { m_controller->command(command); });
    }
    m_camera->setMenu(menu);
}

void PetuniaViewportHeader::applyMetrics() {
    const auto &theme = PetuniaTheme::instance();
    m_layout->setContentsMargins(theme.space(2), theme.space(1), theme.space(2), theme.space(1));
    m_layout->setSpacing(theme.space(1));
    const QSize icon(theme.iconPx(), theme.iconPx());
    for (auto *button : {m_snap, m_proportional, m_display, m_overlays, m_camera, m_split}) {
        button->setIconSize(icon);
        button->setMinimumHeight(theme.controlHeight());
    }
    for (auto *segmented : {m_domain, m_projection})
        for (auto *button : segmented->buttons()) {
            button->setIconSize(icon);
            button->setMinimumHeight(theme.controlHeight());
            button->setMinimumWidth(theme.controlHeight());
        }
    for (auto *button : {m_snap, m_proportional, m_display, m_overlays, m_camera, m_split})
        button->setMinimumWidth(theme.controlHeight());
    m_domainSeparator->setFixedHeight(theme.controlHeight() - theme.space(2));
    setMinimumHeight(theme.controlHeight() + theme.space(2));
    setMaximumHeight(theme.controlHeight() + theme.space(2));
}

void PetuniaViewportHeader::applyCompact(bool compact) {
    if (m_compact == compact)
        return;
    m_compact = compact;
    const auto style = compact ? Qt::ToolButtonIconOnly : Qt::ToolButtonTextBesideIcon;
    for (auto *button : {m_snap, m_proportional, m_display, m_overlays, m_camera, m_split})
        button->setToolButtonStyle(style);
    for (auto *button : m_projection->buttons())
        button->setText(compact ? QString() : button->property("fullText").toString());
}

void PetuniaViewportHeader::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);
    applyCompact(event->size().width() < kCompactBelow);
}

void PetuniaViewportHeader::sync(const QJsonObject &state) {
    m_syncing = true;
    const bool modeling = state["workspace"].toString() == QLatin1String("Model");
    const bool paint = state["workspace"].toString() == QLatin1String("Paint");
    const bool uv = state["workspace"].toString() == QLatin1String("Uv");

    // O domínio de seleção só existe onde há componentes: modelagem e pintura (objeto/face).
    m_domain->setVisible(modeling || paint);
    m_domainSeparator->setVisible(modeling || paint);
    m_domain->setItemVisible(QStringLiteral("vertex"), modeling);
    m_domain->setItemVisible(QStringLiteral("edge"), modeling);
    m_domain->setCurrent(state["selection_domain"].toString().toLower());
    for (const auto &id : {QStringLiteral("object"), QStringLiteral("vertex"), QStringLiteral("edge"), QStringLiteral("face")}) {
        const auto shortcut = m_controller->shortcut(QStringLiteral("model.select_") + id);
        if (auto *button = m_domain->button(id)) {
            const QString base = button->accessibleName();
            const QString tip = shortcut.isEmpty() ? base : base + QStringLiteral("  (%1)").arg(shortcut);
            if (button->toolTip() != tip)
                button->setToolTip(tip);
        }
    }

    m_snap->setVisible(modeling);
    m_proportional->setVisible(modeling);
    {
        const QSignalBlocker a(m_snap), b(m_proportional), c(m_split);
        m_snap->setChecked(state["snap_enabled"].toBool());
        m_proportional->setChecked(state["proportional_editing"].toBool());
        m_split->setChecked(state["split_enabled"].toBool());
    }

    const bool ortho = state["is_orthographic"].toBool();
    m_projection->setCurrent(ortho ? QStringLiteral("ortho") : QStringLiteral("persp"));
    // DRAW trabalha num plano ortográfico guiado pelo workplane.
    m_projection->setItemEnabled(QStringLiteral("persp"), state["modeling_mode"].toString() != QLatin1String("DRAW") || !modeling);

    const QString shading = state["shading_mode"].toString(QStringLiteral("solid"));
    for (auto it = m_shadingActions.cbegin(); it != m_shadingActions.cend(); ++it) {
        const QSignalBlocker block(it.value());
        it.value()->setChecked(it.key() == shading);
    }
    const QString shadingIcon = shading == QLatin1String("wireframe") ? QStringLiteral("shading_wireframe")
                                : shading == QLatin1String("rendered") ? QStringLiteral("shading_rendered")
                                : shading == QLatin1String("material") ? QStringLiteral("shading_material")
                                                                       : QStringLiteral("shading_solid");
    if (m_display->property("icon_id").toString() != shadingIcon) {
        m_display->setProperty("icon_id", shadingIcon);
        m_display->setIcon(PetuniaIcons::get(shadingIcon));
    }
    for (auto *action : m_overlayActions) {
        const QSignalBlocker block(action);
        action->setChecked(state[action->property("stateKey").toString()].toBool());
    }
    for (auto *action : m_camera->menu()->actions()) {
        const auto keymap = action->property("keymapAction").toString();
        if (keymap.isEmpty())
            continue;
        if (!action->property("baseText").isValid())
            action->setProperty("baseText", action->text());
        const QString text = action->property("baseText").toString() + shortcutText(keymap);
        if (action->text() != text)
            action->setText(text);
    }
    m_overlays->setVisible(true);
    m_split->setVisible(modeling || uv || paint);
    m_syncing = false;
}
