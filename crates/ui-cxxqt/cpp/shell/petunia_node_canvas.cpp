#include "petunia_node_canvas.h"

#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QAccessible>
#include <QAccessibleWidget>
#include <QPointer>
#include <QContextMenuEvent>
#include <QFocusEvent>
#include <QGraphicsObject>
#include <QGraphicsPathItem>
#include <QGraphicsScene>
#include <QJsonArray>
#include <QKeyEvent>
#include <QLineEdit>
#include <QMenu>
#include <QPainter>
#include <QPainterPath>
#include <QResizeEvent>
#include <QScrollBar>
#include <QToolButton>
#include <QSet>
#include <QStyleOptionGraphicsItem>
#include <QWheelEvent>
#include <algorithm>
#include <cmath>

namespace {
QPointF rowPosition(const QJsonObject &row) {
    const auto position = row["position"].toArray();
    return QPointF(position.size() > 0 ? position[0].toDouble() : 0.0, position.size() > 1 ? position[1].toDouble() : 0.0);
}

constexpr qreal kNodeWidth = 176;
constexpr qreal kHeader = 26;
constexpr qreal kRow = 22;
constexpr qreal kPort = 7;

QString typeLabel(const QString &type) {
    if (type == QLatin1String("operation"))
        return petuniaTr(QStringLiteral("Operation"));
    if (type == QLatin1String("texture"))
        return petuniaTr(QStringLiteral("Texture"));
    if (type == QLatin1String("color"))
        return petuniaTr(QStringLiteral("Color"));
    if (type == QLatin1String("value"))
        return petuniaTr(QStringLiteral("Value"));
    return petuniaTr(QStringLiteral("Geometry"));
}

/// Cor de um tipo de porta e de fio: o tipo também se distingue pela forma
/// (círculo, losango, quadrado) e pelo traço do fio; a cor só reforça.
QColor typeColor(const QString &type) {
    const auto &t = PetuniaTheme::instance();
    if (type == QLatin1String("operation"))
        return t.accentSelection();
    if (type == QLatin1String("texture"))
        return t.statusSuccess();
    if (type == QLatin1String("color"))
        return t.statusWarning();
    if (type == QLatin1String("value"))
        return t.textSecondary();
    return t.statusInfo(); // geometria
}

/// Identidade visual de um nó: cor do cabeçalho, ícone e rótulo do papel. Nós de
/// objeto, de operação (por tipo) e de receita de material (por categoria) leem
/// diferente à primeira vista.
struct NodeVisual {
    QColor color;
    QString icon;
    QString role;
};

NodeVisual nodeVisual(const QJsonObject &row) {
    const auto &t = PetuniaTheme::instance();
    const QString typeId = row["type_id"].toString();
    if (typeId == QLatin1String("petunia.object"))
        return {t.textSecondary(), QStringLiteral("cube"), petuniaTr(QStringLiteral("Objeto"))};
    if (typeId == QLatin1String("petunia.boolean"))
        return {t.accentSelection(), QStringLiteral("boolean_cut"), petuniaTr(QStringLiteral("Booleana"))};
    if (typeId == QLatin1String("petunia.mirror"))
        return {t.statusInfo(), QStringLiteral("mirror"), petuniaTr(QStringLiteral("Espelho"))};
    if (typeId == QLatin1String("petunia.symmetry"))
        return {t.statusInfo(), QStringLiteral("symmetrize"), petuniaTr(QStringLiteral("Simetria"))};
    const QString category = row["category"].toString();
    if (typeId.startsWith(QLatin1String("recipe."))) {
        if (category == QLatin1String("input"))
            return {t.statusInfo(), QStringLiteral("reference_image"), petuniaTr(QStringLiteral("Entrada"))};
        if (category == QLatin1String("pattern"))
            return {t.accentPrimary(), QStringLiteral("paint_gradient"), petuniaTr(QStringLiteral("Gerador"))};
        if (category == QLatin1String("color"))
            return {t.statusWarning(), QStringLiteral("paint_palette"), petuniaTr(QStringLiteral("Cor"))};
        if (category == QLatin1String("output"))
            return {t.statusSuccess(), QStringLiteral("output"), petuniaTr(QStringLiteral("Saída"))};
    }
    return {t.accentPrimary(), QStringLiteral("modifier"), petuniaTr(QStringLiteral("Operação"))};
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
        setToolTip(portSummary());
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
    QPointF inputScene(int index) const { return mapToScene(inputPos(index)); }
    QPointF outputScene(int index) const { return mapToScene(outputPos(index)); }
    int inputCount() const { return int(m_inputs.size()); }
    int outputCount() const { return int(m_outputs.size()); }
    QString inputType(int index) const { return m_inputs.at(index).toObject()["type"].toString(); }
    QString outputType(int index) const { return m_outputs.at(index).toObject()["type"].toString(); }
    QString inputLabel(int index) const { return petuniaTr(m_inputs.at(index).toObject()["label"].toString()); }

    /// Índice da entrada sob o ponto (coordenadas da cena), ou -1.
    int inputAt(const QPointF &scenePos) const {
        const QPointF local = mapFromScene(scenePos);
        for (int i = 0; i < m_inputs.size(); ++i)
            if (QLineF(local, inputPos(i)).length() <= kPort + 6)
                return i;
        return -1;
    }

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
        // Cabeçalho: o papel do nó não depende só de cor (ícone + rótulo do papel).
        const NodeVisual visual = nodeVisual(m_row);
        p->save();
        p->setClipRect(QRectF(0, 0, kNodeWidth, kHeader));
        p->setPen(Qt::NoPen);
        p->setBrush(isObject() ? t.bgSurfaceActive() : t.bgPanelHeader());
        p->drawRoundedRect(body, 7, 7);
        QColor tint = visual.color;
        tint.setAlpha(enabled ? 64 : 24);
        p->setBrush(tint);
        p->drawRoundedRect(body, 7, 7);
        p->setBrush(enabled ? visual.color : t.textMuted());
        p->drawRect(QRectF(0, 0, 4, kHeader));
        p->restore();
        const QIcon roleIcon = PetuniaIcons::get(visual.icon, (enabled ? visual.color : t.textMuted()).name());
        roleIcon.paint(p, QRect(10, int((kHeader - 16) / 2), 16, 16));
        QFont f = p->font();
        f.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Body));
        f.setBold(true);
        p->setFont(f);
        p->setPen(enabled ? t.textPrimary() : t.textMuted());
        QString title = m_row["title"].toString();
        if (!enabled)
            title += QStringLiteral(" (off)");
        const bool hasError = !m_row["error"].isNull() && !m_row["error"].toString().isEmpty();
        // O rótulo do papel fica à direita (some quando falta espaço ou há erro).
        QFont roleFont = f;
        roleFont.setBold(false);
        roleFont.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Caption));
        const int roleWidth = hasError ? 0 : QFontMetrics(roleFont).horizontalAdvance(visual.role);
        const qreal titleRight = kNodeWidth - 10 - (roleWidth > 0 ? roleWidth + 8 : 0) - (hasError ? 18 : 0);
        p->drawText(QRectF(32, 0, titleRight - 32, kHeader), Qt::AlignVCenter | Qt::AlignLeft,
                    QFontMetrics(f).elidedText(title, Qt::ElideRight, int(titleRight - 32)));
        if (roleWidth > 0 && titleRight - 32 > 56) {
            p->setFont(roleFont);
            p->setPen(t.textMuted());
            p->drawText(QRectF(kNodeWidth - 10 - roleWidth, 0, roleWidth, kHeader), Qt::AlignVCenter | Qt::AlignRight,
                        visual.role);
        }
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
            p->setFont(f);
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
        const QString type = port["type"].toString();
        // A forma distingue o tipo (nunca só a cor): círculo vazado = geometria,
        // losango = operação, quadrado arredondado = textura, círculo cheio = cor,
        // quadrado pequeno = valor.
        const QColor color = typeColor(type);
        p->setPen(QPen(color, 2));
        if (type == QLatin1String("geometry")) {
            p->setBrush(t.bgSurface());
            p->drawEllipse(pos, kPort, kPort);
        } else if (type == QLatin1String("texture")) {
            p->setBrush(color);
            p->drawRoundedRect(QRectF(pos.x() - kPort, pos.y() - kPort, 2 * kPort, 2 * kPort), 3, 3);
        } else if (type == QLatin1String("color")) {
            p->setBrush(color);
            p->drawEllipse(pos, kPort, kPort);
        } else if (type == QLatin1String("value")) {
            p->setBrush(color);
            p->drawRect(QRectF(pos.x() - kPort + 2, pos.y() - kPort + 2, 2 * kPort - 4, 2 * kPort - 4));
        } else {
            p->setBrush(color);
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

    /// Dica do nó: papel e os tipos de cada entrada/saída (forma + nome do tipo).
    QString portSummary() const {
        auto describe = [](const QJsonArray &ports) {
            QStringList parts;
            for (const auto &value : ports) {
                const auto port = value.toObject();
                parts << QStringLiteral("%1 (%2)").arg(petuniaTr(port["label"].toString()), typeLabel(port["type"].toString()));
            }
            return parts.isEmpty() ? petuniaTr(QStringLiteral("nenhuma")) : parts.join(QStringLiteral(", "));
        };
        return QStringLiteral("%1\n%2: %3\n%4: %5")
            .arg(nodeVisual(m_row).role, petuniaTr(QStringLiteral("Entradas")), describe(m_inputs),
                 petuniaTr(QStringLiteral("Saídas")), describe(m_outputs));
    }

    QString summaryText() const {
        const auto params = m_row["params"].toObject();
        QStringList parts;
        if (params.contains(QStringLiteral("operation")))
            parts << petuniaChoiceLabel(params["operation"].toString());
        if (params.contains(QStringLiteral("axis")))
            parts << params["axis"].toString().toUpper();
        if (params.contains(QStringLiteral("channel")))
            parts << petuniaChoiceLabel(params["channel"].toString());
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
    PetuniaWireItem(const QString &from, int fromPort, const QString &to, int toPort, const QString &type)
        : m_from(from), m_to(to), m_type(type), m_fromPort(fromPort), m_toPort(toPort) {
        setZValue(-1);
    }
    const QString &from() const { return m_from; }
    const QString &to() const { return m_to; }
    int fromPort() const { return m_fromPort; }
    int toPort() const { return m_toPort; }
    const QString &portType() const { return m_type; }

    void connectPorts(const QPointF &a, const QPointF &b) {
        QPainterPath path(a);
        const qreal dx = std::max<qreal>(40, std::abs(b.x() - a.x()) / 2);
        path.cubicTo(a + QPointF(dx, 0), b - QPointF(dx, 0), b);
        setPath(path);
        // Cada tipo tem cor e traço próprios (geometria tracejado, valor ponto e
        // traço, operação contínuo grosso): o tipo nunca depende só de cor.
        QPen pen(typeColor(m_type), m_type == QLatin1String("operation") ? 2.8 : 2.0);
        if (m_type == QLatin1String("geometry"))
            pen.setStyle(Qt::DashLine);
        else if (m_type == QLatin1String("value"))
            pen.setStyle(Qt::DashDotLine);
        pen.setCapStyle(Qt::RoundCap);
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
    int m_fromPort{0}, m_toPort{0};
    QPointF m_mid;
};

// ── Acessibilidade ──────────────────────────────────────────────────────────

/// Cada nó é um item acessível (ListItem) filho do canvas: nome, descrição com as
/// ligações, estado (selecionável/selecionado/desativado) e retângulo na tela.
class PetuniaNodeAccessible : public QAccessibleInterface {
  public:
    PetuniaNodeAccessible(PetuniaNodeCanvas *canvas, const QString &id) : m_canvas(canvas), m_id(id) {}

    bool isValid() const override { return !m_canvas.isNull() && m_canvas->nodeIds().contains(m_id); }
    QObject *object() const override { return nullptr; }
    QAccessibleInterface *parent() const override { return QAccessible::queryAccessibleInterface(m_canvas); }
    QAccessibleInterface *child(int) const override { return nullptr; }
    int childCount() const override { return 0; }
    int indexOfChild(const QAccessibleInterface *) const override { return -1; }
    QAccessibleInterface *childAt(int, int) const override { return nullptr; }
    QAccessibleInterface *focusChild() const override { return nullptr; }
    QList<QPair<QAccessibleInterface *, QAccessible::Relation>> relations(QAccessible::Relation) const override { return {}; }
    QRect rect() const override { return m_canvas ? m_canvas->nodeGlobalRect(m_id) : QRect(); }
    QAccessible::Role role() const override { return QAccessible::ListItem; }
    QString text(QAccessible::Text t) const override {
        if (!m_canvas)
            return {};
        switch (t) {
        case QAccessible::Name: return m_canvas->nodeTitle(m_id);
        case QAccessible::Description: return m_canvas->describeNode(m_id);
        default: return {};
        }
    }
    void setText(QAccessible::Text, const QString &) override {}
    QAccessible::State state() const override {
        QAccessible::State s;
        if (!m_canvas)
            return s;
        s.focusable = true;
        s.selectable = true;
        const bool selected = m_canvas->selectedNodeId() == m_id;
        s.selected = selected;
        s.focused = selected && m_canvas->hasFocus();
        s.disabled = !m_canvas->nodeEnabled(m_id);
        return s;
    }

  private:
    QPointer<PetuniaNodeCanvas> m_canvas;
    QString m_id;
};

/// O canvas expõe só os nós como filhos (barras de rolagem não são conteúdo).
class PetuniaCanvasAccessible : public QAccessibleWidget {
  public:
    explicit PetuniaCanvasAccessible(PetuniaNodeCanvas *canvas) : QAccessibleWidget(canvas, QAccessible::List), m_canvas(canvas) {}
    int childCount() const override { return m_canvas->nodeIds().size(); }
    QAccessibleInterface *child(int index) const override {
        const auto ids = m_canvas->nodeIds();
        return index >= 0 && index < ids.size() ? m_canvas->accessibleNode(ids.at(index)) : nullptr;
    }
    int indexOfChild(const QAccessibleInterface *child) const override {
        const auto ids = m_canvas->nodeIds();
        for (int i = 0; i < ids.size(); ++i)
            if (m_canvas->accessibleNode(ids.at(i)) == child)
                return i;
        return -1;
    }
    QAccessibleInterface *focusChild() const override {
        return m_canvas->selectedNodeId().isEmpty() ? nullptr : m_canvas->accessibleNode(m_canvas->selectedNodeId());
    }
    QAccessibleInterface *childAt(int, int) const override { return nullptr; }
    QString text(QAccessible::Text t) const override {
        if (t == QAccessible::Description)
            return m_canvas->describeSelection();
        return QAccessibleWidget::text(t);
    }

  private:
    PetuniaNodeCanvas *m_canvas;
};

namespace {
QAccessibleInterface *canvasAccessibleFactory(const QString &className, QObject *object) {
    if (className == QLatin1String("PetuniaNodeCanvas"))
        if (auto *canvas = qobject_cast<PetuniaNodeCanvas *>(object))
            return new PetuniaCanvasAccessible(canvas);
    return nullptr;
}
} // namespace

QAccessibleInterface *PetuniaNodeCanvas::accessibleNode(const QString &id) {
    if (!m_nodes.contains(id))
        return nullptr;
    auto it = m_accessible.find(id);
    if (it == m_accessible.end())
        it = m_accessible.insert(id, QAccessible::registerAccessibleInterface(new PetuniaNodeAccessible(this, id)));
    return QAccessible::accessibleInterface(it.value());
}

void PetuniaNodeCanvas::releaseStaleAccessibles() {
    for (auto it = m_accessible.begin(); it != m_accessible.end();) {
        if (m_nodes.contains(it.key())) {
            ++it;
            continue;
        }
        QAccessible::deleteAccessibleInterface(it.value());
        it = m_accessible.erase(it);
    }
}

QString PetuniaNodeCanvas::nodeTitle(const QString &id) const {
    const auto row = nodeRow(id);
    return row.isEmpty() ? QString() : petuniaTr(row["label"].toString()) + QStringLiteral(" · ") + row["title"].toString();
}

bool PetuniaNodeCanvas::nodeEnabled(const QString &id) const {
    return nodeRow(id)["enabled"].toBool(true);
}

QRect PetuniaNodeCanvas::nodeGlobalRect(const QString &id) const {
    const QRectF scene = nodeSceneRect(id);
    if (scene.isEmpty())
        return {};
    const QRect local = mapFromScene(scene).boundingRect();
    return QRect(viewport()->mapToGlobal(local.topLeft()), local.size());
}

// ── Canvas ──────────────────────────────────────────────────────────────────

PetuniaNodeCanvas::PetuniaNodeCanvas(PetuniaShellController *controller, QWidget *parent)
    : QGraphicsView(parent), m_controller(controller), m_scene(new QGraphicsScene(this)) {
    setScene(m_scene);
    setRenderHint(QPainter::Antialiasing);
    setFocusPolicy(Qt::StrongFocus);
    setFrameShape(QFrame::NoFrame);
    setDragMode(QGraphicsView::NoDrag);
    // O zoom é ancorado à mão (zoomBy): sem barras de rolagem a âncora do Qt não vale.
    setTransformationAnchor(QGraphicsView::NoAnchor);
    static const bool factoryInstalled = (QAccessible::installFactory(canvasAccessibleFactory), true);
    Q_UNUSED(factoryInstalled);
    setAccessibleName(tr("Editor de nós"));
    // A área interna recebe o foco do QGraphicsView; ela também se anuncia.
    viewport()->setAccessibleName(tr("Editor de nós"));
    setToolTip(tr("Nós: setas navegam, Enter aplica em…, Delete remove, Ctrl+setas movem, Shift+A adiciona"));
    // Vista infinita: a cena é enorme e sem barras de rolagem; o pan é por arrasto
    // (botão do meio, direito ou Espaço+esquerdo), roda/trackpad e Alt+setas.
    m_scene->setSceneRect(-100000, -100000, 200000, 200000);
    setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    // A legenda de tipos é desenhada em primeiro plano, fixa na vista. Com a
    // atualização mínima, o Qt desloca os pixels ao rolar e só repinta a faixa
    // exposta: a legenda "andava" com o pan. O grafo é pequeno, repinta tudo.
    setViewportUpdateMode(QGraphicsView::FullViewportUpdate);

    m_fitButton = new QToolButton(viewport());
    m_fitButton->setAutoRaise(true);
    m_fitButton->setFocusPolicy(Qt::TabFocus);
    m_fitButton->setIcon(PetuniaIcons::get(QStringLiteral("bounding_box")));
    m_fitButton->setToolTip(tr("Enquadrar tudo (Home ou F)"));
    m_fitButton->setAccessibleName(tr("Enquadrar tudo"));
    connect(m_fitButton, &QToolButton::clicked, this, [this] { fitAll(); });
    placeFitButton();
}

void PetuniaNodeCanvas::placeFitButton() {
    if (!m_fitButton)
        return;
    const int side = 28;
    m_fitButton->setFixedSize(side, side);
    m_fitButton->move(viewport()->width() - side - 8, 8);
    m_fitButton->raise();
}

void PetuniaNodeCanvas::resizeEvent(QResizeEvent *event) {
    QGraphicsView::resizeEvent(event);
    placeFitButton();
}

void PetuniaNodeCanvas::panBy(const QPointF &delta) {
    if (!std::isfinite(delta.x()) || !std::isfinite(delta.y()))
        return;
    horizontalScrollBar()->setValue(horizontalScrollBar()->value() - qRound(delta.x()));
    verticalScrollBar()->setValue(verticalScrollBar()->value() - qRound(delta.y()));
}

void PetuniaNodeCanvas::zoomBy(qreal factor, const QPointF &anchorInView) {
    if (!std::isfinite(factor) || factor <= 0)
        return;
    const qreal next = transform().m11() * factor;
    if (next <= 0.3 || next >= 3.0)
        return;
    const QPointF before = mapToScene(anchorInView.toPoint());
    scale(factor, factor);
    // Devolve ao ponto de ancoragem o ponto da cena que estava sob ele.
    panBy(anchorInView - QPointF(mapFromScene(before)));
}

PetuniaNodeCanvas::~PetuniaNodeCanvas() {
    for (auto id : std::as_const(m_accessible))
        QAccessible::deleteAccessibleInterface(id);
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

void PetuniaNodeCanvas::normalizeRecipe(const QJsonObject &recipe, QJsonArray &nodes, QJsonArray &edges) const {
    // A receita usa soquetes indexados e tipos (textura/cor/valor): vira o mesmo
    // formato de linhas do Operation Graph para reaproveitar layout, a11y e teclado.
    auto ports = [](const QJsonArray &types, const QString &prefix) {
        QJsonArray out;
        for (int i = 0; i < types.size(); ++i) {
            const QString type = types[i].toString();
            QString label = type.left(1).toUpper() + type.mid(1);
            if (types.size() > 1)
                label += QLatin1Char(' ') + QChar(QLatin1Char('A').unicode() + i);
            out.append(QJsonObject{{"name", prefix + QString::number(i)}, {"label", label}, {"type", type}});
        }
        return out;
    };
    for (const auto &value : recipe["nodes"].toArray()) {
        auto row = value.toObject();
        row["title"] = petuniaTr(row["label"].toString());
        row["type_id"] = QStringLiteral("recipe.") + row["kind"].toString();
        row["inputs"] = ports(row["inputs"].toArray(), QStringLiteral("in"));
        row["outputs"] = ports(row["outputs"].toArray(), QStringLiteral("out"));
        nodes.append(row);
    }
    for (const auto &value : recipe["edges"].toArray()) {
        const auto e = value.toObject();
        edges.append(QJsonObject{{"from", e["from"]},
                                 {"to", e["to"]},
                                 {"from_port", e["from_socket"]},
                                 {"to_port", e["to_socket"]},
                                 {"from_port_name", QStringLiteral("out") + QString::number(e["from_socket"].toInt())},
                                 {"to_port_name", QStringLiteral("in") + QString::number(e["to_socket"].toInt())},
                                 {"type", e["type"]}});
    }
}

void PetuniaNodeCanvas::setDomain(Domain domain) {
    if (domain == m_domain)
        return;
    m_domain = domain;
    m_selected.clear();
    m_signature.clear();
    m_viewFitted = false; // cada grafo enquadra na primeira vez que aparece
    setToolTip(domain == Domain::Material
                   ? tr("Nós do material: setas navegam, Enter liga a…, Delete remove, Shift+A adiciona")
                   : tr("Nós: setas navegam, Enter aplica em…, Delete remove, Ctrl+setas movem, Shift+A adiciona"));
    if (!m_state.isEmpty())
        sync(m_state);
}

void PetuniaNodeCanvas::sync(const QJsonObject &state) {
    m_state = state;
    QJsonArray nodes;
    QJsonArray edges;
    if (m_domain == Domain::Material) {
        const auto recipe = state["material_recipe"].toObject();
        m_hasRecipe = recipe["has_recipe"].toBool();
        m_catalog = recipe["catalog"].toArray();
        normalizeRecipe(recipe, nodes, edges);
    } else {
        const auto graph = state["op_graph"].toObject();
        nodes = graph["nodes"].toArray();
        edges = graph["edges"].toArray();
    }
    m_activeAsset = state["active_asset_id"].toString();
    QString signature = m_domain == Domain::Material ? QStringLiteral("M|") : QStringLiteral("O|");
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
    // Grupos de nós (só no grafo de operações): quadros nomeados em volta dos membros.
    m_groups.clear();
    if (m_domain == Domain::Operations) {
        for (const auto &value : state["op_graph"].toObject()["groups"].toArray()) {
            const auto row = value.toObject();
            GroupRow group;
            group.id = row["id"].toString();
            group.name = row["name"].toString();
            for (const auto &member : row["nodes"].toArray())
                group.nodes << member.toString();
            if (!group.id.isEmpty() && !group.nodes.isEmpty())
                m_groups.append(group);
        }
    }
    if (signature != m_signature) {
        m_signature = signature;
        rebuild(nodes, edges);
    } else {
        // Mesma estrutura: atualiza só dados (parâmetros, posição, ativo) sem recriar itens.
        for (const auto &n : nodes) {
            const auto row = n.toObject();
            const QString id = row["id"].toString();
            auto *item = m_nodes.value(id);
            if (!item)
                continue;
            item->setRow(row);
            // A posição do documento mudou sem gesto deste canvas (Undo/Redo): segue-a.
            const QPointF server = rowPosition(row);
            if (m_serverPositions.contains(id) && m_serverPositions.value(id) != server &&
                !m_dragOrigins.contains(id) && item != m_dragNode)
                item->setPos(server);
            m_serverPositions.insert(id, server);
        }
    }
    for (auto it = m_nodes.begin(); it != m_nodes.end(); ++it)
        it.value()->setActive(m_domain == Domain::Operations && !m_activeAsset.isEmpty() &&
                              assetOf(it.key()) == m_activeAsset);
    updateWires();
    refreshAccessibility();
    viewport()->update(); // quadros dos grupos e seleção
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
        int column = 1;
        if (m_domain == Domain::Material) {
            const QString category = row["category"].toString();
            column = category == QLatin1String("output") ? 2
                     : (category == QLatin1String("input") || category == QLatin1String("pattern")) ? 0
                                                                                                     : 1;
        } else {
            const bool op = row["type_id"].toString() != QLatin1String("petunia.object");
            column = op ? 1 : (toolObjects.contains(id) ? 0 : 2);
        }
        const QPointF fallback(column * 260.0, rowInColumn[column]++ * 120.0);
        QPointF pos = rowPosition(row);
        m_serverPositions.insert(id, pos);
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
        auto *wire = new PetuniaWireItem(e["from"].toString(), e["from_port"].toInt(), e["to"].toString(),
                                         e["to_port"].toInt(), e["type"].toString());
        m_scene->addItem(wire);
        m_wires.append(wire);
    }
    // Seleção que sobreviveu à reconstrução (nós que sumiram saem dela).
    for (auto it = m_multi.begin(); it != m_multi.end();)
        it = m_nodes.contains(*it) ? std::next(it) : m_multi.erase(it);
    applySelectionVisuals();
    m_rebuilding = false;
    releaseStaleAccessibles();
    // A primeira vez que um grafo aparece, enquadra tudo; depois a vista é do usuário.
    if (!m_viewFitted && !m_nodes.isEmpty()) {
        m_viewFitted = true;
        fitAll();
    }
}

void PetuniaNodeCanvas::updateWires() {
    for (auto *wire : m_wires) {
        auto *from = m_nodes.value(wire->from());
        auto *to = m_nodes.value(wire->to());
        if (!from || !to)
            continue;
        // Fio de operação: saída da operação → entrada "operations" do objeto.
        // Fio de geometria: saída "geometry" do objeto-ferramenta → entrada "tool".
        const int out = std::clamp(wire->fromPort(), 0, std::max(0, from->outputCount() - 1));
        const int in = std::clamp(wire->toPort(), 0, std::max(0, to->inputCount() - 1));
        wire->connectPorts(from->outputScene(out), to->inputScene(in));
    }
}

void PetuniaNodeCanvas::selectNode(const QString &id) {
    if (!m_nodes.contains(id))
        return;
    m_selected = id;
    m_multi = {id};
    applySelectionVisuals();
    if (auto *item = m_nodes.value(id))
        ensureVisible(item, 40, 40);
    refreshAccessibility();
    emit nodeSelected(id);
}

void PetuniaNodeCanvas::applySelectionVisuals() {
    for (auto it = m_nodes.begin(); it != m_nodes.end(); ++it)
        it.value()->setSelectedNode(m_multi.contains(it.key()) || it.key() == m_selected);
    viewport()->update();
}

QStringList PetuniaNodeCanvas::selectedNodeIds() const {
    QStringList ids;
    if (m_nodes.contains(m_selected))
        ids << m_selected;
    for (const auto &id : m_multi)
        if (m_nodes.contains(id) && !ids.contains(id))
            ids << id;
    return ids;
}

void PetuniaNodeCanvas::selectNodes(const QStringList &ids) {
    QSet<QString> valid;
    for (const auto &id : ids)
        if (m_nodes.contains(id))
            valid.insert(id);
    m_multi = valid;
    m_selected = valid.contains(m_selected) ? m_selected : (ids.isEmpty() ? QString() : (valid.contains(ids.first()) ? ids.first() : *valid.begin()));
    if (valid.isEmpty())
        m_selected.clear();
    applySelectionVisuals();
    refreshAccessibility();
    if (!m_selected.isEmpty())
        emit nodeSelected(m_selected);
}

QString PetuniaNodeCanvas::groupId(int index) const {
    return index >= 0 && index < m_groups.size() ? m_groups.at(index).id : QString();
}
QString PetuniaNodeCanvas::groupName(int index) const {
    return index >= 0 && index < m_groups.size() ? m_groups.at(index).name : QString();
}
QStringList PetuniaNodeCanvas::groupMembers(int index) const {
    return index >= 0 && index < m_groups.size() ? m_groups.at(index).nodes : QStringList();
}

/// Retângulo do quadro: envolve os membros, com folga e uma faixa de título.
QRectF PetuniaNodeCanvas::groupSceneRect(int index) const {
    QRectF box;
    for (const auto &id : m_groups.at(index).nodes)
        if (auto *item = m_nodes.value(id))
            box = box.isNull() ? item->sceneBoundingRect() : box.united(item->sceneBoundingRect());
    return box.isNull() ? QRectF() : box.adjusted(-18, -34, 18, 18);
}
QRectF PetuniaNodeCanvas::groupTitleSceneRect(int index) const {
    const QRectF box = groupSceneRect(index);
    return box.isNull() ? QRectF() : QRectF(box.left(), box.top(), box.width(), 26);
}
int PetuniaNodeCanvas::groupAtTitle(const QPointF &scenePos) const {
    for (int i = int(m_groups.size()) - 1; i >= 0; --i)
        if (groupTitleSceneRect(i).contains(scenePos))
            return i;
    return -1;
}
QRect PetuniaNodeCanvas::nodeViewportRect(const QString &id) const {
    auto *item = m_nodes.value(id);
    return item ? mapFromScene(item->sceneBoundingRect()).boundingRect() : QRect();
}
QRect PetuniaNodeCanvas::groupTitleViewportRect(int index) const {
    if (index < 0 || index >= m_groups.size())
        return QRect();
    return mapFromScene(groupTitleSceneRect(index)).boundingRect();
}

void PetuniaNodeCanvas::selectGroupMembers(int index) {
    if (index < 0 || index >= m_groups.size())
        return;
    selectNodes(m_groups.at(index).nodes);
}

bool PetuniaNodeCanvas::groupSelection() {
    if (m_domain != Domain::Operations)
        return false;
    const QStringList ids = selectedNodeIds();
    if (ids.isEmpty())
        return false;
    QJsonArray nodes;
    for (const auto &id : ids)
        nodes.append(id);
    m_controller->request(QStringLiteral("graph_group"), {{"nodes", nodes}});
    return true;
}

bool PetuniaNodeCanvas::ungroupSelection() {
    if (m_domain != Domain::Operations)
        return false;
    for (const auto &id : selectedNodeIds())
        for (const auto &group : std::as_const(m_groups))
            if (group.nodes.contains(id)) {
                m_controller->request(QStringLiteral("graph_ungroup"), {{"group", group.id}});
                return true;
            }
    return false;
}

/// Editor de texto sobre o título do grupo: Enter confirma, Esc cancela.
void PetuniaNodeCanvas::beginRenameGroup(const QString &groupId) {
    int index = -1;
    for (int i = 0; i < m_groups.size(); ++i)
        if (m_groups.at(i).id == groupId)
            index = i;
    if (index < 0 || m_domain != Domain::Operations)
        return;
    finishRename(false);
    const QRect title = groupTitleViewportRect(index);
    m_renamingGroup = groupId;
    m_renameEditor = new QLineEdit(viewport());
    m_renameEditor->setAccessibleName(tr("Nome do grupo de nós"));
    m_renameEditor->setText(m_groups.at(index).name);
    m_renameEditor->setGeometry(title.adjusted(6, 2, -6, -2));
    m_renameEditor->show();
    m_renameEditor->selectAll();
    m_renameEditor->setFocus(Qt::OtherFocusReason);
    connect(m_renameEditor, &QLineEdit::returnPressed, this, [this] { finishRename(true); });
    m_renameEditor->installEventFilter(this);
}

void PetuniaNodeCanvas::finishRename(bool commit) {
    if (!m_renameEditor)
        return;
    QLineEdit *editor = m_renameEditor;
    const QString group = m_renamingGroup;
    const QString text = editor->text().trimmed();
    m_renameEditor = nullptr;
    m_renamingGroup.clear();
    editor->removeEventFilter(this);
    editor->hide();
    editor->deleteLater();
    if (commit && !text.isEmpty())
        m_controller->request(QStringLiteral("graph_rename_group"), {{"group", group}, {"name", text}});
    setFocus(Qt::OtherFocusReason);
}

bool PetuniaNodeCanvas::eventFilter(QObject *watched, QEvent *event) {
    if (m_renameEditor && watched == m_renameEditor) {
        if (event->type() == QEvent::KeyPress && static_cast<QKeyEvent *>(event)->key() == Qt::Key_Escape) {
            finishRename(false);
            return true;
        }
        if (event->type() == QEvent::FocusOut) {
            finishRename(true); // sair do campo confirma o nome (vazio é descartado)
            return false;
        }
    }
    return QGraphicsView::eventFilter(watched, event);
}

void PetuniaNodeCanvas::mouseDoubleClickEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && m_domain == Domain::Operations) {
        const int index = groupAtTitle(mapToScene(event->pos()));
        if (index >= 0) {
            beginRenameGroup(m_groups.at(index).id);
            event->accept();
            return;
        }
    }
    QGraphicsView::mouseDoubleClickEvent(event);
}

QString PetuniaNodeCanvas::describeNode(const QString &id) const {
    const auto row = nodeRow(id);
    if (row.isEmpty())
        return tr("Nenhum nó. Shift+A adiciona uma operação ao objeto ativo.");
    if (m_domain == Domain::Material) {
        QStringList sources, targets;
        for (const auto &edge : m_edgeRows) {
            const auto e = edge.toObject();
            if (e["to"].toString() == id)
                sources << nodeRow(e["from"].toString())["title"].toString();
            if (e["from"].toString() == id)
                targets << nodeRow(e["to"].toString())["title"].toString();
        }
        QString text = row["title"].toString();
        text += sources.isEmpty() ? tr(". Sem entradas ligadas.") : tr(". Recebe de: %1.").arg(sources.join(QStringLiteral(", ")));
        text += targets.isEmpty() ? tr(" Saída livre.") : tr(" Envia para: %1.").arg(targets.join(QStringLiteral(", ")));
        return text;
    }
    QString text = petuniaTr(row["label"].toString()) + QStringLiteral(": ") + row["title"].toString();
    if (row["type_id"].toString() == QLatin1String("petunia.object")) {
        QStringList ops;
        for (const auto &edge : m_edgeRows) {
            const auto e = edge.toObject();
            if (e["to"].toString() == id && e["to_port_name"].toString() == QLatin1String("operations"))
                ops << nodeRow(e["from"].toString())["title"].toString();
        }
        text += ops.isEmpty() ? tr(". Sem operações aplicadas.") : tr(". Operações: %1.").arg(ops.join(QStringLiteral(", ")));
    } else {
        QStringList targets;
        for (const auto &target : targetsOf(id))
            targets << nodeRow(target)["title"].toString();
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
        const int index = nodeIds().indexOf(m_selected);
        if (index >= 0 && hasFocus()) {
            QAccessibleEvent focus(this, QAccessible::Focus);
            focus.setChild(index);
            QAccessible::updateAccessibility(&focus);
        }
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

bool PetuniaNodeCanvas::connectSockets(const QString &from, int fromSocket, const QString &to, int toSocket) {
    if (m_domain != Domain::Material || from == to || !m_nodes.contains(from) || !m_nodes.contains(to))
        return false;
    m_controller->request(QStringLiteral("material_recipe_connect"),
                          {{"from", from}, {"from_socket", fromSocket}, {"to", to}, {"to_socket", toSocket}});
    return true;
}

bool PetuniaNodeCanvas::disconnectSocket(const QString &to, int toSocket) {
    if (m_domain != Domain::Material || !m_nodes.contains(to))
        return false;
    m_controller->request(QStringLiteral("material_recipe_disconnect"), {{"to", to}, {"to_socket", toSocket}});
    return true;
}

QPointF PetuniaNodeCanvas::inputSocketScenePos(const QString &id, int socket) const {
    auto *item = m_nodes.value(id);
    return item ? item->inputScene(socket) : QPointF();
}

QPointF PetuniaNodeCanvas::outputSocketScenePos(const QString &id, int socket) const {
    auto *item = m_nodes.value(id);
    return item ? item->outputScene(socket) : QPointF();
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
    m_controller->request(m_domain == Domain::Material ? QStringLiteral("material_recipe_move")
                                                       : QStringLiteral("graph_move_node"),
                          {{"node", m_selected}, {"x", item->pos().x()}, {"y", item->pos().y()}});
}

void PetuniaNodeCanvas::deleteSelected() {
    QStringList targets = selectedNodeIds();
    if (targets.isEmpty())
        return;
    if (m_domain == Domain::Material) {
        for (const auto &id : targets)
            if (m_nodes.contains(id))
                m_controller->request(QStringLiteral("material_recipe_remove"), {{"node", id}});
        return;
    }
    for (const auto &id : targets)
        if (isOperation(id))
            m_controller->request(QStringLiteral("graph_remove"), {{"node", id}});
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
        if (event->modifiers().testFlag(Qt::AltModifier)) {
            panBy(QPointF(-dx * 80, -dy * 80)); // pan pelo teclado
        } else if (ctrl)
            nudgeSelected(dx * 16, dy * 16);
        else
            moveSelection(dx, dy);
        event->accept();
        return;
    }
    switch (event->key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
        if (m_domain == Domain::Material && m_nodes.contains(m_selected)) {
            auto *item = m_nodes.value(m_selected);
            showRecipeConnectMenu(viewport()->mapToGlobal(mapFromScene(item->pos())));
        } else if (isOperation(m_selected)) {
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
        if (ctrl && !event->modifiers().testFlag(Qt::ShiftModifier)) {
            selectNodes(m_nodes.keys()); // Ctrl+A: todos os nós
            event->accept();
            return;
        }
        if (event->modifiers().testFlag(Qt::ShiftModifier)) {
            showAddMenu(mapToGlobal(QPoint(24, 24)));
            event->accept();
            return;
        }
        break;
    case Qt::Key_G:
        if (ctrl) {
            if (event->modifiers().testFlag(Qt::ShiftModifier))
                ungroupSelection();
            else
                groupSelection();
            event->accept();
            return;
        }
        break;
    case Qt::Key_F2:
        for (const auto &group : std::as_const(m_groups))
            if (group.nodes.contains(m_selected)) {
                beginRenameGroup(group.id);
                break;
            }
        event->accept();
        return;
    case Qt::Key_Home:
        fitAll();
        event->accept();
        return;
    case Qt::Key_F:
        if (event->modifiers() == Qt::NoModifier) {
            fitAll();
            event->accept();
            return;
        }
        break;
    case Qt::Key_Plus:
    case Qt::Key_Equal:
        zoomBy(1.15, QPointF(viewport()->rect().center()));
        event->accept();
        return;
    case Qt::Key_Minus:
        zoomBy(1 / 1.15, QPointF(viewport()->rect().center()));
        event->accept();
        return;
    case Qt::Key_Space:
        if (!event->isAutoRepeat()) {
            m_spaceHeld = true;
            if (!m_panning)
                setCursor(Qt::OpenHandCursor);
        }
        event->accept();
        return;
    default:
        break;
    }
    QGraphicsView::keyPressEvent(event);
}

bool PetuniaNodeCanvas::ownsKey(const QKeyEvent *event) {
    const Qt::KeyboardModifiers mods = event->modifiers() & ~Qt::KeypadModifier;
    switch (event->key()) {
    case Qt::Key_G:
        return mods == Qt::ControlModifier || mods == (Qt::ControlModifier | Qt::ShiftModifier);
    case Qt::Key_A:
        return mods == Qt::ControlModifier || mods == Qt::ShiftModifier;
    case Qt::Key_F2:
        return mods == Qt::NoModifier;
    default:
        return false;
    }
}

bool PetuniaNodeCanvas::event(QEvent *event) {
    // O Qt tenta os atalhos registrados antes de entregar a tecla ao widget com foco.
    // Reivindicar a tecla aqui impede que, por exemplo, Ctrl+G agrupe objetos da cena
    // enquanto o usuário está agrupando nós.
    if (event->type() == QEvent::ShortcutOverride && ownsKey(static_cast<QKeyEvent *>(event))) {
        event->accept();
        return true;
    }
    return QGraphicsView::event(event);
}

void PetuniaNodeCanvas::keyReleaseEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Space && !event->isAutoRepeat()) {
        m_spaceHeld = false;
        if (!m_panning)
            unsetCursor();
        event->accept();
        return;
    }
    QGraphicsView::keyReleaseEvent(event);
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

void PetuniaNodeCanvas::showRecipeConnectMenu(const QPoint &globalPos) {
    auto *source = m_nodes.value(m_selected);
    if (!source)
        return;
    QMenu menu(this);
    menu.setAccessibleName(tr("Ligar a"));
    int count = 0;
    for (int out = 0; out < source->outputCount(); ++out) {
        const QString type = source->outputType(out);
        for (const auto &value : m_nodeRows) {
            const QString target = value.toObject()["id"].toString();
            auto *item = m_nodes.value(target);
            if (!item || target == m_selected)
                continue;
            for (int in = 0; in < item->inputCount(); ++in) {
                if (item->inputType(in) != type)
                    continue;
                bool linked = false;
                for (const auto &edge : m_edgeRows) {
                    const auto e = edge.toObject();
                    linked |= e["from"].toString() == m_selected && e["to"].toString() == target &&
                              e["to_port"].toInt() == in && e["from_port"].toInt() == out;
                }
                auto *action = menu.addAction(tr("%1 · %2").arg(value.toObject()["title"].toString(), item->inputLabel(in)));
                action->setCheckable(true);
                action->setChecked(linked);
                action->setData(QStringList{target, QString::number(in), QString::number(out)});
                ++count;
            }
        }
    }
    if (count == 0)
        menu.addAction(tr("Nenhuma entrada compatível"))->setEnabled(false);
    if (auto *chosen = menu.exec(globalPos)) {
        const auto data = chosen->data().toStringList();
        if (data.size() == 3) {
            if (chosen->isChecked())
                connectSockets(m_selected, data[2].toInt(), data[0], data[1].toInt());
            else
                disconnectSocket(data[0], data[1].toInt());
        }
    }
}

void PetuniaNodeCanvas::showAddMenu(const QPoint &globalPos) {
    if (m_domain == Domain::Material) {
        QMenu menu(this);
        menu.setAccessibleName(tr("Adicionar nó de material"));
        if (!m_hasRecipe) {
            auto *create = menu.addAction(tr("Criar receita de nós para o material"));
            menu.setEnabled(!m_state["material_recipe"].toObject()["material_id"].isNull());
            if (menu.exec(globalPos) == create)
                m_controller->request(QStringLiteral("material_recipe_create"));
            return;
        }
        QString category;
        for (const auto &value : m_catalog) {
            const auto kind = value.toObject();
            if (kind["category"].toString() != category) {
                category = kind["category"].toString();
                menu.addSection(petuniaTr(category.left(1).toUpper() + category.mid(1)));
            }
            menu.addAction(petuniaTr(kind["label"].toString()))->setData(kind["id"].toString());
        }
        if (auto *chosen = menu.exec(globalPos); chosen && !chosen->data().toString().isEmpty())
            m_controller->request(QStringLiteral("material_recipe_add"), {{"kind", chosen->data().toString()}});
        return;
    }
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
    if (event->reason() == QContextMenuEvent::Mouse) {
        event->accept(); // o botão direito abre o menu no release, se não houve arrasto
        return;
    }
    auto *hit = nodeAt(mapToScene(event->pos()));
    if (hit)
        selectNode(hit->id());
    if (m_domain == Domain::Material) {
        if (hit) {
            QMenu menu(this);
            auto *link = menu.addAction(tr("Ligar a…"));
            auto *remove = menu.addAction(tr("Remover nó"));
            auto *chosen = menu.exec(event->globalPos());
            if (chosen == link)
                showRecipeConnectMenu(event->globalPos());
            else if (chosen == remove)
                deleteSelected();
        } else {
            showAddMenu(event->globalPos());
        }
        event->accept();
        return;
    }
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
    // Pan: botão do meio, botão direito (só depois de arrastar; um clique abre o
    // menu) ou Espaço + esquerdo.
    if (event->button() == Qt::MiddleButton || event->button() == Qt::RightButton ||
        (event->button() == Qt::LeftButton && m_spaceHeld)) {
        m_panning = true;
        m_panButton = event->button();
        m_panLast = event->pos();
        m_panStart = event->pos();
        m_panMoved = false;
        if (m_panButton != Qt::RightButton)
            setCursor(Qt::ClosedHandCursor);
        event->accept();
        return;
    }
    if (event->button() != Qt::LeftButton) {
        QGraphicsView::mousePressEvent(event);
        return;
    }
    const bool toggle = event->modifiers() & (Qt::ControlModifier | Qt::ShiftModifier);
    if (auto *node = nodeAt(scenePos)) {
        const bool inMulti = m_multi.contains(node->id()) && m_multi.size() > 1;
        // Arrasto de fio: só saídas de operação conectam (operação → objeto).
        const int output = node->outputAt(scenePos);
        const bool onWire = output >= 0 && (m_domain == Domain::Material || !node->isObject());
        if (toggle && !onWire) {
            // Ctrl/Shift+clique soma ou tira o nó da seleção; não inicia arrasto.
            if (m_multi.contains(node->id()) && m_multi.size() > 1) {
                m_multi.remove(node->id());
                if (m_selected == node->id())
                    m_selected = *m_multi.begin();
            } else {
                m_multi.insert(node->id());
                m_selected = node->id();
            }
            applySelectionVisuals();
            refreshAccessibility();
            event->accept();
            return;
        }
        m_pressedInMulti = inMulti && !onWire;
        if (m_pressedInMulti) {
            m_selected = node->id(); // principal, sem desfazer a multisseleção
            applySelectionVisuals();
        } else {
            selectNode(node->id());
        }
        if (onWire) {
            m_wireFrom = node->id();
            m_wireFromSocket = output;
            m_ghost = m_scene->addPath(QPainterPath(), QPen(PetuniaTheme::instance().accentSelectionActive(), 2.4, Qt::DotLine));
            event->accept();
            return;
        }
        m_dragNode = node;
        m_dragOffset = scenePos - node->pos();
        m_dragStartScene = scenePos;
        m_dragMoved = false;
        m_dragOrigins.clear();
        for (const auto &id : std::as_const(m_multi))
            if (auto *item = m_nodes.value(id))
                m_dragOrigins.insert(id, item->pos());
        m_dragOrigins.insert(node->id(), node->pos());
        event->accept();
        return;
    }
    // Título de um grupo: seleciona os membros e arrasta o grupo inteiro.
    if (const int group = groupAtTitle(scenePos); group >= 0) {
        selectGroupMembers(group);
        m_pressedInMulti = false;
        if (auto *anchor = m_nodes.value(m_groups.at(group).nodes.first())) {
            m_dragNode = anchor;
            m_dragOffset = scenePos - anchor->pos();
            m_dragStartScene = scenePos;
            m_dragMoved = false;
            m_dragOrigins.clear();
            for (const auto &id : std::as_const(m_multi))
                if (auto *item = m_nodes.value(id))
                    m_dragOrigins.insert(id, item->pos());
        }
        event->accept();
        return;
    }
    // Espaço vazio: começa uma caixa de seleção (Shift soma, Ctrl subtrai).
    m_boxing = true;
    m_boxStart = event->pos();
    m_boxRect = QRect();
    m_boxExtend = event->modifiers() & Qt::ShiftModifier;
    m_boxSubtract = event->modifiers() & Qt::ControlModifier;
    event->accept();
}

void PetuniaNodeCanvas::mouseMoveEvent(QMouseEvent *event) {
    if (m_panning) {
        if (m_panButton == Qt::RightButton && !m_panMoved) {
            const QPoint total = event->pos() - m_panStart;
            if (total.manhattanLength() <= 4) {
                event->accept();
                return;
            }
            m_panMoved = true; // passou do limiar: é um arrasto, não um clique
            setCursor(Qt::ClosedHandCursor);
            panBy(total);
        } else {
            m_panMoved = true;
            panBy(event->pos() - m_panLast);
        }
        m_panLast = event->pos();
        event->accept();
        return;
    }
    const QPointF scenePos = mapToScene(event->pos());
    if (m_ghost && !m_wireFrom.isEmpty() && m_nodes.contains(m_wireFrom)) {
        QPainterPath path(m_nodes.value(m_wireFrom)->outputScene(m_wireFromSocket));
        const QPointF a = path.currentPosition();
        const qreal dx = std::max<qreal>(40, std::abs(scenePos.x() - a.x()) / 2);
        path.cubicTo(a + QPointF(dx, 0), scenePos - QPointF(dx, 0), scenePos);
        m_ghost->setPath(path);
        return;
    }
    if (m_boxing) {
        m_boxRect = QRect(m_boxStart, event->pos()).normalized();
        viewport()->update();
        event->accept();
        return;
    }
    if (m_dragNode) {
        const QPointF delta = scenePos - m_dragStartScene;
        if (!m_dragMoved && delta.manhattanLength() > 3)
            m_dragMoved = true;
        // Em bloco: todos os nós da seleção andam o mesmo deslocamento.
        for (auto it = m_dragOrigins.cbegin(); it != m_dragOrigins.cend(); ++it)
            if (auto *item = m_nodes.value(it.key()))
                item->setPos(it.value() + delta);
        updateWires();
        viewport()->update();
        return;
    }
    QGraphicsView::mouseMoveEvent(event);
}

void PetuniaNodeCanvas::mouseReleaseEvent(QMouseEvent *event) {
    if (m_panning && event->button() == m_panButton) {
        const bool plainRightClick = m_panButton == Qt::RightButton && !m_panMoved;
        m_panning = false;
        m_panButton = Qt::NoButton;
        if (m_spaceHeld)
            setCursor(Qt::OpenHandCursor);
        else
            unsetCursor();
        if (plainRightClick) {
            // Clique sem arrasto: abre o menu de contexto (o evento do sistema, que
            // chega no press, é ignorado para não abrir o menu antes do arrasto).
            QContextMenuEvent menuEvent(QContextMenuEvent::Other, event->pos(), event->globalPosition().toPoint());
            contextMenuEvent(&menuEvent);
        }
        event->accept();
        return;
    }
    const QPointF scenePos = mapToScene(event->pos());
    if (!m_wireFrom.isEmpty()) {
        const QString from = m_wireFrom;
        m_wireFrom.clear();
        delete m_ghost;
        m_ghost = nullptr;
        auto *target = nodeAt(scenePos);
        auto *source = m_nodes.value(from); // pode ter sumido num sync durante o arrasto
        if (m_domain == Domain::Material && source && target && target->id() != from &&
            m_wireFromSocket < source->outputCount()) {
            // Solta sobre uma entrada: essa; senão a primeira entrada do mesmo tipo.
            const QString type = source->outputType(m_wireFromSocket);
            int socket = target->inputAt(scenePos);
            if (socket < 0 || target->inputType(socket) != type) {
                socket = -1;
                for (int i = 0; i < target->inputCount() && socket < 0; ++i)
                    if (target->inputType(i) == type)
                        socket = i;
            }
            if (socket >= 0)
                connectSockets(from, m_wireFromSocket, target->id(), socket);
        } else if (target && target->isObject()) {
            connectOperation(from, target->id());
        }
        event->accept();
        return;
    }
    if (m_boxing) {
        m_boxing = false;
        const QRect box = m_boxRect;
        m_boxRect = QRect();
        if (box.width() + box.height() <= 6) {
            // Clique no vazio: uma multisseleção volta ao nó principal.
            if (!m_boxExtend && !m_boxSubtract && m_multi.size() > 1 && m_nodes.contains(m_selected))
                selectNode(m_selected);
        } else {
            const QRectF area = mapToScene(box).boundingRect();
            QSet<QString> hit;
            for (auto it = m_nodes.cbegin(); it != m_nodes.cend(); ++it)
                if (it.value()->sceneBoundingRect().intersects(area))
                    hit.insert(it.key());
            QSet<QString> next = (m_boxExtend || m_boxSubtract) ? m_multi : QSet<QString>();
            if (m_boxSubtract)
                next.subtract(hit);
            else
                next.unite(hit);
            QStringList ordered;
            for (auto it = m_nodes.cbegin(); it != m_nodes.cend(); ++it)
                if (next.contains(it.key()))
                    ordered << it.key();
            selectNodes(ordered);
        }
        viewport()->update();
        event->accept();
        return;
    }
    if (m_dragNode) {
        auto *node = m_dragNode;
        m_dragNode = nullptr;
        const bool moved = m_dragMoved;
        const auto origins = m_dragOrigins;
        m_dragOrigins.clear();
        m_dragMoved = false;
        if (!moved && m_pressedInMulti) {
            // Clique sem arrasto num item da multisseleção: fica só ele.
            m_pressedInMulti = false;
            selectNode(node->id());
            event->accept();
            return;
        }
        m_pressedInMulti = false;
        const bool material = m_domain == Domain::Material;
        if (origins.size() > 1 && moved) {
            if (material) {
                // A receita não tem comando em bloco: um passo de Undo por nó.
                for (auto it = origins.cbegin(); it != origins.cend(); ++it)
                    if (auto *item = m_nodes.value(it.key()))
                        m_controller->request(QStringLiteral("material_recipe_move"),
                                              {{"node", it.key()}, {"x", item->pos().x()}, {"y", item->pos().y()}});
            } else {
                QJsonArray moves;
                for (auto it = origins.cbegin(); it != origins.cend(); ++it)
                    if (auto *item = m_nodes.value(it.key()))
                        moves.append(QJsonObject{{"node", it.key()}, {"x", item->pos().x()}, {"y", item->pos().y()}});
                m_controller->request(QStringLiteral("graph_move_nodes"), {{"moves", moves}});
            }
        } else if (moved || origins.size() <= 1) {
            m_controller->request(material ? QStringLiteral("material_recipe_move") : QStringLiteral("graph_move_node"),
                                  {{"node", node->id()}, {"x", node->pos().x()}, {"y", node->pos().y()}});
        }
        event->accept();
        return;
    }
    QGraphicsView::mouseReleaseEvent(event);
}

void PetuniaNodeCanvas::wheelEvent(QWheelEvent *event) {
    const QPoint pixels = event->pixelDelta();
    const QPoint angle = event->angleDelta();
    const bool ctrl = event->modifiers().testFlag(Qt::ControlModifier);
    const bool shift = event->modifiers().testFlag(Qt::ShiftModifier);
    if (!pixels.isNull()) {
        // Trackpad: dois eixos. Com Ctrl vira zoom; sem, é pan em qualquer direção.
        if (ctrl)
            zoomBy(std::pow(1.0025, pixels.y()), event->position());
        else
            panBy(QPointF(pixels));
    } else if (angle.x() != 0 && angle.y() == 0) {
        panBy(QPointF(angle.x() / 120.0 * 60.0, 0)); // roda inclinada: pan horizontal
    } else if (shift) {
        panBy(QPointF(0, angle.y() / 120.0 * 60.0)); // Shift + roda: pan vertical
    } else if (ctrl) {
        panBy(QPointF(angle.y() / 120.0 * 60.0, 0)); // Ctrl + roda: pan horizontal
    } else {
        zoomBy(angle.y() > 0 ? 1.15 : 1 / 1.15, event->position()); // roda: zoom no cursor
    }
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

/// Legenda dos tipos de porta/fio presentes no grafo (forma + cor + nome),
/// fixa no canto da vista: ajuda a ler o que cada ligação transporta.
void PetuniaNodeCanvas::drawForeground(QPainter *painter, const QRectF &rect) {
    QGraphicsView::drawForeground(painter, rect);
    if (m_boxing && !m_boxRect.isNull()) {
        // Caixa de seleção: em coordenadas da viewport, por cima de tudo.
        painter->save();
        painter->resetTransform();
        QColor edge = PetuniaTheme::instance().accentSelectionActive();
        QColor fill = edge;
        fill.setAlpha(40);
        painter->setPen(QPen(edge, 1, Qt::DashLine));
        painter->setBrush(fill);
        painter->drawRect(m_boxRect);
        painter->restore();
    }
    QStringList types;
    for (const auto &value : m_nodeRows) {
        const auto row = value.toObject();
        for (const char *key : {"inputs", "outputs"})
            for (const auto &port : row[QLatin1String(key)].toArray()) {
                const QString type = port.toObject()["type"].toString();
                if (!type.isEmpty() && !types.contains(type))
                    types << type;
            }
    }
    if (types.isEmpty())
        return;
    static const QStringList order{"geometry", "operation", "texture", "color", "value"};
    std::sort(types.begin(), types.end(),
              [](const QString &a, const QString &b) { return order.indexOf(a) < order.indexOf(b); });
    const auto &theme = PetuniaTheme::instance();
    painter->save();
    painter->resetTransform();
    painter->setRenderHint(QPainter::Antialiasing, true);
    QFont font = painter->font();
    font.setPixelSize(theme.fontPx(PetuniaTheme::TextRole::Caption));
    painter->setFont(font);
    const QFontMetrics metrics(font);
    qreal width = 12;
    for (const auto &type : types)
        width += 18 + metrics.horizontalAdvance(typeLabel(type)) + 12;
    const QRectF box(10, 10, width, 26);
    QColor fill = theme.bgPanel();
    fill.setAlpha(215);
    painter->setPen(QPen(theme.borderSubtle(), 1));
    painter->setBrush(fill);
    painter->drawRoundedRect(box, 6, 6);
    qreal x = box.left() + 10;
    const qreal y = box.center().y();
    for (const auto &type : types) {
        const QColor color = typeColor(type);
        painter->setPen(QPen(color, 2));
        const QPointF at(x + 6, y);
        painter->setBrush(type == QLatin1String("geometry") ? Qt::NoBrush : QBrush(color));
        if (type == QLatin1String("texture")) {
            painter->drawRoundedRect(QRectF(at.x() - 5, at.y() - 5, 10, 10), 2.5, 2.5);
        } else if (type == QLatin1String("value")) {
            painter->drawRect(QRectF(at.x() - 3.5, at.y() - 3.5, 7, 7));
        } else if (type == QLatin1String("operation")) {
            QPolygonF diamond;
            diamond << at + QPointF(0, -6) << at + QPointF(6, 0) << at + QPointF(0, 6) << at + QPointF(-6, 0);
            painter->drawPolygon(diamond);
        } else {
            painter->drawEllipse(at, 5, 5);
        }
        painter->setPen(theme.textSecondary());
        const QString label = typeLabel(type);
        painter->drawText(QRectF(x + 16, box.top(), metrics.horizontalAdvance(label) + 4, box.height()),
                          Qt::AlignVCenter | Qt::AlignLeft, label);
        x += 18 + metrics.horizontalAdvance(label) + 12;
    }
    painter->restore();
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

    // Quadros dos grupos de nós: faixa de título com o nome e borda tracejada.
    for (int i = 0; i < m_groups.size(); ++i) {
        const QRectF frame = groupSceneRect(i);
        if (frame.isNull() || !frame.intersects(rect))
            continue;
        bool allSelected = true;
        for (const auto &id : m_groups.at(i).nodes)
            allSelected = allSelected && m_multi.contains(id);
        QColor accent = t.accentPrimary();
        QColor fill = accent;
        fill.setAlpha(allSelected ? 46 : 26);
        painter->setPen(QPen(allSelected ? t.accentSelectionActive() : accent, allSelected ? 1.8 : 1.2, Qt::DashLine));
        painter->setBrush(fill);
        painter->drawRoundedRect(frame, 10, 10);
        painter->setPen(t.textPrimary());
        QFont font = painter->font();
        font.setPixelSize(t.fontPx(PetuniaTheme::TextRole::Caption));
        font.setBold(true);
        painter->setFont(font);
        painter->drawText(groupTitleSceneRect(i).adjusted(10, 0, -8, 0), Qt::AlignVCenter | Qt::AlignLeft,
                          painter->fontMetrics().elidedText(m_groups.at(i).name, Qt::ElideRight, int(frame.width()) - 24));
    }
}
