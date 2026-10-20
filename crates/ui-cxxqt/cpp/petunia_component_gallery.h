#pragma once

#include <QWidget>
#include <QScrollArea>
#include <QVBoxLayout>
#include <QComboBox>
#include <QLabel>
#include "petunia_number_field.h"
#include "petunia_vector3_field.h"
#include "petunia_property_group.h"
#include "petunia_theme.h"

/// Native Qt Component Gallery for Petunia3D UI-001 design system.
/// Interactive Storybook-like showcase of production primitives:
/// Tokens, NumberFields, Vector3, PropertyGroups, Buttons, Toggles.
class PetuniaComponentGallery : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaComponentGallery(QWidget *parent = nullptr);
    ~PetuniaComponentGallery() override = default;

    /// Runs automated self-test validating component contracts, values, and states.
    bool runSelfTest();

private:
    void setupUi();
    void setupTokenSwatches(QVBoxLayout *layout);
    void setupNumberFields(QVBoxLayout *layout);
    void setupVector3Fields(QVBoxLayout *layout);
    void setupPropertyGroups(QVBoxLayout *layout);
    void applyTheme(const QString &themeId);
    void applyDensity(PetuniaTheme::Density density);

    QComboBox *m_themeCombo{nullptr};
    QComboBox *m_densityCombo{nullptr};
    QWidget *m_contentContainer{nullptr};
    PetuniaNumberField *m_testNumberField{nullptr};
    PetuniaVector3Field *m_testVector3Field{nullptr};
};
