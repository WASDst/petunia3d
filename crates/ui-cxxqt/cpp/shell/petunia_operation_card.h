#pragma once

#include "petunia_shell_controller.h"

#include <QFrame>
#include <QHash>

class PetuniaNumberField;
class PetuniaSegmented;
class QCheckBox;
class QComboBox;
class QLabel;
class QPushButton;
class QVBoxLayout;

/// Cartão de uma operação do Operation Graph (ADR-C13), **gerado do descriptor**:
/// cada parâmetro vira o controle do seu tipo (escolha, booleano, número tipado)
/// a partir de `specs` publicado pelo editor. Um nó novo no núcleo aparece aqui
/// sem código Qt novo. Toda alteração é uma ação `graph_*` (comando com Undo).
class PetuniaOperationCard : public QFrame {
    Q_OBJECT

  public:
    PetuniaOperationCard(PetuniaShellController *controller, const QJsonObject &row, bool asTool,
                         QWidget *parent = nullptr);

    /// Atualiza valores/estados sem recriar controles (sinais bloqueados).
    void update(const QJsonObject &row, const QJsonArray &candidates);
    const QString &nodeId() const { return m_nodeId; }
    /// Estrutura que exige reconstrução quando muda.
    static QString signatureOf(const QJsonObject &row, bool asTool);

  private:
    struct ParamControl {
        QString key;
        QString kind;
        QWidget *widget{nullptr};
    };
    void buildParams(const QJsonArray &specs);
    void rebuildTargets(const QJsonArray &candidates);
    void sendParam(const QString &key, const QJsonValue &value, bool integer = false);

    PetuniaShellController *m_controller;
    QString m_nodeId;
    bool m_asTool;
    QVBoxLayout *m_layout{nullptr};
    QCheckBox *m_enabled{nullptr};
    QLabel *m_subtitle{nullptr};
    QLabel *m_error{nullptr};
    QWidget *m_targets{nullptr};
    QList<ParamControl> m_params;
    QHash<QString, QCheckBox *> m_targetBoxes;
    QString m_targetSignature;
    bool m_syncing{false};
};
