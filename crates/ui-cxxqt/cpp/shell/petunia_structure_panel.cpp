#include "petunia_structure_panel.h"
#include "../petunia_icons.h"
#include "../petunia_color_picker.h"
#include "../petunia_placeholder_message.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QDropEvent>
#include <QHBoxLayout>
#include <QHash>
#include <QHeaderView>
#include <QItemSelectionModel>
#include <QJsonArray>
#include <QMenu>
#include <QShortcut>
#include <QSignalBlocker>
#include <QVBoxLayout>

#include <algorithm>
#include <functional>

namespace {
/// A lista mostra as camadas do topo para a base (`paint_layers` vem invertido),
/// mas `move_paint_layer` conta da base: descer na lista é descer na pilha.
int paintStackDelta(int listDelta) {
    return -listDelta;
}

QString groupItemId(const QString &name) {
    return QStringLiteral("group:") + name;
}

/// Filtro "só selecionadas": grupos ficam (a busca recursiva decide pelos filhos).
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
            const auto *item = model ? model->itemFromIndex(model->index(row, 0, parent)) : nullptr;
            if (item && item->data(Qt::UserRole + 2).toString() != QLatin1String("group") &&
                !item->data(Qt::UserRole + 1).toBool())
                return false;
        }
        return QSortFilterProxyModel::filterAcceptsRow(row, parent);
    }

  private:
    bool m_selectedOnly{false};
};

/// Árvore com arrastar e soltar controlado: o Qt nunca move linhas do modelo;
/// o painel traduz o gesto num pedido ao editor (`scene_drop`) e o estado novo
/// redesenha a lista. Mesma regra com o mouse e, via `dropOn`, nos testes.
class SceneTreeView : public QTreeView {
  public:
    using QTreeView::QTreeView;
    std::function<void(const QModelIndex &, PetuniaStructurePanel::DropPosition)> onDrop;

  protected:
    void dropEvent(QDropEvent *event) override {
        if (!onDrop) {
            QTreeView::dropEvent(event);
            return;
        }
        const QModelIndex target = indexAt(event->position().toPoint());
        using Where = PetuniaStructurePanel::DropPosition;
        Where where = Where::OnViewport;
        switch (dropIndicatorPosition()) {
        case QAbstractItemView::OnItem:
            where = Where::OnItem;
            break;
        case QAbstractItemView::AboveItem:
            where = Where::AboveItem;
            break;
        case QAbstractItemView::BelowItem:
            where = Where::BelowItem;
            break;
        case QAbstractItemView::OnViewport:
            where = Where::OnViewport;
            break;
        }
        onDrop(target, where);
        event->setDropAction(Qt::IgnoreAction);
        event->accept();
        stopAutoScroll();
        setState(QAbstractItemView::NoState);
        viewport()->update();
    }
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
    m_search->setMinimumHeight(theme.compactControlHeight());
    toolbar->addWidget(m_search, 1);

    auto iconButton = [&](const QString &icon, const QString &tip, bool checkable) {
        auto *button = new QPushButton(PetuniaIcons::get(icon), QString(), this);
        button->setProperty("role", QStringLiteral("quiet"));
        button->setCheckable(checkable);
        button->setToolTip(tip);
        button->setAccessibleName(tip);
        button->setFixedSize(theme.compactControlHeight(), theme.compactControlHeight());
        return button;
    };
    m_filterSelected = iconButton(QStringLiteral("filter"), tr("Mostrar só partes selecionadas"), true);
    m_sort = iconButton(QStringLiteral("sort_az"), tr("Ordenar por nome (A–Z)"), true);
    m_group = iconButton(QStringLiteral("collection"), tr("Agrupar seleção  (Ctrl+G)"), false);
    m_align = iconButton(QStringLiteral("bounding_box"), tr("Alinhar e distribuir…"), false);
    m_primary = iconButton(QStringLiteral("separate"), tr("Separar seleção em nova parte"), false);
    toolbar->addWidget(m_filterSelected);
    toolbar->addWidget(m_sort);
    toolbar->addWidget(m_group);
    toolbar->addWidget(m_align);
    toolbar->addWidget(m_primary);
    root->addLayout(toolbar);

    m_model = new QStandardItemModel(this);
    m_model->setColumnCount(4);
    m_model->setHorizontalHeaderLabels({tr("Nome"), tr("Visível"), tr("Bloqueio"), tr("Isolar")});
    m_model->invisibleRootItem()->setFlags(Qt::ItemIsEnabled | Qt::ItemIsDropEnabled);
    auto *proxy = new SelectedOnlyProxy(this);
    proxy->setSourceModel(m_model);
    proxy->setFilterCaseSensitivity(Qt::CaseInsensitive);
    proxy->setFilterKeyColumn(Name);
    proxy->setRecursiveFilteringEnabled(true);
    m_proxy = proxy;

    auto *tree = new SceneTreeView(this);
    m_tree = tree;
    m_tree->setModel(m_proxy);
    m_tree->setAccessibleName(tr("Partes da cena"));
    m_tree->setHeaderHidden(true);
    m_tree->setProperty("dense", true);
    m_tree->setUniformRowHeights(true);
    m_tree->setSelectionMode(QAbstractItemView::ExtendedSelection);
    m_tree->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_tree->setContextMenuPolicy(Qt::CustomContextMenu);
    m_tree->setIconSize(QSize(theme.iconPx(), theme.iconPx()));
    m_tree->setExpandsOnDoubleClick(false);
    m_tree->setAnimated(!theme.reducedMotion());
    m_tree->header()->setStretchLastSection(false);
    m_tree->header()->setSectionResizeMode(Name, QHeaderView::Stretch);
    for (const int column : {int(Visible), int(Locked), int(Isolate)}) {
        m_tree->header()->setSectionResizeMode(column, QHeaderView::Fixed);
        m_tree->header()->resizeSection(column, theme.compactControlHeight());
    }
    tree->onDrop = [this](const QModelIndex &target, DropPosition position) {
        dropOn(target, position);
    };
    configureTree(true);
    root->addWidget(m_tree, 1);

    m_empty = new PetuniaPlaceholderMessage(this);
    m_empty->setTitle(tr("Cena vazia"));
    m_empty->setExplanation(tr("Nenhum objeto na cena. Crie uma primitiva para começar a modelar."));
    m_empty->setActionText(tr("Criar primitiva"));
    root->addWidget(m_empty, 1);
    m_empty->hide();
    connect(m_empty, &PetuniaPlaceholderMessage::actionTriggered, this, &PetuniaStructurePanel::primaryAction);

    connect(m_search, &QLineEdit::textChanged, m_proxy, &QSortFilterProxyModel::setFilterFixedString);
    connect(m_filterSelected, &QPushButton::toggled, this,
            [proxy](bool on) { static_cast<SelectedOnlyProxy *>(proxy)->setSelectedOnly(on); });
    connect(m_sort, &QPushButton::toggled, this, [this](bool on) {
        if (on)
            m_proxy->sort(Name, Qt::AscendingOrder);
        else
            m_proxy->sort(-1);
    });
    connect(m_group, &QPushButton::clicked, this, [this] { m_controller->request(QStringLiteral("scene_group")); });
    connect(m_align, &QPushButton::clicked, this, [this] { emit alignRequested(m_align); });
    connect(m_primary, &QPushButton::clicked, this, [this] {
        const QString ws = m_controller->workspaceId();
        if (ws == QLatin1String("PAINT"))
            m_controller->request(QStringLiteral("add_paint_layer"));
        else if (ws == QLatin1String("ANIMATE"))
            m_controller->request(QStringLiteral("animate"), {{"id", "fit-to-model"}});
        else if (ws == QLatin1String("UV"))
            m_controller->command(QStringLiteral("uv.unwrap_auto"));
        else
            m_controller->command(QStringLiteral("model.separate_selection"));
    });
    connect(m_tree, &QTreeView::clicked, this, &PetuniaStructurePanel::onClicked);
    connect(m_tree, &QTreeView::customContextMenuRequested, this, &PetuniaStructurePanel::onContextMenu);
    connect(m_tree->selectionModel(), &QItemSelectionModel::selectionChanged, this,
            &PetuniaStructurePanel::onSelectionChanged);
    connect(m_tree, &QTreeView::collapsed, this, [this](const QModelIndex &index) {
        if (!m_syncing && kindAt(index) == QLatin1String("group"))
            m_collapsed.insert(groupAt(index));
    });
    connect(m_tree, &QTreeView::expanded, this, [this](const QModelIndex &index) {
        if (!m_syncing && kindAt(index) == QLatin1String("group"))
            m_collapsed.remove(groupAt(index));
    });
    connect(m_tree, &QTreeView::doubleClicked, this, [this](const QModelIndex &index) {
        // Como no Photoshop e no Blender, duplo clique no nome renomeia (grupo ou parte,
        // o mesmo que F2). Nas outras colunas de um grupo, recolhe/expande; a seta e as
        // teclas ←/→ também recolhem e expandem.
        if (!m_sceneMode || kindAt(index) != QLatin1String("group"))
            return;
        if (index.column() == Name)
            m_tree->edit(index);
        else
            m_tree->setExpanded(index.siblingAtColumn(Name), !m_tree->isExpanded(index.siblingAtColumn(Name)));
    });
    connect(m_model, &QStandardItemModel::itemChanged, this, [this](QStandardItem *item) {
        if (m_syncing || item->column() != Name)
            return;
        const QString ws = m_controller->workspaceId();
        if (ws == QLatin1String("PAINT") || ws == QLatin1String("ANIMATE") || ws == QLatin1String("UV"))
            return;
        if (item->data(KindRole).toString() == QLatin1String("group")) {
            m_controller->request(QStringLiteral("group_rename"),
                                  {{"from", item->data(GroupRole).toString()}, {"to", item->text()}});
            return;
        }
        m_controller->request(QStringLiteral("rename"),
                              {{"id", item->data(IdRole).toString()}, {"name", item->text()}});
    });
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaStructurePanel::refreshIcons);
    buildShortcuts();
}

void PetuniaStructurePanel::configureTree(bool scene) {
    m_sceneMode = scene;
    const auto &theme = PetuniaTheme::instance();
    m_tree->setRootIsDecorated(scene);
    m_tree->setIndentation(scene ? theme.space(4) : 0);
    m_tree->setColumnHidden(Isolate, !scene);
    m_tree->setDragEnabled(scene);
    m_tree->setAcceptDrops(scene);
    m_tree->setDropIndicatorShown(scene);
    m_tree->setDragDropMode(scene ? QAbstractItemView::InternalMove : QAbstractItemView::NoDragDrop);
    m_tree->setDefaultDropAction(Qt::MoveAction);
    m_group->setVisible(scene);
    m_align->setVisible(scene);
}

QIcon PetuniaStructurePanel::stateIcon(const QString &name, bool active, bool dim) const {
    const auto &theme = PetuniaTheme::instance();
    const QColor color = active ? theme.accentPrimary() : (dim ? theme.textMuted() : theme.textSecondary());
    return PetuniaIcons::get(name, color.name());
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
    auto modeling = [this] {
        const QString ws = m_controller->workspaceId();
        return ws != QLatin1String("PAINT") && ws != QLatin1String("ANIMATE") && ws != QLatin1String("UV");
    };
    // Mesmas teclas em todo workspace; o alvo é o item listado (parte, camada, osso).
    bind(QKeySequence(Qt::Key_Delete), [this, modeling] {
        if (modeling())
            m_controller->command(QStringLiteral("edit.delete"));
        else
            actOnCurrent(QStringLiteral("remove"));
    });
    bind(QKeySequence(Qt::Key_Backspace), [this, modeling] {
        if (modeling())
            m_controller->command(QStringLiteral("edit.delete"));
        else
            actOnCurrent(QStringLiteral("remove"));
    });
    bind(QKeySequence(Qt::SHIFT | Qt::Key_H), [this, modeling] {
        if (modeling())
            m_controller->request(QStringLiteral("isolate"));
    });
    bind(QKeySequence(Qt::Key_H), [this, eachSelected, modeling] {
        if (modeling())
            eachSelected(QStringLiteral("visibility"));
        else
            actOnCurrent(QStringLiteral("visibility"));
    });
    bind(QKeySequence(Qt::Key_L), [this, eachSelected, modeling] {
        if (modeling())
            eachSelected(QStringLiteral("lock"));
        else
            actOnCurrent(QStringLiteral("lock"));
    });
    // Agrupar e desagrupar a seleção (também funciona com a vista em foco).
    bind(QKeySequence(Qt::CTRL | Qt::Key_G), [this, modeling] {
        if (modeling())
            m_controller->request(QStringLiteral("scene_group"));
    });
    bind(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_G), [this, modeling] {
        if (modeling())
            m_controller->request(QStringLiteral("scene_ungroup"));
    });
    // Reordenar sem arrastar (alternativa de teclado ao drag-and-drop).
    bind(QKeySequence(Qt::ALT | Qt::Key_Up), [this] { actOnCurrent(QStringLiteral("move"), -1); });
    bind(QKeySequence(Qt::ALT | Qt::Key_Down), [this] { actOnCurrent(QStringLiteral("move"), 1); });
    bind(QKeySequence(Qt::Key_F2), [this] {
        const auto current = m_tree->currentIndex();
        if (current.isValid())
            m_tree->edit(current.siblingAtColumn(Name));
    });
    bind(QKeySequence(Qt::CTRL | Qt::Key_D),
         [eachSelected] { eachSelected(QStringLiteral("scene_context"), QStringLiteral("duplicate")); });
}

void PetuniaStructurePanel::actOnCurrent(const QString &verb, int delta) {
    const QString id = idAt(m_tree->currentIndex());
    if (id.isEmpty())
        return;
    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT")) {
        if (verb == QLatin1String("visibility"))
            m_controller->request(QStringLiteral("toggle_paint_layer_visibility"), {{"id", id}});
        else if (verb == QLatin1String("lock"))
            m_controller->request(QStringLiteral("toggle_paint_layer_lock"), {{"id", id}});
        else if (verb == QLatin1String("remove"))
            m_controller->request(QStringLiteral("remove_paint_layer"), {{"id", id}});
        else if (verb == QLatin1String("move"))
            m_controller->request(QStringLiteral("move_paint_layer"), {{"id", id}, {"delta", paintStackDelta(delta)}});
    } else if (ws == QLatin1String("ANIMATE")) {
        if (verb == QLatin1String("visibility"))
            m_controller->request(QStringLiteral("animate"), {{"id", "toggle-bones"}});
    } else if (ws != QLatin1String("UV") && verb == QLatin1String("move") &&
               kindAt(m_tree->currentIndex()) != QLatin1String("group")) {
        m_controller->request(QStringLiteral("scene_move"), {{"id", id}, {"delta", delta}});
    }
}

void PetuniaStructurePanel::primaryAction() {
    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT"))
        m_controller->request(QStringLiteral("add_paint_layer"));
    else if (ws == QLatin1String("ANIMATE"))
        m_controller->request(QStringLiteral("animate"), {{"id", "add-creature"}, {"arg", "Humanoid"}});
    else if (ws == QLatin1String("UV"))
        m_controller->command(QStringLiteral("uv.unwrap_auto"));
    else
        emit createRequested();
}

void PetuniaStructurePanel::updateChrome(const QString &workspace) {
    if (m_chromeWorkspace == workspace)
        return;
    m_chromeWorkspace = workspace;
    QString icon = QStringLiteral("separate");
    QString primary = tr("Separar seleção em nova parte");
    QString list = tr("Partes da cena");
    QString filter = tr("Mostrar só partes selecionadas");
    if (workspace == QLatin1String("PAINT")) {
        icon = QStringLiteral("plus");
        primary = tr("Nova camada");
        list = tr("Camadas de pintura");
        filter = tr("Mostrar só a camada ativa");
    } else if (workspace == QLatin1String("UV")) {
        icon = QStringLiteral("unwrap_auto");
        primary = tr("Desdobrar automaticamente");
        list = tr("Ilhas UV");
        filter = tr("Mostrar só ilhas selecionadas");
    } else if (workspace == QLatin1String("ANIMATE")) {
        icon = QStringLiteral("bone");
        primary = tr("Ajustar ao modelo");
        list = tr("Ossos da criatura");
        filter = tr("Mostrar só ossos selecionados");
    }
    m_primary->setIcon(PetuniaIcons::get(icon));
    m_primary->setToolTip(primary);
    m_primary->setAccessibleName(primary);
    m_filterSelected->setToolTip(filter);
    m_filterSelected->setAccessibleName(filter);
    m_tree->setAccessibleName(list);
    setAccessibleName(tr("Estrutura: %1").arg(list));
    const bool scene = workspace != QLatin1String("PAINT") && workspace != QLatin1String("ANIMATE") &&
                       workspace != QLatin1String("UV");
    configureTree(scene);
}

QStandardItem *PetuniaStructurePanel::itemAt(const QModelIndex &proxyIndex, int column) const {
    const auto source = m_proxy->mapToSource(proxyIndex);
    if (!source.isValid())
        return nullptr;
    return m_model->itemFromIndex(source.siblingAtColumn(column));
}

QString PetuniaStructurePanel::idAt(const QModelIndex &proxyIndex) const {
    const auto *item = itemAt(proxyIndex);
    return item ? item->data(IdRole).toString() : QString();
}

QString PetuniaStructurePanel::kindAt(const QModelIndex &proxyIndex) const {
    const auto *item = itemAt(proxyIndex);
    return item ? item->data(KindRole).toString() : QString();
}

QString PetuniaStructurePanel::groupAt(const QModelIndex &proxyIndex) const {
    const auto *item = itemAt(proxyIndex);
    return item ? item->data(GroupRole).toString() : QString();
}

QModelIndex PetuniaStructurePanel::indexOfId(const QString &id) const {
    QStandardItem *item = m_rowItems.value(id);
    if (!item)
        return {};
    return m_proxy->mapFromSource(item->index());
}

QStringList PetuniaStructurePanel::selectedIds() const {
    QStringList ids;
    auto add = [&ids](const QString &id) {
        if (!id.isEmpty() && !ids.contains(id))
            ids << id;
    };
    for (const auto &row : m_tree->selectionModel()->selectedRows(Name)) {
        if (kindAt(row) == QLatin1String("group")) {
            // Um grupo selecionado vale por todos os seus membros.
            for (int child = 0; child < m_proxy->rowCount(row); ++child)
                add(idAt(m_proxy->index(child, Name, row)));
        } else {
            add(idAt(row));
        }
    }
    return ids;
}

void PetuniaStructurePanel::onClicked(const QModelIndex &proxyIndex) {
    if (m_syncing)
        return;
    const auto source = m_proxy->mapToSource(proxyIndex);
    if (source.column() != Visible && source.column() != Locked && source.column() != Isolate)
        return;

    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT")) {
        const QString id = idAt(proxyIndex);
        if (source.column() == Visible)
            m_controller->request(QStringLiteral("toggle_paint_layer_visibility"), {{"id", id}});
        else if (source.column() == Locked)
            m_controller->request(QStringLiteral("toggle_paint_layer_lock"), {{"id", id}});
        return;
    }
    if (ws == QLatin1String("ANIMATE")) {
        if (source.column() == Visible)
            m_controller->request(QStringLiteral("animate"), {{"id", "toggle-bones"}});
        return;
    }
    if (ws == QLatin1String("UV"))
        return;

    // C7: linha de referência — o olho mostra e esconde a imagem, o cadeiro
    // trava; as demais operações ficam no painel de Referências.
    if (kindAt(proxyIndex) == QLatin1String("reference")) {
        const QString axis = groupAt(proxyIndex);
        if (source.column() == Visible)
            m_controller->request(QStringLiteral("toggle_reference_visible"), {{"axis_str", axis}});
        else if (source.column() == Locked)
            m_controller->request(QStringLiteral("toggle_reference_lock"), {{"axis_str", axis}});
        return;
    }

    // Linha de grupo: as três colunas agem no grupo inteiro.
    if (kindAt(proxyIndex) == QLatin1String("group")) {
        const QString group = groupAt(proxyIndex);
        if (source.column() == Visible) {
            m_controller->request(QStringLiteral("group_visibility"), {{"name", group}});
        } else if (source.column() == Locked) {
            m_controller->request(QStringLiteral("group_lock"), {{"name", group}});
        } else {
            m_controller->request(QStringLiteral("group_select"), {{"name", group}});
            m_controller->request(QStringLiteral("isolate"));
        }
        return;
    }

    if (source.column() == Isolate) {
        m_controller->request(QStringLiteral("isolate_part"), {{"id", idAt(proxyIndex)}});
        return;
    }
    const QString action = source.column() == Visible ? QStringLiteral("visibility") : QStringLiteral("lock");
    const auto rows = m_tree->selectionModel()->selectedRows(Name);
    // Clicar o ícone de uma linha dentro da seleção aplica a toda a seleção.
    if (rows.size() > 1 && rows.contains(proxyIndex.siblingAtColumn(Name))) {
        for (const auto &id : selectedIds())
            m_controller->request(action, {{"id", id}});
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
    if (ws == QLatin1String("UV")) {
        // Ids lidos antes: cada request devolve um snapshot que ressincroniza a lista.
        QList<int> islands;
        for (const auto &row : m_tree->selectionModel()->selectedRows(Name))
            islands << m_proxy->mapToSource(row).row();
        std::sort(islands.begin(), islands.end());
        bool extend = false;
        for (const int island : islands) {
            m_controller->request(QStringLiteral("uv_select_island"), {{"index", island}, {"extend", extend}});
            extend = true;
        }
        return;
    }

    QJsonArray ids;
    for (const auto &id : selectedIds())
        ids.append(id);
    QString active = idAt(m_tree->currentIndex());
    if (kindAt(m_tree->currentIndex()) == QLatin1String("group"))
        active = ids.isEmpty() ? QString() : ids.first().toString();
    m_controller->request(QStringLiteral("scene_selection"), {{"ids", ids}, {"active", active}});
}

void PetuniaStructurePanel::dropOn(const QModelIndex &proxyTarget, DropPosition position) {
    if (!m_sceneMode)
        return;
    const QStringList dragged = selectedIds();
    if (dragged.isEmpty())
        return;
    QJsonArray ids;
    for (const auto &id : dragged)
        ids.append(id);
    QJsonObject args{{"ids", ids}, {"before", QJsonValue()}, {"collection", QJsonValue()}};

    if (!proxyTarget.isValid() || position == DropPosition::OnViewport) {
        // Soltar no vazio: sai dos grupos e vai para o fim da lista.
    } else if (kindAt(proxyTarget) == QLatin1String("group")) {
        // Sobre o grupo: entra nele. Acima/abaixo do cabeçalho: sai dos grupos.
        if (position == DropPosition::OnItem)
            args["collection"] = groupAt(proxyTarget);
    } else {
        const QString targetId = idAt(proxyTarget);
        if (dragged.contains(targetId))
            return;
        QString before = targetId;
        if (position == DropPosition::BelowItem) {
            const auto next = m_proxy->index(proxyTarget.row() + 1, Name, proxyTarget.parent());
            before = next.isValid() ? idAt(next) : QString();
        }
        args["before"] = before.isEmpty() ? QJsonValue() : QJsonValue(before);
        // A parte adota o grupo do vizinho: soltar entre filhos de um grupo a inclui nele.
        const QString group = groupAt(proxyTarget);
        args["collection"] = group.isEmpty() ? QJsonValue() : QJsonValue(group);
    }
    m_controller->request(QStringLiteral("scene_drop"), args);
}

void PetuniaStructurePanel::refreshIcons() {
    m_tree->setIconSize(QSize(PetuniaTheme::instance().iconPx(), PetuniaTheme::instance().iconPx()));
    m_sceneSignature.clear();
    sync(m_controller->state());
}

void PetuniaStructurePanel::sync(const QJsonObject &state) {
    const QString ws = m_controller->workspaceId();
    updateChrome(ws);
    m_isolated = state["isolate_active"].toBool();
    if (ws == QLatin1String("PAINT"))
        syncPaint(state);
    else if (ws == QLatin1String("ANIMATE"))
        syncAnimate(state);
    else if (ws == QLatin1String("UV"))
        syncUv(state);
    else
        syncScene(state);
}

namespace {
/// Identificador da linha de uma referência ("reference:<eixo>").
QString referenceItemId(const QString &axis) {
    return QStringLiteral("reference:") + axis;
}

/// Referências com imagem carregada: são as que viram linhas nas Partes.
QJsonArray referenceSlotsWithImage(const QJsonObject &state) {
    QJsonArray out;
    for (const auto &value : state["reference_slots"].toArray()) {
        const auto slot = value.toObject();
        if (slot["has_image"].toBool())
            out.append(slot);
    }
    return out;
}

/// Itens de uma linha com as quatro colunas; só o nome é editável.
QList<QStandardItem *> makeRow(QStandardItem *name) {
    auto *eye = new QStandardItem;
    eye->setEditable(false);
    auto *lock = new QStandardItem;
    lock->setEditable(false);
    auto *isolate = new QStandardItem;
    isolate->setEditable(false);
    for (auto *item : {eye, lock, isolate})
        item->setFlags(Qt::ItemIsEnabled);
    return {name, eye, lock, isolate};
}
} // namespace

void PetuniaStructurePanel::syncUv(const QJsonObject &state) {
    const auto islands = state["uv_editor"].toObject()["islands"].toArray();
    const bool isEmpty = islands.isEmpty();
    m_empty->setTitle(tr("Sem ilhas UV"));
    m_empty->setExplanation(tr("O objeto ativo ainda não tem UVs. Desdobre para criar as ilhas."));
    m_empty->setActionText(tr("Desdobrar automaticamente"));
    m_empty->setVisible(isEmpty);
    m_tree->setVisible(!isEmpty);
    m_search->setPlaceholderText(tr("Buscar ilhas"));
    const auto &theme = PetuniaTheme::instance();

    m_syncing = true;
    m_sceneSignature.clear();
    // Ilhas não têm id persistente: a identidade é a posição + o tamanho.
    bool rebuild = m_model->rowCount() != islands.size();
    for (int i = 0; !rebuild && i < islands.size(); ++i)
        rebuild = m_model->item(i, Name)->data(IdRole).toString() != QStringLiteral("island:%1").arg(i);
    if (rebuild) {
        m_rowItems.clear();
        m_model->removeRows(0, m_model->rowCount());
        for (int i = 0; i < islands.size(); ++i) {
            auto *name = new QStandardItem;
            name->setData(QStringLiteral("island:%1").arg(i), IdRole);
            name->setData(QStringLiteral("island"), KindRole);
            name->setEditable(false);
            name->setIcon(PetuniaIcons::get(QStringLiteral("uv_island"), theme.accentPrimary().name()));
            m_model->appendRow(makeRow(name));
        }
    }
    const QSignalBlocker block(m_tree->selectionModel());
    m_tree->selectionModel()->clearSelection();
    for (int i = 0; i < islands.size(); ++i) {
        const auto o = islands[i].toObject();
        auto *name = m_model->item(i, Name);
        const int faces = o["face_count"].toInt();
        const QString label = tr("Ilha %1 · %2 faces").arg(i + 1).arg(faces);
        if (name->text() != label)
            name->setText(label);
        const double area = o["area"].toDouble() * 100.0;
        name->setToolTip(tr("%1% da área UV").arg(area, 0, 'f', 1));
        const bool selected = o["selected"].toBool();
        name->setData(selected, SelectedRole);
        if (selected) {
            const auto proxy = m_proxy->mapFromSource(m_model->index(i, Name));
            m_tree->selectionModel()->select(proxy, QItemSelectionModel::Select | QItemSelectionModel::Rows);
        }
    }
    m_syncing = false;
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
    m_sceneSignature.clear();
    bool rebuild = m_model->rowCount() != layers.size();
    for (int i = 0; !rebuild && i < layers.size(); ++i) {
        const auto o = layers[i].toObject();
        rebuild = m_model->item(i, Name)->data(IdRole).toString() != o["id"].toString();
    }
    if (rebuild) {
        m_rowItems.clear();
        m_model->removeRows(0, m_model->rowCount());
        for (const auto &value : layers) {
            const auto o = value.toObject();
            auto *name = new QStandardItem(o["name"].toString());
            name->setData(o["id"].toString(), IdRole);
            name->setData(QStringLiteral("layer"), KindRole);
            name->setEditable(false);
            m_model->appendRow(makeRow(name));
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
        QString iconName = QStringLiteral("paint_layer");
        if (isGroup)
            iconName = QStringLiteral("folder");
        else if (kind == QLatin1String("decal"))
            iconName = QStringLiteral("paint_decal");
        else if (kind == QLatin1String("effect"))
            iconName = QStringLiteral("tune");
        else if (kind == QLatin1String("gradient"))
            iconName = QStringLiteral("paint_gradient");
        name->setIcon(PetuniaIcons::get(iconName, tint.name()));
        QStringList badges;
        if (o["has_mask"].toBool())
            badges << tr("máscara");
        const int effectCount = o["effect_count"].toInt();
        if (effectCount > 0)
            badges << tr("%n efeito(s)", "", effectCount);
        const QString layerTip = active ? tr("Camada ativa") : tr("Camada de pintura");
        name->setToolTip(badges.isEmpty() ? layerTip : layerTip + QStringLiteral(" · ") + badges.join(QStringLiteral(" · ")));
        name->setData(badges.isEmpty() ? layerTip : layerTip + QStringLiteral(", ") + badges.join(QStringLiteral(", ")),
                      Qt::AccessibleDescriptionRole);

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
    m_sceneSignature.clear();
    bool rebuild = m_model->rowCount() != tracks.size();
    for (int i = 0; !rebuild && i < tracks.size(); ++i) {
        const auto o = tracks[i].toObject();
        rebuild = m_model->item(i, Name)->data(IdRole).toString() != o["bone_id"].toString();
    }
    if (rebuild) {
        m_rowItems.clear();
        m_model->removeRows(0, m_model->rowCount());
        for (const auto &value : tracks) {
            const auto o = value.toObject();
            auto *name = new QStandardItem(o["bone_name"].toString());
            name->setData(o["bone_id"].toString(), IdRole);
            name->setData(QStringLiteral("bone"), KindRole);
            name->setEditable(false);
            m_model->appendRow(makeRow(name));
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
    const auto groups = state["scene_groups"].toArray();
    const bool isEmpty = items.isEmpty();
    m_empty->setTitle(tr("Cena vazia"));
    m_empty->setExplanation(tr("Nenhum objeto na cena. Crie uma primitiva para começar a modelar."));
    m_empty->setActionText(tr("Criar primitiva"));
    m_empty->setVisible(isEmpty);
    m_tree->setVisible(!isEmpty);
    m_search->setPlaceholderText(tr("Buscar partes"));
    const auto &theme = PetuniaTheme::instance();

    // Estrutura da árvore: grupos na ordem do projeto, com seus membros na ordem
    // da cena; depois as partes soltas. A assinatura decide entre reconstruir e
    // só atualizar as linhas.
    QStringList groupOrder;
    for (const auto &value : groups)
        groupOrder << value.toObject()["name"].toString();
    // C7: o grupo "Referência" é sempre o primeiro da lista de Partes.
    const auto referenceSlots = referenceSlotsWithImage(state);
    QString signature;
    for (const auto &name : groupOrder)
        signature += QStringLiteral("g:") + name + QLatin1Char(';');
    for (const auto &value : items) {
        const auto o = value.toObject();
        signature += o["id"].toString() + QLatin1Char('@') + o["collection"].toString() + QLatin1Char(';');
    }
    for (const auto &value : referenceSlots) {
        const auto slot = value.toObject();
        signature += QStringLiteral("r:") + slot["axis"].toString() + QLatin1Char(';');
    }

    m_syncing = true;
    if (signature != m_sceneSignature) {
        m_sceneSignature = signature;
        m_rowItems.clear();
        m_model->removeRows(0, m_model->rowCount());
        QHash<QString, QStandardItem *> groupItems;
        // C7: "Referência" primeiro, sempre — as imagens de referência não são
        // partes da cena, mas precisam estar à mão no mesmo lugar.
        QStandardItem *referenceGroup = nullptr;
        if (!referenceSlots.isEmpty()) {
            auto *item = new QStandardItem(tr("Referência"));
            item->setData(QStringLiteral("reference-group"), IdRole);
            item->setData(QStringLiteral("reference_group"), KindRole);
            item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable);
            referenceGroup = item;
            m_model->appendRow(makeRow(item));
            m_rowItems.insert(QStringLiteral("reference-group"), item);
        }
        for (const auto &value : referenceSlots) {
            const auto slot = value.toObject();
            const QString axis = slot["axis"].toString();
            const QString label = slot["image_name"].toString().isEmpty()
                ? slot["title"].toString()
                : slot["image_name"].toString();
            auto *item = new QStandardItem(label);
            item->setData(referenceItemId(axis), IdRole);
            item->setData(QStringLiteral("reference"), KindRole);
            item->setData(axis, GroupRole);
            item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable);
            if (referenceGroup)
                referenceGroup->appendRow(makeRow(item));
            m_rowItems.insert(referenceItemId(axis), item);
        }
        for (const auto &name : groupOrder) {
            auto *item = new QStandardItem(name);
            item->setData(groupItemId(name), IdRole);
            item->setData(QStringLiteral("group"), KindRole);
            item->setData(name, GroupRole);
            item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsEditable | Qt::ItemIsDropEnabled);
            auto row = makeRow(item);
            m_model->appendRow(row);
            groupItems.insert(name, item);
            m_rowItems.insert(groupItemId(name), item);
        }
        for (const auto &value : items) {
            const auto o = value.toObject();
            auto *name = new QStandardItem(o["name"].toString());
            const QString group = o["collection"].toString();
            name->setData(o["id"].toString(), IdRole);
            name->setData(QStringLiteral("asset"), KindRole);
            name->setData(group, GroupRole);
            // Partes se arrastam; só grupos aceitam soltar "sobre" a linha.
            name->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsEditable | Qt::ItemIsDragEnabled);
            auto row = makeRow(name);
            if (auto *parent = groupItems.value(group))
                parent->appendRow(row);
            else
                m_model->appendRow(row);
            m_rowItems.insert(o["id"].toString(), name);
        }
        m_tree->expandAll();
        for (const auto &collapsed : std::as_const(m_collapsed)) {
            if (auto *item = m_rowItems.value(groupItemId(collapsed)))
                m_tree->setExpanded(m_proxy->mapFromSource(item->index()), false);
        }
    }

    // C7: as linhas de referência mostram o olho e o cadeiro do slot.
    for (const auto &value : referenceSlots) {
        const auto slot = value.toObject();
        auto *name = m_rowItems.value(referenceItemId(slot["axis"].toString()));
        if (!name)
            continue;
        const auto sourceName = name->index();
        auto *eye = m_model->itemFromIndex(sourceName.siblingAtColumn(Visible));
        auto *lock = m_model->itemFromIndex(sourceName.siblingAtColumn(Locked));
        if (eye) {
            const bool visible = slot["visible"].toBool();
            eye->setIcon(stateIcon(visible ? QStringLiteral("eye") : QStringLiteral("eye_hidden"), false, !visible));
            eye->setToolTip(visible ? tr("Visível — clique para ocultar") : tr("Oculto — clique para exibir"));
            eye->setData(visible ? tr("Visível") : tr("Oculto"), Qt::AccessibleTextRole);
        }
        if (lock) {
            const bool locked = slot["locked"].toBool();
            lock->setIcon(stateIcon(locked ? QStringLiteral("lock") : QStringLiteral("unlock"), false, !locked));
            lock->setToolTip(locked ? tr("Bloqueado — clique para liberar") : tr("Livre — clique para bloquear"));
            lock->setData(locked ? tr("Bloqueado") : tr("Livre"), Qt::AccessibleTextRole);
        }
    }

    const QSignalBlocker block(m_tree->selectionModel());
    m_tree->selectionModel()->clearSelection();
    QModelIndex current;
    for (const auto &value : items) {
        const auto o = value.toObject();
        auto *name = m_rowItems.value(o["id"].toString());
        if (!name)
            continue;
        const QModelIndex sourceName = name->index();
        auto *eye = m_model->itemFromIndex(sourceName.siblingAtColumn(Visible));
        auto *lock = m_model->itemFromIndex(sourceName.siblingAtColumn(Locked));
        auto *isolate = m_model->itemFromIndex(sourceName.siblingAtColumn(Isolate));
        if (name->text() != o["name"].toString())
            name->setText(o["name"].toString());
        name->setData(o["selected"].toBool(), SelectedRole);
        name->setEditable(!o["locked"].toBool());
        name->setToolTip(o["active"].toBool() ? tr("Objeto ativo") : tr("Parte da cena"));
        // O ícone leva a cor de seleção da parte: é a mesma do contorno na vista.
        QColor tint = theme.accentPrimary();
        const auto custom = o["custom_color_rgb"].toArray();
        if (o["has_custom_color"].toBool() && custom.size() == 3)
            tint = QColor(custom[0].toInt(), custom[1].toInt(), custom[2].toInt());
        name->setIcon(PetuniaIcons::get(QStringLiteral("cube"), tint.name()));

        const bool visible = o["visible"].toBool();
        eye->setIcon(stateIcon(visible ? QStringLiteral("eye") : QStringLiteral("eye_hidden"), false, !visible));
        eye->setTextAlignment(Qt::AlignCenter);
        eye->setToolTip(visible ? tr("Visível — clique para ocultar") : tr("Oculto — clique para exibir"));
        eye->setData(visible ? tr("Visível") : tr("Oculto"), Qt::AccessibleTextRole);

        const bool locked = o["locked"].toBool();
        lock->setIcon(stateIcon(locked ? QStringLiteral("lock") : QStringLiteral("unlock"), false, !locked));
        lock->setTextAlignment(Qt::AlignCenter);
        lock->setToolTip(locked ? tr("Bloqueado — clique para liberar") : tr("Livre — clique para bloquear"));
        lock->setData(locked ? tr("Bloqueado") : tr("Livre"), Qt::AccessibleTextRole);

        // Isolamento: mostra só esta parte (e a seleção). Ligado, as partes que
        // ficaram na cena levam o ícone em destaque.
        const bool isolated = m_isolated && visible;
        isolate->setIcon(stateIcon(QStringLiteral("isolate"), isolated, !isolated));
        isolate->setTextAlignment(Qt::AlignCenter);
        isolate->setToolTip(m_isolated ? tr("Sair do isolamento  (Shift+H)")
                                       : tr("Isolar: mostrar só esta parte  (Shift+H)"));
        isolate->setData(isolated ? tr("Isolado") : tr("Não isolado"), Qt::AccessibleTextRole);

        const auto proxy = m_proxy->mapFromSource(sourceName);
        if (o["selected"].toBool())
            m_tree->selectionModel()->select(proxy, QItemSelectionModel::Select | QItemSelectionModel::Rows);
        if (o["active"].toBool())
            current = proxy;
    }
    for (const auto &value : groups) {
        const auto g = value.toObject();
        auto *name = m_rowItems.value(groupItemId(g["name"].toString()));
        if (!name)
            continue;
        const QModelIndex sourceName = name->index();
        auto *eye = m_model->itemFromIndex(sourceName.siblingAtColumn(Visible));
        auto *lock = m_model->itemFromIndex(sourceName.siblingAtColumn(Locked));
        auto *isolate = m_model->itemFromIndex(sourceName.siblingAtColumn(Isolate));
        const QString label = g["name"].toString();
        if (name->text() != label)
            name->setText(label);
        name->setData(g["selected"].toBool(), SelectedRole);
        const bool hasColor = g["has_color"].toBool();
        const auto rgb = g["color_rgb"].toArray();
        const QColor groupColor = hasColor && rgb.size() == 3 ? QColor(rgb[0].toInt(), rgb[1].toInt(), rgb[2].toInt())
                                                              : theme.accentPrimary();
        name->setIcon(PetuniaIcons::get(QStringLiteral("collection"), groupColor.name()));
        name->setToolTip(hasColor ? tr("Grupo com %1 parte(s) · cor de seleção %2%3")
                                        .arg(g["count"].toInt())
                                        .arg(groupColor.name().toUpper())
                                        .arg(g["force_color"].toBool() ? tr(" (vence a cor da parte)") : QString())
                                  : tr("Grupo com %1 parte(s)").arg(g["count"].toInt()));

        const bool visible = g["visible"].toBool();
        eye->setIcon(stateIcon(visible ? QStringLiteral("eye") : QStringLiteral("eye_hidden"), false, !visible));
        eye->setTextAlignment(Qt::AlignCenter);
        eye->setToolTip(visible ? tr("Grupo visível — clique para ocultar") : tr("Grupo oculto — clique para exibir"));
        eye->setData(visible ? tr("Visível") : tr("Oculto"), Qt::AccessibleTextRole);
        const bool locked = g["locked"].toBool();
        lock->setIcon(stateIcon(locked ? QStringLiteral("lock") : QStringLiteral("unlock"), false, !locked));
        lock->setTextAlignment(Qt::AlignCenter);
        lock->setToolTip(locked ? tr("Grupo bloqueado — clique para liberar") : tr("Grupo livre — clique para bloquear"));
        lock->setData(locked ? tr("Bloqueado") : tr("Livre"), Qt::AccessibleTextRole);
        isolate->setIcon(stateIcon(QStringLiteral("isolate"), false, true));
        isolate->setTextAlignment(Qt::AlignCenter);
        isolate->setToolTip(tr("Isolar o grupo"));
        isolate->setData(tr("Não isolado"), Qt::AccessibleTextRole);

        if (g["selected"].toBool())
            m_tree->selectionModel()->select(m_proxy->mapFromSource(sourceName),
                                             QItemSelectionModel::Select | QItemSelectionModel::Rows);
    }
    if (current.isValid())
        m_tree->selectionModel()->setCurrentIndex(current, QItemSelectionModel::NoUpdate);
    m_syncing = false;
}

void PetuniaStructurePanel::showGroupMenu(const QString &group, const QPoint &globalPos) {
    QMenu menu(this);
    auto *select = menu.addAction(PetuniaIcons::get(QStringLiteral("select_all")), tr("Selecionar os membros"));
    auto *rename = menu.addAction(tr("Renomear grupo\tF2"));
    menu.addSeparator();
    // Cor de seleção do grupo: a da parte vence, a menos que o grupo a force.
    QJsonObject groupState;
    for (const auto &value : m_controller->state()["scene_groups"].toArray())
        if (value.toObject()["name"].toString() == group)
            groupState = value.toObject();
    const bool groupHasColor = groupState["has_color"].toBool();
    const bool groupForces = groupState["force_color"].toBool();
    auto *groupColor = menu.addAction(tr("Cor de seleção do grupo…"));
    auto *forceColor = menu.addAction(tr("Cor do grupo vence a cor da parte"));
    forceColor->setCheckable(true);
    forceColor->setChecked(groupForces);
    forceColor->setEnabled(groupHasColor);
    auto *clearColor = menu.addAction(tr("Remover a cor do grupo"));
    clearColor->setEnabled(groupHasColor);
    menu.addSeparator();
    auto *visibility = menu.addAction(tr("Alternar visibilidade do grupo"));
    auto *lock = menu.addAction(tr("Bloquear / desbloquear o grupo"));
    auto *isolate = menu.addAction(PetuniaIcons::get(QStringLiteral("isolate")), tr("Isolar o grupo"));
    menu.addSeparator();
    auto *ungroup = menu.addAction(tr("Desagrupar\tCtrl+Shift+G"));
    auto *picked = menu.exec(globalPos);
    if (!picked)
        return;
    if (picked == select)
        m_controller->request(QStringLiteral("group_select"), {{"name", group}});
    else if (picked == rename) {
        const auto index = indexOfId(groupItemId(group));
        if (index.isValid())
            m_tree->edit(index.siblingAtColumn(Name));
    } else if (picked == groupColor) {
        PetuniaColorPickerPopover picker(this);
        picker.setTitle(tr("Cor de seleção do grupo"));
        const auto rgb = groupState["color_rgb"].toArray();
        picker.setColor(rgb.size() == 3 ? QColor(rgb[0].toInt(), rgb[1].toInt(), rgb[2].toInt())
                                        : PetuniaTheme::instance().accentSelection());
        if (picker.exec() == QDialog::Accepted) {
            const auto c = picker.color();
            m_controller->request(QStringLiteral("group_color"),
                                  {{"name", group}, {"color", QJsonArray{c.red(), c.green(), c.blue()}},
                                   {"force", groupForces}});
        }
    } else if (picked == forceColor) {
        const auto rgb = groupState["color_rgb"].toArray();
        m_controller->request(QStringLiteral("group_color"),
                              {{"name", group}, {"color", rgb}, {"force", forceColor->isChecked()}});
    } else if (picked == clearColor)
        m_controller->request(QStringLiteral("group_color"),
                              {{"name", group}, {"color", QJsonValue()}, {"force", false}});
    else if (picked == visibility)
        m_controller->request(QStringLiteral("group_visibility"), {{"name", group}});
    else if (picked == lock)
        m_controller->request(QStringLiteral("group_lock"), {{"name", group}});
    else if (picked == isolate) {
        m_controller->request(QStringLiteral("group_select"), {{"name", group}});
        m_controller->request(QStringLiteral("isolate"));
    } else if (picked == ungroup)
        m_controller->request(QStringLiteral("scene_ungroup"), {{"name", group}});
}

void PetuniaStructurePanel::onContextMenu(const QPoint &pos) {
    const auto proxy = m_tree->indexAt(pos);
    const QString ws = m_controller->workspaceId();
    if (ws == QLatin1String("PAINT")) {
        QMenu menu(this);
        auto *addLayer = menu.addAction(PetuniaIcons::get(QStringLiteral("paint_layer")), tr("Nova camada"));
        auto *addGroup = menu.addAction(PetuniaIcons::get(QStringLiteral("folder")), tr("Novo grupo"));
        auto *addGradient = menu.addAction(PetuniaIcons::get(QStringLiteral("paint_gradient")), tr("Nova camada de gradiente"));
        auto *addDecal = menu.addAction(PetuniaIcons::get(QStringLiteral("paint_decal")), tr("Novo decalque"));
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
            up = menu.addAction(tr("Mover para cima\tAlt+↑"));
            down = menu.addAction(tr("Mover para baixo\tAlt+↓"));
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
        else if (picked == addGradient)
            m_controller->request(QStringLiteral("add_gradient_layer"), {{"kind", "linear"}});
        else if (picked == addDecal)
            m_controller->request(QStringLiteral("add_decal_layer"));
        else if (picked == up)
            m_controller->request(QStringLiteral("move_paint_layer"), {{"id", layerId}, {"delta", paintStackDelta(-1)}});
        else if (picked == down)
            m_controller->request(QStringLiteral("move_paint_layer"), {{"id", layerId}, {"delta", paintStackDelta(1)}});
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

    if (ws == QLatin1String("UV")) {
        QMenu menu(this);
        const struct { const char *label; const char *command; } entries[] = {
            {QT_TR_NOOP("Desdobrar automaticamente"), "uv.unwrap_auto"},
            {QT_TR_NOOP("Empacotar ilhas"), "uv.pack_islands"},
            {QT_TR_NOOP("Relaxar"), "uv.relax"},
            {QT_TR_NOOP("Costurar (stitch)"), "uv.stitch"},
            {QT_TR_NOOP("Separar ilha"), "uv.split"},
        };
        QHash<QAction *, QString> commands;
        for (const auto &entry : entries)
            commands.insert(menu.addAction(tr(entry.label)), QLatin1String(entry.command));
        auto *picked = menu.exec(m_tree->viewport()->mapToGlobal(pos));
        if (picked && commands.contains(picked))
            m_controller->command(commands.value(picked));
        return;
    }

    // Cena: linha de grupo, de parte ou área vazia.
    if (proxy.isValid() && kindAt(proxy) == QLatin1String("group")) {
        showGroupMenu(groupAt(proxy), m_tree->viewport()->mapToGlobal(pos));
        return;
    }
    // C7: na referência, o menu do botão direito apaga a imagem.
    if (proxy.isValid() && kindAt(proxy) == QLatin1String("reference")) {
        const QString axis = groupAt(proxy);
        QMenu menu(this);
        auto *remove = menu.addAction(tr("Apagar referência"));
        auto *picked = menu.exec(m_tree->viewport()->mapToGlobal(pos));
        if (picked == remove)
            m_controller->request(QStringLiteral("remove_reference_slot"), {{"axis_str", axis}});
        return;
    }
    if (!proxy.isValid()) {
        QMenu menu(this);
        auto *create = menu.addAction(PetuniaIcons::get(QStringLiteral("plus")), tr("Criar primitiva"));
        auto *picked = menu.exec(m_tree->viewport()->mapToGlobal(pos));
        if (picked == create)
            emit createRequested();
        return;
    }
    const QString id = idAt(proxy);
    if (!selectedIds().contains(id))
        m_controller->request(QStringLiteral("select"), {{"id", id}});
    const int selectedCount = int(selectedIds().size());
    QMenu menu(this);
    auto *rename = menu.addAction(tr("Renomear\tF2"));
    auto *duplicate = menu.addAction(tr("Duplicar\tCtrl+D"));
    auto *remove = menu.addAction(tr("Excluir\tDel"));
    menu.addSeparator();
    auto *visibility = menu.addAction(tr("Alternar visibilidade\tH"));
    auto *lock = menu.addAction(tr("Bloquear / desbloquear\tL"));
    auto *isolate = menu.addAction(PetuniaIcons::get(QStringLiteral("isolate")),
                                   m_isolated ? tr("Sair do isolamento\tShift+H") : tr("Isolar\tShift+H"));
    menu.addSeparator();
    auto *group = menu.addAction(PetuniaIcons::get(QStringLiteral("collection")), tr("Agrupar\tCtrl+G"));
    auto *ungroup = menu.addAction(tr("Desagrupar\tCtrl+Shift+G"));
    ungroup->setEnabled(!groupAt(proxy).isEmpty());
    menu.addSeparator();
    auto *align = menu.addAction(PetuniaIcons::get(QStringLiteral("bounding_box")), tr("Alinhar e distribuir…"));
    align->setEnabled(selectedCount >= 2);
    if (selectedCount < 2)
        align->setToolTip(tr("Selecione 2 ou mais partes"));
    menu.addSeparator();
    auto *saveAsset = menu.addAction(tr("Salvar como asset"));
    auto *selectionColor = menu.addAction(tr("Cor de seleção da parte…"));
    auto *defaultColor = menu.addAction(tr("Usar a cor de seleção global"));
    menu.addSeparator();
    auto *up = menu.addAction(tr("Mover para cima\tAlt+↑"));
    auto *down = menu.addAction(tr("Mover para baixo\tAlt+↓"));

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
    else if (picked == group)
        m_controller->request(QStringLiteral("scene_group"));
    else if (picked == ungroup)
        m_controller->request(QStringLiteral("scene_ungroup"));
    else if (picked == align)
        emit alignRequested(m_align);
    else if (picked == saveAsset)
        m_controller->request(QStringLiteral("save_asset"));
    else if (picked == selectionColor) {
        PetuniaColorPickerPopover picker(this);
        picker.setTitle(tr("Cor de seleção"));
        const auto *item = itemAt(proxy);
        // O seletor abre na cor atual da parte (a mesma do contorno na vista).
        QColor start = PetuniaTheme::instance().accentSelection();
        for (const auto &value : m_controller->state()["scene_items"].toArray()) {
            const auto o = value.toObject();
            if (item && o["id"].toString() == item->data(IdRole).toString()) {
                const auto rgb = o["custom_color_rgb"].toArray();
                if (rgb.size() == 3)
                    start = QColor(rgb[0].toInt(), rgb[1].toInt(), rgb[2].toInt());
            }
        }
        picker.setColor(start);
        if (picker.exec() == QDialog::Accepted) {
            const auto c = picker.color();
            m_controller->request(QStringLiteral("part_selection_color"),
                                  {{"id", id}, {"color", QJsonArray{c.red(), c.green(), c.blue()}}});
        }
    } else if (picked == defaultColor)
        m_controller->request(QStringLiteral("part_selection_color"), {{"id", id}, {"color", QJsonValue()}});
    else if (picked == up || picked == down)
        m_controller->request(QStringLiteral("scene_move"), {{"id", id}, {"delta", picked == up ? -1 : 1}});
}
