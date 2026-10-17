#pragma once

#include <QWidget>
#include <QLineEdit>
#include <QLabel>
#include <QString>
#include <QMouseEvent>
#include <QKeyEvent>

/// Reusable numeric input field conforming to Petunia3D UI-001 design system.
/// Supports f64 values, contextual formatting without trailing zero bloat,
/// keyboard stepping (Up/Down, Shift=10x, Ctrl=0.1x), scrub dragging,
/// explicit commit (Enter) and rollback (Esc), and mixed/indeterminate states.
class PetuniaNumberField : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaNumberField(QWidget *parent = nullptr);
    explicit PetuniaNumberField(const QString &prefix, QWidget *parent = nullptr);
    ~PetuniaNumberField() override = default;

    double value() const { return m_value; }
    void setValue(double val);

    bool isMixed() const { return m_isMixed; }
    void setMixed(bool mixed);

    double minimum() const { return m_min; }
    double maximum() const { return m_max; }
    void setRange(double min, double max);

    double singleStep() const { return m_step; }
    void setSingleStep(double step) { m_step = step; }

    const QString &suffix() const { return m_suffix; }
    void setSuffix(const QString &suffix);

    const QString &prefix() const { return m_prefix; }
    void setPrefix(const QString &prefix);

    const QString &propertyId() const { return m_propertyId; }
    void setPropertyId(const QString &id) { m_propertyId = id; }

    int maxDecimals() const { return m_maxDecimals; }
    void setMaxDecimals(int dec) { m_maxDecimals = dec; updateDisplay(); }

    void setPrefixColor(const QColor &color);

signals:
    void valueChanged(double value);
    void editStarted(const QString &propertyId);
    void previewRequested(const QString &propertyId, double value);
    void commitRequested(const QString &propertyId, double value, const QString &text = QString());
    void editCanceled(const QString &propertyId);

protected:
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    void updateDisplay();
    void commitEdit();
    void cancelEdit();
    QString formatValue(double val) const;

    QString m_propertyId;
    QString m_prefix;
    QString m_suffix;
    double m_value{0.0};
    double m_initialValue{0.0};
    double m_min{-1e9};
    double m_max{1e9};
    double m_step{0.1};
    int m_maxDecimals{4};
    bool m_isMixed{false};
    bool m_isEditing{false};
    bool m_isScrubbing{false};
    QPoint m_scrubStartPos;
    double m_scrubStartValue{0.0};

    QLabel *m_prefixLabel{nullptr};
    QLineEdit *m_lineEdit{nullptr};
};
