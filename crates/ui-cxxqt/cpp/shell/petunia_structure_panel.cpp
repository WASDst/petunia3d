#include "petunia_structure_panel.h"
#include "../petunia_icons.h"
#include "../petunia_color_picker.h"
#include "../petunia_placeholder_message.h"
#include "../petunia_theme.h"

#include <QHBoxLayout>
#include <QHeaderView>
#include <QItemSelectionModel>
#include <QJsonArray>
#include <QMenu>
#include <QShortcut>
#include <QSignalBlocker>
#include <QVBoxLayout>

namespace {
class SelectedOnlyProxy : public QSortFilterProxyModel {
  public:
    using QSortFilterProxyModel::QSortFilterProxyModel;
    void setSelectedOnly(bool enabled) {
        if (m_selectedOnly != enabled) {
            m_selectedOnly = enabled;
            invalidate();
        }
    }

  protected:
    bool filterAcceptsRow(int row, const QModelIndex &parent) const override {
        if (m_selectedOnly) {
            const auto *model = qobject_cast<QStandardItemModel *>(sourceModel());
            const auto *item = model ? model->item(row, 0) : nullptr;
            if (item && !item->data(Qt::UserRole + 1).toBool())
                return false;
        }
        return QSortFilterProxyModel::filterAcceptsRow(row, parent);
    }

  private:
    bool m_selectedOnly{false};
};
} // namespace

PetuniaStructurePanel::PetuniaStructurePanel(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaStructurePanel"));
    setAccessibleName(tr("Estrutura: partes da cena"));
    const auto &theme = PetuniaTheme::instance();

    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(2));
    root->setSpacing(theme.space(2));

    auto *toolbar = new QHBoxLayout();
    toolbar->setSpacing(theme.space(1));
    m_search = new QLineEdit(this);
    m_search->setPlaceholderText(tr("Buscar partes"));
    m_search->setAccessibleName(tr("Buscar partes"));
    m_search->setClearButtonEnabled(true);
    toolbar->addWidget(m_search, 1);

    auto iconButton = [&](const QString &icon, const QString &tip, bool checkable) {
        auto *button = new QPushButton(PetuniaIcons::get(icon), QString(), this);
        button->setProperty("role", QStringLiteral("quiet"));
        button->setCheckable(checkable);
        button->setToolTip(tip);
        button->setAccessibleName(tip);
        button->setFixedWidth(theme.controlHeight());
        return button;
    };
    m_filterSelected = iconButton(QStringLiteral("filter"), tr("Mostrar só partes selecionadas"), true);
    m_sort = iconButton(QStringLiteral("sort_az"), tr("Ordenar por nome (A–Z)"), true);
    auto *separate = iconButton(QStringLiteral("separate"), tr("Separar seleção em nova parte"), false);
    toolbar->addWidget(m_filterSelected);
    toolbar->addWidget(m_sort);
    toolbar->addWidget(separate);
    root->addLayout(toolbar);

    m_model = new QStandardItemModel(this);
    m_model->setHorizontalHeaderLabels({tr("Nome"), tr("Visível"), tr("Bloqueio")});
    auto *proxy = new SelectedOnlyProxy(this);
    proxy->setSourceModel(m_model);
    proxy->setFilterCaseSensitivity(Qt::CaseInsensitive);
    proxy->setFilterKeyColumn(Name);
    m_proxy = proxy;

    m_tree = new QTreeView(this);
    m_tree->setModel(m_proxy);
    m_tree->setAccessibleName(tr("Partes da cena"));
    m_tree->setHeaderHidden(true);
    m_tree->setRootIsDecorated(false);
    m_tree->setIndentation(0);
    m_tree->setUniformRowHeights(true);
    m_tree->setSelectionMode(QAbstractItemView::ExtendedSelection);
    m_tree->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_tree->setContextMenuPolicy(Qt::CustomContextMenu);
    m_tree->setIconSize(QSize(theme.iconPx(), theme.iconPx()));
    m_tree->header()->setStretchLastSection(false);
    m_tree->header()->setSectionResizeMode(Name, QHeaderView::Stretch);
    m_tree->header()->setSectionResizeMode(Visible, QHeaderView::Fixed);
    m_tree->header()->setSectionResizeMode(Locked, QHeaderView::Fixed);
    m_tree->header()->resizeSection(Visible, theme.controlHeight());
    m_tree->header()->resizeSection(Locked, theme.controlHeight());
    root->addWidget(m_tree, 1);

    m_empty = new PetuniaPlaceholderMessage(this);
    m_empty->setTitle(tr("Cena vazia"));
    m_empty->setExplanation(tr("Nenhum objeto na cena. Crie uma primitiva para começar a modelar."));
    m_empty->setActionText(tr("Criar primitiva"));
    root->addWidget(m_empty, 1);
    m_empty->hide();
    connect(m_empty, &PetuniaPlaceholderMessage::actionTriggered, this, &PetuniaStructurePanel::createRequested);

    connect(m_search, &QLineEdit::textChanged, m_proxy, &QSortFilterProxyModel::setFilterFixedString);
    connect(m_filterSelected, &QPushButton::toggled, this,
            [proxy](bool on) { static_cast<SelectedOnlyProxy *>(proxy)->setSelectedOnly(on); });
    connect(m_sort, &QPushButton::toggled, this, [this](bool on) {
        if (on)
            m_proxy->sort(Name, Qt::AscendingOrder);
        else
            m_proxy->sort(-1);
    });
    connect(separate, &QPushButton::clicked, this, [this] {
        const QString ws = m_controller->workspaceId();
        if (ws == QLatin1String("PAINT"))
            m_controller->request(QStringLiteral("add_paint_layer"));
        else if (ws == QLatin1String("ANIMATE"))
            m_controller->request(QStringLiteral("animate"), {{"id", "fit-to-model"}});
        else
            m_controller->command(QStringLiteral("model.separate_selection"));
    });
    connect(m_tree, &QTreeView::clicked, this, &PetuniaStructurePanel::onClicked);
    connect(m_tree, &QTreeView::customContextMenuRequested, this, &PetuniaStructurePanel::onContextMenu);
    connect(m_tree->selectionModel(), &QItemSelectionModel::selectionChanged, this,
            &PetuniaStructurePanel::onSelectionChanged);
    connect(m_model, &QStandardItemModel::itemChanged, this, [this](QStandardItem *item) {
        if (m_syncing || item->column() != Name)
            return;
        const QString ws = m_controller->workspaceId();
        if (ws != QLatin1String("PAINT") && ws != QLatin1String("ANIMATE")) {
            m_controller->request(QStringLiteral("rename"),
                                  {{"id", item->data(IdRole).toString()}, {"name", item->text()}});
        }
    });
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaStructurePanel::refreshIcons);
    buildShortcuts();
}

void PetuniaStructurePanel::buildShortcuts() {
    auto bind = [this](const QKeySequence &keys, std::function<void()> fn) {
        auto *shortcut = new QShortcut(keys, m_tree);
        shortcut->setContext(Qt::WidgetShortcut);
        connect(shortcut, &QShortcut::activated, this, std::move(fn));
    };
    auto eachSelected = [this](const QString &action, const QString &extra = {}) {
        for (const auto &id : selectedIds()) {
            QJsonObject args{{"id", id}};
            if (!extra.isEmpty())
                args.insert("target", extra);
            m_controller->request(action, args);
        }
    };
    bind(QKeySequence(Qt::Key_Delete), [this] { m_controller->command(QStringLiteral("edit.delete")); });
    bind(QKeySequence(Qt::Key_Backspace), [this] { m_controller->command(QStringLiteral("edit.delete")); });
    bind(QKeySequence(Qt::SHIFT | Qt::Key_H), [this] { m_controller->request(QStringLiteral("isolate")); });
    bind(QKeySequence(Qt::Key_H), [eachSelected] { eachSelected(QStringLiteral("visibility")); });
    bind(QKeySequence(Qt::Key_L), [eachSelected] { eachSelected(QStringLiteral("lock")); });
    bind(QKeySequence(Qt::Key_F2), [this] {
        const auto current = m_tree->currentIndex();
        if (current.isValid())
            m_tree->edit(current.siblingAtColumn(Name));
    });
    bind(QKeySequence(Qt::CTRL | Qt::Key_D),
         [eachSelected] { eachSelected(QStringLiteral("scene_context"), QStringLiteral("duplicate")); });
}

QString PetuniaStructurePanel::idAt(const QModelIndex &proxyIndex) const {
    const auto source = m_proxy->mapToSource(proxyIndex);
    if (!source.isValid())
        return {};
    const auto *item = m_model->item(source.row(), Name);
    return item ? item->data(IdRole).toString() : QString();
}

QStringList PetuniaStructurePanel::selectedIds() const {
    QStringList ids;
    for (const auto &row : m_tree->selectionModel()->selectedRows(Name))
        ids << idAt(row);
    return ids;
}

void PetuniaStructurePanel::onClicked(const QModelIndex &proxyIndex) {
    if (m_syncing)
        return;
    const auto source = m_proxy->mapToSource(proxyIndex);
    if (source.column() != Visible && source.column() != Locked)
        return;

    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT")) {
        const QString id = idAt(proxyIndex);
        if (source.column() == Visible)
            m_controller->request(QStringLiteral("toggle_paint_layer_visibility"), {{"id", id}});
        else
            m_controller->request(QStringLiteral("toggle_paint_layer_lock"), {{"id", id}});
        return;
    }
    if (ws == QLatin1String("ANIMATE")) {
        if (source.column() == Visible)
            m_controller->request(QStringLiteral("animate"), {{"id", "toggle-bones"}});
        return;
    }

    const QString action = source.column() == Visible ? QStringLiteral("visibility") : QStringLiteral("lock");
    const auto rows = m_tree->selectionModel()->selectedRows(Name);
    // Clicar o ícone de uma linha dentro da seleção aplica a toda a seleção.
    if (rows.size() > 1 && rows.contains(proxyIndex.siblingAtColumn(Name))) {
        for (const auto &row : rows)
            m_controller->request(action, {{"id", idAt(row)}});
    } else {
        m_controller->request(action, {{"id", idAt(proxyIndex)}});
    }
}

void PetuniaStructurePanel::onSelectionChanged() {
    if (m_syncing)
        return;
    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT")) {
        const QString active = idAt(m_tree->currentIndex());
        if (!active.isEmpty())
            m_controller->request(QStringLiteral("set_paint_layer_active"), {{"id", active}});
        return;
    }
    if (ws == QLatin1String("ANIMATE")) {
        const QString active = idAt(m_tree->currentIndex());
        if (!active.isEmpty())
            m_controller->request(QStringLiteral("animate"), {{"id", "select-bone"}, {"arg", active}});
        return;
    }

    QJsonArray ids;
    for (const auto &id : selectedIds())
        ids.append(id);
    const QString active = idAt(m_tree->currentIndex());
    m_controller->request(QStringLiteral("scene_selection"), {{"ids", ids}, {"active", active}});
}

void PetuniaStructurePanel::refreshIcons() {
    m_tree->setIconSize(QSize(PetuniaTheme::instance().iconPx(), PetuniaTheme::instance().iconPx()));
    sync(m_controller->state());
}

void PetuniaStructurePanel::sync(const QJsonObject &state) {
    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT"))
        syncPaint(state);
    else if (ws == QLatin1String("ANIMATE"))
        syncAnimate(state);
    else
        syncScene(state);
}

void PetuniaStructurePanel::syncPaint(const QJsonObject &state) {
    const auto layers = state["paint_layers"].toArray();
    const bool isEmpty = layers.isEmpty();
    m_empty->setTitle(tr("Sem camadas"));
    m_empty->setExplanation(tr("Nenhuma camada de pintura. Crie uma camada para começar a pintar."));
    m_empty->setActionText(tr("Nova camada"));
    m_empty->setVisible(isEmpty);
    m_tree->setVisible(!isEmpty);
    m_search->setPlaceholderText(tr("Buscar camadas"));
    const auto &theme = PetuniaTheme::instance();

    m_syncing = true;
    bool rebuild = m_model->rowCount() != layers.size();
    for (int i = 0; !rebuild && i < layers.size(); ++i) {
        const auto o = layers[i].toObject();
        rebuild = m_model->item(i, Name)->data(IdRole).toString() != o["id"].toString();
    }
    if (rebuild) {
        m_model->removeRows(0, m_model->rowCount());
        for (const auto &value : layers) {
            const auto o = value.toObject();
            auto *name = new QStandardItem(o["name"].toString());
            name->setData(o["id"].toString(), IdRole);
            name->setEditable(false);
            auto *eye = new QStandardItem;
            eye->setEditable(false);
            auto *lock = new QStandardItem;
            lock->setEditable(false);
            m_model->appendRow({name, eye, lock});
        }
    }
    const QSignalBlocker block(m_tree->selectionModel());
    m_tree->selectionModel()->clearSelection();
    for (int i = 0; i < layers.size(); ++i) {
        const auto o = layers[i].toObject();
        auto *name = m_model->item(i, Name);
        if (name->text() != o["name"].toString())
            name->setText(o["name"].toString());
        const bool active = o["active"].toBool();
        name->setData(active, SelectedRole);
        name->setToolTip(active ? tr("Camada ativa") : tr("Camada de pintura"));

        QColor tint = theme.accentPrimary();
        const QString kind = o["kind"].toString();
        const bool isGroup = o["is_group"].toBool();
        QString iconName = QStringLiteral("layers");
        if (isGroup)
            iconName = QStringLiteral("folder");
        else if (kind == QLatin1String("decal"))
            iconName = QStringLiteral("decal");
        else if (kind == QLatin1String("effect"))
            iconName = QStringLiteral("wand");
        name->setIcon(PetuniaIcons::get(iconName, tint.name()));

        auto *eye = m_model->item(i, Visible);
        const bool visible = o["visible"].toBool(true);
        eye->setIcon(PetuniaIcons::get(visible ? QStringLiteral("eye") : QStringLiteral("eye_hidden"),
                                       (visible ? theme.textSecondary() : theme.textMuted()).name()));
        eye->setTextAlignment(Qt::AlignCenter);
        eye->setToolTip(visible ? tr("Visível — clique para ocultar") : tr("Oculta — clique para exibir"));
        eye->setData(visible ? tr("Visível") : tr("Oculta"), Qt::AccessibleTextRole);

        auto *lock = m_model->item(i, Locked);
        const bool locked = o["locked"].toBool(false);
        lock->setIcon(PetuniaIcons::get(locked ? QStringLiteral("lock") : QStringLiteral("unlock"),
                                        (locked ? theme.textSecondary() : theme.textMuted()).name()));
        lock->setTextAlignment(Qt::AlignCenter);
        lock->setToolTip(locked ? tr("Bloqueada — clique para liberar") : tr("Livre — clique para bloquear"));
        lock->setData(locked ? tr("Bloqueada") : tr("Livre"), Qt::AccessibleTextRole);

        const auto proxy = m_proxy->mapFromSource(m_model->index(i, Name));
        if (active) {
            m_tree->selectionModel()->select(proxy, QItemSelectionModel::Select | QItemSelectionModel::Rows);
            m_tree->selectionModel()->setCurrentIndex(proxy, QItemSelectionModel::NoUpdate);
        }
    }
    m_syncing = false;
}

void PetuniaStructurePanel::syncAnimate(const QJsonObject &state) {
    const auto anim = state["animate"].toObject();
    auto tracks = anim["tracks"].toArray();
    if (tracks.isEmpty()) {
        const auto bones = anim["bones"].toArray();
        for (const auto &b : bones) {
            const auto bo = b.toObject();
            QJsonObject tr;
            tr["bone_id"] = bo["id"];
            tr["bone_name"] = bo["name"];
            tr["selected"] = bo["selected"];
            tracks.append(tr);
        }
    }
    const bool isEmpty = tracks.isEmpty();
    m_empty->setTitle(tr("Sem criatura"));
    m_empty->setExplanation(tr("Nenhum osso ou criatura ativa. Adicione uma criatura para animar."));
    m_empty->setActionText(tr("Adicionar criatura"));
    m_empty->setVisible(isEmpty);
    m_tree->setVisible(!isEmpty);
    m_search->setPlaceholderText(tr("Buscar ossos"));
    const auto &theme = PetuniaTheme::instance();

    m_syncing = true;
    bool rebuild = m_model->rowCount() != tracks.size();
    for (int i = 0; !rebuild && i < tracks.size(); ++i) {
        const auto o = tracks[i].toObject();
        rebuild = m_model->item(i, Name)->data(IdRole).toString() != o["bone_id"].toString();
    }
    if (rebuild) {
        m_model->removeRows(0, m_model->rowCount());
        for (const auto &value : tracks) {
            const auto o = value.toObject();
            auto *name = new QStandardItem(o["bone_name"].toString());
            name->setData(o["bone_id"].toString(), IdRole);
            name->setEditable(false);
            auto *eye = new QStandardItem;
            eye->setEditable(false);
            auto *lock = new QStandardItem;
            lock->setEditable(false);
            m_model->appendRow({name, eye, lock});
        }
    }
    const QSignalBlocker block(m_tree->selectionModel());
    m_tree->selectionModel()->clearSelection();
    const bool showBones = anim["show_bones"].toBool(true);
    for (int i = 0; i < tracks.size(); ++i) {
        const auto o = tracks[i].toObject();
        auto *name = m_model->item(i, Name);
        if (name->text() != o["bone_name"].toString())
            name->setText(o["bone_name"].toString());
        const bool selected = o["selected"].toBool();
        name->setData(selected, SelectedRole);
        name->setToolTip(selected ? tr("Osso selecionado") : tr("Osso do rig"));
        name->setIcon(PetuniaIcons::get(QStringLiteral("bone"), theme.accentPrimary().name()));

        auto *eye = m_model->item(i, Visible);
        eye->setIcon(PetuniaIcons::get(showBones ? QStringLiteral("eye") : QStringLiteral("eye_hidden"),
                                       (showBones ? theme.textSecondary() : theme.textMuted()).name()));
        eye->setTextAlignment(Qt::AlignCenter);
        eye->setToolTip(showBones ? tr("Ossos visíveis") : tr("Ossos ocultos"));
        eye->setData(showBones ? tr("Visível") : tr("Oculto"), Qt::AccessibleTextRole);

        auto *lock = m_model->item(i, Locked);
        lock->setIcon(PetuniaIcons::get(QStringLiteral("bone"), theme.textMuted().name()));
        lock->setTextAlignment(Qt::AlignCenter);
        lock->setToolTip(tr("Rig ativo"));

        const auto proxy = m_proxy->mapFromSource(m_model->index(i, Name));
        if (selected) {
            m_tree->selectionModel()->select(proxy, QItemSelectionModel::Select | QItemSelectionModel::Rows);
            m_tree->selectionModel()->setCurrentIndex(proxy, QItemSelectionModel::NoUpdate);
        }
    }
    m_syncing = false;
}

void PetuniaStructurePanel::syncScene(const QJsonObject &state) {
    const auto items = state["scene_items"].toArray();
    const bool isEmpty = items.isEmpty();
    m_empty->setTitle(tr("Cena vazia"));
    m_empty->setExplanation(tr("Nenhum objeto na cena. Crie uma primitiva para começar a modelar."));
    m_empty->setActionText(tr("Criar primitiva"));
    m_empty->setVisible(isEmpty);
    m_tree->setVisible(!isEmpty);
    m_search->setPlaceholderText(tr("Buscar partes"));
    const auto &theme = PetuniaTheme::instance();

    m_syncing = true;
    bool rebuild = m_model->rowCount() != items.size();
    for (int i = 0; !rebuild && i < items.size(); ++i)
        rebuild = m_model->item(i, Name)->data(IdRole).toString() != items[i].toObject()["id"].toString();
    if (rebuild) {
        m_model->removeRows(0, m_model->rowCount());
        for (const auto &value : items) {
            auto *name = new QStandardItem(value.toObject()["name"].toString());
            name->setData(value.toObject()["id"].toString(), IdRole);
            auto *eye = new QStandardItem;
            eye->setEditable(false);
            auto *lock = new QStandardItem;
            lock->setEditable(false);
            m_model->appendRow({name, eye, lock});
        }
    }
    const QSignalBlocker block(m_tree->selectionModel());
    m_tree->selectionModel()->clearSelection();
    for (int i = 0; i < items.size(); ++i) {
        const auto o = items[i].toObject();
        auto *name = m_model->item(i, Name);
        if (name->text() != o["name"].toString())
            name->setText(o["name"].toString());
        name->setData(o["selected"].toBool(), SelectedRole);
        name->setEditable(!o["locked"].toBool());
        name->setToolTip(o["active"].toBool() ? tr("Objeto ativo") : tr("Parte da cena"));
        QColor tint = theme.accentPrimary();
        const auto custom = o["custom_color_rgb"].toArray();
        if (o["has_custom_color"].toBool() && custom.size() == 3)
            tint = QColor(custom[0].toInt(), custom[1].toInt(), custom[2].toInt());
        name->setIcon(PetuniaIcons::get(QStringLiteral("cube"), tint.name()));

        auto *eye = m_model->item(i, Visible);
        const bool visible = o["visible"].toBool();
        eye->setIcon(PetuniaIcons::get(visible ? QStringLiteral("eye") : QStringLiteral("eye_hidden"),
                                       (visible ? theme.textSecondary() : theme.textMuted()).name()));
        eye->setTextAlignment(Qt::AlignCenter);
        eye->setToolTip(visible ? tr("Visível — clique para ocultar") : tr("Oculto — clique para exibir"));
        eye->setData(visible ? tr("Visível") : tr("Oculto"), Qt::AccessibleTextRole);

        auto *lock = m_model->item(i, Locked);
        const bool locked = o["locked"].toBool();
        lock->setIcon(PetuniaIcons::get(locked ? QStringLiteral("lock") : QStringLiteral("unlock"),
                                        (locked ? theme.textSecondary() : theme.textMuted()).name()));
        lock->setTextAlignment(Qt::AlignCenter);
        lock->setToolTip(locked ? tr("Bloqueado — clique para liberar") : tr("Livre — clique para bloquear"));
        lock->setData(locked ? tr("Bloqueado") : tr("Livre"), Qt::AccessibleTextRole);

        const auto proxy = m_proxy->mapFromSource(m_model->index(i, Name));
        if (o["selected"].toBool())
            m_tree->selectionModel()->select(proxy, QItemSelectionModel::Select | QItemSelectionModel::Rows);
        if (o["active"].toBool())
            m_tree->selectionModel()->setCurrentIndex(proxy, QItemSelectionModel::NoUpdate);
    }
    m_syncing = false;
}

void PetuniaStructurePanel::onContextMenu(const QPoint &pos) {
    const auto proxy = m_tree->indexAt(pos);
    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT")) {
        QMenu menu(this);
        auto *addLayer = menu.addAction(PetuniaIcons::get(QStringLiteral("plus")), tr("Nova camada"));
        auto *addGroup = menu.addAction(PetuniaIcons::get(QStringLiteral("folder")), tr("Novo grupo"));
        auto *addDecal = menu.addAction(PetuniaIcons::get(QStringLiteral("decal")), tr("Novo decalque"));
        menu.addSeparator();
        QAction *up = nullptr;
        QAction *down = nullptr;
        QAction *merge = nullptr;
        QAction *remove = nullptr;
        QAction *toggleVis = nullptr;
        QAction *toggleLock = nullptr;
        QString layerId;
        if (proxy.isValid()) {
            layerId = idAt(proxy);
            up = menu.addAction(tr("Mover para cima"));
            down = menu.addAction(tr("Mover para baixo"));
            merge = menu.addAction(tr("Mesclar abaixo"));
            toggleVis = menu.addAction(tr("Alternar visibilidade\tH"));
            toggleLock = menu.addAction(tr("Alternar bloqueio\tL"));
            menu.addSeparator();
            remove = menu.addAction(tr("Excluir camada\tDel"));
        }
        auto *picked = menu.exec(m_tree->viewport()->mapToGlobal(pos));
        if (!picked)
            return;
        if (picked == addLayer)
            m_controller->request(QStringLiteral("add_paint_layer"));
        else if (picked == addGroup)
            m_controller->request(QStringLiteral("add_paint_group"));
        else if (picked == addDecal)
            m_controller->request(QStringLiteral("add_decal_layer"));
        else if (picked == up)
            m_controller->request(QStringLiteral("move_paint_layer"), {{"id", layerId}, {"delta", -1}});
        else if (picked == down)
            m_controller->request(QStringLiteral("move_paint_layer"), {{"id", layerId}, {"delta", 1}});
        else if (picked == merge)
            m_controller->request(QStringLiteral("merge_down_paint_layer"), {{"id", layerId}});
        else if (picked == toggleVis)
            m_controller->request(QStringLiteral("toggle_paint_layer_visibility"), {{"id", layerId}});
        else if (picked == toggleLock)
            m_controller->request(QStringLiteral("toggle_paint_layer_lock"), {{"id", layerId}});
        else if (picked == remove)
            m_controller->request(QStringLiteral("remove_paint_layer"), {{"id", layerId}});
        return;
    }
    if (ws == QLatin1String("ANIMATE")) {
        QMenu menu(this);
        auto *addCreature = menu.addAction(PetuniaIcons::get(QStringLiteral("plus")), tr("Adicionar criatura…"));
        auto *fitModel = menu.addAction(tr("Ajustar ao modelo"));
        auto *toggleBones = menu.addAction(tr("Alternar visibilidade dos ossos\tH"));
        auto *picked = menu.exec(m_tree->viewport()->mapToGlobal(pos));
        if (!picked)
            return;
        if (picked == addCreature)
            m_controller->request(QStringLiteral("animate"), {{"id", "add-creature"}, {"arg", "Humanoid"}});
        else if (picked == fitModel)
            m_controller->request(QStringLiteral("animate"), {{"id", "fit-to-model"}});
        else if (picked == toggleBones)
            m_controller->request(QStringLiteral("animate"), {{"id", "toggle-bones"}});
        return;
    }

    if (!proxy.isValid())
        return;
    const QString id = idAt(proxy);
    if (!selectedIds().contains(id))
        m_controller->request(QStringLiteral("select"), {{"id", id}});
    QMenu menu(this);
    auto *rename = menu.addAction(tr("Renomear\tF2"));
    auto *duplicate = menu.addAction(tr("Duplicar\tCtrl+D"));
    auto *remove = menu.addAction(tr("Excluir\tDel"));
    menu.addSeparator();
    auto *visibility = menu.addAction(tr("Alternar visibilidade\tH"));
    auto *lock = menu.addAction(tr("Bloquear / desbloquear\tL"));
    auto *isolate = menu.addAction(tr("Isolar\tShift+H"));
    menu.addSeparator();
    auto *saveAsset = menu.addAction(tr("Salvar como asset"));
    auto *selectionColor = menu.addAction(tr("Cor de seleção da parte…"));
    auto *defaultColor = menu.addAction(tr("Usar a cor de seleção global"));
    menu.addSeparator();
    auto *up = menu.addAction(tr("Mover para cima"));
    auto *down = menu.addAction(tr("Mover para baixo"));
    menu.addSeparator();
    auto *alignMenu = menu.addMenu(tr("Alinhar e distribuir"));
    struct Entry { const char *label; const char *command; };
    const Entry entries[] = {
        {"Centralizar em X", "model.align_center_x"}, {"Centralizar em Y", "model.align_center_y"},
        {"Centralizar em Z", "model.align_center_z"}, {nullptr, nullptr},
        {"Esquerda (mín. X)", "model.align_min_x"}, {"Direita (máx. X)", "model.align_max_x"},
        {"Base (mín. Y)", "model.align_min_y"}, {"Topo (máx. Y)", "model.align_max_y"},
        {"Fundo (mín. Z)", "model.align_min_z"}, {"Frente (máx. Z)", "model.align_max_z"}, {nullptr, nullptr},
        {"Distribuir em X", "model.distribute_objects_x"}, {"Distribuir em Y", "model.distribute_objects_y"},
        {"Distribuir em Z", "model.distribute_objects_z"},
    };
    QHash<QAction *, QString> commands;
    for (const auto &entry : entries) {
        if (!entry.label) {
            alignMenu->addSeparator();
            continue;
        }
        commands.insert(alignMenu->addAction(tr(entry.label)), QLatin1String(entry.command));
    }

    auto *picked = menu.exec(m_tree->viewport()->mapToGlobal(pos));
    if (!picked)
        return;
    if (picked == rename)
        m_tree->edit(proxy.siblingAtColumn(Name));
    else if (picked == duplicate)
        m_controller->request(QStringLiteral("scene_context"), {{"id", id}, {"target", "duplicate"}});
    else if (picked == remove)
        m_controller->request(QStringLiteral("scene_context"), {{"id", id}, {"target", "delete"}});
    else if (picked == visibility)
        m_controller->request(QStringLiteral("visibility"), {{"id", id}});
    else if (picked == lock)
        m_controller->request(QStringLiteral("lock"), {{"id", id}});
    else if (picked == isolate)
        m_controller->request(QStringLiteral("isolate"));
    else if (picked == saveAsset)
        m_controller->request(QStringLiteral("save_asset"));
    else if (picked == selectionColor) {
        PetuniaColorPickerPopover picker(this);
        picker.setTitle(tr("Cor de seleção"));
        picker.setColor(PetuniaTheme::instance().accentSelection());
        if (picker.exec() == QDialog::Accepted) {
            const auto c = picker.color();
            m_controller->request(QStringLiteral("part_selection_color"),
                                  {{"id", id}, {"color", QJsonArray{c.red(), c.green(), c.blue()}}});
        }
    } else if (picked == defaultColor)
        m_controller->request(QStringLiteral("part_selection_color"), {{"id", id}, {"color", QJsonValue()}});
    else if (picked == up || picked == down)
        m_controller->request(QStringLiteral("scene_move"), {{"id", id}, {"delta", picked == up ? -1 : 1}});
    else if (commands.contains(picked))
        m_controller->command(commands.value(picked));
}
