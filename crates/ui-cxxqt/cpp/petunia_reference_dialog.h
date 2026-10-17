#pragma once

#include <QDialog>
#include <QListWidget>
#include <QPushButton>
#include <QSlider>
#include <QComboBox>
#include <QLabel>
#include "petunia_number_field.h"

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
    PetuniaNumberField *m_scaleSpin{nullptr};
    PetuniaNumberField *m_offsetX{nullptr};
    PetuniaNumberField *m_offsetY{nullptr};
};
