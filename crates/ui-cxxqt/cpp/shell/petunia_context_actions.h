#pragma once

#include "petunia_shell_controller.h"

#include <QFrame>
#include <QList>
#include <QToolButton>

class QHBoxLayout;

/// Barra de ações contextuais da seleção, flutuante sobre a vista. Projeta o
/// `context_actions` publicado pelo editor: até 4 ações em destaque + "Mais".
/// Habilitação e motivo vêm do catálogo de comandos — nunca filtramos em
/// silêncio. A barra nunca cobre o gizmo: o host a posiciona na borda inferior.
class PetuniaContextActions : public QFrame {
    Q_OBJECT

  public:
    explicit PetuniaContextActions(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    const QList<QToolButton *> &primaryButtons() const { return m_primary; }
    QToolButton *moreButton() const { return m_more; }
    /// Ações visíveis na barra (em destaque), por id.
    QStringList primaryIds() const;
    /// Largura necessária com texto; o host compacta (só ícones) se faltar espaço.
    /// Largura com rótulos. Medida na hora enquanto não está compacta (o estilo do
    /// tema já foi aplicado); em modo compacto vale a última medida com rótulos.
    int fullWidth() const;
    void setCompact(bool compact);
    bool compact() const { return m_compact; }
    /// Oculta a barra enquanto uma operação (HUD) está em curso.
    void setSuppressed(bool suppressed) { m_suppressed = suppressed; }

  private:
    void rebuild(const QJsonArray &actions);
    void refreshStates();
    void run(const QJsonObject &action);

    PetuniaShellController *m_controller;
    QHBoxLayout *m_layout{nullptr};
    QList<QToolButton *> m_primary;
    QToolButton *m_more{nullptr};
    QJsonArray m_actions;
    QString m_signature;
    int m_fullWidth{0};
    bool m_compact{false};
    bool m_suppressed{false};
};
