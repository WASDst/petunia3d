#include "petunia_component_gallery.h"
#include <QHBoxLayout>
#include <QPushButton>
#include <QCheckBox>
#include <QFrame>
#include <QApplication>

PetuniaComponentGallery::PetuniaComponentGallery(QWidget *parent)
    : QWidget(parent) {
    setupUi();
}

void PetuniaComponentGallery::setupUi() {
    setWindowTitle(QStringLiteral("Petunia3D — Component Gallery (UI-001)"));
    resize(700, 850);

    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(12, 12, 12, 12);
    rootLayout->setSpacing(8);

    // Controls header: Theme & Density selectors
    auto *controlsCard = new QFrame(this);
    controlsCard->setFrameShape(QFrame::StyledPanel);
    auto *controlsLayout = new QHBoxLayout(controlsCard);
    controlsLayout->setContentsMargins(8, 6, 8, 6);
    controlsLayout->setSpacing(12);

    controlsLayout->addWidget(new QLabel(QStringLiteral("Theme:"), this));
    m_themeCombo = new QComboBox(this);
    m_themeCombo->addItem(QStringLiteral("petunia-dark"), QStringLiteral("petunia-dark"));
    m_themeCombo->addItem(QStringLiteral("petunia-light"), QStringLiteral("petunia-light"));
    m_themeCombo->addItem(QStringLiteral("petunia-high-contrast"), QStringLiteral("petunia-high-contrast"));
    connect(m_themeCombo, &QComboBox::currentTextChanged, this, &PetuniaComponentGallery::applyTheme);
    controlsLayout->addWidget(m_themeCombo);

    controlsLayout->addWidget(new QLabel(QStringLiteral("Density:"), this));
    m_densityCombo = new QComboBox(this);
    m_densityCombo->addItem(QStringLiteral("Standard"), 1);
    m_densityCombo->addItem(QStringLiteral("Compact"), 0);
    m_densityCombo->addItem(QStringLiteral("Comfortable"), 2);
    connect(m_densityCombo, &QComboBox::currentIndexChanged, this, [this](int idx) {
        applyDensity(static_cast<PetuniaTheme::Density>(m_densityCombo->itemData(idx).toInt()));
    });
    controlsLayout->addWidget(m_densityCombo);

    controlsLayout->addStretch(1);
    rootLayout->addWidget(controlsCard);

    // Scrollable showcase area
    auto *scrollArea = new QScrollArea(this);
    scrollArea->setWidgetResizable(true);
    scrollArea->setFrameShape(QFrame::NoFrame);

    m_contentContainer = new QWidget(scrollArea);
    auto *contentLayout = new QVBoxLayout(m_contentContainer);
    contentLayout->setContentsMargins(4, 4, 4, 4);
    contentLayout->setSpacing(12);

    setupTokenSwatches(contentLayout);
    setupNumberFields(contentLayout);
    setupVector3Fields(contentLayout);
    setupPropertyGroups(contentLayout);

    contentLayout->addStretch(1);
    scrollArea->setWidget(m_contentContainer);
    rootLayout->addWidget(scrollArea, 1);
}

void PetuniaComponentGallery::setupTokenSwatches(QVBoxLayout *layout) {
    auto *group = new PropertyGroup(QStringLiteral("1. Semantic Theme Tokens & Swatches"), m_contentContainer);
    auto *rowWidget = new QWidget(group);
    auto *rowLayout = new QHBoxLayout(rowWidget);
    rowLayout->setContentsMargins(0, 4, 0, 4);
    rowLayout->setSpacing(6);

    auto addSwatch = [&](const QString &label, const QColor &color) {
        auto *swatch = new QFrame(rowWidget);
        swatch->setFixedSize(50, 40);
        swatch->setStyleSheet(QStringLiteral("background-color: %1; border: 1px solid #555; border-radius: 4px;").arg(color.name()));
        auto *vbox = new QVBoxLayout();
        vbox->addWidget(swatch);
        auto *lbl = new QLabel(label, rowWidget);
        lbl->setAlignment(Qt::AlignCenter);
        QFont f = lbl->font(); f.setPointSize(8); lbl->setFont(f);
        vbox->addWidget(lbl);
        rowLayout->addLayout(vbox);
    };

    const auto &t = PetuniaTheme::instance();
    addSwatch(QStringLiteral("Canvas"), t.bgCanvas());
    addSwatch(QStringLiteral("Panel"), t.bgPanel());
    addSwatch(QStringLiteral("Surface"), t.bgSurface());
    addSwatch(QStringLiteral("Accent"), t.accentPrimary());
    addSwatch(QStringLiteral("Selection"), t.accentSelection());
    addSwatch(QStringLiteral("Axis X"), t.axisColor(0));
    addSwatch(QStringLiteral("Axis Y"), t.axisColor(1));
    addSwatch(QStringLiteral("Axis Z"), t.axisColor(2));
    rowLayout->addStretch(1);

    group->addWidget(rowWidget);
    layout->addWidget(group);
}

void PetuniaComponentGallery::setupNumberFields(QVBoxLayout *layout) {
    auto *group = new PropertyGroup(QStringLiteral("2. PetuniaNumberField Primitives"), m_contentContainer);

    // Distance field
    auto *dist = new PetuniaNumberField(QStringLiteral("D"), group);
    dist->setValue(2.5);
    dist->setSuffix(QStringLiteral("m"));
    dist->setRange(-1000.0, 1000.0);
    dist->setSingleStep(0.1);
    group->addRow(QStringLiteral("Distance:"), dist);
    m_testNumberField = dist;

    // Angle field
    auto *angle = new PetuniaNumberField(QStringLiteral("∠"), group);
    angle->setValue(45.0);
    angle->setSuffix(QStringLiteral("°"));
    angle->setRange(-360.0, 360.0);
    angle->setSingleStep(1.0);
    group->addRow(QStringLiteral("Angle:"), angle);

    // Percentage field
    auto *pct = new PetuniaNumberField(QStringLiteral("%"), group);
    pct->setValue(75.0);
    pct->setSuffix(QStringLiteral("%"));
    pct->setRange(0.0, 100.0);
    group->addRow(QStringLiteral("Opacity:"), pct);

    // Mixed indeterminate state
    auto *mixed = new PetuniaNumberField(QStringLiteral("M"), group);
    mixed->setMixed(true);
    group->addRow(QStringLiteral("Mixed Selection:"), mixed);

    layout->addWidget(group);
}

void PetuniaComponentGallery::setupVector3Fields(QVBoxLayout *layout) {
    auto *group = new PropertyGroup(QStringLiteral("3. PetuniaVector3Field (Transform Slice)"), m_contentContainer);

    // Position
    auto *pos = new PetuniaVector3Field(group);
    pos->setValues(1.0, 2.0, 0.0);
    pos->setSuffix(QStringLiteral("m"));
    group->addRow(QStringLiteral("Position:"), pos);
    m_testVector3Field = pos;

    // Rotation
    auto *rot = new PetuniaVector3Field(group);
    rot->setValues(0.0, 45.0, 90.0);
    rot->setSuffix(QStringLiteral("°"));
    rot->setSingleStep(1.0);
    group->addRow(QStringLiteral("Rotation:"), rot);

    // Scale
    auto *scale = new PetuniaVector3Field(group);
    scale->setValues(1.0, 1.0, 1.0);
    scale->setSingleStep(0.1);
    group->addRow(QStringLiteral("Scale:"), scale);

    // Mixed Vector3
    auto *mixedV3 = new PetuniaVector3Field(group);
    mixedV3->setValues(0.0, 0.0, 0.0);
    mixedV3->setMixed(0, true);
    mixedV3->setMixed(2, true);
    group->addRow(QStringLiteral("Mixed XYZ:"), mixedV3);

    layout->addWidget(group);
}

void PetuniaComponentGallery::setupPropertyGroups(QVBoxLayout *layout) {
    auto *group = new PropertyGroup(QStringLiteral("4. Buttons, Toggles & Collapsibles"), m_contentContainer);

    auto *row = new QWidget(group);
    auto *hlayout = new QHBoxLayout(row);
    hlayout->setContentsMargins(0, 0, 0, 0);

    auto *primaryBtn = new QPushButton(QStringLiteral("Primary Action"), row);
    primaryBtn->setDefault(true);
    hlayout->addWidget(primaryBtn);

    auto *secondaryBtn = new QPushButton(QStringLiteral("Secondary"), row);
    hlayout->addWidget(secondaryBtn);

    auto *chk = new QCheckBox(QStringLiteral("Enable Snapping"), row);
    chk->setChecked(true);
    hlayout->addWidget(chk);
    hlayout->addStretch(1);

    group->addRow(QStringLiteral("Actions:"), row);
    layout->addWidget(group);
}

void PetuniaComponentGallery::applyTheme(const QString &themeId) {
    QJsonObject themeColors;
    if (themeId == QStringLiteral("petunia-light")) {
        themeColors.insert(QStringLiteral("bg_canvas"), QStringLiteral("#f5f5f5"));
        themeColors.insert(QStringLiteral("bg_panel"), QStringLiteral("#ebebeb"));
        themeColors.insert(QStringLiteral("bg_surface"), QStringLiteral("#e0e0e0"));
        themeColors.insert(QStringLiteral("text_primary"), QStringLiteral("#1a1a1a"));
        themeColors.insert(QStringLiteral("accent_blue"), QStringLiteral("#7a38ff"));
        themeColors.insert(QStringLiteral("accent_orange"), QStringLiteral("#e05500"));
    } else if (themeId == QStringLiteral("petunia-high-contrast")) {
        themeColors.insert(QStringLiteral("bg_canvas"), QStringLiteral("#000000"));
        themeColors.insert(QStringLiteral("bg_panel"), QStringLiteral("#000000"));
        themeColors.insert(QStringLiteral("bg_surface"), QStringLiteral("#1a1a1a"));
        themeColors.insert(QStringLiteral("text_primary"), QStringLiteral("#ffffff"));
        themeColors.insert(QStringLiteral("accent_blue"), QStringLiteral("#ffff00"));
        themeColors.insert(QStringLiteral("accent_orange"), QStringLiteral("#ff6600"));
    } else {
        // Dark default
        themeColors.insert(QStringLiteral("bg_canvas"), QStringLiteral("#121212"));
        themeColors.insert(QStringLiteral("bg_panel"), QStringLiteral("#202020"));
        themeColors.insert(QStringLiteral("bg_surface"), QStringLiteral("#292929"));
        themeColors.insert(QStringLiteral("text_primary"), QStringLiteral("#eeeeee"));
        themeColors.insert(QStringLiteral("accent_blue"), QStringLiteral("#b58cff"));
        themeColors.insert(QStringLiteral("accent_orange"), QStringLiteral("#e96a00"));
    }
    PetuniaTheme::instance().updateFromSnapshot(themeColors, themeId);
}

void PetuniaComponentGallery::applyDensity(PetuniaTheme::Density density) {
    PetuniaTheme::instance().setDensity(density);
}

bool PetuniaComponentGallery::runSelfTest() {
    if (!m_testNumberField || !m_testVector3Field) {
        return false;
    }

    // 1. Validate number field precision and values
    m_testNumberField->setValue(42.5);
    if (std::abs(m_testNumberField->value() - 42.5) > 1e-6) {
        return false;
    }
    m_testNumberField->setMixed(true);
    if (!m_testNumberField->isMixed()) {
        return false;
    }
    m_testNumberField->setValue(10.0);
    if (m_testNumberField->isMixed()) {
        return false; // setValue must clear mixed
    }

    // 2. Validate Vector3 field values and mixed states
    m_testVector3Field->setValues(10.0, 20.0, 30.0);
    if (std::abs(m_testVector3Field->value(0) - 10.0) > 1e-6 ||
        std::abs(m_testVector3Field->value(1) - 20.0) > 1e-6 ||
        std::abs(m_testVector3Field->value(2) - 30.0) > 1e-6) {
        return false;
    }
    m_testVector3Field->setMixed(1, true);
    if (!m_testVector3Field->isMixed(1) || m_testVector3Field->isMixed(0)) {
        return false;
    }

    // 3. Theme switching preserves values
    applyTheme(QStringLiteral("petunia-light"));
    if (std::abs(m_testNumberField->value() - 10.0) > 1e-6) {
        return false;
    }
    applyTheme(QStringLiteral("petunia-dark"));

    return true;
}

extern "C" bool petunia_component_gallery_run_test() {
    PetuniaComponentGallery gallery;
    return gallery.runSelfTest();
}
