#pragma once

#include <QWidget>
#include <QAccessible>
#include <QtMath>

/// Widget circular de seleção de ângulo para ferramentas DCC (pintura, rotação e transformações).
/// Fornece controle contínuo de 0° a 360°, suporte a snapping (Shift), ajuste fino (Ctrl),
/// rotação por roda do mouse, reset por duplo-clique e acessibilidade completa por teclado.
class PetuniaAngleDial : public QWidget {
    Q_OBJECT
    Q_PROPERTY(float angle READ angle WRITE setAngle NOTIFY angleChanged)

public:
    explicit PetuniaAngleDial(QWidget *parent = nullptr);
    ~PetuniaAngleDial() override = default;

    float angle() const { return m_angle; }
    float snapStep() const { return m_snapStep; }

    QSize sizeHint() const override;
    QSize minimumSizeHint() const override;

public slots:
    void setAngle(float degrees);
    void setSnapStep(float step);

signals:
    void angleChanged(float degrees);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void mouseDoubleClickEvent(QMouseEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void focusInEvent(QFocusEvent *event) override;
    void focusOutEvent(QFocusEvent *event) override;

private:
    void updateAngleFromPoint(const QPointF &pos, bool snap);
    static float normalizeAngle(float degrees);

    float m_angle{0.0f};      // 0.0 a 360.0 graus
    float m_snapStep{15.0f};  // Passo com Shift
    bool m_dragging{false};
};
