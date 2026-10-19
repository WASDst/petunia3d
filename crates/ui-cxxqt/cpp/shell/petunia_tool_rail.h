#pragma once

#include "petunia_shell_controller.h"

#include <QButtonGroup>
#include <QHash>
#include <QList>
#include <QToolButton>
#include <QColor>
#include <QWidget>

class QScrollArea;
class QVBoxLayout;

/// Região "Rail": ferramentas persistentes do workspace, projetadas do
/// `tool_catalog` publicado pelo editor (nunca listas fixas na GUI). Compacto
/// com ícone + dica rica; expandido mostra os nomes. Foco por setas (roving).
class PetuniaToolRail : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaToolRail(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    bool expanded() const { return m_expanded; }
    void setExpanded(bool expanded);
    QList<QToolButton *> buttons() const { return m_buttons; }
    QToolButton *button(const QString &toolId) const { return m_byId.value(toolId); }
    /// Cor atual do pincel (só no PAINT), como no Photoshop: clicar abre o seletor
    /// e a cor é aplicada ao vivo; Cancelar volta à cor anterior.
    QToolButton *colorSwatch() const { return m_color; }
    QColor brushColor() const { return m_brushColor; }

  signals:
    void expandedChanged(bool expanded);

  protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

  private:
    void rebuild(const QJsonArray &catalog);
    void refreshStates(const QJsonObject &state);
    void applyMetrics();
    void applyRoving();
    void showCreateMenu(QToolButton *anchor, const QJsonObject &state);
    QString tipFor(const QJsonObject &tool) const;

    PetuniaShellController *m_controller;
    QScrollArea *m_scroll{nullptr};
    QWidget *m_column{nullptr};
    QVBoxLayout *m_layout{nullptr};
    QToolButton *m_expand{nullptr};
    QToolButton *m_color{nullptr};
    QColor m_brushColor;
    void openColor();
    void applyBrushColor(const QColor &color);
    QButtonGroup *m_group{nullptr};
    QList<QToolButton *> m_buttons;
    QHash<QString, QToolButton *> m_byId;
    QHash<QString, QString> m_kinds;
    QString m_signature;
    bool m_expanded{false};
    bool m_syncing{false};
};
