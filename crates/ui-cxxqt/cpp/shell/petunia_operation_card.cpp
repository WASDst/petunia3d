#include "petunia_operation_card.h"
#include "../petunia_icons.h"
#include "../petunia_number_field.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"
#include "petunia_segmented.h"
#include "../petunia_color_picker.h"

#include <QCheckBox>
#include <QColor>
#include <QToolButton>
#include <QComboBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QLabel>
#include <QPushButton>
#include <QSet>
#include <QSignalBlocker>
#include <QVBoxLayout>

namespace {
// Nomes de exibição das opções (ids estáveis em inglês); o resto cai no id capitalizado.
QString optionLabel(const QString &id) {
    return petuniaChoiceLabel(id);
}
} // namespace

QString PetuniaOperationCard::signatureOf(const QJsonObject &row, bool asTool) {
    return row["id"].toString() + QLatin1Char('|') + row["type_id"].toString() + QLatin1Char('|') +
           (asTool ? QStringLiteral("tool") : QStringLiteral("applied"));
}

PetuniaOperationCard::PetuniaOperationCard(PetuniaShellController *controller, const QJsonObject &row,
                                           bool asTool, QWidget *parent, Mode mode)
    : QFrame(parent), m_controller(controller), m_nodeId(row["id"].toString()), m_asTool(asTool), m_mode(mode) {
    setProperty("role", QStringLiteral("card"));
    const auto &theme = PetuniaTheme::instance();
    m_layout = new QVBoxLayout(this);
    m_layout->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(2));
    m_layout->setSpacing(theme.space(2));

    const QString title = petuniaTr(row["label"].toString());
    auto *header = new QHBoxLayout();
    header->setSpacing(theme.space(1));
    m_enabled = new QCheckBox(title, this);
    m_enabled->setAccessibleName(title);
    m_enabled->setToolTip(tr("Ativar ou desativar a operação"));
    header->addWidget(m_enabled, 1);
    const bool recipe = mode == Mode::Recipe;
    if (recipe) {
        // Nós de receita não se desligam: o título é só um rótulo.
        m_enabled->hide();
        auto *heading = new QLabel(title, this);
        heading->setProperty("role", QStringLiteral("title"));
        header->insertWidget(0, heading, 1);
    }
    const bool applied = !asTool && !recipe && row["tool"].isObject();
    auto *remove = new QPushButton(PetuniaIcons::get(QStringLiteral("close")), QString(), this);
    remove->setProperty("role", QStringLiteral("quiet"));
    remove->setFixedWidth(theme.controlHeight());
    const QString removeTip = recipe    ? tr("Remover nó")
                              : applied ? tr("Desligar a operação deste objeto")
                                        : tr("Remover operação");
    remove->setToolTip(removeTip);
    remove->setAccessibleName(removeTip);
    header->addWidget(remove);
    m_layout->addLayout(header);

    m_subtitle = new QLabel(this);
    m_subtitle->setProperty("role", QStringLiteral("muted"));
    m_subtitle->setWordWrap(true);
    m_layout->addWidget(m_subtitle);

    m_error = new QLabel(this);
    m_error->setProperty("role", QStringLiteral("error"));
    m_error->setWordWrap(true);
    m_error->hide();
    m_layout->addWidget(m_error);

    buildParams(row["specs"].toArray());

    if (asTool) {
        auto *label = new QLabel(tr("Aplicar em:"), this);
        label->setProperty("role", QStringLiteral("caption"));
        m_layout->addWidget(label);
        m_targets = new QWidget(this);
        auto *targets = new QVBoxLayout(m_targets);
        targets->setContentsMargins(0, 0, 0, 0);
        targets->setSpacing(theme.space(1));
        m_layout->addWidget(m_targets);
    }

    const QString id = m_nodeId;
    connect(m_enabled, &QCheckBox::toggled, this, [this, id](bool on) {
        if (!m_syncing)
            m_controller->request(QStringLiteral("graph_set_enabled"), {{"node", id}, {"enabled", on}});
    });
    connect(remove, &QPushButton::clicked, this, [this, id, applied, recipe] {
        if (recipe) {
            m_controller->request(QStringLiteral("material_recipe_remove"), {{"node", id}});
        } else if (applied) {
            // Desliga só deste objeto: o alvo é o objeto ativo do editor.
            m_controller->request(QStringLiteral("graph_unapply"),
                                  {{"operation", id}, {"target", m_controller->text(QStringLiteral("active_asset_id"))}});
        } else {
            m_controller->request(QStringLiteral("graph_remove"), {{"node", id}});
        }
    });
}

void PetuniaOperationCard::sendParam(const QString &key, const QJsonValue &value, bool integer) {
    if (m_syncing)
        return;
    QJsonObject args{{"node", m_nodeId}, {"key", key}, {"value", value}};
    if (integer)
        args.insert("integer", true);
    m_controller->request(m_mode == Mode::Recipe ? QStringLiteral("material_recipe_set")
                                                 : QStringLiteral("graph_set_param"),
                          args);
}

void PetuniaOperationCard::buildParams(const QJsonArray &specs) {
    const auto &theme = PetuniaTheme::instance();
    for (const auto &value : specs) {
        const auto spec = value.toObject();
        const QString kind = spec["kind"].toString();
        const QString key = spec["key"].toString();
        const QString label = petuniaTr(spec["label"].toString());
        QWidget *widget = nullptr;
        if (kind == QLatin1String("choice")) {
            const auto options = spec["options"].toArray();
            if (options.size() <= 4) {
                auto *segmented = new PetuniaSegmented(this);
                QList<PetuniaSegmented::Item> items;
                for (const auto &option : options)
                    items.append({option.toString(), optionLabel(option.toString()), optionLabel(option.toString()), {}});
                segmented->setItems(items);
                segmented->setGroupLabel(label);
                connect(segmented, &PetuniaSegmented::activated, this,
                        [this, key](const QString &id) { sendParam(key, id); });
                widget = segmented;
            } else {
                auto *combo = new QComboBox(this);
                combo->setAccessibleName(label);
                for (const auto &option : options)
                    combo->addItem(optionLabel(option.toString()), option.toString());
                connect(combo, &QComboBox::activated, this,
                        [this, key, combo](int index) { sendParam(key, combo->itemData(index).toString()); });
                widget = combo;
            }
        } else if (kind == QLatin1String("bool")) {
            auto *check = new QCheckBox(label, this);
            check->setAccessibleName(label);
            connect(check, &QCheckBox::toggled, this, [this, key](bool on) { sendParam(key, on); });
            widget = check;
        } else if (kind == QLatin1String("float") || kind == QLatin1String("int")) {
            auto *field = new PetuniaNumberField(this);
            field->setLabel(label);
            field->setPropertyId(key);
            field->setRange(spec["min"].toDouble(-1e9), spec["max"].toDouble(1e9));
            field->setSingleStep(kind == QLatin1String("int") ? 1.0 : 0.01);
            const bool integer = kind == QLatin1String("int");
            connect(field, &PetuniaNumberField::commitRequested, this,
                    [this, key, integer](const QString &, double v, const QString &) { sendParam(key, v, integer); });
            widget = field;
        } else if (kind == QLatin1String("color")) {
            // Amostra de cor: abre o seletor (teclado: Enter/Espaço); o valor vai como RGBA.
            auto *swatch = new QToolButton(this);
            swatch->setAccessibleName(label);
            swatch->setToolTip(label);
            swatch->setMinimumSize(theme.controlHeight() * 2, theme.controlHeight());
            connect(swatch, &QToolButton::clicked, this, [this, key, swatch, label] {
                PetuniaColorPickerPopover popover(this);
                popover.setTitle(label);
                popover.setColor(swatch->property("petuniaColor").value<QColor>());
                connect(&popover, &PetuniaColorPickerPopover::colorChanged, this, [this, key](const QColor &c) {
                    sendParam(key, QJsonArray{c.redF(), c.greenF(), c.blueF(), c.alphaF()});
                });
                popover.move(swatch->mapToGlobal(QPoint(0, swatch->height())));
                popover.exec();
            });
            widget = swatch;
        } else {
            continue; // `asset` e tipos futuros: sem controle genérico (ex.: nó Object)
        }
        if (kind != QLatin1String("bool")) {
            auto *row = new QHBoxLayout();
            auto *caption = new QLabel(label, this);
            caption->setProperty("role", QStringLiteral("caption"));
            row->addWidget(caption);
            row->addWidget(widget, 1);
            m_layout->addLayout(row);
        } else {
            m_layout->addWidget(widget);
        }
        m_params.append({key, kind, widget});
    }
}

void PetuniaOperationCard::rebuildTargets(const QJsonArray &candidates) {
    auto *layout = static_cast<QVBoxLayout *>(m_targets->layout());
    while (auto *item = layout->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    m_targetBoxes.clear();
    if (candidates.isEmpty()) {
        auto *none = new QLabel(tr("Nenhum outro objeto na cena."), m_targets);
        none->setProperty("role", QStringLiteral("muted"));
        layout->addWidget(none);
        return;
    }
    for (const auto &value : candidates) {
        const auto candidate = value.toObject();
        const QString id = candidate["id"].toString();
        auto *box = new QCheckBox(candidate["name"].toString(), m_targets);
        box->setAccessibleName(tr("Aplicar em %1").arg(candidate["name"].toString()));
        connect(box, &QCheckBox::toggled, this, [this, id](bool on) {
            if (m_syncing)
                return;
            m_controller->request(on ? QStringLiteral("graph_apply") : QStringLiteral("graph_unapply"),
                                  {{"operation", m_nodeId}, {"target", id}});
        });
        layout->addWidget(box);
        m_targetBoxes.insert(id, box);
    }
}

void PetuniaOperationCard::update(const QJsonObject &row, const QJsonArray &candidates) {
    m_syncing = true;
    if (m_mode == Mode::Operation) {
        const QSignalBlocker block(m_enabled);
        m_enabled->setChecked(row["enabled"].toBool());
    }
    QString subtitle;
    if (row["tool"].isObject()) // ausente (receita) ou nulo: sem ferramenta
        subtitle = tr("Ferramenta: %1").arg(row["tool"].toObject()["name"].toString());
    m_subtitle->setText(subtitle);
    m_subtitle->setVisible(!subtitle.isEmpty());
    const QString error = row["error"].toString();
    m_error->setText(error.isEmpty() ? QString() : QStringLiteral("⚠ ") + error);
    m_error->setVisible(!error.isEmpty());

    const auto params = row["params"].toObject();
    for (const auto &control : std::as_const(m_params)) {
        const QJsonValue value = params.value(control.key);
        QSignalBlocker block(control.widget);
        if (auto *segmented = qobject_cast<PetuniaSegmented *>(control.widget))
            segmented->setCurrent(value.toString());
        else if (auto *combo = qobject_cast<QComboBox *>(control.widget)) {
            const int index = combo->findData(value.toString());
            if (index >= 0)
                combo->setCurrentIndex(index);
        } else if (auto *check = qobject_cast<QCheckBox *>(control.widget))
            check->setChecked(value.toBool());
        else if (auto *field = qobject_cast<PetuniaNumberField *>(control.widget))
            field->setValue(value.toDouble());
        else if (auto *swatch = qobject_cast<QToolButton *>(control.widget)) {
            const auto rgba = value.toArray();
            const QColor color = QColor::fromRgbF(float(rgba.at(0).toDouble(1.0)), float(rgba.at(1).toDouble(1.0)),
                                                  float(rgba.at(2).toDouble(1.0)), float(rgba.at(3).toDouble(1.0)));
            swatch->setProperty("petuniaColor", color);
            swatch->setStyleSheet(QStringLiteral("QToolButton { background-color: %1; border: 1px solid %2; border-radius: 4px; }")
                                      .arg(color.name(), PetuniaTheme::instance().borderStrong().name()));
            swatch->setAccessibleDescription(color.name().toUpper());
        }
    }
    if (m_asTool) {
        QString signature;
        for (const auto &value : candidates)
            signature += value.toObject()["id"].toString() + QLatin1Char('|');
        if (signature != m_targetSignature) {
            m_targetSignature = signature;
            rebuildTargets(candidates);
        }
        QSet<QString> targets;
        for (const auto &target : row["targets"].toArray())
            targets.insert(target.toObject()["id"].toString());
        for (auto it = m_targetBoxes.cbegin(); it != m_targetBoxes.cend(); ++it) {
            const QSignalBlocker block(it.value());
            it.value()->setChecked(targets.contains(it.key()));
        }
    }
    m_syncing = false;
}
