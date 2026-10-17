#include "petunia_properties_panel.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"
#include "petunia_model_sections.h"
#include "petunia_property_section.h"

#include <QLabel>
#include <QMenu>
#include <QScrollArea>
#include <QStackedWidget>
#include <QTabBar>
#include <QTabWidget>
#include <QToolButton>
#include <QVBoxLayout>

PetuniaPropertiesPanel::PetuniaPropertiesPanel(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaPropertiesPanel"));
    setAccessibleName(tr("Propriedades"));
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);
    m_stack = new QStackedWidget(this);
    root->addWidget(m_stack);
    buildModeling();
}

void PetuniaPropertiesPanel::buildModeling() {
    m_scroll = new QScrollArea(m_stack);
    m_scroll->setFrameShape(QFrame::NoFrame);
    m_scroll->setWidgetResizable(true);
    m_scroll->setAccessibleName(tr("Propriedades do objeto"));
    m_scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_column = new QWidget(m_scroll);
    m_layout = new QVBoxLayout(m_column);
    m_layout->setContentsMargins(0, 0, 0, 0);
    m_layout->setSpacing(0);
    m_scroll->setWidget(m_column);
    m_stack->addWidget(m_scroll);

    m_empty = new QLabel(tr("Selecione um objeto para editar suas propriedades."), m_column);
    m_empty->setProperty("role", QStringLiteral("muted"));
    m_empty->setWordWrap(true);
    m_empty->setMargin(PetuniaTheme::instance().space(4));
    m_empty->setAccessibleName(tr("Nenhum objeto selecionado"));
    m_layout->addWidget(m_empty);

    auto transport = [this](const QString &action, const QJsonObject &args) {
        return m_controller->request(action, args);
    };
    auto bound = [&](PetuniaBoundForm *form) {
        form->setTransport(transport);
        m_forms.append(form);
        return form;
    };

    addSection(QStringLiteral("transform"), tr("Transformação"), bound(new PetuniaTransformForm()), true);
    addSection(QStringLiteral("geometry"), tr("Geometria"), bound(new PetuniaGeometryForm()), true);
    addSection(QStringLiteral("profile"), tr("Perfil e volume"), bound(new PetuniaProfileForm()), true);

    m_operations = new PetuniaOperationsPanel(m_controller);
    auto *operations = addSection(QStringLiteral("operations"), tr("Operações"), m_operations, true);
    auto *add = new QToolButton();
    add->setProperty("role", QStringLiteral("header"));
    add->setIcon(PetuniaIcons::get(QStringLiteral("plus")));
    add->setToolTip(tr("Adicionar operação ao objeto"));
    add->setAccessibleName(tr("Adicionar operação ao objeto"));
    add->setPopupMode(QToolButton::InstantPopup);
    auto *menu = new QMenu(add);
    auto *boolean = menu->addAction(PetuniaIcons::get(QStringLiteral("boolean_cut")),
                                    tr("Booleana (este objeto é a ferramenta)"));
    auto *mirror = menu->addAction(PetuniaIcons::get(QStringLiteral("mirror")), tr("Mirror (espelhar)"));
    auto *symmetry = menu->addAction(PetuniaIcons::get(QStringLiteral("symmetrize")), tr("Symmetry (simetrizar)"));
    connect(boolean, &QAction::triggered, this, [this] {
        m_controller->request(QStringLiteral("graph_add_boolean"));
    });
    connect(mirror, &QAction::triggered, this, [this] {
        m_controller->request(QStringLiteral("graph_add_operation"), {{"kind", "mirror"}});
    });
    connect(symmetry, &QAction::triggered, this, [this] {
        m_controller->request(QStringLiteral("graph_add_operation"), {{"kind", "symmetry"}});
    });
    add->setMenu(menu);
    operations->setHeaderAction(add);
    connect(m_operations, &PetuniaOperationsPanel::countChanged, this, [operations](int count) {
        operations->setSummary(count > 0 ? QObject::tr("%1 ativa(s)").arg(count) : QString());
    });

    addSection(QStringLiteral("boolean"), tr("Booleana (direta)"), bound(new PetuniaBooleanForm()), false);
    auto *material = new PetuniaMaterialForm();
    connect(material, &PetuniaMaterialForm::nodeEditorRequested, this, &PetuniaPropertiesPanel::materialNodesRequested);
    addSection(QStringLiteral("material"), tr("Material"), bound(material), false);
    m_layout->addStretch(1);
}

PetuniaPropertySection *PetuniaPropertiesPanel::addSection(const QString &id, const QString &title,
                                                           QWidget *content, bool open) {
    auto *section = new PetuniaPropertySection(id, title, content, m_column);
    section->setOpen(open);
    connect(section, &PetuniaPropertySection::toggled, this, &PetuniaPropertiesPanel::sectionToggled);
    m_layout->addWidget(section);
    m_sectionMap.insert(id, section);
    return section;
}

void PetuniaPropertiesPanel::setHosted(const QString &workspaceId, QWidget *widget) {
    if (m_hosted.contains(workspaceId))
        return;
    m_hosted.insert(workspaceId, widget);
    // O mínimo de um painel hospedado não pode alargar a coluna: o QStackedWidget
    // considera TODAS as páginas, então elas ignoram a largura preferida.
    widget->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Expanding);
    if (auto *tabs = qobject_cast<QTabWidget *>(widget)) {
        tabs->setUsesScrollButtons(true);
        tabs->tabBar()->setUsesScrollButtons(true);
        tabs->setElideMode(Qt::ElideRight);
        // Contêineres focáveis também precisam de nome (leitor de tela anuncia a região).
        tabs->setAccessibleName(tr("Propriedades de %1").arg(workspaceId));
        auto nameScrollAreas = [tabs] {
            for (int i = 0; i < tabs->count(); ++i) {
                const auto areas = tabs->widget(i)->findChildren<QScrollArea *>();
                QList<QScrollArea *> all = areas;
                if (auto *self = qobject_cast<QScrollArea *>(tabs->widget(i)))
                    all.prepend(self);
                for (auto *area : all)
                    if (area->accessibleName().isEmpty())
                        area->setAccessibleName(tabs->tabText(i).remove(QLatin1Char('&')));
            }
        };
        nameScrollAreas();
        connect(tabs, &QTabWidget::currentChanged, tabs, nameScrollAreas);
    }
    m_stack->addWidget(widget);
}

QJsonObject PetuniaPropertiesPanel::sectionStates() const {
    QJsonObject states;
    for (auto it = m_sectionMap.cbegin(); it != m_sectionMap.cend(); ++it)
        states.insert(it.key(), it.value()->isOpen());
    return states;
}

void PetuniaPropertiesPanel::applySectionStates(const QJsonObject &states) {
    for (auto it = states.constBegin(); it != states.constEnd(); ++it)
        if (auto *section = m_sectionMap.value(it.key()))
            section->setOpen(it.value().toBool(section->isOpen()));
}

QStringList PetuniaPropertiesPanel::visibleSectionIds() const {
    QStringList ids;
    for (auto it = m_sectionMap.cbegin(); it != m_sectionMap.cend(); ++it)
        if (!it.value()->isHidden())
            ids << it.key();
    ids.sort();
    return ids;
}

void PetuniaPropertiesPanel::sync(const QJsonObject &state) {
    const QString workspace = m_controller->workspaceId();
    const bool modeling = workspace == QLatin1String("DRAW") || workspace == QLatin1String("POLY");
    if (modeling) {
        m_stack->setCurrentWidget(m_scroll);
        const bool hasSelection = state["selected_count"].toInt() > 0;
        m_empty->setVisible(!hasSelection);
        for (auto it = m_sectionMap.cbegin(); it != m_sectionMap.cend(); ++it) {
            bool visible = hasSelection;
            if (it.key() == QLatin1String("profile"))
                visible = workspace == QLatin1String("DRAW");
            it.value()->setVisible(visible);
        }
        for (auto *form : m_forms)
            if (form->isVisible() || form->parentWidget())
                form->sync(state);
        m_operations->sync(state);
    } else if (auto *hosted = m_hosted.value(workspace)) {
        m_stack->setCurrentWidget(hosted);
    }
}
