#pragma once

#include <QDialog>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QHBoxLayout>

class PetuniaRecoveryDialog : public QDialog {
    Q_OBJECT

public:
    explicit PetuniaRecoveryDialog(QWidget *parent = nullptr);
    ~PetuniaRecoveryDialog() override = default;

    void setSessionDetails(const QString &title, const QString &body, const QString &detail);

signals:
    void discardRequested();
    void keepRequested();
    void recoverRequested();

private:
    void setupUi();

    QLabel *m_titleLabel{nullptr};
    QLabel *m_bodyLabel{nullptr};
    QLabel *m_detailLabel{nullptr};
    QPushButton *m_discardBtn{nullptr};
    QPushButton *m_keepBtn{nullptr};
    QPushButton *m_recoverBtn{nullptr};
};
