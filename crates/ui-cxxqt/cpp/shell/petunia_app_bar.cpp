#include "petunia_app_bar.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"
#include "petunia_segmented.h"

#include <QFileInfo>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QLabel>
#include <QMenu>
#include <QPushButton>
#include <QResizeEvent>

PetuniaAppBar::PetuniaAppBar(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaAppBar"));
    setAccessibleName(tr("Barra do aplicativo"));
    m_layout = new QHBoxLayout(this);

    m_logo = new QLabel(this);
    m_logo->setAccessibleName(QStringLiteral("Petunia3D"));
    m_layout->addWidget(m_logo);

    m_menuBar = new QMenuBar(this);
    m_menuBar->setObjectName(QStringLiteral("PetuniaMenuBar"));
    m_menuBar->setNativeMenuBar(false);
    m_menuBar->setAccessibleName(tr("Menus"));
    m_layout->addWidget(m_menuBar);

    m_project = new QLabel(this);
    m_project->setProperty("role", QStringLiteral("caption"));
    m_project->setAccessibleName(tr("Projeto"));
    m_layout->addWidget(m_project);
    m_layout->addStretch(1);

    m_workspaces = new PetuniaSegmented(this);
    m_workspaces->setItems({{"DRAW", QStringLiteral("DRAW"), tr("Workspace DRAW — desenho de formas"), {}},
                            {"POLY", QStringLiteral("POLY"), tr("Workspace POLY — modelagem poligonal"), {}},
                            {"PAINT", QStringLiteral("PAINT"), tr("Workspace PAINT — pintura"), {}},
                            {"UV", QStringLiteral("UV"), tr("Workspace UV — mapeamento"), {}},
                            {"ANIMATE", QStringLiteral("ANIMATE"), tr("Workspace ANIMATE — animação"), {}}});
    m_workspaces->setGroupLabel(tr("Workspaces"));
    connect(m_workspaces, &PetuniaSegmented::activated, this, [this](const QString &id) {
        m_controller->request(QStringLiteral("workspace"), {{"id", id}});
    });
    m_layout->addWidget(m_workspaces);
    m_layout->addStretch(1);

    auto iconButton = [this](const QString &icon, const QString &tip) {
        auto *button = new QToolButton(this);
        button->setProperty("role", QStringLiteral("header"));
        button->setIcon(PetuniaIcons::get(icon));
        button->setToolTip(tip);
        button->setAccessibleName(tip);
        return button;
    };
    m_undo = iconButton(QStringLiteral("undo"), tr("Desfazer"));
    m_redo = iconButton(QStringLiteral("redo"), tr("Refazer"));
    connect(m_undo, &QToolButton::clicked, this, [this] { m_controller->request(QStringLiteral("undo")); });
    connect(m_redo, &QToolButton::clicked, this, [this] { m_controller->request(QStringLiteral("redo")); });
    m_layout->addWidget(m_undo);
    m_layout->addWidget(m_redo);

    m_search = new QPushButton(PetuniaIcons::get(QStringLiteral("search")), tr("Buscar comando"), this);
    m_search->setToolTip(tr("Paleta de comandos"));
    m_search->setAccessibleName(tr("Paleta de comandos"));
    connect(m_search, &QPushButton::clicked, this, [this] { emit localAction(QStringLiteral("app.palette")); });
    m_layout->addWidget(m_search);

    m_settings = iconButton(QStringLiteral("settings"), tr("Preferências"));
    connect(m_settings, &QToolButton::clicked, this, [this] { emit localAction(QStringLiteral("app.preferences")); });
    m_layout->addWidget(m_settings);

    m_hamburger = new QToolButton(this);
    m_hamburger->setProperty("role", QStringLiteral("header"));
    m_hamburger->setIcon(PetuniaIcons::get(QStringLiteral("list_view")));
    m_hamburger->setToolTip(tr("Menus"));
    m_hamburger->setAccessibleName(tr("Menus"));
    m_hamburger->setPopupMode(QToolButton::InstantPopup);
    m_layout->insertWidget(1, m_hamburger);
    m_hamburger->hide();

    buildMenus();
    auto *all = new QMenu(m_hamburger);
    for (auto *menu : m_menus)
        all->addMenu(menu);
    m_hamburger->setMenu(all);
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaAppBar::applyMetrics);
    applyMetrics();
}

QAction *PetuniaAppBar::addLocal(QMenu *menu, const QString &text, const QString &id, const QString &keymap) {
    auto *action = menu->addAction(text);
    // O id viaja com a ação: a automação (e os testes) acham a entrada sem
    // depender do idioma do rótulo.
    action->setData(id);
    connect(action, &QAction::triggered, this, [this, id] { emit localAction(id); });
    m_tracked.append({action, text, keymap});
    return action;
}

QAction *PetuniaAppBar::addCommand(QMenu *menu, const QString &text, const QString &command, const QString &keymap) {
    auto *action = menu->addAction(text);
    connect(action, &QAction::triggered, this, [this, command] { m_controller->command(command); });
    m_tracked.append({action, text, keymap});
    return action;
}

QAction *PetuniaAppBar::addRequest(QMenu *menu, const QString &text, const QString &request, const QString &keymap) {
    auto *action = menu->addAction(text);
    connect(action, &QAction::triggered, this, [this, request] { m_controller->request(request); });
    m_tracked.append({action, text, keymap});
    return action;
}

QMenu *PetuniaAppBar::newMenu(const QString &title) {
    auto *menu = new QMenu(title, this);
    m_menus.append(menu);
    m_menuBar->addMenu(menu);
    return menu;
}

void PetuniaAppBar::buildMenus() {
    auto *file = newMenu(tr("&Arquivo"));
    addLocal(file, tr("Novo projeto"), QStringLiteral("file.new"), QStringLiteral("global.new_project"));
    addLocal(file, tr("Nova cena vazia"), QStringLiteral("file.new_empty"));
    addLocal(file, tr("Abrir…"), QStringLiteral("file.open"), QStringLiteral("global.open_project"));
    file->addSeparator();
    auto *importMenu = file->addMenu(tr("Importar"));
    addLocal(importMenu, tr("Modelo 3D (OBJ, glTF, GLB)…"), QStringLiteral("file.import_model"));
    addLocal(importMenu, tr("Paleta de cores (.gpl, .hex)…"), QStringLiteral("file.import_palette"));
    file->addSeparator();
    addLocal(file, tr("Salvar"), QStringLiteral("file.save"), QStringLiteral("global.save_project"));
    addLocal(file, tr("Salvar como…"), QStringLiteral("file.save_as"));
    file->addSeparator();
    auto *exportMenu = file->addMenu(tr("Exportar"));
    addLocal(exportMenu, tr("glTF binário (.glb)…"), QStringLiteral("file.export_glb"));
    addLocal(exportMenu, tr("Wavefront OBJ (.obj)…"), QStringLiteral("file.export_obj"));

    auto *edit = newMenu(tr("&Editar"));
    addRequest(edit, tr("Desfazer"), QStringLiteral("undo"), QStringLiteral("global.undo"));
    addRequest(edit, tr("Refazer"), QStringLiteral("redo"), QStringLiteral("global.redo"));
    edit->addSeparator();
    addCommand(edit, tr("Duplicar"), QStringLiteral("model.duplicate"), QStringLiteral("model.duplicate"));
    addCommand(edit, tr("Excluir"), QStringLiteral("edit.delete"), QStringLiteral("model.delete"));
    edit->addSeparator();
    addLocal(edit, tr("Paleta de comandos…"), QStringLiteral("app.palette"), QStringLiteral("global.command_palette"));
    addLocal(edit, tr("Preferências…"), QStringLiteral("app.preferences"));

    auto *select = newMenu(tr("&Seleção"));
    addCommand(select, tr("Selecionar tudo"), QStringLiteral("select.all"));
    addCommand(select, tr("Limpar seleção"), QStringLiteral("select.clear"));
    addCommand(select, tr("Inverter seleção"), QStringLiteral("select.invert"), QStringLiteral("model.invert_selection"));

    m_addMenu = newMenu(tr("&Adicionar"));

    auto *object = newMenu(tr("&Objeto"));
    auto *align = object->addMenu(tr("Alinhar"));
    struct Entry { const char *label; const char *command; };
    const Entry aligns[] = {
        {"Centralizar em X", "model.align_center_x"}, {"Centralizar em Y", "model.align_center_y"},
        {"Centralizar em Z", "model.align_center_z"}, {nullptr, nullptr},
        {"Esquerda (mín. X)", "model.align_min_x"}, {"Direita (máx. X)", "model.align_max_x"},
        {"Base (mín. Y)", "model.align_min_y"}, {"Topo (máx. Y)", "model.align_max_y"},
        {"Fundo (mín. Z)", "model.align_min_z"}, {"Frente (máx. Z)", "model.align_max_z"},
    };
    for (const auto &entry : aligns) {
        if (!entry.label)
            align->addSeparator();
        else
            addCommand(align, tr(entry.label), QLatin1String(entry.command));
    }
    auto *distribute = object->addMenu(tr("Distribuir"));
    addCommand(distribute, tr("Distribuir em X"), QStringLiteral("model.distribute_objects_x"));
    addCommand(distribute, tr("Distribuir em Y"), QStringLiteral("model.distribute_objects_y"));
    addCommand(distribute, tr("Distribuir em Z"), QStringLiteral("model.distribute_objects_z"));

    auto *tools = newMenu(tr("&Ferramentas"));
    addLocal(tools, tr("Referências…"), QStringLiteral("app.references"), QStringLiteral("window.reference_manager"));
    addLocal(tools, tr("Biblioteca de assets…"), QStringLiteral("app.assets"));

    auto *view = newMenu(tr("E&xibir"));
    addLocal(view, tr("Mostrar nomes das ferramentas"), QStringLiteral("panel.rail_labels"));
    addLocal(view, tr("Estrutura e propriedades"), QStringLiteral("panel.right"));
    addLocal(view, tr("Gaveta inferior"), QStringLiteral("panel.drawer"));
    view->addSeparator();
    addLocal(view, tr("Restaurar layout deste workspace"), QStringLiteral("layout.reset"));
    addLocal(view, tr("Restaurar todos os layouts"), QStringLiteral("layout.reset_all"));

    auto *help = newMenu(tr("A&juda"));
    addLocal(help, tr("Início…"), QStringLiteral("app.home"));
    addLocal(help, tr("Recuperar sessão…"), QStringLiteral("app.recovery"));
    addLocal(help, tr("Sobre o Petunia3D"), QStringLiteral("app.about"));
}

void PetuniaAppBar::rebuildAddMenu(const QJsonObject &state) {
    QString signature;
    for (const auto &value : state["commands"].toArray()) {
        const auto id = value.toObject()["id"].toString();
        if (id.startsWith(QLatin1String("model.add_")))
            signature += id + QLatin1Char('|');
    }
    signature += state["active_language"].toString();
    if (signature == m_addSignature)
        return;
    m_addSignature = signature;
    m_addMenu->clear();
    for (const auto &value : state["commands"].toArray()) {
        const auto cmd = value.toObject();
        const auto id = cmd["id"].toString();
        if (!id.startsWith(QLatin1String("model.add_")))
            continue;
        const QString label = cmd["label"].toString().isEmpty() ? id.mid(10) : cmd["label"].toString();
        auto *action = m_addMenu->addAction(PetuniaIcons::get(id.mid(10)), petuniaTr(label));
        connect(action, &QAction::triggered, this, [this, id] { m_controller->command(id); });
    }
    m_addMenu->addSeparator();
    auto *library = m_addMenu->addAction(tr("Biblioteca de assets…"));
    connect(library, &QAction::triggered, this, [this] { emit localAction(QStringLiteral("app.assets")); });
}

void PetuniaAppBar::resizeEvent(QResizeEvent *event) {
    QWidget::resizeEvent(event);
    applyCompact(event->size().width() < PetuniaTheme::instance().scaled(1040));
}

void PetuniaAppBar::applyCompact(bool compact) {
    if (m_compact == compact)
        return;
    m_compact = compact;
    m_menuBar->setVisible(!compact);
    m_hamburger->setVisible(compact);
    m_project->setVisible(!compact);
    // Busca: só o ícone, mas o nome acessível e a dica continuam completos.
    m_search->setText(compact ? QString() : m_search->property("fullText").toString());
}

void PetuniaAppBar::applyMetrics() {
    const auto &theme = PetuniaTheme::instance();
    m_layout->setContentsMargins(theme.space(3), theme.space(1), theme.space(3), theme.space(1));
    m_layout->setSpacing(theme.space(2));
    const int logo = theme.iconPx() + theme.space(1);
    m_logo->setPixmap(PetuniaIcons::get(QStringLiteral("logo")).pixmap(logo, logo));
    const QSize icon(theme.iconPx(), theme.iconPx());
    for (auto *button : {m_undo, m_redo, m_settings}) {
        button->setIconSize(icon);
        button->setMinimumHeight(theme.controlHeight());
        button->setMinimumWidth(theme.controlHeight());
    }
    m_search->setIconSize(icon);
    m_search->setMinimumHeight(theme.controlHeight());
    m_hamburger->setIconSize(icon);
    m_hamburger->setMinimumSize(theme.controlHeight(), theme.controlHeight());
    for (auto *button : m_workspaces->buttons()) {
        button->setMinimumHeight(theme.controlHeight());
        button->setMinimumWidth(theme.scaled(58));
    }
    setMinimumHeight(theme.controlHeight() + theme.space(2));
    setMaximumHeight(theme.controlHeight() + theme.space(2));
}

void PetuniaAppBar::sync(const QJsonObject &state) {
    m_workspaces->setCurrent(m_controller->workspaceId());
    m_undo->setEnabled(state["can_undo"].toBool());
    m_redo->setEnabled(state["can_redo"].toBool());

    const QString path = state["project_path"].toString();
    QString name = path.isEmpty() ? tr("Sem título") : QFileInfo(path).completeBaseName();
    const bool dirty = !state["saved"].toBool(true);
    // O estado "modificado" nunca depende só de cor: ponto + texto acessível.
    const QString shown = dirty ? QStringLiteral("● ") + name : name;
    if (m_project->text() != shown)
        m_project->setText(shown);
    m_project->setAccessibleDescription(dirty ? tr("Alterações não salvas") : tr("Salvo"));
    m_project->setToolTip(path.isEmpty() ? tr("Projeto ainda não salvo") : path);

    for (const auto &tracked : m_tracked) {
        const QString shortcut = m_controller->shortcut(tracked.keymap);
        const QString text = shortcut.isEmpty() ? tracked.text : tracked.text + QLatin1Char('\t') + shortcut;
        if (tracked.action->text() != text)
            tracked.action->setText(text);
    }
    const auto searchShortcut = m_controller->shortcut(QStringLiteral("global.command_palette"));
    const QString full = searchShortcut.isEmpty() ? tr("Buscar comando") : tr("Buscar comando") + QStringLiteral("  %1").arg(searchShortcut);
    m_search->setProperty("fullText", full);
    m_search->setText(m_compact ? QString() : full);
    m_search->setToolTip(full);
    rebuildAddMenu(state);
}
