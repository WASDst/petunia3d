#include "petunia_command_palette.h"
#include <QGraphicsDropShadowEffect>
#include <QKeyEvent>
#include <QLabel>
#include <QShowEvent>

extern "C" {
int petunia_qt_app_search_commands(const char *query_str, char *results_out,
                                   int max_len);
}

PetuniaCommandPalette::PetuniaCommandPalette(QWidget *parent)
    : QDialog(parent, Qt::Popup | Qt::FramelessWindowHint) {
    setupUi();
}

void PetuniaCommandPalette::setupUi() {
    resize(540, 360);
    setMinimumSize(280, 220);
    setAttribute(Qt::WA_TranslucentBackground);

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(10, 10, 10, 10);

    auto *container = new QWidget(this);
    container->setObjectName(QStringLiteral("PaletteContainer"));
    container->setStyleSheet(
        QStringLiteral("#PaletteContainer { background-color: #202020; border: "
                       "1px solid #404040; border-radius: 8px; }"));

    auto *containerLayout = new QVBoxLayout(container);
    containerLayout->setContentsMargins(8, 8, 8, 8);
    containerLayout->setSpacing(6);

    // Campo de Busca
    m_searchInput = new QLineEdit(container);
    m_searchInput->setAccessibleName(tr("Buscar comando"));
    m_searchInput->setPlaceholderText(
        QStringLiteral("Buscar comando... (ex: cubo, duplicar, desfazer)"));
    m_searchInput->setStyleSheet(QStringLiteral(
        "QLineEdit { background-color: #292929; color: #eeeeee; border: 1px "
        "solid #505050; border-radius: 4px; padding: 8px; font-size: 13px; }"
        "QLineEdit:focus { border: 1px solid #b58cff; }"));
    containerLayout->addWidget(m_searchInput);

    // Lista de Resultados
    m_resultsList = new QListWidget(container);
    m_resultsList->setAccessibleName(tr("Comandos encontrados"));
    m_resultsList->setStyleSheet(
        QStringLiteral("QListWidget { background-color: transparent; border: "
                       "none; color: #eeeeee; font-size: 12px; outline: none; }"
                       "QListWidget::item { padding: 6px 8px; border-radius: "
                       "4px; margin-bottom: 2px; }"
                       "QListWidget::item:hover { background-color: #2d2d2d; }"
                       "QListWidget::item:selected { background-color: "
                       "#b58cff; color: #121212; font-weight: bold; }"));
    containerLayout->addWidget(m_resultsList);

    mainLayout->addWidget(container);

    // Efeito de sombra elegante
    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(24);
    shadow->setColor(QColor(0, 0, 0, 180));
    shadow->setOffset(0, 8);
    container->setGraphicsEffect(shadow);

    connect(m_searchInput, &QLineEdit::textChanged, this,
            &PetuniaCommandPalette::onSearchTextChanged);
    connect(m_resultsList, &QListWidget::itemActivated, this,
            &PetuniaCommandPalette::onItemActivated);
}

void PetuniaCommandPalette::showEvent(QShowEvent *event) {
    QDialog::showEvent(event);
    m_searchInput->clear();
    m_searchInput->setFocus();
    updateResults(QString());
}

void PetuniaCommandPalette::onSearchTextChanged(const QString &text) {
    updateResults(text);
}

void PetuniaCommandPalette::updateResults(const QString &query) {
    m_resultsList->clear();

    char buffer[8192];
    QByteArray utf8 = query.toUtf8();
    int count = petunia_qt_app_search_commands(utf8.constData(), buffer,
                                               sizeof(buffer));
    if (count <= 0) {
        return;
    }

    QString data = QString::fromUtf8(buffer);
    QStringList lines = data.split(QLatin1Char('\n'), Qt::SkipEmptyParts);

    for (const auto &line : lines) {
        QStringList parts = line.split(QLatin1Char('\t'));
        if (parts.size() < 2)
            continue;

        QString id = parts[0];
        QString label = parts[1];
        QString category = (parts.size() > 2) ? parts[2] : QString();
        QString shortcut = (parts.size() > 3) ? parts[3] : QString();

        QString display = label;
        if (!category.isEmpty()) {
            display = QStringLiteral("[%1] %2").arg(category, label);
        }
        if (!shortcut.isEmpty()) {
            display = QStringLiteral("%1  (%2)").arg(display, shortcut);
        }

        auto *item = new QListWidgetItem(display, m_resultsList);
        item->setData(Qt::UserRole, id);
        if (parts.size() > 4 && parts[4] == "false") {
            item->setFlags(item->flags() & ~Qt::ItemIsEnabled);
            item->setToolTip(parts.value(5));
        }
        m_resultsList->addItem(item);
    }

    if (m_resultsList->count() > 0) {
        m_resultsList->setCurrentRow(0);
    }
}

void PetuniaCommandPalette::onItemActivated(QListWidgetItem *item) {
    if (!item || !(item->flags() & Qt::ItemIsEnabled))
        return;
    QString cmdId = item->data(Qt::UserRole).toString();
    if (!cmdId.isEmpty()) {
        emit commandSelected(cmdId);
        accept();
    }
}

void PetuniaCommandPalette::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Down) {
        int next = (m_resultsList->currentRow() + 1) %
                   std::max(1, m_resultsList->count());
        m_resultsList->setCurrentRow(next);
        event->accept();
        return;
    }
    if (event->key() == Qt::Key_Up) {
        int prev = m_resultsList->currentRow() - 1;
        if (prev < 0)
            prev = m_resultsList->count() - 1;
        m_resultsList->setCurrentRow(prev);
        event->accept();
        return;
    }
    if (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter) {
        onItemActivated(m_resultsList->currentItem());
        event->accept();
        return;
    }
    if (event->key() == Qt::Key_Escape) {
        reject();
        event->accept();
        return;
    }
    QDialog::keyPressEvent(event);
}
