#pragma once

#include "petunia_layout_store.h"
#include "petunia_shell_controller.h"

#include <QMainWindow>
#include <QPointer>
#include <QTimer>

class PetuniaAlignPopover;
class PetuniaAppBar;
class PetuniaContextActions;
class PetuniaDrawer;
class PetuniaEditorCanvas;
class PetuniaEditorPanel;
class PetuniaEditorCanvas;
class PetuniaTimelinePage;
class PetuniaGlWidget;
class PetuniaNodeCanvas;
class PetuniaNodePage;
class PetuniaOperationHud;
class PetuniaPropertiesPanel;
class PetuniaRegionNavigator;
class PetuniaStatusBar;
class PetuniaStructurePanel;
class PetuniaToolRail;
class PetuniaViewportHeader;
class PetuniaCommandPalette;
class QSplitter;
class QStackedWidget;

/// Shell v2 da GUI Qt (ADR-C14): regiões fixas — barra do app, rail, vista com
/// cabeçalho, HUD e ações contextuais, estrutura, propriedades, gaveta e status —
/// com layout do usuário versionado. Não contém regra de domínio: observa o
/// estado do editor pelo `PetuniaShellController` e emite intents.
class PetuniaShellWindow : public QMainWindow {
    Q_OBJECT

  public:
    /// `interactive = false` (testes/capturas): nunca abre diálogos modais por conta própria.
    explicit PetuniaShellWindow(QWidget *parent = nullptr, bool interactive = true);
    ~PetuniaShellWindow() override;

    PetuniaShellController *controller() { return &m_controller; }
    PetuniaAppBar *appBar() const { return m_appBar; }
    PetuniaToolRail *toolRail() const { return m_rail; }
    PetuniaViewportHeader *viewportHeader() const { return m_header; }
    PetuniaGlWidget *viewport() const { return m_viewport; }
    /// Canvas 2D de um workspace especializado (PAINT/UV/ANIMATE), se já criado.
    QWidget *workspaceCanvas(const QString &workspace) const;
    PetuniaTimelinePage *timelinePage() const { return m_timelinePage; }
    PetuniaOperationHud *hud() const { return m_hud; }
    PetuniaContextActions *contextActions() const { return m_context; }
    PetuniaStructurePanel *structure() const { return m_structure; }
    PetuniaPropertiesPanel *properties() const { return m_properties; }
    PetuniaDrawer *drawer() const { return m_drawer; }
    PetuniaStatusBar *statusRegion() const { return m_status; }
    PetuniaRegionNavigator *navigator() const { return m_navigator; }
    PetuniaLayoutStore *layoutStore() { return &m_layoutStore; }
    QSplitter *bodySplitter() const { return m_body; }
    QSplitter *surfaceSplitter() const { return m_surface; }
    /// Região que contém o foco atual (-1 se nenhuma).
    int focusedRegion() const;
    /// Ação local equivalente ao item de menu (arquivo, painéis, layout).
    void triggerLocalAction(const QString &id);
    /// Força a leitura e a aplicação do estado (usado por testes e capturas).
    void syncNow();
    PetuniaNodeCanvas *nodeCanvas() const;
    PetuniaNodePage *nodePage() const { return m_nodePage; }
    /// Abre a gaveta "Nós" no grafo de operações ou no do material ativo.
    void openNodeEditor(bool material);
    /// Diálogos do host: o foco entra no diálogo e volta ao invocador ao fechar.
    void openPalette();
    void openPreferences();
    void openReferences();
    PetuniaDrawer *drawerRegion() const { return m_drawer; }
    /// Painel de alinhar e distribuir (criado sob demanda), aberto sob `anchor`.
    void openAlign(QWidget *anchor = nullptr);
    PetuniaAlignPopover *alignPopover() const { return m_align; }

  protected:
    bool eventFilter(QObject *watched, QEvent *event) override;
    void closeEvent(QCloseEvent *event) override;
    void showEvent(QShowEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;

  private:
    void buildUi();
    void onState(const QJsonObject &state);
    void onWorkspaceChanged(const QString &workspace);
    void retranslateUi();
    void arrangeSurface(const QString &workspace);
    /// PAINT/UV: "3d" | "split" | "2d"; `ratio` = fração da largura da vista 3D.
    void applySurfaceMode(const QString &mode, double ratio);
    void updateRegions();
    void ensureSpecialized(const QString &workspace);
    void applyLayout(const QString &workspace);
    void captureLayout();
    void scheduleLayoutSave();
    void repositionOverlays();
    void openHostDialogs(const QJsonObject &state);
    bool handleShortcut(QKeyEvent *key);
    bool confirmDiscard();

    void newProject();
    void openProject();
    bool saveProject();
    bool saveProjectAs();
    void exportTo(const QString &kind);
    void openAssets();
    void openHome();
    void openRecovery();
    void showAbout();

    PetuniaShellController m_controller;
    PetuniaLayoutStore m_layoutStore;
    QString m_workspace;
    bool m_applyingLayout{false};
    bool m_hostDialogPending{false};
    QTimer m_layoutTimer;

    PetuniaAppBar *m_appBar{nullptr};
    PetuniaToolRail *m_rail{nullptr};
    QSplitter *m_body{nullptr};
    QSplitter *m_middle{nullptr};
    QSplitter *m_right{nullptr};
    QSplitter *m_surface{nullptr};
    QWidget *m_workSurface{nullptr};
    QWidget *m_viewArea{nullptr};
    QWidget *m_secondaryArea{nullptr};
    PetuniaViewportHeader *m_header{nullptr};
    PetuniaGlWidget *m_viewport{nullptr};
    PetuniaGlWidget *m_secondary{nullptr};
    PetuniaOperationHud *m_hud{nullptr};
    PetuniaContextActions *m_context{nullptr};
    PetuniaStructurePanel *m_structure{nullptr};
    PetuniaPropertiesPanel *m_properties{nullptr};
    PetuniaDrawer *m_drawer{nullptr};
    PetuniaStatusBar *m_status{nullptr};
    PetuniaRegionNavigator *m_navigator{nullptr};
    PetuniaCommandPalette *m_palette{nullptr};
    PetuniaAlignPopover *m_align{nullptr};

    /// Superfícies 2D dos workspaces (canvas de pintura, editor UV).
    QHash<QString, PetuniaEditorCanvas *> m_canvases;
    /// ANIMATE: dope sheet dentro da página da linha do tempo (gaveta).
    PetuniaEditorCanvas *m_timelineCanvas{nullptr};
    PetuniaTimelinePage *m_timelinePage{nullptr};
    QWidget *m_assetsHost{nullptr};
    PetuniaNodePage *m_nodePage{nullptr};
    QPointer<QWidget> m_paletteInvoker;
    PetuniaEditorPanel *m_assetsPanel{nullptr};
    QString m_projectPath;
    QString m_language;
    bool m_firstShow{true};
    bool m_interactive{true};
};
