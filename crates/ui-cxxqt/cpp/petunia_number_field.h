#pragma once

#include <QColor>
#include <QKeyEvent>
#include <QLineEdit>
#include <QMouseEvent>
#include <QString>
#include <QWidget>

/// Campo numérico tipado do design system UI-001 (newdocs §7).
///
/// Contrato de interação:
///   foco → editar texto → (arrastar/setas = prévia) → Enter confirma uma vez →
///   Esc restaura. Aceita expressões determinísticas ("2+3*4", "1,5/2", "pi").
///   Expressão inválida mantém o rascunho, marca o campo (borda tracejada +
///   descrição acessível) e **não** altera a cena. A roda do mouse nunca altera
///   o valor. O arrasto (scrub) só existe no rótulo do eixo/prefixo e há sempre
///   a alternativa de digitar. Focar e sair sem editar não emite nada.
///
/// O widget é apresentação pura: não conhece unidades de documento nem regras
/// de snap; recebe valor/faixa/unidade do host e emite sinais semânticos.
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
    void setPrefixColor(const QColor &color);

    const QString &propertyId() const { return m_propertyId; }
    void setPropertyId(const QString &id) { m_propertyId = id; }

    int maxDecimals() const { return m_maxDecimals; }
    void setMaxDecimals(int dec);

    /// Nome acessível do campo (também aplicado ao editor interno).
    void setLabel(const QString &label);
    const QString &label() const { return m_label; }

    void setReadOnly(bool readOnly);
    bool isReadOnly() const { return m_readOnly; }

    bool isInvalid() const { return !m_error.isEmpty(); }
    const QString &errorText() const { return m_error; }
    bool isEditing() const { return m_isEditing; }
    /// Texto atualmente exibido no editor (útil para testes e a11y).
    QString displayText() const;

    /// Entrada programática equivalente a digitar `text` e pressionar Enter.
    /// Retorna true se o texto foi aceito e `commitRequested` foi emitido.
    bool commitText(const QString &text);

    QSize sizeHint() const override;
    QSize minimumSizeHint() const override;

signals:
    void valueChanged(double value);
    void editStarted(const QString &propertyId);
    void previewRequested(const QString &propertyId, double value);
    void commitRequested(const QString &propertyId, double value, const QString &text = QString());
    void editCanceled(const QString &propertyId);
    void invalidInput(const QString &propertyId, const QString &message);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void enterEvent(QEnterEvent *event) override;
    void leaveEvent(QEvent *event) override;
    void changeEvent(QEvent *event) override;
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    void applyTheme();
    void updateDisplay();
    void setError(const QString &message);
    QString rawNumberText(double value) const;
    QString formatValue(double val) const;
    QString stripSuffix(QString text) const;
    QRect chipRect() const;
    bool inChip(const QPoint &pos) const;
    void beginEdit();
    void endEdit();
    /// Avalia o texto do editor; true = aceito (value atualizado, sem emitir).
    bool acceptText(const QString &text, double *out);
    void commitEdit();
    void cancelEdit();
    void cancelScrub();
    void stepBy(int direction, Qt::KeyboardModifiers modifiers);

    QString m_propertyId;
    QString m_prefix;
    QString m_suffix;
    QString m_label;
    QString m_error;
    QColor m_prefixColor;
    double m_value{0.0};
    double m_initialValue{0.0};
    double m_min{-1e9};
    double m_max{1e9};
    double m_step{0.1};
    int m_maxDecimals{4};
    bool m_isMixed{false};
    bool m_wasMixed{false};
    bool m_isEditing{false};
    bool m_edited{false};
    bool m_readOnly{false};
    bool m_hover{false};
    bool m_hoverChip{false};

    bool m_scrubArmed{false};
    bool m_scrubbing{false};
    QPoint m_scrubStartPos;
    double m_scrubStartValue{0.0};
    QString m_editStartText;

    QLineEdit *m_lineEdit{nullptr};
};
