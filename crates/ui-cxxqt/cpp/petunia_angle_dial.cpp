#include "petunia_angle_dial.h"
#include <QPainter>
#include <QPainterPath>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QKeyEvent>
#include <cmath>

PetuniaAngleDial::PetuniaAngleDial(QWidget *parent)
    : QWidget(parent)
{
    setFocusPolicy(Qt::StrongFocus);
    setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Preferred);
    setAccessibleName(tr("Ângulo"));
    setAccessibleDescription(tr("Seletor angular de 0 a 360 graus"));
}

QSize PetuniaAngleDial::sizeHint() const {
    return QSize(44, 44);
}

QSize PetuniaAngleDial::minimumSizeHint() const {
    return QSize(32, 32);
}

float PetuniaAngleDial::normalizeAngle(float degrees) {
    if (!std::isfinite(degrees)) {
        return 0.0f;
    }
    float norm = std::fmod(degrees, 360.0f);
    if (norm < 0.0f) {
        norm += 360.0f;
    }
    return norm;
}

void PetuniaAngleDial::setAngle(float degrees) {
    float norm = normalizeAngle(degrees);
    if (std::abs(m_angle - norm) > 0.001f) {
        m_angle = norm;
        update();
        emit angleChanged(m_angle);
    }
}

void PetuniaAngleDial::setSnapStep(float step) {
    if (std::isfinite(step) && step > 0.0f) {
        m_snapStep = step;
    }
}

void PetuniaAngleDial::updateAngleFromPoint(const QPointF &pos, bool snap) {
    const float cx = width() * 0.5f;
    const float cy = height() * 0.5f;
    const float dx = pos.x() - cx;
    const float dy = -(pos.y() - cy); // Inverte Y para sentido trigonométrico padrão

    if (std::abs(dx) < 0.001f && std::abs(dy) < 0.001f) {
        return;
    }

    float rad = std::atan2(dy, dx);
    float deg = qRadiansToDegrees(rad);
    if (deg < 0.0f) {
        deg += 360.0f;
    }

    if (snap && m_snapStep > 0.0f) {
        deg = std::round(deg / m_snapStep) * m_snapStep;
        if (deg >= 360.0f) {
            deg -= 360.0f;
        }
    }

    setAngle(deg);
}

void PetuniaAngleDial::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    const float cx = width() * 0.5f;
    const float cy = height() * 0.5f;
    const float radius = qMin(cx, cy) - 2.0f;
    if (radius <= 2.0f) return;

    // 1. Fundo Circular Escuro
    painter.setPen(hasFocus() ? QPen(QColor(181, 140, 255), 1.5) : QPen(QColor(64, 64, 64), 1.0));
    painter.setBrush(QColor(32, 32, 32));
    painter.drawEllipse(QPointF(cx, cy), radius, radius);

    // 2. Traços de Grau (Ticks a cada 45°)
    for (int deg = 0; deg < 360; deg += 45) {
        float rTick = qDegreesToRadians(static_cast<float>(deg));
        bool isCardinal = (deg % 90 == 0);
        float tickLen = isCardinal ? 4.5f : 2.5f;
        float outerR = radius - 1.5f;
        float innerR = outerR - tickLen;

        float cosA = std::cos(rTick);
        float sinA = -std::sin(rTick); // Y invertido para tela

        QPointF p1(cx + innerR * cosA, cy + innerR * sinA);
        QPointF p2(cx + outerR * cosA, cy + outerR * sinA);

        painter.setPen(QPen(isCardinal ? QColor(100, 100, 100) : QColor(65, 65, 65), 1.0));
        painter.drawLine(p1, p2);
    }

    // 3. Arco preenchido indicando o ângulo
    if (m_angle > 0.5f) {
        QRectF arcRect(cx - radius + 3.0f, cy - radius + 3.0f, (radius - 3.0f) * 2.0f, (radius - 3.0f) * 2.0f);
        QPainterPath arcPath;
        // Qt arcTo usa 0° às 3 horas e ângulos positivos no sentido anti-horário
        arcPath.arcMoveTo(arcRect, 0.0f);
        arcPath.arcTo(arcRect, 0.0f, m_angle);
        painter.setPen(QPen(QColor(181, 140, 255, 60), 2.5));
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(arcPath);
    }

    // 4. Ponteiro / Agulha orientada no ângulo m_angle
    float angleRad = qDegreesToRadians(m_angle);
    float cosAngle = std::cos(angleRad);
    float sinAngle = -std::sin(angleRad);

    float needleLen = radius - 4.0f;
    QPointF tip(cx + needleLen * cosAngle, cy + needleLen * sinAngle);

    QColor needleColor = m_dragging ? QColor(209, 184, 255) : QColor(181, 140, 255);
    painter.setPen(QPen(needleColor, 2.0, Qt::SolidLine, Qt::RoundCap));
    painter.drawLine(QPointF(cx, cy), tip);

    // Círculo na ponta da agulha
    painter.setPen(Qt::NoPen);
    painter.setBrush(needleColor);
    painter.drawEllipse(tip, 2.5, 2.5);

    // 5. Cubo central (Pivot)
    painter.setPen(QPen(QColor(70, 70, 70), 1.0));
    painter.setBrush(QColor(45, 45, 45));
    painter.drawEllipse(QPointF(cx, cy), 3.5, 3.5);
}

void PetuniaAngleDial::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        m_dragging = true;
        const bool snap = (event->modifiers() & Qt::ShiftModifier);
        updateAngleFromPoint(event->position(), snap);
        event->accept();
    } else {
        QWidget::mousePressEvent(event);
    }
}

void PetuniaAngleDial::mouseMoveEvent(QMouseEvent *event) {
    if (m_dragging) {
        const bool snap = (event->modifiers() & Qt::ShiftModifier);
        updateAngleFromPoint(event->position(), snap);
        event->accept();
    } else {
        QWidget::mouseMoveEvent(event);
    }
}

void PetuniaAngleDial::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && m_dragging) {
        m_dragging = false;
        update();
        event->accept();
    } else {
        QWidget::mouseReleaseEvent(event);
    }
}

void PetuniaAngleDial::mouseDoubleClickEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        setAngle(0.0f);
        event->accept();
    } else {
        QWidget::mouseDoubleClickEvent(event);
    }
}

void PetuniaAngleDial::wheelEvent(QWheelEvent *event) {
    float step = (event->modifiers() & Qt::ShiftModifier) ? 1.0f : 5.0f;
    if (event->angleDelta().y() < 0) {
        step = -step;
    }
    setAngle(m_angle + step);
    event->accept();
}

void PetuniaAngleDial::keyPressEvent(QKeyEvent *event) {
    float step = (event->modifiers() & Qt::ShiftModifier) ? 0.5f : 1.0f;
    switch (event->key()) {
    case Qt::Key_Left:
    case Qt::Key_Down:
        setAngle(m_angle - step);
        event->accept();
        break;
    case Qt::Key_Right:
    case Qt::Key_Up:
        setAngle(m_angle + step);
        event->accept();
        break;
    case Qt::Key_PageDown:
        setAngle(m_angle - 15.0f);
        event->accept();
        break;
    case Qt::Key_PageUp:
        setAngle(m_angle + 15.0f);
        event->accept();
        break;
    case Qt::Key_Home:
        setAngle(0.0f);
        event->accept();
        break;
    case Qt::Key_End:
        setAngle(180.0f);
        event->accept();
        break;
    default:
        QWidget::keyPressEvent(event);
        break;
    }
}

void PetuniaAngleDial::focusInEvent(QFocusEvent *event) {
    update();
    QWidget::focusInEvent(event);
}

void PetuniaAngleDial::focusOutEvent(QFocusEvent *event) {
    m_dragging = false;
    update();
    QWidget::focusOutEvent(event);
}
