#pragma once

#include "../petunia_bound_form.h"
#include "petunia_shell_controller.h"

#include <QFormLayout>
#include <QJsonObject>
#include <QWidget>

class QLabel;
class QPushButton;

/// Seções nativas de Propriedades dos workspaces PAINT, UV e ANIMATE (Shell v2,
/// F7). Substituem as abas do painel legado: cada seção agrupa uma
/// responsabilidade canônica (Alvo, Pincel, Camada, Efeitos, Projeção…) e só
/// emite ações tipadas pelo transporte do controlador. Conteúdo que depende de
/// listas do documento (camada ativa, parâmetros do Motion) é reconstruído só
/// quando a estrutura muda; os valores seguem o snapshot a cada sincronização.

/// Base com uma área dinâmica ao fim do formulário.
class PetuniaDynamicForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaDynamicForm(QWidget *parent = nullptr);

  protected:
    QFormLayout *form() const { return m_form; }
    /// Marca o início da área dinâmica (linhas abaixo são refeitas).
    void beginDynamic();
    /// Assinatura da estrutura dinâmica e construção das linhas.
    virtual QString dynamicSignature(const QJsonObject &) const { return {}; }
    virtual void buildDynamic(const QJsonObject &) {}
    void afterSync(const QJsonObject &state) override;

  private:
    QFormLayout *m_form{nullptr};
    int m_fixedRows{-1};
    QString m_signature;
};

// ── PAINT ───────────────────────────────────────────────────────────────────

class PetuniaPaintTargetForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaPaintTargetForm(QWidget *parent = nullptr);
};

class PetuniaBrushForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaBrushForm(QWidget *parent = nullptr);

  protected:
    void afterSync(const QJsonObject &state) override;

  private:
    QPushButton *m_color{nullptr};
};

class PetuniaPaintLayerForm : public PetuniaDynamicForm {
    Q_OBJECT
  public:
    explicit PetuniaPaintLayerForm(QWidget *parent = nullptr);

  protected:
    QString dynamicSignature(const QJsonObject &state) const override;
    void buildDynamic(const QJsonObject &state) override;

  private:
    QLabel *m_title{nullptr};
};

class PetuniaPaintEffectsForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaPaintEffectsForm(QWidget *parent = nullptr);
};

class PetuniaPaintProjectionForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaPaintProjectionForm(QWidget *parent = nullptr);

  private:
    QString m_referenceAxis{QStringLiteral("front")};
};

class PetuniaDecalForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaDecalForm(QWidget *parent = nullptr);
};

class PetuniaPaintCanvasForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaPaintCanvasForm(QWidget *parent = nullptr);
};

// ── UV ──────────────────────────────────────────────────────────────────────

class PetuniaUvSelectionForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaUvSelectionForm(QWidget *parent = nullptr);

  protected:
    void afterSync(const QJsonObject &state) override;

  private:
    QLabel *m_stats{nullptr};
};

class PetuniaUvUnwrapForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaUvUnwrapForm(QWidget *parent = nullptr);
};

class PetuniaUvSeamsForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaUvSeamsForm(QWidget *parent = nullptr);
};

class PetuniaUvPackForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaUvPackForm(QWidget *parent = nullptr);
};

class PetuniaUvTransformForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaUvTransformForm(QWidget *parent = nullptr);
};

class PetuniaUvDisplayForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaUvDisplayForm(QWidget *parent = nullptr);
};

// ── ANIMATE ─────────────────────────────────────────────────────────────────

class PetuniaCreatureForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaCreatureForm(QWidget *parent = nullptr);
};

class PetuniaMotionForm : public PetuniaDynamicForm {
    Q_OBJECT
  public:
    explicit PetuniaMotionForm(QWidget *parent = nullptr);

  protected:
    QString dynamicSignature(const QJsonObject &state) const override;
    void buildDynamic(const QJsonObject &state) override;
};

class PetuniaPoseForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaPoseForm(QWidget *parent = nullptr);
};

class PetuniaAnimationLayersForm : public PetuniaDynamicForm {
    Q_OBJECT
  public:
    explicit PetuniaAnimationLayersForm(QWidget *parent = nullptr);

  protected:
    QString dynamicSignature(const QJsonObject &state) const override;
    void buildDynamic(const QJsonObject &state) override;
};

class PetuniaTimelineForm : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaTimelineForm(QWidget *parent = nullptr);
};

/// Página "Linha do tempo" da gaveta: barra de transporte (tocar, quadros,
/// keyframe) sempre acessível por teclado sobre o dope sheet.
class PetuniaTimelinePage : public QWidget {
    Q_OBJECT
  public:
    PetuniaTimelinePage(PetuniaShellController *controller, QWidget *canvas, QWidget *parent = nullptr);
    void sync(const QJsonObject &state);
    QPushButton *playButton() const { return m_play; }
    QPushButton *keyButton() const { return m_key; }

  private:
    PetuniaShellController *m_controller;
    QPushButton *m_play{nullptr};
    QPushButton *m_key{nullptr};
    QLabel *m_frame{nullptr};
};
