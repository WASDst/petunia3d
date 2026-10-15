#include "petunia_overlay_cards.h"
#include "petunia_editor_api.h"
#include "petunia_icons.h"
#include <QConicalGradient>
#include <QFormLayout>
#include <QGraphicsDropShadowEffect>
#include <QHBoxLayout>
#include <QLinearGradient>
#include <QPainter>
#include <QPainterPath>
#include <QVBoxLayout>
#include <QtMath>

// --- Primitive Card ---
PetuniaPrimitiveCard::PetuniaPrimitiveCard(QWidget *parent) : QFrame(parent) {
    setFrameShape(QFrame::StyledPanel);
    setStyleSheet(QStringLiteral(
        "PetuniaPrimitiveCard { background-color: rgba(24, 24, 24, 235); "
        "border: 1px solid #404040; border-radius: 8px; }"
        "QLabel { color: #eeeeee; font-weight: bold; font-size: 12px; }"
        "QDoubleSpinBox, QSpinBox { background-color: #292929; color: #eeeeee; "
        "border: 1px solid #404040; border-radius: 4px; padding: 3px; font-size: 12px; }"
        "QDoubleSpinBox:focus, QSpinBox:focus { border: 1px solid #b58cff; }"
        "QCheckBox { color: #b5b5b5; font-size: 11px; }"
        "QPushButton#ConfirmBtn { background-color: #b58cff; color: #121212; font-weight: bold; border: none; border-radius: 4px; padding: 4px 10px; }"
        "QPushButton#ConfirmBtn:hover { background-color: #c9a9ff; }"));

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(10, 8, 10, 8);
    layout->setSpacing(6);

    auto *headerRow = new QHBoxLayout();
    m_titleLabel = new QLabel(QStringLiteral("Adicionar Primitiva"), this);
    headerRow->addWidget(m_titleLabel);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), this);
    closeBtn->setFixedSize(16, 16);
    closeBtn->setStyleSheet(
        QStringLiteral("background: transparent; color: #888888; border: none; "
                       "font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, [this] {
        petuniaRequest("cancel_primitive");
        hide();
    });
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

    m_capEndsCheck =
        new QCheckBox(QStringLiteral("Fechar Extremidades (Caps)"), this);
    m_capEndsCheck->setChecked(true);
    form->addRow(m_capEndsCheck);
    connect(m_capEndsCheck, &QCheckBox::toggled, this, [](bool value) {
        petuniaRequest("update_primitive_param_bool",
                       {{"param", "cap_top"}, {"value", value}});
        petuniaRequest("update_primitive_param_bool",
                       {{"param", "cap_bottom"}, {"value", value}});
    });

    layout->addLayout(form);

    auto *confirm = new QPushButton(tr("Confirmar"), this);
    confirm->setObjectName(QStringLiteral("ConfirmBtn"));
    layout->addWidget(confirm);
    connect(confirm, &QPushButton::clicked, this, [this] {
        petuniaRequest("confirm_primitive");
        hide();
    });
    auto onParamChanged = [this]() {
        emit parameterChanged(static_cast<float>(m_sizeSpin->value()),
                              m_segmentsSpin->value(), m_ringsSpin->value());
    };

    connect(m_sizeSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged),
            this, onParamChanged);
    connect(m_segmentsSpin, QOverload<int>::of(&QSpinBox::valueChanged), this,
            onParamChanged);
    connect(m_ringsSpin, QOverload<int>::of(&QSpinBox::valueChanged), this,
            onParamChanged);

    adjustSize();
    hide();
}

void PetuniaPrimitiveCard::setPrimitiveType(const QString &type) {
    m_titleLabel->setText(QStringLiteral("Parâmetros: %1").arg(type));
    show();
}

// --- Loop Cut Card ---
PetuniaLoopCutCard::PetuniaLoopCutCard(QWidget *parent) : QFrame(parent) {
    setFrameShape(QFrame::StyledPanel);
    setStyleSheet(QStringLiteral(
        "PetuniaLoopCutCard { background-color: rgba(24, 24, 24, 235); border: "
        "1px solid #404040; border-radius: 8px; }"
        "QLabel { color: #eeeeee; font-weight: bold; font-size: 12px; }"
        "QDoubleSpinBox, QSpinBox { background-color: #292929; color: #eeeeee; "
        "border: 1px solid #404040; border-radius: 4px; padding: 3px; font-size: 12px; }"
        "QDoubleSpinBox:focus, QSpinBox:focus { border: 1px solid #b58cff; }"
        "QPushButton#ConfirmBtn { background-color: #b58cff; color: #121212; font-weight: bold; border: none; border-radius: 4px; padding: 4px 10px; }"
        "QPushButton#ConfirmBtn:hover { background-color: #c9a9ff; }"
        "QPushButton#CancelBtn { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 4px 10px; }"
        "QPushButton#CancelBtn:hover { background-color: #353535; }"));

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(10, 8, 10, 8);
    layout->setSpacing(6);

    auto *headerRow = new QHBoxLayout();
    auto *title = new QLabel(QStringLiteral("Corte em Loop (Loop Cut)"), this);
    headerRow->addWidget(title);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), this);
    closeBtn->setFixedSize(16, 16);
    closeBtn->setStyleSheet(
        QStringLiteral("background: transparent; color: #888888; border: none; "
                       "font-weight: bold;"));
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

    layout->addLayout(form);

    auto *confirm = new QPushButton(tr("Confirmar"), this);
    confirm->setObjectName(QStringLiteral("ConfirmBtn"));
    layout->addWidget(confirm);
    connect(confirm, &QPushButton::clicked, this, [this] {
        petuniaRequest("commit_loop_cut");
        hide();
    });
    auto *cancel = new QPushButton(tr("Cancelar"), this);
    cancel->setObjectName(QStringLiteral("CancelBtn"));
    layout->addWidget(cancel);
    connect(cancel, &QPushButton::clicked, this, [this] {
        petuniaRequest("cancel_loop_cut");
        hide();
    });
    auto onParamChanged = [this]() {
        emit loopCutChanged(m_cutsSpin->value(),
                            static_cast<float>(m_slideSpin->value()), 0.0f);
    };

    connect(m_cutsSpin, QOverload<int>::of(&QSpinBox::valueChanged), this,
            onParamChanged);
    connect(m_slideSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged),
            this, onParamChanged);

    adjustSize();
    hide();
}

// --- Tool Options Bar (Snapping, Proportional, Pivot, Shading & Overlays) ---
PetuniaToolOptionsBar::PetuniaToolOptionsBar(QWidget *parent) : QFrame(parent) {
    setFixedHeight(38);
    setStyleSheet(QStringLiteral(
        "PetuniaToolOptionsBar { background-color: rgba(24, 24, 24, 235); "
        "border: 1px solid #383838; border-radius: 19px; }"
        "QPushButton { background: transparent; color: #b5b5b5; font-size: "
        "11px; border: none; border-radius: 10px; padding: 2px "
        "8px; font-weight: bold; min-height: 22px; }"
        "QPushButton:hover { background-color: rgba(255, 255, 255, 0.10); color: #eeeeee; }"
        "QPushButton:pressed { background-color: rgba(255, 255, 255, 0.18); }"
        "QPushButton:checked { background-color: rgba(181, 140, 255, 0.32); color: #ffffff; }"
        "QPushButton#DomainBtn:checked { background-color: #e96a00; color: #ffffff; }"
        "QComboBox { background-color: #292929; color: #eeeeee; font-size: "
        "11px; border: 1px solid #383838; border-radius: 4px; padding: 2px "
        "6px; min-height: 20px; }"
        "QDoubleSpinBox { background-color: #292929; color: #eeeeee; "
        "font-size: 11px; border: 1px solid #383838; border-radius: 4px; "
        "padding: 2px 4px; min-height: 20px; }"
        "QLabel { color: #888888; font-size: 11px; }"));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(16);
    shadow->setColor(QColor(0, 0, 0, 160));
    shadow->setOffset(0, 4);
    setGraphicsEffect(shadow);

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(8, 4, 8, 4);
    layout->setSpacing(4);

    // 1. Selection Domains (Object, Vertex, Edge, Face / Shape, Curve, Point, Region)
    m_domainGroup = new QWidget(this);
    auto *domainLayout = new QHBoxLayout(m_domainGroup);
    domainLayout->setContentsMargins(0, 0, 0, 0);
    domainLayout->setSpacing(3);

    m_domainObjBtn = new QPushButton(PetuniaIcons::get("mode_object"), QString(), m_domainGroup);
    m_domainObjBtn->setObjectName(QStringLiteral("DomainBtn"));
    m_domainObjBtn->setCheckable(true);
    m_domainObjBtn->setChecked(true);
    m_domainObjBtn->setFixedSize(28, 26);
    m_domainObjBtn->setToolTip(QStringLiteral("Modo Objeto (Atalho 1)"));
    connect(m_domainObjBtn, &QPushButton::clicked, this, [this]() {
        emit selectionDomainChanged(0);
    });
    domainLayout->addWidget(m_domainObjBtn);

    m_domainVertBtn = new QPushButton(PetuniaIcons::get("select_vertex"), QString(), m_domainGroup);
    m_domainVertBtn->setObjectName(QStringLiteral("DomainBtn"));
    m_domainVertBtn->setCheckable(true);
    m_domainVertBtn->setFixedSize(28, 26);
    m_domainVertBtn->setToolTip(QStringLiteral("Modo Vértice / Ponto (Atalho 1 em POLY)"));
    connect(m_domainVertBtn, &QPushButton::clicked, this, [this]() {
        emit selectionDomainChanged(1);
    });
    domainLayout->addWidget(m_domainVertBtn);

    m_domainEdgeBtn = new QPushButton(PetuniaIcons::get("select_edge"), QString(), m_domainGroup);
    m_domainEdgeBtn->setObjectName(QStringLiteral("DomainBtn"));
    m_domainEdgeBtn->setCheckable(true);
    m_domainEdgeBtn->setFixedSize(28, 26);
    m_domainEdgeBtn->setToolTip(QStringLiteral("Modo Aresta / Curva (Atalho 2)"));
    connect(m_domainEdgeBtn, &QPushButton::clicked, this, [this]() {
        emit selectionDomainChanged(2);
    });
    domainLayout->addWidget(m_domainEdgeBtn);

    m_domainFaceBtn = new QPushButton(PetuniaIcons::get("select_face"), QString(), m_domainGroup);
    m_domainFaceBtn->setObjectName(QStringLiteral("DomainBtn"));
    m_domainFaceBtn->setCheckable(true);
    m_domainFaceBtn->setFixedSize(28, 26);
    m_domainFaceBtn->setToolTip(QStringLiteral("Modo Face / Região (Atalho 3)"));
    connect(m_domainFaceBtn, &QPushButton::clicked, this, [this]() {
        emit selectionDomainChanged(3);
    });
    domainLayout->addWidget(m_domainFaceBtn);

    layout->addWidget(m_domainGroup);

    m_sepDomain = new QFrame(this);
    m_sepDomain->setFrameShape(QFrame::VLine);
    m_sepDomain->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    layout->addWidget(m_sepDomain);

    // 2. Transform Helpers (Snap, Proportional, Pivot)
    m_transformGroup = new QWidget(this);
    auto *transLayout = new QHBoxLayout(m_transformGroup);
    transLayout->setContentsMargins(0, 0, 0, 0);
    transLayout->setSpacing(4);

    m_snapBtn = new QPushButton(PetuniaIcons::get("snap_magnet"), QString(), m_transformGroup);
    m_snapBtn->setCheckable(true);
    m_snapBtn->setFixedSize(28, 26);
    m_snapBtn->setToolTip(QStringLiteral("Ativar/Desativar Snap (Shift+Tab)"));
    connect(m_snapBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::snapToggled);
    transLayout->addWidget(m_snapBtn);

    m_snapTargetCombo = new QComboBox(m_transformGroup);
    m_snapTargetCombo->addItems({QStringLiteral("Grade"), QStringLiteral("Vértice"),
                                  QStringLiteral("Aresta"), QStringLiteral("Face")});
    m_snapTargetCombo->setToolTip(QStringLiteral("Alvo do Snap"));
    for (int i = 0; i < 4; ++i)
        m_snapTargetCombo->setItemData(i, QStringList{"grid", "vertex", "edge", "face"}[i]);
    connect(m_snapTargetCombo, &QComboBox::activated, this, [this](int i) {
        emit snapTargetChanged(m_snapTargetCombo->itemData(i).toString());
    });
    transLayout->addWidget(m_snapTargetCombo);

    m_propBtn = new QPushButton(PetuniaIcons::get("circle_dot"), QString(), m_transformGroup);
    m_propBtn->setCheckable(true);
    m_propBtn->setFixedSize(28, 26);
    m_propBtn->setToolTip(QStringLiteral("Edição Proporcional (O)"));
    connect(m_propBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::proportionalToggled);
    transLayout->addWidget(m_propBtn);

    m_propRadiusSpin = new QDoubleSpinBox(m_transformGroup);
    m_propRadiusSpin->setRange(0.05, 50.0);
    m_propRadiusSpin->setValue(1.0);
    m_propRadiusSpin->setSingleStep(0.1);
    m_propRadiusSpin->setSuffix(QStringLiteral(" m"));
    m_propRadiusSpin->setToolTip(QStringLiteral("Raio de Influência"));
    connect(m_propRadiusSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this,
            &PetuniaToolOptionsBar::proportionalRadiusChanged);
    transLayout->addWidget(m_propRadiusSpin);

    m_pivotCombo = new QComboBox(m_transformGroup);
    m_pivotCombo->addItems({QStringLiteral("Ponto Médio"), QStringLiteral("Cursor 3D"),
                            QStringLiteral("Origens Individuais"), QStringLiteral("Caixa Delimitadora")});
    m_pivotCombo->setToolTip(QStringLiteral("Ponto de Pivô para Transformação"));
    for (int i = 0; i < 4; ++i)
        m_pivotCombo->setItemData(i, QStringList{"median", "cursor", "individual", "bounds"}[i]);
    connect(m_pivotCombo, &QComboBox::activated, this, [this](int i) {
        emit pivotChanged(m_pivotCombo->itemData(i).toString());
    });
    transLayout->addWidget(m_pivotCombo);

    m_originBtn = new QPushButton(QStringLiteral("Origem ▾"), m_transformGroup);
    m_originBtn->setToolTip(QStringLiteral("Ajustar Origem do Objeto"));
    connect(m_originBtn, &QPushButton::clicked, this, &PetuniaToolOptionsBar::openOriginMenu);
    transLayout->addWidget(m_originBtn);

    layout->addWidget(m_transformGroup);

    m_sepTransform = new QFrame(this);
    m_sepTransform->setFrameShape(QFrame::VLine);
    m_sepTransform->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    layout->addWidget(m_sepTransform);

    // 3. Camera & View Controls
    m_planarBtn = new QPushButton(QStringLiteral("Planar"), this);
    m_planarBtn->setCheckable(true);
    m_planarBtn->setToolTip(QStringLiteral("Alternar Vista Planar / Livre"));
    connect(m_planarBtn, &QPushButton::clicked, this, &PetuniaToolOptionsBar::planarNavigationToggled);
    layout->addWidget(m_planarBtn);

    m_cameraGroup = new QWidget(this);
    auto *camLayout = new QHBoxLayout(m_cameraGroup);
    camLayout->setContentsMargins(0, 0, 0, 0);
    camLayout->setSpacing(1);

    m_perspBtn = new QPushButton(QStringLiteral("Persp"), m_cameraGroup);
    m_perspBtn->setCheckable(true);
    m_perspBtn->setChecked(true);
    m_perspBtn->setToolTip(QStringLiteral("Visão em Perspectiva (Numpad 5)"));
    camLayout->addWidget(m_perspBtn);

    m_orthoBtn = new QPushButton(QStringLiteral("Orto"), m_cameraGroup);
    m_orthoBtn->setCheckable(true);
    m_orthoBtn->setToolTip(QStringLiteral("Visão Ortográfica (Numpad 5)"));
    camLayout->addWidget(m_orthoBtn);

    m_iso21Btn = new QPushButton(QStringLiteral("2:1"), m_cameraGroup);
    m_iso21Btn->setCheckable(true);
    m_iso21Btn->setToolTip(QStringLiteral("Câmera Pseudo-Isométrica 2:1 (Numpad 2)"));
    camLayout->addWidget(m_iso21Btn);

    m_projBtn = m_perspBtn;
    m_isoBtn = m_iso21Btn;

    connect(m_perspBtn, &QPushButton::clicked, this, [this]() {
        emit cameraPresetRequested(QStringLiteral("persp"));
    });
    connect(m_orthoBtn, &QPushButton::clicked, this, [this]() {
        emit cameraPresetRequested(QStringLiteral("ortho"));
    });
    connect(m_iso21Btn, &QPushButton::clicked, this, [this]() {
        emit cameraPresetRequested(QStringLiteral("iso21"));
    });
    layout->addWidget(m_cameraGroup);

    m_frameBtn = new QPushButton(PetuniaIcons::get("frame_selection"), QString(), this);
    m_frameBtn->setToolTip(QStringLiteral("Enquadrar Seleção (Numpad .)"));
    connect(m_frameBtn, &QPushButton::clicked, this, &PetuniaToolOptionsBar::frameSelectionRequested);
    layout->addWidget(m_frameBtn);

    m_sepCamera = new QFrame(this);
    m_sepCamera->setFrameShape(QFrame::VLine);
    m_sepCamera->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    layout->addWidget(m_sepCamera);

    // 4. Shading & Overlays
    m_shadingCombo = new QComboBox(this);
    m_shadingCombo->addItems({QStringLiteral("○ Aramado"), QStringLiteral("◐ Sólido"),
                              QStringLiteral("● Material"), QStringLiteral("◉ Render")});
    m_shadingCombo->setCurrentIndex(1);
    for (int i = 0; i < 4; ++i)
        m_shadingCombo->setItemData(i, QStringList{"wireframe", "solid", "material", "rendered"}[i]);
    connect(m_shadingCombo, &QComboBox::activated, this, [this](int i) {
        emit shadingModeChanged(m_shadingCombo->itemData(i).toString());
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

    m_faceOrientationBtn = new QPushButton(QStringLiteral("Normais"), this);
    m_faceOrientationBtn->setCheckable(true);
    m_faceOrientationBtn->setToolTip(QStringLiteral("Orientação de Faces (Azul=Frente, Vermelho=Trás)"));
    connect(m_faceOrientationBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::faceOrientationToggled);
    layout->addWidget(m_faceOrientationBtn);

    m_uvCheckerBtn = new QPushButton(QStringLiteral("Checker"), this);
    m_uvCheckerBtn->setCheckable(true);
    m_uvCheckerBtn->setToolTip(QStringLiteral("Sobreposição de Xadrez UV"));
    connect(m_uvCheckerBtn, &QPushButton::toggled, this, &PetuniaToolOptionsBar::uvCheckerToggled);
    layout->addWidget(m_uvCheckerBtn);
}

bool PetuniaToolOptionsBar::isSnappingEnabled() const {
    return m_snapBtn->isChecked();
}
QString PetuniaToolOptionsBar::snapTarget() const {
    return m_snapTargetCombo->currentText();
}
bool PetuniaToolOptionsBar::isProportionalEditing() const {
    return m_propBtn->isChecked();
}
double PetuniaToolOptionsBar::proportionalRadius() const {
    return m_propRadiusSpin->value();
}
QString PetuniaToolOptionsBar::pivotPoint() const {
    return m_pivotCombo->currentText();
}

void PetuniaToolOptionsBar::openOriginMenu() {
    QMenu menu(this);
    menu.setStyleSheet(QStringLiteral(
        "QMenu { background-color: #202020; border: 1px solid #444444; color: "
        "#eeeeee; font-size: 11px; padding: 4px; }"
        "QMenu::item { padding: 4px 12px; border-radius: 3px; }"
        "QMenu::item:selected { background-color: #b58cff; color: #121212; "
        "font-weight: bold; }"));

    auto *actGeom = menu.addAction(QStringLiteral("Origem para Geometria"));
    auto *actBottom =
        menu.addAction(QStringLiteral("Origem para Base (Bottom)"));
    auto *actCursor = menu.addAction(QStringLiteral("Origem para Cursor 3D"));
    auto *actSelect = menu.addAction(QStringLiteral("Origem para Seleção"));
    auto *actCenter = menu.addAction(QStringLiteral("Geometria para Origem"));

    connect(actGeom, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("model.origin_to_geometry"));
    });
    connect(actBottom, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("model.origin_to_bottom"));
    });
    connect(actCursor, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("model.origin_to_cursor"));
    });
    connect(actSelect, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("model.origin_to_selection"));
    });
    connect(actCenter, &QAction::triggered, this, [this]() {
        emit actionTriggered(QStringLiteral("model.geometry_to_origin"));
    });

    menu.exec(m_originBtn->mapToGlobal(QPoint(0, m_originBtn->height())));
}

// --- Micro Inspector ---
PetuniaMicroInspector::PetuniaMicroInspector(QWidget *parent) : QFrame(parent) {
    resize(260, 190);
    setMinimumSize(200, 150);
    setStyleSheet(QStringLiteral(
        "PetuniaMicroInspector { background-color: rgba(28, 28, 28, 240); "
        "border: 1px solid #b58cff; border-radius: 8px; }"
        "QLabel { color: #dddddd; font-size: 11px; }"
        "QPushButton { background-color: #252525; color: #eeeeee; font-size: "
        "11px; border: 1px solid #3e3e3e; border-radius: 4px; padding: 3px; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; "
        "}"));

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
    title->setStyleSheet(
        QStringLiteral("color: #b58cff; font-weight: bold; font-size: 12px;"));
    header->addWidget(title);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), this);
    closeBtn->setFixedSize(16, 16);
    closeBtn->setStyleSheet(
        QStringLiteral("background: transparent; color: #888888; border: none; "
                       "font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, [this]() {
        hide();
        emit closed();
    });
    header->addWidget(closeBtn);
    layout->addLayout(header);

    // Quick Tool Buttons
    auto *toolsRow = new QHBoxLayout();
    toolsRow->setSpacing(4);

    const struct {
        const char *id;
        const char *label;
    } tools[] = {{"select", "Sel"},
                 {"move", "G"},
                 {"rotate", "R"},
                 {"scale", "S"},
                 {"measure", "M"}};
    for (const auto &t : tools) {
        auto *btn = new QPushButton(QString::fromUtf8(t.label), this);
        btn->setFixedHeight(24);
        connect(
            btn, &QPushButton::clicked, this,
            [this, id = QString::fromUtf8(t.id)]() { emit toolSelected(id); });
        toolsRow->addWidget(btn);
    }
    layout->addLayout(toolsRow);

    auto *sep = new QFrame(this);
    sep->setFrameShape(QFrame::HLine);
    sep->setStyleSheet(
        QStringLiteral("background-color: #383838; max-height: 1px;"));
    layout->addWidget(sep);

    // Readout Coordinates
    m_coordsLabel = new QLabel(
        QStringLiteral("Pos: 0.00, 0.00, 0.00\nRot: 0°, 0°, 0°"), this);
    m_coordsLabel->setStyleSheet(
        QStringLiteral("font-family: monospace; color: #b58cff;"));
    layout->addWidget(m_coordsLabel);

    m_measureLabel = new QLabel(QStringLiteral(""), this);
    m_measureLabel->setStyleSheet(QStringLiteral(
        "font-family: monospace; color: #e96a00; font-weight: bold;"));
    m_measureLabel->setVisible(false);
    layout->addWidget(m_measureLabel);

    // Direct Modeling Actions
    auto *actionsRow = new QHBoxLayout();
    actionsRow->setSpacing(4);
    const struct {
        const char *id;
        const char *label;
    } acts[] = {{"model.extrude", "Extrude"},
                {"model.inset", "Inset"},
                {"model.bevel", "Bevel"},
                {"model.loop_cut", "Loop"}};
    for (const auto &a : acts) {
        auto *btn = new QPushButton(QString::fromUtf8(a.label), this);
        btn->setFixedHeight(22);
        connect(btn, &QPushButton::clicked, this,
                [this, id = QString::fromUtf8(a.id)]() {
                    emit modelingActionTriggered(id);
                });
        actionsRow->addWidget(btn);
    }
    layout->addLayout(actionsRow);

    hide();
}

void PetuniaMicroInspector::setCoordinates(double x, double y, double z,
                                           double rx, double ry, double rz) {
    m_coordsLabel->setText(QStringLiteral("Pos: %1, %2, %3\nRot: %4°, %5°, %6°")
                               .arg(x, 0, 'f', 2)
                               .arg(y, 0, 'f', 2)
                               .arg(z, 0, 'f', 2)
                               .arg(rx, 0, 'f', 1)
                               .arg(ry, 0, 'f', 1)
                               .arg(rz, 0, 'f', 1));
}

void PetuniaMicroInspector::setMeasureData(double distance, double dx,
                                           double dy, double dz,
                                           double angleDeg) {
    m_measureLabel->setText(QStringLiteral("Dist: %1 m (Δ %2, %3, %4) ∠ %5°")
                                .arg(distance, 0, 'f', 3)
                                .arg(dx, 0, 'f', 2)
                                .arg(dy, 0, 'f', 2)
                                .arg(dz, 0, 'f', 2)
                                .arg(angleDeg, 0, 'f', 1));
    m_measureLabel->setVisible(true);
}

// --- Operation HUD ---
PetuniaOperationHud::PetuniaOperationHud(QWidget *parent) : QFrame(parent) {
    setFixedHeight(28);
    setStyleSheet(QStringLiteral(
        "PetuniaOperationHud { background-color: rgba(20, 20, 20, 220); "
        "border: 1px solid #505050; border-radius: 14px; }"
        "QLabel { color: #b58cff; font-weight: bold; font-size: 11px; padding: "
        "0 10px; }"));

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(6, 2, 6, 2);

    m_feedbackLabel = new QLabel(this);
    layout->addWidget(m_feedbackLabel);

    hide();
}

void PetuniaOperationHud::showFeedback(const QString &opName,
                                       const QString &details) {
    m_feedbackLabel->setText(QStringLiteral("%1: %2").arg(opName, details));
    adjustSize();
    show();
}

void PetuniaOperationHud::clearFeedback() { hide(); }

// --- Add Menu (Shift+A Popover) ---
PetuniaAddMenu::PetuniaAddMenu(QWidget *parent) : QMenu(parent) {
    setTitle(QStringLiteral("Adicionar (Shift+A)"));
    setStyleSheet(QStringLiteral(
        "QMenu { background-color: #202020; border: 1px solid #444444; color: "
        "#eeeeee; font-size: 12px; padding: 4px; }"
        "QMenu::item { padding: 5px 18px; border-radius: 4px; }"
        "QMenu::item:selected { background-color: #b58cff; color: #121212; "
        "font-weight: bold; }"));

    const struct {
        const char *id;
        const char *label;
        const char *shortcut;
    } prims[] = {
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
        auto *act = addAction(QStringLiteral("%1  [%2]")
                                  .arg(QString::fromUtf8(p.label),
                                       QString::fromUtf8(p.shortcut)));
        connect(act, &QAction::triggered, this,
                [this, id = QString::fromUtf8(p.id)]() {
                    emit primitiveSelected(id);
                });
    }
}

void PetuniaToolOptionsBar::sync(const QJsonObject &state) {
    for (auto *widget : findChildren<QWidget *>())
        widget->blockSignals(true);

    const auto ws = state["workspace"].toString();
    const auto mode = state["modeling_mode"].toString();
    const bool isDraw = (ws == "Model" && mode == "Draw");
    const bool isPoly = (ws == "Model" && mode != "Draw");
    const bool isPaint = (ws == "Paint");

    // Domain Adaptability
    if (isDraw) {
        m_domainGroup->show();
        m_sepDomain->show();
        m_domainObjBtn->setToolTip(QStringLiteral("Modo Shape (Atalho 1)"));
        m_domainVertBtn->setToolTip(QStringLiteral("Modo Ponto (Atalho 1 em DRAW)"));
        m_domainEdgeBtn->setToolTip(QStringLiteral("Modo Curva (Atalho 2)"));
        m_domainFaceBtn->setToolTip(QStringLiteral("Modo Região (Atalho 3)"));
        m_domainObjBtn->show();
        m_domainVertBtn->show();
        m_domainEdgeBtn->show();
        m_domainFaceBtn->show();
    } else if (isPoly) {
        m_domainGroup->show();
        m_sepDomain->show();
        m_domainObjBtn->setToolTip(QStringLiteral("Modo Objeto (Atalho 1)"));
        m_domainVertBtn->setToolTip(QStringLiteral("Modo Vértice (Atalho 1 em POLY)"));
        m_domainEdgeBtn->setToolTip(QStringLiteral("Modo Aresta (Atalho 2)"));
        m_domainFaceBtn->setToolTip(QStringLiteral("Modo Face (Atalho 3)"));
        m_domainObjBtn->show();
        m_domainVertBtn->show();
        m_domainEdgeBtn->show();
        m_domainFaceBtn->show();
    } else if (isPaint) {
        m_domainGroup->show();
        m_sepDomain->show();
        m_domainObjBtn->setToolTip(QStringLiteral("Modo Objeto"));
        m_domainFaceBtn->setToolTip(QStringLiteral("Modo Face"));
        m_domainObjBtn->show();
        m_domainFaceBtn->show();
        m_domainVertBtn->hide();
        m_domainEdgeBtn->hide();
    } else {
        m_domainGroup->hide();
        m_sepDomain->hide();
    }

    const auto dom = state["selection_domain"].toString();
    m_domainObjBtn->setChecked(dom == "Object");
    m_domainVertBtn->setChecked(dom == "Vertex");
    m_domainEdgeBtn->setChecked(dom == "Edge");
    m_domainFaceBtn->setChecked(dom == "Face");

    // Transform Group (Modeling only)
    const bool hasTransform = isDraw || isPoly;
    m_transformGroup->setVisible(hasTransform);
    m_sepTransform->setVisible(hasTransform);

    m_snapBtn->setChecked(state["snap_enabled"].toBool());
    m_propBtn->setChecked(state["proportional_editing"].toBool());
    m_propRadiusSpin->setValue(state["proportional_radius"].toDouble());

    // Camera buttons
    m_planarBtn->setVisible(isDraw);
    m_planarBtn->setChecked(state["planar_navigation"].toBool());
    m_cameraGroup->setVisible(!isDraw);
    const bool isOrtho = state["is_orthographic"].toBool();
    const bool isIso21 = state["camera_preset"].toString() == "iso21";
    m_iso21Btn->setChecked(isIso21);
    m_orthoBtn->setChecked(isOrtho && !isIso21);
    m_perspBtn->setChecked(!isOrtho && !isIso21);

    // Shading & Overlays
    m_xrayBtn->setChecked(state["show_xray"].toBool());
    m_matcapBtn->setChecked(state["viewport_matcap"].toBool());
    m_aoBtn->setChecked(state["viewport_ambient_occlusion"].toBool());
    m_faceOrientationBtn->setChecked(state["show_face_orientation"].toBool());
    m_uvCheckerBtn->setChecked(state["show_uv_checker"].toBool());

    for (auto pair :
         QList<QPair<QComboBox *, QString>>{{m_snapTargetCombo, "snap_target"},
                                            {m_pivotCombo, "pivot_id"},
                                            {m_shadingCombo, "shading_mode"}}) {
        int index = pair.first->findData(state[pair.second].toString());
        if (index >= 0)
            pair.first->setCurrentIndex(index);
    }

    for (auto *widget : findChildren<QWidget *>())
        widget->blockSignals(false);
}

// =========================================================================
// PetuniaFloatingContextBar (Adaptive Bottom Context Bar)
// =========================================================================

PetuniaFloatingContextBar::PetuniaFloatingContextBar(QWidget *parent) : QFrame(parent) {
    setFixedHeight(40);
    setStyleSheet(QStringLiteral(
        "PetuniaFloatingContextBar { background-color: rgba(24, 24, 24, 235); "
        "border: 1px solid #383838; border-radius: 20px; }\n"
        "QPushButton { background: transparent; color: #b5b5b5; font-size: 11px; "
        "border: none; border-radius: 6px; padding: 2px 4px; font-weight: bold; min-height: 24px; }\n"
        "QPushButton:hover { background-color: rgba(255, 255, 255, 0.10); color: #ffffff; }\n"
        "QPushButton:pressed { background-color: rgba(255, 255, 255, 0.18); }\n"
        "QPushButton:checked { background-color: rgba(181, 140, 255, 0.32); color: #ffffff; }\n"
        "QLabel { color: #888888; font-size: 11px; font-weight: bold; padding: 0 4px; }"));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(18);
    shadow->setColor(QColor(0, 0, 0, 180));
    shadow->setOffset(0, 4);
    setGraphicsEffect(shadow);

    auto *rootLayout = new QHBoxLayout(this);
    rootLayout->setContentsMargins(10, 4, 10, 4);
    rootLayout->setSpacing(4);

    // Helper lambdas for buttons and separators
    auto makeBtn = [this](QWidget *parent, const char *cmd, const char *icon, const char *tip, bool checkable = false) {
        auto *btn = new QPushButton(parent);
        btn->setFixedSize(28, 28);
        btn->setIcon(PetuniaIcons::get(QString::fromUtf8(icon)));
        btn->setIconSize(QSize(18, 18));
        btn->setToolTip(QString::fromUtf8(tip));
        btn->setProperty("toolCmd", QString::fromUtf8(cmd));
        if (checkable)
            btn->setCheckable(true);
        const QString c = QString::fromUtf8(cmd);
        connect(btn, &QPushButton::clicked, this, [this, c]() {
            if (c == "loop_cut")
                emit actionTriggered(QStringLiteral("begin_loop_cut"));
            else if (c == "poly_pen")
                emit toolChanged(QStringLiteral("poly_pen"));
            else if (c == "duplicate" || c == "delete" || c == "origin_geometry")
                emit actionTriggered(c);
            else
                emit commandExecuted(c);
        });
        return btn;
    };

    auto makeSep = [](QWidget *parent) {
        auto *sep = new QFrame(parent);
        sep->setFrameShape(QFrame::VLine);
        sep->setStyleSheet(QStringLiteral("background-color: rgba(255, 255, 255, 0.12); max-width: 1px; margin: 4px 2px;"));
        return sep;
    };

    // 1. MODEL / POLY Context Container
    m_modelWidget = new QWidget(this);
    auto *modelLayout = new QHBoxLayout(m_modelWidget);
    modelLayout->setContentsMargins(0, 0, 0, 0);
    modelLayout->setSpacing(2);

    // 1a. Object Domain Tools
    m_domainObjectWidget = new QWidget(m_modelWidget);
    auto *objLayout = new QHBoxLayout(m_domainObjectWidget);
    objLayout->setContentsMargins(0, 0, 0, 0);
    objLayout->setSpacing(2);
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "duplicate", "copy", "Duplicar Objeto (Shift+D)"));
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "model.separate_selection", "separate", "Separar Seleção (P)"));
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "model.join", "boolean_join", "Unir Objetos (Ctrl+J)"));
    objLayout->addWidget(makeSep(m_domainObjectWidget));
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "origin_geometry", "origin", "Definir Origem na Geometria"));
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "view.shade_smooth", "shade_smooth", "Sombreamento Suave"));
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "view.shade_flat", "shade_flat", "Sombreamento Plano"));
    objLayout->addWidget(makeSep(m_domainObjectWidget));
    objLayout->addWidget(makeBtn(m_domainObjectWidget, "delete", "delete", "Excluir Objeto (X)"));
    modelLayout->addWidget(m_domainObjectWidget);

    // 1b. Vertex Domain Tools
    m_domainVertexWidget = new QWidget(m_modelWidget);
    auto *vertLayout = new QHBoxLayout(m_domainVertexWidget);
    vertLayout->setContentsMargins(0, 0, 0, 0);
    vertLayout->setSpacing(2);
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "model.extrude", "tool_extrude", "Extrudar Vértices (E)", true));
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "model.bevel", "tool_bevel", "Chanfrar Vértices (Ctrl+B)", true));
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "model.merge", "tool_merge", "Fundir Vértices (M)"));
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "model.connect", "tool_connect", "Conectar Vértices (J)"));
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "model.make_face", "make_face", "Criar Face / Aresta (F)"));
    vertLayout->addWidget(makeSep(m_domainVertexWidget));
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "model.dissolve", "tool_dissolve", "Dissolver Vértices (Ctrl+X)"));
    vertLayout->addWidget(makeBtn(m_domainVertexWidget, "delete", "delete", "Excluir Vértices (X)"));
    modelLayout->addWidget(m_domainVertexWidget);

    // 1c. Edge Domain Tools
    m_domainEdgeWidget = new QWidget(m_modelWidget);
    auto *edgeLayout = new QHBoxLayout(m_domainEdgeWidget);
    edgeLayout->setContentsMargins(0, 0, 0, 0);
    edgeLayout->setSpacing(2);
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "model.extrude", "tool_extrude", "Extrudar Arestas (E)", true));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "model.bevel", "tool_bevel", "Chanfrar Arestas (Ctrl+B)", true));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "loop_cut", "tool_loopcut", "Inserir Corte em Loop (Ctrl+R)", true));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "model.subdivide", "tool_subdivide", "Subdividir Arestas"));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "model.knife", "tool_knife", "Corte por Faca (K)", true));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "model.make_face", "make_face", "Criar Face a partir de Arestas (F)"));
    edgeLayout->addWidget(makeSep(m_domainEdgeWidget));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "model.dissolve", "tool_dissolve", "Dissolver Arestas (Ctrl+X)"));
    edgeLayout->addWidget(makeBtn(m_domainEdgeWidget, "delete", "delete", "Excluir Arestas (X)"));
    modelLayout->addWidget(m_domainEdgeWidget);

    // 1d. Face Domain Tools
    m_domainFaceWidget = new QWidget(m_modelWidget);
    auto *faceLayout = new QHBoxLayout(m_domainFaceWidget);
    faceLayout->setContentsMargins(0, 0, 0, 0);
    faceLayout->setSpacing(2);
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.extrude", "tool_extrude", "Extrudar Faces (E)", true));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.push_pull", "tool_pushpull", "Push / Pull Faces (Shift+P)", true));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.inset", "tool_inset", "Inserir Borda Poligonal / Inset (I)", true));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.bevel", "tool_bevel", "Chanfrar Faces (Ctrl+B)", true));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.knife", "tool_knife", "Corte por Faca (K)", true));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.subdivide", "tool_subdivide", "Subdividir Faces"));
    faceLayout->addWidget(makeSep(m_domainFaceWidget));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.flip_normals", "flip_normals", "Inverter Normais"));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "model.dissolve", "tool_dissolve", "Dissolver Faces (Ctrl+X)"));
    faceLayout->addWidget(makeBtn(m_domainFaceWidget, "delete", "delete", "Excluir Faces (X)"));
    modelLayout->addWidget(m_domainFaceWidget);

    rootLayout->addWidget(m_modelWidget);

    // 2. DRAW Context Tools
    m_drawWidget = new QWidget(this);
    auto *drawLayout = new QHBoxLayout(m_drawWidget);
    drawLayout->setContentsMargins(0, 0, 0, 0);
    drawLayout->setSpacing(3);

    auto *rectBtn = new QPushButton(m_drawWidget);
    rectBtn->setFixedSize(28, 28);
    rectBtn->setIcon(PetuniaIcons::get(QStringLiteral("paint_rect")));
    rectBtn->setIconSize(QSize(18, 18));
    rectBtn->setToolTip(QStringLiteral("Retângulo (Desenhar perfil retangular)"));
    connect(rectBtn, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("profile_rectangle"));
    });
    drawLayout->addWidget(rectBtn);

    auto *circleBtn = new QPushButton(m_drawWidget);
    circleBtn->setFixedSize(28, 28);
    circleBtn->setIcon(PetuniaIcons::get(QStringLiteral("circle")));
    circleBtn->setIconSize(QSize(18, 18));
    circleBtn->setToolTip(QStringLiteral("Círculo (Desenhar perfil circular)"));
    connect(circleBtn, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("profile_circle"));
    });
    drawLayout->addWidget(circleBtn);

    auto *closeBtn = new QPushButton(m_drawWidget);
    closeBtn->setFixedSize(28, 28);
    closeBtn->setIcon(PetuniaIcons::get(QStringLiteral("profile_closed")));
    closeBtn->setIconSize(QSize(18, 18));
    closeBtn->setToolTip(QStringLiteral("Fechar perfil ativo"));
    connect(closeBtn, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("close_profile"));
    });
    drawLayout->addWidget(closeBtn);

    auto *sepDraw = new QFrame(m_drawWidget);
    sepDraw->setFrameShape(QFrame::VLine);
    sepDraw->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px; margin: 4px 2px;"));
    drawLayout->addWidget(sepDraw);

    const struct { const char *act; const char *label; const char *tip; } planes[] = {
        {"workplane_auto", "Auto", "Plano de trabalho automático"},
        {"workplane_ground", "Chão", "Plano de trabalho no chão (XZ)"},
        {"workplane_face", "Face", "Plano alinhado à face selecionada"},
        {"workplane_view", "Vista", "Plano alinhado à câmera atual"}
    };
    for (const auto &p : planes) {
        auto *pBtn = new QPushButton(QString::fromUtf8(p.label), m_drawWidget);
        pBtn->setToolTip(QString::fromUtf8(p.tip));
        const QString act = QString::fromUtf8(p.act);
        connect(pBtn, &QPushButton::clicked, this, [this, act]() {
            emit actionTriggered(act);
        });
        drawLayout->addWidget(pBtn);
    }
    auto *lookBtn = new QPushButton(QStringLiteral("Olhar p/ Plano"), m_drawWidget);
    lookBtn->setToolTip(QStringLiteral("Alinhar vista da câmera com o plano ativo"));
    connect(lookBtn, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("workplane_look"));
    });
    drawLayout->addWidget(lookBtn);

    rootLayout->addWidget(m_drawWidget);

    // 3. PAINT Context Tools
    m_paintWidget = new QWidget(this);
    auto *paintLayout = new QHBoxLayout(m_paintWidget);
    paintLayout->setContentsMargins(0, 0, 0, 0);
    paintLayout->setSpacing(4);

    m_paintIsolateBtn = new QPushButton(QStringLiteral("Isolar"), m_paintWidget);
    m_paintIsolateBtn->setCheckable(true);
    m_paintIsolateBtn->setToolTip(QStringLiteral("Isolar Objeto Ativo (Numpad /)"));
    connect(m_paintIsolateBtn, &QPushButton::clicked, this, [this]() {
        emit commandExecuted(QStringLiteral("view.isolate"));
    });
    paintLayout->addWidget(m_paintIsolateBtn);

    auto *sepPaint1 = new QFrame(m_paintWidget);
    sepPaint1->setFrameShape(QFrame::VLine);
    sepPaint1->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    paintLayout->addWidget(sepPaint1);

    m_colorSwatchBtn = new QPushButton(m_paintWidget);
    m_colorSwatchBtn->setFixedSize(22, 22);
    m_colorSwatchBtn->setCursor(Qt::PointingHandCursor);
    m_colorSwatchBtn->setToolTip(QStringLiteral("Cor Ativa do Pincel (clique para alterar)"));
    m_colorSwatchBtn->setStyleSheet(QStringLiteral(
        "background-color: #b58cff; border: 1px solid #666666; border-radius: 11px;"));
    connect(m_colorSwatchBtn, &QPushButton::clicked, this, [this]() {
        emit colorPickerRequested();
    });
    paintLayout->addWidget(m_colorSwatchBtn);

    auto *sizeMinusBtn = new QPushButton(QStringLiteral("−"), m_paintWidget);
    sizeMinusBtn->setFixedWidth(24);
    connect(sizeMinusBtn, &QPushButton::clicked, this, [this]() {
        emit brushSizeDelta(-2.0f);
    });
    paintLayout->addWidget(sizeMinusBtn);

    m_brushSizeLabel = new QLabel(QStringLiteral("Tam: 16px"), m_paintWidget);
    paintLayout->addWidget(m_brushSizeLabel);

    auto *sizePlusBtn = new QPushButton(QStringLiteral("+"), m_paintWidget);
    sizePlusBtn->setFixedWidth(24);
    connect(sizePlusBtn, &QPushButton::clicked, this, [this]() {
        emit brushSizeDelta(2.0f);
    });
    paintLayout->addWidget(sizePlusBtn);

    auto *opacMinusBtn = new QPushButton(QStringLiteral("−"), m_paintWidget);
    opacMinusBtn->setFixedWidth(24);
    connect(opacMinusBtn, &QPushButton::clicked, this, [this]() {
        emit brushOpacityDelta(-0.1f);
    });
    paintLayout->addWidget(opacMinusBtn);

    m_brushOpacityLabel = new QLabel(QStringLiteral("Opac: 100%"), m_paintWidget);
    paintLayout->addWidget(m_brushOpacityLabel);

    auto *opacPlusBtn = new QPushButton(QStringLiteral("+"), m_paintWidget);
    opacPlusBtn->setFixedWidth(24);
    connect(opacPlusBtn, &QPushButton::clicked, this, [this]() {
        emit brushOpacityDelta(0.1f);
    });
    paintLayout->addWidget(opacPlusBtn);

    auto *sepPaint2 = new QFrame(m_paintWidget);
    sepPaint2->setFrameShape(QFrame::VLine);
    sepPaint2->setStyleSheet(QStringLiteral("background-color: #404040; max-width: 1px;"));
    paintLayout->addWidget(sepPaint2);

    m_symXBtn = new QPushButton(QStringLiteral("X"), m_paintWidget);
    m_symXBtn->setCheckable(true);
    m_symXBtn->setToolTip(QStringLiteral("Simetria no Eixo X"));
    connect(m_symXBtn, &QPushButton::toggled, this, [this](bool c) {
        emit symmetryToggled(QStringLiteral("x"), c);
    });
    paintLayout->addWidget(m_symXBtn);

    m_symYBtn = new QPushButton(QStringLiteral("Y"), m_paintWidget);
    m_symYBtn->setCheckable(true);
    m_symYBtn->setToolTip(QStringLiteral("Simetria no Eixo Y"));
    connect(m_symYBtn, &QPushButton::toggled, this, [this](bool c) {
        emit symmetryToggled(QStringLiteral("y"), c);
    });
    paintLayout->addWidget(m_symYBtn);

    m_symZBtn = new QPushButton(QStringLiteral("Z"), m_paintWidget);
    m_symZBtn->setCheckable(true);
    m_symZBtn->setToolTip(QStringLiteral("Simetria no Eixo Z"));
    connect(m_symZBtn, &QPushButton::toggled, this, [this](bool c) {
        emit symmetryToggled(QStringLiteral("z"), c);
    });
    paintLayout->addWidget(m_symZBtn);

    rootLayout->addWidget(m_paintWidget);

    // 4. UV Context Tools
    m_uvWidget = new QWidget(this);
    auto *uvLayout = new QHBoxLayout(m_uvWidget);
    uvLayout->setContentsMargins(0, 0, 0, 0);
    uvLayout->setSpacing(3);

    const struct { const char *cmd; const char *label; } uvTools[] = {
        {"uv.unwrap_auto", "Unwrap"}, {"uv.pack_islands", "Empacotar"},
        {"uv.project_reference", "Projetar Ref"}, {"uv.stitch", "Costurar"},
        {"uv.relax", "Relaxar"}
    };
    for (const auto &u : uvTools) {
        auto *btn = new QPushButton(QString::fromUtf8(u.label), m_uvWidget);
        const QString cmd = QString::fromUtf8(u.cmd);
        connect(btn, &QPushButton::clicked, this, [this, cmd]() {
            emit commandExecuted(cmd);
        });
        uvLayout->addWidget(btn);
    }
    auto *uvChecker = new QPushButton(QStringLiteral("Checker"), m_uvWidget);
    uvChecker->setCheckable(true);
    connect(uvChecker, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("uv_checker"));
    });
    uvLayout->addWidget(uvChecker);

    auto *uvTexture = new QPushButton(QStringLiteral("Textura"), m_uvWidget);
    uvTexture->setCheckable(true);
    connect(uvTexture, &QPushButton::clicked, this, [this]() {
        emit actionTriggered(QStringLiteral("uv_texture"));
    });
    uvLayout->addWidget(uvTexture);

    rootLayout->addWidget(m_uvWidget);

    // 5. ANIMATE Context Tools
    m_animateWidget = new QWidget(this);
    auto *animLayout = new QHBoxLayout(m_animateWidget);
    animLayout->setContentsMargins(0, 0, 0, 0);
    animLayout->setSpacing(3);

    auto *firstBtn = new QPushButton(QStringLiteral("|◀"), m_animateWidget);
    firstBtn->setToolTip(QStringLiteral("Ir para o primeiro frame"));
    connect(firstBtn, &QPushButton::clicked, this, [this]() {
        emit animateActionTriggered(QStringLiteral("first"));
    });
    animLayout->addWidget(firstBtn);

    auto *prevBtn = new QPushButton(QStringLiteral("◀◆"), m_animateWidget);
    prevBtn->setToolTip(QStringLiteral("Keyframe anterior"));
    connect(prevBtn, &QPushButton::clicked, this, [this]() {
        emit animateActionTriggered(QStringLiteral("prev_key"));
    });
    animLayout->addWidget(prevBtn);

    m_playBtn = new QPushButton(QStringLiteral("▶"), m_animateWidget);
    m_playBtn->setFixedWidth(36);
    m_playBtn->setToolTip(QStringLiteral("Reproduzir / Pausar (Espaço)"));
    connect(m_playBtn, &QPushButton::clicked, this, [this]() {
        emit animateActionTriggered(QStringLiteral("toggle_play"));
    });
    animLayout->addWidget(m_playBtn);

    auto *nextBtn = new QPushButton(QStringLiteral("◆▶"), m_animateWidget);
    nextBtn->setToolTip(QStringLiteral("Próximo keyframe"));
    connect(nextBtn, &QPushButton::clicked, this, [this]() {
        emit animateActionTriggered(QStringLiteral("next_key"));
    });
    animLayout->addWidget(nextBtn);

    auto *addKeyBtn = new QPushButton(QStringLiteral("+◆"), m_animateWidget);
    addKeyBtn->setToolTip(QStringLiteral("Inserir Keyframe (I)"));
    connect(addKeyBtn, &QPushButton::clicked, this, [this]() {
        emit animateActionTriggered(QStringLiteral("insert_key"));
    });
    animLayout->addWidget(addKeyBtn);

    m_animLabel = new QLabel(QStringLiteral("0.00s / F 0"), m_animateWidget);
    animLayout->addWidget(m_animLabel);

    rootLayout->addWidget(m_animateWidget);
}

void PetuniaFloatingContextBar::sync(const QJsonObject &state) {
    for (auto *widget : findChildren<QWidget *>())
        widget->blockSignals(true);

    const auto ws = state["workspace"].toString();
    const auto mode = state["modeling_mode"].toString();
    const bool isDraw = (ws == "Model" && mode == "Draw");
    const bool isPoly = (ws == "Model" && mode != "Draw");
    const bool isPaint = (ws == "Paint");
    const bool isUv = (ws == "Uv");
    const bool isAnimate = (ws == "Animate");

    m_modelWidget->setVisible(isPoly);
    m_drawWidget->setVisible(isDraw);
    m_paintWidget->setVisible(isPaint);
    m_uvWidget->setVisible(isUv);
    m_animateWidget->setVisible(isAnimate);

    if (isPoly) {
        const QString domain = state["selection_domain"].toString();
        m_domainObjectWidget->setVisible(domain == "Object" || domain.isEmpty());
        m_domainVertexWidget->setVisible(domain == "Vertex");
        m_domainEdgeWidget->setVisible(domain == "Edge");
        m_domainFaceWidget->setVisible(domain == "Face");

        const auto activeTool = state["active_tool"].toString();
        for (auto *btn : m_modelWidget->findChildren<QPushButton *>()) {
            const auto cmd = btn->property("toolCmd").toString();
            if (btn->isCheckable()) {
                const bool isActive =
                    (!activeTool.isEmpty() &&
                     (cmd == activeTool ||
                      (cmd == "loop_cut" && activeTool == "loop_cut") ||
                      (cmd == "poly_pen" && activeTool == "poly_pen")));
                btn->setChecked(isActive);
            }
        }
    } else if (isPaint) {
        m_paintIsolateBtn->setChecked(state["isolate_active"].toBool());
        m_brushSize = static_cast<float>(state["brush_size"].toDouble(16.0));
        m_brushOpacity = static_cast<float>(state["brush_opacity"].toDouble(1.0));
        m_brushSizeLabel->setText(QStringLiteral("Tam: %1px").arg(qRound(m_brushSize)));
        m_brushOpacityLabel->setText(QStringLiteral("Opac: %1%").arg(qRound(m_brushOpacity * 100)));

        const auto colorArr = state["paint_color"].toArray();
        if (colorArr.size() >= 3) {
            const int r = qBound(0, int(colorArr[0].toDouble() * 255), 255);
            const int g = qBound(0, int(colorArr[1].toDouble() * 255), 255);
            const int b = qBound(0, int(colorArr[2].toDouble() * 255), 255);
            m_colorSwatchBtn->setStyleSheet(QStringLiteral(
                "background-color: rgb(%1, %2, %3); border: 1px solid #777; border-radius: 11px;")
                .arg(r).arg(g).arg(b));
        }
        m_symXBtn->setChecked(state["paint_symmetry_x"].toBool());
        m_symYBtn->setChecked(state["paint_symmetry_y"].toBool());
        m_symZBtn->setChecked(state["paint_symmetry_z"].toBool());
    } else if (isAnimate) {
        const auto anim = state["animate"].toObject();
        const bool playing = anim["playing"].toBool();
        m_playBtn->setText(playing ? QStringLiteral("❚❚") : QStringLiteral("▶"));
        m_animLabel->setText(anim["time_label"].toString(QStringLiteral("0.00s")));
    }

    for (auto *widget : findChildren<QWidget *>())
        widget->blockSignals(false);

    adjustSize();
}

// =========================================================================
// PetuniaColorWheel
// =========================================================================

PetuniaColorWheel::PetuniaColorWheel(QWidget *parent) : QWidget(parent) {
    setFixedSize(140, 140);
    setCursor(Qt::CrossCursor);
}

QColor PetuniaColorWheel::color() const {
    return QColor::fromHsvF(m_hue / 360.0f, m_saturation, m_value);
}

void PetuniaColorWheel::setColor(const QColor &color) {
    const float h = color.hsvHueF();
    if (h >= 0.0f)
        m_hue = h * 360.0f;
    m_saturation = color.hsvSaturationF();
    m_value = color.valueF();
    update();
}

void PetuniaColorWheel::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);

    const QPointF c = rect().center();
    const float rOut = 65.0f;
    const float rIn = 50.0f;

    // 1. Hue Conical Ring
    QConicalGradient conic(c, 0.0);
    for (int deg = 0; deg <= 360; deg += 30) {
        conic.setColorAt(deg / 360.0, QColor::fromHsvF(deg / 360.0f, 1.0f, 1.0f));
    }

    QPainterPath ringPath;
    ringPath.addEllipse(c, rOut, rOut);
    QPainterPath holePath;
    holePath.addEllipse(c, rIn, rIn);
    QPainterPath ring = ringPath.subtracted(holePath);

    painter.fillPath(ring, conic);
    painter.setPen(QColor(60, 60, 60));
    painter.drawPath(ring);

    // Hue Marker on Ring
    const float rad = qDegreesToRadians(m_hue);
    const float rMid = (rOut + rIn) * 0.5f;
    const QPointF markerPos(c.x() + rMid * std::cos(rad), c.y() + rMid * std::sin(rad));
    painter.setPen(QPen(Qt::white, 2));
    painter.setBrush(QColor::fromHsvF(m_hue / 360.0f, 1.0f, 1.0f));
    painter.drawEllipse(markerPos, 4, 4);

    // 2. Inner Saturation / Value Box
    const QRectF svRect(c.x() - 32, c.y() - 32, 64, 64);

    // Horizontal Saturation Gradient
    QLinearGradient satGrad(svRect.left(), 0, svRect.right(), 0);
    satGrad.setColorAt(0, Qt::white);
    satGrad.setColorAt(1, QColor::fromHsvF(m_hue / 360.0f, 1.0f, 1.0f));
    painter.fillRect(svRect, satGrad);

    // Vertical Value Gradient (black at bottom)
    QLinearGradient valGrad(0, svRect.top(), 0, svRect.bottom());
    valGrad.setColorAt(0, QColor(0, 0, 0, 0));
    valGrad.setColorAt(1, Qt::black);
    painter.fillRect(svRect, valGrad);

    painter.setPen(QColor(70, 70, 70));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(svRect);

    // Saturation / Value Marker
    const QPointF svPos(svRect.left() + m_saturation * svRect.width(),
                        svRect.bottom() - m_value * svRect.height());
    painter.setPen(QPen(m_value > 0.5 ? Qt::black : Qt::white, 2));
    painter.setBrush(color());
    painter.drawEllipse(svPos, 4, 4);
}

void PetuniaColorWheel::handleInteraction(const QPointF &pos) {
    const QPointF c = rect().center();
    const QPointF delta = pos - c;
    const float dist = std::hypot(delta.x(), delta.y());
    const float rOut = 68.0f;
    const float rIn = 46.0f;

    if (dist >= rIn && dist <= rOut) {
        float angle = qRadiansToDegrees(std::atan2(delta.y(), delta.x()));
        if (angle < 0.0f)
            angle += 360.0f;
        m_hue = angle;
        emit colorChanged(color());
        update();
    } else {
        const QRectF svRect(c.x() - 32, c.y() - 32, 64, 64);
        if (svRect.adjusted(-6, -6, 6, 6).contains(pos)) {
            m_saturation = qBound(0.0f, float((pos.x() - svRect.left()) / svRect.width()), 1.0f);
            m_value = qBound(0.0f, float((svRect.bottom() - pos.y()) / svRect.height()), 1.0f);
            emit colorChanged(color());
            update();
        }
    }
}

void PetuniaColorWheel::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton)
        handleInteraction(event->position());
}

void PetuniaColorWheel::mouseMoveEvent(QMouseEvent *event) {
    if (event->buttons() & Qt::LeftButton)
        handleInteraction(event->position());
}

// =========================================================================
// PetuniaColorPickerPopover
// =========================================================================

PetuniaColorPickerPopover::PetuniaColorPickerPopover(QWidget *parent)
    : QDialog(parent, Qt::Popup | Qt::FramelessWindowHint) {
    setAttribute(Qt::WA_TranslucentBackground);
    setFixedSize(240, 360);

    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(8, 8, 8, 8);

    auto *card = new QWidget(this);
    card->setObjectName(QStringLiteral("ColorPickerCard"));
    card->setStyleSheet(QStringLiteral(
        "#ColorPickerCard { background-color: #202020; border: 1px solid #404040; border-radius: 10px; }\n"
        "QLabel { color: #eeeeee; font-size: 11px; font-weight: bold; }\n"
        "QDoubleSpinBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 2px 4px; font-size: 11px; min-height: 20px; }\n"
        "QLineEdit { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 2px 6px; font-size: 11px; min-height: 20px; }\n"
        "QPushButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 3px 8px; font-size: 11px; font-weight: bold; }\n"
        "QPushButton:hover { background-color: #353535; border-color: #b58cff; }\n"
        "QPushButton#ConfirmBtn { background-color: #b58cff; color: #121212; border: none; font-weight: bold; }\n"
        "QPushButton#ConfirmBtn:hover { background-color: #c9a9ff; }\n"
    ));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(20);
    shadow->setColor(QColor(0, 0, 0, 190));
    shadow->setOffset(0, 6);
    card->setGraphicsEffect(shadow);

    auto *layout = new QVBoxLayout(card);
    layout->setContentsMargins(10, 8, 10, 8);
    layout->setSpacing(6);

    // Header
    auto *headerRow = new QHBoxLayout();
    m_titleLabel = new QLabel(QStringLiteral("Seletor de Cor"), card);
    headerRow->addWidget(m_titleLabel, 1);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), card);
    closeBtn->setFixedSize(18, 18);
    closeBtn->setStyleSheet(QStringLiteral("background: transparent; border: none; color: #888; font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, &QDialog::reject);
    headerRow->addWidget(closeBtn);
    layout->addLayout(headerRow);

    // Wheel
    m_wheel = new PetuniaColorWheel(card);
    auto *wheelRow = new QHBoxLayout();
    wheelRow->setAlignment(Qt::AlignCenter);
    wheelRow->addWidget(m_wheel);
    layout->addLayout(wheelRow);

    connect(m_wheel, &PetuniaColorWheel::colorChanged, this, [this](const QColor &col) {
        if (m_updating)
            return;
        m_updating = true;
        updateWidgetsFromColor(col);
        emit colorChanged(col);
        m_updating = false;
    });

    // HSV Spinboxes
    auto *hsvRow = new QHBoxLayout();
    hsvRow->setSpacing(4);

    m_hueSpin = new QDoubleSpinBox(card);
    m_hueSpin->setRange(0, 360);
    m_hueSpin->setSuffix(QStringLiteral("°"));
    m_hueSpin->setToolTip(QStringLiteral("Matiz (Hue)"));
    hsvRow->addWidget(m_hueSpin);

    m_satSpin = new QDoubleSpinBox(card);
    m_satSpin->setRange(0, 100);
    m_satSpin->setSuffix(QStringLiteral("%"));
    m_satSpin->setToolTip(QStringLiteral("Saturação"));
    hsvRow->addWidget(m_satSpin);

    m_valSpin = new QDoubleSpinBox(card);
    m_valSpin->setRange(0, 100);
    m_valSpin->setSuffix(QStringLiteral("%"));
    m_valSpin->setToolTip(QStringLiteral("Luminosidade / Valor"));
    hsvRow->addWidget(m_valSpin);

    auto onHsvChanged = [this]() {
        if (m_updating)
            return;
        m_updating = true;
        const auto col = QColor::fromHsvF(float(m_hueSpin->value() / 360.0),
                                          float(m_satSpin->value() / 100.0),
                                          float(m_valSpin->value() / 100.0));
        m_wheel->setColor(col);
        updateWidgetsFromColor(col);
        emit colorChanged(col);
        m_updating = false;
    };
    connect(m_hueSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, onHsvChanged);
    connect(m_satSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, onHsvChanged);
    connect(m_valSpin, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, onHsvChanged);

    layout->addLayout(hsvRow);

    // Swatch + HEX
    auto *hexRow = new QHBoxLayout();
    hexRow->setSpacing(6);

    m_swatchPreview = new QFrame(card);
    m_swatchPreview->setFixedSize(24, 24);
    m_swatchPreview->setStyleSheet(QStringLiteral("border-radius: 4px; border: 1px solid #555; background-color: #b58cff;"));
    hexRow->addWidget(m_swatchPreview);

    auto *hexLabel = new QLabel(QStringLiteral("HEX:"), card);
    hexRow->addWidget(hexLabel);

    m_hexEdit = new QLineEdit(QStringLiteral("#B58CFF"), card);
    m_hexEdit->setMaxLength(7);
    connect(m_hexEdit, &QLineEdit::textEdited, this, [this](const QString &text) {
        if (m_updating)
            return;
        QColor col(text);
        if (col.isValid()) {
            m_updating = true;
            m_wheel->setColor(col);
            updateWidgetsFromColor(col);
            emit colorChanged(col);
            m_updating = false;
        }
    });
    hexRow->addWidget(m_hexEdit, 1);
    layout->addLayout(hexRow);

    // RGB Label
    m_rgbLabel = new QLabel(QStringLiteral("RGB: 181 / 140 / 255"), card);
    m_rgbLabel->setAlignment(Qt::AlignCenter);
    m_rgbLabel->setStyleSheet(QStringLiteral("color: #888; font-size: 10px;"));
    layout->addWidget(m_rgbLabel);

    // Presets Row
    auto *presetsRow = new QHBoxLayout();
    presetsRow->setSpacing(3);
    const QStringList presets = {
        QStringLiteral("#e96a00"), QStringLiteral("#4fc3f7"), QStringLiteral("#b58cff"),
        QStringLiteral("#81c784"), QStringLiteral("#ffd54f"), QStringLiteral("#e53935"),
        QStringLiteral("#ffffff"), QStringLiteral("#121212")
    };
    for (const auto &hex : presets) {
        auto *btn = new QPushButton(card);
        btn->setFixedSize(18, 18);
        btn->setCursor(Qt::PointingHandCursor);
        btn->setStyleSheet(QStringLiteral("background-color: %1; border: 1px solid #444; border-radius: 3px;").arg(hex));
        connect(btn, &QPushButton::clicked, this, [this, hex]() {
            setColor(QColor(hex));
            emit colorChanged(color());
        });
        presetsRow->addWidget(btn);
    }
    layout->addLayout(presetsRow);

    // Actions
    auto *actionsRow = new QHBoxLayout();
    auto *resetBtn = new QPushButton(QStringLiteral("Padrão"), card);
    connect(resetBtn, &QPushButton::clicked, this, [this]() {
        setColor(QColor(QStringLiteral("#e96a00")));
        emit colorChanged(color());
    });
    actionsRow->addWidget(resetBtn);

    auto *confirmBtn = new QPushButton(QStringLiteral("Confirmar"), card);
    confirmBtn->setObjectName(QStringLiteral("ConfirmBtn"));
    connect(confirmBtn, &QPushButton::clicked, this, &QDialog::accept);
    actionsRow->addWidget(confirmBtn);

    layout->addLayout(actionsRow);

    rootLayout->addWidget(card);
}

void PetuniaColorPickerPopover::setColor(const QColor &color) {
    m_updating = true;
    m_wheel->setColor(color);
    updateWidgetsFromColor(color);
    m_updating = false;
}

QColor PetuniaColorPickerPopover::color() const {
    return m_wheel->color();
}

void PetuniaColorPickerPopover::setTitle(const QString &title) {
    m_titleLabel->setText(title);
}

void PetuniaColorPickerPopover::updateWidgetsFromColor(const QColor &col) {
    const float h = col.hsvHueF();
    m_hueSpin->setValue(h >= 0 ? h * 360.0 : 0.0);
    m_satSpin->setValue(col.hsvSaturationF() * 100.0);
    m_valSpin->setValue(col.valueF() * 100.0);

    m_swatchPreview->setStyleSheet(QStringLiteral(
        "border-radius: 4px; border: 1px solid #555; background-color: %1;").arg(col.name()));
    m_hexEdit->setText(col.name().toUpper());
    m_rgbLabel->setText(QStringLiteral("RGB: %1 / %2 / %3")
                            .arg(col.red()).arg(col.green()).arg(col.blue()));
}
