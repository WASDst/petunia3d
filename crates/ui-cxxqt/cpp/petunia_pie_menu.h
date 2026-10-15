#pragma once

#include <QWidget>
#include <QString>
#include <QVector>
#include <QIcon>
#include <QPoint>

struct PetuniaPieItem {
    QString id;
    QString label;
    QString iconName;
    float startAngle;
    float spanAngle;
};

class PetuniaPieMenu : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaPieMenu(QWidget *parent = nullptr);
    ~PetuniaPieMenu() override = default;

    void addItem(const QString &id, const QString &label, const QString &iconName);
    void execAt(const QPoint &globalPos);

  signals:
    void primitiveSelected(const QString &id);

  protected:
    void paintEvent(QPaintEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void focusOutEvent(QFocusEvent *event) override;

  private:
    int itemAt(const QPoint &pos) const;
    void recalculateAngles();

    QVector<PetuniaPieItem> m_items;
    int m_hoveredIndex{-1};
    int m_radius{125};
    int m_innerRadius{38};
    QPoint m_center;
};
