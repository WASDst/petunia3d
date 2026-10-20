#pragma once

#include <QPointer>
#include <QPushButton>
#include <QString>
#include <QVector>
#include <QWidget>

#include <functional>

class QMenu;
class QToolButton;

/// Item de um menu em torta: rótulo, ícone e a ação (sempre um pedido/comando do editor).
struct PetuniaPieItem {
    QString id;
    QString label;
    QString icon;
    std::function<void()> trigger;
    bool enabled{true};
};

/// Menu em torta da vista 3D (ADR-C16, B9): itens em círculo ao redor do ponteiro.
///
/// Escolher: apontar na direção e clicar; a tecla do número (1–8); setas/Tab e
/// Enter; ou clicar no item. Esc, botão direito ou clique no centro cancelam.
/// Cada item é um botão de verdade (foco, nome acessível, leitor de tela). Sem
/// animação (movimento reduzido por construção). Quem prefere lista liga
/// "Mostrar como lista" e passa a receber um menu comum com os mesmos itens.
class PetuniaPieMenu : public QWidget {
    Q_OBJECT

  public:
    explicit PetuniaPieMenu(const QString &title, QWidget *parent = nullptr);

    void setItems(const QVector<PetuniaPieItem> &items);
    int count() const { return int(m_items.size()); }
    /// Abre com o centro em `globalCenter` (ou a lista, se for a preferência).
    void popup(const QPoint &globalCenter);

    /// Índice do item na direção de `offset` (relativo ao centro); -1 na zona morta.
    int indexForOffset(const QPointF &offset) const;
    int highlighted() const { return m_highlight; }
    void setHighlighted(int index);
    /// Dispara o item `index` e fecha. Falso se não existir ou estiver desabilitado.
    bool activate(int index);
    QPushButton *button(int index) const { return m_buttons.value(index); }
    QString title() const { return m_title; }
    /// Centro em coordenadas do widget e raio do anel (testes).
    QPointF center() const;
    int ringRadius() const;

    /// Preferência "Mostrar como lista" (lembrada entre sessões).
    static bool listFallback();
    static void setListFallback(bool enabled);
    /// Menu de lista equivalente (o mesmo que a preferência abre).
    QMenu *buildListMenu(QWidget *parent);

  signals:
    void triggered(const QString &id);
    void closed();

  protected:
    void paintEvent(QPaintEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void hideEvent(QHideEvent *event) override;
    bool focusNextPrevChild(bool next) override;

  private:
    double itemAngle(int index) const;
    void layoutButtons();
    int deadZone() const;

    QString m_title;
    QVector<PetuniaPieItem> m_items;
    QVector<QPushButton *> m_buttons;
    QToolButton *m_asList{nullptr};
    int m_highlight{-1};
    bool m_pressedOutside{false};
    bool m_activated{false};
    QPointF m_pointer;
};
