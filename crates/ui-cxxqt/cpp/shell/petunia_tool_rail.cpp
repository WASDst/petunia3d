#include "petunia_tool_rail.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QEvent>
#include <QFrame>
#include <QJsonArray>
#include <QKeyEvent>
#include <QMenu>
#include <QScrollArea>
#include <QSignalBlocker>
#include <QVBoxLayout>

PetuniaToolRail::PetuniaToolRail(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaToolRail"));
    setAccessibleName(tr("Ferramentas"));
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    m_scroll = new QScrollArea(this);
    m_scroll->setFrameShape(QFrame::NoFrame);
    m_scroll->setWidgetResizable(true);
    m_scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_scroll->setFocusPolicy(Qt::NoFocus);
    m_column = new QWidget(m_scroll);
    m_column->setObjectName(QStringLiteral("PetuniaToolRailColumn"));
    m_layout = new QVBoxLayout(m_column);
    m_scroll->setWidget(m_column);
    root->addWidget(m_scroll, 1);

    m_expand = new QToolButton(this);
    m_expand->setProperty("role", QStringLiteral("rail"));
    m_expand->setCheckable(true);
    m_expand->setToolTip(tr("Mostrar nomes das ferramentas"));
    m_expand->setAccessibleName(tr("Mostrar nomes das ferramentas"));
    m_expand->installEventFilter(this);
    connect(m_expand, &QToolButton::toggled, this, [this](bool on) {
        setExpanded(on);
        emit expandedChanged(on);
    });
    root->addWidget(m_expand);

    m_group = new QButtonGroup(this);
    m_group->setExclusive(true);
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, [this] {
        m_signature.clear();
        applyMetrics();
        sync(m_controller->state());
    });
    applyMetrics();
}

void PetuniaToolRail::applyMetrics() {
    const auto &theme = PetuniaTheme::instance();
    const int pad = theme.space(1);
    m_layout->setContentsMargins(pad, theme.space(2), pad, theme.space(2));
    m_layout->setSpacing(pad);
    const int width = m_expanded ? theme.scaled(176) : theme.railWidth();
    setFixedWidth(width);
    const int icon = theme.iconPx();
    m_expand->setIcon(PetuniaIcons::get(m_expanded ? QStringLiteral("chevron_left") : QStringLiteral("chevron_right")));
    m_expand->setIconSize(QSize(icon, icon));
    m_expand->setMinimumHeight(theme.compactControlHeight());
    for (auto *button : m_buttons) {
        button->setIconSize(QSize(icon, icon));
        button->setMinimumHeight(theme.compactControlHeight());
        button->setMaximumHeight(theme.compactControlHeight());
        button->setToolButtonStyle(m_expanded ? Qt::ToolButtonTextBesideIcon : Qt::ToolButtonIconOnly);
        button->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    }
}

void PetuniaToolRail::setExpanded(bool expanded) {
    if (m_expanded == expanded)
        return;
    m_expanded = expanded;
    if (m_expand->isChecked() != expanded) {
        const QSignalBlocker block(m_expand);
        m_expand->setChecked(expanded);
    }
    applyMetrics();
}

QString PetuniaToolRail::tipFor(const QJsonObject &tool) const {
    QString tip = petuniaTr(tool["label"].toString());
    const QString shortcut = tool["shortcut"].toString();
    if (!shortcut.isEmpty())
        tip += QStringLiteral("  (%1)").arg(shortcut);
    return tip;
}

void PetuniaToolRail::rebuild(const QJsonArray &catalog) {
    qDeleteAll(m_buttons);
    m_buttons.clear();
    m_byId.clear();
    m_kinds.clear();
    while (auto *item = m_layout->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    const auto &theme = PetuniaTheme::instance();
    QString previousGroup;
    for (const auto &value : catalog) {
        const auto tool = value.toObject();
        const QString group = tool["group"].toString();
        if (!previousGroup.isEmpty() && group != previousGroup) {
            auto *separator = new QFrame(m_column);
            separator->setProperty("role", QStringLiteral("separator"));
            separator->setFixedHeight(1);
            m_layout->addWidget(separator);
        }
        previousGroup = group;

        auto *button = new QToolButton(m_column);
        button->setProperty("role", QStringLiteral("rail"));
        button->setProperty("toolId", tool["id"].toString());
        button->setCheckable(tool["kind"].toString() == QLatin1String("mode"));
        button->setIcon(PetuniaIcons::get(tool["icon"].toString()));
        button->setIconSize(QSize(theme.iconPx(), theme.iconPx()));
        button->setText(petuniaTr(tool["label"].toString()));
        button->setToolTip(tipFor(tool));
        button->setAccessibleName(petuniaTr(tool["label"].toString()));
        button->setAccessibleDescription(tool["shortcut"].toString());
        button->setAutoRaise(false);
        button->installEventFilter(this);
        if (button->isCheckable())
            m_group->addButton(button);
        m_layout->addWidget(button);
        m_buttons.append(button);
        const QString id = tool["id"].toString();
        m_byId.insert(id, button);
        m_kinds.insert(id, tool["kind"].toString());
        connect(button, &QToolButton::clicked, this, [this, id, button] {
            if (m_syncing)
                return;
            if (m_kinds.value(id) == QLatin1String("create_menu"))
                showCreateMenu(button, m_controller->state());
            else
                m_controller->request(QStringLiteral("tool"), {{"id", id}});
        });
    }
    m_layout->addStretch(1);
    applyMetrics();
    applyRoving();
}

void PetuniaToolRail::showCreateMenu(QToolButton *anchor, const QJsonObject &state) {
    QMenu menu(this);
    struct Entry { QString id; QString label; };
    QList<Entry> entries;
    for (const auto &value : state["commands"].toArray()) {
        const auto cmd = value.toObject();
        const QString id = cmd["id"].toString();
        if (id.startsWith(QLatin1String("model.add_")))
            entries.append({id, cmd["label"].toString()});
    }
    QHash<QAction *, QString> commands;
    for (const auto &entry : entries) {
        const QString kind = entry.id.mid(10);
        QString label = entry.label.isEmpty() ? kind : entry.label;
        auto *action = menu.addAction(PetuniaIcons::get(kind), petuniaTr(label));
        commands.insert(action, entry.id);
    }
    if (commands.isEmpty()) {
        menu.addAction(tr("Nenhuma primitiva disponível"))->setEnabled(false);
    }
    auto *picked = menu.exec(anchor->mapToGlobal(QPoint(anchor->width() + 2, 0)));
    if (picked && commands.contains(picked))
        m_controller->command(commands.value(picked));
}

void PetuniaToolRail::refreshStates(const QJsonObject &state) {
    QString active = state["active_tool"].toString();
    if (active == QLatin1String("box_select"))
        active = QStringLiteral("select");
    m_syncing = true;
    for (auto it = m_byId.cbegin(); it != m_byId.cend(); ++it) {
        auto *button = it.value();
        if (!button->isCheckable())
            continue;
        const QSignalBlocker block(button);
        button->setChecked(it.key() == active);
    }
    m_syncing = false;
    applyRoving();
}

void PetuniaToolRail::sync(const QJsonObject &state) {
    const auto catalog = state["tool_catalog"].toArray();
    QString signature;
    for (const auto &value : catalog)
        signature += value.toObject()["id"].toString() + QLatin1Char('|') + value.toObject()["shortcut"].toString() + QLatin1Char(';');
    signature += state["active_language"].toString();
    if (signature != m_signature) {
        m_signature = signature;
        rebuild(catalog);
    }
    refreshStates(state);
}

void PetuniaToolRail::applyRoving() {
    QToolButton *tabStop = nullptr;
    for (auto *button : m_buttons)
        if (button->isChecked())
            tabStop = button;
    if (!tabStop && !m_buttons.isEmpty())
        tabStop = m_buttons.first();
    for (auto *button : m_buttons)
        button->setFocusPolicy(button == tabStop ? Qt::StrongFocus : Qt::ClickFocus);
}

bool PetuniaToolRail::eventFilter(QObject *watched, QEvent *event) {
    if (event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);
        auto *button = qobject_cast<QToolButton *>(watched);
        const int index = m_buttons.indexOf(button);
        if (index >= 0 && (key->key() == Qt::Key_Up || key->key() == Qt::Key_Down ||
                           key->key() == Qt::Key_Home || key->key() == Qt::Key_End)) {
            int next = index;
            if (key->key() == Qt::Key_Up)
                next = index - 1;
            else if (key->key() == Qt::Key_Down)
                next = index + 1;
            else if (key->key() == Qt::Key_Home)
                next = 0;
            else
                next = m_buttons.size() - 1;
            next = qBound(0, next, int(m_buttons.size()) - 1);
            m_buttons[next]->setFocus(Qt::TabFocusReason);
            m_scroll->ensureWidgetVisible(m_buttons[next]);
            return true;
        }
    }
    return QWidget::eventFilter(watched, event);
}
