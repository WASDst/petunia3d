#pragma once

#include "../petunia_editor_api.h"
#include <QElapsedTimer>
#include <QJsonObject>
#include <QObject>
#include <QTimer>

/// Único dono do estado projetado do editor para o Shell v2.
///
/// A GUI não consulta o snapshot por conta própria: este controlador mantém o
/// último estado, o atualiza a cada `tick` (só quando mudou) e a cada ação
/// enviada, e avisa as regiões por `stateChanged`. Nenhuma regra de domínio
/// vive aqui — toda mudança passa por `request`/`command` (intents do editor).
class PetuniaShellController : public QObject {
    Q_OBJECT

  public:
    explicit PetuniaShellController(QObject *parent = nullptr);
    ~PetuniaShellController() override;

    /// Abre a sessão do editor, lê o primeiro estado e inicia o relógio de 33 ms.
    void start();
    /// Encerra a sessão (idempotente).
    void stop();

    const QJsonObject &state() const { return m_state; }
    quint64 revision() const { return m_revision; }

    /// Envia uma ação tipada; o editor responde com o estado resultante.
    QJsonObject request(const QString &action, QJsonObject args = {});
    /// Executa um comando registrado (`CommandDispatcher`) pelo id.
    void command(const QString &id);
    /// Relê o estado (após ações externas ao controlador).
    void refresh();

    /// DRAW, POLY, PAINT, UV ou ANIMATE.
    QString workspaceId() const;
    bool isModeling() const;
    bool flag(const QString &path) const;
    QString text(const QString &path) const;
    /// Atalho efetivo (keymap do usuário) de uma ação, ou vazio.
    QString shortcut(const QString &keymapAction) const;
    /// Habilitação e motivo de um comando conforme o catálogo publicado.
    bool commandEnabled(const QString &id, QString *reason = nullptr) const;

  signals:
    void stateChanged(const QJsonObject &state);
    void errorReported(const QString &message);

  private:
    void adopt(const QJsonObject &state);
    void tick();

    QJsonObject m_state;
    quint64 m_revision{0};
    QTimer m_timer;
    QElapsedTimer m_clock;
    bool m_started{false};
};
