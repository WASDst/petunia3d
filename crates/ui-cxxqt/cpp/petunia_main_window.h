#pragma once

#include "petunia_editor_panel.h"
#include "petunia_gl_widget.h"
#include <QCloseEvent>
#include <QComboBox>
#include <QDockWidget>
#include <QDoubleSpinBox>
#include <QElapsedTimer>
#include <QLabel>
#include <QLineEdit>
#include <QMainWindow>
#include <QPointer>
#include <QPushButton>
#include <QSlider>
#include <QSortFilterProxyModel>
#include <QSplitter>
#include <QStackedWidget>
#include <QStandardItemModel>
#include <QToolBar>
#include <QTreeView>
#include <QVBoxLayout>

class PetuniaMainWindow : public QMainWindow {
    Q_OBJECT

  public:
    explicit PetuniaMainWindow(QWidget *parent = nullptr);
    ~PetuniaMainWindow() override;

    PetuniaGlWidget *viewport() const { return m_viewport; }
    class PetuniaDockManager *dockManager() const { return m_dockManager; }

  public slots:
    void syncFromApplication();
    void openCommandPalette();
    void openHomeDialog();
    void openRecoveryDialog();
    void openAddMenu();
    void toggleSplitView();
    void toggleMicroInspector();
    void showOperationHud(const QString &opName, const QString &details);
    void setPrimaryCameraPreset(const QString &preset);
    void setSecondaryCameraPreset(const QString &preset);
    void setWorkspace(const QString &workspace);
    void setSelectionDomain(int domain);
    void pickMaterialColor();
    void updateSelectedCount(int count);
    void updateDirtyState(bool isDirty);
    void setStatusMessage(const QString &message);
    void populateOutliner(const QStringList &objectNames,
                          const QList<bool> &visibilities);
    void onOutlinerContextMenu(const QPoint &pos);
    void newProject();
    void openProject();
    void saveProject();
    void saveProjectAs();
    void exportGltf();
    void exportObj();
    void openPreferences();
    void openReferenceManager();
    void showPrimitiveCard(const QString &type);
    void showLoopCutCard();

  signals:
    void workspaceChanged(const QString &workspace);
    void actionTriggered(const QString &action);
    void objectSelected(int index);
    void objectVisibilityToggled(int index, bool visible);

  protected:
    bool eventFilter(QObject *, QEvent *) override;
    void closeEvent(QCloseEvent *) override;

  private:
    void syncEditorState(const QJsonObject &);
    void applyPreferences(const QJsonObject &);
    void updateToolRail(const QString &, const QJsonObject &);
    QString m_toolRailMode;
    bool m_hostDialogPending{false};
    bool confirmDiscard();
    QList<PetuniaEditorPanel *> m_editorPanels;
    QJsonObject m_editorState;
    QElapsedTimer m_frameClock;
    QString m_themeSignature;
    void setupUi();
    void setupMenuBar();
    void setupWorkspaceBar();
    void setupToolRail();
    void setupDocks();
    void setupInspector();
    void setupStatusBar();
    void applyDarkTheme();
    void repositionFloatingBars();

protected:
    void showEvent(QShowEvent *event) override;

private:
    PetuniaGlWidget *m_viewport{nullptr};
    QStackedWidget *m_primaryStack{nullptr};
    PetuniaEditorCanvas *m_primaryPaintCanvas{nullptr};
    PetuniaGlWidget *m_pipViewport{nullptr};
    QSplitter *m_viewportSplitter{nullptr};
    QWidget *m_secondaryViewportContainer{nullptr};
    PetuniaGlWidget *m_secondaryViewport{nullptr};

    // Docas Destacáveis (Multi-Monitor Suportado via PetuniaDockManager)
    class PetuniaDockManager *m_dockManager{nullptr};
    QDockWidget *m_outlinerDock{nullptr};
    QDockWidget *m_inspectorDock{nullptr};
    QDockWidget *m_timelineDock{nullptr};
    QDockWidget *m_paintDock{nullptr};
    QDockWidget *m_uvDock{nullptr};

    class PetuniaPrimitiveCard *m_primitiveCard{nullptr};
    class PetuniaLoopCutCard *m_loopCutCard{nullptr};
    class PetuniaToolOptionsBar *m_toolOptionsBar{nullptr};
    class PetuniaFloatingContextBar *m_contextBar{nullptr};
    class PetuniaMicroInspector *m_microInspector{nullptr};
    class PetuniaOperationHud *m_operationHud{nullptr};
    class PetuniaAddMenu *m_addMenu{nullptr};
    class PetuniaPieMenu *m_pieMenu{nullptr};
    class PetuniaHomeDialog *m_homeDialog{nullptr};
    class PetuniaRecoveryDialog *m_recoveryDialog{nullptr};
    QPointer<QDialog> m_referenceDialog{nullptr};

    // Componentes da Árvore de Cena (Outliner)
    QTreeView *m_outlinerTree{nullptr};
    class PetuniaPlaceholderMessage *m_outlinerPlaceholder{nullptr};
    QStandardItemModel *m_outlinerModel{nullptr};
    QSortFilterProxyModel *m_outlinerProxy{nullptr};
    QLineEdit *m_outlinerSearch{nullptr};
    QPushButton *m_outlinerFilterBtn{nullptr};
    QPushButton *m_outlinerSortBtn{nullptr};
    QPushButton *m_outlinerSeparateBtn{nullptr};
    QSlider *m_outlinerRowSizeSlider{nullptr};
    int m_outlinerRowHeight{22};
    QMenuBar *m_headerMenuBar{nullptr};

    // Componentes do Inspetor (Transform & Material)
    QDoubleSpinBox *m_posX{nullptr};
    QDoubleSpinBox *m_posY{nullptr};
    QDoubleSpinBox *m_posZ{nullptr};
    QDoubleSpinBox *m_rotX{nullptr};
    QDoubleSpinBox *m_rotY{nullptr};
    QDoubleSpinBox *m_rotZ{nullptr};
    QDoubleSpinBox *m_scaleX{nullptr};
    QDoubleSpinBox *m_scaleY{nullptr};
    QDoubleSpinBox *m_scaleZ{nullptr};
    QPushButton *m_colorBtn{nullptr};
    QSlider *m_roughnessSlider{nullptr};
    QLabel *m_roughnessLabel{nullptr};
    QSlider *m_metallicSlider{nullptr};
    QLabel *m_metallicLabel{nullptr};
    QDoubleSpinBox *m_emissionSpin{nullptr};
    QComboBox *m_profileCombo{nullptr};
    QPushButton *m_flatShadingBtn{nullptr};
    QPushButton *m_smoothShadingBtn{nullptr};
    QWidget *m_modifiersContainer{nullptr};
    QVBoxLayout *m_modifiersListLayout{nullptr};
    bool m_syncingTransforms{false};
    bool m_syncingOutliner{false};
    bool m_syncingMaterial{false};

    void refreshModifiers();
    void refreshMaterialPbr();
    void openAssetLibrary();

    // Botões de Seleção de Domínio (Objeto / Vértice / Aresta / Face)
    QPushButton *m_domainObjBtn{nullptr};
    QPushButton *m_domainVertBtn{nullptr};
    QPushButton *m_domainEdgeBtn{nullptr};
    QPushButton *m_domainFaceBtn{nullptr};

    // Barra de Ferramentas Vertical (Tool Rail)
    QToolBar *m_toolRailBar{nullptr};

    // Barra de Status
    QLabel *m_statusLabel{nullptr};
    QLabel *m_selectionLabel{nullptr};
    QLabel *m_statsLabel{nullptr};
    QLabel *m_modeBadge{nullptr};

    class PetuniaCommandPalette *m_commandPalette{nullptr};

    QString m_activeWorkspace{"DRAW"};
    int m_activeDomain{0};
    QString m_currentProjectPath;
};
