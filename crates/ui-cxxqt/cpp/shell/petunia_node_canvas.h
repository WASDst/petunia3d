#pragma once

#include "petunia_shell_controller.h"

#include <QAccessible>
#include <QGraphicsView>
#include <QHash>
#include <QJsonObject>

class QAccessibleInterface;
class QGraphicsScene;
class PetuniaNodeItem;
class PetuniaWireItem;

/// Canvas do Operation Graph (ADR-C13, O03): projeta `op_graph.nodes/edges` num
/// `QGraphicsView`. Nó = operação ou objeto; fio = "a operação age neste objeto".
///
/// UI fina: o canvas só emite ações `graph_*` (arrastar de uma saída a um objeto =
/// `graph_apply`; Delete num fio = `graph_unapply`; soltar um nó = `graph_move_node`).
/// Operável **sem mouse**: setas navegam entre nós, Enter abre "Aplicar em…", Delete
/// remove/desliga, Ctrl+setas movem o nó, Shift+A adiciona operação ao objeto ativo.
/// O nome/descrição acessíveis resumem o nó selecionado e suas ligações.
class QToolButton;

class PetuniaNodeCanvas : public QGraphicsView {
    Q_OBJECT

  public:
    /// Grafo exibido: operações de geometria (ADR-C13) ou receita do material ativo (O05).
    enum class Domain { Operations, Material };

    explicit PetuniaNodeCanvas(PetuniaShellController *controller, QWidget *parent = nullptr);
    ~PetuniaNodeCanvas() override;

    void sync(const QJsonObject &state);
    void setDomain(Domain domain);
    Domain domain() const { return m_domain; }
    /// Receita: liga `from.saída[fromSocket]` → `to.entrada[toSocket]` (substitui a ligação existente).
    bool connectSockets(const QString &from, int fromSocket, const QString &to, int toSocket);
    bool disconnectSocket(const QString &to, int toSocket);
    QPointF inputSocketScenePos(const QString &id, int socket) const;
    QPointF outputSocketScenePos(const QString &id, int socket) const;
    int nodeCount() const { return m_nodes.size(); }
    int wireCount() const { return m_wires.size(); }
    QStringList nodeIds() const;
    QString selectedNodeId() const { return m_selected; }
    void selectNode(const QString &id);
    /// Texto lido por tecnologias assistivas: tipo, estado e ligações do nó.
    QString describeSelection() const { return describeNode(m_selected); }
    QString describeNode(const QString &id) const;
    QString nodeTitle(const QString &id) const;
    bool nodeEnabled(const QString &id) const;
    /// Retângulo do nó em coordenadas globais (tecnologias assistivas).
    QRect nodeGlobalRect(const QString &id) const;
    /// Interface acessível persistente do nó (um por id; liberada ao sair do grafo).
    QAccessibleInterface *accessibleNode(const QString &id);
    /// Mesmos caminhos do arrasto, acessíveis ao teclado e a testes.
    bool connectOperation(const QString &operationId, const QString &objectNodeId);
    bool disconnectOperation(const QString &operationId, const QString &objectNodeId);
    QPointF nodeScenePosition(const QString &id) const;
    QRectF nodeSceneRect(const QString &id) const;
    QPointF outputPortScenePos(const QString &id) const;
    QPointF inputPortScenePos(const QString &id) const;
    /// Enquadra todos os nós (zoom e posição) sem ampliar além de 100 %.
    void fitAll();
    /// Desloca a vista em pixels (a cena é infinita: não há limite de rolagem).
    void panBy(const QPointF &delta);
    /// Zoom por `factor` ancorado num ponto da vista, dentro de 30 %–300 %.
    void zoomBy(qreal factor, const QPointF &anchorInView);

  signals:
    void nodeSelected(const QString &id);

  protected:
    void keyPressEvent(QKeyEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyReleaseEvent(QKeyEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void focusInEvent(QFocusEvent *event) override;
    void focusOutEvent(QFocusEvent *event) override;
    void drawBackground(QPainter *painter, const QRectF &rect) override;
    void drawForeground(QPainter *painter, const QRectF &rect) override;

  private:
    friend class PetuniaNodeItem;
    void rebuild(const QJsonArray &nodes, const QJsonArray &edges);
    void layoutNodes();
    void updateWires();
    void refreshAccessibility();
    void releaseStaleAccessibles();
    void moveSelection(int dx, int dy);
    void nudgeSelected(int dx, int dy);
    void showConnectMenu(const QPoint &globalPos);
    void showRecipeConnectMenu(const QPoint &globalPos);
    void normalizeRecipe(const QJsonObject &recipe, QJsonArray &nodes, QJsonArray &edges) const;
    void showAddMenu(const QPoint &globalPos);
    void deleteSelected();
    PetuniaNodeItem *nodeAt(const QPointF &scenePos) const;
    QJsonObject nodeRow(const QString &id) const;
    QString assetOf(const QString &nodeId) const;
    QStringList targetsOf(const QString &operationId) const;
    bool isOperation(const QString &id) const;

    PetuniaShellController *m_controller;
    QGraphicsScene *m_scene;
    QHash<QString, PetuniaNodeItem *> m_nodes;
    QHash<QString, QAccessible::Id> m_accessible;
    /// Última posição publicada pelo documento por nó: mudança = Undo/Redo ou outro cliente.
    QHash<QString, QPointF> m_serverPositions;
    QList<PetuniaWireItem *> m_wires;
    QJsonArray m_nodeRows;
    QJsonArray m_edgeRows;
    QString m_signature;
    QString m_selected;
    QString m_activeAsset;
    Domain m_domain{Domain::Operations};
    QJsonObject m_state;
    QJsonArray m_catalog;
    bool m_hasRecipe{false};
    int m_wireFromSocket{0};
    // Arrasto de fio a partir de uma saída.
    QString m_wireFrom;
    QGraphicsPathItem *m_ghost{nullptr};
    PetuniaNodeItem *m_dragNode{nullptr};
    QPointF m_dragOffset;
    bool m_panning{false};
    QPoint m_panLast;
    /// Botão que iniciou o pan (meio, direito ou esquerdo com Espaço).
    Qt::MouseButton m_panButton{Qt::NoButton};
    QPoint m_panStart;
    /// O botão direito só vira pan depois de arrastar; um clique abre o menu.
    bool m_panMoved{false};
    bool m_spaceHeld{false};
    bool m_viewFitted{false};
    QToolButton *m_fitButton{nullptr};
    void placeFitButton();
    bool m_rebuilding{false};
};
