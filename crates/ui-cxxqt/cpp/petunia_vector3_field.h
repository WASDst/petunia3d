#pragma once

#include "petunia_number_field.h"
#include <QWidget>
#include <QBoxLayout>
#include <QResizeEvent>

/// Responsive, semantic Vector3 (X, Y, Z) control for Petunia3D UI-001.
/// Displays axis color tags + letter badges, handles linked/unlinked edits,
/// supports mixed values, and reflows horizontally/vertically without clipping.
class PetuniaVector3Field : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaVector3Field(QWidget *parent = nullptr);
    ~PetuniaVector3Field() override = default;

    void setValues(double x, double y, double z);
    void setValue(int axis, double val);
    double value(int axis) const;

    void setMixed(int axis, bool mixed);
    bool isMixed(int axis) const;

    void setSuffix(const QString &suffix);
    void setRange(double min, double max);
    void setSingleStep(double step);
    void setMaxDecimals(int dec);

    PetuniaNumberField *field(int axis) const;

    /// Cadeado por eixo dentro de cada campo (B7).
    void setLockable(bool lockable);
    void setLocked(int axis, bool locked);
    bool isLocked(int axis) const;

    /// Nome acessível do vetor; cada eixo recebe "<label> X/Y/Z".
    void setLabel(const QString &label);

    QSize minimumSizeHint() const override;

signals:
    void axisValueChanged(int axis, double val);
    void editStarted(int axis);
    void previewRequested(int axis, double val);
    void commitRequested(int axis, double val, const QString &text = QString());
    void editCanceled(int axis);
    void lockToggled(int axis, bool locked);

protected:
    void resizeEvent(QResizeEvent *event) override;
    void showEvent(QShowEvent *event) override;

private:
    void applyThemeColors();
    void updateLayoutDirection(int width);

    QBoxLayout *m_layout{nullptr};
    PetuniaNumberField *m_fields[3]{nullptr, nullptr, nullptr};
    bool m_stacked{false};
};
