#pragma once

#include <QOpenGLWidget>
#include <QOpenGLFunctions>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QTabletEvent>
#include <QKeyEvent>
#include <QPainter>

class PetuniaGlWidget : public QOpenGLWidget, protected QOpenGLFunctions {
    Q_OBJECT

public:
    explicit PetuniaGlWidget(QWidget *parent = nullptr);
    ~PetuniaGlWidget() override;

    void setShadingMode(const QString &mode);
    void setSnapEnabled(bool enabled) { m_snapEnabled = enabled; update(); }
    bool isSnapEnabled() const { return m_snapEnabled; }

signals:
    void cameraOrbitRequested(float deltaX, float deltaY);
    void cameraPanRequested(float deltaX, float deltaY);
    void cameraZoomRequested(float delta);
    void tabletStrokeUpdated(float x, float y, float pressure, float tiltX, float tiltY, bool isEraser);
    void selectionChanged();

protected:
    void initializeGL() override;
    void resizeGL(int w, int h) override;
    void paintGL() override;

    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void tabletEvent(QTabletEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

private:
    void drawNavigationGizmo(QPainter &painter);
    void drawViewportHud(QPainter &painter);
    void drawShadingPill(QPainter &painter);
    void drawTransformGizmo(QPainter &painter);
    void drawSnapIndicator(QPainter &painter);
    bool handleGizmoClick(const QPoint &pos);
    bool handleShadingClick(const QPoint &pos);

    QPoint m_lastMousePos;
    bool m_isInteracting{false};
    bool m_isGizmoInteracting{false};
    bool m_isTransformGizmoDragging{false};
    int m_activeGizmoHandle{0};
    bool m_snapEnabled{false};
    bool m_rendererInitialized{false};

    QPoint m_gizmoCenter{0, 0};
    int m_gizmoRadius{44};
    QRect m_shadingPillRect;
};
