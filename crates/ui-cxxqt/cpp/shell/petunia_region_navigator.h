#pragma once

#include <QHash>
#include <QList>
#include <QObject>
#include <QPointer>
#include <QWidget>

/// Navegação por regiões (F6 / Shift+F6): percorre as regiões do shell na ordem
/// canônica (Barra → Rail → Superfície → Estrutura → Propriedades → Gaveta →
/// Ações contextuais). Lembra o último foco de cada região e o restaura ao
/// voltar (AGENTS.md #7: "restauração de foco"). Tab continua dentro da região.
class PetuniaRegionNavigator : public QObject {
    Q_OBJECT

  public:
    explicit PetuniaRegionNavigator(QObject *parent = nullptr);

    void setRegions(const QList<QWidget *> &regions);
    const QList<QPointer<QWidget>> &regions() const { return m_regions; }
    /// Índice da região que contém `focus` (ou -1).
    int regionOf(const QWidget *focus) const;
    /// Move o foco para a próxima região visível (direction = +1/-1).
    /// Devolve o widget que recebeu o foco, ou nulo se nada era focável.
    QWidget *cycle(int direction, QWidget *current = nullptr);
    /// Primeiro widget focável de uma região (ou ela mesma).
    static QWidget *firstFocusable(QWidget *region);

  private:
    void onFocusChanged(QWidget *old, QWidget *now);

    QList<QPointer<QWidget>> m_regions;
    QHash<QWidget *, QPointer<QWidget>> m_lastFocus;
};
