#pragma once

#include <QObject>
#include <QPointer>
#include <QPoint>

class QAbstractSpinBox;
class QWidget;
class PetuniaNumberField;

/// Arrastar para ajustar um número, em **todo** campo numérico do app.
///
/// Um único filtro de eventos no `QApplication` (nenhuma tela instala nada):
///   - arrastar sobre o **valor** de um campo que não está sendo editado;
///   - arrastar sobre o **nome** do campo (o `QLabel` cujo buddy é o campo).
/// Vale para `PetuniaNumberField` e para `QSpinBox`/`QDoubleSpinBox`. Esquerda e direita
/// diminuem/aumentam pelo passo do campo; Shift ×10, Ctrl/Alt ×0,1. Um clique sem
/// arrasto leva ao editor (digitar continua sendo o caminho principal) e as setas
/// do spin box continuam funcionando como antes.
class PetuniaScrub : public QObject {
public:
    /// Instala o filtro uma vez (idempotente). Seguro de chamar de qualquer tela.
    static void install();

    /// Alvo de um gesto: o campo que muda e se o clique veio do nome ou do valor.
    struct Target {
        QPointer<PetuniaNumberField> number;
        QPointer<QAbstractSpinBox> spin;
        bool valid() const { return !number.isNull() || !spin.isNull(); }
    };
    /// Campo que o widget `w` aciona (valor ou nome), ou alvo vazio. Público para testes.
    static Target targetFor(QWidget *w);

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    explicit PetuniaScrub(QObject *parent);
    void begin(const Target &target, const QPoint &globalPos);
    void move(const QPoint &globalPos, Qt::KeyboardModifiers mods);
    void end();

    Target m_target;
    bool m_armed{false};
    bool m_dragging{false};
    bool m_overrideCursor{false};
    QPoint m_startPos;
    double m_startValue{0.0};
};
