#pragma once

#include <QDialog>
#include <QLineEdit>
#include <QComboBox>
#include <QScrollArea>
#include <QGridLayout>
#include <QPushButton>
#include <QLabel>
#include <QVector>

struct AssetLibraryItem {
    QString id;
    QString name;
    QString category;
    QString icon;
    QString polycount;
    QString description;
    QString command;
};

class PetuniaAssetCard : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaAssetCard(const AssetLibraryItem &item, QWidget *parent = nullptr);
    ~PetuniaAssetCard() override = default;

    const AssetLibraryItem& item() const { return m_item; }

signals:
    void insertRequested(const QString &command);

protected:
    void mouseDoubleClickEvent(QMouseEvent *event) override;

private:
    AssetLibraryItem m_item;
    QPushButton *m_insertBtn{nullptr};
};

class PetuniaAssetLibraryWidget : public QDialog {
    Q_OBJECT

public:
    explicit PetuniaAssetLibraryWidget(QWidget *parent = nullptr);
    ~PetuniaAssetLibraryWidget() override = default;

signals:
    void assetSpawned(const QString &command);

public slots:
    void filterAssets();

private:
    void setupUi();
    void populateItems();

    QLineEdit *m_searchEdit{nullptr};
    QComboBox *m_categoryCombo{nullptr};
    QScrollArea *m_scrollArea{nullptr};
    QWidget *m_gridContainer{nullptr};
    QGridLayout *m_gridLayout{nullptr};

    QVector<AssetLibraryItem> m_allItems;
    QVector<PetuniaAssetCard*> m_cardWidgets;
};
