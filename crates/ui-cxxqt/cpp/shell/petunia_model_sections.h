#pragma once

#include "../petunia_bound_form.h"
#include "petunia_shell_controller.h"

#include <QHash>
#include <QLabel>
#include <QList>
#include <QPointer>
#include <QPushButton>

class PetuniaOperationCard;
class PetuniaSegmented;
class QCheckBox;

/// Seção Transform: posição/rotação/escala com campos tipados e pivô.
class PetuniaTransformForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaTransformForm(QWidget *parent = nullptr);
};

/// Seção Sombreamento: facetado ou suave (resumo e estatísticas ficam na barra de status).
class PetuniaGeometryForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaGeometryForm(QWidget *parent = nullptr);
};

/// Seção Material: slots, cor base e parâmetros PBR do material ativo.
class PetuniaMaterialForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaMaterialForm(QWidget *parent = nullptr);

  signals:
    /// Abrir o editor de nós no grafo do material ativo (O05).
    void nodeEditorRequested();
};

/// Perfil 2D e volume por extrusão/revolução/sweep (workspace DRAW).
class PetuniaProfileForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaProfileForm(QWidget *parent = nullptr);
};

/// Pilha de operações do objeto (vista "Pilha" do Operation Graph,
/// docs/03-geometry/operation-graph.md): cartões gerados dos descriptors para as
/// operações aplicadas ao objeto e as que ele fornece como ferramenta, mais a
/// pilha de modificadores legada (arquivos antigos). Bake = Make Editable.
class PetuniaOperationsPanel : public QWidget {
    Q_OBJECT
  public:
    explicit PetuniaOperationsPanel(PetuniaShellController *controller, QWidget *parent = nullptr);
    void sync(const QJsonObject &state);
    /// Operações exibidas (grafo + legado) para o resumo do cabeçalho.
    int operationCount() const { return m_count; }
    const QList<PetuniaOperationCard *> &appliedCards() const { return m_applied; }
    const QList<PetuniaOperationCard *> &toolCards() const { return m_tools; }
    QPushButton *bakeButton() const { return m_bake; }

  signals:
    void countChanged(int count);

  private:
    struct Row {
        QString id;
        QString kind;
        QWidget *card{nullptr};
        QCheckBox *enabled{nullptr};
        PetuniaSegmented *axis{nullptr};
        QCheckBox *direction{nullptr};
        QPushButton *up{nullptr};
        QPushButton *down{nullptr};
    };
    void rebuildGraph(const QJsonArray &applied, const QJsonArray &tools);
    void rebuildLegacy(const QJsonArray &rows);
    void updateLegacy(const QJsonArray &rows);
    Row makeLegacyRow(const QJsonObject &row);
    QLabel *heading(const QString &text);

    PetuniaShellController *m_controller;
    QVBoxLayout *m_appliedLayout{nullptr};
    QVBoxLayout *m_toolLayout{nullptr};
    QVBoxLayout *m_legacyLayout{nullptr};
    QLabel *m_toolHeading{nullptr};
    QLabel *m_legacyHeading{nullptr};
    QLabel *m_empty{nullptr};
    QPushButton *m_bake{nullptr};
    QList<PetuniaOperationCard *> m_applied;
    QList<PetuniaOperationCard *> m_tools;
    QList<Row> m_rows;
    QString m_graphSignature;
    QString m_legacySignature;
    int m_count{0};
    bool m_syncing{false};
};
