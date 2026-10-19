#pragma once

#include <QColor>
#include <QDialog>
#include <QFrame>
#include <QJsonArray>
#include <QLabel>
#include <QLineEdit>
#include <QWidget>

#include "petunia_number_field.h"

class QComboBox;
class QSlider;
class QStackedWidget;
class QToolButton;

/// Roda de cor: anel de matiz + quadrado saturação/valor. O alvo do arrasto
/// (anel ou quadrado) é decidido no clique e não muda no meio do gesto.
class PetuniaColorWheel : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaColorWheel(QWidget *parent = nullptr);
    ~PetuniaColorWheel() override = default;

    QColor color() const;
    void setColor(const QColor &color);
    /// Diâmetro em px lógicos (escala do tema aplicada).
    void setDiameter(int logicalPx);

  signals:
    void colorChanged(const QColor &color);

  protected:
    void paintEvent(QPaintEvent *) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

  private:
    enum class Target { None, Ring, Square };
    void handleInteraction(const QPointF &pos);
    float outerRadius() const;
    QRectF svRect() const;
    float m_hue{30.0f};
    float m_saturation{0.8f};
    float m_value{0.9f};
    Target m_target{Target::None};
};

/// Caixa de cor: quadrado saturação/valor com uma faixa vertical de matiz ao lado.
class PetuniaColorBox : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaColorBox(QWidget *parent = nullptr);

    QColor color() const;
    void setColor(const QColor &color);

  signals:
    void colorChanged(const QColor &color);

  protected:
    void paintEvent(QPaintEvent *) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

  private:
    enum class Target { None, Square, Strip };
    QRectF squareRect() const;
    QRectF stripRect() const;
    void handleInteraction(const QPointF &pos);
    float m_hue{30.0f};
    float m_saturation{0.8f};
    float m_value{0.9f};
    Target m_target{Target::None};
};

/// Grade de amostras de uma paleta: clique escolhe; botão direito ou Menu abre
/// Editar cor / Apagar cor. Setas percorrem as amostras e Enter escolhe.
class PetuniaSwatchGrid : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaSwatchGrid(QWidget *parent = nullptr);

    void setColors(const QVector<QColor> &colors);
    int count() const { return int(m_colors.size()); }
    QColor colorAt(int index) const { return m_colors.value(index); }
    /// Centro da amostra `index` (testes e tecnologias assistivas).
    QRect swatchRect(int index) const;

    QSize sizeHint() const override;
    bool hasHeightForWidth() const override { return true; }
    int heightForWidth(int width) const override;

  signals:
    void picked(int index);
    void editRequested(int index);
    void removeRequested(int index);

  protected:
    void paintEvent(QPaintEvent *) override;
    void mousePressEvent(QMouseEvent *event) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

  private:
    int columns() const;
    int indexAt(const QPoint &pos) const;
    void showMenu(int index, const QPoint &globalPos);
    QVector<QColor> m_colors;
    int m_current{-1};
};

/// Seletor de cor único do aplicativo (ADR-C16, B1): área visual à esquerda (roda ou
/// caixa) e, à direita, canais RGB/HSL/HSV com alfa, hex, "+ paleta", escolha de
/// paleta (nova, renomear, apagar) e as cores da paleta (editar e apagar no menu).
/// Paletas são dados do documento: toda mudança é um pedido `palette` com Undo.
class PetuniaColorPanel : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaColorPanel(QWidget *parent = nullptr);

    QColor color() const { return m_color; }
    /// Não emite `colorChanged` (o chamador está sincronizando, não o usuário).
    void setColor(const QColor &color);
    /// Sem alfa, o canal A some e a cor é sempre opaca (ex.: cores de seleção).
    void setAlphaEnabled(bool enabled);
    bool alphaEnabled() const { return m_alphaEnabled; }
    /// Relê as paletas do documento (`palettes` do snapshot).
    void syncPalettes(const QJsonArray &palettes);
    void refreshPalettes();

    // Acesso para testes e tecnologias assistivas.
    PetuniaSwatchGrid *swatches() const { return m_swatches; }
    QComboBox *paletteChooser() const { return m_paletteChooser; }
    QLineEdit *hexEditor() const { return m_hex; }
    QString currentPaletteId() const;
    void addCurrentColorToPalette();
    bool createPalette(const QString &name);
    bool renamePalette(const QString &name);
    bool deleteCurrentPalette();
    void setMode(int index);
    int mode() const;

  signals:
    /// Mudança feita pelo usuário (arrasto, campo, hex, amostra).
    void colorChanged(const QColor &color);

  protected:
    /// Largo: área visual à esquerda. Estreito (coluna de propriedades): empilha.
    void resizeEvent(QResizeEvent *event) override;

  private:
    enum class Model { Rgb, Hsl, Hsv };
    class QBoxLayout *m_root{nullptr};
    bool m_stacked{false};
    void applyColor(const QColor &color, bool emitChange);
    void refreshChannels();
    void channelEdited();
    void requestPalette(const QJsonObject &args);
    void rebuildSwatches();

    QColor m_color{QColor::fromRgbF(0.91f, 0.42f, 0.0f)};
    bool m_alphaEnabled{true};
    bool m_updating{false};
    Model m_model{Model::Rgb};
    QJsonArray m_palettes;
    QString m_paletteId; // vazio = paleta do projeto

    QComboBox *m_modeChooser{nullptr};
    QStackedWidget *m_visual{nullptr};
    PetuniaColorWheel *m_wheel{nullptr};
    PetuniaColorBox *m_box{nullptr};
    QComboBox *m_modelChooser{nullptr};
    QLabel *m_channelLabels[4]{};
    QSlider *m_channelSliders[4]{};
    PetuniaNumberField *m_channelFields[4]{};
    QFrame *m_preview{nullptr};
    QLineEdit *m_hex{nullptr};
    QToolButton *m_addToPalette{nullptr};
    QComboBox *m_paletteChooser{nullptr};
    QToolButton *m_newPalette{nullptr};
    QToolButton *m_renamePalette{nullptr};
    QToolButton *m_deletePalette{nullptr};
    PetuniaSwatchGrid *m_swatches{nullptr};
};

/// Diálogo que hospeda o `PetuniaColorPanel` (Cancelar/Confirmar), usado onde a cor
/// é escolhida para depois aplicar (cores de seleção, materiais). Para o pincel, o
/// painel fica embutido e aplica ao vivo.
class PetuniaColorPickerPopover : public QDialog {
    Q_OBJECT

  public:
    explicit PetuniaColorPickerPopover(QWidget *parent = nullptr);
    ~PetuniaColorPickerPopover() override = default;

    void setColor(const QColor &color);
    QColor color() const;
    void setTitle(const QString &title);
    void setAlphaEnabled(bool enabled);
    PetuniaColorPanel *panel() const { return m_panel; }

  signals:
    void colorChanged(const QColor &color);

  private:
    PetuniaColorPanel *m_panel{nullptr};
};
