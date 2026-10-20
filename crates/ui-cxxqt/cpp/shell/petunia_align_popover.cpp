#include "petunia_align_popover.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QPainter>
#include <QPainterPath>
#include <QVBoxLayout>

namespace {
/// Posições de alinhamento (colunas) e a distribuição.
enum GlyphKind { Min = 0, Center = 1, Max = 2, Distribute = 3 };

/// Botão com pictograma desenhado em código: duas barras alinhadas a uma linha
/// de referência na cor do eixo (mín., centro, máx.) ou três barras espaçadas.
class GlyphButton : public QToolButton {
  public:
    GlyphButton(int axis, int kind, QWidget *parent) : QToolButton(parent), m_axis(axis), m_kind(kind) {
        setCursor(Qt::PointingHandCursor);
        setToolButtonStyle(Qt::ToolButtonIconOnly);
        setProperty("role", QStringLiteral("header"));
    }

  protected:
    void paintEvent(QPaintEvent *event) override {
        QToolButton::paintEvent(event);
        const auto &theme = PetuniaTheme::instance();
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing, true);
        const double side = std::min(width(), height()) * 0.62;
        p.translate(width() / 2.0, height() / 2.0);
        // Y alinha na vertical: gira o desenho; Z usa o desenho de X com a cor do eixo.
        if (m_axis == 1)
            p.rotate(-90);
        const QColor axis = theme.axisColor(m_axis);
        const QColor ink = isEnabled() ? theme.textPrimary() : theme.textMuted();
        const double h = side / 2.0;
        p.setPen(Qt::NoPen);
        p.setBrush(ink);
        if (m_kind == Distribute) {
            const double w = side * 0.18;
            const double heights[3] = {side * 0.78, side * 0.5, side * 0.9};
            for (int i = 0; i < 3; ++i) {
                const double x = -h + i * (side - w) / 2.0;
                p.drawRoundedRect(QRectF(x, -heights[i] / 2.0, w, heights[i]), 1.2, 1.2);
            }
            QColor tick = axis;
            tick.setAlpha(isEnabled() ? 255 : 110);
            p.setPen(QPen(tick, 1.4, Qt::SolidLine, Qt::RoundCap));
            p.drawLine(QPointF(-h, h + 2.0), QPointF(h, h + 2.0));
            return;
        }
        // Linha de referência.
        const double refX = m_kind == Min ? -h : (m_kind == Center ? 0.0 : h);
        QColor line = axis;
        line.setAlpha(isEnabled() ? 255 : 110);
        p.setPen(QPen(line, 1.8, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(refX, -h - 1.0), QPointF(refX, h + 1.0));
        p.setPen(Qt::NoPen);
        p.setBrush(ink);
        const double longBar = side * 0.82;
        const double shortBar = side * 0.5;
        const double barH = side * 0.26;
        auto bar = [&](double length, double y) {
            double x0 = refX;
            if (m_kind == Min)
                x0 = refX + 1.8;
            else if (m_kind == Center)
                x0 = refX - length / 2.0;
            else
                x0 = refX - length - 1.8;
            p.drawRoundedRect(QRectF(x0, y, length, barH), 1.2, 1.2);
        };
        bar(longBar, -barH - 1.2);
        bar(shortBar, 1.2);
    }

  private:
    int m_axis;
    int m_kind;
};
} // namespace

PetuniaAlignPopover::PetuniaAlignPopover(PetuniaShellController *controller, QWidget *parent)
    : QFrame(parent, Qt::Popup | Qt::FramelessWindowHint), m_controller(controller) {
    setObjectName(QStringLiteral("PetuniaAlignPopover"));
    setProperty("role", QStringLiteral("overlay"));
    setAccessibleName(tr("Alinhar e distribuir"));
    const auto &theme = PetuniaTheme::instance();
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(theme.space(3), theme.space(3), theme.space(3), theme.space(3));
    root->setSpacing(theme.space(2));

    auto *title = new QLabel(tr("Alinhar e distribuir"), this);
    title->setProperty("role", QStringLiteral("title"));
    root->addWidget(title);
    m_hint = new QLabel(this);
    m_hint->setProperty("role", QStringLiteral("muted"));
    m_hint->setWordWrap(true);
    root->addWidget(m_hint);

    auto *grid = new QGridLayout();
    grid->setHorizontalSpacing(theme.space(1));
    grid->setVerticalSpacing(theme.space(1));
    const QStringList kinds{"min", "center", "max"};
    const QStringList kindNames{tr("mínimos"), tr("centros"), tr("máximos")};
    const QStringList axisNames{"X", "Y", "Z"};
    const QStringList axisKeys{"x", "y", "z"};
    // Cabeçalho das colunas.
    const QStringList columnTitles{tr("Mín."), tr("Centro"), tr("Máx.")};
    for (int c = 0; c < 3; ++c) {
        auto *label = new QLabel(columnTitles[c], this);
        label->setProperty("role", QStringLiteral("caption"));
        label->setAlignment(Qt::AlignCenter);
        grid->addWidget(label, 0, c + 1);
    }
    for (int a = 0; a < 3; ++a) {
        auto *axisLabel = new QLabel(axisNames[a], this);
        QColor c = theme.axisColor(a);
        axisLabel->setStyleSheet(QStringLiteral("QLabel { color: %1; font-weight: 700; }").arg(c.name()));
        axisLabel->setAccessibleName(tr("Eixo %1").arg(axisNames[a]));
        grid->addWidget(axisLabel, a + 1, 0);
        for (int k = 0; k < 3; ++k) {
            const QString command = QStringLiteral("model.align_%1_%2").arg(kinds[k], axisKeys[a]);
            const QString tip = tr("Alinhar os %1 no eixo %2").arg(kindNames[k], axisNames[a]);
            grid->addWidget(makeButton(command, tip, a, k), a + 1, k + 1);
        }
    }
    root->addLayout(grid);

    auto *separator = new QFrame(this);
    separator->setProperty("role", QStringLiteral("separator"));
    separator->setFixedHeight(1);
    root->addWidget(separator);

    auto *distribute = new QHBoxLayout();
    distribute->setSpacing(theme.space(1));
    auto *distributeLabel = new QLabel(tr("Distribuir"), this);
    distributeLabel->setProperty("role", QStringLiteral("caption"));
    distribute->addWidget(distributeLabel, 1);
    for (int a = 0; a < 3; ++a) {
        const QString command = QStringLiteral("model.distribute_objects_%1").arg(axisKeys[a]);
        const QString tip = tr("Distribuir os objetos com espaços iguais no eixo %1").arg(axisNames[a]);
        distribute->addWidget(makeButton(command, tip, a, Distribute));
    }
    root->addLayout(distribute);
    hide();
}

QToolButton *PetuniaAlignPopover::makeButton(const QString &command, const QString &tip, int axis, int kind) {
    const auto &theme = PetuniaTheme::instance();
    auto *button = new GlyphButton(axis, kind, this);
    const int size = theme.compactControlHeight() + theme.space(1);
    button->setFixedSize(size, size);
    button->setToolTip(tip);
    button->setAccessibleName(tip);
    button->setProperty("command", command);
    connect(button, &QToolButton::clicked, this, [this, command] { m_controller->command(command); });
    m_buttons.append(button);
    return button;
}

void PetuniaAlignPopover::sync(const QJsonObject &state) {
    const int selected = state["selected_count"].toInt();
    m_hint->setText(selected >= 2 ? tr("%1 objetos selecionados. Alinha na caixa da seleção.").arg(selected)
                                  : tr("Selecione 2 ou mais objetos para alinhar. Distribuir pede 3."));
    for (auto *button : std::as_const(m_buttons)) {
        const QString command = button->property("command").toString();
        QString reason;
        bool enabled = m_controller->commandEnabled(command, &reason);
        const bool distribute = command.contains(QLatin1String("distribute"));
        if (selected < (distribute ? 3 : 2)) {
            enabled = false;
            reason = distribute ? tr("Distribuir pede 3 ou mais objetos") : tr("Alinhar pede 2 ou mais objetos");
        }
        button->setEnabled(enabled);
        QString tip = button->accessibleName();
        if (!enabled && !reason.isEmpty())
            tip += QStringLiteral(" — ") + reason;
        if (button->toolTip() != tip)
            button->setToolTip(tip);
    }
}

void PetuniaAlignPopover::popupAt(QWidget *anchor) {
    sync(m_controller->state());
    adjustSize();
    QPoint origin;
    if (anchor)
        origin = anchor->mapToGlobal(QPoint(0, -height() - PetuniaTheme::instance().space(1)));
    else if (parentWidget())
        origin = parentWidget()->mapToGlobal(QPoint(parentWidget()->width() / 2, parentWidget()->height() / 2));
    move(origin);
    show();
    raise();
}
