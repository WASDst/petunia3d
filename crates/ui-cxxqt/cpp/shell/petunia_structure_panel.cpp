#include "petunia_structure_panel.h"
#include "../petunia_icons.h"
#include "../petunia_overlay_cards.h"
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
            invalidateFilter();
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
    connect(separate, &QPushButton::clicked, this,
            [this] { m_controller->command(QStringLiteral("model.separate_selection")); });
    connect(m_tree, &QTreeView::clicked, this, &PetuniaStructurePanel::onClicked);
    connect(m_tree, &QTreeView::customContextMenuRequested, this, &PetuniaStructurePanel::onContextMenu);
    connect(m_tree->selectionModel(), &QItemSelectionModel::selectionChanged, this,
            &PetuniaStructurePanel::onSelectionChanged);
    connect(m_model, &QStandardItemModel::itemChanged, this, [this](QStandardItem *item) {
        if (m_syncing || item->column() != Name)
            return;
        m_controller->request(QStringLiteral("rename"),
                              {{"id", item->data(IdRole).toString()}, {"name", item->text()}});
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
    const auto items = state["scene_items"].toArray();
    const bool isEmpty = items.isEmpty();
    m_empty->setVisible(isEmpty);
    m_tree->setVisible(!isEmpty);
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
