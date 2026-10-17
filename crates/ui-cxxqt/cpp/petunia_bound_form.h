#pragma once
#include "petunia_editor_api.h"
#include <QFormLayout>
#include <QImage>
#include <QJsonObject>
#include <QListWidget>
#include <QWidget>
#include <functional>

/// Imagem do documento (referências, prefabs) lida da ponte do editor.
QImage petuniaImage(const QString &kind, const QString &id);

/// Motor declarativo de formulários ligados ao snapshot do editor.
///
/// Cada controle lê uma chave do snapshot JSON e, ao confirmar, envia uma ação
/// tipada (`petuniaRequest`). Foi extraído do `PetuniaEditorPanel` para ser
/// reusado pelas seções de propriedades do Shell v2: os construtores abaixo
/// criam componentes do design system (`PetuniaNumberField`, `Vector3Field`) e
/// nunca mutam o documento — toda mudança passa pelo adaptador do editor.
class PetuniaBoundForm : public QWidget {
    Q_OBJECT
  public:
    explicit PetuniaBoundForm(QWidget *parent = nullptr);
    void sync(const QJsonObject &state);
    QStringList missingBindings(const QJsonObject &state) const;
    const QJsonObject &state() const { return m_state; }
    /// Canal de envio das ações. Sem ele, o formulário usa `petuniaRequest`
    /// diretamente (painéis legados); o Shell v2 injeta o controlador para que
    /// todas as regiões recebam o estado resultante.
    using Transport = std::function<QJsonObject(const QString &, const QJsonObject &)>;
    void setTransport(Transport transport) { m_transport = std::move(transport); }
  signals:
    void editorChanged();

  protected:
    void button(QFormLayout *, const QString &label, const QString &action,
                QJsonObject args = {}, const QString &enabledKey = {});
    void number(QFormLayout *, const QString &label, const QString &key,
                const QString &action, const QString &arg, qreal min, qreal max,
                qreal step = 0.01, QJsonObject args = {});
    void check(QFormLayout *, const QString &label, const QString &key,
               const QString &action,
               const QString &arg = QStringLiteral("value"),
               QJsonObject args = {});
    void choice(QFormLayout *, const QString &label, const QString &key,
                const QString &action, const QString &arg,
                const QStringList &ids, const QStringList &labels = {},
                QJsonObject args = {});
    void text(QFormLayout *, const QString &label, const QString &key,
              const QString &action, const QString &arg, QJsonObject args = {});
    void colorField(QFormLayout *, const QString &label, const QString &key,
                    const QString &action, const QString &arg, QJsonObject args = {});
    void xyz(QFormLayout *, const QString &label, const QString &keyPrefix,
             const QString &action,
             const QString &arg = QStringLiteral("text"),
             QJsonObject baseArgs = {});
    void rows(QFormLayout *, const QString &label, const QString &key,
              const QString &action, const QString &arg = QStringLiteral("id"),
              QJsonObject args = {});
    void fileButton(QFormLayout *, const QString &label, const QString &action,
                    const QString &filter, bool save = false,
                    QJsonObject args = {});
    void send(const QString &action, QJsonObject args = {});
    /// Gancho chamado ao fim de cada `sync` (conteúdo dinâmico, canvas...).
    virtual void afterSync(const QJsonObject &) {}

    struct Binding {
        QWidget *widget;
        QString key;
        QString enabledKey;
        QString labelKey;
    };
    QList<Binding> m_bindings;
    struct RowBinding {
        QListWidget *widget;
        QString key;
        QString action;
        QString arg;
        QJsonObject args;
    };
    QList<RowBinding> m_rows;
    QJsonObject m_state;
    Transport m_transport;
    bool m_syncing{false};
};
