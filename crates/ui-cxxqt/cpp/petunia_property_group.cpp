#include "petunia_property_group.h"
#include <QStyle>

PropertyRow::PropertyRow(const QString &label, QWidget *editorWidget, QWidget *parent)
    : QWidget(parent)
    , m_editor(editorWidget) {
    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 2, 0, 2);
    layout->setSpacing(6);

    m_label = new QLabel(label, this);
    m_label->setMinimumWidth(72);
    m_label->setMaximumWidth(110);
    m_label->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    layout->addWidget(m_label);

    if (m_editor) {
        layout->addWidget(m_editor, 1);
    }
}

PropertyGroup::PropertyGroup(const QString &title, QWidget *parent)
    : QWidget(parent)
    , m_title(title) {
    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(0, 0, 0, 4);
    rootLayout->setSpacing(0);

    // Header bar
    m_headerWidget = new QWidget(this);
    m_headerWidget->setObjectName(QStringLiteral("PropertyGroupHeader"));
    m_headerWidget->setCursor(Qt::PointingHandCursor);

    auto *headerLayout = new QHBoxLayout(m_headerWidget);
    headerLayout->setContentsMargins(4, 4, 4, 4);
    headerLayout->setSpacing(4);

    m_toggleButton = new QToolButton(m_headerWidget);
    m_toggleButton->setText(QStringLiteral("▼"));
    m_toggleButton->setAutoRaise(true);
    m_toggleButton->setFixedSize(16, 16);
    headerLayout->addWidget(m_toggleButton);

    m_titleLabel = new QLabel(title, m_headerWidget);
    QFont font = m_titleLabel->font();
    font.setBold(true);
    font.setPointSize(10);
    m_titleLabel->setFont(font);
    headerLayout->addWidget(m_titleLabel, 1);

    rootLayout->addWidget(m_headerWidget);

    // Content container
    m_contentContainer = new QWidget(this);
    m_contentLayout = new QVBoxLayout(m_contentContainer);
    m_contentLayout->setContentsMargins(12, 2, 2, 6);
    m_contentLayout->setSpacing(4);
    rootLayout->addWidget(m_contentContainer);

    connect(m_toggleButton, &QToolButton::clicked, this, [this]() {
        setExpanded(!m_expanded);
    });

    // Make clicking the header toggle expansion
    m_headerWidget->installEventFilter(this);
}

void PropertyGroup::setTitle(const QString &title) {
    m_title = title;
    if (m_titleLabel) {
        m_titleLabel->setText(title);
    }
}

void PropertyGroup::setExpanded(bool expanded) {
    if (m_expanded != expanded) {
        m_expanded = expanded;
        m_contentContainer->setVisible(m_expanded);
        m_toggleButton->setText(m_expanded ? QStringLiteral("▼") : QStringLiteral("▶"));
        emit toggled(m_expanded);
    }
}

void PropertyGroup::addRow(const QString &label, QWidget *widget) {
    auto *row = new PropertyRow(label, widget, m_contentContainer);
    m_contentLayout->addWidget(row);
}

void PropertyGroup::addWidget(QWidget *widget) {
    m_contentLayout->addWidget(widget);
}

void PropertyGroup::addLayout(QLayout *layout) {
    m_contentLayout->addLayout(layout);
}
