#include "petunia_nav_cube.h"
#include "petunia_theme.h"
#include "shell/petunia_i18n.h"

#include <QFont>
#include <QLinearGradient>
#include <QPainterPath>
#include <QPolygonF>
#include <QTransform>

#include <algorithm>
#include <array>
#include <cmath>

namespace {
constexpr double kPi = 3.14159265358979323846;
constexpr double kHalfPi = kPi / 2.0;
constexpr double kHalf = 1.0;          // meia-aresta do cubo (unidades do cubo)
constexpr double kRingOuter = 2.3;     // raio externo do anel de bússola
constexpr double kRingInner = 1.78;    // raio interno
constexpr double kRingY = -1.55;       // plano do anel, abaixo do cubo
constexpr int kRingSteps = 64;
constexpr double kCornerHit = 8.0;     // px
constexpr double kEdgeHit = 5.5;       // px
constexpr double kVisibleDepth = 0.04; // limiar de face voltada para a câmera

struct FaceDef {
    QVector3D normal;
    QVector3D right;
    QVector3D up;
    int preset;
};

const std::array<FaceDef, 6> &faceDefs() {
    static const std::array<FaceDef, 6> faces{{
        {{0, 0, 1}, {1, 0, 0}, {0, 1, 0}, 1},   // Frente (+Z)
        {{0, 0, -1}, {-1, 0, 0}, {0, 1, 0}, 2}, // Trás (-Z)
        {{1, 0, 0}, {0, 0, -1}, {0, 1, 0}, 3},  // Direita (+X)
        {{-1, 0, 0}, {0, 0, 1}, {0, 1, 0}, 4},  // Esquerda (-X)
        {{0, 1, 0}, {1, 0, 0}, {0, 0, -1}, 5},  // Topo (+Y)
        {{0, -1, 0}, {1, 0, 0}, {0, 0, 1}, 6},  // Base (-Y)
    }};
    return faces;
}

struct EdgeDef {
    QVector3D a;
    QVector3D b;
    QVector3D n1;
    QVector3D n2;
    QString id;
};

const std::array<EdgeDef, 12> &edgeDefs() {
    static const std::array<EdgeDef, 12> edges = [] {
        std::array<EdgeDef, 12> out;
        int index = 0;
        for (int axis = 0; axis < 3; ++axis) {
            const int i = (axis + 1) % 3;
            const int j = (axis + 2) % 3;
            for (int s1 = -1; s1 <= 1; s1 += 2) {
                for (int s2 = -1; s2 <= 1; s2 += 2) {
                    float lo[3]{}, hi[3]{}, n1[3]{}, n2[3]{};
                    lo[axis] = -float(kHalf);
                    hi[axis] = float(kHalf);
                    lo[i] = hi[i] = float(s1) * float(kHalf);
                    lo[j] = hi[j] = float(s2) * float(kHalf);
                    n1[i] = float(s1);
                    n2[j] = float(s2);
                    out[size_t(index)] = {QVector3D(lo[0], lo[1], lo[2]), QVector3D(hi[0], hi[1], hi[2]),
                                          QVector3D(n1[0], n1[1], n1[2]), QVector3D(n2[0], n2[1], n2[2]),
                                          QStringLiteral("edge:%1").arg(index)};
                    ++index;
                }
            }
        }
        return out;
    }();
    return edges;
}

struct CornerDef {
    QVector3D position;
    QString id;
};

const std::array<CornerDef, 8> &cornerDefs() {
    static const std::array<CornerDef, 8> corners = [] {
        std::array<CornerDef, 8> out;
        int index = 0;
        for (int sx = -1; sx <= 1; sx += 2)
            for (int sy = -1; sy <= 1; sy += 2)
                for (int sz = -1; sz <= 1; sz += 2) {
                    out[size_t(index)] = {QVector3D(float(sx), float(sy), float(sz)) * float(kHalf),
                                          QStringLiteral("corner:%1").arg(index)};
                    ++index;
                }
        return out;
    }();
    return corners;
}

double distanceToSegment(const QPointF &p, const QPointF &a, const QPointF &b) {
    const QPointF ab = b - a;
    const double lengthSquared = ab.x() * ab.x() + ab.y() * ab.y();
    if (lengthSquared < 1e-9)
        return std::hypot(p.x() - a.x(), p.y() - a.y());
    const QPointF ap = p - a;
    const double t = std::clamp((ap.x() * ab.x() + ap.y() * ab.y()) / lengthSquared, 0.0, 1.0);
    const QPointF closest = a + ab * t;
    return std::hypot(p.x() - closest.x(), p.y() - closest.y());
}

QString faceTitle(int preset) {
    switch (preset) {
    case 1:
        return petuniaTr(QStringLiteral("FRENTE"));
    case 2:
        return petuniaTr(QStringLiteral("TRÁS"));
    case 3:
        return petuniaTr(QStringLiteral("DIREITA"));
    case 4:
        return petuniaTr(QStringLiteral("ESQUERDA"));
    case 5:
        return petuniaTr(QStringLiteral("TOPO"));
    default:
        return petuniaTr(QStringLiteral("BASE"));
    }
}
} // namespace

QString PetuniaNavCube::faceLabel(int preset) { return faceTitle(preset); }

void PetuniaNavCube::setCamera(const QVector3D &right, const QVector3D &up, const QVector3D &forward) {
    m_right = right;
    m_up = up;
    m_forward = forward;
}

void PetuniaNavCube::setGeometry(const QPointF &center, double unit) {
    m_center = center;
    m_unit = unit;
}

QPointF PetuniaNavCube::project(const QVector3D &world) const {
    return m_center + QPointF(QVector3D::dotProduct(world, m_right), -QVector3D::dotProduct(world, m_up)) * m_unit;
}

double PetuniaNavCube::depth(const QVector3D &world) const { return -QVector3D::dotProduct(world, m_forward); }

QPolygonF PetuniaNavCube::facePolygon(const Face &face) const {
    QPolygonF polygon;
    const QVector3D n = face.normal * float(kHalf);
    const QVector3D a = face.right * float(kHalf);
    const QVector3D b = face.up * float(kHalf);
    polygon << project(n - a - b) << project(n + a - b) << project(n + a + b) << project(n - a + b);
    return polygon;
}

QRectF PetuniaNavCube::bounds() const {
    const double radius = kRingOuter * m_unit + 14.0;
    return QRectF(m_center.x() - radius, m_center.y() - radius, radius * 2.0, radius * 2.0);
}

bool PetuniaNavCube::contains(const QPointF &position) const {
    const double radius = kRingOuter * m_unit + 14.0;
    return std::hypot(position.x() - m_center.x(), position.y() - m_center.y()) <= radius;
}

QRectF PetuniaNavCube::homeRect() const {
    const double edge = kRingOuter * m_unit + 14.0;
    return QRectF(m_center.x() - edge - 2.0, m_center.y() - edge - 2.0, 20.0, 20.0);
}

QRectF PetuniaNavCube::menuRect() const {
    const double edge = kRingOuter * m_unit + 14.0;
    return QRectF(m_center.x() + edge - 18.0, m_center.y() + edge - 18.0, 20.0, 20.0);
}

QPointF PetuniaNavCube::compassPoint(int index) const {
    const double radius = (kRingOuter + kRingInner) * 0.5;
    // N = -Z, L = +X, S = +Z, O = -X.
    static const double angles[4] = {-kHalfPi, 0.0, kHalfPi, kPi};
    const double a = angles[index % 4];
    return project(QVector3D(float(std::cos(a) * radius), float(kRingY), float(std::sin(a) * radius)));
}

PetuniaNavCube::Hit PetuniaNavCube::hitTest(const QPointF &position) const {
    Hit hit;
    if (homeRect().adjusted(-2, -2, 2, 2).contains(position)) {
        hit.type = Hit::Type::Home;
        hit.id = QStringLiteral("home");
        return hit;
    }
    if (menuRect().adjusted(-2, -2, 2, 2).contains(position)) {
        hit.type = Hit::Type::Menu;
        hit.id = QStringLiteral("menu");
        return hit;
    }
    if (!contains(position))
        return hit;

    // Cantos primeiro: alvo pequeno, sobrepõe arestas e faces. Em vistas
    // alinhadas aos eixos o canto da frente e o de trás caem no mesmo pixel: vence
    // o mais próximo da câmera (empate de até 0,75 px na distância).
    {
        double bestDistance = kCornerHit;
        double bestDepth = -1.0e9;
        for (const auto &corner : cornerDefs()) {
            const QVector3D &p = corner.position;
            const bool visible = depth(QVector3D(p.x(), 0, 0)) > kVisibleDepth ||
                                 depth(QVector3D(0, p.y(), 0)) > kVisibleDepth ||
                                 depth(QVector3D(0, 0, p.z())) > kVisibleDepth;
            if (!visible)
                continue;
            const QPointF screen = project(p);
            const double distance = std::hypot(position.x() - screen.x(), position.y() - screen.y());
            const double nearness = depth(p);
            if (distance > kCornerHit)
                continue;
            const bool closer = distance < bestDistance - 0.75;
            const bool tie = std::abs(distance - bestDistance) <= 0.75 && nearness > bestDepth;
            if (hit.type == Hit::Type::None || closer || tie) {
                bestDistance = std::min(bestDistance, distance);
                bestDepth = nearness;
                hit.type = Hit::Type::Direction;
                hit.direction = p;
                hit.id = corner.id;
            }
        }
        if (hit.type != Hit::Type::None)
            return hit;
    }

    {
        double bestDistance = kEdgeHit;
        double bestDepth = -1.0e9;
        for (const auto &edge : edgeDefs()) {
            if (depth(edge.n1) <= kVisibleDepth && depth(edge.n2) <= kVisibleDepth)
                continue;
            const double distance = distanceToSegment(position, project(edge.a), project(edge.b));
            if (distance > kEdgeHit)
                continue;
            const double nearness = depth((edge.a + edge.b) * 0.5f);
            const bool closer = distance < bestDistance - 0.75;
            const bool tie = std::abs(distance - bestDistance) <= 0.75 && nearness > bestDepth;
            if (hit.type == Hit::Type::None || closer || tie) {
                bestDistance = std::min(bestDistance, distance);
                bestDepth = nearness;
                hit.type = Hit::Type::Direction;
                hit.direction = edge.n1 + edge.n2;
                hit.id = edge.id;
            }
        }
        if (hit.type != Hit::Type::None)
            return hit;
    }

    for (const auto &def : faceDefs()) {
        const Face face{def.normal, def.right, def.up, def.preset};
        if (!faceVisible(face))
            continue;
        if (facePolygon(face).containsPoint(position, Qt::OddEvenFill)) {
            hit.type = Hit::Type::Preset;
            hit.preset = def.preset;
            hit.id = QStringLiteral("face:%1").arg(def.preset);
            return hit;
        }
    }

    // Letras do anel de bússola.
    static const double yaws[4] = {180.0, 90.0, 0.0, -90.0};
    for (int i = 0; i < 4; ++i) {
        const QPointF letter = compassPoint(i);
        if (std::hypot(position.x() - letter.x(), position.y() - letter.y()) <= 9.0) {
            hit.type = Hit::Type::Yaw;
            hit.yawDegrees = yaws[i];
            hit.id = QStringLiteral("compass:%1").arg(i);
            return hit;
        }
    }
    return hit;
}

void PetuniaNavCube::paintFace(QPainter &painter, const Face &face, bool hovered) const {
    const auto &theme = PetuniaTheme::instance();
    const QPolygonF polygon = facePolygon(face);

    // Luz fixa em relação à câmera (canto superior esquerdo): o cubo gira, a luz não.
    const QVector3D light = (-m_right * 0.35f + m_up * 0.6f - m_forward * 0.7f).normalized();
    const double lit = std::max(0.0, double(QVector3D::dotProduct(face.normal, light)));
    QColor base = theme.bgSurfaceActive();
    const QColor top = theme.textPrimary();
    const double mix = 0.12 + 0.28 * lit;
    base = QColor::fromRgbF(base.redF() * (1.0 - mix) + top.redF() * mix, base.greenF() * (1.0 - mix) + top.greenF() * mix,
                            base.blueF() * (1.0 - mix) + top.blueF() * mix);
    if (hovered)
        base = theme.accentPrimary();

    painter.setPen(QPen(theme.borderStrong(), 1.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(base);
    painter.drawPolygon(polygon);

    // Rótulo no plano da face: o transform leva o quadrado unitário da face à tela.
    const QPointF center = project(face.normal * float(kHalf));
    const QPointF rightScreen = project(face.normal * float(kHalf) + face.right) - center;
    const QPointF upScreen = project(face.normal * float(kHalf) + face.up) - center;
    QTransform onFace(rightScreen.x(), rightScreen.y(), -upScreen.x(), -upScreen.y(), center.x(), center.y());

    QFont font = painter.font();
    font.setPixelSize(100);
    font.setBold(true);
    QPainterPath text;
    text.addText(0, 0, font, faceTitle(face.preset));
    const QRectF box = text.boundingRect();
    if (box.width() <= 0.0)
        return;
    const double scale = std::min(0.0036, 1.46 / box.width());
    QTransform fit;
    fit.scale(scale, scale);
    fit.translate(-box.center().x(), -box.center().y());
    painter.save();
    painter.setTransform(onFace, true);
    painter.setPen(Qt::NoPen);
    painter.setBrush(hovered ? theme.textActive() : theme.textPrimary());
    painter.drawPath(fit.map(text));
    painter.restore();
}

void PetuniaNavCube::paintCompass(QPainter &painter, const Hit &hover) const {
    const auto &theme = PetuniaTheme::instance();
    QPolygonF outer;
    QPolygonF inner;
    for (int i = 0; i <= kRingSteps; ++i) {
        const double a = double(i) * 2.0 * kPi / kRingSteps;
        outer << project(QVector3D(float(std::cos(a) * kRingOuter), float(kRingY), float(std::sin(a) * kRingOuter)));
        inner << project(QVector3D(float(std::cos(a) * kRingInner), float(kRingY), float(std::sin(a) * kRingInner)));
    }
    QPainterPath ring;
    ring.setFillRule(Qt::OddEvenFill);
    ring.addPolygon(outer);
    ring.addPolygon(inner);
    QColor fill = theme.bgSurface();
    fill.setAlpha(190);
    painter.setPen(QPen(theme.borderSubtle(), 1.0));
    painter.setBrush(fill);
    painter.drawPath(ring);

    QFont font = painter.font();
    font.setPixelSize(std::max(10, int(m_unit * 0.66)));
    font.setBold(true);
    painter.setFont(font);
    const QString letters[4] = {petuniaTr(QStringLiteral("N")), petuniaTr(QStringLiteral("L")),
                                petuniaTr(QStringLiteral("S")), petuniaTr(QStringLiteral("O"))};
    for (int i = 0; i < 4; ++i) {
        const QPointF p = compassPoint(i);
        const bool hovered = hover.type == Hit::Type::Yaw && hover.id == QStringLiteral("compass:%1").arg(i);
        painter.setPen(hovered ? theme.accentPrimary() : (i == 0 ? theme.statusError() : theme.textSecondary()));
        painter.drawText(QRectF(p.x() - 9, p.y() - 9, 18, 18), Qt::AlignCenter, letters[i]);
    }
}

void PetuniaNavCube::paint(QPainter &painter, const Hit &hover, bool active) const {
    const auto &theme = PetuniaTheme::instance();
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setRenderHint(QPainter::TextAntialiasing, true);
    painter.setOpacity(active ? 1.0 : 0.82);

    paintCompass(painter, hover);

    for (const auto &def : faceDefs()) {
        const Face face{def.normal, def.right, def.up, def.preset};
        if (!faceVisible(face))
            continue;
        const bool hovered = hover.type == Hit::Type::Preset && hover.preset == def.preset;
        paintFace(painter, face, hovered);
    }

    // Realce de aresta/canto sob o ponteiro.
    if (hover.type == Hit::Type::Direction) {
        painter.setBrush(Qt::NoBrush);
        for (const auto &edge : edgeDefs()) {
            if (edge.id == hover.id) {
                painter.setPen(QPen(theme.accentPrimary(), 3.5, Qt::SolidLine, Qt::RoundCap));
                painter.drawLine(project(edge.a), project(edge.b));
            }
        }
        for (const auto &corner : cornerDefs()) {
            if (corner.id == hover.id) {
                painter.setPen(QPen(theme.textActive(), 1.0));
                painter.setBrush(theme.accentPrimary());
                painter.drawEllipse(project(corner.position), 5.0, 5.0);
            }
        }
    }

    // Início (casa) e menu de projeção.
    const QRectF home = homeRect();
    const bool homeHover = hover.type == Hit::Type::Home;
    if (homeHover) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(theme.bgSurfaceHover());
        painter.drawRoundedRect(home, 5, 5);
    }
    painter.setPen(QPen(homeHover ? theme.textPrimary() : theme.textSecondary(), 1.6, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(Qt::NoBrush);
    QPainterPath house;
    const QPointF h = home.center();
    house.moveTo(h.x() - 6.5, h.y() + 0.5);
    house.lineTo(h.x(), h.y() - 6.0);
    house.lineTo(h.x() + 6.5, h.y() + 0.5);
    house.moveTo(h.x() - 4.5, h.y() - 0.5);
    house.lineTo(h.x() - 4.5, h.y() + 6.0);
    house.lineTo(h.x() + 4.5, h.y() + 6.0);
    house.lineTo(h.x() + 4.5, h.y() - 0.5);
    painter.drawPath(house);

    const QRectF menu = menuRect();
    const bool menuHover = hover.type == Hit::Type::Menu;
    if (menuHover) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(theme.bgSurfaceHover());
        painter.drawRoundedRect(menu, 5, 5);
    }
    painter.setPen(Qt::NoPen);
    painter.setBrush(menuHover ? theme.textPrimary() : theme.textSecondary());
    const QPointF m = menu.center();
    QPolygonF triangle;
    triangle << QPointF(m.x() - 5, m.y() - 2.5) << QPointF(m.x() + 5, m.y() - 2.5) << QPointF(m.x(), m.y() + 4);
    painter.drawPolygon(triangle);

    // Etiqueta da projeção abaixo do anel.
    QFont font = painter.font();
    font.setPixelSize(std::max(9, int(m_unit * 0.58)));
    font.setBold(false);
    painter.setFont(font);
    painter.setPen(theme.textMuted());
    const QRectF label(m_center.x() - 40, m_center.y() + kRingOuter * m_unit + 4.0, 80, 14);
    painter.drawText(label, Qt::AlignCenter,
                     m_ortho ? petuniaTr(QStringLiteral("Ortográfica")) : petuniaTr(QStringLiteral("Perspectiva")));
    painter.restore();
}
