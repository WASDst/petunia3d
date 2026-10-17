#include "petunia_shell_controller.h"

PetuniaShellController::PetuniaShellController(QObject *parent) : QObject(parent) {
    m_timer.setInterval(33);
    connect(&m_timer, &QTimer::timeout, this, &PetuniaShellController::tick);
}

PetuniaShellController::~PetuniaShellController() {
    stop();
}

void PetuniaShellController::start() {
    if (m_started)
        return;
    m_started = true;
    petuniaRequest(QStringLiteral("session_start"));
    refresh();
    m_clock.start();
    m_timer.start();
}

void PetuniaShellController::stop() {
    if (!m_started)
        return;
    m_started = false;
    m_timer.stop();
    petuniaRequest(QStringLiteral("session_end"));
}

void PetuniaShellController::adopt(const QJsonObject &state) {
    // Respostas que não são um estado completo (ex.: {"changed":false} de um
    // atalho sem efeito) nunca substituem o último estado válido.
    if (!state.contains(QStringLiteral("workspace")))
        return;
    m_state = state;
    ++m_revision;
    emit stateChanged(m_state);
}

QJsonObject PetuniaShellController::request(const QString &action, QJsonObject args) {
    const auto result = petuniaRequest(action, std::move(args));
    if (!result.value(QStringLiteral("ok")).toBool(true)) {
        const auto message = result.value(QStringLiteral("error")).toString();
        if (!message.isEmpty())
            emit errorReported(message);
    }
    adopt(result);
    return result;
}

void PetuniaShellController::command(const QString &id) {
    request(QStringLiteral("command"), {{QStringLiteral("id"), id}});
}

void PetuniaShellController::refresh() {
    adopt(petuniaRequest(QStringLiteral("snapshot")));
}

void PetuniaShellController::tick() {
    const double dt = m_clock.restart() / 1000.0;
    const auto result = petuniaRequest(QStringLiteral("tick"), {{QStringLiteral("dt"), dt}});
    if (result.value(QStringLiteral("changed")).toBool() ||
        !result.value(QStringLiteral("ok")).toBool(true))
        adopt(result);
}

QString PetuniaShellController::workspaceId() const {
    const auto workspace = m_state.value(QStringLiteral("workspace")).toString();
    if (workspace == QLatin1String("Model")) {
        const auto mode = m_state.value(QStringLiteral("modeling_mode")).toString();
        return mode.isEmpty() ? QStringLiteral("POLY") : mode;
    }
    return workspace.toUpper();
}

bool PetuniaShellController::isModeling() const {
    return m_state.value(QStringLiteral("workspace")).toString() == QLatin1String("Model");
}

bool PetuniaShellController::flag(const QString &path) const {
    return petuniaValue(m_state, path).toBool();
}

QString PetuniaShellController::text(const QString &path) const {
    return petuniaValue(m_state, path).toString();
}

QString PetuniaShellController::shortcut(const QString &keymapAction) const {
    if (keymapAction.isEmpty())
        return {};
    return m_state.value(QStringLiteral("shortcut_map")).toObject().value(keymapAction).toString();
}

bool PetuniaShellController::commandEnabled(const QString &id, QString *reason) const {
    for (const auto &entry : m_state.value(QStringLiteral("commands")).toArray()) {
        const auto cmd = entry.toObject();
        if (cmd.value(QStringLiteral("id")).toString() == id) {
            if (reason)
                *reason = cmd.value(QStringLiteral("reason")).toString();
            return cmd.value(QStringLiteral("enabled")).toBool();
        }
    }
    return true; // não catalogado: o editor valida na execução
}
