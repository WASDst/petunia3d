#include "petunia_drawer.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"

#include <QHBoxLayout>
#include <QSignalBlocker>
#include <QStackedWidget>
#include <QToolButton>
#include <QVBoxLayout>

PetuniaDrawer::PetuniaDrawer(QWidget *parent) : QWidget(parent) {
    setObjectName(QStringLiteral("PetuniaDrawer"));
    setAccessibleName(tr("Gaveta inferior"));
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    m_bar = new QWidget(this);
    m_bar->setObjectName(QStringLiteral("PetuniaDrawerBar"));
    auto *bar = new QHBoxLayout(m_bar);
    bar->setContentsMargins(0, 0, 0, 0);
    bar->setSpacing(0);
    m_tabs = new QTabBar(m_bar);
    m_tabs->setExpanding(false);
    m_tabs->setDrawBase(false);
    m_tabs->setFocusPolicy(Qt::StrongFocus);
    m_tabs->setAccessibleName(tr("Abas da gaveta"));
    bar->addWidget(m_tabs);
    bar->addStretch(1);
    m_toggle = new QToolButton(m_bar);
    m_toggle->setProperty("role", QStringLiteral("header"));
    m_toggle->setCheckable(true);
    bar->addWidget(m_toggle);
    root->addWidget(m_bar);

    m_stack = new QStackedWidget(this);
    root->addWidget(m_stack, 1);
    m_stack->hide();

    connect(m_tabs, &QTabBar::currentChanged, this, [this](int index) {
        if (index >= 0)
            m_stack->setCurrentIndex(index);
    });
    connect(m_tabs, &QTabBar::tabBarClicked, this, [this](int index) {
        // Clicar na aba ativa recolhe; em outra aba, abre.
        if (index == m_tabs->currentIndex())
            setOpen(!m_open);
        else
            setOpen(true);
    });
    connect(m_toggle, &QToolButton::toggled, this, [this](bool open) { setOpen(open); });
    connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, this, &PetuniaDrawer::applyMetrics);
    applyMetrics();
    refresh();
}

void PetuniaDrawer::applyMetrics() {
    const auto &theme = PetuniaTheme::instance();
    m_bar->setFixedHeight(theme.controlHeight() + theme.space(1));
    m_toggle->setIconSize(QSize(theme.iconPx(), theme.iconPx()));
    m_toggle->setMinimumHeight(theme.controlHeight());
    refresh();
}

int PetuniaDrawer::barHeight() const {
    return m_bar->height();
}

void PetuniaDrawer::addPage(const QString &id, const QString &title, const QString &icon, QWidget *page) {
    m_ids << id;
    m_stack->addWidget(page);
    const int index = m_tabs->addTab(PetuniaIcons::get(icon), title);
    m_tabs->setTabToolTip(index, title);
    m_tabs->setTabData(index, id);
    if (m_tabs->count() == 1)
        m_tabs->setCurrentIndex(0);
    refresh();
}

void PetuniaDrawer::clearPages() {
    const QSignalBlocker block(m_tabs);
    while (m_tabs->count() > 0)
        m_tabs->removeTab(0);
    while (m_stack->count() > 0) {
        auto *page = m_stack->widget(0);
        m_stack->removeWidget(page);
        page->hide();
        page->setParent(nullptr);
    }
    m_ids.clear();
    setOpen(false);
    refresh();
}

int PetuniaDrawer::pageCount() const {
    return m_tabs->count();
}

QString PetuniaDrawer::currentPageId() const {
    return m_tabs->currentIndex() >= 0 ? m_tabs->tabData(m_tabs->currentIndex()).toString() : QString();
}

void PetuniaDrawer::setCurrentPage(const QString &id) {
    for (int i = 0; i < m_tabs->count(); ++i)
        if (m_tabs->tabData(i).toString() == id) {
            m_tabs->setCurrentIndex(i);
            return;
        }
}

void PetuniaDrawer::setOpen(bool open) {
    if (m_tabs->count() == 0)
        open = false;
    if (m_open == open) {
        refresh();
        return;
    }
    m_open = open;
    refresh();
    emit openChanged(m_open);
}

void PetuniaDrawer::refresh() {
    const QSignalBlocker block(m_toggle);
    m_toggle->setChecked(m_open);
    m_toggle->setIcon(PetuniaIcons::get(m_open ? QStringLiteral("chevron_down") : QStringLiteral("chevron_up")));
    m_toggle->setToolTip(m_open ? tr("Recolher a gaveta") : tr("Abrir a gaveta"));
    m_toggle->setAccessibleName(m_toggle->toolTip());
    m_stack->setVisible(m_open && m_tabs->count() > 0);
    setVisible(m_tabs->count() > 0);
}
