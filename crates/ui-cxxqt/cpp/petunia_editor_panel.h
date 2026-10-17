#pragma once
#include "petunia_bound_form.h"
#include "petunia_editor_api.h"
#include <QComboBox>
#include <QFormLayout>
#include <QHash>
#include <QImage>
#include <QListWidget>
#include <QPainterPath>
#include <QTabWidget>
#include <QWidget>
#include <functional>

QPainterPath petuniaPath(const QString &commands);

class PetuniaEditorCanvas : public QWidget {
    Q_OBJECT
  public:
    explicit PetuniaEditorCanvas(const QString &mode,
                                 QWidget *parent = nullptr);
    void sync(const QJsonObject &state);
  signals:
    void editorChanged();

  protected:
    void paintEvent(QPaintEvent *) override;
    void mousePressEvent(QMouseEvent *) override;
    void mouseMoveEvent(QMouseEvent *) override;
    void mouseReleaseEvent(QMouseEvent *) override;
    void wheelEvent(QWheelEvent *) override;
    void tabletEvent(QTabletEvent *) override;
    void keyPressEvent(QKeyEvent *) override;
    void focusOutEvent(QFocusEvent *) override;
    void enterEvent(QEnterEvent *) override;
    void leaveEvent(QEvent *) override;

  private:
    QRectF canvasRect() const;
    QPointF normalized(const QPointF &) const;
    void stroke(const QPointF &, int phase);
    /// Relê o raster da camada ativa do documento (nunca do widget).
    bool reloadCanvasImage();
    /// Anel do pincel sobre o canvas, com o mesmo diâmetro do carimbo.
    void drawPaintCursor(QPainter &) const;
    bool isPaintBrushTool() const;
    QString m_mode;
    QJsonObject m_state;
    QImage m_image;
    qreal m_zoom{1.0};
    QPointF m_mousePos;
    QPointF m_pan, m_previous;
    bool m_panning{false}, m_stroking{false}, m_uvDragging{false},
        m_selecting{false};
    QPointF m_press, m_dragDelta;
    QString m_dragRevision, m_dragAsset, m_dragSelection;
    qreal m_pressure{1.0};
};

/// Native widgets read the shared DTO; every control sends a typed adapter
/// action.
class PetuniaEditorPanel : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaEditorPanel(const QString &mode, QWidget *parent = nullptr);
    PetuniaEditorCanvas *canvas() const { return m_canvas; }
    /// Shell v2: entrega o canvas e as abas para regiões próprias do shell. O painel
    /// continua sendo o motor de sincronização (`sync`) dos dois.
    QWidget *takeCanvas();
    QTabWidget *takeTabs();

  protected:
    void afterSync(const QJsonObject &state) override;

  private:
    QFormLayout *page(const QString &title);
    void setupModel();
    void setupPaint();
    void setupPaintGizmo();
    void setupUv();
    void setupAnimate();
    void setupSettings();
    void setupReferences();
    void setupAssets();
    void updateDynamic(const QJsonObject &state);
    QString m_mode;
    QTabWidget *m_tabs{nullptr};
    PetuniaEditorCanvas *m_canvas{nullptr};
    QComboBox *m_gizmoAxisCombo{nullptr};
    QFormLayout *m_dynamic{nullptr};
    QString m_dynamicSignature;
};
