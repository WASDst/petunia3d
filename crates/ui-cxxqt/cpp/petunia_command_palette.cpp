#include "petunia_command_palette.h"
#include "petunia_theme.h"
#include <QFrame>
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
    const auto &theme = PetuniaTheme::instance();
    setWindowTitle(tr("Paleta de comandos"));
    setAccessibleName(tr("Paleta de comandos"));
    resize(theme.scaled(540), theme.scaled(360));
    setMinimumSize(theme.scaled(280), theme.scaled(220));
    setAttribute(Qt::WA_TranslucentBackground);

    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(2));

    // Superfície e campos herdam o tema global (claro/escuro/alto contraste).
    auto *container = new QFrame(this);
    container->setProperty("role", QStringLiteral("card"));
    auto *containerLayout = new QVBoxLayout(container);
    containerLayout->setContentsMargins(theme.space(2), theme.space(2), theme.space(2), theme.space(2));
    containerLayout->setSpacing(theme.space(2));

    m_searchInput = new QLineEdit(container);
    m_searchInput->setAccessibleName(tr("Buscar comando"));
    m_searchInput->setPlaceholderText(tr("Buscar comando… (ex.: cubo, duplicar, desfazer)"));
    m_searchInput->setClearButtonEnabled(true);
    containerLayout->addWidget(m_searchInput);

    m_resultsList = new QListWidget(container);
    m_resultsList->setAccessibleName(tr("Comandos encontrados"));
    m_resultsList->setUniformItemSizes(true);
    containerLayout->addWidget(m_resultsList);

    m_emptyLabel = new QLabel(tr("Nenhum comando encontrado."), container);
    m_emptyLabel->setProperty("role", QStringLiteral("muted"));
    m_emptyLabel->setAlignment(Qt::AlignCenter);
    m_emptyLabel->setVisible(false);
    containerLayout->addWidget(m_emptyLabel);

    mainLayout->addWidget(container);

    auto *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(24);
    shadow->setColor(QColor(0, 0, 0, 160));
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

    char buffer[8192] = {};
    QByteArray utf8 = query.toUtf8();
    int count = petunia_qt_app_search_commands(utf8.constData(), buffer,
                                               sizeof(buffer));
    m_emptyLabel->setVisible(count <= 0);
    if (count <= 0)
        return;

    // Limitado ao buffer mesmo se a ponte não terminar a string em zero.
    QString data = QString::fromUtf8(buffer, static_cast<qsizetype>(qstrnlen(buffer, sizeof(buffer))));
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
        if (!category.isEmpty())
            display = QStringLiteral("%1 — %2").arg(label, category);
        if (!shortcut.isEmpty())
            display = QStringLiteral("%1\t%2").arg(display, shortcut);

        auto *item = new QListWidgetItem(display, m_resultsList);
        item->setData(Qt::UserRole, id);
        QString description = category.isEmpty() ? label : QStringLiteral("%1, %2").arg(label, category);
        if (!shortcut.isEmpty())
            description += tr(", atalho %1").arg(shortcut);
        if (parts.size() > 4 && parts[4] == "false") {
            // Desabilitado com motivo visível e anunciado, nunca escondido.
            item->setFlags(item->flags() & ~Qt::ItemIsEnabled);
            item->setToolTip(parts.value(5));
            description += tr(", indisponível: %1").arg(parts.value(5));
        }
        item->setData(Qt::AccessibleTextRole, description);
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
