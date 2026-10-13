#pragma once

#include <QDialog>
#include <QListWidget>
#include <QPushButton>
#include <QSlider>
#include <QComboBox>
#include <QDoubleSpinBox>
#include <QLabel>

class PetuniaReferenceDialog : public QDialog {
    Q_OBJECT

public:
    explicit PetuniaReferenceDialog(QWidget *parent = nullptr);
    ~PetuniaReferenceDialog() override = default;

private slots:
    void addReferenceImage();
    void removeReferenceImage();
    void updateImageOpacity(int val);

private:
    void setupUi();

    QListWidget *m_imageList{nullptr};
    QPushButton *m_addBtn{nullptr};
    QPushButton *m_removeBtn{nullptr};

    QComboBox *m_planeCombo{nullptr};
    QSlider *m_opacitySlider{nullptr};
    QLabel *m_opacityLabel{nullptr};
    QDoubleSpinBox *m_scaleSpin{nullptr};
    QDoubleSpinBox *m_offsetX{nullptr};
    QDoubleSpinBox *m_offsetY{nullptr};
};
