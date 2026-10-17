#include "petunia_color_picker.h"
#include "petunia_theme.h"

#include <QConicalGradient>
#include <QFormLayout>
#include <QGraphicsDropShadowEffect>
#include <QHBoxLayout>
#include <QLinearGradient>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPushButton>
#include <QVBoxLayout>
#include <QtMath>
#include <cmath>
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
    // Cromo do popover a partir dos tokens do tema (claro, escuro e alto contraste).
    const auto &t = PetuniaTheme::instance();
    card->setStyleSheet(QStringLiteral(
        "#ColorPickerCard { background-color: %1; border: 1px solid %2; border-radius: 10px; }\n"
        "QLabel { color: %3; font-weight: bold; }\n"
        "QLineEdit { background-color: %4; color: %3; border: 1px solid %2; border-radius: 4px; padding: 2px 6px; min-height: 20px; }\n"
        "QLineEdit:focus { border: %6px solid %7; }\n"
        "QPushButton { background-color: %4; color: %3; border: 1px solid %2; border-radius: 4px; padding: 3px 8px; font-weight: bold; }\n"
        "QPushButton:hover { background-color: %5; border-color: %7; }\n"
        "QPushButton:focus { border: %6px solid %7; }\n"
        "QPushButton#ConfirmBtn { background-color: %7; color: %8; border: none; }\n")
                            .arg(t.bgPanel().name(), t.borderStrong().name(), t.textPrimary().name(),
                                 t.bgSurface().name(), t.bgSurfaceHover().name())
                            .arg(t.focusRingWidth())
                            .arg(t.accentPrimary().name(), t.textActive().name()));

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
    m_titleLabel = new QLabel(tr("Seletor de cor"), card);
    headerRow->addWidget(m_titleLabel, 1);

    auto *closeBtn = new QPushButton(QStringLiteral("✕"), card);
    closeBtn->setFixedSize(18, 18);
    closeBtn->setStyleSheet(QStringLiteral("background: transparent; border: none; color: %1; font-weight: bold;").arg(t.textSecondary().name()));
    closeBtn->setAccessibleName(tr("Fechar"));
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

    m_hueSpin = new PetuniaNumberField(QStringLiteral("H"), card);
    m_hueSpin->setRange(0, 360);
    m_hueSpin->setSuffix(QStringLiteral("°"));
    m_hueSpin->setToolTip(QStringLiteral("Matiz (Hue)"));
    hsvRow->addWidget(m_hueSpin);

    m_satSpin = new PetuniaNumberField(QStringLiteral("S"), card);
    m_satSpin->setRange(0, 100);
    m_satSpin->setSuffix(QStringLiteral("%"));
    m_satSpin->setToolTip(QStringLiteral("Saturação"));
    hsvRow->addWidget(m_satSpin);

    m_valSpin = new PetuniaNumberField(QStringLiteral("V"), card);
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
    connect(m_hueSpin, &PetuniaNumberField::valueChanged, this, onHsvChanged);
    connect(m_satSpin, &PetuniaNumberField::valueChanged, this, onHsvChanged);
    connect(m_valSpin, &PetuniaNumberField::valueChanged, this, onHsvChanged);

    layout->addLayout(hsvRow);

    // Swatch + HEX
    auto *hexRow = new QHBoxLayout();
    hexRow->setSpacing(6);

    m_swatchPreview = new QFrame(card);
    m_swatchPreview->setFixedSize(24, 24);
    
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
    m_rgbLabel->setStyleSheet(QStringLiteral("color: %1;").arg(t.textSecondary().name()));
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
        btn->setStyleSheet(QStringLiteral("background-color: %1; border: 1px solid %2; border-radius: 3px;").arg(hex, t.borderStrong().name()));
        btn->setAccessibleName(tr("Cor %1").arg(hex.toUpper()));
        btn->setToolTip(hex.toUpper());
        connect(btn, &QPushButton::clicked, this, [this, hex]() {
            setColor(QColor(hex));
            emit colorChanged(color());
        });
        presetsRow->addWidget(btn);
    }
    layout->addLayout(presetsRow);

    // Actions
    auto *actionsRow = new QHBoxLayout();
    auto *resetBtn = new QPushButton(tr("Padrão"), card);
    connect(resetBtn, &QPushButton::clicked, this, [this]() {
        setColor(QColor(QStringLiteral("#e96a00")));
        emit colorChanged(color());
    });
    actionsRow->addWidget(resetBtn);

    auto *confirmBtn = new QPushButton(tr("Confirmar"), card);
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

    m_swatchPreview->setStyleSheet(QStringLiteral("border-radius: 4px; border: 1px solid %1; background-color: %2;")
                                       .arg(PetuniaTheme::instance().borderStrong().name(), col.name()));
    m_hexEdit->setText(col.name().toUpper());
    m_rgbLabel->setText(QStringLiteral("RGB: %1 / %2 / %3")
                            .arg(col.red()).arg(col.green()).arg(col.blue()));
}
