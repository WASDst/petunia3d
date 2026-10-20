#pragma once

#include <QWidget>
#include <QLabel>
#include <QToolButton>
#include <QVBoxLayout>
#include <QHBoxLayout>

/// Aligned label-value row for property inspectors.
class PropertyRow : public QWidget {
    Q_OBJECT

public:
    explicit PropertyRow(const QString &label, QWidget *editorWidget, QWidget *parent = nullptr);
    ~PropertyRow() override = default;

    QLabel *labelWidget() const { return m_label; }
    QWidget *editorWidget() const { return m_editor; }

private:
    QLabel *m_label{nullptr};
    QWidget *m_editor{nullptr};
};

/// Collapsible semantic group for property panels in Petunia3D UI-001.
class PropertyGroup : public QWidget {
    Q_OBJECT

public:
    explicit PropertyGroup(const QString &title, QWidget *parent = nullptr);
    ~PropertyGroup() override = default;

    const QString &title() const { return m_title; }
    void setTitle(const QString &title);

    bool isExpanded() const { return m_expanded; }
    void setExpanded(bool expanded);

    void addRow(const QString &label, QWidget *widget);
    void addWidget(QWidget *widget);
    void addLayout(QLayout *layout);

signals:
    void toggled(bool expanded);

private:
    void updateHeader();

    QString m_title;
    bool m_expanded{true};

    QWidget *m_headerWidget{nullptr};
    QToolButton *m_toggleButton{nullptr};
    QLabel *m_titleLabel{nullptr};
    QWidget *m_contentContainer{nullptr};
    QVBoxLayout *m_contentLayout{nullptr};
};
