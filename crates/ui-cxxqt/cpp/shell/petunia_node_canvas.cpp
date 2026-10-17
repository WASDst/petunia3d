#include "petunia_node_canvas.h"

#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QAccessible>
#include <QContextMenuEvent>
#include <QFocusEvent>
#include <QGraphicsObject>
#include <QGraphicsPathItem>
#include <QGraphicsScene>
#include <QJsonArray>
#include <QKeyEvent>
#include <QMenu>
#include <QPainter>
#include <QPainterPath>
#include <QScrollBar>
#include <QSet>
#include <QStyleOptionGraphicsItem>
#include <QWheelEvent>
#include <algorithm>
#include <cmath>

namespace {
constexpr qreal kNodeWidth = 176;
constexpr qreal kHeader = 26;
constexpr qreal kRow = 22;
constexpr qreal kPort = 7;

QString typeLabel(const QString &type) {
    return type == QLatin1String("operation") ? petuniaTr(QStringLiteral("Operation"))
                                              : petuniaTr(QStringLiteral("Geometry"));
}
} // namespace

// ── Nó ──────────────────────────────────────────────────────────────────────

class PetuniaNodeItem : public QGraphicsObject {
  public:
    PetuniaNodeItem(const QJsonObject &row) : m_row(row) {
        setFlag(QGraphicsItem::ItemIsSelectable, false);
        setAcceptHoverEvents(true);
        m_id = row["id"].toString();
        m_inputs = row["inputs"].toArray();
        m_outputs = row["outputs"].toArray();
    }

    const QString &id() const { return m_id; }
    const QJsonObject &row() const { return m_row; }
    bool isObject() const { return m_row["type_id"].toString() == QLatin1String("petunia.object"); }
    void setSelectedNode(bool selected) { m_selected = selected; update(); }
    void setActive(bool active) { m_active = active; update(); }
    void setFocused(bool focused) { m_focused = focused; update(); }
    void setRow(const QJsonObject &row) { m_row = row; update(); }

    qreal bodyHeight() const {
        const int rows = int(std::max(m_inputs.size(), m_outputs.size()));
        return kHeader + std::max(1, rows) * kRow + 22;
    }
    QRectF boundingRect() const override { return QRectF(-kPort - 2, -2, kNodeWidth + 2 * kPort + 4, bodyHeight() + 4); }

    QPointF inputPos(int index) const { return QPointF(0, kHeader + 12 + index * kRow); }
    QPointF outputPos(int index) const { return QPointF(kNodeWidth, kHeader + 12 + index * kRow); }
    QPointF firstInputScene() const { return mapToScene(inputPos(0)); }
    QPointF firstOutputScene() const { return mapToScene(outputPos(0)); }

    /// Índice da saída sob o ponto (coordenadas da cena), ou -1.
    int outputAt(const QPointF &scenePos) const {
        const QPointF local = mapFromScene(scenePos);
        for (int i = 0; i < m_outputs.size(); ++i)
            if (QLineF(local, outputPos(i)).length() <= kPort + 6)
                return i;
        return -1;
    }

    void paint(QPainter *p, const QStyleOptionGraphicsItem *, QWidget *) override {
        const auto &t = PetuniaTheme::instance();
        const bool known = m_row["known"].toBool(true);
        const bool enabled = m_row["enabled"].toBool(true);
        const QRectF body(0, 0, kNodeWidth, bodyHeight());
        p->setRenderHint(QPainter::Antialiasing);
        p->setBrush(t.bgSurface());
        QColor border = m_selected ? t.accentSelectionActive() : (m_active ? t.accentPrimary() : t.borderStrong());
        p->setPen(QPen(border, m_selected ? t.focusRingWidth() + 1 : 1.2));
        p->drawRoundedRect(body, 7, 7);
        // Cabeçalho: o papel do nó não depende só de cor (glifo + texto).
        p->save();
        p->setClipRect(QRectF(0, 0, kNodeWidth, kHeader));
        p->setPen(Qt::NoPen);
        p->setBrush(isObject() ? t.bgSurfaceActive() : t.bgPanelHeader());
        p->drawRoundedRect(body, 7, 7);
        p->restore();
        QFont f = p->font();
        f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Body));
        f.setBold(true);
        p->setFont(f);
        p->setPen(enabled ? t.textPrimary() : t.textMuted());
        const QString glyph = isObject() ? QStringLiteral("▣ ") : QStringLiteral("ƒ ");
        QString title = glyph + m_row["title"].toString();
        if (!enabled)
            title += QStringLiteral(" (off)");
        p->drawText(QRectF(10, 0, kNodeWidth - 20, kHeader), Qt::AlignVCenter | Qt::AlignLeft,
                    QFontMetrics(f).elidedText(title, Qt::ElideRight, int(kNodeWidth - 20)));
        f.setBold(false);
        f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Caption));
        p->setFont(f);
        for (int i = 0; i < m_inputs.size(); ++i)
            drawPort(p, inputPos(i), m_inputs[i].toObject(), true);
        for (int i = 0; i < m_outputs.size(); ++i)
            drawPort(p, outputPos(i), m_outputs[i].toObject(), false);
        // Resumo de uma linha (parâmetros) + erro.
        p->setPen(t.textSecondary());
        QString summary = summaryText();
        if (!known)
            summary = petuniaTr(QStringLiteral("Nó desconhecido (preservado)"));
        p->drawText(QRectF(10, bodyHeight() - 20, kNodeWidth - 20, 18), Qt::AlignVCenter | Qt::AlignLeft,
                    QFontMetrics(f).elidedText(summary, Qt::ElideRight, int(kNodeWidth - 20)));
        if (!m_row["error"].isNull() && !m_row["error"].toString().isEmpty()) {
            p->setPen(QPen(t.statusError(), 2));
            p->drawText(QRectF(kNodeWidth - 22, 0, 18, kHeader), Qt::AlignCenter, QStringLiteral("!"));
        }
        if (m_focused) {
            p->setBrush(Qt::NoBrush);
            p->setPen(QPen(t.borderFocus(), t.focusRingWidth(), Qt::DashLine));
            p->drawRoundedRect(body.adjusted(-3, -3, 3, 3), 9, 9);
        }
    }

  private:
    void drawPort(QPainter *p, const QPointF &pos, const QJsonObject &port, bool input) const {
        const auto &t = PetuniaTheme::instance();
        const bool geometry = port["type"].toString() == QLatin1String("geometry");
        p->setBrush(geometry ? t.bgSurface() : t.accentPrimary());
        p->setPen(QPen(geometry ? t.accentPrimary() : t.borderStrong(), 2));
        // Forma distingue o tipo (círculo = geometria, losango = operação).
        if (geometry) {
            p->drawEllipse(pos, kPort, kPort);
        } else {
            QPolygonF diamond;
            diamond << pos + QPointF(0, -kPort - 1) << pos + QPointF(kPort + 1, 0) << pos + QPointF(0, kPort + 1)
                    << pos + QPointF(-kPort - 1, 0);
            p->drawPolygon(diamond);
        }
        p->setPen(t.textSecondary());
        const QString label = petuniaTr(port["label"].toString()) + (port["multi"].toBool() ? QStringLiteral(" ×N") : QString());
        const QRectF r = input ? QRectF(pos.x() + 10, pos.y() - 9, kNodeWidth / 2 - 8, 18)
                               : QRectF(pos.x() - kNodeWidth / 2 - 2, pos.y() - 9, kNodeWidth / 2 - 8, 18);
        p->drawText(r, Qt::AlignVCenter | (input ? Qt::AlignLeft : Qt::AlignRight), label);
    }

    QString summaryText() const {
        const auto params = m_row["params"].toObject();
        QStringList parts;
        if (params.contains(QStringLiteral("operation")))
            parts << petuniaTr(params["operation"].toString().left(1).toUpper() + params["operation"].toString().mid(1));
        if (params.contains(QStringLiteral("axis")))
            parts << params["axis"].toString().toUpper();
        return parts.join(QStringLiteral(" · "));
    }

    QJsonObject m_row;
    QString m_id;
    QJsonArray m_inputs;
    QJsonArray m_outputs;
    bool m_selected{false};
    bool m_active{false};
    bool m_focused{false};
};

// ── Fio ─────────────────────────────────────────────────────────────────────

class PetuniaWireItem : public QGraphicsPathItem {
  public:
    PetuniaWireItem(const QString &from, const QString &to, const QString &type)
        : m_from(from), m_to(to), m_type(type) {
        setZValue(-1);
    }
    const QString &from() const { return m_from; }
    const QString &to() const { return m_to; }
    const QString &portType() const { return m_type; }

    void connectPorts(const QPointF &a, const QPointF &b) {
        QPainterPath path(a);
        const qreal dx = std::max<qreal>(40, std::abs(b.x() - a.x()) / 2);
        path.cubicTo(a + QPointF(dx, 0), b - QPointF(dx, 0), b);
        setPath(path);
        const auto &t = PetuniaTheme::instance();
        // Geometria = tracejado; operação = contínuo: o tipo nunca depende só de cor.
        QPen pen(m_type == QLatin1String("geometry") ? t.textSecondary() : t.accentPrimary(),
                 m_type == QLatin1String("geometry") ? 2.0 : 2.6);
        if (m_type == QLatin1String("geometry"))
            pen.setStyle(Qt::DashLine);
        setPen(pen);
        m_mid = path.pointAtPercent(0.5);
    }
    void paint(QPainter *p, const QStyleOptionGraphicsItem *o, QWidget *w) override {
        QGraphicsPathItem::paint(p, o, w);
        const auto &t = PetuniaTheme::instance();
        QFont f = p->font();
        f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Caption));
        p->setFont(f);
        const QString text = typeLabel(m_type);
        const QRectF box(m_mid.x() - 34, m_mid.y() - 9, 68, 18);
        p->setPen(Qt::NoPen);
        p->setBrush(t.bgCanvas());
        p->drawRoundedRect(box, 4, 4);
        p->setPen(t.textSecondary());
        p->drawText(box, Qt::AlignCenter, text);
    }
    QRectF boundingRect() const override { return QGraphicsPathItem::boundingRect().adjusted(-40, -12, 40, 12); }

  private:
    QString m_from, m_to, m_type;
    QPointF m_mid;
};

// ── Canvas ──────────────────────────────────────────────────────────────────

PetuniaNodeCanvas::PetuniaNodeCanvas(PetuniaShellController *controller, QWidget *parent)
    : QGraphicsView(parent), m_controller(controller), m_scene(new QGraphicsScene(this)) {
    setScene(m_scene);
    setRenderHint(QPainter::Antialiasing);
    setFocusPolicy(Qt::StrongFocus);
    setFrameShape(QFrame::NoFrame);
    setDragMode(QGraphicsView::NoDrag);
    setTransformationAnchor(QGraphicsView::AnchorUnderMouse);
    setAccessibleName(tr("Editor de nós"));
    setToolTip(tr("Nós: setas navegam, Enter aplica em…, Delete remove, Ctrl+setas movem, Shift+A adiciona"));
    m_scene->setSceneRect(-400, -300, 1600, 900);
}

QStringList PetuniaNodeCanvas::nodeIds() const {
    QStringList ids;
    for (const auto &row : m_nodeRows)
        ids << row.toObject()["id"].toString();
    return ids;
}

QJsonObject PetuniaNodeCanvas::nodeRow(const QString &id) const {
    for (const auto &row : m_nodeRows)
        if (row.toObject()["id"].toString() == id)
            return row.toObject();
    return {};
}

bool PetuniaNodeCanvas::isOperation(const QString &id) const {
    const auto row = nodeRow(id);
    return !row.isEmpty() && row["type_id"].toString() != QLatin1String("petunia.object");
}

QString PetuniaNodeCanvas::assetOf(const QString &nodeId) const {
    const auto row = nodeRow(nodeId);
    if (row["type_id"].toString() != QLatin1String("petunia.object"))
        return {};
    return row["params"].toObject()["asset"].toString();
}

QStringList PetuniaNodeCanvas::targetsOf(const QString &operationId) const {
    QStringList out;
    for (const auto &edge : m_edgeRows) {
        const auto e = edge.toObject();
        if (e["from"].toString() == operationId && e["type"].toString() == QLatin1String("operation"))
            out << e["to"].toString();
    }
    return out;
}

void PetuniaNodeCanvas::sync(const QJsonObject &state) {
    const auto graph = state["op_graph"].toObject();
    const auto nodes = graph["nodes"].toArray();
    const auto edges = graph["edges"].toArray();
    m_activeAsset = state["active_asset_id"].toString();
    QString signature;
    for (const auto &n : nodes) {
        const auto o = n.toObject();
        signature += o["id"].toString() + QLatin1Char(':') + o["title"].toString() + QLatin1Char(':') +
                     o["type_id"].toString() + QLatin1Char(';');
    }
    for (const auto &e : edges) {
        const auto o = e.toObject();
        signature += o["from"].toString() + QLatin1Char('>') + o["from_port_name"].toString() + QLatin1Char('>') +
                     o["to"].toString() + QLatin1Char('>') + o["to_port_name"].toString() + QLatin1Char(';');
    }
    m_nodeRows = nodes;
    m_edgeRows = edges;
    if (signature != m_signature) {
        m_signature = signature;
        rebuild(nodes, edges);
    } else {
        // Mesma estrutura: atualiza só dados (parâmetros, posição, ativo) sem recriar itens.
        for (const auto &n : nodes) {
            const auto row = n.toObject();
            if (auto *item = m_nodes.value(row["id"].toString()))
                item->setRow(row);
        }
    }
    for (auto it = m_nodes.begin(); it != m_nodes.end(); ++it)
        it.value()->setActive(!m_activeAsset.isEmpty() && assetOf(it.key()) == m_activeAsset);
    updateWires();
    refreshAccessibility();
}

void PetuniaNodeCanvas::rebuild(const QJsonArray &nodes, const QJsonArray &edges) {
    m_rebuilding = true;
    // Guarda posições movidas pelo usuário na sessão (evita saltos ao adicionar nós).
    QHash<QString, QPointF> previous;
    for (auto it = m_nodes.cbegin(); it != m_nodes.cend(); ++it)
        previous.insert(it.key(), it.value()->pos());
    qDeleteAll(m_wires);
    m_wires.clear();
    qDeleteAll(m_nodes);
    m_nodes.clear();
    if (m_ghost) {
        delete m_ghost;
        m_ghost = nullptr;
    }
    // Colunas por papel (ferramenta → operação → alvo) para posições ainda não definidas
    // pelo usuário; o fluxo lê da esquerda para a direita.
    QSet<QString> toolObjects;
    for (const auto &value : edges) {
        const auto e = value.toObject();
        if (e["to_port_name"].toString() == QLatin1String("tool"))
            toolObjects.insert(e["from"].toString());
    }
    QHash<int, int> rowInColumn;
    QSet<QString> used;
    for (const auto &value : nodes) {
        const auto row = value.toObject();
        auto *item = new PetuniaNodeItem(row);
        m_scene->addItem(item);
        const QString id = row["id"].toString();
        const bool op = row["type_id"].toString() != QLatin1String("petunia.object");
        const int column = op ? 1 : (toolObjects.contains(id) ? 0 : 2);
        const QPointF fallback(column * 260.0, rowInColumn[column]++ * 120.0);
        const auto position = row["position"].toArray();
        QPointF pos(position.size() > 0 ? position[0].toDouble() : 0.0, position.size() > 1 ? position[1].toDouble() : 0.0);
        if (previous.contains(id))
            pos = previous.value(id);
        const QString key = QString::number(qRound(pos.x() / 8)) + QLatin1Char(',') + QString::number(qRound(pos.y() / 8));
        // Sem posição do usuário (origem) ou empilhada sobre outro nó: usa a coluna do papel.
        if (used.contains(key) || (pos.isNull() && !previous.contains(id)))
            pos = fallback;
        used.insert(QString::number(qRound(pos.x() / 8)) + QLatin1Char(',') + QString::number(qRound(pos.y() / 8)));
        item->setPos(pos);
        m_nodes.insert(id, item);
    }
    if (!m_selected.isEmpty() && !m_nodes.contains(m_selected))
        m_selected.clear();
    if (m_selected.isEmpty() && !m_nodes.isEmpty())
        m_selected = nodeIds().value(0);
    for (const auto &value : edges) {
        const auto e = value.toObject();
        auto *wire = new PetuniaWireItem(e["from"].toString(), e["to"].toString(),
                                         e["type"].toString());
        m_scene->addItem(wire);
        m_wires.append(wire);
    }
    for (auto it = m_nodes.begin(); it != m_nodes.end(); ++it)
        it.value()->setSelectedNode(it.key() == m_selected);
    m_rebuilding = false;
    if (m_scene->items().size() > 0)
        m_scene->setSceneRect(m_scene->itemsBoundingRect().adjusted(-200, -160, 200, 160));
}

void PetuniaNodeCanvas::updateWires() {
    for (auto *wire : m_wires) {
        auto *from = m_nodes.value(wire->from());
        auto *to = m_nodes.value(wire->to());
        if (!from || !to)
            continue;
        // Fio de operação: saída da operação → entrada "operations" do objeto.
        // Fio de geometria: saída "geometry" do objeto-ferramenta → entrada "tool".
        wire->connectPorts(from->firstOutputScene(), to->firstInputScene());
    }
}

void PetuniaNodeCanvas::selectNode(const QString &id) {
    if (!m_nodes.contains(id))
        return;
    m_selected = id;
    for (auto it = m_nodes.begin(); it != m_nodes.end(); ++it)
        it.value()->setSelectedNode(it.key() == id);
    if (auto *item = m_nodes.value(id))
        ensureVisible(item, 40, 40);
    refreshAccessibility();
}

QString PetuniaNodeCanvas::describeSelection() const {
    const auto row = nodeRow(m_selected);
    if (row.isEmpty())
        return tr("Nenhum nó. Shift+A adiciona uma operação ao objeto ativo.");
    QString text = petuniaTr(row["label"].toString()) + QStringLiteral(": ") + row["title"].toString();
    if (row["type_id"].toString() == QLatin1String("petunia.object")) {
        QStringList ops;
        for (const auto &edge : m_edgeRows) {
            const auto e = edge.toObject();
            if (e["to"].toString() == m_selected && e["to_port_name"].toString() == QLatin1String("operations"))
                ops << nodeRow(e["from"].toString())["title"].toString();
        }
        text += ops.isEmpty() ? tr(". Sem operações aplicadas.") : tr(". Operações: %1.").arg(ops.join(QStringLiteral(", ")));
    } else {
        QStringList targets;
        for (const auto &id : targetsOf(m_selected))
            targets << nodeRow(id)["title"].toString();
        text += targets.isEmpty() ? tr(". Saída sem destino.") : tr(". Aplicada em: %1.").arg(targets.join(QStringLiteral(", ")));
        if (!row["enabled"].toBool(true))
            text += tr(" Desativada.");
    }
    const auto error = row["error"];
    if (error.isString() && !error.toString().isEmpty())
        text += tr(" Erro: %1").arg(error.toString());
    return text;
}

void PetuniaNodeCanvas::refreshAccessibility() {
    const QString text = describeSelection();
    if (text == accessibleDescription())
        return;
    setAccessibleDescription(text);
    if (QAccessible::isActive()) {
        QAccessibleEvent event(this, QAccessible::DescriptionChanged);
        QAccessible::updateAccessibility(&event);
    }
}

bool PetuniaNodeCanvas::connectOperation(const QString &operationId, const QString &objectNodeId) {
    const QString asset = assetOf(objectNodeId);
    if (asset.isEmpty() || !isOperation(operationId))
        return false;
    m_controller->request(QStringLiteral("graph_apply"), {{"operation", operationId}, {"target", asset}});
    return true;
}

bool PetuniaNodeCanvas::disconnectOperation(const QString &operationId, const QString &objectNodeId) {
    const QString asset = assetOf(objectNodeId);
    if (asset.isEmpty() || !isOperation(operationId))
        return false;
    m_controller->request(QStringLiteral("graph_unapply"), {{"operation", operationId}, {"target", asset}});
    return true;
}

QPointF PetuniaNodeCanvas::nodeScenePosition(const QString &id) const {
    auto *item = m_nodes.value(id);
    return item ? item->pos() : QPointF();
}

QRectF PetuniaNodeCanvas::nodeSceneRect(const QString &id) const {
    auto *item = m_nodes.value(id);
    return item ? item->mapRectToScene(QRectF(0, 0, kNodeWidth, item->bodyHeight())) : QRectF();
}

QPointF PetuniaNodeCanvas::outputPortScenePos(const QString &id) const {
    auto *item = m_nodes.value(id);
    return item ? item->firstOutputScene() : QPointF();
}

QPointF PetuniaNodeCanvas::inputPortScenePos(const QString &id) const {
    auto *item = m_nodes.value(id);
    return item ? item->firstInputScene() : QPointF();
}

void PetuniaNodeCanvas::fitAll() {
    if (m_nodes.isEmpty())
        return;
    fitInView(m_scene->itemsBoundingRect().adjusted(-30, -30, 30, 30), Qt::KeepAspectRatio);
    if (transform().m11() > 1.25)
        resetTransform();
}

PetuniaNodeItem *PetuniaNodeCanvas::nodeAt(const QPointF &scenePos) const {
    for (auto *item : m_scene->items(scenePos))
        if (auto *node = dynamic_cast<PetuniaNodeItem *>(item))
            return node;
    return nullptr;
}

// ── Entrada ─────────────────────────────────────────────────────────────────

void PetuniaNodeCanvas::moveSelection(int dx, int dy) {
    auto *current = m_nodes.value(m_selected);
    if (!current) {
        if (!m_nodes.isEmpty())
            selectNode(nodeIds().value(0));
        return;
    }
    const QPointF from = current->pos() + QPointF(kNodeWidth / 2, current->bodyHeight() / 2);
    PetuniaNodeItem *best = nullptr;
    qreal bestScore = 1e18;
    for (auto it = m_nodes.cbegin(); it != m_nodes.cend(); ++it) {
        if (it.value() == current)
            continue;
        const QPointF to = it.value()->pos() + QPointF(kNodeWidth / 2, it.value()->bodyHeight() / 2);
        const QPointF d = to - from;
        const qreal along = d.x() * dx + d.y() * dy;
        if (along <= 1)
            continue;
        const qreal across = std::abs(d.x() * dy) + std::abs(d.y() * dx);
        const qreal score = along + across * 2;
        if (score < bestScore) {
            bestScore = score;
            best = it.value();
        }
    }
    if (best)
        selectNode(best->id());
}

void PetuniaNodeCanvas::nudgeSelected(int dx, int dy) {
    auto *item = m_nodes.value(m_selected);
    if (!item)
        return;
    item->setPos(item->pos() + QPointF(dx, dy));
    updateWires();
    m_controller->request(QStringLiteral("graph_move_node"),
                          {{"node", m_selected}, {"x", item->pos().x()}, {"y", item->pos().y()}});
}

void PetuniaNodeCanvas::deleteSelected() {
    if (!isOperation(m_selected))
        return;
    m_controller->request(QStringLiteral("graph_remove"), {{"node", m_selected}});
}

void PetuniaNodeCanvas::keyPressEvent(QKeyEvent *event) {
    const bool ctrl = event->modifiers().testFlag(Qt::ControlModifier);
    int dx = 0, dy = 0;
    switch (event->key()) {
    case Qt::Key_Left: dx = -1; break;
    case Qt::Key_Right: dx = 1; break;
    case Qt::Key_Up: dy = -1; break;
    case Qt::Key_Down: dy = 1; break;
    default: break;
    }
    if (dx || dy) {
        if (ctrl)
            nudgeSelected(dx * 16, dy * 16);
        else
            moveSelection(dx, dy);
        event->accept();
        return;
    }
    switch (event->key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
        if (isOperation(m_selected)) {
            auto *item = m_nodes.value(m_selected);
            showConnectMenu(item ? viewport()->mapToGlobal(mapFromScene(item->pos())) : QCursor::pos());
        }
        event->accept();
        return;
    case Qt::Key_Delete:
    case Qt::Key_Backspace:
        deleteSelected();
        event->accept();
        return;
    case Qt::Key_A:
        if (event->modifiers().testFlag(Qt::ShiftModifier)) {
            showAddMenu(mapToGlobal(QPoint(24, 24)));
            event->accept();
            return;
        }
        break;
    case Qt::Key_Home:
        fitAll();
        event->accept();
        return;
    default:
        break;
    }
    QGraphicsView::keyPressEvent(event);
}

void PetuniaNodeCanvas::showConnectMenu(const QPoint &globalPos) {
    if (!isOperation(m_selected))
        return;
    QMenu menu(this);
    menu.setAccessibleName(tr("Aplicar em"));
    const auto applied = targetsOf(m_selected);
    const QString op = m_selected;
    int count = 0;
    for (const auto &row : m_nodeRows) {
        const auto o = row.toObject();
        if (o["type_id"].toString() != QLatin1String("petunia.object"))
            continue;
        // A própria ferramenta não pode ser alvo da sua operação.
        bool isTool = false;
        for (const auto &edge : m_edgeRows) {
            const auto e = edge.toObject();
            if (e["to"].toString() == op && e["to_port_name"].toString() == QLatin1String("tool") &&
                e["from"].toString() == o["id"].toString())
                isTool = true;
        }
        if (isTool)
            continue;
        auto *action = menu.addAction(o["title"].toString());
        action->setCheckable(true);
        action->setChecked(applied.contains(o["id"].toString()));
        action->setData(o["id"].toString());
        ++count;
    }
    if (count == 0) {
        menu.addAction(tr("Nenhum objeto disponível"))->setEnabled(false);
    }
    if (auto *chosen = menu.exec(globalPos)) {
        const QString target = chosen->data().toString();
        if (chosen->isChecked())
            connectOperation(op, target);
        else
            disconnectOperation(op, target);
    }
}

void PetuniaNodeCanvas::showAddMenu(const QPoint &globalPos) {
    QMenu menu(this);
    menu.setAccessibleName(tr("Adicionar operação"));
    auto *boolean = menu.addAction(PetuniaIcons::get(QStringLiteral("boolean_cut")), tr("Booleana (objeto ativo é a ferramenta)"));
    auto *mirror = menu.addAction(PetuniaIcons::get(QStringLiteral("mirror")), tr("Mirror (espelhar)"));
    auto *symmetry = menu.addAction(PetuniaIcons::get(QStringLiteral("symmetrize")), tr("Symmetry (simetrizar)"));
    menu.setEnabled(!m_activeAsset.isEmpty());
    auto *chosen = menu.exec(globalPos);
    if (chosen == boolean)
        m_controller->request(QStringLiteral("graph_add_boolean"));
    else if (chosen == mirror)
        m_controller->request(QStringLiteral("graph_add_operation"), {{"kind", "mirror"}});
    else if (chosen == symmetry)
        m_controller->request(QStringLiteral("graph_add_operation"), {{"kind", "symmetry"}});
}

void PetuniaNodeCanvas::contextMenuEvent(QContextMenuEvent *event) {
    if (auto *item = nodeAt(mapToScene(event->pos())))
        selectNode(item->id());
    if (isOperation(m_selected)) {
        QMenu menu(this);
        auto *apply = menu.addAction(tr("Aplicar em…"));
        auto *remove = menu.addAction(tr("Remover operação"));
        auto *chosen = menu.exec(event->globalPos());
        if (chosen == apply)
            showConnectMenu(event->globalPos());
        else if (chosen == remove)
            deleteSelected();
    } else {
        showAddMenu(event->globalPos());
    }
    event->accept();
}

void PetuniaNodeCanvas::mousePressEvent(QMouseEvent *event) {
    setFocus(Qt::MouseFocusReason);
    const QPointF scenePos = mapToScene(event->pos());
    if (event->button() == Qt::MiddleButton) {
        m_panning = true;
        m_panLast = event->pos();
        setCursor(Qt::ClosedHandCursor);
        return;
    }
    if (event->button() != Qt::LeftButton) {
        QGraphicsView::mousePressEvent(event);
        return;
    }
    if (auto *node = nodeAt(scenePos)) {
        selectNode(node->id());
        // Arrasto de fio: só saídas de operação conectam (operação → objeto).
        const int output = node->outputAt(scenePos);
        if (output >= 0 && !node->isObject()) {
            m_wireFrom = node->id();
            m_ghost = m_scene->addPath(QPainterPath(), QPen(PetuniaTheme::instance().accentSelectionActive(), 2.4, Qt::DotLine));
            event->accept();
            return;
        }
        m_dragNode = node;
        m_dragOffset = scenePos - node->pos();
        event->accept();
        return;
    }
    event->accept();
}

void PetuniaNodeCanvas::mouseMoveEvent(QMouseEvent *event) {
    if (m_panning) {
        const QPoint d = event->pos() - m_panLast;
        m_panLast = event->pos();
        horizontalScrollBar()->setValue(horizontalScrollBar()->value() - d.x());
        verticalScrollBar()->setValue(verticalScrollBar()->value() - d.y());
        return;
    }
    const QPointF scenePos = mapToScene(event->pos());
    if (m_ghost && !m_wireFrom.isEmpty()) {
        QPainterPath path(m_nodes.value(m_wireFrom)->firstOutputScene());
        const QPointF a = path.currentPosition();
        const qreal dx = std::max<qreal>(40, std::abs(scenePos.x() - a.x()) / 2);
        path.cubicTo(a + QPointF(dx, 0), scenePos - QPointF(dx, 0), scenePos);
        m_ghost->setPath(path);
        return;
    }
    if (m_dragNode) {
        m_dragNode->setPos(scenePos - m_dragOffset);
        updateWires();
        return;
    }
    QGraphicsView::mouseMoveEvent(event);
}

void PetuniaNodeCanvas::mouseReleaseEvent(QMouseEvent *event) {
    if (m_panning && event->button() == Qt::MiddleButton) {
        m_panning = false;
        unsetCursor();
        return;
    }
    const QPointF scenePos = mapToScene(event->pos());
    if (!m_wireFrom.isEmpty()) {
        const QString from = m_wireFrom;
        m_wireFrom.clear();
        delete m_ghost;
        m_ghost = nullptr;
        if (auto *target = nodeAt(scenePos); target && target->isObject())
            connectOperation(from, target->id());
        event->accept();
        return;
    }
    if (m_dragNode) {
        auto *node = m_dragNode;
        m_dragNode = nullptr;
        m_controller->request(QStringLiteral("graph_move_node"),
                              {{"node", node->id()}, {"x", node->pos().x()}, {"y", node->pos().y()}});
        event->accept();
        return;
    }
    QGraphicsView::mouseReleaseEvent(event);
}

void PetuniaNodeCanvas::wheelEvent(QWheelEvent *event) {
    const qreal factor = event->angleDelta().y() > 0 ? 1.15 : 1 / 1.15;
    const qreal next = transform().m11() * factor;
    if (next > 0.3 && next < 3.0)
        scale(factor, factor);
    event->accept();
}

void PetuniaNodeCanvas::focusInEvent(QFocusEvent *event) {
    QGraphicsView::focusInEvent(event);
    if (auto *item = m_nodes.value(m_selected))
        item->setFocused(true);
}

void PetuniaNodeCanvas::focusOutEvent(QFocusEvent *event) {
    QGraphicsView::focusOutEvent(event);
    for (auto *item : std::as_const(m_nodes))
        item->setFocused(false);
}

void PetuniaNodeCanvas::drawBackground(QPainter *painter, const QRectF &rect) {
    const auto &t = PetuniaTheme::instance();
    painter->fillRect(rect, t.bgCanvas());
    painter->setPen(QPen(t.borderSubtle(), 0));
    constexpr int step = 32;
    const int left = int(std::floor(rect.left() / step)) * step;
    const int top = int(std::floor(rect.top() / step)) * step;
    QVector<QPointF> dots;
    for (int x = left; x < rect.right(); x += step)
        for (int y = top; y < rect.bottom(); y += step)
            dots.append(QPointF(x, y));
    painter->drawPoints(dots.constData(), int(dots.size()));
}
