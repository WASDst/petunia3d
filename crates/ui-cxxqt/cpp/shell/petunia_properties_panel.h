#pragma once

#include "petunia_shell_controller.h"

#include <QHash>
#include <QJsonObject>
#include <QWidget>

class PetuniaBoundForm;
class PetuniaOperationsPanel;
class PetuniaPropertySection;
class QLabel;
class QScrollArea;
class QStackedWidget;
class QToolButton;
class QVBoxLayout;

/// Região "Propriedades": seções contextuais por workspace (design system). Em
/// modelagem: Transformação, Geometria, Operações, Material…; em PAINT: Alvo,
/// Pincel, Camada, Efeitos, Projeção, Decalque, Canvas; em UV: Seleção,
/// Desdobrar, Costuras, Empacotar, Transformar, Exibição; em ANIMATE: Criatura,
/// Motion, Pose, Camadas, Linha do tempo. Cada seção declara seus workspaces.
class PetuniaPropertiesPanel : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaPropertiesPanel(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    /// Estado aberto/fechado das seções (preferência do layout).
    QJsonObject sectionStates() const;
    void applySectionStates(const QJsonObject &states);
    PetuniaPropertySection *section(const QString &id) const { return m_sectionMap.value(id); }
    QStringList visibleSectionIds() const;
    PetuniaOperationsPanel *operations() const { return m_operations; }

  signals:
    void sectionToggled(const QString &id, bool open);
    /// "Editar nós do material" na seção Material.
    void materialNodesRequested();

  private:
    PetuniaPropertySection *addSection(const QString &id, const QString &title, QWidget *content, bool open,
                                       const QStringList &workspaces = {QStringLiteral("DRAW"), QStringLiteral("POLY")});
    void buildModeling();
    void buildPaint();
    void buildUv();
    void buildAnimate();
    PetuniaBoundForm *bound(PetuniaBoundForm *form, const QString &workspace);

    PetuniaShellController *m_controller;
    QStackedWidget *m_stack{nullptr};
    QScrollArea *m_scroll{nullptr};
    QWidget *m_column{nullptr};
    QVBoxLayout *m_layout{nullptr};
    QLabel *m_empty{nullptr};
    QHash<QString, PetuniaPropertySection *> m_sectionMap;
    QHash<QString, QStringList> m_sectionWorkspaces;
    /// Formulários por workspace: só os do workspace atual sincronizam.
    QHash<QString, QList<PetuniaBoundForm *>> m_forms;
    PetuniaOperationsPanel *m_operations{nullptr};
};
