#pragma once

#include <QDialog>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QVBoxLayout>
#include <QString>

class PetuniaCommandPalette : public QDialog {
    Q_OBJECT

public:
    explicit PetuniaCommandPalette(QWidget *parent = nullptr);
    ~PetuniaCommandPalette() override = default;

signals:
    void commandSelected(const QString &commandId);

protected:
    void keyPressEvent(QKeyEvent *event) override;
    void showEvent(QShowEvent *event) override;

private slots:
    void onSearchTextChanged(const QString &text);
    void onItemActivated(QListWidgetItem *item);

private:
    void setupUi();
    void updateResults(const QString &query);

    QLineEdit *m_searchInput{nullptr};
    QListWidget *m_resultsList{nullptr};
    QLabel *m_emptyLabel{nullptr};
};
