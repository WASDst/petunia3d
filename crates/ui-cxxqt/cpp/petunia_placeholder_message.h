#pragma once

#include <QWidget>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QIcon>
#include <functional>

/// Widget reutilizável de estado vazio inspirado no padrão Kirigami.PlaceholderMessage.
/// Exibe ícone semântico, título em destaque, texto explicativo e botão de ação opcional.
class PetuniaPlaceholderMessage : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaPlaceholderMessage(QWidget *parent = nullptr);
    ~PetuniaPlaceholderMessage() override = default;

    void setIcon(const QIcon &icon);
    void setTitle(const QString &title);
    void setExplanation(const QString &explanation);
    void setActionText(const QString &text);
    void setActionVisible(bool visible);

signals:
    void actionTriggered();

private:
    QLabel *m_iconLabel{nullptr};
    QLabel *m_titleLabel{nullptr};
    QLabel *m_explanationLabel{nullptr};
    QPushButton *m_actionButton{nullptr};
};
