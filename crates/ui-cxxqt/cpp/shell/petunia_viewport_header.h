#pragma once

#include "petunia_shell_controller.h"

#include <QHash>
#include <QToolButton>
#include <QWidget>

class PetuniaSegmented;
class QHBoxLayout;

/// Cabeçalho da vista: só controles de vista e o domínio de seleção (newdocs
/// UI-001 §4 — projeção, exibição, overlays, navegação). Nenhum controle de
/// ferramenta mora aqui. Abaixo de ~760 px os menus ficam só com ícone.
class PetuniaViewportHeader : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaViewportHeader(PetuniaShellController *controller, QWidget *parent = nullptr);

    void sync(const QJsonObject &state);

  signals:
    void surfaceModeRequested(const QString &mode);

  public:
    PetuniaSegmented *domain() const { return m_domain; }
    PetuniaSegmented *projection() const { return m_projection; }
    /// PAINT/UV: 3D, dividido ou só a superfície 2D (preferência de layout).
    PetuniaSegmented *surfaceMode() const { return m_surfaceMode; }
    void setSurfaceMode(const QString &mode);
    bool compact() const { return m_compact; }
    /// Botão da grade: clique liga/desliga; a seta abre tamanho, ladrilho, opacidade e eixos (B6).
    QToolButton *gridButton() const { return m_grid; }
    QWidget *gridOptions() const { return m_gridOptions; }

  protected:
    void resizeEvent(QResizeEvent *event) override;

  private:
    QToolButton *makeMenuButton(const QString &icon, const QString &accessible);
    QToolButton *makeToggle(const QString &icon, const QString &tip);
    void buildDisplayMenu();
    void buildOverlaysMenu();
    void buildCameraMenu();
    void buildGridMenu();
    void syncGrid(const QJsonObject &grid);
    void commitGrid();
    void applyMetrics();
    void applyCompact(bool compact);
    QString shortcutText(const QString &action) const;

    PetuniaShellController *m_controller;
    QHBoxLayout *m_layout{nullptr};
    PetuniaSegmented *m_domain{nullptr};
    QToolButton *m_snap{nullptr};
    QToolButton *m_proportional{nullptr};
    PetuniaSegmented *m_projection{nullptr};
    PetuniaSegmented *m_surfaceMode{nullptr};
    QToolButton *m_display{nullptr};
    QToolButton *m_overlays{nullptr};
    QToolButton *m_camera{nullptr};
    QToolButton *m_split{nullptr};
    QToolButton *m_grid{nullptr};
    QWidget *m_gridOptions{nullptr};
    class PetuniaNumberField *m_gridSize{nullptr};
    class PetuniaNumberField *m_gridSpacing{nullptr};
    class PetuniaNumberField *m_gridOpacity{nullptr};
    class QCheckBox *m_gridAxes[3]{};
    QWidget *m_domainSeparator{nullptr};
    QHash<QString, QAction *> m_shadingActions;
    QHash<QString, QAction *> m_overlayActions;
    bool m_compact{false};
    bool m_syncing{false};
};
