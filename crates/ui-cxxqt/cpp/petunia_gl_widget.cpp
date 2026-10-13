#include "petunia_gl_widget.h"
#include <QOpenGLContext>
#include <QDebug>
#include <vector>
#include <algorithm>
#include <cmath>

// Declarado no lado Rust (crates/ui-cxxqt/src/render/opengl_viewport.rs)
extern "C" {
    typedef void* (*QtGlGetProcAddressFn)(const char*);
    bool petunia_qt_gl_initialize(QtGlGetProcAddressFn get_proc);
    void petunia_qt_gl_resize(int width, int height);
    void petunia_qt_gl_paint(uint32_t fbo_id, int width, int height);
    void petunia_qt_camera_orbit(float dx, float dy);
    void petunia_qt_camera_pan(float dx, float dy);
    void petunia_qt_camera_zoom(float delta);
    void petunia_qt_camera_reset();
    void petunia_qt_camera_set_preset(int preset);
    void petunia_qt_camera_toggle_projection();
    bool petunia_qt_camera_get_view_name(char *buf, int max_len);
    bool petunia_qt_camera_get_orientation(
        float *rx, float *ry, float *rz,
        float *ux, float *uy, float *uz,
        float *fx, float *fy, float *fz
    );
    void petunia_qt_app_frame_selection();
    int petunia_qt_app_get_shading();
    void petunia_qt_app_set_shading(int shading);
    void petunia_qt_tablet_paint_stroke(float x, float y, float pressure, float tilt_x, float tilt_y, bool is_eraser);
    bool petunia_qt_viewport_select(float screen_x, float screen_y, float viewport_w, float viewport_h, bool extend);
    bool petunia_qt_viewport_hover(float screen_x, float screen_y, float viewport_w, float viewport_h);
    bool petunia_qt_gizmo_get_projected(float viewport_w, float viewport_h, float *out_origin, float *out_x, float *out_y, float *out_z);
    int petunia_qt_gizmo_hit_test(float mouse_x, float mouse_y, float viewport_w, float viewport_h);
    bool petunia_qt_gizmo_drag_axis(int axis, float dx, float dy, float viewport_w, float viewport_h);
}

static void* qt_gl_get_proc_address(const char* name) {
    auto* ctx = QOpenGLContext::currentContext();
    if (!ctx) return nullptr;
    return reinterpret_cast<void*>(ctx->getProcAddress(QByteArray(name)));
}

PetuniaGlWidget::PetuniaGlWidget(QWidget *parent)
    : QOpenGLWidget(parent)
{
    QSurfaceFormat fmt;
    fmt.setVersion(3, 3);
    fmt.setProfile(QSurfaceFormat::CoreProfile);
    setFormat(fmt);

    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
}

PetuniaGlWidget::~PetuniaGlWidget() = default;

void PetuniaGlWidget::setShadingMode(const QString &mode) {
    if (mode == QStringLiteral("wireframe")) {
        petunia_qt_app_set_shading(0);
    } else if (mode == QStringLiteral("solid")) {
        petunia_qt_app_set_shading(1);
    } else if (mode == QStringLiteral("material")) {
        petunia_qt_app_set_shading(2);
    } else if (mode == QStringLiteral("rendered")) {
        petunia_qt_app_set_shading(3);
    }
    update();
}

void PetuniaGlWidget::initializeGL() {
    auto* ctx = QOpenGLContext::currentContext();
    if (!ctx) {
        qWarning() << "[PetuniaGlWidget] Sem contexto OpenGL ativo no QPA atual.";
        return;
    }
    initializeOpenGLFunctions();
    m_rendererInitialized = petunia_qt_gl_initialize(&qt_gl_get_proc_address);
    if (!m_rendererInitialized) {
        qWarning() << "[PetuniaGlWidget] Falha ao inicializar renderer OpenGL Rust.";
    }
}

void PetuniaGlWidget::resizeGL(int w, int h) {
    if (m_rendererInitialized) {
        petunia_qt_gl_resize(w, h);
    }
}

void PetuniaGlWidget::paintGL() {
    auto* ctx = QOpenGLContext::currentContext();
    if (!ctx || !m_rendererInitialized) {
        return;
    }

    // 1. Renderiza a cena 3D Rust via framebuffer blit
    GLuint fbo = defaultFramebufferObject();
    petunia_qt_gl_paint(fbo, width(), height());

    // 2. Sobrepõe overlays interativos 2D fiéis via QPainter nativo
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setRenderHint(QPainter::TextAntialiasing);

    drawViewportHud(painter);
    drawShadingPill(painter);
    drawNavigationGizmo(painter);
    drawTransformGizmo(painter);
    drawSnapIndicator(painter);

    painter.end();
}

void PetuniaGlWidget::drawViewportHud(QPainter &painter) {
    char buf[64] = {0};
    QString viewName = QStringLiteral("User Persp");
    if (petunia_qt_camera_get_view_name(buf, sizeof(buf))) {
        viewName = QString::fromUtf8(buf);
    }

    QFont font = painter.font();
    font.setPointSize(10);
    font.setBold(true);
    painter.setFont(font);

    QFontMetrics fm(font);
    int textWidth = fm.horizontalAdvance(viewName);
    QRect badgeRect(12, 12, textWidth + 18, 26);

    // Fundo do badge
    painter.setPen(QColor(64, 64, 64, 180));
    painter.setBrush(QColor(24, 24, 24, 200));
    painter.drawRoundedRect(badgeRect, 4, 4);

    // Texto da vista
    painter.setPen(QColor(181, 140, 255)); // Roxo accent Petunia
    painter.drawText(badgeRect, Qt::AlignCenter, viewName);

    // Sub-legenda de status
    font.setPointSize(8);
    font.setBold(false);
    painter.setFont(font);
    painter.setPen(QColor(140, 140, 140));
    painter.drawText(14, 52, QStringLiteral("OpenGL 3.3 Core • MSAA"));
}

void PetuniaGlWidget::drawShadingPill(QPainter &painter) {
    const int pillWidth = 144;
    const int pillHeight = 28;
    const int pillX = width() - (m_gizmoRadius * 2) - pillWidth - 36;
    const int pillY = 14;
    m_shadingPillRect = QRect(pillX, pillY, pillWidth, pillHeight);

    // Fundo pílula
    painter.setPen(QColor(64, 64, 64, 180));
    painter.setBrush(QColor(24, 24, 24, 200));
    painter.drawRoundedRect(m_shadingPillRect, 14, 14);

    int currentShading = petunia_qt_app_get_shading();
    const QString icons[4] = {
        QStringLiteral("○"), // Wireframe
        QStringLiteral("◐"), // Solid
        QStringLiteral("●"), // Material
        QStringLiteral("◉")  // Rendered
    };

    int segWidth = pillWidth / 4;
    QFont font = painter.font();
    font.setPointSize(11);
    painter.setFont(font);

    for (int i = 0; i < 4; ++i) {
        QRect segRect(pillX + i * segWidth, pillY, segWidth, pillHeight);
        if (i == currentShading) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(181, 140, 255)); // Roxo accent Petunia
            painter.drawEllipse(segRect.center(), 11, 11);
            painter.setPen(QColor(18, 18, 18));
        } else {
            painter.setPen(QColor(180, 180, 180));
        }
        painter.drawText(segRect, Qt::AlignCenter, icons[i]);
    }
}

bool PetuniaGlWidget::handleShadingClick(const QPoint &pos) {
    if (!m_shadingPillRect.contains(pos)) {
        return false;
    }
    int segWidth = m_shadingPillRect.width() / 4;
    if (segWidth <= 0) return false;
    int seg = (pos.x() - m_shadingPillRect.x()) / segWidth;
    seg = qBound(0, seg, 3);
    petunia_qt_app_set_shading(seg);
    update();
    return true;
}

struct AxisRenderInfo {
    float dirX, dirY, dirZ;
    float screenX, screenY, depth;
    QColor color;
    QString label;
    int preset;
    bool isPositive;
};

void PetuniaGlWidget::drawNavigationGizmo(QPainter &painter) {
    m_gizmoCenter = QPoint(width() - 54, 54);
    const int radius = m_gizmoRadius;

    // Fundo circular do Gizmo
    painter.setPen(QColor(55, 55, 55, 120));
    painter.setBrush(QColor(25, 25, 25, 140));
    painter.drawEllipse(m_gizmoCenter, radius, radius);

    float rx = 1.0f, ry = 0.0f, rz = 0.0f;
    float ux = 0.0f, uy = 1.0f, uz = 0.0f;
    float fx = 0.0f, fy = 0.0f, fz = 1.0f;

    if (!petunia_qt_camera_get_orientation(&rx, &ry, &rz, &ux, &uy, &uz, &fx, &fy, &fz)) {
        return;
    }

    const float armLength = 26.0f;
    std::vector<AxisRenderInfo> axes = {
        { 1.0f, 0.0f, 0.0f, 0, 0, 0, QColor(255, 110, 110), QStringLiteral("X"), 3, true },   // +X (Right)
        {-1.0f, 0.0f, 0.0f, 0, 0, 0, QColor(140, 45, 45),   QStringLiteral(""),  4, false },  // -X (Left)
        { 0.0f, 1.0f, 0.0f, 0, 0, 0, QColor(105, 219, 124), QStringLiteral("Y"), 2, true },   // +Y (Back)
        { 0.0f,-1.0f, 0.0f, 0, 0, 0, QColor(45, 120, 55),   QStringLiteral(""),  1, false },  // -Y (Front)
        { 0.0f, 0.0f, 1.0f, 0, 0, 0, QColor(77, 171, 247),  QStringLiteral("Z"), 5, true },   // +Z (Top)
        { 0.0f, 0.0f,-1.0f, 0, 0, 0, QColor(35, 80, 140),   QStringLiteral(""),  6, false }   // -Z (Bottom)
    };

    // Calcula projeção na tela de cada eixo
    for (auto &ax : axes) {
        float sx = ax.dirX * rx + ax.dirY * ry + ax.dirZ * rz;
        float sy = -(ax.dirX * ux + ax.dirY * uy + ax.dirZ * uz);
        float sz = ax.dirX * fx + ax.dirY * fy + ax.dirZ * fz;

        ax.screenX = m_gizmoCenter.x() + sx * armLength;
        ax.screenY = m_gizmoCenter.y() + sy * armLength;
        ax.depth = sz;
    }

    // Ordena pelo depth (do mais distante para o mais próximo da tela)
    std::sort(axes.begin(), axes.end(), [](const AxisRenderInfo &a, const AxisRenderInfo &b) {
        return a.depth < b.depth;
    });

    QFont font = painter.font();
    font.setPointSize(8);
    font.setBold(true);
    painter.setFont(font);

    // Desenha linhas e esferas do Gizmo
    for (const auto &ax : axes) {
        QPointF pEnd(ax.screenX, ax.screenY);

        if (ax.isPositive) {
            // Linha da origem até a esfera
            QPen pen(ax.color, 2.5);
            painter.setPen(pen);
            painter.drawLine(m_gizmoCenter, pEnd);

            // Esfera de ponta com rótulo
            painter.setPen(Qt::NoPen);
            painter.setBrush(ax.color);
            painter.drawEllipse(pEnd, 7.5, 7.5);

            painter.setPen(QColor(18, 18, 18));
            painter.drawText(QRectF(pEnd.x() - 7.5, pEnd.y() - 7.5, 15, 15), Qt::AlignCenter, ax.label);
        } else {
            // Eixo negativo: ponto pequeno
            painter.setPen(Qt::NoPen);
            painter.setBrush(ax.color);
            painter.drawEllipse(pEnd, 4.0, 4.0);
        }
    }
}

bool PetuniaGlWidget::handleGizmoClick(const QPoint &pos) {
    int dx = pos.x() - m_gizmoCenter.x();
    int dy = pos.y() - m_gizmoCenter.y();
    float dist = std::sqrt(static_cast<float>(dx * dx + dy * dy));

    if (dist > m_gizmoRadius) {
        return false;
    }

    float rx = 1.0f, ry = 0.0f, rz = 0.0f;
    float ux = 0.0f, uy = 1.0f, uz = 0.0f;
    float fx = 0.0f, fy = 0.0f, fz = 1.0f;
    if (!petunia_qt_camera_get_orientation(&rx, &ry, &rz, &ux, &uy, &uz, &fx, &fy, &fz)) {
        return false;
    }

    const float armLength = 26.0f;
    std::vector<AxisRenderInfo> axes = {
        { 1.0f, 0.0f, 0.0f, 0, 0, 0, QColor(), QString(), 3, true },   // Right
        {-1.0f, 0.0f, 0.0f, 0, 0, 0, QColor(), QString(), 4, false },  // Left
        { 0.0f, 1.0f, 0.0f, 0, 0, 0, QColor(), QString(), 2, true },   // Back
        { 0.0f,-1.0f, 0.0f, 0, 0, 0, QColor(), QString(), 1, false },  // Front
        { 0.0f, 0.0f, 1.0f, 0, 0, 0, QColor(), QString(), 5, true },   // Top
        { 0.0f, 0.0f,-1.0f, 0, 0, 0, QColor(), QString(), 6, false }   // Bottom
    };

    // Testa colisão com esferas dos eixos
    for (const auto &ax : axes) {
        float sx = ax.dirX * rx + ax.dirY * ry + ax.dirZ * rz;
        float sy = -(ax.dirX * ux + ax.dirY * uy + ax.dirZ * uz);
        float endX = m_gizmoCenter.x() + sx * armLength;
        float endY = m_gizmoCenter.y() + sy * armLength;

        float hitDist = std::hypot(pos.x() - endX, pos.y() - endY);
        if (hitDist <= (ax.isPositive ? 10.0f : 7.0f)) {
            petunia_qt_camera_set_preset(ax.preset);
            update();
            return true;
        }
    }

    // Se clicou dentro do círculo mas fora das esferas, inicia órbita pelo Gizmo
    m_isGizmoInteracting = true;
    return true;
}

void PetuniaGlWidget::mousePressEvent(QMouseEvent *event) {
    m_lastMousePos = event->pos();

    if (event->button() == Qt::LeftButton) {
        if (handleShadingClick(event->pos())) {
            event->accept();
            return;
        }
        if (handleGizmoClick(event->pos())) {
            event->accept();
            return;
        }

        // Teste de clique nos eixos do Gizmo 3D Transform
        int gizmoHandle = petunia_qt_gizmo_hit_test(
            static_cast<float>(event->pos().x()),
            static_cast<float>(event->pos().y()),
            static_cast<float>(width()),
            static_cast<float>(height())
        );
        if (gizmoHandle > 0 && !(event->modifiers() & Qt::AltModifier)) {
            m_activeGizmoHandle = gizmoHandle;
            m_isTransformGizmoDragging = true;
            event->accept();
            return;
        }

        if (event->modifiers() & Qt::AltModifier) {
            m_isInteracting = true;
            event->accept();
            return;
        }

        // Seleção de viewport (Objeto, Vértice, Aresta ou Face)
        bool extend = (event->modifiers() & Qt::ShiftModifier);
        petunia_qt_viewport_select(
            static_cast<float>(event->pos().x()),
            static_cast<float>(event->pos().y()),
            static_cast<float>(width()),
            static_cast<float>(height()),
            extend
        );
        update();
        emit selectionChanged();
        event->accept();
        return;
    }

    if (event->button() == Qt::MiddleButton) {
        m_isInteracting = true;
        event->accept();
        return;
    }

    event->accept();
}

void PetuniaGlWidget::mouseMoveEvent(QMouseEvent *event) {
    QPoint delta = event->pos() - m_lastMousePos;
    m_lastMousePos = event->pos();

    if (m_isTransformGizmoDragging) {
        if (petunia_qt_gizmo_drag_axis(
            m_activeGizmoHandle,
            static_cast<float>(delta.x()),
            static_cast<float>(delta.y()),
            static_cast<float>(width()),
            static_cast<float>(height())
        )) {
            emit selectionChanged();
            update();
        }
        event->accept();
        return;
    }

    if (m_isGizmoInteracting) {
        petunia_qt_camera_orbit(static_cast<float>(delta.x()), static_cast<float>(delta.y()));
        emit cameraOrbitRequested(static_cast<float>(delta.x()), static_cast<float>(delta.y()));
        update();
        event->accept();
        return;
    }

    if (!m_isInteracting) {
        if (m_shadingPillRect.contains(event->pos()) || (event->pos() - m_gizmoCenter).manhattanLength() < m_gizmoRadius + 10) {
            return;
        }
        if (petunia_qt_viewport_hover(
            static_cast<float>(event->pos().x()),
            static_cast<float>(event->pos().y()),
            static_cast<float>(width()),
            static_cast<float>(height())
        )) {
            update();
        }
        event->accept();
        return;
    }

    // Alt + Botão Esquerdo ou Botão do Meio (estilo Blender / Maya)
    bool isMiddle = (event->buttons() & Qt::MiddleButton);
    bool isAltLeft = (event->buttons() & Qt::LeftButton) && (event->modifiers() & Qt::AltModifier);

    if (isMiddle || isAltLeft) {
        if (event->modifiers() & Qt::ShiftModifier) {
            petunia_qt_camera_pan(static_cast<float>(delta.x()), static_cast<float>(delta.y()));
            emit cameraPanRequested(static_cast<float>(delta.x()), static_cast<float>(delta.y()));
        } else if (event->modifiers() & Qt::ControlModifier) {
            float zoomDelta = static_cast<float>(-delta.y()) * 0.05f;
            petunia_qt_camera_zoom(zoomDelta);
            emit cameraZoomRequested(zoomDelta);
        } else {
            petunia_qt_camera_orbit(static_cast<float>(delta.x()), static_cast<float>(delta.y()));
            emit cameraOrbitRequested(static_cast<float>(delta.x()), static_cast<float>(delta.y()));
        }
        update();
    }
    event->accept();
}

void PetuniaGlWidget::mouseReleaseEvent(QMouseEvent *event) {
    m_isInteracting = false;
    m_isGizmoInteracting = false;
    m_isTransformGizmoDragging = false;
    m_activeGizmoHandle = 0;
    event->accept();
}

void PetuniaGlWidget::wheelEvent(QWheelEvent *event) {
    float delta = static_cast<float>(event->angleDelta().y()) / 120.0f;
    petunia_qt_camera_zoom(delta);
    emit cameraZoomRequested(delta);
    update();
    event->accept();
}

void PetuniaGlWidget::tabletEvent(QTabletEvent *event) {
    float pressure = static_cast<float>(event->pressure());
    float x = static_cast<float>(event->position().x());
    float y = static_cast<float>(event->position().y());
    float tiltX = static_cast<float>(event->xTilt());
    float tiltY = static_cast<float>(event->yTilt());
    bool isEraser = (event->pointerType() == QPointingDevice::PointerType::Eraser);

    petunia_qt_tablet_paint_stroke(x, y, pressure, tiltX, tiltY, isEraser);
    emit tabletStrokeUpdated(x, y, pressure, tiltX, tiltY, isEraser);

    if (event->type() == QEvent::TabletPress || event->type() == QEvent::TabletMove) {
        update();
    }
    event->accept();
}

void PetuniaGlWidget::keyPressEvent(QKeyEvent *event) {
    bool handled = true;

    switch (event->key()) {
    case Qt::Key_Z:
        if (event->modifiers() & Qt::ShiftModifier) {
            int cur = petunia_qt_app_get_shading();
            petunia_qt_app_set_shading(cur == 0 ? 1 : 0);
        } else {
            int cur = petunia_qt_app_get_shading();
            petunia_qt_app_set_shading(cur == 1 ? 2 : 1);
        }
        break;
    case Qt::Key_1:
        if (event->modifiers() & Qt::KeypadModifier) {
            petunia_qt_camera_set_preset(event->modifiers() & Qt::ControlModifier ? 2 : 1); // Front / Back
        } else {
            handled = false;
        }
        break;
    case Qt::Key_3:
        if (event->modifiers() & Qt::KeypadModifier) {
            petunia_qt_camera_set_preset(event->modifiers() & Qt::ControlModifier ? 4 : 3); // Right / Left
        } else {
            handled = false;
        }
        break;
    case Qt::Key_7:
        if (event->modifiers() & Qt::KeypadModifier) {
            petunia_qt_camera_set_preset(event->modifiers() & Qt::ControlModifier ? 6 : 5); // Top / Bottom
        } else {
            handled = false;
        }
        break;
    case Qt::Key_5:
        if (event->modifiers() & Qt::KeypadModifier) {
            petunia_qt_camera_toggle_projection();
        } else {
            handled = false;
        }
        break;
    case Qt::Key_0:
        if (event->modifiers() & Qt::KeypadModifier) {
            petunia_qt_camera_reset();
        } else {
            handled = false;
        }
        break;
    case Qt::Key_Period:
        petunia_qt_app_frame_selection();
        break;
    case Qt::Key_F:
        petunia_qt_app_frame_selection();
        break;
    default:
        handled = false;
        break;
    }

    if (handled) {
        update();
        event->accept();
    } else {
        QOpenGLWidget::keyPressEvent(event);
    }
}

void PetuniaGlWidget::drawTransformGizmo(QPainter &painter) {
    float orig[2] = {0}, px[2] = {0}, py[2] = {0}, pz[2] = {0};
    if (!petunia_qt_gizmo_get_projected(static_cast<float>(width()), static_cast<float>(height()), orig, px, py, pz)) {
        return;
    }

    QPointF pOrig(orig[0], orig[1]);
    QPointF pX(px[0], px[1]);
    QPointF pY(py[0], py[1]);
    QPointF pZ(pz[0], pz[1]);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing);

    // Eixo X (Vermelho Petunia)
    QPen penX(QColor(235, 75, 75), 3.0, Qt::SolidLine, Qt::RoundCap);
    painter.setPen(penX);
    painter.drawLine(pOrig, pX);
    painter.setBrush(QColor(235, 75, 75));
    painter.drawEllipse(pX, 4.5, 4.5);

    // Eixo Y (Verde Petunia)
    QPen penY(QColor(75, 215, 75), 3.0, Qt::SolidLine, Qt::RoundCap);
    painter.setPen(penY);
    painter.drawLine(pOrig, pY);
    painter.setBrush(QColor(75, 215, 75));
    painter.drawEllipse(pY, 4.5, 4.5);

    // Eixo Z (Azul Petunia)
    QPen penZ(QColor(75, 135, 245), 3.0, Qt::SolidLine, Qt::RoundCap);
    painter.setPen(penZ);
    painter.drawLine(pOrig, pZ);
    painter.setBrush(QColor(75, 135, 245));
    painter.drawEllipse(pZ, 4.5, 4.5);

    // Centro (Branco translúcido)
    painter.setPen(QPen(QColor(240, 240, 240, 220), 1.5));
    painter.setBrush(QColor(255, 255, 255, 60));
    painter.drawEllipse(pOrig, 6.0, 6.0);

    // Rótulos X, Y, Z
    QFont font = painter.font();
    font.setPointSize(9);
    font.setBold(true);
    painter.setFont(font);

    painter.setPen(QColor(255, 100, 100));
    painter.drawText(pX + QPointF(6, 4), QStringLiteral("X"));
    painter.setPen(QColor(100, 255, 100));
    painter.drawText(pY + QPointF(6, 4), QStringLiteral("Y"));
    painter.setPen(QColor(100, 160, 255));
    painter.drawText(pZ + QPointF(6, 4), QStringLiteral("Z"));

    painter.restore();
}

void PetuniaGlWidget::drawSnapIndicator(QPainter &painter) {
    if (!m_snapEnabled) return;
    painter.save();
    painter.setPen(QPen(QColor(0, 220, 255, 200), 2.0, Qt::DashLine));
    painter.setBrush(Qt::NoBrush);
    painter.drawEllipse(m_lastMousePos, 12, 12);
    painter.restore();
}
