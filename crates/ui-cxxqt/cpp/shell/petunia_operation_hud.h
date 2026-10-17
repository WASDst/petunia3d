#pragma once

#include "../petunia_bound_form.h"
#include "petunia_shell_controller.h"

#include <QFrame>
#include <QHash>
#include <QLabel>
#include <QPushButton>

class QCheckBox;
class PetuniaNumberField;
class QVBoxLayout;

/// HUD de operação: parâmetros VIVOS da operação em curso, sobre a vista
/// (Plasticity/Shapr3D). Substitui os cartões soltos de primitiva, loop cut e
/// o campo "valor da operação" que viviam em abas estáticas. Um único painel
/// com seções que só aparecem quando a sessão correspondente está ativa:
/// primitiva, valor modal (extrude/inset/bevel/push-pull), loop cut e corte.
/// Confirmar/cancelar usam as ações do editor (um gesto = um Undo).
class PetuniaOperationHud : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaOperationHud(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    /// Seções ativas no último estado (ids estáveis, para testes e a11y).
    QStringList activeSections() const { return m_active; }
    PetuniaNumberField *modalField() const { return m_modalValue; }

  private:
    class Form;
    QWidget *makeSection(const QString &id, const QString &title);
    void buildPrimitive();
    void buildModal();
    void buildLoopCut();
    void buildSlice();
    QPushButton *actionButton(QWidget *parent, const QString &text, const QString &request, bool primary);
    void relayout();

    PetuniaShellController *m_controller;
    QVBoxLayout *m_root{nullptr};
    QLabel *m_title{nullptr};
    QLabel *m_hint{nullptr};
    QHash<QString, QWidget *> m_sections;
    QHash<QString, Form *> m_forms;
    PetuniaNumberField *m_modalValue{nullptr};
    QLabel *m_modalLabel{nullptr};
    QWidget *m_bevelOptions{nullptr};
    QPushButton *m_loopCutConfirm{nullptr};
    QStringList m_active;
    QString m_primitiveKind;
};
