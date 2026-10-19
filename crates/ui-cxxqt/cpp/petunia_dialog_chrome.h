#pragma once

#include <QObject>
#include <QPoint>
#include <QPointer>
#include <QRect>
#include <QString>

class QDialog;
class QLabel;
class QWidget;

/// Moldura própria para diálogos (ADR-C16, B5): barra de título que arrasta a janela,
/// bordas e cantos que redimensionam, atalhos de teclado e posição/tamanho lembrados.
///
/// Não depende da decoração do gerenciador de janelas: no Wayland sem decoração do
/// servidor um `QDialog` comum fica sem barra e sem bordas (não se move nem se
/// redimensiona). Mover e redimensionar passam pelo compositor (`startSystemMove` /
/// `startSystemResize`) e, se ele recusar (ex.: offscreen), pelo próprio widget.
///
/// Teclado: Ctrl+Alt+setas movem 20 px; Ctrl+Alt+Shift+setas redimensionam.
class PetuniaDialogChrome : public QObject {
  public:
    struct Options {
        /// Bordas e cantos redimensionam (e os atalhos de tamanho valem).
        bool resizable{true};
        /// Cabeçalho que o diálogo já tem: vira a alça de arrasto e nenhuma barra é criada.
        QWidget *existingHeader{nullptr};
    };

    /// Instala em `dialog` (antes de exibi-lo; o layout já deve existir). `key` identifica
    /// a geometria lembrada entre sessões; vazio = não lembra.
    static PetuniaDialogChrome *install(QDialog *dialog, const QString &key, Options options);
    static PetuniaDialogChrome *install(QDialog *dialog, const QString &key) {
        return install(dialog, key, Options{});
    }

    QWidget *titleBar() const { return m_handle; }
    /// Largura (px) da faixa das bordas que redimensiona.
    static int edgeWidth();
    /// Geometria lembrada para `key` (vazia se nunca salva). Para testes.
    static QRect rememberedGeometry(const QString &key);

  protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

  private:
    PetuniaDialogChrome(QDialog *dialog, const QString &key, Options options);
    Qt::Edges edgesAt(const QPoint &pos) const;
    void beginMove(const QPoint &globalPos);
    void beginResize(Qt::Edges edges, const QPoint &globalPos);
    void restoreGeometry();
    void saveGeometry();
    void nudge(int dx, int dy, bool resize);

    QPointer<QDialog> m_dialog;
    QString m_key;
    Options m_options;
    QWidget *m_handle{nullptr};
    QLabel *m_title{nullptr};
    // Arrasto manual (quando o compositor não move/redimensiona).
    enum class Drag { None, Move, Resize };
    Drag m_drag{Drag::None};
    Qt::Edges m_edges;
    QPoint m_pressGlobal;
    QRect m_pressGeometry;
};
