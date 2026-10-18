#include "petunia_recovery_dialog.h"
#include "petunia_theme.h"
#include <QFrame>
#include <QGraphicsDropShadowEffect>

PetuniaRecoveryDialog::PetuniaRecoveryDialog(QWidget *parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint) {
    setupUi();
}

// Cores, fontes e alvos vêm do tema: "Recuperar" é a ação primária e
// "Descartar" usa o papel de perigo (irreversível), sem literais de cor.
void PetuniaRecoveryDialog::setupUi() {
    const auto &theme = PetuniaTheme::instance();
    setWindowTitle(tr("Recuperar sessão"));
    resize(theme.scaled(480), theme.scaled(240));
    setMinimumSize(theme.scaled(280), theme.scaled(220));
    setAttribute(Qt::WA_TranslucentBackground);

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(theme.space(3), theme.space(3), theme.space(3), theme.space(3));

    auto *card = new QFrame(this);
    card->setProperty("role", QStringLiteral("card"));
    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(24);
    shadow->setColor(QColor(0, 0, 0, 160));
    shadow->setOffset(0, 6);
    card->setGraphicsEffect(shadow);

    auto *cardLayout = new QVBoxLayout(card);
    cardLayout->setContentsMargins(theme.space(5), theme.space(5), theme.space(5), theme.space(5));
    cardLayout->setSpacing(theme.space(3));

    m_titleLabel = new QLabel(tr("Recuperar sessão não salva"), card);
    m_titleLabel->setProperty("role", QStringLiteral("title"));
    cardLayout->addWidget(m_titleLabel);

    m_bodyLabel = new QLabel(tr("Foi encontrado um salvamento automático de uma sessão anterior interrompida."), card);
    m_bodyLabel->setWordWrap(true);
    cardLayout->addWidget(m_bodyLabel);

    m_detailLabel = new QLabel(tr("Deseja restaurar as modificações ou descartar a recuperação?"), card);
    m_detailLabel->setProperty("role", QStringLiteral("muted"));
    m_detailLabel->setWordWrap(true);
    cardLayout->addWidget(m_detailLabel);
    cardLayout->addStretch();

    auto *buttonsLayout = new QHBoxLayout();
    buttonsLayout->setSpacing(theme.space(2));

    m_discardBtn = new QPushButton(tr("Descartar"), card);
    m_discardBtn->setProperty("role", QStringLiteral("danger"));
    m_discardBtn->setMinimumHeight(theme.controlHeight());
    m_discardBtn->setToolTip(tr("Apaga o salvamento automático; não pode ser desfeito"));
    connect(m_discardBtn, &QPushButton::clicked, this, [this]() {
        emit discardRequested();
        reject();
    });
    buttonsLayout->addWidget(m_discardBtn);
    buttonsLayout->addStretch();

    m_keepBtn = new QPushButton(tr("Decidir depois"), card);
    m_keepBtn->setMinimumHeight(theme.controlHeight());
    m_keepBtn->setToolTip(tr("Mantém o salvamento automático para recuperar mais tarde"));
    connect(m_keepBtn, &QPushButton::clicked, this, [this]() {
        emit keepRequested();
        accept();
    });
    buttonsLayout->addWidget(m_keepBtn);

    m_recoverBtn = new QPushButton(tr("Recuperar"), card);
    m_recoverBtn->setProperty("role", QStringLiteral("primary"));
    m_recoverBtn->setMinimumHeight(theme.controlHeight());
    m_recoverBtn->setDefault(true);
    connect(m_recoverBtn, &QPushButton::clicked, this, [this]() {
        emit recoverRequested();
        accept();
    });
    buttonsLayout->addWidget(m_recoverBtn);

    cardLayout->addLayout(buttonsLayout);
    mainLayout->addWidget(card);
    // Ação segura com foco inicial: Enter recupera, nunca descarta.
    m_recoverBtn->setFocus();
}

void PetuniaRecoveryDialog::setSessionDetails(const QString &title,
                                              const QString &body,
                                              const QString &detail) {
    if (!title.isEmpty())
        m_titleLabel->setText(title);
    if (!body.isEmpty())
        m_bodyLabel->setText(body);
    if (!detail.isEmpty())
        m_detailLabel->setText(detail);
}
