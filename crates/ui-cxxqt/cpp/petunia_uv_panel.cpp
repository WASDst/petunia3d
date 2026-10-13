#include "petunia_uv_panel.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGroupBox>
#include <QLabel>
#include <QPainter>
#include <QMouseEvent>
#include <QWheelEvent>

extern "C" {
    int petunia_qt_app_get_active_asset();
    int petunia_qt_app_get_uv_face_count(int asset_index);
    int petunia_qt_app_get_uv_face_verts(int asset_index, int face_idx, float *out_uvs, int max_verts);
    bool petunia_qt_app_get_uv_diagnostics(int asset_index, int *island_count, float *overlap_ratio, float *mean_stretch);
}

PetuniaUvCanvasWidget::PetuniaUvCanvasWidget(QWidget *parent)
    : QWidget(parent)
{
    setMinimumHeight(240);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    setMouseTracking(true);
}

void PetuniaUvCanvasWidget::refresh() {
    update();
}

void PetuniaUvCanvasWidget::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);

    // Fundo do canvas 2D
    painter.fillRect(rect(), QColor(24, 24, 24));

    // Determina o quadrado unitário [0, 1] no espaço de tela com pan e zoom
    QPointF center = QPointF(width() / 2.0, height() / 2.0) + m_panOffset;
    float tileSize = qMin(width(), height()) * 0.8f * m_zoom;
    QPointF uvTopLeft = center - QPointF(tileSize / 2.0, tileSize / 2.0);
    QRectF uvRect(uvTopLeft, QSizeF(tileSize, tileSize));

    // 1. Padrão quadriculado (Checkerboard) dentro do tile UV [0, 1]
    const int checks = 8;
    float checkSize = tileSize / checks;
    for (int y = 0; y < checks; ++y) {
        for (int x = 0; x < checks; ++x) {
            QColor col = ((x + y) % 2 == 0) ? QColor(36, 36, 36) : QColor(48, 48, 48);
            painter.fillRect(QRectF(uvTopLeft.x() + x * checkSize, uvTopLeft.y() + y * checkSize, checkSize, checkSize), col);
        }
    }

    // 2. Linhas de grade e borda do espaço [0, 1]
    painter.setPen(QPen(QColor(80, 80, 80, 180), 1.0, Qt::DashLine));
    painter.drawLine(QPointF(uvTopLeft.x() + tileSize * 0.5, uvTopLeft.y()), QPointF(uvTopLeft.x() + tileSize * 0.5, uvTopLeft.y() + tileSize));
    painter.drawLine(QPointF(uvTopLeft.x(), uvTopLeft.y() + tileSize * 0.5), QPointF(uvTopLeft.x() + tileSize, uvTopLeft.y() + tileSize * 0.5));

    // Borda [0, 1]
    painter.setPen(QPen(QColor(233, 106, 0), 2.0)); // Laranja Petunia
    painter.drawRect(uvRect);

    // Rótulos de cantos (0,0 e 1,1)
    QFont font = painter.font();
    font.setPointSize(8);
    font.setBold(true);
    painter.setFont(font);
    painter.setPen(QColor(160, 160, 160));
    painter.drawText(uvTopLeft + QPointF(4, tileSize - 4), QStringLiteral("(0, 0)"));
    painter.drawText(uvTopLeft + QPointF(tileSize - 36, 12), QStringLiteral("(1, 1)"));

    // 3. Renderiza as faces UV da geometria ativa
    int activeAsset = petunia_qt_app_get_active_asset();
    if (activeAsset >= 0) {
        int faceCount = petunia_qt_app_get_uv_face_count(activeAsset);
        painter.setPen(QPen(QColor(79, 195, 247, 230), 1.5)); // Ciano brilhante
        painter.setBrush(QColor(79, 195, 247, 40));           // Preenchimento translúcido

        for (int i = 0; i < faceCount; ++i) {
            float uvs[32] = {0};
            int n = petunia_qt_app_get_uv_face_verts(activeAsset, i, uvs, 16);
            if (n < 3) continue;

            QPolygonF poly;
            for (int v = 0; v < n; ++v) {
                float u = uvs[v * 2];
                float val = uvs[v * 2 + 1];
                float sx = uvTopLeft.x() + u * tileSize;
                float sy = uvTopLeft.y() + (1.0f - val) * tileSize;
                poly.append(QPointF(sx, sy));
            }
            painter.drawPolygon(poly);

            // Vértices individuais destacados
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(255, 183, 77)); // Amarelo/Laranja suave
            for (const auto &pt : poly) {
                painter.drawEllipse(pt, 2.0, 2.0);
            }
            painter.setPen(QPen(QColor(79, 195, 247, 230), 1.5));
            painter.setBrush(QColor(79, 195, 247, 40));
        }
    }
}

void PetuniaUvCanvasWidget::mousePressEvent(QMouseEvent *event) {
    m_lastMousePos = event->pos();
    if (event->button() == Qt::LeftButton || event->button() == Qt::MiddleButton) {
        m_isPanning = true;
        event->accept();
    }
}

void PetuniaUvCanvasWidget::mouseMoveEvent(QMouseEvent *event) {
    if (m_isPanning) {
        QPoint delta = event->pos() - m_lastMousePos;
        m_lastMousePos = event->pos();
        m_panOffset += QPointF(delta.x(), delta.y());
        update();
        event->accept();
    }
}

void PetuniaUvCanvasWidget::mouseReleaseEvent(QMouseEvent *event) {
    m_isPanning = false;
    event->accept();
}

void PetuniaUvCanvasWidget::wheelEvent(QWheelEvent *event) {
    float factor = (event->angleDelta().y() > 0) ? 1.15f : 0.85f;
    m_zoom = qBound(0.2f, m_zoom * factor, 8.0f);
    update();
    event->accept();
}

PetuniaUvPanel::PetuniaUvPanel(QWidget *parent)
    : QWidget(parent)
{
    setupUi();
}

void PetuniaUvPanel::setupUi() {
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(8, 8, 8, 8);
    mainLayout->setSpacing(8);

    setStyleSheet(QStringLiteral(
        "QGroupBox { color: #b5b5b5; font-weight: bold; border: 1px solid #404040; border-radius: 4px; margin-top: 10px; padding-top: 10px; }"
        "QPushButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 6px 12px; font-weight: bold; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; }"
        "QComboBox, QDoubleSpinBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 3px; }"
    ));

    // 1. Canvas 2D Interativo de Coordenadas UV
    auto *canvasGroup = new QGroupBox(QStringLiteral("Canvas 2D de Ilhas UV"), this);
    auto *canvasLayout = new QVBoxLayout(canvasGroup);
    canvasLayout->setContentsMargins(4, 4, 4, 4);

    m_uvCanvas = new PetuniaUvCanvasWidget(canvasGroup);
    canvasLayout->addWidget(m_uvCanvas);
    mainLayout->addWidget(canvasGroup, 1);

    // 2. Diagnósticos e Métricas UV
    auto *diagGroup = new QGroupBox(QStringLiteral("Diagnósticos UV"), this);
    auto *diagForm = new QFormLayout(diagGroup);
    diagForm->setContentsMargins(8, 8, 8, 8);

    m_diagIslandsLabel = new QLabel(QStringLiteral("1"), diagGroup);
    m_diagIslandsLabel->setStyleSheet(QStringLiteral("font-weight: bold; color: #4fc3f7;"));
    diagForm->addRow(QStringLiteral("Ilhas UV:"), m_diagIslandsLabel);

    m_diagOverlapLabel = new QLabel(QStringLiteral("0.0%"), diagGroup);
    m_diagOverlapLabel->setStyleSheet(QStringLiteral("font-weight: bold; color: #81c784;"));
    diagForm->addRow(QStringLiteral("Sobreposição:"), m_diagOverlapLabel);

    m_diagStretchLabel = new QLabel(QStringLiteral("Ótimo (1.00)"), diagGroup);
    m_diagStretchLabel->setStyleSheet(QStringLiteral("font-weight: bold; color: #ffb74d;"));
    diagForm->addRow(QStringLiteral("Distorção:"), m_diagStretchLabel);

    mainLayout->addWidget(diagGroup);

    // 3. Projeções e Desdobramento (Unwrap)
    auto *unwrapGroup = new QGroupBox(QStringLiteral("Desdobramento UV"), this);
    auto *unwrapLayout = new QVBoxLayout(unwrapGroup);

    m_unwrapSmartBtn = new QPushButton(QStringLiteral("⚡ Unwrap Inteligente"), unwrapGroup);
    connect(m_unwrapSmartBtn, &QPushButton::clicked, this, [this]() {
        emit unwrapRequested("smart");
        m_uvCanvas->refresh();
        refreshDiagnostics();
    });
    unwrapLayout->addWidget(m_unwrapSmartBtn);

    m_unwrapCubeBtn = new QPushButton(QStringLiteral("📦 Projeção Cúbica"), unwrapGroup);
    connect(m_unwrapCubeBtn, &QPushButton::clicked, this, [this]() {
        emit unwrapRequested("cube");
        m_uvCanvas->refresh();
        refreshDiagnostics();
    });
    unwrapLayout->addWidget(m_unwrapCubeBtn);

    mainLayout->addWidget(unwrapGroup);

    // 4. Costuras (Seams)
    auto *seamGroup = new QGroupBox(QStringLiteral("Costuras na Malha"), this);
    auto *seamLayout = new QHBoxLayout(seamGroup);

    m_markSeamBtn = new QPushButton(QStringLiteral("Marcar Costura"), seamGroup);
    connect(m_markSeamBtn, &QPushButton::clicked, this, &PetuniaUvPanel::markSeamRequested);
    seamLayout->addWidget(m_markSeamBtn);

    m_clearSeamBtn = new QPushButton(QStringLiteral("Limpar"), seamGroup);
    connect(m_clearSeamBtn, &QPushButton::clicked, this, &PetuniaUvPanel::clearSeamRequested);
    seamLayout->addWidget(m_clearSeamBtn);

    mainLayout->addWidget(seamGroup);

    // 5. Empacotamento de Ilhas UV (Packing)
    auto *packGroup = new QGroupBox(QStringLiteral("Empacotamento de Ilhas"), this);
    auto *packForm = new QFormLayout(packGroup);
    packForm->setContentsMargins(8, 8, 8, 8);

    m_marginSpin = new QDoubleSpinBox(packGroup);
    m_marginSpin->setRange(0.001, 0.2);
    m_marginSpin->setValue(0.02);
    m_marginSpin->setSingleStep(0.005);
    packForm->addRow(QStringLiteral("Margem:"), m_marginSpin);

    m_packBtn = new QPushButton(QStringLiteral("Empacotar Ilhas UV"), packGroup);
    connect(m_packBtn, &QPushButton::clicked, this, [this]() {
        emit packIslandsRequested(static_cast<float>(m_marginSpin->value()));
        m_uvCanvas->refresh();
        refreshDiagnostics();
    });
    packForm->addRow(m_packBtn);

    mainLayout->addWidget(packGroup);
}

void PetuniaUvPanel::refreshDiagnostics() {
    int active = petunia_qt_app_get_active_asset();
    if (active < 0) return;

    int islands = 0;
    float overlap = 0.0f;
    float stretch = 0.0f;
    if (petunia_qt_app_get_uv_diagnostics(active, &islands, &overlap, &stretch)) {
        if (m_diagIslandsLabel) m_diagIslandsLabel->setText(QString::number(islands));
        if (m_diagOverlapLabel) {
            m_diagOverlapLabel->setText(QStringLiteral("%1%").arg(overlap * 100.0f, 0, 'f', 1));
            m_diagOverlapLabel->setStyleSheet(overlap > 0.01f ? QStringLiteral("font-weight: bold; color: #ff5252;") : QStringLiteral("font-weight: bold; color: #81c784;"));
        }
        if (m_diagStretchLabel) {
            m_diagStretchLabel->setText(QString::number(stretch, 'f', 2));
        }
    }
}
