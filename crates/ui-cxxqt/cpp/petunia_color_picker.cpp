#include "petunia_color_picker.h"
#include "petunia_icons.h"
#include "petunia_theme.h"

#include <QConicalGradient>
#include <QFormLayout>
#include <QFrame>
#include <QGridLayout>
#include <QGraphicsDropShadowEffect>
#include <QHBoxLayout>
#include <QKeyEvent>
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
    const int side = PetuniaTheme::instance().scaled(140);
    setFixedSize(side, side);
    setCursor(Qt::CrossCursor);
    // Alternativa ao ponteiro: setas giram o matiz; Shift+setas movem S/V.
    setFocusPolicy(Qt::StrongFocus);
    setAccessibleName(tr("Roda de cores"));
    setAccessibleDescription(tr("Setas esquerda e direita mudam o matiz; Shift com setas muda saturação e brilho"));
}

float PetuniaColorWheel::outerRadius() const {
    return std::max(8.0f, float(std::min(width(), height())) * 0.5f - 5.0f);
}

QRectF PetuniaColorWheel::svRect() const {
    const float half = outerRadius() * 0.49f;
    const QPointF c = QRectF(rect()).center();
    return {c.x() - half, c.y() - half, half * 2.0f, half * 2.0f};
}

void PetuniaColorWheel::keyPressEvent(QKeyEvent *event) {
    const bool shift = event->modifiers() & Qt::ShiftModifier;
    const float step = event->modifiers() & Qt::ControlModifier ? 0.01f : 0.05f;
    bool handled = true;
    switch (event->key()) {
    case Qt::Key_Left:
        if (shift)
            m_saturation = qBound(0.0f, m_saturation - step, 1.0f);
        else
            m_hue = std::fmod(m_hue + 360.0f - step * 100.0f, 360.0f);
        break;
    case Qt::Key_Right:
        if (shift)
            m_saturation = qBound(0.0f, m_saturation + step, 1.0f);
        else
            m_hue = std::fmod(m_hue + step * 100.0f, 360.0f);
        break;
    case Qt::Key_Up:
        m_value = qBound(0.0f, m_value + step, 1.0f);
        break;
    case Qt::Key_Down:
        m_value = qBound(0.0f, m_value - step, 1.0f);
        break;
    default:
        handled = false;
    }
    if (!handled) {
        QWidget::keyPressEvent(event);
        return;
    }
    event->accept();
    emit colorChanged(color());
    update();
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

    const QPointF c = QRectF(rect()).center();
    const float rOut = outerRadius();
    const float rIn = rOut * 0.77f;

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
    const QRectF svRect = this->svRect();

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

    if (hasFocus()) {
        const auto &theme = PetuniaTheme::instance();
        painter.setPen(QPen(theme.borderFocus(), theme.focusRingWidth()));
        painter.setBrush(Qt::NoBrush);
        painter.drawEllipse(c, rOut + 3.0f, rOut + 3.0f);
    }
}

void PetuniaColorWheel::handleInteraction(const QPointF &pos) {
    const QPointF c = QRectF(rect()).center();
    const QPointF delta = pos - c;
    const float dist = std::hypot(delta.x(), delta.y());
    const float rOut = outerRadius() + 3.0f;
    const float rIn = outerRadius() * 0.77f - 4.0f;

    if (dist >= rIn && dist <= rOut) {
        float angle = qRadiansToDegrees(std::atan2(delta.y(), delta.x()));
        if (angle < 0.0f)
            angle += 360.0f;
        m_hue = angle;
        emit colorChanged(color());
        update();
    } else {
        const QRectF svRect = this->svRect();
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
    setAccessibleName(tr("Seletor de cor"));
    // Cromo, campos e botões herdam a folha global do tema (claro, escuro,
    // alto contraste e densidade); só as amostras de cor pintam a própria cor.
    const auto &t = PetuniaTheme::instance();
    const int target = t.minHitTarget();

    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(t.space(2), t.space(2), t.space(2), t.space(2));
    rootLayout->setSizeConstraint(QLayout::SetFixedSize);

    auto *card = new QFrame(this);
    card->setObjectName(QStringLiteral("ColorPickerCard"));
    card->setProperty("role", QStringLiteral("card"));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(20);
    shadow->setColor(QColor(0, 0, 0, 160));
    shadow->setOffset(0, 6);
    card->setGraphicsEffect(shadow);

    auto *layout = new QVBoxLayout(card);
    layout->setContentsMargins(t.space(3), t.space(2), t.space(3), t.space(3));
    layout->setSpacing(t.space(2));

    // Header
    auto *headerRow = new QHBoxLayout();
    m_titleLabel = new QLabel(tr("Seletor de cor"), card);
    m_titleLabel->setProperty("role", QStringLiteral("caption"));
    headerRow->addWidget(m_titleLabel, 1);

    auto *closeBtn = new QPushButton(PetuniaIcons::get(QStringLiteral("close")), QString(), card);
    closeBtn->setProperty("role", QStringLiteral("quiet"));
    closeBtn->setFixedSize(target, target);
    closeBtn->setToolTip(tr("Fechar (Esc)"));
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
    m_hueSpin->setToolTip(tr("Matiz"));
    m_hueSpin->setAccessibleName(tr("Matiz"));
    hsvRow->addWidget(m_hueSpin);

    m_satSpin = new PetuniaNumberField(QStringLiteral("S"), card);
    m_satSpin->setRange(0, 100);
    m_satSpin->setSuffix(QStringLiteral("%"));
    m_satSpin->setToolTip(tr("Saturação"));
    m_satSpin->setAccessibleName(tr("Saturação"));
    hsvRow->addWidget(m_satSpin);

    m_valSpin = new PetuniaNumberField(QStringLiteral("V"), card);
    m_valSpin->setRange(0, 100);
    m_valSpin->setSuffix(QStringLiteral("%"));
    m_valSpin->setToolTip(tr("Valor (brilho)"));
    m_valSpin->setAccessibleName(tr("Valor (brilho)"));
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
    m_swatchPreview->setFixedSize(target, target);
    m_swatchPreview->setAccessibleName(tr("Cor atual"));
    hexRow->addWidget(m_swatchPreview);

    auto *hexLabel = new QLabel(tr("&Hex"), card);
    hexRow->addWidget(hexLabel);

    m_hexEdit = new QLineEdit(QStringLiteral("#B58CFF"), card);
    m_hexEdit->setMaxLength(7);
    m_hexEdit->setAccessibleName(tr("Cor em hexadecimal"));
    hexLabel->setBuddy(m_hexEdit);
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
    m_rgbLabel->setProperty("role", QStringLiteral("muted"));
    layout->addWidget(m_rgbLabel);

    // Presets Row
    // Grade 4×2: alvos de toque ≥ 28 px sem alargar o popover.
    auto *presetsRow = new QGridLayout();
    presetsRow->setSpacing(t.space(1));
    const QStringList presets = {
        QStringLiteral("#e96a00"), QStringLiteral("#4fc3f7"), QStringLiteral("#b58cff"),
        QStringLiteral("#81c784"), QStringLiteral("#ffd54f"), QStringLiteral("#e53935"),
        QStringLiteral("#ffffff"), QStringLiteral("#121212")
    };
    for (int i = 0; i < presets.size(); ++i) {
        const QString hex = presets.value(i);
        auto *btn = new QPushButton(card);
        btn->setFixedSize(target, target);
        btn->setCursor(Qt::PointingHandCursor);
        btn->setStyleSheet(QStringLiteral("background-color: %1; border: 1px solid %2; border-radius: 3px;").arg(hex, t.borderStrong().name()));
        btn->setAccessibleName(tr("Cor %1").arg(hex.toUpper()));
        btn->setToolTip(hex.toUpper());
        connect(btn, &QPushButton::clicked, this, [this, hex]() {
            setColor(QColor(hex));
            emit colorChanged(color());
        });
        presetsRow->addWidget(btn, i / 4, i % 4);
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
    confirmBtn->setProperty("role", QStringLiteral("primary"));
    confirmBtn->setDefault(true);
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
