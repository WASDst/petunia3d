#include "petunia_property_section.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"

#include <QHBoxLayout>
#include <QSignalBlocker>

PetuniaPropertySection::PetuniaPropertySection(const QString &id, const QString &title,
                                               QWidget *content, QWidget *parent)
    : QFrame(parent), m_id(id), m_title(title), m_content(content) {
    setObjectName(QStringLiteral("PetuniaPropertySection"));
    setProperty("role", QStringLiteral("section"));
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    auto *headerRow = new QWidget(this);
    auto *headerLayout = new QHBoxLayout(headerRow);
    headerLayout->setContentsMargins(0, 0, 0, 0);
    headerLayout->setSpacing(0);

    m_header = new QToolButton(headerRow);
    m_header->setProperty("role", QStringLiteral("sectionHeader"));
    m_header->setCheckable(true);
    m_header->setChecked(true);
    m_header->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_header->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    m_header->setFocusPolicy(Qt::StrongFocus);
    headerLayout->addWidget(m_header, 1);

    m_actionHost = new QWidget(headerRow);
    auto *actionLayout = new QHBoxLayout(m_actionHost);
    actionLayout->setContentsMargins(0, 0, PetuniaTheme::instance().space(2), 0);
    actionLayout->setSpacing(0);
    m_actionHost->hide();
    headerLayout->addWidget(m_actionHost, 0);

    root->addWidget(headerRow);
    if (m_content) {
        m_content->setParent(this);
        root->addWidget(m_content);
    }

    connect(m_header, &QToolButton::toggled, this, [this](bool open) {
        applyOpen(open);
        emit toggled(m_id, open);
    });
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaPropertySection::refreshHeader);
    refreshHeader();
}

void PetuniaPropertySection::applyOpen(bool open) {
    m_open = open;
    if (m_content)
        m_content->setVisible(open);
    refreshHeader();
}

void PetuniaPropertySection::setOpen(bool open) {
    if (m_open == open)
        return;
    const QSignalBlocker block(m_header);
    m_header->setChecked(open);
    applyOpen(open);
}

void PetuniaPropertySection::setTitle(const QString &title) {
    m_title = title;
    refreshHeader();
}

void PetuniaPropertySection::setSummary(const QString &summary) {
    if (m_summary == summary)
        return;
    m_summary = summary;
    refreshHeader();
}

void PetuniaPropertySection::setHeaderAction(QWidget *action) {
    auto *layout = static_cast<QHBoxLayout *>(m_actionHost->layout());
    while (auto *item = layout->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    if (action) {
        action->setParent(m_actionHost);
        layout->addWidget(action);
    }
    m_actionHost->setVisible(action != nullptr);
}

void PetuniaPropertySection::refreshHeader() {
    const auto &theme = PetuniaTheme::instance();
    m_header->setIcon(PetuniaIcons::get(m_open ? QStringLiteral("chevron_down") : QStringLiteral("chevron_right")));
    m_header->setIconSize(QSize(theme.iconPx() - 2, theme.iconPx() - 2));
    m_header->setText(m_summary.isEmpty() ? m_title : m_title + QStringLiteral("  ·  ") + m_summary);
    m_header->setAccessibleName(m_title);
    m_header->setAccessibleDescription(m_open ? tr("Expandida") : tr("Recolhida"));
    m_header->setToolTip(m_open ? tr("Recolher %1").arg(m_title) : tr("Expandir %1").arg(m_title));
}
