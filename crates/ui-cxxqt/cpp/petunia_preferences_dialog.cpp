#include "petunia_preferences_dialog.h"
#include "petunia_theme.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QHeaderView>
#include <QDialogButtonBox>
#include <QLabel>
#include <QSettings>
#include <QMessageBox>

PetuniaPreferencesDialog::PetuniaPreferencesDialog(QWidget *parent)
    : QDialog(parent)
{
    setupUi();
}

void PetuniaPreferencesDialog::setupUi() {
    setWindowTitle(QStringLiteral("Preferências do Petunia3D"));
    resize(640, 480);
    setStyleSheet(QStringLiteral(
        "QDialog { background-color: #1e1e1e; color: #eeeeee; }"
        "QTabWidget::pane { border: 1px solid #404040; background-color: #202020; }"
        "QTabBar::tab { background-color: #282828; color: #b5b5b5; padding: 8px 18px; border: 1px solid #404040; margin-right: 2px; border-top-left-radius: 4px; border-top-right-radius: 4px; }"
        "QTabBar::tab:selected { background-color: #323232; color: #b58cff; font-weight: bold; border-bottom-color: #323232; }"
        "QComboBox, QSpinBox, QDoubleSpinBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 4px; }"
        "QTableWidget { background-color: #202020; color: #eeeeee; border: 1px solid #404040; gridline-color: #333333; }"
        "QHeaderView::section { background-color: #292929; color: #b5b5b5; border: 1px solid #404040; padding: 4px; }"
        "QPushButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 6px 16px; font-weight: bold; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; }"
        "QPushButton:default { background-color: #b58cff; color: #121212; }"
    ));

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(12, 12, 12, 12);

    m_tabs = new QTabWidget(this);
    setupGeneralTab();
    setupViewportTab();
    setupKeymapTab();
    mainLayout->addWidget(m_tabs);

    auto *buttonBox = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttonBox, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttonBox, &QDialogButtonBox::rejected, this, &QDialog::reject);
    mainLayout->addWidget(buttonBox);
}

void PetuniaPreferencesDialog::setupGeneralTab() {
    auto *page = new QWidget(m_tabs);
    auto *layout = new QFormLayout(page);
    layout->setContentsMargins(16, 16, 16, 16);
    layout->setSpacing(12);

    m_langCombo = new QComboBox(page);
    m_langCombo->addItem(QStringLiteral("Português (Brasil)"), QStringLiteral("pt-BR"));
    m_langCombo->addItem(QStringLiteral("English (US)"), QStringLiteral("en"));
    layout->addRow(QStringLiteral("Idioma da Interface:"), m_langCombo);

    m_themeCombo = new QComboBox(page);
    m_themeCombo->addItem(QStringLiteral("Petunia Dark (Padrão)"), QStringLiteral("petunia-dark"));
    m_themeCombo->addItem(QStringLiteral("Petunia Light"), QStringLiteral("petunia-light"));
    m_themeCombo->addItem(QStringLiteral("High Contrast Dark"), QStringLiteral("petunia-high-contrast"));
    connect(m_themeCombo, &QComboBox::currentIndexChanged, this, [this](int idx) {
        QString themeId = m_themeCombo->itemData(idx).toString();
        if (!themeId.isEmpty()) {
            QJsonObject colors;
            if (themeId == "petunia-light") {
                colors.insert(QStringLiteral("bg_canvas"), QStringLiteral("#f5f5f5"));
                colors.insert(QStringLiteral("bg_panel"), QStringLiteral("#ebebeb"));
                colors.insert(QStringLiteral("bg_surface"), QStringLiteral("#e0e0e0"));
                colors.insert(QStringLiteral("text_primary"), QStringLiteral("#1a1a1a"));
                colors.insert(QStringLiteral("accent_blue"), QStringLiteral("#7a38ff"));
                colors.insert(QStringLiteral("accent_orange"), QStringLiteral("#e05500"));
            } else if (themeId == "petunia-high-contrast") {
                colors.insert(QStringLiteral("bg_canvas"), QStringLiteral("#000000"));
                colors.insert(QStringLiteral("bg_panel"), QStringLiteral("#000000"));
                colors.insert(QStringLiteral("bg_surface"), QStringLiteral("#1a1a1a"));
                colors.insert(QStringLiteral("text_primary"), QStringLiteral("#ffffff"));
                colors.insert(QStringLiteral("accent_blue"), QStringLiteral("#ffff00"));
                colors.insert(QStringLiteral("accent_orange"), QStringLiteral("#ff6600"));
            }
            PetuniaTheme::instance().updateFromSnapshot(colors, themeId);
        }
    });
    layout->addRow(QStringLiteral("Tema Visual:"), m_themeCombo);

    m_densityCombo = new QComboBox(page);
    m_densityCombo->addItem(QStringLiteral("Padrão (28px)"), 1);
    m_densityCombo->addItem(QStringLiteral("Compacto (24px)"), 0);
    m_densityCombo->addItem(QStringLiteral("Confortável (36px)"), 2);
    connect(m_densityCombo, &QComboBox::currentIndexChanged, this, [this](int idx) {
        PetuniaTheme::instance().setDensity(static_cast<PetuniaTheme::Density>(m_densityCombo->itemData(idx).toInt()));
    });
    layout->addRow(QStringLiteral("Densidade da Interface:"), m_densityCombo);

    auto *resetLayoutBtn = new QPushButton(QStringLiteral("Restaurar Layout de Janelas"), page);
    connect(resetLayoutBtn, &QPushButton::clicked, this, [this]() {
        QSettings settings("Petunia3D", "QtStudio");
        settings.remove("workspace");
        QMessageBox::information(this, QStringLiteral("Layout Redefinido"),
            QStringLiteral("Os layouts de docas e janelas foram redefinidos para os valores padrão de fábrica."));
    });
    layout->addRow(QStringLiteral("Layout de Estúdio:"), resetLayoutBtn);

    m_autosaveMinutes = new QSpinBox(page);
    m_autosaveMinutes->setRange(1, 60);
    m_autosaveMinutes->setValue(5);
    m_autosaveMinutes->setSuffix(QStringLiteral(" min"));
    layout->addRow(QStringLiteral("Intervalo de Salvamento Automático:"), m_autosaveMinutes);

    m_recoveryCheck = new QCheckBox(QStringLiteral("Salvar snapshot antes de operações pesadas"), page);
    m_recoveryCheck->setChecked(true);
    layout->addRow(QStringLiteral("Recuperação:"), m_recoveryCheck);

    m_tabs->addTab(page, QStringLiteral("Geral"));
}

void PetuniaPreferencesDialog::setupViewportTab() {
    auto *page = new QWidget(m_tabs);
    auto *layout = new QFormLayout(page);
    layout->setContentsMargins(16, 16, 16, 16);
    layout->setSpacing(12);

    m_msaaCombo = new QComboBox(page);
    m_msaaCombo->addItem(QStringLiteral("Desativado (1x)"));
    m_msaaCombo->addItem(QStringLiteral("2x MSAA"));
    m_msaaCombo->addItem(QStringLiteral("4x MSAA (Recomendado)"));
    m_msaaCombo->addItem(QStringLiteral("8x MSAA"));
    m_msaaCombo->setCurrentIndex(2);
    layout->addRow(QStringLiteral("Antialiasing da Viewport:"), m_msaaCombo);

    m_gridDivisions = new QSpinBox(page);
    m_gridDivisions->setRange(4, 128);
    m_gridDivisions->setValue(16);
    layout->addRow(QStringLiteral("Subdivisões da Grade 3D:"), m_gridDivisions);

    m_cameraSpeed = new PetuniaNumberField(QStringLiteral("⚡"), page);
    m_cameraSpeed->setRange(0.1, 10.0);
    m_cameraSpeed->setValue(1.0);
    m_cameraSpeed->setSingleStep(0.1);
    layout->addRow(QStringLiteral("Sensibilidade da Câmera:"), m_cameraSpeed);

    m_invertOrbitY = new QCheckBox(QStringLiteral("Inverter eixo vertical ao orbitar"), page);
    layout->addRow(QStringLiteral("Órbita Vertical:"), m_invertOrbitY);

    m_showFps = new QCheckBox(QStringLiteral("Exibir contador de taxa de quadros (FPS)"), page);
    layout->addRow(QStringLiteral("Telemetria:"), m_showFps);

    m_tabs->addTab(page, QStringLiteral("Viewport"));
}

void PetuniaPreferencesDialog::setupKeymapTab() {
    auto *page = new QWidget(m_tabs);
    auto *layout = new QVBoxLayout(page);
    layout->setContentsMargins(12, 12, 12, 12);

    m_keymapTable = new QTableWidget(page);
    m_keymapTable->setColumnCount(3);
    m_keymapTable->setHorizontalHeaderLabels({ QStringLiteral("Ação / Comando"), QStringLiteral("Atalho Atual"), QStringLiteral("Contexto") });
    m_keymapTable->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    m_keymapTable->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    m_keymapTable->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);

    const struct { const char *cmd; const char *shortcut; const char *ctx; } keymaps[] = {
        { "Paleta de Comandos", "Ctrl+P / Ctrl+K / F3", "Global" },
        { "Desfazer Ação", "Ctrl+Z", "Global" },
        { "Refazer Ação", "Ctrl+Shift+Z / Ctrl+Y", "Global" },
        { "Duplicar Seleção", "Shift+D", "Modelagem" },
        { "Deletar Seleção", "Delete / X", "Modelagem" },
        { "Extrusão de Faces", "E", "POLY" },
        { "Inserção de Faces (Inset)", "I", "POLY" },
        { "Chanfro (Bevel)", "Ctrl+B", "POLY" },
        { "Loop Cut", "Ctrl+R", "POLY" },
        { "Alternar Modo Objeto / Sub-Objeto", "Tab", "POLY" },
        { "Vista Frontal", "Numpad 1", "Viewport" },
        { "Vista Direita", "Numpad 3", "Viewport" },
        { "Vista Superior (Top)", "Numpad 7", "Viewport" },
        { "Alternar Perspectiva / Ortográfica", "Numpad 5", "Viewport" },
        { "Focar na Seleção", "F / Numpad .", "Viewport" },
        { "Alternar Wireframe / Sombreado", "Shift+Z", "Viewport" },
    };

    int row = 0;
    for (const auto &km : keymaps) {
        m_keymapTable->insertRow(row);
        m_keymapTable->setItem(row, 0, new QTableWidgetItem(QString::fromUtf8(km.cmd)));
        m_keymapTable->setItem(row, 1, new QTableWidgetItem(QString::fromUtf8(km.shortcut)));
        m_keymapTable->setItem(row, 2, new QTableWidgetItem(QString::fromUtf8(km.ctx)));
        row++;
    }

    layout->addWidget(m_keymapTable);
    m_tabs->addTab(page, QStringLiteral("Atalhos"));
}
