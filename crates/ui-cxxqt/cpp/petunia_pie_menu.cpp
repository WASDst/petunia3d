#include "petunia_pie_menu.h"
#include "petunia_icons.h"
#include <QPainter>
#include <QPainterPath>
#include <QMouseEvent>
#include <QKeyEvent>
#include <QtMath>
#include <QApplication>

PetuniaPieMenu::PetuniaPieMenu(QWidget *parent)
    : QWidget(parent, Qt::Popup | Qt::FramelessWindowHint | Qt::NoDropShadowWindowHint) {
    setAttribute(Qt::WA_TranslucentBackground);
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    const int shadowMargin = 20;
    resize((m_radius + shadowMargin) * 2, (m_radius + shadowMargin) * 2);
    m_center = QPoint(width() / 2, height() / 2);

    addItem(QStringLiteral("cube"), QStringLiteral("Cubo"), QStringLiteral("cube"));
    addItem(QStringLiteral("cylinder"), QStringLiteral("Cilindro"), QStringLiteral("cylinder"));
    addItem(QStringLiteral("sphere"), QStringLiteral("Esfera UV"), QStringLiteral("sphere"));
    addItem(QStringLiteral("plane"), QStringLiteral("Plano"), QStringLiteral("plane"));
    addItem(QStringLiteral("cone"), QStringLiteral("Cone"), QStringLiteral("cone"));
    addItem(QStringLiteral("torus"), QStringLiteral("Toro"), QStringLiteral("torus"));
    addItem(QStringLiteral("capsule"), QStringLiteral("Cápsula"), QStringLiteral("capsule"));
    addItem(QStringLiteral("icosphere"), QStringLiteral("Icoesfera"), QStringLiteral("circle"));
}

void PetuniaPieMenu::addItem(const QString &id, const QString &label, const QString &iconName) {
    m_items.append({id, label, iconName, 0.0f, 0.0f});
    recalculateAngles();
}

void PetuniaPieMenu::recalculateAngles() {
    if (m_items.isEmpty())
        return;
    const float step = 360.0f / m_items.size();
    // Start at top (90 degrees in Qt polar: 0 is 3 o'clock, 90 is 12 o'clock counter-clockwise)
    for (int i = 0; i < m_items.size(); ++i) {
        m_items[i].startAngle = 90.0f - (i + 0.5f) * step;
        m_items[i].spanAngle = step;
    }
}

void PetuniaPieMenu::execAt(const QPoint &globalPos) {
    m_hoveredIndex = -1;
    move(globalPos.x() - width() / 2, globalPos.y() - height() / 2);
    show();
    raise();
    activateWindow();
}

int PetuniaPieMenu::itemAt(const QPoint &pos) const {
    const int dx = pos.x() - m_center.x();
    const int dy = -(pos.y() - m_center.y()); // Invert Y for standard Cartesian coordinates
    const float dist = std::sqrt(static_cast<float>(dx * dx + dy * dy));
    if (dist < m_innerRadius || dist > m_radius + 15)
        return -1;

    float angleDeg = qRadiansToDegrees(std::atan2(static_cast<float>(dy), static_cast<float>(dx)));
    if (angleDeg < 0.0f)
        angleDeg += 360.0f;

    // Normalize with top (90 deg) as sector 0
    const float step = 360.0f / m_items.size();
    float relAngle = 90.0f - angleDeg + step * 0.5f;
    while (relAngle < 0.0f) relAngle += 360.0f;
    while (relAngle >= 360.0f) relAngle -= 360.0f;

    int idx = static_cast<int>(relAngle / step);
    return (idx >= 0 && idx < m_items.size()) ? idx : -1;
}

void PetuniaPieMenu::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setRenderHint(QPainter::SmoothPixmapTransform);

    // Sombra suave de elevação (SDF / Radial drop-off estilo Kirigami Elevation)
    const float shadowR = m_radius + 16.0f;
    QRadialGradient shadowGrad(m_center, shadowR);
    shadowGrad.setColorAt(0.0, QColor(0, 0, 0, 80));
    shadowGrad.setColorAt(m_radius / shadowR, QColor(0, 0, 0, 70));
    shadowGrad.setColorAt((m_radius + 2.0f) / shadowR, QColor(0, 0, 0, 45));
    shadowGrad.setColorAt((m_radius + 8.0f) / shadowR, QColor(0, 0, 0, 18));
    shadowGrad.setColorAt(0.98, QColor(0, 0, 0, 2));
    shadowGrad.setColorAt(1.0, QColor(0, 0, 0, 0));

    painter.setPen(Qt::NoPen);
    painter.setBrush(shadowGrad);
    painter.drawEllipse(m_center, static_cast<int>(shadowR), static_cast<int>(shadowR));

    // Corpo sólido do Pie Menu com borda sutil anti-serrilhado
    painter.setPen(QPen(QColor(65, 65, 65, 220), 1.2));
    painter.setBrush(QColor(24, 24, 24, 245));
    painter.drawEllipse(m_center, m_radius, m_radius);

    // Draw Pie Slices
    const int count = m_items.size();
    if (count > 0) {
        const float step = 360.0f / count;
        for (int i = 0; i < count; ++i) {
            const bool isHovered = (i == m_hoveredIndex);
            const float midAngleDeg = 90.0f - i * step;
            const float rad = qDegreesToRadians(midAngleDeg);

            if (isHovered) {
                // Highlight sector
                QPainterPath path;
                path.moveTo(m_center);
                QRectF pieRect(m_center.x() - m_radius, m_center.y() - m_radius, m_radius * 2, m_radius * 2);
                path.arcTo(pieRect, m_items[i].startAngle, m_items[i].spanAngle);
                path.closeSubpath();

                painter.setPen(Qt::NoPen);
                painter.setBrush(QColor(181, 140, 255, 65)); // Soft purple tint
                painter.drawPath(path);
            }

            // Draw icon along radius
            const float iconDist = (m_innerRadius + m_radius) * 0.52f;
            const float ix = m_center.x() + iconDist * std::cos(rad);
            const float iy = m_center.y() - iconDist * std::sin(rad);

            QIcon icon = PetuniaIcons::get(m_items[i].iconName, isHovered ? QStringLiteral("#ffffff") : QStringLiteral("#bbbbbb"));
            if (!icon.isNull()) {
                const int iconSz = isHovered ? 24 : 20;
                icon.paint(&painter, QRect(ix - iconSz / 2, iy - iconSz / 2, iconSz, iconSz));
            }
        }
    }

    // Draw Inner Center Hub
    painter.setPen(QPen(QColor(70, 70, 70, 200), 1.5));
    painter.setBrush(QColor(28, 28, 28, 250));
    painter.drawEllipse(m_center, m_innerRadius, m_innerRadius);

    // Center Label text
    painter.setPen(m_hoveredIndex >= 0 ? QColor(181, 140, 255) : QColor(160, 160, 160));
    QFont font = painter.font();
    font.setPointSize(9);
    font.setBold(true);
    painter.setFont(font);

    const QString centerText = (m_hoveredIndex >= 0 && m_hoveredIndex < m_items.size())
                                   ? m_items[m_hoveredIndex].label
                                   : QStringLiteral("Primitivas");
    painter.drawText(QRect(m_center.x() - m_innerRadius, m_center.y() - m_innerRadius,
                           m_innerRadius * 2, m_innerRadius * 2),
                     Qt::AlignCenter | Qt::TextWordWrap, centerText);
}

void PetuniaPieMenu::mouseMoveEvent(QMouseEvent *event) {
    int next = itemAt(event->pos());
    if (next != m_hoveredIndex) {
        m_hoveredIndex = next;
        update();
    }
}

void PetuniaPieMenu::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        int idx = itemAt(event->pos());
        if (idx >= 0 && idx < m_items.size()) {
            emit primitiveSelected(m_items[idx].id);
            close();
            return;
        }
    }
    close();
}

void PetuniaPieMenu::mouseReleaseEvent(QMouseEvent *event) {
    int idx = itemAt(event->pos());
    if (idx >= 0 && idx < m_items.size()) {
        emit primitiveSelected(m_items[idx].id);
        close();
    }
}

void PetuniaPieMenu::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Escape) {
        close();
        return;
    }
    QWidget::keyPressEvent(event);
}

void PetuniaPieMenu::focusOutEvent(QFocusEvent *) {
    close();
}
