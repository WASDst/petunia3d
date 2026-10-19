#include "petunia_dialog_chrome.h"

#include <QApplication>
#include <QBoxLayout>
#include <QDialog>
#include <QEvent>
#include <QHBoxLayout>
#include <QKeyEvent>
#include <QLabel>
#include <QMouseEvent>
#include <QScreen>
#include <QSettings>
#include <QToolButton>
#include <QWindow>

#include <algorithm>

#include "petunia_theme.h"

namespace {

constexpr int kNudgePx = 20;
constexpr int kMinSidePx = 160;

QString settingsKey(const QString &key) { return QStringLiteral("dialogs/%1/geometry").arg(key); }

Qt::CursorShape cursorFor(Qt::Edges edges) {
    const bool l = edges & Qt::LeftEdge, r = edges & Qt::RightEdge;
    const bool t = edges & Qt::TopEdge, b = edges & Qt::BottomEdge;
    if ((l && t) || (r && b)) {
        return Qt::SizeFDiagCursor;
    }
    if ((r && t) || (l && b)) {
        return Qt::SizeBDiagCursor;
    }
    if (l || r) {
        return Qt::SizeHorCursor;
    }
    if (t || b) {
        return Qt::SizeVerCursor;
    }
    return Qt::ArrowCursor;
}

/// Mantém ao menos um pedaço da barra de título dentro de alguma tela: uma geometria
/// lembrada de um monitor que já não existe não pode deixar o diálogo inalcançável.
QRect clampToScreens(QRect rect) {
    const auto screens = QGuiApplication::screens();
    if (screens.isEmpty()) {
        return rect;
    }
    for (QScreen *screen : screens) {
        if (screen && screen->availableGeometry().intersects(QRect(rect.topLeft(), QSize(rect.width(), 32)))) {
            return rect;
        }
    }
    QScreen *primary = QGuiApplication::primaryScreen();
    const QRect area = primary ? primary->availableGeometry() : screens.first()->availableGeometry();
    rect.setSize(rect.size().boundedTo(area.size()));
    rect.moveCenter(area.center());
    return rect;
}

} // namespace

int PetuniaDialogChrome::edgeWidth() { return 6; }

QRect PetuniaDialogChrome::rememberedGeometry(const QString &key) {
    if (key.isEmpty()) {
        return {};
    }
    return QSettings(QStringLiteral("Petunia3D"), QStringLiteral("QtShell")).value(settingsKey(key)).toRect();
}

PetuniaDialogChrome *PetuniaDialogChrome::install(QDialog *dialog, const QString &key, Options options) {
    if (!dialog) {
        return nullptr;
    }
    return new PetuniaDialogChrome(dialog, key, options);
}

PetuniaDialogChrome::PetuniaDialogChrome(QDialog *dialog, const QString &key, Options options)
    : QObject(dialog), m_dialog(dialog), m_key(key), m_options(options) {
    setObjectName(QStringLiteral("petunia-dialog-chrome"));
    dialog->setWindowFlag(Qt::FramelessWindowHint, true);
    dialog->setAttribute(Qt::WA_Hover, true);
    dialog->setMouseTracking(true);
    dialog->setProperty("petuniaChrome", true);

    if (options.existingHeader) {
        m_handle = options.existingHeader;
    } else {
        auto *bar = new QWidget(dialog);
        bar->setObjectName(QStringLiteral("petunia-dialog-titlebar"));
        bar->setAccessibleName(QObject::tr("Barra de título"));
        bar->setAccessibleDescription(
            QObject::tr("Arraste para mover. Ctrl+Alt+setas movem; com Shift redimensionam."));
        auto *row = new QHBoxLayout(bar);
        row->setContentsMargins(PetuniaTheme::instance().scaled(10), PetuniaTheme::instance().scaled(4), PetuniaTheme::instance().scaled(4),
                                PetuniaTheme::instance().scaled(4));
        m_title = new QLabel(dialog->windowTitle(), bar);
        m_title->setObjectName(QStringLiteral("petunia-dialog-title"));
        QFont font = m_title->font();
        font.setBold(true);
        m_title->setFont(font);
        row->addWidget(m_title, 1);
        auto *close = new QToolButton(bar);
        close->setObjectName(QStringLiteral("petunia-dialog-close"));
        close->setText(QStringLiteral("✕"));
        close->setAutoRaise(true);
        close->setToolTip(QObject::tr("Fechar (Esc)"));
        close->setAccessibleName(QObject::tr("Fechar"));
        close->setFocusPolicy(Qt::TabFocus);
        QObject::connect(close, &QToolButton::clicked, dialog, &QDialog::reject);
        row->addWidget(close);
        bar->setCursor(Qt::OpenHandCursor);
        if (auto *box = qobject_cast<QBoxLayout *>(dialog->layout())) {
            box->insertWidget(0, bar);
        }
        m_handle = bar;
        connect(dialog, &QWidget::windowTitleChanged, this, [this](const QString &title) {
            if (m_title) {
                m_title->setText(title);
            }
        });
    }
    m_handle->installEventFilter(this);
    dialog->installEventFilter(this);
    // Bordas visíveis: sem decoração do sistema, o próprio diálogo desenha o contorno.
    dialog->setStyleSheet(dialog->styleSheet() +
                          QStringLiteral("QDialog[petuniaChrome=\"true\"]{border:1px solid palette(mid);}"));
    restoreGeometry();
    connect(dialog, &QDialog::finished, this, [this](int) { saveGeometry(); });
}

Qt::Edges PetuniaDialogChrome::edgesAt(const QPoint &pos) const {
    Qt::Edges edges;
    if (!m_dialog || !m_options.resizable) {
        return edges;
    }
    const int w = PetuniaTheme::instance().scaled(edgeWidth());
    const QRect r = m_dialog->rect();
    if (pos.x() <= w) {
        edges |= Qt::LeftEdge;
    } else if (pos.x() >= r.width() - w) {
        edges |= Qt::RightEdge;
    }
    if (pos.y() <= w) {
        edges |= Qt::TopEdge;
    } else if (pos.y() >= r.height() - w) {
        edges |= Qt::BottomEdge;
    }
    return edges;
}

bool PetuniaDialogChrome::eventFilter(QObject *watched, QEvent *event) {
    if (!m_dialog) {
        return false;
    }
    const bool onHandle = watched == m_handle;
    const bool onDialog = watched == m_dialog;
    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto *mouse = static_cast<QMouseEvent *>(event);
        if (mouse->button() != Qt::LeftButton) {
            break;
        }
        if (onDialog) {
            const Qt::Edges edges = edgesAt(mouse->position().toPoint());
            if (edges) {
                beginResize(edges, mouse->globalPosition().toPoint());
                return true;
            }
        } else if (onHandle) {
            beginMove(mouse->globalPosition().toPoint());
            return true;
        }
        break;
    }
    case QEvent::MouseMove: {
        auto *mouse = static_cast<QMouseEvent *>(event);
        const QPoint global = mouse->globalPosition().toPoint();
        if (m_drag == Drag::Move) {
            m_dialog->move(m_pressGeometry.topLeft() + (global - m_pressGlobal));
            return true;
        }
        if (m_drag == Drag::Resize) {
            QRect g = m_pressGeometry;
            const QPoint d = global - m_pressGlobal;
            const int minW = std::max(PetuniaTheme::instance().scaled(kMinSidePx), m_dialog->minimumSizeHint().width());
            const int minH = std::max(PetuniaTheme::instance().scaled(kMinSidePx / 2), m_dialog->minimumSizeHint().height());
            if (m_edges & Qt::LeftEdge) {
                g.setLeft(std::min(g.left() + d.x(), g.right() - minW));
            }
            if (m_edges & Qt::RightEdge) {
                g.setRight(std::max(g.right() + d.x(), g.left() + minW));
            }
            if (m_edges & Qt::TopEdge) {
                g.setTop(std::min(g.top() + d.y(), g.bottom() - minH));
            }
            if (m_edges & Qt::BottomEdge) {
                g.setBottom(std::max(g.bottom() + d.y(), g.top() + minH));
            }
            m_dialog->setGeometry(g);
            return true;
        }
        if (onDialog && mouse->buttons() == Qt::NoButton) {
            const Qt::Edges edges = edgesAt(mouse->position().toPoint());
            if (edges) {
                m_dialog->setCursor(cursorFor(edges));
            } else {
                m_dialog->unsetCursor();
            }
        }
        break;
    }
    case QEvent::MouseButtonRelease:
        if (m_drag != Drag::None) {
            if (m_drag == Drag::Resize) {
                m_dialog->releaseMouse();
            }
            m_drag = Drag::None;
            if (m_handle) {
                m_handle->setCursor(Qt::OpenHandCursor);
            }
            return true;
        }
        break;
    case QEvent::MouseButtonDblClick:
        // Duplo clique na barra recentraliza (alternativa a procurar a janela perdida).
        if (onHandle && m_dialog->parentWidget()) {
            QRect g = m_dialog->geometry();
            g.moveCenter(m_dialog->parentWidget()->window()->geometry().center());
            m_dialog->setGeometry(clampToScreens(g));
            return true;
        }
        break;
    case QEvent::KeyPress:
        if (onDialog) {
            auto *key = static_cast<QKeyEvent *>(event);
            const auto mods = key->modifiers() & ~Qt::KeypadModifier;
            const bool chord = (mods & Qt::ControlModifier) && (mods & Qt::AltModifier);
            if (!chord) {
                break;
            }
            const bool resize = mods & Qt::ShiftModifier;
            switch (key->key()) {
            case Qt::Key_Left:
                nudge(-kNudgePx, 0, resize);
                return true;
            case Qt::Key_Right:
                nudge(kNudgePx, 0, resize);
                return true;
            case Qt::Key_Up:
                nudge(0, -kNudgePx, resize);
                return true;
            case Qt::Key_Down:
                nudge(0, kNudgePx, resize);
                return true;
            default:
                break;
            }
        }
        break;
    case QEvent::Leave:
        if (onDialog && m_drag == Drag::None) {
            m_dialog->unsetCursor();
        }
        break;
    case QEvent::Hide:
        if (onDialog) {
            saveGeometry();
        }
        break;
    default:
        break;
    }
    return false;
}

void PetuniaDialogChrome::beginMove(const QPoint &globalPos) {
    // O compositor move a janela (no Wayland é o único caminho); se recusar, movemos nós.
    QWindow *window = m_dialog->windowHandle();
    if (window && window->startSystemMove()) {
        return;
    }
    m_drag = Drag::Move;
    m_pressGlobal = globalPos;
    m_pressGeometry = m_dialog->geometry();
    if (m_handle) {
        m_handle->setCursor(Qt::ClosedHandCursor);
    }
}

void PetuniaDialogChrome::beginResize(Qt::Edges edges, const QPoint &globalPos) {
    QWindow *window = m_dialog->windowHandle();
    if (window && window->startSystemResize(edges)) {
        return;
    }
    m_drag = Drag::Resize;
    m_edges = edges;
    m_pressGlobal = globalPos;
    m_pressGeometry = m_dialog->geometry();
    // Eventos de movimento fora do diálogo continuam chegando enquanto o botão estiver preso.
    m_dialog->grabMouse();
}

void PetuniaDialogChrome::nudge(int dx, int dy, bool resize) {
    const int step = PetuniaTheme::instance().scaled(1);
    QRect g = m_dialog->geometry();
    if (resize) {
        if (!m_options.resizable) {
            return;
        }
        const QSize min = m_dialog->minimumSizeHint().expandedTo(
            QSize(PetuniaTheme::instance().scaled(kMinSidePx), PetuniaTheme::instance().scaled(kMinSidePx / 2)));
        g.setSize(QSize(std::max(min.width(), g.width() + dx * step), std::max(min.height(), g.height() + dy * step)));
    } else {
        g.translate(dx * step, dy * step);
    }
    m_dialog->setGeometry(clampToScreens(g));
}

void PetuniaDialogChrome::restoreGeometry() {
    if (m_key.isEmpty() || !m_dialog) {
        return;
    }
    const QRect saved = rememberedGeometry(m_key);
    if (!saved.isValid() || saved.width() < kMinSidePx / 2 || saved.height() < kMinSidePx / 4) {
        return;
    }
    QRect g = saved;
    if (!m_options.resizable) {
        g.setSize(m_dialog->sizeHint());
    }
    m_dialog->setGeometry(clampToScreens(g));
}

void PetuniaDialogChrome::saveGeometry() {
    if (m_key.isEmpty() || !m_dialog) {
        return;
    }
    const QRect g = m_dialog->geometry();
    if (g.isValid()) {
        QSettings(QStringLiteral("Petunia3D"), QStringLiteral("QtShell")).setValue(settingsKey(m_key), g);
    }
}
