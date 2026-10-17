#include "petunia_model_sections.h"
#include "../petunia_icons.h"
#include "../petunia_color_picker.h"
#include "../petunia_theme.h"
#include "../petunia_vector3_field.h"
#include "petunia_operation_card.h"
#include "petunia_segmented.h"

#include <QCheckBox>
#include <QComboBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QSignalBlocker>

namespace {
QFormLayout *makeForm(QWidget *owner) {
    auto *form = new QFormLayout(owner);
    const auto &theme = PetuniaTheme::instance();
    form->setContentsMargins(theme.space(3), theme.space(2), theme.space(3), theme.space(3));
    form->setHorizontalSpacing(theme.space(2));
    form->setVerticalSpacing(theme.space(2));
    // Rótulo acima do campo: nunca corta o conteúdo em painéis estreitos.
    form->setRowWrapPolicy(QFormLayout::WrapAllRows);
    form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
    return form;
}
} // namespace

// ── Transform ───────────────────────────────────────────────────────────────

PetuniaTransformForm::PetuniaTransformForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = makeForm(this);
    xyz(form, tr("Posição"), "position", "transform", "text");
    xyz(form, tr("Rotação"), "rotation", "transform", "text");
    xyz(form, tr("Escala"), "scale", "transform", "text");

    auto *pivotRow = new QWidget(this);
    auto *pivotLayout = new QHBoxLayout(pivotRow);
    pivotLayout->setContentsMargins(0, 0, 0, 0);
    pivotLayout->setSpacing(PetuniaTheme::instance().space(1));
    auto *pivot = new QComboBox(pivotRow);
    pivot->setAccessibleName(tr("Origem do pivô"));
    const QStringList ids{"median", "bounds", "individual", "origin_geometry", "origin_bottom", "origin_selection"};
    const QStringList labels{tr("Ponto médio"), tr("Caixa delimitadora"), tr("Origens individuais"),
                             tr("Origem na geometria"), tr("Origem na base"), tr("Origem na seleção")};
    for (int i = 0; i < ids.size(); ++i)
        pivot->addItem(labels[i], ids[i]);
    connect(pivot, &QComboBox::activated, this, [this, pivot](int index) {
        const auto value = pivot->itemData(index).toString();
        if (value.startsWith(QLatin1String("origin_")))
            send(value);
        else
            send(QStringLiteral("set_pivot_point"), {{"id", value}});
    });
    pivotLayout->addWidget(pivot, 1);
    m_bindings.append({pivot, QStringLiteral("pivot_id"), {}, {}});

    auto *reset = new QPushButton(PetuniaIcons::get(QStringLiteral("undo")), QString(), pivotRow);
    reset->setToolTip(tr("Redefinir a origem na geometria"));
    reset->setAccessibleName(tr("Redefinir a origem na geometria"));
    connect(reset, &QPushButton::clicked, this, [this] { send(QStringLiteral("origin_geometry")); });
    pivotLayout->addWidget(reset);
    form->addRow(tr("Pivô"), pivotRow);

    check(form, tr("Editar pivô"), "edit_pivot", "edit_pivot", {});
}

// ── Geometria ───────────────────────────────────────────────────────────────

PetuniaGeometryForm::PetuniaGeometryForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = makeForm(this);
    m_selection = new QLabel(this);
    m_selection->setProperty("role", QStringLiteral("caption"));
    m_selection->setWordWrap(true);
    m_selection->setAccessibleName(tr("Seleção"));
    form->addRow(m_selection);
    m_stats = new QLabel(this);
    m_stats->setProperty("role", QStringLiteral("muted"));
    m_stats->setAccessibleName(tr("Estatísticas da cena"));
    form->addRow(m_stats);

    auto *shading = new PetuniaSegmented(this);
    shading->setItems({{"flat", tr("Facetado"), tr("Sombreamento plano"), QStringLiteral("shade_flat")},
                       {"smooth", tr("Suave"), tr("Sombreamento suave"), QStringLiteral("shade_smooth")}});
    shading->setGroupLabel(tr("Sombreamento da superfície"));
    connect(shading, &PetuniaSegmented::activated, this, [this](const QString &id) {
        send(QStringLiteral("command"),
             {{"id", id == QLatin1String("flat") ? QStringLiteral("model.shade_flat")
                                                  : QStringLiteral("model.shade_smooth")}});
    });
    form->addRow(tr("Sombreamento"), shading);
}

void PetuniaGeometryForm::afterSync(const QJsonObject &state) {
    m_selection->setText(state["selection_summary"].toString());
    m_stats->setText(state["scene_stats"].toString());
}

// ── Material ────────────────────────────────────────────────────────────────

PetuniaMaterialForm::PetuniaMaterialForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = makeForm(this);
    rows(form, tr("Materiais"), "material_options", "material_slot", "index");
    button(form, tr("Novo material"), "create_material");
    button(form, tr("Duplicar material"), "material_duplicate");
    button(form, tr("Remover material"), "material_remove");
    button(form, tr("Atribuir às faces/objeto"), "material_assign");

    auto *color = new QPushButton(tr("Cor base…"), this);
    color->setAccessibleName(tr("Cor base do material"));
    form->addRow(color);
    connect(color, &QPushButton::clicked, this, [this] {
        PetuniaColorPickerPopover picker(this);
        picker.setTitle(tr("Cor base do material"));
        if (picker.exec() == QDialog::Accepted) {
            const auto c = picker.color();
            send(QStringLiteral("set_active_material_base_color"),
                 {{"red", c.redF()}, {"green", c.greenF()}, {"blue", c.blueF()}});
        }
    });
    struct Scalar { const char *field; const char *label; double max; };
    const Scalar scalars[] = {{"roughness", "Rugosidade", 1},
                              {"metallic", "Metálico", 1},
                              {"normal_scale", "Intensidade do normal", 1},
                              {"emission_strength", "Emissão", 1000},
                              {"alpha_cutoff", "Corte alfa", 1}};
    for (const auto &s : scalars)
        number(form, tr(s.label), QStringLiteral("material_") + QLatin1String(s.field),
               "set_active_material_scalar", "value", 0, s.max, 0.01, {{"field", QLatin1String(s.field)}});
    choice(form, tr("Shader"), "material_profile", "material_profile", "index",
           {"pbr", "unlit", "toon", "glass", "emissive"},
           {tr("PBR"), tr("Sem luz"), tr("Toon"), tr("Vidro"), tr("Emissivo")});
    choice(form, tr("Transparência"), "material_alpha_mode", "material_alpha", "index",
           {"opaque", "mask", "blend"}, {tr("Opaco"), tr("Recorte"), tr("Mistura")});
    auto *nodes = new QPushButton(tr("Editar nós do material…"), this);
    nodes->setToolTip(tr("Abre o editor de nós com a receita de texturas deste material"));
    nodes->setAccessibleName(tr("Editar nós do material"));
    form->addRow(nodes);
    connect(nodes, &QPushButton::clicked, this, &PetuniaMaterialForm::nodeEditorRequested);
    button(form, tr("Criar textura albedo"), "create_albedo_texture");
    button(form, tr("Remover textura albedo"), "clear_albedo_texture");
}

// ── Booleana (modo direto, interino) ────────────────────────────────────────

PetuniaBooleanForm::PetuniaBooleanForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = makeForm(this);
    rows(form, tr("Operando"), "scene_items", "set_boolean_operand");
    check(form, tr("Limpar topologia"), "boolean_cleanup", "set_boolean_cleanup", "cleanup");
    check(form, tr("Manter partes"), "boolean_keep_parts", "set_boolean_keep_parts", "keep");
    struct Op { const char *id; const char *label; };
    for (const auto &op : {Op{"model.fuse", "União"}, Op{"model.cut", "Diferença"}, Op{"model.intersect", "Interseção"}})
        button(form, tr(op.label), "boolean_op", {{"id", QLatin1String(op.id)}}, "boolean_ready");
    button(form, tr("Juntar"), "join_operand");
    button(form, tr("Limpar operando"), "clear_boolean_operand");
}

// ── Perfil (DRAW) ───────────────────────────────────────────────────────────

PetuniaProfileForm::PetuniaProfileForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = makeForm(this);
    fileButton(form, tr("Importar SVG…"), "import_svg", tr("SVG (*.svg)"));
    button(form, tr("Retângulo"), "profile_rectangle", {{"width", 1.}, {"height", 1.}});
    button(form, tr("Círculo"), "profile_circle", {{"radius", 0.5}, {"segments", 32}});
    button(form, tr("Fechar perfil"), "close_profile");
    choice(form, tr("Plano de trabalho"), "profile_workplane", "profile_workplane", "id",
           {"auto", "ground", "face", "view", "selection"},
           {tr("Automático"), tr("Chão"), tr("Face"), tr("Vista"), tr("Seleção")});
    button(form, tr("Olhar para o plano"), "workplane_look");
    number(form, tr("Largura"), "profile_width", "profile_dimension", "value", 0.001, 10000, 0.01, {{"field", "width"}});
    number(form, tr("Altura"), "profile_height", "profile_dimension", "value", 0.001, 10000, 0.01, {{"field", "height"}});
    number(form, tr("Segmentos"), "profile_segments", "profile_dimension", "value", 3, 4096, 1, {{"field", "segments"}});
    number(form, tr("Profundidade"), "profile_depth", "set_profile_depth", "depth", 0, 10000);
    number(form, tr("Parede"), "profile_wall_thickness", "set_profile_wall_thickness", "thickness", 0, 10000);
    number(form, tr("Revolução"), "profile_revolve_angle", "set_profile_revolve_angle", "angle_deg", -360, 360, 1);
    number(form, tr("Suavidade"), "profile_smoothness", "set_profile_smoothness", "smoothness", 0, 1);
    button(form, tr("Suavizar curvas"), "profile_smooth_curves");
    button(form, tr("Remover curvas"), "profile_clear_curves");
    button(form, tr("Volume por extrusão"), "generate_profile_extrude");
    button(form, tr("Volume por revolução"), "generate_profile_revolve");
    button(form, tr("Volume por sweep"), "generate_profile_sweep");
    button(form, tr("Confirmar volume"), "commit_profile_volume");
    button(form, tr("Cancelar volume"), "cancel_profile_volume");
}

// ── Pilha de operações ──────────────────────────────────────────────────────

PetuniaOperationsPanel::PetuniaOperationsPanel(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    const auto &theme = PetuniaTheme::instance();
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(theme.space(3), theme.space(2), theme.space(3), theme.space(3));
    root->setSpacing(theme.space(2));
    m_appliedLayout = new QVBoxLayout();
    m_appliedLayout->setSpacing(theme.space(2));
    root->addLayout(m_appliedLayout);
    m_bake = new QPushButton(tr("Tornar editável (aplicar à malha)"), this);
    m_bake->setToolTip(tr("Grava o resultado das operações na malha do objeto (Make Editable)"));
    m_bake->setAccessibleName(tr("Tornar editável (aplicar à malha)"));
    connect(m_bake, &QPushButton::clicked, this, [this] { m_controller->request(QStringLiteral("graph_bake")); });
    m_bake->hide();
    root->addWidget(m_bake);
    m_toolHeading = heading(tr("Usado como ferramenta"));
    root->addWidget(m_toolHeading);
    m_toolLayout = new QVBoxLayout();
    m_toolLayout->setSpacing(theme.space(2));
    root->addLayout(m_toolLayout);
    m_legacyHeading = heading(tr("Modificadores (legado)"));
    root->addWidget(m_legacyHeading);
    m_legacyLayout = new QVBoxLayout();
    m_legacyLayout->setSpacing(theme.space(2));
    root->addLayout(m_legacyLayout);
    m_empty = new QLabel(tr("Nenhuma operação. Use “+” para adicionar uma booleana, Mirror ou Symmetry."), this);
    m_empty->setProperty("role", QStringLiteral("muted"));
    m_empty->setWordWrap(true);
    root->addWidget(m_empty);
    m_toolHeading->hide();
    m_legacyHeading->hide();
}

QLabel *PetuniaOperationsPanel::heading(const QString &text) {
    auto *label = new QLabel(text, this);
    label->setProperty("role", QStringLiteral("caption"));
    return label;
}

void PetuniaOperationsPanel::rebuildGraph(const QJsonArray &applied, const QJsonArray &tools) {
    qDeleteAll(m_applied);
    qDeleteAll(m_tools);
    m_applied.clear();
    m_tools.clear();
    for (const auto &value : applied) {
        auto *card = new PetuniaOperationCard(m_controller, value.toObject(), false, this);
        m_appliedLayout->addWidget(card);
        m_applied.append(card);
    }
    for (const auto &value : tools) {
        auto *card = new PetuniaOperationCard(m_controller, value.toObject(), true, this);
        m_toolLayout->addWidget(card);
        m_tools.append(card);
    }
}

void PetuniaOperationsPanel::sync(const QJsonObject &state) {
    const auto graph = state["op_graph"].toObject();
    const auto applied = graph["applied"].toArray();
    const auto tools = graph["as_tool"].toArray();
    const auto candidates = graph["candidates"].toArray();
    const auto legacy = state["modifier_rows"].toArray();

    QString signature;
    for (const auto &value : applied)
        signature += PetuniaOperationCard::signatureOf(value.toObject(), false) + QLatin1Char(';');
    for (const auto &value : tools)
        signature += PetuniaOperationCard::signatureOf(value.toObject(), true) + QLatin1Char(';');
    m_syncing = true;
    if (signature != m_graphSignature) {
        m_graphSignature = signature;
        rebuildGraph(applied, tools);
    }
    for (int i = 0; i < m_applied.size() && i < applied.size(); ++i)
        m_applied[i]->update(applied[i].toObject(), candidates);
    for (int i = 0; i < m_tools.size() && i < tools.size(); ++i)
        m_tools[i]->update(tools[i].toObject(), candidates);
    m_bake->setVisible(!applied.isEmpty());
    m_toolHeading->setVisible(!tools.isEmpty());

    QString legacySignature;
    for (const auto &value : legacy)
        legacySignature += value.toObject()["id"].toString() + QLatin1Char(':') + value.toObject()["kind"].toString() + QLatin1Char('|');
    if (legacySignature != m_legacySignature) {
        m_legacySignature = legacySignature;
        rebuildLegacy(legacy);
    }
    updateLegacy(legacy);
    m_legacyHeading->setVisible(!legacy.isEmpty());
    m_syncing = false;

    const int total = int(applied.size() + tools.size() + legacy.size());
    m_empty->setVisible(total == 0);
    if (total != m_count) {
        m_count = total;
        emit countChanged(total);
    }
}

PetuniaOperationsPanel::Row PetuniaOperationsPanel::makeLegacyRow(const QJsonObject &row) {
    const auto &theme = PetuniaTheme::instance();
    Row r;
    r.id = row["id"].toString();
    r.kind = row["kind"].toString();
    r.card = new QFrame(this);
    r.card->setProperty("role", QStringLiteral("card"));
    auto *layout = new QVBoxLayout(r.card);
    layout->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(2));
    layout->setSpacing(theme.space(2));

    auto *header = new QHBoxLayout();
    header->setSpacing(theme.space(1));
    r.enabled = new QCheckBox(row["title"].toString(), r.card);
    r.enabled->setAccessibleName(row["title"].toString());
    r.enabled->setToolTip(tr("Ativar ou desativar a operação"));
    header->addWidget(r.enabled, 1);
    auto iconButton = [&](const QString &icon, const QString &tip) {
        auto *b = new QPushButton(PetuniaIcons::get(icon), QString(), r.card);
        b->setProperty("role", QStringLiteral("quiet"));
        b->setToolTip(tip);
        b->setAccessibleName(tip);
        b->setFixedWidth(theme.controlHeight());
        return b;
    };
    r.up = iconButton(QStringLiteral("chevron_up"), tr("Mover para cima"));
    r.down = iconButton(QStringLiteral("chevron_down"), tr("Mover para baixo"));
    auto *apply = iconButton(QStringLiteral("check"), tr("Aplicar à malha (destrutivo)"));
    auto *remove = iconButton(QStringLiteral("close"), tr("Remover operação"));
    header->addWidget(r.up);
    header->addWidget(r.down);
    header->addWidget(apply);
    header->addWidget(remove);
    layout->addLayout(header);

    r.axis = new PetuniaSegmented(r.card);
    r.axis->setItems({{"0", QStringLiteral("X"), tr("Eixo X"), {}},
                      {"1", QStringLiteral("Y"), tr("Eixo Y"), {}},
                      {"2", QStringLiteral("Z"), tr("Eixo Z"), {}}});
    r.axis->setGroupLabel(tr("Eixo de simetria"));
    auto *axisRow = new QHBoxLayout();
    auto *axisLabel = new QLabel(tr("Eixo"), r.card);
    axisLabel->setProperty("role", QStringLiteral("caption"));
    axisRow->addWidget(axisLabel);
    axisRow->addWidget(r.axis, 1);
    layout->addLayout(axisRow);

    r.direction = new QCheckBox(tr("Positivo → negativo"), r.card);
    r.direction->setVisible(r.kind == QLatin1String("symmetry"));
    layout->addWidget(r.direction);

    const QString id = r.id;
    connect(r.enabled, &QCheckBox::toggled, this, [this, id](bool on) {
        if (!m_syncing)
            m_controller->request(QStringLiteral("set_modifier_enabled"), {{"id", id}, {"enabled", on}});
    });
    connect(r.axis, &PetuniaSegmented::activated, this, [this, id](const QString &axis) {
        m_controller->request(QStringLiteral("set_modifier_axis"), {{"id", id}, {"axis", axis.toInt()}});
    });
    connect(r.direction, &QCheckBox::toggled, this, [this, id](bool on) {
        if (!m_syncing)
            m_controller->request(QStringLiteral("set_modifier_direction"), {{"id", id}, {"direction_value", on}});
    });
    connect(r.up, &QPushButton::clicked, this, [this, id] {
        m_controller->request(QStringLiteral("move_modifier"), {{"id", id}, {"direction", -1}});
    });
    connect(r.down, &QPushButton::clicked, this, [this, id] {
        m_controller->request(QStringLiteral("move_modifier"), {{"id", id}, {"direction", 1}});
    });
    connect(apply, &QPushButton::clicked, this, [this, id] {
        m_controller->request(QStringLiteral("apply_modifier"), {{"id", id}});
    });
    connect(remove, &QPushButton::clicked, this, [this, id] {
        m_controller->request(QStringLiteral("remove_modifier"), {{"id", id}});
    });
    return r;
}

void PetuniaOperationsPanel::rebuildLegacy(const QJsonArray &rows) {
    for (auto &row : m_rows)
        delete row.card;
    m_rows.clear();
    for (const auto &value : rows) {
        Row r = makeLegacyRow(value.toObject());
        m_legacyLayout->addWidget(r.card);
        m_rows.append(r);
    }
}

void PetuniaOperationsPanel::updateLegacy(const QJsonArray &rows) {
    for (int i = 0; i < m_rows.size() && i < rows.size(); ++i) {
        const auto o = rows[i].toObject();
        auto &r = m_rows[i];
        const QSignalBlocker b1(r.enabled), b2(r.direction);
        r.enabled->setChecked(o["enabled"].toBool());
        r.axis->setCurrent(QString::number(o["axis"].toInt()));
        r.direction->setChecked(o["positive_to_negative"].toBool());
        r.up->setEnabled(o["can_move_up"].toBool());
        r.down->setEnabled(o["can_move_down"].toBool());
    }
}
