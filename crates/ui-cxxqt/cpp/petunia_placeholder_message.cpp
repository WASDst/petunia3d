#include "petunia_placeholder_message.h"
#include "petunia_theme.h"

PetuniaPlaceholderMessage::PetuniaPlaceholderMessage(QWidget *parent)
    : QWidget(parent)
{
    const auto &theme = PetuniaTheme::instance();
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(theme.space(6), theme.space(8), theme.space(6), theme.space(8));
    layout->setSpacing(theme.space(2));
    layout->setAlignment(Qt::AlignCenter);

    m_iconLabel = new QLabel(this);
    m_iconLabel->setAlignment(Qt::AlignCenter);
    m_iconLabel->setFixedSize(theme.scaled(48), theme.scaled(48));
    m_iconLabel->setAccessibleName(QString());
    layout->addWidget(m_iconLabel, 0, Qt::AlignCenter);

    m_titleLabel = new QLabel(this);
    m_titleLabel->setAlignment(Qt::AlignCenter);
    m_titleLabel->setProperty("role", QStringLiteral("title"));
    m_titleLabel->setWordWrap(true);
    layout->addWidget(m_titleLabel, 0, Qt::AlignCenter);

    m_explanationLabel = new QLabel(this);
    m_explanationLabel->setAlignment(Qt::AlignCenter);
    m_explanationLabel->setProperty("role", QStringLiteral("muted"));
    m_explanationLabel->setWordWrap(true);
    m_explanationLabel->setMaximumWidth(theme.scaled(320));
    layout->addWidget(m_explanationLabel, 0, Qt::AlignCenter);

    layout->addSpacing(theme.space(1));

    m_actionButton = new QPushButton(this);
    m_actionButton->setMinimumHeight(theme.controlHeight());
    m_actionButton->setVisible(false);
    connect(m_actionButton, &QPushButton::clicked, this, &PetuniaPlaceholderMessage::actionTriggered);
    layout->addWidget(m_actionButton, 0, Qt::AlignCenter);
}

void PetuniaPlaceholderMessage::setIcon(const QIcon &icon) {
    if (!icon.isNull()) {
        const int side = PetuniaTheme::instance().scaled(36);
        m_iconLabel->setPixmap(icon.pixmap(side, side));
        m_iconLabel->setVisible(true);
    } else {
        m_iconLabel->setVisible(false);
    }
}

void PetuniaPlaceholderMessage::setTitle(const QString &title) {
    m_titleLabel->setText(title);
    m_titleLabel->setVisible(!title.isEmpty());
    setAccessibleName(title);
}

void PetuniaPlaceholderMessage::setExplanation(const QString &explanation) {
    m_explanationLabel->setText(explanation);
    m_explanationLabel->setVisible(!explanation.isEmpty());
    setAccessibleDescription(explanation);
}

void PetuniaPlaceholderMessage::setActionText(const QString &text) {
    m_actionButton->setText(text);
    m_actionButton->setVisible(!text.isEmpty());
}

void PetuniaPlaceholderMessage::setActionVisible(bool visible) {
    m_actionButton->setVisible(visible);
}
