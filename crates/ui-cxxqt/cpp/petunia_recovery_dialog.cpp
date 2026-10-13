#include "petunia_recovery_dialog.h"
#include <QGraphicsDropShadowEffect>

PetuniaRecoveryDialog::PetuniaRecoveryDialog(QWidget *parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint)
{
    setupUi();
}

void PetuniaRecoveryDialog::setupUi() {
    setFixedSize(480, 240);
    setAttribute(Qt::WA_TranslucentBackground);

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(12, 12, 12, 12);

    auto *card = new QWidget(this);
    card->setObjectName(QStringLiteral("RecoveryCard"));
    card->setStyleSheet(QStringLiteral(
        "#RecoveryCard { background-color: #202020; border: 1px solid #454545; border-radius: 8px; }"
    ));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(24);
    shadow->setColor(QColor(0, 0, 0, 180));
    shadow->setOffset(0, 6);
    card->setGraphicsEffect(shadow);

    auto *cardLayout = new QVBoxLayout(card);
    cardLayout->setContentsMargins(20, 20, 20, 20);
    cardLayout->setSpacing(12);

    m_titleLabel = new QLabel(QStringLiteral("Recuperar Sessão Não Salva"), card);
    m_titleLabel->setStyleSheet(QStringLiteral("font-size: 16px; font-weight: bold; color: #e96a00;"));
    cardLayout->addWidget(m_titleLabel);

    m_bodyLabel = new QLabel(QStringLiteral("Foi detectado um salvamento automático de uma sessão anterior interrompida."), card);
    m_bodyLabel->setStyleSheet(QStringLiteral("font-size: 12px; color: #cccccc;"));
    m_bodyLabel->setWordWrap(true);
    cardLayout->addWidget(m_bodyLabel);

    m_detailLabel = new QLabel(QStringLiteral("Deseja restaurar as modificações ou descartar a recuperação?"), card);
    m_detailLabel->setStyleSheet(QStringLiteral("font-size: 11px; color: #888888;"));
    m_detailLabel->setWordWrap(true);
    cardLayout->addWidget(m_detailLabel);

    cardLayout->addStretch();

    auto *buttonsLayout = new QHBoxLayout();
    buttonsLayout->setSpacing(8);

    m_discardBtn = new QPushButton(QStringLiteral("Descartar"), card);
    m_discardBtn->setFixedHeight(32);
    m_discardBtn->setCursor(Qt::PointingHandCursor);
    m_discardBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #381a1a; color: #ff6b6b; font-size: 12px; border: 1px solid #6b2a2a; border-radius: 4px; padding: 0 14px; }"
        "QPushButton:hover { background-color: #4d2222; }"
    ));
    connect(m_discardBtn, &QPushButton::clicked, this, [this]() {
        emit discardRequested();
        reject();
    });
    buttonsLayout->addWidget(m_discardBtn);

    buttonsLayout->addStretch();

    m_keepBtn = new QPushButton(QStringLiteral("Manter"), card);
    m_keepBtn->setFixedHeight(32);
    m_keepBtn->setCursor(Qt::PointingHandCursor);
    m_keepBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #292929; color: #cccccc; font-size: 12px; border: 1px solid #404040; border-radius: 4px; padding: 0 14px; }"
        "QPushButton:hover { background-color: #333333; }"
    ));
    connect(m_keepBtn, &QPushButton::clicked, this, [this]() {
        emit keepRequested();
        accept();
    });
    buttonsLayout->addWidget(m_keepBtn);

    m_recoverBtn = new QPushButton(QStringLiteral("Recuperar"), card);
    m_recoverBtn->setFixedHeight(32);
    m_recoverBtn->setCursor(Qt::PointingHandCursor);
    m_recoverBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #b58cff; color: #121212; font-weight: bold; font-size: 12px; border: none; border-radius: 4px; padding: 0 16px; }"
        "QPushButton:hover { background-color: #c9abff; }"
    ));
    connect(m_recoverBtn, &QPushButton::clicked, this, [this]() {
        emit recoverRequested();
        accept();
    });
    buttonsLayout->addWidget(m_recoverBtn);

    cardLayout->addLayout(buttonsLayout);
    mainLayout->addWidget(card);
}

void PetuniaRecoveryDialog::setSessionDetails(const QString &title, const QString &body, const QString &detail) {
    if (!title.isEmpty()) m_titleLabel->setText(title);
    if (!body.isEmpty()) m_bodyLabel->setText(body);
    if (!detail.isEmpty()) m_detailLabel->setText(detail);
}
