#include "petunia_number_field.h"
#include "petunia_icons.h"
#include "petunia_expression.h"
#include "petunia_scrub.h"
#include "petunia_theme.h"

#include <QAccessible>
#include <QEnterEvent>
#include <QFontMetrics>
#include <QHBoxLayout>
#include <QPainter>
#include <QPainterPath>
#include <QWheelEvent>
#include <algorithm>
#include <cmath>

namespace {
constexpr int kScrubThresholdPx = 3;

QColor readableOn(const QColor &background) {
    const auto &t = PetuniaTheme::instance();
    return PetuniaTheme::contrastRatio(background, t.textActive()) >=
                   PetuniaTheme::contrastRatio(background, t.textPrimary())
               ? t.textActive()
               : t.textPrimary();
}
} // namespace

PetuniaNumberField::PetuniaNumberField(QWidget *parent)
    : PetuniaNumberField(QString(), parent) {}

PetuniaNumberField::PetuniaNumberField(const QString &prefix, QWidget *parent)
    : QWidget(parent), m_prefix(prefix) {
    setObjectName(QStringLiteral("PetuniaNumberField"));
    PetuniaScrub::install(); // arrastar valor ou nome ajusta o número, em todo o app
    setAttribute(Qt::WA_Hover, true);
    setMouseTracking(true);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);

    auto *layout = new QHBoxLayout(this);
    layout->setSpacing(0);

    m_lineEdit = new QLineEdit(this);
    m_lineEdit->setObjectName(QStringLiteral("PetuniaNumberFieldEdit"));
    m_lineEdit->setFrame(false);
    m_lineEdit->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    // O quadro e o anel de foco são pintados por este widget (tokens); o
    // editor interno só edita texto e precisa ignorar a folha de estilo global.
    m_lineEdit->setStyleSheet(QStringLiteral(
        "QLineEdit#PetuniaNumberFieldEdit { background: transparent; border: none; "
        "padding: 0; min-height: 0; }"));
    m_lineEdit->installEventFilter(this);
    layout->addWidget(m_lineEdit, 1);
    setFocusProxy(m_lineEdit);
    setFocusPolicy(Qt::StrongFocus);

    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaNumberField::applyTheme);
    applyTheme();
    updateDisplay();
}

// ── API pública ─────────────────────────────────────────────────────────────

void PetuniaNumberField::setValue(double val) {
    // O host nunca pisa no valor enquanto o usuário edita ou arrasta.
    if (m_isEditing || m_scrubbing)
        return;
    if (!std::isfinite(val))
        return;
    m_value = std::clamp(val, m_min, m_max);
    m_isMixed = false;
    setError(QString());
    updateDisplay();
}

void PetuniaNumberField::setMixed(bool mixed) {
    if (m_isEditing || m_scrubbing)
        return;
    m_isMixed = mixed;
    updateDisplay();
}

void PetuniaNumberField::setRange(double min, double max) {
    if (!(min <= max))
        return;
    m_min = min;
    m_max = max;
    if (m_value < m_min || m_value > m_max) {
        m_value = std::clamp(m_value, m_min, m_max);
        updateDisplay();
    }
}

void PetuniaNumberField::setSuffix(const QString &suffix) {
    m_suffix = suffix;
    updateDisplay();
    updateGeometry();
}

void PetuniaNumberField::setPrefix(const QString &prefix) {
    m_prefix = prefix;
    applyTheme();
}

void PetuniaNumberField::setPrefixColor(const QColor &color) {
    m_prefixColor = color;
    update();
}

void PetuniaNumberField::setMaxDecimals(int dec) {
    m_maxDecimals = std::clamp(dec, 0, 9);
    updateDisplay();
}

void PetuniaNumberField::setLabel(const QString &label) {
    m_label = label;
    setAccessibleName(label);
    m_lineEdit->setAccessibleName(label);
    m_lineEdit->setToolTip(label);
}

void PetuniaNumberField::setLockable(bool lockable) {
    if (m_lockable == lockable)
        return;
    m_lockable = lockable;
    applyTheme();
}

void PetuniaNumberField::setLocked(bool locked) {
    if (m_locked == locked)
        return;
    m_locked = locked;
    const QString state = locked ? tr("travado: o gizmo e os atalhos não mudam este valor") : QString();
    setAccessibleDescription(state);
    m_lineEdit->setAccessibleDescription(state);
    update();
}

QRect PetuniaNumberField::lockRect() const {
    if (!m_lockable)
        return {};
    const auto &t = PetuniaTheme::instance();
    const int side = std::max(12, t.iconPx() - 2);
    return {width() - side - t.space(1), (height() - side) / 2, side, side};
}

void PetuniaNumberField::setReadOnly(bool readOnly) {
    m_readOnly = readOnly;
    m_lineEdit->setReadOnly(readOnly);
    update();
}

QString PetuniaNumberField::displayText() const {
    return m_lineEdit->text();
}

QSize PetuniaNumberField::sizeHint() const {
    const auto &t = PetuniaTheme::instance();
    const QFontMetrics fm(m_lineEdit->font());
    const int chip = m_prefix.isEmpty() ? 0 : chipRect().width();
    const int text = fm.horizontalAdvance(QStringLiteral("-000.0000")) +
                     (m_suffix.isEmpty() ? 0 : fm.horizontalAdvance(m_suffix + QLatin1Char(' ')));
    return {chip + text + t.space(4), t.fieldHeight()};
}

QSize PetuniaNumberField::minimumSizeHint() const {
    const auto &t = PetuniaTheme::instance();
    const QFontMetrics fm(m_lineEdit->font());
    const int chip = m_prefix.isEmpty() ? 0 : chipRect().width();
    return {chip + fm.horizontalAdvance(QStringLiteral("0.00")) + t.space(3), t.fieldHeight()};
}

// ── Tema ────────────────────────────────────────────────────────────────────

void PetuniaNumberField::applyTheme() {
    const auto &t = PetuniaTheme::instance();
    QFont f = font();
    f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Body));
    m_lineEdit->setFont(f);
    QPalette p = m_lineEdit->palette();
    p.setColor(QPalette::Text, t.textPrimary());
    p.setColor(QPalette::Highlight, t.accentPrimary());
    p.setColor(QPalette::HighlightedText, t.textActive());
    m_lineEdit->setPalette(p);
    setMinimumHeight(t.fieldHeight());
    setMaximumHeight(t.fieldHeight());
    const int chip = m_prefix.isEmpty() ? 0 : chipRect().width();
    const int lock = m_lockable ? std::max(12, t.iconPx() - 2) + t.space(1) : 0;
    layout()->setContentsMargins(chip + t.space(2), 1, t.space(1) + lock, 1);
    updateGeometry();
    update();
}

QRect PetuniaNumberField::chipRect() const {
    if (m_prefix.isEmpty())
        return {};
    const auto &t = PetuniaTheme::instance();
    QFont f = font();
    f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Caption));
    f.setBold(true);
    const QFontMetrics fm(f);
    const int width = std::max(t.space(5), fm.horizontalAdvance(m_prefix) + t.space(2));
    const int h = std::max(0, t.fieldHeight() - 2);
    return {1, 1, width, h};
}

bool PetuniaNumberField::inChip(const QPoint &pos) const {
    return !m_prefix.isEmpty() && chipRect().contains(pos);
}

void PetuniaNumberField::changeEvent(QEvent *event) {
    QWidget::changeEvent(event);
    if (event->type() == QEvent::EnabledChange || event->type() == QEvent::FontChange)
        update();
}

// ── Formatação ──────────────────────────────────────────────────────────────

QString PetuniaNumberField::rawNumberText(double value) const {
    QString s = QString::number(value, 'f', std::max(m_maxDecimals, 6));
    if (s.contains(QLatin1Char('.'))) {
        while (s.endsWith(QLatin1Char('0')))
            s.chop(1);
        if (s.endsWith(QLatin1Char('.')))
            s.chop(1);
    }
    if (s == QLatin1String("-0"))
        s = QStringLiteral("0");
    return s;
}

QString PetuniaNumberField::formatValue(double val) const {
    if (m_isMixed)
        return QStringLiteral("—");
    QString s = QString::number(val, 'f', m_maxDecimals);
    if (s.contains(QLatin1Char('.'))) {
        while (s.endsWith(QLatin1Char('0')))
            s.chop(1);
        if (s.endsWith(QLatin1Char('.')))
            s.chop(1);
    }
    if (s == QLatin1String("-0"))
        s = QStringLiteral("0");
    if (!m_suffix.isEmpty())
        s += QLatin1Char(' ') + m_suffix;
    return s;
}

void PetuniaNumberField::updateDisplay() {
    if (m_isEditing)
        return;
    const QString text = formatValue(m_value);
    if (m_lineEdit->text() != text)
        m_lineEdit->setText(text);
    m_lineEdit->setCursorPosition(0);
    update();
}

void PetuniaNumberField::setError(const QString &message) {
    if (m_error == message)
        return;
    m_error = message;
    m_lineEdit->setAccessibleDescription(message);
    setToolTip(message);
    update();
}

QString PetuniaNumberField::stripSuffix(QString text) const {
    text = text.trimmed();
    if (!m_suffix.isEmpty() && text.endsWith(m_suffix, Qt::CaseInsensitive)) {
        text.chop(m_suffix.size());
        text = text.trimmed();
    }
    return text;
}

bool PetuniaNumberField::acceptText(const QString &text, double *out) {
    const auto result = PetuniaExpression::evaluate(stripSuffix(text));
    if (!result.ok()) {
        setError(PetuniaExpression::describe(result.status));
        emit invalidInput(m_propertyId, m_error);
        return false;
    }
    *out = result.value;
    return true;
}

// ── Edição ──────────────────────────────────────────────────────────────────

bool PetuniaNumberField::commitText(const QString &text) {
    if (m_readOnly)
        return false;
    double v = 0.0;
    if (!acceptText(text, &v))
        return false;
    m_value = std::clamp(v, m_min, m_max);
    m_isMixed = false;
    m_isEditing = false;
    m_edited = false;
    setError(QString());
    updateDisplay();
    emit valueChanged(m_value);
    emit commitRequested(m_propertyId, m_value, text.trimmed());
    return true;
}

void PetuniaNumberField::beginEdit() {
    if (m_isEditing || m_scrubbing || m_readOnly)
        return;
    m_isEditing = true;
    m_edited = false;
    m_initialValue = m_value;
    m_wasMixed = m_isMixed;
    m_editStartText = m_isMixed ? QStringLiteral("—") : rawNumberText(m_value);
    m_lineEdit->setText(m_editStartText);
    m_lineEdit->selectAll();
    emit editStarted(m_propertyId);
    update();
}

void PetuniaNumberField::endEdit() {
    m_isEditing = false;
    m_edited = false;
    setError(QString());
    updateDisplay();
}

void PetuniaNumberField::commitEdit() {
    if (!m_isEditing)
        return;
    const QString text = m_lineEdit->text();
    if (text == m_editStartText && !m_edited) {
        endEdit(); // foco sem edição: nada a confirmar
        return;
    }
    commitText(text); // inválido: mantém o rascunho e a mensagem
}

void PetuniaNumberField::cancelEdit() {
    if (!m_isEditing)
        return;
    const bool changed = m_edited || m_lineEdit->text() != m_editStartText;
    m_value = m_initialValue;
    m_isMixed = m_wasMixed;
    endEdit();
    if (changed)
        emit editCanceled(m_propertyId);
}

void PetuniaNumberField::stepBy(int direction, Qt::KeyboardModifiers modifiers) {
    if (m_readOnly)
        return;
    double step = m_step;
    if (modifiers & Qt::ShiftModifier)
        step *= 10.0;
    else if (modifiers & (Qt::ControlModifier | Qt::AltModifier))
        step *= 0.1;
    double base = m_value;
    const auto parsed = PetuniaExpression::evaluate(stripSuffix(m_lineEdit->text()));
    if (parsed.ok())
        base = parsed.value;
    m_value = std::clamp(base + direction * step, m_min, m_max);
    m_isMixed = false;
    m_edited = true;
    setError(QString());
    m_lineEdit->setText(rawNumberText(m_value));
    emit previewRequested(m_propertyId, m_value);
    emit valueChanged(m_value);
}

void PetuniaNumberField::cancelScrub() {
    if (!m_scrubbing && !m_scrubArmed)
        return;
    const bool wasScrubbing = m_scrubbing;
    m_scrubbing = false;
    m_scrubArmed = false;
    releaseKeyboard();
    m_value = m_initialValue;
    m_isMixed = m_wasMixed;
    updateDisplay();
    if (wasScrubbing)
        emit editCanceled(m_propertyId);
}

// ── Eventos ─────────────────────────────────────────────────────────────────

void PetuniaNumberField::scrubArm(const QPoint &globalPos) {
    m_scrubArmed = true;
    m_scrubbing = false;
    m_scrubStartPos = globalPos;
    m_scrubStartValue = m_value;
    m_initialValue = m_value;
    m_wasMixed = m_isMixed;
}

void PetuniaNumberField::scrubMove(const QPoint &globalPos, Qt::KeyboardModifiers modifiers) {
    if (!m_scrubArmed)
        return;
    const int dx = globalPos.x() - m_scrubStartPos.x();
    if (!m_scrubbing) {
        if (std::abs(dx) < kScrubThresholdPx)
            return;
        m_scrubbing = true;
        grabKeyboard(); // Esc cancela o arrasto
        emit editStarted(m_propertyId);
    }
    double multiplier = m_step;
    if (modifiers & Qt::ShiftModifier)
        multiplier *= 10.0;
    else if (modifiers & (Qt::ControlModifier | Qt::AltModifier))
        multiplier *= 0.1;
    const double next = std::clamp(m_scrubStartValue + dx * multiplier, m_min, m_max);
    if (std::abs(next - m_value) > 1e-9 || m_isMixed) {
        m_value = next;
        m_isMixed = false;
        m_lineEdit->setText(formatValue(m_value));
        emit previewRequested(m_propertyId, m_value);
        emit valueChanged(m_value);
    }
}

void PetuniaNumberField::scrubEnd() {
    if (!m_scrubArmed)
        return;
    const bool wasScrubbing = m_scrubbing;
    m_scrubArmed = false;
    m_scrubbing = false;
    releaseKeyboard();
    if (wasScrubbing) {
        updateDisplay();
        emit commitRequested(m_propertyId, m_value, QString::number(m_value, 'g', 10));
    } else {
        m_lineEdit->setFocus(Qt::MouseFocusReason); // clique simples = editar
    }
}

void PetuniaNumberField::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && m_lockable && isEnabled() &&
        lockRect().adjusted(-2, -2, 2, 2).contains(event->position().toPoint())) {
        emit lockToggled(m_propertyId, !m_locked);
        event->accept();
        return;
    }
    if (event->button() == Qt::LeftButton && inChip(event->position().toPoint()) && !m_readOnly &&
        isEnabled()) {
        scrubArm(event->globalPosition().toPoint());
        event->accept();
        return;
    }
    // Clique no quadro leva o foco ao editor (digitar é o caminho principal).
    if (event->button() == Qt::LeftButton && isEnabled()) {
        m_lineEdit->setFocus(Qt::MouseFocusReason);
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void PetuniaNumberField::mouseMoveEvent(QMouseEvent *event) {
    const bool overChip = inChip(event->position().toPoint());
    if (overChip != m_hoverChip) {
        m_hoverChip = overChip;
        setCursor(overChip && !m_readOnly ? Qt::SizeHorCursor : Qt::ArrowCursor);
    }
    const bool overLock = m_lockable && lockRect().contains(event->position().toPoint());
    if (overLock != m_hoverLock) {
        m_hoverLock = overLock;
        setToolTip(overLock ? (m_locked ? tr("Destravar (Alt+L)") : tr("Travar este valor (Alt+L)")) : QString());
        if (overLock)
            setCursor(Qt::PointingHandCursor);
        else if (!m_hoverChip)
            setCursor(Qt::ArrowCursor);
        update();
    }
    if (!m_scrubArmed || !(event->buttons() & Qt::LeftButton)) {
        QWidget::mouseMoveEvent(event);
        return;
    }
    scrubMove(event->globalPosition().toPoint(), event->modifiers());
    event->accept();
}

void PetuniaNumberField::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && m_scrubArmed) {
        scrubEnd();
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

void PetuniaNumberField::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Escape && (m_scrubbing || m_scrubArmed)) {
        cancelScrub();
        event->accept();
        return;
    }
    QWidget::keyPressEvent(event);
}

void PetuniaNumberField::wheelEvent(QWheelEvent *event) {
    // A roda nunca altera valores (rolagem do painel prevalece).
    event->ignore();
}

void PetuniaNumberField::enterEvent(QEnterEvent *event) {
    m_hover = true;
    update();
    QWidget::enterEvent(event);
}

void PetuniaNumberField::leaveEvent(QEvent *event) {
    m_hover = false;
    m_hoverChip = false;
    unsetCursor();
    update();
    QWidget::leaveEvent(event);
}

bool PetuniaNumberField::eventFilter(QObject *watched, QEvent *event) {
    if (watched != m_lineEdit)
        return QWidget::eventFilter(watched, event);
    switch (event->type()) {
    case QEvent::FocusIn:
        if (!m_scrubbing)
            beginEdit();
        update();
        break;
    case QEvent::FocusOut:
        if (m_isEditing) {
            const bool changed = m_edited || m_lineEdit->text() != m_editStartText;
            if (!changed) {
                endEdit();
            } else {
                const auto parsed = PetuniaExpression::evaluate(stripSuffix(m_lineEdit->text()));
                if (parsed.ok()) {
                    commitText(m_lineEdit->text());
                } else {
                    cancelEdit(); // rascunho inválido não sobrevive à perda de foco
                }
            }
        }
        update();
        break;
    case QEvent::MouseButtonPress:
        // Clique dentro do editor ativa a edição normalmente.
        break;
    case QEvent::Wheel:
        event->ignore();
        return true;
    case QEvent::KeyPress: {
        auto *key = static_cast<QKeyEvent *>(event);
        if (m_lockable && key->key() == Qt::Key_L &&
            (key->modifiers() & ~Qt::KeypadModifier) == Qt::AltModifier) {
            emit lockToggled(m_propertyId, !m_locked);
            return true;
        }
        switch (key->key()) {
        case Qt::Key_Return:
        case Qt::Key_Enter:
            commitEdit();
            if (!isInvalid())
                m_lineEdit->clearFocus();
            return true;
        case Qt::Key_Escape:
            if (m_isEditing || isInvalid()) {
                cancelEdit();
                m_lineEdit->clearFocus();
                return true;
            }
            break;
        case Qt::Key_Up:
        case Qt::Key_Down:
            if (!m_isEditing)
                beginEdit();
            stepBy(key->key() == Qt::Key_Up ? 1 : -1, key->modifiers());
            return true;
        default:
            if (m_isEditing && !key->text().isEmpty() && key->text().at(0).isPrint()) {
                m_edited = true;
                if (isInvalid())
                    setError(QString()); // corrigindo: limpa a marca até a próxima confirmação
            }
            break;
        }
        break;
    }
    default:
        break;
    }
    return QWidget::eventFilter(watched, event);
}

// ── Pintura ─────────────────────────────────────────────────────────────────

void PetuniaNumberField::paintEvent(QPaintEvent *) {
    const auto &t = PetuniaTheme::instance();
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);

    const bool focused = m_lineEdit->hasFocus();
    const bool enabled = isEnabled();
    const double inset = (focused ? t.focusRingWidth() : 1) / 2.0;
    const QRectF frame = QRectF(rect()).adjusted(inset, inset, -inset, -inset);

    QColor fill = !enabled ? t.bgPanel() : focused ? t.bgCanvas() : (m_hover ? t.bgSurfaceHover() : t.bgSurface());
    QPen pen(!enabled ? t.borderSubtle()
             : isInvalid() ? t.statusError()
             : focused     ? t.borderFocus()
             : m_hover     ? t.borderStrong()
                           : t.borderSubtle());
    pen.setWidthF(focused || isInvalid() ? t.focusRingWidth() : 1.0);
    if (isInvalid())
        pen.setStyle(Qt::DashLine); // não depende só de cor
    p.setPen(pen);
    p.setBrush(fill);
    p.drawRoundedRect(frame, t.radius(), t.radius());

    if (!m_prefix.isEmpty()) {
        const QRect chip = chipRect();
        const QColor chipColor = m_prefixColor.isValid() ? m_prefixColor : t.accentPrimary();
        QPainterPath path;
        const QRectF chipF = QRectF(chip).adjusted(1, 1, -1, -1);
        path.addRoundedRect(chipF, t.radius() - 1, t.radius() - 1);
        p.setPen(Qt::NoPen);
        p.setBrush(enabled ? chipColor : t.bgSurfaceActive());
        p.drawPath(path);
        QFont f = font();
        f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Caption));
        f.setBold(true);
        p.setFont(f);
        p.setPen(enabled ? readableOn(chipColor) : t.textMuted());
        p.drawText(chip, Qt::AlignCenter, m_prefix);
    }

    if (m_lockable) {
        // Destravado: discreto (sem competir com o número). Travado: cor de destaque e
        // fundo próprio — a diferença não depende só de cor (ícone muda de forma).
        const QRect lock = lockRect();
        if (m_locked) {
            p.setPen(Qt::NoPen);
            QColor back = t.accentPrimary();
            back.setAlpha(60);
            p.setBrush(back);
            p.drawRoundedRect(QRectF(lock).adjusted(-1, -1, 1, 1), t.radius() - 1, t.radius() - 1);
        }
        const QIcon icon = PetuniaIcons::get(m_locked ? QStringLiteral("lock") : QStringLiteral("unlock"));
        p.setOpacity(!enabled ? 0.35 : m_locked ? 1.0 : (m_hoverLock || m_hover) ? 0.75 : 0.35);
        icon.paint(&p, lock);
        p.setOpacity(1.0);
    }
}
