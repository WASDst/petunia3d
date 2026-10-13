#pragma once

#include <QWidget>
#include <QPushButton>
#include <QDoubleSpinBox>
#include <QComboBox>
#include <QCheckBox>
#include <QLabel>

class PetuniaUvCanvasWidget : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaUvCanvasWidget(QWidget *parent = nullptr);
    ~PetuniaUvCanvasWidget() override = default;

    void refresh();

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    float m_zoom{1.0f};
    QPointF m_panOffset{0.0f, 0.0f};
    QPoint m_lastMousePos;
    bool m_isPanning{false};
};

class PetuniaUvPanel : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaUvPanel(QWidget *parent = nullptr);
    ~PetuniaUvPanel() override = default;

    void refreshDiagnostics();

signals:
    void unwrapRequested(const QString &method);
    void packIslandsRequested(float margin);
    void markSeamRequested();
    void clearSeamRequested();

private:
    void setupUi();

    PetuniaUvCanvasWidget *m_uvCanvas{nullptr};
    QPushButton *m_unwrapSmartBtn{nullptr};
    QPushButton *m_unwrapCubeBtn{nullptr};
    QPushButton *m_markSeamBtn{nullptr};
    QPushButton *m_clearSeamBtn{nullptr};
    QPushButton *m_packBtn{nullptr};

    QDoubleSpinBox *m_marginSpin{nullptr};
    QComboBox *m_resolutionCombo{nullptr};
    QCheckBox *m_showStretchCheck{nullptr};

    QLabel *m_diagIslandsLabel{nullptr};
    QLabel *m_diagOverlapLabel{nullptr};
    QLabel *m_diagStretchLabel{nullptr};
};
