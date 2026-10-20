#pragma once

#include <QPainter>
#include <QPointF>
#include <QRectF>
#include <QString>
#include <QVector3D>

/// Cubo de navegação da viewport: cubo 3D com as faces rotuladas, arestas e
/// cantos clicáveis, anel de bússola (N/L/S/O), atalho "início" e menu de
/// projeção. Só desenha e responde a acertos; a câmera é do núcleo (os pedidos
/// saem como `Hit` para o widget traduzir em chamadas à ponte).
///
/// Convenção de eixos do projeto: Y para cima, a vista Frente olha o rosto +Z,
/// Direita olha +X, Topo olha +Y.
class PetuniaNavCube {
  public:
    struct Hit {
        enum class Type { None, Preset, Direction, Yaw, Home, Menu };
        Type type{Type::None};
        /// Preset de câmera quando `type == Preset` (1 Frente … 6 Base).
        int preset{0};
        /// Direção do alvo até o olho quando `type == Direction`.
        QVector3D direction;
        /// Azimute em graus quando `type == Yaw`.
        double yawDegrees{0.0};
        /// Identidade estável do alvo (realce): "face:+z", "edge:+x+y", "corner:+x+y+z"…
        QString id;
    };

    /// Base da câmera (direita, cima, para a frente) e a posição do centro.
    void setCamera(const QVector3D &right, const QVector3D &up, const QVector3D &forward);
    void setGeometry(const QPointF &center, double unit);
    void setOrtho(bool ortho) { m_ortho = ortho; }

    /// Retângulo que contém tudo o que o cubo desenha (para hit de região).
    QRectF bounds() const;
    bool contains(const QPointF &position) const;

    Hit hitTest(const QPointF &position) const;
    void paint(QPainter &painter, const Hit &hover, bool active) const;

    static QString faceLabel(int preset);

  private:
    struct Face {
        QVector3D normal;
        QVector3D right;
        QVector3D up;
        int preset;
    };
    QPointF project(const QVector3D &world) const;
    /// > 0 quando o ponto olha para a câmera.
    double depth(const QVector3D &world) const;
    bool faceVisible(const Face &face) const { return depth(face.normal) > 0.04; }
    QPolygonF facePolygon(const Face &face) const;
    QRectF homeRect() const;
    QRectF menuRect() const;
    QPointF compassPoint(int index) const;
    void paintCompass(QPainter &painter, const Hit &hover) const;
    void paintFace(QPainter &painter, const Face &face, bool hovered) const;

    QVector3D m_right{1, 0, 0};
    QVector3D m_up{0, 1, 0};
    QVector3D m_forward{0, 0, -1};
    QPointF m_center;
    double m_unit{17.0};
    bool m_ortho{false};
};
