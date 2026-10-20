#include "petunia_node_page.h"

#include "../petunia_theme.h"
#include "petunia_i18n.h"
#include "petunia_operation_card.h"
#include "petunia_segmented.h"

#include <QHBoxLayout>
#include <QJsonArray>
#include <QLabel>
#include <QPushButton>
#include <QScrollArea>
#include <QSplitter>
#include <QVBoxLayout>

PetuniaNodePage::PetuniaNodePage(PetuniaShellController *controller, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    const auto &theme = PetuniaTheme::instance();
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    auto *header = new QHBoxLayout();
    header->setContentsMargins(theme.space(2), theme.space(1), theme.space(2), theme.space(1));
    header->setSpacing(theme.space(2));
    m_domain = new PetuniaSegmented(this);
    m_domain->setItems({{QStringLiteral("operations"), tr("Operações"), tr("Operações de geometria do objeto"), {}},
                        {QStringLiteral("material"), tr("Material"), tr("Receita de nós do material ativo"), {}}});
    m_domain->setGroupLabel(tr("Grafo exibido"));
    m_domain->setCurrent(QStringLiteral("operations"));
    header->addWidget(m_domain);
    m_status = new QLabel(this);
    m_status->setProperty("role", QStringLiteral("muted"));
    header->addWidget(m_status, 1);
    m_create = new QPushButton(tr("Criar receita de nós"), this);
    m_create->setToolTip(tr("Cria uma receita de nós para o material ativo (Imagem → Saída)"));
    m_create->setAccessibleName(m_create->text());
    header->addWidget(m_create);
    m_bake = new QPushButton(tr("Tornar estático"), this);
    m_bake->setToolTip(tr("Mantém os mapas gerados e remove a receita do material"));
    m_bake->setAccessibleName(m_bake->text());
    header->addWidget(m_bake);
    root->addLayout(header);

    auto *split = new QSplitter(Qt::Horizontal, this);
    split->setChildrenCollapsible(false);
    m_canvas = new PetuniaNodeCanvas(controller, split);
    m_panel = new QScrollArea(split);
    m_panel->setWidgetResizable(true);
    m_panel->setFrameShape(QFrame::NoFrame);
    m_panel->setMinimumWidth(theme.scaled(240));
    m_panel->setAccessibleName(tr("Parâmetros do nó"));
    m_panelBody = new QWidget(m_panel);
    m_panelLayout = new QVBoxLayout(m_panelBody);
    m_panelLayout->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(2));
    m_panelHint = new QLabel(m_panelBody);
    m_panelHint->setProperty("role", QStringLiteral("muted"));
    m_panelHint->setWordWrap(true);
    m_panelLayout->addWidget(m_panelHint);
    m_panelLayout->addStretch(1);
    m_panel->setWidget(m_panelBody);
    split->addWidget(m_canvas);
    split->addWidget(m_panel);
    split->setStretchFactor(0, 3);
    split->setStretchFactor(1, 1);
    root->addWidget(split, 1);

    connect(m_domain, &PetuniaSegmented::activated, this, [this](const QString &id) {
        setDomain(id == QLatin1String("material") ? PetuniaNodeCanvas::Domain::Material
                                                  : PetuniaNodeCanvas::Domain::Operations);
    });
    connect(m_create, &QPushButton::clicked, this,
            [this] { m_controller->request(QStringLiteral("material_recipe_create")); });
    connect(m_bake, &QPushButton::clicked, this,
            [this] { m_controller->request(QStringLiteral("material_recipe_bake")); });
    connect(m_canvas, &PetuniaNodeCanvas::nodeSelected, this, [this] { refreshPanel(); });
}

void PetuniaNodePage::setDomain(PetuniaNodeCanvas::Domain domain) {
    {
        const QSignalBlocker block(m_domain);
        m_domain->setCurrent(domain == PetuniaNodeCanvas::Domain::Material ? QStringLiteral("material")
                                                                            : QStringLiteral("operations"));
    }
    m_canvas->setDomain(domain);
    if (!m_state.isEmpty())
        sync(m_state);
}

void PetuniaNodePage::sync(const QJsonObject &state) {
    m_state = state;
    m_canvas->sync(state);
    const bool material = m_canvas->domain() == PetuniaNodeCanvas::Domain::Material;
    const auto recipe = state["material_recipe"].toObject();
    const bool hasMaterial = !recipe["material_id"].isNull();
    const bool hasRecipe = recipe["has_recipe"].toBool();
    m_create->setVisible(material && !hasRecipe);
    m_create->setEnabled(hasMaterial);
    m_bake->setVisible(material && hasRecipe);
    QString status;
    if (material) {
        if (!hasMaterial)
            status = tr("Nenhum material no projeto.");
        else if (!hasRecipe)
            status = tr("Material: %1 — sem receita de nós.").arg(recipe["material_name"].toString());
        else if (!recipe["error"].toString().isEmpty())
            status = tr("Material: %1 — erro: %2").arg(recipe["material_name"].toString(), recipe["error"].toString());
        else
            status = tr("Material: %1 · Shift+A adiciona nós").arg(recipe["material_name"].toString());
    } else {
        status = tr("Operações do objeto · Shift+A adiciona");
    }
    m_status->setText(status);
    refreshPanel();
}

QJsonObject PetuniaNodePage::selectedRow(QJsonArray *candidates) const {
    const QString id = m_canvas->selectedNodeId();
    if (id.isEmpty())
        return {};
    const bool material = m_canvas->domain() == PetuniaNodeCanvas::Domain::Material;
    const auto source = material ? m_state["material_recipe"].toObject() : m_state["op_graph"].toObject();
    if (!material && candidates)
        *candidates = source["candidates"].toArray();
    for (const auto &value : source["nodes"].toArray())
        if (value.toObject()["id"].toString() == id)
            return value.toObject();
    return {};
}

void PetuniaNodePage::refreshPanel() {
    QJsonArray candidates;
    const auto row = selectedRow(&candidates);
    const bool material = m_canvas->domain() == PetuniaNodeCanvas::Domain::Material;
    const bool editable = !row.isEmpty() && (material || row["type_id"].toString() != QLatin1String("petunia.object"));
    if (!editable) {
        delete m_card;
        m_card = nullptr;
        m_cardSignature.clear();
        m_panelHint->setText(row.isEmpty() ? tr("Selecione um nó para editar seus parâmetros.")
                                           : tr("Objeto: %1. Selecione uma operação para editar.")
                                                 .arg(row["title"].toString()));
        m_panelHint->show();
        return;
    }
    const QString signature = (material ? QStringLiteral("M|") : QStringLiteral("O|")) + row["id"].toString() +
                              QLatin1Char('|') + row[material ? "kind" : "type_id"].toString();
    if (!m_card || signature != m_cardSignature) {
        delete m_card;
        m_card = new PetuniaOperationCard(m_controller, row, false, m_panelBody,
                                          material ? PetuniaOperationCard::Mode::Recipe
                                                   : PetuniaOperationCard::Mode::Operation);
        m_panelLayout->insertWidget(0, m_card);
        m_cardSignature = signature;
    }
    m_card->update(row, candidates);
    m_panelHint->hide();
}
