#pragma once

#include "petunia_shell_controller.h"

#include <QFrame>
#include <QList>
#include <QToolButton>

class QLabel;

/// Painel de alinhar e distribuir objetos. Substitui o submenu longo com
/// catorze textos: uma grade de 3 eixos × 3 posições (mín., centro, máx.) com
/// pictogramas desenhados em código, e uma linha para distribuir. A habilitação
/// e o motivo vêm do catálogo de comandos; cada botão executa um comando
/// registrado (Undo incluso) e o painel continua aberto para encadear eixos.
class PetuniaAlignPopover : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaAlignPopover(PetuniaShellController *controller, QWidget *parent = nullptr);

    /// Abre sob `anchor` (ou no canto da vista) e sincroniza o estado.
    void popupAt(QWidget *anchor);
    void sync(const QJsonObject &state);
    const QList<QToolButton *> &buttons() const { return m_buttons; }

  private:
    QToolButton *makeButton(const QString &command, const QString &tip, int axis, int kind);

    PetuniaShellController *m_controller;
    QLabel *m_hint{nullptr};
    QList<QToolButton *> m_buttons;
};
