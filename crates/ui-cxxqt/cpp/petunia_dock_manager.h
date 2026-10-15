#pragma once

#include <QObject>
#include <QString>
#include <QMap>
#include <QByteArray>
#include <QMainWindow>
#include <QDockWidget>

/// Gerenciador desacoplado de docas e layouts de estúdio.
/// Fornece uma camada de abstração sobre o sistema de docking do Qt,
/// desenhada como adaptador drop-in para a transição transparente para KDDockWidgets.
class PetuniaDockManager : public QObject {
    Q_OBJECT

public:
    explicit PetuniaDockManager(QMainWindow *mainWindow, QObject *parent = nullptr);
    ~PetuniaDockManager() override = default;

    /// Registra uma doca gerenciada associando id único, título, widget de conteúdo e área padrão.
    QDockWidget *registerDock(const QString &id, const QString &title, QWidget *widget,
                              Qt::DockWidgetArea defaultArea = Qt::RightDockWidgetArea);

    /// Registra uma doca pré-existente na coleção gerenciada.
    void registerExistingDock(const QString &id, QDockWidget *dock);

    /// Retorna a doca associada ao identificador.
    QDockWidget *dock(const QString &id) const;

    /// Define a visibilidade de uma doca.
    void setDockVisible(const QString &id, bool visible);

    /// Consulta se uma doca está visível.
    bool isDockVisible(const QString &id) const;

    /// Alterna a visibilidade da doca.
    void toggleDock(const QString &id);

    /// Salva o estado completo de todas as docas para persistência em sessão.
    QByteArray saveState() const;

    /// Restaura o estado salvo das docas.
    bool restoreState(const QByteArray &state);

    /// Aplica presets de docas conforme o workspace ativo ("MODEL", "PAINT", "UV", "ANIMATE").
    void applyWorkspacePreset(const QString &workspace);

signals:
    void dockVisibilityChanged(const QString &id, bool visible);

private:
    QMainWindow *m_mainWindow{nullptr};
    QMap<QString, QDockWidget*> m_docks;
};
