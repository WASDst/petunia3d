#include "petunia_number_field.h"
#include "petunia_theme.h"
#include <QHBoxLayout>
#include <QDoubleValidator>
#include <QApplication>
#include <cmath>

PetuniaNumberField::PetuniaNumberField(QWidget *parent)
    : PetuniaNumberField(QString(), parent) {
}

PetuniaNumberField::PetuniaNumberField(const QString &prefix, QWidget *parent)
    : QWidget(parent)
    , m_prefix(prefix) {
    setObjectName(QStringLiteral("PetuniaNumberField"));
    setStyleSheet(QStringLiteral(
        "QWidget#PetuniaNumberField { background-color: #252525; border: 1px solid #3c3c3c; border-radius: 4px; }\n"
        "QWidget#PetuniaNumberField:hover { border-color: #555555; background-color: #2a2a2a; }\n"
        "QWidget#PetuniaNumberField:focus-within { border-color: #b58cff; background-color: #202020; }\n"
        "QLineEdit { background: transparent; border: none; color: #eeeeee; padding: 2px 4px; font-size: 11px; }\n"
    ));

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(2, 1, 2, 1);
    layout->setSpacing(2);

    m_prefixLabel = new QLabel(this);
    m_prefixLabel->setObjectName(QStringLiteral("PrefixLabel"));
    m_prefixLabel->setAlignment(Qt::AlignCenter);
    m_prefixLabel->setCursor(Qt::SizeHorCursor);
    m_prefixLabel->setMinimumWidth(prefix.isEmpty() ? 0 : 14);
    m_prefixLabel->setVisible(!prefix.isEmpty());
    if (!prefix.isEmpty()) {
        m_prefixLabel->setText(prefix);
    }
    layout->addWidget(m_prefixLabel);

    m_lineEdit = new QLineEdit(this);
    m_lineEdit->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    m_lineEdit->installEventFilter(this);
    layout->addWidget(m_lineEdit, 1);

    setFocusProxy(m_lineEdit);
    updateDisplay();
}

void PetuniaNumberField::setValue(double val) {
    if (!m_isEditing && !m_isScrubbing) {
        m_value = std::clamp(val, m_min, m_max);
        m_isMixed = false;
        updateDisplay();
    }
}

void PetuniaNumberField::setMixed(bool mixed) {
    m_isMixed = mixed;
    updateDisplay();
}

void PetuniaNumberField::setRange(double min, double max) {
    m_min = min;
    m_max = max;
    if (m_value < m_min || m_value > m_max) {
        setValue(std::clamp(m_value, m_min, m_max));
    }
}

void PetuniaNumberField::setSuffix(const QString &suffix) {
    m_suffix = suffix;
    updateDisplay();
}

void PetuniaNumberField::setPrefix(const QString &prefix) {
    m_prefix = prefix;
    m_prefixLabel->setText(prefix);
    m_prefixLabel->setVisible(!prefix.isEmpty());
    m_prefixLabel->setMinimumWidth(prefix.isEmpty() ? 0 : 12);
}

void PetuniaNumberField::setPrefixColor(const QColor &color) {
    if (m_prefixLabel) {
        m_prefixLabel->setStyleSheet(QStringLiteral(
            "background-color: %1; color: #121212; font-weight: bold; font-size: 10px; min-width: 14px; padding: 1px 3px; border-radius: 2px; margin: 1px;"
        ).arg(color.name()));
    }
}

QString PetuniaNumberField::formatValue(double val) const {
    if (m_isMixed) {
        return QStringLiteral("—");
    }
    // Remove trailing zeros contextual format: 1.5 instead of 1.5000, 0 instead of 0.0000
    QString s = QString::number(val, 'f', m_maxDecimals);
    if (s.contains('.')) {
        while (s.endsWith('0')) {
            s.chop(1);
        }
        if (s.endsWith('.')) {
            s.chop(1);
        }
    }
    if (!m_suffix.isEmpty()) {
        s += QStringLiteral(" ") + m_suffix;
    }
    return s;
}

void PetuniaNumberField::updateDisplay() {
    if (m_lineEdit && !m_isEditing) {
        m_lineEdit->setText(formatValue(m_value));
    }
}

void PetuniaNumberField::commitEdit() {
    if (m_isEditing) {
        m_isEditing = false;
        QString text = m_lineEdit->text().trimmed();
        if (text == QStringLiteral("—")) {
            m_isMixed = true;
        } else {
            bool ok = false;
            double v = text.toDouble(&ok);
            if (ok) {
                m_value = std::clamp(v, m_min, m_max);
                m_isMixed = false;
                emit valueChanged(m_value);
                emit commitRequested(m_propertyId, m_value, text);
            } else if (!text.isEmpty()) {
                // Could be an expression like "2+3" or "10/2" supported by petunia_editor
                emit commitRequested(m_propertyId, m_value, text);
            } else {
                // Invalid empty text: restore old value
                m_value = m_initialValue;
                emit editCanceled(m_propertyId);
            }
        }
        updateDisplay();
    }
}

void PetuniaNumberField::cancelEdit() {
    if (m_isEditing) {
        m_isEditing = false;
        m_value = m_initialValue;
        updateDisplay();
        emit editCanceled(m_propertyId);
    }
}

void PetuniaNumberField::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        m_isScrubbing = true;
        m_scrubStartPos = event->globalPosition().toPoint();
        m_scrubStartValue = m_value;
        m_initialValue = m_value;
        emit editStarted(m_propertyId);
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void PetuniaNumberField::mouseMoveEvent(QMouseEvent *event) {
    if (m_isScrubbing) {
        int deltaX = event->globalPosition().toPoint().x() - m_scrubStartPos.x();
        double multiplier = m_step;
        if (event->modifiers() & Qt::ShiftModifier) {
            multiplier *= 10.0;
        } else if (event->modifiers() & (Qt::ControlModifier | Qt::AltModifier)) {
            multiplier *= 0.1;
        }
        double nextVal = std::clamp(m_scrubStartValue + (deltaX * multiplier), m_min, m_max);
        if (std::abs(nextVal - m_value) > 1e-9) {
            m_value = nextVal;
            m_isMixed = false;
            m_lineEdit->setText(formatValue(m_value));
            emit previewRequested(m_propertyId, m_value);
            emit valueChanged(m_value);
        }
        event->accept();
        return;
    }
    QWidget::mouseMoveEvent(event);
}

void PetuniaNumberField::mouseReleaseEvent(QMouseEvent *event) {
    if (m_isScrubbing && event->button() == Qt::LeftButton) {
        m_isScrubbing = false;
        emit commitRequested(m_propertyId, m_value, QString::number(m_value, 'g', 10));
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

bool PetuniaNumberField::eventFilter(QObject *watched, QEvent *event) {
    if (watched == m_lineEdit) {
        if (event->type() == QEvent::FocusIn) {
            m_isEditing = true;
            m_initialValue = m_value;
            emit editStarted(m_propertyId);
            if (!m_isMixed) {
                // Display raw number without suffix while editing
                QString s = QString::number(m_value, 'f', m_maxDecimals);
                if (s.contains('.')) {
                    while (s.endsWith('0')) s.chop(1);
                    if (s.endsWith('.')) s.chop(1);
                }
                m_lineEdit->setText(s);
            }
            m_lineEdit->selectAll();
        } else if (event->type() == QEvent::FocusOut) {
            commitEdit();
        } else if (event->type() == QEvent::KeyPress) {
            auto *keyEvent = static_cast<QKeyEvent *>(event);
            if (keyEvent->key() == Qt::Key_Return || keyEvent->key() == Qt::Key_Enter) {
                commitEdit();
                m_lineEdit->clearFocus();
                return true;
            } else if (keyEvent->key() == Qt::Key_Escape) {
                cancelEdit();
                m_lineEdit->clearFocus();
                return true;
            } else if (keyEvent->key() == Qt::Key_Up || keyEvent->key() == Qt::Key_Down) {
                double step = m_step;
                if (keyEvent->modifiers() & Qt::ShiftModifier) {
                    step *= 10.0;
                } else if (keyEvent->modifiers() & (Qt::ControlModifier | Qt::AltModifier)) {
                    step *= 0.1;
                }
                if (keyEvent->key() == Qt::Key_Down) {
                    step = -step;
                }
                bool ok = false;
                double cur = m_lineEdit->text().toDouble(&ok);
                if (!ok) cur = m_value;
                double nextVal = std::clamp(cur + step, m_min, m_max);
                m_value = nextVal;
                m_isMixed = false;
                QString s = QString::number(m_value, 'f', m_maxDecimals);
                if (s.contains('.')) {
                    while (s.endsWith('0')) s.chop(1);
                    if (s.endsWith('.')) s.chop(1);
                }
                m_lineEdit->setText(s);
                emit previewRequested(m_propertyId, m_value);
                emit valueChanged(m_value);
                return true;
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}
