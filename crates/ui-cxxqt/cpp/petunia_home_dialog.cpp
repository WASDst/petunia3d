#include "petunia_home_dialog.h"
#include "petunia_dialog_chrome.h"
#include "petunia_icons.h"
#include "petunia_theme.h"
#include <QFrame>
#include <QGraphicsDropShadowEffect>
#include <QKeyEvent>

PetuniaHomeDialog::PetuniaHomeDialog(QWidget *parent)
    : QDialog(parent, Qt::Dialog | Qt::FramelessWindowHint) {
    setupUi();
}

// Aparência vem do tema (papéis da folha de estilo global): segue claro, escuro,
// alto contraste e densidade sem cores fixas.
void PetuniaHomeDialog::setupUi() {
    const auto &theme = PetuniaTheme::instance();
    setWindowTitle(tr("Petunia3D — Início"));
    resize(theme.scaled(580), theme.scaled(480));
    setMinimumSize(theme.scaled(320), theme.scaled(240));
    setAttribute(Qt::WA_TranslucentBackground);

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(theme.space(3), theme.space(3), theme.space(3), theme.space(3));

    auto *card = new QFrame(this);
    card->setProperty("role", QStringLiteral("card"));
    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(28);
    shadow->setColor(QColor(0, 0, 0, 160));
    shadow->setOffset(0, 8);
    card->setGraphicsEffect(shadow);

    auto *cardLayout = new QVBoxLayout(card);
    cardLayout->setContentsMargins(theme.space(6), theme.space(6), theme.space(6), theme.space(6));
    cardLayout->setSpacing(theme.space(3));

    auto *headerLayout = new QHBoxLayout();
    auto *titleCol = new QVBoxLayout();
    auto *titleLabel = new QLabel(QStringLiteral("Petunia3D"), card);
    titleLabel->setProperty("role", QStringLiteral("title"));
    titleCol->addWidget(titleLabel);
    auto *subLabel = new QLabel(tr("Modelagem 3D, pintura de texturas e animação"), card);
    subLabel->setProperty("role", QStringLiteral("muted"));
    titleCol->addWidget(subLabel);
    headerLayout->addLayout(titleCol);
    headerLayout->addStretch();

    m_closeBtn = new QPushButton(PetuniaIcons::get(QStringLiteral("close")), QString(), card);
    m_closeBtn->setProperty("role", QStringLiteral("quiet"));
    m_closeBtn->setFixedSize(theme.minHitTarget(), theme.minHitTarget());
    m_closeBtn->setToolTip(tr("Fechar (Esc)"));
    m_closeBtn->setAccessibleName(tr("Fechar a tela inicial"));
    connect(m_closeBtn, &QPushButton::clicked, this, &QDialog::accept);
    headerLayout->addWidget(m_closeBtn);
    cardLayout->addLayout(headerLayout);
    // Sem decoração do sistema: o cartão inteiro é a alça de arrasto (B5).
    PetuniaDialogChrome::install(this, QStringLiteral("home"), {true, card});

    auto separator = [card] {
        auto *line = new QFrame(card);
        line->setProperty("role", QStringLiteral("separator"));
        line->setFixedHeight(1);
        return line;
    };
    cardLayout->addWidget(separator());

    auto *actionsLayout = new QHBoxLayout();
    actionsLayout->setSpacing(theme.space(2));
    m_newBtn = new QPushButton(PetuniaIcons::get(QStringLiteral("plus")), tr("Novo projeto"), card);
    m_newBtn->setProperty("role", QStringLiteral("primary"));
    m_newBtn->setMinimumHeight(theme.controlHeight());
    m_newBtn->setDefault(true);
    connect(m_newBtn, &QPushButton::clicked, this, [this]() {
        emit newProjectRequested();
        accept();
    });
    actionsLayout->addWidget(m_newBtn);

    m_openBtn = new QPushButton(tr("Abrir projeto…"), card);
    m_openBtn->setMinimumHeight(theme.controlHeight());
    connect(m_openBtn, &QPushButton::clicked, this, [this]() {
        emit openProjectRequested();
        accept();
    });
    actionsLayout->addWidget(m_openBtn);

    m_recoverBtn = new QPushButton(tr("Recuperar sessão…"), card);
    m_recoverBtn->setMinimumHeight(theme.controlHeight());
    m_recoverBtn->setProperty("role", QStringLiteral("danger"));
    m_recoverBtn->setToolTip(tr("Há um salvamento automático de uma sessão interrompida"));
    m_recoverBtn->setVisible(false);
    connect(m_recoverBtn, &QPushButton::clicked, this, [this]() {
        emit recoverProjectRequested();
        accept();
    });
    actionsLayout->addWidget(m_recoverBtn);
    actionsLayout->addStretch();
    cardLayout->addLayout(actionsLayout);

    auto *recentTitle = new QLabel(tr("Projetos recentes"), card);
    recentTitle->setProperty("role", QStringLiteral("caption"));
    cardLayout->addWidget(recentTitle);

    m_recentList = new QListWidget(card);
    m_recentList->setAccessibleName(tr("Projetos recentes"));
    m_recentList->setToolTip(tr("Enter ou duplo clique abre o projeto"));
    // Enter e duplo clique: `itemActivated` cobre os dois (teclado incluído).
    connect(m_recentList, &QListWidget::itemActivated, this, &PetuniaHomeDialog::onRecentItemDoubleClicked);
    recentTitle->setBuddy(m_recentList);
    cardLayout->addWidget(m_recentList);

    m_noRecentLabel = new QLabel(tr("Nenhum projeto recente."), card);
    m_noRecentLabel->setProperty("role", QStringLiteral("muted"));
    m_noRecentLabel->setAlignment(Qt::AlignCenter);
    cardLayout->addWidget(m_noRecentLabel);

    cardLayout->addWidget(separator());

    auto *footerLayout = new QHBoxLayout();
    m_showOnStartCheck = new QCheckBox(tr("Mostrar esta tela ao iniciar"), card);
    m_showOnStartCheck->setChecked(true);
    footerLayout->addWidget(m_showOnStartCheck);
    footerLayout->addStretch();
    m_settingsBtn = new QPushButton(PetuniaIcons::get(QStringLiteral("settings")), tr("Preferências"), card);
    m_settingsBtn->setProperty("role", QStringLiteral("quiet"));
    connect(m_settingsBtn, &QPushButton::clicked, this, [this]() { emit settingsRequested(); });
    footerLayout->addWidget(m_settingsBtn);
    cardLayout->addLayout(footerLayout);
    mainLayout->addWidget(card);
}

void PetuniaHomeDialog::setRecentProjects(const QStringList &projectNames,
                                          const QStringList &projectPaths) {
    m_recentList->clear();
    int count = std::min(projectNames.size(), projectPaths.size());
    for (int i = 0; i < count; ++i) {
        auto *item = new QListWidgetItem(projectNames[i], m_recentList);
        item->setData(Qt::UserRole, projectPaths[i]);
        item->setToolTip(projectPaths[i]);
        item->setData(Qt::AccessibleDescriptionRole, projectPaths[i]);
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
    if (!item)
        return;
    QString path = item->data(Qt::UserRole).toString();
    if (!path.isEmpty()) {
        emit recentProjectSelected(path);
        accept();
    }
}
