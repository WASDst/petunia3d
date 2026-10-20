#pragma once

#include "petunia_node_canvas.h"
#include "petunia_shell_controller.h"

#include <QJsonObject>
#include <QWidget>

class PetuniaOperationCard;
class PetuniaSegmented;
class QLabel;
class QPushButton;
class QScrollArea;
class QVBoxLayout;

/// Página "Nós" da gaveta: alterna o grafo exibido (Operações | Material), traz as
/// ações do domínio (criar receita, tornar estático) e um painel com os
/// parâmetros do nó selecionado, gerados do descriptor (mesmo cartão da Pilha).
class PetuniaNodePage : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaNodePage(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    void setDomain(PetuniaNodeCanvas::Domain domain);
    PetuniaNodeCanvas *canvas() const { return m_canvas; }
    PetuniaSegmented *domainSwitch() const { return m_domain; }
    PetuniaOperationCard *parameterCard() const { return m_card; }
    QPushButton *createRecipeButton() const { return m_create; }
    QPushButton *bakeButton() const { return m_bake; }
    QLabel *statusLabel() const { return m_status; }

  private:
    void refreshPanel();
    QJsonObject selectedRow(QJsonArray *candidates) const;

    PetuniaShellController *m_controller;
    PetuniaNodeCanvas *m_canvas{nullptr};
    PetuniaSegmented *m_domain{nullptr};
    QLabel *m_status{nullptr};
    QPushButton *m_create{nullptr};
    QPushButton *m_bake{nullptr};
    QScrollArea *m_panel{nullptr};
    QWidget *m_panelBody{nullptr};
    QVBoxLayout *m_panelLayout{nullptr};
    QLabel *m_panelHint{nullptr};
    PetuniaOperationCard *m_card{nullptr};
    QString m_cardSignature;
    QJsonObject m_state;
};
