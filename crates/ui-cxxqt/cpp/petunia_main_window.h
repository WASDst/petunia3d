#pragma once

#include <QMainWindow>
#include <QDockWidget>
#include <QToolBar>
#include <QTreeView>
#include <QStandardItemModel>
#include <QSortFilterProxyModel>
#include <QLineEdit>
#include <QSlider>
#include <QLabel>
#include <QPushButton>
#include <QDoubleSpinBox>
#include <QSplitter>
#include <QComboBox>
#include <QVBoxLayout>
#include "petunia_gl_widget.h"

class PetuniaMainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit PetuniaMainWindow(QWidget *parent = nullptr);
    ~PetuniaMainWindow() override;

    PetuniaGlWidget* viewport() const { return m_viewport; }

public slots:
    void syncFromApplication();
    void openCommandPalette();
    void openHomeDialog();
    void openRecoveryDialog();
    void openAddMenu();
    void toggleSplitView();
    void toggleMicroInspector();
    void showOperationHud(const QString &opName, const QString &details);
    void setSecondaryCameraPreset(const QString &preset);
    void setWorkspace(const QString &workspace);
    void setSelectionDomain(int domain);
    void pickMaterialColor();
    void updateSelectedCount(int count);
    void updateDirtyState(bool isDirty);
    void setStatusMessage(const QString &message);
    void populateOutliner(const QStringList &objectNames, const QList<bool> &visibilities);
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

private:
    void setupUi();
    void setupMenuBar();
    void setupWorkspaceBar();
    void setupToolRail();
    void setupDocks();
    void setupStatusBar();
    void applyDarkTheme();

    PetuniaGlWidget *m_viewport{nullptr};
    QSplitter *m_viewportSplitter{nullptr};
    QWidget *m_secondaryViewportContainer{nullptr};
    PetuniaGlWidget *m_secondaryViewport{nullptr};

    // Docas Destacáveis (Multi-Monitor Suportado)
    QDockWidget *m_outlinerDock{nullptr};
    QDockWidget *m_inspectorDock{nullptr};
    QDockWidget *m_timelineDock{nullptr};
    QDockWidget *m_paintDock{nullptr};
    QDockWidget *m_uvDock{nullptr};

    class PetuniaPaintPanel *m_paintPanel{nullptr};
    class PetuniaUvPanel *m_uvPanel{nullptr};
    class PetuniaTimelineWidget *m_timelineWidget{nullptr};
    class PetuniaAssetLibraryWidget *m_assetLibraryWidget{nullptr};
    class PetuniaPrimitiveCard *m_primitiveCard{nullptr};
    class PetuniaLoopCutCard *m_loopCutCard{nullptr};
    class PetuniaToolOptionsBar *m_toolOptionsBar{nullptr};
    class PetuniaMicroInspector *m_microInspector{nullptr};
    class PetuniaOperationHud *m_operationHud{nullptr};
    class PetuniaAddMenu *m_addMenu{nullptr};
    class PetuniaHomeDialog *m_homeDialog{nullptr};
    class PetuniaRecoveryDialog *m_recoveryDialog{nullptr};

    // Componentes da Árvore de Cena (Outliner)
    QTreeView *m_outlinerTree{nullptr};
    QStandardItemModel *m_outlinerModel{nullptr};
    QSortFilterProxyModel *m_outlinerProxy{nullptr};
    QLineEdit *m_outlinerSearch{nullptr};

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
