#pragma once
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QKeySequence>
#include <QString>
#include <cstddef>
#include <cstdint>
extern "C" {
char *petunia_qt_editor_request(const char *input);
void petunia_qt_editor_free(char *value);
size_t petunia_qt_canvas_rgba(uint8_t *output, size_t capacity, uint32_t *width,
                              uint32_t *height);
size_t petunia_qt_editor_image(const char *kind, const char *id,
                               uint8_t *output, size_t capacity,
                               uint32_t *width, uint32_t *height);
void petunia_qt_view_activate(uint64_t id);
}
inline QJsonObject petuniaRequest(const QString &action,
                                  QJsonObject args = {}) {
    args.insert(QStringLiteral("action"), action);
    const auto input = QJsonDocument(args).toJson(QJsonDocument::Compact);
    char *reply = petunia_qt_editor_request(input.constData());
    if (!reply)
        return {{"ok", false},
                {"error", QStringLiteral("Editor indisponível")}};
    const auto result = QJsonDocument::fromJson(QByteArray(reply)).object();
    petunia_qt_editor_free(reply);
    return result;
}
inline QJsonValue petuniaValue(const QJsonObject &state, const QString &path) {
    QJsonValue value(state);
    for (const auto &part : path.split('.')) {
        if (value.isArray())
            value = value.toArray().at(part.toInt());
        else
            value = value.toObject().value(part);
    }
    return value;
}
struct PetuniaViewScope {
    explicit PetuniaViewScope(uint64_t id) { petunia_qt_view_activate(id); }
    ~PetuniaViewScope() { petunia_qt_view_activate(1); }
};
class QWidget;
void petuniaSetTranslations(const QJsonObject &translations);
void petuniaLocalize(QWidget *root);

inline QString petuniaKeyText(const QKeyEvent *event) {
    if (event->modifiers() & Qt::KeypadModifier) {
        switch (event->key()) {
        case Qt::Key_0: case Qt::Key_Insert: return QStringLiteral("Numpad0");
        case Qt::Key_1: case Qt::Key_End: return QStringLiteral("Numpad1");
        case Qt::Key_2: case Qt::Key_Down: return QStringLiteral("Numpad2");
        case Qt::Key_3: case Qt::Key_PageDown: return QStringLiteral("Numpad3");
        case Qt::Key_4: case Qt::Key_Left: return QStringLiteral("Numpad4");
        case Qt::Key_5: case Qt::Key_Clear: return QStringLiteral("Numpad5");
        case Qt::Key_6: case Qt::Key_Right: return QStringLiteral("Numpad6");
        case Qt::Key_7: case Qt::Key_Home: return QStringLiteral("Numpad7");
        case Qt::Key_8: case Qt::Key_Up: return QStringLiteral("Numpad8");
        case Qt::Key_9: case Qt::Key_PageUp: return QStringLiteral("Numpad9");
        case Qt::Key_Period: case Qt::Key_Delete: return QStringLiteral("Numpad.");
        case Qt::Key_Slash: return QStringLiteral("Numpad/");
        case Qt::Key_Asterisk: return QStringLiteral("Numpad*");
        case Qt::Key_Minus: return QStringLiteral("Numpad-");
        case Qt::Key_Plus: return QStringLiteral("Numpad+");
        case Qt::Key_Enter: return QStringLiteral("Enter");
        default: break;
        }
    }
    QString text;
    switch (event->key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
        text = "Enter";
        break;
    case Qt::Key_Escape:
        text = "Escape";
        break;
    case Qt::Key_Backspace:
        text = "Backspace";
        break;
    case Qt::Key_Delete:
        text = "Delete";
        break;
    case Qt::Key_Tab:
    case Qt::Key_Backtab:
        text = "Tab";
        break;
    default:
        if (event->key() >= Qt::Key_A && event->key() <= Qt::Key_Z)
            text = QString(QChar(event->key())).toLower();
        else if (!event->text().isEmpty() && event->text().at(0).isPrint())
            text = event->text();
        else
            text =
                QKeySequence(event->key()).toString(QKeySequence::PortableText);
    }
    return text;
}
