#include "petunia_scrub.h"

#include "petunia_number_field.h"

#include <QAbstractSpinBox>
#include <QApplication>
#include <QDoubleSpinBox>
#include <QEnterEvent>
#include <QLabel>
#include <QLineEdit>
#include <QMouseEvent>
#include <QSpinBox>
#include <algorithm>
#include <cmath>

namespace {
constexpr int kThresholdPx = 3;

/// O QLineEdit interno de um spin box (o valor), nunca os botões de seta.
QAbstractSpinBox *spinOwningEditor(QWidget *w) {
    auto *edit = qobject_cast<QLineEdit *>(w);
    if (!edit)
        return nullptr;
    return qobject_cast<QAbstractSpinBox *>(edit->parentWidget());
}

/// Editor interno do spin box (`lineEdit()` é protegido; ele é um filho direto).
QLineEdit *editorOf(QAbstractSpinBox *spin) {
    return spin ? spin->findChild<QLineEdit *>(QString(), Qt::FindDirectChildrenOnly) : nullptr;
}

bool spinCanScrub(QAbstractSpinBox *spin) {
    if (!spin || !spin->isEnabled() || spin->isReadOnly() || spin->hasFocus())
        return false;
    const QLineEdit *edit = editorOf(spin);
    return !(edit && edit->hasFocus());
}

double spinValue(QAbstractSpinBox *spin) {
    if (auto *d = qobject_cast<QDoubleSpinBox *>(spin))
        return d->value();
    if (auto *i = qobject_cast<QSpinBox *>(spin))
        return i->value();
    return 0.0;
}

double spinStep(QAbstractSpinBox *spin) {
    if (auto *d = qobject_cast<QDoubleSpinBox *>(spin))
        return d->singleStep();
    if (auto *i = qobject_cast<QSpinBox *>(spin))
        return i->singleStep();
    return 1.0;
}

void setSpinValue(QAbstractSpinBox *spin, double value) {
    if (auto *d = qobject_cast<QDoubleSpinBox *>(spin))
        d->setValue(value);
    else if (auto *i = qobject_cast<QSpinBox *>(spin))
        i->setValue(int(std::lround(value)));
}
} // namespace

PetuniaScrub::PetuniaScrub(QObject *parent) : QObject(parent) {}

void PetuniaScrub::install() {
    static PetuniaScrub *instance = nullptr;
    if (instance || !qApp)
        return;
    instance = new PetuniaScrub(qApp);
    qApp->installEventFilter(instance);
}

PetuniaScrub::Target PetuniaScrub::targetFor(QWidget *w) {
    Target target;
    if (!w)
        return target;
    // Nome do campo: o QLabel cujo buddy é um campo numérico.
    if (auto *label = qobject_cast<QLabel *>(w)) {
        QWidget *buddy = label->buddy();
        if (auto *number = qobject_cast<PetuniaNumberField *>(buddy)) {
            if (number->canScrubFromLabel())
                target.number = number;
        } else if (auto *spin = qobject_cast<QAbstractSpinBox *>(buddy)) {
            if (spinCanScrub(spin) && (qobject_cast<QDoubleSpinBox *>(spin) || qobject_cast<QSpinBox *>(spin)))
                target.spin = spin;
        }
        return target;
    }
    // Valor de um PetuniaNumberField: o editor interno, enquanto não está em edição.
    if (auto *edit = qobject_cast<QLineEdit *>(w)) {
        if (auto *number = qobject_cast<PetuniaNumberField *>(edit->parentWidget())) {
            if (number->valueEditor() == edit && number->canScrub())
                target.number = number;
            return target;
        }
    }
    // Valor de um spin box: o editor interno (as setas ficam de fora).
    if (auto *spin = spinOwningEditor(w)) {
        if (spinCanScrub(spin) && (qobject_cast<QDoubleSpinBox *>(spin) || qobject_cast<QSpinBox *>(spin)))
            target.spin = spin;
    }
    return target;
}

void PetuniaScrub::begin(const Target &target, const QPoint &globalPos) {
    m_target = target;
    m_armed = true;
    m_dragging = false;
    m_startPos = globalPos;
    if (m_target.number) {
        // Pelo nome, com o campo em edição: confirma/descarta o texto e passa a arrastar.
        if (m_target.number->isEditing())
            m_target.number->valueEditor()->clearFocus();
        m_target.number->scrubArm(globalPos);
    } else if (m_target.spin)
        m_startValue = spinValue(m_target.spin);
}

void PetuniaScrub::move(const QPoint &globalPos, Qt::KeyboardModifiers mods) {
    if (!m_armed)
        return;
    const int dx = globalPos.x() - m_startPos.x();
    if (!m_dragging && std::abs(dx) >= kThresholdPx) {
        m_dragging = true;
        if (!m_overrideCursor) {
            QApplication::setOverrideCursor(Qt::SizeHorCursor);
            m_overrideCursor = true;
        }
    }
    if (m_target.number) {
        m_target.number->scrubMove(globalPos, mods);
    } else if (m_target.spin && m_dragging) {
        double multiplier = spinStep(m_target.spin);
        if (mods & Qt::ShiftModifier)
            multiplier *= 10.0;
        else if (mods & (Qt::ControlModifier | Qt::AltModifier))
            multiplier *= 0.1;
        // O spin box fixa o valor na sua faixa; o arrasto só soma o deslocamento.
        setSpinValue(m_target.spin, m_startValue + dx * multiplier);
    }
}

void PetuniaScrub::end() {
    if (m_overrideCursor) {
        QApplication::restoreOverrideCursor();
        m_overrideCursor = false;
    }
    if (m_target.number) {
        m_target.number->scrubEnd();
    } else if (m_target.spin && !m_dragging) {
        // Clique sem arrasto: leva ao editor, como um clique normal no campo.
        if (auto *edit = editorOf(m_target.spin)) {
            edit->setFocus(Qt::MouseFocusReason);
            edit->selectAll();
        }
    }
    m_target = Target{};
    m_armed = false;
    m_dragging = false;
}

bool PetuniaScrub::eventFilter(QObject *watched, QEvent *event) {
    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto *mouse = static_cast<QMouseEvent *>(event);
        if (mouse->button() != Qt::LeftButton || m_armed)
            break;
        const Target target = targetFor(qobject_cast<QWidget *>(watched));
        if (!target.valid())
            break;
        begin(target, mouse->globalPosition().toPoint());
        return true; // não deixa o campo focar já no clique: soltar sem arrasto edita
    }
    case QEvent::MouseMove: {
        if (!m_armed)
            break;
        auto *mouse = static_cast<QMouseEvent *>(event);
        if (!(mouse->buttons() & Qt::LeftButton)) {
            end();
            break;
        }
        move(mouse->globalPosition().toPoint(), mouse->modifiers());
        return true;
    }
    case QEvent::MouseButtonRelease: {
        if (!m_armed)
            break;
        auto *mouse = static_cast<QMouseEvent *>(event);
        if (mouse->button() != Qt::LeftButton)
            break;
        end();
        return true;
    }
    case QEvent::Enter: {
        // Dica de que dá para arrastrar: cursor horizontal sobre o nome e o valor.
        auto *w = qobject_cast<QWidget *>(watched);
        if (w && (qobject_cast<QLabel *>(w) || qobject_cast<QLineEdit *>(w))) {
            if (targetFor(w).valid())
                w->setCursor(Qt::SizeHorCursor);
            else if (qobject_cast<QLabel *>(w))
                w->unsetCursor();
            else if (auto *edit = qobject_cast<QLineEdit *>(w); edit && edit->hasFocus())
                w->setCursor(Qt::IBeamCursor);
        }
        break;
    }
    default:
        break;
    }
    return QObject::eventFilter(watched, event);
}
