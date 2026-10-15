#include "petunia_asset_library_widget.h"
#include "petunia_placeholder_message.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QMouseEvent>
#include <QGraphicsDropShadowEffect>

// =========================================================================
// PetuniaAssetCard
// =========================================================================

PetuniaAssetCard::PetuniaAssetCard(const AssetLibraryItem &item, QWidget *parent)
    : QWidget(parent)
    , m_item(item)
{
    setFixedSize(170, 160);
    setCursor(Qt::PointingHandCursor);

    setStyleSheet(QStringLiteral(
        "QWidget { background-color: #242424; border: 1px solid #3c3c3c; border-radius: 6px; }"
        "QWidget:hover { background-color: #2e2e2e; border: 1px solid #b58cff; }"
        "QLabel { border: none; background: transparent; }"
        "QPushButton { background-color: #333333; color: #eeeeee; border: 1px solid #555555; border-radius: 4px; padding: 4px; font-weight: bold; }"
        "QPushButton:hover { background-color: #b58cff; color: #121212; border-color: #b58cff; }"
    ));

    auto *cardLayout = new QVBoxLayout(this);
    cardLayout->setContentsMargins(8, 8, 8, 8);
    cardLayout->setSpacing(4);

    // Ícone / Emoji
    auto *iconLabel = new QLabel(m_item.icon, this);
    iconLabel->setAlignment(Qt::AlignCenter);
    QFont iconFont = iconLabel->font();
    iconFont.setPointSize(28);
    iconLabel->setFont(iconFont);
    cardLayout->addWidget(iconLabel);

    // Nome
    auto *nameLabel = new QLabel(m_item.name, this);
    nameLabel->setAlignment(Qt::AlignCenter);
    nameLabel->setStyleSheet(QStringLiteral("color: #ffffff; font-weight: bold; font-size: 12px;"));
    cardLayout->addWidget(nameLabel);

    // Badge de Polycount
    auto *badgeLabel = new QLabel(QStringLiteral("%1 • %2").arg(m_item.polycount, m_item.category), this);
    badgeLabel->setAlignment(Qt::AlignCenter);
    badgeLabel->setStyleSheet(QStringLiteral("color: #888888; font-size: 10px;"));
    cardLayout->addWidget(badgeLabel);

    cardLayout->addStretch();

    // Botão Adicionar
    m_insertBtn = new QPushButton(QStringLiteral("+ Inserir"), this);
    connect(m_insertBtn, &QPushButton::clicked, this, [this]() {
        emit insertRequested(m_item.command);
    });
    cardLayout->addWidget(m_insertBtn);
}

void PetuniaAssetCard::mouseDoubleClickEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        emit insertRequested(m_item.command);
    }
}

// =========================================================================
// PetuniaAssetLibraryWidget
// =========================================================================

PetuniaAssetLibraryWidget::PetuniaAssetLibraryWidget(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle(QStringLiteral("Biblioteca de Modelos & Primitivas 3D"));
    resize(760, 520);
    setStyleSheet(QStringLiteral(
        "QDialog { background-color: #1a1a1a; color: #eeeeee; }"
        "QLineEdit, QComboBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 6px; font-size: 12px; }"
        "QScrollArea { background-color: #181818; border: 1px solid #333333; border-radius: 4px; }"
    ));

    populateItems();
    setupUi();
}

void PetuniaAssetLibraryWidget::populateItems() {
    m_allItems = {
        {
            QStringLiteral("cube"),
            QStringLiteral("Cubo B-Rep"),
            QStringLiteral("Primitivas"),
            QStringLiteral("📦"),
            QStringLiteral("12 tris"),
            QStringLiteral("Cubo unitário padrão com 6 faces e normais ortogonais."),
            QStringLiteral("model.add_cube")
        },
        {
            QStringLiteral("sphere"),
            QStringLiteral("Esfera UV"),
            QStringLiteral("Primitivas"),
            QStringLiteral("🌐"),
            QStringLiteral("128 tris"),
            QStringLiteral("Esfera UV esférica com anéis de latitude e meridianos."),
            QStringLiteral("model.add_sphere")
        },
        {
            QStringLiteral("cylinder"),
            QStringLiteral("Cilindro"),
            QStringLiteral("Primitivas"),
            QStringLiteral("🥫"),
            QStringLiteral("64 tris"),
            QStringLiteral("Cilindro com tampas poligonais superior e inferior."),
            QStringLiteral("model.add_cylinder")
        },
        {
            QStringLiteral("cone"),
            QStringLiteral("Cone"),
            QStringLiteral("Primitivas"),
            QStringLiteral("🔺"),
            QStringLiteral("32 tris"),
            QStringLiteral("Cone com base circular e ápice pontiagudo."),
            QStringLiteral("model.add_cone")
        },
        {
            QStringLiteral("plane"),
            QStringLiteral("Plano"),
            QStringLiteral("Superfícies"),
            QStringLiteral("📄"),
            QStringLiteral("2 tris"),
            QStringLiteral("Plano horizontal unitário para pisos e superfícies."),
            QStringLiteral("model.add_plane")
        },
        {
            QStringLiteral("torus"),
            QStringLiteral("Torus (Rosca)"),
            QStringLiteral("Complexos"),
            QStringLiteral("🍩"),
            QStringLiteral("192 tris"),
            QStringLiteral("Superfície toroidal com raio maior e raio menor contínuos."),
            QStringLiteral("model.add_torus")
        },
        {
            QStringLiteral("capsule"),
            QStringLiteral("Cápsula"),
            QStringLiteral("Complexos"),
            QStringLiteral("💊"),
            QStringLiteral("96 tris"),
            QStringLiteral("Cilindro com extremidades semiesféricas contínuas."),
            QStringLiteral("model.add_capsule")
        },
        {
            QStringLiteral("icosphere"),
            QStringLiteral("Icosfera"),
            QStringLiteral("Primitivas"),
            QStringLiteral("⚽"),
            QStringLiteral("80 tris"),
            QStringLiteral("Esfera geodésica subdividida com triângulos homogêneos."),
            QStringLiteral("model.add_icosphere")
        },
        {
            QStringLiteral("wedge"),
            QStringLiteral("Cunha (Rampa)"),
            QStringLiteral("Primitivas"),
            QStringLiteral("🪓"),
            QStringLiteral("8 tris"),
            QStringLiteral("Primitiva em cunha angular para rampas e chanfros."),
            QStringLiteral("model.add_wedge")
        },
        {
            QStringLiteral("circle"),
            QStringLiteral("Círculo / Disco"),
            QStringLiteral("Superfícies"),
            QStringLiteral("⭕"),
            QStringLiteral("32 tris"),
            QStringLiteral("Superfície planar em disco com topologia radial central."),
            QStringLiteral("model.add_circle")
        },
        {
            QStringLiteral("suzanne"),
            QStringLiteral("Suzanne (Macaco)"),
            QStringLiteral("Complexos"),
            QStringLiteral("🐵"),
            QStringLiteral("968 tris"),
            QStringLiteral("Modelo padrão de teste para materiais, curvaturas e shaders."),
            QStringLiteral("model.add_cube") // Fallback seguro adiciona cubo com etiqueta de teste se monkey não compilado
        }
    };
}

void PetuniaAssetLibraryWidget::setupUi() {
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(12, 12, 12, 12);
    mainLayout->setSpacing(10);

    // 1. Barra de Filtro e Busca
    auto *filterRow = new QHBoxLayout();

    m_searchEdit = new QLineEdit(this);
    m_searchEdit->setPlaceholderText(QStringLiteral("Buscar por nome (ex: Cubo, Esfera, Torus)..."));
    m_searchEdit->setClearButtonEnabled(true);
    connect(m_searchEdit, &QLineEdit::textChanged, this, &PetuniaAssetLibraryWidget::filterAssets);
    filterRow->addWidget(m_searchEdit, 2);

    m_categoryCombo = new QComboBox(this);
    m_categoryCombo->addItem(QStringLiteral("Todas as Categorias"), QStringLiteral("all"));
    m_categoryCombo->addItem(QStringLiteral("Primitivas Básicas"), QStringLiteral("Primitivas"));
    m_categoryCombo->addItem(QStringLiteral("Modelos Complexos"), QStringLiteral("Complexos"));
    m_categoryCombo->addItem(QStringLiteral("Superfícies & Planos"), QStringLiteral("Superfícies"));
    connect(m_categoryCombo, &QComboBox::currentIndexChanged, this, &PetuniaAssetLibraryWidget::filterAssets);
    filterRow->addWidget(m_categoryCombo, 1);

    mainLayout->addLayout(filterRow);

    // 2. Grade de Cards com ScrollArea
    m_scrollArea = new QScrollArea(this);
    m_scrollArea->setWidgetResizable(true);

    m_gridContainer = new QWidget(m_scrollArea);
    m_gridLayout = new QGridLayout(m_gridContainer);
    m_gridLayout->setContentsMargins(8, 8, 8, 8);
    m_gridLayout->setSpacing(10);

    m_cardWidgets.clear();
    for (const auto &item : m_allItems) {
        auto *card = new PetuniaAssetCard(item, m_gridContainer);
        connect(card, &PetuniaAssetCard::insertRequested, this, [this](const QString &cmd) {
            emit assetSpawned(cmd);
            accept();
        });
        m_cardWidgets.append(card);
    }

    // Mensagem de estado vazio (Kirigami.PlaceholderMessage)
    m_placeholder = new PetuniaPlaceholderMessage(m_gridContainer);
    m_placeholder->setTitle(tr("Nenhum recurso encontrado"));
    m_placeholder->setExplanation(tr("Nenhum modelo ou primitiva corresponde ao filtro ou busca atual."));
    m_placeholder->setActionText(tr("Limpar busca"));
    connect(m_placeholder, &PetuniaPlaceholderMessage::actionTriggered, this, [this]() {
        m_searchEdit->clear();
        m_categoryCombo->setCurrentIndex(0);
    });
    m_gridLayout->addWidget(m_placeholder, 0, 0, 1, 4, Qt::AlignCenter);
    m_placeholder->hide();

    m_scrollArea->setWidget(m_gridContainer);
    mainLayout->addWidget(m_scrollArea, 1);

    // 3. Rodapé com Instrução e Fechar
    auto *footerRow = new QHBoxLayout();
    auto *hintLabel = new QLabel(QStringLiteral("Dica: Clique duplo no card para adicionar imediatamente à cena."), this);
    hintLabel->setStyleSheet(QStringLiteral("color: #777777; font-size: 11px;"));
    footerRow->addWidget(hintLabel);

    footerRow->addStretch();

    auto *closeBtn = new QPushButton(QStringLiteral("Fechar"), this);
    closeBtn->setStyleSheet(QStringLiteral("background-color: #333333; color: #eeeeee; padding: 6px 14px; border-radius: 4px; font-weight: bold;"));
    connect(closeBtn, &QPushButton::clicked, this, &QDialog::reject);
    footerRow->addWidget(closeBtn);

    mainLayout->addLayout(footerRow);

    filterAssets();
}

void PetuniaAssetLibraryWidget::filterAssets() {
    QString query = m_searchEdit->text().trimmed().toLower();
    QString catFilter = m_categoryCombo->currentData().toString();

    // Remove do grid
    for (auto *card : m_cardWidgets) {
        m_gridLayout->removeWidget(card);
        card->hide();
    }
    if (m_placeholder) {
        m_gridLayout->removeWidget(m_placeholder);
        m_placeholder->hide();
    }

    int visibleCol = 0;
    int visibleRow = 0;
    const int maxCols = 4;

    for (auto *card : m_cardWidgets) {
        const auto &it = card->item();
        bool matchQuery = query.isEmpty() || it.name.toLower().contains(query) || it.description.toLower().contains(query);
        bool matchCat = (catFilter == QStringLiteral("all")) || (it.category == catFilter);

        if (matchQuery && matchCat) {
            m_gridLayout->addWidget(card, visibleRow, visibleCol);
            card->show();
            visibleCol++;
            if (visibleCol >= maxCols) {
                visibleCol = 0;
                visibleRow++;
            }
        }
    }

    if (visibleCol == 0 && visibleRow == 0 && m_placeholder) {
        m_gridLayout->addWidget(m_placeholder, 0, 0, 1, 4, Qt::AlignCenter);
        m_placeholder->show();
    }
}
