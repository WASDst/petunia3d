#pragma once

#include <QJsonObject>
#include <QObject>
#include <QPointF>

class QPainter;

/// Desenho único do cursor do pincel (ADR-C16, B2), igual na vista 3D e no canvas 2D:
/// diâmetro do carimbo, dureza como degradê (núcleo cheio → borda suave), opacidade
/// e fluxo na intensidade, variação de opacidade no degradê, e o alcance da dispersão
/// e da variação de tamanho em anéis tracejados. `ghost` desenha o eco do cursor que
/// está na outra superfície (só contorno tracejado, sem preenchimento).
struct PetuniaBrushCursorStyle {
    QPointF center;
    double squash{1.0};
    double tiltDeg{0.0};
    bool ghost{false};
};
void petuniaDrawBrushCursor(QPainter &painter, const QJsonObject &state, const PetuniaBrushCursorStyle &style);

/// Eco do cursor entre as superfícies: quem está sob o ponteiro publica onde o
/// pincel está na outra (fração da textura para o 2D, px da vista para o 3D).
class PetuniaBrushMirror : public QObject {
    Q_OBJECT

  public:
    static PetuniaBrushMirror &instance();

    /// Lê `brush_mirror` de uma resposta do editor e avisa se mudou.
    void adopt(const QJsonObject &reply);
    void clear();
    bool hasTexturePoint() const { return m_hasTexture; }
    QPointF texturePoint() const { return m_texture; }
    bool hasViewPoint() const { return m_hasView; }
    QPointF viewPoint() const { return m_view; }

  signals:
    void changed();

  private:
    PetuniaBrushMirror() = default;
    bool m_hasTexture{false};
    bool m_hasView{false};
    QPointF m_texture;
    QPointF m_view;
};
