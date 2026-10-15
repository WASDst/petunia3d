#pragma once
#include <QJsonObject>

#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QDoubleSpinBox>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMenu>
#include <QPushButton>
#include <QSpinBox>
#include <QVBoxLayout>
#include <QWidget>

class PetuniaPrimitiveCard : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaPrimitiveCard(QWidget *parent = nullptr);
    ~PetuniaPrimitiveCard() override = default;

    void setPrimitiveType(const QString &type);

  signals:
    void parameterChanged(float size, int segments, int rings);

  private:
    QLabel *m_titleLabel{nullptr};
    QDoubleSpinBox *m_sizeSpin{nullptr};
    QSpinBox *m_segmentsSpin{nullptr};
    QSpinBox *m_ringsSpin{nullptr};
    QCheckBox *m_capEndsCheck{nullptr};
};

class PetuniaLoopCutCard : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaLoopCutCard(QWidget *parent = nullptr);
    ~PetuniaLoopCutCard() override = default;

  signals:
    void loopCutChanged(int cuts, float slide, float smoothness);

  private:
    QSpinBox *m_cutsSpin{nullptr};
    QDoubleSpinBox *m_slideSpin{nullptr};
};

class PetuniaToolOptionsBar : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaToolOptionsBar(QWidget *parent = nullptr);
    ~PetuniaToolOptionsBar() override = default;

    void sync(const QJsonObject &state);
    bool isSnappingEnabled() const;
    QString snapTarget() const;
    bool isProportionalEditing() const;
    double proportionalRadius() const;
    QString pivotPoint() const;

  signals:
    void selectionDomainChanged(int domain);
    void snapToggled(bool enabled);
    void snapTargetChanged(const QString &target);
    void proportionalToggled(bool enabled);
    void proportionalRadiusChanged(double radius);
    void pivotChanged(const QString &pivot);
    void shadingModeChanged(const QString &mode);
    void xrayToggled(bool enabled);
    void matcapToggled(bool enabled);
    void ambientOcclusionToggled(bool enabled);
    void faceOrientationToggled(bool enabled);
    void uvCheckerToggled(bool enabled);
    void cameraPresetRequested(const QString &preset);
    void planarNavigationToggled();
    void frameSelectionRequested();
    void actionTriggered(const QString &action);

  private slots:
    void openOriginMenu();

  private:
    // Selection Domain Group
    QWidget *m_domainGroup{nullptr};
    QPushButton *m_domainObjBtn{nullptr};
    QPushButton *m_domainVertBtn{nullptr};
    QPushButton *m_domainEdgeBtn{nullptr};
    QPushButton *m_domainFaceBtn{nullptr};
    QFrame *m_sepDomain{nullptr};

    // Transform Helpers (Snapping, Proportional, Pivot)
    QWidget *m_transformGroup{nullptr};
    QPushButton *m_snapBtn{nullptr};
    QComboBox *m_snapTargetCombo{nullptr};
    QPushButton *m_propBtn{nullptr};
    QDoubleSpinBox *m_propRadiusSpin{nullptr};
    QComboBox *m_pivotCombo{nullptr};
    QPushButton *m_originBtn{nullptr};
    QFrame *m_sepTransform{nullptr};

    // Camera & View
    QPushButton *m_planarBtn{nullptr};
    QWidget *m_cameraGroup{nullptr};
    QPushButton *m_perspBtn{nullptr};
    QPushButton *m_orthoBtn{nullptr};
    QPushButton *m_iso21Btn{nullptr};
    QPushButton *m_projBtn{nullptr};
    QPushButton *m_isoBtn{nullptr};
    QPushButton *m_frameBtn{nullptr};
    QFrame *m_sepCamera{nullptr};

    // Shading & Overlays
    QComboBox *m_shadingCombo{nullptr};
    QPushButton *m_xrayBtn{nullptr};
    QPushButton *m_matcapBtn{nullptr};
    QPushButton *m_aoBtn{nullptr};
    QPushButton *m_faceOrientationBtn{nullptr};
    QPushButton *m_uvCheckerBtn{nullptr};
};

using PetuniaFloatingViewBar = PetuniaToolOptionsBar;

class PetuniaFloatingContextBar : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaFloatingContextBar(QWidget *parent = nullptr);
    ~PetuniaFloatingContextBar() override = default;

    void sync(const QJsonObject &state);

  signals:
    void actionTriggered(const QString &action);
    void commandExecuted(const QString &commandId);
    void toolChanged(const QString &toolId);
    void brushSizeDelta(float delta);
    void brushOpacityDelta(float delta);
    void symmetryToggled(const QString &axis, bool enabled);
    void colorPickerRequested();
    void animateActionTriggered(const QString &action);

  private:
    QWidget *m_modelWidget{nullptr};
    QWidget *m_domainObjectWidget{nullptr};
    QWidget *m_domainVertexWidget{nullptr};
    QWidget *m_domainEdgeWidget{nullptr};
    QWidget *m_domainFaceWidget{nullptr};
    QWidget *m_drawWidget{nullptr};
    QWidget *m_paintWidget{nullptr};
    QWidget *m_uvWidget{nullptr};
    QWidget *m_animateWidget{nullptr};

    // Paint section widgets
    QPushButton *m_paintIsolateBtn{nullptr};
    QPushButton *m_colorSwatchBtn{nullptr};
    QLabel *m_brushSizeLabel{nullptr};
    QLabel *m_brushOpacityLabel{nullptr};
    QPushButton *m_symXBtn{nullptr};
    QPushButton *m_symYBtn{nullptr};
    QPushButton *m_symZBtn{nullptr};

    // Animate section widgets
    QPushButton *m_playBtn{nullptr};
    QLabel *m_animLabel{nullptr};

    float m_brushSize{16.0f};
    float m_brushOpacity{1.0f};
    QString m_activeWorkspace;
    QString m_modelingMode;
};

class PetuniaMicroInspector : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaMicroInspector(QWidget *parent = nullptr);
    ~PetuniaMicroInspector() override = default;

    void setCoordinates(double x, double y, double z, double rx, double ry,
                        double rz);
    void setMeasureData(double distance, double dx, double dy, double dz,
                        double angleDeg);

  signals:
    void toolSelected(const QString &toolId);
    void modelingActionTriggered(const QString &actionId);
    void closed();

  private:
    QLabel *m_coordsLabel{nullptr};
    QLabel *m_measureLabel{nullptr};
    bool m_measureMode{false};
};

class PetuniaOperationHud : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaOperationHud(QWidget *parent = nullptr);
    ~PetuniaOperationHud() override = default;

    void showFeedback(const QString &opName, const QString &details);
    void clearFeedback();

  private:
    QLabel *m_feedbackLabel{nullptr};
};

class PetuniaAddMenu : public QMenu {
    Q_OBJECT

  public:
    explicit PetuniaAddMenu(QWidget *parent = nullptr);
    ~PetuniaAddMenu() override = default;

  signals:
    void primitiveSelected(const QString &primitiveId);
};

class PetuniaColorWheel : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaColorWheel(QWidget *parent = nullptr);
    ~PetuniaColorWheel() override = default;

    QColor color() const;
    void setColor(const QColor &color);

  signals:
    void colorChanged(const QColor &color);

  protected:
    void paintEvent(QPaintEvent *) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;

  private:
    void handleInteraction(const QPointF &pos);
    float m_hue{30.0f};
    float m_saturation{0.8f};
    float m_value{0.9f};
};

class PetuniaColorPickerPopover : public QDialog {
    Q_OBJECT

  public:
    explicit PetuniaColorPickerPopover(QWidget *parent = nullptr);
    ~PetuniaColorPickerPopover() override = default;

    void setColor(const QColor &color);
    QColor color() const;
    void setTitle(const QString &title);

  signals:
    void colorChanged(const QColor &color);

  private:
    QLabel *m_titleLabel{nullptr};
    PetuniaColorWheel *m_wheel{nullptr};
    QDoubleSpinBox *m_hueSpin{nullptr};
    QDoubleSpinBox *m_satSpin{nullptr};
    QDoubleSpinBox *m_valSpin{nullptr};
    QFrame *m_swatchPreview{nullptr};
    QLineEdit *m_hexEdit{nullptr};
    QLabel *m_rgbLabel{nullptr};
    bool m_updating{false};

    void updateWidgetsFromColor(const QColor &col);
};
