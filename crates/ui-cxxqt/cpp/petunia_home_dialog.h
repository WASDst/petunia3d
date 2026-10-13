#pragma once

#include <QDialog>
#include <QLabel>
#include <QPushButton>
#include <QListWidget>
#include <QCheckBox>
#include <QVBoxLayout>
#include <QHBoxLayout>

class PetuniaHomeDialog : public QDialog {
    Q_OBJECT

public:
    explicit PetuniaHomeDialog(QWidget *parent = nullptr);
    ~PetuniaHomeDialog() override = default;

    void setRecentProjects(const QStringList &projectNames, const QStringList &projectPaths);
    void setRecoveryAvailable(bool available);
    bool showOnStart() const;
    void setShowOnStart(bool show);

signals:
    void newProjectRequested();
    void openProjectRequested();
    void recoverProjectRequested();
    void recentProjectSelected(const QString &path);
    void settingsRequested();

private slots:
    void onRecentItemDoubleClicked(QListWidgetItem *item);

private:
    void setupUi();

    QPushButton *m_newBtn{nullptr};
    QPushButton *m_openBtn{nullptr};
    QPushButton *m_recoverBtn{nullptr};
    QListWidget *m_recentList{nullptr};
    QCheckBox *m_showOnStartCheck{nullptr};
    QPushButton *m_settingsBtn{nullptr};
    QPushButton *m_closeBtn{nullptr};
    QLabel *m_noRecentLabel{nullptr};
};
