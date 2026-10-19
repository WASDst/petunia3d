#include "petunia_brush_cursor.h"

#include <QJsonArray>
#include <QPainter>
#include <QRadialGradient>

#include <algorithm>

void petuniaDrawBrushCursor(QPainter &painter, const QJsonObject &state, const PetuniaBrushCursorStyle &style) {
    const QString tool = state[QStringLiteral("active_tool")].toString();
    // Mesmo diâmetro (px de tela) que o carimbo usa nas duas superfícies.
    const double diameter = std::max(2.0, state[QStringLiteral("brush_size")].toDouble(1.0) * 16.0);
    const double radius = diameter * 0.5;
    const bool erase = tool == QLatin1String("eraser");
    const bool hardEdge = tool == QLatin1String("pixel");
    const double hardness = std::clamp(state[QStringLiteral("brush_hardness")].toDouble(0.0), 0.0, 1.0);
    const double strength = std::clamp(
        state[QStringLiteral("brush_opacity")].toDouble(1.0) * state[QStringLiteral("brush_flow")].toDouble(1.0), 0.0,
        1.0);
    const double opacityJitter = std::clamp(state[QStringLiteral("brush_opacity_jitter")].toDouble(0.0), 0.0, 1.0);
    const double sizeJitter = std::clamp(state[QStringLiteral("brush_size_jitter")].toDouble(0.0), 0.0, 1.0);
    const double scatter = std::clamp(state[QStringLiteral("brush_scatter")].toDouble(0.0), 0.0, 2.0);

    QColor tint(181, 140, 255);
    const auto color = state[QStringLiteral("paint_color")].toArray();
    if (color.size() >= 3)
        tint = QColor::fromRgbF(color[0].toDouble(), color[1].toDouble(), color[2].toDouble());
    const QColor ring = erase ? QColor(239, 107, 115) : QColor(255, 255, 255, 221);
    const QColor base = erase ? QColor(239, 107, 115) : tint;

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    QTransform transform;
    transform.translate(style.center.x(), style.center.y());
    transform.rotate(style.tiltDeg);
    transform.scale(1.0, std::clamp(style.squash, 0.05, 1.0));
    painter.setTransform(transform, true);

    if (style.ghost) {
        // Eco: contorno tracejado claro + escuro, sem preenchimento (não se confunde com o cursor).
        QPen light(QColor(255, 255, 255, 200), 1.5, Qt::DashLine);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(light);
        painter.drawEllipse(QPointF(0, 0), radius, radius);
        QPen dark(QColor(0, 0, 0, 150), 1.0, Qt::DashLine);
        dark.setDashOffset(2.0);
        painter.setPen(dark);
        painter.drawEllipse(QPointF(0, 0), std::max(1.0, radius - 1.5), std::max(1.0, radius - 1.5));
        painter.restore();
        return;
    }

    // Dureza e opacidade no preenchimento: cheio até o núcleo, some até a borda.
    // A variação de opacidade clareia o centro (o dab mais fraco possível).
    const double peak = 0.04 + 0.32 * strength;
    const double low = peak * (1.0 - 0.7 * opacityJitter);
    QRadialGradient gradient(QPointF(0, 0), radius);
    QColor inner = base, core = base, edge = base;
    inner.setAlphaF(low);
    core.setAlphaF(peak);
    edge.setAlphaF(hardEdge ? peak : peak * 0.15);
    gradient.setColorAt(0.0, inner);
    gradient.setColorAt(hardEdge ? 0.999 : std::max(0.02, hardness), core);
    gradient.setColorAt(1.0, edge);
    painter.setBrush(gradient);
    painter.setPen(QPen(ring, 1.5));
    painter.drawEllipse(QPointF(0, 0), radius, radius);
    // Contorno escuro interno: lê bem sobre superfícies claras e escuras.
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(QColor(0, 0, 0, 170), 1.0));
    painter.drawEllipse(QPointF(0, 0), std::max(1.0, radius - 1.5), std::max(1.0, radius - 1.5));
    if (!hardEdge && hardness > 0.02 && hardness < 0.98 && diameter > 10.0) {
        painter.setPen(QPen(QColor(255, 255, 255, 178), 1.0));
        painter.drawEllipse(QPointF(0, 0), radius * hardness, radius * hardness);
    }
    // Alcance: dispersão (até onde um dab pode cair) e o menor dab da variação de tamanho.
    if (scatter > 0.02) {
        painter.setPen(QPen(QColor(255, 255, 255, 150), 1.0, Qt::DashLine));
        painter.drawEllipse(QPointF(0, 0), radius * (1.0 + scatter), radius * (1.0 + scatter));
    }
    if (sizeJitter > 0.02 && diameter > 10.0) {
        painter.setPen(QPen(QColor(255, 255, 255, 140), 1.0, Qt::DotLine));
        const double smallest = std::max(1.0, radius * (1.0 - sizeJitter));
        painter.drawEllipse(QPointF(0, 0), smallest, smallest);
    }
    painter.restore();
    // O ponto central marca o cursor e não se deforma.
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(0, 0, 0, 170), 1.0));
    painter.setBrush(QColor(255, 255, 255));
    painter.drawEllipse(style.center, 1.5, 1.5);
    painter.restore();
}

PetuniaBrushMirror &PetuniaBrushMirror::instance() {
    static PetuniaBrushMirror mirror;
    return mirror;
}

void PetuniaBrushMirror::adopt(const QJsonObject &reply) {
    if (!reply.contains(QStringLiteral("brush_mirror")))
        return;
    const auto mirror = reply[QStringLiteral("brush_mirror")].toObject();
    const auto texture = mirror[QStringLiteral("texture")].toArray();
    const auto view = mirror[QStringLiteral("view")].toArray();
    const bool hasTexture = texture.size() >= 2;
    const bool hasView = view.size() >= 2;
    const QPointF t = hasTexture ? QPointF(texture[0].toDouble(), texture[1].toDouble()) : QPointF();
    const QPointF v = hasView ? QPointF(view[0].toDouble(), view[1].toDouble()) : QPointF();
    if (hasTexture == m_hasTexture && hasView == m_hasView && t == m_texture && v == m_view)
        return;
    m_hasTexture = hasTexture;
    m_hasView = hasView;
    m_texture = t;
    m_view = v;
    emit changed();
}

void PetuniaBrushMirror::clear() {
    if (!m_hasTexture && !m_hasView)
        return;
    m_hasTexture = m_hasView = false;
    emit changed();
}
