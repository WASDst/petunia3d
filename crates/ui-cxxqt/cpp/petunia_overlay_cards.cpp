#include "petunia_overlay_cards.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGraphicsDropShadowEffect>

// --- Primitive Card ---
PetuniaPrimitiveCard::PetuniaPrimitiveCard(QWidget *parent)
    : QFrame(parent)
{
    setFrameShape(QFrame::StyledPanel);
    setStyleSheet(QStringLiteral(
        "PetuniaPrimitiveCard { background-color: rgba(30, 30, 30, 230); border: 1px solid #555555; border-radius: 6px; }"
        "QLabel { color: #eeeeee; font-weight: bold; }"
        "QDoubleSpinBox, QSpinBox { background-color: #252525; color: #eeeeee; border: 1px solid #444444; border-radius: 3px; padding: 2px; }"
        "QCheckBox { color: #b5b5b5; }"
    ));

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(10, 8, 10, 8);
    layout->setSpacing(6);

    auto *headerRow = new QHBoxLayout();
    m_titleLabel = new QLabel(QStringLiteral("Adicionar Primitiva"), this);
    headerRow->addWidget(m_titleLabel);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), this);
    closeBtn->setFixedSize(16, 16);
    closeBtn->setStyleSheet(QStringLiteral("background: transparent; color: #888888; border: none; font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, &QWidget::hide);
    headerRow->addWidget(closeBtn);
    layout->addLayout(headerRow);

    auto *form = new QFormLayout();
    m_sizeSpin = new QDoubleSpinBox(this);
    m_sizeSpin->setRange(0.01, 100.0);
    m_sizeSpin->setValue(2.0);
    m_sizeSpin->setSingleStep(0.1);
    m_sizeSpin->setSuffix(QStringLiteral(" m"));
    form->addRow(QStringLiteral("Tamanho / Raio:"), m_sizeSpin);

    m_segmentsSpin = new QSpinBox(this);
    m_segmentsSpin->setRange(3, 128);
    m_segmentsSpin->setValue(32);
    form->addRow(QStringLiteral("Segmentos:"), m_segmentsSpin);

    m_ringsSpin = new QSpinBox(this);
    m_ringsSpin->setRange(2, 64);
    m_ringsSpin->setValue(16);
    form->addRow(QStringLiteral("Anéis:"), m_ringsSpin);

    m_capEndsCheck = new QCheckBox(QStringLiteral("Fechar Extremidades (Caps)"), this);
    m_capEndsCheck->setChecked(true);
    form->addRow(m_capEndsCheck);

    layout->addLayout(form);

    auto onParamChanged = [this]() {
        emit parameterChanged(
            static_cast<float>(m_sizeSpin->value()),
            m_segmentsSpin->value(),
            m_ringsSpin->value()
        );
    };

    connect(m_sizeSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, onParamChanged);
    connect(m_segmentsSpin, QOverload<int>::of(&QSpinBox::valueChanged), this, onParamChanged);
    connect(m_ringsSpin, QOverload<int>::of(&QSpinBox::valueChanged), this, onParamChanged);

    adjustSize();
    hide();
}

void PetuniaPrimitiveCard::setPrimitiveType(const QString &type) {
    m_titleLabel->setText(QStringLiteral("Parâmetros: %1").arg(type));
    show();
}

// --- Loop Cut Card ---
PetuniaLoopCutCard::PetuniaLoopCutCard(QWidget *parent)
    : QFrame(parent)
{
    setFrameShape(QFrame::StyledPanel);
    setStyleSheet(QStringLiteral(
        "PetuniaLoopCutCard { background-color: rgba(30, 30, 30, 230); border: 1px solid #555555; border-radius: 6px; }"
        "QLabel { color: #eeeeee; font-weight: bold; }"
        "QDoubleSpinBox, QSpinBox { background-color: #252525; color: #eeeeee; border: 1px solid #444444; border-radius: 3px; padding: 2px; }"
    ));

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(10, 8, 10, 8);
    layout->setSpacing(6);

    auto *headerRow = new QHBoxLayout();
    auto *title = new QLabel(QStringLiteral("Corte em Loop (Loop Cut)"), this);
    headerRow->addWidget(title);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), this);
    closeBtn->setFixedSize(16, 16);
    closeBtn->setStyleSheet(QStringLiteral("background: transparent; color: #888888; border: none; font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, &QWidget::hide);
    headerRow->addWidget(closeBtn);
    layout->addLayout(headerRow);

    auto *form = new QFormLayout();
    m_cutsSpin = new QSpinBox(this);
    m_cutsSpin->setRange(1, 64);
    m_cutsSpin->setValue(1);
    form->addRow(QStringLiteral("Número de Cortes:"), m_cutsSpin);

    m_slideSpin = new QDoubleSpinBox(this);
    m_slideSpin->setRange(-1.0, 1.0);
    m_slideSpin->setValue(0.0);
    m_slideSpin->setSingleStep(0.05);
    form->addRow(QStringLiteral("Deslizamento (Slide):"), m_slideSpin);

    m_smoothnessSpin = new QDoubleSpinBox(this);
    m_smoothnessSpin->setRange(0.0, 1.0);
    m_smoothnessSpin->setValue(0.0);
    m_smoothnessSpin->setSingleStep(0.05);
    form->addRow(QStringLiteral("Suavização:"), m_smoothnessSpin);

    layout->addLayout(form);

    auto onParamChanged = [this]() {
        emit loopCutChanged(
            m_cutsSpin->value(),
            static_cast<float>(m_slideSpin->value()),
            static_cast<float>(m_smoothnessSpin->value())
        );
    };

    connect(m_cutsSpin, QOverload<int>::of(&QSpinBox::valueChanged), this, onParamChanged);
    connect(m_slideSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, onParamChanged);
    connect(m_smoothnessSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, onParamChanged);

    adjustSize();
    hide();
}

// --- Tool Options Bar (Snapping, Proportional, Pivot, Shading & Overlays) ---
PetuniaToolOptionsBar::PetuniaToolOptionsBar(QWidget *parent)
    : QFrame(parent)
{
    setFixedHeight(36);
    setStyleSheet(QStringLiteral(
        "PetuniaToolOptionsBar { background-color: rgba(26, 26, 26, 230); border: 1px solid #383838; border-radius: 6px; }"
        "QPushButton { background-color: #252525; color: #cccccc; font-size: 11px; border: 1px solid #404040; border-radius: 4px; padding: 2px 8px; font-weight: bold; }"
        "QPushButton:hover { background-color: #353535; border-color: #555555; }"
        "QPushButton:checked { background-color: #b58cff; color: #121212; border-color: #b58cff; }"
        "QComboBox { background-color: #252525; color: #eeeeee; font-size: 11px; border: 1px solid #404040; border-radius: 4px; padding: 2px 6px; }"
        "QDoubleSpinBox { background-color: #252525; color: #eeeeee; font-size: 11px; border: 1px solid #404040; border-radius: 4px; padding: 2px 4px; }"
        "QLabel { color: #888888; font-size: 11px; }"
    ));

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(6, 4, 6, 4);
    layout->setSpacing(6);

    // 1. Snapping
    m_snapBtn = new QPushButton(QStringLiteral("🧲 Snap"), this);
    m_snapBtn->setCheckable(true);
    m_snapBtn->setToolTip(QStringLiteral("Ativar/Desativar Snap (Shift+Tab)"));
    connect(m_snapBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::snapToggled);
    layout->addWidget(m_snapBtn);

    m_snapTargetCombo = new QComboBox(this);
    m_snapTargetCombo->addItems({QStringLiteral("Grade"), QStringLiteral("Vértice"), QStringLiteral("Aresta"), QStringLiteral("Face")});
    m_snapTargetCombo->setToolTip(QStringLiteral("Alvo do Snap"));
    connect(m_snapTargetCombo, &QComboBox::currentTextChanged, this, &PetuniaToolOptionsBar::snapTargetChanged);
    layout->addWidget(m_snapTargetCombo);

    auto *sep1 = new QFrame(this);
    sep1->setFrameShape(QFrame::VLine);
    sep1->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    layout->addWidget(sep1);

    // 2. Proportional Editing
    m_propBtn = new QPushButton(QStringLiteral("◎ Prop"), this);
    m_propBtn->setCheckable(true);
    m_propBtn->setToolTip(QStringLiteral("Edição Proporcional (O)"));
    connect(m_propBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::proportionalToggled);
    layout->addWidget(m_propBtn);

    m_propRadiusSpin = new QDoubleSpinBox(this);
    m_propRadiusSpin->setRange(0.05, 50.0);
    m_propRadiusSpin->setValue(1.0);
    m_propRadiusSpin->setSingleStep(0.1);
    m_propRadiusSpin->setSuffix(QStringLiteral(" m"));
    m_propRadiusSpin->setToolTip(QStringLiteral("Raio de Influência"));
    connect(m_propRadiusSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, &PetuniaToolOptionsBar::proportionalRadiusChanged);
    layout->addWidget(m_propRadiusSpin);

    auto *sep2 = new QFrame(this);
    sep2->setFrameShape(QFrame::VLine);
    sep2->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    layout->addWidget(sep2);

    // 3. Pivot Point
    auto *pivotLabel = new QLabel(QStringLiteral("Pivô:"), this);
    layout->addWidget(pivotLabel);

    m_pivotCombo = new QComboBox(this);
    m_pivotCombo->addItems({
        QStringLiteral("Ponto Médio"),
        QStringLiteral("Cursor 3D"),
        QStringLiteral("Origens Individuais"),
        QStringLiteral("Caixa Delimitadora")
    });
    m_pivotCombo->setToolTip(QStringLiteral("Ponto de Pivô para Transformação"));
    connect(m_pivotCombo, &QComboBox::currentTextChanged, this, &PetuniaToolOptionsBar::pivotChanged);
    layout->addWidget(m_pivotCombo);

    m_originBtn = new QPushButton(QStringLiteral("Origem ▾"), this);
    m_originBtn->setToolTip(QStringLiteral("Ajustar Origem do Objeto"));
    connect(m_originBtn, &QPushButton::clicked, this, &PetuniaToolOptionsBar::openOriginMenu);
    layout->addWidget(m_originBtn);

    layout->addStretch();

    // 4. Shading & Overlays View Bar
    auto *shadingLabel = new QLabel(QStringLiteral("Shading:"), this);
    layout->addWidget(shadingLabel);

    m_shadingCombo = new QComboBox(this);
    m_shadingCombo->addItems({
        QStringLiteral("○ Aramado"),
        QStringLiteral("◐ Sólido"),
        QStringLiteral("● Material"),
        QStringLiteral("◉ Render")
    });
    m_shadingCombo->setCurrentIndex(1); // Solid default
    connect(m_shadingCombo, &QComboBox::currentTextChanged, this, [this](const QString &text) {
        if (text.contains(QStringLiteral("Aramado"))) emit shadingModeChanged(QStringLiteral("wireframe"));
        else if (text.contains(QStringLiteral("Sólido"))) emit shadingModeChanged(QStringLiteral("solid"));
        else if (text.contains(QStringLiteral("Material"))) emit shadingModeChanged(QStringLiteral("material"));
        else if (text.contains(QStringLiteral("Render"))) emit shadingModeChanged(QStringLiteral("rendered"));
    });
    layout->addWidget(m_shadingCombo);

    m_xrayBtn = new QPushButton(QStringLiteral("Raio-X"), this);
    m_xrayBtn->setCheckable(true);
    m_xrayBtn->setToolTip(QStringLiteral("Alternar Visão de Raio-X (Alt+Z)"));
    connect(m_xrayBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::xrayToggled);
    layout->addWidget(m_xrayBtn);

    m_matcapBtn = new QPushButton(QStringLiteral("MatCap"), this);
    m_matcapBtn->setCheckable(true);
    m_matcapBtn->setToolTip(QStringLiteral("Ativar Sombreamento MatCap"));
    connect(m_matcapBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::matcapToggled);
    layout->addWidget(m_matcapBtn);

    m_aoBtn = new QPushButton(QStringLiteral("AO"), this);
    m_aoBtn->setCheckable(true);
    m_aoBtn->setToolTip(QStringLiteral("Oclusão Ambiente (GTAO)"));
    connect(m_aoBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::ambientOcclusionToggled);
    layout->addWidget(m_aoBtn);

    m_faceOrientationBtn = new QPushButton(QStringLiteral("Orient. Faces"), this);
    m_faceOrientationBtn->setCheckable(true);
    m_faceOrientationBtn->setToolTip(QStringLiteral("Verificar Normais (Azul=Frente, Vermelho=Trás)"));
    connect(m_faceOrientationBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::faceOrientationToggled);
    layout->addWidget(m_faceOrientationBtn);

    m_uvCheckerBtn = new QPushButton(QStringLiteral("UV Checker"), this);
    m_uvCheckerBtn->setCheckable(true);
    m_uvCheckerBtn->setToolTip(QStringLiteral("Sobreposição de Xadrez UV"));
    connect(m_uvCheckerBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::uvCheckerToggled);
    layout->addWidget(m_uvCheckerBtn);
}

bool PetuniaToolOptionsBar::isSnappingEnabled() const { return m_snapBtn->isChecked(); }
QString PetuniaToolOptionsBar::snapTarget() const { return m_snapTargetCombo->currentText(); }
bool PetuniaToolOptionsBar::isProportionalEditing() const { return m_propBtn->isChecked(); }
double PetuniaToolOptionsBar::proportionalRadius() const { return m_propRadiusSpin->value(); }
QString PetuniaToolOptionsBar::pivotPoint() const { return m_pivotCombo->currentText(); }

void PetuniaToolOptionsBar::openOriginMenu() {
    QMenu menu(this);
    menu.setStyleSheet(QStringLiteral(
        "QMenu { background-color: #202020; border: 1px solid #444444; color: #eeeeee; font-size: 11px; padding: 4px; }"
        "QMenu::item { padding: 4px 12px; border-radius: 3px; }"
        "QMenu::item:selected { background-color: #b58cff; color: #121212; font-weight: bold; }"
    ));

    auto *actGeom = menu.addAction(QStringLiteral("Origem para Geometria"));
    auto *actBottom = menu.addAction(QStringLiteral("Origem para Base (Bottom)"));
    auto *actCursor = menu.addAction(QStringLiteral("Origem para Cursor 3D"));
    auto *actSelect = menu.addAction(QStringLiteral("Origem para Seleção"));
    auto *actCenter = menu.addAction(QStringLiteral("Geometria para Origem"));

    connect(actGeom, &QAction::triggered, this, [this]() { emit actionTriggered(QStringLiteral("object.origin_to_geometry")); });
    connect(actBottom, &QAction::triggered, this, [this]() { emit actionTriggered(QStringLiteral("object.origin_to_bottom")); });
    connect(actCursor, &QAction::triggered, this, [this]() { emit actionTriggered(QStringLiteral("object.origin_to_cursor")); });
    connect(actSelect, &QAction::triggered, this, [this]() { emit actionTriggered(QStringLiteral("object.origin_to_selection")); });
    connect(actCenter, &QAction::triggered, this, [this]() { emit actionTriggered(QStringLiteral("object.geometry_to_origin")); });

    menu.exec(m_originBtn->mapToGlobal(QPoint(0, m_originBtn->height())));
}

// --- Micro Inspector ---
PetuniaMicroInspector::PetuniaMicroInspector(QWidget *parent)
    : QFrame(parent)
{
    setFixedSize(260, 190);
    setStyleSheet(QStringLiteral(
        "PetuniaMicroInspector { background-color: rgba(28, 28, 28, 240); border: 1px solid #b58cff; border-radius: 8px; }"
        "QLabel { color: #dddddd; font-size: 11px; }"
        "QPushButton { background-color: #252525; color: #eeeeee; font-size: 11px; border: 1px solid #3e3e3e; border-radius: 4px; padding: 3px; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; }"
    ));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(20);
    shadow->setColor(QColor(0, 0, 0, 180));
    shadow->setOffset(0, 4);
    setGraphicsEffect(shadow);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(10, 8, 10, 8);
    layout->setSpacing(6);

    auto *header = new QHBoxLayout();
    auto *title = new QLabel(QStringLiteral("Micro Inspetor"), this);
    title->setStyleSheet(QStringLiteral("color: #b58cff; font-weight: bold; font-size: 12px;"));
    header->addWidget(title);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), this);
    closeBtn->setFixedSize(16, 16);
    closeBtn->setStyleSheet(QStringLiteral("background: transparent; color: #888888; border: none; font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, [this]() {
        hide();
        emit closed();
    });
    header->addWidget(closeBtn);
    layout->addLayout(header);

    // Quick Tool Buttons
    auto *toolsRow = new QHBoxLayout();
    toolsRow->setSpacing(4);

    const struct { const char *id; const char *label; } tools[] = {
        {"select", "Sel"}, {"move", "G"}, {"rotate", "R"}, {"scale", "S"}, {"measure", "M"}
    };
    for (const auto &t : tools) {
        auto *btn = new QPushButton(QString::fromUtf8(t.label), this);
        btn->setFixedHeight(24);
        connect(btn, &QPushButton::clicked, this, [this, id = QString::fromUtf8(t.id)]() {
            emit toolSelected(id);
        });
        toolsRow->addWidget(btn);
    }
    layout->addLayout(toolsRow);

    auto *sep = new QFrame(this);
    sep->setFrameShape(QFrame::HLine);
    sep->setStyleSheet(QStringLiteral("background-color: #383838; max-height: 1px;"));
    layout->addWidget(sep);

    // Readout Coordinates
    m_coordsLabel = new QLabel(QStringLiteral("Pos: 0.00, 0.00, 0.00\nRot: 0°, 0°, 0°"), this);
    m_coordsLabel->setStyleSheet(QStringLiteral("font-family: monospace; color: #b58cff;"));
    layout->addWidget(m_coordsLabel);

    m_measureLabel = new QLabel(QStringLiteral(""), this);
    m_measureLabel->setStyleSheet(QStringLiteral("font-family: monospace; color: #e96a00; font-weight: bold;"));
    m_measureLabel->setVisible(false);
    layout->addWidget(m_measureLabel);

    // Direct Modeling Actions
    auto *actionsRow = new QHBoxLayout();
    actionsRow->setSpacing(4);
    const struct { const char *id; const char *label; } acts[] = {
        {"model.extrude", "Extrude"}, {"model.inset", "Inset"}, {"model.bevel", "Bevel"}, {"model.loop_cut", "Loop"}
    };
    for (const auto &a : acts) {
        auto *btn = new QPushButton(QString::fromUtf8(a.label), this);
        btn->setFixedHeight(22);
        connect(btn, &QPushButton::clicked, this, [this, id = QString::fromUtf8(a.id)]() {
            emit modelingActionTriggered(id);
        });
        actionsRow->addWidget(btn);
    }
    layout->addLayout(actionsRow);

    hide();
}

void PetuniaMicroInspector::setCoordinates(double x, double y, double z, double rx, double ry, double rz) {
    m_coordsLabel->setText(QStringLiteral("Pos: %1, %2, %3\nRot: %4°, %5°, %6°")
        .arg(x, 0, 'f', 2)
        .arg(y, 0, 'f', 2)
        .arg(z, 0, 'f', 2)
        .arg(rx, 0, 'f', 1)
        .arg(ry, 0, 'f', 1)
        .arg(rz, 0, 'f', 1));
}

void PetuniaMicroInspector::setMeasureData(double distance, double dx, double dy, double dz, double angleDeg) {
    m_measureLabel->setText(QStringLiteral("Dist: %1 m (Δ %2, %3, %4) ∠ %5°")
        .arg(distance, 0, 'f', 3)
        .arg(dx, 0, 'f', 2)
        .arg(dy, 0, 'f', 2)
        .arg(dz, 0, 'f', 2)
        .arg(angleDeg, 0, 'f', 1));
    m_measureLabel->setVisible(true);
}

// --- Operation HUD ---
PetuniaOperationHud::PetuniaOperationHud(QWidget *parent)
    : QFrame(parent)
{
    setFixedHeight(28);
    setStyleSheet(QStringLiteral(
        "PetuniaOperationHud { background-color: rgba(20, 20, 20, 220); border: 1px solid #505050; border-radius: 14px; }"
        "QLabel { color: #b58cff; font-weight: bold; font-size: 11px; padding: 0 10px; }"
    ));

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(6, 2, 6, 2);

    m_feedbackLabel = new QLabel(this);
    layout->addWidget(m_feedbackLabel);

    hide();
}

void PetuniaOperationHud::showFeedback(const QString &opName, const QString &details) {
    m_feedbackLabel->setText(QStringLiteral("%1: %2").arg(opName, details));
    adjustSize();
    show();
}

void PetuniaOperationHud::clearFeedback() {
    hide();
}

// --- Add Menu (Shift+A Popover) ---
PetuniaAddMenu::PetuniaAddMenu(QWidget *parent)
    : QMenu(parent)
{
    setTitle(QStringLiteral("Adicionar (Shift+A)"));
    setStyleSheet(QStringLiteral(
        "QMenu { background-color: #202020; border: 1px solid #444444; color: #eeeeee; font-size: 12px; padding: 4px; }"
        "QMenu::item { padding: 5px 18px; border-radius: 4px; }"
        "QMenu::item:selected { background-color: #b58cff; color: #121212; font-weight: bold; }"
    ));

    const struct { const char *id; const char *label; const char *shortcut; } prims[] = {
        {"cube", "Cubo (Cube)", "1"},
        {"sphere", "Esfera (Sphere)", "2"},
        {"cylinder", "Cilindro (Cylinder)", "3"},
        {"plane", "Plano (Plane)", "4"},
        {"cone", "Cone", "5"},
        {"capsule", "Cápsula (Capsule)", "6"},
        {"torus", "Toro (Torus)", "7"},
        {"circle", "Círculo (Circle)", "8"},
        {"icosphere", "Icoesfera (Icosphere)", "9"},
        {"wedge", "Cunha (Wedge)", "0"},
    };

    for (const auto &p : prims) {
        auto *act = addAction(QStringLiteral("%1  [%2]").arg(QString::fromUtf8(p.label), QString::fromUtf8(p.shortcut)));
        connect(act, &QAction::triggered, this, [this, id = QString::fromUtf8(p.id)]() {
            emit primitiveSelected(id);
        });
    }
}
