#pragma once

#include "petunia_shell_controller.h"

#include <QAbstractItemView>
#include <QHash>
#include <QLineEdit>
#include <QPushButton>
#include <QSet>
#include <QSortFilterProxyModel>
#include <QStandardItemModel>
#include <QTreeView>
#include <QWidget>

class PetuniaPlaceholderMessage;

/// Região "Estrutura": lista do workspace (Partes da cena, Camadas de pintura,
/// Ilhas UV ou Ossos) em model/view com identidade estável por id. Mostra,
/// filtra e organiza; toda mudança é uma ação do editor (seleção,
/// visibilidade, bloqueio, isolamento, renomear, agrupar, mover).
///
/// Na cena, as partes se organizam em grupos (collections) numa árvore de um
/// nível: arrastar e soltar reordena e move entre grupos, Ctrl/Shift+clique
/// seleciona vários, Ctrl+G agrupa e Ctrl+Shift+G desagrupa. Cada linha tem
/// visibilidade, bloqueio e isolamento.
class PetuniaStructurePanel : public QWidget {
    Q_OBJECT

  public:
    /// Onde o arrasto cai em relação à linha alvo (espelha o indicador do Qt).
    enum class DropPosition { OnItem, AboveItem, BelowItem, OnViewport };

    explicit PetuniaStructurePanel(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    QTreeView *tree() const { return m_tree; }
    QLineEdit *search() const { return m_search; }
    /// Linhas de primeiro nível (grupos + partes soltas, ou itens da lista plana).
    int rowCount() const { return m_model->rowCount(); }
    /// Ids das partes selecionadas (grupos expandidos nos seus membros).
    QStringList selectedIds() const;
    /// Resolve e envia o pedido de soltar (a mesma lógica do arrasto, para testes).
    void dropOn(const QModelIndex &proxyTarget, DropPosition position);
    /// Índice (do proxy) da linha de uma parte ou de um grupo (`group:<nome>`).
    QModelIndex indexOfId(const QString &id) const;

  signals:
    void createRequested();
    /// Pede o painel de alinhar e distribuir sob `anchor`.
    void alignRequested(QWidget *anchor);

  private:
    enum Column { Name = 0, Visible = 1, Locked = 2, Isolate = 3 };
    enum Role {
        IdRole = Qt::UserRole,
        SelectedRole = Qt::UserRole + 1,
        KindRole = Qt::UserRole + 2,   // "asset" | "group"
        GroupRole = Qt::UserRole + 3,  // nome do grupo (linha de grupo) ou do grupo da parte
    };

    void buildShortcuts();
    void onClicked(const QModelIndex &proxyIndex);
    void onSelectionChanged();
    void onContextMenu(const QPoint &pos);
    /// Ação do estado vazio e do botão primário: cada workspace cria o que lista.
    void primaryAction();
    void updateChrome(const QString &workspace);
    QString idAt(const QModelIndex &proxyIndex) const;
    QString kindAt(const QModelIndex &proxyIndex) const;
    QString groupAt(const QModelIndex &proxyIndex) const;
    QStandardItem *itemAt(const QModelIndex &proxyIndex, int column = Name) const;
    void refreshIcons();
    void configureTree(bool scene);
    QIcon stateIcon(const QString &name, bool active, bool dim = false) const;

    void syncScene(const QJsonObject &state);
    void syncPaint(const QJsonObject &state);
    void syncAnimate(const QJsonObject &state);
    void syncUv(const QJsonObject &state);
    /// Alterna visibilidade/bloqueio, remove ou move a linha atual conforme o workspace.
    void actOnCurrent(const QString &verb, int delta = 0);
    void showGroupMenu(const QString &group, const QPoint &globalPos);

    PetuniaShellController *m_controller;
    QLineEdit *m_search{nullptr};
    QPushButton *m_filterSelected{nullptr};
    QPushButton *m_sort{nullptr};
    QPushButton *m_group{nullptr};
    QPushButton *m_align{nullptr};
    QPushButton *m_primary{nullptr};
    QString m_chromeWorkspace;
    QTreeView *m_tree{nullptr};
    PetuniaPlaceholderMessage *m_empty{nullptr};
    QStandardItemModel *m_model{nullptr};
    QSortFilterProxyModel *m_proxy{nullptr};
    bool m_syncing{false};
    bool m_sceneMode{false};
    bool m_isolated{false};
    QString m_sceneSignature;
    QSet<QString> m_collapsed;
    QHash<QString, QStandardItem *> m_rowItems;
    QJsonArray m_lastItems;
};
