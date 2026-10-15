#include "petunia_placeholder_message.h"

PetuniaPlaceholderMessage::PetuniaPlaceholderMessage(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(24, 32, 24, 32);
    layout->setSpacing(8);
    layout->setAlignment(Qt::AlignCenter);

    m_iconLabel = new QLabel(this);
    m_iconLabel->setAlignment(Qt::AlignCenter);
    m_iconLabel->setFixedSize(48, 48);
    m_iconLabel->setStyleSheet(QStringLiteral("background: transparent;"));
    layout->addWidget(m_iconLabel, 0, Qt::AlignCenter);

    m_titleLabel = new QLabel(this);
    m_titleLabel->setAlignment(Qt::AlignCenter);
    m_titleLabel->setStyleSheet(QStringLiteral(
        "color: #eeeeee; font-weight: bold; font-size: 14px; background: transparent;"
    ));
    m_titleLabel->setWordWrap(true);
    layout->addWidget(m_titleLabel, 0, Qt::AlignCenter);

    m_explanationLabel = new QLabel(this);
    m_explanationLabel->setAlignment(Qt::AlignCenter);
    m_explanationLabel->setStyleSheet(QStringLiteral(
        "color: #888888; font-size: 12px; background: transparent;"
    ));
    m_explanationLabel->setWordWrap(true);
    m_explanationLabel->setMaximumWidth(280);
    layout->addWidget(m_explanationLabel, 0, Qt::AlignCenter);

    layout->addSpacing(4);

    m_actionButton = new QPushButton(this);
    m_actionButton->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #353535; color: #eeeeee; border: 1px solid #505050; border-radius: 4px; padding: 6px 14px; font-weight: bold; font-size: 12px; }"
        "QPushButton:hover { background-color: #454545; border-color: #b58cff; }"
        "QPushButton:pressed { background-color: #292929; }"
    ));
    m_actionButton->setVisible(false);
    connect(m_actionButton, &QPushButton::clicked, this, &PetuniaPlaceholderMessage::actionTriggered);
    layout->addWidget(m_actionButton, 0, Qt::AlignCenter);
}

void PetuniaPlaceholderMessage::setIcon(const QIcon &icon) {
    if (!icon.isNull()) {
        m_iconLabel->setPixmap(icon.pixmap(36, 36));
        m_iconLabel->setVisible(true);
    } else {
        m_iconLabel->setVisible(false);
    }
}

void PetuniaPlaceholderMessage::setTitle(const QString &title) {
    m_titleLabel->setText(title);
    m_titleLabel->setVisible(!title.isEmpty());
}

void PetuniaPlaceholderMessage::setExplanation(const QString &explanation) {
    m_explanationLabel->setText(explanation);
    m_explanationLabel->setVisible(!explanation.isEmpty());
}

void PetuniaPlaceholderMessage::setActionText(const QString &text) {
    m_actionButton->setText(text);
    m_actionButton->setVisible(!text.isEmpty());
}

void PetuniaPlaceholderMessage::setActionVisible(bool visible) {
    m_actionButton->setVisible(visible);
}
