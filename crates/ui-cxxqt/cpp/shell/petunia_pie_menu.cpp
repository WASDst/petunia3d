#include "petunia_pie_menu.h"

#include "../petunia_icons.h"
#include "../petunia_theme.h"

#include <QGuiApplication>
#include <QKeyEvent>
#include <QMenu>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QScreen>
#include <QSettings>
#include <QTimer>
#include <QToolButton>

#include <algorithm>
#include <cmath>

namespace {
constexpr int kMaxItems = 8;
constexpr double kPi = 3.14159265358979323846;

QSettings shellSettings() { return QSettings(QStringLiteral("Petunia3D"), QStringLiteral("QtShell")); }

double angularDistance(double a, double b) {
    double d = std::fmod(std::abs(a - b), 360.0);
    return d > 180.0 ? 360.0 - d : d;
}
} // namespace

bool PetuniaPieMenu::listFallback() {
    return shellSettings().value(QStringLiteral("ui/pieMenusAsList"), false).toBool();
}

void PetuniaPieMenu::setListFallback(bool enabled) {
    shellSettings().setValue(QStringLiteral("ui/pieMenusAsList"), enabled);
}

PetuniaPieMenu::PetuniaPieMenu(const QString &title, QWidget *parent)
    : QWidget(parent, Qt::Popup | Qt::FramelessWindowHint), m_title(title) {
    setObjectName(QStringLiteral("PetuniaPieMenu"));
    setAttribute(Qt::WA_TranslucentBackground);
    setAttribute(Qt::WA_DeleteOnClose);
    setMouseTracking(true);
    setAccessibleName(title);
    setAccessibleDescription(
        tr("Menu em torta: aponte e clique, use os números 1 a 8, ou setas e Enter. Esc cancela."));
    m_asList = new QToolButton(this);
    m_asList->setObjectName(QStringLiteral("pie-as-list"));
    m_asList->setIcon(PetuniaIcons::get(QStringLiteral("list_view")));
    m_asList->setAutoRaise(true);
    m_asList->setToolTip(tr("Mostrar como lista (lembrado)"));
    m_asList->setAccessibleName(tr("Mostrar como lista"));
    connect(m_asList, &QToolButton::clicked, this, [this] {
        setListFallback(true);
        const QPoint at = mapToGlobal(center().toPoint());
        QWidget *owner = parentWidget();
        QMenu *menu = buildListMenu(owner);
        m_activated = true; // a lista assume; fechar a torta não é "cancelar"
        close();
        menu->popup(at);
    });
}

void PetuniaPieMenu::setItems(const QVector<PetuniaPieItem> &items) {
    qDeleteAll(m_buttons);
    m_buttons.clear();
    m_items = items.mid(0, kMaxItems);
    const auto &theme = PetuniaTheme::instance();
    for (int i = 0; i < m_items.size(); ++i) {
        const auto &item = m_items.at(i);
        auto *button = new QPushButton(QStringLiteral("%1  %2").arg(i + 1).arg(item.label), this);
        button->setObjectName(QStringLiteral("pie-item-%1").arg(item.id));
        if (!item.icon.isEmpty())
            button->setIcon(PetuniaIcons::get(item.icon));
        button->setEnabled(item.enabled);
        button->setAccessibleName(item.label);
        button->setAccessibleDescription(tr("Tecla %1").arg(i + 1));
        button->setMinimumHeight(theme.minHitTarget());
        button->setFocusPolicy(Qt::StrongFocus);
        button->setProperty("role", QStringLiteral("pie"));
        connect(button, &QPushButton::clicked, this, [this, i] { activate(i); });
        m_buttons.append(button);
    }
    const int side = 2 * (ringRadius() + theme.scaled(80));
    setFixedSize(side, side);
    layoutButtons();
}

int PetuniaPieMenu::ringRadius() const { return PetuniaTheme::instance().scaled(m_items.size() > 6 ? 118 : 100); }

int PetuniaPieMenu::deadZone() const { return PetuniaTheme::instance().scaled(26); }

QPointF PetuniaPieMenu::center() const { return QPointF(width() / 2.0, height() / 2.0); }

double PetuniaPieMenu::itemAngle(int index) const {
    // Primeiro item em cima; os demais no sentido horário (0° = direita, anti-horário +).
    const int n = std::max(1, int(m_items.size()));
    return 90.0 - index * 360.0 / n;
}

void PetuniaPieMenu::layoutButtons() {
    const QPointF c = center();
    const int r = ringRadius();
    for (int i = 0; i < m_buttons.size(); ++i) {
        QPushButton *button = m_buttons.at(i);
        button->adjustSize();
        const double a = itemAngle(i) * kPi / 180.0;
        const QPointF at(c.x() + r * std::cos(a), c.y() - r * std::sin(a));
        button->move((at - QPointF(button->width() / 2.0, button->height() / 2.0)).toPoint());
    }
    m_asList->adjustSize();
    const auto &theme = PetuniaTheme::instance();
    m_asList->move((c + QPointF(-m_asList->width() / 2.0, theme.scaled(10))).toPoint());
}

int PetuniaPieMenu::indexForOffset(const QPointF &offset) const {
    if (m_items.isEmpty() || std::hypot(offset.x(), offset.y()) < deadZone())
        return -1;
    const double angle = std::atan2(-offset.y(), offset.x()) * 180.0 / kPi;
    int best = -1;
    double bestDistance = 1e9;
    for (int i = 0; i < m_items.size(); ++i) {
        const double d = angularDistance(angle, itemAngle(i));
        if (d < bestDistance) {
            bestDistance = d;
            best = i;
        }
    }
    return best;
}

void PetuniaPieMenu::setHighlighted(int index) {
    if (index < -1 || index >= m_buttons.size())
        index = -1;
    if (m_highlight == index)
        return;
    m_highlight = index;
    if (QPushButton *button = m_buttons.value(index))
        button->setFocus(Qt::OtherFocusReason);
    update();
}

bool PetuniaPieMenu::activate(int index) {
    if (index < 0 || index >= m_items.size() || !m_items.at(index).enabled || m_activated)
        return false;
    m_activated = true;
    const PetuniaPieItem item = m_items.at(index);
    close();
    if (item.trigger)
        item.trigger();
    emit triggered(item.id);
    return true;
}

void PetuniaPieMenu::popup(const QPoint &globalCenter) {
    if (listFallback()) {
        QMenu *menu = buildListMenu(parentWidget());
        menu->popup(globalCenter);
        m_activated = true;
        deleteLater();
        return;
    }
    QRect g(QPoint(), size());
    g.moveCenter(globalCenter);
    // Perto da borda, a torta entra na tela (o centro deixa de ficar sob o ponteiro).
    if (QScreen *screen = QGuiApplication::screenAt(globalCenter)) {
        const QRect area = screen->availableGeometry();
        g.moveLeft(std::clamp(g.left(), area.left(), std::max(area.left(), area.right() - g.width())));
        g.moveTop(std::clamp(g.top(), area.top(), std::max(area.top(), area.bottom() - g.height())));
    }
    move(g.topLeft());
    show();
    raise();
    activateWindow();
    setFocus(Qt::PopupFocusReason);
}

QMenu *PetuniaPieMenu::buildListMenu(QWidget *parent) {
    auto *menu = new QMenu(parent);
    menu->setObjectName(QStringLiteral("PetuniaPieList"));
    menu->setAttribute(Qt::WA_DeleteOnClose);
    menu->setTitle(m_title);
    menu->setAccessibleName(m_title);
    menu->addSection(m_title);
    for (int i = 0; i < m_items.size(); ++i) {
        const auto item = m_items.at(i);
        QAction *action = menu->addAction(PetuniaIcons::get(item.icon), QStringLiteral("&%1  %2").arg(i + 1).arg(item.label));
        action->setEnabled(item.enabled);
        connect(action, &QAction::triggered, menu, [item] {
            if (item.trigger)
                item.trigger();
        });
    }
    menu->addSeparator();
    QAction *pie = menu->addAction(tr("Mostrar em torta"));
    connect(pie, &QAction::triggered, menu, [] { setListFallback(false); });
    return menu;
}

bool PetuniaPieMenu::focusNextPrevChild(bool next) {
    // Tab percorre só os itens (e o botão de lista), em volta do círculo.
    const int n = int(m_buttons.size());
    if (n == 0)
        return QWidget::focusNextPrevChild(next);
    int index = m_highlight < 0 ? (next ? 0 : n - 1) : (m_highlight + (next ? 1 : -1) + n) % n;
    setHighlighted(index);
    return true;
}

void PetuniaPieMenu::keyPressEvent(QKeyEvent *event) {
    const int n = int(m_buttons.size());
    const int key = event->key();
    if (key == Qt::Key_Escape) {
        close();
        return;
    }
    if (key >= Qt::Key_1 && key <= Qt::Key_8) {
        activate(key - Qt::Key_1);
        return;
    }
    if (n > 0 && (key == Qt::Key_Right || key == Qt::Key_Down)) {
        setHighlighted(m_highlight < 0 ? 0 : (m_highlight + 1) % n);
        return;
    }
    if (n > 0 && (key == Qt::Key_Left || key == Qt::Key_Up)) {
        setHighlighted(m_highlight < 0 ? n - 1 : (m_highlight - 1 + n) % n);
        return;
    }
    if (key == Qt::Key_Return || key == Qt::Key_Enter || key == Qt::Key_Space) {
        activate(m_highlight);
        return;
    }
    QWidget::keyPressEvent(event);
}

void PetuniaPieMenu::mouseMoveEvent(QMouseEvent *event) {
    m_pointer = event->position();
    setHighlighted(indexForOffset(m_pointer - center()));
    update();
}

void PetuniaPieMenu::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::RightButton) {
        close();
        return;
    }
    if (event->button() != Qt::LeftButton)
        return;
    m_pointer = event->position();
    m_pressedOutside = indexForOffset(m_pointer - center()) >= 0;
    if (!m_pressedOutside && !rect().contains(event->position().toPoint())) {
        close(); // clique fora da janela da torta
    }
}

void PetuniaPieMenu::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() != Qt::LeftButton)
        return;
    const int index = indexForOffset(event->position() - center());
    if (index >= 0 && m_pressedOutside) {
        activate(index);
    } else if (index < 0 && std::hypot(event->position().x() - center().x(), event->position().y() - center().y()) <
                                deadZone()) {
        close(); // clique no centro cancela
    }
    m_pressedOutside = false;
}

void PetuniaPieMenu::hideEvent(QHideEvent *event) {
    QWidget::hideEvent(event);
    emit closed();
}

void PetuniaPieMenu::paintEvent(QPaintEvent *) {
    const auto &theme = PetuniaTheme::instance();
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    const QPointF c = center();
    const int r = ringRadius();

    // Disco central com o título e a zona morta; anel guia discreto.
    QColor panel = theme.bgPanel();
    panel.setAlpha(235);
    p.setPen(QPen(theme.borderStrong(), 1));
    p.setBrush(panel);
    const double hub = theme.scaled(46);
    p.drawEllipse(c, hub, hub);
    QPen guide(theme.borderSubtle(), theme.scaled(2));
    guide.setStyle(Qt::DotLine);
    p.setPen(guide);
    p.setBrush(Qt::NoBrush);
    p.drawEllipse(c, r, r);

    // Fatia do item em destaque: forma + cor (não só cor).
    if (m_highlight >= 0 && !m_items.isEmpty()) {
        const double span = 360.0 / m_items.size();
        const double start = itemAngle(m_highlight) - span / 2.0;
        QPainterPath wedge;
        const QRectF outer(c.x() - r, c.y() - r, 2.0 * r, 2.0 * r);
        wedge.moveTo(c);
        wedge.arcTo(outer, start, span);
        wedge.closeSubpath();
        QPainterPath hole;
        hole.addEllipse(c, hub, hub);
        QColor accent = theme.accentPrimary();
        accent.setAlpha(70);
        p.setPen(Qt::NoPen);
        p.setBrush(accent);
        p.drawPath(wedge.subtracted(hole));
        // Direção atual do ponteiro.
        const double a = itemAngle(m_highlight) * kPi / 180.0;
        QPen ray(theme.accentPrimary(), theme.scaled(2));
        ray.setCapStyle(Qt::RoundCap);
        p.setPen(ray);
        p.drawLine(c + QPointF(hub * std::cos(a), -hub * std::sin(a)),
                   c + QPointF((r - theme.scaled(28)) * std::cos(a), -(r - theme.scaled(28)) * std::sin(a)));
    }

    QFont f = font();
    f.setBold(true);
    f.setPixelSize(theme.fontPx(PetuniaTheme::TextRole::Caption));
    p.setFont(f);
    p.setPen(theme.textPrimary());
    p.drawText(QRectF(c.x() - hub, c.y() - hub * 0.7, 2 * hub, hub * 0.8), Qt::AlignCenter | Qt::TextWordWrap,
               m_title);
}
