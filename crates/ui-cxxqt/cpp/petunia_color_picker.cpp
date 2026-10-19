#include "petunia_color_picker.h"
#include "petunia_dialog_chrome.h"
#include "petunia_editor_api.h"
#include "petunia_icons.h"
#include "petunia_theme.h"

#include <QComboBox>
#include <QConicalGradient>
#include <QContextMenuEvent>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QInputDialog>
#include <QJsonObject>
#include <QKeyEvent>
#include <QLinearGradient>
#include <QMenu>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPushButton>
#include <QRegularExpression>
#include <QResizeEvent>
#include <QSlider>
#include <QStackedWidget>
#include <QToolButton>
#include <QVBoxLayout>
#include <QtMath>
#include <algorithm>
#include <cmath>

namespace {
/// `#RRGGBB` ou `#RRGGBBAA` (ordem CSS; o QColor lê `#AARRGGBB`).
bool parseHex(const QString &text, QColor *out) {
    static const QRegularExpression pattern(QStringLiteral("^#?([0-9a-fA-F]{6})([0-9a-fA-F]{2})?$"));
    const auto match = pattern.match(text.trimmed());
    if (!match.hasMatch())
        return false;
    bool ok = false;
    const uint rgb = match.captured(1).toUInt(&ok, 16);
    if (!ok)
        return false;
    int alpha = 255;
    if (!match.captured(2).isEmpty())
        alpha = int(match.captured(2).toUInt(&ok, 16));
    *out = QColor(int((rgb >> 16) & 0xFF), int((rgb >> 8) & 0xFF), int(rgb & 0xFF), alpha);
    return true;
}

QString formatHex(const QColor &color, bool withAlpha) {
    QString text = QStringLiteral("#%1%2%3")
                       .arg(color.red(), 2, 16, QLatin1Char('0'))
                       .arg(color.green(), 2, 16, QLatin1Char('0'))
                       .arg(color.blue(), 2, 16, QLatin1Char('0'));
    if (withAlpha && color.alpha() < 255)
        text += QStringLiteral("%1").arg(color.alpha(), 2, 16, QLatin1Char('0'));
    return text.toUpper();
}

/// Xadrez atrás de cores translúcidas (mostra o alfa).
void paintChecker(QPainter &painter, const QRectF &rect, qreal cell = 5.0) {
    painter.save();
    painter.setClipRect(rect);
    painter.fillRect(rect, QColor(205, 205, 205));
    for (qreal y = rect.top(); y < rect.bottom(); y += cell)
        for (qreal x = rect.left(); x < rect.right(); x += cell)
            if ((int((x - rect.left()) / cell) + int((y - rect.top()) / cell)) % 2 == 0)
                painter.fillRect(QRectF(x, y, cell, cell), QColor(150, 150, 150));
    painter.restore();
}
} // namespace

// =========================================================================
// PetuniaColorWheel
// =========================================================================

PetuniaColorWheel::PetuniaColorWheel(QWidget *parent) : QWidget(parent) {
    setDiameter(140);
    setCursor(Qt::CrossCursor);
    // Alternativa ao ponteiro: setas giram o matiz; Shift+setas movem S/V.
    setFocusPolicy(Qt::StrongFocus);
    setAccessibleName(tr("Roda de cores"));
    setAccessibleDescription(tr("Setas esquerda e direita mudam o matiz; Shift com setas muda saturação e brilho"));
}

void PetuniaColorWheel::setDiameter(int logicalPx) {
    const int side = PetuniaTheme::instance().scaled(std::max(80, logicalPx));
    if (size() == QSize(side, side) && minimumSize() == QSize(side, side))
        return; // sem mudança: evita relayout em cascata a cada resize do painel
    setFixedSize(side, side);
    update();
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
        m_hue = h * 360.0f; // cinza não tem matiz: mantém o anterior
    m_saturation = color.hsvSaturationF();
    m_value = color.valueF();
    update();
}

void PetuniaColorWheel::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    const auto &theme = PetuniaTheme::instance();

    const QPointF c = QRectF(rect()).center();
    const float rOut = outerRadius();
    const float rIn = rOut * 0.77f;

    QConicalGradient conic(c, 0.0);
    for (int deg = 0; deg <= 360; deg += 30)
        conic.setColorAt(deg / 360.0, QColor::fromHsvF(deg / 360.0f, 1.0f, 1.0f));
    QPainterPath ringPath;
    ringPath.addEllipse(c, rOut, rOut);
    QPainterPath holePath;
    holePath.addEllipse(c, rIn, rIn);
    const QPainterPath ring = ringPath.subtracted(holePath);
    painter.fillPath(ring, conic);
    painter.setPen(theme.borderStrong());
    painter.drawPath(ring);

    // Marcador do matiz no anel (o Qt cresce o ângulo no sentido anti-horário).
    const float rad = qDegreesToRadians(-m_hue);
    const float rMid = (rOut + rIn) * 0.5f;
    const QPointF markerPos(c.x() + rMid * std::cos(rad), c.y() + rMid * std::sin(rad));
    painter.setPen(QPen(Qt::white, 2));
    painter.setBrush(QColor::fromHsvF(m_hue / 360.0f, 1.0f, 1.0f));
    painter.drawEllipse(markerPos, 5, 5);

    const QRectF sv = svRect();
    QLinearGradient satGrad(sv.left(), 0, sv.right(), 0);
    satGrad.setColorAt(0, Qt::white);
    satGrad.setColorAt(1, QColor::fromHsvF(m_hue / 360.0f, 1.0f, 1.0f));
    painter.fillRect(sv, satGrad);
    QLinearGradient valGrad(0, sv.top(), 0, sv.bottom());
    valGrad.setColorAt(0, QColor(0, 0, 0, 0));
    valGrad.setColorAt(1, Qt::black);
    painter.fillRect(sv, valGrad);
    painter.setPen(theme.borderStrong());
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(sv);

    const QPointF svPos(sv.left() + m_saturation * sv.width(), sv.bottom() - m_value * sv.height());
    painter.setPen(QPen(m_value > 0.5 ? Qt::black : Qt::white, 2));
    painter.setBrush(color());
    painter.drawEllipse(svPos, 5, 5);

    if (hasFocus()) {
        painter.setPen(QPen(theme.borderFocus(), theme.focusRingWidth()));
        painter.setBrush(Qt::NoBrush);
        painter.drawEllipse(c, rOut + 3.0f, rOut + 3.0f);
    }
}

void PetuniaColorWheel::handleInteraction(const QPointF &pos) {
    const QPointF c = QRectF(rect()).center();
    const QPointF delta = pos - c;
    if (m_target == Target::Ring) {
        float angle = qRadiansToDegrees(std::atan2(-delta.y(), delta.x()));
        if (angle < 0.0f)
            angle += 360.0f;
        m_hue = angle;
    } else if (m_target == Target::Square) {
        const QRectF sv = svRect();
        m_saturation = qBound(0.0f, float((pos.x() - sv.left()) / sv.width()), 1.0f);
        m_value = qBound(0.0f, float((sv.bottom() - pos.y()) / sv.height()), 1.0f);
    } else {
        return;
    }
    emit colorChanged(color());
    update();
}

void PetuniaColorWheel::mousePressEvent(QMouseEvent *event) {
    if (event->button() != Qt::LeftButton)
        return;
    setFocus(Qt::MouseFocusReason);
    // O alvo é decidido no clique: arrastar do anel para dentro não vira S/V.
    const QPointF delta = event->position() - QRectF(rect()).center();
    const float dist = std::hypot(delta.x(), delta.y());
    if (dist >= outerRadius() * 0.77f - 4.0f && dist <= outerRadius() + 3.0f)
        m_target = Target::Ring;
    else if (svRect().adjusted(-6, -6, 6, 6).contains(event->position()))
        m_target = Target::Square;
    else
        m_target = Target::None;
    handleInteraction(event->position());
}

void PetuniaColorWheel::mouseMoveEvent(QMouseEvent *event) {
    if (event->buttons() & Qt::LeftButton)
        handleInteraction(event->position());
}

// =========================================================================
// PetuniaColorBox
// =========================================================================

PetuniaColorBox::PetuniaColorBox(QWidget *parent) : QWidget(parent) {
    const int side = PetuniaTheme::instance().scaled(210);
    setFixedSize(side, side);
    setCursor(Qt::CrossCursor);
    setFocusPolicy(Qt::StrongFocus);
    setAccessibleName(tr("Caixa de cor"));
    setAccessibleDescription(tr("Setas mudam saturação e brilho; Page Up e Page Down mudam o matiz"));
}

QRectF PetuniaColorBox::squareRect() const {
    const qreal strip = PetuniaTheme::instance().scaled(18);
    return QRectF(3, 3, width() - strip - 14, height() - 6);
}

QRectF PetuniaColorBox::stripRect() const {
    const qreal strip = PetuniaTheme::instance().scaled(18);
    return QRectF(width() - strip - 3, 3, strip, height() - 6);
}

QColor PetuniaColorBox::color() const {
    return QColor::fromHsvF(m_hue / 360.0f, m_saturation, m_value);
}

void PetuniaColorBox::setColor(const QColor &color) {
    const float h = color.hsvHueF();
    if (h >= 0.0f)
        m_hue = h * 360.0f;
    m_saturation = color.hsvSaturationF();
    m_value = color.valueF();
    update();
}

void PetuniaColorBox::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    const auto &theme = PetuniaTheme::instance();
    const QRectF sq = squareRect();
    QLinearGradient sat(sq.left(), 0, sq.right(), 0);
    sat.setColorAt(0, Qt::white);
    sat.setColorAt(1, QColor::fromHsvF(m_hue / 360.0f, 1.0f, 1.0f));
    painter.fillRect(sq, sat);
    QLinearGradient val(0, sq.top(), 0, sq.bottom());
    val.setColorAt(0, QColor(0, 0, 0, 0));
    val.setColorAt(1, Qt::black);
    painter.fillRect(sq, val);
    painter.setPen(theme.borderStrong());
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(sq);

    const QRectF st = stripRect();
    QLinearGradient hue(0, st.top(), 0, st.bottom());
    for (int deg = 0; deg <= 360; deg += 30)
        hue.setColorAt(deg / 360.0, QColor::fromHsvF(deg / 360.0f, 1.0f, 1.0f));
    painter.fillRect(st, hue);
    painter.drawRect(st);
    const qreal y = st.top() + (m_hue / 360.0) * st.height();
    painter.setPen(QPen(Qt::white, 2));
    painter.drawRect(QRectF(st.left() - 2, y - 3, st.width() + 4, 6));

    const QPointF at(sq.left() + m_saturation * sq.width(), sq.bottom() - m_value * sq.height());
    painter.setPen(QPen(m_value > 0.5 ? Qt::black : Qt::white, 2));
    painter.setBrush(color());
    painter.drawEllipse(at, 5, 5);
    if (hasFocus()) {
        painter.setPen(QPen(theme.borderFocus(), theme.focusRingWidth()));
        painter.setBrush(Qt::NoBrush);
        painter.drawRect(QRectF(rect()).adjusted(1, 1, -1, -1));
    }
}

void PetuniaColorBox::handleInteraction(const QPointF &pos) {
    if (m_target == Target::Square) {
        const QRectF sq = squareRect();
        m_saturation = qBound(0.0f, float((pos.x() - sq.left()) / sq.width()), 1.0f);
        m_value = qBound(0.0f, float((sq.bottom() - pos.y()) / sq.height()), 1.0f);
    } else if (m_target == Target::Strip) {
        const QRectF st = stripRect();
        m_hue = qBound(0.0f, float((pos.y() - st.top()) / st.height()), 1.0f) * 359.9f;
    } else {
        return;
    }
    emit colorChanged(color());
    update();
}

void PetuniaColorBox::mousePressEvent(QMouseEvent *event) {
    if (event->button() != Qt::LeftButton)
        return;
    setFocus(Qt::MouseFocusReason);
    if (stripRect().adjusted(-4, 0, 4, 0).contains(event->position()))
        m_target = Target::Strip;
    else if (squareRect().adjusted(-6, -6, 6, 6).contains(event->position()))
        m_target = Target::Square;
    else
        m_target = Target::None;
    handleInteraction(event->position());
}

void PetuniaColorBox::mouseMoveEvent(QMouseEvent *event) {
    if (event->buttons() & Qt::LeftButton)
        handleInteraction(event->position());
}

void PetuniaColorBox::keyPressEvent(QKeyEvent *event) {
    const float step = event->modifiers() & Qt::ShiftModifier ? 0.1f : 0.02f;
    switch (event->key()) {
    case Qt::Key_Left: m_saturation = qBound(0.0f, m_saturation - step, 1.0f); break;
    case Qt::Key_Right: m_saturation = qBound(0.0f, m_saturation + step, 1.0f); break;
    case Qt::Key_Up: m_value = qBound(0.0f, m_value + step, 1.0f); break;
    case Qt::Key_Down: m_value = qBound(0.0f, m_value - step, 1.0f); break;
    case Qt::Key_PageUp: m_hue = std::fmod(m_hue + 360.0f - step * 100.0f, 360.0f); break;
    case Qt::Key_PageDown: m_hue = std::fmod(m_hue + step * 100.0f, 360.0f); break;
    default:
        QWidget::keyPressEvent(event);
        return;
    }
    event->accept();
    emit colorChanged(color());
    update();
}

// =========================================================================
// PetuniaSwatchGrid
// =========================================================================

namespace {
constexpr int kSwatch = 18;
constexpr int kSwatchGap = 4;
} // namespace

PetuniaSwatchGrid::PetuniaSwatchGrid(QWidget *parent) : QWidget(parent) {
    setFocusPolicy(Qt::StrongFocus);
    setAccessibleName(tr("Cores da paleta"));
    setAccessibleDescription(tr("Clique escolhe a cor; botão direito ou Menu edita ou apaga; setas percorrem"));
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
}

void PetuniaSwatchGrid::setColors(const QVector<QColor> &colors) {
    m_colors = colors;
    if (m_current >= m_colors.size())
        m_current = int(m_colors.size()) - 1;
    updateGeometry();
    update();
}

int PetuniaSwatchGrid::columns() const {
    const int side = PetuniaTheme::instance().scaled(kSwatch);
    return std::max(1, (width() + kSwatchGap) / (side + kSwatchGap));
}

QRect PetuniaSwatchGrid::swatchRect(int index) const {
    const int side = PetuniaTheme::instance().scaled(kSwatch);
    const int cols = columns();
    return QRect((index % cols) * (side + kSwatchGap), (index / cols) * (side + kSwatchGap), side, side);
}

int PetuniaSwatchGrid::heightForWidth(int width) const {
    const int side = PetuniaTheme::instance().scaled(kSwatch);
    const int cols = std::max(1, (width + kSwatchGap) / (side + kSwatchGap));
    const int rows = std::max(1, int((m_colors.size() + cols - 1) / cols));
    return rows * (side + kSwatchGap);
}

QSize PetuniaSwatchGrid::sizeHint() const {
    const int side = PetuniaTheme::instance().scaled(kSwatch);
    return QSize(8 * (side + kSwatchGap), heightForWidth(8 * (side + kSwatchGap)));
}

int PetuniaSwatchGrid::indexAt(const QPoint &pos) const {
    for (int i = 0; i < m_colors.size(); ++i)
        if (swatchRect(i).contains(pos))
            return i;
    return -1;
}

void PetuniaSwatchGrid::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    const auto &theme = PetuniaTheme::instance();
    if (m_colors.isEmpty()) {
        painter.setPen(theme.textMuted());
        painter.drawText(rect(), Qt::AlignLeft | Qt::AlignVCenter, tr("Paleta vazia: use + para guardar a cor atual"));
        return;
    }
    for (int i = 0; i < m_colors.size(); ++i) {
        const QRectF r = swatchRect(i);
        if (m_colors[i].alpha() < 255)
            paintChecker(painter, r, 4.5);
        painter.setPen(QPen(theme.borderStrong(), 1));
        painter.setBrush(m_colors[i]);
        painter.drawRoundedRect(r.adjusted(0.5, 0.5, -0.5, -0.5), 3, 3);
        if (i == m_current && hasFocus()) {
            painter.setPen(QPen(theme.borderFocus(), theme.focusRingWidth()));
            painter.setBrush(Qt::NoBrush);
            painter.drawRoundedRect(r.adjusted(-2, -2, 2, 2), 4, 4);
        }
    }
}

void PetuniaSwatchGrid::mousePressEvent(QMouseEvent *event) {
    const int index = indexAt(event->pos());
    if (index < 0)
        return;
    m_current = index;
    setFocus(Qt::MouseFocusReason);
    update();
    if (event->button() == Qt::LeftButton)
        emit picked(index);
}

void PetuniaSwatchGrid::contextMenuEvent(QContextMenuEvent *event) {
    const bool keyboard = event->reason() == QContextMenuEvent::Keyboard;
    const int index = keyboard ? m_current : indexAt(event->pos());
    if (index < 0)
        return;
    m_current = index;
    const QPoint at = keyboard ? mapToGlobal(swatchRect(index).center()) : event->globalPos();
    showMenu(index, at);
    event->accept();
}

void PetuniaSwatchGrid::showMenu(int index, const QPoint &globalPos) {
    QMenu menu(this);
    auto *edit = menu.addAction(tr("Editar cor…"));
    auto *remove = menu.addAction(tr("Apagar cor"));
    auto *picked = menu.exec(globalPos);
    if (picked == edit)
        emit editRequested(index);
    else if (picked == remove)
        emit removeRequested(index);
}

void PetuniaSwatchGrid::keyPressEvent(QKeyEvent *event) {
    if (m_colors.isEmpty()) {
        QWidget::keyPressEvent(event);
        return;
    }
    const int cols = columns();
    int next = std::max(0, m_current);
    switch (event->key()) {
    case Qt::Key_Left: next -= 1; break;
    case Qt::Key_Right: next += 1; break;
    case Qt::Key_Up: next -= cols; break;
    case Qt::Key_Down: next += cols; break;
    case Qt::Key_Return:
    case Qt::Key_Enter:
    case Qt::Key_Space:
        if (m_current >= 0)
            emit picked(m_current);
        event->accept();
        return;
    case Qt::Key_Delete:
        if (m_current >= 0)
            emit removeRequested(m_current);
        event->accept();
        return;
    default:
        QWidget::keyPressEvent(event);
        return;
    }
    m_current = std::clamp(next, 0, int(m_colors.size()) - 1);
    update();
    event->accept();
}

// =========================================================================
// PetuniaColorPanel
// =========================================================================

PetuniaColorPanel::PetuniaColorPanel(QWidget *parent) : QWidget(parent) {
    setAccessibleName(tr("Seletor de cor"));
    const auto &t = PetuniaTheme::instance();
    auto *root = new QBoxLayout(QBoxLayout::LeftToRight, this);
    m_root = root;
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(t.space(3));

    // ── Esquerda: área visual (roda ou caixa) ──────────────────────────────
    auto *left = new QVBoxLayout();
    left->setSpacing(t.space(1));
    m_modeChooser = new QComboBox(this);
    m_modeChooser->addItem(tr("Roda"));
    m_modeChooser->addItem(tr("Caixa"));
    m_modeChooser->setAccessibleName(tr("Forma de escolher a cor"));
    left->addWidget(m_modeChooser);
    m_visual = new QStackedWidget(this);
    m_wheel = new PetuniaColorWheel(m_visual);
    m_wheel->setDiameter(210);
    m_box = new PetuniaColorBox(m_visual);
    m_visual->addWidget(m_wheel);
    m_visual->addWidget(m_box);
    left->addWidget(m_visual);
    left->addStretch(1);
    root->addLayout(left);
    connect(m_modeChooser, &QComboBox::currentIndexChanged, this, [this](int index) { setMode(index); });
    const auto fromVisual = [this](const QColor &picked) {
        if (m_updating)
            return;
        QColor next = picked;
        next.setAlphaF(m_color.alphaF());
        applyColor(next, true);
    };
    connect(m_wheel, &PetuniaColorWheel::colorChanged, this, fromVisual);
    connect(m_box, &PetuniaColorBox::colorChanged, this, fromVisual);

    // ── Direita: canais, hex, paletas ──────────────────────────────────────
    auto *right = new QVBoxLayout();
    right->setSpacing(t.space(1));
    m_modelChooser = new QComboBox(this);
    m_modelChooser->addItems({QStringLiteral("RGB"), QStringLiteral("HSL"), QStringLiteral("HSV")});
    m_modelChooser->setAccessibleName(tr("Modelo de cor"));
    auto *modelRow = new QHBoxLayout();
    modelRow->setSpacing(t.space(1));
    modelRow->addWidget(m_modelChooser, 1);
    // Modo compacto (B14): os canais ficam recolhidos atrás deste botão.
    m_channelsToggle = new QToolButton(this);
    m_channelsToggle->setObjectName(QStringLiteral("color-channels-toggle"));
    m_channelsToggle->setCheckable(true);
    m_channelsToggle->setText(tr("Canais"));
    m_channelsToggle->setToolTip(tr("Mostrar os canais numéricos"));
    m_channelsToggle->setAccessibleName(tr("Mostrar os canais numéricos"));
    m_channelsToggle->setVisible(false);
    connect(m_channelsToggle, &QToolButton::toggled, this, [this] { applyCompactLayout(); });
    modelRow->addWidget(m_channelsToggle);
    right->addLayout(modelRow);
    connect(m_modelChooser, &QComboBox::currentIndexChanged, this, [this](int index) {
        m_model = index == 1 ? Model::Hsl : index == 2 ? Model::Hsv : Model::Rgb;
        refreshChannels();
    });

    auto *channels = new QGridLayout();
    channels->setHorizontalSpacing(t.space(1));
    channels->setVerticalSpacing(t.space(1));
    for (int i = 0; i < 4; ++i) {
        m_channelLabels[i] = new QLabel(this);
        m_channelSliders[i] = new QSlider(Qt::Horizontal, this);
        m_channelFields[i] = new PetuniaNumberField(this);
        m_channelFields[i]->setSingleStep(1);
        m_channelFields[i]->setMaxDecimals(0);
        m_channelLabels[i]->setBuddy(m_channelFields[i]); // arrastar o nome ajusta o canal
        channels->addWidget(m_channelLabels[i], i, 0);
        channels->addWidget(m_channelSliders[i], i, 1);
        channels->addWidget(m_channelFields[i], i, 2);
        connect(m_channelSliders[i], &QSlider::valueChanged, this, [this, i](int value) {
            if (m_updating)
                return;
            m_updating = true;
            m_channelFields[i]->setValue(value);
            m_updating = false;
            channelEdited();
        });
        connect(m_channelFields[i], &PetuniaNumberField::valueChanged, this, [this, i](double value) {
            if (m_updating)
                return;
            m_updating = true;
            m_channelSliders[i]->setValue(int(std::lround(value)));
            m_updating = false;
            channelEdited();
        });
    }
    channels->setColumnStretch(1, 1);
    right->addLayout(channels);

    auto *hexRow = new QHBoxLayout();
    hexRow->setSpacing(t.space(1));
    m_preview = new QFrame(this);
    m_preview->setFixedSize(t.minHitTarget(), t.minHitTarget());
    m_preview->setAccessibleName(tr("Cor atual"));
    hexRow->addWidget(m_preview);
    auto *hexLabel = new QLabel(tr("&Hex"), this);
    hexRow->addWidget(hexLabel);
    m_hex = new QLineEdit(this);
    m_hex->setMaxLength(9);
    m_hex->setAccessibleName(tr("Cor em hexadecimal"));
    m_hex->setToolTip(tr("#RRGGBB ou #RRGGBBAA"));
    hexLabel->setBuddy(m_hex);
    hexRow->addWidget(m_hex, 1);
    connect(m_hex, &QLineEdit::textEdited, this, [this](const QString &text) {
        QColor parsed;
        if (!m_updating && parseHex(text, &parsed)) {
            if (!m_alphaEnabled)
                parsed.setAlpha(255);
            m_updating = true; // não reescreve o texto que o usuário digita
            m_color = parsed;
            m_wheel->setColor(parsed);
            m_box->setColor(parsed);
            refreshChannels();
            m_updating = false;
            m_preview->setStyleSheet(QStringLiteral("border-radius: 4px; border: 1px solid %1; background-color: %2;")
                                         .arg(PetuniaTheme::instance().borderStrong().name(), parsed.name(QColor::HexArgb)));
            emit colorChanged(m_color);
        }
    });
    m_addToPalette = new QToolButton(this);
    m_addToPalette->setIcon(PetuniaIcons::get(QStringLiteral("plus")));
    m_addToPalette->setToolTip(tr("Adicionar a cor à paleta"));
    m_addToPalette->setAccessibleName(tr("Adicionar a cor à paleta"));
    connect(m_addToPalette, &QToolButton::clicked, this, [this] { addCurrentColorToPalette(); });
    hexRow->addWidget(m_addToPalette);
    right->addLayout(hexRow);

    auto *paletteRow = new QHBoxLayout();
    paletteRow->setSpacing(t.space(1));
    m_paletteChooser = new QComboBox(this);
    m_paletteChooser->setAccessibleName(tr("Paleta"));
    paletteRow->addWidget(m_paletteChooser, 1);
    const auto tool = [this](const QString &icon, const QString &tip) {
        auto *button = new QToolButton(this);
        button->setIcon(PetuniaIcons::get(icon));
        button->setToolTip(tip);
        button->setAccessibleName(tip);
        return button;
    };
    m_newPalette = tool(QStringLiteral("plus"), tr("Nova paleta"));
    m_renamePalette = tool(QStringLiteral("settings"), tr("Renomear paleta"));
    m_deletePalette = tool(QStringLiteral("close"), tr("Apagar paleta"));
    paletteRow->addWidget(m_newPalette);
    paletteRow->addWidget(m_renamePalette);
    paletteRow->addWidget(m_deletePalette);
    right->addLayout(paletteRow);
    connect(m_paletteChooser, &QComboBox::currentIndexChanged, this, [this](int index) {
        if (m_updating)
            return;
        m_paletteId = m_paletteChooser->itemData(index).toString();
        rebuildSwatches();
    });
    connect(m_newPalette, &QToolButton::clicked, this, [this] {
        bool ok = false;
        const QString name = QInputDialog::getText(this, tr("Nova paleta"), tr("Nome da paleta"), QLineEdit::Normal,
                                                   tr("Paleta %1").arg(m_palettes.size()), &ok);
        if (ok)
            createPalette(name);
    });
    connect(m_renamePalette, &QToolButton::clicked, this, [this] {
        bool ok = false;
        const QString name = QInputDialog::getText(this, tr("Renomear paleta"), tr("Nome da paleta"), QLineEdit::Normal,
                                                   m_paletteChooser->currentText(), &ok);
        if (ok)
            renamePalette(name);
    });
    connect(m_deletePalette, &QToolButton::clicked, this, [this] { deleteCurrentPalette(); });

    m_swatches = new PetuniaSwatchGrid(this);
    right->addWidget(m_swatches);
    connect(m_swatches, &PetuniaSwatchGrid::picked, this, [this](int index) {
        QColor picked = m_swatches->colorAt(index);
        if (!m_alphaEnabled)
            picked.setAlpha(255);
        applyColor(picked, true);
    });
    connect(m_swatches, &PetuniaSwatchGrid::removeRequested, this, [this](int index) {
        QJsonObject args{{"op", "remove"}, {"index", index}};
        args.insert("id", m_paletteId.isEmpty() ? QJsonValue() : QJsonValue(m_paletteId));
        requestPalette(args);
    });
    connect(m_swatches, &PetuniaSwatchGrid::editRequested, this, [this](int index) {
        PetuniaColorPickerPopover editor(this);
        editor.setTitle(tr("Editar cor da paleta"));
        editor.setAlphaEnabled(!m_paletteId.isEmpty()); // a paleta do projeto é RGB
        editor.setColor(m_swatches->colorAt(index));
        if (editor.exec() != QDialog::Accepted)
            return;
        const QColor c = editor.color();
        QJsonObject args{{"op", "set"}, {"index", index}, {"color", QJsonArray{c.redF(), c.greenF(), c.blueF(), c.alphaF()}}};
        args.insert("id", m_paletteId.isEmpty() ? QJsonValue() : QJsonValue(m_paletteId));
        requestPalette(args);
    });
    right->addStretch(1);
    root->addLayout(right, 1);

    applyColor(m_color, false);
    refreshPalettes();
}

void PetuniaColorPanel::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);
    const int threshold = PetuniaTheme::instance().scaled(420);
    const bool stacked = width() < threshold;
    if (stacked != m_stacked) {
        m_stacked = stacked;
        m_root->setDirection(stacked ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
    }
    // Empilhado, a roda acompanha a largura (sem passar de 210 px nem ficar ilegível);
    // compacta, ela não passa de 150 px para o painel continuar com espaço.
    const int scale = std::max(1, PetuniaTheme::instance().scaled(100));
    const int ceiling = m_compact ? 150 : 210;
    const int floor = m_compact ? 120 : 140;
    const int logical = stacked ? std::clamp(width() * 100 / scale - 16, floor, ceiling) : ceiling;
    m_wheel->setDiameter(logical);
    // A caixa acompanha a roda: a pilha reserva o maior filho, e uma caixa fixa
    // de 210 px deixava um vão embaixo da roda compacta (B14).
    if (m_box->size() != m_wheel->size())
        m_box->setFixedSize(m_wheel->size());
}

void PetuniaColorPanel::setCompact(bool compact) {
    if (m_compact == compact)
        return;
    m_compact = compact;
    m_channelsToggle->setVisible(compact);
    applyCompactLayout();
    QResizeEvent event(size(), size());
    resizeEvent(&event);
}

void PetuniaColorPanel::applyCompactLayout() {
    const bool channels = !m_compact || m_channelsToggle->isChecked();
    for (int i = 0; i < 4; ++i) {
        // O alfa é a opacidade do pincel (A8): fica à vista mesmo no compacto.
        const bool alphaRow = i == 3;
        const bool show = alphaRow ? m_alphaEnabled : channels;
        m_channelLabels[i]->setVisible(show);
        m_channelSliders[i]->setVisible(show);
        m_channelFields[i]->setVisible(show);
    }
}

void PetuniaColorPanel::setMode(int index) {
    const int clamped = std::clamp(index, 0, 1);
    if (m_modeChooser->currentIndex() != clamped)
        m_modeChooser->setCurrentIndex(clamped);
    m_visual->setCurrentIndex(clamped);
}

int PetuniaColorPanel::mode() const {
    return m_visual->currentIndex();
}

void PetuniaColorPanel::setColor(const QColor &color) {
    applyColor(color, false);
}

void PetuniaColorPanel::setAlphaEnabled(bool enabled) {
    m_alphaEnabled = enabled;
    applyCompactLayout();
    if (!enabled) {
        QColor opaque = m_color;
        opaque.setAlpha(255);
        applyColor(opaque, false);
    }
}

void PetuniaColorPanel::applyColor(const QColor &color, bool emitChange) {
    QColor next = color.isValid() ? color : m_color;
    if (!m_alphaEnabled)
        next.setAlpha(255);
    const bool wasUpdating = m_updating;
    m_updating = true;
    m_color = next;
    m_wheel->setColor(next);
    m_box->setColor(next);
    m_hex->setText(formatHex(next, m_alphaEnabled));
    m_preview->setStyleSheet(QStringLiteral("border-radius: 4px; border: 1px solid %1; background-color: %2;")
                                 .arg(PetuniaTheme::instance().borderStrong().name(), next.name(QColor::HexArgb)));
    refreshChannels();
    m_updating = wasUpdating;
    if (emitChange)
        emit colorChanged(m_color);
}

void PetuniaColorPanel::refreshChannels() {
    const bool wasUpdating = m_updating;
    m_updating = true;
    struct Channel { QString name; QString tip; int max; double value; QString suffix; };
    Channel channels[4];
    const QColor &c = m_color;
    switch (m_model) {
    case Model::Rgb:
        channels[0] = {QStringLiteral("R"), tr("Vermelho"), 255, double(c.red()), QString()};
        channels[1] = {QStringLiteral("G"), tr("Verde"), 255, double(c.green()), QString()};
        channels[2] = {QStringLiteral("B"), tr("Azul"), 255, double(c.blue()), QString()};
        break;
    case Model::Hsl: {
        const float h = c.hslHueF();
        channels[0] = {QStringLiteral("H"), tr("Matiz"), 360, h >= 0 ? h * 360.0 : m_wheel->color().hsvHueF() * 360.0, QStringLiteral("°")};
        channels[1] = {QStringLiteral("S"), tr("Saturação"), 100, c.hslSaturationF() * 100.0, QStringLiteral("%")};
        channels[2] = {QStringLiteral("L"), tr("Luminosidade"), 100, c.lightnessF() * 100.0, QStringLiteral("%")};
        break;
    }
    case Model::Hsv: {
        const float h = c.hsvHueF();
        channels[0] = {QStringLiteral("H"), tr("Matiz"), 360, h >= 0 ? h * 360.0 : m_wheel->color().hsvHueF() * 360.0, QStringLiteral("°")};
        channels[1] = {QStringLiteral("S"), tr("Saturação"), 100, c.hsvSaturationF() * 100.0, QStringLiteral("%")};
        channels[2] = {QStringLiteral("V"), tr("Valor (brilho)"), 100, c.valueF() * 100.0, QStringLiteral("%")};
        break;
    }
    }
    channels[3] = {QStringLiteral("A"), tr("Opacidade"), 100, c.alphaF() * 100.0, QStringLiteral("%")};
    for (int i = 0; i < 4; ++i) {
        m_channelLabels[i]->setText(channels[i].name);
        m_channelLabels[i]->setToolTip(channels[i].tip);
        m_channelSliders[i]->setRange(0, channels[i].max);
        m_channelSliders[i]->setAccessibleName(channels[i].tip);
        m_channelSliders[i]->setValue(int(std::lround(channels[i].value)));
        m_channelFields[i]->setRange(0, channels[i].max);
        m_channelFields[i]->setSuffix(channels[i].suffix);
        m_channelFields[i]->setLabel(channels[i].tip);
        m_channelFields[i]->setValue(std::round(channels[i].value));
    }
    m_updating = wasUpdating;
}

void PetuniaColorPanel::channelEdited() {
    const double v0 = m_channelFields[0]->value(), v1 = m_channelFields[1]->value(), v2 = m_channelFields[2]->value();
    const double alpha = m_alphaEnabled ? m_channelFields[3]->value() / 100.0 : 1.0;
    QColor next;
    switch (m_model) {
    case Model::Rgb:
        next = QColor(int(v0), int(v1), int(v2));
        break;
    case Model::Hsl:
        next = QColor::fromHslF(float(std::fmod(v0, 360.0) / 360.0), float(v1 / 100.0), float(v2 / 100.0));
        break;
    case Model::Hsv:
        next = QColor::fromHsvF(float(std::fmod(v0, 360.0) / 360.0), float(v1 / 100.0), float(v2 / 100.0));
        break;
    }
    next.setAlphaF(float(std::clamp(alpha, 0.0, 1.0)));
    // O canal que está sendo arrastado não pode ser reescrito pela conversão.
    m_updating = true;
    m_color = next;
    m_wheel->setColor(next);
    m_box->setColor(next);
    m_hex->setText(formatHex(next, m_alphaEnabled));
    m_preview->setStyleSheet(QStringLiteral("border-radius: 4px; border: 1px solid %1; background-color: %2;")
                                 .arg(PetuniaTheme::instance().borderStrong().name(), next.name(QColor::HexArgb)));
    m_updating = false;
    emit colorChanged(m_color);
}

QString PetuniaColorPanel::currentPaletteId() const {
    return m_paletteId;
}

void PetuniaColorPanel::refreshPalettes() {
    syncPalettes(petuniaRequest(QStringLiteral("snapshot"))[QStringLiteral("palettes")].toArray());
}

void PetuniaColorPanel::syncPalettes(const QJsonArray &palettes) {
    m_palettes = palettes;
    const bool wasUpdating = m_updating;
    m_updating = true;
    m_paletteChooser->clear();
    int keep = 0;
    for (int i = 0; i < m_palettes.size(); ++i) {
        const auto row = m_palettes[i].toObject();
        const QString id = row["id"].isNull() ? QString() : row["id"].toString();
        const QString name = row["builtin"].toBool() ? tr("Projeto") : row["name"].toString();
        m_paletteChooser->addItem(name, id);
        if (id == m_paletteId)
            keep = i;
    }
    m_paletteChooser->setCurrentIndex(keep);
    m_paletteId = m_paletteChooser->itemData(keep).toString();
    m_updating = wasUpdating;
    rebuildSwatches();
}

void PetuniaColorPanel::rebuildSwatches() {
    QVector<QColor> colors;
    bool builtin = true;
    for (const auto &value : m_palettes) {
        const auto row = value.toObject();
        const QString id = row["id"].isNull() ? QString() : row["id"].toString();
        if (id != m_paletteId)
            continue;
        builtin = row["builtin"].toBool();
        for (const auto &c : row["colors"].toArray()) {
            const auto rgba = c.toArray();
            if (rgba.size() >= 3)
                colors << QColor::fromRgbF(float(rgba[0].toDouble()), float(rgba[1].toDouble()), float(rgba[2].toDouble()),
                                           float(rgba.size() > 3 ? rgba[3].toDouble() : 1.0));
        }
    }
    m_swatches->setColors(colors);
    // A paleta do projeto não se renomeia nem se apaga.
    m_renamePalette->setEnabled(!builtin);
    m_deletePalette->setEnabled(!builtin);
}

void PetuniaColorPanel::requestPalette(const QJsonObject &args) {
    const auto reply = petuniaRequest(QStringLiteral("palette"), args);
    if (reply.contains(QStringLiteral("palettes")))
        syncPalettes(reply[QStringLiteral("palettes")].toArray());
}

void PetuniaColorPanel::addCurrentColorToPalette() {
    QJsonObject args{{"op", "add"},
                     {"color", QJsonArray{m_color.redF(), m_color.greenF(), m_color.blueF(), m_color.alphaF()}}};
    args.insert("id", m_paletteId.isEmpty() ? QJsonValue() : QJsonValue(m_paletteId));
    requestPalette(args);
}

bool PetuniaColorPanel::createPalette(const QString &name) {
    if (name.trimmed().isEmpty())
        return false;
    const int before = int(m_palettes.size());
    const auto reply = petuniaRequest(QStringLiteral("palette"), {{"op", "create"}, {"name", name}});
    if (!reply.contains(QStringLiteral("palettes")))
        return false;
    const auto palettes = reply[QStringLiteral("palettes")].toArray();
    if (palettes.size() > before)
        m_paletteId = palettes.last().toObject()["id"].toString(); // a nova fica selecionada
    syncPalettes(palettes);
    return palettes.size() > before;
}

bool PetuniaColorPanel::renamePalette(const QString &name) {
    if (m_paletteId.isEmpty() || name.trimmed().isEmpty())
        return false;
    requestPalette({{"op", "rename"}, {"id", m_paletteId}, {"name", name}});
    return true;
}

bool PetuniaColorPanel::deleteCurrentPalette() {
    if (m_paletteId.isEmpty())
        return false;
    const QString doomed = m_paletteId;
    m_paletteId.clear(); // volta para a paleta do projeto
    requestPalette({{"op", "delete"}, {"id", doomed}});
    return true;
}

// =========================================================================
// PetuniaColorPickerPopover
// =========================================================================

PetuniaColorPickerPopover::PetuniaColorPickerPopover(QWidget *parent) : QDialog(parent) {
    setAccessibleName(tr("Seletor de cor"));
    setWindowTitle(tr("Seletor de cor"));
    setModal(true);
    const auto &t = PetuniaTheme::instance();
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(t.space(3), t.space(2), t.space(3), t.space(3));
    layout->setSpacing(t.space(2));
    m_panel = new PetuniaColorPanel(this);
    layout->addWidget(m_panel, 1);
    connect(m_panel, &PetuniaColorPanel::colorChanged, this, &PetuniaColorPickerPopover::colorChanged);

    auto *actions = new QHBoxLayout();
    actions->addStretch(1);
    auto *cancel = new QPushButton(tr("Cancelar"), this);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    actions->addWidget(cancel);
    auto *confirm = new QPushButton(tr("Confirmar"), this);
    confirm->setObjectName(QStringLiteral("ConfirmBtn"));
    confirm->setProperty("role", QStringLiteral("primary"));
    confirm->setDefault(true);
    connect(confirm, &QPushButton::clicked, this, &QDialog::accept);
    actions->addWidget(confirm);
    layout->addLayout(actions);
    // Barra de título própria (arrasta, fecha, lembra posição/tamanho) — B5.
    PetuniaDialogChrome::install(this, QStringLiteral("color-picker"));
}

void PetuniaColorPickerPopover::setColor(const QColor &color) {
    m_panel->setColor(color);
}

QColor PetuniaColorPickerPopover::color() const {
    return m_panel->color();
}

void PetuniaColorPickerPopover::setTitle(const QString &title) {
    setWindowTitle(title);
}

void PetuniaColorPickerPopover::setAlphaEnabled(bool enabled) {
    m_panel->setAlphaEnabled(enabled);
}
