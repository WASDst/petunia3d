#pragma once

#include <QJsonObject>
#include <QLabel>
#include <QStatusBar>

/// Região "Status": ferramenta ativa, mensagem (com severidade por texto e
/// ícone, nunca só cor), estatísticas e resumo da seleção. Não decide nada:
/// só projeta o estado publicado pelo editor.
class PetuniaStatusBar : public QStatusBar {
    Q_OBJECT

  public:
    explicit PetuniaStatusBar(QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    /// Mensagem de uma falha de ação (ex.: erro devolvido pelo editor).
    void showProblem(const QString &message);
    QLabel *message() const { return m_message; }
    QLabel *badge() const { return m_badge; }

  private:
    QLabel *m_badge{nullptr};
    QLabel *m_message{nullptr};
    QLabel *m_stats{nullptr};
    QLabel *m_selection{nullptr};
};
