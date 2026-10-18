#include "petunia_gl_widget.h"
#include "petunia_editor_panel.h"
#include "petunia_icons.h"
#include "petunia_nav_cube.h"
#include "petunia_theme.h"
#include "shell/petunia_i18n.h"
#include <QContextMenuEvent>
#include <QDebug>
#include <QGuiApplication>
#include <QHash>
#include <QJsonDocument>
#include <QKeySequence>
#include <QMenu>
#include <QOpenGLContext>
#include <QResizeEvent>
#include <QShowEvent>
#include <QTimer>
#include <algorithm>
#include <cmath>
#include <functional>
#include <vector>

// Declarado no lado Rust (crates/ui-cxxqt/src/render/opengl_viewport.rs)
extern "C" {
size_t petunia_qt_software_frame_rgba(uint64_t, uint32_t, uint32_t, uint8_t *,
                                      size_t, uint32_t *, uint32_t *);
typedef void *(*QtGlGetProcAddressFn)(const char *);
bool petunia_qt_gl_surface_initialize(uint64_t, QtGlGetProcAddressFn);
void petunia_qt_gl_surface_release(uint64_t);
void petunia_qt_gl_surface_paint(uint64_t, uint32_t, int, int, float);
bool petunia_qt_gl_initialize(QtGlGetProcAddressFn get_proc);
void petunia_qt_gl_resize(int width, int height);
void petunia_qt_gl_paint(uint32_t fbo_id, int width, int height);
void petunia_qt_camera_orbit(float dx, float dy);
void petunia_qt_camera_pan(float dx, float dy);
void petunia_qt_camera_zoom(float delta);
void petunia_qt_camera_reset();
void petunia_qt_camera_set_preset(int preset);
void petunia_qt_camera_toggle_projection();
bool petunia_qt_camera_set_view_direction(float x, float y, float z);
bool petunia_qt_camera_set_yaw(float yaw_degrees);
bool petunia_qt_camera_is_ortho();
bool petunia_qt_camera_get_view_name(char *buf, int max_len);
bool petunia_qt_camera_get_orientation(float *rx, float *ry, float *rz,
                                       float *ux, float *uy, float *uz,
                                       float *fx, float *fy, float *fz);
void petunia_qt_app_frame_selection();
int petunia_qt_app_get_shading();
void petunia_qt_app_set_shading(int shading);
void petunia_qt_tablet_paint_stroke(float x, float y, float pressure,
                                    float tilt_x, float tilt_y, bool is_eraser);
bool petunia_qt_viewport_select(float screen_x, float screen_y,
                                float viewport_w, float viewport_h,
                                bool extend);
bool petunia_qt_viewport_hover(float screen_x, float screen_y, float viewport_w,
                               float viewport_h);
bool petunia_qt_gizmo_get_projected(float viewport_w, float viewport_h,
                                    float *out_origin, float *out_x,
                                    float *out_y, float *out_z);
int petunia_qt_gizmo_hit_test(float mouse_x, float mouse_y, float viewport_w,
                              float viewport_h);
bool petunia_qt_gizmo_drag_axis(int axis, float dx, float dy, float viewport_w,
                                float viewport_h);
}

static void *qt_gl_get_proc_address(const char *name) {
    auto *ctx = QOpenGLContext::currentContext();
    if (!ctx)
        return nullptr;
    return reinterpret_cast<void *>(ctx->getProcAddress(QByteArray(name)));
}

class PetuniaNativeGlCanvas : public QOpenGLWidget {
  public:
    PetuniaGlWidget *owner;
    explicit PetuniaNativeGlCanvas(PetuniaGlWidget *parent)
        : QOpenGLWidget(parent), owner(parent) {
        QSurfaceFormat fmt;
        fmt.setVersion(3, 3);
        fmt.setProfile(QSurfaceFormat::CoreProfile);
        setFormat(fmt);
        setAttribute(Qt::WA_TransparentForMouseEvents);
        setFocusPolicy(Qt::NoFocus);
    }

  protected:
    void initializeGL() override { owner->initializeGL(); }
    void resizeGL(int w, int h) override { owner->resizeGL(w, h); }
    void paintGL() override {
        owner->paintGL(defaultFramebufferObject(), this);
    }
};
PetuniaGlWidget::PetuniaGlWidget(QWidget *parent) : QWidget(parent) {
    const auto platform = QGuiApplication::platformName();
    m_software = platform == "offscreen" || platform == "minimal" ||
                 qEnvironmentVariableIsSet("PETUNIA_VIEWPORT_SOFTWARE");
    if (!m_software)
        m_gl = new PetuniaNativeGlCanvas(this);
    setAccessibleName(tr("Viewport 3D"));
    setAccessibleDescription(tr("Ferramentas do documento; Enter confirma, "
                                "Escape cancela e F6 muda de região"));
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
}

PetuniaGlWidget::~PetuniaGlWidget() { cleanupGl(); }

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
    auto *ctx = QOpenGLContext::currentContext();
    if (!ctx) {
        qWarning()
            << "[PetuniaGlWidget] Sem contexto OpenGL ativo no QPA atual.";
        return;
    }
    m_rendererInitialized =
        petunia_qt_gl_surface_initialize(m_surfaceId, &qt_gl_get_proc_address);
    connect(ctx, &QOpenGLContext::aboutToBeDestroyed, this,
            &PetuniaGlWidget::cleanupGl, Qt::DirectConnection);
    if (!m_rendererInitialized) {
        qWarning()
            << "[PetuniaGlWidget] Falha ao inicializar renderer OpenGL Rust.";
        m_software = true;
        QTimer::singleShot(0, this, [this] {
            if (m_gl)
                m_gl->hide();
            update();
        });
    }
}

void PetuniaGlWidget::resizeGL(int w, int h) {
    PetuniaViewScope scope(m_surfaceId);
    if (m_rendererInitialized) {
        petunia_qt_gl_resize(w, h);
    }
}

void PetuniaGlWidget::paintGL(uint32_t fbo, QPaintDevice *device) {
    auto *ctx = QOpenGLContext::currentContext();
    if (!ctx || !m_rendererInitialized) {
        return;
    }

    // 1. Renderiza a cena 3D Rust via framebuffer blit
    PetuniaViewScope scope(m_surfaceId);
    petunia_qt_gl_surface_paint(m_surfaceId, fbo, width(), height(),
                                devicePixelRatioF());
    refreshOverlayState();

    // 2. Sobrepõe overlays interativos 2D fiéis via QPainter nativo
    QPainter painter(device);
    drawOverlays(painter);
}
void PetuniaGlWidget::update() {
    if (m_gl && !m_software)
        m_gl->update();
    else
        QWidget::update();
}
void PetuniaGlWidget::showEvent(QShowEvent *event) {
    QWidget::showEvent(event);
    if (m_gl && !m_software)
        QTimer::singleShot(500, this, [this] {
            if (isVisible() && !m_rendererInitialized) {
                m_software = true;
                m_gl->hide();
                update();
            }
        });
}
void PetuniaGlWidget::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);
    if (m_gl)
        m_gl->setGeometry(rect());
}
void PetuniaGlWidget::paintEvent(QPaintEvent *) {
    if (!m_software)
        return;
    uint32_t w = 0, h = 0;
    const auto len = petunia_qt_software_frame_rgba(
        m_surfaceId, width(), height(), nullptr, 0, &w, &h);
    QPainter painter(this);
    painter.fillRect(rect(), palette().color(QPalette::Base));
    if (len && len <= 1280 * 720 * 4 && len == size_t(w) * h * 4) {
        QByteArray pixels;
        pixels.resize(len);
        if (petunia_qt_software_frame_rgba(
                m_surfaceId, width(), height(),
                reinterpret_cast<uint8_t *>(pixels.data()), len, &w, &h) == len)
            painter.drawImage(rect(), QImage(reinterpret_cast<const uchar *>(
                                                 pixels.constData()),
                                             w, h, QImage::Format_RGBA8888));
    }
    PetuniaViewScope scope(m_surfaceId);
    refreshOverlayState();
    drawOverlays(painter);
}
void PetuniaGlWidget::refreshOverlayState() {
    if (m_shellChrome && m_hasExternalState)
        m_overlayState = m_externalState;
    else
        m_overlayState = petuniaRequest("snapshot");
}

void PetuniaGlWidget::drawOverlays(QPainter &painter) {
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setRenderHint(QPainter::TextAntialiasing);
    draw3dCursor(painter);
    drawPivotMarker(painter);
    drawDecalGizmo(painter);
    if (!m_shellChrome) {
        drawViewportHud(painter);
        drawShadingPill(painter);
    }
    drawNavigationGizmo(painter);
    drawTransformGizmo(painter);
    drawSnapIndicator(painter);
    drawBrushCursor(painter);
}

void PetuniaGlWidget::drawViewportHud(QPainter &painter) {
    char buf[64] = {0};
    QString viewName = QStringLiteral("User Persp");
    if (petunia_qt_camera_get_view_name(buf, sizeof(buf))) {
        viewName = QString::fromUtf8(buf);
    }

    // Tipografia e cores do tema: legível em alto contraste e com escala.
    const auto &theme = PetuniaTheme::instance();
    QFont font = painter.font();
    font.setPixelSize(theme.fontPx(PetuniaTheme::TextRole::Body));
    font.setBold(true);
    painter.setFont(font);

    QFontMetrics fm(font);
    int textWidth = fm.horizontalAdvance(viewName);
    QRect badgeRect(12, 12, textWidth + 18, std::max(theme.minHitTarget() - 2, fm.height() + 8));

    QColor badgeFill = theme.bgPanel();
    badgeFill.setAlpha(theme.isHighContrast() ? 255 : 215);
    painter.setPen(theme.borderStrong());
    painter.setBrush(badgeFill);
    painter.drawRoundedRect(badgeRect, 4, 4);

    painter.setPen(theme.accentPrimary());
    painter.drawText(badgeRect, Qt::AlignCenter, viewName);

    // Sub-legenda de status
    font.setPixelSize(theme.fontPx(PetuniaTheme::TextRole::Caption));
    font.setBold(false);
    painter.setFont(font);
    painter.setPen(theme.textSecondary());
    painter.drawText(14, badgeRect.bottom() + 6 + QFontMetrics(font).ascent(),
                     m_overlayState["context_hint"].toString());
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
    font.setPixelSize(PetuniaTheme::instance().fontPx(PetuniaTheme::TextRole::Body));
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
    if (m_shellChrome)
        return false;
    if (!m_shadingPillRect.contains(pos)) {
        return false;
    }
    int segWidth = m_shadingPillRect.width() / 4;
    if (segWidth <= 0)
        return false;
    int seg = (pos.x() - m_shadingPillRect.x()) / segWidth;
    seg = qBound(0, seg, 3);
    petunia_qt_app_set_shading(seg);
    update();
    return true;
}

/// Lê a base da câmera do núcleo e posiciona o cubo de navegação no canto
/// superior direito. `false` se a ponte não responde (sem desenho).
bool PetuniaGlWidget::syncNavCube() {
    float rx = 1.0f, ry = 0.0f, rz = 0.0f;
    float ux = 0.0f, uy = 1.0f, uz = 0.0f;
    float fx = 0.0f, fy = 0.0f, fz = 1.0f;
    if (!petunia_qt_camera_get_orientation(&rx, &ry, &rz, &ux, &uy, &uz, &fx,
                                           &fy, &fz))
        return false;
    m_navCube.setCamera(QVector3D(rx, ry, rz), QVector3D(ux, uy, uz),
                        QVector3D(fx, fy, fz));
    const double unit = qMax(14.0, double(PetuniaTheme::instance().scaled(20)));
    m_gizmoCenter = QPoint(width() - int(unit * 3.5), int(unit * 3.6));
    m_navCube.setGeometry(QPointF(m_gizmoCenter), unit);
    m_navCube.setOrtho(petunia_qt_camera_is_ortho());
    return true;
}

void PetuniaGlWidget::drawNavigationGizmo(QPainter &painter) {
    if (!syncNavCube())
        return;
    m_navCube.paint(painter, m_navHover,
                    m_navHover.type != PetuniaNavCube::Hit::Type::None ||
                        m_isGizmoInteracting);
}

/// Executa o pedido do cubo (face, aresta, canto, bússola, início ou menu).
void PetuniaGlWidget::runNavAction(const PetuniaNavCube::Hit &hit) {
    using Type = PetuniaNavCube::Hit::Type;
    switch (hit.type) {
    case Type::Preset:
        petunia_qt_camera_set_preset(hit.preset);
        break;
    case Type::Direction:
        petunia_qt_camera_set_view_direction(hit.direction.x(), hit.direction.y(),
                                             hit.direction.z());
        break;
    case Type::Yaw:
        petunia_qt_camera_set_yaw(float(hit.yawDegrees));
        break;
    case Type::Home:
        petunia_qt_camera_reset();
        break;
    case Type::Menu: {
        QMenu menu(this);
        menu.setAccessibleName(tr("Projeção e enquadramento"));
        const bool ortho = petunia_qt_camera_is_ortho();
        auto *persp = menu.addAction(tr("Perspectiva"));
        persp->setCheckable(true);
        persp->setChecked(!ortho);
        auto *orthographic = menu.addAction(tr("Ortográfica"));
        orthographic->setCheckable(true);
        orthographic->setChecked(ortho);
        menu.addSeparator();
        auto *frame = menu.addAction(tr("Enquadrar seleção"));
        auto *reset = menu.addAction(tr("Restaurar câmera"));
        const QPointF anchor = m_navCube.bounds().bottomRight();
        auto *picked = menu.exec(mapToGlobal(QPoint(int(anchor.x()) - 120, int(anchor.y()))));
        if (picked == persp && ortho)
            petunia_qt_camera_toggle_projection();
        else if (picked == orthographic && !ortho)
            petunia_qt_camera_toggle_projection();
        else if (picked == frame)
            petunia_qt_app_frame_selection();
        else if (picked == reset)
            petunia_qt_camera_reset();
        break;
    }
    case Type::None:
        break;
    }
    update();
}

/// Clique no cubo: ações simples executam ao soltar sem arrasto; arrastar o
/// cubo orbita a câmera. Fora do cubo devolve `false`.
bool PetuniaGlWidget::handleGizmoClick(const QPoint &pos) {
    if (!syncNavCube())
        return false;
    const QPointF point(pos);
    const auto hit = m_navCube.hitTest(point);
    if (hit.type != PetuniaNavCube::Hit::Type::None) {
        m_navPress = hit;
        m_navPressPoint = point;
        m_navPressed = true;
        return true;
    }
    if (m_navCube.contains(point)) {
        m_isGizmoInteracting = true;
        return true;
    }
    return false;
}

static int modifierMask(Qt::KeyboardModifiers mods) {
    return (mods & Qt::ControlModifier ? 1 : 0) |
           (mods & Qt::ShiftModifier ? 2 : 0) |
           (mods & Qt::AltModifier ? 4 : 0);
}
static bool pointerBinding(const QJsonObject &state, const char *key,
                           Qt::KeyboardModifiers mods) {
    const int required = state[key].toInt();
    return required > 0 && (modifierMask(mods) & required) == required;
}

/// Ferramentas de pintura que mostram o anel do pincel (mesma lista do Slint).
static bool isPaintBrushTool(const QJsonObject &state) {
    static const QStringList tools{
        QStringLiteral("brush"),    QStringLiteral("eraser"),
        QStringLiteral("airbrush"), QStringLiteral("pixel"),
        QStringLiteral("smudge"),   QStringLiteral("blur"),
        QStringLiteral("dodge"),    QStringLiteral("burn"),
        QStringLiteral("spray"),    QStringLiteral("clone")};
    return state[QStringLiteral("workspace")].toString().toUpper() ==
               QStringLiteral("PAINT") &&
           tools.contains(state[QStringLiteral("active_tool")].toString());
}

static QPointF pointFromArray(const QJsonValue &value) {
    const auto array = value.toArray();
    if (array.size() < 2)
        return {};
    return QPointF(array[0].toDouble(), array[1].toDouble());
}

/// Realimenta o anel do pincel e o realce das alças do decalque. Durante
/// navegação (órbita/pan) o cursor volta a ser o do sistema.
void PetuniaGlWidget::updatePointerFeedback(const QPointF &position,
                                            bool navigating) {
    const bool brush = m_pointerInside && !navigating &&
                       isPaintBrushTool(m_overlayState);
    m_brushCursorVisible = brush;
    if (brush)
        setCursor(Qt::BlankCursor);
    else
        unsetCursor();
    if (!brush) {
        m_brushSquash = 1.0f;
        m_brushTilt = 0.0f;
        return;
    }
    const auto response = petuniaRequest(
        "brush_footprint", {{"x", position.x()}, {"y", position.y()}});
    m_brushSquash =
        qBound(0.05, response[QStringLiteral("brush_squash")].toDouble(1.0), 1.0);
    m_brushTilt = response[QStringLiteral("brush_tilt")].toDouble(0.0);
}

void PetuniaGlWidget::enterEvent(QEnterEvent *event) {
    m_pointerInside = true;
    QWidget::enterEvent(event);
    update();
}

void PetuniaGlWidget::leaveEvent(QEvent *event) {
    m_pointerInside = false;
    m_navHover = PetuniaNavCube::Hit{};
    m_brushCursorVisible = false;
    m_brushSquash = 1.0f;
    m_brushTilt = 0.0f;
    unsetCursor();
    QWidget::leaveEvent(event);
    update();
}
void PetuniaGlWidget::mousePressEvent(QMouseEvent *event) {
    setFocus();
    PetuniaViewScope scope(m_surfaceId);
    m_lastMousePos = event->pos();
    m_pressPoint = event->position();
    m_overlayState = petuniaRequest("snapshot");
    if (event->button() == Qt::RightButton) {
        // O menu abre em contextMenuEvent (também pela tecla Menu/Shift+F10).
        event->accept();
        return;
    }
    if (event->button() == Qt::MiddleButton ||
        (event->button() == Qt::LeftButton &&
         pointerBinding(m_overlayState, "pointer_orbit_left_mask",
                        event->modifiers()))) {
        m_isInteracting = true;
        event->accept();
        return;
    }
    if (event->button() == Qt::LeftButton) {
        if (handleShadingClick(event->pos()) ||
            handleGizmoClick(event->pos())) {
            event->accept();
            return;
        }
        const auto begin =
            petuniaRequest("gizmo_begin", {{"x", event->position().x()},
                                           {"y", event->position().y()}});
        if (begin["changed"].toBool()) {
            m_isTransformGizmoDragging = true;
            m_overlayState = begin;
            event->accept();
            update();
            return;
        }
        m_overlayState = petuniaRequest("snapshot");
        const auto tool = m_overlayState["active_tool"].toString();
        if (tool == "cursor" || tool == "cursor_3d") {
            const float norm_x = static_cast<float>(event->position().x()) / qMax(1.0f, static_cast<float>(width()));
            const float norm_y = static_cast<float>(event->position().y()) / qMax(1.0f, static_cast<float>(height()));
            m_overlayState = petuniaRequest("place_cursor_3d", {{"x", norm_x}, {"y", norm_y}});
            m_toolPressed = true;
            emit selectionChanged();
            update();
            event->accept();
            return;
        }
        m_regionSelecting =
            tool == "select" || tool == "lasso_select" ||
            (m_overlayState["workspace"] == "Animate" &&
             m_overlayState["animate"].toObject()["selected_bone"].isNull());
        m_lasso = tool == "lasso_select";
        m_regionPoints.clear();
        m_regionPoints << event->position();
        m_toolPressed = true;
        if (!m_regionSelecting)
            pointer(0, event->position(), event->modifiers());
        event->accept();
        update();
        return;
    }
    QWidget::mousePressEvent(event);
}

void PetuniaGlWidget::mouseMoveEvent(QMouseEvent *event) {
    PetuniaViewScope scope(m_surfaceId);
    const auto delta = event->pos() - m_lastMousePos;
    m_lastMousePos = event->pos();
    const auto activeTool = m_overlayState["active_tool"].toString();
    updatePointerFeedback(
        event->position(),
        m_isInteracting || m_isGizmoInteracting || m_isTransformGizmoDragging);
    if (!m_isInteracting && !m_isGizmoInteracting && !m_isTransformGizmoDragging &&
        !m_toolPressed) {
        const bool known = syncNavCube();
        const auto hover = known ? m_navCube.hitTest(QPointF(event->position()))
                                 : PetuniaNavCube::Hit{};
        if (hover.id != m_navHover.id) {
            m_navHover = hover;
            update();
        }
    }
    if (m_toolPressed && (activeTool == "cursor" || activeTool == "cursor_3d")) {
        const float norm_x = static_cast<float>(event->position().x()) / qMax(1.0f, static_cast<float>(width()));
        const float norm_y = static_cast<float>(event->position().y()) / qMax(1.0f, static_cast<float>(height()));
        m_overlayState = petuniaRequest("place_cursor_3d", {{"x", norm_x}, {"y", norm_y}});
        update();
        event->accept();
        return;
    }
    if (m_isTransformGizmoDragging) {
        m_overlayState = petuniaRequest(
            "update_viewport_transform_modified",
            {{"x", event->position().x()},
             {"y", event->position().y()},
             {"fine", pointerBinding(m_overlayState, "pointer_precision_mask",
                                     event->modifiers())},
             {"snap", pointerBinding(m_overlayState, "pointer_snap_mask",
                                     event->modifiers())}});
        update();
        event->accept();
        return;
    }
    if (m_navPressed &&
        QLineF(m_navPressPoint, QPointF(event->position())).length() > 4.0) {
        // Arrastar o cubo orbita: o clique vira gesto contínuo.
        m_navPressed = false;
        m_isGizmoInteracting = true;
    }
    if (m_navPressed) {
        event->accept();
        return;
    }
    if (m_isGizmoInteracting) {
        petunia_qt_camera_orbit(delta.x(), delta.y());
        update();
        event->accept();
        return;
    }
    if (m_isInteracting) {
        if (pointerBinding(m_overlayState, "pointer_pan_mask",
                           event->modifiers()))
            petunia_qt_camera_pan(delta.x(), delta.y());
        else if (event->modifiers() & Qt::ControlModifier)
            petunia_qt_camera_zoom(-delta.y() * 0.05f);
        else
            petunia_qt_camera_orbit(delta.x(), delta.y());
        update();
        event->accept();
        return;
    }
    if (m_regionSelecting) {
        if (m_regionPoints.size() < 4096)
            m_regionPoints << event->position();
        update();
        event->accept();
        return;
    }
    if (m_toolPressed || m_overlayState["tool_gesture_latched"].toBool())
        pointer(1, event->position(), event->modifiers());
    else {
        petuniaRequest("hover_gizmo", {{"x", event->position().x()},
                                       {"y", event->position().y()}});
        m_overlayState = petuniaRequest(
            "hover", {{"x", event->position().x() / qMax(1, width())},
                      {"y", event->position().y() / qMax(1, height())}});
        pointer(1, event->position(), event->modifiers());
    }
    update();
    event->accept();
}

/// Menu de contexto da vista: seleção universal + as ações contextuais do domínio
/// atual (mesma tabela da barra de ações), com atalho e motivo quando
/// desabilitadas. Abre pelo botão direito ou pelo teclado (centro da vista).
void PetuniaGlWidget::contextMenuEvent(QContextMenuEvent *event) {
    PetuniaViewScope scope(m_surfaceId);
    const bool keyboard = event->reason() == QContextMenuEvent::Keyboard;
    const QPointF local = keyboard ? QPointF(width() / 2.0, height() / 2.0) : QPointF(event->pos());
    const QPoint global = keyboard ? mapToGlobal(local.toPoint()) : event->globalPos();
    petuniaRequest("open_viewport_context_menu", {{"x", local.x()}, {"y", local.y()}});
    const auto state = petuniaRequest("snapshot");
    QMenu menu(this);
    menu.setAccessibleName(tr("Ações da vista"));
    menu.setToolTipsVisible(true);
    QHash<QAction *, std::function<void()>> run;
    const struct { const char *id; const char *label; } universal[] = {
        {"select_all", QT_TR_NOOP("Selecionar tudo")},
        {"clear_selection", QT_TR_NOOP("Limpar seleção")},
        {"invert_selection", QT_TR_NOOP("Inverter seleção")},
        {"frame", QT_TR_NOOP("Enquadrar seleção")},
    };
    for (const auto &item : universal) {
        const QString id = QLatin1String(item.id);
        run.insert(menu.addAction(tr(item.label)),
                   [id] { petuniaRequest("context_menu_action", {{"target", id}}); });
    }
    const auto actions = state["context_actions"].toArray();
    if (!actions.isEmpty())
        menu.addSeparator();
    for (const auto &value : actions) {
        const auto action = value.toObject();
        const QString kind = action["kind"].toString();
        const QString target = action["target"].toString();
        QString text = petuniaTr(action["label"].toString());
        bool enabled = true;
        QString reason;
        if (kind == QLatin1String("command")) {
            for (const auto &c : state["commands"].toArray()) {
                const auto command = c.toObject();
                if (command["id"].toString() == target) {
                    enabled = command["enabled"].toBool(true);
                    reason = command["reason"].toString();
                }
            }
        }
        const QString shortcut = action["shortcut"].toString();
        if (!shortcut.isEmpty())
            text += QLatin1Char('\t') + shortcut;
        auto *menuAction = menu.addAction(PetuniaIcons::get(action["icon"].toString()), text);
        menuAction->setEnabled(enabled);
        if (!enabled && !reason.isEmpty())
            menuAction->setToolTip(reason);
        run.insert(menuAction, [this, kind, target] {
            if (target.startsWith(QLatin1String("ui.")))
                emit uiActionRequested(target);
            else if (kind == QLatin1String("command"))
                petuniaRequest("command", {{"id", target}});
            else if (kind == QLatin1String("tool"))
                petuniaRequest("tool", {{"id", target}});
            else
                petuniaRequest(target);
        });
    }
    auto *picked = menu.exec(global);
    if (picked && run.contains(picked))
        run.value(picked)();
    else
        petuniaRequest("close_context_menu");
    emit selectionChanged();
    update();
    event->accept();
}

void PetuniaGlWidget::mouseReleaseEvent(QMouseEvent *event) {
    // Só o botão esquerdo encerra gestos de ferramenta/gizmo: soltar outro botão
    // no meio do arrasto (ex.: o direito) não fecha o traço nem a transformação.
    if (event->button() != Qt::LeftButton && (m_toolPressed || m_isTransformGizmoDragging)) {
        event->accept();
        return;
    }
    if (m_navPressed) {
        const auto hit = m_navPress;
        m_navPressed = false;
        m_navPress = PetuniaNavCube::Hit{};
        runNavAction(hit);
        emit selectionChanged();
        event->accept();
        return;
    }
    if (m_isTransformGizmoDragging) {
        PetuniaViewScope scope(m_surfaceId);
        m_overlayState = petuniaRequest("end_gizmo_drag");
        m_isTransformGizmoDragging = false;
        emit selectionChanged();
        update();
        event->accept();
        return;
    }
    {
        PetuniaViewScope scope(m_surfaceId);
        if (m_toolPressed) {
            if (m_regionSelecting) {
                const auto distance =
                    QLineF(m_pressPoint, event->position()).length();
                const auto threshold =
                    m_overlayState["drag_threshold_px"].toDouble(4);
                if (distance < threshold ||
                    m_overlayState["workspace"] == "Animate") {
                    m_overlayState = petuniaRequest(
                        "viewport_select",
                        {{"x", m_pressPoint.x() / qMax(1, width())},
                         {"y", m_pressPoint.y() / qMax(1, height())},
                         {"extend",
                          pointerBinding(m_overlayState, "pointer_extend_mask",
                                         event->modifiers())},
                         {"loop",
                          pointerBinding(m_overlayState, "pointer_loop_mask",
                                         event->modifiers())}});
                } else {
                    QJsonObject args{
                        {"x0", m_pressPoint.x() / qMax(1, width())},
                        {"y0", m_pressPoint.y() / qMax(1, height())},
                        {"x1", event->position().x() / qMax(1, width())},
                        {"y1", event->position().y() / qMax(1, height())},
                        {"extend",
                         pointerBinding(m_overlayState, "pointer_extend_mask",
                                        event->modifiers())},
                        {"subtract",
                         pointerBinding(m_overlayState, "pointer_subtract_mask",
                                        event->modifiers())},
                        {"lasso", m_lasso}};
                    QString path;
                    for (int i = 0; i < m_regionPoints.size(); ++i)
                        path +=
                            QStringLiteral("%1,%2;")
                                .arg(m_regionPoints[i].x() / qMax(1, width()))
                                .arg(m_regionPoints[i].y() / qMax(1, height()));
                    args.insert("path", path);
                    m_overlayState = petuniaRequest("region_select", args);
                }
            } else
                pointer(2, event->position(), event->modifiers());
        }
        m_toolPressed = false;
        m_regionSelecting = false;
        m_regionPoints.clear();
        m_isInteracting = false;
        m_isGizmoInteracting = false;
    }
    emit selectionChanged();
    update();
    event->accept();
}

void PetuniaGlWidget::wheelEvent(QWheelEvent *event) {
    PetuniaViewScope scope(m_surfaceId);
    const float rawDelta = static_cast<float>(event->angleDelta().y());
    if (m_overlayState["loop_cut_active"].toBool())
        m_overlayState =
            petuniaRequest("scroll_loop_cut_count", {{"delta_y", rawDelta / 120.f}});
    else if (m_overlayState["loop_cut_armed"].toBool())
        m_overlayState = petuniaRequest("adjust_loop_cut_hover_count",
                                        {{"delta", int(rawDelta / 120.f)}});
    else
        petunia_qt_camera_zoom(rawDelta * m_zoomSensitivity);
    update();
    event->accept();
}

void PetuniaGlWidget::tabletEvent(QTabletEvent *event) {
    PetuniaViewScope scope(m_surfaceId);
    const int phase = event->type() == QEvent::TabletPress     ? 0
                      : event->type() == QEvent::TabletRelease ? 2
                                                               : 1;
    petuniaRequest("tablet_input",
                   {{"pressure", event->pressure()},
                    {"tilt_x", event->xTilt()},
                    {"tilt_y", event->yTilt()},
                    {"eraser", event->pointerType() ==
                                   QPointingDevice::PointerType::Eraser}});
    pointer(phase, event->position(), event->modifiers());
    if (phase == 2)
        petuniaRequest("tablet_reset");
    emit selectionChanged();
    update();
    event->accept();
}

void PetuniaGlWidget::finishInterruptedGesture(bool commitPaint) {
    const bool active = m_isTransformGizmoDragging || (m_toolPressed && !m_regionSelecting);
    if (active) {
        PetuniaViewScope scope(m_surfaceId);
        if (m_isTransformGizmoDragging) {
            m_overlayState = petuniaRequest("cancel_transform");
        } else {
            const bool painting = m_overlayState["workspace"].toString() == QLatin1String("Paint");
            // Fase 2 confirma (um Undo); fase 3 cancela sem efeito.
            pointer(painting && commitPaint ? 2 : 3, QPointF(m_lastMousePos), Qt::NoModifier);
        }
    }
    m_toolPressed = false;
    m_regionSelecting = false;
    m_regionPoints.clear();
    m_isInteracting = false;
    m_isGizmoInteracting = false;
    m_isTransformGizmoDragging = false;
    petuniaRequest("tablet_reset");
    if (active) {
        emit selectionChanged();
        update();
    }
}

// Foco perdido no meio de um gesto (Alt+Tab, popup, F6): o traço de pintura é
// confirmado (trabalho intencional); gizmo e ferramentas geométricas cancelam,
// para nada se mover por acidente.
void PetuniaGlWidget::focusOutEvent(QFocusEvent *event) {
    QWidget::focusOutEvent(event);
    finishInterruptedGesture(true);
}

void PetuniaGlWidget::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Escape)
        finishInterruptedGesture(false);
    PetuniaViewScope scope(m_surfaceId);
    const auto text = petuniaKeyText(event);
    const auto state = petuniaRequest(
        "shortcut", {{"text", text},
                     {"ctrl", bool(event->modifiers() & Qt::ControlModifier)},
                     {"shift", bool(event->modifiers() & Qt::ShiftModifier)},
                     {"alt", bool(event->modifiers() & Qt::AltModifier)}});
    m_overlayState = state;
    if (state["changed"].toBool()) {
        emit selectionChanged();
        update();
        event->accept();
    } else
        QWidget::keyPressEvent(event);
}

namespace {
/// Retângulo envolvente de um caminho `M/L` do gizmo (alças pequenas).
QRectF pathBounds(const QPainterPath &path) { return path.boundingRect(); }
} // namespace

/// Marcadores de pivô e de origem. O pivô (ponto de transformação) é um disco
/// com anel e quatro marcas; a origem do objeto, um losango vazado. Ambos são
/// distintos do 3D Cursor (anel tracejado + mira) e do gizmo.
void PetuniaGlWidget::drawPivotMarker(QPainter &painter) {
    const auto gizmo = m_overlayState["gizmo"].toObject();
    if (!gizmo["pivot_visible"].toBool())
        return;
    const auto &theme = PetuniaTheme::instance();
    const bool editing = gizmo["pivot_editing"].toBool();
    const QPointF pivot = pointFromArray(gizmo["pivot_screen"]);
    const QColor accent = editing ? theme.accentSelectionActive() : theme.accentPrimary();

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setBrush(Qt::NoBrush);

    if (gizmo["origin_marker_visible"].toBool()) {
        const QPointF origin = pointFromArray(gizmo["origin_marker_screen"]);
        if (QLineF(origin, pivot).length() > 6.0) {
            QPolygonF diamond;
            diamond << origin + QPointF(0, -5) << origin + QPointF(5, 0) << origin + QPointF(0, 5)
                    << origin + QPointF(-5, 0);
            painter.setPen(QPen(theme.bgCanvas(), 3.2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.drawPolygon(diamond);
            painter.setPen(QPen(theme.accentSelection(), 1.6, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.drawPolygon(diamond);
        }
    }

    const double ring = editing ? 11.0 : 8.0;
    painter.setPen(QPen(theme.bgCanvas(), 3.6));
    painter.drawEllipse(pivot, ring, ring);
    painter.setPen(QPen(accent, editing ? 2.2 : 1.6));
    painter.drawEllipse(pivot, ring, ring);
    for (int k = 0; k < 4; ++k) {
        const double angle = 0.78539816339744830962 + k * 1.57079632679489661923;
        const QPointF a = pivot + QPointF(std::cos(angle), std::sin(angle)) * (ring + 1.0);
        const QPointF b = pivot + QPointF(std::cos(angle), std::sin(angle)) * (ring + 5.0);
        painter.setPen(QPen(theme.bgCanvas(), 3.4, Qt::SolidLine, Qt::RoundCap));
        painter.drawLine(a, b);
        painter.setPen(QPen(accent, 1.6, Qt::SolidLine, Qt::RoundCap));
        painter.drawLine(a, b);
    }
    painter.setPen(Qt::NoPen);
    painter.setBrush(accent);
    painter.drawEllipse(pivot, 3.2, 3.2);

    if (editing) {
        QFont font = painter.font();
        font.setPixelSize(theme.fontPx(PetuniaTheme::TextRole::Caption));
        font.setBold(true);
        painter.setFont(font);
        const QString text = petuniaTr(QStringLiteral("Editando pivô · arraste o gizmo · Esc sai"));
        const QRectF box(pivot.x() - 130, pivot.y() + 22, 260, 18);
        painter.setPen(theme.bgCanvas());
        painter.drawText(box.translated(1, 1), Qt::AlignCenter, text);
        painter.setPen(accent);
        painter.drawText(box, Qt::AlignCenter, text);
    }
    painter.restore();
}

/// Gizmo de transformação: hastes com ponta, alças de escala arredondadas,
/// planos translúcidos, anéis de rotação com o lado da câmera em destaque e
/// uma alça central. Espessura e tamanho vêm das preferências. O eixo sob o
/// ponteiro (ou em arrasto) cresce e os outros recuam.
void PetuniaGlWidget::drawTransformGizmo(QPainter &painter) {
    const auto gizmo = m_overlayState["gizmo"].toObject();
    if (!gizmo["visible"].toBool())
        return;
    const auto &theme = PetuniaTheme::instance();

    const QString activeTool = m_overlayState["active_tool"].toString();
    const bool isRotate = (activeTool == QStringLiteral("rotate"));
    const bool isScale = (activeTool == QStringLiteral("scale"));
    const bool isTransform = (activeTool == QStringLiteral("transform"));
    const bool isMove = (activeTool == QStringLiteral("move") || activeTool == QStringLiteral("select") || activeTool.isEmpty());

    const bool showArrows = isMove || isTransform;
    const bool showPlanes = isMove || isTransform;
    const bool showRings = isRotate || isTransform;
    const bool showScaleBoxes = isScale || isTransform;

    const double thickness = qBound(1.0, gizmo["thickness"].toDouble(2.0), 6.0);
    const int hoverAxis = m_overlayState["gizmo_hover_axis"].toInt(-1);
    const int activeAxis = m_overlayState["gizmo_active_axis"].toInt(-1);
    const bool centerHot = m_overlayState["gizmo_center_hover"].toBool() ||
                           m_overlayState["gizmo_center_active"].toBool();
    const QColor halo = [&] {
        QColor c = theme.bgCanvas();
        c.setAlpha(150);
        return c;
    }();

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    const QStringList axes{"x", "y", "z"};
    for (int i = 0; i < axes.size(); ++i) {
        const bool hot = hoverAxis == i || activeAxis == i;
        const bool dim = activeAxis >= 0 && activeAxis != i;
        QColor color = theme.axisColor(i);
        if (hot)
            color = color.lighter(128);
        color.setAlpha(dim ? 70 : 255);
        const double width = thickness * (hot ? 1.6 : 1.0);

        // 1. Haste do eixo, com halo para ler sobre qualquer superfície.
        if (showArrows || showScaleBoxes) {
            const QPainterPath rod = petuniaPath(gizmo[axes[i] + "_commands"].toString());
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(halo, width + 2.0, Qt::SolidLine, Qt::RoundCap));
            painter.drawPath(rod);
            painter.setPen(QPen(color, width, Qt::SolidLine, Qt::RoundCap));
            painter.drawPath(rod);
        }

        // 2. Anéis de rotação: anel inteiro discreto, lado da câmera em destaque.
        if (showRings) {
            const QString ringKey = isRotate ? axes[i] + "_commands" : axes[i] + "_rotate_commands";
            const QPainterPath full = petuniaPath(gizmo[ringKey].toString());
            const QPainterPath front = petuniaPath(gizmo[axes[i] + "_ring_front"].toString());
            QColor faint = color;
            faint.setAlpha(dim ? 30 : (isRotate ? 95 : 60));
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(faint, qMax(1.0, thickness * 0.7), Qt::SolidLine, Qt::RoundCap));
            painter.drawPath(full);
            QColor strong = color;
            strong.setAlpha(dim ? 70 : (isRotate ? 255 : 200));
            painter.setPen(QPen(halo, thickness * 1.5 + 2.0, Qt::SolidLine, Qt::RoundCap));
            painter.drawPath(front);
            painter.setPen(QPen(strong, thickness * (hot ? 2.0 : 1.5), Qt::SolidLine, Qt::RoundCap));
            painter.drawPath(front);
        }

        // 3. Pontas de seta (cone achatado) com contorno.
        if (showArrows) {
            painter.setPen(QPen(color.darker(180), 1.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.setBrush(color.lighter(112));
            painter.drawPath(petuniaPath(gizmo[axes[i] + "_arrow_commands"].toString()));
        }

        // 4. Alças de escala: quadrados de cantos arredondados.
        if (showScaleBoxes) {
            const QPainterPath box = petuniaPath(gizmo[axes[i] + "_scale_commands"].toString());
            const QPainterPath endBox = isScale ? petuniaPath(gizmo[axes[i] + "_arrow_commands"].toString()) : QPainterPath();
            painter.setPen(QPen(color.darker(180), 1.0));
            painter.setBrush(color.lighter(112));
            for (const auto &shape : {box, endBox}) {
                if (shape.isEmpty())
                    continue;
                painter.drawRoundedRect(pathBounds(shape), 2.0, 2.0);
            }
        }

        if (showArrows || isRotate || isScale) {
            const auto label = gizmo[axes[i] + "_label"].toArray();
            if (label.size() == 2) {
                QFont font = painter.font();
                font.setPixelSize(theme.fontPx(PetuniaTheme::TextRole::Caption));
                font.setBold(true);
                painter.setFont(font);
                const QPointF at(label[0].toDouble(), label[1].toDouble());
                painter.setPen(halo);
                painter.drawText(at + QPointF(1, 1), PetuniaTheme::axisLetter(i));
                painter.setPen(color);
                painter.drawText(at, PetuniaTheme::axisLetter(i));
            }
        }
    }

    // Planos: quadrados translúcidos entre dois eixos.
    if (showPlanes) {
        const struct { const char *key; int axis; } planes[] = {
            {"plane_xy_commands", 2},
            {"plane_xz_commands", 1},
            {"plane_yz_commands", 0},
        };
        for (const auto &plane : planes) {
            const QPainterPath path = petuniaPath(gizmo[plane.key].toString());
            if (path.isEmpty())
                continue;
            const bool hot = hoverAxis == plane.axis || activeAxis == plane.axis;
            QColor fill = theme.axisColor(plane.axis);
            QColor edge = fill;
            fill.setAlpha(hot ? 150 : 82);
            edge.setAlpha(hot ? 255 : 190);
            painter.setPen(QPen(edge, 1.2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.setBrush(fill);
            painter.drawPath(path);
        }
    }

    if (isRotate) {
        const QString roll = gizmo["view_roll_commands"].toString();
        if (!roll.isEmpty()) {
            QColor ring = theme.textPrimary();
            ring.setAlpha(110);
            painter.setPen(QPen(ring, 1.2, Qt::DashLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawPath(petuniaPath(roll));
        }
    }

    // Alça central: translação livre (Mover), escala uniforme (Escalar) ou
    // trackball (Girar). No gizmo combinado é só um ponto de referência.
    const QPointF origin(gizmo["origin_x"].toDouble(), gizmo["origin_y"].toDouble());
    painter.setBrush(Qt::NoBrush);
    if (isTransform) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(theme.textPrimary());
        painter.drawEllipse(origin, 3.0, 3.0);
    } else {
        QColor ring = centerHot ? theme.accentSelectionActive() : theme.textPrimary();
        painter.setPen(QPen(halo, 4.0));
        painter.drawEllipse(origin, 6.5, 6.5);
        painter.setPen(QPen(ring, 1.8));
        painter.drawEllipse(origin, 6.5, 6.5);
    }
    painter.restore();
}

void PetuniaGlWidget::drawSnapIndicator(QPainter &painter) {
    painter.save();
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(palette().color(QPalette::Highlight), 1.5));
    for (const auto &key :
         QStringList{"protractor_commands", "protractor_ticks_commands",
                     "operation_preview_commands", "loop_cut_preview_commands",
                     "profile_preview_commands", "profile_outline_commands",
                     "profile_selection_commands", "poly_pen_preview_commands",
                     "slice_preview_commands", "region_shapes_commands",
                     "region_hover_commands", "axis_guide_commands",
                     "dimension_commands", "measure_commands",
                     "proportional_circle_commands", "drag_link_commands"})
        painter.drawPath(petuniaPath(m_overlayState[key].toString()));
    auto animate = m_overlayState["animate"].toObject();
    for (const auto &key : QStringList{
             "bone_commands", "joint_commands", "selected_bone_commands",
             "ghost_commands", "motion_trail_commands"})
        painter.drawPath(petuniaPath(animate[key].toString()));
    if (m_overlayState["snap_marker_visible"].toBool()) {
        QPointF point(m_overlayState["snap_marker_x"].toDouble(),
                      m_overlayState["snap_marker_y"].toDouble());
        painter.drawEllipse(point, 5, 5);
        painter.drawText(point + QPointF(8, -8),
                         m_overlayState["snap_marker_label"].toString());
    }
    if (m_overlayState["measure_visible"].toBool())
        painter.drawText(QPointF(m_overlayState["measure_x"].toDouble(),
                                 m_overlayState["measure_y"].toDouble()),
                         m_overlayState["measure_text"].toString());
    for (const auto &key : QStringList{"measure_tags", "world_axis_labels"})
        for (const auto &value : m_overlayState[key].toArray()) {
            const auto label = value.toObject();
            const auto rgb = label["color"].toArray();
            painter.setPen(
                rgb.size() == 3
                    ? QColor(rgb[0].toInt(), rgb[1].toInt(), rgb[2].toInt())
                    : palette().color(QPalette::Text));
            painter.drawText(
                QPointF(label["x"].toDouble(), label["y"].toDouble()),
                label["text"].toString());
        }
    if (m_regionSelecting) {
        painter.setPen(
            QPen(palette().color(QPalette::Highlight), 1, Qt::DashLine));
        if (m_lasso)
            painter.drawPolyline(m_regionPoints);
        else
            painter.drawRect(QRectF(m_pressPoint, m_lastMousePos).normalized());
    }
    painter.restore();
}

void PetuniaGlWidget::draw3dCursor(QPainter &painter) {
    const auto gizmo = m_overlayState["gizmo"].toObject();
    const bool visible = m_overlayState["cursor_visible"].toBool() ||
                         gizmo["cursor_visible"].toBool();
    if (!visible)
        return;

    auto cursorArr = m_overlayState["cursor_screen"].toArray();
    if (cursorArr.size() < 2)
        cursorArr = gizmo["cursor_screen"].toArray();
    if (cursorArr.size() < 2)
        return;

    const double cx = cursorArr[0].toDouble();
    const double cy = cursorArr[1].toDouble();

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    // Anel externo branco tracejado sutil e fino
    QPen dashPen(QColor(255, 255, 255, 180), 1.0, Qt::DashLine);
    dashPen.setDashPattern({3.0, 3.0});
    painter.setBrush(Qt::NoBrush);
    painter.setPen(dashPen);
    painter.drawEllipse(QPointF(cx, cy), 12, 12);

    // Anel interno vermelho de precisão
    painter.setPen(QPen(QColor(229, 77, 66, 200), 1.0));
    painter.drawEllipse(QPointF(cx, cy), 9, 9);

    // Mira cruzada sutil (crosshair central fino)
    painter.setPen(QPen(QColor(255, 255, 255, 140), 1.0));
    painter.drawLine(QPointF(cx - 15, cy), QPointF(cx - 12, cy));
    painter.drawLine(QPointF(cx + 12, cy), QPointF(cx + 15, cy));
    painter.drawLine(QPointF(cx, cy - 15), QPointF(cx, cy - 12));
    painter.drawLine(QPointF(cx, cy + 12), QPointF(cx, cy + 15));

    painter.restore();
}

void PetuniaGlWidget::drawBrushCursor(QPainter &painter) {
    if (!m_brushCursorVisible || !underMouse())
        return;
    const auto state = m_overlayState;
    const QString tool = state[QStringLiteral("active_tool")].toString();
    // Mesmo diâmetro (px de tela) que o carimbo e o canvas 2D usam.
    const double diameter = qMax(2.0, state[QStringLiteral("brush_size")].toDouble(1.0) * 16.0);
    const double radius = diameter * 0.5;
    const bool erase = tool == QStringLiteral("eraser");
    // Pixel: borda sempre dura, sem anel de núcleo.
    const bool hardEdge = tool == QStringLiteral("pixel");
    const double hardness =
        qBound(0.0, state[QStringLiteral("brush_hardness")].toDouble(0.0), 1.0);
    const double strength = qBound(
        0.0,
        state[QStringLiteral("brush_opacity")].toDouble(1.0) *
            state[QStringLiteral("brush_flow")].toDouble(1.0),
        1.0);
    QColor tint(181, 140, 255);
    const auto color = state[QStringLiteral("paint_color")].toArray();
    if (color.size() >= 3)
        tint = QColor::fromRgbF(color[0].toDouble(), color[1].toDouble(),
                                color[2].toDouble());
    const QColor ring =
        erase ? QColor(239, 107, 115, 255) : QColor(255, 255, 255, 221);
    QColor fill = erase ? QColor(239, 107, 115, 255) : tint;
    fill.setAlphaF(0.04 + 0.32 * strength);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    QTransform transform;
    transform.translate(m_lastMousePos.x(), m_lastMousePos.y());
    transform.rotate(m_brushTilt);
    transform.scale(1.0, m_brushSquash);
    painter.setTransform(transform, true);
    painter.setBrush(fill);
    painter.setPen(QPen(ring, 1.5));
    painter.drawEllipse(QPointF(0, 0), radius, radius);
    // Contorno escuro interno: o anel lê bem sobre superfícies claras e escuras.
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(QColor(0, 0, 0, 170), 1.0));
    painter.drawEllipse(QPointF(0, 0), qMax(1.0, radius - 1.5),
                        qMax(1.0, radius - 1.5));
    if (!hardEdge && hardness > 0.02 && hardness < 0.98 && diameter > 10.0) {
        painter.setPen(QPen(QColor(255, 255, 255, 178), 1.0));
        painter.drawEllipse(QPointF(0, 0), radius * hardness, radius * hardness);
    }
    painter.restore();
    // O ponto central marca o cursor e não se deforma.
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(0, 0, 0, 170), 1.0));
    painter.setBrush(QColor(255, 255, 255));
    painter.drawEllipse(QPointF(m_lastMousePos), 1.5, 1.5);
    painter.restore();
}

void PetuniaGlWidget::drawDecalGizmo(QPainter &painter) {
    const auto state = m_overlayState;
    if (state[QStringLiteral("workspace")].toString().toUpper() !=
        QStringLiteral("PAINT"))
        return;
    if (!state[QStringLiteral("active_layer_is_decal")].toBool())
        return;
    // O modo escolhido no painel legenda as alças; a geometria é a mesma.
    const auto mode = state[QStringLiteral("decal_gizmo_mode")].toString();
    QColor accent(233, 106, 0);
    if (mode == QStringLiteral("rotate"))
        accent = QColor(79, 195, 247);
    else if (mode == QStringLiteral("scale"))
        accent = QColor(129, 199, 132);

    const auto commands =
        state[QStringLiteral("decal_preview_commands")].toString();
    if (!commands.isEmpty()) {
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        QColor fill = accent;
        fill.setAlphaF(0.10);
        painter.setPen(QPen(accent, 1.5));
        painter.setBrush(fill);
        painter.drawPath(petuniaPath(commands));
        painter.restore();
    }

    // Alças só com uma ferramenta de decalque: a ferramenta Select seleciona
    // objetos e não arrasta o decalque (mesma regra do bridge).
    if (state[QStringLiteral("active_tool")].toString() ==
        QStringLiteral("select"))
        return;
    const auto handles = state[QStringLiteral("decal_handles")].toObject();
    if (handles.isEmpty())
        return;
    const auto hovered =
        state[QStringLiteral("decal_hover_handle")].toString();
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    // Mover: alvo circular no centro.
    const auto center = pointFromArray(handles[QStringLiteral("center")]);
    painter.setPen(QPen(QColor(18, 18, 18, 220), 1.2));
    painter.setBrush(hovered == QStringLiteral("move") ? accent
                                                       : QColor(24, 24, 24, 210));
    painter.drawEllipse(center, 5.5, 5.5);
    // Escalar: quadrados nos cantos (alvo de 24 px, WCAG 2.2).
    for (const auto &corner : handles[QStringLiteral("corners")].toArray()) {
        const auto point = pointFromArray(corner);
        painter.setBrush(hovered == QStringLiteral("scale")
                             ? accent
                             : QColor(255, 255, 255, 230));
        painter.drawRect(QRectF(point.x() - 5.0, point.y() - 5.0, 10.0, 10.0));
    }
    // Girar: haste até o botão acima da borda superior.
    const auto top = pointFromArray(handles[QStringLiteral("top")]);
    const auto rotate = pointFromArray(handles[QStringLiteral("rotate")]);
    painter.setPen(QPen(accent, 1.5));
    painter.drawLine(top, rotate);
    painter.setPen(QPen(QColor(18, 18, 18, 220), 1.2));
    painter.setBrush(hovered == QStringLiteral("rotate")
                         ? accent
                         : QColor(129, 199, 132));
    painter.drawEllipse(rotate, 6.0, 6.0);
    painter.restore();
}

void PetuniaGlWidget::cleanupGl() {
    if (!m_rendererInitialized || !m_gl || !m_gl->context())
        return;
    m_gl->makeCurrent();
    petunia_qt_gl_surface_release(m_surfaceId);
    m_gl->doneCurrent();
    m_rendererInitialized = false;
}
QJsonObject PetuniaGlWidget::pointer(int phase, const QPointF &point,
                                     Qt::KeyboardModifiers mods) {
    auto state =
        petuniaRequest("pointer", {{"phase", phase},
                                   {"x", point.x()},
                                   {"y", point.y()},
                                   {"shift", bool(mods & Qt::ShiftModifier)},
                                   {"ctrl", bool(mods & Qt::ControlModifier)},
                                   {"alt", bool(mods & Qt::AltModifier)}});
    m_overlayState = state;
    return state;
}
