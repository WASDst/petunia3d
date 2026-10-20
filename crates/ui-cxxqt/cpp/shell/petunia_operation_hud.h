#pragma once

#include "../petunia_bound_form.h"
#include "petunia_shell_controller.h"

#include <QFrame>
#include <QHash>
#include <QLabel>
#include <QPushButton>

class QCheckBox;
class PetuniaNumberField;
class PetuniaOperationCard;
class PetuniaSegmented;
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
    /// Mostra no HUD os parâmetros (gerados do descriptor) de uma operação do grafo.
    void focusOperation(const QString &nodeId);
    void dismissOperation();
    const QString &focusedOperation() const { return m_graphFocus; }
    PetuniaOperationCard *operationCard() const { return m_graphCard; }
    /// Controle de modo da operação (C8): região, faces individuais ou ao
    /// longo das normais. Nulo só antes do primeiro sync.
    PetuniaSegmented *modeChooser() const { return m_modeChooser; }
    QPushButton *operationDoneButton() const { return m_graphDone; }
    /// Posição dentro de `area`: a que a pessoa escolheu arrastando o cartão (B5),
    /// presa à área, ou o canto superior esquerdo. Duplo clique volta ao canto.
    QPoint placement(const QSize &area, int margin) const;
    bool hasCustomPlacement() const { return m_customPos; }
    void resetPlacement();

  protected:
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void mouseDoubleClickEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

  private:
    class Form;
    QWidget *makeSection(const QString &id, const QString &title);
    void buildPrimitive();
    void buildModal();
    void buildLoopCut();
    void buildSlice();
    void buildGraphOperation();
    void syncGraphOperation(const QJsonObject &state);
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
    /// Modo da operação com vários elementos (C8): região, faces individuais ou
    /// ao longo das normais. Fica escondido quando a operação não tem modo.
    PetuniaSegmented *m_modeChooser{nullptr};
    QString m_modeSignature;
    QWidget *m_bevelOptions{nullptr};
    QPushButton *m_loopCutConfirm{nullptr};
    QStringList m_active;
    QString m_graphFocus;
    QString m_graphSignature;
    PetuniaOperationCard *m_graphCard{nullptr};
    QPushButton *m_graphDone{nullptr};
    QString m_primitiveKind;
    bool m_customPos{false};
    QPoint m_pos;
    bool m_dragging{false};
    QPoint m_dragOffset;
};
