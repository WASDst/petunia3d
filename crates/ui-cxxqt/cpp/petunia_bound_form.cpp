#include "petunia_bound_form.h"
#include "petunia_number_field.h"
#include "petunia_overlay_cards.h"
#include "petunia_theme.h"
#include "petunia_vector3_field.h"
#include <QApplication>
#include <QCheckBox>
#include <QColorDialog>
#include <QComboBox>
#include <QDoubleSpinBox>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPixmap>
#include <QPushButton>
#include <QSignalBlocker>

PetuniaBoundForm::PetuniaBoundForm(QWidget *parent) : QWidget(parent) {}

void PetuniaBoundForm::send(const QString &action, QJsonObject args) {
    if (m_syncing)
        return;
    const auto state = m_transport ? m_transport(action, args) : petuniaRequest(action, args);
    sync(state);
    emit editorChanged();
}
void PetuniaBoundForm::button(QFormLayout *form, const QString &label,
                                const QString &action, QJsonObject args,
                                const QString &enabled) {
    auto *widget = new QPushButton(label, this);
    widget->setAccessibleName(label);
    form->addRow(widget);
    if (action == "command")
        widget->setProperty("commandId", args["id"].toString());
    connect(widget, &QPushButton::clicked, this,
            [=, this] { send(action, args); });
    m_bindings.append({widget, {}, enabled, {}});
}
void PetuniaBoundForm::number(QFormLayout *form, const QString &label,
                                const QString &key, const QString &action,
                                const QString &arg, qreal min, qreal max,
                                qreal step, QJsonObject args) {
    auto *widget = new PetuniaNumberField(this);
    widget->setRange(min, max);
    widget->setSingleStep(step);
    widget->setLabel(label);
    form->addRow(label, widget);
    connect(widget, &PetuniaNumberField::commitRequested, this,
            [=, this](const QString &, double val, const QString &) mutable {
                args.insert(arg, val);
                send(action, args);
            });
    m_bindings.append({widget, key, {}, {}});
}
void PetuniaBoundForm::check(QFormLayout *form, const QString &label,
                               const QString &key, const QString &action,
                               const QString &arg, QJsonObject args) {
    auto *widget = new QCheckBox(label, this);
    widget->setAccessibleName(label);
    form->addRow(widget);
    connect(widget, &QCheckBox::toggled, this, [=, this](bool value) mutable {
        if (!arg.isEmpty())
            args.insert(arg, value);
        send(action, args);
    });
    m_bindings.append({widget, key, {}, {}});
}
void PetuniaBoundForm::choice(QFormLayout *form, const QString &label,
                                const QString &key, const QString &action,
                                const QString &arg, const QStringList &ids,
                                const QStringList &labels, QJsonObject args) {
    auto *widget = new QComboBox(this);
    widget->setAccessibleName(label);
    for (int i = 0; i < ids.size(); ++i)
        widget->addItem(labels.value(i, ids[i]), ids[i]);
    form->addRow(label, widget);
    connect(widget, &QComboBox::activated, this, [=, this](int index) mutable {
        args.insert(arg, widget->itemData(index).toString());
        send(action, args);
    });
    m_bindings.append({widget, key, {}, {}});
}
void PetuniaBoundForm::text(QFormLayout *form, const QString &label,
                              const QString &key, const QString &action,
                              const QString &arg, QJsonObject args) {
    auto *widget = new QLineEdit(this);
    widget->setAccessibleName(label);
    form->addRow(label, widget);
    connect(widget, &QLineEdit::editingFinished, this, [=, this]() mutable {
        args.insert(arg, widget->text());
        send(action, args);
    });
    m_bindings.append({widget, key, {}, {}});
}
void PetuniaBoundForm::colorField(QFormLayout *form, const QString &label,
                                    const QString &key, const QString &action,
                                    const QString &arg, QJsonObject args) {
    auto *container = new QWidget(this);
    auto *layout = new QHBoxLayout(container);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);

    auto *edit = new QLineEdit(container);
    edit->setAccessibleName(label);
    layout->addWidget(edit, 1);

    auto *btn = new QPushButton(container);
    btn->setFixedSize(26, 24);
    btn->setToolTip(tr("Selecionar cor na roda de cores"));
    btn->setCursor(Qt::PointingHandCursor);
    layout->addWidget(btn);

    form->addRow(label, container);

    auto updateBtn = [btn, edit]() {
        QColor c(edit->text());
        if (c.isValid()) {
            btn->setStyleSheet(QStringLiteral("background-color: %1; border: 1px solid #555555; border-radius: 3px;").arg(c.name()));
        }
    };

    connect(edit, &QLineEdit::editingFinished, this, [=, this]() mutable {
        updateBtn();
        args.insert(arg, edit->text());
        send(action, args);
    });

    connect(btn, &QPushButton::clicked, this, [=, this]() mutable {
        QColor initial(edit->text());
        if (!initial.isValid()) initial = QColor("#b58cff");
        QColor chosen = QColorDialog::getColor(initial, this, label);
        if (chosen.isValid()) {
            edit->setText(chosen.name());
            updateBtn();
            args.insert(arg, chosen.name());
            send(action, args);
        }
    });

    m_bindings.append({edit, key, {}, {}});
}
void PetuniaBoundForm::xyz(QFormLayout *form, const QString &label,
                             const QString &keyPrefix, const QString &action,
                             const QString &arg, QJsonObject baseArgs) {
    auto *v3 = new PetuniaVector3Field(this);
    v3->setLabel(label);
    if (keyPrefix == QStringLiteral("position") || keyPrefix == QStringLiteral("bone_translation")) {
        v3->setSuffix(QStringLiteral("m"));
    } else if (keyPrefix == QStringLiteral("rotation") || keyPrefix == QStringLiteral("bone_rotation")) {
        v3->setSuffix(QStringLiteral("°"));
        v3->setSingleStep(1.0);
    } else if (keyPrefix == QStringLiteral("scale") || keyPrefix == QStringLiteral("bone_scale")) {
        v3->setSingleStep(0.1);
    }

    for (int axis = 0; axis < 3; ++axis) {
        QJsonObject args = baseArgs;
        if (action == "transform") {
            args.insert("kind", keyPrefix);
            args.insert("axis", axis);
        } else if (action == "set_cursor_3d_coord") {
            args.insert("index", axis);
        } else if (action == "bone_transform") {
            args.insert("kind", keyPrefix.startsWith("bone_") ? keyPrefix.mid(5) : keyPrefix);
            args.insert("axis", axis);
        }

        connect(v3, &PetuniaVector3Field::commitRequested, this, [=, this](int editedAxis, double val, const QString &rawText) mutable {
            if (editedAxis != axis) return;
            if (arg == "val" || arg == "value") {
                args.insert(arg, val);
                send(action, args);
            } else {
                args.insert(arg, rawText.isEmpty() ? QString::number(val, 'g', 10) : rawText);
                send(action, args);
            }
        });
    }

    m_bindings.append({v3, keyPrefix, {}, {}});
    form->addRow(label, v3);
}
void PetuniaBoundForm::rows(QFormLayout *form, const QString &label,
                              const QString &key, const QString &action,
                              const QString &arg, QJsonObject args) {
    auto *widget = new QListWidget(this);
    widget->setAccessibleName(label);
    widget->setMinimumHeight(90);
    form->addRow(label, widget);
    connect(widget, &QListWidget::itemClicked, this,
            [=, this](QListWidgetItem *item) mutable {
                args.insert(arg, item->data(Qt::UserRole).toString());
                send(action, args);
            });
    m_rows.append({widget, key, action, arg, args});
}
void PetuniaBoundForm::fileButton(QFormLayout *form, const QString &label,
                                    const QString &action,
                                    const QString &filter, bool save,
                                    QJsonObject args) {
    auto *widget = new QPushButton(label, this);
    widget->setAccessibleName(label);
    form->addRow(widget);
    connect(widget, &QPushButton::clicked, this, [=, this]() mutable {
        const auto path =
            save ? QFileDialog::getSaveFileName(this, label, {}, filter)
                 : QFileDialog::getOpenFileName(this, label, {}, filter);
        if (!path.isEmpty()) {
            args.insert("path", path);
            send(action, args);
        }
    });
}
void PetuniaBoundForm::sync(const QJsonObject &state) {
    if (state.isEmpty())
        return;
    m_syncing = true;
    m_state = state;
    for (const auto &binding : m_bindings) {
        const QSignalBlocker block(binding.widget);
        const auto refAxis =
            binding.widget->property("referenceAxis").toString();
        if (!refAxis.isEmpty()) {
            const auto ref = state["reference_" + refAxis].toObject();
            const auto signature =
                QJsonDocument(ref).toJson(QJsonDocument::Compact);
            if (binding.widget->property("referenceSnapshot").toByteArray() !=
                signature) {
                binding.widget->setProperty("referenceSnapshot", signature);
                auto *label = qobject_cast<QLabel *>(binding.widget);
                const auto image = petuniaImage("reference", refAxis);
                label->setPixmap(QPixmap::fromImage(image).scaled(
                    240, 140, Qt::KeepAspectRatio, Qt::SmoothTransformation));
            }
        }

        if (!binding.enabledKey.isEmpty())
            binding.widget->setEnabled(
                petuniaValue(state, binding.enabledKey).toBool());
        const auto command = binding.widget->property("commandId").toString();
        if (!command.isEmpty()) {
            bool found = false;
            for (const auto &c : state["commands"].toArray()) {
                const auto cmd = c.toObject();
                if (cmd["id"] == command) {
                    binding.widget->setEnabled(cmd["enabled"].toBool());
                    binding.widget->setToolTip(cmd["reason"].toString());
                    found = true;
                    break;
                }
            }
            if (!found) {
                binding.widget->setEnabled(false);
                binding.widget->setToolTip(
                    tr("Comando indisponível nesta versão"));
            }
        }
        if (binding.key.isEmpty() || binding.widget->hasFocus() || binding.widget->isAncestorOf(QApplication::focusWidget()))
            continue;
        const auto value = petuniaValue(state, binding.key);
        if (binding.key.startsWith("primitive_")) {
            const auto allowed = state["primitive_parameters"].toArray();
            const bool visible = allowed.contains(binding.key.mid(10));
            binding.widget->setVisible(visible);
            if (auto *form = qobject_cast<QFormLayout *>(
                    binding.widget->parentWidget()->layout()))
                if (auto *label = form->labelForField(binding.widget))
                    label->setVisible(visible);
        }

        if (auto *w = qobject_cast<PetuniaVector3Field *>(binding.widget)) {
            const auto array = value.toArray();
            if (array.size() >= 3) {
                w->setValues(array.at(0).toDouble(), array.at(1).toDouble(), array.at(2).toDouble());
            } else if (value.isString() && value.toString() == QStringLiteral("mixed")) {
                w->setMixed(0, true);
                w->setMixed(1, true);
                w->setMixed(2, true);
            }
        } else if (auto *w = qobject_cast<PetuniaNumberField *>(binding.widget)) {
            if (value.isString() && value.toString() == QStringLiteral("—")) {
                w->setMixed(true);
            } else {
                w->setValue(value.toDouble());
            }
        } else if (auto *w = qobject_cast<QDoubleSpinBox *>(binding.widget))
            w->setValue(value.toDouble());
        else if (auto *w = qobject_cast<QCheckBox *>(binding.widget))
            w->setChecked(value.toBool());
        else if (auto *w = qobject_cast<QLineEdit *>(binding.widget))
            w->setText(value.isString()
                           ? value.toString()
                           : QString::number(value.toDouble(), 'g', 8));
        else if (auto *w = qobject_cast<QComboBox *>(binding.widget)) {
            const auto index =
                w->findData(value.isString() ? value.toString()
                                             : QString::number(value.toInt()));
            if (index >= 0)
                w->setCurrentIndex(index);
        }
    }
    for (const auto &binding : m_rows) {
        const auto array = petuniaValue(state, binding.key).toArray();
        const auto bytes = QJsonDocument(array).toJson(QJsonDocument::Compact);
        if (binding.widget->property("snapshot").toByteArray() == bytes)
            continue;
        const QSignalBlocker block(binding.widget);
        binding.widget->setProperty("snapshot", bytes);
        binding.widget->clear();
        for (const auto &value : array) {
            auto obj = value.toObject();
            auto *item = new QListWidgetItem(
                obj["label"].toString(obj["name"].toString()), binding.widget);
            if (obj["has_thumbnail"].toBool()) {
                auto icon = petuniaImage("prefab", obj["id"].toString());
                if (!icon.isNull()) {
                    item->setIcon(QPixmap::fromImage(icon));
                    binding.widget->setIconSize(QSize(64, 64));
                }
            }
            item->setData(Qt::UserRole, obj["id"].toString());
            item->setToolTip(
                obj["detail"].toString(obj["disabled_reason"].toString()));
            if ((obj.contains("available") && !obj["available"].toBool()) ||
                (obj.contains("enabled") && !obj["enabled"].toBool() &&
                 binding.action == "execute_quick_action"))
                item->setFlags(item->flags() & ~Qt::ItemIsEnabled);
            if (obj["selected"].toBool() || obj["active"].toBool())
                binding.widget->setCurrentItem(item);
        }
    }
    afterSync(state);
    petuniaLocalize(this);
    m_syncing = false;
}

QStringList
PetuniaBoundForm::missingBindings(const QJsonObject &state) const {
    QStringList missing;
    auto verify = [&](const QString &key) {
        if (!key.isEmpty() && petuniaValue(state, key).isUndefined())
            missing.append(key);
    };
    for (const auto &binding : m_bindings) {
        verify(binding.enabledKey);
        if (binding.enabledKey.isEmpty() ||
            petuniaValue(state, binding.enabledKey).toBool())
            verify(binding.key);
    }
    for (const auto &binding : m_rows)
        verify(binding.key);
    missing.removeDuplicates();
    return missing;
}
