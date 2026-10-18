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
    /// Zoom mantendo `anchor` (px do widget) sobre o mesmo ponto do canvas.
    void zoomAt(qreal factor, const QPointF &anchor);
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

/// Formulários dos diálogos do host (Preferências, Referências, Biblioteca)
/// em abas. Os workspaces usam seções nativas do Shell v2
/// (`petunia_workspace_sections`); o canvas 2D é `PetuniaEditorCanvas`.
class PetuniaEditorPanel : public PetuniaBoundForm {
    Q_OBJECT
  public:
    explicit PetuniaEditorPanel(const QString &mode, QWidget *parent = nullptr);

  private:
    QFormLayout *page(const QString &title);
    void setupSettings();
    void setupReferences();
    void setupAssets();
    QString m_mode;
    QTabWidget *m_tabs{nullptr};
};
