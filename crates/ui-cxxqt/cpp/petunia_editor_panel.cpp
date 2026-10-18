#include "petunia_editor_panel.h"
#include "petunia_color_picker.h"
#include "petunia_number_field.h"
#include "petunia_vector3_field.h"
#include "petunia_theme.h"
#include "petunia_icons.h"
#include <QApplication>
#include <QCheckBox>
#include <QCloseEvent>
#include <QColorDialog>
#include <QComboBox>
#include <QDialog>
#include <QDockWidget>
#include <QDoubleSpinBox>
#include <QFileDialog>
#include <QFileInfo>
#include <QInputDialog>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLabel>
#include <QLineEdit>
#include <QMainWindow>
#include <QMenu>
#include <QMouseEvent>
#include <QPainter>
#include <QPointer>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollArea>
#include <QToolButton>
#include <QSignalBlocker>
#include <QSplitter>
#include <QTabBar>
#include <QTabletEvent>
#include <QTimer>
#include <QWheelEvent>
#include <cmath>

QPainterPath petuniaPath(const QString &commands) {
    QPainterPath path;
    static const QRegularExpression tokens(
        QStringLiteral("[MLCQSTZHVAmlcqstzhva]|[-+]?(?:[0-9]*\\.)?[0-9]+(?:[eE]"
                       "[-+]?[0-9]+)?"));
    QStringList values;
    auto it = tokens.globalMatch(commands);
    while (it.hasNext())
        values.append(it.next().captured());
    int i = 0;
    QChar command, previous;
    QPointF cubicControl, quadraticControl;
    auto numeric = [&](int count) {
        if (i + count > values.size())
            return false;
        for (int j = 0; j < count; ++j) {
            bool ok = false;
            const double value = values[i + j].toDouble(&ok);
            if (!ok || !std::isfinite(value))
                return false;
        }
        return true;
    };
    auto num = [&] { return values[i++].toDouble(); };
    constexpr double pi = 3.14159265358979323846;
    auto arc = [&](double rx, double ry, double degrees, bool large, bool sweep,
                   QPointF end) {
        const QPointF start = path.currentPosition();
        rx = std::abs(rx);
        ry = std::abs(ry);
        if (start == end)
            return;
        if (rx < 1e-9 || ry < 1e-9) {
            path.lineTo(end);
            return;
        }
        const double phi = degrees * pi / 180., c = std::cos(phi),
                     s = std::sin(phi);
        const QPointF half = (start - end) / 2.;
        const double x = c * half.x() + s * half.y(),
                     y = -s * half.x() + c * half.y();
        const double stretch = x * x / (rx * rx) + y * y / (ry * ry);
        if (stretch > 1.) {
            rx *= std::sqrt(stretch);
            ry *= std::sqrt(stretch);
        }
        const double denominator = rx * rx * y * y + ry * ry * x * x;
        const double factor =
            (large == sweep ? -1. : 1.) *
            std::sqrt(qMax(0., (rx * rx * ry * ry - denominator) /
                                   qMax(denominator, 1e-20)));
        const double cx = factor * rx * y / ry, cy = -factor * ry * x / rx;
        const QPointF center =
            (start + end) / 2. + QPointF(c * cx - s * cy, s * cx + c * cy);
        const double ux = (x - cx) / rx, uy = (y - cy) / ry,
                     vx = (-x - cx) / rx, vy = (-y - cy) / ry;
        double angle = std::atan2(uy, ux),
               delta = std::atan2(ux * vy - uy * vx, ux * vx + uy * vy);
        if (!sweep && delta > 0.)
            delta -= 2. * pi;
        if (sweep && delta < 0.)
            delta += 2. * pi;
        const int pieces = qMax(1, int(std::ceil(std::abs(delta) / (pi / 2.))));
        const double step = delta / pieces;
        auto point = [&](double t) {
            return center +
                   QPointF(c * rx * std::cos(t) - s * ry * std::sin(t),
                           s * rx * std::cos(t) + c * ry * std::sin(t));
        };
        auto tangent = [&](double t) {
            return QPointF(-c * rx * std::sin(t) - s * ry * std::cos(t),
                           -s * rx * std::sin(t) + c * ry * std::cos(t));
        };
        for (int part = 0; part < pieces; ++part) {
            const double next = angle + step, k = 4. / 3. * std::tan(step / 4.);
            path.cubicTo(point(angle) + k * tangent(angle),
                         point(next) - k * tangent(next),
                         part + 1 == pieces ? end : point(next));
            angle = next;
        }
    };
    while (i < values.size()) {
        if (values[i].size() == 1 && values[i][0].isLetter())
            command = values[i++][0];
        if (command.isNull())
            break;
        const QChar op = command.toUpper();
        const bool relative = command.isLower();
        const QPointF base = relative ? path.currentPosition() : QPointF();
        auto point = [&] {
            const double x = num(), y = num();
            return base + QPointF(x, y);
        };
        if (op == 'Z') {
            path.closeSubpath();
            previous = op;
            command = QChar();
            continue;
        }
        const int count = op == 'H' || op == 'V'                ? 1
                          : op == 'M' || op == 'L' || op == 'T' ? 2
                          : op == 'Q' || op == 'S'              ? 4
                          : op == 'C'                           ? 6
                          : op == 'A'                           ? 7
                                                                : 0;
        if (!count || !numeric(count))
            break;
        if (op == 'M') {
            path.moveTo(point());
            command = relative ? QChar('l') : QChar('L');
        } else if (op == 'L')
            path.lineTo(point());
        else if (op == 'H')
            path.lineTo(num() + base.x(), path.currentPosition().y());
        else if (op == 'V')
            path.lineTo(path.currentPosition().x(), num() + base.y());
        else if (op == 'C' || op == 'S') {
            const QPointF first =
                op == 'C' ? point()
                          : (previous == 'C' || previous == 'S'
                                 ? 2. * path.currentPosition() - cubicControl
                                 : path.currentPosition());
            const QPointF second = point(), end = point();
            path.cubicTo(first, second, end);
            cubicControl = second;
        } else if (op == 'Q' || op == 'T') {
            const QPointF control =
                op == 'Q'
                    ? point()
                    : (previous == 'Q' || previous == 'T'
                           ? 2. * path.currentPosition() - quadraticControl
                           : path.currentPosition());
            path.quadTo(control, point());
            quadraticControl = control;
        } else if (op == 'A') {
            const double rx = num(), ry = num(), rotation = num();
            const bool large = num() != 0., sweep = num() != 0.;
            arc(rx, ry, rotation, large, sweep, point());
        }
        previous = op;
    }
    return path;
}
QImage petuniaImage(const QString &kind, const QString &id) {
    const auto k = kind.toUtf8(), i = id.toUtf8();
    uint32_t w = 0, h = 0;
    const auto size = petunia_qt_editor_image(k.constData(), i.constData(),
                                              nullptr, 0, &w, &h);
    if (!size || size > 256 * 1024 * 1024 || !w || !h ||
        size != size_t(w) * h * 4)
        return {};
    QByteArray bytes;
    bytes.resize(size);
    if (petunia_qt_editor_image(k.constData(), i.constData(),
                                reinterpret_cast<uint8_t *>(bytes.data()), size,
                                &w, &h) != size)
        return {};
    return QImage(reinterpret_cast<const uchar *>(bytes.constData()), w, h,
                  QImage::Format_RGBA8888)
        .copy();
}
PetuniaEditorPanel::PetuniaEditorPanel(const QString &mode, QWidget *parent)
    : PetuniaBoundForm(parent), m_mode(mode) {
    setAccessibleName(mode + QStringLiteral(" · controles do editor"));
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(4, 4, 4, 4);
    m_tabs = new QTabWidget(this);
    m_tabs->setDocumentMode(true);
    m_tabs->setUsesScrollButtons(true);
    layout->addWidget(m_tabs);
    // Os workspaces usam seções nativas do Shell v2; aqui restam os diálogos do host.
    if (mode == "SETTINGS")
        setupSettings();
    else if (mode == "REFERENCES")
        setupReferences();
    else if (mode == "ASSETS")
        setupAssets();
    sync(petuniaRequest("snapshot"));
    petuniaNameTabContainers(m_tabs, tr("Painel %1").arg(mode));
}

void petuniaNameTabContainers(QTabWidget *tabs, const QString &name) {
    if (!tabs)
        return;
    tabs->setAccessibleName(name);
    auto apply = [tabs] {
        for (int i = 0; i < tabs->count(); ++i) {
            QList<QScrollArea *> areas = tabs->widget(i)->findChildren<QScrollArea *>();
            if (auto *self = qobject_cast<QScrollArea *>(tabs->widget(i)))
                areas.prepend(self);
            for (auto *area : areas)
                if (area->accessibleName().isEmpty())
                    area->setAccessibleName(tabs->tabText(i).remove(QLatin1Char('&')));
        }
        // Setas de rolagem da barra (Qt): nome e dica, inclusive desabilitadas.
        const auto arrows = tabs->tabBar()->findChildren<QToolButton *>();
        for (auto *arrow : arrows) {
            const bool left = arrow->arrowType() == Qt::LeftArrow;
            const QString text = left ? QCoreApplication::translate("PetuniaShell", "Rolar abas para a esquerda")
                                      : QCoreApplication::translate("PetuniaShell", "Rolar abas para a direita");
            arrow->setAccessibleName(text);
            arrow->setToolTip(text);
        }
    };
    apply();
    QObject::connect(tabs, &QTabWidget::currentChanged, tabs, apply);
}
QFormLayout *PetuniaEditorPanel::page(const QString &title) {
    auto *scroll = new QScrollArea(m_tabs);
    scroll->setWidgetResizable(true);
    auto *content = new QWidget(scroll);
    auto *form = new QFormLayout(content);
    form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
    form->setRowWrapPolicy(QFormLayout::WrapLongRows);
    scroll->setWidget(content);

    m_tabs->setElideMode(Qt::ElideNone);
    m_tabs->setUsesScrollButtons(false);
    if (m_tabs->tabBar()) {
        m_tabs->tabBar()->setExpanding(false);
        m_tabs->tabBar()->setUsesScrollButtons(false);
        m_tabs->tabBar()->setElideMode(Qt::ElideNone);
    }

    const QHash<QString, QString> iconMap{
        {tr("Transf"), "tool_transform"},
        {tr("Transformação"), "tool_transform"},
        {tr("Material"), "material"},
        {tr("Modif"), "modifier"},
        {tr("Modificadores"), "modifier"},
        {tr("Ações"), "bolt"},
        {tr("Ações rápidas"), "bolt"},
        {tr("Ferram"), "tune"},
        {tr("Ferramenta"), "tune"},
        {tr("Perfil"), "tool_profile"},
        {tr("Operações DRAW"), "draw_profile"},
        {tr("Booleans"), "boolean_join"},
        {tr("Geral"), "settings"},
        {tr("Acessibilidade"), "accessibility"},
        {tr("Viewport"), "camera"},
        {tr("Atalhos"), "keyboard"},
        {tr("Criatura"), "hierarchy"},
        {tr("Motion"), "play"},
        {tr("Pose"), "bone"},
        {tr("Timeline"), "timeline"},
        {tr("UV"), "uv_editor"},
        {tr("Transformar UV"), "uv_move"}
    };

    QIcon tabIcon;
    if (iconMap.contains(title)) {
        tabIcon = PetuniaIcons::get(iconMap[title]);
    }

    m_tabs->setIconSize(QSize(18, 18));
    QString tabText = (m_mode == "MODEL" && !tabIcon.isNull()) ? QString() : title;
    int idx = m_tabs->addTab(scroll, tabIcon, tabText);
    m_tabs->setTabToolTip(idx, title);
    scroll->setProperty("nativeSourceTab", tabText);

    if (m_mode == "MODEL") {
        const QHash<QString, QString> sections{
            {tr("Transf"), "transform"},
            {tr("Transformação"), "transform"},
            {tr("Material"), "material"},
            {tr("Modif"), "modifiers"},
            {tr("Modificadores"), "modifiers"},
            {tr("Ações"), "quick-actions"},
            {tr("Ações rápidas"), "quick-actions"},
            {tr("Ferram"), "tools"},
            {tr("Ferramenta"), "tools"}};
        scroll->setProperty("sectionId", sections.value(title));
    }
    return form;
}
void PetuniaEditorPanel::setupSettings() {
    auto *general = page(tr("Geral"));
    rows(general, tr("Idioma"), "available_languages", "set_language", "lang");
    rows(general, tr("Temas"), "themes", "theme");
    number(general, tr("Escala da interface"), "ui_scale", "set_ui_scale",
           "scale", 0.9, 2, 0.1);
    choice(general, tr("Ícones"), "icon_theme", "set_icon_theme", "theme",
           {"outline", "filled"}, {tr("Contorno"), tr("Preenchidos")});
    colorField(general, tr("Cor de destaque (#RRGGBB)"), "accent_color_hex",
               "set_accent_color_hex", "value");
    check(general, tr("Mostrar Home ao abrir"), "show_home_on_start",
          "set_show_home_on_start", "enabled");
    check(general, tr("Mostrar nomes de ferramentas"), "show_tool_labels",
          "set_show_tool_labels", "enabled");
    auto *access = page(tr("Acessibilidade"));
    check(access, tr("Reduzir movimento"), "reduced_motion",
          "set_reduced_motion", "enabled");
    check(access, tr("Eixos distinguíveis sem cor"), "colorblind_axes",
          "set_colorblind_axes", "enabled");
    check(access, tr("Clicar, mover, clicar"), "click_move_click",
          "set_click_move_click", "enabled");
    number(access, tr("Limiar de arrasto (px)"), "drag_threshold_px",
           "set_drag_threshold_px", "pixels", 2, 16, 1);
    number(access, tr("Raio de snap (px)"), "snap_radius_px",
           "set_snap_radius_px", "pixels", 4, 48, 1);
    number(access, tr("Duplo toque (ms)"), "double_tap_interval_ms",
           "set_double_tap_interval_ms", "interval_ms", 0, 1000, 10);
    colorField(access, tr("Cor da seleção (#RRGGBB)"), "selection_color_hex",
               "set_selection_color_hex", "value");
    number(access, tr("Espessura da seleção"), "selection_thickness",
           "set_selection_thickness", "thickness", 1, 6, 0.25);
    check(access, tr("Mostrar média de medidas"), "multiselection_measure_tag",
          "set_multiselection_measure_tag", "enabled");
    auto *viewport = page(tr("Viewport"));
    number(viewport, tr("Sensibilidade do Zoom"), "zoom_sensitivity",
           "set_zoom_sensitivity", "sensitivity", 0.2, 5.0, 0.1);
    check(viewport, tr("Inverter arrasto vertical"), "invert_vertical_drag",
          "set_invert_vertical_drag", "invert");
    check(viewport, tr("Luz acompanha câmera"), "studio_light_follows_camera",
          "set_studio_light_follows_camera", "follows");
    check(viewport, tr("Matcap"), "viewport_matcap", "set_viewport_matcap",
          "enabled");
    check(viewport, tr("Ambient occlusion"), "viewport_ambient_occlusion",
          "set_viewport_ambient_occlusion", "enabled");
    check(viewport, tr("Preferir chão no plano automático"),
          "workplane_prefer_ground", "set_workplane_prefer_ground", "enabled");
    choice(viewport, tr("Ativação de ferramentas"), "tool_activation",
           "set_tool_activation", "id", {"persistent", "instant"});
    auto *keymap = page(tr("Atalhos"));
    rows(keymap, tr("Perfis"), "keymap_profiles", "activate_keymap_profile");
    text(keymap, tr("Novo perfil"), {}, "create_keymap_profile", "name");
    auto *profiles = m_rows.last().widget;
    profiles->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(profiles, &QListWidget::customContextMenuRequested, this,
            [this, profiles](const QPoint &pos) {
                auto *item = profiles->itemAt(pos);
                if (!item)
                    return;
                const auto id = item->data(Qt::UserRole).toString();
                bool custom = false;
                for (const auto &v : m_state["keymap_profiles"].toArray())
                    if (v.toObject()["id"].toString() == id)
                        custom = v.toObject()["custom"].toBool();
                QMenu menu(this);
                auto *remove = menu.addAction(tr("Excluir perfil"));
                remove->setEnabled(custom);
                if (menu.exec(profiles->viewport()->mapToGlobal(pos)) == remove)
                    send("delete_keymap_profile", {{"id", id}});
            });
    rows(keymap, tr("Ações · duplo clique para editar"), "keymap_actions",
         "keymap_select");
    auto *actions = m_rows.last().widget;
    connect(
        actions, &QListWidget::itemDoubleClicked, this,
        [this](QListWidgetItem *item) {
            const auto action = item->data(Qt::UserRole).toString();
            send("begin_keymap_capture", {{"target", action}});
            class CaptureDialog : public QDialog {
              public:
                explicit CaptureDialog(QWidget *parent) : QDialog(parent) {
                    setWindowTitle(tr("Pressione o atalho"));
                    auto *l = new QVBoxLayout(this);
                    l->addWidget(new QLabel(
                        tr("Pressione a combinação desejada. Escape cancela."),
                        this));
                    setMinimumWidth(320);
                }

              protected:
                void keyPressEvent(QKeyEvent *e) override {
                    const auto text = petuniaKeyText(e);
                    auto result = petuniaRequest(
                        "capture_keymap_key",
                        {{"text", text},
                         {"ctrl", bool(e->modifiers() & Qt::ControlModifier)},
                         {"shift", bool(e->modifiers() & Qt::ShiftModifier)},
                         {"alt", bool(e->modifiers() & Qt::AltModifier)}});
                    if (e->key() == Qt::Key_Escape)
                        petuniaRequest("cancel_keymap_capture");
                    if (result["changed"].toBool() ||
                        e->key() == Qt::Key_Escape)
                        accept();
                }
            };
            CaptureDialog dlg(this);
            dlg.exec();
            sync(petuniaRequest("snapshot"));
            emit editorChanged();
        });
    actions->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(actions, &QListWidget::customContextMenuRequested, this,
            [this, actions](const QPoint &pos) {
                auto *item = actions->itemAt(pos);
                if (!item)
                    return;
                QMenu menu(this);
                auto *clear = menu.addAction(tr("Limpar atalho"));
                menu.addAction(tr("Restaurar atalho"));
                auto *picked = menu.exec(actions->viewport()->mapToGlobal(pos));
                if (picked)
                    send(picked == clear ? "clear_keymap_binding"
                                         : "reset_keymap_binding",
                         {{"target", item->data(Qt::UserRole).toString()}});
            });
}

void PetuniaEditorPanel::setupReferences() {
    for (const auto &axis :
         QStringList{"front", "back", "left", "right", "top", "bottom"}) {
        auto *form = page(axis);
        auto *add = new QPushButton(tr("Importar/substituir imagem"), this);
        form->addRow(add);
        connect(add, &QPushButton::clicked, this, [this, axis] {
            const auto path = QFileDialog::getOpenFileName(
                this, tr("Imagem de referência"), {},
                tr("Imagens (*.png *.jpg *.jpeg *.webp *.bmp)"));
            if (path.isEmpty())
                return;
            QImage image(path);
            if (image.isNull())
                return;
            if (image.width() > 4096 || image.height() > 4096)
                image = image.scaled(4096, 4096, Qt::KeepAspectRatio,
                                     Qt::SmoothTransformation);
            image = image.convertToFormat(QImage::Format_RGBA8888);
            extern bool loadPetuniaReference(const QString &, const QString &,
                                             const QImage &);
            loadPetuniaReference(axis, QFileInfo(path).fileName(), image);
            sync(petuniaRequest("snapshot"));
            emit editorChanged();
        });
        auto *preview = new QLabel(this);
        preview->setAccessibleName(tr("Prévia da referência"));
        preview->setProperty("referenceAxis", axis);
        preview->setMinimumHeight(80);
        form->addRow(preview);
        m_bindings.append({preview, {}, {}, {}});
        const auto prefix = "reference_" + axis + ".";
        check(form, tr("Visível"), prefix + "visible",
              "toggle_reference_visible", {}, {{"axis_str", axis}});
        check(form, tr("Bloqueada"), prefix + "locked", "toggle_reference_lock",
              {}, {{"axis_str", axis}});
        number(form, tr("Opacidade"), prefix + "opacity", "set_reference_param",
               "val", 0, 1, 0.01, {{"axis_str", axis}, {"param", "opacity"}});
        number(form, tr("Tamanho"), prefix + "size", "set_reference_param",
               "val", 0.1, 50, 0.1, {{"axis_str", axis}, {"param", "size"}});
        number(form, tr("Offset"), prefix + "offset", "set_reference_param",
               "val", -50, 50, 0.1, {{"axis_str", axis}, {"param", "offset"}});
        number(form, tr("Rotação"), prefix + "rotation", "set_reference_param",
               "val", -180, 180, 1,
               {{"axis_str", axis}, {"param", "rotation"}});
        button(form, tr("Alinhar vista"), "align_reference_view",
               {{"axis_str", axis}});
        button(form, tr("Remover referência"), "remove_reference_slot",
               {{"axis_str", axis}});
    }
}
extern "C" bool petunia_qt_reference_load(const char *, const char *, uint32_t,
                                          uint32_t, const uint8_t *, size_t);
bool loadPetuniaReference(const QString &axis, const QString &name,
                          const QImage &image) {
    return petunia_qt_reference_load(
        axis.toUtf8().constData(), name.toUtf8().constData(), image.width(),
        image.height(), image.constBits(), image.sizeInBytes());
}
void PetuniaEditorPanel::setupAssets() {
    auto *form = page(tr("Biblioteca do projeto"));
    text(form, tr("Buscar"), "asset_query", "set_asset_query", "query");
    check(form, tr("Só favoritos"), "asset_only_favorites",
          "set_asset_only_favorites", "only");
    check(form, tr("Ordenar por nome"), "asset_sort_by_name",
          "set_asset_sort_by_name", "sort_by_name");
    button(form, tr("Salvar seleção como asset"), "save_asset");
    rows(form, tr("Assets · clique para inserir"), "prefab_items",
         "place_prefab");
    auto *list = m_rows.last().widget;
    list->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(list, &QListWidget::customContextMenuRequested, this,
            [this, list](const QPoint &pos) {
                auto *item = list->itemAt(pos);
                if (!item)
                    return;
                const auto id = item->data(Qt::UserRole).toString();
                bool favorite = false;
                for (const auto &v : m_state["prefab_items"].toArray())
                    if (v.toObject()["id"] == id)
                        favorite = v.toObject()["favorite"].toBool();
                QMenu menu(this);
                auto *fav = menu.addAction(favorite ? tr("Remover favorito")
                                                    : tr("Favoritar"));
                auto *update = menu.addAction(tr("Atualizar pela seleção"));
                auto *remove = menu.addAction(tr("Remover asset"));
                auto *picked = menu.exec(list->viewport()->mapToGlobal(pos));
                if (picked == fav)
                    send("set_prefab_favorite",
                         {{"id", id}, {"favorite", !favorite}});
                else if (picked == update)
                    send("update_prefab", {{"id", id}});
                else if (picked == remove)
                    send("delete_prefab", {{"id", id}});
            });
    fileButton(form, tr("Importar modelo"), "import_model",
               tr("Modelos (*.obj *.glb *.gltf)"));
    fileButton(form, tr("Exportar OBJ"), "export_obj", tr("OBJ (*.obj)"), true);
    fileButton(form, tr("Exportar GLB"), "export_glb", tr("GLB (*.glb)"), true);
}

PetuniaEditorCanvas::PetuniaEditorCanvas(const QString &mode, QWidget *parent)
    : QWidget(parent), m_mode(mode) {
    setMinimumSize(160, 160);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    setAccessibleName(mode == "PAINT" ? tr("Canvas de pintura do documento")
                      : mode == "UV"  ? tr("Editor UV")
                                      : tr("Dope Sheet e editor de curvas"));
    if (mode != "ANIMATE")
        setAccessibleDescription(mode == "UV"
                                     ? tr("+ e − aproximam, Ctrl+setas deslocam a vista, Home enquadra; setas movem a seleção UV (Shift: passo maior)")
                                     : tr("+ e − aproximam, Ctrl+setas deslocam a vista, Home enquadra"));
}

bool PetuniaEditorCanvas::ownsKey(const QKeyEvent *event) const {
    const int key = event->key();
    if (key == Qt::Key_Escape || key == Qt::Key_Home)
        return true;
    if (m_mode == "ANIMATE")
        return false;
    const bool arrow = key == Qt::Key_Left || key == Qt::Key_Right || key == Qt::Key_Up || key == Qt::Key_Down;
    const bool zoom = key == Qt::Key_Plus || key == Qt::Key_Minus || key == Qt::Key_Equal;
    const auto mods = event->modifiers() & ~(Qt::KeypadModifier);
    return (zoom && !(mods & (Qt::ControlModifier | Qt::AltModifier))) ||
           (arrow && !(mods & Qt::AltModifier));
}

void PetuniaEditorCanvas::zoomAt(qreal factor, const QPointF &anchor) {
    const qreal minimum = m_mode == "PAINT" ? 1. : 0.25;
    const qreal zoom = qBound(minimum, m_zoom * factor, 16.);
    if (!std::isfinite(zoom) || qFuzzyCompare(zoom, m_zoom))
        return;
    // O ponto sob o cursor (ou o centro) continua no mesmo lugar após o zoom.
    const QPointF before = normalized(anchor);
    m_zoom = zoom;
    const auto r = canvasRect();
    m_pan += anchor - QPointF(r.x() + before.x() * r.width(), r.y() + before.y() * r.height());
    if (m_mode == "PAINT") {
        sync(petuniaRequest("paint_canvas_zoom", {{"value", qRound(m_zoom)}}));
        emit editorChanged();
    }
    update();
}
QRectF PetuniaEditorCanvas::canvasRect() const {
    const qreal aspect = m_mode == "PAINT" && !m_image.isNull()
                             ? qreal(m_image.width()) / m_image.height()
                             : 1.;
    const qreal h = qMin(height() - 16., (width() - 16.) / aspect) * m_zoom,
                w = h * aspect;
    return QRectF((width() - w) / 2. + m_pan.x(),
                  (height() - h) / 2. + m_pan.y(), w, h);
}
QPointF PetuniaEditorCanvas::normalized(const QPointF &p) const {
    const auto r = canvasRect();
    return {(p.x() - r.x()) / r.width(), (p.y() - r.y()) / r.height()};
}
bool PetuniaEditorCanvas::reloadCanvasImage() {
    uint32_t w = 0, h = 0;
    const auto size = petunia_qt_canvas_rgba(nullptr, 0, &w, &h);
    if (!size || size > 256u * 1024u * 1024u || !w || !h ||
        size != size_t(w) * h * 4)
        return false;
    QByteArray data;
    data.resize(size);
    if (petunia_qt_canvas_rgba(reinterpret_cast<uint8_t *>(data.data()), size,
                               &w, &h) != size)
        return false;
    m_image = QImage(reinterpret_cast<const uchar *>(data.constData()), w, h,
                     QImage::Format_RGBA8888)
                  .copy();
    return true;
}
void PetuniaEditorCanvas::sync(const QJsonObject &state) {
    // `texture_revision` sobe sempre que os pixels de alguma camada mudam: é
    // o único sinal que distingue pintar de apenas redesenhar.
    const bool pixelsChanged =
        m_state["active_asset_id"] != state["active_asset_id"] ||
        m_state["paint_canvas_revision"] != state["paint_canvas_revision"] ||
        m_state["paint_texture_side"] != state["paint_texture_side"] ||
        m_state["paint_layers"] != state["paint_layers"] ||
        m_state["texture_revision"] != state["texture_revision"];
    m_state = state;
    if (m_mode == "PAINT")
        m_zoom = qBound(0.1, state["paint_canvas_zoom"].toDouble(1.), 16.);
    if ((m_mode == "PAINT" ||
         (m_mode == "UV" && state["uv_show_texture"].toBool())) &&
        (pixelsChanged || m_image.isNull())) {
        if (m_mode == "UV")
            m_image =
                petuniaImage("texture", state["active_asset_id"].toString());
        else if (!reloadCanvasImage())
            m_image = {};
    }
    update();
}
void PetuniaEditorCanvas::paintEvent(QPaintEvent *) {
    QPainter p(this);
    p.fillRect(rect(), palette().color(QPalette::Base));
    p.setRenderHint(QPainter::Antialiasing);
    if (m_mode == "ANIMATE") {
        const auto a = m_state["animate"].toObject();
        const auto tracks = a["tracks"].toArray();
        const int left = 110;
        const qreal span = qMax(1, width() - left - 12);
        const qreal progress = a["progress"].toDouble();
        if (a["curve_mode"].toBool()) {
            p.save();
            p.translate(left, 10);
            p.scale(span / 256., (height() - 20.) / 256.);
            p.setPen(QPen(palette().color(QPalette::Highlight), 1));
            p.drawPath(petuniaPath(a["curve_path"].toString()));
            p.restore();
        } else {
            int row = 0;
            for (const auto &t : tracks) {
                auto obj = t.toObject();
                const qreal y = 30 + row++ * 26;
                p.setPen(palette().color(QPalette::Text));
                p.drawText(QRectF(6, y - 10, left - 10, 22), Qt::AlignVCenter,
                           obj["bone_name"].toString());
                p.setPen(palette().color(QPalette::Mid));
                p.drawLine(left, y, width() - 12, y);
                for (const auto &k : obj["keyframes"].toArray()) {
                    auto key = k.toObject();
                    const auto x =
                        left + key["normalized_pos"].toDouble() * span;
                    p.setPen(Qt::NoPen);
                    p.setBrush(key["selected"].toBool()
                                   ? palette().color(QPalette::Highlight)
                                   : palette().color(QPalette::Text));
                    QPolygonF diamond;
                    diamond << QPointF(x, y - 5) << QPointF(x + 5, y)
                            << QPointF(x, y + 5) << QPointF(x - 5, y);
                    p.drawPolygon(diamond);
                }
            }
        }
        p.setPen(QPen(palette().color(QPalette::Highlight), 2));
        const auto x = left + progress * span;
        p.drawLine(QPointF(x, 8), QPointF(x, height() - 10));
        p.drawText(6, height() - 8, a["frame_label"].toString());
        return;
    }
    const auto r = canvasRect();
    p.fillRect(r, palette().color(QPalette::Base));
    if (m_mode == "PAINT") {
        const auto visible = r.intersected(rect());
        const qreal tile = 16.;
        for (int y = int(std::floor((visible.top() - r.top()) / tile));
             y <= int((visible.bottom() - r.top()) / tile); ++y)
            for (int x = int(std::floor((visible.left() - r.left()) / tile));
                 x <= int((visible.right() - r.left()) / tile); ++x)
                p.fillRect(
                    QRectF(r.x() + x * tile, r.y() + y * tile, tile, tile)
                        .intersected(r),
                    (x + y) % 2 ? palette().color(QPalette::Mid)
                                : palette().color(QPalette::Base));
    }

    if (!m_image.isNull() &&
        (m_mode == "PAINT" || m_state["uv_show_texture"].toBool()))
        p.drawImage(r, m_image);
    p.save();
    p.translate(r.topLeft());
    p.scale(r.width() / 256., r.height() / 256.);
    const auto uv = m_state["uv_editor"].toObject();
    if (m_mode == "UV") {
        // Grade 0..1 em oitavos: referência de espaço UV para mover/escalar.
        p.setPen(QPen(palette().color(QPalette::Mid), 256. / r.width()));
        for (int i = 1; i < 8; ++i) {
            p.drawLine(QPointF(i * 32., 0.), QPointF(i * 32., 256.));
            p.drawLine(QPointF(0., i * 32.), QPointF(256., i * 32.));
        }
    }
    if (m_mode == "UV" || m_state["paint_show_uv_overlay"].toBool()) {
        for (const auto &key :
             QStringList{"layout_commands", "seam_commands",
                         "selected_commands", "pinned_commands"}) {
            const auto &theme = PetuniaTheme::instance();
            p.setPen(QPen(key == "seam_commands" ? theme.statusError()
                          : key == "selected_commands"
                              ? palette().color(QPalette::Highlight)
                          : key == "pinned_commands" ? theme.statusInfo()
                                                     : theme.borderStrong(),
                          (key == "layout_commands" ? 1. : 2.) * 256. / r.width()));
            p.setBrush(Qt::NoBrush);
            if (m_uvDragging && key == "selected_commands") {
                p.save();
                p.translate(m_dragDelta.x() * 256., m_dragDelta.y() * 256.);
                p.drawPath(petuniaPath(uv[key].toString()));
                p.restore();
            } else
                p.drawPath(petuniaPath(uv[key].toString()));
        }
    }
    if (m_mode == "PAINT" && m_state["paint_pixel_grid"].toBool()) {
        p.setPen(QPen(palette().color(QPalette::Mid), 0.25));
        p.drawPath(
            petuniaPath(m_state["paint_canvas_grid_commands"].toString()));
    }
    p.restore();
    p.setPen(palette().color(QPalette::Mid));
    p.drawRect(r);
    if (m_selecting) {
        p.setPen(QPen(palette().color(QPalette::Highlight), 1, Qt::DashLine));
        p.drawRect(QRectF(m_press, m_previous).normalized());
    }
    if (m_image.isNull() && m_mode == "PAINT")
        p.drawText(rect(), Qt::AlignCenter,
                   tr("Selecione um objeto e uma camada de pintura"));
    // Anel do pincel: mesma superfície no canvas 2D e no editor UV.
    const bool paintSurface =
        m_mode == "PAINT" ||
        (m_mode == "UV" && m_state["uv_show_texture"].toBool());
    if (paintSurface && underMouse() && isPaintBrushTool())
        drawPaintCursor(p);
}
bool PetuniaEditorCanvas::isPaintBrushTool() const {
    static const QStringList tools{
        QStringLiteral("brush"),    QStringLiteral("eraser"),
        QStringLiteral("airbrush"), QStringLiteral("pixel"),
        QStringLiteral("smudge"),   QStringLiteral("blur"),
        QStringLiteral("dodge"),    QStringLiteral("burn"),
        QStringLiteral("spray"),    QStringLiteral("clone")};
    return tools.contains(m_state["active_tool"].toString());
}
void PetuniaEditorCanvas::drawPaintCursor(QPainter &p) const {
    // Mesmo diâmetro (px de tela) do carimbo no viewport e no canvas.
    const double diameter = qMax(2.0, m_state["brush_size"].toDouble(1.0) * 16.0);
    const double radius = diameter * 0.5;
    const bool erase =
        m_state["active_tool"].toString() == QStringLiteral("eraser");
    const double hardness =
        qBound(0.0, m_state["brush_hardness"].toDouble(0.0), 1.0);
    const double strength = qBound(
        0.0,
        m_state["brush_opacity"].toDouble(1.0) *
            m_state["brush_flow"].toDouble(1.0),
        1.0);
    QColor tint(181, 140, 255);
    const auto color = m_state["paint_color"].toArray();
    if (color.size() >= 3)
        tint = QColor::fromRgbF(color[0].toDouble(), color[1].toDouble(),
                                color[2].toDouble());
    const QColor ring =
        erase ? QColor(239, 107, 115) : QColor(255, 255, 255, 221);
    QColor fill = erase ? QColor(239, 107, 115) : tint;
    fill.setAlphaF(0.04 + 0.32 * strength);
    p.save();
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setBrush(fill);
    p.setPen(QPen(ring, 1.5));
    p.drawEllipse(m_mousePos, radius, radius);
    // Contorno escuro interno: lê bem sobre pintura clara e escura.
    p.setBrush(Qt::NoBrush);
    p.setPen(QPen(QColor(0, 0, 0, 170), 1.0));
    p.drawEllipse(m_mousePos, qMax(1.0, radius - 1.5),
                  qMax(1.0, radius - 1.5));
    if (m_state["active_tool"].toString() != QStringLiteral("pixel") &&
        hardness > 0.02 && hardness < 0.98 && diameter > 10.0) {
        p.setPen(QPen(QColor(255, 255, 255, 178), 1.0));
        p.drawEllipse(m_mousePos, radius * hardness, radius * hardness);
    }
    p.setPen(QPen(QColor(0, 0, 0, 170), 1.0));
    p.setBrush(QColor(255, 255, 255));
    p.drawEllipse(m_mousePos, 1.5, 1.5);
    p.restore();
}
void PetuniaEditorCanvas::enterEvent(QEnterEvent *event) {
    QWidget::enterEvent(event);
    update();
}
void PetuniaEditorCanvas::leaveEvent(QEvent *event) {
    QWidget::leaveEvent(event);
    update();
}
void PetuniaEditorCanvas::stroke(const QPointF &point, int phase) {
    const auto n = normalized(point);
    // `raster_px` é a largura em px em que a textura aparece no canvas: sem
    // ela o pincel (px de tela) não vira texels e o carimbo não bate com o anel.
    auto result = petuniaRequest(
        "paint_stroke",
        {{"x", n.x()},
         {"y", n.y()},
         {"phase", phase},
         {"raster_px", m_mode == "PAINT" ? canvasRect().width() : 0.}});
    const bool painting = m_mode == "PAINT";
    sync(result);
    // O snapshot não muda com cada dab: a textura é relida aqui, como o Slint
    // recompõe o raster a cada evento. Texturas grandes só no fim do traço.
    if (painting && (phase != 1 || m_image.sizeInBytes() <= 16 * 1024 * 1024))
        reloadCanvasImage();
    emit editorChanged();
}
void PetuniaEditorCanvas::mousePressEvent(QMouseEvent *e) {
    setFocus();
    m_mousePos = e->position();
    m_previous = e->position();
    m_press = e->position();
    m_dragDelta = {};
    if (e->button() == Qt::MiddleButton) {
        m_panning = true;
        e->accept();
        return;
    }
    if (e->button() != Qt::LeftButton)
        return;
    if (m_mode == "PAINT") {
        m_selecting = m_state["active_tool"] == "select" ||
                      m_state["active_tool"] == "lasso_select";
        if (!m_selecting) {
            m_stroking = true;
            stroke(e->position(), 0);
        }
    } else if (m_mode == "UV") {
        const auto n = normalized(e->position());
        sync(petuniaRequest(
            "uv_editor_click",
            {{"u", n.x()},
             {"v", 1. - n.y()},
             {"extend", bool(e->modifiers() & Qt::ShiftModifier)}}));
        m_uvDragging = true;
        m_dragRevision = m_state["document_revision"].toString();
        m_dragAsset = m_state["active_asset_id"].toString();
        m_dragSelection = m_state["uv_selection_stamp"].toString();
        emit editorChanged();
    } else {
        const auto a = m_state["animate"].toObject();
        const auto tracks = a["tracks"].toArray();
        const int row = int((e->position().y() - 17) / 26);
        if (row >= 0 && row < tracks.size()) {
            const auto track = tracks[row].toObject();
            const auto bone = QString::number(track["bone_id"].toInt());
            const auto progress = qBound(
                0., (e->position().x() - 110) / qMax(1., width() - 122.), 1.);
            if (e->position().x() < 110)
                sync(petuniaRequest("animate",
                                    {{"id", "select-bone"}, {"arg", bone}}));
            else {
                bool hit = false;
                for (const auto &v : track["keyframes"].toArray()) {
                    const auto k = v.toObject();
                    if (std::abs(k["normalized_pos"].toDouble() - progress) <
                        0.02) {
                        sync(petuniaRequest("animate",
                                            {{"id", "select-keyframe"},
                                             {"arg", bone},
                                             {"value", k["time"]}}));
                        hit = true;
                        break;
                    }
                }
                if (!hit)
                    sync(petuniaRequest("animate",
                                        {{"id", "seek"}, {"value", progress}}));
            }
        } else
            sync(petuniaRequest("animate",
                                {{"id", "seek"},
                                 {"value", qBound(0.,
                                                  (e->position().x() - 110) /
                                                      qMax(1., width() - 122.),
                                                  1.)}}));
        emit editorChanged();
    }
    e->accept();
}
void PetuniaEditorCanvas::mouseMoveEvent(QMouseEvent *e) {
    m_mousePos = e->position();
    if (m_panning) {
        m_pan += e->position() - m_previous;
        m_previous = e->position();
        update();
    } else if (m_uvDragging) {
        m_dragDelta = normalized(e->position()) - normalized(m_press);
        update();
    } else if (m_selecting) {
        m_previous = e->position();
        update();
    } else if (m_stroking) {
        m_previous = e->position();
        stroke(e->position(), 1);
    }
    e->accept();
}
void PetuniaEditorCanvas::mouseReleaseEvent(QMouseEvent *e) {
    // Só o botão que iniciou o gesto o encerra (soltar o direito no meio de um
    // traço com o esquerdo não fecha o traço).
    if (e->button() == Qt::MiddleButton) {
        m_panning = false;
        e->accept();
        return;
    }
    if (e->button() != Qt::LeftButton) {
        e->accept();
        return;
    }
    if (m_stroking) {
        stroke(e->position(), 2);
        m_stroking = false;
    }
    if (m_uvDragging) {
        m_dragDelta = normalized(e->position()) - normalized(m_press);
        m_uvDragging = false;
        if ((e->position() - m_press).manhattanLength() > 3) {
            sync(petuniaRequest("uv_drag_commit",
                                {{"du", m_dragDelta.x()},
                                 {"dv", -m_dragDelta.y()},
                                 {"revision", m_dragRevision},
                                 {"asset", m_dragAsset},
                                 {"selection", m_dragSelection}}));
            emit editorChanged();
        }
    }
    if (m_selecting) {
        m_selecting = false;
        const auto from = normalized(m_press), to = normalized(e->position());
        sync(petuniaRequest(
            "paint_2d_select",
            {{"x0", from.x()},
             {"y0", from.y()},
             {"x1", to.x()},
             {"y1", to.y()},
             {"extend", bool(e->modifiers() & Qt::ShiftModifier)},
             {"subtract", bool(e->modifiers() & Qt::ControlModifier)},
             {"island", false}}));
        emit editorChanged();
    }
    m_panning = false;
    update();
    e->accept();
}
void PetuniaEditorCanvas::wheelEvent(QWheelEvent *e) {
    if (m_mode == "ANIMATE") {
        e->ignore();
        return;
    }
    zoomAt(std::pow(1.15, e->angleDelta().y() / 120.), e->position());
    e->accept();
}
void PetuniaEditorCanvas::tabletEvent(QTabletEvent *e) {
    if (m_mode != "PAINT") {
        e->ignore();
        return;
    }
    const int phase = e->type() == QEvent::TabletPress     ? 0
                      : e->type() == QEvent::TabletRelease ? 2
                                                           : 1;
    // Pressure is temporary input, never a persistent replacement for opacity.
    extern void setPetuniaTabletInput(float, float, float, bool);
    setPetuniaTabletInput(e->pressure(), e->xTilt(), e->yTilt(),
                          e->pointerType() ==
                              QPointingDevice::PointerType::Eraser);
    stroke(e->position(), phase);
    if (phase == 2)
        petuniaRequest("tablet_reset");
    m_stroking = phase != 2;
    e->accept();
}
void PetuniaEditorCanvas::keyPressEvent(QKeyEvent *e) {
    if (e->key() == Qt::Key_Escape) {
        if (m_stroking)
            stroke(m_previous, 3);
        m_stroking = false;
        m_uvDragging = false;
        m_selecting = false;
        petuniaRequest("tablet_reset");
        petuniaRequest("shortcut", {{"text", "Escape"},
                                    {"ctrl", false},
                                    {"shift", false},
                                    {"alt", false}});
        update();
        e->accept();
        return;
    }
    if (m_mode != "ANIMATE") {
        const QPointF center = QRectF(rect()).center();
        if (e->key() == Qt::Key_Plus || e->key() == Qt::Key_Equal) {
            zoomAt(1.25, center);
            e->accept();
            return;
        }
        if (e->key() == Qt::Key_Minus) {
            zoomAt(1. / 1.25, center);
            e->accept();
            return;
        }
        const bool arrow = e->key() == Qt::Key_Left || e->key() == Qt::Key_Right ||
                           e->key() == Qt::Key_Up || e->key() == Qt::Key_Down;
        if (arrow) {
            const qreal dx = e->key() == Qt::Key_Left ? -1. : e->key() == Qt::Key_Right ? 1. : 0.;
            const qreal dy = e->key() == Qt::Key_Up ? -1. : e->key() == Qt::Key_Down ? 1. : 0.;
            if (e->modifiers() & Qt::ControlModifier) {
                m_pan += QPointF(dx, dy) * 32.;
                update();
            } else if (m_mode == "UV") {
                // Um passo de grade (1/256 ou 1/32 com Shift) por tecla: um Undo cada.
                const qreal step = (e->modifiers() & Qt::ShiftModifier) ? 1. / 32. : 1. / 256.;
                sync(petuniaRequest("uv_move_selected", {{"du", dx * step}, {"dv", -dy * step}}));
                emit editorChanged();
            } else {
                QWidget::keyPressEvent(e);
                return;
            }
            e->accept();
            return;
        }
    }
    if (e->key() == Qt::Key_Home) {
        m_zoom = 1;
        m_pan = {};
        if (m_mode == "PAINT")
            sync(petuniaRequest("paint_canvas_zoom", {{"value", 1}}));
        update();
        e->accept();
        return;
    }
    QWidget::keyPressEvent(e);
}

// Perder o foco no meio de um gesto (Alt+Tab, popup, F6) não pode deixar o
// traço aberto: o traço pintado é trabalho intencional e é confirmado (um Undo);
// arrastos de UV e seleção retangular são descartados sem efeito.
void PetuniaEditorCanvas::focusOutEvent(QFocusEvent *e) {
    QWidget::focusOutEvent(e);
    if (m_stroking) {
        stroke(m_previous, 2);
        m_stroking = false;
        petuniaRequest("tablet_reset");
    }
    m_uvDragging = false;
    m_selecting = false;
    m_panning = false;
    update();
}

void setPetuniaTabletInput(float pressure, float tiltX, float tiltY,
                           bool eraser) {
    petuniaRequest("tablet_input", {{"pressure", pressure},
                                    {"tilt_x", tiltX},
                                    {"tilt_y", tiltY},
                                    {"eraser", eraser}});
}

#include <QAbstractButton>
#include <QAction>
#include <QDockWidget>
#include <QGroupBox>
#include <QPointer>
#include <QTabBar>
#include <QTranslator>
static QJsonObject nativeTranslations;
static int nativeTranslationRevision = 0;
static QString nativeText(const QString &source) {
    auto result = nativeTranslations.value(source).toString();
    if (result.isEmpty() && source.contains('&')) {
        auto plain = source;
        plain.remove('&');
        result = nativeTranslations.value(plain).toString();
        if (!result.isEmpty())
            result.prepend('&');
    }
    return result.isEmpty() ? source : result;
}
class PetuniaNativeTranslator : public QTranslator {
  public:
    explicit PetuniaNativeTranslator(QObject *parent) : QTranslator(parent) {}
    QString translate(const char *, const char *source, const char *,
                      int) const override {
        const auto text = QString::fromUtf8(source);
        const auto value = nativeText(text);
        return value == text ? QString() : value;
    }
};
void petuniaSetTranslations(const QJsonObject &translations) {
    nativeTranslations = translations;
    ++nativeTranslationRevision;
    static QPointer<PetuniaNativeTranslator> translator;
    if (!translator) {
        translator = new PetuniaNativeTranslator(qApp);
        qApp->installTranslator(translator);
    }
}
void petuniaLocalize(QWidget *root) {
    auto widgets = root->findChildren<QWidget *>();
    widgets.prepend(root);
    for (auto *widget : widgets) {
        if (widget->property("nativeTranslationRevision").toInt() ==
                nativeTranslationRevision ||
            widget->property("snapshotDerived").toBool())
            continue;
        widget->setProperty("nativeTranslationRevision",
                            nativeTranslationRevision);
        const QSignalBlocker block(widget);
        auto source = [&](const char *key, const QString &current) {
            if (!widget->property(key).isValid())
                widget->setProperty(key, current);
            return widget->property(key).toString();
        };
        if (!widget->property("dynamic").toBool()) {
            if (auto *w = qobject_cast<QAbstractButton *>(widget))
                w->setText(nativeText(source("nativeSourceText", w->text())));
            else if (auto *w = qobject_cast<QLabel *>(widget))
                w->setText(nativeText(source("nativeSourceText", w->text())));
        }
        if (auto *w = qobject_cast<QGroupBox *>(widget))
            w->setTitle(nativeText(source("nativeSourceTitle", w->title())));
        if (widget->isWindow() || qobject_cast<QDockWidget *>(widget))
            widget->setWindowTitle(
                nativeText(source("nativeSourceTitle", widget->windowTitle())));
        if (auto *w = qobject_cast<QLineEdit *>(widget))
            w->setPlaceholderText(nativeText(
                source("nativeSourcePlaceholder", w->placeholderText())));
        if (auto *w = qobject_cast<QComboBox *>(widget)) {
            if (!w->property("nativeSourceItems").isValid()) {
                QStringList items;
                for (int i = 0; i < w->count(); ++i)
                    items << w->itemText(i);
                w->setProperty("nativeSourceItems", items);
            }
            const auto items = w->property("nativeSourceItems").toStringList();
            for (int i = 0; i < w->count() && i < items.size(); ++i)
                w->setItemText(i, nativeText(items[i]));
        }
        if (auto *w = qobject_cast<QTabWidget *>(widget))
            for (int i = 0; i < w->count(); ++i) {
                auto *page = w->widget(i);
                if (!page->property("nativeSourceTab").isValid())
                    page->setProperty("nativeSourceTab", w->tabText(i));
                const QString srcTab = page->property("nativeSourceTab").toString();
                if (!srcTab.isEmpty()) {
                    w->setTabText(i, nativeText(srcTab));
                }
            }
        if (!widget->accessibleName().isEmpty())
            widget->setAccessibleName(nativeText(source(
                "nativeSourceAccessibleName", widget->accessibleName())));
        if (!widget->toolTip().isEmpty())
            widget->setToolTip(
                nativeText(source("nativeSourceTooltip", widget->toolTip())));
    }
    for (auto *action : root->findChildren<QAction *>()) {
        if (action->property("nativeTranslationRevision").toInt() ==
            nativeTranslationRevision)
            continue;
        action->setProperty("nativeTranslationRevision",
                            nativeTranslationRevision);
        if (!action->property("nativeSourceText").isValid())
            action->setProperty("nativeSourceText", action->text());
        action->setText(
            nativeText(action->property("nativeSourceText").toString()));
    }
}
