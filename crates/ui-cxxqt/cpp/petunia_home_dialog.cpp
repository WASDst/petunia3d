#include "petunia_home_dialog.h"
#include <QGraphicsDropShadowEffect>
#include <QKeyEvent>

PetuniaHomeDialog::PetuniaHomeDialog(QWidget *parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint)
{
    setupUi();
}

void PetuniaHomeDialog::setupUi() {
    setFixedSize(580, 480);
    setAttribute(Qt::WA_TranslucentBackground);

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(12, 12, 12, 12);

    auto *card = new QWidget(this);
    card->setObjectName(QStringLiteral("HomeCard"));
    card->setStyleSheet(QStringLiteral(
        "#HomeCard { background-color: #202020; border: 1px solid #383838; border-radius: 8px; }"
    ));

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(28);
    shadow->setColor(QColor(0, 0, 0, 190));
    shadow->setOffset(0, 8);
    card->setGraphicsEffect(shadow);

    auto *cardLayout = new QVBoxLayout(card);
    cardLayout->setContentsMargins(24, 24, 24, 24);
    cardLayout->setSpacing(14);

    // Header: Título & Subtítulo
    auto *headerLayout = new QHBoxLayout();
    auto *titleCol = new QVBoxLayout();

    auto *titleLabel = new QLabel(QStringLiteral("Petunia3D Studio"), card);
    titleLabel->setStyleSheet(QStringLiteral("font-size: 20px; font-weight: bold; color: #b58cff;"));
    titleCol->addWidget(titleLabel);

    auto *subLabel = new QLabel(QStringLiteral("Modelagem 3D, Pintura de Texturas e Animação Low-Poly"), card);
    subLabel->setStyleSheet(QStringLiteral("font-size: 12px; color: #a0a0a0;"));
    titleCol->addWidget(subLabel);

    headerLayout->addLayout(titleCol);
    headerLayout->addStretch();

    m_closeBtn = new QPushButton(QStringLiteral("✕"), card);
    m_closeBtn->setFixedSize(28, 28);
    m_closeBtn->setCursor(Qt::PointingHandCursor);
    m_closeBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background: transparent; color: #888888; font-size: 14px; border: none; border-radius: 4px; }"
        "QPushButton:hover { background-color: #333333; color: #ffffff; }"
    ));
    connect(m_closeBtn, &QPushButton::clicked, this, &QDialog::accept);
    headerLayout->addWidget(m_closeBtn);

    cardLayout->addLayout(headerLayout);

    // Divisor
    auto *sep1 = new QFrame(card);
    sep1->setFrameShape(QFrame::HLine);
    sep1->setStyleSheet(QStringLiteral("background-color: #303030; max-height: 1px;"));
    cardLayout->addWidget(sep1);

    // Ações Rápidas (Novo, Abrir, Recuperar)
    auto *actionsLayout = new QHBoxLayout();
    actionsLayout->setSpacing(10);

    m_newBtn = new QPushButton(QStringLiteral("+ Novo Projeto"), card);
    m_newBtn->setFixedHeight(36);
    m_newBtn->setCursor(Qt::PointingHandCursor);
    m_newBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #b58cff; color: #121212; font-weight: bold; font-size: 13px; border: none; border-radius: 4px; padding: 0 16px; }"
        "QPushButton:hover { background-color: #c9abff; }"
    ));
    connect(m_newBtn, &QPushButton::clicked, this, [this]() {
        emit newProjectRequested();
        accept();
    });
    actionsLayout->addWidget(m_newBtn);

    m_openBtn = new QPushButton(QStringLiteral("Abrir Projeto..."), card);
    m_openBtn->setFixedHeight(36);
    m_openBtn->setCursor(Qt::PointingHandCursor);
    m_openBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #292929; color: #eeeeee; font-size: 13px; border: 1px solid #404040; border-radius: 4px; padding: 0 16px; }"
        "QPushButton:hover { background-color: #353535; border-color: #555555; }"
    ));
    connect(m_openBtn, &QPushButton::clicked, this, [this]() {
        emit openProjectRequested();
        accept();
    });
    actionsLayout->addWidget(m_openBtn);

    m_recoverBtn = new QPushButton(QStringLiteral("Recuperar Sessão..."), card);
    m_recoverBtn->setFixedHeight(36);
    m_recoverBtn->setCursor(Qt::PointingHandCursor);
    m_recoverBtn->setVisible(false);
    m_recoverBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #e96a00; color: #ffffff; font-weight: bold; font-size: 13px; border: none; border-radius: 4px; padding: 0 16px; }"
        "QPushButton:hover { background-color: #ff8522; }"
    ));
    connect(m_recoverBtn, &QPushButton::clicked, this, [this]() {
        emit recoverProjectRequested();
        accept();
    });
    actionsLayout->addWidget(m_recoverBtn);

    actionsLayout->addStretch();
    cardLayout->addLayout(actionsLayout);

    // Seção de Projetos Recentes
    auto *recentTitle = new QLabel(QStringLiteral("Projetos Recentes"), card);
    recentTitle->setStyleSheet(QStringLiteral("font-size: 13px; font-weight: 600; color: #dddddd; margin-top: 4px;"));
    cardLayout->addWidget(recentTitle);

    m_recentList = new QListWidget(card);
    m_recentList->setStyleSheet(QStringLiteral(
        "QListWidget { background-color: #1a1a1a; border: 1px solid #2e2e2e; border-radius: 6px; padding: 4px; }"
        "QListWidget::item { height: 36px; padding: 4px 8px; border-radius: 4px; color: #dddddd; }"
        "QListWidget::item:hover { background-color: #2a2a2a; color: #ffffff; }"
        "QListWidget::item:selected { background-color: #383838; color: #b58cff; font-weight: bold; }"
    ));
    connect(m_recentList, &QListWidget::itemDoubleClicked, this, &PetuniaHomeDialog::onRecentItemDoubleClicked);
    cardLayout->addWidget(m_recentList);

    m_noRecentLabel = new QLabel(QStringLiteral("Nenhum projeto recente."), card);
    m_noRecentLabel->setStyleSheet(QStringLiteral("color: #777777; font-size: 12px; font-style: italic;"));
    m_noRecentLabel->setAlignment(Qt::AlignCenter);
    m_noRecentLabel->setVisible(true);
    cardLayout->addWidget(m_noRecentLabel);

    // Divisor
    auto *sep2 = new QFrame(card);
    sep2->setFrameShape(QFrame::HLine);
    sep2->setStyleSheet(QStringLiteral("background-color: #303030; max-height: 1px;"));
    cardLayout->addWidget(sep2);

    // Rodapé
    auto *footerLayout = new QHBoxLayout();

    m_showOnStartCheck = new QCheckBox(QStringLiteral("Mostrar tela inicial ao iniciar"), card);
    m_showOnStartCheck->setChecked(true);
    m_showOnStartCheck->setStyleSheet(QStringLiteral(
        "QCheckBox { color: #aaaaaa; font-size: 12px; }"
        "QCheckBox::indicator { width: 14px; height: 14px; border: 1px solid #505050; border-radius: 3px; background: #222; }"
        "QCheckBox::indicator:checked { background: #b58cff; border-color: #b58cff; }"
    ));
    footerLayout->addWidget(m_showOnStartCheck);

    footerLayout->addStretch();

    m_settingsBtn = new QPushButton(QStringLiteral("Configurações"), card);
    m_settingsBtn->setCursor(Qt::PointingHandCursor);
    m_settingsBtn->setStyleSheet(QStringLiteral(
        "QPushButton { background: transparent; color: #aaaaaa; font-size: 12px; border: 1px solid #404040; border-radius: 4px; padding: 4px 12px; }"
        "QPushButton:hover { background: #2a2a2a; color: #ffffff; border-color: #555555; }"
    ));
    connect(m_settingsBtn, &QPushButton::clicked, this, [this]() {
        emit settingsRequested();
    });
    footerLayout->addWidget(m_settingsBtn);

    cardLayout->addLayout(footerLayout);
    mainLayout->addWidget(card);
}

void PetuniaHomeDialog::setRecentProjects(const QStringList &projectNames, const QStringList &projectPaths) {
    m_recentList->clear();
    int count = std::min(projectNames.size(), projectPaths.size());
    for (int i = 0; i < count; ++i) {
        auto *item = new QListWidgetItem(QStringLiteral("%1  (%2)").arg(projectNames[i], projectPaths[i]), m_recentList);
        item->setData(Qt::UserRole, projectPaths[i]);
        m_recentList->addItem(item);
    }

    bool hasRecent = count > 0;
    m_recentList->setVisible(hasRecent);
    m_noRecentLabel->setVisible(!hasRecent);
}

void PetuniaHomeDialog::setRecoveryAvailable(bool available) {
    m_recoverBtn->setVisible(available);
}

bool PetuniaHomeDialog::showOnStart() const {
    return m_showOnStartCheck->isChecked();
}

void PetuniaHomeDialog::setShowOnStart(bool show) {
    m_showOnStartCheck->setChecked(show);
}

void PetuniaHomeDialog::onRecentItemDoubleClicked(QListWidgetItem *item) {
    if (!item) return;
    QString path = item->data(Qt::UserRole).toString();
    if (!path.isEmpty()) {
        emit recentProjectSelected(path);
        accept();
    }
}
