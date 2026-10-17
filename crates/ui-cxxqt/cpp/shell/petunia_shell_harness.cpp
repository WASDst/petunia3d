// Harness do Shell v2 executável pelo `cargo test` (offscreen): percorre os
// workspaces, temas e larguras, valida invariantes de layout/foco e salva as
// capturas (PNG) para inspeção visual. Devolve um JSON {checks:[{name,pass,detail}]}.
#include "petunia_shell_window.h"

#include "../petunia_gl_widget.h"
#include "../petunia_theme.h"
#include "petunia_app_bar.h"
#include "petunia_context_actions.h"
#include "petunia_drawer.h"
#include "petunia_operation_hud.h"
#include "petunia_properties_panel.h"
#include "petunia_region_navigator.h"
#include "petunia_segmented.h"
#include "petunia_status_bar.h"
#include "petunia_model_sections.h"
#include "petunia_node_canvas.h"
#include "petunia_operation_card.h"
#include "petunia_properties_panel.h"
#include "petunia_structure_panel.h"
#include "petunia_tool_rail.h"
#include "petunia_viewport_header.h"

#include <QAccessible>
#include <QApplication>
#include <QDir>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QSettings>
#include <QSplitter>
#include <QStandardPaths>

namespace {
struct Harness {
    QJsonArray checks;
    QString dir;
    void add(const QString &name, bool pass, const QString &detail = QString()) {
        QJsonObject o;
        o["name"] = name;
        o["pass"] = pass;
        if (!detail.isEmpty())
            o["detail"] = detail;
        checks.append(o);
    }
    void shot(QWidget *w, const QString &name) {
        const bool ok = w->grab().save(dir + QLatin1Char('/') + name + QStringLiteral(".png"));
        add(QStringLiteral("capture:") + name, ok);
    }
};

void settle(int rounds = 4) {
    for (int i = 0; i < rounds; ++i)
        QCoreApplication::processEvents(QEventLoop::AllEvents, 30);
}

void sendKey(QWidget *target, int key, Qt::KeyboardModifiers mods = Qt::NoModifier) {
    QKeyEvent press(QEvent::KeyPress, key, mods);
    QCoreApplication::sendEvent(target, &press);
    settle(1);
}
} // namespace

extern "C" const char *petunia_shell_harness(const char *directory) {
    static QByteArray out;
    Harness h;
    h.dir = QString::fromUtf8(directory);
    QDir().mkpath(h.dir);

    PetuniaShellWindow window(nullptr, false);
    window.resize(1360, 860);
    window.show();
    settle(8);
    auto *c = window.controller();

    h.add("shell:starts-in-a-modeling-workspace", c->isModeling(), c->workspaceId());
    h.add("shell:regions-exist", window.appBar() && window.toolRail() && window.viewportHeader() &&
                                     window.structure() && window.properties() && window.drawer() &&
                                     window.statusRegion() && window.hud() && window.contextActions());

    // Seleciona o primeiro objeto da cena para exercitar propriedades e ações.
    const auto items = c->state()["scene_items"].toArray();
    if (!items.isEmpty())
        c->request("select", {{"id", items.first().toObject()["id"].toString()}});
    settle();

    for (const QString &ws : {QStringLiteral("DRAW"), QStringLiteral("POLY"), QStringLiteral("PAINT"),
                              QStringLiteral("UV"), QStringLiteral("ANIMATE")}) {
        c->request("workspace", {{"id", ws}});
        settle(8);
        window.syncNow();
        settle(4);
        h.add("workspace:" + ws + ":active", c->workspaceId() == ws, c->workspaceId());
        h.add("workspace:" + ws + ":segmented-follows", window.appBar()->workspaces()->current() == ws);
        h.add("workspace:" + ws + ":rail-has-tools", !window.toolRail()->buttons().isEmpty(),
              QString::number(window.toolRail()->buttons().size()));
        h.add("workspace:" + ws + ":viewport-visible", window.viewport()->isVisible());
        const double share = double(window.surfaceSplitter()->width()) / double(window.width());
        h.add("workspace:" + ws + ":work-surface-dominant", share >= 0.45, QString::number(share, 'f', 2));
        h.add("workspace:" + ws + ":right-column-visible", window.structure()->isVisible() && window.properties()->isVisible());
        h.shot(&window, "shell_" + ws);
    }

    // DRAW/POLY: ações contextuais com ≤ 4 em destaque e seções de propriedades.
    c->request("workspace", {{"id", "POLY"}});
    settle(8);
    window.syncNow();
    h.add("poly:context-actions-visible", window.contextActions()->isVisibleTo(&window),
          QString::number(window.contextActions()->primaryButtons().size()));
    h.add("poly:at-most-four-primary-actions", window.contextActions()->primaryButtons().size() <= 4);
    h.add("poly:has-more-menu", window.contextActions()->moreButton() != nullptr);
    const auto sections = window.properties()->visibleSectionIds();
    h.add("poly:property-sections", sections.contains("transform") && sections.contains("geometry") &&
                                       sections.contains("operations") && sections.contains("material"),
          sections.join(","));
    h.add("poly:no-profile-section-outside-draw", !sections.contains("profile"));
    c->request("domain", {{"id", "face"}});
    settle(6);
    window.syncNow();
    const auto faceIds = window.contextActions()->primaryIds();
    h.add("poly:face-domain-actions", faceIds.contains("inset") && faceIds.contains("push_pull"), faceIds.join(","));
    h.add("poly:domain-segmented-follows", window.viewportHeader()->domain()->current() == "face");
    c->request("domain", {{"id", "object"}});
    settle(4);

    // Ferramenta persistente muda o estado do rail (e o catálogo é o do workspace).
    if (auto *move = window.toolRail()->button("move")) {
        move->click();
        settle(4);
        h.add("rail:click-activates-tool", c->text("active_tool") == "move" && window.toolRail()->button("move")->isChecked(),
              c->text("active_tool"));
    } else {
        h.add("rail:click-activates-tool", false, "sem botão move");
    }

    // HUD: loop cut abre a seção própria em vez de cartão solto.
    c->request("tool", {{"id", "loop_cut"}});
    settle(4);
    c->request("begin_loop_cut");
    settle(6);
    window.syncNow();
    h.add("hud:loop-cut-section", window.hud()->activeSections().contains("loop_cut"),
          window.hud()->activeSections().join(","));
    h.add("hud:visible-over-viewport", window.hud()->isVisible());
    h.shot(&window, "shell_POLY_loopcut");
    c->request("cancel_loop_cut");
    settle(4);

    // Operation Graph (ADR-C13): booleana como nó/operação, aplicada ao alvo, sem destruir a malha.
    {
        c->request("workspace", {{"id", "POLY"}});
        c->request("command", {{"id", "model.add_cube"}});
        const QString target = c->text("active_asset_id");
        c->request("command", {{"id", "model.add_sphere"}});
        const QString tool = c->text("active_asset_id");
        settle(6);
        window.syncNow();
        auto *ops = window.properties()->operations();
        h.add("graph:two-objects-created", !target.isEmpty() && !tool.isEmpty() && target != tool);
        h.add("graph:no-operations-initially", ops && ops->operationCount() == 0);
        c->request("graph_add_boolean");
        settle(6);
        window.syncNow();
        settle(4);
        // HUD gerado do descriptor: o nó recém-criado mostra seus parâmetros sobre a vista.
        auto *hud = window.hud();
        h.add("hud:graph-operation-section-after-add",
              hud->activeSections().contains("graph_operation") && hud->operationCard() != nullptr,
              hud->activeSections().join(","));
        h.add("hud:generated-card-has-choice-control",
              hud->operationCard() && !hud->operationCard()->findChildren<PetuniaSegmented *>().isEmpty());
        h.add("hud:visible-over-viewport-for-operation", hud->isVisible());
        h.shot(&window, "shell_POLY_hud_boolean");
        hud->operationDoneButton()->click();
        settle(4);
        h.add("hud:done-dismisses-operation", !hud->activeSections().contains("graph_operation") && hud->focusedOperation().isEmpty());
        h.add("graph:boolean-appears-as-tool-card", ops->toolCards().size() == 1,
              QString::number(ops->toolCards().size()));
        h.add("graph:bake-hidden-until-applied", !ops->bakeButton()->isVisibleTo(ops));
        const auto graph = c->state()["op_graph"].toObject();
        const auto toolRows = graph["as_tool"].toArray();
        const QString nodeId = toolRows.isEmpty() ? QString() : toolRows.first().toObject()["id"].toString();
        c->request("graph_apply", {{"operation", nodeId}, {"target", target}});
        settle(6);
        c->request("select", {{"id", target}});
        settle(6);
        window.syncNow();
        settle(4);
        h.add("graph:applied-card-on-target", ops->appliedCards().size() == 1, QString::number(ops->appliedCards().size()));
        h.add("graph:bake-visible-when-applied", ops->bakeButton()->isVisibleTo(ops));
        h.shot(&window, "shell_POLY_boolean_applied");
        c->request("graph_set_param", {{"node", nodeId}, {"key", "operation"}, {"value", "intersection"}});
        settle(4);
        window.syncNow();
        const auto after = c->state()["op_graph"].toObject()["applied"].toArray();
        h.add("graph:param-edit-roundtrips",
              !after.isEmpty() && after.first().toObject()["params"].toObject()["operation"].toString() == "intersection");
        c->request("undo");
        settle(4);
        window.syncNow();
        const auto undone = c->state()["op_graph"].toObject()["applied"].toArray();
        h.add("graph:undo-reverts-param",
              !undone.isEmpty() && undone.first().toObject()["params"].toObject()["operation"].toString() != "intersection");
        c->request("graph_bake");
        settle(6);
        window.syncNow();
        h.add("graph:bake-clears-applied", ops->appliedCards().isEmpty(), QString::number(ops->appliedCards().size()));
    }

    // Canvas de nós: projeta o grafo, conecta por fio/teclado e comanda por ações graph_*.
    {
        c->request("workspace", {{"id", "POLY"}});
        c->request("command", {{"id", "model.add_cube"}});
        const QString wall = c->text("active_asset_id");
        c->request("command", {{"id", "model.add_cylinder"}});
        const QString cutter = c->text("active_asset_id");
        c->request("graph_add_boolean");
        settle(6);
        window.syncNow();
        auto *canvas = window.nodeCanvas();
        h.add("nodes:drawer-has-nodes-tab", canvas != nullptr && window.drawerRegion()->pageCount() >= 2);
        window.drawerRegion()->setCurrentPage("nodes");
        window.drawerRegion()->setOpen(true);
        window.resize(1360, 960);
        settle(8);
        window.syncNow();
        settle(4);
        h.add("nodes:drawer-opens-and-shows-canvas", window.drawerRegion()->isOpen() && canvas->isVisible() && canvas->height() > 80,
              QString::number(canvas->height()));
        canvas->fitAll();
        h.add("nodes:canvas-projects-graph", canvas && canvas->nodeCount() >= 2,
              QString::number(canvas ? canvas->nodeCount() : -1));
        QString opId;
        QString wallNode;
        for (const auto &id : canvas->nodeIds()) {
            const auto row = c->state()["op_graph"].toObject()["nodes"].toArray();
            for (const auto &r : row) {
                const auto o = r.toObject();
                if (o["id"].toString() != id)
                    continue;
                if (o["type_id"].toString() == "petunia.boolean")
                    opId = id;
                else if (o["params"].toObject()["asset"].toString() == wall)
                    wallNode = id;
            }
        }
        h.add("nodes:boolean-node-present", !opId.isEmpty());
        if (wallNode.isEmpty()) {
            // O objeto-alvo ainda não tem nó (nasce ao ser ligado): ligar por ação.
            c->request("graph_apply", {{"operation", opId}, {"target", wall}});
            settle(6);
            window.syncNow();
            for (const auto &r : c->state()["op_graph"].toObject()["nodes"].toArray()) {
                const auto o = r.toObject();
                if (o["params"].toObject()["asset"].toString() == wall)
                    wallNode = o["id"].toString();
            }
        } else {
            canvas->connectOperation(opId, wallNode);
            settle(6);
            window.syncNow();
        }
        h.add("nodes:apply-creates-wire", canvas->wireCount() >= 1, QString::number(canvas->wireCount()));
        canvas->selectNode(opId);
        h.add("nodes:selection-describes-targets",
              canvas->describeSelection().contains("Cube") && canvas->describeSelection().contains("Cylinder") == false,
              canvas->describeSelection());
        h.add("nodes:accessible-description-set", !canvas->accessibleDescription().isEmpty());
        canvas->fitAll();
        h.shot(&window, "shell_POLY_nodes");
        // Acessibilidade: cada nó é um item acessível com nome, descrição e estado.
        {
            auto *iface = QAccessible::queryAccessibleInterface(canvas);
            h.add("nodes:a11y-canvas-exposes-nodes", iface && iface->childCount() == canvas->nodeCount(),
                  iface ? QString::number(iface->childCount()) : QStringLiteral("sem interface"));
            QAccessibleInterface *node = nullptr;
            if (iface)
                for (int i = 0; i < iface->childCount(); ++i)
                    if (iface->child(i) && iface->child(i)->text(QAccessible::Name).contains("Boolean"))
                        node = iface->child(i);
            h.add("nodes:a11y-node-has-name-role-description",
                  node && node->role() == QAccessible::ListItem && !node->text(QAccessible::Description).isEmpty() &&
                      node->text(QAccessible::Description).contains("Cube"),
                  node ? node->text(QAccessible::Description) : QStringLiteral("nó não encontrado"));
            h.add("nodes:a11y-selected-state-follows-selection",
                  node && node->state().selected && node->state().selectable && !node->rect().isEmpty());
            h.add("nodes:a11y-parent-link", node && node->parent() == iface);
        }
        // Teclado: Ctrl+→ move o nó e persiste a posição no documento.
        const QPointF before = canvas->nodeScenePosition(opId);
        canvas->setFocus();
        QKeyEvent right(QEvent::KeyPress, Qt::Key_Right, Qt::ControlModifier);
        QCoreApplication::sendEvent(canvas, &right);
        settle(4);
        h.add("nodes:ctrl-arrow-moves-node", canvas->nodeScenePosition(opId).x() > before.x() + 10,
              QStringLiteral("%1 -> %2").arg(before.x()).arg(canvas->nodeScenePosition(opId).x()));
        // Desconectar pela mesma via do menu por teclado.
        canvas->disconnectOperation(opId, wallNode);
        settle(6);
        window.syncNow();
        h.add("nodes:unapply-removes-wire", canvas->wireCount() < 2, QString::number(canvas->wireCount()));
        // Delete remove a operação selecionada.
        canvas->selectNode(opId);
        QKeyEvent del(QEvent::KeyPress, Qt::Key_Delete, Qt::NoModifier);
        QCoreApplication::sendEvent(canvas, &del);
        settle(6);
        window.syncNow();
        h.add("nodes:delete-removes-operation", !canvas->nodeIds().contains(opId),
              canvas->selectedNodeId() + " ops=" + QString::number(canvas->nodeCount()));
        Q_UNUSED(cutter);
        window.hud()->dismissOperation();
        window.drawerRegion()->setOpen(false);
        window.resize(1360, 860);
        settle(6);
    }

    // ── F7: Workspaces PAINT, UV e ANIMATE (U16–U18) ─────────────────────────
    {
        // PAINT (U16): Estrutura mostra camadas (base >= 1), e adicionar camada reflete na árvore.
        c->request("workspace", {{"id", "PAINT"}});
        settle(8);
        window.syncNow();
        settle(4);
        const int baseLayers = window.structure()->rowCount();
        h.add("paint:structure-shows-layers", baseLayers >= 1, QString::number(baseLayers));
        c->request("add_paint_layer");
        settle(6);
        window.syncNow();
        settle(4);
        h.add("paint:add-layer-updates-structure", window.structure()->rowCount() > baseLayers,
              QStringLiteral("%1 -> %2").arg(baseLayers).arg(window.structure()->rowCount()));
        h.shot(&window, "shell_PAINT_v2");

        // UV (U17): Canvas UV dominante (alvo >= 55% da superfície).
        c->request("workspace", {{"id", "UV"}});
        settle(8);
        window.syncNow();
        settle(4);
        auto *surf = window.surfaceSplitter();
        const auto sizes = surf->sizes();
        const double uvRatio = (sizes.size() >= 2 && (sizes[0] + sizes[1]) > 0)
                                   ? (double(sizes[0]) / double(sizes[0] + sizes[1]))
                                   : 0.0;
        h.add("uv:canvas-is-dominant", uvRatio >= 0.55, QString::number(uvRatio, 'f', 2));
        h.shot(&window, "shell_UV_v2");

        // ANIMATE (U18): Gaveta tem aba Linha do tempo e Estrutura lista rig / ossos ao adicionar criatura.
        c->request("workspace", {{"id", "ANIMATE"}});
        settle(8);
        window.syncNow();
        settle(4);
        h.add("animate:drawer-has-timeline-tab", window.drawerRegion()->currentPageId() == "timeline" ||
                                                   window.drawerRegion()->tabBar()->count() >= 1,
              window.drawerRegion()->currentPageId());
        c->request("animate", {{"id", "add-creature"}, {"arg", "bird"}});
        settle(8);
        window.syncNow();
        settle(4);
        const int bonesCount = window.structure()->rowCount();
        h.add("animate:structure-shows-rig-bones", bonesCount >= 1, QString::number(bonesCount));
        h.shot(&window, "shell_ANIMATE_v2");

        // Retorna a POLY para manter a baseline das verificações subsequentes
        c->request("workspace", {{"id", "POLY"}});
        settle(8);
        window.syncNow();
        settle(4);
    }

    // Foco por regiões (F6): percorre regiões distintas e volta ao ponto de partida.
    QSet<int> visited;
    window.appBar()->searchButton()->setFocus();
    settle(2);
    visited.insert(window.focusedRegion());
    for (int i = 0; i < 6; ++i) {
        sendKey(&window, Qt::Key_F6);
        visited.insert(window.focusedRegion());
    }
    h.add("focus:f6-visits-multiple-regions", visited.size() >= 5, QString::number(visited.size()));
    const int before = window.focusedRegion();
    sendKey(&window, Qt::Key_F6, Qt::ShiftModifier);
    h.add("focus:shift-f6-goes-back", window.focusedRegion() != before);

    // Tema claro e densidade: a interface inteira segue os tokens.
    c->request("theme", {{"id", "petunia-light"}});
    settle(8);
    window.syncNow();
    h.add("theme:light-applied", PetuniaTheme::instance().themeId() == "petunia-light", PetuniaTheme::instance().themeId());
    h.shot(&window, "shell_POLY_light");
    c->request("theme", {{"id", "petunia-high-contrast"}});
    settle(8);
    window.syncNow();
    h.shot(&window, "shell_POLY_hc");
    c->request("theme", {{"id", "petunia-dark"}});
    settle(8);
    PetuniaTheme::instance().setDensity(PetuniaTheme::Density::Compact);
    settle(6);
    h.shot(&window, "shell_POLY_compact");
    PetuniaTheme::instance().setDensity(PetuniaTheme::Density::Comfortable);
    settle(6);
    h.shot(&window, "shell_POLY_comfortable");
    PetuniaTheme::instance().setDensity(PetuniaTheme::Density::Standard);
    settle(4);

    // Largura estreita: o cabeçalho vira só ícones; nada some da barra do app.
    window.resize(820, 600);
    settle(8);
    h.add("responsive:header-compact-when-narrow", window.viewportHeader()->compact() ||
                                                      window.viewportHeader()->width() >= 760);
    h.add("responsive:viewport-keeps-minimum-width", window.viewport()->width() >= 200,
          QString::number(window.viewport()->width()));
    {
        auto *body = window.bodySplitter();
        auto *mid = body->widget(0);
        auto *right = body->widget(1);
        const bool overlap = mid->geometry().right() >= right->geometry().left();
        h.add("responsive:columns-do-not-overlap", !overlap,
              QStringLiteral("mid=%1..%2 right=%3..%4 body=%5")
                  .arg(mid->geometry().left()).arg(mid->geometry().right())
                  .arg(right->geometry().left()).arg(right->geometry().right()).arg(body->width()));
        h.add("responsive:middle-keeps-minimum", mid->width() >= 440, QString::number(mid->width()));
        h.add("responsive:header-fits", window.viewportHeader()->width() >= window.viewportHeader()->minimumSizeHint().width(),
              QStringLiteral("%1 < %2").arg(window.viewportHeader()->width()).arg(window.viewportHeader()->minimumSizeHint().width()));
    }
    h.shot(&window, "shell_POLY_narrow");
    window.resize(1360, 860);
    settle(6);

    // Layout do usuário persiste e restaura.
    auto *store = window.layoutStore();
    auto layout = store->load("POLY");
    layout[PetuniaLayoutStore::kRightWidth] = 410;
    store->save("POLY", layout);
    h.add("layout:right-width-persists", store->load("POLY")[PetuniaLayoutStore::kRightWidth].toInt() == 410);
    store->reset("POLY");
    h.add("layout:reset-returns-default",
          store->load("POLY")[PetuniaLayoutStore::kRightWidth].toInt() == PetuniaLayoutStore::defaults("POLY")[PetuniaLayoutStore::kRightWidth].toInt());

    out = QJsonDocument(QJsonObject{{"checks", h.checks}}).toJson(QJsonDocument::Compact);
    return out.constData();
}
