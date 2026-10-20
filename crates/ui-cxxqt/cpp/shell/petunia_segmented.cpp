#include "petunia_segmented.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"

#include <QHBoxLayout>
#include <QKeyEvent>
#include <QSignalBlocker>
#include <QStyle>

PetuniaSegmented::PetuniaSegmented(QWidget *parent) : QWidget(parent) {
    setObjectName(QStringLiteral("PetuniaSegmented"));
    m_group = new QButtonGroup(this);
    m_group->setExclusive(true);
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaSegmented::refreshIcons);
}

void PetuniaSegmented::setItems(const QList<Item> &items) {
    qDeleteAll(m_order);
    m_order.clear();
    m_buttons.clear();
    m_icons.clear();
    auto *layout = static_cast<QHBoxLayout *>(this->layout());
    for (const auto &item : items) {
        auto *button = new QPushButton(item.text, this);
        button->setCheckable(true);
        button->setProperty("segment", QStringLiteral("middle"));
        button->setToolTip(item.tooltip.isEmpty() ? item.text : item.tooltip);
        button->setAccessibleName(item.tooltip.isEmpty() ? item.text : item.tooltip);
        button->setFocusPolicy(Qt::TabFocus);
        button->installEventFilter(this);
        if (!item.icon.isEmpty()) {
            button->setIcon(PetuniaIcons::get(item.icon));
            button->setIconSize(QSize(PetuniaTheme::instance().iconPx(), PetuniaTheme::instance().iconPx()));
            m_icons.insert(item.id, item.icon);
        }
        m_group->addButton(button);
        layout->addWidget(button);
        m_buttons.insert(item.id, button);
        m_order.append(button);
        connect(button, &QPushButton::clicked, this, [this, id = item.id] {
            m_current = id;
            applyRoving();
            emit activated(id);
        });
    }
    rebuildCorners();
    applyRoving();
}

void PetuniaSegmented::refreshIcons() {
    const int px = PetuniaTheme::instance().iconPx();
    for (auto it = m_icons.cbegin(); it != m_icons.cend(); ++it) {
        if (auto *button = m_buttons.value(it.key())) {
            button->setIcon(PetuniaIcons::get(it.value()));
            button->setIconSize(QSize(px, px));
        }
    }
}

void PetuniaSegmented::rebuildCorners() {
    QList<QPushButton *> visible;
    for (auto *button : m_order)
        if (!button->isHidden())
            visible.append(button);
    for (int i = 0; i < visible.size(); ++i) {
        const char *role = visible.size() == 1 ? "only" : i == 0 ? "first" : i == visible.size() - 1 ? "last" : "middle";
        visible[i]->setProperty("segment", QString::fromLatin1(role));
        visible[i]->style()->unpolish(visible[i]);
        visible[i]->style()->polish(visible[i]);
    }
}

void PetuniaSegmented::setCurrent(const QString &id) {
    m_current = id;
    for (auto it = m_buttons.cbegin(); it != m_buttons.cend(); ++it) {
        const QSignalBlocker block(it.value());
        it.value()->setChecked(it.key() == id);
    }
    applyRoving();
}

void PetuniaSegmented::setItemVisible(const QString &id, bool visible) {
    if (auto *button = m_buttons.value(id)) {
        if (button->isHidden() == !visible)
            return;
        button->setVisible(visible);
        rebuildCorners();
    }
}

void PetuniaSegmented::setItemEnabled(const QString &id, bool enabled, const QString &reason) {
    if (auto *button = m_buttons.value(id)) {
        button->setEnabled(enabled);
        if (!enabled && !reason.isEmpty())
            button->setToolTip(button->accessibleName() + QStringLiteral(" — ") + reason);
    }
}

void PetuniaSegmented::setGroupLabel(const QString &label) {
    setAccessibleName(label);
}

void PetuniaSegmented::applyRoving() {
    // Foco por Tab entra no item atual; as setas percorrem os demais.
    for (auto *button : m_order)
        button->setFocusPolicy(button->isChecked() ? Qt::StrongFocus : Qt::ClickFocus);
    if (m_current.isEmpty() && !m_order.isEmpty() && !m_order.first()->isHidden())
        m_order.first()->setFocusPolicy(Qt::StrongFocus);
}

bool PetuniaSegmented::eventFilter(QObject *watched, QEvent *event) {
    if (event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);
        auto *button = qobject_cast<QPushButton *>(watched);
        const int index = m_order.indexOf(button);
        if (index >= 0 && (key->key() == Qt::Key_Left || key->key() == Qt::Key_Right ||
                           key->key() == Qt::Key_Up || key->key() == Qt::Key_Down ||
                           key->key() == Qt::Key_Home || key->key() == Qt::Key_End)) {
            const int step = (key->key() == Qt::Key_Left || key->key() == Qt::Key_Up) ? -1 : 1;
            int next = index;
            if (key->key() == Qt::Key_Home)
                next = -1;
            if (key->key() == Qt::Key_End)
                next = m_order.size();
            for (int n = 0; n < m_order.size(); ++n) {
                next = (key->key() == Qt::Key_Home || key->key() == Qt::Key_End)
                           ? (key->key() == Qt::Key_Home ? next + 1 : next - 1)
                           : next + step;
                if (next < 0 || next >= m_order.size())
                    break;
                if (!m_order[next]->isHidden() && m_order[next]->isEnabled()) {
                    m_order[next]->setFocus(Qt::TabFocusReason);
                    return true;
                }
            }
            return true;
        }
    }
    return QWidget::eventFilter(watched, event);
}
