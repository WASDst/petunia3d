// Harness do Shell v2 executável pelo `cargo test` (offscreen): percorre os
// workspaces, temas e larguras, valida invariantes de layout/foco e salva as
// capturas (PNG) para inspeção visual. Devolve um JSON {checks:[{name,pass,detail}]}.
#include "petunia_shell_window.h"

#include "../petunia_editor_panel.h"
#include "../petunia_gl_widget.h"
#include "../petunia_nav_cube.h"
#include "../petunia_theme.h"
#include "petunia_align_popover.h"
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
#include "petunia_node_page.h"
#include "petunia_i18n.h"
#include "../petunia_number_field.h"
#include <QToolButton>
#include "petunia_operation_card.h"
#include "petunia_properties_panel.h"
#include "petunia_structure_panel.h"
#include "petunia_tool_rail.h"
#include "petunia_viewport_header.h"
#include "petunia_workspace_sections.h"
#include "petunia_pie_menu.h"
#include "../petunia_color_picker.h"
#include "../petunia_dialog_chrome.h"
#include "../petunia_vector3_field.h"

#include <QCheckBox>
#include <QDialog>
#include <QPointer>
#include <QPushButton>
#include <QVBoxLayout>

#include <QAbstractButton>
#include <QAccessible>
#include <QApplication>
#include <functional>
#include <QTimer>
#include <QLineEdit>
#include <cmath>
#include <QDir>
#include <QJsonArray>
#include <QJsonDocument>
#include <QItemSelectionModel>
#include <QJsonObject>
#include <QKeyEvent>
#include <QMenu>
#include <QMouseEvent>
#include <QSpinBox>
#include <QLabel>
#include <QFormLayout>
#include <QDoubleSpinBox>
#include <QWheelEvent>
#include <QSettings>
#include <QSplitter>
#include <QStandardPaths>

// Exportada pelo QtGui (é o que o QTest usa para disparar QShortcut).
Q_GUI_EXPORT bool qt_sendShortcutOverrideEvent(QObject *o, ulong timestamp, int k, Qt::KeyboardModifiers mods,
                                                const QString &text, bool autorep, ushort count);

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

/// Gaveta recolhida devolve a altura à superfície; ações contextuais cabem na vista.
void checkSurfaceGeometry(Harness &h, PetuniaShellWindow &window, const QString &ws) {
    auto *drawer = window.drawerRegion();
    const int column = drawer->parentWidget() ? drawer->parentWidget()->height() : 0;
    h.add("drawer:" + ws + ":collapsed-returns-space",
          drawer->isOpen() ||
              (drawer->height() <= drawer->barHeight() + 4 && drawer->geometry().bottom() >= column - 4),
          QStringLiteral("open=%1 h=%2 bar=%3 bottom=%4 column=%5 surface=%6")
              .arg(drawer->isOpen())
              .arg(drawer->height())
              .arg(drawer->barHeight())
              .arg(drawer->geometry().bottom())
              .arg(column)
              .arg(window.surfaceSplitter()->height()));
    auto *context = window.contextActions();
    const bool fits = !context->isVisible() ||
                      (context->parentWidget() && context->parentWidget()->rect().contains(context->geometry()));
    h.add("context:" + ws + ":fits-inside-view", fits,
          context->parentWidget() ? QStringLiteral("bar=%1,%2 %3x%4 view=%5x%6")
                                        .arg(context->x())
                                        .arg(context->y())
                                        .arg(context->width())
                                        .arg(context->height())
                                        .arg(context->parentWidget()->width())
                                        .arg(context->parentWidget()->height())
                                  : QStringLiteral("sem pai"));
}

/// Tecla como o usuário: primeiro o mapa de atalhos (QShortcut), depois o
/// widget com foco — o mesmo caminho do QTest::keyClick, sem depender do QtTest.
void realKey(QWidget *target, int key, Qt::KeyboardModifiers mods = Qt::NoModifier, const QString &text = {}) {
    target->setFocus();
    settle(1);
    if (!qt_sendShortcutOverrideEvent(target, 0, key, mods, text, false, 1)) {
        QKeyEvent press(QEvent::KeyPress, key, mods, text);
        QCoreApplication::sendEvent(target, &press);
        QKeyEvent release(QEvent::KeyRelease, key, mods, text);
        QCoreApplication::sendEvent(target, &release);
    }
    settle(3);
}

void sendKey(QWidget *target, int key, Qt::KeyboardModifiers mods = Qt::NoModifier) {
    QKeyEvent press(QEvent::KeyPress, key, mods);
    QCoreApplication::sendEvent(target, &press);
    settle(1);
}
} // namespace

namespace {
QStringList sceneIds(PetuniaShellController *c) {
    QStringList ids;
    for (const auto &value : c->state()["scene_items"].toArray())
        ids << value.toObject()["id"].toString();
    return ids;
}

QJsonObject sceneItem(PetuniaShellController *c, const QString &id) {
    for (const auto &value : c->state()["scene_items"].toArray())
        if (value.toObject()["id"].toString() == id)
            return value.toObject();
    return {};
}

void selectIds(PetuniaShellController *c, const QStringList &ids) {
    QJsonArray array;
    for (const auto &id : ids)
        array.append(id);
    c->request("scene_selection", {{"ids", array}, {"active", ids.isEmpty() ? QString() : ids.first()}});
    settle(4);
}

/// Refinamento de uso (ADR-C15): rail enxuto, cabeçalho só com ícones, grupos,
/// arrastar e soltar e isolamento na Estrutura, alinhar e distribuir, pivô, gizmos
/// configuráveis e cubo de navegação.

/// Arrastar sobre o valor ou o nome ajusta qualquer campo numérico (PetuniaScrub).
void checkScrub(Harness &h) {
    QWidget host;
    auto *form = new QFormLayout(&host);
    auto *number = new PetuniaNumberField(&host);
    number->setRange(-100, 100);
    number->setSingleStep(0.5);
    number->setValue(10);
    auto *real = new QDoubleSpinBox(&host);
    real->setRange(-100, 100);
    real->setSingleStep(0.25);
    real->setValue(1.0);
    auto *whole = new QSpinBox(&host);
    whole->setRange(0, 1000);
    whole->setSingleStep(2);
    whole->setValue(10);
    form->addRow(QStringLiteral("Largura"), number);
    form->addRow(QStringLiteral("Altura"), real);
    form->addRow(QStringLiteral("Lados"), whole);
    host.resize(320, 160);
    host.show();
    host.activateWindow();
    settle(6);
    // O primeiro campo de um formulário abre com foco (em edição). Os testes do valor
    // partem de campos sem foco; o do nome cobre justamente o campo focado.
    const bool startedFocused = number->isEditing();
    int commits = 0;
    QObject::connect(number, &PetuniaNumberField::commitRequested, &host, [&] { ++commits; });

    const auto mouse = [&](QWidget *w, QEvent::Type type, const QPoint &at, Qt::MouseButton button, Qt::MouseButtons held,
                           Qt::KeyboardModifiers mods = Qt::NoModifier) {
        QMouseEvent event(type, QPointF(at), QPointF(w->mapToGlobal(at)), button, held, mods);
        QCoreApplication::sendEvent(w, &event);
    };
    const auto drag = [&](QWidget *w, int dx, Qt::KeyboardModifiers mods = Qt::NoModifier) {
        const QPoint from(qMax(4, w->width() / 3), w->height() / 2);
        mouse(w, QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton, mods);
        for (int step = 1; step <= 8; ++step)
            mouse(w, QEvent::MouseMove, from + QPoint(dx * step / 8, 0), Qt::NoButton, Qt::LeftButton, mods);
        mouse(w, QEvent::MouseButtonRelease, from + QPoint(dx, 0), Qt::LeftButton, Qt::NoButton, mods);
        settle(1);
    };
    auto *numberLabel = form->labelForField(number);
    auto *realLabel = form->labelForField(real);
    auto *wholeLabel = form->labelForField(whole);

    // Nome do campo: arrastar para a direita aumenta pelo passo do campo (20 px × 0,5), mesmo
    // com o campo focado (em edição): o arrasto encerra a edição.
    drag(numberLabel, 20);
    h.add("scrub:label-drag-adjusts-a-number-field", std::abs(number->value() - 20.0) < 0.01 && commits == 1 && !number->isEditing(),
          QStringLiteral("valor %1, confirmações %2, focado no início %3").arg(number->value()).arg(commits).arg(startedFocused));
    number->valueEditor()->clearFocus();
    settle(2);
    // Valor: arrastar para a esquerda diminui; Shift multiplica por 10.
    drag(number->valueEditor(), -10);
    const double afterValue = number->value();
    h.add("scrub:value-drag-adjusts-a-number-field", std::abs(afterValue - 15.0) < 0.01, QString::number(afterValue));
    drag(numberLabel, 4, Qt::ShiftModifier); // passa do limiar de 3 px
    h.add("scrub:shift-multiplies-the-step", std::abs(number->value() - (afterValue + 4 * 0.5 * 10)) < 0.01,
          QString::number(number->value()));

    // Clique sem arrasto no valor leva ao editor (digitar continua sendo o caminho principal).
    const QPoint inside(number->valueEditor()->width() / 2, number->valueEditor()->height() / 2);
    const double beforeClick = number->value();
    mouse(number->valueEditor(), QEvent::MouseButtonPress, inside, Qt::LeftButton, Qt::LeftButton);
    mouse(number->valueEditor(), QEvent::MouseButtonRelease, inside, Qt::LeftButton, Qt::NoButton);
    settle(2);
    h.add("scrub:click-without-drag-edits-the-field", number->isEditing() && std::abs(number->value() - beforeClick) < 1e-9);
    // Em edição, arrastar sobre o **valor** não ajusta: o texto é do usuário (seleção de texto).
    const double whileEditing = number->value();
    drag(number->valueEditor(), 30);
    h.add("scrub:editing-field-is-not-scrubbed", number->isEditing() && std::abs(number->value() - whileEditing) < 1e-9);
    number->valueEditor()->clearFocus();
    settle(2);

    // Spin boxes: nome e valor (as setas do próprio spin box ficam como antes).
    drag(realLabel, 8);
    const bool realByLabel = std::abs(real->value() - 3.0) < 0.01;
    drag(real->findChild<QLineEdit *>(), -4);
    const bool realByValue = std::abs(real->value() - 2.0) < 0.01;
    drag(wholeLabel, 10);
    h.add("scrub:spin-boxes-scrub-by-label-and-value", realByLabel && realByValue && whole->value() == 30,
          QStringLiteral("double %1, inteiro %2").arg(real->value()).arg(whole->value()));
}

void checkRefinement(Harness &h, PetuniaShellWindow &window) {
    checkScrub(h);
    // O formulário temporário do teste ativou outra janela: devolve a ativação ao shell.
    window.activateWindow();
    window.raise();
    settle(6);
    auto *c = window.controller();
    c->request("workspace", {{"id", "POLY"}});
    settle(8);
    window.syncNow();

    // Rail: só transformar, 3D Cursor e criação; operações são contextuais.
    QStringList railIds;
    for (auto *button : window.toolRail()->buttons())
        railIds << button->property("toolId").toString();
    bool noOperations = true;
    for (const QString &operation : {QStringLiteral("extrude"), QStringLiteral("inset"), QStringLiteral("bevel"),
                                     QStringLiteral("push_pull"), QStringLiteral("loop_cut"), QStringLiteral("cut"),
                                     QStringLiteral("slice"), QStringLiteral("shape_builder"), QStringLiteral("measure")})
        noOperations = noOperations && !railIds.contains(operation);
    h.add("rail:only-transform-cursor-and-create", noOperations && railIds.contains("transform") &&
                                                      railIds.contains("cursor") && railIds.contains("primitive"),
          railIds.join(","));

    // Cabeçalho da vista: só ícones (nome e atalho nos tooltips).
    bool iconOnly = true;
    for (auto *button : window.viewportHeader()->findChildren<QToolButton *>())
        iconOnly = iconOnly && button->text().isEmpty() && !button->toolTip().isEmpty();
    h.add("header:icon-only-with-tooltips", iconOnly);

    // Cena com vários objetos: seleciona os três primeiros.
    while (sceneIds(c).size() < 4)
        c->request("command", {{"id", "model.add_cube"}});
    settle(4);
    const QStringList ids = sceneIds(c);
    const QStringList firstThree{ids[0], ids[1], ids[2]};
    selectIds(c, firstThree);
    h.add("structure:multi-select-reaches-the-editor", c->state()["selected_count"].toInt() == 3,
          QString::number(c->state()["selected_count"].toInt()));

    // Ctrl+G com a tecla real: os três viram um grupo.
    auto *tree = window.structure()->tree();
    realKey(tree, Qt::Key_G, Qt::ControlModifier, QStringLiteral("g"));
    window.syncNow();
    const auto groups = c->state()["scene_groups"].toArray();
    bool allGrouped = !groups.isEmpty();
    for (const auto &id : firstThree)
        allGrouped = allGrouped && !sceneItem(c, id)["collection"].toString().isEmpty();
    h.add("structure:ctrl-g-groups-the-selection", groups.size() == 1 && allGrouped,
          QString::number(groups.size()));
    const QString groupName = groups.isEmpty() ? QString() : groups.first().toObject()["name"].toString();
    h.add("structure:group-row-and-members-in-the-tree",
          window.structure()->indexOfId(QStringLiteral("group:") + groupName).isValid() &&
              window.structure()->indexOfId(firstThree[0]).isValid());
    h.shot(&window, "shell_POLY_groups");

    // Duplo clique no nome do grupo renomeia (como F2); Esc fecha sem alterar.
    {
        const auto groupIndex = window.structure()->indexOfId(QStringLiteral("group:") + groupName);
        emit tree->doubleClicked(groupIndex.siblingAtColumn(0));
        auto *editor = qobject_cast<QLineEdit *>(QApplication::focusWidget());
        const bool editing = editor && tree->isAncestorOf(editor) && editor->text() == groupName;
        if (editor)
            sendKey(editor, Qt::Key_Escape);
        window.syncNow();
        const auto after = c->state()["scene_groups"].toArray();
        h.add("structure:double-click-renames-a-group",
              editing && after.size() == 1 && after.first().toObject()["name"].toString() == groupName,
              editing ? QStringLiteral("editou") : QStringLiteral("não abriu o editor"));
    }

    // Arrastar e soltar: a 4ª parte entra no grupo (soltar sobre o grupo)...
    const QString loose = ids[3];
    selectIds(c, {loose});
    const auto groupIndex = window.structure()->indexOfId(QStringLiteral("group:") + groupName);
    window.structure()->dropOn(groupIndex, PetuniaStructurePanel::DropPosition::OnItem);
    window.syncNow();
    h.add("structure:drop-on-group-joins-it", sceneItem(c, loose)["collection"].toString() == groupName,
          sceneItem(c, loose)["collection"].toString());
    // ...e sai dele ao ser solta antes de uma parte que está fora de qualquer grupo.
    selectIds(c, {loose});
    window.structure()->dropOn(QModelIndex(), PetuniaStructurePanel::DropPosition::OnViewport);
    window.syncNow();
    h.add("structure:drop-on-empty-area-leaves-the-group", sceneItem(c, loose)["collection"].toString().isEmpty());
    const QStringList afterDrop = sceneIds(c);
    h.add("structure:drop-on-empty-area-moves-to-the-end", !afterDrop.isEmpty() && afterDrop.last() == loose,
          afterDrop.join(","));

    // Reordenar antes de outra parte (alvo acima).
    selectIds(c, {loose});
    window.structure()->dropOn(window.structure()->indexOfId(ids[0]), PetuniaStructurePanel::DropPosition::AboveItem);
    window.syncNow();
    const QStringList reordered = sceneIds(c);
    h.add("structure:drop-above-reorders", !reordered.isEmpty() && reordered.first() == loose, reordered.join(","));

    // Isolar uma parte pela linha: só ela e a seleção ficam na cena; de novo, volta tudo.
    c->request("isolate_part", {{"id", ids[1]}});
    window.syncNow();
    int visibleWhileIsolated = 0;
    for (const auto &id : sceneIds(c))
        visibleWhileIsolated += sceneItem(c, id)["visible"].toBool() ? 1 : 0;
    h.add("structure:isolate-keeps-only-the-selection", c->state()["isolate_active"].toBool() && visibleWhileIsolated >= 1 &&
                                                          visibleWhileIsolated < sceneIds(c).size(),
          QString::number(visibleWhileIsolated));
    c->request("isolate_part", {{"id", ids[1]}});
    window.syncNow();
    bool allVisible = !c->state()["isolate_active"].toBool();
    for (const auto &id : sceneIds(c))
        allVisible = allVisible && sceneItem(c, id)["visible"].toBool();
    h.add("structure:leaving-isolation-restores-visibility", allVisible);

    // Desagrupar devolve as partes à lista solta.
    selectIds(c, {ids[0]});
    realKey(tree, Qt::Key_G, Qt::ControlModifier | Qt::ShiftModifier, QStringLiteral("G"));
    window.syncNow();
    h.add("structure:ctrl-shift-g-ungroups", c->state()["scene_groups"].toArray().isEmpty());

    // Alinhar e distribuir: painel com 9 + 3 botões; habilitado só com a seleção certa.
    selectIds(c, firstThree);
    window.openAlign();
    settle(4);
    auto *align = window.alignPopover();
    bool aligned = align && align->isVisible() && align->buttons().size() == 12;
    int enabledWithThree = 0;
    if (align)
        for (auto *button : align->buttons())
            enabledWithThree += button->isEnabled() ? 1 : 0;
    h.add("align:popover-has-grid-and-distribute", aligned, QString::number(align ? align->buttons().size() : -1));
    h.add("align:enabled-with-three-objects", enabledWithThree == 12, QString::number(enabledWithThree));
    h.shot(&window, "shell_POLY_align");
    if (align) {
        // O painel é uma janela própria (popup): captura à parte para olhar o resultado.
        h.shot(align, "shell_POLY_align_popup");
        align->hide();
    }
    selectIds(c, {ids[0]});
    window.openAlign();
    settle(4);
    int enabledWithOne = 0;
    if (auto *popover = window.alignPopover())
        for (auto *button : popover->buttons())
            enabledWithOne += button->isEnabled() ? 1 : 0;
    h.add("align:disabled-with-one-object", enabledWithOne == 0, QString::number(enabledWithOne));
    if (auto *popover = window.alignPopover())
        popover->hide();

    // Pivô: marcador publicado e modo de edição refletido no gizmo.
    window.syncNow();
    const auto gizmo = c->state()["gizmo"].toObject();
    h.add("pivot:marker-published", gizmo["pivot_visible"].toBool() && gizmo["origin_marker_visible"].toBool());
    c->request("edit_pivot");
    window.syncNow();
    h.add("pivot:edit-mode-reflected", c->state()["gizmo"].toObject()["pivot_editing"].toBool());
    c->request("edit_pivot");
    window.syncNow();

    // Gizmos: tamanho e espessura vêm das preferências.
    c->request("set_gizmo_size", {{"size", 110}});
    c->request("set_gizmo_thickness", {{"thickness", 3.5}});
    window.syncNow();
    h.add("gizmo:size-and-thickness-preferences", std::abs(c->state()["gizmo_size"].toDouble() - 110.0) < 0.01 &&
                                                     std::abs(c->state()["gizmo_thickness"].toDouble() - 3.5) < 0.01);
    c->request("set_gizmo_size", {{"size", 72}});
    c->request("set_gizmo_thickness", {{"thickness", 2}});

    // Opacidade global do cubo de navegação: vem das preferências e nunca zera.
    {
        c->request("set_nav_cube_opacity", {{"opacity", 0.4}});
        window.syncNow();
        const double chosen = c->state()["nav_cube_opacity"].toDouble();
        c->request("set_nav_cube_opacity", {{"opacity", 0.0}});
        window.syncNow();
        const double floor = c->state()["nav_cube_opacity"].toDouble();
        c->request("set_nav_cube_opacity", {{"opacity", 1.0}});
        window.syncNow();
        h.add("navcube:opacity-preference", std::abs(chosen - 0.4) < 0.01 && std::abs(floor - 0.2) < 0.01 &&
                                                 std::abs(c->state()["nav_cube_opacity"].toDouble() - 1.0) < 0.01,
              QStringLiteral("%1 / %2").arg(chosen).arg(floor));
    }

    // Cubo de navegação: acertos por face, aresta, canto, bússola, início e menu.
    {
        PetuniaNavCube cube;
        const QPointF center(100, 100);
        const double unit = 17.0;
        cube.setGeometry(center, unit);
        // Vista em perspectiva isométrica: olho em (sen y cos p, sen p, cos y cos p).
        const double yaw = 0.7, pitch = 0.6;
        const QVector3D eye(float(std::sin(yaw) * std::cos(pitch)), float(std::sin(pitch)),
                            float(std::cos(yaw) * std::cos(pitch)));
        const QVector3D forward = -eye;
        const QVector3D right(float(std::cos(yaw)), 0.0f, float(-std::sin(yaw)));
        const QVector3D up = QVector3D::crossProduct(right, forward).normalized();
        cube.setCamera(right, up, forward);
        auto onScreen = [&](const QVector3D &world) {
            return center + QPointF(QVector3D::dotProduct(world, right), -QVector3D::dotProduct(world, up)) * unit;
        };
        const auto top = cube.hitTest(onScreen(QVector3D(0, 1, 0)));
        const auto front = cube.hitTest(onScreen(QVector3D(0, 0, 1)));
        const auto side = cube.hitTest(onScreen(QVector3D(1, 0, 0)));
        h.add("navcube:face-hits-map-to-presets",
              top.type == PetuniaNavCube::Hit::Type::Preset && top.preset == 5 &&
                  front.type == PetuniaNavCube::Hit::Type::Preset && front.preset == 1 &&
                  side.type == PetuniaNavCube::Hit::Type::Preset && side.preset == 3,
              QStringLiteral("topo=%1 frente=%2 direita=%3").arg(top.preset).arg(front.preset).arg(side.preset));
        const auto corner = cube.hitTest(onScreen(QVector3D(1, 1, 1)));
        h.add("navcube:corner-hit-gives-its-direction",
              corner.type == PetuniaNavCube::Hit::Type::Direction && corner.direction == QVector3D(1, 1, 1),
              corner.id);
        const auto edge = cube.hitTest(onScreen(QVector3D(1, 1, 0)));
        h.add("navcube:edge-hit-gives-the-two-faces",
              edge.type == PetuniaNavCube::Hit::Type::Direction && edge.direction == QVector3D(1, 1, 0), edge.id);
        h.add("navcube:outside-hits-nothing", cube.hitTest(QPointF(400, 400)).type == PetuniaNavCube::Hit::Type::None);
        const QRectF bounds = cube.bounds();
        const auto home = cube.hitTest(QPointF(bounds.left() + 2, bounds.top() + 2));
        const auto menu = cube.hitTest(QPointF(bounds.right() - 2, bounds.bottom() - 2));
        h.add("navcube:home-and-menu-hit", home.type == PetuniaNavCube::Hit::Type::Home &&
                                              menu.type == PetuniaNavCube::Hit::Type::Menu,
              QStringLiteral("%1 / %2").arg(home.id, menu.id));
        // Vista Frente alinhada aos eixos: cantos da frente e de trás coincidem; vence a frente.
        cube.setCamera(QVector3D(1, 0, 0), QVector3D(0, 1, 0), QVector3D(0, 0, -1));
        const auto frontCorner = cube.hitTest(center + QPointF(unit, -unit));
        h.add("navcube:coincident-corner-prefers-the-nearer-one",
              frontCorner.type == PetuniaNavCube::Hit::Type::Direction && frontCorner.direction.z() > 0.0f,
              frontCorner.id);
        // Pinta sem falhar em todas as vistas dos seis presets.
        QImage canvas(220, 220, QImage::Format_ARGB32_Premultiplied);
        canvas.fill(Qt::transparent);
        QPainter painter(&canvas);
        cube.setCamera(right, up, forward);
        cube.paint(painter, top, true);
        painter.end();
        h.add("navcube:paints-pixels", canvas.pixelColor(100, 100).alpha() > 0);
    }

    // O gizmo acompanha a cena quando a câmera se move. Gestos reais do mouse
    // (botão do meio: órbita; Shift: pan), conferidos NO MEIO do gesto, com o botão
    // apertado: é quando o usuário vê o gizmo parado, e soltar o botão refaz o estado
    // inteiro (esconderia o defeito). A verdade é um snapshot novo do editor.
    {
        c->request("tool", {{"id", "move"}});
        selectIds(c, {ids[0]});
        settle(4);
        window.syncNow();
        auto *view = window.viewport();
        const auto gizmoOf = [](const QJsonObject &state) {
            return QJsonDocument(state["gizmo"].toObject()).toJson(QJsonDocument::Compact);
        };
        const QPoint from(view->width() / 2, view->height() / 2);
        const auto mouse = [&](QEvent::Type type, const QPoint &at, Qt::MouseButton button, Qt::MouseButtons held,
                               Qt::KeyboardModifiers mods) {
            QMouseEvent event(type, QPointF(at), QPointF(view->mapToGlobal(at)), button, held, mods);
            QCoreApplication::sendEvent(view, &event);
        };
        // Devolve {desenhado, verdade} no meio de um arrasto com o botão do meio.
        const auto dragMidway = [&](Qt::KeyboardModifiers mods) {
            mouse(QEvent::MouseButtonPress, from, Qt::MiddleButton, Qt::MiddleButton, mods);
            for (int step = 1; step <= 12; ++step)
                mouse(QEvent::MouseMove, from + QPoint(step * 9, step * 4), Qt::NoButton, Qt::MiddleButton, mods);
            const QByteArray painted = gizmoOf(view->paintedOverlayState());
            const QByteArray truth = gizmoOf(petuniaRequest("snapshot"));
            mouse(QEvent::MouseButtonRelease, from + QPoint(108, 48), Qt::MiddleButton, Qt::NoButton, mods);
            settle(2);
            return std::pair<QByteArray, QByteArray>(painted, truth);
        };
        const QJsonObject first = view->paintedOverlayState();
        const bool visible = first["gizmo"].toObject()["visible"].toBool();
        const QByteArray start = gizmoOf(first);
        const auto orbit = dragMidway(Qt::NoModifier);
        const QByteArray afterOrbit = gizmoOf(view->paintedOverlayState());
        const auto pan = dragMidway(Qt::ShiftModifier);
        h.add("viewport:gizmo-visible-for-the-camera-test", visible);
        h.add("viewport:gizmo-follows-the-orbit-mid-drag", orbit.first == orbit.second,
              QStringLiteral("o gizmo desenhado difere do snapshot novo durante a órbita"));
        h.add("viewport:orbit-moved-the-gizmo", orbit.second != start,
              QStringLiteral("a órbita não alterou o gizmo (o teste não discrimina)"));
        h.add("viewport:gizmo-follows-the-pan-mid-drag", pan.first == pan.second,
              QStringLiteral("o gizmo desenhado difere do snapshot novo durante o pan"));
        h.add("viewport:pan-moved-the-gizmo", pan.second != afterOrbit,
              QStringLiteral("o pan não alterou o gizmo (o teste não discrimina)"));

        // Arrastar uma seta do gizmo (Mover): NO MEIO do gesto o gizmo desenhado tem de
        // estar onde o objeto está, não onde estava quando o botão foi apertado.
        {
            const auto originOf = [](const QJsonObject &state) {
                const auto gizmo = state["gizmo"].toObject();
                return QPointF(gizmo["origin_x"].toDouble(), gizmo["origin_y"].toDouble());
            };
            const auto startState = petuniaRequest("snapshot");
            const QStringList rod = startState["gizmo"].toObject()["x_commands"].toString().split(' ', Qt::SkipEmptyParts);
            bool placed = false;
            QPointF begin, direction;
            if (rod.size() >= 6) {
                begin = QPointF(rod[1].toDouble(), rod[2].toDouble());
                const QPointF tip(rod[4].toDouble(), rod[5].toDouble());
                const QPointF delta = tip - begin;
                const double length = std::hypot(delta.x(), delta.y());
                if (length > 8 && view->rect().contains((begin + delta * 0.6).toPoint())) {
                    direction = delta / length;
                    placed = true;
                    begin = begin + delta * 0.6;
                }
            }
            h.add("viewport:gizmo-arrow-is-reachable-for-the-drag-test", placed);
            if (placed) {
                const QPointF before = originOf(startState);
                mouse(QEvent::MouseMove, begin.toPoint(), Qt::NoButton, Qt::NoButton, Qt::NoModifier); // hover
                mouse(QEvent::MouseButtonPress, begin.toPoint(), Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                const QString afterPress = petuniaRequest("snapshot")["status_message"].toString();
                for (int step = 1; step <= 8; ++step)
                    mouse(QEvent::MouseMove, (begin + direction * (step * 12.0)).toPoint(), Qt::NoButton,
                          Qt::LeftButton, Qt::NoModifier);
                const QPointF drawn = originOf(view->paintedOverlayState());
                const QPointF truth = originOf(petuniaRequest("snapshot"));
                mouse(QEvent::MouseButtonRelease, (begin + direction * 96.0).toPoint(), Qt::LeftButton, Qt::NoButton,
                      Qt::NoModifier);
                settle(2);
                h.add("viewport:object-moved-by-the-gizmo-drag", QLineF(before, truth).length() > 5.0,
                      QStringLiteral("%1 px; status após o clique: \"%2\"; ferramenta: %3")
                          .arg(QLineF(before, truth).length())
                          .arg(afterPress, startState["active_tool"].toString()));
                h.add("viewport:gizmo-follows-the-object-while-dragging", QLineF(drawn, truth).length() < 0.5,
                      QStringLiteral("desenhado %1,%2 / objeto %3,%4").arg(drawn.x()).arg(drawn.y()).arg(truth.x()).arg(truth.y()));
                c->request("undo");
                settle(3);
                window.syncNow();
            }
        }

        // Ferramenta Medir com cliques reais: o primeiro marca, o segundo grava; limpar remove.
        {
            c->request("tool", {{"id", "measure"}});
            settle(2);
            window.syncNow();
            const QPoint first(int(view->width() * 0.3), int(view->height() * 0.6));
            const QPoint second(int(view->width() * 0.7), int(view->height() * 0.6));
            mouse(QEvent::MouseButtonPress, first, Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
            mouse(QEvent::MouseButtonRelease, first, Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
            window.syncNow();
            const bool pendingAfterFirst = c->state()["measure_pending"].toBool();
            mouse(QEvent::MouseMove, (first + second) / 2, Qt::NoButton, Qt::NoButton, Qt::NoModifier);
            const bool previewing = view->paintedOverlayState()["measure_visible"].toBool();
            mouse(QEvent::MouseButtonPress, second, Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
            mouse(QEvent::MouseButtonRelease, second, Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
            window.syncNow();
            const int tags = c->state()["measure_tags"].toArray().size();
            h.add("measure:first-click-marks-and-preview-follows", pendingAfterFirst && previewing,
                  QStringLiteral("pendente %1, prévia %2").arg(pendingAfterFirst).arg(previewing));
            h.add("measure:second-click-stores-the-measurement",
                  !c->state()["measure_pending"].toBool() && tags >= 1, QStringLiteral("%1 etiqueta(s)").arg(tags));
            c->request("command", {{"id", "measure.clear"}});
            window.syncNow();
            h.add("measure:clear-removes-the-stored-measurements", c->state()["measure_tags"].toArray().isEmpty());
            c->request("tool", {{"id", "move"}});
            settle(2);
        }

        // Gizmo universal: o Selecionar e o Transformar mostram setas, anéis e cubos de escala;
        // Mover só setas e planos, Girar só anéis, Escalar só cubos (mesma regra da detecção de cliques).
        {
            using F = PetuniaGlWidget;
            const auto select = F::gizmoFamiliesFor("select");
            const auto transform = F::gizmoFamiliesFor("transform");
            const auto move = F::gizmoFamiliesFor("move");
            const auto rotate = F::gizmoFamiliesFor("rotate");
            const auto scale = F::gizmoFamiliesFor("scale");
            h.add("gizmo:select-shows-move-rotate-and-scale-handles",
                  select.arrows && select.rings && select.scaleBoxes && select.universal);
            h.add("gizmo:transform-shows-move-rotate-and-scale-handles",
                  transform.arrows && transform.rings && transform.scaleBoxes && transform.universal);
            h.add("gizmo:move-rotate-and-scale-keep-their-own-handles",
                  move.arrows && !move.rings && !move.scaleBoxes && !move.universal && rotate.rings &&
                      !rotate.arrows && !rotate.scaleBoxes && scale.scaleBoxes && !scale.arrows && !scale.rings);
        }
    }
}

/// Evento de ponteiro com posição local e global coerentes (o filtro da moldura usa as duas).
void mouseEvent(QWidget *w, QEvent::Type type, QPoint local, Qt::MouseButton button, Qt::MouseButtons buttons,
                Qt::KeyboardModifiers mods = Qt::NoModifier) {
    QMouseEvent e(type, QPointF(local), QPointF(w->mapToGlobal(local)), button, buttons, mods);
    QCoreApplication::sendEvent(w, &e);
}

PetuniaPieMenu *visiblePie() {
    for (QWidget *w : QApplication::topLevelWidgets())
        if (auto *pie = qobject_cast<PetuniaPieMenu *>(w); pie && pie->isVisible())
            return pie;
    return nullptr;
}

bool namesContain(const QJsonArray &palettes, const QString &name) {
    for (const auto &p : palettes)
        if (p.toObject()["name"].toString() == name)
            return true;
    return false;
}

/// Onda B (ADR-C16): seletor de cor e paletas, diálogos com moldura própria, grade e
/// eixos, travas de transform, menus em torta, eco do pincel e quadro de prévia.
void checkWaveB(Harness &h, PetuniaShellWindow &window, PetuniaShellController *c) {
    // ── B1: paletas com CRUD pelo documento, hex e modos ──
    {
        PetuniaColorPanel panel;
        panel.resize(520, 360);
        panel.refreshPalettes();
        const bool created = panel.createPalette(QStringLiteral("Harness B1"));
        h.add("color:create-palette-reaches-the-document",
              created && namesContain(c->request("snapshot")["palettes"].toArray(), QStringLiteral("Harness B1")),
              panel.currentPaletteId());
        panel.setColor(QColor(255, 0, 0));
        panel.addCurrentColorToPalette();
        h.add("color:plus-adds-the-color-to-the-palette", panel.swatches()->count() == 1,
              QString::number(panel.swatches()->count()));
        const bool renamed = panel.renamePalette(QStringLiteral("Harness B1 nova"));
        h.add("color:rename-palette",
              renamed && namesContain(c->request("snapshot")["palettes"].toArray(), QStringLiteral("Harness B1 nova")));
        const bool deleted = panel.deleteCurrentPalette();
        h.add("color:delete-palette",
              deleted && !namesContain(c->request("snapshot")["palettes"].toArray(), QStringLiteral("Harness B1 nova")));
        QMetaObject::invokeMethod(panel.hexEditor(), "textEdited", Q_ARG(QString, QStringLiteral("#336699")));
        h.add("color:hex-sets-the-color", panel.color().name() == QLatin1String("#336699"), panel.color().name());
        panel.setMode(1);
        h.add("color:box-mode", panel.mode() == 1);
        panel.setMode(0);
    }
    c->request("workspace", {{"id", "PAINT"}});
    settle(8);
    window.syncNow();
    settle(4);
    h.add("color:rail-swatch-visible-in-paint",
          window.toolRail()->colorSwatch() && window.toolRail()->colorSwatch()->isVisible());
    h.add("color:brush-form-embeds-the-color-panel",
          window.properties()->findChild<PetuniaColorPanel *>() != nullptr);

    // ── B2: cursor do pincel espelhado entre 3D e 2D ──
    {
        // Checks anteriores giram a câmera e movem o cubo: em vez de um pixel fixo,
        // parte de um ponto da textura que aparece na vista (2D → 3D), aponta o
        // cursor 3D ali (3D → textura) e volta (textura → mesmo pixel).
        auto *vp = window.viewport();
        QPointF view;
        bool found = false;
        for (int iy = 1; iy < 10 && !found; ++iy)
            for (int ix = 1; ix < 10 && !found; ++ix) {
                const auto reply = c->request("brush_texture_hover", {{"x", ix / 10.0}, {"y", iy / 10.0}});
                const auto v = reply["brush_mirror"].toObject()["view"].toArray();
                if (v.size() == 2 && QRectF(vp->rect()).adjusted(4, 4, -4, -4).contains(QPointF(v[0].toDouble(), v[1].toDouble()))) {
                    view = QPointF(v[0].toDouble(), v[1].toDouble());
                    found = true;
                }
            }
        h.add("brush:2d-cursor-maps-to-the-view", found);
        const auto from3d = c->request("brush_footprint", {{"x", view.x()}, {"y", view.y()}});
        const auto texture = from3d["brush_mirror"].toObject()["texture"].toArray();
        h.add("brush:3d-cursor-maps-to-the-texture", found && texture.size() == 2,
              QStringLiteral("%1 em %2,%3 (vista %4x%5)")
                  .arg(QString::fromUtf8(QJsonDocument(from3d["brush_mirror"].toObject()).toJson(QJsonDocument::Compact)))
                  .arg(view.x()).arg(view.y()).arg(vp->width()).arg(vp->height()));
        if (texture.size() == 2) {
            const auto back = c->request("brush_texture_hover", {{"x", texture[0].toDouble()}, {"y", texture[1].toDouble()}});
            const auto v = back["brush_mirror"].toObject()["view"].toArray();
            const double distance =
                v.size() == 2 ? std::hypot(v[0].toDouble() - view.x(), v[1].toDouble() - view.y()) : 1e9;
            h.add("brush:2d-cursor-maps-back-to-the-same-view-point", distance < 3.0,
                  QStringLiteral("distância %1 px").arg(distance, 0, 'f', 2));
        } else {
            h.add("brush:2d-cursor-maps-back-to-the-same-view-point", false, QStringLiteral("sem ponto da textura"));
        }
        const auto cleared = c->request("brush_mirror_clear")["brush_mirror"].toObject();
        h.add("brush:mirror-clears", cleared["texture"].isNull() && cleared["view"].isNull());
    }
    c->request("workspace", {{"id", "POLY"}});
    settle(8);
    window.syncNow();
    settle(4);
    h.add("color:rail-swatch-hidden-outside-paint",
          window.toolRail()->colorSwatch() && !window.toolRail()->colorSwatch()->isVisible());

    // ── B5: moldura própria — arrastar, redimensionar, teclado e geometria lembrada ──
    {
        QDialog dialog(&window);
        dialog.setWindowTitle(QStringLiteral("Harness"));
        auto *layout = new QVBoxLayout(&dialog);
        layout->addWidget(new QPushButton(QStringLiteral("ok"), &dialog));
        auto *chrome = PetuniaDialogChrome::install(&dialog, QStringLiteral("harness-b5"));
        dialog.resize(360, 240);
        dialog.move(200, 200);
        dialog.show();
        settle(6);
        QWidget *bar = chrome ? chrome->titleBar() : nullptr;
        h.add("dialog:frameless-with-own-title-bar",
              bar && bar->isVisible() && (dialog.windowFlags() & Qt::FramelessWindowHint));
        if (bar) {
            const QPoint before = dialog.pos();
            const QPoint grip = bar->rect().center();
            mouseEvent(bar, QEvent::MouseButtonPress, grip, Qt::LeftButton, Qt::LeftButton);
            mouseEvent(bar, QEvent::MouseMove, grip + QPoint(40, 25), Qt::NoButton, Qt::LeftButton);
            mouseEvent(bar, QEvent::MouseButtonRelease, grip + QPoint(40, 25), Qt::LeftButton, Qt::NoButton);
            settle(2);
            h.add("dialog:title-bar-drag-moves", dialog.pos() - before == QPoint(40, 25),
                  QStringLiteral("%1,%2").arg(dialog.x() - before.x()).arg(dialog.y() - before.y()));
        }
        const int widthBefore = dialog.width();
        const QPoint edge(dialog.width() - 2, dialog.height() / 2);
        mouseEvent(&dialog, QEvent::MouseButtonPress, edge, Qt::LeftButton, Qt::LeftButton);
        mouseEvent(&dialog, QEvent::MouseMove, edge + QPoint(50, 0), Qt::NoButton, Qt::LeftButton);
        mouseEvent(&dialog, QEvent::MouseButtonRelease, edge + QPoint(50, 0), Qt::LeftButton, Qt::NoButton);
        settle(2);
        h.add("dialog:right-edge-drag-resizes", dialog.width() == widthBefore + 50,
              QStringLiteral("%1 → %2").arg(widthBefore).arg(dialog.width()));
        const QPoint keyBefore = dialog.pos();
        QKeyEvent right(QEvent::KeyPress, Qt::Key_Right, Qt::ControlModifier | Qt::AltModifier);
        QCoreApplication::sendEvent(&dialog, &right);
        const int heightBefore = dialog.height();
        QKeyEvent grow(QEvent::KeyPress, Qt::Key_Down, Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier);
        QCoreApplication::sendEvent(&dialog, &grow);
        settle(2);
        h.add("dialog:keyboard-moves-and-resizes",
              dialog.x() > keyBefore.x() && dialog.height() > heightBefore,
              QStringLiteral("dx=%1 dh=%2").arg(dialog.x() - keyBefore.x()).arg(dialog.height() - heightBefore));
        const QRect closedAt = dialog.geometry();
        dialog.reject();
        settle(2);
        h.add("dialog:geometry-is-remembered", PetuniaDialogChrome::rememberedGeometry(QStringLiteral("harness-b5")) == closedAt,
              QStringLiteral("%1,%2 %3x%4").arg(closedAt.x()).arg(closedAt.y()).arg(closedAt.width()).arg(closedAt.height()));
    }
    // O cartão da operação (ex.: "Adicionar Icosfera") arrasta dentro da vista.
    {
        c->command(QStringLiteral("model.add_cube"));
        settle(6);
        window.syncNow();
        settle(4);
        auto *hud = window.hud();
        const bool shown = hud && hud->isVisible();
        h.add("hud:visible-for-a-new-primitive", shown, hud ? hud->activeSections().join(",") : QString());
        if (shown) {
            const QPoint before = hud->pos();
            mouseEvent(hud, QEvent::MouseButtonPress, QPoint(12, 12), Qt::LeftButton, Qt::LeftButton);
            mouseEvent(hud, QEvent::MouseMove, QPoint(72, 52), Qt::NoButton, Qt::LeftButton);
            mouseEvent(hud, QEvent::MouseButtonRelease, QPoint(72, 52), Qt::LeftButton, Qt::NoButton);
            settle(2);
            h.add("hud:card-drags-inside-the-view", hud->hasCustomPlacement() && hud->pos() != before,
                  QStringLiteral("%1,%2 → %3,%4").arg(before.x()).arg(before.y()).arg(hud->x()).arg(hud->y()));
            mouseEvent(hud, QEvent::MouseButtonDblClick, QPoint(12, 12), Qt::LeftButton, Qt::LeftButton);
            settle(2);
            h.add("hud:double-click-returns-to-the-corner", !hud->hasCustomPlacement());
        }
        c->request("cancel_primitive");
        settle(4);
        window.syncNow();
    }

    // ── B6: grade e eixos ──
    {
        auto *header = window.viewportHeader();
        auto grid = [&] { return c->request("snapshot")["viewport_grid"].toObject(); };
        const auto initial = grid();
        const auto axes = initial["axes"].toArray();
        h.add("grid:y-axis-hidden-by-default",
              axes.size() == 3 && axes[0].toBool() && !axes[1].toBool() && axes[2].toBool());
        auto *button = header->gridButton();
        h.add("grid:header-button-reflects-the-grid", button && button->isChecked() == initial["visible"].toBool(true));
        if (button) {
            button->click();
            settle(4);
            const bool hidden = !grid()["visible"].toBool(true);
            button->click();
            settle(4);
            h.add("grid:header-button-toggles", hidden && grid()["visible"].toBool());
        }
        window.syncNow();
        auto *yAxis = header->gridOptions() ? header->gridOptions()->findChild<QCheckBox *>(QStringLiteral("grid-axis-y"))
                                            : nullptr;
        if (yAxis) {
            yAxis->click();
            settle(4);
            const bool shown = grid()["axes"].toArray().at(1).toBool();
            yAxis->click();
            settle(4);
            h.add("grid:axis-checkbox-toggles-the-axis", shown && !grid()["axes"].toArray().at(1).toBool());
        } else {
            h.add("grid:axis-checkbox-toggles-the-axis", false, QStringLiteral("sem caixa do eixo Y"));
        }
        c->request("set_viewport_grid", {{"size", 50}, {"spacing", 0.25}, {"opacity", 0.6}});
        const auto custom = grid();
        h.add("grid:size-and-tile-are-configurable",
              std::abs(custom["size"].toDouble() - 50) < 1e-4 && std::abs(custom["spacing"].toDouble() - 0.25) < 1e-4);
        c->request("set_viewport_grid", {{"size", 50}, {"spacing", 0}, {"opacity", 0.6}});
        h.add("grid:invalid-tile-is-refused", std::abs(grid()["spacing"].toDouble() - 0.25) < 1e-4);
        c->request("set_viewport_grid", {{"size", 20}, {"spacing", 1}, {"opacity", 0.5}});
    }

    // ── B7: travas de transform (documento, campo e recusa) ──
    {
        const auto items = c->state()["scene_items"].toArray();
        if (!items.isEmpty())
            c->request("select", {{"id", items.first().toObject()["id"].toString()}});
        settle(4);
        window.syncNow();
        settle(4);
        PetuniaVector3Field *position = nullptr;
        for (auto *field : window.properties()->findChildren<PetuniaVector3Field *>())
            if (field->property("lockKind").toString() == QLatin1String("position"))
                position = field;
        h.add("lock:transform-fields-have-padlocks", position && position->field(1)->isLockable());
        c->request("transform_lock", {{"kind", "position"}, {"axis", 1}, {"locked", true}});
        window.syncNow();
        settle(2);
        const auto locks = c->request("snapshot")["transform_locks"].toObject()["position"].toArray();
        h.add("lock:request-locks-the-channel", locks.at(1).toBool());
        h.add("lock:field-shows-the-padlock-closed", position && position->isLocked(1));
        const double y = c->request("snapshot")["position"].toArray().at(1).toDouble();
        c->request("transform", {{"kind", "position"}, {"axis", 1}, {"text", QString::number(y + 3.0)}});
        h.add("lock:typed-value-on-locked-axis-is-refused",
              std::abs(c->request("snapshot")["position"].toArray().at(1).toDouble() - y) < 1e-5);
        if (position) {
            auto *field = position->field(1);
            const QPoint lock = field->lockRect().center();
            mouseEvent(field, QEvent::MouseButtonPress, lock, Qt::LeftButton, Qt::LeftButton);
            mouseEvent(field, QEvent::MouseButtonRelease, lock, Qt::LeftButton, Qt::NoButton);
            settle(4);
            window.syncNow();
        }
        h.add("lock:clicking-the-padlock-unlocks",
              !c->request("snapshot")["transform_locks"].toObject()["position"].toArray().at(1).toBool());
        PetuniaNumberField solo;
        solo.setLockable(true);
        solo.resize(160, 28);
        bool requested = false;
        QObject::connect(&solo, &PetuniaNumberField::lockToggled, [&](const QString &, bool locked) { requested = locked; });
        QKeyEvent altL(QEvent::KeyPress, Qt::Key_L, Qt::AltModifier, QStringLiteral("l"));
        QCoreApplication::sendEvent(solo.valueEditor(), &altL);
        h.add("lock:alt-l-asks-to-lock", requested);
    }

    // ── B9: menus em torta ──
    {
        const QPoint at = window.viewport()->mapToGlobal(window.viewport()->rect().center());
        QPointer<PetuniaPieMenu> pie = window.openPie(QStringLiteral("select"), at);
        settle(4);
        h.add("pie:opens-with-the-selection-domains", pie && pie->isVisible() && pie->count() == 4);
        if (pie) {
            h.add("pie:direction-picks-the-item",
                  pie->indexForOffset({0, -100}) == 0 && pie->indexForOffset({100, 0}) == 1 &&
                      pie->indexForOffset({0, 100}) == 2 && pie->indexForOffset({-100, 0}) == 3 &&
                      pie->indexForOffset({3, 3}) == -1);
            h.add("pie:items-are-named-buttons",
                  pie->button(1) && pie->button(1)->objectName() == QLatin1String("pie-item-vertex") &&
                      !pie->button(1)->accessibleName().isEmpty());
            QKeyEvent two(QEvent::KeyPress, Qt::Key_2, Qt::NoModifier, QStringLiteral("2"));
            QCoreApplication::sendEvent(pie, &two);
            settle(4);
        }
        h.add("pie:number-key-activates", c->request("snapshot")["selection_domain"].toString() == QLatin1String("Vertex"),
              c->request("snapshot")["selection_domain"].toString());
        h.add("pie:closes-after-choosing", !visiblePie());
        c->request("domain", {{"id", "object"}});
        settle(2);
        const int sceneBefore = c->request("snapshot")["scene_items"].toArray().size();
        QPointer<PetuniaPieMenu> add = window.openPie(QStringLiteral("add"), at);
        settle(4);
        h.add("pie:add-offers-eight-primitives", add && add->count() == 8);
        if (add) {
            QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
            QCoreApplication::sendEvent(add, &esc);
            settle(4);
        }
        h.add("pie:escape-cancels-without-effect",
              !visiblePie() && c->request("snapshot")["scene_items"].toArray().size() == sceneBefore);
        QKeyEvent altD(QEvent::KeyPress, Qt::Key_D, Qt::AltModifier, QStringLiteral("d"));
        window.viewport()->setFocus();
        settle(1);
        QCoreApplication::sendEvent(window.viewport(), &altD);
        settle(4);
        PetuniaPieMenu *byKey = visiblePie();
        h.add("pie:shortcut-in-the-viewport-opens-it", byKey && byKey->title() == window.tr("Selecionar"),
              byKey ? byKey->title() : QStringLiteral("nenhuma"));
        if (byKey)
            byKey->close();
        settle(2);
        PetuniaPieMenu::setListFallback(true);
        window.openPie(QStringLiteral("view"), at);
        settle(4);
        QMenu *list = nullptr;
        for (QWidget *w : QApplication::topLevelWidgets())
            if (auto *menu = qobject_cast<QMenu *>(w); menu && menu->isVisible() && menu->objectName() == QLatin1String("PetuniaPieList"))
                list = menu;
        h.add("pie:list-fallback-has-the-same-items", list && list->actions().size() >= 8,
              list ? QString::number(list->actions().size()) : QStringLiteral("sem lista"));
        if (list)
            list->close();
        PetuniaPieMenu::setListFallback(false);
        settle(2);
    }

    // ── B3: quadro de prévia da ferramenta (âncora, seta, valor) ──
    {
        c->request("domain", {{"id", "face"}});
        c->request("select_all");
        settle(2);
        c->command(QStringLiteral("model.extrude"));
        c->request("set_tool_modal_value", {{"value", 0.5}});
        const auto frame = c->request("snapshot")["tool_preview"].toObject();
        h.add("preview:extrude-publishes-anchor-arrow-and-value",
              frame["tool"].toString() == QLatin1String("extrude") && frame["anchor"].toArray().size() == 2 &&
                  frame["tip"].toArray().size() == 2 && std::abs(frame["value"].toDouble() - 0.5) < 1e-4,
              QString::fromUtf8(QJsonDocument(frame).toJson(QJsonDocument::Compact)));
        window.syncNow();
        settle(2);
        h.shot(window.viewport(), "wave_b_extrude_preview");
        c->request("cancel_tool_modal");
        h.add("preview:gone-after-cancel", c->request("snapshot")["tool_preview"].isNull());
        c->request("domain", {{"id", "object"}});
        settle(4);
        window.syncNow();
    }
}


/// Fechamento da Onda B: política de seleção, orientação, alças do Selecionar,
/// cor compacta e campos densos no painel.
void checkWaveBClosing(Harness &h, PetuniaShellWindow &window, PetuniaShellController *c) {
    c->request("workspace", {{"id", "POLY"}});
    settle(6);
    window.syncNow();
    settle(2);
    auto *header = window.viewportHeader();
    auto policy = [&] { return c->request("snapshot")["select_policy"].toObject(); };
    h.add("select-policy:defaults-to-automatic", policy()["id"].toString() == QLatin1String("auto"));
    QList<QAction *> policyActions =
        header->selectPolicyButton() && header->selectPolicyButton()->menu() ? header->selectPolicyButton()->menu()->actions()
                                                                             : QList<QAction *>();
    h.add("select-policy:header-offers-three-policies", policyActions.size() == 3,
          QString::number(policyActions.size()));
    c->command(QStringLiteral("view.toggle_xray"));
    settle(2);
    h.add("select-policy:automatic-selects-through-in-xray", policy()["through_now"].toBool());
    if (policyActions.size() == 3) {
        policyActions[1]->trigger();
        settle(2);
        h.add("select-policy:visible-only-wins-over-xray",
              policy()["id"].toString() == QLatin1String("visible") && !policy()["through_now"].toBool());
        policyActions[0]->trigger();
        settle(2);
    }
    c->command(QStringLiteral("view.toggle_xray"));
    settle(2);
    window.syncNow();
    h.add("select-policy:state-is-announced", !header->selectPolicyButton()->accessibleDescription().isEmpty(),
          header->selectPolicyButton()->accessibleDescription());

    QList<QAction *> orientationActions;
    if (header->orientationButton() && header->orientationButton()->menu())
        for (auto *action : header->orientationButton()->menu()->actions())
            if (action->isCheckable())
                orientationActions << action;
    h.add("orientation:header-offers-global-local-normal-view", orientationActions.size() == 4,
          QString::number(orientationActions.size()));
    if (orientationActions.size() == 4) {
        orientationActions[1]->trigger();
        settle(2);
        h.add("orientation:local-reaches-the-session",
              c->request("snapshot")["transform_orientation"].toString() == QLatin1String("local"));
        orientationActions[0]->trigger();
        settle(2);
    }

    c->request("set_select_gizmo_rotate", {{"enabled", false}});
    const bool rotateOff = !c->request("snapshot")["select_gizmo_rotate"].toBool(true);
    c->request("set_select_gizmo_rotate", {{"enabled", true}});
    h.add("select-gizmo:rotate-handles-can-be-hidden",
          rotateOff && c->request("snapshot")["select_gizmo_rotate"].toBool());

    bool dense = true;
    QString tall;
    for (auto *field : window.properties()->findChildren<PetuniaNumberField *>()) {
        if (field->isVisible() && field->height() > PetuniaTheme::instance().fieldHeight()) {
            dense = false;
            tall = QStringLiteral("%1=%2").arg(field->accessibleName()).arg(field->height());
        }
    }
    h.add("panel:number-fields-use-the-dense-height", dense, tall);

    c->request("workspace", {{"id", "PAINT"}});
    settle(8);
    window.syncNow();
    settle(4);
    auto *colorPanel = window.properties()->findChild<PetuniaColorPanel *>();
    h.add("color:brush-panel-is-compact", colorPanel && colorPanel->isCompact() &&
                                              colorPanel->channelsToggle()->isVisible());
    h.shot(&window, "shell_PAINT_compact");
    c->request("workspace", {{"id", "POLY"}});
    settle(6);
    window.syncNow();
    h.shot(&window, "shell_POLY_dense");
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

        // Vista infinita: roda/trackpad, botão direito arrastando, Espaço+esquerdo, F e botão.
        {
            auto *surface = canvas->viewport();
            const QPoint mid(surface->width() / 2, surface->height() / 2);
            const auto wheel = [&](QPoint pixels, QPoint angle, const QPoint &at, Qt::KeyboardModifiers mods) {
                QWheelEvent event(QPointF(at), QPointF(surface->mapToGlobal(at)), pixels, angle, Qt::NoButton, mods,
                                  Qt::NoScrollPhase, false);
                QCoreApplication::sendEvent(surface, &event);
            };
            const auto mouse = [&](QEvent::Type type, const QPoint &at, Qt::MouseButton button, Qt::MouseButtons held) {
                QMouseEvent event(type, QPointF(at), QPointF(surface->mapToGlobal(at)), button, held, Qt::NoModifier);
                QCoreApplication::sendEvent(surface, &event);
            };
            const auto nodesVisible = [&] {
                const QRectF seen = canvas->mapToScene(surface->rect()).boundingRect();
                for (const auto &id : canvas->nodeIds())
                    if (!seen.contains(canvas->nodeSceneRect(id)))
                        return false;
                return !canvas->nodeIds().isEmpty();
            };
            const auto sceneAt = [&](const QPoint &at) { return canvas->mapToScene(at); };

            const QPointF before = sceneAt(mid);
            wheel(QPoint(60, 40), QPoint(), mid, Qt::NoModifier); // trackpad: dois eixos
            const QPointF afterPad = sceneAt(mid);
            h.add("nodes:trackpad-pans-both-axes", std::abs(afterPad.x() - before.x()) > 1 && std::abs(afterPad.y() - before.y()) > 1,
                  QStringLiteral("%1,%2").arg(afterPad.x() - before.x()).arg(afterPad.y() - before.y()));

            wheel(QPoint(), QPoint(120, 0), mid, Qt::NoModifier); // roda inclinada: horizontal
            const QPointF afterTilt = sceneAt(mid);
            h.add("nodes:tilted-wheel-pans-horizontally", std::abs(afterTilt.x() - afterPad.x()) > 1 && std::abs(afterTilt.y() - afterPad.y()) < 0.5);

            wheel(QPoint(), QPoint(0, 120), mid, Qt::ShiftModifier); // Shift+roda: vertical
            const QPointF afterShift = sceneAt(mid);
            h.add("nodes:shift-wheel-pans-vertically", std::abs(afterShift.y() - afterTilt.y()) > 1 && std::abs(afterShift.x() - afterTilt.x()) < 0.5);

            // Zoom ancorado: o ponto sob o cursor não se mexe.
            const QPoint aside = mid + QPoint(120, 40);
            const QPointF underCursor = sceneAt(aside);
            const qreal zoom0 = canvas->transform().m11();
            wheel(QPoint(), QPoint(0, 120), aside, Qt::NoModifier);
            const QPointF stillThere = sceneAt(aside);
            h.add("nodes:wheel-zooms-under-the-cursor",
                  canvas->transform().m11() > zoom0 && QLineF(underCursor, stillThere).length() < 1.5,
                  QStringLiteral("%1 px").arg(QLineF(underCursor, stillThere).length()));

            // Botão direito: arrastar faz pan (sem abrir menu).
            const QPointF beforeDrag = sceneAt(mid);
            mouse(QEvent::MouseButtonPress, mid, Qt::RightButton, Qt::RightButton);
            for (int step = 1; step <= 8; ++step)
                mouse(QEvent::MouseMove, mid + QPoint(step * 12, step * 6), Qt::NoButton, Qt::RightButton);
            mouse(QEvent::MouseButtonRelease, mid + QPoint(96, 48), Qt::RightButton, Qt::NoButton);
            const QPointF afterDrag = sceneAt(mid);
            h.add("nodes:right-drag-pans-without-a-menu",
                  QLineF(beforeDrag, afterDrag).length() > 20 && !QApplication::activePopupWidget(),
                  QStringLiteral("%1").arg(QLineF(beforeDrag, afterDrag).length()));

            // Espaço + esquerdo arrastando.
            const QPointF beforeSpace = sceneAt(mid);
            QKeyEvent spaceDown(QEvent::KeyPress, Qt::Key_Space, Qt::NoModifier, QStringLiteral(" "));
            QCoreApplication::sendEvent(canvas, &spaceDown);
            mouse(QEvent::MouseButtonPress, mid, Qt::LeftButton, Qt::LeftButton);
            for (int step = 1; step <= 6; ++step)
                mouse(QEvent::MouseMove, mid + QPoint(step * 10, 0), Qt::NoButton, Qt::LeftButton);
            mouse(QEvent::MouseButtonRelease, mid + QPoint(60, 0), Qt::LeftButton, Qt::NoButton);
            QKeyEvent spaceUp(QEvent::KeyRelease, Qt::Key_Space, Qt::NoModifier, QStringLiteral(" "));
            QCoreApplication::sendEvent(canvas, &spaceUp);
            h.add("nodes:space-drag-pans", QLineF(beforeSpace, sceneAt(mid)).length() > 20);

            // A legenda de tipos é fixa na vista: depois de um pan, a faixa dela é a mesma.
            {
                settle(2);
                const QImage first = canvas->grab().toImage();
                canvas->panBy(QPointF(137, 59));
                settle(2);
                const QImage second = canvas->grab().toImage();
                const QRect legend(10, 10, 150, 26);
                int same = 0, total = 0;
                for (int y = legend.top(); y < legend.bottom() && y < first.height(); ++y)
                    for (int x = legend.left(); x < legend.right() && x < first.width(); ++x, ++total)
                        same += first.pixel(x, y) == second.pixel(x, y) ? 1 : 0;
                h.add("nodes:legend-stays-fixed-while-panning",
                      canvas->viewportUpdateMode() == QGraphicsView::FullViewportUpdate && total > 0 &&
                          same * 100 >= total * 95,
                      QStringLiteral("%1 de %2 pixels iguais").arg(same).arg(total));
            }

            // Longe dos nós a vista continua livre (cena infinita) e "enquadrar" volta.
            canvas->panBy(QPointF(20000, 20000));
            h.add("nodes:view-can-leave-the-graph", !nodesVisible());
            sendKey(canvas, Qt::Key_F);
            h.add("nodes:f-frames-everything", nodesVisible());
            canvas->panBy(QPointF(-15000, 9000));
            const bool lost = !nodesVisible();
            auto *fitButton = canvas->viewport()->findChild<QToolButton *>();
            if (fitButton)
                fitButton->click();
            h.add("nodes:fit-button-frames-everything", lost && fitButton && nodesVisible() && !fitButton->toolTip().isEmpty());
            canvas->panBy(QPointF(-9000, -3000));
            sendKey(canvas, Qt::Key_Home);
            h.add("nodes:home-frames-everything", nodesVisible());
        }

        // Multisseleção e grupos de nós: caixa no vazio, Ctrl+clique, arrastar em bloco,
        // Ctrl+G agrupa, renomear (teclado e duplo clique) e Ctrl+Shift+G desfaz.
        {
            auto *surface = canvas->viewport();
            const auto press = [&](QEvent::Type type, const QPoint &at, Qt::MouseButton button, Qt::MouseButtons held,
                                   Qt::KeyboardModifiers mods = Qt::NoModifier) {
                QMouseEvent event(type, QPointF(at), QPointF(surface->mapToGlobal(at)), button, held, mods);
                QCoreApplication::sendEvent(surface, &event);
                settle(1);
            };
            canvas->fitAll();
            settle(2);
            const QStringList all = canvas->nodeIds();
            QRect bounds;
            for (const auto &id : all)
                bounds = bounds.united(canvas->nodeViewportRect(id));
            const QPoint from = bounds.topLeft() - QPoint(10, 10);
            const QPoint to = bounds.bottomRight() + QPoint(10, 10);
            bool emptyStart = surface->rect().contains(from);
            for (const auto &id : all)
                emptyStart = emptyStart && !canvas->nodeViewportRect(id).contains(from);
            h.add("nodes:box-start-is-empty-space", emptyStart);

            press(QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
            press(QEvent::MouseMove, (from + to) / 2, Qt::NoButton, Qt::LeftButton);
            press(QEvent::MouseMove, to, Qt::NoButton, Qt::LeftButton);
            press(QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton);
            h.add("nodes:box-selects-every-node", canvas->selectedNodeIds().size() == all.size() && all.size() >= 2,
                  QStringLiteral("%1 de %2").arg(canvas->selectedNodeIds().size()).arg(all.size()));
            // Clique no vazio reduz a multisseleção ao nó principal.
            press(QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
            press(QEvent::MouseButtonRelease, from, Qt::LeftButton, Qt::NoButton);
            h.add("nodes:click-on-empty-space-narrows-the-selection", canvas->selectedNodeIds().size() == 1,
                  QString::number(canvas->selectedNodeIds().size()));

            if (all.size() >= 2) {
                const QString a = all[0], b = all[1];
                const auto center = [&](const QString &id) { return canvas->nodeViewportRect(id).center(); };
                press(QEvent::MouseButtonPress, center(a), Qt::LeftButton, Qt::LeftButton);
                press(QEvent::MouseButtonRelease, center(a), Qt::LeftButton, Qt::NoButton);
                press(QEvent::MouseButtonPress, center(b), Qt::LeftButton, Qt::LeftButton, Qt::ControlModifier);
                press(QEvent::MouseButtonRelease, center(b), Qt::LeftButton, Qt::NoButton, Qt::ControlModifier);
                h.add("nodes:ctrl-click-adds-to-the-selection", canvas->selectedNodeIds().size() == 2,
                      QString::number(canvas->selectedNodeIds().size()));

                // Arrastar um nó da multisseleção move os dois, o mesmo deslocamento.
                const QPointF a0 = canvas->nodeScenePosition(a), b0 = canvas->nodeScenePosition(b);
                const QPoint start = center(a);
                press(QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton);
                for (int step = 1; step <= 6; ++step)
                    press(QEvent::MouseMove, start + QPoint(step * 8, step * 4), Qt::NoButton, Qt::LeftButton);
                press(QEvent::MouseButtonRelease, start + QPoint(48, 24), Qt::LeftButton, Qt::NoButton);
                window.syncNow();
                const QPointF da = canvas->nodeScenePosition(a) - a0, db = canvas->nodeScenePosition(b) - b0;
                h.add("nodes:dragging-moves-the-whole-selection",
                      da.manhattanLength() > 10 && QLineF(da, db).length() < 0.5 && canvas->selectedNodeIds().size() == 2,
                      QStringLiteral("A %1,%2  B %3,%4").arg(da.x()).arg(da.y()).arg(db.x()).arg(db.y()));

                // Ctrl+A seleciona todos os nós (o atalho global de selecionar tudo não o engole).
                canvas->selectNodes({a});
                sendKey(canvas, Qt::Key_A, Qt::ControlModifier);
                h.add("nodes:ctrl-a-selects-every-node", canvas->selectedNodeIds().size() == all.size(),
                      QStringLiteral("%1 de %2").arg(canvas->selectedNodeIds().size()).arg(all.size()));
                canvas->selectNodes({a, b});

                // Ctrl+G agrupa a seleção: um quadro com os dois nós.
                sendKey(canvas, Qt::Key_G, Qt::ControlModifier);
                window.syncNow();
                QStringList members = canvas->groupCount() == 1 ? canvas->groupMembers(0) : QStringList();
                members.sort();
                QStringList expected{a, b};
                expected.sort();
                const bool grouped = canvas->groupCount() == 1 && members == expected;
                // Diagnóstico: o que a tecla deixou, ANTES de qualquer pedido de apoio.
                const QString afterKey = QStringLiteral("grupos %1 (estado %2), membros [%3], esperado [%4], selecionados %5")
                                             .arg(canvas->groupCount())
                                             .arg(c->state()["op_graph"].toObject()["groups"].toArray().size())
                                             .arg(members.join(','), expected.join(','))
                                             .arg(canvas->selectedNodeIds().size());
                if (!grouped) {
                    // Apoio para os testes seguintes seguirem com um grupo existente.
                    c->request("graph_group", {{"nodes", QJsonArray{a, b}}});
                    window.syncNow();
                }
                h.add("nodes:ctrl-g-groups-the-selected-nodes", grouped, afterKey);

                // Renomear pelo teclado: o editor abre sobre o título; Enter confirma.
                bool opened = false;
                if (canvas->groupCount() >= 1) {
                    canvas->beginRenameGroup(canvas->groupId(0));
                    auto *editor = surface->findChild<QLineEdit *>();
                    opened = editor && editor->isVisible();
                    if (editor) {
                        editor->setText(QStringLiteral("Parede"));
                        sendKey(editor, Qt::Key_Return);
                    }
                    window.syncNow();
                }
                h.add("nodes:rename-group-by-keyboard",
                      opened && canvas->groupCount() >= 1 && canvas->groupName(0) == QStringLiteral("Parede"),
                      canvas->groupName(0));

                // Duplo clique no título abre o editor; Esc cancela sem mudar o nome.
                bool opensOnDoubleClick = false;
                if (canvas->groupCount() >= 1) {
                    const QPoint title = canvas->groupTitleViewportRect(0).center();
                    QMouseEvent doubleClick(QEvent::MouseButtonDblClick, QPointF(title), QPointF(surface->mapToGlobal(title)),
                                            Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
                    QCoreApplication::sendEvent(surface, &doubleClick);
                    settle(2);
                    auto *second = surface->findChild<QLineEdit *>();
                    opensOnDoubleClick = second && second->isVisible() && second->text() == QStringLiteral("Parede");
                    if (second) {
                        second->setText(QStringLiteral("Descartado"));
                        sendKey(second, Qt::Key_Escape);
                    }
                    window.syncNow();
                }
                h.add("nodes:double-click-renames-and-escape-cancels",
                      opensOnDoubleClick && canvas->groupCount() >= 1 && canvas->groupName(0) == QStringLiteral("Parede"),
                      canvas->groupName(0));

                // Ctrl+Shift+G desfaz o grupo e mantém os nós.
                const int nodesBefore = canvas->nodeCount();
                const QStringList selectedBefore = canvas->selectedNodeIds();
                sendKey(canvas, Qt::Key_G, Qt::ControlModifier | Qt::ShiftModifier);
                window.syncNow();
                h.add("nodes:ctrl-shift-g-ungroups-keeping-the-nodes",
                      canvas->groupCount() == 0 && canvas->nodeCount() == nodesBefore && grouped,
                      QStringLiteral("grupos %1, nós %2/%3, selecionados antes [%4], tecla agrupou antes: %5")
                          .arg(canvas->groupCount())
                          .arg(canvas->nodeCount())
                          .arg(nodesBefore)
                          .arg(selectedBefore.join(','))
                          .arg(grouped));
            }
        }
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
        c->request("undo");
        settle(4);
        window.syncNow();
        h.add("nodes:undo-restores-node-position", std::abs(canvas->nodeScenePosition(opId).x() - before.x()) < 0.5,
              QStringLiteral("%1 vs %2").arg(canvas->nodeScenePosition(opId).x()).arg(before.x()));
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
        // M03: camadas reordenadas, ocultas e removidas pelo teclado na Estrutura.
        {
            auto layerIds = [&] {
                QStringList ids;
                for (const auto &v : c->state()["paint_layers"].toArray())
                    ids << v.toObject()["id"].toString();
                return ids;
            };
            auto *tree = window.structure()->tree();
            tree->setFocus();
            tree->setCurrentIndex(tree->model()->index(0, 0));
            settle(3);
            window.syncNow();
            const auto before = layerIds();
            const int row = tree->currentIndex().row();
            realKey(tree, Qt::Key_Down, Qt::AltModifier);
            window.syncNow();
            const auto moved = layerIds();
            h.add("paint:alt-down-reorders-layer", moved != before && moved.size() == before.size(),
                  before.join(",") + " -> " + moved.join(","));
            realKey(tree, Qt::Key_Up, Qt::AltModifier);
            window.syncNow();
            h.add("paint:alt-up-restores-order", layerIds() == before, layerIds().join(","));
            auto visibleAt = [&](int index) {
                return c->state()["paint_layers"].toArray().at(index).toObject()["visible"].toBool(true);
            };
            const int current = std::max(0, tree->currentIndex().row());
            const bool wasVisible = visibleAt(current);
            realKey(tree, Qt::Key_H);
            window.syncNow();
            h.add("paint:h-toggles-layer-visibility", visibleAt(current) != wasVisible, QString::number(row));
            realKey(tree, Qt::Key_H);
            window.syncNow();
            const int objects = c->state()["scene_items"].toArray().size();
            const int layers = layerIds().size();
            realKey(tree, Qt::Key_Delete);
            window.syncNow();
            h.add("paint:delete-removes-layer-not-object",
                  layerIds().size() == layers - 1 && c->state()["scene_items"].toArray().size() == objects,
                  QStringLiteral("%1 -> %2 camadas, %3 objetos").arg(layers).arg(layerIds().size()).arg(objects));
        }
        checkSurfaceGeometry(h, window, "PAINT");
        h.shot(&window, "shell_PAINT_v2");

        // F7: seções nativas de PAINT no lugar das abas legadas.
        {
            const auto ids = window.properties()->visibleSectionIds();
            h.add("paint:native-property-sections",
                  ids.contains("paint.brush") && ids.contains("paint.layer") && ids.contains("paint.projection") &&
                      !ids.contains("transform") && !ids.contains("uv.unwrap"),
                  ids.join(","));
        }
        // Wireframe UV sobre a pintura: alternância no próprio canvas 2D (sem abrir seção).
        {
            auto *paintCanvas = qobject_cast<PetuniaEditorCanvas *>(window.workspaceCanvas("PAINT"));
            auto *toggle = paintCanvas ? paintCanvas->findChild<QToolButton *>() : nullptr;
            const bool initial = c->flag("paint_show_uv_overlay");
            if (toggle)
                toggle->click();
            settle(3);
            window.syncNow();
            const bool turnedOn = c->flag("paint_show_uv_overlay");
            const bool buttonShowsIt = toggle && toggle->isChecked() == turnedOn;
            if (toggle)
                toggle->click();
            settle(3);
            window.syncNow();
            const bool turnedOff = c->flag("paint_show_uv_overlay");
            h.add("paint:canvas-button-toggles-the-uv-wireframe",
                  toggle && turnedOn == !initial && turnedOff == initial && !toggle->toolTip().isEmpty(),
                  QStringLiteral("%1 -> %2 -> %3").arg(initial).arg(turnedOn).arg(turnedOff));
            h.add("paint:canvas-button-reflects-the-state", buttonShowsIt && toggle->isChecked() == turnedOff);
        }
        // Superfície 3D | Dividido | 2D: escolha do usuário, persistida por workspace.
        {
            auto *modes = window.viewportHeader()->surfaceMode();
            QWidget *canvas = window.workspaceCanvas("PAINT");
            h.add("surface:header-offers-modes-in-paint",
                  modes->isVisibleTo(&window) && modes->button("3d") && modes->button("split") && modes->button("2d"));
            if (modes->button("2d") && canvas) {
                modes->button("2d")->click();
                settle(4);
                h.add("surface:2d-hides-3d-view", canvas->isVisible() && !window.viewport()->isVisible());
                modes->button("3d")->click();
                settle(4);
                h.add("surface:3d-hides-canvas", !canvas->isVisible() && window.viewport()->isVisible());
                modes->button("2d")->click();
                settle(4);
                c->request("workspace", {{"id", "POLY"}});
                settle(6);
                window.syncNow();
                h.add("surface:modeling-has-no-surface-modes", !modes->isVisibleTo(&window) &&
                                                                   window.viewport()->isVisible());
                c->request("workspace", {{"id", "PAINT"}});
                settle(6);
                window.syncNow();
                settle(2);
                h.add("surface:mode-persists-per-workspace",
                      modes->current() == "2d" && canvas->isVisible() && !window.viewport()->isVisible(),
                      modes->current());
                modes->button("split")->click();
                settle(4);
                h.add("surface:split-shows-both", canvas->isVisible() && window.viewport()->isVisible());
            }
        }

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
        {
            const auto ids = window.properties()->visibleSectionIds();
            h.add("uv:native-property-sections",
                  ids.contains("uv.unwrap") && ids.contains("uv.pack") && !ids.contains("paint.brush"), ids.join(","));
        }
        // V03: Estrutura lista as ilhas UV; selecionar a linha seleciona a ilha no documento.
        c->request("command", {{"id", "uv.unwrap_auto"}});
        settle(4);
        window.syncNow();
        settle(2);
        {
            const auto islands = c->state()["uv_editor"].toObject()["islands"].toArray();
            h.add("uv:structure-lists-islands", islands.size() >= 2 && window.structure()->rowCount() == islands.size(),
                  QStringLiteral("%1 linhas / %2 ilhas").arg(window.structure()->rowCount()).arg(islands.size()));
            auto *tree = window.structure()->tree();
            if (tree->model()->rowCount() > 0) {
                tree->selectionModel()->select(tree->model()->index(0, 0),
                                               QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);
                settle(4);
                window.syncNow();
                const auto after = c->state()["uv_editor"].toObject()["islands"].toArray();
                h.add("uv:selecting-island-row-selects-uv",
                      !after.isEmpty() && after[0].toObject()["selected"].toBool() &&
                          c->state()["uv_editor"].toObject()["uv_selected_count"].toInt() ==
                              after[0].toObject()["face_count"].toInt());
            }
        }
        // V03: teclado do editor UV — zoom, pan e nudge da seleção (um Undo por tecla).
        if (auto *uvCanvas = qobject_cast<PetuniaEditorCanvas *>(window.workspaceCanvas("UV"))) {
            const qreal zoom0 = uvCanvas->zoom();
            realKey(uvCanvas, Qt::Key_Plus, Qt::NoModifier, QStringLiteral("+"));
            const qreal zoomIn = uvCanvas->zoom();
            realKey(uvCanvas, Qt::Key_Minus, Qt::NoModifier, QStringLiteral("-"));
            h.add("uv:plus-minus-zoom", zoomIn > zoom0 && std::abs(uvCanvas->zoom() - zoom0) < 0.01,
                  QStringLiteral("%1 -> %2 -> %3").arg(zoom0).arg(zoomIn).arg(uvCanvas->zoom()));
            const QPointF pan0 = uvCanvas->pan();
            const QString rev0 = c->text("document_revision");
            realKey(uvCanvas, Qt::Key_Right, Qt::ControlModifier);
            h.add("uv:ctrl-arrow-pans-without-editing",
                  uvCanvas->pan().x() > pan0.x() && c->text("document_revision") == rev0);
            realKey(uvCanvas, Qt::Key_Right);
            window.syncNow();
            const QString rev1 = c->text("document_revision");
            h.add("uv:arrow-nudges-selection", rev1 != rev0, rev0 + " -> " + rev1);
            c->request("undo");
            settle(3);
            window.syncNow();
            realKey(uvCanvas, Qt::Key_Home);
            h.add("uv:home-frames", std::abs(uvCanvas->zoom() - 1.0) < 0.01 && uvCanvas->pan().isNull());

            // Multisseleção: caixa no vazio, arrastar move TODAS as ilhas, clique reduz, vazio limpa.
            {
                const auto islandsNow = [&] { return c->state()["uv_editor"].toObject()["islands"].toArray(); };
                const auto centerOf = [&](int index) {
                    const auto center = islandsNow()[index].toObject()["center"].toArray();
                    return QPointF(center[0].toDouble(), center[1].toDouble());
                };
                const auto mouse = [&](QEvent::Type type, const QPointF &at, Qt::MouseButton button, Qt::MouseButtons held,
                                       Qt::KeyboardModifiers mods = Qt::NoModifier) {
                    QMouseEvent event(type, at, uvCanvas->mapToGlobal(at.toPoint()), button, held, mods);
                    QCoreApplication::sendEvent(uvCanvas, &event);
                    settle(1);
                };
                const auto selectedCount = [&] {
                    int count = 0;
                    for (const auto &value : islandsNow())
                        count += value.toObject()["selected"].toBool() ? 1 : 0;
                    return count;
                };
                if (islandsNow().size() >= 2) {
                    const int total = islandsNow().size();
                    // Caixa começando fora do canvas (espaço vazio) e cobrindo tudo.
                    const QPointF from(2, 2), to(uvCanvas->width() - 2, uvCanvas->height() - 2);
                    mouse(QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
                    mouse(QEvent::MouseMove, (from + to) / 2, Qt::NoButton, Qt::LeftButton);
                    mouse(QEvent::MouseMove, to, Qt::NoButton, Qt::LeftButton);
                    mouse(QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton);
                    window.syncNow();
                    h.add("uv:box-selects-every-island", selectedCount() == total,
                          QStringLiteral("%1 de %2").arg(selectedCount()).arg(total));

                    // Arrastar a partir de uma ilha já selecionada move todas.
                    const QPointF a0 = centerOf(0), b0 = centerOf(1);
                    const QPointF press = uvCanvas->uvToWidget(a0);
                    mouse(QEvent::MouseButtonPress, press, Qt::LeftButton, Qt::LeftButton);
                    h.add("uv:pressing-a-selected-island-keeps-the-multi-selection", selectedCount() == total,
                          QString::number(selectedCount()));
                    for (int step = 1; step <= 10; ++step)
                        mouse(QEvent::MouseMove, press + QPointF(step * 2.0, 0), Qt::NoButton, Qt::LeftButton);
                    mouse(QEvent::MouseButtonRelease, press + QPointF(20, 0), Qt::LeftButton, Qt::NoButton);
                    window.syncNow();
                    const double moveA = centerOf(0).x() - a0.x(), moveB = centerOf(1).x() - b0.x();
                    h.add("uv:dragging-moves-every-selected-island",
                          moveA > 0.001 && std::abs(moveA - moveB) < 0.002,
                          QStringLiteral("A %1  B %2").arg(moveA).arg(moveB));
                    c->request("undo");
                    settle(3);
                    window.syncNow();

                    // O Undo do arrasto também desfaz a seleção: refaz a multisseleção por caixa.
                    mouse(QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
                    mouse(QEvent::MouseMove, to, Qt::NoButton, Qt::LeftButton);
                    mouse(QEvent::MouseButtonRelease, to, Qt::LeftButton, Qt::NoButton);
                    window.syncNow();

                    // Clique sem arrasto num item da multisseleção: fica só ele.
                    const int facesBefore = c->state()["uv_editor"].toObject()["uv_selected_count"].toInt();
                    const QPointF clickAt = uvCanvas->uvToWidget(centerOf(0));
                    mouse(QEvent::MouseButtonPress, clickAt, Qt::LeftButton, Qt::LeftButton);
                    mouse(QEvent::MouseButtonRelease, clickAt, Qt::LeftButton, Qt::NoButton);
                    window.syncNow();
                    // Modo Face: o clique deixa só a face clicada (uma ilha tem várias faces).
                    const int facesAfterClick = c->state()["uv_editor"].toObject()["uv_selected_count"].toInt();
                    h.add("uv:click-without-drag-narrows-to-the-clicked-item",
                          facesAfterClick == 1 && facesBefore > 1,
                          QStringLiteral("%1 faces depois, %2 antes").arg(facesAfterClick).arg(facesBefore));

                    // Clique no vazio limpa a seleção.
                    mouse(QEvent::MouseButtonPress, from, Qt::LeftButton, Qt::LeftButton);
                    mouse(QEvent::MouseButtonRelease, from, Qt::LeftButton, Qt::NoButton);
                    window.syncNow();
                    h.add("uv:click-on-empty-space-clears", selectedCount() == 0, QString::number(selectedCount()));
                } else {
                    h.add("uv:box-selects-every-island", false, "menos de duas ilhas");
                }
            }
        } else {
            h.add("uv:plus-minus-zoom", false, "sem canvas UV");
        }
        checkSurfaceGeometry(h, window, "UV");
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
        {
            const auto ids = window.properties()->visibleSectionIds();
            h.add("animate:native-property-sections",
                  ids.contains("animate.motion") && ids.contains("animate.creature") && !ids.contains("transform"),
                  ids.join(","));
        }
        // Transporte da linha do tempo: tocar/pausar pelo botão e rótulo de quadro.
        if (auto *page = window.timelinePage()) {
            window.drawerRegion()->setOpen(true);
            settle(4);
            h.add("timeline:transport-visible-in-drawer", page->playButton()->isVisibleTo(&window));
            // Tocar exige um Motion: usa o primeiro do catálogo publicado.
            const auto catalog = c->state()["animate"].toObject()["catalog"].toArray();
            if (!c->flag("animate.has_motion") && !catalog.isEmpty()) {
                c->request("animate", {{"id", "add-motion"}, {"arg", catalog.first().toObject()["id"]}});
                settle(4);
                window.syncNow();
            }
            const bool before = c->flag("animate.playing");
            page->playButton()->click();
            settle(4);
            window.syncNow();
            const bool toggled = c->flag("animate.playing") != before;
            page->playButton()->click();
            settle(4);
            window.syncNow();
            h.add("timeline:play-button-toggles-playback", toggled && c->flag("animate.playing") == before);
            h.add("timeline:key-button-explains-when-disabled",
                  page->keyButton()->isEnabled() || !page->keyButton()->toolTip().isEmpty());
            window.drawerRegion()->setOpen(false);
            settle(2);
        } else {
            h.add("timeline:transport-visible-in-drawer", false, "sem página");
        }
        checkSurfaceGeometry(h, window, "ANIMATE");
        h.shot(&window, "shell_ANIMATE_v2");

        // Retorna a POLY para manter a baseline das verificações subsequentes
        c->request("workspace", {{"id", "POLY"}});
        settle(8);
        window.syncNow();
        settle(4);
    }

    // U07: menu de contexto da vista por teclado (Shift+F10) com rótulos traduzidos.
    {
        QStringList texts;
        bool opened = false;
        QTimer::singleShot(120, [&] {
            if (auto *menu = qobject_cast<QMenu *>(QApplication::activePopupWidget())) {
                opened = true;
                for (auto *action : menu->actions())
                    if (!action->isSeparator())
                        texts << action->text();
                menu->close();
            }
        });
        window.viewport()->setFocus();
        settle(2);
        QKeyEvent shiftF10(QEvent::KeyPress, Qt::Key_F10, Qt::ShiftModifier);
        QCoreApplication::sendEvent(QApplication::focusWidget() ? QApplication::focusWidget() : window.viewport(),
                                    &shiftF10);
        settle(4);
        h.add("viewport:shift-f10-opens-context-menu", opened && !texts.isEmpty(), texts.join(" | "));
        // Fechar o menu devolve o foco ao invocador (U07); a janela segue ativa.
        window.activateWindow();
        settle(4);
        h.add("viewport:context-menu-returns-focus", QApplication::focusWidget() == window.viewport(),
              QApplication::focusWidget() ? QString::fromLatin1(QApplication::focusWidget()->metaObject()->className())
                                          : QStringLiteral("sem foco"));
        bool rawIds = false;
        for (const auto &text : texts) {
            const QString label = text.section(QLatin1Char('\t'), 0, 0);
            rawIds |= label.contains(QLatin1Char('.')) || label.contains(QLatin1Char('_'));
        }
        h.add("viewport:context-menu-has-no-raw-ids", !texts.isEmpty() && !rawIds, texts.join(" | "));
    }

    // O05: materiais como nós — receita criada, ligada, editada e desfeita pela GUI.
    {
        c->request("workspace", {{"id", "POLY"}});
        c->request("create_material");
        settle(4);
        window.syncNow();
        window.openNodeEditor(true);
        settle(6);
        window.syncNow();
        auto *page = window.nodePage();
        auto *canvas = page ? page->canvas() : nullptr;
        h.add("material:page-opens-in-material-domain",
              canvas && canvas->domain() == PetuniaNodeCanvas::Domain::Material && window.drawerRegion()->isOpen());
        if (page && canvas) {
            h.add("material:create-offered-without-recipe", page->createRecipeButton()->isVisibleTo(page) &&
                                                                page->createRecipeButton()->isEnabled());
            page->createRecipeButton()->click();
            settle(4);
            window.syncNow();
            h.add("material:recipe-created", c->flag("material_recipe.has_recipe"));
            h.add("material:canvas-shows-image-to-output", canvas->nodeCount() == 2 && canvas->wireCount() == 1,
                  QStringLiteral("%1 nós, %2 fios").arg(canvas->nodeCount()).arg(canvas->wireCount()));
            c->request("material_recipe_add", {{"kind", "checker"}});
            settle(2);
            window.syncNow();
            QString checker, output;
            for (const auto &value : c->state()["material_recipe"].toObject()["nodes"].toArray()) {
                const auto node = value.toObject();
                if (node["kind"].toString() == "checker")
                    checker = node["id"].toString();
                if (node["kind"].toString() == "output")
                    output = node["id"].toString();
            }
            canvas->connectSockets(checker, 0, output, 0);
            settle(2);
            window.syncNow();
            int intoOutput = 0;
            bool fromChecker = false;
            for (const auto &value : c->state()["material_recipe"].toObject()["edges"].toArray()) {
                const auto edge = value.toObject();
                if (edge["to"].toString() == output) {
                    ++intoOutput;
                    fromChecker = edge["from"].toString() == checker;
                }
            }
            h.add("material:new-wire-replaces-input", intoOutput == 1 && fromChecker);
            canvas->selectNode(checker);
            settle(2);
            auto *card = page->parameterCard();
            {
                // Nós de receita não têm ferramenta: nenhuma legenda "Ferramenta:" visível.
                const QString prefix = petuniaTr(QStringLiteral("Ferramenta: %1")).section(QLatin1Char('%'), 0, 0).trimmed();
                bool toolSubtitle = false;
                if (card) {
                    const auto labels = card->findChildren<QLabel *>();
                    for (auto *label : labels)
                        toolSubtitle |= !label->isHidden() && label->text().startsWith(prefix);
                }
                h.add("material:panel-has-no-tool-subtitle", card && !toolSubtitle);
            }
            h.add("material:panel-generates-checker-controls",
                  card && card->findChildren<QToolButton *>().size() >= 2 &&
                      !card->findChildren<PetuniaNumberField *>().isEmpty());
            c->request("material_recipe_set", {{"node", checker}, {"key", "cell_size"}, {"value", 8}});
            settle(2);
            window.syncNow();
            int cell = 0;
            for (const auto &value : c->state()["material_recipe"].toObject()["nodes"].toArray())
                if (value.toObject()["id"].toString() == checker)
                    cell = value.toObject()["params"].toObject()["cell_size"].toInt();
            h.add("material:param-edit-roundtrips", cell == 8, QString::number(cell));
            auto *iface = QAccessible::queryAccessibleInterface(canvas);
            h.add("material:a11y-lists-recipe-nodes", iface && iface->childCount() == canvas->nodeCount());
            // Posição do nó vira dado do documento (Ctrl+→ move e persiste).
            {
                const QPointF before = canvas->nodeScenePosition(checker);
                canvas->setFocus();
                QKeyEvent right(QEvent::KeyPress, Qt::Key_Right, Qt::ControlModifier);
                QCoreApplication::sendEvent(canvas, &right);
                settle(2);
                window.syncNow();
                double stored = -1e9;
                for (const auto &value : c->state()["material_recipe"].toObject()["nodes"].toArray())
                    if (value.toObject()["id"].toString() == checker)
                        stored = value.toObject()["position"].toArray().at(0).toDouble();
                h.add("material:node-position-persists", std::abs(stored - (before.x() + 16)) < 0.5,
                      QStringLiteral("%1 vs %2").arg(stored).arg(before.x() + 16));
                c->request("undo");
                settle(2);
                window.syncNow();
            }
            canvas->fitAll();
            h.shot(&window, "shell_POLY_material_nodes");
            // Paridade visual: o mapa gerado pela receita chega ao viewport (Material preview).
            {
                const auto items = c->state()["scene_items"].toArray();
                if (!items.isEmpty())
                    c->request("select", {{"id", items.first().toObject()["id"].toString()}});
                c->request("material_assign");
                c->request("set_shading_mode", {{"id", "material"}});
                settle(6);
                window.syncNow();
                settle(4);
                h.shot(&window, "shell_POLY_material_preview");
                c->request("set_shading_mode", {{"id", "solid"}});
                c->request("undo"); // atribuição
                settle(2);
            }
            c->request("undo");
            c->request("undo");
            settle(2);
            window.syncNow();
            bool imageBack = false;
            for (const auto &value : c->state()["material_recipe"].toObject()["edges"].toArray())
                imageBack |= value.toObject()["to"].toString() == output && value.toObject()["from"].toString() != checker;
            h.add("material:undo-restores-previous-wiring", imageBack);
            page->bakeButton()->click();
            settle(2);
            window.syncNow();
            h.add("material:bake-removes-recipe", !c->flag("material_recipe.has_recipe"));
            page->setDomain(PetuniaNodeCanvas::Domain::Operations);
        }
        window.drawerRegion()->setOpen(false);
        settle(4);
    }

    // U05: um gesto de pintura interrompido por perda de foco não fica aberto;
    // o traço é confirmado (um passo de Undo) e o próximo começa limpo.
    {
        c->request("workspace", {{"id", "PAINT"}});
        settle(8);
        window.syncNow();
        c->request("tool", {{"id", "brush"}});
        settle(2);
        auto press = [](QWidget *w, QPointF at, QEvent::Type type, Qt::MouseButton button, Qt::MouseButtons buttons) {
            QMouseEvent event(type, at, w->mapToGlobal(at), button, buttons, Qt::NoModifier);
            QCoreApplication::sendEvent(w, &event);
        };
        auto interrupted = [&](QWidget *target, const QString &name) {
            if (!target || !target->isVisible()) {
                h.add("paint:" + name + "-available", false);
                return;
            }
            target->setFocus(Qt::MouseFocusReason);
            settle(2);
            const QPointF center(target->width() / 2.0, target->height() / 2.0);
            press(target, center, QEvent::MouseButtonPress, Qt::LeftButton, Qt::LeftButton);
            press(target, center + QPointF(12, 6), QEvent::MouseMove, Qt::NoButton, Qt::LeftButton);
            settle(2);
            window.syncNow();
            const bool opened = c->flag("paint_stroke_open");
            // Solta o botão direito no meio: o traço do esquerdo continua.
            press(target, center + QPointF(14, 6), QEvent::MouseButtonRelease, Qt::RightButton, Qt::LeftButton);
            window.syncNow();
            const bool survivesOtherButton = c->flag("paint_stroke_open") == opened;
            window.appBar()->searchButton()->setFocus(Qt::ActiveWindowFocusReason);
            settle(4);
            window.syncNow();
            h.add("paint:" + name + "-stroke-opens", opened);
            h.add("paint:" + name + "-other-button-release-keeps-stroke", survivesOtherButton);
            h.add("paint:" + name + "-focus-loss-closes-stroke", !c->flag("paint_stroke_open"));
            // O botão esquerdo ainda "solto" depois do foco: nenhum efeito residual.
            press(target, center, QEvent::MouseButtonRelease, Qt::LeftButton, Qt::NoButton);
            settle(2);
            window.syncNow();
            h.add("paint:" + name + "-late-release-is-harmless", !c->flag("paint_stroke_open"));
        };
        interrupted(window.workspaceCanvas("PAINT"), "canvas2d");
        interrupted(window.viewport(), "viewport3d");
        // Páginas da gaveta abertas (Assets, Nós nos dois domínios, Linha do tempo).
        auto auditOpenDrawer = [&](const QString &tag) {
            settle(6);
            window.syncNow();
            settle(4);
            QStringList unnamed;
            const auto pageWidgets = window.drawerRegion()->findChildren<QWidget *>();
            for (auto *w : pageWidgets) {
                if (!w->isVisible() || !w->isEnabled() || !(w->focusPolicy() & Qt::TabFocus))
                    continue;
                auto *iface = QAccessible::queryAccessibleInterface(w);
                if (!iface || iface->text(QAccessible::Name).trimmed().isEmpty())
                    unnamed << QString::fromLatin1(w->metaObject()->className()) + QStringLiteral(" < ") +
                                   QString::fromLatin1(w->parentWidget() ? w->parentWidget()->metaObject()->className() : "");
            }
            unnamed.removeDuplicates();
            h.add("a11y:drawer-" + tag + ":controls-have-names", unnamed.isEmpty(),
                  QStringLiteral("%1: %2").arg(unnamed.size()).arg(unnamed.mid(0, 12).join(QStringLiteral(" | "))));
        };
        c->request("workspace", {{"id", "POLY"}});
        settle(6);
        window.syncNow();
        window.drawerRegion()->setCurrentPage("assets");
        window.drawerRegion()->setOpen(true);
        auditOpenDrawer("assets");
        window.openNodeEditor(false);
        auditOpenDrawer("nodes-operations");
        window.openNodeEditor(true);
        auditOpenDrawer("nodes-material");
        window.nodePage()->setDomain(PetuniaNodeCanvas::Domain::Operations);
        window.drawerRegion()->setOpen(false);
        c->request("workspace", {{"id", "ANIMATE"}});
        settle(6);
        window.syncNow();
        window.drawerRegion()->setCurrentPage("timeline");
        window.drawerRegion()->setOpen(true);
        auditOpenDrawer("timeline");
        window.drawerRegion()->setOpen(false);
        c->request("workspace", {{"id", "POLY"}});
        settle(6);
        window.syncNow();
    }

    // Camada 6 (a11y): todo controle visível alcançável por teclado tem nome acessível.
    {
        for (const QString &ws : {QStringLiteral("DRAW"), QStringLiteral("POLY"), QStringLiteral("PAINT"),
                                  QStringLiteral("UV"), QStringLiteral("ANIMATE")}) {
            c->request("workspace", {{"id", ws}});
            settle(6);
            window.syncNow();
            settle(4);
            QStringList offenders;
            int audited = 0;
            const auto widgets = window.findChildren<QWidget *>();
            for (auto *w : widgets) {
                if (!w->isVisible() || !w->isEnabled() || !(w->focusPolicy() & Qt::TabFocus))
                    continue;
                ++audited;
                auto *iface = QAccessible::queryAccessibleInterface(w);
                const QString name = iface ? iface->text(QAccessible::Name).trimmed() : QString();
                if (name.isEmpty()) {
                    QString path = QString::fromLatin1(w->metaObject()->className());
                    for (auto *p = w->parentWidget(); p && path.size() < 160; p = p->parentWidget())
                        path += QStringLiteral(" < ") + QString::fromLatin1(p->metaObject()->className());
                    offenders << path;
                }
            }
            offenders.removeDuplicates();
            // Botão desabilitado diz por quê (dica ou descrição); controle de valor expõe o valor.
            QStringList noReason, noValue;
            for (auto *w : widgets) {
                if (!w->isVisible())
                    continue;
                auto *iface = QAccessible::queryAccessibleInterface(w);
                if (!iface)
                    continue;
                if (!w->isEnabled() && qobject_cast<QAbstractButton *>(w) && w->toolTip().trimmed().isEmpty() &&
                    iface->text(QAccessible::Description).trimmed().isEmpty())
                    noReason << iface->text(QAccessible::Name) + QStringLiteral(" (") +
                                    QString::fromLatin1(w->metaObject()->className()) + QLatin1Char(')');
                const auto role = iface->role();
                if (w->isEnabled() && (role == QAccessible::SpinBox || role == QAccessible::Slider ||
                                       role == QAccessible::ComboBox) &&
                    iface->text(QAccessible::Value).trimmed().isEmpty() && !iface->valueInterface())
                    noValue << iface->text(QAccessible::Name) + QStringLiteral(" (") +
                                   QString::fromLatin1(w->metaObject()->className()) + QLatin1Char(')');
            }
            noReason.removeDuplicates();
            noValue.removeDuplicates();
            h.add("a11y:" + ws + ":disabled-buttons-explain-why", noReason.isEmpty(),
                  QStringLiteral("%1: %2").arg(noReason.size()).arg(noReason.mid(0, 12).join(QStringLiteral(" | "))));
            h.add("a11y:" + ws + ":value-controls-expose-value", noValue.isEmpty(),
                  QStringLiteral("%1: %2").arg(noValue.size()).arg(noValue.mid(0, 12).join(QStringLiteral(" | "))));
            h.add("a11y:" + ws + ":focusable-controls-have-names", offenders.isEmpty(),
                  QStringLiteral("%1 sem nome de %2: %3").arg(offenders.size()).arg(audited)
                      .arg(offenders.mid(0, 12).join(QStringLiteral(" | "))));
        }
        c->request("workspace", {{"id", "POLY"}});
        settle(6);
        window.syncNow();
    }

    // U07: diálogos recebem o foco, Esc fecha só o topo e o foco volta ao invocador.
    {
        c->request("workspace", {{"id", "POLY"}});
        settle(4);
        window.syncNow();
        QLineEdit *invoker = nullptr;
        const auto edits = window.structure()->findChildren<QLineEdit *>();
        for (auto *edit : edits)
            if (edit->isVisible() && !invoker)
                invoker = edit;
        struct Probe { bool opened{false}; bool focusInside{false}; };
        auto probeModal = [](Probe *probe) {
            QTimer::singleShot(150, [probe] {
                QWidget *top = QApplication::activeModalWidget();
                if (!top)
                    top = QApplication::activePopupWidget();
                probe->opened = top != nullptr;
                QWidget *focus = QApplication::focusWidget();
                probe->focusInside = top && focus && (focus == top || top->isAncestorOf(focus));
                QWidget *target = focus ? focus : top;
                if (target) {
                    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
                    QCoreApplication::sendEvent(target, &esc);
                }
                // Rede de segurança: nunca deixar o harness preso num diálogo.
                QTimer::singleShot(400, [] {
                    if (auto *modal = QApplication::activeModalWidget())
                        modal->close();
                });
            });
        };
        auto check = [&](const QString &name, const std::function<void()> &open, bool modal) {
            if (!invoker) {
                h.add("focus:" + name + "-invoker-available", false);
                return;
            }
            invoker->setFocus(Qt::OtherFocusReason);
            settle(2);
            Probe probe;
            if (modal) {
                probeModal(&probe);
                open();
            } else {
                open();
                settle(4);
                QWidget *top = QApplication::activePopupWidget();
                if (!top)
                    top = QApplication::activeWindow() != &window ? QApplication::activeWindow() : nullptr;
                probe.opened = top != nullptr;
                QWidget *focus = QApplication::focusWidget();
                probe.focusInside = top && focus && (focus == top || top->isAncestorOf(focus));
                if (focus) {
                    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
                    QCoreApplication::sendEvent(focus, &esc);
                }
            }
            settle(4);
            h.add("focus:" + name + "-opens-with-focus-inside", probe.opened && probe.focusInside);
            h.add("focus:" + name + "-escape-returns-focus-to-invoker", QApplication::focusWidget() == invoker,
                  QApplication::focusWidget() ? QString::fromLatin1(QApplication::focusWidget()->metaObject()->className())
                                              : QStringLiteral("sem foco"));
        };
        check("palette", [&] { window.openPalette(); }, false);
        check("preferences", [&] { window.openPreferences(); }, true);
        check("references", [&] { window.openReferences(); }, true);
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

    checkRefinement(h, window);

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

    checkWaveB(h, window, c);
    checkWaveBClosing(h, window, c);

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
