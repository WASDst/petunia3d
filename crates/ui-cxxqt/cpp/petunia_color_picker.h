#pragma once

#include <QColor>
#include <QDialog>
#include <QFrame>
#include <QLabel>
#include <QLineEdit>
#include <QWidget>

#include "petunia_number_field.h"

/// Roda de cor (anel de matiz + quadrado saturação/valor) e o popover que a
/// hospeda, com campos numéricos HSV, hex e cores rápidas. O cromo segue os
/// tokens de `PetuniaTheme`; as amostras são dados (cores), não tema.
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
    PetuniaNumberField *m_hueSpin{nullptr};
    PetuniaNumberField *m_satSpin{nullptr};
    PetuniaNumberField *m_valSpin{nullptr};
    QFrame *m_swatchPreview{nullptr};
    QLineEdit *m_hexEdit{nullptr};
    QLabel *m_rgbLabel{nullptr};
    bool m_updating{false};

    void updateWidgetsFromColor(const QColor &col);
};
