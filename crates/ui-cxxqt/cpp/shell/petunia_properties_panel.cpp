#include "petunia_properties_panel.h"
#include "../petunia_editor_api.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"
#include "petunia_model_sections.h"
#include "petunia_property_section.h"
#include "petunia_workspace_sections.h"

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
    buildPaint();
    buildUv();
    buildAnimate();
    m_layout->addStretch(1);
}

PetuniaBoundForm *PetuniaPropertiesPanel::bound(PetuniaBoundForm *form, const QString &workspace) {
    form->setTransport([this](const QString &action, const QJsonObject &args) {
        return m_controller->request(action, args);
    });
    m_forms[workspace].append(form);
    return form;
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

    auto bound = [this](PetuniaBoundForm *form) { return this->bound(form, QStringLiteral("DRAW")); };

    addSection(QStringLiteral("transform"), tr("Transformação"), bound(new PetuniaTransformForm()), true);
    addSection(QStringLiteral("geometry"), tr("Sombreamento"), bound(new PetuniaGeometryForm()), true);
    addSection(QStringLiteral("profile"), tr("Perfil e volume"), bound(new PetuniaProfileForm()), false);

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

    auto *material = new PetuniaMaterialForm();
    connect(material, &PetuniaMaterialForm::nodeEditorRequested, this, &PetuniaPropertiesPanel::materialNodesRequested);
    addSection(QStringLiteral("material"), tr("Material"), bound(material), false);
}

void PetuniaPropertiesPanel::buildPaint() {
    const QStringList paint{QStringLiteral("PAINT")};
    auto b = [this](PetuniaBoundForm *form) { return bound(form, QStringLiteral("PAINT")); };
    addSection(QStringLiteral("paint.brush"), tr("Pincel"), b(new PetuniaBrushForm()), true, paint);
    addSection(QStringLiteral("paint.layer"), tr("Camada ativa"), b(new PetuniaPaintLayerForm()), true, paint);
    addSection(QStringLiteral("paint.target"), tr("Alvo da pintura"), b(new PetuniaPaintTargetForm()), false, paint);
    addSection(QStringLiteral("paint.effects"), tr("Efeitos"), b(new PetuniaPaintEffectsForm()), false, paint);
    addSection(QStringLiteral("paint.projection"), tr("Projeção e superfície"), b(new PetuniaPaintProjectionForm()),
               false, paint);
    addSection(QStringLiteral("paint.decal"), tr("Decalque"), b(new PetuniaDecalForm()), false, paint);
    addSection(QStringLiteral("paint.canvas"), tr("Canvas"), b(new PetuniaPaintCanvasForm()), false, paint);
}

void PetuniaPropertiesPanel::buildUv() {
    const QStringList uv{QStringLiteral("UV")};
    auto b = [this](PetuniaBoundForm *form) { return bound(form, QStringLiteral("UV")); };
    addSection(QStringLiteral("uv.selection"), tr("Seleção UV"), b(new PetuniaUvSelectionForm()), true, uv);
    addSection(QStringLiteral("uv.unwrap"), tr("Desdobrar"), b(new PetuniaUvUnwrapForm()), true, uv);
    addSection(QStringLiteral("uv.seams"), tr("Costuras e pins"), b(new PetuniaUvSeamsForm()), false, uv);
    addSection(QStringLiteral("uv.pack"), tr("Empacotar e relaxar"), b(new PetuniaUvPackForm()), false, uv);
    addSection(QStringLiteral("uv.transform"), tr("Transformar UV"), b(new PetuniaUvTransformForm()), false, uv);
    addSection(QStringLiteral("uv.display"), tr("Exibição"), b(new PetuniaUvDisplayForm()), false, uv);
}

void PetuniaPropertiesPanel::buildAnimate() {
    const QStringList animate{QStringLiteral("ANIMATE")};
    auto b = [this](PetuniaBoundForm *form) { return bound(form, QStringLiteral("ANIMATE")); };
    addSection(QStringLiteral("animate.creature"), tr("Criatura"), b(new PetuniaCreatureForm()), true, animate);
    addSection(QStringLiteral("animate.motion"), tr("Motion"), b(new PetuniaMotionForm()), true, animate);
    addSection(QStringLiteral("animate.pose"), tr("Pose do osso"), b(new PetuniaPoseForm()), false, animate);
    addSection(QStringLiteral("animate.layers"), tr("Camadas de animação"), b(new PetuniaAnimationLayersForm()), false,
               animate);
    addSection(QStringLiteral("animate.timeline"), tr("Keyframes e curvas"), b(new PetuniaTimelineForm()), false,
               animate);
}

PetuniaPropertySection *PetuniaPropertiesPanel::addSection(const QString &id, const QString &title,
                                                           QWidget *content, bool open, const QStringList &workspaces) {
    auto *section = new PetuniaPropertySection(id, title, content, m_column);
    section->setOpen(open);
    connect(section, &PetuniaPropertySection::toggled, this, &PetuniaPropertiesPanel::sectionToggled);
    m_layout->addWidget(section);
    m_sectionMap.insert(id, section);
    m_sectionWorkspaces.insert(id, workspaces);
    return section;
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
    // Modelagem depende de um objeto selecionado; PAINT/UV/ANIMATE agem no ativo.
    const bool hasSelection = state["selected_count"].toInt() > 0;
    m_empty->setVisible(modeling && !hasSelection);
    for (auto it = m_sectionMap.cbegin(); it != m_sectionMap.cend(); ++it) {
        bool visible = m_sectionWorkspaces.value(it.key()).contains(workspace);
        if (modeling && visible)
            visible = hasSelection && (it.key() != QLatin1String("profile") || workspace == QLatin1String("DRAW"));
        it.value()->setVisible(visible);
    }
    const QString group = modeling ? QStringLiteral("DRAW") : workspace;
    for (auto *form : m_forms.value(group))
        form->sync(state);
    if (modeling)
        m_operations->sync(state);
}
