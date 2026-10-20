#include "petunia_context_actions.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QHBoxLayout>
#include <QJsonArray>
#include <QLabel>
#include <QMenu>
#include <QWidgetAction>

PetuniaContextActions::PetuniaContextActions(PetuniaShellController *controller, QWidget *parent)
    : QFrame(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaContextActions"));
    setProperty("role", QStringLiteral("overlay"));
    setAccessibleName(tr("Ações da seleção"));
    m_layout = new QHBoxLayout(this);
    const auto &theme = PetuniaTheme::instance();
    m_layout->setContentsMargins(theme.space(2), theme.space(1), theme.space(2), theme.space(1));
    m_layout->setSpacing(theme.space(1));
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, [this] {
        m_signature.clear();
        sync(m_controller->state());
    });
    hide();
}

QStringList PetuniaContextActions::primaryIds() const {
    QStringList ids;
    for (const auto *button : m_primary)
        ids << button->property("actionId").toString();
    return ids;
}

namespace {
/// Título de grupo do menu "Mais": rótulo não selecionável com o estilo do tema.
void addGroupHeader(QMenu *menu, const QString &text) {
    auto *label = new QLabel(text, menu);
    label->setProperty("role", QStringLiteral("muted"));
    const auto &theme = PetuniaTheme::instance();
    label->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(1));
    label->setAccessibleName(text);
    auto *holder = new QWidgetAction(menu);
    holder->setDefaultWidget(label);
    holder->setEnabled(false);
    menu->addAction(holder);
}

QString groupTitle(const QString &id) {
    if (id == QLatin1String("object"))
        return PetuniaContextActions::tr("Objeto");
    if (id == QLatin1String("scene"))
        return PetuniaContextActions::tr("Cena");
    if (id == QLatin1String("form"))
        return PetuniaContextActions::tr("Forma");
    if (id == QLatin1String("topology"))
        return PetuniaContextActions::tr("Topologia");
    if (id == QLatin1String("shape"))
        return PetuniaContextActions::tr("Perfil e formas");
    if (id == QLatin1String("uv"))
        return PetuniaContextActions::tr("UV");
    if (id == QLatin1String("utility"))
        return PetuniaContextActions::tr("Utilitários");
    return PetuniaContextActions::tr("Remover");
}
} // namespace

void PetuniaContextActions::run(const QJsonObject &action) {
    const QString kind = action["kind"].toString();
    const QString target = action["target"].toString();
    if (target.startsWith(QLatin1String("ui."))) {
        QWidget *anchor = nullptr;
        for (auto *button : std::as_const(m_primary))
            if (button->property("actionId").toString() == action["id"].toString())
                anchor = button;
        if (!anchor)
            anchor = m_more;
        emit uiActionRequested(target, anchor ? anchor : static_cast<QWidget *>(this));
        return;
    }
    if (kind == QLatin1String("command"))
        m_controller->command(target);
    else if (kind == QLatin1String("tool"))
        m_controller->request(QStringLiteral("tool"), {{"id", target}});
    else if (kind == QLatin1String("request"))
        m_controller->request(target);
}

void PetuniaContextActions::rebuild(const QJsonArray &actions) {
    qDeleteAll(m_primary);
    m_primary.clear();
    if (m_more) {
        delete m_more;
        m_more = nullptr;
    }
    const auto &theme = PetuniaTheme::instance();
    QList<QJsonObject> overflow;
    for (const auto &value : actions) {
        const auto action = value.toObject();
        if (!action["primary"].toBool()) {
            overflow.append(action);
            continue;
        }
        auto *button = new QToolButton(this);
        button->setProperty("role", QStringLiteral("header"));
        button->setProperty("actionId", action["id"].toString());
        button->setIcon(PetuniaIcons::get(action["icon"].toString()));
        button->setIconSize(QSize(theme.iconPx(), theme.iconPx()));
        button->setText(petuniaTr(action["label"].toString()));
        button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        button->setMinimumHeight(theme.compactControlHeight());
        button->setAccessibleName(petuniaTr(action["label"].toString()));
        connect(button, &QToolButton::clicked, this, [this, action] { run(action); });
        m_layout->addWidget(button);
        m_primary.append(button);
    }
    if (!overflow.isEmpty()) {
        m_more = new QToolButton(this);
        m_more->setProperty("role", QStringLiteral("header"));
        m_more->setText(tr("Mais"));
        m_more->setIcon(PetuniaIcons::get(QStringLiteral("more_horiz")));
        m_more->setIconSize(QSize(theme.iconPx(), theme.iconPx()));
        m_more->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        m_more->setPopupMode(QToolButton::InstantPopup);
        m_more->setMinimumHeight(theme.compactControlHeight());
        m_more->setAccessibleName(tr("Mais ações"));
        auto *menu = new QMenu(m_more);
        // O "Mais" é organizado por grupo (objeto, cena, forma, topologia…), na
        // ordem publicada pelo editor, em vez de uma lista única e longa.
        static const QStringList order{"object", "scene", "form", "topology", "shape", "uv", "utility", "danger"};
        for (const auto &group : order) {
            bool headerShown = false;
            for (const auto &action : overflow) {
                if (action["group"].toString(QStringLiteral("object")) != group)
                    continue;
                if (!headerShown) {
                    if (!menu->actions().isEmpty())
                        menu->addSeparator();
                    addGroupHeader(menu, groupTitle(group));
                    headerShown = true;
                }
                auto *item = menu->addAction(PetuniaIcons::get(action["icon"].toString()),
                                             petuniaTr(action["label"].toString()));
                item->setProperty("actionId", action["id"].toString());
                connect(item, &QAction::triggered, this, [this, action] { run(action); });
            }
        }
        m_more->setMenu(menu);
        m_layout->addWidget(m_more);
    }
    m_actions = actions;
    // Largura "cheia" (com texto) para decidir a compactação sem oscilar.
    const bool wasCompact = m_compact;
    m_compact = false;
    for (auto *button : std::as_const(m_primary))
        button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    if (m_more)
        m_more->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_layout->activate();
    m_fullWidth = sizeHint().width();
    if (wasCompact)
        setCompact(true);
}

int PetuniaContextActions::fullWidth() const {
    return m_compact ? m_fullWidth : sizeHint().width();
}

void PetuniaContextActions::setCompact(bool compact) {
    if (m_compact == compact)
        return;
    if (compact)
        m_fullWidth = sizeHint().width();
    m_compact = compact;
    const auto style = compact ? Qt::ToolButtonIconOnly : Qt::ToolButtonTextBesideIcon;
    for (auto *button : std::as_const(m_primary))
        button->setToolButtonStyle(style);
    if (m_more)
        m_more->setToolButtonStyle(style);
    adjustSize();
}

void PetuniaContextActions::refreshStates() {
    auto describe = [this](const QJsonObject &action, bool *enabled) {
        QString reason;
        bool ok = true;
        if (action["kind"].toString() == QLatin1String("command"))
            ok = m_controller->commandEnabled(action["target"].toString(), &reason);
        *enabled = ok;
        QString tip = petuniaTr(action["label"].toString());
        const QString shortcut = action["shortcut"].toString();
        if (!shortcut.isEmpty())
            tip += QStringLiteral("  (%1)").arg(shortcut);
        if (!ok && !reason.isEmpty())
            tip += QStringLiteral(" — ") + reason;
        return tip;
    };
    int index = 0;
    for (const auto &value : m_actions) {
        const auto action = value.toObject();
        if (!action["primary"].toBool())
            continue;
        if (index >= m_primary.size())
            break;
        bool enabled = true;
        const QString tip = describe(action, &enabled);
        m_primary[index]->setEnabled(enabled);
        if (m_primary[index]->toolTip() != tip)
            m_primary[index]->setToolTip(tip);
        ++index;
    }
    if (m_more && m_more->menu()) {
        const auto items = m_more->menu()->actions();
        for (const auto &value : m_actions) {
            const auto action = value.toObject();
            if (action["primary"].toBool())
                continue;
            for (auto *item : items) {
                if (item->property("actionId").toString() != action["id"].toString())
                    continue;
                bool enabled = true;
                const QString tip = describe(action, &enabled);
                item->setEnabled(enabled);
                item->setToolTip(tip);
            }
        }
    }
}

void PetuniaContextActions::sync(const QJsonObject &state) {
    const auto actions = state["context_actions"].toArray();
    QString signature = state["workspace"].toString() + state["selection_domain"].toString() +
                        state["modeling_mode"].toString() + state["active_language"].toString();
    for (const auto &value : actions)
        signature += value.toObject()["id"].toString() + QLatin1Char('|');
    if (signature != m_signature) {
        m_signature = signature;
        rebuild(actions);
    }
    // Sem objeto selecionado não há o que operar (UV opera sobre a malha ativa).
    const bool needsSelection = state["workspace"].toString() == QLatin1String("Model");
    const bool hasSelection = state["selected_count"].toInt() > 0;
    const bool visible = !m_suppressed && !actions.isEmpty() && (!needsSelection || hasSelection);
    if (visible)
        refreshStates();
    if (isVisible() != visible) {
        setVisible(visible);
        if (visible)
            adjustSize();
    } else if (visible) {
        adjustSize();
    }
}
