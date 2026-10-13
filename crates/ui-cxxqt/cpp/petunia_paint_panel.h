#pragma once

#include <QWidget>
#include <QSlider>
#include <QSpinBox>
#include <QDoubleSpinBox>
#include <QComboBox>
#include <QListWidget>
#include <QPushButton>
#include <QLabel>
#include <QCheckBox>
#include <QImage>
#include <QColor>
#include <QPointF>
#include <QHBoxLayout>

class PetuniaPaintCanvasWidget : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaPaintCanvasWidget(QWidget *parent = nullptr);
    ~PetuniaPaintCanvasWidget() override = default;

    void setBrushColor(const QColor &c) { m_brushColor = c; }
    void setBrushSize(int sz) { m_brushSize = sz; update(); }
    void setBrushOpacity(float op) { m_brushOpacity = op; }
    void setBrushHardness(float hd) { m_brushHardness = hd; }
    void setTool(int tool) { m_activeTool = tool; update(); } // 0=Brush, 1=Eraser, 2=Bucket, 3=Eyedropper
    void setSymmetryX(bool sym) { m_symmetryX = sym; }
    void setSymmetryY(bool sym) { m_symmetryY = sym; }

    void clearCanvas(const QColor &fillColor = QColor(240, 240, 240));
    void saveImage(const QString &filePath);
    const QImage& canvasImage() const { return m_canvasImage; }

signals:
    void colorSampled(const QColor &color);
    void strokeFinished();

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    void paintAtPoint(const QPointF &pt);
    QPointF widgetToImage(const QPointF &widgetPos) const;
    QPointF imageToWidget(const QPointF &imgPos) const;

    QImage m_canvasImage;
    float m_zoom{1.0f};
    QPointF m_panOffset{0.0f, 0.0f};
    QPoint m_lastMousePos;
    bool m_isPanning{false};
    bool m_isDrawing{false};

    QColor m_brushColor{233, 106, 0};
    int m_brushSize{32};
    float m_brushOpacity{1.0f};
    float m_brushHardness{0.8f};
    int m_activeTool{0}; // 0=Brush, 1=Eraser, 2=Bucket, 3=Eyedropper
    bool m_symmetryX{false};
    bool m_symmetryY{false};
};

class PetuniaPaintPanel : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaPaintPanel(QWidget *parent = nullptr);
    ~PetuniaPaintPanel() override = default;

    PetuniaPaintCanvasWidget* canvas() const { return m_paintCanvas; }

signals:
    void brushSizeChanged(int size);
    void brushHardnessChanged(float hardness);
    void brushOpacityChanged(float opacity);
    void blendModeChanged(const QString &mode);
    void activeColorChanged(const QColor &color);
    void addLayerRequested();
    void removeLayerRequested(int index);

private slots:
    void pickColor();
    void applyPaletteColor(const QColor &color);
    void generateComplementaryHarmonies();
    void generateTriadicHarmonies();
    void exportCanvasPng();

private:
    void setupUi();

    // Canvas 2D
    PetuniaPaintCanvasWidget *m_paintCanvas{nullptr};

    // Ferramentas de Pintura
    QPushButton *m_brushBtn{nullptr};
    QPushButton *m_eraserBtn{nullptr};
    QPushButton *m_bucketBtn{nullptr};
    QPushButton *m_eyedropperBtn{nullptr};

    // Simetria
    QCheckBox *m_symmetryXCheck{nullptr};
    QCheckBox *m_symmetryYCheck{nullptr};

    // Parâmetros do Pincel
    QSlider *m_sizeSlider{nullptr};
    QSpinBox *m_sizeSpin{nullptr};
    QSlider *m_hardnessSlider{nullptr};
    QLabel *m_hardnessLabel{nullptr};
    QSlider *m_opacitySlider{nullptr};
    QLabel *m_opacityLabel{nullptr};
    QSlider *m_spacingSlider{nullptr};
    QLabel *m_spacingLabel{nullptr};
    QSlider *m_jitterSizeSlider{nullptr};
    QLabel *m_jitterSizeLabel{nullptr};
    QSlider *m_smoothingSlider{nullptr};
    QLabel *m_smoothingLabel{nullptr};
    QComboBox *m_blendModeCombo{nullptr};

    // Cor & Swatches
    QPushButton *m_colorPreviewBtn{nullptr};
    QColor m_currentColor{233, 106, 0}; // Laranja Petunia
    QWidget *m_swatchesContainer{nullptr};
    QHBoxLayout *m_swatchesLayout{nullptr};

    // Camadas de Pintura
    QListWidget *m_layersList{nullptr};
    QSlider *m_layerOpacitySlider{nullptr};
    QPushButton *m_addLayerBtn{nullptr};
    QPushButton *m_removeLayerBtn{nullptr};
    QPushButton *m_exportBtn{nullptr};
    QPushButton *m_clearBtn{nullptr};
};
