#include "petunia_vector3_field.h"
#include "petunia_theme.h"

PetuniaVector3Field::PetuniaVector3Field(QWidget *parent)
    : QWidget(parent) {
    m_layout = new QBoxLayout(QBoxLayout::LeftToRight, this);
    m_layout->setContentsMargins(0, 0, 0, 0);
    m_layout->setSpacing(4);

    const char *axisNames[3] = {"X", "Y", "Z"};

    for (int i = 0; i < 3; ++i) {
        m_fields[i] = new PetuniaNumberField(QString::fromLatin1(axisNames[i]), this);
        m_fields[i]->setPropertyId(QString::fromLatin1(axisNames[i]).toLower());
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

void PetuniaVector3Field::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);
    updateLayoutDirection(event->size().width());
}

void PetuniaVector3Field::updateLayoutDirection(int width) {
    // If container is very narrow (< 160px), stack vertically so numbers don't get clipped
    bool shouldStack = (width > 0 && width < 160);
    if (shouldStack != m_stacked) {
        m_stacked = shouldStack;
        m_layout->setDirection(m_stacked ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
    }
}
