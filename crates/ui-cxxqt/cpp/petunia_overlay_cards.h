#pragma once

#include <QWidget>
#include <QFrame>
#include <QSpinBox>
#include <QDoubleSpinBox>
#include <QCheckBox>
#include <QLabel>
#include <QPushButton>
#include <QComboBox>
#include <QMenu>
#include <QHBoxLayout>
#include <QVBoxLayout>

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
    QDoubleSpinBox *m_smoothnessSpin{nullptr};
};

class PetuniaToolOptionsBar : public QFrame {
    Q_OBJECT

public:
    explicit PetuniaToolOptionsBar(QWidget *parent = nullptr);
    ~PetuniaToolOptionsBar() override = default;

    bool isSnappingEnabled() const;
    QString snapTarget() const;
    bool isProportionalEditing() const;
    double proportionalRadius() const;
    QString pivotPoint() const;

signals:
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
    void actionTriggered(const QString &action);

private slots:
    void openOriginMenu();

private:
    QPushButton *m_snapBtn{nullptr};
    QComboBox *m_snapTargetCombo{nullptr};
    QPushButton *m_propBtn{nullptr};
    QDoubleSpinBox *m_propRadiusSpin{nullptr};
    QComboBox *m_pivotCombo{nullptr};
    QPushButton *m_originBtn{nullptr};

    QComboBox *m_shadingCombo{nullptr};
    QPushButton *m_xrayBtn{nullptr};
    QPushButton *m_matcapBtn{nullptr};
    QPushButton *m_aoBtn{nullptr};
    QPushButton *m_faceOrientationBtn{nullptr};
    QPushButton *m_uvCheckerBtn{nullptr};
};

class PetuniaMicroInspector : public QFrame {
    Q_OBJECT

public:
    explicit PetuniaMicroInspector(QWidget *parent = nullptr);
    ~PetuniaMicroInspector() override = default;

    void setCoordinates(double x, double y, double z, double rx, double ry, double rz);
    void setMeasureData(double distance, double dx, double dy, double dz, double angleDeg);

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
