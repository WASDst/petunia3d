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

/// Região "Propriedades": seções contextuais do objeto selecionado. Em
/// modelagem são seções do design system (Transform, Geometria, Operações,
/// Material…); nos demais workspaces hospeda as abas dos painéis especializados
/// enquanto não migram para seções (F7).
class PetuniaPropertiesPanel : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaPropertiesPanel(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);
    /// Hospeda o conteúdo de um workspace não-modelagem (PAINT/UV/ANIMATE).
    void setHosted(const QString &workspaceId, QWidget *widget);
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
    PetuniaPropertySection *addSection(const QString &id, const QString &title, QWidget *content, bool open);
    void buildModeling();

    PetuniaShellController *m_controller;
    QStackedWidget *m_stack{nullptr};
    QScrollArea *m_scroll{nullptr};
    QWidget *m_column{nullptr};
    QVBoxLayout *m_layout{nullptr};
    QLabel *m_empty{nullptr};
    QHash<QString, PetuniaPropertySection *> m_sectionMap;
    QList<PetuniaBoundForm *> m_forms;
    PetuniaOperationsPanel *m_operations{nullptr};
    QHash<QString, QWidget *> m_hosted;
};
