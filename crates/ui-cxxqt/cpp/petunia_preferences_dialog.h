#pragma once

#include <QDialog>
#include <QTabWidget>
#include <QComboBox>
#include <QSpinBox>
#include <QCheckBox>
#include <QTableWidget>
#include <QPushButton>
#include "petunia_number_field.h"

class PetuniaPreferencesDialog : public QDialog {
    Q_OBJECT

public:
    explicit PetuniaPreferencesDialog(QWidget *parent = nullptr);
    ~PetuniaPreferencesDialog() override = default;

private:
    void setupUi();
    void setupGeneralTab();
    void setupViewportTab();
    void setupKeymapTab();

    QTabWidget *m_tabs{nullptr};

    // Geral
    QComboBox *m_langCombo{nullptr};
    QComboBox *m_themeCombo{nullptr};
    QComboBox *m_densityCombo{nullptr};
    QSpinBox *m_autosaveMinutes{nullptr};
    QCheckBox *m_recoveryCheck{nullptr};

    // Viewport
    QComboBox *m_msaaCombo{nullptr};
    QSpinBox *m_gridDivisions{nullptr};
    PetuniaNumberField *m_cameraSpeed{nullptr};
    QCheckBox *m_invertOrbitY{nullptr};
    QCheckBox *m_showFps{nullptr};

    // Atalhos
    QTableWidget *m_keymapTable{nullptr};
};
