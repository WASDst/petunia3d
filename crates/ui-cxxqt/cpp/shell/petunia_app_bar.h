#pragma once

#include "petunia_shell_controller.h"

#include <QHash>
#include <QMenuBar>
#include <QToolButton>
#include <QWidget>

class PetuniaSegmented;
class QLabel;
class QPushButton;
class QHBoxLayout;

/// Região "Barra do app": identidade do projeto, menus, workspaces, desfazer/
/// refazer, busca de comandos e preferências. Apresentação pura — ações locais
/// (diálogos de arquivo, painéis) saem por `localAction`; o resto vai ao editor.
class PetuniaAppBar : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaAppBar(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    /// Abaixo de ~1040 px os menus viram um único botão e a busca fica só com ícone.
    bool compact() const { return m_compact; }
    PetuniaSegmented *workspaces() const { return m_workspaces; }
    QMenuBar *menuBar() const { return m_menuBar; }
    QPushButton *searchButton() const { return m_search; }
    QToolButton *undoButton() const { return m_undo; }
    QToolButton *redoButton() const { return m_redo; }

  protected:
    void resizeEvent(QResizeEvent *event) override;

  signals:
    /// Ações que dependem da janela: file.*, app.*, panel.*.
    void localAction(const QString &id);

  private:
    struct Tracked {
        QAction *action;
        QString text;
        QString keymap;
    };
    QAction *addLocal(QMenu *menu, const QString &text, const QString &id, const QString &keymap = {});
    QAction *addCommand(QMenu *menu, const QString &text, const QString &command, const QString &keymap = {});
    QAction *addRequest(QMenu *menu, const QString &text, const QString &request, const QString &keymap = {});
    void buildMenus();
    void rebuildAddMenu(const QJsonObject &state);
    void applyMetrics();
    void applyCompact(bool compact);
    QMenu *newMenu(const QString &title);

    PetuniaShellController *m_controller;
    QHBoxLayout *m_layout{nullptr};
    QLabel *m_logo{nullptr};
    QLabel *m_project{nullptr};
    QMenuBar *m_menuBar{nullptr};
    QMenu *m_addMenu{nullptr};
    QList<QMenu *> m_menus;
    QToolButton *m_hamburger{nullptr};
    bool m_compact{false};
    PetuniaSegmented *m_workspaces{nullptr};
    QToolButton *m_undo{nullptr};
    QToolButton *m_redo{nullptr};
    QPushButton *m_search{nullptr};
    QToolButton *m_settings{nullptr};
    QList<Tracked> m_tracked;
    QString m_addSignature;
};
