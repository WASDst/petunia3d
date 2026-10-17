#include "petunia_vector3_field.h"
#include "petunia_theme.h"

PetuniaVector3Field::PetuniaVector3Field(QWidget *parent)
    : QWidget(parent) {
    m_layout = new QBoxLayout(QBoxLayout::LeftToRight, this);
    m_layout->setContentsMargins(0, 0, 0, 0);
    // O mínimo vem de `minimumSizeHint()` (um campo ou a pilha); o layout não o impõe.
    m_layout->setSizeConstraint(QLayout::SetNoConstraint);
    m_layout->setSpacing(PetuniaTheme::instance().space(1));

    for (int i = 0; i < 3; ++i) {
        m_fields[i] = new PetuniaNumberField(PetuniaTheme::axisLetter(i), this);
        m_fields[i]->setPropertyId(PetuniaTheme::axisLetter(i).toLower());
        m_layout->addWidget(m_fields[i], 1);

        connect(m_fields[i], &PetuniaNumberField::valueChanged, this, [this, i](double val) {
            emit axisValueChanged(i, val);
        });
        connect(m_fields[i], &PetuniaNumberField::editStarted, this, [this, i]() {
            emit editStarted(i);
        });
        connect(m_fields[i], &PetuniaNumberField::previewRequested, this, [this, i](const QString &, double val) {
            emit previewRequested(i, val);
        });
        connect(m_fields[i], &PetuniaNumberField::commitRequested, this, [this, i](const QString &, double val, const QString &text) {
            emit commitRequested(i, val, text);
        });
        connect(m_fields[i], &PetuniaNumberField::editCanceled, this, [this, i]() {
            emit editCanceled(i);
        });
    }

    applyThemeColors();
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this,
            &PetuniaVector3Field::applyThemeColors);
}

void PetuniaVector3Field::applyThemeColors() {
    for (int i = 0; i < 3; ++i) {
        if (m_fields[i]) {
            m_fields[i]->setPrefixColor(PetuniaTheme::instance().axisColor(i));
        }
    }
}

void PetuniaVector3Field::setValues(double x, double y, double z) {
    m_fields[0]->setValue(x);
    m_fields[1]->setValue(y);
    m_fields[2]->setValue(z);
}

void PetuniaVector3Field::setValue(int axis, double val) {
    if (axis >= 0 && axis < 3 && m_fields[axis]) {
        m_fields[axis]->setValue(val);
    }
}

double PetuniaVector3Field::value(int axis) const {
    if (axis >= 0 && axis < 3 && m_fields[axis]) {
        return m_fields[axis]->value();
    }
    return 0.0;
}

void PetuniaVector3Field::setMixed(int axis, bool mixed) {
    if (axis >= 0 && axis < 3 && m_fields[axis]) {
        m_fields[axis]->setMixed(mixed);
    }
}

bool PetuniaVector3Field::isMixed(int axis) const {
    if (axis >= 0 && axis < 3 && m_fields[axis]) {
        return m_fields[axis]->isMixed();
    }
    return false;
}

void PetuniaVector3Field::setSuffix(const QString &suffix) {
    for (int i = 0; i < 3; ++i) {
        if (m_fields[i]) {
            m_fields[i]->setSuffix(suffix);
        }
    }
}

void PetuniaVector3Field::setRange(double min, double max) {
    for (int i = 0; i < 3; ++i) {
        if (m_fields[i]) {
            m_fields[i]->setRange(min, max);
        }
    }
}

void PetuniaVector3Field::setSingleStep(double step) {
    for (int i = 0; i < 3; ++i) {
        if (m_fields[i]) {
            m_fields[i]->setSingleStep(step);
        }
    }
}

void PetuniaVector3Field::setMaxDecimals(int dec) {
    for (int i = 0; i < 3; ++i) {
        if (m_fields[i]) {
            m_fields[i]->setMaxDecimals(dec);
        }
    }
}

PetuniaNumberField *PetuniaVector3Field::field(int axis) const {
    if (axis >= 0 && axis < 3) {
        return m_fields[axis];
    }
    return nullptr;
}

void PetuniaVector3Field::setLabel(const QString &label) {
    setAccessibleName(label);
    for (int i = 0; i < 3; ++i) {
        if (m_fields[i])
            m_fields[i]->setLabel(label + QLatin1Char(' ') + PetuniaTheme::axisLetter(i));
    }
}

void PetuniaVector3Field::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);
    updateLayoutDirection(event->size().width());
}

void PetuniaVector3Field::showEvent(QShowEvent *event) {
    QWidget::showEvent(event);
    updateLayoutDirection(width());
}

QSize PetuniaVector3Field::minimumSizeHint() const {
    // Mínimo = um campo; abaixo da largura necessária para os três lado a lado,
    // os eixos empilham (nunca cortamos X/Y/Z nem forçamos rolagem horizontal).
    const auto &t = PetuniaTheme::instance();
    const QSize one = m_fields[0] ? m_fields[0]->minimumSizeHint() : QSize(0, t.controlHeight());
    const int height = m_stacked ? 3 * one.height() + 2 * m_layout->spacing() : one.height();
    return {one.width(), height};
}

void PetuniaVector3Field::updateLayoutDirection(int width) {
    int needed = 2 * m_layout->spacing();
    for (auto *field : m_fields) {
        if (field) {
            const int fieldMin = std::max(field->minimumSizeHint().width(),
                                          PetuniaTheme::instance().scaled(48));
            needed += fieldMin;
        }
    }
    const bool shouldStack = width > 0 && width < needed;
    if (shouldStack != m_stacked) {
        m_stacked = shouldStack;
        m_layout->setDirection(m_stacked ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
        m_layout->activate();
        updateGeometry();
    }
}
