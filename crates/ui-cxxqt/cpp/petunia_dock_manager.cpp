#include "petunia_dock_manager.h"

PetuniaDockManager::PetuniaDockManager(QMainWindow *mainWindow, QObject *parent)
    : QObject(parent)
    , m_mainWindow(mainWindow)
{
}

QDockWidget *PetuniaDockManager::registerDock(const QString &id, const QString &title, QWidget *widget,
                                              Qt::DockWidgetArea defaultArea)
{
    if (!m_mainWindow) return nullptr;

    auto *dock = new QDockWidget(title, m_mainWindow);
    dock->setObjectName(id);
    if (widget) {
        dock->setWidget(widget);
    }
    m_mainWindow->addDockWidget(defaultArea, dock);
    m_docks.insert(id, dock);

    connect(dock, &QDockWidget::visibilityChanged, this, [this, id](bool visible) {
        emit dockVisibilityChanged(id, visible);
    });

    return dock;
}

void PetuniaDockManager::registerExistingDock(const QString &id, QDockWidget *dock)
{
    if (!dock) return;

    dock->setObjectName(id);
    m_docks.insert(id, dock);

    connect(dock, &QDockWidget::visibilityChanged, this, [this, id](bool visible) {
        emit dockVisibilityChanged(id, visible);
    });
}

QDockWidget *PetuniaDockManager::dock(const QString &id) const {
    return m_docks.value(id, nullptr);
}

void PetuniaDockManager::setDockVisible(const QString &id, bool visible) {
    if (auto *d = dock(id)) {
        d->setVisible(visible);
    }
}

bool PetuniaDockManager::isDockVisible(const QString &id) const {
    auto *d = dock(id);
    return d && d->isVisible();
}

void PetuniaDockManager::toggleDock(const QString &id) {
    if (auto *d = dock(id)) {
        d->setVisible(!d->isVisible());
    }
}

QByteArray PetuniaDockManager::saveState() const {
    return m_mainWindow ? m_mainWindow->saveState() : QByteArray();
}

bool PetuniaDockManager::restoreState(const QByteArray &state) {
    return m_mainWindow ? m_mainWindow->restoreState(state) : false;
}

void PetuniaDockManager::applyWorkspacePreset(const QString &workspace) {
    if (m_mainWindow && !m_currentWorkspace.isEmpty()) {
        m_workspaceStates[m_currentWorkspace] = m_mainWindow->saveState();
    }
    m_currentWorkspace = workspace;

    if (m_mainWindow && m_workspaceStates.contains(workspace) && !m_workspaceStates[workspace].isEmpty()) {
        m_mainWindow->restoreState(m_workspaceStates[workspace]);
        for (auto it = m_docks.begin(); it != m_docks.end(); ++it) {
            emit dockVisibilityChanged(it.key(), it.value()->isVisible());
        }
        return;
    }

    if (workspace == QStringLiteral("MODEL") || workspace == QStringLiteral("DRAW") || workspace == QStringLiteral("POLY")) {
        setDockVisible(QStringLiteral("OutlinerDock"), true);
        setDockVisible(QStringLiteral("InspectorDock"), true);
        setDockVisible(QStringLiteral("PaintDock"), false);
        setDockVisible(QStringLiteral("UvDock"), false);
        setDockVisible(QStringLiteral("TimelineDock"), false);
    } else if (workspace == QStringLiteral("PAINT")) {
        setDockVisible(QStringLiteral("OutlinerDock"), true);
        setDockVisible(QStringLiteral("InspectorDock"), true);
        setDockVisible(QStringLiteral("PaintDock"), true);
        setDockVisible(QStringLiteral("UvDock"), false);
        setDockVisible(QStringLiteral("TimelineDock"), false);
    } else if (workspace == QStringLiteral("UV")) {
        setDockVisible(QStringLiteral("OutlinerDock"), true);
        setDockVisible(QStringLiteral("InspectorDock"), true);
        setDockVisible(QStringLiteral("PaintDock"), false);
        setDockVisible(QStringLiteral("UvDock"), true);
        setDockVisible(QStringLiteral("TimelineDock"), false);
    } else if (workspace == QStringLiteral("ANIMATE")) {
        setDockVisible(QStringLiteral("OutlinerDock"), true);
        setDockVisible(QStringLiteral("InspectorDock"), true);
        setDockVisible(QStringLiteral("PaintDock"), false);
        setDockVisible(QStringLiteral("UvDock"), false);
        setDockVisible(QStringLiteral("TimelineDock"), true);
    }

    if (m_mainWindow) {
        m_workspaceStates[workspace] = m_mainWindow->saveState();
    }
}
