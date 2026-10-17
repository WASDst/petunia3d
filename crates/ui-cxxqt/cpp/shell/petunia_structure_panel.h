#pragma once

#include "petunia_shell_controller.h"

#include <QLineEdit>
#include <QPushButton>
#include <QSortFilterProxyModel>
#include <QStandardItemModel>
#include <QTreeView>
#include <QWidget>

class PetuniaPlaceholderMessage;

/// Região "Estrutura": árvore de Partes da cena (model/view com identidade
/// estável por id de objeto). Mostra, filtra e organiza; toda mudança é uma
/// ação do editor (seleção, visibilidade, bloqueio, renomear, mover).
class PetuniaStructurePanel : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaStructurePanel(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    QTreeView *tree() const { return m_tree; }
    QLineEdit *search() const { return m_search; }
    int rowCount() const { return m_model->rowCount(); }

  signals:
    void createRequested();

  private:
    enum Column { Name = 0, Visible = 1, Locked = 2 };
    enum Role { IdRole = Qt::UserRole, SelectedRole = Qt::UserRole + 1 };

    void buildShortcuts();
    void onClicked(const QModelIndex &proxyIndex);
    void onSelectionChanged();
    void onContextMenu(const QPoint &pos);
    QStringList selectedIds() const;
    QString idAt(const QModelIndex &proxyIndex) const;
    void refreshIcons();

    void syncScene(const QJsonObject &state);
    void syncPaint(const QJsonObject &state);
    void syncAnimate(const QJsonObject &state);

    PetuniaShellController *m_controller;
    QLineEdit *m_search{nullptr};
    QPushButton *m_filterSelected{nullptr};
    QPushButton *m_sort{nullptr};
    QTreeView *m_tree{nullptr};
    PetuniaPlaceholderMessage *m_empty{nullptr};
    QStandardItemModel *m_model{nullptr};
    QSortFilterProxyModel *m_proxy{nullptr};
    bool m_syncing{false};
    QJsonArray m_lastItems;
};
