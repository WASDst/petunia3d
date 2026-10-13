#include "petunia_paint_panel.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGroupBox>
#include <QColorDialog>
#include <QFileDialog>
#include <QPainter>
#include <QPaintEvent>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QScrollArea>
#include <QRadialGradient>
#include <QMessageBox>
#include <QButtonGroup>

// =========================================================================
// PetuniaPaintCanvasWidget
// =========================================================================

PetuniaPaintCanvasWidget::PetuniaPaintCanvasWidget(QWidget *parent)
    : QWidget(parent)
    , m_canvasImage(512, 512, QImage::Format_ARGB32_Premultiplied)
{
    setMinimumSize(256, 256);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    setMouseTracking(true);
    clearCanvas(QColor(230, 230, 230));
}

void PetuniaPaintCanvasWidget::clearCanvas(const QColor &fillColor) {
    m_canvasImage.fill(fillColor);
    update();
}

void PetuniaPaintCanvasWidget::saveImage(const QString &filePath) {
    m_canvasImage.save(filePath, "PNG");
}

QPointF PetuniaPaintCanvasWidget::widgetToImage(const QPointF &widgetPos) const {
    float cx = width() * 0.5f + m_panOffset.x();
    float cy = height() * 0.5f + m_panOffset.y();
    float side = qMin(width(), height()) * 0.85f * m_zoom;
    float startX = cx - side * 0.5f;
    float startY = cy - side * 0.5f;

    float ix = ((widgetPos.x() - startX) / side) * m_canvasImage.width();
    float iy = ((widgetPos.y() - startY) / side) * m_canvasImage.height();
    return QPointF(ix, iy);
}

QPointF PetuniaPaintCanvasWidget::imageToWidget(const QPointF &imgPos) const {
    float cx = width() * 0.5f + m_panOffset.x();
    float cy = height() * 0.5f + m_panOffset.y();
    float side = qMin(width(), height()) * 0.85f * m_zoom;
    float startX = cx - side * 0.5f;
    float startY = cy - side * 0.5f;

    float wx = startX + (imgPos.x() / m_canvasImage.width()) * side;
    float wy = startY + (imgPos.y() / m_canvasImage.height()) * side;
    return QPointF(wx, wy);
}

void PetuniaPaintCanvasWidget::paintAtPoint(const QPointF &pt) {
    if (m_activeTool == 3) { // Eyedropper
        int x = qBound(0, static_cast<int>(pt.x()), m_canvasImage.width() - 1);
        int y = qBound(0, static_cast<int>(pt.y()), m_canvasImage.height() - 1);
        QColor sampled = m_canvasImage.pixelColor(x, y);
        emit colorSampled(sampled);
        return;
    }

    if (m_activeTool == 2) { // Preenchimento / Bucket
        m_canvasImage.fill(m_brushColor);
        update();
        emit strokeFinished();
        return;
    }

    QPainter painter(&m_canvasImage);
    painter.setRenderHint(QPainter::Antialiasing, true);

    auto drawBrushSpot = [&](const QPointF &pos) {
        float r = m_brushSize * 0.5f;
        if (m_activeTool == 1) { // Borracha
            painter.setCompositionMode(QPainter::CompositionMode_Clear);
            painter.setBrush(Qt::transparent);
            painter.setPen(Qt::NoPen);
            painter.drawEllipse(pos, r, r);
        } else { // Pincel normal
            painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
            QRadialGradient grad(pos, r);
            QColor inner = m_brushColor;
            inner.setAlphaF(qBound(0.0f, m_brushOpacity, 1.0f));
            QColor mid = inner;
            mid.setAlphaF(inner.alphaF() * m_brushHardness);
            QColor outer = inner;
            outer.setAlphaF(0.0f);

            grad.setColorAt(0.0, inner);
            grad.setColorAt(qBound(0.0f, m_brushHardness, 0.99f), mid);
            grad.setColorAt(1.0, outer);

            painter.setPen(Qt::NoPen);
            painter.setBrush(grad);
            painter.drawEllipse(pos, r, r);
        }
    };

    drawBrushSpot(pt);

    // Simetria X
    if (m_symmetryX) {
        float symX = m_canvasImage.width() - pt.x();
        drawBrushSpot(QPointF(symX, pt.y()));
    }
    // Simetria Y
    if (m_symmetryY) {
        float symY = m_canvasImage.height() - pt.y();
        drawBrushSpot(QPointF(pt.x(), symY));
    }
    // Simetria Dupla
    if (m_symmetryX && m_symmetryY) {
        float symX = m_canvasImage.width() - pt.x();
        float symY = m_canvasImage.height() - pt.y();
        drawBrushSpot(QPointF(symX, symY));
    }

    update();
}

void PetuniaPaintCanvasWidget::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.fillRect(rect(), QColor(24, 24, 24));

    float cx = width() * 0.5f + m_panOffset.x();
    float cy = height() * 0.5f + m_panOffset.y();
    float side = qMin(width(), height()) * 0.85f * m_zoom;
    QRectF destRect(cx - side * 0.5f, cy - side * 0.5f, side, side);

    // 1. Padrão de Tabuleiro / Checkerboard
    int checkSize = 16;
    int cols = qCeil(destRect.width() / checkSize) + 1;
    int rows = qCeil(destRect.height() / checkSize) + 1;
    painter.save();
    painter.setClipRect(destRect);
    for (int r = 0; r < rows; ++r) {
        for (int c = 0; c < cols; ++c) {
            QColor checkCol = ((r + c) % 2 == 0) ? QColor(36, 36, 36) : QColor(48, 48, 48);
            painter.fillRect(destRect.x() + c * checkSize, destRect.y() + r * checkSize, checkSize, checkSize, checkCol);
        }
    }

    // 2. Imagem da Textura
    painter.drawImage(destRect, m_canvasImage);
    painter.restore();

    // 3. Moldura de Limite da Textura
    painter.setPen(QPen(QColor(79, 195, 247, 180), 1.5));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(destRect);

    // 4. Cursor Circular do Pincel
    if (underMouse() && (m_activeTool == 0 || m_activeTool == 1)) {
        float screenR = (m_brushSize * 0.5f / m_canvasImage.width()) * side;
        painter.setPen(QPen(QColor(255, 255, 255, 200), 1.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawEllipse(m_lastMousePos, static_cast<int>(screenR), static_cast<int>(screenR));
    }
}

void PetuniaPaintCanvasWidget::mousePressEvent(QMouseEvent *event) {
    m_lastMousePos = event->pos();
    if (event->button() == Qt::MiddleButton || (event->button() == Qt::LeftButton && (event->modifiers() & Qt::AltModifier))) {
        m_isPanning = true;
        setCursor(Qt::ClosedHandCursor);
    } else if (event->button() == Qt::LeftButton) {
        m_isDrawing = true;
        QPointF imgPt = widgetToImage(event->pos());
        paintAtPoint(imgPt);
    }
}

void PetuniaPaintCanvasWidget::mouseMoveEvent(QMouseEvent *event) {
    if (m_isPanning) {
        QPoint delta = event->pos() - m_lastMousePos;
        m_panOffset += delta;
        m_lastMousePos = event->pos();
        update();
    } else if (m_isDrawing) {
        m_lastMousePos = event->pos();
        QPointF imgPt = widgetToImage(event->pos());
        paintAtPoint(imgPt);
    } else {
        m_lastMousePos = event->pos();
        update(); // Atualiza cursor circular
    }
}

void PetuniaPaintCanvasWidget::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::MiddleButton || (event->modifiers() & Qt::AltModifier)) {
        m_isPanning = false;
        unsetCursor();
    } else if (event->button() == Qt::LeftButton) {
        if (m_isDrawing) {
            m_isDrawing = false;
            emit strokeFinished();
        }
    }
}

void PetuniaPaintCanvasWidget::wheelEvent(QWheelEvent *event) {
    float factor = (event->angleDelta().y() > 0) ? 1.15f : 0.87f;
    m_zoom = qBound(0.2f, m_zoom * factor, 10.0f);
    update();
}

// =========================================================================
// PetuniaPaintPanel
// =========================================================================

PetuniaPaintPanel::PetuniaPaintPanel(QWidget *parent)
    : QWidget(parent)
{
    setupUi();
}

void PetuniaPaintPanel::setupUi() {
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(6, 6, 6, 6);
    mainLayout->setSpacing(8);

    setStyleSheet(QStringLiteral(
        "QGroupBox { color: #b5b5b5; font-weight: bold; border: 1px solid #404040; border-radius: 4px; margin-top: 8px; padding-top: 8px; }"
        "QPushButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 4px 8px; font-weight: bold; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; }"
        "QPushButton:checked { background-color: #b58cff; color: #121212; }"
        "QComboBox, QSpinBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 3px; }"
        "QListWidget { background-color: #202020; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; selection-background-color: #b58cff; selection-color: #121212; }"
    ));

    // 1. Barra Superior de Ferramentas e Simetria
    auto *topRow = new QHBoxLayout();
    m_brushBtn = new QPushButton(QStringLiteral("🖌 Pincel"), this);
    m_brushBtn->setCheckable(true);
    m_brushBtn->setChecked(true);
    m_eraserBtn = new QPushButton(QStringLiteral("⌫ Borracha"), this);
    m_eraserBtn->setCheckable(true);
    m_bucketBtn = new QPushButton(QStringLiteral("🪣 Preencher"), this);
    m_bucketBtn->setCheckable(true);
    m_eyedropperBtn = new QPushButton(QStringLiteral("💉 Amostra"), this);
    m_eyedropperBtn->setCheckable(true);

    auto *toolGroup = new QButtonGroup(this);
    toolGroup->addButton(m_brushBtn, 0);
    toolGroup->addButton(m_eraserBtn, 1);
    toolGroup->addButton(m_bucketBtn, 2);
    toolGroup->addButton(m_eyedropperBtn, 3);

    connect(toolGroup, &QButtonGroup::idClicked, this, [this](int id) {
        if (m_paintCanvas) m_paintCanvas->setTool(id);
    });

    topRow->addWidget(m_brushBtn);
    topRow->addWidget(m_eraserBtn);
    topRow->addWidget(m_bucketBtn);
    topRow->addWidget(m_eyedropperBtn);

    m_symmetryXCheck = new QCheckBox(QStringLiteral("Espelho X"), this);
    m_symmetryYCheck = new QCheckBox(QStringLiteral("Espelho Y"), this);
    connect(m_symmetryXCheck, &QCheckBox::toggled, this, [this](bool c) {
        if (m_paintCanvas) m_paintCanvas->setSymmetryX(c);
    });
    connect(m_symmetryYCheck, &QCheckBox::toggled, this, [this](bool c) {
        if (m_paintCanvas) m_paintCanvas->setSymmetryY(c);
    });
    topRow->addWidget(m_symmetryXCheck);
    topRow->addWidget(m_symmetryYCheck);

    mainLayout->addLayout(topRow);

    // 2. Canvas 2D Interativo
    m_paintCanvas = new PetuniaPaintCanvasWidget(this);
    m_paintCanvas->setFixedHeight(220);
    connect(m_paintCanvas, &PetuniaPaintCanvasWidget::colorSampled, this, [this](const QColor &c) {
        applyPaletteColor(c);
    });
    mainLayout->addWidget(m_paintCanvas);

    // Barra de Ações Rápidas do Canvas (Exportar / Limpar)
    auto *canvasActRow = new QHBoxLayout();
    m_clearBtn = new QPushButton(QStringLiteral("Limpar Canvas"), this);
    connect(m_clearBtn, &QPushButton::clicked, this, [this]() {
        if (m_paintCanvas) m_paintCanvas->clearCanvas(QColor(230, 230, 230));
    });
    canvasActRow->addWidget(m_clearBtn);

    m_exportBtn = new QPushButton(QStringLiteral("💾 Exportar PNG"), this);
    connect(m_exportBtn, &QPushButton::clicked, this, &PetuniaPaintPanel::exportCanvasPng);
    canvasActRow->addWidget(m_exportBtn);
    mainLayout->addLayout(canvasActRow);

    // 3. Propriedades do Pincel (ScrollArea para não estourar layout)
    auto *scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);

    auto *scrollContent = new QWidget(scroll);
    auto *propsLayout = new QVBoxLayout(scrollContent);
    propsLayout->setContentsMargins(0, 0, 0, 0);

    auto *brushGroup = new QGroupBox(QStringLiteral("Propriedades do Pincel"), scrollContent);
    auto *form = new QFormLayout(brushGroup);
    form->setContentsMargins(6, 6, 6, 6);
    form->setSpacing(6);

    // Tamanho
    auto *sizeRow = new QHBoxLayout();
    m_sizeSlider = new QSlider(Qt::Horizontal, brushGroup);
    m_sizeSlider->setRange(1, 256);
    m_sizeSlider->setValue(32);
    m_sizeSpin = new QSpinBox(brushGroup);
    m_sizeSpin->setRange(1, 256);
    m_sizeSpin->setValue(32);
    m_sizeSpin->setSuffix(QStringLiteral(" px"));
    connect(m_sizeSlider, &QSlider::valueChanged, m_sizeSpin, &QSpinBox::setValue);
    connect(m_sizeSpin, QOverload<int>::of(&QSpinBox::valueChanged), m_sizeSlider, &QSlider::setValue);
    connect(m_sizeSlider, &QSlider::valueChanged, this, [this](int sz) {
        if (m_paintCanvas) m_paintCanvas->setBrushSize(sz);
        emit brushSizeChanged(sz);
    });
    sizeRow->addWidget(m_sizeSlider);
    sizeRow->addWidget(m_sizeSpin);
    form->addRow(QStringLiteral("Tamanho:"), sizeRow);

    // Dureza
    auto *hardRow = new QHBoxLayout();
    m_hardnessSlider = new QSlider(Qt::Horizontal, brushGroup);
    m_hardnessSlider->setRange(0, 100);
    m_hardnessSlider->setValue(80);
    m_hardnessLabel = new QLabel(QStringLiteral("80%"), brushGroup);
    connect(m_hardnessSlider, &QSlider::valueChanged, this, [this](int v) {
        m_hardnessLabel->setText(QStringLiteral("%1%").arg(v));
        float hd = v / 100.0f;
        if (m_paintCanvas) m_paintCanvas->setBrushHardness(hd);
        emit brushHardnessChanged(hd);
    });
    hardRow->addWidget(m_hardnessSlider);
    hardRow->addWidget(m_hardnessLabel);
    form->addRow(QStringLiteral("Dureza:"), hardRow);

    // Opacidade
    auto *opacRow = new QHBoxLayout();
    m_opacitySlider = new QSlider(Qt::Horizontal, brushGroup);
    m_opacitySlider->setRange(1, 100);
    m_opacitySlider->setValue(100);
    m_opacityLabel = new QLabel(QStringLiteral("100%"), brushGroup);
    connect(m_opacitySlider, &QSlider::valueChanged, this, [this](int v) {
        m_opacityLabel->setText(QStringLiteral("%1%").arg(v));
        float op = v / 100.0f;
        if (m_paintCanvas) m_paintCanvas->setBrushOpacity(op);
        emit brushOpacityChanged(op);
    });
    opacRow->addWidget(m_opacitySlider);
    opacRow->addWidget(m_opacityLabel);
    form->addRow(QStringLiteral("Opacidade:"), opacRow);

    // Espaçamento (Spacing)
    auto *spacingRow = new QHBoxLayout();
    m_spacingSlider = new QSlider(Qt::Horizontal, brushGroup);
    m_spacingSlider->setRange(1, 100);
    m_spacingSlider->setValue(15);
    m_spacingLabel = new QLabel(QStringLiteral("15%"), brushGroup);
    connect(m_spacingSlider, &QSlider::valueChanged, this, [this](int v) {
        m_spacingLabel->setText(QStringLiteral("%1%").arg(v));
    });
    spacingRow->addWidget(m_spacingSlider);
    spacingRow->addWidget(m_spacingLabel);
    form->addRow(QStringLiteral("Espaçamento:"), spacingRow);

    // Jitter Tamanho
    auto *jitterRow = new QHBoxLayout();
    m_jitterSizeSlider = new QSlider(Qt::Horizontal, brushGroup);
    m_jitterSizeSlider->setRange(0, 100);
    m_jitterSizeSlider->setValue(0);
    m_jitterSizeLabel = new QLabel(QStringLiteral("0%"), brushGroup);
    connect(m_jitterSizeSlider, &QSlider::valueChanged, this, [this](int v) {
        m_jitterSizeLabel->setText(QStringLiteral("%1%").arg(v));
    });
    jitterRow->addWidget(m_jitterSizeSlider);
    jitterRow->addWidget(m_jitterSizeLabel);
    form->addRow(QStringLiteral("Jitter:"), jitterRow);

    // Suavização (Smoothing)
    auto *smoothRow = new QHBoxLayout();
    m_smoothingSlider = new QSlider(Qt::Horizontal, brushGroup);
    m_smoothingSlider->setRange(0, 100);
    m_smoothingSlider->setValue(50);
    m_smoothingLabel = new QLabel(QStringLiteral("50%"), brushGroup);
    connect(m_smoothingSlider, &QSlider::valueChanged, this, [this](int v) {
        m_smoothingLabel->setText(QStringLiteral("%1%").arg(v));
    });
    smoothRow->addWidget(m_smoothingSlider);
    smoothRow->addWidget(m_smoothingLabel);
    form->addRow(QStringLiteral("Suavização:"), smoothRow);

    // Modo de Mesclagem
    m_blendModeCombo = new QComboBox(brushGroup);
    m_blendModeCombo->addItem(QStringLiteral("Normal"));
    m_blendModeCombo->addItem(QStringLiteral("Multiplicação (Multiply)"));
    m_blendModeCombo->addItem(QStringLiteral("Adição (Add)"));
    m_blendModeCombo->addItem(QStringLiteral("Sobreposição (Overlay)"));
    connect(m_blendModeCombo, &QComboBox::currentTextChanged, this, &PetuniaPaintPanel::blendModeChanged);
    form->addRow(QStringLiteral("Blend Mode:"), m_blendModeCombo);

    propsLayout->addWidget(brushGroup);

    // 4. Seção Cores e Paleta de Amostras (Swatches)
    auto *colorGroup = new QGroupBox(QStringLiteral("Paleta de Cores & Swatches"), scrollContent);
    auto *colorLayout = new QVBoxLayout(colorGroup);
    colorLayout->setContentsMargins(6, 6, 6, 6);

    auto *colorBtnRow = new QHBoxLayout();
    m_colorPreviewBtn = new QPushButton(QStringLiteral("Alterar Cor Base"), colorGroup);
    m_colorPreviewBtn->setStyleSheet(QStringLiteral(
        "background-color: %1; color: #121212; font-weight: bold; border-radius: 4px; padding: 6px;"
    ).arg(m_currentColor.name()));
    connect(m_colorPreviewBtn, &QPushButton::clicked, this, &PetuniaPaintPanel::pickColor);
    colorBtnRow->addWidget(m_colorPreviewBtn);
    colorLayout->addLayout(colorBtnRow);

    // Grid de Swatches Rápidos
    m_swatchesContainer = new QWidget(colorGroup);
    m_swatchesLayout = new QHBoxLayout(m_swatchesContainer);
    m_swatchesLayout->setContentsMargins(0, 4, 0, 4);
    m_swatchesLayout->setSpacing(4);

    const QStringList swatches = {
        QStringLiteral("#e96a00"), // Petunia Laranja
        QStringLiteral("#b58cff"), // Roxo
        QStringLiteral("#4fc3f7"), // Ciano
        QStringLiteral("#81c784"), // Verde
        QStringLiteral("#ffd54f"), // Amarelo
        QStringLiteral("#e53935"), // Vermelho
        QStringLiteral("#ffffff"), // Branco
        QStringLiteral("#888888"), // Cinza
        QStringLiteral("#121212")  // Preto
    };

    for (const auto &hex : swatches) {
        auto *swBtn = new QPushButton(m_swatchesContainer);
        swBtn->setFixedSize(22, 22);
        swBtn->setCursor(Qt::PointingHandCursor);
        swBtn->setStyleSheet(QStringLiteral("background-color: %1; border: 1px solid #555555; border-radius: 3px;").arg(hex));
        connect(swBtn, &QPushButton::clicked, this, [this, hex]() {
            applyPaletteColor(QColor(hex));
        });
        m_swatchesLayout->addWidget(swBtn);
    }
    colorLayout->addWidget(m_swatchesContainer);

    // Geradores de Harmonia
    auto *harmRow = new QHBoxLayout();
    auto *compBtn = new QPushButton(QStringLiteral("Harmonia Complementar"), colorGroup);
    connect(compBtn, &QPushButton::clicked, this, &PetuniaPaintPanel::generateComplementaryHarmonies);
    harmRow->addWidget(compBtn);

    auto *triBtn = new QPushButton(QStringLiteral("Triádica"), colorGroup);
    connect(triBtn, &QPushButton::clicked, this, &PetuniaPaintPanel::generateTriadicHarmonies);
    harmRow->addWidget(triBtn);
    colorLayout->addLayout(harmRow);

    propsLayout->addWidget(colorGroup);

    // 5. Camadas de Textura
    auto *layersGroup = new QGroupBox(QStringLiteral("Camadas de Textura"), scrollContent);
    auto *layersLayout = new QVBoxLayout(layersGroup);
    layersLayout->setContentsMargins(6, 6, 6, 6);

    m_layersList = new QListWidget(layersGroup);
    m_layersList->addItem(QStringLiteral("👁 [100%] Camada Base (Albedo)"));
    m_layersList->setCurrentRow(0);
    layersLayout->addWidget(m_layersList);

    auto *layerOpacRow = new QHBoxLayout();
    auto *opacLabel = new QLabel(QStringLiteral("Opacidade:"), layersGroup);
    m_layerOpacitySlider = new QSlider(Qt::Horizontal, layersGroup);
    m_layerOpacitySlider->setRange(0, 100);
    m_layerOpacitySlider->setValue(100);
    layerOpacRow->addWidget(opacLabel);
    layerOpacRow->addWidget(m_layerOpacitySlider);
    layersLayout->addLayout(layerOpacRow);

    auto *layersBtnRow = new QHBoxLayout();
    m_addLayerBtn = new QPushButton(QStringLiteral("+ Nova Camada"), layersGroup);
    connect(m_addLayerBtn, &QPushButton::clicked, this, [this]() {
        int idx = m_layersList->count() + 1;
        m_layersList->addItem(QStringLiteral("👁 [100%] Camada %1").arg(idx));
        emit addLayerRequested();
    });
    layersBtnRow->addWidget(m_addLayerBtn);

    m_removeLayerBtn = new QPushButton(QStringLiteral("- Excluir"), layersGroup);
    connect(m_removeLayerBtn, &QPushButton::clicked, this, [this]() {
        int row = m_layersList->currentRow();
        if (row > 0) { // Preserva a camada base
            delete m_layersList->takeItem(row);
            emit removeLayerRequested(row);
        }
    });
    layersBtnRow->addWidget(m_removeLayerBtn);
    layersLayout->addLayout(layersBtnRow);

    propsLayout->addWidget(layersGroup);

    scroll->setWidget(scrollContent);
    mainLayout->addWidget(scroll, 1);
}

void PetuniaPaintPanel::pickColor() {
    QColor col = QColorDialog::getColor(m_currentColor, this, QStringLiteral("Cor do Pincel"));
    if (col.isValid()) {
        applyPaletteColor(col);
    }
}

void PetuniaPaintPanel::applyPaletteColor(const QColor &color) {
    m_currentColor = color;
    QString textCol = color.lightness() > 130 ? QStringLiteral("#121212") : QStringLiteral("#eeeeee");
    m_colorPreviewBtn->setStyleSheet(QStringLiteral(
        "background-color: %1; color: %2; font-weight: bold; border-radius: 4px; padding: 6px;"
    ).arg(color.name(), textCol));
    if (m_paintCanvas) {
        m_paintCanvas->setBrushColor(color);
    }
    emit activeColorChanged(color);
}

void PetuniaPaintPanel::generateComplementaryHarmonies() {
    int h, s, v;
    m_currentColor.getHsv(&h, &s, &v);
    int compH = (h + 180) % 360;
    QColor compCol = QColor::fromHsv(compH, s, v);
    applyPaletteColor(compCol);
}

void PetuniaPaintPanel::generateTriadicHarmonies() {
    int h, s, v;
    m_currentColor.getHsv(&h, &s, &v);
    int triadH = (h + 120) % 360;
    QColor triadCol = QColor::fromHsv(triadH, s, v);
    applyPaletteColor(triadCol);
}

void PetuniaPaintPanel::exportCanvasPng() {
    QString path = QFileDialog::getSaveFileName(this, QStringLiteral("Salvar Textura em Imagem PNG"), QString(), QStringLiteral("PNG (*.png)"));
    if (!path.isEmpty()) {
        if (m_paintCanvas) {
            m_paintCanvas->saveImage(path);
            QMessageBox::information(this, QStringLiteral("Exportado"), QStringLiteral("Textura salva com sucesso em:\n%1").arg(path));
        }
    }
}
