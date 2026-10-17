#include "petunia_operation_hud.h"
#include "petunia_operation_card.h"
#include "../petunia_number_field.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QCheckBox>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QSignalBlocker>
#include <QVBoxLayout>

/// Formulário interno com os construtores do `PetuniaBoundForm`, enviando as
/// ações pelo controlador do shell.
class PetuniaOperationHud::Form : public PetuniaBoundForm {
  public:
    explicit Form(PetuniaShellController *controller, QWidget *parent) : PetuniaBoundForm(parent) {
        setTransport([controller](const QString &action, const QJsonObject &args) {
            return controller->request(action, args);
        });
        form = new QFormLayout(this);
        const auto &theme = PetuniaTheme::instance();
        form->setContentsMargins(0, 0, 0, 0);
        form->setHorizontalSpacing(theme.space(2));
        form->setVerticalSpacing(theme.space(1));
        form->setRowWrapPolicy(QFormLayout::DontWrapRows);
        form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
    }
    QFormLayout *form{nullptr};
    using PetuniaBoundForm::check;
    using PetuniaBoundForm::number;
    using PetuniaBoundForm::button;
};

PetuniaOperationHud::PetuniaOperationHud(PetuniaShellController *controller, QWidget *parent)
    : QFrame(parent), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaOperationHud"));
    setProperty("role", QStringLiteral("overlay"));
    setAccessibleName(tr("Parâmetros da operação"));
    const auto &theme = PetuniaTheme::instance();
    m_root = new QVBoxLayout(this);
    m_root->setContentsMargins(theme.space(3), theme.space(2), theme.space(3), theme.space(3));
    m_root->setSpacing(theme.space(2));
    m_title = new QLabel(this);
    m_title->setProperty("role", QStringLiteral("title"));
    m_root->addWidget(m_title);
    m_hint = new QLabel(this);
    m_hint->setProperty("role", QStringLiteral("muted"));
    m_hint->setWordWrap(true);
    m_root->addWidget(m_hint);

    buildPrimitive();
    buildModal();
    buildLoopCut();
    buildSlice();
    buildGraphOperation();
    connect(m_controller, &PetuniaShellController::operationAdded, this, &PetuniaOperationHud::focusOperation);
    setMinimumWidth(theme.scaled(260));
    setMaximumWidth(theme.scaled(400));
    hide();
}

QWidget *PetuniaOperationHud::makeSection(const QString &id, const QString &) {
    auto *section = new QWidget(this);
    section->setProperty("sectionId", id);
    auto *layout = new QVBoxLayout(section);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(PetuniaTheme::instance().space(2));
    m_root->addWidget(section);
    m_sections.insert(id, section);
    section->hide();
    return section;
}

QPushButton *PetuniaOperationHud::actionButton(QWidget *parent, const QString &text, const QString &request, bool primary) {
    auto *button = new QPushButton(text, parent);
    if (primary)
        button->setProperty("role", QStringLiteral("primary"));
    button->setAccessibleName(text);
    connect(button, &QPushButton::clicked, this, [this, request] { m_controller->request(request); });
    return button;
}

void PetuniaOperationHud::buildPrimitive() {
    auto *section = makeSection(QStringLiteral("primitive"), tr("Primitiva"));
    auto *form = new Form(m_controller, section);
    m_forms.insert(QStringLiteral("primitive"), form);
    struct Param { const char *name; const char *label; double min; double max; double step; };
    const Param params[] = {
        {"width", "Largura", 0.001, 1000, 0.1},   {"height", "Altura", 0.001, 1000, 0.1},
        {"depth", "Profundidade", 0.001, 1000, 0.1}, {"radius", "Raio", 0.001, 1000, 0.1},
        {"radius_b", "Raio secundário", 0.001, 1000, 0.1}, {"sides", "Lados", 3, 256, 1},
        {"rings", "Anéis", 2, 256, 1}, {"subdiv", "Subdivisões", 0, 8, 1},
    };
    for (const auto &p : params)
        form->number(form->form, tr(p.label), QStringLiteral("primitive_") + QLatin1String(p.name),
                     QStringLiteral("update_primitive_param_float"), QStringLiteral("value"), p.min, p.max, p.step,
                     {{"param", QLatin1String(p.name)}});
    struct Flag { const char *name; const char *label; };
    for (const auto &f : {Flag{"cap_top", "Tampa superior"}, Flag{"cap_bottom", "Tampa inferior"}, Flag{"fill_disc", "Preencher disco"}})
        form->check(form->form, tr(f.label), QStringLiteral("primitive_") + QLatin1String(f.name),
                    QStringLiteral("update_primitive_param_bool"), QStringLiteral("value"),
                    {{"param", QLatin1String(f.name)}});
    section->layout()->addWidget(form);

    auto *buttons = new QHBoxLayout();
    buttons->addWidget(actionButton(section, tr("Cancelar"), QStringLiteral("cancel_primitive"), false));
    buttons->addWidget(actionButton(section, tr("Confirmar"), QStringLiteral("confirm_primitive"), true));
    static_cast<QVBoxLayout *>(section->layout())->addLayout(buttons);
}

void PetuniaOperationHud::buildModal() {
    auto *section = makeSection(QStringLiteral("modal"), tr("Valor da operação"));
    m_modalLabel = new QLabel(section);
    m_modalLabel->setProperty("role", QStringLiteral("caption"));
    section->layout()->addWidget(m_modalLabel);
    m_modalValue = new PetuniaNumberField(section);
    m_modalValue->setPropertyId(QStringLiteral("tool_modal_value"));
    m_modalValue->setLabel(tr("Valor da operação"));
    m_modalValue->setRange(-100000, 100000);
    connect(m_modalValue, &PetuniaNumberField::previewRequested, this, [this](const QString &, double value) {
        m_controller->request(QStringLiteral("set_tool_modal_value"), {{"value", value}});
    });
    connect(m_modalValue, &PetuniaNumberField::commitRequested, this, [this](const QString &, double value, const QString &) {
        m_controller->request(QStringLiteral("set_tool_modal_value"), {{"value", value}});
    });
    section->layout()->addWidget(m_modalValue);

    m_bevelOptions = new QWidget(section);
    auto *bevel = new QVBoxLayout(m_bevelOptions);
    bevel->setContentsMargins(0, 0, 0, 0);
    auto *clamp = new QCheckBox(tr("Limitar sobreposição"), m_bevelOptions);
    clamp->setObjectName(QStringLiteral("bevel_clamp_overlap"));
    auto *vertices = new QCheckBox(tr("Afetar vértices"), m_bevelOptions);
    vertices->setObjectName(QStringLiteral("bevel_affect_vertices"));
    connect(clamp, &QCheckBox::clicked, this, [this] { m_controller->request(QStringLiteral("toggle_bevel_clamp_overlap")); });
    connect(vertices, &QCheckBox::clicked, this, [this] { m_controller->request(QStringLiteral("toggle_bevel_affect_vertices")); });
    bevel->addWidget(clamp);
    bevel->addWidget(vertices);
    section->layout()->addWidget(m_bevelOptions);

    auto *buttons = new QHBoxLayout();
    buttons->addWidget(actionButton(section, tr("Cancelar"), QStringLiteral("cancel_tool_modal"), false));
    buttons->addWidget(actionButton(section, tr("Confirmar"), QStringLiteral("commit_tool_modal"), true));
    static_cast<QVBoxLayout *>(section->layout())->addLayout(buttons);
}

void PetuniaOperationHud::buildLoopCut() {
    auto *section = makeSection(QStringLiteral("loop_cut"), tr("Loop cut"));
    auto *form = new Form(m_controller, section);
    m_forms.insert(QStringLiteral("loop_cut"), form);
    form->number(form->form, tr("Cortes"), QStringLiteral("loop_cut_cuts"), QStringLiteral("set_loop_cut_count"),
                 QStringLiteral("cuts"), 1, 128, 1);
    form->number(form->form, tr("Deslizamento"), QStringLiteral("loop_cut_slide"), QStringLiteral("set_loop_cut_slide"),
                 QStringLiteral("slide"), -1, 1, 0.05);
    form->check(form->form, tr("Corte balanceado"), QStringLiteral("loop_cut_balanced"),
                QStringLiteral("set_loop_cut_balanced"), QStringLiteral("balanced"));
    section->layout()->addWidget(form);
    auto *buttons = new QHBoxLayout();
    buttons->addWidget(actionButton(section, tr("Cancelar"), QStringLiteral("cancel_loop_cut"), false));
    m_loopCutConfirm = actionButton(section, tr("Confirmar"), QStringLiteral("commit_loop_cut"), true);
    buttons->addWidget(m_loopCutConfirm);
    static_cast<QVBoxLayout *>(section->layout())->addLayout(buttons);
}

void PetuniaOperationHud::buildSlice() {
    auto *section = makeSection(QStringLiteral("slice"), tr("Corte"));
    auto *form = new Form(m_controller, section);
    m_forms.insert(QStringLiteral("slice"), form);
    form->check(form->form, tr("Aparar o corte"), QStringLiteral("slice_trim"), QStringLiteral("set_slice_trim"),
                QStringLiteral("trim"));
    section->layout()->addWidget(form);
    auto *buttons = new QHBoxLayout();
    buttons->addWidget(actionButton(section, tr("Cancelar"), QStringLiteral("cancel_slice"), false));
    buttons->addWidget(actionButton(section, tr("Confirmar"), QStringLiteral("commit_slice"), true));
    static_cast<QVBoxLayout *>(section->layout())->addLayout(buttons);
}

void PetuniaOperationHud::buildGraphOperation() {
    auto *section = makeSection(QStringLiteral("graph_operation"), tr("Operação"));
    m_graphDone = new QPushButton(tr("Concluir"), section);
    m_graphDone->setProperty("role", QStringLiteral("primary"));
    m_graphDone->setAccessibleName(tr("Concluir a edição da operação"));
    connect(m_graphDone, &QPushButton::clicked, this, &PetuniaOperationHud::dismissOperation);
    section->layout()->addWidget(m_graphDone);
}

void PetuniaOperationHud::focusOperation(const QString &nodeId) {
    m_graphFocus = nodeId;
    m_graphSignature.clear();
    syncGraphOperation(m_controller->state());
    const bool visible = !m_active.isEmpty();
    if (isVisible() != visible)
        setVisible(visible);
}

void PetuniaOperationHud::dismissOperation() {
    m_graphFocus.clear();
    syncGraphOperation(m_controller->state());
    if (m_active.isEmpty())
        hide();
}

/// O cartão do HUD é o mesmo gerado da Pilha: um nó novo aparece aqui sem código Qt novo.
void PetuniaOperationHud::syncGraphOperation(const QJsonObject &state) {
    auto *section = m_sections.value(QStringLiteral("graph_operation"));
    QJsonObject row;
    const auto graph = state["op_graph"].toObject();
    if (!m_graphFocus.isEmpty()) {
        for (const auto &node : graph["nodes"].toArray())
            if (node.toObject()["id"].toString() == m_graphFocus)
                row = node.toObject();
    }
    const bool on = !row.isEmpty() && row["type_id"].toString() != QLatin1String("petunia.object");
    if (!on) {
        m_graphFocus.clear();
        if (m_graphCard) {
            delete m_graphCard;
            m_graphCard = nullptr;
        }
        m_graphSignature.clear();
        section->setVisible(false);
        m_active.removeAll(QStringLiteral("graph_operation"));
        return;
    }
    const bool asTool = row["tool"].toObject()["id"].toString() == state["active_asset_id"].toString();
    const QString signature = PetuniaOperationCard::signatureOf(row, asTool);
    if (!m_graphCard || signature != m_graphSignature) {
        delete m_graphCard;
        m_graphCard = new PetuniaOperationCard(m_controller, row, asTool, section);
        static_cast<QVBoxLayout *>(section->layout())->insertWidget(0, m_graphCard);
        m_graphSignature = signature;
    }
    m_graphCard->update(row, graph["candidates"].toArray());
    section->setVisible(true);
    if (!m_active.contains(QStringLiteral("graph_operation")))
        m_active << QStringLiteral("graph_operation");
}

void PetuniaOperationHud::sync(const QJsonObject &state) {
    m_active.clear();
    auto show = [&](const char *id, bool on) {
        if (on)
            m_active << QLatin1String(id);
        if (auto *section = m_sections.value(QLatin1String(id)))
            section->setVisible(on);
    };
    const bool primitive = state["primitive_active"].toBool();
    const bool modal = state["tool_modal_active"].toBool();
    const bool loopCutActive = state["loop_cut_active"].toBool();
    // Os parâmetros do loop cut existem desde que a ferramenta está ativa (antes do
    // primeiro hover); confirmar só faz sentido com um corte em prévia.
    const bool loopCut = loopCutActive || state["active_tool"].toString() == QLatin1String("loop_cut");
    const bool slice = state["slice_preview_visible"].toBool() || state["active_tool"].toString() == QLatin1String("slice");
    show("primitive", primitive);
    show("modal", modal);
    show("loop_cut", loopCut);
    show("slice", slice);
    syncGraphOperation(state);

    QString title;
    if (primitive)
        title = state["primitive_title"].toString();
    else if (modal)
        title = state["tool_modal_title"].toString(state["operation_hud_title"].toString());
    else if (loopCut)
        title = tr("Loop cut");
    else if (slice)
        title = tr("Corte");
    else if (m_active.contains(QStringLiteral("graph_operation")))
        title = tr("Operação");
    m_title->setText(petuniaTr(title));
    m_title->setVisible(!m_active.isEmpty());

    QString hint = state["operation_hud_hint"].toString();
    QStringList lines;
    for (const auto &line : state["operation_hud_lines"].toArray())
        lines << line.toString();
    if (!lines.isEmpty())
        hint = lines.join(QStringLiteral(" · ")) + (hint.isEmpty() ? QString() : QStringLiteral("\n") + hint);
    m_hint->setText(hint);
    m_hint->setVisible(!m_active.isEmpty() && !hint.isEmpty());

    if (modal) {
        m_modalLabel->setText(petuniaTr(state["tool_modal_label"].toString()));
        m_modalLabel->setVisible(!m_modalLabel->text().isEmpty());
        const double lo = state["tool_modal_min"].toDouble(-100000);
        const double hi = state["tool_modal_max"].toDouble(100000);
        m_modalValue->setRange(lo, hi);
        const double step = state["tool_modal_step"].toDouble(0.1);
        m_modalValue->setSingleStep(step > 0 ? step : 0.1);
        m_modalValue->setValue(state["tool_modal_value"].toDouble());
        const bool bevel = state["tool_modal_id"].toString() == QLatin1String("bevel");
        m_bevelOptions->setVisible(bevel);
        if (bevel) {
            if (auto *clamp = m_bevelOptions->findChild<QCheckBox *>(QStringLiteral("bevel_clamp_overlap"))) {
                const QSignalBlocker block(clamp);
                clamp->setChecked(state["bevel_clamp_overlap"].toBool());
            }
            if (auto *vertices = m_bevelOptions->findChild<QCheckBox *>(QStringLiteral("bevel_affect_vertices"))) {
                const QSignalBlocker block(vertices);
                vertices->setChecked(state["bevel_affect_vertices"].toBool());
            }
        }
    }
    if (m_loopCutConfirm)
        m_loopCutConfirm->setEnabled(loopCutActive);
    for (auto *form : m_forms)
        if (form->parentWidget() && form->parentWidget()->isVisible())
            form->sync(state);

    const bool visible = !m_active.isEmpty();
    if (isVisible() != visible)
        setVisible(visible);
    if (visible)
        adjustSize();
}
