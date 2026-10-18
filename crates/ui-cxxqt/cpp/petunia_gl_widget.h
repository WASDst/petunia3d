#pragma once

#include "petunia_editor_api.h"
#include <QKeyEvent>
#include <QMouseEvent>
#include <QOpenGLFunctions>
#include <QOpenGLWidget>
#include <QPainter>
#include <QPolygonF>
#include <QTabletEvent>
#include <QWheelEvent>

class PetuniaNativeGlCanvas;
class PetuniaGlWidget : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaGlWidget(QWidget *parent = nullptr);
    ~PetuniaGlWidget() override;

    void update();
    QString renderingBackend() const {
        return m_software              ? QStringLiteral("cpu")
               : m_rendererInitialized ? QStringLiteral("opengl")
                                       : QStringLiteral("uninitialized");
    }
    void setSurfaceId(uint64_t id) { m_surfaceId = id; }
    void setShadingMode(const QString &mode);
    void setSnapEnabled(bool enabled) {
        m_snapEnabled = enabled;
        update();
    }
    bool isSnapEnabled() const { return m_snapEnabled; }
    /// Shell v2: o cromo da vista (rótulo da vista, pílula de sombreamento) vive no
    /// cabeçalho do shell e o estado vem do controlador (sem snapshot por quadro).
    void setShellChrome(bool enabled) { m_shellChrome = enabled; }
    void setExternalState(const QJsonObject &state) {
        m_externalState = state;
        m_hasExternalState = true;
    }
    void setZoomSensitivity(float s) { m_zoomSensitivity = qBound(0.1f, s, 5.0f); }
    float zoomSensitivity() const { return m_zoomSensitivity; }

  signals:
    void cameraOrbitRequested(float deltaX, float deltaY);
    void cameraPanRequested(float deltaX, float deltaY);
    void cameraZoomRequested(float delta);
    void tabletStrokeUpdated(float x, float y, float pressure, float tiltX,
                             float tiltY, bool isEraser);
    void selectionChanged();

  protected:
    void paintEvent(QPaintEvent *) override;
    void showEvent(QShowEvent *) override;
    void resizeEvent(QResizeEvent *) override;

    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void leaveEvent(QEvent *event) override;
    void enterEvent(QEnterEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void tabletEvent(QTabletEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
    void focusOutEvent(QFocusEvent *event) override;

  private:
    /// Encerra um gesto interrompido (foco perdido ou Escape): traço de pintura
    /// confirma ou cancela conforme `commitPaint`; gesto geométrico cancela.
    void finishInterruptedGesture(bool commitPaint);
    friend class PetuniaNativeGlCanvas;
    void initializeGL();
    void resizeGL(int w, int h);
    void paintGL(uint32_t fbo, QPaintDevice *device);
    void drawOverlays(QPainter &painter);
    PetuniaNativeGlCanvas *m_gl{nullptr};
    bool m_software{false};
    void drawNavigationGizmo(QPainter &painter);
    void drawViewportHud(QPainter &painter);
    void drawShadingPill(QPainter &painter);
    void drawTransformGizmo(QPainter &painter);
    void drawSnapIndicator(QPainter &painter);
    void drawSelectionOutline(QPainter &painter);
    void draw3dCursor(QPainter &painter);
    /// Anel do pincel de pintura: mesmo diâmetro (px de tela) que o carimbo,
    /// com o achatamento e o giro da superfície sob o cursor.
    void drawBrushCursor(QPainter &painter);
    /// Alças do decalque ativo (mover/escalar/girar) desenhadas sobre a malha.
    void drawDecalGizmo(QPainter &painter);
    void updatePointerFeedback(const QPointF &position, bool navigating);
    bool handleGizmoClick(const QPoint &pos);
    bool handleShadingClick(const QPoint &pos);

    void cleanupGl();
    QJsonObject pointer(int phase, const QPointF &, Qt::KeyboardModifiers);
    void refreshOverlayState();
    uint64_t m_surfaceId{1};
    QJsonObject m_overlayState;
    QJsonObject m_externalState;
    bool m_hasExternalState{false};
    bool m_shellChrome{false};
    QPointF m_pressPoint;
    QPolygonF m_regionPoints;
    bool m_toolPressed{false}, m_regionSelecting{false}, m_lasso{false};
    QPoint m_lastMousePos;
    bool m_pointerInside{false};
    bool m_brushCursorVisible{false};
    float m_brushSquash{1.0f};
    float m_brushTilt{0.0f};
    bool m_isInteracting{false};
    bool m_isGizmoInteracting{false};
    bool m_isTransformGizmoDragging{false};
    int m_activeGizmoHandle{0};
    bool m_snapEnabled{false};
    bool m_rendererInitialized{false};

    QPoint m_gizmoCenter{0, 0};
    int m_gizmoRadius{44};
    QRect m_shadingPillRect;
    float m_zoomSensitivity{1.0f};
};
