#pragma once

#include "petunia_shell_controller.h"

#include <QGraphicsView>
#include <QHash>
#include <QJsonObject>

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
class PetuniaNodeCanvas : public QGraphicsView {
    Q_OBJECT

  public:
    explicit PetuniaNodeCanvas(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    int nodeCount() const { return m_nodes.size(); }
    int wireCount() const { return m_wires.size(); }
    QStringList nodeIds() const;
    QString selectedNodeId() const { return m_selected; }
    void selectNode(const QString &id);
    /// Texto lido por tecnologias assistivas: tipo, estado e ligações do nó.
    QString describeSelection() const;
    /// Mesmos caminhos do arrasto, acessíveis ao teclado e a testes.
    bool connectOperation(const QString &operationId, const QString &objectNodeId);
    bool disconnectOperation(const QString &operationId, const QString &objectNodeId);
    QPointF nodeScenePosition(const QString &id) const;
    QRectF nodeSceneRect(const QString &id) const;
    QPointF outputPortScenePos(const QString &id) const;
    QPointF inputPortScenePos(const QString &id) const;
    void fitAll();

  protected:
    void keyPressEvent(QKeyEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void focusInEvent(QFocusEvent *event) override;
    void focusOutEvent(QFocusEvent *event) override;
    void drawBackground(QPainter *painter, const QRectF &rect) override;

  private:
    friend class PetuniaNodeItem;
    void rebuild(const QJsonArray &nodes, const QJsonArray &edges);
    void layoutNodes();
    void updateWires();
    void refreshAccessibility();
    void moveSelection(int dx, int dy);
    void nudgeSelected(int dx, int dy);
    void showConnectMenu(const QPoint &globalPos);
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
    QList<PetuniaWireItem *> m_wires;
    QJsonArray m_nodeRows;
    QJsonArray m_edgeRows;
    QString m_signature;
    QString m_selected;
    QString m_activeAsset;
    // Arrasto de fio a partir de uma saída.
    QString m_wireFrom;
    QGraphicsPathItem *m_ghost{nullptr};
    PetuniaNodeItem *m_dragNode{nullptr};
    QPointF m_dragOffset;
    bool m_panning{false};
    QPoint m_panLast;
    bool m_rebuilding{false};
};
