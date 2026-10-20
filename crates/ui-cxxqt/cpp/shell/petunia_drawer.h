#pragma once

#include <QJsonObject>
#include <QTabBar>
#include <QWidget>

class QStackedWidget;
class QToolButton;

/// Gaveta inferior recolhível (Assets, Timeline, e futuramente Nós): a faixa de
/// abas fica sempre visível; clicar numa aba (ou no botão) abre/fecha o corpo.
/// O estado aberto/fechado e a altura são preferências do layout do workspace.
class PetuniaDrawer : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaDrawer(QWidget *parent = nullptr);

    /// Adiciona uma página; o `id` é estável (layout e testes).
    void addPage(const QString &id, const QString &title, const QString &icon, QWidget *page);
    void clearPages();
    int pageCount() const;
    QString currentPageId() const;
    void setCurrentPage(const QString &id);
    bool isOpen() const { return m_open; }
    void setOpen(bool open);
    int barHeight() const;
    QTabBar *tabBar() const { return m_tabs; }

  signals:
    void openChanged(bool open);

  private:
    void applyMetrics();
    void refresh();

    QTabBar *m_tabs{nullptr};
    QToolButton *m_toggle{nullptr};
    QStackedWidget *m_stack{nullptr};
    QWidget *m_bar{nullptr};
    QStringList m_ids;
    bool m_open{false};
};
