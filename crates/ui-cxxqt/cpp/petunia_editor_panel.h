#pragma once
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
class PetuniaEditorPanel : public QWidget {
    Q_OBJECT
  public:
    explicit PetuniaEditorPanel(const QString &mode, QWidget *parent = nullptr);
    void sync(const QJsonObject &state);
    PetuniaEditorCanvas *canvas() const { return m_canvas; }
    QStringList missingBindings(const QJsonObject &state) const;
  signals:
    void editorChanged();

  private:
    QFormLayout *page(const QString &title);
    void button(QFormLayout *, const QString &label, const QString &action,
                QJsonObject args = {}, const QString &enabledKey = {});
    void number(QFormLayout *, const QString &label, const QString &key,
                const QString &action, const QString &arg, qreal min, qreal max,
                qreal step = 0.01, QJsonObject args = {});
    void check(QFormLayout *, const QString &label, const QString &key,
               const QString &action,
               const QString &arg = QStringLiteral("value"),
               QJsonObject args = {});
    void choice(QFormLayout *, const QString &label, const QString &key,
                const QString &action, const QString &arg,
                const QStringList &ids, const QStringList &labels = {},
                QJsonObject args = {});
    void text(QFormLayout *, const QString &label, const QString &key,
              const QString &action, const QString &arg, QJsonObject args = {});
    void colorField(QFormLayout *, const QString &label, const QString &key,
                    const QString &action, const QString &arg, QJsonObject args = {});
    void xyz(QFormLayout *, const QString &label, const QString &keyPrefix,
             const QString &action,
             const QString &arg = QStringLiteral("text"),
             QJsonObject baseArgs = {});
    void rows(QFormLayout *, const QString &label, const QString &key,
              const QString &action, const QString &arg = QStringLiteral("id"),
              QJsonObject args = {});
    void fileButton(QFormLayout *, const QString &label, const QString &action,
                    const QString &filter, bool save = false,
                    QJsonObject args = {});
    void send(const QString &action, QJsonObject args = {});
    void setupModel();
    void setupPaint();
    void setupPaintGizmo();
    void setupUv();
    void setupAnimate();
    void setupSettings();
    void setupReferences();
    void setupAssets();
    void updateDynamic(const QJsonObject &state);
    struct Binding {
        QWidget *widget;
        QString key;
        QString enabledKey;
        QString labelKey;
    };
    QList<Binding> m_bindings;
    struct RowBinding {
        QListWidget *widget;
        QString key;
        QString action;
        QString arg;
        QJsonObject args;
    };
    QList<RowBinding> m_rows;
    QString m_mode;
    QJsonObject m_state;
    QTabWidget *m_tabs{nullptr};
    PetuniaEditorCanvas *m_canvas{nullptr};
    QComboBox *m_gizmoAxisCombo{nullptr};
    QFormLayout *m_dynamic{nullptr};
    QString m_dynamicSignature;
    bool m_syncing{false};
};
